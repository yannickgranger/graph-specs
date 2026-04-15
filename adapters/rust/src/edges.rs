//! Rust-side edge extraction (v0.3).
//!
//! Walks a parsed [`syn::File`] alongside the concept-level walk and emits
//! [`Edge`] values for the three v0.3 relationship kinds:
//!
//! - [`EdgeKind::Implements`] — from `impl Trait for Type`.
//! - [`EdgeKind::DependsOn`] — from struct/enum field types and `pub fn`
//!   parameter types inside inherent impls and trait definitions.
//! - [`EdgeKind::Returns`] — from the primary head of `pub fn` return
//!   types inside inherent impls and trait definitions.
//!
//! Trait impls (`impl Trait for Type`) emit [`EdgeKind::Implements`] and
//! the `DEPENDS_ON` / RETURNS edges derived from their method signatures.
//! This lets downstream consumers reason about a concrete type's actual
//! declared surface without having to chase back through the trait.
//!
//! All emitted edges are filtered by [`filter_by_known_concepts`] against
//! the concept names discovered on the code side — only edges whose
//! `target` matches a discovered concept are kept. This removes
//! dependencies on primitives (`u32`, `String`) and external types (`Vec`,
//! `HashMap`, `Path`).

use domain::{tokenise_target, ConceptNode, Edge, EdgeKind, Source};
use proc_macro2::Span;
use std::collections::HashSet;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use syn::spanned::Spanned;
use syn::{
    FnArg, GenericArgument, ImplItem, Item, ItemEnum, ItemImpl, ItemStruct, ItemTrait,
    PathArguments, ReturnType, Signature, TraitItem, Type, Visibility,
};

/// Retain only edges whose `target` appears as a concept in `nodes`.
#[must_use]
pub fn filter_by_known_concepts(edges: Vec<Edge>, nodes: &[ConceptNode]) -> Vec<Edge> {
    let known: HashSet<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
    edges
        .into_iter()
        .filter(|e| known.contains(e.target.as_str()))
        .collect()
}

/// Top-level dispatch: emit edges for a single parsed item.
pub fn emit_for_item(item: &Item, path: &Path, out: &mut Vec<Edge>) {
    match item {
        Item::Struct(s) => emit_struct_depends_on(s, path, out),
        Item::Enum(e) => emit_enum_depends_on(e, path, out),
        Item::Trait(t) => emit_trait_edges(t, path, out),
        Item::Impl(i) => emit_impl_edges(i, path, out),
        _ => {}
    }
}

fn emit_struct_depends_on(s: &ItemStruct, path: &Path, out: &mut Vec<Edge>) {
    if !matches!(s.vis, Visibility::Public(_)) {
        return;
    }
    let owner = s.ident.to_string();
    for field in &s.fields {
        push_depends_on_from_type(&field.ty, &owner, path, out);
    }
}

fn emit_enum_depends_on(e: &ItemEnum, path: &Path, out: &mut Vec<Edge>) {
    if !matches!(e.vis, Visibility::Public(_)) {
        return;
    }
    let owner = e.ident.to_string();
    for variant in &e.variants {
        for field in &variant.fields {
            push_depends_on_from_type(&field.ty, &owner, path, out);
        }
    }
}

fn emit_trait_edges(t: &ItemTrait, path: &Path, out: &mut Vec<Edge>) {
    if !matches!(t.vis, Visibility::Public(_)) {
        return;
    }
    let owner = t.ident.to_string();
    for ti in &t.items {
        if let TraitItem::Fn(f) = ti {
            emit_fn_edges(&f.sig, &owner, path, out);
        }
    }
}

fn emit_impl_edges(i: &ItemImpl, path: &Path, out: &mut Vec<Edge>) {
    let Some(owner) = impl_target_name(&i.self_ty) else {
        return;
    };

    if let Some((_, trait_path, _)) = &i.trait_ {
        let trait_name = trait_path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if !trait_name.is_empty() {
            out.push(Edge {
                source_concept: owner.clone(),
                kind: EdgeKind::Implements,
                target: trait_name.clone(),
                raw_target: trait_name,
                source: code_source(path, trait_path.span()),
            });
        }
    }

    // Walk methods for `DEPENDS_ON` + RETURNS. Applies to both inherent
    // impls (no trait_) and trait impls — the AC treats "function
    // parameter types" uniformly and makes no inherent-vs-trait-impl
    // distinction for `DEPENDS_ON` / RETURNS emission.
    for item in &i.items {
        if let ImplItem::Fn(f) = item {
            if matches!(f.vis, Visibility::Public(_)) || i.trait_.is_some() {
                emit_fn_edges(&f.sig, &owner, path, out);
            }
        }
    }
}

fn emit_fn_edges(sig: &Signature, owner: &str, path: &Path, out: &mut Vec<Edge>) {
    for input in &sig.inputs {
        if let FnArg::Typed(t) = input {
            push_depends_on_from_type(&t.ty, owner, path, out);
        }
    }
    if let ReturnType::Type(_, ty) = &sig.output {
        push_returns_from_type(ty, owner, path, out);
        push_return_inner_as_depends_on(ty, owner, path, out);
    }
}

/// Resolve `Self` against the enclosing impl/trait owner. All other tokens
/// pass through untouched. Used by every edge emitter so `impl Graph { fn
/// empty() -> Self }` produces an edge to `Graph`, not to the unresolved
/// literal `Self`.
fn resolve_self<'a>(head: &'a str, owner: &'a str) -> &'a str {
    if head == "Self" { owner } else { head }
}

/// A `pub fn` returning `Result<Graph, E>` declares a RETURNS edge on the
/// outer head (`Result`) and `DEPENDS_ON` edges on every inner generic
/// argument (`Graph`, `E`). Without the inner pass, the concept graph
/// would lose the fact that the function's result is shaped around those
/// inner types — which is usually the information worth locking down.
fn push_return_inner_as_depends_on(ty: &Type, owner: &str, path: &Path, out: &mut Vec<Edge>) {
    let mut heads: Vec<(String, String)> = Vec::new();
    collect_type_path_heads(ty, &mut heads);
    if heads.is_empty() {
        return;
    }
    // Drop the outermost head — that one is already RETURNS.
    let inner = heads.into_iter().skip(1);
    let line_source = code_source(path, ty.span());
    for (head, raw) in inner {
        if head.is_empty() {
            continue;
        }
        out.push(Edge {
            source_concept: owner.to_string(),
            kind: EdgeKind::DependsOn,
            target: resolve_self(&head, owner).to_string(),
            raw_target: raw,
            source: line_source.clone(),
        });
    }
}

/// `DEPENDS_ON` emits one edge per type-path head discovered recursively
/// inside the type (outermost + all generic inner types). Each head is
/// filtered later by [`filter_by_known_concepts`].
fn push_depends_on_from_type(ty: &Type, owner: &str, path: &Path, out: &mut Vec<Edge>) {
    let mut heads: Vec<(String, String)> = Vec::new();
    collect_type_path_heads(ty, &mut heads);
    let line_source = code_source(path, ty.span());
    for (head, raw) in heads {
        if head.is_empty() {
            continue;
        }
        out.push(Edge {
            source_concept: owner.to_string(),
            kind: EdgeKind::DependsOn,
            target: resolve_self(&head, owner).to_string(),
            raw_target: raw,
            source: line_source.clone(),
        });
    }
}

/// RETURNS emits exactly one edge per `pub fn`: the outermost type head,
/// normalised via [`tokenise_target`]. The raw form preserves the full
/// token stream for display in drift messages.
fn push_returns_from_type(ty: &Type, owner: &str, path: &Path, out: &mut Vec<Edge>) {
    let raw = quote::quote!(#ty).to_string();
    let head = tokenise_target(&raw);
    if head.is_empty() {
        return;
    }
    out.push(Edge {
        source_concept: owner.to_string(),
        kind: EdgeKind::Returns,
        target: resolve_self(&head, owner).to_string(),
        raw_target: raw,
        source: code_source(path, ty.span()),
    });
}

fn impl_target_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(tp) => tp.path.segments.last().map(|s| s.ident.to_string()),
        Type::Reference(r) => impl_target_name(&r.elem),
        _ => None,
    }
}

/// Walk a type expression and push every type-path head encountered.
/// `Result<Graph, E>` → pushes `Result`, `Graph`, `E`. Non-path types
/// (tuples, arrays, references, etc.) are recursed into but do not
/// themselves emit a head.
fn collect_type_path_heads(ty: &Type, out: &mut Vec<(String, String)>) {
    match ty {
        Type::Path(tp) => {
            if let Some(last) = tp.path.segments.last() {
                let head = last.ident.to_string();
                out.push((head.clone(), head));
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    for arg in &args.args {
                        if let GenericArgument::Type(inner) = arg {
                            collect_type_path_heads(inner, out);
                        }
                    }
                }
            }
        }
        Type::Reference(r) => collect_type_path_heads(&r.elem, out),
        Type::Paren(p) => collect_type_path_heads(&p.elem, out),
        Type::Group(g) => collect_type_path_heads(&g.elem, out),
        Type::Slice(s) => collect_type_path_heads(&s.elem, out),
        Type::Array(a) => collect_type_path_heads(&a.elem, out),
        Type::Tuple(t) => {
            for elem in &t.elems {
                collect_type_path_heads(elem, out);
            }
        }
        _ => {}
    }
}

fn code_source(path: &Path, span: Span) -> Source {
    Source::Code {
        path: path.to_path_buf(),
        line: span.start().line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    fn edges_of(src: &str) -> Vec<Edge> {
        let file: syn::File = parse_str(src).expect("test src parses");
        let path = PathBuf::from("test.rs");
        let mut out = Vec::new();
        for item in &file.items {
            emit_for_item(item, &path, &mut out);
        }
        out
    }

    fn nodes(names: &[&str]) -> Vec<ConceptNode> {
        names
            .iter()
            .map(|n| ConceptNode {
                name: (*n).to_string(),
                source: Source::Code {
                    path: PathBuf::from("test.rs"),
                    line: 1,
                },
                signature: domain::SignatureState::Absent,
            })
            .collect()
    }

    #[test]
    fn trait_impl_emits_implements_edge() {
        let edges = edges_of("pub struct Foo; pub trait Bar {} impl Bar for Foo {}");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Foo", "Bar"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Foo"
            && e.kind == EdgeKind::Implements
            && e.target == "Bar"));
    }

    #[test]
    fn inherent_impl_emits_no_implements_edge() {
        let edges = edges_of("pub struct Foo; impl Foo { pub fn new() -> Foo { Foo } }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Foo"]));
        assert!(filtered.iter().all(|e| e.kind != EdgeKind::Implements));
    }

    #[test]
    fn struct_field_emits_depends_on() {
        let edges = edges_of("pub struct Foo; pub struct Bar { pub f: Foo }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Foo", "Bar"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Bar"
            && e.kind == EdgeKind::DependsOn
            && e.target == "Foo"));
    }

    #[test]
    fn enum_variant_field_emits_depends_on() {
        let edges = edges_of("pub struct Foo; pub enum E { V(Foo) }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Foo", "E"]));
        assert!(filtered.iter().any(|e| e.source_concept == "E"
            && e.kind == EdgeKind::DependsOn
            && e.target == "Foo"));
    }

    #[test]
    fn nested_generic_field_emits_multiple_depends_on() {
        let edges = edges_of(
            "pub struct Graph; pub struct Violation; pub struct Holder { pub v: Result<Graph, Violation> }",
        );
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph", "Violation", "Holder"]));
        let holder_deps: Vec<&str> = filtered
            .iter()
            .filter(|e| e.source_concept == "Holder" && e.kind == EdgeKind::DependsOn)
            .map(|e| e.target.as_str())
            .collect();
        assert!(holder_deps.contains(&"Graph"));
        assert!(holder_deps.contains(&"Violation"));
    }

    #[test]
    fn primitive_dependencies_are_filtered_out() {
        let edges = edges_of("pub struct Foo { pub n: u32, pub s: String }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Foo"]));
        assert!(filtered
            .iter()
            .all(|e| e.kind != EdgeKind::DependsOn || e.target == "Foo"));
    }

    #[test]
    fn inherent_impl_pub_fn_returns_emits_returns() {
        let edges = edges_of("pub struct Graph; impl Graph { pub fn empty() -> Graph { Graph } }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Graph"
            && e.kind == EdgeKind::Returns
            && e.target == "Graph"));
    }

    #[test]
    fn inherent_impl_non_pub_fn_is_skipped() {
        let edges = edges_of("pub struct Graph; impl Graph { fn private() -> Graph { Graph } }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph"]));
        assert!(filtered.iter().all(|e| e.kind != EdgeKind::Returns));
    }

    #[test]
    fn trait_method_return_emits_returns() {
        let edges = edges_of("pub struct Graph; pub trait Reader { fn extract(&self) -> Graph; }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph", "Reader"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Reader"
            && e.kind == EdgeKind::Returns
            && e.target == "Graph"));
    }

    #[test]
    fn trait_method_params_emit_depends_on() {
        let edges = edges_of(
            "pub struct Graph; pub trait Reader { fn consume(&self, g: Graph) -> Graph; }",
        );
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph", "Reader"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Reader"
            && e.kind == EdgeKind::DependsOn
            && e.target == "Graph"));
    }

    #[test]
    fn trait_impl_method_bodies_emit_edges_like_inherent_impls() {
        // `impl Reader for Foo { fn extract(&self) -> Graph { Graph } }`
        // emits IMPLEMENTS(Foo, Reader) and RETURNS(Foo, Graph). Spec
        // authors can pin the concrete type's surface independently of
        // the trait's contract.
        let edges = edges_of(
            "pub struct Graph; pub struct Foo; pub trait Reader { fn extract(&self) -> Graph; } impl Reader for Foo { fn extract(&self) -> Graph { Graph } }",
        );
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph", "Foo", "Reader"]));
        let foo_edges: Vec<_> = filtered
            .iter()
            .filter(|e| e.source_concept == "Foo")
            .collect();
        assert!(foo_edges.iter().any(|e| e.kind == EdgeKind::Implements));
        assert!(foo_edges
            .iter()
            .any(|e| e.kind == EdgeKind::Returns && e.target == "Graph"));
    }

    #[test]
    fn return_type_inner_generics_emit_depends_on() {
        // `fn extract() -> Result<Graph, ReaderError>` emits
        // RETURNS(owner, Result) (filtered out — Result not a concept)
        // and `DEPENDS_ON`(owner, Graph), `DEPENDS_ON`(owner, ReaderError).
        let edges = edges_of(
            "pub struct Graph; pub struct ReaderError; pub trait Reader { fn extract(&self) -> Result<Graph, ReaderError>; }",
        );
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph", "ReaderError", "Reader"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Reader"
            && e.kind == EdgeKind::DependsOn
            && e.target == "Graph"));
        assert!(filtered.iter().any(|e| e.source_concept == "Reader"
            && e.kind == EdgeKind::DependsOn
            && e.target == "ReaderError"));
    }

    #[test]
    fn reference_return_tokenises_to_concept_head() {
        let edges = edges_of(
            "pub struct Source; pub struct Node; impl Node { pub fn where_at(&self) -> &Source { unimplemented!() } }",
        );
        let filtered = filter_by_known_concepts(edges, &nodes(&["Source", "Node"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Node"
            && e.kind == EdgeKind::Returns
            && e.target == "Source"));
    }

    #[test]
    fn unknown_targets_are_filtered_out() {
        let edges = edges_of("pub struct Foo { pub x: std::collections::HashMap<String, u32> }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Foo"]));
        // HashMap, String, u32 are not concepts → all filtered out.
        assert!(
            filtered.is_empty()
                || filtered
                    .iter()
                    .all(|e| e.source_concept == "Foo" && e.target == "Foo")
        );
    }

    #[test]
    fn non_public_struct_emits_no_edges() {
        let edges = edges_of("pub struct Target; struct Hidden { f: Target }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Target", "Hidden"]));
        assert!(filtered.iter().all(|e| e.source_concept != "Hidden"));
    }

    #[test]
    fn self_return_resolves_to_enclosing_impl_owner() {
        // `impl Graph { pub fn empty() -> Self }` must emit
        // RETURNS(Graph, Graph), not RETURNS(Graph, Self). Clippy's
        // `use_self` lint pushes authors toward `Self`; the extractor
        // must resolve it to keep the dogfood honest.
        let edges = edges_of("pub struct Graph; impl Graph { pub fn empty() -> Self { Graph } }");
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Graph"
            && e.kind == EdgeKind::Returns
            && e.target == "Graph"));
        assert!(!filtered.iter().any(|e| e.target == "Self"));
    }

    #[test]
    fn self_param_resolves_to_enclosing_owner() {
        let edges = edges_of(
            "pub struct Graph; impl Graph { pub fn merge(&self, other: Self) -> Self { other } }",
        );
        let filtered = filter_by_known_concepts(edges, &nodes(&["Graph"]));
        assert!(filtered.iter().any(|e| e.source_concept == "Graph"
            && e.kind == EdgeKind::DependsOn
            && e.target == "Graph"));
        assert!(!filtered.iter().any(|e| e.target == "Self"));
    }
}

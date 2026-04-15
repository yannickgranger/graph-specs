//! Signature normalisation — the byte-equal comparison target.
//!
//! Both the markdown reader (parsing fenced `rust` blocks from specs) and
//! the Rust reader (parsing source files) funnel through [`normalize`]
//! before emitting a [`domain::SignatureState::Normalized`]. Byte equality
//! of the output string is the signature-match criterion.
//!
//! Normalisation rules (per issue #7):
//!
//! - Strip `#[doc = "..."]` / `///` / `//!`.
//! - Strip `#[cfg(...)]`, `#[must_use]`, `#[inline]`, `#[derive(...)]`.
//! - Collapse visibility refinements other than `pub`: `pub(crate)`,
//!   `pub(super)`, `pub(in ...)` → inherited (non-public marker). The
//!   top-level reader only emits `pub` items, so this mostly matters on
//!   fields and trait items.
//! - Render via `quote!` — this gives consistent token-level whitespace
//!   and trailing-comma handling.
//!
//! The function is a pure transform on a cloned `syn::Item` — the input is
//! not mutated.

use syn::{Attribute, Fields, Item, TraitItem, Visibility};

/// Produce the normalised string form of `item`.
#[must_use]
pub fn normalize(item: &Item) -> String {
    let mut item = item.clone();
    strip_item(&mut item);
    let raw = quote::quote!(#item).to_string();
    collapse_trailing_commas(&raw)
}

/// `quote!` emits tokens space-separated, so a trailing comma inside a
/// group renders as ` , }` or ` , )`. We collapse both to make signatures
/// with and without trailing commas byte-equal.
fn collapse_trailing_commas(s: &str) -> String {
    s.replace(" , }", " }").replace(" , )", " )")
}

fn strip_item(item: &mut Item) {
    match item {
        Item::Struct(s) => {
            strip_attrs(&mut s.attrs);
            normalize_vis(&mut s.vis);
            strip_fields(&mut s.fields);
        }
        Item::Enum(e) => {
            strip_attrs(&mut e.attrs);
            normalize_vis(&mut e.vis);
            for variant in &mut e.variants {
                strip_attrs(&mut variant.attrs);
                strip_fields(&mut variant.fields);
            }
        }
        Item::Trait(t) => {
            strip_attrs(&mut t.attrs);
            normalize_vis(&mut t.vis);
            for ti in &mut t.items {
                strip_trait_item(ti);
            }
        }
        Item::Type(t) => {
            strip_attrs(&mut t.attrs);
            normalize_vis(&mut t.vis);
        }
        _ => {}
    }
}

fn strip_fields(fields: &mut Fields) {
    match fields {
        Fields::Named(n) => {
            for f in &mut n.named {
                strip_attrs(&mut f.attrs);
                normalize_vis(&mut f.vis);
            }
        }
        Fields::Unnamed(u) => {
            for f in &mut u.unnamed {
                strip_attrs(&mut f.attrs);
                normalize_vis(&mut f.vis);
            }
        }
        Fields::Unit => {}
    }
}

fn strip_trait_item(ti: &mut TraitItem) {
    match ti {
        TraitItem::Fn(f) => strip_attrs(&mut f.attrs),
        TraitItem::Type(t) => strip_attrs(&mut t.attrs),
        TraitItem::Const(c) => strip_attrs(&mut c.attrs),
        _ => {}
    }
}

fn strip_attrs(attrs: &mut Vec<Attribute>) {
    attrs.retain(|a| !is_noise_attr(a));
}

fn is_noise_attr(attr: &Attribute) -> bool {
    let p = attr.path();
    p.is_ident("doc")
        || p.is_ident("cfg")
        || p.is_ident("must_use")
        || p.is_ident("inline")
        || p.is_ident("derive")
}

fn normalize_vis(vis: &mut Visibility) {
    if matches!(vis, Visibility::Restricted(_)) {
        *vis = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Item {
        syn::parse_str(s).expect("test input must parse")
    }

    #[test]
    fn identical_items_normalize_identical() {
        let a = normalize(&parse("pub struct Foo(pub u32);"));
        let b = normalize(&parse("pub struct Foo(pub u32);"));
        assert_eq!(a, b);
    }

    #[test]
    fn differing_whitespace_normalizes_identical() {
        let a = normalize(&parse("pub struct Foo(pub u32);"));
        let b = normalize(&parse("pub  struct   Foo  (  pub u32  )  ;"));
        assert_eq!(a, b);
    }

    #[test]
    fn doc_comments_are_stripped() {
        let a = normalize(&parse("pub struct Foo;"));
        let b = normalize(&parse("/// lots of doc\npub struct Foo;"));
        assert_eq!(a, b);
    }

    #[test]
    fn cfg_attributes_are_stripped() {
        let a = normalize(&parse("pub struct Foo;"));
        let b = normalize(&parse("#[cfg(feature = \"x\")] pub struct Foo;"));
        assert_eq!(a, b);
    }

    #[test]
    fn derives_are_stripped() {
        let a = normalize(&parse("pub struct Foo;"));
        let b = normalize(&parse("#[derive(Debug, Clone)] pub struct Foo;"));
        assert_eq!(a, b);
    }

    #[test]
    fn must_use_and_inline_are_stripped() {
        let a = normalize(&parse("pub struct Foo;"));
        let b = normalize(&parse("#[must_use] #[inline] pub struct Foo;"));
        assert_eq!(a, b);
    }

    #[test]
    fn pub_crate_collapses_to_non_pub() {
        let pub_crate = normalize(&parse("pub(crate) struct Foo;"));
        let inherited = normalize(&parse("struct Foo;"));
        assert_eq!(pub_crate, inherited);
    }

    #[test]
    fn pub_stays_pub() {
        let a = normalize(&parse("pub struct Foo;"));
        assert!(a.contains("pub"));
    }

    #[test]
    fn field_doc_comments_are_stripped() {
        let a = normalize(&parse("pub struct Foo { pub bar: u32 }"));
        let b = normalize(&parse(
            "pub struct Foo {\n    /// a field\n    pub bar: u32,\n}",
        ));
        assert_eq!(a, b);
    }

    #[test]
    fn differing_field_types_drift() {
        let a = normalize(&parse("pub struct Foo { pub bar: u32 }"));
        let b = normalize(&parse("pub struct Foo { pub bar: u64 }"));
        assert_ne!(a, b);
    }

    #[test]
    fn enum_variant_docs_are_stripped() {
        let a = normalize(&parse("pub enum E { A, B }"));
        let b = normalize(&parse("pub enum E { /// doc\n A, /// other\n B }"));
        assert_eq!(a, b);
    }

    #[test]
    fn added_enum_variant_drifts() {
        let a = normalize(&parse("pub enum E { A }"));
        let b = normalize(&parse("pub enum E { A, B }"));
        assert_ne!(a, b);
    }

    #[test]
    fn trait_method_doc_comments_are_stripped() {
        let a = normalize(&parse("pub trait T { fn f(&self) -> u32; }"));
        let b = normalize(&parse(
            "pub trait T {\n    /// method doc\n    fn f(&self) -> u32;\n}",
        ));
        assert_eq!(a, b);
    }

    #[test]
    fn generic_bound_change_drifts() {
        let a = normalize(&parse("pub struct Foo<T: Clone>(T);"));
        let b = normalize(&parse("pub struct Foo<T: Copy>(T);"));
        assert_ne!(a, b);
    }
}

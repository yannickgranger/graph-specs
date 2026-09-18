use crate::{
    ConceptNode, ContextDecl, ContextViolation, DeclaredSurface, Edge, EdgeKind, Graph, OwnedUnit,
    Source, Violation,
};
use std::collections::{HashMap, HashSet};

type ImportKey = (String, String, String);
type ExportKey = (String, String);
type NodeIndex = HashMap<String, (Option<String>, Option<String>)>;

pub(super) fn context_pass(
    spec_contexts: Vec<ContextDecl>,
    code: Graph,
    surface: &DeclaredSurface,
    answerable: Option<&[EdgeKind]>,
    out: &mut Vec<Violation>,
) {
    if spec_contexts.is_empty() {
        return;
    }
    let membership = Membership::over(surface);
    let node_index = build_node_index(&code.nodes, &membership);
    let (imports, exports, context_sources) = index_contexts(spec_contexts);

    let Graph {
        nodes: code_nodes,
        edges: code_edges,
    } = code;

    emit_membership_unknown(code_nodes, &membership, out);
    let realised = emit_cross_context_edge_violations(
        code_edges,
        &node_index,
        &membership,
        &imports,
        &exports,
        &context_sources,
        out,
    );
    if answerable.is_some_and(|kinds| kinds.contains(&EdgeKind::Uses)) {
        emit_import_unrealised(&imports, &realised, &context_sources, out);
    }
}

fn emit_import_unrealised(
    imports: &HashSet<ImportKey>,
    realised: &HashSet<ImportKey>,
    context_sources: &HashMap<String, Source>,
    out: &mut Vec<Violation>,
) {
    let mut unrealised: Vec<&ImportKey> = imports.difference(realised).collect();
    unrealised.sort();
    for (owning_context, from_context, concept) in unrealised {
        let Some(spec_source) = context_sources.get(owning_context) else {
            continue;
        };
        out.push(Violation::Context(ContextViolation::ImportUnrealised {
            concept: concept.clone(),
            owning_context: owning_context.clone(),
            from_context: from_context.clone(),
            spec_source: spec_source.clone(),
        }));
    }
}

struct Membership<'a> {
    surface: &'a DeclaredSurface,
}

impl<'a> Membership<'a> {
    const fn over(surface: &'a DeclaredSurface) -> Self {
        Self { surface }
    }

    fn context_of(&self, unit: &str) -> Option<String> {
        self.surface
            .context_of(unit)
            .map(std::borrow::ToOwned::to_owned)
    }
}

fn build_node_index(nodes: &[ConceptNode], membership: &Membership) -> NodeIndex {
    nodes
        .iter()
        .map(|node| {
            let unit = owning_unit_str(&node.source);
            let context = unit.as_ref().and_then(|u| membership.context_of(u));
            (node.name.clone(), (context, unit))
        })
        .collect()
}

fn resolve_endpoint(reference: &mut crate::ConceptRef, index: &NodeIndex, membership: &Membership) {
    if let Some(unit) = &reference.unit {
        reference.context = membership.context_of(&unit.0);
        return;
    }
    if let Some((context, unit)) = index.get(reference.name.as_str()) {
        reference.context.clone_from(context);
        reference.unit = unit.clone().map(OwnedUnit);
    }
}

fn index_contexts(
    contexts: Vec<ContextDecl>,
) -> (
    HashSet<ImportKey>,
    HashSet<ExportKey>,
    HashMap<String, Source>,
) {
    let mut imports = HashSet::new();
    let mut exports = HashSet::new();
    let mut sources = HashMap::new();
    for ctx in contexts {
        absorb_one_context(ctx, &mut imports, &mut exports, &mut sources);
    }
    (imports, exports, sources)
}

fn absorb_one_context(
    ctx: ContextDecl,
    imports: &mut HashSet<ImportKey>,
    exports: &mut HashSet<ExportKey>,
    sources: &mut HashMap<String, Source>,
) {
    let ContextDecl {
        name,
        imports: im_vec,
        exports: ex_vec,
        source,
        ..
    } = ctx;
    imports.extend(
        im_vec
            .into_iter()
            .map(|im| (name.clone(), im.from_context, im.concept)),
    );
    exports.extend(ex_vec.into_iter().map(|ex| (name.clone(), ex.concept)));
    sources.insert(name, source);
}

fn emit_membership_unknown(
    nodes: Vec<ConceptNode>,
    membership: &Membership,
    out: &mut Vec<Violation>,
) {
    for node in nodes {
        let Some(unit_str) = owning_unit_str(&node.source) else {
            continue;
        };
        if membership.context_of(&unit_str).is_some() {
            continue;
        }
        out.push(Violation::Context(ContextViolation::MembershipUnknown {
            concept: node.name,
            owned_unit: OwnedUnit(unit_str),
            code_source: node.source,
        }));
    }
}

fn emit_cross_context_edge_violations(
    code_edges: Vec<Edge>,
    node_index: &NodeIndex,
    membership: &Membership,
    imports: &HashSet<ImportKey>,
    exports: &HashSet<ExportKey>,
    context_sources: &HashMap<String, Source>,
    out: &mut Vec<Violation>,
) -> HashSet<ImportKey> {
    let mut realised = HashSet::new();
    for mut edge in code_edges {
        resolve_endpoint(&mut edge.source_concept, node_index, membership);
        resolve_endpoint(&mut edge.target, node_index, membership);
        if !node_index.contains_key(edge.source_concept.name.as_str()) {
            continue;
        }
        if !node_index.contains_key(edge.target.name.as_str()) {
            out.push(Violation::Context(ContextViolation::CrossEdgeOffSurface {
                concept: edge.source_concept.name.clone(),
                owning_context: edge.source_concept.context.clone(),
                edge_kind: edge.kind,
                target: edge.target.name.clone(),
                code_source: edge.source.clone(),
            }));
            continue;
        }
        let (Some(source_ctx), Some(target_ctx)) = (
            edge.source_concept.context.clone(),
            edge.target.context.clone(),
        ) else {
            continue;
        };
        if source_ctx == target_ctx {
            continue;
        }
        realised.insert((
            source_ctx.clone(),
            target_ctx.clone(),
            edge.target.name.clone(),
        ));
        let Some(spec_source) = context_sources.get(&source_ctx) else {
            continue;
        };
        if let Some(v) = classify_cross_edge(
            &edge,
            &source_ctx,
            &target_ctx,
            imports,
            exports,
            spec_source,
        ) {
            out.push(v);
        }
    }
    realised
}

fn classify_cross_edge(
    edge: &Edge,
    source_ctx: &str,
    target_ctx: &str,
    imports: &HashSet<ImportKey>,
    exports: &HashSet<ExportKey>,
    spec_source: &Source,
) -> Option<Violation> {
    let import_key = (
        source_ctx.to_string(),
        target_ctx.to_string(),
        edge.target.name.clone(),
    );
    if !imports.contains(&import_key) {
        return Some(Violation::Context(
            ContextViolation::CrossEdgeUnauthorized {
                concept: edge.source_concept.name.clone(),
                owning_context: source_ctx.to_string(),
                edge_kind: edge.kind,
                target: edge.target.name.clone(),
                target_context: target_ctx.to_string(),
                spec_source: spec_source.clone(),
            },
        ));
    }
    let export_key = (target_ctx.to_string(), edge.target.name.clone());
    if !exports.contains(&export_key) {
        return Some(Violation::Context(ContextViolation::CrossEdgeUndeclared {
            concept: edge.source_concept.name.clone(),
            owning_context: source_ctx.to_string(),
            edge_kind: edge.kind,
            target: edge.target.name.clone(),
            target_context: target_ctx.to_string(),
            spec_source: spec_source.clone(),
        }));
    }
    None
}

fn owning_unit_str(source: &Source) -> Option<String> {
    if let Some(unit) = source.unit() {
        return Some(unit.to_owned());
    }
    let path = match source {
        Source::Code { path, .. } => path,
        Source::Spec { .. } => return None,
    };
    let path_str = path.to_string_lossy();
    let trimmed = path_str.trim_start_matches("./");
    trimmed
        .split_once("/src/")
        .map(|(unit, _)| unit.to_string())
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;

use crate::{
    ConceptNode, ContextDecl, ContextViolation, Edge, Graph, OwnedUnit, Source, Violation,
};
use std::collections::{HashMap, HashSet};

type ImportKey = (String, String, String);
type ExportKey = (String, String);
type NodeIndex = HashMap<String, (Option<String>, Option<String>)>;

pub(super) fn context_pass(spec_contexts: Vec<ContextDecl>, code: Graph, out: &mut Vec<Violation>) {
    if spec_contexts.is_empty() {
        return;
    }
    let unit_to_context = build_unit_index(&spec_contexts);
    let node_index = build_node_index(&code.nodes, &unit_to_context);
    let (imports, exports, context_sources) = index_contexts(spec_contexts);

    let Graph {
        nodes: code_nodes,
        edges: code_edges,
    } = code;

    emit_membership_unknown(code_nodes, &unit_to_context, out);
    emit_cross_context_edge_violations(
        code_edges,
        &node_index,
        &imports,
        &exports,
        &context_sources,
        out,
    );
}

fn build_unit_index(contexts: &[ContextDecl]) -> HashMap<String, String> {
    contexts
        .iter()
        .flat_map(|ctx| {
            let name = ctx.name.as_str();
            ctx.owned_units
                .iter()
                .map(move |u| (u.0.clone(), name.to_owned()))
        })
        .collect()
}

fn build_node_index(nodes: &[ConceptNode], unit_to_context: &HashMap<String, String>) -> NodeIndex {
    nodes
        .iter()
        .map(|node| {
            let unit = owning_unit_str(&node.source);
            let context = unit.as_ref().and_then(|u| unit_to_context.get(u)).cloned();
            (node.name.clone(), (context, unit))
        })
        .collect()
}

fn resolve_endpoint(reference: &mut crate::ConceptRef, index: &NodeIndex) {
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
    unit_to_context: &HashMap<String, String>,
    out: &mut Vec<Violation>,
) {
    for node in nodes {
        let Some(unit_str) = owning_unit_str(&node.source) else {
            continue;
        };
        if unit_to_context.contains_key(&unit_str) {
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
    imports: &HashSet<ImportKey>,
    exports: &HashSet<ExportKey>,
    context_sources: &HashMap<String, Source>,
    out: &mut Vec<Violation>,
) {
    for mut edge in code_edges {
        resolve_endpoint(&mut edge.source_concept, node_index);
        resolve_endpoint(&mut edge.target, node_index);
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

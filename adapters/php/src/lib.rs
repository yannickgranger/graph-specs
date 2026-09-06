use domain::{
    ConceptNode, ConceptRef, Edge, EdgeKind, Graph, SignatureState, Source, SpecFormat, Violation,
};
use ports::{ReaderError, SpecFileSet, SpecReader};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

pub const ACCEPTED_KEYS: [&str; 3] = ["implements", "extends", "signature"];

const ATTRIBUTE: &str = "Spec";

const CONCEPT_CONSTRUCTS: [&str; 3] = [
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
];

pub struct PhpAttributeReader;

impl PhpAttributeReader {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for PhpAttributeReader {
    fn default() -> Self {
        Self::new()
    }
}

struct Attributed {
    concept: String,
    line: usize,
    keys: Vec<(String, String, usize)>,
}

pub struct PhpSignatures;

impl ports::SignatureNormalizer for PhpSignatures {
    fn fence_tag(&self) -> &'static str {
        "php"
    }

    fn normalize(&self, block: &str) -> Result<String, String> {
        let mut parser = parser().map_err(|e| e.to_string())?;
        let tagged = if block.trim_start().starts_with("<?php") {
            block.to_owned()
        } else {
            format!("<?php\n{block}")
        };
        let tree = parser
            .parse(&tagged, None)
            .ok_or_else(|| "the php grammar returned no tree".to_string())?;
        let root = tree.root_node();
        if root.has_error() {
            return Err("the php grammar could not parse the block".to_string());
        }
        let src = tagged.as_bytes();
        let mut declarations = Vec::new();
        collect_declarations(root, src, &mut declarations);
        match declarations.as_slice() {
            [] => Err("the block declares nothing the php grammar recognises".to_string()),
            [only] => Ok(only.clone()),
            many => Err(format!(
                "the block declares {} constructs; a signature block declares one",
                many.len()
            )),
        }
    }
}

fn collect_declarations(node: Node, src: &[u8], out: &mut Vec<String>) {
    if CONCEPT_CONSTRUCTS.contains(&node.kind()) || node.kind() == "method_declaration" {
        out.push(reprinted(node, src));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_declarations(child, src, out);
    }
}

fn reprinted(node: Node, src: &[u8]) -> String {
    let mut tokens = Vec::new();
    tokens_of(node, src, &mut tokens);
    tokens.join(" ")
}

fn tokens_of(node: Node, src: &[u8], out: &mut Vec<String>) {
    let kind = node.kind();
    if kind == "comment" || kind == "attribute_list" || kind == "declaration_list" {
        return;
    }
    if node.child_count() == 0 {
        if let Ok(text) = node.utf8_text(src) {
            if !text.trim().is_empty() {
                out.push(text.to_owned());
            }
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        tokens_of(child, src, out);
    }
}

impl ports::SpecLoader for PhpAttributeReader {
    fn load(&self, root: &Path) -> Result<SpecFileSet, ReaderError> {
        Ok(SpecFileSet::new(
            walk(root)?
                .into_iter()
                .map(|(path, text)| ports::LoadedFile { path, text })
                .collect(),
        ))
    }
}

impl SpecReader for PhpAttributeReader {
    fn extract(&self, files: &SpecFileSet) -> Result<Graph, ReaderError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for (path, source) in files
            .files()
            .iter()
            .filter(|f| f.path.extension().and_then(|e| e.to_str()) == Some("php"))
            .map(|f| (f.path.clone(), f.text.clone()))
        {
            for a in attributed(&source)? {
                let site = Source::Spec {
                    path: path.clone(),
                    line: a.line,
                    context: None,
                    format: SpecFormat::InlineAttribute,
                };
                let signature = a
                    .keys
                    .iter()
                    .find(|(k, _, _)| k == "signature")
                    .map_or(SignatureState::Absent, |(_, v, _)| {
                        SignatureState::Normalized(v.clone())
                    });
                nodes.push(ConceptNode::new(a.concept.clone(), site.clone(), signature));
                for (key, value, _) in &a.keys {
                    if key == "implements" {
                        edges.push(Edge {
                            source_concept: ConceptRef::named(a.concept.clone()),
                            kind: EdgeKind::Implements,
                            target: ConceptRef::named(value.clone()),
                            raw_target: value.clone(),
                            source: site.clone(),
                        });
                    }
                }
            }
        }
        Ok(Graph::new(nodes, edges))
    }
}

impl PhpAttributeReader {
    pub fn extract_findings(&self, root: &Path) -> Result<Vec<Violation>, ReaderError> {
        let mut findings = Vec::new();
        for (path, source) in walk(root)? {
            for a in attributed(&source)? {
                for (key, _, line) in &a.keys {
                    if !ACCEPTED_KEYS.contains(&key.as_str()) {
                        findings.push(Violation::UnknownAttributeKey {
                            concept: a.concept.clone(),
                            key: key.clone(),
                            spec_source: Source::Spec {
                                path: path.clone(),
                                line: *line,
                                context: None,
                                format: SpecFormat::InlineAttribute,
                            },
                        });
                    }
                }
            }
        }
        Ok(findings)
    }
}

fn parser() -> Result<Parser, ReaderError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
        .map_err(|e| ReaderError::ParseFailed {
            path: PathBuf::from("<php grammar>"),
            line: 0,
            message: e.to_string(),
        })?;
    Ok(parser)
}

fn attributed(source: &str) -> Result<Vec<Attributed>, ReaderError> {
    let mut parser = parser()?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| ReaderError::ParseFailed {
            path: PathBuf::from("<php source>"),
            line: 0,
            message: "the php grammar returned no tree".to_string(),
        })?;
    let mut out = Vec::new();
    collect(tree.root_node(), source.as_bytes(), &mut out);
    Ok(out)
}

fn collect(node: Node, src: &[u8], out: &mut Vec<Attributed>) {
    if CONCEPT_CONSTRUCTS.contains(&node.kind()) {
        if let (Some(name), Some(keys)) = (
            node.child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(str::to_owned),
            spec_keys(node, src),
        ) {
            out.push(Attributed {
                concept: name,
                line: node.start_position().row + 1,
                keys,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, src, out);
    }
}

fn spec_keys(node: Node, src: &[u8]) -> Option<Vec<(String, String, usize)>> {
    let list = node.child_by_field_name("attributes")?;
    let mut cursor = list.walk();
    for group in list.children(&mut cursor) {
        let mut group_cursor = group.walk();
        for attribute in group.children(&mut group_cursor) {
            if attribute.kind() != "attribute" {
                continue;
            }
            let named = attribute
                .child(0)
                .and_then(|n| n.utf8_text(src).ok())
                .unwrap_or_default();
            if named != ATTRIBUTE {
                continue;
            }
            return Some(arguments(attribute, src));
        }
    }
    None
}

fn arguments(attribute: Node, src: &[u8]) -> Vec<(String, String, usize)> {
    let Some(arguments) = attribute.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cursor = arguments.walk();
    for argument in arguments.children(&mut cursor) {
        if argument.kind() != "argument" {
            continue;
        }
        let Some(key) = argument
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(src).ok())
        else {
            continue;
        };
        let value = argument
            .named_children(&mut argument.walk())
            .find(|n| n.kind() == "encapsed_string" || n.kind() == "string")
            .and_then(|n| n.utf8_text(src).ok())
            .map(|raw| raw.trim_matches(|c| c == '"' || c == '\'').to_owned())
            .unwrap_or_default();
        out.push((key.to_owned(), value, argument.start_position().row + 1));
    }
    out
}

fn walk(root: &Path) -> Result<Vec<(PathBuf, String)>, ReaderError> {
    let mut out = Vec::new();
    visit(root, &mut out)?;
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn visit(dir: &Path, out: &mut Vec<(PathBuf, String)>) -> Result<(), ReaderError> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "vendor" || name == "node_modules" || name.starts_with('.') {
                continue;
            }
            visit(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("php") {
            let source = std::fs::read_to_string(&path).map_err(|e| ReaderError::IoFailed {
                path: path.clone(),
                cause: e.to_string(),
            })?;
            out.push((path, source));
        }
    }
    Ok(())
}

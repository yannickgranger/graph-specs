use super::*;
use crate::{OwnedUnit, Source};
use std::path::PathBuf;

fn context(units: &[&str]) -> ContextDecl {
    ContextDecl::new(
        "ctx".to_string(),
        units.iter().map(|u| OwnedUnit((*u).to_string())).collect(),
        Vec::new(),
        Vec::new(),
        Source::Spec {
            path: PathBuf::from("specs/contexts/ctx.md"),
            line: 1,
            context: None,
        },
    )
}

#[test]
fn a_declared_prefix_admits_a_name_beneath_it() {
    let surface = DeclaredSurface::from_contexts(&[context(&["App\\Catalogue"])])
        .expect("no nested prefixes across contexts");
    assert!(surface.admits("App\\Catalogue\\Domain\\Course"));
    assert_eq!(
        surface.unit_of("App\\Catalogue\\Domain\\Course"),
        Some("App\\Catalogue")
    );
}

#[test]
fn a_prefix_binds_only_on_a_separator_boundary() {
    let surface = DeclaredSurface::from_contexts(&[context(&["App\\Catalog"])])
        .expect("no nested prefixes across contexts");
    assert!(!surface.admits("App\\Catalogue\\Domain\\Course"));
}

#[test]
fn a_name_outside_every_prefix_is_not_admitted() {
    let surface = DeclaredSurface::from_contexts(&[context(&["App\\Catalogue"])])
        .expect("no nested prefixes across contexts");
    assert!(!surface.admits("App\\Marketing\\Flyer"));
    assert_eq!(surface.unit_of("App\\Marketing\\Flyer"), None);
}

#[test]
fn the_prefix_itself_is_admitted() {
    let surface = DeclaredSurface::from_contexts(&[context(&["App\\Catalogue"])])
        .expect("no nested prefixes across contexts");
    assert!(surface.admits("App\\Catalogue"));
}

#[test]
fn one_leading_backslash_is_immaterial_on_either_side() {
    let declared = DeclaredSurface::from_contexts(&[context(&["\\App\\Catalogue"])])
        .expect("no nested prefixes across contexts");
    assert!(declared.admits("App\\Catalogue\\Course"));
    let plain = DeclaredSurface::from_contexts(&[context(&["App\\Catalogue"])])
        .expect("no nested prefixes across contexts");
    assert!(plain.admits("\\App\\Catalogue\\Course"));
}

#[test]
fn the_longest_declared_prefix_wins() {
    let surface =
        DeclaredSurface::from_contexts(&[context(&["App\\Catalogue", "App\\Catalogue\\Domain"])])
            .expect("no nested prefixes across contexts");
    assert_eq!(
        surface.unit_of("App\\Catalogue\\Domain\\Course"),
        Some("App\\Catalogue\\Domain")
    );
}

#[test]
fn a_rust_owned_unit_binds_on_its_own_separator() {
    let surface = DeclaredSurface::from_contexts(&[context(&["domain"])])
        .expect("no nested prefixes across contexts");
    assert!(surface.admits("domain::Thing"));
    assert!(!surface.admits("domainx::Thing"));
}

#[test]
fn a_repository_declaring_no_context_has_an_empty_surface() {
    let surface = DeclaredSurface::from_contexts(&[]).expect("no nested prefixes across contexts");
    assert!(surface.is_empty());
    assert!(!surface.admits("App\\Catalogue\\Course"));
}

#[test]
fn one_trailing_backslash_on_the_declaration_is_immaterial() {
    let surface = DeclaredSurface::from_contexts(&[context(&["App\\Catalogue\\"])])
        .expect("no nested prefixes across contexts");
    assert!(surface.admits("App\\Catalogue\\Domain\\Course"));
    assert_eq!(
        surface.unit_of("App\\Catalogue\\Domain\\Course"),
        Some("App\\Catalogue")
    );
}

#[test]
fn a_prefix_matches_case_insensitively_because_php_namespaces_do() {
    let surface = DeclaredSurface::from_contexts(&[context(&["App\\Catalogue"])])
        .expect("no nested prefixes across contexts");
    assert!(surface.admits("app\\catalogue\\Domain\\Course"));
    assert!(surface.admits("APP\\CATALOGUE\\Course"));
    assert_eq!(
        surface.unit_of("app\\catalogue\\Course"),
        Some("App\\Catalogue"),
        "the declared spelling is what the unit reports, not the call site's"
    );
}

#[test]
fn case_insensitivity_does_not_cross_a_separator_boundary() {
    let surface = DeclaredSurface::from_contexts(&[context(&["App\\Catalog"])])
        .expect("no nested prefixes across contexts");
    assert!(!surface.admits("app\\catalogue\\Course"));
}

fn named_context(name: &str, units: &[&str]) -> ContextDecl {
    ContextDecl::new(
        name.to_string(),
        units.iter().map(|u| OwnedUnit((*u).to_string())).collect(),
        Vec::new(),
        Vec::new(),
        Source::Spec {
            path: PathBuf::from("specs/contexts/x.md"),
            line: 1,
            context: None,
        },
    )
}

#[test]
fn two_contexts_declaring_nested_prefixes_are_an_ownership_ambiguity() {
    let err = DeclaredSurface::from_contexts(&[
        named_context("catalogue", &["App\\Catalogue"]),
        named_context("enrolment", &["App\\Catalogue\\Enrolment"]),
    ])
    .expect_err("nested prefixes across two contexts have no single owner");
    assert_eq!(err.outer.0, "App\\Catalogue");
    assert_eq!(err.outer_context, "catalogue");
    assert_eq!(err.inner.0, "App\\Catalogue\\Enrolment");
    assert_eq!(err.inner_context, "enrolment");
}

#[test]
fn nested_prefixes_inside_one_owns_block_still_resolve_by_length() {
    let surface = DeclaredSurface::from_contexts(&[named_context(
        "catalogue",
        &["App\\Catalogue", "App\\Catalogue\\Domain"],
    )])
    .expect("one context may nest its own prefixes");
    assert_eq!(
        surface.unit_of("App\\Catalogue\\Domain\\Course"),
        Some("App\\Catalogue\\Domain")
    );
}

#[test]
fn two_contexts_owning_unrelated_prefixes_are_not_ambiguous() {
    DeclaredSurface::from_contexts(&[
        named_context("catalogue", &["App\\Catalogue"]),
        named_context("privacy", &["App\\Privacy"]),
    ])
    .expect("sibling prefixes are not nested");
}

#[test]
fn the_declarations_are_not_reachable_from_outside_the_domain() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("domain/src/context/check_input.rs"),
    )
    .expect("read the check input");

    assert!(
        source.contains("pub(crate) contexts: Vec<ContextDecl>,"),
        "the declarations the surface was derived from are crate-private, so no caller outside \
         domain can push a context after construction and leave the surface describing a set the \
         input no longer carries"
    );
    assert!(
        !source.contains("pub contexts:"),
        "and the field is not published under any other spelling"
    );
    assert!(
        source.contains("surface: DeclaredSurface,") && !source.contains("pub surface:"),
        "the surface it was derived from stays private with it"
    );
}

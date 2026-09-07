#[test]
fn the_declarations_and_the_surface_cannot_be_reached_from_outside_the_domain() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile-fail/check_input_struct_expression.rs");
    t.compile_fail("tests/compile-fail/check_input_field_mutation.rs");
}

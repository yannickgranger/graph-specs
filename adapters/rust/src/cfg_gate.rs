use syn::Attribute;

pub fn is_test_gated(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_cfg_test_gate)
}

fn is_cfg_test_gate(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let mut gated = false;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("test") {
            gated = true;
        }
        if meta.path.is_ident("feature") && feature_value_contains_test(&meta) {
            gated = true;
        }
        Ok(())
    });
    gated
}

fn feature_value_contains_test(meta: &syn::meta::ParseNestedMeta<'_>) -> bool {
    meta.value()
        .ok()
        .and_then(|value| value.parse::<syn::LitStr>().ok())
        .is_some_and(|lit| lit.value().contains("test"))
}

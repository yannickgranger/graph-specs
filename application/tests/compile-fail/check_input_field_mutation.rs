fn main() {
    let mut input = domain::CheckInput::new(
        domain::Graph::default(),
        Vec::new(),
        domain::VerbOwnership::default(),
    )
    .expect("no contexts declare one surface");
    input.contexts.push(unimplemented!());
}

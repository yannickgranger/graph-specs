fn main() {
    let _ = domain::CheckInput {
        graph: domain::Graph::default(),
        contexts: Vec::new(),
        ..Default::default()
    };
}

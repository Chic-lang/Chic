#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Lightweight graph lowering scaffold for LLVM backend tests.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub op: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[must_use]
pub fn lower_graph(name: &str, nodes: &[GraphNode]) -> Vec<String> {
    let mut ordered = Vec::new();
    ordered.push(format!("graph_begin {name}"));
    for node in nodes {
        ordered.push(format!(
            "node {} op={} inputs={:?} outputs={:?}",
            node.id, node.op, node.inputs, node.outputs
        ));
    }
    ordered.push(format!("graph_end {name}"));
    ordered
}

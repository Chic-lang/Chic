#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::pedantic
)]

/// Pseudo-lowering for graphs in the WASM backend.
#[derive(Debug, Clone)]
pub struct WasmGraphNode {
    pub id: String,
    pub op: String,
}

#[must_use]
pub fn lower_graph(name: &str, nodes: &[WasmGraphNode]) -> Vec<String> {
    let mut ops = Vec::new();
    ops.push(format!("graph_begin {name}"));
    for node in nodes {
        ops.push(format!("dispatch {}", node.op));
    }
    ops.push(format!("graph_end {name}"));
    ops
}

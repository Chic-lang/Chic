use chic::codegen::llvm::graphs::{GraphNode, lower_graph as lower_llvm_graph};
use chic::codegen::wasm::graphs::{WasmGraphNode, lower_graph as lower_wasm_graph};
use chic::driver::graph_registry::GraphRegistry;

#[test]
fn llvm_graph_lowering_preserves_order() {
    let nodes = vec![
        GraphNode {
            id: "n0".into(),
            op: "matmul".into(),
            inputs: vec!["a".into(), "b".into()],
            outputs: vec!["c".into()],
        },
        GraphNode {
            id: "n1".into(),
            op: "relu".into(),
            inputs: vec!["c".into()],
            outputs: vec!["d".into()],
        },
    ];
    let lowered = lower_llvm_graph("g0", &nodes);
    assert_eq!(lowered[0], "graph_begin g0");
    assert_eq!(lowered.last().unwrap(), "graph_end g0");
    assert!(
        lowered.iter().any(|l| l.contains("node n1 op=relu")),
        "should include second node"
    );
}

#[test]
fn wasm_graph_lowering_dispatches_nodes() {
    let nodes = vec![
        WasmGraphNode {
            id: "n0".into(),
            op: "add".into(),
        },
        WasmGraphNode {
            id: "n1".into(),
            op: "mul".into(),
        },
    ];
    let lowered = lower_wasm_graph("wg", &nodes);
    assert_eq!(lowered.len(), 4);
    assert!(lowered[1].contains("add") && lowered[2].contains("mul"));
}

#[test]
fn graph_registry_returns_registered_sequence() {
    let mut registry = GraphRegistry::default();
    registry.register("g1", vec!["a".into(), "b".into()]);
    let seq = registry.lookup("g1").unwrap();
    assert_eq!(seq, ["a".to_string(), "b".to_string()]);
}

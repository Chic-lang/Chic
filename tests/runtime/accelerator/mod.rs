use chic::mir::{StreamMetadata, TypeLayoutTable};

#[test]
fn accelerator_types_exist_in_layout_table() {
    let layouts = TypeLayoutTable::default();
    assert!(
        layouts
            .layout_for_name("Std::Accelerator::Stream")
            .is_some(),
        "missing stream layout"
    );
    assert!(
        layouts.layout_for_name("Std::Accelerator::Event").is_some(),
        "missing event layout"
    );
    assert!(
        layouts.layout_for_name("Std::Accelerator::Host").is_some(),
        "missing host memspace layout"
    );
    assert!(
        layouts
            .layout_for_name("Std::Accelerator::PinnedHost")
            .is_some(),
        "missing pinned host layout"
    );
}

#[test]
fn mir_bodies_start_without_stream_metadata() {
    let body = chic::mir::MirBody::new(0, None);
    assert!(body.stream_metadata.is_empty());
    let meta = StreamMetadata {
        local: chic::mir::LocalId(0),
        mem_space: None,
        stream_id: 0,
    };
    assert_eq!(meta.stream_id, 0);
}

use crate::mir::builder::accelerator::AcceleratorBuilder;
use crate::mir::data::{
    AcceleratorCopyKind, BasicBlock, BlockId, ConstOperand, ConstValue, LocalDecl, LocalId,
    LocalKind, MirBody, Operand, Place, StatementKind, Ty,
};

fn body_with_streams() -> (MirBody, LocalId, LocalId) {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let stream_a = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("stream_a".into()),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::Host>"),
        true,
        None,
        LocalKind::Local,
    ));
    let stream_b = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("stream_b".into()),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::PinnedHost>"),
        true,
        None,
        LocalKind::Local,
    ));
    (body, stream_a, stream_b)
}

#[test]
fn register_stream_reuses_existing_id() {
    let (mut body, stream_a, _) = body_with_streams();
    let mut builder = AcceleratorBuilder::new(&mut body);
    let first = builder.register_stream(stream_a, Some(Ty::named("Std::Accelerator::Host")));
    let second = builder.register_stream(stream_a, None);
    assert_eq!(first, second);
    assert_eq!(body.stream_metadata.len(), 1);
    assert_eq!(body.stream_metadata[0].stream_id, first);
    assert_eq!(
        body.stream_metadata[0].mem_space,
        Some(Ty::named("Std::Accelerator::Host"))
    );
}

#[test]
fn enqueue_records_metadata_for_multiple_streams() {
    let (mut body, stream_a, stream_b) = body_with_streams();
    let event_a = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("event_a".into()),
        Ty::named("Std::Accelerator::Event"),
        true,
        None,
        LocalKind::Local,
    ));
    let event_b = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("event_b".into()),
        Ty::named("Std::Accelerator::Event"),
        true,
        None,
        LocalKind::Local,
    ));
    let dst = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("dst".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    let src = LocalId(body.locals.len());
    body.locals.push(LocalDecl::new(
        Some("src".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    {
        let mut builder = AcceleratorBuilder::new(&mut body);
        builder.enqueue_kernel(
            &mut block,
            Place::new(stream_a),
            Operand::Const(ConstOperand::new(ConstValue::Symbol("Demo::Kernel".into()))),
            vec![Operand::Const(ConstOperand::new(ConstValue::Int(1)))],
            Some(Place::new(event_a)),
            None,
        );
        builder.enqueue_copy(
            &mut block,
            Place::new(stream_b),
            Place::new(dst),
            Place::new(src),
            Operand::Const(ConstOperand::new(ConstValue::Int(16))),
            AcceleratorCopyKind::DeviceToDevice,
            Some(Place::new(event_b)),
            None,
        );
    }

    assert_eq!(body.stream_metadata.len(), 2);
    assert!(matches!(
        block.statements.get(0).map(|stmt| &stmt.kind),
        Some(StatementKind::EnqueueKernel { .. })
    ));
    assert!(matches!(
        block.statements.get(1).map(|stmt| &stmt.kind),
        Some(StatementKind::EnqueueCopy { .. })
    ));
}

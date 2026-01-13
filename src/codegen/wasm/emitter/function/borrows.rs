use std::collections::{HashMap, HashSet, VecDeque};

use crate::codegen::wasm::layout::local_requires_memory;
use crate::mir::{
    BorrowId, BorrowKind, BorrowOperand, InlineAsmOperandKind, MirFunction, Operand, Place, Rvalue,
    StatementKind, Terminator, TypeLayoutTable,
};

#[derive(Clone)]
pub(super) struct BorrowMeta {
    pub(super) borrow_id: BorrowId,
    pub(super) kind: BorrowKind,
    pub(super) place: Place,
}

pub(super) fn collect_borrow_metadata(
    function: &MirFunction,
    layouts: &TypeLayoutTable,
) -> (
    HashSet<usize>,
    HashMap<usize, BorrowMeta>,
    HashMap<usize, BorrowMeta>,
) {
    let mut address_taken = HashSet::new();
    let mut borrow_destinations = HashMap::new();
    let mut borrow_regions = HashMap::new();
    let mut pending: VecDeque<BorrowMeta> = VecDeque::new();

    for block in &function.body.blocks {
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Borrow {
                    borrow_id,
                    kind,
                    place,
                    ..
                } => {
                    address_taken.insert(place.local.0);
                    pending.push_back(BorrowMeta {
                        borrow_id: *borrow_id,
                        kind: *kind,
                        place: place.clone(),
                    });
                }
                StatementKind::Assign { place, value } => match value {
                    Rvalue::Use(Operand::Borrow(op)) => {
                        address_taken.insert(op.place.local.0);
                        let meta = match_pending_borrow(&mut pending, op);
                        if place.projection.is_empty() {
                            if let Some(info) = meta {
                                borrow_destinations.insert(place.local.0, info);
                            }
                        }
                    }
                    _ => {
                        record_address_taken_in_rvalue(
                            value,
                            &mut address_taken,
                            &mut pending,
                            &mut borrow_regions,
                        );
                    }
                },
                StatementKind::DeferDrop { place }
                | StatementKind::Drop { place, .. }
                | StatementKind::Deinit(place) => {
                    address_taken.insert(place.local.0);
                }
                StatementKind::DefaultInit { place } => {
                    address_taken.insert(place.local.0);
                }
                StatementKind::ZeroInit { place } => {
                    address_taken.insert(place.local.0);
                }
                StatementKind::ZeroInitRaw { pointer, length } => {
                    record_address_taken_in_operand(
                        pointer,
                        &mut address_taken,
                        &mut pending,
                        &mut borrow_regions,
                    );
                    record_address_taken_in_operand(
                        length,
                        &mut address_taken,
                        &mut pending,
                        &mut borrow_regions,
                    );
                }
                StatementKind::Retag { place } => {
                    address_taken.insert(place.local.0);
                }
                StatementKind::MmioStore { value, .. }
                | StatementKind::StaticStore { value, .. } => {
                    record_address_taken_in_operand(
                        value,
                        &mut address_taken,
                        &mut pending,
                        &mut borrow_regions,
                    );
                }
                StatementKind::InlineAsm(asm) => {
                    for operand in &asm.operands {
                        match &operand.kind {
                            InlineAsmOperandKind::In { value }
                            | InlineAsmOperandKind::Const { value } => {
                                record_address_taken_in_operand(
                                    value,
                                    &mut address_taken,
                                    &mut pending,
                                    &mut borrow_regions,
                                );
                            }
                            InlineAsmOperandKind::InOut { input, output, .. } => {
                                record_address_taken_in_operand(
                                    input,
                                    &mut address_taken,
                                    &mut pending,
                                    &mut borrow_regions,
                                );
                                address_taken.insert(output.local.0);
                            }
                            InlineAsmOperandKind::Out { place, .. } => {
                                address_taken.insert(place.local.0);
                            }
                            InlineAsmOperandKind::Sym { .. } => {}
                        }
                    }
                }
                StatementKind::AtomicStore { value, .. } => {
                    record_address_taken_in_operand(
                        value,
                        &mut address_taken,
                        &mut pending,
                        &mut borrow_regions,
                    );
                }
                StatementKind::AtomicFence { .. } => {}
                StatementKind::Assert { cond, .. } => {
                    record_address_taken_in_operand(
                        cond,
                        &mut address_taken,
                        &mut pending,
                        &mut borrow_regions,
                    );
                }
                StatementKind::EnqueueKernel { .. }
                | StatementKind::EnqueueCopy { .. }
                | StatementKind::RecordEvent { .. }
                | StatementKind::WaitEvent { .. } => {}
                StatementKind::Eval(_) => {}
                StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::EnterUnsafe
                | StatementKind::ExitUnsafe
                | StatementKind::Pending(_)
                | StatementKind::MarkFallibleHandled { .. }
                | StatementKind::Nop => {}
            }
        }

        if let Some(term) = &block.terminator {
            record_address_taken_in_terminator(
                term,
                function,
                layouts,
                &mut address_taken,
                &mut pending,
                &mut borrow_regions,
            );
        }
    }

    (address_taken, borrow_destinations, borrow_regions)
}

fn record_address_taken_in_operand(
    operand: &Operand,
    address_taken: &mut HashSet<usize>,
    pending: &mut VecDeque<BorrowMeta>,
    borrow_regions: &mut HashMap<usize, BorrowMeta>,
) {
    if let Operand::Borrow(borrow) = operand {
        address_taken.insert(borrow.place.local.0);
        if let Some(meta) = match_pending_borrow(pending, borrow) {
            borrow_regions.insert(borrow.region.0, meta);
        }
    }
}

fn record_address_taken_in_operands(
    operands: &[Operand],
    address_taken: &mut HashSet<usize>,
    pending: &mut VecDeque<BorrowMeta>,
    borrow_regions: &mut HashMap<usize, BorrowMeta>,
) {
    for operand in operands {
        record_address_taken_in_operand(operand, address_taken, pending, borrow_regions);
    }
}

fn record_address_taken_in_rvalue(
    value: &Rvalue,
    address_taken: &mut HashSet<usize>,
    pending: &mut VecDeque<BorrowMeta>,
    borrow_regions: &mut HashMap<usize, BorrowMeta>,
) {
    match value {
        Rvalue::Use(operand) => {
            record_address_taken_in_operand(operand, address_taken, pending, borrow_regions)
        }
        Rvalue::Aggregate { fields, .. } => {
            record_address_taken_in_operands(fields, address_taken, pending, borrow_regions);
        }
        Rvalue::AddressOf { place, .. } => {
            address_taken.insert(place.local.0);
        }
        Rvalue::Unary { operand, .. } => {
            record_address_taken_in_operand(operand, address_taken, pending, borrow_regions);
        }
        Rvalue::Binary { lhs, rhs, .. } => {
            record_address_taken_in_operand(lhs, address_taken, pending, borrow_regions);
            record_address_taken_in_operand(rhs, address_taken, pending, borrow_regions);
        }
        Rvalue::Cast { operand, .. } => {
            record_address_taken_in_operand(operand, address_taken, pending, borrow_regions);
        }
        Rvalue::AtomicRmw { value, .. } => {
            record_address_taken_in_operand(value, address_taken, pending, borrow_regions);
        }
        Rvalue::AtomicCompareExchange {
            expected, desired, ..
        } => {
            record_address_taken_in_operand(expected, address_taken, pending, borrow_regions);
            record_address_taken_in_operand(desired, address_taken, pending, borrow_regions);
        }
        _ => {}
    }
}

fn record_address_taken_in_terminator(
    term: &Terminator,
    function: &MirFunction,
    layouts: &TypeLayoutTable,
    address_taken: &mut HashSet<usize>,
    pending: &mut VecDeque<BorrowMeta>,
    borrow_regions: &mut HashMap<usize, BorrowMeta>,
) {
    match term {
        Terminator::Call {
            args, destination, ..
        } => {
            record_address_taken_in_operands(args, address_taken, pending, borrow_regions);
            if let Some(place) = destination {
                let local_ty = &function.body.locals[place.local.0].ty;
                if local_requires_memory(local_ty, layouts) {
                    address_taken.insert(place.local.0);
                }
            }
        }
        Terminator::Yield { value, .. } => {
            record_address_taken_in_operand(value, address_taken, pending, borrow_regions);
        }
        Terminator::Await { destination, .. } => {
            if let Some(dest) = destination {
                address_taken.insert(dest.local.0);
            }
        }
        Terminator::Match { value, .. } => {
            address_taken.insert(value.local.0);
        }
        Terminator::SwitchInt { discr, .. } => {
            record_address_taken_in_operand(discr, address_taken, pending, borrow_regions);
        }
        Terminator::Pending(_) => {}
        _ => {}
    }
}

fn match_pending_borrow(
    pending: &mut VecDeque<BorrowMeta>,
    borrow: &BorrowOperand,
) -> Option<BorrowMeta> {
    if let Some(index) = pending
        .iter()
        .position(|meta| meta.kind == borrow.kind && same_place(&meta.place, &borrow.place))
    {
        return pending.remove(index);
    }
    None
}

fn same_place(lhs: &Place, rhs: &Place) -> bool {
    lhs.local == rhs.local && lhs.projection == rhs.projection
}

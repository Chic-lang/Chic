use std::collections::HashMap;

use crate::mir::{
    ConstValue, DefaultArgumentKind, InlineAsm, InlineAsmOperandKind, InternedStr,
    InterpolatedStringSegment, MirBody, MirModule, Operand, Pattern, PatternField,
    PendingOperandInfo, Rvalue, StatementKind, StrId, StrLifetime, Terminator,
    VariantPatternFields,
};

pub fn intern_raw_strings(module: &mut MirModule) {
    let mut interner = RawStringInterner::new(&mut module.interned_strs);
    for function in &mut module.functions {
        intern_raw_strings_in_body(&mut function.body, &mut interner);
    }
    for static_var in &mut module.statics {
        if let Some(initializer) = static_var.initializer.as_mut() {
            intern_raw_strings_in_const(initializer, &mut interner);
        }
    }
    for default in &mut module.default_arguments {
        if let DefaultArgumentKind::Const(value) = &mut default.value {
            intern_raw_strings_in_const(value, &mut interner);
        }
    }
}

struct RawStringInterner<'a> {
    interned: &'a mut Vec<InternedStr>,
    map: HashMap<String, StrId>,
}

impl<'a> RawStringInterner<'a> {
    fn new(interned: &'a mut Vec<InternedStr>) -> Self {
        let mut map = HashMap::with_capacity(interned.len());
        for entry in interned.iter() {
            map.entry(entry.value.clone()).or_insert(entry.id);
        }
        Self { interned, map }
    }

    fn intern(&mut self, value: &str) -> StrId {
        if let Some(id) = self.map.get(value) {
            return *id;
        }
        let id = StrId::new(self.interned.len() as u32);
        self.interned.push(InternedStr {
            id,
            value: value.to_string(),
            lifetime: StrLifetime::Static,
            span: None,
        });
        self.map.insert(value.to_string(), id);
        id
    }
}

fn intern_raw_strings_in_body(body: &mut MirBody, interner: &mut RawStringInterner<'_>) {
    for block in &mut body.blocks {
        for statement in &mut block.statements {
            intern_raw_strings_in_statement(&mut statement.kind, interner);
        }
        if let Some(terminator) = block.terminator.as_mut() {
            intern_raw_strings_in_terminator(terminator, interner);
        }
    }
}

fn intern_raw_strings_in_statement(kind: &mut StatementKind, interner: &mut RawStringInterner<'_>) {
    match kind {
        StatementKind::Assign { value, .. } => intern_raw_strings_in_rvalue(value, interner),
        StatementKind::ZeroInitRaw { pointer, length } => {
            intern_raw_strings_in_operand(pointer, interner);
            intern_raw_strings_in_operand(length, interner);
        }
        StatementKind::AtomicStore { value, .. } => {
            intern_raw_strings_in_operand(value, interner);
        }
        StatementKind::MmioStore { value, .. } => {
            intern_raw_strings_in_operand(value, interner);
        }
        StatementKind::Assert { cond, .. } => intern_raw_strings_in_operand(cond, interner),
        StatementKind::EnqueueKernel { kernel, args, .. } => {
            intern_raw_strings_in_operand(kernel, interner);
            for arg in args {
                intern_raw_strings_in_operand(arg, interner);
            }
        }
        StatementKind::EnqueueCopy { bytes, .. } => {
            intern_raw_strings_in_operand(bytes, interner);
        }
        StatementKind::StaticStore { value, .. } => {
            intern_raw_strings_in_operand(value, interner);
        }
        StatementKind::InlineAsm(asm) => intern_raw_strings_in_inline_asm(asm, interner),
        StatementKind::AtomicFence { .. }
        | StatementKind::StorageLive(_)
        | StatementKind::StorageDead(_)
        | StatementKind::MarkFallibleHandled { .. }
        | StatementKind::Deinit(_)
        | StatementKind::Drop { .. }
        | StatementKind::Borrow { .. }
        | StatementKind::Retag { .. }
        | StatementKind::DeferDrop { .. }
        | StatementKind::DefaultInit { .. }
        | StatementKind::ZeroInit { .. }
        | StatementKind::EnterUnsafe
        | StatementKind::ExitUnsafe
        | StatementKind::RecordEvent { .. }
        | StatementKind::WaitEvent { .. }
        | StatementKind::Eval(_)
        | StatementKind::Nop
        | StatementKind::Pending(_) => {}
    }
}

fn intern_raw_strings_in_inline_asm(asm: &mut InlineAsm, interner: &mut RawStringInterner<'_>) {
    for operand in &mut asm.operands {
        match &mut operand.kind {
            InlineAsmOperandKind::In { value } => {
                intern_raw_strings_in_operand(value, interner);
            }
            InlineAsmOperandKind::InOut { input, .. } => {
                intern_raw_strings_in_operand(input, interner);
            }
            InlineAsmOperandKind::Const { value } => {
                intern_raw_strings_in_operand(value, interner);
            }
            InlineAsmOperandKind::Out { .. } | InlineAsmOperandKind::Sym { .. } => {}
        }
    }
}

fn intern_raw_strings_in_rvalue(rvalue: &mut Rvalue, interner: &mut RawStringInterner<'_>) {
    match rvalue {
        Rvalue::Use(operand) => intern_raw_strings_in_operand(operand, interner),
        Rvalue::Unary { operand, .. } => intern_raw_strings_in_operand(operand, interner),
        Rvalue::Binary { lhs, rhs, .. } => {
            intern_raw_strings_in_operand(lhs, interner);
            intern_raw_strings_in_operand(rhs, interner);
        }
        Rvalue::Aggregate { fields, .. } => {
            for field in fields {
                intern_raw_strings_in_operand(field, interner);
            }
        }
        Rvalue::SpanStackAlloc { length, source, .. } => {
            intern_raw_strings_in_operand(length, interner);
            if let Some(source) = source.as_mut() {
                intern_raw_strings_in_operand(source, interner);
            }
        }
        Rvalue::Cast { operand, .. } => intern_raw_strings_in_operand(operand, interner),
        Rvalue::StringInterpolate { segments } => {
            for segment in segments {
                if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                    intern_raw_strings_in_operand(operand, interner);
                }
            }
        }
        Rvalue::NumericIntrinsic(intrinsic) => {
            for operand in &mut intrinsic.operands {
                intern_raw_strings_in_operand(operand, interner);
            }
        }
        Rvalue::DecimalIntrinsic(decimal) => {
            intern_raw_strings_in_operand(&mut decimal.lhs, interner);
            intern_raw_strings_in_operand(&mut decimal.rhs, interner);
            if let Some(addend) = decimal.addend.as_mut() {
                intern_raw_strings_in_operand(addend, interner);
            }
            intern_raw_strings_in_operand(&mut decimal.rounding, interner);
            intern_raw_strings_in_operand(&mut decimal.vectorize, interner);
        }
        Rvalue::AtomicRmw { value, .. } => intern_raw_strings_in_operand(value, interner),
        Rvalue::AtomicCompareExchange {
            expected, desired, ..
        } => {
            intern_raw_strings_in_operand(expected, interner);
            intern_raw_strings_in_operand(desired, interner);
        }
        Rvalue::AtomicLoad { .. }
        | Rvalue::AddressOf { .. }
        | Rvalue::Len(_)
        | Rvalue::Pending(_)
        | Rvalue::StaticLoad { .. }
        | Rvalue::StaticRef { .. } => {}
    }
}

fn intern_raw_strings_in_operand(operand: &mut Operand, interner: &mut RawStringInterner<'_>) {
    match operand {
        Operand::Const(constant) => {
            intern_raw_strings_in_const(&mut constant.value, interner);
        }
        Operand::Pending(pending) => {
            if let Some(info) = pending.info.as_mut() {
                match info.as_mut() {
                    PendingOperandInfo::FunctionGroup { receiver, .. } => {
                        if let Some(receiver) = receiver.as_mut() {
                            intern_raw_strings_in_operand(receiver, interner);
                        }
                    }
                }
            }
        }
        Operand::Copy(_) | Operand::Move(_) | Operand::Borrow(_) | Operand::Mmio(_) => {}
    }
}

fn intern_raw_strings_in_terminator(
    terminator: &mut Terminator,
    interner: &mut RawStringInterner<'_>,
) {
    match terminator {
        Terminator::SwitchInt { discr, .. } => intern_raw_strings_in_operand(discr, interner),
        Terminator::Match { arms, .. } => {
            for arm in arms {
                intern_raw_strings_in_pattern(&mut arm.pattern, interner);
            }
        }
        Terminator::Call { func, args, .. } => {
            intern_raw_strings_in_operand(func, interner);
            for arg in args {
                intern_raw_strings_in_operand(arg, interner);
            }
        }
        Terminator::Yield { value, .. } => intern_raw_strings_in_operand(value, interner),
        Terminator::Throw { exception, .. } => {
            if let Some(exception) = exception.as_mut() {
                intern_raw_strings_in_operand(exception, interner);
            }
        }
        Terminator::Goto { .. }
        | Terminator::Return
        | Terminator::Await { .. }
        | Terminator::Panic
        | Terminator::Unreachable
        | Terminator::Pending(_) => {}
    }
}

fn intern_raw_strings_in_pattern(pattern: &mut Pattern, interner: &mut RawStringInterner<'_>) {
    match pattern {
        Pattern::Literal(value) => intern_raw_strings_in_const(value, interner),
        Pattern::Tuple(items) => {
            for item in items {
                intern_raw_strings_in_pattern(item, interner);
            }
        }
        Pattern::Struct { fields, .. } => {
            for field in fields {
                intern_raw_strings_in_pattern_field(field, interner);
            }
        }
        Pattern::Enum { fields, .. } => match fields {
            VariantPatternFields::Unit => {}
            VariantPatternFields::Tuple(items) => {
                for item in items {
                    intern_raw_strings_in_pattern(item, interner);
                }
            }
            VariantPatternFields::Struct(fields) => {
                for field in fields {
                    intern_raw_strings_in_pattern_field(field, interner);
                }
            }
        },
        Pattern::Wildcard | Pattern::Binding(_) => {}
    }
}

fn intern_raw_strings_in_pattern_field(
    field: &mut PatternField,
    interner: &mut RawStringInterner<'_>,
) {
    intern_raw_strings_in_pattern(&mut field.pattern, interner);
}

fn intern_raw_strings_in_const(value: &mut ConstValue, interner: &mut RawStringInterner<'_>) {
    match value {
        ConstValue::RawStr(text) => {
            let raw = std::mem::take(text);
            let id = interner.intern(&raw);
            *value = ConstValue::Str { id, value: raw };
        }
        ConstValue::Struct { fields, .. } => {
            for (_, field_value) in fields {
                intern_raw_strings_in_const(field_value, interner);
            }
        }
        _ => {}
    }
}

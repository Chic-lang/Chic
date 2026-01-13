use std::collections::HashSet;

use crate::error::Error;
use crate::runtime::wasm_executor::{
    instructions::Instruction as RuntimeInstruction,
    module::TableElementType as RuntimeTableElementType,
    parser::parse_module as parse_runtime_module,
    types::{Value as RuntimeValue, ValueType as RuntimeValueType},
};

use super::builder::ModuleBuilder;
use super::support::make_unique_label;

impl<'a> ModuleBuilder<'a> {
    pub(crate) fn emit_wat(&self, bytes: &[u8]) -> Result<String, Error> {
        const INDENT: usize = 2;
        let runtime = parse_runtime_module(bytes).map_err(|err| {
            Error::Codegen(format!(
                "failed to decode wasm module for textual emission: {}",
                err.message
            ))
        })?;

        let labels = self.build_function_labels();
        if labels.len() != self.imports.len() + self.module.functions.len() {
            return Err(Error::Codegen(
                "function label table size mismatch during .wat emission".into(),
            ));
        }

        let mut out = String::new();
        out.push_str("(module\n");

        for (idx, ty) in runtime.types.iter().enumerate() {
            let mut line = format!("(type (;{idx};) (func");
            for param in &ty.params {
                line.push_str(" (param ");
                line.push_str(runtime_value_type_name(*param));
                line.push(')');
            }
            for result in &ty.results {
                line.push_str(" (result ");
                line.push_str(runtime_value_type_name(*result));
                line.push(')');
            }
            line.push_str("))");
            push_wat_line(&mut out, INDENT, &line);
        }

        for (idx, import) in runtime.imports.iter().enumerate() {
            let label = labels.get(idx).ok_or_else(|| {
                Error::Codegen("import index out of range during .wat emission".into())
            })?;
            let line = format!(
                "(import \"{}\" \"{}\" (func {} (type {})))",
                import.module, import.name, label, import.type_index
            );
            push_wat_line(&mut out, INDENT, &line);
        }

        for (idx, table) in runtime.tables.iter().enumerate() {
            let element_type = table_element_type_name(table.element_type);
            let mut line = format!("(table (;{idx};) ");
            match table.max {
                Some(max) => line.push_str(&format!("{} {} {}", table.min, max, element_type)),
                None => line.push_str(&format!("{} {}", table.min, element_type)),
            }
            line.push(')');
            push_wat_line(&mut out, INDENT, &line);
        }

        if let Some(min_pages) = runtime.memory_min_pages {
            push_wat_line(&mut out, INDENT, &format!("(memory (;0;) {min_pages})"));
        }

        for (idx, global) in runtime.globals.iter().enumerate() {
            let mut line = format!("(global (;{idx};) ");
            if global.mutable {
                line.push_str("(mut ");
                line.push_str(runtime_value_type_name(global.ty));
                line.push(')');
            } else {
                line.push_str(runtime_value_type_name(global.ty));
            }
            line.push(' ');
            line.push_str(&runtime_value_expr(&global.initial));
            line.push(')');
            push_wat_line(&mut out, INDENT, &line);
        }

        let import_count = self.imports.len();
        for (func_idx, function) in runtime.functions.iter().enumerate() {
            let label = labels.get(import_count + func_idx).ok_or_else(|| {
                Error::Codegen("function index out of range during .wat emission".into())
            })?;
            let mut header = format!("(func {} (type {})", label, function.type_index);
            if !function.locals.is_empty() {
                header.push_str(" (local");
                for local in &function.locals {
                    header.push(' ');
                    header.push_str(runtime_value_type_name(*local));
                }
                header.push(')');
            }
            push_wat_line(&mut out, INDENT, &header);

            let mut body_indent = INDENT * 2;
            let mut control_stack: Vec<()> = Vec::new();

            for inst in &function.code {
                match inst {
                    RuntimeInstruction::Block { .. } => {
                        push_wat_line(&mut out, body_indent, "(block");
                        control_stack.push(());
                        body_indent += INDENT;
                    }
                    RuntimeInstruction::Loop { .. } => {
                        push_wat_line(&mut out, body_indent, "(loop");
                        control_stack.push(());
                        body_indent += INDENT;
                    }
                    RuntimeInstruction::If { .. } => {
                        push_wat_line(&mut out, body_indent, "(if");
                        control_stack.push(());
                        body_indent += INDENT;
                    }
                    RuntimeInstruction::End => {
                        if control_stack.pop().is_some() {
                            body_indent = body_indent.saturating_sub(INDENT);
                            push_wat_line(&mut out, body_indent, ")");
                        } else {
                            return Err(Error::Codegen(
                                "unbalanced control flow detected during .wat emission".into(),
                            ));
                        }
                    }
                    _ => {
                        let line = instruction_to_wat(inst, &labels)?;
                        push_wat_line(&mut out, body_indent, &line);
                    }
                }
            }

            if !control_stack.is_empty() {
                return Err(Error::Codegen(
                    "unbalanced control flow detected during .wat emission".into(),
                ));
            }

            push_wat_line(&mut out, INDENT, ")");
            out.push('\n');
        }

        for (table_idx, table) in runtime.tables.iter().enumerate() {
            if table.elements.is_empty() {
                continue;
            }
            let mut line = format!("(elem (table {table_idx}) (i32.const 0)");
            for entry in &table.elements {
                let func_index = entry.ok_or_else(|| {
                    Error::Codegen(
                        "function table entry is uninitialised during .wat emission".into(),
                    )
                })?;
                let idx = usize::try_from(func_index).map_err(|_| {
                    Error::Codegen(
                        "call target index exceeds wasm limits during .wat emission".into(),
                    )
                })?;
                let label = labels.get(idx).ok_or_else(|| {
                    Error::Codegen("function label table size mismatch during .wat emission".into())
                })?;
                line.push_str(" func ");
                line.push_str(label);
            }
            line.push(')');
            push_wat_line(&mut out, INDENT, &line);
        }

        let mut exports: Vec<_> = runtime.exports.iter().collect();
        exports.sort_by(|a, b| a.0.cmp(b.0));
        for (name, index) in exports {
            let idx = usize::try_from(*index).map_err(|_| {
                Error::Codegen("export index exceeds wasm limits during .wat emission".into())
            })?;
            let label = labels
                .get(idx)
                .ok_or_else(|| Error::Codegen("export references missing function".into()))?;
            let line = format!("(export \"{}\" (func {}))", name, label);
            push_wat_line(&mut out, INDENT, &line);
        }

        out.push_str(")\n");
        Ok(out)
    }

    fn build_function_labels(&self) -> Vec<String> {
        let mut used = HashSet::new();
        let mut labels = Vec::with_capacity(self.imports.len() + self.module.functions.len());
        for (idx, import) in self.imports.iter().enumerate() {
            let raw = format!("{}::{}", import.module, import.name);
            let label = make_unique_label(&raw, idx, &mut used);
            labels.push(format!("${label}"));
        }
        for (idx, function) in self.module.functions.iter().enumerate() {
            let label = make_unique_label(&function.name, self.imports.len() + idx, &mut used);
            labels.push(format!("${label}"));
        }
        labels
    }
}

fn push_wat_line(buf: &mut String, indent: usize, line: &str) {
    for _ in 0..indent {
        buf.push(' ');
    }
    buf.push_str(line);
    buf.push('\n');
}

fn runtime_value_type_name(ty: RuntimeValueType) -> &'static str {
    match ty {
        RuntimeValueType::I32 => "i32",
        RuntimeValueType::I64 => "i64",
        RuntimeValueType::F32 => "f32",
        RuntimeValueType::F64 => "f64",
    }
}

fn table_element_type_name(ty: RuntimeTableElementType) -> &'static str {
    match ty {
        RuntimeTableElementType::FuncRef => "funcref",
    }
}

fn runtime_value_expr(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::I32(v) => format!("i32.const {v}"),
        RuntimeValue::I64(v) => format!("i64.const {v}"),
        RuntimeValue::F32(v) => format!("f32.const {:?}", v),
        RuntimeValue::F64(v) => format!("f64.const {:?}", v),
    }
}

fn instruction_to_wat(inst: &RuntimeInstruction, labels: &[String]) -> Result<String, Error> {
    match inst {
        RuntimeInstruction::Br { depth } => Ok(format!("br {depth}")),
        RuntimeInstruction::Call { func } => {
            let index = usize::try_from(*func).map_err(|_| {
                Error::Codegen("call index exceeds wasm limits during .wat emission".into())
            })?;
            let label = labels.get(index).ok_or_else(|| {
                Error::Codegen(format!(
                    "call target {index} missing from wasm label table during .wat emission"
                ))
            })?;
            Ok(format!("call {label}"))
        }
        RuntimeInstruction::CallIndirect {
            type_index,
            table_index,
        } => Ok(format!(
            "call_indirect (type {}) (table {})",
            type_index, table_index
        )),
        RuntimeInstruction::Return => Ok("return".into()),
        RuntimeInstruction::Unreachable => Ok("unreachable".into()),
        RuntimeInstruction::Drop => Ok("drop".into()),
        RuntimeInstruction::I32Const(value) => Ok(format!("i32.const {value}")),
        RuntimeInstruction::I64Const(value) => Ok(format!("i64.const {value}")),
        RuntimeInstruction::F32Const(value) => Ok(format!("f32.const {:?}", value)),
        RuntimeInstruction::F64Const(value) => Ok(format!("f64.const {:?}", value)),
        RuntimeInstruction::I32Eq => Ok("i32.eq".into()),
        RuntimeInstruction::I32Ne => Ok("i32.ne".into()),
        RuntimeInstruction::I32Eqz => Ok("i32.eqz".into()),
        RuntimeInstruction::I32LtS => Ok("i32.lt_s".into()),
        RuntimeInstruction::I32LtU => Ok("i32.lt_u".into()),
        RuntimeInstruction::I32LeS => Ok("i32.le_s".into()),
        RuntimeInstruction::I32LeU => Ok("i32.le_u".into()),
        RuntimeInstruction::I32GtS => Ok("i32.gt_s".into()),
        RuntimeInstruction::I32GtU => Ok("i32.gt_u".into()),
        RuntimeInstruction::I32GeS => Ok("i32.ge_s".into()),
        RuntimeInstruction::I32GeU => Ok("i32.ge_u".into()),
        RuntimeInstruction::I64Eq => Ok("i64.eq".into()),
        RuntimeInstruction::I64Ne => Ok("i64.ne".into()),
        RuntimeInstruction::I64LtS => Ok("i64.lt_s".into()),
        RuntimeInstruction::I64LeS => Ok("i64.le_s".into()),
        RuntimeInstruction::I64GtS => Ok("i64.gt_s".into()),
        RuntimeInstruction::I64GeS => Ok("i64.ge_s".into()),
        RuntimeInstruction::I64LtU => Ok("i64.lt_u".into()),
        RuntimeInstruction::I64LeU => Ok("i64.le_u".into()),
        RuntimeInstruction::I64GtU => Ok("i64.gt_u".into()),
        RuntimeInstruction::I64GeU => Ok("i64.ge_u".into()),
        RuntimeInstruction::I64Eqz => Ok("i64.eqz".into()),
        RuntimeInstruction::F32Eq => Ok("f32.eq".into()),
        RuntimeInstruction::F32Ne => Ok("f32.ne".into()),
        RuntimeInstruction::F32Lt => Ok("f32.lt".into()),
        RuntimeInstruction::F32Gt => Ok("f32.gt".into()),
        RuntimeInstruction::F32Le => Ok("f32.le".into()),
        RuntimeInstruction::F32Ge => Ok("f32.ge".into()),
        RuntimeInstruction::F64Eq => Ok("f64.eq".into()),
        RuntimeInstruction::F64Ne => Ok("f64.ne".into()),
        RuntimeInstruction::F64Lt => Ok("f64.lt".into()),
        RuntimeInstruction::F64Gt => Ok("f64.gt".into()),
        RuntimeInstruction::F64Le => Ok("f64.le".into()),
        RuntimeInstruction::F64Ge => Ok("f64.ge".into()),
        RuntimeInstruction::I32Add => Ok("i32.add".into()),
        RuntimeInstruction::I32Sub => Ok("i32.sub".into()),
        RuntimeInstruction::I32Mul => Ok("i32.mul".into()),
        RuntimeInstruction::I32DivS => Ok("i32.div_s".into()),
        RuntimeInstruction::I32DivU => Ok("i32.div_u".into()),
        RuntimeInstruction::I32RemS => Ok("i32.rem_s".into()),
        RuntimeInstruction::I32RemU => Ok("i32.rem_u".into()),
        RuntimeInstruction::I32And => Ok("i32.and".into()),
        RuntimeInstruction::I32Or => Ok("i32.or".into()),
        RuntimeInstruction::I32Xor => Ok("i32.xor".into()),
        RuntimeInstruction::I64And => Ok("i64.and".into()),
        RuntimeInstruction::I64Or => Ok("i64.or".into()),
        RuntimeInstruction::I64Xor => Ok("i64.xor".into()),
        RuntimeInstruction::I64Add => Ok("i64.add".into()),
        RuntimeInstruction::I64Sub => Ok("i64.sub".into()),
        RuntimeInstruction::I64Mul => Ok("i64.mul".into()),
        RuntimeInstruction::I64DivS => Ok("i64.div_s".into()),
        RuntimeInstruction::I64DivU => Ok("i64.div_u".into()),
        RuntimeInstruction::I64RemS => Ok("i64.rem_s".into()),
        RuntimeInstruction::I64RemU => Ok("i64.rem_u".into()),
        RuntimeInstruction::I32Shl => Ok("i32.shl".into()),
        RuntimeInstruction::I32ShrS => Ok("i32.shr_s".into()),
        RuntimeInstruction::I32ShrU => Ok("i32.shr_u".into()),
        RuntimeInstruction::I64Shl => Ok("i64.shl".into()),
        RuntimeInstruction::I64ShrS => Ok("i64.shr_s".into()),
        RuntimeInstruction::I64ShrU => Ok("i64.shr_u".into()),
        RuntimeInstruction::I32WrapI64 => Ok("i32.wrap_i64".into()),
        RuntimeInstruction::F32Add => Ok("f32.add".into()),
        RuntimeInstruction::F32Sub => Ok("f32.sub".into()),
        RuntimeInstruction::F32Mul => Ok("f32.mul".into()),
        RuntimeInstruction::F32Div => Ok("f32.div".into()),
        RuntimeInstruction::F32Trunc => Ok("f32.trunc".into()),
        RuntimeInstruction::F32ConvertI32S => Ok("f32.convert_i32_s".into()),
        RuntimeInstruction::F32ConvertI32U => Ok("f32.convert_i32_u".into()),
        RuntimeInstruction::F32ConvertI64S => Ok("f32.convert_i64_s".into()),
        RuntimeInstruction::F32ConvertI64U => Ok("f32.convert_i64_u".into()),
        RuntimeInstruction::F64Add => Ok("f64.add".into()),
        RuntimeInstruction::F64Sub => Ok("f64.sub".into()),
        RuntimeInstruction::F64Mul => Ok("f64.mul".into()),
        RuntimeInstruction::F64Div => Ok("f64.div".into()),
        RuntimeInstruction::F64Trunc => Ok("f64.trunc".into()),
        RuntimeInstruction::F64ConvertI32S => Ok("f64.convert_i32_s".into()),
        RuntimeInstruction::F64ConvertI32U => Ok("f64.convert_i32_u".into()),
        RuntimeInstruction::F64ConvertI64S => Ok("f64.convert_i64_s".into()),
        RuntimeInstruction::F64ConvertI64U => Ok("f64.convert_i64_u".into()),
        RuntimeInstruction::I32TruncF32S => Ok("i32.trunc_f32_s".into()),
        RuntimeInstruction::I32TruncF32U => Ok("i32.trunc_f32_u".into()),
        RuntimeInstruction::I32TruncF64S => Ok("i32.trunc_f64_s".into()),
        RuntimeInstruction::I32TruncF64U => Ok("i32.trunc_f64_u".into()),
        RuntimeInstruction::I64TruncF32S => Ok("i64.trunc_f32_s".into()),
        RuntimeInstruction::I64TruncF32U => Ok("i64.trunc_f32_u".into()),
        RuntimeInstruction::I64TruncF64S => Ok("i64.trunc_f64_s".into()),
        RuntimeInstruction::I64TruncF64U => Ok("i64.trunc_f64_u".into()),
        RuntimeInstruction::F64PromoteF32 => Ok("f64.promote_f32".into()),
        RuntimeInstruction::F32DemoteF64 => Ok("f32.demote_f64".into()),
        RuntimeInstruction::I32ReinterpretF32 => Ok("i32.reinterpret_f32".into()),
        RuntimeInstruction::I64ReinterpretF64 => Ok("i64.reinterpret_f64".into()),
        RuntimeInstruction::F32ReinterpretI32 => Ok("f32.reinterpret_i32".into()),
        RuntimeInstruction::F64ReinterpretI64 => Ok("f64.reinterpret_i64".into()),
        RuntimeInstruction::I64ExtendI32S => Ok("i64.extend_i32_s".into()),
        RuntimeInstruction::I64ExtendI32U => Ok("i64.extend_i32_u".into()),
        RuntimeInstruction::LocalGet(index) => Ok(format!("local.get {index}")),
        RuntimeInstruction::LocalSet(index) => Ok(format!("local.set {index}")),
        RuntimeInstruction::LocalTee(index) => Ok(format!("local.tee {index}")),
        RuntimeInstruction::GlobalGet(index) => Ok(format!("global.get {index}")),
        RuntimeInstruction::GlobalSet(index) => Ok(format!("global.set {index}")),
        RuntimeInstruction::I32Load { offset } => Ok(format!("i32.load offset={offset}")),
        RuntimeInstruction::I32Load8S { offset } => Ok(format!("i32.load8_s offset={offset}")),
        RuntimeInstruction::I32Load8U { offset } => Ok(format!("i32.load8_u offset={offset}")),
        RuntimeInstruction::I32Load16S { offset } => Ok(format!("i32.load16_s offset={offset}")),
        RuntimeInstruction::I32Load16U { offset } => Ok(format!("i32.load16_u offset={offset}")),
        RuntimeInstruction::I64Load { offset } => Ok(format!("i64.load offset={offset}")),
        RuntimeInstruction::F32Load { offset } => Ok(format!("f32.load offset={offset}")),
        RuntimeInstruction::F64Load { offset } => Ok(format!("f64.load offset={offset}")),
        RuntimeInstruction::I32Store { offset } => Ok(format!("i32.store offset={offset}")),
        RuntimeInstruction::I32Store8 { offset } => Ok(format!("i32.store8 offset={offset}")),
        RuntimeInstruction::I32Store16 { offset } => Ok(format!("i32.store16 offset={offset}")),
        RuntimeInstruction::I64Store { offset } => Ok(format!("i64.store offset={offset}")),
        RuntimeInstruction::F32Store { offset } => Ok(format!("f32.store offset={offset}")),
        RuntimeInstruction::F64Store { offset } => Ok(format!("f64.store offset={offset}")),
        RuntimeInstruction::I32AtomicLoad { offset } => {
            Ok(format!("i32.atomic.load offset={offset}"))
        }
        RuntimeInstruction::I64AtomicLoad { offset } => {
            Ok(format!("i64.atomic.load offset={offset}"))
        }
        RuntimeInstruction::I32AtomicStore { offset } => {
            Ok(format!("i32.atomic.store offset={offset}"))
        }
        RuntimeInstruction::I64AtomicStore { offset } => {
            Ok(format!("i64.atomic.store offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwAdd { offset } => {
            Ok(format!("i32.atomic.rmw.add offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwAdd { offset } => {
            Ok(format!("i64.atomic.rmw.add offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwSub { offset } => {
            Ok(format!("i32.atomic.rmw.sub offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwSub { offset } => {
            Ok(format!("i64.atomic.rmw.sub offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwAnd { offset } => {
            Ok(format!("i32.atomic.rmw.and offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwAnd { offset } => {
            Ok(format!("i64.atomic.rmw.and offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwOr { offset } => {
            Ok(format!("i32.atomic.rmw.or offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwOr { offset } => {
            Ok(format!("i64.atomic.rmw.or offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwXor { offset } => {
            Ok(format!("i32.atomic.rmw.xor offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwXor { offset } => {
            Ok(format!("i64.atomic.rmw.xor offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwXchg { offset } => {
            Ok(format!("i32.atomic.rmw.xchg offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwXchg { offset } => {
            Ok(format!("i64.atomic.rmw.xchg offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwCmpxchg { offset } => {
            Ok(format!("i32.atomic.rmw.cmpxchg offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwCmpxchg { offset } => {
            Ok(format!("i64.atomic.rmw.cmpxchg offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwMinS { offset } => {
            Ok(format!("i32.atomic.rmw.min_s offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwMinS { offset } => {
            Ok(format!("i64.atomic.rmw.min_s offset={offset}"))
        }
        RuntimeInstruction::I32AtomicRmwMaxS { offset } => {
            Ok(format!("i32.atomic.rmw.max_s offset={offset}"))
        }
        RuntimeInstruction::I64AtomicRmwMaxS { offset } => {
            Ok(format!("i64.atomic.rmw.max_s offset={offset}"))
        }
        RuntimeInstruction::AtomicFence => Ok("atomic.fence 0".into()),
        RuntimeInstruction::MemoryFill { mem } => Ok(format!("memory.fill {mem}")),
        RuntimeInstruction::Block { .. }
        | RuntimeInstruction::Loop { .. }
        | RuntimeInstruction::If { .. }
        | RuntimeInstruction::End => Err(Error::Codegen(
            "structural instruction routed through textual helper unexpectedly".into(),
        )),
    }
}

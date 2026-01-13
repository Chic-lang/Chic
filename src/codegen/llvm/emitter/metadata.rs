use std::collections::HashMap;
use std::fmt::Write;

use crate::error::Error;
use crate::mir::async_types::{is_task_ty, task_result_ty};
use crate::mir::{MirModule, Ty};
use crate::runtime::startup;

use crate::codegen::llvm::signatures::LlvmFunctionSignature;

pub(crate) const STARTUP_DESCRIPTOR_TYPE: &str = "%__chx_startup_descriptor";
pub(crate) const STARTUP_TESTCASE_TYPE: &str = "%__chx_testcase_descriptor";

pub(crate) fn entry_return_flags(ty: &Ty) -> Result<u32, Error> {
    if let Some(inner) = task_result_ty(ty) {
        return entry_return_flags(&inner);
    }
    if is_task_ty(ty) {
        return Ok(startup::ENTRY_FLAG_RET_VOID);
    }
    match ty {
        Ty::Unit => Ok(startup::ENTRY_FLAG_RET_VOID),
        Ty::Named(name) => match name.as_str() {
            "bool" => Ok(startup::ENTRY_FLAG_RET_BOOL),
            "int" | "uint" => Ok(startup::ENTRY_FLAG_RET_I32),
            other => Err(Error::Codegen(format!(
                "entry point return type `{other}` is not supported for native startup"
            ))),
        },
        other => Err(Error::Codegen(format!(
            "entry point return type `{other:?}` is not supported for native startup"
        ))),
    }
}

fn is_args_parameter(ty: &Ty) -> bool {
    match ty {
        Ty::Array(array) => array.rank == 1 && matches!(&*array.element, Ty::String),
        Ty::Vec(vec) => matches!(&*vec.element, Ty::String),
        _ => false,
    }
}

pub(crate) fn emit_startup_metadata(
    out: &mut String,
    mir: &MirModule,
    signatures: &HashMap<String, LlvmFunctionSignature>,
    entry: Option<&str>,
) -> Result<(), Error> {
    let descriptor_symbol = startup::STARTUP_DESCRIPTOR_SYMBOL;

    // Default to a null pointer so modules without an entry function do not emit
    // a startup descriptor that can override real descriptors during linking.
    let mut entry_symbol_expr = "null".to_string();
    let mut entry_flags = 0u32;

    writeln!(out, "%__chx_entry_descriptor = type {{ ptr, i32, i32 }}").ok();
    writeln!(
        out,
        "%__chx_testcase_descriptor = type {{ ptr, ptr, i64, i32, i32 }}"
    )
    .ok();
    writeln!(out, "%__chx_testsuite_descriptor = type {{ ptr, i64 }}").ok();
    writeln!(out, "%__chx_startup_descriptor = type {{ i32, %__chx_entry_descriptor, %__chx_testsuite_descriptor }}").ok();
    writeln!(out).ok();

    if mir.statics.iter().any(|var| {
        if var.is_extern {
            return false;
        }
        if let Some(spec) = var
            .extern_spec
            .as_ref()
            .and_then(|spec| spec.alias.as_ref())
        {
            return spec == descriptor_symbol;
        }
        var.qualified.ends_with(descriptor_symbol)
    }) {
        // A concrete startup descriptor definition already exists in this module.
        return Ok(());
    }

    if let Some(entry_name) = entry {
        if let Some(function) = mir.functions.iter().find(|func| func.name == entry_name) {
            let sig = signatures.get(&function.name).ok_or_else(|| {
                Error::Codegen(format!(
                    "missing LLVM signature for entry `{}` during startup metadata emission",
                    function.name
                ))
            })?;
            entry_symbol_expr = format!("@{}", sig.symbol);
            entry_flags |= entry_return_flags(&function.signature.ret)?;
            if function.is_async {
                entry_flags |= startup::ENTRY_FLAG_ASYNC;
            }
            if function.signature.params.iter().any(is_args_parameter) {
                entry_flags |= startup::ENTRY_FLAG_PARAM_ARGS;
            }
        }
    }

    let mut testcase_entries = Vec::new();
    for function in &mir.functions {
        if matches!(function.kind, crate::mir::FunctionKind::Testcase) {
            let sig = signatures.get(&function.name).ok_or_else(|| {
                Error::Codegen(format!(
                    "missing LLVM signature for testcase `{}`",
                    function.name
                ))
            })?;
            testcase_entries.push((function, sig));
        }
    }

    for (index, (function, _sig)) in testcase_entries.iter().enumerate() {
        let symbol = format!("@__chx_testcase_name_{index}");
        let encoded = super::literals::encode_llvm_bytes(function.name.as_bytes());
        let array_len = function.name.len().saturating_add(1);
        writeln!(
            out,
            "{symbol} = private unnamed_addr constant [{array_len} x i8] c\"{encoded}\\00\""
        )
        .ok();
    }
    if !testcase_entries.is_empty() {
        writeln!(out).ok();
    }

    let testcase_symbol = "@__chic_testcases";
    if !testcase_entries.is_empty() {
        let count = testcase_entries.len();
        writeln!(
            out,
            "{testcase_symbol} = private constant [{count} x {STARTUP_TESTCASE_TYPE}] ["
        )
        .ok();
        for (index, (function, sig)) in testcase_entries.iter().enumerate() {
            let separator = if index + 1 == testcase_entries.len() {
                ""
            } else {
                ","
            };
            let name_symbol = format!("@__chx_testcase_name_{index}");
            let name_len = function.name.len() as i64;
            let array_len = function.name.len().saturating_add(1);
            let name_ptr = format!(
                "ptr getelementptr inbounds ([{array_len} x i8], ptr {name_symbol}, i32 0, i32 0)"
            );
            let flags = if function.is_async {
                startup::TESTCASE_FLAG_ASYNC
            } else {
                0
            };
            writeln!(
                out,
                "    {STARTUP_TESTCASE_TYPE} {{ ptr @{func_symbol}, {name_ptr}, i64 {name_len}, i32 {flags}, i32 0 }}{separator}",
                func_symbol = sig.symbol
            )
            .ok();
        }
        writeln!(out, "]").ok();
        writeln!(out).ok();
    }

    let testcase_ptr_expr = if testcase_entries.is_empty() {
        "ptr null".to_string()
    } else {
        format!(
            "ptr getelementptr inbounds ([{len} x {STARTUP_TESTCASE_TYPE}], ptr {testcase_symbol}, i32 0, i32 0)",
            len = testcase_entries.len(),
        )
    };
    let testcase_len = testcase_entries.len() as i64;

    let descriptor_linkage = if entry_symbol_expr == "null" {
        "weak_odr dso_local constant"
    } else {
        "dso_local constant"
    };
    writeln!(
        out,
        "@{descriptor_symbol} = {descriptor_linkage} {STARTUP_DESCRIPTOR_TYPE} {{ i32 {version}, %__chx_entry_descriptor {{ ptr {entry_ptr}, i32 {entry_flags}, i32 0 }}, %__chx_testsuite_descriptor {{ {testcase_ptr}, i64 {testcase_len} }} }}, align 8",
        version = startup::STARTUP_DESCRIPTOR_VERSION,
        entry_ptr = entry_symbol_expr,
        testcase_ptr = testcase_ptr_expr,
        testcase_len = testcase_len,
    )
    .ok();
    writeln!(out).ok();

    Ok(())
}

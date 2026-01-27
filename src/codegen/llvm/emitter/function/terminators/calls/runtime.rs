use std::fmt::Write;

use super::args::render_args_for_types;
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::function::values::ValueRef;
use crate::codegen::llvm::emitter::literals::LLVM_STRING_TYPE;
use crate::codegen::llvm::signatures::canonical_function_name;
use crate::error::Error;
use crate::mir::{BlockId, Operand, Place, TypeLayout};

pub(super) const OBJECT_NEW_RUNTIME_FN: &str = "chic_rt_object_new";

impl<'a> FunctionEmitter<'a> {
    pub(super) fn try_emit_object_new_call(
        &mut self,
        repr: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<bool, Error> {
        if repr != OBJECT_NEW_RUNTIME_FN {
            return Ok(false);
        }
        if args.len() != 1 {
            return Err(Error::Codegen(
                "`chic_rt_object_new` expects a single type-id argument".into(),
            ));
        }
        let dest_label = self.block_label(target)?;
        let type_id = self.emit_operand(&args[0], Some("i64"))?;
        self.externals.insert(OBJECT_NEW_RUNTIME_FN);
        if let Some(place) = destination {
            let result = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {result} = call ptr @{OBJECT_NEW_RUNTIME_FN}(i64 {})",
                type_id.repr()
            )
            .ok();
            let store_ty = self.place_type(place)?.ok_or_else(|| {
                Error::Codegen("`chic_rt_object_new` destination is missing an LLVM type".into())
            })?;
            let value = if store_ty == "ptr" {
                ValueRef::new(result.clone(), "ptr")
            } else if store_ty.ends_with('*') {
                let cast = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {cast} = bitcast ptr {result} to {store_ty}"
                )
                .ok();
                ValueRef::new(cast, &store_ty)
            } else {
                return Err(Error::Codegen(format!(
                    "`chic_rt_object_new` cannot assign to destination type `{store_ty}` in `{}`",
                    self.function.name
                )));
            };
            self.store_place(place, &value)?;
        } else {
            writeln!(
                &mut self.builder,
                "  call ptr @{OBJECT_NEW_RUNTIME_FN}(i64 {})",
                type_id.repr()
            )
            .ok();
        }
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(true)
    }

    pub(super) fn try_emit_startup_runtime_call(
        &mut self,
        repr: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<bool, Error> {
        let canonical = canonical_function_name(repr);
        if canonical.ends_with("::RuntimeExports::TaskHeader")
            || canonical == "RuntimeExports::TaskHeader"
            || canonical == "chic_rt_async_task_header"
        {
            self.emit_async_task_header(args, destination, target)?;
            return Ok(true);
        }
        if canonical.ends_with("::chic_rt_startup_exit") || canonical == "chic_rt_startup_exit" {
            self.emit_startup_exit(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_descriptor_snapshot") {
            self.emit_startup_descriptor_snapshot(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_test_descriptor") {
            self.emit_startup_test_descriptor(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_call_entry") {
            self.emit_startup_call_entry(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_call_entry_async") {
            self.emit_startup_call_entry_async(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_complete_entry_async") {
            self.emit_startup_complete_entry_async(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_call_testcase") {
            self.emit_startup_call_testcase(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_call_testcase_async") {
            self.emit_startup_call_testcase_async(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_complete_testcase_async") {
            self.emit_startup_complete_testcase_async(args, destination, target)?;
            return Ok(true);
        }
        if canonical.ends_with("::StartupRuntimeState::chic_rt_startup_store_state")
            || canonical == "StartupRuntimeState::chic_rt_startup_store_state"
            || canonical == "chic_rt_startup_store_state"
            || canonical.ends_with(
                "::Std::Runtime::Startup::StartupRuntimeState::chic_rt_startup_store_state",
            )
        {
            self.emit_startup_store_state(args, destination, target)?;
            return Ok(true);
        }
        if canonical.ends_with("::StartupRuntimeState::chic_rt_startup_raw_argc")
            || canonical == "StartupRuntimeState::chic_rt_startup_raw_argc"
            || canonical == "chic_rt_startup_raw_argc"
        {
            self.emit_startup_raw_argc(destination, target)?;
            return Ok(true);
        }
        if canonical.ends_with("::StartupRuntimeState::chic_rt_startup_raw_argv")
            || canonical == "StartupRuntimeState::chic_rt_startup_raw_argv"
            || canonical == "chic_rt_startup_raw_argv"
        {
            self.emit_startup_raw_pointer("chic_rt_startup_raw_argv", destination, target)?;
            return Ok(true);
        }
        if canonical.ends_with("::StartupRuntimeState::chic_rt_startup_raw_envp")
            || canonical == "StartupRuntimeState::chic_rt_startup_raw_envp"
            || canonical == "chic_rt_startup_raw_envp"
        {
            self.emit_startup_raw_pointer("chic_rt_startup_raw_envp", destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_has_run_tests_flag") {
            self.emit_startup_scalar_call(
                "chic_rt_startup_has_run_tests_flag",
                "i32",
                Vec::new(),
                destination,
                target,
            )?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_ptr_at") {
            self.emit_startup_ptr_at(args, destination, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_cstr_to_string") {
            self.emit_startup_string_call(
                "chic_rt_startup_cstr_to_string",
                args,
                vec!["ptr".to_string()],
                destination,
                target,
            )?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_slice_to_string") {
            self.emit_startup_string_call(
                "chic_rt_startup_slice_to_string",
                args,
                vec!["ptr".to_string(), self.pointer_int_type().to_string()],
                destination,
                target,
            )?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_i32_to_string") {
            self.emit_startup_string_call(
                "chic_rt_startup_i32_to_string",
                args,
                vec!["i32".to_string()],
                destination,
                target,
            )?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_startup_usize_to_string") {
            self.emit_startup_string_call(
                "chic_rt_startup_usize_to_string",
                args,
                vec![self.pointer_int_type().to_string()],
                destination,
                target,
            )?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_stdout_write_string") {
            self.emit_startup_io_write("chic_rt_stdout_write_string", args, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_stdout_write_line_string") {
            self.emit_startup_io_write("chic_rt_stdout_write_line_string", args, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_stdout_flush") {
            self.emit_startup_io_flush("chic_rt_stdout_flush", target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_stderr_write_string") {
            self.emit_startup_io_write("chic_rt_stderr_write_string", args, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_stderr_write_line_string") {
            self.emit_startup_io_write("chic_rt_stderr_write_line_string", args, target)?;
            return Ok(true);
        }
        if Self::runtime_intrinsic_matches(&canonical, "chic_rt_stderr_flush") {
            self.emit_startup_io_flush("chic_rt_stderr_flush", target)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn emit_startup_exit(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if destination.is_some() {
            return Err(Error::Codegen(
                "startup exit call cannot assign to a destination".into(),
            ));
        }
        if args.len() != 1 {
            return Err(Error::Codegen(
                "startup exit call expects exactly one argument".into(),
            ));
        }
        let exit_code = self.emit_operand(&args[0], Some("i32"))?;
        self.externals.insert("chic_rt_startup_exit");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_startup_exit(i32 {})",
            exit_code.repr()
        )
        .ok();
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_store_state(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if destination.is_some() {
            return Err(Error::Codegen(
                "startup store-state call cannot assign to a destination".into(),
            ));
        }
        if args.len() != 3 {
            return Err(Error::Codegen(
                "startup store-state call expects argc, argv, envp arguments".into(),
            ));
        }
        let arg_count = self.emit_operand(&args[0], Some("i32"))?;
        let arg_values_ptr = self.emit_operand(&args[1], Some("ptr"))?;
        let env_ptr = self.emit_operand(&args[2], Some("ptr"))?;
        self.externals.insert("chic_rt_startup_store_state");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_startup_store_state(i32 {}, ptr {}, ptr {})",
            arg_count.repr(),
            arg_values_ptr.repr(),
            env_ptr.repr()
        )
        .ok();
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_raw_argc(
        &mut self,
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        let place = destination.ok_or_else(|| {
            Error::Codegen("startup argc access must assign to a destination".into())
        })?;
        self.externals.insert("chic_rt_startup_raw_argc");
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = call i32 @chic_rt_startup_raw_argc()"
        )
        .ok();
        self.store_place(place, &ValueRef::new(tmp, "i32"))?;
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_raw_pointer(
        &mut self,
        symbol: &'static str,
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        let place = destination.ok_or_else(|| {
            Error::Codegen("startup pointer access must assign to a destination".into())
        })?;
        if place.projection.is_empty() {
            if let Some(slot) = self.local_tys.get_mut(place.local.0) {
                *slot = Some("ptr".to_string());
            }
        }
        self.externals.insert(symbol);
        let tmp = self.new_temp();
        writeln!(&mut self.builder, "  {tmp} = call ptr @{symbol}()").ok();
        self.store_place(place, &ValueRef::new(tmp, "ptr"))?;
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_ptr_at(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 3 {
            return Err(Error::Codegen(
                "startup ptr_at call expects list, index, and limit arguments".into(),
            ));
        }
        let place = destination.ok_or_else(|| {
            Error::Codegen("startup ptr_at call must assign to a destination".into())
        })?;
        if place.projection.is_empty() {
            if let Some(slot) = self.local_tys.get_mut(place.local.0) {
                *slot = Some("ptr".to_string());
            }
        }
        let list = self.emit_operand(&args[0], Some("ptr"))?;
        let index = self.emit_operand(&args[1], Some("i32"))?;
        let limit = self.emit_operand(&args[2], Some("i32"))?;
        self.externals.insert("chic_rt_startup_ptr_at");
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = call ptr @chic_rt_startup_ptr_at(ptr {}, i32 {}, i32 {})",
            list.repr(),
            index.repr(),
            limit.repr()
        )
        .ok();
        self.store_place(place, &ValueRef::new(tmp, "ptr"))?;
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_descriptor_snapshot(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        self.emit_startup_call_with_destination(
            "chic_rt_startup_descriptor_snapshot",
            args,
            Vec::new(),
            destination,
            target,
        )
    }

    pub(super) fn emit_startup_test_descriptor(
        &mut self,
        args: &[Operand],
        _destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 2 {
            return Err(Error::Codegen(
                "chic_rt_startup_test_descriptor expects destination pointer and index".into(),
            ));
        }
        let dest_ptr = self.emit_operand(&args[0], Some("ptr"))?;
        let index = self.emit_operand(&args[1], Some(self.pointer_int_type()))?;
        self.externals.insert("chic_rt_startup_test_descriptor");
        let dest_label = self.block_label(target)?;
        let width = self.pointer_int_type();
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_startup_test_descriptor(ptr {}, {width} {})",
            dest_ptr.repr(),
            index.repr(),
        )
        .ok();
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_call_entry(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 5 {
            return Err(Error::Codegen(
                "startup call_entry expects function, flags, argc, argv, and envp arguments".into(),
            ));
        }
        let function_ptr = self.emit_operand(&args[0], Some("ptr"))?;
        let flags = self.emit_operand(&args[1], Some("i32"))?;
        let arg_count = self.emit_operand(&args[2], Some("i32"))?;
        let arg_values_ptr = self.emit_operand(&args[3], Some("ptr"))?;
        let env_ptr = self.emit_operand(&args[4], Some("ptr"))?;
        let mut call_args = Vec::new();
        call_args.push(format!("ptr {}", function_ptr.repr()));
        call_args.push(format!("i32 {}", flags.repr()));
        call_args.push(format!("i32 {}", arg_count.repr()));
        call_args.push(format!("ptr {}", arg_values_ptr.repr()));
        call_args.push(format!("ptr {}", env_ptr.repr()));
        self.emit_startup_scalar_call(
            "chic_rt_startup_call_entry",
            "i32",
            call_args,
            destination,
            target,
        )
    }

    pub(super) fn emit_startup_call_entry_async(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 5 {
            return Err(Error::Codegen(
                "startup call_entry_async expects function, flags, argc, argv, and envp arguments"
                    .into(),
            ));
        }
        let function_ptr = self.emit_operand(&args[0], Some("ptr"))?;
        let flags = self.emit_operand(&args[1], Some("i32"))?;
        let arg_count = self.emit_operand(&args[2], Some("i32"))?;
        let arg_values_ptr = self.emit_operand(&args[3], Some("ptr"))?;
        let env_ptr = self.emit_operand(&args[4], Some("ptr"))?;
        let mut call_args = Vec::new();
        call_args.push(format!("ptr {}", function_ptr.repr()));
        call_args.push(format!("i32 {}", flags.repr()));
        call_args.push(format!("i32 {}", arg_count.repr()));
        call_args.push(format!("ptr {}", arg_values_ptr.repr()));
        call_args.push(format!("ptr {}", env_ptr.repr()));
        self.emit_startup_scalar_call(
            "chic_rt_startup_call_entry_async",
            "ptr",
            call_args,
            destination,
            target,
        )
    }

    pub(super) fn emit_startup_complete_entry_async(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 2 {
            return Err(Error::Codegen(
                "startup complete_entry_async expects task pointer and flag arguments".into(),
            ));
        }
        let task = self.emit_operand(&args[0], Some("ptr"))?;
        let flags = self.emit_operand(&args[1], Some("i32"))?;
        let call_args = vec![
            format!("ptr {}", task.repr()),
            format!("i32 {}", flags.repr()),
        ];
        self.emit_startup_scalar_call(
            "chic_rt_startup_complete_entry_async",
            "i32",
            call_args,
            destination,
            target,
        )
    }

    pub(super) fn emit_startup_call_testcase(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 1 {
            return Err(Error::Codegen(
                "startup call_testcase expects a function pointer argument".into(),
            ));
        }
        let function_ptr = self.emit_operand(&args[0], Some("ptr"))?;
        let call_args = vec![format!("ptr {}", function_ptr.repr())];
        self.emit_startup_scalar_call(
            "chic_rt_startup_call_testcase",
            "i32",
            call_args,
            destination,
            target,
        )
    }

    pub(super) fn emit_startup_call_testcase_async(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 1 {
            return Err(Error::Codegen(
                "startup call_testcase_async expects a function pointer argument".into(),
            ));
        }
        let function_ptr = self.emit_operand(&args[0], Some("ptr"))?;
        let call_args = vec![format!("ptr {}", function_ptr.repr())];
        self.emit_startup_scalar_call(
            "chic_rt_startup_call_testcase_async",
            "ptr",
            call_args,
            destination,
            target,
        )
    }

    pub(super) fn emit_startup_complete_testcase_async(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 1 {
            return Err(Error::Codegen(
                "startup complete_testcase_async expects a task pointer argument".into(),
            ));
        }
        let task = self.emit_operand(&args[0], Some("ptr"))?;
        let call_args = vec![format!("ptr {}", task.repr())];
        self.emit_startup_scalar_call(
            "chic_rt_startup_complete_testcase_async",
            "i32",
            call_args,
            destination,
            target,
        )
    }

    pub(super) fn emit_startup_string_call(
        &mut self,
        symbol: &'static str,
        args: &[Operand],
        arg_types: Vec<String>,
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        self.emit_startup_call_with_destination(symbol, args, arg_types, destination, target)
    }

    pub(super) fn emit_startup_call_with_destination(
        &mut self,
        symbol: &'static str,
        args: &[Operand],
        arg_types: Vec<String>,
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        let place = destination
            .ok_or_else(|| Error::Codegen(format!("{symbol} call must assign to a destination")))?;
        let rendered =
            render_args_for_types(self, &arg_types, args, &format!("{symbol} call arguments"))?;
        let call_ty = self.place_type(place)?.ok_or_else(|| {
            Error::Codegen(format!("{symbol} destination missing type information"))
        })?;
        self.externals.insert(symbol);
        if call_ty == LLVM_STRING_TYPE {
            let out_ptr = self.place_ptr(place)?;
            if rendered.repr.is_empty() {
                writeln!(&mut self.builder, "  call void @{symbol}(ptr {out_ptr})").ok();
            } else {
                writeln!(
                    &mut self.builder,
                    "  call void @{symbol}(ptr {out_ptr}, {})",
                    rendered.repr
                )
                .ok();
            }
            let dest_label = self.block_label(target)?;
            writeln!(&mut self.builder, "  br label %{dest_label}").ok();
            return Ok(());
        }
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = call {call_ty} @{symbol}({})",
            rendered.repr
        )
        .ok();
        self.store_place(place, &ValueRef::new(tmp, &call_ty))?;
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_scalar_call(
        &mut self,
        symbol: &'static str,
        ret_ty: &str,
        args: Vec<String>,
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        self.externals.insert(symbol);
        let args_repr = args.join(", ");
        if let Some(place) = destination {
            let tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {tmp} = call {ret_ty} @{symbol}({args_repr})"
            )
            .ok();
            self.store_place(place, &ValueRef::new(tmp, ret_ty))?;
        } else {
            writeln!(&mut self.builder, "  call {ret_ty} @{symbol}({args_repr})").ok();
        }
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_io_write(
        &mut self,
        symbol: &'static str,
        args: &[Operand],
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 1 {
            return Err(Error::Codegen(format!(
                "{symbol} expects exactly one string argument"
            )));
        }
        let place = match &args[0] {
            Operand::Copy(place) | Operand::Move(place) => place,
            Operand::Borrow(borrow) => &borrow.place,
            _ => {
                return Err(Error::Codegen(format!(
                    "{symbol} argument must reference a string place"
                )));
            }
        };
        let ptr = self.place_ptr(place)?;
        self.externals.insert(symbol);
        writeln!(&mut self.builder, "  call i32 @{symbol}(ptr {ptr})").ok();
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn emit_startup_io_flush(
        &mut self,
        symbol: &'static str,
        target: BlockId,
    ) -> Result<(), Error> {
        self.externals.insert(symbol);
        writeln!(&mut self.builder, "  call i32 @{symbol}()").ok();
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    fn emit_async_task_header(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 1 {
            return Err(Error::Codegen(
                "RuntimeExports::TaskHeader expects a single task argument".into(),
            ));
        }
        let place = match &args[0] {
            Operand::Copy(place) | Operand::Move(place) => place,
            Operand::Borrow(borrow) => &borrow.place,
            _ => {
                return Err(Error::Codegen(
                    "RuntimeExports::TaskHeader argument must be a place".into(),
                ));
            }
        };
        let task_ty = self.mir_ty_of_place(place)?;
        let canonical = task_ty.canonical_name();
        let layout = self.type_layouts.layout_for_name(&canonical);
        let header_offset = layout.and_then(|layout| match layout {
            TypeLayout::Struct(struct_layout) | TypeLayout::Class(struct_layout) => struct_layout
                .fields
                .iter()
                .find(|field| field.name == "Header")
                .and_then(|field| field.offset),
            _ => None,
        });
        let base_ptr = self.place_ptr(place)?;
        let header_ptr = if let Some(offset) = header_offset {
            self.offset_ptr(&base_ptr, offset)?
        } else {
            base_ptr.clone()
        };
        if let Some(dest) = destination {
            let dest_ty = self.place_type(dest)?.unwrap_or_else(|| "ptr".to_string());
            self.store_place(dest, &ValueRef::new(header_ptr.clone(), &dest_ty))?;
        }
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(super) fn runtime_intrinsic_matches(canonical: &str, name: &str) -> bool {
        canonical == name
            || canonical.ends_with(&format!("::{name}"))
            || canonical == format!("RuntimeIntrinsics::{name}")
            || canonical.ends_with(&format!("::RuntimeIntrinsics::{name}"))
            || canonical == format!("StartupRuntimeState::{name}")
            || canonical.ends_with(&format!("::StartupRuntimeState::{name}"))
    }

    pub(super) fn note_async_runtime_intrinsic(&mut self, repr: &str) {
        let canonical = canonical_function_name(repr);
        const ASYNC_INTRINSICS: [(&str, &str); 15] = [
            ("chic_rt_async_block_on", "chic_rt_async_block_on"),
            ("chic_rt_async_spawn", "chic_rt_async_spawn"),
            ("chic_rt_async_spawn_local", "chic_rt_async_spawn_local"),
            ("chic_rt_async_scope", "chic_rt_async_scope"),
            ("chic_rt_async_cancel", "chic_rt_async_cancel"),
            ("chic_rt_async_task_result", "chic_rt_async_task_result"),
            ("chic_rt_async_task_header", "chic_rt_async_task_header"),
            (
                "chic_rt_async_task_bool_result",
                "chic_rt_async_task_bool_result",
            ),
            (
                "chic_rt_async_task_int_result",
                "chic_rt_async_task_int_result",
            ),
            (
                "chic_rt_async_register_future",
                "chic_rt_async_register_future",
            ),
            ("chic_rt_async_token_state", "chic_rt_async_token_state"),
            ("chic_rt_async_token_cancel", "chic_rt_async_token_cancel"),
            ("chic_rt_async_token_new", "chic_rt_async_token_new"),
            ("chic_rt_await", "chic_rt_await"),
            ("chic_rt_yield", "chic_rt_yield"),
        ];
        for (suffix, symbol) in ASYNC_INTRINSICS {
            if Self::runtime_intrinsic_matches(&canonical, suffix) {
                self.externals.insert(symbol);
            }
        }
    }

    pub(super) fn pointer_int_type(&self) -> &'static str {
        match self.arch {
            crate::target::TargetArch::X86_64 | crate::target::TargetArch::Aarch64 => "i64",
        }
    }
}

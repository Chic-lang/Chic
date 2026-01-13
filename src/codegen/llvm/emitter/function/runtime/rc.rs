use std::fmt::Write;

use crate::error::Error;
use crate::mir::{Operand, Place, Rvalue, Ty};

use super::super::builder::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn place_is_rc(&self, place: &Place) -> Result<bool, Error> {
        let ty = self.mir_ty_of_place(place)?;
        Ok(matches!(ty, Ty::Rc(_)))
    }

    pub(crate) fn place_is_arc(&self, place: &Place) -> Result<bool, Error> {
        let ty = self.mir_ty_of_place(place)?;
        Ok(matches!(ty, Ty::Arc(_)))
    }

    pub(crate) fn emit_rc_assignment(
        &mut self,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src)) => {
                if self.place_is_rc(src)? {
                    self.emit_rc_clone(place, src)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Rvalue::Use(Operand::Move(src)) => {
                if self.place_is_rc(src)? {
                    self.emit_rc_clone(place, src)?;
                    self.emit_rc_drop(src)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn emit_arc_assignment(
        &mut self,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src)) => {
                if self.place_is_arc(src)? {
                    self.emit_arc_clone(place, src)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Rvalue::Use(Operand::Move(src)) => {
                if self.place_is_arc(src)? {
                    self.emit_arc_clone(place, src)?;
                    self.emit_arc_drop(src)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn emit_rc_clone(&mut self, dest: &Place, src: &Place) -> Result<(), Error> {
        let dest_ptr = self.place_ptr(dest)?;
        let src_ptr = self.place_ptr(src)?;
        self.externals.insert("chic_rt_rc_clone");
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_rc_clone(ptr {dest_ptr}, ptr {src_ptr})"
        )
        .ok();
        Ok(())
    }

    pub(crate) fn emit_arc_clone(&mut self, dest: &Place, src: &Place) -> Result<(), Error> {
        let dest_ptr = self.place_ptr(dest)?;
        let src_ptr = self.place_ptr(src)?;
        self.externals.insert("chic_rt_arc_clone");
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_arc_clone(ptr {dest_ptr}, ptr {src_ptr})"
        )
        .ok();
        Ok(())
    }

    pub(crate) fn emit_rc_drop(&mut self, place: &Place) -> Result<(), Error> {
        let ptr = self.place_ptr(place)?;
        self.externals.insert("chic_rt_rc_drop");
        writeln!(&mut self.builder, "  call void @chic_rt_rc_drop(ptr {ptr})").ok();
        Ok(())
    }

    pub(crate) fn emit_arc_drop(&mut self, place: &Place) -> Result<(), Error> {
        let ptr = self.place_ptr(place)?;
        self.externals.insert("chic_rt_arc_drop");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_arc_drop(ptr {ptr})"
        )
        .ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::CpuIsaTier;
    use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
    use crate::codegen::llvm::signatures::LlvmFunctionSignature;
    use crate::codegen::llvm::types::map_type_owned;
    use crate::mir::{ArcTy, RcTy};
    use crate::mir::{
        FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, Operand, Place,
        Rvalue, Ty, TypeLayoutTable,
    };
    use crate::target::TargetArch;
    use std::collections::{BTreeSet, HashMap, HashSet};

    fn rc_ty() -> Ty {
        Ty::Rc(RcTy {
            element: Box::new(Ty::named("int")),
        })
    }

    fn arc_ty() -> Ty {
        Ty::Arc(ArcTy {
            element: Box::new(Ty::named("int")),
        })
    }

    fn with_emitter<F, R>(
        local_tys: Vec<Ty>,
        ptrs: Vec<Option<&str>>,
        f: F,
    ) -> (R, String, BTreeSet<&'static str>)
    where
        F: FnOnce(&mut FunctionEmitter<'_>) -> R,
    {
        let mut body = MirBody::new(0, None);
        for ty in &local_tys {
            body.locals.push(LocalDecl::new(
                None,
                ty.clone(),
                false,
                None,
                LocalKind::Local,
            ));
        }
        let function = MirFunction {
            name: "Rc::demo".into(),
            kind: FunctionKind::Function,
            signature: FnSig::empty(),
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };
        let signatures: HashMap<String, LlvmFunctionSignature> = HashMap::new();
        let mut externals: BTreeSet<&'static str> = BTreeSet::new();
        let vtable_symbols: HashSet<String> = HashSet::new();
        let trait_vtables = Vec::new();
        let class_vtables = Vec::new();
        let statics: Vec<crate::mir::StaticVar> = Vec::new();
        let str_literals: HashMap<crate::mir::StrId, StrLiteralInfo> = HashMap::new();
        let type_layouts = TypeLayoutTable::default();
        let mut metadata = MetadataRegistry::new();
        let target = crate::target::Target::parse("aarch64-unknown-linux-gnu").expect("target");
        let mut emitter = FunctionEmitter::new(
            &function,
            &signatures,
            &mut externals,
            &vtable_symbols,
            &trait_vtables,
            &class_vtables,
            CpuIsaTier::Baseline,
            &[CpuIsaTier::Baseline],
            TargetArch::Aarch64,
            &target,
            &statics,
            &str_literals,
            &type_layouts,
            &mut metadata,
            None,
        );
        let llvm_tys = local_tys
            .iter()
            .map(|ty| map_type_owned(ty, Some(&type_layouts)).expect("map type"))
            .collect();
        emitter.set_local_types_for_tests(llvm_tys);
        emitter.local_ptrs = ptrs.into_iter().map(|p| p.map(str::to_string)).collect();

        let result = f(&mut emitter);
        let ir = emitter.ir().to_string();
        (result, ir, externals)
    }

    #[test]
    fn rc_assignment_copy_emits_clone_only() {
        let dest = Place::new(LocalId(0));
        let src = Place::new(LocalId(1));
        let (result, ir, externals) = with_emitter(
            vec![rc_ty(), rc_ty()],
            vec![Some("%dest"), Some("%src")],
            |emitter| emitter.emit_rc_assignment(&dest, &Rvalue::Use(Operand::Copy(src.clone()))),
        );
        assert!(result.expect("rc copy assignment should succeed"));
        assert!(
            externals.contains("chic_rt_rc_clone"),
            "rc clone external should be recorded"
        );
        assert!(
            ir.contains("call i32 @chic_rt_rc_clone(ptr %dest, ptr %src)"),
            "clone call should be emitted"
        );
        assert!(
            !ir.contains("rc_drop"),
            "rc drop should not appear for copy assignments"
        );
    }

    #[test]
    fn rc_assignment_move_emits_clone_and_drop() {
        let dest = Place::new(LocalId(0));
        let src = Place::new(LocalId(1));
        let (result, ir, externals) = with_emitter(
            vec![rc_ty(), rc_ty()],
            vec![Some("%dest"), Some("%src")],
            |emitter| emitter.emit_rc_assignment(&dest, &Rvalue::Use(Operand::Move(src.clone()))),
        );
        assert!(result.expect("rc move assignment should succeed"));
        assert!(externals.contains("chic_rt_rc_clone"));
        assert!(externals.contains("chic_rt_rc_drop"));
        let clone_idx = ir
            .find("call i32 @chic_rt_rc_clone(ptr %dest, ptr %src)")
            .expect("clone should be emitted");
        let drop_idx = ir
            .find("call void @chic_rt_rc_drop(ptr %src)")
            .expect("drop should be emitted");
        assert!(
            clone_idx < drop_idx,
            "clone should precede drop in move assignments"
        );
    }

    #[test]
    fn rc_assignment_ignores_non_rc_values() {
        let dest = Place::new(LocalId(0));
        let int_place = Place::new(LocalId(1));
        let (result, ir, externals) = with_emitter(
            vec![rc_ty(), Ty::named("int")],
            vec![Some("%dest"), Some("%int")],
            |emitter| emitter.emit_rc_assignment(&dest, &Rvalue::Use(Operand::Copy(int_place))),
        );
        assert!(
            !result.expect("non-rc assignment should return false"),
            "non-rc sources should fall through"
        );
        assert!(ir.trim().is_empty(), "non-rc path should not emit IR");
        assert!(
            externals.is_empty(),
            "non-rc path should not record externals"
        );
    }

    #[test]
    fn arc_assignment_copy_and_move_cover_clone_and_drop() {
        let dest = Place::new(LocalId(0));
        let src = Place::new(LocalId(1));
        let (copy_result, copy_ir, copy_ext) = with_emitter(
            vec![arc_ty(), arc_ty()],
            vec![Some("%dest"), Some("%src")],
            |emitter| emitter.emit_arc_assignment(&dest, &Rvalue::Use(Operand::Copy(src.clone()))),
        );
        assert!(copy_result.expect("arc copy assignment should succeed"));
        assert!(
            copy_ext.contains("chic_rt_arc_clone"),
            "arc clone external should be recorded on copy"
        );
        assert!(
            copy_ir.contains("call i32 @chic_rt_arc_clone(ptr %dest, ptr %src)"),
            "arc copy should emit clone call"
        );
        assert!(
            !copy_ir.contains("arc_drop"),
            "arc drop should not be emitted for copy"
        );

        let (move_result, move_ir, move_ext) = with_emitter(
            vec![arc_ty(), arc_ty()],
            vec![Some("%dest"), Some("%src")],
            |emitter| emitter.emit_arc_assignment(&dest, &Rvalue::Use(Operand::Move(src))),
        );
        assert!(move_result.expect("arc move assignment should succeed"));
        assert!(move_ext.contains("chic_rt_arc_clone"));
        assert!(move_ext.contains("chic_rt_arc_drop"));
        assert!(
            move_ir.contains("call i32 @chic_rt_arc_clone(ptr %dest, ptr %src)"),
            "arc move should clone source"
        );
        assert!(
            move_ir.contains("call void @chic_rt_arc_drop(ptr %src)"),
            "arc move should drop source"
        );
    }

    #[test]
    fn arc_assignment_falls_through_for_non_arc() {
        let dest = Place::new(LocalId(0));
        let bool_place = Place::new(LocalId(1));
        let (result, ir, externals) = with_emitter(
            vec![arc_ty(), Ty::named("bool")],
            vec![Some("%dest"), Some("%bool")],
            |emitter| emitter.emit_arc_assignment(&dest, &Rvalue::Use(Operand::Copy(bool_place))),
        );
        assert!(
            !result.expect("non-arc assignment should return false"),
            "non-arc sources should not be handled by arc assignment"
        );
        assert!(ir.trim().is_empty(), "non-arc path should not emit IR");
        assert!(
            externals.is_empty(),
            "non-arc path should not record externals"
        );
    }

    #[test]
    fn place_is_rc_and_arc_detect_correct_types() {
        let rc_place = Place::new(LocalId(0));
        let arc_place = Place::new(LocalId(1));
        let (result, _, _) = with_emitter(
            vec![rc_ty(), arc_ty(), Ty::named("int")],
            vec![Some("%rc"), Some("%arc"), Some("%int")],
            |emitter| {
                (
                    emitter.place_is_rc(&rc_place),
                    emitter.place_is_arc(&arc_place),
                    emitter.place_is_rc(&Place::new(LocalId(2))),
                )
            },
        );
        let (rc_ok, arc_ok, rc_false) = result;
        assert!(rc_ok.expect("rc place should be recognised as Rc"));
        assert!(arc_ok.expect("arc place should be recognised as Arc"));
        assert!(
            !rc_false.expect("non-rc place should return false"),
            "non-rc should not be treated as Rc"
        );
    }
}

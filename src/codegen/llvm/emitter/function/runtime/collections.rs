use std::fmt::Write;

use crate::codegen::llvm::emitter::literals::LLVM_VEC_TYPE;
use crate::error::Error;
use crate::mir::{Operand, Place, Rvalue, Ty};

use super::super::builder::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn place_is_vec(&self, place: &Place) -> Result<bool, Error> {
        let ty = self.mir_ty_of_place(place)?;
        Ok(matches!(ty, Ty::Vec(_) | Ty::Array(_)))
    }

    pub(crate) fn emit_vec_assignment(
        &mut self,
        place: &Place,
        value: &Rvalue,
    ) -> Result<bool, Error> {
        match value {
            Rvalue::Use(Operand::Copy(src)) => {
                if self.place_is_vec(src)? {
                    self.emit_vec_clone(place, src)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Rvalue::Use(Operand::Move(src)) => {
                if self.place_is_vec(src)? {
                    self.emit_vec_clone(place, src)?;
                    self.emit_vec_drop(src)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn emit_vec_clone(&mut self, dest: &Place, src: &Place) -> Result<(), Error> {
        let dest_ptr = self.prepare_vec_destination(dest)?;
        let src_ptr = self.place_ptr(src)?;
        self.externals.insert("chic_rt_vec_clone");
        writeln!(
            &mut self.builder,
            "  call i32 @chic_rt_vec_clone(ptr {dest_ptr}, ptr {src_ptr})"
        )
        .ok();
        Ok(())
    }

    pub(crate) fn emit_vec_drop(&mut self, place: &Place) -> Result<(), Error> {
        let ptr = self.place_ptr(place)?;
        self.externals.insert("chic_rt_vec_drop");
        writeln!(
            &mut self.builder,
            "  call void @chic_rt_vec_drop(ptr {ptr})"
        )
        .ok();
        Ok(())
    }

    fn prepare_vec_destination(&mut self, place: &Place) -> Result<String, Error> {
        let ptr = self.place_ptr(place)?;
        writeln!(
            &mut self.builder,
            "  store {LLVM_VEC_TYPE} zeroinitializer, ptr {ptr}"
        )
        .ok();
        Ok(ptr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::CpuIsaTier;
    use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
    use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
    use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
    use crate::codegen::llvm::signatures::LlvmFunctionSignature;
    use crate::codegen::llvm::types::map_type_owned;
    use crate::mir::{
        ArrayTy, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, Operand,
        Place, Rvalue, Ty, TypeLayoutTable, VecTy,
    };
    use crate::target::TargetArch;
    use std::collections::{BTreeSet, HashMap, HashSet};

    fn vec_ty() -> Ty {
        Ty::Vec(VecTy::new(Box::new(Ty::named("int"))))
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
            name: "Vec::demo".into(),
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
    fn vec_assignment_clone_from_copy_inserts_clone_call() {
        let dest = Place::new(LocalId(0));
        let src = Place::new(LocalId(1));
        let (result, ir, externals) = with_emitter(
            vec![vec_ty(), vec_ty()],
            vec![Some("%dest"), Some("%src")],
            |emitter| emitter.emit_vec_assignment(&dest, &Rvalue::Use(Operand::Copy(src.clone()))),
        );
        assert!(result.expect("vec assignment should succeed"));
        assert!(
            externals.contains("chic_rt_vec_clone"),
            "vec clone external should be recorded"
        );
        assert!(
            ir.contains(&format!("store {LLVM_VEC_TYPE} zeroinitializer, ptr %dest")),
            "destination should be zero-initialised"
        );
        assert!(
            ir.contains("call i32 @chic_rt_vec_clone(ptr %dest, ptr %src)"),
            "clone call should be emitted for copy assignments"
        );
        assert!(
            !ir.contains("vec_drop"),
            "copy-based assignment should not emit drop"
        );
    }

    #[test]
    fn vec_assignment_move_clones_and_drops_source() {
        let dest = Place::new(LocalId(0));
        let src = Place::new(LocalId(1));
        let (result, ir, externals) = with_emitter(
            vec![vec_ty(), vec_ty()],
            vec![Some("%dest"), Some("%src")],
            |emitter| emitter.emit_vec_assignment(&dest, &Rvalue::Use(Operand::Move(src.clone()))),
        );
        assert!(result.expect("vec move assignment should succeed"));
        assert!(
            externals.contains("chic_rt_vec_clone"),
            "move assignment should still clone the source"
        );
        assert!(
            externals.contains("chic_rt_vec_drop"),
            "move assignment should drop the source after cloning"
        );
        assert!(
            ir.contains("call i32 @chic_rt_vec_clone(ptr %dest, ptr %src)"),
            "clone call should be emitted before drop"
        );
        assert!(
            ir.contains("call void @chic_rt_vec_drop(ptr %src)"),
            "move assignment should drop the source place"
        );
    }

    #[test]
    fn vec_assignment_ignores_non_vec_sources() {
        let dest = Place::new(LocalId(0));
        let int_place = Place::new(LocalId(1));
        let (result, ir, externals) = with_emitter(
            vec![vec_ty(), Ty::named("int")],
            vec![Some("%dest"), Some("%int")],
            |emitter| emitter.emit_vec_assignment(&dest, &Rvalue::Use(Operand::Copy(int_place))),
        );
        assert!(
            !result.expect("non-vec assignment should return false"),
            "non-vec sources should not be handled by vec assignment"
        );
        assert!(ir.trim().is_empty(), "non-vec handling should not emit IR");
        assert!(
            externals.is_empty(),
            "non-vec handling should not register externals"
        );
    }

    #[test]
    fn place_is_vec_handles_arrays() {
        let array_ty = Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 4));
        let place = Place::new(LocalId(0));
        let (is_vec, _, _) = with_emitter(vec![array_ty], vec![Some("%array")], |emitter| {
            emitter.place_is_vec(&place)
        });
        assert!(is_vec.expect("arrays should be treated as vec-like"));
    }
}

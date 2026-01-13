use std::collections::HashMap;

use crate::chic_kind::ChicKind;
use crate::codegen::{CodegenOptions, CpuIsaTier};
use crate::drop_glue::SynthesisedDropGlue;
use crate::eq_glue::SynthesisedEqGlue;
use crate::hash_glue::SynthesisedHashGlue;
use crate::mir::MirModule;
use crate::perf::PerfMetadata;
use crate::target::{Target, TargetArch};
use crate::type_metadata::SynthesisedTypeMetadata;

use crate::codegen::llvm::signatures::LlvmFunctionSignature;

pub(crate) struct LlvmEmitContext<'a> {
    pub(crate) mir: &'a MirModule,
    pub(crate) global_mir: Option<&'a MirModule>,
    pub(crate) perf: &'a PerfMetadata,
    pub(crate) signatures: &'a HashMap<String, LlvmFunctionSignature>,
    pub(crate) async_vtables: HashMap<String, String>,
    pub(crate) entry: Option<&'a str>,
    pub(crate) kind: ChicKind,
    pub(crate) target: &'a Target,
    pub(crate) options: &'a CodegenOptions,
    drop_glue: &'a [SynthesisedDropGlue],
    hash_glue: &'a [SynthesisedHashGlue],
    eq_glue: &'a [SynthesisedEqGlue],
    type_metadata: &'a [SynthesisedTypeMetadata],
    isa_tiers: Vec<CpuIsaTier>,
    multiversion: bool,
    is_apple_target: bool,
}

impl<'a> LlvmEmitContext<'a> {
    pub(crate) fn new(
        mir: &'a MirModule,
        global_mir: Option<&'a MirModule>,
        perf: &'a PerfMetadata,
        signatures: &'a HashMap<String, LlvmFunctionSignature>,
        async_vtables: HashMap<String, String>,
        entry: Option<&'a str>,
        kind: ChicKind,
        target_triple: &'a str,
        target: &'a Target,
        options: &'a CodegenOptions,
        drop_glue: &'a [SynthesisedDropGlue],
        hash_glue: &'a [SynthesisedHashGlue],
        eq_glue: &'a [SynthesisedEqGlue],
        type_metadata: &'a [SynthesisedTypeMetadata],
    ) -> Self {
        let arch = target.arch();
        let mut isa_tiers = options.cpu_isa.effective_tiers(arch);
        let is_apple_target = target_triple.contains("apple");
        if is_apple_target {
            isa_tiers.retain(|tier| !matches!(tier, CpuIsaTier::Sve | CpuIsaTier::Sve2));
        }
        let multiversion = match arch {
            TargetArch::X86_64 | TargetArch::Aarch64 => isa_tiers.len() > 1,
        };
        Self {
            mir,
            global_mir,
            perf,
            signatures,
            async_vtables,
            entry,
            kind,
            target,
            options,
            drop_glue,
            hash_glue,
            eq_glue,
            type_metadata,
            isa_tiers,
            multiversion,
            is_apple_target,
        }
    }

    pub(crate) fn arch(&self) -> TargetArch {
        self.target.arch()
    }

    pub(crate) fn isa_tiers(&self) -> &[CpuIsaTier] {
        &self.isa_tiers
    }

    pub(crate) fn multiversion_enabled(&self) -> bool {
        self.multiversion
    }

    pub(crate) fn is_apple_target(&self) -> bool {
        self.is_apple_target
    }

    pub(crate) fn options(&self) -> &CodegenOptions {
        self.options
    }

    pub(crate) fn drop_glue(&self) -> &[SynthesisedDropGlue] {
        self.drop_glue
    }

    pub(crate) fn hash_glue(&self) -> &[SynthesisedHashGlue] {
        self.hash_glue
    }

    pub(crate) fn eq_glue(&self) -> &[SynthesisedEqGlue] {
        self.eq_glue
    }

    pub(crate) fn type_metadata(&self) -> &[SynthesisedTypeMetadata] {
        self.type_metadata
    }

    pub(crate) fn async_vtable_symbol(&self, function: &str) -> Option<&String> {
        self.async_vtables.get(function)
    }
}

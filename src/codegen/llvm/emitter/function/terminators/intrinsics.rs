use std::fmt::Write;

use crate::codegen::CpuIsaTier;
use crate::error::Error;
use crate::mir::{BlockId, Operand, Place};
use crate::target::TargetArch;

use super::super::builder::FunctionEmitter;
use super::super::values::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_simd_f32x8_fma(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 3 {
            return Err(Error::Codegen(
                "std.simd.f32x8.fma expects three arguments".into(),
            ));
        }
        let vector_ty = "<8 x float>";
        let multiplicand = self.emit_operand(&args[0], Some(vector_ty))?;
        let multiplier = self.emit_operand(&args[1], Some(vector_ty))?;
        let addend = self.emit_operand(&args[2], Some(vector_ty))?;

        let result = if self.isa_tier >= CpuIsaTier::Avx2 {
            self.externals.insert("llvm.fma.v8f32");
            let call_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {call_tmp} = call {vector_ty} @llvm.fma.v8f32({vector_ty} {}, {vector_ty} {}, {vector_ty} {})",
                multiplicand.repr(),
                multiplier.repr(),
                addend.repr()
            )
            .ok();
            ValueRef::new(call_tmp, vector_ty)
        } else {
            let mul_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {mul_tmp} = fmul {vector_ty} {}, {}",
                multiplicand.repr(),
                multiplier.repr()
            )
            .ok();
            let add_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {add_tmp} = fadd {vector_ty} {mul_tmp}, {}",
                addend.repr()
            )
            .ok();
            ValueRef::new(add_tmp, vector_ty)
        };

        if let Some(place) = destination {
            self.store_place(place, &result)?;
        }

        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(crate) fn emit_simd_f32x4_fma(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if self.arch != TargetArch::Aarch64 {
            return Err(Error::Codegen(
                "std.simd.f32x4.fma is only supported on AArch64 targets".into(),
            ));
        }
        if args.len() != 3 {
            return Err(Error::Codegen(
                "std.simd.f32x4.fma expects three arguments".into(),
            ));
        }

        let vector_ty = "<4 x float>";
        let multiplicand = self.emit_operand(&args[0], Some(vector_ty))?;
        let multiplier = self.emit_operand(&args[1], Some(vector_ty))?;
        let addend = self.emit_operand(&args[2], Some(vector_ty))?;

        self.externals.insert("llvm.aarch64.neon.fmla.v4f32");
        let call_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {call_tmp} = call {vector_ty} @llvm.aarch64.neon.fmla.v4f32({vector_ty} {}, {vector_ty} {}, {vector_ty} {})",
            addend.repr(),
            multiplicand.repr(),
            multiplier.repr()
        )
        .ok();

        let result = ValueRef::new(call_tmp, vector_ty);
        if let Some(place) = destination {
            self.store_place(place, &result)?;
        }
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(crate) fn emit_simd_f16x8_fma(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if self.arch != TargetArch::Aarch64 {
            return Err(Error::Codegen(
                "std.simd.f16x8.fma is only supported on AArch64 targets".into(),
            ));
        }
        if args.len() != 3 {
            return Err(Error::Codegen(
                "std.simd.f16x8.fma expects three arguments".into(),
            ));
        }
        if self.isa_tier < CpuIsaTier::Fp16Fml {
            self.externals.insert("llvm.trap");
            writeln!(&mut self.builder, "  call void @llvm.trap()").ok();
            writeln!(&mut self.builder, "  unreachable").ok();
            return Ok(());
        }

        let vector_ty = "<8 x half>";
        let multiplicand = self.emit_operand(&args[0], Some(vector_ty))?;
        let multiplier = self.emit_operand(&args[1], Some(vector_ty))?;
        let addend = self.emit_operand(&args[2], Some(vector_ty))?;

        self.externals.insert("llvm.aarch64.neon.fmla.v8f16");
        let call_tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {call_tmp} = call {vector_ty} @llvm.aarch64.neon.fmla.v8f16({vector_ty} {}, {vector_ty} {}, {vector_ty} {})",
            addend.repr(),
            multiplicand.repr(),
            multiplier.repr()
        )
        .ok();

        let result = ValueRef::new(call_tmp, vector_ty);
        if let Some(place) = destination {
            self.store_place(place, &result)?;
        }
        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(crate) fn emit_linalg_bf16_mmla(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
        require_sme: bool,
    ) -> Result<(), Error> {
        if self.arch != TargetArch::Aarch64 {
            return Err(Error::Codegen(
                "std.linalg.bf16x32.mmla is only supported on AArch64 targets".into(),
            ));
        }
        if args.len() != 3 {
            return Err(Error::Codegen(
                "std.linalg.bf16x32.mmla expects three arguments".into(),
            ));
        }
        if require_sme && self.isa_tier < CpuIsaTier::Sme {
            self.externals.insert("llvm.trap");
            writeln!(&mut self.builder, "  call void @llvm.trap()").ok();
            writeln!(&mut self.builder, "  unreachable").ok();
            return Ok(());
        }
        if self.isa_tier < CpuIsaTier::Bf16 {
            self.externals.insert("llvm.trap");
            writeln!(&mut self.builder, "  call void @llvm.trap()").ok();
            writeln!(&mut self.builder, "  unreachable").ok();
            return Ok(());
        }

        let acc_ty = "<4 x float>";
        let lanes_ty = "<8 x bfloat>";
        let acc = self.emit_operand(&args[0], Some(acc_ty))?;
        let lhs = self.emit_operand(&args[1], Some(lanes_ty))?;
        let rhs = self.emit_operand(&args[2], Some(lanes_ty))?;

        let result = if matches!(self.isa_tier, CpuIsaTier::Sve | CpuIsaTier::Sve2) {
            let sve_acc_ty = "<vscale x 4 x float>";
            let sve_lane_ty = "<vscale x 8 x bfloat>";
            self.externals.insert("llvm.aarch64.sve.bfmmla.nxv4f32");
            let acc_sve = self.bitcast_value(&acc, acc_ty, sve_acc_ty)?;
            let lhs_sve = self.bitcast_value(&lhs, lanes_ty, sve_lane_ty)?;
            let rhs_sve = self.bitcast_value(&rhs, lanes_ty, sve_lane_ty)?;
            let call_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {call_tmp} = call {sve_acc_ty} @llvm.aarch64.sve.bfmmla.nxv4f32({sve_acc_ty} {}, {sve_lane_ty} {}, {sve_lane_ty} {})",
                acc_sve.repr(),
                lhs_sve.repr(),
                rhs_sve.repr()
            )
            .ok();
            let result_sve = ValueRef::new(call_tmp, sve_acc_ty);
            self.bitcast_value(&result_sve, sve_acc_ty, acc_ty)?
        } else {
            let use_sme = require_sme || self.isa_tier >= CpuIsaTier::Sme;
            if use_sme {
                self.externals.insert("llvm.aarch64.sme.za.enable");
                writeln!(
                    &mut self.builder,
                    "  call void @llvm.aarch64.sme.za.enable()"
                )
                .ok();
            }

            self.externals.insert("llvm.aarch64.neon.bfmmla.v4f32");
            let call_tmp = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {call_tmp} = call {acc_ty} @llvm.aarch64.neon.bfmmla.v4f32({acc_ty} {}, {lanes_ty} {}, {lanes_ty} {})",
                acc.repr(),
                lhs.repr(),
                rhs.repr()
            )
            .ok();

            if use_sme {
                self.externals.insert("llvm.aarch64.sme.za.disable");
                writeln!(
                    &mut self.builder,
                    "  call void @llvm.aarch64.sme.za.disable()"
                )
                .ok();
            }

            ValueRef::new(call_tmp, acc_ty)
        };

        if let Some(place) = destination {
            self.store_place(place, &result)?;
        }

        let dest_label = self.block_label(target)?;
        writeln!(&mut self.builder, "  br label %{dest_label}").ok();
        Ok(())
    }

    pub(crate) fn emit_linalg_dpbusd(
        &mut self,
        args: &[Operand],
        destination: Option<&Place>,
        target: BlockId,
    ) -> Result<(), Error> {
        if args.len() != 3 {
            return Err(Error::Codegen(
                "std.linalg.int8x64.dpbusd expects three arguments".into(),
            ));
        }
        match self.arch {
            TargetArch::X86_64 => {
                if !self
                    .available_tiers
                    .iter()
                    .any(|tier| *tier >= CpuIsaTier::Avx512)
                {
                    return Err(Error::Codegen(
                        "std.linalg.int8x64.dpbusd requires --cpu-isa to include avx512 or higher"
                            .into(),
                    ));
                }

                if self.isa_tier < CpuIsaTier::Avx512 {
                    self.externals.insert("llvm.trap");
                    writeln!(&mut self.builder, "  call void @llvm.trap()").ok();
                    writeln!(&mut self.builder, "  unreachable").ok();
                    return Ok(());
                }

                let vector_ty = "<16 x i32>";
                let acc = self.emit_operand(&args[0], Some(vector_ty))?;
                let lhs = self.emit_operand(&args[1], Some(vector_ty))?;
                let rhs = self.emit_operand(&args[2], Some(vector_ty))?;

                self.externals.insert("llvm.x86.avx512.vpdpbusd.512");
                let call_tmp = self.new_temp();
                writeln!(
                    &mut self.builder,
                    "  {call_tmp} = call {vector_ty} @llvm.x86.avx512.vpdpbusd.512({vector_ty} {}, {vector_ty} {}, {vector_ty} {})",
                    acc.repr(),
                    lhs.repr(),
                    rhs.repr()
                )
                .ok();

                let result = ValueRef::new(call_tmp, vector_ty);
                if let Some(place) = destination {
                    self.store_place(place, &result)?;
                }

                let dest_label = self.block_label(target)?;
                writeln!(&mut self.builder, "  br label %{dest_label}").ok();
                Ok(())
            }
            TargetArch::Aarch64 => {
                let acc_ty = "<4 x i32>";
                let lanes_ty = "<16 x i8>";
                let acc = self.emit_operand(&args[0], Some(acc_ty))?;
                let lhs = self.emit_operand(&args[1], Some(lanes_ty))?;
                let rhs = self.emit_operand(&args[2], Some(lanes_ty))?;

                let result_ref = if matches!(self.isa_tier, CpuIsaTier::Sve | CpuIsaTier::Sve2) {
                    let sve_acc_ty = "<vscale x 4 x i32>";
                    let sve_lane_ty = "<vscale x 16 x i8>";
                    self.externals.insert("llvm.aarch64.sve.usmmla.nxv4i32");
                    let acc_sve = self.bitcast_value(&acc, acc_ty, sve_acc_ty)?;
                    let lhs_sve = self.bitcast_value(&lhs, lanes_ty, sve_lane_ty)?;
                    let rhs_sve = self.bitcast_value(&rhs, lanes_ty, sve_lane_ty)?;
                    let call_tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {call_tmp} = call {sve_acc_ty} @llvm.aarch64.sve.usmmla.nxv4i32({sve_acc_ty} {}, {sve_lane_ty} {}, {sve_lane_ty} {})",
                        acc_sve.repr(),
                        lhs_sve.repr(),
                        rhs_sve.repr()
                    )
                    .ok();
                    let result_sve = ValueRef::new(call_tmp, sve_acc_ty);
                    self.bitcast_value(&result_sve, sve_acc_ty, acc_ty)?
                } else if self.isa_tier >= CpuIsaTier::I8mm {
                    self.externals.insert("llvm.aarch64.neon.usmmla.v4i32");
                    let call_tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {call_tmp} = call {acc_ty} @llvm.aarch64.neon.usmmla.v4i32({acc_ty} {}, {lanes_ty} {}, {lanes_ty} {})",
                        acc.repr(),
                        lhs.repr(),
                        rhs.repr()
                    )
                    .ok();
                    ValueRef::new(call_tmp, acc_ty)
                } else if self.isa_tier >= CpuIsaTier::DotProd {
                    self.externals.insert("llvm.aarch64.neon.udot.v4i32.v16i8");
                    let call_tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {call_tmp} = call {acc_ty} @llvm.aarch64.neon.udot.v4i32.v16i8({acc_ty} {}, {lanes_ty} {}, {lanes_ty} {})",
                        acc.repr(),
                        lhs.repr(),
                        rhs.repr()
                    )
                    .ok();
                    ValueRef::new(call_tmp, acc_ty)
                } else {
                    self.externals.insert("llvm.trap");
                    writeln!(&mut self.builder, "  call void @llvm.trap()").ok();
                    writeln!(&mut self.builder, "  unreachable").ok();
                    return Ok(());
                };

                if let Some(place) = destination {
                    self.store_place(place, &result_ref)?;
                }

                let dest_label = self.block_label(target)?;
                writeln!(&mut self.builder, "  br label %{dest_label}").ok();
                Ok(())
            }
        }
    }
}

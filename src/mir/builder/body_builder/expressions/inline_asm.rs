use super::*;
use crate::syntax::expr::builders::{
    InlineAsmExpr as AstInlineAsmExpr, InlineAsmOperand as AstInlineAsmOperand,
    InlineAsmOperandMode as AstInlineAsmOperandMode, InlineAsmRegister as AstInlineAsmRegister,
    InlineAsmRegisterClass as AstInlineAsmRegisterClass,
    InlineAsmTemplateOperandRef as AstInlineAsmTemplateOperandRef,
    InlineAsmTemplatePiece as AstInlineAsmTemplatePiece,
};
use std::collections::HashMap;

impl BodyBuilder<'_> {
    pub(super) fn lower_inline_asm_expr(
        &mut self,
        asm: AstInlineAsmExpr,
        span: Option<Span>,
    ) -> Option<Operand> {
        if self.unsafe_depth == 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "inline assembly requires an `unsafe` block".into(),
                span: asm.span.or(span),
            });
        }

        let mut errored = false;
        let mut operands = Vec::new();
        for operand in asm.operands {
            match self.lower_inline_asm_operand(operand, span) {
                Some(op) => operands.push(op),
                None => errored = true,
            }
        }

        let clobbers = asm
            .clobbers
            .into_iter()
            .map(map_inline_asm_register)
            .collect::<Vec<_>>();

        let mut name_map = HashMap::new();
        for (index, operand) in operands.iter().enumerate() {
            if let Some(name) = &operand.name {
                if name_map.insert(name.clone(), index).is_some() {
                    errored = true;
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("duplicate inline assembly operand name `{name}`"),
                        span: operand.span.or(span),
                    });
                }
            }
        }

        let template = match self.lower_inline_asm_template(
            asm.template.pieces,
            &name_map,
            operands.len(),
            span,
        ) {
            Some(template) => template,
            None => {
                errored = true;
                Vec::new()
            }
        };

        if errored {
            return None;
        }

        let mir_asm = InlineAsm {
            template,
            operands,
            clobbers,
            options: InlineAsmOptions {
                volatile: asm.options.volatile,
                alignstack: asm.options.alignstack,
                intel_syntax: asm.options.intel_syntax,
                nomem: asm.options.nomem,
                nostack: asm.options.nostack,
                preserves_flags: asm.options.preserves_flags,
                pure: asm.options.pure,
                readonly: asm.options.readonly,
                noreturn: asm.options.noreturn,
            },
            span: asm.span.or(span),
        };

        self.push_statement(MirStatement {
            span: mir_asm.span,
            kind: MirStatementKind::InlineAsm(mir_asm),
        });

        Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
    }

    fn lower_inline_asm_template(
        &mut self,
        pieces: Vec<AstInlineAsmTemplatePiece>,
        name_map: &HashMap<String, usize>,
        operand_count: usize,
        span: Option<Span>,
    ) -> Option<Vec<InlineAsmTemplatePiece>> {
        let mut errored = false;
        let mut template = Vec::new();
        for piece in pieces {
            match piece {
                AstInlineAsmTemplatePiece::Literal(text) => {
                    template.push(InlineAsmTemplatePiece::Literal(text));
                }
                AstInlineAsmTemplatePiece::Placeholder {
                    operand,
                    modifier,
                    span: piece_span,
                } => {
                    let resolved = match operand {
                        AstInlineAsmTemplateOperandRef::Position(idx) => {
                            if idx >= operand_count {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "inline assembly placeholder references missing operand {idx}"
                                    ),
                                    span: piece_span.or(span),
                                });
                                errored = true;
                                None
                            } else {
                                Some(idx)
                            }
                        }
                        AstInlineAsmTemplateOperandRef::Named(name) => {
                            if let Some(idx) = name_map.get(&name).copied() {
                                Some(idx)
                            } else {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "inline assembly placeholder `{name}` has no matching operand"
                                    ),
                                    span: piece_span.or(span),
                                });
                                errored = true;
                                None
                            }
                        }
                    };
                    if let Some(idx) = resolved {
                        template.push(InlineAsmTemplatePiece::Placeholder {
                            operand_idx: idx,
                            modifier,
                            span: piece_span.or(span),
                        });
                    }
                }
            }
        }
        if errored { None } else { Some(template) }
    }

    fn lower_inline_asm_operand(
        &mut self,
        operand: AstInlineAsmOperand,
        span: Option<Span>,
    ) -> Option<InlineAsmOperand> {
        let reg = map_inline_asm_register(operand.reg);
        let operand_span = operand.span.or(span);
        let name = operand.name;
        match operand.mode {
            AstInlineAsmOperandMode::In { expr } => {
                let value = self.lower_expr_node(expr, operand_span)?;
                Some(InlineAsmOperand {
                    name,
                    reg,
                    kind: InlineAsmOperandKind::In { value },
                    span: operand_span,
                })
            }
            AstInlineAsmOperandMode::Out { expr, late } => {
                let place = self.lower_place_expr(expr, operand_span)?;
                Some(InlineAsmOperand {
                    name,
                    reg,
                    kind: InlineAsmOperandKind::Out { place, late },
                    span: operand_span,
                })
            }
            AstInlineAsmOperandMode::InOut {
                input,
                output,
                late,
            } => {
                let (value, place) = if let Some(output_expr) = output {
                    let value = self.lower_expr_node(input, operand_span)?;
                    let place = self.lower_place_expr(output_expr, operand_span)?;
                    (value, place)
                } else {
                    let place = self.lower_place_expr(input.clone(), operand_span)?;
                    let value = Operand::Copy(place.clone());
                    (value, place)
                };
                Some(InlineAsmOperand {
                    name,
                    reg,
                    kind: InlineAsmOperandKind::InOut {
                        input: value,
                        output: place,
                        late,
                    },
                    span: operand_span,
                })
            }
            AstInlineAsmOperandMode::Const { expr } => {
                let value = self.lower_expr_node(expr, operand_span)?;
                if !matches!(value, Operand::Const(_)) {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "inline assembly `const` operands must be compile-time constants"
                            .into(),
                        span: operand_span,
                    });
                    return None;
                }
                Some(InlineAsmOperand {
                    name,
                    reg,
                    kind: InlineAsmOperandKind::Const { value },
                    span: operand_span,
                })
            }
            AstInlineAsmOperandMode::Sym { path } => Some(InlineAsmOperand {
                name,
                reg,
                kind: InlineAsmOperandKind::Sym { symbol: path },
                span: operand_span,
            }),
        }
    }
}

fn map_inline_asm_register(reg: AstInlineAsmRegister) -> InlineAsmRegister {
    match reg {
        AstInlineAsmRegister::Class(class) => InlineAsmRegister::Class(match class {
            AstInlineAsmRegisterClass::Reg => InlineAsmRegisterClass::Reg,
            AstInlineAsmRegisterClass::Reg8 => InlineAsmRegisterClass::Reg8,
            AstInlineAsmRegisterClass::Reg16 => InlineAsmRegisterClass::Reg16,
            AstInlineAsmRegisterClass::Reg32 => InlineAsmRegisterClass::Reg32,
            AstInlineAsmRegisterClass::Reg64 => InlineAsmRegisterClass::Reg64,
            AstInlineAsmRegisterClass::Xmm => InlineAsmRegisterClass::Xmm,
            AstInlineAsmRegisterClass::Ymm => InlineAsmRegisterClass::Ymm,
            AstInlineAsmRegisterClass::Zmm => InlineAsmRegisterClass::Zmm,
            AstInlineAsmRegisterClass::Vreg => InlineAsmRegisterClass::Vreg,
            AstInlineAsmRegisterClass::Kreg => InlineAsmRegisterClass::Kreg,
        }),
        AstInlineAsmRegister::Explicit(name) => {
            InlineAsmRegister::Explicit(name.to_ascii_lowercase())
        }
    }
}

use super::*;
use crate::frontend::parser::parse_type_expression_text;

const TYPE_ID_INTRINSIC: &str = "__type_id_of";

body_builder_impl! {
    pub(crate) fn try_lower_type_id_intrinsic(
        &mut self,
        callee: &ExprNode,
        args: &[CallArgument],
        generics: &Option<Vec<String>>,
        span: Option<Span>,
    ) -> Option<Operand> {
        let ExprNode::Identifier(name) = callee else {
            return None;
        };
        if name.trim() != TYPE_ID_INTRINSIC {
            return None;
        }

        if let Some(arg) = args.first() {
            let arg_span = arg.span.or(arg.value_span).or(span);
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{TYPE_ID_INTRINSIC}` does not accept runtime arguments"),
                span: arg_span,
            });
        }

        let Some(generic_args) = generics else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{TYPE_ID_INTRINSIC}` requires a type argument (e.g. `{TYPE_ID_INTRINSIC}<MyType>()`)"),
                span,
            });
            return Some(Operand::Const(ConstOperand::new(ConstValue::UInt(0))));
        };

        let Some(type_text) = generic_args.first() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{TYPE_ID_INTRINSIC}` requires a type argument (e.g. `{TYPE_ID_INTRINSIC}<MyType>()`)"),
                span,
            });
            return Some(Operand::Const(ConstOperand::new(ConstValue::UInt(0))));
        };

        let trimmed = type_text.trim();
        let Some(type_expr) = parse_type_expression_text(trimmed) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{trimmed}` is not a valid type for `{TYPE_ID_INTRINSIC}`"),
                span,
            });
            return Some(Operand::Const(ConstOperand::new(ConstValue::UInt(0))));
        };

        let ty = Ty::from_type_expr(&type_expr);
        self.type_id_operand_for_ty(&ty, span)
    }
}

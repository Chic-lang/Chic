use super::*;
use crate::frontend::ast::{ConstructorInitTarget, ConstructorInitializer};

body_builder_impl! {
    pub(crate) fn lower_constructor_initializer(
        &mut self,
        initializer: &ConstructorInitializer,
        self_local: LocalId,
        class_name: &str,
    ) {
        let mut args = Vec::with_capacity(initializer.arguments.len() + 1);
        args.push(Operand::Copy(Place::new(self_local)));
        for argument in &initializer.arguments {
            let Some(operand) = self.lower_expression_operand(argument) else {
                return;
            };
            args.push(operand);
        }

        let repr = match initializer.target {
            ConstructorInitTarget::SelfType => format!("{class_name}::init#self"),
            ConstructorInitTarget::Super => format!("{class_name}::init#super"),
        };

        let func_operand = Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr,
            span: initializer.span,
            info: None,
        });

        let continue_block = self.new_block(initializer.span);
        let arg_modes = vec![ParamMode::Value; args.len()];
        let unwind_target = self.current_unwind_target();
        self.set_terminator(
            initializer.span,
            Terminator::Call {
                func: func_operand,
                args,
                arg_modes,
                destination: None,
                target: continue_block,
                unwind: unwind_target,
                dispatch: None,
            },
        );
        self.switch_to_block(continue_block);
    }
}

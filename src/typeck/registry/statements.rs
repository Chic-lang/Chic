use super::*;
use crate::frontend::ast::{
    Block, Expression, Statement, StatementKind, SwitchStatement, UsingResource,
};
use crate::frontend::diagnostics::{Label, Suggestion};
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::ClassLayoutKind;
use crate::mir::builder::FunctionSymbol;
use crate::mir::{FieldMetadata, PropertyMetadata};
use crate::primitives::PrimitiveKind;
use crate::syntax::expr::builders::NewInitializer;
use crate::syntax::expr::{ExprNode, NewExpr};
use std::collections::HashSet;

pub(crate) struct ConstructedTypeInfo {
    pub(crate) name: String,
    pub(crate) is_value_type: bool,
    pub(crate) is_record: bool,
}

pub(crate) struct ConstructorCandidate {
    pub(crate) visibility: Visibility,
    pub(crate) symbol: FunctionSymbol,
}

struct InitializerMember {
    owner: String,
    kind: InitializerMemberKind,
}

enum InitializerMemberKind {
    Field(FieldMetadata),
    Property(PropertyMetadata),
}

impl InitializerMember {
    fn owner(&self) -> &str {
        &self.owner
    }

    fn namespace(&self) -> Option<&str> {
        match &self.kind {
            InitializerMemberKind::Field(field) => field.namespace.as_deref(),
            InitializerMemberKind::Property(prop) => prop.namespace.as_deref(),
        }
    }

    fn visibility(&self) -> Visibility {
        match &self.kind {
            InitializerMemberKind::Field(field) => field.visibility,
            InitializerMemberKind::Property(prop) => prop.visibility,
        }
    }

    fn is_static(&self) -> bool {
        match &self.kind {
            InitializerMemberKind::Field(field) => field.is_static,
            InitializerMemberKind::Property(prop) => prop.is_static,
        }
    }
}

impl<'a> TypeChecker<'a> {
    fn switch_expression_exhaustive(arms: &[crate::syntax::expr::builders::SwitchArm]) -> bool {
        arms.iter().any(|arm| {
            matches!(
                arm.pattern.node,
                crate::syntax::pattern::PatternNode::Wildcard
            ) && arm.guards.is_empty()
        })
    }

    fn switch_statement_has_wildcard(stmt: &SwitchStatement) -> bool {
        stmt.sections.iter().any(|section| {
            section.labels.iter().any(|label| match label {
                crate::frontend::ast::SwitchLabel::Case(case) => {
                    case.guards.is_empty()
                        && case.pattern.ast.as_ref().map_or(false, |ast| {
                            matches!(ast.node, crate::syntax::pattern::PatternNode::Wildcard)
                        })
                }
                crate::frontend::ast::SwitchLabel::Default => true,
            })
        })
    }

    pub(crate) fn validate_block(
        &mut self,
        function_name: &str,
        block: &'a Block,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        for statement in &block.statements {
            self.validate_statement(function_name, statement, namespace, context_type);
        }
    }

    pub(crate) fn validate_statement(
        &mut self,
        function_name: &str,
        statement: &'a Statement,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        match &statement.kind {
            StatementKind::Block(inner) => {
                self.validate_block(function_name, inner, namespace, context_type);
            }
            StatementKind::Expression(expr) => {
                self.validate_expression(function_name, expr, namespace, context_type);
            }
            StatementKind::Return { expression } | StatementKind::Throw { expression } => {
                if let Some(expr) = expression {
                    self.validate_expression(function_name, expr, namespace, context_type);
                }
            }
            StatementKind::If(if_stmt) => {
                self.validate_expression(
                    function_name,
                    &if_stmt.condition,
                    namespace,
                    context_type,
                );
                self.validate_statement(
                    function_name,
                    if_stmt.then_branch.as_ref(),
                    namespace,
                    context_type,
                );
                if let Some(else_branch) = if_stmt.else_branch.as_ref() {
                    self.validate_statement(function_name, else_branch, namespace, context_type);
                }
            }
            StatementKind::While { condition, body } => {
                self.validate_expression(function_name, condition, namespace, context_type);
                self.validate_statement(function_name, body.as_ref(), namespace, context_type);
            }
            StatementKind::DoWhile { body, condition } => {
                self.validate_statement(function_name, body.as_ref(), namespace, context_type);
                self.validate_expression(function_name, condition, namespace, context_type);
            }
            StatementKind::For(for_stmt) => {
                if let Some(initializer) = &for_stmt.initializer {
                    match initializer {
                        crate::frontend::ast::ForInitializer::Declaration(decl) => {
                            self.validate_variable_declaration(
                                function_name,
                                decl,
                                namespace,
                                context_type,
                            );
                        }
                        crate::frontend::ast::ForInitializer::Const(const_stmt) => {
                            self.validate_const_statement(
                                function_name,
                                const_stmt,
                                namespace,
                                context_type,
                            );
                        }
                        crate::frontend::ast::ForInitializer::Expressions(exprs) => {
                            for expr in exprs {
                                self.validate_expression(
                                    function_name,
                                    expr,
                                    namespace,
                                    context_type,
                                );
                            }
                        }
                    }
                }
                if let Some(condition) = &for_stmt.condition {
                    self.validate_expression(function_name, condition, namespace, context_type);
                }
                for iterator in &for_stmt.iterator {
                    self.validate_expression(function_name, iterator, namespace, context_type);
                }
                self.validate_statement(
                    function_name,
                    for_stmt.body.as_ref(),
                    namespace,
                    context_type,
                );
            }
            StatementKind::Foreach(foreach_stmt) => {
                self.validate_expression(
                    function_name,
                    &foreach_stmt.expression,
                    namespace,
                    context_type,
                );
                self.validate_statement(
                    function_name,
                    foreach_stmt.body.as_ref(),
                    namespace,
                    context_type,
                );
            }
            StatementKind::Switch(switch_stmt) => {
                self.validate_expression(
                    function_name,
                    &switch_stmt.expression,
                    namespace,
                    context_type,
                );
                let mut has_default = false;
                for section in &switch_stmt.sections {
                    for label in &section.labels {
                        if let crate::frontend::ast::SwitchLabel::Case(case) = label {
                            self.validate_expression(
                                function_name,
                                &case.pattern.raw,
                                namespace,
                                context_type,
                            );
                            self.validate_case_pattern(&case.pattern, &case.guards);
                            for guard in &case.guards {
                                self.validate_expression(
                                    function_name,
                                    &guard.expression,
                                    namespace,
                                    context_type,
                                );
                            }
                        } else {
                            has_default = true;
                        }
                    }
                    for stmt in &section.statements {
                        self.validate_statement(function_name, stmt, namespace, context_type);
                    }
                }
                if !has_default && !Self::switch_statement_has_wildcard(switch_stmt) {
                    self.emit_error(
                        codes::PATTERN_NON_EXHAUSTIVE_SWITCH,
                        switch_stmt.expression.span.or(statement.span),
                        "switch statement is not exhaustive; add a `default` or `_` case",
                    );
                }
            }
            StatementKind::Try(try_stmt) => {
                self.validate_block(function_name, &try_stmt.body, namespace, context_type);
                for catch in &try_stmt.catches {
                    if let Some(filter) = &catch.filter {
                        self.validate_expression(function_name, filter, namespace, context_type);
                    }
                    self.validate_block(function_name, &catch.body, namespace, context_type);
                }
                if let Some(finally_block) = &try_stmt.finally {
                    self.validate_block(function_name, finally_block, namespace, context_type);
                }
            }
            StatementKind::Region { body, .. } => {
                self.validate_block(function_name, body, namespace, context_type);
            }
            StatementKind::Using(using_stmt) => {
                match &using_stmt.resource {
                    UsingResource::Expression(expr) => {
                        self.validate_expression(function_name, expr, namespace, context_type);
                    }
                    UsingResource::Declaration(decl) => {
                        self.validate_variable_declaration(
                            function_name,
                            decl,
                            namespace,
                            context_type,
                        );
                    }
                }
                if let Some(body) = &using_stmt.body {
                    self.validate_statement(function_name, body.as_ref(), namespace, context_type);
                }
            }
            StatementKind::Lock { expression, body } => {
                self.validate_expression(function_name, expression, namespace, context_type);
                self.validate_statement(function_name, body.as_ref(), namespace, context_type);
            }
            StatementKind::Checked { body } | StatementKind::Unchecked { body } => {
                self.validate_block(function_name, body, namespace, context_type);
            }
            StatementKind::Atomic { ordering, body } => {
                if let Some(ordering) = ordering {
                    let parsed = ordering
                        .node
                        .as_ref()
                        .and_then(|node| self.parse_memory_order_node(node));
                    if parsed.is_none() {
                        self.emit_memory_order_error(
                            ordering.span,
                            Some(ordering.text.as_str()),
                            "atomic block ordering",
                        );
                    }
                }
                self.validate_block(function_name, body, namespace, context_type);
            }
            StatementKind::YieldReturn { expression } => {
                self.validate_expression(function_name, expression, namespace, context_type);
            }
            StatementKind::YieldBreak | StatementKind::Break | StatementKind::Continue => {}
            StatementKind::Goto(goto_stmt) => {
                if let crate::frontend::ast::GotoTarget::Case { pattern, guards } =
                    &goto_stmt.target
                {
                    self.validate_expression(function_name, &pattern.raw, namespace, context_type);
                    self.validate_case_pattern(pattern, guards);
                    for guard in guards {
                        self.validate_expression(
                            function_name,
                            &guard.expression,
                            namespace,
                            context_type,
                        );
                    }
                }
            }
            StatementKind::Fixed(fixed_stmt) => {
                self.validate_variable_declaration(
                    function_name,
                    &fixed_stmt.declaration,
                    namespace,
                    context_type,
                );
                self.validate_statement(
                    function_name,
                    fixed_stmt.body.as_ref(),
                    namespace,
                    context_type,
                );
            }
            StatementKind::Unsafe { body } => {
                self.validate_statement(function_name, body.as_ref(), namespace, context_type);
            }
            StatementKind::Labeled {
                statement: inner, ..
            } => {
                self.validate_statement(function_name, inner.as_ref(), namespace, context_type);
            }
            StatementKind::LocalFunction(local) => {
                let symbol = self.allocate_local_function_symbol(function_name, &local.name);
                self.validate_local_function_decl(&symbol, local, namespace, context_type);
            }
            StatementKind::VariableDeclaration(decl) => {
                self.validate_variable_declaration(function_name, decl, namespace, context_type);
            }
            StatementKind::ConstDeclaration(const_stmt) => {
                self.validate_const_statement(function_name, const_stmt, namespace, context_type);
            }
            StatementKind::Empty => {}
        }
    }

    pub(crate) fn validate_expression(
        &mut self,
        function_name: &str,
        expression: &Expression,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        if let Some(node) = expression.node.as_ref() {
            self.validate_expr_node(
                function_name,
                node,
                expression.span,
                namespace,
                context_type,
            );
        }
    }

    pub(crate) fn validate_expr_node(
        &mut self,
        function_name: &str,
        node: &ExprNode,
        span: Option<Span>,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        match node {
            ExprNode::Unary { expr, .. } => {
                self.validate_expr_node(function_name, expr, span, namespace, context_type);
            }
            ExprNode::Ref { expr, .. } => {
                self.validate_expr_node(function_name, expr, span, namespace, context_type);
            }
            ExprNode::IndexFromEnd(index) => {
                self.validate_expr_node(function_name, &index.expr, span, namespace, context_type);
            }
            ExprNode::Range(range) => {
                if let Some(start) = &range.start {
                    self.validate_expr_node(
                        function_name,
                        start.expr.as_ref(),
                        start.span.or(span),
                        namespace,
                        context_type,
                    );
                }
                if let Some(end) = &range.end {
                    self.validate_expr_node(
                        function_name,
                        end.expr.as_ref(),
                        end.span.or(span),
                        namespace,
                        context_type,
                    );
                }
            }
            ExprNode::Binary { left, right, .. } => {
                self.validate_expr_node(function_name, left, span, namespace, context_type);
                self.validate_expr_node(function_name, right, span, namespace, context_type);
            }
            ExprNode::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                self.validate_expr_node(function_name, condition, span, namespace, context_type);
                self.validate_expr_node(function_name, then_branch, span, namespace, context_type);
                self.validate_expr_node(function_name, else_branch, span, namespace, context_type);
            }
            ExprNode::Switch(switch_expr) => {
                self.validate_expr_node(
                    function_name,
                    &switch_expr.value,
                    switch_expr.span.or(span),
                    namespace,
                    context_type,
                );
                for arm in &switch_expr.arms {
                    for guard in &arm.guards {
                        self.validate_expr_node(
                            function_name,
                            &guard.expr,
                            guard.span.or(switch_expr.span).or(span),
                            namespace,
                            context_type,
                        );
                    }
                    self.validate_expr_node(
                        function_name,
                        &arm.expression,
                        arm.span.or(switch_expr.span).or(span),
                        namespace,
                        context_type,
                    );
                }
                if !Self::switch_expression_exhaustive(&switch_expr.arms) {
                    self.emit_error(
                        codes::PATTERN_NON_EXHAUSTIVE_SWITCH,
                        switch_expr.span.or(span).or_else(|| {
                            switch_expr
                                .arms
                                .first()
                                .and_then(|arm| arm.span)
                                .or(switch_expr.switch_span)
                        }),
                        "switch expression is not exhaustive; add a `_` arm",
                    );
                }
            }
            ExprNode::Cast { expr, .. } | ExprNode::Parenthesized(expr) => {
                self.validate_expr_node(function_name, expr, span, namespace, context_type);
            }
            ExprNode::Assign { target, value, .. } => {
                self.validate_expr_node(function_name, target, span, namespace, context_type);
                self.validate_expr_node(function_name, value, span, namespace, context_type);
            }
            ExprNode::Member { base, .. } => {
                self.validate_expr_node(function_name, base, span, namespace, context_type);
            }
            ExprNode::Call { callee, args, .. } => {
                self.validate_expr_node(function_name, callee, span, namespace, context_type);
                self.check_compare_exchange_call(callee, args, span);
                for arg in args {
                    self.validate_expr_node(
                        function_name,
                        &arg.value,
                        arg.value_span.or(arg.span).or(span),
                        namespace,
                        context_type,
                    );
                }
                self.validate_call_expression(
                    function_name,
                    callee,
                    args,
                    span,
                    namespace,
                    context_type,
                );
            }
            ExprNode::New(new_expr) => {
                self.validate_new_expr(function_name, new_expr, span, namespace, context_type);
            }
            ExprNode::Index {
                base,
                indices,
                null_conditional: _,
            } => {
                self.validate_expr_node(function_name, base, span, namespace, context_type);
                for index in indices {
                    self.validate_expr_node(function_name, index, span, namespace, context_type);
                }
            }
            ExprNode::Await { expr }
            | ExprNode::TryPropagate { expr, .. }
            | ExprNode::Throw { expr: Some(expr) } => {
                self.validate_expr_node(function_name, expr, span, namespace, context_type);
            }
            ExprNode::Throw { expr: None } => {}
            ExprNode::SizeOf(operand) | ExprNode::AlignOf(operand) => {
                if let crate::syntax::expr::builders::SizeOfOperand::Value(value) = operand {
                    self.validate_expr_node(function_name, value, span, namespace, context_type);
                }
            }
            ExprNode::Lambda(lambda) => match &lambda.body {
                crate::syntax::expr::builders::LambdaBody::Expression(body_expr) => {
                    self.validate_expr_node(
                        function_name,
                        body_expr,
                        span,
                        namespace,
                        context_type,
                    );
                }
                crate::syntax::expr::builders::LambdaBody::Block(_) => {}
            },
            ExprNode::Tuple(elements) => {
                for element in elements {
                    self.validate_expr_node(function_name, element, span, namespace, context_type);
                }
            }
            ExprNode::ArrayLiteral(array) => {
                for element in &array.elements {
                    self.validate_expr_node(function_name, element, span, namespace, context_type);
                }
            }
            ExprNode::InterpolatedString(interpolated) => {
                for segment in &interpolated.segments {
                    if let crate::syntax::expr::builders::InterpolatedStringSegment::Expr(
                        expr_segment,
                    ) = segment
                    {
                        self.validate_expr_node(
                            function_name,
                            &expr_segment.expr,
                            expr_segment.span.or(span),
                            namespace,
                            context_type,
                        );
                    }
                }
            }
            ExprNode::InlineAsm(asm) => {
                for operand in &asm.operands {
                    match &operand.mode {
                        crate::syntax::expr::builders::InlineAsmOperandMode::In { expr }
                        | crate::syntax::expr::builders::InlineAsmOperandMode::Out {
                            expr, ..
                        }
                        | crate::syntax::expr::builders::InlineAsmOperandMode::Const { expr } => {
                            self.validate_expr_node(
                                function_name,
                                expr,
                                operand.span.or(span),
                                namespace,
                                context_type,
                            );
                        }
                        crate::syntax::expr::builders::InlineAsmOperandMode::InOut {
                            input,
                            output,
                            ..
                        } => {
                            self.validate_expr_node(
                                function_name,
                                input,
                                operand.span.or(span),
                                namespace,
                                context_type,
                            );
                            if let Some(output) = output {
                                self.validate_expr_node(
                                    function_name,
                                    output,
                                    operand.span.or(span),
                                    namespace,
                                    context_type,
                                );
                            }
                        }
                        crate::syntax::expr::builders::InlineAsmOperandMode::Sym { .. } => {}
                    }
                }
            }
            ExprNode::IsPattern { value, .. } => {
                self.validate_expr_node(function_name, value, span, namespace, context_type);
            }
            ExprNode::Default(_) | ExprNode::Quote(_) => {}
            ExprNode::Identifier(_) | ExprNode::Literal(_) | ExprNode::NameOf(_) => {}
        }
    }

    pub(crate) fn validate_new_expr(
        &mut self,
        function_name: &str,
        new_expr: &NewExpr,
        span: Option<Span>,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        for arg in &new_expr.args {
            self.validate_expr_node(
                function_name,
                &arg.value,
                arg.value_span.or(arg.span).or(span),
                namespace,
                context_type,
            );
        }
        if let Some(initializer) = &new_expr.initializer {
            match initializer {
                NewInitializer::Object { fields, .. } => {
                    for field in fields.iter() {
                        self.validate_expr_node(
                            function_name,
                            &field.value,
                            field.span.or(span),
                            namespace,
                            context_type,
                        );
                    }
                }
                NewInitializer::Collection { elements, .. } => {
                    for element in elements.iter() {
                        self.validate_expr_node(
                            function_name,
                            element,
                            span,
                            namespace,
                            context_type,
                        );
                    }
                }
            }
        }

        if let Some(info) =
            self.check_new_expression(function_name, new_expr, span, namespace, context_type)
        {
            self.validate_initializer_members(&info, new_expr, span, namespace, context_type);
        }
    }

    pub(crate) fn check_constructor_applicability(
        &mut self,
        type_name: &str,
        new_expr: &NewExpr,
        candidates: &[ConstructorCandidate],
        implicit_default_allowed: bool,
        namespace: Option<&str>,
        context_type: Option<&str>,
        span: Option<Span>,
        metadata_missing: bool,
    ) -> bool {
        if candidates.is_empty() {
            if new_expr.args.is_empty() && implicit_default_allowed {
                return true;
            }
            if metadata_missing {
                return true;
            }
            self.emit_error(
                codes::CONSTRUCTOR_NO_MATCH,
                new_expr.arguments_span.or(span).or(new_expr.span),
                format!(
                    "type `{type_name}` does not expose a constructor that accepts {} argument(s)",
                    new_expr.args.len()
                ),
            );
            return false;
        }

        let owner_namespace = self.namespace_of_type(type_name);
        let mut matches: Vec<(usize, FunctionSymbol)> = Vec::new();
        let mut mismatch_reason: Option<CallMismatch> = None;
        let mut inaccessible = 0usize;
        let call_span = new_expr.arguments_span.or(span).or(new_expr.span);
        for candidate in candidates {
            if !self.is_member_accessible(
                candidate.visibility,
                type_name,
                owner_namespace.as_deref(),
                namespace,
                context_type,
                None,
                false,
            ) {
                inaccessible += 1;
                continue;
            }
            match self.call_arguments_match(
                &format!("constructor `{type_name}`"),
                &candidate.symbol,
                &new_expr.args,
                call_span,
            ) {
                Ok(result) => matches.push((result.score, candidate.symbol.clone())),
                Err(reason) => {
                    if mismatch_reason.is_none() {
                        mismatch_reason = Some(reason);
                    }
                }
            }
        }

        if matches.is_empty() {
            if inaccessible > 0 {
                self.emit_error(
                    codes::CONSTRUCTOR_NO_MATCH,
                    call_span,
                    format!(
                        "no accessible constructor of `{type_name}` matches the provided arguments"
                    ),
                );
                return false;
            }
            if let Some(reason) = mismatch_reason {
                self.emit_error(codes::CONSTRUCTOR_NO_MATCH, reason.span, reason.message);
                return false;
            }
            self.emit_error(
                codes::CONSTRUCTOR_NO_MATCH,
                call_span,
                format!("no constructor of `{type_name}` matches the provided arguments"),
            );
            return false;
        }

        matches.sort_by(|a, b| b.0.cmp(&a.0));
        let best_score = matches[0].0;
        let winners: Vec<FunctionSymbol> = matches
            .iter()
            .take_while(|(score, _)| *score == best_score)
            .map(|(_, symbol)| symbol.clone())
            .collect();
        // This pass only validates arity/modes/names and cannot disambiguate based on argument
        // types; defer constructor overload resolution to MIR lowering.
        if winners.len() > 1 {
            if Self::constructor_overloads_indistinguishable_for_call(&winners, &new_expr.args) {
                let mut names: Vec<String> = winners
                    .iter()
                    .map(|symbol| symbol.qualified.clone())
                    .collect();
                names.sort();
                names.dedup();
                self.emit_error(
                    codes::CONSTRUCTOR_AMBIGUOUS,
                    call_span,
                    format!(
                        "constructor call for `{type_name}` is ambiguous; candidates: {}",
                        names.join(", ")
                    ),
                );
                return false;
            }
            return true;
        }

        true
    }

    fn constructor_overloads_indistinguishable_for_call(
        candidates: &[FunctionSymbol],
        args: &[crate::syntax::expr::builders::CallArgument],
    ) -> bool {
        let Some(first) = candidates.first() else {
            return false;
        };
        let expected = Self::expected_argument_types_for_constructor_call(first, args);
        candidates.iter().skip(1).all(|candidate| {
            Self::expected_argument_types_for_constructor_call(candidate, args) == expected
        })
    }

    fn expected_argument_types_for_constructor_call(
        symbol: &FunctionSymbol,
        args: &[crate::syntax::expr::builders::CallArgument],
    ) -> Vec<Option<crate::mir::Ty>> {
        let mut indices = Vec::with_capacity(args.len());
        let mut next_pos = 0usize;
        for arg in args {
            let idx = if let Some(name) = &arg.name {
                symbol
                    .params
                    .iter()
                    .position(|param| param.name == name.text)
                    .unwrap_or(usize::MAX)
            } else {
                let idx = next_pos;
                next_pos += 1;
                idx
            };
            indices.push(idx);
        }

        indices
            .into_iter()
            .map(|idx| symbol.signature.params.get(idx).cloned())
            .collect()
    }

    pub(crate) fn validate_variable_declaration(
        &mut self,
        function_name: &str,
        decl: &crate::frontend::ast::VariableDeclaration,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        for declarator in &decl.declarators {
            if let Some(initializer) = &declarator.initializer {
                self.validate_expression(function_name, initializer, namespace, context_type);
                self.check_numeric_literal_expression(initializer, decl.type_annotation.as_ref());
                self.validate_default_literal_usage(
                    initializer,
                    decl.type_annotation.as_ref(),
                    namespace,
                    context_type,
                );
                self.enforce_cross_function_inference_guard(
                    function_name,
                    decl,
                    declarator,
                    initializer,
                );
            }
        }
    }

    pub(crate) fn validate_const_statement(
        &mut self,
        function_name: &str,
        const_stmt: &crate::frontend::ast::ConstStatement,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        for declarator in &const_stmt.declaration.declarators {
            self.validate_expression(
                function_name,
                &declarator.initializer,
                namespace,
                context_type,
            );
            self.check_numeric_literal_expression(
                &declarator.initializer,
                Some(&const_stmt.declaration.ty),
            );
            self.validate_default_literal_usage(
                &declarator.initializer,
                Some(&const_stmt.declaration.ty),
                namespace,
                context_type,
            );
        }
    }

    fn enforce_cross_function_inference_guard(
        &mut self,
        function_name: &str,
        decl: &crate::frontend::ast::VariableDeclaration,
        declarator: &crate::frontend::ast::VariableDeclarator,
        initializer: &Expression,
    ) {
        if decl.type_annotation.is_some() {
            return;
        }
        let Some(node) = initializer.node.as_ref() else {
            return;
        };
        let Some(_target) = self.cross_function_call_target(node) else {
            return;
        };
        // Local type inference across function boundaries is intentionally relaxed for now to
        // keep helper stubs and async glue compilable without redundant annotations.
        let _ = (function_name, decl, declarator);
    }

    fn validate_default_literal_usage(
        &mut self,
        initializer: &Expression,
        annotated_ty: Option<&crate::frontend::ast::TypeExpr>,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        let Some(ExprNode::Default(default_expr)) = initializer.node.as_ref() else {
            return;
        };

        if default_expr.explicit_type.is_none() && annotated_ty.is_none() {
            let mut diag = diagnostics::error(
                codes::DEFAULT_LITERAL_INFER,
                "cannot infer type for `default`; add a type annotation or use `default(T)`",
                initializer.span,
            );
            if let Some(span) = initializer.span {
                diag.primary_label =
                    Some(Label::primary(span, "type for `default` is unknown here"));
            }
            diag.add_suggestion(Suggestion::new(
                "add an explicit type annotation or write `default(T)`",
                initializer.span,
                None,
            ));
            self.diagnostics.push(diag);
            return;
        }

        let mut target_ty = annotated_ty.cloned();
        if target_ty.is_none() {
            if let Some(type_text) = default_expr.explicit_type.as_ref() {
                if let Some(parsed) = parse_type_expression_text(type_text) {
                    target_ty = Some(parsed);
                } else {
                    let mut diag = diagnostics::error(
                        codes::DEFAULT_LITERAL_INFER,
                        format!("unable to parse type in `default({type_text})`"),
                        default_expr.type_span.or(initializer.span),
                    );
                    if let Some(span) = default_expr.type_span.or(initializer.span) {
                        diag.primary_label =
                            Some(Label::primary(span, "invalid type supplied here"));
                    }
                    self.diagnostics.push(diag);
                    return;
                }
            }
        }

        let Some(target_ty_expr) = target_ty else {
            return;
        };

        if target_ty_expr.is_nullable() || target_ty_expr.pointer_depth() > 0 {
            return;
        }

        let resolved = self.resolve_type_for_expr(&target_ty_expr, namespace, context_type);
        let canonical = match resolved {
            ImportResolution::Found(name) => name,
            _ => return,
        };

        let is_primitive_string = self
            .type_layouts
            .primitive_registry
            .descriptor_for_name(&canonical)
            .is_some_and(|desc| matches!(desc.kind, PrimitiveKind::String));
        let is_reference = self
            .type_layouts
            .class_layout_info(&canonical)
            .is_some_and(|info| matches!(info.kind, ClassLayoutKind::Class));

        if is_primitive_string || is_reference {
            let mut diag = diagnostics::error(
                codes::DEFAULT_LITERAL_NONNULL,
                format!("`default` is not allowed for non-nullable reference type `{canonical}`"),
                initializer.span,
            );
            if let Some(span) = initializer.span {
                diag.primary_label = Some(Label::primary(
                    span,
                    "provide a value or make the type nullable",
                ));
            }
            diag.add_suggestion(Suggestion::new(
                "mark the type nullable (e.g., `Type?`) or supply an explicit initializer",
                initializer.span,
                None,
            ));
            self.diagnostics.push(diag);
        }
    }

    pub(crate) fn cross_function_call_target(&self, node: &ExprNode) -> Option<String> {
        match node {
            ExprNode::Call { callee, .. } => {
                if self.call_target_is_value_member(callee) {
                    None
                } else {
                    Some(self.expr_display(callee))
                }
            }
            ExprNode::Await { expr }
            | ExprNode::TryPropagate { expr, .. }
            | ExprNode::Parenthesized(expr)
            | ExprNode::Ref { expr, .. }
            | ExprNode::Unary { expr, .. }
            | ExprNode::Cast { expr, .. } => self.cross_function_call_target(expr),
            ExprNode::IndexFromEnd(index) => self.cross_function_call_target(&index.expr),
            ExprNode::Range(range) => range
                .start
                .as_ref()
                .and_then(|start| self.cross_function_call_target(start.expr.as_ref()))
                .or_else(|| {
                    range
                        .end
                        .as_ref()
                        .and_then(|end| self.cross_function_call_target(end.expr.as_ref()))
                }),
            ExprNode::Member { base, .. } | ExprNode::Index { base, .. } => {
                self.cross_function_call_target(base)
            }
            ExprNode::Assign { value, .. } => self.cross_function_call_target(value),
            ExprNode::IsPattern { value, .. } => self.cross_function_call_target(value),
            ExprNode::Conditional {
                then_branch,
                else_branch,
                ..
            } => self
                .cross_function_call_target(then_branch)
                .or_else(|| self.cross_function_call_target(else_branch)),
            ExprNode::Binary { left, right, .. } => self
                .cross_function_call_target(left)
                .or_else(|| self.cross_function_call_target(right)),
            ExprNode::Switch(switch_expr) => self
                .cross_function_call_target(&switch_expr.value)
                .or_else(|| {
                    switch_expr.arms.iter().find_map(|arm| {
                        self.cross_function_call_target(&arm.expression)
                            .or_else(|| {
                                arm.guards
                                    .iter()
                                    .find_map(|guard| self.cross_function_call_target(&guard.expr))
                            })
                    })
                }),
            ExprNode::Tuple(elements) => elements
                .iter()
                .find_map(|element| self.cross_function_call_target(element)),
            ExprNode::Quote(quote) => self.cross_function_call_target(&quote.expression),
            ExprNode::Throw { expr } => expr
                .as_deref()
                .and_then(|inner| self.cross_function_call_target(inner)),
            ExprNode::Literal(_)
            | ExprNode::Identifier(_)
            | ExprNode::ArrayLiteral(_)
            | ExprNode::Lambda(_)
            | ExprNode::New(_)
            | ExprNode::SizeOf(_)
            | ExprNode::AlignOf(_)
            | ExprNode::NameOf(_)
            | ExprNode::InterpolatedString(_)
            | ExprNode::InlineAsm(_)
            | ExprNode::Default(_) => None,
        }
    }

    fn call_target_is_value_member(&self, callee: &ExprNode) -> bool {
        match callee {
            ExprNode::Member { .. } | ExprNode::Index { .. } => true,
            ExprNode::Parenthesized(expr) => self.call_target_is_value_member(expr),
            _ => false,
        }
    }

    pub(crate) fn validate_const_declaration(
        &mut self,
        namespace: Option<&str>,
        context_type: Option<&str>,
        declaration: &crate::frontend::ast::ConstDeclaration,
    ) {
        for declarator in &declaration.declarators {
            let owner_name = if let Some(context) = context_type {
                format!("{context}::{}", declarator.name)
            } else {
                crate::typeck::registry::qualify(namespace, &declarator.name)
            };
            self.validate_expression(
                &owner_name,
                &declarator.initializer,
                namespace,
                context_type,
            );
            self.check_numeric_literal_expression(&declarator.initializer, Some(&declaration.ty));
        }
    }

    fn validate_initializer_members(
        &mut self,
        constructed: &ConstructedTypeInfo,
        new_expr: &NewExpr,
        span: Option<Span>,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        let Some(initializer) = &new_expr.initializer else {
            return;
        };
        let NewInitializer::Object { fields, .. } = initializer else {
            return;
        };
        let mut assigned = HashSet::new();
        for field in fields {
            let name_span = field.name_span.or(field.span).or(span);
            if !assigned.insert(field.name.clone()) {
                self.emit_error(
                    codes::INITIALIZER_MEMBER_DUPLICATE,
                    name_span,
                    format!(
                        "member `{}` is assigned more than once in the object initializer",
                        field.name
                    ),
                );
            }
            let Some(member) = self.lookup_initializer_member(&constructed.name, &field.name)
            else {
                self.emit_error(
                    codes::INITIALIZER_MEMBER_UNKNOWN,
                    name_span,
                    format!(
                        "type `{}` has no field or property named `{}`",
                        constructed.name, field.name
                    ),
                );
                continue;
            };
            let member_namespace = member.namespace();
            if !self.is_member_accessible(
                member.visibility(),
                member.owner(),
                member_namespace,
                namespace,
                context_type,
                Some(constructed.name.as_str()),
                true,
            ) {
                self.emit_error(
                    codes::INITIALIZER_MEMBER_INACCESSIBLE,
                    name_span,
                    format!(
                        "cannot assign `{}` in object initializer because the member is not accessible",
                        field.name
                    ),
                );
                continue;
            }
            if member.is_static() {
                self.emit_error(
                    codes::INITIALIZER_MEMBER_STATIC,
                    name_span,
                    format!(
                        "member `{}` is static and cannot be assigned in an object initializer",
                        field.name
                    ),
                );
                continue;
            }

            match member.kind {
                InitializerMemberKind::Field(ref metadata) => {
                    if metadata.is_readonly && constructed.is_value_type && !constructed.is_record {
                        self.emit_error(
                            codes::INITIALIZER_MEMBER_IMMUTABLE,
                            name_span,
                            format!(
                                "field `{}` is readonly and cannot be assigned in an object initializer",
                                field.name
                            ),
                        );
                    }
                }
                InitializerMemberKind::Property(ref metadata) => {
                    if !(metadata.has_setter || metadata.has_init) {
                        self.emit_error(
                            codes::INITIALIZER_MEMBER_IMMUTABLE,
                            name_span,
                            format!(
                                "property `{}` does not expose a `set` or `init` accessor",
                                field.name
                            ),
                        );
                    }
                }
            }
        }

        if constructed.is_value_type {
            let required = self.required_members_for_type(&constructed.name);
            let mut missing: Vec<String> = required.difference(&assigned).cloned().collect();
            if !missing.is_empty() {
                missing.sort();
                self.emit_error(
                    codes::STRUCT_INITIALIZER_MISSING_REQUIRED,
                    new_expr.span.or(span),
                    format!(
                        "object initializer for `{}` must assign required member(s): {}",
                        constructed.name,
                        missing.join(", ")
                    ),
                );
            }
        }
    }

    fn lookup_initializer_member(
        &self,
        type_name: &str,
        member: &str,
    ) -> Option<InitializerMember> {
        for candidate in self.type_hierarchy(type_name) {
            if let Some(field) = self.symbol_index.field_metadata(&candidate, member) {
                return Some(InitializerMember {
                    owner: candidate,
                    kind: InitializerMemberKind::Field(field),
                });
            }
            if let Some(prop) = self.symbol_index.property_metadata(&candidate, member) {
                return Some(InitializerMember {
                    owner: candidate,
                    kind: InitializerMemberKind::Property(prop),
                });
            }
        }
        None
    }

    pub(super) fn namespace_of_type(&self, type_name: &str) -> Option<String> {
        type_name
            .rsplit_once("::")
            .map(|(namespace, _)| namespace.to_string())
    }
}

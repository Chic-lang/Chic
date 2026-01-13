use super::{CapturedLocal, LambdaLoweringBody};
use crate::frontend::ast::{
    Block as AstBlock, CatchClause, ConstStatement, Expression as AstExpression, ForInitializer,
    Statement, StatementKind, SwitchLabel, SwitchSection, UsingResource, VariableDeclaration,
};
use crate::mir::LocalId;
use crate::mir::builder::BodyBuilder;
use crate::syntax::expr::{
    ExprNode,
    builders::{InlineAsmOperandMode as AstInlineAsmOperandMode, NewInitializer},
    parse_expression,
};
use std::collections::HashSet;

use super::pattern::collect_pattern_node;

pub(crate) fn analyze_captures(
    builder: &BodyBuilder<'_>,
    lowering_body: &LambdaLoweringBody,
) -> Vec<CapturedLocal> {
    let mut collector = CaptureCollector::new(builder);
    match lowering_body {
        LambdaLoweringBody::Expression(expr) => collect_expr_node(&mut collector, expr),
        LambdaLoweringBody::Block(block) => collect_block(&mut collector, block),
    }
    collector.into_locals()
}

pub(crate) fn parse_expression_node(expr: &AstExpression) -> Option<ExprNode> {
    if let Some(node) = expr.node.clone() {
        return Some(node);
    }
    parse_expression(&expr.text).ok()
}

pub(super) struct CaptureCollector<'a> {
    builder: &'a BodyBuilder<'a>,
    seen: HashSet<LocalId>,
    ordered: Vec<LocalId>,
}

impl<'a> CaptureCollector<'a> {
    fn new(builder: &'a BodyBuilder<'a>) -> Self {
        Self {
            builder,
            seen: HashSet::new(),
            ordered: Vec::new(),
        }
    }

    fn record(&mut self, name: &str) {
        if let Some(local) = self.builder.lookup_name(name) {
            if self.seen.insert(local) {
                self.ordered.push(local);
            }
        }
    }

    fn into_locals(self) -> Vec<CapturedLocal> {
        self.ordered
            .into_iter()
            .filter_map(|local| {
                let decl = self.builder.locals.get(local.0)?;
                let name = decl.name.clone().unwrap_or_else(|| format!("_{}", local.0));
                Some(CapturedLocal {
                    name,
                    local,
                    ty: decl.ty.clone(),
                    is_nullable: decl.is_nullable,
                    is_mutable: decl.mutable,
                })
            })
            .collect()
    }
}

pub(super) fn collect_expr_node(collector: &mut CaptureCollector<'_>, expr: &ExprNode) {
    match expr {
        ExprNode::Identifier(name) => {
            collector.record(name);
        }
        ExprNode::Literal(_) | ExprNode::Default(_) => {}
        ExprNode::Unary { expr, .. } => collect_expr_node(collector, expr),
        ExprNode::IndexFromEnd(index) => collect_expr_node(collector, &index.expr),
        ExprNode::Range(range) => {
            if let Some(start) = range.start.as_ref() {
                collect_expr_node(collector, start.expr.as_ref());
            }
            if let Some(end) = range.end.as_ref() {
                collect_expr_node(collector, end.expr.as_ref());
            }
        }
        ExprNode::Ref { expr, .. } => collect_expr_node(collector, expr),
        ExprNode::Binary { left, right, .. } => {
            collect_expr_node(collector, left);
            collect_expr_node(collector, right);
        }
        ExprNode::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_node(collector, condition);
            collect_expr_node(collector, then_branch);
            collect_expr_node(collector, else_branch);
        }
        ExprNode::Call { callee, args, .. } => {
            collect_expr_node(collector, callee);
            for arg in args {
                collect_expr_node(collector, &arg.value);
            }
        }
        ExprNode::New(new_expr) => {
            for arg in &new_expr.args {
                collect_expr_node(collector, &arg.value);
            }
            if let Some(initializer) = &new_expr.initializer {
                match initializer {
                    NewInitializer::Object { fields, .. } => {
                        for field in fields {
                            collect_expr_node(collector, &field.value);
                        }
                    }
                    NewInitializer::Collection { elements, .. } => {
                        for element in elements {
                            collect_expr_node(collector, element);
                        }
                    }
                }
            }
        }
        ExprNode::Member { base, .. } => collect_expr_node(collector, base),
        ExprNode::Index {
            base,
            indices,
            null_conditional: _,
        } => {
            collect_expr_node(collector, base);
            for index in indices {
                collect_expr_node(collector, index);
            }
        }
        ExprNode::Tuple(elements) => {
            for element in elements {
                collect_expr_node(collector, element);
            }
        }
        ExprNode::ArrayLiteral(array) => {
            for element in &array.elements {
                collect_expr_node(collector, element);
            }
        }
        ExprNode::Parenthesized(inner) => collect_expr_node(collector, inner),
        ExprNode::Assign { target, value, .. } => {
            collect_expr_node(collector, target);
            collect_expr_node(collector, value);
        }
        ExprNode::IsPattern {
            value,
            pattern,
            guards,
        } => {
            collect_expr_node(collector, value);
            collect_pattern_node(collector, &pattern.node);
            for guard in guards {
                collect_expr_node(collector, &guard.expr);
            }
        }
        ExprNode::Switch(switch_expr) => {
            collect_expr_node(collector, &switch_expr.value);
            for arm in &switch_expr.arms {
                collect_pattern_node(collector, &arm.pattern.node);
                for guard in &arm.guards {
                    collect_expr_node(collector, &guard.expr);
                }
                collect_expr_node(collector, &arm.expression);
            }
        }
        ExprNode::Cast { expr, .. } => collect_expr_node(collector, expr),
        ExprNode::Await { expr } => collect_expr_node(collector, expr),
        ExprNode::TryPropagate { expr, .. } => collect_expr_node(collector, expr),
        ExprNode::Throw { expr } => {
            if let Some(inner) = expr {
                collect_expr_node(collector, inner);
            }
        }
        ExprNode::SizeOf(operand) => {
            if let crate::syntax::expr::SizeOfOperand::Value(inner) = operand {
                collect_expr_node(collector, inner);
            }
        }
        ExprNode::AlignOf(operand) => {
            if let crate::syntax::expr::SizeOfOperand::Value(inner) = operand {
                collect_expr_node(collector, inner);
            }
        }
        ExprNode::NameOf(_) => {}
        ExprNode::InterpolatedString(interpolated) => {
            for segment in &interpolated.segments {
                if let crate::syntax::expr::InterpolatedStringSegment::Expr(
                    crate::syntax::expr::InterpolatedExprSegment { expr, .. },
                ) = segment
                {
                    collect_expr_node(collector, expr);
                }
            }
        }
        ExprNode::Lambda(_) => {
            // Nested lambdas will perform their own capture analysis.
        }
        ExprNode::InlineAsm(asm) => {
            for operand in &asm.operands {
                match &operand.mode {
                    AstInlineAsmOperandMode::In { expr } => {
                        collect_expr_node(collector, expr);
                    }
                    AstInlineAsmOperandMode::Out { expr, .. } => {
                        collect_expr_node(collector, expr);
                    }
                    AstInlineAsmOperandMode::InOut { input, output, .. } => {
                        collect_expr_node(collector, input);
                        if let Some(output) = output {
                            collect_expr_node(collector, output);
                        }
                    }
                    AstInlineAsmOperandMode::Const { expr } => {
                        collect_expr_node(collector, expr);
                    }
                    AstInlineAsmOperandMode::Sym { .. } => {}
                }
            }
        }
        ExprNode::Quote(_) => {}
    }
}

fn collect_block(collector: &mut CaptureCollector<'_>, block: &AstBlock) {
    for statement in &block.statements {
        collect_statement(collector, statement);
    }
}

fn collect_statement(collector: &mut CaptureCollector<'_>, statement: &Statement) {
    match &statement.kind {
        StatementKind::Block(block) => collect_block(collector, block),
        StatementKind::Expression(expr) => collect_expression(collector, expr),
        StatementKind::VariableDeclaration(decl) => collect_variable_declaration(collector, decl),
        StatementKind::ConstDeclaration(decl) => collect_const_statement(collector, decl),
        StatementKind::LocalFunction(_) => {
            // Local functions perform their own capture analysis when lowered.
        }
        StatementKind::Return { expression } => {
            if let Some(expr) = expression {
                collect_expression(collector, expr);
            }
        }
        StatementKind::Throw { expression } => {
            if let Some(expr) = expression {
                collect_expression(collector, expr);
            }
        }
        StatementKind::If(if_stmt) => {
            collect_expression(collector, &if_stmt.condition);
            collect_statement(collector, if_stmt.then_branch.as_ref());
            if let Some(else_branch) = &if_stmt.else_branch {
                collect_statement(collector, else_branch.as_ref());
            }
        }
        StatementKind::While { condition, body } => {
            collect_expression(collector, condition);
            collect_statement(collector, body.as_ref());
        }
        StatementKind::DoWhile { body, condition } => {
            collect_statement(collector, body.as_ref());
            collect_expression(collector, condition);
        }
        StatementKind::For(for_stmt) => {
            if let Some(initializer) = &for_stmt.initializer {
                collect_for_initializer(collector, initializer);
            }
            if let Some(condition) = &for_stmt.condition {
                collect_expression(collector, condition);
            }
            for expr in &for_stmt.iterator {
                collect_expression(collector, expr);
            }
            collect_statement(collector, for_stmt.body.as_ref());
        }
        StatementKind::Region { body, .. } => collect_block(collector, body),
        StatementKind::Foreach(foreach) => {
            collect_expression(collector, &foreach.expression);
            collect_statement(collector, foreach.body.as_ref());
        }
        StatementKind::Switch(switch_stmt) => {
            collect_expression(collector, &switch_stmt.expression);
            for section in &switch_stmt.sections {
                collect_switch_section(collector, section);
            }
        }
        StatementKind::Try(try_stmt) => {
            collect_block(collector, &try_stmt.body);
            for clause in &try_stmt.catches {
                collect_catch_clause(collector, clause);
            }
            if let Some(finally) = &try_stmt.finally {
                collect_block(collector, finally);
            }
        }
        StatementKind::Using(using_stmt) => {
            match &using_stmt.resource {
                UsingResource::Expression(expr) => collect_expression(collector, expr),
                UsingResource::Declaration(decl) => collect_variable_declaration(collector, decl),
            }
            if let Some(body) = &using_stmt.body {
                collect_statement(collector, body.as_ref());
            }
        }
        StatementKind::Lock { expression, body } => {
            collect_expression(collector, expression);
            collect_statement(collector, body.as_ref());
        }
        StatementKind::Atomic { ordering, body } => {
            if let Some(ordering) = ordering {
                collect_expression(collector, ordering);
            }
            collect_block(collector, body);
        }
        StatementKind::Checked { body } | StatementKind::Unchecked { body } => {
            collect_block(collector, body);
        }
        StatementKind::YieldReturn { expression } => collect_expression(collector, expression),
        StatementKind::Fixed(fixed_stmt) => {
            collect_variable_declaration(collector, &fixed_stmt.declaration);
            collect_statement(collector, fixed_stmt.body.as_ref());
        }
        StatementKind::Unsafe { body }
        | StatementKind::Labeled {
            statement: body, ..
        } => {
            collect_statement(collector, body.as_ref());
        }
        StatementKind::Goto(goto_stmt) => match &goto_stmt.target {
            crate::frontend::ast::GotoTarget::Case { pattern, guards } => {
                collect_expression(collector, &pattern.raw);
                for guard in guards {
                    collect_expression(collector, &guard.expression);
                }
            }
            _ => {}
        },
        StatementKind::YieldBreak
        | StatementKind::Break
        | StatementKind::Continue
        | StatementKind::Empty => {}
    }
}

fn collect_expression(collector: &mut CaptureCollector<'_>, expr: &AstExpression) {
    if let Some(node) = parse_expression_node(expr) {
        collect_expr_node(collector, &node);
    }
}

fn collect_variable_declaration(collector: &mut CaptureCollector<'_>, decl: &VariableDeclaration) {
    for declarator in &decl.declarators {
        if let Some(initializer) = &declarator.initializer {
            collect_expression(collector, initializer);
        }
    }
}

fn collect_const_statement(collector: &mut CaptureCollector<'_>, decl: &ConstStatement) {
    for declarator in &decl.declaration.declarators {
        collect_expression(collector, &declarator.initializer);
    }
}

fn collect_for_initializer(collector: &mut CaptureCollector<'_>, init: &ForInitializer) {
    match init {
        ForInitializer::Declaration(decl) => collect_variable_declaration(collector, decl),
        ForInitializer::Const(decl) => collect_const_statement(collector, decl),
        ForInitializer::Expressions(exprs) => {
            for expr in exprs {
                collect_expression(collector, expr);
            }
        }
    }
}

fn collect_catch_clause(collector: &mut CaptureCollector<'_>, clause: &CatchClause) {
    if let Some(filter) = &clause.filter {
        collect_expression(collector, filter);
    }
    collect_block(collector, &clause.body);
}

fn collect_switch_section(collector: &mut CaptureCollector<'_>, section: &SwitchSection) {
    for label in &section.labels {
        match label {
            SwitchLabel::Case(case) => {
                collect_expression(collector, &case.pattern.raw);
                for guard in &case.guards {
                    collect_expression(collector, &guard.expression);
                }
            }
            SwitchLabel::Default => {}
        }
    }
    for statement in &section.statements {
        collect_statement(collector, statement);
    }
}

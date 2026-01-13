use crate::frontend::ast::StatementKind;
use crate::frontend::diagnostics::Span;
use crate::syntax::expr::ExprNode;

#[derive(Debug, Clone)]
pub struct ConstEvalError {
    pub message: String,
    pub span: Option<Span>,
}

impl ConstEvalError {
    pub fn with_span_if_missing(mut self, span: Option<Span>) -> Self {
        if self.span.is_none() {
            self.span = span;
        }
        self
    }
}

pub fn statement_kind_name(kind: &StatementKind) -> &'static str {
    match kind {
        StatementKind::While { .. } => "while",
        StatementKind::DoWhile { .. } => "do-while",
        StatementKind::For(_) => "for",
        StatementKind::Foreach(_) => "foreach",
        StatementKind::Switch(_) => "switch",
        StatementKind::Try(_) => "try",
        StatementKind::Using(_) => "using",
        StatementKind::Lock { .. } => "lock",
        StatementKind::Checked { .. } => "checked",
        StatementKind::Atomic { .. } => "atomic",
        StatementKind::Unchecked { .. } => "unchecked",
        StatementKind::YieldReturn { .. } => "yield return",
        StatementKind::YieldBreak => "yield break",
        StatementKind::Fixed(_) => "fixed",
        StatementKind::Unsafe { .. } => "unsafe",
        StatementKind::Break => "break",
        StatementKind::Continue => "continue",
        StatementKind::Goto(_) => "goto",
        StatementKind::Throw { .. } => "throw",
        StatementKind::Labeled { .. } => "labeled",
        StatementKind::ConstDeclaration(_) => "const",
        StatementKind::VariableDeclaration(_) => "variable",
        StatementKind::Expression(_) => "expression",
        StatementKind::Return { .. } => "return",
        StatementKind::Block(_) => "block",
        StatementKind::Empty => "empty",
        StatementKind::If(_) => "if",
        StatementKind::Region { .. } => "region",
        StatementKind::LocalFunction(_) => "local function",
    }
}

pub fn expr_path_segments(node: &ExprNode) -> Result<Vec<String>, String> {
    match node {
        ExprNode::Identifier(name) => Ok(vec![name.clone()]),
        ExprNode::Member { base, member, .. } => {
            let mut segments = expr_path_segments(base)?;
            segments.push(member.clone());
            Ok(segments)
        }
        ExprNode::Parenthesized(inner) => expr_path_segments(inner),
        _ => Err("expression is not a simple path".to_string()),
    }
}

pub fn simple_name(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

pub fn compare_numbers<T>(op: crate::mir::data::BinOp, lhs: T, rhs: T) -> bool
where
    T: PartialOrd + PartialEq,
{
    match op {
        crate::mir::data::BinOp::Lt => lhs < rhs,
        crate::mir::data::BinOp::Le => lhs <= rhs,
        crate::mir::data::BinOp::Gt => lhs > rhs,
        crate::mir::data::BinOp::Ge => lhs >= rhs,
        _ => unreachable!("unsupported comparison operator"),
    }
}

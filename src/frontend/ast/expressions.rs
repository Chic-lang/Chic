//! AST expression and statement nodes.
use super::items::{Attribute, ConstStatement, FunctionDecl};
use super::patterns::{CasePattern, PatternGuard};
use super::types::TypeExpr;
use crate::frontend::diagnostics::Span;
use crate::syntax::expr::{ExprNode, NewExpr};

/// Local expression captured as source text.
#[derive(Debug, Clone)]
pub struct Expression {
    pub text: String,
    pub span: Option<Span>,
    pub node: Option<ExprNode>,
}

impl Expression {
    pub fn new(text: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            text: text.into(),
            span,
            node: None,
        }
    }

    pub fn with_node(text: impl Into<String>, span: Option<Span>, node: ExprNode) -> Self {
        Self {
            text: text.into(),
            span,
            node: Some(node),
        }
    }

    #[must_use]
    pub fn as_new_expr(&self) -> Option<&NewExpr> {
        match self.node.as_ref()? {
            ExprNode::New(new_expr) => Some(new_expr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Option<Span>,
}

/// Statement node with span metadata.
#[derive(Debug, Clone)]
pub struct Statement {
    pub span: Option<Span>,
    pub kind: StatementKind,
    pub attributes: Option<Vec<Attribute>>,
}

impl Statement {
    #[must_use]
    pub fn new(span: Option<Span>, kind: StatementKind) -> Self {
        Self {
            span,
            kind,
            attributes: None,
        }
    }

    #[must_use]
    pub fn with_attributes(mut self, attributes: Vec<Attribute>) -> Self {
        if attributes.is_empty() {
            self.attributes = None;
        } else {
            self.attributes = Some(attributes);
        }
        self
    }
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Block(Block),
    Empty,
    VariableDeclaration(VariableDeclaration),
    ConstDeclaration(ConstStatement),
    LocalFunction(FunctionDecl),
    Expression(Expression),
    Return {
        expression: Option<Expression>,
    },
    Break,
    Continue,
    Goto(GotoStatement),
    Throw {
        expression: Option<Expression>,
    },
    If(IfStatement),
    While {
        condition: Expression,
        body: Box<Statement>,
    },
    DoWhile {
        body: Box<Statement>,
        condition: Expression,
    },
    For(ForStatement),
    Foreach(ForeachStatement),
    Switch(SwitchStatement),
    Try(TryStatement),
    Region {
        name: String,
        body: Block,
    },
    Using(UsingStatement),
    Lock {
        expression: Expression,
        body: Box<Statement>,
    },
    Checked {
        body: Block,
    },
    Atomic {
        ordering: Option<Expression>,
        body: Block,
    },
    Unchecked {
        body: Block,
    },
    YieldReturn {
        expression: Expression,
    },
    YieldBreak,
    Fixed(FixedStatement),
    Unsafe {
        body: Box<Statement>,
    },
    Labeled {
        label: String,
        statement: Box<Statement>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct VariableDeclaration {
    pub modifier: VariableModifier,
    pub type_annotation: Option<TypeExpr>,
    pub declarators: Vec<VariableDeclarator>,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, Default)]
pub enum VariableModifier {
    #[default]
    Let,
    Var,
}

#[derive(Debug, Clone)]
pub struct VariableDeclarator {
    pub name: String,
    pub initializer: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_branch: Box<Statement>,
    pub else_branch: Option<Box<Statement>>,
}

#[derive(Debug, Clone)]
pub struct ForStatement {
    pub initializer: Option<ForInitializer>,
    pub condition: Option<Expression>,
    pub iterator: Vec<Expression>,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone)]
pub enum ForInitializer {
    Declaration(VariableDeclaration),
    Const(ConstStatement),
    Expressions(Vec<Expression>),
}

#[derive(Debug, Clone)]
pub struct ForeachStatement {
    /// Raw binding text between `foreach (` and `in`.
    pub binding: String,
    pub binding_span: Option<Span>,
    pub expression: Expression,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone)]
pub struct SwitchStatement {
    pub expression: Expression,
    pub sections: Vec<SwitchSection>,
}

#[derive(Debug, Clone)]
pub struct SwitchSection {
    pub labels: Vec<SwitchLabel>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum SwitchLabel {
    Case(SwitchCaseLabel),
    Default,
}

#[derive(Debug, Clone)]
pub struct SwitchCaseLabel {
    pub pattern: CasePattern,
    pub guards: Vec<PatternGuard>,
}

#[derive(Debug, Clone)]
pub struct TryStatement {
    pub body: Block,
    pub catches: Vec<CatchClause>,
    pub finally: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub type_annotation: Option<TypeExpr>,
    pub identifier: Option<String>,
    pub filter: Option<Expression>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct UsingStatement {
    pub resource: UsingResource,
    pub body: Option<Box<Statement>>,
}

#[derive(Debug, Clone)]
pub enum UsingResource {
    Expression(Expression),
    Declaration(VariableDeclaration),
}

#[derive(Debug, Clone)]
pub struct GotoStatement {
    pub target: GotoTarget,
}

#[derive(Debug, Clone)]
pub enum GotoTarget {
    Label(String),
    Case {
        pattern: CasePattern,
        guards: Vec<PatternGuard>,
    },
    Default,
}

#[derive(Debug, Clone)]
pub struct FixedStatement {
    pub declaration: VariableDeclaration,
    pub body: Box<Statement>,
}

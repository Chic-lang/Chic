//! Expression builders, shared constructors, and AST definitions used by the
//! expression parser and formatter.

use crate::frontend::ast::Expression;
use crate::frontend::diagnostics::Span;
use crate::mir::{BinOp, ConstValue, UnOp};
use crate::syntax::numeric::NumericLiteralMetadata;
use crate::syntax::pattern::PatternAst;

/// Switch expression arm containing a pattern, optional guards, and the arm body.
#[derive(Clone, Debug)]
pub struct SwitchArm {
    pub pattern: PatternAst,
    pub guards: Vec<PatternGuardExpr>,
    pub expression: ExprNode,
    pub span: Option<Span>,
    pub arrow_span: Option<Span>,
}

/// Switch expression representation mirroring C#-style `expr switch { .. }`.
#[derive(Clone, Debug)]
pub struct SwitchExpr {
    pub value: Box<ExprNode>,
    pub arms: Vec<SwitchArm>,
    pub span: Option<Span>,
    pub switch_span: Option<Span>,
    pub braces_span: Option<Span>,
}

/// Name specified for a call argument slot.
#[derive(Clone, Debug)]
pub struct CallArgumentName {
    pub text: String,
    pub span: Option<Span>,
}

impl CallArgumentName {
    #[must_use]
    pub fn new(text: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}

/// Modifier attached to a call argument (e.g. `ref`, `in`, `out`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallArgumentModifier {
    In,
    Ref,
    Out,
}

impl CallArgumentModifier {
    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Ref => "ref",
            Self::Out => "out",
        }
    }
}

/// Parsed argument supplied to a call expression.
#[derive(Clone, Debug)]
pub struct CallArgument {
    pub name: Option<CallArgumentName>,
    pub value: ExprNode,
    pub span: Option<Span>,
    pub value_span: Option<Span>,
    pub modifier: Option<CallArgumentModifier>,
    pub modifier_span: Option<Span>,
    pub inline_binding: Option<InlineBinding>,
}

impl CallArgument {
    #[must_use]
    pub fn positional(value: ExprNode, span: Option<Span>, value_span: Option<Span>) -> Self {
        Self {
            name: None,
            value,
            span,
            value_span,
            modifier: None,
            modifier_span: None,
            inline_binding: None,
        }
    }

    #[must_use]
    pub fn named(
        name: CallArgumentName,
        value: ExprNode,
        span: Option<Span>,
        value_span: Option<Span>,
    ) -> Self {
        Self {
            name: Some(name),
            value,
            span,
            value_span,
            modifier: None,
            modifier_span: None,
            inline_binding: None,
        }
    }

    #[must_use]
    pub fn with_modifier(
        mut self,
        modifier: CallArgumentModifier,
        modifier_span: Option<Span>,
    ) -> Self {
        self.modifier = Some(modifier);
        self.modifier_span = modifier_span;
        self
    }

    #[must_use]
    pub fn with_inline_binding(mut self, binding: InlineBinding) -> Self {
        self.inline_binding = Some(binding);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineBindingKind {
    Var,
    Typed {
        type_name: String,
        type_span: Option<Span>,
    },
}

#[derive(Clone, Debug)]
pub struct InlineBinding {
    pub kind: InlineBindingKind,
    pub name: String,
    pub keyword_span: Option<Span>,
    pub name_span: Option<Span>,
    pub initializer: Option<ExprNode>,
    pub initializer_span: Option<Span>,
}

/// Named inline assembly operand reference within a template string.
#[derive(Clone, Debug)]
pub enum InlineAsmTemplateOperandRef {
    Position(usize),
    Named(String),
}

/// Template fragment for inline assembly expressions.
#[derive(Clone, Debug)]
pub enum InlineAsmTemplatePiece {
    Literal(String),
    Placeholder {
        operand: InlineAsmTemplateOperandRef,
        modifier: Option<String>,
        span: Option<Span>,
    },
}

/// Inline assembly template made up of literal and placeholder pieces.
#[derive(Clone, Debug)]
pub struct InlineAsmTemplate {
    pub pieces: Vec<InlineAsmTemplatePiece>,
    pub span: Option<Span>,
}

/// Supported inline assembly register classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineAsmRegisterClass {
    Reg,
    Reg8,
    Reg16,
    Reg32,
    Reg64,
    Xmm,
    Ymm,
    Zmm,
    Vreg,
    Kreg,
}

/// Register selection for inline assembly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineAsmRegister {
    Class(InlineAsmRegisterClass),
    Explicit(String),
}

/// Operand kinds supported by inline assembly.
#[allow(clippy::large_enum_variant)] // AST fidelity trumps layout; variants mirror the surface syntax.
#[derive(Clone, Debug)]
pub enum InlineAsmOperandMode {
    In {
        expr: ExprNode,
    },
    Out {
        expr: ExprNode,
        late: bool,
    },
    InOut {
        input: ExprNode,
        output: Option<ExprNode>,
        late: bool,
    },
    Const {
        expr: ExprNode,
    },
    Sym {
        path: String,
    },
}

/// Operand supplied to an inline assembly expression.
#[derive(Clone, Debug)]
pub struct InlineAsmOperand {
    pub name: Option<String>,
    pub reg: InlineAsmRegister,
    pub mode: InlineAsmOperandMode,
    pub span: Option<Span>,
}

/// Inline assembly options mirroring Rust's `asm!` surface.
#[allow(clippy::struct_excessive_bools)] // Options map 1:1 with the `asm!` surface; keep them explicit.
#[derive(Clone, Debug, Default)]
pub struct InlineAsmOptions {
    pub volatile: bool,
    pub alignstack: bool,
    pub intel_syntax: bool,
    pub nomem: bool,
    pub nostack: bool,
    pub preserves_flags: bool,
    pub pure: bool,
    pub readonly: bool,
    pub noreturn: bool,
}

/// Inline assembly expression representation.
#[derive(Clone, Debug)]
pub struct InlineAsmExpr {
    pub template: InlineAsmTemplate,
    pub operands: Vec<InlineAsmOperand>,
    pub clobbers: Vec<InlineAsmRegister>,
    pub options: InlineAsmOptions,
    pub span: Option<Span>,
}

/// Endpoint within a range expression.
#[derive(Clone, Debug)]
pub struct RangeEndpoint {
    pub expr: Box<ExprNode>,
    pub from_end: bool,
    pub span: Option<Span>,
}

impl RangeEndpoint {
    #[must_use]
    pub fn new(expr: ExprNode, from_end: bool, span: Option<Span>) -> Self {
        Self {
            expr: Box::new(expr),
            from_end,
            span,
        }
    }
}

/// Parsed representation of a range expression.
#[derive(Clone, Debug)]
pub struct RangeExpr {
    pub start: Option<Box<RangeEndpoint>>,
    pub end: Option<Box<RangeEndpoint>>,
    pub inclusive: bool,
    pub span: Option<Span>,
}

/// Index-from-end unary expression (`^value`).
#[derive(Clone, Debug)]
pub struct IndexFromEndExpr {
    pub expr: Box<ExprNode>,
    pub span: Option<Span>,
}

/// Field assignment within an object initializer.
#[derive(Clone, Debug)]
pub struct ObjectInitializerField {
    pub name: String,
    pub name_span: Option<Span>,
    pub value: ExprNode,
    pub value_span: Option<Span>,
    pub span: Option<Span>,
}

/// Supported initializer flavours for `new` expressions.
#[derive(Clone, Debug)]
pub enum NewInitializer {
    Object {
        fields: Vec<ObjectInitializerField>,
        span: Option<Span>,
    },
    Collection {
        elements: Vec<ExprNode>,
        span: Option<Span>,
    },
}

/// Parsed representation of a `new` expression.
#[derive(Clone, Debug)]
pub struct NewExpr {
    pub type_name: String,
    pub type_span: Option<Span>,
    pub keyword_span: Option<Span>,
    /// Optional array length expressions supplied via `new T[length]`.
    pub array_lengths: Option<Vec<ExprNode>>,
    pub args: Vec<CallArgument>,
    pub arguments_span: Option<Span>,
    pub initializer: Option<NewInitializer>,
    pub span: Option<Span>,
}

/// Lambda expression representation.
#[derive(Clone, Debug)]
pub struct LambdaExpr {
    pub params: Vec<LambdaParam>,
    pub captures: Vec<String>,
    pub body: LambdaBody,
    pub is_async: bool,
    pub span: Option<Span>,
}

/// Lambda parameter description.
#[derive(Clone, Debug)]
pub struct LambdaParam {
    pub modifier: Option<LambdaParamModifier>,
    pub ty: Option<String>,
    pub name: String,
    pub span: Option<Span>,
    pub default: Option<Expression>,
}

/// Optional modifier for a lambda parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LambdaParamModifier {
    In,
    Ref,
    Out,
}

impl LambdaParamModifier {
    #[must_use]
    pub fn keyword(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Ref => "ref",
            Self::Out => "out",
        }
    }
}

/// Lambda body can be either an expression or a block.
#[derive(Clone, Debug)]
pub enum LambdaBody {
    Expression(Box<ExprNode>),
    Block(LambdaBlock),
}

/// Block body captured from source text.
#[derive(Clone, Debug)]
pub struct LambdaBlock {
    pub text: String,
    pub span: Option<Span>,
}

/// Parsed interpolated string with structured segments.
#[derive(Clone, Debug)]
pub struct InterpolatedStringExpr {
    pub segments: Vec<InterpolatedStringSegment>,
    pub span: Option<Span>,
}

/// Segment within an interpolated string.
#[allow(clippy::large_enum_variant)] // Maintain the natural representation of interpolated string pieces.
#[derive(Clone, Debug)]
pub enum InterpolatedStringSegment {
    Text(String),
    Expr(InterpolatedExprSegment),
}

/// Captured `quote(expr)` expression plus interpolation metadata.
#[derive(Clone, Debug)]
pub struct QuoteLiteral {
    pub expression: Box<ExprNode>,
    pub source: String,
    pub sanitized: String,
    pub content_span: Option<QuoteSourceSpan>,
    pub interpolations: Vec<QuoteInterpolation>,
    pub hygiene_anchor: usize,
}

/// Interpolation slot captured within a quoted expression.
#[derive(Clone, Debug)]
pub struct QuoteInterpolation {
    pub placeholder: String,
    pub expression: ExprNode,
    pub expression_text: String,
    pub span: Option<QuoteSourceSpan>,
}

/// Span tracked relative to the containing expression text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuoteSourceSpan {
    pub start: usize,
    pub end: usize,
}

impl QuoteSourceSpan {
    #[must_use]
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn to_span(self) -> Span {
        Span::new(self.start, self.end)
    }

    #[must_use]
    pub fn shift(self, offset: usize) -> Span {
        Span::new(self.start + offset, self.end + offset)
    }
}

/// Metadata for an interpolated expression segment.
#[derive(Clone, Debug)]
pub struct InterpolatedExprSegment {
    pub expr: ExprNode,
    pub expr_text: String,
    pub alignment: Option<i32>,
    pub format: Option<String>,
    pub span: Option<Span>,
}

/// Differentiates the syntax used to express a cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastSyntax {
    /// Traditional parenthesised cast: `(T)expr`.
    Paren,
    /// Rust-style explicit cast: `expr as T`.
    As,
}

/// Operand accepted by the `sizeof`/`alignof` operators.
#[derive(Clone, Debug)]
pub enum SizeOfOperand {
    Type(String),
    Value(Box<ExprNode>),
}

/// Operand captured by the `nameof` operator.
#[derive(Clone, Debug)]
pub struct NameOfOperand {
    pub segments: Vec<String>,
    pub text: String,
    pub span: Option<Span>,
}

impl NameOfOperand {
    #[must_use]
    pub fn display(&self) -> &str {
        self.text.as_str()
    }
}

/// Assignment operators recognised by the expression parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    NullCoalesceAssign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}

/// Literal value captured by the AST.
#[derive(Clone, Debug)]
pub struct LiteralConst {
    pub value: ConstValue,
    pub numeric: Option<NumericLiteralMetadata>,
}

impl LiteralConst {
    #[must_use]
    pub fn new(value: ConstValue, numeric: Option<NumericLiteralMetadata>) -> Self {
        Self { value, numeric }
    }

    #[must_use]
    pub fn without_numeric(value: ConstValue) -> Self {
        Self {
            value,
            numeric: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArrayLiteralExpr {
    pub explicit_type: Option<String>,
    pub explicit_type_span: Option<Span>,
    pub elements: Vec<ExprNode>,
    pub element_spans: Vec<Option<Span>>,
    pub open_span: Option<Span>,
    pub close_span: Option<Span>,
    pub trailing_comma: bool,
    pub span: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct DefaultExpr {
    pub explicit_type: Option<String>,
    pub keyword_span: Option<Span>,
    pub type_span: Option<Span>,
}

/// Parsed expression tree used during MIR lowering.
#[derive(Clone, Debug)]
pub enum ExprNode {
    Literal(LiteralConst),
    Identifier(String),
    Unary {
        op: UnOp,
        expr: Box<ExprNode>,
        postfix: bool,
    },
    IndexFromEnd(IndexFromEndExpr),
    Range(RangeExpr),
    Binary {
        op: BinOp,
        left: Box<ExprNode>,
        right: Box<ExprNode>,
    },
    Conditional {
        condition: Box<ExprNode>,
        then_branch: Box<ExprNode>,
        else_branch: Box<ExprNode>,
    },
    Cast {
        target: String,
        expr: Box<ExprNode>,
        syntax: CastSyntax,
    },
    IsPattern {
        value: Box<ExprNode>,
        pattern: PatternAst,
        guards: Vec<PatternGuardExpr>,
    },
    Lambda(LambdaExpr),
    Parenthesized(Box<ExprNode>),
    Tuple(Vec<ExprNode>),
    Assign {
        target: Box<ExprNode>,
        op: AssignOp,
        value: Box<ExprNode>,
    },
    Member {
        base: Box<ExprNode>,
        member: String,
        null_conditional: bool,
    },
    Call {
        callee: Box<ExprNode>,
        args: Vec<CallArgument>,
        generics: Option<Vec<String>>,
    },
    Ref {
        expr: Box<ExprNode>,
        readonly: bool,
    },
    Index {
        base: Box<ExprNode>,
        indices: Vec<ExprNode>,
        null_conditional: bool,
    },
    Await {
        expr: Box<ExprNode>,
    },
    TryPropagate {
        expr: Box<ExprNode>,
        question_span: Option<Span>,
    },
    Throw {
        expr: Option<Box<ExprNode>>,
    },
    New(NewExpr),
    ArrayLiteral(ArrayLiteralExpr),
    Switch(SwitchExpr),
    SizeOf(SizeOfOperand),
    AlignOf(SizeOfOperand),
    NameOf(NameOfOperand),
    InterpolatedString(InterpolatedStringExpr),
    Quote(Box<QuoteLiteral>),
    InlineAsm(InlineAsmExpr),
    Default(DefaultExpr),
}

impl ExprNode {
    #[must_use]
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

#[derive(Clone, Debug)]
pub struct PatternGuardExpr {
    pub expr: ExprNode,
    pub span: Option<Span>,
    pub depth: usize,
    pub keyword_span: Option<Span>,
}

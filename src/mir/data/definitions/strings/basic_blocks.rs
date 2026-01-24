use std::fmt;

use crate::decimal::Decimal128;
use crate::frontend::diagnostics::Span;
use crate::mir::layout::{MmioAccess, MmioEndianness};
use crate::mir::state::{AsyncStateMachine, ExceptionRegion, GeneratorStateMachine};
use crate::mmio::AddressSpaceId;
use half::f16;

use super::StrId;
use super::module::StaticId;
use super::types::{FnTy, Ty};
use super::utils::{new_mir_body, new_place};

/// Mid-level representation of a function body.
/// Body/basic block structures live alongside their string forms here to keep the MIR
/// text definitions consistent while the split modules are stabilised.
#[derive(Debug, Clone)]
pub struct MirBody {
    pub arg_count: usize,
    pub locals: Vec<LocalDecl>,
    pub blocks: Vec<BasicBlock>,
    pub span: Option<Span>,
    pub async_machine: Option<AsyncStateMachine>,
    pub generator: Option<GeneratorStateMachine>,
    pub exception_regions: Vec<ExceptionRegion>,
    pub vectorize_decimal: bool,
    pub effects: Vec<Ty>,
    pub stream_metadata: Vec<StreamMetadata>,
    pub debug_notes: Vec<DebugNote>,
}

impl MirBody {
    #[must_use]
    pub fn new(arg_count: usize, span: Option<Span>) -> Self {
        new_mir_body(arg_count, span)
    }

    #[must_use]
    pub fn entry(&self) -> BlockId {
        BlockId(0)
    }

    #[must_use]
    pub fn local(&self, id: LocalId) -> Option<&LocalDecl> {
        self.locals.get(id.0)
    }

    #[must_use]
    pub fn local_mut(&mut self, id: LocalId) -> Option<&mut LocalDecl> {
        self.locals.get_mut(id.0)
    }
}

/// Metadata recorded for each accelerator stream present in a MIR body.
#[derive(Debug, Clone)]
pub struct StreamMetadata {
    pub local: LocalId,
    pub mem_space: Option<Ty>,
    pub stream_id: u32,
}

/// Debug-only notes attached to a MIR body.
#[derive(Debug, Clone)]
pub struct DebugNote {
    pub message: String,
    pub span: Option<Span>,
}

/// Declaration for a local slot in MIR.
#[derive(Debug, Clone)]
pub struct LocalDecl {
    pub name: Option<String>,
    pub ty: Ty,
    pub mutable: bool,
    pub span: Option<Span>,
    pub is_pinned: bool,
    pub is_nullable: bool,
    pub kind: LocalKind,
    pub param_mode: Option<ParamMode>,
    pub aliasing: AliasContract,
}

impl LocalDecl {
    #[must_use]
    pub fn new(
        name: Option<String>,
        ty: Ty,
        mutable: bool,
        span: Option<Span>,
        kind: LocalKind,
    ) -> Self {
        let is_nullable = matches!(ty, Ty::Nullable(_));
        Self {
            name,
            ty,
            mutable,
            span,
            is_pinned: false,
            is_nullable,
            kind,
            param_mode: None,
            aliasing: AliasContract::default(),
        }
    }

    #[must_use]
    pub fn with_param_mode(mut self, mode: ParamMode) -> Self {
        self.param_mode = Some(mode);
        self
    }

    #[must_use]
    pub fn with_alias_contract(mut self, contract: AliasContract) -> Self {
        self.aliasing = contract;
        self
    }
}

/// The role played by a local slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    Return,
    Arg(usize),
    Local,
    Temp,
}

/// Parameter binding semantics for a function argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    Value,
    In,
    Ref,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct AliasContract {
    pub noalias: bool,
    pub nocapture: bool,
    pub readonly: bool,
    pub writeonly: bool,
    pub restrict: bool,
    pub expose_address: bool,
    pub alignment: Option<u32>,
}

/// Identifier for a MIR local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

impl fmt::Display for LocalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_{}", self.0)
    }
}

/// Identifier for a MIR basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// Identifier for a borrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorrowId(pub usize);

/// Identifier for a region variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionVar(pub usize);

/// A MIR basic block: linear list of statements ending in a terminator.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub statements: Vec<Statement>,
    pub terminator: Option<Terminator>,
    pub span: Option<Span>,
}

impl BasicBlock {
    #[must_use]
    pub fn new(id: BlockId, span: Option<Span>) -> Self {
        Self {
            id,
            statements: Vec::new(),
            terminator: None,
            span,
        }
    }
}

/// Statement inside a MIR basic block.
/// Statements and terminators are defined here with the block model to keep the stringified
/// MIR definitions coherent during the ongoing module split.
#[derive(Debug, Clone)]
pub struct Statement {
    pub span: Option<Span>,
    pub kind: StatementKind,
}

/// Ownership-aware categories for MIR operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueCategory {
    Move,
    Copy,
    Borrow(BorrowKind),
    Pending,
}

/// Statement variants recognised by MIR.
#[derive(Debug, Clone)]
pub enum StatementKind {
    Assign {
        place: Place,
        value: Rvalue,
    },
    StorageLive(LocalId),
    StorageDead(LocalId),
    MarkFallibleHandled {
        local: LocalId,
    },
    Deinit(Place),
    Drop {
        place: Place,
        target: BlockId,
        unwind: Option<BlockId>,
    },
    Borrow {
        borrow_id: BorrowId,
        kind: BorrowKind,
        place: Place,
        region: RegionVar,
    },
    Retag {
        place: Place,
    },
    DeferDrop {
        place: Place,
    },
    DefaultInit {
        place: Place,
    },
    ZeroInit {
        place: Place,
    },
    ZeroInitRaw {
        pointer: Operand,
        length: Operand,
    },
    AtomicStore {
        target: Place,
        value: Operand,
        order: AtomicOrdering,
    },
    AtomicFence {
        order: AtomicOrdering,
        scope: AtomicFenceScope,
    },
    EnterUnsafe,
    ExitUnsafe,
    MmioStore {
        target: MmioOperand,
        value: Operand,
    },
    Assert {
        cond: Operand,
        expected: bool,
        message: String,
        target: BlockId,
        cleanup: Option<BlockId>,
    },
    EnqueueKernel {
        stream: Place,
        kernel: Operand,
        args: Vec<Operand>,
        completion: Option<Place>,
    },
    EnqueueCopy {
        stream: Place,
        dst: Place,
        src: Place,
        bytes: Operand,
        kind: AcceleratorCopyKind,
        completion: Option<Place>,
    },
    RecordEvent {
        stream: Place,
        event: Place,
    },
    WaitEvent {
        event: Place,
        stream: Option<Place>,
    },
    Eval(PendingRvalue),
    Nop,
    Pending(PendingStatement),
    StaticStore {
        id: StaticId,
        value: Operand,
    },
    InlineAsm(InlineAsm),
}

/// Inline assembly representation in MIR.
#[derive(Debug, Clone)]
pub struct InlineAsm {
    pub template: Vec<InlineAsmTemplatePiece>,
    pub operands: Vec<InlineAsmOperand>,
    pub clobbers: Vec<InlineAsmRegister>,
    pub options: InlineAsmOptions,
    pub span: Option<Span>,
}

/// Template fragment emitted for inline assembly.
#[derive(Debug, Clone)]
pub enum InlineAsmTemplatePiece {
    Literal(String),
    Placeholder {
        operand_idx: usize,
        modifier: Option<String>,
        span: Option<Span>,
    },
}

/// Register selector supported by inline assembly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InlineAsmRegister {
    Class(InlineAsmRegisterClass),
    Explicit(String),
}

/// Register classes recognised by inline assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Operand supplied to an inline assembly expression.
#[derive(Debug, Clone)]
pub struct InlineAsmOperand {
    pub name: Option<String>,
    pub reg: InlineAsmRegister,
    pub kind: InlineAsmOperandKind,
    pub span: Option<Span>,
}

/// Operand categories for inline assembly.
#[derive(Debug, Clone)]
pub enum InlineAsmOperandKind {
    In {
        value: Operand,
    },
    Out {
        place: Place,
        late: bool,
    },
    InOut {
        input: Operand,
        output: Place,
        late: bool,
    },
    Const {
        value: Operand,
    },
    Sym {
        symbol: String,
    },
}

/// Copy directions recognised by accelerator copy statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceleratorCopyKind {
    HostToDevice,
    DeviceToHost,
    DeviceToDevice,
    PeerToPeer,
}

/// Inline assembly options (mirrors Rust `asm!` surface).
#[derive(Debug, Clone, Default)]
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

/// Terminators transfer control at the end of a block.
/// Kept adjacent to `BasicBlock`/`Statement` for the string-backed MIR definitions.
#[derive(Debug, Clone)]
pub enum Terminator {
    Goto {
        target: BlockId,
    },
    SwitchInt {
        discr: Operand,
        targets: Vec<(i128, BlockId)>,
        otherwise: BlockId,
    },
    Match {
        value: Place,
        arms: Vec<MatchArm>,
        otherwise: BlockId,
    },
    Return,
    Call {
        func: Operand,
        args: Vec<Operand>,
        arg_modes: Vec<ParamMode>,
        destination: Option<Place>,
        target: BlockId,
        unwind: Option<BlockId>,
        dispatch: Option<CallDispatch>,
    },
    Yield {
        value: Operand,
        resume: BlockId,
        drop: BlockId,
    },
    Await {
        future: Place,
        destination: Option<Place>,
        resume: BlockId,
        drop: BlockId,
    },
    Throw {
        exception: Option<Operand>,
        ty: Option<Ty>,
    },
    Panic,
    Unreachable,
    Pending(PendingTerminator),
}

/// Metadata required to lower a `dyn Trait` call through a vtable slot.
#[derive(Debug, Clone)]
pub struct TraitObjectDispatch {
    pub trait_name: String,
    pub method: String,
    pub slot_index: u32,
    pub slot_count: u32,
    pub receiver_index: usize,
    pub impl_type: Option<String>,
}

/// Dispatch metadata attached to indirect calls.
#[derive(Debug, Clone)]
pub enum CallDispatch {
    Trait(TraitObjectDispatch),
    Virtual(VirtualDispatch),
}

/// Metadata for class virtual calls.
#[derive(Debug, Clone)]
pub struct VirtualDispatch {
    pub slot_index: u32,
    pub receiver_index: usize,
    pub base_owner: Option<String>,
}

/// Borrow classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Unique,
    Raw,
}

/// An lvalue/Place in MIR.
#[derive(Debug, Clone)]
pub struct Place {
    pub local: LocalId,
    pub projection: Vec<ProjectionElem>,
}

impl Place {
    #[must_use]
    pub fn new(local: LocalId) -> Self {
        new_place(local)
    }
}

/// Projection applied when traversing an aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionElem {
    Field(u32),
    FieldNamed(String),
    Index(LocalId),
    ConstantIndex {
        offset: usize,
        length: usize,
        from_end: bool,
    },
    Deref,
    Downcast {
        variant: u32,
    },
    Subslice {
        from: usize,
        to: usize,
    },
    UnionField {
        index: u32,
        name: String,
    },
}

/// Operand consumed by statements/terminators.
#[derive(Debug, Clone)]
pub enum Operand {
    Copy(Place),
    Move(Place),
    Borrow(BorrowOperand),
    Mmio(MmioOperand),
    Const(ConstOperand),
    Pending(PendingOperand),
}

#[derive(Debug, Clone)]
pub struct ConstOperand {
    pub value: ConstValue,
    pub literal: Option<crate::syntax::numeric::NumericLiteralMetadata>,
}

impl ConstOperand {
    #[must_use]
    pub fn new(value: ConstValue) -> Self {
        Self {
            value,
            literal: None,
        }
    }

    #[must_use]
    pub fn value(&self) -> &ConstValue {
        &self.value
    }

    #[must_use]
    pub fn literal(&self) -> Option<&crate::syntax::numeric::NumericLiteralMetadata> {
        self.literal.as_ref()
    }

    #[must_use]
    pub fn symbol_name(&self) -> Option<&str> {
        if let ConstValue::Symbol(name) = &self.value {
            Some(name.as_str())
        } else {
            None
        }
    }

    #[must_use]
    pub fn with_literal(
        value: ConstValue,
        literal: Option<crate::syntax::numeric::NumericLiteralMetadata>,
    ) -> Self {
        Self { value, literal }
    }
}

/// Borrow operand.
#[derive(Debug, Clone)]
pub struct BorrowOperand {
    pub kind: BorrowKind,
    pub place: Place,
    pub region: RegionVar,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct MmioOperand {
    pub base_address: u64,
    pub offset: u32,
    pub width_bits: u16,
    pub access: MmioAccess,
    pub endianness: MmioEndianness,
    pub address_space: AddressSpaceId,
    pub requires_unsafe: bool,
    pub ty: Ty,
    pub name: Option<String>,
}

/// Supported floating widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatWidth {
    F16,
    F32,
    F64,
    F128,
}

/// Supported integer widths for numeric intrinsics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericWidth {
    W8,
    W16,
    W32,
    W64,
    W128,
    Pointer,
}

/// Intrinsic operations on scalar integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericIntrinsicKind {
    TryAdd,
    TrySub,
    TryMul,
    TryNeg,
    LeadingZeroCount,
    TrailingZeroCount,
    PopCount,
    RotateLeft,
    RotateRight,
    ReverseEndianness,
    IsPowerOfTwo,
}

/// Rvalue representation for numeric intrinsics (checked arithmetic and bit ops).
#[derive(Debug, Clone)]
pub struct NumericIntrinsic {
    pub kind: NumericIntrinsicKind,
    pub width: NumericWidth,
    pub signed: bool,
    pub symbol: String,
    pub operands: Vec<Operand>,
    pub out: Option<Place>,
}

impl FloatWidth {
    #[must_use]
    pub fn bits(self) -> u16 {
        match self {
            FloatWidth::F16 => 16,
            FloatWidth::F32 => 32,
            FloatWidth::F64 => 64,
            FloatWidth::F128 => 128,
        }
    }
}

/// IEEE 754 rounding modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    NearestTiesToEven,
    NearestTiesToAway,
    TowardZero,
    TowardPositive,
    TowardNegative,
}

/// IEEE 754 status flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FloatStatusFlags {
    pub invalid: bool,
    pub div_by_zero: bool,
    pub overflow: bool,
    pub underflow: bool,
    pub inexact: bool,
}

impl FloatStatusFlags {
    #[must_use]
    pub fn any(self) -> bool {
        self.invalid || self.div_by_zero || self.overflow || self.underflow || self.inexact
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn merge(&mut self, other: Self) {
        self.invalid |= other.invalid;
        self.div_by_zero |= other.div_by_zero;
        self.overflow |= other.overflow;
        self.underflow |= other.underflow;
        self.inexact |= other.inexact;
    }
}

/// Floating-point literal/constant with exact bit representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatValue {
    pub bits: u128,
    pub width: FloatWidth,
}

impl FloatValue {
    #[must_use]
    pub fn f16_from_bits(bits: u16) -> Self {
        Self {
            bits: u128::from(bits),
            width: FloatWidth::F16,
        }
    }

    #[must_use]
    pub fn f32_from_bits(bits: u32) -> Self {
        Self {
            bits: u128::from(bits),
            width: FloatWidth::F32,
        }
    }

    #[must_use]
    pub fn f64_from_bits(bits: u64) -> Self {
        Self {
            bits: bits.into(),
            width: FloatWidth::F64,
        }
    }

    #[must_use]
    pub fn f128_from_bits(bits: u128) -> Self {
        Self {
            bits,
            width: FloatWidth::F128,
        }
    }

    #[must_use]
    pub fn from_f16(value: f32) -> Self {
        let bits = f32_to_f16_bits(value);
        Self::f16_from_bits(bits)
    }

    #[must_use]
    pub fn from_f32(value: f32) -> Self {
        Self::f32_from_bits(value.to_bits())
    }

    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        Self::f64_from_bits(value.to_bits())
    }

    #[must_use]
    pub fn from_f64_as(value: f64, width: FloatWidth) -> Self {
        match width {
            FloatWidth::F16 => Self::from_f16(value as f32),
            FloatWidth::F32 => Self::from_f32(value as f32),
            FloatWidth::F64 => Self::from_f64(value),
            FloatWidth::F128 => {
                let bits = f64_to_binary128_bits(value);
                Self::f128_from_bits(bits)
            }
        }
    }

    #[must_use]
    pub fn to_f64(self) -> f64 {
        match self.width {
            FloatWidth::F128 => binary128_bits_to_f64(self.bits),
            FloatWidth::F64 => f64::from_bits(self.bits as u64),
            FloatWidth::F32 => f32::from_bits(self.bits as u32) as f64,
            FloatWidth::F16 => f16_bits_to_f32(self.bits as u16) as f64,
        }
    }

    #[must_use]
    pub fn to_f32(self) -> f32 {
        match self.width {
            FloatWidth::F128 => self.to_f64() as f32,
            FloatWidth::F64 => f64::from_bits(self.bits as u64) as f32,
            FloatWidth::F32 => f32::from_bits(self.bits as u32),
            FloatWidth::F16 => f16_bits_to_f32(self.bits as u16),
        }
    }

    #[must_use]
    pub fn to_bits(self) -> u128 {
        self.bits
    }

    #[must_use]
    pub fn sign_bit(self) -> bool {
        match self.width {
            FloatWidth::F128 => (self.bits >> 127) != 0,
            FloatWidth::F64 => (self.bits >> 63) != 0,
            FloatWidth::F32 => (self.bits >> 31) != 0,
            FloatWidth::F16 => (self.bits >> 15) != 0,
        }
    }

    #[must_use]
    pub fn magnitude_bits(self) -> u128 {
        match self.width {
            FloatWidth::F128 => self.bits & 0x7FFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF,
            FloatWidth::F64 => self.bits & 0x7FFF_FFFF_FFFF_FFFF,
            FloatWidth::F32 => self.bits & 0x7FFF_FFFF,
            FloatWidth::F16 => self.bits & 0x7FFF,
        }
    }

    #[must_use]
    pub fn is_negative_zero(self) -> bool {
        self.sign_bit() && self.magnitude_bits() == 0
    }

    #[must_use]
    pub fn is_nan(self) -> bool {
        match self.width {
            FloatWidth::F128 => {
                let magnitude = self.magnitude_bits();
                let exponent = (magnitude >> 112) & 0x7FFF;
                let mantissa = magnitude & ((1u128 << 112) - 1);
                exponent == 0x7FFF && mantissa != 0
            }
            FloatWidth::F64 => {
                let magnitude = self.magnitude_bits();
                magnitude > 0x7FF0_0000_0000_0000 && magnitude <= 0x7FFF_FFFF_FFFF_FFFF
            }
            FloatWidth::F32 => {
                let magnitude = self.magnitude_bits();
                magnitude > 0x7F80_0000 && magnitude <= 0x7FFF_FFFF
            }
            FloatWidth::F16 => {
                let magnitude = self.magnitude_bits();
                let exponent = (magnitude >> 10) & 0x1F;
                let mantissa = magnitude & 0x3FF;
                exponent == 0x1F && mantissa != 0
            }
        }
    }

    #[must_use]
    pub fn hex_bits(self) -> String {
        match self.width {
            FloatWidth::F128 => format!("0x{:032x}", self.bits),
            FloatWidth::F64 => format!("0x{:016x}", self.bits as u64),
            FloatWidth::F32 => format!("0x{:08x}", self.bits as u32),
            FloatWidth::F16 => format!("0x{:04x}", self.bits as u16),
        }
    }

    #[must_use]
    pub fn display(self) -> String {
        if self.is_nan() {
            return format!("nan({})", self.hex_bits());
        }
        if self.is_negative_zero() {
            return "-0.0".to_string();
        }
        let value = self.to_f64();
        value.to_string()
    }
}

const F128_EXP_BIAS: i32 = 16_383;
const F128_MANT_BITS: u32 = 112;

fn f16_bits_to_f32(bits: u16) -> f32 {
    f16::from_bits(bits).to_f32()
}

fn f32_to_f16_bits(value: f32) -> u16 {
    f16::from_f32(value).to_bits()
}

fn f64_to_binary128_bits(value: f64) -> u128 {
    let sign_bit = if value.is_sign_negative() { 1u128 } else { 0 };
    if value.is_nan() {
        let payload = (value.to_bits() & 0x000F_FFFF_FFFF_FFFF) as u128;
        let mantissa =
            ((1u128 << (F128_MANT_BITS - 1)) | (payload << 59)) & ((1u128 << F128_MANT_BITS) - 1);
        return (sign_bit << 127) | (0x7FFFu128 << 112) | mantissa;
    }
    if value.is_infinite() {
        return (sign_bit << 127) | (0x7FFFu128 << 112);
    }
    if value == 0.0 {
        return sign_bit << 127;
    }

    let abs = value.abs();
    let exp = abs.log2().floor() as i32;
    let biased_exp = exp + F128_EXP_BIAS;
    let mantissa_scale = 1u128 << F128_MANT_BITS;

    if biased_exp <= 0 {
        // Subnormal in binary128.
        let scaled = abs * 2f64.powi(F128_EXP_BIAS - 1);
        let mantissa = (scaled * mantissa_scale as f64).round() as u128;
        return (sign_bit << 127) | mantissa;
    }
    if biased_exp >= 0x7FFF {
        return (sign_bit << 127) | (0x7FFFu128 << 112);
    }

    let normalized = abs / 2f64.powi(exp);
    let fraction = normalized - 1.0;
    let mut mantissa = (fraction * mantissa_scale as f64).round() as u128;
    if mantissa >= mantissa_scale {
        mantissa = mantissa_scale - 1;
    }

    (sign_bit << 127) | ((biased_exp as u128) << 112) | (mantissa & (mantissa_scale - 1))
}

fn binary128_bits_to_f64(bits: u128) -> f64 {
    let sign = (bits >> 127) != 0;
    let exponent = ((bits >> 112) & 0x7FFF) as i32;
    let mantissa = bits & ((1u128 << F128_MANT_BITS) - 1);
    let value = if exponent == 0x7FFF {
        if mantissa == 0 {
            f64::INFINITY
        } else {
            f64::NAN
        }
    } else if exponent == 0 {
        let fraction = mantissa as f64 / (1u128 << F128_MANT_BITS) as f64;
        fraction * 2f64.powi(1 - F128_EXP_BIAS)
    } else {
        let fraction = 1.0 + mantissa as f64 / (1u128 << F128_MANT_BITS) as f64;
        fraction * 2f64.powi(exponent - F128_EXP_BIAS)
    };
    if sign { -value } else { value }
}

/// Constant values recognised by MIR.
#[derive(Debug, Clone)]
pub enum ConstValue {
    Int(i128),
    Int32(i128),
    UInt(u128),
    Float(FloatValue),
    Decimal(Decimal128),
    Bool(bool),
    Char(u16),
    Str {
        id: StrId,
        value: String,
    },
    RawStr(String),
    Symbol(String),
    Enum {
        type_name: String,
        variant: String,
        discriminant: i128,
    },
    Struct {
        type_name: String,
        fields: Vec<(String, ConstValue)>,
    },
    Null,
    Unit,
    Unknown,
}

/// Rvalue representing the right-hand side of an assignment.
#[derive(Debug, Clone)]
pub enum Rvalue {
    Use(Operand),
    Unary {
        op: UnOp,
        operand: Operand,
        rounding: Option<RoundingMode>,
    },
    Binary {
        op: BinOp,
        lhs: Operand,
        rhs: Operand,
        rounding: Option<RoundingMode>,
    },
    Aggregate {
        kind: AggregateKind,
        fields: Vec<Operand>,
    },
    AddressOf {
        mutability: Mutability,
        place: Place,
    },
    Len(Place),
    SpanStackAlloc {
        element: Ty,
        length: Operand,
        source: Option<Operand>,
    },
    Cast {
        kind: CastKind,
        operand: Operand,
        source: Ty,
        target: Ty,
        rounding: Option<RoundingMode>,
    },
    StringInterpolate {
        segments: Vec<InterpolatedStringSegment>,
    },
    NumericIntrinsic(NumericIntrinsic),
    DecimalIntrinsic(DecimalIntrinsic),
    AtomicLoad {
        target: Place,
        order: AtomicOrdering,
    },
    AtomicRmw {
        op: AtomicRmwOp,
        target: Place,
        value: Operand,
        order: AtomicOrdering,
    },
    AtomicCompareExchange {
        target: Place,
        expected: Operand,
        desired: Operand,
        success: AtomicOrdering,
        failure: AtomicOrdering,
        weak: bool,
    },
    Pending(PendingRvalue),
    StaticLoad {
        id: StaticId,
    },
    StaticRef {
        id: StaticId,
    },
}

/// Unary operators supported by MIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
    UnaryPlus,
    Increment,
    Decrement,
    Deref,
    AddrOf,
    AddrOfMut,
}

#[derive(Debug, Clone)]
pub struct DecimalIntrinsic {
    pub kind: DecimalIntrinsicKind,
    pub lhs: Operand,
    pub rhs: Operand,
    pub addend: Option<Operand>,
    pub rounding: Operand,
    pub vectorize: Operand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecimalIntrinsicKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Fma,
}

impl DecimalIntrinsicKind {
    #[must_use]
    pub fn operand_count(self) -> usize {
        match self {
            Self::Fma => 3,
            Self::Add | Self::Sub | Self::Mul | Self::Div | Self::Rem => 2,
        }
    }
}

/// Binary operators supported by MIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    NullCoalesce,
}

/// Aggregate construction.
#[derive(Debug, Clone)]
pub enum AggregateKind {
    Tuple,
    Array,
    Adt {
        name: String,
        variant: Option<String>,
    },
}

/// Casting semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    IntToInt,
    IntToFloat,
    FloatToInt,
    FloatToFloat,
    PointerToInt,
    IntToPointer,
    DynTrait,
    Unknown,
}

/// Mutability marker used by `AddressOf` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Immutable,
    Mutable,
}

/// Match arm used by structured pattern matching.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<MatchGuard>,
    pub bindings: Vec<PatternBinding>,
    pub target: BlockId,
}

/// Guard expression attached to a match arm.
#[derive(Debug, Clone)]
pub struct MatchGuard {
    pub expr: String,
    pub span: Option<Span>,
    pub parsed: bool,
}

/// Structured pattern used during `match` lowering.
#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Literal(ConstValue),
    Type(Ty),
    Binding(BindingPattern),
    Tuple(Vec<Pattern>),
    Struct {
        path: Vec<String>,
        fields: Vec<PatternField>,
    },
    Enum {
        path: Vec<String>,
        variant: String,
        fields: VariantPatternFields,
    },
}

/// Binding captured by a pattern (mode + mutability).
#[derive(Debug, Clone)]
pub struct BindingPattern {
    pub name: String,
    pub mutability: PatternBindingMutability,
    pub mode: PatternBindingMode,
}

/// Binding mode requested by the pattern author.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternBindingMode {
    Value,
    In,
    Ref,
    RefReadonly,
    Move,
}

/// Whether the binding is mutable (`var`) or immutable (`let`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternBindingMutability {
    Immutable,
    Mutable,
}

/// Binding information extracted from a pattern.
#[derive(Debug, Clone)]
pub struct PatternBinding {
    pub name: String,
    pub local: LocalId,
    pub projection: Vec<PatternProjectionElem>,
    pub span: Option<Span>,
    pub mutability: PatternBindingMutability,
    pub mode: PatternBindingMode,
}

/// One segment of a pattern binding projection.
#[derive(Debug, Clone)]
pub enum PatternProjectionElem {
    Variant { path: Vec<String>, variant: String },
    FieldNamed(String),
    FieldIndex(u32),
    Index(LocalId),
    Subslice { from: usize, to: usize },
}

/// Field entry for struct/enum patterns.
#[derive(Debug, Clone)]
pub struct PatternField {
    pub name: String,
    pub pattern: Pattern,
}

/// Enum variant payload description.
#[derive(Debug, Clone)]
pub enum VariantPatternFields {
    Unit,
    Tuple(Vec<Pattern>),
    Struct(Vec<PatternField>),
}

/// Structured record of statements not yet fully lowered.
#[derive(Debug, Clone)]
pub struct PendingStatement {
    pub kind: PendingStatementKind,
    pub detail: Option<String>,
}

/// Classifies why a statement is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingStatementKind {
    Expression,
    Const,
    Break,
    Continue,
    Goto,
    Throw,
    If,
    While,
    DoWhile,
    For,
    Foreach,
    Switch,
    Try,
    Region,
    Using,
    Lock,
    Checked,
    Atomic,
    Unchecked,
    YieldReturn,
    YieldBreak,
    Fixed,
    Unsafe,
    Labeled,
}

/// Terminators awaiting a richer lowering.
#[derive(Debug, Clone)]
pub struct PendingTerminator {
    pub kind: PendingTerminatorKind,
    pub detail: Option<String>,
}

/// Classifies pending terminators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingTerminatorKind {
    Branch,
    Loop,
    Await,
    Yield,
    Exception,
    Unknown,
}

/// Placeholder operand recorded when we cannot yet produce MIR-level semantics.
#[derive(Debug, Clone)]
pub struct PendingOperand {
    pub category: ValueCategory,
    pub repr: String,
    pub span: Option<Span>,
    pub info: Option<Box<PendingOperandInfo>>,
}

#[derive(Debug, Clone)]
pub enum PendingOperandInfo {
    FunctionGroup {
        path: String,
        candidates: Vec<PendingFunctionCandidate>,
        receiver: Option<Box<Operand>>,
    },
}

#[derive(Debug, Clone)]
pub struct PendingFunctionCandidate {
    pub qualified: String,
    pub signature: FnTy,
    pub is_static: bool,
}

/// Placeholder Rvalue recorded when expression lowering is deferred.
#[derive(Debug, Clone)]
pub struct PendingRvalue {
    pub repr: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomicOrdering {
    Relaxed,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

impl AtomicOrdering {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Relaxed => "Relaxed",
            Self::Acquire => "Acquire",
            Self::Release => "Release",
            Self::AcqRel => "AcqRel",
            Self::SeqCst => "SeqCst",
        }
    }

    #[must_use]
    pub fn from_variant(name: &str) -> Option<Self> {
        match name {
            "Relaxed" => Some(Self::Relaxed),
            "Acquire" => Some(Self::Acquire),
            "Release" => Some(Self::Release),
            "AcqRel" => Some(Self::AcqRel),
            "SeqCst" => Some(Self::SeqCst),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomicFenceScope {
    Full,
    BlockEnter,
    BlockExit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomicRmwOp {
    Exchange,
    Add,
    Sub,
    And,
    Or,
    Xor,
    Min,
    Max,
}

/// Segment composing a runtime string interpolation.
#[derive(Debug, Clone)]
pub enum InterpolatedStringSegment {
    Text {
        id: StrId,
    },
    Expr {
        operand: Operand,
        alignment: Option<i32>,
        format: Option<StrId>,
        expr_text: String,
        span: Option<Span>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_value_preserves_bits_and_sign() {
        let neg_zero = FloatValue::from_f64(-0.0);
        assert!(neg_zero.is_negative_zero());
        assert_eq!(neg_zero.hex_bits(), "0x8000000000000000");
        assert_eq!(neg_zero.display(), "-0.0");

        let payload = FloatValue::f32_from_bits(0x7fc0_0123);
        assert!(payload.is_nan());
        assert_eq!(payload.hex_bits(), "0x7fc00123");
        assert!(payload.display().starts_with("nan("));
    }

    #[test]
    fn float_value_handles_f16_and_f128_bits() {
        let f16_zero = FloatValue::f16_from_bits(0x8000);
        assert!(f16_zero.is_negative_zero());
        assert_eq!(f16_zero.hex_bits(), "0x8000");

        let quad_nan = FloatValue::f128_from_bits(0x7fff_8000_0000_0000_0000_0000_0000_0123);
        assert!(quad_nan.is_nan());
        assert_eq!(quad_nan.hex_bits(), "0x7fff8000000000000000000000000123");
        assert!(quad_nan.display().contains("nan("));
    }
}

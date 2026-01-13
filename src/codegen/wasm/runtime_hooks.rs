use super::module_builder::FunctionSignature;
use super::types::ValueType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RuntimeHook {
    ObjectNew,
    Panic,
    Abort,
    Throw,
    HasPendingException,
    TakePendingException,
    Await,
    Yield,
    AsyncCancel,
    AsyncSpawn,
    AsyncSpawnLocal,
    AsyncScope,
    AsyncTaskHeader,
    AsyncTaskResult,
    AsyncTokenState,
    AsyncTokenNew,
    AsyncTokenCancel,
    BorrowShared,
    BorrowUnique,
    BorrowRelease,
    DropResource,
    DropMissing,
    TypeSize,
    TypeAlign,
    TypeMetadata,
    TypeDropGlue,
    TypeCloneGlue,
    TypeHashGlue,
    TypeEqGlue,
    DropInvoke,
    HashInvoke,
    EqInvoke,
    TraceEnter,
    TraceExit,
    TraceFlush,
    CoverageHit,
    Alloc,
    AllocZeroed,
    Realloc,
    Free,
    Memcpy,
    Memmove,
    Memset,
    MmioRead,
    MmioWrite,
    StringFromSlice,
    StringClone,
    StringCloneSlice,
    StringDrop,
    VecWithCapacity,
    VecClone,
    VecIntoArray,
    VecCopyToArray,
    VecDrop,
    ArrayIntoVec,
    ArrayCopyToVec,
    HashSetNew,
    HashSetWithCapacity,
    HashSetDrop,
    HashSetClear,
    HashSetReserve,
    HashSetShrinkTo,
    HashSetLen,
    HashSetCapacity,
    HashSetTombstones,
    HashSetInsert,
    HashSetReplace,
    HashSetContains,
    HashSetGetPtr,
    HashSetTake,
    HashSetRemove,
    HashSetTakeAt,
    HashSetBucketState,
    HashSetBucketHash,
    HashSetIter,
    HashSetIterNext,
    HashSetIterNextPtr,
    HashMapNew,
    HashMapWithCapacity,
    HashMapDrop,
    HashMapClear,
    HashMapReserve,
    HashMapShrinkTo,
    HashMapLen,
    HashMapCapacity,
    HashMapInsert,
    HashMapContains,
    HashMapGetPtr,
    HashMapTake,
    HashMapRemove,
    HashMapBucketState,
    HashMapBucketHash,
    HashMapTakeAt,
    HashMapIter,
    HashMapIterNext,
    RcClone,
    RcDrop,
    ArcNew,
    ArcClone,
    ArcDrop,
    ArcGet,
    ArcGetMut,
    ArcDowngrade,
    WeakClone,
    WeakDrop,
    WeakUpgrade,
    ArcStrongCount,
    ArcWeakCount,
    StringAppendSlice,
    StringAppendBool,
    StringAppendChar,
    StringAppendSigned,
    StringAppendUnsigned,
    StringAppendF32,
    StringAppendF64,
    StringAsSlice,
    StringTryCopyUtf8,
    StringAsChars,
    StrAsChars,
    F32Rem,
    F64Rem,
    MathAbsF64,
    MathFloorF64,
    MathCeilF64,
    MathTruncF64,
    MathCopySignF64,
    MathBitIncrementF64,
    MathBitDecrementF64,
    MathScaleBF64,
    MathILogBF64,
    MathIeeeRemainderF64,
    MathFmaF64,
    MathCbrtF64,
    MathSqrtF64,
    MathPowF64,
    MathSinF64,
    MathCosF64,
    MathTanF64,
    MathAsinF64,
    MathAcosF64,
    MathAtanF64,
    MathAtan2F64,
    MathSinhF64,
    MathCoshF64,
    MathTanhF64,
    MathAsinhF64,
    MathAcoshF64,
    MathAtanhF64,
    MathExpF64,
    MathLogF64,
    MathLog10F64,
    MathLog2F64,
    MathRoundF64,
    MathAbsF32,
    MathFloorF32,
    MathCeilF32,
    MathTruncF32,
    MathCopySignF32,
    MathBitIncrementF32,
    MathBitDecrementF32,
    MathScaleBF32,
    MathILogBF32,
    MathIeeeRemainderF32,
    MathFmaF32,
    MathCbrtF32,
    MathSqrtF32,
    MathPowF32,
    MathSinF32,
    MathCosF32,
    MathTanF32,
    MathAsinF32,
    MathAcosF32,
    MathAtanF32,
    MathAtan2F32,
    MathSinhF32,
    MathCoshF32,
    MathTanhF32,
    MathAsinhF32,
    MathAcoshF32,
    MathAtanhF32,
    MathExpF32,
    MathLogF32,
    MathLog10F32,
    MathLog2F32,
    MathRoundF32,
    I128Add,
    U128Add,
    I128Sub,
    U128Sub,
    I128Mul,
    U128Mul,
    I128Div,
    U128Div,
    I128Rem,
    U128Rem,
    I128Eq,
    U128Eq,
    I128Cmp,
    U128Cmp,
    I128Neg,
    I128Not,
    U128Not,
    I128And,
    U128And,
    I128Or,
    U128Or,
    I128Xor,
    U128Xor,
    I128Shl,
    U128Shl,
    I128Shr,
    U128Shr,
    SpanCopyTo,
    SpanFromRawMut,
    SpanFromRawConst,
    SpanSliceMut,
    SpanSliceReadonly,
    SpanToReadonly,
    SpanPtrAtMut,
    SpanPtrAtReadonly,
    DecimalAdd,
    DecimalAddSimd,
    DecimalSub,
    DecimalSubSimd,
    DecimalMul,
    DecimalMulSimd,
    DecimalDiv,
    DecimalDivSimd,
    DecimalRem,
    DecimalRemSimd,
    DecimalFma,
    DecimalFmaSimd,
    DecimalSum,
    DecimalDot,
    DecimalMatMul,
    ClosureEnvAlloc,
    ClosureEnvClone,
    ClosureEnvFree,
}

pub(crate) const ALL_RUNTIME_HOOKS: &[RuntimeHook] = &[
    RuntimeHook::ObjectNew,
    RuntimeHook::Panic,
    RuntimeHook::Abort,
    RuntimeHook::Throw,
    RuntimeHook::HasPendingException,
    RuntimeHook::TakePendingException,
    RuntimeHook::Await,
    RuntimeHook::Yield,
    RuntimeHook::AsyncCancel,
    RuntimeHook::AsyncSpawn,
    RuntimeHook::AsyncSpawnLocal,
    RuntimeHook::AsyncScope,
    RuntimeHook::AsyncTaskHeader,
    RuntimeHook::AsyncTaskResult,
    RuntimeHook::AsyncTokenState,
    RuntimeHook::AsyncTokenNew,
    RuntimeHook::AsyncTokenCancel,
    RuntimeHook::BorrowShared,
    RuntimeHook::BorrowUnique,
    RuntimeHook::BorrowRelease,
    RuntimeHook::DropResource,
    RuntimeHook::DropMissing,
    RuntimeHook::TypeSize,
    RuntimeHook::TypeAlign,
    RuntimeHook::TypeMetadata,
    RuntimeHook::TypeDropGlue,
    RuntimeHook::TypeCloneGlue,
    RuntimeHook::TypeHashGlue,
    RuntimeHook::TypeEqGlue,
    RuntimeHook::TraceEnter,
    RuntimeHook::TraceExit,
    RuntimeHook::TraceFlush,
    RuntimeHook::CoverageHit,
    RuntimeHook::Alloc,
    RuntimeHook::AllocZeroed,
    RuntimeHook::Realloc,
    RuntimeHook::Free,
    RuntimeHook::Memcpy,
    RuntimeHook::Memmove,
    RuntimeHook::Memset,
    RuntimeHook::StringFromSlice,
    RuntimeHook::StringClone,
    RuntimeHook::StringCloneSlice,
    RuntimeHook::StringDrop,
    RuntimeHook::VecWithCapacity,
    RuntimeHook::VecClone,
    RuntimeHook::VecIntoArray,
    RuntimeHook::VecCopyToArray,
    RuntimeHook::VecDrop,
    RuntimeHook::ArrayIntoVec,
    RuntimeHook::ArrayCopyToVec,
    RuntimeHook::HashSetNew,
    RuntimeHook::HashSetWithCapacity,
    RuntimeHook::HashSetDrop,
    RuntimeHook::HashSetClear,
    RuntimeHook::HashSetReserve,
    RuntimeHook::HashSetShrinkTo,
    RuntimeHook::HashSetLen,
    RuntimeHook::HashSetCapacity,
    RuntimeHook::HashSetTombstones,
    RuntimeHook::HashSetInsert,
    RuntimeHook::HashSetReplace,
    RuntimeHook::HashSetContains,
    RuntimeHook::HashSetGetPtr,
    RuntimeHook::HashSetTake,
    RuntimeHook::HashSetRemove,
    RuntimeHook::HashSetTakeAt,
    RuntimeHook::HashSetBucketState,
    RuntimeHook::HashSetBucketHash,
    RuntimeHook::HashSetIter,
    RuntimeHook::HashSetIterNext,
    RuntimeHook::HashSetIterNextPtr,
    RuntimeHook::HashMapNew,
    RuntimeHook::HashMapWithCapacity,
    RuntimeHook::HashMapDrop,
    RuntimeHook::HashMapClear,
    RuntimeHook::HashMapReserve,
    RuntimeHook::HashMapShrinkTo,
    RuntimeHook::HashMapLen,
    RuntimeHook::HashMapCapacity,
    RuntimeHook::HashMapInsert,
    RuntimeHook::HashMapContains,
    RuntimeHook::HashMapGetPtr,
    RuntimeHook::HashMapTake,
    RuntimeHook::HashMapRemove,
    RuntimeHook::HashMapBucketState,
    RuntimeHook::HashMapBucketHash,
    RuntimeHook::HashMapTakeAt,
    RuntimeHook::HashMapIter,
    RuntimeHook::HashMapIterNext,
    RuntimeHook::RcClone,
    RuntimeHook::RcDrop,
    RuntimeHook::ArcNew,
    RuntimeHook::ArcClone,
    RuntimeHook::ArcDrop,
    RuntimeHook::ArcGet,
    RuntimeHook::ArcGetMut,
    RuntimeHook::ArcDowngrade,
    RuntimeHook::WeakClone,
    RuntimeHook::WeakDrop,
    RuntimeHook::WeakUpgrade,
    RuntimeHook::ArcStrongCount,
    RuntimeHook::ArcWeakCount,
    RuntimeHook::StringAppendSlice,
    RuntimeHook::StringAppendBool,
    RuntimeHook::StringAppendChar,
    RuntimeHook::StringAppendSigned,
    RuntimeHook::StringAppendUnsigned,
    RuntimeHook::StringAppendF32,
    RuntimeHook::StringAppendF64,
    RuntimeHook::StringAsSlice,
    RuntimeHook::StringTryCopyUtf8,
    RuntimeHook::StringAsChars,
    RuntimeHook::StrAsChars,
    RuntimeHook::F32Rem,
    RuntimeHook::F64Rem,
    RuntimeHook::MathAbsF64,
    RuntimeHook::MathFloorF64,
    RuntimeHook::MathCeilF64,
    RuntimeHook::MathTruncF64,
    RuntimeHook::MathCopySignF64,
    RuntimeHook::MathBitIncrementF64,
    RuntimeHook::MathBitDecrementF64,
    RuntimeHook::MathScaleBF64,
    RuntimeHook::MathILogBF64,
    RuntimeHook::MathIeeeRemainderF64,
    RuntimeHook::MathFmaF64,
    RuntimeHook::MathCbrtF64,
    RuntimeHook::MathSqrtF64,
    RuntimeHook::MathPowF64,
    RuntimeHook::MathSinF64,
    RuntimeHook::MathCosF64,
    RuntimeHook::MathTanF64,
    RuntimeHook::MathAsinF64,
    RuntimeHook::MathAcosF64,
    RuntimeHook::MathAtanF64,
    RuntimeHook::MathAtan2F64,
    RuntimeHook::MathSinhF64,
    RuntimeHook::MathCoshF64,
    RuntimeHook::MathTanhF64,
    RuntimeHook::MathAsinhF64,
    RuntimeHook::MathAcoshF64,
    RuntimeHook::MathAtanhF64,
    RuntimeHook::MathExpF64,
    RuntimeHook::MathLogF64,
    RuntimeHook::MathLog10F64,
    RuntimeHook::MathLog2F64,
    RuntimeHook::MathRoundF64,
    RuntimeHook::MathAbsF32,
    RuntimeHook::MathFloorF32,
    RuntimeHook::MathCeilF32,
    RuntimeHook::MathTruncF32,
    RuntimeHook::MathCopySignF32,
    RuntimeHook::MathBitIncrementF32,
    RuntimeHook::MathBitDecrementF32,
    RuntimeHook::MathScaleBF32,
    RuntimeHook::MathILogBF32,
    RuntimeHook::MathIeeeRemainderF32,
    RuntimeHook::MathFmaF32,
    RuntimeHook::MathCbrtF32,
    RuntimeHook::MathSqrtF32,
    RuntimeHook::MathPowF32,
    RuntimeHook::MathSinF32,
    RuntimeHook::MathCosF32,
    RuntimeHook::MathTanF32,
    RuntimeHook::MathAsinF32,
    RuntimeHook::MathAcosF32,
    RuntimeHook::MathAtanF32,
    RuntimeHook::MathAtan2F32,
    RuntimeHook::MathSinhF32,
    RuntimeHook::MathCoshF32,
    RuntimeHook::MathTanhF32,
    RuntimeHook::MathAsinhF32,
    RuntimeHook::MathAcoshF32,
    RuntimeHook::MathAtanhF32,
    RuntimeHook::MathExpF32,
    RuntimeHook::MathLogF32,
    RuntimeHook::MathLog10F32,
    RuntimeHook::MathLog2F32,
    RuntimeHook::MathRoundF32,
    RuntimeHook::I128Add,
    RuntimeHook::U128Add,
    RuntimeHook::I128Sub,
    RuntimeHook::U128Sub,
    RuntimeHook::I128Mul,
    RuntimeHook::U128Mul,
    RuntimeHook::I128Div,
    RuntimeHook::U128Div,
    RuntimeHook::I128Rem,
    RuntimeHook::U128Rem,
    RuntimeHook::I128Eq,
    RuntimeHook::U128Eq,
    RuntimeHook::I128Cmp,
    RuntimeHook::U128Cmp,
    RuntimeHook::I128Neg,
    RuntimeHook::I128Not,
    RuntimeHook::U128Not,
    RuntimeHook::I128And,
    RuntimeHook::U128And,
    RuntimeHook::I128Or,
    RuntimeHook::U128Or,
    RuntimeHook::I128Xor,
    RuntimeHook::U128Xor,
    RuntimeHook::I128Shl,
    RuntimeHook::U128Shl,
    RuntimeHook::I128Shr,
    RuntimeHook::U128Shr,
    RuntimeHook::SpanCopyTo,
    RuntimeHook::SpanFromRawMut,
    RuntimeHook::SpanFromRawConst,
    RuntimeHook::SpanSliceMut,
    RuntimeHook::SpanSliceReadonly,
    RuntimeHook::SpanToReadonly,
    RuntimeHook::SpanPtrAtMut,
    RuntimeHook::SpanPtrAtReadonly,
    RuntimeHook::MmioRead,
    RuntimeHook::MmioWrite,
    RuntimeHook::DecimalAdd,
    RuntimeHook::DecimalAddSimd,
    RuntimeHook::DecimalSub,
    RuntimeHook::DecimalSubSimd,
    RuntimeHook::DecimalMul,
    RuntimeHook::DecimalMulSimd,
    RuntimeHook::DecimalDiv,
    RuntimeHook::DecimalDivSimd,
    RuntimeHook::DecimalRem,
    RuntimeHook::DecimalRemSimd,
    RuntimeHook::DecimalFma,
    RuntimeHook::DecimalFmaSimd,
    RuntimeHook::DecimalSum,
    RuntimeHook::DecimalDot,
    RuntimeHook::DecimalMatMul,
    RuntimeHook::ClosureEnvAlloc,
    RuntimeHook::ClosureEnvClone,
    RuntimeHook::ClosureEnvFree,
    RuntimeHook::DropInvoke,
    RuntimeHook::HashInvoke,
    RuntimeHook::EqInvoke,
];

impl RuntimeHook {
    pub(crate) const fn module(self) -> &'static str {
        "chic_rt"
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            RuntimeHook::ObjectNew => "object_new",
            RuntimeHook::Panic => "panic",
            RuntimeHook::Abort => "abort",
            RuntimeHook::Throw => "throw",
            RuntimeHook::HasPendingException => "has_pending_exception",
            RuntimeHook::TakePendingException => "take_pending_exception",
            RuntimeHook::Await => "await",
            RuntimeHook::Yield => "yield",
            RuntimeHook::AsyncCancel => "async_cancel",
            RuntimeHook::AsyncSpawn => "async_spawn",
            RuntimeHook::AsyncSpawnLocal => "async_spawn_local",
            RuntimeHook::AsyncScope => "async_scope",
            RuntimeHook::AsyncTaskHeader => "async_task_header",
            RuntimeHook::AsyncTaskResult => "async_task_result",
            RuntimeHook::AsyncTokenState => "async_token_state",
            RuntimeHook::AsyncTokenNew => "async_token_new",
            RuntimeHook::AsyncTokenCancel => "async_token_cancel",
            RuntimeHook::BorrowShared => "borrow_shared",
            RuntimeHook::BorrowUnique => "borrow_unique",
            RuntimeHook::BorrowRelease => "borrow_release",
            RuntimeHook::DropResource => "drop_resource",
            RuntimeHook::DropMissing => "drop_missing",
            RuntimeHook::TypeSize => "type_size",
            RuntimeHook::TypeAlign => "type_align",
            RuntimeHook::TypeMetadata => "type_metadata",
            RuntimeHook::TypeDropGlue => "type_drop_glue",
            RuntimeHook::TypeCloneGlue => "type_clone_glue",
            RuntimeHook::TypeHashGlue => "type_hash_glue",
            RuntimeHook::TypeEqGlue => "type_eq_glue",
            RuntimeHook::DropInvoke => "drop_invoke",
            RuntimeHook::HashInvoke => "hash_invoke",
            RuntimeHook::EqInvoke => "eq_invoke",
            RuntimeHook::TraceEnter => "trace_enter",
            RuntimeHook::TraceExit => "trace_exit",
            RuntimeHook::TraceFlush => "trace_flush",
            RuntimeHook::CoverageHit => "coverage_hit",
            RuntimeHook::Alloc => "alloc",
            RuntimeHook::AllocZeroed => "alloc_zeroed",
            RuntimeHook::Realloc => "realloc",
            RuntimeHook::Free => "free",
            RuntimeHook::Memcpy => "memcpy",
            RuntimeHook::Memmove => "memmove",
            RuntimeHook::Memset => "memset",
            RuntimeHook::MmioRead => "mmio_read",
            RuntimeHook::MmioWrite => "mmio_write",
            RuntimeHook::StringFromSlice => "string_from_slice",
            RuntimeHook::StringClone => "string_clone",
            RuntimeHook::StringCloneSlice => "string_clone_slice",
            RuntimeHook::StringDrop => "string_drop",
            RuntimeHook::VecWithCapacity => "vec_with_capacity",
            RuntimeHook::VecClone => "vec_clone",
            RuntimeHook::VecIntoArray => "vec_into_array",
            RuntimeHook::VecCopyToArray => "vec_copy_to_array",
            RuntimeHook::VecDrop => "vec_drop",
            RuntimeHook::ArrayIntoVec => "array_into_vec",
            RuntimeHook::ArrayCopyToVec => "array_copy_to_vec",
            RuntimeHook::HashSetNew => "hashset_new",
            RuntimeHook::HashSetWithCapacity => "hashset_with_capacity",
            RuntimeHook::HashSetDrop => "hashset_drop",
            RuntimeHook::HashSetClear => "hashset_clear",
            RuntimeHook::HashSetReserve => "hashset_reserve",
            RuntimeHook::HashSetShrinkTo => "hashset_shrink_to",
            RuntimeHook::HashSetLen => "hashset_len",
            RuntimeHook::HashSetCapacity => "hashset_capacity",
            RuntimeHook::HashSetTombstones => "hashset_tombstones",
            RuntimeHook::HashSetInsert => "hashset_insert",
            RuntimeHook::HashSetReplace => "hashset_replace",
            RuntimeHook::HashSetContains => "hashset_contains",
            RuntimeHook::HashSetGetPtr => "hashset_get_ptr",
            RuntimeHook::HashSetTake => "hashset_take",
            RuntimeHook::HashSetRemove => "hashset_remove",
            RuntimeHook::HashSetTakeAt => "hashset_take_at",
            RuntimeHook::HashSetBucketState => "hashset_bucket_state",
            RuntimeHook::HashSetBucketHash => "hashset_bucket_hash",
            RuntimeHook::HashSetIter => "hashset_iter",
            RuntimeHook::HashSetIterNext => "hashset_iter_next",
            RuntimeHook::HashSetIterNextPtr => "hashset_iter_next_ptr",
            RuntimeHook::HashMapNew => "hashmap_new",
            RuntimeHook::HashMapWithCapacity => "hashmap_with_capacity",
            RuntimeHook::HashMapDrop => "hashmap_drop",
            RuntimeHook::HashMapClear => "hashmap_clear",
            RuntimeHook::HashMapReserve => "hashmap_reserve",
            RuntimeHook::HashMapShrinkTo => "hashmap_shrink_to",
            RuntimeHook::HashMapLen => "hashmap_len",
            RuntimeHook::HashMapCapacity => "hashmap_capacity",
            RuntimeHook::HashMapInsert => "hashmap_insert",
            RuntimeHook::HashMapContains => "hashmap_contains",
            RuntimeHook::HashMapGetPtr => "hashmap_get_ptr",
            RuntimeHook::HashMapTake => "hashmap_take",
            RuntimeHook::HashMapRemove => "hashmap_remove",
            RuntimeHook::HashMapBucketState => "hashmap_bucket_state",
            RuntimeHook::HashMapBucketHash => "hashmap_bucket_hash",
            RuntimeHook::HashMapTakeAt => "hashmap_take_at",
            RuntimeHook::HashMapIter => "hashmap_iter",
            RuntimeHook::HashMapIterNext => "hashmap_iter_next",
            RuntimeHook::RcClone => "rc_clone",
            RuntimeHook::RcDrop => "rc_drop",
            RuntimeHook::ArcNew => "arc_new",
            RuntimeHook::ArcClone => "arc_clone",
            RuntimeHook::ArcDrop => "arc_drop",
            RuntimeHook::ArcGet => "arc_get",
            RuntimeHook::ArcGetMut => "arc_get_mut",
            RuntimeHook::ArcDowngrade => "arc_downgrade",
            RuntimeHook::WeakClone => "weak_clone",
            RuntimeHook::WeakDrop => "weak_drop",
            RuntimeHook::WeakUpgrade => "weak_upgrade",
            RuntimeHook::ArcStrongCount => "arc_strong_count",
            RuntimeHook::ArcWeakCount => "arc_weak_count",
            RuntimeHook::StringAppendSlice => "string_append_slice",
            RuntimeHook::StringAppendBool => "string_append_bool",
            RuntimeHook::StringAppendChar => "string_append_char",
            RuntimeHook::StringAppendSigned => "string_append_signed",
            RuntimeHook::StringAppendUnsigned => "string_append_unsigned",
            RuntimeHook::StringAppendF32 => "string_append_f32",
            RuntimeHook::StringAppendF64 => "string_append_f64",
            RuntimeHook::StringAsSlice => "string_as_slice",
            RuntimeHook::StringTryCopyUtf8 => "string_try_copy_utf8",
            RuntimeHook::StringAsChars => "string_as_chars",
            RuntimeHook::StrAsChars => "str_as_chars",
            RuntimeHook::F32Rem => "f32_rem",
            RuntimeHook::F64Rem => "f64_rem",
            RuntimeHook::MathAbsF64 => "math_abs_f64",
            RuntimeHook::MathFloorF64 => "math_floor_f64",
            RuntimeHook::MathCeilF64 => "math_ceil_f64",
            RuntimeHook::MathTruncF64 => "math_trunc_f64",
            RuntimeHook::MathCopySignF64 => "math_copy_sign_f64",
            RuntimeHook::MathBitIncrementF64 => "math_bit_increment_f64",
            RuntimeHook::MathBitDecrementF64 => "math_bit_decrement_f64",
            RuntimeHook::MathScaleBF64 => "math_scale_b_f64",
            RuntimeHook::MathILogBF64 => "math_ilogb_f64",
            RuntimeHook::MathIeeeRemainderF64 => "math_ieee_remainder_f64",
            RuntimeHook::MathFmaF64 => "math_fma_f64",
            RuntimeHook::MathCbrtF64 => "math_cbrt_f64",
            RuntimeHook::MathSqrtF64 => "math_sqrt_f64",
            RuntimeHook::MathPowF64 => "math_pow_f64",
            RuntimeHook::MathSinF64 => "math_sin_f64",
            RuntimeHook::MathCosF64 => "math_cos_f64",
            RuntimeHook::MathTanF64 => "math_tan_f64",
            RuntimeHook::MathAsinF64 => "math_asin_f64",
            RuntimeHook::MathAcosF64 => "math_acos_f64",
            RuntimeHook::MathAtanF64 => "math_atan_f64",
            RuntimeHook::MathAtan2F64 => "math_atan2_f64",
            RuntimeHook::MathSinhF64 => "math_sinh_f64",
            RuntimeHook::MathCoshF64 => "math_cosh_f64",
            RuntimeHook::MathTanhF64 => "math_tanh_f64",
            RuntimeHook::MathAsinhF64 => "math_asinh_f64",
            RuntimeHook::MathAcoshF64 => "math_acosh_f64",
            RuntimeHook::MathAtanhF64 => "math_atanh_f64",
            RuntimeHook::MathExpF64 => "math_exp_f64",
            RuntimeHook::MathLogF64 => "math_log_f64",
            RuntimeHook::MathLog10F64 => "math_log10_f64",
            RuntimeHook::MathLog2F64 => "math_log2_f64",
            RuntimeHook::MathRoundF64 => "math_round_f64",
            RuntimeHook::MathAbsF32 => "math_abs_f32",
            RuntimeHook::MathFloorF32 => "math_floor_f32",
            RuntimeHook::MathCeilF32 => "math_ceil_f32",
            RuntimeHook::MathTruncF32 => "math_trunc_f32",
            RuntimeHook::MathCopySignF32 => "math_copy_sign_f32",
            RuntimeHook::MathBitIncrementF32 => "math_bit_increment_f32",
            RuntimeHook::MathBitDecrementF32 => "math_bit_decrement_f32",
            RuntimeHook::MathScaleBF32 => "math_scale_b_f32",
            RuntimeHook::MathILogBF32 => "math_ilogb_f32",
            RuntimeHook::MathIeeeRemainderF32 => "math_ieee_remainder_f32",
            RuntimeHook::MathFmaF32 => "math_fma_f32",
            RuntimeHook::MathCbrtF32 => "math_cbrt_f32",
            RuntimeHook::MathSqrtF32 => "math_sqrt_f32",
            RuntimeHook::MathPowF32 => "math_pow_f32",
            RuntimeHook::MathSinF32 => "math_sin_f32",
            RuntimeHook::MathCosF32 => "math_cos_f32",
            RuntimeHook::MathTanF32 => "math_tan_f32",
            RuntimeHook::MathAsinF32 => "math_asin_f32",
            RuntimeHook::MathAcosF32 => "math_acos_f32",
            RuntimeHook::MathAtanF32 => "math_atan_f32",
            RuntimeHook::MathAtan2F32 => "math_atan2_f32",
            RuntimeHook::MathSinhF32 => "math_sinh_f32",
            RuntimeHook::MathCoshF32 => "math_cosh_f32",
            RuntimeHook::MathTanhF32 => "math_tanh_f32",
            RuntimeHook::MathAsinhF32 => "math_asinh_f32",
            RuntimeHook::MathAcoshF32 => "math_acosh_f32",
            RuntimeHook::MathAtanhF32 => "math_atanh_f32",
            RuntimeHook::MathExpF32 => "math_exp_f32",
            RuntimeHook::MathLogF32 => "math_log_f32",
            RuntimeHook::MathLog10F32 => "math_log10_f32",
            RuntimeHook::MathLog2F32 => "math_log2_f32",
            RuntimeHook::MathRoundF32 => "math_round_f32",
            RuntimeHook::I128Add => "i128_add",
            RuntimeHook::U128Add => "u128_add",
            RuntimeHook::I128Sub => "i128_sub",
            RuntimeHook::U128Sub => "u128_sub",
            RuntimeHook::I128Mul => "i128_mul",
            RuntimeHook::U128Mul => "u128_mul",
            RuntimeHook::I128Div => "i128_div",
            RuntimeHook::U128Div => "u128_div",
            RuntimeHook::I128Rem => "i128_rem",
            RuntimeHook::U128Rem => "u128_rem",
            RuntimeHook::I128Eq => "i128_eq",
            RuntimeHook::U128Eq => "u128_eq",
            RuntimeHook::I128Cmp => "i128_cmp",
            RuntimeHook::U128Cmp => "u128_cmp",
            RuntimeHook::I128Neg => "i128_neg",
            RuntimeHook::I128Not => "i128_not",
            RuntimeHook::U128Not => "u128_not",
            RuntimeHook::I128And => "i128_and",
            RuntimeHook::U128And => "u128_and",
            RuntimeHook::I128Or => "i128_or",
            RuntimeHook::U128Or => "u128_or",
            RuntimeHook::I128Xor => "i128_xor",
            RuntimeHook::U128Xor => "u128_xor",
            RuntimeHook::I128Shl => "i128_shl",
            RuntimeHook::U128Shl => "u128_shl",
            RuntimeHook::I128Shr => "i128_shr",
            RuntimeHook::U128Shr => "u128_shr",
            RuntimeHook::SpanCopyTo => "span_copy_to",
            RuntimeHook::SpanFromRawMut => "span_from_raw_mut",
            RuntimeHook::SpanFromRawConst => "span_from_raw_const",
            RuntimeHook::SpanSliceMut => "span_slice_mut",
            RuntimeHook::SpanSliceReadonly => "span_slice_readonly",
            RuntimeHook::SpanToReadonly => "span_to_readonly",
            RuntimeHook::SpanPtrAtMut => "span_ptr_at_mut",
            RuntimeHook::SpanPtrAtReadonly => "span_ptr_at_readonly",
            RuntimeHook::DecimalAdd => "decimal_add_out",
            RuntimeHook::DecimalAddSimd => "decimal_add_simd_out",
            RuntimeHook::DecimalSub => "decimal_sub_out",
            RuntimeHook::DecimalSubSimd => "decimal_sub_simd_out",
            RuntimeHook::DecimalMul => "decimal_mul_out",
            RuntimeHook::DecimalMulSimd => "decimal_mul_simd_out",
            RuntimeHook::DecimalDiv => "decimal_div_out",
            RuntimeHook::DecimalDivSimd => "decimal_div_simd_out",
            RuntimeHook::DecimalRem => "decimal_rem_out",
            RuntimeHook::DecimalRemSimd => "decimal_rem_simd_out",
            RuntimeHook::DecimalFma => "decimal_fma_out",
            RuntimeHook::DecimalFmaSimd => "decimal_fma_simd_out",
            RuntimeHook::DecimalSum => "decimal_sum_out",
            RuntimeHook::DecimalDot => "decimal_dot_out",
            RuntimeHook::DecimalMatMul => "decimal_matmul",
            RuntimeHook::ClosureEnvAlloc => "closure_env_alloc",
            RuntimeHook::ClosureEnvClone => "closure_env_clone",
            RuntimeHook::ClosureEnvFree => "closure_env_free",
        }
    }

    pub(super) fn signature(self) -> FunctionSignature {
        let params = match self {
            RuntimeHook::ObjectNew => vec![ValueType::I64],
            RuntimeHook::Panic => vec![ValueType::I32],
            RuntimeHook::Abort => vec![ValueType::I32],
            RuntimeHook::Throw => vec![ValueType::I32, ValueType::I64],
            RuntimeHook::HasPendingException => Vec::new(),
            RuntimeHook::TakePendingException => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::Await => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::Yield => vec![ValueType::I32],
            RuntimeHook::AsyncCancel => vec![ValueType::I32],
            RuntimeHook::AsyncSpawn => vec![ValueType::I32],
            RuntimeHook::AsyncSpawnLocal => vec![ValueType::I32],
            RuntimeHook::AsyncScope => vec![ValueType::I32],
            RuntimeHook::AsyncTaskHeader => vec![ValueType::I32],
            RuntimeHook::AsyncTaskResult => vec![ValueType::I32, ValueType::I32, ValueType::I32],
            RuntimeHook::AsyncTokenState => vec![ValueType::I32],
            RuntimeHook::AsyncTokenNew => Vec::new(),
            RuntimeHook::AsyncTokenCancel => vec![ValueType::I32],
            RuntimeHook::BorrowShared => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::BorrowUnique => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::BorrowRelease => vec![ValueType::I32],
            RuntimeHook::DropResource => vec![ValueType::I32],
            RuntimeHook::DropMissing => vec![ValueType::I32],
            RuntimeHook::TypeSize
            | RuntimeHook::TypeAlign
            | RuntimeHook::TypeDropGlue
            | RuntimeHook::TypeCloneGlue
            | RuntimeHook::TypeHashGlue
            | RuntimeHook::TypeEqGlue => vec![ValueType::I64],
            RuntimeHook::TypeMetadata => vec![ValueType::I64, ValueType::I32],
            RuntimeHook::DropInvoke => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::HashInvoke => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::EqInvoke => vec![ValueType::I32, ValueType::I32, ValueType::I32],
            RuntimeHook::TraceEnter => vec![
                ValueType::I64,
                ValueType::I32,
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
                ValueType::I64,
            ],
            RuntimeHook::TraceExit => vec![ValueType::I64],
            RuntimeHook::TraceFlush => vec![ValueType::I32, ValueType::I64],
            RuntimeHook::CoverageHit => vec![ValueType::I64],
            RuntimeHook::Alloc | RuntimeHook::AllocZeroed => {
                vec![ValueType::I32, ValueType::I32, ValueType::I32]
            }
            RuntimeHook::Realloc => vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::Free => vec![ValueType::I32],
            RuntimeHook::Memcpy | RuntimeHook::Memmove | RuntimeHook::Memset => {
                vec![ValueType::I32, ValueType::I32, ValueType::I32]
            }
            RuntimeHook::MmioRead => vec![ValueType::I64, ValueType::I32, ValueType::I32],
            RuntimeHook::MmioWrite => {
                vec![
                    ValueType::I64,
                    ValueType::I64,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::StringFromSlice => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::StringClone => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::StringCloneSlice => {
                vec![ValueType::I32, ValueType::I32, ValueType::I32]
            }
            RuntimeHook::StringDrop => vec![ValueType::I32],
            RuntimeHook::VecWithCapacity => vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::VecClone => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::VecIntoArray | RuntimeHook::VecCopyToArray => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::VecDrop => vec![ValueType::I32],
            RuntimeHook::ArrayIntoVec | RuntimeHook::ArrayCopyToVec => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::HashSetNew => vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::HashSetWithCapacity => vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::HashSetDrop
            | RuntimeHook::HashSetClear
            | RuntimeHook::HashSetLen
            | RuntimeHook::HashSetCapacity
            | RuntimeHook::HashSetTombstones => vec![ValueType::I32],
            RuntimeHook::HashSetReserve | RuntimeHook::HashSetShrinkTo => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::HashSetInsert | RuntimeHook::HashSetTake => {
                vec![
                    ValueType::I32,
                    ValueType::I64,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::HashSetReplace => vec![
                ValueType::I32,
                ValueType::I64,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::HashSetContains | RuntimeHook::HashSetRemove => {
                vec![ValueType::I32, ValueType::I64, ValueType::I32]
            }
            RuntimeHook::HashSetGetPtr => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I64,
                    ValueType::I32,
                ]
            }
            RuntimeHook::HashSetTakeAt => vec![ValueType::I32, ValueType::I32, ValueType::I32],
            RuntimeHook::HashSetBucketState => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::HashSetBucketHash => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::HashSetIter | RuntimeHook::HashSetIterNext => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::HashSetIterNextPtr => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::HashMapNew => vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::HashMapWithCapacity => vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::HashMapDrop
            | RuntimeHook::HashMapClear
            | RuntimeHook::HashMapLen
            | RuntimeHook::HashMapCapacity => vec![ValueType::I32],
            RuntimeHook::HashMapReserve | RuntimeHook::HashMapShrinkTo => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::HashMapInsert => vec![
                ValueType::I32,
                ValueType::I64,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::HashMapContains | RuntimeHook::HashMapRemove => {
                vec![ValueType::I32, ValueType::I64, ValueType::I32]
            }
            RuntimeHook::HashMapGetPtr => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I64,
                    ValueType::I32,
                ]
            }
            RuntimeHook::HashMapTake => {
                vec![
                    ValueType::I32,
                    ValueType::I64,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::HashMapBucketState => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::HashMapBucketHash => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::HashMapTakeAt => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::HashMapIter => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::HashMapIterNext => vec![ValueType::I32, ValueType::I32, ValueType::I32],
            RuntimeHook::RcClone => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::RcDrop => vec![ValueType::I32],
            RuntimeHook::ArcNew => vec![
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I64,
            ],
            RuntimeHook::ArcClone => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::ArcDrop => vec![ValueType::I32],
            RuntimeHook::ArcGet | RuntimeHook::ArcGetMut => vec![ValueType::I32],
            RuntimeHook::ArcDowngrade | RuntimeHook::WeakClone | RuntimeHook::WeakUpgrade => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::WeakDrop => vec![ValueType::I32],
            RuntimeHook::ArcStrongCount | RuntimeHook::ArcWeakCount => vec![ValueType::I32],
            RuntimeHook::StringAppendSlice => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::StringAppendBool => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::StringAppendChar => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::StringAppendSigned | RuntimeHook::StringAppendUnsigned => {
                vec![
                    ValueType::I32,
                    ValueType::I64,
                    ValueType::I64,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::StringAppendF32 => vec![
                ValueType::I32,
                ValueType::F32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::StringAppendF64 => vec![
                ValueType::I32,
                ValueType::F64,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
                ValueType::I32,
            ],
            RuntimeHook::StringAsSlice => vec![ValueType::I32],
            RuntimeHook::StringTryCopyUtf8 => vec![ValueType::I32, ValueType::I32, ValueType::I32],
            RuntimeHook::StringAsChars => vec![ValueType::I32],
            RuntimeHook::StrAsChars => vec![ValueType::I32],
            RuntimeHook::F32Rem => vec![ValueType::F32, ValueType::F32],
            RuntimeHook::F64Rem => vec![ValueType::F64, ValueType::F64],
            RuntimeHook::MathAbsF64
            | RuntimeHook::MathFloorF64
            | RuntimeHook::MathCeilF64
            | RuntimeHook::MathTruncF64
            | RuntimeHook::MathBitIncrementF64
            | RuntimeHook::MathBitDecrementF64
            | RuntimeHook::MathCbrtF64
            | RuntimeHook::MathSqrtF64
            | RuntimeHook::MathSinF64
            | RuntimeHook::MathCosF64
            | RuntimeHook::MathTanF64
            | RuntimeHook::MathAsinF64
            | RuntimeHook::MathAcosF64
            | RuntimeHook::MathAtanF64
            | RuntimeHook::MathSinhF64
            | RuntimeHook::MathCoshF64
            | RuntimeHook::MathTanhF64
            | RuntimeHook::MathAsinhF64
            | RuntimeHook::MathAcoshF64
            | RuntimeHook::MathAtanhF64
            | RuntimeHook::MathExpF64
            | RuntimeHook::MathLogF64
            | RuntimeHook::MathLog10F64
            | RuntimeHook::MathLog2F64 => vec![ValueType::F64],
            RuntimeHook::MathCopySignF64
            | RuntimeHook::MathIeeeRemainderF64
            | RuntimeHook::MathPowF64
            | RuntimeHook::MathAtan2F64 => vec![ValueType::F64, ValueType::F64],
            RuntimeHook::MathScaleBF64 => vec![ValueType::F64, ValueType::I32],
            RuntimeHook::MathILogBF64 => vec![ValueType::F64],
            RuntimeHook::MathFmaF64 => vec![ValueType::F64, ValueType::F64, ValueType::F64],
            RuntimeHook::MathRoundF64 => vec![ValueType::F64, ValueType::I32, ValueType::I32],
            RuntimeHook::MathAbsF32
            | RuntimeHook::MathFloorF32
            | RuntimeHook::MathCeilF32
            | RuntimeHook::MathTruncF32
            | RuntimeHook::MathBitIncrementF32
            | RuntimeHook::MathBitDecrementF32
            | RuntimeHook::MathCbrtF32
            | RuntimeHook::MathSqrtF32
            | RuntimeHook::MathSinF32
            | RuntimeHook::MathCosF32
            | RuntimeHook::MathTanF32
            | RuntimeHook::MathAsinF32
            | RuntimeHook::MathAcosF32
            | RuntimeHook::MathAtanF32
            | RuntimeHook::MathSinhF32
            | RuntimeHook::MathCoshF32
            | RuntimeHook::MathTanhF32
            | RuntimeHook::MathAsinhF32
            | RuntimeHook::MathAcoshF32
            | RuntimeHook::MathAtanhF32
            | RuntimeHook::MathExpF32
            | RuntimeHook::MathLogF32
            | RuntimeHook::MathLog10F32
            | RuntimeHook::MathLog2F32 => vec![ValueType::F32],
            RuntimeHook::MathCopySignF32
            | RuntimeHook::MathIeeeRemainderF32
            | RuntimeHook::MathPowF32
            | RuntimeHook::MathAtan2F32 => vec![ValueType::F32, ValueType::F32],
            RuntimeHook::MathScaleBF32 => vec![ValueType::F32, ValueType::I32],
            RuntimeHook::MathILogBF32 => vec![ValueType::F32],
            RuntimeHook::MathFmaF32 => vec![ValueType::F32, ValueType::F32, ValueType::F32],
            RuntimeHook::MathRoundF32 => vec![ValueType::F32, ValueType::I32, ValueType::I32],
            RuntimeHook::I128Add
            | RuntimeHook::U128Add
            | RuntimeHook::I128Sub
            | RuntimeHook::U128Sub
            | RuntimeHook::I128Mul
            | RuntimeHook::U128Mul
            | RuntimeHook::I128Div
            | RuntimeHook::U128Div
            | RuntimeHook::I128Rem
            | RuntimeHook::U128Rem
            | RuntimeHook::I128And
            | RuntimeHook::U128And
            | RuntimeHook::I128Or
            | RuntimeHook::U128Or
            | RuntimeHook::I128Xor
            | RuntimeHook::U128Xor => vec![ValueType::I32, ValueType::I32, ValueType::I32],
            RuntimeHook::I128Eq
            | RuntimeHook::U128Eq
            | RuntimeHook::I128Cmp
            | RuntimeHook::U128Cmp => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::I128Neg | RuntimeHook::I128Not | RuntimeHook::U128Not => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::I128Shl
            | RuntimeHook::U128Shl
            | RuntimeHook::I128Shr
            | RuntimeHook::U128Shr => {
                vec![ValueType::I32, ValueType::I32, ValueType::I32]
            }
            RuntimeHook::SpanCopyTo => vec![ValueType::I32; 8],
            RuntimeHook::SpanFromRawMut | RuntimeHook::SpanFromRawConst => {
                vec![ValueType::I32, ValueType::I32, ValueType::I32]
            }
            RuntimeHook::SpanSliceMut | RuntimeHook::SpanSliceReadonly => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::SpanToReadonly => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::SpanPtrAtMut | RuntimeHook::SpanPtrAtReadonly => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::DecimalAdd
            | RuntimeHook::DecimalAddSimd
            | RuntimeHook::DecimalSub
            | RuntimeHook::DecimalSubSimd
            | RuntimeHook::DecimalMul
            | RuntimeHook::DecimalMulSimd
            | RuntimeHook::DecimalDiv
            | RuntimeHook::DecimalDivSimd
            | RuntimeHook::DecimalRem
            | RuntimeHook::DecimalRemSimd
            | RuntimeHook::DecimalSum => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::DecimalFma | RuntimeHook::DecimalFmaSimd => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::DecimalDot => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::DecimalMatMul => {
                vec![
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                    ValueType::I32,
                ]
            }
            RuntimeHook::ClosureEnvAlloc => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::ClosureEnvClone => vec![ValueType::I32, ValueType::I32, ValueType::I32],
            RuntimeHook::ClosureEnvFree => vec![ValueType::I32, ValueType::I32, ValueType::I32],
        };
        let results = match self {
            RuntimeHook::ObjectNew => vec![ValueType::I32],
            RuntimeHook::Await => vec![ValueType::I32],
            RuntimeHook::Yield => vec![ValueType::I32],
            RuntimeHook::AsyncCancel => vec![ValueType::I32],
            RuntimeHook::AsyncSpawn => vec![ValueType::I32],
            RuntimeHook::AsyncSpawnLocal => vec![ValueType::I32],
            RuntimeHook::AsyncScope => vec![ValueType::I32],
            RuntimeHook::AsyncTaskHeader => vec![ValueType::I32],
            RuntimeHook::AsyncTaskResult => vec![ValueType::I32],
            RuntimeHook::AsyncTokenState => vec![ValueType::I32],
            RuntimeHook::AsyncTokenNew => vec![ValueType::I32],
            RuntimeHook::AsyncTokenCancel => vec![ValueType::I32],
            RuntimeHook::MmioRead => vec![ValueType::I64],
            RuntimeHook::StringFromSlice
            | RuntimeHook::StringClone
            | RuntimeHook::StringCloneSlice => vec![ValueType::I32],
            RuntimeHook::VecWithCapacity => vec![ValueType::I32],
            RuntimeHook::VecClone
            | RuntimeHook::VecIntoArray
            | RuntimeHook::VecCopyToArray
            | RuntimeHook::ArrayIntoVec
            | RuntimeHook::ArrayCopyToVec => vec![ValueType::I32],
            RuntimeHook::HashSetNew
            | RuntimeHook::HashSetWithCapacity
            | RuntimeHook::HashSetClear
            | RuntimeHook::HashSetReserve
            | RuntimeHook::HashSetShrinkTo
            | RuntimeHook::HashSetLen
            | RuntimeHook::HashSetCapacity
            | RuntimeHook::HashSetTombstones
            | RuntimeHook::HashSetInsert
            | RuntimeHook::HashSetReplace
            | RuntimeHook::HashSetContains
            | RuntimeHook::HashSetGetPtr
            | RuntimeHook::HashSetTake
            | RuntimeHook::HashSetRemove
            | RuntimeHook::HashSetTakeAt
            | RuntimeHook::HashSetBucketState
            | RuntimeHook::HashSetIter
            | RuntimeHook::HashSetIterNext
            | RuntimeHook::HashSetIterNextPtr
            | RuntimeHook::HashMapNew
            | RuntimeHook::HashMapWithCapacity
            | RuntimeHook::HashMapClear
            | RuntimeHook::HashMapReserve
            | RuntimeHook::HashMapShrinkTo
            | RuntimeHook::HashMapLen
            | RuntimeHook::HashMapCapacity
            | RuntimeHook::HashMapInsert
            | RuntimeHook::HashMapContains
            | RuntimeHook::HashMapGetPtr
            | RuntimeHook::HashMapTake
            | RuntimeHook::HashMapRemove
            | RuntimeHook::HashMapBucketState
            | RuntimeHook::HashMapTakeAt
            | RuntimeHook::HashMapIter
            | RuntimeHook::HashMapIterNext => vec![ValueType::I32],
            RuntimeHook::HashSetBucketHash | RuntimeHook::HashMapBucketHash => vec![ValueType::I64],
            RuntimeHook::RcClone
            | RuntimeHook::ArcClone
            | RuntimeHook::ArcNew
            | RuntimeHook::ArcDowngrade
            | RuntimeHook::WeakClone
            | RuntimeHook::WeakUpgrade
            | RuntimeHook::ArcStrongCount
            | RuntimeHook::ArcWeakCount => vec![ValueType::I32],
            RuntimeHook::ArcGet | RuntimeHook::ArcGetMut => vec![ValueType::I32],
            RuntimeHook::WeakDrop => Vec::new(),
            RuntimeHook::SpanCopyTo => vec![ValueType::I32],
            RuntimeHook::SpanFromRawMut
            | RuntimeHook::SpanFromRawConst
            | RuntimeHook::SpanToReadonly => vec![ValueType::I32],
            RuntimeHook::SpanSliceMut
            | RuntimeHook::SpanSliceReadonly
            | RuntimeHook::SpanPtrAtMut
            | RuntimeHook::SpanPtrAtReadonly => vec![ValueType::I32],
            RuntimeHook::StringAppendSlice
            | RuntimeHook::StringAppendBool
            | RuntimeHook::StringAppendChar
            | RuntimeHook::StringAppendSigned
            | RuntimeHook::StringAppendUnsigned
            | RuntimeHook::StringAppendF32
            | RuntimeHook::StringAppendF64 => vec![ValueType::I32],
            RuntimeHook::StringAsSlice => vec![ValueType::I32, ValueType::I32],
            RuntimeHook::StringTryCopyUtf8 => vec![ValueType::I32],
            RuntimeHook::StringAsChars | RuntimeHook::StrAsChars => {
                vec![ValueType::I32, ValueType::I32]
            }
            RuntimeHook::F32Rem => vec![ValueType::F32],
            RuntimeHook::F64Rem => vec![ValueType::F64],
            RuntimeHook::MathAbsF64
            | RuntimeHook::MathFloorF64
            | RuntimeHook::MathCeilF64
            | RuntimeHook::MathTruncF64
            | RuntimeHook::MathCopySignF64
            | RuntimeHook::MathBitIncrementF64
            | RuntimeHook::MathBitDecrementF64
            | RuntimeHook::MathScaleBF64
            | RuntimeHook::MathIeeeRemainderF64
            | RuntimeHook::MathFmaF64
            | RuntimeHook::MathCbrtF64
            | RuntimeHook::MathSqrtF64
            | RuntimeHook::MathPowF64
            | RuntimeHook::MathSinF64
            | RuntimeHook::MathCosF64
            | RuntimeHook::MathTanF64
            | RuntimeHook::MathAsinF64
            | RuntimeHook::MathAcosF64
            | RuntimeHook::MathAtanF64
            | RuntimeHook::MathAtan2F64
            | RuntimeHook::MathSinhF64
            | RuntimeHook::MathCoshF64
            | RuntimeHook::MathTanhF64
            | RuntimeHook::MathAsinhF64
            | RuntimeHook::MathAcoshF64
            | RuntimeHook::MathAtanhF64
            | RuntimeHook::MathExpF64
            | RuntimeHook::MathLogF64
            | RuntimeHook::MathLog10F64
            | RuntimeHook::MathLog2F64
            | RuntimeHook::MathRoundF64 => vec![ValueType::F64],
            RuntimeHook::MathAbsF32
            | RuntimeHook::MathFloorF32
            | RuntimeHook::MathCeilF32
            | RuntimeHook::MathTruncF32
            | RuntimeHook::MathCopySignF32
            | RuntimeHook::MathBitIncrementF32
            | RuntimeHook::MathBitDecrementF32
            | RuntimeHook::MathScaleBF32
            | RuntimeHook::MathIeeeRemainderF32
            | RuntimeHook::MathFmaF32
            | RuntimeHook::MathCbrtF32
            | RuntimeHook::MathSqrtF32
            | RuntimeHook::MathPowF32
            | RuntimeHook::MathSinF32
            | RuntimeHook::MathCosF32
            | RuntimeHook::MathTanF32
            | RuntimeHook::MathAsinF32
            | RuntimeHook::MathAcosF32
            | RuntimeHook::MathAtanF32
            | RuntimeHook::MathAtan2F32
            | RuntimeHook::MathSinhF32
            | RuntimeHook::MathCoshF32
            | RuntimeHook::MathTanhF32
            | RuntimeHook::MathAsinhF32
            | RuntimeHook::MathAcoshF32
            | RuntimeHook::MathAtanhF32
            | RuntimeHook::MathExpF32
            | RuntimeHook::MathLogF32
            | RuntimeHook::MathLog10F32
            | RuntimeHook::MathLog2F32
            | RuntimeHook::MathRoundF32 => vec![ValueType::F32],
            RuntimeHook::MathILogBF64 | RuntimeHook::MathILogBF32 => vec![ValueType::I32],
            RuntimeHook::I128Eq
            | RuntimeHook::U128Eq
            | RuntimeHook::I128Cmp
            | RuntimeHook::U128Cmp => vec![ValueType::I32],
            RuntimeHook::DecimalMatMul => vec![ValueType::I32],
            RuntimeHook::TraceFlush => vec![ValueType::I32],
            RuntimeHook::Alloc | RuntimeHook::AllocZeroed | RuntimeHook::Realloc => {
                vec![ValueType::I32]
            }
            RuntimeHook::Free
            | RuntimeHook::Memcpy
            | RuntimeHook::Memmove
            | RuntimeHook::Memset => Vec::new(),
            RuntimeHook::TypeSize | RuntimeHook::TypeAlign => vec![ValueType::I32],
            RuntimeHook::TypeMetadata => vec![ValueType::I32],
            RuntimeHook::TypeDropGlue
            | RuntimeHook::TypeCloneGlue
            | RuntimeHook::TypeHashGlue
            | RuntimeHook::TypeEqGlue => vec![ValueType::I32],
            RuntimeHook::HashInvoke => vec![ValueType::I64],
            RuntimeHook::EqInvoke => vec![ValueType::I32],
            RuntimeHook::ClosureEnvAlloc | RuntimeHook::ClosureEnvClone => vec![ValueType::I32],
            RuntimeHook::HasPendingException | RuntimeHook::TakePendingException => {
                vec![ValueType::I32]
            }
            _ => Vec::new(),
        };
        FunctionSignature { params, results }
    }

    pub(crate) fn qualified_name(self) -> String {
        format!("{}::{}", self.module(), self.name())
    }

    pub(crate) fn legacy_symbol(self) -> String {
        format!("{}_{}", self.module(), self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeHook, ValueType};

    #[test]
    fn string_hooks_use_i32_handles() {
        let slice_sig = RuntimeHook::StringAsSlice.signature();
        assert_eq!(slice_sig.params, vec![ValueType::I32]);
        assert_eq!(slice_sig.results, vec![ValueType::I32, ValueType::I32]);

        let append_sig = RuntimeHook::StringAppendSlice.signature();
        assert!(
            append_sig
                .params
                .iter()
                .all(|param| *param == ValueType::I32),
            "string append slice should only use i32 handles/offsets"
        );
        assert_eq!(append_sig.results, vec![ValueType::I32]);
    }
}

mod calls;
mod control;
mod decimal;
mod intrinsics;

// The terminator lowering logic is intentionally decomposed by concern:
// - control.rs owns structural terminators (return/goto/throw/etc.).
// - calls.rs handles call-style terminators and intrinsic routing.
// - intrinsics.rs implements the specialised SIMD/Linalg helpers that the call
//   lowering delegates to after dispatch.

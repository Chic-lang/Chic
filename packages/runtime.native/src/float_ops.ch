namespace Std.Runtime.Native;
// Floating-point remainder helpers for targets lacking direct instructions (e.g., WASM fallback).
@extern("C") private static extern float fmodf(float lhs, float rhs);
@extern("C") private static extern double fmod(double lhs, double rhs);
@export("chic_rt_f32_rem") public static float chic_rt_f32_rem(float lhs, float rhs) {
    return fmodf(lhs, rhs);
}
@export("chic_rt_f64_rem") public static double chic_rt_f64_rem(double lhs, double rhs) {
    return fmod(lhs, rhs);
}

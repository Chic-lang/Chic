#include <stdint.h>
#include <stdio.h>

int __chic_program_main(int argc, char **argv, char **envp) {
  (void)argc;
  (void)argv;
  (void)envp;
  return 0;
}

struct S48 {
  uint64_t a;
  uint64_t b;
  uint64_t c;
  uint64_t d;
  uint64_t e;
  uint64_t f;
};

struct S64 {
  uint64_t a;
  uint64_t b;
  uint64_t c;
  uint64_t d;
  uint64_t e;
  uint64_t f;
  uint64_t g;
  uint64_t h;
};

struct Hfa4d {
  double a;
  double b;
  double c;
  double d;
};

struct Mix {
  uint32_t a;
  double b;
  uint16_t c;
};

extern struct S48 chic_make_s48(uint64_t v);
extern uint64_t chic_sum_s48(struct S48 v);
extern struct S48 chic_bump_s48(struct S48 v);
extern struct S64 chic_make_s64(uint64_t v);
extern uint64_t chic_sum_s64(struct S64 v);
extern struct Hfa4d chic_make_hfa4d(double x);
extern double chic_sum_hfa4d(struct Hfa4d v);
extern struct Mix chic_make_mix(uint32_t a, double b, uint16_t c);

static int assert_u64(uint64_t got, uint64_t expected, const char *label) {
  if (got != expected) {
    fprintf(stderr, "assert failed: %s got=%llu expected=%llu\n", label,
            (unsigned long long)got, (unsigned long long)expected);
    return 0;
  }
  return 1;
}

static int assert_f64(double got, double expected, const char *label) {
  if (got != expected) {
    fprintf(stderr, "assert failed: %s got=%f expected=%f\n", label, got, expected);
    return 0;
  }
  return 1;
}

int main(void) {
  struct S48 s = chic_make_s48(7);
  if (!assert_u64(s.a, 7, "s48.a")) return 1;
  if (!assert_u64(s.f, 12, "s48.f")) return 2;

  uint64_t sum = chic_sum_s48(s);
  if (!assert_u64(sum, 7 + 8 + 9 + 10 + 11 + 12, "sum_s48")) return 3;

  struct S48 bumped = chic_bump_s48(s);
  if (!assert_u64(bumped.a, 17, "bump_s48.a")) return 4;
  if (!assert_u64(bumped.f, 22, "bump_s48.f")) return 5;

  struct Hfa4d hf = chic_make_hfa4d(1.5);
  if (!assert_f64(hf.a, 1.5, "hfa.a")) return 6;
  if (!assert_f64(hf.d, 4.5, "hfa.d")) return 7;
  double hf_sum = chic_sum_hfa4d(hf);
  if (!assert_f64(hf_sum, 1.5 + 2.5 + 3.5 + 4.5, "sum_hfa4d")) return 8;

  struct S64 s64 = chic_make_s64(3);
  if (!assert_u64(s64.a, 3, "s64.a")) return 9;
  if (!assert_u64(s64.h, 10, "s64.h")) return 10;
  uint64_t sum64 = chic_sum_s64(s64);
  if (!assert_u64(sum64, 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10, "sum_s64")) return 11;

  struct Mix mix = chic_make_mix(0xdecafbadU, 1.5, 0x4321u);
  if (!assert_u64(mix.a, 0xdecafbadU, "mix.a")) return 12;
  if (!assert_f64(mix.b, 1.5, "mix.b")) return 13;
  if (!assert_u64(mix.c, 0x4321u, "mix.c")) return 14;

  return 0;
}

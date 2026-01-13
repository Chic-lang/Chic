#include <stdint.h>

struct __attribute__((packed)) S1 {
  uint8_t a;
};

struct __attribute__((packed)) S2 {
  uint16_t a;
};

struct __attribute__((packed)) S3 {
  uint8_t a;
  uint16_t b;
};

struct S4 {
  uint32_t a;
};

struct S8 {
  uint64_t a;
};

struct S16 {
  uint64_t a;
  uint64_t b;
};

struct S24 {
  uint64_t a;
  uint64_t b;
  uint64_t c;
};

struct S32 {
  uint64_t a;
  uint64_t b;
  uint64_t c;
  uint64_t d;
};

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

struct S72 {
  uint64_t a;
  uint64_t b;
  uint64_t c;
  uint64_t d;
  uint64_t e;
  uint64_t f;
  uint64_t g;
  uint64_t h;
  uint64_t i;
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

struct Outer {
  struct S16 inner;
  uint32_t tail;
};

struct __attribute__((packed)) S1 make_s1(uint8_t v) { return (struct S1){v}; }
struct __attribute__((packed)) S2 make_s2(uint16_t v) { return (struct S2){v}; }
struct __attribute__((packed)) S3 make_s3(uint8_t a, uint16_t b) { return (struct S3){a, b}; }

struct S4 make_s4(uint32_t v) { return (struct S4){v}; }

struct S8 make_s8(uint64_t v) { return (struct S8){v}; }
struct S16 make_s16(uint64_t v) { return (struct S16){v, v + 1}; }
struct S24 make_s24(uint64_t v) { return (struct S24){v, v + 1, v + 2}; }
struct S32 make_s32(uint64_t v) { return (struct S32){v, v + 1, v + 2, v + 3}; }

struct S48 make_s48(uint64_t v) {
  return (struct S48){v, v + 1, v + 2, v + 3, v + 4, v + 5};
}

uint64_t sum_s48(struct S48 v) { return v.a + v.b + v.c + v.d + v.e + v.f; }

struct S48 bump_s48(struct S48 v) {
  v.a += 10;
  v.b += 10;
  v.c += 10;
  v.d += 10;
  v.e += 10;
  v.f += 10;
  return v;
}

struct S64 make_s64(uint64_t v) {
  return (struct S64){v, v + 1, v + 2, v + 3, v + 4, v + 5, v + 6, v + 7};
}

uint64_t sum_s64(struct S64 v) { return v.a + v.b + v.c + v.d + v.e + v.f + v.g + v.h; }

struct S72 make_s72(uint64_t v) {
  return (struct S72){v, v + 1, v + 2, v + 3, v + 4, v + 5, v + 6, v + 7, v + 8};
}

uint64_t sum_s72(struct S72 v) {
  return v.a + v.b + v.c + v.d + v.e + v.f + v.g + v.h + v.i;
}

struct Mix make_mix(uint32_t a, double b, uint16_t c) { return (struct Mix){a, b, c}; }

struct Outer make_outer(uint64_t v, uint32_t tail) {
  return (struct Outer){make_s16(v), tail};
}

struct Hfa4d make_hfa4d(double x) { return (struct Hfa4d){x, x + 1.0, x + 2.0, x + 3.0}; }
double sum_hfa4d(struct Hfa4d v) { return v.a + v.b + v.c + v.d; }

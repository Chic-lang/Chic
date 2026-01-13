#include <stdint.h>
#include <stddef.h>

struct S1 {
    uint8_t a;
};

struct S2 {
    uint8_t a;
    uint8_t b;
};

struct S3 {
    uint8_t a;
    uint8_t b;
    uint8_t c;
};

struct S4 {
    int32_t a;
};

struct S8 {
    int32_t a;
    int32_t b;
};

struct S16 {
    int64_t a;
    int64_t b;
};

struct S24 {
    int64_t a;
    int64_t b;
    int64_t c;
};

struct S32 {
    int64_t a;
    int64_t b;
    int64_t c;
    int64_t d;
};

struct S64 {
    int64_t items[8];
};

struct S72 {
    int64_t items[9];
};

struct Packed {
    uint16_t a;
    uint32_t b;
    uint8_t c;
} __attribute__((packed));

struct Hfa4 {
    float a;
    float b;
    float c;
    float d;
};

struct Mixed16 {
    double a;
    float b;
};

// Chic exports implemented in the Chic test program.
extern struct S1 chic_make_s1(int64_t base);
extern int64_t chic_take_s1(struct S1 v);
extern struct S2 chic_make_s2(int64_t base);
extern int64_t chic_take_s2(struct S2 v);
extern struct S3 chic_make_s3(int64_t base);
extern int64_t chic_take_s3(struct S3 v);
extern struct S4 chic_make_s4(int64_t base);
extern int64_t chic_take_s4(struct S4 v);
extern struct S8 chic_make_s8(int64_t base);
extern int64_t chic_take_s8(struct S8 v);
extern struct S16 chic_make_s16(int64_t base);
extern int64_t chic_take_s16(struct S16 v);
extern struct S24 chic_make_s24(int64_t base);
extern int64_t chic_take_s24(struct S24 v);
extern struct S32 chic_make_s32(int64_t base);
extern int64_t chic_take_s32(struct S32 v);
extern struct S64 chic_make_s64(int64_t base);
extern int64_t chic_take_s64(struct S64 v);
extern struct S72 chic_make_s72(int64_t base);
extern int64_t chic_take_s72(struct S72 v);
extern struct Packed chic_make_packed(int64_t base);
extern int64_t chic_take_packed(struct Packed v);
extern struct Hfa4 chic_make_hfa4(int64_t base);
extern int64_t chic_take_hfa4(struct Hfa4 v);
extern struct Mixed16 chic_make_mixed16(int64_t base);
extern int64_t chic_take_mixed16(struct Mixed16 v);

static int64_t sum_array(const int64_t *items, size_t count) {
    int64_t acc = 0;
    for (size_t i = 0; i < count; i++) {
        acc += items[i];
    }
    return acc;
}

struct S1 make_s1(int64_t base) {
    struct S1 v = { (uint8_t)(base + 1) };
    return v;
}

int64_t sum_s1(struct S1 v) { return v.a; }

struct S2 make_s2(int64_t base) {
    struct S2 v = { (uint8_t)(base + 1), (uint8_t)(base + 2) };
    return v;
}

int64_t sum_s2(struct S2 v) { return v.a + v.b; }

struct S3 make_s3(int64_t base) {
    struct S3 v = { (uint8_t)(base + 1), (uint8_t)(base + 2), (uint8_t)(base + 3) };
    return v;
}

int64_t sum_s3(struct S3 v) { return v.a + v.b + v.c; }

struct S4 make_s4(int64_t base) {
    struct S4 v = { (int32_t)(base * 2) };
    return v;
}

int64_t sum_s4(struct S4 v) { return v.a; }

struct S8 make_s8(int64_t base) {
    struct S8 v = { (int32_t)(base), (int32_t)(base + 10) };
    return v;
}

int64_t sum_s8(struct S8 v) { return (int64_t)v.a + (int64_t)v.b; }

struct S16 make_s16(int64_t base) {
    struct S16 v = { base + 1, base + 2 };
    return v;
}

int64_t sum_s16(struct S16 v) { return v.a + v.b; }

struct S24 make_s24(int64_t base) {
    struct S24 v = { base + 5, base + 6, base + 7 };
    return v;
}

int64_t sum_s24(struct S24 v) { return v.a + v.b + v.c; }

struct S32 make_s32(int64_t base) {
    struct S32 v = { base + 1, base + 2, base + 3, base + 4 };
    return v;
}

int64_t sum_s32(struct S32 v) { return v.a + v.b + v.c + v.d; }

struct S64 make_s64(int64_t base) {
    struct S64 v;
    for (int i = 0; i < 8; i++) {
        v.items[i] = base + i;
    }
    return v;
}

int64_t sum_s64(struct S64 v) { return sum_array(v.items, 8); }

struct S72 make_s72(int64_t base) {
    struct S72 v;
    for (int i = 0; i < 9; i++) {
        v.items[i] = base + (i * 2);
    }
    return v;
}

int64_t sum_s72(struct S72 v) { return sum_array(v.items, 9); }

struct Packed make_packed(int64_t base) {
    struct Packed v = { (uint16_t)(base + 1), (uint32_t)(base + 2), (uint8_t)(base + 3) };
    return v;
}

int64_t sum_packed(struct Packed v) { return (int64_t)v.a + v.b + v.c; }

struct Hfa4 make_hfa4(int64_t base) {
    struct Hfa4 v = { (float)(base + 1.5f), (float)(base + 2.5f), (float)(base + 3.5f), (float)(base + 4.5f) };
    return v;
}

int64_t sum_hfa4(struct Hfa4 v) {
    double acc = (double)v.a + (double)v.b + (double)v.c + (double)v.d;
    return (int64_t)acc;
}

struct Mixed16 make_mixed16(int64_t base) {
    struct Mixed16 v = { (double)(base + 8.0), (float)(base + 2.0f) };
    return v;
}

int64_t sum_mixed16(struct Mixed16 v) {
    double acc = v.a + (double)v.b;
    return (int64_t)acc;
}

int64_t call_chic_make_s1(int64_t base) {
    struct S1 v = chic_make_s1(base);
    return sum_s1(v);
}

int64_t call_chic_take_s1(struct S1 v) { return chic_take_s1(v); }

int64_t call_chic_make_s2(int64_t base) {
    struct S2 v = chic_make_s2(base);
    return sum_s2(v);
}

int64_t call_chic_take_s2(struct S2 v) { return chic_take_s2(v); }

int64_t call_chic_make_s3(int64_t base) {
    struct S3 v = chic_make_s3(base);
    return sum_s3(v);
}

int64_t call_chic_take_s3(struct S3 v) { return chic_take_s3(v); }

int64_t call_chic_make_s4(int64_t base) {
    struct S4 v = chic_make_s4(base);
    return sum_s4(v);
}

int64_t call_chic_take_s4(struct S4 v) { return chic_take_s4(v); }

int64_t call_chic_make_s8(int64_t base) {
    struct S8 v = chic_make_s8(base);
    return sum_s8(v);
}

int64_t call_chic_take_s8(struct S8 v) { return chic_take_s8(v); }

int64_t call_chic_make_s16(int64_t base) {
    struct S16 v = chic_make_s16(base);
    return sum_s16(v);
}

int64_t call_chic_take_s16(struct S16 v) { return chic_take_s16(v); }

int64_t call_chic_make_s24(int64_t base) {
    struct S24 v = chic_make_s24(base);
    return sum_s24(v);
}

int64_t call_chic_take_s24(struct S24 v) { return chic_take_s24(v); }

int64_t call_chic_make_s32(int64_t base) {
    struct S32 v = chic_make_s32(base);
    return sum_s32(v);
}

int64_t call_chic_take_s32(struct S32 v) { return chic_take_s32(v); }

int64_t call_chic_make_s64(int64_t base) {
    struct S64 v = chic_make_s64(base);
    return sum_s64(v);
}

int64_t call_chic_take_s64(struct S64 v) { return chic_take_s64(v); }

int64_t call_chic_make_s72(int64_t base) {
    struct S72 v = chic_make_s72(base);
    return sum_s72(v);
}

int64_t call_chic_take_s72(struct S72 v) { return chic_take_s72(v); }

int64_t call_chic_make_packed(int64_t base) {
    struct Packed v = chic_make_packed(base);
    return sum_packed(v);
}

int64_t call_chic_take_packed(struct Packed v) { return chic_take_packed(v); }

int64_t call_chic_make_hfa4(int64_t base) {
    struct Hfa4 v = chic_make_hfa4(base);
    return sum_hfa4(v);
}

int64_t call_chic_take_hfa4(struct Hfa4 v) { return chic_take_hfa4(v); }

int64_t call_chic_make_mixed16(int64_t base) {
    struct Mixed16 v = chic_make_mixed16(base);
    return sum_mixed16(v);
}

int64_t call_chic_take_mixed16(struct Mixed16 v) { return chic_take_mixed16(v); }

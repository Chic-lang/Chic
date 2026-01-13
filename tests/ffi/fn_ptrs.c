#include <stdint.h>

struct Big {
  int64_t a;
  int64_t b;
  int64_t c;
};

typedef struct Big (*make_big_fn)(int64_t base);
typedef int64_t (*sum_big_fn)(struct Big value);

struct Big c_make_big(int64_t base) { return (struct Big){base, base + 1, base + 2}; }

int64_t c_sum_big(struct Big value) { return value.a + value.b + value.c; }

int64_t c_call_chic_make(make_big_fn cb) {
  struct Big v = cb(50);
  return v.a + v.b + v.c;
}

int64_t c_call_chic_sum(sum_big_fn cb) {
  struct Big v = c_make_big(7);
  return cb(v);
}

make_big_fn c_provide_big_cb(void) { return &c_make_big; }

sum_big_fn c_provide_sum_cb(void) { return &c_sum_big; }

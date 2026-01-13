#include <stdint.h>

struct Value {
    long marker;
    long other;
};

static struct Value GLOBAL_VALUE = {99, 0};

void touch_void(void *ptr) {
    struct Value *value = (struct Value *)ptr;
    value->marker = 42;
}

long read_const(const struct Value *ptr) {
    return ptr->marker + ptr->other;
}

void *get_void_pointer(void) { return &GLOBAL_VALUE; }

int is_null(void *ptr) { return ptr == 0; }

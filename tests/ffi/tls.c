#include <pthread.h>
#include <stdint.h>

// Weak stubs to satisfy the runtime when the stdlib is skipped. They are
// never invoked in this fixture but keep the linker happy.
__attribute__((weak)) void chic_thread_invoke(void *ctx) { (void)ctx; }
__attribute__((weak)) void chic_thread_drop(void *ctx) { (void)ctx; }

extern int chic_tls_get(void);
extern int chic_tls_inc(int delta);

struct thread_args {
    int delta;
    int iterations;
    int start;
    int end;
};

static void *thread_entry(void *ctx) {
    struct thread_args *args = (struct thread_args *)ctx;
    args->start = chic_tls_get();
    int value = args->start;
    for (int i = 0; i < args->iterations; i++) {
        value = chic_tls_inc(args->delta);
    }
    args->end = value;
    return NULL;
}

int run_tls_threads(int delta_a, int delta_b) {
    struct thread_args a = {.delta = delta_a, .iterations = 2, .start = -1, .end = -1};
    struct thread_args b = {.delta = delta_b, .iterations = 3, .start = -1, .end = -1};

    pthread_t thread_a;
    pthread_t thread_b;
    if (pthread_create(&thread_a, NULL, thread_entry, &a) != 0) {
        return -11;
    }
    if (pthread_create(&thread_b, NULL, thread_entry, &b) != 0) {
        return -12;
    }
    pthread_join(thread_a, NULL);
    pthread_join(thread_b, NULL);

    if (a.start != 0 || b.start != 0) {
        return -21;
    }
    if (a.end != a.delta * a.iterations) {
        return -31;
    }
    if (b.end != b.delta * b.iterations) {
        return -32;
    }

    return a.end + b.end;
}

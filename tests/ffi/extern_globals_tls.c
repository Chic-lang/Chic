#include <pthread.h>
#include <stdint.h>

_Thread_local int tls_value = 1;

extern int chic_tls_read(void);
extern int chic_tls_add(int delta);

static void* thread_add(void* arg) {
    int delta = (int)(intptr_t)arg;
    int result = chic_tls_add(delta);
    return (void*)(intptr_t)result;
}

int run_tls_threads(int delta_a, int delta_b) {
    pthread_t thread_a;
    pthread_t thread_b;
    pthread_create(&thread_a, 0, thread_add, (void*)(intptr_t)delta_a);
    pthread_create(&thread_b, 0, thread_add, (void*)(intptr_t)delta_b);

    void* out_a = 0;
    void* out_b = 0;
    pthread_join(thread_a, &out_a);
    pthread_join(thread_b, &out_b);

    return (int)(intptr_t)out_a + (int)(intptr_t)out_b;
}

void pthread_link_anchor(void) {}

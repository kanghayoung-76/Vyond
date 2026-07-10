//******************************************************************************
// Native baseline driver for BEEBS crc32 (paper eval #5). Same timed region as
// the enclave eapp (main.c) — REPEAT_FACTOR crc32 calls, no warmup — but prints
// the cycle count directly (real Linux, no UTM constraint). Compare its
// cycles= against the enclave's compute cycles for the pure-running overhead.
//******************************************************************************
#include <stdio.h>
#include "support.h"

extern void initialise_benchmark(void);
extern int  benchmark(void);
extern int  verify_benchmark(int result);

int main(void) {
    volatile int result = 0;
    unsigned long t0, t1;

    initialise_benchmark();
    asm volatile("rdcycle %0" : "=r"(t0));   // rdtime for latency
    for (int i = 0; i < REPEAT_FACTOR; i++) {
        initialise_benchmark();
        result = benchmark();
    }
    asm volatile("rdcycle %0" : "=r"(t1));
    int correct = verify_benchmark((int)result);

    printf("[beebs crc32 native] cycles=%lu correct=%d REPEAT=%d\n",
           t1 - t0, correct, REPEAT_FACTOR);
    return correct ? 0 : 1;
}

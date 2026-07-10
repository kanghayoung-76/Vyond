//******************************************************************************
// WorldGuard paper eval #5 — workload (BEEBS crc32) driver.
//
// Plain program used BOTH as the enclave eapp (built with io_syscall/
// linux_syscall plugins so printf is forwarded to the host) and as the native
// baseline binary. This first version has NO timing — it just runs the kernel
// and prints the verify result, to confirm the workload runs in the enclave.
// (rdcycle timing is added once this boots cleanly.)
//******************************************************************************
#include <stdio.h>
#include "support.h"

extern void initialise_benchmark(void);
extern int  benchmark(void);
extern int  verify_benchmark(int result);

int main(void) {
    volatile int result = 0;

    initialise_benchmark();
    for (int i = 0; i < REPEAT_FACTOR; i++) {
        initialise_benchmark();
        result = benchmark();
    }
    int correct = verify_benchmark((int)result);

    printf("[beebs crc32] result=%d correct=%d REPEAT=%d\n",
           (int)result, correct, REPEAT_FACTOR);
    return correct ? 0 : 1;
}

//******************************************************************************
// WorldGuard paper eval #5 — workload (BEEBS crc32) driver.
// Plain program used as the enclave eapp (built with io_syscall/linux_syscall
// plugins so printf is forwarded to the host). No timing yet — just runs the
// kernel and prints the verify result, to confirm the workload runs in-enclave.
//******************************************************************************
#include "support.h"

extern void initialise_benchmark(void);
extern int  benchmark(void);
extern int  verify_benchmark(int result);

// No printf: avoid the io_syscall/UTM path entirely (isolate whether the UTM
// access is what faults). Result is returned as the exit code: 0 = correct,
// 1 = wrong. No page fault + clean exit(0) => workload ran & verified in-enclave.
int main(void) {
    volatile int result = 0;

    initialise_benchmark();
    for (int i = 0; i < REPEAT_FACTOR; i++) {
        initialise_benchmark();
        result = benchmark();
    }
    int correct = verify_benchmark((int)result);

    return correct ? 0 : 1;
}

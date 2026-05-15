/*
 * wg-reuse-test eapp
 *
 * Each printf triggers an io_syscall OCALL:
 *   sbi_stop_enclave(0) -> EnclaveInterrupted -> SDK resume() -> resume_enclave
 *   -> enter_enclave_context finds WID already in WID_SLOTS -> [WID] reuse logged.
 *
 * Flow:
 *   1st enter_enclave_context: no WID yet (EPM not loaded)
 *   ACCESS FAULT: assign_wid -> [WID] assign
 *   resume from fault, eapp runs
 *   each printf below: OCALL cycle -> [WID] reuse
 */
#include <stdio.h>

int main()
{
    printf("[EAPP] start\n");
    for (int i = 0; i < 5; i++) {
        printf("[EAPP] step %d\n", i);
    }
    printf("[EAPP] done\n");
    return 0;
}

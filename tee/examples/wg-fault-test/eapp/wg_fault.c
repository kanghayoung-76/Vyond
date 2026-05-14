#include <stdio.h>

/* This eapp should never be reached:
 * The eyrie loader (built with WG_FAULT_TEST=1) accesses SM physical memory
 * (0x80000000) before page tables are set up (satp=0). That triggers a
 * WGChecker violation -> CAUSE_LOAD_ACCESS_FAULT -> SM M-mode trap handler. */
int main()
{
    printf("[ENCLAVE] wg-fault-test eapp reached (unexpected - loader should have faulted)\n");
    return 0;
}

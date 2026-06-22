/*
 * split-host-test: subscriber enclave (enc2)
 *
 * sub-host initializes this enclave and parks it at wait_shm().
 * enc1 (bridge) calls notify_shm() → SM directly resumes enc2 here
 * (no host involvement).  After 3 iterations enc2 exits — SM restores
 * bridge-host's thread context.
 *
 * OCALLs:
 *   OCALL_GET_TDDS_RID (5): sub-host returns rid of the TDDS channel
 *   OCALL_PRINT        (3): print a string via sub-host
 */
#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "tdds_rclcpp.h"

#define OCALL_PRINT        3
#define OCALL_GET_TDDS_RID 5

#define N_ITERS 3

static void eprint(const char *msg)
{
    size_t avail;
    char *dst = (char *)tdds_utm_data(&avail);
    size_t len = strlen(msg) + 1;
    if (len > avail) len = avail;
    for (size_t i = 0; i < len; i++) dst[i] = msg[i];
    ocall(OCALL_PRINT, dst, len, NULL, 0);
}

int main(void)
{
    __asm__ __volatile__(
        ".option push\n\t.option norelax\n\t"
        "la gp, __global_pointer$\n\t"
        ".option pop\n\t" ::: "memory");

    /* Get TDDS channel RID from sub-host */
    uintptr_t rid_ptr = 0;
    ocall(OCALL_GET_TDDS_RID, NULL, 0, &rid_ptr, sizeof(rid_ptr));
    rid_t rid = *(rid_t *)rid_ptr;

    Subscription sub;
    if (create_subscription(&sub, rid) != 0) {
        eprint("[SUB] init fail");
        EAPP_RETURN(1);
    }

    /* Park here: SM (via wait_shm) suspends enc2, returns to sub-host.
     * sub-host's run() returns Success.  When bridge calls notify_shm,
     * SM directly resumes enc2 (single-core: within bridge-host's ioctl
     * thread; multi-core: via M-mode MSIP IPI). */
    wait(&sub);

    char buf[TDDS_MAX_MSG_SIZE + 1];
    for (int i = 0; i < N_ITERS; i++) {
        int n = take(&sub, buf, TDDS_MAX_MSG_SIZE);
        if (n > 0) {
            buf[n] = '\0';
            eprint("[ENC2] received: ");
            eprint(buf);
        }
        if (i < N_ITERS - 1)
            wait(&sub); /* park again → SM resumes enc1 */
    }

    /* enc2 exits: SM restores bridge-host's Linux context (single-core),
     * bridge-host's ioctl returns Success.  bridge-host then resumes enc1. */
    EAPP_RETURN(0);
}

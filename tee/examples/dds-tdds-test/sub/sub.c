#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "tdds_rclcpp.h"

#define OCALL_GET_RID  2
#define OCALL_PRINT    3

static void eprint(const char *msg) {
    size_t avail;
    char *dst = (char *)tdds_utm_data(&avail);
    size_t len = strlen(msg) + 1;
    if (len > avail) len = avail;
    for (size_t i = 0; i < len; i++) dst[i] = msg[i];
    ocall(OCALL_PRINT, dst, len, NULL, 0);
}

int main() {
    __asm__ __volatile__(
        ".option push\n\t.option norelax\n\t"
        "la gp, __global_pointer$\n\t"
        ".option pop\n\t" ::: "memory");

    uintptr_t ret_ptr = 0;
    ocall(OCALL_GET_RID, NULL, 0, &ret_ptr, sizeof(ret_ptr));
    rid_t rid = *(rid_t *)ret_ptr;

    Subscription sub;
    if (create_subscription(&sub, rid) != 0) { eprint("[SUB] init fail"); EAPP_RETURN(1); }

    /* Phase 2: suspend here until enc1 publishes for the first time */
    wait(&sub);

    /* Loop: take + print, then wait again for next publish.
     * On each wait_shm call, SM resumes enc1 directly (no HOST involvement). */
    char buf[TDDS_MAX_MSG_SIZE + 1];
    for (int i = 0; i < 3; i++) {
        int n = take(&sub, buf, TDDS_MAX_MSG_SIZE);
        if (n > 0) {
            buf[n] = '\0';
            eprint("[SUB] received: ");
            eprint(buf);
        }
        if (i < 2) wait(&sub); /* suspend; SM switches back to enc1 */
    }
    /* After 3rd take, enc2 exits — restoring enc1's HOST context → bridgeEnc.run() returns */
    EAPP_RETURN(0);
}

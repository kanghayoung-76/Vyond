#include "eapp_utils.h"
#include "string.h"
#include "syscall.h"
#include "tdds.h"

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

    tdds_channel_t ch;
    if (tdds_init_channel(&ch, rid) != 0) { eprint("[SUB] init failed"); EAPP_RETURN(1); }

    char buf[TDDS_MAX_MSG_SIZE];
    int n;
    while ((n = tdds_subscribe(&ch, buf, sizeof(buf))) == TDDS_NOT_READY)
        ocall_tdds_wait(ch.rid);

    if (n < 0) { eprint("[SUB] subscribe error"); EAPP_RETURN(1); }

    eprint("[SUB] received (hex):");
    const unsigned char *p = (const unsigned char *)buf;
    for (int row = 0; row < n; row += 16) {
        char line[3 * 16 + 1];
        int lim = (n - row < 16) ? (n - row) : 16;
        for (int i = 0; i < lim; i++) {
            unsigned char b = p[row + i];
            line[3*i]   = "0123456789abcdef"[b >> 4];
            line[3*i+1] = "0123456789abcdef"[b & 0xf];
            line[3*i+2] = ' ';
        }
        line[3*lim - 1] = '\0';
        eprint(line);
    }

    tdds_destroy_channel(&ch);
    EAPP_RETURN(0);
}

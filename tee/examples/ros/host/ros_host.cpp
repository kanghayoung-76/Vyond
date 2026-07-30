#include <pthread.h>
#include <cstddef>
#include <string.h>
#include "host/keystone.h"
#include "host/SharedMemory.hpp"
#include "edge_wrapper.h"

// FPGA static-link workaround: glibc IFUNC memcpy (R_RISCV_IRELATIVE) is not
// resolved in our static binary, so the PLT self-loops and crashes. Providing
// our own memcpy makes the linker drop the IFUNC indirection entirely.
extern "C" void* memcpy(void* dest, const void* src, size_t n) {
  char* d = (char*)dest;
  const char* s = (const char*)src;
  while (n--) *d++ = *s++;
  return dest;
}

using namespace Keystone;

Enclave enc_publisher, enc_subscriber;
SharedMemory shm;

shm_t loan_shm(int id)
{
    shm_t s;
    s.rid  = shm.getRID();
    s.pa   = (uintptr_t)shm.getPA();
    s.size = shm.getSize();
    printf("[HOST] loan_shm id=%d rid=%d pa=%#lx size=%zu\n", id, s.rid, s.pa, s.size);
    return s;
}

void print_value(unsigned long val)
{
    printf("[HOST] enclave value: %lu (%#lx)\n", val, val);
}

unsigned long print_buffer(char *str)
{
    printf("[HOST] enclave says: %s", str);
    return strlen(str);
}

const char *get_host_string() { return "hello"; }

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 5) {
        fprintf(stderr, "Usage: %s <publisher> <subscriber> <eyrie-rt> <loader>\n", argv[0]);
        return 1;
    }

    printf("[HOST] Entering main...\n");

    Params params;
    params.setFreeMemSize(256 * 1024);
    params.setUntrustedSize(256 * 1024);

    enc_publisher.init(argv[1], argv[3], argv[4], params);
    printf("[HOST] Publisher (eid=%d) initialized\n", enc_publisher.getEID());

    enc_subscriber.init(argv[2], argv[3], argv[4], params);
    printf("[HOST] Subscriber (eid=%d) initialized\n", enc_subscriber.getEID());

    /* Create one SHM shared by both enclaves */
    rid_t rid = shm.createShm(0x1000);
    shm.changeShm(rid, 7);
    shm.shareShm(rid, enc_publisher.getEID(), 7);
    shm.shareShm(rid, enc_subscriber.getEID(), 7);
    printf("[HOST] SHM created: rid=%d pa=%p\n", rid, shm.getPA());

    /* Publisher: writes data[0..2] to SHM */
    edge_init(&enc_publisher);
    printf("[HOST] Running publisher...\n");
    enc_publisher.run();
    printf("[HOST] Publisher done\n");

    /* Subscriber: reads data[0..2] from the same SHM */
    edge_init(&enc_subscriber);
    printf("[HOST] Running subscriber...\n");
    enc_subscriber.run();
    printf("[HOST] Subscriber done\n");

    printf("[HOST] Done\n");
    return 0;
}

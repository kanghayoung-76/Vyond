#include <edge_call.h>
#include <keystone.h>
#include <stdio.h>
#include <string.h>

#define OCALL_PRINT_STRING 1
#define OCALL_ADD_NUMBERS  2

/* Shared buffer base/size — set once after enclave init */
static uintptr_t shm_base = 0;
static size_t    shm_size = 0;

/*
 * edge_call helpers that operate on our shm_base / shm_size instead of
 * the global _shared_start used by the old edge_call_init_internals().
 */
static uintptr_t args_ptr(struct edge_call* ec, size_t* size_out) {
    *size_out = ec->call_arg_size;
    uintptr_t ptr;
    if (edge_call_get_ptr_from_offset(ec->call_arg_offset, ec->call_arg_size,
                                      &ptr, shm_base, shm_size) != 0)
        return 0;
    return ptr;
}

static void set_ret(struct edge_call* ec, const void* data, size_t size) {
    /* write return payload just after the edge_call header */
    uintptr_t ret_area = shm_base + sizeof(struct edge_call);
    memcpy((void*)ret_area, data, size);
    if (edge_call_setup_ret(ec, (void*)ret_area, size,
                            shm_base, shm_size) != 0) {
        ec->return_data.call_status = CALL_STATUS_BAD_PTR;
    } else {
        ec->return_data.call_status = CALL_STATUS_OK;
    }
}

/* OCALL handler: print a string the enclave sent */
static void handle_print_string(void* shm) {
    struct edge_call* ec = (struct edge_call*)shm;
    size_t arg_len;
    uintptr_t str_ptr = args_ptr(ec, &arg_len);
    if (!str_ptr) {
        ec->return_data.call_status = CALL_STATUS_BAD_OFFSET;
        return;
    }
    /* ensure null-terminated within bounds */
    char buf[256] = {0};
    size_t copy_len = arg_len < sizeof(buf) - 1 ? arg_len : sizeof(buf) - 1;
    memcpy(buf, (void*)str_ptr, copy_len);

    unsigned long printed = printf("[HOST][OCALL] Enclave says: \"%s\"\n", buf);
    set_ret(ec, &printed, sizeof(printed));
}

/* OCALL handler: add two unsigned longs and return the sum */
static void handle_add_numbers(void* shm) {
    struct edge_call* ec = (struct edge_call*)shm;
    size_t arg_len;
    uintptr_t args_p = args_ptr(ec, &arg_len);
    if (!args_p || arg_len < 2 * sizeof(unsigned long)) {
        ec->return_data.call_status = CALL_STATUS_BAD_OFFSET;
        return;
    }
    unsigned long* nums = (unsigned long*)args_p;
    unsigned long result = nums[0] + nums[1];
    printf("[HOST][OCALL] add_numbers(%lu, %lu) = %lu\n",
           nums[0], nums[1], result);
    set_ret(ec, &result, sizeof(result));
}

/* Dispatch table — called by the Keystone host library on each OCALL */
static void ocall_dispatch(void* buffer, size_t /*size*/) {
    struct edge_call* ec = (struct edge_call*)buffer;
    switch (ec->call_id) {
        case OCALL_PRINT_STRING: handle_print_string(buffer); break;
        case OCALL_ADD_NUMBERS:  handle_add_numbers(buffer);  break;
        default:
            printf("[HOST] unknown OCALL id=%lu\n", ec->call_id);
            ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
    }
}

int main(int argc, char** argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <eapp> <runtime> <loader>\n", argv[0]);
        return 1;
    }

    printf("[HOST] shm-ocall-test: initialising enclave...\n");

    Keystone::Enclave enclave;
    Keystone::Params  params;
    params.setFreeMemSize(256 * 1024);
    params.setUntrustedSize(256 * 1024);   /* this physical region becomes the SHM */

    enclave.init(argv[1], argv[2], argv[3], params);

    /* The Keystone SDK maps the shared buffer (formerly UTM, now SHM) and
     * exposes it via getSharedBuffer() / getSharedBufferSize(). */
    shm_base = (uintptr_t)enclave.getSharedBuffer();
    shm_size = enclave.getSharedBufferSize();
    printf("[HOST] SHM buffer: base=0x%lx size=%zu\n", shm_base, shm_size);

    /* Register our dispatch function; the SDK calls it on each OCALL stop. */
    enclave.registerOcallDispatch(
        [](void* buf, size_t sz) { ocall_dispatch(buf, sz); });

    printf("[HOST] Running enclave...\n");
    enclave.run();
    printf("[HOST] Enclave finished.\n");

    return 0;
}

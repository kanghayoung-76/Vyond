#include "edge_wrapper.h"
#include <string.h>

#define OCALL_PRINT_BUFFER 1
#define OCALL_PRINT_VALUE  2

void edge_init(Keystone::Enclave *enclave)
{
    enclave->registerOcallDispatch(incoming_call_dispatch);
    register_call(OCALL_PRINT_BUFFER, print_buffer_wrapper);
    register_call(OCALL_PRINT_VALUE,  print_value_wrapper);
}

void print_buffer_wrapper(void *buffer, size_t _shared_len)
{
    uintptr_t _shared_start = (uintptr_t)buffer;
    struct edge_call *edge_call = (struct edge_call *)buffer;

    uintptr_t call_args;
    unsigned long ret_val;
    size_t arg_len;
    if (edge_call_args_ptr(edge_call, &call_args, &arg_len, _shared_start, _shared_len) != 0) {
        edge_call->return_data.call_status = CALL_STATUS_BAD_OFFSET;
        return;
    }

    ret_val = print_buffer((char *)call_args);

    uintptr_t data_section = edge_call_data_ptr(_shared_start, _shared_len);
    memcpy((void *)data_section, &ret_val, sizeof(unsigned long));

    if (edge_call_setup_ret(edge_call, (void *)data_section, sizeof(unsigned long),
                            _shared_start, _shared_len)) {
        edge_call->return_data.call_status = CALL_STATUS_BAD_PTR;
    } else {
        edge_call->return_data.call_status = CALL_STATUS_OK;
    }
}

void print_value_wrapper(void *buffer, size_t _shared_len)
{
    uintptr_t _shared_start = (uintptr_t)buffer;
    struct edge_call *edge_call = (struct edge_call *)buffer;

    uintptr_t call_args;
    size_t args_len;
    if (edge_call_args_ptr(edge_call, &call_args, &args_len, _shared_start, _shared_len) != 0) {
        edge_call->return_data.call_status = CALL_STATUS_BAD_OFFSET;
        return;
    }

    print_value(*(unsigned long *)call_args);
    edge_call->return_data.call_status = CALL_STATUS_OK;
}

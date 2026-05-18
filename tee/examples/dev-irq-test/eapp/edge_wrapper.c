#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "edge_wrapper.h"

#define OCALL_PRINT_BUFFER 1
#define OCALL_PRINT_VALUE  2

void edge_init() {}

unsigned long ocall_print_buffer(char *data, size_t data_len)
{
    unsigned long retval;
    ocall(OCALL_PRINT_BUFFER, data, data_len, &retval, sizeof(unsigned long));
    return retval;
}

void ocall_print_value(unsigned long val)
{
    unsigned long val_ = val;
    ocall(OCALL_PRINT_VALUE, &val_, sizeof(unsigned long), 0, 0);
}

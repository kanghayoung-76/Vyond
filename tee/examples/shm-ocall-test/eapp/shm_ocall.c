#include "eapp_utils.h"
#include "edge_call.h"
#include "string.h"
#include <syscall.h>

#define OCALL_PRINT_STRING 1
#define OCALL_ADD_NUMBERS  2

unsigned long ocall_print_string(const char* str);
unsigned long ocall_add_numbers(unsigned long a, unsigned long b);

int main() {
    /* Test 1: print string via ocall */
    ocall_print_string("Hello from enclave via SHM-based OCALL!");

    /* Test 2: compute via ocall and verify return value */
    unsigned long result = ocall_add_numbers(40, 2);
    if (result == 42) {
        ocall_print_string("ADD test passed: 40 + 2 = 42");
    } else {
        ocall_print_string("ADD test FAILED: unexpected result");
    }

    EAPP_RETURN(0);
}

unsigned long ocall_print_string(const char* str) {
    unsigned long retval;
    ocall(OCALL_PRINT_STRING, (void*)str, strlen(str) + 1,
          &retval, sizeof(unsigned long));
    return retval;
}

/* Pass two numbers as a two-element array */
unsigned long ocall_add_numbers(unsigned long a, unsigned long b) {
    unsigned long args[2] = {a, b};
    unsigned long retval;
    ocall(OCALL_ADD_NUMBERS, args, sizeof(args),
          &retval, sizeof(unsigned long));
    return retval;
}

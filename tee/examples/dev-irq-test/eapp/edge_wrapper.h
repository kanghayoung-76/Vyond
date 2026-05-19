#ifndef _EDGE_WRAPPER_H_
#define _EDGE_WRAPPER_H_
#include "edge/edge_call.h"
#include "shared/keystone_user.h"

typedef struct shm {
    rid_t    rid;
    uintptr_t pa;
    size_t   size;
} shm_t;

void edge_init();

unsigned long ocall_print_buffer(char *data, size_t data_len);
void ocall_print_value(unsigned long val);
shm_t ocall_loan_shm(int id);

#endif /* _EDGE_WRAPPER_H_ */

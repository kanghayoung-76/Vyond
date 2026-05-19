#ifndef _EDGE_WRAPPER_H_
#define _EDGE_WRAPPER_H_

#include "edge/edge_call.h"
#include "host/keystone.h"
#include "host/SharedMemory.hpp"

typedef struct shm {
    rid_t     rid;
    uintptr_t pa;
    size_t    size;
} shm_t;

void edge_init(Keystone::Enclave *enclave);

void print_buffer_wrapper(void *buffer, size_t size);
unsigned long print_buffer(char *str);

void print_value_wrapper(void *buffer, size_t size);
void print_value(unsigned long val);

void set_dev_shm(Keystone::SharedMemory *shm);
shm_t loan_shm(int id);
void loan_shm_wrapper(void *buffer, size_t size);

#endif /* _EDGE_WRAPPER_H_ */

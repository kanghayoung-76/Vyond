#ifndef _EDGE_WRAPPER_H_
#define _EDGE_WRAPPER_H_

#include "edge/edge_call.h"
#include "host/keystone.h"

void edge_init(Keystone::Enclave *enclave);

void print_buffer_wrapper(void *buffer, size_t size);
unsigned long print_buffer(char *str);

void print_value_wrapper(void *buffer, size_t size);
void print_value(unsigned long val);

#endif /* _EDGE_WRAPPER_H_ */

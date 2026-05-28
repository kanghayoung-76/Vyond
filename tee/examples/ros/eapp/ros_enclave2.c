#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "edge_wrapper.h"

void EAPP_ENTRY eapp_entry()
{
    /* Get the same SHM (id=0) — publisher already wrote to it */
    shm_t shm = ocall_loan_shm(0);

    int *data = (int *)map_shm(shm.rid);

    /* Print what the publisher wrote */
    ocall_print_value((unsigned long)(unsigned int)data[0]);
    ocall_print_value((unsigned long)data[1]);
    ocall_print_value((unsigned long)data[2]);

    unmap_shm(shm.rid, data, shm.size);
    EAPP_RETURN(0);
}

#include "app/eapp_utils.h"
#include "app/string.h"
#include "app/syscall.h"
#include "edge_wrapper.h"

void EAPP_ENTRY eapp_entry()
{
    /* Get the SHM rid/pa/size from the host dynamically */
    shm_t shm = ocall_loan_shm(0);

    int *data = (int *)map_shm(shm.rid);

    /* Write data for the subscriber to read */
    data[0] = 0xCAFEBABE;
    data[1] = 42;
    data[2] = 1234;

    ocall_print_value((unsigned long)(unsigned int)data[0]);

    unmap_shm(shm.rid, data, shm.size);
    EAPP_RETURN(0);
}

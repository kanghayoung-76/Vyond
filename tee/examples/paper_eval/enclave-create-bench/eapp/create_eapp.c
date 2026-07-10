//******************************************************************************
// WorldGuard paper eval #4 — enclave create/destroy latency.
// Minimal enclave: the benchmark only times host-side init()+destroy(), it
// never run()s the enclave. This eapp exists solely as a valid, hashable image.
//******************************************************************************
#include "eapp_utils.h"

int main(void) {
    EAPP_RETURN(0);
}

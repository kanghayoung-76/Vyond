#include "host/keystone.h"
#include "edge/edge_call.h"
#include <stdio.h>

using namespace Keystone;

int main(int argc, char** argv)
{
    if (argc < 4) {
        printf("Usage: %s <eapp> <runtime> <loader>\n", argv[0]);
        return 1;
    }

    /*
     * WID REUSE test
     *
     * Each printf in the eapp triggers an io_syscall OCALL:
     *   sbi_stop_enclave(0) -> EnclaveInterrupted -> SDK resume()
     *   -> SM resume_enclave -> enter_enclave_context(eid)
     *   -> get_assigned_wid(eid) = Some(WID)  [HW slot still valid]
     *   -> [WID] reuse: eid=X WID=Y (HW slot valid, skip fault)
     *
     * ASSIGN fires once (first EPM access fault).
     * REUSE fires on every printf OCALL resume.
     */

    printf("[HOST] wg-reuse-test: running enclave with OCALL-based reuse demo...\n");
    printf("[HOST] Expect: one [WID] assign, then [WID] reuse on each printf OCALL.\n\n");

    Enclave enclave;
    Params params;
    params.setFreeMemSize(256 * 1024);
    params.setUntrustedSize(256 * 1024);

    Error err = enclave.init(argv[1], argv[2], argv[3], params);
    if (err != Error::Success) {
        printf("[HOST] FAIL: enclave init (err=%d)\n", (int)err);
        return 1;
    }
    printf("[HOST] enclave created EID=%d\n", enclave.getEID());

    enclave.registerOcallDispatch(incoming_call_dispatch);

    err = enclave.run();
    if (err != Error::Success) {
        printf("[HOST] FAIL: enclave run (err=%d)\n", (int)err);
        return 1;
    }

    printf("\n[HOST] Done. Check SM log above for [WID] assign + [WID] reuse entries.\n");
    return 0;
}

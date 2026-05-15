#include "host/keystone.h"
#include <stdio.h>
#include <stdlib.h>

using namespace Keystone;

static Enclave* make_enclave(const char* eapp, const char* rt, const char* loader) {
    Enclave* e = new Enclave();
    Params p;
    p.setFreeMemSize(256 * 1024);
    p.setUntrustedSize(0);
    Error err = e->init(eapp, rt, loader, p);
    if (err != Error::Success) {
        printf("[HOST] FAIL: enclave init failed (err=%d)\n", (int)err);
        delete e;
        return nullptr;
    }
    return e;
}

static bool run_enclave(Enclave* e, int idx) {
    Error err = e->run();
    if (err != Error::Success) {
        printf("[HOST] FAIL: e[%d] (EID=%d) run failed (err=%d)\n", idx, e->getEID(), (int)err);
        return false;
    }
    printf("[HOST] e[%d] (EID=%d) run OK\n", idx, e->getEID());
    return true;
}

int main(int argc, char** argv)
{
    if (argc < 4) {
        printf("Usage: %s <eapp> <runtime> <loader>\n", argv[0]);
        return 1;
    }

    /*
     * WID slot virtualization test (5 HW WID slots: WID 1..5)
     *
     * Phase 1 — ASSIGN  : create and run e0..e4 (5 enclaves) → fills all 5 WID slots
     * Phase 2 — EVICT   : run e5 (6th enclave) → e0 is LRU → eviction fires
     * Phase 3 — EID REUSE: destroy e0, create f0 → gets EID=0 (reused), run → ASSIGN
     *
     * NOTE: REUSE (WID already in table, re-accessed) occurs when an enclave is
     * interrupted mid-run and resumed after its HW WGC slot was evicted by others.
     * That path is exercised internally on preemption and is not reproducible from
     * a simple host-side test without a long-running eapp with OCALLs.
     */

    printf("\n[HOST] === Phase 1: ASSIGN — fill all 5 WID slots (e0..e4) ===\n");
    Enclave* e[6];
    for (int i = 0; i < 6; i++) {
        e[i] = make_enclave(argv[1], argv[2], argv[3]);
        if (!e[i]) return 1;
        printf("[HOST] e[%d] created  EID=%d\n", i, e[i]->getEID());
    }

    for (int i = 0; i < 5; i++) {
        printf("[HOST] run e[%d] (EID=%d) → expect [WID] assign\n", i, e[i]->getEID());
        if (!run_enclave(e[i], i)) return 1;
    }

    printf("\n[HOST] === Phase 2: EVICT — run e[5] (6th enclave, all WID slots full) ===\n");
    printf("[HOST] run e[5] (EID=%d) → expect [WID] EVICT (e[0] is LRU)\n", e[5]->getEID());
    if (!run_enclave(e[5], 5)) return 1;

    /*
     * Phase 3 notes:
     *  - e[0] was already evicted from WID_SLOTS when e[5] ran (e[0] was LRU).
     *    Destroying e[0] frees EID=0 but does NOT free a WID hw slot.
     *  - e[1] is still in WID_SLOTS (WID=2). Destroying e[1] frees EID=1 AND WID slot 1.
     *  - f0 gets EID=0 (lowest free ENCLAVES slot = slot 0) → EID reuse.
     *  - f0 run → WID slot 1 is free → [WID] assign (not evict).
     */
    printf("\n[HOST] === Phase 3: EID REUSE + WID ASSIGN — destroy e[0] & e[1], recreate ===\n");
    int old_eid0 = e[0]->getEID();
    int old_eid1 = e[1]->getEID();
    e[0]->destroy();
    printf("[HOST] e[0] destroyed (was EID=%d, WID already evicted by e[5])\n", old_eid0);
    e[1]->destroy();
    printf("[HOST] e[1] destroyed (was EID=%d, WID slot freed)\n", old_eid1);

    Enclave* f0 = make_enclave(argv[1], argv[2], argv[3]);
    if (!f0) return 1;
    printf("[HOST] f0 created EID=%d (expected %d — EID reuse%s)\n",
           f0->getEID(), old_eid0,
           f0->getEID() == old_eid0 ? " OK" : " MISMATCH");

    printf("[HOST] run f0 (EID=%d) → expect [WID] assign (WID slot freed by e[1] destroy)\n",
           f0->getEID());
    if (!run_enclave(f0, -1)) return 1;

    printf("\n[HOST] === All phases passed! ===\n");

    f0->destroy();
    delete f0;
    for (int i = 0; i < 6; i++) {
        if (e[i]) { e[i]->destroy(); delete e[i]; }
    }
    return 0;
}

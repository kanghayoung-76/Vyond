//******************************************************************************
// WorldGuard paper eval #2 — memory access latency under WID enforcement.
//
// Tightest micro-measurement from inside an enclave (U-mode eapp). Each timed
// region contains WGC_UNROLL memory ops emitted back-to-back as inline asm
// (no loop => no branch/counter overhead), bracketed by inlined rdcycle:
//
//   ld block:  rdcycle; (ld p,0(p)) x100; rdcycle       <- dependent chain
//   sd block:  rdcycle; (addrgen; sd x0,0(addr)) x100; rdcycle
//
//   - cache tier (8 KiB < L1D): hits -> WID tag check only, WGChecker bypassed.
//   - DRAM tier (4 MiB > L2): misses -> refill traverses the WGChecker
//     (D$ blocking, nMSHRs=0, so the miss stalls and the timing captures it).
//
// per-op(ld) = (block - ld_bracket)/UNROLL           ld_bracket = 2x rdcycle
// per-op(sd) = (block - sd_bracket)/UNROLL   sd_bracket = same block w/o the sd
// (address-gen + rdcycle bracket cancel), isolating the store instruction.
//
// The host takes the geometric mean over WGC_REPEAT blocks. Requires
// scounteren.CY (eyrie boot.c) so U-mode rdcycle does not trap.
//******************************************************************************
#include "eapp_utils.h"
#include "edge_call.h"
#include <syscall.h>
#include <stdint.h>

#include "wgc_mem_bench.h"

// ---------------------------------------------------------------------------
// Timer source for every timed region. Default = rdcycle (CPU cycle count).
// For wall-clock LATENCY instead of cycles, comment the rdcycle line and
// uncomment the rdtime line (rdtime = fixed-frequency mtime counter; divide the
// result by the timebase frequency to get seconds). Both read identically;
// only the CSR differs. rdtime in enclave U-mode needs scounteren.TM, which
// eyrie boot.c enables (scounteren = 0x7 = CY|TM|IR).
// ---------------------------------------------------------------------------
#define READ_TIMER(dst) asm volatile("rdcycle %0" : "=r"(dst) ::"memory")
// #define READ_TIMER(dst) asm volatile("rdtime %0"  : "=r"(dst) ::"memory")

#define LINE 64
#define STRIDE_LINES 1009
#define STRIDE (STRIDE_LINES * LINE)

#define TIER_CACHE_BYTES (8UL * 1024)          /* < 16 KiB L1D, power of two */
#define TIER_DRAM_BYTES  (4UL * 1024 * 1024)   /* > 512 KiB L2, power of two */

/* Emit `s` exactly 100 times (10 x 10). */
#define R5(s)   s s s s s
#define R10(s)  R5(s) R5(s)
#define R100(s) R10(R10(s))

static const uint64_t TIER_BYTES[WGC_NTIER] = {
    TIER_CACHE_BYTES, TIER_DRAM_BYTES};

static uint8_t buffer[TIER_DRAM_BYTES] __attribute__((aligned(LINE)));
static volatile uint64_t sink;

unsigned long ocall_print_result(struct wgc_mem_result *r);

/* Each node's first 8 bytes hold the ABSOLUTE address of the next node, so a
 * dependent load chain is simply `ld p, 0(p)`. */
static void build_chase(uint64_t size) {
    uint64_t n = size / LINE;
    for (uint64_t i = 0; i < n; i++)
        *(uint64_t **)(buffer + i * LINE) =
            (uint64_t *)(buffer + ((i + STRIDE_LINES) % n) * LINE);
}

static uint64_t warm_pass(uint64_t size) {
    uint64_t n = size / LINE;
    uint64_t *p = (uint64_t *)buffer;
    for (uint64_t k = 0; k < n; k++)
        p = (uint64_t *)(*p);
    sink = (uint64_t)p;
    return (uint64_t)p;
}

/* min cost of two back-to-back rdcycles (the ld timing bracket). */
static uint64_t measure_ld_bracket(void) {
    uint64_t best = ~0UL;
    for (int i = 0; i < 1000; i++) {
        uint64_t a, b;
        READ_TIMER(a);
        READ_TIMER(b);
        if (b - a < best) best = b - a;
    }
    return best;
}

/* min cost of UNROLL address-gen sequences WITHOUT the sd (isolates sd later).*/
static uint64_t measure_sd_bracket(uint64_t base, uint64_t stride, uint64_t mask) {
    uint64_t best = ~0UL;
    for (int i = 0; i < 200; i++) {
        uint64_t off = 0, tmp, t0, t1;
        READ_TIMER(t0);
        asm volatile(
            R100("add %1, %2, %0\n\t"   /* tmp = base + off */
                 "add %0, %0, %3\n\t"   /* off += stride    */
                 "and %0, %0, %4\n\t")  /* off &= mask       */
            "fence\n\t"                 /* match store block's trailing fence */
            : "+r"(off), "=&r"(tmp)
            : "r"(base), "r"(stride), "r"(mask)
            : "memory");
        READ_TIMER(t1);
        if (t1 - t0 < best) best = t1 - t0;
    }
    return best;
}

unsigned long ocall_print_result(struct wgc_mem_result *r) {
    unsigned long ret;
    ocall(OCALL_PRINT_RESULT, r, sizeof(*r), &ret, sizeof(ret));
    return ret;
}

int main(void) {
    struct wgc_mem_result r;
    for (unsigned i = 0; i < sizeof(r); i++)
        ((volatile uint8_t *)&r)[i] = 0;
    r.ntier = WGC_NTIER;
    r.unroll = WGC_UNROLL;
    r.repeat = WGC_REPEAT;

    const uint64_t base = (uint64_t)buffer;
    r.ld_bracket = measure_ld_bracket();
    r.sd_bracket = measure_sd_bracket(base, STRIDE, TIER_DRAM_BYTES - 1);

    for (int t = 0; t < WGC_NTIER; t++) {
        uint64_t size = TIER_BYTES[t];
        uint64_t mask = size - 1;
        r.bytes[t] = size;

        /* --- ld: dependent chain over the working set --- */
        build_chase(size);
        uint64_t p0 = warm_pass(size);        /* untimed: pages + cache state */
        uint64_t p = p0;
        for (int i = 0; i < WGC_REPEAT; i++) {
            uint64_t t0, t1;
            READ_TIMER(t0);
            asm volatile(R100("ld %0, 0(%0)\n\t") : "+r"(p)::"memory");
            READ_TIMER(t1);
            r.ld_cyc[t][i] = t1 - t0;
        }
        sink = p;

        /* --- sd: store while striding across the working set (add/and) ---
         * (overwrites the chain buffer, so this MUST run after the ld pass). */
        uint64_t off = 0, tmp;
        for (int i = 0; i < WGC_REPEAT; i++) {
            uint64_t t0, t1;
            READ_TIMER(t0);
            asm volatile(
                R100("add %1, %2, %0\n\t"   /* tmp = base + off */
                     "sd  x0, 0(%1)\n\t"    /* store            */
                     "add %0, %0, %3\n\t"   /* off += stride    */
                     "and %0, %0, %4\n\t")  /* off &= mask       */
                "fence\n\t"                 /* drain all 100 stores before t1 */
                : "+r"(off), "=&r"(tmp)
                : "r"(base), "r"(STRIDE), "r"(mask)
                : "memory");
            READ_TIMER(t1);
            r.sd_cyc[t][i] = t1 - t0;
        }
        sink = off;
    }

    ocall_print_result(&r);
    EAPP_RETURN(0);
}

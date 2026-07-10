//******************************************************************************
// Shared eapp<->host layout for WorldGuard paper eval #2 (memory access
// latency under WID enforcement), read (ld) and write (sd).
//
// Tightest micro-measurement: each timed region is WGC_UNROLL memory ops
// emitted back-to-back as inline asm (NO loop => no branch/counter overhead),
// bracketed by inlined rdcycle. The block is repeated WGC_REPEAT times; the
// host takes the geometric mean of the per-op cycles.
//
//   ld: WGC_UNROLL dependent `ld` (address = previous load's result).
//       per-op = (block - ld_bracket) / UNROLL       [ld_bracket = 2x rdcycle]
//   sd: WGC_UNROLL `sd` interleaved with address-gen (add/and) since a store
//       returns no address to chain. The identical address-gen WITHOUT the sd
//       is timed as sd_bracket, so per-op = (block - sd_bracket)/UNROLL isolates
//       the store instruction (address-gen + rdcycle bracket cancel out).
//******************************************************************************
#ifndef WGC_MEM_BENCH_H
#define WGC_MEM_BENCH_H

#include <stdint.h>

#define OCALL_PRINT_RESULT 1

#define WGC_NTIER  2      /* 0 = cache (L1-fit), 1 = DRAM        */
#define WGC_UNROLL 100    /* memory ops per timed block (no loop) */
#define WGC_REPEAT 100    /* timed blocks per tier (host geomean) */

struct wgc_mem_result {
    uint64_t ld_bracket;                     /* min 2x rdcycle                    */
    uint64_t sd_bracket;                     /* min UNROLL x addr-gen (no sd)      */
    uint32_t ntier;                          /* == WGC_NTIER                       */
    uint32_t unroll;                         /* == WGC_UNROLL                      */
    uint32_t repeat;                         /* == WGC_REPEAT                      */
    uint32_t _pad;
    uint64_t bytes[WGC_NTIER];               /* working-set size per tier          */
    uint64_t ld_cyc[WGC_NTIER][WGC_REPEAT];  /* raw block delta, UNROLL loads      */
    uint64_t sd_cyc[WGC_NTIER][WGC_REPEAT];  /* raw block delta, UNROLL stores     */
};

#endif /* WGC_MEM_BENCH_H */

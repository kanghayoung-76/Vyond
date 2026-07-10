//******************************************************************************
// WorldGuard paper eval #2 — BASELINE (plain Linux, no enclave).
//
// Same tightest micro-measurement as the enclave eapp (WGC_UNROLL memory ops
// unrolled per timed block, no loop), built as an ordinary Linux/BusyBox binary
// so the numbers compare directly:
//   - WG bitstream, normal process -> latency under OS_WID (WID+checker on)
//   - no-WG Rocket bitstream        -> true "WID-check-off" baseline
//
//        |   cache (hit)        |   DRAM (miss)
//  base  |  raw hit             |  raw DRAM
//  WG    |  hit + WID cmp        |  DRAM + WGChecker      (ld and sd separately)
//
// per-op(ld) = (block - ld_bracket)/UNROLL ; ld_bracket = 2x rdcycle
// per-op(sd) = (block - sd_bracket)/UNROLL ; sd_bracket = same block minus sd
// Reported values are the geometric mean over REPEAT blocks.
//******************************************************************************
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <math.h>

// ---------------------------------------------------------------------------
// Timer source for every timed region. Default = rdcycle (CPU cycle count).
// For wall-clock LATENCY instead of cycles, comment the rdcycle line and
// uncomment the rdtime line (rdtime = fixed-frequency mtime counter; divide the
// result by the timebase frequency to get seconds). Both read identically;
// only the CSR differs. In Linux userspace both are enabled by the kernel.
// ---------------------------------------------------------------------------
#define READ_TIMER(dst) asm volatile("rdcycle %0" : "=r"(dst) ::"memory")
// #define READ_TIMER(dst) asm volatile("rdtime %0"  : "=r"(dst) ::"memory")

#define LINE 64
#define STRIDE_LINES 1009
#define STRIDE (STRIDE_LINES * LINE)
#define NTIER 2
#define UNROLL 100
#define REPEAT 100

#define TIER_CACHE_BYTES (8UL * 1024)
#define TIER_DRAM_BYTES  (4UL * 1024 * 1024)

#define R5(s)   s s s s s
#define R10(s)  R5(s) R5(s)
#define R100(s) R10(R10(s))

static const uint64_t TIER_BYTES[NTIER] = {TIER_CACHE_BYTES, TIER_DRAM_BYTES};
static const char *TIER_NAME[NTIER] = {"cache", "DRAM"};
static const char *TIER_PATH[NTIER] = {"hit (no checker)", "miss->DRAM (+WGChecker)"};

static uint8_t *buffer;
static volatile uint64_t sink;

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

static uint64_t measure_sd_bracket(uint64_t base, uint64_t stride, uint64_t mask) {
    uint64_t best = ~0UL;
    for (int i = 0; i < 200; i++) {
        uint64_t off = 0, tmp, t0, t1;
        READ_TIMER(t0);
        asm volatile(
            R100("add %1, %2, %0\n\t"
                 "add %0, %0, %3\n\t"
                 "and %0, %0, %4\n\t")
            "fence\n\t"                 /* match store block's trailing fence */
            : "+r"(off), "=&r"(tmp)
            : "r"(base), "r"(stride), "r"(mask)
            : "memory");
        READ_TIMER(t1);
        if (t1 - t0 < best) best = t1 - t0;
    }
    return best;
}

static double geomean_per_op(const uint64_t *cyc, int repeat,
                             uint64_t bracket, int unroll) {
    double s = 0.0;
    int n = 0;
    for (int i = 0; i < repeat; i++) {
        double net = ((double)cyc[i] - (double)bracket) / (double)unroll;
        if (net < 0.01) net = 0.01;
        s += log(net);
        n++;
    }
    return n ? exp(s / n) : 0.0;
}

int main(void) {
    if (posix_memalign((void **)&buffer, LINE, TIER_DRAM_BYTES) != 0 || !buffer) {
        fprintf(stderr, "alloc failed\n");
        return 1;
    }
    memset(buffer, 0, TIER_DRAM_BYTES);

    const uint64_t base = (uint64_t)buffer;
    uint64_t ld_bracket = measure_ld_bracket();
    uint64_t sd_bracket = measure_sd_bracket(base, STRIDE, TIER_DRAM_BYTES - 1);

    printf("\n");
    printf("== memory access latency (BASELINE / native)  [unroll=%d, blocks=%d,"
           " brackets ld=%lu sd=%lu] ==\n",
           UNROLL, REPEAT, (unsigned long)ld_bracket, (unsigned long)sd_bracket);
    printf(" tier    working-set   ld cyc/access   sd cyc/access   path\n");

    for (int t = 0; t < NTIER; t++) {
        uint64_t size = TIER_BYTES[t];
        uint64_t mask = size - 1;
        uint64_t ld_cyc[REPEAT], sd_cyc[REPEAT];

        build_chase(size);
        uint64_t p = warm_pass(size);
        for (int i = 0; i < REPEAT; i++) {
            uint64_t t0, t1;
            READ_TIMER(t0);
            asm volatile(R100("ld %0, 0(%0)\n\t") : "+r"(p)::"memory");
            READ_TIMER(t1);
            ld_cyc[i] = t1 - t0;
        }
        sink = p;

        uint64_t off = 0, tmp;
        for (int i = 0; i < REPEAT; i++) {
            uint64_t t0, t1;
            READ_TIMER(t0);
            asm volatile(
                R100("add %1, %2, %0\n\t"
                     "sd  x0, 0(%1)\n\t"
                     "add %0, %0, %3\n\t"
                     "and %0, %0, %4\n\t")
                "fence\n\t"                 /* drain all 100 stores before t1 */
                : "+r"(off), "=&r"(tmp)
                : "r"(base), "r"(STRIDE), "r"(mask)
                : "memory");
            READ_TIMER(t1);
            sd_cyc[i] = t1 - t0;
        }
        sink = off;

        double gl = geomean_per_op(ld_cyc, REPEAT, ld_bracket, UNROLL);
        double gs = geomean_per_op(sd_cyc, REPEAT, sd_bracket, UNROLL);
        printf(" %-6s  %9lu B   %11.2f   %13.2f   %s\n",
               TIER_NAME[t], (unsigned long)size, gl, gs, TIER_PATH[t]);
    }
    printf("=========================================================================\n\n");

    free(buffer);
    return 0;
}

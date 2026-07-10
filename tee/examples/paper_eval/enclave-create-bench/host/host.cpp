//******************************************************************************
// WorldGuard paper eval #4 — enclave create/destroy latency (host-observed).
//
// Times enclave.init() (create: EPM alloc + load loader/runtime/eapp + SBI
// finalize + measurement hash) and enclave.destroy() (SBI destroy + EPM scrub)
// with host rdcycle, over many create/destroy cycles (no run()). Reports the
// geometric mean plus min/max per phase.
//
// Note: since the SM now hashes at create (fair with Keystone), the create
// number includes measurement; destroy includes the EPM scrub. Both scale with
// enclave/EPM size, so pass freememKB to sweep.
//******************************************************************************
#include <keystone.h>
#include <cstdio>
#include <cstdint>
#include <cstdlib>
#include <cmath>

#define WARMUP 5
#define REPEAT 100

// Timer source. Default = rdcycle (CPU cycle count). For wall-clock LATENCY
// instead of cycles, comment the rdcycle line and uncomment the rdtime line
// (rdtime = fixed-frequency mtime counter; divide by the timebase frequency to
// get seconds). Both read identically; only the CSR differs.
static inline uint64_t read_timer(void) {
  uint64_t c;
  asm volatile("rdcycle %0" : "=r"(c));   // CPU cycles (default)
  // asm volatile("rdtime %0" : "=r"(c)); // wall-clock ticks (uncomment for latency)
  return c;
}

static double geomean(const uint64_t* v, int n) {
  double s = 0.0;
  int m = 0;
  for (int i = 0; i < n; i++) {
    if (!v[i]) continue;
    s += std::log((double)v[i]);
    m++;
  }
  return m ? std::exp(s / m) : 0.0;
}
static uint64_t vmin(const uint64_t* v, int n) {
  uint64_t m = ~0UL;
  for (int i = 0; i < n; i++) if (v[i] < m) m = v[i];
  return m;
}
static uint64_t vmax(const uint64_t* v, int n) {
  uint64_t m = 0;
  for (int i = 0; i < n; i++) if (v[i] > m) m = v[i];
  return m;
}

int main(int argc, char** argv) {
  if (argc < 4) {
    printf("usage: %s <eapp> <runtime> <loader> [freememKB]\n", argv[0]);
    return 1;
  }
  const char* eapp = argv[1];
  const char* rt   = argv[2];
  const char* ld   = argv[3];
  size_t freemem = (size_t)((argc > 4) ? atoi(argv[4]) : 512) * 1024;
  size_t utm     = 256 * 1024;

  uint64_t create_cyc[REPEAT], destroy_cyc[REPEAT];

  for (int i = 0; i < WARMUP + REPEAT; i++) {
    Keystone::Params params;
    params.setFreeMemSize(freemem);
    params.setUntrustedSize(utm);

    Keystone::Enclave enclave;

    uint64_t t0 = read_timer();
    Keystone::Error err = enclave.init(eapp, rt, ld, params);
    uint64_t t1 = read_timer();
    enclave.destroy();
    uint64_t t2 = read_timer();

    if (err != Keystone::Error::Success) {
      printf("[create-bench] init failed (iter %d)\n", i);
      return 1;
    }
    if (i >= WARMUP) {
      create_cyc[i - WARMUP]  = t1 - t0;
      destroy_cyc[i - WARMUP] = t2 - t1;
    }
    // enclave destructor runs here (redundant destroy(), harmless, untimed)
  }

  printf("\n== enclave create/destroy latency  [samples=%d, freemem=%zuKB] ==\n",
         REPEAT, freemem / 1024);
  printf(" op        geomean        min        max    (cycles)\n");
  printf(" create   %9.0f  %9lu  %9lu\n",
         geomean(create_cyc, REPEAT), (unsigned long)vmin(create_cyc, REPEAT),
         (unsigned long)vmax(create_cyc, REPEAT));
  printf(" destroy  %9.0f  %9lu  %9lu\n",
         geomean(destroy_cyc, REPEAT), (unsigned long)vmin(destroy_cyc, REPEAT),
         (unsigned long)vmax(destroy_cyc, REPEAT));
  printf("============================================================\n\n");
  return 0;
}

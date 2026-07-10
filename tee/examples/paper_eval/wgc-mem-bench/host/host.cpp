//******************************************************************************
// Host runner for WorldGuard paper eval #2. The enclave ships per-run cycle
// counts; the host reports the geometric-mean cyc/access per tier.
//******************************************************************************
#include <edge_call.h>
#include <keystone.h>
#include <cstdio>
#include <cstring>
#include <cmath>

#include "wgc_mem_bench.h"

void print_result_wrapper(void* buffer);

// Geometric mean of per-op cycles = (block delta - bracket)/unroll over blocks.
static double geomean_per_op(const uint64_t* cyc, uint32_t repeat,
                             uint64_t bracket, uint32_t unroll) {
  double s = 0.0;
  uint32_t n = 0;
  for (uint32_t i = 0; i < repeat; i++) {
    double net = ((double)cyc[i] - (double)bracket) / (double)unroll;
    if (net < 0.01) net = 0.01;   // guard log() at the tiny (L1-hit) end
    s += std::log(net);
    n++;
  }
  return n ? std::exp(s / n) : 0.0;
}

static void print_result(const struct wgc_mem_result* r) {
  const char* names[WGC_NTIER] = {"cache", "DRAM"};
  const char* path[WGC_NTIER]  = {"hit (WID cmp, no checker)",
                                  "miss->DRAM (+WGChecker)"};
  printf("\n");
  printf("== memory access latency (WG / enclave WID)  [unroll=%u, blocks=%u,"
         " brackets ld=%lu sd=%lu] ==\n",
         r->unroll, r->repeat,
         (unsigned long)r->ld_bracket, (unsigned long)r->sd_bracket);
  printf(" tier    working-set   ld cyc/access   sd cyc/access   path\n");
  for (uint32_t t = 0; t < r->ntier && t < WGC_NTIER; t++) {
    double gl = geomean_per_op(r->ld_cyc[t], r->repeat, r->ld_bracket, r->unroll);
    double gs = geomean_per_op(r->sd_cyc[t], r->repeat, r->sd_bracket, r->unroll);
    printf(" %-6s  %9lu B   %11.2f   %13.2f   %s\n",
           names[t], (unsigned long)r->bytes[t], gl, gs, path[t]);
  }
  printf("========================================================================\n\n");
}

int main(int argc, char** argv) {
  Keystone::Enclave enclave;
  Keystone::Params params;

  params.setFreeMemSize(32 * 1024 * 1024);  // > DRAM tier (4 MiB) + headroom
  params.setUntrustedSize(1 * 1024 * 1024);

  enclave.init(argv[1], argv[2], argv[3], params);

  enclave.registerOcallDispatch(incoming_call_dispatch);
  register_call(OCALL_PRINT_RESULT, print_result_wrapper);

  edge_call_init_internals(
      (uintptr_t)enclave.getSharedBuffer(), enclave.getSharedBufferSize());

  enclave.run();

  return 0;
}

void print_result_wrapper(void* buffer) {
  struct edge_call* edge_call = (struct edge_call*)buffer;
  uintptr_t call_args;
  size_t arg_len;
  if (edge_call_args_ptr(edge_call, &call_args, &arg_len) != 0) {
    edge_call->return_data.call_status = CALL_STATUS_BAD_OFFSET;
    return;
  }
  if (arg_len < sizeof(struct wgc_mem_result)) {
    edge_call->return_data.call_status = CALL_STATUS_BAD_PTR;
    return;
  }

  print_result((const struct wgc_mem_result*)call_args);

  unsigned long ret_val = 0;
  uintptr_t data_section = edge_call_data_ptr();
  memcpy((void*)data_section, &ret_val, sizeof(unsigned long));
  if (edge_call_setup_ret(edge_call, (void*)data_section, sizeof(unsigned long))) {
    edge_call->return_data.call_status = CALL_STATUS_BAD_PTR;
  } else {
    edge_call->return_data.call_status = CALL_STATUS_OK;
  }
}

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

/* 2026-07-28 포팅: 구 edge_call API(edge_call_init_internals/register_call/
 * 2-인자 edge_call_args_ptr)가 제거되어 빌드가 깨져 있었다. hello-native·shm-ocall-test와
 * 동일한 방식으로, 공유버퍼 base/size를 로컬에 보관하고 OCALL을 switch로 디스패치한다. */
static uintptr_t shm_base = 0;
static size_t    shm_size = 0;

static void ocall_dispatch(void* buffer, size_t /*size*/) {
  struct edge_call* ec = (struct edge_call*)buffer;
  switch (ec->call_id) {
    case OCALL_PRINT_RESULT: print_result_wrapper(buffer); break;
    default:
      printf("[HOST] unknown OCALL id=%lu\n", (unsigned long)ec->call_id);
      ec->return_data.call_status = CALL_STATUS_BAD_CALL_ID;
  }
}

int main(int argc, char** argv) {
  if (argc < 4) {
    fprintf(stderr, "usage: %s <eapp> <runtime> <loader>\n", argv[0]);
    return 1;
  }

  Keystone::Enclave enclave;
  Keystone::Params params;

  /* [2026-07-30] 32 MiB -> 2 MiB. EPM 은 loader+runtime+eapp(BSS 1 MiB 버퍼 포함)+freemem
   * 을 모두 담아야 하고, 커널 버디 상한이 order 10(4 MiB)이다. 32 MiB 요청은 order 14 가
   * 되어 __alloc_pages 가 WARNING 과 함께 실패했다. */
  params.setFreeMemSize(2 * 1024 * 1024);   // > DRAM tier (1 MiB) + headroom
  params.setUntrustedSize(1 * 1024 * 1024);

  Keystone::Error err = enclave.init(argv[1], argv[2], argv[3], params);
  printf("[TRACE][APP] init() -> %d (0=Success)\n", (int)err);
  if (err != Keystone::Error::Success) return 1;

  shm_base = (uintptr_t)enclave.getSharedBuffer();
  shm_size = enclave.getSharedBufferSize();
  printf("[HOST] wgc-mem-bench: SHM base=0x%lx size=%zu\n", shm_base, shm_size);

  enclave.registerOcallDispatch(
      [](void* b, size_t s) { ocall_dispatch(b, s); });

  printf("[HOST] Running enclave...\n");
  enclave.run();
  printf("[HOST] Enclave finished.\n");

  return 0;
}

void print_result_wrapper(void* buffer) {
  struct edge_call* ec = (struct edge_call*)buffer;
  uintptr_t call_args;
  size_t arg_len = ec->call_arg_size;
  /* 현 API: 오프셋 → 포인터 변환에 공유버퍼 base/size를 명시적으로 넘긴다. */
  if (edge_call_get_ptr_from_offset(ec->call_arg_offset, ec->call_arg_size,
                                    &call_args, shm_base, shm_size) != 0) {
    ec->return_data.call_status = CALL_STATUS_BAD_OFFSET;
    return;
  }
  if (arg_len < sizeof(struct wgc_mem_result)) {
    ec->return_data.call_status = CALL_STATUS_BAD_PTR;
    return;
  }

  print_result((const struct wgc_mem_result*)call_args);

  unsigned long ret_val = 0;
  uintptr_t ret_area = shm_base + sizeof(struct edge_call);
  memcpy((void*)ret_area, &ret_val, sizeof(unsigned long));
  if (edge_call_setup_ret(ec, (void*)ret_area, sizeof(unsigned long),
                          shm_base, shm_size) != 0) {
    ec->return_data.call_status = CALL_STATUS_BAD_PTR;
  } else {
    ec->return_data.call_status = CALL_STATUS_OK;
  }
}

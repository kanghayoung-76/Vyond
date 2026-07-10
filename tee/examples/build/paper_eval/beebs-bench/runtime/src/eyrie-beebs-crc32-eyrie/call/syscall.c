//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#include <stdint.h>
#include <stddef.h>
#include <sys/select.h>
#include "call/syscall.h"
#include "util/string.h"
#include "edge_call.h"
#include "uaccess.h"
#include "mm/mm.h"
#include "util/rt_util.h"
#include "call/syscall_nums.h"
#include "keystone_user.h"

#ifdef USE_IO_SYSCALL
#include "call/io_wrap.h"
#endif /* USE_IO_SYSCALL */

#ifdef USE_LINUX_SYSCALL
#include "call/linux_wrap.h"
#endif /* USE_LINUX_SYSCALL */

#ifdef USE_NET_SYSCALL
#include "call/net_wrap.h"
#endif /* USE_NET_SYSCALL */

#define read_reg(reg)                                     \
  ({                                                      \
    unsigned long __v;                                    \
    __asm__ __volatile__("mv %0, " #reg : "=r"(__v) : :); \
    __v;                                                  \
  })

extern void exit_enclave(uintptr_t arg0);

uintptr_t dispatch_edgecall_syscall(struct edge_syscall* syscall_data_ptr, size_t data_len){
  int ret;

  // Syscall data should already be at the edge_call_data section
  /* For now we assume by convention that the start of the buffer is
   * the right place to put calls */
  struct edge_call* edge_call = (struct edge_call*)shared_buffer;

  edge_call->call_id = EDGECALL_SYSCALL;


  if(edge_call_setup_call(edge_call, (void*)syscall_data_ptr, data_len, shared_buffer, shared_buffer_size) != 0){
    return -1;
  }

  ret = sbi_stop_enclave(STOP_EDGE_CALL_HOST);

  if (ret != 0) {
    return -1;
  }

  if(edge_call->return_data.call_status != CALL_STATUS_OK){
    return -1;
  }

  uintptr_t return_ptr;
  size_t return_len;
  if(edge_call_ret_ptr(edge_call, &return_ptr, &return_len, shared_buffer, shared_buffer_size) != 0){
    return -1;
  }

  if(return_len < sizeof(uintptr_t)){
    return -1;
  }

  return *(uintptr_t*)return_ptr;
}

uintptr_t dispatch_edgecall_ocall( unsigned long call_id,
				   void* data, size_t data_len,
				   void* return_buffer, size_t return_len){

  uintptr_t ret;
  /* For now we assume by convention that the start of the buffer is
   * the right place to put calls */
  struct edge_call* edge_call = (struct edge_call*)shared_buffer;

  /* We encode the call id, copy the argument data into the shared
   * region, calculate the offsets to the argument data, and then
   * dispatch the ocall to host */

  edge_call->call_id = call_id;

  /* UTM is now PTE_U: eapp accesses EYRIE_UNTRUSTED_START directly.
   * data must already point into the UTM — no copy needed.
   * Skip setup_call for the no-data case (data_len==0) because ptr may be
   * NULL, which fails the UTM range check even with size=0. */
  if(data_len == 0){
    edge_call->call_arg_size   = 0;
    edge_call->call_arg_offset = 0;
  } else if(edge_call_setup_call(edge_call, (void*)data, data_len, shared_buffer, shared_buffer_size) != 0){
    goto ocall_error;
  }

  ret = sbi_stop_enclave(STOP_EDGE_CALL_HOST);

  if (ret != 0) {
    goto ocall_error;
  }

  if(edge_call->return_data.call_status != CALL_STATUS_OK){
    goto ocall_error;
  }

  if( return_len == 0 ){
    return (uintptr_t)NULL;
  }

  uintptr_t return_ptr;
  size_t ret_len_untrusted;
  if(edge_call_ret_ptr(edge_call, &return_ptr, &ret_len_untrusted, shared_buffer, shared_buffer_size) != 0){
    goto ocall_error;
  }

  /* ZEROCOPY: store UTM return-data address into return_buffer.
   * SUM=1 (set in eyrie_boot) allows this S-mode write to the U-mode stack.
   * Eapp dereferences return_buffer to get the UTM pointer, then reads the
   * payload directly from UTM (PTE_U — no memcpy involved). */
  *(uintptr_t*)return_buffer = return_ptr;

  return 0;

 ocall_error:
  /* TODO In the future, this should fault */
  return 1;
}

uintptr_t handle_copy_from_shared(void* dst, uintptr_t offset, size_t size){
  /* UTM is PTE_U: eapp accesses EYRIE_UNTRUSTED_START directly.
   * Just validate bounds and return the UTM pointer — no copy needed. */
  uintptr_t src_ptr;
  if(edge_call_get_ptr_from_offset(offset, size,
				   &src_ptr, shared_buffer, shared_buffer_size) != 0){
    return 1;
  }
  /* dst receives the UTM virtual address; eapp dereferences it directly. */
  *(uintptr_t*)dst = src_ptr;
  return 0;
}

uintptr_t shm_va_ptr = RUNTIME_SHARED_START;

static int handle_map_shm(rid_t rid, uintptr_t* ret_vaddr) {
  uintptr_t paddr, size;
  uintptr_t ret = SBI_CALL_1(SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE, SBI_SM_MAP_SHM_REGION, (uintptr_t)rid);
  if (ret) return 1;
  paddr = read_reg(a2);
  size  = read_reg(a3);

  uintptr_t va = shm_va_ptr;
  *ret_vaddr   = va;
  while (va < shm_va_ptr + size) {
    if (!map_page(vpn(va), ppn(paddr), PAGE_MODE_USER_DATA)) {
      return -1;
    }
    va += RISCV_PAGE_SIZE;
    paddr += RISCV_PAGE_SIZE;
  }

  shm_va_ptr = va;

  return 0;  // TODO: better error handling
}

static int
handle_unmap_shm(rid_t rid, uintptr_t vaddr, size_t size) {
  uintptr_t ret = SBI_CALL_1(
      SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE, SBI_SM_UNMAP_SHM_REGION,
      (uintptr_t)rid);
  if (ret) return -1;

  free_pages(vpn(vaddr), size / RISCV_PAGE_SIZE);
  return 0;
}

static int
handle_mydev_map(uintptr_t paddr, size_t size, uintptr_t* ret_vaddr) {
  uintptr_t va = shm_va_ptr;
  *ret_vaddr   = va;
  while (va < shm_va_ptr + size) {
    if (!map_page(vpn(va), ppn(paddr), PAGE_MODE_USER_DATA)) {
      return -1;
    }
    va += RISCV_PAGE_SIZE;
    paddr += RISCV_PAGE_SIZE;
  }

  shm_va_ptr = va;

  return 0;  // TODO: better error handling
}

static int
handle_mydev_unmap(uintptr_t vaddr, size_t size) {
  free_pages(vpn(vaddr), size / RISCV_PAGE_SIZE);
  return 0;
}

static int handle_map_utm(uintptr_t* ret_vaddr) {
  uintptr_t pa = translate(shared_buffer);
  uintptr_t sz = shared_buffer_size;

  uintptr_t va = shm_va_ptr;
  *ret_vaddr   = va;
  while (va < shm_va_ptr + sz) {
    if (!map_page(vpn(va), ppn(pa), PAGE_MODE_USER_DATA)) {
      return -1;
    }
    va += RISCV_PAGE_SIZE;
    pa += RISCV_PAGE_SIZE;
  }
  shm_va_ptr = va;

  /* Update shared_buffer so edge_call_setup_call validates against the
   * eapp-mapped VA range.  Both VAs alias the same physical memory. */
  shared_buffer = *ret_vaddr;
  return 0;
}

void
handle_syscall(struct encl_ctx* ctx) {
  /* Re-set SUM on every syscall entry.  copy_to_user (used in MAP_SHM,
   * GET_SHM_EIDS, etc.) clears SUM after each call.  Without this, any
   * handler that writes directly to a PTE_U address (eapp stack or UTM)
   * would fault.  SM saves/restores sstatus across stop/run, so SUM set
   * here also persists through sbi_stop_enclave → sbi_run_enclave. */
  __asm__ __volatile__("csrs sstatus, %0" :: "r"(0x40000UL) : "memory");

  uintptr_t n    = ctx->regs.a7;
  uintptr_t arg0 = ctx->regs.a0;
  uintptr_t arg1 = ctx->regs.a1;
  uintptr_t arg2 = ctx->regs.a2;
  uintptr_t arg3 = ctx->regs.a3;
  uintptr_t arg4 = ctx->regs.a4;

  // We only use arg5 in these for now, keep warnings happy.
#if defined(USE_LINUX_SYSCALL) || defined(USE_NET_SYSCALL)
  uintptr_t arg5 = ctx->regs.a5;
#endif /* IO_SYSCALL */
  uintptr_t ret = 0, ret_val = 0;

  ctx->regs.sepc += 4;

  switch (n) {
  case(RUNTIME_SYSCALL_EXIT):
    sbi_exit_enclave(arg0);
    break;
  case(RUNTIME_SYSCALL_OCALL):
    ret = dispatch_edgecall_ocall(arg0, (void*)arg1, arg2, (void*)arg3, arg4);
    break;
  case(RUNTIME_SYSCALL_SHAREDCOPY):
    ret = handle_copy_from_shared((void*)arg0, arg1, arg2);
    break;
  case(RUNTIME_SYSCALL_ATTEST_ENCLAVE):;
    copy_from_user((void*)rt_copy_buffer_2, (void*)arg1, arg2);

    ret = sbi_attest_enclave(rt_copy_buffer_1, rt_copy_buffer_2, arg2);

    /* TODO we consistently don't have report size when we need it */
    copy_to_user((void*)arg0, (void*)rt_copy_buffer_1, 2048);
    //print_strace("[ATTEST] p1 0x%p->0x%p p2 0x%p->0x%p sz %lx = %lu\r\n",arg0,arg0_trans,arg1,arg1_trans,arg2,ret);
    break;
  case(RUNTIME_SYSCALL_GET_SEALING_KEY):;
    /* Stores the key receive structure */
    uintptr_t buffer_1_pa = translate((uintptr_t) rt_copy_buffer_1);

    /* Stores the key identifier */
    uintptr_t buffer_2_pa = translate((uintptr_t) rt_copy_buffer_2);

    if (arg1 > sizeof(rt_copy_buffer_1) ||
        arg3 > sizeof(rt_copy_buffer_2)) {
      ret = -1;
      break;
    }

    copy_from_user(rt_copy_buffer_2, (void *)arg2, arg3);

    ret = sbi_get_sealing_key(buffer_1_pa, buffer_2_pa, arg3);

    if (!ret) {
      copy_to_user((void *)arg0, (void *)rt_copy_buffer_1, arg1);
    }

    /* Delete key from copy buffer */
    memset(rt_copy_buffer_1, 0x00, sizeof(rt_copy_buffer_1));

    break;

      break;
    case (RUNTIME_SYSCALL_MAP_SHM):
      ret = handle_map_shm((rid_t)arg0, &ret_val);
      copy_to_user((void*)arg1, &ret_val, sizeof(ret_val));
      break;
    case (RUNTIME_SYSCALL_UNMAP_SHM):
      ret = handle_unmap_shm((rid_t)arg0, (uintptr_t)arg2, (size_t)arg3);
      break;
    case (RUNTIME_SYSCALL_MYDEV_MAP):
      ret = handle_mydev_map((uintptr_t)arg0, (size_t)arg1, &ret_val);
      copy_to_user((void*)arg2, &ret_val, sizeof(ret_val));
      break;
    case (RUNTIME_SYSCALL_MYDEV_UNMAP):
      ret = handle_mydev_unmap((uintptr_t)arg0, (size_t)arg1);
      break;
    case (RUNTIME_SYSCALL_REGISTER_DEV_IRQ):
      ret = sbi_register_dev_irq((uint32_t)arg0);
      break;
    case (RUNTIME_SYSCALL_WAIT_DEV_DATA):
      ret = sbi_wait_dev_data((uint32_t)arg0);
      break;
    case (RUNTIME_SYSCALL_WAIT_SHM):
      ret = sbi_wait_shm((uint32_t)arg0);
      break;
    case (RUNTIME_SYSCALL_NOTIFY_SHM):
      ret = sbi_notify_shm((uint32_t)arg0);
      break;
    case (RUNTIME_SYSCALL_TRANSLATE_VA):
      ret = translate((uintptr_t)arg0);
      break;
    case (RUNTIME_SYSCALL_MAP_UTM):
      /* Map UTM pages into enclave VA (like map_shm).  Updates shared_buffer
       * so edge_call_setup_call validates against the eapp-accessible VA.
       * Direct writes — SUM=1 is guaranteed by the csrs at handle_syscall entry. */
      ret = handle_map_utm(&ret_val);
      *(uintptr_t*)arg0 = ret_val;
      *(uintptr_t*)arg1 = shared_buffer_size;
      break;
    case (RUNTIME_SYSCALL_GET_SHM_EIDS): {
      /* arg0=rid, arg1=eids_out (user VA), arg2=max_count, arg3=count_out (user VA) */
      uintptr_t buf_pa = translate((uintptr_t)rt_copy_buffer_1);
      size_t max = (size_t)arg2;
      if (max > sizeof(rt_copy_buffer_1) / sizeof(uintptr_t))
        max = sizeof(rt_copy_buffer_1) / sizeof(uintptr_t);
      ret = SBI_CALL_3(SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE,
                       SBI_SM_GET_SHM_EIDS,
                       (uintptr_t)arg0, buf_pa, (uintptr_t)max);
      if (!ret) {
        register uintptr_t a1 __asm__("a1");  /* count returned by SM */
        uintptr_t count = a1;
        copy_to_user((void*)arg1, rt_copy_buffer_1, count * sizeof(uintptr_t));
        copy_to_user((void*)arg3, &count, sizeof(uintptr_t));
      }
      break;
    }
    case (RUNTIME_SYSCALL_REGISTER_ENC_CHANNEL): {
      /* arg0=rid, arg1=allowed_hash VA (64 bytes)
       * Copy hash from eapp VA into rt_copy_buffer_1, translate to PA, call SM. */
      copy_from_user(rt_copy_buffer_1, (void*)arg1, 64);
      uintptr_t hash_pa = translate((uintptr_t)rt_copy_buffer_1);
      ret = SBI_CALL_2(SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE,
                       SBI_SM_REGISTER_ENC_CHANNEL,
                       (uintptr_t)arg0, hash_pa);
      break;
    }
    case (RUNTIME_SYSCALL_FIND_SHM_BY_HASH): {
      /* arg0=creator_hash VA (64 bytes), arg1=rid_out VA (sizeof(rid_t)) */
      copy_from_user(rt_copy_buffer_1, (void*)arg0, 64);
      uintptr_t hash_pa    = translate((uintptr_t)rt_copy_buffer_1);
      uintptr_t rid_out_pa = translate((uintptr_t)rt_copy_buffer_2);
      ret = SBI_CALL_2(SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE,
                       SBI_SM_FIND_SHM_BY_HASH,
                       hash_pa, rid_out_pa);
      if (!ret) {
        copy_to_user((void*)arg1, rt_copy_buffer_2, sizeof(uintptr_t));
      }
      break;
    }
    case (RUNTIME_SYSCALL_GET_MY_HASH): {
      /* arg0=hash_out VA (64 bytes)
       * Use rt_copy_buffer_1 as landing buffer, translate to PA, call SM,
       * then copy 64 bytes back to eapp VA. */
      uintptr_t hash_out_pa = translate((uintptr_t)rt_copy_buffer_1);
      ret = SBI_CALL_1(SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE,
                       SBI_SM_GET_MY_HASH,
                       hash_out_pa);
      if (!ret) {
        copy_to_user((void*)arg0, rt_copy_buffer_1, 64);
      }
      break;
    }
    case (RUNTIME_SYSCALL_FIND_DEV_SHM): {
      /* arg0=rid_out VA (sizeof(rid_t))
       * Use rt_copy_buffer_1 as landing buffer for the rid. */
      uintptr_t rid_out_pa = translate((uintptr_t)rt_copy_buffer_1);
      ret = SBI_CALL_1(SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE,
                       SBI_SM_FIND_DEV_SHM,
                       rid_out_pa);
      if (!ret) {
        copy_to_user((void*)arg0, rt_copy_buffer_1, sizeof(uintptr_t));
      }
      break;
    }
    case (RUNTIME_SYSCALL_TRIGGER_DEV): {
      /* arg0=device_wid (u32) */
      ret = SBI_CALL_1(SBI_EXT_EXPERIMENTAL_KEYSTONE_ENCLAVE,
                       SBI_SM_TRIGGER_DEV,
                       (uintptr_t)arg0);
      break;
    }

#ifdef USE_LINUX_SYSCALL
  case(SYS_clock_gettime):
    ret = linux_clock_gettime((__clockid_t)arg0, (struct timespec*)arg1);
    break;

  case(SYS_getrandom):
    ret = linux_getrandom((void*)arg0, (size_t)arg1, (unsigned int)arg2);
    break;

  case(SYS_rt_sigprocmask):
    ret = linux_rt_sigprocmask((int)arg0, (const sigset_t*)arg1, (sigset_t*)arg2);
    break;

  case(SYS_getpid):
    ret = linux_getpid();
    break;

  case(SYS_uname):
    ret = linux_uname((void*) arg0);
    break;

  case(SYS_rt_sigaction):
    ret = linux_RET_ZERO_wrap(n);
    break;

  case(SYS_set_tid_address):
    ret = linux_set_tid_address((int*) arg0);
    break;

  case(SYS_brk):
    ret = syscall_brk((void*) arg0);
    break;

  case(SYS_mmap):
    ret = syscall_mmap((void*) arg0, (size_t)arg1, (int)arg2,
                       (int)arg3, (int)arg4, (__off_t)arg5);
    break;

  case(SYS_munmap):
    ret = syscall_munmap((void*) arg0, (size_t)arg1);
    break;

  case(SYS_mprotect):
    ret = syscall_mprotect((void *) arg0, (size_t) arg1, (int) arg2);
    break;

  case(SYS_exit):
  case(SYS_exit_group):
    print_strace("[runtime] exit or exit_group (%lu)\r\n",n);
    sbi_exit_enclave(arg0);
    break;
#endif /* USE_LINUX_SYSCALL */

#ifdef USE_IO_SYSCALL
  case(SYS_read):
    ret = io_syscall_read((int)arg0, (void*)arg1, (size_t)arg2);
    break;
  case(SYS_write):
    ret = io_syscall_write((int)arg0, (void*)arg1, (size_t)arg2);
    break;
  case(SYS_writev):
    ret = io_syscall_writev((int)arg0, (const struct iovec*)arg1, (int)arg2);
    break;
  case(SYS_readv):
    ret = io_syscall_readv((int)arg0, (const struct iovec*)arg1, (int)arg2);
    break;
  case(SYS_openat):
    ret = io_syscall_openat((int)arg0, (char*)arg1, (int)arg2, (mode_t)arg3);
    break;
  case(SYS_unlinkat):
    ret = io_syscall_unlinkat((int)arg0, (char*)arg1, (int)arg2);
    break;
  case(SYS_fstatat):
    ret = io_syscall_fstatat((int)arg0, (char*)arg1, (struct stat*)arg2, (int)arg3);
    break;
  case(SYS_fstat): 
    ret = io_syscall_fstat((int)arg0, (struct stat*)arg1); 
    break;
  case(SYS_lseek):
    ret = io_syscall_lseek((int)arg0, (off_t)arg1, (int)arg2);
    break;
  case(SYS_ftruncate):
    ret = io_syscall_ftruncate((int)arg0, (off_t)arg1);
    break;
  case(SYS_sync):
    ret = io_syscall_sync();
    break;
  case(SYS_fsync):
    ret = io_syscall_fsync((int)arg0);
    break;
  case(SYS_close):
    ret = io_syscall_close((int)arg0);
    break;
  case(SYS_epoll_create1):
    ret = io_syscall_epoll_create((int) arg0); 
    break;
  case(SYS_epoll_ctl):
    ret = io_syscall_epoll_ctl((int) arg0, (int) arg1, (int) arg2, (uintptr_t) arg3); 
    break;
  case(SYS_epoll_pwait):
    ret = io_syscall_epoll_pwait((int) arg0, (uintptr_t) arg1, (int) arg2, (int) arg3); 
    break;
  case(SYS_fcntl): 
    ret = io_syscall_fcntl((int)arg0, (int)arg1, (uintptr_t)arg2);
    break;
  case(SYS_chdir): 
    ret = io_syscall_chdir((char *) arg0);
    break;
  case(SYS_renameat2): 
    ret = io_syscall_renameat2((int) arg0, (uintptr_t) arg1,  (int) arg2, (uintptr_t) arg3, (int) arg4);
    break;
  case(SYS_umask): 
    ret = io_syscall_umask((int) arg0);
    break;
  case(SYS_getcwd): 
    ret = io_syscall_getcwd((char *)arg0, (size_t)arg1); 
    break;
  case(SYS_pipe2):
    ret = io_syscall_pipe((int*)arg0);
    break;

#endif /* USE_IO_SYSCALL */

#ifdef USE_NET_SYSCALL
  case(SYS_socket):
    ret = io_syscall_socket((int) arg0, (int) arg1, (int) arg2); 
    break; 
  case(SYS_setsockopt):
    ret = io_syscall_setsockopt((int) arg0, (int) arg1, (int) arg2, (int *) arg3, (int) arg4); 
    break; 
  case(SYS_connect):
    ret = io_syscall_connect((int) arg0, (uintptr_t) arg1, (int) arg2);
    break;
  case (SYS_bind):
    ret = io_syscall_bind((int) arg0, (uintptr_t) arg1, (int) arg2);
    break;
  case (SYS_listen):
    ret = io_syscall_listen((int) arg0, (uintptr_t) arg1);
    break;
  case (SYS_accept):
    ret = io_syscall_accept((int) arg0, (uintptr_t) arg1, (uintptr_t) arg2);
    break;
  case(SYS_recvfrom):
    ret = io_syscall_recvfrom((int) arg0, (uintptr_t) arg1, (int) arg2, (int) arg3, (uintptr_t) arg4, (uintptr_t) arg5);
    break;
  case(SYS_sendto):
    ret = io_syscall_sendto((int) arg0, (uintptr_t) arg1, (int) arg2, (int) arg3, (uintptr_t) arg4, (int) arg5);
    break;
  case(SYS_sendfile):
    ret = io_syscall_sendfile((int) arg0, (int) arg1, (uintptr_t) arg2, (int) arg3);
    break;
  case(SYS_getpeername): 
    ret = io_syscall_getpeername((int) arg0,  (uintptr_t) arg1, (uintptr_t) arg2);
    break;
  case(SYS_getsockname): 
    ret = io_syscall_getsockname((int) arg0,  (uintptr_t) arg1, (uintptr_t) arg2);
    break;
  case(SYS_getuid): 
    ret = io_syscall_getuid(); 
    break; 
  case(SYS_pselect6): 
    ret = io_syscall_pselect((int) arg0, (uintptr_t) arg1, (uintptr_t) arg2, (uintptr_t) arg3, (uintptr_t) arg4, (uintptr_t) arg5);
    break;
#endif /* USE_NET_SYSCALL */


  case(RUNTIME_SYSCALL_UNKNOWN):
  default:
    print_strace("[runtime] syscall %ld not implemented\r\n", (unsigned long) n);
    ret = -1;
    break;
  }

  /* store the result in the stack */
  ctx->regs.a0 = ret;
  return;
}

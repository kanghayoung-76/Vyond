//******************************************************************************
// Copyright (c) 2020, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#include "KeystoneDevice.hpp"

#include <stdio.h>
#include <sys/mman.h>

namespace Keystone {

KeystoneDevice::KeystoneDevice() { eid = -1; }

Error
KeystoneDevice::create(uint64_t minPages) {
  struct keystone_ioctl_create_enclave encl;
  encl.min_pages = minPages;

  if (ioctl(fd, KEYSTONE_IOC_CREATE_ENCLAVE, &encl)) {
    perror("ioctl error");
    eid = -1;
    return Error::IoctlErrorCreate;
  }

  eid      = encl.eid;
  physAddr = encl.epm_paddr;

  return Error::Success;
}

uintptr_t
KeystoneDevice::initUTM(size_t size) {
  struct keystone_ioctl_create_enclave encl;
  encl.eid      = eid;
  encl.utm_size = size;
  if (ioctl(fd, KEYSTONE_IOC_UTM_INIT, &encl)) {
    return 0;
  }

  return encl.utm_paddr;
}

Error
KeystoneDevice::finalize(
    uintptr_t runtimePhysAddr, uintptr_t eappPhysAddr, uintptr_t freePhysAddr,
    uintptr_t freeRequested) {
  struct keystone_ioctl_create_enclave encl;
  encl.eid            = eid;
  encl.runtime_paddr  = runtimePhysAddr;
  encl.user_paddr     = eappPhysAddr;
  encl.free_paddr     = freePhysAddr;
  encl.free_requested = freeRequested;

  printf("[TRACE][DEV] ioctl FINALIZE: eid=%d runtime=%#lx user=%#lx free=%#lx free_req=%#lx\n",
         encl.eid, (unsigned long)encl.runtime_paddr, (unsigned long)encl.user_paddr,
         (unsigned long)encl.free_paddr, (unsigned long)encl.free_requested);
  if (ioctl(fd, KEYSTONE_IOC_FINALIZE_ENCLAVE, &encl)) {
    perror("ioctl error");
    return Error::IoctlErrorFinalize;
  }
  return Error::Success;
}

Error
KeystoneDevice::destroy() {
  struct keystone_ioctl_create_enclave encl;
  encl.eid = eid;

  /* if the enclave has never created */
  if (eid < 0) {
    return Error::Success;
  }

  if (ioctl(fd, KEYSTONE_IOC_DESTROY_ENCLAVE, &encl)) {
    perror("ioctl error");
    return Error::IoctlErrorDestroy;
  }

  /* destroy 를 idempotent 하게: 성공 후 eid 를 무효화한다. ~Enclave() 가 destroy() 를
   * 한 번 더 부르는 구조라, 이게 없으면 낡은 eid 로 ioctl 이 재발행되어 드라이버가
   * "invalid enclave id" 를 찍는다(create/destroy 반복 벤치에서 회차당 1건씩 쌓였다). */
  eid = -1;

  return Error::Success;
}

Error
KeystoneDevice::__run(bool resume, uintptr_t* ret) {
  //printf("[SDK] __run resume: %d\n", resume);
  struct keystone_ioctl_run_enclave encl;
  encl.eid = eid;

  Error error;
  uint64_t request;

  if (resume) {
    error   = Error::IoctlErrorResume;
    request = KEYSTONE_IOC_RESUME_ENCLAVE;
  } else {
    error   = Error::IoctlErrorRun;
    request = KEYSTONE_IOC_RUN_ENCLAVE;
  }

  if (ioctl(fd, request, &encl)) {
    return error;
  }
  //printf("[SDK] __run done ioctl result: %d\n", encl.error);

  switch (encl.error) {
    case SBI_ERR_SM_ENCLAVE_EDGE_CALL_HOST:
      return Error::EdgeCallHost;
    case SBI_ERR_SM_ENCLAVE_INTERRUPTED:
      return Error::EnclaveInterrupted;
    case SBI_ERR_SM_ENCLAVE_WAITING_FOR_DEVICE:
      return Error::EnclaveWaitingForDevice;
    case SBI_ERR_SM_ENCLAVE_WAITING_FOR_SHM:
      return Error::EnclaveWaitingForShm;
    case SBI_ERR_SM_ENCLAVE_SUCCESS:
      if (ret) {
        *ret = encl.value;
      }
      return Error::Success;
    default:
      ERROR(
          "Unknown SBI error (%d) returned by %s_enclave\n", encl.error,
          resume ? "resume" : "run");
      return error;
  }
}

Error
KeystoneDevice::run(uintptr_t* ret) {
  return __run(false, ret);
}

Error
KeystoneDevice::resume(uintptr_t* ret) {
  return __run(true, ret);
}

void*
KeystoneDevice::map(uintptr_t addr, size_t size) {
  assert(fd >= 0);
  void* ret;
  ret = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, addr);
  assert(ret != MAP_FAILED);
  return ret;
}

bool
KeystoneDevice::initDevice(Params params) { // TODO: why does this need params
  /* open device driver */
  fd = open(KEYSTONE_DEV_PATH, O_RDWR);
  if (fd < 0) {
    PERROR("cannot open device file");
    return false;
  }
  return true;
}

int
KeystoneDevice::getEID() {
  return eid;
}

Error
MockKeystoneDevice::create(uint64_t minPages) {
  eid = -1;
  return Error::Success;
}

uintptr_t
MockKeystoneDevice::initUTM(size_t size) {
  return 0;
}

Error
MockKeystoneDevice::finalize(
    uintptr_t runtimePhysAddr, uintptr_t eappPhysAddr, uintptr_t freePhysAddr,
    uintptr_t freeRequested) {
  return Error::Success;
}

Error
MockKeystoneDevice::destroy() {
  return Error::Success;
}

Error
MockKeystoneDevice::run(uintptr_t* ret) {
  return Error::Success;
}

Error
MockKeystoneDevice::resume(uintptr_t* ret) {
  return Error::Success;
}

bool
MockKeystoneDevice::initDevice(Params params) {
  return true;
}

void*
MockKeystoneDevice::map(uintptr_t addr, size_t size) {
  sharedBuffer = malloc(size);
  return sharedBuffer;
}

MockKeystoneDevice::~MockKeystoneDevice() {
  if (sharedBuffer) free(sharedBuffer);
}

int
MockKeystoneDevice::getEID() {
  return 0;
}

}  // namespace Keystone

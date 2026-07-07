//******************************************************************************
// Copyright (c) 2018, The Regents of the University of California (Regents).
// All Rights Reserved. See LICENSE for license details.
//------------------------------------------------------------------------------
#include "Memory.hpp"

namespace Keystone {

void
PhysicalEnclaveMemory::init(
    KeystoneDevice* dev, uintptr_t phys_addr, size_t min_pages) {
  pDevice = dev;
  // TODO(dayeol): need to set actual EPM size
  epmSize       = PAGE_SIZE * min_pages;
  epmFreeList   = 0; 
  startAddr 		= phys_addr;
}

uintptr_t
PhysicalEnclaveMemory::allocUtm(size_t size) {
  uintptr_t ret = pDevice->initUTM(size);
  untrustedSize = size;
  utmPhysAddr   = ret;
  return ret;
}

uintptr_t
PhysicalEnclaveMemory::allocMem(size_t size) {
  assert(pDevice);
  return reinterpret_cast<uintptr_t>(pDevice->map(0, size));
}

uintptr_t
PhysicalEnclaveMemory::readMem(uintptr_t src, size_t size) {
  assert(pDevice);
  return reinterpret_cast<uintptr_t>(pDevice->map(src, size));
}

/* src: virtual address */
void
PhysicalEnclaveMemory::writeMem(uintptr_t src, uintptr_t offset, size_t size) {
  assert(pDevice);
  Error err = pDevice->writeEPM(offset, src, size);
  assert(err == Error::Success);
}

}  // namespace Keystone

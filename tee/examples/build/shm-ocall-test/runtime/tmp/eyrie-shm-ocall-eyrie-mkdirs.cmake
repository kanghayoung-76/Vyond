# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file Copyright.txt or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION 3.5)

file(MAKE_DIRECTORY
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/src/eyrie-shm-ocall-eyrie"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/src/eyrie-shm-ocall-eyrie-build"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/tmp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/src/eyrie-shm-ocall-eyrie-stamp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/src"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/src/eyrie-shm-ocall-eyrie-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/src/eyrie-shm-ocall-eyrie-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/shm-ocall-test/runtime/src/eyrie-shm-ocall-eyrie-stamp${cfgdir}") # cfgdir has leading slash
endif()

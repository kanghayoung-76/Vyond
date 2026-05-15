# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file Copyright.txt or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION 3.5)

file(MAKE_DIRECTORY
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/src/eyrie-wg-lru-eyrie"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/src/eyrie-wg-lru-eyrie-build"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/tmp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/src/eyrie-wg-lru-eyrie-stamp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/src"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/src/eyrie-wg-lru-eyrie-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/src/eyrie-wg-lru-eyrie-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/wg-lru-test/runtime/src/eyrie-wg-lru-eyrie-stamp${cfgdir}") # cfgdir has leading slash
endif()

# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file Copyright.txt or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION 3.5)

file(MAKE_DIRECTORY
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/src/eyrie-dds-bridge-eyrie"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/src/eyrie-dds-bridge-eyrie-build"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/tmp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/src/eyrie-dds-bridge-eyrie-stamp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/src"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/src/eyrie-dds-bridge-eyrie-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/src/eyrie-dds-bridge-eyrie-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dds-tdds-test/runtime/src/eyrie-dds-bridge-eyrie-stamp${cfgdir}") # cfgdir has leading slash
endif()

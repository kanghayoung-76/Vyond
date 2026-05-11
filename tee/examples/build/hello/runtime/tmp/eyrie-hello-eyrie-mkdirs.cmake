# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file Copyright.txt or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION 3.5)

file(MAKE_DIRECTORY
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/src/eyrie-hello-eyrie"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/src/eyrie-hello-eyrie-build"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/tmp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/src/eyrie-hello-eyrie-stamp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/src"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/src/eyrie-hello-eyrie-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/src/eyrie-hello-eyrie-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/hello/runtime/src/eyrie-hello-eyrie-stamp${cfgdir}") # cfgdir has leading slash
endif()

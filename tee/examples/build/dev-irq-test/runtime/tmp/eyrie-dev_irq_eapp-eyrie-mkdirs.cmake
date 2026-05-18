# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file Copyright.txt or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION 3.5)

file(MAKE_DIRECTORY
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-build"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/tmp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/src"
  "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/data/hykang/RVSS/WGTEE_project/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp${cfgdir}") # cfgdir has leading slash
endif()

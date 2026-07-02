# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file LICENSE.rst or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION ${CMAKE_VERSION}) # this file comes with cmake

# If CMAKE_DISABLE_SOURCE_CHANGES is set to true and the source directory is an
# existing directory in our source tree, calling file(MAKE_DIRECTORY) on it
# would cause a fatal error, even though it would be a no-op.
if(NOT EXISTS "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie")
  file(MAKE_DIRECTORY "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie")
endif()
file(MAKE_DIRECTORY
  "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-build"
  "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime"
  "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/tmp"
  "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp"
  "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src"
  "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/home/hykang/Vyond/tee/examples/build/dev-irq-test/runtime/src/eyrie-dev_irq_eapp-eyrie-stamp${cfgdir}") # cfgdir has leading slash
endif()

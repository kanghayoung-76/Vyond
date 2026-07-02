# Distributed under the OSI-approved BSD 3-Clause License.  See accompanying
# file LICENSE.rst or https://cmake.org/licensing for details.

cmake_minimum_required(VERSION ${CMAKE_VERSION}) # this file comes with cmake

# If CMAKE_DISABLE_SOURCE_CHANGES is set to true and the source directory is an
# existing directory in our source tree, calling file(MAKE_DIRECTORY) on it
# would cause a fatal error, even though it would be a no-op.
if(NOT EXISTS "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src/eyrie-tdds-pub-eyrie")
  file(MAKE_DIRECTORY "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src/eyrie-tdds-pub-eyrie")
endif()
file(MAKE_DIRECTORY
  "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src/eyrie-tdds-pub-eyrie-build"
  "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime"
  "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/tmp"
  "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src/eyrie-tdds-pub-eyrie-stamp"
  "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src"
  "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src/eyrie-tdds-pub-eyrie-stamp"
)

set(configSubDirs )
foreach(subDir IN LISTS configSubDirs)
    file(MAKE_DIRECTORY "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src/eyrie-tdds-pub-eyrie-stamp/${subDir}")
endforeach()
if(cfgdir)
  file(MAKE_DIRECTORY "/home/hykang/Vyond/tee/examples/build/tdds-test/runtime/src/eyrie-tdds-pub-eyrie-stamp${cfgdir}") # cfgdir has leading slash
endif()

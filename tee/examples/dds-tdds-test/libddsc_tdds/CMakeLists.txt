cmake_minimum_required(VERSION 3.10)
project(ddsc_tdds C)

# ── Standalone build instructions ────────────────────────────────────────────
# This CMakeLists can be used to build libddsc_tdds.a independently, e.g.:
#
#   KEYSTONE_SDK_DIR=<path>/sdk/install
#   cmake -B build \
#         -DCMAKE_C_COMPILER=riscv64-linux-gnu-gcc \
#         -DKEYSTONE_SDK_DIR=$KEYSTONE_SDK_DIR
#   make -C build
#
# In normal usage the library is built by the parent dds-tdds-test CMakeLists
# via add_library(ddsc_tdds STATIC libddsc_tdds/ddsc_tdds.c).
# ─────────────────────────────────────────────────────────────────────────────

if(NOT DEFINED KEYSTONE_SDK_DIR)
    set(KEYSTONE_SDK_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../sdk/install")
endif()

add_library(ddsc_tdds STATIC ddsc_tdds.c)

target_include_directories(ddsc_tdds PUBLIC
    ${KEYSTONE_SDK_DIR}/include/app
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include
)

target_compile_options(ddsc_tdds PRIVATE
    -march=rv64gc
    -mabi=lp64d
    -mcmodel=medany
    -Wall
    -Wextra
    -Wno-unused-parameter
)

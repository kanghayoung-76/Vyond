set(eapp_bin wgc-mem-bench)
set(eapp_src eapp/wgc_mem_bench.c)
set(host_bin wgc-mem-bench-runner)
set(host_src host/host.cpp)
set(package_name "wgc-mem-bench.ke")
set(package_script "./wgc-mem-bench-runner wgc-mem-bench eyrie-rt loader.bin")
set(eyrie_plugins "none")

# eapp

add_executable(${eapp_bin} ${eapp_src})
# [2026-07-30] -Wl,-e,main 필수. KEYSTONE_LIB_EAPP 에는 `_start` 가 없어서(nm 상 `U _start`)
# 링커가 .text 의 첫 심볼을 ELF 엔트리로 잡는다. 다른 eapp 들은 main 이 첫 심볼이라 우연히
# 동작했지만, 이 eapp 은 main 앞에 static 헬퍼가 있어 엔트리가 build_chase 가 됐다. eyrie 는
# elf_getEntryPoint() 를 sepc 에 그대로 넣으므로(runtime/sys/boot.c:45) U-모드가 build_chase
# 에서 시작해 a0=0 으로 즉시 ret → 런타임 복귀주소(ra=rt_base+0x14)로 점프 → instruction
# page fault 무한 루프였다.
target_link_libraries(${eapp_bin} "-nostdlib -static -Wl,-e,main" ${KEYSTONE_LIB_EAPP} ${KEYSTONE_LIB_EDGE})

target_include_directories(${eapp_bin}
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}
  PUBLIC ${KEYSTONE_SDK_DIR}/include/app
  PUBLIC ${KEYSTONE_SDK_DIR}/include/edge)

# host

add_executable(${host_bin} ${host_src})
# static: FPGA busybox RAM rootfs 에는 공유 glibc 가 없다(다른 앱과 동일 설정).
target_link_libraries(${host_bin} ${KEYSTONE_LIB_HOST} ${KEYSTONE_LIB_EDGE} m "-static")
set_target_properties(${host_bin}
  PROPERTIES CXX_STANDARD 11 CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO
)
target_include_directories(${host_bin}
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}
  PUBLIC ${KEYSTONE_SDK_DIR}/include/host
  PUBLIC ${KEYSTONE_SDK_DIR}/include/edge)

# Eyrie runtime

set(eyrie_files_to_copy .options_log eyrie-rt loader.bin)
add_eyrie_runtime(${eapp_bin}-eyrie
  ${eyrie_plugins}
  ${eyrie_files_to_copy})

# package

add_keystone_package(${eapp_bin}-package
  ${package_name}
  ${package_script}
  ${eyrie_files_to_copy} ${eapp_bin} ${host_bin})

add_dependencies(${eapp_bin}-package ${eapp_bin}-eyrie)

add_dependencies(examples ${eapp_bin}-package)

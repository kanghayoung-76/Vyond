set(eapp_bin enter-exit-test)
set(eapp_src eapp/eapp.c)
set(host_bin enter-exit-runner)
set(host_src host/host.cpp)
set(package_name "enter-exit-test.ke")
set(package_script "./enter-exit-runner enter-exit-test eyrie-rt loader.bin")
# Working plugins (same as hello/ros) so the loader loads the runtime OK.
set(eyrie_plugins "io_syscall linux_syscall env_setup")

# eapp (plain static enclave program)
add_executable(${eapp_bin} ${eapp_src})
target_link_libraries(${eapp_bin} "-static")

# host (static — FPGA busybox RAM rootfs has no shared libc)
add_executable(${host_bin} ${host_src})
target_link_libraries(${host_bin} ${KEYSTONE_LIB_HOST} ${KEYSTONE_LIB_EDGE} "-static")
set_target_properties(${host_bin}
  PROPERTIES CXX_STANDARD 11 CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO)
target_include_directories(${host_bin}
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

set(eapp_bin wg-reuse)
set(eapp_src eapp/wg_reuse.c)
set(host_bin wg-reuse-runner)
set(host_src host/host.cpp)
set(package_name "wg-reuse-test.ke")
set(package_script "./wg-reuse-runner wg-reuse eyrie-rt loader.bin")
set(eyrie_plugins "io_syscall linux_syscall env_setup")

# eapp
add_executable(${eapp_bin} ${eapp_src})
target_link_libraries(${eapp_bin} "-static")

# host
add_executable(${host_bin} ${host_src})
target_link_libraries(${host_bin} ${KEYSTONE_LIB_HOST})
target_include_directories(${host_bin}
  PUBLIC ${KEYSTONE_SDK_DIR}/include/host)

set(eyrie_files_to_copy .options_log eyrie-rt loader.bin)
add_eyrie_runtime(${eapp_bin}-eyrie
  ${eyrie_plugins}
  ${eyrie_files_to_copy})

add_keystone_package(${eapp_bin}-package
  ${package_name}
  ${package_script}
  ${eyrie_files_to_copy} ${eapp_bin} ${host_bin})

add_dependencies(${eapp_bin}-package ${eapp_bin}-eyrie)
add_dependencies(examples ${eapp_bin}-package)

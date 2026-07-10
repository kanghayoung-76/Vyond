set(eapp_bin beebs-crc32)
set(eapp_src main.c crc_32.c)
set(host_bin beebs-crc32-runner)
set(host_src host/host.cpp)
set(package_name "beebs-crc32.ke")
set(package_script "./beebs-crc32-runner beebs-crc32 eyrie-rt loader.bin")
set(eyrie_plugins "io_syscall linux_syscall env_setup")

add_executable(${eapp_bin} ${eapp_src})
target_link_libraries(${eapp_bin} "-static")
target_include_directories(${eapp_bin} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

add_executable(${host_bin} ${host_src})
target_link_libraries(${host_bin} ${KEYSTONE_LIB_HOST} ${KEYSTONE_LIB_EDGE})

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

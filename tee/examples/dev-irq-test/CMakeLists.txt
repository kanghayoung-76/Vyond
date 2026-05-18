set(app_name dev-irq-test)
set(eapp_bin dev_irq_eapp)
set(eapp_src eapp/dev_irq_eapp.c eapp/edge_wrapper.c)
set(host_bin dev-irq-runner)
set(host_src host/dev_irq_host.cpp host/edge_wrapper.cpp)
set(package_name "dev-irq-test.ke")
set(package_script "./dev-irq-runner dev_irq_eapp eyrie-rt loader.bin")
set(eyrie_plugins "io_syscall linux_syscall env_setup")

# host
add_executable(${host_bin} ${host_src})
target_link_libraries(${host_bin} PRIVATE ${KEYSTONE_LIB_HOST} ${KEYSTONE_LIB_EDGE})

# eapp
add_executable(${eapp_bin} ${eapp_src})
target_link_libraries(${eapp_bin} ${KEYSTONE_LIB_EAPP} ${KEYSTONE_LIB_EDGE} "-static")
set_target_properties(${eapp_bin}
  PROPERTIES LINK_FLAGS "-nostdlib -static -T ${CMAKE_CURRENT_SOURCE_DIR}/app.lds")

# Eyrie runtime
set(eyrie_files_to_copy .options_log eyrie-rt loader.bin)
add_eyrie_runtime(${eapp_bin}-eyrie
  ${eyrie_plugins}
  ${eyrie_files_to_copy})

# package
add_keystone_package(${app_name}-package
  ${package_name}
  ${package_script}
  ${eyrie_files_to_copy} ${eapp_bin} ${host_bin})

add_dependencies(${app_name}-package ${eapp_bin}-eyrie)
add_dependencies(examples ${app_name}-package)

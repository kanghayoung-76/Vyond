set(bridge_bin dds-bridge)
set(sub_bin    dds-sub)
set(host_bin   dds-runner)

set(bridge_src bridge/bridge.c)
set(sub_src    sub/sub.c)
set(host_src   host/host.cpp)

set(package_name   "dds-tdds-test.ke")
set(package_script "./dds-runner dds-bridge dds-sub eyrie-rt loader.bin")
set(eyrie_plugins  "none")

# --- bridge enclave ---
add_executable(${bridge_bin} ${bridge_src})
target_link_libraries(${bridge_bin}
    "-nostdlib -static -e main"
    ${KEYSTONE_LIB_EAPP}
    ${KEYSTONE_LIB_EDGE})
target_include_directories(${bridge_bin} PUBLIC
    ${KEYSTONE_SDK_DIR}/include/app
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include)

# --- sub enclave ---
add_executable(${sub_bin} ${sub_src})
target_link_libraries(${sub_bin}
    "-nostdlib -static -e main"
    ${KEYSTONE_LIB_EAPP}
    ${KEYSTONE_LIB_EDGE})
target_include_directories(${sub_bin} PUBLIC
    ${KEYSTONE_SDK_DIR}/include/app
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include)

# --- host runner ---
add_executable(${host_bin} ${host_src})
target_link_libraries(${host_bin}
    ${KEYSTONE_LIB_HOST}
    ${KEYSTONE_LIB_EDGE})
set_target_properties(${host_bin}
    PROPERTIES CXX_STANDARD 11 CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO)
target_include_directories(${host_bin} PUBLIC
    ${KEYSTONE_SDK_DIR}/include/host
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include)

# --- Eyrie runtime: reuse already-built files from tdds-test ---
# tdds-test runs add_eyrie_runtime first (processed before this subdir),
# so we just copy its output instead of creating a new ExternalProject.
set(TDDS_EYRIE_SRC ${CMAKE_BINARY_DIR}/tdds-test/runtime/src/eyrie-tdds-pub-eyrie)
set(eyrie_files_to_copy .options_log eyrie-rt loader.bin)

set(eyrie_out_files "")
foreach(f ${eyrie_files_to_copy})
    list(APPEND eyrie_out_files ${CMAKE_CURRENT_BINARY_DIR}/${f})
    add_custom_command(
        OUTPUT  ${CMAKE_CURRENT_BINARY_DIR}/${f}
        COMMAND ${CMAKE_COMMAND} -E copy ${TDDS_EYRIE_SRC}/${f} ${CMAKE_CURRENT_BINARY_DIR}/${f}
        DEPENDS eyrie-tdds-pub-eyrie ${TDDS_EYRIE_SRC}/${f}
        COMMENT "eyrie copy: ${f}"
    )
endforeach()
add_custom_target(${bridge_bin}-eyrie DEPENDS ${eyrie_out_files})

# --- Package ---
add_keystone_package(${bridge_bin}-package
    ${package_name}
    ${package_script}
    ${eyrie_files_to_copy}
    ${bridge_bin}
    ${sub_bin}
    ${host_bin})

add_dependencies(${bridge_bin}-package ${bridge_bin}-eyrie)
add_dependencies(examples ${bridge_bin}-package)

set(bridge_bin  split-bridge)
set(sub_bin     split-sub)
set(bridge_host split-bridge-host)
set(sub_host    split-sub-host)

# ---- enc1: bridge enclave ------------------------------------------------
add_executable(${bridge_bin} bridge/bridge.c)
target_link_libraries(${bridge_bin}
    "-nostdlib -static -e main"
    ${KEYSTONE_LIB_EAPP}
    ${KEYSTONE_LIB_EDGE})
target_include_directories(${bridge_bin} PUBLIC
    ${KEYSTONE_SDK_DIR}/include/app
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include)

# ---- enc2: sub enclave ---------------------------------------------------
add_executable(${sub_bin} sub/sub.c)
target_link_libraries(${sub_bin}
    "-nostdlib -static -e main"
    ${KEYSTONE_LIB_EAPP}
    ${KEYSTONE_LIB_EDGE})
target_include_directories(${sub_bin} PUBLIC
    ${KEYSTONE_SDK_DIR}/include/app
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include)

# ---- bridge-host binary --------------------------------------------------
add_executable(${bridge_host} bridge-host/bridge-host.cpp)
target_link_libraries(${bridge_host}
    ${KEYSTONE_LIB_HOST}
    ${KEYSTONE_LIB_EDGE})
set_target_properties(${bridge_host}
    PROPERTIES CXX_STANDARD 11 CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO)
target_include_directories(${bridge_host} PUBLIC
    ${KEYSTONE_SDK_DIR}/include/host
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include)

# ---- sub-host binary -----------------------------------------------------
add_executable(${sub_host} sub-host/sub-host.cpp)
target_link_libraries(${sub_host}
    ${KEYSTONE_LIB_HOST}
    ${KEYSTONE_LIB_EDGE})
set_target_properties(${sub_host}
    PROPERTIES CXX_STANDARD 11 CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO)
target_include_directories(${sub_host} PUBLIC
    ${KEYSTONE_SDK_DIR}/include/host
    ${KEYSTONE_SDK_DIR}/include/edge
    ${KEYSTONE_SDK_DIR}/include)

# ---- Eyrie runtime: reuse from tdds-test (same TDDS syscall support) -----
set(TDDS_EYRIE_SRC ${CMAKE_BINARY_DIR}/tdds-test/runtime/src/eyrie-tdds-pub-eyrie)
set(eyrie_files .options_log eyrie-rt loader.bin)

set(eyrie_out_files "")
foreach(f ${eyrie_files})
    list(APPEND eyrie_out_files ${CMAKE_CURRENT_BINARY_DIR}/${f})
    add_custom_command(
        OUTPUT  ${CMAKE_CURRENT_BINARY_DIR}/${f}
        COMMAND ${CMAKE_COMMAND} -E copy ${TDDS_EYRIE_SRC}/${f} ${CMAKE_CURRENT_BINARY_DIR}/${f}
        DEPENDS eyrie-tdds-pub-eyrie ${TDDS_EYRIE_SRC}/${f}
        COMMENT "eyrie copy: ${f}"
    )
endforeach()
add_custom_target(split-eyrie DEPENDS ${eyrie_out_files})

# ---- Package helper macro (inline) ----------------------------------------
# add_keystone_package uses a single pkg/ directory, so calling it twice from
# the same CMakeLists.txt would conflict.  We build each package manually with
# its own staging directory (pkg-sub/ and pkg-bridge/).

find_program(MAKESELF makeself)

# ------ split-sub-host.ke  (sub-host + enc2 + runtime) --------------------
set(SUB_PKG_DIR  ${CMAKE_CURRENT_BINARY_DIR}/pkg-sub)
set(SUB_PKG_NAME split-sub-host.ke)
set(SUB_BINS
    ${CMAKE_CURRENT_BINARY_DIR}/${sub_host}
    ${CMAKE_CURRENT_BINARY_DIR}/${sub_bin})

set(sub_pkg_files "")
add_custom_command(OUTPUT ${SUB_PKG_DIR}
    COMMAND ${CMAKE_COMMAND} -E make_directory ${SUB_PKG_DIR})

foreach(f ${SUB_BINS})
    get_filename_component(fname ${f} NAME)
    set(dst ${SUB_PKG_DIR}/${fname})
    list(APPEND sub_pkg_files ${dst})
    add_custom_command(OUTPUT ${dst}
        DEPENDS ${f} ${SUB_PKG_DIR}
        COMMAND ${CMAKE_COMMAND} -E copy ${f} ${dst})
endforeach()
foreach(f ${eyrie_files})
    set(src ${CMAKE_CURRENT_BINARY_DIR}/${f})
    set(dst ${SUB_PKG_DIR}/${f})
    list(APPEND sub_pkg_files ${dst})
    add_custom_command(OUTPUT ${dst}
        DEPENDS ${src} ${SUB_PKG_DIR}
        COMMAND ${CMAKE_COMMAND} -E copy ${src} ${dst})
endforeach()

add_custom_target(split-sub-host-package
    DEPENDS ${sub_pkg_files}
    COMMAND ${MAKESELF} --noprogress ${SUB_PKG_DIR} ${SUB_PKG_NAME}
            "Keystone split-sub-host package"
            ./split-sub-host split-sub eyrie-rt loader.bin)
add_dependencies(split-sub-host-package split-eyrie ${sub_host} ${sub_bin})

# ------ split-bridge-host.ke  (bridge-host + enc1 + runtime) --------------
set(BRIDGE_PKG_DIR  ${CMAKE_CURRENT_BINARY_DIR}/pkg-bridge)
set(BRIDGE_PKG_NAME split-bridge-host.ke)
set(BRIDGE_BINS
    ${CMAKE_CURRENT_BINARY_DIR}/${bridge_host}
    ${CMAKE_CURRENT_BINARY_DIR}/${bridge_bin})

set(bridge_pkg_files "")
add_custom_command(OUTPUT ${BRIDGE_PKG_DIR}
    COMMAND ${CMAKE_COMMAND} -E make_directory ${BRIDGE_PKG_DIR})

foreach(f ${BRIDGE_BINS})
    get_filename_component(fname ${f} NAME)
    set(dst ${BRIDGE_PKG_DIR}/${fname})
    list(APPEND bridge_pkg_files ${dst})
    add_custom_command(OUTPUT ${dst}
        DEPENDS ${f} ${BRIDGE_PKG_DIR}
        COMMAND ${CMAKE_COMMAND} -E copy ${f} ${dst})
endforeach()
foreach(f ${eyrie_files})
    set(src ${CMAKE_CURRENT_BINARY_DIR}/${f})
    set(dst ${BRIDGE_PKG_DIR}/${f})
    list(APPEND bridge_pkg_files ${dst})
    add_custom_command(OUTPUT ${dst}
        DEPENDS ${src} ${BRIDGE_PKG_DIR}
        COMMAND ${CMAKE_COMMAND} -E copy ${src} ${dst})
endforeach()

add_custom_target(split-bridge-host-package
    DEPENDS ${bridge_pkg_files}
    COMMAND ${MAKESELF} --noprogress ${BRIDGE_PKG_DIR} ${BRIDGE_PKG_NAME}
            "Keystone split-bridge-host package"
            ./split-bridge-host split-bridge eyrie-rt loader.bin)
add_dependencies(split-bridge-host-package split-eyrie ${bridge_host} ${bridge_bin})

# ---- register both packages with the top-level 'examples' target ----------
add_dependencies(examples split-sub-host-package split-bridge-host-package)

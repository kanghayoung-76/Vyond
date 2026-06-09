set(pub_bin  tdds-pub)
set(sub_bin  tdds-sub)
set(host_bin tdds-runner)

set(pub_src  pub/pub.c)
set(sub_src  sub/sub.c)
set(host_src host/host.cpp)

set(package_name   "tdds-test.ke")
set(package_script "./tdds-runner tdds-pub tdds-sub eyrie-rt loader.bin")
set(eyrie_plugins  "none")

# ---- publisher enclave ----
add_executable(${pub_bin} ${pub_src})
target_link_libraries(${pub_bin}
  "-nostdlib -static -e main"
  ${KEYSTONE_LIB_EAPP}
  ${KEYSTONE_LIB_EDGE})
target_include_directories(${pub_bin}
  PUBLIC ${KEYSTONE_SDK_DIR}/include/app
  PUBLIC ${KEYSTONE_SDK_DIR}/include/edge
  PUBLIC ${KEYSTONE_SDK_DIR}/include)

# ---- subscriber enclave ----
add_executable(${sub_bin} ${sub_src})
target_link_libraries(${sub_bin}
  "-nostdlib -static -e main"
  ${KEYSTONE_LIB_EAPP}
  ${KEYSTONE_LIB_EDGE})
target_include_directories(${sub_bin}
  PUBLIC ${KEYSTONE_SDK_DIR}/include/app
  PUBLIC ${KEYSTONE_SDK_DIR}/include/edge
  PUBLIC ${KEYSTONE_SDK_DIR}/include)

# ---- host runner ----
add_executable(${host_bin} ${host_src})
target_link_libraries(${host_bin}
  ${KEYSTONE_LIB_HOST}
  ${KEYSTONE_LIB_EDGE})
set_target_properties(${host_bin}
  PROPERTIES CXX_STANDARD 11 CXX_STANDARD_REQUIRED YES CXX_EXTENSIONS NO)
target_include_directories(${host_bin}
  PUBLIC ${KEYSTONE_SDK_DIR}/include/host
  PUBLIC ${KEYSTONE_SDK_DIR}/include/edge
  PUBLIC ${KEYSTONE_SDK_DIR}/include)

# ---- Eyrie runtime (shared by both enclaves) ----
set(eyrie_files_to_copy .options_log eyrie-rt loader.bin)
add_eyrie_runtime(${pub_bin}-eyrie
  ${eyrie_plugins}
  ${eyrie_files_to_copy})

# ---- Package: pub + sub + runner + runtime ----
add_keystone_package(${pub_bin}-package
  ${package_name}
  ${package_script}
  ${eyrie_files_to_copy}
  ${pub_bin}
  ${sub_bin}
  ${host_bin})

add_dependencies(${pub_bin}-package ${pub_bin}-eyrie)
add_dependencies(examples ${pub_bin}-package)

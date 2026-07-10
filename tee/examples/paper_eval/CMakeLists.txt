# WorldGuard / WorldClave paper evaluation experiments.
# Each experiment is its own subdirectory (eapp + host + package).
#
#   wgc-mem-bench  : #2 WGChecker enforcement cost (in-enclave memory latency)
#
# Add future experiments (create-latency, workload overhead, scalability, ...)
# as additional add_subdirectory() lines below.

add_subdirectory(wgc-mem-bench)
add_subdirectory(enclave-create-bench)
add_subdirectory(beebs-bench)

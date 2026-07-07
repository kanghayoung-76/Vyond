savedcmd_keystone-driver.mod := printf '%s\n'   keystone.o keystone-page.o keystone-ioctl.o keystone-enclave.o keystone-sbi.o enclave-pool.o | awk '!x[$$0]++ { print("./"$$0) }' > keystone-driver.mod

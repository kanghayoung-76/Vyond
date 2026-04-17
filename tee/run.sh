#!/bin/bash

HOST_PORT="$((RANDOM % 3000 + 3000))"
export HOST_PORT

ROOT_PATH="$(readlink -f "$(cd "$(dirname "$0")" && pwd)")"
echo $ROOT_PATH

#ROOTFS_IMAGE="$ROOT_PATH/../prebuilt/rootfs.ext2"
#LINUX_IMAGE="$ROOT_PATH/../prebuilt/Image"

echo "**** Running QEMU SSH on port ${HOST_PORT} ****";

#export SMP=1;

while [ "$1" != "" ]; do
    if [ "$1" = "-debug" ];
    then
        echo "**** GDB port $((HOST_PORT + 1)) ****";
        DEBUG="-gdb tcp::$((HOST_PORT + 1)) -S -d in_asm -D debug.log";
    fi;
    if [ "$1" = "-smp" ];
    then
        SMP="$2";
        shift;
    fi;
    if [ "$1" = "-fw" ];
    then
        FW_ELF="$2";
        shift;
    fi;
    if [ "$1" = "-qemu" ];
    then
        QEMU_SYSTEM="$2";
        shift;
    fi;
    shift;
done;

#for var in QEMU_SYSTEM FW_ELF; do
#    if [ -z "${!var}" ]; then
#        echo "$var is not set"
#        exit 1
#    fi
#done

QEMU_SYSTEM="../qemu/build/qemu-system-riscv64"
FW_BIN="sbi/opensbi/build/platform/generic/firmware/fw_dynamic.bin"
LINUX_IMAGE="$ROOT_PATH/../prebuilt/Image"
export SMP=1;

#echo $CMD
$QEMU_SYSTEM \
    -d guest_errors -D ./qemu.log \
    -m 16384 \
    -nographic \
    -machine virt,wg=on \
    -bios "$FW_BIN" \
    -kernel "$LINUX_IMAGE" \
    -initrd ubuntu24/ubuntu24-initrd.img \
    -drive file=ubuntu24/ubuntu-24.04-preinstalled-server-riscv64.img,format=raw,if=virtio \
    -append "root=/dev/vda3 rw console=ttyS0 earlycon" \
    -netdev user,id=net0,host=10.0.2.10,hostfwd=tcp::2211-:22 \
    -device virtio-net-pci,netdev=net0,romfile="" \
    -netdev tap,id=net1,ifname=tap0,script=no,downscript=no \
    -device virtio-net-pci,netdev=net1,romfile="" \
    -smp "$SMP" \
    -semihosting-config enable=on,userspace=on

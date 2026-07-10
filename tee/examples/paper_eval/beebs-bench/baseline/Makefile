# Native baseline: BEEBS crc32 as a plain static Linux/BusyBox binary.
# Same driver (../main.c) + kernel (../crc_32.c) as the enclave eapp, so the
# numbers compare directly. Static => runs on the BusyBox rootfs.

CROSS  ?= riscv64-linux-gnu-
CC      = $(CROSS)gcc
CFLAGS  = -O2 -static -march=rv64gc -mabi=lp64d -I.. -Wall

TARGET  = beebs-crc32-native
SRC     = ../main.c ../crc_32.c

all: $(TARGET)

$(TARGET): $(SRC) ../support.h
	$(CC) $(CFLAGS) -o $@ $(SRC)

clean:
	rm -f $(TARGET)

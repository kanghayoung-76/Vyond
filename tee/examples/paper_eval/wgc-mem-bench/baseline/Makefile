# Baseline (plain Linux) memory-latency binary for paper eval #2.
# Static so it runs on the BusyBox rootfs with no shared-lib deps.
#
#   make                      # build with riscv64-linux-gnu-gcc (default)
#   make CROSS=riscv64-unknown-linux-gnu-   # override toolchain prefix

CROSS ?= riscv64-linux-gnu-
CC     = $(CROSS)gcc
CFLAGS = -O2 -static -march=rv64gc -mabi=lp64d -Wall
LDLIBS = -lm

TARGET = mem-bench-native
SRC    = mem_bench_native.c

all: $(TARGET)

$(TARGET): $(SRC)
	$(CC) $(CFLAGS) -o $@ $< $(LDLIBS)

clean:
	rm -f $(TARGET)

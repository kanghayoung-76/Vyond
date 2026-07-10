// Minimal BEEBS support header (extracted so a single kernel builds standalone
// without the board/chip autotools machinery). crc32 only needs REPEAT_FACTOR
// plus the standard entry-point prototypes; the driver supplies any timing.
#ifndef SUPPORT_H
#define SUPPORT_H

// crc32's BEEBS calibration is CALIB_SCALE=7 => REPEAT_FACTOR = 4096>>7 = 32.
// crc32's rand_beebs() seed is static (carries across benchmark() calls), so the
// verify constant (1207487004) matches only after EXACTLY 32 benchmark() calls.
// This is a BEEBS build parameter, not a change to the crc32 kernel.
#ifndef REPEAT_FACTOR
#define REPEAT_FACTOR 32
#endif

int  benchmark(void);
int  verify_benchmark(int result);
void initialise_benchmark(void);

#endif /* SUPPORT_H */

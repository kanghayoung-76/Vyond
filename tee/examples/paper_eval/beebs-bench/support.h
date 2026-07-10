// Minimal BEEBS support header (extracted from the BEEBS suite so a single
// kernel builds standalone without the board/chip autotools machinery). The
// crc32 kernel only needs REPEAT_FACTOR plus the standard entry-point
// prototypes; the driver (eapp / native) supplies the timing.
#ifndef SUPPORT_H
#define SUPPORT_H

#ifndef REPEAT_FACTOR
#define REPEAT_FACTOR 4096   /* BEEBS BOARD_REPEAT_FACTOR default */
#endif

int  benchmark(void);
int  verify_benchmark(int result);
void initialise_benchmark(void);

#endif /* SUPPORT_H */

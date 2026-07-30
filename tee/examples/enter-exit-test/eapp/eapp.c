/* enter-exit-test enclave app.
 *
 * The absolute minimum: enter the enclave, do nothing, exit.
 * No ocall, no printf, no shared memory. Used to isolate whether the freeze
 * comes from the enclave enter/exit path itself, or from the loader's
 * ACCESS-FAULT-on-load path (which happens BEFORE main runs).
 *
 * If this app enters and exits cleanly, the enter/exit machinery is fine and
 * the freeze is specific to the load failure. If it still freezes, the problem
 * is in enter/exit (or teardown) regardless of the loader fault.
 */
int main(void) {
  volatile int x = 0;   /* keep main non-empty so it isn't optimised away */
  x++;
  return 0;             /* eyrie runtime turns this into a clean enclave exit */
}

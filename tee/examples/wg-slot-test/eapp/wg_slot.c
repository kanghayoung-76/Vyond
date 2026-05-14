int main()
{
    /* WGC slot virtualization test:
     * If we reach here, the SM successfully loaded the EPM WGC slot
     * on-demand and the enclave runtime booted normally.
     * No UTM/edge-call is used; clean exit proves the test passed. */
    return 0;
}

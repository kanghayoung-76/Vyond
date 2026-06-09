#pragma once

#include <cstdint>
#include <cstddef>
#include <functional>
#include <unordered_map>

namespace Keystone {

/*
 * TddsBroker — host-side coordinator for Trusted DDS channels.
 *
 * When a subscriber enclave sends OCALL_TDDS_WAIT (signalling that the SHM
 * flag is not yet set), the host must:
 *   1. Identify the publisher associated with that channel RID.
 *   2. Run/resume the publisher enclave so it writes the message.
 *   3. Return from the OCALL handler so Enclave::run() resumes the subscriber.
 *
 * Usage:
 *   TddsBroker broker;
 *
 *   // Register: for RID `rid`, the publisher is triggered by calling `fn`.
 *   broker.registerChannel(rid, [&pubEnc]() {
 *       uintptr_t dummy;
 *       pubEnc.run(&dummy);   // or resume, depending on state
 *   });
 *
 *   // Wire into the subscriber enclave's OCALL dispatcher:
 *   subEnc.registerOcallDispatch([&broker](void* buf, size_t sz) {
 *       broker.handleOcall(buf, sz);
 *   });
 *
 *   subEnc.run(&retval);
 */
class TddsBroker {
public:
    /* Callback type: called by handleOcall to trigger the publisher. */
    using PubTrigger = std::function<void()>;

    /* Associate a channel RID with a publisher trigger callback.
     * The trigger is called each time the subscriber yields waiting for data. */
    void registerChannel(uint32_t rid, PubTrigger trigger);

    /* Unregister a channel (e.g. after teardown). */
    void unregisterChannel(uint32_t rid);

    /* To be passed as the OcallFunc to Enclave::registerOcallDispatch().
     * Reads the edge_call from shared_buf: if call_id == OCALL_TDDS_WAIT,
     * fires the registered publisher trigger and writes CALL_STATUS_OK back.
     * Non-TDDS call IDs are left untouched so other dispatchers can handle them. */
    void handleOcall(void* shared_buf, size_t shared_size);

private:
    std::unordered_map<uint32_t, PubTrigger> channels_;
};

}  // namespace Keystone

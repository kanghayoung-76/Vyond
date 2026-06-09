#include "host/TddsBroker.hpp"

extern "C" {
#include "edge/edge_call.h"
#include "edge/edge_common.h"
#include "shared/tdds_common.h"
}

namespace Keystone {

void
TddsBroker::registerChannel(uint32_t rid, PubTrigger trigger) {
    channels_[rid] = std::move(trigger);
}

void
TddsBroker::unregisterChannel(uint32_t rid) {
    channels_.erase(rid);
}

void
TddsBroker::handleOcall(void* shared_buf, size_t shared_size) {
    auto* ec = static_cast<struct edge_call*>(shared_buf);

    if (ec->call_id != OCALL_TDDS_WAIT)
        return;  /* not a TDDS call — leave call_id intact for other handlers */

    /* Extract args from the shared buffer. */
    uintptr_t args_ptr  = 0;
    size_t    args_size = 0;
    if (edge_call_args_ptr(ec, &args_ptr, &args_size,
                           (uintptr_t)shared_buf, shared_size) != 0) {
        ec->return_data.call_status = CALL_STATUS_BAD_OFFSET;
        return;
    }
    if (args_size < sizeof(struct tdds_ocall_wait_args)) {
        ec->return_data.call_status = CALL_STATUS_ERROR;
        return;
    }

    uint32_t rid =
        reinterpret_cast<struct tdds_ocall_wait_args*>(args_ptr)->rid;

    /* Fire the registered publisher trigger for this channel.
     * The trigger runs (or resumes) the publisher enclave; it returns only
     * after the publisher has yielded back to the host.  By that point the
     * SHM flag should be set and the subscriber can proceed on resume. */
    auto it = channels_.find(rid);
    if (it != channels_.end())
        it->second();

    /* Signal success back to the runtime so the enclave's ocall() returns 0. */
    ec->return_data.call_status     = CALL_STATUS_OK;
    ec->return_data.call_ret_offset = 0;
    ec->return_data.call_ret_size   = 0;
}

}  // namespace Keystone

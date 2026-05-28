use semihosting::heprintln;

pub enum WIDAction {
    Assigned { slot: usize },
    Evicted  { slot: usize, evicted_eid: usize },
}

// WIDs 1-28 are available for enclaves (WID 0 = untrusted/OS default, WID 29 = DEV, WID 30 = OS, WID 31 = Trusted)
pub const ENCLAVE_WID_MIN: usize = 1;
pub const ENCLAVE_WID_MAX: usize = 28;
const NUM_ENCLAVE_WIDS: usize = ENCLAVE_WID_MAX - ENCLAVE_WID_MIN + 1; // 28

// valid=0 means the slot is free. Using usize guarantees 8-byte word width and
// unambiguous BSS zero-initialization on all Rust nightly targets.
#[derive(Copy, Clone)]
struct WIDEntry {
    valid: usize,  // 0 = free, 1 = occupied
    eid: usize,
    region_id: usize,
    last_used: u64,
}

struct WIDState {
    slots: [WIDEntry; NUM_ENCLAVE_WIDS],
    clock: u64,
}

const EMPTY_SLOT: WIDEntry = WIDEntry { valid: 0, eid: 0, region_id: 0, last_used: 0 };

// Single-CPU M-mode: no preemption, so no lock needed.
// slots[i] corresponds to WID (i + ENCLAVE_WID_MIN): slots[0]->WID1 ... slots[27]->WID28
static mut WID_STATE: WIDState = WIDState {
    slots: [EMPTY_SLOT; NUM_ENCLAVE_WIDS],
    clock: 0,
};

pub fn wid_init() {
    let state = unsafe { &mut WID_STATE };
    for slot in state.slots.iter_mut() {
        *slot = EMPTY_SLOT;
    }
    state.clock = 0;
}

pub fn get_assigned_wid(eid: usize) -> Option<usize> {
    let state = unsafe { &WID_STATE };
    for i in 0..NUM_ENCLAVE_WIDS {
        if state.slots[i].valid != 0 && state.slots[i].eid == eid {
            return Some(i + ENCLAVE_WID_MIN);
        }
    }
    None
}

pub fn assign_wid(eid: usize, region_id: usize) -> (usize, WIDAction) {
    let state = unsafe { &mut WID_STATE };
    state.clock += 1;
    let now = state.clock;

    // 1. Already assigned? Update LRU timestamp and return.
    for i in 0..NUM_ENCLAVE_WIDS {
        if state.slots[i].valid != 0 && state.slots[i].eid == eid {
            state.slots[i].last_used = now;
            heprintln!("[WID] reuse slot={} wid={}", i, i + ENCLAVE_WID_MIN);
            return (i + ENCLAVE_WID_MIN, WIDAction::Assigned { slot: i });
        }
    }

    // 2. Find lowest-numbered free slot
    for i in 0..NUM_ENCLAVE_WIDS {
        if state.slots[i].valid == 0 {
            state.slots[i] = WIDEntry { valid: 1, eid, region_id, last_used: now };
            let wid = i + ENCLAVE_WID_MIN;
            heprintln!("[WID] new slot={} wid={}", i, wid);
            return (wid, WIDAction::Assigned { slot: i });
        }
    }

    // 3. LRU eviction
    let mut lru_idx = 0;
    let mut lru_time = u64::MAX;
    for i in 0..NUM_ENCLAVE_WIDS {
        if state.slots[i].valid != 0 && state.slots[i].last_used < lru_time {
            lru_time = state.slots[i].last_used;
            lru_idx = i;
        }
    }

    let evicted_wid = lru_idx + ENCLAVE_WID_MIN;
    let evicted_eid = state.slots[lru_idx].eid;
    heprintln!("[WID] evict slot={} wid={} eid={}", lru_idx, evicted_wid, evicted_eid);

    state.slots[lru_idx] = WIDEntry { valid: 1, eid, region_id, last_used: now };

    crate::wg::invalidate_wid_in_all_slots(evicted_wid);

    (evicted_wid, WIDAction::Evicted { slot: lru_idx, evicted_eid })
}

pub fn release_wid_for_eid(eid: usize) {
    let state = unsafe { &mut WID_STATE };
    for i in 0..NUM_ENCLAVE_WIDS {
        if state.slots[i].valid != 0 && state.slots[i].eid == eid {
            let wid = i + ENCLAVE_WID_MIN;
            state.slots[i] = EMPTY_SLOT;
            crate::wg::invalidate_wid_in_all_slots(wid);
            return;
        }
    }
}

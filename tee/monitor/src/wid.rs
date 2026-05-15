use semihosting::hprintln;

/// WID (World ID) slot virtualization for WGC enclave regions.
///
/// Hardware WIDs 1-5 are shared across potentially many enclaves using LRU eviction.
/// The SM assigns a WID slot to each active enclave on-demand (via the ACCESS FAULT handler)
/// and evicts the least-recently-used slot when all slots are occupied.

// WIDs 1-5 are available for enclaves (WID 0 = untrusted/OS default, WID 6 = OS, WID 7 = Trusted)
pub const ENCLAVE_WID_MIN: usize = 1;
pub const ENCLAVE_WID_MAX: usize = 5;
const NUM_ENCLAVE_WIDS: usize = ENCLAVE_WID_MAX - ENCLAVE_WID_MIN + 1; // 5

#[derive(Copy, Clone)]
struct WIDEntry {
    eid: usize,
    region_id: usize, // wg region index (used for reset_wg on eviction)
    last_used: u64,
}

// WID_SLOTS[i] corresponds to WID (i + ENCLAVE_WID_MIN)
// WID_SLOTS[0] -> WID 1, WID_SLOTS[1] -> WID 2, ... WID_SLOTS[4] -> WID 5
static mut WID_SLOTS: [Option<WIDEntry>; NUM_ENCLAVE_WIDS] = [None; NUM_ENCLAVE_WIDS];
static mut CLOCK: u64 = 0;

/// Returns the WID currently assigned to this eid (1-5), or None if not assigned.
pub fn get_assigned_wid(eid: usize) -> Option<usize> {
    unsafe {
        for i in 0..NUM_ENCLAVE_WIDS {
            if let Some(ref entry) = WID_SLOTS[i] {
                if entry.eid == eid {
                    return Some(i + ENCLAVE_WID_MIN);
                }
            }
        }
    }
    None
}

/// Assigns a WID to the given eid (with associated wg region_id for eviction).
///
/// 1. If eid already has a WID, update last_used and return it.
/// 2. Find the lowest-numbered free WID slot and assign it.
/// 3. If no free slots: evict the LRU slot (calling wg::reset_wg), then assign.
///
/// Returns the assigned WID (1-5).
pub fn assign_wid(eid: usize, region_id: usize) -> usize {
    unsafe {
        // Bump the clock
        CLOCK += 1;
        let now = CLOCK;

        // 1. Already assigned?
        for i in 0..NUM_ENCLAVE_WIDS {
            if let Some(ref mut entry) = WID_SLOTS[i] {
                if entry.eid == eid {
                    entry.last_used = now;
                    let wid = i + ENCLAVE_WID_MIN;
                    hprintln!("[WID] reuse: eid={} WID={} region={}", eid, wid, region_id);
                    return wid;
                }
            }
        }

        // 2. Find lowest-numbered free slot
        for i in 0..NUM_ENCLAVE_WIDS {
            if WID_SLOTS[i].is_none() {
                WID_SLOTS[i] = Some(WIDEntry {
                    eid,
                    region_id,
                    last_used: now,
                });
                let wid = i + ENCLAVE_WID_MIN;
                hprintln!("[WID] assign: eid={} -> WID={} (slot {}, region={})", eid, wid, i, region_id);
                return wid;
            }
        }

        // 3. LRU eviction: find slot with smallest last_used
        let mut lru_idx = 0;
        let mut lru_time = u64::MAX;
        for i in 0..NUM_ENCLAVE_WIDS {
            if let Some(ref entry) = WID_SLOTS[i] {
                if entry.last_used < lru_time {
                    lru_time = entry.last_used;
                    lru_idx = i;
                }
            }
        }

        // Evict: clear the hardware WGC slot for the evicted region
        let evicted_eid;
        let evicted_region;
        let evicted_wid = lru_idx + ENCLAVE_WID_MIN;
        if let Some(ref evicted) = WID_SLOTS[lru_idx] {
            evicted_eid = evicted.eid;
            evicted_region = evicted.region_id;
            hprintln!(
                "[WID] EVICT: slot {} WID={} eid={} region={} -> eid={} region={}",
                lru_idx, evicted_wid, evicted_eid, evicted_region, eid, region_id
            );
            let _ = crate::wg::reset_wg(evicted.region_id);
        }

        // Assign the freed slot to the new eid
        WID_SLOTS[lru_idx] = Some(WIDEntry {
            eid,
            region_id,
            last_used: now,
        });

        evicted_wid
    }
}

/// Clears any WID assignment for this eid (called on enclave destroy).
pub fn release_wid_for_eid(eid: usize) {
    unsafe {
        for i in 0..NUM_ENCLAVE_WIDS {
            if let Some(ref entry) = WID_SLOTS[i] {
                if entry.eid == eid {
                    WID_SLOTS[i] = None;
                    return;
                }
            }
        }
    }
}

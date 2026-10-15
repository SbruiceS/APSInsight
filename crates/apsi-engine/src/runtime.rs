//! Memory budget and checkpoint store.

use crate::types::Batch;
use crate::{EngineError, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bytes of intermediate results a run may hold at once. Reservations are
/// released when dropped, so an error path can never leak budget.
#[derive(Debug)]
pub struct MemoryBudget {
    limit: u64,
    used: AtomicU64,
    peak: AtomicU64,
}

impl MemoryBudget {
    pub fn new(limit: u64) -> Arc<Self> {
        Arc::new(Self { limit, used: AtomicU64::new(0), peak: AtomicU64::new(0) })
    }

    pub fn reserve(self: &Arc<Self>, bytes: u64, what: &str) -> Result<Reservation> {
        let mut cur = self.used.load(Ordering::Relaxed);
        loop {
            let next = cur.saturating_add(bytes);
            if next > self.limit {
                return Err(EngineError::MemoryExceeded { what: what.into(), needed: next, limit: self.limit });
            }
            match self.used.compare_exchange_weak(cur, next, Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => {
                    self.peak.fetch_max(next, Ordering::AcqRel);
                    return Ok(Reservation { budget: Arc::clone(self), bytes });
                }
                Err(actual) => cur = actual,
            }
        }
    }

    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Acquire)
    }

    pub fn peak(&self) -> u64 {
        self.peak.load(Ordering::Acquire)
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }
}

#[derive(Debug)]
pub struct Reservation {
    budget: Arc<MemoryBudget>,
    bytes: u64,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheState {
    Hit,
    Miss,
    Off,
}

/// Content-addressed results of eligible sub-plans, keyed by a hash of the
/// sub-plan and the fingerprints of the data it reads. Bounded by bytes;
/// the oldest entries are evicted first.
pub struct CheckpointStore {
    capacity: u64,
    inner: Mutex<Entries>,
}

/// Entries by key, insertion order (for eviction), and total bytes.
type Entries = (BTreeMap<String, Arc<Batch>>, VecDeque<String>, u64);

impl CheckpointStore {
    pub fn new(capacity_bytes: u64) -> Self {
        Self { capacity: capacity_bytes, inner: Mutex::new((BTreeMap::new(), VecDeque::new(), 0)) }
    }

    pub fn get(&self, key: &str) -> Option<Arc<Batch>> {
        self.inner.lock().0.get(key).cloned()
    }

    pub fn put(&self, key: String, batch: Arc<Batch>) {
        let size = batch.bytes();
        if size > self.capacity {
            return;
        }
        let mut g = self.inner.lock();
        let (map, order, used) = &mut *g;
        if map.contains_key(&key) {
            return;
        }
        while *used + size > self.capacity {
            let Some(old) = order.pop_front() else { break };
            if let Some(b) = map.remove(&old) {
                *used -= b.bytes();
            }
        }
        *used += size;
        order.push_back(key.clone());
        map.insert(key, batch);
    }

    pub fn len(&self) -> usize {
        self.inner.lock().0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn bytes(&self) -> u64 {
        self.inner.lock().2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Column, DataType, Field, Schema};

    #[test]
    fn budget_rejects_overcommit_and_releases_on_drop() {
        let b = MemoryBudget::new(100);
        let r1 = b.reserve(60, "a").unwrap();
        assert!(matches!(b.reserve(50, "b"), Err(EngineError::MemoryExceeded { .. })));
        drop(r1);
        let _r2 = b.reserve(90, "c").unwrap();
        assert_eq!(b.used(), 90);
        assert_eq!(b.peak(), 90);
    }

    #[test]
    fn checkpoints_are_bounded() {
        let batch = |n: usize| Arc::new(Batch::new(Schema::new(vec![Field::new("x", DataType::Float64, None)]).unwrap(), vec![Column::Float64(vec![Some(1.0); n])]).unwrap());
        let s = CheckpointStore::new(16 * 25);
        s.put("a".into(), batch(10));
        s.put("b".into(), batch(10));
        assert_eq!(s.len(), 2);
        s.put("c".into(), batch(10)); // evicts "a"
        assert!(s.get("a").is_none() && s.get("c").is_some());
        s.put("huge".into(), batch(1000)); // larger than capacity: not stored
        assert!(s.get("huge").is_none());
        assert!(s.bytes() <= 16 * 25);
    }
}

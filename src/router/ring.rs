use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendReplica {
    pub id: String,
    pub url: String,
    pub weight: u32,
    pub healthy: bool,
    #[serde(default)]
    pub failure_count: u32,
}

impl BackendReplica {
    pub fn new(id: impl Into<String>, url: impl Into<String>, weight: u32) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
            weight: weight.max(1),
            healthy: true,
            failure_count: 0,
        }
    }
}

fn fnv1a_hash(data: &str) -> u64 {
    let mut hasher = fnv::FnvHasher::default();
    data.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone)]
pub struct ConsistentHashRouter {
    inner: Arc<RwLock<RouterInner>>,
    vnodes_per_weight: u32,
}

struct RouterInner {
    replicas: HashMap<String, BackendReplica>,
    ring: BTreeMap<u64, String>, // hash -> replica_id
}

impl ConsistentHashRouter {
    pub fn new(vnodes_per_weight: u32) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RouterInner {
                replicas: HashMap::new(),
                ring: BTreeMap::new(),
            })),
            vnodes_per_weight: vnodes_per_weight.max(10),
        }
    }

    pub fn add_replica(&self, replica: BackendReplica) {
        let mut inner = self.inner.write();
        let id = replica.id.clone();
        let total_vnodes = replica.weight * self.vnodes_per_weight;

        for vnode in 0..total_vnodes {
            let key = format!("replica:{}:vnode:{}", id, vnode);
            let hash = fnv1a_hash(&key);
            inner.ring.insert(hash, id.clone());
        }

        inner.replicas.insert(id, replica);
    }

    pub fn remove_replica(&self, id: &str) {
        let mut inner = self.inner.write();
        if let Some(replica) = inner.replicas.remove(id) {
            let total_vnodes = replica.weight * self.vnodes_per_weight;
            for vnode in 0..total_vnodes {
                let key = format!("replica:{}:vnode:{}", id, vnode);
                let hash = fnv1a_hash(&key);
                inner.ring.remove(&hash);
            }
        }
    }

    pub fn set_health(&self, id: &str, healthy: bool) {
        let mut inner = self.inner.write();
        if let Some(replica) = inner.replicas.get_mut(id) {
            replica.healthy = healthy;
            if healthy {
                replica.failure_count = 0;
            }
        }
    }

    pub fn record_success(&self, id: &str) {
        let mut inner = self.inner.write();
        if let Some(replica) = inner.replicas.get_mut(id) {
            replica.healthy = true;
            replica.failure_count = 0;
        }
    }

    pub fn record_failure(&self, id: &str) -> bool {
        let mut inner = self.inner.write();
        if let Some(replica) = inner.replicas.get_mut(id) {
            replica.failure_count = replica.failure_count.saturating_add(1);
            if replica.failure_count >= 3 {
                replica.healthy = false;
                tracing::warn!(
                    replica_id = %id,
                    url = %replica.url,
                    failure_count = replica.failure_count,
                    "Replica circuit breaker tripped: marked unhealthy"
                );
                return true; // Tripped
            }
        }
        false
    }

    /// Route by session ID to guarantee that multi-turn calls hit the same replica (for KV cache reuse)
    pub fn route_by_session(&self, session_id: &str) -> Option<BackendReplica> {
        self.route_by_key(session_id)
    }

    /// Route by prompt prefix hash
    pub fn route_by_prefix(&self, prefix: &str) -> Option<BackendReplica> {
        self.route_by_key(prefix)
    }

    fn route_by_key(&self, key: &str) -> Option<BackendReplica> {
        let inner = self.inner.read();
        if inner.ring.is_empty() {
            return None;
        }

        let hash = fnv1a_hash(key);

        // Find first replica at or clockwise from hash
        let replica_id = inner
            .ring
            .range(hash..)
            .next()
            .map(|(_, id)| id)
            .or_else(|| inner.ring.iter().next().map(|(_, id)| id))?;

        let replica = inner.replicas.get(replica_id)?;
        if replica.healthy {
            return Some(replica.clone());
        }

        // Fallback: look for next healthy replica on the ring
        for (_, alt_id) in inner.ring.range(hash..) {
            if let Some(alt) = inner.replicas.get(alt_id) {
                if alt.healthy {
                    return Some(alt.clone());
                }
            }
        }

        for (_, alt_id) in inner.ring.iter() {
            if let Some(alt) = inner.replicas.get(alt_id) {
                if alt.healthy {
                    return Some(alt.clone());
                }
            }
        }

        None
    }

    pub fn list_replicas(&self) -> Vec<BackendReplica> {
        let inner = self.inner.read();
        inner.replicas.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash_routing_affinity() {
        let router = ConsistentHashRouter::new(50);
        let node1 = BackendReplica::new("node-1", "http://10.0.0.1:8000", 1);
        let node2 = BackendReplica::new("node-2", "http://10.0.0.2:8000", 1);

        router.add_replica(node1.clone());
        router.add_replica(node2.clone());

        // Same session key must always resolve to the exact same node
        let session = "ep_12345_session_abc";
        let target1 = router.route_by_session(session).unwrap();
        let target2 = router.route_by_session(session).unwrap();
        assert_eq!(target1.id, target2.id);

        // Unhealthy node should automatically fall back
        let chosen_id = target1.id.clone();
        router.set_health(&chosen_id, false);

        let fallback = router.route_by_session(session).unwrap();
        assert_ne!(fallback.id, chosen_id);
        assert!(fallback.healthy);
    }

    #[test]
    fn test_circuit_breaker_trip_and_recovery() {
        let router = ConsistentHashRouter::new(50);
        let node1 = BackendReplica::new("node-1", "http://10.0.0.1:8000", 1);
        let node2 = BackendReplica::new("node-2", "http://10.0.0.2:8000", 1);

        router.add_replica(node1.clone());
        router.add_replica(node2.clone());

        let session = "session-cb-test";
        let initial = router.route_by_session(session).unwrap();

        // 1st failure: not tripped yet
        assert!(!router.record_failure(&initial.id));
        assert!(router.route_by_session(session).unwrap().id == initial.id);

        // 2nd failure: not tripped yet
        assert!(!router.record_failure(&initial.id));

        // 3rd failure: trips!
        assert!(router.record_failure(&initial.id));

        // Now routes to fallback node
        let fallback = router.route_by_session(session).unwrap();
        assert_ne!(fallback.id, initial.id);

        // Recovery
        router.record_success(&initial.id);
        let recovered = router.route_by_session(session).unwrap();
        assert_eq!(recovered.id, initial.id);
    }
}

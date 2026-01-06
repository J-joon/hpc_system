use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::lattice::{Lattice, OrderBot};

// Section 0: Primitive Types & Identifiers
pub type EID = String;
pub type NamespaceId = String;
pub type JobId = String;
pub type NodeId = String;
pub type GenId = String;
pub type Cursor = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WriteOnce<T: Eq + std::hash::Hash + Clone>(pub HashSet<T>);

impl<T: Eq + std::hash::Hash + Clone> WriteOnce<T> {
    pub fn new(val: T) -> Self {
        let mut set = HashSet::new();
        set.insert(val);
        Self(set)
    }

    pub fn choose(&self) -> Option<T> {
        self.0.iter().next().cloned()
    }
}

impl<T: Eq + std::hash::Hash + Clone> Default for WriteOnce<T> {
    fn default() -> Self {
        Self(HashSet::new())
    }
}

impl<T: Eq + std::hash::Hash + Clone> PartialOrd for WriteOnce<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.0 == other.0 {
            Some(std::cmp::Ordering::Equal)
        } else if self.0.is_subset(&other.0) {
            Some(std::cmp::Ordering::Less)
        } else if self.0.is_superset(&other.0) {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}

impl<T: Eq + std::hash::Hash + Clone> Lattice for WriteOnce<T> {
    fn join(&self, other: &Self) -> Self {
        let mut new_set = self.0.clone();
        for item in &other.0 {
            new_set.insert(item.clone());
        }
        Self(new_set)
    }

    fn meet(&self, other: &Self) -> Self {
        let new_set = self.0.intersection(&other.0).cloned().collect();
        Self(new_set)
    }
}

impl<T: Eq + std::hash::Hash + Clone> OrderBot for WriteOnce<T> {
    fn bot() -> Self {
        Self(HashSet::new())
    }
}

// Section 1: Payload Taxonomy
pub type WorkSpec = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct SysContext {
    pub job_id: Option<JobId>,
    pub ns_id: Option<NamespaceId>,
    pub node_id: Option<NodeId>,
    pub target: Option<GenId>,
    pub priority: Option<u64>,
    pub slots: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Payload {
    pub ctx: SysContext,
    pub spec: WorkSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    pub payload: Payload,
}

// Section 2: Domain Data Schema (Facts)
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct JobInfo {
    pub phase: u64,
    pub namespace_id: WriteOnce<NamespaceId>,
    pub priority: u64,
    pub resources: WriteOnce<u64>,
}

impl Default for JobInfo {
    fn default() -> Self {
        Self {
            phase: 0,
            namespace_id: OrderBot::bot(),
            priority: 0,
            resources: OrderBot::bot(),
        }
    }
}

impl Lattice for JobInfo {
    fn join(&self, other: &Self) -> Self {
        Self {
            phase: self.phase.join(&other.phase),
            namespace_id: self.namespace_id.join(&other.namespace_id),
            priority: self.priority.join(&other.priority),
            resources: self.resources.join(&other.resources),
        }
    }

    fn meet(&self, other: &Self) -> Self {
        Self {
            phase: self.phase.meet(&other.phase),
            namespace_id: self.namespace_id.meet(&other.namespace_id),
            priority: self.priority.meet(&other.priority),
            resources: self.resources.meet(&other.resources),
        }
    }
}

impl OrderBot for JobInfo {
    fn bot() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NodeInfo {
    pub down: bool,
    pub capacity: WriteOnce<u64>,
}

impl Default for NodeInfo {
    fn default() -> Self {
        Self {
            down: false,
            capacity: OrderBot::bot(),
        }
    }
}

impl Lattice for NodeInfo {
    fn join(&self, other: &Self) -> Self {
        Self {
            down: self.down.join(&other.down),
            capacity: self.capacity.join(&other.capacity),
        }
    }

    fn meet(&self, other: &Self) -> Self {
        Self {
            down: self.down.meet(&other.down),
            capacity: self.capacity.meet(&other.capacity),
        }
    }
}

impl OrderBot for NodeInfo {
    fn bot() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Facts {
    pub jobs: HashMap<JobId, JobInfo>,
    pub nodes: HashMap<NodeId, NodeInfo>,
}

impl Default for Facts {
    fn default() -> Self {
        Self {
            jobs: HashMap::new(),
            nodes: HashMap::new(),
        }
    }
}

impl PartialOrd for Facts {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Implement pointwise comparison
        let mut le = true;
        let mut ge = true;

        for (k, v) in &self.jobs {
            let other_v = other.jobs.get(k).cloned().unwrap_or_else(OrderBot::bot);
            if v > &other_v { le = false; }
            if v < &other_v { ge = false; }
        }
        for (k, v) in &other.jobs {
            if !self.jobs.contains_key(k) {
                if v > &OrderBot::bot() { le = false; }
            }
        }

        for (k, v) in &self.nodes {
            let other_v = other.nodes.get(k).cloned().unwrap_or_else(OrderBot::bot);
            if v > &other_v { le = false; }
            if v < &other_v { ge = false; }
        }
        for (k, v) in &other.nodes {
            if !self.nodes.contains_key(k) {
                if v > &OrderBot::bot() { le = false; }
            }
        }

        match (le, ge) {
            (true, true) => Some(std::cmp::Ordering::Equal),
            (true, false) => Some(std::cmp::Ordering::Less),
            (false, true) => Some(std::cmp::Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl Lattice for Facts {
    fn join(&self, other: &Self) -> Self {
        let mut jobs = self.jobs.clone();
        for (k, v) in &other.jobs {
            let entry = jobs.entry(k.clone()).or_insert_with(OrderBot::bot);
            *entry = entry.join(v);
        }

        let mut nodes = self.nodes.clone();
        for (k, v) in &other.nodes {
            let entry = nodes.entry(k.clone()).or_insert_with(OrderBot::bot);
            *entry = entry.join(v);
        }

        Self { jobs, nodes }
    }

    fn meet(&self, other: &Self) -> Self {
        let mut jobs = HashMap::new();
        let all_job_keys: HashSet<_> = self.jobs.keys().chain(other.jobs.keys()).collect();
        for k in all_job_keys {
            let v1 = self.jobs.get(k).cloned().unwrap_or_else(OrderBot::bot);
            let v2 = other.jobs.get(k).cloned().unwrap_or_else(OrderBot::bot);
            let meet_v = v1.meet(&v2);
            if meet_v != OrderBot::bot() {
                jobs.insert(k.clone(), meet_v);
            }
        }

        let mut nodes = HashMap::new();
        let all_node_keys: HashSet<_> = self.nodes.keys().chain(other.nodes.keys()).collect();
        for k in all_node_keys {
            let v1 = self.nodes.get(k).cloned().unwrap_or_else(OrderBot::bot);
            let v2 = other.nodes.get(k).cloned().unwrap_or_else(OrderBot::bot);
            let meet_v = v1.meet(&v2);
            if meet_v != OrderBot::bot() {
                nodes.insert(k.clone(), meet_v);
            }
        }

        Self { jobs, nodes }
    }
}

impl OrderBot for Facts {
    fn bot() -> Self {
        Self::default()
    }
}

// Section 3: Control Plane Structures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Int,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldReq {
    pub key: String,
    pub required: bool,
    pub expected_type: FieldType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamespaceSpec {
    pub preemptible: bool,
    pub handler_type: String,
    pub schema: Vec<FieldReq>,
}

pub type DeltaFn = fn(&Event) -> Facts;

fn dummy_delta_fn() -> DeltaFn {
    |_| OrderBot::bot()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GeneratorSpec {
    #[serde(skip, default = "dummy_delta_fn")]
    pub delta_fn: DeltaFn,
    pub kind: String,
    pub callback_url: Option<String>,
}

impl std::fmt::Debug for GeneratorSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GenSpec({})", self.kind)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Registration {
    pub namespaces: HashMap<NamespaceId, NamespaceSpec>,
    pub generators: HashMap<GenId, GeneratorSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlRequest {
    RegisterNamespace(NamespaceId, NamespaceSpec),
    RegisterGenerator(GenId, GeneratorSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poke {
    pub gid: GenId,
}

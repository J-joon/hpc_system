use hpc_core::domain::{
    Event, Facts, JobInfo, NamespaceSpec, FieldReq, FieldType, 
    GeneratorSpec, WriteOnce, WorkSpec
};
use hpc_core::lattice::OrderBot;
use std::collections::HashMap;

pub const BATCH_NS_ID: &str = "batch";
pub const BATCH_GEN_ID: &str = "batch_generator";

pub struct BatchJob {
    pub command: String,
    pub cpu: u64,
    pub memory: u64,
}

impl BatchJob {
    pub fn from_spec(spec: &WorkSpec) -> Option<Self> {
        let command = spec.get("command")?.clone();
        let cpu = spec.get("cpu")?.parse().ok()?;
        let memory = spec.get("memory")?.parse().ok()?;
        Some(Self { command, cpu, memory })
    }

    pub fn to_spec(&self) -> WorkSpec {
        let mut spec = HashMap::new();
        spec.insert("command".to_string(), self.command.clone());
        spec.insert("cpu".to_string(), self.cpu.to_string());
        spec.insert("memory".to_string(), self.memory.to_string());
        spec
    }
}

pub fn batch_delta_fn(e: &Event) -> Facts {
    let mut facts = Facts::bot();
    
    if e.kind == "SubmitJob" {
        if let Some(job_id) = &e.payload.ctx.job_id {
            if let Some(batch_job) = BatchJob::from_spec(&e.payload.spec) {
                let job_info = JobInfo {
                    phase: 1, // Phase 1: Submitted
                    namespace_id: WriteOnce::new(BATCH_NS_ID.to_string()),
                    priority: e.payload.ctx.priority.unwrap_or(0),
                    resources: WriteOnce::new(batch_job.cpu),
                };
                facts.jobs.insert(job_id.clone(), job_info);
            }
        }
    }
    
    facts
}

pub fn get_batch_namespace_spec() -> NamespaceSpec {
    NamespaceSpec {
        preemptible: false,
        handler_type: "batch_executor".to_string(),
        schema: vec![
            FieldReq {
                key: "command".to_string(),
                required: true,
                expected_type: FieldType::String,
            },
            FieldReq {
                key: "cpu".to_string(),
                required: true,
                expected_type: FieldType::Int,
            },
            FieldReq {
                key: "memory".to_string(),
                required: true,
                expected_type: FieldType::Int,
            },
        ],
    }
}

pub fn get_batch_generator_spec() -> GeneratorSpec {
    GeneratorSpec {
        delta_fn: batch_delta_fn,
        kind: "batch".to_string(),
        callback_url: None,
    }
}


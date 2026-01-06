use hpc_core::domain::{
    Event, Facts, JobInfo, NamespaceSpec, FieldReq, FieldType, 
    GeneratorSpec, WriteOnce, WorkSpec
};
use hpc_core::lattice::OrderBot;
use std::collections::HashMap;

pub const SIM_NS_ID: &str = "simulation";

pub struct SimulationJob {
    pub env_cmd: String,
    pub policy_cmd: String,
}

impl SimulationJob {
    pub fn from_spec(spec: &WorkSpec) -> Option<Self> {
        let env_cmd = spec.get("env_cmd")?.clone();
        let policy_cmd = spec.get("policy_cmd")?.clone();
        Some(Self { env_cmd, policy_cmd })
    }

    pub fn to_spec(&self) -> WorkSpec {
        let mut spec = HashMap::new();
        spec.insert("env_cmd".to_string(), self.env_cmd.clone());
        spec.insert("policy_cmd".to_string(), self.policy_cmd.clone());
        spec
    }
}

pub fn simulation_delta_fn(e: &Event) -> Facts {
    let mut facts = Facts::bot();
    let job_id = match &e.payload.ctx.job_id {
        Some(id) => id.clone(),
        None => return facts,
    };

    match e.kind.as_str() {
        "SubmitSimulation" => {
            if let Some(_) = SimulationJob::from_spec(&e.payload.spec) {
                let job_info = JobInfo {
                    phase: 1, // Submitted
                    namespace_id: WriteOnce::new(SIM_NS_ID.to_string()),
                    ..Default::default()
                };
                facts.jobs.insert(job_id, job_info);
            }
        }
        "PortAllocated" => {
            // Log port in resources or metadata (mocked here as phase change for demo)
            let mut job_info = JobInfo::default();
            if let Some(port) = e.payload.spec.get("port") {
                if let Ok(p) = port.parse::<u64>() {
                    job_info.resources = WriteOnce::new(p);
                }
            }
            facts.jobs.insert(job_id, job_info);
        }
        "SimulationStarted" => {
            let mut job_info = JobInfo::default();
            job_info.phase = 2; // Running
            facts.jobs.insert(job_id, job_info);
        }
        "SimulationFinished" => {
            let mut job_info = JobInfo::default();
            job_info.phase = 3; // Success
            facts.jobs.insert(job_id, job_info);
        }
        _ => {}
    }
    
    facts
}

pub fn get_simulation_namespace_spec() -> NamespaceSpec {
    NamespaceSpec {
        preemptible: false,
        handler_type: "simulation_executor".to_string(),
        schema: vec![
            FieldReq {
                key: "env_cmd".to_string(),
                required: true,
                expected_type: FieldType::String,
            },
            FieldReq {
                key: "policy_cmd".to_string(),
                required: true,
                expected_type: FieldType::String,
            },
        ],
    }
}

pub fn get_simulation_generator_spec() -> GeneratorSpec {
    GeneratorSpec {
        delta_fn: simulation_delta_fn,
        kind: SIM_NS_ID.to_string(),
        callback_url: None,
    }
}


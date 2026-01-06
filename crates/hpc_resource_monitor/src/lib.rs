use std::time::Duration;
use sysinfo::System;
use hpc_core::domain::{
    Event, Facts, NodeInfo, NamespaceSpec, FieldReq, FieldType, 
    GeneratorSpec, WriteOnce, Payload, SysContext
};
use hpc_core::lattice::OrderBot;
use std::collections::HashMap;
use anyhow::Result;

pub const MONITOR_NS_ID: &str = "monitor";
pub const MONITOR_GEN_ID: &str = "monitor_generator";

/// Delta function for the Hub to interpret HardwareStats events
pub fn monitor_delta_fn(e: &Event) -> Facts {
    let mut facts = Facts::bot();
    
    if e.kind == "HardwareStats" {
        if let Some(node_id) = &e.payload.ctx.node_id {
            // Map CPU usage string back to u64 for the lattice state (approximate)
            let cpu_usage: u64 = e.payload.spec.get("cpu_usage")
                .and_then(|v| v.parse::<f32>().ok())
                .map(|f| f as u64)
                .unwrap_or(0);

            let node_info = NodeInfo {
                down: false,
                capacity: WriteOnce::new(cpu_usage),
            };
            facts.nodes.insert(node_id.clone(), node_info);
        }
    }
    
    facts
}

pub fn get_monitor_namespace_spec() -> NamespaceSpec {
    NamespaceSpec {
        preemptible: true, // Hardware stats are low-priority/informational
        handler_type: "resource_logger".to_string(),
        schema: vec![
            FieldReq { key: "cpu_usage".to_string(), required: true, expected_type: FieldType::String },
            FieldReq { key: "mem_used_kb".to_string(), required: true, expected_type: FieldType::Int },
            FieldReq { key: "mem_total_kb".to_string(), required: true, expected_type: FieldType::Int },
        ],
    }
}

pub fn get_monitor_generator_spec() -> GeneratorSpec {
    GeneratorSpec {
        delta_fn: monitor_delta_fn,
        kind: MONITOR_NS_ID.to_string(),
        callback_url: None, // Hub just logs these, no one "pokes" yet
    }
}

pub struct ResourceMonitor {
    hub_url: String,
    node_id: String,
    interval: Duration,
    sys: System,
}

impl ResourceMonitor {
    pub fn new(hub_url: String, node_id: String, interval_secs: u64) -> Self {
        Self {
            hub_url,
            node_id,
            interval: Duration::from_secs(interval_secs),
            sys: System::new_all(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("[Monitor] Starting hardware monitor for node: {}", self.node_id);
        let client = reqwest::Client::new();

        loop {
            self.sys.refresh_cpu();
            self.sys.refresh_memory();

            let mut stats = HashMap::new();
            stats.insert("cpu_usage".to_string(), format!("{:.2}", self.sys.global_cpu_info().cpu_usage()));
            stats.insert("mem_used_kb".to_string(), self.sys.used_memory().to_string());
            stats.insert("mem_total_kb".to_string(), self.sys.total_memory().to_string());

            let event = Event {
                kind: "HardwareStats".to_string(),
                payload: Payload {
                    ctx: SysContext {
                        node_id: Some(self.node_id.clone()),
                        ns_id: Some(MONITOR_NS_ID.to_string()),
                        ..Default::default()
                    },
                    spec: stats,
                },
            };

            match client.post(format!("{}/ingest", self.hub_url))
                .json(&event)
                .send()
                .await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        println!("[Monitor] Logged hardware stats to Hub");
                    } else {
                        eprintln!("[Monitor] Hub returned error: {}", resp.status());
                    }
                }
                Err(e) => {
                    eprintln!("[Monitor] Failed to send stats to Hub: {}", e);
                }
            }

            tokio::time::sleep(self.interval).await;
        }
    }
}

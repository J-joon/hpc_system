use hpc_logic::Hub;
use hpc_core::domain::{Event, Payload, SysContext};
use hpc_ns_batch::{BatchJob, BATCH_NS_ID};
use std::sync::Arc;

/// Simulates receiving a batch job submission
pub async fn simulate_incoming_batch_job(hub: Arc<Hub>) {
    let batch_job = BatchJob {
        command: "ls -R /".to_string(),
        cpu: 4,
        memory: 1024,
    };

    let event = Event {
        kind: "SubmitJob".to_string(),
        payload: Payload {
            ctx: SysContext {
                job_id: Some("batch-job-001".into()),
                ns_id: Some(BATCH_NS_ID.to_string()),
                priority: Some(10),
                ..Default::default()
            },
            spec: batch_job.to_spec(),
        },
    };
    
    match hub.ingest_event(event).await {
        Ok(pokes) => {
            println!("Batch job ingested successfully!");
            println!("Generated {} pokes", pokes.len());
            for poke in pokes {
                println!("  - Poke for generator: {}", poke.gid);
            }
        },
        Err(e) => eprintln!("Error ingesting batch job: {}", e),
    }
}

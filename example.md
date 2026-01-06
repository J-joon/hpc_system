# HPC System Example Guide

This guide explains how to run the demo and how to extend the system by writing your own namespaces and executors.

## 1. Running the Demo

The demo script (`demo.sh`) automates the setup of a Hub, a Relay, and a Resource Monitor, followed by submitting both a `batch` job and a `simulation` job.

```bash
./demo.sh
```

### What happens in the demo:
1. **Hub** starts on port 8080.
2. **Relay** starts on port 8081, registers for the `batch` namespace, and listens for pokes.
3. **Monitor** starts, registers the `monitor` namespace, and begins sending hardware stats.
4. **Submit** sends a `SubmitJob` (batch) and a `SubmitSimulation` (complex) request to the Hub.
5. **Relay** receives pokes, pulls the jobs, and executes them:
   - **Batch**: Simply runs a command and reports `JobStarted` -> `JobFinished`.
   - **Simulation**: Starts an environment, discovers its port, reports `PortAllocated`, then starts the policy module.

---

## 2. Writing Your Own Namespace

A namespace defines the **semantics** of a job. To create one, you need to define:
1. **WorkSpec**: The data schema for your job (e.g., `cpu`, `memory`, `command`).
2. **Delta Function**: Pure logic that converts an `Event` into `Facts` (state).
3. **NamespaceSpec**: metadata about the namespace (preemptible, schema).

### Example: A "Health Check" Namespace

```rust
// Define your job structure
pub struct HealthCheckJob {
    pub target_url: String,
}

// Implement the Delta Function
pub fn health_delta_fn(e: &Event) -> Facts {
    let mut facts = Facts::bot();
    if e.kind == "TriggerHealthCheck" {
        if let Some(job_id) = &e.payload.ctx.job_id {
            // Update state to phase 1 (Submitted)
            facts.jobs.insert(job_id.clone(), JobInfo {
                phase: 1,
                ..Default::default()
            });
        }
    }
    facts
}
```

---

## 3. Writing Your Own Executor

An executor handles the **side effects** (running processes, network calls). It listens for pokes from the Hub and acts on them.

### Example: Custom Relay Logic

In `hpc_node/src/main.rs`, you can extend the `relay_poke_handler` to handle your new namespace:

```rust
if gen_id.contains("health_check") {
    tokio::spawn(async move {
        println!("[Relay] Running health check for {}", job_id);
        
        // 1. Report Start
        ingest_event_to_hub(&hub_url, &Event { kind: "CheckStarted", ... }).await;

        // 2. Do the work
        let resp = reqwest::get(target_url).await;
        
        // 3. Report result
        if resp.is_ok() {
            ingest_event_to_hub(&hub_url, &Event { kind: "CheckPassed", ... }).await;
        } else {
            ingest_event_to_hub(&hub_url, &Event { kind: "CheckFailed", ... }).await;
        }
    });
}
```

---

## 4. Complex Job Coordination (Env + Policy)

The `simulation` namespace demonstrates how to coordinate multiple processes:

1. **Relay** starts the `env` process.
2. **Env** picks a random port and writes it to a file.
3. **Relay** reads the file and sends a `PortAllocated` event to the Hub.
4. **Relay** starts the `policy` process, passing the port as an argument.

This ensures that the "discovered" port is logged in the Hub's immutable event log for auditing purposes, while still allowing the two local processes to communicate.


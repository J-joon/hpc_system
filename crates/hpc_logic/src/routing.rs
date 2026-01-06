use hpc_core::domain::Job;

/// Determines if a job should be handled locally or relayed.
/// This could be part of the verified logic in Lean.
pub fn should_relay(job: &Job) -> bool {
    // Example: If tagged "gpu", relay to specific GPU node
    job.tags.contains(&"gpu".to_string())
}
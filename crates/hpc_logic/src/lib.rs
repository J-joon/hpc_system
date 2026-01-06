use std::sync::Arc;
use std::collections::HashMap;
use hpc_core::contracts::{StorageBackend, TransportBackend, DynResult};
use hpc_core::domain::{
    Event, Facts, Registration, ControlRequest, Cursor, Poke, 
    FieldReq, FieldType, GenId, WorkSpec, DeltaFn
};
use hpc_core::lattice::{Lattice, OrderBot};

pub struct Hub {
    storage: Arc<dyn StorageBackend>,
    transport: Arc<dyn TransportBackend>,
    pub registry: Registration,
    pub delta_fns: HashMap<String, DeltaFn>,
}

impl Hub {
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        transport: Arc<dyn TransportBackend>,
    ) -> Self {
        Self {
            storage,
            transport,
            registry: Registration::default(),
            delta_fns: HashMap::new(),
        }
    }

    /// 4.2) Entry Point: Handle Control Request
    pub fn process_control(&mut self, req: ControlRequest) {
        match req {
            ControlRequest::RegisterNamespace(ns, spec) => {
                self.registry.namespaces.insert(ns, spec);
            }
            ControlRequest::RegisterGenerator(gid, spec) => {
                self.registry.generators.insert(gid, spec);
            }
        }
    }

    /// 4.3) WF Check (Read-Only)
    pub fn validate_spec(schema: &[FieldReq], spec: &WorkSpec) -> bool {
        schema.iter().all(|req| {
            match spec.get(&req.key) {
                None => !req.required,
                Some(val) => {
                    match req.expected_type {
                        FieldType::String => true,
                        FieldType::Int => val.parse::<u64>().is_ok(),
                        FieldType::Bool => val == "true" || val == "false",
                    }
                }
            }
        })
    }

    pub fn check_wf(&self, e: &Event) -> bool {
        match e.kind.as_str() {
            "SubmitJob" => {
                if let Some(ns_id) = &e.payload.ctx.ns_id {
                    if let Some(ns_spec) = self.registry.namespaces.get(ns_id) {
                        return Self::validate_spec(&ns_spec.schema, &e.payload.spec);
                    }
                }
                false // Namespace not registered or no ns_id
            }
            _ => true, // Other events are whitelisted
        }
    }

    /// 4.4) Delta Logic (Per Generator)
    pub fn compute_delta(&self, gid: &GenId, e: &Event) -> Facts {
        if let Some(spec) = self.registry.generators.get(gid) {
            // Try local registry first (for serialized/distributed setups), then fallback to the spec's fn
            if let Some(f) = self.delta_fns.get(&spec.kind) {
                f(e)
            } else {
                (spec.delta_fn)(e)
            }
        } else {
            OrderBot::bot()
        }
    }

    /// 4.5) Poke Logic
    pub fn check_poke(&self, gid: &GenId, e: &Event) -> Option<Poke> {
        if self.compute_delta(gid, e) != OrderBot::bot() {
            Some(Poke { gid: gid.clone() })
        } else {
            None
        }
    }

    /// 6.1) Entry Point: Ingest Event (Data Plane)
    pub async fn ingest_event(&self, e: Event) -> DynResult<Vec<Poke>> {
        // 1. WF Check
        if self.check_wf(&e) {
            // 2. Append to Data Plane Log
            self.storage.append_event(&e).await?;

            // 3. Check Pokes (for ALL registered generators)
            let mut pokes = Vec::new();
            for (gid, spec) in self.registry.generators.iter() {
                if let Some(poke) = self.check_poke(gid, &e) {
                    println!("[Hub] Generated poke for generator: {}", gid);
                    pokes.push(poke.clone());
                    // 4. Notify transport with callback URL if available
                    if let Some(url) = &spec.callback_url {
                        println!("[Hub] Sending poke to {} at {}", gid, url);
                        self.transport.send_poke_to_url(&poke, url).await?;
                    } else {
                        self.transport.send_poke(&poke).await?;
                    }
                }
            }
            Ok(pokes)
        } else {
            Ok(vec![]) // Rejected by WF
        }
    }

    /// 5.1) Pull Logic
    pub async fn handle_pull(&self, gid: &GenId, cursor: Cursor) -> DynResult<Facts> {
        let events = self.storage.get_events(cursor).await?;
        let mut delta: Facts = OrderBot::bot();
        for e in events {
            delta = delta.join(&self.compute_delta(gid, &e));
        }
        Ok(delta)
    }
}

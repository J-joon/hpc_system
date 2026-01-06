use async_trait::async_trait;
use hpc_core::contracts::{TransportBackend, DynResult};
use hpc_core::domain::Poke;

pub struct HttpTransport {
    pub base_url: String,
}

impl HttpTransport {
    pub fn new(url: &str) -> Self {
        Self { base_url: url.to_string() }
    }
}

#[async_trait]
impl TransportBackend for HttpTransport {
    async fn send_poke(&self, poke: &Poke) -> DynResult<()> {
        println!("[Transport] Poking generator: {}", poke.gid);
        Ok(())
    }
}

//! Reads cloudflared's local endpoints, served on loopback by `--metrics`. Each reading is kept with
//! its failure, so the panel can say which input it lacked.

use std::time::Duration;

use ureq::Agent;

pub const ENDPOINTS: [&str; 3] = ["/metrics", "/ready", "/config"];

#[derive(Debug, Clone)]
pub struct Readings {
    pub metrics: Result<String, String>,
    pub ready: Result<String, String>,
    pub config: Result<String, String>,
}

impl Readings {
    pub fn read(addr: &str) -> Self {
        // /ready answers 503 with its JSON body while no connection is registered, so a status code
        // is not an error here.
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(2)))
            .http_status_as_error(false)
            .build()
            .into();
        let get = |path: &str| -> Result<String, String> {
            agent
                .get(&format!("http://{addr}{path}"))
                .call()
                .map_err(|e| e.to_string())?
                .body_mut()
                .read_to_string()
                .map_err(|e| e.to_string())
        };
        Self {
            metrics: get(ENDPOINTS[0]),
            ready: get(ENDPOINTS[1]),
            config: get(ENDPOINTS[2]),
        }
    }

    /// Each endpoint with its failure, for those that failed.
    pub fn failures(&self) -> Vec<(&'static str, &str)> {
        ENDPOINTS
            .iter()
            .zip([&self.metrics, &self.ready, &self.config])
            .filter_map(|(path, r)| r.as_ref().err().map(|e| (*path, e.as_str())))
            .collect()
    }
}

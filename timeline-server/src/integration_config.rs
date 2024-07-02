use serde::{Deserialize, Serialize};

use crate::integration::IntegrationT;
use crate::integrations::paperless_ngx;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum IntegrationConfig {
    PaperlessNGX { host: String, token: String },
}

impl IntegrationConfig {
    pub fn into_integration(self) -> impl IntegrationT + Clone + Send {
        match self {
            IntegrationConfig::PaperlessNGX { host, token } => paperless_ngx::new(host, token),
        }
    }
}

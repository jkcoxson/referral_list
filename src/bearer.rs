// Jackson Coxson & Adam Morgan

use base64::Engine;
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct BearerToken {
    pub token: String,
    pub claims: Claims,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(rename = "missionId")]
    pub mission_id: usize,
}

impl BearerToken {
    pub fn from_base64(token: String) -> anyhow::Result<Self> {
        let parts = token.split('.').nth(1);
        if let Some(part) = parts {
            let padding_needed = (4 - part.len() % 4) % 4;
            let padded_claims = format!("{}{}", part, "=".repeat(padding_needed));
            let claims = base64::engine::general_purpose::URL_SAFE.decode(padded_claims)?;
            let claims = serde_json::from_slice::<Claims>(&claims)?;
            Ok(Self { token, claims })
        } else {
            error!("Bearer token doesn't contain enough sections");
            Err(anyhow::anyhow!(
                "Bearer token doesn't contain enough sections"
            ))
        }
    }
}

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
        // Split the token into parts (header, claims, signature)
        let parts: Vec<&str> = token.split('.').collect();

        if parts.len() != 3 {
            error!("Bearer token doesn't contain the expected number of parts.");
            return Err(anyhow::anyhow!(
                "Bearer token doesn't contain the expected number of parts"
            ));
        }

        let claims_part = parts[1]; // The middle part contains the claims

        // Calculate and apply the necessary padding
        let padding_needed = (4 - claims_part.len() % 4) % 4;
        let padded_claims = format!("{}{}", claims_part, "=".repeat(padding_needed));

        // Decode from base64 URL-safe
        match base64::engine::general_purpose::URL_SAFE.decode(padded_claims) {
            Ok(decoded_claims) => {
                // Deserialize into the Claims struct
                match serde_json::from_slice::<Claims>(&decoded_claims) {
                    Ok(claims) => Ok(Self { token, claims }),
                    Err(e) => {
                        error!("Failed to deserialize claims: {:?}", e);
                        Err(anyhow::anyhow!("Failed to deserialize claims"))
                    }
                }
            }
            Err(e) => {
                error!("Failed to decode base64 claims: {:?}", e);
                Err(anyhow::anyhow!("Failed to decode base64 claims"))
            }
        }
    }
}

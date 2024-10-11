// Jackson Coxson
// Code to interact with church servers

use std::{io::Write, sync::Arc};

use reqwest::{redirect::Policy, Client};
use reqwest_cookie_store::CookieStoreMutex;
use serde::Deserialize;
use serde_json::json;

use crate::env;

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.82 Safari/537.36";

pub struct ChurchClient {
    http_client: Client,
    cookie_store: Arc<CookieStoreMutex>,
    env: env::Env,
    bearer_token: Option<String>,
}

impl ChurchClient {
    pub async fn new(env: env::Env) -> anyhow::Result<Self> {
        let cookie_store_path = std::env::var("COOKIE_STORE_PATH").unwrap();

        // Check if the bearer token exists
        let bearer_token = std::fs::read_to_string(&env.bearer_token_path).ok();

        // Check if the file exists
        if !std::fs::exists(&cookie_store_path)? {
            std::fs::write(&cookie_store_path, "[]".as_bytes())?;
        }
        let cookie_store = {
            let file = std::fs::File::open(&cookie_store_path)
                .map(std::io::BufReader::new)
                .unwrap();
            // use re-exported version of `CookieStore` for crate compatibility
            reqwest_cookie_store::CookieStore::load_json(file).unwrap()
        };
        let cookie_store = reqwest_cookie_store::CookieStoreMutex::new(cookie_store);
        let cookie_store = std::sync::Arc::new(cookie_store);

        let http_client = Client::builder()
            .user_agent(USER_AGENT)
            .cookie_provider(Arc::clone(&cookie_store))
            .redirect(Policy::custom(|a| {
                println!("Redir to {}", a.url());
                if a.previous().len() > 5 {
                    a.stop()
                } else {
                    a.follow()
                }
            }))
            .build()
            .expect("Couldn't build the HTTP client");
        Ok(Self {
            http_client,
            cookie_store,
            env,
            bearer_token,
        })
    }

    pub async fn save_cookies(&self) -> anyhow::Result<()> {
        let mut writer = std::fs::File::create(&self.env.cookie_store_path)
            .map(std::io::BufWriter::new)
            .unwrap();
        let store = self.cookie_store.lock().unwrap();
        store.save_json(&mut writer).unwrap();
        Ok(())
    }

    async fn write_bearer_token(&self, token: &str) -> anyhow::Result<()> {
        let mut writer = std::fs::File::create(&self.env.bearer_token_path)
            .map(std::io::BufWriter::new)
            .unwrap();
        writer.write_all(token.as_bytes())?;
        Ok(())
    }

    /// Logs into churchofjesuschrist.org
    pub async fn login(&mut self) -> anyhow::Result<()> {
        self.cookie_store.lock().unwrap().clear();

        // Get the inital login page
        let res = self
            .http_client
            .get("https://referralmanager.churchofjesuschrist.org")
            .send()
            .await?
            .text()
            .await?;

        // Extract the JSON embedded in the HTML
        let start_token = "\"stateToken\":\"";
        let end_token = "\",";

        let start_index = res
            .find(start_token)
            .ok_or_else(|| anyhow::anyhow!("stateToken not found in response"))?
            + start_token.len();

        let end_index = res[start_index..]
            .find(end_token)
            .ok_or_else(|| anyhow::anyhow!("End token not found in response"))?
            + start_index;

        // Ensure the indices are valid
        if start_index >= end_index {
            return Err(anyhow::anyhow!("Invalid indices for stateToken extraction"));
        }

        let state_token = &res[start_index..end_index];
        let state_token = decode_escape_sequences(state_token)?;
        let state_token: String = serde_json::from_str(&format!("\"{state_token}\"")).unwrap();

        #[derive(Deserialize)]
        struct StateHandle {
            #[serde(rename = "stateHandle")]
            state_handle: String,
        }
        // Trade the state token for the state handle
        let state_handle = self
            .http_client
            .post("https://id.churchofjesuschrist.org/idp/idx/introspect")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(format!("{{\"stateToken\": \"{state_token}\"}}"))
            .send()
            .await?
            .json::<StateHandle>()
            .await?
            .state_handle;

        // Send the username
        let body = json!({
            "stateHandle": state_handle,
            "identifier": self.env.church_username
        })
        .to_string();
        let state_handle = self
            .http_client
            .post("https://id.churchofjesuschrist.org/idp/idx/identify")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body)
            .send()
            .await?
            .json::<StateHandle>()
            .await?
            .state_handle;

        // Send the password
        #[derive(Deserialize)]
        struct PasswordResponse {
            success: SuccessResponse,
        }

        #[derive(Deserialize)]
        struct SuccessResponse {
            href: String,
        }
        let body = json!({
            "stateHandle": state_handle,
            "credentials": {
                "passcode": self.env.church_password
            }
        })
        .to_string();
        let res = self
            .http_client
            .post("https://id.churchofjesuschrist.org/idp/idx/challenge/answer")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body)
            .send()
            .await?
            .json::<PasswordResponse>()
            .await?;

        // Set cookies
        self.http_client.get(res.success.href).send().await?;

        // Get the bearer token
        let token = self
            .http_client
            .get("https://referralmanager.churchofjesuschrist.org/services/auth")
            .header("Accept", "application/json, text/plain, */*")
            .header("Authorization", "")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?["token"]
            .clone();
        let token = match token {
            serde_json::Value::String(t) => Ok(t),
            _ => Err(anyhow::anyhow!("No token in response json")),
        }?;

        self.save_cookies().await?;
        self.write_bearer_token(&token).await?;
        self.bearer_token = Some(token);

        Ok(())
    }
}

/// Function to decode escape sequences including \xNN
fn decode_escape_sequences(s: &str) -> anyhow::Result<String> {
    // Replace URL encoded sequences
    let decoded_string = s
        .replace("\\x2D", "-") // Replace \x2D with '-'
        .replace("\\x5F", "_") // Replace \x5F with '_'
        .replace("\\x2E", ".") // Replace \x2E with '.'
        .replace("\\x2F", "/") // Replace \x2F with '/'
        .replace("\\x3D", "="); // Replace \x3D with '='

    Ok(decoded_string.to_string())
}

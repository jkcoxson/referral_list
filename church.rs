// Jackson Coxson
// Code to interact with church servers

use std::{
    io::Write,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use chrono::NaiveDateTime;
use log::{info, warn};
use reqwest::{redirect::Policy, Client};
use reqwest_cookie_store::CookieStoreMutex;
use serde::Deserialize;
use serde_json::json;

use crate::{bearer::BearerToken, env, persons};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/93.0.4577.82 Safari/537.36";
const MAX_RETRIES: u8 = 3;

#[derive(Debug)]
pub struct ChurchClient {
    http_client: Client,
    cookie_store: Arc<CookieStoreMutex>,
    pub env: env::Env,
    bearer_token: Option<BearerToken>,
    //pub holly_config: Option<crate::holly::config::Config>,
}

impl ChurchClient {
    pub async fn new(env: env::Env) -> anyhow::Result<Self> {
        // Check if the bearer token exists
        let bearer_path = PathBuf::from_str(&env.working_path)?.join("bearer.token");
        let cookies_path = PathBuf::from_str(&env.working_path)?.join("cookies.json");

        let bearer_token = if let Ok(b) = std::fs::read_to_string(&bearer_path) {
            Some(BearerToken::from_base64(b)?)
        } else {
            info!("No bearer token saved");
            None
        };

        // Check if the file exists
        if !std::fs::exists(&cookies_path)? {
            info!("No cookies saved");
            std::fs::write(&cookies_path, "".as_bytes())?;
        }
        let cookie_store = {
            let file = std::fs::File::open(&cookies_path)
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
                if a.previous().len() > 2 {
                    a.stop()
                } else {
                    info!("Redirecting to {}", a.url());
                    a.follow()
                }
            }))
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Couldn't build the HTTP client");

        //let holly_config = crate::holly::config::Config::potential_load(&env).await?;

        Ok(Self {
            http_client,
            cookie_store,
            env,
            bearer_token,
            //holly_config,
        })
    }

    pub async fn save_cookies(&self) -> anyhow::Result<()> {
        info!("Saving cookies");
        let cookies_path = PathBuf::from_str(&self.env.working_path)?.join("cookies.json");
        let mut writer = std::fs::File::create(&cookies_path)
            .map(std::io::BufWriter::new)
            .unwrap();
        let store = self.cookie_store.lock().unwrap();
        store
            .save_incl_expired_and_nonpersistent_json(&mut writer)
            .unwrap();
        Ok(())
    }

    async fn write_bearer_token(&self, token: &str) -> anyhow::Result<()> {
        info!("Saving bearer token");
        let bearer_path = PathBuf::from_str(&self.env.working_path)?.join("bearer.token");
        let mut writer = std::fs::File::create(&bearer_path)
            .map(std::io::BufWriter::new)
            .unwrap();
        writer.write_all(token.as_bytes())?;
        Ok(())
    }

    /// Logs into churchofjesuschrist.org
    pub async fn login(&mut self) -> anyhow::Result<BearerToken> {
        info!("Logging into referral manager");
        self.cookie_store.lock().unwrap().clear();

        // Get the inital login page
        info!("Loading the initial login page");
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
        info!("Trading the token for the state handle");
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
        info!("Sending the username");
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

        info!("Sending the password");
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
        info!("Getting the success href");
        self.http_client.get(res.success.href).send().await?;

        // Get the bearer token
        info!("Getting the bearer token");
        let token = self
            .http_client
            .get("https://referralmanager.churchofjesuschrist.org/services/auth")
            .header("Accept", "application/json")
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

        let token = BearerToken::from_base64(token)?;
        self.bearer_token = Some(token.clone());

        Ok(token)
    }

    /// Gets the list of everyone from the referral manager. This is a HUGE request at roughly 8mb in the CSDM
    pub async fn get_people_list(&mut self) -> anyhow::Result<Vec<persons::Person>> {
        info!("Getting the people list from referral manager");
        let mut tries = 0;

        while tries < MAX_RETRIES {
            let token = match &self.bearer_token {
                Some(t) => t,
                None => &self.login().await?,
            };
            tries += 1;
            if let Ok(list) = self.http_client.get(format!("https://referralmanager.churchofjesuschrist.org/services/people/mission/{}?includeDroppedPersons=true", token.claims.mission_id))
            .header("Authorization", format!("Bearer {}", token.token))
            .send().await {
                if let Ok(list) = list.json::<serde_json::Value>().await {
                    let list = persons::Person::parse_lossy(list);
                    info!("Received {} people from referral manager", list.len());
                    return Ok(list);
                } else {
                    warn!("Getting the people list failed at JSON parse");
                    self.bearer_token = None;
                }
            } else {
                warn!("Getting the people list failed at the request");
                self.bearer_token = None;
            }
        }
        Err(anyhow::anyhow!("Max tries exceeded"))
    }

    /// Gets a cached list from referral manager to save trips to church servers.
    /// A cache will be considered 'hit' if the list is less than an hour old.
    pub async fn get_cached_people_list(&mut self) -> anyhow::Result<Vec<persons::Person>> {
        let lists_path = PathBuf::from_str(&self.env.working_path)?.join("people_lists");
        std::fs::create_dir_all(&lists_path)?;

        let now = SystemTime::now();
        let now = now
            .duration_since(UNIX_EPOCH)
            .context("Your clock is wrong")?
            .as_secs();

        // Read all the entries in the cache
        for f in std::fs::read_dir(&lists_path)? {
            match f {
                Ok(f) if f.file_type()?.is_file() => {
                    if let Ok(file_name) = f.file_name().into_string() {
                        if let Some(file_name) = file_name.split_once('.') {
                            if let Ok(timestamp) = file_name.0.parse::<u64>() {
                                if let Some(diff) = now.checked_sub(timestamp) {
                                    if diff < 60 * 60 {
                                        info!("Cache hit");
                                        return Ok(persons::Person::parse_lossy(
                                            serde_json::from_str(
                                                &std::fs::read_to_string(f.path()).unwrap(),
                                            )?,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => (),
            }
        }
        info!("Cache miss");
        let list = self.get_people_list().await?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(lists_path.join(format!("{now}.json")))?;
        serde_json::to_writer(file, &json!({"persons": &list}))?;
        Ok(list)
    }

    pub async fn get_person_timeline(
        &mut self,
        person: &persons::Person,
    ) -> anyhow::Result<Vec<persons::TimelineEvent>> {
        info!("Getting timeline for {}", person.guid);
        let mut tries = 0;

        while tries < MAX_RETRIES {
            tries += 1;
            if let Ok(list) = self
                .http_client
                .get(format!(
                    "https://referralmanager.churchofjesuschrist.org/services/progress/timeline/{}",
                    person.guid
                ))
                .send()
                .await
            {
                if let Ok(list) = list.json::<serde_json::Value>().await {
                    let mut list: Vec<persons::TimelineEvent> = persons::TimelineEvent::parse_lossy(list);
                    
                    //Apply MST to EST conversion for each event
                    for event in &mut list {
                        event.convert_mst_to_est();
                    }


                    info!(
                        "Received {} timeline events from referral manager",
                        list.len()
                    );
                    return Ok(list);
                } else {
                    warn!("Getting the timeline events list failed at JSON parse");
                    self.login().await?;
                }
            } else {
                warn!("Getting the timeline events list failed at the request");
                self.login().await?;
            }
        }
        Err(anyhow::anyhow!("Max tries exceeded"))
    }

    pub async fn get_person_last_contact(
        &mut self,
        person: &persons::Person,
    ) -> anyhow::Result<Option<NaiveDateTime>> {
        let timeline = self.get_person_timeline(person).await?;
        for item in timeline {
            match item.item_type {
                persons::TimelineItemType::Contact | persons::TimelineItemType::Teaching => {
                    return Ok(Some(item.item_date))
                }
                persons::TimelineItemType::NewReferral => return Ok(None),
                _ => {
                    continue;
                }
            }
        }
        Ok(None)
    }

    pub async fn get_person_contact_time(
        &mut self,
        person: &persons::Person,
    ) -> anyhow::Result<Option<usize>> {
        let mut timeline = self.get_person_timeline(person).await?;
        timeline.reverse();

        let mut referral_sent = None;
        let mut last_contact = None;

        for item in timeline {
            match item.item_type {
                persons::TimelineItemType::NewReferral => {
                    referral_sent = Some(item.item_date);
                    last_contact = None;
                }
                persons::TimelineItemType::Contact | persons::TimelineItemType::Teaching => {
                    if last_contact.is_none() {
                        last_contact = Some(item.item_date);
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        if let Some(referral_sent) = referral_sent {
            if let Some(last_contact) = last_contact {
                let duration = last_contact.signed_duration_since(referral_sent);
                return Ok(Some(duration.num_minutes() as usize));
            }
        }
        Ok(None)
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

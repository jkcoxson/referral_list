// Jackson Coxson

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
};

use dialoguer::{theme::ColorfulTheme, Input};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub last_transfer_start: i64,
    pub zone_chats: HashMap<usize, String>,
    pub unassigned_chat: Option<String>,
    pub holly_socket: String,
    pub name: String,
    pub blacklist: Option<Vec<String>>,
}

impl Config {
    pub async fn force_load(
        church_client: &mut crate::church::ChurchClient,
    ) -> anyhow::Result<Self> {
        let config_path =
            PathBuf::from_str(&church_client.env.working_path)?.join("holly_config.json");
        if !std::fs::exists(&config_path)? {
            println!("You haven't configured this program to work with Holly yet.");
            let mut res = Self::default();
            res.update(church_client).await?;
        }
        let c = std::fs::read_to_string(&config_path)?;
        Ok(serde_json::from_str(&c)?)
    }

    pub async fn potential_load(env: &crate::env::Env) -> anyhow::Result<Option<Self>> {
        let config_path = PathBuf::from_str(&env.working_path)?.join("holly_config.json");
        if std::fs::exists(&config_path)? {
            let c = std::fs::read_to_string(&config_path)?;
            return Ok(Some(serde_json::from_str(&c)?));
        }
        Ok(None)
    }

    pub async fn update(
        &mut self,
        church_client: &mut crate::church::ChurchClient,
    ) -> anyhow::Result<()> {
        println!("Getting the newest data about zone chats...");
        let person_list = church_client.get_cached_people_list().await?;
        let mut zones = Vec::new();
        let mut zone_ids = HashSet::new();
        for p in person_list {
            if let (Some(zone_id), Some(zone_name)) = (p.zone_id, p.zone_name) {
                if zone_ids.insert(zone_id) {
                    zones.push((zone_id, zone_name));
                }
            }
        }

        for (zone_id, zone_name) in zones {
            let blank = "".to_string();
            let past_id = self.zone_chats.get(&zone_id).unwrap_or(&blank);
            let messenger_id: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Enter the Messenger zone chat ID for {}. Leave blank to skip.",
                    zone_name
                ))
                .allow_empty(true)
                .default(past_id.to_string())
                .interact_text()
                .unwrap();

            if messenger_id.is_empty() {
                self.zone_chats.remove(&zone_id);
            } else {
                self.zone_chats.insert(zone_id, messenger_id);
            }
        }

        let blank = "".to_string();
        let past_id = self.unassigned_chat.clone().unwrap_or(blank);
        let messenger_id: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(
                "Enter the Messenger zone chat ID for the referral secretary. Leave blank to skip.",
            )
            .allow_empty(true)
            .default(past_id)
            .interact_text()
            .unwrap();
        if messenger_id.is_empty() {
            self.unassigned_chat = None
        } else {
            self.unassigned_chat = Some(messenger_id)
        }

        let last_transfer_start = chrono::DateTime::from_timestamp(self.last_transfer_start, 0)
            .unwrap_or_default()
            .date_naive();
        let transfer_date = loop {
            let date_input: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the date of when the transfer started in YYYY-MM-DD format.")
                .allow_empty(true)
                .default(last_transfer_start.to_string())
                .interact_text()
                .unwrap();

            if let Ok(ts) = chrono::NaiveDate::from_str(&date_input) {
                break ts
                    .and_time(chrono::NaiveTime::default())
                    .and_utc()
                    .timestamp();
            }
            println!("Invalid date format, try again");
        };

        self.last_transfer_start = transfer_date;

        let holly_socket: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(
                "Enter the path at which to connect to Holly. If unsure, leave as default.",
            )
            .default(self.holly_socket.to_string())
            .interact_text()
            .unwrap();

        self.holly_socket = holly_socket;

        let config_path =
            PathBuf::from_str(&church_client.env.working_path)?.join("holly_config.json");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&config_path)?;

        serde_json::to_writer(file, &self)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_transfer_start: chrono::Utc::now().timestamp(),
            zone_chats: Default::default(),
            unassigned_chat: None,
            holly_socket: "127.0.0.1:8011".to_string(),
            name: "Holly".to_string(),
            blacklist: None,
        }
    }
}

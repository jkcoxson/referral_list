// Jackson Coxson & Karter Arritt

use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::PathBuf,
    str::FromStr,
    fs::OpenOptions
};


use crate::persons;
use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
//use log::error;
use serde_json;

#[derive(Clone, Debug)]
pub struct Env {
    pub church_username: String,
    pub church_password: String,
    pub timeline_send_url: String,
    pub timeline_send_crypt_key: String,
    pub working_path: String,
}

/// Checks the environment variables to make sure we are good to go.
/// Returns true on a successful startup
///
/// # Safety
/// Call this before calling async code.
/// Apparently we haven't, as society, figured out how to make
/// reading and writing env vars thread safe.
pub fn check_vars() -> Env {
    dotenvy::dotenv().ok();

    Env {
        church_username: std::env::var("CHURCH_USERNAME").unwrap_or_else(|_| {
            let password: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter your churchofjesuschrist.org username")
                .interact()
                .unwrap();

            save_var("CHURCH_USERNAME", &password);
            password
        }),
        church_password: std::env::var("CHURCH_PASSWORD").unwrap_or_else(|_| {
            let password: String = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter your churchofjesuschrist.org password")
                .with_confirmation("Repeat password", "Error: the passwords don't match.")
                .interact()
                .unwrap();

            save_var("CHURCH_PASSWORD", &password);
            password
        }),
        timeline_send_url: std::env::var("TIMELINE_SEND_URL").unwrap_or_else(|_| {
            let password: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the url to POST timeline data to")
                .default("/".to_string())
                .interact()
                .unwrap();

            save_var("TIMELINE_SEND_URL", &password);
            password
        }),
        timeline_send_crypt_key: std::env::var("TIMELINE_SEND_CRYPT_KEY").unwrap_or_else(|_| {
            let password: String = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the private encryption key for POSTing the timeline data.")
                .interact()
                .unwrap();

            save_var("TIMELINE_SEND_CRYPT_KEY", &password);
            password
        }),
        working_path: "rm_working_path".to_string(),
    }
}

fn save_var(key: &str, val: &str) {
    // Use sanitize_input to make sure we are saving a sanitized value

    // Save the variable to the environment
    std::env::set_var(key, val);

    // Ask if the user wants to save the value to the .env file
    let selections = &["Yes", "No"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Save this for future startups? This value will be saved in your .env file. If unsure, select yes.")
        .default(0)
        .items(selections)
        .interact()
        .unwrap();

    if selection == 0 {
        // Write to the .env file
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(".env")
            .unwrap();

        // Write the sanitized variable to the .env file
        file.write_all(format!("{key}={val}\n").as_bytes()).unwrap();
    }
}

impl Env {
    pub fn load_contacts(&self) -> anyhow::Result<HashMap<String, usize>> {
        // Load or create the CSV file
        let csv_path = PathBuf::from_str(&self.working_path)?.join("contact_times.csv");
        if !std::fs::exists(&csv_path)? {
            return Ok(HashMap::new());
        }

        let mut res = HashMap::new();

        let file = std::fs::File::open(&csv_path)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let mut line = line.split(',');
                    if let Some(guid) = line.next() {
                        if let Some(time) = line.next() {
                            if let Ok(time) = time.parse::<usize>() {
                                res.insert(guid.to_string(), time);
                            }
                        }
                    }
                }
                Err(e) => return Err(anyhow::anyhow!(e)),
            }
        }

        Ok(res)
    }

    pub fn save_contacts(&self, contacts: &HashMap<String, usize>) -> anyhow::Result<()> {
        // Load or create the CSV file
        let csv_path = PathBuf::from_str(&self.working_path)?.join("contact_times.csv");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&csv_path)?;

        let mut writer = std::io::BufWriter::new(file);
        for (k, v) in contacts {
            writeln!(&mut writer, "{k},{v}")?;
        }
        Ok(())
    }

    pub fn save_data(&self, contacts: &Vec<persons::ReferralPerson>) -> anyhow::Result<()> {
        // Load or create the CSV file
        let persons_path = PathBuf::from_str(&self.working_path)?.join("data.json");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&persons_path)?;

        let json_data = serde_json::to_string(&contacts)?;

        // Write the JSON string to the file
        let mut writer = std::io::BufWriter::new(file);
        writer.write_all(json_data.as_bytes())?;

        Ok(())
    }
}

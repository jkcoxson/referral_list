// Jackson Coxson

use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::PathBuf,
    str::FromStr,
};

use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use log::error;

#[derive(Clone, Debug)]
pub struct Env {
    pub church_username: String,
    pub church_password: String,
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
        working_path: std::env::var("WORKING_PATH").unwrap_or_else(|_| {
            let here = std::env::current_dir().unwrap().join("rm_working_path");
            if std::fs::create_dir_all(&here).is_err() {
                error!("Creating directory {here:?} failed!");
            }
            let here = here.to_string_lossy();
            let password: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Path to cache data fetched and processed from church servers. If unsure, just press enter for the default value.")
                .default(here.to_string())
                .interact_text()
                .unwrap();
            save_var("WORKING_PATH", &password);
            password
        }),
    }
}

fn save_var(key: &str, val: &str) {
    std::env::set_var(key, val);
    let selections = &["Yes", "No"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Save this for future startups? This value will be saved in your .env file. If unsure, select yes.")
        .default(0)
        .items(&selections[..])
        .interact()
        .unwrap();

    if selection == 0 {
        // Write to the .env file
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(".env")
            .unwrap();

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
}

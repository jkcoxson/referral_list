// Jackson Coxson

use std::io::Write;

use dialoguer::{theme::ColorfulTheme, Input, Password, Select};

#[derive(Clone)]
pub struct Env {
    pub church_username: String,
    pub church_password: String,
    pub cookie_store_path: String,
    pub bearer_token_path: String,
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
        cookie_store_path: std::env::var("COOKIE_STORE_PATH").unwrap_or_else(|_| {
            let here = std::env::current_dir().unwrap().join("cookies.json");
            let here = here.to_string_lossy();
            let password: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the path to store the auth cookies. If unsure, just press enter for the default value.")
                .default(here.to_string())
                .interact_text()
                .unwrap();
            save_var("COOKIE_STORE_PATH", &password);
            password
        }),
        bearer_token_path: std::env::var("BEARER_TOKEN_PATH").unwrap_or_else(|_| {
            let here = std::env::current_dir().unwrap().join("bearer.token");
            let here = here.to_string_lossy();
            let password: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the path to store the bearer token. This allows us to reuse sessions. If unsure, just press enter for the default value.")
                .default(here.to_string())
                .interact_text()
                .unwrap();
            save_var("BEARER_TOKEN_PATH", &password);
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

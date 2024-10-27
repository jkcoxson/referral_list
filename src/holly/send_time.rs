// Jackson Coxson

use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use chrono::{Days, NaiveDateTime, NaiveTime, Timelike};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SendTime {
    pub last: NaiveDateTime,
    pub next: NaiveDateTime,
    #[serde(skip)]
    path: PathBuf,
}

impl SendTime {
    pub async fn load(env: &crate::env::Env) -> anyhow::Result<Self> {
        let file_path = PathBuf::from_str(&env.working_path)?.join("send_time.json");
        if !std::fs::exists(&file_path)? {
            let res = Self {
                path: file_path,
                ..Default::default()
            };
            return Ok(res);
        }
        let s = std::fs::read_to_string(&file_path)?;
        let mut res: Self = serde_json::from_str(&s)?;
        res.path = file_path;
        Ok(res)
    }

    pub async fn is_go_time(&mut self) -> anyhow::Result<bool> {
        if self.last == self.next {
            self.set_next().await?;
            return Ok(false);
        }
        let now = chrono::Local::now().naive_local();
        if now > self.next {
            self.last = self.next;
            self.set_next().await?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn set_next(&mut self) -> anyhow::Result<()> {
        let now = chrono::Local::now().naive_local();

        // Define the start and end times
        // We are still doing 6:30 to 12 even on the same day because this will auto fire even if the time is after.
        let start_time = NaiveTime::from_hms_opt(6, 30, 0).unwrap(); // 6:30 AM
        let end_time = NaiveTime::from_hms_opt(12, 0, 0).unwrap(); // 12:00 PM

        // Calculate the range in minutes
        let start_minutes = start_time.num_seconds_from_midnight() / 60;
        let end_minutes = end_time.num_seconds_from_midnight() / 60;

        // Generate a random number of minutes between the start and end
        let random_minutes = rand::thread_rng().gen_range(start_minutes..=end_minutes);
        let random_time = NaiveTime::from_hms_opt(random_minutes / 60, random_minutes % 60, 0);

        let res = NaiveDateTime::new(
            if self.last.date() == now.date() {
                now.date().checked_add_days(Days::new(1)).unwrap()
            } else {
                now.date()
            },
            random_time.unwrap(),
        );
        self.next = res;
        self.save().await?;
        println!("Sending Holly's list at {}", self.next);
        Ok(())
    }

    async fn save(&self) -> anyhow::Result<()> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        serde_json::to_writer(file, self)
            .context("Unable to serialize or write send time to file")?;
        Ok(())
    }
}

// Jackson Coxson

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use log::info;
use serde::{Deserialize, Serialize};

use crate::persons::Person;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Report {
    people: HashMap<usize, HashMap<String, Vec<String>>>,
    zones: HashMap<usize, String>,
    pub unassigned: Vec<String>,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_person(&mut self, person: Person) {
        if let Some(zone_id) = person.zone_id {
            let zone = match self.people.get_mut(&zone_id) {
                Some(z) => z,
                None => {
                    self.zones
                        .insert(zone_id, person.zone_name.unwrap_or(zone_id.to_string()));
                    self.people.insert(zone_id, HashMap::new());
                    self.people.get_mut(&zone_id).unwrap()
                }
            };
            let area_name = person.area_name.unwrap_or("NO AREA".to_string());
            if let Some(area) = zone.get_mut(&area_name) {
                area.push(person.first_name);
            } else {
                zone.insert(area_name, vec![person.first_name]);
            }
        } else {
            self.unassigned.push(person.first_name)
        }
    }

    pub fn pretty_print(&self) -> String {
        let mut res = "".to_string();
        for (zone_id, areas) in &self.people {
            res = format!("{res}{}", self.pretty_print_zone(zone_id, areas));
            res = format!("{res}\n\n");
        }
        res = format!("{res}\nUnassigned Referrals");
        for p in &self.unassigned {
            res = format!("  - {p}")
        }
        res
    }

    fn pretty_print_zone(&self, zone_id: &usize, areas: &HashMap<String, Vec<String>>) -> String {
        let mut res = "".to_string();
        let zone_name = &zone_id.to_string();
        let zone_name = self.zones.get(zone_id).unwrap_or(zone_name);

        res = format!("{res}\n{zone_name}");
        for (area, people) in areas {
            res = format!("{res}\n\n - {area}");
            for p in people {
                res = format!("{res}\n  - {p}");
            }
        }
        res
    }

    pub fn get_pretty_zone(&self, zone_id: &usize) -> Option<String> {
        let areas = self.people.get(zone_id)?;
        Some(self.pretty_print_zone(zone_id, areas))
    }

    pub fn save_report(&self, env: &crate::env::Env) -> anyhow::Result<()> {
        info!("Saving report");
        let today = chrono::Local::now();
        let today_str = today.format("%Y-%m-%d").to_string();

        let reports_path = PathBuf::from_str(&env.working_path)?.join("reports");
        std::fs::create_dir_all(&reports_path)?;

        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(reports_path.join(format!("{today_str}.json")))?;
        serde_json::to_writer(file, self)?;

        info!("Saved report to {reports_path:?}");

        Ok(())
    }

    pub fn read_report(env: &crate::env::Env) -> anyhow::Result<Option<Self>> {
        let today = chrono::Local::now();
        let today_str = today.format("%Y-%m-%d").to_string();

        let reports_path = PathBuf::from_str(&env.working_path)?.join("reports");
        std::fs::create_dir_all(&reports_path)?;

        if std::fs::exists(reports_path.join(format!("{today_str}.json")))? {
            let s = std::fs::read_to_string(reports_path.join(format!("{today_str}.json")))?;
            Ok(Some(serde_json::from_str(&s)?))
        } else {
            Ok(None)
        }
    }
}

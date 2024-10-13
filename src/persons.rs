// Jackson Coxson

use chrono::naive::serde::ts_milliseconds;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Persons {
    persons: Vec<Person>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    #[serde(rename = "personGuid")]
    guid: String,

    #[serde(rename = "firstName")]
    first_name: String,

    #[serde(rename = "referralStatusId")]
    referral_status: ReferralStatus,

    #[serde(rename = "personStatusId")]
    person_status: PersonStatus,

    #[serde(rename = "missionId")]
    mission_id: usize,

    #[serde(rename = "zoneId")]
    zone_id: Option<usize>,

    #[serde(rename = "districtId")]
    district_id: Option<usize>,

    #[serde(rename = "areaName")]
    area_name: Option<String>,

    #[serde(rename = "referralAssignedDate")]
    #[serde(with = "ts_milliseconds")]
    assigned_date: NaiveDateTime,
}

impl Person {
    pub fn parse_lossy(mut object: serde_json::Value) -> Vec<Self> {
        if let serde_json::Value::Array(persons) = object["persons"].take() {
            let mut res: Vec<Self> = Vec::with_capacity(persons.len());
            for person in persons {
                if let Ok(p) = serde_json::from_value(person) {
                    res.push(p);
                }
            }
            res
        } else {
            Vec::new()
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Clone, Debug)]
#[repr(u8)]
pub enum ReferralStatus {
    NotAttempted = 10,
    NotSuccessful = 20,
    Successful = 30,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Clone, Debug)]
#[repr(u8)]
pub enum PersonStatus {
    Yellow = 1,
    Green = 2,
    BetterGreen = 3,
    ProgressingGreen = 4,
    NewMember = 6,
    NotInterested = 20,
    NotInterestedDeclared = 21,
    NotProgressing = 22,
    UnableToContact = 23,
    Prank = 25, // unsure
    NotRecentlyContacted = 26,
    TooBusy = 27,
    OutsideAreaStrength = 28,
    Member = 40,
    Moved = 201,
}

#[cfg(test)]
mod tests {
    #[test]
    fn t1() {
        let list = std::fs::read_to_string("list.json").unwrap();
        let list = super::Person::parse_lossy(serde_json::from_str(&list).unwrap());
        println!("{list:?}");
    }
}

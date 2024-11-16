// Jackson Coxson

use chrono::naive::serde::ts_milliseconds;
use chrono::{NaiveDateTime, Duration};
//use chrono::{Datelike, Utc, TimeZone, FixedOffset, DateTime};
use log::warn;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
//use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Persons {
    persons: Vec<Person>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferralPerson {
    pub id: String,
    pub name: String,
    pub contact_time: usize,
    pub events: Vec<TimelineEvent>,
    pub score: String,
    pub area: String,
}

impl ReferralPerson {
    pub fn new(id:String, name:String, contact_time:usize, events:Vec<TimelineEvent>, area:String) -> ReferralPerson{
        ReferralPerson {
            id,
            name,
            contact_time,
            events,
            score:"0/0".to_string(),
            area,
        }
    }
    pub fn set_score(&mut self, score:String ) {
        self.score = score;
    }
}

pub fn convert_referral_to_gas(referral_people: Vec<ReferralPerson>) -> Vec<GASPerson> {
    referral_people.into_iter().map(|referral_person| {
        let contact_time = referral_person.contact_time as f64 / 1440.0;  // Convert minutes to decimal days
        GASPerson {
            name: referral_person.name,
            contact_time,  // Decimal days
            score: referral_person.score,
            area: referral_person.area,
        }
    }).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GASPerson {
    pub name: String,
    pub contact_time: f64,  // Store the contact time in decimal days
    pub score: String,
    pub area: String,
}

// impl GASPerson {
//     // Constructor for GASPerson: Accepts minutes, converts to decimal days
//     pub fn new(name: String, contact_time_minutes: usize, score: String, area: String) -> GASPerson {
//         let contact_time = contact_time_minutes as f64 / 1440.0;  // Convert minutes to decimal days
//         GASPerson {
//             name,
//             contact_time,
//             score,
//             area,
//         }
//     }
// }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    #[serde(rename = "personGuid")]
    pub guid: String,

    #[serde(rename = "firstName")]
    pub first_name: String,

    #[serde(rename = "referralStatusId")]
    pub referral_status: ReferralStatus,

    #[serde(rename = "personStatusId")]
    pub person_status: PersonStatus,

    #[serde(rename = "missionId")]
    pub mission_id: usize,

    #[serde(rename = "zoneId")]
    pub zone_id: Option<usize>,

    #[serde(rename = "zoneName")]
    pub zone_name: Option<String>,

    #[serde(rename = "districtId")]
    pub district_id: Option<usize>,

    #[serde(rename = "areaName")]
    pub area_name: Option<String>,

    #[serde(rename = "referralAssignedDate")]
    #[serde(with = "ts_milliseconds")]
    pub assigned_date: NaiveDateTime,
}

impl Person {
    pub fn parse_lossy(mut object: serde_json::Value) -> Vec<Self> {
        if let serde_json::Value::Array(persons) = object["persons"].take() {
            let mut res: Vec<Self> = Vec::with_capacity(persons.len());
            for person in persons {
                if let Ok(p) = serde_json::from_value(person.clone()) {
                    res.push(p);
                } else {
                    warn!("Unable to parse person: {person:?}");
                }
            }
            res
        } else {
            Vec::new()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineEvent {
    #[serde(rename = "timelineItemType")]
    pub item_type: TimelineItemType,

    #[serde(rename = "itemDate")]
    #[serde(with = "ts_milliseconds")]
    pub item_date: NaiveDateTime,

    #[serde(rename = "eventStatus")]
    pub status: Option<bool>,
}

impl TimelineEvent {
    pub fn parse_lossy(object: serde_json::Value) -> Vec<Self> {
        if let serde_json::Value::Array(persons) = object {
            let mut res: Vec<Self> = Vec::with_capacity(persons.len());
            for person in persons {
                if let Ok(p) = serde_json::from_value(person.clone()) {
                    res.push(p);
                } else {
                    warn!("Unable to parse timeline event: {person:?}");
                }
            }
            res
        } else {
            Vec::new()
        }
    }

    // Convert MST (UTC-7) to EST (UTC-5)
    pub fn convert_mst_to_est(&mut self) {

        //println!("Initial NaiveDateTime (MST): {}", self.item_date);

        self.item_date = self.item_date - Duration::hours(5); //adjust from MST to EST

        //println!("Converted NaiveDateTime (EST): {}", self.item_date);
    }
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Clone, Debug)]
#[repr(u8)]
pub enum ReferralStatus {
    NotAttempted = 10,
    NotSuccessful = 20,
    Successful = 30,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Clone, Debug, Eq, PartialOrd, Ord)]
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

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum TimelineItemType {
    #[serde(rename = "STOPPED_TEACHING")]
    StoppedTeaching,
    #[serde(rename = "CONTACT")]
    Contact,
    #[serde(rename = "TEACHING")]
    Teaching,
    #[serde(rename = "NEW_REFERRAL")]
    NewReferral,
    #[serde(rename = "PERSON_CREATE")]
    PersonCreate,
    #[serde(rename = "PERSON_OFFER_ITEM")]
    PersonOfferItem,
    #[serde(rename = "SACRAMENT")]
    Sacrament,
    #[serde(rename = "TEACHING_RESET")]
    TeachingReset,
    #[serde(rename = "PERSON_PLN_NOTE")]
    Note,
    #[serde(rename = "PERSON_TASK")]
    Task,
    #[serde(rename = "EMAIL_SUBSCRIPTION")]
    EmailSubscription,
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

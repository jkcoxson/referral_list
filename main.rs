// Jackson Coxson & Karter Arritt

use std::collections::HashMap;
//use std::{cell::Ref, fmt::format}

use chrono::{Duration, Utc};
use church::ChurchClient;
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;
use log::info;

mod bearer;
mod church;
mod env;
mod persons;
mod report;
mod send;

const CLI_OPTIONS: [&str; 4] = ["report", "generate", "send timeline", "exit"];
const CLI_DESCRIPTONS: [&str; 4] = [
    "Reads today's report of uncontacted referrals or fetches a new one",
    "Generates a new list of uncontacted referrals, regardless of the cache.",
    "Gets all the timeline events and send a person by person score to a Web Endpoint",
    "Exits the program",
];

#[tokio::main]
async fn main() {
    println!("Starting referral list program... Checking environment...");
    let save_env = env::check_vars();
    env_logger::init();
    let mut church_client = church::ChurchClient::new(save_env).await.unwrap();



    let mut args = std::env::args();
    if args.len() > 1 {
        if let Err(e) = parse_argument(&args.nth(1).unwrap(), &mut church_client).await {
            println!("Ran into an error while processing: {e:?}");
        }
        return;
    }

    let select_options = CLI_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, val)| format!("{} - {}", val, CLI_DESCRIPTONS[i]))
        .collect::<Vec<String>>();

    loop {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an option")
            .default(0)
            .items(&select_options)
            .interact()
            .unwrap();

        match parse_argument(CLI_OPTIONS[selection], &mut church_client).await {
            Ok(true) => continue,
            Ok(false) => return,
            Err(e) => {
                println!("Ran into an error while processing: {e:?}");
            }
        }
    }
}

async fn parse_argument(arg: &str, church_client: &mut ChurchClient) -> anyhow::Result<bool> {
    match arg {
        "report" => {
            if let Some(report) = report::Report::read_report(&church_client.env)? {
                println!("{}", report.pretty_print());
            } else {
                let report = generate_report(church_client).await?;
                println!("{}", report.pretty_print());
            }
            Ok(true)
        }
        "generate" => {
            generate_report(church_client).await?;
            Ok(true)
        }
        "send timeline" => {
            let da_peeps = store_timeline(church_client).await?;

            let out = persons::convert_referral_to_gas(da_peeps);

            let encrypted_data = match send::encrypt_struct_with_otp(out,church_client.env.timeline_send_crypt_key.clone()) {
                Ok(data) => data,
                Err(e) => {
                    println!("Error encrypting data: {}", e);
                    return Ok(false); // or return Err(e) if needed
                }
            };

            match send::send_to_google_apps_script(encrypted_data, church_client.env.timeline_send_url.clone()).await {
                Ok(decrypted_json) => {
                    println!("Success! Decrypted response: {}", decrypted_json);
                }
                Err(e) => {
                    eprintln!("Error sending request: {}", e);
                }
            }

            Ok(true)
        }
        "exit" => Ok(false),
        "help" | "-h" => {
            println!("Referral List - a tool to get and parse a list of referrals from referral manager.");
            for i in 0..CLI_OPTIONS.len() {
                println!("  {} - {}", CLI_OPTIONS[i], CLI_DESCRIPTONS[i]);
            }
            Ok(false)
        }
        _ => Err(anyhow::anyhow!(
            "Unknown usage '{arg}' - run without arguments to see options"
        )),
    }
}

pub async fn generate_report(church_client: &mut ChurchClient) -> anyhow::Result<report::Report> {
    let persons_list = church_client.get_cached_people_list().await?;
    let now = Utc::now().naive_utc();
    let persons_list: Vec<persons::Person> = persons_list
        .into_iter()
        .filter(|x| {
            x.referral_status != persons::ReferralStatus::Successful
                && x.person_status < persons::PersonStatus::NewMember
                && now.signed_duration_since(x.assigned_date) > Duration::hours(48)
        })
        .collect();
    info!("{} uncontacted referrals", persons_list.len());

    let mut report = report::Report::new();
    let bar = ProgressBar::new(persons_list.len() as u64);
    for person in persons_list {
        bar.inc(1);
        if match church_client.get_person_last_contact(&person).await? {
            Some(t) => now.signed_duration_since(t) > Duration::hours(48),
            None => true,
        } {
            report.add_person(person);
        }
    }

    report.save_report(&church_client.env)?;
    Ok(report)
}


pub async fn store_timeline(
    church_client: &mut ChurchClient,
) -> anyhow::Result<Vec<persons::ReferralPerson>> {
    let persons_list = church_client.get_cached_people_list().await?.to_vec();
    let now = Utc::now().naive_utc();
    let persons_list: Vec<persons::Person> = persons_list
        .into_iter()
        .filter(|x| {
                x.person_status < persons::PersonStatus::NewMember
                && now.signed_duration_since(x.assigned_date) < Duration::days(8)
        })
        .collect();

    let mut da_peeps = Vec::new();
    let bar = ProgressBar::new(persons_list.len() as u64);
    for person in persons_list {
        
        bar.inc(1);
        let t:Vec<persons::TimelineEvent> = if let Ok(t) = church_client.get_person_timeline(&person).await {          
            t.iter()
            .filter(|event| matches!(event.item_type, persons::TimelineItemType::Contact | persons::TimelineItemType::Teaching | persons::TimelineItemType::NewReferral) && if event.item_type != persons::TimelineItemType::NewReferral && event.status.is_none() {false} else {true})
            .cloned()
            .collect()
        } else {
            continue;
        };
        let cont_time: usize;
        if let Some(t ) = church_client.get_person_contact_time(&person).await? {cont_time = t;t}else{continue;};
        let mut this_guy = persons::ReferralPerson::new(
            person.guid,
            person.first_name,
            cont_time,
            t.clone(),
            match person.area_name {
                Some(s) => s.clone(),  // return the String if present
                None => String::from("default_area"),  // return a default value if None
            },
        );

        // Print the timeline (t) when this_guy is created
        // println!("Timeline for {} ({}):", this_guy.name, this_guy.area);
        // for event in &t {
        //     let formatted_date = event.item_date.format("%Y-%m-%d %H:%M:%S").to_string();  // Format date-time as YYYY-MM-DD HH:MM:SS

        //     let event_status = match event.status {
        //         Some(true) => "Completed",
        //         Some(false) => "Not completed",
        //         None => "No status",
        //     };

        //     println!(
        //         "[{}] Event Type: {:?}, Date: {}, Status: {}",
        //         this_guy.name,
        //         event.item_type,
        //         formatted_date,
        //         event_status
        //     );
        // }

        
        let yesterday = chrono::Local::now().naive_utc().date() - Duration::days(1);
        
        let last_new_referral = t.iter().find(|event| {
            event.item_type == persons::TimelineItemType::NewReferral
        });
        
        let mut current_date: chrono::NaiveDate = last_new_referral.unwrap().item_date.date();
        let mut contact_days = 0;
        let mut total_days = 0;
        
        while current_date <= yesterday && total_days < 7 {
            total_days += 1;
            
            // Format the current date to a readable format
            //let formatted_date = current_date.format("%Y-%m-%d").to_string();  // Format as YYYY-MM-DD
            
            // Log the current action and day
            //println!("{}: checking day {} (date: {})", this_guy.name, total_days, formatted_date);
            
            let c = check_day(current_date, t.clone());
            if c == -1 {
                contact_days += 1;
                // Log when the day check is -1
                //println!("{}: day {} (date: {}) resulted in -1 (contact day)", this_guy.name, total_days, formatted_date);
                break;
            } else {
                contact_days += c;
                // Log the result of check_day
                //println!("{}: day {} (date: {}) resulted in {}", this_guy.name, total_days, formatted_date, c);
            }
            
            current_date = current_date + Duration::days(1);
        }
        
        //println!("{}: finished checking days. Contact days: {}, Total days checked: {}", this_guy.name, contact_days, total_days);
        

        this_guy.set_score(format!("{contact_days}/{total_days}"));

        da_peeps.push(this_guy);
    }
    bar.finish();



    church_client.env.save_data(&da_peeps)?;

    Ok(da_peeps)
}

fn check_day(day: chrono::naive::NaiveDate, person: Vec<persons::TimelineEvent>) -> i32 {
    // Find all events that match the day and the 'Contact' type
    let events_on_day: Vec<&persons::TimelineEvent> = person.iter()
        .filter(|event| event.item_date.date() == day && (event.item_type == persons::TimelineItemType::Contact || event.item_type == persons::TimelineItemType::Teaching))
        .collect();
    
    // If there are no events for the day, return 0
    if events_on_day.is_empty() {
        return 0;
    }

    // Check each event. If any event has a status of Some(true), return -1
    for event in events_on_day {
        if event.status.unwrap_or(false) {
            return -1;
        }
    }

    // If no events with status Some(true) were found, return 1
    return 1;
}


pub async fn get_average(
    church_client: &mut ChurchClient,
) -> anyhow::Result<HashMap<String, usize>> {
    let mut contacts = church_client.env.load_contacts()?;

    let persons_list = church_client.get_cached_people_list().await?.to_vec();
    let now = Utc::now().naive_utc();
    let persons_list: Vec<persons::Person> = persons_list
        .into_iter()
        .filter(|x| {
            x.referral_status != persons::ReferralStatus::NotAttempted
                && x.person_status < persons::PersonStatus::NewMember
                && now.signed_duration_since(x.assigned_date) < Duration::days(42)
        })
        .collect();

    let mut zones = HashMap::new();
    let bar = ProgressBar::new(persons_list.len() as u64);
    for person in persons_list {
        if let Some(zone_name) = &person.zone_name {
            bar.inc(1);
            let t = if let Some(t) = contacts.get(&person.guid) {
                t.to_owned()
            } else if let Some(t) = church_client.get_person_contact_time(&person).await? {
                contacts.insert(person.guid, t);
                t
            } else {
                continue;
            };
            let zone = match zones.get_mut(zone_name) {
                Some(z) => z,
                None => {
                    zones.insert(zone_name.clone(), Vec::new());
                    zones.get_mut(zone_name).unwrap()
                }
            };
            zone.push(t);
        }
    }
    bar.finish();

    church_client.env.save_contacts(&contacts)?;

    let mut res = HashMap::new();
    for (k, v) in zones {
        let sum: usize = v.iter().sum();
        let avg = sum / v.len();
        res.insert(k, avg);
    }
    Ok(res)
}
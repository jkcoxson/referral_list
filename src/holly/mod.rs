// Jackson Coxson

use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::UnboundedSender,
    time::sleep_until,
};

use crate::church::ChurchClient;

pub mod config;
mod send_time;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Message {
    sender: String,
    content: String,
    chat_id: String,
}

impl Message {
    fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

pub async fn main(church_client: &mut ChurchClient) -> anyhow::Result<()> {
    info!("Connecting to Holly...");
    let holly_config = church_client
        .holly_config
        .clone()
        .unwrap_or(config::Config::force_load(church_client).await?);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::task::spawn_blocking(move || user_input_loop(tx));

    loop {
        let mut stream = tokio::net::TcpStream::connect(&holly_config.holly_socket).await?;
        let mut next_time_check = tokio::time::Instant::now() + tokio::time::Duration::from_secs(1);
        if loop {
            let mut buf = [0u8; 1024 * 8];
            tokio::select! {
                written = stream.read(&mut buf) => {
                    let written = match written {
                        Ok(w) => w,
                        Err(e) => match e.kind() {
                            std::io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            _ => {
                                error!("Error receiving data from Holly! {e:?}");
                                break false;
                            }
                        },
                    };
                    if written == 0 {
                        error!("Holly stopped sending data!");
                        break false;
                    }

                    if let Ok(payload) = String::from_utf8(buf[0..written].to_vec()) {
                        if let Ok(payload) = serde_json::from_str::<Message>(&payload) {
                            info!("Recieved message from Holly: {payload:?}");
                        }
                    } else {
                        error!("Recieved a non-utf8 vector from Holly");
                        break false;
                    }
                }
                _ = sleep_until(next_time_check) => {
                    info!("Checking if it's time to send Holly's list");
                    next_time_check = tokio::time::Instant::now() + tokio::time::Duration::from_secs(20);
                    let mut st = send_time::SendTime::load(&church_client.env).await?;
                    if st.is_go_time().await? {
                        info!("Sending Holly's list!");
                        let report = if let Some(report) = crate::report::Report::read_report(&church_client.env)? {
                            report
                        } else {
                            crate::generate_report(church_client).await?
                        };

                        let contacts = crate::get_average(church_client).await?;
                        let mut contacts = contacts.into_iter().collect::<Vec<(String, usize)>>();
                        contacts.sort_unstable_by(|a, b| a.1.cmp(&b.1));

                        let mut avg_report = "".to_string();
                        for (k, v) in contacts {
                            if let Some(bl) = &holly_config.blacklist {
                                if bl.contains(&k) {
                                    continue;
                                }
                            }
                            avg_report = format!("{avg_report}\n{k}: {v}");
                        }
                        for (zone_id, chat_id) in &holly_config.zone_chats {
                            let msg = if let Some(p) = report.get_pretty_zone(zone_id) {
                                format!("Average Contact Time:\n{avg_report}\n\nUncontacted Referrals:\n\n{p}")
                            } else {
                                info!("No uncontacted referrals in {zone_id}");
                                format!("Average Contact Time:{avg_report}\n\nNo uncontacted referrals! Great work!")
                            };
                            info!("Sending {msg} to {chat_id}");
                            stream.write_all(&Message { content: msg, chat_id: chat_id.to_string(), ..Default::default() }.to_bytes()).await?;
                        }
                        if let Some(chat_id) = &holly_config.unassigned_chat {
                            let msg = report.unassigned.join("\n");
                            info!("Sending {msg} to {chat_id}");
                            stream.write_all(&Message { content: msg, chat_id: chat_id.to_string(), ..Default::default() }.to_bytes()).await?;
                        }
                    }
                }
                _ = rx.recv() => {
                    info!("Disconnecting from Holly...");
                    break true;
                }
            }
        } {
            break;
        }
    }
    Ok(())
}

fn user_input_loop(sender: UnboundedSender<()>) {
    println!("Press 'q' and then enter to disconnect from Holly gracefully.");
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf).is_ok();
    let _ = sender.send(()).is_ok();
}

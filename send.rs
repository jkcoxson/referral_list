//Karter Arritt

use reqwest::Client;
use serde::{Serialize, Deserialize};
use base64::{engine::general_purpose, Engine};
//use serde_json::json;
//use std::{error::Error, iter::repeat_with};


#[derive(Serialize, Deserialize)]
pub struct PostPayload {
    body: String,
}

pub fn pad_until_divisible_by_3(s: String) -> String {
    let mut padded = s.to_string();
    while padded.len() % 3 != 0 {
        padded.push(' ');
    }
    padded
}

pub fn encrypt_struct_with_otp<T: Serialize>(data: T, encryption_key: String) -> Result<String, Box<dyn std::error::Error>> {
    // Serialize struct to JSON string
    let json_data = serde_json::to_string(&data)?;
    
    let padded_json = pad_until_divisible_by_3(json_data);

    // Ensure OTP is long enough by repeating it until it matches the length of the JSON data
    let otp_length = padded_json.len();
    let extended_otp: Vec<u8> = encryption_key.bytes()
        .cycle()  // This will create an infinite iterator over the OTP bytes
        .take(otp_length)  // Take only as many bytes as needed to match the length of padded_json
        .collect();
        
    


    // Encrypt the JSON data using XOR with the extended OTP
    let encrypted_bytes: Vec<u8> = padded_json
        .bytes()
        .zip(extended_otp)
        .map(|(data_byte, otp_byte)| data_byte ^ otp_byte)
        .collect();

    // Convert encrypted bytes into a base64 string (easy to transfer and store)
    let encoded = general_purpose::STANDARD.encode(&encrypted_bytes);
    Ok(encoded)
}



pub async fn send_to_google_apps_script(body: String, endpoint_url: String) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

    // Construct the JSON payload
    let payload = PostPayload {
        body: body.clone(),  // Use body directly or clone it for the payload
    };

    // Send POST request
    let res = client
        .post(endpoint_url)
        .json(&payload)
        .send().await?;

    // Check for successful response
    if res.status().is_success() {
        // Parse the response JSON (assuming it's a decrypted object)
        let response_text = res.text().await?;
        Ok(response_text)
    } else {
        Err(format!("Request failed with status: {}", res.status()).into())
    }
}
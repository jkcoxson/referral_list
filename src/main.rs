// Jackson Coxson

mod bearer;
mod church;
mod env;
mod persons;

#[tokio::main]
async fn main() {
    println!("Starting referral list program... Checking environment...");
    let env = env::check_vars();
    let mut church_client = church::ChurchClient::new(env).await.unwrap();
    church_client.login().await.unwrap();
}

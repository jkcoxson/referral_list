// Jackson Coxson

use church::ChurchClient;
use dialoguer::{theme::ColorfulTheme, Select};

mod bearer;
mod church;
mod env;
mod persons;

const CLI_OPTIONS: [&str; 2] = ["generate", "exit"];
const CLI_DESCRIPTONS: [&str; 2] = [
    "Generates a list of uncontacted referrals",
    "Exits the program",
];

#[tokio::main]
async fn main() {
    println!("Starting referral list program... Checking environment...");
    let env = env::check_vars();
    env_logger::init();
    let mut church_client = church::ChurchClient::new(env).await.unwrap();

    let mut args = std::env::args();
    if args.len() > 1 {
        if let Err(e) = parse_argument(&args.nth(1).unwrap(), &church_client).await {
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

        match parse_argument(CLI_OPTIONS[selection], &church_client).await {
            Ok(true) => continue,
            Ok(false) => return,
            Err(e) => {
                println!("Ran into an error while processing: {e:?}");
            }
        }
    }
}

async fn parse_argument(arg: &str, mut church_client: &ChurchClient) -> anyhow::Result<bool> {
    match arg {
        "generate" => Ok(true),
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

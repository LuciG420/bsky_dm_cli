use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use clap::{Parser, Subcommand};
//use clap::command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Add { name: Option<String> },
    /// Login with an app token
    Login {
        /// The app token
        token: String,
    },
    /// Send a direct message
    Send {
        /// The recipient's username
        recipient: String,
        /// The message text
        message: String,
    },
    /// Get the latest direct messages
    Get,
}

// Define the API response structure
#[derive(Deserialize, Serialize)]
struct Message {
    id: String,
    text: String,
    sender: String,
    recipient: String,
}

// Define the API client
struct BskyClient {
    client: Client,
    api_url: String,
    token: Option<String>,
}

impl BskyClient {
    fn new(api_url: String) -> Self {
        Self {
            client: Client::new(),
            api_url,
            token: None,
        }
    }

    async fn login(&mut self, token: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/api/v1/auth/app", self.api_url);
        let json = json!({
            "token": token,
        });
        let response = self.client.post(url).json(&json).send().await?;
        if response.status().is_success() {
            self.token = Some(token.to_string());
            Ok(())
        } else {
            Err(response.error_for_status().unwrap_err())
        }
    }

    async fn send_message(&self, recipient: &str, message: &str) -> Result<Message, reqwest::Error> {
        if self.token.is_none() {
            return Err(reqwest::Error::from(()));
        }
        let url = format!("{}/api/v1/dm/send", self.api_url);
        let json = json!({
            "recipient": recipient,
            "text": message,
        });
        let response = self.client.post(url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap()))
            .json(&json)
            .send()
            .await?;
        let message: Message = response.json().await?;
        Ok(message)
    }

    async fn get_messages(&self) -> Result<Vec<Message>, reqwest::Error> {
        if self.token.is_none() {
            return Err(reqwest::Error::from(()));
        }
        let url = format!("{}/api/v1/dm/inbox", self.api_url);
        let response = self.client.get(url)
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap()))
            .send()
            .await?;
        let messages: Vec<Message> = response.json().await?;
        Ok(messages)
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let cli = Cli::parse();
    let api_url = "https://bsky.app/api/v1".to_string();
    let mut client = BskyClient::new(api_url);

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    match cli.debug {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    match cli.command {
        Some(Commands::Login { token }) => {
            client.login(&token).await?;
            println!("Logged in successfully! {}", token);
        }
        Some(Commands::Send { recipient, message }) => {
            if client.token.is_none() {
                println!("Please login first! r: {}", recipient);
                return Ok(());
            }
            let message = client.send_message(&recipient, &message).await?;
            println!("Sent message: r:{} {}({})", recipient, message.text, message.id);
        }
        Some(Commands::Get) => {
            if client.token.is_none() {
                println!("Please login first!");
                return Ok(());
            }
            let messages = client.get_messages().await?;
            for message in messages {
                println!("Message from {}: {}", message.sender, message.text);
            }
        }
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}

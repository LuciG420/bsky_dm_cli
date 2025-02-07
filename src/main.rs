use anyhow::{Context, Ok, Result};
use atrium_api::{
    AtpClient,
    types::{
        app::bsky::feed::Post,
        com::atproto::server::CreateSession,
    }
};
use ably::Rest as AblyRest;
use dotenv::dotenv;
use serde_json::json;
use tokio::{
    task,
    sync::mpsc,
    time::{sleep, Duration}
};
use std::{env, fmt::Error};
use tracing::{info, error, Level};
use tracing_subscriber;

struct BskyXrpcDaemon {
    atp_client: AtpClient,
    ably_client: AblyRest,
    channel_name: String,
}

impl BskyXrpcDaemon {
    async fn new() -> Result<Self> {
        dotenv().ok();

        let username = env::var("BSKY_USERNAME")?;
        let password = env::var("BSKY_PASSWORD")?;
        let ably_api_key = env::var("ABLY_API_KEY")?;

        let atp_client = AtpClient::create_session(CreateSession {
            identifier: username,
            password: password.clone(),
        }).await?;

        let ably_client = AblyRest::new(&ably_api_key)?;

        Ok(Self {
            atp_client,
            ably_client,
            channel_name: "bsky-events".to_string(),
        })
    }

    async fn stream_posts(&self, tx: mpsc::Sender<Post>) -> Result<()> {
        loop {
            match self.atp_client.stream_posts().await {
                Ok(post) => {
                    tx.send(post).await?;
                },
                Err(e) => {
                    error!("Post streaming error: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn stream_notifications(&self, tx: mpsc::Sender<Post>) -> Result<()> {
        loop {
            match self.atp_client.stream_notifications().await {
                Ok(notification) => {
                    // Convert notification to post-like structure
                    let post = notification.convert_to_post();
                    tx.send(post).await?;
                },
                Err(e) => {
                    error!("Notification streaming error: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn publish_events(&self, mut rx: mpsc::Receiver<Post>) -> Result<()> {
        let channel = self.ably_client.channel(&self.channel_name);

        while let Some(post) = rx.recv().await {
            let event_data = json!({
                "type": "post",
                "did": post.author.did,
                "text": post.record.text,
                "timestamp": post.record.created_at
            });

            channel.publish("post", event_data).await?;
            info!("Published post event from {}", post.author.did);
        }

        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel(100);

        let posts_stream = task::spawn(self.stream_posts(tx.clone()));
        let notifications_stream = task::spawn(self.stream_notifications(tx.clone()));
        let event_publisher = task::spawn(self.publish_events(rx));

            tokio::try_join!(
                posts_stream,
                notifications_stream,
                event_publisher
            )?;
        
                Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let daemon = BskyXrpcDaemon::new();
    daemon.run().await?;
    Ok(())
}

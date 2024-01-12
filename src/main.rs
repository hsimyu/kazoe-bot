use serde::{Deserialize, Serialize};
use tokio;

#[derive(Serialize)]
struct Message {
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bot_token = "TOKEN";
    let channel_id = "ID";

    let message_content = "Hello!";
    let api_url = format!(
        "https://discord.com/api/v10/channels/{}/messages",
        channel_id
    );

    let message = Message {
        content: message_content.to_string(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&api_url)
        .header("Authorization", format!("Bot {}", bot_token))
        .header("Content-Type", "application/json")
        .json(&message)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Message sent successfully!");
    } else {
        println!("Failed to send message. Status code: {}", response.status());
    }

    Ok(())
}

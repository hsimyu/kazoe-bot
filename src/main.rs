use dotenv::dotenv;
use serenity::{
    all::GatewayIntents,
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;
use tokio;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            println!("Received message: {}", msg.content);

            if msg.content.contains("hello bot") {
                if let Err(why) = msg.reply(&ctx.http, "Hello!").await {
                    println!("Error replying: {:?}", why);
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let bot_token = env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN must be set in .env");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    // let channel_id = env::var("TARGET_CHANNEL_ID").expect("TARGET_CHANNEL_ID must be set in .env");

    let mut client = Client::builder(bot_token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

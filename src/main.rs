mod handler;
mod record;

use dotenv::dotenv;
use handler::Handler;
use record::create_table;
use rusqlite::Connection;
use serenity::{all::GatewayIntents, prelude::*};
use std::{env, sync::Arc, sync::Mutex};
use tokio;

#[tokio::main]
async fn main() {
    let db_file_path = "umakazoe.db";
    let conn = Connection::open(db_file_path).unwrap();

    println!("-- Initialize Table --");
    create_table(&conn);

    println!("-- Initialize Bot Client --");
    dotenv().ok();

    let conn = Arc::new(Mutex::new(conn));

    let bot_token = env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN must be set in .env");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(bot_token, intents)
        .event_handler(Handler {
            db_connection: Arc::clone(&conn),
        })
        .await
        .expect("Error creating client");

    println!("-- Start Bot --");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

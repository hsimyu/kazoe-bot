use dotenv::dotenv;
use record::create_table;
use regex::Regex;
use rusqlite::{params, Connection, Result};
use serenity::{
    all::{GatewayIntents, User},
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::{env, sync::Arc, sync::Mutex};
use tokio;

use crate::record::*;

mod record;

struct Handler {
    db_connection: Arc<Mutex<Connection>>,
}

async fn reply_to(ctx: &Context, msg: &Message, str: &str) {
    println!("Send reply: {}", str);
    if let Err(why) = msg.reply(&ctx.http, str).await {
        println!("Error replying: {:?}", why);
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            println!("Message: {}", msg.content);

            if msg.content.contains("かぞえて") {
                // パターンをキャプチャ
                let re = Regex::new(r"かぞえて\s+(?<pattern>.*)$").unwrap();

                let pattern: String = match re.captures(&msg.content) {
                    Some(caps) => caps["pattern"].to_string(),
                    None => {
                        reply_to(
                            &ctx,
                            &msg,
                            "\"かぞえて [pattern]\" のようにお願いしてくださいヒン",
                        )
                        .await;
                        return;
                    }
                };

                println!("Captured pattern: {}", pattern);

                let record = PatternRecord {
                    id: 0,
                    channel_id: msg.channel_id.to_string(),
                    pattern: pattern,
                };

                register_pattern(&self.db_connection.lock().unwrap(), &record);
                reply_to(&ctx, &msg, "ヒヒーン！").await;
            } else {
                // 登録依頼ではないのでパターンを検索する
                let channel_id = msg.channel_id.to_string();

                let result = find_pattern(
                    &self.db_connection.lock().unwrap(),
                    &channel_id,
                    &msg.content,
                );

                match result {
                    Some(pattern_record) => {
                        println!("Found: pattern = {:?}", pattern_record);

                        // 数字を抽出できたらそれを量として加算する
                        let re = Regex::new(
                            format!("(?<count>\\d+)\\s*{}", pattern_record.pattern.as_str())
                                .as_str(),
                        )
                        .unwrap();

                        let amount: i32 = match re.captures(&msg.content) {
                            Some(caps) => caps["count"].to_string().parse().unwrap(),
                            None => 1,
                        };

                        println!("captured amount = {}", amount);

                        // 発言がマッチしていたのでカウントする
                        let count = find_count(
                            &self.db_connection.lock().unwrap(),
                            pattern_record.id,
                            &msg.author.id.to_string(),
                        );

                        match count {
                            Some(mut count_record) => {
                                println!("Found: count = {:?}", count_record);
                                count_record.count += amount;
                                update_count(&self.db_connection.lock().unwrap(), &count_record);
                                reply_to(&ctx, &msg, format!("{}", count_record.count).as_str())
                                    .await;
                            }
                            None => {
                                register_new_count(
                                    &self.db_connection.lock().unwrap(),
                                    &CountRecord {
                                        id: 0,
                                        pattern_id: pattern_record.id,
                                        user_id: msg.author.id.to_string(),
                                        count: amount,
                                    },
                                );
                                reply_to(&ctx, &msg, format!("{}", amount).as_str()).await;
                            }
                        }
                    }
                    None => return, // 何もしない
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
    let conn = Connection::open_in_memory().expect("Failed to open memory");
    create_table(&conn);

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

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

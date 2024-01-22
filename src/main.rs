use dotenv::dotenv;
use regex::Regex;
use rusqlite::{params, Connection, Result};
use serenity::{
    all::{GatewayIntents, User},
    async_trait,
    futures::future::Then,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::{env, sync::Arc, sync::Mutex};
use tokio;

#[derive(Debug)]
struct PatternRecord {
    id: i32,
    channel_id: String,
    pattern: String,
}

#[derive(Debug)]
struct CountRecord {
    id: i32,
    pattern_id: i32,
    user_id: String,
    count: i32,
}

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

                        // 発言がマッチしていたのでカウントする
                        // TODO: パターン設定がある場合は、数字を抽出してインクリメント以外の数え方を実装したい
                        let count = find_count(
                            &self.db_connection.lock().unwrap(),
                            pattern_record.id,
                            &msg.author.id.to_string(),
                        );

                        match count {
                            Some(mut count_record) => {
                                println!("Found: count = {:?}", count_record);
                                count_record.count += 1;
                                update_count(&self.db_connection.lock().unwrap(), &count_record);
                                reply_to(
                                    &ctx,
                                    &msg,
                                    format!("count: {}", count_record.count).as_str(),
                                )
                                .await;
                            }
                            None => {
                                register_new_count(
                                    &self.db_connection.lock().unwrap(),
                                    &CountRecord {
                                        id: 0,
                                        pattern_id: pattern_record.id,
                                        user_id: msg.author.id.to_string(),
                                        count: 1,
                                    },
                                );
                                reply_to(&ctx, &msg, format!("start: {}", 1).as_str()).await;
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

fn create_table(conn: &Connection) {
    conn.execute(
        "CREATE TABLE pattern_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel_id TEXT NOT NULL,
            pattern TEXT NOT NULL
        )",
        (),
    )
    .expect("Failed to create pattern_record");

    conn.execute(
        "CREATE TABLE count_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern_id INTEGER NOT NULL,
            user_id TEXT NOT NULL,
            count INTEGER NOT NULL
        )",
        (),
    )
    .expect("Failed to create count_record");
}

fn register_pattern(conn: &Connection, record: &PatternRecord) {
    conn.execute(
        "INSERT INTO pattern_record (channel_id, pattern) VALUES (?1, ?2)",
        (&record.channel_id, &record.pattern),
    )
    .expect("Failed to insert message");
}

fn register_new_count(conn: &Connection, record: &CountRecord) {
    conn.execute(
        "INSERT INTO count_record (pattern_id, user_id, count) VALUES (?1, ?2, ?3)",
        (&record.pattern_id, &record.user_id, &record.count),
    )
    .expect("Failed to insert count");
}

fn update_count(conn: &Connection, record: &CountRecord) {
    conn.execute(
        "UPDATE count_record SET count = ?1 WHERE id = ?2",
        params![&record.count, &record.id],
    )
    .expect("Failed to update count");
}

fn find_pattern(conn: &Connection, channel_id: &String, message: &String) -> Option<PatternRecord> {
    let mut stmt = conn
        .prepare("SELECT * FROM pattern_record WHERE channel_id = ?1")
        .unwrap();

    let iter = stmt
        .query_map([&channel_id], |row| {
            Ok(PatternRecord {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                pattern: row.get(2)?,
            })
        })
        .unwrap();

    for pattern_record in iter {
        let pattern_record = pattern_record.unwrap();
        // 発言がマッチしているかを確認する
        if !pattern_record.pattern.is_empty() {
            if message.contains(pattern_record.pattern.as_str()) {
                return Some(pattern_record);
            }
        }
    }

    // マッチするパターンがなかった
    None
}

fn find_count(conn: &Connection, pattern_id: i32, user_id: &String) -> Option<CountRecord> {
    println!("search count by {} {}", pattern_id, user_id);
    let mut stmt = conn
        .prepare("SELECT * FROM count_record WHERE pattern_id = ?1 and user_id = ?2")
        .unwrap();

    let mut iter = stmt
        .query_map(params![&pattern_id, &user_id], |row| {
            Ok(CountRecord {
                id: row.get(0)?,
                pattern_id: row.get(1)?,
                user_id: row.get(2)?,
                count: row.get(3)?,
            })
        })
        .unwrap();

    match iter.next() {
        Some(count) => return Some(count.unwrap()),
        None => return None,
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

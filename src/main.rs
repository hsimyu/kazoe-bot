use dotenv::dotenv;
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

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            // println!("Context: {:?}", ctx);
            println!("Message: {}", msg.content);
            // println!("{:?}", msg);

            if msg.content.contains("かぞえて") {
                if let Err(why) = msg.reply(&ctx.http, "承知しました！").await {
                    println!("Error replying: {:?}", why);
                }

                // パターンをキャプチャ
                let re = Regex::new(r"かぞえて\s+(?<pattern>.*)$").unwrap();
                let Some(caps) = re.captures(&msg.content) else {
                    println!("no match!");
                    return;
                };

                println!("Captured pattern: {}", &caps["pattern"]);

                let record = PatternRecord {
                    id: 0,
                    channel_id: msg.channel_id.to_string(),
                    pattern: caps["pattern"].to_string(),
                };

                register_pattern(&self.db_connection.lock().unwrap(), &record);
                dump_pattern_record(&self.db_connection.lock().unwrap());
            } else {
                // 登録依頼ではないのでパターンを検索する
                let channel_id = msg.channel_id.to_string();
                let pattern_id = find_pattern(&self.db_connection.lock().unwrap(), &channel_id);

                match pattern_id {
                    Some(pattern_id) => {
                        println!("Found: pattern_id = {}", pattern_id);
                        let count = find_count(
                            &self.db_connection.lock().unwrap(),
                            pattern_id,
                            &msg.author.id.to_string(),
                        );

                        match count {
                            Some(count) => {
                                // TODO: カウントをインクリメント
                                println!("Found: count = {}", count);
                            }
                            None => {
                                println!("count not found, insert new count_record");
                                register_new_count(
                                    &self.db_connection.lock().unwrap(),
                                    &CountRecord {
                                        id: 0,
                                        pattern_id: pattern_id,
                                        user_id: msg.author.id.to_string(),
                                        count: 0,
                                    },
                                )
                            }
                        }
                    }
                    None => println!("Registered pattern not found"),
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
            id INTEGER PRIMARY KEY,
            channel_id TEXT NOT NULL,
            pattern TEXT NOT NULL
        )",
        (),
    )
    .expect("Failed to create pattern_record");

    conn.execute(
        "CREATE TABLE count_record (
            id INTEGER PRIMARY KEY,
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
        "INSERT INTO pattern_record (id, channel_id, pattern) VALUES (?1, ?2, ?3)",
        (&record.id, &record.channel_id, &record.pattern),
    )
    .expect("Failed to insert message");
}

fn register_new_count(conn: &Connection, record: &CountRecord) {
    conn.execute(
        "INSERT INTO count_record (id, pattern_id, user_id, count) VALUES (?1, ?2, ?3, ?4)",
        (&record.id, &record.pattern_id, &record.user_id, 0),
    )
    .expect("Failed to insert count");
}

fn dump_pattern_record(conn: &Connection) {
    let mut stmt = conn
        .prepare("SELECT id, channel_id, pattern FROM pattern_record")
        .unwrap();

    let iter = stmt
        .query_map([], |row| {
            Ok(PatternRecord {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                pattern: row.get(2)?,
            })
        })
        .unwrap();

    for mes in iter {
        println!("Pattern: {:?}", mes.unwrap());
    }
}

fn find_pattern(conn: &Connection, channel_id: &String) -> Option<i32> {
    let mut stmt = conn
        .prepare("SELECT * FROM pattern_record WHERE channel_id = ?1")
        .unwrap();

    let mut iter = stmt
        .query_map([&channel_id], |row| {
            Ok(PatternRecord {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                pattern: row.get(2)?,
            })
        })
        .unwrap();

    // for mes in iter {
    //     println!("Found Pattern: {:?}", mes.unwrap());
    // }

    match iter.next() {
        Some(pattern) => return Some(pattern.unwrap().id),
        None => return None,
    }
}

fn find_count(conn: &Connection, pattern_id: i32, user_id: &String) -> Option<i32> {
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
        Some(count) => return Some(count.unwrap().count),
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

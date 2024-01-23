use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct PatternRecord {
    pub id: i32,
    pub channel_id: String,
    pub pattern: String,
}

#[derive(Debug)]
pub struct CountRecord {
    pub id: i32,
    pub pattern_id: i32,
    pub user_id: String,
    pub count: i32,
}

pub fn create_table(conn: &Connection) {
    match conn.execute(
        "CREATE TABLE pattern_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel_id TEXT NOT NULL,
            pattern TEXT NOT NULL
        )",
        (),
    ) {
        Ok(_) => println!("Create: pattern_record"),
        Err(_) => println!("pattern_record already exists."),
    };

    match conn.execute(
        "CREATE TABLE count_record (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pattern_id INTEGER NOT NULL,
            user_id TEXT NOT NULL,
            count INTEGER NOT NULL
        )",
        (),
    ) {
        Ok(_) => println!("Create: count_record"),
        Err(_) => println!("count_record already exists."),
    }
}

pub fn register_pattern(conn: &Connection, record: &PatternRecord) {
    conn.execute(
        "INSERT INTO pattern_record (channel_id, pattern) VALUES (?1, ?2)",
        (&record.channel_id, &record.pattern),
    )
    .expect("Failed to insert message");
}

pub fn delete_pattern(conn: &Connection, pattern_id: i32) {
    conn.execute(
        "DELETE FROM pattern_record WHERE id = ?1",
        params![pattern_id],
    )
    .expect("Failed to delete pattern");
}

pub fn register_new_count(conn: &Connection, record: &CountRecord) {
    conn.execute(
        "INSERT INTO count_record (pattern_id, user_id, count) VALUES (?1, ?2, ?3)",
        (&record.pattern_id, &record.user_id, &record.count),
    )
    .expect("Failed to insert count");
}

pub fn update_count(conn: &Connection, record: &CountRecord) {
    conn.execute(
        "UPDATE count_record SET count = ?1 WHERE id = ?2",
        params![&record.count, &record.id],
    )
    .expect("Failed to update count");
}

pub fn find_pattern(
    conn: &Connection,
    channel_id: &String,
    message: &String,
) -> Option<PatternRecord> {
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

pub fn find_count(conn: &Connection, pattern_id: i32, user_id: &String) -> Option<CountRecord> {
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

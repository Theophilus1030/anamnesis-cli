use rusqlite::{Connection, Result};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");
        std::fs::create_dir_all(&app_dir).expect("failed to create app data dir");

        let db_path = app_dir.join("anamnesis.db");
        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pages (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        name TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS page_meta (
        page_id TEXT PRIMARY KEY,
        image_remote_url TEXT,
        image_local_path TEXT,
        line_count INTEGER,
        description_en TEXT,
        description_other TEXT,
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
    )",
            [],
        )?;
// 删除旧的三个表的建表语句（layout、transcription、translation）
// 修改 page_meta 表，只保留图片相关字段
conn.execute(
    "CREATE TABLE IF NOT EXISTS page_meta (
        page_id TEXT PRIMARY KEY,
        image_remote_url TEXT,
        image_local_path TEXT,
        description_en TEXT,
        description_other TEXT,
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
    )",
    [],
)?;

// 新增 alto 表，存原始 XML
conn.execute(
    "CREATE TABLE IF NOT EXISTS alto (
        page_id TEXT PRIMARY KEY,
        xml TEXT NOT NULL,
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
    )",
    [],
)?;

// 新增 sentence 表，存语义层 JSON
conn.execute(
    "CREATE TABLE IF NOT EXISTS sentence (
        page_id TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
    )",
    [],
)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

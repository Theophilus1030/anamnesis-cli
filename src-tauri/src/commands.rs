use crate::db::Database;
use serde::{Deserialize, Serialize};
use tauri::State;
use rusqlite::OptionalExtension;
#[derive(Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[tauri::command]
pub fn get_projects(db: State<Database>) -> Result<Vec<Project>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, created_at FROM projects ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(projects)
}

#[tauri::command]
pub fn create_project(db: State<Database>, id: String, name: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO projects (id, name, created_at) VALUES (?1, ?2, ?3)",
        (&id, &name, &now),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_project(db: State<Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM projects WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub created_at: i64,
}

#[tauri::command]
pub fn get_pages(db: State<Database>, project_id: String) -> Result<Vec<Page>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, project_id, name, created_at FROM pages WHERE project_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let pages = stmt
        .query_map([&project_id], |row| {
            Ok(Page {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(pages)
}

#[tauri::command]
pub fn create_page(db: State<Database>, id: String, project_id: String, name: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO pages (id, project_id, name, created_at) VALUES (?1, ?2, ?3, ?4)",
        (&id, &project_id, &name, &now),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_page(db: State<Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM pages WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PageMeta {
    pub page_id: String,
    pub image_remote_url: Option<String>,
    pub image_local_path: Option<String>,
    pub description_en: Option<String>,
    pub description_other: Option<String>,
}

#[tauri::command]
pub fn get_page_meta(db: State<Database>, page_id: String) -> Result<Option<PageMeta>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT page_id, image_remote_url, image_local_path, description_en, description_other FROM page_meta WHERE page_id = ?1")
        .map_err(|e| e.to_string())?;

    let meta = stmt
        .query_row([&page_id], |row| {
            Ok(PageMeta {
                page_id: row.get(0)?,
                image_remote_url: row.get(1)?,
                image_local_path: row.get(2)?,
                description_en: row.get(3)?,
                description_other: row.get(4)?,
            })
        })
        .optional()
        .map_err(|e: rusqlite::Error| e.to_string())?;

    Ok(meta)
}

#[tauri::command]
pub fn save_page_meta(db: State<Database>, meta: PageMeta) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT INTO page_meta (page_id, image_remote_url, image_local_path, description_en, description_other)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(page_id) DO UPDATE SET
            image_remote_url = excluded.image_remote_url,
            image_local_path = excluded.image_local_path,
            description_en = excluded.description_en,
            description_other = excluded.description_other",
        (
            &meta.page_id,
            &meta.image_remote_url,
            &meta.image_local_path,
            &meta.description_en,
            &meta.description_other,
        ),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn save_alto(db: State<Database>, page_id: String, xml: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO alto (page_id, xml) VALUES (?1, ?2)
         ON CONFLICT(page_id) DO UPDATE SET xml = excluded.xml",
        (&page_id, &xml),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_alto(db: State<Database>, page_id: String) -> Result<Option<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT xml FROM alto WHERE page_id = ?1")
        .map_err(|e| e.to_string())?;
    
    let xml = stmt
        .query_row([&page_id], |row| row.get(0))
        .optional()
        .map_err(|e: rusqlite::Error| e.to_string())?;
    
    Ok(xml)
}

#[tauri::command]
pub fn save_sentence(db: State<Database>, page_id: String, data: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO sentence (page_id, data) VALUES (?1, ?2)
         ON CONFLICT(page_id) DO UPDATE SET data = excluded.data",
        (&page_id, &data),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_sentence(db: State<Database>, page_id: String) -> Result<Option<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT data FROM sentence WHERE page_id = ?1")
        .map_err(|e| e.to_string())?;
    
    let data = stmt
        .query_row([&page_id], |row| row.get(0))
        .optional()
        .map_err(|e: rusqlite::Error| e.to_string())?;
    
    Ok(data)
}
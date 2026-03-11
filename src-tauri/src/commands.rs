use crate::db::Database;
use serde::{Deserialize, Serialize};
use tauri::State;
use rusqlite::OptionalExtension;
use std::path::PathBuf;
use std::process::Command;
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

#[tauri::command]
pub fn update_project_name(db: State<Database>, id: String, name: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE projects SET name = ?1 WHERE id = ?2",
        [&name, &id],
    )
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

#[tauri::command]
pub fn update_page_name(db: State<Database>, id: String, name: String) -> Result<(), String> {
    // 1. 获取数据库连接锁
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // 2. 执行更新语句
    // 注意 SQL 参数顺序：name 对应 ?1，id 对应 ?2
    conn.execute(
        "UPDATE pages SET name = ?1 WHERE id = ?2",
        [&name, &id],
    )
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





/// 获取资源目录路径
fn get_resource_dir() -> Result<PathBuf, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("获取exe路径失败: {}", e))?
        .parent()
        .ok_or("无法获取父目录")?
        .to_path_buf();

    // 尝试多个可能的路径
    let possible_paths = vec![
        // 打包后 Windows: exe同级的resources目录
        exe_dir.join("resources"),
        // 打包后 macOS: Resources目录
        exe_dir.join("../Resources/resources"),
        // 开发模式
        std::env::current_dir()
            .unwrap_or_default()
            .join("src-tauri")
            .join("resources"),
    ];

    possible_paths
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| "找不到资源目录".to_string())
}

/// 获取默认模型路径
fn get_default_model_path(model_name: &str) -> Result<String, String> {
    let resource_dir = get_resource_dir()?;
    let model_path = resource_dir.join(model_name);
    
    if model_path.exists() {
        Ok(model_path.to_string_lossy().to_string())
    } else {
        Err(format!("默认模型不存在: {}", model_path.display()))
    }
}

#[tauri::command]
pub async fn run_kraken_ocr(
    image_path: String,
    model_path: Option<String>,      // 改为 Option
    seg_model_path: Option<String>,  // 改为 Option
) -> Result<String, String> {
    // 如果没有指定模型，使用默认模型
    let model = match model_path {
        Some(p) if !p.is_empty() => p,
        _ => get_default_model_path("catmus-medieval.mlmodel")?,
    };

    let seg_model = match seg_model_path {
        Some(p) if !p.is_empty() => p,
        _ => get_default_model_path("blla.mlmodel")?,
    };

    // 获取可执行文件所在目录
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("获取exe路径失败: {}", e))?
        .parent()
        .ok_or("无法获取父目录")?
        .to_path_buf();

    // 尝试多个可能的路径
    // 尝试多个可能的路径
    let possible_paths = vec![
        // A. 开发环境 (必须带架构后缀)
        std::env::current_dir()
            .unwrap_or_default()
            .join("src-tauri")
            .join("binaries")
            .join("kraken_sidecar-x86_64-pc-windows-msvc.exe"),
        
        // B. 打包环境 - 情况1: Tauri 标准 externalBin (带后缀)
        exe_dir.join("kraken_sidecar-x86_64-pc-windows-msvc.exe"),
        
        // C. 打包环境 - 情况2: 你的现状 (不带后缀) <--- 新增这行
        exe_dir.join("kraken_sidecar.exe"),

        // D. 备用: 如果在 resources/binaries 子目录下
        exe_dir.join("resources").join("kraken_sidecar.exe"),
        exe_dir.join("binaries").join("kraken_sidecar.exe"),
    ];

    let sidecar_path = possible_paths
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            format!(
                "找不到 sidecar，尝试过的路径:\n{}",
                possible_paths
                    .iter()
                    .map(|p| format!("  - {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })?;

    // 执行命令
    let output = Command::new(sidecar_path)
        .arg(&image_path)
        .arg("--model")
        .arg(&model)
        .arg("--seg-model")
        .arg(&seg_model)
        .output()
        .map_err(|e| format!("执行失败: {}", e))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| format!("输出编码错误: {}", e))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("进程错误 (code {:?}): {}", output.status.code(), stderr))
    }
}

/// 检查默认模型是否存在
#[tauri::command]
pub fn check_default_models() -> Result<DefaultModelsStatus, String> {
    let resource_dir = get_resource_dir().ok();
    
    let rec_model_exists = resource_dir
        .as_ref()
        .map(|d| d.join("catmus-medieval.mlmodel").exists())
        .unwrap_or(false);
    
    let seg_model_exists = resource_dir
        .as_ref()
        .map(|d| d.join("blla.mlmodel").exists())
        .unwrap_or(false);

    Ok(DefaultModelsStatus {
        recognition_model: rec_model_exists,
        segmentation_model: seg_model_exists,
    })
}

#[derive(serde::Serialize)]
pub struct DefaultModelsStatus {
    recognition_model: bool,
    segmentation_model: bool,
}

#[tauri::command]
pub fn save_content(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}
#[macro_use]
extern crate rocket;

use rocket::fs::FileServer;
use rocket::http::Status;
use rocket::response::status::{self, Custom};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::State;
use rocket_cors::CorsOptions;
use rusqlite::{params, Connection, Row};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
struct Note {
    id: i64,
    title: String,
    content: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct NoteInput {
    title: String,
    content: String,
}

/// SQLite 连接放在 Mutex 里通过 Rocket state 共享
struct Db(Mutex<Connection>);

const SELECT_COLS: &str = "id, title, content, created_at, updated_at";

fn row_to_note(row: &Row) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

type DbError = Custom<String>;

fn db_lock_err() -> DbError {
    Custom(
        Status::InternalServerError,
        "database lock poisoned".to_string(),
    )
}

fn db_err(e: rusqlite::Error) -> DbError {
    eprintln!("sqlite error: {}", e);
    Custom(
        Status::InternalServerError,
        "database error".to_string(),
    )
}

fn fetch_note(conn: &Connection, id: i64) -> rusqlite::Result<Note> {
    let sql = format!("SELECT {} FROM notes WHERE id = ?1", SELECT_COLS);
    conn.query_row(&sql, params![id], row_to_note)
}

/// 执行列表查询；param 非空时按标题/内容模糊匹配
fn query_notes(conn: &Connection, param: &str) -> Result<Vec<Note>, DbError> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {} FROM notes {} ORDER BY updated_at DESC",
            SELECT_COLS,
            if param.is_empty() {
                ""
            } else {
                "WHERE title LIKE ?1 OR content LIKE ?1"
            }
        ))
        .map_err(db_err)?;
    let rows = if param.is_empty() {
        stmt.query_map([], row_to_note).map_err(db_err)?
    } else {
        stmt.query_map(params![param], row_to_note).map_err(db_err)?
    };
    rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
}

/// GET /api/notes?query=xxx —— 列表，支持按标题/内容模糊搜索
#[get("/api/notes?<query>")]
fn list_notes(db: &State<Db>, query: Option<String>) -> Result<Json<Vec<Note>>, DbError> {
    let conn = db.0.lock().map_err(|_| db_lock_err())?;
    let like = query
        .filter(|q| !q.trim().is_empty())
        .map(|q| format!("%{}%", q.trim()))
        .unwrap_or_default();
    Ok(Json(query_notes(&conn, &like)?))
}

/// GET /api/notes/<id> —— 单条详情
#[get("/api/notes/<id>")]
fn get_note(db: &State<Db>, id: i64) -> Result<Json<Note>, DbError> {
    let conn = db.0.lock().map_err(|_| db_lock_err())?;
    let note = fetch_note(&conn, id).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            Custom(Status::NotFound, format!("note {} not found", id))
        }
        other => db_err(other),
    })?;
    Ok(Json(note))
}

/// POST /api/notes —— 新建
#[post("/api/notes", data = "<input>", format = "json")]
fn create_note(
    db: &State<Db>,
    input: Json<NoteInput>,
) -> Result<status::Created<Json<Note>>, DbError> {
    let conn = db.0.lock().map_err(|_| db_lock_err())?;
    conn.execute(
        "INSERT INTO notes (title, content) VALUES (?1, ?2)",
        params![input.title, input.content],
    )
    .map_err(db_err)?;
    let id = conn.last_insert_rowid();
    let note = fetch_note(&conn, id).map_err(db_err)?;
    Ok(status::Created::new(format!("/api/notes/{}", id)).body(Json(note)))
}

/// PUT /api/notes/<id> —— 更新
#[put("/api/notes/<id>", data = "<input>", format = "json")]
fn update_note(db: &State<Db>, id: i64, input: Json<NoteInput>) -> Result<Json<Note>, DbError> {
    let conn = db.0.lock().map_err(|_| db_lock_err())?;
    let n = conn
        .execute(
            "UPDATE notes SET title = ?1, content = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![input.title, input.content, id],
        )
        .map_err(db_err)?;
    if n == 0 {
        return Err(Custom(
            Status::NotFound,
            format!("note {} not found", id),
        ));
    }
    let note = fetch_note(&conn, id).map_err(db_err)?;
    Ok(Json(note))
}

/// DELETE /api/notes/<id> —— 删除
#[delete("/api/notes/<id>")]
fn delete_note(db: &State<Db>, id: i64) -> Result<Status, DbError> {
    let conn = db.0.lock().map_err(|_| db_lock_err())?;
    let n = conn
        .execute("DELETE FROM notes WHERE id = ?1", params![id])
        .map_err(db_err)?;
    if n == 0 {
        return Err(Custom(
            Status::NotFound,
            format!("note {} not found", id),
        ));
    }
    Ok(Status::NoContent)
}

#[launch]
fn rocket() -> _ {
    // 数据库文件路径，可用 DATABASE_URL 环境变量覆盖（如 Docker 中 /data/notes.db）
    let db_path = std::env::var("DATABASE_URL").unwrap_or_else(|_| "notes.db".to_string());
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create database directory");
        }
    }
    let conn = Connection::open(&db_path).expect("open sqlite database");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS notes (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            title       TEXT NOT NULL,
            content     TEXT NOT NULL DEFAULT '',
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .expect("initialize notes table");

    // 开发模式下前端（Vite dev server，5173 端口）跨域访问
    let cors = CorsOptions::default().to_cors().expect("build cors");

    // 生产模式下由 Rocket 直接托管前端构建产物（backend/static 目录）
    rocket::build()
        .manage(Db(Mutex::new(conn)))
        .attach(cors)
        .mount(
            "/",
            routes![list_notes, get_note, create_note, update_note, delete_note],
        )
        .mount("/", FileServer::from("static"))
}

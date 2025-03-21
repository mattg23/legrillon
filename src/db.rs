use std::sync::Arc;

use fltk::app::{self, Sender};
use sqlx::{Pool, Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::{GlobalAppMsg, WINDOW_ID_COUNTER};

#[derive(Debug, sqlx::FromRow)]
struct SentRequest {
    id: i64,
    sent_at: chrono::DateTime<chrono::Local>,
    method: String,
    uri: String,
    path: String,
    query: String,
    headers: sqlx::types::Json<Vec<(String, String)>>,
    body: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OpenWindow {
    pub id: i32,
    pub method: String,
    pub uri: String,
    pub path: String,
    pub query: String,
    pub headers: sqlx::types::Json<Vec<(String, String)>>,
    pub body: String,
}

pub struct LeGrillonDb {
    pool: Pool<Sqlite>,
    global: Sender<GlobalAppMsg>,
}

const DB_URL: &str = "sqlite://sqlite.db";

impl LeGrillonDb {
    pub async fn new() -> Self {
        if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
            println!("Creating database {}", DB_URL);
            match Sqlite::create_database(DB_URL).await {
                Ok(_) => println!("Create db success"),
                Err(error) => panic!("error: {}", error),
            };
        } else {
            println!("Database already exists");
        }

        let pool = SqlitePool::connect(DB_URL).await.unwrap();

        Self::setup(&pool).await;

        let (global, _) = app::channel();

        Self { pool, global }
    }

    pub fn handle(s: Arc<Self>, msg: GlobalAppMsg) {
        tokio::spawn(async move {
            s.handle_msg(msg).await;
        });
    }

    pub fn restore(s: Arc<Self>) {
        tokio::spawn(async move {
            s.restore_open_windows().await;
        });
    }

    async fn restore_open_windows(&self) {
        let wins = sqlx::query_as::<_, OpenWindow>(
            "
            SELECT * FROM OpenWindows
        ",
        )
        .fetch_all(&self.pool)
        .await;

        println!("{wins:?}");

        let mut max_id = 0;

        if let Ok(wins) = wins {
            for w in wins {
                if w.id > max_id {
                    max_id = w.id;
                }

                self.global.send(GlobalAppMsg::Restore(w));
            }
        }

        WINDOW_ID_COUNTER.fetch_max(max_id as usize, std::sync::atomic::Ordering::SeqCst);
    }

    async fn handle_msg(&self, msg: GlobalAppMsg) {
        match msg {
            GlobalAppMsg::OpenEmptyWindow => (),
            GlobalAppMsg::Restore(_) => (),
            GlobalAppMsg::CloseWindow(id) => {
                let close_window = sqlx::query(
                    "
                    DELETE FROM OpenWindows
                    WHERE id = ?
                ",
                )
                .bind(id as i64)
                .execute(&self.pool)
                .await;
                println!("DB::CLOSE_WINDOW:: {close_window:?}")
            }
            GlobalAppMsg::SaveWindowState(open_window) => {
                let save = sqlx::query(
                    "
                    INSERT INTO OpenWindows (id, method, uri, path, query, headers, body)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ON CONFLICT(id) DO UPDATE SET
                        method = excluded.method,
                        uri = excluded.uri,
                        path = excluded.path,
                        query = excluded.query,
                        headers = excluded.headers,
                        body = excluded.body

                ",
                )
                .bind(open_window.id)
                .bind(open_window.method)
                .bind(open_window.uri)
                .bind(open_window.path)
                .bind(open_window.query)
                .bind(open_window.headers)
                .bind(open_window.body)
                .execute(&self.pool)
                .await;
                println!("{save:?}");
            }
            GlobalAppMsg::CloseApp => (),
        }
    }

    async fn setup(pool: &Pool<Sqlite>) {
        let r = sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS OpenWindows (
                id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                method VARCHAR(32) NOT NULL,
                uri VARCHAR(256) NOT NULL,
                path VARCHAR(1024),
                query TEXT,
                headers TEXT,
                body TEXT
            );
        ",
        )
        .execute(pool)
        .await;

        println!("{r:?}");
        let r = sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS SentRequest (
                id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                sent_at TEXT NOT NULL,
                method VARCHAR(32) NOT NULL,
                uri VARCHAR(256) NOT NULL,
                path VARCHAR(1024),
                query TEXT,
                headers TEXT,
                body TEXT
            );
        ",
        )
        .execute(pool)
        .await;

        println!("{r:?}");
    }
}

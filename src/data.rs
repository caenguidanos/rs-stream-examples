use std::fmt::{Display, Formatter};
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Error, FromRow, Pool, Sqlite, SqlitePool};

#[derive(FromRow, Debug, Clone)]
pub struct Document {
    pub id: String,
    pub name: String,
}

impl Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "id={},name={}", self.id, self.name)
    }
}

fn create_documents() -> Vec<Document> {
    let mut documents = vec![];

    for n in 0..10_000 {
        documents.push(Document {
            id: ulid::Ulid::new().to_string(),
            name: format!("Name {n}"),
        });
    }

    documents
}

pub async fn get_documents(conn: &Pool<Sqlite>) -> Vec<Document> {
    sqlx::query_as::<_, Document>("SELECT * FROM documents")
        .fetch_all(conn)
        .await
        .unwrap_or_default()
}

pub async fn create_database_conn() -> Result<Pool<Sqlite>, Error> {
    let conn_options =
        SqliteConnectOptions::from_str("sqlite::memory:")?.journal_mode(SqliteJournalMode::Wal);

    SqlitePool::connect_with(conn_options).await
}

pub async fn seed_database() -> Result<Pool<Sqlite>, Error> {
    let conn = create_database_conn().await?;

    sqlx::query(
        r#"
                CREATE TABLE documents (
                    id TEXT,
                    name TEXT NOT NULL,
                    PRIMARY KEY (id)
                )
            "#,
    )
    .execute(&conn)
    .await?;

    let documents = create_documents();

    for document in documents {
        sqlx::query("INSERT INTO documents (id, name) VALUES (?, ?)")
            .bind(&document.id)
            .bind(&document.name)
            .execute(&conn)
            .await?;
    }

    Ok(conn)
}

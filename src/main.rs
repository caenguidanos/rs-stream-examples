use axum::response::Html;
use axum::{routing::get, Router};
use sqlx::{Pool, Sqlite};

mod data;
mod route;

#[derive(Clone)]
pub struct AppState {
    pub conn: Pool<Sqlite>,
}

#[tokio::main]
async fn main() {
    let conn = data::seed_database().await.unwrap();

    let app = Router::new()
        .route(
            "/",
            get(|| async {
                Html(
                    r#"
                        <a href="/stream">Stream!</a>
                        <a href="/stream-zip">Stream ZIP!</a>
                        <a href="/stream-zip-and-gzip">Stream ZIP with gzip compression!</a>
                    "#,
                )
            }),
        )
        .route("/stream", get(route::stream))
        .route("/stream-zip", get(route::stream_zip))
        .route("/stream-zip-and-gzip", get(route::stream_zip_and_gzip))
        .with_state(AppState { conn });

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

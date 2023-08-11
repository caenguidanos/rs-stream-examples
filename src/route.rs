use std::io::Write;
use std::sync::Arc;

use axum::body::StreamBody;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;

use crate::{data, AppState};

pub async fn stream(State(state): State<AppState>) -> impl IntoResponse {
    let (sender, stream) = hyper::body::Body::channel();

    let response_sender = Arc::new(Mutex::new(sender));

    tokio::spawn(async move {
        let documents = data::get_documents(&state.conn).await;

        let mut tasks = tokio::task::JoinSet::new();

        for document in documents.into_iter() {
            let response_sender = Arc::clone(&response_sender);

            tasks.spawn(async move {
                let content = format!("Invoice {}", document.id);

                let mut response_sender_locked = response_sender.lock().await;

                if let Err(err) = response_sender_locked
                    .send_data(bytes::Bytes::from(content))
                    .await
                {
                    println!("{err}");
                }

                document.id
            });
        }

        while let Some(id) = tasks.join_next().await {
            println!("Processed {id:?}");
        }
    });

    let headers = [("content-type", "text/plain; charset=utf-8")];

    (StatusCode::OK, headers, StreamBody::from(stream))
}

pub async fn stream_zip(State(state): State<AppState>) -> impl IntoResponse {
    let (wx, rx) = tokio::io::duplex(4 * 1024);

    tokio::spawn(async move {
        let documents = data::get_documents(&state.conn).await;

        let mut tasks = tokio::task::JoinSet::new();

        let file_archive = Arc::new(Mutex::new(zipit::Archive::new(wx)));

        for document in documents.into_iter() {
            let file_archive = Arc::clone(&file_archive);

            tasks.spawn(async move {
                let file_name = format!("{}.txt", document.id);
                let file_content = format!("Invoice {}", document.id);

                let mut file_content_cursor = std::io::Cursor::new(file_content);
                let mut file_archive = file_archive.lock().await;

                if let Err(error) = file_archive
                    .append(
                        file_name,
                        zipit::FileDateTime::now(),
                        &mut file_content_cursor,
                    )
                    .await
                {
                    println!("{error}");
                }

                document.id
            });
        }

        while let Some(id) = tasks.join_next().await {
            println!("Processed {id:?}");
        }

        if let Ok(inner) = Arc::try_unwrap(file_archive) {
            let file_archive = inner.into_inner();

            if let Err(error) = file_archive.finalize().await {
                println!("{error}");
            }
        }
    });

    let headers = [
        ("content-type", "application/zip"),
        (
            "content-disposition",
            r#"attachment; filename="documents.zip""#,
        ),
    ];

    (
        StatusCode::OK,
        headers,
        StreamBody::from(ReaderStream::new(rx)),
    )
}

pub async fn stream_zip_and_gzip(State(state): State<AppState>) -> impl IntoResponse {
    let (wx, rx) = tokio::io::duplex(4 * 1024);

    tokio::spawn(async move {
        let documents = data::get_documents(&state.conn).await;

        let mut tasks = tokio::task::JoinSet::new();

        let file_archive = Arc::new(Mutex::new(zipit::Archive::new(wx)));

        for document in documents.into_iter() {
            let file_archive = Arc::clone(&file_archive);

            tasks.spawn(async move {
                let file_name = format!("{}.zip", document.id);
                let file_content = format!("Invoice {}", document.id);

                let mut compressor =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());

                if compressor.write_all(file_content.as_bytes()).is_ok() {
                    let file_content_compressed = compressor.finish().unwrap_or_default();

                    let mut file_content_cursor = std::io::Cursor::new(file_content_compressed);
                    let mut file_archive_locked = file_archive.lock().await;

                    if let Err(error) = file_archive_locked
                        .append(
                            file_name,
                            zipit::FileDateTime::now(),
                            &mut file_content_cursor,
                        )
                        .await
                    {
                        println!("{error}");
                    }
                }

                document.id
            });
        }

        while let Some(id) = tasks.join_next().await {
            println!("Processed {id:?}");
        }

        if let Ok(inner) = Arc::try_unwrap(file_archive) {
            let file_archive = inner.into_inner();

            if let Err(error) = file_archive.finalize().await {
                println!("{error}");
            }
        }
    });

    let headers = [
        ("content-type", "application/zip"),
        (
            "content-disposition",
            "attachment; filename=\"documents.zip\"",
        ),
    ];

    (
        StatusCode::OK,
        headers,
        StreamBody::from(ReaderStream::new(rx)),
    )
}

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

    let sender = Arc::new(Mutex::new(sender));

    tokio::spawn(async move {
        let documents = data::get_documents(&state.conn).await;

        let mut tasks = tokio::task::JoinSet::new();

        for document in documents.into_iter() {
            let sender = Arc::clone(&sender);

            tasks.spawn(async move {
                let parsed = format!("Invoice {}", document.id);

                let mut sender = sender.lock().await;

                if let Err(err) = sender.send_data(bytes::Bytes::from(parsed)).await {
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

        let archive = Arc::new(Mutex::new(zipit::Archive::new(wx)));

        for document in documents.into_iter() {
            let archive = Arc::clone(&archive);

            tasks.spawn(async move {
                let filename = format!("{}.txt", document.id);
                let content = format!("Invoice {}", document.id);

                let mut content_cursor = std::io::Cursor::new(bytes::Bytes::from(content));

                let mut archive = archive.lock().await;

                if let Err(error) = archive
                    .append(filename, zipit::FileDateTime::now(), &mut content_cursor)
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

        if let Ok(inner) = Arc::try_unwrap(archive) {
            let archive = inner.into_inner();

            if let Err(error) = archive.finalize().await {
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

        let archive = Arc::new(Mutex::new(zipit::Archive::new(wx)));

        for document in documents.into_iter() {
            let archive = Arc::clone(&archive);

            tasks.spawn(async move {
                let filename = format!("{}.zip", document.id);
                let content = format!("Invoice {}", document.id);

                let mut compressor =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());

                if compressor.write_all(content.as_bytes()).is_ok() {
                    let compressed_content = compressor.finish().unwrap_or_default();

                    let mut content_cursor = std::io::Cursor::new(compressed_content);

                    let mut archive = archive.lock().await;

                    if let Err(error) = archive
                        .append(filename, zipit::FileDateTime::now(), &mut content_cursor)
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

        if let Ok(inner) = Arc::try_unwrap(archive) {
            let archive = inner.into_inner();

            if let Err(error) = archive.finalize().await {
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

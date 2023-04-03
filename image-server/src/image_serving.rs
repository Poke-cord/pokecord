use std::path::PathBuf;
use std::time::Duration;

use bytes::BytesMut;
use color_eyre::eyre::Context;
use color_eyre::{eyre::eyre, Result};
use futures::TryStreamExt;
use hyper::{
    header::{self, HeaderValue},
    Body, Response, StatusCode,
};
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot::Sender;
use tracing::error;

async fn fetch_image(client: &Client, path: &str) -> Option<reqwest::Response> {
    let resp = client.get(path).send().await.ok()?;
    let content_type = resp.headers().get("content-type")?.to_str().ok()?;
    if content_type.starts_with("image/") {
        Some(resp)
    } else {
        None
    }
}

async fn serve_image(
    local_cache_ttl: &Duration,
    path: PathBuf,
) -> tokio::io::Result<Response<Body>> {
    let mut file = match File::open(&path).await {
        Ok(file) => file,
        Err(err) => return Err(err),
    };
    let file_metadata = file.metadata().await?;

    if let Ok(time_since_file_created) = file_metadata.created()?.elapsed() {
        if &time_since_file_created > local_cache_ttl {
            tokio::fs::remove_file(&path).await?;
            return Err(tokio::io::Error::new(
                tokio::io::ErrorKind::NotFound,
                eyre!("Local cache expired for {}", path.display()),
            ));
        }
    }

    let file_len = file_metadata.len();

    let mut buf = BytesMut::with_capacity(8192);

    let (mut tx, body) = Body::channel();

    tokio::spawn(async move {
        loop {
            let n = match file.read_buf(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(n) => n,
                Err(err) => {
                    error!("Failed to read file: {}", err);
                    tx.abort();
                    break;
                }
            };
            buf.truncate(n);
            let _ = tx.send_data(buf.clone().freeze()).await;
            buf.clear();
        }
    });

    Ok(Response::builder()
        .header(header::CONTENT_LENGTH, file_len)
        .header(header::CONTENT_TYPE, HeaderValue::from_static("image/png"))
        .body(body)
        .unwrap())
}

async fn fetch_save_and_serve_image(
    client: Client,
    path: PathBuf,
    url: String,
    sender: Sender<Response<Body>>,
) -> Result<()> {
    let resp = match fetch_image(&client, &url).await {
        Some(resp) => resp,
        None => {
            sender
                .send(
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::empty())?,
                )
                .unwrap();
            return Err(eyre!("Invalid image URL"));
        }
    };

    let internal_server_error = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::empty())?;

    match path
        .parent()
        .ok_or(eyre!("Parent folder not found for {}", path.display()))
        .map(tokio::fs::create_dir_all)
    {
        Err(e) => {
            sender.send(internal_server_error).unwrap();
            return Err(e);
        }
        Ok(a) => {
            if let Err(e) = a.await {
                sender.send(internal_server_error).unwrap();
                return Err(e).wrap_err_with(|| {
                    format!("Failed to create parent directory for {}", path.display())
                });
            }
        }
    }

    let mut file = match File::create(&path).await {
        Ok(file) => file,
        Err(err) => {
            return Err(eyre!(format!("failed to create file: {:?}", err),));
        }
    };

    let (mut body_sender, body) = Body::channel();

    let response = Response::builder().status(StatusCode::OK).body(body)?;

    sender.send(response).unwrap();

    let mut writer = io::BufWriter::new(&mut file);

    let mut buf = BytesMut::with_capacity(8192);

    let mut resp_byte_stream = resp
        .bytes_stream()
        .map_err(|e| eyre!("Failed to read bytes stream: {}", e));

    while let Some(chunk) = resp_byte_stream.try_next().await? {
        buf.extend_from_slice(&chunk);
        writer.write_all(&chunk).await?;
        body_sender.send_data(chunk.clone()).await?;
        if buf.len() >= 8192 {
            let _ = writer.flush().await;
            buf.clear();
        }
    }

    let _ = writer.flush().await;

    Ok(())
}

pub async fn fetch_if_needed_and_serve_image(
    client: Client,
    path: PathBuf,
    url: String,
    local_cache_ttl: &Duration,
) -> Response<Body> {
    match serve_image(local_cache_ttl, path.clone()).await {
        Ok(resp) => return resp,
        Err(e) => {
            if e.kind() != tokio::io::ErrorKind::NotFound {
                error!("Failed to serve image: {}", e);
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap();
            }
        }
    }
    let (tx, rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        match fetch_save_and_serve_image(client, path, url, tx).await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to save image: {}", e);
            }
        }
    });

    rx.await.unwrap()
}

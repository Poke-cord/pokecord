use std::net::SocketAddr;
use std::str::FromStr;

use axum::{routing::get, Extension, Router, Server};
use color_eyre::{eyre::eyre, Result};
use directories::ProjectDirs;
use reqwest::Client;
use tracing::info;

mod image_serving;
mod routes;
mod settings;

use settings::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt::init();

    let settings = Settings::load();
    let reqwest_client = Client::new();

    let app = Router::new()
        .route("/:type/:file", get(routes::image_handler))
        .layer(Extension(
            ProjectDirs::from("xyz.pokecord", "Pokecord LLC", "pokecord-image-server")
                .ok_or(eyre!("Failed to determine project directories."))?,
        ))
        .layer(Extension(reqwest_client))
        .layer(Extension(settings));

    let addr =
        SocketAddr::from_str(&std::env::var("LISTEN_ADDR").unwrap_or("0.0.0.0:3000".to_string()))?;

    info!("listening on {}", addr);

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

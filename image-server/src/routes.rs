use axum::{extract::Path, Extension};
use axum_macros::debug_handler;
use directories::ProjectDirs;
use hyper::{Body, Response};
use reqwest::Client;

use crate::{image_serving::fetch_if_needed_and_serve_image, settings::Settings};

#[debug_handler]
pub async fn image_handler(
    Path((typ, file)): Path<(String, String)>,
    Extension(project_dirs): Extension<ProjectDirs>,
    Extension(settings): Extension<Settings>,
    Extension(reqwest_client): Extension<Client>,
) -> Response<Body> {
    let path = format!("{}/{}", typ, file);
    let url = format!("{}/{}", settings.image_host_base_url(), path);
    let file_path = project_dirs.cache_dir().join(path);
    fetch_if_needed_and_serve_image(reqwest_client, file_path, url, settings.local_cache_ttl())
        .await
}

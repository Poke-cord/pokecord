use std::{env, time::Duration};

const DEFAULT_IMAGE_HOST_BASE_URL: &str =
    "https://raw.githubusercontent.com/Poke-cord/images/main/";
const DEFAULT_LOCAL_CACHE_TTL_MS: u64 = 21600000;

#[derive(Clone, Debug)]
pub struct Settings {
    image_host_base_url: String,
    local_cache_ttl: Duration,
}

impl Settings {
    pub fn load() -> Self {
        let image_host_base_url =
            env::var("IMAGE_HOST_BASE_URL").unwrap_or(DEFAULT_IMAGE_HOST_BASE_URL.to_string());
        let local_cache_ttl_ms = env::var("LOCAL_CACHE_TTL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_LOCAL_CACHE_TTL_MS);

        Self {
            image_host_base_url,
            local_cache_ttl: Duration::from_millis(local_cache_ttl_ms),
        }
    }

    pub fn image_host_base_url(&self) -> &str {
        &self.image_host_base_url
    }

    pub fn local_cache_ttl(&self) -> &Duration {
        &self.local_cache_ttl
    }
}

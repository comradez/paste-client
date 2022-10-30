use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub base_url: String,
    pub proxy: Option<String>,
}

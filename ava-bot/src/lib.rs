use crate::handlers::AssistantEvent;
use clap::Parser;
use dashmap::DashMap;
use llm_sdk::{LlmSDK};
use once_cell::sync::Lazy;
use std::env;
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;

mod error;
pub mod handlers;
mod tools;

#[derive(Debug, Parser)]
#[clap(name = "ava")]
pub struct Args {
    #[clap(short, long, default_value = "8080")]
    pub port: u16,
    #[clap(short, long, default_value = ".certs")]
    pub cert_path: String,
}

pub static LLM_SDK: Lazy<LlmSDK> = Lazy::new(|| {
    let sdk = LlmSDK::new_with_base_url(
        env::var("OPENAI_API_KEY").unwrap(),
        "https://api.xty.app/v1",
    );
    sdk
    /*LlmSDK::new_with_base_url(
        "sk-".to_string(),
        "https://api.openai.com/v1",
    )*/
});

pub(crate) static EVENTS: Lazy<DashMap<String, broadcast::Sender<AssistantEvent>>> =
    Lazy::new(DashMap::new);

pub fn audio_path(device_id: &str, name: &str) -> PathBuf {
    Path::new("./tmp/ava-bot/audio")
        .join(device_id)
        .join(format!("{}.mp3", name))
}
pub fn audio_url(device_id: &str, name: &str) -> String {
    format!("./assets/audio/{}/{}.mp3", device_id, name)
}

pub fn image_path(device_id: &str, name: &str) -> PathBuf {
    Path::new("./tmp/ava-bot/image")
        .join(device_id)
        .join(format!("{}.png", name))
}

pub fn image_url(device_id: &str, name: &str) -> String {
    format!("./assets/image/{}/{}.png", device_id, name)
}

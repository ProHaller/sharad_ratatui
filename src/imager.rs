use crate::{
    error::{Error, Result},
    settings::Settings,
};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{CreateImageRequestArgs, ImageModel, ImageResponseFormat, ImageSize, ImagesResponse},
};
use futures::TryFutureExt;
use std::{path::PathBuf, process::Command};
use tokio::time::{Duration, timeout};

pub async fn generate_and_save_image(prompt: &str) -> Result<PathBuf> {
    let settings = Settings::load()?;
    let api_key = match settings.openai_api_key {
        Some(key) => key,
        None => return Err(Error::from("No API key provided.")),
    };

    let openai_config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(openai_config);
    let request = CreateImageRequestArgs::default()
        .prompt(prompt)
        .model(ImageModel::DallE3)
        .n(1)
        .response_format(ImageResponseFormat::Url)
        .size(ImageSize::S1024x1792)
        .build()
        .map_err(|e| Error::AI(e.into()))?;

    let response: ImagesResponse =
        match timeout(Duration::from_secs(120), client.images().create(request)).await {
            Ok(res) => res.map_err(|e| Error::AI(e.into()))?,
            Err(_) => return Err("Request timed out.".into()),
        };

    if response.data.is_empty() {
        return Err("No image URLs received.".into());
    }

    let home_dir = dir::home_dir().expect("Failed to get home directory");
    let path = home_dir.join("sharad").join("data");
    let paths: Vec<PathBuf> = response.save(path).map_err(|e| Error::AI(e.into())).await?;
    if let Some(path) = paths.first() {
        // Convert the path to a string
        let path_str = path.to_str().ok_or("Invalid path")?;

        // Open the image using the default image viewer based on the OS
        #[cfg(target_os = "macos")]
        Command::new("open").arg(path_str).spawn()?;

        #[cfg(target_os = "windows")]
        Command::new("cmd")
            .args(&["/C", "start", "", path_str])
            .spawn()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg(path_str).spawn()?;

        Ok(path.clone())
    } else {
        Err("No image file path received.".into())
    }
}

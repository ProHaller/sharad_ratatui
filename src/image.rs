use async_openai::{
    config::OpenAIConfig,
    types::{CreateImageRequestArgs, ImageModel, ImageResponseFormat, ImageSize, ResponseFormat},
    Client,
};
use std::error::Error;
use tokio::time::{timeout, Duration};

pub async fn generate_and_save_image(api_key: &str, prompt: &str) -> Result<(), Box<dyn Error>> {
    let openai_config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(openai_config);
    let request = CreateImageRequestArgs::default()
        .prompt(prompt)
        .model(ImageModel::DallE3)
        .n(1)
        .response_format(ImageResponseFormat::Url)
        .size(ImageSize::S1024x1792)
        .build()?;

    let response = match timeout(Duration::from_secs(120), client.images().create(request)).await {
        Ok(res) => res?,
        Err(_) => return Err("Request timed out.".into()),
    };

    if response.data.is_empty() {
        return Err("No image URLs received.".into());
    }

    let paths = response.save("./data").await?;
    if let Some(_path) = paths.first() {
        Ok(())
    } else {
        Err("No image file path received.".into())
    }
}

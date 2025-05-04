use crate::error::{Error, Result};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{CreateImageRequestArgs, ImageModel, ImageResponseFormat, ImageSize, ImagesResponse},
};
use futures::TryFutureExt;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use std::{path::PathBuf, process::Command};

fn add_sharad_prepromt(prompt: &str) -> String {
    let sharad_prompt = format!(
        "Create a detailed character portrait in the gritty, high-tech noir style of Shadowrun. The artwork should evoke a dark cyberpunk atmosphere, with dramatic lighting, dystopian urban backgrounds, and a mix of futuristic tech and urban decay. The character should look like they belong in a world of shadowy megacorps, street samurai, deckers, and awakened magic. Use bold lines, strong contrasts, and realistic proportions. Clothing and gear should reflect their role—cybernetic implants, armor, magical auras, or hacker rigs—blending fantasy and cyberpunk elements in a grounded, worn world. Do not write text on the image. Use the full 9:16 ratio. Image prompt: {}",
        prompt
    );
    sharad_prompt
}

// TODO: implement an image correction/edition method.
pub async fn generate_and_save_image(
    client: Client<OpenAIConfig>,
    prompt: &str,
) -> Result<PathBuf> {
    log::debug!("generate_and_save_image: {prompt}");
    let prompt = add_sharad_prepromt(prompt);
    log::debug!("Arranged Prompt: {prompt}");

    let request = CreateImageRequestArgs::default()
        .prompt(prompt)
        .model(ImageModel::DallE3)
        .n(1)
        .response_format(ImageResponseFormat::Url)
        .size(ImageSize::S1024x1792)
        .build()
        .map_err(|e| Error::AI(e.into()))?;

    let response: ImagesResponse = match client.images().create(request).await {
        Ok(res) => {
            log::debug!("generate_and_save_image response: {res:#?}");
            res
        }
        Err(e) => {
            log::error!("generate_and_save_image: {e:#?}");
            return Err(Error::AI(e.into()));
        }
    };

    if response.data.is_empty() {
        log::error!("Image creation response is empty.");
        return Err("No image URLs received.".into());
    }

    let home_dir = dir::home_dir().expect("Failed to get home directory");
    let path = home_dir.join("sharad").join("data");
    log::debug!("Saving the image here: {path:#?}");
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

pub fn load_image_from_file(picker: &Picker, path: &PathBuf) -> Result<StatefulProtocol> {
    // Open and decode the image file
    match image::ImageReader::open(path)?.decode() {
        Ok(image) => Ok(picker.new_resize_protocol(image)),
        Err(err) => {
            // Convert ImageError to ShadowrunError using the implemented From trait
            Err(err.to_string().into())
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_image_load_from_file() {
        let picker = Picker::from_query_stdio().unwrap();
        let path =
            PathBuf::from("/Users/prohaller/sharad/save/portrait/img-sds4GqNc5Fbm4G7T6rMKgHr4.png");
        let result = load_image_from_file(&picker, &path);
        assert!(result.is_ok());
    }
}

use include_dir::{Dir, DirEntry, include_dir};
use serde_json::Value;
use std::{error::Error, fs::File, io::Read, path::PathBuf};

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        AssistantObject, AssistantTools, AssistantsApiResponseFormatOption,
        CreateAssistantRequestArgs, FunctionObject, ResponseFormat, ResponseFormatJsonSchema,
    },
};

// TODO: Make sure the model is formating properly the dialogue responses in French and english.
static ASSETS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

fn load_function_objects() -> Result<Vec<FunctionObject>, Box<dyn Error>> {
    let folder_dir = ASSETS_DIR
        .get_dir("assistant_functions")
        .expect("Failed to get assistant_functions directory");

    let mut function_objects = Vec::new();

    // Read the folder
    for entry in folder_dir.entries() {
        match entry {
            DirEntry::File(file) => {
                let path = file.path();

                // Ensure the entry is a JSON file
                if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                    // Read the file contents
                    let content = file
                        .contents_utf8()
                        .ok_or("File content is not valid UTF-8")?;

                    // Parse the content as a JSON value
                    let function_data: Value = serde_json::from_str(content)?;

                    // Extract relevant fields from the JSON object
                    let name = function_data["name"].as_str().unwrap_or_default();
                    let description = function_data["description"].as_str().unwrap_or_default();
                    let parameters = function_data["parameters"].clone(); // This extracts the parameters part
                    let strict = function_data["strict"].as_bool().unwrap_or(true); // Defaults to true if not found

                    // Create a FunctionObject and push it to the vector
                    let function_object = FunctionObject {
                        name: name.to_string(),
                        description: Some(description.to_string()),
                        parameters: Some(parameters), // Use the extracted parameters
                        strict: Some(strict),
                    };
                    function_objects.push(function_object);
                }
            }
            DirEntry::Dir(_) => {
                // Optionally handle subdirectories if needed
            }
        }
    }
    Ok(function_objects)
}

fn define_schema() -> Result<ResponseFormat, Box<dyn Error>> {
    let schema_file = ASSETS_DIR
        .get_file("assistant_instructions/schema.json")
        .expect("Failed to get assistant schema file")
        .contents_utf8()
        .expect("Failed to read assistant schema file");

    let json_schema: Value = serde_json::from_str(schema_file)?;
    let name = json_schema["name"].as_str().expect("Expected a String");
    let schema = json_schema["schema"].clone(); // This extracts the parameters part
    let strict = json_schema["strict"].as_bool().unwrap_or(true); // Defaults to true if not found
    let response_format = ResponseFormat::JsonSchema {
        json_schema: ResponseFormatJsonSchema {
            description: None,
            name: name.into(),
            schema: Some(schema),
            strict: Some(strict),
        },
    };
    Ok(response_format)
}

// Function to create the assistant with multiple function objects
pub async fn create_assistant(
    client: &Client<OpenAIConfig>,
    model: &str,
    name: &str,
) -> Result<AssistantObject, Box<dyn Error>> {
    // Load all FunctionObjects from the specified folder
    let function_objects = load_function_objects()?;
    let instructions = ASSETS_DIR
        .get_file("assistant_instructions/instructions.json")
        .expect("Failed to get assistant instructions file")
        .contents_utf8()
        .expect("Failed to read assistant instructions file");

    // Convert FunctionObjects to AssistantTools using the Into trait
    let assistant_tools = function_objects
        .into_iter()
        .map(Into::into) // Use the Into trait for conversion
        .collect::<Vec<AssistantTools>>();

    let response_format = match define_schema() {
        Ok(schema) => schema,
        Err(err) => return Err(err),
    };
    // Build the CreateAssistantRequestArgs with all the function objects
    let create_assistant_request = CreateAssistantRequestArgs::default()
        .name(name)
        .temperature(0.7)
        .instructions(instructions)
        .model(model)
        .response_format(AssistantsApiResponseFormatOption::Format(response_format))
        .tools(assistant_tools) // Pass the vector of AssistantTools
        .build()?;

    // Create the assistant
    let assistant = client.assistants().create(create_assistant_request).await?;
    Ok(assistant)
}

pub fn get_assistant_id(save_name: &PathBuf) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(save_name)?;

    // Read the file content into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Parse the JSON string into a serde_json::Value
    let json: Value = serde_json::from_str(&content)?;

    // Extract the "assistant_id" field
    if let Some(assistant_id) = json["assistant_id"].as_str() {
        Ok(assistant_id.to_string())
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Couldn't find assistant_id in the file",
        )))
    }
}

pub async fn delete_assistant(client: &Client<OpenAIConfig>, assistant_id: &str) {
    let _ = client.assistants().delete(assistant_id).await;
}

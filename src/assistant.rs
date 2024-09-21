use async_openai::types::{ResponseFormat, ResponseFormatJsonSchema};
use serde_json::Value;
use std::error::Error;
use std::fs::{self, File};
use std::io::Read;

use crate::save::SAVE_DIR;
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantObject, AssistantTools, AssistantsApiResponseFormatOption,
        CreateAssistantRequestArgs, FunctionObject, ListAssistantsResponse,
    },
    Client,
};

const INSTRUCTIONS: &str = include_str!("../assistant_instructions/instructions.json");
const SCHEMA: &str = include_str!("../assistant_instructions/schema.json");
const FUNCTIONS: &str = "/Users/Haller/Dev/Rust/projects/sharad_ratatui/assistant_functions";

fn load_function_objects() -> Result<Vec<FunctionObject>, Box<dyn Error>> {
    let folder_path = FUNCTIONS;
    let mut function_objects = Vec::new();

    // Read the folder
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();

        // Ensure the entry is a JSON file
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            // Read the file contents
            let mut file = File::open(&path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;

            // Parse the content as a JSON value
            let function_data: Value = serde_json::from_str(&content)?;

            // Extract relevant fields from the JSON object
            let name = function_data["name"].as_str().unwrap();
            let description = function_data["description"].as_str().unwrap();
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
    Ok(function_objects)
}

fn define_schema() -> Result<ResponseFormat, Box<dyn Error>> {
    let json_schema: Value = serde_json::from_str(SCHEMA)?;
    let name = json_schema["name"].as_str().unwrap();
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
    name: &str,
) -> Result<AssistantObject, Box<dyn Error>> {
    // Load all FunctionObjects from the specified folder
    let function_objects = load_function_objects()?;

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
        .instructions(INSTRUCTIONS)
        .model("gpt-4o-mini")
        .response_format(AssistantsApiResponseFormatOption::Format(response_format))
        .tools(assistant_tools) // Pass the vector of AssistantTools
        .build()?;

    // Create the assistant
    let assistant = client.assistants().create(create_assistant_request).await?;
    Ok(assistant)
}

// TODO: Handle error properly
pub fn get_assistant_id(save_name: &str) -> String {
    let file_path = format!("{}/{}.json", SAVE_DIR, save_name);
    let mut file = File::open(file_path).expect("Couldn't open the file");

    // Read the file content into a string
    let mut content = String::new();
    file.read_to_string(&mut content)
        .expect("Couldn't read the file");

    // Parse the JSON string into a serde_json::Value
    let json: Value = serde_json::from_str(&content).expect("Couldn't parse Json");

    // Extract the "assistant_id" field
    if let Some(assistant_id) = json["assistant_id"].as_str() {
        assistant_id.into()
    } else {
        panic!("No assistant id found");
    }
}

pub async fn delete_assistant(client: &Client<OpenAIConfig>, assistant_id: &str) {
    let _ = client.assistants().delete(assistant_id).await;
}

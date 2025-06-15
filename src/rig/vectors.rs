use std::path::PathBuf;

use crate::error::Result;
use pdfium_render::prelude::{Pdfium, PdfiumError};
use rig::{
    Embed,
    client::{EmbeddingsClient, ProviderClient},
    embeddings::EmbeddingsBuilder,
    providers::{openai, openai::EmbeddingModel, openai::TEXT_EMBEDDING_3_SMALL},
};
use rig_sqlite::{
    Column, ColumnValue, SqliteVectorIndex, SqliteVectorStore, SqliteVectorStoreTable,
};
use serde::{Deserialize, Serialize};
use sqlite_vec::sqlite3_vec_init;
use tokio_rusqlite::{Connection, ffi::sqlite3_auto_extension};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Document {
    id: String,
    content: String,
}

impl Embed for Document {
    fn embed(
        &self,
        embedder: &mut rig::embeddings::TextEmbedder,
    ) -> std::result::Result<(), rig::embeddings::EmbedError> {
        embedder.embed(self.content.clone());
        Ok(())
    }
}

impl SqliteVectorStoreTable for Document {
    fn name() -> &'static str {
        "documents"
    }

    fn schema() -> Vec<Column> {
        vec![
            Column::new("id", "TEXT PRIMARY KEY"),
            Column::new("content", "TEXT"),
        ]
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn column_values(&self) -> Vec<(&'static str, Box<dyn ColumnValue>)> {
        vec![
            ("id", Box::new(self.id.clone())),
            ("content", Box::new(self.content.clone())),
        ]
    }
}

pub async fn set_vector_store() -> Result<SqliteVectorIndex<EmbeddingModel, Document>> {
    let openai_client = openai::Client::from_env();
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }

    // Initialize SQLite connection
    let vector_present = dbg!(std::fs::exists(
        "/Volumes/Dock/Dev/Rust/projects/rig-rag-system-example/rag_system/openai_vector_store_complete.db"
    ))?;
    let conn = Connection::open(
        "/Volumes/Dock/Dev/Rust/projects/rig-rag-system-example/rag_system/openai_vector_store_complete.db",
    )
    .await?;

    // Create embedding model
    let embedding_model = openai_client.embedding_model(TEXT_EMBEDDING_3_SMALL);
    // Create vector store and index
    let vector_store = SqliteVectorStore::new(conn, &embedding_model).await?;

    if !vector_present {
        // Load PDFs using Pdfium
        let documents_dir = std::env::current_dir()?.join("documents");
        let pdf_content = pdf_extract(documents_dir.join("rules_5.pdf"))?;
        let rules = chunk_pdf(&pdf_content)?;

        println!("Successfully loaded and chunked PDF documents");

        // Create embeddings builder
        let mut builder = EmbeddingsBuilder::new(embedding_model.clone());
        for (page, chunk) in rules.clone().into_iter().enumerate() {
            for (chunk_nb, chunk_txt) in chunk.into_iter().enumerate() {
                builder = builder.document(Document {
                    id: format!("rules_p{}_{}", page, chunk_nb),
                    content: chunk_txt,
                })?;
            }
        }
        let embeddings = builder.build().await?;

        println!("Successfully generated embeddings");

        // Add embeddings to vector store
        vector_store.add_rows(embeddings).await?;

        println!("Successfully created vector store and index");
    }
    let index = vector_store.index(embedding_model);
    println!("Successfully indexed vector store");
    Ok(index)
}

fn pdf_extract(path: PathBuf) -> Result<Vec<String>> {
    let mut contents = Vec::new();
    Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(
            &std::env::var("PDFIUM_LIB_PATH").unwrap(),
        ))
        .unwrap(),
    )
    .load_pdf_from_file(&path, None)?
    .pages()
    .iter()
    .for_each(|p| contents.push(p.text().unwrap().all()));

    Ok(contents)
}

fn chunk_pdf(pdf_content: &Vec<String>) -> Result<Vec<Vec<String>>> {
    let mut chunks_vec = Vec::new();
    for page in pdf_content {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let chunk_size = 2000; // Approximately 2000 characters per chunk

        // Split content into words

        let words: Vec<&str> = page.split_whitespace().collect();
        for word in words {
            if current_chunk.len() + word.len() + 1 > chunk_size {
                // If adding the next word would exceed chunk size,
                // save current chunk and start a new one
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk.clear();
                }
            }
            current_chunk.push_str(word);
            current_chunk.push(' ');
        }

        // last chunk
        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }
        if !chunks.is_empty() {
            chunks_vec.push(chunks);
        }
    }

    Ok(chunks_vec)
}

use pocketflow_rs::ProcessState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RagState {
    // Offline states
    FileLoadedError,
    DocumentsLoaded,
    DocumentsChunked,
    ChunksEmbedded,
    IndexCreated,
    // Offline error states
    DocumentLoadError,
    ChunkingError,
    EmbeddingError,
    IndexCreationError,
    // Online states
    QueryEmbedded,
    DocumentsRetrieved,
    AnswerGenerated,
    // Online error states
    QueryEmbeddingError,
    RetrievalError,
    GenerationError,
    Default,
    QueryRewriteError,
}

impl ProcessState for RagState {
    fn is_default(&self) -> bool {
        matches!(self, RagState::Default)
    }

    fn to_condition(&self) -> String {
        match self {
            // Offline states
            RagState::FileLoadedError => "file_loaded_error".to_string(),
            RagState::DocumentsLoaded => "documents_loaded".to_string(),
            RagState::DocumentsChunked => "documents_chunked".to_string(),
            RagState::ChunksEmbedded => "chunks_embedded".to_string(),
            RagState::IndexCreated => "index_created".to_string(),
            // Offline error states
            RagState::DocumentLoadError => "document_load_error".to_string(),
            RagState::ChunkingError => "chunking_error".to_string(),
            RagState::EmbeddingError => "embedding_error".to_string(),
            RagState::IndexCreationError => "index_creation_error".to_string(),
            // Online states
            RagState::QueryEmbedded => "query_embedded".to_string(),
            RagState::DocumentsRetrieved => "documents_retrieved".to_string(),
            RagState::AnswerGenerated => "answer_generated".to_string(),
            // Online error states
            RagState::QueryEmbeddingError => "query_embedding_error".to_string(),
            RagState::RetrievalError => "retrieval_error".to_string(),
            RagState::GenerationError => "generation_error".to_string(),
            RagState::Default => "default".to_string(),
            RagState::QueryRewriteError => "query_rewrite_error".to_string(),
        }
    }
}

impl Default for RagState {
    fn default() -> Self {
        RagState::Default
    }
}

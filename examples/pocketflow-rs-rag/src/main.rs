use anyhow::Result;
use clap::{Parser, Subcommand};
use pocketflow_rs::utils::{text_chunking::ChunkingStrategy, vector_db::DistanceMetric};
use pocketflow_rs::{Context as FlowContext, build_flow};
use pocketflow_rs_rag::{
    QueryRewriteNode,
    nodes::{
        ChunkDocumentsNode, CreateIndexNode, EmbedDocumentsNode, EmbedQueryNode, FileLoaderNode,
        GenerateAnswerNode, RetrieveDocumentNode,
    },
    state::RagState,
};
use serde_json::json;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process documents offline
    Offline {
        /// Qdrant database URL
        #[arg(long, default_value = "http://localhost:6333")]
        db_url: String,

        /// Collection name in Qdrant
        #[arg(long, default_value = "documents")]
        collection: String,

        /// OpenAI API key
        #[arg(long)]
        api_key: String,

        /// Qdrant API key
        #[arg(long)]
        qdrant_api_key: Option<String>,

        /// OpenAI API endpoint
        #[arg(short, long, default_value = "https://api.openai.com/v1")]
        endpoint: String,

        /// Chunk size for document splitting
        #[arg(long, default_value = "1000")]
        chunk_size: usize,

        /// Overlap between chunks
        #[arg(long, default_value = "200")]
        overlap: usize,

        /// OpenAI model to use
        #[arg(long, default_value = "text-embedding-ada-002")]
        model: String,

        #[arg(long, default_value = "1024")]
        dimension: usize,

        /// Paths to document files
        #[arg(required = true)]
        files: Vec<String>,
    },
    /// Online processing: answer questions based on indexed documents
    Online {
        /// Qdrant database URL
        #[arg(long, default_value = "http://localhost:6333")]
        db_url: String,

        /// Collection name in Qdrant
        #[arg(long, default_value = "documents")]
        collection: String,

        /// OpenAI API key
        #[arg(long)]
        api_key: String,

        /// OpenAI API endpoint
        #[arg(long, default_value = "https://api.openai.com/v1")]
        endpoint: String,

        /// Number of documents to retrieve
        #[arg(short, long, default_value = "3")]
        k: usize,

        /// chat mode
        #[arg(long, default_value = "chat")]
        chat_mode: String,

        /// embedding dimension
        #[arg(long, default_value = "1024")]
        dimension: usize,

        /// Qdrant API key
        #[arg(long)]
        qdrant_api_key: Option<String>,

        /// Embedding model
        #[arg(long, default_value = "text-embedding-ada-002")]
        embedding_model: String,

        /// Question to answer
        #[arg(required = true)]
        query: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    FmtSubscriber::builder().with_max_level(Level::INFO).init();

    match cli.command {
        Commands::Offline {
            files,
            db_url,
            collection,
            api_key,
            qdrant_api_key,
            endpoint,
            chunk_size,
            overlap,
            model,
            dimension,
        } => {
            let file_loader = FileLoaderNode::new(files);
            let chunk_documents =
                ChunkDocumentsNode::new(chunk_size, overlap, ChunkingStrategy::Sentence);
            let embed_documents = EmbedDocumentsNode::new(
                api_key.clone(),
                endpoint.clone(),
                model.clone(),
                Some(dimension),
            );
            let create_index = CreateIndexNode::new(
                db_url,
                qdrant_api_key,
                collection,
                dimension,
                DistanceMetric::Cosine,
            )
            .await?;

            let flow = build_flow!(
                start: ("file_loader", file_loader),
                nodes: [
                    ("chunk_documents", chunk_documents),
                    ("embed_documents", embed_documents),
                    ("create_index", create_index)
                ],
                edges: [
                    ("file_loader", "chunk_documents", RagState::Default),
                    ("chunk_documents", "embed_documents", RagState::Default),
                    ("embed_documents", "create_index", RagState::Default)
                ]
            );

            flow.run(FlowContext::new()).await?;
        }
        Commands::Online {
            query,
            db_url,
            collection,
            api_key,
            endpoint,
            k,
            chat_mode,
            dimension,
            qdrant_api_key,
            embedding_model,
        } => {
            let mut context = FlowContext::new();
            context.set("user_query", json!(query.clone()));

            let query_rewrite_node =
                QueryRewriteNode::new(api_key.clone(), chat_mode.clone(), endpoint.clone());

            let embed_query_node = EmbedQueryNode::new(
                api_key.clone(),
                endpoint.clone(),
                embedding_model.clone(),
                Some(dimension),
            );

            let retrieve_node = RetrieveDocumentNode::new(
                db_url,
                qdrant_api_key,
                collection,
                dimension,
                DistanceMetric::Cosine,
                k,
            )
            .await?;

            let generate_node = GenerateAnswerNode::new(api_key, chat_mode, endpoint, query);

            // Build and execute online flow
            let flow = build_flow!(
                start: ("query_rewrite", query_rewrite_node),
                nodes: [
                    ("embed_query", embed_query_node),
                    ("retrieve", retrieve_node),
                    ("generate", generate_node)
                ],
                edges: [
                    ("query_rewrite", "embed_query", RagState::Default),
                    ("embed_query", "retrieve", RagState::Default),
                    ("retrieve", "generate", RagState::Default)
                ]
            );

            let result = flow.run(context).await?;

            termimad::print_text(result.as_str().unwrap());
        }
    }

    Ok(())
}

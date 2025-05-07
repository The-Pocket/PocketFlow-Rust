use std::env;

use anyhow::Result;
use duckdb::Connection;
use pocketflow_rs::{Context, build_flow};
use text2sql::flow::{ExecuteSQLNode, OpenAISQLGenerationNode, SchemaRetrievalNode};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let db_path = "ecommerce.duckdb";
    let conn = Connection::open(db_path)?;

    conn.execute(&format!(
        "CREATE TABLE IF NOT EXISTS customers AS SELECT * FROM read_csv_auto('{}', AUTO_DETECT=TRUE)",
        "example_data/customers.csv"
    ), [])?;

    conn.execute(&format!(
        "CREATE TABLE IF NOT EXISTS orders AS SELECT * FROM read_csv_auto('{}', AUTO_DETECT=TRUE)",
        "example_data/orders.csv"
    ), [])?;

    println!("please input your query using natural language?");
    let mut user_query = String::new();
    std::io::stdin().read_line(&mut user_query)?;
    user_query = user_query.trim().to_string();

    let schema_retrieval = SchemaRetrievalNode::new(db_path.to_string());
    let openai_sql_gen =
        OpenAISQLGenerationNode::new(env::var("DASH_SCOPE_API_KEY").unwrap(), user_query);
    let execute_sql = ExecuteSQLNode::new(db_path.to_string());

    let flow = build_flow! (
        start: ("start", schema_retrieval),
        nodes: [
            ("generate_sql", openai_sql_gen),
            ("execute_sql", execute_sql),
        ],
        edges: [
            ("start", "generate_sql", text2sql::flow::SqlExecutorState::Default),
            ("generate_sql", "execute_sql", text2sql::flow::SqlExecutorState::Default)
        ]
    );
    let context = Context::new();

    let result = flow.run(context).await?;
    println!("result: {:?}", result);

    Ok(())
}

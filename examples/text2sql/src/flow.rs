use anyhow::{Context as AnyhowContext, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use duckdb::types::ValueRef;
use duckdb::{Connection, Result as DuckResult};
use openai_api_rust::chat::*;
use openai_api_rust::*;
use pocketflow_rs::{Context, Node, ProcessResult, ProcessState};
use serde_json::{Value, json};
use tracing::{error, info};

#[derive(Debug, Clone, PartialEq)]
pub enum SqlExecutorState {
    SchemaRetrieved,
    SqlGenerated,
    SqlExecuted,
    Default,
}

impl ProcessState for SqlExecutorState {
    fn is_default(&self) -> bool {
        matches!(self, SqlExecutorState::Default)
    }

    fn to_condition(&self) -> String {
        match self {
            SqlExecutorState::SchemaRetrieved => "schema_retrieved".to_string(),
            SqlExecutorState::SqlGenerated => "sql_generated".to_string(),
            SqlExecutorState::SqlExecuted => "sql_executed".to_string(),
            SqlExecutorState::Default => "default".to_string(),
        }
    }
}

impl Default for SqlExecutorState {
    fn default() -> Self {
        SqlExecutorState::Default
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("NodeExecution: {0}")]
    NodeExecution(String),
}

pub struct SchemaRetrievalNode {
    db_path: String,
}

impl SchemaRetrievalNode {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }
}

#[async_trait]
impl Node for SchemaRetrievalNode {
    type State = SqlExecutorState;

    #[allow(unused_variables)]
    async fn execute(&self, context: &Context) -> Result<Value> {
        info!("Exec SchemaRetrievalNode");
        let conn = Connection::open(&self.db_path)?;

        let query = "SELECT table_name FROM information_schema.tables WHERE table_schema='main'";
        let mut stmt = conn.prepare(query)?;
        let tables = stmt.query_map([], |row| Ok(row.get(0)?));

        let tables = tables.context("获取表名失败")?;

        let mut schema = serde_json::Map::new();
        for table in tables {
            let table_name = table?;
            let query = format!(
                "SELECT column_name, data_type, is_nullable, column_default
                 FROM information_schema.columns
                 WHERE table_name='{}' AND table_schema='main'",
                table_name
            );

            let mut stmt = conn.prepare(&query)?;
            let columns = stmt
                .query_map([], |row| {
                    Ok(json!({
                        "name": row.get::<_, String>(0)?,
                        "type": row.get::<_, String>(1)?,
                        "nullable": row.get::<_, String>(2)? == "YES",
                        "default_value": row.get::<_, Option<String>>(3)?,
                    }))
                })?
                .collect::<DuckResult<Vec<Value>>>()
                .context("Get Column Info Failed")?;

            schema.insert(table_name, Value::Array(columns));
        }
        info!("Get Result Final");

        Ok(Value::Object(schema))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<SqlExecutorState>> {
        context.set("result", result.as_ref().unwrap().clone());
        Ok(ProcessResult::new(
            SqlExecutorState::SchemaRetrieved,
            "schema_retrieved".to_string(),
        ))
    }
}

pub struct OpenAISQLGenerationNode {
    api_key: String,
    user_query: String,
}

impl OpenAISQLGenerationNode {
    pub fn new(api_key: String, user_query: String) -> Self {
        Self {
            api_key,
            user_query,
        }
    }
}

pub fn print_table(headers: &[String], data: &[Vec<String>]) {
    if headers.is_empty() {
        println!("Query returned no columns.");
        return;
    }

    // Calculate column widths based on headers and data
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in data {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Print Header
    let header_line = headers
        .iter()
        .zip(&widths)
        .map(|(h, w)| format!("{:<width$}", h, width = w))
        .collect::<Vec<_>>()
        .join(" | ");
    println!("\n{}", header_line);

    // Print Separator
    let separator_line = widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("-+-");
    println!("{}", separator_line);

    // Print Data Rows
    if data.is_empty() {
        println!("(No rows returned)");
    } else {
        for row in data {
            let row_line = row
                .iter()
                .zip(&widths)
                .map(|(cell, w)| format!("{:<width$}", cell, width = w))
                .collect::<Vec<_>>()
                .join(" | ");
            println!("{}", row_line);
        }
    }
}

#[async_trait]
impl Node for OpenAISQLGenerationNode {
    type State = SqlExecutorState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let schema = context.get("result").ok_or_else(|| {
            WorkflowError::NodeExecution("Failed to get database schema".to_string())
        })?;

        let system_prompt = "You are a SQL expert. Based on the provided database schema and user query, generate the correct SQL query. Only return the SQL query, do not include any explanation or other text. The condition content uses English, you can choose to query some fields first, then make a general query.";

        let schema_json =
            serde_json::to_string_pretty(schema).context("Failed to serialize database schema")?;

        let user_prompt = format!(
            "database schema:\n{}\n\nuser query:\n{}\n\nPlease generate a SQL query to answer this question.",
            schema_json, self.user_query
        );

        let auth = Auth::new(self.api_key.as_str());
        let openai = OpenAI::new(auth, "https://dashscope.aliyuncs.com/compatible-mode/v1/");
        let body = ChatBody {
            model: "qwen-plus".to_string(),
            max_tokens: Some(1024),
            temperature: Some(0.8_f32),
            top_p: Some(0_f32),
            n: Some(1),
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            messages: vec![
                Message {
                    role: Role::System,
                    content: system_prompt.to_string(),
                },
                Message {
                    role: Role::User,
                    content: user_prompt,
                },
            ],
        };
        let rs = openai.chat_completion_create(&body);
        if rs.is_err() {
            error!("OpenAI Error {}", rs.as_ref().err().unwrap().to_string());
        }
        let choice = rs.unwrap().choices;
        let message = &choice[0].message.as_ref().unwrap();

        let sql = message.content.clone();

        println!("生成的SQL查询: {}", sql);

        Ok(Value::String(sql))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<SqlExecutorState>> {
        context.set("result", result.as_ref().unwrap().clone());
        Ok(ProcessResult::new(
            SqlExecutorState::SqlGenerated,
            "sql_generated".to_string(),
        ))
    }
}

pub struct ExecuteSQLNode {
    db_path: String,
}

impl ExecuteSQLNode {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }
}

#[async_trait]
impl Node for ExecuteSQLNode {
    type State = SqlExecutorState;

    async fn execute(&self, context: &Context) -> Result<Value> {
        let conn = Connection::open(&self.db_path)?;

        let sql = context
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                WorkflowError::NodeExecution("SQL query not found in context".to_string())
            })?;

        info!("ExecuteSQLNode: Get Sql: {}", sql);

        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query([])?;

        let mut headers = Vec::new();
        let mut data_rows = Vec::new();

        if let Some(first_row) = rows.next()? {
            // Get column names from the first row
            headers = first_row.as_ref().column_names();
            let column_count = headers.len();

            // Process first row
            let mut row_values = Vec::with_capacity(column_count);
            for i in 0..column_count {
                let value_ref = first_row.get_ref(i)?;
                let string_value = match value_ref {
                    ValueRef::Null => "NULL".to_string(),
                    ValueRef::Boolean(b) => b.to_string(),
                    ValueRef::TinyInt(i) => i.to_string(),
                    ValueRef::SmallInt(i) => i.to_string(),
                    ValueRef::Int(i) => i.to_string(),
                    ValueRef::BigInt(i) => i.to_string(),
                    ValueRef::Float(f) => f.to_string(),
                    ValueRef::Double(d) => d.to_string(),
                    ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).to_string(),
                    ValueRef::Blob(_) => "[BLOB]".to_string(),
                    ValueRef::Date32(d) => {
                        let date = NaiveDate::from_num_days_from_ce_opt(d as i32 + 719163).unwrap();
                        date.format("%Y-%m-%d").to_string()
                    }
                    _ => format!("Unsupported: {:?}", value_ref),
                };
                row_values.push(string_value);
            }
            data_rows.push(row_values);

            // Process remaining rows
            while let Some(row) = rows.next()? {
                let mut row_values = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    let value_ref = row.get_ref(i)?;
                    let string_value = match value_ref {
                        ValueRef::Null => "NULL".to_string(),
                        ValueRef::Boolean(b) => b.to_string(),
                        ValueRef::TinyInt(i) => i.to_string(),
                        ValueRef::SmallInt(i) => i.to_string(),
                        ValueRef::Int(i) => i.to_string(),
                        ValueRef::BigInt(i) => i.to_string(),
                        ValueRef::Float(f) => f.to_string(),
                        ValueRef::Double(d) => d.to_string(),
                        ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).to_string(),
                        ValueRef::Blob(_) => "[BLOB]".to_string(),
                        ValueRef::Date32(d) => {
                            let date =
                                NaiveDate::from_num_days_from_ce_opt(d as i32 + 719163).unwrap();
                            date.format("%Y-%m-%d").to_string()
                        }
                        _ => format!("Unsupported: {:?}", value_ref),
                    };
                    row_values.push(string_value);
                }
                data_rows.push(row_values);
            }
        }

        print_table(&headers, &data_rows);

        Ok(json!({
            "columns": headers,
            "data": data_rows
        }))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &Result<Value>,
    ) -> Result<ProcessResult<SqlExecutorState>> {
        context.set("result", result.as_ref().unwrap().clone());
        Ok(ProcessResult::new(
            SqlExecutorState::SqlExecuted,
            "sql_executed".to_string(),
        ))
    }
}

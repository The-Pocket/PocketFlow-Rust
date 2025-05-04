use anyhow::{Context as AnyhowContext, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use duckdb::types::ValueRef;
use duckdb::{Connection, Result as DuckResult};
use openai_api_rust::chat::*;
use openai_api_rust::*;
use pocketflow_rs::{Context, Flow, Node, build_flow};
use serde_json::{Value, json};
use std::env;
use tracing::{error, info, debug};
use tracing_subscriber;

#[derive(Debug, thiserror::Error)]
enum WorkflowError {
    #[error("节点执行错误: {0}")]
    NodeExecution(String),
}

struct SchemaRetrievalNode {
    db_path: String,
}

impl SchemaRetrievalNode {
    fn new(db_path: String) -> Self {
        Self { db_path }
    }
}

#[async_trait]
impl Node for SchemaRetrievalNode {
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
                .context("获取列信息失败")?;

            schema.insert(table_name, Value::Array(columns));
        }
        info!("Get Result Final");

        Ok(Value::Object(schema))
    }

    async fn post_process(
        &self,
        context: &mut Context,
        result: &serde_json::Value,
    ) -> Result<&str> {
        context.set("result", result.clone());
        Ok("default")
    }
}

struct OpenAISQLGenerationNode {
    api_key: String,
    user_query: String,
}

impl OpenAISQLGenerationNode {
    fn new(api_key: String, user_query: String) -> Self {
        Self {
            api_key,
            user_query,
        }
    }
}

fn print_table(headers: &[String], data: &[Vec<String>]) {
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
    async fn execute(&self, context: &Context) -> Result<Value> {
        let schema = context
            .get("result")
            .ok_or_else(|| WorkflowError::NodeExecution("无法获取数据库模式".to_string()))?;

        let system_prompt = "你是一个SQL专家。根据提供的数据库模式和用户查询，生成正确的SQL查询。仅返回SQL查询，不要包含任何解释或其他文本。条件内容使用英文, 你可以选择先查询某些字段，再做整体查询";

        let schema_json = serde_json::to_string_pretty(schema).context("无法序列化数据库模式")?;

        let user_prompt = format!(
            "数据库模式:\n{}\n\n用户查询: {}\n\n请生成一个SQL查询来回答这个问题。",
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

}

struct ExecuteSQLNode {
    db_path: String,
}

impl ExecuteSQLNode {
    fn new(db_path: String) -> Self {
        Self { db_path }
    }
}

#[async_trait]
impl Node for ExecuteSQLNode {
    async fn execute(&self, context: &Context) -> Result<Value> {
        let conn = Connection::open(&self.db_path)?;

        let sql = context
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WorkflowError::NodeExecution("上下文中没有找到SQL查询".to_string()))?;

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
                    },
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
                            let date = NaiveDate::from_num_days_from_ce_opt(d as i32 + 719163).unwrap();
                            date.format("%Y-%m-%d").to_string()
                        },
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

}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("启动Text-to-SQL工作流");
    let api_key = env::var("OPENAI_API_KEY").context("请设置OPENAI_API_KEY环境变量")?;

    let db_path = "ecommerce.duckdb";
    info!("打开数据库连接: {}", db_path);
    let db_path = "ecommerce.duckdb";
    let conn = Connection::open(db_path)?;

    // 通过导入CSV创建示例表
    conn.execute(&format!(
        "CREATE TABLE IF NOT EXISTS customers AS SELECT * FROM read_csv_auto('{}', AUTO_DETECT=TRUE)",
        "example_data/customers.csv"
    ), [])?;

    conn.execute(&format!(
        "CREATE TABLE IF NOT EXISTS orders AS SELECT * FROM read_csv_auto('{}', AUTO_DETECT=TRUE)",
        "example_data/orders.csv"
    ), [])?;

    // 提示用户输入查询
    println!("请输入您的自然语言查询（例如：'找出所有来自纽约的客户'）：");
    let mut user_query = String::new();
    std::io::stdin().read_line(&mut user_query)?;
    user_query = user_query.trim().to_string();

    // 创建Flow节点
    let schema_retrieval = SchemaRetrievalNode::new(db_path.to_string());
    let openai_sql_gen = OpenAISQLGenerationNode::new(api_key, user_query);
    let execute_sql = ExecuteSQLNode::new(db_path.to_string());

    // 创建流程
    let flow = build_flow! (
        start: ("start", schema_retrieval),
        nodes: [
            ("generate_sql", openai_sql_gen),
            ("execute_sql", execute_sql),
        ],
        edges: [
            ("start", "generate_sql"),
            ("generate_sql", "execute_sql"),
        ]
    );
    // 创建上下文
    let context = Context::new();

    // 执行流程
    println!("正在执行Text-to-SQL工作流...");
    let result = flow.run(context).await?;
    println!("流程执行完成！");
    println!("结果：");
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

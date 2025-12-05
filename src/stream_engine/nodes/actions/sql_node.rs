use crate::stream_engine::StreamNode;
use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;
use sqlx::AnyPool;
use sqlx::any::AnyRow;
use sqlx::{Row, Column};

#[derive(Clone, Debug)]
pub struct SqlNode {
    connection_string: String,
    query: String,
    parameters_template: Option<String>,
}

impl SqlNode {
    pub fn new(config: Value) -> Self {
        let connection_string = config.get("connection_string")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
            
        let query = config.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let parameters_template = config.get("parameters")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Self {
            connection_string,
            query,
            parameters_template,
        }
    }

    async fn execute_query(&self, pool: &AnyPool, query_str: &str, params: Vec<Value>) -> anyhow::Result<Vec<Value>> {
        let mut query_builder = sqlx::query(query_str);
        
        for param in params {
            match param {
                Value::Null => query_builder = query_builder.bind(Option::<String>::None),
                Value::Bool(b) => query_builder = query_builder.bind(b),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        query_builder = query_builder.bind(i);
                    } else if let Some(f) = n.as_f64() {
                         query_builder = query_builder.bind(f);
                    }
                },
                Value::String(s) => query_builder = query_builder.bind(s),
                Value::Array(_) | Value::Object(_) => {
                    // For complex types, bind as string? Or fail?
                    // Let's bind as string for now
                    query_builder = query_builder.bind(param.to_string());
                }
            }
        }

        let rows = query_builder.fetch_all(pool).await?;
        let mut results = Vec::new();

        for row in rows {
            let mut row_json = serde_json::Map::new();
            for col in row.columns() {
                let col_name = col.name();
                
                // Try to decode based on type info if possible, or try common types
                // AnyRow handling is a bit specific.
                // We'll use a helper to map to Value
                let val = map_row_value(&row, col.ordinal());
                row_json.insert(col_name.to_string(), val);
            }
            results.push(Value::Object(row_json));
        }

        Ok(results)
    }
}

fn map_row_value(row: &AnyRow, ordinal: usize) -> Value {
    // This is tricky with AnyRow because we don't know the types statically.
    // We have to try decoding into common types.
    
    // Try string first as it covers most
    if let Ok(s) = row.try_get::<String, _>(ordinal) {
        return Value::String(s);
    }
    if let Ok(i) = row.try_get::<i64, _>(ordinal) {
        return Value::Number(serde_json::Number::from(i));
    }
    if let Ok(f) = row.try_get::<f64, _>(ordinal) {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }
    if let Ok(b) = row.try_get::<bool, _>(ordinal) {
        return Value::Bool(b);
    }
    
    // Fallback logic could be better, but AnyRow makes this hard without extensive matching on TypeInfo
    Value::Null
}

#[async_trait]
impl StreamNode for SqlNode {
    async fn run(
        &self,
        mut inputs: Vec<mpsc::Receiver<Value>>,
        outputs: Vec<mpsc::Sender<Value>>,
    ) -> anyhow::Result<()> {
        
        if inputs.is_empty() {
             return Ok(());
        }

        let mut rx = inputs.remove(0);
        let tx = if !outputs.is_empty() {
            outputs[0].clone()
        } else {
             return Ok(()); // No output, just execute?
        };

        // Connect to DB once? Or per run? 
        // Install drivers for AnyPool (idempotent)
        sqlx::any::install_default_drivers();
        
        let pool = AnyPool::connect(&self.connection_string).await?;

        while let Some(data) = rx.recv().await {
            // Render Parameters if template
            let mut params = Vec::new();
            if let Some(tmpl_str) = &self.parameters_template {
                 let env = crate::stream_engine::expressions::create_environment();
                 // Render the whole JSON string first
                 match env.render_str(tmpl_str, &data) {
                     Ok(rendered) => {
                         if let Ok(parsed) = serde_json::from_str::<Value>(&rendered) {
                             if let Some(arr) = parsed.as_array() {
                                 params = arr.clone();
                             }
                         } else {
                             // Maybe it wasn't valid JSON, try parsing the un-rendered if it was just static?
                             // User is expected to provide valid JSON array string
                         }
                     },
                     Err(e) => {
                         eprintln!("SqlNode Param Render Error: {}", e);
                         continue;
                     }
                 }
            }

            match self.execute_query(&pool, &self.query, params).await {
                Ok(rows) => {
                    let output = serde_json::json!({
                        "rows": rows,
                        "original_input": data
                    });
                     if tx.send(output).await.is_err() {
                        break;
                    }
                },
                Err(e) => {
                    eprintln!("SqlNode Query Error: {}", e);
                    // Add error handling policy here later
                }
            }
        }
        
        pool.close().await;

        Ok(())
    }
}

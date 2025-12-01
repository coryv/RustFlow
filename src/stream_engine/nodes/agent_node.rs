use crate::stream_engine::StreamNode;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc::{Receiver, Sender};
use anyhow::{Result, anyhow, Context};
use minijinja::Environment;
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub api_key: Option<String>, // Can be overridden by input or env
    pub credential_id: Option<String>, // UUID of stored credential
    pub api_base: Option<String>, // Defaults to OpenAI
    pub json_schema: Option<Value>, // Optional JSON Schema for validation
}

impl AgentNode {
    async fn call_llm(&self, system: &str, user: &str, api_key: &str) -> Result<Value> {
        let client = Client::new();
        let base_url = self.api_base.as_deref().unwrap_or("https://api.openai.com/v1");
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let messages = vec![
            json!({ "role": "system", "content": system }),
            json!({ "role": "user", "content": user }),
        ];

        let mut body = json!({
            "model": self.model,
            "messages": messages,
        });

        // If JSON schema is provided, enforce structured output
        if self.json_schema.is_some() {
            body["response_format"] = json!({ "type": "json_object" });
        }

        let res = client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            return Err(anyhow!("LLM API Error: {}", error_text));
        }

        let res_json: Value = res.json().await?;
        let content = res_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("No content in LLM response"))?;

        // Parse content as JSON if schema is present (or if response_format was json_object)
        if self.json_schema.is_some() {
            let parsed: Value = serde_json::from_str(content)
                .context("LLM response was not valid JSON")?;
            
            // TODO: Validate against self.json_schema using a library like jsonschema
            // For now, we just ensure it parses as JSON.
            
            Ok(parsed)
        } else {
            Ok(Value::String(content.to_string()))
        }
    }
}

#[async_trait]
impl StreamNode for AgentNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, mut outputs: Vec<Sender<Value>>) -> Result<()> {
        let output = outputs.remove(0);
        let mut rx = inputs.remove(0);
        let mut env = Environment::new();

        // Get API Key from config or env
        // Note: If credential_id was used, the Loader should have injected the decrypted key into self.api_key
        let api_key = self.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow!("No API Key provided for AgentNode"))?;

        while let Some(input) = rx.recv().await {
            // Render Prompts
            env.add_template("system", &self.system_prompt)?;
            env.add_template("user", &self.user_prompt)?;

            let system_rendered = env.get_template("system")?.render(&input)?;
            let user_rendered = env.get_template("user")?.render(&input)?;

            // Append schema instruction to system prompt if needed
            let final_system = if let Some(schema) = &self.json_schema {
                format!("{}\n\nYou must respond with valid JSON matching this schema:\n{}", system_rendered, serde_json::to_string_pretty(schema)?)
            } else {
                system_rendered
            };

            // Call LLM
            let result = self.call_llm(&final_system, &user_rendered, &api_key).await;

            match result {
                Ok(val) => {
                    if output.send(val).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("AgentNode Error: {:?}", e);
                    // Optionally emit error object?
                }
            }
        }

        Ok(())
    }
}

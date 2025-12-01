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
    pub api_key: Option<String>,
    pub credential_id: Option<String>,
    pub api_base: Option<String>,
    pub json_schema: Option<Value>,
    #[serde(default)]
    pub provider: String, // "openai" or "gemini"
}

impl AgentNode {
    async fn call_llm(&self, system: &str, user: &str, api_key: &str) -> Result<Value> {
        let client = Client::new();
        
        if self.provider == "gemini" {
             let base_url = self.api_base.as_deref().unwrap_or("https://generativelanguage.googleapis.com/v1beta");
             let url = format!("{}/models/{}:generateContent?key={}", base_url.trim_end_matches('/'), self.model, api_key);

             // Gemini API format
             let body = json!({
                 "contents": [{
                     "parts": [
                         { "text": system }, // Gemini doesn't have system role in same way, usually passed as context or just text
                         { "text": user }
                     ]
                 }],
                 "generationConfig": {
                     "responseMimeType": if self.json_schema.is_some() { "application/json" } else { "text/plain" }
                 }
             });

             let res = client.post(&url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .context("Failed to send request to Gemini API")?;

            if !res.status().is_success() {
                let error_text = res.text().await?;
                return Err(anyhow!("Gemini API Error: {}", error_text));
            }

            let res_json: Value = res.json().await?;
            // Extract content from Gemini response
            let content = res_json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .ok_or_else(|| anyhow!("No content in Gemini response"))?;

             if self.json_schema.is_some() {
                let parsed: Value = serde_json::from_str(content)
                    .context("Gemini response was not valid JSON")?;
                Ok(parsed)
            } else {
                Ok(Value::String(content.to_string()))
            }

        } else {
            // OpenAI (Default)
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

            if self.json_schema.is_some() {
                let parsed: Value = serde_json::from_str(content)
                    .context("LLM response was not valid JSON")?;
                Ok(parsed)
            } else {
                Ok(Value::String(content.to_string()))
            }
        }
    }
}

#[async_trait]
impl StreamNode for AgentNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, mut outputs: Vec<Sender<Value>>) -> Result<()> {
        let output = outputs.remove(0);
        let mut rx = inputs.remove(0);
        let mut env = Environment::new();

        while let Some(input) = rx.recv().await {
            // Get API Key from config or env
            let api_key = self.api_key.clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| anyhow!("No API Key provided for AgentNode"));

            let api_key = match api_key {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("AgentNode Error: {:?}", e);
                    continue;
                }
            };
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

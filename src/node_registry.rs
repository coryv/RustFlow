use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeProperty {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub property_type: String, // text, number, select, json, code, boolean
    pub options: Option<Vec<String>>, // For select
    pub default: Option<String>,
    pub required: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeType {
    pub id: String,
    pub label: String,
    pub category: String, // Trigger, Action, Logic, Integration
    pub description: Option<String>,
    pub properties: Vec<NodeProperty>,
}

pub fn get_node_registry() -> Vec<NodeType> {
    let mut nodes = vec![
        // --- Triggers ---
        NodeType {
            id: "manual_trigger".to_string(),
            label: "Manual Trigger".to_string(),
            category: "Trigger".to_string(),
            description: Some("Manually start the workflow".to_string()),
            properties: vec![],
        },
        NodeType {
            id: "time_trigger".to_string(),
            label: "Time Trigger".to_string(),
            category: "Trigger".to_string(),
            description: Some("Run workflow on a schedule".to_string()),
            properties: vec![
                NodeProperty {
                    name: "cron".to_string(),
                    label: "Cron Expression".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("* * * * *".to_string()),
                    required: true,
                },
            ],
        },
        NodeType {
            id: "webhook_trigger".to_string(),
            label: "Webhook".to_string(),
            category: "Trigger".to_string(),
            description: Some("Start workflow via HTTP request".to_string()),
            properties: vec![
                NodeProperty {
                    name: "path".to_string(),
                    label: "Path".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("/".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "method".to_string(),
                    label: "Method".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()]),
                    default: Some("POST".to_string()),
                    required: true,
                },
            ],
        },

        // --- Actions ---
        NodeType {
            id: "http_request".to_string(),
            label: "HTTP Request".to_string(),
            category: "Action".to_string(),
            description: Some("Make an HTTP request".to_string()),
            properties: vec![
                NodeProperty {
                    name: "method".to_string(),
                    label: "Method".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()]),
                    default: Some("GET".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "url".to_string(),
                    label: "URL".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                },
                NodeProperty {
                    name: "headers".to_string(),
                    label: "Headers".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: Some("{}".to_string()),
                    required: false,
                },
                NodeProperty {
                    name: "body".to_string(),
                    label: "Body".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: None,
                    required: false,
                },
            ],
        },
        NodeType {
            id: "set_data".to_string(),
            label: "Set Data".to_string(),
            category: "Action".to_string(),
            description: Some("Set workflow data".to_string()),
            properties: vec![
                NodeProperty {
                    name: "json".to_string(), // Mapped to 'data' in UI logic or handled specially
                    label: "JSON Data".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: Some("{}".to_string()),
                    required: true,
                },
            ],
        },
        NodeType {
            id: "code".to_string(),
            label: "Code".to_string(),
            category: "Action".to_string(),
            description: Some("Run JavaScript code".to_string()),
            properties: vec![
                NodeProperty {
                    name: "code".to_string(),
                    label: "Code".to_string(),
                    property_type: "code".to_string(),
                    options: None,
                    default: Some("return {};".to_string()),
                    required: true,
                },
            ],
        },
        NodeType {
            id: "console_output".to_string(),
            label: "Console Log".to_string(),
            category: "Action".to_string(),
            description: Some("Log data to console".to_string()),
            properties: vec![],
        },
        NodeType {
            id: "delay".to_string(),
            label: "Delay".to_string(),
            category: "Action".to_string(),
            description: Some("Pause execution for a duration".to_string()),
            properties: vec![
                NodeProperty {
                    name: "duration_ms".to_string(),
                    label: "Duration (ms)".to_string(),
                    property_type: "number".to_string(),
                    options: None,
                    default: Some("1000".to_string()),
                    required: true,
                },
            ],
        },

        // --- Logic ---
        NodeType {
            id: "router".to_string(),
            label: "Router".to_string(),
            category: "Logic".to_string(),
            description: Some("Route based on condition".to_string()),
            properties: vec![
                NodeProperty {
                    name: "key".to_string(),
                    label: "Key to check".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("id".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "value".to_string(),
                    label: "Value to match".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                },
            ],
        },

        // --- AI ---
        NodeType {
            id: "agent".to_string(),
            label: "AI Agent".to_string(),
            category: "AI".to_string(),
            description: Some("LLM Agent".to_string()),
            properties: vec![
                NodeProperty {
                    name: "system_prompt".to_string(),
                    label: "System Prompt".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("You are a helpful assistant.".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "user_prompt".to_string(),
                    label: "User Prompt".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                },
                NodeProperty {
                    name: "json_schema".to_string(),
                    label: "JSON Schema".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: None,
                    required: false,
                },
            ],
        },
        NodeType {
            id: "openai_model".to_string(),
            label: "OpenAI Model".to_string(),
            category: "Models".to_string(),
            description: Some("GPT-4 / GPT-3.5".to_string()),
            properties: vec![
                NodeProperty {
                    name: "model".to_string(),
                    label: "Model Name".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("gpt-4o".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "api_key".to_string(),
                    label: "API Key".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                },
            ],
        },
        NodeType {
            id: "gemini_model".to_string(),
            label: "Gemini Model".to_string(),
            category: "Models".to_string(),
            description: Some("Gemini Pro / Flash".to_string()),
            properties: vec![
                NodeProperty {
                    name: "model".to_string(),
                    label: "Model Name".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("gemini-1.5-flash".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "api_key".to_string(),
                    label: "API Key".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                },
            ],
        },

        // --- Data Processing ---
        NodeType {
            id: "html_extract".to_string(),
            label: "HTML Extract".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Extract data from HTML".to_string()),
            properties: vec![
                NodeProperty {
                    name: "selector".to_string(),
                    label: "CSS Selector".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("body".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "mode".to_string(),
                    label: "Mode".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["text".to_string(), "html".to_string(), "attribute".to_string()]),
                    default: Some("text".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "attribute".to_string(),
                    label: "Attribute Name".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                },
            ],
        },
        NodeType {
            id: "join".to_string(),
            label: "Join".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Join two streams".to_string()),
            properties: vec![
                NodeProperty {
                    name: "type".to_string(),
                    label: "Join Type".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["index".to_string(), "key".to_string()]),
                    default: Some("index".to_string()),
                    required: true,
                },
                NodeProperty {
                    name: "key".to_string(),
                    label: "Join Key".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("id".to_string()),
                    required: false,
                },
                NodeProperty {
                    name: "right_key".to_string(),
                    label: "Right Key (Optional)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                },
            ],
        },
        NodeType {
            id: "union".to_string(),
            label: "Union".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Merge multiple streams".to_string()),
            properties: vec![
                NodeProperty {
                    name: "mode".to_string(),
                    label: "Mode".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["interleaved".to_string(), "sequential".to_string()]),
                    default: Some("interleaved".to_string()),
                    required: true,
                },
            ],
        },
        NodeType {
            id: "file_source".to_string(),
            label: "File Source".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Read from file".to_string()),
            properties: vec![
                NodeProperty {
                    name: "path".to_string(),
                    label: "File Path".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                },
            ],
        },
    ];

    // Append generated integration nodes
    let mut integrations = crate::integrations::get_integration_node_definitions();
    nodes.append(&mut integrations);

    nodes
}


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
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeType {
    pub id: String,
    pub label: String,
    pub category: String, // Trigger, Action, Logic, Integration
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub properties: Vec<NodeProperty>,
    #[serde(default)]
    pub outputs: Vec<String>, // List of named outputs. If empty, assumes single default output.
}

pub fn get_node_registry() -> Vec<NodeType> {
    let mut nodes = Vec::new();
    nodes.extend(get_trigger_nodes());
    nodes.extend(get_action_nodes());
    nodes.extend(get_logic_nodes());
    nodes.extend(get_ai_nodes());
    nodes.extend(get_model_nodes());
    nodes.extend(get_data_processing_nodes());

    // Append generated integration nodes
    let mut integrations = crate::integrations::get_integration_node_definitions();
    nodes.append(&mut integrations);

    nodes
}

fn get_trigger_nodes() -> Vec<NodeType> {
    vec![
        NodeType {
            id: "manual_trigger".to_string(),
            label: "Manual Trigger".to_string(),
            category: "Trigger".to_string(),
            description: Some("Manually start the workflow".to_string()),
            documentation: None,
            properties: vec![],
            outputs: vec![],
        },
        NodeType {
            id: "time_trigger".to_string(),
            label: "Time Trigger".to_string(),
            category: "Trigger".to_string(),
            description: Some("Run workflow on a schedule".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "cron".to_string(),
                    label: "Cron Expression".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("* * * * *".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "webhook_trigger".to_string(),
            label: "Webhook".to_string(),
            category: "Trigger".to_string(),
            description: Some("Start workflow via HTTP request".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "path".to_string(),
                    label: "Path".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("/".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "method".to_string(),
                    label: "Method".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()]),
                    default: Some("POST".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
    ]
}

fn get_action_nodes() -> Vec<NodeType> {
    vec![
        NodeType {
            id: "http_request".to_string(),
            label: "HTTP Request".to_string(),
            category: "Action".to_string(),
            description: Some("Make an HTTP request".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "method".to_string(),
                    label: "Method".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()]),
                    default: Some("GET".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "url".to_string(),
                    label: "URL".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "headers".to_string(),
                    label: "Headers".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: Some("{}".to_string()),
                    required: false,
                    json_schema: None,
                },
                NodeProperty {
                    name: "body".to_string(),
                    label: "Body".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: None,
                    required: false,
                    json_schema: None,
                },
                NodeProperty {
                    name: "retry_count".to_string(),
                    label: "Retry Count".to_string(),
                    property_type: "number".to_string(),
                    options: None,
                    default: Some("0".to_string()),
                    required: false,
                    json_schema: None,
                },
                NodeProperty {
                    name: "retry_delay_ms".to_string(),
                    label: "Retry Delay (ms)".to_string(),
                    property_type: "number".to_string(),
                    options: None,
                    default: Some("0".to_string()),
                    required: false,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "set_data".to_string(),
            label: "Set Data".to_string(),
            category: "Action".to_string(),
            description: Some("Set workflow data".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "json".to_string(), // Mapped to 'data' in UI logic or handled specially
                    label: "JSON Data".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: Some("{}".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "code".to_string(),
            label: "Code".to_string(),
            category: "Action".to_string(),
            description: Some("Run JavaScript code".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "code".to_string(),
                    label: "Code".to_string(),
                    property_type: "code".to_string(),
                    options: None,
                    default: Some("return {};".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "console_output".to_string(),
            label: "Console Log".to_string(),
            category: "Action".to_string(),
            description: Some("Log data to console".to_string()),
            documentation: None,
            properties: vec![],
            outputs: vec![],
        },
        NodeType {
            id: "delay".to_string(),
            label: "Delay".to_string(),
            category: "Action".to_string(),
            description: Some("Pause execution for a duration".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "duration_ms".to_string(),
                    label: "Duration (ms)".to_string(),
                    property_type: "number".to_string(),
                    options: None,
                    default: Some("1000".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
    ]
}

fn get_logic_nodes() -> Vec<NodeType> {
    vec![
        NodeType {
            id: "router".to_string(),
            label: "Router".to_string(),
            category: "Logic".to_string(),
            description: Some("Route based on condition".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "key".to_string(),
                    label: "Key to check".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("id".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "value".to_string(),
                    label: "Value to match".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "operator".to_string(),
                    label: "Operator".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["==".to_string(), "!=".to_string(), ">".to_string(), "<".to_string(), ">=".to_string(), "<=".to_string(), "contains".to_string()]),
                    default: Some("==".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec!["true".to_string(), "false".to_string()],
        },
    ]
}

fn get_ai_nodes() -> Vec<NodeType> {
    vec![
        NodeType {
            id: "agent".to_string(),
            label: "AI Agent".to_string(),
            category: "AI".to_string(),
            description: Some("LLM Agent".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "system_prompt".to_string(),
                    label: "System Prompt".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("You are a helpful assistant.".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "user_prompt".to_string(),
                    label: "User Prompt".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                    json_schema: None,
                },
                NodeProperty {
                    name: "json_schema".to_string(),
                    label: "JSON Schema".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: None,
                    required: false,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
    ]
}

fn get_model_nodes() -> Vec<NodeType> {
    vec![
        NodeType {
            id: "openai_model".to_string(),
            label: "OpenAI Model".to_string(),
            category: "Models".to_string(),
            description: Some("GPT-4 / GPT-3.5".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "model".to_string(),
                    label: "Model Name".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("gpt-4o".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "api_key".to_string(),
                    label: "API Key".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "gemini_model".to_string(),
            label: "Gemini Model".to_string(),
            category: "Models".to_string(),
            description: Some("Gemini Pro / Flash".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "model".to_string(),
                    label: "Model Name".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("gemini-1.5-flash".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "api_key".to_string(),
                    label: "API Key".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
    ]
}

fn get_data_processing_nodes() -> Vec<NodeType> {
    vec![
        NodeType {
            id: "html_extract".to_string(),
            label: "HTML Extract".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Extract data from HTML".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "selector".to_string(),
                    label: "CSS Selector".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("body".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "mode".to_string(),
                    label: "Mode".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["text".to_string(), "html".to_string(), "attribute".to_string()]),
                    default: Some("text".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "attribute".to_string(),
                    label: "Attribute Name".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "join".to_string(),
            label: "Join".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Join two streams".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "type".to_string(),
                    label: "Join Type".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["index".to_string(), "key".to_string()]),
                    default: Some("index".to_string()),
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "key".to_string(),
                    label: "Join Key (comma-separated)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("id".to_string()),
                    required: false,
                    json_schema: None,
                },
                NodeProperty {
                    name: "right_key".to_string(),
                    label: "Right Key (comma-separated)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "union".to_string(),
            label: "Union".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Merge multiple streams".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "mode".to_string(),
                    label: "Mode".to_string(),
                    property_type: "select".to_string(),
                    options: Some(vec!["interleaved".to_string(), "sequential".to_string()]),
                    default: Some("interleaved".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "file_source".to_string(),
            label: "File Source".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Read from file".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "path".to_string(),
                    label: "File Path".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "split".to_string(),
            label: "Split".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Split array into individual items".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "path".to_string(),
                    label: "Path to Array (Optional)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "accumulate".to_string(),
            label: "Accumulate".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Accumulate items into a list".to_string()),
            documentation: None,
            properties: vec![],
            outputs: vec![],
        },
        NodeType {
            id: "dedupe".to_string(),
            label: "Dedupe".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Filter duplicate records".to_string()),
            documentation: None,
            properties: vec![
                NodeProperty {
                    name: "key".to_string(),
                    label: "Key to check (Optional)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: false,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "group_by".to_string(),
            label: "Group By".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Group and aggregate data".to_string()),
            documentation: Some(r#"
Groups data by specified keys and performs aggregations on other columns.

### Properties

- **group_by**: Comma-separated list of keys to group by (e.g., `category,region`).
- **aggregations**: JSON array of aggregation objects.

### Aggregation Object

```json
{
  "column": "value",
  "function": "sum", // sum, count, avg, min, max, median, variance, stddev
  "alias": "total_value" // Optional output key name
}
```
"#.to_string()),
            properties: vec![
                NodeProperty {
                    name: "group_by".to_string(),
                    label: "Group By Keys (comma-separated)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "aggregations".to_string(),
                    label: "Aggregations (JSON)".to_string(),
                    property_type: "json".to_string(),
                    options: None,
                    default: Some("[]".to_string()),
                    required: true,
                    json_schema: Some(serde_json::json!({
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "column": { "type": "string", "title": "Column" },
                                "function": { 
                                    "type": "string", 
                                    "title": "Function",
                                    "enum": ["sum", "count", "avg", "min", "max", "median", "variance", "stddev"] 
                                },
                                "alias": { "type": "string", "title": "Alias (Optional)" }
                            },
                            "required": ["column", "function"]
                        }
                    })),
                },
            ],
            outputs: vec![],
        },
        NodeType {
            id: "stats".to_string(),
            label: "Statistics".to_string(),
            category: "Data Processing".to_string(),
            description: Some("Calculate global statistics".to_string()),
            documentation: Some(r#"
Calculates global statistics for the entire dataset.

### Properties

- **columns**: Comma-separated list of columns to analyze (e.g., `score,age`).
- **operations**: Comma-separated list of operations to perform.

### Supported Operations

- `mean` (or `avg`)
- `median`
- `variance`
- `stddev`
- `min`
- `max`
- `sum`
- `count`
"#.to_string()),
            properties: vec![
                NodeProperty {
                    name: "columns".to_string(),
                    label: "Columns (comma-separated)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: None,
                    required: true,
                    json_schema: None,
                },
                NodeProperty {
                    name: "operations".to_string(),
                    label: "Operations (comma-separated)".to_string(),
                    property_type: "text".to_string(),
                    options: None,
                    default: Some("mean,max,min,count".to_string()),
                    required: true,
                    json_schema: None,
                },
            ],
            outputs: vec![],
        },
    ]
}


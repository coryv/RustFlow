# RustFlow User Guide

This guide provides detailed information on how to use RustFlow to build powerful automation workflows.

## 🧠 Core Concepts

### Nodes
Nodes are the building blocks of a workflow. Each node performs a specific task, such as fetching data, processing it, or making decisions.
-   **Inputs**: Nodes receive data streams from upstream nodes.
-   **Outputs**: Nodes emit data streams to downstream nodes.
-   **Configuration**: Static settings defined in the workflow YAML.

### Edges
Edges connect nodes, defining the flow of data. In RustFlow, edges are asynchronous channels that allow for streaming data processing.

## 🧩 Node Reference

### Standard Nodes
-   **`manual_trigger`**: Starts a workflow manually. Emits a single empty message.
-   **`time_trigger`**: Emits a message on a schedule (Cron expression).
-   **`webhook_trigger`**: Listens for HTTP requests (Mockable).
-   **`console_output`**: Prints received data to stdout (useful for debugging).
-   **`set_data`**: Injects static JSON data into the stream.
-   **`file_source`**: Reads a JSON file (Array or NDJSON) and streams each item.

### Logic & Flow Control
-   **`router`**: Splits a stream based on a condition (e.g., `data.region == "US"`).
-   **`join`**: Merges two streams based on an index or a key (SQL-style join).
-   **`union`**: Combines multiple streams into one (Interleaved or Sequential).

### AI & Compute
-   **`agent`**: Invokes an LLM (like GPT-4).
    -   Supports `system_prompt` and `user_prompt` templating.
    -   Supports `json_schema` for structured output enforcement.
    -   Uses `credential_id` for secure API key access.
-   **`code`**: Executes custom JavaScript or Python code (Sandboxed).
-   **`http_request`**: Makes HTTP requests to external APIs.

## 🔐 Credentials Management

RustFlow includes a secure credentials management system to keep your secrets safe.

### Setup
Set the master encryption key:
```bash
export RUSTFLOW_MASTER_KEY=$(openssl rand -hex 32)
```

### CLI Commands
-   **Create**: `cargo run --bin cli -- create-credential --name "My Key" --type "api_key" --account-id "default"`
-   **List**: `cargo run --bin cli -- list-credentials --account-id "default"`

### Usage in Workflows
Reference the credential by its UUID in your node configuration:
```yaml
- id: "my-agent"
  type: "agent"
  config:
    credential_id: "550e8400-e29b-41d4-a716-446655440000"
```

## 🔌 Integration Protocol

RustFlow allows you to define custom integrations using a simple YAML schema.

### Creating an Integration
1.  Create a YAML file (e.g., `my_integration.yaml`).
2.  Define the nodes and their HTTP implementation details.

```yaml
name: "MyService"
nodes:
  - name: "GetData"
    type: "action"
    implementation:
      type: "http"
      method: "GET"
      url: "https://api.myservice.com/data"
```

The `rust_flow_macros` crate will compile this into high-performance Rust code at build time.

## 🌊 Streaming Engine

RustFlow uses a push-based streaming engine.
-   **Backpressure**: Handled automatically by async channels.
-   **Concurrency**: Parallel branches run on separate tasks.
-   **Splitting**: One output can feed multiple inputs (Broadcast).
-   **Merging**: Nodes like `Join` and `Union` handle multi-input synchronization.

## 📝 Examples

### Basic Workflow
A simple workflow that triggers manually, sets some data, and logs it.
```yaml
nodes:
  - id: "trigger"
    type: "manual_trigger"
  - id: "data"
    type: "set_data"
    config:
      message: "Hello from RustFlow!"
      count: 42
  - id: "log"
    type: "console_output"

edges:
  - from: "trigger"
    to: "data"
  - from: "data"
    to: "log"
```

### AI Agent Workflow
An advanced workflow using an LLM to analyze text, with structured output enforcement.
```yaml
nodes:
  - id: "trigger"
    type: "manual_trigger"
  - id: "data"
    type: "set_data"
    config:
      bio: "I love coding in Rust and building scalable systems."
  - id: "agent"
    type: "agent"
    config:
      model: "gpt-4"
      system_prompt: "You are a helpful assistant. Extract sentiment and keywords."
      user_prompt: "Analyze this bio: {{ bio }}"
      credential_id: "YOUR_CREDENTIAL_UUID"
      json_schema:
        type: "object"
        properties:
          sentiment: { type: "string" }
          keywords: { type: "array", items: { type: "string" } }
        required: ["sentiment", "keywords"]
  - id: "log"
    type: "console_output"

edges:
  - from: "trigger"
    to: "data"
  - from: "data"
    to: "agent"
  - from: "agent"
    to: "log"
```

### Split & Join Logic
Demonstrates branching logic and joining streams based on a key.
```yaml
nodes:
  - id: "trigger"
    type: "manual_trigger"
  - id: "data_us"
    type: "set_data"
    config: { id: "1", region: "US", value: "A" }
  - id: "data_eu"
    type: "set_data"
    config: { id: "1", region: "EU", value: "B" }
  - id: "router"
    type: "router"
    config:
      key: "region"
      value: "US"
  - id: "join"
    type: "join"
    config:
      type: "key"
      key: "id"
  - id: "log"
    type: "console_output"

edges:
  # Flow 1: US Data -> Router -> Join (Left)
  - from: "trigger"
    to: "data_us"
  - from: "data_us"
    to: "router"
  - from: "router"
    from_port: 0 # Matches "US"
    to: "join"
    to_port: 0

  # Flow 2: EU Data -> Join (Right)
  - from: "trigger"
    to: "data_eu"
  - from: "data_eu"
    to: "join"
    to_port: 1

  # Final Output
  - from: "join"
    to: "log"
```

### Index Join (Inner/Outer)
Joins two streams by their position (1st item with 1st item, etc.). Supports `inner`, `left`, `right`, and `outer` modes.

```yaml
nodes:
  - id: "trigger"
    type: "manual_trigger"
  - id: "data_a"
    type: "set_data"
    config: { items: ["A1", "A2"] }
  - id: "data_b"
    type: "set_data"
    config: { items: ["B1", "B2", "B3"] }
  - id: "join"
    type: "join"
    config:
      type: "index"
      mode: "left" # Options: inner, left, right, outer
  - id: "log"
    type: "console_output"

edges:
  - from: "trigger"
    to: "data_a"
  - from: "trigger"
    to: "data_b"
  - from: "data_a"
    to: "join"
    to_port: 0
  - from: "data_b"
    to: "join"
    to_port: 1
  - from: "join"
    to: "log"
```

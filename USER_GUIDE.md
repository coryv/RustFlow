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

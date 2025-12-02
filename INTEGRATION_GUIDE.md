# Integration Development Guide

RustFlow uses a declarative YAML-based system to generate high-performance Rust code for integrations. This allows you to add new integrations without writing Rust code manually.

## 📂 Directory Structure

All integration definitions are located in the `integrations/` directory at the root of the project.

```text
rust_flow/
├── integrations/
│   ├── notion.yaml
│   ├── slack.yaml
│   └── your_new_integration.yaml
├── build.rs
└── ...
```

To add a new integration, simply create a new `.yaml` file in this directory.

## 📝 YAML Schema

An integration file consists of a top-level name and a list of nodes.

### Root Object

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | The display name of the integration (e.g., "Notion", "Slack"). |
| `nodes` | List | A list of node definitions. |

### Node Object

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | The name of the specific action (e.g., "Create Page"). |
| `type` | String | The node type. Currently supports `action`. |
| `implementation` | Object | Defines how the node executes. |
| `properties` | List | Defines the input fields exposed to the UI. |

### Implementation Object (HTTP)

Currently, the system supports HTTP-based implementations.

| Field | Type | Description |
|-------|------|-------------|
| `type` | String | Must be `http`. |
| `method` | String | HTTP method (`GET`, `POST`, `PUT`, `DELETE`, `PATCH`). |
| `url` | String | The target URL. Supports templating. |
| `headers` | Map | Key-value pairs for headers. Supports templating. |
| `body` | Object | The request body (JSON). Supports templating. |

### Property Object

These properties define the form fields shown to the user in the Workflow Editor.

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | The internal variable name (used in templates). |
| `label` | String | The human-readable label shown in the UI. |
| `type` | String | The data type: `text`, `json`, `boolean`, `select`. |
| `required` | Boolean | Whether the field is mandatory. |
| `default` | String | (Optional) Default value. |
| `options` | List | (Optional) List of strings for `select` type. |

## 🎨 Templating

You can use `minijinja` syntax (similar to Jinja2) to inject dynamic values into the `url`, `headers`, and `body`.

-   **`{{ input.field_name }}`**: Access values passed from the previous node or defined in the node's properties.
-   **`{{ credential.token }}`**: Access the credential token associated with the execution.

## 🚀 Example

Here is a complete example of a Slack integration with a "Post Message" node.

```yaml
name: "Slack"
nodes:
  - name: "Post Message"
    type: "action"
    implementation:
      type: "http"
      method: "POST"
      url: "https://slack.com/api/chat.postMessage"
      headers:
        Authorization: "Bearer {{ credential.token }}"
        Content-Type: "application/json"
      body:
        channel: "{{ input.channel }}"
        text: "{{ input.text }}"
    properties:
      - name: "credential_id"
        label: "Credential ID"
        type: "text"
        required: true
      - name: "channel"
        label: "Channel"
        type: "text"
        required: true
      - name: "text"
        label: "Message Text"
        type: "text"
        required: true
```

## ⚙️ How It Works

1.  **Build Script**: The `build.rs` script runs before compilation. It scans the `integrations/` directory for `.yaml` files.
2.  **Code Generation**: It generates Rust structs for each node, implementing the `StreamNode` trait with high-performance `reqwest` HTTP clients.
3.  **Registry**: It generates a `create_integration_node` factory function and a `get_integration_node_definitions` metadata function.
4.  **Compilation**: The generated code is included in `src/integrations.rs` and compiled into the binary.
5.  **Runtime**: The application uses the generated registry to expose nodes to the frontend and execute them at runtime.

No manual Rust coding is required to add standard HTTP-based integrations!

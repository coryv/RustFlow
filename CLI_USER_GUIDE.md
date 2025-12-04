# RustFlow CLI User Guide

The RustFlow CLI is a powerful tool for building, testing, and debugging workflows directly from your terminal.

## Getting Started

To start the CLI, run:

```bash
cargo run --bin cli
```

## Workflow Builder

The Builder allows you to interactively create workflows by adding nodes, configuring properties, and connecting them.

### Adding Nodes
1. Select **Build Workflow** from the main menu.
2. Choose **Add Node**.
3. Select a category (Trigger, Action, Logic, etc.) and then the specific node type.

### Configuring Properties & Variable Mapping
When configuring node properties, you can enter static values or map outputs from previous nodes.

**Variable Mapping (`?` Trigger):**
1. When prompted for a value (e.g., URL, Body), type `?` and press Enter.
2. Select the source node you want to use data from.
3. Select a common variable path (e.g., `body.id`) or enter a custom path.
4. The CLI will insert the variable in the correct template format: `{{ node_id.body.id }}`.

### Connecting Nodes
1. Select **Connect Nodes**.
2. Choose the **From Node** (source).
3. Choose the **To Node** (destination).
4. **Output Selection**: If the source node has multiple named outputs (e.g., `success`, `error`), you will be prompted to select which output port to connect.

## Data Inspector

The Data Inspector allows you to view and query the data flowing through your workflow during execution.

### Inspecting Data
1. Run a workflow in **Debug Mode** (or use the **Test Workflow** option).
2. When execution pauses or completes, you can inspect the data at any node.

### Navigation & Querying
- **Navigate**: Use arrow keys to browse JSON objects and arrays.
- **JMESPath Query**: Type a JMESPath expression (e.g., `body.results[*].id`) to filter and extract specific data from the current view.

## YAML Configuration

You can also define workflows manually using YAML. This gives you fine-grained control, including the ability to use **Named Ports** for logic nodes.

### Named Ports
Instead of using numeric indices (e.g., `0`, `1`) for output ports, you can use descriptive names. This is especially useful for nodes like the **Router**.

**Example:**
```yaml
nodes:
  - id: check_status
    type: router
    config:
      key: status
      value: active
      operator: "=="

edges:
  # Route for "True" (Match)
  - from: check_status
    to: process_active
    from_port: "true"

  # Route for "False" (Default)
  - from: check_status
    to: handle_inactive
    from_port: "false"
```

Currently, the `router` node supports:
- `"true"`: The condition was met (equivalent to port `0`).
- `"false"`: The condition was not met (equivalent to port `1`).

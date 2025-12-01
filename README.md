# RustFlow

**RustFlow** is a high-performance, concurrent workflow automation engine written in Rust. It is designed to orchestrate AI agents, handle enterprise-grade data flows, and run anywhere—from high-performance servers to the browser via WebAssembly.

## 🚀 Key Features

-   **Streaming Engine**: True streaming architecture allows for processing large datasets with low latency and high concurrency.
-   **AI Native**: Built-in `AgentNode` for seamless integration with LLMs (OpenAI, etc.), featuring prompt templating and JSON schema guardrails.
-   **Secure Credentials**: AES-GCM encrypted credential storage with runtime injection.
-   **Extensible Integrations**: Declarative YAML-based integration protocol with macro-generated high-performance Rust code.
-   **Wasm Compatible**: Core engine abstracts the async runtime, allowing it to run in the browser.
-   **Developer Friendly**: CLI tools for managing workflows, accounts, and credentials.

## 📦 Installation

### Prerequisites
-   Rust (latest stable)
-   SQLite (for persistence)

### Build
```bash
cargo build --release
```

## ⚡ Quick Start

### 1. Initialize Database
```bash
cargo run --bin cli -- init-db
```

### 2. Create a Credential (Optional)
If you plan to use AI agents, store your API key securely:
```bash
export RUSTFLOW_MASTER_KEY=$(openssl rand -hex 32)
cargo run --bin cli -- create-credential --name "OpenAI Key" --type "openai_api" --account-id "default"
```

### 3. Run a Workflow
Create a simple workflow file `hello.yaml`:
```yaml
nodes:
  - id: "trigger"
    type: "manual_trigger"
  - id: "log"
    type: "console_output"
edges:
  - from: "trigger"
    to: "log"
```

Run it:
```bash
cargo run --bin cli -- run --file hello.yaml
```

## 📚 Documentation

For detailed usage instructions, node reference, and advanced features, see the [User Guide](USER_GUIDE.md).

## 🏗️ Project Structure

-   `src/stream_engine`: Core execution engine and node traits.
-   `src/nodes`: Standard node implementations.
-   `src/storage`: Persistence layer (SQLite) and encryption.
-   `rust_flow_macros`: Proc-macros for generating integrations.
-   `rust_flow_wasm`: Wasm bindings for browser usage.
-   `ui`: React-based visual editor (Work in Progress).

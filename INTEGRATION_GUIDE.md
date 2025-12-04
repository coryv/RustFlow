# RustFlow Integration Guide

This guide explains how to add new integrations to RustFlow using YAML definitions.

## Overview

Integrations are defined in the `integrations/` directory as `.yaml` files. The build system automatically parses these files and generates the necessary Rust code and node registry entries.

## File Structure

Create a new file `integrations/<integration_name>.yaml`.

```yaml
name: MyService
credentials:
  - name: api_key
    label: API Key
    type: password
    required: true
    description: "Your API Key from MyService dashboard"
nodes:
  - name: CreateItem
    type: action
    documentation: |
      Creates a new item in MyService.
      
      ### Properties
      - **name**: The name of the item.
    implementation:
      type: http
      method: POST
      url: https://api.myservice.com/items
      headers:
        Authorization: "Bearer {{ api_key }}"
      body:
        name: "{{ name }}"
    properties:
      - name: api_key
        label: API Key
        type: text
        required: true
      - name: name
        label: Item Name
        type: text
        required: true
```

## Fields

### Integration
- `name`: Name of the integration (e.g., "Slack", "Notion").
- `credentials`: List of credential properties required for authentication.

### Credential Property
- `name`: Internal variable name (e.g., `token`).
- `label`: UI label.
- `type`: Data type (`text`, `password`).
- `required`: Boolean.
- `description`: Helper text for the user.

### Node
- `name`: Name of the node (e.g., "PostMessage").
- `type`: Node category (currently mostly `action`).
- `documentation`: (Optional) Markdown documentation for the node.
- `outputs`: (Optional) List of named outputs (e.g., `success`, `error`). Defaults to a single output if omitted.
- `implementation`: Implementation details.
- `properties`: List of input properties.

### Implementation (HTTP)
- `type`: `http`
- `method`: HTTP method (GET, POST, PUT, DELETE, PATCH).
- `url`: Target URL. Supports Jinja2 templating (e.g., `{{ id }}`).
- `headers`: Map of headers. Supports templating.
- `body`: Request body (JSON or string). Supports templating.
- `transform`: (Optional) JMESPath expression to transform the response body before outputting.

### Property
- `name`: Internal variable name.
- `label`: UI label.
- `type`: Data type (`text`, `number`, `select`, `json`, `code`, `boolean`).
- `required`: Boolean.
- `default`: Default value.
- `options`: List of options for `select` type.

## Advanced Features

### Multiple Outputs
You can define multiple named outputs for a node to handle different outcomes (e.g., success vs. error).

```yaml
outputs:
  - name: success
  - name: error
```

In the implementation, the system will automatically route successful HTTP responses to the `success` output (or the first output if `success` isn't found) and errors to the `error` output.

### Data Transformation
You can use [JMESPath](https://jmespath.org/) to transform the response data before it is passed to the next node. This is useful for extracting specific fields or restructuring the JSON.

```yaml
implementation:
  type: http
  # ...
  transform: "results[].{id: id, name: title}"
```

## Templating

You can use Jinja2 syntax `{{ variable }}` in `url`, `headers`, and `body`. The variables available are the keys from the incoming data stream, which should match the `properties` you define.

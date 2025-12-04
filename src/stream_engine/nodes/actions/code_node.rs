use async_trait::async_trait;
use crate::stream_engine::StreamNode;
use tokio::sync::mpsc::{Receiver, Sender};
use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use boa_engine::{Context, Source, JsValue, JsString};
use rustpython_vm as vm;
use rustpython_vm::PyObjectRef;

pub struct CodeNode {
    lang: String,
    code: String,
}

impl CodeNode {
    pub fn new(lang: String, code: String) -> Self {
        Self { lang, code }
    }

    fn run_js(&self, input: Value) -> Result<Value> {
        // Create a new context for each execution to ensure isolation
        let mut context = Context::default();

        // Convert input serde Value to JsValue
        let input_js = JsValue::from_json(&input, &mut context)
            .map_err(|e| anyhow!("Failed to convert input to JS: {}", e))?;

        // Set 'input' global variable
        let input_key = JsString::from("input");
        context.global_object()
            .set(input_key, input_js, true, &mut context)
            .map_err(|e| anyhow!("Failed to set global 'input': {}", e))?;

        // Execute the code
        let source = Source::from_bytes(self.code.as_bytes());
        context.eval(source)
            .map_err(|e| anyhow!("JS Execution Error: {}", e))?;

        // Get 'output' global variable
        let output_key = JsString::from("output");
        let output_js = context.global_object()
            .get(output_key, &mut context)
            .map_err(|e| anyhow!("Failed to get global 'output': {}", e))?;

        // Convert back to serde Value
        // `to_json` returns Result<serde_json::Value, ...>
        let output_json = output_js.to_json(&mut context)
            .map_err(|e| anyhow!("Failed to convert output to JSON: {}", e))?;

        Ok(output_json.unwrap_or(Value::Null))
    }

    fn run_python(&self, input: Value) -> Result<Value> {
        vm::Interpreter::without_stdlib(Default::default()).enter(|vm| {
            let scope = vm.new_scope_with_builtins();

            // Convert input to Python object
            // We need a way to convert serde_json::Value to PyObjectRef.
            // RustPython has serde support but it might need a feature flag or manual conversion.
            // For MVP, let's convert via JSON string if possible, or manual recursion.
            // Manual recursion is safer/cleaner for now.
            let input_py = Self::json_to_py(vm, &input);
            
            scope.locals.set_item("input", input_py, vm)
                .map_err(|e| anyhow!("Failed to set input: {:?}", e))?;

            // Execute code
            let code_obj = vm.compile(&self.code, vm::compiler::Mode::Exec, "<embedded>".to_owned())
                .map_err(|e| vm.new_syntax_error(&e, Some(&self.code)))
                .map_err(|e| anyhow!("Python Compile Error: {:?}", e))?;

            vm.run_code_obj(code_obj, scope.clone())
                .map_err(|e| anyhow!("Python Execution Error: {:?}", e))?;

            // Get output
            let output_py = scope.locals.get_item("output", vm)
                .map_err(|_| anyhow!("Failed to get 'output' variable"))?;

            // Convert back to JSON
            let output_json = Self::py_to_json(vm, output_py)?;
            Ok(output_json)
        })
    }

    fn json_to_py(vm: &vm::VirtualMachine, val: &Value) -> PyObjectRef {
        match val {
            Value::Null => vm.ctx.none(),
            Value::Bool(b) => vm.ctx.new_bool(*b).into(),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    vm.ctx.new_int(i).into()
                } else if let Some(f) = n.as_f64() {
                    vm.ctx.new_float(f).into()
                } else {
                    vm.ctx.none() // Should not happen
                }
            }
            Value::String(s) => vm.ctx.new_str(s.as_str()).into(),
            Value::Array(arr) => {
                let elements: Vec<PyObjectRef> = arr.iter().map(|v| Self::json_to_py(vm, v)).collect();
                vm.ctx.new_list(elements).into()
            }
            Value::Object(obj) => {
                let dict = vm.ctx.new_dict();
                for (k, v) in obj {
                    let _ = dict.set_item(k.as_str(), Self::json_to_py(vm, v), vm);
                }
                dict.into()
            }
        }
    }

    fn py_to_json(vm: &vm::VirtualMachine, obj: PyObjectRef) -> Result<Value> {
        // Basic conversion. 
        if vm.is_none(&obj) {
            return Ok(Value::Null);
        }
        if let Some(s) = obj.payload::<vm::builtins::PyStr>() {
            return Ok(Value::String(s.as_str().to_string()));
        }
        if let Some(_i) = obj.payload::<vm::builtins::PyInt>() {
             if let Ok(val) = obj.clone().try_into_value::<i64>(vm) {
                 return Ok(json!(val));
             }
        }
        if let Some(f) = obj.payload::<vm::builtins::PyFloat>() {
            return Ok(json!(f.to_f64()));
        }
        if let Ok(b) = obj.clone().try_into_value::<bool>(vm) {
            return Ok(Value::Bool(b));
        }
        if let Some(list) = obj.payload::<vm::builtins::PyList>() {
            let mut arr = Vec::new();
            for item in list.borrow_vec().iter() {
                arr.push(Self::py_to_json(vm, item.clone())?);
            }
            return Ok(Value::Array(arr));
        }
        if let Some(dict) = obj.payload::<vm::builtins::PyDict>() {
            let mut map = serde_json::Map::new();
            for (k, v) in dict {
                let key_str = k.str(vm).map_err(|e| anyhow!("Failed to convert key to string: {:?}", e))?.to_string();
                map.insert(key_str, Self::py_to_json(vm, v)?);
            }
            return Ok(Value::Object(map));
        }

        // Fallback: Convert to string
        Ok(Value::String(obj.str(vm).map_err(|e| anyhow!("Failed to convert to string: {:?}", e))?.to_string()))
    }
}

#[async_trait]
impl StreamNode for CodeNode {
    async fn run(&self, mut inputs: Vec<Receiver<Value>>, outputs: Vec<Sender<Value>>) -> Result<()> {
        if let Some(rx) = inputs.get_mut(0) {
            if let Some(tx) = outputs.first() {
                while let Some(data) = rx.recv().await {
                    let result = match self.lang.as_str() {
                        "js" | "javascript" => self.run_js(data),
                        "py" | "python" => self.run_python(data),
                        _ => Err(anyhow!("Unsupported language: {}", self.lang)),
                    };

                    match result {
                        Ok(val) => {
                            tx.send(val).await?;
                        }
                        Err(e) => {
                            eprintln!("CodeNode Error: {}", e);
                            // Optionally emit error
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

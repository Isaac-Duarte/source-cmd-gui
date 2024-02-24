use std::collections::HashMap;

use log::error;
use pyo3::{
    types::{PyDict, PyModule, PyString},
    PyErr, Python,
};

use serde::{Deserialize, Serialize};

use serde_json::Value;
use source_cmd_parser::{
    error::SourceCmdError,
    model::{ChatMessage, ChatResponse},
};

use crate::{
    error::{SourceCmdGuiError, SourceCmdGuiResult},
    model::{entity::Script, state::Config},
};

pub trait ToPyDict {
    fn to_py_dict(&self, py: Python<'_>) -> Result<pyo3::prelude::PyObject, PyErr>;
}

impl ToPyDict for ChatMessage {
    fn to_py_dict(&self, py: Python<'_>) -> Result<pyo3::prelude::PyObject, PyErr> {
        let dict = PyDict::new(py);

        dict.set_item("time_stamp", self.time_stamp.to_rfc3339())?;
        dict.set_item("user_name", self.user_name.clone())?;
        dict.set_item("message", self.message.clone())?;
        dict.set_item("command", self.command.clone())?;

        Ok(dict.into())
    }
}

impl ToPyDict for Config {
    fn to_py_dict(&self, py: Python<'_>) -> Result<pyo3::prelude::PyObject, PyErr> {
        let dict = PyDict::new(py);

        dict.set_item("file_path", self.file_path.clone())?;
        dict.set_item("command_timeout", self.command_timeout)?;
        dict.set_item("owner", self.owner.clone())?;
        dict.set_item("openai_api_key", self.openai_api_key.clone())?;
        dict.set_item("disabled_commands", self.disabled_commands.clone())?;
        dict.set_item("response_direction", self.response_direction.clone())?;

        Ok(dict.into())
    }
}

pub async fn process_python_command(
    script: &Script,
    message: ChatMessage,
    config: &Config,
    python_context: DynamicPythonCtx,
) -> SourceCmdGuiResult<(Option<ChatResponse>, Option<DynamicPythonCtx>)> {
    let code = script.get_code().await?;

    let result: Result<(Option<String>, Option<String>), PyErr> = Python::with_gil(|py| {
        let locals = PyDict::new(py);

        locals.set_item("message", message.to_py_dict(py)?)?;
        locals.set_item("config", config.to_py_dict(py)?)?;

        let serialized: String = python_context.try_into().unwrap_or("{}".to_owned());

        let py_string = PyString::new(py, &serialized);
        locals.set_item("context", py_string)?;

        let code = code.clone()
            + r#"
ref_context = dict()
modified_context = dict()

def get_object(name):
    global modified_context
    global ref_context

    if name in modified_context:
        return modified_context[name]
    
    return ref_context[name]

def set_object(name, value):
    modified_context[name] = value

def _main(locals):
    import io
    import sys
    import json
    
    result = None
    
    global ref_context
    ref_context = json.loads(locals['context'])

    from contextlib import redirect_stdout, redirect_stderr
    with io.StringIO() as new_stdout, io.StringIO() as new_stderr:
        with redirect_stdout(new_stdout), redirect_stderr(new_stderr):
            try:
                result = main(locals)
            except Exception as e:
                print(e, file=sys.stderr)
        output = new_stdout.getvalue()
        error_output = new_stderr.getvalue()

    return error_output, result or None, json.dumps(modified_context)
"#;

        let py_module = PyModule::from_code(py, &code, "main.py", "main")?;
        let main_func = py_module.getattr("_main")?;
        let result = main_func.call1((locals,))?;

        let error_output = result.get_item(0).unwrap().extract::<String>()?;
        let output = result.get_item(1).unwrap().extract::<Option<String>>()?;
        let context = result.get_item(2).unwrap().extract::<String>()?;

        if !error_output.is_empty() {
            error!("Error running python command: {}", error_output)
        }

        Ok((output, Some(context)))
    });

    Ok(match result {
        Ok((output, context)) => (
            output.map(ChatResponse::new),
            context
                .map(|x| DynamicPythonCtx::try_from(x).ok())
                .flatten(),
        ),
        Err(e) => {
            error!("Error running python command: {}", e);
            (None, None)
        }
    })
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DynamicPythonCtx {
    /// Transferring data between Python & Rust is tricky as it requires the same GIL
    /// So to prevent unsafe & segmentation fault code, we're going to serialize the data to json.
    /// We were going to do a hashmap, but it is tricky to Serialize the fucking PyObject.
    inner: HashMap<String, Value>,
}

impl Default for DynamicPythonCtx {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl TryFrom<String> for DynamicPythonCtx {
    type Error = SourceCmdGuiError;

    fn try_from(value: String) -> SourceCmdGuiResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(&value)?,
        })
    }
}

impl TryInto<String> for DynamicPythonCtx {
    type Error = SourceCmdGuiError;

    fn try_into(self) -> Result<String, Self::Error> {
        Ok(serde_json::to_string(&self.inner)?)
    }
}

impl DynamicPythonCtx {
    pub fn override_values(&mut self, reference: &Self) {
        reference.inner.iter().for_each(|(key, value)| {
            self.inner.insert(key.to_string(), value.clone());
        });
    }
}

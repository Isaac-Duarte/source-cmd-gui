use log::error;
use pyo3::{
    types::{PyDict, PyList, PyString},
    PyErr, Python,
};
use serde::{Deserialize, Serialize};
use source_cmd_parser::model::{ChatMessage, ChatResponse};

use crate::model::state::Config;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Script {
    pub id: Option<i64>,
    pub name: String,
    pub code: String,
    pub trigger: String,
}

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

pub fn process_python_command(
    script: &Script,
    message: ChatMessage,
    config: &Config,
) -> Option<ChatResponse> {
    let result: Result<String, PyErr> = Python::with_gil(|py| {
        let locals = PyDict::new(py);

        locals.set_item("message", message.to_py_dict(py)?)?;
        locals.set_item("config", config.to_py_dict(py)?)?;
        locals.set_item("code", &script.code)?;
    
        let code = r#"
import io
import sys

from contextlib import redirect_stdout, redirect_stderr
with io.StringIO() as new_stdout, io.StringIO() as new_stderr:
    with redirect_stdout(new_stdout), redirect_stderr(new_stderr):
        try:
            exec(code)
        except Exception as e:
            print(e)
    output = new_stdout.getvalue() + new_stderr.getvalue()
        "#;

        py.run(&code, None, Some(locals))?;

        let output: String = locals.get_item("output").unwrap().unwrap().extract()?;

        Ok(output)
    });

    match result {
        Ok(output) => Some(ChatResponse::new(output)),
        Err(e) => {
            error!("Error running python command: {}", e);
            None
        }
    }
}


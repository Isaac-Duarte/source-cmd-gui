use std::path::Path;

use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    error::{SourceCmdGuiError, SourceCmdGuiResult},
    model::entity::Script,
};

pub trait ScriptRepository {
    async fn init(&mut self) -> SourceCmdGuiResult;
    async fn add_script(&mut self, script: String) -> SourceCmdGuiResult<Script>;
    fn get_script(&self, id: &str) -> SourceCmdGuiResult<Script>;
    async fn update_script(&mut self, id: &str, script: Script) -> SourceCmdGuiResult;
    async fn delete_script(&mut self, id: &str) -> SourceCmdGuiResult;
    async fn get_scripts(&self) -> SourceCmdGuiResult<Vec<Script>>;
    async fn get_script_by_trigger(&self, trigger: &str) -> SourceCmdGuiResult<Option<Script>>;
}
pub struct JsonRepository {
    internal_scripts: Vec<Script>,
    file_path: String,
}

impl JsonRepository {
    pub async fn new(file_path: String) -> Self {
        JsonRepository {
            internal_scripts: Vec::new(),
            file_path,
        }
    }

    async fn read_from_file(&mut self) -> Result<(), std::io::Error> {
        let path = Path::new(&self.file_path);

        if path.exists() {
            let mut file = File::open(path).await?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).await?;
            self.internal_scripts = serde_json::from_str(&contents)?;
        }

        Ok(())
    }

    async fn write_to_file(&self) -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file_path)
            .await?;

        let contents = serde_json::to_string(&self.internal_scripts)?;
        file.write_all(contents.as_bytes()).await?;

        Ok(())
    }
}

impl ScriptRepository for JsonRepository {
    async fn init(&mut self) -> SourceCmdGuiResult {
        self.read_from_file().await?;

        Ok(())
    }

    async fn add_script(&mut self, script_name: String) -> SourceCmdGuiResult<Script> {
        let script = Script::new(script_name);

        script.save_code(
r#"# The entry point of the script
def main(args):
    # Args is layed as below
    
    # message_struct = args['message'] # This is the message dict
    # message = message_struct['message'] # This is the chat message
    # command = message_struct['command'] # This is the message command (Trigger)
    # user_name = message_struct['user_name'] # This is the user name of the user who sent the message
    # time_stamp = message_struct['time_stamp'] # This is a TFC3339 string

    # config = args['config']
    
    # The return can be None or a String
    
    pass"#).await?;

        self.internal_scripts.push(script.clone());
        self.write_to_file().await?;

        Ok(script)
    }

    fn get_script(&self, id: &str) -> SourceCmdGuiResult<Script> {
        self.internal_scripts
            .iter()
            .find(|&script| script.id == id)
            .cloned()
            .ok_or_else(|| SourceCmdGuiError::ScriptNotFound(id.to_string()))
    }

    async fn update_script(&mut self, id: &str, script: Script) -> SourceCmdGuiResult {
        let position = self.internal_scripts.iter().position(|s| s.id == id);
        if let Some(pos) = position {
            self.internal_scripts[pos] = script;
            self.write_to_file().await?;
            Ok(())
        } else {
            Err(SourceCmdGuiError::ScriptNotFound(id.to_string()))
        }
    }

    async fn delete_script(&mut self, id: &str) -> SourceCmdGuiResult {
        if let Some(pos) = self.internal_scripts.iter().position(|s| s.id == id) {
            let script = self.internal_scripts.get(pos).unwrap();
            script.delete_script().await?;

            self.internal_scripts.remove(pos);
            self.write_to_file().await?;

            Ok(())
        } else {
            Err(SourceCmdGuiError::ScriptNotFound(id.to_string()))
        }
    }

    async fn get_scripts(&self) -> SourceCmdGuiResult<Vec<Script>> {
        Ok(self.internal_scripts.clone())
    }

    async fn get_script_by_trigger(&self, trigger: &str) -> SourceCmdGuiResult<Option<Script>> {
        Ok(self
            .internal_scripts
            .iter()
            .find(|&s| s.enabled && s.trigger == trigger)
            .cloned())
    }
}

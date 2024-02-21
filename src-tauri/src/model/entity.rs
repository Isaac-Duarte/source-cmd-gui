use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
};
use uuid::Uuid;

use crate::SCRIPTS_DIR;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub trigger: String,
    pub enabled: bool,
    pub file_path: String,
}

impl Script {
    pub fn new(name: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let script_dir = SCRIPTS_DIR.to_string_lossy().to_string();

        Self {
            id: id.clone(),
            file_path: format!("{}/{}.py", script_dir, &id),
            name,
            enabled: true,
            ..Default::default()
        }
    }

    // Function to read the script content from the file
    pub async fn get_code(&self) -> Result<String, std::io::Error> {
        fs::read_to_string(&self.file_path).await
    }

    pub async fn save_code(&self, code: &str) -> Result<(), std::io::Error> {
        let mut file = File::create(&self.file_path).await?;
        file.write_all(code.as_bytes()).await?;

        Ok(())
    }

    pub async fn delete_script(&self) -> Result<(), std::io::Error> {
        fs::remove_file(&self.file_path).await?;

        Ok(())
    }
}

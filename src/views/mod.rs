use std::{fs, path::PathBuf};

use dynamic_html::{DynamicHtml, GenerateOptions};

use crate::walker::WalkerLeaf;

#[derive(Debug, Clone)]
pub struct ViewLeaf {
    file_path: PathBuf,
    output_path: PathBuf,
    path: String,
}

impl From<WalkerLeaf> for ViewLeaf {
    fn from(mut value: WalkerLeaf) -> Self {
        value.output_path.set_extension("ts");
        Self {
            file_path: value.file_path,
            output_path: value.output_path,
            path: value.path,
        }
    }
}

impl ViewLeaf {
    pub fn output_path(&self) -> PathBuf {
        self.output_path.clone()
    }

    fn get_options(&self) -> GenerateOptions {
        GenerateOptions::new(
            self.file_path.display().to_string(),
            self.output_path.display().to_string(),
        )
    }

    pub fn generate_file(&self) -> Option<String> {
        let content = match fs::read_to_string(&self.file_path) {
            Ok(c) => c,
            Err(_) => return None,
        };

        let parsed = DynamicHtml::parse(&content).unwrap();
        let result = parsed.generate(&self.get_options());

        Some(result)
    }
}

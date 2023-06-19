use std::{fs, path::PathBuf};

use dynamic_html::{DynamicHtml, GenerateOptions};

use crate::walker::WalkerLeaf;

mod discover;
pub use discover::*;

#[derive(Debug)]
pub struct ViewLeaf {
    file_path: PathBuf,
    output_path: PathBuf,
}

impl From<WalkerLeaf> for ViewLeaf {
    fn from(mut value: WalkerLeaf) -> Self {
        value.output_path.set_extension("ts");
        Self {
            file_path: value.file_path,
            output_path: value.output_path,
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

    /// Transform html to ts.
    /// Returns the TS Code and source map. Type: (code, source\_map)
    pub fn generate_file(&self) -> Option<(String, String)> {
        let content = match fs::read_to_string(&self.file_path) {
            Ok(c) => c,
            Err(_) => return None,
        };

        let parsed = DynamicHtml::parse(&content).unwrap();
        let result = parsed.generate(&self.get_options());

        let (result, source_map) = prettify_js::prettyprint(&result);
        let source_map = prettify_js::generate_source_map(
            self.file_path.display().to_string(),
            content,
            source_map,
        );

        Some((result, source_map))
    }
}

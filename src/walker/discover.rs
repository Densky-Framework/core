use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use glob::{glob, GlobError, PatternError};
use pathdiff::diff_paths;

use crate::{utils::join_paths, CompileContext};

use super::{container::WalkerContainer, WalkerLeaf, WalkerTree};

#[derive(Debug)]
pub enum HttpDiscoverError {
    GlobError(PatternError),
    EntryError(GlobError),
}

pub fn walker_tree_discover<F, R>(
    folder_name: F,
    input_path: R,
    ctx: CompileContext,
) -> Result<(WalkerContainer, Arc<Mutex<WalkerTree>>), HttpDiscoverError>
where
    F: AsRef<Path>,
    R: AsRef<Path>,
{
    let output_dir = join_paths(folder_name, &ctx.output_dir);

    let glob_iter = input_path.as_ref().join("**/*.ts");
    let glob_iter = glob_iter.to_str().unwrap();

    let glob_iter = match glob(glob_iter) {
        Ok(glob_iter) => glob_iter,
        Err(err) => return Err(HttpDiscoverError::GlobError(err)),
    };

    let mut container = WalkerContainer::new(output_dir.clone());
    let root = container.create_root();

    for entry in glob_iter {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let relative = match diff_paths(&entry, &input_path) {
            Some(path) => path,
            None => continue,
        };
        let path = "/".to_string() + &relative.with_extension("").display().to_string();
        let file_path = entry.display().to_string();
        let output_path = join_paths(relative, &output_dir);

        let leaf = WalkerLeaf::new(path, file_path, output_path);
        let leaf = container.add_leaf(leaf);
        root.lock().unwrap().add_child(leaf, &mut container);
    }

    Ok((container, root))
}

use std::{cell::RefCell, path::Path, rc::Rc};

use glob::{glob, GlobError, PatternError};
use pathdiff::diff_paths;

use crate::{utils::join_paths, CompileContext};

use super::{HttpLeaf, HttpTree};

#[derive(Debug)]
pub enum HttpDiscoverError {
    GlobError(PatternError),
    EntryError(GlobError),
}

pub fn http_discover(ctx: CompileContext) -> Result<Rc<RefCell<HttpTree>>, HttpDiscoverError> {
    let output_dir = join_paths("http", &ctx.output_dir);

    let glob_iter = Path::new(&ctx.routes_path)
        .join("**/*.ts")
        .display()
        .to_string();

    let glob_iter = match glob(glob_iter.as_str()) {
        Ok(glob_iter) => glob_iter,
        Err(err) => return Err(HttpDiscoverError::GlobError(err)),
    };

    let tree = HttpTree {
        output_path: join_paths("_index", &output_dir).into(),
        is_root: true,
        ..Default::default()
    };
    let tree = Rc::new(RefCell::new(tree));

    for entry in glob_iter.filter_map(Result::ok) {
        let relative = match diff_paths(&entry, &ctx.routes_path) {
            Some(path) => path,
            None => continue,
        };
        let path = "/".to_string() + &relative.with_extension("").display().to_string();
        let file_path = entry.display().to_string();
        let output_path = join_paths(relative, &output_dir);

        let leaf = HttpLeaf::new(path, file_path, output_path);
        let mut leaf = HttpTree::new_leaf(leaf);
        HttpTree::add_child(tree.clone(), &mut leaf, &output_dir);
    }

    Ok(tree)
}

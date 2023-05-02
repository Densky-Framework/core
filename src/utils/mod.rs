use std::path::{Path, PathBuf};

mod url_to_matcher;
pub use url_to_matcher::*;

pub fn relative_path<T: AsRef<Path>, B: AsRef<Path>>(target: T, base: B) -> Option<PathBuf> {
    let relative = pathdiff::diff_paths(target.as_ref(), base.as_ref())?;
    let relative = relative.to_str()?;

    let relative = if &relative.chars().nth(0) == &Some('.') {
        relative.to_string()
    } else {
        format!("./{}", relative)
    };

    Some(relative.into())
}

pub fn join_paths<T: AsRef<Path>, B: AsRef<Path>>(target: T, base: B) -> String {
    let target = PathBuf::from(target.as_ref());

    if target.has_root() {
        return target.display().to_string();
    }

    let mut base = PathBuf::from(base.as_ref());

    for section in target.iter() {
        match section.to_str().unwrap() {
            "." => {
                continue;
            }
            ".." => {
                base.pop();
            }
            str => base.push(str),
        }
    }

    base.display().to_string()
}

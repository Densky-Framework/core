use std::fmt;
use std::path::{Path, PathBuf};

mod importer;
mod url_to_matcher;
pub use importer::*;
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

pub fn normalize_path<T: AsRef<Path>>(target: T) -> String {
    let target = PathBuf::from(target.as_ref());

    let mut base = PathBuf::from("/");

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

pub struct Fmt<F>(pub F)
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result;

impl<F> fmt::Debug for Fmt<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.0)(f)
    }
}

impl<F> fmt::Display for Fmt<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.0)(f)
    }
}

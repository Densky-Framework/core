use std::path::PathBuf;
use std::str::FromStr;

mod url_to_matcher;
pub use url_to_matcher::*;

pub fn relative_path(target: String, base: String) -> Option<PathBuf> {
    let relative = pathdiff::diff_paths(target, base)?;
    let relative = relative.to_str()?;

    let relative = if &relative.chars().nth(0) == &Some('.') {
        relative.to_string()
    } else {
        format!("./{}", relative)
    };

    Some(relative.into())
}

pub fn join_paths<B: AsRef<str>, T: AsRef<str>>(target: T, base: B) -> String {
    let target = PathBuf::from_str(target.as_ref()).unwrap();

    if target.has_root() {
        return target.display().to_string();
    }

    let mut base = PathBuf::from_str(base.as_ref()).unwrap();

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

use std::env::JoinPathsError;
use std::path::PathBuf;
use std::str::FromStr;

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

pub fn join_paths<B: Into<String>, T: Into<String>>(
    target: T,
    base: B,
) -> Result<String, JoinPathsError> {
    let target = PathBuf::from_str(&target.into()).unwrap();

    if target.has_root() {
        return Ok(target.display().to_string());
    }

    let mut base = PathBuf::from_str(&base.into()).unwrap();

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

    Ok(base.display().to_string())
}

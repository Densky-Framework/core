use std::{cell::RefCell, fmt};

use pathdiff::diff_paths;
use recur_fn::{recur_fn, RecurFn};
use regex::Regex;

#[derive(Debug)]
pub enum HttpParseError {
    Empty(String),
    NoClass(String),
    InvalidSyntax(String, String),
    CurrentDir(std::io::Error),
}

impl fmt::Display for HttpParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDir(err) => err.fmt(f),
            Self::Empty(rel_path) | Self::NoClass(rel_path) | Self::InvalidSyntax(rel_path, _) => {
                f.write_str(format!("[{}]", rel_path).as_str())?;
                f.write_str(match self {
                    Self::Empty(_) => "The file is empty or very short (less than 10 characters)",
                    Self::NoClass(_) => "The file isn't export correct class. The class must be exported as default and must implement IController",
                    Self::InvalidSyntax(_, message) => &message,
                    _ => ""
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum HTTPMethod {
    GET,
    POST,
    DELETE,
    PATCH,
    OPTIONS,
    ANY,
}

impl HTTPMethod {
    pub fn from_string(value: String) -> Option<Self> {
        match value.to_uppercase().as_str() {
            "GET" => Some(Self::GET),
            "POST" => Some(Self::POST),
            "DELETE" => Some(Self::DELETE),
            "PATCH" => Some(Self::PATCH),
            "OPTIONS" => Some(Self::OPTIONS),
            "ANY" => Some(Self::ANY),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpHandler {
    pub method: HTTPMethod,
    pub body: String,
    pub req_param: Option<String>,
}

/*
 * export default class _ implements IController {
 * ^ Class definition
 *   GET      (reqParam: TYPE): TYPE {
 *   ^ METHOD  ^ req_param
 *     // ..
 *     ^ BODY
 *   }
 * }
 */

pub const REQ_PARAM: &str = "__req_param__";

// https://regex101.com/r/Bw126K/1
const CLASS_REGEX: &str =
    r"(?m)^\s*export\s+default\s+class\s+(.+)\simplements\s+(.*)IController\s+\{\s*";

const METHOD_REGEX: &str = r"GET|POST|DELETE|PATCH|OPTIONS|ANY";

// https://regex101.com/r/FEA6Zz/2
const HANDLER_REGEX: &str =
    r"\(\s*(?:([a-zA-Z0-9_]+)(?::\s*(?:\w+)\s*)?)?\)(?:\s*:\s*[^{]+\s*)?\s*\{";

pub fn http_parse(
    content: String,
    file_path: String,
) -> Result<RefCell<Vec<HttpHandler>>, HttpParseError> {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => return Err(HttpParseError::CurrentDir(err)),
    };
    let rel_path = diff_paths(&file_path, cwd).unwrap();
    let rel_path = rel_path.display().to_string();
    if content.len() <= 10 {
        return Err(HttpParseError::Empty(rel_path));
    }

    let class_regex = Regex::new(CLASS_REGEX).unwrap();
    let class_col_index = class_regex.find(&content);
    let class_col_index = match class_col_index {
        Some(find) => find,
        None => return Err(HttpParseError::NoClass(rel_path)),
    };
    drop(class_regex);

    let handlers: RefCell<Vec<HttpHandler>> = RefCell::new(Vec::new());
    let content = (&content[class_col_index.end()..]).to_string();

    let method_regex = Regex::new(METHOD_REGEX).unwrap();
    let handler_regex = Regex::new(HANDLER_REGEX).unwrap();

    let next_handler = recur_fn(|next_handler, remain: String| -> Option<HttpParseError> {
        // Empty file
        if remain.len() <= 1 {
            return None;
        }

        let method = method_regex.find(&remain);
        let method = match method {
            Some(method) => method,
            None => return None,
        };

        let (method, handler_idx) = (
            remain[method.start()..method.end()].to_string(),
            method.end(),
        );

        let remain = remain[handler_idx..].to_string();

        let handler = handler_regex.captures(&remain);
        let handler = match handler {
            Some(handler) => handler,
            None => {
                return Some(HttpParseError::InvalidSyntax(
                    rel_path.clone(),
                    format!("Method {} doesn't have handler", method),
                ))
            }
        };

        let req_param = handler
            .get(1)
            .map(|req_param| req_param.as_str().to_string());

        let body_idx = handler.get(0).unwrap().end();
        let remain = remain[body_idx..].trim().to_string();

        let mut brace_count = 1;
        let mut tmp_remain = remain.clone();
        let mut length = 0;

        while brace_count > 0 {
            let near_close_bracket = tmp_remain.find('}');
            let near_open_bracket = tmp_remain.find('{');

            // If 'close_bracket' is more close then substract one to braceCount
            // Open = None, Close = 0..
            // else, add one to 'brace_count'
            // Open = 0.., Close = None..
            let (delta, bracket_pos) = match (near_close_bracket, near_open_bracket) {
                (Some(near_close_bracket), None) => (-1, near_close_bracket),
                (None, Some(near_open_bracket)) => (1, near_open_bracket),
                // Win the most near
                (Some(near_close_bracket), Some(near_open_bracket)) => {
                    if near_open_bracket > near_close_bracket {
                        (-1, near_close_bracket)
                    } else {
                        (1, near_open_bracket)
                    }
                }
                // Both can't be none
                (None, None) => {
                    return Some(HttpParseError::InvalidSyntax(
                        rel_path.clone(),
                        "".to_string(),
                    ));
                }
            };
            brace_count += delta;
            length += bracket_pos + 1;
            tmp_remain = tmp_remain[(bracket_pos + 1)..].to_string();
        }
        // Set variable only if it's different to "req"
        let req_decl = if let Some(req_param) = req_param.clone() {
            if req_param.as_str() == REQ_PARAM {
                "".to_string()
            } else {
                format!("let {}: $Densky$.HTTPRequest = {};\n", req_param, REQ_PARAM)
            }
        } else {
            "".to_string()
        };

        let end = req_decl + remain[0..(length - 1)].trim();

        handlers.borrow_mut().push(HttpHandler {
            method: HTTPMethod::from_string(method).unwrap(),
            body: end,
            req_param,
        });

        next_handler(remain[length..].to_string())
    });

    match next_handler.call(content) {
        Some(err) => Err(err),
        None => {
            if handlers.borrow().len() == 0 {
                Err(HttpParseError::Empty(rel_path))
            } else {
                Ok(handlers)
            }
        }
    }
}

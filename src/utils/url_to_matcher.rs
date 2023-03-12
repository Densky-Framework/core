pub const PREPARE_PREFIX: &str = "__matcher_prepare_";
pub const SERIAL_PREFIX: &str = "__matcher_serial_";

#[derive(Debug, Clone)]
pub enum UrlMatcherSegment {
    Static(String),
    Var(String),
}

impl UrlMatcherSegment {
    pub fn is_static(&self) -> bool {
        match self {
            Self::Static(_) => true,
            _ => false,
        }
    }

    pub fn is_var(&self) -> bool {
        match self {
            Self::Var(_) => true,
            _ => false,
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{ raw: \"{}\", isVar: {}{} }}",
            match self {
                Self::Static(raw) => raw.clone(),
                Self::Var(varname) => format!("${}", varname),
            },
            self.is_var(),
            match self {
                Self::Static(_) => "".to_owned(),
                Self::Var(varname) => format!(", varname: \"{}\"", varname),
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct UrlMatcher {
    pub target_name: String,
    pub url: String,
    pub segments: Vec<UrlMatcherSegment>,
    pub has_variables: bool,
}

impl UrlMatcher {
    pub fn new(target_name: String, url: String) -> UrlMatcher {
        let segments: Vec<UrlMatcherSegment> = url
            .split('/')
            .map(|segment| {
                if &segment[0..1] == "$" {
                    UrlMatcherSegment::Var(segment[1..].to_string())
                } else {
                    UrlMatcherSegment::Static(segment.to_string())
                }
            })
            .collect();

        let has_variables = segments.iter().find(|segment| segment.is_var()).is_some();

        UrlMatcher {
            target_name,
            url,
            segments,
            has_variables,
        }
    }

    pub fn exact_decl(&self, param: Option<String>) -> String {
        if self.has_variables {
            format!(
                "((t, p, m, n) => {{\
          if (t.length !== p.length) return false;\
          const s = t.every((tp,i) => {{\
            if (!p[i]) return false;\
            if (p[i].isVar) {{\
              n.set(p[i].varname,tp);\
              return true;\
            }} \
            if (p[i].raw === tp) return true;\
            return false;\
          }});\
          if (s) [...n.entries()].forEach(([k, v]) => m.set(k, v));\
          return s;\
        }})({2}{0}, {3}{0}, {1}, new Map())",
                self.target_name,
                param.unwrap(),
                PREPARE_PREFIX,
                SERIAL_PREFIX
            )
        } else {
            format!("{}{} == '{}'", PREPARE_PREFIX, self.target_name, self.url)
        }
    }

    pub fn start_decl(&self, param: Option<String>) -> String {
        if self.has_variables {
            format!(
                "((t, p, m, n) => {{\
          if (t.length < p.length) return false;\
          const s = p.every((tp,i) => {{\
            if (!t[i]) return false;\
            if (tp.isVar) {{\
              n.set(tp.varname, p[i]);\
              return true;\
            }} \
            if (tp.raw === p[i]) return true;\
            return false;\
          }});\
          if (s) [...n.entries()].forEach(([k, v]) => m.set(k, v));\
          return s;\
        }})({2}{0}, {3}{0}, {1}, new Map())",
                self.target_name,
                param.unwrap(),
                PREPARE_PREFIX,
                SERIAL_PREFIX
            )
        } else {
            format!(
                "{}{}.startsWith('{}')",
                PREPARE_PREFIX, self.target_name, self.url
            )
        }
    }

    pub fn prepare_decl(&self, val: String) -> String {
        format!(
            "const {}{} = {}.__accumulator__.{}",
            PREPARE_PREFIX,
            self.target_name,
            val,
            if self.has_variables {
                "segments"
            } else {
                "path"
            }
        )
    }

    pub fn serial_decl(&self) -> String {
        if !self.has_variables {
            "".to_string()
        } else {
            let mut serialized = "[".to_string();
            for segment in &self.segments {
                serialized += &segment.to_json();
                serialized += ",";
            }
            serialized.pop();
            serialized += "]";
            format!(
                "const {}{} = {};",
                SERIAL_PREFIX, self.target_name, serialized
            )
        }
    }
}

pub const PREPARE_PREFIX: &str = "__matcher_prepare_";
pub const SERIAL_PREFIX: &str = "__matcher_serial_";
pub const MATCHER_PREFIX: &str = "__matcher_matcher_";

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
                if segment.len() == 0 {
                    UrlMatcherSegment::Static(segment.to_string())
                } else if &segment[0..1] == "$" {
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
                "{4}EXACT({2}{0}, {3}{0}, {1}, new Map())",
                self.target_name,
                param.unwrap(),
                PREPARE_PREFIX,
                SERIAL_PREFIX,
                MATCHER_PREFIX
            )
        } else {
            format!("{}{} === '{}'", PREPARE_PREFIX, self.target_name, self.url)
        }
    }

    pub fn start_decl(&self, param: Option<String>) -> String {
        if self.has_variables {
            format!(
                "{4}START({2}{0}, {3}{0}, {1}, new Map())",
                self.target_name,
                param.unwrap(),
                PREPARE_PREFIX,
                SERIAL_PREFIX,
                MATCHER_PREFIX
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

    pub fn update_decl(&self, val: String) -> String {
        let accumulator = format!("{}.__accumulator__", &val);
        let corrector = if self.url == "/" { 1 } else { 0 };
        let segments_code = format!(
            "{0}.segments = {0}.segments.slice({1});",
            &accumulator,
            self.segments.len() - corrector
        );
        if self.has_variables {
            format!(
                "{{
  const my = {1}.path.split(\"/\").slice(0, {2});
  {0}
  {1}.path = {1}.path.slice(my.join(\"/\").length + 1)
}}",
                segments_code,
                accumulator,
                self.segments.len()
            )
        } else {
            format!(
                "{0}\n{1}.path = {1}.path.slice({2})",
                segments_code,
                accumulator,
                self.url.len() + 1 - corrector
            )
        }
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
                "{}\n{}\n{}",
                format!(
                    "const {}{} = {};",
                    SERIAL_PREFIX, self.target_name, serialized
                ),
                format!(
                    "const {}EXACT = (target, serial, resultMap, paramMap) => {{
  if (target.length !== serial.length) return false;

  for (let i = 0; i < target.length; i++) {{
    const targetParam = target[i];
    const serialParam = serial[i];

    if (serialParam.isVar) {{
      paramMap.set(serialParam.varname, targetParam);
    }} else if (serialParam.raw !== targetParam) {{
      return false;
    }}
  }}

  for (const [key, value] of paramMap) {{
    resultMap.set(key, value);
  }}

  return true;
}};
",
                    MATCHER_PREFIX
                ),
                format!(
                    "const {}START = (target, serial, resultMap, paramMap) => {{
  if (target.length < serial.length) return false;

  for (let i = 0; i < serial.length; i++) {{
    if (!target[i]) return false;

    const serialParam = serial[i];
    const targetParam = target[i];

    if (serialParam.isVar) {{
      paramMap.set(serialParam.varname, targetParam);
    }} else if (serialParam.raw !== targetParam) {{
      return false;
    }}
  }}

  for (const [key, value] of paramMap.entries()) {{
    resultMap.set(key, value);
  }}

  return true;
}}",
                    MATCHER_PREFIX
                )
            )
        }
    }
}

pub const PREPARE_PREFIX: &str = "__matcher_prepare";
pub const SERIAL_PREFIX: &str = "__matcher_serial";
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
    pub url: String,
    pub segments: Vec<UrlMatcherSegment>,
    pub has_variables: bool,
}

impl UrlMatcher {
    pub fn new(url: String) -> UrlMatcher {
        let mut has_variables = false;
        let segments: Vec<UrlMatcherSegment> = url
            .split('/')
            .map(|segment| {
                if segment.len() == 0 {
                    UrlMatcherSegment::Static(segment.to_string())
                } else if &segment[0..1] == "$" {
                    has_variables = true;
                    UrlMatcherSegment::Var(segment[1..].to_string())
                } else {
                    UrlMatcherSegment::Static(segment.to_string())
                }
            })
            .collect();

        UrlMatcher {
            url,
            segments,
            has_variables,
        }
    }

    pub fn exact_decl<V>(&self, val: V) -> String
    where
        V: AsRef<str>,
    {
        format!("{}.__accumulator__.segments.length === 0", val.as_ref())
        // if self.has_variables {
        //     format!(
        //         "{4}EXACT({2}{0}, {3}{0}, {1}, new Map())",
        //         self.target_name,
        //         param.unwrap(),
        //         PREPARE_PREFIX,
        //         SERIAL_PREFIX,
        //         MATCHER_PREFIX
        //     )
        // } else {
        //     format!("{}{} === '{}'", PREPARE_PREFIX, self.target_name, self.url)
        // }
    }

    pub fn start_decl<V>(&self, req: V) -> String
    where
        V: AsRef<str>,
    {
        let req = req.as_ref();
        if self.has_variables {
            format!(
                "{1}START({0}.__accumulator__.segments, {2}, {0}.params, new Map())",
                req, MATCHER_PREFIX, SERIAL_PREFIX,
            )
        } else {
            format!("{}.__accumulator__.path.startsWith('{}')", req, self.url)
        }
    }

    pub fn update_decl<V>(&self, val: V) -> String
    where
        V: AsRef<str>,
    {
        let accumulator = format!("{}.__accumulator__", val.as_ref());
        let corrector = if self.url == "/" { 1 } else { 0 };
        format!(
            "{0}.segments = {0}.segments.slice({1});
{0}.path = {0}.segments.join(\"/\");",
            accumulator,
            self.segments.len() - corrector
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
                "{}\n{}\n{}",
                format!("const {} = {};", SERIAL_PREFIX, serialized),
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

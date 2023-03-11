pub struct CompileOptions {
    pub verbose: bool,
}

pub struct CompileContext {
    pub output_dir: String,
    pub static_path: String,
    pub static_prefix: String,
    pub routes_path: String,
    pub views_path: String,
    pub verbose: bool,
}

#[derive(Clone, Debug)]
pub struct WalkOptions {
    pub follow_links: bool,
    pub max_depth: usize,
    pub ignore_errors: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            follow_links: false,
            max_depth: 512,
            ignore_errors: false,
        }
    }
}

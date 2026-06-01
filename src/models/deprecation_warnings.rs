#[derive(Clone, Debug)]
pub struct DeprecationConfig {
    pub deprecated_versions: Vec<u32>,
    pub sunset_date: Option<String>,
}

impl Default for DeprecationConfig {
    fn default() -> Self {
        Self {
            deprecated_versions: vec![],
            sunset_date: None,
        }
    }
}

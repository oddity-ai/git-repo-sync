#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    name: String,
}

impl Host {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}

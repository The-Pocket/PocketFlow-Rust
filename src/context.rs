use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct Context {
    data: HashMap<String, Value>,
    metadata: HashMap<String, Value>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn from_data(data: HashMap<String, Value>) -> Self {
        Self {
            data,
            metadata: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.data.insert(key.to_string(), value);
    }

    pub fn set_metadata(&mut self, key: &str, value: Value) {
        self.metadata.insert(key.to_string(), value);
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    pub fn remove_metadata(&mut self, key: &str) -> Option<Value> {
        self.metadata.remove(key)
    }

    pub fn get_all_data(&self) -> &HashMap<String, Value> {
        &self.data
    }

    pub fn get_all_metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    pub fn merge(&mut self, other: &Context) {
        for (key, value) in &other.data {
            self.data.insert(key.clone(), value.clone());
        }
        for (key, value) in &other.metadata {
            self.metadata.insert(key.clone(), value.clone());
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.metadata.clear();
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn contains_metadata_key(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Context {{")?;

        // Display data
        writeln!(f, "  data: {{")?;
        for (key, value) in &self.data {
            writeln!(f, "    \"{}\": {},", key, value)?;
        }
        writeln!(f, "  }},")?;

        // Display metadata
        writeln!(f, "  metadata: {{")?;
        for (key, value) in &self.metadata {
            writeln!(f, "    \"{}\": {},", key, value)?;
        }
        writeln!(f, "  }}")?;

        write!(f, "}}")
    }
}

impl From<HashMap<String, Value>> for Context {
    fn from(data: HashMap<String, Value>) -> Self {
        Self::from_data(data)
    }
}

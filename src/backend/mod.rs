pub mod from_sys;

use std::collections::HashMap;

use serde_json::Value as JsonValue;

pub trait Backend {
    type Script;
    type Error;

    fn run_scripts(
        &self,
        script: Vec<Self::Script>,
        data: HashMap<String, JsonValue>,
    ) -> Result<Vec<Values>, Self::Error>;

    fn run_script(
        &self,
        script: Self::Script,
        data: HashMap<String, JsonValue>,
    ) -> Result<Values, Self::Error>;

    fn run(
        &mut self,
        script: Self::Script,
        data: HashMap<String, JsonValue>,
    ) -> Result<Values, Self::Error>;

    fn clear(&mut self) -> Result<(), Self::Error>;

    fn get_data(&self) -> Result<Values, Self::Error>;
}
pub struct Values {
    json_values: HashMap<String, JsonValue>,
    result_name: String,
}

impl Values {
    pub fn new(json_values: HashMap<String, JsonValue>, result_name: String) -> Self {
        Self {
            json_values,
            result_name,
        }
    }

    pub fn get_values(&self) -> &HashMap<String, JsonValue> {
        &self.json_values
    }

    pub fn get_result(&self) -> JsonValue {
        self.json_values
            .get(&self.result_name)
            .cloned()
            .unwrap_or(JsonValue::Null)
    }
}

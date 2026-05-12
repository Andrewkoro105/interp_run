use std::collections::HashSet;

pub trait Parser {
    fn get_use_fns(&self, script: String) -> Result<HashSet<String>, Box<dyn ToString>>;
}

#[derive(Debug)]
pub enum InspectorError {
    ParsingError(String),
    UseUnsafeFunctions(Vec<String>),
}

pub struct MatParser {}

pub struct ScriptInspector {
    pub restricted_functions: HashSet<String>,
    pub parser: Box<dyn Parser>,
}

pub struct Script(String);

impl ScriptInspector {
    pub fn to_string(&self, script: Script) -> Result<String, InspectorError> {
        if !self.restricted_functions.is_empty() {
            let use_unsafe_functions = self
                .restricted_functions
                .intersection(
                    &self
                        .parser
                        .get_use_fns(script.0.clone())
                        .map_err(|err| err.to_string())
                        .map_err(InspectorError::ParsingError)?,
                )
                .cloned()
                .collect::<Vec<_>>();

            if !use_unsafe_functions.is_empty() {
                return Err(InspectorError::UseUnsafeFunctions(use_unsafe_functions));
            }
        }
        Ok(script.0)
    }
}

impl From<String> for Script {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Parser for MatParser {
    fn get_use_fns(&self, script: String) -> Result<HashSet<String>, Box<dyn ToString>> {
        todo!()
    }
}

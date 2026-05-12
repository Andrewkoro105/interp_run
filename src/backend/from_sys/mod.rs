pub mod script;

use script::{Script, ScriptInspector};
use serde_json::Value as JsonValue;
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader, Read, Write},
    process::{ChildStdout, Command, Stdio},
};
use tracing::debug;

use crate::backend::from_sys::script::{InspectorError, MatParser};

#[derive(Debug)]
pub enum FromSysError {
    Inspector(InspectorError),
    Io(std::io::Error),
}

pub struct FromSys {
    pub base_command: String,
    pub print_value_pattern: String,
    pub input_value_pattern: String,
    pub script_inspector: ScriptInspector,
}

impl super::Backend for FromSys {
    type Script = Script;
    type Error = FromSysError;

    fn run_scripts(
        &self,
        script: Vec<Self::Script>,
        data: HashMap<String, JsonValue>,
    ) -> Result<Vec<super::Values>, Self::Error> {
        todo!()
    }

    fn run_script(
        &self,
        script: Self::Script,
        data: HashMap<String, JsonValue>,
    ) -> Result<super::Values, Self::Error> {
        let script: String = self.get_script(script, &data)?;
        debug!("{script}");

        let mut child = Command::new(&self.base_command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(FromSysError::Io)?;

        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();

        stdin
            .write_all(script.as_bytes())
            .map_err(FromSysError::Io)?;
        stdin.write_all("\n".as_bytes()).map_err(FromSysError::Io)?;

        stdin.flush().map_err(FromSysError::Io)?;

        self.get_data(&mut stdout)
    }

    fn run(
        &mut self,
        script: Self::Script,
        data: HashMap<String, JsonValue>,
    ) -> Result<super::Values, Self::Error> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        todo!()
    }

    fn get_data(&self) -> Result<super::Values, Self::Error> {
        todo!()
    }
}

impl FromSys {
    pub fn new_matlab_like(name: impl ToString) -> Self {
        let start_out_block = Self::get_start_out_block();
        let end_out_block = Self::get_end_out_block();
        Self {
            base_command: name.to_string(),
            print_value_pattern: format!(
                r#"
printf("{start_out_block}")
vars = whos;

allVars = struct();

for k = 1:length(vars)
    name = vars(k).name;
    value = evalin('caller', name);
    
    if isnumeric(value) || islogical(value) || ischar(value) || isstring(value) || ...
       isstruct(value) || iscell(value)
        if ndims(value) > 2
            allVars.(name) = struct('data', value(:), 'size', size(value), 'class', class(value));
        else
            allVars.(name) = value;
        end
        
    elseif istable(value)
        allVars.(name) = table2struct(value);
        
    elseif isdatetime(value) || isduration(value) || iscategorical(value)
        allVars.(name) = string(value);
        
    elseif iscomplex(value)
        allVars.(name) = struct('real', real(value), 'imag', imag(value));
        
    elseif issparse(value)
        [i,j,s] = find(value);
        allVars.(name) = struct('i', i, 'j', j, 'value', s, 'size', size(value));
    else
        allVars.(name) = sprintf('<%s>', class(value));
    end
end

jsonString = jsonencode(allVars, 'PrettyPrint', true);
disp(jsonString);

printf("{end_out_block}")
            "#
            ),
            input_value_pattern: "input_data = jsondecode({});".to_string(),
            script_inspector: ScriptInspector {
                restricted_functions: HashSet::new(),
                parser: Box::new(MatParser {}) as _,
            },
        }
    }

    fn get_script(
        &self,
        script: Script,
        data: &HashMap<String, JsonValue>,
    ) -> Result<String, FromSysError> {
        let mut base_script = self
            .script_inspector
            .to_string(script)
            .map_err(FromSysError::Inspector)?;
        if !base_script.is_empty() {
            let pos = base_script.rfind('\n').map_or(0, |p| p + 1);
            base_script.insert_str(pos, &format!("{} = ", self.get_result_name()));
        }
        base_script = format!(
            r"
{}
{base_script}
{}
",
            self.input_value_pattern.replace(
                "{}",
                &format!("\"{}\"", serde_json::to_string(&data).unwrap().replace("\"", "\\\""))
            ),
            self.print_value_pattern,
        );
        Ok(base_script)
    }

    fn get_data(&self, stdout: &mut ChildStdout) -> Result<super::Values, FromSysError> {
        let start_marker = Self::get_start_out_block().replace("\\n", "\n");
        let end_marker = Self::get_end_out_block().replace("\\n", "\n");

        let mut buf_reader = BufReader::new(stdout);
        let mut out = String::new();
        loop {
            let mut line = String::new();
            buf_reader.read_line(&mut line).map_err(FromSysError::Io)?;
            out = format!("{out}\n{line}");

            let start_idx = out.rfind(&start_marker);
            if let Some(start_idx) = start_idx {
                let end_idx = out.rfind(&end_marker);

                if let Some(end_idx) = end_idx {
                    let slice_start = start_idx + start_marker.len();
                    let slice_end = end_idx;

                    if slice_start <= slice_end
                        && out.is_char_boundary(slice_start)
                        && out.is_char_boundary(slice_end)
                    {
                        break Ok(super::Values::new(
                            serde_json::from_slice(out[slice_start..slice_end].as_bytes()).unwrap(),
                            self.get_result_name(),
                        ));
                    }
                }
            }
        }
    }

    fn get_start_out_block() -> String {
        let project_name = env!("CARGO_PKG_NAME");
        format!("[{project_name} output bloc] start\\n")
    }

    fn get_end_out_block() -> String {
        let project_name = env!("CARGO_PKG_NAME");
        format!("[{project_name} output bloc] end\\n")
    }

    fn get_result_name(&self) -> String {
        format!("{}_{}_result", env!("CARGO_PKG_NAME"), self.base_command)
    }
}

#[cfg(test)]
mod test {
    use serde_json::Number;

    use crate::backend::Backend;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn simle() {
        let mut data = HashMap::new();
        data.insert("test_value".to_string(), JsonValue::Number(Number::from_u128(42).unwrap()));

        let script_result = FromSys::new_matlab_like("octave")
            .run_script("input_data.test_value ^ 2".to_string().into(), data)
            .unwrap()
            .get_result()
            .as_number()
            .unwrap()
            .as_u128()
            .unwrap();

        assert_eq!(script_result, 1764);
    }
}

use super::runtime::Bindings;
use super::tables::Tables;
use super::{FaultKind, Value};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default)]
pub struct Library;

fn bytes(value: &[u8]) -> Value {
    Value::Bytes(Arc::from(value))
}

fn argument(arguments: &[Value], index: usize) -> Result<&Value, FaultKind> {
    arguments.get(index).ok_or(FaultKind::Argument(index + 1))
}

fn table(arguments: &[Value], index: usize) -> Result<&Value, FaultKind> {
    match argument(arguments, index)? {
        value @ Value::Reference { kind: 5, .. } => Ok(value),
        _ => Err(FaultKind::Argument(index + 1)),
    }
}

impl Library {
    pub fn install(tables: &mut Tables) -> Result<Self, FaultKind> {
        let globals = tables.globals();
        for (index, name) in [
            b"type".as_slice(),
            b"rawget",
            b"rawset",
            b"getmetatable",
            b"setmetatable",
            b"assert",
        ]
        .iter()
        .enumerate()
        {
            tables.raw_set(
                &globals,
                &bytes(name),
                Value::Reference {
                    kind: 10,
                    identity: index as u32 + 1,
                },
            )?;
        }
        Ok(Self)
    }

    fn type_name(value: &Value) -> Result<Value, FaultKind> {
        let name: &[u8] = match value {
            Value::Nil => b"nil",
            Value::Boolean(_) => b"boolean",
            Value::Number(_) => b"number",
            Value::Bytes(_) => b"string",
            Value::Reference { kind: 2 | 7, .. } => b"userdata",
            Value::Reference { kind: 5, .. } => b"table",
            Value::Reference {
                kind: 6 | 9 | 10, ..
            } => b"function",
            Value::Reference { kind: 8, .. } => b"thread",
            Value::Reference { kind: 11, .. } => b"ui64",
            Value::Reference { kind: 12, .. } => b"struct",
            Value::Reference { kind: 15, .. } => b"object",
            _ => return Err(FaultKind::BindingRequired),
        };
        Ok(bytes(name))
    }

    fn protected(tables: &Tables, metatable: &Value) -> Result<Value, FaultKind> {
        tables.raw_get(metatable, &bytes(b"__metatable"))
    }
}

impl Bindings for Library {
    fn call(
        &mut self,
        function: u32,
        arguments: &[Value],
        tables: &mut Tables,
    ) -> Result<Vec<Value>, FaultKind> {
        match function {
            1 => Ok(vec![Self::type_name(
                arguments.first().unwrap_or(&Value::Nil),
            )?]),
            2 => Ok(vec![
                tables.raw_get(table(arguments, 0)?, argument(arguments, 1)?)?,
            ]),
            3 => {
                tables.raw_set(
                    table(arguments, 0)?,
                    argument(arguments, 1)?,
                    argument(arguments, 2)?.clone(),
                )?;
                Ok(Vec::new())
            }
            4 => {
                let container = argument(arguments, 0)?;
                let Some(metatable) = tables.metatable(container)? else {
                    return Ok(vec![Value::Nil]);
                };
                let protected = Self::protected(tables, &metatable)?;
                Ok(vec![if matches!(protected, Value::Nil) {
                    metatable
                } else {
                    protected
                }])
            }
            5 => {
                let container = table(arguments, 0)?;
                let metatable = match argument(arguments, 1)? {
                    Value::Nil => None,
                    value @ Value::Reference { kind: 5, .. } => Some(value.clone()),
                    _ => return Err(FaultKind::Argument(2)),
                };
                if let Some(current) = tables.metatable(container)?
                    && !matches!(Self::protected(tables, &current)?, Value::Nil)
                {
                    return Err(FaultKind::ProtectedMetatable);
                }
                tables.set_metatable(container, metatable)?;
                Ok(vec![container.clone()])
            }
            6 => {
                let condition = argument(arguments, 0)?;
                let message = match arguments.get(1) {
                    None | Some(Value::Nil) => b"assertion failed!".as_slice(),
                    Some(Value::Bytes(value)) => value.as_ref(),
                    _ => return Err(FaultKind::BindingRequired),
                };
                if condition.truthy() {
                    Ok(arguments.to_vec())
                } else {
                    Err(FaultKind::ScriptError(message.to_vec()))
                }
            }
            _ => Err(FaultKind::BindingRequired),
        }
    }
}

//! Black Ops 2's builtins, each as its handler in the executable reads its
//! arguments and answers. Where the handler reads engine state IW4L keeps
//! differently, the choice is listed in `docs/fidelity/t6.md`.

use super::super::args::{arg, kind};
use crate::script::{Namespace, NativeRegistry, RoundScript, Value};
use bevy_ecs::prelude::{Resource, World};

/// How Black Ops 2 finds a dvar: djb2 over the lowercased name.
fn dvar_hash(name: &str) -> u32 {
    name.bytes().fold(5381u32, |hash, byte| {
        hash.wrapping_mul(33)
            .wrapping_add(u32::from(byte.to_ascii_lowercase()))
    })
}

/// The value of the dvar an argument names, a hash (an int is taken as one)
/// or a name; a missing dvar reads as empty.
fn dvar(world: &World, args: &[Value]) -> Result<String, String> {
    let dvars = &world.resource::<crate::script::MatchScript>().dvars;
    let found = match arg(args, 0)? {
        Value::Int(hash) => {
            let hash = *hash as u32;
            dvars
                .iter()
                .find(|(name, _)| dvar_hash(name) == hash)
                .map(|(_, value)| value.clone())
        }
        Value::String(name) => dvars.get(&name.to_string().to_ascii_lowercase()).cloned(),
        other => return Err(format!("{} is not a dvar name", kind(other))),
    };
    Ok(found.unwrap_or_else(String::new))
}

/// `atoi`: the leading decimal integer, 0 when there is none.
fn atoi(text: &str) -> i32 {
    let text = text.trim_start();
    let (sign, digits) = match text.as_bytes().first() {
        Some(b'-') => (-1i64, &text[1..]),
        Some(b'+') => (1, &text[1..]),
        _ => (1, text),
    };
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i64, |n, d| {
            (n * 10 + i64::from(d - b'0')).min(i64::from(u32::MAX))
        });
    (sign * value) as i32
}

/// `atof`: the leading decimal number, 0 when there is none.
fn atof(text: &str) -> f32 {
    let text = text.trim_start();
    let end = text
        .char_indices()
        .take_while(|&(i, c)| c.is_ascii_digit() || c == '.' || (i == 0 && (c == '-' || c == '+')))
        .last()
        .map_or(0, |(i, c)| i + c.len_utf8());
    text[..end].parse().unwrap_or(0.0)
}

/// A builtin's int parameter: only an int is one.
fn int(args: &[Value], index: usize) -> Result<i32, String> {
    match arg(args, index)? {
        Value::Int(n) => Ok(*n),
        other => Err(format!("type {} is not an int", kind(other))),
    }
}

/// A builtin's string parameter: a string or a localized string.
fn string(args: &[Value], index: usize) -> Result<String, String> {
    match arg(args, index)? {
        Value::String(text) => Ok(text.to_string()),
        Value::LocalizedString(text) => Ok(text.to_string()),
        other => Err(format!("type {} is not a string", kind(other))),
    }
}

/// The session and game mode of the solo Classic match the owner started in
/// Black Ops 2: an online public session (`docs/fidelity/t6.md`).
const SESSION_FLAGS: u32 = 0x14;
const GAME_MODE_FLAGS: u32 = 0x1;

/// The names `clientsysregister` gave out, in order; the engine keeps eight.
#[derive(Resource, Default)]
struct ClientSystems(Vec<String>);

pub(crate) fn register(registry: &mut NativeRegistry) {
    use Namespace::Function;

    registry.register(Function, "isdefined", |world, _, args| {
        let defined = match arg(args, 0)? {
            Value::Undefined => false,
            Value::Object(id) => world.resource::<RoundScript>().script_is_defined(id),
            _ => true,
        };
        Ok(Value::Int(defined.into()))
    });
    registry.register(Function, "gettime", |world, _, _| {
        let tick = world.resource::<crate::step::StepRequest>().tick;
        let ms = u64::from(tick.0) * u64::from(crate::MATCH_TICK_MS);
        i32::try_from(ms)
            .map(Value::Int)
            .map_err(|_| "gettime overflow".into())
    });

    // The GetDvar opcodes: a missing dvar reads as "" (0, 0.0).
    registry.register(Function, "getdvar", |world, _, args| {
        Ok(Value::String(dvar(world, args)?.into()))
    });
    registry.register(Function, "getdvarint", |world, _, args| {
        Ok(Value::Int(atoi(&dvar(world, args)?)))
    });
    registry.register(Function, "getdvarfloat", |world, _, args| {
        Ok(Value::Float(atof(&dvar(world, args)?)))
    });
    registry.register(Function, "setdvar", |world, _, args| {
        let name = string(args, 0)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("Dvar {name} has an invalid dvar name"));
        }
        let value = match arg(args, 1)? {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                crate::script::runtime::to_text(&Value::Float(*f)).ok_or("float as text")?
            }
            _ => string(args, 1)?,
        };
        crate::script::set_dvar(world, &name, &value);
        Ok(Value::Undefined)
    });

    // Session and game mode flags.
    registry.register(Function, "sessionmodeisonlinegame", |_, _, _| {
        Ok(Value::Int((SESSION_FLAGS >> 2 & 1) as i32))
    });
    registry.register(Function, "sessionmodeissystemlink", |_, _, _| {
        Ok(Value::Int((SESSION_FLAGS >> 1 & 1) as i32))
    });
    registry.register(Function, "sessionmodeisprivate", |_, _, _| {
        Ok(Value::Int((SESSION_FLAGS >> 3 & 1) as i32))
    });
    registry.register(Function, "sessionmodeisprivateonlinegame", |_, _, _| {
        Ok(Value::Int(
            (SESSION_FLAGS >> 2 & SESSION_FLAGS >> 3 & 1) as i32,
        ))
    });
    registry.register(Function, "sessionmodeiszombiesgame", |_, _, _| {
        Ok(Value::Int(1))
    });
    registry.register(Function, "gamemodeisusingxp", |_, _, _| {
        Ok(Value::Int((GAME_MODE_FLAGS & 1) as i32))
    });
    registry.register(Function, "gamemodeisusingstats", |_, _, _| {
        Ok(Value::Int((GAME_MODE_FLAGS & 0x41 != 0).into()))
    });
    registry.register(Function, "gamemodeismode", |_, _, args| {
        let mode = int(args, 0)?;
        if !(0..=7).contains(&mode) {
            return Err(format!("Bad game mode {mode}"));
        }
        Ok(Value::Int((GAME_MODE_FLAGS >> mode & 1) as i32))
    });

    registry.register(Function, "getaitriggerflags", |_, _, _| Ok(Value::Int(7)));
    registry.register(Function, "getvehicletriggerflags", |_, _, _| {
        Ok(Value::Int(0x10))
    });

    registry.register(Function, "clientsysregister", |world, _, args| {
        let name = string(args, 0)?;
        let systems = &mut world.get_resource_or_insert_with(ClientSystems::default).0;
        let index = match systems.iter().position(|known| *known == name) {
            Some(index) => index,
            None if systems.len() < 8 => {
                systems.push(name);
                systems.len() - 1
            }
            None => return Ok(Value::Int(-1)),
        };
        Ok(Value::Int(index as i32))
    });
}

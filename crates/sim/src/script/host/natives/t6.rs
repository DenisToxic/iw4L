//! Black Ops 2's builtins, each as its handler in the executable reads its
//! arguments and answers. Where the handler reads engine state IW4L keeps
//! differently, the choice is listed in `docs/fidelity/t6.md`.

use super::super::args::arg;
use super::super::arrays::new_array;
use super::super::entities::EntityKind;
use crate::script::{ArrayKey, Namespace, NativeRegistry, RoundScript, Value};
use bevy_ecs::prelude::{Resource, World};
use std::collections::BTreeMap;

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
        other => return Err(format!("{} is not a dvar name", type_name(other))),
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

/// Black Ops 2's name for a value's type.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Undefined => "undefined",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::LocalizedString(_) => "localized string",
        Value::Vector(_) => "vector",
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::Function(_) | Value::Builtin(_) => "function",
        Value::Animation { .. } | Value::AnimationTree(_) => "animation",
    }
}

/// A builtin's int parameter: only an int is one.
fn int(args: &[Value], index: usize) -> Result<i32, String> {
    match arg(args, index)? {
        Value::Int(n) => Ok(*n),
        other => Err(format!("type {} is not an int", type_name(other))),
    }
}

/// A builtin's string parameter: a string, or a number or vector as text.
fn string(args: &[Value], index: usize) -> Result<String, String> {
    match arg(args, index)? {
        Value::String(text) => Ok(text.to_string()),
        other => crate::script::runtime::to_text(other)
            .ok_or_else(|| format!("cannot cast {} to string", type_name(other))),
    }
}

/// A builtin's localized string parameter: only a localized string is one.
fn istring(args: &[Value], index: usize) -> Result<String, String> {
    match arg(args, index)? {
        Value::LocalizedString(text) => Ok(text.to_string()),
        other => Err(format!(
            "type {} is not a localized string",
            type_name(other)
        )),
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
    use Namespace::{Function, Method};

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
            Value::LocalizedString(text) => text.to_string(),
            _ => string(args, 1)?,
        };
        crate::script::set_dvar(world, &name, &value.replace('"', "'"));
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

    registry.register(Function, "isstring", |_, _, args| {
        Ok(Value::Int(matches!(arg(args, 0)?, Value::String(_)).into()))
    });
    // Split screen needs an offline, not system link session.
    registry.register(Function, "issplitscreen", |_, _, _| Ok(Value::Int(0)));
    registry.register(Function, "tolower", |_, _, args| {
        let text = string(args, 0)?;
        if text.len() >= 1024 {
            return Err("string too long".into());
        }
        Ok(Value::string(&text.to_ascii_lowercase()))
    });
    registry.register(Function, "spawnstruct", |world, _, _| {
        let mut runtime = world.resource_mut::<RoundScript>();
        let id = runtime.next_object;
        runtime.next_object = id.checked_add(1).ok_or("object identifier exhausted")?;
        runtime.objects.insert(id, BTreeMap::new());
        Ok(Value::Object(id))
    });

    // Precaching: only while the level loads, before any wait.
    registry.register(Function, "precachemodel", |world, _, args| {
        let name = string(args, 0)?;
        if name.is_empty() {
            return Err("Model name string is empty".into());
        }
        super::iw4::precache(world, "model", name).map_err(|_| String::from("precacheModel must be called before any wait statements in the gametype or level script"))?;
        Ok(Value::Undefined)
    });
    registry.register(Function, "precacheshellshock", |world, _, args| {
        let name = string(args, 0)?;
        super::iw4::precache(world, "shellshock", name).map_err(|_| String::from("precacheShellShock must be called before any wait statements in the gametype or level script"))?;
        Ok(Value::Undefined)
    });
    registry.register(Function, "precacherumble", |world, _, args| {
        let name = string(args, 0)?;
        super::iw4::precache(world, "rumble", name).map_err(|_| {
            String::from(
                "precacheRumble must be called before any wait statements in the level script",
            )
        })?;
        Ok(Value::Undefined)
    });
    registry.register(Function, "loadfx", |world, _, args| {
        let name = string(args, 0)?;
        if name.get(..3).is_some_and(|prefix| prefix == "fx/") {
            return Err("effect name should start after the 'fx' folder.".into());
        }
        super::iw4::precache(world, "fx", name).map(Value::Int).map_err(|_| String::from("loadFx must be called before any wait statements in the level script, or on an already loaded effect"))
    });

    // Entities: a lookup by one of the entity's string keys; more than one
    // match is an error.
    registry.register(
        Function,
        "getent",
        |world, _, args| match entities_by_key(world, args)?.as_slice() {
            [] => Ok(Value::Undefined),
            [id] => Ok(Value::Object(*id)),
            _ => Err("getent used with more than one entity".into()),
        },
    );
    registry.register(Function, "getentarray", |world, _, args| {
        let ids = if args.is_empty() {
            world
                .resource::<RoundScript>()
                .entities
                .iter()
                .filter(|(_, entity)| entity.kind != EntityKind::HudElem)
                .map(|(id, _)| *id)
                .collect()
        } else {
            entities_by_key(world, args)?
        };
        new_array(world, ids.into_iter().map(Value::Object).collect())
    });
    // The connected players by client number; a team other than "all" keeps
    // that team's.
    registry.register(Function, "getplayers", |world, _, args| {
        let team = match args.first() {
            None => None,
            Some(Value::String(team)) if team.as_bytes() == b"all" => None,
            Some(_) => Some(string(args, 0)?),
        };
        let mut players: Vec<(i32, u64)> = world
            .resource::<RoundScript>()
            .entities
            .iter()
            .filter(|(_, entity)| entity.kind == EntityKind::Player)
            .map(|(id, entity)| (entity.number, *id))
            .collect();
        players.sort_unstable();
        let mut listed = Vec::new();
        for (_, id) in players {
            let keep = team.as_deref().is_none_or(|team| {
                match world.resource_mut::<RoundScript>().object_field(id, "team") {
                    Value::String(own) => own.as_bytes() == team.as_bytes(),
                    _ => false,
                }
            });
            if keep {
                listed.push(Value::Object(id));
            }
        }
        new_array(world, listed)
    });

    // A gametype setting the configs name; a whole number reads as an int.
    registry.register(Function, "getgametypesetting", |world, _, args| {
        let name = string(args, 0)?.to_ascii_lowercase();
        let Some(value) = world
            .get_resource::<crate::script::GametypeSettings>()
            .and_then(|settings| settings.0.get(&name).cloned())
        else {
            return Ok(Value::Undefined);
        };
        Ok(match value.parse::<i32>() {
            Ok(n) => Value::Int(n),
            Err(_) => Value::Float(atof(&value)),
        })
    });

    registry.register(Function, "precachestring", |world, _, args| {
        if !world.resource::<RoundScript>().loading {
            return Err("precacheString must be called before any wait statements in the gametype or level script\n".into());
        }
        let text = istring(args, 0)?;
        if !text.is_empty() {
            super::iw4::precache(world, "string", text)?;
        }
        Ok(Value::Undefined)
    });
    registry.register(Function, "precacheshader", |world, _, args| {
        if !world.resource::<RoundScript>().loading {
            return Err("precacheShader must be called before any wait statements in the gametype or level script\n".into());
        }
        let name = string(args, 0)?;
        if name.is_empty() {
            return Err("Shader name string is empty".into());
        }
        super::iw4::precache(world, "shader", name)?;
        Ok(Value::Undefined)
    });
    registry.register(Function, "precacheitem", |world, _, args| {
        if !world.resource::<RoundScript>().loading {
            return Err("precacheItem must be called before any wait statements in the gametype or level script\n".into());
        }
        let name = string(args, 0)?;
        crate::frame::FrameWorld::from_world(world)
            .weapon_index_by_script_name(&name)
            .ok_or_else(|| format!("unknown weapon '{name}'"))?;
        Ok(Value::Undefined)
    });
    // Status icons and menus each fill a fixed run of config strings; a
    // second precache of a name only prints.
    registry.register(Function, "precachestatusicon", |world, _, args| {
        let name = string(args, 0)?;
        let icons = &mut world
            .get_resource_or_insert_with(Precached::default)
            .status_icons;
        if !icons.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
            if icons.len() == 8 {
                return Err("Too many player status icons precached. Max allowed is 8".into());
            }
            icons.push(name);
        }
        Ok(Value::Undefined)
    });
    registry.register(Function, "precachemenu", |world, _, args| {
        let name = string(args, 0)?;
        let menus = &mut world.get_resource_or_insert_with(Precached::default).menus;
        if !menus.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
            if menus.len() == 32 {
                return Err("Too many menus precached. Max allowed menus is 32".into());
            }
            menus.push(name);
        }
        Ok(Value::Undefined)
    });

    // An existing dvar only becomes server info; a new one takes the value
    // with its double quotes made single.
    registry.register(Function, "makedvarserverinfo", |world, _, args| {
        let name = string(args, 0)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("Dvar {name} has an invalid dvar name"));
        }
        let name = name.to_ascii_lowercase();
        world
            .resource_mut::<RoundScript>()
            .server_info
            .insert(name.clone());
        if world
            .resource::<crate::script::MatchScript>()
            .dvars
            .contains_key(&name)
        {
            return Ok(Value::Undefined);
        }
        let value = match arg(args, 1)? {
            Value::LocalizedString(text) => text.to_string(),
            _ => string(args, 1)?,
        };
        crate::script::set_dvar(world, &name, &value.replace('"', "'"));
        Ok(Value::Undefined)
    });
    registry.register(Function, "setmatchflag", |world, _, args| {
        let name = string(args, 0)?;
        let on = int(args, 1)? != 0;
        let (word, bit) = MATCH_FLAGS
            .iter()
            .find(|(flag, _, _)| flag.eq_ignore_ascii_case(&name))
            .map(|&(_, word, bit)| (word, bit))
            .ok_or_else(|| {
                format!("GScr_SetMatchFlag: Couldn't find a matching bit for flag name : {name} ")
            })?;
        let flags = &mut world
            .get_resource_or_insert_with(MatchFlags::default)
            .match_words[word];
        let mask = (1u32 << (bit & 31)) >> 4;
        if on {
            *flags |= mask;
        } else {
            *flags &= !mask;
        }
        Ok(Value::Undefined)
    });
    // An unknown column name stays as the invalid column and only prints.
    registry.register(Function, "setscoreboardcolumns", |world, _, args| {
        let mut columns = [0; 5];
        for (index, column) in columns.iter_mut().enumerate() {
            let name = string(args, index)?;
            *column = SCOREBOARD_COLUMNS
                .iter()
                .position(|known| **known == *name)
                .unwrap_or(0);
        }
        world.insert_resource(ScoreboardColumns(columns));
        Ok(Value::Undefined)
    });
    registry.register(Function, "setroundsplayed", |world, _, args| {
        world.insert_resource(RoundsPlayed(int(args, 0)?));
        Ok(Value::Undefined)
    });

    registry.register(Function, "randomint", |world, _, args| {
        let max = int(args, 0)?;
        if max < 1 {
            return Err("RandomInt parm must be positive integer.\n".into());
        }
        Ok(Value::Int(irand(world, 0, max)))
    });
    registry.register(Function, "int", |_, _, args| match arg(args, 0)? {
        Value::String(text) => Ok(Value::Int(atoi(text))),
        Value::Float(f) => Ok(Value::Int(*f as i32)),
        Value::Int(n) => Ok(Value::Int(*n)),
        other => Err(format!("cannot cast {} to int", type_name(other))),
    });

    registry.register(Function, "float", |_, _, args| match arg(args, 0)? {
        Value::String(text) => Ok(Value::Float(atof(text))),
        Value::Float(f) => Ok(Value::Float(*f)),
        Value::Int(n) => Ok(Value::Float(*n as f32)),
        other => Err(format!("cannot cast {} to float", type_name(other))),
    });
    registry.register(Function, "randomintrange", |world, _, args| {
        let (min, max) = (int(args, 0)?, int(args, 1)?);
        if max <= min {
            return Err(
                "RandomIntRange's second parameter must be greater than the first.\n".into(),
            );
        }
        Ok(Value::Int(irand(world, min, max)))
    });
    registry.register(Function, "randomfloat", |world, _, args| {
        let max = float(args, 0)?;
        Ok(Value::Float(frand(world, 0.0, max)))
    });
    registry.register(Function, "randomfloatrange", |world, _, args| {
        let (min, max) = (float(args, 0)?, float(args, 1)?);
        if max <= min {
            return Err(
                "Scr_RandomFloatRange's second parameter must be greater than the first.\n".into(),
            );
        }
        Ok(Value::Float(frand(world, min, max)))
    });
    registry.register(Function, "getminbitcountfornum", |_, _, args| {
        let n = int(args, 0)?;
        if n < 1 {
            return Err("GScr_GetMinBitCountForNum: parm must be positive integer.\n".into());
        }
        Ok(Value::Int(32 - n.leading_zeros() as i32))
    });
    registry.register(Function, "ismp", |_, _, _| Ok(Value::Int(1)));
    // One online client, the owner's.
    registry.register(Function, "getnumexpectedplayers", |_, _, _| {
        Ok(Value::Int(1))
    });

    // The math opcodes.
    registry.register(Function, "anglestoforward", |_, _, args| {
        angles_to(args, "anglestoforward", 0)
    });
    registry.register(Function, "anglestoright", |_, _, args| {
        angles_to(args, "anglestoright", 1)
    });
    registry.register(Function, "anglestoup", |_, _, args| {
        angles_to(args, "anglestoup", 2)
    });
    registry.register(Function, "vectortoangles", |_, _, args| {
        match arg(args, 0)? {
            Value::Vector(v) => Ok(Value::Vector(math_iw4::vect_to_angles(*v))),
            other => Err(format!(
                "vectorstoangle expecting vector, given {})",
                type_name(other)
            )),
        }
    });
    registry.register(Function, "angleclamp180", |_, _, args| {
        let angle = match arg(args, 0)? {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f32,
            other => {
                return Err(format!(
                    "angleclamp180 expecting float, given {})",
                    type_name(other)
                ));
            }
        };
        let turns = angle * (1.0 / 360.0);
        let clamped = (turns - turns.floor()) * 360.0;
        Ok(Value::Float(if clamped > 180.0 {
            clamped - 360.0
        } else {
            clamped
        }))
    });
    registry.register(Function, "abs", |_, _, args| match arg(args, 0)? {
        Value::Float(f) => Ok(Value::Float(if *f < 0.0 { -f } else { *f })),
        Value::Int(n) => Ok(Value::Int(n.wrapping_abs())),
        other => Err(format!(
            "abs expecting float or int, given {})",
            type_name(other)
        )),
    });

    // Arrays: values match when their type and value do.
    registry.register(Function, "array", |world, _, args| {
        let values = args
            .iter()
            .take_while(|value| !matches!(value, Value::Undefined))
            .cloned()
            .collect();
        new_array(world, values)
    });
    // Without keeping keys the remaining entries are numbered from 0.
    registry.register(Function, "arrayremovevalue", |world, _, args| {
        let id = array_id(args, 0)?;
        let wanted = arg(args, 1)?.clone();
        let keep_keys = args.len() == 3 && int(args, 2)? != 0;
        let mut runtime = world.resource_mut::<RoundScript>();
        let array = runtime
            .arrays
            .get_mut(&id)
            .ok_or("invalid array reference")?;
        array.retain(|_, value| *value != wanted);
        if !keep_keys {
            let values = std::mem::take(array).into_values();
            *array = values
                .enumerate()
                .map(|(index, value)| (ArrayKey::Integer(index as i32), value))
                .collect();
        }
        Ok(Value::Undefined)
    });
    // An index inside the array moves the entries from it up by one; any
    // other appends.
    registry.register(Function, "arrayinsert", |world, _, args| {
        let id = array_id(args, 0)?;
        let value = arg(args, 1)?.clone();
        let index = int(args, 2)?;
        let mut runtime = world.resource_mut::<RoundScript>();
        let array = runtime
            .arrays
            .get_mut(&id)
            .ok_or("invalid array reference")?;
        let size = array.len() as i32;
        if (0..size).contains(&index) {
            for key in (index..size).rev() {
                if let Some(moved) = array.remove(&ArrayKey::Integer(key)) {
                    array.insert(ArrayKey::Integer(key + 1), moved);
                }
            }
            array.insert(ArrayKey::Integer(index), value);
        } else {
            array.insert(ArrayKey::Integer(size), value);
        }
        Ok(Value::Undefined)
    });
    registry.register(Function, "isinarray", |world, _, args| {
        let values = array(world, args, 0)?;
        let wanted = arg(args, 1)?;
        Ok(Value::Int(
            values.values().any(|value| value == wanted).into(),
        ))
    });
    registry.register(Function, "getarraykeys", |world, _, args| {
        let keys = array(world, args, 0)?
            .into_keys()
            .rev()
            .map(|key| match key {
                ArrayKey::Integer(n) => Value::Int(n),
                ArrayKey::String(s) => Value::String(s),
            })
            .collect();
        new_array(world, keys)
    });
    // Keeping keys needs both arrays keyed by strings, judged by their first
    // entry, and no key in both; without duplicates the second array's values
    // already in the first are left out.
    registry.register(Function, "arraycombine", |world, _, args| {
        let first = array(world, args, 0)?;
        let second = array(world, args, 1)?;
        let duplicates = int(args, 2)? != 0;
        let string_keyed = |array: &BTreeMap<ArrayKey, Value>| {
            array
                .keys()
                .next_back()
                .is_none_or(|key| matches!(key, ArrayKey::String(_)))
        };
        let keep_keys = int(args, 3)? != 0 && string_keyed(&first) && string_keyed(&second);
        if keep_keys && second.keys().any(|key| first.contains_key(key)) {
            return Err("Duplicate key found when merging arrays".into());
        }
        let mut combined: Vec<(ArrayKey, Value)> = first.clone().into_iter().collect();
        combined.extend(
            second
                .into_iter()
                .filter(|(_, value)| duplicates || !first.values().any(|known| known == value)),
        );
        if !keep_keys {
            return new_array(
                world,
                combined.into_iter().map(|(_, value)| value).collect(),
            );
        }
        let mut runtime = world.resource_mut::<RoundScript>();
        let id = runtime.next_object;
        runtime.next_object = id.checked_add(1).ok_or("object identifier exhausted")?;
        runtime.arrays.insert(id, combined.into_iter().collect());
        Ok(Value::Array(id))
    });

    registry.register(Function, "tablelookup", |world, _, args| {
        Ok(Value::String(table_lookup(world, args)?.into()))
    });

    registry.register(Function, "isarray", |_, _, args| {
        Ok(Value::Int(matches!(arg(args, 0)?, Value::Array(_)).into()))
    });
    registry.register(Function, "setteamscore", |world, _, args| {
        let team = team(args, 0)?;
        let score = int(args, 1)?;
        world.get_resource_or_insert_with(TeamScores::default).0[team] = score;
        Ok(Value::Undefined)
    });
    registry.register(Function, "getteamscore", |world, _, args| {
        let team = team(args, 0)?;
        Ok(Value::Int(
            world
                .get_resource::<TeamScores>()
                .map_or(0, |scores| scores.0[team]),
        ))
    });
    registry.register(Function, "setmatchtalkflag", |world, _, args| {
        let name = string(args, 0)?;
        let on = int(args, 1)? != 0;
        let bit = TALK_FLAGS
            .iter()
            .position(|flag| flag.eq_ignore_ascii_case(&name))
            .ok_or_else(|| {
                format!(
                    "GScr_SetMatchTalkFlag: Couldn't find a matching bit for flag name : {name} "
                )
            })?;
        let flags = &mut world.get_resource_or_insert_with(MatchFlags::default).talk;
        if on {
            *flags |= 1 << bit;
        } else {
            *flags &= !(1 << bit);
        }
        Ok(Value::Undefined)
    });
    registry.register(Function, "setgameendtime", |world, _, args| {
        let time = int(args, 0)?;
        world.resource_mut::<RoundScript>().engine.game_end_time = time;
        Ok(Value::Undefined)
    });
    // The level's vision set, eased in over the time given (a second by
    // default); other argument counts do nothing.
    registry.register(Function, "visionsetnaked", |world, _, args| {
        let seconds = match args.len() {
            1 => 1.0,
            2 => float(args, 1)?,
            _ => return Ok(Value::Undefined),
        };
        let name = string(args, 0)?;
        let set_ms = crate::level_time_ms(world.resource::<crate::step::StepRequest>().tick);
        world.resource_mut::<RoundScript>().engine.naked_vision = Some(crate::VisionChange {
            name,
            duration_ms: (seconds * 1000.0).round() as i32,
            set_ms,
        });
        Ok(Value::Undefined)
    });
    // Reports the match start to the online backend only.
    registry.register(Function, "recordmatchbegin", |_, _, _| Ok(Value::Undefined));

    // The classes a script can spawn.
    registry.register(Function, "spawn", |world, receiver, args| {
        let classname = string(args, 0)?;
        match classname.as_str() {
            "script_origin" | "script_model" | "trigger_radius" | "info_notnull"
            | "info_notnull_big" => {
                let spawn = NativeRegistry::engine_services()
                    .get(Function, "spawn")
                    .ok_or("spawn: no engine spawn")?;
                spawn(world, receiver, args)
            }
            "info_volume" | "trigger_box" | "trigger_box_use" | "trigger_radius_use"
            | "trigger_damage" => Err(format!("spawn {classname}: not implemented yet")),
            _ if classname.starts_with("weapon_")
                || (classname.starts_with("mp_") && classname.contains("_spawn")) =>
            {
                Err(format!("spawn {classname}: not implemented yet"))
            }
            _ => Err(format!("unable to spawn \"{classname}\" entity")),
        }
    });

    registry.register(Method, "fadeovertime", |world, receiver, args| {
        let seconds = float(args, 0)?;
        if seconds <= 0.0 {
            return Err(format!("fade time {} <= 0", g(seconds)));
        }
        if seconds > 60.0 {
            return Err(format!("fade time {} > 60", g(seconds)));
        }
        let fade = NativeRegistry::engine_services()
            .get(Method, "fadeovertime")
            .ok_or("fadeovertime: no engine fade")?;
        fade(world, receiver, args)
    });

    // Path nodes, in map order; a lookup names one of the node's string
    // fields and an empty field never matches.
    registry.register(Function, "getallnodes", |world, _, _| {
        let nodes = super::super::actor_nav::nodes_where(world, |_| true)?;
        new_array(world, nodes)
    });
    registry.register(Function, "getnodearray", |world, _, args| {
        let value = string(args, 0)?;
        let key = string(args, 1)?;
        let field = node_string_field(&key)?;
        let nodes = super::super::actor_nav::nodes_where(world, |node| {
            let text = field(node);
            !text.is_empty() && *text == *value
        })?;
        new_array(world, nodes)
    });

    // IW4L's own builtins whose state and checks match Black Ops 2's handler
    // on the arguments it accepts.
    let shared = NativeRegistry::engine_services();
    for (namespace, name) in [
        (Function, "newhudelem"),
        (Method, "setshader"),
        (Method, "setvalue"),
        (Method, "freezecontrols"),
        (Function, "visionsetnight"),
    ] {
        if let Some(native) = shared.get(namespace, name) {
            registry.register(namespace, name, native);
        }
    }
}

/// The node field a node lookup key names; it has to hold a string.
fn node_string_field(key: &str) -> Result<fn(&crate::script::NavNode) -> &str, String> {
    Ok(match key {
        "targetname" => |node| &node.targetname,
        "target" => |node| &node.target,
        "animscript" => |node| &node.animscript,
        "script_linkname" => |node| node.fields.as_ref().map_or("", |f| &f.script_linkname),
        "script_noteworthy" => |node| node.fields.as_ref().map_or("", |f| &f.script_noteworthy),
        "origin" | "angles" | "radius" | "minusedistsq" | "spawnflags" | "type" | "suspended" => {
            return Err("key is not internally a string".into());
        }
        _ => return Err(format!("key '{key}' does not internally belong to nodes")),
    })
}

/// The entity fields Black Ops 2 looks an entity up by: its string keys.
const ENTITY_STRING_KEYS: [&str; 4] = ["classname", "target", "targetname", "script_noteworthy"];

/// The tables a lookup can name by number.
const NUMBERED_TABLES: [&str; 17] = [
    "mp/statstable.csv",
    "mp/statsmilestones1.csv",
    "mp/statsmilestones2.csv",
    "mp/statsmilestones3.csv",
    "mp/statsmilestones4.csv",
    "mp/globalcounters.csv",
    "mp/globalCounterRefs.csv",
    "mp/trackercolumns.csv",
    "sp/statstable.csv",
    "mp/rankicontable.csv",
    "mp/scoreinfo.csv",
    "mp/gametypesTable.csv",
    "zm/gamemodesTable.csv",
    "mp/rankicontable_zm.csv",
    "mp/gunlevels.csv",
    "zm/zm_statstable.csv",
    "mp/mtxitems.csv",
];

/// `setmatchflag`'s names: the flag word and the bit shifted down by four.
const MATCH_FLAGS: [(&str, usize, u32); 16] = [
    ("radar_allies", 0, 6),
    ("radar_axis", 0, 7),
    ("bomb_timer", 0, 9),
    ("bomb_timer_a", 0, 10),
    ("bomb_timer_b", 0, 11),
    ("ammocounterhide", 0, 12),
    ("enable_popups", 1, 8),
    ("hud_hardcore", 1, 13),
    ("pregame", 1, 14),
    ("final_killcam", 1, 4),
    ("round_end_killcam", 1, 5),
    ("cg_drawSpectatorMessages", 1, 15),
    ("disableIngameMenu", 1, 16),
    ("hud_zombie", 1, 0x36),
    ("game_ended", 1, 17),
    ("overtime", 0, 18),
];

const SCOREBOARD_COLUMNS: [&str; 26] = [
    "_invalid",
    "none",
    "score",
    "kills",
    "deaths",
    "assists",
    "defends",
    "plants",
    "defuses",
    "returns",
    "captures",
    "destructions",
    "kdratio",
    "survived",
    "stabs",
    "tomahawks",
    "humiliated",
    "x2score",
    "headshots",
    "downs",
    "revives",
    "agrkills",
    "hacks",
    "pointstowin",
    "killsconfirmed",
    "killsdenied",
];

#[derive(Resource, Default)]
struct Precached {
    status_icons: Vec<String>,
    menus: Vec<String>,
}

/// The match flag words the HUD reads, and the voice chat rules.
#[derive(Resource, Default)]
pub(crate) struct MatchFlags {
    pub(crate) match_words: [u32; 2],
    pub(crate) talk: u32,
}

/// Scores by team number.
#[derive(Resource, Default)]
struct TeamScores([i32; 10]);

/// `setmatchtalkflag`'s names, by bit.
const TALK_FLAGS: [&str; 7] = [
    "EveryoneHearsEveryone",
    "DeadChatWithDead",
    "DeadChatWithTeam",
    "DeadHearTeamLiving",
    "DeadHearAllLiving",
    "DeadHearKiller",
    "KillersHearVictim",
];

/// A team parameter as its number: allies 1, axis 2, team3 to team8, and
/// spectator 9; none, free and neutral are 0.
fn team(args: &[Value], index: usize) -> Result<usize, String> {
    let name = string(args, index)?;
    Ok(match name.as_str() {
        "allies" => 1,
        "axis" => 2,
        "team3" => 3,
        "team4" => 4,
        "team5" => 5,
        "team6" => 6,
        "team7" => 7,
        "team8" => 8,
        "spectator" => 9,
        "none" | "free" | "neutral" => 0,
        _ => {
            return Err(format!(
                "'{name}' is an illegal team string. Must be allies, axis, team3, team4, team5, team6, team7, team8, none, or spectator."
            ));
        }
    })
}

/// A builtin's float parameter: a float or an int.
fn float(args: &[Value], index: usize) -> Result<f32, String> {
    match arg(args, index)? {
        Value::Float(f) => Ok(*f),
        Value::Int(n) => Ok(*n as f32),
        other => Err(format!("type {} is not a float", type_name(other))),
    }
}

/// `%g`.
fn g(value: f32) -> String {
    crate::script::runtime::to_text(&Value::Float(value)).unwrap_or_else(|| value.to_string())
}

#[derive(Resource)]
#[expect(
    dead_code,
    reason = "the scoreboard reads it once it draws Black Ops 2's"
)]
pub(crate) struct ScoreboardColumns(pub(crate) [usize; 5]);

#[derive(Resource)]
#[expect(
    dead_code,
    reason = "the scoreboard reads it once it draws Black Ops 2's"
)]
pub(crate) struct RoundsPlayed(pub(crate) i32);

/// The game's one random state; the engine seeds it from the clock when the
/// server spawns, IW4L from its match random.
#[derive(Resource)]
struct HoldRand(u32);

fn irand(world: &mut World, min: i32, max: i32) -> i32 {
    let state = next_rand(world);
    let span = i64::from(max.wrapping_sub(min));
    (((i64::from((state >> 17) as i32) * span) >> 15) as i32).wrapping_add(min)
}

fn frand(world: &mut World, min: f32, max: f32) -> f32 {
    let state = next_rand(world);
    (f64::from(state >> 17) * (f64::from(max) - f64::from(min)) * f64::from(1.0f32 / 32768.0)
        + f64::from(min)) as f32
}

fn next_rand(world: &mut World) -> u32 {
    if !world.contains_resource::<HoldRand>() {
        let seed = super::math::random(world);
        world.insert_resource(HoldRand(seed));
    }
    let mut state = world.resource_mut::<HoldRand>();
    state.0 = state.0.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
    state.0
}

/// The opcodes turning angles into one of their axes take only a vector.
fn angles_to(args: &[Value], name: &str, axis: usize) -> Result<Value, String> {
    let angles = match arg(args, 0)? {
        Value::Vector(v) => *v,
        other => {
            return Err(format!(
                "{name} expecting vector, given {})",
                type_name(other)
            ));
        }
    };
    let (forward, right, up) = math_iw4::angle_vectors(angles);
    Ok(Value::Vector([forward, right, up][axis]))
}

/// An array parameter's id.
fn array_id(args: &[Value], index: usize) -> Result<u64, String> {
    match arg(args, index)? {
        Value::Array(id) => Ok(*id),
        other => Err(format!("Parameter ({}) must be an array", type_name(other))),
    }
}

/// An array parameter's entries.
fn array(world: &World, args: &[Value], index: usize) -> Result<BTreeMap<ArrayKey, Value>, String> {
    match arg(args, index)? {
        Value::Array(id) => world
            .resource::<RoundScript>()
            .arrays
            .get(id)
            .cloned()
            .ok_or_else(|| "invalid array reference".into()),
        other => Err(format!("Parameter ({}) must be an array", type_name(other))),
    }
}

/// `tablelookup`: the table by name or number, then one to three column and
/// value pairs and the column to answer; any miss answers "".
fn table_lookup(world: &World, args: &[Value]) -> Result<String, String> {
    let name = match arg(args, 0)? {
        Value::Int(n) => usize::try_from(*n)
            .ok()
            .and_then(|n| NUMBERED_TABLES.get(n))
            .ok_or("table number out of range")?
            .to_string(),
        Value::String(name) => name.to_string(),
        _ => return Ok(String::new()),
    };
    if !matches!(args.len(), 4 | 6 | 8) {
        return Err(format!(
            "Incorrect number of arguments ({}) passed to TableLookup!\n",
            args.len()
        ));
    }
    let mut keys = Vec::new();
    for pair in (1..args.len() - 1).step_by(2) {
        keys.push((int(args, pair)?, string(args, pair + 1)?));
    }
    let answer = int(args, args.len() - 1)?;
    let tables = world.resource::<RoundScript>().tables.clone();
    let Some(table) = crate::script::host::tables::table(&tables, &name) else {
        return Ok(String::new());
    };
    let row = (0..table.rows).find(|&row| {
        keys.iter().all(|(column, value)| {
            usize::try_from(*column)
                .ok()
                .and_then(|column| table.cell(row, column))
                .is_some_and(|cell| cell.eq_ignore_ascii_case(value))
        })
    });
    Ok(row
        .zip(usize::try_from(answer).ok())
        .and_then(|(row, column)| table.cell(row, column))
        .unwrap_or("")
        .to_owned())
}

fn entities_by_key(world: &mut World, args: &[Value]) -> Result<Vec<u64>, String> {
    let value = string(args, 0)?;
    let key = string(args, 1)?;
    if !ENTITY_STRING_KEYS.contains(&key.as_str()) {
        return Ok(Vec::new());
    }
    let mut runtime = world.resource_mut::<RoundScript>();
    let field = runtime.symbol(&key);
    let runtime = &*runtime;
    Ok(runtime
        .entities
        .iter()
        .filter(|(_, entity)| entity.kind != EntityKind::HudElem)
        .map(|(id, _)| *id)
        .filter(|id| {
            matches!(
                runtime.objects.get(id).and_then(|fields| fields.get(&field)),
                Some(Value::String(text)) if **text == *value
            )
        })
        .collect())
}

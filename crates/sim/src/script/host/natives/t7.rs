//! Black Ops 3's builtins, each as Treyarch's script reference for the Mod
//! Tools (`bo3_scriptapifunctions.htm`) describes it. Where the reference
//! leaves a behaviour open, the choice is IW4L's and `docs/fidelity/t7.md`
//! lists it.

use super::super::args::{arg, byte_string, float, int, kind, optional, string, vector};
use super::super::arrays::{iteration_key, new_array};
use super::super::entities::EntityKind;
use crate::script::{ArrayKey, Namespace, NativeRegistry, RoundScript, Value};
use bevy_ecs::prelude::{Resource, World};
use std::collections::BTreeMap;

/// The gametype's settings, by lower-case name.
#[derive(Resource, Default)]
pub struct GametypeSettings(pub BTreeMap<String, String>);

/// What the scripts set for the match that IW4L does not act on yet: the
/// scoreboard's columns, match flag bits, the navigation clearance ceiling.
#[derive(Resource, Default)]
pub(crate) struct MatchState {
    pub(crate) scoreboard_columns: Vec<String>,
    pub(crate) flags: BTreeMap<String, i32>,
    pub(crate) talk_flags: BTreeMap<String, String>,
    pub(crate) clearance_ceiling: Option<f32>,
}

/// The client fields the scripts registered: pool and name → bits and type.
#[derive(Resource, Default)]
pub(crate) struct ClientFields(BTreeMap<(String, String), (i32, String)>);

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
    registry.register(Function, "gettime", |world, _, args| {
        if !args.is_empty() {
            return Err("gettime expects no arguments".into());
        }
        let tick = world.resource::<crate::step::StepRequest>().tick;
        let ms = u64::from(tick.0) * u64::from(crate::MATCH_TICK_MS);
        i32::try_from(ms)
            .map(Value::Int)
            .map_err(|_| "gettime overflow".into())
    });
    registry.register(Function, "spawnstruct", |world, _, _| {
        let mut runtime = world.resource_mut::<RoundScript>();
        let id = runtime.next_object;
        runtime.next_object = id.checked_add(1).ok_or("object identifier exhausted")?;
        runtime.objects.insert(id, BTreeMap::new());
        Ok(Value::Object(id))
    });

    // Entities: "Looks up entities by key and name"; more than one match is
    // Black Ops 3's error "getent used with more than one entity" (its
    // executable carries the text). Spawners are not told apart yet.
    registry.register(Function, "getent", |world, _, args| {
        super::engine::single(super::engine::matching_entities(world, args)?, "getent")
    });
    registry.register(Function, "getentarray", |world, _, args| {
        let ids: Vec<u64> = if args.iter().all(|a| matches!(a, Value::Undefined)) {
            world
                .resource::<RoundScript>()
                .entities
                .iter()
                .filter(|(_, e)| e.kind != EntityKind::HudElem)
                .map(|(id, _)| *id)
                .collect()
        } else {
            super::engine::matching_entities(world, args)?
        };
        new_array(world, ids.into_iter().map(Value::Object).collect())
    });
    // "Returns an array of the currently connected players"; [team]
    // "allies", "axis", "neutral" or "all".
    registry.register(Function, "getplayers", |world, _, args| {
        let team = match args.first() {
            None | Some(Value::Undefined) => None,
            Some(Value::String(team)) if team.as_bytes() == b"all" => None,
            Some(Value::String(team)) => Some(team.to_string()),
            Some(other) => return Err(format!("getplayers: team is {}", kind(other))),
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

    // Arrays: "create an array", "Checks whether this entity/variable is an
    // array", keys and their walk.
    registry.register(Function, "array", |world, _, args| {
        new_array(world, args.to_vec())
    });
    registry.register(Function, "isarray", |_, _, args| {
        Ok(Value::Int(matches!(arg(args, 0)?, Value::Array(_)).into()))
    });
    registry.register(Function, "isinarray", |world, _, args| {
        let values = array(world, arg(args, 0)?)?;
        let wanted = arg(args, 1)?;
        Ok(Value::Int(values.values().any(|v| v == wanted).into()))
    });
    registry.register(Function, "getarraykeys", |world, _, args| {
        let keys: Vec<Value> = walk(&array(world, arg(args, 0)?)?)
            .map(|(key, _)| key_value(key))
            .collect();
        new_array(world, keys)
    });
    registry.register(Function, "getfirstarraykey", |world, _, args| {
        iteration_key(world, arg(args, 0)?, None)
    });
    registry.register(Function, "getnextarraykey", |world, _, args| {
        iteration_key(world, arg(args, 0)?, Some(arg(args, 1)?))
    });
    registry.register(Function, "getlastarraykey", |world, _, args| {
        let values = array(world, arg(args, 0)?)?;
        Ok(walk(&values)
            .last()
            .map_or(Value::Undefined, |(k, _)| key_value(k)))
    });
    registry.register(Function, "getprevarraykey", |world, _, args| {
        let values = array(world, arg(args, 0)?)?;
        let key = array_key(arg(args, 1)?)?;
        if !values.contains_key(&key) {
            return Err("array key does not exist".into());
        }
        Ok(values
            .range((std::ops::Bound::Excluded(key), std::ops::Bound::Unbounded))
            .next()
            .map_or(Value::Undefined, |(k, _)| key_value(k)))
    });
    // "combine two arrays into and returns one new array": duplicates pruned
    // unless <allowdupes>; string keys kept with <preserve_keys>, the rest
    // renumbered (in key order: IW4L's choice).
    registry.register(Function, "arraycombine", |world, _, args| {
        let first = array(world, arg(args, 0)?)?;
        let second = array(world, arg(args, 1)?)?;
        let allow_dupes = truth(arg(args, 2)?);
        let preserve = truth(arg(args, 3)?);
        let mut out: BTreeMap<ArrayKey, Value> = BTreeMap::new();
        let mut next = 0;
        for (key, value) in first.iter().chain(second.iter()) {
            if !allow_dupes && out.values().any(|v| v == value) {
                continue;
            }
            match key {
                ArrayKey::String(_) if preserve => {
                    out.insert(key.clone(), value.clone());
                }
                _ => {
                    out.insert(ArrayKey::Integer(next), value.clone());
                    next += 1;
                }
            }
        }
        new_keyed_array(world, out)
    });
    // "Remove a value from an array (may affect multiple indexes)";
    // [preserve_keys] "otherwise creates integer keys".
    registry.register(Function, "arrayremovevalue", |world, _, args| {
        let values = array(world, arg(args, 0)?)?;
        let removed = arg(args, 1)?;
        let preserve = args.get(2).is_some_and(truth);
        let kept = values.into_iter().filter(|(_, v)| v != removed);
        let out: BTreeMap<ArrayKey, Value> = if preserve {
            kept.collect()
        } else {
            kept.enumerate()
                .map(|(i, (_, v))| (ArrayKey::Integer(i as i32), v))
                .collect()
        };
        new_keyed_array(world, out)
    });
    registry.register(Function, "arraycopy", |world, _, args| {
        let values = array(world, arg(args, 0)?)?;
        new_keyed_array(world, values)
    });

    // Types: "Checks whether this entity/variable is …".
    registry.register(Function, "isint", |_, _, args| {
        Ok(Value::Int(matches!(arg(args, 0)?, Value::Int(_)).into()))
    });
    registry.register(Function, "isfloat", |_, _, args| {
        Ok(Value::Int(matches!(arg(args, 0)?, Value::Float(_)).into()))
    });
    registry.register(Function, "isstring", |_, _, args| {
        Ok(Value::Int(matches!(arg(args, 0)?, Value::String(_)).into()))
    });
    registry.register(Function, "isvec", |_, _, args| {
        Ok(Value::Int(matches!(arg(args, 0)?, Value::Vector(_)).into()))
    });
    // No weapon object exists yet: Black Ops 3's weapon assets are not read.
    registry.register(Function, "isweapon", |_, _, _| Ok(Value::Int(0)));
    registry.register(Function, "isfunctionptr", |_, _, args| {
        Ok(Value::Int(
            matches!(arg(args, 0)?, Value::Function(_) | Value::Builtin(_)).into(),
        ))
    });

    // Strings.
    registry.register(Function, "tolower", |_, _, args| {
        Ok(Value::byte_string(
            &byte_string(args, 0)?.as_bytes().to_ascii_lowercase(),
        ))
    });
    registry.register(Function, "toupper", |_, _, args| {
        Ok(Value::byte_string(
            &byte_string(args, 0)?.as_bytes().to_ascii_uppercase(),
        ))
    });
    registry.register(Function, "strtok", |world, _, args| {
        // "Tokenizes <string> by the delimiters <delim>": each byte of
        // <delim> is a delimiter; empty tokens are dropped (IW4L's choice).
        let text = byte_string(args, 0)?;
        let delimiters = byte_string(args, 1)?;
        let tokens = text
            .as_bytes()
            .split(|byte| delimiters.as_bytes().contains(byte))
            .filter(|token| !token.is_empty())
            .map(Value::byte_string)
            .collect();
        new_array(world, tokens)
    });
    registry.register(Function, "issubstr", |_, _, args| {
        let text = string(args, 0)?;
        Ok(Value::Int(text.contains(&string(args, 1)?).into()))
    });
    registry.register(Function, "strstartswith", |_, _, args| {
        let text = string(args, 0)?;
        Ok(Value::Int(text.starts_with(&string(args, 1)?).into()))
    });
    registry.register(Function, "strendswith", |_, _, args| {
        let text = string(args, 0)?;
        Ok(Value::Int(text.ends_with(&string(args, 1)?).into()))
    });
    registry.register(Function, "getsubstr", |_, _, args| {
        // "the substring of characters >= <start index> and < <end index>".
        let text = byte_string(args, 0)?;
        let bytes = text.as_bytes();
        let start = (int(args, 1)?.max(0) as usize).min(bytes.len());
        let end = optional(args, 2, int)?
            .map_or(bytes.len(), |end| (end.max(0) as usize).min(bytes.len()));
        Ok(Value::byte_string(&bytes[start..end.max(start)]))
    });

    // Dvars: "The dvar name as a string or hash"; "[default_value] The value
    // to be returned if the dvar doesn't exist".
    registry.register(Function, "getdvarstring", |world, _, args| {
        Ok(match dvar(world, args)? {
            Some(text) => Value::String(text.into()),
            None => default(args, || Value::string(""))?,
        })
    });
    registry.register(Function, "getdvarint", |world, _, args| {
        Ok(match dvar(world, args)? {
            Some(text) => Value::Int(leading_number(&text) as i32),
            None => default(args, || Value::Int(0))?,
        })
    });
    registry.register(Function, "getdvarfloat", |world, _, args| {
        Ok(match dvar(world, args)? {
            Some(text) => Value::Float(leading_number(&text) as f32),
            None => default(args, || Value::Float(0.0))?,
        })
    });
    registry.register(Function, "getdvarvector", |world, _, args| {
        Ok(match dvar(world, args)? {
            Some(text) => {
                let mut parts = text.split_ascii_whitespace().map(leading_number);
                Value::Vector(std::array::from_fn(|_| parts.next().unwrap_or(0.0) as f32))
            }
            None => default(args, || Value::Vector([0.0; 3]))?,
        })
    });
    registry.register(Function, "setdvar", |world, _, args| {
        let name = string(args, 0)?.to_ascii_lowercase();
        let value = match arg(args, 1)? {
            Value::Undefined => String::new(),
            _ => string(args, 1)?,
        };
        crate::script::set_dvar(world, &name, &value);
        Ok(Value::Undefined)
    });

    // The session: a Classic zombies match IW4L hosts on its own network
    // (IW4L's choice: not online, not private, not system link).
    for (name, answer) in [
        ("sessionmodeiszombiesgame", true),
        ("sessionmodeismultiplayergame", false),
        ("sessionmodeiscampaigngame", false),
        ("sessionmodeiscampaignzombiesgame", false),
        ("sessionmodeiscampaigndeadopsgame", false),
        ("sessionmodeisonlinegame", false),
        ("sessionmodeisprivate", false),
        ("sessionmodeisprivateonlinegame", false),
        ("sessionmodeissystemlink", false),
    ] {
        registry.register(
            Function,
            name,
            if answer {
                |_, _, _| Ok(Value::Int(1))
            } else {
                |_, _, _| Ok(Value::Int(0))
            },
        );
    }

    // A match IW4L hosts is never split screen and does not rank players
    // (IW4L's choice: neither builtin is in the reference).
    registry.register(Function, "issplitscreen", |_, _, _| Ok(Value::Int(0)));
    registry.register(Function, "gamemodeisusingxp", |_, _, _| Ok(Value::Int(0)));
    // "Set the scoreboard columns", "Flags a bit in the match state", "Sets a
    // new clearance ceiling": recorded; nothing reads them yet.
    registry.register(Function, "setscoreboardcolumns", |world, _, args| {
        let columns = (0..args.len())
            .map(|i| string(args, i))
            .collect::<Result<Vec<_>, _>>()?;
        world
            .get_resource_or_insert_with(MatchState::default)
            .scoreboard_columns = columns;
        Ok(Value::Undefined)
    });
    // The scripts name the bit (`SetMatchFlag( "disableIngameMenu", 1 )`).
    registry.register(Function, "setmatchflag", |world, _, args| {
        let (bit, value) = (string(args, 0)?, int(args, 1)?);
        world
            .get_resource_or_insert_with(MatchState::default)
            .flags
            .insert(bit, value);
        Ok(Value::Undefined)
    });
    // "Flags a bit in the match talk state": `SetMatchTalkFlag(
    // "DeadChatWithDead", "1" )`.
    registry.register(Function, "setmatchtalkflag", |world, _, args| {
        let (bit, value) = (string(args, 0)?, string(args, 1)?);
        world
            .get_resource_or_insert_with(MatchState::default)
            .talk_flags
            .insert(bit, value);
        Ok(Value::Undefined)
    });
    registry.register(Function, "setclearanceceiling", |world, _, args| {
        let ceiling = float(args, 0)?;
        world
            .get_resource_or_insert_with(MatchState::default)
            .clearance_ceiling = Some(ceiling);
        Ok(Value::Undefined)
    });

    // Client fields: "Register a client field" from a pool ("world",
    // "actor", "vehicle", "scriptmover", …) with 1-32 bits, "int" or "float".
    registry.register(Function, "registerclientfield", |world, _, args| {
        let pool = string(args, 0)?;
        let name = string(args, 1)?;
        let bits = int(args, 3)?;
        let ty = string(args, 4)?;
        if !(1..=32).contains(&bits) {
            return Err(format!("client field {name}: {bits} bits"));
        }
        world
            .get_resource_or_insert_with(ClientFields::default)
            .0
            .insert((pool, name), (bits, ty));
        Ok(Value::Undefined)
    });

    // "Gets the value of a gametype setting": a whole number reads as an
    // int, any other number as a float, the rest as a string.
    registry.register(Function, "getgametypesetting", |world, _, args| {
        let name = string(args, 0)?.to_ascii_lowercase();
        let value = world
            .get_resource::<GametypeSettings>()
            .and_then(|settings| settings.0.get(&name).cloned())
            .ok_or_else(|| format!("gametype setting {name} is in no gamesettings file"))?;
        Ok(if let Ok(n) = value.parse::<i32>() {
            Value::Int(n)
        } else if let Ok(f) = value.parse::<f32>() {
            Value::Float(f)
        } else {
            Value::String(value.into())
        })
    });

    // Math.
    registry.register(Function, "getminbitcountfornum", |_, _, args| {
        // "GetMinBitCountForNum( 15 ) returns 4".
        let n = int(args, 0)? as u32;
        Ok(Value::Int((u32::BITS - n.leading_zeros()) as i32))
    });
    registry.register(Function, "int", |_, _, args| {
        Ok(Value::Int(match arg(args, 0)? {
            Value::Int(n) => *n,
            Value::Float(n) => *n as i32,
            Value::String(s) => leading_number(s) as i32,
            other => return Err(format!("cannot cast {} to int", kind(other))),
        }))
    });
    registry.register(Function, "float", |_, _, args| {
        Ok(Value::Float(match arg(args, 0)? {
            Value::Int(n) => *n as f32,
            Value::Float(n) => *n,
            Value::String(s) => leading_number(s) as f32,
            other => return Err(format!("cannot cast {} to float", kind(other))),
        }))
    });
    registry.register(Function, "abs", |_, _, args| {
        Ok(match arg(args, 0)? {
            Value::Int(n) => Value::Int(n.wrapping_abs()),
            _ => Value::Float(float(args, 0)?.abs()),
        })
    });
    registry.register(Function, "max", |_, _, args| {
        // "Returns whichever of <a> or <b> is larger."
        let (a, b) = (float(args, 0)?, float(args, 1)?);
        Ok(args[if a >= b { 0 } else { 1 }].clone())
    });
    registry.register(Function, "min", |_, _, args| {
        let (a, b) = (float(args, 0)?, float(args, 1)?);
        Ok(args[if a <= b { 0 } else { 1 }].clone())
    });
    registry.register(Function, "floor", |_, _, args| {
        Ok(Value::Float(float(args, 0)?.floor()))
    });
    registry.register(Function, "ceil", |_, _, args| {
        Ok(Value::Float(float(args, 0)?.ceil()))
    });
    registry.register(Function, "sqrt", |_, _, args| {
        Ok(Value::Float(float(args, 0)?.sqrt()))
    });
    registry.register(Function, "pow", |_, _, args| {
        Ok(Value::Float(float(args, 0)?.powf(float(args, 1)?)))
    });
    registry.register(Function, "sin", |_, _, args| {
        Ok(Value::Float(float(args, 0)?.to_radians().sin()))
    });
    registry.register(Function, "cos", |_, _, args| {
        Ok(Value::Float(float(args, 0)?.to_radians().cos()))
    });
    registry.register(Function, "tan", |_, _, args| {
        Ok(Value::Float(float(args, 0)?.to_radians().tan()))
    });
    registry.register(Function, "lerpfloat", |_, _, args| {
        let (from, to, t) = (float(args, 0)?, float(args, 1)?, float(args, 2)?);
        Ok(Value::Float(from + (to - from) * t))
    });
    registry.register(Function, "randomint", |world, _, args| {
        // "a random integer between 0 and max-1 inclusive".
        let max = int(args, 0)?;
        if max <= 0 {
            return Ok(Value::Int(0));
        }
        Ok(Value::Int((super::math::random(world) % max as u32) as i32))
    });
    registry.register(Function, "randomintrange", |world, _, args| {
        // "a random integer r, where min <= r < max".
        let (min, max) = (int(args, 0)?, int(args, 1)?);
        if max <= min {
            return Ok(Value::Int(min));
        }
        let span = (i64::from(max) - i64::from(min)) as u32;
        Ok(Value::Int(
            min.wrapping_add((super::math::random(world) % span) as i32),
        ))
    });
    registry.register(Function, "randomfloat", |world, _, args| {
        Ok(Value::Float(unit(world) * float(args, 0)?))
    });
    registry.register(Function, "randomfloatrange", |world, _, args| {
        let (min, max) = (float(args, 0)?, float(args, 1)?);
        Ok(Value::Float(min + unit(world) * (max - min)))
    });
    registry.register(Function, "distance", |_, _, args| {
        Ok(Value::Float(length(sub(
            vector(args, 0)?,
            vector(args, 1)?,
        ))))
    });
    registry.register(Function, "distancesquared", |_, _, args| {
        let d = sub(vector(args, 0)?, vector(args, 1)?);
        Ok(Value::Float(dot(d, d)))
    });
    registry.register(Function, "distance2d", |_, _, args| {
        let d = sub(vector(args, 0)?, vector(args, 1)?);
        Ok(Value::Float((d[0] * d[0] + d[1] * d[1]).sqrt()))
    });
    registry.register(Function, "distance2dsquared", |_, _, args| {
        let d = sub(vector(args, 0)?, vector(args, 1)?);
        Ok(Value::Float(d[0] * d[0] + d[1] * d[1]))
    });
    registry.register(Function, "length", |_, _, args| {
        Ok(Value::Float(length(vector(args, 0)?)))
    });
    registry.register(Function, "lengthsquared", |_, _, args| {
        let v = vector(args, 0)?;
        Ok(Value::Float(dot(v, v)))
    });
    registry.register(Function, "vectordot", |_, _, args| {
        Ok(Value::Float(dot(vector(args, 0)?, vector(args, 1)?)))
    });
    registry.register(Function, "vectorcross", |_, _, args| {
        let (a, b) = (vector(args, 0)?, vector(args, 1)?);
        Ok(Value::Vector([
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]))
    });
    registry.register(Function, "vectornormalize", |_, _, args| {
        let v = vector(args, 0)?;
        let l = length(v);
        Ok(Value::Vector(if l == 0.0 {
            v
        } else {
            [v[0] / l, v[1] / l, v[2] / l]
        }))
    });
    // Forward, right and up of pitch, yaw, roll in degrees (the engine
    // family's angle convention: IW4L's reading, the reference only names
    // them).
    registry.register(Function, "anglestoforward", |_, _, args| {
        Ok(Value::Vector(angle_vectors(vector(args, 0)?)[0]))
    });
    registry.register(Function, "anglestoright", |_, _, args| {
        Ok(Value::Vector(angle_vectors(vector(args, 0)?)[1]))
    });
    registry.register(Function, "anglestoup", |_, _, args| {
        Ok(Value::Vector(angle_vectors(vector(args, 0)?)[2]))
    });
    registry.register(Function, "vectorlerp", |_, _, args| {
        let (a, b, t) = (vector(args, 0)?, vector(args, 1)?, float(args, 2)?);
        Ok(Value::Vector(std::array::from_fn(|i| {
            a[i] + (b[i] - a[i]) * t
        })))
    });

    // Script errors: "throw a script error if this is false", "Throws a
    // script error with the given message", "Throws a non-fatal script error".
    registry.register(Function, "assert", |_, _, args| {
        let holds = match arg(args, 0)? {
            Value::Undefined => false,
            Value::Int(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            _ => true,
        };
        if holds {
            return Ok(Value::Undefined);
        }
        Err(optional(args, 1, string)?.unwrap_or_else(|| "assert failed".to_owned()))
    });
    registry.register(Function, "assertmsg", |_, _, args| Err(string(args, 0)?));
    registry.register(Function, "errormsg", |_, _, args| {
        diag::warn!(Sim, "gsc: script error: {}", string(args, 0)?);
        Ok(Value::Undefined)
    });
}

fn array(world: &World, value: &Value) -> Result<BTreeMap<ArrayKey, Value>, String> {
    let Value::Array(id) = value else {
        return Err(format!("{} is not an array", kind(value)));
    };
    world
        .resource::<RoundScript>()
        .arrays
        .get(id)
        .cloned()
        .ok_or_else(|| "invalid array reference".into())
}

fn new_keyed_array(world: &mut World, values: BTreeMap<ArrayKey, Value>) -> Result<Value, String> {
    let mut runtime = world.resource_mut::<RoundScript>();
    let id = runtime.next_object;
    runtime.next_object = id.checked_add(1).ok_or("object identifier exhausted")?;
    runtime.arrays.insert(id, values);
    Ok(Value::Array(id))
}

/// An array's entries in the order `GetFirstArrayKey`/`GetNextArrayKey`
/// visit them.
fn walk(values: &BTreeMap<ArrayKey, Value>) -> impl Iterator<Item = (&ArrayKey, &Value)> {
    values.iter().rev()
}

fn array_key(value: &Value) -> Result<ArrayKey, String> {
    match value {
        Value::Int(n) => Ok(ArrayKey::Integer(*n)),
        Value::String(s) => Ok(ArrayKey::String(s.clone())),
        _ => Err("array key must be an int or string".into()),
    }
}

fn key_value(key: &ArrayKey) -> Value {
    match key {
        ArrayKey::Integer(n) => Value::Int(*n),
        ArrayKey::String(s) => Value::String(s.clone()),
    }
}

/// A dvar's text, or `None` when it does not exist.
fn dvar(world: &mut World, args: &[Value]) -> Result<Option<String>, String> {
    let name = string(args, 0)?.to_ascii_lowercase();
    Ok(world
        .resource::<crate::script::MatchScript>()
        .dvars
        .get(&name)
        .cloned())
}

fn default(args: &[Value], absent: impl FnOnce() -> Value) -> Result<Value, String> {
    Ok(match args.get(1) {
        None | Some(Value::Undefined) => absent(),
        Some(value) => value.clone(),
    })
}

/// The number a text starts with (after spaces), or 0.
fn leading_number(text: &str) -> f64 {
    let text = text.trim_start();
    let bytes = text.as_bytes();
    let mut end = usize::from(matches!(bytes.first(), Some(b'-' | b'+')));
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }
    text[..end].parse().unwrap_or(0.0)
}

fn unit(world: &mut World) -> f32 {
    (super::math::random(world) >> 8) as f32 / (1u32 << 24) as f32
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

fn truth(value: &Value) -> bool {
    match value {
        Value::Undefined => false,
        Value::Int(n) => *n != 0,
        Value::Float(n) => *n != 0.0,
        _ => true,
    }
}

/// Forward, right and up for pitch, yaw, roll in degrees.
fn angle_vectors(angles: [f32; 3]) -> [[f32; 3]; 3] {
    let [pitch, yaw, roll] = angles.map(f32::to_radians);
    let (sp, cp) = pitch.sin_cos();
    let (sy, cy) = yaw.sin_cos();
    let (sr, cr) = roll.sin_cos();
    [
        [cp * cy, cp * sy, -sp],
        [-sr * sp * cy + cr * sy, -sr * sp * sy - cr * cy, -sr * cp],
        [cr * sp * cy + sr * sy, cr * sp * sy - sr * cy, cr * cp],
    ]
}

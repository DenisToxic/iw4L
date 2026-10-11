//! Black Ops 2's `playfx`: a temporary event entity at the whole-unit origin,
//! turned by the angles the handler works out from forward and up.

use super::{int, vector};
use crate::script::{MatchScript, Namespace, NativeRegistry, RoundScript, Value};
use bevy_ecs::prelude::World;

/// The engine's float `Vec3Normalize`: a zero vector keeps its length.
fn normalize(v: [f32; 3]) -> ([f32; 3], f32) {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let scale = 1.0 / if -length >= 0.0 { 1.0 } else { length };
    (v.map(|c| c * scale), length)
}

fn atan2_degrees(y: f32, x: f32, degrees: f32) -> f32 {
    f64::from(y).atan2(f64::from(x)) as f32 * degrees * 0.318_309_87
}

/// The pitch of a vector, without wrapping.
fn signed_pitch(v: [f32; 3]) -> f32 {
    if v[1] == 0.0 && v[0] == 0.0 {
        return if -v[2] >= 0.0 { 90.0 } else { -90.0 };
    }
    atan2_degrees(v[2], (v[0] * v[0] + v[1] * v[1]).sqrt(), -180.0)
}

fn vector_angles(v: [f32; 3]) -> [f32; 3] {
    if v[1] == 0.0 && v[0] == 0.0 {
        return [if -v[2] < 0.0 { 270.0 } else { 90.0 }, 0.0, 0.0];
    }
    let wrap = |a: f32| if a < 0.0 { a + 360.0 } else { a };
    let yaw = wrap(atan2_degrees(v[1], v[0], 180.0));
    let pitch = wrap(atan2_degrees(
        v[2],
        (v[0] * v[0] + v[1] * v[1]).sqrt(),
        -180.0,
    ));
    [pitch, yaw, 0.0]
}

fn sin_cos(degrees: f32) -> (f32, f32) {
    let radians = f64::from(degrees * 0.017_453_292);
    (radians.sin() as f32, radians.cos() as f32)
}

/// The angles of an axis: its forward's, then the roll its left makes.
fn axis_angles(axis: [[f32; 3]; 3]) -> [f32; 3] {
    let mut angles = vector_angles(axis[0]);
    let left = axis[1];
    let (sin, cos) = sin_cos(-angles[1]);
    let y = left[0] * sin + left[1] * cos;
    let x = left[0] * cos - left[1] * sin;
    let (sin, cos) = sin_cos(-angles[0]);
    let rolled = [left[2] * sin + x * cos, y, left[2] * cos - x * sin];
    let roll = signed_pitch(rolled);
    angles[2] = if y >= 0.0 {
        -roll
    } else if roll < 0.0 {
        roll + 180.0
    } else {
        roll - 180.0
    };
    angles
}

/// Forward, left and up of a set of angles.
fn angles_axis(angles: [f32; 3]) -> [[f32; 3]; 3] {
    let (sp, cp) = sin_cos(angles[0]);
    let (sy, cy) = sin_cos(angles[1]);
    let (sr, cr) = sin_cos(angles[2]);
    [
        [cp * cy, cp * sy, -sp],
        [sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, sr * cp],
        [cr * sp * cy + sr * sy, cr * sp * sy - sr * cy, cr * cp],
    ]
}

fn effect_name(world: &World, index: i32) -> Option<String> {
    world
        .resource::<MatchScript>()
        .precached
        .iter()
        .find(|((kind, _), id)| *kind == "fx" && **id == index)
        .map(|((_, name), _)| name.clone())
}

fn param_error(world: &World, index: i32, text: &str) -> String {
    let effect = effect_name(world, index).unwrap_or_else(|| "not successfully loaded".into());
    format!("{text} (effect = {effect})\n")
}

fn playfx(world: &mut World, args: &[Value]) -> Result<Value, String> {
    let index = int(args, 0)? & 0xff;
    let origin = vector(args, 1)?.map(f32::trunc);
    let angles = match args.len() {
        2 => [270.0, 0.0, 0.0],
        _ => {
            let (forward, length) = normalize(vector(args, 2)?);
            if length == 0.0 {
                return Err(param_error(
                    world,
                    index,
                    "playFx called with (0 0 0) forward direction",
                ));
            }
            if args.len() == 3 {
                vector_angles(forward)
            } else {
                let (up, length) = normalize(vector(args, 3)?);
                if length == 0.0 {
                    return Err(param_error(
                        world,
                        index,
                        "playFx called with (0 0 0) up direction",
                    ));
                }
                let along = -(up[0] * forward[0] + up[1] * forward[1] + up[2] * forward[2]);
                let (up, length) = normalize(std::array::from_fn(|i| forward[i] * along + up[i]));
                if length == 0.0 {
                    return Err(
                        "forward and up vectors are the same direction or exact opposite directions"
                            .into(),
                    );
                }
                let left = [
                    up[1] * forward[2] - up[2] * forward[1],
                    up[2] * forward[0] - up[0] * forward[2],
                    up[0] * forward[1] - up[1] * forward[0],
                ];
                axis_angles([forward, left, up])
            }
        }
    };
    let Some(name) = effect_name(world, index) else {
        return Ok(Value::Undefined);
    };
    if world
        .resource::<RoundScript>()
        .restart
        .as_ref()
        .is_some_and(|plan| plan.absent_effects.contains(&name))
    {
        return Ok(Value::Undefined);
    }
    let axis = angles_axis(angles);
    let effect = crate::frame::FrameWorld::from_world(world).effect_name_index(&name);
    super::super::engine::play_fx_at(world, effect, origin, axis[0], axis[2]);
    Ok(Value::Undefined)
}

pub(super) fn register(registry: &mut NativeRegistry) {
    registry.register(Namespace::Function, "playfx", |world, _, args| {
        playfx(world, args)
    });
}

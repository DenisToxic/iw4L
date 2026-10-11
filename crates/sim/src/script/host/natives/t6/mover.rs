//! Black Ops 2's `vibrate`: a script mover's angles swing on a sine
//! trajectory, then return to where they started.

use super::{float, vector};
use crate::script::host::mechanics::{Mechanics, Motion, MotionPath};
use crate::script::{Namespace, NativeRegistry, RoundScript, Value};
use bevy_ecs::prelude::World;

const MOVERS: [&str; 4] = [
    "script_brushmodel",
    "script_model",
    "script_origin",
    "light",
];

fn normalize_180(angle: f32) -> f32 {
    let turns = angle * 0.002_777_777_8;
    (turns - (turns + 0.5).floor()) * 360.0
}

fn normalize_360(angle: f32) -> f32 {
    let turns = angle * 0.002_777_777_8;
    let angle = (turns - turns.floor()) * 360.0;
    if angle - 360.0 >= 0.0 {
        angle - 360.0
    } else {
        angle
    }
}

fn sin_cos(degrees: f32) -> (f32, f32) {
    let radians = f64::from(degrees * 0.017_453_292);
    (radians.sin() as f32, radians.cos() as f32)
}

/// Forward and left of a set of angles.
fn forward_left(angles: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let (sy, cy) = sin_cos(angles[1]);
    let (sp, cp) = sin_cos(angles[0]);
    let (sr, cr) = sin_cos(angles[2]);
    (
        [cp * cy, cp * sy, -sp],
        [sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, sr * cp],
    )
}

fn vibrate(world: &mut World, receiver: &Value, args: &[Value]) -> Result<Value, String> {
    let id = match receiver {
        Value::Object(id) if world.resource::<RoundScript>().entities.contains_key(id) => *id,
        _ => return Err("not an entity".into()),
    };
    let classname = world.resource::<RoundScript>().entities[&id]
        .classname
        .clone();
    if !MOVERS.contains(&&*classname) {
        return Err(format!(
            "entity {id} is not a script_brushmodel, script_model, script_origin, or light"
        ));
    }
    let direction = vector(args, 0)?;
    let amplitude = float(args, 1)?;
    let period = float(args, 2)?;
    let time = float(args, 3)?;
    let length =
        (direction[1] * direction[1] + direction[0] * direction[0] + direction[2] * direction[2])
            .sqrt();
    let scale = 1.0 / if -length >= 0.0 { 1.0 } else { length };
    let delta = direction.map(|c| c * scale * amplitude);
    let angles = match world
        .resource_mut::<RoundScript>()
        .object_field(id, "angles")
    {
        Value::Vector(angles) => angles,
        _ => [0.0; 3],
    };
    let (forward, left) = forward_left(angles);
    let now = super::super::super::players::now_ms(world);
    let phase_ms = now - i64::from((time * -1000.0) as i32);
    let period_ms = (period * 1000.0) as i32;
    let path = MotionPath::Sine {
        base: angles,
        delta: [
            forward[1] * delta[1] + forward[0] * delta[0] + forward[2] * delta[2],
            0.0,
            -(left[1] * delta[1] + left[0] * delta[0] + left[2] * delta[2]),
        ],
        phase_ms,
        period_ms,
        end: [
            normalize_180(angles[0]),
            normalize_360(angles[1]),
            normalize_180(angles[2]),
        ],
    };
    world.resource_mut::<Mechanics>().start(
        id,
        Motion {
            field: "angles",
            path,
            start_ms: now,
            duration_ms: phase_ms + i64::from(period_ms) - now,
            done: "rotatedone",
        },
    );
    Ok(Value::Undefined)
}

pub(super) fn register(registry: &mut NativeRegistry) {
    registry.register(Namespace::Method, "vibrate", vibrate);
}

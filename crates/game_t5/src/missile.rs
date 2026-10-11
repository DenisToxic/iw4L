//! Black Ops' missiles: a projectile weapon's rocket, moved every frame
//! along its trajectory and traced against what it may hit, and its
//! impact: the direct hit, the explosion and its blast.

use fastfile_t5::weapon_def::WeaponDefView;
use game_api::fire::{
    BulletHitType, BulletTrace, FireWorld, GAME_MISSILE_BYTES, GameMissile, GameTrajectory,
    MissileExplosion, MissileRun, NotifyArg, RadiusDamage, TrajectoryKind,
};
use game_api::movement::{ENTITY_NONE, ENTITY_WORLD};
use game_api::{Unknown, unknown};

use crate::fire::{DEG2RAD, Hit, MEANS, damage, hit_id};

/// What a missile stops at.
const MISSILE_MASK: u32 = 0x0280_e091;
const CONTENTS_WATER: u32 = 0x20;
/// Contents a missile flies through as if they were open sky.
const CONTENTS_MISSILE_PASS: u32 = 0x800;
const SURF_SKY: u32 = 0x4;
const SURF_WATER: i32 = 20;
const SURF_GLASS: i32 = 9;
/// A missile's direct hit and its blast.
const MOD_PROJECTILE: usize = 5;
const MOD_PROJECTILE_SPLASH: usize = 6;
const MOD_IMPACT: usize = 16;
/// `weaponType`: a projectile weapon.
const WEAPTYPE_PROJECTILE: i32 = 2;

fn missile_unknown() -> &'static Unknown {
    unknown!(
        "t5.weapons.missile",
        "a missile that accelerates, curves, destabilises, is guided, bounces or sticks flies straight instead",
        "Black Ops' missile acceleration, curvature, destabilisation, guidance, bounce and stick rules"
    )
}

fn missile_water_unknown() -> &'static Unknown {
    unknown!(
        "t5.weapons.missile_water",
        "a missile entering water makes no splash, and an explosion under water is not told apart",
        "Black Ops' missile splash and its water height test in the runtime"
    )
}

fn missile_glass_unknown() -> &'static Unknown {
    unknown!(
        "t5.weapons.missile_glass",
        "a missile does not break glass it flies into",
        "Black Ops' missile glass damage in the runtime"
    )
}

fn ai_missile_events_unknown() -> &'static Unknown {
    unknown!(
        "t5.ai.missile_events",
        "actors are not told of missiles flying near them or striking",
        "Black Ops' AI event broadcast for missiles and the actors' reaction to it"
    )
}

/// What the game keeps per missile beyond the shared fields.
#[derive(Clone, Copy, Default)]
struct State {
    /// How far it has flown, for its arming distance.
    travel: f32,
    /// It bounces (`forceBounce`).
    bounce: bool,
}

impl State {
    fn read(game: &[u8; GAME_MISSILE_BYTES]) -> Self {
        Self {
            travel: f32::from_le_bytes([game[0], game[1], game[2], game[3]]),
            bounce: game[4] != 0,
        }
    }

    fn write(self, game: &mut [u8; GAME_MISSILE_BYTES]) {
        game[0..4].copy_from_slice(&self.travel.to_le_bytes());
        game[4] = u8::from(self.bounce);
    }
}

fn mad(base: [f32; 3], v: [f32; 3], scale: f32) -> [f32; 3] {
    [
        v[0] * scale + base[0],
        v[1] * scale + base[1],
        v[2] * scale + base[2],
    ]
}

/// The trajectory's position at `time`.
pub(crate) fn evaluate(tr: &GameTrajectory, time: i32) -> [f32; 3] {
    let t = (time - tr.time) as f32 * 0.00100000005;
    match tr.kind {
        TrajectoryKind::Stationary => tr.base,
        TrajectoryKind::Linear => mad(tr.base, tr.delta, t),
        TrajectoryKind::Gravity(gravity) => {
            let mut at = mad(tr.base, tr.delta, t);
            at[2] -= gravity * t * t * 0.5;
            at
        }
    }
}

/// The trajectory's velocity at `time`.
pub(crate) fn evaluate_delta(tr: &GameTrajectory, time: i32) -> [f32; 3] {
    match tr.kind {
        TrajectoryKind::Stationary => [0.0; 3],
        TrajectoryKind::Linear => tr.delta,
        TrajectoryKind::Gravity(gravity) => {
            let t = (time - tr.time) as f32 * 0.00100000005;
            [tr.delta[0], tr.delta[1], tr.delta[2] - gravity * t]
        }
    }
}

/// The angles a direction points at.
fn vectoangles(v: [f32; 3]) -> [f32; 3] {
    let yaw = if v[1] == 0.0 && v[0] == 0.0 {
        0.0
    } else {
        movement_t5::math::vectoyaw(v)
    };
    [movement_t5::math::vectopitch(v), yaw, 0.0]
}

/// The server's random point in the unit circle (not seeded by the shot).
fn random_in_circle(world: &mut dyn FireWorld) -> (f32, f32) {
    let r = |world: &mut dyn FireWorld| f64::from(world.rand()) * f64::from(3.05175781e-05f32);
    let angle = (r(world) * 360.0) as f32;
    let radius = r(world);
    let radians = f64::from(angle) * f64::from(DEG2RAD);
    let (sin, cos) = radians.sin_cos();
    ((cos as f32 * radius as f32), (radius as f32 * sin as f32))
}

/// A projectile weapon's shot: a rocket along the spread, and the shooter
/// pushed back by any weapon not firing fully automatic.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fire_rocket(
    attacker: i32,
    weapon: u32,
    def: &WeaponDefView<'_>,
    spread: f32,
    forward: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    muzzle: [f32; 3],
    world: &mut dyn FireWorld,
    gaps: &mut Vec<&'static Unknown>,
) {
    let aim = (f64::from(spread * DEG2RAD).tan() as f32) * 16.0;
    let (x, y) = random_in_circle(world);
    let (x, y) = (x * aim, y * aim);
    let dir = [
        up[0] * y + right[0] * x + forward[0] * 16.0,
        up[1] * y + right[1] * x + forward[1] * 16.0,
        up[2] * y + right[2] * x + forward[2] * 16.0,
    ];
    let len = (dir[2] * dir[2] + dir[1] * dir[1] + dir[0] * dir[0]).sqrt();
    let len = if -len >= 0.0 { 1.0 } else { len };
    let inv = 1.0 / len;
    let dir = [dir[0] * inv, dir[1] * inv, dir[2] * inv];
    spawn_rocket(attacker, weapon, def, muzzle, dir, [0.0; 3], world, gaps);
    if def.fire_type() != 0 {
        let z = (-forward[2]).min(0.0);
        world.push_player(attacker, [-forward[0] * 64.0, -forward[1] * 64.0, z * 64.0]);
    }
}

/// A rocket leaving the muzzle: straight along its direction at the
/// weapon's speed, drawn once clear of the shooter.
#[allow(clippy::too_many_arguments)]
fn spawn_rocket(
    owner: i32,
    weapon: u32,
    def: &WeaponDefView<'_>,
    start: [f32; 3],
    dir: [f32; 3],
    gun_velocity: [f32; 3],
    world: &mut dyn FireWorld,
    gaps: &mut Vec<&'static Unknown>,
) {
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let len = if -len >= 0.0 { 1.0 } else { len };
    let inv = 1.0 / len;
    let dir = [dir[0] * inv, dir[1] * inv, dir[2] * inv];
    if def.guided_missile_type() != 0
        || def.time_to_accelerate() > 0.0
        || def.projectile_curvature() > 0.0
    {
        gaps.push(missile_unknown());
    }
    let level = world.level_time();
    let speed = def.projectile_speed() as f32;
    let hidden = ((speed * -35.0 * 0.00166666671 + 85.0) as i32).clamp(20, 50);
    let delta = [
        ((dir[0] * speed) + gun_velocity[0]) as i32 as f32,
        ((speed * dir[1]) + gun_velocity[1]) as i32 as f32,
        ((def.projectile_speed_up() as f32 + speed * dir[2]) + gun_velocity[2]) as i32 as f32,
    ];
    let angles = vectoangles(dir);
    let mut game = [0u8; GAME_MISSILE_BYTES];
    State {
        travel: 0.0,
        bounce: def.force_bounce(),
    }
    .write(&mut game);
    world.spawn_missile(GameMissile {
        entity: ENTITY_NONE,
        owner,
        weapon,
        position: GameTrajectory {
            kind: TrajectoryKind::Linear,
            time: level,
            duration: 0,
            base: start,
            delta,
        },
        angles: GameTrajectory {
            kind: TrajectoryKind::Stationary,
            time: level,
            duration: 0,
            base: angles,
            delta: [0.0; 3],
        },
        origin: start,
        visible_at: level + hidden,
        game,
    });
}

/// One frame of a missile: along its trajectory, traced against what its
/// mask names (water too once clear of the muzzle), into its impact.
pub(crate) fn run_missile(
    missile: &mut GameMissile,
    world: &mut dyn FireWorld,
) -> (MissileRun, Vec<&'static Unknown>) {
    let mut gaps = vec![ai_missile_events_unknown()];
    let Some((def_bytes, variant_bytes)) = world
        .weapon(missile.weapon)
        .map(|w| (w.def.clone(), w.variant.clone()))
    else {
        gaps.push(crate::movement::weapon_unknown());
        return (MissileRun::Gone, gaps);
    };
    let Some(def) = WeaponDefView::new(&def_bytes, &variant_bytes) else {
        gaps.push(crate::movement::weapon_unknown());
        return (MissileRun::Gone, gaps);
    };
    let mut state = State::read(&missile.game);
    if state.bounce {
        gaps.push(missile_unknown());
    }
    let level = world.level_time();
    let old = missile.origin;
    let new = evaluate(&missile.position, level);
    let d = [new[0] - old[0], new[1] - old[1], new[2] - old[2]];
    let len = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
    if len < 0.00100000005 {
        return (MissileRun::Flying, gaps);
    }
    if def.weapon_type() == WEAPTYPE_PROJECTILE && def.proj_explosion_type() == 9 {
        gaps.push(missile_unknown());
    }
    let mask = if len <= 20.0 || world.point_contents(old, CONTENTS_WATER) != 0 {
        MISSILE_MASK
    } else {
        MISSILE_MASK | CONTENTS_WATER
    };
    let mut trace = missile_trace(old, new, missile.owner, mask, world);
    if surface_type(&trace) == SURF_WATER {
        gaps.push(missile_water_unknown());
        trace = missile_trace(old, new, missile.owner, MISSILE_MASK, world);
    }
    if surface_type(&trace) == SURF_GLASS {
        gaps.push(missile_glass_unknown());
    }
    let f = trace.fraction;
    let end = [
        (new[0] - old[0]) * f + old[0],
        (new[1] - old[1]) * f + old[1],
        (new[2] - old[2]) * f + old[2],
    ];
    missile.origin = end;
    if def.projectile_activate_dist() > 0 {
        let m = [end[0] - old[0], end[1] - old[1], end[2] - old[2]];
        state.travel += (m[2] * m[2] + m[1] * m[1] + m[0] * m[0]).sqrt();
    }
    state.write(&mut missile.game);
    if f == 1.0 {
        if def.destabilize_distance() > 0 && def.destabilization_rate_time() != 0.0 {
            gaps.push(missile_unknown());
        }
    } else {
        if trace.surface_flags & SURF_SKY != 0 || trace.contents & CONTENTS_MISSILE_PASS != 0 {
            missile.origin = new;
            return (MissileRun::Flying, gaps);
        }
        let inv = 1.0 / if -len >= 0.0 { 1.0 } else { len };
        let dir = [d[0] * inv, d[1] * inv, d[2] * inv];
        if let Some(run) = impact(missile, &def, &trace, dir, end, state, world, &mut gaps) {
            return (run, gaps);
        }
    }
    if !in_world(missile.origin, world.map_center()) {
        return (MissileRun::Gone, gaps);
    }
    (MissileRun::Flying, gaps)
}

/// A missile's trace; from inside a solid, it reports the way it came.
fn missile_trace(
    start: [f32; 3],
    end: [f32; 3],
    owner: i32,
    mask: u32,
    world: &mut dyn FireWorld,
) -> BulletTrace {
    let mut trace = world.bullet_trace(start, end, owner, mask);
    if trace.start_solid {
        trace.fraction = 0.0;
        let v = [start[0] - end[0], start[1] - end[1], start[2] - end[2]];
        let len = (v[0] * v[0] + v[2] * v[2] + v[1] * v[1]).sqrt();
        let len = if -len >= 0.0 { 1.0 } else { len };
        let inv = 1.0 / len;
        trace.normal = [inv * v[0], inv * v[1], inv * v[2]];
    }
    trace
}

fn surface_type(trace: &BulletTrace) -> i32 {
    (trace.surface_flags >> 20) as i32 & 0x3f
}

/// Within the bounds a missile may fly from the map's centre.
fn in_world(at: [f32; 3], center: [f32; 3]) -> bool {
    let reach_z = (1 << 16) as f32;
    let reach = (1 << 17) as f32;
    let (x, y, z) = (at[0] as i32, at[1] as i32, at[2] as i32);
    z <= (center[2] + reach_z) as i32
        && (center[2] - reach_z) as i32 <= z
        && x <= (center[0] + reach) as i32
        && (center[0] - reach) as i32 <= x
        && y <= (center[1] + reach) as i32
        && (center[1] - reach) as i32 <= y
}

/// A missile striking: its direct hit, its explosion and the blast; `None`
/// when it flies on (glass).
#[allow(clippy::too_many_arguments)]
fn impact(
    missile: &mut GameMissile,
    def: &WeaponDefView<'_>,
    trace: &BulletTrace,
    dir: [f32; 3],
    end: [f32; 3],
    state: State,
    world: &mut dyn FireWorld,
    gaps: &mut Vec<&'static Unknown>,
) -> Option<MissileRun> {
    if matches!(trace.hit, BulletHitType::Glass(_)) {
        gaps.push(missile_glass_unknown());
        return None;
    }
    let target = hit_id(trace);
    let level = world.level_time();
    let unarmed =
        def.projectile_activate_dist() > 0 && state.travel <= def.projectile_activate_dist() as f32;
    let explodes = def.proj_impact_explode() && !unarmed;
    let means = if explodes { MOD_PROJECTILE } else { MOD_IMPACT };
    let amount = def.damage();
    let hit = world.entity(target);
    if let Some(entity) = hit.filter(|e| e.takes_damage)
        && amount > 0
    {
        let mut velocity = evaluate_delta(&missile.position, level);
        if velocity[2] * velocity[2] + velocity[1] * velocity[1] + velocity[0] * velocity[0]
            < 0.00100000005
        {
            velocity = [0.0, 0.0, 1.0];
        }
        damage(
            gaps,
            missile.owner,
            missile.entity,
            entity,
            Hit {
                target,
                dir: velocity,
                point: missile.origin,
                amount,
                flags: 0,
                means,
                weapon: missile.weapon,
                location: trace.hit_location,
                bone: trace.bone,
            },
            world,
        );
    }
    if !explodes {
        // A missile that does not explode on impact bounces or sticks.
        gaps.push(missile_unknown());
        return Some(MissileRun::Gone);
    }
    if amount != 0 {
        world.damage_triggers(missile.owner, missile.origin, end, amount, MEANS[means]);
    }
    let normal = if def.proj_explosion_effect_force_normal_up() {
        [0.0, 0.0, 1.0]
    } else {
        trace.normal
    };
    gaps.push(missile_water_unknown());
    world.explosion(&MissileExplosion {
        missile: missile.entity,
        owner: missile.owner,
        weapon: missile.weapon,
        origin: end,
        normal,
        surface_type: surface_type(trace) as u8,
    });
    if def.explosion_inner_damage() != 0 {
        let cone = (def.damage_cone_angle() - 180.0 < 0.0)
            .then(|| ((def.damage_cone_angle() * DEG2RAD) as f64).cos() as f32);
        // The entity struck directly is spared the blast, unless it has its
        // own blast rule.
        let ignore = match hit {
            Some(entity) if entity.blast_handler => ENTITY_WORLD,
            _ => target,
        };
        gaps.extend(crate::radius::radius_damage(
            &RadiusDamage {
                origin: end,
                inflictor: missile.entity,
                attacker: missile.owner,
                inner: def.explosion_inner_damage() as f32,
                outer: def.explosion_outer_damage() as f32,
                radius: def.explosion_radius() as f32,
                cone: cone.map(|cos| (cos, dir)),
                ignore,
                means: MEANS[MOD_PROJECTILE_SPLASH],
                weapon: missile.weapon,
                script: false,
            },
            world,
        ));
        world.notify(
            missile.owner,
            "projectile_impact",
            &[
                NotifyArg::Weapon(missile.weapon),
                NotifyArg::Vector(end),
                NotifyArg::Float(def.explosion_radius() as f32),
                NotifyArg::Entity(missile.entity),
            ],
        );
    }
    if def.proj_explosion_type() == 2 {
        gaps.push(crate::radius::blast_effects_unknown());
    }
    missile.origin = end;
    Some(MissileRun::Gone)
}

//! What a game's server does with its players' fire events: the game runs
//! its own shot (spread, bullets, penetration, damage) and the match traces
//! and applies it through [`FireWorld`].

use crate::movement::GameWeapon;

/// What a bullet trace ran into, in the game's trace terms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BulletHitType {
    /// Nothing: the bullet reached its end.
    #[default]
    None,
    /// An entity, or the world itself as [`crate::movement::ENTITY_WORLD`].
    Entity(i32),
    /// A breakable glass piece, by number.
    Glass(i32),
}

/// One bullet trace.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BulletTrace {
    pub fraction: f32,
    pub normal: [f32; 3],
    /// The surface's flags, the surface type among them, in the game's bits.
    pub surface_flags: u32,
    pub hit: BulletHitType,
    /// The hit location the trace reports, in the game's numbering (0 none).
    pub hit_location: i32,
    /// The bone hit, as the match numbers it on the entity's model; handed
    /// back with the damage.
    pub bone: Option<u16>,
    pub start_solid: bool,
    pub all_solid: bool,
}

/// A team, as the match keeps one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireTeam {
    Axis,
    Allies,
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireEntityKind {
    Player,
    Actor,
    Other,
}

/// An entity a shot reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireEntity {
    pub kind: FireEntityKind,
    pub takes_damage: bool,
    pub health: i32,
    pub team: Option<FireTeam>,
    /// An actor's `ignorelocationaldamage`.
    pub ignore_locational_damage: bool,
    /// An actor's `allowdeath`.
    pub allow_death: bool,
    /// God mode: no damage at all.
    pub god: bool,
    /// Demigod: damage never takes the last point of health.
    pub demigod: bool,
    /// An actor's `type`, when it is one the game names.
    pub ai_type: Option<&'static str>,
}

/// Damage a shot does to one entity, as the game settled it.
#[derive(Clone, Debug, PartialEq)]
pub struct FireDamage {
    pub target: i32,
    pub inflictor: i32,
    pub attacker: i32,
    pub dir: [f32; 3],
    pub point: [f32; 3],
    pub amount: i32,
    pub flags: i32,
    /// The means of death, by its script name.
    pub means: &'static str,
    pub weapon: u32,
    /// The hit location, by its script name.
    pub hit_location: &'static str,
    pub bone: Option<u16>,
}

/// A bullet's mark on what it hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BulletImpact {
    pub attacker: i32,
    pub weapon: u32,
    pub start: [f32; 3],
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub surface_type: u8,
    pub surface_flags: u32,
    /// The entity hit; [`crate::movement::ENTITY_NONE`] for none.
    pub entity: i32,
    /// The game's impact flags (a head hit, a kill, an exit).
    pub flags: u8,
    /// Where the bullet left a surface it went through.
    pub exit: bool,
    /// The game's server sends this impact; otherwise its client draws it.
    pub from_server: bool,
}

/// The match around a shot.
pub trait FireWorld {
    /// The match's time, in milliseconds.
    fn level_time(&self) -> i32;

    /// A dvar's value; `None` when the match does not keep it.
    fn dvar(&self, name: &str) -> Option<String>;

    /// The server's shared random draw, 0 to 32767: the C library's
    /// generator over the match's state.
    fn rand(&mut self) -> i32;

    fn weapon(&self, weapon: u32) -> Option<&GameWeapon>;

    /// Traces a bullet from `start` to `end`, passing through `pass_entity`.
    fn bullet_trace(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        pass_entity: i32,
        mask: u32,
    ) -> BulletTrace;

    fn entity(&mut self, entity: i32) -> Option<FireEntity>;

    /// How deep a bullet of `penetrate_type` goes into `surface_type`, from
    /// the game's penetration table; `None` when the match has no table.
    fn penetration_depth(&self, penetrate_type: i32, surface_type: i32) -> Option<f32>;

    /// The damage multiplier at a hit location, from the game's table; `None`
    /// when the match has no table.
    fn hit_location_multiplier(&self, hit_location: i32) -> Option<f32>;

    /// Hands damage to the entity: an actor's damage callback, a player's,
    /// or the entity's own health and notifies.
    fn damage(&mut self, damage: &FireDamage);

    /// Damage triggers the bullet's path crosses.
    fn damage_triggers(
        &mut self,
        attacker: i32,
        start: [f32; 3],
        end: [f32; 3],
        amount: i32,
        means: &'static str,
    );

    fn impact(&mut self, impact: &BulletImpact);

    /// Notifies `name` on an entity, with the weapon's name when given.
    fn notify(&mut self, entity: i32, name: &str, weapon: Option<u32>);
}

//! A game's own shots in the match: the bullet traces, entities, damage and
//! notifies its server fire rules ask for, through the runtime's collision
//! and script host.

use crate::bullet_collision::{BulletTraceQuery, ColliderId, LagcompQuery, TraceOutcome};
use crate::frame::FrameWorld;
use crate::identities::ShotId;
use crate::world::{ClientId, Tick};
use crate::{ScriptModelId, SimContent};
use game_api::fire::{
    BulletHitType, BulletImpact, BulletTrace, FireDamage, FireEntity, FireEntityKind, FireTeam,
    FireWorld,
};
use game_api::movement::{ENTITY_NONE, ENTITY_WORLD, GameWeapon};
use std::collections::HashMap;
use std::sync::Arc;

/// The match around one player's shots in one command.
pub(crate) struct MatchFireWorld<'w, 'a> {
    world: &'w mut FrameWorld<'a>,
    content: Arc<SimContent>,
    tick: Tick,
    query: LagcompQuery,
    shot: ShotId,
    segment: u16,
    /// Script entities the traces reached, by the number the game knows.
    models: Vec<(i32, ScriptModelId)>,
}

impl<'w, 'a> MatchFireWorld<'w, 'a> {
    /// The match as `attacker` saw it when it fired; as it stands for a
    /// blast with no shooter.
    pub(crate) fn new(
        world: &'w mut FrameWorld<'a>,
        tick: Tick,
        attacker: Option<ClientId>,
    ) -> Self {
        world.record_collision_history(tick, 0, None);
        world.record_entity_collision_history(tick);
        let query = world.lagcomp_query_for(attacker.unwrap_or(ClientId(u32::MAX)), tick);
        let shot = world.alloc_shot_id();
        Self {
            content: world.content(),
            world,
            tick,
            query,
            shot,
            segment: 0,
            models: Vec::new(),
        }
    }

    fn model(&self, entity: i32) -> Option<ScriptModelId> {
        self.models
            .iter()
            .find(|(number, _)| *number == entity)
            .map(|(_, id)| *id)
    }

    fn client(&self, entity: i32) -> Option<ClientId> {
        let id = ClientId(u32::try_from(entity).ok()?);
        self.world.player(id).is_some().then_some(id)
    }

    /// The number the game knows a script entity by.
    fn number_of(&mut self, id: ScriptModelId) -> i32 {
        let number = self.world.gentity_number(id).unwrap_or(ENTITY_NONE);
        if number != ENTITY_NONE && self.model(number).is_none() {
            self.models.push((number, id));
        }
        number
    }

    fn trace_of(&mut self, outcome: TraceOutcome) -> BulletTrace {
        let (fraction, normal, collider, start_solid, all_solid) = match outcome {
            TraceOutcome::Miss { .. } | TraceOutcome::Invalid { .. } => {
                return BulletTrace {
                    fraction: 1.0,
                    ..BulletTrace::default()
                };
            }
            TraceOutcome::Hit {
                fraction,
                normal,
                collider,
                ..
            } => (fraction, normal, Some(collider), false, false),
            TraceOutcome::StartSolid {
                collider,
                all_solid,
                ..
            } => (0.0, [0.0; 3], collider, true, all_solid),
        };
        let mut trace = BulletTrace {
            fraction,
            normal,
            start_solid,
            all_solid,
            ..BulletTrace::default()
        };
        match collider {
            None => trace.hit = BulletHitType::Entity(ENTITY_WORLD),
            Some(ColliderId::World {
                surface_flags,
                glass_encoded,
                ..
            }) => {
                trace.surface_flags = surface_flags;
                trace.hit = match glass_encoded {
                    0 => BulletHitType::Entity(ENTITY_WORLD),
                    encoded => BulletHitType::Glass(i32::from(encoded) - 1),
                };
            }
            Some(ColliderId::Player { client, hitloc, .. }) => {
                trace.hit = BulletHitType::Entity(client.0 as i32);
                trace.hit_location = i32::from(hitloc);
            }
            Some(ColliderId::EntityDObjBone {
                owner,
                bone,
                part_classification,
                surface_flags,
            }) => {
                trace.surface_flags = surface_flags;
                trace.hit_location = i32::from(part_classification);
                trace.bone = Some(bone);
                trace.hit = match owner.script_model() {
                    Some(id) => BulletHitType::Entity(self.number_of(id)),
                    None => BulletHitType::Entity(ENTITY_WORLD),
                };
            }
            Some(ColliderId::EntityLinkedBrush {
                owner,
                surface_flags,
                ..
            }) => {
                trace.surface_flags = surface_flags;
                trace.hit = match owner.script_model() {
                    Some(id) => BulletHitType::Entity(self.number_of(id)),
                    None => BulletHitType::Entity(ENTITY_WORLD),
                };
            }
        }
        trace
    }
}

fn team_of(name: Option<&str>) -> Option<FireTeam> {
    match name? {
        "axis" => Some(FireTeam::Axis),
        "allies" => Some(FireTeam::Allies),
        "neutral" => Some(FireTeam::Neutral),
        _ => None,
    }
}

impl FireWorld for MatchFireWorld<'_, '_> {
    fn level_time(&self) -> i32 {
        crate::corpse::level_time_ms(self.tick)
    }

    fn dvar(&self, name: &str) -> Option<String> {
        self.world
            .ecs_ref()
            .get_resource::<crate::script::MatchScript>()?
            .dvars
            .get(name)
            .cloned()
    }

    fn rand(&mut self) -> i32 {
        let state = self.world.stuck_holdrand_mut();
        *state = state.wrapping_mul(0x0003_43fd).wrapping_add(0x0026_9ec3);
        (*state >> 17) as i32
    }

    fn weapon(&self, weapon: u32) -> Option<&GameWeapon> {
        self.content.game_weapon(weapon)
    }

    fn bullet_trace(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        pass_entity: i32,
        mask: u32,
    ) -> BulletTrace {
        let glass: HashMap<u32, u16> = self
            .world
            .world_objects()
            .glass_damage_pairs()
            .into_iter()
            .collect();
        let outcome = crate::bullet_collision::bullet_trace_with_entity_models(
            self.world.clip_brushes(),
            self.world.clip_bsp(),
            self.world.clip_cmodels(),
            self.world.clip_mesh(),
            &self.query.players.poses,
            &self.query.entities.rows,
            &BulletTraceQuery {
                start,
                end,
                mask,
                ignore: self.client(pass_entity),
                ignore_hit: None,
                ignore_model: self.model(pass_entity),
            },
            &|piece| {
                crate::world_objects::glass_piece_is_solid(
                    glass.get(&u32::from(piece)).copied().unwrap_or(0),
                )
            },
        );
        self.trace_of(outcome)
    }

    fn entity(&mut self, entity: i32) -> Option<FireEntity> {
        if let Some(client) = self.client(entity) {
            let meta = self.world.client_meta(client)?;
            let alive = meta.lifecycle == crate::match_state::ClientLifecycle::Alive;
            let team = team_of(crate::spawn::team_name(meta.client_state_team));
            let ps = self.world.player(client)?;
            // A sentient links one unit wider than its bounds.
            let bounds = self.world.player_area_bounds(client).map(|b| {
                let (mid, half) = (b.mid(), b.half());
                (
                    core::array::from_fn(|i| mid[i] - half[i] - 1.0),
                    core::array::from_fn(|i| mid[i] + half[i] + 1.0),
                )
            });
            return Some(FireEntity {
                kind: FireEntityKind::Player,
                origin: ps.origin,
                angles: ps.viewangles,
                contents: 0,
                bounds,
                model_bounds: None,
                eye: None,
                script_model: false,
                blast_handler: false,
                takes_damage: alive,
                health: ps.health,
                team,
                ignore_locational_damage: false,
                allow_death: true,
                god: false,
                demigod: false,
                ai_type: None,
            });
        }
        let id = self.model(entity)?;
        let target = crate::script::shot_target(self.world.ecs(), id)?;
        Some(FireEntity {
            kind: if target.actor {
                FireEntityKind::Actor
            } else {
                FireEntityKind::Other
            },
            origin: target.origin,
            angles: target.angles,
            contents: target.contents,
            bounds: target.brush.map(|(world, _)| world),
            model_bounds: target.brush.map(|(_, local)| local),
            eye: target.eye,
            script_model: target.script_model,
            blast_handler: target.destructible,
            takes_damage: target.takes_damage,
            health: target.health,
            team: team_of(target.team),
            ignore_locational_damage: target.ignore_locational_damage,
            allow_death: target.allow_death,
            god: false,
            demigod: false,
            ai_type: target.ai_type,
        })
    }

    fn penetration_depth(&self, penetrate_type: i32, surface_type: i32) -> Option<f32> {
        if !self.world.pen_table_loaded() {
            return None;
        }
        Some(
            self.world
                .penetration_table()
                .depth(penetrate_type, u32::try_from(surface_type).ok()?),
        )
    }

    fn hit_location_multiplier(&self, hit_location: i32) -> Option<f32> {
        self.content
            .hit_location_multiplier(usize::try_from(hit_location).ok()?)
    }

    fn damage(&mut self, damage: &FireDamage) {
        if let Some(victim) = self.client(damage.target) {
            let attacker = self.client(damage.attacker);
            let commit = attacker.and_then(|attacker| {
                Some(crate::damage::DeathCommit {
                    victim,
                    victim_life: self.world.client_meta(victim)?.life_sequence,
                    attacker,
                    attacker_life: self.world.client_meta(attacker)?.life_sequence,
                    source: crate::DamageSource::Shot(self.shot),
                    pellet: crate::identities::PelletId(0),
                    weapon: damage.weapon,
                    amount: damage.amount,
                    killcam_entity_start_time: 0,
                    inflictor_origin: Some(damage.point),
                    hitloc: 0,
                })
            });
            let hit = crate::script_player::Hit {
                victim,
                attacker,
                attacker_object: None,
                amount: damage.amount,
                flags: damage.flags,
                means: damage.means,
                weapon: damage.weapon,
                point: damage.point,
                dir: damage.dir,
                hitloc: u8::try_from(damage.location).unwrap_or(0),
                inflictor: None,
                commit,
            };
            crate::script::game_player_damage(self.world.ecs(), self.tick, &hit);
            return;
        }
        let Some(target) = self.model(damage.target) else {
            return;
        };
        if let Some(bone) = damage.bone
            && crate::t5_destructible::apply_hit(
                self.world,
                self.tick,
                crate::AuthorityModelOwner::ScriptModel(target),
                bone,
                damage.amount.max(0) as u32,
                self.client(damage.attacker),
            )
        {
            return;
        }
        let attacker = self.client(damage.attacker);
        crate::script::damage_entity(
            self.world.ecs(),
            &crate::script::EntityHit {
                target,
                amount: damage.amount,
                attacker,
                means: damage.means,
                weapon: damage.weapon,
                point: damage.point,
                dir: damage.dir,
                bone: damage.bone.map(usize::from),
                flags: damage.flags,
                hit_location: Some(damage.hit_location),
            },
        );
    }

    fn damage_triggers(
        &mut self,
        attacker: i32,
        start: [f32; 3],
        end: [f32; 3],
        amount: i32,
        means: &'static str,
    ) {
        let Some(client) = self.client(attacker) else {
            return;
        };
        crate::script::host::triggers::damage_line(
            self.world.ecs(),
            start,
            end,
            amount,
            client,
            None,
            means,
        );
    }

    fn impact(&mut self, impact: &BulletImpact) {
        let segment = self.segment;
        self.segment = self.segment.saturating_add(1);
        let mut surface_flags = impact.surface_flags;
        if impact.exit {
            surface_flags |= crate::combat::IMPACT_EXIT_SURFACE;
        }
        self.world.push_pellet_fx(crate::PelletFxRecord {
            attacker: impact.attacker,
            weapon: impact.weapon,
            correlation: self.shot.0,
            fire_cause: None,
            pellet: 0,
            segment,
            hand: 0,
            start: impact.start,
            end: impact.point,
            normal: impact.normal,
            surf_type: impact.surface_type,
            surface_flags,
            flesh_flags: if impact.entity == ENTITY_NONE {
                0
            } else {
                impact.flags & 0x3
            },
        });
    }

    fn entities_in_box(&mut self, mins: [f32; 3], maxs: [f32; 3]) -> Vec<i32> {
        // Candidates: the game tests each one's own bounds. A body reaches at
        // most this far beyond its origin.
        const REACH: f32 = 128.0;
        let overlaps = |lo: [f32; 3], hi: [f32; 3]| (0..3).all(|i| lo[i] <= maxs[i] && hi[i] >= mins[i]);
        let mut out = Vec::new();
        for id in self.world.client_ids_sorted() {
            let Some(ps) = self.world.player(id) else {
                continue;
            };
            let o = ps.origin;
            if overlaps(o.map(|c| c - REACH), o.map(|c| c + REACH)) {
                out.push(id.0 as i32);
            }
        }
        for (id, origin) in crate::script::presented_origins(self.world.ecs()) {
            let near = overlaps(origin.map(|c| c - REACH), origin.map(|c| c + REACH));
            let brush = self
                .world
                .entity_collision_capabilities()
                .iter()
                .find(|row| row.owner.script_model() == Some(id))
                .and_then(|row| row.linked_brushes.first())
                .and_then(|brush| {
                    let model = self
                        .world
                        .clip_cmodels()
                        .models
                        .get(brush.cmodel_handle as usize)?;
                    let at = brush.origin;
                    Some(overlaps(
                        core::array::from_fn(|i| at[i] + model.mins[i] - model.radius),
                        core::array::from_fn(|i| at[i] + model.maxs[i] + model.radius),
                    ))
                });
            if near || brush == Some(true) {
                let number = self.number_of(id);
                if number != ENTITY_NONE {
                    out.push(number);
                }
            }
        }
        out
    }

    fn sight_clear(&mut self, start: [f32; 3], end: [f32; 3], pass: [i32; 2], mask: u32) -> bool {
        let ignore = pass.iter().find_map(|&e| self.client(e));
        let ignore_model = pass.iter().find_map(|&e| self.model(e));
        let outcome = crate::bullet_collision::bullet_trace_with_entity_models(
            self.world.clip_brushes(),
            self.world.clip_bsp(),
            self.world.clip_cmodels(),
            self.world.clip_mesh(),
            &self.query.players.poses,
            &self.query.entities.rows,
            &BulletTraceQuery {
                start,
                end,
                mask,
                ignore,
                ignore_hit: None,
                ignore_model,
            },
            &|_| true,
        );
        matches!(outcome, TraceOutcome::Miss { .. })
    }

    fn player(&self, entity: i32) -> Option<game_api::movement::MovePlayer> {
        let id = self.client(entity)?;
        let frozen = self.world.client_meta(id).is_some_and(|m| m.controls.frozen);
        Some(self.world.player(id)?.move_player(id.0 as i32, frozen))
    }

    fn players_ignore_radius_damage(&self) -> bool {
        self.world
            .ecs_ref()
            .get_resource::<crate::script::RoundScript>()
            .is_some_and(|runtime| runtime.engine.players_ignore_radius_damage)
    }

    fn notify(&mut self, entity: i32, name: &str, weapon: Option<u32>) {
        let Some(client) = self.client(entity) else {
            return;
        };
        let weapon = weapon.map(|w| crate::script_player::weapon_name(self.world, w));
        crate::script::notify_player(self.world.ecs(), client.0, name, weapon.as_deref());
    }
}

/// A script's RadiusDamage by the game's own blast rules; false when the
/// match runs no game rules for it.
pub(crate) fn script_blast(
    world: &mut FrameWorld,
    tick: Tick,
    blast: &crate::script::ScriptBlast,
) -> bool {
    let Some(movement) = crate::game_move::rules(world) else {
        return false;
    };
    if !crate::game_move::game_weapons(world) {
        return false;
    }
    let mut around = MatchFireWorld::new(world, tick, blast.attacker);
    let inflictor = blast
        .inflictor
        .map_or(ENTITY_NONE, |id| around.number_of(id));
    let attacker = blast.attacker.map_or(ENTITY_WORLD, |id| id.0 as i32);
    let rules = game_api::fire::RadiusDamage {
        origin: blast.origin,
        inflictor,
        attacker,
        inner: blast.max,
        outer: blast.min,
        radius: blast.radius,
        cone: None,
        ignore: inflictor,
        means: blast.means,
        weapon: blast.weapon,
        script: true,
    };
    let Some(gaps) = movement.radius_damage(&rules, &mut around) else {
        return false;
    };
    for gap in gaps {
        world.report_game_gap(gap);
    }
    true
}

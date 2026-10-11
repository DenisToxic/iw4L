use crate::walk::{LoadedAsset, decode_ptr};
use crate::{AssetType, ZoneLoad};
use alloc::string::String;
use alloc::vec::Vec;

const DEF_SIZE: usize = 560;
const BOARDS: usize = 6;
const BOARD_SIZE: usize = 80;
const BOARD_AT: usize = 80;

/// A zombies barrier type (`ZBarrierDef`): its pieces ("boards") and the
/// zombie animation states it offers.
pub struct Barrier {
    pub name: String,
    pub piece_count: i32,
    pub auto_hide_open_pieces: u32,
    pub taunts: u32,
    pub reach_through_attacks: u32,
    pub taunt_anim_state: String,
    pub reach_through_anim_state: String,
    pub attack_slots: i32,
    pub attack_slot_horz_offset: f32,
    pub pieces: Vec<BarrierPiece>,
}

pub struct BarrierPiece {
    /// The default, alternate and upgraded models.
    pub models: [Option<String>; 3],
    /// The animation a piece plays opening (torn off) and closing (repaired).
    pub tear_anim: Option<BarrierAnim>,
    pub board_anim: Option<BarrierAnim>,
    pub anim_state: String,
    pub anim_substate: String,
    pub upgraded_lives: u32,
}

/// An animation by name, with the frame count and rate its length comes from
/// when the same zone holds it.
pub struct BarrierAnim {
    pub name: String,
    pub timing: Option<(u16, f32)>,
}

impl ZoneLoad {
    /// The zone's barrier types.
    pub fn barriers(&self) -> Vec<Barrier> {
        self.assets
            .iter()
            .filter(|asset| asset.ty == AssetType::ZBarrier && asset.header.len() >= DEF_SIZE)
            .filter_map(|asset| self.barrier(asset))
            .collect()
    }

    fn barrier(&self, asset: &LoadedAsset) -> Option<Barrier> {
        let h = &asset.header;
        let u32_at = |o: usize| u32::from_le_bytes(h[o..o + 4].try_into().unwrap());
        let u16_at = |o: usize| u16::from_le_bytes([h[o], h[o + 1]]);
        let text = |o: usize| String::from(self.script_string(u16_at(o)).unwrap_or(""));
        let piece_count = u32_at(52) as i32;
        let pieces = (0..BOARDS)
            .map(|board| {
                let at = BOARD_AT + BOARD_SIZE * board;
                let model = |k: usize| self.inline_asset_name(asset, at + 4 * k);
                let anim = |o: usize| self.anim_timing(self.header_str(h, o)?);
                BarrierPiece {
                    models: [model(0), model(1), model(2)],
                    tear_anim: anim(at + 12),
                    board_anim: anim(at + 16),
                    anim_state: text(at + 72),
                    anim_substate: text(at + 74),
                    upgraded_lives: u32_at(at + 76),
                }
            })
            .collect();
        Some(Barrier {
            name: String::from(self.header_str(h, 0)?),
            piece_count,
            auto_hide_open_pieces: u32_at(56),
            taunts: u32_at(60),
            reach_through_attacks: u32_at(64),
            taunt_anim_state: text(68),
            reach_through_anim_state: text(70),
            attack_slots: u32_at(72) as i32,
            attack_slot_horz_offset: f32::from_bits(u32_at(76)),
            pieces,
        })
    }

    fn header_str<'a>(&'a self, header: &[u8], at: usize) -> Option<&'a str> {
        let raw = u32::from_le_bytes(header.get(at..at + 4)?.try_into().ok()?);
        let p = decode_ptr(raw)?;
        core::str::from_utf8(self.blocks.cstr(p).ok()?).ok()
    }

    /// The name of the asset an asset's header points to; a name the zone
    /// only references is stored with a leading comma.
    fn inline_asset_name(&self, owner: &LoadedAsset, at: usize) -> Option<String> {
        let target = &self.assets[owner.field(at as u32)?];
        let name = self.header_str(&target.header, 0)?;
        Some(String::from(name.trim_start_matches(','))).filter(|name| !name.is_empty())
    }

    fn anim_timing(&self, name: &str) -> Option<BarrierAnim> {
        if name.is_empty() {
            return None;
        }
        let parts = self.assets.iter().find(|asset| {
            asset.ty == AssetType::XAnimParts
                && asset.header.len() >= 52
                && self
                    .header_str(&asset.header, 0)
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
        });
        Some(BarrierAnim {
            name: String::from(name),
            timing: parts.map(|p| {
                (
                    u16::from_le_bytes([p.header[14], p.header[15]]),
                    f32::from_le_bytes(p.header[48..52].try_into().unwrap()),
                )
            }),
        })
    }
}

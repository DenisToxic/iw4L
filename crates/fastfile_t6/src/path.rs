use crate::walk::decode_ptr;
use crate::{AssetType, ZoneLoad};
use alloc::string::String;
use alloc::vec::Vec;

const NODE_SIZE: u32 = 144;
const LINK_SIZE: u32 = 16;

/// The engine's name for each node type, as script reads `node.type`.
const TYPE_NAMES: [&str; 21] = [
    "BAD NODE",
    "Path",
    "Cover Stand",
    "Cover Crouch",
    "Cover Crouch Window",
    "Cover Prone",
    "Cover Right",
    "Cover Left",
    "Cover Pillar",
    "Ambush",
    "Exposed",
    "Conceal Stand",
    "Conceal Crouch",
    "Conceal Prone",
    "Reacquire",
    "Balcony",
    "Scripted",
    "Begin",
    "End",
    "Turret",
    "Guard",
];

/// A node of the game world's path network.
pub struct PathNode {
    pub kind: u32,
    pub spawnflags: u32,
    pub targetname: String,
    pub script_linkname: String,
    pub script_noteworthy: String,
    pub target: String,
    pub animscript: String,
    pub origin: [f32; 3],
    pub angle: f32,
    pub radius: f32,
    pub min_use_dist_sq: f32,
    pub links: Vec<PathLink>,
}

pub struct PathLink {
    pub distance: f32,
    pub node: u16,
    pub negotiation: bool,
}

impl PathNode {
    pub fn type_name(&self) -> &'static str {
        TYPE_NAMES.get(self.kind as usize).copied().unwrap_or("")
    }
}

impl ZoneLoad {
    /// The nodes of the zone's game world, in map order.
    pub fn path_nodes(&self) -> Vec<PathNode> {
        let Some(world) = self
            .assets
            .iter()
            .find(|asset| matches!(asset.ty, AssetType::GameWorldMp | AssetType::GameWorldSp))
        else {
            return Vec::new();
        };
        let word = |at: usize| {
            world
                .header
                .get(at..at + 4)
                .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
        };
        let count = word(4);
        let Some(nodes) = decode_ptr(word(12)) else {
            return Vec::new();
        };
        (0..count)
            .map_while(|index| self.path_node(nodes.at(index * NODE_SIZE)))
            .collect()
    }

    fn path_node(&self, at: crate::Ptr) -> Option<PathNode> {
        let bytes = self.blocks.bytes(at, 68).ok()?;
        let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let f32_at = |o: usize| f32::from_bits(u32_at(o));
        let text = |o: usize| String::from(self.script_string(u16_at(o)).unwrap_or(""));
        let link_count = u32::from(u16_at(60));
        let links = match decode_ptr(u32_at(64)) {
            Some(links) => (0..link_count)
                .map(|k| {
                    let link = self.blocks.bytes(links.at(k * LINK_SIZE), 8).ok()?;
                    Some(PathLink {
                        distance: f32::from_le_bytes(link[0..4].try_into().unwrap()),
                        node: u16::from_le_bytes([link[4], link[5]]),
                        negotiation: link[7] != 0,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
            None => Vec::new(),
        };
        Some(PathNode {
            kind: u32_at(0),
            spawnflags: u32_at(4),
            targetname: text(8),
            script_linkname: text(10),
            script_noteworthy: text(12),
            target: text(14),
            animscript: text(16),
            origin: [f32_at(24), f32_at(28), f32_at(32)],
            angle: f32_at(36),
            radius: f32_at(48),
            min_use_dist_sq: f32_at(52),
            links,
        })
    }
}

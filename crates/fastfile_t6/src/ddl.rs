use crate::walk::decode_ptr;
use crate::{AssetType, LoadedAsset, Ptr, ZoneLoad};
use alloc::string::String;
use alloc::vec::Vec;

/// A data definition (`ddl/…`): the layout of a bit-packed buffer such as a
/// player's stats, newest version first.
pub struct Ddl {
    pub name: String,
    pub versions: Vec<DdlDef>,
}

pub struct DdlDef {
    pub version: u32,
    /// The buffer's size in bytes.
    pub size: u32,
    /// The root is the first struct.
    pub structs: Vec<DdlStruct>,
    pub enums: Vec<DdlEnum>,
}

pub struct DdlStruct {
    pub name: String,
    pub size: u32,
    pub members: Vec<DdlMember>,
}

/// A member: its size and offset in bits, its type, the struct or enum it
/// names, the fraction bits of a fixed point float and its array length.
pub struct DdlMember {
    pub name: String,
    pub bits: u32,
    pub offset: u32,
    pub kind: u32,
    pub external: i32,
    pub range: u32,
    pub array_size: i32,
    pub enum_index: i32,
}

pub struct DdlEnum {
    pub name: String,
    pub members: Vec<String>,
}

const STRUCT_SIZE: u32 = 20;
const MEMBER_SIZE: u32 = 44;
const ENUM_SIZE: u32 = 16;

pub fn ddl(load: &ZoneLoad, asset: &LoadedAsset) -> Option<Ddl> {
    if asset.ty != AssetType::Ddl {
        return None;
    }
    let word = |at: usize| {
        asset
            .header
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    };
    let name = text(load, decode_ptr(word(0)?)?)?;
    let mut versions = Vec::new();
    let mut next = decode_ptr(word(4)?);
    while let Some(at) = next {
        let def = Reader { load, at };
        versions.push(DdlDef {
            version: def.u32(0)?,
            size: def.u32(4)?,
            structs: def.list(8, 12, STRUCT_SIZE, |s| {
                Some(DdlStruct {
                    name: s.text(0)?,
                    size: s.u32(4)?,
                    members: s.list(12, 8, MEMBER_SIZE, |m| {
                        Some(DdlMember {
                            name: m.text(0)?,
                            bits: m.u32(4)?,
                            offset: m.u32(8)?,
                            kind: m.u32(12)?,
                            external: m.u32(16)? as i32,
                            range: m.u32(20)?,
                            array_size: m.u32(32)? as i32,
                            enum_index: m.u32(36)? as i32,
                        })
                    })?,
                })
            })?,
            enums: def.list(16, 20, ENUM_SIZE, |e| {
                let count = e.u32(4)?;
                let names = decode_ptr(e.u32(8)?);
                let members = match names {
                    Some(names) => (0..count)
                        .map(|i| {
                            let p = load.blocks.ptr_at(names.at(4 * i)).ok()??;
                            text(load, p)
                        })
                        .collect::<Option<Vec<_>>>()?,
                    None => Vec::new(),
                };
                Some(DdlEnum {
                    name: e.text(0)?,
                    members,
                })
            })?,
        });
        next = decode_ptr(def.u32(24)?);
    }
    Some(Ddl { name, versions })
}

fn text(load: &ZoneLoad, at: Ptr) -> Option<String> {
    Some(String::from(
        core::str::from_utf8(load.blocks.cstr(at).ok()?).ok()?,
    ))
}

#[derive(Clone, Copy)]
struct Reader<'z> {
    load: &'z ZoneLoad,
    at: Ptr,
}

impl Reader<'_> {
    fn u32(&self, off: u32) -> Option<u32> {
        self.load.blocks.u32_at(self.at.at(off)).ok()
    }

    fn text(&self, off: u32) -> Option<String> {
        text(self.load, decode_ptr(self.u32(off)?)?)
    }

    /// The `count_off` count of `size` byte records the pointer at `list_off`
    /// points to.
    fn list<T>(
        &self,
        list_off: u32,
        count_off: u32,
        size: u32,
        read: impl Fn(Reader<'_>) -> Option<T>,
    ) -> Option<Vec<T>> {
        let count = self.u32(count_off)?;
        let Some(list) = decode_ptr(self.u32(list_off)?) else {
            return Some(Vec::new());
        };
        (0..count)
            .map(|i| {
                read(Reader {
                    load: self.load,
                    at: list.at(i * size),
                })
            })
            .collect()
    }
}

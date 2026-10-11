use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::{AssetType, ZoneLoad, walk::decode_ptr};

const NAME: usize = 0;
const LEN: usize = 4;
const BUFFER: usize = 8;

/// A compiled script module (`ScriptParseTree`) as the zone stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptAsset<'z> {
    pub name: &'z str,
    pub bytes: &'z [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    Header { index: usize },
    Name { index: usize },
    Buffer { name: String, len: u32 },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZoneScripts<'z> {
    /// Modules whose bytes this zone carries, in asset-list order.
    pub modules: Vec<ScriptAsset<'z>>,
    /// Modules this zone names but another zone carries (`,name`, no bytes).
    pub references: Vec<&'z str>,
    pub errors: Vec<ScriptError>,
}

pub fn scripts(load: &ZoneLoad) -> ZoneScripts<'_> {
    let mut out = ZoneScripts::default();
    for (index, asset) in load
        .assets
        .iter()
        .enumerate()
        .filter(|(_, asset)| asset.ty == AssetType::Script)
    {
        match script(load, &asset.header, index) {
            Ok(Entry::Module(module)) => out.modules.push(module),
            Ok(Entry::Reference(name)) => out.references.push(name),
            Err(error) => out.errors.push(error),
        }
    }
    out
}

enum Entry<'z> {
    Module(ScriptAsset<'z>),
    Reference(&'z str),
}

fn script<'z>(load: &'z ZoneLoad, header: &[u8], index: usize) -> Result<Entry<'z>, ScriptError> {
    let word = |at: usize| {
        header
            .get(at..at + 4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
            .ok_or(ScriptError::Header { index })
    };
    let name = decode_ptr(word(NAME)?)
        .and_then(|at| load.blocks.cstr(at).ok())
        .and_then(|bytes| core::str::from_utf8(bytes).ok())
        .ok_or(ScriptError::Name { index })?;
    let len = word(LEN)?;
    if let Some(name) = name.strip_prefix(',')
        && len == 0
    {
        return Ok(Entry::Reference(name));
    }
    let bytes = decode_ptr(word(BUFFER)?)
        .and_then(|at| load.blocks.bytes(at, len as usize).ok())
        .ok_or_else(|| ScriptError::Buffer {
            name: name.to_string(),
            len,
        })?;
    Ok(Entry::Module(ScriptAsset { name, bytes }))
}

/// A raw file (`RawFile`): its name and bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawFile<'z> {
    pub name: &'z str,
    pub bytes: &'z [u8],
}

pub fn raw_file<'z>(load: &'z ZoneLoad, asset: &crate::LoadedAsset) -> Option<RawFile<'z>> {
    if asset.ty != AssetType::RawFile {
        return None;
    }
    let word = |at: usize| {
        asset
            .header
            .get(at..at + 4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
    };
    let name = decode_ptr(word(NAME)?)
        .and_then(|at| load.blocks.cstr(at).ok())
        .and_then(|bytes| core::str::from_utf8(bytes).ok())?;
    let bytes =
        decode_ptr(word(BUFFER)?).and_then(|at| load.blocks.bytes(at, word(LEN)? as usize).ok())?;
    Some(RawFile { name, bytes })
}

/// A string table (`StringTable`): its name and cells, row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringTable<'z> {
    pub name: &'z str,
    pub columns: usize,
    pub rows: usize,
    pub cells: Vec<&'z str>,
}

const TABLE_COLUMNS: usize = 4;
const TABLE_ROWS: usize = 8;
const TABLE_VALUES: usize = 12;
/// Each cell: a string pointer and its hash.
const CELL_LEN: u32 = 8;

pub fn string_table<'z>(load: &'z ZoneLoad, asset: &crate::LoadedAsset) -> Option<StringTable<'z>> {
    if asset.ty != AssetType::StringTable {
        return None;
    }
    let word = |at: usize| {
        asset
            .header
            .get(at..at + 4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
    };
    let text = |p: Option<crate::Ptr>| {
        p.and_then(|p| load.blocks.cstr(p).ok())
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
    };
    let name = text(decode_ptr(word(NAME)?))?;
    let columns = usize::try_from(word(TABLE_COLUMNS)? as i32).ok()?;
    let rows = usize::try_from(word(TABLE_ROWS)? as i32).ok()?;
    let values = decode_ptr(word(TABLE_VALUES)?);
    let cells = (0..rows * columns)
        .map(|i| {
            let cell = values.map(|v| v.at(CELL_LEN * i as u32));
            text(cell.and_then(|c| load.blocks.ptr_at(c).ok().flatten())).unwrap_or("")
        })
        .collect();
    Some(StringTable {
        name,
        columns,
        rows,
        cells,
    })
}

use crate::{AssetType, LoadedAsset, RawFile, ZoneLoad, walk::decode_ptr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawFileError {
    Header,
    Name,
    Buffer,
}

pub fn read_raw_file<'a>(
    load: &'a ZoneLoad,
    asset: &LoadedAsset,
) -> Result<Option<RawFile<'a>>, RawFileError> {
    if asset.ty != AssetType::RawFile {
        return Ok(None);
    }
    let word = |at: usize| {
        asset
            .header
            .get(at..at + 4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
            .ok_or(RawFileError::Header)
    };
    let name = decode_ptr(word(0)?)
        .and_then(|at| load.blocks.cstr(at).ok())
        .and_then(|bytes| core::str::from_utf8(bytes).ok())
        .ok_or(RawFileError::Name)?;
    let length = word(4)? as usize;
    let pointer = word(8)?;
    if pointer == 0 && length == 0 {
        return Ok(None);
    }
    let bytes = decode_ptr(pointer)
        .and_then(|at| load.blocks.bytes(at, length).ok())
        .ok_or(RawFileError::Buffer)?;
    Ok(Some(RawFile { name, bytes }))
}

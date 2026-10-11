use std::sync::RwLock;

use asset_core::ZoneFormat;

static FORMATS: RwLock<Vec<ZoneFormat>> = RwLock::new(Vec::new());

/// The zone formats of games whose zones the session's game registry knows
/// and this crate does not open itself.
pub fn register_zone_formats(formats: Vec<ZoneFormat>) {
    *FORMATS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = formats;
}

pub fn registered_zone_format(game: asset_core::FamilyId) -> Option<ZoneFormat> {
    registered().into_iter().find(|format| format.game == game)
}

pub(crate) fn registered() -> Vec<ZoneFormat> {
    FORMATS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

static SCRIPT_FORMATS: RwLock<Vec<asset_core::ScriptZoneFormat>> = RwLock::new(Vec::new());

/// Where the games the session's game registry knows keep the compiled
/// scripts their maps run, for zones this crate opens itself.
pub fn register_script_zone_formats(formats: Vec<asset_core::ScriptZoneFormat>) {
    *SCRIPT_FORMATS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = formats;
}

pub fn registered_script_zone_format(
    game: asset_core::FamilyId,
) -> Option<asset_core::ScriptZoneFormat> {
    SCRIPT_FORMATS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .copied()
        .find(|format| format.game == game)
}

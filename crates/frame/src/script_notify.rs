use bevy::prelude::*;

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitLevelCalled;

/// The match's level loads again from the start.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelReloadCalled;

pub fn register_script_notify(app: &mut App) {
    app.add_message::<ExitLevelCalled>();
    app.add_message::<LevelReloadCalled>();
}

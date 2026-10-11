//! Where Black Ops 2 keeps the compiled scripts a zombies map runs.

use std::sync::OnceLock;

pub static SCRIPT_ZONE_FORMAT: asset_core::ScriptZoneFormat = asset_core::ScriptZoneFormat {
    game: asset_core::FamilyId::T6,
    script_zones,
    scripts,
};

/// The engine loads the common zone, its patch, the gametype's zone, the map
/// and the map's own patch, in that order.
fn script_zones(map: &str) -> Vec<String> {
    if !map.starts_with("zm_") {
        return Vec::new();
    }
    vec![
        "common_zm".to_owned(),
        "patch_zm".to_owned(),
        format!("so_zclassic_{map}"),
        map.to_owned(),
        format!("{map}_patch"),
    ]
}

fn schema() -> Result<&'static fastfile_t6::schema::Schema, String> {
    static SCHEMA: OnceLock<Result<fastfile_t6::schema::Schema, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| fastfile_t6::schema::parse().map_err(|error| format!("{error:?}")))
        .as_ref()
        .map_err(Clone::clone)
}

fn scripts(content: &[u8]) -> asset_core::ZoneScripts {
    let mut out = asset_core::ZoneScripts::default();
    let schema = match schema() {
        Ok(schema) => schema,
        Err(error) => {
            out.report.push(format!("t6 zone schema: {error}"));
            return out;
        }
    };
    let (load, walked) = fastfile_t6::load_zone(schema, content, |_, _| true);
    if let Err(error) = walked {
        out.report.push(format!("t6 zone walk: {error:?}"));
        return out;
    }
    for module in fastfile_t6::scripts(&load).modules {
        out.modules
            .push((module.name.to_owned(), module.bytes.to_vec()));
    }
    for asset in &load.assets {
        if let Some(file) = fastfile_t6::raw_file(&load, asset)
            && file.name.ends_with(".cfg")
            && let Ok(text) = std::str::from_utf8(file.bytes)
        {
            out.configs.push((file.name.to_owned(), text.to_owned()));
        }
        if let Some(table) = fastfile_t6::string_table(&load, asset) {
            out.tables.push(asset_core::ZoneTable {
                name: table.name.to_owned(),
                columns: table.columns,
                rows: table.rows,
                cells: table.cells.into_iter().map(str::to_owned).collect(),
            });
        }
    }
    out
}

// Generated from Black Ops 3's executable (its table of map entity keys and
// their types). Do not edit by hand.

use game_api::KeyType::{self, Float, Int, String, Vector};

/// Key hash and type, sorted by hash.
pub(crate) static KEY_TYPES: [(u32, KeyType); 349] = [
    (0x0075dbd7, String),
    (0x00e3f4f9, String),
    (0x01430971, Int),   // script_index
    (0x015dfd6c, Float), // script_accuracy
    (0x01c0f890, Int),
    (0x0310c71c, String),
    (0x032aa11e, String),
    (0x03f50133, Int),
    (0x042c70f2, Int),
    (0x043fdcb2, String),
    (0x05886c75, String), // script_gametype_sd
    (0x05f11911, Int),
    (0x063be9b6, String),
    (0x064f4c3f, Int),
    (0x066715b4, String), // script_notify
    (0x06a48009, Int),
    (0x06df9264, String), // model
    (0x070b143d, String),
    (0x07d52d58, Float),
    (0x08c058b5, Int),
    (0x09194748, String), // script_linkto
    (0x097fed22, Int),    // script_ammo_clip
    (0x09954af1, Int),    // script_nodropsecondaryweapon
    (0x09cc44d9, String), // script_gametype_ctf
    (0x0a43ea74, Int),
    (0x0c31055d, Int),
    (0x0d47f13c, String), // script_sound
    (0x0d5ae553, Int),
    (0x103a9781, Int),    // script_forcespawn
    (0x1157a889, String), // scriptbundlename
    (0x132307a0, Float),  // script_dot
    (0x141e3307, String), // script_ender
    (0x14c8a0a0, String),
    (0x14fe6f41, String),
    (0x154f3c77, Float),
    (0x160bf4ad, Int),
    (0x16782c0f, Int),
    (0x170527fb, String), // targetname
    (0x19c21b2d, String),
    (0x1a4f59fa, Int),
    (0x1a8374c3, Int),
    (0x1b15c72f, Int),
    (0x1ca2b3fc, Int),
    (0x1e3cbdc2, Int),
    (0x1eeaf593, String),
    (0x1f5254df, Int), // script_onlyidle
    (0x200967b5, Int),
    (0x20149b34, Int),
    (0x20602c5f, String),
    (0x206640fa, Int),
    (0x20bb15e3, String),
    (0x2151d6c6, Float),
    (0x2153efe0, Int),
    (0x218d5549, String),
    (0x219c140a, String), // script_startingposition
    (0x22c28736, String),
    (0x2452285e, Int), // script_repeat
    (0x24a7a3c8, Vector),
    (0x256df68b, Int),
    (0x25b25f18, String), // script_exploder_radiant
    (0x2622527e, Int),
    (0x266606dd, String), // script_mapsize_64
    (0x27699cdc, Int),
    (0x276adba1, Float), // script_balconydeathchance
    (0x27988d25, Int),
    (0x29534eb8, Int),
    (0x29b74a30, Int),
    (0x2a589309, Int), // script_stop_exploder
    (0x2b8e9503, Int),
    (0x2b941e09, Int), // script_radius
    (0x2d29332f, String),
    (0x2e552217, Int),
    (0x2e596046, Float),
    (0x31afeda1, Int),    // script_fallback
    (0x32d094e8, String), // script_hint
    (0x32f757dd, Int),
    (0x3367c99d, Int),
    (0x373667a2, String), // script_firefxsound
    (0x374c928a, Float),  // script_firefxdelay
    (0x37f7858a, String), // linkto
    (0x381566ba, Int),
    (0x385b9f0a, Int),    // script_nodropsidearm
    (0x38e27f26, String), // script_linkname
    (0x39a11fd8, String), // script_flag_set_on_touching
    (0x3a59872c, Float),
    (0x3bb5d0b3, Float),
    (0x3c5fdb8b, Int),
    (0x3c82c8a2, Int), // script_ammo_extra
    (0x3d2183f2, String),
    (0x3e1ce913, Float), // script_chance
    (0x3eaeabf0, Int),
    (0x3f24db40, String), // script_earthquake
    (0x3fffd1c6, Int),    // script_airspeed
    (0x4125b0c2, Int),
    (0x418b54d5, String),
    (0x41d8b98d, String), // script_gameobjectname
    (0x41fd77e2, Int),
    (0x4297d58b, Int),
    (0x44294e7d, Int),
    (0x44ef2d67, Int),
    (0x47c44e16, String),
    (0x48a9494c, String),
    (0x49e9bfc4, Float),
    (0x4a3ececb, String),
    (0x4be20d44, String), // name
    (0x4d0db49d, String), // script_firefx
    (0x4d210f7a, Int),
    (0x4dd7b6e6, Int),
    (0x4e7785d7, Float),
    (0x4e9de3a6, Float),
    (0x4f6f06ab, String),
    (0x50e1f8f8, Int),
    (0x5132a283, Int),
    (0x532dbe62, Int),
    (0x535014a9, Int), // script_accel
    (0x53e817b8, Int),
    (0x5404b486, Int),
    (0x56321b67, String), // script_vehiclespawngroup
    (0x571f3418, String),
    (0x587133ed, Int),
    (0x59c48d5c, Int),
    (0x5b3dca5e, Int),
    (0x5b917145, Int),
    (0x5c4445e4, Int),
    (0x5d148eed, Int),
    (0x5d88b7aa, Int),
    (0x5dd011cb, Int),
    (0x5f8b9cbf, Int), // spawnflags
    (0x5fa59123, Int),
    (0x5fa98d68, String), // script_unitrigger_type
    (0x5fbb5fc7, Int),
    (0x60796ed9, String),
    (0x60e37d49, Int),
    (0x61f4876b, String), // script_flag_wait
    (0x62978788, String), // script_flag_set
    (0x636859fe, Float),
    (0x63f63e28, Int),
    (0x6428f243, Int),
    (0x6604b19f, Float),
    (0x665ba85b, Int),
    (0x66a618ee, Int),
    (0x67520c6a, Float),  // delay
    (0x68d729f4, Int),    // script_moveoverride
    (0x6ab6f4fd, Vector), // angles
    (0x6b1b9b82, Int),
    (0x6c003e31, String), // script_forcecolor
    (0x6df368db, String), // zombie_weapon_upgrade
    (0x6f6c14df, Int),
    (0x6fba26cb, String),
    (0x70182c90, String), // script_startstate
    (0x71ce47a1, Int),
    (0x722885f3, Vector), // origin
    (0x722be5e1, Int),
    (0x7356d647, Int),
    (0x73a95774, Int),
    (0x741cbf33, Int), // script_prefab_exploder
    (0x7423fccb, Float),
    (0x74952a22, Int), // count
    (0x74e9cde6, Int), // script_growl
    (0x7516ae19, Float),
    (0x75ac8b8e, String),
    (0x76ec4fca, String), // script_friendname
    (0x77ddff8b, Int),
    (0x7806fba9, Int),
    (0x792b512a, Int),    // script_trace
    (0x7a00772a, Float),  // script_burst_min
    (0x7a8bcce3, Int),    // script_vehiclegroupdelete
    (0x7ac1e270, String), // script_crashtype
    (0x7b6755f4, String), // destructibledef
    (0x7c1262c3, String),
    (0x7cc697d0, Int),
    (0x7d402220, Float), // script_wait
    (0x7e9a337e, Int),   // script_auto_use
    (0x7f3abf67, Int),
    (0x7fc707c5, Int),
    (0x7febc1b9, String),
    (0x801bfae0, String), // script_crashtypeoverride
    (0x807e0293, Int),
    (0x813f37e8, Int),
    (0x81ec425a, String),
    (0x81f95669, String), // script_animname
    (0x8202763a, Int),    // script_int
    (0x8208f918, String),
    (0x8284273c, String), // script_vehiclestartmove
    (0x8347bf17, Int),
    (0x83f92d46, Int),
    (0x843a4dd6, String),
    (0x84526fd4, Int),
    (0x848c1554, Int),
    (0x84c1412d, Int),
    (0x84edac03, Int),    // dontdropweapon
    (0x85057f6d, Float),  // script_wait_min
    (0x850b2330, String), // script_soundalias
    (0x85279041, String), // script_stop_exploder_radiant
    (0x8532e4f8, Int),
    (0x863daf04, String),
    (0x86fd6d2b, Int),
    (0x87e4acba, Int),
    (0x892e1b48, Float),
    (0x8942a465, Float),
    (0x894b217e, Int),
    (0x8ccee908, Int),
    (0x8df71eef, String),
    (0x8e3c1a4e, Int),
    (0x8e4e9edb, Float), // script_delay_max
    (0x8ee98814, Int),   // script_exploder
    (0x8ef763b8, Int),
    (0x8f1f1ec4, Float), // script_burst_max
    (0x8f471bf9, Int),
    (0x8f6be231, Int),
    (0x9167c22e, String),
    (0x9181fc65, Int),
    (0x9187e0d4, String),
    (0x92dff85a, String), // destructible_type
    (0x958f1367, String),
    (0x97952882, Int),
    (0x97a31b9e, Float),
    (0x98ff68f8, Float),
    (0x99a48697, Int),
    (0x99d7d770, Int),
    (0x9a1101a9, String),
    (0x9a6e8567, Int),
    (0x9c7d5929, String), // script_gametype_dm
    (0x9d34dca7, String), // vehicletype
    (0x9d6203a5, Int),    // script_trigger_group
    (0x9ee19502, Int),
    (0x9ee65140, Int),
    (0xa010fca7, String),
    (0xa015ce94, Int),
    (0xa1562d9c, String),
    (0xa33b36db, String), // script_parameters
    (0xa36d4675, Float),  // script_delay_min
    (0xa3b61124, String), // script_team
    (0xa3d02500, Int),    // script_damage
    (0xa56a2c0f, Int),
    (0xa5c9822e, Int),
    (0xa6967b66, Int),
    (0xa8675c26, Float), // script_height
    (0xa9249115, Int),
    (0xaa2b0bf8, Int),
    (0xab928682, String),
    (0xaccbddd0, Int),
    (0xace82c7e, Int),
    (0xad02ef27, String),
    (0xae51b794, Int),
    (0xae57e607, Int),
    (0xae6f65e0, Int),
    (0xaf5cac92, String), // script_location
    (0xafecb75f, Int),
    (0xb07228b6, String), // target
    (0xb2ff4d87, String), // script_label
    (0xb3450b4b, Float),
    (0xb34a9879, Float),
    (0xb3d35556, Int),
    (0xb3f494a6, Float),
    (0xb609b0b9, Int),
    (0xb6fa4bcd, String),
    (0xb8c6dec1, Vector),
    (0xb943c6f7, Int),
    (0xb9ca24be, Int),
    (0xb9d825b0, Int),
    (0xba4daae8, Int),
    (0xbad30b66, Vector), // script_vector
    (0xbcdbf157, String), // script_mapsize_08
    (0xbcf80582, String), // script_skilloverride
    (0xbf5a8f68, String), // script_mapsize_32
    (0xbfc66181, Int),
    (0xbfef2183, Float), // script_wait_max
    (0xc0650ec0, String),
    (0xc09c1326, Float),  // script_firefxtimeout
    (0xc0ee8e92, String), // script_mapsize_16
    (0xc129ea4d, Int),
    (0xc1535232, Int),
    (0xc18f4702, String), // door_hold_trigger
    (0xc1d40316, Int),
    (0xc29caf6b, Int),
    (0xc2d7a491, String), // script_flag_false
    (0xc78c1f22, Int),
    (0xc82cd422, String),
    (0xc856abde, Int),
    (0xc89295e3, Float),
    (0xc8c5d62f, String), // script_presound
    (0xc913c757, Vector),
    (0xcaae374e, String), // script_noteworthy
    (0xcbcbfc97, Int),
    (0xcc3505f3, Float),
    (0xcdaa9751, Int),
    (0xcfab185b, Int), // require_look_from
    (0xcfb78f1e, Int),
    (0xd007da9e, Int),    // speed
    (0xd0821c81, String), // weaponinfo
    (0xd2013ecd, String), // script_gametype_tdm
    (0xd48c6ab5, String), // script_flag
    (0xd588bf62, Float),  // script_transition_time
    (0xd66f2978, Float),
    (0xd981bb2d, Int),
    (0xd99cccd0, Int),
    (0xda0607a0, String),
    (0xda9d6b0d, Float), // script_width
    (0xdae5ebe2, Int),
    (0xdb7bb468, String), // script_string
    (0xdb7de487, Int),
    (0xdd6edb31, Int),
    (0xdd8ad30c, String),
    (0xdfd2cefb, Int),
    (0xe01dffee, Int),
    (0xe0e78654, Float),
    (0xe126cca7, Int), // script_disconnectpaths
    (0xe148a662, Int),
    (0xe16a6ec5, Int),
    (0xe290d32d, Int),
    (0xe363b850, Float), // height
    (0xe4ad1eef, Int),
    (0xe4b0837d, Int),
    (0xe4daa8a7, String), // script_vehicleride
    (0xe5034c07, Int),
    (0xe70ce58b, String),
    (0xe7bd1e21, Int),
    (0xe7f51b27, Int),
    (0xe84e18a9, String),
    (0xe8bcb276, Int),    // script_forcegrenade
    (0xe90a6b23, Vector), // script_angles
    (0xe92f61de, Int),
    (0xea35d00d, Int), // zombie_cost
    (0xea8b0b3a, Int),
    (0xeb328a48, Float), // script_delay
    (0xec676387, Int),
    (0xecc91fa9, Int),
    (0xed89847f, String),
    (0xef42adb1, String), // script_flag_set_on_cleared
    (0xefec70a1, Int),
    (0xf1a012cd, String),
    (0xf1c6675d, String), // script_deathflag
    (0xf26f10b3, Int),
    (0xf3026080, String),
    (0xf748f939, Int),
    (0xf8456b37, Int),    // radius
    (0xf8fd6930, String), // script_flag_true
    (0xfa67f159, String), // script_killspawner
    (0xfbc41b7d, Int),
    (0xfc5ff01a, String), // script_fxid
    (0xfcca5c8d, Vector),
    (0xfcd34915, Int),
    (0xfdb12b1c, Int),
    (0xfebe33bf, String), // script_flag_clear
    (0xfed10831, String), // script_gametype_hq
    (0xff1f6868, String),
    (0xff3739ca, String),
    (0xff844e3f, String),
];

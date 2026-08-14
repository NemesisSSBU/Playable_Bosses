use once_cell::sync::Lazy;
use prc::{hash40::Hash40, ParamKind, ParamList, ParamStruct};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;

const FIELD_UI_AMIIBO_ID: Hash40 = Hash40(0xc8c2_eed80);
const FIELD_UI_CHARA_ID: Hash40 = Hash40(0xbd8f_6a88e);
const FIELD_IS_VALID: Hash40 = Hash40(0x8c0f_bd68f);
const FIELD_UNKNOWN_BOOL: Hash40 = Hash40(0x13a2_6bd6a0);
// Keep these labels aligned with the serialized field names. The values are
// easy to misread because the PRC field order is not the same as the logical
// name order used by the original reverse-engineering notes.
const FIELD_NFP_CHARACTER_ID_UPPER: Hash40 = Hash40(0x16d9_89b32f);
const FIELD_NFP_CHARACTER_ID_LOWER: Hash40 = Hash40(0x16b9_4c1790);
const FIELD_NFP_NUMBERING_ID: Hash40 = Hash40(0x109c_47bcd7);
const FIELD_DEFAULT_COLOR: Hash40 = Hash40(0x0d8f_701ce7);
const FIELD_ENABLE_UNKNOWN_NUMBERING_ID: Hash40 = Hash40(0x1bc5_7e4ce5);

// Nintendo's NFP layout stores the character identifier in the first two head
// bytes, the character variant in the third, and the model/numbering value in
// the first two tail bytes. ui_amiibo_db uses the latter three values to choose
// a character row; ui_amiibo_id is a UI identifier, not a replacement for that
// NFP match key.
const PRIVATE_VIRTUAL_BOSS_HEAD: u32 = 0x5042_0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NfpMatchKey {
    pub character_id_upper: u16,
    pub character_id_lower: u8,
    pub numbering_id: u16,
    pub enable_unknown_numbering_id: bool,
}

fn duplicate_nfp_match_key(
    left: Option<NfpMatchKey>,
    right: Option<NfpMatchKey>,
) -> Option<NfpMatchKey> {
    match (left, right) {
        (Some(left), Some(right)) if left == right => Some(left),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum AmiiboFieldType {
    Bool,
    U8,
    U16,
    Hash40,
}

const VERIFIED_AMIIBO_FIELDS: [(Hash40, AmiiboFieldType); 9] = [
    (FIELD_UI_AMIIBO_ID, AmiiboFieldType::Hash40),
    (FIELD_UI_CHARA_ID, AmiiboFieldType::Hash40),
    (FIELD_IS_VALID, AmiiboFieldType::Bool),
    (FIELD_UNKNOWN_BOOL, AmiiboFieldType::Bool),
    (FIELD_NFP_NUMBERING_ID, AmiiboFieldType::U16),
    (FIELD_DEFAULT_COLOR, AmiiboFieldType::U8),
    (FIELD_ENABLE_UNKNOWN_NUMBERING_ID, AmiiboFieldType::Bool),
    (FIELD_NFP_CHARACTER_ID_UPPER, AmiiboFieldType::U16),
    (FIELD_NFP_CHARACTER_ID_LOWER, AmiiboFieldType::U8),
];

fn field(record: &ParamStruct, hash: Hash40) -> Option<&ParamKind> {
    record
        .0
        .iter()
        .find(|(field_hash, _)| *field_hash == hash)
        .map(|(_, value)| value)
}

fn field_has_type(value: &ParamKind, field_type: AmiiboFieldType) -> bool {
    match field_type {
        AmiiboFieldType::Bool => matches!(value, ParamKind::Bool(_)),
        AmiiboFieldType::U8 => matches!(value, ParamKind::U8(_)),
        AmiiboFieldType::U16 => matches!(value, ParamKind::U16(_)),
        AmiiboFieldType::Hash40 => matches!(value, ParamKind::Hash(_)),
    }
}

pub(crate) fn is_verified_amiibo_record(record: &ParamStruct) -> bool {
    record.0.len() == VERIFIED_AMIIBO_FIELDS.len()
        && VERIFIED_AMIIBO_FIELDS.iter().all(|(hash, field_type)| {
            field(record, *hash)
                .map(|value| field_has_type(value, *field_type))
                .unwrap_or(false)
        })
}

pub(crate) fn select_amiibo_template(records: &ParamList) -> Option<(usize, ParamStruct)> {
    let mut fallback = None;
    let mut valid_fallback = None;
    for (index, param) in records.0.iter().enumerate() {
        let Ok(record) = param.try_into_ref::<ParamStruct>() else {
            continue;
        };
        if !is_verified_amiibo_record(record) {
            continue;
        }

        if fallback.is_none() {
            fallback = Some((index, record.clone()));
        }

        let is_valid = field(record, FIELD_IS_VALID)
            .and_then(|value| value.try_into_ref::<bool>().ok())
            .copied()
            .unwrap_or(false);
        let uses_unknown_numbering = field(record, FIELD_ENABLE_UNKNOWN_NUMBERING_ID)
            .and_then(|value| value.try_into_ref::<bool>().ok())
            .copied()
            .unwrap_or(false);
        let numbering_id = field(record, FIELD_NFP_NUMBERING_ID)
            .and_then(|value| value.try_into_ref::<u16>().ok())
            .copied();
        let character_id_lower = field(record, FIELD_NFP_CHARACTER_ID_LOWER)
            .and_then(|value| value.try_into_ref::<u8>().ok())
            .copied();
        let character_id_upper = field(record, FIELD_NFP_CHARACTER_ID_UPPER)
            .and_then(|value| value.try_into_ref::<u16>().ok())
            .copied();
        if is_valid
            && uses_unknown_numbering
            && numbering_id == Some(0)
            && character_id_lower == Some(0)
            && character_id_upper == Some(0)
        {
            return Some((index, record.clone()));
        }
        if is_valid && valid_fallback.is_none() {
            valid_fallback = Some((index, record.clone()));
        }
    }
    valid_fallback.or(fallback)
}

pub(crate) fn amiibo_schema_record_count(records: &ParamList) -> usize {
    records
        .0
        .iter()
        .filter_map(|param| param.try_into_ref::<ParamStruct>().ok())
        .filter(|record| is_verified_amiibo_record(record))
        .count()
}

pub(crate) fn amiibo_structural_fingerprint(records: &ParamList) -> u64 {
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |value: u64| {
        fingerprint ^= value;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    };

    mix(records.0.len() as u64);
    for param in &records.0 {
        let Ok(record) = param.try_into_ref::<ParamStruct>() else {
            mix(0xff);
            continue;
        };
        mix(record.0.len() as u64);
        for (hash, value) in &record.0 {
            mix(hash.0);
            mix(match value {
                ParamKind::Bool(_) => 1,
                ParamKind::U8(_) => 2,
                ParamKind::U16(_) => 3,
                ParamKind::Hash(_) => 4,
                ParamKind::I8(_) => 5,
                ParamKind::I16(_) => 6,
                ParamKind::I32(_) => 7,
                ParamKind::U32(_) => 8,
                ParamKind::Float(_) => 9,
                ParamKind::Str(_) => 10,
                ParamKind::List(_) => 11,
                ParamKind::Struct(_) => 12,
            });
        }
    }
    fingerprint
}

fn patch_hash40_field(record: &mut ParamStruct, hash: Hash40, value: Hash40) -> bool {
    for (field_hash, field_value) in &mut record.0 {
        if *field_hash != hash {
            continue;
        }
        let Ok(field_value) = field_value.try_into_mut::<Hash40>() else {
            return false;
        };
        *field_value = value;
        return true;
    }
    false
}

fn patch_bool_field(record: &mut ParamStruct, hash: Hash40, value: bool) -> bool {
    for (field_hash, field_value) in &mut record.0 {
        if *field_hash != hash {
            continue;
        }
        let Ok(field_value) = field_value.try_into_mut::<bool>() else {
            return false;
        };
        *field_value = value;
        return true;
    }
    false
}

fn patch_u16_field(record: &mut ParamStruct, hash: Hash40, value: u16) -> bool {
    for (field_hash, field_value) in &mut record.0 {
        if *field_hash != hash {
            continue;
        }
        let Ok(field_value) = field_value.try_into_mut::<u16>() else {
            return false;
        };
        *field_value = value;
        return true;
    }
    false
}

fn patch_u8_field(record: &mut ParamStruct, hash: Hash40, value: u8) -> bool {
    for (field_hash, field_value) in &mut record.0 {
        if *field_hash != hash {
            continue;
        }
        let Ok(field_value) = field_value.try_into_mut::<u8>() else {
            return false;
        };
        *field_value = value;
        return true;
    }
    false
}

/// Build one appended database row from a schema-validated row.
///
/// Physical donor/remap mappings preserve the template's NFP subfields. The
/// private virtual catalog has a verified NFP match key, so its append rows
/// replace the character variant and model-number fields explicitly.
pub(crate) fn prepare_append_record(
    template: &ParamStruct,
    ui_amiibo_id: u64,
    ui_chara_id: Hash40,
    nfp_character_id_upper: u16,
    default_color: u8,
    nfp_match_key: Option<NfpMatchKey>,
) -> Option<ParamStruct> {
    if !is_verified_amiibo_record(template) {
        return None;
    }

    let mut record = template.clone();
    let mut patched = patch_hash40_field(&mut record, FIELD_UI_AMIIBO_ID, Hash40(ui_amiibo_id))
        && patch_hash40_field(&mut record, FIELD_UI_CHARA_ID, ui_chara_id)
        && patch_bool_field(&mut record, FIELD_IS_VALID, true)
        && patch_u16_field(
            &mut record,
            FIELD_NFP_CHARACTER_ID_UPPER,
            nfp_character_id_upper,
        )
        && patch_u8_field(&mut record, FIELD_DEFAULT_COLOR, default_color);

    if let Some(key) = nfp_match_key {
        patched = patched
            && key.character_id_upper == nfp_character_id_upper
            && patch_u8_field(
                &mut record,
                FIELD_NFP_CHARACTER_ID_LOWER,
                key.character_id_lower,
            )
            && patch_u16_field(&mut record, FIELD_NFP_NUMBERING_ID, key.numbering_id)
            && patch_bool_field(
                &mut record,
                FIELD_ENABLE_UNKNOWN_NUMBERING_ID,
                key.enable_unknown_numbering_id,
            );
    }

    patched.then_some(record)
}

const DEFAULT_MAPPING_PATH: &str = "sd:/ultimate/mods/Bosses/amiibo.toml";

#[derive(Debug, Clone, Copy)]
pub struct BossAmiiboIdentity {
    pub key: &'static str,
    pub name: &'static str,
    pub ui_chara_id: &'static str,
    pub selector_id: u16,
    pub default_color: u8,
    pub backing_fighter: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfiguredBossAmiibo {
    pub identity: BossAmiiboIdentity,
    pub tag_id: u64,
    pub ui_amiibo_id: u64,
    pub nfp_character_id_upper: u16,
    pub nfp_match_key: Option<NfpMatchKey>,
    pub default_color: u8,
    pub remap_existing: bool,
}

#[derive(Debug, Deserialize, Default)]
struct AmiiboFile {
    #[serde(default)]
    bosses: BTreeMap<String, AmiiboEntry>,
}

#[derive(Debug, Deserialize, Default)]
struct AmiiboEntry {
    amiibo_id: Option<String>,
    default_color: Option<u8>,
    #[serde(default)]
    remap_existing: bool,
}

#[derive(Debug, Default)]
struct LoadedAmiiboFile {
    path: Option<String>,
    file: AmiiboFile,
    parse_error: Option<String>,
}

pub const BOSS_IDENTITIES: [BossAmiiboIdentity; 11] = [
    BossAmiiboIdentity {
        key: "master_hand",
        name: "Master Hand",
        ui_chara_id: "ui_chara_masterhand",
        selector_id: 0x160,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_MASTERHAND",
    },
    BossAmiiboIdentity {
        key: "crazy_hand",
        name: "Crazy Hand",
        ui_chara_id: "ui_chara_crazyhand",
        selector_id: 0x169,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_CRAZYHAND",
    },
    BossAmiiboIdentity {
        key: "wol_master_hand",
        name: "WOL Master Hand",
        ui_chara_id: "ui_chara_mewtwo_masterhand",
        selector_id: 0x1A6,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_PLAYABLE_MASTERHAND",
    },
    BossAmiiboIdentity {
        key: "galeem",
        name: "Galeem",
        ui_chara_id: "ui_chara_kiila",
        selector_id: 0x18F,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_KIILA",
    },
    BossAmiiboIdentity {
        key: "dharkon",
        name: "Dharkon",
        ui_chara_id: "ui_chara_darz",
        selector_id: 0x19A,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_DARZ",
    },
    BossAmiiboIdentity {
        key: "dracula",
        name: "Dracula",
        ui_chara_id: "ui_chara_dracula",
        selector_id: 0x175,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_DRACULA",
    },
    BossAmiiboIdentity {
        key: "ganon_boss",
        name: "Ganon / The Demon King",
        ui_chara_id: "ui_chara_ganonboss",
        selector_id: 0x172,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_GANONBOSS",
    },
    BossAmiiboIdentity {
        key: "galleom",
        name: "Galleom",
        ui_chara_id: "ui_chara_galleom",
        selector_id: 0x16F,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_GALLEOM",
    },
    BossAmiiboIdentity {
        key: "rathalos",
        name: "Rathalos",
        ui_chara_id: "ui_chara_lioleus",
        selector_id: 0x188,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_LIOLEUSBOSS",
    },
    BossAmiiboIdentity {
        key: "marx",
        name: "Marx",
        ui_chara_id: "ui_chara_marx",
        selector_id: 0x180,
        default_color: 0,
        backing_fighter: "fighter_kind_mario + ITEM_KIND_MARX",
    },
    BossAmiiboIdentity {
        key: "giga_bowser",
        name: "Giga Bowser",
        ui_chara_id: "ui_chara_koopag",
        selector_id: 0x18E,
        default_color: 0,
        backing_fighter: "fighter_kind_koopag",
    },
];

fn mapping_paths() -> Vec<String> {
    vec![
        DEFAULT_MAPPING_PATH.to_string(),
        "sd:/ultimate/comp_boss/amiibo.toml".to_string(),
        "sd:/config/comp_boss/amiibo.toml".to_string(),
        "sd:/comp_boss/amiibo.toml".to_string(),
    ]
}

fn load_file() -> LoadedAmiiboFile {
    let Some(path) = mapping_paths()
        .into_iter()
        .find(|candidate| fs::metadata(candidate).is_ok())
    else {
        return LoadedAmiiboFile::default();
    };

    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            return LoadedAmiiboFile {
                path: Some(path),
                file: AmiiboFile::default(),
                parse_error: Some(error.to_string()),
            };
        }
    };

    match toml::from_str::<AmiiboFile>(&contents) {
        Ok(file) => LoadedAmiiboFile {
            path: Some(path),
            file,
            parse_error: None,
        },
        Err(error) => LoadedAmiiboFile {
            path: Some(path),
            file: AmiiboFile::default(),
            parse_error: Some(error.to_string()),
        },
    }
}

static SETTINGS: Lazy<LoadedAmiiboFile> = Lazy::new(load_file);

fn parse_tag_id(value: &str) -> Result<(u64, u64, u16), String> {
    let trimmed = value.trim();
    let digits = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if digits.len() != 16 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("expected exactly 16 hexadecimal digits for a tag ID".to_string());
    }

    let tag_id = u64::from_str_radix(digits, 16)
        .map_err(|_| "tag ID is outside the 64-bit range".to_string())?;
    // ui_amiibo_id is serialized as the tag's low 40 bits. The character
    // variant byte at bits 40..47 has its own NFP field; the supported config
    // format keeps it zero so physical/remap behavior remains unchanged.
    if tag_id & 0x0000_FF00_0000_0000 != 0 {
        return Err("tag bits 40..47 must be zero for ui_amiibo_db.prc".to_string());
    }

    let upper = (tag_id >> 48) as u16;
    let lower = tag_id & 0x0000_00FF_FFFF_FFFF;
    if lower == 0 {
        return Err("tag ID must contain a non-zero lower database portion".to_string());
    }

    Ok((tag_id, lower, upper))
}

#[inline(always)]
fn is_private_virtual_boss_id(tag_id: u64) -> bool {
    (tag_id >> 32) as u32 == PRIVATE_VIRTUAL_BOSS_HEAD
}

#[inline(always)]
fn nfp_character_id_lower_from_tag_id(tag_id: u64) -> u8 {
    ((tag_id >> 40) & 0xff) as u8
}

#[inline(always)]
fn nfp_numbering_id_from_tag_id(tag_id: u64) -> u16 {
    ((tag_id >> 16) & 0xffff) as u16
}

#[inline(always)]
fn private_virtual_nfp_match_key(tag_id: u64) -> Option<NfpMatchKey> {
    if !is_private_virtual_boss_id(tag_id) {
        return None;
    }
    let numbering_id = nfp_numbering_id_from_tag_id(tag_id);
    (numbering_id != 0).then_some(NfpMatchKey {
        character_id_upper: (tag_id >> 48) as u16,
        character_id_lower: nfp_character_id_lower_from_tag_id(tag_id),
        numbering_id,
        enable_unknown_numbering_id: false,
    })
}

#[inline(always)]
fn private_virtual_layout_error(tag_id: u64) -> Option<&'static str> {
    if is_private_virtual_boss_id(tag_id) && nfp_numbering_id_from_tag_id(tag_id) == 0 {
        return Some(
            "private virtual Boss Amiibo IDs require a non-zero model-number field (bits 16..31); the legacy 0x504200010000000N layout makes every boss share the same native NFP match key",
        );
    }
    None
}

fn parsed_configured_mappings() -> Vec<ConfiguredBossAmiibo> {
    let mut mappings = Vec::new();

    for identity in BOSS_IDENTITIES {
        let Some(entry) = SETTINGS.file.bosses.get(identity.key) else {
            continue;
        };
        let Some(raw_id) = entry.amiibo_id.as_deref() else {
            continue;
        };
        if raw_id.trim().is_empty() {
            continue;
        }

        let Ok((tag_id, ui_amiibo_id, nfp_character_id_upper)) = parse_tag_id(raw_id) else {
            continue;
        };
        if private_virtual_layout_error(tag_id).is_some() {
            continue;
        }
        mappings.push(ConfiguredBossAmiibo {
            identity,
            tag_id,
            ui_amiibo_id,
            nfp_character_id_upper,
            nfp_match_key: private_virtual_nfp_match_key(tag_id),
            default_color: entry.default_color.unwrap_or(identity.default_color),
            remap_existing: entry.remap_existing,
        });
    }

    mappings
}

pub fn configured_mappings() -> Vec<ConfiguredBossAmiibo> {
    let mappings = parsed_configured_mappings();
    mappings
        .iter()
        .enumerate()
        .filter_map(|(index, mapping)| {
            let duplicate = mappings.iter().enumerate().any(|(other_index, other)| {
                index != other_index
                    && (mapping.tag_id == other.tag_id
                        || mapping.ui_amiibo_id == other.ui_amiibo_id
                        || (!mapping.remap_existing
                            && !other.remap_existing
                            && duplicate_nfp_match_key(mapping.nfp_match_key, other.nfp_match_key)
                                .is_some()))
            });
            (!duplicate).then_some(*mapping)
        })
        .collect()
}

pub fn source_path() -> Option<&'static str> {
    SETTINGS.path.as_deref()
}

pub fn parse_error() -> Option<&'static str> {
    SETTINGS.parse_error.as_deref()
}

pub fn validation_errors() -> Vec<String> {
    let mut errors = Vec::new();

    for key in SETTINGS.file.bosses.keys() {
        if !BOSS_IDENTITIES.iter().any(|identity| identity.key == key) {
            errors.push(format!("unknown boss mapping key `{}`", key));
        }
    }

    for identity in BOSS_IDENTITIES {
        let Some(entry) = SETTINGS.file.bosses.get(identity.key) else {
            continue;
        };
        let Some(raw_id) = entry.amiibo_id.as_deref() else {
            continue;
        };
        if raw_id.trim().is_empty() {
            continue;
        }
        match parse_tag_id(raw_id) {
            Err(error) => errors.push(format!("{} ({}): {}", identity.name, identity.key, error)),
            Ok((tag_id, _, _)) => {
                if let Some(error) = private_virtual_layout_error(tag_id) {
                    errors.push(format!("{} ({}): {}", identity.name, identity.key, error));
                }
            }
        }
    }

    let mappings = parsed_configured_mappings();
    for (index, mapping) in mappings.iter().enumerate() {
        for other in mappings.iter().skip(index + 1) {
            if mapping.ui_amiibo_id == other.ui_amiibo_id {
                errors.push(format!(
                    "{} and {} reuse ui_amiibo_id 0x{:010x}; each boss needs a unique donor ID",
                    mapping.identity.name, other.identity.name, mapping.ui_amiibo_id
                ));
            }
            if mapping.tag_id == other.tag_id {
                errors.push(format!(
                    "{} and {} reuse full amiibo ID 0x{:016x}; each boss needs a unique donor ID",
                    mapping.identity.name, other.identity.name, mapping.tag_id
                ));
            }
            if !mapping.remap_existing && !other.remap_existing {
                let Some(key) = duplicate_nfp_match_key(mapping.nfp_match_key, other.nfp_match_key)
                else {
                    continue;
                };
                errors.push(format!(
                    "{} and {} reuse native NFP key upper=0x{:04x} lower=0x{:02x} numbering=0x{:04x}; private virtual IDs must use unique model numbers",
                    mapping.identity.name,
                    other.identity.name,
                    key.character_id_upper,
                    key.character_id_lower,
                    key.numbering_id
                ));
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_current_figure_id_layout() {
        assert_eq!(
            parse_tag_id("0x2106000003601202"),
            Ok((0x2106000003601202, 0x03601202, 0x2106))
        );
        assert_eq!(
            parse_tag_id("0x3740000103741402"),
            Ok((0x3740000103741402, 0x0103741402, 0x3740))
        );
    }

    #[test]
    fn accepts_zero_upper_ids() {
        assert_eq!(
            parse_tag_id("0x0000000000000002"),
            Ok((0x0000000000000002, 0x0000000002, 0x0000))
        );
    }

    #[test]
    fn accepts_private_virtual_boss_ids_with_a_unique_model_number() {
        assert_eq!(
            parse_tag_id("0x5042000100010001"),
            Ok((0x5042000100010001, 0x0100010001, 0x5042))
        );
        assert_eq!(
            private_virtual_nfp_match_key(0x5042_0001_0001_0001),
            Some(NfpMatchKey {
                character_id_upper: 0x5042,
                character_id_lower: 0,
                numbering_id: 1,
                enable_unknown_numbering_id: false,
            })
        );
    }

    #[test]
    fn rejects_non_fixed_width_or_nonzero_middle_bits() {
        assert!(parse_tag_id("0x3601202").is_err());
        assert!(parse_tag_id("0x2106010003601202").is_err());
        assert!(parse_tag_id("0x0000000000000000").is_err());
    }

    #[test]
    fn keeps_the_complete_boss_identity_set_centralized() {
        assert_eq!(BOSS_IDENTITIES.len(), 11);
        let mut keys: Vec<_> = BOSS_IDENTITIES
            .iter()
            .map(|identity| identity.key)
            .collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 11);
    }

    #[test]
    fn discovers_a_schema_template_without_a_product_sentinel() {
        let mut reader = std::io::Cursor::new(include_bytes!("../ui_amiibo_db.prc"));
        let root = prc::read_stream(&mut reader).expect("fixture PRC should parse");
        let records = root
            .0
            .iter()
            .find(|(hash, _)| *hash == Hash40::new("db_root"))
            .and_then(|(_, value)| value.try_into_ref::<ParamList>().ok())
            .expect("fixture should contain db_root");

        assert_eq!(amiibo_schema_record_count(records), records.0.len());
        let (index, template) = select_amiibo_template(records).expect("schema template");
        assert_eq!(index, 0);
        assert!(is_verified_amiibo_record(&template));
    }

    #[test]
    fn serialized_field_constants_match_their_labels() {
        assert_eq!(FIELD_UI_AMIIBO_ID, Hash40::new("ui_amiibo_id"));
        assert_eq!(FIELD_UI_CHARA_ID, Hash40::new("ui_chara_id"));
        assert_eq!(FIELD_IS_VALID, Hash40::new("is_valid"));
        assert_eq!(
            FIELD_NFP_CHARACTER_ID_UPPER,
            Hash40::new("nfp_character_id_upper")
        );
        assert_eq!(
            FIELD_NFP_CHARACTER_ID_LOWER,
            Hash40::new("nfp_character_id_lower")
        );
        assert_eq!(FIELD_NFP_NUMBERING_ID, Hash40::new("nfp_numbering_id"));
        assert_eq!(FIELD_DEFAULT_COLOR, Hash40::new("default_color"));
        assert_eq!(
            FIELD_ENABLE_UNKNOWN_NUMBERING_ID,
            Hash40::new("enable_unknown_numbering_id")
        );
    }

    #[test]
    fn schema_rejection_does_not_mutate_the_database_tree() {
        let records = ParamList(vec![ParamKind::Bool(true)]);
        let before = records.clone();
        assert!(select_amiibo_template(&records).is_none());
        assert_eq!(records, before);
    }

    #[test]
    fn schema_rejection_keeps_prc_bytes_identical() {
        let root = ParamStruct(vec![(
            Hash40::new("db_root"),
            ParamKind::List(ParamList(vec![ParamKind::Bool(true)])),
        )]);
        let mut before_writer = std::io::Cursor::new(Vec::new());
        prc::write_stream(&mut before_writer, &root).expect("test PRC should serialize");
        let before = before_writer.into_inner();

        let mut reader = std::io::Cursor::new(&before);
        let parsed = prc::read_stream(&mut reader).expect("test PRC should parse");
        let records = parsed
            .0
            .iter()
            .find(|(hash, _)| *hash == Hash40::new("db_root"))
            .and_then(|(_, value)| value.try_into_ref::<ParamList>().ok())
            .expect("test PRC should contain db_root");
        assert!(select_amiibo_template(records).is_none());

        let mut after_writer = std::io::Cursor::new(Vec::new());
        prc::write_stream(&mut after_writer, &parsed).expect("test PRC should reserialize");
        assert_eq!(before, after_writer.into_inner());
    }

    #[test]
    fn zero_upper_mario_append_is_one_complete_master_hand_record() {
        let mut reader = std::io::Cursor::new(include_bytes!("../ui_amiibo_db.prc"));
        let root = prc::read_stream(&mut reader).expect("fixture PRC should parse");
        let records = root
            .0
            .iter()
            .find(|(hash, _)| *hash == Hash40::new("db_root"))
            .and_then(|(_, value)| value.try_into_ref::<ParamList>().ok())
            .expect("fixture should contain db_root");
        let (_, template) = select_amiibo_template(records).expect("schema template");

        let appended = prepare_append_record(
            &template,
            0x0000_0000_02,
            Hash40::new("ui_chara_masterhand"),
            0,
            0,
            None,
        )
        .expect("zero-upper Mario row should be constructible");

        assert_eq!(
            field(&appended, FIELD_UI_AMIIBO_ID)
                .and_then(|value| value.try_into_ref::<Hash40>().ok())
                .copied(),
            Some(Hash40(2))
        );
        assert_eq!(
            field(&appended, FIELD_UI_CHARA_ID)
                .and_then(|value| value.try_into_ref::<Hash40>().ok())
                .copied(),
            Some(Hash40::new("ui_chara_masterhand"))
        );
        assert_eq!(
            field(&appended, FIELD_IS_VALID)
                .and_then(|value| value.try_into_ref::<bool>().ok())
                .copied(),
            Some(true)
        );
        assert_eq!(
            field(&appended, FIELD_NFP_CHARACTER_ID_UPPER)
                .and_then(|value| value.try_into_ref::<u16>().ok())
                .copied(),
            Some(0)
        );
        assert_eq!(
            field(&appended, FIELD_DEFAULT_COLOR)
                .and_then(|value| value.try_into_ref::<u8>().ok())
                .copied(),
            Some(0)
        );
        assert_eq!(
            field(&appended, FIELD_NFP_CHARACTER_ID_LOWER)
                .and_then(|value| value.try_into_ref::<u8>().ok())
                .copied(),
            Some(0)
        );
        assert_eq!(
            field(&appended, FIELD_NFP_NUMBERING_ID)
                .and_then(|value| value.try_into_ref::<u16>().ok())
                .copied(),
            Some(0)
        );

        let original_count = records.0.len();
        let mut simulated_records = records.clone();
        simulated_records.0.push(ParamKind::Struct(appended));
        assert_eq!(original_count, 124);
        assert_eq!(simulated_records.0.len(), original_count + 1);
        assert_eq!(records.0.len(), original_count);
    }

    #[test]
    fn virtual_master_hand_append_patches_the_complete_native_nfp_key() {
        let mut reader = std::io::Cursor::new(include_bytes!("../ui_amiibo_db.prc"));
        let root = prc::read_stream(&mut reader).expect("fixture PRC should parse");
        let records = root
            .0
            .iter()
            .find(|(hash, _)| *hash == Hash40::new("db_root"))
            .and_then(|(_, value)| value.try_into_ref::<ParamList>().ok())
            .expect("fixture should contain db_root");
        let (_, template) = select_amiibo_template(records).expect("schema template");

        let appended = prepare_append_record(
            &template,
            0x0100_0100_01,
            Hash40::new("ui_chara_masterhand"),
            0x5042,
            3,
            Some(NfpMatchKey {
                character_id_upper: 0x5042,
                character_id_lower: 0,
                numbering_id: 1,
                enable_unknown_numbering_id: false,
            }),
        )
        .expect("virtual Master Hand row should be constructible");

        assert_eq!(
            field(&appended, FIELD_NFP_CHARACTER_ID_UPPER)
                .and_then(|value| value.try_into_ref::<u16>().ok())
                .copied(),
            Some(0x5042)
        );
        assert_eq!(
            field(&appended, FIELD_DEFAULT_COLOR)
                .and_then(|value| value.try_into_ref::<u8>().ok())
                .copied(),
            Some(3)
        );
        assert_eq!(
            field(&appended, FIELD_NFP_NUMBERING_ID)
                .and_then(|value| value.try_into_ref::<u16>().ok())
                .copied(),
            Some(1)
        );
        assert_eq!(
            field(&appended, FIELD_NFP_CHARACTER_ID_LOWER)
                .and_then(|value| value.try_into_ref::<u8>().ok())
                .copied(),
            Some(0)
        );
        assert_eq!(
            field(&appended, FIELD_ENABLE_UNKNOWN_NUMBERING_ID)
                .and_then(|value| value.try_into_ref::<bool>().ok())
                .copied(),
            Some(false)
        );
    }

    #[test]
    fn private_virtual_ids_use_the_model_number_as_the_native_discriminator() {
        let master = 0x5042_0001_0001_0001;
        let crazy = 0x5042_0001_0002_0002;
        assert_eq!(nfp_numbering_id_from_tag_id(master), 1);
        assert_eq!(nfp_numbering_id_from_tag_id(crazy), 2);
        assert_ne!(
            private_virtual_nfp_match_key(master),
            private_virtual_nfp_match_key(crazy)
        );
        assert!(private_virtual_layout_error(0x5042_0001_0000_0001).is_some());
    }

    #[test]
    fn all_eleven_virtual_catalog_ids_have_distinct_explicit_nfp_keys() {
        let mut keys = Vec::new();
        for ordinal in 1u64..=11 {
            let tag_id = 0x5042_0001_0000_0000 | (ordinal << 16) | ordinal;
            let key = private_virtual_nfp_match_key(tag_id).expect("valid private virtual key");
            assert_eq!(key.character_id_upper, 0x5042);
            assert_eq!(key.character_id_lower, 0);
            assert_eq!(key.numbering_id, ordinal as u16);
            assert!(!key.enable_unknown_numbering_id);
            assert!(private_virtual_layout_error(tag_id).is_none());
            assert!(!keys.contains(&key));
            keys.push(key);
        }
    }
}

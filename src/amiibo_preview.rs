//! Presentation data for boss Figure Players.
//!
//! The amiibo management screen is not a battle scene.  In the current public
//! bindings there is no safe hook for its menu model factory, so this module
//! deliberately contains data and diagnostics only.  It must not create boss
//! items, fighter objects, or combat state from a UI callback.

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::BTreeSet;
use std::sync::Once;

use crate::config::CONFIG;

const MAX_NRO_TRACE_ENTRIES: usize = 96;
const MAX_NRO_SYMBOL_TRACE_ENTRIES: usize = 48;
const MAX_IDENTITY_TRACE_ENTRIES: usize = 32;

// The NRO hook is a safe observation boundary for the first Switch trace. It
// tells us which lazily loaded UI module owns the amiibo screen without
// guessing a version-specific function address or touching a menu object.
static NRO_TRACE_INSTALLED: Once = Once::new();
static NRO_TRACE_SEEN: Lazy<Mutex<BTreeSet<String>>> = Lazy::new(|| Mutex::new(BTreeSet::new()));
static NRO_SYMBOL_TRACE_COUNT: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
static IDENTITY_TRACE_SEEN: Lazy<Mutex<BTreeSet<String>>> = Lazy::new(|| Mutex::new(BTreeSet::new()));

fn is_ui_like_nro(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "common"
        || lower.contains("ui")
        || lower.contains("menu")
        || lower.contains("chara")
}

fn is_preview_symbol(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "amiibo", "nfp", "figure", "preview", "viewer", "model", "motion", "camera",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn nro_trace_enabled() -> bool {
    crate::debug::enabled()
        && CONFIG
            .options
            .debug_amiibo_nro_trace
            .unwrap_or(false)
}

fn nro_symbol_trace_enabled() -> bool {
    nro_trace_enabled()
        && CONFIG
            .options
            .debug_amiibo_nro_symbols
            .unwrap_or(false)
}

unsafe fn read_symbol_name(pointer: *const u8, max_len: usize) -> Option<String> {
    if pointer.is_null() {
        return None;
    }

    let mut bytes = Vec::new();
    for index in 0..max_len {
        let byte = *pointer.add(index);
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }
    String::from_utf8(bytes).ok()
}

// This read-only symbol inventory is deliberately limited to likely UI NROs
// and bounded to exported names matching preview concepts. It is useful when
// a module is stripped of documented symbols, but it never hooks a discovered
// address. The final function hook still requires a version-stable boundary.
unsafe fn log_exported_preview_symbols(info: &skyline::nro::NroInfo, module_base: u64) {
    if !is_ui_like_nro(info.name) {
        return;
    }

    let module_object = info.module.ModuleObject;
    if module_object.is_null() {
        return;
    }

    let module_object = &*module_object;
    if module_object.dynsym.is_null()
        || module_object.dynstr.is_null()
        || module_object.hash_nchain_value == 0
        || module_object.hash_nchain_value > 0x10000
        || module_object.dynstr_size == 0
        || module_object.dynstr_size > 0x0100_0000
    {
        return;
    }

    for index in 0..module_object.hash_nchain_value as usize {
        let symbol = *module_object.dynsym.add(index);
        if symbol.st_name as u64 >= module_object.dynstr_size {
            continue;
        }

        let Some(name) = read_symbol_name(module_object.dynstr.add(symbol.st_name as usize), 160)
        else {
            continue;
        };
        if name.is_empty() || !is_preview_symbol(&name) {
            continue;
        }

        let mut logged = NRO_SYMBOL_TRACE_COUNT.lock();
        if *logged >= MAX_NRO_SYMBOL_TRACE_ENTRIES {
            return;
        }
        *logged += 1;
        drop(logged);

        crate::boss_log!(
            "[PB][AmiiboPreview][NRO][symbol] module={} name={} address=0x{:x}",
            info.name,
            name,
            module_base.wrapping_add(symbol.st_value)
        );
    }
}

fn log_nro_event(event: &str, info: &skyline::nro::NroInfo) {
    if !nro_trace_enabled() {
        return;
    }

    let key = format!("{}:{}", event, info.name);
    {
        let mut seen = NRO_TRACE_SEEN.lock();
        if seen.contains(&key) {
            return;
        }
        if seen.len() >= MAX_NRO_TRACE_ENTRIES {
            return;
        }
        seen.insert(key);
    }

    let module_base = unsafe {
        let module_object = info.module.ModuleObject;
        if module_object.is_null() {
            0
        } else {
            (*module_object).module_base
        }
    };

    crate::boss_log!(
        "[PB][AmiiboPreview][NRO] event={} name={} module_base=0x{:x}",
        event,
        info.name,
        module_base
    );

    if event == "load" && nro_symbol_trace_enabled() {
        unsafe { log_exported_preview_symbols(info, module_base) };
    }
}

extern "Rust" fn log_nro_load(info: &skyline::nro::NroInfo) {
    log_nro_event("load", info);
}

extern "Rust" fn log_nro_unload(info: &skyline::nro::NroInfo) {
    log_nro_event("unload", info);
}

/// Install only the version-independent NRO lifecycle trace needed to find
/// the native amiibo viewer module on hardware. This does not hook a viewer
/// function and does not create or mutate any preview object.
pub fn install_nro_trace() {
    if !nro_trace_enabled() {
        return;
    }

    NRO_TRACE_INSTALLED.call_once(|| {
        let load_result = skyline::nro::add_hook(log_nro_load);
        let unload_result = skyline::nro::add_unload_hook(log_nro_unload);
        crate::boss_log!(
            "[PB][AmiiboPreview] nro_trace installed load={} unload={}",
            load_result.is_ok(),
            unload_result.is_ok()
        );
    });
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossAmiiboPreviewProfile {
    pub key: &'static str,
    pub ui_chara_id: &'static str,
    pub preview_source: &'static str,
    pub idle_motion: &'static str,
    pub preview_scale: Option<f32>,
    pub position_profile: &'static str,
    pub camera_profile: &'static str,
    pub menu_model_status: &'static str,
    pub notes: &'static str,
}

// Item/idle/scale values are the existing CSS battle-preview presentation
// inputs. They are references for a future menu-model hook, not menu runtime
// instructions. The menu must not instantiate these combat items directly.
pub const BOSS_AMIIBO_PREVIEW_PROFILES: [BossAmiiboPreviewProfile; 11] = [
    BossAmiiboPreviewProfile {
        key: "master_hand",
        ui_chara_id: "ui_chara_masterhand",
        preview_source: "item:masterhand",
        idle_motion: "wait",
        preview_scale: Some(0.08),
        position_profile: "floating_center",
        camera_profile: "wide_hand",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Use the real Master Hand model without a Mario host.",
    },
    BossAmiiboPreviewProfile {
        key: "crazy_hand",
        ui_chara_id: "ui_chara_crazyhand",
        preview_source: "item:crazyhand",
        idle_motion: "wait",
        preview_scale: Some(0.08),
        position_profile: "floating_center",
        camera_profile: "wide_hand",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Show Crazy Hand alone; do not create a Master Hand partner.",
    },
    BossAmiiboPreviewProfile {
        key: "wol_master_hand",
        ui_chara_id: "ui_chara_mewtwo_masterhand",
        preview_source: "item:playable_masterhand",
        idle_motion: "wait",
        preview_scale: Some(0.08),
        position_profile: "floating_center",
        camera_profile: "wide_hand",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Keep WOL Master Hand distinct from regular Master Hand.",
    },
    BossAmiiboPreviewProfile {
        key: "galeem",
        ui_chara_id: "ui_chara_kiila",
        preview_source: "item:kiilacore",
        idle_motion: "wait",
        preview_scale: Some(0.05),
        position_profile: "large_center",
        camera_profile: "extra_wide",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Presentation only; no Galeem puppets, rage, or attacks.",
    },
    BossAmiiboPreviewProfile {
        key: "dharkon",
        ui_chara_id: "ui_chara_darz",
        preview_source: "item:darzcentipede",
        idle_motion: "wait",
        preview_scale: Some(0.05),
        position_profile: "large_center",
        camera_profile: "extra_wide",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Presentation only; no Dharkon puppets, rage, or attacks.",
    },
    BossAmiiboPreviewProfile {
        key: "dracula",
        ui_chara_id: "ui_chara_dracula",
        preview_source: "item:dracula_phase1",
        idle_motion: "wait",
        preview_scale: Some(0.08),
        position_profile: "tall_center",
        camera_profile: "tall",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Use a stable Phase 1 presentation; do not run the combat phase transition.",
    },
    BossAmiiboPreviewProfile {
        key: "ganon_boss",
        ui_chara_id: "ui_chara_ganonboss",
        preview_source: "item:ganonboss",
        idle_motion: "wait",
        preview_scale: Some(0.065),
        position_profile: "large_center",
        camera_profile: "large_boss",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Keep Boss Ganon separate from normal Ganondorf.",
    },
    BossAmiiboPreviewProfile {
        key: "galleom",
        ui_chara_id: "ui_chara_galleom",
        preview_source: "item:galleom",
        idle_motion: "wait",
        preview_scale: Some(0.04),
        position_profile: "large_center",
        camera_profile: "extra_wide",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Use a stable presentation state; no vehicle or combat transitions.",
    },
    BossAmiiboPreviewProfile {
        key: "rathalos",
        ui_chara_id: "ui_chara_lioleus",
        preview_source: "item:lioleusboss",
        idle_motion: "hovering_move",
        preview_scale: Some(0.04),
        position_profile: "air_center",
        camera_profile: "wide_flying_boss",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Use a deliberate hovering pose; do not start ground/air combat switching.",
    },
    BossAmiiboPreviewProfile {
        key: "marx",
        ui_chara_id: "ui_chara_marx",
        preview_source: "item:marx",
        idle_motion: "wait",
        preview_scale: Some(0.05),
        position_profile: "floating_center",
        camera_profile: "floating_boss",
        menu_model_status: "native_menu_model_hook_required",
        notes: "Keep teleport, Black Hole, and attack states disabled in presentation.",
    },
    BossAmiiboPreviewProfile {
        key: "giga_bowser",
        ui_chara_id: "ui_chara_koopag",
        preview_source: "fighter:koopag",
        idle_motion: "wait",
        preview_scale: None,
        position_profile: "native_large_fighter",
        camera_profile: "large_fighter",
        menu_model_status: "native_fighter_model_path_hardware_verification_required",
        notes: "Prefer the native koopag model path; never alter official Bowser mappings.",
    },
];

pub fn profiles() -> &'static [BossAmiiboPreviewProfile; 11] {
    &BOSS_AMIIBO_PREVIEW_PROFILES
}

pub fn profile_for_ui_chara_id(ui_chara_id: &str) -> Option<&'static BossAmiiboPreviewProfile> {
    profiles()
        .iter()
        .find(|profile| profile.ui_chara_id == ui_chara_id)
}

pub fn log_mapping_profiles(mappings: &[crate::amiibo::ConfiguredBossAmiibo]) {
    crate::boss_log!(
        "[PB][AmiiboPreview] native_menu_model_hook=unresolved configured_mappings={} identity_source=ui_amiibo_db->ui_chara_db",
        mappings.len()
    );

    for mapping in mappings {
        let Some(profile) = profile_for_ui_chara_id(mapping.identity.ui_chara_id) else {
            crate::boss_log!(
                "[PB][AmiiboPreview] mapping_ready boss={} ui_chara_id={} profile=missing status=blocked",
                mapping.identity.name,
                mapping.identity.ui_chara_id
            );
            continue;
        };

        if profile.key != mapping.identity.key {
            crate::boss_log!(
                "[PB][AmiiboPreview] mapping_ready boss={} ui_chara_id={} profile_key_mismatch={} expected={} status=blocked",
                mapping.identity.name,
                profile.ui_chara_id,
                profile.key,
                mapping.identity.key
            );
            continue;
        }

        crate::boss_log!(
            "[PB][AmiiboPreview] mapping_ready boss={} ui_chara_id={} source={} idle_motion={} scale={:?} position={} camera={} status={} custom_preview_required=true",
            mapping.identity.name,
            profile.ui_chara_id,
            profile.preview_source,
            profile.idle_motion,
            profile.preview_scale,
            profile.position_profile,
            profile.camera_profile,
            profile.menu_model_status
        );
    }
}

/// Log the last safe boundary owned by this plugin. The native amiibo menu
/// viewer runs after the ARC database lookup and is not exposed by the pinned
/// bindings, so this records what the plugin supplied without pretending that
/// a model/fighter conversion was observed.
pub fn log_identity_boundary(
    mapping: &crate::amiibo::ConfiguredBossAmiibo,
    mode: &str,
    original_ui_chara_hash: Option<u64>,
) {
    if !crate::debug::enabled() {
        return;
    }

    let key = format!("{}:{:016x}", mode, mapping.tag_id);
    {
        let mut seen = IDENTITY_TRACE_SEEN.lock();
        if seen.contains(&key) || seen.len() >= MAX_IDENTITY_TRACE_ENTRIES {
            return;
        }
        seen.insert(key);
    }

    let profile = profile_for_ui_chara_id(mapping.identity.ui_chara_id);
    crate::boss_log!(
        "[PB][AmiiboPreview] identity_boundary mode={} figure_id=0x{:016x} original_ui_chara_hash={} requested_ui_chara={} requested_ui_chara_hash=0x{:010x} logical_boss={} battle_backing={} profile_source={} idle_motion={} native_fighter_kind=unobserved native_model=unobserved preview_actor=unobserved preview_motion=unobserved preview_camera=unobserved",
        mode,
        mapping.tag_id,
        original_ui_chara_hash
            .map(|hash| format!("0x{:010x}", hash))
            .unwrap_or_else(|| "<none>".to_string()),
        mapping.identity.ui_chara_id,
        crate::to_hash40(mapping.identity.ui_chara_id).0,
        mapping.identity.name,
        mapping.identity.backing_fighter,
        profile.map(|value| value.preview_source).unwrap_or("<missing>"),
        profile.map(|value| value.idle_motion).unwrap_or("<missing>"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amiibo::BOSS_IDENTITIES;

    #[test]
    fn every_boss_identity_has_one_preview_profile() {
        assert_eq!(profiles().len(), BOSS_IDENTITIES.len());
        for identity in BOSS_IDENTITIES {
            let profile = profile_for_ui_chara_id(identity.ui_chara_id)
                .expect("missing boss preview profile");
            assert_eq!(profile.key, identity.key);
            assert_eq!(profile.ui_chara_id, identity.ui_chara_id);
        }
    }

    #[test]
    fn preview_profile_keys_and_ui_ids_are_unique() {
        for (index, profile) in profiles().iter().enumerate() {
            assert!(!profiles()[index + 1..]
                .iter()
                .any(|other| other.key == profile.key || other.ui_chara_id == profile.ui_chara_id));
        }
    }
}

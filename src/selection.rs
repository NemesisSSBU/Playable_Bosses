use crate::config::CONFIG;
use once_cell::sync::Lazy;
use skyline::nn::oe::{DisplayVersion, GetDisplayVersion, Initialize};
use smash::app::lua_bind::*;
use smash::app::sv_battle_object;
use smash::app::sv_information;
use smash::app::{BattleObjectModuleAccessor, FighterInformation};
use smash::lib::lua_const::*;

const MAX_FIGHTERS: usize = 8;
const DETECT_CHARACTER_NAME_ENTRY_STRIDE: u64 = 0x260;
const DETECT_CHARACTER_NAME_TEXT_OFFSET: u64 = 0x8E;
static mut LAST_LOGGED_SELECTOR_ID: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_LOG_SELECTOR_ID: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_NORMALIZED_SELECTOR_ID: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_RESOLVED_SELECTOR_ID: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_SELECTION_LOG_SELECTOR_ID: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_CACHE_SELECTOR_ID: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LOG_INT_SELECTOR_KEY: [Option<(i32, i32, i32)>; MAX_FIGHTERS] = [None; MAX_FIGHTERS];
static mut CACHED_BOSS_UI_HASH_GLOBAL: u64 = 0;
static mut CACHED_BOSS_UI_HASH_BY_ENTRY: [u64; MAX_FIGHTERS] = [0; MAX_FIGHTERS];
static mut CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY: [OpaqueSelectionCacheOrigin; MAX_FIGHTERS] =
    [OpaqueSelectionCacheOrigin::None; MAX_FIGHTERS];
static mut PENDING_OPAQUE_SELECTION_ENTRY: usize = usize::MAX;
static mut LAST_LOGGED_GLOBAL_CAPTURE_HASH: u64 = 0;
static mut LAST_LOGGED_SELECTION_INFO_HASH: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_CSS_SELECTION_RAW: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_CSS_SELECTION_HASH: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_NAME_SELECTOR_HASH: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_NAME_SELECTOR_RESULT: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut SUPPRESS_BOSS_SELECTION_BY_ENTRY: [bool; MAX_FIGHTERS] = [false; MAX_FIGHTERS];
static mut SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY: [i32; MAX_FIGHTERS] = [i32::MIN; MAX_FIGHTERS];
// Begin/end slots for the four installed selection hooks. These diagnostics are
// state-transition based so WOL selection can be bisected without logging every
// menu update.
static mut LAST_SELECTION_BISECT_SIGNATURE: [u64; 8] = [u64::MAX; 8];

// Raw selection callbacks run before the battle-stage subsystem is guaranteed
// to exist.  Keep their identity handoff explicitly opaque; only a validated
// fighter frame may classify a tentative value as a WOL preview selection.
#[derive(Copy, Clone, PartialEq, Eq)]
enum OpaqueSelectionCacheOrigin {
    None,
    TentativeUiSelection,
    ConfirmedUiLookup,
}

impl OpaqueSelectionCacheOrigin {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::TentativeUiSelection => "tentative_ui_selection",
            Self::ConfirmedUiLookup => "confirmed_ui_lookup",
        }
    }
}

static TITLE_VERSION: Lazy<(u16, u16, u16)> = Lazy::new(|| unsafe {
    Initialize();
    let mut display_version = DisplayVersion { name: [0; 16] };
    GetDisplayVersion(&mut display_version);
    let name = std::str::from_utf8(&display_version.name)
        .unwrap_or_default()
        .trim_end_matches(char::from(0))
        .to_string();
    let mut parts = name.split('.').filter_map(|s| s.parse::<u16>().ok());
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let micro = parts.next().unwrap_or(0);
    (major, minor, micro)
});

const UI_CHARA_KOOPAG_SELECTOR: i32 = 0x18E;
const UI_CHARA_MASTERHAND_SELECTOR: i32 = 0x160;
const UI_CHARA_CRAZYHAND_SELECTOR: i32 = 0x169;
const UI_CHARA_DARZ_SELECTOR: i32 = 0x19A;
const UI_CHARA_KIILA_SELECTOR: i32 = 0x18F;
const UI_CHARA_MARX_SELECTOR: i32 = 0x180;
const UI_CHARA_GANONBOSS_SELECTOR: i32 = 0x172;
const UI_CHARA_DRACULA_SELECTOR: i32 = 0x175;
const UI_CHARA_GALLEOM_SELECTOR: i32 = 0x16F;
const UI_CHARA_LIOLEUS_SELECTOR: i32 = 0x188;
const UI_CHARA_MEWTWO_MASTERHAND_SELECTOR: i32 = 0x1A6;

const UI_CHARA_KOOPAG_HASH: u64 = 0x0F93DBBF13;
const UI_CHARA_MASTERHAND_HASH: u64 = 0x1389102CBF;
const UI_CHARA_CRAZYHAND_HASH: u64 = 0x12CEF82D30;
const UI_CHARA_DARZ_HASH: u64 = 0x0D65ACCD76;
const UI_CHARA_KIILA_HASH: u64 = 0x0E1ABB80FF;
const UI_CHARA_MARX_HASH: u64 = 0x0DF6AAE3D0;
const UI_CHARA_GANONBOSS_HASH: u64 = 0x120F2FC612;
const UI_CHARA_DRACULA_HASH: u64 = 0x1020DDD1F9;
const UI_CHARA_GALLEOM_HASH: u64 = 0x100A39D32E;
const UI_CHARA_LIOLEUS_HASH: u64 = 0x10E9EFB8D1;
const UI_CHARA_MEWTWO_MASTERHAND_HASH: u64 = 0x1AA4AF9031;
const UI_CHARA_MARIO_HASH: u64 = 0x0EDAF3C863;
const HASH40_MASK: u64 = 0xFFFF_FFFFFF;

// Known hook points for CSS selection capture across current supported builds.
const SELECTION_UPDATE_SELECTED_FIGHTER_13_0_1: usize = 0x3310760;
const SELECTION_UPDATE_SELECTED_FIGHTER_13_0_2_PLUS: usize = 0x3311190;
const SELECTION_UPDATE_CSS_13_0_1_PLUS: usize = 0x1A12460;

fn detect_character_name_enabled() -> bool {
    CONFIG.options.detect_character_name.unwrap_or(false)
}

fn canonicalize_detected_character_name(name: &str) -> String {
    let mut canonical = String::with_capacity(name.len());
    let mut last_was_space = false;

    for mut ch in name.trim().chars() {
        if matches!(ch, '_' | '-') {
            ch = ' ';
        }

        if ch.is_ascii_whitespace() {
            if !canonical.is_empty() && !last_was_space {
                canonical.push(' ');
                last_was_space = true;
            }
            continue;
        }

        if ch.is_ascii() {
            canonical.push(ch.to_ascii_uppercase());
        } else {
            canonical.push(ch);
        }
        last_was_space = false;
    }

    canonical
}

fn detected_character_name_to_ui_hash(name: &str) -> Option<u64> {
    match canonicalize_detected_character_name(name).as_str() {
        "GIGA BOWSER" | "GIGABOWSER" | "KOOPAG" => Some(UI_CHARA_KOOPAG_HASH),
        "MASTER HAND"
        | "MASTERHAND"
        | "マスターハンド"
        | "CRÉA MAIN"
        | "CRÉA-MAIN"
        | "MEISTER HAND"
        | "大师之手"
        | "大師之手"
        | "마스터 핸드"
        | "ГЛАВНАЯ РУКА"
        | "MÃO MESTRA" => Some(UI_CHARA_MASTERHAND_HASH),
        "CRAZY HAND" | "CRAZYHAND" | "クレイジーハンド" => Some(UI_CHARA_CRAZYHAND_HASH),
        "DHARKON" | "DARZ" => Some(UI_CHARA_DARZ_HASH),
        "GALEEM" | "KIILA" => Some(UI_CHARA_KIILA_HASH),
        "MARX" => Some(UI_CHARA_MARX_HASH),
        "GANON" | "GANON BOSS" | "GANONBOSS" => Some(UI_CHARA_GANONBOSS_HASH),
        "DRACULA" => Some(UI_CHARA_DRACULA_HASH),
        "GALLEOM" => Some(UI_CHARA_GALLEOM_HASH),
        "RATHALOS" | "LIOLEUS" => Some(UI_CHARA_LIOLEUS_HASH),
        "WOL MASTER HAND"
        | "WOL MASTERHAND"
        | "WORLD OF LIGHT MASTER HAND"
        | "PLAYABLE MASTER HAND"
        | "PLAYABLE MASTERHAND"
        | "MEWTWO MASTERHAND"
        | "MEWTWO MASTER HAND" => Some(UI_CHARA_MEWTWO_MASTERHAND_HASH),
        _ => None,
    }
}

unsafe fn detect_character_name_text_base() -> u64 {
    let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
    let offset = match *TITLE_VERSION {
        (13, 0, 4) => 0x52C4758,
        (13, 0, 3) => 0x52C5758,
        (13, 0, 2) => 0x52C3758,
        _ => 0x52C4758,
    };
    text + offset
}

unsafe fn read_detected_character_name(addr: u64) -> Option<String> {
    let mut bytes = Vec::with_capacity(32);
    let mut cursor = addr as *const u16;

    for _ in 0..64 {
        let value = std::ptr::read_unaligned(cursor);
        if value == 0 {
            break;
        }
        bytes.push(value as u8);
        cursor = cursor.add(1);
    }

    if bytes.is_empty() {
        return None;
    }

    Some(String::from_utf8_lossy(&bytes).trim().to_string())
}

unsafe fn entry_idx_for_detected_character_name(
    mut module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<usize> {
    if module_accessor.is_null() {
        return None;
    }

    if smash::app::utility::get_kind(&mut *module_accessor) == *WEAPON_KIND_PTRAINER_PTRAINER {
        let entry_id = WorkModule::get_int(
            module_accessor,
            *WEAPON_PTRAINER_PTRAINER_INSTANCE_WORK_ID_INT_FIGHTER_ENTRY_ID,
        );
        return (0..MAX_FIGHTERS as i32)
            .contains(&entry_id)
            .then_some(entry_id as usize);
    }

    if smash::app::utility::get_category(&mut *module_accessor) == *BATTLE_OBJECT_CATEGORY_FIGHTER {
        let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
        return (0..MAX_FIGHTERS as i32)
            .contains(&entry_id)
            .then_some(entry_id as usize);
    }

    for _ in 0..8 {
        let owner_id =
            WorkModule::get_int(module_accessor, *WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER);
        if owner_id < 0 {
            return None;
        }

        let owner = sv_battle_object::module_accessor(owner_id as u32);
        if owner.is_null() {
            return None;
        }

        if smash::app::utility::get_category(&mut *owner) == *BATTLE_OBJECT_CATEGORY_FIGHTER {
            let entry_id = WorkModule::get_int(owner, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
            return (0..MAX_FIGHTERS as i32)
                .contains(&entry_id)
                .then_some(entry_id as usize);
        }

        module_accessor = owner;
    }

    None
}

unsafe fn selected_boss_selector_id_from_character_name(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<u64> {
    if !detect_character_name_enabled() {
        return None;
    }

    let entry_idx = entry_idx_for_detected_character_name(module_accessor)?;
    let name_base = detect_character_name_text_base();
    let addr = name_base
        + DETECT_CHARACTER_NAME_ENTRY_STRIDE * entry_idx as u64
        + DETECT_CHARACTER_NAME_TEXT_OFFSET;
    let detected_name = read_detected_character_name(addr)?;
    let resolved = detected_character_name_to_ui_hash(&detected_name);

    if crate::debug::enabled() {
        let canonical = canonicalize_detected_character_name(&detected_name);
        let name_hash = smash::hash40(canonical.as_str());
        let resolved_hash = resolved.unwrap_or(0);
        if LAST_LOGGED_NAME_SELECTOR_HASH[entry_idx] != name_hash
            || LAST_LOGGED_NAME_SELECTOR_RESULT[entry_idx] != resolved_hash
        {
            LAST_LOGGED_NAME_SELECTOR_HASH[entry_idx] = name_hash;
            LAST_LOGGED_NAME_SELECTOR_RESULT[entry_idx] = resolved_hash;
            crate::boss_log!(
                "[PB][SelectionName] entry {} version={}.{}.{} name=\"{}\" canonical=\"{}\" resolved=0x{:x}",
                entry_idx,
                TITLE_VERSION.0,
                TITLE_VERSION.1,
                TITLE_VERSION.2,
                detected_name,
                canonical,
                resolved_hash
            );
        }
    }

    resolved
}

#[inline(always)]
unsafe fn selected_boss_selector_id_from_runtime_sources(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<u64> {
    let info = fighter_information(module_accessor);
    if info.is_null() {
        return None;
    }
    let selector_id = smash::app::lua_bind::FighterInformation::summon_boss_id(info);
    let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    let log_selector_id: Option<u64>;
    let mut cache_selector_id: Option<u64> = None;
    let mut selected = selector_id;
    if crate::debug::enabled() && entry_id >= 0 && (entry_id as usize) < MAX_FIGHTERS {
        let idx = entry_id as usize;
        log_selector_id = log_int_css_selector_id(info, idx);
        if !is_known_boss_selector_value(selected) {
            if let Some(v) = log_selector_id {
                selected = v;
            }
        }
        if !is_known_boss_selector_value(selected) {
            cache_selector_id = cached_css_boss_hash(module_accessor, idx);
            if let Some(v) = cache_selector_id {
                selected = v;
            }
        }
        let normalized_selected = normalize_selector_value(selected);
        let resolved_selected = resolve_selector_value_to_ui_hash(normalized_selected);
        let should_log = LAST_LOGGED_SELECTOR_ID[idx] != selector_id
            || LAST_LOGGED_NORMALIZED_SELECTOR_ID[idx] != normalized_selected
            || LAST_LOGGED_RESOLVED_SELECTOR_ID[idx] != resolved_selected
            || LAST_LOGGED_SELECTION_LOG_SELECTOR_ID[idx] != log_selector_id.unwrap_or(u64::MAX)
            || LAST_LOGGED_CACHE_SELECTOR_ID[idx] != cache_selector_id.unwrap_or(u64::MAX);
        if should_log {
            LAST_LOGGED_SELECTOR_ID[idx] = selector_id;
            LAST_LOGGED_NORMALIZED_SELECTOR_ID[idx] = normalized_selected;
            LAST_LOGGED_RESOLVED_SELECTOR_ID[idx] = resolved_selected;
            LAST_LOGGED_SELECTION_LOG_SELECTOR_ID[idx] = log_selector_id.unwrap_or(u64::MAX);
            LAST_LOGGED_CACHE_SELECTOR_ID[idx] = cache_selector_id.unwrap_or(u64::MAX);
            let has_item = ItemModule::is_have_item(module_accessor, 0);
            let decoded_scalar = decode_tagged_selector_scalar(selector_id);
            let fighter_color = smash::app::lua_bind::FighterInformation::fighter_color(info);
            crate::boss_log!(
                "[PB][Selection] entry {} css_selector_raw=0x{:x} css_selector_decoded={:?} normalized=0x{:x} resolved=0x{:x} log_selector={:?} cache_selector={:?} fighter_color=0x{:x} has_item={}",
                idx,
                selector_id,
                decoded_scalar,
                normalized_selected,
                resolved_selected,
                log_selector_id.map(|v| format!("0x{:x}", v)),
                cache_selector_id.map(|v| format!("0x{:x}", v)),
                fighter_color,
                has_item
            );
        }
    } else if entry_id >= 0 && (entry_id as usize) < MAX_FIGHTERS {
        let idx = entry_id as usize;
        log_selector_id = log_int_css_selector_id(info, idx);
        if !is_known_boss_selector_value(selected) {
            if let Some(v) = log_selector_id {
                selected = v;
            }
        }
        if !is_known_boss_selector_value(selected) {
            cache_selector_id = cached_css_boss_hash(module_accessor, idx);
            if let Some(v) = cache_selector_id {
                selected = v;
            }
        }
    }
    selected = normalize_selector_value(selected);
    selected = resolve_selector_value_to_ui_hash(selected);
    if selected == 0 {
        None
    } else {
        Some(selected)
    }
}

fn hash_for_ui_chara_selector_id(selector: i32) -> Option<u64> {
    match selector {
        UI_CHARA_KOOPAG_SELECTOR => Some(UI_CHARA_KOOPAG_HASH),
        UI_CHARA_MASTERHAND_SELECTOR => Some(UI_CHARA_MASTERHAND_HASH),
        UI_CHARA_CRAZYHAND_SELECTOR => Some(UI_CHARA_CRAZYHAND_HASH),
        UI_CHARA_DARZ_SELECTOR => Some(UI_CHARA_DARZ_HASH),
        UI_CHARA_KIILA_SELECTOR => Some(UI_CHARA_KIILA_HASH),
        UI_CHARA_MARX_SELECTOR => Some(UI_CHARA_MARX_HASH),
        UI_CHARA_GANONBOSS_SELECTOR => Some(UI_CHARA_GANONBOSS_HASH),
        UI_CHARA_DRACULA_SELECTOR => Some(UI_CHARA_DRACULA_HASH),
        UI_CHARA_GALLEOM_SELECTOR => Some(UI_CHARA_GALLEOM_HASH),
        UI_CHARA_LIOLEUS_SELECTOR => Some(UI_CHARA_LIOLEUS_HASH),
        UI_CHARA_MEWTWO_MASTERHAND_SELECTOR => Some(UI_CHARA_MEWTWO_MASTERHAND_HASH),
        _ => None,
    }
}

fn normalize_ui_hash_candidate(raw: u64) -> Option<u64> {
    let masked = raw & HASH40_MASK;
    if is_boss_css_hash(masked) {
        return Some(masked);
    }

    let swapped_masked = raw.swap_bytes() & HASH40_MASK;
    if is_boss_css_hash(swapped_masked) {
        return Some(swapped_masked);
    }

    if is_boss_css_hash(raw) {
        return Some(raw);
    }

    let swapped = raw.swap_bytes();
    if is_boss_css_hash(swapped) {
        return Some(swapped);
    }

    None
}

#[inline(always)]
unsafe fn log_selection_hook(
    slot: usize,
    hook: &'static str,
    phase: &'static str,
    player_id: u32,
    raw_ui_hash: u64,
    normalized_ui_hash: Option<u64>,
    info_ptr: u64,
) {
    if !crate::debug::enabled() {
        return;
    }

    let normalized = normalized_ui_hash.unwrap_or(0);
    let signature = ((player_id as u64) << 8)
        ^ raw_ui_hash.rotate_left(17)
        ^ normalized.rotate_left(31)
        ^ info_ptr.rotate_left(47);
    let slot = slot.min(7);
    if LAST_SELECTION_BISECT_SIGNATURE[slot] == signature {
        return;
    }
    LAST_SELECTION_BISECT_SIGNATURE[slot] = signature;

    crate::boss_log!(
        "[PB][SelectionHook] hook={} phase={} source=raw_ui_callback stage=unobserved_ui_hook player={} raw_ui_hash=0x{:x} normalized_ui_hash=0x{:x} info_ptr=0x{:x} payload=opaque",
        hook,
        phase,
        player_id,
        raw_ui_hash,
        normalized,
        info_ptr
    );
}

/// Associate the latest boss lookup with the selection callback without
/// dereferencing its payload.  CSS normally confirms this candidate through a
/// lookup nested in the original callback; WOL retains it as tentative until a
/// validated stage-0x139 host consumes it later.
#[inline(always)]
unsafe fn arm_opaque_selection_candidate(player_id: u32, payload_ptr: u64, hook: &'static str) {
    let Some(entry_idx) = ((player_id as usize) < MAX_FIGHTERS).then_some(player_id as usize)
    else {
        return;
    };

    crate::amiibo_preview::discard_unbound_identity_from_raw_selection_callback();
    SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx] = false;
    SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY[entry_idx] = i32::MIN;
    let cached_hash = CACHED_BOSS_UI_HASH_GLOBAL;
    if is_boss_css_hash(cached_hash) {
        CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx] = cached_hash;
        CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx] =
            OpaqueSelectionCacheOrigin::TentativeUiSelection;
    } else {
        CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx] = 0;
        CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx] = OpaqueSelectionCacheOrigin::None;
    }
    PENDING_OPAQUE_SELECTION_ENTRY = entry_idx;

    if crate::debug::enabled() && LAST_LOGGED_SELECTION_INFO_HASH[entry_idx] != cached_hash {
        LAST_LOGGED_SELECTION_INFO_HASH[entry_idx] = cached_hash;
        crate::boss_log!(
            "[PB][SelectionHook] hook={} phase=opaque_candidate player={} entry={} candidate_ui_chara_hash=0x{:x} origin={} info_ptr=0x{:x}",
            hook,
            player_id,
            entry_idx,
            cached_hash,
            CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx].name(),
            payload_ptr
        );
    }
}

#[inline(always)]
unsafe fn resolve_pending_opaque_selection_lookup(
    raw_ui_hash: u64,
    normalized_ui_hash: Option<u64>,
) {
    let entry_idx = PENDING_OPAQUE_SELECTION_ENTRY;
    if entry_idx >= MAX_FIGHTERS {
        return;
    }

    PENDING_OPAQUE_SELECTION_ENTRY = usize::MAX;
    match normalized_ui_hash {
        Some(ui_hash) => {
            CACHED_BOSS_UI_HASH_GLOBAL = ui_hash;
            CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx] = ui_hash;
            CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx] =
                OpaqueSelectionCacheOrigin::ConfirmedUiLookup;
            log_css_selection_transition(entry_idx as u32, Some(entry_idx), raw_ui_hash, ui_hash);
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][SelectionHook] hook=capture_lookup_fighter_kind_from_ui_hash phase=lookup_confirmed entry={} ui_chara_hash=0x{:x}",
                    entry_idx,
                    ui_hash
                );
            }
        }
        None => {
            CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx] = 0;
            CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx] = OpaqueSelectionCacheOrigin::None;
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][SelectionHook] hook=capture_lookup_fighter_kind_from_ui_hash phase=lookup_non_boss entry={} action=clear_candidate",
                    entry_idx
                );
            }
        }
    }
}

#[inline(always)]
unsafe fn finish_opaque_selection_candidate(entry_idx: usize) {
    if PENDING_OPAQUE_SELECTION_ENTRY == entry_idx {
        // No lookup was nested in this callback. Keep a WOL-only tentative
        // candidate, but never let it affect an ordinary battle host.
        PENDING_OPAQUE_SELECTION_ENTRY = usize::MAX;
    }
}

#[skyline::hook(offset = SELECTION_UPDATE_SELECTED_FIGHTER_13_0_1)]
unsafe fn update_selected_fighter_capture_3310760(
    unk: u64,
    player_id: u32,
    new_selection_info: u64,
) {
    log_selection_hook(
        0,
        "update_selected_fighter_capture_3310760",
        "begin",
        player_id,
        0,
        None,
        new_selection_info,
    );
    arm_opaque_selection_candidate(
        player_id,
        new_selection_info,
        "update_selected_fighter_capture_3310760",
    );
    original!()(unk, player_id, new_selection_info);
    if (player_id as usize) < MAX_FIGHTERS {
        finish_opaque_selection_candidate(player_id as usize);
    }
    log_selection_hook(
        1,
        "update_selected_fighter_capture_3310760",
        "end",
        player_id,
        0,
        None,
        new_selection_info,
    );
}

// Some plugin stacks/game revisions route this callback at a nearby offset.
#[skyline::hook(offset = SELECTION_UPDATE_SELECTED_FIGHTER_13_0_2_PLUS)]
unsafe fn update_selected_fighter_capture_3311190(
    unk: u64,
    player_id: u32,
    new_selection_info: *const u8,
) {
    log_selection_hook(
        2,
        "update_selected_fighter_capture_3311190",
        "begin",
        player_id,
        0,
        None,
        new_selection_info as u64,
    );
    arm_opaque_selection_candidate(
        player_id,
        new_selection_info as u64,
        "update_selected_fighter_capture_3311190",
    );
    original!()(unk, player_id, new_selection_info);
    if (player_id as usize) < MAX_FIGHTERS {
        finish_opaque_selection_candidate(player_id as usize);
    }
    log_selection_hook(
        3,
        "update_selected_fighter_capture_3311190",
        "end",
        player_id,
        0,
        None,
        new_selection_info as u64,
    );
}

#[skyline::hook(offset = 0x3262130)]
unsafe fn capture_lookup_fighter_kind_from_ui_hash(database: u64, hash: u64) -> i32 {
    let normalized = normalize_ui_hash_candidate(hash);
    log_selection_hook(
        4,
        "capture_lookup_fighter_kind_from_ui_hash",
        "begin",
        u32::MAX,
        hash,
        normalized,
        database,
    );

    // A lookup nested in selected-fighter handling belongs to CSS/WOL state,
    // not the Figure Player viewer. Preserve its boss identity for selection,
    // but do not arm the stage-0x135 handoff from it.
    let selection_callback_pending = PENDING_OPAQUE_SELECTION_ENTRY < MAX_FIGHTERS;
    resolve_pending_opaque_selection_lookup(hash, normalized);
    if let Some(ui_hash) = normalized {
        CACHED_BOSS_UI_HASH_GLOBAL = ui_hash;
        if !selection_callback_pending {
            crate::amiibo_preview::observe_logical_identity_lookup(hash, ui_hash);
        }
        if crate::debug::enabled() && LAST_LOGGED_GLOBAL_CAPTURE_HASH != ui_hash {
            LAST_LOGGED_GLOBAL_CAPTURE_HASH = ui_hash;
            crate::boss_log!(
                "[PB][SelectionCapture] lookup_ui_hash raw=0x{:x} normalized=0x{:x}",
                hash,
                ui_hash
            );
        }
    }

    let result = original!()(database, hash);
    log_selection_hook(
        5,
        "capture_lookup_fighter_kind_from_ui_hash",
        "end",
        u32::MAX,
        hash,
        normalized,
        database,
    );
    result
}

#[skyline::hook(offset = SELECTION_UPDATE_CSS_13_0_1_PLUS)]
unsafe fn update_css_cache(unk: u64) {
    log_selection_hook(6, "update_css_cache", "begin", u32::MAX, 0, None, unk);
    // This callback is shared with WOL and has no stable payload contract at
    // startup. Its pointer remains opaque; the lookup hook carries identity.
    original!()(unk);
    log_selection_hook(7, "update_css_cache", "end", u32::MAX, 0, None, unk);
}

unsafe fn cached_css_boss_hash(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry_idx: usize,
) -> Option<u64> {
    if module_accessor.is_null() || entry_idx >= MAX_FIGHTERS {
        return None;
    }
    let by_entry = CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx];
    if !is_boss_css_hash(by_entry) {
        return None;
    }
    match CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx] {
        OpaqueSelectionCacheOrigin::ConfirmedUiLookup => Some(by_entry),
        // The previous lookup can be used for WOL only after a real Mario host
        // exists.  Raw UI callbacks never query this stage value.
        OpaqueSelectionCacheOrigin::TentativeUiSelection
            if smash::app::stage::get_stage_id() == crate::boss_helpers::STAGE_ID_BOSS_PREVIEW =>
        {
            Some(by_entry)
        }
        OpaqueSelectionCacheOrigin::None | OpaqueSelectionCacheOrigin::TentativeUiSelection => None,
    }
}

fn decode_tagged_selector_scalar(raw: u64) -> Option<u32> {
    // Some builds surface selector IDs as 0x5xxxxxxx tagged scalars.
    if raw <= u32::MAX as u64 {
        let scalar = raw as u32;
        if scalar & 0xF000_0000 == 0x5000_0000 {
            return Some(scalar & 0x0FFF_FFFF);
        }
        return Some(scalar);
    }

    let upper = (raw >> 32) as u32;
    let lower = raw as u32;
    // Some builds surface selector IDs as a 64-bit value with a small upper tag.
    if (1..=0x10).contains(&upper) {
        return Some(lower);
    }
    if upper & 0xF000_0000 == 0x5000_0000 {
        return Some(upper & 0x0FFF_FFFF);
    }
    None
}

fn is_boss_selector_id(value: i32) -> bool {
    matches!(
        value,
        UI_CHARA_KOOPAG_SELECTOR
            | UI_CHARA_MASTERHAND_SELECTOR
            | UI_CHARA_CRAZYHAND_SELECTOR
            | UI_CHARA_DARZ_SELECTOR
            | UI_CHARA_KIILA_SELECTOR
            | UI_CHARA_MARX_SELECTOR
            | UI_CHARA_GANONBOSS_SELECTOR
            | UI_CHARA_DRACULA_SELECTOR
            | UI_CHARA_GALLEOM_SELECTOR
            | UI_CHARA_LIOLEUS_SELECTOR
            | UI_CHARA_MEWTWO_MASTERHAND_SELECTOR
    )
}

fn is_boss_css_hash(value: u64) -> bool {
    matches!(
        value,
        UI_CHARA_KOOPAG_HASH
            | UI_CHARA_MASTERHAND_HASH
            | UI_CHARA_CRAZYHAND_HASH
            | UI_CHARA_DARZ_HASH
            | UI_CHARA_KIILA_HASH
            | UI_CHARA_MARX_HASH
            | UI_CHARA_GANONBOSS_HASH
            | UI_CHARA_DRACULA_HASH
            | UI_CHARA_GALLEOM_HASH
            | UI_CHARA_LIOLEUS_HASH
            | UI_CHARA_MEWTWO_MASTERHAND_HASH
    )
}

fn css_identity_label(value: u64) -> &'static str {
    match value {
        UI_CHARA_MARIO_HASH => "mario",
        UI_CHARA_KOOPAG_HASH => "giga_bowser",
        UI_CHARA_MASTERHAND_HASH => "master_hand",
        UI_CHARA_CRAZYHAND_HASH => "crazy_hand",
        UI_CHARA_DARZ_HASH => "dharkon",
        UI_CHARA_KIILA_HASH => "galeem",
        UI_CHARA_MARX_HASH => "marx",
        UI_CHARA_GANONBOSS_HASH => "ganon_boss",
        UI_CHARA_DRACULA_HASH => "dracula",
        UI_CHARA_GALLEOM_HASH => "galleom",
        UI_CHARA_LIOLEUS_HASH => "rathalos",
        UI_CHARA_MEWTWO_MASTERHAND_HASH => "wol_master_hand",
        _ => "unknown",
    }
}

unsafe fn log_css_selection_transition(
    player_id: u32,
    entry_idx: Option<usize>,
    raw_field_value: u64,
    selected_hash: u64,
) {
    if !crate::debug::enabled() {
        return;
    }

    let idx = entry_idx.unwrap_or((player_id as usize).min(MAX_FIGHTERS - 1));
    if LAST_LOGGED_CSS_SELECTION_RAW[idx] == raw_field_value
        && LAST_LOGGED_CSS_SELECTION_HASH[idx] == selected_hash
    {
        return;
    }

    LAST_LOGGED_CSS_SELECTION_RAW[idx] = raw_field_value;
    LAST_LOGGED_CSS_SELECTION_HASH[idx] = selected_hash;
    let identity = css_identity_label(selected_hash);
    let boss_mapping = if is_boss_css_hash(selected_hash) {
        identity
    } else {
        "none"
    };
    crate::boss_log!(
        "[PB][CSS] slot_index={} ui_chara_id=0x{:010x} selected_hash=0x{:010x} fighter_kind=unknown boss_mapping={} mario_selected={} custom_css={} detect_character_name={}",
        idx,
        raw_field_value & HASH40_MASK,
        selected_hash,
        boss_mapping,
        selected_hash == UI_CHARA_MARIO_HASH,
        CONFIG.options.custom_css.unwrap_or(false),
        CONFIG.options.detect_character_name.unwrap_or(false)
    );
}

fn resolve_selector_value_to_ui_hash(value: u64) -> u64 {
    if let Some(hash) = normalize_ui_hash_candidate(value) {
        return hash;
    }

    if let Some(decoded) = decode_tagged_selector_scalar(value) {
        if let Some(hash) = hash_for_ui_chara_selector_id(decoded as i32) {
            return hash;
        }
    }

    if value <= i32::MAX as u64 {
        if let Some(hash) = hash_for_ui_chara_selector_id(value as i32) {
            return hash;
        }
    }

    value
}

fn normalize_selector_value(value: u64) -> u64 {
    if is_boss_css_hash(value) {
        return value;
    }
    if let Some(decoded) = decode_tagged_selector_scalar(value) {
        if is_boss_selector_id(decoded as i32) {
            return decoded as u64;
        }
    }
    value
}

fn is_known_boss_selector_value(value: u64) -> bool {
    if is_boss_css_hash(value) {
        return true;
    }
    if let Some(decoded) = decode_tagged_selector_scalar(value) {
        return is_boss_selector_id(decoded as i32);
    }
    if value <= i32::MAX as u64 {
        return hash_for_ui_chara_selector_id(value as i32).is_some();
    }
    false
}

unsafe fn log_int_css_selector_id(info: *mut FighterInformation, entry_idx: usize) -> Option<u64> {
    let known_key = LOG_INT_SELECTOR_KEY[entry_idx];
    if let Some((a, b, c)) = known_key {
        let value = smash::cpp::root::app::lua_bind::FighterInformation::get_log_int(info, a, b, c);
        if is_known_boss_selector_value(value) {
            let normalized = normalize_selector_value(value);
            if crate::debug::enabled() && LAST_LOGGED_LOG_SELECTOR_ID[entry_idx] != normalized {
                LAST_LOGGED_LOG_SELECTOR_ID[entry_idx] = normalized;
                crate::boss_log!(
                    "[PB][SelectionLog] entry {} get_log_int({}, {}, {}) => raw=0x{:x} normalized=0x{:x}",
                    entry_idx,
                    a,
                    b,
                    c,
                    value,
                    normalized
                );
            }
            return Some(normalized);
        }
        // The previously discovered key stopped producing a boss hash (version drift or stale state).
        LOG_INT_SELECTOR_KEY[entry_idx] = None;
    }

    // Do not brute-force get_log_int index triples at runtime.
    // Some game builds crash on invalid tuples even when reads are infrequent.
    None
}

unsafe fn fighter_information(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> *mut FighterInformation {
    if module_accessor.is_null() {
        return std::ptr::null_mut();
    }
    let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if entry_id < 0 {
        return std::ptr::null_mut();
    }
    let manager = crate::boss_helpers::fighter_manager();
    if manager.is_null() {
        return std::ptr::null_mut();
    }
    crate::boss_helpers::fighter_information_entry(manager, entry_id as usize)
}

// Returns the raw CSS-selected boss selector value (from ui_chara_* row data),
// not the currently held/spawned item.
pub unsafe fn selected_css_boss_selector_id(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<u64> {
    if let Some(selected) = selected_boss_selector_id_from_runtime_sources(module_accessor) {
        return Some(selected);
    }

    selected_boss_selector_id_from_character_name(module_accessor)
}

pub fn install() {
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][SelectionInstall] hooks=[0x{:x},0x{:x},0x{:x},0x{:x}] mode=ui_chara_capture_only",
            SELECTION_UPDATE_CSS_13_0_1_PLUS,
            0x3262130usize,
            SELECTION_UPDATE_SELECTED_FIGHTER_13_0_1,
            SELECTION_UPDATE_SELECTED_FIGHTER_13_0_2_PLUS
        );
    }

    skyline::install_hooks!(
        update_css_cache,
        capture_lookup_fighter_kind_from_ui_hash,
        update_selected_fighter_capture_3310760,
        update_selected_fighter_capture_3311190
    );
}

unsafe fn expected_css_hash_for_selector(expected_selector_id: i32) -> Option<u64> {
    if expected_selector_id == *ITEM_KIND_MASTERHAND {
        Some(UI_CHARA_MASTERHAND_HASH)
    } else if expected_selector_id == *ITEM_KIND_CRAZYHAND {
        Some(UI_CHARA_CRAZYHAND_HASH)
    } else if expected_selector_id == *ITEM_KIND_DARZ {
        Some(UI_CHARA_DARZ_HASH)
    } else if expected_selector_id == *ITEM_KIND_KIILA {
        Some(UI_CHARA_KIILA_HASH)
    } else if expected_selector_id == *ITEM_KIND_MARX {
        Some(UI_CHARA_MARX_HASH)
    } else if expected_selector_id == *ITEM_KIND_GANONBOSS {
        Some(UI_CHARA_GANONBOSS_HASH)
    } else if expected_selector_id == *ITEM_KIND_DRACULA {
        Some(UI_CHARA_DRACULA_HASH)
    } else if expected_selector_id == *ITEM_KIND_GALLEOM {
        Some(UI_CHARA_GALLEOM_HASH)
    } else if expected_selector_id == *ITEM_KIND_LIOLEUSBOSS
        || expected_selector_id == *ITEM_KIND_LIOLEUS
    {
        Some(UI_CHARA_LIOLEUS_HASH)
    } else if expected_selector_id == *ITEM_KIND_PLAYABLE_MASTERHAND {
        Some(UI_CHARA_MEWTWO_MASTERHAND_HASH)
    } else {
        None
    }
}

pub unsafe fn is_selected_css_boss(
    module_accessor: *mut BattleObjectModuleAccessor,
    expected_selector_id: i32,
) -> bool {
    if !module_accessor.is_null() {
        let entry_idx =
            WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        if expected_selector_id == *ITEM_KIND_PLAYABLE_MASTERHAND
            && entry_idx < MAX_FIGHTERS
            && SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx]
        {
            return false;
        }
    }
    let Some(found) = selected_css_boss_selector_id(module_accessor) else {
        return false;
    };
    let expected_selector_u64 = expected_selector_id as u64;
    if found == expected_selector_u64 {
        return true;
    }
    if let Some(expected_hash) = expected_css_hash_for_selector(expected_selector_id) {
        if found == expected_hash {
            return true;
        }
    }
    match decode_tagged_selector_scalar(found) {
        Some(decoded) => decoded as i32 == expected_selector_id,
        None => false,
    }
}

pub unsafe fn suppress_boss_selection_until_ready_go(entry_idx: usize) {
    if entry_idx >= MAX_FIGHTERS {
        return;
    }
    let stage_id = smash::app::stage::get_stage_id();
    if !SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx] {
        SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx] = true;
        SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY[entry_idx] = stage_id;
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][Selection] suppress boss selection for entry {} until scene advances (stage=0x{:x} cached_hash=0x{:x})",
                entry_idx,
                stage_id,
                CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx]
            );
        }
    } else if SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY[entry_idx] != stage_id {
        SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY[entry_idx] = stage_id;
    }
}

pub unsafe fn is_boss_selection_suppressed(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let entry_idx =
        WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
    entry_idx < MAX_FIGHTERS && SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx]
}

pub unsafe fn clear_boss_selection_suppression_if_ready_go(
    module_accessor: *mut BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() {
        return;
    }
    let entry_idx =
        WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
    let ready_go = sv_information::is_ready_go();
    let current_stage = smash::app::stage::get_stage_id();
    let fighter_status = StatusModule::status_kind(module_accessor);
    let new_round_entry = fighter_status == *FIGHTER_STATUS_KIND_ENTRY
        || fighter_status == *FIGHTER_STATUS_KIND_REBIRTH;
    let preview_stage = crate::boss_helpers::is_boss_preview_stage(current_stage);

    if entry_idx < MAX_FIGHTERS && SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx] {
        let suppressed_stage = SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY[entry_idx];
        if !ready_go && current_stage == suppressed_stage && !new_round_entry && !preview_stage {
            return;
        }
        SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx] = false;
        SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY[entry_idx] = i32::MIN;
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][Selection] clear boss selection suppression for entry {} on {} ready_go={} current_stage=0x{:x} suppressed_stage=0x{:x} fighter_status={} new_round_entry={} preview_stage={} cached_hash=0x{:x}",
                entry_idx,
                if ready_go {
                    "ready_go"
                } else if new_round_entry {
                    "fighter_entry"
                } else if preview_stage {
                    "preview_stage"
                } else {
                    "scene_change"
                },
                ready_go,
                current_stage,
                suppressed_stage,
                fighter_status,
                new_round_entry,
                preview_stage,
                CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx]
            );
        }
    }
}

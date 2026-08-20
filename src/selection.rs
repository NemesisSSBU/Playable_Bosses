use crate::config::CONFIG;
use crate::config::CONFIG_DIR;
use once_cell::sync::Lazy;
use skyline::nn::hid::{
    GetNpadFullKeyState, GetNpadGcState, GetNpadHandheldState, GetNpadJoyDualState,
    GetNpadJoyLeftState, GetNpadJoyRightState, GetNpadStyleSet, NpadGcState, NpadHandheldState,
};
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
/// Provenance capture only. Monotonic transaction counter, and the last
/// entry/hash pair seen by the shared `update_css_cache` callback, so a commit
/// can report whether an entry-specific callback corroborated the same identity.
static mut SELECTION_TXN_SEQUENCE: u32 = 0;
static mut LAST_CSS_CACHE_TXN_SEQUENCE: u32 = u32::MAX;
static mut SELECTION_TXN_LOG_BUDGET: u32 = 400;

#[inline(always)]
unsafe fn selection_txn_budget_take() -> bool {
    if !crate::debug::enabled() || SELECTION_TXN_LOG_BUDGET == 0 {
        return false;
    }
    SELECTION_TXN_LOG_BUDGET -= 1;
    true
}
static mut CACHED_BOSS_UI_HASH_BY_ENTRY: [u64; MAX_FIGHTERS] = [0; MAX_FIGHTERS];
static mut CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY: [OpaqueSelectionCacheOrigin; MAX_FIGHTERS] =
    [OpaqueSelectionCacheOrigin::None; MAX_FIGHTERS];
static mut LAST_LOGGED_GLOBAL_CAPTURE_HASH: u64 = 0;
static mut LAST_LOGGED_SELECTION_INFO_HASH: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_CSS_SELECTION_RAW: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_CSS_SELECTION_HASH: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_NAME_SELECTOR_HASH: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_NAME_SELECTOR_RESULT: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_CONDENSED_SELECTION: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_LOGGED_CONDENSED_CARRIER: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut SUPPRESS_BOSS_SELECTION_BY_ENTRY: [bool; MAX_FIGHTERS] = [false; MAX_FIGHTERS];
static mut SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY: [i32; MAX_FIGHTERS] = [i32::MIN; MAX_FIGHTERS];
// Begin/end slots for the four installed selection hooks. These diagnostics are
// state-transition based so WOL selection can be bisected without logging every
// menu update.
static mut LAST_SELECTION_BISECT_SIGNATURE: [u64; 8] = [u64::MAX; 8];

// Raw selection callbacks run before the battle-stage subsystem is guaranteed
// to exist.  Keep their identity handoff explicitly opaque; only a validated
// fighter frame may classify a tentative value as a WOL preview selection.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum OpaqueSelectionCacheOrigin {
    None,
    TentativeUiSelection,
    /// Named boss lookup nested in an owned selected-fighter transaction.
    /// Observational only: Smash's startup/entry sync produces this without a
    /// user pick. Must not persist, must not apply to a CPU, and must not
    /// outrank Restored until an independent corroboration signal exists.
    CandidateUiLookup,
    ConfirmedUiLookup,
    ConfirmedCondensedCarrier,
    /// Restored from disk at plugin load. Bootstrap fallback only: weaker than
    /// any positively resolved current-session identity (corroborated CSS
    /// selection, character name, live summon/log selector). Kept distinct so
    /// logs can tell a live confirmation from a restored one, and so
    /// consumption can refuse to apply leftover persist to whoever later
    /// occupies the slot.
    RestoredPersistedSelection,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct PendingOpaqueSelection {
    entry_idx: usize,
    fallback_hash: u64,
    observed_boss_hash: u64,
    /// Transaction sequence that produced `observed_boss_hash`. Zero means no
    /// named boss lookup has been observed while this transaction was active.
    /// That stamp proves nesting, not user selection. `fallback_hash` never
    /// writes it.
    observed_at_sequence: u32,
    saw_mario: bool,
    ambiguous: bool,
    // Provenance capture only. None of these influence any decision; they exist
    // so an A/B hardware capture can separate a genuine per-entry selection from
    // a generic roster enumeration pass. Hash equality with a restored slot is
    // also not a substitute for these fields.
    outer_hook: u8,
    outer_player_id: u32,
    outer_info_ptr: u64,
    sequence: u32,
    observation_count: u16,
    first_raw_lookup: u64,
}

impl PendingOpaqueSelection {
    const EMPTY: Self = Self {
        entry_idx: usize::MAX,
        fallback_hash: 0,
        observed_boss_hash: 0,
        observed_at_sequence: 0,
        saw_mario: false,
        ambiguous: false,
        outer_hook: 0,
        outer_player_id: u32::MAX,
        outer_info_ptr: 0,
        sequence: 0,
        observation_count: 0,
        first_raw_lookup: 0,
    };

    const fn begin(entry_idx: usize, fallback_hash: u64) -> Self {
        Self {
            entry_idx,
            fallback_hash,
            observed_boss_hash: 0,
            observed_at_sequence: 0,
            saw_mario: false,
            ambiguous: false,
            outer_hook: 0,
            outer_player_id: u32::MAX,
            outer_info_ptr: 0,
            // Non-zero so an open transaction is distinguishable from EMPTY.
            // Production overwrites this with the monotonic counter in `arm`.
            sequence: 1,
            observation_count: 0,
            first_raw_lookup: 0,
        }
    }

    fn observe_named_lookup(&mut self, ui_hash: Option<u64>) {
        let Some(ui_hash) = ui_hash else {
            return;
        };
        if ui_hash == UI_CHARA_MARIO_HASH {
            self.saw_mario = true;
            return;
        }
        if !is_boss_css_hash(ui_hash) {
            return;
        }
        if self.observed_boss_hash == 0 || self.observed_boss_hash == ui_hash {
            self.observed_boss_hash = ui_hash;
            self.observed_at_sequence = self.sequence;
        } else {
            self.ambiguous = true;
        }
    }
}

static mut PENDING_OPAQUE_SELECTION: PendingOpaqueSelection = PendingOpaqueSelection::EMPTY;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum OpaqueSelectionCommit {
    Cached {
        ui_hash: u64,
        origin: OpaqueSelectionCacheOrigin,
    },
    Clear {
        reason: &'static str,
    },
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

fn canonical_detected_character_name_to_ui_hash(name: &str) -> Option<u64> {
    match name {
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
    let mut bytes = [0u8; 64];
    let mut len = 0;
    let mut cursor = addr as *const u16;

    while len < bytes.len() {
        let value = std::ptr::read_unaligned(cursor);
        if value == 0 {
            break;
        }
        bytes[len] = value as u8;
        len += 1;
        cursor = cursor.add(1);
    }

    if len == 0 {
        return None;
    }

    Some(String::from_utf8_lossy(&bytes[..len]).trim().to_string())
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
    let canonical = canonicalize_detected_character_name(&detected_name);
    let resolved = canonical_detected_character_name_to_ui_hash(&canonical);

    if crate::debug::enabled() {
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
    if module_accessor.is_null() {
        return None;
    }
    let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    let info = fighter_information_for_entry(entry_id);
    if info.is_null() {
        return None;
    }
    let selector_id = smash::app::lua_bind::FighterInformation::summon_boss_id(info);
    let mut selected = selector_id;
    let mut log_context = None;
    if entry_id >= 0 && (entry_id as usize) < MAX_FIGHTERS {
        let idx = entry_id as usize;
        let log_selector_id = log_int_css_selector_id(info, idx);
        if !is_known_boss_selector_value(selected) {
            if let Some(v) = log_selector_id {
                selected = v;
            }
        }
        // Cache is not a live fighter-info source. Including it here made
        // `runtime_sources` report a restored persist hash and then beat
        // `name_detection` in `selected_css_boss_selector_id`. Log it only.
        let cache_selector_id = if !is_known_boss_selector_value(selected) {
            cached_css_boss_hash(module_accessor, idx)
        } else {
            None
        };
        log_context = Some((idx, log_selector_id, cache_selector_id));
    }

    let normalized_selected = normalize_selector_value(selected);
    let resolved_selected = resolve_selector_value_to_ui_hash(normalized_selected);
    if crate::debug::enabled() {
        if let Some((idx, log_selector_id, cache_selector_id)) = log_context {
            let should_log = LAST_LOGGED_SELECTOR_ID[idx] != selector_id
                || LAST_LOGGED_NORMALIZED_SELECTOR_ID[idx] != normalized_selected
                || LAST_LOGGED_RESOLVED_SELECTOR_ID[idx] != resolved_selected
                || LAST_LOGGED_SELECTION_LOG_SELECTOR_ID[idx]
                    != log_selector_id.unwrap_or(u64::MAX)
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
        }
    }
    resolved_live_boss_hash(resolved_selected)
}

/// Live fighter-info may only supply a recognised boss CSS identity.
/// The 0x50000000 "no summon" sentinel, raw zeros, and other non-boss
/// scalars are absence of current evidence — not an identity that can
/// outrank name detection or a restored fallback.
fn resolved_live_boss_hash(resolved_selected: u64) -> Option<u64> {
    is_boss_css_hash(resolved_selected).then_some(resolved_selected)
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

fn normalize_known_ui_hash_candidate(raw: u64) -> Option<u64> {
    let masked = raw & HASH40_MASK;
    if masked == UI_CHARA_MARIO_HASH || is_boss_css_hash(masked) {
        return Some(masked);
    }

    let swapped_masked = raw.swap_bytes() & HASH40_MASK;
    if swapped_masked == UI_CHARA_MARIO_HASH || is_boss_css_hash(swapped_masked) {
        return Some(swapped_masked);
    }

    if raw == UI_CHARA_MARIO_HASH || is_boss_css_hash(raw) {
        return Some(raw);
    }

    let swapped = raw.swap_bytes();
    if swapped == UI_CHARA_MARIO_HASH || is_boss_css_hash(swapped) {
        return Some(swapped);
    }

    None
}

fn normalize_ui_hash_candidate(raw: u64) -> Option<u64> {
    normalize_known_ui_hash_candidate(raw).filter(|hash| is_boss_css_hash(*hash))
}

/// A named boss observation counts as in-transaction evidence only when it was
/// stamped with this transaction's sequence. That is not enough for Confirmed*:
/// Smash's generic entry sync nests lookups in owned selected-fighter
/// callbacks. `fallback_hash` is seeded at `begin` from the pre-transaction
/// global and must never take this path.
fn named_observation_belongs_to_this_transaction(pending: &PendingOpaqueSelection) -> bool {
    pending.observed_boss_hash != 0
        && pending.sequence != 0
        && pending.observed_at_sequence == pending.sequence
}

/// Second, independent selection signal required before Confirmed* / disk /
/// CPU authority. Hardware 2026-08-17: `observed_at_sequence == sequence`
/// was true for an automatic Rathalos confirm on Spirit CPU Mario
/// (`css_cache_corroborated=false`). Do not gate on css-cache corroboration
/// until an explicit CPU pick A/B exists. Fail closed until then.
fn independent_selection_corroborated() -> bool {
    false
}

fn pending_selection_commit(
    condensed_enabled: bool,
    pending: PendingOpaqueSelection,
) -> OpaqueSelectionCommit {
    if pending.ambiguous {
        return OpaqueSelectionCommit::Clear {
            reason: "ambiguous_named_boss_lookups",
        };
    }

    if named_observation_belongs_to_this_transaction(&pending) {
        if independent_selection_corroborated() {
            let origin =
                if condensed_enabled && pending.observed_boss_hash == UI_CHARA_MASTERHAND_HASH {
                    OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier
                } else {
                    OpaqueSelectionCacheOrigin::ConfirmedUiLookup
                };
            return OpaqueSelectionCommit::Cached {
                ui_hash: pending.observed_boss_hash,
                origin,
            };
        }
        return OpaqueSelectionCommit::Cached {
            ui_hash: pending.observed_boss_hash,
            origin: OpaqueSelectionCacheOrigin::CandidateUiLookup,
        };
    }

    if pending.saw_mario {
        return OpaqueSelectionCommit::Clear {
            reason: "named_mario_selection",
        };
    }

    if is_boss_css_hash(pending.fallback_hash) {
        return OpaqueSelectionCommit::Cached {
            ui_hash: pending.fallback_hash,
            origin: OpaqueSelectionCacheOrigin::TentativeUiSelection,
        };
    }

    OpaqueSelectionCommit::Clear {
        reason: "no_named_ui_identity",
    }
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
        // Nested lookups during an owned txn must remain visible. An ambient
        // lookup of the same hash would otherwise suppress the SelectionHook
        // line that proves the observation happened after begin.
        let txn_open = PENDING_OPAQUE_SELECTION.entry_idx < MAX_FIGHTERS;
        if !txn_open || (slot != 4 && slot != 5) {
            return;
        }
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

/// Open a per-entry selection transaction without dereferencing the callback's
/// opaque payload. Only named UI lookups nested in this callback may commit a
/// production CSS identity. The global lookup remains a WOL-only fallback.
#[inline(always)]
unsafe fn arm_opaque_selection_candidate(player_id: u32, payload_ptr: u64, hook: &'static str) {
    let Some(entry_idx) = ((player_id as usize) < MAX_FIGHTERS).then_some(player_id as usize)
    else {
        return;
    };

    crate::amiibo_preview::discard_unbound_identity_from_raw_selection_callback();
    SUPPRESS_BOSS_SELECTION_BY_ENTRY[entry_idx] = false;
    SUPPRESS_BOSS_SELECTION_STAGE_BY_ENTRY[entry_idx] = i32::MIN;
    reset_condensed_selection(entry_idx);
    // Pre-transaction ambient identity. Stored only as `fallback_hash`.
    // It must not populate `observed_boss_hash`; that field is written only
    // by a nested lookup after this transaction is already in PENDING.
    let fallback_hash = CACHED_BOSS_UI_HASH_GLOBAL;
    SELECTION_TXN_SEQUENCE = SELECTION_TXN_SEQUENCE.wrapping_add(1);
    if SELECTION_TXN_SEQUENCE == 0 {
        SELECTION_TXN_SEQUENCE = 1;
    }
    let mut pending = PendingOpaqueSelection::begin(entry_idx, fallback_hash);
    pending.outer_hook = if hook.ends_with("3310760") { 1 } else { 2 };
    pending.outer_player_id = player_id;
    pending.outer_info_ptr = payload_ptr;
    pending.sequence = SELECTION_TXN_SEQUENCE;
    PENDING_OPAQUE_SELECTION = pending;
    if selection_txn_budget_take() {
        let seq = SELECTION_TXN_SEQUENCE;
        crate::boss_log!(
            "[PB][SelectionTxn] phase=begin entry={} outer_hook={} outer_player_id={} outer_info_ptr=0x{:x} transaction_sequence={} fallback_hash=0x{:010x}",
            entry_idx,
            hook,
            player_id,
            payload_ptr,
            seq,
            fallback_hash
        );
    }

    if crate::debug::enabled() && LAST_LOGGED_SELECTION_INFO_HASH[entry_idx] != fallback_hash {
        LAST_LOGGED_SELECTION_INFO_HASH[entry_idx] = fallback_hash;
        crate::boss_log!(
            "[PB][SelectionHook] hook={} phase=selection_transaction_begin player={} entry={} wol_fallback_ui_chara_hash=0x{:x} info_ptr=0x{:x} payload=opaque",
            hook,
            player_id,
            entry_idx,
            fallback_hash,
            payload_ptr
        );
    }
}

#[inline(always)]
unsafe fn observe_pending_opaque_selection_lookup(raw_ui_hash: u64) {
    // Sole producer of `observation_count` and of in-txn `observed_boss_hash`.
    // Called only from `capture_lookup_fighter_kind_from_ui_hash`. If no owned
    // transaction is armed, this returns without writing either field.
    if PENDING_OPAQUE_SELECTION.entry_idx >= MAX_FIGHTERS {
        return;
    }
    let mut pending = PENDING_OPAQUE_SELECTION;
    let normalized = normalize_known_ui_hash_candidate(raw_ui_hash);
    pending.observation_count = pending.observation_count.saturating_add(1);
    if pending.first_raw_lookup == 0 {
        pending.first_raw_lookup = raw_ui_hash;
    }
    pending.observe_named_lookup(normalized);
    PENDING_OPAQUE_SELECTION = pending;
    if selection_txn_budget_take() {
        crate::boss_log!(
            "[PB][SelectionTxn] phase=observe entry={} transaction_sequence={} observed_at_sequence={} raw_lookup_hash=0x{:x} normalized_lookup_hash=0x{:010x} observed_boss_hash=0x{:010x} observation_count={} boss={}",
            pending.entry_idx,
            pending.sequence,
            pending.observed_at_sequence,
            raw_ui_hash,
            normalized.unwrap_or(0),
            pending.observed_boss_hash,
            pending.observation_count,
            css_identity_label(normalized.unwrap_or(0))
        );
    }
}

#[inline(always)]
unsafe fn log_condensed_carrier_transition(
    pending: PendingOpaqueSelection,
    commit: OpaqueSelectionCommit,
) {
    if !crate::debug::enabled() || !condensed_mode_enabled() || pending.entry_idx >= MAX_FIGHTERS {
        return;
    }

    let (selected_ui_hash, carrier_confirmed, failure_reason, detection_source) = match commit {
        OpaqueSelectionCommit::Cached {
            ui_hash,
            origin: OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier,
        } => (
            ui_hash,
            true,
            "none",
            "selected_fighter_named_master_hand_lookup",
        ),
        OpaqueSelectionCommit::Cached {
            ui_hash,
            origin: OpaqueSelectionCacheOrigin::ConfirmedUiLookup,
        } => (
            ui_hash,
            false,
            "not_master_hand_selection",
            "selected_fighter_named_ui_lookup",
        ),
        OpaqueSelectionCommit::Cached {
            ui_hash,
            origin: OpaqueSelectionCacheOrigin::CandidateUiLookup,
        } => (
            ui_hash,
            false,
            "named_observation_candidate_not_corroborated",
            "selected_fighter_named_ui_lookup",
        ),
        OpaqueSelectionCommit::Cached {
            ui_hash,
            origin: OpaqueSelectionCacheOrigin::TentativeUiSelection,
        } => (
            ui_hash,
            false,
            "named_selection_unobserved",
            "wol_tentative_global_only",
        ),
        OpaqueSelectionCommit::Cached { ui_hash, .. } => {
            (ui_hash, false, "unrecognized_cache_origin", "internal")
        }
        OpaqueSelectionCommit::Clear { reason } => (
            0,
            false,
            reason,
            "selected_fighter_named_lookup_transaction",
        ),
    };
    let master_hand_selection_detected =
        pending.observed_boss_hash == UI_CHARA_MASTERHAND_HASH && !pending.ambiguous;
    let signature = selected_ui_hash
        ^ pending.observed_boss_hash.rotate_left(11)
        ^ pending.fallback_hash.rotate_left(23)
        ^ ((pending.saw_mario as u64) << 61)
        ^ ((pending.ambiguous as u64) << 62)
        ^ ((carrier_confirmed as u64) << 63);
    if LAST_LOGGED_CONDENSED_CARRIER[pending.entry_idx] == signature {
        return;
    }
    LAST_LOGGED_CONDENSED_CARRIER[pending.entry_idx] = signature;
    crate::boss_log!(
        "[PB][CondensedCarrier] entry={} condensed_enabled=true master_hand_selection_detected={} detection_source={} selected_ui_hash=0x{:010x} carrier_confirmed={} failure_reason={}",
        pending.entry_idx,
        master_hand_selection_detected,
        detection_source,
        selected_ui_hash,
        carrier_confirmed,
        failure_reason
    );
}

#[inline(always)]
unsafe fn finish_opaque_selection_candidate(entry_idx: usize) {
    if PENDING_OPAQUE_SELECTION.entry_idx != entry_idx {
        return;
    }

    let pending = PENDING_OPAQUE_SELECTION;
    PENDING_OPAQUE_SELECTION = PendingOpaqueSelection::EMPTY;
    let commit = pending_selection_commit(condensed_mode_enabled(), pending);
    if selection_txn_budget_take() {
        let (proposed_origin, proposed_hash) = match commit {
            OpaqueSelectionCommit::Cached { ui_hash, origin } => (origin_label(origin), ui_hash),
            OpaqueSelectionCommit::Clear { reason } => (reason, 0u64),
        };
        // Corroboration: did the shared entry-specific CSS callback run inside
        // this exact transaction window?
        let css_cache_corroborated = LAST_CSS_CACHE_TXN_SEQUENCE == pending.sequence;
        crate::boss_log!(
            "[PB][SelectionTxn] phase=commit entry={} outer_hook={} outer_player_id={} outer_info_ptr=0x{:x} transaction_sequence={} observed_at_sequence={} raw_lookup_hash=0x{:x} observed_boss_hash=0x{:010x} fallback_hash=0x{:010x} observation_count={} saw_mario={} ambiguous={} proposed_origin={} proposed_hash=0x{:010x} boss={} css_cache_corroborated={} independent_corroboration={} would_persist={}",
            pending.entry_idx,
            if pending.outer_hook == 1 { "3310760" } else { "3311190" },
            pending.outer_player_id,
            pending.outer_info_ptr,
            pending.sequence,
            pending.observed_at_sequence,
            pending.first_raw_lookup,
            pending.observed_boss_hash,
            pending.fallback_hash,
            pending.observation_count,
            pending.saw_mario,
            pending.ambiguous,
            proposed_origin,
            proposed_hash,
            css_identity_label(proposed_hash),
            css_cache_corroborated,
            independent_selection_corroborated(),
            matches!(commit, OpaqueSelectionCommit::Cached { ui_hash, origin }
                if origin_is_authoritative_selection(origin)
                    && is_persistable_host_boss_hash(ui_hash))
        );
    }
    match commit {
        OpaqueSelectionCommit::Cached { ui_hash, origin } => {
            // `arm_opaque_selection_candidate` seeds every transaction with the
            // single global CACHED_BOSS_UI_HASH_GLOBAL, so enumerating players
            // 4..7 leaves that global holding an unrelated boss. Committing it
            // as a per-entry TentativeUiSelection overwrote entry 0's restored
            // Master Hand with Dharkon; `cached_css_boss_hash` then refuses a
            // Tentative origin outside the WOL preview stage, which is exactly
            // the observed `cache_selector=None` and the lost takeover (#89).
            //
            // A weaker origin may never replace a stronger one for an entry.
            let existing_hash = CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx];
            let existing_origin = CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx];
            let existing_rank = origin_authority_rank(existing_origin);
            let has_existing = is_boss_css_hash(existing_hash);
            if has_existing && origin_authority_rank(origin) < existing_rank {
                log_condensed_carrier_transition(pending, commit);
                return;
            }
            // Nested named lookups are Candidate until an independent
            // corroboration signal exists. Candidate cannot outrank Restored,
            // cannot persist, and cannot apply to a CPU. Hash equality is
            // not a veto and not a promotion.
            CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx] = ui_hash;
            CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx] = origin;
            // Authoritative named/carrier selections reach disk immediately.
            // Tentative guesses and restored bootstrap values never do.
            if origin_is_authoritative_selection(origin) {
                persist_authoritative_selection(entry_idx, ui_hash);
            }
            log_css_selection_transition(entry_idx as u32, Some(entry_idx), ui_hash, ui_hash);
        }
        OpaqueSelectionCommit::Clear { reason } => {
            // Menu navigation constantly emits selection transactions carrying
            // no named UI identity (`no_named_ui_identity`). Before 3.1.0 that
            // was harmless, but the persisted-selection restore now seeds this
            // cache at plugin init, and the first such transaction wiped it --
            // so a cold launch straight into Spirit Board resolved to the Mario
            // host until the Fighter tab was visited (issue #89).
            //
            // Identity-free noise must not destroy a restored selection. A
            // genuine Mario pick still reports `named_mario_selection`, and an
            // ambiguous lookup still reports its own reason, so both continue
            // to clear normally.
            let restored_selection_pending = CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx]
                == OpaqueSelectionCacheOrigin::RestoredPersistedSelection;
            if !(restored_selection_pending && reason == "no_named_ui_identity") {
                CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx] = 0;
                CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx] = OpaqueSelectionCacheOrigin::None;
            }
            // Only an explicit Mario pick is authoritative enough to erase the
            // saved boss. Ambiguous lookups and identity-free menu noise clear
            // at most the in-memory cache and never touch disk.
            if reason == "named_mario_selection" {
                clear_persisted_selection(entry_idx);
            }
        }
    }
    log_condensed_carrier_transition(pending, commit);
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
    let known_ui_hash = normalize_known_ui_hash_candidate(hash);
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
    let selection_callback_pending = PENDING_OPAQUE_SELECTION.entry_idx < MAX_FIGHTERS;
    observe_pending_opaque_selection_lookup(hash);
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
    } else if !selection_callback_pending && known_ui_hash == Some(UI_CHARA_MARIO_HASH) {
        // A real Mario lookup invalidates any stale global boss candidate.
        // This prevents selecting ordinary Mario after browsing the carrier
        // from arming a condensed boss for that player.
        CACHED_BOSS_UI_HASH_GLOBAL = 0;
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
    // Provenance capture only: note that this shared callback ran inside the
    // currently-open transaction, so a commit can report corroboration.
    if PENDING_OPAQUE_SELECTION.entry_idx < MAX_FIGHTERS {
        LAST_CSS_CACHE_TXN_SEQUENCE = PENDING_OPAQUE_SELECTION.sequence;
    }
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
        OpaqueSelectionCacheOrigin::ConfirmedUiLookup
        | OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier => Some(by_entry),
        // Restored persist is a cold-launch fallback for the human player
        // who never revisited Fighter Selection. It must not transform an
        // unrelated fighter that later occupies this entry index — including
        // a Spirit CPU Mario sitting in a slot that last stored a CPU boss.
        // Genuine this-session CPU bosses arrive as Confirmed* above.
        origin @ OpaqueSelectionCacheOrigin::RestoredPersistedSelection => {
            if cache_visible_on_battle_stage(origin, entry_operation_cpu(entry_idx)) {
                Some(by_entry)
            } else {
                None
            }
        }
        // The previous lookup can be used for WOL only after a real Mario host
        // exists.  Raw UI callbacks never query this stage value.
        OpaqueSelectionCacheOrigin::TentativeUiSelection
            if smash::app::stage::get_stage_id() == crate::boss_helpers::STAGE_ID_BOSS_PREVIEW =>
        {
            Some(by_entry)
        }
        OpaqueSelectionCacheOrigin::None
        | OpaqueSelectionCacheOrigin::TentativeUiSelection
        | OpaqueSelectionCacheOrigin::CandidateUiLookup => None,
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

/// Persistence eligibility, deliberately distinct from [`is_boss_css_hash`].
///
/// `is_boss_css_hash` answers "is this a selectable boss CSS identity?" and must
/// keep recognising Giga Bowser so ordinary selection and gameplay work.
///
/// This answers a narrower question: "can this identity be reconstructed through
/// the Mario-host takeover on a cold launch?" Giga Bowser cannot -- he is a
/// dedicated native fighter (`FIGHTER_KIND_KOOPAG`) whose selection Smash itself
/// persists, and no code outside this module consumes a koopag selection to
/// convert a Mario host. Restoring him therefore produced a cache that named a
/// boss no module could ever claim, leaving a plain Mario in Spirit battles
/// while the HUD still showed the game's own saved fighter (issue #89).
fn is_persistable_host_boss_hash(value: u64) -> bool {
    is_boss_css_hash(value) && value != UI_CHARA_KOOPAG_HASH
}

/// Authority rank. A commit may only replace a cached identity of equal or
/// lower rank, which is what stops generic menu enumeration from overwriting a
/// restored or confirmed selection.
///
/// Confirmed (current process, independently corroborated)
///   > RestoredPersistedSelection (bootstrap fallback)
///   > CandidateUiLookup / TentativeUiSelection (observational / global guess)
///   > None
///
/// Rank is not consumption eligibility. Restored can still lose to a live
/// character name or summon. A nested named lookup without independent
/// corroboration is Candidate, not Confirmed, even when the hash is new.
fn origin_authority_rank(origin: OpaqueSelectionCacheOrigin) -> u8 {
    match origin {
        OpaqueSelectionCacheOrigin::None => 0,
        OpaqueSelectionCacheOrigin::TentativeUiSelection
        | OpaqueSelectionCacheOrigin::CandidateUiLookup => 1,
        OpaqueSelectionCacheOrigin::RestoredPersistedSelection => 2,
        OpaqueSelectionCacheOrigin::ConfirmedUiLookup
        | OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier => 3,
    }
}

/// True for origins that represent an authoritative selection made in THIS
/// process. Candidate, Tentative, and Restored must not write disk.
fn origin_is_authoritative_selection(origin: OpaqueSelectionCacheOrigin) -> bool {
    matches!(
        origin,
        OpaqueSelectionCacheOrigin::ConfirmedUiLookup
            | OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier
    )
}

/// Restored persist is eligible only for a human-controlled occupant. CPU
/// entries need a this-session Confirmed* origin; they must not inherit a
/// stale slot from last_boss_selection.txt. This is provenance, not a CPU
/// boss blacklist. Missing fighter-info fails closed (treated as ineligible).
unsafe fn entry_operation_cpu(entry_idx: usize) -> bool {
    let info = fighter_information_for_entry(entry_idx as i32);
    info.is_null() || smash::app::lua_bind::FighterInformation::is_operation_cpu(info)
}

/// Whether a cached origin is visible to the battle resolver for this occupant.
fn cache_visible_on_battle_stage(origin: OpaqueSelectionCacheOrigin, operation_cpu: bool) -> bool {
    match origin {
        OpaqueSelectionCacheOrigin::ConfirmedUiLookup
        | OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier => true,
        OpaqueSelectionCacheOrigin::RestoredPersistedSelection => !operation_cpu,
        OpaqueSelectionCacheOrigin::None
        | OpaqueSelectionCacheOrigin::TentativeUiSelection
        | OpaqueSelectionCacheOrigin::CandidateUiLookup => false,
    }
}

/// Current-session evidence outranks restored persist. Live summon/log, then
/// character name, then eligible cache. Non-boss values (sentinel, Mario) are
/// absence of evidence, not identities.
fn resolve_current_boss_identity(
    live_runtime: Option<u64>,
    name_detection: Option<u64>,
    cache: Option<u64>,
) -> Option<u64> {
    live_runtime
        .filter(|hash| is_boss_css_hash(*hash))
        .or_else(|| name_detection.filter(|hash| is_boss_css_hash(*hash)))
        .or_else(|| cache.filter(|hash| is_boss_css_hash(*hash)))
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

unsafe fn fighter_information_for_entry(entry_id: i32) -> *mut FighterInformation {
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

// ---------------------------------------------------------------------------
// Condensed single-slot boss roster (CONDENSE_BOSSES_INTO_SINGLE_SLOT).
//
// Presentation feature only. It changes which CSS entry the player picks from;
// it never changes how a boss spawns. The condensed branch resolves to the same
// selector hashes the individual boss rows produce, so everything downstream —
// `is_selected_css_boss`, every boss module, Results, respawn — is untouched.
//
// Galleom and WOL Master Hand are reached through per-entry secondary choices
// on Ganon and Master Hand respectively. Giga Bowser remains a real fighter
// with its own CSS entry.
// ---------------------------------------------------------------------------

/// The ten logical bosses reachable from the single carrier. The first eight
/// indices are native colors; WOL Master Hand and Galleom are secondary choices.
/// These are the existing per-boss UI hashes — no new identity is minted, so a
/// condensed selection is indistinguishable downstream from picking the boss's
/// own CSS row.
const CONDENSED_BOSS_ROSTER: [u64; 10] = [
    UI_CHARA_MASTERHAND_HASH,
    UI_CHARA_CRAZYHAND_HASH,
    UI_CHARA_DARZ_HASH,
    UI_CHARA_KIILA_HASH,
    UI_CHARA_GANONBOSS_HASH,
    UI_CHARA_LIOLEUS_HASH,
    UI_CHARA_DRACULA_HASH,
    UI_CHARA_MARX_HASH,
    UI_CHARA_MEWTWO_MASTERHAND_HASH,
    UI_CHARA_GALLEOM_HASH,
];

const CONDENSED_NATIVE_VARIANT_COUNT: u64 = 8;
const CONDENSED_WOL_ALTERNATE_COLOR: u64 = 0;
const CONDENSED_WOL_LOGICAL_INDEX: usize = 8;
const CONDENSED_GALLEOM_ALTERNATE_COLOR: u64 = 4;
const CONDENSED_GALLEOM_LOGICAL_INDEX: usize = 9;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CondensedSecondarySelection {
    None,
    WolMasterHand,
    Galleom,
}

impl CondensedSecondarySelection {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::WolMasterHand => "wol_master_hand",
            Self::Galleom => "galleom",
        }
    }

    const fn signature_tag(self) -> u64 {
        match self {
            Self::None => 0,
            Self::WolMasterHand => 1,
            Self::Galleom => 2,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct CondensedSelectionDecision {
    logical_index: usize,
    boss_hash: u64,
    secondary_override: CondensedSecondarySelection,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct CondensedColorObservation {
    host_work_color: i32,
    fighter_color: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CondensedSelectionFailure {
    FighterInformationUnavailable,
    HostColorOutOfRange,
    FighterColorOutOfRange,
    ColorMismatch,
    LogicalIndexOutOfRange,
    CarrierCallbackUnconfirmed,
}

impl CondensedSelectionFailure {
    const fn name(self) -> &'static str {
        match self {
            Self::FighterInformationUnavailable => "fighter_information_unavailable",
            Self::HostColorOutOfRange => "host_work_color_out_of_range",
            Self::FighterColorOutOfRange => "fighter_information_color_out_of_range",
            Self::ColorMismatch => "host_and_fighter_color_mismatch",
            Self::LogicalIndexOutOfRange => "logical_index_out_of_range",
            Self::CarrierCallbackUnconfirmed => "carrier_callback_unconfirmed",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CondensedSelectorResolution {
    NotCarrier,
    Resolved(u64),
    Unresolved,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct CondensedSelectionLatch {
    host_work_color: i32,
    fighter_color: u64,
    decision: Option<CondensedSelectionDecision>,
    last_status_was_entry: bool,
    initialized: bool,
}

impl CondensedSelectionLatch {
    const EMPTY: Self = Self {
        host_work_color: i32::MIN,
        fighter_color: u64::MAX,
        decision: None,
        last_status_was_entry: false,
        initialized: false,
    };

    fn needs_latch(self, observation: CondensedColorObservation, status_is_entry: bool) -> bool {
        !self.initialized
            || self.host_work_color != observation.host_work_color
            || self.fighter_color != observation.fighter_color
            || (self.decision.is_none() && status_is_entry && !self.last_status_was_entry)
    }

    fn latch(
        &mut self,
        observation: CondensedColorObservation,
        decision: Option<CondensedSelectionDecision>,
        status_is_entry: bool,
    ) {
        self.host_work_color = observation.host_work_color;
        self.fighter_color = observation.fighter_color;
        self.decision = decision;
        self.last_status_was_entry = status_is_entry;
        self.initialized = true;
    }

    fn observe_status(&mut self, status_is_entry: bool) {
        self.last_status_was_entry = status_is_entry;
    }

    fn resolved_hash(self) -> Option<u64> {
        self.decision.map(|decision| decision.boss_hash)
    }

    fn reset(&mut self) {
        *self = Self::EMPTY;
    }
}

static mut CONDENSED_SELECTION_LATCHES: [CondensedSelectionLatch; MAX_FIGHTERS] =
    [CondensedSelectionLatch::EMPTY; MAX_FIGHTERS];

#[inline(always)]
fn condensed_mode_enabled() -> bool {
    CONFIG.options.condense_bosses_into_single_slot()
}

/// Maps a native variant index to its logical boss. Out-of-range fails closed
/// to `None` rather than wrapping — a bad index must never silently select the
/// wrong boss.
#[inline(always)]
fn condensed_boss_for_variant(variant: usize) -> Option<u64> {
    CONDENSED_BOSS_ROSTER.get(variant).copied()
}

/// Resolve the eight stock Mario colors plus the two BOSSES-only alternates.
/// A native color 8 is accepted if the engine ever supplies it safely, but the
/// PRC carrier intentionally exposes only colors 0..7 so Mario never requests
/// an unproven c08 resource.
fn condensed_selection_for_color(
    color: u64,
    secondary_override: CondensedSecondarySelection,
) -> Option<CondensedSelectionDecision> {
    let logical_index = match (color, secondary_override) {
        (color, CondensedSecondarySelection::None)
            if color == CONDENSED_WOL_LOGICAL_INDEX as u64 =>
        {
            CONDENSED_WOL_LOGICAL_INDEX
        }
        (color, CondensedSecondarySelection::None) if color < CONDENSED_NATIVE_VARIANT_COUNT => {
            color as usize
        }
        (CONDENSED_WOL_ALTERNATE_COLOR, CondensedSecondarySelection::WolMasterHand) => {
            CONDENSED_WOL_LOGICAL_INDEX
        }
        (CONDENSED_GALLEOM_ALTERNATE_COLOR, CondensedSecondarySelection::Galleom) => {
            CONDENSED_GALLEOM_LOGICAL_INDEX
        }
        _ => return None,
    };

    condensed_boss_for_variant(logical_index).map(|boss_hash| CondensedSelectionDecision {
        logical_index,
        boss_hash,
        secondary_override,
    })
}

fn condensed_secondary_selection(color: u64, shield_held: bool) -> CondensedSecondarySelection {
    if !shield_held {
        return CondensedSecondarySelection::None;
    }
    match color {
        CONDENSED_GALLEOM_ALTERNATE_COLOR => CondensedSecondarySelection::Galleom,
        CONDENSED_WOL_ALTERNATE_COLOR => CondensedSecondarySelection::WolMasterHand,
        _ => CondensedSecondarySelection::None,
    }
}

#[cfg(test)]
fn condensed_color_has_secondary_choice(color: u64) -> bool {
    matches!(
        color,
        CONDENSED_GALLEOM_ALTERNATE_COLOR | CONDENSED_WOL_ALTERNATE_COLOR
    )
}

fn condensed_selection_decision(
    condensed_enabled: bool,
    carrier_selected: bool,
    color: u64,
    secondary_override: CondensedSecondarySelection,
) -> Option<CondensedSelectionDecision> {
    if !condensed_enabled || !carrier_selected {
        return None;
    }
    condensed_selection_for_color(color, secondary_override)
}

fn confirmed_condensed_color(
    observation: CondensedColorObservation,
) -> Result<u64, CondensedSelectionFailure> {
    let Ok(host_color) = u64::try_from(observation.host_work_color) else {
        return Err(CondensedSelectionFailure::HostColorOutOfRange);
    };
    if host_color > CONDENSED_WOL_LOGICAL_INDEX as u64 {
        return Err(CondensedSelectionFailure::HostColorOutOfRange);
    }
    if observation.fighter_color > CONDENSED_WOL_LOGICAL_INDEX as u64 {
        return Err(CondensedSelectionFailure::FighterColorOutOfRange);
    }
    if host_color != observation.fighter_color {
        return Err(CondensedSelectionFailure::ColorMismatch);
    }
    Ok(host_color)
}

const NPAD_BUTTON_L: u64 = 1 << 6;
const NPAD_BUTTON_R: u64 = 1 << 7;
const NPAD_ID_HANDHELD: u32 = 0x20;
const NPAD_STYLE_FULLKEY: u32 = 0x1;
const NPAD_STYLE_HANDHELD: u32 = 0x2;
const NPAD_STYLE_JOYDUAL: u32 = 0x4;
const NPAD_STYLE_JOYLEFT: u32 = 0x8;
const NPAD_STYLE_JOYRIGHT: u32 = 0x10;
const NPAD_STYLE_GC: u32 = 0x20;
const NPAD_GC_TRIGGER_SHIELD: u32 = 0x80;

#[inline(always)]
fn npad_buttons_include_shoulder(buttons: u64) -> bool {
    buttons & (NPAD_BUTTON_L | NPAD_BUTTON_R) != 0
}

#[inline(always)]
fn gc_triggers_include_shield(l_trigger: u32, r_trigger: u32) -> bool {
    l_trigger >= NPAD_GC_TRIGGER_SHIELD || r_trigger >= NPAD_GC_TRIGGER_SHIELD
}

#[inline(always)]
fn npad_ids_for_entry(entry: usize) -> [Option<u32>; 2] {
    if entry == 0 {
        [Some(0), Some(NPAD_ID_HANDHELD)]
    } else if entry < MAX_FIGHTERS {
        [Some(entry as u32), None]
    } else {
        [None, None]
    }
}

#[inline(always)]
unsafe fn npad_empty_state() -> NpadHandheldState {
    NpadHandheldState {
        updateCount: 0,
        Buttons: 0,
        LStickX: 0,
        LStickY: 0,
        RStickX: 0,
        RStickY: 0,
        Flags: 0,
    }
}

#[inline(always)]
unsafe fn npad_style_buttons(id: u32) -> (u64, u32, u32) {
    let style = GetNpadStyleSet(&id).flags;
    let probe = if style == 0 {
        NPAD_STYLE_FULLKEY | NPAD_STYLE_HANDHELD | NPAD_STYLE_JOYDUAL
    } else {
        style
    };
    let mut state = npad_empty_state();
    let mut buttons = 0u64;
    let mut l_trigger = 0u32;
    let mut r_trigger = 0u32;
    if probe & NPAD_STYLE_FULLKEY != 0 {
        GetNpadFullKeyState(&mut state, &id);
        buttons |= state.Buttons;
    }
    if probe & NPAD_STYLE_HANDHELD != 0 {
        GetNpadHandheldState(&mut state, &id);
        buttons |= state.Buttons;
    }
    if probe & NPAD_STYLE_JOYDUAL != 0 {
        GetNpadJoyDualState(&mut state, &id);
        buttons |= state.Buttons;
    }
    if probe & NPAD_STYLE_JOYLEFT != 0 {
        GetNpadJoyLeftState(&mut state, &id);
        buttons |= state.Buttons;
    }
    if probe & NPAD_STYLE_JOYRIGHT != 0 {
        GetNpadJoyRightState(&mut state, &id);
        buttons |= state.Buttons;
    }
    if probe & NPAD_STYLE_GC != 0 {
        let mut gc = NpadGcState::default();
        GetNpadGcState(&mut gc, &id);
        buttons |= gc.Buttons;
        l_trigger = gc.LTrigger;
        r_trigger = gc.RTrigger;
    }
    (buttons, l_trigger, r_trigger)
}

#[inline(always)]
unsafe fn npad_shield_held(entry: usize) -> bool {
    for id in npad_ids_for_entry(entry).into_iter().flatten() {
        let (buttons, l_trigger, r_trigger) = npad_style_buttons(id);
        if npad_buttons_include_shoulder(buttons)
            || gc_triggers_include_shield(l_trigger, r_trigger)
        {
            return true;
        }
    }
    false
}

#[inline(always)]
unsafe fn condensed_shield_held(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry: usize,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD)
        || npad_shield_held(entry)
}

#[inline(always)]
unsafe fn is_confirmed_condensed_masterhand_carrier(entry_idx: usize) -> bool {
    entry_idx < MAX_FIGHTERS
        && CACHED_BOSS_UI_HASH_BY_ENTRY[entry_idx] == UI_CHARA_MASTERHAND_HASH
        && CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry_idx]
            == OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier
}

/// Resolve a carrier confirmed by the existing per-entry selected-fighter
/// callback. `summon_boss_id` is diagnostic only: it is a battle-object ID and
/// must never be used as the CSS identity or logical boss authority.
#[inline(always)]
unsafe fn condensed_boss_selector_id(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> CondensedSelectorResolution {
    if module_accessor.is_null() {
        return CondensedSelectorResolution::NotCarrier;
    }

    let entry = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if entry < 0 || (entry as usize) >= MAX_FIGHTERS {
        return CondensedSelectorResolution::NotCarrier;
    }

    let entry_idx = entry as usize;
    if !is_confirmed_condensed_masterhand_carrier(entry_idx) {
        return CondensedSelectorResolution::NotCarrier;
    }

    let info = fighter_information_for_entry(entry);
    let host_work_color = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR);
    if info.is_null() {
        log_condensed_selection(
            entry,
            true,
            CondensedColorObservation {
                host_work_color,
                fighter_color: u64::MAX,
            },
            *BATTLE_OBJECT_ID_INVALID as u64,
            None,
            Some(CondensedSelectionFailure::FighterInformationUnavailable),
        );
        return CondensedSelectorResolution::Unresolved;
    }

    let observation = CondensedColorObservation {
        host_work_color,
        fighter_color: smash::app::lua_bind::FighterInformation::fighter_color(info),
    };
    let summon_boss_id = smash::app::lua_bind::FighterInformation::summon_boss_id(info);
    let status = StatusModule::status_kind(module_accessor);
    let status_is_entry = status == *FIGHTER_STATUS_KIND_ENTRY;
    let latch = &mut CONDENSED_SELECTION_LATCHES[entry_idx];

    // L/R are readable during load. Latch immediately: Shield selects the
    // same-slot alternate, otherwise the picked Master Hand / Ganon variant
    // must spawn for entry instead of waiting on a missed chord.
    let needs_latch = latch.needs_latch(observation, status_is_entry);

    if needs_latch {
        let (selection, failure) = match confirmed_condensed_color(observation) {
            Ok(color) => {
                let shield_held =
                    condensed_mode_enabled() && condensed_shield_held(module_accessor, entry_idx);
                let secondary_override = condensed_secondary_selection(color, shield_held);
                let selection = condensed_selection_decision(true, true, color, secondary_override);
                let failure = selection
                    .is_none()
                    .then_some(CondensedSelectionFailure::LogicalIndexOutOfRange);
                (selection, failure)
            }
            Err(failure) => (None, Some(failure)),
        };

        latch.latch(observation, selection, status_is_entry);
        LAST_LOGGED_CONDENSED_SELECTION[entry_idx] = u64::MAX;
        log_condensed_selection(entry, true, observation, summon_boss_id, selection, failure);
    } else {
        latch.observe_status(status_is_entry);
    }

    match latch.resolved_hash() {
        Some(hash) => CondensedSelectorResolution::Resolved(hash),
        None => CondensedSelectorResolution::Unresolved,
    }
}

/// Reset the per-entry secondary-selection latch at a proven scene/match
/// generation boundary. This prevents a secondary chord from surviving into a
/// rematch while preserving the decision throughout the active match.
pub unsafe fn reset_condensed_selection(entry: usize) {
    if entry >= MAX_FIGHTERS {
        return;
    }
    CONDENSED_SELECTION_LATCHES[entry].reset();
    LAST_LOGGED_CONDENSED_SELECTION[entry] = u64::MAX;
}

/// Bounded: logs only when this entry's resolved condensed boss changes.
#[inline(always)]
unsafe fn log_condensed_selection(
    entry: i32,
    carrier_confirmed: bool,
    observation: CondensedColorObservation,
    summon_boss_id: u64,
    selection: Option<CondensedSelectionDecision>,
    failure: Option<CondensedSelectionFailure>,
) {
    if !crate::debug::enabled() || entry < 0 || (entry as usize) >= MAX_FIGHTERS {
        return;
    }
    let idx = entry as usize;
    let resolved = selection.map(|decision| decision.boss_hash);
    let logical_index = selection.map(|decision| decision.logical_index);
    let secondary_override = selection
        .map(|decision| decision.secondary_override)
        .unwrap_or(CondensedSecondarySelection::None);
    let value = resolved.unwrap_or(u64::MAX)
        ^ (logical_index.unwrap_or(usize::MAX) as u64).rotate_left(17)
        ^ (observation.host_work_color as u32 as u64).rotate_left(23)
        ^ observation.fighter_color.rotate_left(29)
        ^ summon_boss_id.rotate_left(41)
        ^ (failure.map(|value| value as u64).unwrap_or(u64::MAX)).rotate_left(11)
        ^ ((carrier_confirmed as u64) << 62)
        ^ secondary_override.signature_tag().rotate_left(61);
    if LAST_LOGGED_CONDENSED_SELECTION[idx] == value {
        return;
    }
    LAST_LOGGED_CONDENSED_SELECTION[idx] = value;
    if let Some(logical_index) = logical_index {
        crate::boss_log!(
            "[PB][CondensedBossSelection] entry={} carrier=master_hand carrier_detected={} css_local_color=unavailable_named_api host_work_color={} fighter_color={} summon_boss_id=0x{:x} logical_index={} resolved_boss={} selector_hash=0x{:010x} color_source=host_work_and_fighter_info secondary_input=shield secondary_override={} wol_secondary_override={} galleom_secondary_override={} fallback_reason=none",
            entry,
            carrier_confirmed,
            observation.host_work_color,
            observation.fighter_color,
            summon_boss_id,
            logical_index,
            resolved.map(css_identity_label).unwrap_or("<out_of_range>"),
            resolved.unwrap_or(0),
            secondary_override.name(),
            secondary_override == CondensedSecondarySelection::WolMasterHand,
            secondary_override == CondensedSecondarySelection::Galleom
        );
    } else {
        crate::boss_log!(
            "[PB][CondensedBossSelection] entry={} carrier=master_hand carrier_detected={} css_local_color=unavailable_named_api host_work_color={} fighter_color={} summon_boss_id=0x{:x} logical_index=unresolved resolved_boss=<unresolved> selector_hash=0x0000000000 color_source=unresolved secondary_input=shield secondary_override=none wol_secondary_override=false galleom_secondary_override=false fallback_reason={}",
            entry,
            carrier_confirmed,
            observation.host_work_color,
            observation.fighter_color,
            summon_boss_id,
            failure
                .map(CondensedSelectionFailure::name)
                .unwrap_or("logical_index_unresolved")
        );
    }
}

unsafe fn log_unconfirmed_condensed_carrier(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() {
        return;
    }
    let entry = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if entry < 0 || (entry as usize) >= MAX_FIGHTERS {
        return;
    }
    let info = fighter_information_for_entry(entry);
    let observation = CondensedColorObservation {
        host_work_color: WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR),
        fighter_color: if info.is_null() {
            u64::MAX
        } else {
            smash::app::lua_bind::FighterInformation::fighter_color(info)
        },
    };
    let summon_boss_id = if info.is_null() {
        *BATTLE_OBJECT_ID_INVALID as u64
    } else {
        smash::app::lua_bind::FighterInformation::summon_boss_id(info)
    };
    log_condensed_selection(
        entry,
        false,
        observation,
        summon_boss_id,
        None,
        Some(CondensedSelectionFailure::CarrierCallbackUnconfirmed),
    );
}

// ---------------------------------------------------------------------------
// Persisted boss selection.
//
// The per-entry selection cache is `static mut`, so a reboot wipes it. On a
// cold launch the game restores a saved fighter whose `fighter_kind` is Mario
// for every item-backed boss, and `summon_boss_id` carries no boss identity —
// so Spirit Board battles and the World of Light map both resolved to Mario
// until the player visited the CSS and repopulated the cache by hand.
//
// Only a selection that actually entered a boss battle is persisted, so
// browsing the CSS without starting a match never overwrites the saved value.
// Every failure path here is silent and non-fatal: if the file is missing,
// unreadable, or malformed the plugin behaves exactly as it did before.
// ---------------------------------------------------------------------------

const PERSISTED_SELECTION_FILE: &str = "last_boss_selection.txt";
static mut PERSISTED_SELECTION_SNAPSHOT: [u64; MAX_FIGHTERS] = [0; MAX_FIGHTERS];

fn persisted_selection_path() -> Option<String> {
    CONFIG_DIR
        .as_ref()
        .map(|dir| format!("{}/{}", dir, PERSISTED_SELECTION_FILE))
}

/// Restores the last battle-confirmed selection for every entry. Called once at
/// plugin load, before any battle can query the resolver. Occupancy is unknown
/// at this point, so every valid line is armed as Restored; consumption later
/// refuses to apply that bootstrap state to an unrelated occupant of the slot.
pub unsafe fn restore_persisted_selections() {
    let Some(path) = persisted_selection_path() else {
        return;
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return;
    };

    let mut restored = 0usize;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((entry_text, hash_text)) = line.split_once('=') else {
            continue;
        };
        let Ok(entry) = entry_text.trim().parse::<usize>() else {
            continue;
        };
        let hash_text = hash_text.trim();
        let hash_text = hash_text.strip_prefix("0x").unwrap_or(hash_text);
        let Ok(hash) = u64::from_str_radix(hash_text, 16) else {
            continue;
        };
        // Migration for stale 3.1.0 files: a persisted Giga Bowser (or any
        // unknown/malformed hash) fails closed -- it is neither armed nor kept
        // in the snapshot, so the next write drops the line automatically and
        // the user never has to delete the file by hand.
        if entry >= MAX_FIGHTERS || !is_persistable_host_boss_hash(hash) {
            continue;
        }
        CACHED_BOSS_UI_HASH_BY_ENTRY[entry] = hash;
        CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[entry] =
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection;
        PERSISTED_SELECTION_SNAPSHOT[entry] = hash;
        restored += 1;
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][PersistRestore] entry={} disk_hash=0x{:010x} cache_ui_hash=0x{:010x} cache_origin=restored_persisted_selection boss={}",
                entry,
                hash,
                hash,
                css_identity_label(hash)
            );
        }
    }

    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][PersistedSelection] action=restore path={} entries_restored={} origin=restored_persisted_selection",
            path,
            restored
        );
    }
}

/// Records the selection for an entry that has just begun a boss battle.
/// Writes only when the value actually changed, so a normal match costs no
/// file I/O after the first time that boss is used.
/// Rewrite the whole file from the in-memory snapshot. Only persistable
/// host-backed identities are ever emitted, so a stale Giga Bowser line from a
/// 3.1.0 install is dropped the first time anything else is written.
unsafe fn write_persisted_selection_file(action: &'static str, entry_id: usize, hash: u64) {
    let Some(path) = persisted_selection_path() else {
        return;
    };
    let mut body = String::from(
        "# Competitive Playable Bosses - last authoritative boss selection per entry.\n",
    );
    for entry in 0..MAX_FIGHTERS {
        let value = PERSISTED_SELECTION_SNAPSHOT[entry];
        if is_persistable_host_boss_hash(value) {
            body.push_str(&format!("{}=0x{:010x}\n", entry, value));
        }
    }
    let wrote = std::fs::write(&path, body).is_ok();
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][PersistedSelection] action={} entry={} boss={} hash=0x{:010x} path={} wrote={}",
            action,
            entry_id,
            css_identity_label(hash),
            hash,
            path,
            wrote
        );
    }
}

/// Record an authoritative selection immediately, without waiting for Ready-Go.
///
/// This is the fix for the stale-file defect: persistence used to be written
/// ONLY at Ready-Go, so "Giga Bowser played a match, user then picked Master
/// Hand but did not start one, reboot" restored the battle-stale Giga Bowser.
/// Idempotent -- an unchanged value performs no disk write.
unsafe fn persist_authoritative_selection(entry_id: usize, hash: u64) {
    if entry_id >= MAX_FIGHTERS
        || !is_persistable_host_boss_hash(hash)
        || PERSISTED_SELECTION_SNAPSHOT[entry_id] == hash
    {
        return;
    }
    PERSISTED_SELECTION_SNAPSHOT[entry_id] = hash;
    write_persisted_selection_file("save_authoritative", entry_id, hash);
}

/// Drop an entry when the user authoritatively picks plain Mario, so the next
/// cold launch does not resurrect a boss they deselected.
unsafe fn clear_persisted_selection(entry_id: usize) {
    if entry_id >= MAX_FIGHTERS || PERSISTED_SELECTION_SNAPSHOT[entry_id] == 0 {
        return;
    }
    PERSISTED_SELECTION_SNAPSHOT[entry_id] = 0;
    write_persisted_selection_file("clear_named_mario", entry_id, 0);
}

/// Final confirmation of the identity that actually entered battle. Retained as
/// a confirmation, no longer the only writer, and idempotent when the
/// authoritative commit already stored the same value.
///
/// Takes the RESOLVED identity rather than the raw cache so a condensed Shield
/// alternate (Master Hand + Shield -> WOL Master Hand, Ganon + Shield ->
/// Galleom) persists the boss that truly loaded, which is only knowable here.
pub unsafe fn persist_selection_for_started_battle(entry_id: usize, resolved_hash: u64) {
    if entry_id >= MAX_FIGHTERS {
        return;
    }
    // Confirmation only. It persists ONLY an identity the entry's own battle
    // resolution positively produced. It must never fall back to the cache,
    // the global menu fallback, or any provisional candidate: doing so replaced
    // a known-good Master Hand on disk with an unrelated Dharkon that had never
    // been selected or spawned. No resolved identity => no write at all.
    if !is_persistable_host_boss_hash(resolved_hash) {
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][PersistedSelection] action=skip_ready_go entry={} reason=no_resolved_battle_identity resolved=0x{:010x} disk_preserved=0x{:010x}",
                entry_id,
                resolved_hash,
                PERSISTED_SELECTION_SNAPSHOT[entry_id]
            );
        }
        return;
    }
    let hash = resolved_hash;
    if PERSISTED_SELECTION_SNAPSHOT[entry_id] == hash {
        return;
    }
    PERSISTED_SELECTION_SNAPSHOT[entry_id] = hash;
    write_persisted_selection_file("save_ready_go", entry_id, hash);
}

static mut SELECTOR_AUTHORITY_LOGGED: [bool; MAX_FIGHTERS] = [false; MAX_FIGHTERS];

/// One bounded, READ-ONLY snapshot of every candidate feeding the battle
/// selector for an entry. Mutates nothing; latched once per entry so it cannot
/// spam. Exists to prove which variable supplies an unexpected identity when a
/// restored selection fails to take over (#89).
pub unsafe fn log_selector_authority(module_accessor: *mut BattleObjectModuleAccessor) {
    if !crate::debug::enabled() || module_accessor.is_null() {
        return;
    }
    let entry = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if entry < 0 || (entry as usize) >= MAX_FIGHTERS {
        return;
    }
    let idx = entry as usize;
    if SELECTOR_AUTHORITY_LOGGED[idx] {
        return;
    }
    SELECTOR_AUTHORITY_LOGGED[idx] = true;

    let info = fighter_information_for_entry(entry);
    let raw_summon = if info.is_null() {
        0
    } else {
        smash::app::lua_bind::FighterInformation::summon_boss_id(info)
    };
    let decoded = decode_tagged_selector_scalar(raw_summon);
    let log_selector = if info.is_null() {
        None
    } else {
        log_int_css_selector_id(info, idx)
    };
    let cache_hash = CACHED_BOSS_UI_HASH_BY_ENTRY[idx];
    let cache_origin = CACHED_BOSS_UI_HASH_ORIGIN_BY_ENTRY[idx];
    let restored_hash = PERSISTED_SELECTION_SNAPSHOT[idx];
    let global_hash = CACHED_BOSS_UI_HASH_GLOBAL;
    let cache_selector = cached_css_boss_hash(module_accessor, idx);
    let name_hash = selected_boss_selector_id_from_character_name(module_accessor);
    let runtime_hash = selected_boss_selector_id_from_runtime_sources(module_accessor);
    let carrier_confirmed = is_confirmed_condensed_masterhand_carrier(idx);
    let chosen = selected_css_boss_selector_id(module_accessor);

    let chosen_reason = selector_choice_reason(
        condensed_mode_enabled(),
        carrier_confirmed,
        runtime_hash,
        name_hash,
        cache_selector,
        cache_origin,
        chosen,
    );

    crate::boss_log!(
        "[PB][SelectorAuthority] entry={} stage=0x{:x} host_kind={} raw_summon_boss_id=0x{:x} decoded_summon_selector={:?} log_selector={:?} current_cache_hash=0x{:010x} current_cache_origin={} cache_selector={:?} restored_hash=0x{:010x} global_hash=0x{:010x} name_detection={:?} runtime_sources={:?} condensed_enabled={} carrier_confirmed={} chosen_hash={:?} chosen_reason={} boss={}",
        idx,
        smash::app::stage::get_stage_id(),
        smash::app::utility::get_kind(&mut *module_accessor),
        raw_summon,
        decoded,
        log_selector,
        cache_hash,
        origin_label(cache_origin),
        cache_selector,
        restored_hash,
        global_hash,
        name_hash,
        runtime_hash,
        condensed_mode_enabled(),
        carrier_confirmed,
        chosen,
        chosen_reason,
        css_identity_label(chosen.unwrap_or(0))
    );
}

fn origin_label(origin: OpaqueSelectionCacheOrigin) -> &'static str {
    match origin {
        OpaqueSelectionCacheOrigin::None => "none",
        OpaqueSelectionCacheOrigin::TentativeUiSelection => "tentative_ui_selection",
        OpaqueSelectionCacheOrigin::CandidateUiLookup => "candidate_ui_lookup",
        OpaqueSelectionCacheOrigin::ConfirmedUiLookup => "confirmed_ui_lookup",
        OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier => "confirmed_condensed_carrier",
        OpaqueSelectionCacheOrigin::RestoredPersistedSelection => "restored_persisted_selection",
    }
}

/// Re-arm the snapshot when match state resets so each battle logs once.
pub unsafe fn reset_selector_authority_log(entry_id: usize) {
    if entry_id < MAX_FIGHTERS {
        SELECTOR_AUTHORITY_LOGGED[entry_id] = false;
    }
}

fn selector_choice_reason(
    condensed_enabled: bool,
    carrier_confirmed: bool,
    live: Option<u64>,
    name: Option<u64>,
    cache: Option<u64>,
    cache_origin: OpaqueSelectionCacheOrigin,
    chosen: Option<u64>,
) -> &'static str {
    if condensed_enabled {
        if carrier_confirmed {
            return "condensed_carrier_resolution";
        }
        if live == Some(UI_CHARA_MASTERHAND_HASH) || name == Some(UI_CHARA_MASTERHAND_HASH) {
            return "not_carrier_master_hand_failed_closed";
        }
        return "not_carrier_passthrough_non_master_hand";
    }
    if chosen.is_some() && chosen == live {
        return "live_fighter_info";
    }
    if chosen.is_some() && chosen == name {
        if cache.is_some() && cache != name {
            return "current_name_outranks_cache";
        }
        return "name_detection";
    }
    if chosen.is_some() && chosen == cache {
        return match cache_origin {
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection => "restored_fallback",
            OpaqueSelectionCacheOrigin::ConfirmedUiLookup => "confirmed_ui_lookup",
            OpaqueSelectionCacheOrigin::ConfirmedCondensedCarrier => "confirmed_condensed_carrier",
            OpaqueSelectionCacheOrigin::CandidateUiLookup => "candidate_ui_lookup",
            _ => "cache_fallback",
        };
    }
    "no_boss_identity"
}

pub unsafe fn selected_css_boss_selector_id(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<u64> {
    if module_accessor.is_null() {
        return None;
    }
    let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    let cache = if entry_id >= 0 && (entry_id as usize) < MAX_FIGHTERS {
        cached_css_boss_hash(module_accessor, entry_id as usize)
    } else {
        None
    };
    let selected = resolve_current_boss_identity(
        selected_boss_selector_id_from_runtime_sources(module_accessor),
        selected_boss_selector_id_from_character_name(module_accessor),
        cache,
    );

    if !condensed_mode_enabled() {
        return selected;
    }

    match condensed_boss_selector_id(module_accessor) {
        CondensedSelectorResolution::Resolved(hash) => Some(hash),
        CondensedSelectorResolution::Unresolved => None,
        CondensedSelectorResolution::NotCarrier => {
            // Config true disambiguates Master Hand as the BOSSES carrier. If
            // legacy identity sees Master Hand but the selected-fighter bridge
            // did not confirm the carrier, fail closed rather than disguising
            // the broken producer as a successful Master Hand selection.
            if selected == Some(UI_CHARA_MASTERHAND_HASH) {
                log_unconfirmed_condensed_carrier(module_accessor);
                None
            } else {
                selected
            }
        }
    }
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

#[allow(dead_code)]
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

#[cfg(test)]
mod persisted_selection_tests {
    use super::*;

    /// Issue #89. The restored persisted selection must survive identity-free
    /// menu traffic but still yield to a genuine Mario pick. Both paths reach
    /// `finish_opaque_selection_candidate` as `Clear`, so they are only
    /// separable by reason -- this locks those two reasons apart.
    #[test]
    fn menu_noise_and_a_real_mario_pick_clear_for_different_reasons() {
        let noise = PendingOpaqueSelection::EMPTY;
        match pending_selection_commit(false, noise) {
            OpaqueSelectionCommit::Clear { reason } => {
                assert_eq!(
                    reason, "no_named_ui_identity",
                    "identity-free menu traffic must stay distinguishable so a \
                     restored persisted selection is not wiped by it"
                );
            }
            other => panic!("identity-free traffic must clear, got {other:?}"),
        }

        let mut mario_pick = PendingOpaqueSelection::EMPTY;
        mario_pick.saw_mario = true;
        match pending_selection_commit(false, mario_pick) {
            OpaqueSelectionCommit::Clear { reason } => {
                assert_eq!(
                    reason, "named_mario_selection",
                    "a real Mario pick must still clear a restored selection"
                );
                assert_ne!(reason, "no_named_ui_identity");
            }
            other => panic!("a named Mario pick must clear, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod condensed_tests {
    use super::*;

    /// The first eight entries are native colors; WOL Master Hand and Galleom
    /// are same-slot secondary choices. Giga Bowser remains separate.
    /// The persisted-selection file is untrusted input: a corrupt or hand-edited
    /// line must be skipped, never applied. Anything that is not a recognised
    /// boss hash for a valid entry is rejected.
    #[test]
    fn persisted_selection_parsing_rejects_untrusted_lines() {
        fn parse(line: &str) -> Option<(usize, u64)> {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (entry_text, hash_text) = line.split_once('=')?;
            let entry = entry_text.trim().parse::<usize>().ok()?;
            let hash_text = hash_text.trim();
            let hash_text = hash_text.strip_prefix("0x").unwrap_or(hash_text);
            let hash = u64::from_str_radix(hash_text, 16).ok()?;
            if entry >= MAX_FIGHTERS || !is_boss_css_hash(hash) {
                return None;
            }
            Some((entry, hash))
        }

        // Valid: a real boss hash for a real entry.
        let good = format!("0=0x{:010x}", UI_CHARA_MASTERHAND_HASH);
        assert_eq!(parse(&good), Some((0, UI_CHARA_MASTERHAND_HASH)));
        assert_eq!(
            parse(&format!("  7 = 0x{:010x}  ", UI_CHARA_MARX_HASH)),
            Some((7, UI_CHARA_MARX_HASH))
        );

        // Rejected: comments, blanks, junk, out-of-range entries, non-boss hashes.
        assert_eq!(parse("# comment"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("garbage"), None);
        assert_eq!(parse("0="), None);
        assert_eq!(parse("notanentry=0x1"), None);
        assert_eq!(
            parse(&format!("8=0x{:010x}", UI_CHARA_MASTERHAND_HASH)),
            None
        );
        assert_eq!(parse(&format!("99=0x{:010x}", UI_CHARA_MARX_HASH)), None);
        assert_eq!(parse("0=0xdeadbeef"), None);
        assert_eq!(parse("0=0x0"), None);
        // The condensed carrier is not a boss identity and must never restore.
        let carrier = crate::to_hash40("ui_chara_playable_bosses").0;
        assert_eq!(parse(&format!("0=0x{:010x}", carrier)), None);
    }

    /// A restored selection is bootstrap fallback, not this-process authority.
    /// Live Confirmed* origins remain the only identities that may write disk.
    #[test]
    fn restored_persisted_origin_is_bootstrap_not_this_session_authority() {
        assert!(!origin_is_authoritative_selection(
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
        ));
        assert!(origin_is_authoritative_selection(
            OpaqueSelectionCacheOrigin::ConfirmedUiLookup
        ));
        assert!(!origin_is_authoritative_selection(
            OpaqueSelectionCacheOrigin::CandidateUiLookup
        ));
        assert!(!origin_is_authoritative_selection(
            OpaqueSelectionCacheOrigin::TentativeUiSelection
        ));
        assert!(!origin_is_authoritative_selection(
            OpaqueSelectionCacheOrigin::None
        ));
        assert_ne!(
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection,
            OpaqueSelectionCacheOrigin::ConfirmedUiLookup
        );
    }

    /// Every roster boss must survive a save/restore round trip.
    #[test]
    fn every_boss_round_trips_through_the_persisted_format() {
        for (entry, hash) in CONDENSED_BOSS_ROSTER.iter().enumerate() {
            let line = format!("{}=0x{:010x}", entry.min(MAX_FIGHTERS - 1), hash);
            let (entry_text, hash_text) = line.split_once('=').unwrap();
            let parsed_entry = entry_text.parse::<usize>().unwrap();
            let parsed_hash =
                u64::from_str_radix(hash_text.strip_prefix("0x").unwrap(), 16).unwrap();
            assert!(parsed_entry < MAX_FIGHTERS);
            assert_eq!(parsed_hash, *hash);
            assert!(is_boss_css_hash(parsed_hash));
        }
    }

    #[test]
    fn condensed_roster_contains_native_and_secondary_bosses() {
        assert_eq!(CONDENSED_BOSS_ROSTER.len(), 10);
        assert_eq!(
            CONDENSED_BOSS_ROSTER,
            [
                UI_CHARA_MASTERHAND_HASH,
                UI_CHARA_CRAZYHAND_HASH,
                UI_CHARA_DARZ_HASH,
                UI_CHARA_KIILA_HASH,
                UI_CHARA_GANONBOSS_HASH,
                UI_CHARA_LIOLEUS_HASH,
                UI_CHARA_DRACULA_HASH,
                UI_CHARA_MARX_HASH,
                UI_CHARA_MEWTWO_MASTERHAND_HASH,
                UI_CHARA_GALLEOM_HASH,
            ]
        );
        assert!(!CONDENSED_BOSS_ROSTER.contains(&UI_CHARA_KOOPAG_HASH));
    }

    /// No boss may occupy two variants: a duplicate would make one selection
    /// unreachable and silently shadow another boss.
    #[test]
    fn condensed_roster_has_no_duplicates() {
        for (i, a) in CONDENSED_BOSS_ROSTER.iter().enumerate() {
            for b in &CONDENSED_BOSS_ROSTER[i + 1..] {
                assert_ne!(a, b, "duplicate condensed roster entry");
            }
        }
    }

    /// Every condensed member must already be a recognised boss identity, so a
    /// condensed selection is indistinguishable downstream from the boss's own
    /// CSS row.
    #[test]
    fn condensed_members_are_existing_boss_identities() {
        for hash in CONDENSED_BOSS_ROSTER {
            assert!(is_boss_css_hash(hash));
        }
    }

    /// Variant index maps 1:1 to roster order, and out-of-range fails closed
    /// rather than wrapping onto the wrong boss.
    #[test]
    fn variant_mapping_is_exact_and_fails_closed() {
        for (index, expected) in CONDENSED_BOSS_ROSTER.iter().enumerate() {
            assert_eq!(condensed_boss_for_variant(index), Some(*expected));
        }
        assert_eq!(condensed_boss_for_variant(10), None);
        assert_eq!(condensed_boss_for_variant(255), None);
        assert_eq!(condensed_boss_for_variant(usize::MAX), None);
    }

    #[test]
    fn selected_fighter_bridge_uses_the_named_master_hand_lookup_as_carrier_authority() {
        let mut pending = PendingOpaqueSelection::begin(0, UI_CHARA_KIILA_HASH);
        // An unrelated lookup must not end the transaction before the selected
        // row's named Master Hand lookup arrives.
        pending.observe_named_lookup(None);
        pending.observe_named_lookup(Some(UI_CHARA_MASTERHAND_HASH));
        pending.observe_named_lookup(Some(UI_CHARA_MARIO_HASH));

        assert_eq!(
            pending_selection_commit(true, pending),
            OpaqueSelectionCommit::Cached {
                ui_hash: UI_CHARA_MASTERHAND_HASH,
                origin: OpaqueSelectionCacheOrigin::CandidateUiLookup,
            }
        );
        assert_eq!(
            pending_selection_commit(false, pending),
            OpaqueSelectionCommit::Cached {
                ui_hash: UI_CHARA_MASTERHAND_HASH,
                origin: OpaqueSelectionCacheOrigin::CandidateUiLookup,
            }
        );
    }

    #[test]
    fn stale_global_hash_never_confirms_the_condensed_carrier() {
        let pending = PendingOpaqueSelection::begin(0, UI_CHARA_MASTERHAND_HASH);
        assert_eq!(
            pending_selection_commit(true, pending),
            OpaqueSelectionCommit::Cached {
                ui_hash: UI_CHARA_MASTERHAND_HASH,
                origin: OpaqueSelectionCacheOrigin::TentativeUiSelection,
            }
        );

        let mut mario = pending;
        mario.observe_named_lookup(Some(UI_CHARA_MARIO_HASH));
        assert_eq!(
            pending_selection_commit(true, mario),
            OpaqueSelectionCommit::Clear {
                reason: "named_mario_selection",
            }
        );
    }

    #[test]
    fn ambiguous_named_boss_lookups_fail_closed() {
        let mut pending = PendingOpaqueSelection::begin(0, 0);
        pending.observe_named_lookup(Some(UI_CHARA_MASTERHAND_HASH));
        pending.observe_named_lookup(Some(UI_CHARA_CRAZYHAND_HASH));
        assert_eq!(
            pending_selection_commit(true, pending),
            OpaqueSelectionCommit::Clear {
                reason: "ambiguous_named_boss_lookups",
            }
        );
    }

    #[test]
    fn tagged_mario_lookup_is_recognized_without_becoming_a_boss_identity() {
        let tagged_mario = 0xc100_0100_0000_0000 | UI_CHARA_MARIO_HASH;
        assert_eq!(
            normalize_known_ui_hash_candidate(tagged_mario),
            Some(UI_CHARA_MARIO_HASH)
        );
        assert_eq!(normalize_ui_hash_candidate(tagged_mario), None);
    }

    #[test]
    fn native_colors_and_secondary_choices_cover_the_complete_roster() {
        for color in 0..CONDENSED_NATIVE_VARIANT_COUNT {
            let expected_index = color as usize;
            assert_eq!(
                condensed_selection_for_color(color, CondensedSecondarySelection::None),
                Some(CondensedSelectionDecision {
                    logical_index: expected_index,
                    boss_hash: CONDENSED_BOSS_ROSTER[expected_index],
                    secondary_override: CondensedSecondarySelection::None,
                })
            );
        }
        assert_eq!(
            condensed_selection_for_color(
                CONDENSED_WOL_ALTERNATE_COLOR,
                CondensedSecondarySelection::WolMasterHand,
            ),
            Some(CondensedSelectionDecision {
                logical_index: CONDENSED_WOL_LOGICAL_INDEX,
                boss_hash: UI_CHARA_MEWTWO_MASTERHAND_HASH,
                secondary_override: CondensedSecondarySelection::WolMasterHand,
            })
        );
        assert_eq!(
            condensed_selection_for_color(
                CONDENSED_GALLEOM_ALTERNATE_COLOR,
                CondensedSecondarySelection::Galleom,
            ),
            Some(CondensedSelectionDecision {
                logical_index: CONDENSED_GALLEOM_LOGICAL_INDEX,
                boss_hash: UI_CHARA_GALLEOM_HASH,
                secondary_override: CondensedSecondarySelection::Galleom,
            })
        );
        assert_eq!(
            condensed_selection_for_color(
                CONDENSED_WOL_LOGICAL_INDEX as u64,
                CondensedSecondarySelection::None,
            ),
            Some(CondensedSelectionDecision {
                logical_index: CONDENSED_WOL_LOGICAL_INDEX,
                boss_hash: UI_CHARA_MEWTWO_MASTERHAND_HASH,
                secondary_override: CondensedSecondarySelection::None,
            })
        );
        assert_eq!(
            condensed_selection_for_color(9, CondensedSecondarySelection::None),
            None
        );
    }

    #[test]
    fn condensed_authority_requires_both_mode_and_carrier() {
        assert_eq!(
            condensed_selection_decision(false, true, 2, CondensedSecondarySelection::None,),
            None
        );
        assert_eq!(
            condensed_selection_decision(
                false,
                true,
                CONDENSED_GALLEOM_ALTERNATE_COLOR,
                CondensedSecondarySelection::Galleom,
            ),
            None
        );
        assert_eq!(
            condensed_selection_decision(true, false, 2, CondensedSecondarySelection::None,),
            None
        );
        assert_eq!(
            condensed_selection_decision(true, true, 2, CondensedSecondarySelection::None),
            Some(CondensedSelectionDecision {
                logical_index: 2,
                boss_hash: UI_CHARA_DARZ_HASH,
                secondary_override: CondensedSecondarySelection::None,
            })
        );
    }

    #[test]
    fn production_color_observation_requires_named_sources_to_agree() {
        for color in [0, 1, 2, 7, 8] {
            assert_eq!(
                confirmed_condensed_color(CondensedColorObservation {
                    host_work_color: color,
                    fighter_color: color as u64,
                }),
                Ok(color as u64)
            );
        }
        assert_eq!(
            confirmed_condensed_color(CondensedColorObservation {
                host_work_color: 0,
                fighter_color: 1,
            }),
            Err(CondensedSelectionFailure::ColorMismatch)
        );
        assert_eq!(
            confirmed_condensed_color(CondensedColorObservation {
                host_work_color: -1,
                fighter_color: 0,
            }),
            Err(CondensedSelectionFailure::HostColorOutOfRange)
        );
        assert_eq!(
            confirmed_condensed_color(CondensedColorObservation {
                host_work_color: 0,
                fighter_color: u64::MAX,
            }),
            Err(CondensedSelectionFailure::FighterColorOutOfRange)
        );
    }

    #[test]
    fn production_color_observations_resolve_master_crazy_rathalos_and_marx() {
        for (color, expected) in [
            (0, UI_CHARA_MASTERHAND_HASH),
            (1, UI_CHARA_CRAZYHAND_HASH),
            (5, UI_CHARA_LIOLEUS_HASH),
            (7, UI_CHARA_MARX_HASH),
        ] {
            let observation = CondensedColorObservation {
                host_work_color: color,
                fighter_color: color as u64,
            };
            let confirmed = confirmed_condensed_color(observation).unwrap();
            assert_eq!(
                condensed_selection_decision(
                    true,
                    true,
                    confirmed,
                    CondensedSecondarySelection::None,
                )
                .map(|decision| decision.boss_hash),
                Some(expected)
            );
        }
    }

    #[test]
    fn wol_secondary_selection_is_latched_until_explicit_reset() {
        let mut latch = CondensedSelectionLatch::EMPTY;
        let observation = CondensedColorObservation {
            host_work_color: CONDENSED_WOL_ALTERNATE_COLOR as i32,
            fighter_color: CONDENSED_WOL_ALTERNATE_COLOR,
        };
        let wol = condensed_selection_for_color(
            CONDENSED_WOL_ALTERNATE_COLOR,
            CondensedSecondarySelection::WolMasterHand,
        );
        latch.latch(observation, wol, false);

        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_MEWTWO_MASTERHAND_HASH));
        assert_eq!(
            latch.decision.unwrap().secondary_override,
            CondensedSecondarySelection::WolMasterHand
        );
        assert!(!latch.needs_latch(observation, true));

        // Releasing Shield and entering the fighter's ENTRY status do not
        // change the stored logical identity.
        latch.observe_status(false);
        assert!(!latch.needs_latch(observation, true));
        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_MEWTWO_MASTERHAND_HASH));

        latch.reset();
        let master_hand = condensed_selection_for_color(
            CONDENSED_WOL_ALTERNATE_COLOR,
            CondensedSecondarySelection::None,
        );
        latch.latch(observation, master_hand, true);
        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_MASTERHAND_HASH));
        assert_eq!(
            latch.decision.unwrap().secondary_override,
            CondensedSecondarySelection::None
        );
    }

    #[test]
    fn npad_shoulder_bits_are_physical_l_and_r() {
        assert_eq!(NPAD_BUTTON_L, 0x40);
        assert_eq!(NPAD_BUTTON_R, 0x80);
        assert!(npad_buttons_include_shoulder(NPAD_BUTTON_L));
        assert!(npad_buttons_include_shoulder(NPAD_BUTTON_R));
        assert!(npad_buttons_include_shoulder(NPAD_BUTTON_L | NPAD_BUTTON_R));
        assert!(!npad_buttons_include_shoulder(0));
        assert!(
            !npad_buttons_include_shoulder(1 << 8),
            "ZL is grab, not shield"
        );
        assert!(gc_triggers_include_shield(NPAD_GC_TRIGGER_SHIELD, 0));
        assert!(gc_triggers_include_shield(0, NPAD_GC_TRIGGER_SHIELD));
        assert!(!gc_triggers_include_shield(0, 0));
        assert_eq!(npad_ids_for_entry(0), [Some(0), Some(NPAD_ID_HANDHELD)]);
        assert_eq!(npad_ids_for_entry(1), [Some(1), None]);
        assert_eq!(npad_ids_for_entry(7), [Some(7), None]);
    }

    #[test]
    fn shield_secondary_selection_is_scoped_to_ganon_and_master_hand() {
        assert_eq!(
            condensed_secondary_selection(CONDENSED_GALLEOM_ALTERNATE_COLOR, true),
            CondensedSecondarySelection::Galleom
        );
        assert_eq!(
            condensed_secondary_selection(CONDENSED_WOL_ALTERNATE_COLOR, true),
            CondensedSecondarySelection::WolMasterHand
        );
        assert_eq!(
            condensed_secondary_selection(2, true),
            CondensedSecondarySelection::None
        );
        assert_eq!(
            condensed_secondary_selection(7, true),
            CondensedSecondarySelection::None,
            "Marx must remain Marx even while Shield is held"
        );
        assert_eq!(
            condensed_selection_for_color(7, CondensedSecondarySelection::None)
                .map(|decision| decision.boss_hash),
            Some(UI_CHARA_MARX_HASH)
        );

        let observation = CondensedColorObservation {
            host_work_color: CONDENSED_GALLEOM_ALTERNATE_COLOR as i32,
            fighter_color: CONDENSED_GALLEOM_ALTERNATE_COLOR,
        };
        let mut latch = CondensedSelectionLatch::EMPTY;
        latch.latch(
            observation,
            condensed_selection_for_color(
                CONDENSED_GALLEOM_ALTERNATE_COLOR,
                CondensedSecondarySelection::Galleom,
            ),
            true,
        );
        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_GALLEOM_HASH));

        latch.observe_status(false);
        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_GALLEOM_HASH));
        latch.reset();
        assert_eq!(latch.resolved_hash(), None);
    }

    #[test]
    fn missed_shield_latches_the_picked_master_hand_or_ganon() {
        assert!(condensed_color_has_secondary_choice(
            CONDENSED_WOL_ALTERNATE_COLOR
        ));
        assert!(condensed_color_has_secondary_choice(
            CONDENSED_GALLEOM_ALTERNATE_COLOR
        ));
        assert!(!condensed_color_has_secondary_choice(2));

        let observation = CondensedColorObservation {
            host_work_color: CONDENSED_WOL_ALTERNATE_COLOR as i32,
            fighter_color: CONDENSED_WOL_ALTERNATE_COLOR,
        };
        let mut latch = CondensedSelectionLatch::EMPTY;
        latch.latch(
            observation,
            condensed_selection_for_color(
                CONDENSED_WOL_ALTERNATE_COLOR,
                CondensedSecondarySelection::None,
            ),
            true,
        );
        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_MASTERHAND_HASH));
        assert_eq!(
            latch.decision.unwrap().secondary_override,
            CondensedSecondarySelection::None
        );

        let ganon = CondensedColorObservation {
            host_work_color: CONDENSED_GALLEOM_ALTERNATE_COLOR as i32,
            fighter_color: CONDENSED_GALLEOM_ALTERNATE_COLOR,
        };
        latch.latch(
            ganon,
            condensed_selection_for_color(
                CONDENSED_GALLEOM_ALTERNATE_COLOR,
                CondensedSecondarySelection::None,
            ),
            true,
        );
        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_GANONBOSS_HASH));
    }

    #[test]
    fn shield_on_first_query_selects_wol_or_galleom() {
        let observation = CondensedColorObservation {
            host_work_color: CONDENSED_WOL_ALTERNATE_COLOR as i32,
            fighter_color: CONDENSED_WOL_ALTERNATE_COLOR,
        };
        let mut latch = CondensedSelectionLatch::EMPTY;
        let secondary = condensed_secondary_selection(CONDENSED_WOL_ALTERNATE_COLOR, true);
        latch.latch(
            observation,
            condensed_selection_for_color(CONDENSED_WOL_ALTERNATE_COLOR, secondary),
            true,
        );
        assert_eq!(latch.resolved_hash(), Some(UI_CHARA_MEWTWO_MASTERHAND_HASH));
    }

    #[test]
    fn shield_alternates_require_condensed_single_slot() {
        assert!(condensed_selection_decision(
            false,
            true,
            CONDENSED_WOL_ALTERNATE_COLOR,
            CondensedSecondarySelection::WolMasterHand,
        )
        .is_none());
        assert!(condensed_selection_decision(
            false,
            true,
            CONDENSED_GALLEOM_ALTERNATE_COLOR,
            CondensedSecondarySelection::Galleom,
        )
        .is_none());
        assert_eq!(
            condensed_selection_decision(
                true,
                true,
                CONDENSED_WOL_ALTERNATE_COLOR,
                CondensedSecondarySelection::WolMasterHand,
            )
            .map(|decision| decision.boss_hash),
            Some(UI_CHARA_MEWTWO_MASTERHAND_HASH)
        );
        assert_eq!(
            condensed_selection_decision(
                true,
                true,
                CONDENSED_GALLEOM_ALTERNATE_COLOR,
                CondensedSecondarySelection::Galleom,
            )
            .map(|decision| decision.boss_hash),
            Some(UI_CHARA_GALLEOM_HASH)
        );
        assert_eq!(
            condensed_selection_decision(
                true,
                true,
                CONDENSED_WOL_ALTERNATE_COLOR,
                CondensedSecondarySelection::None,
            )
            .map(|decision| decision.boss_hash),
            Some(UI_CHARA_MASTERHAND_HASH)
        );
    }

    #[test]
    fn simultaneous_players_keep_independent_condensed_decisions() {
        let mut latches = [CondensedSelectionLatch::EMPTY; 2];
        latches[0].latch(
            CondensedColorObservation {
                host_work_color: 2,
                fighter_color: 2,
            },
            condensed_selection_for_color(2, CondensedSecondarySelection::None),
            true,
        );
        latches[1].latch(
            CondensedColorObservation {
                host_work_color: CONDENSED_WOL_ALTERNATE_COLOR as i32,
                fighter_color: CONDENSED_WOL_ALTERNATE_COLOR,
            },
            condensed_selection_for_color(
                CONDENSED_WOL_ALTERNATE_COLOR,
                CondensedSecondarySelection::WolMasterHand,
            ),
            true,
        );

        assert_eq!(latches[0].resolved_hash(), Some(UI_CHARA_DARZ_HASH));
        assert_eq!(
            latches[1].resolved_hash(),
            Some(UI_CHARA_MEWTWO_MASTERHAND_HASH)
        );

        latches[1].reset();
        assert_eq!(latches[0].resolved_hash(), Some(UI_CHARA_DARZ_HASH));
        assert_eq!(latches[1].resolved_hash(), None);
    }

    #[test]
    fn galleom_and_wol_secondary_choices_remain_entry_local() {
        let mut latches = [CondensedSelectionLatch::EMPTY; 2];
        let galleom_color = CondensedColorObservation {
            host_work_color: CONDENSED_GALLEOM_ALTERNATE_COLOR as i32,
            fighter_color: CONDENSED_GALLEOM_ALTERNATE_COLOR,
        };
        let wol_color = CondensedColorObservation {
            host_work_color: CONDENSED_WOL_ALTERNATE_COLOR as i32,
            fighter_color: CONDENSED_WOL_ALTERNATE_COLOR,
        };
        latches[0].latch(
            galleom_color,
            condensed_selection_for_color(
                CONDENSED_GALLEOM_ALTERNATE_COLOR,
                CondensedSecondarySelection::Galleom,
            ),
            true,
        );
        latches[1].latch(
            wol_color,
            condensed_selection_for_color(
                CONDENSED_WOL_ALTERNATE_COLOR,
                CondensedSecondarySelection::WolMasterHand,
            ),
            true,
        );

        assert_eq!(latches[0].resolved_hash(), Some(UI_CHARA_GALLEOM_HASH));
        assert_eq!(
            latches[1].resolved_hash(),
            Some(UI_CHARA_MEWTWO_MASTERHAND_HASH)
        );
        latches[0].reset();
        assert_eq!(latches[0].resolved_hash(), None);
        assert_eq!(
            latches[1].resolved_hash(),
            Some(UI_CHARA_MEWTWO_MASTERHAND_HASH)
        );
    }

    #[test]
    fn simultaneous_native_variations_resolve_per_entry() {
        let mut latches = [CondensedSelectionLatch::EMPTY; 2];
        let p1 = CondensedColorObservation {
            host_work_color: 2,
            fighter_color: 2,
        };
        let p2 = CondensedColorObservation {
            host_work_color: 7,
            fighter_color: 7,
        };

        latches[0].latch(
            p1,
            condensed_selection_for_color(
                confirmed_condensed_color(p1).unwrap(),
                CondensedSecondarySelection::None,
            ),
            true,
        );
        latches[1].latch(
            p2,
            condensed_selection_for_color(
                confirmed_condensed_color(p2).unwrap(),
                CondensedSecondarySelection::None,
            ),
            true,
        );

        assert_eq!(latches[0].resolved_hash(), Some(UI_CHARA_DARZ_HASH));
        assert_eq!(latches[1].resolved_hash(), Some(UI_CHARA_MARX_HASH));
        latches[0].reset();
        assert_eq!(latches[0].resolved_hash(), None);
        assert_eq!(latches[1].resolved_hash(), Some(UI_CHARA_MARX_HASH));
    }

    #[test]
    fn unobserved_or_disagreeing_color_never_defaults_to_master_hand() {
        for observation in [
            CondensedColorObservation {
                host_work_color: -1,
                fighter_color: 0,
            },
            CondensedColorObservation {
                host_work_color: 0,
                fighter_color: u64::MAX,
            },
            CondensedColorObservation {
                host_work_color: 0,
                fighter_color: 1,
            },
        ] {
            let mut latch = CondensedSelectionLatch::EMPTY;
            let decision = confirmed_condensed_color(observation)
                .ok()
                .and_then(|color| {
                    condensed_selection_for_color(color, CondensedSecondarySelection::None)
                });
            latch.latch(observation, decision, true);
            assert_eq!(latch.resolved_hash(), None);
        }

        assert_eq!(
            condensed_selection_for_color(0, CondensedSecondarySelection::None)
                .map(|decision| decision.boss_hash),
            Some(UI_CHARA_MASTERHAND_HASH),
            "Master Hand is valid only when both named color sources explicitly report zero"
        );
    }

    #[test]
    fn native_color_eight_is_understood_but_not_exposed() {
        assert_eq!(CONDENSED_NATIVE_VARIANT_COUNT, 8);
        assert_eq!(
            condensed_selection_for_color(8, CondensedSecondarySelection::None),
            Some(CondensedSelectionDecision {
                logical_index: CONDENSED_WOL_LOGICAL_INDEX,
                boss_hash: UI_CHARA_MEWTWO_MASTERHAND_HASH,
                secondary_override: CondensedSecondarySelection::None,
            })
        );
        assert_eq!(
            condensed_selection_for_color(9, CondensedSecondarySelection::None),
            None
        );
    }
}

/// Issue #89 regression suite: the persisted selection must track the last
/// AUTHORITATIVE selection, not merely the last boss that reached Ready-Go.
///
/// These drive the real pure decision functions (`pending_selection_commit`,
/// `is_persistable_host_boss_hash`, `origin_is_authoritative_selection`) and
/// model only the cache/disk transitions those decisions produce, since the
/// production writers touch `static mut` state and the filesystem.
#[cfg(test)]
mod persistence_semantics_tests {
    use super::*;

    /// Mirrors `finish_opaque_selection_candidate` + the persistence writers.
    #[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
    struct Model {
        cache: u64,
        origin_authoritative: bool,
        restored_pending: bool,
        disk: u64,
    }

    impl Model {
        fn restore(disk: u64) -> Self {
            // Migration: only persistable host-backed identities are armed.
            if is_persistable_host_boss_hash(disk) {
                Self {
                    cache: disk,
                    origin_authoritative: false,
                    restored_pending: true,
                    disk,
                }
            } else {
                Self {
                    cache: 0,
                    origin_authoritative: false,
                    restored_pending: false,
                    disk: 0,
                }
            }
        }

        fn apply(&mut self, pending: PendingOpaqueSelection, condensed: bool) {
            match pending_selection_commit(condensed, pending) {
                OpaqueSelectionCommit::Cached { ui_hash, origin } => {
                    let existing_origin = if self.restored_pending {
                        OpaqueSelectionCacheOrigin::RestoredPersistedSelection
                    } else if self.origin_authoritative {
                        OpaqueSelectionCacheOrigin::ConfirmedUiLookup
                    } else if is_boss_css_hash(self.cache) {
                        OpaqueSelectionCacheOrigin::TentativeUiSelection
                    } else {
                        OpaqueSelectionCacheOrigin::None
                    };
                    if is_boss_css_hash(self.cache)
                        && origin_authority_rank(origin) < origin_authority_rank(existing_origin)
                    {
                        return;
                    }
                    self.cache = ui_hash;
                    self.restored_pending = false;
                    self.origin_authoritative = origin_is_authoritative_selection(origin);
                    if self.origin_authoritative && is_persistable_host_boss_hash(ui_hash) {
                        self.disk = ui_hash;
                    }
                }
                OpaqueSelectionCommit::Clear { reason } => {
                    if !(self.restored_pending && reason == "no_named_ui_identity") {
                        self.cache = 0;
                        self.origin_authoritative = false;
                    }
                    if reason == "named_mario_selection" {
                        self.disk = 0;
                        self.restored_pending = false;
                    }
                }
            }
        }

        /// Ready-Go confirmation, taking the RESOLVED identity.
        fn ready_go(&mut self, resolved: u64) {
            let hash = if is_persistable_host_boss_hash(resolved) {
                resolved
            } else {
                self.cache
            };
            if is_persistable_host_boss_hash(hash) {
                self.disk = hash;
            }
        }
    }

    fn named(entry: usize, hash: u64) -> PendingOpaqueSelection {
        let mut p = PendingOpaqueSelection::begin(entry, 0);
        p.observe_named_lookup(Some(hash));
        p
    }
    fn mario(entry: usize) -> PendingOpaqueSelection {
        let mut p = PendingOpaqueSelection::begin(entry, 0);
        p.observe_named_lookup(Some(UI_CHARA_MARIO_HASH));
        p
    }
    fn noise(entry: usize) -> PendingOpaqueSelection {
        PendingOpaqueSelection::begin(entry, 0)
    }

    /// TEST 1 - a nested named lookup without corroboration must not persist.
    /// Ready-Go still writes a positively resolved identity.
    #[test]
    fn stale_giga_bowser_is_replaced_by_a_later_named_selection() {
        // Giga Bowser is not even persistable, so the stale line fails closed.
        let mut m = Model::restore(UI_CHARA_KOOPAG_HASH);
        assert_eq!(m.cache, 0, "koopag must never arm a host-backed selection");
        assert_eq!(m.disk, 0, "stale koopag line is dropped on restore");

        m.apply(named(0, UI_CHARA_MASTERHAND_HASH), false);
        assert_eq!(m.disk, 0, "candidate must not write disk");
        m.ready_go(UI_CHARA_MASTERHAND_HASH);
        assert_eq!(m.disk, UI_CHARA_MASTERHAND_HASH);

        // Reboot.
        let after = Model::restore(m.disk);
        assert_eq!(after.cache, UI_CHARA_MASTERHAND_HASH);
    }

    /// TEST 2 - nested named lookup is observational. It must not persist
    /// without independent corroboration.
    #[test]
    fn explicit_selection_persists_without_entering_a_battle() {
        let mut m = Model::restore(UI_CHARA_CRAZYHAND_HASH);
        assert_eq!(m.cache, UI_CHARA_CRAZYHAND_HASH);

        m.apply(named(0, UI_CHARA_MASTERHAND_HASH), false);
        assert_eq!(
            m.disk, UI_CHARA_CRAZYHAND_HASH,
            "candidate must not clobber restored disk"
        );
        assert_eq!(m.cache, UI_CHARA_CRAZYHAND_HASH);
    }

    /// TEST 3 - explicitly picking Mario clears the saved boss.
    #[test]
    fn named_mario_selection_clears_persistence() {
        let mut m = Model::restore(UI_CHARA_MASTERHAND_HASH);
        m.apply(mario(0), false);
        assert_eq!(m.disk, 0);
        assert_eq!(m.cache, 0);
        assert_eq!(Model::restore(m.disk).cache, 0);
    }

    /// TEST 4 - identity-free menu noise must not erase a restored selection.
    #[test]
    fn identity_free_noise_preserves_a_restored_selection() {
        let mut m = Model::restore(UI_CHARA_MASTERHAND_HASH);
        for _ in 0..25 {
            m.apply(noise(0), false);
        }
        assert_eq!(m.cache, UI_CHARA_MASTERHAND_HASH);
        assert_eq!(m.disk, UI_CHARA_MASTERHAND_HASH);
    }

    /// TEST 5 - a nested named lookup without corroboration cannot replace
    /// restored persist on disk or in cache.
    #[test]
    fn authoritative_selection_replaces_a_restored_one() {
        let mut m = Model::restore(UI_CHARA_MASTERHAND_HASH);
        m.apply(named(0, UI_CHARA_CRAZYHAND_HASH), false);
        assert_eq!(m.cache, UI_CHARA_MASTERHAND_HASH);
        assert_eq!(m.disk, UI_CHARA_MASTERHAND_HASH);
        assert!(m.restored_pending, "candidate must not supersede restore");
    }

    /// TEST 6 - Giga Bowser never produces a host-backed restored selection.
    #[test]
    fn giga_bowser_is_never_restored_as_a_host_boss() {
        assert!(!is_persistable_host_boss_hash(UI_CHARA_KOOPAG_HASH));
        let m = Model::restore(UI_CHARA_KOOPAG_HASH);
        assert_eq!(m.cache, 0);
        assert_eq!(m.disk, 0);
        // A live Ready-Go as Giga Bowser must not write him back either.
        let mut m2 = Model::default();
        m2.ready_go(UI_CHARA_KOOPAG_HASH);
        assert_eq!(m2.disk, 0);
    }

    /// TEST 7 - general CSS identity logic still recognises Giga Bowser, so his
    /// normal selection and gameplay are untouched.
    #[test]
    fn giga_bowser_remains_a_recognised_css_identity() {
        assert!(is_boss_css_hash(UI_CHARA_KOOPAG_HASH));
        assert_eq!(css_identity_label(UI_CHARA_KOOPAG_HASH), "giga_bowser");
        // Only persistence excludes him; every other boss is persistable.
        for hash in [
            UI_CHARA_MASTERHAND_HASH,
            UI_CHARA_CRAZYHAND_HASH,
            UI_CHARA_MEWTWO_MASTERHAND_HASH,
            UI_CHARA_KIILA_HASH,
            UI_CHARA_DARZ_HASH,
            UI_CHARA_GANONBOSS_HASH,
            UI_CHARA_LIOLEUS_HASH,
            UI_CHARA_DRACULA_HASH,
            UI_CHARA_MARX_HASH,
            UI_CHARA_GALLEOM_HASH,
        ] {
            assert!(is_boss_css_hash(hash));
            assert!(is_persistable_host_boss_hash(hash), "{hash:#x}");
        }
    }

    /// TEST 8 - entries are independent. Candidate cache stays per-entry;
    /// Ready-Go persist stays per-entry.
    #[test]
    fn per_entry_persistence_is_isolated() {
        let mut e0 = Model::default();
        let mut e1 = Model::default();
        e0.apply(named(0, UI_CHARA_MASTERHAND_HASH), false);
        e1.apply(named(1, UI_CHARA_CRAZYHAND_HASH), false);
        assert_eq!(e0.cache, UI_CHARA_MASTERHAND_HASH);
        assert_eq!(e1.cache, UI_CHARA_CRAZYHAND_HASH);
        assert_eq!(e0.disk, 0, "candidate must not persist");
        assert_eq!(e1.disk, 0, "candidate must not persist");
        e0.ready_go(UI_CHARA_MASTERHAND_HASH);
        e1.ready_go(UI_CHARA_CRAZYHAND_HASH);
        assert_eq!(e0.disk, UI_CHARA_MASTERHAND_HASH);
        assert_eq!(e1.disk, UI_CHARA_CRAZYHAND_HASH);
        e0.apply(mario(0), false);
        assert_eq!(e0.disk, 0);
        assert_eq!(e1.disk, UI_CHARA_CRAZYHAND_HASH, "entry 1 unaffected");
    }

    /// TEST 9 - Ready-Go is an idempotent confirmation of a resolved identity.
    #[test]
    fn ready_go_confirmation_is_idempotent() {
        let mut m = Model::default();
        m.apply(named(0, UI_CHARA_MASTERHAND_HASH), false);
        m.ready_go(UI_CHARA_MASTERHAND_HASH);
        let before = m;
        m.ready_go(UI_CHARA_MASTERHAND_HASH);
        assert_eq!(m, before, "same value must not change any state");
    }

    /// TEST 10 - condensed Shield alternates resolve at load, so Ready-Go
    /// persists the boss that loaded. Nested named lookup is not Confirmed.
    #[test]
    fn condensed_shield_alternates_persist_the_resolved_boss() {
        // Master Hand carrier + Shield -> WOL Master Hand.
        let mut m = Model::default();
        m.apply(named(0, UI_CHARA_MASTERHAND_HASH), true);
        assert_eq!(m.disk, 0, "candidate carrier must not persist");
        m.ready_go(UI_CHARA_MEWTWO_MASTERHAND_HASH);
        assert_eq!(m.disk, UI_CHARA_MEWTWO_MASTERHAND_HASH);

        // Ganon carrier + Shield -> Galleom.
        let mut g = Model::default();
        g.apply(named(0, UI_CHARA_GANONBOSS_HASH), true);
        assert_eq!(g.disk, 0);
        g.ready_go(UI_CHARA_GALLEOM_HASH);
        assert_eq!(g.disk, UI_CHARA_GALLEOM_HASH);
    }

    /// Malformed or unknown persisted hashes fail closed.
    #[test]
    fn unknown_persisted_hashes_fail_closed() {
        for bad in [0u64, 0x1, UI_CHARA_MARIO_HASH, 0xDEAD_BEEF_u64] {
            assert!(!is_persistable_host_boss_hash(bad), "{bad:#x}");
            assert_eq!(Model::restore(bad).cache, 0);
        }
    }
}

/// Regression suite for the #89 battle-consumption failure: a restored identity
/// must survive generic menu enumeration and reach the battle resolver, and the
/// Ready-Go confirmation must never guess.
#[cfg(test)]
mod restore_consumption_tests {
    use super::*;

    /// Models the cache exactly as `finish_opaque_selection_candidate` now does,
    /// including the authority-rank guard (weaker origins cannot clobber),
    /// plus `cached_css_boss_hash`'s origin gating and the Ready-Go contract.
    #[derive(Clone, Copy, Debug)]
    struct Entry {
        hash: u64,
        origin: OpaqueSelectionCacheOrigin,
        disk: u64,
    }

    impl Entry {
        fn restored(hash: u64) -> Self {
            Self {
                hash,
                origin: OpaqueSelectionCacheOrigin::RestoredPersistedSelection,
                disk: hash,
            }
        }
        fn confirmed(hash: u64) -> Self {
            Self {
                hash,
                origin: OpaqueSelectionCacheOrigin::ConfirmedUiLookup,
                disk: hash,
            }
        }
        fn empty() -> Self {
            Self {
                hash: 0,
                origin: OpaqueSelectionCacheOrigin::None,
                disk: 0,
            }
        }

        fn commit(&mut self, pending: PendingOpaqueSelection, condensed: bool) {
            match pending_selection_commit(condensed, pending) {
                OpaqueSelectionCommit::Cached { ui_hash, origin } => {
                    let has_existing = is_boss_css_hash(self.hash);
                    if has_existing
                        && origin_authority_rank(origin) < origin_authority_rank(self.origin)
                    {
                        return; // weaker source may not clobber
                    }
                    self.hash = ui_hash;
                    self.origin = origin;
                    if origin_is_authoritative_selection(origin)
                        && is_persistable_host_boss_hash(ui_hash)
                    {
                        self.disk = ui_hash;
                    }
                }
                OpaqueSelectionCommit::Clear { reason } => {
                    let restored_pending =
                        self.origin == OpaqueSelectionCacheOrigin::RestoredPersistedSelection;
                    if !(restored_pending && reason == "no_named_ui_identity") {
                        self.hash = 0;
                        self.origin = OpaqueSelectionCacheOrigin::None;
                    }
                    if reason == "named_mario_selection" {
                        self.disk = 0;
                    }
                }
            }
        }

        /// Mirror of `cached_css_boss_hash` on an ordinary battle stage.
        fn cache_selector_on_battle_stage(&self, operation_cpu: bool) -> Option<u64> {
            if !is_boss_css_hash(self.hash) {
                return None;
            }
            if cache_visible_on_battle_stage(self.origin, operation_cpu) {
                Some(self.hash)
            } else {
                None
            }
        }

        /// Ready-Go confirmation: writes only a positively resolved identity.
        fn ready_go(&mut self, resolved: u64) {
            if !is_persistable_host_boss_hash(resolved) {
                return;
            }
            self.disk = resolved;
        }
    }

    /// The global fallback that enumerating players 4..7 leaves behind.
    fn global_fallback(entry: usize, global: u64) -> PendingOpaqueSelection {
        PendingOpaqueSelection::begin(entry, global)
    }
    fn named(entry: usize, hash: u64) -> PendingOpaqueSelection {
        let mut p = PendingOpaqueSelection::begin(entry, 0);
        p.observe_named_lookup(Some(hash));
        p
    }

    /// TEST 1 - a restored identity must be visible to the human player's
    /// battle resolver, and must not be visible to a CPU occupant of the slot.
    #[test]
    fn restored_identity_is_consumed_by_the_battle_resolver() {
        let e = Entry::restored(UI_CHARA_MASTERHAND_HASH);
        assert_eq!(
            e.cache_selector_on_battle_stage(false),
            Some(UI_CHARA_MASTERHAND_HASH),
            "restored Master Hand must reach the human player's resolver"
        );
        assert_eq!(
            e.cache_selector_on_battle_stage(true),
            None,
            "a CPU occupant must not inherit restored persist for this slot"
        );
    }

    /// TEST 2 - the 0x50000000 sentinel is not a valid boss selector, so it must
    /// not outrank the restored identity.
    #[test]
    fn sentinel_selector_is_not_a_valid_boss_selector() {
        const SENTINEL: u64 = 0x5000_0000;
        assert_eq!(decode_tagged_selector_scalar(SENTINEL), Some(0));
        assert!(!is_boss_selector_id(0));
        assert!(
            !is_known_boss_selector_value(SENTINEL),
            "0x50000000 decodes to 0 and must never count as a boss selector"
        );
        // With no valid raw selector, the restored identity supplies the answer.
        let e = Entry::restored(UI_CHARA_MASTERHAND_HASH);
        assert_eq!(
            e.cache_selector_on_battle_stage(false),
            Some(UI_CHARA_MASTERHAND_HASH)
        );
    }

    /// TEST 3 - THE HARDWARE BUG: generic enumeration leaves Dharkon in the
    /// global fallback; entry 0 must not adopt it.
    #[test]
    fn generic_enumeration_cannot_overwrite_a_restored_selection() {
        let mut e = Entry::restored(UI_CHARA_MASTERHAND_HASH);
        for _ in 0..8 {
            e.commit(global_fallback(0, UI_CHARA_DARZ_HASH), false);
        }
        assert_eq!(e.hash, UI_CHARA_MASTERHAND_HASH, "Dharkon must not clobber");
        assert_eq!(
            e.origin,
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
        );
        assert_eq!(
            e.cache_selector_on_battle_stage(false),
            Some(UI_CHARA_MASTERHAND_HASH),
            "resolver must still see Master Hand, not cache_selector=None"
        );
    }

    /// TEST 4 - Ready-Go must not guess when nothing resolved.
    #[test]
    fn ready_go_never_persists_an_unresolved_identity() {
        let mut e = Entry::restored(UI_CHARA_MASTERHAND_HASH);
        e.commit(global_fallback(0, UI_CHARA_DARZ_HASH), false); // menu pollution
        e.ready_go(0); // battle resolved nothing
        assert_eq!(e.disk, UI_CHARA_MASTERHAND_HASH, "disk must be preserved");
        e.ready_go(0x5000_0000); // sentinel
        assert_eq!(e.disk, UI_CHARA_MASTERHAND_HASH);
    }

    /// TEST 5 - a genuine resolution still confirms (Shield alternate).
    #[test]
    fn ready_go_confirms_a_positively_resolved_identity() {
        let mut e = Entry::restored(UI_CHARA_MASTERHAND_HASH);
        e.ready_go(UI_CHARA_MEWTWO_MASTERHAND_HASH);
        assert_eq!(e.disk, UI_CHARA_MEWTWO_MASTERHAND_HASH);
    }

    /// TEST 6 - entries stay isolated while other players enumerate.
    #[test]
    fn cross_entry_enumeration_is_isolated() {
        let mut e0 = Entry::restored(UI_CHARA_MASTERHAND_HASH);
        let mut e4 = Entry::empty();
        e4.commit(named(4, UI_CHARA_DARZ_HASH), false);
        e0.commit(global_fallback(0, UI_CHARA_DARZ_HASH), false);
        assert_eq!(e0.hash, UI_CHARA_MASTERHAND_HASH);
        assert_eq!(e0.disk, UI_CHARA_MASTERHAND_HASH);
        assert_eq!(e4.hash, UI_CHARA_DARZ_HASH, "entry 4 keeps its own choice");
        assert_eq!(e4.origin, OpaqueSelectionCacheOrigin::CandidateUiLookup);
        assert_eq!(e4.disk, 0);
        assert_eq!(e4.cache_selector_on_battle_stage(true), None);
    }

    /// TEST 7 - a nested named lookup without corroboration cannot outrank
    /// a restore. That was the automatic Rathalos/MH false confirm.
    #[test]
    fn authoritative_selection_still_outranks_restore() {
        let mut e = Entry::restored(UI_CHARA_CRAZYHAND_HASH);
        e.commit(named(0, UI_CHARA_MASTERHAND_HASH), false);
        assert_eq!(e.hash, UI_CHARA_CRAZYHAND_HASH);
        assert_eq!(
            e.origin,
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
        );
        assert_eq!(e.disk, UI_CHARA_CRAZYHAND_HASH);
    }

    /// Same-hash nested lookup is still only a candidate. Restored stays
    /// Restored; CPU cannot consume it.
    #[test]
    fn same_hash_named_lookup_does_not_promote_restored() {
        let mut e = Entry::restored(UI_CHARA_DARZ_HASH);
        e.commit(named(1, UI_CHARA_DARZ_HASH), false);
        assert_eq!(e.hash, UI_CHARA_DARZ_HASH);
        assert_eq!(
            e.origin,
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
        );
        assert_eq!(e.disk, UI_CHARA_DARZ_HASH);
        assert_eq!(e.cache_selector_on_battle_stage(true), None);
    }

    /// Ambient Dracula in the global, then an owned txn with no nested named
    /// lookup: Tentative only. Must not Confirm or write disk.
    #[test]
    fn ambient_fallback_without_in_txn_observe_is_not_confirmed() {
        let pending = PendingOpaqueSelection::begin(0, UI_CHARA_DRACULA_HASH);
        assert_eq!(pending.observed_boss_hash, 0);
        assert_eq!(pending.observed_at_sequence, 0);
        assert_eq!(pending.observation_count, 0);
        assert_eq!(
            pending_selection_commit(false, pending),
            OpaqueSelectionCommit::Cached {
                ui_hash: UI_CHARA_DRACULA_HASH,
                origin: OpaqueSelectionCacheOrigin::TentativeUiSelection,
            }
        );
        let mut e = Entry::restored(UI_CHARA_MEWTWO_MASTERHAND_HASH);
        e.commit(pending, false);
        assert_eq!(e.hash, UI_CHARA_MEWTWO_MASTERHAND_HASH);
        assert_eq!(
            e.origin,
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
        );
        assert_eq!(e.disk, UI_CHARA_MEWTWO_MASTERHAND_HASH);
    }

    /// Copying a pre-txn hash into `observed_boss_hash` without an in-txn
    /// sequence stamp must not Confirm. That is the fallback-upgrade hole.
    #[test]
    fn unstamped_observed_hash_cannot_confirm() {
        let mut leaked = PendingOpaqueSelection::begin(0, UI_CHARA_DRACULA_HASH);
        leaked.observed_boss_hash = UI_CHARA_DRACULA_HASH;
        assert_eq!(leaked.observed_at_sequence, 0);
        assert!(!named_observation_belongs_to_this_transaction(&leaked));
        assert_eq!(
            pending_selection_commit(false, leaked),
            OpaqueSelectionCommit::Cached {
                ui_hash: UI_CHARA_DRACULA_HASH,
                origin: OpaqueSelectionCacheOrigin::TentativeUiSelection,
            }
        );
    }

    /// Global-fallback Tentative repeating the restored hash is ambient, not
    /// a named selected-fighter commit. Rank keeps Restored; CPU still cannot
    /// consume it.
    #[test]
    fn same_hash_tentative_fallback_does_not_promote_restored() {
        let mut e = Entry::restored(UI_CHARA_DARZ_HASH);
        e.commit(global_fallback(1, UI_CHARA_DARZ_HASH), false);
        assert_eq!(e.hash, UI_CHARA_DARZ_HASH);
        assert_eq!(
            e.origin,
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
        );
        assert_eq!(e.cache_selector_on_battle_stage(true), None);
    }

    /// A different nested named identity is still Candidate, not Confirmed.
    /// Restored wins on rank; CPU still cannot consume it.
    #[test]
    fn different_hash_named_lookup_does_not_confirm_over_restore() {
        let mut e = Entry::restored(UI_CHARA_DARZ_HASH);
        e.commit(named(1, UI_CHARA_DRACULA_HASH), false);
        assert_eq!(e.hash, UI_CHARA_DARZ_HASH);
        assert_eq!(
            e.origin,
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
        );
        assert_eq!(e.disk, UI_CHARA_DARZ_HASH);
        assert_eq!(e.cache_selector_on_battle_stage(true), None);
    }

    #[test]
    fn confirmed_cpu_boss_is_visible_on_battle_stage() {
        let e = Entry::confirmed(UI_CHARA_DARZ_HASH);
        assert_eq!(
            e.cache_selector_on_battle_stage(true),
            Some(UI_CHARA_DARZ_HASH)
        );
    }

    /// Hardware 2026-08-17: nested Rathalos in an empty CPU slot must stay
    /// Candidate, must not persist, and must not spawn on that CPU.
    #[test]
    fn nested_named_lookup_on_empty_cpu_slot_is_candidate_only() {
        let mut e = Entry::empty();
        e.commit(named(1, UI_CHARA_LIOLEUS_HASH), false);
        assert_eq!(e.hash, UI_CHARA_LIOLEUS_HASH);
        assert_eq!(e.origin, OpaqueSelectionCacheOrigin::CandidateUiLookup);
        assert_eq!(e.disk, 0);
        assert_eq!(e.cache_selector_on_battle_stage(true), None);
        assert_eq!(e.cache_selector_on_battle_stage(false), None);
    }

    /// Authority ordering is total and correctly ranked.
    #[test]
    fn authority_ranks_are_ordered() {
        use OpaqueSelectionCacheOrigin::*;
        assert!(
            origin_authority_rank(ConfirmedUiLookup)
                > origin_authority_rank(RestoredPersistedSelection)
        );
        assert!(
            origin_authority_rank(ConfirmedCondensedCarrier)
                > origin_authority_rank(RestoredPersistedSelection)
        );
        assert!(
            origin_authority_rank(RestoredPersistedSelection)
                > origin_authority_rank(TentativeUiSelection)
        );
        assert_eq!(
            origin_authority_rank(CandidateUiLookup),
            origin_authority_rank(TentativeUiSelection)
        );
        assert!(origin_authority_rank(TentativeUiSelection) > origin_authority_rank(None));
    }
}

/// Hardware 2026-08-17: last_boss_selection.txt was
/// `0=WOL Master Hand, 1=Dharkon, 2=Rathalos, 3=Marx`. Selecting Dracula vs a
/// Spirit CPU Mario spawned WOL vs Dharkon because restored persist was folded
/// into runtime sources (outranking live name detection) and then consumed by
/// whoever occupied the raw entry index.
#[cfg(test)]
mod persisted_slot_contamination_tests {
    use super::*;

    const FIXTURE_ENTRY0_WOL: u64 = 0x1AA4AF9031;
    const FIXTURE_ENTRY1_DHARKON: u64 = 0x0D65ACCD76;
    const FIXTURE_ENTRY2_RATHALOS: u64 = 0x10E9EFB8D1;
    const FIXTURE_ENTRY3_MARX: u64 = 0x0DF6AAE3D0;
    const LIVE_DRACULA: u64 = 0x1020DDD1F9;
    const SENTINEL: u64 = 0x5000_0000;

    fn human_restored(hash: u64) -> Option<u64> {
        cache_visible_on_battle_stage(
            OpaqueSelectionCacheOrigin::RestoredPersistedSelection,
            false,
        )
        .then_some(hash)
    }

    fn cpu_restored(hash: u64) -> Option<u64> {
        cache_visible_on_battle_stage(OpaqueSelectionCacheOrigin::RestoredPersistedSelection, true)
            .then_some(hash)
    }

    #[test]
    fn fixture_matches_the_hardware_persist_file() {
        assert_eq!(FIXTURE_ENTRY0_WOL, UI_CHARA_MEWTWO_MASTERHAND_HASH);
        assert_eq!(FIXTURE_ENTRY1_DHARKON, UI_CHARA_DARZ_HASH);
        assert_eq!(FIXTURE_ENTRY2_RATHALOS, UI_CHARA_LIOLEUS_HASH);
        assert_eq!(FIXTURE_ENTRY3_MARX, UI_CHARA_MARX_HASH);
        assert_eq!(LIVE_DRACULA, UI_CHARA_DRACULA_HASH);
        assert_eq!(LIVE_DRACULA, 69270884857);
    }

    #[test]
    fn live_dracula_name_outranks_restored_wol() {
        let cache = human_restored(FIXTURE_ENTRY0_WOL);
        assert_eq!(cache, Some(UI_CHARA_MEWTWO_MASTERHAND_HASH));
        assert_eq!(
            resolve_current_boss_identity(None, Some(LIVE_DRACULA), cache),
            Some(UI_CHARA_DRACULA_HASH)
        );
        assert_eq!(
            selector_choice_reason(
                false,
                false,
                None,
                Some(LIVE_DRACULA),
                cache,
                OpaqueSelectionCacheOrigin::RestoredPersistedSelection,
                Some(LIVE_DRACULA),
            ),
            "current_name_outranks_cache"
        );
    }

    #[test]
    fn cpu_mario_does_not_consume_restored_dharkon() {
        assert_eq!(cpu_restored(FIXTURE_ENTRY1_DHARKON), None);
        assert_eq!(resolve_current_boss_identity(None, None, None), None);
        assert_eq!(
            selector_choice_reason(
                false,
                false,
                None,
                None,
                None,
                OpaqueSelectionCacheOrigin::RestoredPersistedSelection,
                None,
            ),
            "no_boss_identity"
        );
    }

    #[test]
    fn human_cold_launch_still_restores_the_player_boss() {
        let cache = human_restored(FIXTURE_ENTRY0_WOL);
        assert_eq!(
            resolve_current_boss_identity(None, None, cache),
            Some(UI_CHARA_MEWTWO_MASTERHAND_HASH)
        );
        assert_eq!(
            selector_choice_reason(
                false,
                false,
                None,
                None,
                cache,
                OpaqueSelectionCacheOrigin::RestoredPersistedSelection,
                cache,
            ),
            "restored_fallback"
        );
    }

    #[test]
    fn confirmed_cpu_dharkon_still_applies() {
        assert!(cache_visible_on_battle_stage(
            OpaqueSelectionCacheOrigin::ConfirmedUiLookup,
            true
        ));
        assert_eq!(
            resolve_current_boss_identity(None, None, Some(FIXTURE_ENTRY1_DHARKON)),
            Some(UI_CHARA_DARZ_HASH)
        );
        assert_eq!(
            selector_choice_reason(
                false,
                false,
                None,
                None,
                Some(FIXTURE_ENTRY1_DHARKON),
                OpaqueSelectionCacheOrigin::ConfirmedUiLookup,
                Some(FIXTURE_ENTRY1_DHARKON),
            ),
            "confirmed_ui_lookup"
        );
    }

    #[test]
    fn cpu_slots_two_and_three_do_not_inject_rathalos_or_marx() {
        for hash in [FIXTURE_ENTRY2_RATHALOS, FIXTURE_ENTRY3_MARX] {
            assert_eq!(cpu_restored(hash), None, "{hash:#x}");
        }
        assert_eq!(resolve_current_boss_identity(None, None, None), None);
    }

    #[test]
    fn summon_sentinel_is_absence_of_live_evidence() {
        assert_eq!(resolved_live_boss_hash(SENTINEL), None);
        assert_eq!(resolved_live_boss_hash(0), None);
        assert_eq!(resolved_live_boss_hash(UI_CHARA_MARIO_HASH), None);
        assert_eq!(
            resolved_live_boss_hash(UI_CHARA_DRACULA_HASH),
            Some(UI_CHARA_DRACULA_HASH)
        );
        assert_eq!(
            resolve_current_boss_identity(
                resolved_live_boss_hash(SENTINEL),
                Some(LIVE_DRACULA),
                Some(FIXTURE_ENTRY0_WOL)
            ),
            Some(UI_CHARA_DRACULA_HASH)
        );
    }

    #[test]
    fn named_unique_lookup_is_candidate_not_confirmed() {
        let mut same = PendingOpaqueSelection::begin(1, FIXTURE_ENTRY1_DHARKON);
        same.observe_named_lookup(Some(FIXTURE_ENTRY1_DHARKON));
        assert_eq!(
            pending_selection_commit(false, same),
            OpaqueSelectionCommit::Cached {
                ui_hash: FIXTURE_ENTRY1_DHARKON,
                origin: OpaqueSelectionCacheOrigin::CandidateUiLookup,
            }
        );
        let mut different = PendingOpaqueSelection::begin(1, 0);
        different.observe_named_lookup(Some(LIVE_DRACULA));
        assert_eq!(
            pending_selection_commit(false, different),
            OpaqueSelectionCommit::Cached {
                ui_hash: LIVE_DRACULA,
                origin: OpaqueSelectionCacheOrigin::CandidateUiLookup,
            }
        );
        let tentative = PendingOpaqueSelection::begin(1, FIXTURE_ENTRY1_DHARKON);
        assert_eq!(
            pending_selection_commit(false, tentative),
            OpaqueSelectionCommit::Cached {
                ui_hash: FIXTURE_ENTRY1_DHARKON,
                origin: OpaqueSelectionCacheOrigin::TentativeUiSelection,
            }
        );
        assert!(!independent_selection_corroborated());
        assert!(!origin_is_authoritative_selection(
            OpaqueSelectionCacheOrigin::CandidateUiLookup
        ));
        assert!(
            origin_authority_rank(OpaqueSelectionCacheOrigin::TentativeUiSelection)
                < origin_authority_rank(OpaqueSelectionCacheOrigin::RestoredPersistedSelection)
        );
        assert!(
            origin_authority_rank(OpaqueSelectionCacheOrigin::ConfirmedUiLookup)
                > origin_authority_rank(OpaqueSelectionCacheOrigin::RestoredPersistedSelection)
        );
    }

    /// Hardware 2026-08-17: startup sweep nested Rathalos in owned entry 1
    /// and Confirmed it onto a Spirit CPU Mario. Candidate must not persist
    /// or apply to that CPU.
    #[test]
    fn startup_nested_rathalos_on_empty_cpu_slot_is_candidate_only() {
        let mut pending = PendingOpaqueSelection::begin(1, 0);
        pending.observe_named_lookup(Some(FIXTURE_ENTRY2_RATHALOS));
        assert_eq!(
            pending_selection_commit(false, pending),
            OpaqueSelectionCommit::Cached {
                ui_hash: FIXTURE_ENTRY2_RATHALOS,
                origin: OpaqueSelectionCacheOrigin::CandidateUiLookup,
            }
        );
        assert!(!origin_is_authoritative_selection(
            OpaqueSelectionCacheOrigin::CandidateUiLookup
        ));
        assert!(!cache_visible_on_battle_stage(
            OpaqueSelectionCacheOrigin::CandidateUiLookup,
            true
        ));
        assert!(!cache_visible_on_battle_stage(
            OpaqueSelectionCacheOrigin::CandidateUiLookup,
            false
        ));
        assert_eq!(resolve_current_boss_identity(None, None, None), None);
    }

    /// Hardware 2026-08-17: startup sweep nested Master Hand in owned entry 2
    /// over Restored Rathalos and saved it. Candidate must not overwrite or
    /// persist.
    #[test]
    fn startup_nested_master_hand_cannot_overwrite_restored() {
        let mut pending = PendingOpaqueSelection::begin(2, 0);
        pending.observe_named_lookup(Some(UI_CHARA_MASTERHAND_HASH));
        match pending_selection_commit(false, pending) {
            OpaqueSelectionCommit::Cached { ui_hash, origin } => {
                assert_eq!(ui_hash, UI_CHARA_MASTERHAND_HASH);
                assert_eq!(origin, OpaqueSelectionCacheOrigin::CandidateUiLookup);
                assert!(
                    origin_authority_rank(origin)
                        < origin_authority_rank(
                            OpaqueSelectionCacheOrigin::RestoredPersistedSelection
                        )
                );
                assert!(!origin_is_authoritative_selection(origin));
            }
            other => panic!("expected candidate, got {other:?}"),
        }
        assert_eq!(cpu_restored(FIXTURE_ENTRY2_RATHALOS), None);
    }

    #[test]
    fn cache_must_not_be_folded_into_live_runtime() {
        // Production runtime sources return None for the sentinel. Name then
        // beats restored WOL. Folding cache into live was the hardware miss.
        assert_eq!(
            resolve_current_boss_identity(None, Some(LIVE_DRACULA), Some(FIXTURE_ENTRY0_WOL)),
            Some(UI_CHARA_DRACULA_HASH)
        );
    }
}


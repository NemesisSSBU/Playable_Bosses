#![feature(proc_macro_hygiene)]

use arcropolis_api::*;
use prc::hash40::Hash40;
use prc::*;
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, Main};

mod ai_diagnostics;
mod amiibo;
mod amiibo_preview;
mod boss_helpers;
mod boss_runtime;
mod boss_summon;
mod config;
mod debug;
mod dharkon;
mod dracula;
mod galeem;
mod galleom;
mod ganon;
mod gigabowser;
mod marx;
mod mastercrazy;
mod playable_masterhand;
mod rathalos;
mod result_camera;
mod selection;

use crate::config::CONFIG;

const MAX_FIGHTERS: usize = 8;
static mut BOSS_MATCH_STARTED: [bool; 8] = [false; 8];
static mut BOSS_HAD_READY_GO: [bool; 8] = [false; 8];
static mut MARIO_READY_GO_LOGGED: [bool; 8] = [false; 8];
static mut POST_MATCH_PRE_RESULT: [bool; 8] = [false; 8];
static mut RESULT_MODE_SEEN: [bool; 8] = [false; 8];
static mut POST_MATCH_TRACKING_INVALIDATED: [bool; 8] = [false; 8];
static mut TRANSITION_DEBUG_LAST_STAGE: [i32; 8] = [-1; 8];
static mut TRANSITION_DEBUG_LAST_STATUS: [i32; 8] = [i32::MIN; 8];
static mut TRANSITION_DEBUG_LAST_FLAGS: [u16; 8] = [u16::MAX; 8];
static mut TRANSITION_DEBUG_LAST_HAVE_ITEM: [i32; 8] = [i32::MIN; 8];
static mut TRANSITION_DEBUG_LAST_SCALE_BITS: [u32; 8] = [u32::MAX; 8];
static mut TRANSITION_DEBUG_LAST_HOST_KIND: [i32; 8] = [i32::MIN; 8];
static mut TRANSITION_DEBUG_LAST_BOSS_KIND: [i32; 8] = [i32::MIN; 8];
static mut TRANSITION_DEBUG_LAST_SELECTED_UI_HASH: [u64; 8] = [u64::MAX; 8];
static mut TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut PRESENTATION_DEBUG_LAST_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut RESULT_TRANSITION_LAST_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut RESULT_BISECT_ACTIVE: [bool; 8] = [false; 8];
static mut RESULT_BISECT_TICKS: [u16; 8] = [0; 8];
static mut RESULT_BISECT_ALIVE_LOGGED: [bool; 8] = [false; 8];
static mut BOSS_LIFECYCLE_GENERATION: [u32; 8] = [0; 8];
static mut BOSS_LIFECYCLE_PHASE: [u8; 8] = [0; 8];
static mut BOSS_LIFECYCLE_LAST_SIGNATURE: [u64; 8] = [u64::MAX; 8];

const LIFECYCLE_PHASE_PRE_MATCH: u8 = 1;
const LIFECYCLE_PHASE_BATTLE: u8 = 2;
const LIFECYCLE_PHASE_POST_MATCH_PRE_RESULT: u8 = 3;
const LIFECYCLE_PHASE_RESULT_READY: u8 = 4;
const LIFECYCLE_PHASE_SCENE_EXIT: u8 = 5;

#[derive(Copy, Clone, PartialEq, Eq)]
enum BossTransitionPhase {
    NotApplicable,
    Battle,
    PostMatchPreResult,
    ResultReady,
    SceneExit,
}

#[inline(always)]
fn lifecycle_phase_name(phase: u8) -> &'static str {
    match phase {
        LIFECYCLE_PHASE_PRE_MATCH => "pre_match",
        LIFECYCLE_PHASE_BATTLE => "battle",
        LIFECYCLE_PHASE_POST_MATCH_PRE_RESULT => "post_match_pre_result",
        LIFECYCLE_PHASE_RESULT_READY => "result_ready",
        LIFECYCLE_PHASE_SCENE_EXIT => "scene_exit",
        _ => "unknown",
    }
}

#[inline(always)]
unsafe fn log_lifecycle_phase(
    entry_id: usize,
    phase: u8,
    stage_id: i32,
    ready_go: bool,
    result_mode: bool,
    fighter_status: i32,
    selected_ui_hash: u64,
    reason: &'static str,
) {
    if !crate::debug::enabled() {
        return;
    }

    let entry = entry_id.min(MAX_FIGHTERS - 1);
    let signature = (BOSS_LIFECYCLE_GENERATION[entry] as u64)
        ^ (phase as u64).rotate_left(7)
        ^ (stage_id as u32 as u64).rotate_left(13)
        ^ ((ready_go as u64) << 29)
        ^ ((result_mode as u64) << 30)
        ^ (fighter_status as u32 as u64).rotate_left(31)
        ^ selected_ui_hash.rotate_left(43);
    if BOSS_LIFECYCLE_LAST_SIGNATURE[entry] == signature {
        return;
    }
    BOSS_LIFECYCLE_LAST_SIGNATURE[entry] = signature;
    crate::boss_log!(
        "[PB][MatchLifecycle] generation={} entry={} phase={} stage=0x{:x} ready_go={} result_mode={} fighter_status={} selected_ui_hash=0x{:010x} reason={}",
        BOSS_LIFECYCLE_GENERATION[entry],
        entry,
        lifecycle_phase_name(phase),
        stage_id,
        ready_go,
        result_mode,
        fighter_status,
        selected_ui_hash,
        reason
    );
}

/// Clear result/quarantine state only at a real new-round entry boundary.
/// Rebirth during an active stock match is intentionally excluded unless the
/// previous result state is still latched, so normal respawns do not reset the
/// boss runtime.
#[inline(always)]
unsafe fn reset_stale_match_generation_if_new_round(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
    entry_id: usize,
    stage_id: i32,
    ready_go: bool,
    result_mode: bool,
) -> bool {
    if module_accessor.is_null() || ready_go || result_mode {
        return false;
    }

    let entry = entry_id.min(MAX_FIGHTERS - 1);
    let fighter_status = StatusModule::status_kind(module_accessor);
    let entry_status = fighter_status == *FIGHTER_STATUS_KIND_ENTRY;
    let rebirth_after_result =
        RESULT_MODE_SEEN[entry] && fighter_status == *FIGHTER_STATUS_KIND_REBIRTH;
    let stale_result_state = POST_MATCH_PRE_RESULT[entry]
        || POST_MATCH_TRACKING_INVALIDATED[entry]
        || RESULT_MODE_SEEN[entry];

    // Spirit "Replay" reuses the scene: it never enters result mode, so
    // RESULT_MODE_SEEN stays false and the next battle's host is never seen in
    // ENTRY. Hardware showed it sitting in FIGHTER_STATUS_KIND_WAIT (0x0) with
    // ready_go=false, result_mode=false and the previous match's latches still
    // set, which left `transition_phase=post_match_pre_result` and blocked
    // acquisition with `transition_quarantine` until Ready-Go cleared it.
    //
    // Accept that boundary, but only once the finished match's own teardown has
    // completed (POST_MATCH_TRACKING_INVALIDATED) and the host has been settled
    // in that idle status for several consecutive frames. The genuine
    // finished-match window is a brief transition heading into results, so it
    // cannot hold this state; a real result scene sets result_mode first and is
    // excluded by the caller's guard above.
    let idle_pre_match = fighter_status == *FIGHTER_STATUS_KIND_WAIT
        && POST_MATCH_TRACKING_INVALIDATED[entry]
        && !RESULT_MODE_SEEN[entry];
    if idle_pre_match {
        NEW_ROUND_IDLE_FRAMES[entry] = NEW_ROUND_IDLE_FRAMES[entry].saturating_add(1);
    } else {
        NEW_ROUND_IDLE_FRAMES[entry] = 0;
    }
    let replay_boundary =
        idle_pre_match && NEW_ROUND_IDLE_FRAMES[entry] >= NEW_ROUND_IDLE_FRAMES_REQUIRED;

    if !stale_result_state || (!entry_status && !rebirth_after_result && !replay_boundary) {
        return false;
    }
    NEW_ROUND_IDLE_FRAMES[entry] = 0;
    let reset_reason = if entry_status {
        "new_round_entry"
    } else if rebirth_after_result {
        "rebirth_after_result"
    } else {
        "replay_idle_pre_match"
    };

    let previous_phase = lifecycle_phase_name(BOSS_LIFECYCLE_PHASE[entry]);
    // Clear the previous match's per-entry WOL secondary-selection latch
    // before resolving this new round. The fresh decision made below remains
    // armed for the new match rather than being cleared after resolution.
    selection::reset_condensed_selection(entry);
    let selected_ui_hash = selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0);

    // This clears only the temporary scene suppression. The selected boss UI
    // identity remains intact and is re-consumed by the normal setup path.
    selection::clear_boss_selection_suppression_if_ready_go(module_accessor);
    reset_boss_runtime_for_fighter(module_accessor, entry);

    BOSS_MATCH_STARTED[entry] = false;
    BOSS_HAD_READY_GO[entry] = false;
    POST_MATCH_PRE_RESULT[entry] = false;
    POST_MATCH_TRACKING_INVALIDATED[entry] = false;
    RESULT_MODE_SEEN[entry] = false;
    RESULT_BISECT_ACTIVE[entry] = false;
    RESULT_BISECT_TICKS[entry] = 0;
    RESULT_BISECT_ALIVE_LOGGED[entry] = false;
    TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry] = u64::MAX;
    RESULT_TRANSITION_LAST_SIGNATURE[entry] = u64::MAX;
    BOSS_LIFECYCLE_GENERATION[entry] = BOSS_LIFECYCLE_GENERATION[entry].wrapping_add(1);
    BOSS_LIFECYCLE_PHASE[entry] = LIFECYCLE_PHASE_PRE_MATCH;
    BOSS_LIFECYCLE_LAST_SIGNATURE[entry] = u64::MAX;
    boss_summon::reset_result_roster_diagnostics();

    crate::boss_log!(
        "[PB][NewRoundReset] entry={} generation={} reason={} fighter_status={} stale_cleared=true quarantine_released=true",
        entry,
        BOSS_LIFECYCLE_GENERATION[entry],
        reset_reason,
        fighter_status
    );
    crate::boss_log!(
        "[PB][MatchLifecycle] new_generation entry={} generation={} previous_phase={} reset_reason=new_round_entry stage=0x{:x} fighter_status={} selected_ui_hash=0x{:010x} selected_identity_preserved=true",
        entry,
        BOSS_LIFECYCLE_GENERATION[entry],
        previous_phase,
        stage_id,
        fighter_status,
        selected_ui_hash
    );
    log_lifecycle_phase(
        entry,
        LIFECYCLE_PHASE_PRE_MATCH,
        stage_id,
        ready_go,
        result_mode,
        fighter_status,
        selected_ui_hash,
        "new_round_entry",
    );
    true
}

#[inline(always)]
unsafe fn reset_boss_runtime_bookkeeping(entry_id: usize) {
    boss_runtime::reset_all_for_entry(entry_id);
    boss_helpers::clear_boss_mario_host_latch(entry_id);
    selection::reset_selector_authority_log(entry_id);
    result_camera::reset_battle_identity(entry_id);
    playable_masterhand::reset_match_state(entry_id);
    mastercrazy::reset_match_state(entry_id);
    galeem::reset_match_state(entry_id);
    dharkon::reset_match_state(entry_id);
    marx::reset_match_state(entry_id);
    dracula::reset_match_state(entry_id);
    rathalos::reset_match_state(entry_id);
    galleom::reset_match_state(entry_id);
    ganon::reset_match_state(entry_id);
}

/// Reset only the runtime owned by the fighter that is crossing a scene
/// boundary. Giga Bowser is a dedicated `koopag` fighter with process-local
/// state, so it must not be reset by an unrelated hidden Mario host.
#[inline(always)]
unsafe fn reset_boss_runtime_for_fighter(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
    entry_id: usize,
) {
    if !module_accessor.is_null()
        && smash::app::utility::get_kind(&mut *module_accessor) == *FIGHTER_KIND_KOOPAG
    {
        gigabowser::reset_match_state(entry_id);
    } else {
        reset_boss_runtime_bookkeeping(entry_id);
    }
}

/// Invalidate plugin bookkeeping after native battle teardown has started.
///
/// The post-match callback already releases any temporary native ownership
/// while the item objects are still valid. After that point this helper must
/// not call a reset routine that can inspect an item, change its status, or
/// restore a WorkModule flag. Native owns destruction during this window;
/// this path only clears IDs and scalar state held by the plugin.
#[inline(always)]
unsafe fn invalidate_boss_tracking_during_native_teardown(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
    entry_id: usize,
) {
    let entry = boss_runtime::sanitize_entry_id(entry_id);
    boss_runtime::reset_all_for_entry(entry);

    if !module_accessor.is_null()
        && smash::app::utility::get_kind(&mut *module_accessor) == *FIGHTER_KIND_KOOPAG
    {
        // Giga Bowser owns a dedicated fighter path and its reset is scalar
        // bookkeeping only. The shared item-boss reset is intentionally not
        // used here because it can inspect live Master/Crazy items.
        gigabowser::reset_match_state(entry);
    } else {
        playable_masterhand::reset_match_state(entry);
        mastercrazy::invalidate_transition_tracking(entry);
        galeem::reset_match_state(entry);
        dharkon::reset_match_state(entry);
        marx::reset_match_state(entry);
        dracula::reset_match_state(entry);
        rathalos::reset_match_state(entry);
        galleom::reset_match_state(entry);
        ganon::reset_match_state(entry);
    }
}

unsafe fn any_boss_active() -> bool {
    mastercrazy::check_status()
        || mastercrazy::check_status_2()
        || playable_masterhand::check_status()
        || galeem::check_status()
        || dharkon::check_status()
        || marx::check_status()
        || dracula::check_status()
        || rathalos::check_status()
        || galleom::check_status()
        || ganon::check_status()
}

unsafe fn suppress_hidden_host_result_audio(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() || !boss_helpers::is_hidden_host(module_accessor) {
        return;
    }
    let fighter_manager = boss_helpers::fighter_manager();
    if fighter_manager.is_null() || !FighterManager::is_result_mode(fighter_manager) {
        return;
    }
    boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
}

/// Narrow predicate for AUDIO suppression only.
///
/// `is_boss_mario_host` intentionally falls back to "this entry has a boss CSS
/// selection", which is correct for identity work but far too broad to hang
/// `stop_all_sound` on. Since 3.1.0 the persisted-selection restore populates
/// that selection cache at plugin init, so on a cold launch a *plain* Mario
/// sitting at that entry answered true and had `stop_all_sound` called twice
/// per frame, forever -- reported as "most of Mario's sound effects are
/// completely silent, just Mario, as both player and CPU".
///
/// Audio suppression therefore requires physical evidence that this fighter is
/// actually acting as a boss host right now: a live hidden-host scale, or the
/// per-entry latch while that host is still in the death window (scale can
/// reset before the KO scream finishes). A CSS/persisted selection is never
/// enough, and a leftover latch on a visible living Mario (training after WOL
/// Master Hand, typically with no result scene to reset bookkeeping) is
/// released instead of muting the next match.
unsafe fn is_boss_mario_host_for_audio(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let hidden = boss_helpers::is_hidden_host(module_accessor)
        || boss_helpers::is_hidden_host_entry_prep(module_accessor)
        || boss_helpers::is_hidden_host_entry_stage_two(module_accessor)
        || boss_helpers::is_hidden_host_baseline(module_accessor);
    let latched = boss_helpers::is_marked_boss_mario_host(module_accessor);
    let death_audio_status = boss_helpers::is_mario_death_audio_status(StatusModule::status_kind(
        module_accessor,
    ));
    match boss_helpers::boss_mario_host_audio_decision(hidden, latched, death_audio_status) {
        boss_helpers::BossMarioHostAudioDecision::Suppress => {
            if hidden {
                boss_helpers::mark_boss_mario_host(module_accessor);
            }
            true
        }
        boss_helpers::BossMarioHostAudioDecision::ReleaseLatch => {
            let entry = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
            boss_helpers::clear_boss_mario_host_latch(entry);
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][MarioAudio] drop_host_latch entry={} status={} scale={:.4}",
                    entry,
                    StatusModule::status_kind(module_accessor),
                    ModelModule::scale(module_accessor)
                );
            }
            false
        }
        boss_helpers::BossMarioHostAudioDecision::None => false,
    }
}

unsafe fn suppress_boss_mario_host_death_voice(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if !is_boss_mario_host_for_audio(module_accessor) {
        return;
    }
    boss_helpers::suppress_boss_mario_death_voice(module_accessor);
}

unsafe fn log_hidden_host_transition_snapshot(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }

    let entry_id = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    let fighter_manager = boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let ready_go = smash::app::sv_information::is_ready_go();
    let hidden_host = boss_helpers::is_hidden_host(module_accessor);
    let match_started = BOSS_MATCH_STARTED[entry_id];

    if ready_go && !result_mode && !match_started {
        return;
    }

    let stage_id = smash::app::stage::get_stage_id();
    let fighter_status = StatusModule::status_kind(module_accessor);
    let any_boss = any_boss_active();
    let have_item_id = if ItemModule::is_have_item(module_accessor, 0) {
        ItemModule::get_have_item_id(module_accessor, 0) as i32
    } else {
        -1
    };
    let host_kind = smash::app::utility::get_kind(&mut *module_accessor);
    let operation_cpu = boss_helpers::is_operation_cpu_entry(fighter_manager, entry_id);
    let boss_kind =
        if have_item_id >= 0 && smash::app::sv_battle_object::is_active(have_item_id as u32) {
            let boss_boma = smash::app::sv_battle_object::module_accessor(have_item_id as u32);
            if boss_boma.is_null() {
                -1
            } else {
                smash::app::utility::get_kind(&mut *boss_boma)
            }
        } else {
            -1
        };
    let selected_ui_hash = selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0);
    let scale_bits = ModelModule::scale(module_accessor).to_bits();
    let flags = (ready_go as u16)
        | ((result_mode as u16) << 1)
        | ((hidden_host as u16) << 2)
        | ((match_started as u16) << 3)
        | ((any_boss as u16) << 4)
        | ((operation_cpu as u16) << 5);

    if TRANSITION_DEBUG_LAST_STAGE[entry_id] == stage_id
        && TRANSITION_DEBUG_LAST_STATUS[entry_id] == fighter_status
        && TRANSITION_DEBUG_LAST_FLAGS[entry_id] == flags
        && TRANSITION_DEBUG_LAST_HAVE_ITEM[entry_id] == have_item_id
        && TRANSITION_DEBUG_LAST_SCALE_BITS[entry_id] == scale_bits
        && TRANSITION_DEBUG_LAST_HOST_KIND[entry_id] == host_kind
        && TRANSITION_DEBUG_LAST_BOSS_KIND[entry_id] == boss_kind
        && TRANSITION_DEBUG_LAST_SELECTED_UI_HASH[entry_id] == selected_ui_hash
    {
        return;
    }

    TRANSITION_DEBUG_LAST_STAGE[entry_id] = stage_id;
    TRANSITION_DEBUG_LAST_STATUS[entry_id] = fighter_status;
    TRANSITION_DEBUG_LAST_FLAGS[entry_id] = flags;
    TRANSITION_DEBUG_LAST_HAVE_ITEM[entry_id] = have_item_id;
    TRANSITION_DEBUG_LAST_SCALE_BITS[entry_id] = scale_bits;
    TRANSITION_DEBUG_LAST_HOST_KIND[entry_id] = host_kind;
    TRANSITION_DEBUG_LAST_BOSS_KIND[entry_id] = boss_kind;
    TRANSITION_DEBUG_LAST_SELECTED_UI_HASH[entry_id] = selected_ui_hash;

    crate::boss_log!(
        "[PB][TransitionState] entry={} stage=0x{:x} ready_go={} result_mode={} hidden_host={} match_started={} any_boss={} operation_cpu={} host_kind={} boss_kind={} selected_ui_hash=0x{:010x} fighter_status={} have_item_id={} scale={:.4}",
        entry_id,
        stage_id,
        ready_go,
        result_mode,
        hidden_host,
        match_started,
        any_boss,
        operation_cpu,
        host_kind,
        boss_kind,
        selected_ui_hash,
        fighter_status,
        have_item_id,
        f32::from_bits(scale_bits)
    );
}

unsafe fn log_boss_presentation_snapshot(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }

    let entry_id = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    let fighter_manager = boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let hidden_host = boss_helpers::is_hidden_host(module_accessor);
    let host_scale = ModelModule::scale(module_accessor);
    let host_status = StatusModule::status_kind(module_accessor);
    let have_item_id = if ItemModule::is_have_item(module_accessor, 0) {
        ItemModule::get_have_item_id(module_accessor, 0) as i32
    } else {
        -1
    };
    let (boss_kind, boss_status, boss_active) =
        if have_item_id >= 0 && smash::app::sv_battle_object::is_active(have_item_id as u32) {
            let boss_boma = smash::app::sv_battle_object::module_accessor(have_item_id as u32);
            if boss_boma.is_null() {
                (-1, -1, false)
            } else {
                (
                    smash::app::utility::get_kind(&mut *boss_boma),
                    StatusModule::status_kind(boss_boma),
                    true,
                )
            }
        } else {
            (-1, -1, false)
        };

    // This is a transition logger, not a frame logger. It records the exact
    // host/item visibility changes that can explain result or Final Smash
    // presentation regressions without changing either lifecycle.
    let signature = host_scale.to_bits() as u64
        ^ ((host_status as u64) << 32)
        ^ ((have_item_id as u32 as u64) << 1)
        ^ ((boss_status as u64) << 17)
        ^ ((result_mode as u64) << 63);
    if PRESENTATION_DEBUG_LAST_SIGNATURE[entry_id] == signature {
        return;
    }
    PRESENTATION_DEBUG_LAST_SIGNATURE[entry_id] = signature;

    if !hidden_host && !boss_active && !result_mode && have_item_id < 0 {
        return;
    }

    crate::boss_log!(
        "[PB][BossVisibility] entry={} scene={} host_kind={} host_status={} hidden_host={} host_scale={:.4} boss_object_id={} boss_kind={} boss_status={} boss_active={} selected_ui_hash=0x{:010x}",
        entry_id,
        if result_mode { "result" } else { "battle" },
        smash::app::utility::get_kind(&mut *module_accessor),
        host_status,
        hidden_host,
        host_scale,
        have_item_id,
        boss_kind,
        boss_status,
        boss_active,
        selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0)
    );
}

/// Observe the narrow transition between the battle ending and native result
/// mode.  Boss frame callbacks must not run in this window: their normal
/// recovery paths are valid during gameplay, but can race native item teardown
/// after Ready-Go has ended.
unsafe fn update_result_transition_state(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) -> BossTransitionPhase {
    if module_accessor.is_null() {
        return BossTransitionPhase::NotApplicable;
    }

    let entry_id = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    let fighter_manager = boss_helpers::fighter_manager();
    let stage_id = smash::app::stage::get_stage_id();
    let ready_go = smash::app::sv_information::is_ready_go();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    // A new round can reuse the same hidden host and selection identity before
    // Ready-Go becomes true. Clear only stale result-generation state at the
    // native ENTRY boundary; ordinary in-match REBIRTH is left untouched.
    reset_stale_match_generation_if_new_round(
        module_accessor,
        entry_id,
        stage_id,
        ready_go,
        result_mode,
    );
    // Once native result mode is active, the host/item and selection scene are
    // already crossing ownership boundaries.  Boss-match state was latched
    // during battle, so use that bookkeeping to identify our result context
    // instead of reading presentation data while native teardown is running.
    let boss_result_context = BOSS_MATCH_STARTED[entry_id]
        || POST_MATCH_PRE_RESULT[entry_id]
        || RESULT_MODE_SEEN[entry_id];
    if result_mode && !boss_result_context {
        return BossTransitionPhase::ResultReady;
    }
    let hidden_host = if result_mode {
        false
    } else {
        boss_helpers::is_hidden_host(module_accessor)
    };
    let selected_ui_hash = if result_mode {
        0
    } else {
        // Read-only, latched once per entry: proves which candidate supplies the
        // battle selector when a restored selection fails to take over (#89).
        selection::log_selector_authority(module_accessor);
        selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0)
    };
    let had_ready_go = BOSS_HAD_READY_GO[entry_id];

    if !result_mode
        && !hidden_host
        && selected_ui_hash == 0
        && !BOSS_MATCH_STARTED[entry_id]
        && !RESULT_MODE_SEEN[entry_id]
    {
        return BossTransitionPhase::NotApplicable;
    }

    // The quarantine is global to the result scene. A normal fighter callback
    // must not fall through to result-camera or boss-frame work merely because
    // that entry has no boss selection of its own.
    if ready_go && !result_mode && (hidden_host || selected_ui_hash != 0 || any_boss_active()) {
        BOSS_HAD_READY_GO[entry_id] = true;
        // A boss battle has actually begun, so this selection is worth
        // remembering across a reboot. Browsing the CSS without starting a
        // match never reaches here and therefore never overwrites the saved
        // value. Writes only when the boss actually changed.
        // Pass the RESOLVED identity so a condensed Shield alternate
        // (WOL Master Hand / Galleom) confirms as the boss that actually loaded.
        selection::persist_selection_for_started_battle(entry_id, selected_ui_hash);
        BOSS_MATCH_STARTED[entry_id] = true;
        POST_MATCH_PRE_RESULT[entry_id] = false;
        POST_MATCH_TRACKING_INVALIDATED[entry_id] = false;
        RESULT_BISECT_ACTIVE[entry_id] = false;
        RESULT_BISECT_TICKS[entry_id] = 0;
        RESULT_BISECT_ALIVE_LOGGED[entry_id] = false;
        BOSS_LIFECYCLE_PHASE[entry_id] = LIFECYCLE_PHASE_BATTLE;
        log_lifecycle_phase(
            entry_id,
            LIFECYCLE_PHASE_BATTLE,
            stage_id,
            ready_go,
            result_mode,
            StatusModule::status_kind(module_accessor),
            selected_ui_hash,
            "ready_go",
        );
    }

    let mut transition_action = "none";
    let phase = if result_mode {
        if !RESULT_BISECT_ACTIVE[entry_id] {
            RESULT_BISECT_ACTIVE[entry_id] = true;
            RESULT_BISECT_TICKS[entry_id] = 0;
            RESULT_BISECT_ALIVE_LOGGED[entry_id] = false;
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][ResultBisect] stage=A native_result_quarantine_enter entry={} stage_id=0x{:x} result_pipeline={}",
                    entry_id,
                    stage_id,
                    result_camera::active_result_pipeline_stage_name()
                );
                crate::boss_log!(
                    "[PB][ResultBisect] stage=A step=authority_abort begin entry={}",
                    entry_id
                );
            }
            // This is an idempotent final barrier for the rare transition
            // that reaches result mode without exposing the intermediate
            // post-match callback. It only restores plugin-owned authority;
            // native item destruction remains entirely native-owned.
            // The post-match path normally restores temporary hand ownership
            // before native result teardown. If that callback is skipped, do
            // not dereference battle items here: native result mode owns them.
            mastercrazy::quarantine_hand_authority_for_result("result_ready_quarantine");
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][ResultBisect] stage=A step=authority_abort ok entry={}",
                    entry_id
                );
            }
        }
        RESULT_MODE_SEEN[entry_id] = true;
        POST_MATCH_PRE_RESULT[entry_id] = false;
        BOSS_LIFECYCLE_PHASE[entry_id] = LIFECYCLE_PHASE_RESULT_READY;
        log_lifecycle_phase(
            entry_id,
            LIFECYCLE_PHASE_RESULT_READY,
            stage_id,
            ready_go,
            result_mode,
            StatusModule::status_kind(module_accessor),
            selected_ui_hash,
            "native_result_mode",
        );
        BossTransitionPhase::ResultReady
    } else if !ready_go && had_ready_go && BOSS_MATCH_STARTED[entry_id] {
        if !POST_MATCH_PRE_RESULT[entry_id] {
            let pair_was_active = mastercrazy::hand_team_authority_active_for_debug();
            mastercrazy::abort_hand_team_for_transition("post_match_pre_result");
            POST_MATCH_PRE_RESULT[entry_id] = true;
            BOSS_LIFECYCLE_PHASE[entry_id] = LIFECYCLE_PHASE_POST_MATCH_PRE_RESULT;
            log_lifecycle_phase(
                entry_id,
                LIFECYCLE_PHASE_POST_MATCH_PRE_RESULT,
                stage_id,
                ready_go,
                result_mode,
                StatusModule::status_kind(module_accessor),
                selected_ui_hash,
                "ready_go_ended",
            );
            transition_action = if pair_was_active {
                "hand_authority_aborted_once"
            } else {
                "post_match_guard_armed"
            };
        }
        BossTransitionPhase::PostMatchPreResult
    } else if !result_mode && RESULT_MODE_SEEN[entry_id] {
        transition_action = "scene_exit_observed";
        BossTransitionPhase::SceneExit
    } else if ready_go {
        BossTransitionPhase::Battle
    } else {
        return BossTransitionPhase::NotApplicable;
    };

    // Result mode and the post-match gap are quarantine states.  Do not read
    // the host's item slots or touch an item object here: native teardown owns
    // those objects, and stale access is exactly what this boundary must avoid.
    if phase != BossTransitionPhase::Battle {
        // Capture Galeem/Dharkon's logical entry and summon state before the
        // observational summon record is cleared below.  This call is
        // explicitly read-only in the transition window.
        let audit_phase = match phase {
            BossTransitionPhase::PostMatchPreResult => "post_match_pre_result",
            BossTransitionPhase::ResultReady => "result_ready",
            BossTransitionPhase::SceneExit => "scene_exit",
            _ => "quarantine",
        };

        // Galeem/Dharkon summon tracing is observational only. Clear that
        // plugin bookkeeping at the same boundary as the other transient
        // systems, without touching the native summon or its fighter object.
        boss_summon::cancel_for_transition(match phase {
            BossTransitionPhase::PostMatchPreResult => "post_match_pre_result",
            BossTransitionPhase::ResultReady => "result_ready",
            BossTransitionPhase::SceneExit => "scene_exit",
            _ => "quarantine",
        });
        boss_summon::log_result_roster_snapshot(audit_phase);
        if phase == BossTransitionPhase::ResultReady {
            RESULT_BISECT_TICKS[entry_id] = RESULT_BISECT_TICKS[entry_id].saturating_add(1);
            if RESULT_BISECT_TICKS[entry_id] == 45 && !RESULT_BISECT_ALIVE_LOGGED[entry_id] {
                RESULT_BISECT_ALIVE_LOGGED[entry_id] = true;
                crate::boss_log!(
                    "[PB][ResultBisect] stage=A native_result_quarantine_alive entry={} ticks={} result_pipeline={} battle_item_deref=false central_presentation_authority=enabled",
                    entry_id,
                    RESULT_BISECT_TICKS[entry_id],
                    result_camera::active_result_pipeline_stage_name()
                );
            }
        }

        let fighter_status = StatusModule::status_kind(module_accessor);
        let operation_cpu = boss_helpers::is_operation_cpu_entry(fighter_manager, entry_id);
        let pair_active = false;
        let phase_name = match phase {
            BossTransitionPhase::PostMatchPreResult => "post_match_pre_result",
            BossTransitionPhase::ResultReady => "result_ready",
            BossTransitionPhase::SceneExit => "scene_exit",
            _ => "quarantine",
        };
        let phase_tag = match phase {
            BossTransitionPhase::PostMatchPreResult => 1u64,
            BossTransitionPhase::ResultReady => 2u64,
            BossTransitionPhase::SceneExit => 3u64,
            _ => 0u64,
        };
        let signature = (stage_id as u32 as u64)
            ^ ((ready_go as u64) << 1)
            ^ ((result_mode as u64) << 2)
            ^ ((had_ready_go as u64) << 3)
            ^ ((BOSS_MATCH_STARTED[entry_id] as u64) << 4)
            ^ phase_tag.rotate_left(9)
            ^ selected_ui_hash.rotate_left(19)
            ^ (fighter_status as u32 as u64).rotate_left(31);
        if RESULT_TRANSITION_LAST_SIGNATURE[entry_id] != signature {
            RESULT_TRANSITION_LAST_SIGNATURE[entry_id] = signature;
            crate::boss_log!(
                "[PB][ResultTransition] probe_begin entry={} stage=0x{:x} ready_go={} had_ready_go={} match_started={} result_mode={} selected_ui_hash=0x{:010x}",
                entry_id,
                stage_id,
                ready_go,
                had_ready_go,
                BOSS_MATCH_STARTED[entry_id],
                result_mode,
                selected_ui_hash
            );
            crate::boss_log!(
                "[PB][ResultTransition] probe_end entry={} phase={} fighter_status={} operation_cpu={} battle_object_id=0x0 battle_object_active=false tracked_object_id=0x0 boss_kind=-1 boss_status=-1 paired_authority_active={} temporary_ai_suppression={} recovery_enabled=false reacquisition_enabled=false result_quarantine=true custom_result_pipeline={} cleanup_action={}",
                entry_id,
                phase_name,
                fighter_status,
                operation_cpu,
                pair_active,
                pair_active,
                result_camera::custom_result_pipeline_enabled(),
                if phase == BossTransitionPhase::ResultReady {
                    "result_presentation_authority_armed"
                } else {
                    transition_action
                }
            );
        }
        return phase;
    }

    boss_summon::log_result_roster_snapshot("battle");
    let have_item_id = if ItemModule::is_have_item(module_accessor, 0) {
        ItemModule::get_have_item_id(module_accessor, 0) as i32
    } else {
        -1
    };
    let battle_object_active =
        have_item_id >= 0 && smash::app::sv_battle_object::is_active(have_item_id as u32);
    let (boss_kind, boss_status) = if battle_object_active {
        let boss_boma = smash::app::sv_battle_object::module_accessor(have_item_id as u32);
        if boss_boma.is_null() {
            (-1, -1)
        } else {
            (
                smash::app::utility::get_kind(&mut *boss_boma),
                StatusModule::status_kind(boss_boma),
            )
        }
    } else {
        (-1, -1)
    };
    let fighter_status = StatusModule::status_kind(module_accessor);
    let operation_cpu = boss_helpers::is_operation_cpu_entry(fighter_manager, entry_id);
    let pair_active = mastercrazy::hand_team_authority_active_for_debug();
    let signature = (stage_id as u32 as u64)
        ^ ((ready_go as u64) << 1)
        ^ ((result_mode as u64) << 2)
        ^ ((had_ready_go as u64) << 3)
        ^ ((BOSS_MATCH_STARTED[entry_id] as u64) << 4)
        ^ ((phase == BossTransitionPhase::PostMatchPreResult) as u64) << 5
        ^ ((battle_object_active as u64) << 6)
        ^ ((operation_cpu as u64) << 7)
        ^ ((pair_active as u64) << 8)
        ^ (have_item_id as u32 as u64).rotate_left(11)
        ^ (selected_ui_hash.rotate_left(19))
        ^ (fighter_status as u32 as u64).rotate_left(31)
        ^ (boss_kind as u32 as u64).rotate_left(43)
        ^ (boss_status as u32 as u64).rotate_left(47);

    if RESULT_TRANSITION_LAST_SIGNATURE[entry_id] == signature {
        return phase;
    }
    RESULT_TRANSITION_LAST_SIGNATURE[entry_id] = signature;

    crate::boss_log!(
        "[PB][ResultTransition] probe_begin entry={} stage=0x{:x} ready_go={} had_ready_go={} match_started={} result_mode={} selected_ui_hash=0x{:010x}",
        entry_id,
        stage_id,
        ready_go,
        had_ready_go,
        BOSS_MATCH_STARTED[entry_id],
        result_mode,
        selected_ui_hash
    );
    crate::boss_log!(
        "[PB][ResultTransition] probe_end entry={} phase={} fighter_status={} operation_cpu={} battle_object_id=0x{:x} battle_object_active={} tracked_object_id=0x{:x} boss_kind={} boss_status={} paired_authority_active={} temporary_ai_suppression={} recovery_enabled={} reacquisition_enabled={} cleanup_action={}",
        entry_id,
        "battle",
        fighter_status,
        operation_cpu,
        have_item_id.max(0) as u32,
        battle_object_active,
        have_item_id.max(0) as u32,
        boss_kind,
        boss_status,
        pair_active,
        pair_active,
        true,
        true,
        transition_action
    );

    phase
}

pub unsafe fn any_post_match_pre_result() -> bool {
    let mut entry = 0;
    while entry < MAX_FIGHTERS {
        if POST_MATCH_PRE_RESULT[entry] {
            return true;
        }
        entry += 1;
    }
    false
}

#[inline(always)]
pub unsafe fn boss_lifecycle_generation(entry_id: usize) -> u32 {
    BOSS_LIFECYCLE_GENERATION[entry_id.min(MAX_FIGHTERS - 1)]
}

static mut EARLY_TAKEOVER_LAST_SIGNATURE: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
/// Consecutive qualifying frames observed for the Spirit-Replay new-round
/// boundary. Bounded, plugin-owned, no engine reads.
static mut NEW_ROUND_IDLE_FRAMES: [u8; MAX_FIGHTERS] = [0; MAX_FIGHTERS];
/// Frames of settled pre-Ready-Go idle required before the replay boundary is
/// accepted. Long enough that the finished match's own transition cannot be
/// mistaken for it, short enough that acquisition still precedes Ready-Go.
const NEW_ROUND_IDLE_FRAMES_REQUIRED: u8 = 6;

/// MINIMUM-SAFE pre-Ready-Go trace.
///
/// A previous revision of this diagnostic crashed on cold launch before it could
/// emit anything. Every field that required an additional engine lookup has been
/// removed; what remains is the caller's already-computed phase plus plugin-owned
/// statics and the three engine reads this exact callback provably already makes
/// on every frame (see the safety note per call below).
///
/// Deliberately absent, and not to be re-added without proof:
///   `any_boss_active()`      - fans out to 10 boss-module `check_status()` calls,
///                              each dereferencing a tracked battle object.
///   `ItemModule::get_have_item_id`, `ModelModule::scale`, `is_hidden_host`
///   `FighterInformation::{stock_count, dead_count, is_on_rebirth}` and the
///   `fighter_information_entry` lookup that produces the pointer they need.
unsafe fn log_early_takeover(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
    phase: BossTransitionPhase,
    entry_id: usize,
    fighter_status: i32,
    ready_go: bool,
    result_mode: bool,
) {
    if !crate::debug::enabled() || module_accessor.is_null() {
        return;
    }
    let entry = entry_id.min(MAX_FIGHTERS - 1);

    // Plugin-owned statics only.
    let post_match_pre_result = POST_MATCH_PRE_RESULT[entry];
    let post_match_tracking_invalidated = POST_MATCH_TRACKING_INVALIDATED[entry];
    let result_mode_seen = RESULT_MODE_SEEN[entry];
    let had_ready_go = BOSS_HAD_READY_GO[entry];
    let match_started = BOSS_MATCH_STARTED[entry];
    let generation = BOSS_LIFECYCLE_GENERATION[entry];

    // Derived from the caller's already-computed phase; no re-invocation of
    // `update_result_transition_state` (which mutates).
    let result_quarantine = matches!(
        phase,
        BossTransitionPhase::PostMatchPreResult
            | BossTransitionPhase::ResultReady
            | BossTransitionPhase::SceneExit
    );

    // Sub-conditions of the EXISTING `reset_stale_match_generation_if_new_round`
    // predicate, recomputed from the values above only.
    let entry_status = fighter_status == *FIGHTER_STATUS_KIND_ENTRY;
    let rebirth_after_result = result_mode_seen && fighter_status == *FIGHTER_STATUS_KIND_REBIRTH;
    let stale_result_state =
        post_match_pre_result || post_match_tracking_invalidated || result_mode_seen;
    let new_round_would_fire =
        !ready_go && !result_mode && stale_result_state && (entry_status || rebirth_after_result);

    let signature = (fighter_status as u32 as u64)
        ^ ((ready_go as u64) << 8)
        ^ ((result_mode as u64) << 9)
        ^ ((post_match_pre_result as u64) << 10)
        ^ ((had_ready_go as u64) << 11)
        ^ ((match_started as u64) << 12)
        ^ ((post_match_tracking_invalidated as u64) << 13)
        ^ ((result_mode_seen as u64) << 14)
        ^ ((result_quarantine as u64) << 15)
        ^ ((entry_status as u64) << 16)
        ^ ((stale_result_state as u64) << 17)
        ^ ((new_round_would_fire as u64) << 18)
        ^ ((generation as u64) << 32);
    if EARLY_TAKEOVER_LAST_SIGNATURE[entry] == signature {
        return;
    }
    EARLY_TAKEOVER_LAST_SIGNATURE[entry] = signature;

    crate::boss_log!(
        "[PB][EarlyTakeover] entry={} generation={} fighter_status={} ready_go={} result_mode={} post_match_pre_result={} post_match_tracking_invalidated={} result_mode_seen={} had_ready_go={} match_started={} entry_status={} rebirth_after_result={} stale_result_state={} new_round_would_fire={} transition_phase={} result_quarantine={} acquire_allowed={} acquire_block_reason={}",
        entry,
        generation,
        fighter_status,
        ready_go,
        result_mode,
        post_match_pre_result,
        post_match_tracking_invalidated,
        result_mode_seen,
        had_ready_go,
        match_started,
        entry_status,
        rebirth_after_result,
        stale_result_state,
        new_round_would_fire,
        match phase {
            BossTransitionPhase::NotApplicable => "not_applicable",
            BossTransitionPhase::Battle => "battle",
            BossTransitionPhase::PostMatchPreResult => "post_match_pre_result",
            BossTransitionPhase::ResultReady => "result_ready",
            BossTransitionPhase::SceneExit => "scene_exit",
        },
        result_quarantine,
        !result_quarantine,
        if result_quarantine {
            "transition_quarantine"
        } else {
            "none"
        }
    );
}

/// Dedicated fighter agents do not pass through the Mario-host dispatcher.
/// Give them the same read-only result quarantine without exposing the
/// transition implementation or allowing them to inspect native item slots.
#[inline(always)]
pub unsafe fn should_quarantine_boss_frame(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) -> bool {
    matches!(
        update_result_transition_state(module_accessor),
        BossTransitionPhase::PostMatchPreResult
            | BossTransitionPhase::ResultReady
            | BossTransitionPhase::SceneExit
    )
}

/// Complete the shared scene-exit bookkeeping for a dedicated boss agent.
/// This is intentionally separate from the result quarantine: native result
/// teardown remains native-owned, and reset only occurs after result mode ends.
#[inline(always)]
pub unsafe fn finish_boss_transition_cleanup(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    cleanup_hidden_host_post_match_transition(module_accessor);
}

unsafe fn cleanup_hidden_host_post_match_transition(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() {
        return;
    }

    let entry_id = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    let fighter_manager = boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let ready_go = smash::app::sv_information::is_ready_go();

    if result_mode {
        // Result mode is a strict quarantine. The selected UI identity is
        // deliberately retained for the result resolver, but no selection
        // suppression, item lookup, or battle cleanup is performed here.
        // Native result teardown owns the battle objects and the dedicated
        // result manager is the only code allowed to create presentation
        // objects after its pipeline is enabled.
        TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] = u64::MAX;
        return;
    }

    let hidden_host = boss_helpers::is_hidden_host(module_accessor);

    // Result mode has ended. This is the first safe point at which all
    // result-owned objects and hidden-host bookkeeping can be discarded,
    // including when the next scene reports Ready-Go immediately.
    if RESULT_MODE_SEEN[entry_id] {
        let stage_id = smash::app::stage::get_stage_id();
        TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] = u64::MAX;
        BOSS_LIFECYCLE_PHASE[entry_id] = LIFECYCLE_PHASE_SCENE_EXIT;
        log_lifecycle_phase(
            entry_id,
            LIFECYCLE_PHASE_SCENE_EXIT,
            stage_id,
            ready_go,
            result_mode,
            StatusModule::status_kind(module_accessor),
            0,
            "result_scene_exit",
        );
        selection::suppress_boss_selection_until_ready_go(entry_id);
        BOSS_MATCH_STARTED[entry_id] = false;
        BOSS_HAD_READY_GO[entry_id] = false;
        POST_MATCH_PRE_RESULT[entry_id] = false;
        POST_MATCH_TRACKING_INVALIDATED[entry_id] = false;
        RESULT_MODE_SEEN[entry_id] = false;
        RESULT_BISECT_ACTIVE[entry_id] = false;
        RESULT_BISECT_TICKS[entry_id] = 0;
        RESULT_BISECT_ALIVE_LOGGED[entry_id] = false;
        selection::reset_condensed_selection(entry_id);
        reset_boss_runtime_for_fighter(module_accessor, entry_id);
        crate::boss_log!(
            "[PB][ResultTransition] entry={} phase=scene_exit cleanup_action=result_bookkeeping_cleared stage=0x{:x}",
            entry_id,
            stage_id
        );
        if !fighter_manager.is_null() {
            FighterManager::set_cursor_whole(fighter_manager, true);
            FighterManager::set_position_lock(
                fighter_manager,
                smash::app::FighterEntryID(entry_id as i32),
                false,
            );
        }
        return;
    }

    if ready_go {
        TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] = u64::MAX;
        if hidden_host || any_boss_active() {
            BOSS_MATCH_STARTED[entry_id] = true;
        }
        return;
    }

    if !BOSS_MATCH_STARTED[entry_id] {
        TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] = u64::MAX;
        return;
    }

    if POST_MATCH_PRE_RESULT[entry_id] {
        TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] = u64::MAX;
        if !POST_MATCH_TRACKING_INVALIDATED[entry_id] {
            // This only clears plugin bookkeeping. Native item teardown owns
            // destruction; no item is removed or reacquired here.
            invalidate_boss_tracking_during_native_teardown(module_accessor, entry_id);
            POST_MATCH_TRACKING_INVALIDATED[entry_id] = true;
            crate::boss_log!(
                "[PB][ResultTransition] entry={} phase=post_match_pre_result cleanup_action=battle_tracking_invalidated_no_native_access",
                entry_id
            );
            boss_summon::log_result_roster_snapshot("post_match_after_tracking_invalidation");
        }
        return;
    }

    let stage_id = smash::app::stage::get_stage_id();
    let selected_ui_hash = selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0);
    let boss_selected = selected_ui_hash != 0;
    if boss_selected {
        let signature = (stage_id as u32 as u64)
            ^ selected_ui_hash.rotate_left(17)
            ^ ((hidden_host as u64) << 63);
        if crate::debug::enabled()
            && TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] != signature
        {
            TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] = signature;
            crate::boss_log!(
                "[PB][TransitionCleanup] entry {}: deferred cleanup because a boss selection is armed on stage=0x{:x} selected_ui_hash=0x{:010x}",
                entry_id,
                stage_id,
                selected_ui_hash
            );
        }
        return;
    }

    TRANSITION_DEBUG_LAST_DEFERRED_SIGNATURE[entry_id] = u64::MAX;
    selection::suppress_boss_selection_until_ready_go(entry_id);
    BOSS_MATCH_STARTED[entry_id] = false;
    selection::reset_condensed_selection(entry_id);
    reset_boss_runtime_for_fighter(module_accessor, entry_id);
    BOSS_HAD_READY_GO[entry_id] = false;
    POST_MATCH_PRE_RESULT[entry_id] = false;
    POST_MATCH_TRACKING_INVALIDATED[entry_id] = false;

    crate::boss_log!(
        "[PB][TransitionCleanup] entry {}: clearing boss runtime after non-result match transition on stage=0x{:x} hidden_host={}",
        entry_id,
        stage_id,
        hidden_host
    );

    if !fighter_manager.is_null() {
        FighterManager::set_cursor_whole(fighter_manager, true);
        FighterManager::set_position_lock(
            fighter_manager,
            smash::app::FighterEntryID(entry_id as i32),
            false,
        );
    }

    crate::boss_log!(
        "[PB][TransitionCleanupState] entry {}: bookkeeping-only cleanup complete stage=0x{:x}",
        entry_id,
        stage_id
    );
}

unsafe fn restore_plain_mario_after_hidden_host_cleanup(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() {
        return;
    }

    let current_scale = ModelModule::scale(module_accessor);
    if !boss_helpers::is_hidden_host(module_accessor)
        && !boss_helpers::is_hidden_host_baseline(module_accessor)
    {
        return;
    }

    let boss_selected = selection::selected_css_boss_selector_id(module_accessor).is_some();
    if boss_selected {
        return;
    }

    // A result boss is still the public presentation while the hidden host
    // remains attached to the result entry. Never restore Mario over it.
    if any_boss_active() {
        return;
    }

    let fighter_status = StatusModule::status_kind(module_accessor);
    let spawn_state = fighter_status == *FIGHTER_STATUS_KIND_ENTRY
        || fighter_status == *FIGHTER_STATUS_KIND_REBIRTH
        || fighter_status == *FIGHTER_STATUS_KIND_WAIT
        || fighter_status == *FIGHTER_STATUS_KIND_STANDBY
        || fighter_status == *FIGHTER_STATUS_KIND_FALL;
    if !spawn_state {
        return;
    }

    boss_helpers::restore_plain_mario_visuals(module_accessor);
    crate::boss_log!(
        "[PB][HiddenHost][PlainRestore] entry={} stage=0x{:x} fighter_status={} scale={:.4} -> 1.0000",
        boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1),
        smash::app::stage::get_stage_id(),
        fighter_status,
        current_scale
    );
}

unsafe fn log_mario_ready_go_dispatch_enter(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() {
        return;
    }
    let entry = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    if !smash::app::sv_information::is_ready_go() {
        MARIO_READY_GO_LOGGED[entry] = false;
        return;
    }
    if MARIO_READY_GO_LOGGED[entry] || !crate::debug::enabled() {
        return;
    }
    MARIO_READY_GO_LOGGED[entry] = true;
    let selector = selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0);
    crate::boss_log!(
        "[PB][MarioReadyGo] edge=dispatch_enter entry={} stage=0x{:x} selector=0x{:x} have_item={} host_status={} scale={:.4}",
        entry,
        smash::app::stage::get_stage_id(),
        selector,
        ItemModule::is_have_item(module_accessor, 0),
        StatusModule::status_kind(module_accessor),
        ModelModule::scale(module_accessor)
    );
}

extern "C" fn mario_boss_dispatch_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let module_accessor = fighter.module_accessor;
        log_mario_ready_go_dispatch_enter(module_accessor);

        // The Figure Player viewer has a real Mario host, but it is not a
        // battle host.  Its presentation-only item must consume the callback
        // before any boss AI, recovery, result, or match-lifecycle work sees
        // the menu scene.
        let stage_id = smash::app::stage::get_stage_id();
        let amiibo_viewer = stage_id == boss_helpers::STAGE_ID_AMIIBO_PREVIEW;
        let amiibo_consumed = amiibo_preview::frame(module_accessor, stage_id);
        if amiibo_consumed || amiibo_viewer {
            return;
        }

        let transition_phase = update_result_transition_state(module_accessor);
        // State-transition-only; the per-frame `EarlyTakeoverProbe` breadcrumbs
        // are removed now that the boundary is understood. Still lifecycle-safe:
        // four caller-local primitives, no battle-object/FighterInformation/
        // item/model access.
        if crate::debug::enabled() {
            let et_entry = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
            let et_status = StatusModule::status_kind(module_accessor);
            let et_ready_go = smash::app::sv_information::is_ready_go();
            let et_fm = boss_helpers::fighter_manager();
            let et_result_mode = !et_fm.is_null() && FighterManager::is_result_mode(et_fm);
            log_early_takeover(
                module_accessor,
                transition_phase,
                et_entry,
                et_status,
                et_ready_go,
                et_result_mode,
            );
        }
        suppress_boss_mario_host_death_voice(module_accessor);
        let battle_active = matches!(
            transition_phase,
            BossTransitionPhase::Battle | BossTransitionPhase::NotApplicable
        );
        if battle_active {
            // Snapshot the logical boss while the battle host still owns its
            // battle item. Result mode later consumes this immutable identity
            // instead of querying mutable CSS/menu state during teardown.
            result_camera::observe_battle_identity(module_accessor);
            selection::clear_boss_selection_suppression_if_ready_go(module_accessor);
            mastercrazy::master_frame(fighter);
            mastercrazy::crazy_frame(fighter);
            mastercrazy::hand_entrance_step(fighter.lua_state_agent, module_accessor);
            playable_masterhand::frame(fighter);
            galeem::frame(fighter);
            dharkon::frame(fighter);
            marx::frame(fighter);
            dracula::frame(fighter);
            rathalos::frame(fighter);
            galleom::frame(fighter);
            ganon::frame(fighter);
            ai_diagnostics::log_item_host(module_accessor);
            ai_diagnostics::log_fighter_control_state(module_accessor);
            suppress_hidden_host_result_audio(module_accessor);
        }

        // Normal boss frames are quarantined in ResultReady, so retain the
        // existing hidden-host audio suppression here rather than allowing a
        // Mario victory voice to leak through the presentation handoff.
        if transition_phase == BossTransitionPhase::ResultReady {
            suppress_hidden_host_result_audio(module_accessor);
        }

        // Result presentation is a separate authority. It must keep running
        // in ResultReady, where normal boss frames are quarantined.
        result_camera::frame(module_accessor);
        cleanup_hidden_host_post_match_transition(module_accessor);
        if battle_active {
            restore_plain_mario_after_hidden_host_cleanup(module_accessor);
            log_hidden_host_transition_snapshot(module_accessor);
            log_boss_presentation_snapshot(module_accessor);
        }
        suppress_boss_mario_host_death_voice(module_accessor);
    }
}

pub fn to_hash40(word: &str) -> Hash40 {
    Hash40(crc32_with_len(word))
}

fn crc32_with_len(word: &str) -> u64 {
    let mut hash = !0u32;
    let mut len: u8 = 0;
    for b in word.bytes() {
        let shift = hash >> 8;
        let index = (hash ^ (b as u32)) & 0xff;
        hash = shift ^ _CRC_TABLE[index as usize];
        len += 1;
    }
    ((len as u64) << 32) | (!hash as u64)
}

const _CRC_TABLE: [u32; 256] = [
    0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
    0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
    0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
    0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
    0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
    0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
    0x76dc4190, 0x01db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
    0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
    0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
    0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
    0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
    0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683,
    0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
    0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
    0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
    0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
    0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
    0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21,
    0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
    0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
    0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
    0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
];

fn patch_selector_param_value(
    param: &mut ParamKind,
    ui_chara_hash: Hash40,
    selector_id: i32,
) -> bool {
    if let Ok(value) = param.try_into_mut::<Hash40>() {
        *value = ui_chara_hash;
        return true;
    }
    if let Ok(value) = param.try_into_mut::<i32>() {
        *value = selector_id;
        return true;
    }
    if let Ok(value) = param.try_into_mut::<u32>() {
        *value = selector_id as u32;
        return true;
    }
    if let Ok(value) = param.try_into_mut::<i16>() {
        if let Ok(small) = i16::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    if let Ok(value) = param.try_into_mut::<u16>() {
        if let Ok(small) = u16::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    if let Ok(value) = param.try_into_mut::<i8>() {
        if let Ok(small) = i8::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    if let Ok(value) = param.try_into_mut::<u8>() {
        if let Ok(small) = u8::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    false
}

fn patch_tagged_selector_param_value(param: &mut ParamKind, selector_id: i32) -> bool {
    let tagged_selector = 0x5000_0000u64 | ((selector_id as u64) & 0x0FFF_FFFF);

    if let Ok(value) = param.try_into_mut::<u32>() {
        if *value == 0x5000_0000 {
            *value = tagged_selector as u32;
            return true;
        }
    }
    if let Ok(value) = param.try_into_mut::<i32>() {
        if (*value as u32) == 0x5000_0000 {
            *value = tagged_selector as i32;
            return true;
        }
    }

    false
}

fn patch_css_selector_fields(charroot: &mut ParamStruct, ui_chara_name: &str, selector_id: i32) {
    let ui_chara_hash = to_hash40(ui_chara_name);
    let selector_field_hashes = [
        to_hash40("summon_boss_id"),
        to_hash40("boss_id"),
        to_hash40("summon_id"),
    ];
    let mut found_field = false;
    let mut patched_field = false;
    let mut tagged_patch_count = 0usize;
    for (hash, param) in charroot.0.iter_mut() {
        if selector_field_hashes.contains(hash) {
            found_field = true;
            if patch_selector_param_value(param, ui_chara_hash, selector_id) {
                patched_field = true;
            }
        }
    }
    if !patched_field {
        for (_hash, param) in charroot.0.iter_mut() {
            if patch_tagged_selector_param_value(param, selector_id) {
                patched_field = true;
                tagged_patch_count += 1;
            }
        }
    }
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][SelectionPatch] {} selector=0x{:x} found_field={} patched_field={} tagged_patches={}",
            ui_chara_name,
            selector_id,
            found_field,
            patched_field,
            tagged_patch_count
        );
    }
}

fn struct_hash40_field_matches(
    param: &ParamKind,
    field_hash: Hash40,
    expected_value: Hash40,
) -> bool {
    let Ok(param_struct) = param.try_into_ref::<ParamStruct>() else {
        return false;
    };

    param_struct.0.iter().any(|(hash, field)| {
        *hash == field_hash
            && field
                .try_into_ref::<Hash40>()
                .map(|value| *value == expected_value)
                .unwrap_or(false)
    })
}

fn patch_hash40_field(
    param_struct: &mut ParamStruct,
    field_hash: Hash40,
    new_value: Hash40,
) -> bool {
    let mut patched = false;

    for (hash, field) in param_struct.0.iter_mut() {
        if *hash != field_hash {
            continue;
        }
        if let Ok(value) = field.try_into_mut::<Hash40>() {
            *value = new_value;
            patched = true;
        }
    }

    patched
}

fn patch_bool_field(param_struct: &mut ParamStruct, field_hash: Hash40, new_value: bool) -> bool {
    let mut patched = false;

    for (hash, field) in param_struct.0.iter_mut() {
        if *hash != field_hash {
            continue;
        }
        if let Ok(value) = field.try_into_mut::<bool>() {
            *value = new_value;
            patched = true;
        }
    }

    patched
}

fn patch_i8_field(param_struct: &mut ParamStruct, field_hash: Hash40, new_value: i8) -> bool {
    let mut patched = false;

    for (hash, field) in param_struct.0.iter_mut() {
        if *hash != field_hash {
            continue;
        }
        if let Ok(value) = field.try_into_mut::<i8>() {
            *value = new_value;
            patched = true;
        }
    }

    patched
}

fn read_u8_field(param_struct: &ParamStruct, field_hash: Hash40) -> Option<u8> {
    param_struct
        .0
        .iter()
        .find(|(hash, _)| *hash == field_hash)
        .and_then(|(_, field)| field.try_into_ref::<u8>().ok().copied())
}

#[cfg(test)]
fn read_i8_field(param_struct: &ParamStruct, field_hash: Hash40) -> Option<i8> {
    param_struct
        .0
        .iter()
        .find(|(hash, _)| *hash == field_hash)
        .and_then(|(_, field)| field.try_into_ref::<i8>().ok().copied())
}

fn read_bool_field(param_struct: &ParamStruct, field_hash: Hash40) -> Option<bool> {
    param_struct
        .0
        .iter()
        .find(|(hash, _)| *hash == field_hash)
        .and_then(|(_, field)| field.try_into_ref::<bool>().ok().copied())
}

fn read_hash40_field(param_struct: &ParamStruct, field_hash: Hash40) -> Option<Hash40> {
    param_struct
        .0
        .iter()
        .find(|(hash, _)| *hash == field_hash)
        .and_then(|(_, field)| field.try_into_ref::<Hash40>().ok().copied())
}

#[cfg(test)]
fn read_string_field(param_struct: &ParamStruct, field_hash: Hash40) -> Option<&str> {
    param_struct
        .0
        .iter()
        .find(|(hash, _)| *hash == field_hash)
        .and_then(|(_, field)| field.try_into_ref::<String>().ok())
        .map(String::as_str)
}

fn read_u16_field(param_struct: &ParamStruct, field_hash: Hash40) -> Option<u16> {
    param_struct
        .0
        .iter()
        .find(|(hash, _)| *hash == field_hash)
        .and_then(|(_, field)| field.try_into_ref::<u16>().ok().copied())
}

fn patch_u8_field(param_struct: &mut ParamStruct, field_hash: Hash40, new_value: u8) -> bool {
    for (hash, field) in param_struct.0.iter_mut() {
        if *hash != field_hash {
            continue;
        }
        if let Ok(value) = field.try_into_mut::<u8>() {
            *value = new_value;
            return true;
        }
        return false;
    }
    false
}

fn upsert_hash40_field(param_struct: &mut ParamStruct, field_hash: Hash40, new_value: Hash40) {
    if !patch_hash40_field(param_struct, field_hash, new_value) {
        param_struct
            .0
            .push((field_hash, ParamKind::Hash(new_value)));
    }
}

fn upsert_u8_field(param_struct: &mut ParamStruct, field_hash: Hash40, new_value: u8) {
    let mut patched = false;

    for (hash, field) in param_struct.0.iter_mut() {
        if *hash != field_hash {
            continue;
        }
        if let Ok(value) = field.try_into_mut::<u8>() {
            *value = new_value;
            patched = true;
        }
    }

    if !patched {
        param_struct.0.push((field_hash, ParamKind::U8(new_value)));
    }
}

fn copy_field_from_struct(
    source: &ParamStruct,
    target: &mut ParamStruct,
    field_hash: Hash40,
) -> bool {
    let Some((_, source_field)) = source.0.iter().find(|(hash, _)| *hash == field_hash) else {
        return false;
    };

    if let Some((_, target_field)) = target.0.iter_mut().find(|(hash, _)| *hash == field_hash) {
        *target_field = source_field.clone();
    } else {
        target.0.push((field_hash, source_field.clone()));
    }

    true
}

const CONDENSED_CARRIER_UI_CHARA: &str = "ui_chara_masterhand";
const CONDENSED_NATIVE_COLOR_COUNT: u8 = 8;
const CONDENSED_COLOR_MAP_FIELDS: [[&str; 3]; CONDENSED_NATIVE_COLOR_COUNT as usize] = [
    ["c00_index", "n00_index", "c00_group"],
    ["c01_index", "n01_index", "c01_group"],
    ["c02_index", "n02_index", "c02_group"],
    ["c03_index", "n03_index", "c03_group"],
    ["c04_index", "n04_index", "c04_group"],
    ["c05_index", "n05_index", "c05_group"],
    ["c06_index", "n06_index", "c06_group"],
    ["c07_index", "n07_index", "c07_group"],
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct CondensedNativePresentation {
    logical_index: u8,
    logical_boss: &'static str,
    source_ui_chara_id: &'static str,
    layout_template_ui_chara_id: &'static str,
    layout_template_id: &'static str,
    carrier_layout_id: &'static str,
    characall_field: &'static str,
    narration_label: &'static str,
}

/// The `ui_chara_db` row remains the Master Hand carrier. Each `ui_layout_db`
/// alias copies the boss-authored layout data when available, then uses the
/// carrier identity and local color. Galeem and Dharkon have no native layout
/// rows, so their 512x512 portraits use Master Hand's native boss layout.
const CONDENSED_NATIVE_PRESENTATIONS: [CondensedNativePresentation;
    CONDENSED_NATIVE_COLOR_COUNT as usize] = [
    CondensedNativePresentation {
        logical_index: 0,
        logical_boss: "master_hand",
        source_ui_chara_id: "ui_chara_masterhand",
        layout_template_ui_chara_id: "ui_chara_masterhand",
        layout_template_id: "ui_chara_master_hand_00",
        carrier_layout_id: "ui_chara_master_hand_00",
        characall_field: "characall_label_c00",
        narration_label: "vc_narration_characall_masterhand",
    },
    CondensedNativePresentation {
        logical_index: 1,
        logical_boss: "crazy_hand",
        source_ui_chara_id: "ui_chara_crazyhand",
        layout_template_ui_chara_id: "ui_chara_crazyhand",
        layout_template_id: "ui_chara_crazy_hand_00",
        carrier_layout_id: "ui_chara_master_hand_01",
        characall_field: "characall_label_c01",
        narration_label: "vc_narration_characall_crazyhand",
    },
    CondensedNativePresentation {
        logical_index: 2,
        logical_boss: "dharkon",
        source_ui_chara_id: "ui_chara_darz",
        layout_template_ui_chara_id: "ui_chara_masterhand",
        layout_template_id: "ui_chara_master_hand_00",
        carrier_layout_id: "ui_chara_master_hand_02",
        characall_field: "characall_label_c02",
        narration_label: "vc_narration_characall_darz",
    },
    CondensedNativePresentation {
        logical_index: 3,
        logical_boss: "galeem",
        source_ui_chara_id: "ui_chara_kiila",
        layout_template_ui_chara_id: "ui_chara_masterhand",
        layout_template_id: "ui_chara_master_hand_00",
        carrier_layout_id: "ui_chara_master_hand_03",
        characall_field: "characall_label_c03",
        narration_label: "vc_narration_characall_kiila",
    },
    CondensedNativePresentation {
        logical_index: 4,
        logical_boss: "ganon_boss",
        source_ui_chara_id: "ui_chara_ganonboss",
        layout_template_ui_chara_id: "ui_chara_ganonboss",
        layout_template_id: "ui_chara_ganonboss_00",
        carrier_layout_id: "ui_chara_master_hand_04",
        characall_field: "characall_label_c04",
        narration_label: "vc_narration_characall_ganonboss",
    },
    CondensedNativePresentation {
        logical_index: 5,
        logical_boss: "rathalos",
        source_ui_chara_id: "ui_chara_lioleus",
        layout_template_ui_chara_id: "ui_chara_lioleus",
        layout_template_id: "ui_chara_lioleus_00",
        carrier_layout_id: "ui_chara_master_hand_05",
        characall_field: "characall_label_c05",
        narration_label: "vc_narration_characall_lioleus",
    },
    CondensedNativePresentation {
        logical_index: 6,
        logical_boss: "dracula",
        source_ui_chara_id: "ui_chara_dracula",
        layout_template_ui_chara_id: "ui_chara_dracula",
        layout_template_id: "ui_chara_dracula_00",
        carrier_layout_id: "ui_chara_master_hand_06",
        characall_field: "characall_label_c06",
        narration_label: "vc_narration_characall_dracula",
    },
    CondensedNativePresentation {
        logical_index: 7,
        logical_boss: "marx",
        source_ui_chara_id: "ui_chara_marx",
        layout_template_ui_chara_id: "ui_chara_marx",
        layout_template_id: "ui_chara_marx_00",
        carrier_layout_id: "ui_chara_master_hand_07",
        characall_field: "characall_label_c07",
        narration_label: "vc_narration_characall_marx",
    },
];
const CONDENSED_SUPPRESSED_UI_CHARA: [&str; 9] = [
    "ui_chara_crazyhand",
    "ui_chara_darz",
    "ui_chara_kiila",
    "ui_chara_ganonboss",
    "ui_chara_lioleus",
    "ui_chara_dracula",
    "ui_chara_marx",
    "ui_chara_mewtwo_masterhand",
    "ui_chara_galleom",
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CondensedCssRole {
    Carrier,
    Suppressed,
    Untouched,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CondensedCarrierError {
    MissingCarrierNameId,
    InvalidCarrierNameId,
    MissingMarioColorMapField(&'static str),
    InvalidMarioColorMapField(&'static str),
}

#[inline]
fn condensed_css_enabled() -> bool {
    CONFIG.options.condense_bosses_into_single_slot()
}

fn condensed_css_role(ui_chara_id: &str) -> CondensedCssRole {
    if ui_chara_id == CONDENSED_CARRIER_UI_CHARA {
        CondensedCssRole::Carrier
    } else if CONDENSED_SUPPRESSED_UI_CHARA.contains(&ui_chara_id) {
        CondensedCssRole::Suppressed
    } else {
        CondensedCssRole::Untouched
    }
}

fn apply_condensed_css_visibility_for_mode(
    charroot: &mut ParamStruct,
    ui_chara_id: &str,
    condensed_enabled: bool,
) {
    if !condensed_enabled {
        return;
    }

    match condensed_css_role(ui_chara_id) {
        CondensedCssRole::Carrier => {
            patch_bool_field(charroot, to_hash40("can_select"), true);
            patch_bool_field(charroot, to_hash40("is_hidden_boss"), false);
        }
        CondensedCssRole::Suppressed => {
            // Keep the database row and selector intact for Amiibo and runtime
            // identity consumers; only remove its standalone CSS availability.
            patch_bool_field(charroot, to_hash40("can_select"), false);
            patch_bool_field(charroot, to_hash40("is_hidden_boss"), true);
            patch_i8_field(charroot, to_hash40("disp_order"), -1);
            patch_i8_field(charroot, to_hash40("skill_list_order"), -1);
        }
        CondensedCssRole::Untouched => {}
    }
}

fn apply_condensed_css_visibility(charroot: &mut ParamStruct, ui_chara_id: &str) {
    apply_condensed_css_visibility_for_mode(charroot, ui_chara_id, condensed_css_enabled());
}

fn configure_condensed_masterhand_carrier_fields(
    charroot: &mut ParamStruct,
    mario_row: &ParamStruct,
) -> Result<usize, CondensedCarrierError> {
    let name_id_hash = to_hash40("name_id");
    let Some((_, name_id_field)) = charroot.0.iter().find(|(hash, _)| *hash == name_id_hash) else {
        return Err(CondensedCarrierError::MissingCarrierNameId);
    };
    let Ok(name_id) = name_id_field.try_into_ref::<String>() else {
        return Err(CondensedCarrierError::InvalidCarrierNameId);
    };
    if name_id != "masterhand" {
        return Err(CondensedCarrierError::InvalidCarrierNameId);
    }

    // `color_num` alone does not create selectable variations. A normal
    // fighter row also maps each local slot through cXX_index, nXX_index, and
    // cXX_group. Copy Mario's complete named maps so the Mario-backed carrier
    // produces real host colors 0..7 without requesting any new costumes.
    for field_name in CONDENSED_COLOR_MAP_FIELDS.iter().flatten() {
        let field_hash = to_hash40(field_name);
        let Some((_, value)) = mario_row.0.iter().find(|(hash, _)| *hash == field_hash) else {
            return Err(CondensedCarrierError::MissingMarioColorMapField(field_name));
        };
        if value.try_into_ref::<u8>().is_err() {
            return Err(CondensedCarrierError::InvalidMarioColorMapField(field_name));
        }
    }
    for field_name in CONDENSED_COLOR_MAP_FIELDS.iter().flatten() {
        let copied = copy_field_from_struct(mario_row, charroot, to_hash40(field_name));
        debug_assert!(copied);
    }

    // `cXX_index`/`cXX_group` must stay Mario's so the carrier loads real,
    // existing costume resources. `nXX_index` must NOT: it is the per-color
    // NAME index, and Mario uses one name for all eight costumes, so copying
    // his map made every slot resolve to name index 00 and display
    // "MASTER HAND" on every skin. Identity mapping gives slot N the
    // `nam_chr*_NN_masterhand` label that already exists in msg_name.msbt.
    for (slot, field_names) in CONDENSED_COLOR_MAP_FIELDS.iter().enumerate() {
        upsert_u8_field(charroot, to_hash40(field_names[1]), slot as u8);
    }

    apply_condensed_css_visibility_for_mode(charroot, CONDENSED_CARRIER_UI_CHARA, true);
    // Keep the native string-valued `name_id = "masterhand"` untouched. Smash
    // uses it for both masterhand_XX resources and per-color MSBT labels.
    upsert_u8_field(
        charroot,
        to_hash40("color_num"),
        CONDENSED_NATIVE_COLOR_COUNT,
    );

    // Native colors remain visually Ganon and Master Hand. Shield selects
    // Galleom from Ganon or WOL Master Hand from Master Hand without asking Mario to
    // load an unsupported ninth/tenth costume.
    for presentation in CONDENSED_NATIVE_PRESENTATIONS {
        upsert_hash40_field(
            charroot,
            to_hash40(presentation.characall_field),
            to_hash40(presentation.narration_label),
        );
    }
    Ok(CONDENSED_COLOR_MAP_FIELDS.len() * CONDENSED_COLOR_MAP_FIELDS[0].len())
}

/// Condensed mode owns the carrier/suppression rows regardless of legacy CSS
/// options. With condensed mode off, this is exactly the previous CUSTOM_CSS
/// and per-boss/Amiibo callback policy. Character-name detection is selection
/// logic and deliberately cannot disable these named PRC mutations.
fn should_install_item_boss_css_callback(
    condensed_enabled: bool,
    custom_css: bool,
    _detect_character_name: bool,
    individual_css_enabled: bool,
    amiibo_mapping_enabled: bool,
) -> bool {
    condensed_enabled || (!custom_css && (individual_css_enabled || amiibo_mapping_enabled))
}

fn should_install_giga_bowser_css_callback(
    custom_css: bool,
    giga_bowser_css: bool,
    amiibo_mapping_enabled: bool,
) -> bool {
    !custom_css && (giga_bowser_css || amiibo_mapping_enabled)
}

#[arc_callback]
fn callback_amiibo(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    // ARCropolis may invoke a file callback more than once. Always start from
    // the original file so appends/remaps are deterministic and idempotent.
    let Some(original_size) = load_original_file(hash, &mut data) else {
        crate::boss_log!("[PB][Amiibo] callback could not load the original file; failing closed");
        return None;
    };
    crate::boss_log!(
        "[PB][Amiibo] callback invoked file_hash=0x{:010x} original_size={} callback_buffer_len={}",
        hash,
        original_size,
        data.len()
    );
    if original_size > data.len() {
        crate::boss_log!(
            "[PB][Amiibo] original file size {} exceeds callback buffer {}; failing closed",
            original_size,
            data.len()
        );
        return None;
    }
    let mut root = {
        let mut reader = std::io::Cursor::new(&data[..original_size]);
        match prc::read_stream(&mut reader) {
            Ok(root) => root,
            Err(error) => {
                crate::boss_log!("[PB][Amiibo] failed to parse ui_amiibo_db.prc: {:?}", error);
                return None;
            }
        }
    };
    let db_root_hash = to_hash40("db_root");
    let root_field_count = root.0.len();
    let Some((_, db_root)) = root.0.iter_mut().find(|(key, _)| *key == db_root_hash) else {
        crate::boss_log!("[PB][Amiibo] ui_amiibo_db.prc has no db_root field");
        return Some(original_size);
    };
    let Ok(db_root_list) = db_root.try_into_mut::<ParamList>() else {
        crate::boss_log!("[PB][Amiibo] ui_amiibo_db.prc db_root is not a list");
        return Some(original_size);
    };

    let mappings = amiibo::configured_mappings();
    if mappings.is_empty() {
        return Some(original_size);
    }

    let ui_amiibo_id_hash = to_hash40("ui_amiibo_id");
    let ui_chara_id_hash = to_hash40("ui_chara_id");
    let default_color_hash = to_hash40("default_color");
    let nfp_character_id_upper_hash = to_hash40("nfp_character_id_upper");
    let nfp_character_id_lower_hash = to_hash40("nfp_character_id_lower");
    let nfp_numbering_id_hash = to_hash40("nfp_numbering_id");
    let enable_unknown_numbering_id_hash = to_hash40("enable_unknown_numbering_id");
    let unknown_bool_hash = Hash40(0x13a2_6bd6a0);
    let is_valid_hash = to_hash40("is_valid");

    // The upper half may legitimately be zero. The complete figure ID is
    // still unambiguous when the lower database portion is nonzero.
    let mut existing_records: Vec<(usize, u64, u64)> = Vec::new();
    let template = amiibo::select_amiibo_template(db_root_list);
    let schema_record_count = amiibo::amiibo_schema_record_count(db_root_list);
    let structural_fingerprint = amiibo::amiibo_structural_fingerprint(db_root_list);
    let mut schema_candidates = Vec::new();

    for (index, param) in db_root_list.0.iter().enumerate() {
        let Ok(record) = param.try_into_ref::<ParamStruct>() else {
            continue;
        };
        let Some(ui_amiibo_id) = read_hash40_field(record, ui_amiibo_id_hash) else {
            continue;
        };
        let Some(upper) = read_u16_field(record, nfp_character_id_upper_hash) else {
            continue;
        };
        let full_tag_id = ((upper as u64) << 48) | (ui_amiibo_id.0 & 0x0000_00FF_FFFF_FFFF);
        existing_records.push((index, ui_amiibo_id.0, full_tag_id));

        if amiibo::is_verified_amiibo_record(record) && schema_candidates.len() < 4 {
            schema_candidates.push((
                index,
                ui_amiibo_id.0,
                upper,
                read_u8_field(record, nfp_character_id_lower_hash).unwrap_or(0),
                read_u16_field(record, nfp_numbering_id_hash).unwrap_or(0),
                read_bool_field(record, enable_unknown_numbering_id_hash).unwrap_or(false),
                read_bool_field(record, unknown_bool_hash).unwrap_or(false),
                read_hash40_field(record, ui_chara_id_hash)
                    .map(|value| value.0)
                    .unwrap_or(0),
                read_bool_field(record, is_valid_hash).unwrap_or(false),
            ));
        }
    }

    let legacy_template_present = existing_records
        .iter()
        .any(|(_, ui_id, full_id)| *ui_id == 0x0361_1202 && (*full_id >> 48) == 8455);
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][Amiibo] runtime_db root_fields={} records={} schema_valid_records={} template_index={:?} legacy_fixture_sentinel_present={} fingerprint=0x{:016x}",
            root_field_count,
            db_root_list.0.len(),
            schema_record_count,
            template.as_ref().map(|(index, _)| *index),
            legacy_template_present,
            structural_fingerprint
        );
        crate::boss_log!(
            "[PB][Amiibo] runtime_db schema=ui_amiibo_id:Hash40,ui_chara_id:Hash40,is_valid:Bool,0x13a26bd6a0:Bool,nfp_numbering_id:U16,default_color:U8,enable_unknown_numbering_id:Bool,nfp_character_id_upper:U16,nfp_character_id_lower:U8 candidates={:?}",
            schema_candidates
        );
        for (
            index,
            ui_amiibo_id,
            upper,
            lower,
            numbering,
            unknown_numbering,
            unknown_bool,
            ui_chara,
            is_valid,
        ) in &schema_candidates
        {
            crate::boss_log!(
                "[PB][AmiiboKeyAudit] source=runtime_existing index={} ui_amiibo_id=0x{:010x} upper=0x{:04x} lower=0x{:02x} numbering=0x{:04x} unknown_numbering={} unknown_bool={} is_valid={} ui_chara=0x{:010x}",
                index,
                ui_amiibo_id,
                upper,
                lower,
                numbering,
                unknown_numbering,
                unknown_bool,
                is_valid,
                ui_chara
            );
        }
    }

    if template.is_none() {
        crate::boss_log!(
            "[PB][Amiibo] ui_amiibo_db.prc has no schema-valid nine-field template; append mappings will be rejected without modifying the original bytes"
        );
    }

    let original_entries = db_root_list.0.len();
    let mut added = 0usize;
    let mut remapped = 0usize;
    let mut new_ui_amiibo_ids = Vec::new();
    let mut new_tag_ids = Vec::new();
    let mut remapped_tag_ids = Vec::new();

    for mapping in mappings {
        if crate::debug::enabled() {
            if let Some(key) = mapping.nfp_match_key {
                crate::boss_log!(
                    "[PB][AmiiboKeyAudit] source=config boss={} figure_id=0x{:016x} ui_amiibo_id=0x{:010x} upper=0x{:04x} lower=0x{:02x} numbering=0x{:04x} unknown_numbering={} unknown_bool=template ui_chara={} match_mode=explicit_private_virtual",
                    mapping.identity.key,
                    mapping.tag_id,
                    mapping.ui_amiibo_id,
                    key.character_id_upper,
                    key.character_id_lower,
                    key.numbering_id,
                    key.enable_unknown_numbering_id,
                    mapping.identity.ui_chara_id
                );
            } else {
                crate::boss_log!(
                    "[PB][AmiiboKeyAudit] source=config boss={} figure_id=0x{:016x} ui_amiibo_id=0x{:010x} upper=0x{:04x} lower=template numbering=template unknown_numbering=template unknown_bool=template ui_chara={} match_mode=preserve_template",
                    mapping.identity.key,
                    mapping.tag_id,
                    mapping.ui_amiibo_id,
                    mapping.nfp_character_id_upper,
                    mapping.identity.ui_chara_id
                );
            }
        }
        if mapping.remap_existing {
            let matching_records: Vec<usize> = existing_records
                .iter()
                .filter(|(_, _, full_tag_id)| *full_tag_id == mapping.tag_id)
                .map(|(index, _, _)| *index)
                .collect();

            if matching_records.len() != 1 {
                let lower_collision = existing_records
                    .iter()
                    .any(|(_, ui_amiibo_id, _)| *ui_amiibo_id == mapping.ui_amiibo_id);
                crate::boss_log!(
                    "[PB][Amiibo] rejected {}: remap_existing=true requires exactly one existing record for full figure ID 0x{:016x}; matching_records={} lower_collision={}",
                    mapping.identity.name,
                    mapping.tag_id,
                    matching_records.len(),
                    lower_collision
                );
                continue;
            }
            if remapped_tag_ids.contains(&mapping.tag_id) {
                crate::boss_log!(
                    "[PB][Amiibo] rejected {}: full tag ID 0x{:016x} was already remapped by another boss",
                    mapping.identity.name,
                    mapping.tag_id
                );
                continue;
            }

            let record_index = matching_records[0];
            let Some(param) = db_root_list.0.get_mut(record_index) else {
                crate::boss_log!(
                    "[PB][Amiibo] rejected {}: existing record index {} disappeared",
                    mapping.identity.name,
                    record_index
                );
                continue;
            };
            let Ok(record) = param.try_into_mut::<ParamStruct>() else {
                crate::boss_log!(
                    "[PB][Amiibo] rejected {}: existing record index {} is not a struct",
                    mapping.identity.name,
                    record_index
                );
                continue;
            };
            let original_ui_chara_id = read_hash40_field(record, ui_chara_id_hash);
            let protected_official_mapping = (mapping.identity.key == "giga_bowser"
                && original_ui_chara_id == Some(to_hash40("ui_chara_koopa")))
                || (mapping.identity.key == "ganon_boss"
                    && original_ui_chara_id == Some(to_hash40("ui_chara_ganon")));
            if protected_official_mapping {
                crate::boss_log!(
                    "[PB][Amiibo] rejected {}: remap_existing cannot replace the protected official Bowser/Ganondorf mapping",
                    mapping.identity.name
                );
                continue;
            }
            let patched_ui_chara_id = patch_hash40_field(
                record,
                ui_chara_id_hash,
                to_hash40(mapping.identity.ui_chara_id),
            );
            let patched_default_color =
                patch_u8_field(record, default_color_hash, mapping.default_color);
            if !(patched_ui_chara_id && patched_default_color) {
                crate::boss_log!(
                    "[PB][Amiibo] rejected {}: existing record fields were incomplete ui_chara_id={} default_color={}",
                    mapping.identity.name,
                    patched_ui_chara_id,
                    patched_default_color
                );
                continue;
            }

            remapped_tag_ids.push(mapping.tag_id);
            remapped += 1;
            crate::boss_log!(
                "[PB][Amiibo] remapped existing {} mode=remap tag=0x{:016x} ui_amiibo_id=0x{:010x} original_ui_chara_hash={} result_ui_chara_id={} result_ui_chara_hash=0x{:010x} nfp_character_id_upper={} default_color={} record_index={}",
                mapping.identity.name,
                mapping.tag_id,
                mapping.ui_amiibo_id,
                original_ui_chara_id
                    .map(|hash| format!("0x{:010x}", hash.0))
                    .unwrap_or_else(|| "<missing>".to_string()),
                mapping.identity.ui_chara_id,
                to_hash40(mapping.identity.ui_chara_id).0,
                mapping.nfp_character_id_upper,
                mapping.default_color,
                record_index
            );
            amiibo_preview::log_identity_boundary(
                &mapping,
                "remap",
                original_ui_chara_id.map(|hash| hash.0),
            );
            continue;
        }

        let exact_collision = existing_records
            .iter()
            .any(|(_, _, full_tag_id)| *full_tag_id == mapping.tag_id)
            || new_tag_ids.contains(&mapping.tag_id);
        if exact_collision {
            crate::boss_log!(
                "[PB][Amiibo] rejected {}: exact figure ID 0x{:016x} already exists; set remap_existing=true only when intentionally replacing that exact record",
                mapping.identity.name,
                mapping.tag_id
            );
            continue;
        }

        let lower_collision = existing_records
            .iter()
            .map(|(_, ui_amiibo_id, _)| *ui_amiibo_id)
            .chain(new_ui_amiibo_ids.iter().copied())
            .any(|id| id == mapping.ui_amiibo_id);
        if lower_collision {
            crate::boss_log!(
                "[PB][Amiibo] rejected {}: lower database ID 0x{:010x} collides with another record; the exact figure ID is absent but appending would make the lookup ambiguous",
                mapping.identity.name,
                mapping.ui_amiibo_id
            );
            continue;
        }

        if new_tag_ids.contains(&mapping.tag_id) {
            crate::boss_log!(
                "[PB][Amiibo] rejected {}: full tag ID 0x{:016x} collides with another configured mapping",
                mapping.identity.name,
                mapping.tag_id
            );
            continue;
        }

        let Some((template_index, template)) = template.as_ref() else {
            crate::boss_log!(
                "[PB][Amiibo] rejected {}: append mode has no verified nine-field template",
                mapping.identity.name
            );
            continue;
        };
        let Some(record) = amiibo::prepare_append_record(
            template,
            mapping.ui_amiibo_id,
            to_hash40(mapping.identity.ui_chara_id),
            mapping.nfp_character_id_upper,
            mapping.default_color,
            mapping.nfp_match_key,
        ) else {
            crate::boss_log!(
                "[PB][Amiibo] rejected {}: selected schema-valid template could not be patched",
                mapping.identity.name,
            );
            continue;
        };

        db_root_list.0.push(ParamKind::Struct(record));
        new_ui_amiibo_ids.push(mapping.ui_amiibo_id);
        new_tag_ids.push(mapping.tag_id);
        added += 1;

        crate::boss_log!(
            "[PB][Amiibo] added {} mode=append template_index={} tag=0x{:016x} ui_amiibo_id=0x{:010x} original_ui_chara_id=<none> result_ui_chara_id={} result_ui_chara_hash=0x{:010x} nfp_character_id_upper={} default_color={} explicit_nfp_match={:?}",
            mapping.identity.name,
            template_index,
            mapping.tag_id,
            mapping.ui_amiibo_id,
            mapping.identity.ui_chara_id,
            to_hash40(mapping.identity.ui_chara_id).0,
            mapping.nfp_character_id_upper,
            mapping.default_color,
            mapping.nfp_match_key
        );
        amiibo_preview::log_identity_boundary(&mapping, "append", None);
        if crate::debug::enabled() {
            let appended_record = db_root_list
                .0
                .last()
                .and_then(|param| param.try_into_ref::<ParamStruct>().ok());
            if let Some(record) = appended_record {
                crate::boss_log!(
                    "[PB][Amiibo] append_record_fields index={} ui_amiibo_id=0x{:010x} ui_chara_id=0x{:010x} is_valid={} unknown_bool={} nfp_numbering_id={} default_color={} enable_unknown_numbering_id={} nfp_character_id_upper=0x{:04x} nfp_character_id_lower={}",
                    db_root_list.0.len() - 1,
                    read_hash40_field(record, ui_amiibo_id_hash)
                        .map(|value| value.0)
                        .unwrap_or(0),
                    read_hash40_field(record, ui_chara_id_hash)
                        .map(|value| value.0)
                        .unwrap_or(0),
                    read_bool_field(record, is_valid_hash).unwrap_or(false),
                    read_bool_field(record, unknown_bool_hash).unwrap_or(false),
                    read_u16_field(record, nfp_numbering_id_hash).unwrap_or(0),
                    read_u8_field(record, default_color_hash).unwrap_or(0),
                    read_bool_field(record, enable_unknown_numbering_id_hash).unwrap_or(false),
                    read_u16_field(record, nfp_character_id_upper_hash).unwrap_or(0),
                    read_u8_field(record, nfp_character_id_lower_hash).unwrap_or(0),
                );
            }
        }
    }

    if added == 0 && remapped == 0 {
        return Some(original_size);
    }

    let written_size = {
        let mut writer = std::io::Cursor::new(&mut *data);
        write_stream(&mut writer, &root).map(|_| writer.position() as usize)
    };
    let written_size = match written_size {
        Ok(written_size) => written_size,
        Err(error) => {
            if load_original_file(hash, &mut data).is_none() {
                crate::boss_log!(
                    "[PB][Amiibo] failed to restore the original ui_amiibo_db.prc after a write error; failing closed"
                );
                return None;
            }
            crate::boss_log!(
                "[PB][Amiibo] failed to write patched ui_amiibo_db.prc: {:?}",
                error
            );
            return Some(original_size);
        }
    };

    crate::boss_log!(
        "[PB][Amiibo] ui_amiibo_db.prc entries {} -> {} (added={} remapped={})",
        original_entries,
        original_entries + added,
        added,
        remapped
    );
    Some(written_size)
}

#[arc_callback]
fn callback_koopag_layout(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let original_size = reader.position() as usize;
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();

    let ui_layout_id_hash = to_hash40("ui_layout_id");
    let ui_chara_id_hash = to_hash40("ui_chara_id");
    let source_layout_id = to_hash40("ui_chara_koopa_00");
    let target_layout_id = to_hash40("ui_chara_koopag_00");
    let target_chara_id = to_hash40("ui_chara_koopag");

    if db_root_list
        .0
        .iter()
        .any(|param| struct_hash40_field_matches(param, ui_layout_id_hash, target_layout_id))
    {
        crate::boss_log!("[PB][CSSLayout] kept existing ui_layout_db row ui_chara_koopag_00");
        return Some(original_size);
    }

    let Some(source_layout) = db_root_list
        .0
        .iter()
        .find(|param| struct_hash40_field_matches(param, ui_layout_id_hash, source_layout_id))
        .and_then(|param| param.try_into_ref::<ParamStruct>().ok())
        .cloned()
    else {
        crate::boss_log!(
            "[PB][CSSLayout] ui_layout_db missing Bowser template row ui_chara_koopa_00"
        );
        return Some(original_size);
    };

    let mut cloned_layout = source_layout;
    let patched_layout_id =
        patch_hash40_field(&mut cloned_layout, ui_layout_id_hash, target_layout_id);
    let patched_chara_id =
        patch_hash40_field(&mut cloned_layout, ui_chara_id_hash, target_chara_id);

    if !patched_layout_id || !patched_chara_id {
        crate::boss_log!(
            "[PB][CSSLayout] ui_layout_db failed to patch cloned Koopag layout row layout_id={} chara_id={}",
            patched_layout_id,
            patched_chara_id
        );
        return Some(original_size);
    }

    db_root_list.0.push(ParamKind::Struct(cloned_layout));
    crate::boss_log!(
        "[PB][CSSLayout] appended ui_layout_db row ui_chara_koopag_00 from Bowser template"
    );

    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    Some(writer.position() as usize)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum CondensedLayoutError {
    MissingLayoutTemplate(&'static str),
    MissingNamedFields(&'static str),
}

fn ensure_condensed_carrier_layouts(
    db_root_list: &mut ParamList,
) -> Result<usize, CondensedLayoutError> {
    let ui_layout_id_hash = to_hash40("ui_layout_id");
    let ui_chara_id_hash = to_hash40("ui_chara_id");
    let chara_color_hash = to_hash40("chara_color");

    // Prepare all rows before mutating the database so a missing boss layout
    // cannot leave a partially rewritten carrier.
    let mut prepared_layouts = Vec::with_capacity(CONDENSED_NATIVE_PRESENTATIONS.len());
    for presentation in CONDENSED_NATIVE_PRESENTATIONS {
        let layout_template_id = to_hash40(presentation.layout_template_id);
        let Some(mut cloned_layout) = db_root_list
            .0
            .iter()
            .find(|param| struct_hash40_field_matches(param, ui_layout_id_hash, layout_template_id))
            .and_then(|param| param.try_into_ref::<ParamStruct>().ok())
            .cloned()
        else {
            return Err(CondensedLayoutError::MissingLayoutTemplate(
                presentation.layout_template_id,
            ));
        };

        let target_layout_id = to_hash40(presentation.carrier_layout_id);
        let template_chara_id = to_hash40(presentation.layout_template_ui_chara_id);
        if read_hash40_field(&cloned_layout, ui_chara_id_hash) != Some(template_chara_id)
            || !patch_hash40_field(&mut cloned_layout, ui_layout_id_hash, target_layout_id)
            || !patch_hash40_field(
                &mut cloned_layout,
                ui_chara_id_hash,
                to_hash40(CONDENSED_CARRIER_UI_CHARA),
            )
            || !patch_u8_field(
                &mut cloned_layout,
                chara_color_hash,
                presentation.logical_index,
            )
        {
            return Err(CondensedLayoutError::MissingNamedFields(
                presentation.layout_template_id,
            ));
        }

        prepared_layouts.push((target_layout_id, cloned_layout));
    }

    for (target_layout_id, cloned_layout) in prepared_layouts {
        if let Some(target_param) = db_root_list
            .0
            .iter_mut()
            .find(|param| struct_hash40_field_matches(param, ui_layout_id_hash, target_layout_id))
        {
            *target_param = ParamKind::Struct(cloned_layout);
        } else {
            db_root_list.0.push(ParamKind::Struct(cloned_layout));
        }
    }

    Ok(CONDENSED_NATIVE_PRESENTATIONS.len())
}

#[arc_callback]
fn callback_condensed_boss_layout(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let original_size = reader.position() as usize;
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();

    let carrier_layout_count = match ensure_condensed_carrier_layouts(db_root_list) {
        Ok(count) => count,
        Err(reason) => {
            crate::boss_log!("[PB][CondensedBossCSS] ui_layout_db failure={:?}", reason);
            return Some(original_size);
        }
    };

    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    crate::boss_log!(
        "[PB][CondensedBossCSS] carrier_layouts={} presentation=boss_layout_data+carrier_identity_assets asset_name_id=masterhand native_colors=0..7",
        carrier_layout_count
    );
    for presentation in CONDENSED_NATIVE_PRESENTATIONS {
        crate::boss_log!(
            "[PB][CondensedBossCSS] logical_index={} logical_boss={} carrier_layout={} layout_template={} layout_template_ui_chara={} source_ui_chara={} asset_ui_chara={} chara_color={} asset_color={:02} asset_families=chara_1|chara_2|chara_4|chara_7",
            presentation.logical_index,
            presentation.logical_boss,
            presentation.carrier_layout_id,
            presentation.layout_template_id,
            presentation.layout_template_ui_chara_id,
            presentation.source_ui_chara_id,
            CONDENSED_CARRIER_UI_CHARA,
            presentation.logical_index,
            presentation.logical_index
        );
    }
    Some(writer.position() as usize)
}

// Giga Bowser

#[arc_callback]
fn callback_koopag(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let original_size = reader.position() as usize;

    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();

    let ui_chara_id_hash = to_hash40("ui_chara_id");
    let target_chara_id = to_hash40("ui_chara_koopag");
    let fighter_kind_hash = to_hash40("fighter_kind");
    let fighter_kind_koopag = to_hash40("fighter_kind_koopag");

    let Some(target_index) = db_root_list
        .0
        .iter()
        .position(|param| struct_hash40_field_matches(param, ui_chara_id_hash, target_chara_id))
    else {
        crate::boss_log!("[PB][CSSChara] ui_chara_db missing target row ui_chara_koopag");
        return Some(original_size);
    };

    let Some(mut target_row) = db_root_list
        .0
        .get(target_index)
        .and_then(|param| param.try_into_ref::<ParamStruct>().ok())
        .cloned()
    else {
        crate::boss_log!(
            "[PB][CSSChara] ui_chara_db target row ui_chara_koopag was not a ParamStruct"
        );
        return Some(original_size);
    };

    // 3.1.0 cloned Bowser's entire CSS row onto Giga Bowser, then tried to
    // put fighter_kind_koopag back. CSS portraits still loaded, but the VS
    // screen asked for Bowser costume slot 8 (`color_start_index` /
    // `original_ui_chara_hash = ui_chara_koopa`) on a fighter that only has
    // koopag c00. Hardware waits there forever; emulator often does not.
    // Patch the genuine koopag row in place and leave his costume identity
    // alone.
    let existing_fighter_kind = read_hash40_field(&target_row, fighter_kind_hash);
    let fighter_kind_source = if existing_fighter_kind == Some(fighter_kind_koopag) {
        "target_row"
    } else if patch_hash40_field(&mut target_row, fighter_kind_hash, fighter_kind_koopag) {
        "explicit_fighter_kind_koopag"
    } else {
        "unresolved"
    };

    patch_bool_field(&mut target_row, to_hash40("can_select"), true);
    patch_bool_field(&mut target_row, to_hash40("is_boss"), true);
    patch_bool_field(&mut target_row, to_hash40("is_hidden_boss"), false);
    patch_i8_field(&mut target_row, to_hash40("disp_order"), 15);
    patch_i8_field(&mut target_row, to_hash40("skill_list_order"), 15);
    patch_i8_field(&mut target_row, to_hash40("save_no"), -1);
    patch_hash40_field(
        &mut target_row,
        to_hash40("characall_label_c00"),
        to_hash40("vc_narration_characall_koopa"),
    );
    patch_hash40_field(
        &mut target_row,
        to_hash40("ui_series_id"),
        to_hash40("ui_series_mario"),
    );
    patch_hash40_field(
        &mut target_row,
        to_hash40("fighter_type"),
        to_hash40("fighter_type_normal"),
    );
    patch_css_selector_fields(&mut target_row, "ui_chara_koopag", 0x18E);

    db_root_list.0[target_index] = ParamKind::Struct(target_row);
    crate::boss_log!(
        "[PB][CSSChara] patched ui_chara_koopag in place fighter_kind_source={} color_start_index=unchanged original_ui_chara_hash=unchanged save_no=-1",
        fighter_kind_source
    );
    amiibo_preview::log_ui_chara_db_boundary(
        "ui_chara_koopag",
        "fighter_kind_koopag",
        "fighter:koopag",
    );

    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    Some(writer.position() as usize)
}

// Master Hand

#[arc_callback]
fn callback_masterhand(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let original_size = reader.position() as usize;
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let ui_chara_id_hash = to_hash40("ui_chara_id");
    let carrier_index = db_root_list
        .0
        .iter()
        .position(|param| {
            struct_hash40_field_matches(
                param,
                ui_chara_id_hash,
                to_hash40(CONDENSED_CARRIER_UI_CHARA),
            )
        })
        .expect("ui_chara_db must contain ui_chara_masterhand");
    let mut charroot = db_root_list.0[carrier_index]
        .try_into_ref::<ParamStruct>()
        .unwrap()
        .clone();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 118;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 87;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() =
                to_hash40("vc_narration_characall_masterhand");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    patch_css_selector_fields(&mut charroot, "ui_chara_masterhand", 0x160);
    if condensed_css_enabled() {
        let mario_row = db_root_list
            .0
            .iter()
            .find(|param| {
                struct_hash40_field_matches(param, ui_chara_id_hash, to_hash40("ui_chara_mario"))
            })
            .and_then(|param| param.try_into_ref::<ParamStruct>().ok())
            .cloned();

        match mario_row {
            Some(mario_row) => {
                match configure_condensed_masterhand_carrier_fields(&mut charroot, &mario_row) {
                    Ok(copied_color_fields) => {
                        if crate::debug::enabled() {
                            let mut costume_indices = [u8::MAX; 8];
                            let mut local_indices = [u8::MAX; 8];
                            let mut color_groups = [u8::MAX; 8];
                            for (color, field_names) in
                                CONDENSED_COLOR_MAP_FIELDS.iter().enumerate()
                            {
                                costume_indices[color] =
                                    read_u8_field(&charroot, to_hash40(field_names[0]))
                                        .unwrap_or(u8::MAX);
                                local_indices[color] =
                                    read_u8_field(&charroot, to_hash40(field_names[1]))
                                        .unwrap_or(u8::MAX);
                                color_groups[color] =
                                    read_u8_field(&charroot, to_hash40(field_names[2]))
                                        .unwrap_or(u8::MAX);
                            }
                            crate::boss_log!(
                                "[PB][CondensedBossCSS] carrier=ui_chara_masterhand name_id=masterhand native_colors={} copied_mario_color_map_fields={} c_indices={:?} n_indices={:?} c_groups={:?} galleom=ganon_color+shield wol_master_hand=master_hand_color+shield backing=fighter_kind_mario",
                                CONDENSED_NATIVE_COLOR_COUNT,
                                copied_color_fields,
                                costume_indices,
                                local_indices,
                                color_groups
                            );
                        }
                    }
                    Err(reason) => {
                        crate::boss_log!(
                            "[PB][CondensedBossCSS] carrier=ui_chara_masterhand action=preserve_traditional_master_hand reason={:?}",
                            reason
                        );
                    }
                }
            }
            None => {
                crate::boss_log!(
                    "[PB][CondensedBossCSS] carrier=ui_chara_masterhand action=preserve_traditional_master_hand reason=missing_ui_chara_mario_template"
                );
            }
        }
    }
    db_root_list.0[carrier_index] = ParamKind::Struct(charroot);
    amiibo_preview::log_ui_chara_db_boundary(
        "ui_chara_masterhand",
        "fighter_kind_mario",
        "item:masterhand",
    );
    let mut writer = std::io::Cursor::new(data);
    if let Err(error) = write_stream(&mut writer, &root) {
        crate::boss_log!(
            "[PB][CondensedBossCSS] ui_chara_db write failed: {:?}",
            error
        );
        return Some(original_size);
    }
    Some(writer.position() as usize)
}

// Crazy Hand

#[arc_callback]
fn callback_crazyhand(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_crazyhand")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 119;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 88;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() =
                to_hash40("vc_narration_characall_crazyhand");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_crazyhand", 0x169);
    apply_condensed_css_visibility(charroot, "ui_chara_crazyhand");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Dharkon

#[arc_callback]
fn callback_dharkon(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_darz")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 120;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 89;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_darz");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_darz", 0x19A);
    apply_condensed_css_visibility(charroot, "ui_chara_darz");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Galeem

#[arc_callback]
fn callback_galeem(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_kiila")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 121;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 90;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_kiila");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_kiila", 0x18F);
    apply_condensed_css_visibility(charroot, "ui_chara_kiila");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Marx

#[arc_callback]
fn callback_marx(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_marx")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 122;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 91;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_marx");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_kirby");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_marx", 0x180);
    apply_condensed_css_visibility(charroot, "ui_chara_marx");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Ganon

#[arc_callback]
fn callback_ganon(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_ganonboss")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 123;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 92;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() =
                to_hash40("vc_narration_characall_ganonboss");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_zelda");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_ganonboss", 0x172);
    apply_condensed_css_visibility(charroot, "ui_chara_ganonboss");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Dracula

#[arc_callback]
fn callback_dracula(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_dracula")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 124;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 93;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_dracula");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_castlevania");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_dracula", 0x175);
    apply_condensed_css_visibility(charroot, "ui_chara_dracula");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Galleom

#[arc_callback]
fn callback_galleom(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_galleom")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 125;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 94;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_galleom");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_galleom", 0x16F);
    apply_condensed_css_visibility(charroot, "ui_chara_galleom");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Rathalos

#[arc_callback]
fn callback_rathalos(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_lioleus")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 126;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 95;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_rathalos");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_lioleus", 0x188);
    apply_condensed_css_visibility(charroot, "ui_chara_lioleus");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// WOL Master Hand

#[arc_callback]
fn callback_wolmh(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_chara_mewtwo_masterhand")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 127;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 96;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() =
                to_hash40("vc_narration_characall_masterhandwol2");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    patch_css_selector_fields(charroot, "ui_chara_mewtwo_masterhand", 0x1A6);
    apply_condensed_css_visibility(charroot, "ui_chara_mewtwo_masterhand");
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// UNLOCKS HIDDEN MAPS

#[arc_callback]
fn callback_map_1(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_stage_boss_final2")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_2(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_stage_boss_final3")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_3(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_stage_boss_ganon")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_zelda");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_4(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_stage_boss_rathalos")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_5(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_stage_boss_marx")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_kirby");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_6(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_stage_boss_galleom")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_7(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list
        .0
        .iter_mut()
        .find(|param| {
            let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
            let (_, ui_chara_id) = &ui_chara_struct.0[0];
            let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
            *ui_chara_hash == to_hash40("ui_stage_boss_dracula")
        })
        .unwrap()
        .try_into_mut::<ParamStruct>()
        .unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_castlevania");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// ARCropolis callback buffer needs to be >= the largest patched PRC in load order.
// Logs showed ui_chara_db.prc around 0x9D3280, so keep comfortable headroom.
const MAX_FILE_SIZE: usize = 0x00C00000;

#[cfg(not(test))]
#[skyline::main(name = "comp_boss")]
pub fn main() {
    let cfg = &*CONFIG;
    let opts = &cfg.options;
    amiibo_preview::install_nro_trace();
    let giga_bowser_normal = !opts.giga_bowser_normal.unwrap_or(false);
    let condensed_css = opts.condense_bosses_into_single_slot();
    let custom_css = opts.custom_css.unwrap_or(false);
    let detect_character_name = opts.detect_character_name.unwrap_or(false);
    let master_hand_css = opts.master_hand_css.unwrap_or(true);
    let crazy_hand_css = opts.crazy_hand_css.unwrap_or(true);
    let dharkon_css = opts.dharkon_css.unwrap_or(true);
    let galeem_css = opts.galeem_css.unwrap_or(true);
    let marx_css = opts.marx_css.unwrap_or(true);
    let giga_bowser_css = opts.giga_bowser_css.unwrap_or(true);
    let ganon_css = opts.ganon_css.unwrap_or(true);
    let dracula_css = opts.dracula_css.unwrap_or(true);
    let rathalos_css = opts.rathalos_css.unwrap_or(true);
    let galleom_css = opts.galleom_css.unwrap_or(true);
    let wol_master_hand_css = opts.wol_master_hand_css.unwrap_or(true);
    let final2_stage = opts.final2_stage.unwrap_or(true);
    let final3_stage = opts.final3_stage.unwrap_or(true);
    let ganon_stage = opts.ganon_stage.unwrap_or(true);
    let rathalos_stage = opts.rathalos_stage.unwrap_or(true);
    let marx_stage = opts.marx_stage.unwrap_or(true);
    let galleom_stage = opts.galleom_stage.unwrap_or(true);
    let dracula_stage = opts.dracula_stage.unwrap_or(true);
    let amiibo_mappings = amiibo::configured_mappings();
    let amiibo_has = |key: &str| {
        amiibo_mappings
            .iter()
            .any(|mapping| mapping.identity.key == key)
    };
    // The viewer's identity handoff is runtime behavior, not debug-only
    // instrumentation. Populate its allowlist before any selection hook can
    // observe a Figure Player row.
    amiibo_preview::configure_mapping_profiles(&amiibo_mappings);

    if crate::debug::enabled() {
        if let Some(error) = amiibo::parse_error() {
            crate::boss_log!("[PB][Amiibo] mapping file parse error: {}", error);
        }
        for error in amiibo::validation_errors() {
            crate::boss_log!("[PB][Amiibo] mapping rejected: {}", error);
        }
        crate::boss_log!(
            "[PB][Amiibo] mapping source={} configured={}/11",
            amiibo::source_path().unwrap_or("<none>"),
            amiibo_mappings.len()
        );
        for mapping in &amiibo_mappings {
            crate::boss_log!(
                "[PB][Amiibo] configured boss={} tag=0x{:016x} ui_amiibo_id=0x{:010x} upper={} ui_chara_id={} selector=0x{:x} default_color={} remap_existing={} backing={}",
                mapping.identity.name,
                mapping.tag_id,
                mapping.ui_amiibo_id,
                mapping.nfp_character_id_upper,
                mapping.identity.ui_chara_id,
                mapping.identity.selector_id,
                mapping.default_color,
                mapping.remap_existing,
                mapping.identity.backing_fighter
            );
        }
        if !amiibo_mappings.is_empty() {
            amiibo_preview::log_mapping_profiles(&amiibo_mappings);
        }
        if !condensed_css && custom_css && !amiibo_mappings.is_empty() {
            crate::boss_log!(
                "[PB][Amiibo] CUSTOM_CSS=true: amiibo rows require the user's custom ui_chara_db entries"
            );
        }
        if condensed_css {
            crate::boss_log!(
                "[PB][CondensedBossCSS] enabled=true custom_css={} carrier=ui_chara_masterhand native_colors=8 galleom=ganon_color+shield wol_master_hand=master_hand_color+shield",
                custom_css
            );
        }
    }

    Agent::new("mario")
        .on_line(Main, mario_boss_dispatch_frame)
        .install();
    // Repopulate the per-entry selection cache from disk before anything can
    // query it. Without this a cold launch into Spirit Board or the World of
    // Light map resolves to the Mario host until the player visits the CSS.
    unsafe { selection::restore_persisted_selections() };
    selection::install();

    mastercrazy::install();
    if giga_bowser_normal || amiibo_has("giga_bowser") {
        gigabowser::install();
    }

    if !amiibo_mappings.is_empty() {
        callback_amiibo::install("ui/param/database/ui_amiibo_db.prc", MAX_FILE_SIZE);
    }

    if should_install_giga_bowser_css_callback(
        custom_css,
        giga_bowser_css,
        amiibo_has("giga_bowser"),
    ) {
        callback_koopag::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
        callback_koopag_layout::install("ui/param/database/ui_layout_db.prc", MAX_FILE_SIZE);
    }

    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        master_hand_css,
        amiibo_has("master_hand"),
    ) {
        callback_masterhand::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        crazy_hand_css,
        amiibo_has("crazy_hand"),
    ) {
        callback_crazyhand::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        dharkon_css,
        amiibo_has("dharkon"),
    ) {
        callback_dharkon::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        galeem_css,
        amiibo_has("galeem"),
    ) {
        callback_galeem::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        dracula_css,
        amiibo_has("dracula"),
    ) {
        callback_dracula::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        marx_css,
        amiibo_has("marx"),
    ) {
        callback_marx::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        ganon_css,
        amiibo_has("ganon_boss"),
    ) {
        callback_ganon::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        galleom_css,
        amiibo_has("galleom"),
    ) {
        callback_galleom::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        rathalos_css,
        amiibo_has("rathalos"),
    ) {
        callback_rathalos::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if should_install_item_boss_css_callback(
        condensed_css,
        custom_css,
        detect_character_name,
        wol_master_hand_css,
        amiibo_has("wol_master_hand"),
    ) {
        callback_wolmh::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE);
    }
    if condensed_css {
        callback_condensed_boss_layout::install(
            "ui/param/database/ui_layout_db.prc",
            MAX_FILE_SIZE,
        );
    }

    if final2_stage {
        callback_map_1::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE);
    }
    if final3_stage {
        callback_map_2::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE);
    }
    if ganon_stage {
        callback_map_3::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE);
    }
    if rathalos_stage {
        callback_map_4::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE);
    }
    if marx_stage {
        callback_map_5::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE);
    }
    if galleom_stage {
        callback_map_6::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE);
    }
    if dracula_stage {
        callback_map_7::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE);
    }
}

#[cfg(test)]
mod condensed_css_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn read_le_u16(data: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
    }

    fn read_le_u32(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
    }

    fn read_le_u64(data: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
    }

    #[derive(Debug, PartialEq, Eq)]
    struct BntxTextureInfo<'a> {
        name: &'a str,
        width: u32,
        height: u32,
    }

    fn parse_single_texture_bntx(data: &[u8]) -> BntxTextureInfo<'_> {
        assert!(data.len() >= 0x40, "truncated BNTX");
        assert_eq!(&data[..4], b"BNTX");
        assert_eq!(read_le_u32(data, 0x1c) as usize, data.len());
        assert_eq!(&data[0x20..0x24], b"NX  ");
        assert_eq!(read_le_u32(data, 0x24), 1, "expected one texture");

        let texture_table = read_le_u64(data, 0x28) as usize;
        assert!(texture_table + 8 <= data.len());
        let brti = read_le_u64(data, texture_table) as usize;
        assert!(brti + 0x68 <= data.len());
        assert_eq!(&data[brti..brti + 4], b"BRTI");

        let name_length_offset = read_le_u64(data, brti + 0x60) as usize;
        assert!(name_length_offset + 2 <= data.len());
        let name_length = usize::from(read_le_u16(data, name_length_offset));
        let name_start = name_length_offset + 2;
        let name_end = name_start + name_length;
        assert!(name_end < data.len());
        assert_eq!(data[name_end], 0, "BNTX texture name must be terminated");

        BntxTextureInfo {
            name: std::str::from_utf8(&data[name_start..name_end]).unwrap(),
            width: read_le_u32(data, brti + 0x24),
            height: read_le_u32(data, brti + 0x28),
        }
    }

    fn find_section(data: &[u8], magic: &[u8; 4]) -> usize {
        data.windows(magic.len())
            .position(|window| window == magic)
            .unwrap_or_else(|| panic!("missing {:?} section", std::str::from_utf8(magic)))
    }

    fn bntx_texture_payload(data: &[u8]) -> &[u8] {
        let start = find_section(data, b"BRTD") + 0x10;
        let end = data[start..]
            .windows(4)
            .position(|window| window == b"_RLT")
            .map(|offset| start + offset)
            .expect("missing BNTX relocation section");
        &data[start..end]
    }

    fn fnv1a64(data: &[u8]) -> u64 {
        data.iter().fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
    }

    fn assert_manifest_path_once(manifest: &str, path: &str) {
        assert_eq!(
            manifest.matches(path).count(),
            1,
            "{path} must appear exactly once in the install manifest"
        );
    }

    fn msbt_label_group(label: &str, group_count: u32) -> u32 {
        label.bytes().fold(0u32, |hash, byte| {
            hash.wrapping_mul(0x492).wrapping_add(u32::from(byte))
        }) % group_count
    }

    fn parse_msbt_label_indices(data: &[u8]) -> BTreeMap<String, u32> {
        assert_eq!(&data[..8], b"MsgStdBn");
        assert_eq!(&data[8..10], b"\xff\xfe", "test archive must be LE");
        assert_eq!(read_le_u32(data, 0x12) as usize, data.len());

        let section_count = usize::from(read_le_u16(data, 0x0e));
        let mut section_offset = 0x20usize;
        for _ in 0..section_count {
            let section_size = read_le_u32(data, section_offset + 4) as usize;
            if &data[section_offset..section_offset + 4] == b"LBL1" {
                let payload_offset = section_offset + 0x10;
                let group_count = read_le_u32(data, payload_offset);
                let mut labels = BTreeMap::new();
                for group in 0..group_count {
                    let group_entry = payload_offset + 4 + group as usize * 8;
                    let label_count = read_le_u32(data, group_entry);
                    let mut cursor = payload_offset + read_le_u32(data, group_entry + 4) as usize;
                    for _ in 0..label_count {
                        let label_length = usize::from(data[cursor]);
                        cursor += 1;
                        let label = std::str::from_utf8(&data[cursor..cursor + label_length])
                            .unwrap()
                            .to_owned();
                        cursor += label_length;
                        let text_index = read_le_u32(data, cursor);
                        cursor += 4;
                        assert_eq!(
                            msbt_label_group(&label, group_count),
                            group,
                            "MSBT label is stored in the wrong hash group: {label}"
                        );
                        assert!(labels.insert(label, text_index).is_none());
                    }
                }
                return labels;
            }
            section_offset = (section_offset + 0x10 + section_size + 0x0f) & !0x0f;
        }

        panic!("msg_name.msbt has no LBL1 section");
    }

    fn parse_msbt_texts(data: &[u8]) -> Vec<String> {
        let section_count = usize::from(read_le_u16(data, 0x0e));
        let mut section_offset = 0x20usize;
        for _ in 0..section_count {
            let section_size = read_le_u32(data, section_offset + 4) as usize;
            if &data[section_offset..section_offset + 4] == b"TXT2" {
                let payload_offset = section_offset + 0x10;
                let text_count = read_le_u32(data, payload_offset) as usize;
                return (0..text_count)
                    .map(|index| {
                        let relative_offset =
                            read_le_u32(data, payload_offset + 4 + index * 4) as usize;
                        let mut cursor = payload_offset + relative_offset;
                        let mut utf16 = Vec::new();
                        loop {
                            let code_unit = read_le_u16(data, cursor);
                            cursor += 2;
                            if code_unit == 0 {
                                break;
                            }
                            utf16.push(code_unit);
                        }
                        String::from_utf16(&utf16).expect("invalid UTF-16 in msg_name.msbt")
                    })
                    .collect();
            }
            section_offset = (section_offset + 0x10 + section_size + 0x0f) & !0x0f;
        }

        panic!("msg_name.msbt has no TXT2 section");
    }

    fn css_visibility_row(
        can_select: bool,
        is_hidden_boss: bool,
        disp_order: i8,
        skill_list_order: i8,
    ) -> ParamStruct {
        ParamStruct(vec![
            (to_hash40("can_select"), ParamKind::Bool(can_select)),
            (to_hash40("is_hidden_boss"), ParamKind::Bool(is_hidden_boss)),
            (to_hash40("disp_order"), ParamKind::I8(disp_order)),
            (
                to_hash40("skill_list_order"),
                ParamKind::I8(skill_list_order),
            ),
        ])
    }

    fn mario_color_map_row() -> ParamStruct {
        let mut fields = Vec::new();
        for (color, field_names) in CONDENSED_COLOR_MAP_FIELDS.iter().enumerate() {
            fields.push((to_hash40(field_names[0]), ParamKind::U8(color as u8)));
            fields.push((to_hash40(field_names[1]), ParamKind::U8(16 + color as u8)));
            fields.push((to_hash40(field_names[2]), ParamKind::U8(32 + color as u8)));
        }
        ParamStruct(fields)
    }

    fn masterhand_carrier_row() -> ParamStruct {
        let mut row = css_visibility_row(false, true, 118, 87);
        row.0.push((
            to_hash40("name_id"),
            ParamKind::Str("masterhand".to_owned()),
        ));
        row
    }

    #[test]
    fn condensed_css_roles_are_explicit_for_every_required_entry() {
        assert_eq!(
            CONDENSED_SUPPRESSED_UI_CHARA,
            [
                "ui_chara_crazyhand",
                "ui_chara_darz",
                "ui_chara_kiila",
                "ui_chara_ganonboss",
                "ui_chara_lioleus",
                "ui_chara_dracula",
                "ui_chara_marx",
                "ui_chara_mewtwo_masterhand",
                "ui_chara_galleom",
            ]
        );
        assert_eq!(
            condensed_css_role(CONDENSED_CARRIER_UI_CHARA),
            CondensedCssRole::Carrier
        );
        for ui_chara_id in CONDENSED_SUPPRESSED_UI_CHARA {
            assert_eq!(
                condensed_css_role(ui_chara_id),
                CondensedCssRole::Suppressed,
                "{ui_chara_id} must not remain a standalone condensed CSS entry"
            );
        }
        assert_eq!(
            condensed_css_role("ui_chara_koopag"),
            CondensedCssRole::Untouched
        );
    }

    #[test]
    fn condensed_visibility_mutations_preserve_rows_and_leave_giga_untouched() {
        let mut carrier = css_visibility_row(false, true, 118, 87);
        apply_condensed_css_visibility_for_mode(&mut carrier, CONDENSED_CARRIER_UI_CHARA, true);
        assert_eq!(
            read_bool_field(&carrier, to_hash40("can_select")),
            Some(true)
        );
        assert_eq!(
            read_bool_field(&carrier, to_hash40("is_hidden_boss")),
            Some(false)
        );

        for ui_chara_id in CONDENSED_SUPPRESSED_UI_CHARA {
            let mut row = css_visibility_row(true, false, 120, 90);
            apply_condensed_css_visibility_for_mode(&mut row, ui_chara_id, true);
            assert_eq!(read_bool_field(&row, to_hash40("can_select")), Some(false));
            assert_eq!(
                read_bool_field(&row, to_hash40("is_hidden_boss")),
                Some(true)
            );
            assert_eq!(read_i8_field(&row, to_hash40("disp_order")), Some(-1));
            assert_eq!(read_i8_field(&row, to_hash40("skill_list_order")), Some(-1));
        }

        let mut giga = css_visibility_row(true, false, 117, 86);
        let original_giga = giga.clone();
        apply_condensed_css_visibility_for_mode(&mut giga, "ui_chara_koopag", true);
        assert_eq!(giga, original_giga);

        let mut disabled = css_visibility_row(true, false, 120, 90);
        let original_disabled = disabled.clone();
        apply_condensed_css_visibility_for_mode(&mut disabled, "ui_chara_crazyhand", false);
        assert_eq!(disabled, original_disabled);
    }

    #[test]
    fn carrier_row_exposes_exactly_native_colors_zero_through_seven() {
        let mut carrier = masterhand_carrier_row();
        let mario = mario_color_map_row();
        assert_eq!(
            configure_condensed_masterhand_carrier_fields(&mut carrier, &mario),
            Ok(24)
        );

        assert_eq!(CONDENSED_NATIVE_COLOR_COUNT, 8);
        assert_eq!(
            read_u8_field(&carrier, to_hash40("color_num")),
            Some(CONDENSED_NATIVE_COLOR_COUNT)
        );
        assert_eq!(
            read_string_field(&carrier, to_hash40("name_id")),
            Some("masterhand")
        );
        assert_eq!(read_hash40_field(&carrier, to_hash40("name_id")), None);
        assert_eq!(
            carrier
                .0
                .iter()
                .filter(|(hash, _)| *hash == to_hash40("name_id"))
                .count(),
            1,
            "the carrier must retain exactly one string-valued name_id"
        );
        for (color, field_names) in CONDENSED_COLOR_MAP_FIELDS.iter().enumerate() {
            assert_eq!(
                read_u8_field(&carrier, to_hash40(field_names[0])),
                Some(color as u8)
            );
            // Name index is deliberately NOT Mario's. Mario names all eight
            // costumes the same, so copying his map made every carrier skin
            // display "MASTER HAND". Identity mapping gives slot N the
            // `nam_chr*_NN_masterhand` label. Costume index and group above/
            // below still come from Mario so only real costumes are requested.
            assert_eq!(
                read_u8_field(&carrier, to_hash40(field_names[1])),
                Some(color as u8)
            );
            assert_eq!(
                read_u8_field(&carrier, to_hash40(field_names[2])),
                Some(32 + color as u8)
            );
        }
        for presentation in CONDENSED_NATIVE_PRESENTATIONS {
            assert_eq!(
                read_hash40_field(&carrier, to_hash40(presentation.characall_field)),
                Some(to_hash40(presentation.narration_label))
            );
        }
    }

    /// Each carrier color must resolve its own MSBT name label. Mario names all
    /// eight costumes the same, so copying his `nXX_index` map made every skin
    /// display "MASTER HAND". The name map is identity; the costume map still
    /// follows Mario so only real Mario costume resources are requested.
    #[test]
    fn carrier_name_index_map_is_identity_not_marios() {
        let mut carrier = masterhand_carrier_row();
        let mario = mario_color_map_row();
        configure_condensed_masterhand_carrier_fields(&mut carrier, &mario)
            .expect("carrier setup should succeed");

        for (slot, field_names) in CONDENSED_COLOR_MAP_FIELDS.iter().enumerate() {
            // Name index: identity, so slot N asks for nam_chr*_NN_masterhand.
            assert_eq!(
                read_u8_field(&carrier, to_hash40(field_names[1])),
                Some(slot as u8),
                "{} must map to name index {slot}",
                field_names[1]
            );
            // Costume index and group still come from Mario.
            assert_eq!(
                read_u8_field(&carrier, to_hash40(field_names[0])),
                read_u8_field(&mario, to_hash40(field_names[0])),
                "{} must keep Mario's costume index",
                field_names[0]
            );
            assert_eq!(
                read_u8_field(&carrier, to_hash40(field_names[2])),
                read_u8_field(&mario, to_hash40(field_names[2])),
                "{} must keep Mario's color group",
                field_names[2]
            );
        }

        // Every name index must be distinct, or two skins would share a name.
        let names: Vec<_> = CONDENSED_COLOR_MAP_FIELDS
            .iter()
            .map(|f| read_u8_field(&carrier, to_hash40(f[1])))
            .collect();
        for (i, a) in names.iter().enumerate() {
            for b in &names[i + 1..] {
                assert_ne!(a, b, "duplicate name index would repeat a boss name");
            }
        }
    }

    #[test]
    fn carrier_color_map_setup_is_atomic_when_mario_schema_is_incomplete() {
        let carrier = masterhand_carrier_row();
        let mut incomplete_mario = mario_color_map_row();
        incomplete_mario
            .0
            .retain(|(hash, _)| *hash != to_hash40("n07_index"));
        let mut candidate = carrier.clone();

        assert_eq!(
            configure_condensed_masterhand_carrier_fields(&mut candidate, &incomplete_mario),
            Err(CondensedCarrierError::MissingMarioColorMapField(
                "n07_index"
            ))
        );
        assert_eq!(candidate, carrier);
    }

    #[test]
    fn carrier_rejects_a_non_string_name_id_without_mutation() {
        let mut carrier = css_visibility_row(false, true, 118, 87);
        carrier.0.push((
            to_hash40("name_id"),
            ParamKind::Hash(to_hash40("masterhand")),
        ));
        let original = carrier.clone();

        assert_eq!(
            configure_condensed_masterhand_carrier_fields(&mut carrier, &mario_color_map_row()),
            Err(CondensedCarrierError::InvalidCarrierNameId)
        );
        assert_eq!(carrier, original);
    }

    #[test]
    fn native_presentation_table_is_complete_ordered_and_unique() {
        let expected = [
            (
                0,
                "master_hand",
                "ui_chara_masterhand",
                "ui_chara_masterhand",
                "ui_chara_master_hand_00",
                "ui_chara_master_hand_00",
            ),
            (
                1,
                "crazy_hand",
                "ui_chara_crazyhand",
                "ui_chara_crazyhand",
                "ui_chara_crazy_hand_00",
                "ui_chara_master_hand_01",
            ),
            (
                2,
                "dharkon",
                "ui_chara_darz",
                "ui_chara_masterhand",
                "ui_chara_master_hand_00",
                "ui_chara_master_hand_02",
            ),
            (
                3,
                "galeem",
                "ui_chara_kiila",
                "ui_chara_masterhand",
                "ui_chara_master_hand_00",
                "ui_chara_master_hand_03",
            ),
            (
                4,
                "ganon_boss",
                "ui_chara_ganonboss",
                "ui_chara_ganonboss",
                "ui_chara_ganonboss_00",
                "ui_chara_master_hand_04",
            ),
            (
                5,
                "rathalos",
                "ui_chara_lioleus",
                "ui_chara_lioleus",
                "ui_chara_lioleus_00",
                "ui_chara_master_hand_05",
            ),
            (
                6,
                "dracula",
                "ui_chara_dracula",
                "ui_chara_dracula",
                "ui_chara_dracula_00",
                "ui_chara_master_hand_06",
            ),
            (
                7,
                "marx",
                "ui_chara_marx",
                "ui_chara_marx",
                "ui_chara_marx_00",
                "ui_chara_master_hand_07",
            ),
        ];

        assert_eq!(CONDENSED_NATIVE_PRESENTATIONS.len(), expected.len());
        for (presentation, expected) in CONDENSED_NATIVE_PRESENTATIONS.iter().zip(expected) {
            assert_eq!(
                (
                    presentation.logical_index,
                    presentation.logical_boss,
                    presentation.source_ui_chara_id,
                    presentation.layout_template_ui_chara_id,
                    presentation.layout_template_id,
                    presentation.carrier_layout_id,
                ),
                expected
            );
        }

        for (index, presentation) in CONDENSED_NATIVE_PRESENTATIONS.iter().enumerate() {
            for other in CONDENSED_NATIVE_PRESENTATIONS.iter().skip(index + 1) {
                assert_ne!(presentation.source_ui_chara_id, other.source_ui_chara_id);
                assert_ne!(presentation.carrier_layout_id, other.carrier_layout_id);
                assert_ne!(presentation.characall_field, other.characall_field);
            }
        }
    }

    #[test]
    fn carrier_layout_callback_uses_carrier_identity_and_local_color() {
        let layout_templates = [
            ("ui_chara_master_hand_00", "ui_chara_masterhand", 0),
            ("ui_chara_crazy_hand_00", "ui_chara_crazyhand", 1),
            ("ui_chara_ganonboss_00", "ui_chara_ganonboss", 4),
            ("ui_chara_lioleus_00", "ui_chara_lioleus", 5),
            ("ui_chara_dracula_00", "ui_chara_dracula", 6),
            ("ui_chara_marx_00", "ui_chara_marx", 7),
        ];
        let mut layouts = ParamList(
            layout_templates
                .iter()
                .map(|(layout_id, ui_chara_id, marker)| {
                    ParamKind::Struct(ParamStruct(vec![
                        (
                            to_hash40("ui_layout_id"),
                            ParamKind::Hash(to_hash40(layout_id)),
                        ),
                        (
                            to_hash40("ui_chara_id"),
                            ParamKind::Hash(to_hash40(ui_chara_id)),
                        ),
                        (to_hash40("chara_color"), ParamKind::U8(7)),
                        (to_hash40("presentation_marker"), ParamKind::U8(*marker)),
                    ]))
                })
                .collect(),
        );

        assert_eq!(ensure_condensed_carrier_layouts(&mut layouts), Ok(8));
        assert_eq!(layouts.0.len(), 13);
        for presentation in CONDENSED_NATIVE_PRESENTATIONS {
            let row = layouts
                .0
                .iter()
                .find(|param| {
                    struct_hash40_field_matches(
                        param,
                        to_hash40("ui_layout_id"),
                        to_hash40(presentation.carrier_layout_id),
                    )
                })
                .and_then(|param| param.try_into_ref::<ParamStruct>().ok())
                .expect("every native carrier variation needs a layout row");
            assert_eq!(
                read_hash40_field(row, to_hash40("ui_chara_id")),
                Some(to_hash40(CONDENSED_CARRIER_UI_CHARA))
            );
            assert_eq!(
                read_u8_field(row, to_hash40("chara_color")),
                Some(presentation.logical_index)
            );
            assert_eq!(
                read_u8_field(row, to_hash40("presentation_marker")),
                layout_templates
                    .iter()
                    .find(|(layout_id, _, _)| *layout_id == presentation.layout_template_id)
                    .map(|(_, _, marker)| *marker),
                "the full native layout template must survive aliasing"
            );
        }

        for (layout_id, ui_chara_id, _) in layout_templates {
            let source_row = layouts
                .0
                .iter()
                .find(|param| {
                    struct_hash40_field_matches(
                        param,
                        to_hash40("ui_layout_id"),
                        to_hash40(layout_id),
                    )
                })
                .and_then(|param| param.try_into_ref::<ParamStruct>().ok())
                .expect("condensed aliases must preserve native layout templates");
            assert_eq!(
                read_hash40_field(source_row, to_hash40("ui_chara_id")),
                Some(to_hash40(ui_chara_id))
            );
        }
        assert!(!CONDENSED_NATIVE_PRESENTATIONS
            .iter()
            .any(|presentation| presentation.carrier_layout_id == "ui_chara_master_hand_08"));
    }

    #[test]
    fn condensed_native_portraits_are_installable_and_self_named() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = std::fs::read_to_string(root.join("ultimate/mods/Bosses/config.json"))
            .expect("missing ARCropolis mod manifest");
        // Colors 1-7 are byte-for-byte Ultimate's native boss texture payloads
        // from the matching chara_1 family. Only their single BNTX texture key
        // is changed to the Master Hand carrier's color-specific lookup.
        let native_payload_fingerprints = [
            0xb451a665b2de2d9f,
            0x3c93b7ca716a256e,
            0x0c51f6e5b257133a,
            0x6d4d97d9c81a8590,
            0xf76775032f8ae450,
            0xb3bedd15e7b87e4e,
            0xce44555f1fc66007,
        ];
        for color in 0..CONDENSED_NATIVE_COLOR_COUNT {
            let internal_name = format!("chara_1_masterhand_{color:02}");
            let relative_path =
                format!("ultimate/mods/Bosses/ui/replace/chara/chara_1/{internal_name}.bntx");
            let data = std::fs::read(root.join(&relative_path))
                .unwrap_or_else(|_| panic!("missing condensed portrait asset {relative_path}"));
            let info = parse_single_texture_bntx(&data);
            assert_eq!(info.name, internal_name);
            assert_manifest_path_once(
                &manifest,
                relative_path.trim_start_matches("ultimate/mods/Bosses/"),
            );

            if color > 0 {
                assert_eq!(data.len(), 266_344);
                assert_eq!((info.width, info.height), (512, 512));
                assert_eq!(
                    fnv1a64(bntx_texture_payload(&data)),
                    native_payload_fingerprints[usize::from(color - 1)],
                    "color {color} must retain its source-proven native boss portrait payload"
                );
            }
        }
    }

    #[test]
    fn condensed_native_css_icons_are_installable_and_self_named() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = std::fs::read_to_string(root.join("ultimate/mods/Bosses/config.json"))
            .expect("missing ARCropolis mod manifest");

        // Color zero uses Ultimate's existing Master Hand icon. The remaining
        // carrier colors are new resource paths and must ship explicitly.
        let variant_names = [
            "masterhand",
            "crazyhand",
            "darz",
            "kiila",
            "ganonboss",
            "lioleus",
            "dracula",
            "marx",
        ];
        for color in 1..CONDENSED_NATIVE_COLOR_COUNT {
            let internal_name = format!("chara_4_masterhand_{color:02}");
            let relative_path =
                format!("ultimate/mods/Bosses/ui/replace/chara/chara_4/{internal_name}.bntx");
            let data = std::fs::read(root.join(&relative_path))
                .unwrap_or_else(|_| panic!("missing condensed CSS icon {relative_path}"));
            assert_eq!(parse_single_texture_bntx(&data).name, internal_name);

            let source_name = variant_names[usize::from(color)];
            let source_path = root.join(format!(
                "ultimate/mods/Bosses/ui/replace/chara/chara_4/chara_4_{source_name}_00.bntx"
            ));
            let source_data = std::fs::read(&source_path)
                .unwrap_or_else(|_| panic!("missing source CSS icon {}", source_path.display()));
            assert_eq!(
                bntx_texture_payload(&data),
                bntx_texture_payload(&source_data),
                "color {color} must retain the {source_name} CSS icon payload"
            );
            assert_manifest_path_once(
                &manifest,
                relative_path.trim_start_matches("ultimate/mods/Bosses/"),
            );
        }

        assert!(
            !manifest.contains("chara_4_masterhand_00.bntx"),
            "the native color-zero icon must not be declared as a missing new file"
        );
    }

    #[test]
    fn condensed_native_stock_and_layout_portraits_are_installable_and_self_named() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = std::fs::read_to_string(root.join("ultimate/mods/Bosses/config.json"))
            .expect("missing ARCropolis mod manifest");
        let variant_names = [
            "masterhand",
            "crazyhand",
            "darz",
            "kiila",
            "ganonboss",
            "lioleus",
            "dracula",
            "marx",
        ];

        for family in [2, 7] {
            for color in 0..CONDENSED_NATIVE_COLOR_COUNT {
                let internal_name = format!("chara_{family}_masterhand_{color:02}");
                let relative_path = format!(
                    "ultimate/mods/Bosses/ui/replace/chara/chara_{family}/{internal_name}.bntx"
                );
                let data = std::fs::read(root.join(&relative_path))
                    .unwrap_or_else(|_| panic!("missing condensed carrier asset {relative_path}"));
                assert_eq!(parse_single_texture_bntx(&data).name, internal_name);
                assert_manifest_path_once(
                    &manifest,
                    relative_path.trim_start_matches("ultimate/mods/Bosses/"),
                );

                if color > 0 {
                    let source_name = variant_names[usize::from(color)];
                    let source_path = root.join(format!(
                        "ultimate/mods/Bosses/ui/replace/chara/chara_{family}/chara_{family}_{source_name}_00.bntx"
                    ));
                    let source_data = std::fs::read(&source_path).unwrap_or_else(|_| {
                        panic!("missing carrier source asset {}", source_path.display())
                    });
                    assert_eq!(
                        bntx_texture_payload(&data),
                        bntx_texture_payload(&source_data),
                        "family {family} color {color} must retain the {source_name} texture payload"
                    );
                }
            }
        }
    }

    #[test]
    fn condensed_name_archive_contains_carrier_and_variant_labels() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let data = std::fs::read(root.join("ultimate/mods/Bosses/ui/message/msg_name.msbt"))
            .expect("missing supplied msg_name.msbt");
        let labels = parse_msbt_label_indices(&data);
        let texts = parse_msbt_texts(&data);

        for name_id in [
            "playable_bosses",
            "masterhand",
            "crazyhand",
            "darz",
            "kiila",
            "ganonboss",
            "lioleus",
            "dracula",
            "marx",
            "galleom",
            "mewtwo_masterhand",
        ] {
            for prefix in [
                "nam_chr0_00_",
                "nam_chr1_00_",
                "nam_chr2_00_",
                "nam_chr3_00_",
            ] {
                let label = format!("{prefix}{name_id}");
                assert!(labels.contains_key(&label), "missing message label {label}");
            }
        }

        let variant_names = [
            "masterhand",
            "crazyhand",
            "darz",
            "kiila",
            "ganonboss",
            "lioleus",
            "dracula",
            "marx",
        ];
        for prefix in ["nam_chr0", "nam_chr1", "nam_chr2", "nam_chr3"] {
            for (color, source_name) in variant_names.iter().enumerate() {
                let source_label = format!("{prefix}_00_{source_name}");
                let carrier_label = format!("{prefix}_{color:02}_masterhand");
                assert_eq!(
                    labels.get(&carrier_label),
                    labels.get(&source_label),
                    "carrier color {color} must reuse the existing {source_name} text through the active masterhand lookup"
                );
            }
        }

        let expected_text = [
            [
                "Master Hand",
                "Crazy Hand",
                "Dharkon",
                "Galeem",
                "Ganon",
                "Rathalos",
                "Dracula",
                "Marx",
            ],
            [
                "Master Hand",
                "Crazy Hand",
                "Dharkon",
                "Galeem",
                "Ganon, The Demon King",
                "Rathalos",
                "Dracula",
                "Marx",
            ],
            [
                "MASTER HAND",
                "CRAZY HAND",
                "DHARKON",
                "GALEEM",
                "GANON",
                "RATHALOS",
                "DRACULA",
                "MARX",
            ],
            [
                "MASTER HAND",
                "CRAZY HAND",
                "DHARKON",
                "GALEEM",
                "GANON",
                "RATHALOS",
                "DRACULA",
                "MARX",
            ],
        ];
        for (prefix_index, prefix) in ["nam_chr0", "nam_chr1", "nam_chr2", "nam_chr3"]
            .iter()
            .enumerate()
        {
            for color in 0..CONDENSED_NATIVE_COLOR_COUNT {
                let label = format!("{prefix}_{color:02}_masterhand");
                let text_index = usize::try_from(labels[&label]).unwrap();
                assert_eq!(
                    texts[text_index],
                    expected_text[prefix_index][usize::from(color)],
                    "incorrect visible name for carrier color {color} in {prefix}"
                );
            }
        }
    }

    #[test]
    fn carrier_layout_aliasing_is_atomic_when_a_source_layout_is_missing() {
        let original = ParamList(vec![ParamKind::Struct(ParamStruct(vec![
            (
                to_hash40("ui_layout_id"),
                ParamKind::Hash(to_hash40("ui_chara_master_hand_00")),
            ),
            (
                to_hash40("ui_chara_id"),
                ParamKind::Hash(to_hash40("ui_chara_masterhand")),
            ),
            (to_hash40("chara_color"), ParamKind::U8(0)),
        ]))]);
        let mut layouts = original.clone();

        assert_eq!(
            ensure_condensed_carrier_layouts(&mut layouts),
            Err(CondensedLayoutError::MissingLayoutTemplate(
                "ui_chara_crazy_hand_00"
            ))
        );
        assert_eq!(layouts, original);
    }

    #[test]
    fn condensed_config_takes_precedence_over_legacy_css_options() {
        // Required matrix: condensed mode owns the item-boss rows regardless
        // of CUSTOM_CSS, DETECT_CHARACTER_NAME, or individual row flags.
        assert!(should_install_item_boss_css_callback(
            true, false, true, false, false
        ));
        assert!(should_install_item_boss_css_callback(
            true, true, false, false, false
        ));
        assert!(should_install_item_boss_css_callback(
            true, false, true, true, false
        ));

        // With condensed mode off, preserve the prior CUSTOM_CSS and
        // per-boss/Amiibo behavior exactly.
        assert!(should_install_item_boss_css_callback(
            false, false, true, true, false
        ));
        assert!(should_install_item_boss_css_callback(
            false, false, false, false, true
        ));
        assert!(!should_install_item_boss_css_callback(
            false, false, true, false, false
        ));
        assert!(!should_install_item_boss_css_callback(
            false, true, false, false, false
        ));
        assert!(!should_install_item_boss_css_callback(
            false, true, false, true, true
        ));
    }

    #[test]
    fn condensed_mode_does_not_change_giga_bowser_callback_policy() {
        assert!(should_install_giga_bowser_css_callback(false, true, false));
        assert!(should_install_giga_bowser_css_callback(false, false, true));
        assert!(!should_install_giga_bowser_css_callback(true, true, true));
    }
}

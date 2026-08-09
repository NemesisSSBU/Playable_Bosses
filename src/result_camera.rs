//! Native result-camera handoff for item-backed playable bosses.
//!
//! Result items already own the boss presentation model and their native item
//! camera data. This module only makes the winning result item the native
//! camera subject; it does not create objects, alter camera ranges, or change
//! the result-item lifecycle.

use smash::app::lua_bind::{
    CameraModule, FighterManager, ItemCameraModuleImpl, ItemModule, MotionModule, PostureModule,
    StatusModule,
};
use smash::app::{sv_battle_object, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;

use crate::{boss_helpers, selection};

const MAX_FIGHTERS: usize = 8;

#[derive(Copy, Clone)]
struct ResultCameraProfile {
    name: &'static str,
    item_kind: i32,
}

static mut ACTIVE_SUBJECT_ID: u32 = 0;
static mut ACTIVE_SUBJECT_ENTRY: usize = usize::MAX;
static mut ACTIVE_SUBJECT_KIND: i32 = -1;
static mut LAST_RESULT_MODE: bool = false;
static mut LAST_WINNER_RAW: u64 = u64::MAX;
static mut LAST_MISSING_SIGNATURE: u64 = u64::MAX;

#[inline(always)]
unsafe fn profile_for_item_kind(item_kind: i32) -> Option<ResultCameraProfile> {
    let profile = if item_kind == *ITEM_KIND_MASTERHAND {
        ResultCameraProfile {
            name: "master_hand",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_CRAZYHAND {
        ResultCameraProfile {
            name: "crazy_hand",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_PLAYABLE_MASTERHAND {
        ResultCameraProfile {
            name: "wol_master_hand",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_KIILA {
        ResultCameraProfile {
            name: "galeem",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_DARZ {
        ResultCameraProfile {
            name: "dharkon",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_DRACULA || item_kind == *ITEM_KIND_DRACULA2 {
        ResultCameraProfile {
            name: "dracula",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_GANONBOSS {
        ResultCameraProfile {
            name: "ganon",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_GALLEOM {
        ResultCameraProfile {
            name: "galleom",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_LIOLEUSBOSS {
        ResultCameraProfile {
            name: "rathalos",
            item_kind,
        }
    } else if item_kind == *ITEM_KIND_MARX {
        ResultCameraProfile {
            name: "marx",
            item_kind,
        }
    } else {
        return None;
    };

    Some(profile)
}

#[inline(always)]
unsafe fn result_item_for_host(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<(u32, *mut BattleObjectModuleAccessor, ResultCameraProfile)> {
    if module_accessor.is_null() {
        return None;
    }

    for slot in 0..4 {
        if !ItemModule::is_have_item(module_accessor, slot) {
            continue;
        }

        let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
        if item_id == 0 || !sv_battle_object::is_active(item_id) {
            continue;
        }

        let item_boma = sv_battle_object::module_accessor(item_id);
        if item_boma.is_null() {
            continue;
        }

        let item_kind = smash::app::utility::get_kind(&mut *item_boma);
        let Some(profile) = profile_for_item_kind(item_kind) else {
            continue;
        };
        return Some((item_id, item_boma, profile));
    }

    None
}

#[inline(always)]
unsafe fn release_active_subject(reason: &str) {
    let subject_id = ACTIVE_SUBJECT_ID;
    if subject_id == 0 {
        return;
    }

    let mut restored = false;
    let mut end_result = 0u64;
    if sv_battle_object::is_active(subject_id) {
        let subject_boma = sv_battle_object::module_accessor(subject_id);
        if !subject_boma.is_null() {
            end_result = ItemCameraModuleImpl::end_camera_subject(subject_boma);
            restored = true;
        }
    }

    let active_entry = ACTIVE_SUBJECT_ENTRY;
    let active_kind = ACTIVE_SUBJECT_KIND;
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][ResultCamera] subject_end reason={} entry={} object_id=0x{:x} item_kind={} end_result=0x{:x} restored={}",
            reason,
            active_entry,
            subject_id,
            active_kind,
            end_result,
            restored
        );
    }

    ACTIVE_SUBJECT_ID = 0;
    ACTIVE_SUBJECT_ENTRY = usize::MAX;
    ACTIVE_SUBJECT_KIND = -1;
}

#[inline(always)]
unsafe fn start_subject(
    host: *mut BattleObjectModuleAccessor,
    entry: usize,
    item_id: u32,
    item_boma: *mut BattleObjectModuleAccessor,
    profile: ResultCameraProfile,
) {
    if ACTIVE_SUBJECT_ID == item_id {
        return;
    }

    if ACTIVE_SUBJECT_ID != 0 {
        release_active_subject("subject_replaced");
    }

    // This is the game's native item-camera handoff. Do not set camera type,
    // range, FOV, or stage bounds here; the result item owns those values.
    let start_result = ItemCameraModuleImpl::start_camera_subject(item_boma);
    ACTIVE_SUBJECT_ID = item_id;
    ACTIVE_SUBJECT_ENTRY = entry;
    ACTIVE_SUBJECT_KIND = profile.item_kind;

    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][ResultCamera] subject_start entry={} object_id=0x{:x} profile={} item_kind={} start_result=0x{:x} host_camera_type=0x{:x} item_status={} item_motion=0x{:x} item_pos=({:.2},{:.2},{:.2}) selected_ui_hash=0x{:010x}",
            entry,
            item_id,
            profile.name,
            profile.item_kind,
            start_result,
            CameraModule::get_camera_type(host),
            StatusModule::status_kind(item_boma),
            MotionModule::motion_kind(item_boma),
            PostureModule::pos_x(item_boma),
            PostureModule::pos_y(item_boma),
            PostureModule::pos_z(item_boma),
            selection::selected_css_boss_selector_id(host).unwrap_or(0)
        );
    }
}

/// Runs from the existing Mario host callback. It is intentionally result
/// only and only acts for the actual final actor entry, never for P1 by rule.
pub unsafe fn frame(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() {
        return;
    }

    let fighter_manager = boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    if !result_mode {
        if LAST_RESULT_MODE {
            release_active_subject("result_exit");
            if crate::debug::enabled() {
                crate::boss_log!("[PB][ResultCamera] scene_exit reason=result_exit restored=true");
            }
        } else if ACTIVE_SUBJECT_ID != 0 {
            release_active_subject("result_mode_lost");
        }
        LAST_RESULT_MODE = false;
        LAST_WINNER_RAW = u64::MAX;
        LAST_MISSING_SIGNATURE = u64::MAX;
        return;
    }

    if !LAST_RESULT_MODE {
        LAST_RESULT_MODE = true;
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][ResultCamera] scene_enter stage=0x{:x}",
                smash::app::stage::get_stage_id()
            );
        }
    }

    let winner_raw = FighterManager::get_final_actor_entry_id(fighter_manager);
    if winner_raw != LAST_WINNER_RAW {
        LAST_WINNER_RAW = winner_raw;
        LAST_MISSING_SIGNATURE = u64::MAX;
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][ResultCamera] winner_raw=0x{:x} winner_entry={}",
                winner_raw,
                if winner_raw < MAX_FIGHTERS as u64 {
                    winner_raw as i32
                } else {
                    -1
                }
            );
        }
    }

    let Some(winner_entry) = (winner_raw < MAX_FIGHTERS as u64).then_some(winner_raw as usize)
    else {
        return;
    };

    let entry = boss_helpers::entry_id(module_accessor);
    if entry != winner_entry || entry >= MAX_FIGHTERS {
        return;
    }

    // Giga Bowser is the one dedicated fighter-backed boss. Leave its native
    // fighter result camera untouched rather than treating it as a missing
    // item result, but record the profile boundary for hardware comparison.
    if smash::app::utility::get_kind(&mut *module_accessor) == *FIGHTER_KIND_KOOPAG {
        if ACTIVE_SUBJECT_ID != 0 {
            release_active_subject("winner_changed_to_giga_bowser");
        }
        let signature = winner_raw ^ ((entry as u64) << 32) ^ 0x4749_4741;
        if crate::debug::enabled() && LAST_MISSING_SIGNATURE != signature {
            LAST_MISSING_SIGNATURE = signature;
            crate::boss_log!(
                "[PB][ResultCamera] native_fighter_profile entry={} profile=giga_bowser fighter_kind={} native_subject=unchanged camera_type=0x{:x}",
                entry,
                *FIGHTER_KIND_KOOPAG,
                CameraModule::get_camera_type(module_accessor)
            );
        }
        return;
    }

    if let Some((item_id, item_boma, profile)) = result_item_for_host(module_accessor) {
        LAST_MISSING_SIGNATURE = u64::MAX;
        start_subject(module_accessor, entry, item_id, item_boma, profile);
    } else {
        let signature = winner_raw ^ ((entry as u64) << 32);
        if crate::debug::enabled() && ACTIVE_SUBJECT_ID == 0 && LAST_MISSING_SIGNATURE != signature
        {
            LAST_MISSING_SIGNATURE = signature;
            crate::boss_log!(
                "[PB][ResultCamera] awaiting_result_item entry={} winner_raw=0x{:x}",
                entry,
                winner_raw
            );
        }
        if ACTIVE_SUBJECT_ID != 0 {
            release_active_subject("result_item_missing");
        }
    }
}

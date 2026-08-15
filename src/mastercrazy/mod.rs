use crate::boss_helpers;
use crate::boss_runtime::{self, BossCommonRuntime, CommonRuntimeSyncGuard};
use crate::config::CONFIG;
use crate::selection;
use skyline::hooks::InlineCtx;
use smash::app::lua_bind;
use smash::app::lua_bind::*;
use smash::app::sv_battle_object;
use smash::app::sv_information;
use smash::app::BattleObjectModuleAccessor;
use smash::app::FighterUtil;
use smash::app::ItemKind;
use smash::hash40;
use smash::lib::{lua_const::*, L2CValue};
use smash::lua2cpp::{L2CAgentBase, L2CFighterCommon};
use smash::phx::Hash40;
use smash::phx::{Vector3f, Vector4f};
use std::arch::asm;
use std::sync::Once;
use std::u32;

// Global
static mut BARK: bool = false;
static mut PUNCH: bool = false;
static mut SHOCK: bool = false;
static mut LASER: bool = false;
static mut SCRATCH_BLOW: bool = false;
static mut FINDER: bool = false;
static mut MASTER_FINDER_ACTIVE: bool = false;
static mut CRAZY_FINDER_ACTIVE: bool = false;
static mut FINDER_SYNC_FRAMES: i32 = 0;
static mut FINDER_CAMERA_APPLIED: bool = false;
static mut FINDER_DEAD_RANGE_APPLIED: bool = false;
static mut FINDER_BASE_RANGE_CAPTURED: bool = false;
static mut FINDER_NATIVE_ACTIVE_SEEN: bool = false;
static mut FINDER_COOLDOWN_FRAMES: i32 = 0;
static mut FINDER_MASTER_ENTRY: usize = 8;
static mut FINDER_CRAZY_ENTRY: usize = 8;
static mut FINDER_BASE_RANGE: Vector4f = Vector4f {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 0.0,
};
static mut CONTROL_SPEED_MUL: f32 = 2.0;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;

static mut MASTER_X_POS: f32 = 0.0;
static mut MASTER_Y_POS: f32 = 0.0;
static mut MASTER_Z_POS: f32 = 0.0;
static mut MASTER_USABLE: bool = false;
static mut MASTER_FACING_LEFT: bool = true;
static mut CONTROLLER_X_MASTER: f32 = 0.0;
static mut CONTROLLER_Y_MASTER: f32 = 0.0;

static mut CRAZY_X_POS: f32 = 0.0;
static mut CRAZY_Y_POS: f32 = 0.0;
static mut CRAZY_Z_POS: f32 = 0.0;
static mut CRAZY_USABLE: bool = false;
static mut CRAZY_FACING_RIGHT: bool = true;
static mut CONTROLLER_X_CRAZY: f32 = 0.0;
static mut CONTROLLER_Y_CRAZY: f32 = 0.0;

// Master Hand
static mut CONTROLLABLE: bool = true;
static mut ENTRY_ID: usize = 0;
static mut BOSS_ID: [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut MULTIPLE_BULLETS: usize = 0;
static mut DEAD: bool = false;
static mut JUMP_START: bool = false;
static mut RESULT_SPAWNED: bool = false;
static mut STOP: bool = false;
static mut MASTER_EXISTS: bool = false;
static mut EXISTS_PUBLIC: bool = false;
static mut Y_POS: f32 = 0.0;
static mut MASTER_TEAM: u64 = 99;
static mut MASTER_LAST_IRON_BALL_ID: u32 = 0;
static mut MASTER_IRON_BALL_OFFSTAGE_FRAMES: i32 = 0;
static mut MASTER_IRON_BALL_SMOOTH_CANCEL: bool = false;
static mut MASTER_KENZAN_SPAWNED: bool = false;
static mut MASTER_CPU_IDLE_STALL_FRAMES: [i32; 8] = [0; 8];
static mut MASTER_CPU_LAST_X: [f32; 8] = [0.0; 8];
static mut MASTER_CPU_LAST_Y: [f32; 8] = [0.0; 8];
static mut MASTER_CPU_RECOVERY_LOG_COOLDOWN: [i32; 8] = [0; 8];

// Crazy Hand
static mut CONTROLLABLE_2: bool = true;
static mut ENTRY_ID_2: usize = 0;
static mut BOSS_ID_2: [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER_2: usize = 0;
static mut DEAD_2: bool = false;
static mut JUMP_START_2: bool = false;
static mut RESULT_SPAWNED_2: bool = false;
static mut STOP_2: bool = false;
static mut CRAZY_EXISTS: bool = false;
static mut EXISTS_PUBLIC_2: bool = false;
static mut Y_POS_2: f32 = 0.0;
static mut CRAZY_TEAM: u64 = 98;
static mut CRAZY_KUMO_ACTIVE: bool = false;
static mut CRAZY_KUMO_START_Y: f32 = 0.0;
static mut CRAZY_KUMO_ENDING: bool = false;
static mut CRAZY_CPU_IDLE_STALL_FRAMES: [i32; 8] = [0; 8];
static mut CRAZY_CPU_LAST_X: [f32; 8] = [0.0; 8];
static mut CRAZY_CPU_LAST_Y: [f32; 8] = [0.0; 8];
static mut CRAZY_CPU_RECOVERY_LOG_COOLDOWN: [i32; 8] = [0; 8];
static mut CRAZY_FIRE_CHARIOT_PINKY_LATCH: [bool; 8] = [false; 8];
static mut CRAZY_FIRE_CHARIOT_THUMB_LATCH: [bool; 8] = [false; 8];

// A paired hand move is a short-lived ownership window.  It prevents the
// normal CPU recovery/status correction code from changing one hand while the
// other hand is in the synchronized native move, then restores each item's
// prior owner flag when the window closes.
static mut HAND_TEAM_AUTHORITY_ACTIVE: bool = false;
static mut HAND_TEAM_ACTION: i32 = 0;
static mut HAND_TEAM_INITIATOR_ENTRY: usize = usize::MAX;
static mut HAND_TEAM_MASTER_ENTRY: usize = usize::MAX;
static mut HAND_TEAM_CRAZY_ENTRY: usize = usize::MAX;
static mut HAND_TEAM_MASTER_ID: u32 = 0;
static mut HAND_TEAM_CRAZY_ID: u32 = 0;
static mut HAND_TEAM_MASTER_PLAYER_WAS_SET: bool = false;
static mut HAND_TEAM_CRAZY_PLAYER_WAS_SET: bool = false;
static mut HAND_TEAM_REQUESTED_MASTER_STATUS: i32 = -1;
static mut HAND_TEAM_REQUESTED_CRAZY_STATUS: i32 = -1;
static mut HAND_TEAM_LAST_STATUS_SIGNATURE: u64 = u64::MAX;

// Entrance is separate from the normal paired-attack authority. The native
// entry2 animation is a short pre-Ready-Go window, so it needs its own
// ownership barrier when one hand is an operation CPU.
static mut HAND_ENTRANCE_AUTHORITY_ACTIVE: bool = false;
static mut HAND_ENTRANCE_MASTER_ENTRY: usize = usize::MAX;
static mut HAND_ENTRANCE_CRAZY_ENTRY: usize = usize::MAX;
static mut HAND_ENTRANCE_MASTER_ID: u32 = 0;
static mut HAND_ENTRANCE_CRAZY_ID: u32 = 0;
static mut HAND_ENTRANCE_MASTER_PLAYER_WAS_SET: bool = false;
static mut HAND_ENTRANCE_CRAZY_PLAYER_WAS_SET: bool = false;
static mut HAND_ENTRANCE_MASTER_SEEN: bool = false;
static mut HAND_ENTRANCE_CRAZY_SEEN: bool = false;
static mut HAND_ENTRANCE_MASTER_STATUS_ACCEPTED: bool = false;
static mut HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED: bool = false;
static mut HAND_ENTRANCE_TICKS: i32 = 0;
static mut HAND_ENTRANCE_LAST_SIGNATURE: u64 = u64::MAX;
static mut HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED: bool = false;
static mut HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK: i32 = -1;
// Entry2 is requested from both hidden-host callbacks.  Keep one stable
// stage-local anchor instead of allowing the second callback to move the pair
// toward a different host or the dead-area midpoint.
static mut HAND_ENTRANCE_ANCHOR_VALID: bool = false;
static mut HAND_ENTRANCE_ANCHOR_X: f32 = 0.0;
static mut HAND_ENTRANCE_ANCHOR_Y: f32 = 0.0;
static mut HAND_ENTRANCE_ANCHOR_Z: f32 = 0.0;
// A completed or failed entrance is terminal for the current pre-Ready-Go
// lifecycle. Without this latch, a hand that remains in the native wait state
// can satisfy the discovery predicate again and re-request Entry2 every frame.
static mut HAND_ENTRANCE_DONE: bool = false;
// Keep the authority claim separate from live-object validation. Spawn
// bookkeeping can observe a transient native state while the item objects are
// still valid; only the entrance coordinator may release this claim.
const HAND_ENTRANCE_PHASE_IDLE: u8 = 0;
const HAND_ENTRANCE_PHASE_REQUESTED: u8 = 1;
const HAND_ENTRANCE_PHASE_ACTIVE: u8 = 2;
static mut HAND_ENTRANCE_PHASE: u8 = HAND_ENTRANCE_PHASE_IDLE;
const HAND_ENTRANCE_TIMEOUT: i32 = 180;
const HAND_ENTRANCE_ANCHOR_FRAMES: i32 = 60;
static mut FINDER_LAST_STATUS_SIGNATURE: u64 = u64::MAX;
static mut FINDER_TRIGGER_LATCH: [bool; 8] = [false; 8];
static mut FINDER_LAST_REQUEST_SIGNATURE: u64 = u64::MAX;

// Logging snapshots copy mutable static state before formatting. Apart from
// avoiding mutable-static references in the formatter, this makes one log
// line describe a single coherent authority state.
#[derive(Copy, Clone)]
struct HandTeamLogSnapshot {
    action: i32,
    initiator_entry: usize,
    master_entry: usize,
    crazy_entry: usize,
    master_id: u32,
    crazy_id: u32,
    requested_master_status: i32,
    requested_crazy_status: i32,
}

#[inline(always)]
unsafe fn hand_team_log_snapshot() -> HandTeamLogSnapshot {
    HandTeamLogSnapshot {
        action: HAND_TEAM_ACTION,
        initiator_entry: HAND_TEAM_INITIATOR_ENTRY,
        master_entry: HAND_TEAM_MASTER_ENTRY,
        crazy_entry: HAND_TEAM_CRAZY_ENTRY,
        master_id: HAND_TEAM_MASTER_ID,
        crazy_id: HAND_TEAM_CRAZY_ID,
        requested_master_status: HAND_TEAM_REQUESTED_MASTER_STATUS,
        requested_crazy_status: HAND_TEAM_REQUESTED_CRAZY_STATUS,
    }
}

#[derive(Copy, Clone)]
struct HandEntranceLogSnapshot {
    master_entry: usize,
    crazy_entry: usize,
    master_id: u32,
    crazy_id: u32,
    phase: u8,
    master_seen: bool,
    crazy_seen: bool,
}

#[inline(always)]
unsafe fn hand_entrance_log_snapshot() -> HandEntranceLogSnapshot {
    HandEntranceLogSnapshot {
        master_entry: HAND_ENTRANCE_MASTER_ENTRY,
        crazy_entry: HAND_ENTRANCE_CRAZY_ENTRY,
        master_id: HAND_ENTRANCE_MASTER_ID,
        crazy_id: HAND_ENTRANCE_CRAZY_ID,
        phase: HAND_ENTRANCE_PHASE,
        master_seen: HAND_ENTRANCE_MASTER_SEEN,
        crazy_seen: HAND_ENTRANCE_CRAZY_SEEN,
    }
}

const HAND_TEAM_ACTION_BARK: i32 = 1;
const HAND_TEAM_ACTION_PUNCH: i32 = 2;
const HAND_TEAM_ACTION_SHOCK: i32 = 3;
const HAND_TEAM_ACTION_LASER: i32 = 4;
const HAND_TEAM_ACTION_SCRATCH: i32 = 5;
const HAND_TEAM_ACTION_FINDER: i32 = 6;

extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
}

extern "C" {
    #[link_name = "\u{1}_ZN3app9crazyhand14set_dead_rangeERKN3phx8Vector4fE"]
    pub fn crazyhand_set_dead_range(range: *const Vector4f);
}

extern "C" {
    #[link_name = "\u{1}_ZN3app9crazyhand13revert_cameraEv"]
    pub fn crazyhand_revert_camera();
}

extern "C" {
    #[link_name = "\u{1}_ZN3app10item_other6actionEPNS_26BattleObjectModuleAccessorEif"]
    pub fn action(module_accessor: *mut BattleObjectModuleAccessor, action: i32, unk: f32);
}

extern "C" {
    #[link_name = "\u{1}_ZN3app4item8owner_idEP9lua_State"]
    pub fn owner_id(lua_state: u64) -> u32;
}

extern "C" {
    #[link_name = "\u{1}_ZN3app10item_other6removeEPNS_26BattleObjectModuleAccessorE"]
    pub fn remove(module_accessor: *mut BattleObjectModuleAccessor);
}

const ITEM_INSTANCE_WORK_FLAG_PLAYER: i32 = 0x20000033;
const ITEM_INSTANCE_WORK_INT_ENTRY_ID: i32 = 0x20000036;

static mut MH_CHAKRAM_THROW_SUB: usize = 0x5643f0;
static mut MH_IRON_BALL_THROW_SUB: usize = 0x569d50;
static mut MH_KENZAN_NEEDLE_SUB: usize = 0x56e7f0;
static mut MH_WAIT_TIME_SETTING: usize = 0x54cd90;
static mut CH_FIRE_CHARIOT_MOTION: usize = 0x36ba10;
static mut CH_CHARIOT_SPEED: usize = 0x36c038;
static mut CH_CHARIOT_RADIUS_MIN: usize = 0x36c0fc;
static mut CH_CHARIOT_RADIUS_MAX: usize = 0x36c0fc;

static MASTERCRAZY_ITEM_HOOKS_ONCE: Once = Once::new();
static MASTERCRAZY_NRO_HOOK_ONCE: Once = Once::new();

const MASTER_FLOAT_FLOOR_CLEARANCE: f32 = 0.1;
const CRAZY_FLOAT_FLOOR_CLEARANCE: f32 = 0.1;
const MASTER_KENZAN_GROUND_CLEARANCE: f32 = 0.5;
const MASTER_KENZAN_SPAWN_X_OFFSET: f32 = 18.5;
const CRAZY_KUMO_ASCENT: f32 = 70.0;
const CRAZY_KUMO_DESCEND_FRAME: f32 = 110.0;
const CRAZY_KUMO_GROUND_CLEARANCE: f32 = 0.1;
const CRAZY_NOTAUTSU_GROUND_CLEARANCE: f32 = 0.1;
const MASTER_IRON_BALL_OFFSTAGE_LIMIT: i32 = 30;
const MASTER_IRON_BALL_END_TAIL_FRAMES: f32 = 40.0;
const CRAZY_KUMO_END_TAIL_FRAMES: f32 = 45.0;
const FINDER_HAND_SPACING: f32 = 24.0;
const FINDER_HAND_HEIGHT: f32 = 10.0;
const FINDER_MASTER_HEIGHT_OFFSET: f32 = 70.0;
const FINDER_COOLDOWN_DURATION: i32 = 240;

#[inline(always)]
unsafe fn boss_floor_y(
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
) -> Option<f32> {
    if module_accessor.is_null() || boss_boma.is_null() {
        return None;
    }
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(boss_boma),
        y: PostureModule::pos_y(boss_boma),
        z: PostureModule::pos_z(boss_boma),
    };
    let probe_pos = Vector3f {
        x: boss_pos.x,
        y: boss_pos.y + 60.0,
        z: boss_pos.z,
    };
    let probe_dist =
        GroundModule::get_distance_to_floor(module_accessor, &probe_pos, probe_pos.y, true);
    if probe_dist > 0.0 && probe_dist < 400.0 {
        Some(probe_pos.y - probe_dist)
    } else {
        None
    }
}

#[inline(always)]
unsafe fn boss_floor_dist(
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
) -> f32 {
    if module_accessor.is_null() || boss_boma.is_null() {
        return -1.0;
    }
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(boss_boma),
        y: PostureModule::pos_y(boss_boma),
        z: PostureModule::pos_z(boss_boma),
    };
    GroundModule::get_distance_to_floor(module_accessor, &boss_pos, boss_pos.y, true)
}

#[inline(always)]
unsafe fn reset_master_cpu_idle_recovery(entry_id: usize) {
    if entry_id < 8 {
        MASTER_CPU_IDLE_STALL_FRAMES[entry_id] = 0;
        MASTER_CPU_LAST_X[entry_id] = 0.0;
        MASTER_CPU_LAST_Y[entry_id] = 0.0;
        MASTER_CPU_RECOVERY_LOG_COOLDOWN[entry_id] = 0;
    }
}

#[inline(always)]
unsafe fn reset_crazy_cpu_idle_recovery(entry_id: usize) {
    if entry_id < 8 {
        CRAZY_CPU_IDLE_STALL_FRAMES[entry_id] = 0;
        CRAZY_CPU_LAST_X[entry_id] = 0.0;
        CRAZY_CPU_LAST_Y[entry_id] = 0.0;
        CRAZY_CPU_RECOVERY_LOG_COOLDOWN[entry_id] = 0;
    }
}

#[inline(always)]
unsafe fn reset_crazy_fire_chariot_latches(entry_id: usize) {
    if entry_id < 8 {
        CRAZY_FIRE_CHARIOT_PINKY_LATCH[entry_id] = false;
        CRAZY_FIRE_CHARIOT_THUMB_LATCH[entry_id] = false;
    }
}

#[inline(always)]
unsafe fn master_cpu_wait_family_status(status: i32) -> bool {
    status == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE
        || status == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT
        || status == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME
        || status == *ITEM_MASTERHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT
        || status == *ITEM_MASTERHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT
        || status == *ITEM_MASTERHAND_STATUS_KIND_RND_WAIT
        || status == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TO_POINT
        || status == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
        || status == *ITEM_STATUS_KIND_WAIT
}

#[inline(always)]
unsafe fn crazy_cpu_wait_family_status(status: i32) -> bool {
    status == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE
        || status == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT
        || status == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME
        || status == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT
        || status == *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT
        || status == *ITEM_CRAZYHAND_STATUS_KIND_RND_WAIT
        || status == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TO_POINT
        || status == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
        || status == *ITEM_STATUS_KIND_WAIT
}

#[inline(always)]
unsafe fn maybe_recover_master_cpu_idle(
    boss_boma: *mut BattleObjectModuleAccessor,
    entry_id: usize,
) {
    if boss_boma.is_null() || entry_id >= 8 {
        return;
    }
    // Pair actions temporarily own both item objects. Recovery is a safety
    // net for an idle CPU hand, not an authority that may rewrite a native
    // synchronized status while the partner is acting.
    if hand_team_authority_active_for_boma(boss_boma) {
        reset_master_cpu_idle_recovery(entry_id);
        return;
    }
    let status = StatusModule::status_kind(boss_boma);
    if !master_cpu_wait_family_status(status) {
        reset_master_cpu_idle_recovery(entry_id);
        return;
    }
    if MASTER_CPU_RECOVERY_LOG_COOLDOWN[entry_id] > 0 {
        MASTER_CPU_RECOVERY_LOG_COOLDOWN[entry_id] -= 1;
    }

    let current_x = PostureModule::pos_x(boss_boma);
    let current_y = PostureModule::pos_y(boss_boma);
    let moved = (current_x - MASTER_CPU_LAST_X[entry_id]).abs()
        + (current_y - MASTER_CPU_LAST_Y[entry_id]).abs();

    if moved < 0.25 {
        MASTER_CPU_IDLE_STALL_FRAMES[entry_id] += 1;
    } else {
        MASTER_CPU_IDLE_STALL_FRAMES[entry_id] = 0;
    }

    MASTER_CPU_LAST_X[entry_id] = current_x;
    MASTER_CPU_LAST_Y[entry_id] = current_y;

    if MASTER_CPU_IDLE_STALL_FRAMES[entry_id] >= 90 {
        MASTER_CPU_IDLE_STALL_FRAMES[entry_id] = 0;
        let should_log = MASTER_CPU_RECOVERY_LOG_COOLDOWN[entry_id] == 0;
        MASTER_CPU_RECOVERY_LOG_COOLDOWN[entry_id] = 300;
        MotionModule::change_motion(
            boss_boma,
            Hash40::new("wait"),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
        StatusModule::change_status_request_from_script(
            boss_boma,
            *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
            true,
        );
        if should_log {
            crate::boss_log!(
                "[PB][MasterHand][CPURecovery] entry={} status={} pos=({:.2},{:.2},{:.2}) cooldown=300",
                entry_id,
                status,
                current_x,
                current_y,
                PostureModule::pos_z(boss_boma),
            );
        }
    }
}

#[inline(always)]
unsafe fn maybe_recover_crazy_cpu_idle(
    boss_boma: *mut BattleObjectModuleAccessor,
    entry_id: usize,
) {
    if boss_boma.is_null() || entry_id >= 8 {
        return;
    }
    if hand_team_authority_active_for_boma(boss_boma) {
        reset_crazy_cpu_idle_recovery(entry_id);
        return;
    }
    let status = StatusModule::status_kind(boss_boma);
    if !crazy_cpu_wait_family_status(status) {
        reset_crazy_cpu_idle_recovery(entry_id);
        return;
    }
    if CRAZY_CPU_RECOVERY_LOG_COOLDOWN[entry_id] > 0 {
        CRAZY_CPU_RECOVERY_LOG_COOLDOWN[entry_id] -= 1;
    }

    let current_x = PostureModule::pos_x(boss_boma);
    let current_y = PostureModule::pos_y(boss_boma);
    let moved = (current_x - CRAZY_CPU_LAST_X[entry_id]).abs()
        + (current_y - CRAZY_CPU_LAST_Y[entry_id]).abs();

    if moved < 0.25 {
        CRAZY_CPU_IDLE_STALL_FRAMES[entry_id] += 1;
    } else {
        CRAZY_CPU_IDLE_STALL_FRAMES[entry_id] = 0;
    }

    CRAZY_CPU_LAST_X[entry_id] = current_x;
    CRAZY_CPU_LAST_Y[entry_id] = current_y;

    if CRAZY_CPU_IDLE_STALL_FRAMES[entry_id] >= 90 {
        CRAZY_CPU_IDLE_STALL_FRAMES[entry_id] = 0;
        let should_log = CRAZY_CPU_RECOVERY_LOG_COOLDOWN[entry_id] == 0;
        CRAZY_CPU_RECOVERY_LOG_COOLDOWN[entry_id] = 300;
        MotionModule::change_motion(
            boss_boma,
            Hash40::new("wait"),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
        StatusModule::change_status_request_from_script(
            boss_boma,
            *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
            true,
        );
        if should_log {
            crate::boss_log!(
                "[PB][CrazyHand][CPURecovery] entry={} status={} pos=({:.2},{:.2},{:.2}) cooldown=300",
                entry_id,
                status,
                current_x,
                current_y,
                PostureModule::pos_z(boss_boma),
            );
        }
    }
}

#[inline(always)]
unsafe fn current_master_boma() -> *mut BattleObjectModuleAccessor {
    if ENTRY_ID < 8 && BOSS_ID[ENTRY_ID] != 0 {
        sv_battle_object::module_accessor(BOSS_ID[ENTRY_ID])
    } else {
        core::ptr::null_mut()
    }
}

unsafe fn finder_master_entry_boma() -> (usize, *mut BattleObjectModuleAccessor) {
    if FINDER_MASTER_ENTRY < 8 {
        let boss_id = BOSS_ID[FINDER_MASTER_ENTRY];
        if boss_id != 0 && sv_battle_object::is_active(boss_id) {
            let boss_boma = sv_battle_object::module_accessor(boss_id);
            if !boss_boma.is_null()
                && smash::app::utility::get_kind(&mut *boss_boma) == *ITEM_KIND_MASTERHAND
            {
                return (FINDER_MASTER_ENTRY, boss_boma);
            }
        }
    }
    if FINDER {
        return (usize::MAX, core::ptr::null_mut());
    }
    let mut fallback_entry = usize::MAX;
    let mut fallback_boma: *mut BattleObjectModuleAccessor = core::ptr::null_mut();
    for entry in 0..8 {
        let boss_id = BOSS_ID[entry];
        if boss_id == 0 || !sv_battle_object::is_active(boss_id) {
            continue;
        }
        let boss_boma = sv_battle_object::module_accessor(boss_id);
        if boss_boma.is_null() {
            continue;
        }
        if smash::app::utility::get_kind(&mut *boss_boma) != *ITEM_KIND_MASTERHAND {
            continue;
        }
        if fallback_boma.is_null() {
            fallback_entry = entry;
            fallback_boma = boss_boma;
        }
        if TeamModule::team_no(boss_boma) == CRAZY_TEAM {
            return (entry, boss_boma);
        }
    }
    (fallback_entry, fallback_boma)
}

#[inline(always)]
unsafe fn finder_crazy_entry_boma() -> (usize, *mut BattleObjectModuleAccessor) {
    if FINDER_CRAZY_ENTRY < 8 {
        let boss_id = BOSS_ID_2[FINDER_CRAZY_ENTRY];
        if boss_id != 0 && sv_battle_object::is_active(boss_id) {
            let boss_boma = sv_battle_object::module_accessor(boss_id);
            if !boss_boma.is_null()
                && smash::app::utility::get_kind(&mut *boss_boma) == *ITEM_KIND_CRAZYHAND
            {
                return (FINDER_CRAZY_ENTRY, boss_boma);
            }
        }
    }
    if FINDER {
        return (usize::MAX, core::ptr::null_mut());
    }
    if ENTRY_ID_2 < 8 && BOSS_ID_2[ENTRY_ID_2] != 0 {
        let boss_id = BOSS_ID_2[ENTRY_ID_2];
        if sv_battle_object::is_active(boss_id) {
            let boss_boma = sv_battle_object::module_accessor(boss_id);
            if !boss_boma.is_null()
                && smash::app::utility::get_kind(&mut *boss_boma) == *ITEM_KIND_CRAZYHAND
            {
                return (ENTRY_ID_2, boss_boma);
            }
        }
    }
    (usize::MAX, core::ptr::null_mut())
}

#[inline(always)]
unsafe fn finder_master_for_crazy(
    crazy_boma: *mut BattleObjectModuleAccessor,
) -> (usize, *mut BattleObjectModuleAccessor) {
    if crazy_boma.is_null() {
        return (usize::MAX, core::ptr::null_mut());
    }
    let crazy_team = TeamModule::team_no(crazy_boma);
    let mut matching_entry = usize::MAX;
    let mut matching_boma: *mut BattleObjectModuleAccessor = core::ptr::null_mut();
    for entry in 0..8 {
        let boss_id = BOSS_ID[entry];
        if boss_id == 0 || !sv_battle_object::is_active(boss_id) {
            continue;
        }
        let master_boma = sv_battle_object::module_accessor(boss_id);
        if master_boma.is_null()
            || smash::app::utility::get_kind(&mut *master_boma) != *ITEM_KIND_MASTERHAND
            || master_boma == crazy_boma
            || TeamModule::team_no(master_boma) != crazy_team
        {
            continue;
        }
        if !matching_boma.is_null() {
            // An ambiguous same-team topology is unsafe: do not let one
            // Crazy Hand steal another pair's Master Hand.
            return (usize::MAX, core::ptr::null_mut());
        }
        matching_entry = entry;
        matching_boma = master_boma;
    }
    (matching_entry, matching_boma)
}

#[inline(always)]
unsafe fn tick_finder_cooldown() {
    if FINDER_COOLDOWN_FRAMES > 0 {
        FINDER_COOLDOWN_FRAMES -= 1;
    }
}

#[inline(always)]
unsafe fn finder_native_status(boma: *mut BattleObjectModuleAccessor, master: bool) -> bool {
    if boma.is_null() {
        return false;
    }
    let status = StatusModule::status_kind(boma);
    if master {
        status == *ITEM_MASTERHAND_STATUS_KIND_FINDER
    } else {
        status == *ITEM_CRAZYHAND_STATUS_KIND_FINDER
    }
}

#[inline(always)]
unsafe fn finder_ready_status(status: i32, master: bool) -> bool {
    let is_compound_wait = if master {
        status == *ITEM_MASTERHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT
    } else {
        status == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT
    };
    if is_compound_wait {
        return false;
    }
    if master {
        master_cpu_wait_family_status(status)
    } else {
        crazy_cpu_wait_family_status(status)
    }
}

#[inline(always)]
fn finder_dead_range_is_valid(range: Vector4f) -> bool {
    range.x.is_finite()
        && range.y.is_finite()
        && range.z.is_finite()
        && range.w.is_finite()
        && range.x < range.y
        && range.z > range.w
}

#[inline(always)]
unsafe fn clear_finder_runtime(reason: &str) {
    let master_entry = FINDER_MASTER_ENTRY;
    let crazy_entry = FINDER_CRAZY_ENTRY;
    let (_, master_boma) = finder_master_entry_boma();
    let (_, crazy_boma) = finder_crazy_entry_boma();
    let master_status = if master_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(master_boma)
    };
    let crazy_status = if crazy_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(crazy_boma)
    };
    let native_master = finder_native_status(master_boma, true);
    let native_crazy = finder_native_status(crazy_boma, false);
    let base_captured = FINDER_BASE_RANGE_CAPTURED;
    let base = core::ptr::addr_of!(FINDER_BASE_RANGE).read();
    let elapsed_sync_frames = FINDER_SYNC_FRAMES;

    // Native Finder owns the camera/dead-area transition. These calls are the
    // abort/reset safety net for deaths, scene changes, and failed activation.
    if !crazy_boma.is_null() {
        WorkModule::off_flag(
            crazy_boma,
            *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FINDER_SHIRINK_START,
        );
        if native_crazy {
            CameraModule::reset_camera_range(crazy_boma, 0);
        }
    }
    if !master_boma.is_null() && native_master {
        CameraModule::reset_camera_range(master_boma, 0);
    }
    if base_captured {
        crazyhand_revert_camera();
        crazyhand_set_dead_range(&base);
    }

    // Do not turn a dead/despawned hand back into a live wait state. On an
    // abort, only unwind a hand that is still in the native Finder protocol.
    if reason != "normal_complete" {
        if !master_boma.is_null() && native_master && !DEAD && !STOP && !RESULT_SPAWNED {
            let next_status = if master_entry < 8
                && boss_helpers::is_operation_cpu_entry(
                    boss_helpers::fighter_manager(),
                    master_entry,
                ) {
                *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE
            } else {
                *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
            };
            StatusModule::change_status_request_from_script(master_boma, next_status, true);
        }
        if !crazy_boma.is_null() && native_crazy && !DEAD_2 && !STOP_2 && !RESULT_SPAWNED_2 {
            let next_status = if crazy_entry < 8
                && boss_helpers::is_operation_cpu_entry(
                    boss_helpers::fighter_manager(),
                    crazy_entry,
                ) {
                *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE
            } else {
                *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
            };
            StatusModule::change_status_request_from_script(crazy_boma, next_status, true);
        }
    }

    let restored_master_status = if master_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(master_boma)
    };
    let restored_crazy_status = if crazy_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(crazy_boma)
    };
    let restored_master_camera_type = if master_boma.is_null() {
        0
    } else {
        CameraModule::get_camera_type(master_boma)
    };
    let restored_crazy_camera_type = if crazy_boma.is_null() {
        0
    } else {
        CameraModule::get_camera_type(crazy_boma)
    };
    let restored_master_clip_in =
        !master_boma.is_null() && CameraModule::is_clip_in(master_boma, false);
    let restored_master_clip_in_all =
        !master_boma.is_null() && CameraModule::is_clip_in_all(master_boma, false);
    let restored_crazy_clip_in =
        !crazy_boma.is_null() && CameraModule::is_clip_in(crazy_boma, false);
    let restored_crazy_clip_in_all =
        !crazy_boma.is_null() && CameraModule::is_clip_in_all(crazy_boma, false);
    let restored_crazy_shrink_flag = !crazy_boma.is_null()
        && WorkModule::is_flag(
            crazy_boma,
            *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FINDER_SHIRINK_START,
        );

    release_hand_team_authority(reason);

    FINDER = false;
    MASTER_FINDER_ACTIVE = false;
    CRAZY_FINDER_ACTIVE = false;
    FINDER_SYNC_FRAMES = 0;
    FINDER_CAMERA_APPLIED = false;
    FINDER_DEAD_RANGE_APPLIED = false;
    FINDER_BASE_RANGE_CAPTURED = false;
    FINDER_NATIVE_ACTIVE_SEEN = false;
    FINDER_LAST_STATUS_SIGNATURE = u64::MAX;
    FINDER_LAST_REQUEST_SIGNATURE = u64::MAX;
    FINDER_MASTER_ENTRY = 8;
    FINDER_CRAZY_ENTRY = 8;
    if reason == "normal_complete" {
        FINDER_COOLDOWN_FRAMES = FINDER_COOLDOWN_DURATION;
    }
    FINDER_BASE_RANGE = Vector4f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };

    let fighter_manager = boss_helpers::fighter_manager();
    if !fighter_manager.is_null() {
        if master_entry < 8 && !boss_helpers::is_operation_cpu_entry(fighter_manager, master_entry)
        {
            CONTROLLABLE = true;
        }
        if crazy_entry < 8 && !boss_helpers::is_operation_cpu_entry(fighter_manager, crazy_entry) {
            CONTROLLABLE_2 = true;
        }
    }

    crate::boss_log!(
        "[PB][Finder] restored=true reason={} start_sync_frame=0 exit_sync_frame={} master_entry={} crazy_entry={} status_before=master:{} crazy:{} status_after=master:{} crazy:{} captured_dead_area=({:.1},{:.1},{:.1},{:.1}) restored_dead_area=({:.1},{:.1},{:.1},{:.1}) crazy_shrink_flag={} camera_state=restored master_camera_type=0x{:x} crazy_camera_type=0x{:x} master_clip_in={} master_clip_in_all={} crazy_clip_in={} crazy_clip_in_all={}",
        reason,
        elapsed_sync_frames,
        master_entry,
        crazy_entry,
        master_status,
        crazy_status,
        restored_master_status,
        restored_crazy_status,
        base.x,
        base.y,
        base.z,
        base.w,
        base.x,
        base.y,
        base.z,
        base.w,
        restored_crazy_shrink_flag,
        restored_master_camera_type,
        restored_crazy_camera_type,
        restored_master_clip_in,
        restored_master_clip_in_all,
        restored_crazy_clip_in,
        restored_crazy_clip_in_all
    );
}

#[inline(always)]
unsafe fn clear_finder_runtime_with_reason(reason: &str) {
    if !FINDER && !FINDER_BASE_RANGE_CAPTURED {
        return;
    }
    let (_, master_boma) = finder_master_entry_boma();
    let (_, crazy_boma) = finder_crazy_entry_boma();
    let master_entry = FINDER_MASTER_ENTRY;
    let crazy_entry = FINDER_CRAZY_ENTRY;
    crate::boss_log!(
        "[PB][Finder] restoration_reason={} master_entry={} crazy_entry={} native_status=master:{} crazy:{}",
        reason,
        master_entry,
        crazy_entry,
        if finder_native_status(master_boma, true) { "finder" } else { "other" },
        if finder_native_status(crazy_boma, false) { "finder" } else { "other" }
    );
    clear_finder_runtime(reason);
}

#[inline(always)]
unsafe fn finder_anchor_pos(reference_boma: *mut BattleObjectModuleAccessor) -> Vector3f {
    let base = core::ptr::addr_of!(FINDER_BASE_RANGE).read();
    let center_x = (base.x + base.y) * 0.5;
    let center_y = if reference_boma.is_null() {
        (MASTER_Y_POS + CRAZY_Y_POS) * 0.5
    } else {
        let probe_y = MASTER_Y_POS.max(CRAZY_Y_POS) + 80.0;
        let probe_pos = Vector3f {
            x: center_x,
            y: probe_y,
            z: PostureModule::pos_z(reference_boma),
        };
        let probe_dist =
            GroundModule::get_distance_to_floor(reference_boma, &probe_pos, probe_pos.y, true);
        if probe_dist > 0.0 && probe_dist < 400.0 {
            (probe_pos.y - probe_dist) + FINDER_HAND_HEIGHT
        } else {
            (MASTER_Y_POS + CRAZY_Y_POS) * 0.5
        }
    };
    Vector3f {
        x: center_x,
        y: center_y,
        z: if reference_boma.is_null() {
            0.0
        } else {
            PostureModule::pos_z(reference_boma)
        },
    }
}

#[inline(always)]
unsafe fn finder_hand_targets(
    reference_boma: *mut BattleObjectModuleAccessor,
) -> (Vector3f, Vector3f) {
    let anchor = finder_anchor_pos(reference_boma);
    let crazy_offset = if CRAZY_FACING_RIGHT {
        -FINDER_HAND_SPACING
    } else {
        FINDER_HAND_SPACING
    };
    let crazy_target = Vector3f {
        x: anchor.x + crazy_offset,
        y: anchor.y,
        z: anchor.z,
    };
    let master_target = Vector3f {
        x: anchor.x - crazy_offset,
        y: anchor.y + FINDER_MASTER_HEIGHT_OFFSET,
        z: anchor.z,
    };
    (master_target, crazy_target)
}

#[inline(always)]
unsafe fn start_finder_pair(lua_state: u64, crazy_boma: *mut BattleObjectModuleAccessor) -> bool {
    if FINDER {
        return false;
    }
    if crazy_boma.is_null() {
        return false;
    }

    let crazy_status = StatusModule::status_kind(crazy_boma);
    let (master_entry, master_boma) = finder_master_for_crazy(crazy_boma);
    let mut crazy_entry = usize::MAX;
    for entry in 0..8 {
        let boss_id = BOSS_ID_2[entry];
        if boss_id == 0 || !sv_battle_object::is_active(boss_id) {
            continue;
        }
        if sv_battle_object::module_accessor(boss_id) == crazy_boma {
            crazy_entry = entry;
            break;
        }
    }
    let master_status = if master_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(master_boma)
    };
    let same_team = !master_boma.is_null()
        && TeamModule::team_no(master_boma) == TeamModule::team_no(crazy_boma);
    let host_boma = smash::app::sv_system::battle_object_module_accessor(lua_state);
    let floor_dist = boss_floor_dist(host_boma, crazy_boma);
    let cooldown_ready = FINDER_COOLDOWN_FRAMES == 0;
    let facing_ok = !master_boma.is_null()
        && ((PostureModule::lr(crazy_boma) == 1.0 && PostureModule::lr(master_boma) == -1.0)
            || (PostureModule::lr(crazy_boma) == -1.0 && PostureModule::lr(master_boma) == 1.0));
    let cooldown_frames = FINDER_COOLDOWN_FRAMES;
    let pair_ready = !master_boma.is_null()
        && master_entry < 8
        && crazy_entry < 8
        && MASTER_EXISTS
        && CRAZY_EXISTS
        && !DEAD
        && !DEAD_2
        && !STOP
        && !STOP_2
        && !RESULT_SPAWNED
        && !RESULT_SPAWNED_2
        && same_team
        && facing_ok
        && finder_ready_status(master_status, true)
        && finder_ready_status(crazy_status, false)
        && !BARK
        && !PUNCH
        && !SHOCK
        && !LASER
        && !SCRATCH_BLOW
        && cooldown_ready;

    let request_signature = (master_entry as u64)
        ^ (crazy_entry as u64).rotate_left(7)
        ^ (master_status as u32 as u64).rotate_left(13)
        ^ (crazy_status as u32 as u64).rotate_left(29)
        ^ ((same_team as u64) << 45)
        ^ ((facing_ok as u64) << 46)
        ^ ((pair_ready as u64) << 47)
        ^ ((cooldown_ready as u64) << 48);
    if request_signature != FINDER_LAST_REQUEST_SIGNATURE {
        FINDER_LAST_REQUEST_SIGNATURE = request_signature;
        crate::boss_log!(
            "[PB][Finder] request master_entry={} crazy_entry={} team_valid={} facing_valid={} floor={:.1} cooldown={} master_status={} crazy_status={} ready={}",
            master_entry,
            crazy_entry,
            same_team,
            facing_ok,
            floor_dist,
            cooldown_frames,
            master_status,
            crazy_status,
            pair_ready
        );
    }
    if !pair_ready || floor_dist <= 0.0 || floor_dist > 50.0 {
        return false;
    }

    let base_range = dead_range(lua_state);
    if !finder_dead_range_is_valid(base_range) {
        crate::boss_log!(
            "[PB][Finder] abort invalid_dead_area=({:.1},{:.1},{:.1},{:.1})",
            base_range.x,
            base_range.y,
            base_range.z,
            base_range.w
        );
        return false;
    }

    BARK = false;
    PUNCH = false;
    SHOCK = false;
    LASER = false;
    SCRATCH_BLOW = false;
    FINDER = true;
    MASTER_FINDER_ACTIVE = false;
    CRAZY_FINDER_ACTIVE = false;
    FINDER_NATIVE_ACTIVE_SEEN = false;
    FINDER_SYNC_FRAMES = 0;
    FINDER_CAMERA_APPLIED = false;
    FINDER_DEAD_RANGE_APPLIED = false;
    FINDER_BASE_RANGE = base_range;
    FINDER_BASE_RANGE_CAPTURED = true;
    FINDER_MASTER_ENTRY = master_entry;
    FINDER_CRAZY_ENTRY = crazy_entry;

    // Set the native move's initial picture-frame positions once. The native
    // Finder statuses own the animation, camera, and boundary transitions;
    // this hook must not fight them every frame.
    let (master_target, crazy_target) = finder_hand_targets(crazy_boma);
    PostureModule::set_pos(master_boma, &master_target);
    PostureModule::set_pos(crazy_boma, &crazy_target);
    CONTROLLABLE = false;
    CONTROLLABLE_2 = false;
    CONTROLLER_X_MASTER = 0.0;
    CONTROLLER_Y_MASTER = 0.0;
    CONTROLLER_X_CRAZY = 0.0;
    CONTROLLER_Y_CRAZY = 0.0;

    // These are the real item statuses used by Ultimate's compound Finder
    // protocol. Request both before returning so neither hand remains in the
    // normal gameplay path after the pair is accepted.
    if !begin_hand_team_authority(
        HAND_TEAM_ACTION_FINDER,
        crazy_entry,
        master_entry,
        master_boma,
        crazy_entry,
        crazy_boma,
    ) {
        FINDER = false;
        FINDER_BASE_RANGE_CAPTURED = false;
        crate::boss_log!(
            "[PB][Finder] abort pair_authority_failed master_entry={} crazy_entry={}",
            master_entry,
            crazy_entry
        );
        return false;
    }

    // The native compound path marks this Crazy Hand work flag before the
    // Finder status is accepted. Set the named flag before requesting either
    // status, then request Crazy first so its partner role is ready when the
    // Master Hand Finder state starts.
    WorkModule::on_flag(
        crazy_boma,
        *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FINDER_SHIRINK_START,
    );
    let crazy_request_result = StatusModule::change_status_request_from_script(
        crazy_boma,
        *ITEM_CRAZYHAND_STATUS_KIND_FINDER,
        true,
    );
    let master_request_result = StatusModule::change_status_request_from_script(
        master_boma,
        *ITEM_MASTERHAND_STATUS_KIND_FINDER,
        true,
    );
    let base = core::ptr::addr_of!(FINDER_BASE_RANGE).read();
    let master_motion = MotionModule::motion_kind(master_boma);
    let crazy_motion = MotionModule::motion_kind(crazy_boma);
    let master_frame = MotionModule::frame(master_boma);
    let crazy_frame = MotionModule::frame(crazy_boma);
    crate::boss_log!(
        "[PB][Finder] native_status=queued request_order=crazy_then_master crazy_status_id={} master_status_id={} crazy_request_result=0x{:x} master_request_result=0x{:x} crazy_shrink_flag={} start_sync_frame=0 master_entry={} crazy_entry={} status_before=master:{} crazy:{} status_after_request=master:{} crazy:{} master_motion=0x{:x} master_frame={:.1} crazy_motion=0x{:x} crazy_frame={:.1} captured_dead_area=({:.1},{:.1},{:.1},{:.1}) camera_state=native_status_pending finder_dead_area=native_status_pending",
        *ITEM_CRAZYHAND_STATUS_KIND_FINDER,
        *ITEM_MASTERHAND_STATUS_KIND_FINDER,
        crazy_request_result,
        master_request_result,
        WorkModule::is_flag(
            crazy_boma,
            *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FINDER_SHIRINK_START,
        ),
        master_entry,
        crazy_entry,
        master_status,
        crazy_status,
        StatusModule::status_kind(master_boma),
        StatusModule::status_kind(crazy_boma),
        master_motion,
        master_frame,
        crazy_motion,
        crazy_frame,
        base.x,
        base.y,
        base.z,
        base.w
    );
    true
}

#[inline(always)]
unsafe fn update_finder_runtime(lua_state: u64) {
    if !FINDER {
        return;
    }

    let (_, crazy_boma) = finder_crazy_entry_boma();
    let (_, master_boma) = finder_master_entry_boma();
    if crazy_boma.is_null()
        || master_boma.is_null()
        || !MASTER_EXISTS
        || !CRAZY_EXISTS
        || DEAD
        || DEAD_2
        || RESULT_SPAWNED
        || RESULT_SPAWNED_2
        || STOP
        || STOP_2
    {
        clear_finder_runtime_with_reason("abort");
        return;
    }

    let master_status = StatusModule::status_kind(master_boma);
    let crazy_status = StatusModule::status_kind(crazy_boma);
    let master_native = finder_native_status(master_boma, true);
    let crazy_native = finder_native_status(crazy_boma, false);
    let was_active = FINDER_NATIVE_ACTIVE_SEEN;
    MASTER_FINDER_ACTIVE = master_native;
    CRAZY_FINDER_ACTIVE = crazy_native;
    FINDER_CAMERA_APPLIED = master_native && crazy_native;
    FINDER_DEAD_RANGE_APPLIED = master_native && crazy_native;

    let status_signature = (master_status as u32 as u64)
        ^ ((crazy_status as u32 as u64) << 19)
        ^ MotionModule::motion_kind(master_boma).rotate_left(7)
        ^ MotionModule::motion_kind(crazy_boma).rotate_left(29);
    if status_signature != FINDER_LAST_STATUS_SIGNATURE {
        FINDER_LAST_STATUS_SIGNATURE = status_signature;
        let sync_frames = FINDER_SYNC_FRAMES;
        crate::boss_log!(
            "[PB][Finder] status_transition sync_frame={} master_status={} crazy_status={} master_motion=0x{:x} crazy_motion=0x{:x} crazy_shrink_flag={} native_master={} native_crazy={}",
            sync_frames,
            master_status,
            crazy_status,
            MotionModule::motion_kind(master_boma),
            MotionModule::motion_kind(crazy_boma),
            WorkModule::is_flag(
                crazy_boma,
                *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FINDER_SHIRINK_START,
            ),
            master_native,
            crazy_native
        );
    }

    if master_native && crazy_native {
        FINDER_NATIVE_ACTIVE_SEEN = true;
        CONTROLLABLE = false;
        CONTROLLABLE_2 = false;
        if !was_active {
            let base = core::ptr::addr_of!(FINDER_BASE_RANGE).read();
            let master_entry = FINDER_MASTER_ENTRY;
            let crazy_entry = FINDER_CRAZY_ENTRY;
            let active_range = dead_range(lua_state);
            let crazy_shrink_flag = WorkModule::is_flag(
                crazy_boma,
                *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FINDER_SHIRINK_START,
            );
            let master_motion = MotionModule::motion_kind(master_boma);
            let crazy_motion = MotionModule::motion_kind(crazy_boma);
            let master_frame = MotionModule::frame(master_boma);
            let crazy_frame = MotionModule::frame(crazy_boma);
            let master_camera_type = CameraModule::get_camera_type(master_boma);
            let crazy_camera_type = CameraModule::get_camera_type(crazy_boma);
            let master_clip_in = CameraModule::is_clip_in(master_boma, false);
            let master_clip_in_all = CameraModule::is_clip_in_all(master_boma, false);
            let crazy_clip_in = CameraModule::is_clip_in(crazy_boma, false);
            let crazy_clip_in_all = CameraModule::is_clip_in_all(crazy_boma, false);
            let active_sync_frame = FINDER_SYNC_FRAMES;
            crate::boss_log!(
                "[PB][Finder] active=true active_sync_frame={} master_entry={} crazy_entry={} native_status=master:{} crazy:{} master_motion=0x{:x} master_frame={:.1} crazy_motion=0x{:x} crazy_frame={:.1} captured_dead_area=({:.1},{:.1},{:.1},{:.1}) active_dead_area=({:.1},{:.1},{:.1},{:.1}) crazy_shrink_flag={} camera_state=native_status_owned master_camera_type=0x{:x} crazy_camera_type=0x{:x} master_clip_in={} master_clip_in_all={} crazy_clip_in={} crazy_clip_in_all={}",
                active_sync_frame,
                master_entry,
                crazy_entry,
                master_status,
                crazy_status,
                master_motion,
                master_frame,
                crazy_motion,
                crazy_frame,
                base.x,
                base.y,
                base.z,
                base.w,
                active_range.x,
                active_range.y,
                active_range.z,
                active_range.w,
                crazy_shrink_flag,
                master_camera_type,
                crazy_camera_type,
                master_clip_in,
                master_clip_in_all,
                crazy_clip_in,
                crazy_clip_in_all
            );
        }
    } else if FINDER_NATIVE_ACTIVE_SEEN && master_native != crazy_native {
        // Once the native move has started, one hand leaving its status before
        // its partner is an incomplete compound transition. Abort and let the
        // idempotent cleanup restore the battlefield rather than re-playing a
        // motion or forcing a surviving hand to continue alone.
        clear_finder_runtime_with_reason("native_status_mismatch");
        return;
    }

    FINDER_SYNC_FRAMES += 1;
    if !FINDER_NATIVE_ACTIVE_SEEN && FINDER_SYNC_FRAMES > 120 {
        clear_finder_runtime_with_reason("activation_timeout");
        return;
    }
    if FINDER_NATIVE_ACTIVE_SEEN && !master_native && !crazy_native {
        clear_finder_runtime_with_reason("normal_complete");
    }
}

#[inline(always)]
unsafe fn master_should_clamp_floor(boss_boma: *mut BattleObjectModuleAccessor) -> bool {
    if !CONTROLLABLE {
        return false;
    }
    let status = StatusModule::status_kind(boss_boma);
    status != *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
        && status != *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
        && status != *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
        && status != *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
}

#[inline(always)]
unsafe fn crazy_should_clamp_floor(boss_boma: *mut BattleObjectModuleAccessor) -> bool {
    if !CONTROLLABLE_2 {
        return false;
    }
    let status = StatusModule::status_kind(boss_boma);
    status != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
        && status != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
        && status != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
        && status != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LANDING
        && status != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
}

#[inline(always)]
unsafe fn weapon_owner_is_player(lua_state: u64) -> bool {
    let owner_boma = sv_battle_object::module_accessor(owner_id(lua_state));
    !owner_boma.is_null() && WorkModule::is_flag(owner_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER)
}

#[inline(always)]
unsafe fn mark_boss_player_owned(boss_boma: *mut BattleObjectModuleAccessor, entry_id: i32) {
    if boss_boma.is_null() {
        return;
    }
    WorkModule::on_flag(boss_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    WorkModule::set_int(boss_boma, entry_id, ITEM_INSTANCE_WORK_INT_ENTRY_ID);
}

#[inline(always)]
unsafe fn configure_boss_owner_mode(boss_boma: *mut BattleObjectModuleAccessor, entry_id: usize) {
    if boss_boma.is_null() {
        return;
    }
    let fighter_manager = boss_helpers::fighter_manager();
    if boss_helpers::is_operation_cpu_entry(fighter_manager, entry_id) {
        WorkModule::off_flag(boss_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        WorkModule::set_int(boss_boma, entry_id as i32, ITEM_INSTANCE_WORK_INT_ENTRY_ID);
        println!(
            "[PB][MasterCrazy] entry={} cpu item_owner=native_ai",
            entry_id,
        );
    } else {
        mark_boss_player_owned(boss_boma, entry_id as i32);
    }
}

#[inline(always)]
fn hand_team_action_name(action: i32) -> &'static str {
    match action {
        HAND_TEAM_ACTION_BARK => "bark",
        HAND_TEAM_ACTION_PUNCH => "team_punch",
        HAND_TEAM_ACTION_SHOCK => "electric_shock",
        HAND_TEAM_ACTION_LASER => "double_finger_beam",
        HAND_TEAM_ACTION_SCRATCH => "scratch",
        HAND_TEAM_ACTION_FINDER => "finder",
        _ => "none",
    }
}

#[inline(always)]
unsafe fn shared_hand_action() -> i32 {
    if FINDER {
        HAND_TEAM_ACTION_FINDER
    } else if BARK {
        HAND_TEAM_ACTION_BARK
    } else if PUNCH {
        HAND_TEAM_ACTION_PUNCH
    } else if SHOCK {
        HAND_TEAM_ACTION_SHOCK
    } else if LASER {
        HAND_TEAM_ACTION_LASER
    } else if SCRATCH_BLOW {
        HAND_TEAM_ACTION_SCRATCH
    } else {
        0
    }
}

#[inline(always)]
unsafe fn hand_team_action_statuses(action: i32) -> (i32, i32) {
    match action {
        HAND_TEAM_ACTION_BARK => (
            *ITEM_MASTERHAND_STATUS_KIND_BARK,
            *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
        ),
        HAND_TEAM_ACTION_PUNCH => (
            *ITEM_MASTERHAND_STATUS_KIND_GOOPAA,
            *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
        ),
        HAND_TEAM_ACTION_SHOCK => (
            *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START,
            *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
        ),
        HAND_TEAM_ACTION_LASER => (
            *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START,
            *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
        ),
        HAND_TEAM_ACTION_SCRATCH => (
            *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START,
            *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START,
        ),
        HAND_TEAM_ACTION_FINDER => (
            *ITEM_MASTERHAND_STATUS_KIND_FINDER,
            *ITEM_CRAZYHAND_STATUS_KIND_FINDER,
        ),
        _ => (-1, -1),
    }
}

#[inline(always)]
unsafe fn hand_team_authority_active_for_boma(boma: *mut BattleObjectModuleAccessor) -> bool {
    if !HAND_TEAM_AUTHORITY_ACTIVE || boma.is_null() {
        return false;
    }
    (HAND_TEAM_MASTER_ID != 0
        && sv_battle_object::is_active(HAND_TEAM_MASTER_ID)
        && sv_battle_object::module_accessor(HAND_TEAM_MASTER_ID) == boma)
        || (HAND_TEAM_CRAZY_ID != 0
            && sv_battle_object::is_active(HAND_TEAM_CRAZY_ID)
            && sv_battle_object::module_accessor(HAND_TEAM_CRAZY_ID) == boma)
}

#[inline(always)]
unsafe fn begin_hand_team_authority(
    action: i32,
    initiator_entry: usize,
    master_entry: usize,
    master_boma: *mut BattleObjectModuleAccessor,
    crazy_entry: usize,
    crazy_boma: *mut BattleObjectModuleAccessor,
) -> bool {
    if action == 0
        || master_entry >= 8
        || crazy_entry >= 8
        || master_boma.is_null()
        || crazy_boma.is_null()
        || !sv_battle_object::is_active(BOSS_ID[master_entry])
        || !sv_battle_object::is_active(BOSS_ID_2[crazy_entry])
        || TeamModule::team_no(master_boma) != TeamModule::team_no(crazy_boma)
    {
        return false;
    }

    if HAND_TEAM_AUTHORITY_ACTIVE {
        return HAND_TEAM_MASTER_ID == BOSS_ID[master_entry]
            && HAND_TEAM_CRAZY_ID == BOSS_ID_2[crazy_entry]
            && HAND_TEAM_ACTION == action;
    }

    let master_player_was_set = WorkModule::is_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    let crazy_player_was_set = WorkModule::is_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    let requested = hand_team_action_statuses(action);
    let fighter_manager = boss_helpers::fighter_manager();
    let master_cpu = !fighter_manager.is_null()
        && boss_helpers::is_operation_cpu_entry(fighter_manager, master_entry);
    let crazy_cpu = !fighter_manager.is_null()
        && boss_helpers::is_operation_cpu_entry(fighter_manager, crazy_entry);

    HAND_TEAM_AUTHORITY_ACTIVE = true;
    HAND_TEAM_ACTION = action;
    HAND_TEAM_INITIATOR_ENTRY = initiator_entry;
    HAND_TEAM_MASTER_ENTRY = master_entry;
    HAND_TEAM_CRAZY_ENTRY = crazy_entry;
    let master_id = BOSS_ID[master_entry];
    let crazy_id = BOSS_ID_2[crazy_entry];
    HAND_TEAM_MASTER_ID = master_id;
    HAND_TEAM_CRAZY_ID = crazy_id;
    HAND_TEAM_MASTER_PLAYER_WAS_SET = master_player_was_set;
    HAND_TEAM_CRAZY_PLAYER_WAS_SET = crazy_player_was_set;
    HAND_TEAM_REQUESTED_MASTER_STATUS = requested.0;
    HAND_TEAM_REQUESTED_CRAZY_STATUS = requested.1;
    HAND_TEAM_LAST_STATUS_SIGNATURE = u64::MAX;

    // The native item status is still the source of the animation/action. The
    // temporary player-owner flag only prevents an operation-CPU item from
    // immediately handing the synchronized status back to its generic AI.
    WorkModule::on_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    WorkModule::on_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    WorkModule::set_int(
        master_boma,
        master_entry as i32,
        ITEM_INSTANCE_WORK_INT_ENTRY_ID,
    );
    WorkModule::set_int(
        crazy_boma,
        crazy_entry as i32,
        ITEM_INSTANCE_WORK_INT_ENTRY_ID,
    );

    crate::boss_log!(
        "[PB][HandTeam] request action={} initiator_entry={} master_entry={} crazy_entry={} master_cpu={} crazy_cpu={} master_object=0x{:x} crazy_object=0x{:x} pre_status=master:{} crazy:{} pre_motion=master:0x{:x} crazy:0x{:x} requested_status=master:{} crazy:{} temporary_ai_suppression=true prior_player_flag=master:{} crazy:{} activation_result=authority_acquired",
        hand_team_action_name(action),
        initiator_entry,
        master_entry,
        crazy_entry,
        master_cpu,
        crazy_cpu,
        master_id,
        crazy_id,
        StatusModule::status_kind(master_boma),
        StatusModule::status_kind(crazy_boma),
        MotionModule::motion_kind(master_boma),
        MotionModule::motion_kind(crazy_boma),
        requested.0,
        requested.1,
        master_player_was_set,
        crazy_player_was_set
    );
    true
}

#[inline(always)]
unsafe fn log_hand_team_status() {
    if !HAND_TEAM_AUTHORITY_ACTIVE {
        return;
    }
    let snapshot = hand_team_log_snapshot();
    let master_boma = if snapshot.master_id != 0 && sv_battle_object::is_active(snapshot.master_id)
    {
        sv_battle_object::module_accessor(snapshot.master_id)
    } else {
        core::ptr::null_mut()
    };
    let crazy_boma = if snapshot.crazy_id != 0 && sv_battle_object::is_active(snapshot.crazy_id) {
        sv_battle_object::module_accessor(snapshot.crazy_id)
    } else {
        core::ptr::null_mut()
    };
    if master_boma.is_null() || crazy_boma.is_null() {
        return;
    }

    let master_status = StatusModule::status_kind(master_boma);
    let crazy_status = StatusModule::status_kind(crazy_boma);
    let master_motion = MotionModule::motion_kind(master_boma);
    let crazy_motion = MotionModule::motion_kind(crazy_boma);
    let signature = (master_status as u32 as u64)
        ^ ((crazy_status as u32 as u64) << 17)
        ^ (master_motion.rotate_left(7))
        ^ (crazy_motion.rotate_left(23));
    if signature == HAND_TEAM_LAST_STATUS_SIGNATURE {
        return;
    }
    HAND_TEAM_LAST_STATUS_SIGNATURE = signature;
    crate::boss_log!(
        "[PB][HandTeam] active=true action={} initiator_entry={} master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} requested_master_status={} requested_crazy_status={} observed_master_status={} observed_crazy_status={} observed_master_motion=0x{:x} observed_crazy_motion=0x{:x} temporary_ai_suppression=true",
        hand_team_action_name(snapshot.action),
        snapshot.initiator_entry,
        snapshot.master_entry,
        snapshot.crazy_entry,
        snapshot.master_id,
        snapshot.crazy_id,
        snapshot.requested_master_status,
        snapshot.requested_crazy_status,
        master_status,
        crazy_status,
        master_motion,
        crazy_motion
    );
}

#[inline(always)]
unsafe fn release_hand_team_authority(reason: &str) {
    if !HAND_TEAM_AUTHORITY_ACTIVE {
        return;
    }

    let master_id = HAND_TEAM_MASTER_ID;
    let crazy_id = HAND_TEAM_CRAZY_ID;
    let master_entry = HAND_TEAM_MASTER_ENTRY;
    let crazy_entry = HAND_TEAM_CRAZY_ENTRY;
    let initiator_entry = HAND_TEAM_INITIATOR_ENTRY;
    let action = HAND_TEAM_ACTION;
    let master_was_player = HAND_TEAM_MASTER_PLAYER_WAS_SET;
    let crazy_was_player = HAND_TEAM_CRAZY_PLAYER_WAS_SET;
    let master_boma = if master_id != 0 && sv_battle_object::is_active(master_id) {
        sv_battle_object::module_accessor(master_id)
    } else {
        core::ptr::null_mut()
    };
    let crazy_boma = if crazy_id != 0 && sv_battle_object::is_active(crazy_id) {
        sv_battle_object::module_accessor(crazy_id)
    } else {
        core::ptr::null_mut()
    };
    if !master_boma.is_null() {
        if master_was_player {
            WorkModule::on_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        } else {
            WorkModule::off_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        }
    }
    if !crazy_boma.is_null() {
        if crazy_was_player {
            WorkModule::on_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        } else {
            WorkModule::off_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        }
    }

    let fighter_manager = boss_helpers::fighter_manager();
    if !fighter_manager.is_null() {
        if master_entry < 8 {
            CONTROLLABLE = !boss_helpers::is_operation_cpu_entry(fighter_manager, master_entry);
        }
        if crazy_entry < 8 {
            CONTROLLABLE_2 = !boss_helpers::is_operation_cpu_entry(fighter_manager, crazy_entry);
        }
    }

    crate::boss_log!(
        "[PB][HandTeam] exit action={} initiator_entry={} master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} reason={} observed_status=master:{} crazy:{} temporary_ai_suppression=false restored_player_flag=master:{} crazy:{}",
        hand_team_action_name(action),
        initiator_entry,
        master_entry,
        crazy_entry,
        master_id,
        crazy_id,
        reason,
        if master_boma.is_null() { -1 } else { StatusModule::status_kind(master_boma) },
        if crazy_boma.is_null() { -1 } else { StatusModule::status_kind(crazy_boma) },
        master_was_player,
        crazy_was_player
    );

    HAND_TEAM_AUTHORITY_ACTIVE = false;
    HAND_TEAM_ACTION = 0;
    HAND_TEAM_INITIATOR_ENTRY = usize::MAX;
    HAND_TEAM_MASTER_ENTRY = usize::MAX;
    HAND_TEAM_CRAZY_ENTRY = usize::MAX;
    HAND_TEAM_MASTER_ID = 0;
    HAND_TEAM_CRAZY_ID = 0;
    HAND_TEAM_MASTER_PLAYER_WAS_SET = false;
    HAND_TEAM_CRAZY_PLAYER_WAS_SET = false;
    HAND_TEAM_REQUESTED_MASTER_STATUS = -1;
    HAND_TEAM_REQUESTED_CRAZY_STATUS = -1;
    HAND_TEAM_LAST_STATUS_SIGNATURE = u64::MAX;
}

#[inline(always)]
unsafe fn release_hand_entrance_authority(reason: &str) {
    if !HAND_ENTRANCE_AUTHORITY_ACTIVE {
        return;
    }

    let master_id = HAND_ENTRANCE_MASTER_ID;
    let crazy_id = HAND_ENTRANCE_CRAZY_ID;
    let master_entry = HAND_ENTRANCE_MASTER_ENTRY;
    let crazy_entry = HAND_ENTRANCE_CRAZY_ENTRY;
    let phase = HAND_ENTRANCE_PHASE;
    let master_was_player = HAND_ENTRANCE_MASTER_PLAYER_WAS_SET;
    let crazy_was_player = HAND_ENTRANCE_CRAZY_PLAYER_WAS_SET;
    let master_boma = if master_id != 0 && sv_battle_object::is_active(master_id) {
        sv_battle_object::module_accessor(master_id)
    } else {
        core::ptr::null_mut()
    };
    let crazy_boma = if crazy_id != 0 && sv_battle_object::is_active(crazy_id) {
        sv_battle_object::module_accessor(crazy_id)
    } else {
        core::ptr::null_mut()
    };

    if !master_boma.is_null() {
        if master_was_player {
            WorkModule::on_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        } else {
            WorkModule::off_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        }
    }
    if !crazy_boma.is_null() {
        if crazy_was_player {
            WorkModule::on_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        } else {
            WorkModule::off_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
        }
    }

    crate::boss_log!(
        "[PB][HandEntrance] exit master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} reason={} phase={} observed_status=master:{} crazy:{} observed_motion=master:0x{:x} crazy:0x{:x} position=master:({:.2},{:.2},{:.2}) crazy:({:.2},{:.2},{:.2}) authority_restored=true",
        master_entry,
        crazy_entry,
        master_id,
        crazy_id,
        reason,
        phase,
        if master_boma.is_null() { -1 } else { StatusModule::status_kind(master_boma) },
        if crazy_boma.is_null() { -1 } else { StatusModule::status_kind(crazy_boma) },
        if master_boma.is_null() { 0 } else { MotionModule::motion_kind(master_boma) },
        if crazy_boma.is_null() { 0 } else { MotionModule::motion_kind(crazy_boma) },
        if master_boma.is_null() { 0.0 } else { PostureModule::pos_x(master_boma) },
        if master_boma.is_null() { 0.0 } else { PostureModule::pos_y(master_boma) },
        if master_boma.is_null() { 0.0 } else { PostureModule::pos_z(master_boma) },
        if crazy_boma.is_null() { 0.0 } else { PostureModule::pos_x(crazy_boma) },
        if crazy_boma.is_null() { 0.0 } else { PostureModule::pos_y(crazy_boma) },
        if crazy_boma.is_null() { 0.0 } else { PostureModule::pos_z(crazy_boma) },
    );

    HAND_ENTRANCE_AUTHORITY_ACTIVE = false;
    HAND_ENTRANCE_MASTER_ENTRY = usize::MAX;
    HAND_ENTRANCE_CRAZY_ENTRY = usize::MAX;
    HAND_ENTRANCE_MASTER_ID = 0;
    HAND_ENTRANCE_CRAZY_ID = 0;
    HAND_ENTRANCE_MASTER_PLAYER_WAS_SET = false;
    HAND_ENTRANCE_CRAZY_PLAYER_WAS_SET = false;
    HAND_ENTRANCE_MASTER_SEEN = false;
    HAND_ENTRANCE_CRAZY_SEEN = false;
    HAND_ENTRANCE_MASTER_STATUS_ACCEPTED = false;
    HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED = false;
    HAND_ENTRANCE_TICKS = 0;
    HAND_ENTRANCE_LAST_SIGNATURE = u64::MAX;
    HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED = false;
    HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK = -1;
    HAND_ENTRANCE_PHASE = HAND_ENTRANCE_PHASE_IDLE;
    HAND_ENTRANCE_DONE = true;
    HAND_ENTRANCE_ANCHOR_VALID = false;
    HAND_ENTRANCE_ANCHOR_X = 0.0;
    HAND_ENTRANCE_ANCHOR_Y = 0.0;
    HAND_ENTRANCE_ANCHOR_Z = 0.0;
}

#[inline(always)]
unsafe fn find_hand_entrance_pair() -> Option<(
    usize,
    u32,
    *mut BattleObjectModuleAccessor,
    usize,
    u32,
    *mut BattleObjectModuleAccessor,
)> {
    if sv_information::is_ready_go() || HAND_ENTRANCE_DONE {
        return None;
    }
    let fighter_manager = boss_helpers::fighter_manager();
    if !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager) {
        return None;
    }
    if boss_helpers::is_boss_preview_stage(smash::app::stage::get_stage_id()) {
        return None;
    }

    let entry_motion = smash::hash40("entry2");
    for master_entry in 0..8 {
        let master_id = BOSS_ID[master_entry];
        if master_id == 0 || !sv_battle_object::is_active(master_id) {
            continue;
        }
        let master_boma = sv_battle_object::module_accessor(master_id);
        if master_boma.is_null()
            || smash::app::utility::get_kind(&mut *master_boma) != *ITEM_KIND_MASTERHAND
        {
            continue;
        }
        let master_status = StatusModule::status_kind(master_boma);
        let master_motion = MotionModule::motion_kind(master_boma);
        let master_is_entering =
            master_status == *ITEM_STATUS_KIND_ENTRY || master_motion == entry_motion;
        let master_is_safe_wait = master_cpu_wait_family_status(master_status);

        for crazy_entry in 0..8 {
            let crazy_id = BOSS_ID_2[crazy_entry];
            if crazy_id == 0 || !sv_battle_object::is_active(crazy_id) {
                continue;
            }
            let crazy_boma = sv_battle_object::module_accessor(crazy_id);
            if crazy_boma.is_null()
                || smash::app::utility::get_kind(&mut *crazy_boma) != *ITEM_KIND_CRAZYHAND
                || TeamModule::team_no(master_boma) != TeamModule::team_no(crazy_boma)
            {
                continue;
            }
            let crazy_status = StatusModule::status_kind(crazy_boma);
            let crazy_motion = MotionModule::motion_kind(crazy_boma);
            let crazy_is_entering =
                crazy_status == *ITEM_STATUS_KIND_ENTRY || crazy_motion == entry_motion;
            let crazy_is_safe_wait = crazy_cpu_wait_family_status(crazy_status);

            // One hand can advance into its native wait state before the
            // partner item is acquired. Only accept that ordering while the
            // other hand is still entering, so a completed entrance cannot
            // be retriggered on subsequent pre-Ready-Go frames.
            if (master_is_entering || (master_is_safe_wait && crazy_is_entering))
                && (crazy_is_entering || (crazy_is_safe_wait && master_is_entering))
            {
                return Some((
                    master_entry,
                    master_id,
                    master_boma,
                    crazy_entry,
                    crazy_id,
                    crazy_boma,
                ));
            }
        }
    }
    None
}

/// Entry2 is authored for the native boss presentation and does not guarantee
/// that an item acquired on a normal stage starts inside that stage's camera.
/// Rebase the pair only when the current item positions are outside the live
/// dead range, preserving their relative spacing and using the current hidden
/// host as the stage-local anchor when it is valid.
#[inline(always)]
unsafe fn anchor_hand_entrance_pair(
    master_boma: *mut BattleObjectModuleAccessor,
    crazy_boma: *mut BattleObjectModuleAccessor,
    host_boma: *mut BattleObjectModuleAccessor,
    lua_state: u64,
    force_stage_center: bool,
) {
    if master_boma.is_null() || crazy_boma.is_null() {
        return;
    }

    let master_before = Vector3f {
        x: PostureModule::pos_x(master_boma),
        y: PostureModule::pos_y(master_boma),
        z: PostureModule::pos_z(master_boma),
    };
    let crazy_before = Vector3f {
        x: PostureModule::pos_x(crazy_boma),
        y: PostureModule::pos_y(crazy_boma),
        z: PostureModule::pos_z(crazy_boma),
    };
    let range = dead_range(lua_state);
    // dead_range is ordered as left, right, top, bottom.  Treating the first
    // two fields as symmetric extents can put the native Entry2 pair outside
    // the current stage camera on asymmetric stages.
    let range_width = range.y - range.x;
    let range_height = range.z - range.w;
    if !range.x.is_finite()
        || !range.y.is_finite()
        || !range.z.is_finite()
        || !range.w.is_finite()
        || range_width <= 1.0
        || range_height <= 1.0
        || !master_before.x.is_finite()
        || !master_before.y.is_finite()
        || !crazy_before.x.is_finite()
        || !crazy_before.y.is_finite()
    {
        let anchor_tick = HAND_ENTRANCE_TICKS;
        if HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK != anchor_tick
            && (anchor_tick <= 1 || anchor_tick % 15 == 0)
        {
            HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK = anchor_tick;
            crate::boss_log!(
                "[PB][HandEntrance] position_anchor skipped reason=invalid_range_or_position master_pos=({:.2},{:.2},{:.2}) crazy_pos=({:.2},{:.2},{:.2}) dead_range=({:.2},{:.2},{:.2},{:.2})",
                master_before.x,
                master_before.y,
                master_before.z,
                crazy_before.x,
                crazy_before.y,
                crazy_before.z,
                range.x,
                range.y,
                range.z,
                range.w
            );
        }
        return;
    }

    let center = Vector3f {
        x: (master_before.x + crazy_before.x) * 0.5,
        y: (master_before.y + crazy_before.y) * 0.5,
        z: (master_before.z + crazy_before.z) * 0.5,
    };
    let half_width = (master_before.x - crazy_before.x).abs() * 0.5;
    let half_height = (master_before.y - crazy_before.y).abs() * 0.5;
    let margin_x = range_width * 0.10;
    let margin_y = range_height * 0.10;
    let visible_left = range.x + margin_x;
    let visible_right = range.y - margin_x;
    let visible_bottom = range.w + margin_y;
    let visible_top = range.z - margin_y;
    let pair_min_x = visible_left + half_width;
    let pair_max_x = visible_right - half_width;
    let pair_min_y = visible_bottom + half_height;
    let pair_max_y = visible_top - half_height;
    let pair_fits = pair_min_x <= pair_max_x && pair_min_y <= pair_max_y;
    let pair_outside = !pair_fits
        || center.x < pair_min_x
        || center.x > pair_max_x
        || center.y < pair_min_y
        || center.y > pair_max_y;

    let host_anchor = if HAND_ENTRANCE_ANCHOR_VALID {
        Vector3f {
            x: HAND_ENTRANCE_ANCHOR_X,
            y: HAND_ENTRANCE_ANCHOR_Y,
            z: HAND_ENTRANCE_ANCHOR_Z,
        }
    } else if !host_boma.is_null() {
        Vector3f {
            x: PostureModule::pos_x(host_boma),
            y: PostureModule::pos_y(host_boma),
            z: PostureModule::pos_z(host_boma),
        }
    } else {
        Vector3f {
            x: (range.x + range.y) * 0.5,
            y: (range.z + range.w) * 0.5,
            z: center.z,
        }
    };
    let host_is_finite =
        host_anchor.x.is_finite() && host_anchor.y.is_finite() && host_anchor.z.is_finite();
    let stage_center = Vector3f {
        x: (range.x + range.y) * 0.5,
        y: (range.z + range.w) * 0.5,
        z: center.z,
    };
    let anchor = if host_is_finite {
        host_anchor
    } else {
        stage_center
    };
    // A newly acquired pair can inherit the native boss-stage separation. If
    // that separation is larger than the current stage's visible area, merely
    // translating both objects preserves an off-screen pair. Derive a safe
    // temporary separation from the live range instead; this changes only the
    // bounded Entry2 presentation transform and never changes collision data.
    let safe_half_width = ((visible_right - visible_left) * 0.32).max(1.0);
    let safe_half_height = ((visible_top - visible_bottom) * 0.18).max(1.0);
    let temporary_half_width = if pair_fits {
        half_width
    } else {
        safe_half_width
    };
    let temporary_half_height = if pair_fits {
        half_height
    } else {
        safe_half_height
    };
    let target_center = if force_stage_center {
        // Prefer the stable hidden-host anchor when it is available. The
        // native Entry2 motion is authored around the host/presentation
        // origin, while the dead-area midpoint can be outside the active
        // camera on normal and custom stages. Clamp either anchor to the
        // current live range so this remains stage-agnostic.
        let preferred_center = if HAND_ENTRANCE_ANCHOR_VALID || !host_boma.is_null() {
            anchor
        } else {
            stage_center
        };
        if pair_fits {
            Vector3f {
                x: preferred_center.x.clamp(pair_min_x, pair_max_x),
                y: preferred_center.y.clamp(pair_min_y, pair_max_y),
                z: preferred_center.z,
            }
        } else {
            Vector3f {
                x: preferred_center.x.clamp(
                    visible_left + safe_half_width,
                    visible_right - safe_half_width,
                ),
                y: preferred_center.y.clamp(
                    visible_bottom + safe_half_height,
                    visible_top - safe_half_height,
                ),
                z: preferred_center.z,
            }
        }
    } else if pair_fits {
        Vector3f {
            x: anchor.x.clamp(pair_min_x, pair_max_x),
            y: anchor.y.clamp(pair_min_y, pair_max_y),
            z: anchor.z,
        }
    } else {
        Vector3f {
            x: anchor.x.clamp(
                visible_left + safe_half_width,
                visible_right - safe_half_width,
            ),
            y: anchor.y.clamp(
                visible_bottom + safe_half_height,
                visible_top - safe_half_height,
            ),
            z: anchor.z,
        }
    };

    let mut master_after = master_before;
    let mut crazy_after = crazy_before;
    let mut rebased = false;
    let mut recomposed = false;
    if force_stage_center || pair_outside {
        if !pair_fits {
            let horizontal_pair = (master_before.x - crazy_before.x).abs()
                >= (master_before.y - crazy_before.y).abs();
            let master_side = if master_before.x >= crazy_before.x {
                1.0
            } else {
                -1.0
            };
            let master_vertical = if master_before.y >= crazy_before.y {
                1.0
            } else {
                -1.0
            };
            master_after.x = target_center.x
                + if horizontal_pair {
                    master_side * temporary_half_width
                } else {
                    0.0
                };
            crazy_after.x = target_center.x
                - if horizontal_pair {
                    master_side * temporary_half_width
                } else {
                    0.0
                };
            master_after.y = target_center.y
                + if horizontal_pair {
                    0.0
                } else {
                    master_vertical * temporary_half_height
                };
            crazy_after.y = target_center.y
                - if horizontal_pair {
                    0.0
                } else {
                    master_vertical * temporary_half_height
                };
            master_after.z = target_center.z;
            crazy_after.z = target_center.z;
            recomposed = true;
        } else {
            let delta = Vector3f {
                x: target_center.x - center.x,
                y: target_center.y - center.y,
                z: target_center.z - center.z,
            };
            master_after.x += delta.x;
            master_after.y += delta.y;
            master_after.z += delta.z;
            crazy_after.x += delta.x;
            crazy_after.y += delta.y;
            crazy_after.z += delta.z;
        }
        PostureModule::set_pos(master_boma, &master_after);
        PostureModule::set_pos(crazy_boma, &crazy_after);
        rebased = true;
    }

    let anchor_tick = HAND_ENTRANCE_TICKS;
    if HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK != anchor_tick
        && (anchor_tick <= 1 || anchor_tick % 15 == 0)
    {
        HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK = anchor_tick;
        crate::boss_log!(
            "[PB][HandEntrance] position_anchor rebased={} recomposed={} force_stage_center={} pair_outside={} pair_fits={} anchor=({:.2},{:.2},{:.2}) dead_range=({:.2},{:.2},{:.2},{:.2}) master_before=({:.2},{:.2},{:.2}) crazy_before=({:.2},{:.2},{:.2}) master_after=({:.2},{:.2},{:.2}) crazy_after=({:.2},{:.2},{:.2})",
            rebased,
            recomposed,
            force_stage_center,
            pair_outside,
            pair_fits,
            target_center.x,
            target_center.y,
            target_center.z,
            range.x,
            range.y,
            range.z,
            range.w,
            master_before.x,
            master_before.y,
            master_before.z,
            crazy_before.x,
            crazy_before.y,
            crazy_before.z,
            master_after.x,
            master_after.y,
            master_after.z,
            crazy_after.x,
            crazy_after.y,
            crazy_after.z
        );
    }
}

/// Return true while the coordinator owns an entrance claim. Spawn bookkeeping
/// must not reset the shared HandTeam state after both native Entry2 objects
/// have been accepted, even if a transient native activity read is stale.
#[inline(always)]
unsafe fn hand_entrance_authority_claimed() -> bool {
    HAND_ENTRANCE_AUTHORITY_ACTIVE && HAND_ENTRANCE_MASTER_ID != 0 && HAND_ENTRANCE_CRAZY_ID != 0
}

#[inline(always)]
unsafe fn hand_entrance_owns_entry(entry_id: usize, crazy: bool) -> bool {
    // The per-host reset path must not revoke a valid entrance merely because
    // native status processing temporarily makes a kind/activity read stale.
    // hand_entrance_step performs the authoritative live-object check and
    // releases the claim if either object is genuinely gone.
    if !hand_entrance_authority_claimed() {
        return false;
    }
    if crazy {
        HAND_ENTRANCE_CRAZY_ENTRY == entry_id
    } else {
        HAND_ENTRANCE_MASTER_ENTRY == entry_id
    }
}

#[inline(always)]
unsafe fn begin_hand_entrance_authority(
    master_entry: usize,
    master_id: u32,
    master_boma: *mut BattleObjectModuleAccessor,
    crazy_entry: usize,
    crazy_id: u32,
    crazy_boma: *mut BattleObjectModuleAccessor,
    lua_state: u64,
    host_boma: *mut BattleObjectModuleAccessor,
) {
    if HAND_ENTRANCE_AUTHORITY_ACTIVE {
        return;
    }

    HAND_ENTRANCE_AUTHORITY_ACTIVE = true;
    HAND_ENTRANCE_PHASE = HAND_ENTRANCE_PHASE_REQUESTED;
    HAND_ENTRANCE_MASTER_ENTRY = master_entry;
    HAND_ENTRANCE_CRAZY_ENTRY = crazy_entry;
    HAND_ENTRANCE_MASTER_ID = master_id;
    HAND_ENTRANCE_CRAZY_ID = crazy_id;
    HAND_ENTRANCE_MASTER_PLAYER_WAS_SET =
        WorkModule::is_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    HAND_ENTRANCE_CRAZY_PLAYER_WAS_SET =
        WorkModule::is_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    HAND_ENTRANCE_MASTER_SEEN = MotionModule::motion_kind(master_boma) == smash::hash40("entry2");
    HAND_ENTRANCE_CRAZY_SEEN = MotionModule::motion_kind(crazy_boma) == smash::hash40("entry2");
    HAND_ENTRANCE_MASTER_STATUS_ACCEPTED =
        StatusModule::status_kind(master_boma) == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT;
    HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED =
        StatusModule::status_kind(crazy_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT;
    HAND_ENTRANCE_TICKS = 0;
    HAND_ENTRANCE_LAST_SIGNATURE = u64::MAX;
    HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED = false;
    HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK = -1;

    if !host_boma.is_null() {
        let host_x = PostureModule::pos_x(host_boma);
        let host_y = PostureModule::pos_y(host_boma);
        let host_z = PostureModule::pos_z(host_boma);
        if host_x.is_finite() && host_y.is_finite() && host_z.is_finite() {
            HAND_ENTRANCE_ANCHOR_VALID = true;
            HAND_ENTRANCE_ANCHOR_X = host_x;
            HAND_ENTRANCE_ANCHOR_Y = host_y;
            HAND_ENTRANCE_ANCHOR_Z = host_z;
        } else {
            HAND_ENTRANCE_ANCHOR_VALID = false;
        }
    } else {
        HAND_ENTRANCE_ANCHOR_VALID = false;
    }

    let fighter_manager = boss_helpers::fighter_manager();
    let master_cpu = boss_helpers::is_operation_cpu_entry(fighter_manager, master_entry);
    let crazy_cpu = boss_helpers::is_operation_cpu_entry(fighter_manager, crazy_entry);
    let pre_master_status = StatusModule::status_kind(master_boma);
    let pre_crazy_status = StatusModule::status_kind(crazy_boma);
    let pre_master_motion = MotionModule::motion_kind(master_boma);
    let pre_crazy_motion = MotionModule::motion_kind(crazy_boma);
    let master_status = *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT;
    let crazy_status = *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT;

    // Entry2 is authored for the native boss presentation scene. Start the
    // pair at the stable stage-local host anchor (clamped to the live range)
    // so its first camera-facing frame is not inherited from an off-stage
    // boss spawn coordinate.
    anchor_hand_entrance_pair(master_boma, crazy_boma, host_boma, lua_state, true);

    WorkModule::on_flag(master_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    WorkModule::on_flag(crazy_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
    WorkModule::set_int(
        master_boma,
        master_entry as i32,
        ITEM_INSTANCE_WORK_INT_ENTRY_ID,
    );
    WorkModule::set_int(
        crazy_boma,
        crazy_entry as i32,
        ITEM_INSTANCE_WORK_INT_ENTRY_ID,
    );
    StatusModule::change_status_request_from_script(master_boma, master_status, true);
    StatusModule::change_status_request_from_script(crazy_boma, crazy_status, true);
    MotionModule::change_motion(
        master_boma,
        Hash40::new("entry2"),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    MotionModule::change_motion(
        crazy_boma,
        Hash40::new("entry2"),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    MotionModule::set_rate(master_boma, 1.5);
    MotionModule::set_rate(crazy_boma, 1.5);
    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(master_boma, 1.5);
    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(crazy_boma, 1.5);

    // Entry2 may apply its authored translation when the status/motion is
    // accepted. Recheck the pair after that native setup, not only before it,
    // so the generic stage camera sees the same stage-local anchor.
    anchor_hand_entrance_pair(master_boma, crazy_boma, host_boma, lua_state, true);

    crate::boss_log!(
        "[PB][HandEntrance] request master_entry={} crazy_entry={} master_cpu={} crazy_cpu={} master_object=0x{:x} crazy_object=0x{:x} pre_status=master:{} crazy:{} pre_motion=master:0x{:x} crazy:0x{:x} requested_status=master:{} crazy:{} requested_motion=entry2 position=master:({:.2},{:.2},{:.2}) crazy:({:.2},{:.2},{:.2}) authority_acquired=true",
        master_entry,
        crazy_entry,
        master_cpu,
        crazy_cpu,
        master_id,
        crazy_id,
        pre_master_status,
        pre_crazy_status,
        pre_master_motion,
        pre_crazy_motion,
        master_status,
        crazy_status,
        PostureModule::pos_x(master_boma),
        PostureModule::pos_y(master_boma),
        PostureModule::pos_z(master_boma),
        PostureModule::pos_x(crazy_boma),
        PostureModule::pos_y(crazy_boma),
        PostureModule::pos_z(crazy_boma),
    );
}

/// Synchronize the native Master/Crazy entry2 animation after both item
/// objects exist. This closes the ordering gap where the two per-host frame
/// callbacks each saw only one hand during their own ENTRY branch.
pub unsafe fn hand_entrance_step(lua_state: u64, host_boma: *mut BattleObjectModuleAccessor) {
    if crate::any_post_match_pre_result() {
        return;
    }

    if HAND_ENTRANCE_AUTHORITY_ACTIVE {
        // The dispatcher visits both hidden hosts. Advance the coordinator's
        // clock from one canonical side only, otherwise a two-hand pair ages
        // twice as fast and can hit timeout/recovery paths inconsistently.
        let caller_entry = boss_helpers::entry_id(host_boma);
        let coordinator_tick = caller_entry == HAND_ENTRANCE_MASTER_ENTRY;
        if coordinator_tick {
            HAND_ENTRANCE_TICKS += 1;
        }
        let master_boma = if HAND_ENTRANCE_MASTER_ID != 0
            && sv_battle_object::is_active(HAND_ENTRANCE_MASTER_ID)
        {
            sv_battle_object::module_accessor(HAND_ENTRANCE_MASTER_ID)
        } else {
            core::ptr::null_mut()
        };
        let crazy_boma =
            if HAND_ENTRANCE_CRAZY_ID != 0 && sv_battle_object::is_active(HAND_ENTRANCE_CRAZY_ID) {
                sv_battle_object::module_accessor(HAND_ENTRANCE_CRAZY_ID)
            } else {
                core::ptr::null_mut()
            };
        let fighter_manager = boss_helpers::fighter_manager();
        if master_boma.is_null()
            || crazy_boma.is_null()
            || (!fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager))
        {
            release_hand_entrance_authority("transition_or_object_invalid");
            return;
        }

        let master_status = StatusModule::status_kind(master_boma);
        let crazy_status = StatusModule::status_kind(crazy_boma);
        let master_motion = MotionModule::motion_kind(master_boma);
        let crazy_motion = MotionModule::motion_kind(crazy_boma);
        let entry_motion = smash::hash40("entry2");
        HAND_ENTRANCE_MASTER_SEEN |= master_motion == entry_motion;
        HAND_ENTRANCE_CRAZY_SEEN |= crazy_motion == entry_motion;
        HAND_ENTRANCE_MASTER_STATUS_ACCEPTED |=
            master_status == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT;
        HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED |=
            crazy_status == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT;
        let entrance_was_active = HAND_ENTRANCE_PHASE == HAND_ENTRANCE_PHASE_ACTIVE;
        if HAND_ENTRANCE_MASTER_SEEN
            && HAND_ENTRANCE_CRAZY_SEEN
            && HAND_ENTRANCE_MASTER_STATUS_ACCEPTED
            && HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED
        {
            HAND_ENTRANCE_PHASE = HAND_ENTRANCE_PHASE_ACTIVE;
            if !entrance_was_active {
                let snapshot = hand_entrance_log_snapshot();
                crate::boss_log!(
                    "[PB][HandEntrance] phase=active master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} observed_status=master:{} crazy:{} observed_motion=master:0x{:x} crazy:0x{:x}",
                    snapshot.master_entry,
                    snapshot.crazy_entry,
                    snapshot.master_id,
                    snapshot.crazy_id,
                    master_status,
                    crazy_status,
                    master_motion,
                    crazy_motion
                );
            }
        }

        // Native Entry2 can reapply its authored translation for a few frames
        // after the request. Re-anchor only during that bounded entrance
        // window; after it closes, native motion owns the positions normally.
        if coordinator_tick && HAND_ENTRANCE_TICKS <= HAND_ENTRANCE_ANCHOR_FRAMES {
            // Entry2 is authored for the native boss presentation scene. On a
            // normal stage its root translation can leave the camera before
            // Ready-Go, so keep the pair inside the live dead-range margins
            // around the stable stage-local anchor for the bounded entrance
            // window. This changes only the temporary item presentation
            // transform; no camera or collision state is modified.
            anchor_hand_entrance_pair(master_boma, crazy_boma, host_boma, lua_state, false);
        }

        let signature = (master_status as u32 as u64)
            ^ ((crazy_status as u32 as u64) << 17)
            ^ master_motion.rotate_left(7)
            ^ crazy_motion.rotate_left(23);
        if HAND_ENTRANCE_LAST_SIGNATURE != signature {
            HAND_ENTRANCE_LAST_SIGNATURE = signature;
            let snapshot = hand_entrance_log_snapshot();
            crate::boss_log!(
                "[PB][HandEntrance] active master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} observed_status=master:{} crazy:{} observed_motion=master:0x{:x} crazy:0x{:x} position=master:({:.2},{:.2},{:.2}) crazy:({:.2},{:.2},{:.2}) seen=master:{} crazy:{}",
                snapshot.master_entry,
                snapshot.crazy_entry,
                snapshot.master_id,
                snapshot.crazy_id,
                master_status,
                crazy_status,
                master_motion,
                crazy_motion,
                PostureModule::pos_x(master_boma),
                PostureModule::pos_y(master_boma),
                PostureModule::pos_z(master_boma),
                PostureModule::pos_x(crazy_boma),
                PostureModule::pos_y(crazy_boma),
                PostureModule::pos_z(crazy_boma),
                snapshot.master_seen,
                snapshot.crazy_seen,
            );
        }

        let both_native_accepted = HAND_ENTRANCE_MASTER_STATUS_ACCEPTED
            && HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED
            && HAND_ENTRANCE_MASTER_SEEN
            && HAND_ENTRANCE_CRAZY_SEEN;
        if sv_information::is_ready_go() {
            release_hand_entrance_authority(if both_native_accepted {
                "native_entrance_complete"
            } else {
                "ready_go_before_native_accept"
            });
            return;
        }

        if HAND_ENTRANCE_MASTER_SEEN
            && HAND_ENTRANCE_CRAZY_SEEN
            && ((master_motion != entry_motion && crazy_motion != entry_motion)
                || (MotionModule::is_end(master_boma) && MotionModule::is_end(crazy_boma)))
        {
            release_hand_entrance_authority("native_entrance_complete");
        } else if HAND_ENTRANCE_TICKS >= HAND_ENTRANCE_TIMEOUT {
            release_hand_entrance_authority("timeout");
        }
        return;
    }

    if let Some((master_entry, master_id, master_boma, crazy_entry, crazy_id, crazy_boma)) =
        find_hand_entrance_pair()
    {
        begin_hand_entrance_authority(
            master_entry,
            master_id,
            master_boma,
            crazy_entry,
            crazy_id,
            crazy_boma,
            lua_state,
            host_boma,
        );
    }
}

pub unsafe fn hand_team_authority_active_for_debug() -> bool {
    HAND_TEAM_AUTHORITY_ACTIVE || HAND_ENTRANCE_AUTHORITY_ACTIVE
}

pub unsafe fn abort_hand_team_for_transition(reason: &str) {
    if FINDER || FINDER_BASE_RANGE_CAPTURED {
        clear_finder_runtime_with_reason(reason);
    }
    release_hand_entrance_authority(reason);
    release_hand_team_authority(reason);
    reset_mastercrazy_shared_runtime();
}

/// Drop plugin-owned hand-action latches after native result mode has begun.
///
/// The normal post-match path calls `abort_hand_team_for_transition` while the
/// battle objects are still available, which restores temporary item flags and
/// Finder state. Result mode can also be observed without that intermediate
/// callback, however. At that point native teardown owns the item objects, so
/// this fallback must not dereference them or write their WorkModule state.
/// Scene-exit reset retains the Finder snapshot until the native transition is
/// over and performs the ordinary idempotent restoration at the safe boundary.
pub unsafe fn quarantine_hand_authority_for_result(reason: &str) {
    let had_hand_team = HAND_TEAM_AUTHORITY_ACTIVE;
    let had_entrance = HAND_ENTRANCE_AUTHORITY_ACTIVE;
    if crate::debug::enabled() && (had_hand_team || had_entrance) {
        crate::boss_log!(
            "[PB][ResultTransition] hand_authority_quarantine reason={} hand_team_active={} entrance_active={} native_item_access=false finder_state_preserved={}",
            reason,
            had_hand_team,
            had_entrance,
            FINDER || FINDER_BASE_RANGE_CAPTURED
        );
    }

    // Only clear plugin bookkeeping. The original player flags are not
    // restored here because doing so would require touching objects owned by
    // native result teardown. The post-match path normally restores them
    // before this fallback is reached.
    HAND_TEAM_AUTHORITY_ACTIVE = false;
    HAND_TEAM_ACTION = 0;
    HAND_TEAM_INITIATOR_ENTRY = usize::MAX;
    HAND_TEAM_MASTER_ENTRY = usize::MAX;
    HAND_TEAM_CRAZY_ENTRY = usize::MAX;
    HAND_TEAM_MASTER_ID = 0;
    HAND_TEAM_CRAZY_ID = 0;
    HAND_TEAM_MASTER_PLAYER_WAS_SET = false;
    HAND_TEAM_CRAZY_PLAYER_WAS_SET = false;
    HAND_TEAM_REQUESTED_MASTER_STATUS = -1;
    HAND_TEAM_REQUESTED_CRAZY_STATUS = -1;
    HAND_TEAM_LAST_STATUS_SIGNATURE = u64::MAX;

    HAND_ENTRANCE_AUTHORITY_ACTIVE = false;
    HAND_ENTRANCE_MASTER_ENTRY = usize::MAX;
    HAND_ENTRANCE_CRAZY_ENTRY = usize::MAX;
    HAND_ENTRANCE_MASTER_ID = 0;
    HAND_ENTRANCE_CRAZY_ID = 0;
    HAND_ENTRANCE_MASTER_PLAYER_WAS_SET = false;
    HAND_ENTRANCE_CRAZY_PLAYER_WAS_SET = false;
    HAND_ENTRANCE_MASTER_SEEN = false;
    HAND_ENTRANCE_CRAZY_SEEN = false;
    HAND_ENTRANCE_MASTER_STATUS_ACCEPTED = false;
    HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED = false;
    HAND_ENTRANCE_TICKS = 0;
    HAND_ENTRANCE_LAST_SIGNATURE = u64::MAX;
    HAND_ENTRANCE_PHASE = HAND_ENTRANCE_PHASE_IDLE;
    HAND_ENTRANCE_DONE = true;
}

#[inline(always)]
unsafe fn sync_hand_team_authority_from_flags(
    crazy_boma: *mut BattleObjectModuleAccessor,
    initiator_entry: usize,
) {
    let action = shared_hand_action();
    if action == 0 || action == HAND_TEAM_ACTION_FINDER || HAND_TEAM_AUTHORITY_ACTIVE {
        return;
    }
    let (master_entry, master_boma) = finder_master_for_crazy(crazy_boma);
    let mut crazy_entry = usize::MAX;
    for entry in 0..8 {
        if BOSS_ID_2[entry] != 0
            && sv_battle_object::is_active(BOSS_ID_2[entry])
            && sv_battle_object::module_accessor(BOSS_ID_2[entry]) == crazy_boma
        {
            crazy_entry = entry;
            break;
        }
    }
    if master_entry < 8 && crazy_entry < 8 {
        let _ = begin_hand_team_authority(
            action,
            initiator_entry,
            master_entry,
            master_boma,
            crazy_entry,
            crazy_boma,
        );
    }
}

#[inline(always)]
unsafe fn hand_team_native_action_still_active(
    action: i32,
    master_boma: *mut BattleObjectModuleAccessor,
    crazy_boma: *mut BattleObjectModuleAccessor,
) -> bool {
    if master_boma.is_null() || crazy_boma.is_null() {
        return false;
    }

    let master_status = StatusModule::status_kind(master_boma);
    let crazy_status = StatusModule::status_kind(crazy_boma);
    let master_motion = MotionModule::motion_kind(master_boma);
    let crazy_motion = MotionModule::motion_kind(crazy_boma);

    let motion_is = |motion: u64, name: &str| motion == smash::hash40(name);
    match action {
        HAND_TEAM_ACTION_BARK => {
            master_status == *ITEM_MASTERHAND_STATUS_KIND_BARK
                || motion_is(master_motion, "bark")
                || motion_is(crazy_motion, "bark")
        }
        HAND_TEAM_ACTION_PUNCH => {
            master_status == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                || motion_is(master_motion, "goopaa")
                || motion_is(crazy_motion, "taggoopaa")
        }
        HAND_TEAM_ACTION_SHOCK => {
            master_status == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START
                || master_status == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK
                || master_status == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_END
                || motion_is(master_motion, "electroshock_start")
                || motion_is(master_motion, "electroshock")
                || motion_is(master_motion, "electroshock_end")
                || motion_is(crazy_motion, "electroshock_start")
                || motion_is(crazy_motion, "electroshock")
                || motion_is(crazy_motion, "electroshock_end")
        }
        HAND_TEAM_ACTION_LASER => {
            master_status == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                || crazy_status == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START
                || motion_is(master_motion, "wfinger_beam_start")
                || motion_is(master_motion, "finger_beam")
                || motion_is(crazy_motion, "wfinger_beam_start")
                || motion_is(crazy_motion, "finger_beam")
        }
        HAND_TEAM_ACTION_SCRATCH => {
            master_status == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START
                || master_status == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                || master_status == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW
                || crazy_status == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START
                || crazy_status == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                || crazy_status == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW
        }
        HAND_TEAM_ACTION_FINDER => {
            master_status == *ITEM_MASTERHAND_STATUS_KIND_FINDER
                || crazy_status == *ITEM_CRAZYHAND_STATUS_KIND_FINDER
        }
        _ => false,
    }
}

#[inline(always)]
unsafe fn maybe_finish_hand_team_authority(reason: &str) {
    if !HAND_TEAM_AUTHORITY_ACTIVE || HAND_TEAM_ACTION == HAND_TEAM_ACTION_FINDER {
        return;
    }
    if shared_hand_action() == 0 {
        let master_boma =
            if HAND_TEAM_MASTER_ID != 0 && sv_battle_object::is_active(HAND_TEAM_MASTER_ID) {
                sv_battle_object::module_accessor(HAND_TEAM_MASTER_ID)
            } else {
                core::ptr::null_mut()
            };
        let crazy_boma =
            if HAND_TEAM_CRAZY_ID != 0 && sv_battle_object::is_active(HAND_TEAM_CRAZY_ID) {
                sv_battle_object::module_accessor(HAND_TEAM_CRAZY_ID)
            } else {
                core::ptr::null_mut()
            };
        if !hand_team_native_action_still_active(HAND_TEAM_ACTION, master_boma, crazy_boma) {
            release_hand_team_authority(reason);
        }
    } else if !hand_team_native_action_still_active(
        HAND_TEAM_ACTION,
        if HAND_TEAM_MASTER_ID != 0 && sv_battle_object::is_active(HAND_TEAM_MASTER_ID) {
            sv_battle_object::module_accessor(HAND_TEAM_MASTER_ID)
        } else {
            core::ptr::null_mut()
        },
        if HAND_TEAM_CRAZY_ID != 0 && sv_battle_object::is_active(HAND_TEAM_CRAZY_ID) {
            sv_battle_object::module_accessor(HAND_TEAM_CRAZY_ID)
        } else {
            core::ptr::null_mut()
        },
    ) {
        // A shared flag can be cleared by a failed native transition. Do not
        // leave the temporary ownership barrier installed in that case.
        release_hand_team_authority(reason);
    }
}

#[inline(always)]
unsafe fn reset_mastercrazy_shared_runtime() {
    // This helper resets ordinary shared attack state. It is deliberately not
    // allowed to touch any shared state while HandEntrance owns both objects:
    // per-host spawn bookkeeping can call it while native Entry2 is active.
    // Only the coordinator or an explicit scene/match reset may release the
    // entrance claim. The explicit early return also prevents a runtime reset
    // from clearing attack/entrance latches and causing request/reset thrash.
    if HAND_ENTRANCE_AUTHORITY_ACTIVE && !HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED {
        HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED = true;
        let snapshot = hand_entrance_log_snapshot();
        crate::boss_log!(
            "[PB][HandEntrance] shared_runtime_reset_suppressed master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} phase={}",
            snapshot.master_entry,
            snapshot.crazy_entry,
            snapshot.master_id,
            snapshot.crazy_id,
            snapshot.phase
        );
    }
    if HAND_ENTRANCE_AUTHORITY_ACTIVE {
        return;
    }
    release_hand_team_authority("runtime_reset");
    BARK = false;
    PUNCH = false;
    SHOCK = false;
    LASER = false;
    SCRATCH_BLOW = false;
    FINDER = false;
    MASTER_FINDER_ACTIVE = false;
    CRAZY_FINDER_ACTIVE = false;
    FINDER_SYNC_FRAMES = 0;
    FINDER_CAMERA_APPLIED = false;
    FINDER_DEAD_RANGE_APPLIED = false;
    FINDER_BASE_RANGE_CAPTURED = false;
    FINDER_NATIVE_ACTIVE_SEEN = false;
    FINDER_LAST_STATUS_SIGNATURE = u64::MAX;
    FINDER_LAST_REQUEST_SIGNATURE = u64::MAX;
    FINDER_TRIGGER_LATCH = [false; 8];
    FINDER_COOLDOWN_FRAMES = 0;
    FINDER_BASE_RANGE = Vector4f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    CONTROL_SPEED_MUL = 2.0;
    CONTROL_SPEED_MUL_2 = 0.05;

    MASTER_X_POS = 0.0;
    MASTER_Y_POS = 0.0;
    MASTER_Z_POS = 0.0;
    MASTER_USABLE = false;
    MASTER_FACING_LEFT = true;
    CONTROLLER_X_MASTER = 0.0;
    CONTROLLER_Y_MASTER = 0.0;

    CRAZY_X_POS = 0.0;
    CRAZY_Y_POS = 0.0;
    CRAZY_Z_POS = 0.0;
    CRAZY_USABLE = false;
    CRAZY_FACING_RIGHT = true;
    CONTROLLER_X_CRAZY = 0.0;
    CONTROLLER_Y_CRAZY = 0.0;
}

#[inline(always)]
unsafe fn reset_master_runtime_for_spawn() {
    if FINDER || FINDER_BASE_RANGE_CAPTURED {
        clear_finder_runtime_with_reason("master_spawn_reset");
    }
    JUMP_START = false;
    STOP = false;
    MULTIPLE_BULLETS = 0;
    MASTER_LAST_IRON_BALL_ID = 0;
    MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
    MASTER_IRON_BALL_SMOOTH_CANCEL = false;
    MASTER_KENZAN_SPAWNED = false;
    reset_master_cpu_idle_recovery(ENTRY_ID);
    if hand_entrance_authority_claimed() {
        if !HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED {
            HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED = true;
            let snapshot = hand_entrance_log_snapshot();
            crate::boss_log!(
                "[PB][HandEntrance] spawn_reset_suppressed master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} phase={} reason=authority_claimed",
                snapshot.master_entry,
                snapshot.crazy_entry,
                snapshot.master_id,
                snapshot.crazy_id,
                snapshot.phase
            );
        }
    } else {
        reset_mastercrazy_shared_runtime();
    }
}

#[inline(always)]
unsafe fn reset_crazy_runtime_for_spawn() {
    if FINDER || FINDER_BASE_RANGE_CAPTURED {
        clear_finder_runtime_with_reason("crazy_spawn_reset");
    }
    JUMP_START_2 = false;
    STOP_2 = false;
    CRAZY_KUMO_ACTIVE = false;
    CRAZY_KUMO_START_Y = 0.0;
    CRAZY_KUMO_ENDING = false;
    reset_crazy_cpu_idle_recovery(ENTRY_ID_2);
    reset_crazy_fire_chariot_latches(ENTRY_ID_2);
    if hand_entrance_authority_claimed() {
        if !HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED {
            HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED = true;
            let snapshot = hand_entrance_log_snapshot();
            crate::boss_log!(
                "[PB][HandEntrance] spawn_reset_suppressed master_entry={} crazy_entry={} master_object=0x{:x} crazy_object=0x{:x} phase={} reason=authority_claimed",
                snapshot.master_entry,
                snapshot.crazy_entry,
                snapshot.master_id,
                snapshot.crazy_id,
                snapshot.phase
            );
        }
    } else {
        reset_mastercrazy_shared_runtime();
    }
}

/// Clear only plugin-owned Master/Crazy bookkeeping after native item
/// teardown has begun. The caller must have already released HandTeam,
/// HandEntrance, and Finder authority while both item objects were valid.
/// This function intentionally performs no battle-object lookup and no native
/// module access, so it is safe in the post-match/pre-result gap.
pub unsafe fn invalidate_transition_tracking(entry_id: usize) {
    let entry = boss_runtime::sanitize_entry_id(entry_id);
    let had_tracking = BOSS_ID[entry] != 0
        || BOSS_ID_2[entry] != 0
        || MASTER_EXISTS
        || CRAZY_EXISTS
        || HAND_TEAM_AUTHORITY_ACTIVE
        || HAND_ENTRANCE_AUTHORITY_ACTIVE
        || FINDER
        || FINDER_BASE_RANGE_CAPTURED;

    CONTROLLABLE = true;
    ENTRY_ID = entry;
    FIGHTER_MANAGER = 0;
    BOSS_ID[entry] = 0;
    MULTIPLE_BULLETS = 0;
    DEAD = false;
    JUMP_START = false;
    RESULT_SPAWNED = false;
    STOP = false;
    MASTER_EXISTS = false;
    EXISTS_PUBLIC = false;
    Y_POS = 0.0;
    MASTER_TEAM = 99;
    MASTER_LAST_IRON_BALL_ID = 0;
    MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
    MASTER_IRON_BALL_SMOOTH_CANCEL = false;
    MASTER_KENZAN_SPAWNED = false;
    MASTER_CPU_IDLE_STALL_FRAMES = [0; 8];
    MASTER_CPU_LAST_X = [0.0; 8];
    MASTER_CPU_LAST_Y = [0.0; 8];
    MASTER_CPU_RECOVERY_LOG_COOLDOWN = [0; 8];

    CONTROLLABLE_2 = true;
    ENTRY_ID_2 = entry;
    FIGHTER_MANAGER_2 = 0;
    BOSS_ID_2[entry] = 0;
    DEAD_2 = false;
    JUMP_START_2 = false;
    RESULT_SPAWNED_2 = false;
    STOP_2 = false;
    CRAZY_EXISTS = false;
    EXISTS_PUBLIC_2 = false;
    Y_POS_2 = 0.0;
    CRAZY_TEAM = 98;
    CRAZY_KUMO_ACTIVE = false;
    CRAZY_KUMO_START_Y = 0.0;
    CRAZY_KUMO_ENDING = false;
    CRAZY_CPU_IDLE_STALL_FRAMES = [0; 8];
    CRAZY_CPU_LAST_X = [0.0; 8];
    CRAZY_CPU_LAST_Y = [0.0; 8];
    CRAZY_CPU_RECOVERY_LOG_COOLDOWN = [0; 8];
    CRAZY_FIRE_CHARIOT_PINKY_LATCH = [false; 8];
    CRAZY_FIRE_CHARIOT_THUMB_LATCH = [false; 8];

    // Shared attack/Finder state is cleared by assignment only. Native
    // camera/dead-area and WorkModule restoration happened in the preceding
    // transition authority abort while the objects were still live.
    BARK = false;
    PUNCH = false;
    SHOCK = false;
    LASER = false;
    SCRATCH_BLOW = false;
    FINDER = false;
    MASTER_FINDER_ACTIVE = false;
    CRAZY_FINDER_ACTIVE = false;
    FINDER_SYNC_FRAMES = 0;
    FINDER_CAMERA_APPLIED = false;
    FINDER_DEAD_RANGE_APPLIED = false;
    FINDER_BASE_RANGE_CAPTURED = false;
    FINDER_NATIVE_ACTIVE_SEEN = false;
    FINDER_COOLDOWN_FRAMES = 0;
    FINDER_MASTER_ENTRY = 8;
    FINDER_CRAZY_ENTRY = 8;
    FINDER_BASE_RANGE = Vector4f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    FINDER_LAST_STATUS_SIGNATURE = u64::MAX;
    FINDER_LAST_REQUEST_SIGNATURE = u64::MAX;
    FINDER_TRIGGER_LATCH = [false; 8];
    CONTROL_SPEED_MUL = 2.0;
    CONTROL_SPEED_MUL_2 = 0.05;
    MASTER_X_POS = 0.0;
    MASTER_Y_POS = 0.0;
    MASTER_Z_POS = 0.0;
    MASTER_USABLE = false;
    MASTER_FACING_LEFT = true;
    CONTROLLER_X_MASTER = 0.0;
    CONTROLLER_Y_MASTER = 0.0;
    CRAZY_X_POS = 0.0;
    CRAZY_Y_POS = 0.0;
    CRAZY_Z_POS = 0.0;
    CRAZY_USABLE = false;
    CRAZY_FACING_RIGHT = true;
    CONTROLLER_X_CRAZY = 0.0;
    CONTROLLER_Y_CRAZY = 0.0;

    // Both authority records are invalidated without touching their objects.
    // The done latch prevents a stale entrance discovery from re-requesting
    // Entry2 during the remainder of the native transition.
    HAND_TEAM_AUTHORITY_ACTIVE = false;
    HAND_TEAM_ACTION = 0;
    HAND_TEAM_INITIATOR_ENTRY = usize::MAX;
    HAND_TEAM_MASTER_ENTRY = usize::MAX;
    HAND_TEAM_CRAZY_ENTRY = usize::MAX;
    HAND_TEAM_MASTER_ID = 0;
    HAND_TEAM_CRAZY_ID = 0;
    HAND_TEAM_MASTER_PLAYER_WAS_SET = false;
    HAND_TEAM_CRAZY_PLAYER_WAS_SET = false;
    HAND_TEAM_REQUESTED_MASTER_STATUS = -1;
    HAND_TEAM_REQUESTED_CRAZY_STATUS = -1;
    HAND_TEAM_LAST_STATUS_SIGNATURE = u64::MAX;
    HAND_ENTRANCE_AUTHORITY_ACTIVE = false;
    HAND_ENTRANCE_MASTER_ENTRY = usize::MAX;
    HAND_ENTRANCE_CRAZY_ENTRY = usize::MAX;
    HAND_ENTRANCE_MASTER_ID = 0;
    HAND_ENTRANCE_CRAZY_ID = 0;
    HAND_ENTRANCE_MASTER_PLAYER_WAS_SET = false;
    HAND_ENTRANCE_CRAZY_PLAYER_WAS_SET = false;
    HAND_ENTRANCE_MASTER_SEEN = false;
    HAND_ENTRANCE_CRAZY_SEEN = false;
    HAND_ENTRANCE_MASTER_STATUS_ACCEPTED = false;
    HAND_ENTRANCE_CRAZY_STATUS_ACCEPTED = false;
    HAND_ENTRANCE_TICKS = 0;
    HAND_ENTRANCE_LAST_SIGNATURE = u64::MAX;
    HAND_ENTRANCE_RESET_SUPPRESSION_LOGGED = false;
    HAND_ENTRANCE_LAST_ANCHOR_LOG_TICK = -1;
    HAND_ENTRANCE_PHASE = HAND_ENTRANCE_PHASE_IDLE;
    HAND_ENTRANCE_DONE = true;
    HAND_ENTRANCE_ANCHOR_VALID = false;
    HAND_ENTRANCE_ANCHOR_X = 0.0;
    HAND_ENTRANCE_ANCHOR_Y = 0.0;
    HAND_ENTRANCE_ANCHOR_Z = 0.0;

    if had_tracking {
        crate::boss_log!(
            "[PB][ResultTransition] mastercrazy_tracking_invalidated entry={} native_item_access=false hand_authority_cleared=true",
            entry
        );
    }
}

pub unsafe fn reset_match_state(entry_id: usize) {
    let entry = boss_runtime::sanitize_entry_id(entry_id);

    if crate::debug::enabled()
        && (BOSS_ID[entry] != 0
            || BOSS_ID_2[entry] != 0
            || DEAD
            || DEAD_2
            || RESULT_SPAWNED
            || RESULT_SPAWNED_2
            || STOP
            || STOP_2
            || MASTER_EXISTS
            || EXISTS_PUBLIC)
    {
        crate::boss_log!(
            "[PB][MasterCrazy][Reset] entry={} master_id=0x{:x} crazy_id=0x{:x} master_exists={} exists_public={} master_dead={} crazy_dead={} master_result={} crazy_result={} master_stop={} crazy_stop={}",
            entry,
            BOSS_ID[entry],
            BOSS_ID_2[entry],
            core::ptr::addr_of!(MASTER_EXISTS).read(),
            core::ptr::addr_of!(EXISTS_PUBLIC).read(),
            core::ptr::addr_of!(DEAD).read(),
            core::ptr::addr_of!(DEAD_2).read(),
            core::ptr::addr_of!(RESULT_SPAWNED).read(),
            core::ptr::addr_of!(RESULT_SPAWNED_2).read(),
            core::ptr::addr_of!(STOP).read(),
            core::ptr::addr_of!(STOP_2).read()
        );
    }

    if FINDER || FINDER_BASE_RANGE_CAPTURED {
        clear_finder_runtime_with_reason("match_state_reset");
    }
    release_hand_entrance_authority("match_state_reset");
    reset_mastercrazy_shared_runtime();
    HAND_ENTRANCE_DONE = false;

    CONTROLLABLE = true;
    ENTRY_ID = entry;
    BOSS_ID[entry] = 0;
    MULTIPLE_BULLETS = 0;
    DEAD = false;
    JUMP_START = false;
    RESULT_SPAWNED = false;
    STOP = false;
    MASTER_EXISTS = false;
    EXISTS_PUBLIC = false;
    Y_POS = 0.0;
    MASTER_TEAM = 99;
    MASTER_LAST_IRON_BALL_ID = 0;
    MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
    MASTER_IRON_BALL_SMOOTH_CANCEL = false;
    MASTER_KENZAN_SPAWNED = false;
    reset_master_cpu_idle_recovery(entry);

    CONTROLLABLE_2 = true;
    ENTRY_ID_2 = entry;
    BOSS_ID_2[entry] = 0;
    DEAD_2 = false;
    JUMP_START_2 = false;
    RESULT_SPAWNED_2 = false;
    STOP_2 = false;
    CRAZY_EXISTS = false;
    EXISTS_PUBLIC_2 = false;
    Y_POS_2 = 0.0;
    CRAZY_TEAM = 98;
    CRAZY_KUMO_ACTIVE = false;
    CRAZY_KUMO_START_Y = 0.0;
    CRAZY_KUMO_ENDING = false;
    reset_crazy_cpu_idle_recovery(entry);
    reset_crazy_fire_chariot_latches(entry);
}

#[inline(always)]
unsafe fn acquire_master_hand_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry_id: usize,
) -> *mut BattleObjectModuleAccessor {
    let boss_boma =
        boss_helpers::acquire_boss_item(module_accessor, &raw mut BOSS_ID, *ITEM_KIND_MASTERHAND);
    configure_boss_owner_mode(boss_boma, entry_id);
    if boss_boma.is_null() {
        return boss_boma;
    }
    crate::boss_log!(
        "[PB][MasterHand][Acquire] entry={} tracked_id=0x{:x} boss_kind={} boss_status={} host_scale={:.4}",
        entry_id,
        BOSS_ID[entry_id.min(7)],
        smash::app::utility::get_kind(&mut *boss_boma),
        StatusModule::status_kind(boss_boma),
        ModelModule::scale(module_accessor)
    );
    boss_boma
}

#[inline(always)]
unsafe fn cancel_master_iron_ball(
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
    reason: &str,
) {
    let entry_id = ENTRY_ID;
    let last_iron_ball_id = MASTER_LAST_IRON_BALL_ID;
    println!(
        "[PB][MasterHand][IronBall] cancel reason={} entry={} ball=0x{:x}",
        reason, entry_id, last_iron_ball_id,
    );
    if !module_accessor.is_null() && ItemModule::is_have_item(module_accessor, 0) {
        let held_item_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
        if held_item_id != 0 && sv_battle_object::is_active(held_item_id) {
            let held_item_boma = sv_battle_object::module_accessor(held_item_id);
            if !held_item_boma.is_null()
                && smash::app::utility::get_kind(&mut *held_item_boma)
                    == *ITEM_KIND_MASTERHANDIRONBALL
            {
                ItemModule::remove_item(module_accessor, 0);
            }
        }
    }
    if !boss_boma.is_null() && ItemModule::is_have_item(boss_boma, 0) {
        let held_item_id = ItemModule::get_have_item_id(boss_boma, 0) as u32;
        if held_item_id != 0 && sv_battle_object::is_active(held_item_id) {
            let held_item_boma = sv_battle_object::module_accessor(held_item_id);
            if !held_item_boma.is_null()
                && smash::app::utility::get_kind(&mut *held_item_boma)
                    == *ITEM_KIND_MASTERHANDIRONBALL
            {
                ItemModule::remove_item(boss_boma, 0);
            }
        }
    }
    if MASTER_LAST_IRON_BALL_ID != 0 && sv_battle_object::is_active(MASTER_LAST_IRON_BALL_ID) {
        let iron_ball_boma = sv_battle_object::module_accessor(MASTER_LAST_IRON_BALL_ID);
        if !iron_ball_boma.is_null() {
            remove(iron_ball_boma);
        }
    }
    MASTER_LAST_IRON_BALL_ID = 0;
    MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
    MASTER_IRON_BALL_SMOOTH_CANCEL = true;
    if !boss_boma.is_null() {
        WorkModule::off_flag(
            boss_boma,
            *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_CREATE,
        );
        WorkModule::off_flag(
            boss_boma,
            *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_THROW,
        );
        StatusModule::change_status_request_from_script(
            boss_boma,
            *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_END,
            true,
        );
    }
    CONTROLLABLE = false;
    CONTROLLER_X_MASTER = 0.0;
    CONTROLLER_Y_MASTER = 0.0;
}

#[inline(always)]
unsafe fn acquire_crazy_hand_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry_id: usize,
) -> *mut BattleObjectModuleAccessor {
    let boss_boma =
        boss_helpers::acquire_boss_item(module_accessor, &raw mut BOSS_ID_2, *ITEM_KIND_CRAZYHAND);
    configure_boss_owner_mode(boss_boma, entry_id);
    if boss_boma.is_null() {
        return boss_boma;
    }
    crate::boss_log!(
        "[PB][CrazyHand][Acquire] entry={} tracked_id=0x{:x} boss_kind={} boss_status={} host_scale={:.4}",
        entry_id,
        BOSS_ID_2[entry_id.min(7)],
        smash::app::utility::get_kind(&mut *boss_boma),
        StatusModule::status_kind(boss_boma),
        ModelModule::scale(module_accessor)
    );
    boss_boma
}

#[inline(always)]
unsafe fn initialize_master_hand_boss(
    boss_boma: *mut BattleObjectModuleAccessor,
    get_boss_intensity: f32,
) {
    WorkModule::set_int(
        boss_boma,
        *ITEM_TRAIT_FLAG_BOSS,
        *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG,
    );
    WorkModule::set_float(
        boss_boma,
        get_boss_intensity,
        *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
    );
    WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
    WorkModule::set_int(
        boss_boma,
        *ITEM_BOSS_MODE_ADVENTURE_HARD,
        *ITEM_INSTANCE_WORK_INT_BOSS_MODE,
    );
    WorkModule::set_int(
        boss_boma,
        *ITEM_VARIATION_MASTERHAND_CRAZYHAND_STANDARD,
        *ITEM_INSTANCE_WORK_INT_VARIATION,
    );
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
}

#[inline(always)]
unsafe fn initialize_crazy_hand_boss(
    boss_boma: *mut BattleObjectModuleAccessor,
    get_boss_intensity: f32,
) {
    WorkModule::set_int(
        boss_boma,
        *ITEM_BOSS_MODE_ADVENTURE_HARD,
        *ITEM_INSTANCE_WORK_INT_BOSS_MODE,
    );
    WorkModule::set_float(
        boss_boma,
        get_boss_intensity,
        *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
    );
    WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
    WorkModule::set_int(
        boss_boma,
        *ITEM_TRAIT_FLAG_BOSS,
        *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG,
    );
    WorkModule::set_int(
        boss_boma,
        *ITEM_VARIATION_CRAZYHAND_MASTERHAND_STANDARD,
        *ITEM_INSTANCE_WORK_INT_VARIATION,
    );
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
}

#[inline(always)]
unsafe fn restore_master_hand_after_item_wipe(
    module_accessor: *mut BattleObjectModuleAccessor,
    fighter_manager: *mut smash::app::FighterManager,
) {
    if module_accessor.is_null() || !sv_information::is_ready_go() || DEAD {
        return;
    }
    if !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager) {
        return;
    }

    let entry = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
    ENTRY_ID = entry;
    let tracked_id = BOSS_ID[entry];
    if tracked_id != 0 && sv_battle_object::is_active(tracked_id) {
        boss_helpers::ensure_boss_item_visible(sv_battle_object::module_accessor(tracked_id));
        return;
    }
    if let Some((_, held_id, _)) =
        boss_helpers::held_item_by_kind(module_accessor, &[*ITEM_KIND_MASTERHAND])
    {
        BOSS_ID[entry] = held_id;
        return;
    }

    ItemModule::remove_all(module_accessor);
    reset_master_runtime_for_spawn();
    EXISTS_PUBLIC = true;
    RESULT_SPAWNED = false;
    MASTER_EXISTS = true;
    let boss_boma = acquire_master_hand_item(module_accessor, entry);
    initialize_master_hand_boss(boss_boma, CONFIG.options.boss_difficulty.unwrap_or(10.0));
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    ModelModule::set_scale(module_accessor, 0.0001);
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(module_accessor),
        y: PostureModule::pos_y(module_accessor),
        z: PostureModule::pos_z(module_accessor),
    };
    PostureModule::set_pos(boss_boma, &boss_pos);
    StatusModule::change_status_request_from_script(
        boss_boma,
        *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
        true,
    );
    MotionModule::change_motion(
        boss_boma,
        Hash40::new("wait"),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    if !boss_helpers::is_operation_cpu_entry(fighter_manager, entry) {
        CONTROLLABLE = true;
    }
    crate::boss_log!(
        "[PB][Recover] entry {}: restored Master Hand after item wipe tracked_id=0x{:x} tracked_kind={} tracked_status={} cpu_entry={} host_scale={:.4}",
        entry,
        BOSS_ID[entry],
        smash::app::utility::get_kind(&mut *boss_boma),
        StatusModule::status_kind(boss_boma),
        boss_helpers::is_operation_cpu_entry(fighter_manager, entry),
        ModelModule::scale(module_accessor)
    );
}

#[inline(always)]
unsafe fn restore_crazy_hand_after_item_wipe(
    module_accessor: *mut BattleObjectModuleAccessor,
    fighter_manager: *mut smash::app::FighterManager,
) {
    if module_accessor.is_null() || !sv_information::is_ready_go() || DEAD_2 {
        return;
    }
    if !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager) {
        return;
    }

    let entry = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
    ENTRY_ID_2 = entry;
    let tracked_id = BOSS_ID_2[entry];
    if tracked_id != 0 && sv_battle_object::is_active(tracked_id) {
        boss_helpers::ensure_boss_item_visible(sv_battle_object::module_accessor(tracked_id));
        return;
    }
    if let Some((_, held_id, _)) =
        boss_helpers::held_item_by_kind(module_accessor, &[*ITEM_KIND_CRAZYHAND])
    {
        BOSS_ID_2[entry] = held_id;
        return;
    }

    ItemModule::remove_all(module_accessor);
    reset_crazy_runtime_for_spawn();
    EXISTS_PUBLIC_2 = true;
    RESULT_SPAWNED_2 = false;
    CRAZY_EXISTS = true;
    let boss_boma = acquire_crazy_hand_item(module_accessor, entry);
    initialize_crazy_hand_boss(boss_boma, CONFIG.options.boss_difficulty.unwrap_or(10.0));
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    ModelModule::set_scale(module_accessor, 0.0001);
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(module_accessor),
        y: PostureModule::pos_y(module_accessor),
        z: PostureModule::pos_z(module_accessor),
    };
    PostureModule::set_pos(boss_boma, &boss_pos);
    StatusModule::change_status_request_from_script(
        boss_boma,
        *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
        true,
    );
    MotionModule::change_motion(
        boss_boma,
        Hash40::new("wait"),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    if !boss_helpers::is_operation_cpu_entry(fighter_manager, entry) {
        CONTROLLABLE_2 = true;
    }
    crate::boss_log!(
        "[PB][Recover] entry {}: restored Crazy Hand after item wipe tracked_id=0x{:x} tracked_kind={} tracked_status={} cpu_entry={} host_scale={:.4}",
        entry,
        BOSS_ID_2[entry],
        smash::app::utility::get_kind(&mut *boss_boma),
        StatusModule::status_kind(boss_boma),
        boss_helpers::is_operation_cpu_entry(fighter_manager, entry),
        ModelModule::scale(module_accessor)
    );
}

#[skyline::hook(replace = MH_CHAKRAM_THROW_SUB)]
unsafe fn mh_chakram_throw_sub(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    if weapon_owner_is_player(lua_state) && AttackModule::is_attack(module_accessor, 0, false) {
        AttackModule::set_target_category(module_accessor, 0, *COLLISION_CATEGORY_MASK_ALL as u32);
    }
    original!()(item)
}

#[skyline::hook(replace = MH_IRON_BALL_THROW_SUB)]
unsafe fn mh_iron_ball_throw_sub(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    if weapon_owner_is_player(lua_state) && AttackModule::is_attack(module_accessor, 0, false) {
        AttackModule::set_target_category(module_accessor, 0, *COLLISION_CATEGORY_MASK_ALL as u32);
    }
    original!()(item)
}

#[skyline::hook(replace = MH_KENZAN_NEEDLE_SUB)]
unsafe fn mh_kenzan_needle_sub(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    if weapon_owner_is_player(lua_state) {
        if AttackModule::is_attack(module_accessor, 0, false) {
            AttackModule::set_target_category(
                module_accessor,
                0,
                *COLLISION_CATEGORY_MASK_ALL as u32,
            );
        }
        if AttackModule::is_attack(module_accessor, 1, false) {
            AttackModule::set_target_category(
                module_accessor,
                1,
                *COLLISION_CATEGORY_MASK_ALL as u32,
            );
        }
    }
    original!()(item)
}

#[inline(always)]
unsafe fn install_masterhand_kenzan_status(item: &mut L2CAgentBase) {
    let mh_kenzan_coroutine_func: &mut skyline::libc::c_void =
        std::mem::transmute(L2CValue::Ptr(mh_kenzan_coroutine as *const () as _).get_ptr());
    item.sv_set_status_func(
        L2CValue::I32(*ITEM_MASTERHAND_STATUS_KIND_KENZAN),
        L2CValue::I32(*ITEM_LUA_SCRIPT_STATUS_FUNC_STATUS_COROUTINE),
        mh_kenzan_coroutine_func,
    );
    let mh_kenzan_status_func: &mut skyline::libc::c_void =
        std::mem::transmute(L2CValue::Ptr(mh_kenzan_status as *const () as _).get_ptr());
    item.sv_set_status_func(
        L2CValue::I32(*ITEM_MASTERHAND_STATUS_KIND_KENZAN),
        L2CValue::I32(*ITEM_LUA_SCRIPT_STATUS_FUNC_STATUS),
        mh_kenzan_status_func,
    );
}

#[skyline::hook(replace = MH_WAIT_TIME_SETTING)]
unsafe fn mh_wait_time_setting(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    if WorkModule::is_flag(module_accessor, ITEM_INSTANCE_WORK_FLAG_PLAYER) {
        install_masterhand_kenzan_status(item);
    }
    original!()(item)
}

#[skyline::hook(replace = CH_FIRE_CHARIOT_MOTION, inline)]
unsafe fn ch_chariot_motion(ctx: &InlineCtx) {
    let agent_base: &mut L2CAgentBase =
        &mut *std::ptr::with_exposed_provenance_mut::<L2CAgentBase>(ctx.registers[20].x() as usize);
    if WorkModule::is_flag(agent_base.module_accessor, ITEM_INSTANCE_WORK_FLAG_PLAYER) == false {
        return;
    }
    let value: u64 = hash40("fire_chariot_start_5");
    asm!("mov x0, {}", in(reg) value);
}

#[skyline::hook(replace = CH_CHARIOT_SPEED, inline)]
unsafe fn ch_chariot_speed(ctx: &InlineCtx) {
    let agent_base: &mut L2CAgentBase =
        &mut *std::ptr::with_exposed_provenance_mut::<L2CAgentBase>(ctx.registers[22].x() as usize);
    if WorkModule::is_flag(agent_base.module_accessor, ITEM_INSTANCE_WORK_FLAG_PLAYER) == false {
        return;
    }
    let chariot_speed: f32 = 10.0;
    asm!("fmov s0, w8", in("w8") chariot_speed);
}

#[skyline::hook(replace = CH_CHARIOT_RADIUS_MIN, inline)]
unsafe fn ch_chariot_radius_min(ctx: &InlineCtx) {
    let agent_base: &mut L2CAgentBase =
        &mut *std::ptr::with_exposed_provenance_mut::<L2CAgentBase>(ctx.registers[22].x() as usize);
    if WorkModule::is_flag(agent_base.module_accessor, ITEM_INSTANCE_WORK_FLAG_PLAYER) == false {
        return;
    }
    let min_radius: f32 = 35.0;
    asm!("fmov s0, w8", in("w8") min_radius);
}

#[skyline::hook(replace = CH_CHARIOT_RADIUS_MAX, inline)]
unsafe fn ch_chariot_radius_max(ctx: &InlineCtx) {
    let agent_base: &mut L2CAgentBase =
        &mut *std::ptr::with_exposed_provenance_mut::<L2CAgentBase>(ctx.registers[22].x() as usize);
    if WorkModule::is_flag(agent_base.module_accessor, ITEM_INSTANCE_WORK_FLAG_PLAYER) == false {
        return;
    }
    let max_radius: f32 = 70.0;
    asm!("fmov s0, w8", in("w8") max_radius);
}

unsafe fn mh_kenzan_coroutine(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    MASTER_KENZAN_SPAWNED = false;
    println!(
        "[PB][MasterHand][Kenzan] coroutine start status={} motion={} pos=({:.2},{:.2},{:.2})",
        StatusModule::status_kind(module_accessor),
        MotionModule::motion_kind(module_accessor),
        PostureModule::pos_x(module_accessor),
        PostureModule::pos_y(module_accessor),
        PostureModule::pos_z(module_accessor),
    );
    MotionModule::change_motion(
        module_accessor,
        Hash40::new("kenzan"),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    smash::app::boss_private::main_energy_from_param(
        lua_state,
        ItemKind(*ITEM_KIND_MASTERHAND),
        Hash40::new("energy_param_kenzan"),
        0.0,
    );
    L2CValue::I32(0)
}

unsafe fn mh_kenzan_status(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    if MotionModule::is_end(module_accessor) && !MASTER_KENZAN_SPAWNED {
        let entry_id = boss_runtime::sanitize_entry_id(WorkModule::get_int(
            module_accessor,
            ITEM_INSTANCE_WORK_INT_ENTRY_ID,
        ) as usize);
        let spawn_x = PostureModule::pos_x(module_accessor)
            + (MASTER_KENZAN_SPAWN_X_OFFSET * PostureModule::lr(module_accessor));
        let kenzan_id = smash::app::boss_private::create_weapon(
            lua_state,
            ItemKind(*ITEM_KIND_MASTERHANDKENZAN),
            spawn_x,
            0.0,
            0.0,
            PostureModule::lr(module_accessor),
        ) as u32;
        println!(
            "[PB][MasterHand][Kenzan] motion end entry={} boss_id=0x{:x} spawn_x={:.2} spawn_offset={:.2} kenzan_id=0x{:x}",
            entry_id,
            BOSS_ID[entry_id],
            spawn_x,
            MASTER_KENZAN_SPAWN_X_OFFSET,
            kenzan_id,
        );
        if kenzan_id != 0 && sv_battle_object::is_active(kenzan_id) {
            let kenzan_boma = sv_battle_object::module_accessor(kenzan_id);
            if !kenzan_boma.is_null() {
                LinkModule::link(kenzan_boma, *ITEM_LINK_NO_MESSAGE, BOSS_ID[entry_id]);
                WorkModule::on_flag(kenzan_boma, ITEM_INSTANCE_WORK_FLAG_PLAYER);
                WorkModule::set_int(
                    kenzan_boma,
                    entry_id as i32,
                    ITEM_INSTANCE_WORK_INT_ENTRY_ID,
                );
            } else {
                println!("[PB][MasterHand][Kenzan] weapon accessor was null after create_weapon");
            }
        } else {
            println!("[PB][MasterHand][Kenzan] create_weapon failed or inactive");
        }
        MASTER_KENZAN_SPAWNED = true;
        StatusModule::change_status_request(
            module_accessor,
            *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END,
            false,
        );
    }
    L2CValue::I32(0)
}

fn nro_hook(info: &skyline::nro::NroInfo) {
    if info.name == "item" {
        MASTERCRAZY_ITEM_HOOKS_ONCE.call_once(|| unsafe {
            let module_base = (*info.module.ModuleObject).module_base as usize;
            CH_FIRE_CHARIOT_MOTION += module_base;
            skyline::install_hook!(ch_chariot_motion);
            CH_CHARIOT_SPEED += module_base;
            skyline::install_hook!(ch_chariot_speed);
            CH_CHARIOT_RADIUS_MAX += module_base;
            skyline::install_hook!(ch_chariot_radius_max);
            CH_CHARIOT_RADIUS_MIN += module_base;
            skyline::install_hook!(ch_chariot_radius_min);
            MH_WAIT_TIME_SETTING += module_base;
            skyline::install_hook!(mh_wait_time_setting);
            MH_CHAKRAM_THROW_SUB += module_base;
            skyline::install_hook!(mh_chakram_throw_sub);
            MH_IRON_BALL_THROW_SUB += module_base;
            skyline::install_hook!(mh_iron_ball_throw_sub);
            MH_KENZAN_NEEDLE_SUB += module_base;
            skyline::install_hook!(mh_kenzan_needle_sub);
        });
    }
}

pub unsafe fn check_status() -> bool {
    EXISTS_PUBLIC || boss_runtime::any_exists_public(&raw const boss_runtime::MASTER_HAND_RUNTIME)
}

pub unsafe fn check_status_2() -> bool {
    EXISTS_PUBLIC_2 || boss_runtime::any_exists_public(&raw const boss_runtime::CRAZY_HAND_RUNTIME)
}

#[inline(always)]
unsafe fn load_master_hand_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    CONTROLLABLE = (*slot).controllable;
    STOP = (*slot).stop;
    DEAD = (*slot).dead;
    RESULT_SPAWNED = (*slot).result_spawned;
    EXISTS_PUBLIC = (*slot).exists_public;
    JUMP_START = (*slot).jump_start;
    CONTROLLER_X_MASTER = (*slot).controller_x;
    CONTROLLER_Y_MASTER = (*slot).controller_y;
}

#[inline(always)]
unsafe fn store_master_hand_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    (*slot).controllable = CONTROLLABLE;
    (*slot).stop = STOP;
    (*slot).dead = DEAD;
    (*slot).result_spawned = RESULT_SPAWNED;
    (*slot).exists_public = EXISTS_PUBLIC;
    (*slot).fresh_control = false;
    (*slot).jump_start = JUMP_START;
    (*slot).controller_x = CONTROLLER_X_MASTER;
    (*slot).controller_y = CONTROLLER_Y_MASTER;
}

#[inline(always)]
unsafe fn load_crazy_hand_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    CONTROLLABLE_2 = (*slot).controllable;
    STOP_2 = (*slot).stop;
    DEAD_2 = (*slot).dead;
    RESULT_SPAWNED_2 = (*slot).result_spawned;
    EXISTS_PUBLIC_2 = (*slot).exists_public;
    JUMP_START_2 = (*slot).jump_start;
    CONTROLLER_X_CRAZY = (*slot).controller_x;
    CONTROLLER_Y_CRAZY = (*slot).controller_y;
}

#[inline(always)]
unsafe fn store_crazy_hand_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    (*slot).controllable = CONTROLLABLE_2;
    (*slot).stop = STOP_2;
    (*slot).dead = DEAD_2;
    (*slot).result_spawned = RESULT_SPAWNED_2;
    (*slot).exists_public = EXISTS_PUBLIC_2;
    (*slot).fresh_control = false;
    (*slot).jump_start = JUMP_START_2;
    (*slot).controller_x = CONTROLLER_X_CRAZY;
    (*slot).controller_y = CONTROLLER_Y_CRAZY;
}

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        if crate::should_quarantine_boss_frame(module_accessor) {
            return;
        }
        tick_finder_cooldown();
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            ENTRY_ID = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
            // The paired entrance coordinator is the sole writer while the
            // native Entry2 pair is active. Returning before the ordinary
            // host/item logic prevents CPU recovery, entry setup, or a second
            // status request from competing with the synchronized entrance.
            if hand_entrance_owns_entry(ENTRY_ID, false) {
                return;
            }
            let _runtime_guard = CommonRuntimeSyncGuard::new(
                boss_runtime::slot_ptr(&raw mut boss_runtime::MASTER_HAND_RUNTIME, ENTRY_ID),
                load_master_hand_runtime,
                store_master_hand_runtime,
            );
            let fighter_manager = boss_helpers::fighter_manager();

            let selected_via_slot =
                selection::is_selected_css_boss(module_accessor, *ITEM_KIND_MASTERHAND);
            if selected_via_slot {
                boss_helpers::clear_hidden_host_effects(module_accessor);
                let stage_id = smash::app::stage::get_stage_id();
                if boss_helpers::is_boss_preview_stage(stage_id) {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor =
                        smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001
                        || !ItemModule::is_have_item(module_accessor, 0)
                    {
                        ItemModule::remove_all(module_accessor);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = acquire_master_hand_item(module_accessor, ENTRY_ID);
                        ModelModule::set_scale(boss_boma, 0.08);
                        MotionModule::change_motion(
                            boss_boma,
                            Hash40::new("wait"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(
                            module_accessor,
                            Hash40::new("none"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                        ModelModule::set_joint_rotate(
                            module_accessor,
                            Hash40::new("root"),
                            &mut Vector3f {
                                x: -270.0,
                                y: 180.0,
                                z: -90.0,
                            },
                            smash::app::MotionNodeRotateCompose {
                                _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
                            },
                            ModelModule::rotation_order(module_accessor),
                        );
                    }
                } else if !boss_helpers::is_boss_passthrough_stage(stage_id) {
                    restore_master_hand_after_item_wipe(module_accessor, fighter_manager);
                    if sv_information::is_ready_go() == false {
                        let entry = boss_helpers::entry_id(module_accessor);
                        let needs_entry_init = !hand_entrance_owns_entry(entry, false)
                            && boss_helpers::needs_hidden_host_entry_init(
                                module_accessor,
                                &raw const BOSS_ID,
                                entry,
                            );
                        if needs_entry_init {
                            DEAD = false;
                            CONTROLLABLE = true;
                            // This reset must happen once for a new hidden
                            // host, not on every pre-Ready-Go frame. The old
                            // unconditional call released HandEntrance while
                            // its native pair was already active.
                            reset_master_runtime_for_spawn();
                        }
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor =
                            smash::app::sv_system::battle_object_module_accessor(lua_state);
                        let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                        ENTRY_ID = WorkModule::get_int(
                            module_accessor,
                            *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                        ) as usize;
                        let needs_entry_init = !hand_entrance_owns_entry(ENTRY_ID, false)
                            && boss_helpers::needs_hidden_host_entry_init(
                                module_accessor,
                                &raw const BOSS_ID,
                                ENTRY_ID,
                            );
                        if needs_entry_init {
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            MASTER_EXISTS = true;
                            let boss_boma = acquire_master_hand_item(module_accessor, ENTRY_ID);
                            initialize_master_hand_boss(boss_boma, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            let host_pos = Vector3f {
                                x: PostureModule::pos_x(module_accessor),
                                y: PostureModule::pos_y(module_accessor),
                                z: PostureModule::pos_z(module_accessor),
                            };
                            PostureModule::set_pos(boss_boma, &host_pos);
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_STATUS_KIND_FOR_BOSS_START,
                                true,
                            );
                        }
                    }

                    if !smash::app::smashball::is_training_mode()
                        && StatusModule::status_kind(module_accessor)
                            == *FIGHTER_STATUS_KIND_REBIRTH
                        && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                        && !STOP
                        && !CONFIG.options.boss_respawn.unwrap_or(false)
                    {
                        StatusModule::change_status_request_from_script(
                            module_accessor,
                            *FIGHTER_STATUS_KIND_DEAD,
                            true,
                        );
                    }
                    if !smash::app::smashball::is_training_mode()
                        && StatusModule::status_kind(module_accessor)
                            == *FIGHTER_STATUS_KIND_REBIRTH
                        && StatusModule::status_kind(module_accessor)
                            != *FIGHTER_STATUS_KIND_STANDBY
                        && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                        && STOP
                        && !CONFIG.options.boss_respawn.unwrap_or(false)
                    {
                        StatusModule::change_status_request_from_script(
                            module_accessor,
                            *FIGHTER_STATUS_KIND_STANDBY,
                            true,
                        );
                        let x = 0.0;
                        let y = 0.0;
                        let z = 0.0;
                        let module_pos = Vector3f { x: x, y: y, z: z };
                        PostureModule::set_pos(module_accessor, &module_pos);
                    }

                    // Respawn in case of Squad Strike or Specific Circumstances

                    if sv_information::is_ready_go()
                        && !ItemModule::is_have_item(module_accessor, 0)
                        && ModelModule::scale(module_accessor) == 0.0001
                        && StatusModule::status_kind(module_accessor)
                            == *FIGHTER_STATUS_KIND_REBIRTH
                    {
                        if smash::app::smashball::is_training_mode()
                            || CONFIG.options.boss_respawn.unwrap_or(false)
                        {
                            StatusModule::change_status_request_from_script(
                                module_accessor,
                                *FIGHTER_STATUS_KIND_FALL,
                                true,
                            );
                            DEAD = false;
                            CONTROLLABLE = true;
                            reset_master_runtime_for_spawn();
                            MASTER_TEAM = TeamModule::team_no(module_accessor);
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor =
                                smash::app::sv_system::battle_object_module_accessor(lua_state);
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(1.0);
                            ENTRY_ID = WorkModule::get_int(
                                module_accessor,
                                *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                            ) as usize;
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            MASTER_EXISTS = true;
                            let boss_boma = acquire_master_hand_item(module_accessor, ENTRY_ID);
                            initialize_master_hand_boss(boss_boma, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
                                true,
                            );

                            let x = PostureModule::pos_x(module_accessor);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(module_accessor);
                            let module_pos = Vector3f { x: x, y: y, z: z };
                            PostureModule::set_pos(boss_boma, &module_pos);

                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == false
                            {
                                CONTROLLABLE = true;
                            }
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        boss_helpers::ensure_boss_item_visible(boss_boma);
                        let x = PostureModule::pos_x(boss_boma);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(boss_boma);
                        MASTER_X_POS = x;
                        MASTER_Y_POS = y;
                        MASTER_Z_POS = z;
                        if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                            let sub_hp = 999.0
                                - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                        {
                            WorkModule::off_flag(
                                boss_boma,
                                *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK,
                            );
                            WorkModule::off_flag(
                                boss_boma,
                                *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM,
                            );
                            WorkModule::off_flag(
                                boss_boma,
                                *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT,
                            );
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                            // right
                            MASTER_FACING_LEFT = false;
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                            // left
                            MASTER_FACING_LEFT = true;
                        }
                        JostleModule::set_status(module_accessor, false);
                    }

                    if sv_information::is_ready_go()
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_BARK
                            && !CRAZY_USABLE
                            && !HAND_TEAM_AUTHORITY_ACTIVE
                        {
                            BARK = false;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT,
                                true,
                            );
                        }
                    }
                    if sv_information::is_ready_go() == true
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        if MotionModule::motion_kind(boss_boma) == hash40("wait") && !DEAD {
                            SoundModule::stop_se(
                                boss_boma,
                                smash::phx::Hash40::new("se_boss_masterhand_chakram_fly"),
                                0,
                            );
                        }
                    }
                    // Team Attack Trigger
                    if sv_information::is_ready_go() == true
                        && !DEAD
                        && !FINDER
                        && !HAND_TEAM_AUTHORITY_ACTIVE
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        let curr_pos = Vector3f {
                            x: PostureModule::pos_x(module_accessor),
                            y: PostureModule::pos_y(module_accessor),
                            z: PostureModule::pos_z(module_accessor),
                        };
                        if MotionModule::motion_kind(boss_boma) == hash40("wait")
                            && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                        {
                            if CONTROLLABLE == false
                                && smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                    == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                || CONTROLLABLE
                                    && smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                                        == smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                            {
                                if GroundModule::get_distance_to_floor(
                                    module_accessor,
                                    &curr_pos,
                                    curr_pos.y,
                                    true,
                                ) <= 40.0
                                    && GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) > 0.0
                                    && CRAZY_EXISTS
                                    && CRAZY_USABLE
                                    && MASTER_TEAM == CRAZY_TEAM
                                {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT
                                    {
                                        CONTROLLABLE = false;
                                        BARK = true;
                                        PUNCH = false;
                                        SHOCK = false;
                                        LASER = false;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_BARK,
                                            true,
                                        );
                                    }
                                }
                            } else if CONTROLLABLE == false
                                && smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                    == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                || CONTROLLABLE
                                    && smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                                        == smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                            {
                                if GroundModule::get_distance_to_floor(
                                    module_accessor,
                                    &curr_pos,
                                    curr_pos.y,
                                    true,
                                ) <= 50.0
                                    && GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) > 0.0
                                    && CRAZY_EXISTS
                                    && CRAZY_USABLE
                                    && MASTER_TEAM == CRAZY_TEAM
                                {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT
                                    {
                                        CONTROLLABLE = false;
                                        BARK = false;
                                        PUNCH = false;
                                        SHOCK = false;
                                        LASER = false;
                                        SCRATCH_BLOW = true;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START,
                                            true,
                                        );
                                    }
                                }
                            } else if CONTROLLABLE == false
                                && smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                    == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                || CONTROLLABLE
                                    && smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                                        == smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                            {
                                if CRAZY_EXISTS == true && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM
                                {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT
                                    {
                                        CONTROLLABLE = false;
                                        BARK = false;
                                        PUNCH = false;
                                        SHOCK = true;
                                        LASER = false;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START,
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // STUBS AI

                    if sv_information::is_ready_go()
                        && !DEAD
                        && !FINDER
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                            && StatusModule::status_kind(boss_boma)
                                != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                            && StatusModule::status_kind(boss_boma)
                                != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                        {
                            if CONTROLLABLE {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                                if StatusModule::status_kind(boss_boma)
                                    != *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_TURN
                                {
                                    MotionModule::change_motion(
                                        boss_boma,
                                        smash::phx::Hash40::new("wait"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT,
                                        true,
                                    );
                                }
                                if StatusModule::status_kind(boss_boma)
                                    == *ITEM_MASTERHAND_STATUS_KIND_TURN
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_TURN
                                {
                                    MotionModule::change_motion(
                                        boss_boma,
                                        smash::phx::Hash40::new("wait"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT,
                                        true,
                                    );
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD == true && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0 {
                            let boss_boma = sv_battle_object::module_accessor(
                                BOSS_ID[boss_helpers::entry_id(module_accessor)],
                            );
                            if STOP == false
                                && CONFIG.options.boss_respawn.unwrap_or(false)
                                && StatusModule::status_kind(module_accessor)
                                    != *FIGHTER_STATUS_KIND_STANDBY
                            {
                                StatusModule::change_status_request_from_script(
                                    module_accessor,
                                    *FIGHTER_STATUS_KIND_STANDBY,
                                    true,
                                );
                            }
                            MASTER_EXISTS = false;
                            if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_DEAD
                                || StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD
                                    && MotionModule::frame(boss_boma) > 250.0
                            {
                                HitModule::set_whole(
                                    module_accessor,
                                    smash::app::HitStatus(*HIT_STATUS_OFF),
                                    0,
                                );
                                HitModule::set_whole(
                                    boss_boma,
                                    smash::app::HitStatus(*HIT_STATUS_OFF),
                                    0,
                                );
                                ItemModule::remove_all(module_accessor);
                                if STOP == false && CONFIG.options.boss_respawn.unwrap_or(false) {
                                    StatusModule::change_status_request_from_script(
                                        module_accessor,
                                        *FIGHTER_STATUS_KIND_DEAD,
                                        true,
                                    );
                                    STOP = true;
                                }
                                if STOP == false && !CONFIG.options.boss_respawn.unwrap_or(false) {
                                    boss_helpers::request_hidden_host_stock_drain(
                                        module_accessor,
                                        fighter_manager,
                                        ENTRY_ID,
                                        &raw mut STOP,
                                    );
                                }
                            }
                        }
                    }

                    if DEAD == true {
                        if sv_information::is_ready_go() == true
                            && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                        {
                            let boss_boma = sv_battle_object::module_accessor(
                                BOSS_ID[boss_helpers::entry_id(module_accessor)],
                            );
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_STANDBY
                                {
                                    MASTER_EXISTS = false;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                        // left
                                        let vec3 = Vector3f {
                                            x: 0.0,
                                            y: 0.0,
                                            z: 0.0,
                                        };
                                        PostureModule::set_rot(boss_boma, &vec3, 0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                        // right
                                        let vec3 = Vector3f {
                                            x: 0.0,
                                            y: 180.0,
                                            z: 0.0,
                                        };
                                        PostureModule::set_rot(boss_boma, &vec3, 0);
                                    }
                                    if MotionModule::frame(boss_boma) == 0.0 {
                                        smash_script::macros::CAM_ZOOM_IN_arg5(
                                            fighter, 0.0, 0.0, 5.0, 0.0, 0.0,
                                        );
                                        smash_script::macros::EFFECT_OFF_KIND(
                                            fighter,
                                            Hash40::new("sys_dead"),
                                            true,
                                            false,
                                        );
                                        smash_script::macros::EFFECT(
                                            fighter,
                                            Hash40::new("sys_bg_criticalhit"),
                                            Hash40::new("top"),
                                            0,
                                            7,
                                            0,
                                            0,
                                            0,
                                            0,
                                            1,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            false,
                                        );
                                        smash_script::macros::EFFECT(
                                            fighter,
                                            Hash40::new("sys_bg_boss_finishhit"),
                                            Hash40::new("top"),
                                            0,
                                            7,
                                            0,
                                            0,
                                            0,
                                            0,
                                            1,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            false,
                                        );
                                    }
                                    if MotionModule::frame(boss_boma) == 0.5 {
                                        SlowModule::set_whole(module_accessor, 100, 0);
                                    }
                                    if MotionModule::frame(boss_boma) == 1.0 {
                                        SlowModule::clear_whole(module_accessor);
                                        SlowModule::set_whole(module_accessor, 10, 0);
                                        CameraModule::reset_all(module_accessor);
                                    }
                                    if MotionModule::frame(boss_boma) >= 1.1 {
                                        CameraModule::reset_all(module_accessor);
                                    }
                                    if MotionModule::frame(boss_boma) >= 5.0 {
                                        CameraModule::reset_all(module_accessor);
                                        smash_script::macros::CAM_ZOOM_OUT(fighter);
                                        smash_script::macros::EFFECT_OFF_KIND(
                                            fighter,
                                            Hash40::new("sys_bg_criticalhit"),
                                            true,
                                            false,
                                        );
                                        smash_script::macros::EFFECT_OFF_KIND(
                                            fighter,
                                            Hash40::new("sys_bg_boss_finishhit"),
                                            true,
                                            false,
                                        );
                                        SlowModule::clear_whole(module_accessor);
                                    }
                                    if MotionModule::frame(boss_boma)
                                        >= MotionModule::end_frame(boss_boma) - 10.0
                                    {
                                        EXISTS_PUBLIC = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_STATUS_KIND_STANDBY,
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY
                        {
                            FighterManager::set_cursor_whole(fighter_manager, false);
                            ArticleModule::set_visibility_whole(
                                module_accessor,
                                *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP,
                                false,
                                smash::app::ArticleOperationTarget(0),
                            );
                            StatusModule::change_status_request_from_script(
                                module_accessor,
                                *FIGHTER_STATUS_KIND_WAIT,
                                true,
                            );
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY
                            && !CRAZY_EXISTS
                        {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY
                            && CRAZY_EXISTS
                        {
                            CONTROLLABLE = true;
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                            MASTER_TEAM = TeamModule::team_no(module_accessor);
                            if MASTER_TEAM == CRAZY_TEAM {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                MotionModule::change_motion(
                                    boss_boma,
                                    smash::phx::Hash40::new("entry2"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                            }
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("entry2") {
                            MotionModule::set_rate(boss_boma, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.5,
                            );
                        }
                    }

                    // SET FIGHTER LOOP

                    if sv_information::is_ready_go() == true {
                        if StatusModule::status_kind(module_accessor)
                            != *FIGHTER_STATUS_KIND_STANDBY
                        {
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING,
                            ); // I did yoink these transition terms and ability to hide the player cursor from Claude's awesome mod which can be found here: https://github.com/ClaudevonRiegan/Playable_Bosses
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ITEM,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_SPECIAL,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_JUMP_AERIAL,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_TREAD_JUMP,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ITEM_THROW,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ATTACK,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_WALL_JUMP,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ESCAPE,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_CATCH,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_JUMP,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_GUARD,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ATTACK,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ESCAPE,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_CLIFF,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LASSO,
                            );
                            FighterManager::set_cursor_whole(fighter_manager, false);
                            fighter.set_situation(SITUATION_KIND_AIR.into());
                            GroundModule::set_correct(
                                module_accessor,
                                smash::app::GroundCorrectKind(*GROUND_CORRECT_KIND_AIR),
                            );
                            MotionModule::change_motion(
                                module_accessor,
                                Hash40::new("fall"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }
                    }

                    if DEAD == false {
                        // SET POS AND STOPS OUT OF BOUNDS
                        if !FINDER
                            && ModelModule::scale(module_accessor) == 0.0001
                            && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                        {
                            let boss_boma = sv_battle_object::module_accessor(
                                BOSS_ID[boss_helpers::entry_id(module_accessor)],
                            );
                            if FighterUtil::is_hp_mode(module_accessor) == true {
                                if StatusModule::status_kind(module_accessor)
                                    == *FIGHTER_STATUS_KIND_DEAD
                                    || StatusModule::status_kind(module_accessor) == 79
                                {
                                    if DEAD == false {
                                        CONTROLLABLE = false;
                                        DEAD = true;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_STATUS_KIND_DEAD,
                                            true,
                                        );
                                    }
                                }
                            }
                            let x = PostureModule::pos_x(boss_boma);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(boss_boma);
                            let boss_pos = Vector3f {
                                x: x,
                                y: y + 20.0,
                                z: z,
                            };
                            if !CONTROLLABLE
                                || boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == true
                            {
                                if PostureModule::pos_y(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0
                                {
                                    let boss_y_pos_2 = Vector3f {
                                        x: x,
                                        y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                {
                                    let boss_x_pos_1 = Vector3f {
                                        x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0
                                {
                                    let boss_x_pos_2 = Vector3f {
                                        x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                } else if PostureModule::pos_y(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                {
                                    let boss_y_pos_1 = Vector3f {
                                        x: x,
                                        y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: x,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                } else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            } else {
                                if PostureModule::pos_y(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0
                                {
                                    let boss_y_pos_2 = Vector3f {
                                        x: x,
                                        y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                {
                                    let boss_x_pos_1 = Vector3f {
                                        x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0
                                {
                                    let boss_x_pos_2 = Vector3f {
                                        x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                } else if PostureModule::pos_y(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                {
                                    let boss_y_pos_1 = Vector3f {
                                        x: x,
                                        y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: x,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                } else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                            if master_should_clamp_floor(boss_boma) {
                                boss_helpers::clamp_flying_boss_floor(
                                    module_accessor,
                                    boss_boma,
                                    MASTER_FLOAT_FLOOR_CLEARANCE,
                                );
                            }
                        }
                    }

                    // DAMAGE MODULES

                    if BOSS_ID[boss_helpers::entry_id(module_accessor)] == 0 {
                        return;
                    }
                    let boss_boma = sv_battle_object::module_accessor(
                        BOSS_ID[boss_helpers::entry_id(module_accessor)],
                    );
                    HitModule::set_whole(
                        module_accessor,
                        smash::app::HitStatus(*HIT_STATUS_OFF),
                        0,
                    );
                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma, i, false) {
                            AttackModule::set_target_category(
                                boss_boma,
                                i,
                                *COLLISION_CATEGORY_MASK_ALL as u32,
                            );
                        }
                    }
                    if MASTER_LAST_IRON_BALL_ID != 0 {
                        if !sv_battle_object::is_active(MASTER_LAST_IRON_BALL_ID) {
                            MASTER_LAST_IRON_BALL_ID = 0;
                            MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
                        } else {
                            let iron_ball_boma =
                                sv_battle_object::module_accessor(MASTER_LAST_IRON_BALL_ID);
                            if !iron_ball_boma.is_null() {
                                if AttackModule::is_attack(iron_ball_boma, 0, false) {
                                    AttackModule::set_target_category(
                                        iron_ball_boma,
                                        0,
                                        *COLLISION_CATEGORY_MASK_ALL as u32,
                                    );
                                }
                                if MotionModule::motion_kind(iron_ball_boma) == hash40("appear") {
                                    AttackModule::clear_all(iron_ball_boma);
                                }
                                if StatusModule::status_kind(iron_ball_boma)
                                    == *ITEM_MASTERHANDIRONBALL_STATUS_KIND_MOVE1
                                {
                                    action(
                                        iron_ball_boma,
                                        *ITEM_MASTERHANDIRONBALL_ACTION_SET_BOUND,
                                        0.0,
                                    );
                                    StatusModule::change_status_request_from_script(
                                        iron_ball_boma,
                                        *ITEM_MASTERHANDIRONBALL_STATUS_KIND_MOVE2,
                                        true,
                                    );
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if FighterUtil::is_hp_mode(module_accessor) == false {
                            let hp = CONFIG.options.master_hand_hp.unwrap_or(400.0);
                            if DamageModule::damage(module_accessor, 0) >= hp {
                                // HEALTH
                                if DEAD == false {
                                    CONTROLLABLE = false;
                                    DEAD = true;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_STATUS_KIND_DEAD,
                                        true,
                                    );
                                    if FINDER {
                                        clear_finder_runtime_with_reason("master_death");
                                    }
                                }
                            }
                        }
                    }

                    // FIXES SPAWN

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            if JUMP_START == false {
                                JUMP_START = true;
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == true
                                {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
                                        true,
                                    );
                                } else {
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME,
                                        true,
                                    );
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true && !DEAD {
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                        {
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                // right
                                let master_pos = Vector3f {
                                    x: CRAZY_X_POS + 100.0,
                                    y: CRAZY_Y_POS,
                                    z: CRAZY_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                // left
                                let master_pos = Vector3f {
                                    x: CRAZY_X_POS - 100.0,
                                    y: CRAZY_Y_POS,
                                    z: CRAZY_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_BARK
                        {
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 95.0
                                && MotionModule::frame(boss_boma)
                                    <= MotionModule::end_frame(boss_boma) - 92.0
                            {
                                MotionModule::set_rate(boss_boma, 0.1);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 0.1,
                                );
                            } else {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == false
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                    BARK = false;
                                }
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    BARK = false;
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                        {
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 10.0
                            {
                                LASER = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW
                        {
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 10.0
                            {
                                SCRATCH_BLOW = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                        {
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 10.0
                            {
                                PUNCH = false;
                            }
                        }
                        sync_hand_team_authority_from_flags(boss_boma, ENTRY_ID);
                        let hand_team_active = hand_team_authority_active_for_boma(boss_boma);
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                            CONTROLLABLE = false;
                        }
                        if !hand_team_active
                            && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                            && StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                        {
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
                                true,
                            );
                        }
                        if !FINDER
                            && !hand_team_active
                            && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                        {
                            maybe_recover_master_cpu_idle(boss_boma, ENTRY_ID);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                            && !DEAD
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                            let stunned = !CONFIG.options.full_stun_duration.unwrap_or(false);
                            if stunned {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_DOWN_END,
                                    true,
                                );
                            }
                            CONTROLLABLE = false;
                        }
                        if MotionModule::motion_kind(boss_boma)
                            == smash::hash40("electroshock_start")
                            && SHOCK
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                            CONTROLLABLE = false;
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 5.0
                                && !DEAD
                            {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK,
                                    true,
                                );
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                            && StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_END
                            && !CONTROLLABLE
                            && SHOCK
                        {
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 5.0
                            {
                                CONTROLLABLE = true;
                                SHOCK = false;
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true
                            && StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_END
                            && SHOCK
                        {
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 5.0
                            {
                                SHOCK = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME
                            || StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                            || CONTROLLABLE
                        {
                            MASTER_USABLE = true;
                        } else {
                            MASTER_USABLE = false;
                        }

                        if PUNCH
                            && StatusModule::status_kind(boss_boma)
                                != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                            && CRAZY_EXISTS
                            && !DEAD
                            && MASTER_USABLE
                        {
                            CONTROLLABLE = false;
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                // right
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == true
                                {
                                    let master_pos = Vector3f {
                                        x: CRAZY_X_POS - 130.0,
                                        y: CRAZY_Y_POS + 15.0,
                                        z: CRAZY_Z_POS,
                                    };
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                } else {
                                    let master_pos = Vector3f {
                                        x: CRAZY_X_POS - 130.0,
                                        y: CRAZY_Y_POS + 10.0,
                                        z: CRAZY_Z_POS,
                                    };
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                // left
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == true
                                {
                                    let master_pos = Vector3f {
                                        x: CRAZY_X_POS + 130.0,
                                        y: CRAZY_Y_POS + 15.0,
                                        z: CRAZY_Z_POS,
                                    };
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                } else {
                                    let master_pos = Vector3f {
                                        x: CRAZY_X_POS + 130.0,
                                        y: CRAZY_Y_POS + 10.0,
                                        z: CRAZY_Z_POS,
                                    };
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                            }
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_GOOPAA,
                                true,
                            );
                        }
                        if PUNCH
                            && !DEAD
                            && StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                            && MASTER_USABLE
                        {
                            MotionModule::set_rate(boss_boma, 1.15);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.15,
                            );
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                            {
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS,
                                    y: CRAZY_Y_POS + 15.0,
                                    z: CRAZY_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma, &master_pos);
                            } else {
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS,
                                    y: CRAZY_Y_POS + 10.0,
                                    z: CRAZY_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 10.0
                            {
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == false
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                        }

                        if FINDER && CRAZY_EXISTS && !DEAD {
                            // The native Finder status owns the hand motion,
                            // partner synchronization, and camera/dead-area
                            // work. Do not replay the motion or pin the hand
                            // in place from this per-frame hook.
                            CONTROLLABLE = false;
                        } else if !FINDER
                            && MotionModule::motion_kind(boss_boma) == hash40("finder")
                        {
                            crate::boss_log!(
                                "[PB][Finder][MasterRuntime] recover_after_clear status={} frame={:.1}/{:.1}",
                                StatusModule::status_kind(boss_boma),
                                MotionModule::frame(boss_boma),
                                MotionModule::end_frame(boss_boma)
                            );
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
                                    true,
                                );
                            } else {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                CONTROLLABLE = true;
                            }
                            MotionModule::change_motion(
                                boss_boma,
                                Hash40::new("wait"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }

                        if LASER
                            && StatusModule::status_kind(boss_boma)
                                != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                            && CRAZY_EXISTS
                            && !DEAD
                            && MASTER_USABLE
                        {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                // right
                                let master_pos = Vector3f {
                                    x: CRAZY_X_POS + 130.0,
                                    y: CRAZY_Y_POS,
                                    z: CRAZY_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                // left
                                let master_pos = Vector3f {
                                    x: CRAZY_X_POS - 130.0,
                                    y: CRAZY_Y_POS,
                                    z: CRAZY_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START,
                                true,
                            );
                        }
                        if LASER
                            && !DEAD
                            && StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                            && MASTER_USABLE
                        {
                            if MotionModule::frame(boss_boma)
                                >= MotionModule::end_frame(boss_boma) - 10.0
                            {
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == false
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START
                        {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_FIRING
                        {
                            MotionModule::set_rate(boss_boma, 2.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_HOLD
                        {
                            MotionModule::set_rate(boss_boma, 2.4);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_SHOOT
                        {
                            MotionModule::set_rate(boss_boma, 2.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END
                        {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM
                        {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );

                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER
                                < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER >= 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER
                                > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER <= 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER
                                < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER >= 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER
                                > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER <= 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f {
                                x: CONTROLLER_X_MASTER * 0.75,
                                y: CONTROLLER_Y_MASTER * 0.75,
                                z: 0.0,
                            };
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                        {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.2,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW
                        {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.2,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START
                        {
                            MotionModule::set_rate(boss_boma, 1.5);
                            //BOSS POSITION
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 1.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 1.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD
                        {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_TARGET_FOUND);
                            WorkModule::set_float(
                                boss_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_X,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Y,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Z,
                            );
                            MotionModule::set_rate(boss_boma, 2.0);
                            //BOSS POSITION
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 1.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 1.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOMING
                        {
                            MotionModule::set_rate(boss_boma, 2.0);
                            //BOSS POSITION
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 1.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 1.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END
                        {
                            MotionModule::set_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD
                        {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.2,
                            );
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == false
                            {
                                PostureModule::set_pos(
                                    boss_boma,
                                    &Vector3f {
                                        x: PostureModule::pos_x(boss_boma),
                                        y: Y_POS,
                                        z: PostureModule::pos_z(boss_boma),
                                    },
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU
                            || StatusModule::status_kind(boss_boma) == 78
                        {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == false
                            {
                                PostureModule::set_pos(
                                    boss_boma,
                                    &Vector3f {
                                        x: PostureModule::pos_x(boss_boma),
                                        y: Y_POS,
                                        z: PostureModule::pos_z(boss_boma),
                                    },
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START
                        {
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER
                                < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER >= 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER
                                > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER <= 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER
                                < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER >= 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER
                                > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER <= 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f {
                                x: CONTROLLER_X_MASTER,
                                y: CONTROLLER_Y_MASTER,
                                z: 0.0,
                            };
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING
                        {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.75,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.75,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.75,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.75,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER
                                < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER >= 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER
                                > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER <= 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER
                                < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER >= 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER
                                > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER <= 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f {
                                x: CONTROLLER_X_MASTER * 0.75,
                                y: CONTROLLER_Y_MASTER * 0.75,
                                z: 0.0,
                            };
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL
                        {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.1);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER
                                < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER >= 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER
                                > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER <= 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER
                                < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER >= 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER
                                > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER <= 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f {
                                x: CONTROLLER_X_MASTER * 0.75,
                                y: CONTROLLER_Y_MASTER * 0.75,
                                z: 0.0,
                            };
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_PRE_MOVE
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL
                        {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START
                        {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER
                                < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER >= 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER
                                > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER <= 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER
                                < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER >= 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER
                                > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER <= 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f {
                                x: CONTROLLER_X_MASTER * 0.75,
                                y: CONTROLLER_Y_MASTER * 0.75,
                                z: 0.0,
                            };
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL
                        {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER
                                < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER >= 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER
                                > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER <= 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER
                                < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER >= 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER
                                > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER <= 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f {
                                x: CONTROLLER_X_MASTER * 0.75,
                                y: CONTROLLER_Y_MASTER * 0.75,
                                z: 0.0,
                            };
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if (StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_PRE_MOVE)
                            && !DEAD
                        {
                            if boss_floor_y(module_accessor, boss_boma).is_none() {
                                MASTER_IRON_BALL_OFFSTAGE_FRAMES += 1;
                                if MASTER_IRON_BALL_OFFSTAGE_FRAMES
                                    > MASTER_IRON_BALL_OFFSTAGE_LIMIT
                                {
                                    cancel_master_iron_ball(
                                        module_accessor,
                                        boss_boma,
                                        "offstage_timeout",
                                    );
                                }
                            } else {
                                MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
                            }
                            if WorkModule::is_flag(
                                boss_boma,
                                *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_THROW,
                            ) {
                                if ItemModule::is_have_item(module_accessor, 0) {
                                    let held_item_id =
                                        ItemModule::get_have_item_id(module_accessor, 0) as u32;
                                    if held_item_id != 0
                                        && sv_battle_object::is_active(held_item_id)
                                    {
                                        let held_item_boma =
                                            sv_battle_object::module_accessor(held_item_id);
                                        if !held_item_boma.is_null()
                                            && smash::app::utility::get_kind(&mut *held_item_boma)
                                                == *ITEM_KIND_MASTERHANDIRONBALL
                                        {
                                            ItemModule::remove_item(module_accessor, 0);
                                        }
                                    }
                                }
                                let mut throw_joint = Vector3f {
                                    x: PostureModule::pos_x(boss_boma),
                                    y: PostureModule::pos_y(boss_boma),
                                    z: PostureModule::pos_z(boss_boma),
                                };
                                let throw_joint = ModelModule::joint_global_position(
                                    boss_boma,
                                    Hash40::new("throw"),
                                    &mut throw_joint,
                                    true,
                                );
                                let iron_ball_id = smash::app::boss_private::create_weapon(
                                    lua_state,
                                    ItemKind(*ITEM_KIND_MASTERHANDIRONBALL),
                                    throw_joint.x,
                                    throw_joint.y - 1.0,
                                    throw_joint.z,
                                    lua_bind::PostureModule::lr(boss_boma),
                                ) as u32;
                                if iron_ball_id != 0 && sv_battle_object::is_active(iron_ball_id) {
                                    MASTER_LAST_IRON_BALL_ID = iron_ball_id;
                                    let iron_ball_boma =
                                        sv_battle_object::module_accessor(iron_ball_id);
                                    if !iron_ball_boma.is_null() {
                                        LinkModule::remove_model_constraint(iron_ball_boma, true);
                                        if LinkModule::is_link(iron_ball_boma, *ITEM_LINK_NO_HAVE) {
                                            LinkModule::unlink(iron_ball_boma, *ITEM_LINK_NO_HAVE);
                                        }
                                        action(
                                            iron_ball_boma,
                                            *ITEM_MASTERHANDIRONBALL_ACTION_SET_BOUND,
                                            0.0,
                                        );
                                        StatusModule::change_status_request_from_script(
                                            iron_ball_boma,
                                            *ITEM_MASTERHANDIRONBALL_STATUS_KIND_MOVE2,
                                            true,
                                        );
                                    }
                                } else {
                                    MASTER_LAST_IRON_BALL_ID = 0;
                                }
                                WorkModule::off_flag(
                                    boss_boma,
                                    *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_CREATE,
                                );
                                WorkModule::off_flag(
                                    boss_boma,
                                    *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_THROW,
                                );
                            }
                        } else {
                            MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_DRILL_START
                        {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END
                        {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START
                        {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.3,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI
                        {
                            MotionModule::set_rate(boss_boma, 1.1);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.1,
                            );
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.2,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.2,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.2,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END
                        {
                            MotionModule::set_rate(boss_boma, 1.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.4,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                            || StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LANDING
                        {
                            CONTROLLABLE = false;
                        }
                        if MotionModule::is_end(boss_boma)
                            && MotionModule::motion_kind(boss_boma) == hash40("teleport_end")
                            && !DEAD
                        {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                            {
                                MotionModule::change_motion(
                                    boss_boma,
                                    Hash40::new("wait"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
                                    true,
                                );
                            } else {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                MotionModule::change_motion(
                                    boss_boma,
                                    Hash40::new("wait"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                                CONTROLLABLE = true;
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                        {
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CANCEL
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    MotionModule::set_rate(boss_boma, 2.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma, 2.0,
                                    );
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_RUSH_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME,
                                        true,
                                    );
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_THROW_END_1
                            {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_MISS_END
                            {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START
                                || StatusModule::status_kind(boss_boma)
                                    == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE
                                || StatusModule::status_kind(boss_boma)
                                    == *ITEM_MASTERHAND_STATUS_KIND_KENZAN
                            {
                                if let Some(floor_y) = boss_floor_y(module_accessor, boss_boma) {
                                    let target_y = floor_y + MASTER_KENZAN_GROUND_CLEARANCE;
                                    PostureModule::set_pos(
                                        boss_boma,
                                        &Vector3f {
                                            x: PostureModule::pos_x(boss_boma),
                                            y: target_y,
                                            z: PostureModule::pos_z(boss_boma),
                                        },
                                    );
                                    PostureModule::set_pos(
                                        module_accessor,
                                        &Vector3f {
                                            x: PostureModule::pos_x(boss_boma),
                                            y: target_y,
                                            z: PostureModule::pos_z(boss_boma),
                                        },
                                    );
                                    if MotionModule::frame(boss_boma) <= 1.0 {
                                        println!(
                                            "[PB][MasterHand][Kenzan] active status={} frame={:.2} y={:.2} target_y={:.2}",
                                            StatusModule::status_kind(boss_boma),
                                            MotionModule::frame(boss_boma),
                                            PostureModule::pos_y(boss_boma),
                                            target_y,
                                        );
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    MASTER_KENZAN_SPAWNED = false;
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                != *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START
                                && StatusModule::status_kind(boss_boma)
                                    != *ITEM_MASTERHAND_STATUS_KIND_KENZAN
                                && StatusModule::status_kind(boss_boma)
                                    != *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END
                                && StatusModule::status_kind(boss_boma)
                                    != *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE
                            {
                                MASTER_KENZAN_SPAWNED = false;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_END
                            {
                                if MASTER_IRON_BALL_SMOOTH_CANCEL {
                                    let tail_start_frame = (MotionModule::end_frame(boss_boma)
                                        - MASTER_IRON_BALL_END_TAIL_FRAMES)
                                        .max(0.0);
                                    if MotionModule::frame(boss_boma) < tail_start_frame {
                                        MotionModule::set_frame(boss_boma, tail_start_frame, false);
                                    }
                                    println!(
                                        "[PB][MasterHand][IronBall] smooth end tail_start={:.2} current={:.2}",
                                        tail_start_frame,
                                        MotionModule::frame(boss_boma),
                                    );
                                    MASTER_IRON_BALL_SMOOTH_CANCEL = false;
                                }
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                            {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
                            {
                                CONTROLLABLE = false;
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT
                            {
                                CONTROLLABLE = !FINDER;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME
                            {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT
                            {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_RND_WAIT
                            {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE
                            {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TO_POINT
                            {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT
                            {
                                CONTROLLABLE = true;
                            }
                        }

                        if CONTROLLABLE
                            && StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_TURN
                        {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if CONTROLLABLE
                            && StatusModule::status_kind(boss_boma)
                                != *ITEM_MASTERHAND_STATUS_KIND_TURN
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOMING
                        {
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_ATTACK,
                            ) {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD,
                                    true,
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_DRILL_ATTACK
                        {
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_ATTACK,
                            ) {
                                MotionModule::set_rate(boss_boma, 4.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 4.0,
                                );
                            }
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_ATTACK,
                            ) == false
                            {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 2.0,
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_DRILL_START
                        {
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_ATTACK,
                            ) {
                                MotionModule::set_rate(boss_boma, 3.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 3.0,
                                );
                            }
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_ATTACK,
                            ) == false
                            {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 2.0,
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START
                        {
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_ATTACK,
                            ) {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START,
                                    true,
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_HOMING
                        {
                            WorkModule::set_float(
                                boss_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_X,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Y,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Z,
                            );
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_ATTACK,
                            ) {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START,
                                    true,
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING
                        {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == false
                            {
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_SPECIAL,
                                ) == false
                                {
                                    MULTIPLE_BULLETS = 0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_SPECIAL,
                                ) == true
                                {
                                    MULTIPLE_BULLETS = 2;
                                }
                            } else {
                                MULTIPLE_BULLETS = 2;
                            }
                        }

                        if StatusModule::status_kind(boss_boma)
                            != *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU
                            && !DEAD
                        {
                            if StatusModule::status_kind(boss_boma)
                                != *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING
                            {
                                if MULTIPLE_BULLETS != 0 {
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU,
                                        true,
                                    );
                                    MULTIPLE_BULLETS = MULTIPLE_BULLETS - 1;
                                }
                            }
                        }

                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END
                        {
                            if MULTIPLE_BULLETS != 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                            }
                            if MULTIPLE_BULLETS == 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU
                        {
                            if MULTIPLE_BULLETS != 0 {
                                MotionModule::set_rate(boss_boma, 5.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 5.0,
                                );
                            }
                            if MULTIPLE_BULLETS == 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                            }
                        }

                        if CONTROLLABLE {
                            MULTIPLE_BULLETS = 0;
                        }

                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START
                        {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_HOMING
                        {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("teleport_start")
                            && MotionModule::is_end(boss_boma)
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                            MotionModule::change_motion(
                                boss_boma,
                                Hash40::new("teleport_end"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_HOMING
                        {
                            MotionModule::set_rate(boss_boma, 1.25);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.25,
                            );
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::check_button_on(
                                module_accessor,
                                *CONTROL_PAD_BUTTON_SPECIAL,
                            ) {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CANCEL,
                                    true,
                                );
                            }
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CENTER_MOVE
                        {
                            MotionModule::set_rate(boss_boma, 4.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 4.4,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START
                            || MotionModule::motion_kind(boss_boma) == hash40("chakram_start")
                            || MotionModule::motion_kind(boss_boma)
                                == hash40("chakram_start_reverse")
                        {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 1.0,
                            );
                        }
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 2.0
                            && MotionModule::motion_kind(boss_boma) == hash40("chakram_start")
                            && !DEAD
                        {
                            MotionModule::change_motion(
                                boss_boma,
                                Hash40::new("chakram_end"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 2.0
                            && MotionModule::motion_kind(boss_boma)
                                == hash40("chakram_start_reverse")
                            && !DEAD
                        {
                            MotionModule::change_motion(
                                boss_boma,
                                Hash40::new("chakram_end"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 20.0
                            && StatusModule::status_kind(boss_boma)
                                == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START
                            && !DEAD
                        {
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT,
                                true,
                            );
                            MotionModule::change_motion(
                                boss_boma,
                                Hash40::new("chakram_start_reverse"),
                                MotionModule::end_frame(boss_boma) - 19.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }
                        if MotionModule::frame(boss_boma)
                            == MotionModule::end_frame(boss_boma) - 18.0
                            && MotionModule::motion_kind(boss_boma)
                                == hash40("chakram_start_reverse")
                            && !DEAD
                        {
                            ItemModule::remove_item(module_accessor, 0);
                            ItemModule::have_item(
                                module_accessor,
                                ItemKind(*ITEM_KIND_MASTERHANDCHAKRAM),
                                0,
                                0,
                                false,
                                false,
                            );
                            SoundModule::stop_se(
                                module_accessor,
                                smash::phx::Hash40::new("se_item_item_get"),
                                0,
                            );
                            let chakram1_boma = sv_battle_object::module_accessor(
                                ItemModule::get_have_item_id(module_accessor, 0) as u32,
                            );
                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                // left
                                smash::app::lua_bind::PostureModule::set_lr(chakram1_boma, -1.0);
                            }
                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                // right
                                smash::app::lua_bind::PostureModule::set_lr(chakram1_boma, 1.0);
                            }
                            action(chakram1_boma, *ITEM_MASTERHANDCHAKRAM_ACTION_SHOOT3, 0.0);

                            ItemModule::have_item(
                                module_accessor,
                                ItemKind(*ITEM_KIND_MASTERHANDCHAKRAM),
                                0,
                                0,
                                false,
                                false,
                            );
                            SoundModule::stop_se(
                                module_accessor,
                                smash::phx::Hash40::new("se_item_item_get"),
                                0,
                            );
                            let chakram2_boma = sv_battle_object::module_accessor(
                                ItemModule::get_have_item_id(module_accessor, 0) as u32,
                            );
                            let chakram2_pos = Vector3f {
                                x: PostureModule::pos_x(chakram1_boma),
                                y: PostureModule::pos_y(chakram1_boma) - 10.0,
                                z: PostureModule::pos_z(chakram1_boma),
                            };
                            LinkModule::remove_model_constraint(chakram2_boma, true);
                            PostureModule::set_pos(chakram2_boma, &chakram2_pos);
                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                // left
                                smash::app::lua_bind::PostureModule::set_lr(chakram2_boma, -1.0);
                            }
                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                // right
                                smash::app::lua_bind::PostureModule::set_lr(chakram2_boma, 1.0);
                            }
                            SoundModule::play_se(
                                boss_boma,
                                Hash40::new("se_boss_masterhand_chakram_fly"),
                                true,
                                false,
                                false,
                                false,
                                smash::app::enSEType(0),
                            );
                            action(chakram2_boma, *ITEM_MASTERHANDCHAKRAM_ACTION_SHOOT2, 0.0);
                        }
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 2.0
                            && MotionModule::motion_kind(boss_boma) == hash40("chakram_end")
                            && !DEAD
                        {
                            SoundModule::stop_se(
                                boss_boma,
                                smash::phx::Hash40::new("se_boss_masterhand_chakram_fly"),
                                0,
                            );
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT,
                                    true,
                                );
                            } else {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT,
                                    true,
                                );
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                MotionModule::change_motion(
                                    boss_boma,
                                    Hash40::new("wait"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                                CONTROLLABLE = true;
                            }
                        }
                        if MotionModule::motion_kind(boss_boma) == hash40("chakram_end") && !DEAD {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_PRE_MOVE
                            && !DEAD
                        {
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START,
                                true,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CAPTURE
                        {
                            MotionModule::set_rate(boss_boma, 4.75);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 4.75,
                            );
                        }
                        if StatusModule::status_kind(boss_boma)
                            == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU
                        {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma, 2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN
                        {
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER
                                < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER >= 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER
                                > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_X_MASTER <= 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && CONTROLLER_X_MASTER != 0.0
                                && ControlModule::get_stick_x(module_accessor) == 0.0
                            {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0
                                && ControlModule::get_stick_x(module_accessor) < 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0
                                && ControlModule::get_stick_x(module_accessor) > 0.0
                            {
                                CONTROLLER_X_MASTER +=
                                    (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER
                                < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER >= 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER
                                > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                                && CONTROLLER_Y_MASTER <= 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && CONTROLLER_Y_MASTER != 0.0
                                && ControlModule::get_stick_y(module_accessor) == 0.0
                            {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0
                                && ControlModule::get_stick_y(module_accessor) < 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0
                                && ControlModule::get_stick_y(module_accessor) > 0.0
                            {
                                CONTROLLER_Y_MASTER +=
                                    (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f {
                                x: CONTROLLER_X_MASTER * 0.75,
                                y: CONTROLLER_Y_MASTER * 0.75,
                                z: 0.0,
                            };
                            PostureModule::add_pos(boss_boma, &pos);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == false
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma, 1.0,
                                    );
                                    CONTROLLABLE = true;
                                }
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma, 1.0,
                                    );
                                }
                            }
                        }
                        if MotionModule::frame(boss_boma) <= 0.0
                            && MotionModule::motion_kind(boss_boma) == hash40("teleport_end")
                        {
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.5 {
                                let pos = Vector3f {
                                    x: -100.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_x(module_accessor) >= -0.5 {
                                let pos = Vector3f {
                                    x: 100.0,
                                    y: 0.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) <= 0.5 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: -50.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) >= -0.5 {
                                let pos = Vector3f {
                                    x: 0.0,
                                    y: 50.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                    }
                    if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                        && StatusModule::status_kind(boss_boma)
                            != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                        && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN
                        && StatusModule::status_kind(boss_boma)
                            != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                    {
                        if CONTROLLABLE == true {
                            if DEAD == false {
                                let curr_pos = Vector3f {
                                    x: PostureModule::pos_x(module_accessor),
                                    y: PostureModule::pos_y(module_accessor),
                                    z: PostureModule::pos_z(module_accessor),
                                };
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X_MASTER
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_MASTER >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_MASTER +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_MASTER <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_MASTER +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER > 0.0
                                    && CONTROLLER_X_MASTER != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER < 0.0
                                    && CONTROLLER_X_MASTER != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                        CONTROLLER_X_MASTER = 0.0;
                                    }
                                }
                                if CONTROLLER_X_MASTER > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_MASTER +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_MASTER +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_MASTER
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_MASTER >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_MASTER +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_MASTER <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_MASTER +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER > 0.0
                                    && CONTROLLER_Y_MASTER != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER < 0.0
                                    && CONTROLLER_Y_MASTER != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                        CONTROLLER_Y_MASTER = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_MASTER > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_MASTER +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_MASTER +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X_MASTER * 0.75,
                                    y: CONTROLLER_Y_MASTER * 0.75,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);

                                // Boss Moves
                                if PostureModule::lr(boss_boma) == 1.0 {
                                    // right
                                    if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_TURN,
                                            true,
                                        );
                                    }
                                }
                                if PostureModule::lr(boss_boma) == -1.0 {
                                    // left
                                    if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_TURN,
                                            true,
                                        );
                                    }
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_JUMP,
                                ) {
                                    if CRAZY_EXISTS == true
                                        && CRAZY_USABLE
                                        && MASTER_TEAM == CRAZY_TEAM
                                    {
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT
                                        {
                                            CONTROLLABLE = false;
                                            BARK = false;
                                            PUNCH = false;
                                            SHOCK = true;
                                            LASER = false;
                                            SCRATCH_BLOW = false;
                                            CONTROLLER_X_MASTER = 0.0;
                                            CONTROLLER_Y_MASTER = 0.0;
                                            let z = PostureModule::pos_z(boss_boma);
                                            let module_pos = Vector3f {
                                                x: 50.0,
                                                y: 25.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(boss_boma, &module_pos);
                                            StatusModule::change_status_request_from_script(
                                                boss_boma,
                                                *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START,
                                                true,
                                            );
                                        }
                                    }
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_SPECIAL,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_GUARD,
                                ) && MotionModule::motion_kind(boss_boma)
                                    != smash::hash40("teleport_start")
                                    && MotionModule::motion_kind(boss_boma)
                                        != smash::hash40("teleport_end")
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_MASTERHAND_STATUS_KIND_TURN
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma, 1.0,
                                    );
                                    MotionModule::change_motion(
                                        boss_boma,
                                        Hash40::new("teleport_start"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_ATTACK,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    if GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) <= 50.0
                                        && GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) > 0.0
                                    {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CAPTURE,
                                            true,
                                        );
                                    } else {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START,
                                            true,
                                        );
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    if GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) <= 50.0
                                        && GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) > 0.0
                                    {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START,
                                            true,
                                        );
                                    } else {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START,
                                            true,
                                        );
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    if GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) <= 55.0
                                        && GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) > 0.0
                                    {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START,
                                            true,
                                        );
                                    } else {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_UP,
                                            true,
                                        );
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_DRILL_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3
                                    != 0
                                {
                                    Y_POS = PostureModule::pos_y(boss_boma);
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_APPEAL_HI,
                                ) {
                                    if GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) <= 40.0
                                        && GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) > 0.0
                                        && CRAZY_EXISTS
                                        && CRAZY_USABLE
                                        && MASTER_TEAM == CRAZY_TEAM
                                    {
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT
                                        {
                                            CONTROLLABLE = false;
                                            BARK = true;
                                            PUNCH = false;
                                            SHOCK = false;
                                            LASER = false;
                                            SCRATCH_BLOW = false;
                                            CONTROLLER_X_MASTER = 0.0;
                                            CONTROLLER_Y_MASTER = 0.0;
                                            StatusModule::change_status_request_from_script(
                                                boss_boma,
                                                *ITEM_MASTERHAND_STATUS_KIND_BARK,
                                                true,
                                            );
                                        }
                                    }
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_APPEAL_LW,
                                ) {
                                    if GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) <= 50.0
                                        && GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) > 0.0
                                        && CRAZY_EXISTS
                                        && CRAZY_USABLE
                                        && MASTER_TEAM == CRAZY_TEAM
                                    {
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT
                                        {
                                            CONTROLLABLE = false;
                                            BARK = false;
                                            PUNCH = false;
                                            SHOCK = false;
                                            LASER = false;
                                            SCRATCH_BLOW = true;
                                            CONTROLLER_X_MASTER = 0.0;
                                            CONTROLLER_Y_MASTER = 0.0;
                                            StatusModule::change_status_request_from_script(
                                                boss_boma,
                                                *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START,
                                                true,
                                            );
                                        }
                                    } else {
                                        CONTROLLABLE = false;
                                        CONTROLLER_X_MASTER = 0.0;
                                        CONTROLLER_Y_MASTER = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START,
                                            true,
                                        );
                                    }
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_APPEAL_S_L,
                                ) {
                                    let floor_dist = boss_floor_dist(module_accessor, boss_boma);
                                    if floor_dist <= 50.0 && floor_dist > 0.0 {
                                        if let Some(floor_y) =
                                            boss_floor_y(module_accessor, boss_boma)
                                        {
                                            let target_y = floor_y + MASTER_KENZAN_GROUND_CLEARANCE;
                                            println!(
                                                "[PB][MasterHand][Kenzan] trigger floor_dist={:.2} floor_y={:.2} current_y={:.2} target_y={:.2}",
                                                floor_dist,
                                                floor_y,
                                                PostureModule::pos_y(boss_boma),
                                                target_y,
                                            );
                                            let grounded_pos = Vector3f {
                                                x: PostureModule::pos_x(boss_boma),
                                                y: target_y,
                                                z: PostureModule::pos_z(boss_boma),
                                            };
                                            PostureModule::set_pos(boss_boma, &grounded_pos);
                                            PostureModule::set_pos(module_accessor, &grounded_pos);
                                        }
                                        CONTROLLABLE = false;
                                        CONTROLLER_X_MASTER = 0.0;
                                        CONTROLLER_Y_MASTER = 0.0;
                                        MASTER_KENZAN_SPAWNED = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START,
                                            true,
                                        );
                                    }
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_APPEAL_S_R,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START,
                                        true,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

extern "C" fn once_per_fighter_frame_2(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        if crate::should_quarantine_boss_frame(module_accessor) {
            return;
        }
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            ENTRY_ID_2 = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
            if hand_entrance_owns_entry(ENTRY_ID_2, true) {
                return;
            }
            let _runtime_guard = CommonRuntimeSyncGuard::new(
                boss_runtime::slot_ptr(&raw mut boss_runtime::CRAZY_HAND_RUNTIME, ENTRY_ID_2),
                load_crazy_hand_runtime,
                store_crazy_hand_runtime,
            );
            let fighter_manager = boss_helpers::fighter_manager();

            let selected_via_slot =
                selection::is_selected_css_boss(module_accessor, *ITEM_KIND_CRAZYHAND);
            if selected_via_slot {
                boss_helpers::clear_hidden_host_effects(module_accessor);
                let stage_id = smash::app::stage::get_stage_id();
                if boss_helpers::is_boss_preview_stage(stage_id) {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor =
                        smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001
                        || !ItemModule::is_have_item(module_accessor, 0)
                    {
                        ItemModule::remove_all(module_accessor);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma_2 = acquire_crazy_hand_item(module_accessor, ENTRY_ID_2);
                        ModelModule::set_scale(boss_boma_2, 0.08);
                        MotionModule::change_motion(
                            boss_boma_2,
                            Hash40::new("wait"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(
                            module_accessor,
                            Hash40::new("none"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                        ModelModule::set_joint_rotate(
                            module_accessor,
                            Hash40::new("root"),
                            &mut Vector3f {
                                x: -270.0,
                                y: 180.0,
                                z: -90.0,
                            },
                            smash::app::MotionNodeRotateCompose {
                                _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
                            },
                            ModelModule::rotation_order(module_accessor),
                        );
                    }
                } else if !boss_helpers::is_boss_passthrough_stage(stage_id) {
                    restore_crazy_hand_after_item_wipe(module_accessor, fighter_manager);
                    if sv_information::is_ready_go() == false {
                        let entry = boss_helpers::entry_id(module_accessor);
                        let needs_entry_init = !hand_entrance_owns_entry(entry, true)
                            && boss_helpers::needs_hidden_host_entry_init(
                                module_accessor,
                                &raw const BOSS_ID_2,
                                entry,
                            );
                        if needs_entry_init {
                            DEAD_2 = false;
                            CONTROLLABLE_2 = true;
                            // See the Master Hand path above: resetting the
                            // shared runtime per frame cancels a valid pair
                            // entrance and causes request/reset thrashing.
                            reset_crazy_runtime_for_spawn();
                        }
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor =
                            smash::app::sv_system::battle_object_module_accessor(lua_state);
                        let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                        ENTRY_ID_2 = WorkModule::get_int(
                            module_accessor,
                            *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                        ) as usize;
                        let needs_entry_init = !hand_entrance_owns_entry(ENTRY_ID_2, true)
                            && boss_helpers::needs_hidden_host_entry_init(
                                module_accessor,
                                &raw const BOSS_ID_2,
                                ENTRY_ID_2,
                            );
                        if needs_entry_init {
                            EXISTS_PUBLIC_2 = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            CRAZY_EXISTS = true;
                            let boss_boma_2 = acquire_crazy_hand_item(module_accessor, ENTRY_ID_2);
                            initialize_crazy_hand_boss(boss_boma_2, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            let host_pos = Vector3f {
                                x: PostureModule::pos_x(module_accessor),
                                y: PostureModule::pos_y(module_accessor),
                                z: PostureModule::pos_z(module_accessor),
                            };
                            PostureModule::set_pos(boss_boma_2, &host_pos);
                            StatusModule::change_status_request_from_script(
                                boss_boma_2,
                                *ITEM_STATUS_KIND_FOR_BOSS_START,
                                true,
                            );
                        }
                    }

                    if !smash::app::smashball::is_training_mode()
                        && StatusModule::status_kind(module_accessor)
                            == *FIGHTER_STATUS_KIND_REBIRTH
                        && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                        && !STOP_2
                        && !CONFIG.options.boss_respawn.unwrap_or(false)
                    {
                        StatusModule::change_status_request_from_script(
                            module_accessor,
                            *FIGHTER_STATUS_KIND_DEAD,
                            true,
                        );
                    }
                    if !smash::app::smashball::is_training_mode()
                        && StatusModule::status_kind(module_accessor)
                            == *FIGHTER_STATUS_KIND_REBIRTH
                        && StatusModule::status_kind(module_accessor)
                            != *FIGHTER_STATUS_KIND_STANDBY
                        && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                        && STOP_2
                        && !CONFIG.options.boss_respawn.unwrap_or(false)
                    {
                        StatusModule::change_status_request_from_script(
                            module_accessor,
                            *FIGHTER_STATUS_KIND_STANDBY,
                            true,
                        );
                        let x = 0.0;
                        let y = 0.0;
                        let z = 0.0;
                        let module_pos = Vector3f { x: x, y: y, z: z };
                        PostureModule::set_pos(module_accessor, &module_pos);
                    }

                    // Respawn in case of Squad Strike or Specific Circumstances

                    if sv_information::is_ready_go()
                        && !ItemModule::is_have_item(module_accessor, 0)
                        && ModelModule::scale(module_accessor) == 0.0001
                        && StatusModule::status_kind(module_accessor)
                            == *FIGHTER_STATUS_KIND_REBIRTH
                    {
                        if smash::app::smashball::is_training_mode()
                            || CONFIG.options.boss_respawn.unwrap_or(false)
                        {
                            StatusModule::change_status_request_from_script(
                                module_accessor,
                                *FIGHTER_STATUS_KIND_FALL,
                                true,
                            );
                            DEAD_2 = false;
                            CONTROLLABLE_2 = true;
                            reset_crazy_runtime_for_spawn();
                            CRAZY_EXISTS = true;
                            CRAZY_TEAM = TeamModule::team_no(module_accessor);
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor =
                                smash::app::sv_system::battle_object_module_accessor(lua_state);
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                            ENTRY_ID_2 = WorkModule::get_int(
                                module_accessor,
                                *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                            ) as usize;
                            EXISTS_PUBLIC_2 = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            let boss_boma_2 = acquire_crazy_hand_item(module_accessor, ENTRY_ID_2);
                            initialize_crazy_hand_boss(boss_boma_2, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(
                                boss_boma_2,
                                *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
                                true,
                            );

                            let x = PostureModule::pos_x(module_accessor);
                            let y = PostureModule::pos_y(boss_boma_2);
                            let z = PostureModule::pos_z(module_accessor);
                            let module_pos = Vector3f { x: x, y: y, z: z };
                            PostureModule::set_pos(boss_boma_2, &module_pos);

                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                == false
                            {
                                CONTROLLABLE_2 = true;
                            }
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true
                        && BOSS_ID_2[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                        );
                        boss_helpers::ensure_boss_item_visible(boss_boma);
                        let x = PostureModule::pos_x(boss_boma);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(boss_boma);
                        CRAZY_X_POS = x;
                        CRAZY_Y_POS = y;
                        CRAZY_Z_POS = z;
                        if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                            let sub_hp = 999.0
                                - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                        {
                            WorkModule::off_flag(
                                boss_boma,
                                *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK,
                            );
                            WorkModule::off_flag(
                                boss_boma,
                                *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM,
                            );
                            WorkModule::off_flag(
                                boss_boma,
                                *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT,
                            );
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                            // right
                            CRAZY_FACING_RIGHT = true;
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                            // left
                            CRAZY_FACING_RIGHT = false;
                        }
                        JostleModule::set_status(module_accessor, false);
                    }

                    // STUBS AI

                    if sv_information::is_ready_go()
                        && !DEAD_2
                        && BOSS_ID_2[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                            == false
                        {
                            let boss_boma = sv_battle_object::module_accessor(
                                BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                            );
                            if CONTROLLABLE_2 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma, 1.0,
                                );
                                if StatusModule::status_kind(boss_boma)
                                    != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW
                                {
                                    MotionModule::change_motion(
                                        boss_boma,
                                        smash::phx::Hash40::new("wait"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                        true,
                                    );
                                }
                                if StatusModule::status_kind(boss_boma)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                                    && StatusModule::status_kind(boss_boma)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW
                                {
                                    MotionModule::change_motion(
                                        boss_boma,
                                        smash::phx::Hash40::new("wait"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                        true,
                                    );
                                }
                            }
                        }
                    }

                    // Team Attack Trigger
                    if sv_information::is_ready_go() == true
                        && !DEAD_2
                        && !FINDER
                        && !HAND_TEAM_AUTHORITY_ACTIVE
                        && BOSS_ID_2[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma_2 = sv_battle_object::module_accessor(
                            BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                        );
                        let curr_pos = Vector3f {
                            x: PostureModule::pos_x(module_accessor),
                            y: PostureModule::pos_y(module_accessor),
                            z: PostureModule::pos_z(module_accessor),
                        };
                        if MotionModule::motion_kind(boss_boma_2) == hash40("wait")
                            && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                == true
                        {
                            if CONTROLLABLE_2 == false
                                && smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                    == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                || CONTROLLABLE_2
                                    && smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                                        == smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                            {
                                if GroundModule::get_distance_to_floor(
                                    module_accessor,
                                    &curr_pos,
                                    curr_pos.y,
                                    true,
                                ) <= 50.0
                                    && GroundModule::get_distance_to_floor(
                                        module_accessor,
                                        &curr_pos,
                                        curr_pos.y,
                                        true,
                                    ) > 0.0
                                    && MASTER_EXISTS
                                    && MASTER_USABLE
                                    && MASTER_TEAM == CRAZY_TEAM
                                {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT
                                    {
                                        CONTROLLABLE_2 = false;
                                        BARK = false;
                                        PUNCH = false;
                                        SHOCK = false;
                                        LASER = true;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START,
                                            true,
                                        );
                                    }
                                }
                            } else if CONTROLLABLE_2 == false
                                && smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                    == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                || CONTROLLABLE_2
                                    && smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                                        == smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                            {
                                if MASTER_EXISTS
                                    && MASTER_USABLE
                                    && MASTER_TEAM == CRAZY_TEAM
                                    && StatusModule::status_kind(boss_boma_2)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT
                                    {
                                        CONTROLLABLE_2 = false;
                                        BARK = false;
                                        PUNCH = true;
                                        SHOCK = false;
                                        LASER = false;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                            true,
                                        );
                                        MotionModule::change_motion(
                                            boss_boma_2,
                                            Hash40::new("taggoopaa"),
                                            0.0,
                                            1.0,
                                            false,
                                            0.0,
                                            false,
                                            false,
                                        );
                                    }
                                }
                            } else if CONTROLLABLE_2 == false
                                && smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                    == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                                || CONTROLLABLE_2
                                    && smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                                        == smash::app::sv_math::rand(hash40("fighter"), 900) as f32
                            {
                                let floor_dist = boss_floor_dist(module_accessor, boss_boma_2);
                                if floor_dist > 0.0
                                    && floor_dist <= 50.0
                                    && MASTER_EXISTS
                                    && MASTER_USABLE
                                    && MASTER_TEAM == CRAZY_TEAM
                                    && StatusModule::status_kind(boss_boma_2)
                                        != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0
                                        && MASTER_FACING_LEFT
                                        || lua_bind::PostureModule::lr(boss_boma_2) == -1.0
                                            && !MASTER_FACING_LEFT
                                    {
                                        let finder_started =
                                            start_finder_pair(fighter.lua_state_agent, boss_boma_2);
                                        if finder_started {
                                            let master_boma = current_master_boma();
                                            crate::boss_log!(
                                                "[PB][Finder] cpu_trigger started=true floor={:.1} crazy_status={} crazy_motion=0x{:x} master_status={} master_motion=0x{:x}",
                                                floor_dist,
                                                StatusModule::status_kind(boss_boma_2),
                                                MotionModule::motion_kind(boss_boma_2),
                                                if master_boma.is_null() { -1 } else { StatusModule::status_kind(master_boma) },
                                                if master_boma.is_null() { 0 } else { MotionModule::motion_kind(master_boma) }
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD_2 == true && BOSS_ID_2[boss_helpers::entry_id(module_accessor)] != 0
                        {
                            let boss_boma_2 = sv_battle_object::module_accessor(
                                BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                            );
                            if STOP_2 == false
                                && CONFIG.options.boss_respawn.unwrap_or(false)
                                && StatusModule::status_kind(module_accessor)
                                    != *FIGHTER_STATUS_KIND_STANDBY
                            {
                                StatusModule::change_status_request_from_script(
                                    module_accessor,
                                    *FIGHTER_STATUS_KIND_STANDBY,
                                    true,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2) != *ITEM_STATUS_KIND_DEAD
                                || StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_DEAD
                                    && MotionModule::frame(boss_boma_2) > 250.0
                            {
                                HitModule::set_whole(
                                    module_accessor,
                                    smash::app::HitStatus(*HIT_STATUS_OFF),
                                    0,
                                );
                                HitModule::set_whole(
                                    boss_boma_2,
                                    smash::app::HitStatus(*HIT_STATUS_OFF),
                                    0,
                                );
                                ItemModule::remove_all(module_accessor);
                                if STOP_2 == false && CONFIG.options.boss_respawn.unwrap_or(false) {
                                    StatusModule::change_status_request_from_script(
                                        module_accessor,
                                        *FIGHTER_STATUS_KIND_DEAD,
                                        true,
                                    );
                                    STOP_2 = true;
                                }
                                if STOP_2 == false && !CONFIG.options.boss_respawn.unwrap_or(false)
                                {
                                    boss_helpers::request_hidden_host_stock_drain(
                                        module_accessor,
                                        fighter_manager,
                                        ENTRY_ID_2,
                                        &raw mut STOP_2,
                                    );
                                }
                            }
                        }
                    }

                    if DEAD_2 == true {
                        if sv_information::is_ready_go() == true
                            && BOSS_ID_2[boss_helpers::entry_id(module_accessor)] != 0
                        {
                            let boss_boma_2 = sv_battle_object::module_accessor(
                                BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                            );
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma_2)
                                    != *ITEM_STATUS_KIND_STANDBY
                                {
                                    CRAZY_EXISTS = false;
                                    if lua_bind::PostureModule::lr(boss_boma_2) == -1.0 {
                                        // left
                                        let vec3 = Vector3f {
                                            x: 0.0,
                                            y: 180.0,
                                            z: 0.0,
                                        };
                                        PostureModule::set_rot(boss_boma_2, &vec3, 0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 {
                                        // right
                                        let vec3 = Vector3f {
                                            x: 0.0,
                                            y: 0.0,
                                            z: 0.0,
                                        };
                                        PostureModule::set_rot(boss_boma_2, &vec3, 0);
                                    }
                                    if MotionModule::frame(boss_boma_2) == 0.0 {
                                        smash_script::macros::CAM_ZOOM_IN_arg5(
                                            fighter, 0.0, 0.0, 5.0, 0.0, 0.0,
                                        );
                                        smash_script::macros::EFFECT_OFF_KIND(
                                            fighter,
                                            Hash40::new("sys_dead"),
                                            true,
                                            false,
                                        );
                                        smash_script::macros::EFFECT(
                                            fighter,
                                            Hash40::new("sys_bg_criticalhit"),
                                            Hash40::new("top"),
                                            0,
                                            7,
                                            0,
                                            0,
                                            0,
                                            0,
                                            1,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            false,
                                        );
                                        smash_script::macros::EFFECT(
                                            fighter,
                                            Hash40::new("sys_bg_boss_finishhit"),
                                            Hash40::new("top"),
                                            0,
                                            7,
                                            0,
                                            0,
                                            0,
                                            0,
                                            1,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            0,
                                            false,
                                        );
                                    }
                                    if MotionModule::frame(boss_boma_2) == 0.5 {
                                        SlowModule::set_whole(module_accessor, 100, 0);
                                    }
                                    if MotionModule::frame(boss_boma_2) == 1.0 {
                                        SlowModule::clear_whole(module_accessor);
                                        SlowModule::set_whole(module_accessor, 10, 0);
                                    }
                                    if MotionModule::frame(boss_boma_2) >= 1.1 {
                                        CameraModule::reset_all(module_accessor);
                                    }
                                    if MotionModule::frame(boss_boma_2) >= 5.0 {
                                        CameraModule::reset_all(module_accessor);
                                        smash_script::macros::CAM_ZOOM_OUT(fighter);
                                        smash_script::macros::EFFECT_OFF_KIND(
                                            fighter,
                                            Hash40::new("sys_bg_criticalhit"),
                                            true,
                                            false,
                                        );
                                        smash_script::macros::EFFECT_OFF_KIND(
                                            fighter,
                                            Hash40::new("sys_bg_boss_finishhit"),
                                            true,
                                            false,
                                        );
                                        SlowModule::clear_whole(module_accessor);
                                    }
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        EXISTS_PUBLIC = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_STATUS_KIND_STANDBY,
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY
                        {
                            FighterManager::set_cursor_whole(fighter_manager, false);
                            ArticleModule::set_visibility_whole(
                                module_accessor,
                                *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP,
                                false,
                                smash::app::ArticleOperationTarget(0),
                            );
                            StatusModule::change_status_request_from_script(
                                module_accessor,
                                *FIGHTER_STATUS_KIND_WAIT,
                                true,
                            );
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001
                        && !DEAD_2
                        && BOSS_ID_2[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma_2 = sv_battle_object::module_accessor(
                            BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                        );
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY
                            && !MASTER_EXISTS
                        {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                2.0,
                            );
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY
                            && MASTER_EXISTS
                        {
                            CONTROLLABLE_2 = true;
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                2.0,
                            );
                            CRAZY_TEAM = TeamModule::team_no(module_accessor);
                            if MASTER_TEAM == CRAZY_TEAM {
                                StatusModule::change_status_request_from_script(
                                    boss_boma_2,
                                    *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    smash::phx::Hash40::new("entry2"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                            }
                        }
                        if MotionModule::motion_kind(boss_boma_2) == smash::hash40("entry2") {
                            MotionModule::set_rate(boss_boma_2, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                1.5,
                            );
                        }
                    }

                    //DAMAGE MODULES

                    if BOSS_ID_2[boss_helpers::entry_id(module_accessor)] == 0 {
                        return;
                    }
                    let boss_boma_2 = sv_battle_object::module_accessor(
                        BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                    );
                    HitModule::set_whole(
                        module_accessor,
                        smash::app::HitStatus(*HIT_STATUS_OFF),
                        0,
                    );
                    HitModule::set_whole(boss_boma_2, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma_2, i, false) {
                            AttackModule::set_target_category(
                                boss_boma_2,
                                i,
                                *COLLISION_CATEGORY_MASK_ALL as u32,
                            );
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if FighterUtil::is_hp_mode(module_accessor) == false {
                            let hp = CONFIG.options.crazy_hand_hp.unwrap_or(400.0);
                            if DamageModule::damage(module_accessor, 0) >= hp {
                                // HEALTH
                                if DEAD_2 == false {
                                    CONTROLLABLE_2 = false;
                                    DEAD_2 = true;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_STATUS_KIND_DEAD,
                                        true,
                                    );
                                    if FINDER {
                                        clear_finder_runtime_with_reason("crazy_death");
                                    }
                                }
                            }
                        }
                    }

                    // SET FIGHTER LOOP

                    if sv_information::is_ready_go() == true {
                        if StatusModule::status_kind(module_accessor)
                            != *FIGHTER_STATUS_KIND_STANDBY
                        {
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ITEM,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_SPECIAL,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_JUMP_AERIAL,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_TREAD_JUMP,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ITEM_THROW,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ATTACK,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_WALL_JUMP,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ESCAPE,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_CATCH,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_JUMP,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_GUARD,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ATTACK,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ESCAPE,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_CLIFF,
                            );
                            WorkModule::enable_transition_term_forbid_group(
                                module_accessor,
                                *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LASSO,
                            );
                            FighterManager::set_cursor_whole(fighter_manager, false);
                            fighter.set_situation(SITUATION_KIND_AIR.into());
                            GroundModule::set_correct(
                                module_accessor,
                                smash::app::GroundCorrectKind(*GROUND_CORRECT_KIND_AIR),
                            );
                            MotionModule::change_motion(
                                module_accessor,
                                Hash40::new("fall"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }
                    }

                    if DEAD_2 == false {
                        // SET POS AND STOPS OUT OF BOUNDS
                        if !FINDER
                            && ModelModule::scale(module_accessor) == 0.0001
                            && BOSS_ID_2[boss_helpers::entry_id(module_accessor)] != 0
                        {
                            let boss_boma = sv_battle_object::module_accessor(
                                BOSS_ID_2[boss_helpers::entry_id(module_accessor)],
                            );
                            if FighterUtil::is_hp_mode(module_accessor) == true {
                                if StatusModule::status_kind(module_accessor)
                                    == *FIGHTER_STATUS_KIND_DEAD
                                    || StatusModule::status_kind(module_accessor) == 79
                                {
                                    if DEAD_2 == false {
                                        CONTROLLABLE_2 = false;
                                        DEAD_2 = true;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_STATUS_KIND_DEAD,
                                            true,
                                        );
                                    }
                                }
                            }
                            let x = PostureModule::pos_x(boss_boma);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(boss_boma);
                            let boss_pos = Vector3f {
                                x: x,
                                y: y + 20.0,
                                z: z,
                            };
                            if !CONTROLLABLE_2
                                || boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                    == true
                            {
                                if PostureModule::pos_y(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0
                                {
                                    let boss_y_pos_2 = Vector3f {
                                        x: x,
                                        y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                {
                                    let boss_x_pos_1 = Vector3f {
                                        x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0
                                {
                                    let boss_x_pos_2 = Vector3f {
                                        x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                } else if PostureModule::pos_y(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                {
                                    let boss_y_pos_1 = Vector3f {
                                        x: x,
                                        y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: x,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                } else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            } else {
                                if PostureModule::pos_y(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0
                                {
                                    let boss_y_pos_2 = Vector3f {
                                        x: x,
                                        y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                {
                                    let boss_x_pos_1 = Vector3f {
                                        x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                } else if PostureModule::pos_x(boss_boma)
                                    <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0
                                {
                                    let boss_x_pos_2 = Vector3f {
                                        x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0,
                                        y: y,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                } else if PostureModule::pos_y(boss_boma)
                                    >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                {
                                    let boss_y_pos_1 = Vector3f {
                                        x: x,
                                        y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                        z: z,
                                    };
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: x,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                } else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                            let crazy_floor_clearance = if StatusModule::status_kind(boss_boma)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU
                            {
                                CRAZY_NOTAUTSU_GROUND_CLEARANCE
                            } else if StatusModule::status_kind(boss_boma)
                                == *ITEM_CRAZYHAND_STATUS_KIND_KUMO
                            {
                                CRAZY_KUMO_GROUND_CLEARANCE
                            } else {
                                CRAZY_FLOAT_FLOOR_CLEARANCE
                            };
                            if crazy_should_clamp_floor(boss_boma) {
                                boss_helpers::clamp_flying_boss_floor(
                                    module_accessor,
                                    boss_boma,
                                    crazy_floor_clearance,
                                );
                            }
                        }
                    }

                    sync_hand_team_authority_from_flags(boss_boma_2, ENTRY_ID_2);
                    let hand_team_active_2 = hand_team_authority_active_for_boma(boss_boma_2);
                    if StatusModule::status_kind(boss_boma_2)
                        == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE
                        || StatusModule::status_kind(boss_boma_2)
                            == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT
                        || StatusModule::status_kind(boss_boma_2)
                            == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME
                        || StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT
                        || StatusModule::status_kind(boss_boma_2)
                            == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                        || CONTROLLABLE_2
                    {
                        CRAZY_USABLE = true;
                    } else {
                        CRAZY_USABLE = false;
                    }
                    if !FINDER
                        && !hand_team_active_2
                        && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true
                        && StatusModule::status_kind(boss_boma_2)
                            == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                    {
                        StatusModule::change_status_request_from_script(
                            boss_boma_2,
                            *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
                            true,
                        );
                    }
                    if !FINDER
                        && !hand_team_active_2
                        && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true
                    {
                        maybe_recover_crazy_cpu_idle(boss_boma_2, ENTRY_ID_2);
                    }
                    update_finder_runtime(fighter.lua_state_agent);
                    log_hand_team_status();
                    maybe_finish_hand_team_authority("native_pair_complete");

                    if !FINDER {
                        if BARK
                            && !DEAD_2
                            && MASTER_EXISTS
                            && MotionModule::motion_kind(boss_boma_2) != smash::hash40("bark")
                            && CRAZY_USABLE
                        {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                1.0,
                            );
                            CONTROLLABLE_2 = false;
                            StatusModule::change_status_request_from_script(
                                boss_boma_2,
                                *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                true,
                            );
                            MotionModule::change_motion(
                                boss_boma_2,
                                Hash40::new("bark"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }

                        if MotionModule::motion_kind(boss_boma_2) == hash40("bark") {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                1.0,
                            );
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 {
                                // right
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS + 30.0,
                                    y: MASTER_Y_POS,
                                    z: MASTER_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma_2, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 {
                                // left
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS - 30.0,
                                    y: MASTER_Y_POS,
                                    z: MASTER_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma_2, &master_pos);
                            }
                        }

                        if SCRATCH_BLOW
                            && !DEAD_2
                            && MASTER_EXISTS
                            && MotionModule::motion_kind(boss_boma_2) != smash::hash40("bark")
                            && CRAZY_USABLE
                        {
                            CONTROLLABLE_2 = false;
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                1.2,
                            );
                            StatusModule::change_status_request_from_script(
                                boss_boma_2,
                                *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START,
                                true,
                            );
                        }

                        if MotionModule::motion_kind(boss_boma_2)
                            == smash::hash40("electroshock_start")
                            || MotionModule::motion_kind(boss_boma_2)
                                == smash::hash40("electroshock")
                            || MotionModule::motion_kind(boss_boma_2)
                                == smash::hash40("electroshock_end")
                        {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                1.0,
                            );
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 {
                                // right
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS + 100.0,
                                    y: MASTER_Y_POS,
                                    z: MASTER_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma_2, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 {
                                // left
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS - 100.0,
                                    y: MASTER_Y_POS,
                                    z: MASTER_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma_2, &master_pos);
                            }
                        }

                        if StatusModule::status_kind(boss_boma_2)
                            == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START
                        {
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 {
                                // right
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS - 200.0,
                                    y: MASTER_Y_POS,
                                    z: MASTER_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma_2, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 {
                                // left
                                let master_pos = Vector3f {
                                    x: MASTER_X_POS + 200.0,
                                    y: MASTER_Y_POS,
                                    z: MASTER_Z_POS,
                                };
                                PostureModule::set_pos(boss_boma_2, &master_pos);
                            }
                        }

                        if MotionModule::is_end(boss_boma_2)
                            && MotionModule::motion_kind(boss_boma_2) == hash40("teleport_end")
                            && !DEAD_2
                        {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                == true
                            {
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    Hash40::new("wait"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                                StatusModule::change_status_request_from_script(
                                    boss_boma_2,
                                    *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
                                    true,
                                );
                            } else {
                                StatusModule::change_status_request_from_script(
                                    boss_boma_2,
                                    *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    Hash40::new("wait"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                                CONTROLLABLE_2 = true;
                            }
                        }

                        if MotionModule::frame(boss_boma_2)
                            >= MotionModule::end_frame(boss_boma_2) - 10.0
                            && MotionModule::motion_kind(boss_boma_2) == hash40("bark")
                            && !DEAD_2
                        {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                1.0,
                            );
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                == true
                            {
                                BARK = false;
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    Hash40::new("wait"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                                StatusModule::change_status_request_from_script(
                                    boss_boma_2,
                                    *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
                                    true,
                                );
                            } else {
                                BARK = false;
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(
                                    boss_boma_2,
                                    *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    Hash40::new("wait"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                            }
                        }

                        if sv_information::is_ready_go() == true {
                            if SHOCK
                                && !DEAD_2
                                && MASTER_EXISTS
                                && MotionModule::motion_kind(boss_boma_2)
                                    != smash::hash40("electroshock_start")
                                && MotionModule::motion_kind(boss_boma_2)
                                    != smash::hash40("electroshock")
                                && MotionModule::motion_kind(boss_boma_2)
                                    != smash::hash40("electroshock_end")
                                && CRAZY_USABLE
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                                CONTROLLABLE_2 = false;
                                let z = PostureModule::pos_z(boss_boma_2);
                                let module_pos = Vector3f {
                                    x: 50.0,
                                    y: 25.0,
                                    z: z,
                                };
                                PostureModule::set_pos(boss_boma_2, &module_pos);
                                StatusModule::change_status_request_from_script(
                                    boss_boma_2,
                                    *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                    true,
                                );
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    Hash40::new("electroshock_start"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                            }

                            if MotionModule::is_end(boss_boma_2)
                                && MotionModule::motion_kind(boss_boma_2)
                                    == hash40("electroshock_start")
                            {
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    Hash40::new("electroshock"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                            }

                            if MotionModule::is_end(boss_boma_2)
                                && MotionModule::motion_kind(boss_boma_2) == hash40("electroshock")
                            {
                                MotionModule::change_motion(
                                    boss_boma_2,
                                    Hash40::new("electroshock_end"),
                                    0.0,
                                    1.0,
                                    false,
                                    0.0,
                                    false,
                                    false,
                                );
                            }

                            if MotionModule::is_end(boss_boma_2)
                                && MotionModule::motion_kind(boss_boma_2)
                                    == hash40("electroshock_end")
                                && !DEAD_2
                            {
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == false
                                {
                                    CONTROLLABLE_2 = true;
                                    SHOCK = false;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                        true,
                                    );
                                    MotionModule::change_motion(
                                        boss_boma_2,
                                        Hash40::new("wait"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                } else {
                                    SHOCK = false;
                                    MotionModule::change_motion(
                                        boss_boma_2,
                                        Hash40::new("wait"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
                                        true,
                                    );
                                }
                            }
                        }

                        // FIXES SPAWN

                        if DEAD_2 == false {
                            if sv_information::is_ready_go() == true {
                                if JUMP_START_2 == false {
                                    JUMP_START_2 = true;
                                    if boss_helpers::is_operation_cpu_entry(
                                        fighter_manager,
                                        ENTRY_ID_2,
                                    ) == true
                                    {
                                        CONTROLLABLE_2 = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
                                            true,
                                        );
                                    } else {
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME,
                                            true,
                                        );
                                    }
                                }
                            }
                        }

                        if sv_information::is_ready_go() == true && !DEAD_2 {
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                                let stunned = !CONFIG.options.full_stun_duration.unwrap_or(false);
                                if stunned {
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END,
                                        true,
                                    );
                                }
                                CONTROLLABLE_2 = false;
                            }
                            if CONTROLLABLE_2
                                && StatusModule::status_kind(boss_boma_2)
                                    != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM
                            {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    2.0,
                                );
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY * 0.75,
                                    y: CONTROLLER_Y_CRAZY * 0.75,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START
                            {
                                MotionModule::set_rate(boss_boma_2, 2.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    2.2,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END
                            {
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    2.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                            {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.2,
                                );
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                == false
                            {
                                if StatusModule::status_kind(boss_boma_2)
                                    != *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START
                                {
                                    reset_crazy_fire_chariot_latches(ENTRY_ID_2);
                                } else {
                                    if MotionModule::frame(boss_boma_2) >= 40.0
                                        && !CRAZY_FIRE_CHARIOT_PINKY_LATCH[ENTRY_ID_2]
                                    {
                                        WorkModule::set_flag(
                                            boss_boma_2,
                                            true,
                                            *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_PINKY,
                                        );
                                        CRAZY_FIRE_CHARIOT_PINKY_LATCH[ENTRY_ID_2] = true;
                                    }
                                    if MotionModule::frame(boss_boma_2) >= 55.0
                                        && !CRAZY_FIRE_CHARIOT_THUMB_LATCH[ENTRY_ID_2]
                                    {
                                        WorkModule::set_flag(
                                            boss_boma_2,
                                            true,
                                            *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_THUMB,
                                        );
                                        CRAZY_FIRE_CHARIOT_THUMB_LATCH[ENTRY_ID_2] = true;
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == 117 {
                                if MotionModule::frame(boss_boma_2)
                                    == MotionModule::end_frame(boss_boma_2) - 2.0
                                {
                                    PostureModule::set_pos(
                                        boss_boma_2,
                                        &Vector3f {
                                            x: 0.0,
                                            y: 20.0,
                                            z: 0.0,
                                        },
                                    );
                                    lua_bind::PostureModule::set_lr(boss_boma_2, 1.0);
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START,
                                        true,
                                    );
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_LOOP
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW
                            {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.2,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START
                            {
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY * 2.0,
                                    y: CONTROLLER_Y_CRAZY * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP
                            {
                                WorkModule::set_float(
                                    boss_boma_2,
                                    0.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_X,
                                );
                                WorkModule::set_float(
                                    boss_boma_2,
                                    0.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Y,
                                );
                                WorkModule::set_float(
                                    boss_boma_2,
                                    0.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Z,
                                );
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY * 2.0,
                                    y: CONTROLLER_Y_CRAZY * 2.0,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START
                            {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START
                            {
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY,
                                    y: CONTROLLER_Y_CRAZY,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP
                            {
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY,
                                    y: CONTROLLER_Y_CRAZY,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END
                            {
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY,
                                    y: CONTROLLER_Y_CRAZY,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START
                            {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.2,
                                );
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY,
                                    y: CONTROLLER_Y_CRAZY,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK
                            {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.2,
                                );
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY * 0.5,
                                    y: CONTROLLER_Y_CRAZY * 0.5,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                || StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                                || StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                                || StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LANDING
                            {
                                CONTROLLABLE_2 = false;
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DIG_START
                            {
                                MotionModule::set_rate(boss_boma_2, 1.175);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.175,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP
                            {
                                MotionModule::set_rate(boss_boma_2, 1.7);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.7,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE
                            {
                                MotionModule::set_rate(boss_boma_2, 4.75);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    4.75,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START
                            {
                                MotionModule::set_rate(boss_boma_2, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.4,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD
                            {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.2,
                                );
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                    == false
                                {
                                    PostureModule::set_pos(
                                        boss_boma_2,
                                        &Vector3f {
                                            x: PostureModule::pos_x(boss_boma_2),
                                            y: Y_POS_2,
                                            z: PostureModule::pos_z(boss_boma_2),
                                        },
                                    );
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU
                                || StatusModule::status_kind(boss_boma_2) == 84
                                || StatusModule::status_kind(boss_boma_2) == 85
                            {
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                    == false
                                {
                                    PostureModule::set_pos(
                                        boss_boma_2,
                                        &Vector3f {
                                            x: PostureModule::pos_x(boss_boma_2),
                                            y: Y_POS_2,
                                            z: PostureModule::pos_z(boss_boma_2),
                                        },
                                    );
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI
                            {
                                MotionModule::set_rate(boss_boma_2, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.4,
                                );
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f {
                                        x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                        y: 0.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f {
                                        x: ControlModule::get_stick_x(module_accessor) * 2.0,
                                        y: 0.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f {
                                        x: 0.0,
                                        y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f {
                                        x: 0.0,
                                        y: ControlModule::get_stick_y(module_accessor) * 2.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_HOMING
                            {
                                MotionModule::set_rate(boss_boma_2, 1.25);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.25,
                                );
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X_CRAZY <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && CONTROLLER_X_CRAZY != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X_CRAZY +=
                                        (ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y_CRAZY <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && CONTROLLER_Y_CRAZY != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y_CRAZY +=
                                        (ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL)
                                            * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f {
                                    x: CONTROLLER_X_CRAZY,
                                    y: CONTROLLER_Y_CRAZY,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CENTER_MOVE
                            {
                                MotionModule::set_rate(boss_boma_2, 4.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    4.4,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU
                            {
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    2.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END
                            {
                                MotionModule::set_rate(boss_boma_2, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.4,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP
                            {
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_ATTACK,
                                ) {
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END,
                                        true,
                                    );
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP
                            {
                                StatusModule::change_status_request_from_script(
                                    boss_boma_2,
                                    *ITEM_CRAZYHAND_STATUS_KIND_DIG_END,
                                    true,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_ATTACK
                            {
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_ATTACK,
                                ) {
                                    MotionModule::set_rate(boss_boma_2, 4.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma_2,
                                        4.0,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_ATTACK,
                                ) == false
                                {
                                    MotionModule::set_rate(boss_boma_2, 2.2);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma_2,
                                        2.2,
                                    );
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END
                            {
                                MotionModule::set_rate(boss_boma_2, 1.5);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.5,
                                );
                            }
                            if MotionModule::frame(boss_boma_2)
                                >= MotionModule::end_frame(boss_boma_2)
                            {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.0,
                                );
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START
                            {
                                if MotionModule::frame(boss_boma_2)
                                    >= MotionModule::end_frame(boss_boma_2) - 10.0
                                {
                                    LASER = false;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW
                            {
                                if MotionModule::frame(boss_boma_2)
                                    >= MotionModule::end_frame(boss_boma_2) - 10.0
                                {
                                    SCRATCH_BLOW = false;
                                }
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                                == false
                            {
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
                                {
                                    CONTROLLABLE_2 = false;
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CANCEL
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT
                                {
                                    CONTROLLABLE_2 = !FINDER;
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME
                                {
                                    CONTROLLABLE_2 = true;
                                }
                                if MotionModule::motion_kind(boss_boma_2) == smash::hash40("wait") {
                                    CONTROLLABLE_2 = true;
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_RND_WAIT
                                {
                                    CONTROLLABLE_2 = true;
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE
                                {
                                    CONTROLLABLE_2 = true;
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TO_POINT
                                {
                                    CONTROLLABLE_2 = true;
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT
                                {
                                    CONTROLLABLE_2 = true;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT,
                                        true,
                                    );
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT
                                {
                                    CONTROLLABLE_2 = true;
                                }
                                if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT
                                {
                                    CONTROLLABLE_2 = true;
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_END
                                {
                                    MotionModule::set_rate(boss_boma_2, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma_2,
                                        1.0,
                                    );
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_KUMO
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if MotionModule::motion_kind(boss_boma_2)
                                    == smash::hash40("teleport_start")
                                    && MotionModule::is_end(boss_boma_2)
                                {
                                    MotionModule::set_rate(boss_boma_2, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma_2,
                                        1.0,
                                    );
                                    MotionModule::change_motion(
                                        boss_boma_2,
                                        Hash40::new("teleport_end"),
                                        0.0,
                                        1.0,
                                        false,
                                        0.0,
                                        false,
                                        false,
                                    );
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                {
                                    // Boss Control Movement
                                    // X Controllable
                                    if CONTROLLER_X_CRAZY
                                        < ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_X_CRAZY >= 0.0
                                        && ControlModule::get_stick_x(module_accessor) > 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY
                                        > ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_X_CRAZY <= 0.0
                                        && ControlModule::get_stick_x(module_accessor) < 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY > 0.0
                                        && CONTROLLER_X_CRAZY != 0.0
                                        && ControlModule::get_stick_x(module_accessor) == 0.0
                                    {
                                        CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0
                                        && CONTROLLER_X_CRAZY != 0.0
                                        && ControlModule::get_stick_x(module_accessor) == 0.0
                                    {
                                        CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                    }
                                    if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                        if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                            CONTROLLER_X_CRAZY = 0.0;
                                        }
                                    }
                                    if CONTROLLER_X_CRAZY > 0.0
                                        && ControlModule::get_stick_x(module_accessor) < 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0
                                        && ControlModule::get_stick_x(module_accessor) > 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }

                                    // Y Controllable
                                    if CONTROLLER_Y_CRAZY
                                        < ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_Y_CRAZY >= 0.0
                                        && ControlModule::get_stick_y(module_accessor) > 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY
                                        > ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_Y_CRAZY <= 0.0
                                        && ControlModule::get_stick_y(module_accessor) < 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY > 0.0
                                        && CONTROLLER_Y_CRAZY != 0.0
                                        && ControlModule::get_stick_y(module_accessor) == 0.0
                                    {
                                        CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0
                                        && CONTROLLER_Y_CRAZY != 0.0
                                        && ControlModule::get_stick_y(module_accessor) == 0.0
                                    {
                                        CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                    }
                                    if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                        if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                            CONTROLLER_Y_CRAZY = 0.0;
                                        }
                                    }
                                    if CONTROLLER_Y_CRAZY > 0.0
                                        && ControlModule::get_stick_y(module_accessor) < 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0
                                        && ControlModule::get_stick_y(module_accessor) > 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    let pos = Vector3f {
                                        x: CONTROLLER_X_CRAZY,
                                        y: CONTROLLER_Y_CRAZY,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                    if boss_helpers::is_operation_cpu_entry(
                                        fighter_manager,
                                        ENTRY_ID_2,
                                    ) == false
                                    {
                                        if MotionModule::frame(boss_boma_2)
                                            >= MotionModule::end_frame(boss_boma_2) - 10.0
                                        {
                                            MotionModule::set_rate(boss_boma_2, 1.0);
                                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                            CONTROLLABLE_2 = true;
                                        }
                                    }
                                    if boss_helpers::is_operation_cpu_entry(
                                        fighter_manager,
                                        ENTRY_ID_2,
                                    ) == true
                                    {
                                        if MotionModule::frame(boss_boma_2)
                                            >= MotionModule::end_frame(boss_boma_2) - 10.0
                                        {
                                            MotionModule::set_rate(boss_boma_2, 1.0);
                                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                        }
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU
                                {
                                    if MotionModule::frame(boss_boma_2)
                                        >= MotionModule::end_frame(boss_boma_2) - 10.0
                                    {
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                {
                                    CONTROLLABLE_2 = false;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_KUMO_PRE_MOVE
                            {
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    2.0,
                                );
                            }

                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU
                            {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                    boss_boma_2,
                                    1.2,
                                );
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f {
                                        x: ControlModule::get_stick_x(module_accessor) * 0.75,
                                        y: 0.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f {
                                        x: ControlModule::get_stick_x(module_accessor) * 0.75,
                                        y: 0.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                                if boss_floor_y(module_accessor, boss_boma_2).is_none() {
                                    StatusModule::change_status_request_from_script(
                                        boss_boma_2,
                                        *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME,
                                        true,
                                    );
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2)
                                == *ITEM_CRAZYHAND_STATUS_KIND_KUMO
                            {
                                CONTROLLABLE_2 = false;
                                if !CRAZY_KUMO_ACTIVE {
                                    CRAZY_KUMO_ACTIVE = true;
                                    CRAZY_KUMO_ENDING = false;
                                    CRAZY_KUMO_START_Y = PostureModule::pos_y(boss_boma_2);
                                }
                                if let Some(floor_y) = boss_floor_y(module_accessor, boss_boma_2) {
                                    if !CRAZY_KUMO_ENDING {
                                        let current_y = PostureModule::pos_y(boss_boma_2);
                                        let target_y = CRAZY_KUMO_START_Y + CRAZY_KUMO_ASCENT;
                                        let next_y = if MotionModule::frame(boss_boma_2)
                                            < CRAZY_KUMO_DESCEND_FRAME
                                        {
                                            (current_y + 6.0).min(target_y)
                                        } else {
                                            let grounded_y = floor_y + CRAZY_KUMO_GROUND_CLEARANCE;
                                            (current_y - 6.0).max(grounded_y)
                                        };
                                        PostureModule::set_pos(
                                            boss_boma_2,
                                            &Vector3f {
                                                x: PostureModule::pos_x(boss_boma_2),
                                                y: next_y,
                                                z: PostureModule::pos_z(boss_boma_2),
                                            },
                                        );
                                    }
                                } else {
                                    if !CRAZY_KUMO_ENDING {
                                        let tail_start_frame =
                                            (MotionModule::end_frame(boss_boma_2)
                                                - CRAZY_KUMO_END_TAIL_FRAMES)
                                                .max(MotionModule::frame(boss_boma_2));
                                        MotionModule::set_frame(
                                            boss_boma_2,
                                            tail_start_frame,
                                            false,
                                        );
                                        MotionModule::set_rate(boss_boma_2, 1.0);
                                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                        boss_boma_2,
                                        1.0,
                                    );
                                        CRAZY_KUMO_ENDING = true;
                                        println!(
                                        "[PB][CrazyHand][Kumo] graceful offstage end tail_start={:.2}",
                                        tail_start_frame,
                                    );
                                    }
                                    if MotionModule::is_end(boss_boma_2) {
                                        CRAZY_KUMO_ACTIVE = false;
                                        CRAZY_KUMO_ENDING = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME,
                                            true,
                                        );
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                            } else {
                                CRAZY_KUMO_ACTIVE = false;
                                CRAZY_KUMO_ENDING = false;
                            }
                            if CONTROLLABLE_2
                                && StatusModule::status_kind(boss_boma_2)
                                    == *ITEM_CRAZYHAND_STATUS_KIND_TURN
                            {
                                MotionModule::set_rate(boss_boma_2, 1.4);
                            }
                            if MotionModule::frame(boss_boma_2) <= 0.0
                                && MotionModule::motion_kind(boss_boma_2) == hash40("teleport_end")
                            {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.5 {
                                    let pos = Vector3f {
                                        x: -100.0,
                                        y: 0.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                                if ControlModule::get_stick_x(module_accessor) >= -0.5 {
                                    let pos = Vector3f {
                                        x: 100.0,
                                        y: 0.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                                if ControlModule::get_stick_y(module_accessor) <= 0.5 {
                                    let pos = Vector3f {
                                        x: 0.0,
                                        y: -50.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                                if ControlModule::get_stick_y(module_accessor) >= -0.5 {
                                    let pos = Vector3f {
                                        x: 0.0,
                                        y: 50.0,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                        }
                        if MotionModule::motion_kind(boss_boma_2) == smash::hash40("taggoopaa") {
                            MotionModule::set_rate(boss_boma_2, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(
                                boss_boma_2,
                                1.3,
                            );
                            let x = PostureModule::pos_x(boss_boma_2);
                            CONTROLLABLE_2 = false;
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 {
                                // right
                                if MotionModule::frame(boss_boma_2) >= 120.0 {
                                    if MotionModule::frame(boss_boma_2) <= 140.0 {
                                        let pos = Vector3f {
                                            x: -0.5,
                                            y: 0.0,
                                            z: 0.0,
                                        };
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                }
                                if MotionModule::frame(boss_boma_2) >= 130.0 {
                                    if MotionModule::frame(boss_boma_2) <= 140.0 {
                                        if x < MASTER_X_POS - 25.0 {
                                            let pos = Vector3f {
                                                x: 14.75,
                                                y: 0.0,
                                                z: 0.0,
                                            };
                                            PostureModule::add_pos(boss_boma_2, &pos);
                                        }
                                    }
                                }
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 {
                                // left
                                if MotionModule::frame(boss_boma_2) >= 120.0 {
                                    if MotionModule::frame(boss_boma_2) <= 140.0 {
                                        let pos = Vector3f {
                                            x: 0.5,
                                            y: 0.0,
                                            z: 0.0,
                                        };
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                }
                                if MotionModule::frame(boss_boma_2) >= 130.0 {
                                    if MotionModule::frame(boss_boma_2) <= 140.0 {
                                        if x > MASTER_X_POS + 25.0 {
                                            let pos = Vector3f {
                                                x: -14.75,
                                                y: 0.0,
                                                z: 0.0,
                                            };
                                            PostureModule::add_pos(boss_boma_2, &pos);
                                        }
                                    }
                                }
                            }
                        }
                        if MotionModule::is_end(boss_boma_2)
                            && MotionModule::motion_kind(boss_boma_2) == hash40("taggoopaa")
                            && !DEAD_2
                        {
                            PUNCH = false;
                            StatusModule::change_status_request_from_script(
                                boss_boma_2,
                                *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT,
                                true,
                            );
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2)
                            == false
                            && StatusModule::status_kind(boss_boma_2)
                                != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START
                            && StatusModule::status_kind(boss_boma_2)
                                != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                            && StatusModule::status_kind(boss_boma_2)
                                != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW
                            && StatusModule::status_kind(boss_boma_2)
                                != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                        {
                            if CONTROLLABLE_2 == true {
                                if DEAD_2 == false {
                                    let curr_pos = Vector3f {
                                        x: PostureModule::pos_x(module_accessor),
                                        y: PostureModule::pos_y(module_accessor),
                                        z: PostureModule::pos_z(module_accessor),
                                    };
                                    // Boss Control Movement
                                    // X Controllable
                                    if CONTROLLER_X_CRAZY
                                        < ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_X_CRAZY >= 0.0
                                        && ControlModule::get_stick_x(module_accessor) > 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY
                                        > ControlModule::get_stick_x(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_X_CRAZY <= 0.0
                                        && ControlModule::get_stick_x(module_accessor) < 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY > 0.0
                                        && CONTROLLER_X_CRAZY != 0.0
                                        && ControlModule::get_stick_x(module_accessor) == 0.0
                                    {
                                        CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0
                                        && CONTROLLER_X_CRAZY != 0.0
                                        && ControlModule::get_stick_x(module_accessor) == 0.0
                                    {
                                        CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                    }
                                    if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                        if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                            CONTROLLER_X_CRAZY = 0.0;
                                        }
                                    }
                                    if CONTROLLER_X_CRAZY > 0.0
                                        && ControlModule::get_stick_x(module_accessor) < 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0
                                        && ControlModule::get_stick_x(module_accessor) > 0.0
                                    {
                                        CONTROLLER_X_CRAZY +=
                                            (ControlModule::get_stick_x(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }

                                    // Y Controllable
                                    if CONTROLLER_Y_CRAZY
                                        < ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_Y_CRAZY >= 0.0
                                        && ControlModule::get_stick_y(module_accessor) > 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY
                                        > ControlModule::get_stick_y(module_accessor)
                                            * CONTROL_SPEED_MUL
                                        && CONTROLLER_Y_CRAZY <= 0.0
                                        && ControlModule::get_stick_y(module_accessor) < 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY > 0.0
                                        && CONTROLLER_Y_CRAZY != 0.0
                                        && ControlModule::get_stick_y(module_accessor) == 0.0
                                    {
                                        CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0
                                        && CONTROLLER_Y_CRAZY != 0.0
                                        && ControlModule::get_stick_y(module_accessor) == 0.0
                                    {
                                        CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                    }
                                    if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                        if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                            CONTROLLER_Y_CRAZY = 0.0;
                                        }
                                    }
                                    if CONTROLLER_Y_CRAZY > 0.0
                                        && ControlModule::get_stick_y(module_accessor) < 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0
                                        && ControlModule::get_stick_y(module_accessor) > 0.0
                                    {
                                        CONTROLLER_Y_CRAZY +=
                                            (ControlModule::get_stick_y(module_accessor)
                                                * CONTROL_SPEED_MUL)
                                                * CONTROL_SPEED_MUL_2;
                                    }
                                    let pos = Vector3f {
                                        x: CONTROLLER_X_CRAZY,
                                        y: CONTROLLER_Y_CRAZY,
                                        z: 0.0,
                                    };
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                    //Boss Moves
                                    if PostureModule::lr(boss_boma_2) == 1.0 {
                                        // right
                                        if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_TURN,
                                                true,
                                            );
                                        }
                                    }
                                    if PostureModule::lr(boss_boma_2) == -1.0 {
                                        // left
                                        if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_TURN,
                                                true,
                                            );
                                        }
                                    }
                                    if ControlModule::check_button_on(
                                        module_accessor,
                                        *CONTROL_PAD_BUTTON_JUMP,
                                    ) && MASTER_EXISTS
                                        && MASTER_USABLE
                                        && MASTER_TEAM == CRAZY_TEAM
                                        && StatusModule::status_kind(boss_boma_2)
                                            != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                    {
                                        if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT
                                        {
                                            let floor_dist =
                                                boss_floor_dist(module_accessor, boss_boma_2);
                                            if floor_dist > 0.0 && floor_dist <= 50.0 {
                                                CONTROLLABLE_2 = false;
                                                BARK = false;
                                                PUNCH = true;
                                                SHOCK = false;
                                                LASER = false;
                                                SCRATCH_BLOW = false;
                                                CONTROLLER_X_CRAZY = 0.0;
                                                CONTROLLER_Y_CRAZY = 0.0;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma_2,
                                                    *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT,
                                                    true,
                                                );
                                                MotionModule::change_motion(
                                                    boss_boma_2,
                                                    Hash40::new("taggoopaa"),
                                                    0.0,
                                                    1.0,
                                                    false,
                                                    0.0,
                                                    false,
                                                    false,
                                                );
                                            }
                                        }
                                    }
                                    let cat1 = ControlModule::get_command_flag_cat(
                                        fighter.module_accessor,
                                        0,
                                    );
                                    let finder_entry = ENTRY_ID_2.min(7);
                                    let finder_chord = cat1 & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_N
                                        != 0
                                        && ControlModule::check_button_on(
                                            fighter.module_accessor,
                                            *CONTROL_PAD_BUTTON_GUARD,
                                        );
                                    let finder_edge =
                                        finder_chord && !FINDER_TRIGGER_LATCH[finder_entry];
                                    FINDER_TRIGGER_LATCH[finder_entry] = finder_chord;
                                    if finder_edge && !FINDER {
                                        let floor_dist =
                                            boss_floor_dist(module_accessor, boss_boma_2);
                                        let finder_started =
                                            start_finder_pair(fighter.lua_state_agent, boss_boma_2);
                                        let (_, master_boma) = finder_master_entry_boma();
                                        crate::boss_log!(
                                            "[PB][Finder] trigger_edge chord=special_n+guard started={} floor={:.1} master_status={} crazy_status={} master_motion=0x{:x} crazy_motion=0x{:x}",
                                            finder_started,
                                            floor_dist,
                                            if master_boma.is_null() {
                                                -1
                                            } else {
                                                StatusModule::status_kind(master_boma)
                                            },
                                            StatusModule::status_kind(boss_boma_2),
                                            if master_boma.is_null() {
                                                0
                                            } else {
                                                MotionModule::motion_kind(master_boma)
                                            },
                                            MotionModule::motion_kind(boss_boma_2)
                                        );
                                    }
                                    // Finder is deliberately a B+Guard chord. Plain B must
                                    // remain Crazy Hand's native bomb-drop opener; this status
                                    // already has the normal movement/recovery handling below.
                                    if cat1 & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_N != 0
                                        && ControlModule::check_button_trigger(
                                            fighter.module_accessor,
                                            *CONTROL_PAD_BUTTON_SPECIAL,
                                        )
                                        && !finder_chord
                                        && !FINDER
                                    {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START,
                                            true,
                                        );
                                        crate::boss_log!(
                                            "[PB][CrazyHand] neutral_special action=bomb_drop status={}",
                                            *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START
                                        );
                                    }
                                    if ControlModule::check_button_on(
                                        module_accessor,
                                        *CONTROL_PAD_BUTTON_GUARD,
                                    ) && MotionModule::motion_kind(boss_boma_2)
                                        != smash::hash40("teleport_start")
                                        && MotionModule::motion_kind(boss_boma_2)
                                            != smash::hash40("teleport_end")
                                        && StatusModule::status_kind(boss_boma_2)
                                            != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                    {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        MotionModule::set_rate(boss_boma_2, 1.0);
                                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                        MotionModule::change_motion(
                                            boss_boma_2,
                                            Hash40::new("teleport_start"),
                                            0.0,
                                            1.0,
                                            false,
                                            0.0,
                                            false,
                                            false,
                                        );
                                    }
                                    if ControlModule::check_button_on(
                                        module_accessor,
                                        *CONTROL_PAD_BUTTON_ATTACK,
                                    ) {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        if GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) <= 30.0
                                            && GroundModule::get_distance_to_floor(
                                                module_accessor,
                                                &curr_pos,
                                                curr_pos.y,
                                                true,
                                            ) > 5.0
                                        {
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_DIG_START,
                                                true,
                                            );
                                        } else {
                                            Y_POS_2 = PostureModule::pos_y(boss_boma_2);
                                            StatusModule::change_status_request(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD,
                                                true,
                                            );
                                        }
                                    }
                                    if cat1 & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM,
                                            true,
                                        );
                                    }
                                    if cat1 & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        if GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) <= 50.0
                                            && GroundModule::get_distance_to_floor(
                                                module_accessor,
                                                &curr_pos,
                                                curr_pos.y,
                                                true,
                                            ) > 0.0
                                        {
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_READY,
                                                true,
                                            );
                                        } else {
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START,
                                                true,
                                            );
                                        }
                                    }
                                    if cat1 & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START,
                                            true,
                                        );
                                    }
                                    if ControlModule::get_command_flag_cat(
                                        fighter.module_accessor,
                                        0,
                                    ) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3
                                        != 0
                                    {
                                        let floor_dist =
                                            boss_floor_dist(module_accessor, boss_boma_2);
                                        if floor_dist > 0.0 && floor_dist <= 50.0 {
                                            CONTROLLABLE_2 = false;
                                            CONTROLLER_X_CRAZY = 0.0;
                                            CONTROLLER_Y_CRAZY = 0.0;
                                            CRAZY_KUMO_ACTIVE = false;
                                            CRAZY_KUMO_ENDING = false;
                                            CRAZY_KUMO_START_Y = PostureModule::pos_y(boss_boma_2);
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_KUMO,
                                                true,
                                            );
                                        } else {
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE,
                                                true,
                                            );
                                        }
                                    }
                                    if ControlModule::get_command_flag_cat(
                                        fighter.module_accessor,
                                        0,
                                    ) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3
                                        != 0
                                    {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START,
                                            true,
                                        );
                                    }
                                    if ControlModule::get_command_flag_cat(
                                        fighter.module_accessor,
                                        0,
                                    ) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3
                                        != 0
                                    {
                                        Y_POS_2 = PostureModule::pos_y(boss_boma_2);
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD,
                                            true,
                                        );
                                    }
                                    if ControlModule::check_button_on(
                                        module_accessor,
                                        *CONTROL_PAD_BUTTON_APPEAL_HI,
                                    ) {
                                        if GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) <= 50.0
                                            && GroundModule::get_distance_to_floor(
                                                module_accessor,
                                                &curr_pos,
                                                curr_pos.y,
                                                true,
                                            ) > 0.0
                                            && MASTER_EXISTS
                                            && MASTER_USABLE
                                            && MASTER_TEAM == CRAZY_TEAM
                                        {
                                            if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT
                                            {
                                                CONTROLLABLE_2 = false;
                                                BARK = false;
                                                PUNCH = false;
                                                SHOCK = false;
                                                LASER = true;
                                                SCRATCH_BLOW = false;
                                                CONTROLLER_X_CRAZY = 0.0;
                                                CONTROLLER_Y_CRAZY = 0.0;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma_2,
                                                    *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START,
                                                    true,
                                                );
                                            }
                                        }
                                    }
                                    if ControlModule::check_button_on(
                                        module_accessor,
                                        *CONTROL_PAD_BUTTON_APPEAL_LW,
                                    ) {
                                        if GroundModule::get_distance_to_floor(
                                            module_accessor,
                                            &curr_pos,
                                            curr_pos.y,
                                            true,
                                        ) <= 30.0
                                            && GroundModule::get_distance_to_floor(
                                                module_accessor,
                                                &curr_pos,
                                                curr_pos.y,
                                                true,
                                            ) > 0.0
                                        {
                                            CONTROLLABLE_2 = false;
                                            CONTROLLER_X_CRAZY = 0.0;
                                            CONTROLLER_Y_CRAZY = 0.0;
                                            StatusModule::change_status_request_from_script(
                                                boss_boma_2,
                                                *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU,
                                                true,
                                            );
                                        }
                                    }
                                    if ControlModule::check_button_on(
                                        module_accessor,
                                        *CONTROL_PAD_BUTTON_APPEAL_S_L,
                                    ) {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_START,
                                            true,
                                        );
                                    }
                                    if ControlModule::check_button_on(
                                        module_accessor,
                                        *CONTROL_PAD_BUTTON_APPEAL_S_R,
                                    ) {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma_2,
                                            *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START,
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn install() {
    MASTERCRAZY_NRO_HOOK_ONCE.call_once(|| {
        let _ = skyline::nro::add_hook(nro_hook);
    });
}

pub unsafe fn master_frame(fighter: &mut L2CFighterCommon) {
    if crate::should_quarantine_boss_frame(fighter.module_accessor) {
        return;
    }
    once_per_fighter_frame(fighter);
}

pub unsafe fn crazy_frame(fighter: &mut L2CFighterCommon) {
    if crate::should_quarantine_boss_frame(fighter.module_accessor) {
        return;
    }
    once_per_fighter_frame_2(fighter);
}

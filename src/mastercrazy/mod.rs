use smash::lib::{L2CValue, lua_const::*};
use smash::app::lua_bind::*;
use smash::lua2cpp::{L2CAgentBase, L2CFighterCommon};
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::FighterUtil;
use smash::app::sv_information;
use smash::app::lua_bind;
use smash::hash40;
use smash::phx::Hash40;
use std::sync::Once;
use std::arch::asm;
use skyline::hooks::InlineCtx;
use crate::config::CONFIG;
use crate::selection;
use crate::boss_helpers;
use crate::boss_runtime::{self, BossCommonRuntime, CommonRuntimeSyncGuard};

// Global
static mut BARK : bool = false;
static mut PUNCH : bool = false;
static mut SHOCK : bool = false;
static mut LASER : bool = false;
static mut SCRATCH_BLOW : bool = false;
static mut CONTROL_SPEED_MUL: f32 = 2.0;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;

static mut MASTER_X_POS: f32 = 0.0;
static mut MASTER_Y_POS: f32 = 0.0;
static mut MASTER_Z_POS: f32 = 0.0;
static mut MASTER_USABLE : bool = false;
static mut MASTER_FACING_LEFT : bool = true;
static mut CONTROLLER_X_MASTER: f32 = 0.0;
static mut CONTROLLER_Y_MASTER: f32 = 0.0;

static mut CRAZY_X_POS: f32 = 0.0;
static mut CRAZY_Y_POS: f32 = 0.0;
static mut CRAZY_Z_POS: f32 = 0.0;
static mut CRAZY_USABLE : bool = false;
static mut CRAZY_FACING_RIGHT : bool = true;
static mut CONTROLLER_X_CRAZY: f32 = 0.0;
static mut CONTROLLER_Y_CRAZY: f32 = 0.0;

// Master Hand
static mut CONTROLLABLE : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut MULTIPLE_BULLETS : usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
static mut STOP : bool = false;
static mut MASTER_EXISTS : bool = false;
static mut EXISTS_PUBLIC : bool = false;
static mut Y_POS: f32 = 0.0;
static mut MASTER_TEAM : u64 = 99;
static mut MASTER_LAST_IRON_BALL_ID: u32 = 0;
static mut MASTER_IRON_BALL_OFFSTAGE_FRAMES: i32 = 0;
static mut MASTER_IRON_BALL_SMOOTH_CANCEL: bool = false;
static mut MASTER_KENZAN_SPAWNED: bool = false;
static mut MASTER_CPU_IDLE_STALL_FRAMES: [i32; 8] = [0; 8];
static mut MASTER_CPU_LAST_X: [f32; 8] = [0.0; 8];
static mut MASTER_CPU_LAST_Y: [f32; 8] = [0.0; 8];

// Crazy Hand
static mut CONTROLLABLE_2 : bool = true;
static mut ENTRY_ID_2 : usize = 0;
static mut BOSS_ID_2 : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER_2: usize = 0;
static mut DEAD_2 : bool = false;
static mut JUMP_START_2 : bool = false;
static mut RESULT_SPAWNED_2 : bool = false;
static mut STOP_2 : bool = false;
static mut CRAZY_EXISTS : bool = false;
static mut EXISTS_PUBLIC_2 : bool = false;
static mut Y_POS_2: f32 = 0.0;
static mut CRAZY_TEAM : u64 = 98;
static mut CRAZY_KUMO_ACTIVE: bool = false;
static mut CRAZY_KUMO_START_Y: f32 = 0.0;
static mut CRAZY_KUMO_ENDING: bool = false;
static mut CRAZY_CPU_IDLE_STALL_FRAMES: [i32; 8] = [0; 8];
static mut CRAZY_CPU_LAST_X: [f32; 8] = [0.0; 8];
static mut CRAZY_CPU_LAST_Y: [f32; 8] = [0.0; 8];
static mut CRAZY_FIRE_CHARIOT_PINKY_LATCH: [bool; 8] = [false; 8];
static mut CRAZY_FIRE_CHARIOT_THUMB_LATCH: [bool; 8] = [false; 8];


extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
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
    let probe_dist = GroundModule::get_distance_to_floor(module_accessor, &probe_pos, probe_pos.y, true);
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
    }
}

#[inline(always)]
unsafe fn reset_crazy_cpu_idle_recovery(entry_id: usize) {
    if entry_id < 8 {
        CRAZY_CPU_IDLE_STALL_FRAMES[entry_id] = 0;
        CRAZY_CPU_LAST_X[entry_id] = 0.0;
        CRAZY_CPU_LAST_Y[entry_id] = 0.0;
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
    let status = StatusModule::status_kind(boss_boma);
    if !master_cpu_wait_family_status(status) {
        reset_master_cpu_idle_recovery(entry_id);
        return;
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
        println!(
            "[PB][MasterHand][CPURecovery] entry={} status={} pos=({:.2},{:.2},{:.2})",
            entry_id,
            status,
            current_x,
            current_y,
            PostureModule::pos_z(boss_boma),
        );
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
    let status = StatusModule::status_kind(boss_boma);
    if !crazy_cpu_wait_family_status(status) {
        reset_crazy_cpu_idle_recovery(entry_id);
        return;
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
        println!(
            "[PB][CrazyHand][CPURecovery] entry={} status={} pos=({:.2},{:.2},{:.2})",
            entry_id,
            status,
            current_x,
            current_y,
            PostureModule::pos_z(boss_boma),
        );
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
unsafe fn configure_boss_owner_mode(
    boss_boma: *mut BattleObjectModuleAccessor,
    entry_id: usize,
) {
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
unsafe fn reset_mastercrazy_shared_runtime() {
    BARK = false;
    PUNCH = false;
    SHOCK = false;
    LASER = false;
    SCRATCH_BLOW = false;
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
    JUMP_START = false;
    STOP = false;
    MULTIPLE_BULLETS = 0;
    MASTER_LAST_IRON_BALL_ID = 0;
    MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
    MASTER_IRON_BALL_SMOOTH_CANCEL = false;
    MASTER_KENZAN_SPAWNED = false;
    reset_master_cpu_idle_recovery(ENTRY_ID);
    reset_mastercrazy_shared_runtime();
}

#[inline(always)]
unsafe fn reset_crazy_runtime_for_spawn() {
    JUMP_START_2 = false;
    STOP_2 = false;
    CRAZY_KUMO_ACTIVE = false;
    CRAZY_KUMO_START_Y = 0.0;
    CRAZY_KUMO_ENDING = false;
    reset_crazy_cpu_idle_recovery(ENTRY_ID_2);
    reset_crazy_fire_chariot_latches(ENTRY_ID_2);
    reset_mastercrazy_shared_runtime();
}

#[inline(always)]
unsafe fn reset_mastercrazy_result_runtime() {
    reset_mastercrazy_shared_runtime();

    CONTROLLABLE = true;
    ENTRY_ID = 0;
    FIGHTER_MANAGER = 0;
    MULTIPLE_BULLETS = 0;
    DEAD = false;
    JUMP_START = false;
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

    CONTROLLABLE_2 = true;
    ENTRY_ID_2 = 0;
    FIGHTER_MANAGER_2 = 0;
    DEAD_2 = false;
    JUMP_START_2 = false;
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
    CRAZY_FIRE_CHARIOT_PINKY_LATCH = [false; 8];
    CRAZY_FIRE_CHARIOT_THUMB_LATCH = [false; 8];
}

#[inline(always)]
unsafe fn acquire_master_hand_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry_id: usize,
) -> *mut BattleObjectModuleAccessor {
    let boss_boma = boss_helpers::acquire_boss_item(
        module_accessor,
        &raw mut BOSS_ID,
        *ITEM_KIND_MASTERHAND,
    );
    configure_boss_owner_mode(boss_boma, entry_id);
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
        reason,
        entry_id,
        last_iron_ball_id,
    );
    if !module_accessor.is_null() && ItemModule::is_have_item(module_accessor, 0) {
        let held_item_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
        if held_item_id != 0 && sv_battle_object::is_active(held_item_id) {
            let held_item_boma = sv_battle_object::module_accessor(held_item_id);
            if !held_item_boma.is_null()
            && smash::app::utility::get_kind(&mut *held_item_boma) == *ITEM_KIND_MASTERHANDIRONBALL {
                ItemModule::remove_item(module_accessor, 0);
            }
        }
    }
    if !boss_boma.is_null() && ItemModule::is_have_item(boss_boma, 0) {
        let held_item_id = ItemModule::get_have_item_id(boss_boma, 0) as u32;
        if held_item_id != 0 && sv_battle_object::is_active(held_item_id) {
            let held_item_boma = sv_battle_object::module_accessor(held_item_id);
            if !held_item_boma.is_null()
            && smash::app::utility::get_kind(&mut *held_item_boma) == *ITEM_KIND_MASTERHANDIRONBALL {
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
        WorkModule::off_flag(boss_boma, *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_CREATE);
        WorkModule::off_flag(boss_boma, *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_THROW);
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
    let boss_boma = boss_helpers::acquire_boss_item(
        module_accessor,
        &raw mut BOSS_ID_2,
        *ITEM_KIND_CRAZYHAND,
    );
    configure_boss_owner_mode(boss_boma, entry_id);
    boss_boma
}

#[inline(always)]
unsafe fn initialize_master_hand_boss(
    boss_boma: *mut BattleObjectModuleAccessor,
    get_boss_intensity: f32,
) {
    WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
    WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
    WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
    WorkModule::set_int(boss_boma, *ITEM_BOSS_MODE_ADVENTURE_HARD, *ITEM_INSTANCE_WORK_INT_BOSS_MODE);
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
    WorkModule::set_int(boss_boma, *ITEM_BOSS_MODE_ADVENTURE_HARD, *ITEM_INSTANCE_WORK_INT_BOSS_MODE);
    WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
    WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
    WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
    WorkModule::set_int(
        boss_boma,
        *ITEM_VARIATION_CRAZYHAND_MASTERHAND_STANDARD,
        *ITEM_INSTANCE_WORK_INT_VARIATION,
    );
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
}

#[skyline::hook(replace = MH_CHAKRAM_THROW_SUB)]
unsafe fn mh_chakram_throw_sub(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    if weapon_owner_is_player(lua_state)
        && AttackModule::is_attack(module_accessor, 0, false)
    {
        AttackModule::set_target_category(module_accessor, 0, *COLLISION_CATEGORY_MASK_ALL as u32);
    }
    original!()(item)
}

#[skyline::hook(replace = MH_IRON_BALL_THROW_SUB)]
unsafe fn mh_iron_ball_throw_sub(item: &mut L2CAgentBase) -> L2CValue {
    let lua_state = item.lua_state_agent;
    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
    if weapon_owner_is_player(lua_state)
        && AttackModule::is_attack(module_accessor, 0, false)
    {
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
            AttackModule::set_target_category(module_accessor, 0, *COLLISION_CATEGORY_MASK_ALL as u32);
        }
        if AttackModule::is_attack(module_accessor, 1, false) {
            AttackModule::set_target_category(module_accessor, 1, *COLLISION_CATEGORY_MASK_ALL as u32);
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
        let entry_id = boss_runtime::sanitize_entry_id(
            WorkModule::get_int(module_accessor, ITEM_INSTANCE_WORK_INT_ENTRY_ID) as usize,
        );
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
                WorkModule::set_int(kenzan_boma, entry_id as i32, ITEM_INSTANCE_WORK_INT_ENTRY_ID);
            } else {
                println!("[PB][MasterHand][Kenzan] weapon accessor was null after create_weapon");
            }
        } else {
            println!("[PB][MasterHand][Kenzan] create_weapon failed or inactive");
        }
        MASTER_KENZAN_SPAWNED = true;
        StatusModule::change_status_request(module_accessor, *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END, false);
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
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            ENTRY_ID = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
            let _runtime_guard = CommonRuntimeSyncGuard::new(
                boss_runtime::slot_ptr(&raw mut boss_runtime::MASTER_HAND_RUNTIME, ENTRY_ID),
                load_master_hand_runtime,
                store_master_hand_runtime,
            );
            let fighter_manager = boss_helpers::fighter_manager();
            
            let selected_via_slot = selection::is_selected_css_boss(module_accessor, *ITEM_KIND_MASTERHAND);
            if selected_via_slot {
                boss_helpers::clear_hidden_host_effects(module_accessor);
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 || !ItemModule::is_have_item(module_accessor, 0) {
                        ItemModule::remove_all(module_accessor);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = acquire_master_hand_item(module_accessor, ENTRY_ID);
                        ModelModule::set_scale(boss_boma, 0.08);
                        MotionModule::change_motion(boss_boma, Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor, Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        ModelModule::set_joint_rotate(module_accessor, Hash40::new("root") , &mut Vector3f{x: -270.0, y: 180.0, z: -90.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                    }
                }
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            DEAD = false;
                            CONTROLLABLE = true;
                        }
                        reset_master_runtime_for_spawn();
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            MASTER_EXISTS = true;
                            let boss_boma = acquire_master_hand_item(module_accessor, ENTRY_ID);
                            initialize_master_hand_boss(boss_boma, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                        }
                    }

                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && !STOP
                    && !CONFIG.options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                    }
                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && STOP
                    && !CONFIG.options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                        let x = 0.0;
                        let y = 0.0;
                        let z = 0.0;
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &module_pos);
                    }

                    // Respawn in case of Squad Strike or Specific Circumstances

                    if sv_information::is_ready_go() && !ItemModule::is_have_item(module_accessor, 0) && ModelModule::scale(module_accessor) == 0.0001
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                        if smash::app::smashball::is_training_mode() || CONFIG.options.boss_respawn.unwrap_or(false) {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);
                            DEAD = false;
                            CONTROLLABLE = true;
                            reset_master_runtime_for_spawn();
                            MASTER_TEAM = TeamModule::team_no(module_accessor);
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(1.0);
                            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            MASTER_EXISTS = true;
                            let boss_boma = acquire_master_hand_item(module_accessor, ENTRY_ID);
                            initialize_master_hand_boss(boss_boma, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE, true);

                            let x = PostureModule::pos_x(module_accessor);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(module_accessor);
                            let module_pos = Vector3f{x: x, y: y, z: z};
                            PostureModule::set_pos(boss_boma, &module_pos);

                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                CONTROLLABLE = true;
                            }
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        let x = PostureModule::pos_x(boss_boma);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(boss_boma);
                        MASTER_X_POS = x;
                        MASTER_Y_POS = y;
                        MASTER_Z_POS = z;
                        if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                            let sub_hp = 999.0 - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                            MASTER_FACING_LEFT = false;
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                            MASTER_FACING_LEFT = true;
                        }
                        JostleModule::set_status(module_accessor, false);
                    }

                    if sv_information::is_ready_go() {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_BARK && !CRAZY_USABLE {
                            BARK = false;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT, true);
                        }
                    }
                    if sv_information::is_ready_go() == true {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        if MotionModule::motion_kind(boss_boma) == hash40("wait") && !DEAD {
                            SoundModule::stop_se(boss_boma, smash::phx::Hash40::new("se_boss_masterhand_chakram_fly"), 0);
                        }
                    }
                    // Team Attack Trigger
                    if sv_information::is_ready_go() == true && !DEAD {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                        if MotionModule::motion_kind(boss_boma) == hash40("wait") && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                            if CONTROLLABLE == false && smash::app::sv_math::rand(hash40("fighter"), 500) as f32 == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                            || CONTROLLABLE && smash::app::sv_math::rand(hash40("fighter"), 900) as f32 == smash::app::sv_math::rand(hash40("fighter"), 900) as f32 {
                                if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 40.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                        CONTROLLABLE = false;
                                        BARK = true;
                                        PUNCH = false;
                                        SHOCK = false;
                                        LASER = false;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_BARK, true);
                                    }
                                }
                            }
                            else if CONTROLLABLE == false && smash::app::sv_math::rand(hash40("fighter"), 500) as f32 == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                            || CONTROLLABLE && smash::app::sv_math::rand(hash40("fighter"), 900) as f32 == smash::app::sv_math::rand(hash40("fighter"), 900) as f32 {
                                if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                        CONTROLLABLE = false;
                                        BARK = false;
                                        PUNCH = false;
                                        SHOCK = false;
                                        LASER = false;
                                        SCRATCH_BLOW = true;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                    }
                                }
                            }
                            else if CONTROLLABLE == false && smash::app::sv_math::rand(hash40("fighter"), 500) as f32 == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                            || CONTROLLABLE && smash::app::sv_math::rand(hash40("fighter"), 900) as f32 == smash::app::sv_math::rand(hash40("fighter"), 900) as f32 {
                                if CRAZY_EXISTS == true && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                        CONTROLLABLE = false;
                                        BARK = false;
                                        PUNCH = false;
                                        SHOCK = true;
                                        LASER = false;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START, true);
                                    }
                                }
                            }
                        }
                    }

                    // STUBS AI

                    if sv_information::is_ready_go() && !DEAD {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if CONTROLLABLE {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD == true {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                            if STOP == false && CONFIG.options.boss_respawn.unwrap_or(false) && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                            }
                            MASTER_EXISTS = false;
                            if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_DEAD
                            || StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD
                            && MotionModule::frame(boss_boma) > 250.0 {
                                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                ItemModule::remove_all(module_accessor);
                                if STOP == false && CONFIG.options.boss_respawn.unwrap_or(false) {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
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
                        if sv_information::is_ready_go() == true {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_STANDBY {
                                    MASTER_EXISTS = false;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 0.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: 180.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if MotionModule::frame(boss_boma) == 0.0 {
                                        smash_script::macros::CAM_ZOOM_IN_arg5(fighter, 0.0, 0.0, 5.0, 0.0, 0.0);
                                        smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_dead"),true,false);
                                        smash_script::macros::EFFECT(fighter, Hash40::new("sys_bg_criticalhit"), Hash40::new("top"), 0,7,0,0,0,0,1,0,0,0,0,0,0,false);
                                        smash_script::macros::EFFECT(fighter, Hash40::new("sys_bg_boss_finishhit"), Hash40::new("top"), 0,7,0,0,0,0,1,0,0,0,0,0,0,false);
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
                                        smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_bg_criticalhit"),true,false);
                                        smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_bg_boss_finishhit"),true,false);
                                        SlowModule::clear_whole(module_accessor);
                                    }
                                    if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                        EXISTS_PUBLIC = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                                    }
                                }
                            }
                        }
                    }

                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if RESULT_SPAWNED == false {
                            RESULT_SPAWNED = true;
                            reset_mastercrazy_result_runtime();
                            let boss_boma = acquire_master_hand_item(module_accessor, ENTRY_ID);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START,true);
                        }
                        boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY && !CRAZY_EXISTS {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY && CRAZY_EXISTS {
                            CONTROLLABLE = true;
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            MASTER_TEAM = TeamModule::team_no(module_accessor);
                            if MASTER_TEAM == CRAZY_TEAM {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("entry2"),0.0,1.0,false,0.0,false,false);
                            }
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("entry2") {
                            MotionModule::set_rate(boss_boma, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.5);
                        }
                    }

                    // SET FIGHTER LOOP

                    if sv_information::is_ready_go() == true {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING); // I did yoink these transition terms and ability to hide the player cursor from Claude's awesome mod which can be found here: https://github.com/ClaudevonRiegan/Playable_Bosses
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ITEM);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_JUMP_AERIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_TREAD_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ITEM_THROW);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_WALL_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_CATCH);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_GUARD);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_CLIFF);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LASSO);
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            fighter.set_situation(SITUATION_KIND_AIR.into());
                            GroundModule::set_correct(module_accessor, smash::app::GroundCorrectKind(*GROUND_CORRECT_KIND_AIR));
                            MotionModule::change_motion(module_accessor,Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if DEAD == false {
                        // SET POS AND STOPS OUT OF BOUNDS
                        if ModelModule::scale(module_accessor) == 0.0001 {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                            if FighterUtil::is_hp_mode(module_accessor) == true {
                                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD
                                || StatusModule::status_kind(module_accessor) == 79 {
                                    if DEAD == false {
                                        CONTROLLABLE = false;
                                        DEAD = true;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                    }
                                }
                            }
                            let x = PostureModule::pos_x(boss_boma);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(boss_boma);
                            let boss_pos = Vector3f{x: x, y: y + 20.0, z: z};
                            if !CONTROLLABLE || boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                    let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                    let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                    let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                }
                                else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                    let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                }
                                else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                            else {
                                if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                    let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                    let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                    let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                }
                                else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                    let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                }
                                else {
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
                    
                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma, i, false) {
                            AttackModule::set_target_category(boss_boma, i, *COLLISION_CATEGORY_MASK_ALL as u32);
                        }
                    }
                    if MASTER_LAST_IRON_BALL_ID != 0 {
                        if !sv_battle_object::is_active(MASTER_LAST_IRON_BALL_ID) {
                            MASTER_LAST_IRON_BALL_ID = 0;
                            MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
                        } else {
                            let iron_ball_boma = sv_battle_object::module_accessor(MASTER_LAST_IRON_BALL_ID);
                            if !iron_ball_boma.is_null() {
                                if AttackModule::is_attack(iron_ball_boma, 0, false) {
                                    AttackModule::set_target_category(iron_ball_boma, 0, *COLLISION_CATEGORY_MASK_ALL as u32);
                                }
                                if MotionModule::motion_kind(iron_ball_boma) == hash40("appear") {
                                    AttackModule::clear_all(iron_ball_boma);
                                }
                                if StatusModule::status_kind(iron_ball_boma) == *ITEM_MASTERHANDIRONBALL_STATUS_KIND_MOVE1 {
                                    action(iron_ball_boma, *ITEM_MASTERHANDIRONBALL_ACTION_SET_BOUND, 0.0);
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
                            if DamageModule::damage(module_accessor, 0) >= hp { // HEALTH
                                if DEAD == false {
                                    CONTROLLABLE = false;
                                    DEAD = true;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                }
                            }
                        }
                    }

                    // FIXES SPAWN

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            if JUMP_START == false {
                                JUMP_START = true;
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                    CONTROLLABLE = false;
                                }
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true && !DEAD {
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                let master_pos = Vector3f{x: CRAZY_X_POS + 100.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                let master_pos = Vector3f{x: CRAZY_X_POS - 100.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_BARK {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 95.0 && MotionModule::frame(boss_boma) <= MotionModule::end_frame(boss_boma) - 92.0 {
                                MotionModule::set_rate(boss_boma, 0.1);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 0.1);
                            }
                            else {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                    BARK = false;
                                }
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    BARK = false;
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                LASER = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                SCRATCH_BLOW = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                PUNCH = false;
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                            CONTROLLABLE = false;
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true
                        && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT {
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE,
                                true,
                            );
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                            maybe_recover_master_cpu_idle(boss_boma, ENTRY_ID);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP && !DEAD {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            let stunned = !CONFIG.options.full_stun_duration.unwrap_or(false);
                            if stunned {
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_DOWN_END,true);
                            }
                            CONTROLLABLE = false;
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("electroshock_start") && SHOCK {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            CONTROLLABLE = false;
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 && !DEAD {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK, true);
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                        && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_END && !CONTROLLABLE && SHOCK {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                CONTROLLABLE = true;
                                SHOCK = false;
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true
                        && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_END && SHOCK {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                SHOCK = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME
                        || StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                        || CONTROLLABLE {
                            MASTER_USABLE = true;
                        }
                        else {
                            MASTER_USABLE = false;
                        }

                        if PUNCH && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                        && CRAZY_EXISTS
                        && !DEAD
                        && MASTER_USABLE {
                            CONTROLLABLE = false;
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                    let master_pos = Vector3f{x: CRAZY_X_POS - 130.0, y: CRAZY_Y_POS + 15.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                                else {
                                    let master_pos = Vector3f{x: CRAZY_X_POS - 130.0, y: CRAZY_Y_POS + 10.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                    let master_pos = Vector3f{x: CRAZY_X_POS + 130.0, y: CRAZY_Y_POS + 15.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                                else {
                                    let master_pos = Vector3f{x: CRAZY_X_POS + 130.0, y: CRAZY_Y_POS + 10.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                            }
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_GOOPAA, true);
                        }
                        if PUNCH && !DEAD && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA && MASTER_USABLE {
                            MotionModule::set_rate(boss_boma, 1.15);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.15);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                let master_pos = Vector3f{x: MASTER_X_POS, y: CRAZY_Y_POS + 15.0, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            else {
                                let master_pos = Vector3f{x: MASTER_X_POS, y: CRAZY_Y_POS + 10.0, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                    CONTROLLABLE = true;
                                }
                            }
                        }

                        if LASER && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                        && CRAZY_EXISTS
                        && !DEAD
                        && MASTER_USABLE {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                let master_pos = Vector3f{x: CRAZY_X_POS + 130.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                let master_pos = Vector3f{x: CRAZY_X_POS - 130.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START,true);
                        }
                        if LASER && !DEAD && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START && MASTER_USABLE {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                    CONTROLLABLE = true;
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_FIRING {
                            MotionModule::set_rate(boss_boma, 2.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_HOLD {
                            MotionModule::set_rate(boss_boma, 2.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_SHOOT {
                            MotionModule::set_rate(boss_boma, 2.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);

                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START {
                            MotionModule::set_rate(boss_boma, 1.5);
                            //BOSS POSITION
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_TARGET_FOUND);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_X);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Y);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Z);
                            MotionModule::set_rate(boss_boma, 2.0);
                            //BOSS POSITION
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOMING {
                            MotionModule::set_rate(boss_boma, 2.0);
                            //BOSS POSITION
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                            MotionModule::set_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                PostureModule::set_pos(boss_boma, &Vector3f{x: PostureModule::pos_x(boss_boma), y: Y_POS, z: PostureModule::pos_z(boss_boma)});
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU || StatusModule::status_kind(boss_boma) == 78 {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                PostureModule::set_pos(boss_boma, &Vector3f{x: PostureModule::pos_x(boss_boma), y: Y_POS, z: PostureModule::pos_z(boss_boma)});
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START {
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START {
                            MotionModule::set_rate(boss_boma, 1.0);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.1);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_PRE_MOVE {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if (StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_PRE_MOVE) && !DEAD {
                            if boss_floor_y(module_accessor, boss_boma).is_none() {
                                MASTER_IRON_BALL_OFFSTAGE_FRAMES += 1;
                                if MASTER_IRON_BALL_OFFSTAGE_FRAMES > MASTER_IRON_BALL_OFFSTAGE_LIMIT {
                                    cancel_master_iron_ball(module_accessor, boss_boma, "offstage_timeout");
                                }
                            } else {
                                MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
                            }
                            if WorkModule::is_flag(boss_boma, *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_THROW) {
                                if ItemModule::is_have_item(module_accessor, 0) {
                                    let held_item_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                                    if held_item_id != 0 && sv_battle_object::is_active(held_item_id) {
                                        let held_item_boma = sv_battle_object::module_accessor(held_item_id);
                                        if !held_item_boma.is_null()
                                        && smash::app::utility::get_kind(&mut *held_item_boma) == *ITEM_KIND_MASTERHANDIRONBALL {
                                            ItemModule::remove_item(module_accessor, 0);
                                        }
                                    }
                                }
                                let mut throw_joint = Vector3f {
                                    x: PostureModule::pos_x(boss_boma),
                                    y: PostureModule::pos_y(boss_boma),
                                    z: PostureModule::pos_z(boss_boma),
                                };
                                let throw_joint = ModelModule::joint_global_position(boss_boma, Hash40::new("throw"), &mut throw_joint, true);
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
                                    let iron_ball_boma = sv_battle_object::module_accessor(iron_ball_id);
                                    if !iron_ball_boma.is_null() {
                                        LinkModule::remove_model_constraint(iron_ball_boma, true);
                                        if LinkModule::is_link(iron_ball_boma, *ITEM_LINK_NO_HAVE) {
                                            LinkModule::unlink(iron_ball_boma, *ITEM_LINK_NO_HAVE);
                                        }
                                        action(iron_ball_boma, *ITEM_MASTERHANDIRONBALL_ACTION_SET_BOUND, 0.0);
                                        StatusModule::change_status_request_from_script(
                                            iron_ball_boma,
                                            *ITEM_MASTERHANDIRONBALL_STATUS_KIND_MOVE2,
                                            true,
                                        );
                                    }
                                } else {
                                    MASTER_LAST_IRON_BALL_ID = 0;
                                }
                                WorkModule::off_flag(boss_boma, *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_CREATE);
                                WorkModule::off_flag(boss_boma, *ITEM_MASTERHAND_INSTANCE_WORK_FLAG_IRON_BALL_THROW);
                            }
                        } else {
                            MASTER_IRON_BALL_OFFSTAGE_FRAMES = 0;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_START {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.3);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI {
                            MotionModule::set_rate(boss_boma, 1.1);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.1);
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.2, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.2, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END {
                            MotionModule::set_rate(boss_boma, 1.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LANDING {
                            CONTROLLABLE = false;
                        }
                        if MotionModule::is_end(boss_boma) && MotionModule::motion_kind(boss_boma) == hash40("teleport_end") && !DEAD {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                MotionModule::change_motion(boss_boma,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE, true);
                            }
                            else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                MotionModule::change_motion(boss_boma,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                CONTROLLABLE = true;
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CANCEL {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    MotionModule::set_rate(boss_boma, 2.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_RUSH_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_THROW_END_1 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_MISS_END {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START
                                || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE
                                || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN {
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    MASTER_KENZAN_SPAWNED = false;
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_KENZAN
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE
                            {
                                MASTER_KENZAN_SPAWNED = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_END {
                                if MASTER_IRON_BALL_SMOOTH_CANCEL {
                                    let tail_start_frame =
                                        (MotionModule::end_frame(boss_boma) - MASTER_IRON_BALL_END_TAIL_FRAMES)
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
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_END {
                                CONTROLLABLE = false;
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_RND_WAIT {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TO_POINT {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT {
                                CONTROLLABLE = true;
                            }
                        }

                        if CONTROLLABLE && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if CONTROLLABLE && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOMING {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_ATTACK {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                MotionModule::set_rate(boss_boma, 4.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.0);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_START {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                MotionModule::set_rate(boss_boma, 3.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 3.0);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_HOMING {
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_X);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Y);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Z);
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == false {
                                    MULTIPLE_BULLETS = 0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == true {
                                    MULTIPLE_BULLETS = 2;
                                }
                            }
                            else {
                                MULTIPLE_BULLETS = 2;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU && !DEAD {
                            if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                                if MULTIPLE_BULLETS != 0 {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                                    MULTIPLE_BULLETS = MULTIPLE_BULLETS - 1;
                                }
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                            if MULTIPLE_BULLETS != 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if MULTIPLE_BULLETS == 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU {
                            if MULTIPLE_BULLETS != 0 {
                                MotionModule::set_rate(boss_boma, 5.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 5.0);
                            }
                            if MULTIPLE_BULLETS == 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                        }

                        if CONTROLLABLE {
                            MULTIPLE_BULLETS = 0;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_HOMING {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("teleport_start") && MotionModule::is_end(boss_boma) {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            MotionModule::change_motion(boss_boma,Hash40::new("teleport_end"),0.0,1.0,false,0.0,false,false);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_HOMING {
                            MotionModule::set_rate(boss_boma, 1.25);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.25);
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CANCEL, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CENTER_MOVE {
                            MotionModule::set_rate(boss_boma, 4.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START || MotionModule::motion_kind(boss_boma) == hash40("chakram_start") || MotionModule::motion_kind(boss_boma) == hash40("chakram_start_reverse") {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 && MotionModule::motion_kind(boss_boma) == hash40("chakram_start") && !DEAD {
                            MotionModule::change_motion(boss_boma,Hash40::new("chakram_end"),0.0,1.0,false,0.0,false,false);
                        }
                        if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 && MotionModule::motion_kind(boss_boma) == hash40("chakram_start_reverse") && !DEAD {
                            MotionModule::change_motion(boss_boma,Hash40::new("chakram_end"),0.0,1.0,false,0.0,false,false);
                        }
                        if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 20.0 && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START && !DEAD {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                            MotionModule::change_motion(boss_boma,Hash40::new("chakram_start_reverse"),MotionModule::end_frame(boss_boma) - 19.0,1.0,false,0.0,false,false);
                        }
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 18.0 && MotionModule::motion_kind(boss_boma) == hash40("chakram_start_reverse") && !DEAD {
                            ItemModule::remove_item(module_accessor, 0);
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHANDCHAKRAM), 0, 0, false, false);
                            SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                            let chakram1_boma = sv_battle_object::module_accessor(ItemModule::get_have_item_id(module_accessor, 0) as u32);
                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                smash::app::lua_bind::PostureModule::set_lr(chakram1_boma, -1.0);
                            }
                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                smash::app::lua_bind::PostureModule::set_lr(chakram1_boma, 1.0);
                            }
                            action(chakram1_boma, *ITEM_MASTERHANDCHAKRAM_ACTION_SHOOT3, 0.0);

                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHANDCHAKRAM), 0, 0, false, false);
                            SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                            let chakram2_boma = sv_battle_object::module_accessor(ItemModule::get_have_item_id(module_accessor, 0) as u32);
                            let chakram2_pos = Vector3f{x: PostureModule::pos_x(chakram1_boma), y: PostureModule::pos_y(chakram1_boma) - 10.0, z: PostureModule::pos_z(chakram1_boma)};
                            LinkModule::remove_model_constraint(chakram2_boma, true);
                            PostureModule::set_pos(chakram2_boma, &chakram2_pos);
                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                smash::app::lua_bind::PostureModule::set_lr(chakram2_boma, -1.0);
                            }
                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                smash::app::lua_bind::PostureModule::set_lr(chakram2_boma, 1.0);
                            }
                            SoundModule::play_se(boss_boma, Hash40::new("se_boss_masterhand_chakram_fly"), true, false, false, false, smash::app::enSEType(0));
                            action(chakram2_boma, *ITEM_MASTERHANDCHAKRAM_ACTION_SHOOT2, 0.0);
                        }
                        if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 && MotionModule::motion_kind(boss_boma) == hash40("chakram_end") && !DEAD {
                            SoundModule::stop_se(boss_boma, smash::phx::Hash40::new("se_boss_masterhand_chakram_fly"), 0);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT, true);
                            }
                            else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT, true);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                MotionModule::change_motion(boss_boma,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                CONTROLLABLE = true;
                            }
                        }
                        if MotionModule::motion_kind(boss_boma) == hash40("chakram_end") && !DEAD {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_PRE_MOVE && !DEAD {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START, true);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CAPTURE {
                            MotionModule::set_rate(boss_boma, 4.75);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.75);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN {
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                    CONTROLLER_X_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                    CONTROLLER_Y_MASTER = 0.0;
                                }
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                    CONTROLLABLE = true;
                                }
                            }
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                }
                            }
                        }
                        if MotionModule::frame(boss_boma) <= 0.0 && MotionModule::motion_kind(boss_boma) == hash40("teleport_end")  {
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: -100.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_x(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 100.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: 0.0, y: -50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 0.0, y: 50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                    }
                    if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                        if CONTROLLABLE == true {
                            if DEAD == false {
                                let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.06 {
                                        CONTROLLER_X_MASTER = 0.0;
                                    }
                                    if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.06 {
                                        CONTROLLER_X_MASTER = 0.0;
                                    }
                                }
                                if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.06 {
                                        CONTROLLER_Y_MASTER = 0.0;
                                    }
                                    if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.06 {
                                        CONTROLLER_Y_MASTER = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                                
                                // Boss Moves
                                if PostureModule::lr(boss_boma) == 1.0 { // right
                                    if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                if PostureModule::lr(boss_boma) == -1.0 { // left
                                    if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                    if CRAZY_EXISTS == true && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                            CONTROLLABLE = false;
                                            BARK = false;
                                            PUNCH = false;
                                            SHOCK = true;
                                            LASER = false;
                                            SCRATCH_BLOW = false;
                                            CONTROLLER_X_MASTER = 0.0;
                                            CONTROLLER_Y_MASTER = 0.0;
                                            let z = PostureModule::pos_z(boss_boma);
                                            let module_pos = Vector3f{x: 50.0, y: 25.0, z: z};
                                            PostureModule::set_pos(boss_boma, &module_pos);
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START, true);
                                        }
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) && MotionModule::motion_kind(boss_boma) != smash::hash40("teleport_start") && MotionModule::motion_kind(boss_boma) != smash::hash40("teleport_end") && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                    MotionModule::change_motion(boss_boma,Hash40::new("teleport_start"),0.0,1.0,false,0.0,false,false);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CAPTURE, true);
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START, true);
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START, true);
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START, true);
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 55.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START, true);
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_UP, true);
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                    Y_POS = PostureModule::pos_y(boss_boma);
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 40.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                            CONTROLLABLE = false;
                                            BARK = true;
                                            PUNCH = false;
                                            SHOCK = false;
                                            LASER = false;
                                            SCRATCH_BLOW = false;
                                            CONTROLLER_X_MASTER = 0.0;
                                            CONTROLLER_Y_MASTER = 0.0;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_BARK, true);
                                        }
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                            CONTROLLABLE = false;
                                            BARK = false;
                                            PUNCH = false;
                                            SHOCK = false;
                                            LASER = false;
                                            SCRATCH_BLOW = true;
                                            CONTROLLER_X_MASTER = 0.0;
                                            CONTROLLER_Y_MASTER = 0.0;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                        }
                                    }
                                    else {
                                        CONTROLLABLE = false;
                                        CONTROLLER_X_MASTER = 0.0;
                                        CONTROLLER_Y_MASTER = 0.0;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                    let floor_dist = boss_floor_dist(module_accessor, boss_boma);
                                    if floor_dist <= 50.0 && floor_dist > 0.0 {
                                        if let Some(floor_y) = boss_floor_y(module_accessor, boss_boma) {
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
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X_MASTER = 0.0;
                                    CONTROLLER_Y_MASTER = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START, true);
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
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            ENTRY_ID_2 = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
            let _runtime_guard = CommonRuntimeSyncGuard::new(
                boss_runtime::slot_ptr(&raw mut boss_runtime::CRAZY_HAND_RUNTIME, ENTRY_ID_2),
                load_crazy_hand_runtime,
                store_crazy_hand_runtime,
            );
            let fighter_manager = boss_helpers::fighter_manager();
            
            let selected_via_slot = selection::is_selected_css_boss(module_accessor, *ITEM_KIND_CRAZYHAND);
            if selected_via_slot {
                boss_helpers::clear_hidden_host_effects(module_accessor);
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 || !ItemModule::is_have_item(module_accessor, 0) {
                        ItemModule::remove_all(module_accessor);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma_2 = acquire_crazy_hand_item(module_accessor, ENTRY_ID_2);
                        ModelModule::set_scale(boss_boma_2, 0.08);
                        MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor,Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        ModelModule::set_joint_rotate(module_accessor, Hash40::new("root") , &mut Vector3f{x: -270.0, y: 180.0, z: -90.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                    }
                }
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            DEAD_2 = false;
                            CONTROLLABLE_2 = true;
                        }
                        reset_crazy_runtime_for_spawn();
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                        ENTRY_ID_2 = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            EXISTS_PUBLIC_2 = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            CRAZY_EXISTS = true;
                            let boss_boma_2 = acquire_crazy_hand_item(module_accessor, ENTRY_ID_2);
                            initialize_crazy_hand_boss(boss_boma_2, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                        }
                    }

                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && !STOP_2
                    && !CONFIG.options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                    }
                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && STOP_2
                    && !CONFIG.options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                        let x = 0.0;
                        let y = 0.0;
                        let z = 0.0;
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &module_pos);
                    }

                    // Respawn in case of Squad Strike or Specific Circumstances

                    if sv_information::is_ready_go() && !ItemModule::is_have_item(module_accessor, 0) && ModelModule::scale(module_accessor) == 0.0001
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                        if smash::app::smashball::is_training_mode() || CONFIG.options.boss_respawn.unwrap_or(false) {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);
                            DEAD_2 = false;
                            CONTROLLABLE_2 = true;
                            reset_crazy_runtime_for_spawn();
                            CRAZY_EXISTS = true;
                            CRAZY_TEAM = TeamModule::team_no(module_accessor);
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                            ENTRY_ID_2 = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            EXISTS_PUBLIC_2 = true;
                            RESULT_SPAWNED = false;
                            RESULT_SPAWNED_2 = false;
                            let boss_boma_2 = acquire_crazy_hand_item(module_accessor, ENTRY_ID_2);
                            initialize_crazy_hand_boss(boss_boma_2, get_boss_intensity);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE, true);

                            let x = PostureModule::pos_x(module_accessor);
                            let y = PostureModule::pos_y(boss_boma_2);
                            let z = PostureModule::pos_z(module_accessor);
                            let module_pos = Vector3f{x: x, y: y, z: z};
                            PostureModule::set_pos(boss_boma_2, &module_pos);

                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false {
                                CONTROLLABLE_2 = true;
                            }
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                        let x = PostureModule::pos_x(boss_boma);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(boss_boma);
                        CRAZY_X_POS = x;
                        CRAZY_Y_POS = y;
                        CRAZY_Z_POS = z;
                        if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                            let sub_hp = 999.0 - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                            CRAZY_FACING_RIGHT = true;
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                            CRAZY_FACING_RIGHT = false;
                        }
                        JostleModule::set_status(module_accessor, false);
                    }

                    // STUBS AI

                    if sv_information::is_ready_go() && !DEAD_2 {
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                            if CONTROLLABLE_2 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                            }
                        }
                    }

                    // Team Attack Trigger
                    if sv_information::is_ready_go() == true && !DEAD_2 {
                        let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                        let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                        if MotionModule::motion_kind(boss_boma_2) == hash40("wait") && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true {
                            if CONTROLLABLE_2 == false && smash::app::sv_math::rand(hash40("fighter"), 500) as f32 == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                            || CONTROLLABLE_2 && smash::app::sv_math::rand(hash40("fighter"), 900) as f32 == smash::app::sv_math::rand(hash40("fighter"), 900) as f32 {
                                if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                        CONTROLLABLE_2 = false;
                                        BARK = false;
                                        PUNCH = false;
                                        SHOCK = false;
                                        LASER = true;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                    }
                                }
                            }
                            else if CONTROLLABLE_2 == false && smash::app::sv_math::rand(hash40("fighter"), 500) as f32 == smash::app::sv_math::rand(hash40("fighter"), 500) as f32
                            || CONTROLLABLE_2 && smash::app::sv_math::rand(hash40("fighter"), 900) as f32 == smash::app::sv_math::rand(hash40("fighter"), 900) as f32 {
                                if MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                        CONTROLLABLE_2 = false;
                                        BARK = false;
                                        PUNCH = true;
                                        SHOCK = false;
                                        LASER = false;
                                        SCRATCH_BLOW = false;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                        MotionModule::change_motion(boss_boma_2,Hash40::new("taggoopaa"),0.0,1.0,false,0.0,false,false);
                                    }
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD_2 == true {
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                            if STOP_2 == false && CONFIG.options.boss_respawn.unwrap_or(false) && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) != *ITEM_STATUS_KIND_DEAD
                            || StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_DEAD
                            && MotionModule::frame(boss_boma_2) > 250.0 {
                                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                HitModule::set_whole(boss_boma_2, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                ItemModule::remove_all(module_accessor);
                                if STOP_2 == false && CONFIG.options.boss_respawn.unwrap_or(false) {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                                    STOP_2 = true;
                                }
                                if STOP_2 == false && !CONFIG.options.boss_respawn.unwrap_or(false) {
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
                        if sv_information::is_ready_go() == true {
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma_2) != *ITEM_STATUS_KIND_STANDBY {
                                    CRAZY_EXISTS = false;
                                    if lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 180.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma_2,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: 0.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma_2,&vec3,0);
                                    }
                                    if MotionModule::frame(boss_boma_2) == 0.0 {
                                        smash_script::macros::CAM_ZOOM_IN_arg5(fighter, 0.0, 0.0, 5.0, 0.0, 0.0);
                                        smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_dead"),true,false);
                                        smash_script::macros::EFFECT(fighter, Hash40::new("sys_bg_criticalhit"), Hash40::new("top"), 0,7,0,0,0,0,1,0,0,0,0,0,0,false);
                                        smash_script::macros::EFFECT(fighter, Hash40::new("sys_bg_boss_finishhit"), Hash40::new("top"), 0,7,0,0,0,0,1,0,0,0,0,0,0,false);
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
                                        smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_bg_criticalhit"),true,false);
                                        smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_bg_boss_finishhit"),true,false);
                                        SlowModule::clear_whole(module_accessor);
                                    }
                                    if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                        EXISTS_PUBLIC = false;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_STANDBY, true);
                                    }
                                }
                            }
                        }
                    }

                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if RESULT_SPAWNED_2 == false {
                            RESULT_SPAWNED_2 = true;
                            reset_mastercrazy_result_runtime();
                            let boss_boma_2 = acquire_crazy_hand_item(module_accessor, ENTRY_ID_2);
                            ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("entry"),0.0,1.0,false,0.0,false,false);
                        }
                        boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 && !DEAD_2 {
                        let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY && !MASTER_EXISTS {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY && MASTER_EXISTS {
                            CONTROLLABLE_2 = true;
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                            CRAZY_TEAM = TeamModule::team_no(module_accessor);
                            if MASTER_TEAM == CRAZY_TEAM {
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                MotionModule::change_motion(boss_boma_2,smash::phx::Hash40::new("entry2"),0.0,1.0,false,0.0,false,false);
                            }
                        }
                        if MotionModule::motion_kind(boss_boma_2) == smash::hash40("entry2") {
                            MotionModule::set_rate(boss_boma_2, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.5);
                        }
                    }

                    //DAMAGE MODULES

                    let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                    HitModule::set_whole(boss_boma_2, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma_2, i, false) {
                            AttackModule::set_target_category(boss_boma_2, i, *COLLISION_CATEGORY_MASK_ALL as u32);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if FighterUtil::is_hp_mode(module_accessor) == false {
                            let hp = CONFIG.options.crazy_hand_hp.unwrap_or(400.0);
                            if DamageModule::damage(module_accessor, 0) >= hp { // HEALTH
                                if DEAD_2 == false {
                                    CONTROLLABLE_2 = false;
                                    DEAD_2 = true;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_DEAD, true);
                                }
                            }
                        }
                    }

                    // SET FIGHTER LOOP

                    if sv_information::is_ready_go() == true {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ITEM);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_JUMP_AERIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_TREAD_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ITEM_THROW);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_WALL_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_CATCH);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_GUARD);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_CLIFF);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LASSO);
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            fighter.set_situation(SITUATION_KIND_AIR.into());
                            GroundModule::set_correct(module_accessor, smash::app::GroundCorrectKind(*GROUND_CORRECT_KIND_AIR));
                            MotionModule::change_motion(module_accessor,Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if DEAD_2 == false {
                        // SET POS AND STOPS OUT OF BOUNDS
                        if ModelModule::scale(module_accessor) == 0.0001 {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID_2[boss_helpers::entry_id(module_accessor)]);
                            if FighterUtil::is_hp_mode(module_accessor) == true {
                                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD
                                || StatusModule::status_kind(module_accessor) == 79 {
                                    if DEAD_2 == false {
                                        CONTROLLABLE_2 = false;
                                        DEAD_2 = true;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_DEAD, true);
                                    }
                                }
                            }
                            let x = PostureModule::pos_x(boss_boma);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(boss_boma);
                            let boss_pos = Vector3f{x: x, y: y + 20.0, z: z};
                            if !CONTROLLABLE_2 || boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true {
                                if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                    let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                    let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                    let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                }
                                else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                    let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    }
                                }
                                else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                            else {
                                if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                    let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                    let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                    let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                }
                                else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                    let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                }
                                else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                            let crazy_floor_clearance = if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU {
                                    CRAZY_NOTAUTSU_GROUND_CLEARANCE
                                } else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO {
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

                    if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                    || CONTROLLABLE_2 {
                        CRAZY_USABLE = true;
                    }
                    else {
                        CRAZY_USABLE = false;
                    }
                    if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true
                    && StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                        StatusModule::change_status_request_from_script(
                            boss_boma_2,
                            *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE,
                            true,
                        );
                    }
                    if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true {
                        maybe_recover_crazy_cpu_idle(boss_boma_2, ENTRY_ID_2);
                    }

                    if BARK && !DEAD_2 && MASTER_EXISTS && MotionModule::motion_kind(boss_boma_2) != smash::hash40("bark") && CRAZY_USABLE {
                        MotionModule::set_rate(boss_boma_2, 1.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        CONTROLLABLE_2 = false;
                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                        MotionModule::change_motion(boss_boma_2,Hash40::new("bark"),0.0,1.0,false,0.0,false,false);
                    }

                    if MotionModule::motion_kind(boss_boma_2) == hash40("bark") {
                        MotionModule::set_rate(boss_boma_2, 1.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            let master_pos = Vector3f{x: MASTER_X_POS + 30.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            let master_pos = Vector3f{x: MASTER_X_POS - 30.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                    }

                    if SCRATCH_BLOW && !DEAD_2 && MASTER_EXISTS && MotionModule::motion_kind(boss_boma_2) != smash::hash40("bark") && CRAZY_USABLE {
                        CONTROLLABLE_2 = false;
                        MotionModule::set_rate(boss_boma_2, 1.2);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                    }

                    if MotionModule::motion_kind(boss_boma_2) == smash::hash40("electroshock_start")
                    || MotionModule::motion_kind(boss_boma_2) == smash::hash40("electroshock")
                    || MotionModule::motion_kind(boss_boma_2) == smash::hash40("electroshock_end") {
                        MotionModule::set_rate(boss_boma_2, 1.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            let master_pos = Vector3f{x: MASTER_X_POS + 100.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            let master_pos = Vector3f{x: MASTER_X_POS - 100.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                    }

                    if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START {
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            let master_pos = Vector3f{x: MASTER_X_POS - 200.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            let master_pos = Vector3f{x: MASTER_X_POS + 200.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                    }

                    if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("teleport_end") && !DEAD_2 {
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true {
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE, true);
                        }
                        else {
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            CONTROLLABLE_2 = true;
                        }
                    }

                    if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 && MotionModule::motion_kind(boss_boma_2) == hash40("bark") && !DEAD_2 {
                        MotionModule::set_rate(boss_boma_2, 1.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true {
                            BARK = false;
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE, true);
                        }
                        else {
                            BARK = false;
                            CONTROLLABLE_2 = true;
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if SHOCK && !DEAD_2 && MASTER_EXISTS && MotionModule::motion_kind(boss_boma_2) != smash::hash40("electroshock_start")
                        && MotionModule::motion_kind(boss_boma_2) != smash::hash40("electroshock")
                        && MotionModule::motion_kind(boss_boma_2) != smash::hash40("electroshock_end")
                        && CRAZY_USABLE {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            CONTROLLABLE_2 = false;
                            let z = PostureModule::pos_z(boss_boma_2);
                            let module_pos = Vector3f{x: 50.0, y: 25.0, z: z};
                            PostureModule::set_pos(boss_boma_2, &module_pos);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("electroshock_start"),0.0,1.0,false,0.0,false,false);
                        }

                        if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("electroshock_start") {
                            MotionModule::change_motion(boss_boma_2,Hash40::new("electroshock"),0.0,1.0,false,0.0,false,false);
                        }

                        if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("electroshock") {
                            MotionModule::change_motion(boss_boma_2,Hash40::new("electroshock_end"),0.0,1.0,false,0.0,false,false);
                        }

                        if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("electroshock_end") && !DEAD_2 {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                CONTROLLABLE_2 = true;
                                SHOCK = false;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            }
                            else {
                                SHOCK = false;
                                MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE, true);
                            }
                        }
                    }

                    // FIXES SPAWN

                    if DEAD_2 == false {
                        if sv_information::is_ready_go() == true {
                            if JUMP_START_2 == false {
                                JUMP_START_2 = true;
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true {
                                    CONTROLLABLE_2 = false;
                                }
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true && !DEAD_2 {
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            let stunned = !CONFIG.options.full_stun_duration.unwrap_or(false);
                            if stunned {
                                StatusModule::change_status_request_from_script(boss_boma_2,*ITEM_CRAZYHAND_STATUS_KIND_DOWN_END,true);
                            }
                            CONTROLLABLE_2 = false;
                        }
                        if CONTROLLABLE_2 && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 0.75, y: CONTROLLER_Y_CRAZY * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START {
                            MotionModule::set_rate(boss_boma_2, 2.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.2);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false {
                            if StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START {
                                reset_crazy_fire_chariot_latches(ENTRY_ID_2);
                            } else {
                                if MotionModule::frame(boss_boma_2) >= 40.0
                                    && !CRAZY_FIRE_CHARIOT_PINKY_LATCH[ENTRY_ID_2]
                                {
                                    WorkModule::set_flag(boss_boma_2, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_PINKY);
                                    CRAZY_FIRE_CHARIOT_PINKY_LATCH[ENTRY_ID_2] = true;
                                }
                                if MotionModule::frame(boss_boma_2) >= 55.0
                                    && !CRAZY_FIRE_CHARIOT_THUMB_LATCH[ENTRY_ID_2]
                                {
                                    WorkModule::set_flag(boss_boma_2, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_THUMB);
                                    CRAZY_FIRE_CHARIOT_THUMB_LATCH[ENTRY_ID_2] = true;
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == 117 {
                            if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                PostureModule::set_pos(boss_boma_2, &Vector3f{x: 0.0, y: 20.0, z: 0.0});
                                lua_bind::PostureModule::set_lr(boss_boma_2, 1.0);
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_LOOP {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 2.0, y: CONTROLLER_Y_CRAZY * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }

                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                            WorkModule::set_float(boss_boma_2, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_X);
                            WorkModule::set_float(boss_boma_2, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Y);
                            WorkModule::set_float(boss_boma_2, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Z);
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 2.0, y: CONTROLLER_Y_CRAZY * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                            // Boss Control Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 0.5, y: CONTROLLER_Y_CRAZY * 0.5, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                        || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                        || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                        || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LANDING {
                            CONTROLLABLE_2 = false;
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_START {
                            MotionModule::set_rate(boss_boma_2, 1.175);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.175);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP {
                            MotionModule::set_rate(boss_boma_2, 1.7);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.7);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2 {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3 {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE {
                            MotionModule::set_rate(boss_boma_2, 4.75);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 4.75);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START {
                            MotionModule::set_rate(boss_boma_2, 1.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false {
                                PostureModule::set_pos(boss_boma_2, &Vector3f{x: PostureModule::pos_x(boss_boma_2), y: Y_POS_2, z: PostureModule::pos_z(boss_boma_2)});
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU || StatusModule::status_kind(boss_boma_2) == 84 || StatusModule::status_kind(boss_boma_2) == 85 {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false {
                                PostureModule::set_pos(boss_boma_2, &Vector3f{x: PostureModule::pos_x(boss_boma_2), y: Y_POS_2, z: PostureModule::pos_z(boss_boma_2)});
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI {
                            MotionModule::set_rate(boss_boma_2, 1.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_HOMING {
                            MotionModule::set_rate(boss_boma_2, 1.25);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.25);
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CENTER_MOVE {
                            MotionModule::set_rate(boss_boma_2, 4.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 4.4);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                            MotionModule::set_rate(boss_boma_2, 1.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP {
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DIG_END, true);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_ATTACK {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                MotionModule::set_rate(boss_boma_2, 4.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 4.0);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                MotionModule::set_rate(boss_boma_2, 2.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.2);
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                            MotionModule::set_rate(boss_boma_2, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.5);
                        }
                        if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                LASER = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                            if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                SCRATCH_BLOW = false;
                            }
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false {
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END {
                                CONTROLLABLE_2 = false;
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CANCEL {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME {
                                CONTROLLABLE_2 = true;
                            }
                            if MotionModule::motion_kind(boss_boma_2) == smash::hash40("wait") {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_RND_WAIT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TO_POINT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2 {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3 {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_END {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if MotionModule::motion_kind(boss_boma_2) == smash::hash40("teleport_start") && MotionModule::is_end(boss_boma_2) {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                MotionModule::change_motion(boss_boma_2,Hash40::new("teleport_end"),0.0,1.0,false,0.0,false,false);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false {
                                    if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                        MotionModule::set_rate(boss_boma_2, 1.0);
                                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                        CONTROLLABLE_2 = true;
                                    }
                                }
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == true {
                                    if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                        MotionModule::set_rate(boss_boma_2, 1.0);
                                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {
                                CONTROLLABLE_2 = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO_PRE_MOVE {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                        }

                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
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
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO {
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
                                    let next_y = if MotionModule::frame(boss_boma_2) < CRAZY_KUMO_DESCEND_FRAME {
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
                                        (MotionModule::end_frame(boss_boma_2) - CRAZY_KUMO_END_TAIL_FRAMES)
                                            .max(MotionModule::frame(boss_boma_2));
                                    MotionModule::set_frame(boss_boma_2, tail_start_frame, false);
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
                        if CONTROLLABLE_2 && StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                            MotionModule::set_rate(boss_boma_2, 1.4);
                        }
                        if MotionModule::frame(boss_boma_2) <= 0.0 && MotionModule::motion_kind(boss_boma_2) == hash40("teleport_end") {
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: -100.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if ControlModule::get_stick_x(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 100.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: 0.0, y: -50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 0.0, y: 50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                        }
                    }
                    if MotionModule::motion_kind(boss_boma_2) == smash::hash40("taggoopaa") {
                        MotionModule::set_rate(boss_boma_2, 1.3);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.3);
                        let x = PostureModule::pos_x(boss_boma_2);
                        CONTROLLABLE_2 = false;
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            if MotionModule::frame(boss_boma_2) >= 120.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: -0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if MotionModule::frame(boss_boma_2) >= 130.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    if x < MASTER_X_POS - 25.0 {
                                        let pos = Vector3f{x: 14.75, y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                }
                            }
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            if MotionModule::frame(boss_boma_2) >= 120.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: 0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if MotionModule::frame(boss_boma_2) >= 130.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    if x > MASTER_X_POS + 25.0 {
                                        let pos = Vector3f{x: -14.75, y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                }
                            }
                        }
                    }
                    if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("taggoopaa") && !DEAD_2 {
                        PUNCH = false;
                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID_2) == false && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                        if CONTROLLABLE_2 == true {
                            if DEAD_2 == false {
                                let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                                // Boss Control Movement
                                // X Controllable
                                if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.06 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.06 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                                //Boss Moves
                                if PostureModule::lr(boss_boma_2) == 1.0 { // right
                                    if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                if PostureModule::lr(boss_boma_2) == -1.0 { // left
                                    if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) && MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                        // if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                            CONTROLLABLE_2 = false;
                                            BARK = false;
                                            PUNCH = true;
                                            SHOCK = false;
                                            LASER = false;
                                            SCRATCH_BLOW = false;
                                            CONTROLLER_X_CRAZY = 0.0;
                                            CONTROLLER_Y_CRAZY = 0.0;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                            MotionModule::change_motion(boss_boma_2,Hash40::new("taggoopaa"),0.0,1.0,false,0.0,false,false);
                                        // }
                                        // else {
                                            // CONTROLLABLE_2 = false;
                                            // StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                            // set_camera_range(smash::phx::Vector4f{x: dead_range(fighter.lua_state_agent).x.abs() / 2.0, y: dead_range(fighter.lua_state_agent).y.abs() / 2.0, z: dead_range(fighter.lua_state_agent).z.abs() / 2.0, w: dead_range(fighter.lua_state_agent).w.abs()});
                                            // MotionModule::change_motion(boss_boma_2,Hash40::new("finder"),0.0,1.0,false,0.0,false,false);
                                        // }
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) && MotionModule::motion_kind(boss_boma_2) != smash::hash40("teleport_start") && MotionModule::motion_kind(boss_boma_2) != smash::hash40("teleport_end") && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    MotionModule::set_rate(boss_boma_2, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                    MotionModule::change_motion(boss_boma_2,Hash40::new("teleport_start"),0.0,1.0,false,0.0,false,false);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 30.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 5.0 {
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DIG_START, true);
                                    }
                                    else {
                                        Y_POS_2 = PostureModule::pos_y(boss_boma_2);
                                        StatusModule::change_status_request(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_READY, true);
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                    let floor_dist = boss_floor_dist(module_accessor, boss_boma_2);
                                    if floor_dist > 0.0 && floor_dist <= 50.0 {
                                            CONTROLLABLE_2 = false;
                                            CONTROLLER_X_CRAZY = 0.0;
                                            CONTROLLER_Y_CRAZY = 0.0;
                                            CRAZY_KUMO_ACTIVE = false;
                                            CRAZY_KUMO_ENDING = false;
                                            CRAZY_KUMO_START_Y = PostureModule::pos_y(boss_boma_2);
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_KUMO, true);
                                    } else {
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE, true);
                                    }
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                    Y_POS_2 = PostureModule::pos_y(boss_boma_2);
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                        if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                        || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                            CONTROLLABLE_2 = false;
                                            BARK = false;
                                            PUNCH = false;
                                            SHOCK = false;
                                            LASER = true;
                                            SCRATCH_BLOW = false;
                                            CONTROLLER_X_CRAZY = 0.0;
                                            CONTROLLER_Y_CRAZY = 0.0;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                        }
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                    if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 30.0
                                    && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                        CONTROLLABLE_2 = false;
                                        CONTROLLER_X_CRAZY = 0.0;
                                        CONTROLLER_Y_CRAZY = 0.0;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU, true);
                                    }
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                    CONTROLLABLE_2 = false;
                                    CONTROLLER_X_CRAZY = 0.0;
                                    CONTROLLER_Y_CRAZY = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START, true);
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
    once_per_fighter_frame(fighter);
}

pub unsafe fn crazy_frame(fighter: &mut L2CFighterCommon) {
    once_per_fighter_frame_2(fighter);
}

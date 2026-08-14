use crate::config::CONFIG;
use smash::app::lua_bind;
use smash::app::lua_bind::*;
use smash::app::sv_battle_object;
use smash::app::sv_information;
use smash::app::BattleObjectModuleAccessor;
use smash::app::FighterUtil;
use smash::app::ItemKind;
use smash::hash40;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Vector3f;
use std::u32;

use crate::boss_helpers;
use crate::boss_runtime::{self, BossCommonRuntime, CommonRuntimeSyncGuard};
use crate::selection;

static mut CONTROLLABLE: bool = true;
static mut IS_ANGRY: bool = false;
static mut ENTRY_ID: usize = 0;
static mut RANDOM_ATTACK: i32 = 0;
static mut BOSS_ID: [u32; 8] = [0; 8];
static mut DEAD: bool = false;
static mut JUMP_START: bool = false;
static mut RESULT_SPAWNED: bool = false;
static mut STOP: bool = false;
static mut EXISTS_PUBLIC: bool = false;
static mut CONTROLLER_X: f32 = 0.0;
static mut CONTROLLER_Y: f32 = 0.0;
static mut CONTROL_SPEED_MUL: f32 = 1.25;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;
static mut HIDDEN_CPU: [u32; 8] = [0; 8];
static mut PREVIEW_ITEM_ID: [u32; 8] = [0; 8];
static mut WOL_PREVIEW_BATTLE_STATE_RESET: [bool; 8] = [false; 8];
static mut SPAWN_BISECT_LAST_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut ENTRY_PHASE_LAST_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut STAGED_PREPARATION_ATTEMPTED: [bool; 8] = [false; 8];
static mut STAGED_BOSS_PREPARED: [bool; 8] = [false; 8];
static mut INITIAL_ACTIVATION_ATTEMPTED: [bool; 8] = [false; 8];
static mut WOL_PREVIEW_LAST_SIGNATURE: [u64; 8] = [u64::MAX; 8];
const WOL_PRESENTATION_SCALE: f32 = 0.05;

const DHARKON_FLOOR_CLEARANCE: f32 = 0.1;

/// Visual-only WOL and Amiibo presentations must stay on Dharkon's native
/// idle rather than entering any combat, movement, or summon status.
pub(crate) const PRESENTATION_IDLE_MOTION: &str = "wait";

#[inline(always)]
pub(crate) unsafe fn wol_presentation_item_kind() -> i32 {
    // The WOL map preview uses the CORE object, not the full boss.
    // Requesting ITEM_KIND_DARZ here crashed the game on hardware.
    *ITEM_KIND_DARZCENTIPEDE
}

extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
}

pub unsafe fn check_status() -> bool {
    EXISTS_PUBLIC || boss_runtime::any_exists_public(&raw const boss_runtime::DHARKON_RUNTIME)
}

#[inline(always)]
unsafe fn dharkon_should_clamp_floor(
    boss_boma: *mut smash::app::BattleObjectModuleAccessor,
) -> bool {
    if !CONTROLLABLE {
        return false;
    }
    let status = StatusModule::status_kind(boss_boma);
    status != *ITEM_DARZ_STATUS_KIND_DOWN_START
        && status != *ITEM_DARZ_STATUS_KIND_DOWN_LOOP
        && status != *ITEM_DARZ_STATUS_KIND_DOWN_END
}

#[inline(always)]
unsafe fn load_dharkon_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    CONTROLLABLE = (*slot).controllable;
    STOP = (*slot).stop;
    DEAD = (*slot).dead;
    JUMP_START = (*slot).jump_start;
    RESULT_SPAWNED = (*slot).result_spawned;
    EXISTS_PUBLIC = (*slot).exists_public;
    CONTROLLER_X = (*slot).controller_x;
    CONTROLLER_Y = (*slot).controller_y;
}

#[inline(always)]
unsafe fn store_dharkon_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    (*slot).controllable = CONTROLLABLE;
    (*slot).stop = STOP;
    (*slot).dead = DEAD;
    (*slot).jump_start = JUMP_START;
    (*slot).result_spawned = RESULT_SPAWNED;
    (*slot).exists_public = EXISTS_PUBLIC;
    (*slot).fresh_control = false;
    (*slot).controller_x = CONTROLLER_X;
    (*slot).controller_y = CONTROLLER_Y;
}

pub unsafe fn reset_match_state(entry_id: usize) {
    let entry = boss_runtime::sanitize_entry_id(entry_id);
    if crate::debug::enabled()
        && (BOSS_ID[entry] != 0
            || HIDDEN_CPU[entry] != 0
            || DEAD
            || RESULT_SPAWNED
            || STOP
            || EXISTS_PUBLIC)
    {
        crate::boss_log!(
            "[PB][Dharkon][Reset] entry={} tracked_id=0x{:x} hidden_cpu=0x{:x} controllable={} stop={} dead={} result_spawned={} exists_public={} jump_start={} angry={} controller=({:.2},{:.2})",
            entry,
            BOSS_ID[entry],
            HIDDEN_CPU[entry],
            core::ptr::addr_of!(CONTROLLABLE).read(),
            core::ptr::addr_of!(STOP).read(),
            core::ptr::addr_of!(DEAD).read(),
            core::ptr::addr_of!(RESULT_SPAWNED).read(),
            core::ptr::addr_of!(EXISTS_PUBLIC).read(),
            core::ptr::addr_of!(JUMP_START).read(),
            core::ptr::addr_of!(IS_ANGRY).read(),
            core::ptr::addr_of!(CONTROLLER_X).read(),
            core::ptr::addr_of!(CONTROLLER_Y).read()
        );
    }
    CONTROLLABLE = true;
    IS_ANGRY = false;
    ENTRY_ID = entry;
    RANDOM_ATTACK = 0;
    if BOSS_ID[entry] != 0 || HIDDEN_CPU[entry] != 0 || DEAD || EXISTS_PUBLIC {
        crate::boss_summon::log_boss_scene_exit("dharkon", entry, BOSS_ID[entry], "match_reset");
    }
    BOSS_ID[entry] = 0;
    DEAD = false;
    JUMP_START = false;
    RESULT_SPAWNED = false;
    STOP = false;
    EXISTS_PUBLIC = false;
    CONTROLLER_X = 0.0;
    CONTROLLER_Y = 0.0;
    CONTROL_SPEED_MUL = 1.25;
    CONTROL_SPEED_MUL_2 = 0.05;
    HIDDEN_CPU[entry] = 0;
    SPAWN_BISECT_LAST_SIGNATURE[entry] = u64::MAX;
    ENTRY_PHASE_LAST_SIGNATURE[entry] = u64::MAX;
    STAGED_PREPARATION_ATTEMPTED[entry] = false;
    STAGED_BOSS_PREPARED[entry] = false;
    INITIAL_ACTIVATION_ATTEMPTED[entry] = false;
    crate::boss_summon::reset("dharkon", entry, "match_reset");
}

#[inline(always)]
unsafe fn begin_wol_preview(entry: usize) {
    let entry = entry.min(7);
    if WOL_PREVIEW_BATTLE_STATE_RESET[entry] {
        return;
    }

    reset_match_state(entry);
    let runtime = boss_runtime::slot_ptr(&raw mut boss_runtime::DHARKON_RUNTIME, entry);
    if !runtime.is_null() {
        *runtime = BossCommonRuntime::new();
    }
    PREVIEW_ITEM_ID[entry] = 0;
    WOL_PREVIEW_LAST_SIGNATURE[entry] = u64::MAX;
    WOL_PREVIEW_BATTLE_STATE_RESET[entry] = true;
    crate::boss_log!(
        "[PB][Dharkon][WolPreview] action=battle_state_isolated entry={} battle_id=0x0 preview_id=0x0",
        entry
    );
}

#[inline(always)]
unsafe fn end_wol_preview(entry: usize) {
    let entry = entry.min(7);
    if !WOL_PREVIEW_BATTLE_STATE_RESET[entry] {
        return;
    }

    WOL_PREVIEW_BATTLE_STATE_RESET[entry] = false;
    PREVIEW_ITEM_ID[entry] = 0;
    WOL_PREVIEW_LAST_SIGNATURE[entry] = u64::MAX;
}

#[inline(always)]
unsafe fn discard_invalid_battle_tracking(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry: usize,
    ready_go: bool,
    transition_quarantined: bool,
) {
    let entry = entry.min(7);
    let tracked_id = BOSS_ID[entry];
    if tracked_id == 0 || transition_quarantined {
        return;
    }

    let expected_kind_active =
        boss_helpers::tracked_item_by_kind(&raw const BOSS_ID, entry, *ITEM_KIND_DARZ).is_some();
    if !boss_helpers::should_discard_tracked_boss(
        ready_go,
        transition_quarantined,
        expected_kind_active,
        STAGED_BOSS_PREPARED[entry],
    ) {
        return;
    }

    let active = sv_battle_object::is_active(tracked_id);
    let actual_kind = if active {
        let tracked_boma = sv_battle_object::module_accessor(tracked_id);
        if tracked_boma.is_null() {
            -1
        } else {
            smash::app::utility::get_kind(&mut *tracked_boma)
        }
    } else {
        -1
    };
    if !ready_go {
        boss_helpers::clear_owned_boss_item_slot(
            module_accessor,
            &raw mut BOSS_ID,
            &[*ITEM_KIND_DARZ, *ITEM_KIND_DARZCENTIPEDE],
            true,
        );
        EXISTS_PUBLIC = false;
        STAGED_BOSS_PREPARED[entry] = false;
        INITIAL_ACTIVATION_ATTEMPTED[entry] = false;
    }
    BOSS_ID[entry] = 0;
    crate::boss_log!(
        "[PB][Dharkon][StartupGate] action=discard_stale_tracking entry={} reason={} tracked_id=0x{:x} object_active={} actual_kind={} expected_kind={} ready_go={}",
        entry,
        if ready_go {
            "invalid_object_or_kind"
        } else {
            "pre_ready_go_object_forbidden"
        },
        tracked_id,
        active,
        actual_kind,
        *ITEM_KIND_DARZ,
        ready_go
    );
}

#[inline(always)]
unsafe fn is_tracked_dharkon_active(entry: usize) -> bool {
    boss_helpers::tracked_item_by_kind(&raw const BOSS_ID, entry, *ITEM_KIND_DARZ).is_some()
}

/// Feed the shared, read-only match-end audit without exposing Dharkon's item
/// ownership to the transition code.  The audit deliberately accepts a phase
/// from the caller so the post-match boundary can be recorded after Ready-Go
/// ends without touching stale item accessors.
pub unsafe fn audit_transition(
    module_accessor: *mut BattleObjectModuleAccessor,
    phase: &'static str,
    allow_object_reads: bool,
) {
    if module_accessor.is_null() {
        return;
    }
    let entry = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
    if BOSS_ID[entry] == 0 && HIDDEN_CPU[entry] == 0 && !DEAD && !EXISTS_PUBLIC {
        return;
    }
    crate::boss_summon::log_result_roster_helper(
        phase,
        "dharkon_hidden_cpu_item",
        HIDDEN_CPU[entry],
        allow_object_reads,
    );
    crate::boss_summon::audit_match_end(
        "dharkon",
        entry,
        module_accessor,
        BOSS_ID[entry],
        HIDDEN_CPU[entry],
        DEAD,
        EXISTS_PUBLIC,
        phase,
        allow_object_reads,
    );
}

#[inline(always)]
unsafe fn log_dharkon_entry_phase(
    tag: &str,
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_active: bool,
    stage_one_prepared: bool,
    stage_two_prepared: bool,
) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }
    let entry = boss_helpers::entry_id(module_accessor).min(7);
    let tracked_id = BOSS_ID[entry];
    let hidden_cpu_id = HIDDEN_CPU[entry];
    let tracked_active = boss_active;
    let tracked_status = if tracked_active {
        let tracked_boma = sv_battle_object::module_accessor(tracked_id);
        if tracked_boma.is_null() {
            -1
        } else {
            StatusModule::status_kind(tracked_boma)
        }
    } else {
        -1
    };
    let hidden_cpu_active = hidden_cpu_id != 0 && sv_battle_object::is_active(hidden_cpu_id);
    let hidden_cpu_status = if hidden_cpu_active {
        let hidden_cpu_boma = sv_battle_object::module_accessor(hidden_cpu_id);
        if hidden_cpu_boma.is_null() {
            -1
        } else {
            StatusModule::status_kind(hidden_cpu_boma)
        }
    } else {
        -1
    };
    let mut tag_signature = 0u64;
    for byte in tag.as_bytes() {
        tag_signature = tag_signature.rotate_left(5) ^ (*byte as u64);
    }
    let signature = tag_signature
        ^ (tracked_id as u64).rotate_left(7)
        ^ (hidden_cpu_id as u64).rotate_left(19)
        ^ (tracked_status as u32 as u64).rotate_left(31)
        ^ (hidden_cpu_status as u32 as u64).rotate_left(43)
        ^ (stage_one_prepared as u64).rotate_left(3)
        ^ (stage_two_prepared as u64).rotate_left(4)
        ^ (STAGED_PREPARATION_ATTEMPTED[entry] as u64).rotate_left(53)
        ^ (STAGED_BOSS_PREPARED[entry] as u64).rotate_left(54)
        ^ (sv_information::is_ready_go() as u64).rotate_left(5);
    if ENTRY_PHASE_LAST_SIGNATURE[entry] == signature {
        return;
    }
    ENTRY_PHASE_LAST_SIGNATURE[entry] = signature;
    crate::boss_log!(
        "[PB][Dharkon][Phase] tag={} entry={} stage=0x{:x} ready_go={} fighter_status={} frame={:.2} scale={:.4} tracked_id=0x{:x} tracked_active={} tracked_status={} hidden_cpu=0x{:x} hidden_cpu_active={} hidden_cpu_status={} stage1={} stage2={} preparation_attempted={} staged_boss_prepared={} exists_public={} controllable={} dead={} stop={} result_spawned={}",
        tag,
        entry,
        smash::app::stage::get_stage_id(),
        sv_information::is_ready_go(),
        StatusModule::status_kind(module_accessor),
        MotionModule::frame(module_accessor),
        ModelModule::scale(module_accessor),
        tracked_id,
        tracked_active,
        tracked_status,
        hidden_cpu_id,
        hidden_cpu_active,
        hidden_cpu_status,
        stage_one_prepared,
        stage_two_prepared,
        STAGED_PREPARATION_ATTEMPTED[entry],
        STAGED_BOSS_PREPARED[entry],
        core::ptr::addr_of!(EXISTS_PUBLIC).read(),
        core::ptr::addr_of!(CONTROLLABLE).read(),
        core::ptr::addr_of!(DEAD).read(),
        core::ptr::addr_of!(STOP).read(),
        core::ptr::addr_of!(RESULT_SPAWNED).read()
    );
}

/// Bounded breadcrumbs for the first native item spawn.  This deliberately
/// reads only the already-valid host accessor and the acquired pointer's null
/// state; it does not walk arbitrary battle objects while isolating a crash.
#[inline(always)]
unsafe fn log_dharkon_spawn_bisect(
    entry: usize,
    step: u8,
    edge: &'static str,
    name: &'static str,
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }
    let entry = entry.min(7);
    let edge_code = if edge == "begin" { 1u64 } else { 2u64 };
    let signature = (step as u64)
        ^ edge_code.rotate_left(8)
        ^ (BOSS_ID[entry] as u64).rotate_left(16)
        ^ (HIDDEN_CPU[entry] as u64).rotate_left(32);
    if SPAWN_BISECT_LAST_SIGNATURE[entry] == signature {
        return;
    }
    SPAWN_BISECT_LAST_SIGNATURE[entry] = signature;
    crate::boss_log!(
        "[PB][DharkonSpawnBisect] step={:02} edge={} name={} entry={} host_status={} tracked_id=0x{:x} hidden_cpu=0x{:x} boss_boma_present={}",
        step,
        edge,
        name,
        entry,
        StatusModule::status_kind(module_accessor),
        BOSS_ID[entry],
        HIDDEN_CPU[entry],
        !boss_boma.is_null()
    );
}

#[inline(always)]
unsafe fn log_dharkon_spawn_state(
    tag: &str,
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }
    let entry = boss_helpers::entry_id(module_accessor).min(7);
    let hidden_cpu_id = HIDDEN_CPU[entry];
    let hidden_cpu_active = hidden_cpu_id != 0 && sv_battle_object::is_active(hidden_cpu_id);
    let hidden_cpu_boma = if hidden_cpu_active {
        sv_battle_object::module_accessor(hidden_cpu_id)
    } else {
        std::ptr::null_mut()
    };
    let hidden_cpu_status = if hidden_cpu_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(hidden_cpu_boma)
    };
    let boss_status = if boss_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(boss_boma)
    };
    crate::boss_log!(
        "[PB][Dharkon][SpawnState] tag={} entry={} stage=0x{:x} ready_go={} host_status={} host_scale={:.4} host_pos=({:.2},{:.2},{:.2}) tracked_id=0x{:x} boss_status={} boss_pos=({:.2},{:.2},{:.2}) hidden_cpu=0x{:x} hidden_cpu_active={} hidden_cpu_status={} hidden_pos=({:.2},{:.2},{:.2})",
        tag,
        entry,
        smash::app::stage::get_stage_id(),
        sv_information::is_ready_go(),
        StatusModule::status_kind(module_accessor),
        ModelModule::scale(module_accessor),
        PostureModule::pos_x(module_accessor),
        PostureModule::pos_y(module_accessor),
        PostureModule::pos_z(module_accessor),
        BOSS_ID[entry],
        boss_status,
        if boss_boma.is_null() { 0.0 } else { PostureModule::pos_x(boss_boma) },
        if boss_boma.is_null() { 0.0 } else { PostureModule::pos_y(boss_boma) },
        if boss_boma.is_null() { 0.0 } else { PostureModule::pos_z(boss_boma) },
        hidden_cpu_id,
        hidden_cpu_active,
        hidden_cpu_status,
        if hidden_cpu_boma.is_null() { 0.0 } else { PostureModule::pos_x(hidden_cpu_boma) },
        if hidden_cpu_boma.is_null() { 0.0 } else { PostureModule::pos_y(hidden_cpu_boma) },
        if hidden_cpu_boma.is_null() { 0.0 } else { PostureModule::pos_z(hidden_cpu_boma) }
    );
}

/// Create Dharkon while item construction is still safe, but keep the object in
/// the same inert presentation state used by the Amiibo/WOL viewers until the
/// match reaches Ready-Go.
unsafe fn prepare_staged_dharkon_before_ready_go(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() || sv_information::is_ready_go() {
        return false;
    }

    let entry = boss_helpers::entry_id(module_accessor).min(7);
    if STAGED_PREPARATION_ATTEMPTED[entry] {
        return STAGED_BOSS_PREPARED[entry] && is_tracked_dharkon_active(entry);
    }
    STAGED_PREPARATION_ATTEMPTED[entry] = true;

    let hidden_cpu_id = HIDDEN_CPU[entry];
    if hidden_cpu_id == 0 || !sv_battle_object::is_active(hidden_cpu_id) {
        crate::boss_log!(
            "[PB][Dharkon][StartupGate] action=prepare_failed entry={} reason=hidden_cpu_inactive hidden_cpu=0x{:x}",
            entry,
            hidden_cpu_id
        );
        return false;
    }

    log_dharkon_spawn_bisect(
        entry,
        2,
        "begin",
        "pre_ready_go_prepare",
        module_accessor,
        std::ptr::null_mut(),
    );
    DamageModule::heal(module_accessor, -999.0, 0);
    ItemModule::throw_item(module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
    log_dharkon_spawn_bisect(
        entry,
        3,
        "after",
        "hidden_cpu_detached",
        module_accessor,
        std::ptr::null_mut(),
    );
    log_dharkon_spawn_bisect(
        entry,
        4,
        "begin",
        "pre_ready_go_have_item",
        module_accessor,
        std::ptr::null_mut(),
    );
    let boss_boma = boss_helpers::acquire_boss_item_excluding(
        module_accessor,
        &raw mut BOSS_ID,
        *ITEM_KIND_DARZ,
        hidden_cpu_id,
    );
    let boss_id = BOSS_ID[entry];
    let acquired_kind = if boss_boma.is_null() {
        -1
    } else {
        smash::app::utility::get_kind(&mut *boss_boma)
    };
    if boss_boma.is_null() || acquired_kind != *ITEM_KIND_DARZ {
        BOSS_ID[entry] = 0;
        STAGED_BOSS_PREPARED[entry] = false;
        crate::boss_log!(
            "[PB][Dharkon][StartupGate] action=prepare_failed entry={} reason=invalid_acquired_boss acquired_id=0x{:x} acquired_kind={}",
            entry,
            boss_id,
            acquired_kind
        );
        return false;
    }

    ModelModule::set_scale(boss_boma, boss_helpers::HIDDEN_HOST_SCALE);
    MotionModule::change_motion(
        boss_boma,
        smash::phx::Hash40::new(PRESENTATION_IDLE_MOTION),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    boss_helpers::maintain_nonbattle_boss_presentation(boss_boma);
    STAGED_BOSS_PREPARED[entry] = true;
    log_dharkon_spawn_bisect(
        entry,
        5,
        "after",
        "pre_ready_go_prepared",
        module_accessor,
        boss_boma,
    );
    crate::boss_log!(
        "[PB][Dharkon][StartupGate] action=prepared_inert entry={} ready_go=false hidden_cpu=0x{:x} boss_id=0x{:x} boss_kind={} boss_status={} motion=wait",
        entry,
        hidden_cpu_id,
        boss_id,
        acquired_kind,
        StatusModule::status_kind(boss_boma)
    );
    true
}

#[inline(always)]
unsafe fn maintain_staged_dharkon_before_ready_go(entry: usize) {
    let entry = entry.min(7);
    if sv_information::is_ready_go() || !STAGED_BOSS_PREPARED[entry] {
        return;
    }
    let Some((_, boss_boma)) =
        boss_helpers::tracked_item_by_kind(&raw const BOSS_ID, entry, *ITEM_KIND_DARZ)
    else {
        BOSS_ID[entry] = 0;
        STAGED_BOSS_PREPARED[entry] = false;
        crate::boss_log!(
            "[PB][Dharkon][StartupGate] action=prepare_lost entry={} retry=false",
            entry
        );
        return;
    };

    if (ModelModule::scale(boss_boma) - boss_helpers::HIDDEN_HOST_SCALE).abs() > f32::EPSILON {
        ModelModule::set_scale(boss_boma, boss_helpers::HIDDEN_HOST_SCALE);
    }
    if MotionModule::motion_kind(boss_boma) != smash::hash40(PRESENTATION_IDLE_MOTION) {
        MotionModule::change_motion(
            boss_boma,
            smash::phx::Hash40::new(PRESENTATION_IDLE_MOTION),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
    }
    boss_helpers::maintain_nonbattle_boss_presentation(boss_boma);
}

/// Activate only the verified object prepared before Ready-Go. Item creation at
/// this boundary crashes on hardware and must never be retried here.
unsafe fn activate_staged_dharkon_after_ready_go(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() || !sv_information::is_ready_go() {
        return false;
    }

    let entry = boss_helpers::entry_id(module_accessor).min(7);
    if INITIAL_ACTIVATION_ATTEMPTED[entry] {
        return false;
    }
    INITIAL_ACTIVATION_ATTEMPTED[entry] = true;

    if !STAGED_BOSS_PREPARED[entry] {
        crate::boss_log!(
            "[PB][Dharkon][StartupGate] action=activate_failed entry={} reason=staged_boss_not_prepared",
            entry
        );
        return false;
    }

    let hidden_cpu_id = HIDDEN_CPU[entry];
    if hidden_cpu_id == 0 || !sv_battle_object::is_active(hidden_cpu_id) {
        crate::boss_log!(
            "[PB][Dharkon][StartupGate] action=activate_failed entry={} reason=hidden_cpu_inactive hidden_cpu=0x{:x}",
            entry,
            hidden_cpu_id
        );
        return false;
    }

    let Some((boss_id, boss_boma)) =
        boss_helpers::tracked_item_by_kind(&raw const BOSS_ID, entry, *ITEM_KIND_DARZ)
    else {
        BOSS_ID[entry] = 0;
        STAGED_BOSS_PREPARED[entry] = false;
        crate::boss_log!(
            "[PB][Dharkon][StartupGate] action=activate_failed entry={} reason=prepared_boss_invalid",
            entry
        );
        return false;
    };

    log_dharkon_spawn_bisect(
        entry,
        10,
        "begin",
        "ready_go_activate_prepared",
        module_accessor,
        boss_boma,
    );
    DEAD = false;
    CONTROLLABLE = true;
    JUMP_START = false;
    STOP = false;
    RESULT_SPAWNED = false;
    CONTROLLER_X = 0.0;
    CONTROLLER_Y = 0.0;
    DamageModule::heal(module_accessor, -999.0, 0);
    EXISTS_PUBLIC = true;
    ModelModule::set_scale(boss_boma, 1.0);
    DamageModule::set_damage_lock(boss_boma, false);
    JostleModule::set_status(boss_boma, true);
    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);
    let boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
    WorkModule::set_float(boss_boma, boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
    WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    ModelModule::set_scale(module_accessor, boss_helpers::HIDDEN_HOST_SCALE);
    StatusModule::change_status_request_from_script(
        boss_boma,
        *ITEM_STATUS_KIND_FOR_BOSS_START,
        true,
    );
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    WorkModule::set_int(
        boss_boma,
        *ITEM_VARIATION_DARZ_KIILA,
        *ITEM_INSTANCE_WORK_INT_VARIATION,
    );
    STAGED_BOSS_PREPARED[entry] = false;

    log_dharkon_spawn_bisect(
        entry,
        12,
        "after",
        "ready_go_activate_prepared",
        module_accessor,
        boss_boma,
    );
    log_dharkon_spawn_state("initial_ready_go", module_accessor, boss_boma);
    crate::boss_log!(
        "[PB][Dharkon][StartupGate] action=activated entry={} ready_go=true hidden_cpu=0x{:x} boss_id=0x{:x} boss_status={}",
        entry,
        hidden_cpu_id,
        boss_id,
        StatusModule::status_kind(boss_boma)
    );
    true
}

#[inline(always)]
unsafe fn log_dharkon_wol_preview(
    entry: usize,
    phase: u8,
    action: &'static str,
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }
    let entry = entry.min(7);
    let boss_id = PREVIEW_ITEM_ID[entry];
    let signature = (phase as u64)
        ^ (boss_id as u64).rotate_left(12)
        ^ (ModelModule::scale(module_accessor).to_bits() as u64).rotate_left(29)
        ^ ((!boss_boma.is_null() as u64) << 61);
    if WOL_PREVIEW_LAST_SIGNATURE[entry] == signature {
        return;
    }
    WOL_PREVIEW_LAST_SIGNATURE[entry] = signature;
    crate::boss_log!(
        "[PB][Dharkon][WolPreview] phase={} action={} entry={} host_valid=true selected={} host_scale={:.4} boss_id=0x{:x} boss_present={}",
        phase,
        action,
        entry,
        selection::is_selected_css_boss(module_accessor, *ITEM_KIND_DARZ),
        ModelModule::scale(module_accessor),
        boss_id,
        !boss_boma.is_null()
    );
}

/// WOL constructs its hidden Mario preview host after UI selection. Only then
/// create the visual Dharkon item; Regular Smash summon/recovery/transition
/// state is deliberately excluded from this presentation-only path.
#[inline(always)]
unsafe fn ensure_wol_dharkon_preview(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry: usize,
) {
    if module_accessor.is_null() {
        return;
    }

    let entry = entry.min(7);
    if !selection::is_selected_css_boss(module_accessor, *ITEM_KIND_DARZ) {
        end_wol_preview(entry);
        log_dharkon_wol_preview(
            entry,
            1,
            "not_selected",
            module_accessor,
            std::ptr::null_mut(),
        );
        return;
    }

    begin_wol_preview(entry);
    boss_helpers::clear_hidden_host_effects(module_accessor);
    let expected_kind = wol_presentation_item_kind();
    let mut presentation =
        boss_helpers::tracked_item_by_kind(&raw const PREVIEW_ITEM_ID, entry, expected_kind);
    if presentation.is_none() {
        log_dharkon_wol_preview(
            entry,
            2,
            "create_begin",
            module_accessor,
            std::ptr::null_mut(),
        );
        ItemModule::remove_all(module_accessor);
        ModelModule::set_scale(module_accessor, boss_helpers::HIDDEN_HOST_SCALE);
        let boss_boma = boss_helpers::acquire_boss_item(
            module_accessor,
            &raw mut PREVIEW_ITEM_ID,
            expected_kind,
        );
        presentation =
            boss_helpers::tracked_item_by_kind(&raw const PREVIEW_ITEM_ID, entry, expected_kind);
        if boss_boma.is_null() || presentation.is_none() {
            PREVIEW_ITEM_ID[entry] = 0;
            log_dharkon_wol_preview(entry, 3, "create_failed", module_accessor, boss_boma);
            return;
        }
        ModelModule::set_scale(boss_boma, WOL_PRESENTATION_SCALE);
        MotionModule::change_motion(
            boss_boma,
            smash::phx::Hash40::new(PRESENTATION_IDLE_MOTION),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
        log_dharkon_wol_preview(entry, 4, "create_complete", module_accessor, boss_boma);
    }

    if let Some((_, preview_boma)) = presentation {
        if (ModelModule::scale(preview_boma) - WOL_PRESENTATION_SCALE).abs() > f32::EPSILON {
            ModelModule::set_scale(preview_boma, WOL_PRESENTATION_SCALE);
        }
        if MotionModule::motion_kind(preview_boma) != smash::hash40(PRESENTATION_IDLE_MOTION) {
            MotionModule::change_motion(
                preview_boma,
                smash::phx::Hash40::new(PRESENTATION_IDLE_MOTION),
                0.0,
                1.0,
                false,
                0.0,
                false,
                false,
            );
        }
        boss_helpers::maintain_nonbattle_boss_presentation(preview_boma);
    }

    if ModelModule::scale(module_accessor) == boss_helpers::HIDDEN_HOST_SCALE {
        MotionModule::change_motion(
            module_accessor,
            smash::phx::Hash40::new("none"),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
        PostureModule::set_rot(
            module_accessor,
            &Vector3f {
                x: -180.0,
                y: 90.0,
                z: 0.0,
            },
            0,
        );
        ModelModule::set_joint_rotate(
            module_accessor,
            smash::phx::Hash40::new("root"),
            &mut Vector3f {
                x: 90.0,
                y: 50.0,
                z: 0.0,
            },
            smash::app::MotionNodeRotateCompose {
                _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
            },
            ModelModule::rotation_order(module_accessor),
        );
        PostureModule::set_pos(
            module_accessor,
            &Vector3f {
                x: PostureModule::pos_x(module_accessor),
                y: 7.25,
                z: PostureModule::pos_z(module_accessor) + 3.0,
            },
        );
    }

    let tracked_boma = presentation
        .map(|(_, preview_boma)| preview_boma)
        .unwrap_or(std::ptr::null_mut());
    log_dharkon_wol_preview(entry, 5, "ready", module_accessor, tracked_boma);
}

#[inline(always)]
unsafe fn restore_dharkon_after_item_wipe(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null()
        || !sv_information::is_ready_go()
        || DEAD
        || crate::any_post_match_pre_result()
    {
        return;
    }
    let fighter_manager = boss_helpers::fighter_manager();
    if !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager) {
        return;
    }

    let entry = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
    ENTRY_ID = entry;
    let hidden_cpu_id = HIDDEN_CPU[entry];
    let tracked_active = is_tracked_dharkon_active(entry);
    let hidden_cpu_active = hidden_cpu_id != 0 && sv_battle_object::is_active(hidden_cpu_id);
    let staged_initial_sequence = !tracked_active
        && hidden_cpu_active
        && !EXISTS_PUBLIC
        && (boss_helpers::is_hidden_host_entry_prep(module_accessor)
            || boss_helpers::is_hidden_host_entry_stage_two(module_accessor));
    let failed_initial_activation =
        INITIAL_ACTIVATION_ATTEMPTED[entry] && !tracked_active && !EXISTS_PUBLIC;
    if staged_initial_sequence || failed_initial_activation {
        return;
    }
    if tracked_active && hidden_cpu_active {
        return;
    }

    ItemModule::remove_all(module_accessor);
    ItemModule::have_item(
        module_accessor,
        ItemKind(*ITEM_KIND_DRACULA2),
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
    HIDDEN_CPU[entry] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
    let hidden_cpu_boma = sv_battle_object::module_accessor(HIDDEN_CPU[entry]);
    if hidden_cpu_boma.is_null() {
        return;
    }
    ModelModule::set_scale(hidden_cpu_boma, 0.0001);
    ItemModule::throw_item(module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);

    let hidden_cpu_id = HIDDEN_CPU[entry];
    let boss_boma = boss_helpers::acquire_boss_item_excluding(
        module_accessor,
        &raw mut BOSS_ID,
        *ITEM_KIND_DARZ,
        hidden_cpu_id,
    );
    if boss_boma.is_null() || smash::app::utility::get_kind(&mut *boss_boma) != *ITEM_KIND_DARZ {
        BOSS_ID[entry] = 0;
        EXISTS_PUBLIC = false;
        crate::boss_log!(
            "[PB][Dharkon][Recover] action=acquire_failed entry={} expected_kind={}",
            entry,
            *ITEM_KIND_DARZ
        );
        return;
    }
    let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
    WorkModule::set_float(
        boss_boma,
        get_boss_intensity,
        *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
    );
    WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    WorkModule::set_int(
        boss_boma,
        *ITEM_VARIATION_DARZ_KIILA,
        *ITEM_INSTANCE_WORK_INT_VARIATION,
    );
    ModelModule::set_scale(module_accessor, 0.0001);
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(module_accessor),
        y: PostureModule::pos_y(module_accessor),
        z: PostureModule::pos_z(module_accessor),
    };
    PostureModule::set_pos(boss_boma, &boss_pos);
    PostureModule::set_pos(hidden_cpu_boma, &boss_pos);
    DamageModule::set_damage_lock(hidden_cpu_boma, true);
    JostleModule::set_status(hidden_cpu_boma, false);
    WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
    WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    if StatusModule::status_kind(hidden_cpu_boma) != *ITEM_STATUS_KIND_NONE {
        StatusModule::change_status_request_from_script(
            hidden_cpu_boma,
            *ITEM_STATUS_KIND_NONE,
            true,
        );
    }
    StatusModule::change_status_request_from_script(
        boss_boma,
        *ITEM_DARZ_STATUS_KIND_MANAGER_WAIT,
        true,
    );
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
    EXISTS_PUBLIC = true;
    RESULT_SPAWNED = false;
    crate::boss_log!(
        "[PB][Recover] entry {}: restored Dharkon after item wipe tracked_id=0x{:x} hidden_cpu=0x{:x} tracked_active={} hidden_cpu_active={}",
        entry,
        BOSS_ID[entry],
        HIDDEN_CPU[entry],
        is_tracked_dharkon_active(entry),
        HIDDEN_CPU[entry] != 0 && sv_battle_object::is_active(HIDDEN_CPU[entry])
    );
}

#[inline(always)]
unsafe fn teardown_dharkon_post_match_transition(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }

    let entry = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
    let tracked_id = BOSS_ID[entry];
    let hidden_cpu_id = HIDDEN_CPU[entry];
    let tracked_active = tracked_id != 0 && sv_battle_object::is_active(tracked_id);
    let hidden_cpu_active = hidden_cpu_id != 0 && sv_battle_object::is_active(hidden_cpu_id);
    if !tracked_active && !hidden_cpu_active && !EXISTS_PUBLIC {
        return false;
    }

    if tracked_active {
        let boss_boma = sv_battle_object::module_accessor(tracked_id);
        if !boss_boma.is_null() {
            HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
            SlowModule::clear_whole(boss_boma);
            StatusModule::change_status_request_from_script(
                boss_boma,
                *ITEM_STATUS_KIND_STANDBY,
                true,
            );
        }
    }

    if hidden_cpu_active {
        let hidden_cpu_boma = sv_battle_object::module_accessor(hidden_cpu_id);
        if !hidden_cpu_boma.is_null() {
            HitModule::set_whole(hidden_cpu_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
            SlowModule::clear_whole(hidden_cpu_boma);
            if StatusModule::status_kind(hidden_cpu_boma) != *ITEM_STATUS_KIND_NONE {
                StatusModule::change_status_request_from_script(
                    hidden_cpu_boma,
                    *ITEM_STATUS_KIND_NONE,
                    true,
                );
            }
        }
    }

    ItemModule::remove_all(module_accessor);
    boss_helpers::clear_hidden_host_effects(module_accessor);
    boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
    ModelModule::set_scale(module_accessor, boss_helpers::HIDDEN_HOST_SCALE);
    MotionModule::change_motion(
        module_accessor,
        smash::phx::Hash40::new("none"),
        0.0,
        1.0,
        false,
        0.0,
        false,
        false,
    );
    reset_match_state(entry);
    CONTROLLABLE = false;

    crate::boss_log!(
        "[PB][Dharkon][Cleanup] entry {}: cleared Dharkon runtime on non-ready_go transition tracked_active={} hidden_cpu_active={}",
        entry,
        tracked_active,
        hidden_cpu_active
    );

    true
}

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            ENTRY_ID = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
            let stage_id = smash::app::stage::get_stage_id();
            if boss_helpers::is_world_of_light_boss_preview_stage(stage_id) {
                ensure_wol_dharkon_preview(module_accessor, ENTRY_ID);
                return;
            }
            end_wol_preview(ENTRY_ID);
            let _runtime_guard = CommonRuntimeSyncGuard::new(
                boss_runtime::slot_ptr(&raw mut boss_runtime::DHARKON_RUNTIME, ENTRY_ID),
                load_dharkon_runtime,
                store_dharkon_runtime,
            );
            let fighter_manager = boss_helpers::fighter_manager();
            let result_mode =
                !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
            let ready_go = sv_information::is_ready_go();
            let post_match_transition = crate::any_post_match_pre_result();
            discard_invalid_battle_tracking(
                module_accessor,
                ENTRY_ID,
                ready_go,
                result_mode || post_match_transition,
            );
            if result_mode {
                audit_transition(module_accessor, "result_ready", false);
            } else if ready_go {
                audit_transition(module_accessor, "battle", true);
            }
            if result_mode {
                // Dharkon remains gated out of custom result presentation until
                // its Regular Smash death lifecycle is stable. The native scene
                // owns its battle objects here.
                return;
            }

            // The central transition guard owns the post-match/pre-result
            // boundary.  Once Ready-Go has ended, do not observe, recover, or
            // reacquire Dharkon's summon/item state while native teardown is
            // still moving the hidden host toward Results.
            if post_match_transition {
                return;
            }

            let summon_id = BOSS_ID[ENTRY_ID];
            let summon_boma = if summon_id != 0 && sv_battle_object::is_active(summon_id) {
                sv_battle_object::module_accessor(summon_id)
            } else {
                std::ptr::null_mut()
            };
            // No summon can exist during the hidden-host entry sequence.  Do
            // not traverse FighterManager/child topology before Ready-Go;
            // that diagnostic path is intentionally battle-only.
            if sv_information::is_ready_go() {
                crate::boss_summon::observe_native(
                    "dharkon",
                    ENTRY_ID,
                    summon_id,
                    summon_boma,
                    *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER,
                    *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER_WAIT,
                );
            }

            let selected_via_slot =
                selection::is_selected_css_boss(module_accessor, *ITEM_KIND_DARZ);
            if !selected_via_slot
                && !sv_information::is_ready_go()
                && (BOSS_ID[ENTRY_ID] != 0 || HIDDEN_CPU[ENTRY_ID] != 0 || EXISTS_PUBLIC)
            {
                teardown_dharkon_post_match_transition(module_accessor);
                return;
            }
            if selected_via_slot {
                boss_helpers::clear_hidden_host_effects(module_accessor);
                if boss_helpers::is_boss_preview_stage(stage_id) {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor =
                        smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001
                        || !ItemModule::is_have_item(module_accessor, 0)
                    {
                        ItemModule::remove_all(module_accessor);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = boss_helpers::acquire_boss_item(
                            module_accessor,
                            &raw mut PREVIEW_ITEM_ID,
                            *ITEM_KIND_DARZCENTIPEDE,
                        );
                        ModelModule::set_scale(boss_boma, 0.05);
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
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(
                            module_accessor,
                            smash::phx::Hash40::new("none"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                        PostureModule::set_rot(
                            module_accessor,
                            &Vector3f {
                                x: -180.0,
                                y: 90.0,
                                z: 0.0,
                            },
                            0,
                        );
                        ModelModule::set_joint_rotate(
                            module_accessor,
                            smash::phx::Hash40::new("root"),
                            &mut Vector3f {
                                x: 90.0,
                                y: 50.0,
                                z: 0.0,
                            },
                            smash::app::MotionNodeRotateCompose {
                                _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
                            },
                            ModelModule::rotation_order(module_accessor),
                        );
                        PostureModule::set_pos(
                            module_accessor,
                            &Vector3f {
                                x: PostureModule::pos_x(module_accessor),
                                y: 7.25,
                                z: PostureModule::pos_z(module_accessor) + 3.0,
                            },
                        );
                    }
                } else if !boss_helpers::is_boss_passthrough_stage(stage_id) {
                    restore_dharkon_after_item_wipe(module_accessor);
                    if sv_information::is_ready_go() == false {
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor =
                            smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(
                            module_accessor,
                            *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                        ) as usize;
                        let boss_active = is_tracked_dharkon_active(ENTRY_ID);
                        let stage_one_prepared =
                            boss_helpers::is_hidden_host_entry_prep(module_accessor);
                        let stage_two_prepared =
                            boss_helpers::is_hidden_host_entry_stage_two(module_accessor);
                        log_dharkon_entry_phase(
                            "pre_ready_go_gate",
                            module_accessor,
                            boss_active,
                            stage_one_prepared,
                            stage_two_prepared,
                        );
                        if !boss_active && !stage_one_prepared && !stage_two_prepared {
                            DEAD = false;
                            CONTROLLABLE = true;
                            JUMP_START = false;
                            IS_ANGRY = false;
                            STOP = false;
                            CONTROLLER_X = 0.0;
                            CONTROLLER_Y = 0.0;
                            EXISTS_PUBLIC = false;
                            RESULT_SPAWNED = false;
                            if BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0 {
                                boss_helpers::clear_owned_boss_item_slot(
                                    module_accessor,
                                    &raw mut BOSS_ID,
                                    &[*ITEM_KIND_DARZ, *ITEM_KIND_DARZCENTIPEDE],
                                    true,
                                );
                            }
                        }
                        if smash::app::smashball::is_training_mode() == false {
                            if !boss_active && !stage_one_prepared && !stage_two_prepared {
                                ModelModule::set_scale(
                                    module_accessor,
                                    boss_helpers::HIDDEN_HOST_ENTRY_PREP_SCALE,
                                );
                                ItemModule::have_item(
                                    module_accessor,
                                    ItemKind(*ITEM_KIND_DRACULA2),
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
                                HIDDEN_CPU[boss_helpers::entry_id(module_accessor)] =
                                    ItemModule::get_have_item_id(module_accessor, 0) as u32;
                                let hidden_cpu_boma = sv_battle_object::module_accessor(
                                    HIDDEN_CPU[boss_helpers::entry_id(module_accessor)],
                                );
                                if hidden_cpu_boma.is_null() {
                                    HIDDEN_CPU[boss_helpers::entry_id(module_accessor)] = 0;
                                    ModelModule::set_scale(
                                        module_accessor,
                                        boss_helpers::HIDDEN_HOST_SCALE,
                                    );
                                } else {
                                    ModelModule::set_scale(
                                        hidden_cpu_boma,
                                        boss_helpers::HIDDEN_HOST_SCALE,
                                    );
                                }
                                log_dharkon_entry_phase(
                                    "stage1_prepare",
                                    module_accessor,
                                    false,
                                    true,
                                    false,
                                );
                            }
                            if MotionModule::frame(module_accessor) >= 2.0
                                && !boss_active
                                && stage_one_prepared
                            {
                                ModelModule::set_scale(
                                    module_accessor,
                                    boss_helpers::HIDDEN_HOST_ENTRY_STAGE2_SCALE,
                                );
                                log_dharkon_entry_phase(
                                    "stage2_prepare",
                                    module_accessor,
                                    false,
                                    false,
                                    true,
                                );
                            }
                            if MotionModule::frame(module_accessor) >= 5.0
                                && !boss_active
                                && boss_helpers::is_hidden_host_entry_stage_two(module_accessor)
                                && !STAGED_PREPARATION_ATTEMPTED[ENTRY_ID]
                            {
                                prepare_staged_dharkon_before_ready_go(module_accessor);
                            }
                            maintain_staged_dharkon_before_ready_go(ENTRY_ID);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        let hidden_cpu_id = HIDDEN_CPU[boss_helpers::entry_id(module_accessor)];
                        let hidden_cpu_boma =
                            if hidden_cpu_id != 0 && sv_battle_object::is_active(hidden_cpu_id) {
                                sv_battle_object::module_accessor(hidden_cpu_id)
                            } else {
                                std::ptr::null_mut()
                            };
                        if !hidden_cpu_boma.is_null() {
                            DamageModule::set_damage_lock(hidden_cpu_boma, true);
                            JostleModule::set_status(hidden_cpu_boma, false);
                            WorkModule::set_float(
                                hidden_cpu_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
                            );
                            WorkModule::set_float(
                                hidden_cpu_boma,
                                0.0,
                                *ITEM_INSTANCE_WORK_FLOAT_STRENGTH,
                            );
                            WorkModule::set_float(
                                hidden_cpu_boma,
                                999.0,
                                *ITEM_INSTANCE_WORK_FLOAT_HP_MAX,
                            );
                            WorkModule::set_float(
                                hidden_cpu_boma,
                                999.0,
                                *ITEM_INSTANCE_WORK_FLOAT_HP,
                            );
                            if StatusModule::status_kind(hidden_cpu_boma) != *ITEM_STATUS_KIND_NONE
                            {
                                StatusModule::change_status_request_from_script(
                                    hidden_cpu_boma,
                                    *ITEM_STATUS_KIND_NONE,
                                    true,
                                );
                            }
                        }
                        let tracked_id = BOSS_ID[boss_helpers::entry_id(module_accessor)];
                        if tracked_id != 0
                            && sv_battle_object::is_active(tracked_id)
                            && !hidden_cpu_boma.is_null()
                        {
                            let boss_boma = sv_battle_object::module_accessor(tracked_id);
                            if !boss_boma.is_null() {
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f { x: x, y: y, z: z };
                                PostureModule::set_pos(hidden_cpu_boma, &boss_pos);
                            }
                        }
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
                            JUMP_START = false;
                            IS_ANGRY = false;
                            STOP = false;
                            CONTROLLER_X = 0.0;
                            CONTROLLER_Y = 0.0;
                            DamageModule::heal(module_accessor, -999.0, 0);
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor =
                                smash::app::sv_system::battle_object_module_accessor(lua_state);
                            ENTRY_ID = WorkModule::get_int(
                                module_accessor,
                                *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                            ) as usize;
                            ItemModule::have_item(
                                module_accessor,
                                ItemKind(*ITEM_KIND_DRACULA2),
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
                            HIDDEN_CPU[boss_helpers::entry_id(module_accessor)] =
                                ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let hidden_cpu_boma = sv_battle_object::module_accessor(
                                HIDDEN_CPU[boss_helpers::entry_id(module_accessor)],
                            );
                            if !hidden_cpu_boma.is_null() {
                                ModelModule::set_scale(hidden_cpu_boma, 0.0001);
                            }
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                            ItemModule::throw_item(
                                fighter.module_accessor,
                                0.0,
                                0.0,
                                0.0,
                                0,
                                true,
                                0.0,
                            );
                            let hidden_cpu_id = HIDDEN_CPU[boss_helpers::entry_id(module_accessor)];
                            let boss_boma = boss_helpers::acquire_boss_item_excluding(
                                module_accessor,
                                &raw mut BOSS_ID,
                                *ITEM_KIND_DARZ,
                                hidden_cpu_id,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                get_boss_intensity,
                                *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                1.0,
                                *ITEM_INSTANCE_WORK_FLOAT_STRENGTH,
                            );
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_DARZ_STATUS_KIND_TELEPORT,
                                true,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                999.0,
                                *ITEM_INSTANCE_WORK_FLOAT_HP_MAX,
                            );
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            WorkModule::set_int(
                                boss_boma,
                                *ITEM_VARIATION_DARZ_KIILA,
                                *ITEM_INSTANCE_WORK_INT_VARIATION,
                            );
                            println!(
                                "[PB][Dharkon][Spawn] rebirth hidden_cpu=0x{:x} boss_id=0x{:x} status={}",
                                hidden_cpu_id,
                                BOSS_ID[boss_helpers::entry_id(module_accessor)],
                                StatusModule::status_kind(boss_boma),
                            );
                            log_dharkon_spawn_state("rebirth", module_accessor, boss_boma);

                            let x = PostureModule::pos_x(module_accessor);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(module_accessor);
                            let module_pos = Vector3f { x: x, y: y, z: z };
                            PostureModule::set_pos(boss_boma, &module_pos);
                            CONTROLLABLE = false;
                        }
                    }

                    if sv_information::is_ready_go()
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        if !boss_boma.is_null() {
                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                // left
                                let vec3 = Vector3f {
                                    x: 0.0,
                                    y: 90.0,
                                    z: 0.0,
                                };
                                PostureModule::set_rot(boss_boma, &vec3, 0);
                            }
                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                // right
                                let vec3 = Vector3f {
                                    x: 0.0,
                                    y: -90.0,
                                    z: 0.0,
                                };
                                PostureModule::set_rot(boss_boma, &vec3, 0);
                            }
                        }
                    }

                    if DEAD == false {
                        if sv_information::is_ready_go() == true
                            && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                        {
                            let boss_boma = sv_battle_object::module_accessor(
                                BOSS_ID[boss_helpers::entry_id(module_accessor)],
                            );
                            if !boss_boma.is_null()
                                && StatusModule::status_kind(boss_boma)
                                    == *ITEM_DARZ_STATUS_KIND_DOWN_LOOP
                            {
                                let stunned = !CONFIG.options.full_stun_duration.unwrap_or(false);
                                if stunned {
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_DOWN_END,
                                        true,
                                    );
                                }
                                CONTROLLABLE = false;
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
                                smash::phx::Hash40::new("fall"),
                                0.0,
                                1.0,
                                false,
                                0.0,
                                false,
                                false,
                            );
                        }
                    }

                    let staged_initial_sequence = {
                        let entry = boss_helpers::entry_id(module_accessor).min(7);
                        let hidden_cpu_id = HIDDEN_CPU[entry];
                        boss_helpers::staged_boss_ready_for_activation(
                            STAGED_BOSS_PREPARED[entry],
                            is_tracked_dharkon_active(entry),
                            INITIAL_ACTIVATION_ATTEMPTED[entry],
                            hidden_cpu_id != 0 && sv_battle_object::is_active(hidden_cpu_id),
                        )
                    };
                    let respawn_enabled = smash::app::smashball::is_training_mode()
                        || CONFIG.options.boss_respawn.unwrap_or(false);
                    if sv_information::is_ready_go() && (staged_initial_sequence || respawn_enabled)
                    {
                        let boss_active = is_tracked_dharkon_active(ENTRY_ID);
                        let stage_one_prepared =
                            boss_helpers::is_hidden_host_entry_prep(module_accessor);
                        let stage_two_prepared =
                            boss_helpers::is_hidden_host_entry_stage_two(module_accessor);
                        log_dharkon_entry_phase(
                            "ready_go_gate",
                            module_accessor,
                            boss_active,
                            stage_one_prepared,
                            stage_two_prepared,
                        );
                        if respawn_enabled
                            && !boss_active
                            && !stage_one_prepared
                            && !stage_two_prepared
                        {
                            DEAD = false;
                            CONTROLLABLE = true;
                            JUMP_START = false;
                            IS_ANGRY = false;
                            STOP = false;
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor =
                                smash::app::sv_system::battle_object_module_accessor(lua_state);
                            ENTRY_ID = WorkModule::get_int(
                                module_accessor,
                                *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                            ) as usize;
                            ModelModule::set_scale(
                                module_accessor,
                                boss_helpers::HIDDEN_HOST_ENTRY_PREP_SCALE,
                            );
                            log_dharkon_entry_phase(
                                "ready_go_stage1",
                                module_accessor,
                                false,
                                true,
                                false,
                            );
                        }
                        if !boss_active && stage_one_prepared {
                            ModelModule::set_scale(
                                module_accessor,
                                boss_helpers::HIDDEN_HOST_ENTRY_STAGE2_SCALE,
                            );
                            log_dharkon_entry_phase(
                                "ready_go_stage2",
                                module_accessor,
                                false,
                                false,
                                true,
                            );
                        }
                        if staged_initial_sequence || (stage_two_prepared && !boss_active) {
                            log_dharkon_entry_phase(
                                "ready_go_spawn_start",
                                module_accessor,
                                boss_active,
                                false,
                                stage_two_prepared,
                            );
                            if staged_initial_sequence {
                                activate_staged_dharkon_after_ready_go(module_accessor);
                            } else {
                                RESULT_SPAWNED = false;
                                let boss_boma = boss_helpers::acquire_boss_item(
                                    module_accessor,
                                    &raw mut BOSS_ID,
                                    *ITEM_KIND_DARZ,
                                );
                                if boss_boma.is_null() {
                                    return;
                                }
                                let get_boss_intensity =
                                    CONFIG.options.boss_difficulty.unwrap_or(10.0);
                                WorkModule::set_float(
                                    boss_boma,
                                    get_boss_intensity,
                                    *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
                                );
                                WorkModule::set_float(
                                    boss_boma,
                                    1.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_STRENGTH,
                                );
                                WorkModule::set_int(
                                    boss_boma,
                                    *ITEM_TRAIT_FLAG_BOSS,
                                    *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG,
                                );
                                WorkModule::set_float(
                                    boss_boma,
                                    999.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_HP_MAX,
                                );
                                WorkModule::set_float(
                                    boss_boma,
                                    999.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_HP,
                                );
                                WorkModule::set_int(
                                    boss_boma,
                                    *ITEM_VARIATION_DARZ_KIILA,
                                    *ITEM_INSTANCE_WORK_INT_VARIATION,
                                );
                                ModelModule::set_scale(
                                    module_accessor,
                                    boss_helpers::HIDDEN_HOST_SCALE,
                                );
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_STATUS_KIND_FOR_BOSS_START,
                                    true,
                                );
                                println!(
                                    "[PB][Dharkon][Spawn] ready_go boss_id=0x{:x} status={}",
                                    BOSS_ID[boss_helpers::entry_id(module_accessor)],
                                    StatusModule::status_kind(boss_boma),
                                );
                                log_dharkon_spawn_state("ready_go", module_accessor, boss_boma);
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
                        if !boss_boma.is_null() {
                            if !JUMP_START {
                                if DamageModule::damage(module_accessor, 0) > 0.0 {
                                    DamageModule::heal(module_accessor, -999.0, 0);
                                }
                                WorkModule::set_float(
                                    boss_boma,
                                    999.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_HP,
                                );
                            } else if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP)
                                != 999.0
                            {
                                let sub_hp = 999.0
                                    - WorkModule::get_float(
                                        boss_boma,
                                        *ITEM_INSTANCE_WORK_FLOAT_HP,
                                    );
                                DamageModule::add_damage(module_accessor, sub_hp, 0);
                                WorkModule::set_float(
                                    boss_boma,
                                    999.0,
                                    *ITEM_INSTANCE_WORK_FLOAT_HP,
                                );
                            }
                            if CONTROLLABLE {
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
                        }
                        JostleModule::set_status(module_accessor, false);
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
                        && is_tracked_dharkon_active(ENTRY_ID)
                    {
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        if !boss_boma.is_null()
                            && (StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("entry2"))
                        {
                            MotionModule::set_rate(boss_boma, 7.0);
                        }
                    }

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            // SET POS AND STOPS OUT OF BOUNDS
                            if ModelModule::scale(module_accessor) == 0.0001
                                && is_tracked_dharkon_active(ENTRY_ID)
                            {
                                let boss_boma = sv_battle_object::module_accessor(
                                    BOSS_ID[boss_helpers::entry_id(module_accessor)],
                                );
                                if FighterUtil::is_hp_mode(module_accessor) == true
                                    && !boss_boma.is_null()
                                {
                                    if StatusModule::status_kind(module_accessor)
                                        == *FIGHTER_STATUS_KIND_DEAD
                                        || StatusModule::status_kind(module_accessor) == 79
                                    {
                                        if DEAD == false {
                                            CONTROLLABLE = false;
                                            DEAD = true;
                                            audit_transition(module_accessor, "battle", true);
                                            crate::boss_summon::cancel_for_entry(
                                                "dharkon",
                                                ENTRY_ID,
                                                "boss_eliminated",
                                            );
                                            StatusModule::change_status_request_from_script(
                                                boss_boma,
                                                *ITEM_STATUS_KIND_DEAD,
                                                true,
                                            );
                                        }
                                    }
                                }
                                if StatusModule::status_kind(module_accessor)
                                    != *FIGHTER_STATUS_KIND_STANDBY
                                {
                                    let x = PostureModule::pos_x(boss_boma);
                                    let y = PostureModule::pos_y(boss_boma);
                                    let z = PostureModule::pos_z(boss_boma);
                                    let boss_pos = Vector3f { x: x, y: y, z: z };
                                    if !CONTROLLABLE
                                        || boss_helpers::is_operation_cpu_entry(
                                            fighter_manager,
                                            ENTRY_ID,
                                        ) == true
                                    {
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: x,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            if PostureModule::pos_x(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0
                                            {
                                                let boss_x_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_1,
                                                );
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0
                                            {
                                                let boss_x_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_2,
                                                );
                                            }
                                        } else if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            if PostureModule::pos_x(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0
                                            {
                                                let boss_x_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: y,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_2,
                                                );
                                            }
                                            if PostureModule::pos_y(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0
                                            {
                                                let boss_y_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_1,
                                                );
                                            }
                                            if PostureModule::pos_y(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0
                                            {
                                                let boss_y_pos_2 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_2,
                                                );
                                            }
                                        } else if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            if PostureModule::pos_y(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0
                                            {
                                                let boss_y_pos_1 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_1,
                                                );
                                            }
                                            if PostureModule::pos_y(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0
                                            {
                                                let boss_y_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_2,
                                                );
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0
                                            {
                                                let boss_x_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: y,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_1,
                                                );
                                            }
                                        } else if PostureModule::pos_y(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                        {
                                            let boss_y_pos_1 = Vector3f {
                                                x: x,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            if PostureModule::pos_y(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0
                                            {
                                                let boss_y_pos_2 = Vector3f {
                                                    x: x,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_2,
                                                );
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0
                                            {
                                                let boss_x_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_1,
                                                );
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0
                                            {
                                                let boss_x_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_2,
                                                );
                                            }
                                        } else {
                                            PostureModule::set_pos(module_accessor, &boss_pos);
                                        }
                                    } else {
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: x,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                            if PostureModule::pos_x(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0
                                            {
                                                let boss_x_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_1,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0
                                            {
                                                let boss_x_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_2,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                            }
                                        } else if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                            if PostureModule::pos_x(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0
                                            {
                                                let boss_x_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: y,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_2,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                            }
                                            if PostureModule::pos_y(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0
                                            {
                                                let boss_y_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_1,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                            }
                                            if PostureModule::pos_y(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0
                                            {
                                                let boss_y_pos_2 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_2,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                            }
                                        } else if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                            if PostureModule::pos_y(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0
                                            {
                                                let boss_y_pos_1 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_1,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                            }
                                            if PostureModule::pos_y(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0
                                            {
                                                let boss_y_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_2,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0
                                            {
                                                let boss_x_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: y,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_1,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                            }
                                        } else if PostureModule::pos_y(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                        {
                                            let boss_y_pos_1 = Vector3f {
                                                x: x,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                            if PostureModule::pos_y(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0
                                            {
                                                let boss_y_pos_2 = Vector3f {
                                                    x: x,
                                                    y: (dead_range(fighter.lua_state_agent)
                                                        .y
                                                        .abs()
                                                        * -1.0)
                                                        + 160.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_y_pos_2,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                >= dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0
                                            {
                                                let boss_x_pos_1 = Vector3f {
                                                    x: dead_range(fighter.lua_state_agent).x.abs()
                                                        - 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_1,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                            }
                                            if PostureModule::pos_x(boss_boma)
                                                <= (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0
                                            {
                                                let boss_x_pos_2 = Vector3f {
                                                    x: (dead_range(fighter.lua_state_agent)
                                                        .x
                                                        .abs()
                                                        * -1.0)
                                                        + 100.0,
                                                    y: dead_range(fighter.lua_state_agent).y.abs()
                                                        - 100.0,
                                                    z: z,
                                                };
                                                PostureModule::set_pos(
                                                    module_accessor,
                                                    &boss_x_pos_2,
                                                );
                                                PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                            }
                                        } else {
                                            PostureModule::set_pos(module_accessor, &boss_pos);
                                        }
                                    }
                                    if dharkon_should_clamp_floor(boss_boma) {
                                        boss_helpers::clamp_flying_boss_floor(
                                            module_accessor,
                                            boss_boma,
                                            DHARKON_FLOOR_CLEARANCE,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true
                        && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0
                    {
                        // DAMAGE MODULES
                        let boss_boma = sv_battle_object::module_accessor(
                            BOSS_ID[boss_helpers::entry_id(module_accessor)],
                        );
                        HitModule::set_whole(
                            module_accessor,
                            smash::app::HitStatus(*HIT_STATUS_OFF),
                            0,
                        );
                        if !boss_boma.is_null() {
                            HitModule::set_whole(
                                boss_boma,
                                smash::app::HitStatus(*HIT_STATUS_NORMAL),
                                0,
                            );
                            for i in 0..10 {
                                if AttackModule::is_attack(boss_boma, i, false) {
                                    AttackModule::set_target_category(
                                        boss_boma,
                                        i,
                                        *COLLISION_CATEGORY_MASK_ALL as u32,
                                    );
                                }
                            }
                        }
                        if sv_information::is_ready_go() == true {
                            if FighterUtil::is_hp_mode(module_accessor) == false {
                                let hp = CONFIG.options.dharkon_hp.unwrap_or(400.0);
                                if JUMP_START && DamageModule::damage(module_accessor, 0) >= hp {
                                    if DEAD == false {
                                        CONTROLLABLE = false;
                                        DEAD = true;
                                        audit_transition(module_accessor, "battle", true);
                                        crate::boss_summon::cancel_for_entry(
                                            "dharkon",
                                            ENTRY_ID,
                                            "boss_eliminated",
                                        );
                                        if !boss_boma.is_null() {
                                            StatusModule::change_status_request_from_script(
                                                boss_boma,
                                                *ITEM_STATUS_KIND_DEAD,
                                                true,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // DEATH CHECK

                        if sv_information::is_ready_go() == true {
                            if DEAD == true {
                                HitModule::set_whole(
                                    module_accessor,
                                    smash::app::HitStatus(*HIT_STATUS_OFF),
                                    0,
                                );
                                let death_boss_id =
                                    BOSS_ID[boss_helpers::entry_id(module_accessor)];
                                if death_boss_id != 0 && sv_battle_object::is_active(death_boss_id)
                                {
                                    let death_boss_boma =
                                        sv_battle_object::module_accessor(death_boss_id);
                                    if !death_boss_boma.is_null() {
                                        HitModule::set_whole(
                                            death_boss_boma,
                                            smash::app::HitStatus(*HIT_STATUS_OFF),
                                            0,
                                        );
                                    }
                                }
                                // The native DEAD request above owns summon
                                // cleanup. The shared state machine gives it
                                // exactly three callbacks, then permits the
                                // legacy fallback once if the parent remains
                                // active in the expected DEAD status.
                                let cleanup_action = crate::boss_summon::parent_death_cleanup_step(
                                    "dharkon",
                                    ENTRY_ID,
                                    death_boss_id,
                                );
                                match cleanup_action {
                                    crate::boss_summon::ParentDeathCleanupAction::RunFallback => {
                                        ItemModule::remove_all(module_accessor);
                                    }
                                    crate::boss_summon::ParentDeathCleanupAction::Complete => {
                                        BOSS_ID[boss_helpers::entry_id(module_accessor)] = 0;
                                        EXISTS_PUBLIC = false;
                                    }
                                    crate::boss_summon::ParentDeathCleanupAction::Defer
                                    | crate::boss_summon::ParentDeathCleanupAction::Abort => {}
                                }
                                if STOP == false
                                    && smash::app::smashball::is_training_mode() == false
                                {
                                    boss_helpers::request_hidden_host_stock_drain(
                                        module_accessor,
                                        fighter_manager,
                                        ENTRY_ID,
                                        &raw mut STOP,
                                    );
                                }
                                if STOP == true
                                    && smash::app::smashball::is_training_mode() == false
                                {
                                    if StatusModule::status_kind(module_accessor)
                                        == *FIGHTER_STATUS_KIND_REBIRTH
                                    {
                                        StatusModule::change_status_request_from_script(
                                            module_accessor,
                                            *FIGHTER_STATUS_KIND_STANDBY,
                                            true,
                                        );
                                    }
                                }
                            }
                        }

                        // ItemModule::remove_all above can destroy the native
                        // boss object. Never reuse the pre-removal pointer;
                        // reacquire only after an active-object check.
                        let post_death_boss_id = BOSS_ID[boss_helpers::entry_id(module_accessor)];
                        if DEAD == true
                            && sv_information::is_ready_go() == true
                            && post_death_boss_id != 0
                            && sv_battle_object::is_active(post_death_boss_id)
                        {
                            let post_death_boma =
                                sv_battle_object::module_accessor(post_death_boss_id);
                            if !post_death_boma.is_null()
                                && (StatusModule::status_kind(post_death_boma)
                                    == *ITEM_STATUS_KIND_DEAD
                                    || MotionModule::motion_kind(post_death_boma)
                                        == smash::hash40("dead"))
                                && StatusModule::status_kind(post_death_boma)
                                    == *ITEM_STATUS_KIND_STANDBY
                                && MotionModule::frame(post_death_boma)
                                    >= MotionModule::end_frame(post_death_boma)
                            {
                                EXISTS_PUBLIC = false;
                                StatusModule::change_status_request_from_script(
                                    post_death_boma,
                                    *ITEM_STATUS_KIND_STANDBY,
                                    true,
                                );
                            }
                        }

                        // FIXES SPAWN

                        if DEAD == false {
                            if JUMP_START == false {
                                JUMP_START = true;
                                CONTROLLABLE = false;
                                DamageModule::heal(module_accessor, -999.0, 0);
                                if !boss_boma.is_null() {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                        // left
                                        let vec3 = Vector3f {
                                            x: 0.0,
                                            y: 90.0,
                                            z: 0.0,
                                        };
                                        PostureModule::set_rot(boss_boma, &vec3, 0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                        // right
                                        let vec3 = Vector3f {
                                            x: 0.0,
                                            y: -90.0,
                                            z: 0.0,
                                        };
                                        PostureModule::set_rot(boss_boma, &vec3, 0);
                                    }
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_MANAGER_WAIT,
                                        true,
                                    );
                                    println!(
                                        "[PB][Dharkon][Spawn] jump_start boss_id=0x{:x} status={}",
                                        BOSS_ID[boss_helpers::entry_id(module_accessor)],
                                        StatusModule::status_kind(boss_boma),
                                    );
                                    log_dharkon_spawn_state(
                                        "jump_start",
                                        module_accessor,
                                        boss_boma,
                                    );
                                }
                            }
                        }

                        // BUILT IN BOSS AI
                        if !boss_boma.is_null() {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                == true
                            {
                                if DEAD == false {
                                    if CONTROLLABLE == true {
                                        if MotionModule::frame(fighter.module_accessor)
                                            >= smash::app::sv_math::rand(hash40("fighter"), 59)
                                                as f32
                                        {
                                            RANDOM_ATTACK =
                                                smash::app::sv_math::rand(hash40("fighter"), 12);
                                            if RANDOM_ATTACK == 0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_CROSS_BOMB,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 1 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_TELEPORT,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 2 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_TEAR_UP_START,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 3 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_PIERCE_START,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 4 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_CENTIPEDE_START,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 5 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_START,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 6 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_TEAR_UP_START,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 7 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_DARK_PILLAR_START,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 8 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_GATLING_START,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 9 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_CHASE_HAMMER,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 10 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_TORRENT,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 11 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_MANAGER_VANISH,
                                                    true,
                                                );
                                            }
                                            if RANDOM_ATTACK == 12 {
                                                CONTROLLABLE = false;
                                                crate::boss_summon::request_native(
                                                    "dharkon",
                                                    ENTRY_ID,
                                                    BOSS_ID[ENTRY_ID],
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER,
                                                    "cpu_random",
                                                );
                                                StatusModule::change_status_request_from_script(
                                                    boss_boma,
                                                    *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER,
                                                    true,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            let rage_hp = CONFIG.options.dharkon_rage_hp.unwrap_or(220.0);
                            if DamageModule::damage(module_accessor, 0) >= rage_hp && !DEAD {
                                if IS_ANGRY == false {
                                    CONTROLLABLE = false;
                                    IS_ANGRY = true;
                                    DamageModule::add_damage(module_accessor, 1.1, 0);
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_CHANGE_ANGRY,
                                        true,
                                    );
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE = true;
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
                            }
                            if MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WARP {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_DOWN_START
                            {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_MANAGER_WAIT
                            {
                                CONTROLLABLE = true;
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
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_MANAGER_VANISH
                            {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == 63 && !CONTROLLABLE {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_DARZ_STATUS_KIND_TELEPORT,
                                    true,
                                );
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER_WAIT
                            {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_DOWN_LOOP
                            {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_DOWN_END
                            {
                                CONTROLLABLE = false;
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_LOST
                                && !DEAD
                            {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_DARZ_STATUS_KIND_TELEPORT,
                                    true,
                                );
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_TEAR_UP_ANGER
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X,
                                    y: CONTROLLER_Y,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_TEAR_UP
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X,
                                    y: CONTROLLER_Y,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_LOOP
                            {
                                CONTROLLABLE = false;
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X,
                                    y: CONTROLLER_Y,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_DARK_PILLAR_END
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_GATLING_LOOP
                            {
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X,
                                    y: CONTROLLER_Y,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_GATLING_HOLD_LOOP
                            {
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X,
                                    y: CONTROLLER_Y,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_CHASE_HAMMER
                            {
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X,
                                    y: CONTROLLER_Y,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma) == 68 {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma)
                                == *ITEM_DARZ_STATUS_KIND_TEAR_UP
                            {
                                if MotionModule::frame(boss_boma)
                                    >= MotionModule::end_frame(boss_boma) - 10.0
                                {
                                    CONTROLLABLE = true;
                                }
                            }
                            // println!("{}", StatusModule::status_kind(boss_boma));
                            if CONTROLLABLE == true
                                && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                                    == false
                                && !DEAD
                            {
                                //Boss Control Stick Movement

                                // X Controllable
                                if CONTROLLER_X
                                    < ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X >= 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X
                                    > ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_X <= 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && CONTROLLER_X != 0.0
                                    && ControlModule::get_stick_x(module_accessor) == 0.0
                                {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                    CONTROLLER_X = 0.0;
                                }
                                if CONTROLLER_X > 0.0
                                    && ControlModule::get_stick_x(module_accessor) < 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0
                                    && ControlModule::get_stick_x(module_accessor) > 0.0
                                {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y
                                    < ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y >= 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y
                                    > ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL
                                    && CONTROLLER_Y <= 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && CONTROLLER_Y != 0.0
                                    && ControlModule::get_stick_y(module_accessor) == 0.0
                                {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                    CONTROLLER_Y = 0.0;
                                }
                                if CONTROLLER_Y > 0.0
                                    && ControlModule::get_stick_y(module_accessor) < 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0
                                    && ControlModule::get_stick_y(module_accessor) > 0.0
                                {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)
                                        * CONTROL_SPEED_MUL)
                                        * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f {
                                    x: CONTROLLER_X,
                                    y: CONTROLLER_Y,
                                    z: 0.0,
                                };
                                PostureModule::add_pos(boss_boma, &pos);

                                //Boss Moves
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_SPECIAL,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_CROSS_BOMB,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_GUARD,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_TELEPORT,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_ATTACK,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_TEAR_UP_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_PIERCE_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_CENTIPEDE_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_TEAR_UP_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_DARK_PILLAR_START,
                                        true,
                                    );
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                                    & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3
                                    != 0
                                {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_GATLING_START,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_APPEAL_HI,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_CHASE_HAMMER,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_APPEAL_LW,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    crate::boss_summon::request_native(
                                        "dharkon",
                                        ENTRY_ID,
                                        BOSS_ID[ENTRY_ID],
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER,
                                        "human_input",
                                    );
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER,
                                        true,
                                    );
                                }
                                if ControlModule::check_button_on(
                                    module_accessor,
                                    *CONTROL_PAD_BUTTON_APPEAL_S_R,
                                ) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_DARZ_STATUS_KIND_TORRENT,
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

pub unsafe fn frame(fighter: &mut L2CFighterCommon) {
    if !boss_helpers::is_world_of_light_boss_preview_stage(smash::app::stage::get_stage_id())
        && crate::should_quarantine_boss_frame(fighter.module_accessor)
    {
        return;
    }
    once_per_fighter_frame(fighter);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wol_preview_uses_core_dharkon_without_battle_tracking() {
        // Reverted: the full boss object crashed WOL on hardware.
        unsafe {
            assert_eq!(wol_presentation_item_kind(), *ITEM_KIND_DARZCENTIPEDE);
            assert_ne!(wol_presentation_item_kind(), *ITEM_KIND_DARZ);
        }
        assert_eq!(PRESENTATION_IDLE_MOTION, "wait");
        assert_eq!(WOL_PRESENTATION_SCALE, 0.05);
        assert_ne!(
            core::ptr::addr_of!(BOSS_ID) as usize,
            core::ptr::addr_of!(PREVIEW_ITEM_ID) as usize
        );
    }
}

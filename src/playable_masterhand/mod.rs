use smash::lib::lua_const::*;
use smash::app::BattleObjectModuleAccessor;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Vector3f;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::sv_information;
use smash::app::FighterUtil;
use smash::phx::Hash40;
use crate::config::CONFIG;
use crate::boss_helpers;
use crate::boss_runtime::{self, BossCommonRuntime, CommonRuntimeSyncGuard};
use crate::selection;

static mut CONTROLLABLE : bool = true;
static mut STOP : bool = false;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
static mut DEAD : bool = false;
static mut RESULT_SPAWNED : bool = false;
static mut EXISTS_PUBLIC : bool = false;
static mut FRESH_CONTROL : bool = false;
static mut JUMP_START : bool = false;
static mut DEBUG_LAST_STAGE: [i32; 8] = [-1; 8];
static mut DEBUG_LAST_FIGHTER_STATUS: [i32; 8] = [i32::MIN; 8];
static mut DEBUG_LAST_BOSS_STATUS: [i32; 8] = [i32::MIN; 8];
static mut DEBUG_LAST_FLAGS: [u16; 8] = [u16::MAX; 8];
static mut DEBUG_LAST_TRACKED_ID: [u32; 8] = [u32::MAX; 8];
static mut DEBUG_LAST_HAVE_ITEM_ID: [u32; 8] = [u32::MAX; 8];
static mut DEBUG_LAST_SCALE_BITS: [u32; 8] = [u32::MAX; 8];
    static mut CONTROLLER_X: f32 = 0.0;
    static mut CONTROLLER_Y: f32 = 0.0;
static mut CONTROL_SPEED_MUL: f32 = 2.0;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;
const HIDDEN_HOST_SCALE: f32 = 0.0001;
const PREVIEW_MASTERHAND_SCALE: f32 = 0.08;
const DEFAULT_CONTROL_SPEED_MUL: f32 = 2.0;
const DEFAULT_CONTROL_SPEED_MUL_2: f32 = 0.05;

extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
}

pub unsafe fn check_status() -> bool {
    EXISTS_PUBLIC || boss_runtime::any_exists_public(&raw const boss_runtime::PLAYABLE_MASTERHAND_RUNTIME)
}

#[inline(always)]
unsafe fn load_playable_masterhand_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    CONTROLLABLE = (*slot).controllable;
    STOP = (*slot).stop;
    DEAD = (*slot).dead;
    RESULT_SPAWNED = (*slot).result_spawned;
    EXISTS_PUBLIC = (*slot).exists_public;
    FRESH_CONTROL = (*slot).fresh_control;
    JUMP_START = (*slot).jump_start;
    CONTROLLER_X = (*slot).controller_x;
    CONTROLLER_Y = (*slot).controller_y;
}

#[inline(always)]
unsafe fn store_playable_masterhand_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    (*slot).controllable = CONTROLLABLE;
    (*slot).stop = STOP;
    (*slot).dead = DEAD;
    (*slot).result_spawned = RESULT_SPAWNED;
    (*slot).exists_public = EXISTS_PUBLIC;
    (*slot).fresh_control = FRESH_CONTROL;
    (*slot).jump_start = JUMP_START;
    (*slot).controller_x = CONTROLLER_X;
    (*slot).controller_y = CONTROLLER_Y;
}

#[inline(always)]
unsafe fn reset_playable_masterhand_controls() {
    FRESH_CONTROL = false;
    JUMP_START = false;
    CONTROLLER_X = 0.0;
    CONTROLLER_Y = 0.0;
    CONTROL_SPEED_MUL = DEFAULT_CONTROL_SPEED_MUL;
    CONTROL_SPEED_MUL_2 = DEFAULT_CONTROL_SPEED_MUL_2;
}

#[inline(always)]
unsafe fn reset_playable_masterhand_state(entry_id: usize) {
    CONTROLLABLE = true;
    STOP = false;
    ENTRY_ID = entry_id;
    DEAD = false;
    RESULT_SPAWNED = false;
    EXISTS_PUBLIC = true;
    reset_playable_masterhand_controls();
}

pub unsafe fn reset_match_state(entry_id: usize) {
    let entry = boss_runtime::sanitize_entry_id(entry_id);
    if crate::debug::enabled()
        && (BOSS_ID[entry] != 0 || DEAD || RESULT_SPAWNED || STOP || EXISTS_PUBLIC)
    {
        crate::boss_log!(
            "[PB][WOL_MH][Reset] entry={} tracked_id=0x{:x} controllable={} stop={} dead={} result_spawned={} exists_public={} fresh_control={} jump_start={} controller=({:.2},{:.2})",
            entry,
            BOSS_ID[entry],
            core::ptr::addr_of!(CONTROLLABLE).read(),
            core::ptr::addr_of!(STOP).read(),
            core::ptr::addr_of!(DEAD).read(),
            core::ptr::addr_of!(RESULT_SPAWNED).read(),
            core::ptr::addr_of!(EXISTS_PUBLIC).read(),
            core::ptr::addr_of!(FRESH_CONTROL).read(),
            core::ptr::addr_of!(JUMP_START).read(),
            core::ptr::addr_of!(CONTROLLER_X).read(),
            core::ptr::addr_of!(CONTROLLER_Y).read()
        );
    }
    CONTROLLABLE = true;
    STOP = false;
    ENTRY_ID = entry;
    BOSS_ID[entry] = 0;
    DEAD = false;
    RESULT_SPAWNED = false;
    EXISTS_PUBLIC = false;
    reset_playable_masterhand_controls();
}

#[inline(always)]
unsafe fn ensure_preview_masterhand(module_accessor: *mut BattleObjectModuleAccessor) {
    if ModelModule::scale(module_accessor) != HIDDEN_HOST_SCALE || !ItemModule::is_have_item(module_accessor, 0) {
        ItemModule::remove_all(module_accessor);
        let boss_boma = boss_helpers::acquire_boss_item(
            module_accessor,
            &raw mut BOSS_ID,
            *ITEM_KIND_MASTERHAND,
        );
        ModelModule::set_scale(module_accessor, HIDDEN_HOST_SCALE);
        ModelModule::set_scale(boss_boma, PREVIEW_MASTERHAND_SCALE);
        MotionModule::change_motion(boss_boma, Hash40::new("wait"), 0.0, 1.0, false, 0.0, false, false);
    }
    if ModelModule::scale(module_accessor) == HIDDEN_HOST_SCALE {
        MotionModule::change_motion(module_accessor, Hash40::new("none"), 0.0, 1.0, false, 0.0, false, false);
        ModelModule::set_joint_rotate(
            module_accessor,
            Hash40::new("root"),
            &mut Vector3f { x: -270.0, y: 180.0, z: -90.0 },
            smash::app::MotionNodeRotateCompose { _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8 },
            ModelModule::rotation_order(module_accessor),
        );
    }
}

#[inline(always)]
unsafe fn log_playable_masterhand_transition_snapshot(
    module_accessor: *mut BattleObjectModuleAccessor,
    fighter_manager: *mut smash::app::FighterManager,
    selected_via_slot: bool,
) {
    if module_accessor.is_null() {
        return;
    }

    let entry_id = ENTRY_ID.min(7);
    let ready_go = sv_information::is_ready_go();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let fighter_status = StatusModule::status_kind(module_accessor);

    if !selected_via_slot
        && !result_mode
        && ready_go
        && fighter_status != *FIGHTER_STATUS_KIND_ENTRY
    {
        return;
    }

    let hidden_host = boss_helpers::is_hidden_host(module_accessor);
    let cpu_entry = boss_helpers::is_operation_cpu_entry(fighter_manager, entry_id);
    let stage_id = smash::app::stage::get_stage_id();
    let tracked_id = BOSS_ID[entry_id];
    let tracked_active = tracked_id != 0 && sv_battle_object::is_active(tracked_id);
    let boss_status = if tracked_active {
        let boss_boma = sv_battle_object::module_accessor(tracked_id);
        if boss_boma.is_null() {
            i32::MIN + 1
        } else {
            StatusModule::status_kind(boss_boma)
        }
    } else {
        -1
    };
    let have_item_id = if ItemModule::is_have_item(module_accessor, 0) {
        ItemModule::get_have_item_id(module_accessor, 0) as u32
    } else {
        0
    };
    let scale_bits = ModelModule::scale(module_accessor).to_bits();
    let flags =
        (selected_via_slot as u16)
        | ((cpu_entry as u16) << 1)
        | ((ready_go as u16) << 2)
        | ((result_mode as u16) << 3)
        | ((hidden_host as u16) << 4)
        | ((EXISTS_PUBLIC as u16) << 5)
        | ((CONTROLLABLE as u16) << 6)
        | ((DEAD as u16) << 7)
        | ((STOP as u16) << 8)
        | ((RESULT_SPAWNED as u16) << 9)
        | ((tracked_active as u16) << 10);

    if DEBUG_LAST_STAGE[entry_id] == stage_id
        && DEBUG_LAST_FIGHTER_STATUS[entry_id] == fighter_status
        && DEBUG_LAST_BOSS_STATUS[entry_id] == boss_status
        && DEBUG_LAST_FLAGS[entry_id] == flags
        && DEBUG_LAST_TRACKED_ID[entry_id] == tracked_id
        && DEBUG_LAST_HAVE_ITEM_ID[entry_id] == have_item_id
        && DEBUG_LAST_SCALE_BITS[entry_id] == scale_bits
    {
        return;
    }

    DEBUG_LAST_STAGE[entry_id] = stage_id;
    DEBUG_LAST_FIGHTER_STATUS[entry_id] = fighter_status;
    DEBUG_LAST_BOSS_STATUS[entry_id] = boss_status;
    DEBUG_LAST_FLAGS[entry_id] = flags;
    DEBUG_LAST_TRACKED_ID[entry_id] = tracked_id;
    DEBUG_LAST_HAVE_ITEM_ID[entry_id] = have_item_id;
    DEBUG_LAST_SCALE_BITS[entry_id] = scale_bits;

    crate::boss_log!(
        "[PB][WOL_MH][Transition] entry={} stage=0x{:x} selected={} cpu={} ready_go={} result_mode={} hidden_host={} fighter_status={} scale={:.4} have_item_id=0x{:x} tracked_id=0x{:x} tracked_active={} boss_status={} exists_public={} controllable={} dead={} stop={} result_spawned={}",
        entry_id,
        stage_id,
        selected_via_slot,
        cpu_entry,
        ready_go,
        result_mode,
        hidden_host,
        fighter_status,
        f32::from_bits(scale_bits),
        have_item_id,
        tracked_id,
        tracked_active,
        boss_status,
        core::ptr::addr_of!(EXISTS_PUBLIC).read(),
        core::ptr::addr_of!(CONTROLLABLE).read(),
        core::ptr::addr_of!(DEAD).read(),
        core::ptr::addr_of!(STOP).read(),
        core::ptr::addr_of!(RESULT_SPAWNED).read()
    );
}

#[inline(always)]
unsafe fn acquire_cpu_world_masterhand(
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_intensity: f32,
) -> *mut BattleObjectModuleAccessor {
    let boss_boma = boss_helpers::acquire_boss_item(
        module_accessor,
        &raw mut BOSS_ID,
        *ITEM_KIND_MASTERHAND,
    );
    WorkModule::set_float(boss_boma, boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
    WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
    WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
    WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    ModelModule::set_scale(module_accessor, HIDDEN_HOST_SCALE);
    if !CONFIG.options.wol_master_hand_normal.unwrap_or(false) {
        ModelModule::set_scale(boss_boma, 1.15);
    }
    crate::boss_log!(
        "[PB][WOL_MH][Acquire] mode=cpu entry={} requested_kind={} tracked_id=0x{:x} boss_kind={} boss_status={} host_scale={:.4}",
        core::ptr::addr_of!(ENTRY_ID).read(),
        *ITEM_KIND_MASTERHAND,
        BOSS_ID[ENTRY_ID.min(7)],
        smash::app::utility::get_kind(&mut *boss_boma),
        StatusModule::status_kind(boss_boma),
        ModelModule::scale(module_accessor)
    );
    boss_boma
}

#[inline(always)]
unsafe fn acquire_player_world_masterhand(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> *mut BattleObjectModuleAccessor {
    let boss_boma = boss_helpers::acquire_boss_item(
        module_accessor,
        &raw mut BOSS_ID,
        *ITEM_KIND_PLAYABLE_MASTERHAND,
    );
    WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    ModelModule::set_scale(module_accessor, HIDDEN_HOST_SCALE);
    crate::boss_log!(
        "[PB][WOL_MH][Acquire] mode=player entry={} requested_kind={} tracked_id=0x{:x} boss_kind={} boss_status={} host_scale={:.4}",
        core::ptr::addr_of!(ENTRY_ID).read(),
        *ITEM_KIND_PLAYABLE_MASTERHAND,
        BOSS_ID[ENTRY_ID.min(7)],
        smash::app::utility::get_kind(&mut *boss_boma),
        StatusModule::status_kind(boss_boma),
        ModelModule::scale(module_accessor)
    );
    boss_boma
}

#[inline(always)]
unsafe fn resolve_world_masterhand_boss(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> *mut BattleObjectModuleAccessor {
    let entry = boss_helpers::entry_id(module_accessor);
    let tracked_id = BOSS_ID[entry];
    if tracked_id != 0 && sv_battle_object::is_active(tracked_id) {
        let boss_boma = sv_battle_object::module_accessor(tracked_id);
        if !boss_boma.is_null() {
            return boss_boma;
        }
    }

    if ItemModule::is_have_item(module_accessor, 0) {
        let current_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
        if current_id != 0 && sv_battle_object::is_active(current_id) {
            BOSS_ID[entry] = current_id;
            let boss_boma = sv_battle_object::module_accessor(current_id);
            if !boss_boma.is_null() {
                return boss_boma;
            }
        }
    }

    std::ptr::null_mut()
}

#[inline(always)]
unsafe fn restore_world_masterhand_after_item_wipe(
    module_accessor: *mut BattleObjectModuleAccessor,
    fighter_manager: *mut smash::app::FighterManager,
) {
    if module_accessor.is_null()
    || !sv_information::is_ready_go()
    || DEAD {
        return;
    }

    let entry = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
    ENTRY_ID = entry;
    if !resolve_world_masterhand_boss(module_accessor).is_null() {
        return;
    }
    if let Some((_, held_id, _)) = boss_helpers::held_item_by_kind(
        module_accessor,
        &[*ITEM_KIND_MASTERHAND, *ITEM_KIND_PLAYABLE_MASTERHAND],
    ) {
        BOSS_ID[entry] = held_id;
        return;
    }

    ItemModule::remove_all(module_accessor);
    EXISTS_PUBLIC = true;
    RESULT_SPAWNED = false;
    let cpu_entry = boss_helpers::is_operation_cpu_entry(fighter_manager, entry);
    let boss_boma = if cpu_entry {
        acquire_cpu_world_masterhand(module_accessor, CONFIG.options.boss_difficulty.unwrap_or(10.0))
    } else {
        acquire_player_world_masterhand(module_accessor)
    };
    WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(module_accessor),
        y: PostureModule::pos_y(module_accessor),
        z: PostureModule::pos_z(module_accessor),
    };
    PostureModule::set_pos(boss_boma, &boss_pos);
    StatusModule::change_status_request_from_script(
        boss_boma,
        if cpu_entry {
            *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT
        } else {
            *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_WAIT
        },
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
    crate::boss_log!(
        "[PB][Recover] entry {}: restored WOL Master Hand after item wipe cpu_entry={} tracked_id=0x{:x} tracked_kind={} tracked_status={} host_scale={:.4}",
        entry,
        cpu_entry,
        BOSS_ID[entry],
        smash::app::utility::get_kind(&mut *boss_boma),
        StatusModule::status_kind(boss_boma),
        ModelModule::scale(module_accessor)
    );
}

#[inline(always)]
unsafe fn is_world_masterhand_item_kind(kind: i32) -> bool {
    kind == *ITEM_KIND_MASTERHAND || kind == *ITEM_KIND_PLAYABLE_MASTERHAND
}

#[inline(always)]
unsafe fn is_world_masterhand_boma(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    !module_accessor.is_null()
        && is_world_masterhand_item_kind(smash::app::utility::get_kind(&mut *module_accessor))
}

#[inline(always)]
unsafe fn held_world_masterhand_item_slot(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<(i32, u32)> {
    let mut slot = 0;
    while slot < 4 {
        if ItemModule::is_have_item(module_accessor, slot) {
            let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
            if item_id != 0 && sv_battle_object::is_active(item_id) {
                let item_boma = sv_battle_object::module_accessor(item_id);
                if is_world_masterhand_boma(item_boma) {
                    return Some((slot, item_id));
                }
            }
        }
        slot += 1;
    }
    None
}

#[inline(always)]
unsafe fn handle_playable_masterhand_stock_drain(
    module_accessor: *mut BattleObjectModuleAccessor,
    fighter_manager: *mut smash::app::FighterManager,
    boss_boma: *mut BattleObjectModuleAccessor,
) {
    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {
        return;
    }
    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
    ItemModule::remove_all(module_accessor);
    if STOP {
        return;
    }
    if CONFIG.options.boss_respawn.unwrap_or(false) {
        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
        STOP = true;
    } else {
        boss_helpers::request_hidden_host_stock_drain(
            module_accessor,
            fighter_manager,
            ENTRY_ID,
            &raw mut STOP,
        );
    }
}

#[inline(always)]
unsafe fn teardown_world_masterhand_post_match_transition(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    let entry = boss_helpers::entry_id(module_accessor);
    let tracked_id = BOSS_ID[entry];
    let tracked_boma = if tracked_id != 0 && sv_battle_object::is_active(tracked_id) {
        sv_battle_object::module_accessor(tracked_id)
    } else {
        std::ptr::null_mut()
    };
    let tracked_is_world_masterhand = is_world_masterhand_boma(tracked_boma);
    let held_world_masterhand = held_world_masterhand_item_slot(module_accessor);
    let held_world_masterhand_id = held_world_masterhand.map(|(_, item_id)| item_id);
    let terminate_tracked_world_masterhand = tracked_is_world_masterhand
        && held_world_masterhand_id.map_or(true, |held_item_id| held_item_id != tracked_id);
    if !tracked_is_world_masterhand && held_world_masterhand.is_none() && !EXISTS_PUBLIC {
        return false;
    }

    if tracked_is_world_masterhand {
        HitModule::set_whole(tracked_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
        SlowModule::clear_whole(tracked_boma);
        StatusModule::change_status_request_from_script(
            tracked_boma,
            if terminate_tracked_world_masterhand {
                *ITEM_STATUS_KIND_DEAD
            } else {
                *ITEM_STATUS_KIND_STANDBY
            },
            true,
        );
    }

    if let Some((slot, held_item_id)) = held_world_masterhand {
        if held_item_id != tracked_id {
            let held_boma = sv_battle_object::module_accessor(held_item_id);
            if is_world_masterhand_boma(held_boma) {
                HitModule::set_whole(held_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                StatusModule::change_status_request_from_script(held_boma, *ITEM_STATUS_KIND_STANDBY, true);
            }
        }
        ItemModule::remove_item(module_accessor, slot);
    }

    boss_helpers::clear_hidden_host_effects(module_accessor);
    boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
    ModelModule::set_scale(module_accessor, HIDDEN_HOST_SCALE);
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

    BOSS_ID[entry] = 0;
    EXISTS_PUBLIC = false;
    CONTROLLABLE = false;
    STOP = false;
    DEAD = false;
    RESULT_SPAWNED = false;
    reset_playable_masterhand_controls();

    crate::boss_log!(
        "[PB][WOL_MH][Cleanup] entry {}: cleared WOL runtime on non-ready_go transition tracked_wol={} held_wol={}",
        entry,
        tracked_is_world_masterhand,
        held_world_masterhand.is_some()
    );
    crate::boss_log!(
        "[PB][WOL_MH][CleanupState] entry {}: terminate_tracked_wol={} tracked_id=0x{:x} held_id=0x{:x}",
        entry,
        terminate_tracked_world_masterhand,
        tracked_id,
        held_world_masterhand_id.unwrap_or(0)
    );

    true
}

#[inline(always)]
unsafe fn update_smoothed_control_axis(current: *mut f32, stick: f32) {
    if current.is_null() {
        return;
    }
    if *current < stick * CONTROL_SPEED_MUL && *current >= 0.0 && stick > 0.0 {
        *current += (stick * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
    }
    if *current > stick * CONTROL_SPEED_MUL && *current <= 0.0 && stick < 0.0 {
        *current += (stick * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
    }
    if *current > 0.0 && *current != 0.0 && stick == 0.0 {
        *current -= CONTROL_SPEED_MUL_2;
    }
    if *current < 0.0 && *current != 0.0 && stick == 0.0 {
        *current += CONTROL_SPEED_MUL_2;
    }
    if stick == 0.0 {
        if *current > 0.0 && *current < 0.06 {
            *current = 0.0;
        }
        if *current < 0.0 && *current > 0.06 {
            *current = 0.0;
        }
    }
    if *current > 0.0 && stick < 0.0 {
        *current += (stick * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
    }
    if *current < 0.0 && stick > 0.0 {
        *current += (stick * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
    }
}

#[inline(always)]
unsafe fn apply_smoothed_playable_masterhand_input(
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
    move_scale: f32,
) {
    update_smoothed_control_axis(&raw mut CONTROLLER_X, ControlModule::get_stick_x(module_accessor));
    update_smoothed_control_axis(&raw mut CONTROLLER_Y, ControlModule::get_stick_y(module_accessor));
    let pos = Vector3f {
        x: CONTROLLER_X * move_scale,
        y: CONTROLLER_Y * move_scale,
        z: 0.0,
    };
    PostureModule::add_pos(boss_boma, &pos);
}

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            ENTRY_ID = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
            let _runtime_guard = CommonRuntimeSyncGuard::new(
                boss_runtime::slot_ptr(&raw mut boss_runtime::PLAYABLE_MASTERHAND_RUNTIME, ENTRY_ID),
                load_playable_masterhand_runtime,
                store_playable_masterhand_runtime,
            );
            let fighter_manager = boss_helpers::fighter_manager();
            
            let selected_via_slot = selection::is_selected_css_boss(module_accessor, *ITEM_KIND_PLAYABLE_MASTERHAND);
            log_playable_masterhand_transition_snapshot(module_accessor, fighter_manager, selected_via_slot);
            if selection::is_boss_selection_suppressed(module_accessor)
                && !selected_via_slot
                && !sv_information::is_ready_go()
                && (BOSS_ID[ENTRY_ID] != 0 || EXISTS_PUBLIC)
            {
                teardown_world_masterhand_post_match_transition(module_accessor);
                return;
            }
            if selected_via_slot {
                if boss_helpers::is_boss_preview_stage(smash::app::stage::get_stage_id()) {
                    ensure_preview_masterhand(module_accessor);
                }
                else if !boss_helpers::is_boss_passthrough_stage(smash::app::stage::get_stage_id()) {
                    restore_world_masterhand_after_item_wipe(module_accessor, fighter_manager);
                    if boss_helpers::needs_hidden_host_entry_init(module_accessor, &raw const BOSS_ID, ENTRY_ID) {
                        let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        reset_playable_masterhand_state(entry_id);
                        let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                            let boss_boma = acquire_cpu_world_masterhand(module_accessor, get_boss_intensity);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME, true);
                        }
                        else {
                            let boss_boma = acquire_player_world_masterhand(module_accessor);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
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

                    if sv_information::is_ready_go() && !ItemModule::is_have_item(module_accessor, 0) && ModelModule::scale(module_accessor) != HIDDEN_HOST_SCALE && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID)
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) {
                        if smash::app::smashball::is_training_mode() || CONFIG.options.boss_respawn.unwrap_or(false) {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);
                            let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            reset_playable_masterhand_state(entry_id);
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                            let boss_boma = acquire_cpu_world_masterhand(module_accessor, get_boss_intensity);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE, true);

                            let x = PostureModule::pos_x(module_accessor);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(module_accessor);
                            let module_pos = Vector3f{x: x, y: y, z: z};
                            PostureModule::set_pos(boss_boma, &module_pos);
                        }
                    }

                    if sv_information::is_ready_go()
                    && !ItemModule::is_have_item(module_accessor, 0)
                    && ModelModule::scale(module_accessor) != HIDDEN_HOST_SCALE
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false
                    && (smash::app::smashball::is_training_mode() || CONFIG.options.boss_respawn.unwrap_or(false)) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);
                        let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        reset_playable_masterhand_state(entry_id);
                        let boss_boma = acquire_player_world_masterhand(module_accessor);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);


                        let x = PostureModule::pos_x(module_accessor);
                        let y = PostureModule::pos_y(module_accessor);
                        let z = PostureModule::pos_z(module_accessor);
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(boss_boma, &module_pos);
                        CONTROLLABLE = true;
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if crate::mastercrazy::check_status() || crate::mastercrazy::check_status_2() {
                            EXISTS_PUBLIC = false;
                            CONTROLLABLE = false;
                            RESULT_SPAWNED = true;
                            boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
                            return;
                        }
                        if RESULT_SPAWNED == false {
                            EXISTS_PUBLIC = false;
                            CONTROLLABLE = false;
                            RESULT_SPAWNED = true;
                            let entry_id_log = ENTRY_ID;
                            crate::boss_log!(
                                "[PB][Result][WOL_MH] entry {}: keeping active playable hand, skipping fallback result spawn",
                                entry_id_log
                            );
                        }
                        boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
                        return;
                    }

                    // Flags and new damage stuff

                    let boss_boma = resolve_world_masterhand_boss(module_accessor);
                    if boss_boma.is_null() {
                        return;
                    }

                    if sv_information::is_ready_go() == true {
                        let boss_hp = WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        if boss_hp != 999.0 {
                            let sub_hp = 999.0 - boss_hp;
                            let entry_id = ENTRY_ID;
                            crate::boss_log!(
                                "[PB][DamageSet][WOL_MH] entry {}: boss_hp={} -> add_damage={}",
                                entry_id,
                                boss_hp,
                                sub_hp
                            );
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if CONTROLLABLE && ENTRY_ID != 0 {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        // println!("RATE: {}", MotionModule::rate(boss_boma));
                        JostleModule::set_status(module_accessor, false);
                    }

                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma, i, false) {
                            AttackModule::set_target_category(boss_boma, i, *COLLISION_CATEGORY_MASK_ALL as u32);
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
                            MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if DEAD == false {
                        if ModelModule::scale(module_accessor) == 0.0001 && BOSS_ID[boss_helpers::entry_id(module_accessor)] != 0 {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                            if FighterUtil::is_hp_mode(module_accessor) == true {
                                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD
                                || StatusModule::status_kind(module_accessor) == 79 {
                                    if DEAD == false {
                                        CONTROLLABLE = false;
                                        EXISTS_PUBLIC = false;
                                        DEAD = true;
                                        let entry_id = ENTRY_ID;
                                        crate::boss_log!(
                                            "[PB][StatusDead][WOL_MH] entry {}: fighter dead in HP mode, killing boss",
                                            entry_id
                                        );
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                    }
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        JostleModule::set_status(module_accessor, false);
                        let hp = CONFIG.options.wol_master_hand_hp.unwrap_or(400.0);
                        if DamageModule::damage(module_accessor, 0) >= hp && FighterUtil::is_hp_mode(module_accessor) == false {
                            if DEAD == false {
                                DEAD = true;
                                let entry_id = ENTRY_ID;
                                crate::boss_log!(
                                    "[PB][StatusDead][WOL_MH] entry {}: reached HP threshold {}, killing boss",
                                    entry_id,
                                    hp
                                );
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        if DEAD == false {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT,
                                    true,
                                );
                            } else {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_WAIT,
                                    true,
                                );
                            }
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 {
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

                        // SETS POWER

                        if !CONFIG.options.wol_master_hand_normal.unwrap_or(false) {
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                if StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING
                                && StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU
                                && StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                                    AttackModule::set_power_mul(boss_boma, 0.5);
                                    AttackModule::set_power_mul_2nd(boss_boma, 1.0);
                                    AttackModule::set_power_mul_3rd(boss_boma, 1.0);
                                    AttackModule::set_power_mul_4th(boss_boma, 1.0);
                                    AttackModule::set_power_mul_5th(boss_boma, 1.0);
                                }
                                else {
                                    AttackModule::set_power_mul(boss_boma, 0.02);
                                    AttackModule::set_power_mul_2nd(boss_boma, 0.2);
                                    AttackModule::set_power_mul_3rd(boss_boma, 0.2);
                                    AttackModule::set_power_mul_4th(boss_boma, 0.2);
                                    AttackModule::set_power_mul_5th(boss_boma, 0.2);
                                }
                            }
                        }

                        if DEAD == true {
                            if sv_information::is_ready_go() == true {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_STANDBY {
                                    if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_DEAD {
                                        if StatusModule::status_kind(boss_boma) != *FIGHTER_STATUS_KIND_DEAD {
                                            if StatusModule::status_kind(boss_boma) != *FIGHTER_STATUS_KIND_STANDBY {
                                                SlowModule::clear_whole(boss_boma);
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if sv_information::is_ready_go() == true {
                            if DEAD == true {
                                handle_playable_masterhand_stock_drain(
                                    module_accessor,
                                    fighter_manager,
                                    boss_boma,
                                );
                            }
                        }

                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END,true);
                            }
                        }

                        if ENTRY_ID == 0 && sv_information::is_ready_go() == true && !JUMP_START {
                            JUMP_START = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_WAIT,true);
                        }
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false && sv_information::is_ready_go() == true && ENTRY_ID != 0 {
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                                apply_smoothed_playable_masterhand_input(module_accessor, boss_boma, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN {
                                apply_smoothed_playable_masterhand_input(module_accessor, boss_boma, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_IRON_BALL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_CHAKRAM_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                                apply_smoothed_playable_masterhand_input(module_accessor, boss_boma, 1.0);
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == false {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TURN {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }

                            if FRESH_CONTROL && !DEAD {
                                FRESH_CONTROL = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                MotionModule::change_motion(boss_boma, smash::phx::Hash40::new("wait"), 0.0, 1.0, false, 0.0, false, false);
                            }

                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT
                            || StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_WAIT
                            || MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                                if !CONTROLLABLE {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_MOVE {
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f{x: x, y: y, z: z};
                                let boss_pos_2 = Vector3f{x: x, y: y + 20.0, z: z};
                                PostureModule::set_pos(module_accessor, &boss_pos_2);
                                PostureModule::set_pos(boss_boma, &boss_pos);
                            }

                            if StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN && CONTROLLABLE && !DEAD {
                                apply_smoothed_playable_masterhand_input(module_accessor, boss_boma, 1.0);

                                if PostureModule::lr(boss_boma) == 1.0 { // right
                                    if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                if PostureModule::lr(boss_boma) == -1.0 { // left
                                    if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                // Boss Moves
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    MotionModule::set_rate(boss_boma, 1.2);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    MotionModule::set_rate(boss_boma, 2.5);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.5);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    MotionModule::set_rate(boss_boma, 1.3);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_IRON_BALL_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    MotionModule::set_rate(boss_boma, 1.3);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.3);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_CHAKRAM_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
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
}

pub unsafe fn frame(fighter: &mut L2CFighterCommon) {
    once_per_fighter_frame(fighter);
}

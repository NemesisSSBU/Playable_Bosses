use smash::app::BattleObjectModuleAccessor;
use smash::app::FighterEntryID;
use smash::app::FighterManager;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::phx::Hash40;
use smash::phx::Vector3f;
use skyline::nn::ro::LookupSymbol;

static mut FIGHTER_MANAGER_ADDR: usize = 0;

pub const HIDDEN_HOST_SCALE: f32 = 0.0001;
pub const HIDDEN_HOST_ENTRY_PREP_SCALE: f32 = 0.001;
pub const HIDDEN_HOST_ENTRY_STAGE2_SCALE: f32 = 0.002;
pub const HIDDEN_HOST_BASELINE_SCALE: f32 = 0.008;
const HIDDEN_HOST_ENTRY_PREP_EPSILON: f32 = 0.00005;
const HIDDEN_HOST_BASELINE_EPSILON: f32 = 0.0005;

pub const STAGE_ID_BOSS_PREVIEW: i32 = 0x139;
pub const STAGE_ID_CLASSIC_BONUS_GAME: i32 = 0x13A;
pub const STAGE_ID_CLASSIC_STAFFROLL: i32 = 0x13C;

#[inline(always)]
pub unsafe fn entry_id(module_accessor: *mut BattleObjectModuleAccessor) -> usize {
    if module_accessor.is_null() {
        return 0;
    }
    WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize
}

#[inline(always)]
pub unsafe fn fighter_manager() -> *mut FighterManager {
    if FIGHTER_MANAGER_ADDR == 0 {
        LookupSymbol(
            &raw mut FIGHTER_MANAGER_ADDR,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\0"
                .as_bytes()
                .as_ptr(),
        );
    }
    if FIGHTER_MANAGER_ADDR == 0 {
        return std::ptr::null_mut();
    }
    *(FIGHTER_MANAGER_ADDR as *mut *mut FighterManager)
}

#[inline(always)]
pub unsafe fn fighter_information_entry(
    fighter_manager: *mut FighterManager,
    entry_id: usize,
) -> *mut smash::app::FighterInformation {
    if fighter_manager.is_null() {
        return std::ptr::null_mut();
    }
    smash::app::lua_bind::FighterManager::get_fighter_information(
        fighter_manager,
        FighterEntryID(entry_id as i32),
    )
}

#[inline(always)]
pub unsafe fn is_operation_cpu_entry(fighter_manager: *mut FighterManager, entry_id: usize) -> bool {
    let info = fighter_information_entry(fighter_manager, entry_id);
    !info.is_null() && FighterInformation::is_operation_cpu(info)
}

#[inline(always)]
pub unsafe fn stock_count_entry(fighter_manager: *mut FighterManager, entry_id: usize) -> u64 {
    let info = fighter_information_entry(fighter_manager, entry_id);
    if info.is_null() {
        return 0;
    }
    FighterInformation::stock_count(info)
}

#[inline(always)]
pub unsafe fn acquire_boss_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *mut [u32; 8],
    item_kind: i32,
) -> *mut BattleObjectModuleAccessor {
    if module_accessor.is_null() || slot_ids.is_null() {
        return std::ptr::null_mut();
    }
    ItemModule::have_item(module_accessor, ItemKind(item_kind), 0, 0, false, false);
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);
    let entry = entry_id(module_accessor);
    let boss_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
    (*slot_ids)[entry] = boss_id;
    let boss_boma = sv_battle_object::module_accessor(boss_id);
    if crate::debug::enabled() {
        let fighter_status = StatusModule::status_kind(module_accessor);
        let boss_kind = if boss_boma.is_null() {
            -1
        } else {
            smash::app::utility::get_kind(&mut *boss_boma)
        };
        let mut slot_ids_debug = [0u32; 4];
        let mut slot_kinds_debug = [-1i32; 4];
        for slot in 0..4 {
            if ItemModule::is_have_item(module_accessor, slot) {
                let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
                slot_ids_debug[slot as usize] = item_id;
                if item_id != 0 && sv_battle_object::is_active(item_id) {
                    let item_boma = sv_battle_object::module_accessor(item_id);
                    if !item_boma.is_null() {
                        slot_kinds_debug[slot as usize] = smash::app::utility::get_kind(&mut *item_boma);
                    }
                }
            }
        }
        crate::boss_log!(
            "[PB][BossItem] acquire entry={} requested_kind={} acquired_id=0x{:x} acquired_kind={} stage=0x{:x} fighter_status={} scale={:.4} slots={:?} slot_kinds={:?}",
            entry,
            item_kind,
            boss_id,
            boss_kind,
            smash::app::stage::get_stage_id(),
            fighter_status,
            ModelModule::scale(module_accessor),
            slot_ids_debug,
            slot_kinds_debug
        );
    }
    boss_boma
}

pub unsafe fn acquire_boss_item_excluding(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *mut [u32; 8],
    item_kind: i32,
    excluded_item_id: u32,
) -> *mut BattleObjectModuleAccessor {
    if module_accessor.is_null() || slot_ids.is_null() {
        return std::ptr::null_mut();
    }
    ItemModule::have_item(module_accessor, ItemKind(item_kind), 0, 0, false, false);
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);
    let entry = entry_id(module_accessor);
    let mut boss_id = 0;
    for slot in 0..4 {
        if ItemModule::is_have_item(module_accessor, slot) {
            let candidate = ItemModule::get_have_item_id(module_accessor, slot) as u32;
            if candidate != 0 && candidate != excluded_item_id {
                boss_id = candidate;
                break;
            }
        }
    }
    if boss_id == 0 {
        boss_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
    }
    (*slot_ids)[entry] = boss_id;
    let boss_boma = sv_battle_object::module_accessor(boss_id);
    if crate::debug::enabled() {
        let boss_kind = if boss_boma.is_null() {
            -1
        } else {
            smash::app::utility::get_kind(&mut *boss_boma)
        };
        crate::boss_log!(
            "[PB][BossItem] acquire_excluding entry={} requested_kind={} excluded_id=0x{:x} acquired_id=0x{:x} acquired_kind={} stage=0x{:x} fighter_status={} scale={:.4}",
            entry,
            item_kind,
            excluded_item_id,
            boss_id,
            boss_kind,
            smash::app::stage::get_stage_id(),
            StatusModule::status_kind(module_accessor),
            ModelModule::scale(module_accessor)
        );
    }
    boss_boma
}

#[inline(always)]
pub unsafe fn held_item_by_kind(
    module_accessor: *mut BattleObjectModuleAccessor,
    expected_kinds: &[i32],
) -> Option<(i32, u32, *mut BattleObjectModuleAccessor)> {
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
        if expected_kinds.iter().any(|&expected_kind| expected_kind == item_kind) {
            return Some((slot, item_id, item_boma));
        }
    }
    None
}

#[inline(always)]
pub unsafe fn clear_boss_item_slot(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *mut [u32; 8],
    set_standby: bool,
) {
    if module_accessor.is_null() || slot_ids.is_null() {
        return;
    }
    let entry = entry_id(module_accessor);
    let boss_id = (*slot_ids)[entry];
    if boss_id != 0 && sv_battle_object::is_active(boss_id) {
        let boss_boma = sv_battle_object::module_accessor(boss_id);
        if !boss_boma.is_null() {
            HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
            if set_standby {
                StatusModule::change_status_request_from_script(
                    boss_boma,
                    *ITEM_STATUS_KIND_STANDBY,
                    true,
                );
            }
        }
    }
    ItemModule::remove_all(module_accessor);
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][BossItem] clear entry={} tracked_id=0x{:x} set_standby={} stage=0x{:x} fighter_status={} scale={:.4}",
            entry,
            boss_id,
            set_standby,
            smash::app::stage::get_stage_id(),
            StatusModule::status_kind(module_accessor),
            ModelModule::scale(module_accessor)
        );
    }
    (*slot_ids)[entry] = 0;
}

#[inline(always)]
pub unsafe fn is_hidden_host(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    !module_accessor.is_null() && ModelModule::scale(module_accessor) <= HIDDEN_HOST_ENTRY_STAGE2_SCALE
}

#[inline(always)]
pub unsafe fn is_hidden_host_entry_prep(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    scale >= HIDDEN_HOST_ENTRY_PREP_SCALE - HIDDEN_HOST_ENTRY_PREP_EPSILON
        && scale <= HIDDEN_HOST_ENTRY_PREP_SCALE + HIDDEN_HOST_ENTRY_PREP_EPSILON
}

#[inline(always)]
pub unsafe fn is_hidden_host_entry_stage_two(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    scale >= HIDDEN_HOST_ENTRY_STAGE2_SCALE - HIDDEN_HOST_ENTRY_PREP_EPSILON
        && scale <= HIDDEN_HOST_ENTRY_STAGE2_SCALE + HIDDEN_HOST_ENTRY_PREP_EPSILON
}

#[inline(always)]
pub unsafe fn is_hidden_host_baseline(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    scale >= HIDDEN_HOST_BASELINE_SCALE - HIDDEN_HOST_BASELINE_EPSILON
        && scale <= HIDDEN_HOST_BASELINE_SCALE + HIDDEN_HOST_BASELINE_EPSILON
}

#[inline(always)]
pub unsafe fn is_tracked_boss_active(slot_ids: *const [u32; 8], entry: usize) -> bool {
    if slot_ids.is_null() {
        return false;
    }
    let entry = entry.min(7);
    let item_id = (*slot_ids)[entry];
    item_id != 0 && sv_battle_object::is_active(item_id)
}

#[inline(always)]
pub unsafe fn needs_hidden_host_entry_init(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *const [u32; 8],
    entry: usize,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    ModelModule::scale(module_accessor) > HIDDEN_HOST_ENTRY_PREP_SCALE
        || !is_tracked_boss_active(slot_ids, entry)
}

#[inline(always)]
pub unsafe fn clear_hidden_host_effects(module_accessor: *mut BattleObjectModuleAccessor) {
    if is_hidden_host(module_accessor) {
        EffectModule::kill_all(module_accessor, 0, false, false);
    }
}

#[inline(always)]
pub unsafe fn stop_hidden_host_mario_result_sfx(module_accessor: *mut BattleObjectModuleAccessor) {
    if !is_hidden_host(module_accessor) {
        return;
    }
    SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_05"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_013"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_09"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_common_punch_kick_swing_l"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win02"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_mario_win2"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_014"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win03"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_015"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_mario_jump01"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_mario_landing02"), 0);
}

#[inline(always)]
pub unsafe fn stop_hidden_host_knockout_sfx(module_accessor: *mut BattleObjectModuleAccessor) {
    if !is_hidden_host(module_accessor) {
        return;
    }
    SoundModule::stop_se(module_accessor, Hash40::new("death"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("dead"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_damage_reaction"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_dead_frame"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_reaction"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_slow_frame"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_slow_mag"), 0);
}

#[inline(always)]
pub unsafe fn restore_plain_mario_visuals(
    module_accessor: *mut BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() {
        return;
    }

    clear_hidden_host_effects(module_accessor);
    stop_hidden_host_mario_result_sfx(module_accessor);
    stop_hidden_host_knockout_sfx(module_accessor);
    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);
    JostleModule::set_status(module_accessor, true);
    VisibilityModule::set_whole(module_accessor, true);
    ModelModule::set_scale(module_accessor, 1.0);

    let reset_rot = Vector3f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    PostureModule::set_rot(module_accessor, &reset_rot, 0);

    let mut reset_joint_rot = Vector3f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    ModelModule::set_joint_rotate(
        module_accessor,
        Hash40::new("root"),
        &mut reset_joint_rot,
        smash::app::MotionNodeRotateCompose {
            _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
        },
        ModelModule::rotation_order(module_accessor),
    );
}

#[inline(always)]
pub unsafe fn request_hidden_host_stock_drain(
    module_accessor: *mut BattleObjectModuleAccessor,
    fighter_manager: *mut FighterManager,
    entry_id: usize,
    stop: *mut bool,
) {
    if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
        StatusModule::change_status_request_from_script(
            module_accessor,
            *FIGHTER_STATUS_KIND_DEAD,
            true,
        );
        stop_hidden_host_knockout_sfx(module_accessor);
    }
    if stock_count_entry(fighter_manager, entry_id) == 0 {
        *stop = true;
    }
}

#[inline(always)]
pub unsafe fn clamp_flying_boss_floor(
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
    clearance: f32,
) {
    if module_accessor.is_null() || boss_boma.is_null() {
        return;
    }
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(boss_boma),
        y: PostureModule::pos_y(boss_boma),
        z: PostureModule::pos_z(boss_boma),
    };
    let probe_pos = Vector3f {
        x: boss_pos.x,
        y: boss_pos.y + 120.0,
        z: boss_pos.z,
    };
    let probe_dist = GroundModule::get_distance_to_floor(module_accessor, &probe_pos, probe_pos.y, true);
    if probe_dist > 0.0 && probe_dist < 400.0 {
        let floor_y = probe_pos.y - probe_dist;
        let clamped_y = floor_y + clearance;
        if boss_pos.y < clamped_y {
            let clamped = Vector3f {
                x: boss_pos.x,
                y: clamped_y,
                z: boss_pos.z,
            };
            PostureModule::set_pos(module_accessor, &clamped);
            PostureModule::set_pos(boss_boma, &clamped);
        }
    }
}

#[inline(always)]
pub fn is_boss_preview_stage(stage_id: i32) -> bool {
    // These scenes use the preview/interstitial boss presentation path.
    stage_id == STAGE_ID_BOSS_PREVIEW
        || stage_id == STAGE_ID_CLASSIC_STAFFROLL
}

#[inline(always)]
pub fn is_boss_passthrough_stage(stage_id: i32) -> bool {
    // These scenes should stay on the base fighter because the boss takeover
    // path is not playable there.
    stage_id == STAGE_ID_CLASSIC_BONUS_GAME
}

#[inline(always)]
pub fn is_boss_nonbattle_stage(stage_id: i32) -> bool {
    is_boss_preview_stage(stage_id) || is_boss_passthrough_stage(stage_id)
}

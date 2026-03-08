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
    sv_battle_object::module_accessor(boss_id)
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
    (*slot_ids)[entry] = 0;
}

#[inline(always)]
pub unsafe fn is_hidden_host(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    !module_accessor.is_null() && ModelModule::scale(module_accessor) <= 0.0002
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

use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::FighterUtil;
use smash::app::sv_information;
use smash::app::lua_bind;
use skyline::nn::ro::LookupSymbol;
use smash::phx::Hash40;
use smash::hash40;
use smash::app::utility::get_category;

static mut CONTROLLABLE : bool = true;
static mut TRANSITIONING : bool = false;
static mut TELEPORTED : bool = false;
static mut TRANSFORMED_MODE : bool = false;
static mut KICKSTART_ANIM_BEGIN : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
pub static mut FIGHTER_NAME: [u64;7] = [0;7];

pub unsafe fn read_tag(addr: u64) -> String {
    let mut s: Vec<u8> = vec![];

    let mut addr = addr as *const u16;
    loop {
        if *addr == 0_u16 {
            break;
        }
        s.push(*(addr as *const u8));
        addr = addr.offset(1);
    }
    // No null terminator needed

    std::str::from_utf8(&s).unwrap().to_owned()
}

pub unsafe fn get_player_number(module_accessor:  &mut smash::app::BattleObjectModuleAccessor) -> usize {
    let player_number;
    if smash::app::utility::get_kind(module_accessor) == *WEAPON_KIND_PTRAINER_PTRAINER {
        player_number = WorkModule::get_int(module_accessor, *WEAPON_PTRAINER_PTRAINER_INSTANCE_WORK_ID_INT_FIGHTER_ENTRY_ID) as usize;
    }
    else if get_category(module_accessor) == *BATTLE_OBJECT_CATEGORY_FIGHTER {
        player_number = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
    }
    else {
        let mut owner_module_accessor = &mut *sv_battle_object::module_accessor((WorkModule::get_int(module_accessor, *WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER)) as u32);
        while get_category(owner_module_accessor) != *BATTLE_OBJECT_CATEGORY_FIGHTER { //Keep checking the owner of the boma we're working with until we've hit a boma that belongs to a fighter
            owner_module_accessor = &mut *sv_battle_object::module_accessor((WorkModule::get_int(owner_module_accessor, *WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER)) as u32);
        }
        player_number = WorkModule::get_int(owner_module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
    }
    return player_number;
}

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_KOOPAG {
            pub unsafe fn entry_id(module_accessor: &mut BattleObjectModuleAccessor) -> usize {
                let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                return entry_id;
            }
            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
            LookupSymbol(
                &mut FIGHTER_MANAGER,
                "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
                .as_bytes()
                .as_ptr(),
            );
            let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
            let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
            let name_base = text + 0x52c3758;
            FIGHTER_NAME[get_player_number(&mut *fighter.module_accessor)] = hash40(&read_tag(name_base + 0x260 * get_player_number(&mut *fighter.module_accessor) as u64 + 0x8e));
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("DRACULA") {
                if sv_information::is_ready_go() == false {
                    DEAD = false;
                    CONTROLLABLE = true;
                    JUMP_START = false;
                    TRANSITIONING = false;
                    TRANSFORMED_MODE = false;
                    KICKSTART_ANIM_BEGIN = false;
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        RESULT_SPAWNED = false;
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DRACULA), 0, 0, false, false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                    }
                }

                if sv_information::is_ready_go() == true {
                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    let attack = WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_ATTACK_KIND);
                    if CONTROLLABLE == true {
                        MotionModule::change_motion(boss_boma,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                        if attack != 0 {
                            //boss_private::main_energy_from_param(lua_state,ItemKind(*ITEM_KIND_MASTERHAND),Hash40::new("energy_param_wait"),0.0);
                            StatusModule::change_status_request_from_script(boss_boma,attack,false);
                            WorkModule::set_int(boss_boma, 0, *ITEM_INSTANCE_WORK_INT_ATTACK_KIND);
                        }
                    }
                }

                if DEAD == false {
                    if sv_information::is_ready_go() == true {
                        // SET POS
                        if ModelModule::scale(module_accessor) == 0.0001 {
                            if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                            }
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_STANDBY {
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f{x: x, y: y, z: z};
                                PostureModule::set_pos(module_accessor, &boss_pos);
                            }
                        }
                    }
                }

                //DAMAGE MODULES
                
                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                DamageModule::set_damage_lock(boss_boma, true);
                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);

                if StopModule::is_damage(boss_boma) {
                    if TRANSFORMED_MODE == false {
                        if DamageModule::damage(module_accessor, 0) >= 199.0 {
                            let none_pos = Vector3f{x: 0.0, y: 0.0, z: 0.0};
                            if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_DEAD {
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                TRANSITIONING = true;
                            }
                            if TRANSITIONING == true {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_DEAD | *ITEM_DRACULA_STATUS_KIND_CHANGE_START | *ITEM_STATUS_KIND_TRANS_PHASE {
                                    TRANSFORMED_MODE = true;
                                    PostureModule::set_pos(module_accessor, &none_pos);
                                    PostureModule::set_pos(boss_boma, &none_pos);
                                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                                    ModelModule::set_scale(module_accessor, 1.0);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                                    ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DRACULA2), 0, 0, false, false);
                                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                    PostureModule::set_pos(module_accessor, &none_pos);
                                    PostureModule::set_pos(boss_boma, &none_pos);
                                    ModelModule::set_scale(module_accessor, 0.0001);
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                    }
                                    smash::app::lua_bind::ItemModule::set_have_item_visibility(boss_boma, true, 0);
                                    TRANSITIONING = false;
                                }
                            }
                        }
                    }
                    if sv_information::is_ready_go() == true {
                        if CONTROLLABLE == true {
                            if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_WAIT {
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                    CONTROLLABLE = false;
                                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                    }
                                }
                                else {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                }
                            }
                        }
                    }
                    if FighterUtil::is_hp_mode(module_accessor) == true {
                        if DamageModule::damage(module_accessor, 0) < 1.0 {
                            if DEAD == false {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD,true);
                                DEAD = true;
                            }
                        }
                    }
                    if FighterUtil::is_hp_mode(module_accessor) == false {
                        if DamageModule::damage(module_accessor, 0) >= 359.0 {
                            if DEAD == false {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD,true);
                                DEAD = true;
                            }
                        }
                    }
                    DamageModule::add_damage(module_accessor, 4.1, 0);
                    if StopModule::is_stop(module_accessor) {
                        StopModule::end_stop(module_accessor);
                    }
                    if StopModule::is_stop(boss_boma) {
                        StopModule::end_stop(boss_boma);
                    }
                }

                if ModelModule::scale(module_accessor) == 0.0001 {
                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {
                        MotionModule::set_rate(boss_boma, 4.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.0);
                    }
                }
                
                if DEAD == true {
                    if sv_information::is_ready_go() == true {
                        if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                        }
                    }
                }

                // DEATH CHECK
                if DEAD == true {
                    if sv_information::is_ready_go() == true {
                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                            }
                        }
                    }
                }

                let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                if FighterManager::is_result_mode(fighter_manager) == true {
                    if RESULT_SPAWNED == false {
                        RESULT_SPAWNED = true;
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DRACULA), 0, 0, false, false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START,true);
                    }
                }

                // SETS POWER

                AttackModule::set_power_mul(boss_boma, 1.5);

                // FIXES SPAWN

                if DEAD == false {
                    if sv_information::is_ready_go() == true {
                        if JUMP_START == false {
                            JUMP_START = true;
                            CONTROLLABLE = false;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                        }
                    }
                }

                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                    if CONTROLLABLE == true {
                        TELEPORTED = false;
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                        CONTROLLABLE = true;
                    }

                    if TELEPORTED == false {
                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_TELEPORT_START {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_WAIT {
                        CONTROLLABLE = true;
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_WAIT {
                        CONTROLLABLE = true;
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_WAIT {
                        CONTROLLABLE = true;
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {
                        CONTROLLABLE = true;
                    }

                    if TRANSFORMED_MODE == false {
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_STANDBY {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_STRAIGHT_END, true);
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_TELEPORT_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_3WAY_END {
                        if MotionModule::frame(boss_boma) == 27.0 {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_FILL_END {
                        if MotionModule::frame(boss_boma) == 37.0 {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_RUSH_END {
                        if MotionModule::frame(boss_boma) == 27.0 {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_PILLAR_END {
                        if MotionModule::frame(boss_boma) == 37.0 {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_STRAIGHT_END {
                        if MotionModule::frame(boss_boma) == 40.0 {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_TURN_3WAY_END {
                        if MotionModule::frame(boss_boma) == 27.0 {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_SQUASH_END {
                        if MotionModule::frame(boss_boma) == 27.0 {
                            CONTROLLABLE = true;
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_FIRE_SHOT_END {
                        if MotionModule::frame(boss_boma) == 27.0 {
                            CONTROLLABLE = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_HOMING_SHOT_END {
                        if MotionModule::frame(boss_boma) == 27.0 {
                            CONTROLLABLE = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_SQUASH_END_TURN {
                        if MotionModule::frame(boss_boma) == 10.0 {
                            CONTROLLABLE = true;
                        }
                    }
                    if CONTROLLABLE == true {
                        if TRANSFORMED_MODE == false {
                            if DEAD == false {
                                if sv_information::is_ready_go() == true {
                                    if ControlModule::get_stick_x(module_accessor) <= 1.0 {
                                        if KICKSTART_ANIM_BEGIN == false {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_WAIT, true);
                                            KICKSTART_ANIM_BEGIN = true;
                                        }
                                        if StatusModule::status_kind(boss_boma) != *ITEM_DRACULA_STATUS_KIND_WAIT {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_WAIT, true);
                                        }
                                    }
                                    if ControlModule::get_stick_x(module_accessor) >= 1.0 {
                                        if KICKSTART_ANIM_BEGIN == false {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_WAIT, true);
                                            KICKSTART_ANIM_BEGIN = true;
                                        }
                                        if StatusModule::status_kind(boss_boma) != *ITEM_DRACULA_STATUS_KIND_WAIT {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_WAIT, true);
                                        }
                                    }
                                    //Boss Control Movement
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_3WAY_START, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_TELEPORT_START, true);
                                        TELEPORTED = true;
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_FILL_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_RUSH_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_PILLAR_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_STRAIGHT_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_TURN_3WAY_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_TURN_3WAY_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_ATTACK_TURN_3WAY_START, true);
                                    }
                                }
                            }
                        }
                    }
                    if CONTROLLABLE == true {
                        if TRANSFORMED_MODE == true {
                            if DEAD == false {
                                if sv_information::is_ready_go() == true {
                                    //Boss Control Movement
                                    if StatusModule::status_kind(boss_boma) != *ITEM_DRACULA2_STATUS_KIND_TURN {
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                            if ControlModule::get_stick_x(module_accessor) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_TURN, true);
                                            }
                                        }
                                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                            if ControlModule::get_stick_x(module_accessor) < -0.1 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_TURN, true);
                                            }
                                        }
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_FRONT_JUMP, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_STEP_STRIKE, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_TELEPORT_START, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_SLASH, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_STEP_SLASH, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_HOMING_SHOT_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_FRONT_JUMP, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_FIRE_SHOT_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_SQUASH_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_BACK_JUMP, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_SHOCK_WAVE, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_SLASH_THREE, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_TURN_SLASH, true);
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
    acmd::add_custom_hooks!(once_per_fighter_frame);
}
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
static mut KICKSTART_ANIM_BEGIN : bool = false;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
pub static mut FIGHTER_NAME: [u64;5] = [0;5];

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
        if fighter_kind == *FIGHTER_KIND_MARIO {
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
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("GANON") {
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_GANONBOSS), 0, 0, false, false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        ModelModule::set_scale(boss_boma, 0.05);
                        MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        ModelModule::set_joint_rotate(module_accessor, smash::phx::Hash40::new("root") , &mut Vector3f{x: -270.0, y: 180.0, z: -90.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                    }
                }
                else {
                    if sv_information::is_ready_go() == false {
                        DEAD = false;
                        CONTROLLABLE = true;
                        JUMP_START = false;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            RESULT_SPAWNED = false;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_GANONBOSS), 0, 0, false, false);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                            if FighterUtil::is_hp_mode(module_accessor) == true {
                                DamageModule::add_damage(module_accessor, 1.1, 0);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            let attack = WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_ATTACK_KIND);
                            if CONTROLLABLE == true {
                                MotionModule::change_motion(boss_boma,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                if attack != 0 {
                                    //boss_private::main_energy_from_param(lua_state,ItemKind(*ITEM_KIND_GANON),Hash40::new("energy_param_wait"),0.0);
                                    StatusModule::change_status_request_from_script(boss_boma,attack,false);
                                    WorkModule::set_int(boss_boma, 0, *ITEM_INSTANCE_WORK_INT_ATTACK_KIND);
                                }
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
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    if sv_information::is_ready_go() == false {
                        if StopModule::is_damage(boss_boma) | StopModule::is_damage(module_accessor) {
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
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_GANONBOSS), 0, 0, false, false);
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
                                    JostleModule::set_status(boss_boma, true);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                }
                            }
                        }
                    }
                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_WALK_FRONT {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_WALK_BACK {
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_STANDBY {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_DOWN_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_ATTACK_LASER_BEAM_LOOP_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_ATTACK_SPIN_SLASH_END_REVERSE {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_ATTACK_BODY_ATTACK_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_ATTACK_SPIN_SLASH_END {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_ATTACK_LASER_BEAM_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_ATTACK_BIG_JUMP_END {
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_DOWN_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_GANONBOSS_STATUS_KIND_ATTACK_LASER_BEAM_LOOP_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if sv_information::is_ready_go() == true {
                            if CONTROLLABLE == true {
                                if DEAD == false {
                                    if StatusModule::status_kind(boss_boma) != *ITEM_GANONBOSS_STATUS_KIND_WALK_FRONT {
                                        if StatusModule::status_kind(boss_boma) != *ITEM_GANONBOSS_STATUS_KIND_WALK_BACK {
                                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                                if ControlModule::get_stick_x(module_accessor) < -0.1 {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_TURN, true);
                                                }
                                                if ControlModule::get_stick_x(module_accessor) > 0.0 {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_WALK_FRONT, true);
                                                }
                                            }
                                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                                if ControlModule::get_stick_x(module_accessor) > 0.0 {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_TURN, true);
                                                }
                                                if ControlModule::get_stick_x(module_accessor) < -0.1 {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_WALK_BACK, true);
                                                }
                                            }
                                            if ControlModule::get_stick_x(module_accessor) <= 1.0 {
                                                if KICKSTART_ANIM_BEGIN == false {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                                    KICKSTART_ANIM_BEGIN = true;
                                                }
                                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_WAIT {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                                }
                                            }
                                            if ControlModule::get_stick_x(module_accessor) >= 1.0 {
                                                if KICKSTART_ANIM_BEGIN == false {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                                    KICKSTART_ANIM_BEGIN = true;
                                                }
                                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_WAIT {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                                }
                                            }
                                            //Boss Moves
                                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_BACK_JUMP, true);
                                            }
                                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_JUMP_SLASH_START, true);
                                            }
                                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_LASER_BEAM_READY_TURN_JUMP, true);
                                            }
                                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_LASER_BEAM_HOLD, true);
                                            }
                                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_SPIN_SLASH_START, true);
                                            }
                                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_BODY_ATTACK_HOLD, true);
                                            }
                                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_HOMING_BOMB_HOLD, true);
                                            }
                                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_DOUBLE_SLASH_START, true);
                                            }
                                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_ATTACK_THUNDER_SLASH_START, true);
                                            }
                                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GANONBOSS_STATUS_KIND_RETURN_START, true);
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
    }
}

pub fn install() {
    acmd::add_custom_hooks!(once_per_fighter_frame);
}

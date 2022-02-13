use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::sv_information;
use smash::app::lua_bind;
use skyline::nn::ro::LookupSymbol;

static mut CONTROLLABLE : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        if WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) == 0 {
            let fighter_kind = smash::app::utility::get_kind(module_accessor);
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
            if fighter_kind == *FIGHTER_KIND_SAMUSD {
                let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                if sv_information::is_ready_go() == false {
                    DEAD = false;
                    CONTROLLABLE = true;
                    JUMP_START = false;
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        RESULT_SPAWNED = false;
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_GALLEOM), 0, 0, false, false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                    }
                }

                if DEAD == false {
                    if sv_information::is_ready_go() == true {
                        // SET POS AND STOPS OUT OF BOUNDS
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
                                if PostureModule::pos_y(boss_boma) >= 150.0 {
                                    let boss_y_pos_1 = Vector3f{x: x, y: 150.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    if PostureModule::pos_y(boss_boma) <= -100.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: -100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= 200.0 {
                                        let boss_x_pos_1 = Vector3f{x: 200.0, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= -200.0 {
                                        let boss_x_pos_2 = Vector3f{x: -200.0, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                }
                                else if PostureModule::pos_y(boss_boma) <= -100.0 {
                                    let boss_y_pos_2 = Vector3f{x: x, y: -100.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma) >= 200.0 {
                                        let boss_x_pos_1 = Vector3f{x: 200.0, y: -100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma) <= -200.0 {
                                        let boss_x_pos_2 = Vector3f{x: -200.0, y: -100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma) >= 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) >= 200.0 {
                                    let boss_x_pos_1 = Vector3f{x: 200.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma) <= -200.0 {
                                        let boss_x_pos_2 = Vector3f{x: -200.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma) >= 150.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: 150.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= -100.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: -100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) <= -200.0 {
                                    let boss_x_pos_2 = Vector3f{x: -200.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma) >= 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= -100.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: -100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= 200.0 {
                                        let boss_x_pos_1 = Vector3f{x: 200.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                }
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
                    if DamageModule::damage(module_accessor, 0) >= 500.0 {
                        if DEAD == false {
                            DEAD = true;
                            CONTROLLABLE = false;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD,true);
                        }
                    }
                    if DamageModule::damage(module_accessor, 0) < 0.0 {
                        if DamageModule::damage(module_accessor, 0) >= -1.0 {
                            if DEAD == false {
                                DEAD = true;
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD,true);
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
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_GALLEOM), 0, 0, false, false);
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
                    if sv_information::is_ready_go() == true {
                        if CONTROLLABLE == true {
                            if DEAD == false {
                                if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TURN {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_FALL {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_EXIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_PASS_FLOOR {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_START {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TERM {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_INITIALIZE {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_SHOOT_END {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_RUSH_FINISH_END {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_GRAB_MISS {

                                }
                                else {
                                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                    }
                                }
                            }
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                        CONTROLLABLE = true;
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_WAIT {
                        CONTROLLABLE = true;
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {
                        CONTROLLABLE = true;
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_STANDBY {
                        CONTROLLABLE = true;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_HAMMER_KNUCKLE, true);
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_SHOOT_END {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_RUSH_FINISH_END {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_GRAB_MISS {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_GALLEOM_STATUS_KIND_TURN {
                        CONTROLLABLE = true;
                    }
                    if CONTROLLABLE == true {
                        if DEAD == false {
                            if sv_information::is_ready_go() == true {
                                if StatusModule::status_kind(boss_boma) != *ITEM_GALLEOM_STATUS_KIND_WALK_FRONT {
                                    if StatusModule::status_kind(boss_boma) != *ITEM_GALLEOM_STATUS_KIND_WALK_BACK {
                                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                            if ControlModule::get_stick_x(module_accessor) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_WALK_FRONT, true);
                                            }
                                            if ControlModule::get_stick_x(module_accessor) < -0.1 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_WALK_BACK, true);
                                            }
                                        }
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                            if ControlModule::get_stick_x(module_accessor) < -0.1 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_WALK_FRONT, true);
                                            }
                                            if ControlModule::get_stick_x(module_accessor) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_WALK_BACK, true);
                                            }
                                        }
                                    }
                                }
                                //Boss Moves
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_SHOOT_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_TURN, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_UPPERCUT, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_GRAB, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_RUSH_TRANSFORM, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_TURN_GRAB, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_DOUBLE_LARIAT_SQUAT, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_HAMMER_KNUCKLE, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_LARGE_JUMP_SQUAT, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_FALL_DOWN, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_GALLEOM_STATUS_KIND_DOUBLE_ARM_PRESS, true);
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

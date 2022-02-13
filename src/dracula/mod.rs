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
            if fighter_kind == *FIGHTER_KIND_CHROM {
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
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DRACULA), 0, 0, false, false);
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
                                    ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DRACULA2), 0, 0, false, false);
                                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                                    PostureModule::set_pos(module_accessor, &none_pos);
                                    PostureModule::set_pos(boss_boma, &none_pos);
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                                    TRANSITIONING = false;
                                }
                            }
                        }
                    }
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
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_TELEPORT_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_3WAY_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_FILL_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_RUSH_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_PILLAR_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_STRAIGHT_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_TURN_3WAY_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_SQUASH_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_FIRE_SHOT_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_HOMING_SHOT_END {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_SQUASH_END_TURN {

                                    }
                                    else if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_CHANGE_START {

                                    }
                                    else {
                                        if TRANSFORMED_MODE == false {
                                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                            }
                                        }
                                        if TRANSFORMED_MODE == true {
                                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_WAIT, true);
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_3WAY_END {
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_FILL_END {
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_RUSH_END {
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_PILLAR_END {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_STRAIGHT_END {
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA_STATUS_KIND_ATTACK_TURN_3WAY_END {
                            CONTROLLABLE = true;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_SQUASH_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_FIRE_SHOT_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_HOMING_SHOT_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DRACULA2_STATUS_KIND_SQUASH_END_TURN {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
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
                                        if TRANSITIONING == true {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA_STATUS_KIND_CHANGE_START, true);
                                        }
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
                                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DRACULA2_STATUS_KIND_TURN, true);
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
    }

pub fn install() {
    acmd::add_custom_hooks!(once_per_fighter_frame);
}
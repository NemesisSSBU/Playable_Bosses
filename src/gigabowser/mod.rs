use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::sv_information;
use skyline::nn::ro::LookupSymbol;
use smash::app::lua_bind;
use smash_script::macros::WHOLE_HIT;

static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut IS_BOSS_DEAD : bool = false;
static mut STOP_CONTROL_LOOP : bool = true;
static mut STOP_CONTROL_LOOP_MAIN : bool = true;
static mut CHARACTER_IS_TURNING : bool = false;
static mut TELEPORTED : bool = false;

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
            ENTRY_ID = WorkModule::get_int(module_accessor,*FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
            LookupSymbol(
                &mut FIGHTER_MANAGER,
                "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
                .as_bytes()
                .as_ptr(),
            );
            if fighter_kind == *FIGHTER_KIND_MARIO {
                let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                    if sv_information::is_ready_go() == false {
                        IS_BOSS_DEAD = false;
                        CHARACTER_IS_TURNING = false;
                        STOP_CONTROL_LOOP = true;
                        STOP_CONTROL_LOOP_MAIN = true;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                        }
                    }
                    if IS_BOSS_DEAD == false {
                        if sv_information::is_ready_go() == true {
                            //SET POS
                            if ModelModule::scale(module_accessor) == 0.0001 {
                                if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                                }
                                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_STANDBY {
                                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                    let x = PostureModule::pos_x(boss_boma);
                                    let y = PostureModule::pos_y(boss_boma);
                                    let y_owner = PostureModule::pos_y(module_accessor);
                                    let z = PostureModule::pos_z(boss_boma);
                                    let boss_pos = Vector3f{x: x, y: y + 7.0, z: z};
                                    let fighter_pos_fix = Vector3f{x: x, y: y_owner + 7.0, z: z};
                                    if PostureModule::pos_y(boss_boma) >= 220.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: 220.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        if PostureModule::pos_y(boss_boma) <= -130.0 {
                                            let boss_y_pos_2 = Vector3f{x: x, y: -130.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= 130.0 {
                                            let boss_x_pos_1 = Vector3f{x: 130.0, y: 220.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= -100.0 {
                                            let boss_x_pos_2 = Vector3f{x: -100.0, y: 220.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_y(boss_boma) <= -130.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: -130.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        if PostureModule::pos_x(boss_boma) >= 220.0 {
                                            let boss_x_pos_1 = Vector3f{x: 220.0, y: -130.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= -220.0 {
                                            let boss_x_pos_2 = Vector3f{x: -220.0, y: -130.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma) >= 220.0 {
                                            let boss_y_pos_1 = Vector3f{x: x, y: 220.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) >= 220.0 {
                                        let boss_x_pos_1 = Vector3f{x: 220.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        if PostureModule::pos_x(boss_boma) <= -220.0 {
                                            let boss_x_pos_2 = Vector3f{x: -220.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma) >= 220.0 {
                                            let boss_y_pos_1 = Vector3f{x: 220.0, y: 220.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= -130.0 {
                                            let boss_y_pos_2 = Vector3f{x: 220.0, y: -130.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) <= -220.0 {
                                        let boss_x_pos_2 = Vector3f{x: -220.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        if PostureModule::pos_y(boss_boma) >= 220.0 {
                                            let boss_y_pos_1 = Vector3f{x: -220.0, y: 220.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= -130.0 {
                                            let boss_y_pos_2 = Vector3f{x: -220.0, y: -130.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= 220.0 {
                                            let boss_x_pos_1 = Vector3f{x: 220.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                    }
                                    else {
                                        if StatusModule::situation_kind(fighter.module_accessor) == *SITUATION_KIND_GROUND {
                                            PostureModule::set_pos(module_accessor, &fighter_pos_fix);
                                        }
                                        else {
                                            PostureModule::set_pos(module_accessor, &boss_pos);
                                        }
                                    }
                                }
                            }
                        }
                        //DAMAGE MODULES

                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        DamageModule::set_damage_lock(boss_boma, true);
                        WHOLE_HIT(fighter, *HIT_STATUS_XLU);
                        HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);

                        if StopModule::is_damage(boss_boma) {
                            if DamageModule::damage(module_accessor, 0) >= 359.0 {
                                if IS_BOSS_DEAD == false {
                                    IS_BOSS_DEAD = true;
                                    StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                    StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                                }
                            }
                            if DamageModule::damage(module_accessor, 0) < 0.0 {
                                if DamageModule::damage(module_accessor, 0) >= -1.0 {
                                    if IS_BOSS_DEAD == false {
                                        IS_BOSS_DEAD = true;
                                        StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                        StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
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

                        //DISABLES AI

                        if STOP_CONTROL_LOOP_MAIN == false {
                            if MotionModule::frame(fighter.module_accessor) >= 5.0 {
                                STOP_CONTROL_LOOP_MAIN = true;
                            }
                        }

                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP_MAIN = true;
                            STOP_CONTROL_LOOP = true;
                            CHARACTER_IS_TURNING = false;
                        }
                        
                        if sv_information::is_ready_go() == true {
                            if CHARACTER_IS_TURNING == false {
                                if IS_BOSS_DEAD == false {
                                    if STOP_CONTROL_LOOP | STOP_CONTROL_LOOP_MAIN == true {
                                        if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT | *ITEM_STATUS_KIND_WAIT | *ITEM_STATUS_KIND_NONE {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_NONE, true);
                                        }
                                    }
                                }
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TURN {
                            CHARACTER_IS_TURNING = true;
                        }
                        if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_TURN {
                            CHARACTER_IS_TURNING = false;
                        }

                        if CHARACTER_IS_TURNING == false {
                            if STOP_CONTROL_LOOP_MAIN == true {
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_UP {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START_MOVE {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT {
                                    STOP_CONTROL_LOOP = true
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                                    STOP_CONTROL_LOOP = true
                                }
                            }
                        }

                        //CONTROLS
                        if CHARACTER_IS_TURNING == false {
                            if STOP_CONTROL_LOOP == true {
                                if IS_BOSS_DEAD == false {
                                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                    //Boss Control Stick Movement
                                    if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                        let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * - 2.0 * ControlModule::get_stick_x(module_accessor), y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                    }
                                
                                    if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                        let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0 * ControlModule::get_stick_x(module_accessor), y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                    }
                                
                                    if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                        let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * - 2.0 * ControlModule::get_stick_y(module_accessor), z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                    }
                                
                                    if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                        let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0 * ControlModule::get_stick_y(module_accessor), z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                    }
                                    if CHARACTER_IS_TURNING == false {
                                        //Boss Moves
                                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                            if ControlModule::get_stick_x(module_accessor) < -0.1 {
                                                if CHARACTER_IS_TURNING == false {
                                                    CHARACTER_IS_TURNING = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                            if ControlModule::get_stick_x(module_accessor) > -0.0 {
                                                if CHARACTER_IS_TURNING == false {
                                                    CHARACTER_IS_TURNING = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                                        }
                                        if STOP_CONTROL_LOOP == true {
                                            TELEPORTED = false;
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                                            if TELEPORTED == false {
                                                MotionModule::set_rate(boss_boma, 0.0);
                                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 0.0);
                                                //Boss Control Stick Movement
                                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                                        let pos = Vector3f{x: -140.0, y: 0.0, z: 0.0};
                                                        PostureModule::add_pos(boss_boma, &pos);
                                                        TELEPORTED = true;
                                                }
                                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                                        let pos = Vector3f{x: 140.0, y: 0.0, z: 0.0};
                                                        PostureModule::add_pos(boss_boma, &pos);
                                                        TELEPORTED = true;
                                                }
                                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                                        let pos = Vector3f{x: 0.0, y: -50.0, z: 0.0};
                                                        PostureModule::add_pos(boss_boma, &pos);
                                                        TELEPORTED = true;
                                                }
                                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                                        let pos = Vector3f{x: 0.0, y: 50.0, z: 0.0};
                                                        PostureModule::add_pos(boss_boma, &pos);
                                                        TELEPORTED = true;
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            if PostureModule::pos_y(boss_boma) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 25.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -25.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_HOMING, true);
                                                        }
                                                        else {
                                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START, true);
                                                            }
                                                    }
                                                    else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START, true);
                                                        }
                                                }
                                                else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START, true);
                                                    }
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            if PostureModule::pos_y(boss_boma) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 40.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -40.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                                            }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                                        }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                                    }
                                            }
                                            else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            if PostureModule::pos_y(boss_boma) <= 45.0 {
                                                if PostureModule::pos_y(boss_boma) >= -45.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 45.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -25.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START, true);
                                                        }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START, true);
                                                    }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START, true);
                                                }
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            if PostureModule::pos_y(boss_boma) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 75.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -75.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START, true);
                                                        }
                                                        else {
                                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_UP, true);
                                                            }
                                                    }
                                                    else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_UP, true);
                                                        }
                                                }
                                                else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_UP, true);
                                                    }
                                                }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_UP, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            if PostureModule::pos_y(boss_boma) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 40.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -40.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                                            }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                                        }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                                    }
                                            }
                                            else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_GOOPAA, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                            STOP_CONTROL_LOOP = false;
                                            STOP_CONTROL_LOOP_MAIN = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_PRE_MOVE, true);
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
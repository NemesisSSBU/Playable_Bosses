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

static mut SPAWN_BOSS : bool = true;
static mut TELEPORTED : bool = false;
static mut HAVE_ITEM : bool = true;
static mut CHARACTER_IS_TURNING : bool = false;
static mut ENTRANCE_ANIM : bool = false;
static mut STOP_CONTROL_LOOP : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
static mut IS_BOSS_DEAD : bool = false;

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
        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
        if SPAWN_BOSS == true {
            if fighter_kind == *FIGHTER_KIND_MARIO {
                let x = PostureModule::pos_x(boss_boma);
                let y = PostureModule::pos_y(boss_boma);
                let z = PostureModule::pos_z(boss_boma);
                let boss_pos = Vector3f{x: x, y: y, z: z};
                if PostureModule::pos_y(boss_boma) >= 220.0 {
                    let boss_y_pos = Vector3f{x: x, y: 220.0, z: z};
                    PostureModule::set_pos(module_accessor, &boss_y_pos);
                }
                else if PostureModule::pos_y(boss_boma) <= -100.0 {
                    let boss_y_pos = Vector3f{x: x, y: -100.0, z: z};
                    PostureModule::set_pos(module_accessor, &boss_y_pos);
                }
                else if PostureModule::pos_x(boss_boma) >= 200.0 {
                    let boss_x_pos = Vector3f{x: 200.0, y: y, z: z};
                    PostureModule::set_pos(module_accessor, &boss_x_pos);
                }
                else if PostureModule::pos_x(boss_boma) <= -220.0 {
                    let boss_x_pos = Vector3f{x: -220.0, y: y, z: z};
                    PostureModule::set_pos(module_accessor, &boss_x_pos);
                }
                else {
                    PostureModule::set_pos(module_accessor, &boss_pos);
                }
                if MotionModule::frame(fighter.module_accessor) >= 29.0 {
                    if sv_information::is_ready_go() == false {
                        HAVE_ITEM = false;
                        IS_BOSS_DEAD = false;
                        CHARACTER_IS_TURNING = false;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if SPAWN_BOSS == true {
                            //let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            
                            ENTRANCE_ANIM = true;
                            ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_MASTERHAND),*ITEM_GEN_LEVEL_VERY_HARD,0,true,true);
                                BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                            ModelModule::set_scale(module_accessor, 0.0001);
                            //DamageModule::add_damage(module_accessor, DamageModule::damage(module_accessor, 0) -1.0, 0);
                            //DamageModule::add_damage(boss_boma, 149.0, 0);
                            //DamageModule::add_damage(module_accessor, -299.0, 0);
                            //DamageModule::add_damage(boss_boma, -299.0, 0);
                            //DamageModule::add_damage(boss_boma, 299.0, 0);
                            StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                            //ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_ENTRY, true);
                            //StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                            HAVE_ITEM = true;
                        }
                    }
                }
            }
            if STOP_CONTROL_LOOP == false {
                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT {
                    if TELEPORTED == false {
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
            }
            if CHARACTER_IS_TURNING == false {
                if STOP_CONTROL_LOOP == true {
                    if IS_BOSS_DEAD == false {
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
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                            }
                            if STOP_CONTROL_LOOP == true {
                                TELEPORTED = false;
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                                if TELEPORTED == false {
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
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                STOP_CONTROL_LOOP = false;
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
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                STOP_CONTROL_LOOP = false;
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
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_GOOPAA, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                STOP_CONTROL_LOOP = false;
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

pub fn install() {
acmd::add_custom_hooks!(once_per_fighter_frame);
}
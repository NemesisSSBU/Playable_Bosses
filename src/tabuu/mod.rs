use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Vector3f;
use skyline::nn::ro::LookupSymbol;
use smash::app::sv_information;

static mut ENTRY_ID : usize = 0;
pub static mut FIGHTER_MANAGER: usize = 0;
static mut STOP_CONTROL_LOOP : bool = true;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        if WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_COLOR) == 0 {
            let fighter_kind = smash::app::utility::get_kind(module_accessor);
            ENTRY_ID = WorkModule::get_int(module_accessor,*FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
            LookupSymbol(
                &mut FIGHTER_MANAGER,
                "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
                .as_bytes()
                .as_ptr(),
            );
            if fighter_kind == *FIGHTER_KIND_MEWTWO {
                if sv_information::is_ready_go() == true {
                    let x = PostureModule::pos_x(module_accessor);
                    let y = PostureModule::pos_y(module_accessor);
                    let z = PostureModule::pos_z(module_accessor);
                    let boss_pos = Vector3f{x: x, y: y, z: z};
                    if PostureModule::pos_y(module_accessor) >= 220.0 {
                        let boss_y_pos_1 = Vector3f{x: x, y: 220.0, z: z};
                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                        if PostureModule::pos_y(module_accessor) <= -130.0 {
                            let boss_y_pos_2 = Vector3f{x: x, y: -130.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                        }
                        if PostureModule::pos_x(module_accessor) >= 130.0 {
                            let boss_x_pos_1 = Vector3f{x: 130.0, y: 220.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                        }
                        if PostureModule::pos_x(module_accessor) <= -100.0 {
                            let boss_x_pos_2 = Vector3f{x: -100.0, y: 220.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                        }
                    }
                    else if PostureModule::pos_y(module_accessor) <= -130.0 {
                        let boss_y_pos_2 = Vector3f{x: x, y: -130.0, z: z};
                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                        if PostureModule::pos_x(module_accessor) >= 220.0 {
                            let boss_x_pos_1 = Vector3f{x: 220.0, y: -130.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                        }
                        if PostureModule::pos_x(module_accessor) <= -220.0 {
                            let boss_x_pos_2 = Vector3f{x: -220.0, y: -130.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                        }
                        if PostureModule::pos_y(module_accessor) >= 220.0 {
                            let boss_y_pos_1 = Vector3f{x: x, y: 220.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                        }
                    }
                    else if PostureModule::pos_x(module_accessor) >= 220.0 {
                        let boss_x_pos_1 = Vector3f{x: 220.0, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                        if PostureModule::pos_x(module_accessor) <= -220.0 {
                            let boss_x_pos_2 = Vector3f{x: -220.0, y: y, z: z};
                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                        }
                        if PostureModule::pos_y(module_accessor) >= 220.0 {
                            let boss_y_pos_1 = Vector3f{x: 220.0, y: 220.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                        }
                        if PostureModule::pos_y(module_accessor) <= -130.0 {
                            let boss_y_pos_2 = Vector3f{x: 220.0, y: -130.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                        }
                    }
                    else if PostureModule::pos_x(module_accessor) <= -220.0 {
                        let boss_x_pos_2 = Vector3f{x: -220.0, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                        if PostureModule::pos_y(module_accessor) >= 220.0 {
                            let boss_y_pos_1 = Vector3f{x: -220.0, y: 220.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                        }
                        if PostureModule::pos_y(module_accessor) <= -130.0 {
                            let boss_y_pos_2 = Vector3f{x: -220.0, y: -130.0, z: z};
                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                        }
                        if PostureModule::pos_x(module_accessor) >= 220.0 {
                            let boss_x_pos_1 = Vector3f{x: 220.0, y: y, z: z};
                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                        }
                    }
                    else {
                        PostureModule::set_pos(module_accessor, &boss_pos);
                    }
                }
            }
            let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                if fighter_kind == *FIGHTER_KIND_MEWTWO {
                    if smash::app::sv_information::is_ready_go() == false {
                        smash::app::lua_bind::MotionModule::enable_shift_material_animation(module_accessor, false);
                        ModelModule::set_scale(module_accessor, 0.006);
                    }
                }
            }
            else {
                if fighter_kind == *FIGHTER_KIND_MEWTWO {
                    if smash::app::sv_information::is_ready_go() == false {
                        smash::app::lua_bind::MotionModule::enable_shift_material_animation(module_accessor, false);
                        ModelModule::set_scale(module_accessor, 0.006);
                    }
                    //Boss Control Stick Movement
                    if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                        let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.5, y: 0.0, z: 0.0};
                        PostureModule::add_pos(module_accessor, &pos);
                    }
                
                    if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                        let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.5, y: 0.0, z: 0.0};
                        PostureModule::add_pos(module_accessor, &pos);
                    }
                
                    if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                        let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                        PostureModule::add_pos(module_accessor, &pos);
                    }
                
                    if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                        let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.5, z: 0.0};
                        PostureModule::add_pos(module_accessor, &pos);
                    }
                    if MotionModule::frame(module_accessor) == MotionModule::end_frame(module_accessor) {
                        STOP_CONTROL_LOOP = true;
                    }
                    if STOP_CONTROL_LOOP == true {
                        //Boss Moves
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_HI_2, true);
                        }
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_HI_2, true);
                        }
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_N_SHOOT, true);
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_HI_2, true);
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_HI_2, true);
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_S_THROW, true);
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_N_MAX, true);
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_HI_2, true);
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                            STOP_CONTROL_LOOP = false;
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_MEWTWO_STATUS_KIND_SPECIAL_N_SHOOT, true);
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

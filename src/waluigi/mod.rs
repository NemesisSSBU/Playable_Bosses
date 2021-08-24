use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::lib::lua_const::HIT_STATUS_XLU;
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash_script::macros::WHOLE_HIT;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::sv_information;
use skyline::nn::ro::LookupSymbol;

static mut SPAWN_ASSIST : bool = true;
static mut HAVE_ITEM : bool = false;
static mut ASSIST_TROPHEY_SPAWNED : bool = false;
static mut CHARACTER_IS_TURNING : bool = false;
static mut ENTRANCE_ANIM : bool = false;
static mut STOP_CONTROL_LOOP : bool = false;
static mut ENTRY_ID : usize = 0;
pub static mut FIGHTER_MANAGER: usize = 0;
static mut ASSIST_ID : [u32; 8] = [0; 8];
static mut IS_ASSIST_DEAD : bool = false;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
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
        let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
            if fighter_kind == *FIGHTER_KIND_SIMON {

            }
        }
        else {
            let assist_boma = sv_battle_object::module_accessor(ASSIST_ID[entry_id(module_accessor)]);
            if SPAWN_ASSIST == true {
                if fighter_kind == *FIGHTER_KIND_SIMON {
                    if MotionModule::frame(fighter.module_accessor) >= 29.0 {
                        if sv_information::is_ready_go() == false {
                            ASSIST_TROPHEY_SPAWNED = false;
                            HAVE_ITEM = false;
                            IS_ASSIST_DEAD = false;
                            CHARACTER_IS_TURNING = false;
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            if SPAWN_ASSIST == true {
                                let assist_boma = sv_battle_object::module_accessor(ASSIST_ID[entry_id(module_accessor)]);
                                
                                ENTRANCE_ANIM = false;
                                ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_WALUIGI),0,0,false,false);
                                    ASSIST_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                                ModelModule::set_scale(module_accessor,0.0001);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                                //ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_WAIT, true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                                HAVE_ITEM = true;
                            }
                        }
                    }

                    DamageModule::set_damage_lock(assist_boma,true);
                    WHOLE_HIT(fighter, *HIT_STATUS_XLU);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);

                    if StopModule::is_damage(assist_boma) {
                        DamageModule::add_damage(module_accessor, 0.5, 0);
                    }

                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                        if IS_ASSIST_DEAD == false {
                            IS_ASSIST_DEAD = true;
                            StatusModule::change_status_request_from_script(assist_boma,*ITEM_STATUS_KIND_DEAD,true);
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        STOP_CONTROL_LOOP = true;
                        if MotionModule::frame(fighter.module_accessor) >= 30.0 {
                            StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_WAIT, true);
                        }
                    }
                    if sv_information::is_ready_go() == true {
                        ASSIST_TROPHEY_SPAWNED = true;
                    }

                    if MotionModule::frame(fighter.module_accessor) >= 30.0 {
                        if sv_information::is_ready_go() == true {
                            HAVE_ITEM = true;
                        }
                    }
                    if STOP_CONTROL_LOOP == true {
                        if CHARACTER_IS_TURNING == true {

                        }
                        else {
                            MotionModule::set_rate(assist_boma, 1.0);
                        }
                    }

                    if MotionModule::frame(assist_boma) == MotionModule::end_frame(assist_boma) {
                        STOP_CONTROL_LOOP = true;
                        CHARACTER_IS_TURNING = false;
                        StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_WAIT, true);
                    }
                    if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_WAIT {
                        STOP_CONTROL_LOOP = true;
                        CHARACTER_IS_TURNING = false;
                    }
                    if STOP_CONTROL_LOOP == true {
                        if HAVE_ITEM == true {
                            if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_WAIT {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_ENTRY {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_RUN {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_TURN {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_NONE {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_JUMP {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_JUMP_AIR {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_FALL {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_EXIT {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_PASS_FLOOR {

                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_START {
                                
                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_TERM {
                                
                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_LANDING {
                                
                            }
                            else if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_INITIALIZE {

                            }
                            else {
                                if StatusModule::status_kind(assist_boma) == *ITEM_STATUS_KIND_ENTRY {

                                }
                                else {
                                    StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_WAIT, true);
                                }
                            }
                        }
                    }

                        if STOP_CONTROL_LOOP == true {
                            if ASSIST_TROPHEY_SPAWNED == true {
                            //Assist Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let vec3 = Vector3f{x: 0.0, y: -180.0, z: 0.0};
                                PostureModule::set_rot(assist_boma,&vec3,0);
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_RUN, true);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let vec3 = Vector3f{x: 0.0, y: 0.0, z: 0.0};
                                PostureModule::set_rot(assist_boma,&vec3,0);
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_RUN, true);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.5 {
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_JUMP, true);
                            }
                        
                            //Assist Moves
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_JUMP, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_CAPTURE, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_DAMAGE, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_JUMP_AIR, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_JUMP_AIR, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_RUN, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_DAMAGE, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_DAMAGE, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(assist_boma, *ITEM_STATUS_KIND_DAMAGE, true);
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

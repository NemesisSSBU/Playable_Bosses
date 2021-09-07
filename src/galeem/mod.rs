use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash_script::macros::WHOLE_HIT;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::sv_information;
use skyline::nn::ro::LookupSymbol;
use smash::app::lua_bind;

static mut SPAWN_BOSS : bool = true;
static mut HAVE_ITEM : bool = false;
static mut ENTRANCE_ANIM : bool = false;
static mut STOP_CONTROL_LOOP : bool = true;
static mut IS_ANGRY : bool = false;
static mut ENTRY_ID : usize = 0;
static mut CURRENT_HEALTH : f32 = 0.0;
static mut BOSS_ID : [u32; 8] = [0; 8];
static mut IS_BOSS_DEAD : bool = false;
pub static mut FIGHTER_MANAGER: usize = 0;

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
            if fighter_kind == *FIGHTER_KIND_DAISY {
                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                if HAVE_ITEM == true {
                    if IS_BOSS_DEAD == false {
                        if sv_information::is_ready_go() == false {
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_KIILA_STATUS_KIND_MANAGER_WAIT,true);
                            if lua_bind::PostureModule::lr(module_accessor) == -1.0 { // left
                                let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                PostureModule::set_rot(boss_boma,&vec3,0);
                            }
                            if lua_bind::PostureModule::lr(module_accessor) == 1.0 { // right
                                let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                PostureModule::set_rot(boss_boma,&vec3,0);
                            }
                        }
                    }
                }
                if HAVE_ITEM == true {
                    if IS_BOSS_DEAD == false {
                        if sv_information::is_ready_go() == true {
                            StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_FALL_SPECIAL,true);
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
            }
            let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                if fighter_kind == *FIGHTER_KIND_DAISY {
                    if MotionModule::frame(fighter.module_accessor) >= 29.0 {
                        if sv_information::is_ready_go() == false {
                            HAVE_ITEM = false;
                            IS_BOSS_DEAD = false;
                            IS_ANGRY = false;
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            if SPAWN_BOSS == true {
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                
                                ENTRANCE_ANIM = false;
                                ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_KIILA),0,0,false,false);
                                    BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                                ModelModule::set_scale(module_accessor,0.0001);
                                HAVE_ITEM = true;
                                ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT, true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        CURRENT_HEALTH = DamageModule::damage(module_accessor,0);
                    }

                    
                    if HAVE_ITEM == true {
                        if DamageModule::damage(module_accessor, 0) >= 200.0 {
                            if IS_ANGRY == false {
                                IS_ANGRY = true;
                                DamageModule::add_damage(module_accessor, 4.1, 0);
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY,true);
                            }
                        }
                    }
                    
                    if HAVE_ITEM == true {
                        if DamageModule::damage(module_accessor, 0) <= -200.0 {
                            if IS_ANGRY == false {
                                IS_ANGRY = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY,true);
                            }
                        }
                    }

                    DamageModule::set_damage_lock(boss_boma,true);
                    WHOLE_HIT(fighter, *HIT_STATUS_XLU);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);

                    if StopModule::is_damage(boss_boma) {
                        if DamageModule::damage(module_accessor, 0) == 1.0 {
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_START,true);
                            DamageModule::add_damage(module_accessor, 4.1, 0);
                        }
                        if DamageModule::damage(module_accessor, 0) >= 399.0 {
                            if IS_BOSS_DEAD == false {
                                IS_BOSS_DEAD = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                            }
                        }
                        if DamageModule::damage(module_accessor, 0) == -1.0 {
                            if IS_BOSS_DEAD == false {
                                IS_BOSS_DEAD = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
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

                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                        if IS_BOSS_DEAD == false {
                            IS_BOSS_DEAD = true;
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                        }
                    }

                    if IS_BOSS_DEAD == true {
                        if sv_information::is_ready_go() == true {
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                                DamageModule::add_damage(module_accessor, 300.0, 0);
                                STOP_CONTROL_LOOP = false;
                            }
                        }
                    }

                    if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                        }
                    }

                    if MotionModule::frame(fighter.module_accessor) >= 30.0 {
                        if sv_information::is_ready_go() == true {
                            HAVE_ITEM = true;
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        if HAVE_ITEM == true {
                            if IS_BOSS_DEAD == false {
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_KIILA_STATUS_KIND_MANAGER_WAIT,true);
                            }
                        }
                    }
                }
            }
            else {
                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                if SPAWN_BOSS == true {
                    if fighter_kind == *FIGHTER_KIND_DAISY {
                        if MotionModule::frame(fighter.module_accessor) >= 9.0 {
                            if sv_information::is_ready_go() == false {
                                HAVE_ITEM = false;
                                IS_BOSS_DEAD = false;
                                IS_ANGRY = false;
                                let lua_state = fighter.lua_state_agent;
                                let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                                ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                                if SPAWN_BOSS == true {
                                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                    
                                    ENTRANCE_ANIM = false;
                                    ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_KIILA),0,0,false,false);
                                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                                    ModelModule::set_scale(module_accessor,0.0001);
                                    HAVE_ITEM = true;
                                    ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                                    StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT, true);
                                    StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                                }
                            }
                        }

                        if MotionModule::frame(fighter.module_accessor) >= 10.0 {
                            if sv_information::is_ready_go() == true {
                                HAVE_ITEM = true;
                            }
                        }

                        if HAVE_ITEM == true {

                        if sv_information::is_ready_go() == false {
                            CURRENT_HEALTH = DamageModule::damage(module_accessor,0);
                        }

                        if IS_BOSS_DEAD == true {
                            if sv_information::is_ready_go() == true {
                                if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                                    StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                                    DamageModule::add_damage(module_accessor, 400.0, 0);
                                    STOP_CONTROL_LOOP = false;
                                }
                            }
                        }

                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                            if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                            }
                        }

                        if HAVE_ITEM == true {
                            if DamageModule::damage(module_accessor, 0) >= 200.0 {
                                if IS_ANGRY == false {
                                    IS_ANGRY = true;
                                    DamageModule::add_damage(module_accessor, 4.1, 0);
                                    StatusModule::change_status_request_from_script(boss_boma,*ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY,true);
                                }
                            }
                        }

                        if IS_BOSS_DEAD == true {
                            if sv_information::is_ready_go() == true {
                                STOP_CONTROL_LOOP = false;
                            }
                        }
                        
                        if HAVE_ITEM == true {
                            if sv_information::is_ready_go() == true {
                                if DamageModule::damage(module_accessor, 0) <= -200.0 {
                                    if IS_ANGRY == false {
                                        IS_ANGRY = true;
                                        StatusModule::change_status_request_from_script(boss_boma,*ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY,true);
                                    }
                                }
                            }
                        }

                        DamageModule::set_damage_lock(boss_boma,true);
                        WHOLE_HIT(fighter, *HIT_STATUS_XLU);
                        HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);

                        if StopModule::is_damage(boss_boma) {
                            if DamageModule::damage(module_accessor, 0) >= 399.0 {
                                if IS_BOSS_DEAD == false {
                                    IS_BOSS_DEAD = true;
                                    StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                    StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                                }
                            }
                            if DamageModule::damage(module_accessor, 0) == -1.0 {
                                if IS_BOSS_DEAD == false {
                                    IS_BOSS_DEAD = true;
                                    StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                    StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
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

                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                            if IS_BOSS_DEAD == false {
                                IS_BOSS_DEAD = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                            }
                        }
                    
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_TELEPORT {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN_LOOP {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER_WAIT {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_START {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TERM {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_EXIT {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_LOOP {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_END {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CROSS_BOMB {
                            STOP_CONTROL_LOOP = true;
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                STOP_CONTROL_LOOP = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_START {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_LOOP {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_START {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_END {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_LOOP {
                            STOP_CONTROL_LOOP = false;
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY {
                            STOP_CONTROL_LOOP = false;
                            if MotionModule::frame(fighter.module_accessor) >= 100.0 {
                                STOP_CONTROL_LOOP = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_THREAT_END {
                            STOP_CONTROL_LOOP = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_THREAT_START {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN_START {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR {
                            STOP_CONTROL_LOOP = false;
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_TORRENT {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START {
                            STOP_CONTROL_LOOP = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER {
                            STOP_CONTROL_LOOP = false;
                        }
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }

                        if STOP_CONTROL_LOOP == true {
                            if IS_BOSS_DEAD == false {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                //Boss Moves
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CROSS_BOMB, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TELEPORT, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_THREAT_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TORRENT, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                    STOP_CONTROL_LOOP = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH, true);
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
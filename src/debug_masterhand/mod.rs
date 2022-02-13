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
use smash_script::macros::WHOLE_HIT;

static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DEAD : bool = false;
static mut FULLY_DEAD : bool = false;
static mut CONTROLLABLE : bool = true;
static mut MULTIPLE_BULLETS : usize = 0;
static mut JUMP_START : bool = false;
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
                if sv_information::is_ready_go() == false {
                    DEAD = false;
                    FULLY_DEAD = false;
                    CONTROLLABLE = true;
                    JUMP_START = false;
                    TELEPORTED = false;
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

                //DAMAGE MODULES
                
                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                DamageModule::set_damage_lock(boss_boma, true);
                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);
                WHOLE_HIT(fighter, *HIT_STATUS_XLU);
                if DamageModule::damage(module_accessor, 0) >= 359.0 {
                    if DEAD == false {
                        DEAD = true;
                        CONTROLLABLE = false;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD,true);
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                    }
                }
                if DamageModule::damage(module_accessor, 0) < 0.0 {
                    if DamageModule::damage(module_accessor, 0) >= -1.0 {
                        if DEAD == false {
                            DEAD = true;
                            CONTROLLABLE = false;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD,true);
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                        }
                    }
                }

                if StopModule::is_damage(boss_boma) {
                    DamageModule::add_damage(module_accessor, 4.1, 0);
                    if StopModule::is_stop(module_accessor) {
                        StopModule::end_stop(module_accessor);
                    }
                    if StopModule::is_stop(boss_boma) {
                        StopModule::end_stop(boss_boma);
                    }
                }

                // DEATH CHECK

                if DEAD == true {
                    if sv_information::is_ready_go() == true {
                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                            }
                        }
                    }
                }

                if DEAD == true {
                    if FULLY_DEAD == false {
                        if sv_information::is_ready_go() == true {
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                                if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                                    ModelModule::set_scale(module_accessor, 1.0);
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                    FULLY_DEAD = true;
                                }
                            }
                        }
                    }
                }

                // DISABLES AI

                if DEAD == false {
                    if sv_information::is_ready_go() == true {
                        if JUMP_START == false {
                            JUMP_START = true;
                            CONTROLLABLE = false;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME, true);
                        }

                        if DEAD == false {
                            if CONTROLLABLE == true {
                                if TELEPORTED == false {
                                    if MotionModule::frame(boss_boma) >= 30.0 {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                                    }
                                }
                            }
                            if TELEPORTED == true {
                                if MotionModule::frame(boss_boma) <= 140.0 {
                                    TELEPORTED = false;
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.2, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.2, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
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
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                            CONTROLLABLE = false;
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
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                            CONTROLLABLE = false;
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOMING {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START {
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

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START {
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
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL {
                            CONTROLLABLE = false;
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL {
                            CONTROLLABLE = false;
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.2, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.2, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END {

                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_END {
                            CONTROLLABLE = false;
                        }
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) <= 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) <= 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) <= 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_RND_WAIT {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) <= 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) <= 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TO_POINT {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) <= 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) <= 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOMING {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_HOMING {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == true {
                                MULTIPLE_BULLETS = 2;
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == false {
                                MULTIPLE_BULLETS = 0;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                            }
                        }

                        if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU {
                            if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                                if MULTIPLE_BULLETS != 0 {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                                    MULTIPLE_BULLETS = MULTIPLE_BULLETS - 1;
                                }
                            }
                        }
                        
                        //if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                        //    if MULTIPLE_BULLETS != 0 {
                        //        MotionModule::set_rate(boss_boma, 1.0);
                        //        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        //    }
                        //    if MULTIPLE_BULLETS == 0 {
                        //        MotionModule::set_rate(boss_boma, 1.0);
                        //        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        //    }
                        //}
                        //if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU {
                        //    if MULTIPLE_BULLETS != 0 {
                        //        MotionModule::set_rate(boss_boma, 5.0);
                        //        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 5.0);
                        //    }
                        //    if MULTIPLE_BULLETS == 0 {
                        //        MotionModule::set_rate(boss_boma, 1.0);
                        //        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        //    }
                        //}

                        if CONTROLLABLE == true {
                            MULTIPLE_BULLETS = 0;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_HOMING {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_HOMING {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_RUSH_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_THROW_END_1 {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_MISS_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_END {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN {
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
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT {
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
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
                            if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_STANDBY {
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f{x: x, y: y , z: z};
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
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                        }
                    }
                }
                // IS CPU?
                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                    // CONTROLS
                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_STANDBY {
                                if CONTROLLABLE == true {
                                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                    // Boss Control Stick Movement
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
                                    // FACING LEFT OR RIGHT?
                                    if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                                        if PostureModule::lr(boss_boma) == 1.0 { // RIGHT
                                            if ControlModule::get_stick_x(module_accessor) < -0.9 {
                                                if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                                                    CONTROLLABLE = false;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if PostureModule::lr(boss_boma) == -1.0 { // LEFT
                                            if ControlModule::get_stick_x(module_accessor) > 0.9 {
                                                if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                                                    CONTROLLABLE = false;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                    }
                                    // Boss Moves
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                        CONTROLLABLE = false;
                                        TELEPORTED = true;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                        CONTROLLABLE = false;
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
                                        CONTROLLABLE = false;
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
                                        CONTROLLABLE = false;
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
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START, true);
                                    }
                                    if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                        CONTROLLABLE = false;
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
                                        CONTROLLABLE = false;
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
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_GOOPAA, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE, true);
                                    }
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                        CONTROLLABLE = false;
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
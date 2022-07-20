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
use smash::hash40;
use smash::app::utility::get_category;

static mut STOP_CONTROL_LOOP : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
static mut IS_BOSS_DEAD : bool = false;
pub static mut FIGHTER_MANAGER: usize = 0;
pub static mut FIGHTER_NAME: [u64;16] = [0;16];

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
        while get_category(owner_module_accessor) != *BATTLE_OBJECT_CATEGORY_FIGHTER {
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
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("WOL MASTER HAND") | hash40("") {
                if ModelModule::scale(module_accessor) != 0.0001 {
                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                    ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_PLAYABLE_MASTERHAND),0,0,false,false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                    ModelModule::set_scale(module_accessor,0.0001);
                    ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                }

                if sv_information::is_ready_go() == false {
                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                        ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                    }
                }

                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                DamageModule::set_damage_lock(boss_boma,true);
                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                if IS_BOSS_DEAD == false {
                    if sv_information::is_ready_go() == true {
                        if ModelModule::scale(module_accessor) == 0.0001 {
                            if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                            }
                        }
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

                if sv_information::is_ready_go() == true {
                    if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                        }
                    }
                }

                if StopModule::is_damage(boss_boma) | StopModule::is_damage(module_accessor) {
                    if DamageModule::damage(module_accessor, 0) >= 299.0 {
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

                if sv_information::is_ready_go() == false {
                    if IS_BOSS_DEAD == false {
                        StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT,true);
                    }
                }

                if ModelModule::scale(module_accessor) == 0.0001 {
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

                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                        if IS_BOSS_DEAD == false {
                            IS_BOSS_DEAD = true;
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DamageModule::damage(module_accessor, 0) == -1.0 {
                            if IS_BOSS_DEAD == false {
                                IS_BOSS_DEAD = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                            }
                        }
                    }

                    if IS_BOSS_DEAD == true {
                        if sv_information::is_ready_go() == true {
                            let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                                DamageModule::add_damage(module_accessor, 300.0, 0);
                                STOP_CONTROL_LOOP = false;
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                            if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                            }
                            if StatusModule::status_kind(module_accessor) != *ITEM_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY,true);
                            }
                        }
                    }


                    if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN {
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
                    if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                        STOP_CONTROL_LOOP = true;
                    }
                    if STOP_CONTROL_LOOP == true {
                        if IS_BOSS_DEAD == false {
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
                            //Boss Moves
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_IRON_BALL_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_CHAKRAM_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_DRILL_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
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
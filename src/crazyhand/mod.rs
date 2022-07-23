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
use skyline::nn::ro::LookupSymbol;
use smash::hash40;
use smash::app::utility::get_category;
use smash::phx::Hash40;

static mut TELEPORTED : bool = false;
static mut TURNING : bool = false;
static mut CONTROLLABLE : bool = true;
static mut SUMMONED : bool = false;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
pub static mut FIGHTER_NAME: [u64;9] = [0;9];

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
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("CRAZY HAND") {
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        ModelModule::set_scale(boss_boma, 0.08);
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
                        TELEPORTED = false;
                        TURNING = false;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 1.0001 {
                            RESULT_SPAWNED = false;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
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

                    if ModelModule::scale(module_accessor) == 0.0001 {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                    }

                    //DAMAGE MODULES

                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    DamageModule::set_damage_lock(boss_boma, true);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    if sv_information::is_ready_go() == true {
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
                            if StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                                if StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                                    if StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                        DamageModule::add_damage(module_accessor, 4.1, 0);
                                        if StopModule::is_stop(module_accessor) {
                                            StopModule::end_stop(module_accessor);
                                        }
                                        if StopModule::is_stop(boss_boma) {
                                            StopModule::end_stop(boss_boma);
                                        }
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                                DamageModule::add_damage(module_accessor, 0.1, 0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                                DamageModule::add_damage(module_accessor, 0.1, 0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                DamageModule::add_damage(module_accessor, 0.1, 0);
                            }
                        }
                    }

                    if DEAD == true {
                        if sv_information::is_ready_go() == true {
                            if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_STANDBY {
                                if MotionModule::frame(boss_boma) == 100.0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                                }
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
                                    if PostureModule::pos_y(boss_boma) >= 170.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: 170.0, z: z};
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
                                        if PostureModule::pos_y(boss_boma) >= 170.0 {
                                            let boss_y_pos_1 = Vector3f{x: x, y: 170.0, z: z};
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

                    let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if RESULT_SPAWNED == false {
                            RESULT_SPAWNED = true;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                            MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("entry"),0.0,1.0,false,0.0,false,false);
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
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD == false {
                            if CONTROLLABLE == true {
                                if TURNING == false {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.4, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.4, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.4, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.4, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START {
                                MotionModule::set_rate(boss_boma, 2.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.2);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                                MotionModule::set_rate(boss_boma, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
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
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START {
                                if MotionModule::frame(boss_boma) == 40.0 {
                                    ArticleModule::set_flag(boss_boma, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_PINKY, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_PINKY);
                                    WorkModule::set_flag(boss_boma, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_PINKY);
                                }
                                if MotionModule::frame(boss_boma) == 55.0 {
                                    ArticleModule::set_flag(boss_boma, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_THUMB, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_THUMB);
                                    WorkModule::set_flag(boss_boma, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_THUMB);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_LOOP {
                                if SUMMONED == false {
                                    if MotionModule::frame(boss_boma) == 25.0 {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_LOOP, true);
                                        SUMMONED = true;
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END {
                                SUMMONED = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                                MotionModule::set_rate(boss_boma, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
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

                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                                //Boss Control Stick Movement
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.55, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.55, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.55, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.55, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                //Boss Control Stick Movement
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START {
                                MotionModule::set_rate(boss_boma, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                                //Boss Control Stick Movement
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK {
                                MotionModule::set_rate(boss_boma, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.1, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.1, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.1, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.1, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_START {
                                MotionModule::set_rate(boss_boma, 1.175);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.175);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP {
                                MotionModule::set_rate(boss_boma, 1.7);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.7);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_END {
                                MotionModule::set_rate(boss_boma, 1.6);
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE {
                                MotionModule::set_rate(boss_boma, 4.75);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.75);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                CONTROLLABLE = true;
                                TURNING = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START {
                                MotionModule::set_rate(boss_boma, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.4);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD {
                                MotionModule::set_rate(boss_boma, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU {
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI {
                                MotionModule::set_rate(boss_boma, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.4);
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
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_HOMING {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma, 1.25);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.25);
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
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CANCEL, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CANCEL {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CENTER_MOVE {
                                MotionModule::set_rate(boss_boma, 4.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.4);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                                MotionModule::set_rate(boss_boma, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.4);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DIG_END, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_ATTACK {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    MotionModule::set_rate(boss_boma, 4.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.0);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                    MotionModule::set_rate(boss_boma, 2.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    MotionModule::set_rate(boss_boma, 3.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 3.0);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                    MotionModule::set_rate(boss_boma, 2.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_RND_WAIT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TO_POINT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT {
                                CONTROLLABLE = true;
                                TURNING = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                            CONTROLLABLE = true;
                        }
                        
                        if CONTROLLABLE == true {
                            if DEAD == false {
                                if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_RUN {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TURN {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_JUMP {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_JUMP_AIR {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_FALL {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_EXIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_PASS_FLOOR {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_START {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TERM {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_LANDING {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_INITIALIZE {

                                }
                                else {
                                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                    }
                                    else {
                                        if TELEPORTED == false {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                        }
                                    }
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO_PRE_MOVE {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                        }

                        if TURNING == true {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }

                        if TURNING == true {
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

                        if StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                            if CONTROLLABLE == false {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {

                            }
                            else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            }
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                            CONTROLLABLE = true;
                            if MotionModule::frame(boss_boma) >= 40.0 {
                                TURNING = false;
                            }
                        }
                        
                        if CONTROLLABLE == true {
                            if TURNING == true {

                            }
                            else {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LANDING {

                            }
                            else {
                                //StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if TELEPORTED == true {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if CONTROLLABLE == true {
                            TELEPORTED = false;
                        }
                        if CONTROLLABLE == false {
                            if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT {
                                if TELEPORTED == false {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                    //Boss Control Stick Movement
                                    if MotionModule::frame(boss_boma) == 10.0 {
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
                        }
                    }
                    if MotionModule::motion_kind(boss_boma) == smash::hash40("taggoopaa") {
                        CONTROLLABLE = false;
                        if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                            if MotionModule::frame(boss_boma) >= 120.0 {
                                if MotionModule::frame(boss_boma) <= 140.0 {
                                    let pos = Vector3f{x: -0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            }
                            if MotionModule::frame(boss_boma) >= 130.0 {
                                if MotionModule::frame(boss_boma) <= 140.0 {
                                    let pos = Vector3f{x: 10.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                    AttackModule::set_power_mul(boss_boma, 12.5);
                                }
                            }
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                            if MotionModule::frame(boss_boma) >= 120.0 {
                                if MotionModule::frame(boss_boma) <= 140.0 {
                                    let pos = Vector3f{x: 0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                }
                            }
                            if MotionModule::frame(boss_boma) >= 130.0 {
                                if MotionModule::frame(boss_boma) <= 140.0 {
                                    let pos = Vector3f{x: -10.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);
                                    AttackModule::set_power_mul(boss_boma, 12.5);
                                }
                            }
                        }
                    }

                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                        if TURNING == false {
                            if CONTROLLABLE == true {
                                if DEAD == false {
                                    //Boss Control Movement
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
                                    if TURNING == false {
                                        //Boss Moves
                                        if PostureModule::lr(boss_boma) == 1.0 { // right
                                            if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                                if TURNING == false {
                                                    TURNING = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if PostureModule::lr(boss_boma) == -1.0 { // left
                                            if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                                if TURNING == false {
                                                    TURNING = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                            CONTROLLABLE = false;
                                            MotionModule::change_motion(boss_boma,Hash40::new("taggoopaa"),0.0,1.0,false,0.0,false,false);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                            TELEPORTED = false;
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DIG_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                            CONTROLLABLE = false;
                                            if PostureModule::pos_y(boss_boma) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 40.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -40.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                                            }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                                        }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                                    }
                                            }
                                            else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                            CONTROLLABLE = false;
                                            if PostureModule::pos_y(boss_boma) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 25.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -25.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_READY, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                                        }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                                    }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                                }
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_KUMO, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                            CONTROLLABLE = false;
                                            if PostureModule::pos_y(boss_boma) <= 35.0 {
                                                if PostureModule::pos_y(boss_boma) >= 0.0 {
                                                    if PostureModule::pos_x(boss_boma) <= 75.0 {
                                                        if PostureModule::pos_x(boss_boma) >= -75.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                                            }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                                        }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                                    }
                                            }
                                            else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START, true);
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

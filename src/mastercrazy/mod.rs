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
use smash::app::lua_bind;

static mut TELEPORTED : bool = false;
static mut TURNING : bool = false;
static mut CONTROLLABLE : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_NAME: [u64;9] = [0;9];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut MULTIPLE_BULLETS : usize = 0;
static mut DEAD : bool = false;
static mut MASTERHAND_BROADCAST_1 : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
static mut STOP : bool = false;
static mut STUNNED : bool = false;
static mut MASTER_EXISTS : bool = false;
static mut RECOVERY : usize = 0;
static mut EXISTS_PUBLIC : bool = false;

static mut TELEPORTED_2 : bool = false;
static mut TURNING_2 : bool = false;
static mut CONTROLLABLE_2 : bool = true;
static mut SUMMONED_2 : bool = false;
static mut ENTRY_ID_2 : usize = 0;
static mut BOSS_ID_2 : [u32; 8] = [0; 8];
pub static mut FIGHTER_NAME_2: [u64;9] = [0;9];
pub static mut FIGHTER_MANAGER_2: usize = 0;
static mut DEAD_2 : bool = false;
static mut CRAZYHAND_BROADCAST_1 : bool = false;
static mut JUMP_START_2 : bool = false;
static mut RESULT_SPAWNED_2 : bool = false;
static mut STOP_2 : bool = false;
static mut STUNNED_2 : bool = false;
static mut CRAZY_EXISTS : bool = false;
static mut RECOVERY_2 : usize = 0;
static mut EXISTS_PUBLIC_2 : bool = false;

pub unsafe fn check_status() -> bool {
    return EXISTS_PUBLIC;
}

pub unsafe fn check_status_2() -> bool {
    return EXISTS_PUBLIC_2;
}

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
        while get_category(owner_module_accessor) != *BATTLE_OBJECT_CATEGORY_FIGHTER { //Keep checking the owner of the boma we're working with until we've hit a boma that belongs to a fighter
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
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MASTER HAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("マスターハンド")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("CRÉA-MAIN")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEISTERHAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("大师之手")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("大師之手")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("마스터 핸드")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("ГЛАВНАЯ РУКА")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MÃO MESTRA") {
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
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
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        DEAD = false;
                        CONTROLLABLE = true;
                        JUMP_START = false;
                        TELEPORTED = false;
                        TURNING = false;
                        STOP = false;
                        STUNNED = false;
                        MULTIPLE_BULLETS = 0;
                        RECOVERY = 0;
                        MASTERHAND_BROADCAST_1 = false;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            MASTER_EXISTS = true;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD == true {
                            HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                            if STOP == false {
                                if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                    MASTER_EXISTS = false;
                                }
                                if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                    STOP = true;
                                    MASTER_EXISTS = false;
                                }
                            }
                            if STOP == true {
                                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                                    MASTER_EXISTS = false;
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if MASTERHAND_BROADCAST_1 == true {
                            MASTERHAND_BROADCAST_1 = false;
                            CONTROLLABLE = false;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_GOOPAA, true);
                        }
                    }

                    if DEAD == true {
                        if sv_information::is_ready_go() == true {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_STANDBY {
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -180.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if MotionModule::frame(boss_boma) == 100.0 {
                                        EXISTS_PUBLIC = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                                    }
                                }
                            }
                        }
                    }

                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if RESULT_SPAWNED == false {
                            EXISTS_PUBLIC = false;
                            RESULT_SPAWNED = true;
                            MASTER_EXISTS = false;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START,true);
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

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            // SET POS AND STOPS OUT OF BOUNDS
                            if ModelModule::scale(module_accessor) == 0.0001 {
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                    FighterManager::set_cursor_whole(fighter_manager,false);
                                    fighter.set_situation(SITUATION_KIND_AIR.into());
                                    MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                                }
                                if FighterUtil::is_hp_mode(module_accessor) == true {
                                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                                        if DEAD == false {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                            DEAD = true;
                                        }
                                    }
                                }
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f{x: x, y: y + 20.0, z: z};
                                if PostureModule::pos_y(boss_boma) <= -100.0 {
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
                                    if PostureModule::pos_y(boss_boma) >= 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: 200.0, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= -50.0 {
                                        let boss_y_pos_2 = Vector3f{x: 200.0, y: -50.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma) <= -200.0 {
                                    let boss_x_pos_2 = Vector3f{x: -200.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma) >= 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: -200.0, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma) <= -50.0 {
                                        let boss_y_pos_2 = Vector3f{x: -200.0, y: -50.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma) >= 200.0 {
                                        let boss_x_pos_1 = Vector3f{x: 200.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                    }
                                }
                                else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                        }
                    }

                    //DAMAGE MODULES
                    
                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    DamageModule::set_damage_lock(boss_boma, true);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);
                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    if sv_information::is_ready_go() == true {
                        if DamageModule::damage(module_accessor, 0) >= 200.0 {
                            if STUNNED == false {
                                CONTROLLABLE = false;
                                STUNNED = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_DOWN_START,true);
                            }
                        }
                        if StopModule::is_damage(boss_boma) | StopModule::is_damage(module_accessor) {
                            if FighterUtil::is_hp_mode(module_accessor) == false {
                                if DamageModule::damage(module_accessor, 0) >= 399.0 {
                                    if DEAD == false {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                        DEAD = true;
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL {
                                if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START {
                                    if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_END {
                                        if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL {
                                            if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START {
                                                if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_END {
                                                    DamageModule::add_damage(module_accessor, 4.1, 0);
                                                    if StopModule::is_stop(module_accessor) {
                                                        StopModule::end_stop(module_accessor);
                                                    }
                                                    if StopModule::is_stop(boss_boma) {
                                                        StopModule::end_stop(boss_boma);
                                                    }
                                                    else {
                                                        DamageModule::add_damage(module_accessor, 0.1, 0);
                                                    }
                                                }
                                                else {
                                                    DamageModule::add_damage(module_accessor, 0.1, 0);
                                                }
                                            }
                                            else {
                                                DamageModule::add_damage(module_accessor, 0.1, 0);
                                            }
                                        }
                                        else {
                                            DamageModule::add_damage(module_accessor, 0.1, 0);
                                        }
                                    }
                                    else {
                                        DamageModule::add_damage(module_accessor, 0.1, 0);
                                    }
                                }
                                else {
                                    DamageModule::add_damage(module_accessor, 0.1, 0);
                                }
                            }
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
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if CONTROLLABLE == true {
                            if TURNING == false {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_FIRING {
                            MotionModule::set_rate(boss_boma, 2.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_HOLD {
                            MotionModule::set_rate(boss_boma, 2.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_SHOOT {
                            MotionModule::set_rate(boss_boma, 2.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END {
                            MotionModule::set_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                            CONTROLLABLE = false;
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                            CONTROLLABLE = false;
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START {
                            MotionModule::set_rate(boss_boma, 1.5);
                            //BOSS POSITION
                            //Boss Control Stick Movement
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
                            MotionModule::set_rate(boss_boma, 2.0);
                            //BOSS POSITION
                            //Boss Control Stick Movement
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
                            MotionModule::set_rate(boss_boma, 2.0);
                            //BOSS POSITION
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                            MotionModule::set_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU {
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
                            MotionModule::set_rate(boss_boma, 1.0);
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.1);
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START {

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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL {
                            CONTROLLABLE = false;
                            //BOSS POSITION
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_START {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_START {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.3);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                            //Boss Control Stick Movement
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
                            MotionModule::set_rate(boss_boma, 1.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.5);
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
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CANCEL {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    MotionModule::set_rate(boss_boma, 2.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA {
                                if MotionModule::frame(boss_boma) != MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = false;
                                }
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END_START {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_RUSH_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_START_DOWN {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_THROW_END_1 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 2.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_MISS_END {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 2.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_END {
                                if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT {
                                CONTROLLABLE = true;
                                if MotionModule::frame(boss_boma) >= 40.0 {
                                    TURNING = false;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_END {
                                CONTROLLABLE = false;
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma)  {
                                if MotionModule::motion_kind(boss_boma) != smash::hash40("electroshock_end") {
                                    CONTROLLABLE = true;
                                    TURNING = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                                }
                            }
                            if MotionModule::motion_kind(boss_boma) == smash::hash40("electroshock_start") {
                                CONTROLLABLE = false;
                            }
        
                            if MotionModule::motion_kind(boss_boma) == smash::hash40("electroshock") {
                                CONTROLLABLE = false;
                            }
                            if MotionModule::motion_kind(boss_boma) == smash::hash40("electroshock_end") {
                                CONTROLLABLE_2 = false;
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK,true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_RND_WAIT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TO_POINT {
                                CONTROLLABLE = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT {
                                CONTROLLABLE = true;
                                TURNING = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START {
                                RECOVERY = 0;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP {
                                if RECOVERY >= 135 {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DOWN_END, true);
                                }
                                else {
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) || ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 || 
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) || ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                        RECOVERY += 1;
                                    }
                                }
                            }
                        }

                        // TESTING DISTANCE

                        // let x = PostureModule::pos_x(module_accessor);
                        // let y = PostureModule::pos_y(module_accessor);
                        // let z = PostureModule::pos_z(module_accessor);
                        // let module_accessor_pos = Vector3f{x: x, y: y, z: z};

                        // println!("[Comp. Playable Bosses] Distance: {:?}", GroundModule::get_distance_to_floor(module_accessor, &module_accessor_pos, 1.0, true));
                        // println!("[Comp. Playable Bosses] Border: {:?}", CameraModule::get_main_camera_range(module_accessor));

                        if TURNING == true {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOMING {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_ATTACK {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                MotionModule::set_rate(boss_boma, 4.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.0);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_START {
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                MotionModule::set_rate(boss_boma, 3.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 3.0);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                MotionModule::set_rate(boss_boma, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
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
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == true {
                                    MULTIPLE_BULLETS = 2;
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == false {
                                    MULTIPLE_BULLETS = 0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                                }
                            }
                            else {
                                MULTIPLE_BULLETS = 2;
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

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                            if MULTIPLE_BULLETS != 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if MULTIPLE_BULLETS == 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU {
                            if MULTIPLE_BULLETS != 0 {
                                MotionModule::set_rate(boss_boma, 5.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 5.0);
                            }
                            if MULTIPLE_BULLETS == 0 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                        }

                        if CONTROLLABLE == true {
                            MULTIPLE_BULLETS = 0;
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

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 2.0);
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
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 2.0);
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
                            MotionModule::set_rate(boss_boma, 1.25);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.25);
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
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CANCEL, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN {
                            MotionModule::set_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CENTER_MOVE {
                            MotionModule::set_rate(boss_boma, 4.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START {
                            MotionModule::set_rate(boss_boma, 1.25);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.25);
                        }
                        if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT {
                            if CONTROLLABLE == false {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN {

                            }
                            else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CAPTURE {
                            MotionModule::set_rate(boss_boma, 4.75);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.75);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
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
                                else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_JUMP_AIR {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_FALL {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_EXIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_PASS_FLOOR {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT {

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
                        if CONTROLLABLE == true {
                            if TURNING == true {

                            }
                            else {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_END {

                            }
                            else if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LANDING {

                            }
                            else {

                            }
                        }
                        if TELEPORTED == true {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if CONTROLLABLE == false {
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TELEPORT {
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
                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                        if TURNING == false {
                            if CONTROLLABLE == true {
                                if DEAD == false {
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
                                    if TURNING == false {
                                        //Boss Moves
                                        if PostureModule::lr(boss_boma) == 1.0 { // right
                                            if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                                if TURNING == false {
                                                    TURNING = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if PostureModule::lr(boss_boma) == -1.0 { // left
                                            if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                                if TURNING == false {
                                                    TURNING = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                            if CRAZY_EXISTS == true {
                                                CONTROLLABLE = false;
                                                CONTROLLABLE_2 = false;
                                                CRAZYHAND_BROADCAST_1 = true;
                                                MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("electroshock_start"),0.0,1.0,false,0.0,false,false);
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                                        }
                                        if CONTROLLABLE == true {
                                            TELEPORTED = false;
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                            TELEPORTED = false;
                                            CONTROLLABLE = false;
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
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
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
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START, true);
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
pub fn once_per_fighter_frame_2(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            pub unsafe fn entry_id(module_accessor: &mut BattleObjectModuleAccessor) -> usize {
                let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                return entry_id;
            }
            ENTRY_ID_2 = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
            LookupSymbol(
                &mut FIGHTER_MANAGER_2,
                "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
                .as_bytes()
                .as_ptr(),
            );
            let fighter_manager = *(FIGHTER_MANAGER_2 as *mut *mut smash::app::FighterManager);
            let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
            let name_base = text + 0x52c3758;
            FIGHTER_NAME_2[get_player_number(&mut *fighter.module_accessor)] = hash40(&read_tag(name_base + 0x260 * get_player_number(&mut *fighter.module_accessor) as u64 + 0x8e));
            if FIGHTER_NAME_2[get_player_number(module_accessor)] == hash40("CRAZY HAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("クレイジーハンド")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("DÉ-MAINIAQUE")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("疯狂之手")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("瘋狂之手")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("크레이지 핸드")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("БЕЗУМНАЯ РУКА")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MÃO MANÍACA") {
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                        BOSS_ID_2[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                        ModelModule::set_scale(boss_boma_2, 0.08);
                        MotionModule::change_motion(boss_boma_2,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        ModelModule::set_joint_rotate(module_accessor, smash::phx::Hash40::new("root") , &mut Vector3f{x: -270.0, y: 180.0, z: -90.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                    }
                }
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        DEAD_2 = false;
                        CONTROLLABLE_2 = true;
                        JUMP_START_2 = false;
                        TELEPORTED_2 = false;
                        TURNING_2 = false;
                        STOP_2 = false;
                        STUNNED_2 = false;
                        RECOVERY_2 = 0;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID_2 = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            EXISTS_PUBLIC_2 = true;
                            RESULT_SPAWNED_2 = false;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                            BOSS_ID_2[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD_2 == true {
                            HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            HitModule::set_whole(boss_boma_2, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                            if STOP_2 == false {
                                if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) != 0 {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                }
                                if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == 0 {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                    STOP_2 = true;
                                }
                            }
                            if STOP_2 == true {
                                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                                }
                            }
                        }
                    }

                    if DEAD_2 == true {
                        if sv_information::is_ready_go() == true {
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma_2) != *ITEM_STATUS_KIND_STANDBY {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -180.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma_2,&vec3,0);
                                    }
                                    if MotionModule::frame(boss_boma_2) == 100.0 {
                                        EXISTS_PUBLIC = false;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_STANDBY, true);
                                    }
                                }
                            }
                        }
                    }

                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if RESULT_SPAWNED_2 == false {
                            EXISTS_PUBLIC_2 = false;
                            RESULT_SPAWNED_2 = true;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                            BOSS_ID_2[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                            MotionModule::change_motion(boss_boma_2,smash::phx::Hash40::new("entry"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 {
                        let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                        }
                    }

                    //DAMAGE MODULES

                    let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                    DamageModule::set_damage_lock(boss_boma_2, true);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);
                    HitModule::set_whole(boss_boma_2, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    if sv_information::is_ready_go() == true {
                        if DamageModule::damage(module_accessor, 0) >= 200.0 {
                            if STUNNED_2 == false {
                                CONTROLLABLE_2 = false;
                                STUNNED_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2,*ITEM_CRAZYHAND_STATUS_KIND_DOWN_START,true);
                            }
                        }
                        if StopModule::is_damage(boss_boma_2) | StopModule::is_damage(module_accessor) {
                            if FighterUtil::is_hp_mode(module_accessor) == false {
                                if DamageModule::damage(module_accessor, 0) >= 399.0 {
                                    if DEAD_2 == false {
                                        CONTROLLABLE_2 = false;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_DEAD, true);
                                        DEAD_2 = true;
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                                if StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                                    if StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                        DamageModule::add_damage(module_accessor, 4.1, 0);
                                        if StopModule::is_stop(module_accessor) {
                                            StopModule::end_stop(module_accessor);
                                        }
                                        if StopModule::is_stop(boss_boma_2) {
                                            StopModule::end_stop(boss_boma_2);
                                        }
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                                DamageModule::add_damage(module_accessor, 0.1, 0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                                DamageModule::add_damage(module_accessor, 0.1, 0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                DamageModule::add_damage(module_accessor, 0.1, 0);
                            }
                        }
                    }

                    if DEAD_2 == false {
                        if sv_information::is_ready_go() == true {
                            // SET POS AND STOPS OUT OF BOUNDS
                            if ModelModule::scale(module_accessor) == 0.0001 {
                                let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                                if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                    FighterManager::set_cursor_whole(fighter_manager,false);
                                    fighter.set_situation(SITUATION_KIND_AIR.into());
                                    MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                                }
                                if FighterUtil::is_hp_mode(module_accessor) == true {
                                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                                        if DEAD_2 == false {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_DEAD, true);
                                            DEAD_2 = true;
                                        }
                                    }
                                }
                                let x = PostureModule::pos_x(boss_boma_2);
                                let y = PostureModule::pos_y(boss_boma_2);
                                let z = PostureModule::pos_z(boss_boma_2);
                                let boss_pos = Vector3f{x: x, y: y + 20.0, z: z};
                                if PostureModule::pos_y(boss_boma_2) <= -100.0 {
                                    let boss_y_pos_2 = Vector3f{x: x, y: -100.0, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                    PostureModule::set_pos(boss_boma_2, &boss_y_pos_2);
                                    if PostureModule::pos_x(boss_boma_2) >= 200.0 {
                                        let boss_x_pos_1 = Vector3f{x: 200.0, y: -100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma_2, &boss_x_pos_1);
                                    }
                                    if PostureModule::pos_x(boss_boma_2) <= -200.0 {
                                        let boss_x_pos_2 = Vector3f{x: -200.0, y: -100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma_2, &boss_x_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma_2) >= 200.0 {
                                    let boss_x_pos_1 = Vector3f{x: 200.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                    PostureModule::set_pos(boss_boma_2, &boss_x_pos_1);
                                    if PostureModule::pos_x(boss_boma_2) <= -200.0 {
                                        let boss_x_pos_2 = Vector3f{x: -200.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma_2, &boss_x_pos_2);
                                    }
                                    if PostureModule::pos_y(boss_boma_2) >= 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: 200.0, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma_2, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma_2) <= -50.0 {
                                        let boss_y_pos_2 = Vector3f{x: 200.0, y: -50.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma_2, &boss_y_pos_2);
                                    }
                                }
                                else if PostureModule::pos_x(boss_boma_2) <= -200.0 {
                                    let boss_x_pos_2 = Vector3f{x: -200.0, y: y, z: z};
                                    PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                    PostureModule::set_pos(boss_boma_2, &boss_x_pos_2);
                                    if PostureModule::pos_y(boss_boma_2) >= 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: -200.0, y: 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma_2, &boss_y_pos_1);
                                    }
                                    if PostureModule::pos_y(boss_boma_2) <= -50.0 {
                                        let boss_y_pos_2 = Vector3f{x: -200.0, y: -50.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma_2, &boss_y_pos_2);
                                    }
                                    if PostureModule::pos_x(boss_boma_2) >= 200.0 {
                                        let boss_x_pos_1 = Vector3f{x: 200.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma_2, &boss_x_pos_1);
                                    }
                                }
                                else {
                                    PostureModule::set_pos(module_accessor, &boss_pos);
                                }
                            }
                        }
                    }

                    // SETS POWER

                    AttackModule::set_power_mul(boss_boma_2, 1.5);

                    // FIXES SPAWN

                    if DEAD_2 == false {
                        if sv_information::is_ready_go() == true {
                            if JUMP_START_2 == false {
                                JUMP_START_2 = true;
                                CONTROLLABLE_2 = false;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if DEAD_2 == false {
                            if CONTROLLABLE_2 == true {
                                if TURNING_2 == false {
                                    MotionModule::set_rate(boss_boma_2, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.4, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.4, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.4, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.4, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START {
                                MotionModule::set_rate(boss_boma_2, 2.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.2);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START {
                                if MotionModule::frame(boss_boma_2) == 40.0 {
                                    WorkModule::set_flag(boss_boma_2, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_PINKY);
                                }
                                if MotionModule::frame(boss_boma_2) == 55.0 {
                                    WorkModule::set_flag(boss_boma_2, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_THUMB);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_LOOP {
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_LOOP {
                                if SUMMONED_2 == false {
                                    if MotionModule::frame(boss_boma_2) == 25.0 {
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_LOOP, true);
                                        SUMMONED_2 = true;
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END {
                                SUMMONED_2 = false;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }

                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.75, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.75, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.55, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.55, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.55, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.55, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.2, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.2, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.1, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.1, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.1, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.1, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {
                                CONTROLLABLE_2 = false;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {
                                CONTROLLABLE_2 = false;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL {
                                CONTROLLABLE_2 = false;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_START {
                                MotionModule::set_rate(boss_boma_2, 1.175);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.175);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP {
                                MotionModule::set_rate(boss_boma_2, 1.7);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.7);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                MotionModule::set_rate(boss_boma_2, 1.5);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.5);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2 {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3 {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE {
                                MotionModule::set_rate(boss_boma_2, 4.75);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 4.75);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START {
                                MotionModule::set_rate(boss_boma_2, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD {
                                MotionModule::set_rate(boss_boma_2, 1.2);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI {
                                MotionModule::set_rate(boss_boma_2, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_HOMING {
                                //Boss Control Stick Movement
                                MotionModule::set_rate(boss_boma_2, 1.25);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.25);
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }

                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                    let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CANCEL, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CENTER_MOVE {
                                MotionModule::set_rate(boss_boma_2, 4.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 4.4);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU {
                                MotionModule::set_rate(boss_boma_2, 2.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                                MotionModule::set_rate(boss_boma_2, 1.4);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3 {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2 {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_LOOP {
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DIG_END, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_ATTACK {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    MotionModule::set_rate(boss_boma_2, 4.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 4.0);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                    MotionModule::set_rate(boss_boma_2, 2.2);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.2);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START {
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    MotionModule::set_rate(boss_boma_2, 3.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 3.0);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                                    MotionModule::set_rate(boss_boma_2, 2.2);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.2);
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                                MotionModule::set_rate(boss_boma_2, 1.5);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.5);
                            }
                            if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == false {
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END {
                                CONTROLLABLE_2 = false;
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CANCEL {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) {
                                CONTROLLABLE_2 = true;
                                TURNING_2 = false;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_RND_WAIT {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TO_POINT {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT {
                                CONTROLLABLE_2 = true;
                                TURNING_2 = false;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2 {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3 {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_END {
                                MotionModule::set_rate(boss_boma_2, 1.6);
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_END {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END {
                                if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {
                                RECOVERY_2 = 0;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {
                                if RECOVERY_2 >= 135 {
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END, true);
                                }
                                else {
                                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) || ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 || 
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 ||
                                     ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) || ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) ||
                                     ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                        RECOVERY_2 += 1;
                                    }
                                }
                            }
                        }
                        
                        if CONTROLLABLE_2 == true {
                            if DEAD_2 == false {
                                if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_RUN {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_TURN {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_NONE {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_JUMP {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_JUMP_AIR {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_FALL {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_EXIT {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_PASS_FLOOR {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT {

                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_START {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_TERM {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_LANDING {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_INITIALIZE {

                                }
                                else {
                                    if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY {

                                    }
                                    else {
                                        if TELEPORTED_2 == false {
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_WAIT, true);
                                        }
                                    }
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO_PRE_MOVE {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                        }

                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                        }

                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                        }

                        if TURNING_2 == true {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                        }

                        if TURNING_2 == true {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                        }

                        if StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                            if CONTROLLABLE_2 == false {

                            }
                            else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {

                            }
                            else {
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            }
                        }

                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                            CONTROLLABLE_2 = true;
                            if MotionModule::frame(boss_boma_2) >= 40.0 {
                                TURNING_2 = false;
                            }
                        }
                        
                        if CONTROLLABLE_2 == true {
                            if TURNING_2 == true {

                            }
                            else {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_DEAD {

                            }
                            else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {

                            }
                            else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL {

                            }
                            else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {

                            }
                            else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END {

                            }
                            else if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LANDING {

                            }
                            else {
                                //StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                        }
                        if TELEPORTED_2 == true {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if CONTROLLABLE_2 == true {
                            TELEPORTED_2 = false;
                        }
                        if CONTROLLABLE_2 == false {
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT {
                                if TELEPORTED_2 == false {
                                    MotionModule::set_rate(boss_boma_2, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                    //Boss Control Stick Movement
                                    if MotionModule::frame(boss_boma_2) == 10.0 {
                                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                            let pos = Vector3f{x: -140.0, y: 0.0, z: 0.0};
                                            PostureModule::add_pos(boss_boma_2, &pos);
                                            TELEPORTED_2 = true;
                                        }
                                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                            let pos = Vector3f{x: 140.0, y: 0.0, z: 0.0};
                                            PostureModule::add_pos(boss_boma_2, &pos);
                                            TELEPORTED_2 = true;
                                        }
                                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                            let pos = Vector3f{x: 0.0, y: -50.0, z: 0.0};
                                            PostureModule::add_pos(boss_boma_2, &pos);
                                            TELEPORTED_2 = true;
                                        }
                                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                            let pos = Vector3f{x: 0.0, y: 50.0, z: 0.0};
                                            PostureModule::add_pos(boss_boma_2, &pos);
                                            TELEPORTED_2 = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if MotionModule::motion_kind(boss_boma_2) == smash::hash40("taggoopaa") {
                        CONTROLLABLE_2 = false;
                        if MotionModule::frame(boss_boma_2) <= 100.0 {
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.5, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }

                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.5, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            if MotionModule::frame(boss_boma_2) >= 120.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: -0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                    AttackModule::set_power_mul(boss_boma_2, 1.75);
                                }
                            }
                            if MotionModule::frame(boss_boma_2) >= 130.0 {
                                AttackModule::set_power_mul(boss_boma_2, 2.0);
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: 10.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                    AttackModule::set_power_mul(boss_boma_2, 2.375);
                                }
                            }
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            if MotionModule::frame(boss_boma_2) >= 120.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: 0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                    AttackModule::set_power_mul(boss_boma_2, 1.75);
                                }
                            }
                            if MotionModule::frame(boss_boma_2) >= 130.0 {
                                AttackModule::set_power_mul(boss_boma_2, 2.0);
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: -10.0, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                    AttackModule::set_power_mul(boss_boma_2, 2.375);
                                }
                            }
                        }
                    }

                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == false {
                        if TURNING_2 == false {
                            if CONTROLLABLE_2 == true {
                                if DEAD == false {
                                    //Boss Control Movement
                                    if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                        let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * - 2.0 * ControlModule::get_stick_x(module_accessor), y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                
                                    if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                        let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0 * ControlModule::get_stick_x(module_accessor), y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                
                                    if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                        let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * - 2.0 * ControlModule::get_stick_y(module_accessor), z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                
                                    if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                        let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0 * ControlModule::get_stick_y(module_accessor), z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                    if TURNING_2 == false {
                                        //Boss Moves
                                        if PostureModule::lr(boss_boma_2) == 1.0 { // right
                                            if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                                if TURNING_2 == false {
                                                    TURNING_2 = true;
                                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if PostureModule::lr(boss_boma_2) == -1.0 { // left
                                            if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                                if TURNING_2 == false {
                                                    TURNING_2 = true;
                                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_TURN, true);
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                            CONTROLLABLE_2 = false;
                                            MotionModule::change_motion(boss_boma_2,smash::phx::Hash40::new("taggoopaa"),0.0,1.0,false,0.0,false,false);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                            TELEPORTED_2 = false;
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DIG_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                            CONTROLLABLE_2 = false;
                                            if PostureModule::pos_y(boss_boma_2) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma_2) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma_2) <= 40.0 {
                                                        if PostureModule::pos_x(boss_boma_2) >= -40.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                                            }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                                        }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                                    }
                                            }
                                            else {
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                            CONTROLLABLE_2 = false;
                                            if PostureModule::pos_y(boss_boma_2) <= 25.0 {
                                                if PostureModule::pos_y(boss_boma_2) >= -25.0 {
                                                    if PostureModule::pos_x(boss_boma_2) <= 25.0 {
                                                        if PostureModule::pos_x(boss_boma_2) >= -25.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_READY, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                                        }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                                    }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                                }
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_KUMO, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                            CONTROLLABLE_2 = false;
                                            if PostureModule::pos_y(boss_boma_2) <= 35.0 {
                                                if PostureModule::pos_y(boss_boma_2) >= 0.0 {
                                                    if PostureModule::pos_x(boss_boma_2) <= 75.0 {
                                                        if PostureModule::pos_x(boss_boma_2) >= -75.0 {
                                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU, true);
                                                        }
                                                        else {
                                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                                            }
                                                    }
                                                    else {
                                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                                        }
                                                }
                                                else {
                                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                                    }
                                            }
                                            else {
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START, true);
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
    acmd::add_custom_hooks!(once_per_fighter_frame_2);
}
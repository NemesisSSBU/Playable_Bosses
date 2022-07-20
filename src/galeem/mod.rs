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
use smash::app::lua_bind;
use skyline::nn::ro::LookupSymbol;
use smash::hash40;
use smash::app::utility::get_category;

static mut CONTROLLABLE : bool = true;
static mut IS_ANGRY : bool = false;
static mut ENTRY_ID : usize = 0;
static mut RANDOM_ATTACK : i32 = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
pub static mut FIGHTER_NAME: [u64;6] = [0;6];

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
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("GALEEM") {
                if sv_information::is_ready_go() == false {
                    DEAD = false;
                    CONTROLLABLE = true;
                    JUMP_START = false;
                    IS_ANGRY = false;
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        RESULT_SPAWNED = false;
                        if MotionModule::frame(module_accessor) >= 1.0 {
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_KIILA), 0, 0, false, false);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
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
                        MotionModule::set_rate(boss_boma, 4.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.0);
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
                
                if FighterManager::is_result_mode(fighter_manager) == true {
                    if RESULT_SPAWNED == false {
                        RESULT_SPAWNED = true;
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_KIILA), 0, 0, false, false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START,true);
                    }
                }

                if sv_information::is_ready_go() == true {
                    //DAMAGE MODULES
                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    DamageModule::set_damage_lock(boss_boma, true);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);
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

                    // SETS POWER

                    AttackModule::set_power_mul(boss_boma, 1.5);

                    // FIXES SPAWN

                    if DEAD == false {
                        if JUMP_START == false {
                            JUMP_START = true;
                            CONTROLLABLE = false;
                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                PostureModule::set_rot(boss_boma,&vec3,0);
                            }
                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                PostureModule::set_rot(boss_boma,&vec3,0);
                            }
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT, true);
                        }
                    }
                    if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_TELEPORT {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_START {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER_WAIT {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TERM {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_EXIT {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_LOOP {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_END {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN_LOOP {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CROSS_BOMB {
                        CONTROLLABLE = true;
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            CONTROLLABLE = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_TORRENT {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_END {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_END {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY {
                        CONTROLLABLE = false;
                        if MotionModule::frame(fighter.module_accessor) >= 100.0 {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TELEPORT, true);
                        }
                    }
                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                        if DEAD == false {
                            if CONTROLLABLE == true {
                                if MotionModule::frame(fighter.module_accessor) >= smash::app::sv_math::rand(hash40("fighter"), 200) as f32 {
                                    RANDOM_ATTACK = smash::app::sv_math::rand(hash40("fighter"), 12);
                                    // BUILT IN BOSS AI
                                    if RANDOM_ATTACK == 0 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CROSS_BOMB, true);
                                    }
                                    if RANDOM_ATTACK == 1 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TELEPORT, true);
                                    }
                                    if RANDOM_ATTACK == 2 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START, true);
                                    }
                                    if RANDOM_ATTACK == 3 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR, true);
                                    }
                                    if RANDOM_ATTACK == 4 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_START, true);
                                    }
                                    if RANDOM_ATTACK == 5 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START, true);
                                    }
                                    if RANDOM_ATTACK == 6 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_START, true);
                                    }
                                    if RANDOM_ATTACK == 7 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_THREAT_START, true);
                                    }
                                    if RANDOM_ATTACK == 8 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TORRENT, true);
                                    }
                                    if RANDOM_ATTACK == 9 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN_START, true);
                                    }
                                    if RANDOM_ATTACK == 10 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER, true);
                                    }
                                    if RANDOM_ATTACK == 11 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH, true);
                                    }
                                    if RANDOM_ATTACK == 12 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH, true);
                                    }
                                }
                            }
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START {
                        CONTROLLABLE = false;
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
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_END {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR {
                        CONTROLLABLE = false;
                        MotionModule::set_rate(boss_boma, 2.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
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
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER {
                        CONTROLLABLE = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY {
                        CONTROLLABLE = false;
                        if MotionModule::frame(fighter.module_accessor) >= 100.0 {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TELEPORT, true);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {
                        CONTROLLABLE = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_END {
                        CONTROLLABLE = true;
                    }
                    if CONTROLLABLE == true {
                        if DEAD == false {
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
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CROSS_BOMB, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TELEPORT, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_THREAT_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_TORRENT, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH, true);
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
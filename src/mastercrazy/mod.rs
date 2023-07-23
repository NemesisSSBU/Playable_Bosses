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
use smash::phx::Hash40;

// Global
static mut BARK : bool = false;
static mut PUNCH : bool = false;
static mut SHOCK : bool = false;
static mut LASER : bool = false;
static mut SCRATCH_BLOW : bool = false;
static mut CONTROL_SPEED_MUL: f32 = 2.0;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;

static mut MASTER_X_POS: f32 = 0.0;
static mut MASTER_Y_POS: f32 = 0.0;
static mut MASTER_Z_POS: f32 = 0.0;
static mut MASTER_USABLE : bool = false;
static mut MASTER_FACING_LEFT : bool = true;
static mut CONTROLLER_X_MASTER: f32 = 0.0;
static mut CONTROLLER_Y_MASTER: f32 = 0.0;

static mut CRAZY_X_POS: f32 = 0.0;
static mut CRAZY_Y_POS: f32 = 0.0;
static mut CRAZY_Z_POS: f32 = 0.0;
static mut CRAZY_USABLE : bool = false;
static mut CRAZY_FACING_RIGHT : bool = true;
static mut CONTROLLER_X_CRAZY: f32 = 0.0;
static mut CONTROLLER_Y_CRAZY: f32 = 0.0;

// Master Hand
static mut TURNING : bool = false;
static mut CONTROLLABLE : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_NAME: [u64;9] = [0;9];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut MULTIPLE_BULLETS : usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
static mut STOP : bool = false;
static mut MASTER_EXISTS : bool = false;
static mut EXISTS_PUBLIC : bool = false;
static mut Y_POS: f32 = 0.0;
static mut MASTER_AI_LEVEL: f32 = 0.0;
static mut MASTER_TEAM : u64 = 0;

// Crazy Hand
static mut TURNING_2 : bool = false;
static mut CONTROLLABLE_2 : bool = true;
static mut ENTRY_ID_2 : usize = 0;
static mut BOSS_ID_2 : [u32; 8] = [0; 8];
pub static mut FIGHTER_NAME_2: [u64;9] = [0;9];
pub static mut FIGHTER_MANAGER_2: usize = 0;
static mut DEAD_2 : bool = false;
static mut JUMP_START_2 : bool = false;
static mut RESULT_SPAWNED_2 : bool = false;
static mut STOP_2 : bool = false;
static mut CRAZY_EXISTS : bool = false;
static mut EXISTS_PUBLIC_2 : bool = false;
static mut Y_POS_2: f32 = 0.0;
static mut CRAZY_AI_LEVEL: f32 = 0.0;
static mut CRAZY_TEAM : u64 = 0;

extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
}

extern "C" {
    #[link_name = "\u{1}_ZN3app2ai4rankEP9lua_State"]
    pub fn cpu_level(lua_state: u64) -> f32;
}

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
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEISTER HAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("大师之手")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("大師之手")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("마스터 핸드")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("ГЛАВНАЯ РУКА")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MÃO MESTRA") {
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);
                    if ModelModule::scale(module_accessor) != 0.0001 || !ItemModule::is_have_item(module_accessor, 0) {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        ModelModule::set_scale(boss_boma, 0.08);
                        MotionModule::change_motion(boss_boma, Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor, Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        ModelModule::set_joint_rotate(module_accessor, Hash40::new("root") , &mut Vector3f{x: -270.0, y: 180.0, z: -90.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                    }
                }
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        DEAD = false;
                        CONTROLLABLE = true;
                        JUMP_START = false;
                        TURNING = false;
                        STOP = false;
                        PUNCH = false;
                        BARK = false;
                        MASTER_USABLE = false;
                        SHOCK = false;
                        LASER = false;
                        SCRATCH_BLOW = false;
                        MASTER_FACING_LEFT = true;
                        MULTIPLE_BULLETS = 0;
                        CONTROLLER_X_MASTER = 0.0;
                        CONTROLLER_Y_MASTER = 0.0;
                        MASTER_TEAM = TeamModule::team_no(module_accessor);
                        if !CRAZY_EXISTS {
                            CRAZY_AI_LEVEL = 0.0;
                        }
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        println!("{}", cpu_level(fighter.lua_state_agent));
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            MASTER_EXISTS = true;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                WorkModule::set_float(boss_boma, 10.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                                MASTER_AI_LEVEL = 10.0;
                                WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            }
                            else {
                                WorkModule::set_float(boss_boma, cpu_level(fighter.lua_state_agent) + 1.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                                MASTER_AI_LEVEL = cpu_level(fighter.lua_state_agent);
                                WorkModule::set_float(boss_boma, (cpu_level(fighter.lua_state_agent) * 0.1) + 0.1, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            }
                            WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
                            WorkModule::set_int(boss_boma, *ITEM_BOSS_MODE_ADVENTURE_HARD, *ITEM_INSTANCE_WORK_INT_BOSS_MODE);
                            WorkModule::set_int(boss_boma, *ITEM_VARIATION_MASTERHAND_CRAZYHAND_STANDARD, *ITEM_INSTANCE_WORK_INT_VARIATION);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                            println!("{}", cpu_level(fighter.lua_state_agent));
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        let x = PostureModule::pos_x(boss_boma);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(boss_boma);
                        MASTER_X_POS = x;
                        MASTER_Y_POS = y;
                        MASTER_Z_POS = z;
                        if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                            let sub_hp = 999.0 - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                            MASTER_FACING_LEFT = false;
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                            MASTER_FACING_LEFT = true;
                        }
                        JostleModule::set_status(module_accessor, false);
                    }

                    // Team Attack Trigger
                    if sv_information::is_ready_go() == true {
                        println!("{}", cpu_level(fighter.lua_state_agent));
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                        if MotionModule::motion_kind(boss_boma) == hash40("wait") && FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                            if smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 == smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 {
                                if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 40.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_BARK, true);
                                        BARK = true;
                                    }
                                }
                            }
                            else if smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 == smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 {
                                if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                        CONTROLLABLE = false;
                                        SCRATCH_BLOW = true;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                    }
                                }
                            }
                            else if smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 == smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 {
                                if CRAZY_EXISTS == true && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                        CONTROLLABLE = false;
                                        SHOCK = true;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START, true);
                                    }
                                }
                            }
                        }
                    }

                    // STUBS AI

                    if sv_information::is_ready_go() {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if CONTROLLABLE {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
                                && !TURNING {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_DOWN_END
                                && !TURNING {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                            }
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
                                    if MotionModule::frame(module_accessor) >= MotionModule::end_frame(module_accessor) {
                                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                                        MASTER_EXISTS = false;
                                    }
                                }
                            }
                        }
                    }

                    if DEAD == true {
                        if sv_information::is_ready_go() == true {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_STANDBY {
                                    MASTER_EXISTS = false;
                                    PostureModule::set_lr(boss_boma, -1.0);
                                    if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
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
                        SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_05"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_013"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_09"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_common_punch_kick_swing_l"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win02"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_win2"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_014"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_win2"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win03"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_015"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_jump01"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_landing02"), 0);
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY && !EXISTS_PUBLIC_2 {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY && EXISTS_PUBLIC_2 {
                            CONTROLLABLE = true;
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                            if MASTER_TEAM == CRAZY_TEAM {
                                MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("entry2"),0.0,1.0,false,0.0,false,false);
                            }
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("entry2") {
                            MotionModule::set_rate(boss_boma, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.5);
                        }
                    }

                    // SET FIGHTER LOOP

                    if sv_information::is_ready_go() == true {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING); // I did yoink these transition terms and ability to hide the player cursor from Claude's awesome mod which can be found here: https://github.com/ClaudevonRiegan/Playable_Bosses
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ITEM);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_JUMP_AERIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_TREAD_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ITEM_THROW);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_WALL_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_CATCH);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_GUARD);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_CLIFF);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LASSO);
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            fighter.set_situation(SITUATION_KIND_AIR.into());
                            GroundModule::set_correct(module_accessor, smash::app::GroundCorrectKind(*GROUND_CORRECT_KIND_AIR));
                            MotionModule::change_motion(module_accessor,Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            // SET POS AND STOPS OUT OF BOUNDS
                            if ModelModule::scale(module_accessor) == 0.0001 {
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
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
                                if !CONTROLLABLE || FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                    }
                                    else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                    }
                                    else {
                                        PostureModule::set_pos(module_accessor, &boss_pos);
                                    }
                                }
                                else {
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                    }
                                    else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
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
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);
                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma, i, false) {
                            AttackModule::set_target_category(boss_boma, i, *COLLISION_CATEGORY_MASK_ALL as u32);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if FighterUtil::is_hp_mode(module_accessor) == false {
                            if DamageModule::damage(module_accessor, 0) >= 399.0 {
                                if DEAD == false {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                    DEAD = true;
                                }
                            }
                        }
                    }

                    // FIXES SPAWN

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            if JUMP_START == false {
                                if CRAZY_AI_LEVEL > MASTER_AI_LEVEL {
                                    WorkModule::set_float(boss_boma, CRAZY_AI_LEVEL, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                                    WorkModule::set_float(boss_boma, (CRAZY_AI_LEVEL * 0.1) + 0.1, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                                }
                                JUMP_START = true;
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                    CONTROLLABLE = false;
                                }
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true && !DEAD {
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                let master_pos = Vector3f{x: CRAZY_X_POS + 100.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                let master_pos = Vector3f{x: CRAZY_X_POS - 100.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_BARK {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 95.0 && MotionModule::frame(boss_boma) <= MotionModule::end_frame(boss_boma) - 92.0 {
                                MotionModule::set_rate(boss_boma, 0.1);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 0.1);
                            }
                            else {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            }
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    BARK = false;
                                    println!("STOPPED AT 1199");
                                }
                            }
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    BARK = false;
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                LASER = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                SCRATCH_BLOW = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                PUNCH = false;
                            }
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_DOWN_END,true);
                            CONTROLLABLE = false;
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("electroshock_start") && SHOCK {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            CONTROLLABLE = false;
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK, true);
                            }
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false
                        && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_END && !CONTROLLABLE && SHOCK {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                CONTROLLABLE = true;
                                SHOCK = false;
                                println!("STOPPED AT 584");
                            }
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true
                        && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_END && SHOCK {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                SHOCK = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME
                        || StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT
                        || CONTROLLABLE {
                            MASTER_USABLE = true;
                        }
                        else {
                            MASTER_USABLE = false;
                        }

                        if PUNCH && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA
                        && CRAZY_EXISTS
                        && !DEAD
                        && MASTER_USABLE {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                    let master_pos = Vector3f{x: CRAZY_X_POS - 130.0, y: CRAZY_Y_POS + 15.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                                else {
                                    let master_pos = Vector3f{x: CRAZY_X_POS - 130.0, y: CRAZY_Y_POS + 10.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                    let master_pos = Vector3f{x: CRAZY_X_POS + 130.0, y: CRAZY_Y_POS + 15.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                                else {
                                    let master_pos = Vector3f{x: CRAZY_X_POS + 130.0, y: CRAZY_Y_POS + 10.0, z: CRAZY_Z_POS};
                                    PostureModule::set_pos(boss_boma, &master_pos);
                                }
                            }
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_GOOPAA,true);
                        }
                        if PUNCH && !DEAD && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA && MASTER_USABLE {
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                let master_pos = Vector3f{x: MASTER_X_POS, y: CRAZY_Y_POS + 15.0, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            else {
                                let master_pos = Vector3f{x: MASTER_X_POS, y: CRAZY_Y_POS + 10.0, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 {
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 606");
                                }
                            }
                        }

                        if LASER && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START
                        && CRAZY_EXISTS
                        && !DEAD
                        && MASTER_USABLE {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                let master_pos = Vector3f{x: CRAZY_X_POS + 130.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            if smash::app::lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                let master_pos = Vector3f{x: CRAZY_X_POS - 130.0, y: CRAZY_Y_POS, z: CRAZY_Z_POS};
                                PostureModule::set_pos(boss_boma, &master_pos);
                            }
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START,true);
                        }
                        if LASER && !DEAD && StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START && MASTER_USABLE {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 {
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 653");
                                }
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

                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER * 0.75, y: CONTROLLER_Y_MASTER * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                            MotionModule::set_rate(boss_boma, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
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
                            PostureModule::set_pos(boss_boma, &Vector3f{x: PostureModule::pos_x(boss_boma), y: Y_POS, z: PostureModule::pos_z(boss_boma)});
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU {
                            PostureModule::set_pos(boss_boma, &Vector3f{x: PostureModule::pos_x(boss_boma), y: Y_POS, z: PostureModule::pos_z(boss_boma)});
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START {
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.75, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START {
                            MotionModule::set_rate(boss_boma, 1.0);
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL {
                            CONTROLLABLE = false;
                            MotionModule::set_rate(boss_boma, 1.1);
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_PRE_MOVE {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL {
                            MotionModule::set_rate(boss_boma, 1.3);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_START {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
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
                            MotionModule::set_rate(boss_boma, 1.1);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.1);
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LOOP
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_FALL
                        || StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_LANDING {
                            CONTROLLABLE = false;
                        }
                        if MotionModule::is_end(boss_boma) && MotionModule::motion_kind(boss_boma) == hash40("teleport_end") {
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                MotionModule::change_motion(boss_boma,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE, true);
                            }
                            else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT, true);
                                MotionModule::change_motion(boss_boma,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1009");
                            }
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CANCEL {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    MotionModule::set_rate(boss_boma, 2.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1015");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    SCRATCH_BLOW = false;
                                    println!("STOPPED AT 1014");
                                }
                            }
                            if MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                                CONTROLLABLE = true;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_GOOPAA {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1022");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1050");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_ENERGY_SHOT_RUSH_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1057");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_THROW_END_1 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1073");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1080");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1087");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1094");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_MISS_END {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1102");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1110");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1116");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DEBUG_WAIT {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 2.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1118");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_KENZAN_START, true);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_KENZAN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1127");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1133");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1141");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIKOUKI_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1148");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_IRON_BALL_END {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1155");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1162");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE = true;
                                if MotionModule::frame(boss_boma) >= 40.0 {
                                    TURNING = false;
                                    println!("STOPPED AT 1178");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_START {
                                CONTROLLABLE = false;
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_DOWN_END {
                                CONTROLLABLE = false;
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1189");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1205");
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME {
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1215");
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT {
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1220");
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_RND_WAIT {
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1225");
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE {
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1230");
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_TO_POINT {
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1235");
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    println!("STOPPED AT 1241");
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT {
                                CONTROLLABLE = true;
                                println!("STOPPED AT 1248");
                            }
                        }

                        if TURNING {
                            MotionModule::set_rate(boss_boma, 1.4);
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
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == false {
                                    MULTIPLE_BULLETS = 0;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == true {
                                    MULTIPLE_BULLETS = 2;
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

                        if CONTROLLABLE {
                            MULTIPLE_BULLETS = 0;
                        }

                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
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
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
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
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("teleport_start") && MotionModule::is_end(boss_boma) {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            MotionModule::change_motion(boss_boma,Hash40::new("teleport_end"),0.0,1.0,false,0.0,false,false);
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
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CENTER_MOVE {
                            MotionModule::set_rate(boss_boma, 4.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.4);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START {
                            MotionModule::set_rate(boss_boma, 1.25);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.25);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_PRE_MOVE {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_CHAKRAM_START, true);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CAPTURE {
                            MotionModule::set_rate(boss_boma, 4.75);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 4.75);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_NIGIRU {
                            MotionModule::set_rate(boss_boma, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_TURN {
                            // X Controllable
                            if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                CONTROLLER_X_MASTER = 0.0;
                            }
                            if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                CONTROLLER_Y_MASTER = 0.0;
                            }
                            if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    MotionModule::set_rate(boss_boma, 1.0);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                    CONTROLLABLE = true;
                                    TURNING = false;
                                    println!("STOPPED AT TURN");
                                }
                            }
                        }
                        if MotionModule::frame(boss_boma) <= 0.0 && MotionModule::motion_kind(boss_boma) == hash40("teleport_end")  {
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: -140.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_x(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 140.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: 0.0, y: -50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 0.0, y: 50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                    }
                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_GOOPAA && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_WFINGER_BEAM_START {
                        if TURNING == false {
                            if CONTROLLABLE == true {
                                if DEAD == false {
                                    let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                                    //Boss Control Stick Movement

                                    // X Controllable
                                    if CONTROLLER_X_MASTER < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                        CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_MASTER > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_MASTER <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                        CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                        CONTROLLER_X_MASTER -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                        CONTROLLER_X_MASTER += CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_MASTER > 0.0 && CONTROLLER_X_MASTER < 0.02 {
                                        CONTROLLER_X_MASTER = 0.0;
                                    }
                                    if CONTROLLER_X_MASTER < 0.0 && CONTROLLER_X_MASTER > 0.02 {
                                        CONTROLLER_X_MASTER = 0.0;
                                    }
                                    if CONTROLLER_X_MASTER > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                        CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_MASTER < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                        CONTROLLER_X_MASTER += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }

                                    // Y Controllable
                                    if CONTROLLER_Y_MASTER < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                        CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_MASTER > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_MASTER <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                        CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                        CONTROLLER_Y_MASTER -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                        CONTROLLER_Y_MASTER += CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_MASTER > 0.0 && CONTROLLER_Y_MASTER < 0.02 {
                                        CONTROLLER_Y_MASTER = 0.0;
                                    }
                                    if CONTROLLER_Y_MASTER < 0.0 && CONTROLLER_Y_MASTER > 0.02 {
                                        CONTROLLER_Y_MASTER = 0.0;
                                    }
                                    if CONTROLLER_Y_MASTER > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                        CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_MASTER < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                        CONTROLLER_Y_MASTER += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }

                                    let pos = Vector3f{x: CONTROLLER_X_MASTER, y: CONTROLLER_Y_MASTER, z: 0.0};
                                    PostureModule::add_pos(boss_boma, &pos);

                                    if TURNING == false {
                                        // Boss Moves
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
                                            if CRAZY_EXISTS == true && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                                if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                                || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                                    CONTROLLABLE = false;
                                                    SHOCK = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_ELECTROSHOCK_START, true);
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) && !TURNING && MotionModule::motion_kind(boss_boma) != smash::hash40("teleport_start") && MotionModule::motion_kind(boss_boma) != smash::hash40("teleport_end") && StatusModule::status_kind(boss_boma) != *ITEM_MASTERHAND_STATUS_KIND_TURN {
                                            CONTROLLABLE = false;
                                            MotionModule::set_rate(boss_boma, 1.0);
                                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                            MotionModule::change_motion(boss_boma,Hash40::new("teleport_start"),0.0,1.0,false,0.0,false,false);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                            CONTROLLABLE = false;
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_NIGIRU_CAPTURE, true);
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAINT_BALL_START, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                            CONTROLLABLE = false;
                                            MotionModule::set_rate(boss_boma, 2.0);
                                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                            CONTROLLABLE = false;
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SATELLITE_GUN_START, true);
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
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_START, true);
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
                                            Y_POS = PostureModule::pos_y(boss_boma);
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 40.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                                if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                                || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                                    CONTROLLABLE = false;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_BARK, true);
                                                    BARK = true;
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && CRAZY_EXISTS && CRAZY_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                                if lua_bind::PostureModule::lr(boss_boma) == -1.0 && CRAZY_FACING_RIGHT // Master Hand Facing left but Crazy Hand facing right, next line is opposite
                                                || lua_bind::PostureModule::lr(boss_boma) == 1.0 && !CRAZY_FACING_RIGHT {
                                                    CONTROLLABLE = false;
                                                    SCRATCH_BLOW = true;
                                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                                                }
                                            }
                                            else {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 40.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 25.0 {
                                                CONTROLLABLE = false;
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_KENZAN_PRE_MOVE, true);
                                            }
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
                    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);
                    if ModelModule::scale(module_accessor) != 0.0001 || !ItemModule::is_have_item(module_accessor, 0) {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                        BOSS_ID_2[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                        ModelModule::set_scale(boss_boma_2, 0.08);
                        MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor,Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        ModelModule::set_joint_rotate(module_accessor, Hash40::new("root") , &mut Vector3f{x: -270.0, y: 180.0, z: -90.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                    }
                }
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        DEAD_2 = false;
                        CONTROLLABLE_2 = true;
                        JUMP_START_2 = false;
                        TURNING_2 = false;
                        STOP_2 = false;
                        CRAZY_USABLE = false;
                        BARK = false;
                        PUNCH = false;
                        SHOCK = false;
                        SCRATCH_BLOW = false;
                        CRAZY_EXISTS = true;
                        CRAZY_FACING_RIGHT = true;
                        LASER = false;
                        CONTROLLER_X_CRAZY = 0.0;
                        CONTROLLER_Y_CRAZY = 0.0;
                        CRAZY_TEAM = TeamModule::team_no(module_accessor);
                        if !MASTER_EXISTS {
                            MASTER_AI_LEVEL = 0.0;
                        }
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID_2 = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            EXISTS_PUBLIC_2 = true;
                            RESULT_SPAWNED_2 = false;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                            BOSS_ID_2[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            WorkModule::set_int(boss_boma_2, *ITEM_BOSS_MODE_ADVENTURE_HARD, *ITEM_INSTANCE_WORK_INT_BOSS_MODE);
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                WorkModule::set_float(boss_boma_2, 10.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                                CRAZY_AI_LEVEL = 10.0;
                                WorkModule::set_float(boss_boma_2, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            }
                            else {
                                WorkModule::set_float(boss_boma_2, cpu_level(fighter.lua_state_agent) + 1.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                                CRAZY_AI_LEVEL = cpu_level(fighter.lua_state_agent);
                                WorkModule::set_float(boss_boma_2, (cpu_level(fighter.lua_state_agent) * 0.1) + 0.1, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            }
                            WorkModule::on_flag(boss_boma_2, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
                            WorkModule::set_int(boss_boma_2, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
                            WorkModule::set_int(boss_boma_2, *ITEM_VARIATION_CRAZYHAND_MASTERHAND_STANDARD, *ITEM_INSTANCE_WORK_INT_VARIATION);
                            WorkModule::set_float(boss_boma_2, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                            WorkModule::set_float(boss_boma_2, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                        let x = PostureModule::pos_x(boss_boma);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(boss_boma);
                        CRAZY_X_POS = x;
                        CRAZY_Y_POS = y;
                        CRAZY_Z_POS = z;
                        if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                            let sub_hp = 999.0 - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                            CRAZY_FACING_RIGHT = true;
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                            CRAZY_FACING_RIGHT = false;
                        }
                        JostleModule::set_status(module_accessor, false);
                    }

                    // STUBS AI

                    if sv_information::is_ready_go() {
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == false {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            if CONTROLLABLE_2 {
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
                                && !TURNING_2 {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                                if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_TURN
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                                && StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END
                                && !TURNING_2 {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                }
                            }
                        }
                    }

                    // Team Attack Trigger
                    if sv_information::is_ready_go() == true {
                        let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                        let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                        if MotionModule::motion_kind(boss_boma_2) == hash40("wait") && FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == true {
                            if smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 == smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 {
                                if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                        CONTROLLABLE_2 = false;
                                        LASER = true;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                    }
                                }
                            }
                            else if smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 == smash::app::sv_math::rand(hash40("fighter"), 1000) as f32 {
                                if MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                    if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                    || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                        CONTROLLABLE_2 = false;
                                        PUNCH = true;
                                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                        MotionModule::change_motion(boss_boma_2,Hash40::new("taggoopaa"),0.0,1.0,false,0.0,false,false);
                                    }
                                }
                            }
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
                                    if MotionModule::frame(module_accessor) >= MotionModule::end_frame(module_accessor) {
                                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                                    }
                                }
                            }
                        }
                    }

                    if DEAD_2 == true {
                        if sv_information::is_ready_go() == true {
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_DEAD {
                                if StatusModule::status_kind(boss_boma_2) != *ITEM_STATUS_KIND_STANDBY {
                                    CRAZY_EXISTS = false;
                                    PostureModule::set_lr(boss_boma_2, 1.0);
                                    if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
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
                            CRAZY_EXISTS = false;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_CRAZYHAND), 0, 0, false, false);
                            BOSS_ID_2[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                            ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("entry"),0.0,1.0,false,0.0,false,false);
                        }
                        SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_05"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_013"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_09"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_common_punch_kick_swing_l"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win02"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_win2"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_014"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_win2"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win03"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_015"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_jump01"), 0);
                        SoundModule::stop_se(module_accessor, Hash40::new("se_mario_landing02"), 0);
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 {
                        let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY && !EXISTS_PUBLIC {
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_ENTRY && EXISTS_PUBLIC {
                            CONTROLLABLE_2 = true;
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            if MASTER_TEAM == CRAZY_TEAM {
                                MotionModule::change_motion(boss_boma_2,smash::phx::Hash40::new("entry2"),0.0,1.0,false,0.0,false,false);
                            }
                        }
                        if MotionModule::motion_kind(boss_boma_2) == smash::hash40("entry2") {
                            MotionModule::set_rate(boss_boma_2, 1.5);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.5);
                        }
                    }

                    //DAMAGE MODULES

                    let boss_boma_2 = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);
                    HitModule::set_whole(boss_boma_2, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma_2, i, false) {
                            AttackModule::set_target_category(boss_boma_2, i, *COLLISION_CATEGORY_MASK_ALL as u32);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if FighterUtil::is_hp_mode(module_accessor) == false {
                            if DamageModule::damage(module_accessor, 0) >= 399.0 {
                                if DEAD_2 == false {
                                    CONTROLLABLE_2 = false;
                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_STATUS_KIND_DEAD, true);
                                    DEAD_2 = true;
                                }
                            }
                        }
                    }

                    // SET FIGHTER LOOP

                    if sv_information::is_ready_go() == true {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ITEM);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_SPECIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_JUMP_AERIAL);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_TREAD_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ITEM_THROW);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_WALL_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_CATCH);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_JUMP);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_GUARD);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ATTACK);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ESCAPE);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_CLIFF);
                            WorkModule::enable_transition_term_forbid_group(module_accessor, *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LASSO);
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            fighter.set_situation(SITUATION_KIND_AIR.into());
                            GroundModule::set_correct(module_accessor, smash::app::GroundCorrectKind(*GROUND_CORRECT_KIND_AIR));
                            MotionModule::change_motion(module_accessor,Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if DEAD_2 == false {
                        if sv_information::is_ready_go() == true {
                            // SET POS AND STOPS OUT OF BOUNDS
                            if ModelModule::scale(module_accessor) == 0.0001 {
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID_2[entry_id(module_accessor)]);
                                if FighterUtil::is_hp_mode(module_accessor) == true {
                                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                                        if DEAD_2 == false {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                            DEAD_2 = true;
                                        }
                                    }
                                }
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f{x: x, y: y + 20.0, z: z};
                                if !CONTROLLABLE_2 || FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                    }
                                    else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                    }
                                    else {
                                        PostureModule::set_pos(module_accessor, &boss_pos);
                                    }
                                }
                                else {
                                    if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                        let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                        let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                    }
                                    else if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                        let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: y, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                            let boss_y_pos_1 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: y, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                    }
                                    else if PostureModule::pos_y(boss_boma) >= dead_range(fighter.lua_state_agent).y.abs() - 100.0 {
                                        let boss_y_pos_1 = Vector3f{x: x, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        if PostureModule::pos_y(boss_boma) <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0 {
                                            let boss_y_pos_2 = Vector3f{x: x, y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0) + 160.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma) >= dead_range(fighter.lua_state_agent).x.abs() - 100.0 {
                                            let boss_x_pos_1 = Vector3f{x: dead_range(fighter.lua_state_agent).x.abs() - 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma) <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0 {
                                            let boss_x_pos_2 = Vector3f{x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0) + 100.0, y: dead_range(fighter.lua_state_agent).y.abs() - 100.0, z: z};
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                    }
                                    else {
                                        PostureModule::set_pos(module_accessor, &boss_pos);
                                    }
                                }
                            }
                        }
                    }

                    if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT
                    || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT
                    || CONTROLLABLE_2 {
                        CRAZY_USABLE = true;
                    }
                    else {
                        CRAZY_USABLE = false;
                    }

                    if BARK && !DEAD_2 && MASTER_EXISTS && MotionModule::motion_kind(boss_boma_2) != smash::hash40("bark") && CRAZY_USABLE {
                        MotionModule::set_rate(boss_boma_2, 1.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        CONTROLLABLE_2 = false;
                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                        MotionModule::change_motion(boss_boma_2,Hash40::new("bark"),0.0,1.0,false,0.0,false,false);
                    }

                    if MotionModule::motion_kind(boss_boma_2) == hash40("bark") {
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            let master_pos = Vector3f{x: MASTER_X_POS + 30.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            let master_pos = Vector3f{x: MASTER_X_POS - 30.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                    }

                    if SCRATCH_BLOW && !DEAD_2 && MASTER_EXISTS && MotionModule::motion_kind(boss_boma_2) != smash::hash40("bark") && CRAZY_USABLE {
                        CONTROLLABLE_2 = false;
                        MotionModule::set_rate(boss_boma_2, 1.4);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                    }

                    if MotionModule::motion_kind(boss_boma_2) == smash::hash40("electroshock_start")
                    || MotionModule::motion_kind(boss_boma_2) == smash::hash40("electroshock")
                    || MotionModule::motion_kind(boss_boma_2) == smash::hash40("electroshock_end") {
                        MotionModule::set_rate(boss_boma_2, 1.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            let master_pos = Vector3f{x: MASTER_X_POS + 100.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            let master_pos = Vector3f{x: MASTER_X_POS - 100.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                    }

                    if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START {
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            let master_pos = Vector3f{x: MASTER_X_POS - 200.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            let master_pos = Vector3f{x: MASTER_X_POS + 200.0, y: MASTER_Y_POS, z: MASTER_Z_POS};
                            PostureModule::set_pos(boss_boma_2, &master_pos);
                        }
                    }

                    if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("teleport_end") {
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager, smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == true {
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE, true);
                        }
                        else {
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            CONTROLLABLE_2 = true;
                        }
                    }

                    if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 && MotionModule::motion_kind(boss_boma_2) == hash40("bark") {
                        MotionModule::set_rate(boss_boma_2, 1.0);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == true {
                            BARK = false;
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE, true);
                        }
                        else {
                            BARK = false;
                            CONTROLLABLE_2 = true;
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME, true);
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if SHOCK && !DEAD_2 && MASTER_EXISTS && MotionModule::motion_kind(boss_boma_2) != smash::hash40("electroshock_start")
                        && MotionModule::motion_kind(boss_boma_2) != smash::hash40("electroshock")
                        && MotionModule::motion_kind(boss_boma_2) != smash::hash40("electroshock_end")
                        && CRAZY_USABLE {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            CONTROLLABLE_2 = false;
                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                            MotionModule::change_motion(boss_boma_2,Hash40::new("electroshock_start"),0.0,1.0,false,0.0,false,false);
                        }

                        if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("electroshock_start") {
                            MotionModule::change_motion(boss_boma_2,Hash40::new("electroshock"),0.0,1.0,false,0.0,false,false);
                        }

                        if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("electroshock") {
                            MotionModule::change_motion(boss_boma_2,Hash40::new("electroshock_end"),0.0,1.0,false,0.0,false,false);
                        }

                        if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("electroshock_end") {
                            if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                                CONTROLLABLE_2 = true;
                                SHOCK = false;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                            }
                            else {
                                SHOCK = false;
                                MotionModule::change_motion(boss_boma_2,Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE, true);
                            }
                        }
                    }

                    // FIXES SPAWN

                    if DEAD_2 == false {
                        if sv_information::is_ready_go() == true {
                            if JUMP_START_2 == false {
                                if MASTER_AI_LEVEL > CRAZY_AI_LEVEL {
                                    WorkModule::set_float(boss_boma_2, MASTER_AI_LEVEL, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                                    WorkModule::set_float(boss_boma_2, (MASTER_AI_LEVEL * 0.1) + 0.1, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                                }
                                JUMP_START_2 = true;
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == true {
                                    CONTROLLABLE_2 = false;
                                }
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true && !DEAD_2 {
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                            StatusModule::change_status_request_from_script(boss_boma_2,*ITEM_CRAZYHAND_STATUS_KIND_DOWN_END,true);
                            CONTROLLABLE_2 = false;
                        }
                        if CONTROLLABLE_2 && TURNING_2 {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma_2, 2.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 2.0);
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 0.75, y: CONTROLLER_Y_CRAZY * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
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
                            MotionModule::set_rate(boss_boma_2, 1.4);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.4);
                        }
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == false {
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START {
                                if MotionModule::frame(boss_boma_2) == 40.0 {
                                    WorkModule::set_flag(boss_boma_2, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_PINKY);
                                }
                                if MotionModule::frame(boss_boma_2) == 55.0 {
                                    WorkModule::set_flag(boss_boma_2, true, *ITEM_CRAZYHAND_INSTANCE_WORK_FLAG_FIRE_CHARIOT_THUMB);
                                }
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == 117 {
                            if MotionModule::frame(boss_boma_2) == MotionModule::end_frame(boss_boma_2) - 2.0 {
                                PostureModule::set_pos(boss_boma_2, &Vector3f{x: 0.0, y: 20.0, z: 0.0});
                                lua_bind::PostureModule::set_lr(boss_boma_2, 1.0);
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_START, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_LOOP {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 2.0, y: CONTROLLER_Y_CRAZY * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }

                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 2.0, y: CONTROLLER_Y_CRAZY * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                            //Boss Control Stick Movement
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK {
                            MotionModule::set_rate(boss_boma_2, 1.2);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.2);
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY * 0.5, y: CONTROLLER_Y_CRAZY * 0.5, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START
                        || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP
                        || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL
                        || StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LANDING {
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
                            PostureModule::set_pos(boss_boma_2, &Vector3f{x: PostureModule::pos_x(boss_boma_2), y: Y_POS_2, z: PostureModule::pos_z(boss_boma_2)});
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU {
                            PostureModule::set_pos(boss_boma_2, &Vector3f{x: PostureModule::pos_x(boss_boma_2), y: Y_POS_2, z: PostureModule::pos_z(boss_boma_2)});
                            //Boss Control Stick Movement
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
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
                            // X Controllable
                            if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                CONTROLLER_X_CRAZY = 0.0;
                            }
                            if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            // Y Controllable
                            if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                CONTROLLER_Y_CRAZY = 0.0;
                            }
                            if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                            PostureModule::add_pos(boss_boma_2, &pos);
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
                        if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) {
                            MotionModule::set_rate(boss_boma_2, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                            if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 10.0 {
                                LASER = false;
                            }
                        }
                        if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                            if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                SCRATCH_BLOW = false;
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
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME {
                                CONTROLLABLE_2 = true;
                            }
                            if MotionModule::motion_kind(boss_boma_2) == smash::hash40("wait") {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_RND_WAIT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TO_POINT {
                                CONTROLLABLE_2 = true;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT {
                                CONTROLLABLE_2 = true;
                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_PH_RANDOM_TIME_WAIT, true);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT {
                                CONTROLLABLE_2 = true;
                                TURNING_2 = false;
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_STATUS_KIND_WAIT {
                                CONTROLLABLE_2 = true;
                                if MotionModule::frame(boss_boma_2) >= 40.0 {
                                    TURNING_2 = false;
                                }
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
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_END {
                                MotionModule::set_rate(boss_boma_2, 1.6);
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 2.0 {
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
                            if MotionModule::motion_kind(boss_boma_2) == smash::hash40("teleport_start") && MotionModule::is_end(boss_boma_2) {
                                MotionModule::set_rate(boss_boma_2, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                MotionModule::change_motion(boss_boma_2,Hash40::new("teleport_end"),0.0,1.0,false,0.0,false,false);
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                // X Controllable
                                if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                    CONTROLLER_X_CRAZY = 0.0;
                                }
                                if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                    CONTROLLER_Y_CRAZY = 0.0;
                                }
                                if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                                if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == false {
                                    if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                        MotionModule::set_rate(boss_boma_2, 1.0);
                                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                        CONTROLLABLE_2 = true;
                                        TURNING_2 = false;
                                    }
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma_2) >= MotionModule::end_frame(boss_boma_2) - 5.0 {
                                    CONTROLLABLE_2 = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma_2) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {
                                CONTROLLABLE_2 = false;
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
                        if TURNING_2 {
                            MotionModule::set_rate(boss_boma_2, 1.4);
                        }
                        if MotionModule::frame(boss_boma_2) <= 0.0 && MotionModule::motion_kind(boss_boma_2) == hash40("teleport_end") {
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: -140.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if ControlModule::get_stick_x(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 140.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) <= 0.5 {
                                let pos = Vector3f{x: 0.0, y: -50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                            if ControlModule::get_stick_y(module_accessor) >= -0.5 {
                                let pos = Vector3f{x: 0.0, y: 50.0, z: 0.0};
                                PostureModule::add_pos(boss_boma_2, &pos);
                            }
                        }
                    }
                    if MotionModule::motion_kind(boss_boma_2) == smash::hash40("taggoopaa") {
                        MotionModule::set_rate(boss_boma_2, 1.3);
                        smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.3);
                        let x = PostureModule::pos_x(boss_boma_2);
                        CONTROLLABLE_2 = false;
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == 1.0 { // right
                            if MotionModule::frame(boss_boma_2) >= 120.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: -0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if MotionModule::frame(boss_boma_2) >= 130.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    if x < MASTER_X_POS - 25.0 {
                                        let pos = Vector3f{x: 14.75, y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                }
                            }
                        }
                        if smash::app::lua_bind::PostureModule::lr(boss_boma_2) == -1.0 { // left
                            if MotionModule::frame(boss_boma_2) >= 120.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    let pos = Vector3f{x: 0.5, y: 0.0, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
                                }
                            }
                            if MotionModule::frame(boss_boma_2) >= 130.0 {
                                if MotionModule::frame(boss_boma_2) <= 140.0 {
                                    if x > MASTER_X_POS + 25.0 {
                                        let pos = Vector3f{x: -14.75, y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma_2, &pos);
                                    }
                                }
                            }
                        }
                    }

                    if MotionModule::is_end(boss_boma_2) && MotionModule::motion_kind(boss_boma_2) == hash40("taggoopaa") {
                        PUNCH = false;
                        StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }

                    if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID_2 as i32))) == false {
                        if TURNING_2 == false {
                            if CONTROLLABLE_2 == true {
                                if DEAD_2 == false {
                                    let curr_pos = Vector3f{x: PostureModule::pos_x(module_accessor), y: PostureModule::pos_y(module_accessor), z: PostureModule::pos_z(module_accessor)};
                                    // Boss Control Movement
                                    // X Controllable
                                    if CONTROLLER_X_CRAZY < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                        CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X_CRAZY <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                        CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                        CONTROLLER_X_CRAZY -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                        CONTROLLER_X_CRAZY += CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY > 0.0 && CONTROLLER_X_CRAZY < 0.02 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0 && CONTROLLER_X_CRAZY > 0.02 {
                                        CONTROLLER_X_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_X_CRAZY > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                        CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_X_CRAZY < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                        CONTROLLER_X_CRAZY += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }

                                    // Y Controllable
                                    if CONTROLLER_Y_CRAZY < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                        CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y_CRAZY <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                        CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                        CONTROLLER_Y_CRAZY -= CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                        CONTROLLER_Y_CRAZY += CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY > 0.0 && CONTROLLER_Y_CRAZY < 0.02 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0 && CONTROLLER_Y_CRAZY > 0.02 {
                                        CONTROLLER_Y_CRAZY = 0.0;
                                    }
                                    if CONTROLLER_Y_CRAZY > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                        CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    if CONTROLLER_Y_CRAZY < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                        CONTROLLER_Y_CRAZY += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                    }
                                    let pos = Vector3f{x: CONTROLLER_X_CRAZY, y: CONTROLLER_Y_CRAZY, z: 0.0};
                                    PostureModule::add_pos(boss_boma_2, &pos);
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
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) && MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                            if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                            || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                                // if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                                    CONTROLLABLE_2 = false;
                                                    PUNCH = true;
                                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                                    MotionModule::change_motion(boss_boma_2,Hash40::new("taggoopaa"),0.0,1.0,false,0.0,false,false);
                                                // }
                                                // else {
                                                    // CONTROLLABLE_2 = false;
                                                    // StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                                    // set_camera_range(smash::phx::Vector4f{x: dead_range(fighter.lua_state_agent).x.abs() / 2.0, y: dead_range(fighter.lua_state_agent).y.abs() / 2.0, z: dead_range(fighter.lua_state_agent).z.abs() / 2.0, w: dead_range(fighter.lua_state_agent).w.abs()});
                                                    // MotionModule::change_motion(boss_boma_2,Hash40::new("finder"),0.0,1.0,false,0.0,false,false);
                                                // }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) && !TURNING_2 && MotionModule::motion_kind(boss_boma_2) != smash::hash40("teleport_start") && MotionModule::motion_kind(boss_boma_2) != smash::hash40("teleport_end") && StatusModule::status_kind(boss_boma_2) != *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                                            CONTROLLABLE_2 = false;
                                            MotionModule::set_rate(boss_boma_2, 1.0);
                                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma_2, 1.0);
                                            MotionModule::change_motion(boss_boma_2,Hash40::new("teleport_start"),0.0,1.0,false,0.0,false,false);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                            CONTROLLABLE_2 = false;
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 30.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 5.0 {
                                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DIG_START, true);
                                            }
                                            else {
                                                Y_POS_2 = PostureModule::pos_y(boss_boma_2);
                                                StatusModule::change_status_request(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                            CONTROLLABLE_2 = false;
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_READY, true);
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
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 40.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 15.0 {
                                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_KUMO, true);
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE, true);
                                            }
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                        }
                                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                            Y_POS_2 = PostureModule::pos_y(boss_boma_2);
                                            CONTROLLABLE_2 = false;
                                            StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 && MASTER_EXISTS && MASTER_USABLE && MASTER_TEAM == CRAZY_TEAM {
                                                if lua_bind::PostureModule::lr(boss_boma_2) == 1.0 && MASTER_FACING_LEFT // Crazy Hand Facing right but Master Hand facing left, next line is opposite
                                                || lua_bind::PostureModule::lr(boss_boma_2) == -1.0 && !MASTER_FACING_LEFT {
                                                    CONTROLLABLE_2 = false;
                                                    LASER = true;
                                                    StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                                }
                                            }
                                        }
                                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                            CONTROLLABLE_2 = false;
                                            if GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) <= 50.0 && GroundModule::get_distance_to_floor(module_accessor, &curr_pos, curr_pos.y, true) > 0.0 {
                                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU, true);
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma_2, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_CAPTURE, true);
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
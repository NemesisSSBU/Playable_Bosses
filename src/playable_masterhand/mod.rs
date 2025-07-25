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
use smash::app::FighterUtil;
use smash::hash40;
use smash::app::utility::get_category;
use smash::phx::Hash40;
use smashline::{Agent, Main};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use skyline::nn::oe::{Initialize, GetDisplayVersion, DisplayVersion};

static mut CONTROLLABLE : bool = true;
static mut STOP : bool = false;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
static mut DEAD : bool = false;
static mut RESULT_SPAWNED : bool = false;
pub static mut FIGHTER_MANAGER: usize = 0;
pub static mut FIGHTER_NAME: [u64;9] = [0;9];
static mut EXISTS_PUBLIC : bool = false;
static mut FRESH_CONTROL : bool = false;
static mut JUMP_START : bool = false;
static mut CONTROLLER_X: f32 = 0.0;
static mut CONTROLLER_Y: f32 = 0.0;
static mut CONTROL_SPEED_MUL: f32 = 2.0;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;

use crate::config::{Config, load_config};

pub static TITLE_VERSION: Lazy<(u16, u16, u16)> = Lazy::new(|| {
    unsafe {
        Initialize();
        let mut display_version = std::mem::MaybeUninit::<DisplayVersion>::uninit();
        GetDisplayVersion(display_version.as_mut_ptr());
        let version = display_version.assume_init();
        let name = std::str::from_utf8(&version.name)
            .unwrap_or_default()
            .trim_end_matches(char::from(0))
            .to_string();
        let mut parts = name.split('.').filter_map(|s| s.parse::<u16>().ok());
        let major = parts.next().unwrap_or(0);
        let minor = parts.next().unwrap_or(0);
        let micro = parts.next().unwrap_or(0);
        (major, minor, micro)
    }
});

pub unsafe fn get_version_offset() -> u64 {
    let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
    let offset = match *TITLE_VERSION {
        (13, 0, 4) => 0x52C4758,
        (13, 0, 3) => 0x52C5758,
        (13, 0, 2) => 0x52C3758,
        _ => 0x52C4758, // fallback
    };
    text + offset
}

pub static CONFIG: Lazy<RwLock<Config>> = Lazy::new(|| {
    RwLock::new(load_config())
});

extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
}

pub unsafe fn check_status() -> bool {
    return EXISTS_PUBLIC;
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
        while get_category(owner_module_accessor) != *BATTLE_OBJECT_CATEGORY_FIGHTER {
            owner_module_accessor = &mut *sv_battle_object::module_accessor((WorkModule::get_int(owner_module_accessor, *WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER)) as u32);
        }
        player_number = WorkModule::get_int(owner_module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
    }
    return player_number;
}

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
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
                &raw mut FIGHTER_MANAGER,
                "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
                .as_bytes()
                .as_ptr(),
            );
            let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
            
            let name_base = get_version_offset();
            // println!("{}", hash40(&read_tag(name_base + 0x260 * get_player_number(&mut *fighter.module_accessor) as u64 + 0x8e)));
            FIGHTER_NAME[get_player_number(&mut *fighter.module_accessor)] = hash40(&read_tag(name_base + 0x260 * get_player_number(&mut *fighter.module_accessor) as u64 + 0x8e));
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("WOL MASTER HAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEWTWO MASTER HAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("ミュウツー マスターハンド")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MAIN DE MAÎTRE MEWTWO")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEWTWO MEISTERHAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEWTWO MANO MAESTRA")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEWTWO MAESTRA MANO")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEWTWO MEESTERHAND")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("MEWTWO МАСТЕР РУКА")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("뮤츠 마스터 핸드")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("超梦大师")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("超夢大師") {
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 || !ItemModule::is_have_item(module_accessor, 0) {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                        SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
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
                    if ModelModule::scale(module_accessor) != 0.0001 {
                        *CONFIG.write() = load_config();
                        EXISTS_PUBLIC = true;
                        CONTROLLABLE = true;
                        DEAD = false;
                        RESULT_SPAWNED = false;
                        STOP = false;
                        FRESH_CONTROL = false;
                        JUMP_START = false;
                        CONTROLLER_X = 0.0;
                        CONTROLLER_Y = 0.0;
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        let get_boss_intensity = CONFIG.read().options.boss_difficulty.unwrap_or(10.0);
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                            SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                            WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
                            WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
                            WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            ModelModule::set_scale(boss_boma, 1.15);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_TIME, true);
                        }
                        else {
                            ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_PLAYABLE_MASTERHAND),0,0,false,false);
                            SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            ModelModule::set_scale(module_accessor,0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                        }
                    }

                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && !STOP
                    && !CONFIG.read().options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                    }
                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && STOP
                    && !CONFIG.read().options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                        let x = 0.0;
                        let y = 0.0;
                        let z = 0.0;
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &module_pos);
                    }

                    // Respawn in case of Squad Strike or Specific Circumstances

                    if sv_information::is_ready_go() && !ItemModule::is_have_item(module_accessor, 0) && ModelModule::scale(module_accessor) != 0.0001 && FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32)))
                    || smash::app::smashball::is_training_mode()
                    || CONFIG.read().options.boss_respawn.unwrap_or(false)
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH && FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);

                        EXISTS_PUBLIC = true;
                        CONTROLLABLE = true;
                        DEAD = false;
                        RESULT_SPAWNED = false;
                        STOP = false;
                        FRESH_CONTROL = false;
                        JUMP_START = false;
                        CONTROLLER_X = 0.0;
                        CONTROLLER_Y = 0.0;
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        let get_boss_intensity = CONFIG.read().options.boss_difficulty.unwrap_or(10.0);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                        SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                        WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                        WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
                        WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
                        WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        ModelModule::set_scale(boss_boma, 1.15);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_WAIT_CHASE, true);

                        let x = PostureModule::pos_x(module_accessor);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(module_accessor);
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(boss_boma, &module_pos);
                    }

                    if sv_information::is_ready_go() && !ItemModule::is_have_item(module_accessor, 0) && ModelModule::scale(module_accessor) != 0.0001 && FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false
                    || smash::app::smashball::is_training_mode()
                    || CONFIG.read().options.boss_respawn.unwrap_or(false)
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH && FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);

                        EXISTS_PUBLIC = true;
                        CONTROLLABLE = true;
                        DEAD = false;
                        RESULT_SPAWNED = false;
                        STOP = false;
                        FRESH_CONTROL = false;
                        JUMP_START = false;
                        CONTROLLER_X = 0.0;
                        CONTROLLER_Y = 0.0;
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_PLAYABLE_MASTERHAND),0,0,false,false);
                        SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        ModelModule::set_scale(module_accessor,0.0001);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);


                        let x = PostureModule::pos_x(module_accessor);
                        let y = PostureModule::pos_y(module_accessor);
                        let z = PostureModule::pos_z(module_accessor);
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(boss_boma, &module_pos);
                        CONTROLLABLE = true;
                    }

                    if sv_information::is_ready_go() == false {
                        if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                            FighterManager::set_cursor_whole(fighter_manager,false);
                            ArticleModule::set_visibility_whole(module_accessor, *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP, false, smash::app::ArticleOperationTarget(0));
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_WAIT, true);
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                            let sub_hp = 999.0 - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            DamageModule::add_damage(module_accessor, sub_hp, 0);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        }
                        if CONTROLLABLE && ENTRY_ID != 0 {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        // println!("RATE: {}", MotionModule::rate(boss_boma));
                        JostleModule::set_status(module_accessor, false);
                    }

                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma, i, false) {
                            AttackModule::set_target_category(boss_boma, i, *COLLISION_CATEGORY_MASK_ALL as u32);
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
                            MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if DEAD == false {
                        if ModelModule::scale(module_accessor) == 0.0001 {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            if FighterUtil::is_hp_mode(module_accessor) == true {
                                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD
                                || StatusModule::status_kind(module_accessor) == 79 {
                                    if DEAD == false {
                                        CONTROLLABLE = false;
                                        EXISTS_PUBLIC = false;
                                        DEAD = true;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                    }
                                }
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        JostleModule::set_status(module_accessor, false);
                        let hp = CONFIG.read().options.wol_master_hand_hp.unwrap_or(400.0);
                        if DamageModule::damage(module_accessor, 0) >= hp && FighterUtil::is_hp_mode(module_accessor) == false {
                            if DEAD == false {
                                DEAD = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        if DEAD == false {
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_MASTERHAND_STATUS_KIND_WAIT_FEINT,true);
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 {
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

                        // SETS POWER

                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false {
                            if StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START
                            && StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING
                            && StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU
                            && StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                                AttackModule::set_power_mul(boss_boma, 0.5);
                                AttackModule::set_power_mul_2nd(boss_boma, 1.0);
                                AttackModule::set_power_mul_3rd(boss_boma, 1.0);
                                AttackModule::set_power_mul_4th(boss_boma, 1.0);
                                AttackModule::set_power_mul_5th(boss_boma, 1.0);
                            }
                            else {
                                AttackModule::set_power_mul(boss_boma, 0.02);
                                AttackModule::set_power_mul_2nd(boss_boma, 0.2);
                                AttackModule::set_power_mul_3rd(boss_boma, 0.2);
                                AttackModule::set_power_mul_4th(boss_boma, 0.2);
                                AttackModule::set_power_mul_5th(boss_boma, 0.2);
                            }
                        }

                        if FighterManager::is_result_mode(fighter_manager) == true {
                            if RESULT_SPAWNED == false {
                                EXISTS_PUBLIC = false;
                                RESULT_SPAWNED = true;
                                ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_MASTERHAND), 0, 0, false, false);
                                SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
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

                        if DEAD == true {
                            if sv_information::is_ready_go() == true {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_STANDBY {
                                    if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_DEAD {
                                        if StatusModule::status_kind(boss_boma) != *FIGHTER_STATUS_KIND_DEAD {
                                            if StatusModule::status_kind(boss_boma) != *FIGHTER_STATUS_KIND_STANDBY {
                                                SlowModule::clear_whole(boss_boma);
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if sv_information::is_ready_go() == true {
                            if DEAD == true {
                                if StatusModule::status_kind(boss_boma) != *ITEM_STATUS_KIND_DEAD {
                                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                    if STOP == false && CONFIG.read().options.boss_respawn.unwrap_or(false) {
                                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                                        STOP = true;
                                    }
                                    if STOP == false && !CONFIG.read().options.boss_respawn.unwrap_or(false) {
                                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0
                                        && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                            SoundModule::stop_se(module_accessor, Hash40::new("death"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("dead"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_damage_reaction"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_dead_frame"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_reaction"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_slow_frame"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_slow_mag"), 0);
                                        }
                                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0
                                        && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                            SoundModule::stop_se(module_accessor, Hash40::new("death"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("dead"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_damage_reaction"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_dead_frame"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_reaction"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_slow_frame"), 0);
                                            SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_knockout_slow_mag"), 0);
                                            STOP = true;
                                        }
                                    }
                                }
                            }
                        }

                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                            if StatusModule::status_kind(boss_boma) == *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END,true);
                            }
                        }

                        if ENTRY_ID == 0 && sv_information::is_ready_go() == true && !JUMP_START {
                            JUMP_START = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_WAIT,true);
                        }

                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false && sv_information::is_ready_go() == true && ENTRY_ID != 0 {
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                    if CONTROLLER_X < 0.0 && CONTROLLER_X > 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                    if CONTROLLER_Y < 0.0 && CONTROLLER_Y > 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f{x: CONTROLLER_X, y: CONTROLLER_Y, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN {
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                    if CONTROLLER_X < 0.0 && CONTROLLER_X > 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                    if CONTROLLER_Y < 0.0 && CONTROLLER_Y > 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f{x: CONTROLLER_X, y: CONTROLLER_Y, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_HIPPATAKU {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBI_BEAM {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_IRON_BALL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_CHAKRAM_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_DRILL_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_HOMING {
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                    if CONTROLLER_X < 0.0 && CONTROLLER_X > 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                    if CONTROLLER_Y < 0.0 && CONTROLLER_Y > 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f{x: CONTROLLER_X, y: CONTROLLER_Y, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == false {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU, true);
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TURN {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_END {
                                if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }

                            if FRESH_CONTROL && !DEAD {
                                FRESH_CONTROL = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                MotionModule::change_motion(boss_boma, smash::phx::Hash40::new("wait"), 0.0, 1.0, false, 0.0, false, false);
                            }

                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT
                            || StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_WAIT
                            || MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                                if !CONTROLLABLE {
                                    CONTROLLABLE = true;
                                    FRESH_CONTROL = true;
                                }
                            }
                            
                            if StatusModule::status_kind(boss_boma) == *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_MOVE {
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f{x: x, y: y, z: z};
                                let boss_pos_2 = Vector3f{x: x, y: y + 20.0, z: z};
                                PostureModule::set_pos(module_accessor, &boss_pos_2);
                                PostureModule::set_pos(boss_boma, &boss_pos);
                            }

                            if StatusModule::status_kind(boss_boma) != *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN && CONTROLLABLE && !DEAD {
                                //Boss Control Stick Movement
                                // X Controllable
                                if CONTROLLER_X < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X >= 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_X <= 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X > 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && CONTROLLER_X != 0.0 && ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    CONTROLLER_X += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_x(module_accessor) == 0.0 {
                                    if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                    if CONTROLLER_X < 0.0 && CONTROLLER_X > 0.06 {
                                        CONTROLLER_X = 0.0;
                                    }
                                }
                                if CONTROLLER_X > 0.0 && ControlModule::get_stick_x(module_accessor) < 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_X < 0.0 && ControlModule::get_stick_x(module_accessor) > 0.0 {
                                    CONTROLLER_X += (ControlModule::get_stick_x(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                // Y Controllable
                                if CONTROLLER_Y < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y >= 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL && CONTROLLER_Y <= 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y > 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y -= CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && CONTROLLER_Y != 0.0 && ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    CONTROLLER_Y += CONTROL_SPEED_MUL_2;
                                }
                                if ControlModule::get_stick_y(module_accessor) == 0.0 {
                                    if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                    if CONTROLLER_Y < 0.0 && CONTROLLER_Y > 0.06 {
                                        CONTROLLER_Y = 0.0;
                                    }
                                }
                                if CONTROLLER_Y > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }
                                if CONTROLLER_Y < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                    CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                                }

                                let pos = Vector3f{x: CONTROLLER_X, y: CONTROLLER_Y, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);

                                if PostureModule::lr(boss_boma) == 1.0 { // right
                                    if ControlModule::get_stick_x(module_accessor) < -0.95 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                if PostureModule::lr(boss_boma) == -1.0 { // left
                                    if ControlModule::get_stick_x(module_accessor) > 0.95 {
                                        CONTROLLABLE = false;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_TURN, true);
                                    }
                                }
                                // Boss Moves
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    CONTROLLABLE = false;
                                    MotionModule::set_rate(boss_boma, 1.2);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.2);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIDEPPOU_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBIPACCHIN_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                    CONTROLLABLE = false;
                                    MotionModule::set_rate(boss_boma, 2.5);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.5);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_YUBI_BEAM, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                    CONTROLLABLE = false;
                                    MotionModule::set_rate(boss_boma, 1.3);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 2.0);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_IRON_BALL_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                    CONTROLLABLE = false;
                                    MotionModule::set_rate(boss_boma, 1.3);
                                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.3);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_CHAKRAM_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_PAA_TSUBUSHI_HOLD, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_DRILL_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                    CONTROLLABLE = false;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_PLAYABLE_MASTERHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
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
    Agent::new("mario")
    .on_line(Main, once_per_fighter_frame)
    .install();
}
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
use smashline::{Agent, Main};
use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::galeem;

static mut CONTROLLABLE : bool = true;
static mut IS_ANGRY : bool = false;
static mut ENTRY_ID : usize = 0;
static mut RANDOM_ATTACK : i32 = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
pub static mut FIGHTER_NAME: [u64;9] = [0;9];
static mut STOP : bool = false;
static mut EXISTS_PUBLIC : bool = false;
static mut CONTROLLER_X: f32 = 0.0;
static mut CONTROLLER_Y: f32 = 0.0;
static mut CONTROL_SPEED_MUL: f32 = 1.25;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;
static mut HIDDEN_CPU : [u32; 8] = [0; 8];

use crate::config::{Config, load_config};

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
        while get_category(owner_module_accessor) != *BATTLE_OBJECT_CATEGORY_FIGHTER { //Keep checking the owner of the boma we're working with until we've hit a boma that belongs to a fighter
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
            let text = skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as u64;
            let game_version: String = CONFIG.read().options.game_version.clone().unwrap_or_else(|| "13.0.4".to_string());
            let offset_value =
            if game_version == "13.0.4" {
                0x52c4758
            } else if game_version == "13.0.3" {
                0x52c5758
            } else if game_version == "13.0.2" {
                0x52c3758
            } else {
                0x52c4758
            };
            let name_base = text + offset_value;
            FIGHTER_NAME[get_player_number(&mut *fighter.module_accessor)] = hash40(&read_tag(name_base + 0x260 * get_player_number(&mut *fighter.module_accessor) as u64 + 0x8e));
            if FIGHTER_NAME[get_player_number(module_accessor)] == hash40("DHARKON")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("ダーズ")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("SUMBRA")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("DHARS")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("LÚGUBRA")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("TENEBER")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("达兹")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("다즈")
            || FIGHTER_NAME[get_player_number(module_accessor)] == hash40("МУРАК") {
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 || !ItemModule::is_have_item(module_accessor, 0) {
                        ItemModule::remove_all(module_accessor);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DARZCENTIPEDE), 0, 0, false, false);
                        SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        ModelModule::set_scale(boss_boma, 0.05);
                        MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        PostureModule::set_rot(module_accessor,&Vector3f{x: -180.0, y: 90.0, z: 0.0},0);
                        ModelModule::set_joint_rotate(module_accessor, smash::phx::Hash40::new("root") , &mut Vector3f{x: 90.0, y: 50.0, z: 0.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                        PostureModule::set_pos(module_accessor, &Vector3f{x: PostureModule::pos_x(module_accessor), y: 7.25, z: PostureModule::pos_z(module_accessor) + 3.0 });
                    }
                }
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        DEAD = false;
                        CONTROLLABLE = true;
                        JUMP_START = false;
                        IS_ANGRY = false;
                        STOP = false;
                        CONTROLLER_X = 0.0;
                        CONTROLLER_Y = 0.0;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if smash::app::smashball::is_training_mode() == false {
                            if ModelModule::scale(module_accessor) != 0.0001 && ModelModule::scale(module_accessor) != 0.0002 {
                                let mut cfg = CONFIG.write();
                                *cfg = load_config();
                                ModelModule::set_scale(module_accessor, 0.0002);
                                ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DRACULA2), 0, 0, false, false);
                                SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                                HIDDEN_CPU[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                                let hidden_cpu_boma = sv_battle_object::module_accessor(HIDDEN_CPU[entry_id(module_accessor)]);
                                ModelModule::set_scale(hidden_cpu_boma, 0.0001);
                            }
                            if MotionModule::frame(module_accessor) >= 5.0 && ModelModule::scale(module_accessor) != 0.0001 {
                                EXISTS_PUBLIC = true;
                                RESULT_SPAWNED = false;
                                let get_boss_intensity = CONFIG.read().options.boss_difficulty.unwrap_or(10.0);
                                ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                                ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DARZ), 0, 0, false, false);
                                SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                                BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                                WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                                ModelModule::set_scale(module_accessor, 0.0001);
                                if galeem::check_status() {
                                    // MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("entry2"),0.0,1.0,false,0.0,false,false);
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                                }
                                else {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                                }
                                WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                                WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                                WorkModule::set_int(boss_boma, *ITEM_VARIATION_DARZ_KIILA, *ITEM_INSTANCE_WORK_INT_VARIATION);
                            }
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        let hidden_cpu_boma = sv_battle_object::module_accessor(HIDDEN_CPU[entry_id(module_accessor)]);
                        DamageModule::set_damage_lock(hidden_cpu_boma, true);
                        JostleModule::set_status(hidden_cpu_boma, false);
                        WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                        WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                        WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                        WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        if StatusModule::status_kind(hidden_cpu_boma) != *ITEM_STATUS_KIND_NONE {
                            StatusModule::change_status_request_from_script(hidden_cpu_boma, *ITEM_STATUS_KIND_NONE, true);
                        }
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        let x = PostureModule::pos_x(boss_boma);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(boss_boma);
                        let boss_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(hidden_cpu_boma, &boss_pos);
                    }

                    // Respawn in case of Squad Strike or Specific Circumstances

                    if sv_information::is_ready_go()
                    && !ItemModule::is_have_item(module_accessor, 0)
                    && ModelModule::scale(module_accessor) != 0.0001
                    || smash::app::smashball::is_training_mode()
                    || CONFIG.read().options.boss_respawn.unwrap_or(false)
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);
                        DEAD = false;
                        CONTROLLABLE = true;
                        JUMP_START = false;
                        IS_ANGRY = false;
                        STOP = false;
                        CONTROLLER_X = 0.0;
                        CONTROLLER_Y = 0.0;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DRACULA2), 0, 0, false, false);
                        SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                        HIDDEN_CPU[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let hidden_cpu_boma = sv_battle_object::module_accessor(HIDDEN_CPU[entry_id(module_accessor)]);
                        ModelModule::set_scale(hidden_cpu_boma, 0.0001);
                        EXISTS_PUBLIC = true;
                        RESULT_SPAWNED = false;
                        let get_boss_intensity = CONFIG.read().options.boss_difficulty.unwrap_or(10.0);
                        ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                        ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DARZ), 0, 0, false, false);
                        SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                        BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                        WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TELEPORT, true);
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        WorkModule::set_int(boss_boma, *ITEM_VARIATION_DARZ_KIILA, *ITEM_INSTANCE_WORK_INT_VARIATION);

                        let x = PostureModule::pos_x(module_accessor);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(module_accessor);
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(boss_boma, &module_pos);
                        CONTROLLABLE = false;
                    }

                    if sv_information::is_ready_go() {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                            let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                            PostureModule::set_rot(boss_boma,&vec3,0);
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                            let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                            PostureModule::set_rot(boss_boma,&vec3,0);
                        }
                    }

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_DOWN_LOOP {
                                let stunned = !CONFIG.read().options.full_stun_duration.unwrap_or(false);
                                if stunned {
                                    StatusModule::change_status_request_from_script(boss_boma,*ITEM_DARZ_STATUS_KIND_DOWN_END,true);
                                }
                                CONTROLLABLE = false;
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
                            MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if sv_information::is_ready_go() == true && smash::app::smashball::is_training_mode() == true || CONFIG.read().options.boss_respawn.unwrap_or(false) {
                        if ModelModule::scale(module_accessor) != 0.0002 && ModelModule::scale(module_accessor) != 0.0001 {
                            DEAD = false;
                            CONTROLLABLE = true;
                            JUMP_START = false;
                            IS_ANGRY = false;
                            STOP = false;
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            ModelModule::set_scale(module_accessor, 0.0002);
                        }
                        if ModelModule::scale(module_accessor) == 0.0002 {
                            RESULT_SPAWNED = false;
                            ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DARZ), 0, 0, false, false);
                            SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                            BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            let get_boss_intensity = CONFIG.read().options.boss_difficulty.unwrap_or(10.0);
                            WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                            WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            WorkModule::set_int(boss_boma, *ITEM_VARIATION_DARZ_KIILA, *ITEM_INSTANCE_WORK_INT_VARIATION);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                            ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
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
                        if CONTROLLABLE {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                        }
                        JostleModule::set_status(module_accessor, false);
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY || MotionModule::motion_kind(boss_boma) == smash::hash40("entry2") {
                            MotionModule::set_rate(boss_boma, 7.0);
                        }
                    }

                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            // SET POS AND STOPS OUT OF BOUNDS
                            if ModelModule::scale(module_accessor) == 0.0001 {
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                if FighterUtil::is_hp_mode(module_accessor) == true {
                                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD
                                    || StatusModule::status_kind(module_accessor) == 79 {
                                        if DEAD == false {
                                            CONTROLLABLE = false;
                                            DEAD = true;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                        }
                                    }
                                }
                                if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                    let x = PostureModule::pos_x(boss_boma);
                                    let y = PostureModule::pos_y(boss_boma);
                                    let z = PostureModule::pos_z(boss_boma);
                                    let boss_pos = Vector3f{x: x, y: y, z: z};
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
                    }
                    
                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if RESULT_SPAWNED == false {
                            EXISTS_PUBLIC = false;
                            RESULT_SPAWNED = true;
                            // ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DARZ), 0, 0, false, false);
                            // SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                            // BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            // let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            // StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START,true);
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

                    if sv_information::is_ready_go() == true {
                        // DAMAGE MODULES
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                        HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                        HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);
                        for i in 0..10 {
                            if AttackModule::is_attack(boss_boma, i, false) {
                                AttackModule::set_target_category(boss_boma, i, *COLLISION_CATEGORY_MASK_ALL as u32);
                            }
                        }
                        if sv_information::is_ready_go() == true {
                            if FighterUtil::is_hp_mode(module_accessor) == false {
                                let hp = CONFIG.read().options.dharkon_hp.unwrap_or(400.0);
                                if DamageModule::damage(module_accessor, 0) >= hp {
                                    if DEAD == false {
                                        CONTROLLABLE = false;
                                        DEAD = true;
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                    }
                                }
                            }
                        }

                        // DEATH CHECK

                        if sv_information::is_ready_go() == true {
                            if DEAD == true {
                                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                if STOP == false && smash::app::smashball::is_training_mode() == false {
                                    if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0
                                    && StatusModule::status_kind(module_accessor) != *ITEM_STATUS_KIND_STANDBY {
                                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                                        STOP = true;
                                    }
                                    if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0
                                    && StatusModule::status_kind(module_accessor) != *ITEM_STATUS_KIND_STANDBY {
                                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                                        STOP = true;
                                    }
                                }
                                if STOP == true && smash::app::smashball::is_training_mode() == false {
                                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                                    }
                                }
                            }
                        }
    
                        if DEAD == true {
                            if sv_information::is_ready_go() == true {
                                if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD || MotionModule::motion_kind(boss_boma) == smash::hash40("dead") {
                                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_STANDBY {
                                        if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) {
                                            EXISTS_PUBLIC = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY, true);
                                        }
                                    }
                                }
                            }
                        }

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
                                MotionModule::set_rate(boss_boma, 1.0);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_MANAGER_WAIT, true);
                            }
                        }

                        // BUILT IN BOSS AI
                        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
                            if DEAD == false {
                                if CONTROLLABLE == true {
                                    if MotionModule::frame(fighter.module_accessor) >= smash::app::sv_math::rand(hash40("fighter"), 59) as f32 {
                                        RANDOM_ATTACK = smash::app::sv_math::rand(hash40("fighter"), 12);
                                        if RANDOM_ATTACK == 0 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_CROSS_BOMB, true);
                                        }
                                        if RANDOM_ATTACK == 1 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TELEPORT, true);
                                        }
                                        if RANDOM_ATTACK == 2 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TEAR_UP_START, true);
                                        }
                                        if RANDOM_ATTACK == 3 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_PIERCE_START, true);
                                        }
                                        if RANDOM_ATTACK == 4 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_CENTIPEDE_START, true);
                                        }
                                        if RANDOM_ATTACK == 5 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_START, true);
                                        }
                                        if RANDOM_ATTACK == 6 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TEAR_UP_START, true);
                                        }
                                        if RANDOM_ATTACK == 7 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_DARK_PILLAR_START, true);
                                        }
                                        if RANDOM_ATTACK == 8 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_GATLING_START, true);
                                        }
                                        if RANDOM_ATTACK == 9 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_CHASE_HAMMER, true);
                                        }
                                        if RANDOM_ATTACK == 10 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TORRENT, true);
                                        }
                                        if RANDOM_ATTACK == 11 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_MANAGER_VANISH, true);
                                        }
                                        if RANDOM_ATTACK == 12 {
                                            CONTROLLABLE = false;
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER, true);
                                        }
                                    }
                                }
                            }
                        }
                        let rage_hp = CONFIG.read().options.dharkon_rage_hp.unwrap_or(220.0);
                        if DamageModule::damage(module_accessor, 0) >= rage_hp && !DEAD {
                            if IS_ANGRY == false {
                                CONTROLLABLE = false;
                                IS_ANGRY = true;
                                DamageModule::add_damage(module_accessor, 1.1, 0);
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_DARZ_STATUS_KIND_CHANGE_ANGRY,true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                            CONTROLLABLE = true;
                            MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WARP {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_DOWN_START {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_MANAGER_WAIT {
                            CONTROLLABLE = true;
                            MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_MANAGER_VANISH {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == 63 && !CONTROLLABLE {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TELEPORT, true);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER_WAIT {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_DOWN_LOOP {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_DOWN_END {
                            CONTROLLABLE = false;
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_LOST && !DEAD {
                            CONTROLLABLE = true;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TELEPORT, true);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_TEAR_UP_ANGER {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                CONTROLLABLE = true;
                            }
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_TEAR_UP {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                CONTROLLABLE = true;
                            }
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_LOOP {
                            CONTROLLABLE = false;
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_END {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_DARK_PILLAR_END {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_GATLING_LOOP {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_GATLING_HOLD_LOOP {
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
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_CHASE_HAMMER {
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
                        if StatusModule::status_kind(boss_boma) == 68 {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_DARZ_STATUS_KIND_TEAR_UP {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 10.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        // println!("{}", StatusModule::status_kind(boss_boma));
                        if CONTROLLABLE == true && FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == false && !DEAD {
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
                            if CONTROLLER_X > 0.0 && CONTROLLER_X < 0.06 {
                                CONTROLLER_X = 0.0;
                            }
                            if CONTROLLER_X < 0.0 && CONTROLLER_X > 0.06 {
                                CONTROLLER_X = 0.0;
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
                            if CONTROLLER_Y > 0.0 && CONTROLLER_Y < 0.06 {
                                CONTROLLER_Y = 0.0;
                            }
                            if CONTROLLER_Y < 0.0 && CONTROLLER_Y > 0.06 {
                                CONTROLLER_Y = 0.0;
                            }
                            if CONTROLLER_Y > 0.0 && ControlModule::get_stick_y(module_accessor) < 0.0 {
                                CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }
                            if CONTROLLER_Y < 0.0 && ControlModule::get_stick_y(module_accessor) > 0.0 {
                                CONTROLLER_Y += (ControlModule::get_stick_y(module_accessor)  * CONTROL_SPEED_MUL) * CONTROL_SPEED_MUL_2;
                            }

                            let pos = Vector3f{x: CONTROLLER_X, y: CONTROLLER_Y, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        
                            //Boss Moves
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_CROSS_BOMB, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TELEPORT, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TEAR_UP_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_PIERCE_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_CENTIPEDE_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_SPACE_RUSH_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TEAR_UP_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_DARK_PILLAR_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_GATLING_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_CHASE_HAMMER, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_SUMMON_FIGHTER, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                CONTROLLABLE = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_DARZ_STATUS_KIND_TORRENT, true);
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
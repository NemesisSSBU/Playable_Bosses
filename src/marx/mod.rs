use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Vector3f;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::FighterUtil;
use smash::app::sv_information;
use smash::app::lua_bind;
use smash::phx::Hash40;
use crate::config::CONFIG;
use crate::selection;
use crate::boss_helpers;
use crate::boss_runtime::{self, BossCommonRuntime, CommonRuntimeSyncGuard};

static mut CONTROLLABLE : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;
static mut STOP : bool = false;
static mut EXISTS_PUBLIC : bool = false;
static mut BLACK_HOLE_END : bool = false;
static mut CONTROLLER_X: f32 = 0.0;
static mut CONTROLLER_Y: f32 = 0.0;
static mut CONTROL_SPEED_MUL: f32 = 2.0;
static mut CONTROL_SPEED_MUL_2: f32 = 0.05;

const MARX_FLOOR_CLEARANCE: f32 = 0.1;

extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
}

pub unsafe fn check_status() -> bool {
    EXISTS_PUBLIC || boss_runtime::any_exists_public(&raw const boss_runtime::MARX_RUNTIME)
}

#[inline(always)]
unsafe fn update_marx_item_collision(
    boss_boma: *mut smash::app::BattleObjectModuleAccessor,
) {
    let status = StatusModule::status_kind(boss_boma);
    let intangible = status == *ITEM_MARX_STATUS_KIND_MOVE_TELEPORT
        || status == *ITEM_MARX_STATUS_KIND_AVOID_TELEPORT
        || status == *ITEM_STATUS_KIND_WARP;
    let hit_status = if intangible {
        *HIT_STATUS_OFF
    } else {
        *HIT_STATUS_NORMAL
    };
    HitModule::set_whole(boss_boma, smash::app::HitStatus(hit_status), 0);
    JostleModule::set_status(boss_boma, !intangible);
}

#[inline(always)]
unsafe fn marx_should_clamp_floor(
    boss_boma: *mut smash::app::BattleObjectModuleAccessor,
) -> bool {
    if !CONTROLLABLE {
        return false;
    }
    MotionModule::motion_kind(boss_boma) != smash::hash40("wait_convulsion")
}

#[inline(always)]
unsafe fn load_marx_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    CONTROLLABLE = (*slot).controllable;
    STOP = (*slot).stop;
    DEAD = (*slot).dead;
    JUMP_START = (*slot).jump_start;
    RESULT_SPAWNED = (*slot).result_spawned;
    EXISTS_PUBLIC = (*slot).exists_public;
    CONTROLLER_X = (*slot).controller_x;
    CONTROLLER_Y = (*slot).controller_y;
}

#[inline(always)]
unsafe fn store_marx_runtime(slot: *mut BossCommonRuntime) {
    if slot.is_null() {
        return;
    }
    (*slot).controllable = CONTROLLABLE;
    (*slot).stop = STOP;
    (*slot).dead = DEAD;
    (*slot).jump_start = JUMP_START;
    (*slot).result_spawned = RESULT_SPAWNED;
    (*slot).exists_public = EXISTS_PUBLIC;
    (*slot).fresh_control = false;
    (*slot).controller_x = CONTROLLER_X;
    (*slot).controller_y = CONTROLLER_Y;
}

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            ENTRY_ID = boss_runtime::sanitize_entry_id(boss_helpers::entry_id(module_accessor));
            let _runtime_guard = CommonRuntimeSyncGuard::new(
                boss_runtime::slot_ptr(&raw mut boss_runtime::MARX_RUNTIME, ENTRY_ID),
                load_marx_runtime,
                store_marx_runtime,
            );
            let fighter_manager = boss_helpers::fighter_manager();
            
            let selected_via_slot = selection::is_selected_css_boss(module_accessor, *ITEM_KIND_MARX);
            if selected_via_slot {
                boss_helpers::clear_hidden_host_effects(module_accessor);
                if smash::app::stage::get_stage_id() == 0x139 {
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if ModelModule::scale(module_accessor) != 0.0001 || !ItemModule::is_have_item(module_accessor, 0) {
                        ItemModule::remove_all(module_accessor);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        let boss_boma = boss_helpers::acquire_boss_item(
                            module_accessor,
                            &raw mut BOSS_ID,
                            *ITEM_KIND_MARX,
                        );
                        ModelModule::set_scale(boss_boma, 0.05);
                        MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                    }
                    if ModelModule::scale(module_accessor) == 0.0001 {
                        MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("none"),0.0,1.0,false,0.0,false,false);
                        PostureModule::set_rot(module_accessor,&Vector3f{x: -180.0, y: 90.0, z: 0.0},0);
                        ModelModule::set_joint_rotate(module_accessor, smash::phx::Hash40::new("root") , &mut Vector3f{x: 90.0, y: 40.0, z: 0.0}, smash::app::MotionNodeRotateCompose{_address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8}, ModelModule::rotation_order(module_accessor));
                        PostureModule::set_pos(module_accessor, &Vector3f{x: PostureModule::pos_x(module_accessor), y: 2.0, z: PostureModule::pos_z(module_accessor)});
                    }
                }
                else if smash::app::stage::get_stage_id() != 0x13A {
                    if sv_information::is_ready_go() == false {
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            DEAD = false;
                            CONTROLLABLE = true;
                        }
                        JUMP_START = false;
                        STOP = false;
                        CONTROLLER_X = 0.0;
                        CONTROLLER_Y = 0.0;
                        BLACK_HOLE_END = false;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if ModelModule::scale(module_accessor) != 0.0001 {
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            let boss_boma = boss_helpers::acquire_boss_item(
                                module_accessor,
                                &raw mut BOSS_ID,
                                *ITEM_KIND_MARX,
                            );
                            
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                            WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                            WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
                            WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
                            WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                        }
                    }

                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && !STOP
                    && !CONFIG.options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                    }
                    if !smash::app::smashball::is_training_mode()
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && STOP
                    && !CONFIG.options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                        let x = 0.0;
                        let y = 0.0;
                        let z = 0.0;
                        let module_pos = Vector3f{x: x, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &module_pos);
                    }

                    // Respawn in case of Squad Strike or Specific Circumstances

                    if sv_information::is_ready_go() && !ItemModule::is_have_item(module_accessor, 0) && ModelModule::scale(module_accessor) == 0.0001
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                        if smash::app::smashball::is_training_mode() || CONFIG.options.boss_respawn.unwrap_or(false) {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_FALL, true);
                            DEAD = false;
                            CONTROLLABLE = true;
                            JUMP_START = false;
                            STOP = false;
                            CONTROLLER_X = 0.0;
                            CONTROLLER_Y = 0.0;
                            BLACK_HOLE_END = false;
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            EXISTS_PUBLIC = true;
                            RESULT_SPAWNED = false;
                            let boss_boma = boss_helpers::acquire_boss_item(
                                module_accessor,
                                &raw mut BOSS_ID,
                                *ITEM_KIND_MARX,
                            );
                            
                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                            WorkModule::set_float(boss_boma, get_boss_intensity, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                            WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                            WorkModule::on_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_ANGRY);
                            WorkModule::set_int(boss_boma, *ITEM_TRAIT_FLAG_BOSS, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
                            WorkModule::set_float(boss_boma, 9999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            ModelModule::set_scale(module_accessor, 0.0001);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_MOVE_TELEPORT, true);

                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) {
                                if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                    let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                    PostureModule::set_rot(boss_boma,&vec3,0);
                                }
                                if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                    let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                    PostureModule::set_rot(boss_boma,&vec3,0);
                                }
                            }

                            let x = PostureModule::pos_x(module_accessor);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(module_accessor);
                            let module_pos = Vector3f{x: x, y: y, z: z};
                            PostureModule::set_pos(boss_boma, &module_pos);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                CONTROLLABLE = false;
                            }
                        }
                    }

                    if sv_information::is_ready_go() {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                            let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                            PostureModule::set_rot(boss_boma,&vec3,0);
                        }
                        if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                            let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                            PostureModule::set_rot(boss_boma,&vec3,0);
                        }
                    }

                    // Flags and new damage stuff

                    if sv_information::is_ready_go() == true {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
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

                    //STUBS AI

                    if sv_information::is_ready_go() && !DEAD {
                        if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                            if CONTROLLABLE {
                                if MotionModule::motion_kind(boss_boma) != smash::hash40("wait") {
                                    if StatusModule::status_kind(boss_boma) != ITEM_STATUS_KIND_NONE {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_NONE, true);
                                        VisibilityModule::set_whole(module_accessor, true);
                                        MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                    }
                                }
                            }
                            if !CONTROLLABLE {
                                if MotionModule::motion_kind(boss_boma) == smash::hash40("wait")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_up_loop")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_up_end")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_down_start")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_down_loop")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_up_start")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_down_end")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_right_start")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_right_loop")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_right_end")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_left_end")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_left_loop")
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("move_left_start") {
                                    CONTROLLABLE = true;
                                }
                            }
                        }
                    }

                    if ModelModule::scale(module_accessor) == 0.0001 {
                        let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {
                            MotionModule::set_rate(boss_boma, 7.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 7.0);
                        }
                    }

                    // FIXES SPAWN

                    if sv_information::is_ready_go() == true {
                        if CONTROLLABLE == true {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
                                CONTROLLABLE = false;
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_MOVE_STRAIGHT, true);
                            }
                        }
                    }
                    
                    if DEAD == false {
                        if sv_information::is_ready_go() == true {
                            if JUMP_START == false {
                                JUMP_START = true;
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                                MotionModule::set_rate(boss_boma, 1.0);
                                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                                if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                                    MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                                }
                                if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                    let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                    PostureModule::set_rot(boss_boma,&vec3,0);
                                }
                                if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                    let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                    PostureModule::set_rot(boss_boma,&vec3,0);
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
                            MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("fall"),0.0,1.0,false,0.0,false,false);
                        }
                    }

                    if DEAD == false {
                        // SET POS AND STOPS OUT OF BOUNDS
                        if ModelModule::scale(module_accessor) == 0.0001 {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
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
                            let x = PostureModule::pos_x(boss_boma);
                            let y = PostureModule::pos_y(boss_boma);
                            let z = PostureModule::pos_z(boss_boma);
                            let boss_pos = Vector3f{x: x, y: y + 20.0, z: z};
                            if !CONTROLLABLE || boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == true {
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
                            if marx_should_clamp_floor(boss_boma) {
                                boss_helpers::clamp_flying_boss_floor(
                                    module_accessor,
                                    boss_boma,
                                    MARX_FLOOR_CLEARANCE,
                                );
                            }
                        }
                    }

                    //DAMAGE MODULES
                    
                    let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                    update_marx_item_collision(boss_boma);

                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma, i, false) {
                            AttackModule::set_target_category(boss_boma, i, *COLLISION_CATEGORY_MASK_ALL as u32);
                        }
                    }

                    if sv_information::is_ready_go() == true {
                        if FighterUtil::is_hp_mode(module_accessor) == false {
                            
                            let hp = CONFIG.options.marx_hp.unwrap_or(400.0);
                            if DamageModule::damage(module_accessor, 0) >= hp {
                                if DEAD == false {
                                    CONTROLLABLE = false;
                                    DEAD = true;
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_DEAD, true);
                                    CameraModule::reset_all(module_accessor);
                                    CameraModule::reset_all(boss_boma);
                                }
                            }
                        }
                    }

                    //DEATH CHECK

                    if sv_information::is_ready_go() == true {
                        if DEAD == true {
                            if STOP == false && CONFIG.options.boss_respawn.unwrap_or(false) && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY, true);
                            }
                            if MotionModule::frame(boss_boma) > 210.0 {
                                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[boss_helpers::entry_id(module_accessor)]);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_STANDBY,true);
                                HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                                ItemModule::remove_all(module_accessor);
                                if STOP == false && CONFIG.options.boss_respawn.unwrap_or(false) {
                                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                                    STOP = true;
                                }
                                if STOP == false && !CONFIG.options.boss_respawn.unwrap_or(false) {
                                    boss_helpers::request_hidden_host_stock_drain(
                                        module_accessor,
                                        fighter_manager,
                                        ENTRY_ID,
                                        &raw mut STOP,
                                    );
                                }
                            }
                        }
                    }

                    if DEAD == true {
                        if sv_information::is_ready_go() == true {
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {
                                if MotionModule::frame(boss_boma) == 0.0 {
                                    smash_script::macros::CAM_ZOOM_IN_arg5(fighter, 0.0, 0.0, 5.0, 0.0, 0.0);
                                    smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_dead"),true,false);
                                    smash_script::macros::EFFECT(fighter, Hash40::new("sys_bg_criticalhit"), Hash40::new("top"), 0,7,0,0,0,0,1,0,0,0,0,0,0,false);
                                    smash_script::macros::EFFECT(fighter, Hash40::new("sys_bg_boss_finishhit"), Hash40::new("top"), 0,7,0,0,0,0,1,0,0,0,0,0,0,false);
                                }
                                if MotionModule::frame(boss_boma) == 0.5 {
                                    SlowModule::set_whole(module_accessor, 100, 0);
                                }
                                if MotionModule::frame(boss_boma) == 1.0 {
                                    SlowModule::clear_whole(module_accessor);
                                    SlowModule::set_whole(module_accessor, 10, 0);
                                    CameraModule::reset_all(module_accessor);
                                }
                                if MotionModule::frame(boss_boma) >= 1.1 {
                                    CameraModule::reset_all(module_accessor);
                                }
                                if MotionModule::frame(boss_boma) >= 5.0 {
                                    CameraModule::reset_all(module_accessor);
                                    smash_script::macros::CAM_ZOOM_OUT(fighter);
                                    smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_bg_criticalhit"),true,false);
                                    smash_script::macros::EFFECT_OFF_KIND(fighter,Hash40::new("sys_bg_boss_finishhit"),true,false);
                                    SlowModule::clear_whole(module_accessor);
                                }
                            }
                        }
                    }

                    let fighter_manager = boss_helpers::fighter_manager();
                    if FighterManager::is_result_mode(fighter_manager) == true {
                        if RESULT_SPAWNED == false {
                            EXISTS_PUBLIC = false;
                            RESULT_SPAWNED = true;
                            let boss_boma = boss_helpers::acquire_boss_item(
                                module_accessor,
                                &raw mut BOSS_ID,
                                *ITEM_KIND_MARX,
                            );
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START,true);
                        }
                        boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
                    }

                    if boss_helpers::is_operation_cpu_entry(fighter_manager, ENTRY_ID) == false {
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("wait")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("wait_convulsion")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_up_start")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_up_loop")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_up_end")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_down_start")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_down_loop")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_down_end")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_right_start")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_right_loop")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_right_end")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_left_start")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_left_loop")
                        || MotionModule::motion_kind(boss_boma) == smash::hash40("move_left_end") {
                            ControlModule::stop_rumble(boss_boma, true);
                            CONTROLLABLE = true;
                        }
                        if MotionModule::motion_kind(boss_boma) == smash::hash40("move_up_loop") | smash::hash40("move_up_end") | smash::hash40("move_down_start") | smash::hash40("move_down_loop") |  smash::hash40("move_up_start") | smash::hash40("move_down_end") | smash::hash40("move_right_start") | smash::hash40("move_right_loop") | smash::hash40("move_right_end") | smash::hash40("move_left_end") | smash::hash40("move_left_loop") | smash::hash40("move_left_start") {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_MOVE_STRAIGHT {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TRANS_PHASE {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_MOVE_TELEPORT && !DEAD  {
                            CONTROLLABLE = false;
                            if MotionModule::frame(boss_boma) == 22.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_PLANT_START {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 8.0 {
                                // CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WARP {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {
                            CONTROLLABLE = true;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_AVOID_TELEPORT {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 8.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_BLACK_HOLE_START {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_BLACK_HOLE_LOOP {
                            CONTROLLABLE = false;
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_BLACK_HOLE_END {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 8.0 {
                                BLACK_HOLE_END = true;
                            }
                        }
                        if BLACK_HOLE_END && StatusModule::status_kind(boss_boma) == 72
                        || StatusModule::status_kind(boss_boma) == 65 {
                            BLACK_HOLE_END = false;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_MOVE_TELEPORT, true);
                            CONTROLLABLE = true;
                        }
                        if !BLACK_HOLE_END && StatusModule::status_kind(boss_boma) == 65 && CONTROLLABLE {
                            CONTROLLABLE = false;
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_MOVE_TELEPORT, true);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_FACET_EYE_LASER_END {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 8.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_FLY_OUT {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_FLY_OUT_HOMING {
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == true && !DEAD  {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_FLY_OUT_HOMING_END, true);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_CAPILLARY_END && !DEAD {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_FOLLOW_EYE_END && !DEAD  {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_ICE_BOMB_SHOOT {
                            if MotionModule::frame(boss_boma) >= MotionModule::end_frame(boss_boma) - 5.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_4_CUTTER {
                            MotionModule::set_rate(boss_boma, 1.0);
                            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_4_CUTTER {
                            if MotionModule::frame(boss_boma) == 68.0 {
                                CONTROLLABLE = true;
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_ICE_BOMB_LOOP {
                            WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_TARGET_FOUND);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_X);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Y);
                            WorkModule::set_float(boss_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_TARGET_POS_Z);
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 3.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 3.0, y: 0.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 3.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 3.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_MARX_STATUS_KIND_ATTACK_THICK_LASER_LOOP {
                            //Boss Control Stick Movement
                            if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        
                            if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                                PostureModule::add_pos(boss_boma, &pos);
                            }
                        }
                        if CONTROLLABLE == true {
                            if DEAD == false {
                                // Stick Movement Controlling Rotation
                                if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                    let vec3 = Vector3f{x: CONTROLLER_Y/2.0 * - 15.0, y: 90.0 - CONTROLLER_X/2.0 * 22.0, z: 0.0};
                                    PostureModule::set_rot(boss_boma,&vec3,0);
                                }
                                if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                    let vec3 = Vector3f{x: CONTROLLER_Y/2.0 * - 15.0, y: -90.0 + CONTROLLER_X/2.0 * 22.0, z: 0.0};
                                    PostureModule::set_rot(boss_boma,&vec3,0);
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
                            
                                //Boss Moves
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_PLANT_START, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_4_CUTTER, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_MOVE_TELEPORT, true);
                                }
                                if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_ICE_BOMB_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_CAPILLARY_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_THICK_LASER_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_FACET_EYE_LASER_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_BLACK_HOLE_READY, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_FOLLOW_EYE_START, true);
                                }
                                if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                    CONTROLLABLE = false;
                                    CONTROLLER_X = 0.0;
                                    CONTROLLER_Y = 0.0;
                                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 { // left
                                        let vec3 = Vector3f{x: 0.0, y: 90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 { // right
                                        let vec3 = Vector3f{x: 0.0, y: -90.0, z: 0.0};
                                        PostureModule::set_rot(boss_boma,&vec3,0);
                                    }
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_MARX_STATUS_KIND_ATTACK_FLY_OUT_HOMING, true);
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
}

pub unsafe fn frame(fighter: &mut L2CFighterCommon) {
    once_per_fighter_frame(fighter);
}

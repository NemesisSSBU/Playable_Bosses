use crate::{config::CONFIG, dharkon, MAX_PLAYERS, SELECTED_UI_CHARA};
use skyline::nn::ro::LookupSymbol;
use smash::app::lua_bind;
use smash::app::lua_bind::*;
use smash::app::sv_battle_object;
use smash::app::sv_information;
use smash::app::BattleObjectModuleAccessor;
use smash::app::FighterUtil;
use smash::app::ItemKind;
use smash::hash40;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Hash40;
use smash::phx::Vector3f;
use smashline::{Agent, Main};
use std::u32;

#[derive(Clone, Copy)]
struct GaleemEntryState {
    controllable: bool,
    is_angry: bool,
    dead: bool,
    jump_start: bool,
    result_spawned: bool,
    stop: bool,
    exists_public: bool,
    controller_x: f32,
    controller_y: f32,
    random_attack: i32,
}

impl GaleemEntryState {
    const fn new() -> Self {
        Self {
            controllable: true,
            is_angry: false,
            dead: false,
            jump_start: false,
            result_spawned: false,
            stop: false,
            exists_public: false,
            controller_x: 0.0,
            controller_y: 0.0,
            random_attack: 0,
        }
    }
}

static mut ENTRY_STATE: [GaleemEntryState; MAX_PLAYERS] = [GaleemEntryState::new(); MAX_PLAYERS];
static mut BOSS_ID: [u32; MAX_PLAYERS] = [0; MAX_PLAYERS];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut HIDDEN_CPU: [u32; MAX_PLAYERS] = [0; MAX_PLAYERS];

const CONTROL_SPEED_MUL: f32 = 1.25;
const CONTROL_SPEED_MUL_2: f32 = 0.05;

#[inline(always)]
unsafe fn entry_state(entry: usize) -> &'static mut GaleemEntryState {
    &mut ENTRY_STATE[entry]
}

#[inline(always)]
unsafe fn fighter_entry(module_accessor: *mut BattleObjectModuleAccessor) -> usize {
    WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize
}

#[inline(always)]
unsafe fn boss_accessor(entry: usize) -> *mut BattleObjectModuleAccessor {
    sv_battle_object::module_accessor(BOSS_ID[entry])
}

#[inline(always)]
unsafe fn hidden_cpu_accessor(entry: usize) -> *mut BattleObjectModuleAccessor {
    sv_battle_object::module_accessor(HIDDEN_CPU[entry])
}

#[inline(always)]
unsafe fn stop_sound_list(module_accessor: *mut BattleObjectModuleAccessor, sounds: &[&str]) {
    for sound in sounds {
        SoundModule::stop_se(module_accessor, Hash40::new(sound), 0);
    }
}

extern "C" {
    #[link_name = "\u{1}_ZN3app17sv_camera_manager10dead_rangeEP9lua_State"]
    pub fn dead_range(lua_state: u64) -> smash::phx::Vector4f;
}

pub unsafe fn check_status() -> bool {
    for index in 0..MAX_PLAYERS {
        if ENTRY_STATE[index].exists_public {
            return true;
        }
    }
    false
}

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_MARIO {
            let entry = fighter_entry(module_accessor);
            let state = entry_state(entry);
            LookupSymbol(
                &raw mut FIGHTER_MANAGER,
                "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
                    .as_bytes()
                    .as_ptr(),
            );
            let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);

            let selected_hash = SELECTED_UI_CHARA[entry];
            if selected_hash != hash40("ui_chara_kiila") {
                *state = GaleemEntryState::new();
                BOSS_ID[entry] = 0;
                HIDDEN_CPU[entry] = 0;
                return;
            }

            let stage_id = smash::app::stage::get_stage_id();
            let ready_go = sv_information::is_ready_go();
            let training_mode = smash::app::smashball::is_training_mode();
            if stage_id == 0x139 {
                let lua_state = fighter.lua_state_agent;
                let module_accessor =
                    smash::app::sv_system::battle_object_module_accessor(lua_state);
                if ModelModule::scale(module_accessor) != 0.0001
                    || !ItemModule::is_have_item(module_accessor, 0)
                {
                    ItemModule::remove_all(module_accessor);
                    ItemModule::have_item(
                        module_accessor,
                        ItemKind(*ITEM_KIND_KIILACORE),
                        0,
                        0,
                        false,
                        false,
                    );
                    SoundModule::stop_se(
                        module_accessor,
                        smash::phx::Hash40::new("se_item_item_get"),
                        0,
                    );
                    BOSS_ID[entry] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                    ModelModule::set_scale(module_accessor, 0.0001);
                    let boss_boma = boss_accessor(entry);
                    ModelModule::set_scale(boss_boma, 0.05);
                    MotionModule::change_motion(
                        boss_boma,
                        smash::phx::Hash40::new("wait"),
                        0.0,
                        1.0,
                        false,
                        0.0,
                        false,
                        false,
                    );
                }
                if ModelModule::scale(module_accessor) == 0.0001 {
                    MotionModule::change_motion(
                        module_accessor,
                        smash::phx::Hash40::new("none"),
                        0.0,
                        1.0,
                        false,
                        0.0,
                        false,
                        false,
                    );
                    PostureModule::set_rot(
                        module_accessor,
                        &Vector3f {
                            x: -180.0,
                            y: 90.0,
                            z: 0.0,
                        },
                        0,
                    );
                    ModelModule::set_joint_rotate(
                        module_accessor,
                        smash::phx::Hash40::new("root"),
                        &mut Vector3f {
                            x: 90.0,
                            y: 50.0,
                            z: 0.0,
                        },
                        smash::app::MotionNodeRotateCompose {
                            _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
                        },
                        ModelModule::rotation_order(module_accessor),
                    );
                    PostureModule::set_pos(
                        module_accessor,
                        &Vector3f {
                            x: PostureModule::pos_x(module_accessor),
                            y: 7.25,
                            z: PostureModule::pos_z(module_accessor) + 3.0,
                        },
                    );
                }
            } else if stage_id != 0x13A {
                if !ready_go {
                    state.dead = false;
                    state.controllable = true;
                    state.jump_start = false;
                    state.is_angry = false;
                    state.stop = false;
                    state.controller_x = 0.0;
                    state.controller_y = 0.0;
                    let lua_state = fighter.lua_state_agent;
                    let module_accessor =
                        smash::app::sv_system::battle_object_module_accessor(lua_state);
                    if !training_mode {
                        if ModelModule::scale(module_accessor) != 0.0001
                            && ModelModule::scale(module_accessor) != 0.0002
                        {
                            ModelModule::set_scale(module_accessor, 0.0002);
                            ItemModule::have_item(
                                module_accessor,
                                ItemKind(*ITEM_KIND_DRACULA2),
                                0,
                                0,
                                false,
                                false,
                            );
                            SoundModule::stop_se(
                                module_accessor,
                                smash::phx::Hash40::new("se_item_item_get"),
                                0,
                            );
                            HIDDEN_CPU[entry] =
                                ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let hidden_cpu_boma = hidden_cpu_accessor(entry);
                            ModelModule::set_scale(hidden_cpu_boma, 0.0001);
                        }
                        if MotionModule::frame(module_accessor) >= 5.0
                            && ModelModule::scale(module_accessor) != 0.0001
                        {
                            state.exists_public = true;
                            state.result_spawned = false;
                            ItemModule::throw_item(
                                fighter.module_accessor,
                                0.0,
                                0.0,
                                0.0,
                                0,
                                true,
                                0.0,
                            );
                            ItemModule::have_item(
                                module_accessor,
                                ItemKind(*ITEM_KIND_KIILA),
                                0,
                                0,
                                false,
                                false,
                            );
                            SoundModule::stop_se(
                                module_accessor,
                                smash::phx::Hash40::new("se_item_item_get"),
                                0,
                            );
                            BOSS_ID[entry] =
                                ItemModule::get_have_item_id(module_accessor, 0) as u32;
                            let boss_boma = boss_accessor(entry);

                            let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                            WorkModule::set_float(
                                boss_boma,
                                get_boss_intensity,
                                *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
                            );
                            WorkModule::set_float(
                                boss_boma,
                                1.0,
                                *ITEM_INSTANCE_WORK_FLOAT_STRENGTH,
                            );
                            ModelModule::set_scale(module_accessor, 0.0001);
                            if dharkon::check_status() {
                                // MotionModule::change_motion(boss_boma,smash::phx::Hash40::new("entry2"),0.0,1.0,false,0.0,false,false);
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_STATUS_KIND_FOR_BOSS_START,
                                    true,
                                );
                            } else {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_STATUS_KIND_FOR_BOSS_START,
                                    true,
                                );
                            }
                            WorkModule::set_float(
                                boss_boma,
                                999.0,
                                *ITEM_INSTANCE_WORK_FLOAT_HP_MAX,
                            );
                            WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                            WorkModule::set_int(
                                boss_boma,
                                *ITEM_VARIATION_KIILA_DARZ,
                                *ITEM_INSTANCE_WORK_INT_VARIATION,
                            );
                        }
                    }
                }

                if ready_go {
                    let hidden_cpu_boma = hidden_cpu_accessor(entry);
                    DamageModule::set_damage_lock(hidden_cpu_boma, true);
                    JostleModule::set_status(hidden_cpu_boma, false);
                    WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
                    WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                    WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                    WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                    if StatusModule::status_kind(hidden_cpu_boma) != *ITEM_STATUS_KIND_NONE {
                        StatusModule::change_status_request_from_script(
                            hidden_cpu_boma,
                            *ITEM_STATUS_KIND_NONE,
                            true,
                        );
                    }
                    let boss_boma = boss_accessor(entry);
                    let x = PostureModule::pos_x(boss_boma);
                    let y = PostureModule::pos_y(boss_boma);
                    let z = PostureModule::pos_z(boss_boma);
                    let boss_pos = Vector3f { x: x, y: y, z: z };
                    PostureModule::set_pos(hidden_cpu_boma, &boss_pos);
                }

                if (ready_go && training_mode) || CONFIG.options.boss_respawn.unwrap_or(false) {
                    if ModelModule::scale(module_accessor) != 0.0002
                        && ModelModule::scale(module_accessor) != 0.0001
                    {
                        state.dead = false;
                        state.controllable = true;
                        state.jump_start = false;
                        state.is_angry = false;
                        state.stop = false;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor =
                            smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ModelModule::set_scale(module_accessor, 0.0002);
                    }
                    if ModelModule::scale(module_accessor) == 0.0002 {
                        state.result_spawned = false;
                        ItemModule::have_item(
                            module_accessor,
                            ItemKind(*ITEM_KIND_KIILA),
                            0,
                            0,
                            false,
                            false,
                        );
                        SoundModule::stop_se(
                            module_accessor,
                            smash::phx::Hash40::new("se_item_item_get"),
                            0,
                        );
                        BOSS_ID[entry] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = boss_accessor(entry);

                        let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                        WorkModule::set_float(
                            boss_boma,
                            get_boss_intensity,
                            *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
                        );
                        WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                        WorkModule::set_int(
                            boss_boma,
                            *ITEM_TRAIT_FLAG_BOSS,
                            *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG,
                        );
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        WorkModule::set_int(
                            boss_boma,
                            *ITEM_VARIATION_KIILA_DARZ,
                            *ITEM_INSTANCE_WORK_INT_VARIATION,
                        );
                        ModelModule::set_scale(module_accessor, 0.0001);
                        StatusModule::change_status_request_from_script(
                            boss_boma,
                            *ITEM_STATUS_KIND_FOR_BOSS_START,
                            true,
                        );
                        ItemModule::throw_item(
                            fighter.module_accessor,
                            0.0,
                            0.0,
                            0.0,
                            0,
                            true,
                            0.0,
                        );
                    }
                }

                // Respawn in case of Squad Strike or Specific Circumstances

                if ready_go
                    && !ItemModule::is_have_item(module_accessor, 0)
                    && ModelModule::scale(module_accessor) == 0.0001
                    && StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH
                {
                    if training_mode || CONFIG.options.boss_respawn.unwrap_or(false) {
                        StatusModule::change_status_request_from_script(
                            module_accessor,
                            *FIGHTER_STATUS_KIND_FALL,
                            true,
                        );
                        state.dead = false;
                        state.controllable = true;
                        state.jump_start = false;
                        state.is_angry = false;
                        state.stop = false;
                        state.controller_x = 0.0;
                        state.controller_y = 0.0;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor =
                            smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ItemModule::have_item(
                            module_accessor,
                            ItemKind(*ITEM_KIND_DRACULA2),
                            0,
                            0,
                            false,
                            false,
                        );
                        SoundModule::stop_se(
                            module_accessor,
                            smash::phx::Hash40::new("se_item_item_get"),
                            0,
                        );
                        HIDDEN_CPU[entry] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let hidden_cpu_boma = hidden_cpu_accessor(entry);
                        ModelModule::set_scale(hidden_cpu_boma, 0.0001);
                        state.exists_public = true;
                        state.result_spawned = false;
                        ItemModule::throw_item(
                            fighter.module_accessor,
                            0.0,
                            0.0,
                            0.0,
                            0,
                            true,
                            0.0,
                        );
                        ItemModule::have_item(
                            module_accessor,
                            ItemKind(*ITEM_KIND_KIILA),
                            0,
                            0,
                            false,
                            false,
                        );
                        SoundModule::stop_se(
                            module_accessor,
                            smash::phx::Hash40::new("se_item_item_get"),
                            0,
                        );
                        BOSS_ID[entry] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        let boss_boma = boss_accessor(entry);

                        let get_boss_intensity = CONFIG.options.boss_difficulty.unwrap_or(10.0);
                        WorkModule::set_float(
                            boss_boma,
                            get_boss_intensity,
                            *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
                        );
                        WorkModule::set_float(boss_boma, 1.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
                        ModelModule::set_scale(module_accessor, 0.0001);
                        StatusModule::change_status_request_from_script(
                            boss_boma,
                            *ITEM_KIILA_STATUS_KIND_TELEPORT,
                            true,
                        );
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        WorkModule::set_int(
                            boss_boma,
                            *ITEM_VARIATION_KIILA_DARZ,
                            *ITEM_INSTANCE_WORK_INT_VARIATION,
                        );

                        let x = PostureModule::pos_x(module_accessor);
                        let y = PostureModule::pos_y(boss_boma);
                        let z = PostureModule::pos_z(module_accessor);
                        let module_pos = Vector3f { x: x, y: y, z: z };
                        PostureModule::set_pos(boss_boma, &module_pos);
                        state.controllable = false;
                    }
                }

                if ready_go {
                    let boss_boma = boss_accessor(entry);
                    if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                        // left
                        let vec3 = Vector3f {
                            x: 0.0,
                            y: 90.0,
                            z: 0.0,
                        };
                        PostureModule::set_rot(boss_boma, &vec3, 0);
                    }
                    if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                        // right
                        let vec3 = Vector3f {
                            x: 0.0,
                            y: -90.0,
                            z: 0.0,
                        };
                        PostureModule::set_rot(boss_boma, &vec3, 0);
                    }
                }

                // Flags and new damage stuff

                if ready_go {
                    let boss_boma = boss_accessor(entry);
                    if WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP) != 999.0 {
                        let sub_hp =
                            999.0 - WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_HP);
                        DamageModule::add_damage(module_accessor, sub_hp, 0);
                        WorkModule::set_float(boss_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
                    }
                    if state.controllable {
                        WorkModule::off_flag(
                            boss_boma,
                            *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK,
                        );
                        WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_BOSS_KEYOFF_BGM);
                        WorkModule::off_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT);
                    }
                    JostleModule::set_status(module_accessor, false);
                }

                if !ready_go {
                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ENTRY {
                        FighterManager::set_cursor_whole(fighter_manager, false);
                        ArticleModule::set_visibility_whole(
                            module_accessor,
                            *FIGHTER_MARIO_GENERATE_ARTICLE_PUMP,
                            false,
                            smash::app::ArticleOperationTarget(0),
                        );
                        StatusModule::change_status_request_from_script(
                            module_accessor,
                            *FIGHTER_STATUS_KIND_WAIT,
                            true,
                        );
                    }
                }

                // SET FIGHTER LOOP

                if ready_go {
                    if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LANDING,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_SPECIAL,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ITEM,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_SPECIAL,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_JUMP_AERIAL,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_TREAD_JUMP,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ITEM_THROW,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ATTACK,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_WALL_JUMP,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ESCAPE,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_CATCH,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_JUMP,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_GUARD,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_ATTACK,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_GROUND_ESCAPE,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_CLIFF,
                        );
                        WorkModule::enable_transition_term_forbid_group(
                            module_accessor,
                            *FIGHTER_STATUS_TRANSITION_GROUP_CHK_AIR_LASSO,
                        );
                        FighterManager::set_cursor_whole(fighter_manager, false);
                        fighter.set_situation(SITUATION_KIND_AIR.into());
                        GroundModule::set_correct(
                            module_accessor,
                            smash::app::GroundCorrectKind(*GROUND_CORRECT_KIND_AIR),
                        );
                        MotionModule::change_motion(
                            module_accessor,
                            smash::phx::Hash40::new("fall"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                    }
                }

                if !state.dead {
                    if ready_go {
                        let boss_boma = boss_accessor(entry);
                        if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_LOOP
                        {
                            let stunned = !CONFIG.options.full_stun_duration.unwrap_or(false);
                            if stunned {
                                StatusModule::change_status_request_from_script(
                                    boss_boma,
                                    *ITEM_KIILA_STATUS_KIND_DOWN_END,
                                    true,
                                );
                            }
                            state.controllable = false;
                        }
                    }
                }

                if !state.dead {
                    if ready_go {
                        // SET POS AND STOPS OUT OF BOUNDS
                        if ModelModule::scale(module_accessor) == 0.0001 {
                            let boss_boma = boss_accessor(entry);
                            if FighterUtil::is_hp_mode(module_accessor) {
                                if StatusModule::status_kind(module_accessor)
                                    == *FIGHTER_STATUS_KIND_DEAD
                                    || StatusModule::status_kind(module_accessor) == 79
                                {
                                    if !state.dead {
                                        state.controllable = false;
                                        state.dead = true;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_STATUS_KIND_DEAD,
                                            true,
                                        );
                                    }
                                }
                            }
                            if StatusModule::status_kind(module_accessor)
                                != *FIGHTER_STATUS_KIND_STANDBY
                            {
                                let x = PostureModule::pos_x(boss_boma);
                                let y = PostureModule::pos_y(boss_boma);
                                let z = PostureModule::pos_z(boss_boma);
                                let boss_pos = Vector3f { x: x, y: y, z: z };
                                if !state.controllable
                                    || FighterInformation::is_operation_cpu(
                                        FighterManager::get_fighter_information(
                                            fighter_manager,
                                            smash::app::FighterEntryID(entry as i32),
                                        ),
                                    )
                                {
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: x,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                    } else if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                        {
                                            let boss_y_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                    } else if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        if PostureModule::pos_y(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                        {
                                            let boss_y_pos_1 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                    } else if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: x,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: x,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        }
                                    } else {
                                        PostureModule::set_pos(module_accessor, &boss_pos);
                                    }
                                } else {
                                    if PostureModule::pos_y(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                            + 160.0
                                    {
                                        let boss_y_pos_2 = Vector3f {
                                            x: x,
                                            y: (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                    } else if PostureModule::pos_x(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                    {
                                        let boss_x_pos_1 = Vector3f {
                                            x: dead_range(fighter.lua_state_agent).x.abs() - 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                        if PostureModule::pos_y(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                        {
                                            let boss_y_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                    } else if PostureModule::pos_x(boss_boma)
                                        <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                            + 100.0
                                    {
                                        let boss_x_pos_2 = Vector3f {
                                            x: (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0,
                                            y: y,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                        PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        if PostureModule::pos_y(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                        {
                                            let boss_y_pos_1 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        }
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: y,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                    } else if PostureModule::pos_y(boss_boma)
                                        >= dead_range(fighter.lua_state_agent).y.abs() - 100.0
                                    {
                                        let boss_y_pos_1 = Vector3f {
                                            x: x,
                                            y: dead_range(fighter.lua_state_agent).y.abs() - 100.0,
                                            z: z,
                                        };
                                        PostureModule::set_pos(module_accessor, &boss_y_pos_1);
                                        PostureModule::set_pos(boss_boma, &boss_y_pos_1);
                                        if PostureModule::pos_y(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).y.abs() * -1.0)
                                                + 160.0
                                        {
                                            let boss_y_pos_2 = Vector3f {
                                                x: x,
                                                y: (dead_range(fighter.lua_state_agent).y.abs()
                                                    * -1.0)
                                                    + 160.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_y_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_y_pos_2);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            >= dead_range(fighter.lua_state_agent).x.abs() - 100.0
                                        {
                                            let boss_x_pos_1 = Vector3f {
                                                x: dead_range(fighter.lua_state_agent).x.abs()
                                                    - 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_1);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_1);
                                        }
                                        if PostureModule::pos_x(boss_boma)
                                            <= (dead_range(fighter.lua_state_agent).x.abs() * -1.0)
                                                + 100.0
                                        {
                                            let boss_x_pos_2 = Vector3f {
                                                x: (dead_range(fighter.lua_state_agent).x.abs()
                                                    * -1.0)
                                                    + 100.0,
                                                y: dead_range(fighter.lua_state_agent).y.abs()
                                                    - 100.0,
                                                z: z,
                                            };
                                            PostureModule::set_pos(module_accessor, &boss_x_pos_2);
                                            PostureModule::set_pos(boss_boma, &boss_x_pos_2);
                                        }
                                    } else {
                                        PostureModule::set_pos(module_accessor, &boss_pos);
                                    }
                                }
                            }
                        }
                    }
                }

                if FighterManager::is_result_mode(fighter_manager) {
                    if !state.result_spawned {
                        state.exists_public = false;
                        state.result_spawned = true;
                        // ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_KIILA), 0, 0, false, false);
                        // SoundModule::stop_se(module_accessor, smash::phx::Hash40::new("se_item_item_get"), 0);
                        // BOSS_ID[entry] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                        // let boss_boma = boss_accessor(entry);
                        // StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START,true);
                    }
                    stop_sound_list(
                        module_accessor,
                        &[
                            "se_common_swing_05",
                            "vc_mario_013",
                            "se_common_swing_09",
                            "se_common_punch_kick_swing_l",
                            "vc_mario_win02",
                            "se_mario_win2",
                            "vc_mario_014",
                            "vc_mario_win03",
                            "vc_mario_015",
                            "se_mario_jump01",
                            "se_mario_landing02",
                        ],
                    );
                }

                if ready_go {
                    // DAMAGE MODULES
                    let boss_boma = boss_accessor(entry);
                    HitModule::set_whole(
                        module_accessor,
                        smash::app::HitStatus(*HIT_STATUS_OFF),
                        0,
                    );
                    HitModule::set_whole(boss_boma, smash::app::HitStatus(*HIT_STATUS_NORMAL), 0);
                    for i in 0..10 {
                        if AttackModule::is_attack(boss_boma, i, false) {
                            AttackModule::set_target_category(
                                boss_boma,
                                i,
                                *COLLISION_CATEGORY_MASK_ALL as u32,
                            );
                        }
                    }
                    if ready_go {
                        if !FighterUtil::is_hp_mode(module_accessor) {
                            let hp = CONFIG.options.galeem_hp.unwrap_or(400.0);
                            if DamageModule::damage(module_accessor, 0) >= hp {
                                if !state.dead {
                                    state.controllable = false;
                                    state.dead = true;
                                    StatusModule::change_status_request_from_script(
                                        boss_boma,
                                        *ITEM_STATUS_KIND_DEAD,
                                        true,
                                    );
                                }
                            }
                        }
                    }

                    // DEATH CHECK

                    if ready_go {
                        if state.dead {
                            HitModule::set_whole(
                                module_accessor,
                                smash::app::HitStatus(*HIT_STATUS_OFF),
                                0,
                            );
                            let boss_boma = boss_accessor(entry);
                            HitModule::set_whole(
                                boss_boma,
                                smash::app::HitStatus(*HIT_STATUS_OFF),
                                0,
                            );
                            ItemModule::remove_all(module_accessor);
                            if !state.stop && !training_mode {
                                if FighterInformation::stock_count(
                                    FighterManager::get_fighter_information(
                                        fighter_manager,
                                        smash::app::FighterEntryID(entry as i32),
                                    ),
                                ) == 0
                                    && StatusModule::status_kind(module_accessor)
                                        != *ITEM_STATUS_KIND_STANDBY
                                {
                                    StatusModule::change_status_request_from_script(
                                        module_accessor,
                                        *FIGHTER_STATUS_KIND_STANDBY,
                                        true,
                                    );
                                    state.stop = true;
                                }
                                if FighterInformation::stock_count(
                                    FighterManager::get_fighter_information(
                                        fighter_manager,
                                        smash::app::FighterEntryID(entry as i32),
                                    ),
                                ) != 0
                                    && StatusModule::status_kind(module_accessor)
                                        != *ITEM_STATUS_KIND_STANDBY
                                {
                                    StatusModule::change_status_request_from_script(
                                        module_accessor,
                                        *FIGHTER_STATUS_KIND_STANDBY,
                                        true,
                                    );
                                    state.stop = true;
                                }
                            }
                            if state.stop && !training_mode {
                                if StatusModule::status_kind(module_accessor)
                                    == *FIGHTER_STATUS_KIND_REBIRTH
                                {
                                    StatusModule::change_status_request_from_script(
                                        module_accessor,
                                        *FIGHTER_STATUS_KIND_STANDBY,
                                        true,
                                    );
                                }
                            }
                        }
                    }

                    if state.dead {
                        if ready_go {
                            if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD
                                || MotionModule::motion_kind(boss_boma) == smash::hash40("dead")
                            {
                                if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_STANDBY
                                {
                                    if MotionModule::frame(boss_boma)
                                        >= MotionModule::end_frame(boss_boma)
                                    {
                                        state.exists_public = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_STATUS_KIND_STANDBY,
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // FIXES SPAWN

                    if !state.dead {
                        if !state.jump_start {
                            state.jump_start = true;
                            state.controllable = false;
                            if lua_bind::PostureModule::lr(boss_boma) == -1.0 {
                                // left
                                let vec3 = Vector3f {
                                    x: 0.0,
                                    y: 90.0,
                                    z: 0.0,
                                };
                                PostureModule::set_rot(boss_boma, &vec3, 0);
                            }
                            if lua_bind::PostureModule::lr(boss_boma) == 1.0 {
                                // right
                                let vec3 = Vector3f {
                                    x: 0.0,
                                    y: -90.0,
                                    z: 0.0,
                                };
                                PostureModule::set_rot(boss_boma, &vec3, 0);
                            }
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT,
                                true,
                            );
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                        state.controllable = true;
                        MotionModule::change_motion(
                            boss_boma,
                            smash::phx::Hash40::new("wait"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_MANAGER_WAIT
                    {
                        state.controllable = true;
                        MotionModule::change_motion(
                            boss_boma,
                            smash::phx::Hash40::new("wait"),
                            0.0,
                            1.0,
                            false,
                            0.0,
                            false,
                            false,
                        );
                    }
                    if MotionModule::motion_kind(boss_boma) == smash::hash40("wait") {
                        state.controllable = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_START {
                        state.controllable = false;
                    }
                    if StatusModule::status_kind(boss_boma)
                        == *ITEM_KIILA_STATUS_KIND_MANAGER_VANISH
                    {
                        state.controllable = true;
                    }
                    if StatusModule::status_kind(boss_boma)
                        == *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER_WAIT
                    {
                        state.controllable = true;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_LOOP {
                        state.controllable = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_DOWN_END {
                        state.controllable = false;
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 10.0
                        {
                            state.controllable = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == 63 && !state.controllable {
                        StatusModule::change_status_request_from_script(
                            boss_boma,
                            *ITEM_KIILA_STATUS_KIND_TELEPORT,
                            true,
                        );
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CRUSH_DOWN {
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 10.0
                        {
                            state.controllable = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WARP {
                        state.controllable = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR {
                        //Boss Control Stick Movement
                        // X Controllable
                        if state.controller_x
                            < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_x >= 0.0
                            && ControlModule::get_stick_x(module_accessor) > 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x
                            > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_x <= 0.0
                            && ControlModule::get_stick_x(module_accessor) < 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x > 0.0
                            && state.controller_x != 0.0
                            && ControlModule::get_stick_x(module_accessor) == 0.0
                        {
                            state.controller_x -= CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x < 0.0
                            && state.controller_x != 0.0
                            && ControlModule::get_stick_x(module_accessor) == 0.0
                        {
                            state.controller_x += CONTROL_SPEED_MUL_2;
                        }
                        if ControlModule::get_stick_x(module_accessor) == 0.0 {
                            if state.controller_x > 0.0 && state.controller_x < 0.06 {
                                state.controller_x = 0.0;
                            }
                            if state.controller_x < 0.0 && state.controller_x > 0.06 {
                                state.controller_x = 0.0;
                            }
                        }
                        if state.controller_x > 0.0
                            && ControlModule::get_stick_x(module_accessor) < 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x < 0.0
                            && ControlModule::get_stick_x(module_accessor) > 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }

                        // Y Controllable
                        if state.controller_y
                            < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_y >= 0.0
                            && ControlModule::get_stick_y(module_accessor) > 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y
                            > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_y <= 0.0
                            && ControlModule::get_stick_y(module_accessor) < 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y > 0.0
                            && state.controller_y != 0.0
                            && ControlModule::get_stick_y(module_accessor) == 0.0
                        {
                            state.controller_y -= CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y < 0.0
                            && state.controller_y != 0.0
                            && ControlModule::get_stick_y(module_accessor) == 0.0
                        {
                            state.controller_y += CONTROL_SPEED_MUL_2;
                        }
                        if ControlModule::get_stick_y(module_accessor) == 0.0 {
                            if state.controller_y > 0.0 && state.controller_y < 0.06 {
                                state.controller_y = 0.0;
                            }
                            if state.controller_y < 0.0 && state.controller_y > 0.06 {
                                state.controller_y = 0.0;
                            }
                        }
                        if state.controller_y > 0.0
                            && ControlModule::get_stick_y(module_accessor) < 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y < 0.0
                            && ControlModule::get_stick_y(module_accessor) > 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }

                        let pos = Vector3f {
                            x: state.controller_x,
                            y: state.controller_y,
                            z: 0.0,
                        };
                        PostureModule::add_pos(boss_boma, &pos);
                    }
                    if StatusModule::status_kind(boss_boma)
                        == *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_END
                    {
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 10.0
                        {
                            state.controllable = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY
                    {
                        state.controllable = false;
                    }
                    if StatusModule::status_kind(boss_boma) == 73 {
                        state.controllable = true;
                    }
                    // println!("{}", StatusModule::status_kind(boss_boma));

                    let rage_hp = CONFIG.options.galeem_rage_hp.unwrap_or(220.0);
                    if DamageModule::damage(module_accessor, 0) >= rage_hp && !state.dead {
                        if !state.is_angry && !state.dead {
                            state.controllable = false;
                            state.is_angry = true;
                            DamageModule::add_damage(module_accessor, 1.1, 0);
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_CHANGE_ANGRY,
                                true,
                            );
                        }
                    }

                    // BUILT IN BOSS AI
                    if FighterInformation::is_operation_cpu(
                        FighterManager::get_fighter_information(
                            fighter_manager,
                            smash::app::FighterEntryID(entry as i32),
                        ),
                    ) {
                        if !state.dead {
                            if state.controllable {
                                if MotionModule::frame(fighter.module_accessor)
                                    >= smash::app::sv_math::rand(hash40("fighter"), 59) as f32
                                {
                                    state.random_attack =
                                        smash::app::sv_math::rand(hash40("fighter"), 10);
                                    if state.random_attack == 0 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_CROSS_BOMB,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 1 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_TELEPORT,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 2 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 3 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 4 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_START,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 5 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 6 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_START,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 7 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_THREAT_START,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 8 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_TORRENT,
                                            true,
                                        );
                                    }
                                    if state.random_attack == 9 {
                                        state.controllable = false;
                                        StatusModule::change_status_request_from_script(
                                            boss_boma,
                                            *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER,
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    if StatusModule::status_kind(boss_boma)
                        == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START
                    {
                        //Boss Control Stick Movement
                        // X Controllable
                        if state.controller_x
                            < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_x >= 0.0
                            && ControlModule::get_stick_x(module_accessor) > 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x
                            > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_x <= 0.0
                            && ControlModule::get_stick_x(module_accessor) < 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x > 0.0
                            && state.controller_x != 0.0
                            && ControlModule::get_stick_x(module_accessor) == 0.0
                        {
                            state.controller_x -= CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x < 0.0
                            && state.controller_x != 0.0
                            && ControlModule::get_stick_x(module_accessor) == 0.0
                        {
                            state.controller_x += CONTROL_SPEED_MUL_2;
                        }
                        if ControlModule::get_stick_x(module_accessor) == 0.0 {
                            if state.controller_x > 0.0 && state.controller_x < 0.06 {
                                state.controller_x = 0.0;
                            }
                            if state.controller_x < 0.0 && state.controller_x > 0.06 {
                                state.controller_x = 0.0;
                            }
                        }
                        if state.controller_x > 0.0
                            && ControlModule::get_stick_x(module_accessor) < 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x < 0.0
                            && ControlModule::get_stick_x(module_accessor) > 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }

                        // Y Controllable
                        if state.controller_y
                            < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_y >= 0.0
                            && ControlModule::get_stick_y(module_accessor) > 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y
                            > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_y <= 0.0
                            && ControlModule::get_stick_y(module_accessor) < 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y > 0.0
                            && state.controller_y != 0.0
                            && ControlModule::get_stick_y(module_accessor) == 0.0
                        {
                            state.controller_y -= CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y < 0.0
                            && state.controller_y != 0.0
                            && ControlModule::get_stick_y(module_accessor) == 0.0
                        {
                            state.controller_y += CONTROL_SPEED_MUL_2;
                        }
                        if ControlModule::get_stick_y(module_accessor) == 0.0 {
                            if state.controller_y > 0.0 && state.controller_y < 0.06 {
                                state.controller_y = 0.0;
                            }
                            if state.controller_y < 0.0 && state.controller_y > 0.06 {
                                state.controller_y = 0.0;
                            }
                        }
                        if state.controller_y > 0.0
                            && ControlModule::get_stick_y(module_accessor) < 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y < 0.0
                            && ControlModule::get_stick_y(module_accessor) > 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }

                        let pos = Vector3f {
                            x: state.controller_x,
                            y: state.controller_y,
                            z: 0.0,
                        };
                        PostureModule::add_pos(boss_boma, &pos);
                    }
                    if StatusModule::status_kind(boss_boma)
                        == *ITEM_KIILA_STATUS_KIND_LASER_RUSH_END
                    {
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 10.0
                        {
                            state.controllable = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma)
                        == *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_END
                    {
                        if MotionModule::frame(boss_boma)
                            >= MotionModule::end_frame(boss_boma) - 10.0
                        {
                            state.controllable = true;
                        }
                    }
                    if state.controllable
                        && !FighterInformation::is_operation_cpu(
                            FighterManager::get_fighter_information(
                                fighter_manager,
                                smash::app::FighterEntryID(entry as i32),
                            ),
                        )
                        && !state.dead
                    {
                        //Boss Control Stick Movement
                        // X Controllable
                        if state.controller_x
                            < ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_x >= 0.0
                            && ControlModule::get_stick_x(module_accessor) > 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x
                            > ControlModule::get_stick_x(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_x <= 0.0
                            && ControlModule::get_stick_x(module_accessor) < 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x > 0.0
                            && state.controller_x != 0.0
                            && ControlModule::get_stick_x(module_accessor) == 0.0
                        {
                            state.controller_x -= CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x < 0.0
                            && state.controller_x != 0.0
                            && ControlModule::get_stick_x(module_accessor) == 0.0
                        {
                            state.controller_x += CONTROL_SPEED_MUL_2;
                        }
                        if ControlModule::get_stick_x(module_accessor) == 0.0 {
                            if state.controller_x > 0.0 && state.controller_x < 0.06 {
                                state.controller_x = 0.0;
                            }
                            if state.controller_x < 0.0 && state.controller_x > 0.06 {
                                state.controller_x = 0.0;
                            }
                        }
                        if state.controller_x > 0.0
                            && ControlModule::get_stick_x(module_accessor) < 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_x < 0.0
                            && ControlModule::get_stick_x(module_accessor) > 0.0
                        {
                            state.controller_x += (ControlModule::get_stick_x(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }

                        // Y Controllable
                        if state.controller_y
                            < ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_y >= 0.0
                            && ControlModule::get_stick_y(module_accessor) > 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y
                            > ControlModule::get_stick_y(module_accessor) * CONTROL_SPEED_MUL
                            && state.controller_y <= 0.0
                            && ControlModule::get_stick_y(module_accessor) < 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y > 0.0
                            && state.controller_y != 0.0
                            && ControlModule::get_stick_y(module_accessor) == 0.0
                        {
                            state.controller_y -= CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y < 0.0
                            && state.controller_y != 0.0
                            && ControlModule::get_stick_y(module_accessor) == 0.0
                        {
                            state.controller_y += CONTROL_SPEED_MUL_2;
                        }
                        if ControlModule::get_stick_y(module_accessor) == 0.0 {
                            if state.controller_y > 0.0 && state.controller_y < 0.06 {
                                state.controller_y = 0.0;
                            }
                            if state.controller_y < 0.0 && state.controller_y > 0.06 {
                                state.controller_y = 0.0;
                            }
                        }
                        if state.controller_y > 0.0
                            && ControlModule::get_stick_y(module_accessor) < 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }
                        if state.controller_y < 0.0
                            && ControlModule::get_stick_y(module_accessor) > 0.0
                        {
                            state.controller_y += (ControlModule::get_stick_y(module_accessor)
                                * CONTROL_SPEED_MUL)
                                * CONTROL_SPEED_MUL_2;
                        }

                        let pos = Vector3f {
                            x: state.controller_x,
                            y: state.controller_y,
                            z: 0.0,
                        };
                        PostureModule::add_pos(boss_boma, &pos);

                        //Boss Moves
                        if ControlModule::check_button_on(
                            module_accessor,
                            *CONTROL_PAD_BUTTON_SPECIAL,
                        ) {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_CROSS_BOMB,
                                true,
                            );
                        }
                        if ControlModule::check_button_on(
                            module_accessor,
                            *CONTROL_PAD_BUTTON_GUARD,
                        ) {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_TELEPORT,
                                true,
                            );
                        }
                        if ControlModule::check_button_on(
                            module_accessor,
                            *CONTROL_PAD_BUTTON_ATTACK,
                        ) {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START,
                                true,
                            );
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                            & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW
                            != 0
                        {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_CHASE_SPEAR,
                                true,
                            );
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                            & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI
                            != 0
                        {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_EXPLODE_SHOT_START,
                                true,
                            );
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                            & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S
                            != 0
                        {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_LASER_RUSH_START,
                                true,
                            );
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                            & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3
                            != 0
                        {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_ENERGY_SMART_BOMB_START,
                                true,
                            );
                        }
                        if ControlModule::get_command_flag_cat(fighter.module_accessor, 0)
                            & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3
                            != 0
                        {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_THREAT_START,
                                true,
                            );
                        }
                        if ControlModule::check_button_on(
                            module_accessor,
                            *CONTROL_PAD_BUTTON_APPEAL_HI,
                        ) {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_TORRENT,
                                true,
                            );
                        }
                        if ControlModule::check_button_on(
                            module_accessor,
                            *CONTROL_PAD_BUTTON_APPEAL_LW,
                        ) {
                            state.controllable = false;
                            state.controller_x = 0.0;
                            state.controller_y = 0.0;
                            StatusModule::change_status_request_from_script(
                                boss_boma,
                                *ITEM_KIILA_STATUS_KIND_SUMMON_FIGHTER,
                                true,
                            );
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

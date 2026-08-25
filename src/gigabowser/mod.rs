use crate::boss_helpers;
use crate::config::CONFIG;
use smash::app::lua_bind::*;
use smash::app::sv_information;
use smash::app::FighterUtil;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Vector3f;
use smashline::{Agent, Main};

/// Giga Bowser is built by the native amiibo viewer at full fighter size,
/// which overfills the preview frame. Half scale is the preview-only size.
const AMIIBO_PREVIEW_SCALE: f32 = 0.5;

/// `GIGA_BOWSER_NORMAL = true` means play as a regular hacked-in `koopag`
/// fighter: knockback, stocks, and no HP-threshold KO. Default false keeps
/// the Classic/boss-battle rules. Amiibo mapping must not override this.
#[inline(always)]
pub fn giga_bowser_uses_boss_battle_rules(giga_bowser_normal: bool) -> bool {
    !giga_bowser_normal
}

#[inline(always)]
fn uses_boss_battle_rules() -> bool {
    giga_bowser_uses_boss_battle_rules(CONFIG.options.giga_bowser_normal.unwrap_or(false))
}

#[inline]
fn should_request_boss_hp_death(
    training_mode: bool,
    hp_mode: bool,
    damage: f32,
    hp: f32,
    lifecycle_blocked: bool,
    stop: bool,
) -> bool {
    !training_mode && !hp_mode && damage >= hp && !lifecycle_blocked && !stop
}

#[inline]
const fn should_reset_respawn_lifecycle(respawn_enabled: bool, status_is_rebirth: bool) -> bool {
    respawn_enabled && status_is_rebirth
}

#[inline]
const fn should_latch_respawn_death(
    respawn_enabled: bool,
    dead: bool,
    stop: bool,
    status_is_dead: bool,
) -> bool {
    respawn_enabled && dead && !stop && status_is_dead
}

static mut DEAD: bool = false;
static mut STOP: bool = false;
static mut ENTRY_ID: usize = 0;
static mut DECREASING: bool = false;
static mut INITIAL_STOCK_COUNT: u64 = 0;

/// Clear only the dedicated Giga Bowser lifecycle state. Unlike the
/// item-backed bosses, Giga Bowser is not reset by the shared runtime table.
/// This is bookkeeping-only; native fighter teardown remains engine-owned.
pub unsafe fn reset_match_state(entry_id: usize) {
    let dead = DEAD;
    let stop = STOP;
    let decreasing = DECREASING;
    let initial_stock_count = INITIAL_STOCK_COUNT;
    if crate::debug::enabled() && (dead || stop || decreasing || initial_stock_count != 0) {
        crate::boss_log!(
            "[PB][GigaBowser][Reset] entry={} dead={} stop={} decreasing={} initial_stock_count={}",
            entry_id.min(7),
            dead,
            stop,
            decreasing,
            initial_stock_count
        );
    }
    ENTRY_ID = entry_id.min(7);
    DEAD = false;
    STOP = false;
    DECREASING = false;
    INITIAL_STOCK_COUNT = 0;
}

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);

        // Giga Bowser is the native-fighter Result camera control. Keep this
        // observation read-only and run it before the shared quarantine exits
        // the fighter callback in Result mode.
        if fighter_kind == *FIGHTER_KIND_KOOPAG {
            crate::result_camera::observe_native_fighter_result_reference(module_accessor);
        }

        // Amiibo Figure Player and Classic staff roll share the same preview
        // scale. Giga Bowser is a native fighter, so there is no presentation
        // item — half scale is the viewer/credits size. Battle stages keep
        // his real scale.
        let stage_id = smash::app::stage::get_stage_id();
        if fighter_kind == *FIGHTER_KIND_KOOPAG
            && (stage_id == boss_helpers::STAGE_ID_AMIIBO_PREVIEW
                || boss_helpers::is_classic_staffroll_stage(stage_id))
        {
            if ModelModule::scale(module_accessor) != AMIIBO_PREVIEW_SCALE {
                ModelModule::set_scale(module_accessor, AMIIBO_PREVIEW_SCALE);
            }
            return;
        }

        // Giga Bowser uses its own fighter agent instead of the Mario-host
        // dispatcher. Apply the same post-match/result quarantine before any
        // damage, stock, or rebirth logic can touch a native result object.
        if crate::should_quarantine_boss_frame(module_accessor) {
            crate::finish_boss_transition_cleanup(module_accessor);
            return;
        }

        crate::ai_diagnostics::log_native_fighter(module_accessor);
        ENTRY_ID =
            WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        if fighter_kind == *FIGHTER_KIND_KOOPAG {
            if !boss_helpers::is_boss_nonbattle_stage(smash::app::stage::get_stage_id()) {
                if !uses_boss_battle_rules() {
                    return;
                }
                let fighter_manager = boss_helpers::fighter_manager();
                if fighter_manager.is_null() {
                    return;
                }
                FighterManager::set_cursor_whole(fighter_manager, false);
                let training_mode = smash::app::smashball::is_training_mode();
                let respawn_enabled = CONFIG.options.boss_respawn.unwrap_or(false);
                let status = StatusModule::status_kind(module_accessor);
                if sv_information::is_ready_go() == false {
                    DEAD = false;
                    STOP = false;
                    DECREASING = false;
                    if FighterUtil::is_hp_mode(module_accessor) {
                        INITIAL_STOCK_COUNT = FighterInformation::stock_count(
                            FighterManager::get_fighter_information(
                                fighter_manager,
                                smash::app::FighterEntryID(ENTRY_ID as i32),
                            ),
                        );
                    }
                }
                if should_reset_respawn_lifecycle(
                    respawn_enabled,
                    status == *FIGHTER_STATUS_KIND_REBIRTH,
                ) {
                    DEAD = false;
                    STOP = false;
                    DECREASING = false;
                    if FighterUtil::is_hp_mode(module_accessor) {
                        INITIAL_STOCK_COUNT =
                            boss_helpers::stock_count_entry(fighter_manager, ENTRY_ID);
                    }
                }
                if sv_information::is_ready_go() {
                    DamageModule::set_reaction_mul(module_accessor, 0.0);
                    DamageModule::set_reaction_mul_2nd(module_accessor, 0.0);
                    DamageModule::set_reaction_mul_4th(module_accessor, 0.0);
                }

                let hp = CONFIG.options.giga_bowser_hp.unwrap_or(600.0);
                let lifecycle_blocked = status == *FIGHTER_STATUS_KIND_DEAD
                    || status == *FIGHTER_STATUS_KIND_REBIRTH
                    || status == *FIGHTER_STATUS_KIND_STANDBY;
                if should_request_boss_hp_death(
                    training_mode,
                    FighterUtil::is_hp_mode(module_accessor),
                    DamageModule::damage(module_accessor, 0),
                    hp,
                    lifecycle_blocked,
                    STOP,
                ) {
                    StatusModule::change_status_request_from_script(
                        module_accessor,
                        *FIGHTER_STATUS_KIND_DEAD,
                        true,
                    );
                }
                if !training_mode
                    && DamageModule::damage(module_accessor, 0) >= hp
                    && FighterUtil::is_hp_mode(module_accessor) == false
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY
                    && STOP
                    && !respawn_enabled
                {
                    let x = 0.0;
                    let y = 0.0;
                    let z = 0.0;
                    let module_pos = Vector3f { x: x, y: y, z: z };
                    PostureModule::set_pos(module_accessor, &module_pos);
                    StatusModule::change_status_request_from_script(
                        module_accessor,
                        *FIGHTER_STATUS_KIND_STANDBY,
                        true,
                    );
                }
                // DECREASING FOR STAMINA MODE
                if StatusModule::status_kind(module_accessor) == 470
                    || StatusModule::status_kind(module_accessor) == 181
                {
                    if FighterUtil::is_hp_mode(module_accessor) && !training_mode {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                            if DECREASING
                                && FighterInformation::stock_count(
                                    FighterManager::get_fighter_information(
                                        fighter_manager,
                                        smash::app::FighterEntryID(ENTRY_ID as i32),
                                    ),
                                ) == 0
                            {
                                StatusModule::change_status_request_from_script(
                                    module_accessor,
                                    *FIGHTER_STATUS_KIND_DEAD,
                                    true,
                                );
                                INITIAL_STOCK_COUNT = 0;
                                DECREASING = false;
                            }
                            if DECREASING
                                && FighterInformation::stock_count(
                                    FighterManager::get_fighter_information(
                                        fighter_manager,
                                        smash::app::FighterEntryID(ENTRY_ID as i32),
                                    ),
                                ) != 0
                            {
                                StatusModule::change_status_request_from_script(
                                    module_accessor,
                                    *FIGHTER_STATUS_KIND_DEAD,
                                    true,
                                );
                            }
                            if FighterInformation::stock_count(
                                FighterManager::get_fighter_information(
                                    fighter_manager,
                                    smash::app::FighterEntryID(ENTRY_ID as i32),
                                ),
                            ) < INITIAL_STOCK_COUNT
                            {
                                DECREASING = true;
                            }
                        }
                    }
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD
                    && !training_mode
                {
                    DEAD = true;
                }
                if !training_mode || respawn_enabled {
                    if DEAD == true {
                        if should_latch_respawn_death(
                            respawn_enabled,
                            DEAD,
                            STOP,
                            StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD,
                        ) {
                            STOP = true;
                        }
                        if STOP == false && !respawn_enabled {
                            if FighterInformation::stock_count(
                                FighterManager::get_fighter_information(
                                    fighter_manager,
                                    smash::app::FighterEntryID(ENTRY_ID as i32),
                                ),
                            ) != 0
                                && StatusModule::status_kind(module_accessor)
                                    != *FIGHTER_STATUS_KIND_DEAD
                            {
                                StatusModule::change_status_request_from_script(
                                    module_accessor,
                                    *FIGHTER_STATUS_KIND_DEAD,
                                    true,
                                );
                            }
                            if FighterInformation::stock_count(
                                FighterManager::get_fighter_information(
                                    fighter_manager,
                                    smash::app::FighterEntryID(ENTRY_ID as i32),
                                ),
                            ) == 0
                                && StatusModule::status_kind(module_accessor)
                                    != *FIGHTER_STATUS_KIND_DEAD
                            {
                                StatusModule::change_status_request_from_script(
                                    module_accessor,
                                    *FIGHTER_STATUS_KIND_DEAD,
                                    true,
                                );
                                STOP = true;
                            }
                        }
                        if STOP == true {
                            if StatusModule::status_kind(module_accessor)
                                == *FIGHTER_STATUS_KIND_REBIRTH
                                && !respawn_enabled
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
            }
        }
    }
}

pub fn install() {
    Agent::new("koopag")
        .on_line(Main, once_per_fighter_frame)
        .install();
}

#[cfg(test)]
mod tests {
    use super::{
        giga_bowser_uses_boss_battle_rules, should_latch_respawn_death,
        should_request_boss_hp_death, should_reset_respawn_lifecycle,
    };

    #[test]
    fn giga_bowser_normal_disables_boss_battle_rules() {
        assert!(
            giga_bowser_uses_boss_battle_rules(false),
            "default false is Classic/boss-battle mode"
        );
        assert!(
            !giga_bowser_uses_boss_battle_rules(true),
            "true is vanilla hacked-in koopag: knockback, stocks, ignore BOSS_RESPAWN"
        );
    }

    #[test]
    fn boss_respawn_does_not_disable_hp_threshold_death() {
        assert!(should_request_boss_hp_death(
            false, false, 600.0, 600.0, false, false
        ));
        assert!(!should_request_boss_hp_death(
            true, false, 600.0, 600.0, false, false
        ));
        assert!(!should_request_boss_hp_death(
            false, true, 600.0, 600.0, false, false
        ));
        assert!(!should_request_boss_hp_death(
            false, false, 600.0, 600.0, true, false
        ));
    }

    #[test]
    fn boss_respawn_lifecycle_resets_once_rebirth_begins() {
        assert!(should_latch_respawn_death(true, true, false, true));
        assert!(!should_latch_respawn_death(true, true, true, true));
        assert!(should_reset_respawn_lifecycle(true, true));
        assert!(!should_reset_respawn_lifecycle(false, true));
        assert!(!should_reset_respawn_lifecycle(true, false));
    }
}

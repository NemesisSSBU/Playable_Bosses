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
                if sv_information::is_ready_go() {
                    DamageModule::set_reaction_mul(module_accessor, 0.0);
                    DamageModule::set_reaction_mul_2nd(module_accessor, 0.0);
                    DamageModule::set_reaction_mul_4th(module_accessor, 0.0);
                }

                let hp = CONFIG.options.giga_bowser_hp.unwrap_or(600.0);
                if !smash::app::smashball::is_training_mode()
                    && DamageModule::damage(module_accessor, 0) >= hp
                    && FighterUtil::is_hp_mode(module_accessor) == false
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                    && !STOP
                    && !CONFIG.options.boss_respawn.unwrap_or(false)
                {
                    StatusModule::change_status_request_from_script(
                        module_accessor,
                        *FIGHTER_STATUS_KIND_DEAD,
                        true,
                    );
                }
                if !smash::app::smashball::is_training_mode()
                    && DamageModule::damage(module_accessor, 0) >= hp
                    && FighterUtil::is_hp_mode(module_accessor) == false
                    && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY
                    && STOP
                    && !CONFIG.options.boss_respawn.unwrap_or(false)
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
                    if FighterUtil::is_hp_mode(module_accessor)
                        && smash::app::smashball::is_training_mode() == false
                    {
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
                    && smash::app::smashball::is_training_mode() == false
                {
                    DEAD = true;
                }
                if smash::app::smashball::is_training_mode() == false
                    || CONFIG.options.boss_respawn.unwrap_or(false)
                {
                    if DEAD == true {
                        if STOP == false && CONFIG.options.boss_respawn.unwrap_or(false) {
                            StatusModule::change_status_request_from_script(
                                module_accessor,
                                *FIGHTER_STATUS_KIND_DEAD,
                                true,
                            );
                            STOP = true;
                        }
                        if STOP == false && !CONFIG.options.boss_respawn.unwrap_or(false) {
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
                                && !CONFIG.options.boss_respawn.unwrap_or(false)
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
    use super::giga_bowser_uses_boss_battle_rules;

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
}

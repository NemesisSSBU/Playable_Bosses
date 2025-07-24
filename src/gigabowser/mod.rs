use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::phx::Vector3f;
use skyline::nn::ro::LookupSymbol;
use smash::app::sv_information;
use smash::app::FighterUtil;
use smashline::{Agent, Main};
use once_cell::sync::Lazy;
use parking_lot::RwLock;

static mut DEAD : bool = false;
static mut STOP : bool = false;
static mut ENTRY_ID : usize = 0;
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DECREASING : bool = false;
static mut INITIAL_STOCK_COUNT : u64 = 0;

use crate::config::{Config, load_config};

pub static CONFIG: Lazy<RwLock<Config>> = Lazy::new(|| {
    RwLock::new(load_config())
});

extern "C" fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        LookupSymbol(
            &raw mut FIGHTER_MANAGER,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
            .as_bytes()
            .as_ptr(),
        );
        if fighter_kind == *FIGHTER_KIND_KOOPAG {
            if smash::app::stage::get_stage_id() != 0x139 {
                let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                FighterManager::set_cursor_whole(fighter_manager, false);
                if sv_information::is_ready_go() == false {
                    DEAD = false;
                    STOP = false;
                    DECREASING = false;
                    if FighterUtil::is_hp_mode(module_accessor) {
                        INITIAL_STOCK_COUNT = FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32)));
                    }
                }
                if sv_information::is_ready_go() {
                    DamageModule::set_reaction_mul(module_accessor, 0.0);
                    DamageModule::set_reaction_mul_2nd(module_accessor, 0.0);
                    DamageModule::set_reaction_mul_4th(module_accessor, 0.0);
                }
                
                let hp = CONFIG.read().options.giga_bowser_hp.unwrap_or(600.0);
                if !smash::app::smashball::is_training_mode()
                && DamageModule::damage(module_accessor, 0) >= hp && FighterUtil::is_hp_mode(module_accessor) == false
                && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD
                && !STOP
                && !CONFIG.read().options.boss_respawn.unwrap_or(false) {
                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                }
                if !smash::app::smashball::is_training_mode()
                && DamageModule::damage(module_accessor, 0) >= hp && FighterUtil::is_hp_mode(module_accessor) == false
                && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY
                && STOP {
                    let x = 0.0;
                    let y = 0.0;
                    let z = 0.0;
                    let module_pos = Vector3f{x: x, y: y, z: z};
                    PostureModule::set_pos(module_accessor, &module_pos);
                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                }
                // DECREASING FOR STAMINA MODE
                if StatusModule::status_kind(module_accessor) == 470 || StatusModule::status_kind(module_accessor) == 181 {
                    if FighterUtil::is_hp_mode(module_accessor) && smash::app::smashball::is_training_mode() == false {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                            if DECREASING && FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                INITIAL_STOCK_COUNT = 0;
                                DECREASING = false;
                            }
                            if DECREASING && FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                            }
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) < INITIAL_STOCK_COUNT {
                                DECREASING = true;
                            }
                        }
                    }
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD
                && smash::app::smashball::is_training_mode() == false {
                    DEAD = true;
                }
                if smash::app::smashball::is_training_mode() == false || CONFIG.read().options.boss_respawn.unwrap_or(false) {
                    if DEAD == true {
                        if STOP == false && CONFIG.read().options.boss_respawn.unwrap_or(false) {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                            STOP = true;
                        }
                        if STOP == false && !CONFIG.read().options.boss_respawn.unwrap_or(false) {
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0
                            && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                            }
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0
                            && StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                                STOP = true;
                            }
                        }
                        if STOP == true {
                            if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_REBIRTH {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
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
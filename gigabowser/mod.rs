use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use skyline::nn::ro::LookupSymbol;
use smash::app::sv_information;
use smash::app::FighterUtil;

static mut DEAD : bool = false;
static mut STOP : bool = false;
static mut ENTRY_ID : usize = 0;
pub static mut FIGHTER_MANAGER: usize = 0;
static mut DECREASING : bool = false;
static mut INITIAL_STOCK_COUNT : u64 = 0;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        LookupSymbol(
            &mut FIGHTER_MANAGER,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
            .as_bytes()
            .as_ptr(),
        );
        if fighter_kind == *FIGHTER_KIND_KOOPAG {
            if smash::app::stage::get_stage_id() == 0x139 {
                // if MotionModule::motion_kind(boss_boma) != smash::hash40("wait") {
                //     MotionModule::change_motion(module_accessor,smash::phx::Hash40::new("wait"),0.0,1.0,false,0.0,false,false);
                // }
            }
            else {
                let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
                MotionModule::set_rate(module_accessor, 1.0);
                smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.0);
                if sv_information::is_ready_go() == false {
                    DEAD = false;
                    STOP = false;
                    DECREASING = false;
                    if FighterUtil::is_hp_mode(module_accessor) == true {
                        INITIAL_STOCK_COUNT = FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32)));
                    }
                }
                if sv_information::is_ready_go() == true {
                    DamageModule::set_reaction_mul(module_accessor, 0.0);
                    if DamageModule::damage(module_accessor, 0) >= 499.0 && FighterUtil::is_hp_mode(module_accessor) == false {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                    }
                    if DECREASING && FighterUtil::is_hp_mode(module_accessor) == true {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                    }
                }
                // DECREASING FOR STAMINA MODE
                if FighterUtil::is_hp_mode(module_accessor) == true && sv_information::is_ready_go() == true {
                    if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) < INITIAL_STOCK_COUNT {
                        DECREASING = true;
                    }
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                    DEAD = true;
                }
                if sv_information::is_ready_go() == true {
                    if DEAD == true {
                        if STOP == false {
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                                StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
                            }
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
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
    acmd::add_custom_hooks!(once_per_fighter_frame);
}
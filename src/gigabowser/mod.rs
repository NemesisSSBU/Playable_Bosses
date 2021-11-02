use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use skyline::nn::ro::LookupSymbol;
use smash::app::sv_information;

static mut IS_BOSS_DEAD : bool = false;
static mut ENTRY_ID : usize = 0;
pub static mut FIGHTER_MANAGER: usize = 0;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        ENTRY_ID = WorkModule::get_int(module_accessor,*FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        LookupSymbol(
            &mut FIGHTER_MANAGER,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
            .as_bytes()
            .as_ptr(),
        );
        let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
            if fighter_kind == *FIGHTER_KIND_KOOPAG {
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_WAIT {
                    MotionModule::set_rate(module_accessor, 1.0);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.0);
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ATTACK {
                    MotionModule::set_rate(module_accessor, 1.9);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.9);
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ATTACK_AIR {
                    MotionModule::set_rate(module_accessor, 1.9);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.9);
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ATTACK_HI3 {
                    MotionModule::set_rate(module_accessor, 1.9);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.9);
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ATTACK_HI4 {
                    MotionModule::set_rate(module_accessor, 1.9);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.9);
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ATTACK_LW3 {
                    MotionModule::set_rate(module_accessor, 1.9);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.9);
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_ATTACK_LW4 {
                    MotionModule::set_rate(module_accessor, 1.9);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.9);
                }
                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_SPECIAL_S {
                    MotionModule::set_rate(module_accessor, 2.0);
                    smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 2.0);
                }
                if sv_information::is_ready_go() == true {
                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                        IS_BOSS_DEAD = true;
                    }
                }
                if IS_BOSS_DEAD == true {
                    if sv_information::is_ready_go() == true {
                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                            if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                            }
                        }
                    }
                }
                if sv_information::is_ready_go() == false {
                    IS_BOSS_DEAD = false;
                }
            }
        }
    }

pub fn install() {
    acmd::add_custom_hooks!(once_per_fighter_frame);
}

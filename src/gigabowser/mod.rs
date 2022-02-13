use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use skyline::nn::ro::LookupSymbol;
use smash::app::sv_information;

static mut DEAD : bool = false;
static mut ENTRY_ID : usize = 0;
pub static mut FIGHTER_MANAGER: usize = 0;

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
            let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
            MotionModule::set_rate(module_accessor, 1.0);
            smash::app::lua_bind::ItemMotionAnimcmdModuleImpl::set_fix_rate(module_accessor, 1.0);
            if sv_information::is_ready_go() == false {
                DEAD = false;
            }
            if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                DEAD = true;
            }
            // DEATH CHECK
            if DEAD == true {
                if sv_information::is_ready_go() == true {
                    if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD,true);
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
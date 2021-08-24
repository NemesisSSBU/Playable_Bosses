use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::sv_information;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind != *FIGHTER_KIND_MARIO {
            if sv_information::is_ready_go() == true {
                if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
                    StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
                }
            }
        }
    }
}

            

pub fn install() {
    acmd::add_custom_hooks!(once_per_fighter_frame);
}

use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::sv_information;
use smash_script::lua_args;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        //let name = smash::app::stage::get_stage_id();
        //println!("[Comp. Playable Bosses] StageID: {:?}", name);
        //if fighter_kind != *FIGHTER_KIND_MARIO {
        if fighter_kind == *FIGHTER_KIND_MARIO {
            if sv_information::is_ready_go() == true {
                smash_script::notify_event_msc_cmd!(fighter, smash::phx::Hash40::new_raw(0x149ac79c98));
            }
        //        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_DEAD {
        //            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_DEAD, true);
        //        }
        //    }
        }
    }
}

            

pub fn install() {
    acmd::add_custom_hooks!(once_per_fighter_frame);
}

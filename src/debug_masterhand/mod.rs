use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::BattleObjectModuleAccessor;
use smash::app::ItemKind;
use std::u32;
use smash::app::FighterUtil;
use smash::app::sv_information;

static mut CONTROLLABLE : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        if fighter_kind == *FIGHTER_KIND_LUIGI {
            pub unsafe fn entry_id(module_accessor: &mut BattleObjectModuleAccessor) -> usize {
                let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                return entry_id;
            }
            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
            if sv_information::is_ready_go() == true {
                let lua_state = fighter.lua_state_agent;
                let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                if ModelModule::scale(module_accessor) != 0.0001 {
                    CONTROLLABLE = true;
                    ItemModule::have_item(module_accessor, ItemKind(*ITEM_KIND_DARZ), 0, 0, false, false);
                    BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor, 0) as u32;
                    //let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                    ModelModule::set_scale(module_accessor, 0.0001);
                    //StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_FOR_BOSS_START, true);
                    if FighterUtil::is_hp_mode(module_accessor) == true {
                        DamageModule::add_damage(module_accessor, 1.1, 0);
                    }
                }
                if CONTROLLABLE == true {
                    if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                        CONTROLLABLE = false;
                        ItemModule::remove_all(module_accessor);
                    }
                }
            }
        }
    }
}

pub fn install() {
    acmd::add_custom_hooks!(once_per_fighter_frame);
}
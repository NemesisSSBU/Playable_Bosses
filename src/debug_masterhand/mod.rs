use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::FighterUtil;
use smash::app::sv_information;
use skyline::nn::ro::LookupSymbol;

static mut TELEPORTED : bool = false;
static mut TURNING : bool = false;
static mut CONTROLLABLE : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut MULTIPLE_BULLETS : usize = 0;
static mut DEAD : bool = false;
static mut JUMP_START : bool = false;
static mut RESULT_SPAWNED : bool = false;

let pos = Vector2f{x: ControlModule::get_stick_x(module_accessor)*2.0, y: ControlModule::get_stick_y(module_accessor)*2.0};
PostureModule::add_pos_2d(module_accessor,&pos);

pub fn install() {
    skyline::install_hooks!(
        mh_wait_time_main,
        mh_wait_time_sub,
        mh_wait_time_sub1
    );
}

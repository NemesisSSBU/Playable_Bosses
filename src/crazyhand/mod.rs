use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use smash::lua2cpp::L2CFighterCommon;
use smash::app::BattleObjectModuleAccessor;
use smash::phx::Vector3f;
use smash_script::macros::WHOLE_HIT;
use smash::app::ItemKind;
use smash::app::sv_battle_object;
use std::u32;
use smash::app::sv_information;
use skyline::nn::ro::LookupSymbol;

static mut SPAWN_BOSS : bool = true;
static mut TELEPORTED : bool = false;
static mut HAVE_ITEM : bool = false;
static mut CHARACTER_IS_TURNING : bool = false;
static mut ENTRANCE_ANIM : bool = false;
static mut STOP_CONTROL_LOOP : bool = true;
static mut ENTRY_ID : usize = 0;
static mut BOSS_ID : [u32; 8] = [0; 8];
pub static mut FIGHTER_MANAGER: usize = 0;
static mut IS_BOSS_DEAD : bool = false;

pub fn once_per_fighter_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        pub unsafe fn entry_id(module_accessor: &mut BattleObjectModuleAccessor) -> usize {
            let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
            return entry_id;
        }
        ENTRY_ID = WorkModule::get_int(module_accessor,*FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        LookupSymbol(
            &mut FIGHTER_MANAGER,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
            .as_bytes()
            .as_ptr(),
        );
        let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
        if FighterInformation::is_operation_cpu(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == true {
            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
            if fighter_kind == *FIGHTER_KIND_PICHU {
                if MotionModule::frame(fighter.module_accessor) >= 29.0 {
                    if sv_information::is_ready_go() == false {
                        HAVE_ITEM = false;
                        IS_BOSS_DEAD = false;
                        CHARACTER_IS_TURNING = false;
                        let lua_state = fighter.lua_state_agent;
                        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                        ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                        if SPAWN_BOSS == true {
                            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                            
                            ENTRANCE_ANIM = false;
                            ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_CRAZYHAND),0,0,false,false);
                                BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                            ModelModule::set_scale(module_accessor,0.0001);
                            HAVE_ITEM = true;
                            ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                            StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT, true);
                            StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                        }
                    }
                }
                DamageModule::set_damage_lock(boss_boma,true);
                WHOLE_HIT(fighter, *HIT_STATUS_XLU);
                HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);

                if StopModule::is_damage(boss_boma) {
                    if DamageModule::damage(module_accessor, 0) >= 360.0 {
                        if IS_BOSS_DEAD == false {
                            IS_BOSS_DEAD = true;
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                            StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                        }
                    }
                    if DamageModule::damage(module_accessor, 0) == -1.0 {
                        if IS_BOSS_DEAD == false {
                            IS_BOSS_DEAD = true;
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                            StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                        }
                    }
                    DamageModule::add_damage(module_accessor, 0.5, 0);
                }

                if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                    if IS_BOSS_DEAD == false {
                        IS_BOSS_DEAD = true;
                        StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                    }
                }

                if IS_BOSS_DEAD == true {
                    if sv_information::is_ready_go() == true {
                        if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                            StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                            DamageModule::add_damage(module_accessor, 360.0, 0);
                            STOP_CONTROL_LOOP = false;
                        }
                    }
                }

                if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                    if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                        StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                    }
                }

                if MotionModule::frame(fighter.module_accessor) >= 30.0 {
                    if sv_information::is_ready_go() == true {
                        HAVE_ITEM = true;
                    }
                }

                if sv_information::is_ready_go() == true {
                    if HAVE_ITEM == true {
                        if IS_BOSS_DEAD == false {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                    }
                }

                if sv_information::is_ready_go() == false {
                    if HAVE_ITEM == true {
                        if IS_BOSS_DEAD == false {
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_WAIT,true);
                            MotionModule::set_rate(boss_boma, 0.0);
                        }
                    }
                }

                if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_RUN {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TURN {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_JUMP {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_JUMP_AIR {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_FALL {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_EXIT {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_PASS_FLOOR {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT {

                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_START {
                    
                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TERM {
                    
                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_LANDING {
                    
                }
                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_INITIALIZE {

                }
                else {
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                    }
                    else {
                        //MotionModule::set_rate(boss_boma, 1.8);
                    }
                }
            }}
            else {
            let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
            if SPAWN_BOSS == true {
                if fighter_kind == *FIGHTER_KIND_PICHU {
                    let x = PostureModule::pos_x(boss_boma);
                    let y = PostureModule::pos_y(boss_boma);
                    let z = PostureModule::pos_z(boss_boma);
                    let boss_pos = Vector3f{x: x, y: y, z: z};
                    if PostureModule::pos_y(boss_boma) >= 220.0 {
                        let boss_y_pos = Vector3f{x: x, y: 220.0, z: z};
                        PostureModule::set_pos(module_accessor, &boss_y_pos);
                    }
                    else if PostureModule::pos_y(boss_boma) <= -100.0 {
                        let boss_y_pos = Vector3f{x: x, y: -100.0, z: z};
                        PostureModule::set_pos(module_accessor, &boss_y_pos);
                    }
                    else if PostureModule::pos_x(boss_boma) >= 200.0 {
                        let boss_x_pos = Vector3f{x: 200.0, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &boss_x_pos);
                    }
                    else if PostureModule::pos_x(boss_boma) <= -220.0 {
                        let boss_x_pos = Vector3f{x: -220.0, y: y, z: z};
                        PostureModule::set_pos(module_accessor, &boss_x_pos);
                    }
                    else {
                        PostureModule::set_pos(module_accessor, &boss_pos);
                    }
                    if MotionModule::frame(fighter.module_accessor) >= 29.0 {
                        if sv_information::is_ready_go() == false {
                            HAVE_ITEM = false;
                            IS_BOSS_DEAD = false;
                            CHARACTER_IS_TURNING = false;
                            let lua_state = fighter.lua_state_agent;
                            let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
                            ENTRY_ID = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
                            if SPAWN_BOSS == true {
                                let boss_boma = sv_battle_object::module_accessor(BOSS_ID[entry_id(module_accessor)]);
                                
                                ENTRANCE_ANIM = false;
                                ItemModule::have_item(module_accessor,ItemKind(*ITEM_KIND_CRAZYHAND),0,0,false,false);
                                    BOSS_ID[entry_id(module_accessor)] = ItemModule::get_have_item_id(module_accessor,0) as u32;
                                ModelModule::set_scale(module_accessor,0.0001);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                                //ItemModule::throw_item(fighter.module_accessor, 0.0, 0.0, 0.0, 0, true, 0.0);
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_STANDBY,true);
                                HAVE_ITEM = true;
                            }
                        }
                    }

                    DamageModule::set_damage_lock(boss_boma,true);
                    WHOLE_HIT(fighter, *HIT_STATUS_XLU);
                    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_XLU), 0);

                    if StopModule::is_damage(boss_boma) {
                        if DamageModule::damage(module_accessor, 0) >= 360.0 {
                            if IS_BOSS_DEAD == false {
                                IS_BOSS_DEAD = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                            }
                        }
                        if DamageModule::damage(module_accessor, 0) == -1.0 {
                            if IS_BOSS_DEAD == false {
                                IS_BOSS_DEAD = true;
                                StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                            }
                        }
                        DamageModule::add_damage(module_accessor, 0.5, 0);
                    }

                    if StatusModule::status_kind(module_accessor) == *FIGHTER_STATUS_KIND_DEAD {
                        if IS_BOSS_DEAD == false {
                            IS_BOSS_DEAD = true;
                            StatusModule::change_status_request_from_script(boss_boma,*ITEM_STATUS_KIND_DEAD,true);
                        }
                    }

                    if IS_BOSS_DEAD == true {
                        if sv_information::is_ready_go() == true {
                            if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) != 0 {
                                StatusModule::change_status_request_from_script(module_accessor,*FIGHTER_STATUS_KIND_DEAD,true);
                                DamageModule::add_damage(module_accessor, 360.0, 0);
                                STOP_CONTROL_LOOP = false;
                            }
                        }
                    }

                    if FighterInformation::stock_count(FighterManager::get_fighter_information(fighter_manager,smash::app::FighterEntryID(ENTRY_ID as i32))) == 0 {
                        if StatusModule::status_kind(module_accessor) != *FIGHTER_STATUS_KIND_STANDBY {
                            StatusModule::change_status_request_from_script(module_accessor, *FIGHTER_STATUS_KIND_STANDBY,true);
                        }
                    }

                    if sv_information::is_ready_go() == false {
                        STOP_CONTROL_LOOP = true;
                        if MotionModule::frame(fighter.module_accessor) >= 30.0 {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                        }
                    }

                    if MotionModule::frame(fighter.module_accessor) >= 30.0 {
                        if sv_information::is_ready_go() == true {
                            HAVE_ITEM = true;
                        }
                    }
                    if STOP_CONTROL_LOOP == true {
                        if CHARACTER_IS_TURNING == true {

                        }
                        else {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM {
                        //BOSS POSITION
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.4, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.4, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.4, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.4, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START {
                        
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START {
                        MotionModule::set_rate(boss_boma, 1.7);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                        MotionModule::set_rate(boss_boma, 2.0);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_LOOP {
                        MotionModule::set_rate(boss_boma, 1.2);
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW {
                        MotionModule::set_rate(boss_boma, 1.2);
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START {
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }

                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START {
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.5, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.5, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START {
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.75, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_LOOP {
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.55, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.55, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.55, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.55, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 0.75, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START {
                        MotionModule::set_rate(boss_boma, 1.2);
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.2, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.2, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.2, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK {
                        //BOSS POSITION
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.1, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.1, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.1, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.1, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {
                        STOP_CONTROL_LOOP = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {
                        STOP_CONTROL_LOOP = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL {
                        STOP_CONTROL_LOOP = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END {
                        STOP_CONTROL_LOOP = false;
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                        MotionModule::set_rate(boss_boma, 2.0);
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_END {
                        MotionModule::set_rate(boss_boma, 1.6);
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_LOOK_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DIG_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_MISS_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_END {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_2 {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_THROW_END_3 {
                        if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                            STOP_CONTROL_LOOP = true;
                        }
                    }
                    if MotionModule::frame(boss_boma) == MotionModule::end_frame(boss_boma) {
                        STOP_CONTROL_LOOP = true;
                        CHARACTER_IS_TURNING = false;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START {
                        MotionModule::set_rate(boss_boma, 1.4);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD {
                        MotionModule::set_rate(boss_boma, 1.2);
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU {
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 1.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 1.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI {
                        MotionModule::set_rate(boss_boma, 1.4);
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_END {
                        MotionModule::set_rate(boss_boma, 1.5);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK {
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) == false {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_END, true);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START {
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW, true);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_LOOP {
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END, true);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DRILL_ATTACK {
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                            MotionModule::set_rate(boss_boma, 4.0);
                        }
                        if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) == false {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_END {
                        MotionModule::set_rate(boss_boma, 1.0);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_COMPOUND_ATTACK_WAIT {
                        STOP_CONTROL_LOOP = true;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TIME {
                        STOP_CONTROL_LOOP = true;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_RND_WAIT {
                        STOP_CONTROL_LOOP = true;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_CHASE {
                        STOP_CONTROL_LOOP = true;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TO_POINT {
                        STOP_CONTROL_LOOP = true;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_FEINT {
                        STOP_CONTROL_LOOP = true;
                        CHARACTER_IS_TURNING = false;
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {
                        STOP_CONTROL_LOOP = true;
                    }
                    if STOP_CONTROL_LOOP == true {
                        if HAVE_ITEM == true {
                            if IS_BOSS_DEAD == false {
                                if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_RUN {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TURN {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_NONE {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_JUMP {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_JUMP_AIR {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_FALL {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_EXIT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_PASS_FLOOR {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT {

                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_START {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_TERM {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_LANDING {
                                    
                                }
                                else if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_INITIALIZE {

                                }
                                else {
                                    if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_ENTRY {

                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_STATUS_KIND_WAIT, true);
                                    }
                                }
                            }
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO_PRE_MOVE {
                        MotionModule::set_rate(boss_boma, 2.0);
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU {
                        //Boss Control Stick Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }

                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 0.75, y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_KUMO {
                        
                    }
                    if CHARACTER_IS_TURNING == true {
                        MotionModule::set_rate(boss_boma, 2.0);
                    }
                    if StatusModule::status_kind(boss_boma) != *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                        if STOP_CONTROL_LOOP == false {

                        }
                        else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_TURN {
                            MotionModule::set_rate(boss_boma, 2.0);
                        }
                        else {
                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT, true);
                        }
                    }
                    if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DEBUG_WAIT {
                        STOP_CONTROL_LOOP = true;
                        if MotionModule::frame(boss_boma) >= 40.0 {
                            CHARACTER_IS_TURNING = false;
                        }
                    }
                    if STOP_CONTROL_LOOP == true {
                        if CHARACTER_IS_TURNING == true {

                        }
                        else {
                            MotionModule::set_rate(boss_boma, 1.0);
                        }
                        if StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_DEAD {

                        }
                        else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_START {

                        }
                        else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_FALL {

                        }
                        else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LOOP {

                        }
                        else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_END {

                        }
                        else if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_DOWN_LANDING {

                        }
                        else {
                            //StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                        }
                    }
                    if STOP_CONTROL_LOOP == true {
                        TELEPORTED = false;
                    }
                    if STOP_CONTROL_LOOP == false {
                        if StatusModule::status_kind(boss_boma) == *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT {
                            if TELEPORTED == false {
                                //Boss Control Stick Movement
                                if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                                        let pos = Vector3f{x: -140.0, y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                        TELEPORTED = true;
                                }
                                if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                                        let pos = Vector3f{x: 140.0, y: 0.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                        TELEPORTED = true;
                                }
                                if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                                        let pos = Vector3f{x: 0.0, y: -50.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                        TELEPORTED = true;
                                }
                                if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                                        let pos = Vector3f{x: 0.0, y: 50.0, z: 0.0};
                                        PostureModule::add_pos(boss_boma, &pos);
                                        TELEPORTED = true;
                                }
                            }
                        }
                    }
                    if STOP_CONTROL_LOOP == true {
                        //Boss Control Movement
                        if ControlModule::get_stick_x(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * - 2.0 * ControlModule::get_stick_x(module_accessor), y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_x(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: ControlModule::get_stick_x(module_accessor) * 2.0 * ControlModule::get_stick_x(module_accessor), y: 0.0, z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) <= 0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * - 2.0 * ControlModule::get_stick_y(module_accessor), z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                    
                        if ControlModule::get_stick_y(module_accessor) >= -0.001 {
                            let pos = Vector3f{x: 0.0, y: ControlModule::get_stick_y(module_accessor) * 2.0 * ControlModule::get_stick_y(module_accessor), z: 0.0};
                            PostureModule::add_pos(boss_boma, &pos);
                        }
                        if CHARACTER_IS_TURNING == false {
                            //Boss Moves
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP) {
                                if CHARACTER_IS_TURNING == false {
                                CHARACTER_IS_TURNING = true;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_TURN, true);
                                }
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_BOMB_ATTACK_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WAIT_TELEPORT, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DIG_PRE_MOVE, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_LW != 0 {
                                STOP_CONTROL_LOOP = false;
                                if PostureModule::pos_y(boss_boma) <= 25.0 {
                                    if PostureModule::pos_y(boss_boma) >= -25.0 {
                                        if PostureModule::pos_x(boss_boma) <= 40.0 {
                                            if PostureModule::pos_x(boss_boma) >= -40.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_WFINGER_BEAM_START, true);
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                                }
                                        }
                                        else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                            }
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                        }
                                }
                                else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_YUBI_BEAM, true);
                                }
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_HI != 0 {
                                STOP_CONTROL_LOOP = false;
                                if PostureModule::pos_y(boss_boma) <= 25.0 {
                                    if PostureModule::pos_y(boss_boma) >= -25.0 {
                                        if PostureModule::pos_x(boss_boma) <= 25.0 {
                                            if PostureModule::pos_x(boss_boma) >= -25.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_FIRE_CHARIOT_PRE_MOVE, true);
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                            }
                                        }
                                        else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                        }
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                    }
                                }
                                else {
                                    StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_LOOK_START, true);
                                }
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_SPECIAL_S != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_GROW_FINGER_START, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_LW3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_KUMO_PRE_MOVE, true);
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_HI3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                if PostureModule::pos_y(boss_boma) <= 25.0 {
                                    if PostureModule::pos_y(boss_boma) >= -25.0 {
                                        if PostureModule::pos_x(boss_boma) <= 40.0 {
                                            if PostureModule::pos_x(boss_boma) >= -40.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_BARK, true);
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                                }
                                        }
                                        else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                            }
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                        }
                                }
                                else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_DRILL_START, true);
                                }
                            }
                            if ControlModule::get_command_flag_cat(fighter.module_accessor, 0) & *FIGHTER_PAD_CMD_CAT1_FLAG_ATTACK_S3 != 0 {
                                STOP_CONTROL_LOOP = false;
                                let slap_y_pos_local = Vector3f{x: x, y: PostureModule::pos_y(boss_boma), z: z};
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_HIPPATAKU_HOLD, true);
                                PostureModule::set_pos(boss_boma, &slap_y_pos_local);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_HI) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_SCRATCH_BLOW_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_LW) {
                                STOP_CONTROL_LOOP = false;
                                if PostureModule::pos_y(boss_boma) <= 35.0 {
                                    if PostureModule::pos_y(boss_boma) >= 0.0 {
                                        if PostureModule::pos_x(boss_boma) <= 75.0 {
                                            if PostureModule::pos_x(boss_boma) >= -75.0 {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NOTAUTSU_PRE_MOVE, true);
                                            }
                                            else {
                                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                                }
                                        }
                                        else {
                                            StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                            }
                                    }
                                    else {
                                        StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                        }
                                }
                                else {
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_NIGIRU_START, true);
                                }
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_GRAVITY_BALL_START, true);
                            }
                            if ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
                                STOP_CONTROL_LOOP = false;
                                StatusModule::change_status_request_from_script(boss_boma, *ITEM_CRAZYHAND_STATUS_KIND_HIKOUKI_START, true);
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

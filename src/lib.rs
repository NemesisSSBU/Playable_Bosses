#![feature(proc_macro_hygiene)]

use prc::*;
use prc::hash40::Hash40;
use arcropolis_api::*;
use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use skyline::nn::ro::LookupSymbol;
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, Main};

mod mastercrazy;
mod playable_masterhand;
mod galeem;
mod dharkon;
mod marx;
mod dracula;
mod rathalos;
mod galleom;
mod ganon;
mod gigabowser;
mod config;

use crate::config::CONFIG;

static mut ENTRY_ID : usize = 0;
pub static mut FIGHTER_MANAGER: usize = 0;

static mut ENTRY_ID_2 : usize = 0;
pub static mut FIGHTER_MANAGER_2: usize = 0;

static mut ENTRY_ID_3 : usize = 0;
pub static mut FIGHTER_MANAGER_3: usize = 0;

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
        let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
        if fighter_kind == *FIGHTER_KIND_PEACH {
            if FighterManager::is_final(fighter_manager) {
                if mastercrazy::check_status()
                || mastercrazy::check_status_2()
                || playable_masterhand::check_status()
                || galeem::check_status()
                || dharkon::check_status()
                || marx::check_status()
                || dracula::check_status()
                || rathalos::check_status()
                || galleom::check_status()
                || ganon::check_status() {
                    WorkModule::enable_transition_term_forbid(fighter.module_accessor,*FIGHTER_STATUS_TRANSITION_TERM_ID_FINAL);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_STATUS);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_AVAILABLE);
                    FighterManager::set_visible_finalbg(fighter_manager, false);
                }
            }
        }
    }
}

extern "C" fn once_per_fighter_frame_2(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        ENTRY_ID_2 = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        LookupSymbol(
            &raw mut FIGHTER_MANAGER_2,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
            .as_bytes()
            .as_ptr(),
        );
        let fighter_manager = *(FIGHTER_MANAGER_2 as *mut *mut smash::app::FighterManager);
        if fighter_kind == *FIGHTER_KIND_DAISY {
            if FighterManager::is_final(fighter_manager) {
                if mastercrazy::check_status()
                || mastercrazy::check_status_2()
                || playable_masterhand::check_status()
                || galeem::check_status()
                || dharkon::check_status()
                || marx::check_status()
                || dracula::check_status()
                || rathalos::check_status()
                || galleom::check_status()
                || ganon::check_status() {
                    WorkModule::enable_transition_term_forbid(fighter.module_accessor,*FIGHTER_STATUS_TRANSITION_TERM_ID_FINAL);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_STATUS);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_AVAILABLE);
                    FighterManager::set_visible_finalbg(fighter_manager, false);
                }
            }
        }
    }
}

extern "C" fn once_per_fighter_frame_3(fighter: &mut L2CFighterCommon) {
    unsafe {
        let lua_state = fighter.lua_state_agent;
        let module_accessor = smash::app::sv_system::battle_object_module_accessor(lua_state);
        let fighter_kind = smash::app::utility::get_kind(module_accessor);
        ENTRY_ID_3 = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize;
        LookupSymbol(
            &raw mut FIGHTER_MANAGER_3,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
            .as_bytes()
            .as_ptr(),
        );
        let fighter_manager = *(FIGHTER_MANAGER_3 as *mut *mut smash::app::FighterManager);
        if fighter_kind == *FIGHTER_KIND_SZEROSUIT {
            if FighterManager::is_final(fighter_manager) {
                if ganon::check_status() {
                    WorkModule::enable_transition_term_forbid(fighter.module_accessor,*FIGHTER_STATUS_TRANSITION_TERM_ID_FINAL);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_STATUS);
                    WorkModule::off_flag(fighter.module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_AVAILABLE);
                    FighterManager::set_visible_finalbg(fighter_manager, false);
                }
            }
        }
    }
}

pub fn to_hash40(word: &str) -> Hash40 {
    Hash40(crc32_with_len(word))
}

fn crc32_with_len(word: &str) -> u64 {
    let mut hash = !0u32;
    let mut len: u8 = 0;
    for b in word.bytes() {
        let shift = hash >> 8;
        let index = (hash ^ (b as u32)) & 0xff;
        hash = shift ^ _CRC_TABLE[index as usize];
        len += 1;
    }
    ((len as u64) << 32) | (!hash as u64)
}

const _CRC_TABLE: [u32; 256] = [
    0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
    0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
    0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
    0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
    0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
    0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
    0x76dc4190, 0x01db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
    0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
    0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
    0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
    0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
    0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683,
    0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
    0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
    0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
    0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
    0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
    0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21,
    0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
    0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
    0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
    0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
];

// Giga Bowser

#[arc_callback]
fn callback_koopag(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    // These comments are from the great people over on the SSBU Modding Discord Server
    // with the param data ready,
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();

    // enter the first and only node of the file ("db_root")
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));

    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();

    // iterate the list to find the param with mario's data
    // we could go to the exact index, but this is subject to change across game updates.
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();

        // we assume ui_chara_id will always be the first param.
        // given the file, this is a safe assumption, but there are
        // more fool-proof ways of searching for the right node.
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();

        // check to make sure it's Koopag
        *ui_chara_hash == to_hash40("ui_chara_koopag")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();

    // now we have Koopags's data, we can convert to a dictionary to gain faster access
    // to arbitrary keys, but since we only want to change 1 param, we'll just iterate
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_koopa");
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 15;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 15;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_koopa");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_mario");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Master Hand

#[arc_callback]
fn callback_masterhand(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_masterhand")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 118;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 87;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_masterhand");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Crazy Hand

#[arc_callback]
fn callback_crazyhand(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_crazyhand")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 119;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 88;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_crazyhand");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Dharkon

#[arc_callback]
fn callback_dharkon(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_darz")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 120;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 89;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_darz");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Galeem

#[arc_callback]
fn callback_galeem(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_kiila")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 121;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 90;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_kiila");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Marx

#[arc_callback]
fn callback_marx(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_marx")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 122;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 91;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_marx");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_kirby");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Ganon

#[arc_callback]
fn callback_ganon(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_ganonboss")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 123;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 92;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_ganonboss");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_zelda");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Dracula

#[arc_callback]
fn callback_dracula(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_dracula")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 124;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 93;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_dracula");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_castlevania");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Galleom

#[arc_callback]
fn callback_galleom(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_galleom")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 125;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 94;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_galleom");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// Rathalos

#[arc_callback]
fn callback_rathalos(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_lioleus")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = false;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 126;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 95;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_rathalos");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// WOL Master Hand

#[arc_callback]
fn callback_wolmh(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_chara_mewtwo_masterhand")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("is_hidden_boss") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 127;
        }
        if *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 96;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("fighter_kind") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_kind_mario");
        }
        if *hash == to_hash40("characall_label_c00") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("vc_narration_characall_masterhandwol2");
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
        if *hash == to_hash40("fighter_type") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("fighter_type_normal");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

// UNLOCKS HIDDEN MAPS

#[arc_callback]
fn callback_map_1(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_stage_boss_final2")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_2(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_stage_boss_final3")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_3(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_stage_boss_ganon")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_zelda");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_4(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_stage_boss_rathalos")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_5(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_stage_boss_marx")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_kirby");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_6(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_stage_boss_galleom")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

#[arc_callback]
fn callback_map_7(hash: u64, mut data: &mut [u8]) -> Option<usize> {
    load_original_file(hash, &mut data);
    let mut reader = std::io::Cursor::new(&mut data);
    let mut root = prc::read_stream(&mut reader).unwrap();
    let (db_root_hash, db_root) = &mut root.0[0];
    assert_eq!(*db_root_hash, to_hash40("db_root"));
    let db_root_list = db_root.try_into_mut::<ParamList>().unwrap();
    let charroot = db_root_list.0.iter_mut().find(|param| {
        let ui_chara_struct = param.try_into_ref::<ParamStruct>().unwrap();
        let (_, ui_chara_id) = &ui_chara_struct.0[0];
        let ui_chara_hash = ui_chara_id.try_into_ref::<Hash40>().unwrap();
        *ui_chara_hash == to_hash40("ui_stage_boss_dracula")
    }).unwrap().try_into_mut::<ParamStruct>().unwrap();
    charroot.0.iter_mut().for_each(|(hash, param)| {
        if *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 0;
        }
        if *hash == to_hash40("is_usable") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("can_select") {
            *param.try_into_mut::<bool>().unwrap() = true;
        }
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_castlevania");
        }
    });
    let mut writer = std::io::Cursor::new(data);
    write_stream(&mut writer, &root).unwrap();
    return Some(writer.position() as usize);
}

const MAX_FILE_SIZE: usize = 0xFFFF;

#[skyline::main(name = "comp_boss")]
pub fn main() {
    let cfg = &*CONFIG;
    let opts = &cfg.options;
    let giga_bowser_normal = !opts.giga_bowser_normal.unwrap_or(false);
    let use_disp_order_char = !opts.custom_css.unwrap_or(false);
    let master_hand_css = opts.master_hand_css.unwrap_or(true);
    let crazy_hand_css = opts.crazy_hand_css.unwrap_or(true);
    let dharkon_css = opts.dharkon_css.unwrap_or(true);
    let galeem_css = opts.galeem_css.unwrap_or(true);
    let marx_css = opts.marx_css.unwrap_or(true);
    let giga_bowser_css = opts.giga_bowser_css.unwrap_or(true);
    let ganon_css = opts.ganon_css.unwrap_or(true);
    let dracula_css = opts.dracula_css.unwrap_or(true);
    let rathalos_css = opts.rathalos_css.unwrap_or(true);
    let galleom_css = opts.galleom_css.unwrap_or(true);
    let wol_master_hand_css = opts.wol_master_hand_css.unwrap_or(true);
    let final2_stage = opts.final2_stage.unwrap_or(true);
    let final3_stage = opts.final3_stage.unwrap_or(true);
    let ganon_stage = opts.ganon_stage.unwrap_or(true);
    let rathalos_stage = opts.rathalos_stage.unwrap_or(true);
    let marx_stage = opts.marx_stage.unwrap_or(true);
    let galleom_stage = opts.galleom_stage.unwrap_or(true);
    let dracula_stage = opts.dracula_stage.unwrap_or(true);

    Agent::new("daisy").on_line(Main, once_per_fighter_frame).install();
    Agent::new("peach").on_line(Main, once_per_fighter_frame_2).install();
    Agent::new("szerosuit").on_line(Main, once_per_fighter_frame_3).install();

    mastercrazy::install();
    playable_masterhand::install();
    galeem::install();
    dharkon::install();
    marx::install();
    rathalos::install();
    dracula::install();
    galleom::install();
    ganon::install();
    if giga_bowser_normal { gigabowser::install(); }

    if use_disp_order_char {
        if giga_bowser_css { callback_koopag::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if master_hand_css { callback_masterhand::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if crazy_hand_css { callback_crazyhand::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if dharkon_css { callback_dharkon::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if galeem_css { callback_galeem::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if dracula_css { callback_dracula::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if marx_css { callback_marx::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if ganon_css { callback_ganon::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if galleom_css { callback_galleom::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if rathalos_css { callback_rathalos::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
        if wol_master_hand_css { callback_wolmh::install("ui/param/database/ui_chara_db.prc", MAX_FILE_SIZE); }
    }

    if final2_stage { callback_map_1::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if final3_stage { callback_map_2::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if ganon_stage { callback_map_3::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if rathalos_stage { callback_map_4::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if marx_stage { callback_map_5::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if galleom_stage { callback_map_6::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if dracula_stage { callback_map_7::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
}

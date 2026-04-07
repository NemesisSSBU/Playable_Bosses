#![feature(proc_macro_hygiene)]

use prc::*;
use prc::hash40::Hash40;
use arcropolis_api::*;
use smash::lib::lua_const::*;
use smash::app::lua_bind::*;
use skyline::nn::ro::LookupSymbol;
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, Main};
use std::fs;

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
mod selection;
mod debug;
mod boss_helpers;
mod boss_runtime;

use crate::config::CONFIG;

pub static mut FIGHTER_MANAGER: usize = 0;

const MAX_FIGHTERS: usize = 8;
static mut BOSS_FINAL_GUARD_ACTIVE: [bool; 8] = [false; 8];
static mut BOSS_MATCH_STARTED: [bool; 8] = [false; 8];
static mut TRANSITION_DEBUG_LAST_STAGE: [i32; 8] = [-1; 8];
static mut TRANSITION_DEBUG_LAST_STATUS: [i32; 8] = [i32::MIN; 8];
static mut TRANSITION_DEBUG_LAST_FLAGS: [u16; 8] = [u16::MAX; 8];
static mut TRANSITION_DEBUG_LAST_HAVE_ITEM: [i32; 8] = [i32::MIN; 8];
static mut TRANSITION_DEBUG_LAST_SCALE_BITS: [u32; 8] = [u32::MAX; 8];

unsafe fn any_boss_active() -> bool {
    mastercrazy::check_status()
    || mastercrazy::check_status_2()
    || playable_masterhand::check_status()
    || galeem::check_status()
    || dharkon::check_status()
    || marx::check_status()
    || dracula::check_status()
    || rathalos::check_status()
    || galleom::check_status()
    || ganon::check_status()
}

unsafe fn suppress_hidden_host_result_audio(module_accessor: *mut smash::app::BattleObjectModuleAccessor) {
    if module_accessor.is_null() || !boss_helpers::is_hidden_host(module_accessor) {
        return;
    }
    LookupSymbol(
        &raw mut FIGHTER_MANAGER,
        "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\u{0}"
        .as_bytes()
        .as_ptr(),
    );
    let fighter_manager = *(FIGHTER_MANAGER as *mut *mut smash::app::FighterManager);
    if fighter_manager.is_null() || !FighterManager::is_result_mode(fighter_manager) {
        return;
    }
    boss_helpers::stop_hidden_host_mario_result_sfx(module_accessor);
}

unsafe fn log_hidden_host_transition_snapshot(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() {
        return;
    }

    let entry_id = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    let fighter_manager = boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let ready_go = smash::app::sv_information::is_ready_go();
    let hidden_host = boss_helpers::is_hidden_host(module_accessor);
    let match_started = BOSS_MATCH_STARTED[entry_id];

    if ready_go && !result_mode && !match_started {
        return;
    }

    let stage_id = smash::app::stage::get_stage_id();
    let fighter_status = StatusModule::status_kind(module_accessor);
    let any_boss = any_boss_active();
    let have_item_id = if ItemModule::is_have_item(module_accessor, 0) {
        ItemModule::get_have_item_id(module_accessor, 0) as i32
    } else {
        -1
    };
    let scale_bits = ModelModule::scale(module_accessor).to_bits();
    let flags =
        (ready_go as u16)
        | ((result_mode as u16) << 1)
        | ((hidden_host as u16) << 2)
        | ((match_started as u16) << 3)
        | ((any_boss as u16) << 4);

    if TRANSITION_DEBUG_LAST_STAGE[entry_id] == stage_id
        && TRANSITION_DEBUG_LAST_STATUS[entry_id] == fighter_status
        && TRANSITION_DEBUG_LAST_FLAGS[entry_id] == flags
        && TRANSITION_DEBUG_LAST_HAVE_ITEM[entry_id] == have_item_id
        && TRANSITION_DEBUG_LAST_SCALE_BITS[entry_id] == scale_bits
    {
        return;
    }

    TRANSITION_DEBUG_LAST_STAGE[entry_id] = stage_id;
    TRANSITION_DEBUG_LAST_STATUS[entry_id] = fighter_status;
    TRANSITION_DEBUG_LAST_FLAGS[entry_id] = flags;
    TRANSITION_DEBUG_LAST_HAVE_ITEM[entry_id] = have_item_id;
    TRANSITION_DEBUG_LAST_SCALE_BITS[entry_id] = scale_bits;

    crate::boss_log!(
        "[PB][TransitionState] entry={} stage=0x{:x} ready_go={} result_mode={} hidden_host={} match_started={} any_boss={} fighter_status={} have_item_id={} scale={:.4}",
        entry_id,
        stage_id,
        ready_go,
        result_mode,
        hidden_host,
        match_started,
        any_boss,
        fighter_status,
        have_item_id,
        f32::from_bits(scale_bits)
    );
}

unsafe fn cleanup_hidden_host_post_match_transition(
    module_accessor: *mut smash::app::BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() {
        return;
    }

    let entry_id = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    let fighter_manager = boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let hidden_host = boss_helpers::is_hidden_host(module_accessor);
    let ready_go = smash::app::sv_information::is_ready_go();
    let stage_id = smash::app::stage::get_stage_id();

    if ready_go {
        if boss_helpers::is_boss_passthrough_stage(stage_id)
            && (BOSS_MATCH_STARTED[entry_id] || hidden_host || any_boss_active())
        {
            selection::suppress_boss_selection_until_ready_go(entry_id);
            BOSS_MATCH_STARTED[entry_id] = false;
            boss_runtime::reset_all_for_entry(entry_id);
            playable_masterhand::reset_match_state(entry_id);
            mastercrazy::reset_match_state(entry_id);
            galeem::reset_match_state(entry_id);
            dharkon::reset_match_state(entry_id);
            marx::reset_match_state(entry_id);
            dracula::reset_match_state(entry_id);
            rathalos::reset_match_state(entry_id);
            galleom::reset_match_state(entry_id);
            ganon::reset_match_state(entry_id);

            crate::boss_log!(
                "[PB][TransitionCleanup] entry {}: restoring boss host for passthrough stage=0x{:x} hidden_host={}",
                entry_id,
                stage_id,
                hidden_host
            );

            boss_helpers::restore_hidden_host_baseline(module_accessor);
            crate::boss_log!(
                "[PB][TransitionCleanupState] entry {}: passthrough restore complete stage=0x{:x} fighter_status={} scale={:.4} pos=({:.2},{:.2},{:.2})",
                entry_id,
                stage_id,
                StatusModule::status_kind(module_accessor),
                ModelModule::scale(module_accessor),
                PostureModule::pos_x(module_accessor),
                PostureModule::pos_y(module_accessor),
                PostureModule::pos_z(module_accessor)
            );

            if !fighter_manager.is_null() {
                FighterManager::set_cursor_whole(fighter_manager, true);
                FighterManager::set_position_lock(
                    fighter_manager,
                    smash::app::FighterEntryID(entry_id as i32),
                    false,
                );
            }

            return;
        }
        if hidden_host || any_boss_active() {
            BOSS_MATCH_STARTED[entry_id] = true;
        }
        return;
    }

    if result_mode {
        if BOSS_MATCH_STARTED[entry_id] || hidden_host || any_boss_active() {
            boss_helpers::pin_hidden_host_result_state(module_accessor);
        }
        if BOSS_MATCH_STARTED[entry_id] {
            selection::suppress_boss_selection_until_ready_go(entry_id);
        }
        return;
    }

    if !BOSS_MATCH_STARTED[entry_id] {
        return;
    }

    if galeem::entry_transition_in_progress(module_accessor)
        || dharkon::entry_transition_in_progress(module_accessor)
    {
        return;
    }

    selection::suppress_boss_selection_until_ready_go(entry_id);
    if boss_helpers::is_classic_route_terminal_battle(stage_id) {
        selection::clear_cached_boss_hash_for_entry(
            entry_id,
            "classic_route_terminal_battle",
        );
    }
    BOSS_MATCH_STARTED[entry_id] = false;
    boss_runtime::reset_all_for_entry(entry_id);
    playable_masterhand::reset_match_state(entry_id);
    mastercrazy::reset_match_state(entry_id);
    galeem::reset_match_state(entry_id);
    dharkon::reset_match_state(entry_id);
    marx::reset_match_state(entry_id);
    dracula::reset_match_state(entry_id);
    rathalos::reset_match_state(entry_id);
    galleom::reset_match_state(entry_id);
    ganon::reset_match_state(entry_id);

    crate::boss_log!(
        "[PB][TransitionCleanup] entry {}: restoring boss host after non-result match transition on stage=0x{:x} hidden_host={}",
        entry_id,
        stage_id,
        hidden_host
    );

    // Do NOT call any game module APIs here (pin_hidden_host_result_state,
    // FighterManager::set_cursor_whole, set_position_lock, etc.).
    // The fighter NRO unloads and reloads between stages, giving a fresh
    // fighter state.  Calling module APIs during the post-match transition
    // window can corrupt the game's internal transition state machine —
    // intermediate battles recover because the next stage load resets
    // everything, but the final classic-mode battle has no next stage,
    // causing a softlock (black screen, no credits).
    crate::boss_log!(
        "[PB][TransitionCleanupState] entry {}: bookkeeping-only cleanup complete stage=0x{:x} (no game API calls)",
        entry_id,
        stage_id,
    );
}

extern "C" fn mario_boss_dispatch_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let module_accessor = fighter.module_accessor;
        selection::clear_boss_selection_suppression_if_ready_go(module_accessor);
        mastercrazy::master_frame(fighter);
        mastercrazy::crazy_frame(fighter);
        playable_masterhand::frame(fighter);
        galeem::frame(fighter);
        dharkon::frame(fighter);
        marx::frame(fighter);
        dracula::frame(fighter);
        rathalos::frame(fighter);
        galleom::frame(fighter);
        ganon::frame(fighter);
        log_hidden_host_transition_snapshot(module_accessor);
        suppress_hidden_host_result_audio(module_accessor);
        cleanup_hidden_host_post_match_transition(module_accessor);
    }
}

#[inline(always)]
unsafe fn apply_boss_final_guard(
    fighter: &mut L2CFighterCommon,
    fighter_manager: *mut smash::app::FighterManager,
) {
    if fighter_manager.is_null() {
        return;
    }

    let module_accessor = fighter.module_accessor;
    if module_accessor.is_null() {
        return;
    }

    let entry_id = WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if entry_id < 0 || entry_id as usize >= MAX_FIGHTERS {
        return;
    }
    let entry_id = entry_id as usize;
    let boss_active = any_boss_active();
    let fighter_kind = smash::app::utility::get_kind(&mut *module_accessor);
    let allow_boss_final = fighter_kind == *FIGHTER_KIND_PEACH
        || fighter_kind == *FIGHTER_KIND_DAISY;

    if boss_active && allow_boss_final {
        if BOSS_FINAL_GUARD_ACTIVE[entry_id] {
            WorkModule::unable_transition_term_forbid(
                module_accessor,
                *FIGHTER_STATUS_TRANSITION_TERM_ID_FINAL,
            );
            BOSS_FINAL_GUARD_ACTIVE[entry_id] = false;
            crate::boss_log!(
                "[PB][Final] entry {}: Peach/Daisy Final Smash allowed while boss active",
                entry_id
            );
        }
        return;
    }

    if boss_active {
        if !BOSS_FINAL_GUARD_ACTIVE[entry_id] {
            BOSS_FINAL_GUARD_ACTIVE[entry_id] = true;
            crate::boss_log!(
                "[PB][Final] entry {}: generic guard enabled (boss active)",
                entry_id
            );
        }
        WorkModule::enable_transition_term_forbid(
            module_accessor,
            *FIGHTER_STATUS_TRANSITION_TERM_ID_FINAL,
        );
        WorkModule::off_flag(
            module_accessor,
            *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_AVAILABLE,
        );
        if WorkModule::is_flag(module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL)
            || WorkModule::is_flag(module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_STATUS)
            || FighterManager::is_final(fighter_manager)
        {
            crate::boss_log!(
                "[PB][Final] entry {}: clearing active Final Smash state",
                entry_id
            );
            WorkModule::off_flag(module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL);
            WorkModule::off_flag(module_accessor, *FIGHTER_INSTANCE_WORK_ID_FLAG_FINAL_STATUS);
            FighterManager::set_visible_finalbg(fighter_manager, false);
        }
    } else if BOSS_FINAL_GUARD_ACTIVE[entry_id] {
        WorkModule::unable_transition_term_forbid(
            module_accessor,
            *FIGHTER_STATUS_TRANSITION_TERM_ID_FINAL,
        );
        BOSS_FINAL_GUARD_ACTIVE[entry_id] = false;
        crate::boss_log!(
            "[PB][Final] entry {}: generic guard disabled (no boss active)",
            entry_id
        );
    }
}

extern "C" fn boss_final_guard_frame(fighter: &mut L2CFighterCommon) {
    unsafe {
        let fighter_manager = boss_helpers::fighter_manager();
        apply_boss_final_guard(fighter, fighter_manager);
    }
}

pub fn to_hash40(word: &str) -> Hash40 {
    Hash40(crc32_with_len(word))
}

pub fn crc32_with_len(word: &str) -> u64 {
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

fn patch_selector_param_value(param: &mut ParamKind, ui_chara_hash: Hash40, selector_id: i32) -> bool {
    if let Ok(value) = param.try_into_mut::<Hash40>() {
        *value = ui_chara_hash;
        return true;
    }
    if let Ok(value) = param.try_into_mut::<i32>() {
        *value = selector_id;
        return true;
    }
    if let Ok(value) = param.try_into_mut::<u32>() {
        *value = selector_id as u32;
        return true;
    }
    if let Ok(value) = param.try_into_mut::<i16>() {
        if let Ok(small) = i16::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    if let Ok(value) = param.try_into_mut::<u16>() {
        if let Ok(small) = u16::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    if let Ok(value) = param.try_into_mut::<i8>() {
        if let Ok(small) = i8::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    if let Ok(value) = param.try_into_mut::<u8>() {
        if let Ok(small) = u8::try_from(selector_id) {
            *value = small;
            return true;
        }
        return false;
    }
    false
}

fn patch_tagged_selector_param_value(param: &mut ParamKind, selector_id: i32) -> bool {
    let tagged_selector = 0x5000_0000u64 | ((selector_id as u64) & 0x0FFF_FFFF);

    if let Ok(value) = param.try_into_mut::<u32>() {
        if *value == 0x5000_0000 {
            *value = tagged_selector as u32;
            return true;
        }
    }
    if let Ok(value) = param.try_into_mut::<i32>() {
        if (*value as u32) == 0x5000_0000 {
            *value = tagged_selector as i32;
            return true;
        }
    }

    false
}

fn patch_css_selector_fields(charroot: &mut ParamStruct, ui_chara_name: &str, selector_id: i32) {
    let ui_chara_hash = to_hash40(ui_chara_name);
    let selector_field_hashes = [
        to_hash40("summon_boss_id"),
        to_hash40("boss_id"),
        to_hash40("summon_id"),
    ];
    let mut found_field = false;
    let mut patched_field = false;
    let mut tagged_patch_count = 0usize;
    for (hash, param) in charroot.0.iter_mut() {
        if selector_field_hashes.contains(hash) {
            found_field = true;
            if patch_selector_param_value(param, ui_chara_hash, selector_id) {
                patched_field = true;
            }
        }
    }
    if !patched_field {
        for (_hash, param) in charroot.0.iter_mut() {
            if patch_tagged_selector_param_value(param, selector_id) {
                patched_field = true;
                tagged_patch_count += 1;
            }
        }
    }
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][SelectionPatch] {} selector=0x{:x} found_field={} patched_field={} tagged_patches={}",
            ui_chara_name,
            selector_id,
            found_field,
            patched_field,
            tagged_patch_count
        );
    }
}

fn find_mod_asset_path(relative_path: &str) -> Option<String> {
    let default_path = format!("sd:/ultimate/mods/Bosses/{}", relative_path);
    if fs::metadata(&default_path).is_ok() {
        return Some(default_path);
    }

    let mods_root = "sd:/ultimate/mods";
    let mut preferred = Vec::new();
    let mut others = Vec::new();
    if let Ok(entries) = fs::read_dir(mods_root) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                let candidate = format!("{}/{}", entry.path().to_string_lossy(), relative_path);
                if fs::metadata(&candidate).is_ok() {
                    if dir_name.contains("boss") || dir_name.contains("comp_boss") {
                        preferred.push(candidate);
                    } else {
                        others.push(candidate);
                    }
                }
            }
        }
    }

    preferred.into_iter().chain(others.into_iter()).next()
}

fn upsert_hash_param(struct_params: &mut ParamStruct, field_name: &str, value: Hash40) {
    let field_hash = to_hash40(field_name);
    if let Some((_, param)) = struct_params
        .0
        .iter_mut()
        .find(|(hash, _)| *hash == field_hash)
    {
        *param = ParamKind::Hash(value);
    } else {
        struct_params.0.push((field_hash, ParamKind::Hash(value)));
    }
}

fn upsert_u8_param(struct_params: &mut ParamStruct, field_name: &str, value: u8) {
    let field_hash = to_hash40(field_name);
    if let Some((_, param)) = struct_params
        .0
        .iter_mut()
        .find(|(hash, _)| *hash == field_hash)
    {
        *param = ParamKind::U8(value);
    } else {
        struct_params.0.push((field_hash, ParamKind::U8(value)));
    }
}

fn apply_mario_redirect_metadata(charroot: &mut ParamStruct) {
    // Mirror the CSK redirected-entry metadata so the boss rows inherit Mario's
    // unlock context without leaving Mario's own CSS tile in an undefined state.
    upsert_hash_param(charroot, "original_ui_chara_hash", to_hash40("ui_chara_mario"));
    upsert_u8_param(charroot, "color_start_index", 0);
}

#[derive(Clone, Debug)]
struct SarcEntry {
    name: String,
    data_start: usize,
    data_end: usize,
}

fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset + 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn parse_sarc(data: &[u8]) -> Option<(usize, Vec<SarcEntry>)> {
    if data.get(0..4)? != b"SARC" {
        return None;
    }

    let header_size = read_u16_le(data, 4)? as usize;
    let data_offset = read_u32_le(data, 0x0C)? as usize;
    if data_offset > data.len() {
        return None;
    }

    let sfat_offset = header_size;
    if data.get(sfat_offset..sfat_offset + 4)? != b"SFAT" {
        return None;
    }
    let node_count = read_u16_le(data, sfat_offset + 6)? as usize;
    let nodes_offset = sfat_offset + 0x0C;

    let sfnt_offset = nodes_offset + (node_count * 0x10);
    if data.get(sfnt_offset..sfnt_offset + 4)? != b"SFNT" {
        return None;
    }
    let names_offset = sfnt_offset + 0x08;

    let mut entries = Vec::with_capacity(node_count);
    for index in 0..node_count {
        let node_offset = nodes_offset + (index * 0x10);
        let attributes = read_u32_le(data, node_offset + 0x04)?;
        let has_name = (attributes & 0x0100_0000) != 0;
        if !has_name {
            return None;
        }

        let name_word_offset = (attributes & 0x00FF_FFFF) as usize;
        let name_offset = names_offset + (name_word_offset * 4);
        let name_end = data.get(name_offset..)?.iter().position(|byte| *byte == 0)?;
        let name = std::str::from_utf8(data.get(name_offset..name_offset + name_end)?).ok()?.to_string();

        let file_start = read_u32_le(data, node_offset + 0x08)? as usize;
        let file_end = read_u32_le(data, node_offset + 0x0C)? as usize;
        if data_offset + file_end > data.len() || file_end < file_start {
            return None;
        }

        entries.push(SarcEntry {
            name,
            data_start: data_offset + file_start,
            data_end: data_offset + file_end,
        });
    }

    Some((data_offset, entries))
}

fn replace_sarc_member_in_place(
    patched_data: &mut [u8],
    original_data: &[u8],
    member_name: &str,
) -> Result<bool, &'static str> {
    let (_, patched_entries) = parse_sarc(patched_data).ok_or("patched_sarc_parse_failed")?;
    let (_, original_entries) = parse_sarc(original_data).ok_or("original_sarc_parse_failed")?;

    let patched_entry = patched_entries
        .iter()
        .find(|entry| entry.name == member_name)
        .ok_or("patched_member_missing")?;
    let original_entry = original_entries
        .iter()
        .find(|entry| entry.name == member_name)
        .ok_or("original_member_missing")?;

    let patched_len = patched_entry.data_end - patched_entry.data_start;
    let original_len = original_entry.data_end - original_entry.data_start;
    if patched_len != original_len {
        return Err("member_size_mismatch");
    }

    if patched_data[patched_entry.data_start..patched_entry.data_end]
        == original_data[original_entry.data_start..original_entry.data_end]
    {
        return Ok(false);
    }

    patched_data[patched_entry.data_start..patched_entry.data_end]
        .copy_from_slice(&original_data[original_entry.data_start..original_entry.data_end]);
    Ok(true)
}

#[arc_callback]
fn callback_chara_icon_arrangement(hash: u64, data: &mut [u8]) -> Option<usize> {
    let original_capacity = data.len().max(0x0004_0000);
    let mut original_data = vec![0u8; original_capacity];
    let original_size = match load_original_file(hash, original_data.as_mut_slice()) {
        Some(size) => size,
        None => {
            crate::boss_log!(
                "[PB][ArrangementPatch] failed to load original arrangement buffer={}",
                original_capacity
            );
            return None;
        }
    };

    if original_size > data.len() {
        crate::boss_log!(
            "[PB][ArrangementPatch] original arrangement larger than callback buffer output={} buffer={}",
            original_size,
            data.len()
        );
        return None;
    }

    crate::boss_log!(
        "[PB][ArrangementPatch] replaced full chara_icon_arrangement.prc with original file size={}",
        original_size
    );

    data[..original_size].copy_from_slice(&original_data[..original_size]);
    Some(original_size)
}

fn log_sarc_member_diffs(layout_name: &str, patched_data: &[u8], original_data: &[u8]) {
    if !crate::debug::enabled() {
        return;
    }

    let (_, patched_entries) = match parse_sarc(patched_data) {
        Some(parsed) => parsed,
        None => {
            crate::boss_log!(
                "[PB][LayoutPatch] {} could not diff members reason=patched_sarc_parse_failed",
                layout_name
            );
            return;
        }
    };
    let (_, original_entries) = match parse_sarc(original_data) {
        Some(parsed) => parsed,
        None => {
            crate::boss_log!(
                "[PB][LayoutPatch] {} could not diff members reason=original_sarc_parse_failed",
                layout_name
            );
            return;
        }
    };

    let mut diff_count = 0usize;
    for patched_entry in patched_entries.iter() {
        let Some(original_entry) = original_entries
            .iter()
            .find(|entry| entry.name == patched_entry.name) else {
            continue;
        };
        let patched_bytes = &patched_data[patched_entry.data_start..patched_entry.data_end];
        let original_bytes = &original_data[original_entry.data_start..original_entry.data_end];
        if patched_bytes != original_bytes {
            diff_count += 1;
            if diff_count <= 24 {
                crate::boss_log!(
                    "[PB][LayoutPatch] {} diff member={} patched_size={} original_size={}",
                    layout_name,
                    patched_entry.name,
                    patched_bytes.len(),
                    original_bytes.len()
                );
            }
        }
    }

    crate::boss_log!(
        "[PB][LayoutPatch] {} total differing members={}",
        layout_name,
        diff_count
    );
}

fn patch_layout_arc_members(hash: u64, data: &mut [u8], layout_name: &str) -> Option<usize> {
    let relative_path = match layout_name {
        "chara_select/layout.arc" => "ui/layout/menu/chara_select/chara_select/layout.arc",
        "compe_chara_select/layout.arc" => "ui/layout/menu/compe_chara_select/compe_chara_select/layout.arc",
        _ => return None,
    };
    let modded_path = find_mod_asset_path(relative_path)?;
    let mut modded_data = fs::read(&modded_path).ok()?;

    let mut original_data = vec![0u8; data.len()];
    let original_size = load_original_file(hash, original_data.as_mut_slice())?;
    let original_data = &original_data[..original_size];

    log_sarc_member_diffs(layout_name, modded_data.as_slice(), original_data);

    let target_members = [
        "blyt/chara_select_btn0_00.bflyt",
        "blyt/com_lct_chara_hd.bflyt",
    ];

    let mut patched_members = 0usize;
    for member_name in target_members {
        match replace_sarc_member_in_place(modded_data.as_mut_slice(), original_data, member_name) {
            Ok(true) => {
                patched_members += 1;
                crate::boss_log!(
                    "[PB][LayoutPatch] {} replaced member {} from original layout",
                    layout_name,
                    member_name
                );
            }
            Ok(false) => {
                crate::boss_log!(
                    "[PB][LayoutPatch] {} member {} already matched original",
                    layout_name,
                    member_name
                );
            }
            Err(reason) => {
                crate::boss_log!(
                    "[PB][LayoutPatch] {} could not replace member {} reason={}",
                    layout_name,
                    member_name,
                    reason
                );
            }
        }
    }

    if patched_members == 0 {
        return None;
    }

    if modded_data.len() > data.len() {
        crate::boss_log!(
            "[PB][LayoutPatch] {} patched file larger than callback buffer output={} buffer={}",
            layout_name,
            modded_data.len(),
            data.len()
        );
        return None;
    }

    data[..modded_data.len()].copy_from_slice(&modded_data);
    Some(modded_data.len())
}

#[arc_callback]
fn callback_chara_select_layout(hash: u64, data: &mut [u8]) -> Option<usize> {
    patch_layout_arc_members(hash, data, "chara_select/layout.arc")
}

#[arc_callback]
fn callback_compe_chara_select_layout(hash: u64, data: &mut [u8]) -> Option<usize> {
    patch_layout_arc_members(hash, data, "compe_chara_select/layout.arc")
}

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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 15;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_koopag", 0x18E);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 117;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 86;
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
        if *hash == to_hash40("ui_series_id") {
            *param.try_into_mut::<Hash40>().unwrap() = to_hash40("ui_series_smashbros");
        }
    });
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_masterhand", 0x160);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 118;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 87;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_crazyhand", 0x169);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 119;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 88;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_darz", 0x19A);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 120;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 89;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_kiila", 0x18F);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 121;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 90;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_marx", 0x180);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 122;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 91;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_ganonboss", 0x172);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 123;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 92;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_dracula", 0x175);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 124;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 93;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_galleom", 0x16F);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 125;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 94;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_lioleus", 0x188);
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
        if use_builtin_css_ordering() && *hash == to_hash40("disp_order") {
            *param.try_into_mut::<i8>().unwrap() = 126;
        }
        if use_builtin_css_ordering() && *hash == to_hash40("skill_list_order") {
            *param.try_into_mut::<i8>().unwrap() = 95;
        }
        if *hash == to_hash40("save_no") {
            *param.try_into_mut::<i8>().unwrap() = 0;
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
    apply_mario_redirect_metadata(charroot);
    patch_css_selector_fields(charroot, "ui_chara_mewtwo_masterhand", 0x1A6);
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

// ARCropolis callback buffer needs to be >= the largest patched PRC in load order.
// Logs showed ui_chara_db.prc around 0x9D3280, so keep comfortable headroom.
const MAX_FILE_SIZE: usize = 0x00C00000;
const MENU_PARAM_FILE_SIZE: usize = 0x00010000;
const MENU_LAYOUT_FILE_SIZE: usize = 0x00800000;

fn use_builtin_css_ordering() -> bool {
    !CONFIG.options.custom_css.unwrap_or(false)
}

#[skyline::main(name = "comp_boss")]
pub fn main() {
    let cfg = &*CONFIG;
    let opts = &cfg.options;
    let giga_bowser_normal = !opts.giga_bowser_normal.unwrap_or(false);
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

    Agent::new("fighter").on_line(Main, boss_final_guard_frame).install();
    Agent::new("mario").on_line(Main, mario_boss_dispatch_frame).install();
    selection::install();
    callback_chara_icon_arrangement::install("ui/param/menu/chara_icon_arrangement.prc", MENU_PARAM_FILE_SIZE);
    callback_chara_select_layout::install("ui/layout/menu/chara_select/chara_select/layout.arc", MENU_LAYOUT_FILE_SIZE);
    callback_compe_chara_select_layout::install("ui/layout/menu/compe_chara_select/compe_chara_select/layout.arc", MENU_LAYOUT_FILE_SIZE);

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

    if final2_stage { callback_map_1::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if final3_stage { callback_map_2::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if ganon_stage { callback_map_3::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if rathalos_stage { callback_map_4::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if marx_stage { callback_map_5::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if galleom_stage { callback_map_6::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
    if dracula_stage { callback_map_7::install("ui/param/database/ui_stage_db.prc", MAX_FILE_SIZE); }
}

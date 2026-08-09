use smash::app::lua_bind::*;
use smash::app::sv_battle_object;
use smash::app::BattleObjectModuleAccessor;
use smash::lib::lua_const::*;

use crate::boss_helpers;
use crate::config::CONFIG;

const MAX_FIGHTERS: usize = 8;
const SAMPLE_PERIOD: u32 = 60;
const FP_COMPARE_SAMPLE_PERIOD: u32 = 4;
const FP_COMPARE_WINDOW_SAMPLES: u32 = 60;

static mut SAMPLE_TICKS: [u32; MAX_FIGHTERS] = [0; MAX_FIGHTERS];
static mut LAST_BOSS_ID: [u32; MAX_FIGHTERS] = [u32::MAX; MAX_FIGHTERS];
static mut LAST_UI_HASH: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];
static mut LAST_FP_SIGNATURE: [u64; MAX_FIGHTERS] = [u64::MAX; MAX_FIGHTERS];

#[derive(Copy, Clone)]
struct FpCommandSummary {
    initialized: bool,
    signature: u64,
    selected_hash: u64,
    boss_object_id: u32,
    boss_kind: i32,
    operation_cpu: bool,
    host_kind: i32,
    fighter_category: u64,
    summon_boss_id: u64,
    samples: u32,
    nonzero_command_samples: u32,
    stick_samples: u32,
    attack_events: u32,
    special_events: u32,
    guard_events: u32,
    jump_events: u32,
    unique_cat1: i32,
    unique_cat2: i32,
    unique_cat3: i32,
    unique_cat4: i32,
    previous_attack: bool,
    previous_special: bool,
    previous_guard: bool,
    previous_jump: bool,
}

impl FpCommandSummary {
    const fn empty() -> Self {
        Self {
            initialized: false,
            signature: u64::MAX,
            selected_hash: 0,
            boss_object_id: 0,
            boss_kind: -1,
            operation_cpu: false,
            host_kind: -1,
            fighter_category: 0,
            summon_boss_id: 0,
            samples: 0,
            nonzero_command_samples: 0,
            stick_samples: 0,
            attack_events: 0,
            special_events: 0,
            guard_events: 0,
            jump_events: 0,
            unique_cat1: 0,
            unique_cat2: 0,
            unique_cat3: 0,
            unique_cat4: 0,
            previous_attack: false,
            previous_special: false,
            previous_guard: false,
            previous_jump: false,
        }
    }
}

static mut FP_COMPARE_TICKS: [u32; MAX_FIGHTERS] = [0; MAX_FIGHTERS];
static mut FP_COMPARE_SUMMARIES: [FpCommandSummary; MAX_FIGHTERS] =
    [FpCommandSummary::empty(); MAX_FIGHTERS];

#[inline(always)]
unsafe fn is_known_boss_item_kind(kind: i32) -> bool {
    kind == *ITEM_KIND_MASTERHAND
        || kind == *ITEM_KIND_PLAYABLE_MASTERHAND
        || kind == *ITEM_KIND_CRAZYHAND
        || kind == *ITEM_KIND_DRACULA2
        || kind == *ITEM_KIND_DRACULA
        || kind == *ITEM_KIND_DARZ
        || kind == *ITEM_KIND_KIILA
        || kind == *ITEM_KIND_MARX
        || kind == *ITEM_KIND_GANONBOSS
        || kind == *ITEM_KIND_GALLEOM
        || kind == *ITEM_KIND_LIOLEUS
        || kind == *ITEM_KIND_LIOLEUSBOSS
}

#[inline(always)]
unsafe fn should_sample(entry: usize, boss_id: u32, ui_hash: u64) -> bool {
    let entry = entry.min(MAX_FIGHTERS - 1);
    let changed = LAST_BOSS_ID[entry] != boss_id || LAST_UI_HASH[entry] != ui_hash;
    let tick = SAMPLE_TICKS[entry];
    SAMPLE_TICKS[entry] = tick.wrapping_add(1);
    LAST_BOSS_ID[entry] = boss_id;
    LAST_UI_HASH[entry] = ui_hash;
    changed || tick % SAMPLE_PERIOD == 0
}

#[inline(always)]
unsafe fn selected_ui_hash(module_accessor: *mut BattleObjectModuleAccessor) -> u64 {
    crate::selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0)
}

#[inline(always)]
fn selected_boss_name(ui_hash: u64) -> &'static str {
    crate::amiibo::BOSS_IDENTITIES
        .iter()
        .find(|identity| crate::to_hash40(identity.ui_chara_id).0 == ui_hash)
        .map(|identity| identity.name)
        .unwrap_or("<none-or-unknown>")
}

#[inline(always)]
unsafe fn known_boss_context(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> (u32, i32) {
    if module_accessor.is_null() {
        return (0, -1);
    }

    for slot in 0..4 {
        if !ItemModule::is_have_item(module_accessor, slot) {
            continue;
        }
        let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
        if item_id == 0 || !sv_battle_object::is_active(item_id) {
            continue;
        }
        let item_boma = sv_battle_object::module_accessor(item_id);
        if item_boma.is_null() {
            continue;
        }
        let item_kind = smash::app::utility::get_kind(&mut *item_boma);
        if is_known_boss_item_kind(item_kind) {
            return (item_id, item_kind);
        }
    }

    (0, -1)
}

#[inline(always)]
unsafe fn emit_fp_compare_summary(entry: usize, summary: FpCommandSummary, reason: &str) {
    if !summary.initialized || summary.samples == 0 || !crate::debug::enabled() {
        return;
    }

    crate::boss_log!(
        "[PB][FPCompare] reason={} entry={} selected_ui_hash=0x{:010x} boss_object_id=0x{:x} boss_kind={} operation_cpu={} host_kind={} fighter_category=0x{:x} summon_boss_id=0x{:x} candidate_player_kind=unavailable candidate_controller_kind=unavailable candidate_ai_kind=unavailable samples={} nonzero_command_samples={} stick_samples={} attack_events={} special_events={} guard_events={} jump_events={} unique_cat1=0x{:x} unique_cat2=0x{:x} unique_cat3=0x{:x} unique_cat4=0x{:x}",
        reason,
        entry,
        summary.selected_hash,
        summary.boss_object_id,
        summary.boss_kind,
        summary.operation_cpu,
        summary.host_kind,
        summary.fighter_category,
        summary.summon_boss_id,
        summary.samples,
        summary.nonzero_command_samples,
        summary.stick_samples,
        summary.attack_events,
        summary.special_events,
        summary.guard_events,
        summary.jump_events,
        summary.unique_cat1,
        summary.unique_cat2,
        summary.unique_cat3,
        summary.unique_cat4
    );
}

#[inline(always)]
unsafe fn observe_fp_commands(
    host: *mut BattleObjectModuleAccessor,
    selected_hash: u64,
    boss_object_id: u32,
    boss_kind: i32,
    operation_cpu: bool,
    fighter_category: u64,
    summon_boss_id: u64,
    host_kind: i32,
    command_cat1: i32,
    command_cat2: i32,
    command_cat3: i32,
    command_cat4: i32,
    stick_x: f32,
    stick_y: f32,
    attack: bool,
    special: bool,
    guard: bool,
    jump: bool,
) {
    if host.is_null() || !crate::debug::enabled() {
        return;
    }

    let entry = boss_helpers::entry_id(host).min(MAX_FIGHTERS - 1);
    let signature = selected_hash
        ^ (boss_object_id as u64).rotate_left(11)
        ^ (boss_kind as i64 as u64).rotate_left(17)
        ^ (operation_cpu as u64).rotate_left(23)
        ^ fighter_category.rotate_left(29)
        ^ summon_boss_id.rotate_left(37)
        ^ (host_kind as i64 as u64).rotate_left(43);

    if !FP_COMPARE_SUMMARIES[entry].initialized
        || FP_COMPARE_SUMMARIES[entry].signature != signature
    {
        let previous = FP_COMPARE_SUMMARIES[entry];
        emit_fp_compare_summary(entry, previous, "context_change");
        FP_COMPARE_SUMMARIES[entry] = FpCommandSummary {
            initialized: true,
            signature,
            selected_hash,
            boss_object_id,
            boss_kind,
            operation_cpu,
            host_kind,
            fighter_category,
            summon_boss_id,
            ..FpCommandSummary::empty()
        };
        FP_COMPARE_TICKS[entry] = 0;
    }

    let tick = FP_COMPARE_TICKS[entry];
    FP_COMPARE_TICKS[entry] = tick.wrapping_add(1);
    if !operation_cpu || tick % FP_COMPARE_SAMPLE_PERIOD != 0 {
        return;
    }

    let summary = &mut FP_COMPARE_SUMMARIES[entry];
    summary.samples += 1;
    summary.unique_cat1 |= command_cat1;
    summary.unique_cat2 |= command_cat2;
    summary.unique_cat3 |= command_cat3;
    summary.unique_cat4 |= command_cat4;

    let stick_active = stick_x.abs() > 0.05 || stick_y.abs() > 0.05;
    if stick_active {
        summary.stick_samples += 1;
    }
    if command_cat1 != 0
        || command_cat2 != 0
        || command_cat3 != 0
        || command_cat4 != 0
        || stick_active
        || attack
        || special
        || guard
        || jump
    {
        summary.nonzero_command_samples += 1;
    }
    if attack && !summary.previous_attack {
        summary.attack_events += 1;
    }
    if special && !summary.previous_special {
        summary.special_events += 1;
    }
    if guard && !summary.previous_guard {
        summary.guard_events += 1;
    }
    if jump && !summary.previous_jump {
        summary.jump_events += 1;
    }
    summary.previous_attack = attack;
    summary.previous_special = special;
    summary.previous_guard = guard;
    summary.previous_jump = jump;

    if summary.samples >= FP_COMPARE_WINDOW_SAMPLES {
        let completed = *summary;
        emit_fp_compare_summary(entry, completed, "window");
        FP_COMPARE_SUMMARIES[entry] = FpCommandSummary {
            initialized: true,
            signature,
            selected_hash,
            boss_object_id,
            boss_kind,
            operation_cpu,
            host_kind,
            fighter_category,
            summon_boss_id,
            ..FpCommandSummary::empty()
        };
    }
}

/// Records only stable control-state transitions. The public bindings expose
/// the operation-CPU bit and command readers, but no native Figure Player
/// discriminator, FP level, or learning-state API. Keep those fields explicit
/// rather than treating every operation CPU as an amiibo.
#[inline(always)]
unsafe fn log_fp_transition(
    host: *mut BattleObjectModuleAccessor,
    selected_hash: u64,
    boss_object_id: u32,
    boss_kind: i32,
    operation_cpu: Option<bool>,
    fighter_category: u64,
    summon_boss_id: u64,
    host_kind: i32,
    host_status: i32,
    command_cat1: i32,
    command_cat2: i32,
    command_cat3: i32,
    command_cat4: i32,
    stick_x: f32,
    stick_y: f32,
    attack: bool,
    special: bool,
    guard: bool,
    jump: bool,
) {
    if host.is_null() || !crate::debug::enabled() {
        return;
    }

    let entry = boss_helpers::entry_id(host).min(MAX_FIGHTERS - 1);
    let operation_cpu_value = operation_cpu.unwrap_or(false);
    let mut signature = selected_hash
        ^ (boss_object_id as u64).rotate_left(11)
        ^ (boss_kind as i64 as u64).rotate_left(17)
        ^ (fighter_category).rotate_left(23)
        ^ (summon_boss_id).rotate_left(29)
        ^ (host_kind as i64 as u64).rotate_left(37)
        ^ (host_status as i64 as u64).rotate_left(43);
    signature ^= (operation_cpu_value as u64) << 7;
    if operation_cpu.is_none() {
        signature ^= 1 << 8;
    }

    if LAST_FP_SIGNATURE[entry] == signature {
        return;
    }
    LAST_FP_SIGNATURE[entry] = signature;

    let control_source = match operation_cpu {
        Some(false) => "human",
        Some(true) => "cpu_or_figure_player",
        None => "unknown",
    };
    let host_cpu_state = match operation_cpu {
        Some(value) => value.to_string(),
        None => "unknown".to_string(),
    };

    crate::boss_log!(
        "[PB][FP] entry={} boss={} selected_ui_hash=0x{:010x} boss_object_id=0x{:x} boss_kind={} control_source={} fp_detected=unavailable operation_cpu={} host_kind={} host_status={} fighter_category=0x{:x} summon_boss_id=0x{:x} native_ai_mode=unavailable fp_level=unavailable training_state=unavailable figure_id=unavailable save_state=unavailable command_observation=available command_cat1=0x{:x} command_cat2=0x{:x} command_cat3=0x{:x} command_cat4=0x{:x} stick=({:.3},{:.3}) buttons=attack:{} special:{} guard:{} jump:{}",
        entry,
        selected_boss_name(selected_hash),
        selected_hash,
        boss_object_id,
        boss_kind,
        control_source,
        host_cpu_state,
        host_kind,
        host_status,
        fighter_category,
        summon_boss_id,
        command_cat1,
        command_cat2,
        command_cat3,
        command_cat4,
        stick_x,
        stick_y,
        attack,
        special,
        guard,
        jump
    );
}

#[inline(always)]
unsafe fn log_snapshot(
    host: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
    boss_id: u32,
    selected_hash: u64,
    native_fighter_path: bool,
) {
    if !crate::debug::enabled() || host.is_null() {
        return;
    }

    let entry = boss_helpers::entry_id(host).min(MAX_FIGHTERS - 1);
    if !should_sample(entry, boss_id, selected_hash) {
        return;
    }

    let fighter_manager = boss_helpers::fighter_manager();
    let observation = boss_helpers::fighter_ai_observation(fighter_manager, entry);
    let operation_cpu = observation.map(|value| value.operation_cpu);
    let control_source = match operation_cpu {
        Some(false) => "human",
        Some(true) => "cpu_or_figure_player",
        None => "unknown",
    };
    let fighter_category = observation.map(|value| value.fighter_category).unwrap_or(0);
    let summon_boss_id = observation.map(|value| value.summon_boss_id).unwrap_or(0);

    let host_kind = smash::app::utility::get_kind(&mut *host);
    let host_status = StatusModule::status_kind(host);
    let host_cpu_state = match operation_cpu {
        Some(value) => {
            if value {
                "true"
            } else {
                "false"
            }
        }
        None => "unknown",
    };
    let boss_kind = if boss_boma.is_null() {
        -1
    } else {
        smash::app::utility::get_kind(&mut *boss_boma)
    };
    let boss_status = if boss_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(boss_boma)
    };

    let boss_item_level =
        if !boss_boma.is_null() && !native_fighter_path && is_known_boss_item_kind(boss_kind) {
            Some(WorkModule::get_float(
                boss_boma,
                *ITEM_INSTANCE_WORK_FLOAT_LEVEL,
            ))
        } else {
            None
        };
    let configured_difficulty = CONFIG.options.boss_difficulty;
    let difficulty_source = match operation_cpu {
        Some(true) if native_fighter_path => "native_fighter_ai_unverified",
        Some(true) => "config_or_boss_native",
        Some(false) => "host_input",
        None => "unknown",
    };

    let cat1 = ControlModule::get_command_flag_cat(host, 0);
    let cat2 = ControlModule::get_command_flag_cat(host, 1);
    let cat3 = ControlModule::get_command_flag_cat(host, 2);
    let cat4 = ControlModule::get_command_flag_cat(host, 3);
    let stick_x = ControlModule::get_stick_x(host);
    let stick_y = ControlModule::get_stick_y(host);
    let attack = ControlModule::check_button_on(host, *CONTROL_PAD_BUTTON_ATTACK);
    let special = ControlModule::check_button_on(host, *CONTROL_PAD_BUTTON_SPECIAL);
    let guard = ControlModule::check_button_on(host, *CONTROL_PAD_BUTTON_GUARD);
    let jump = ControlModule::check_button_on(host, *CONTROL_PAD_BUTTON_JUMP);

    log_fp_transition(
        host,
        selected_hash,
        boss_id,
        boss_kind,
        operation_cpu,
        fighter_category,
        summon_boss_id,
        host_kind,
        host_status,
        cat1,
        cat2,
        cat3,
        cat4,
        stick_x,
        stick_y,
        attack,
        special,
        guard,
        jump,
    );

    crate::boss_log!(
        "[PB][AIAuthority] entry={} selected_ui_hash=0x{:010x} control_source={} host_cpu_state={} host_kind={} host_status={} fighter_category=0x{:x} summon_boss_id=0x{:x} boss_id=0x{:x} boss_kind={} boss_status={} native_fighter_path={} configured_boss_difficulty={:?} effective_boss_difficulty={:?} difficulty_source={} native_cpu_level=unavailable native_ai_intensity=unavailable fp_state=unobservable command_cat1=0x{:x} command_cat2=0x{:x} command_cat3=0x{:x} command_cat4=0x{:x} stick=({:.3},{:.3}) buttons=attack:{} special:{} guard:{} jump:{}",
        entry,
        selected_hash,
        control_source,
        host_cpu_state,
        host_kind,
        host_status,
        fighter_category,
        summon_boss_id,
        boss_id,
        boss_kind,
        boss_status,
        native_fighter_path,
        configured_difficulty,
        boss_item_level,
        difficulty_source,
        cat1,
        cat2,
        cat3,
        cat4,
        stick_x,
        stick_y,
        attack,
        special,
        guard,
        jump
    );
}

/// Samples the hidden host and the active boss item without changing control.
/// The CPU branch intentionally remains untouched until a native Figure Player
/// discriminator is available.
pub unsafe fn log_item_host(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }

    for slot in 0..4 {
        if !ItemModule::is_have_item(module_accessor, slot) {
            continue;
        }
        let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
        if item_id == 0 || !sv_battle_object::is_active(item_id) {
            continue;
        }
        let boss_boma = sv_battle_object::module_accessor(item_id);
        if boss_boma.is_null() {
            continue;
        }
        let boss_kind = smash::app::utility::get_kind(&mut *boss_boma);
        if is_known_boss_item_kind(boss_kind) {
            log_snapshot(
                module_accessor,
                boss_boma,
                item_id,
                selected_ui_hash(module_accessor),
                false,
            );
            return;
        }
    }
}

/// Samples ordinary fighter entries as a comparison control for the hardware
/// trace. This intentionally reports only observable state; it does not infer
/// Figure Player status from `is_operation_cpu`.
pub unsafe fn log_fighter_control_state(
    module_accessor: *mut BattleObjectModuleAccessor,
) {
    if module_accessor.is_null() || !crate::debug::enabled() {
        return;
    }

    let entry = boss_helpers::entry_id(module_accessor).min(MAX_FIGHTERS - 1);
    let observation = boss_helpers::fighter_ai_observation(boss_helpers::fighter_manager(), entry);
    let Some(observation) = observation else {
        return;
    };

    let cat1 = ControlModule::get_command_flag_cat(module_accessor, 0);
    let cat2 = ControlModule::get_command_flag_cat(module_accessor, 1);
    let cat3 = ControlModule::get_command_flag_cat(module_accessor, 2);
    let cat4 = ControlModule::get_command_flag_cat(module_accessor, 3);
    let stick_x = ControlModule::get_stick_x(module_accessor);
    let stick_y = ControlModule::get_stick_y(module_accessor);
    let attack = ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_ATTACK);
    let special = ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_SPECIAL);
    let guard = ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_GUARD);
    let jump = ControlModule::check_button_on(module_accessor, *CONTROL_PAD_BUTTON_JUMP);
    let host_kind = smash::app::utility::get_kind(&mut *module_accessor);
    let host_status = StatusModule::status_kind(module_accessor);
    let selected_hash = selected_ui_hash(module_accessor);
    let (boss_object_id, boss_kind) = known_boss_context(module_accessor);

    observe_fp_commands(
        module_accessor,
        selected_hash,
        boss_object_id,
        boss_kind,
        observation.operation_cpu,
        observation.fighter_category,
        observation.summon_boss_id,
        host_kind,
        cat1,
        cat2,
        cat3,
        cat4,
        stick_x,
        stick_y,
        attack,
        special,
        guard,
        jump,
    );

    // Item-backed hosts are included in FPCompare, while the less frequent
    // transition log remains owned by log_item_host to avoid duplicate lines.
    if boss_object_id != 0 {
        return;
    }

    log_fp_transition(
        module_accessor,
        selected_hash,
        0,
        -1,
        Some(observation.operation_cpu),
        observation.fighter_category,
        observation.summon_boss_id,
        host_kind,
        host_status,
        cat1,
        cat2,
        cat3,
        cat4,
        stick_x,
        stick_y,
        attack,
        special,
        guard,
        jump,
    );
}

/// Samples the dedicated Giga Bowser fighter path. It has no boss item level,
/// so the effective difficulty is intentionally reported as unavailable.
pub unsafe fn log_native_fighter(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null()
        || !crate::debug::enabled()
        || smash::app::utility::get_kind(&mut *module_accessor) != *FIGHTER_KIND_KOOPAG
    {
        return;
    }

    log_snapshot(
        module_accessor,
        module_accessor,
        0,
        selected_ui_hash(module_accessor),
        true,
    );
}

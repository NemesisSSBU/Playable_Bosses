#![allow(dead_code)]
use skyline::nn::ro::LookupSymbol;
use smash::app::lua_bind::*;
use smash::app::sv_battle_object;
use smash::app::BattleObjectModuleAccessor;
use smash::app::FighterEntryID;
use smash::app::FighterManager;
use smash::app::ItemKind;
use smash::lib::lua_const::*;
use smash::phx::Hash40;
use smash::phx::Vector3f;

static mut FIGHTER_MANAGER_ADDR: usize = 0;
static mut LAST_TRANSITION_BLOCK_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut LAST_STATUS_TRACE_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut LAST_SCENE_PHASE_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut LAST_PRE_GO_SIGNATURE: [u64; 8] = [u64::MAX; 8];
static mut LAST_NATIVE_DRIFT_STATUS: [i32; 8] = [i32::MIN; 8];
static mut LAST_NATIVE_DRIFT_READY_GO: [u8; 8] = [0xff; 8];
static mut PRE_GO_ACQUIRED_TRAIT_FLAG: [i32; 8] = [i32::MIN; 8];
static mut LAST_CATEGORY_PROBE_SIGNATURE: u64 = u64::MAX;
static mut BOSS_MARIO_HOST_LATCH: [bool; 8] = [false; 8];

pub const HIDDEN_HOST_SCALE: f32 = 0.0001;
pub const HIDDEN_HOST_ENTRY_PREP_SCALE: f32 = 0.001;
pub const HIDDEN_HOST_ENTRY_STAGE2_SCALE: f32 = 0.002;
pub const HIDDEN_HOST_BASELINE_SCALE: f32 = 0.008;
const HIDDEN_HOST_ENTRY_PREP_EPSILON: f32 = 0.00005;
const HIDDEN_HOST_BASELINE_EPSILON: f32 = 0.0005;

pub const STAGE_ID_BOSS_PREVIEW: i32 = 0x139;
pub const STAGE_ID_CLASSIC_BONUS_GAME: i32 = 0x13A;
pub const STAGE_ID_CLASSIC_STAFFROLL: i32 = 0x13C;
pub const STAGE_ID_AMIIBO_PREVIEW: i32 = 0x135;
pub const STAGE_ID_RESULT: i32 = 0x136;

#[inline(always)]
unsafe fn transition_block_log_once(
    operation: u64,
    entry: usize,
    value_a: u32,
    value_b: u32,
) -> bool {
    if !crate::debug::enabled() {
        return false;
    }
    let entry = entry.min(7);
    let signature = operation
        ^ ((entry as u64) << 8)
        ^ ((value_a as u64) << 24)
        ^ ((value_b as u64).rotate_left(17));
    if LAST_TRANSITION_BLOCK_SIGNATURE[entry] == signature {
        return false;
    }
    LAST_TRANSITION_BLOCK_SIGNATURE[entry] = signature;
    true
}

#[inline(always)]
unsafe fn reset_transition_block_log(entry: usize) {
    LAST_TRANSITION_BLOCK_SIGNATURE[entry.min(7)] = u64::MAX;
}

#[inline(always)]
pub unsafe fn entry_id(module_accessor: *mut BattleObjectModuleAccessor) -> usize {
    if module_accessor.is_null() {
        return 0;
    }
    WorkModule::get_int(module_accessor, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID) as usize
}

#[inline(always)]
pub unsafe fn fighter_manager() -> *mut FighterManager {
    if FIGHTER_MANAGER_ADDR == 0 {
        LookupSymbol(
            &raw mut FIGHTER_MANAGER_ADDR,
            "_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E\0"
                .as_bytes()
                .as_ptr(),
        );
    }
    if FIGHTER_MANAGER_ADDR == 0 {
        return std::ptr::null_mut();
    }
    *(FIGHTER_MANAGER_ADDR as *mut *mut FighterManager)
}

#[inline(always)]
pub unsafe fn fighter_information_entry(
    fighter_manager: *mut FighterManager,
    entry_id: usize,
) -> *mut smash::app::FighterInformation {
    if fighter_manager.is_null() {
        return std::ptr::null_mut();
    }
    smash::app::lua_bind::FighterManager::get_fighter_information(
        fighter_manager,
        FighterEntryID(entry_id as i32),
    )
}

#[inline(always)]
pub unsafe fn is_operation_cpu_entry(
    fighter_manager: *mut FighterManager,
    entry_id: usize,
) -> bool {
    // This is only the operation-CPU bit. It is NOT a Figure Player test.
    //
    // Audited against the pinned bindings: they expose no FP/NFP/amiibo/
    // FigurePlayer symbol of any kind, and `FighterInformation` exposes
    // exactly one operation predicate (`is_operation_cpu`) with no level,
    // personality, or learning-state accessor. An ordinary CPU and a Figure
    // Player are therefore indistinguishable at this boundary.
    //
    // Consequence, recorded deliberately rather than papered over: every boss
    // spawn writes `BOSS_DIFFICULTY` into the boss item's
    // `ITEM_INSTANCE_WORK_FLOAT_LEVEL` unconditionally, so an FP-backed boss
    // receives the configured difficulty rather than its own amiibo level.
    // That cannot be fixed by gating on this predicate — gating on it would
    // catch FPs too. Input authority IS preserved (the plugin only sets
    // `CONTROLLABLE` when this returns false, so Nintendo keeps driving CPU
    // and FP hosts). Do not invent an amiibo-level -> difficulty mapping to
    // close the gap; no source-backed relationship exists.
    let info = fighter_information_entry(fighter_manager, entry_id);
    !info.is_null() && FighterInformation::is_operation_cpu(info)
}

/// The current public bindings expose the operation-CPU bit, but do not expose
/// whether that CPU is an ordinary CPU or a Figure Player. Keep that ambiguity
/// explicit instead of treating every CPU entry as an amiibo.
#[derive(Copy, Clone)]
pub struct FighterAiObservation {
    pub operation_cpu: bool,
    pub fighter_category: u64,
    pub summon_boss_id: u64,
}

#[inline(always)]
pub unsafe fn fighter_ai_observation(
    fighter_manager: *mut FighterManager,
    entry_id: usize,
) -> Option<FighterAiObservation> {
    let info = fighter_information_entry(fighter_manager, entry_id);
    if info.is_null() {
        return None;
    }

    Some(FighterAiObservation {
        operation_cpu: FighterInformation::is_operation_cpu(info),
        fighter_category: FighterInformation::fighter_category(info),
        summon_boss_id: FighterInformation::summon_boss_id(info),
    })
}

#[inline(always)]
pub unsafe fn stock_count_entry(fighter_manager: *mut FighterManager, entry_id: usize) -> u64 {
    let info = fighter_information_entry(fighter_manager, entry_id);
    if info.is_null() {
        return 0;
    }
    FighterInformation::stock_count(info)
}

/// One line per change of the scene-phase tuple, with boss/decoy liveness.
///
/// The open question on the crash is whether this plugin ever gets a frame
/// between "the match stopped" and "the scene tore down". If the callback simply
/// stops being invoked, no teardown we install can possibly run, and the fix has
/// to move somewhere that is still being called. This makes that sequence
/// visible: roughly five lines per match rather than one per frame.
pub unsafe fn trace_scene_phase(
    tag: &str,
    entry: usize,
    ready_go: bool,
    result_mode: bool,
    post_match: bool,
    boss_id: u32,
    decoy_id: u32,
    host_boma: *mut BattleObjectModuleAccessor,
) {
    if !crate::debug::enabled() {
        return;
    }
    // Both boss modules run this callback on the same Mario host, so the module
    // that does not own this boss reports zeroed ids and the two alternate every
    // frame. Only the owning module logs.
    if boss_id == 0 && decoy_id == 0 {
        return;
    }
    let entry = entry.min(7);
    let stage_id = smash::app::stage::get_stage_id();
    let boss_active = boss_id != 0 && sv_battle_object::is_active(boss_id);
    let decoy_active = decoy_id != 0 && sv_battle_object::is_active(decoy_id);
    let host_status = if host_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(host_boma)
    };

    let signature = (ready_go as u64)
        ^ (result_mode as u64).rotate_left(3)
        ^ (post_match as u64).rotate_left(6)
        ^ (boss_active as u64).rotate_left(9)
        ^ (decoy_active as u64).rotate_left(12)
        ^ ((stage_id as u32 as u64) << 16)
        ^ ((host_status as u32 as u64).rotate_left(40));
    if LAST_SCENE_PHASE_SIGNATURE[entry] == signature {
        return;
    }
    LAST_SCENE_PHASE_SIGNATURE[entry] = signature;

    crate::boss_log!(
        "[PB][ScenePhase] tag={} entry={} stage=0x{:x} ready_go={} result_mode={} post_match={} host_status={} boss_id=0x{:x} boss_active={} decoy_id=0x{:x} decoy_active={}",
        tag,
        entry,
        stage_id,
        ready_go,
        result_mode,
        post_match,
        host_status,
        boss_id,
        boss_active,
        decoy_id,
        decoy_active
    );
}

/// Resolves every roster entry whose `fighter_category` is non-zero back to the
/// object its `summon_boss_id` points at.
///
/// The result roster keeps showing an entry with `fighter_category=0x5` and a
/// live `summon_boss_id`, and nothing so far has identified what that object
/// actually is. Naming its item kind says whether it is the boss, the hidden
/// decoy, or a third object the game created on its own -- which decides where
/// the crash fix belongs. Battle-only by design: resolving arbitrary object ids
/// during the result quarantine is exactly what that quarantine forbids.
pub unsafe fn log_boss_category_entries(tag: &str, fighter_manager: *mut FighterManager) {
    if !crate::debug::enabled() || fighter_manager.is_null() {
        return;
    }
    if !smash::app::sv_information::is_ready_go() {
        return;
    }

    let mut signature = 0u64;
    let mut findings = [(0usize, 0u64, 0u32, false, -1i32); 8];
    let mut count = 0usize;
    for entry in 0..8usize {
        let Some(obs) = fighter_ai_observation(fighter_manager, entry) else {
            continue;
        };
        if obs.fighter_category == 0 {
            continue;
        }
        // 0x50000000 is the "no summon" sentinel the roster logger already prints.
        let id = obs.summon_boss_id as u32;
        let resolvable = id != 0 && id != 0x50000000 && sv_battle_object::is_active(id);
        let kind = if resolvable {
            let boma = sv_battle_object::module_accessor(id);
            if boma.is_null() {
                -1
            } else {
                smash::app::utility::get_kind(&mut *boma)
            }
        } else {
            -1
        };
        findings[count] = (entry, obs.fighter_category, id, resolvable, kind);
        signature ^= ((entry as u64) << 1)
            ^ obs.fighter_category.rotate_left(8)
            ^ (id as u64).rotate_left(20)
            ^ ((kind as u32 as u64).rotate_left(44));
        count += 1;
    }

    if count == 0 || LAST_CATEGORY_PROBE_SIGNATURE == signature {
        return;
    }
    LAST_CATEGORY_PROBE_SIGNATURE = signature;

    for finding in findings.iter().take(count) {
        crate::boss_log!(
            "[PB][CategoryProbe] tag={} entry={} fighter_category=0x{:x} summon_boss_id=0x{:x} resolvable={} resolved_item_kind={}",
            tag,
            finding.0,
            finding.1,
            finding.2,
            finding.3,
            finding.4
        );
    }
}

/// Traces the boss object's own status/motion machine, one line per change.
///
/// This exists to settle why the Galeem/Dharkon entrance does not play. Unlike
/// the other bosses, these two are created several frames BEFORE Ready-Go and
/// held inert, then handed `FOR_BOSS_START` at Ready-Go. If the status machine
/// refuses that request from the held/inert state, the request silently does
/// nothing and the boss simply pops into its idle -- exactly the reported
/// symptom. Logging status before and after the request distinguishes
/// "request rejected" from "request accepted then overwritten".
pub unsafe fn trace_boss_status(
    tag: &str,
    entry: usize,
    boss_boma: *mut BattleObjectModuleAccessor,
    host_boma: *mut BattleObjectModuleAccessor,
) {
    if !crate::debug::enabled() || boss_boma.is_null() {
        return;
    }
    let entry = entry.min(7);
    let status = StatusModule::status_kind(boss_boma);
    let prev_status = StatusModule::prev_status_kind(boss_boma, 0);
    let motion = MotionModule::motion_kind(boss_boma);
    let frame = MotionModule::frame(boss_boma);
    let scale = ModelModule::scale(boss_boma);
    let host_scale = if host_boma.is_null() {
        -1.0
    } else {
        ModelModule::scale(host_boma)
    };

    // Deliberately excludes the motion frame: including it produced one log line
    // per frame and buried the transitions this is meant to show.
    let signature = (status as u32 as u64)
        ^ (prev_status as u32 as u64).rotate_left(11)
        ^ motion.rotate_left(23)
        ^ ((scale * 10000.0) as i32 as u64).rotate_left(47);
    if LAST_STATUS_TRACE_SIGNATURE[entry] == signature {
        return;
    }
    LAST_STATUS_TRACE_SIGNATURE[entry] = signature;

    crate::boss_log!(
        "[PB][StatusTrace] tag={} entry={} status={} prev_status={} motion=0x{:x} frame={:.1} boss_scale={:.4} host_scale={:.4} ready_go={}",
        tag,
        entry,
        status,
        prev_status,
        motion,
        frame,
        scale,
        host_scale,
        smash::app::sv_information::is_ready_go()
    );
}

/// Same as [`acquire_boss_item`], but skips an item the host is already
/// holding. Galeem and Dharkon park a hidden decoy in slot 0, so the freshly
/// created boss has to be located by scanning past it rather than assuming
/// slot 0. Rebuilt on `acquire_boss_item` so it keeps the transition-quarantine
/// guard; the pre-quarantine version in git history does not have it.
pub unsafe fn acquire_boss_item_excluding(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *mut [u32; 8],
    item_kind: i32,
    excluded_item_id: u32,
) -> *mut BattleObjectModuleAccessor {
    if module_accessor.is_null() || slot_ids.is_null() {
        return std::ptr::null_mut();
    }
    let entry = entry_id(module_accessor);
    if crate::should_quarantine_boss_frame(module_accessor) {
        if crate::debug::enabled()
            && transition_block_log_once(2, entry, item_kind as u32, excluded_item_id)
        {
            crate::boss_log!(
                "[PB][BossItem] acquire_blocked reason=transition_quarantine entry={} requested_kind={} excluded=0x{:x} stage=0x{:x}",
                entry,
                item_kind,
                excluded_item_id,
                smash::app::stage::get_stage_id()
            );
        }
        return std::ptr::null_mut();
    }
    if crate::debug::enabled() {
        reset_transition_block_log(entry);
    }
    ItemModule::have_item(module_accessor, ItemKind(item_kind), 0, 0, false, false);
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);
    let mut boss_id = 0u32;
    for slot in 0..4 {
        if ItemModule::is_have_item(module_accessor, slot) {
            let candidate = ItemModule::get_have_item_id(module_accessor, slot) as u32;
            if candidate != 0 && candidate != excluded_item_id {
                boss_id = candidate;
                break;
            }
        }
    }
    if boss_id == 0 {
        boss_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
    }
    let boss_boma = if boss_id != 0 && sv_battle_object::is_active(boss_id) {
        sv_battle_object::module_accessor(boss_id)
    } else {
        std::ptr::null_mut()
    };
    (*slot_ids)[entry] = if boss_boma.is_null() { 0 } else { boss_id };
    ensure_boss_item_visible(boss_boma);
    if crate::debug::enabled() {
        let boss_kind = if boss_boma.is_null() {
            -1
        } else {
            smash::app::utility::get_kind(&mut *boss_boma)
        };
        crate::boss_log!(
            "[PB][BossItem] acquire_excluding entry={} requested_kind={} excluded=0x{:x} acquired_id=0x{:x} acquired_kind={} stage=0x{:x}",
            entry,
            item_kind,
            excluded_item_id,
            boss_id,
            boss_kind,
            smash::app::stage::get_stage_id()
        );
    }
    boss_boma
}

#[inline(always)]
pub unsafe fn acquire_boss_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *mut [u32; 8],
    item_kind: i32,
) -> *mut BattleObjectModuleAccessor {
    if module_accessor.is_null() || slot_ids.is_null() {
        return std::ptr::null_mut();
    }
    let entry = entry_id(module_accessor);
    if crate::should_quarantine_boss_frame(module_accessor) {
        if crate::debug::enabled() && transition_block_log_once(1, entry, item_kind as u32, 0) {
            crate::boss_log!(
                "[PB][BossItem] acquire_blocked reason=transition_quarantine entry={} requested_kind={} stage=0x{:x}",
                entry,
                item_kind,
                smash::app::stage::get_stage_id()
            );
        }
        return std::ptr::null_mut();
    }
    if crate::debug::enabled() {
        reset_transition_block_log(entry);
        crate::boss_log!(
            "[PB][BossItem] before_have_item entry={} requested_kind={} stage=0x{:x}",
            entry,
            item_kind,
            smash::app::stage::get_stage_id()
        );
    }
    ItemModule::have_item(module_accessor, ItemKind(item_kind), 0, 0, false, false);
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);
    let boss_id = ItemModule::get_have_item_id(module_accessor, 0) as u32;
    let boss_boma = if boss_id != 0 && sv_battle_object::is_active(boss_id) {
        sv_battle_object::module_accessor(boss_id)
    } else {
        std::ptr::null_mut()
    };
    (*slot_ids)[entry] = if boss_boma.is_null() { 0 } else { boss_id };
    ensure_boss_item_visible(boss_boma);
    if crate::debug::enabled() {
        let fighter_status = StatusModule::status_kind(module_accessor);
        let boss_kind = if boss_boma.is_null() {
            -1
        } else {
            smash::app::utility::get_kind(&mut *boss_boma)
        };
        let mut slot_ids_debug = [0u32; 4];
        let mut slot_kinds_debug = [-1i32; 4];
        for slot in 0..4 {
            if ItemModule::is_have_item(module_accessor, slot) {
                let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
                slot_ids_debug[slot as usize] = item_id;
                if item_id != 0 && sv_battle_object::is_active(item_id) {
                    let item_boma = sv_battle_object::module_accessor(item_id);
                    if !item_boma.is_null() {
                        slot_kinds_debug[slot as usize] =
                            smash::app::utility::get_kind(&mut *item_boma);
                    }
                }
            }
        }
        crate::boss_log!(
            "[PB][BossItem] acquire entry={} requested_kind={} acquired_id=0x{:x} acquired_kind={} stage=0x{:x} fighter_status={} scale={:.4} slots={:?} slot_kinds={:?}",
            entry,
            item_kind,
            boss_id,
            boss_kind,
            smash::app::stage::get_stage_id(),
            fighter_status,
            ModelModule::scale(module_accessor),
            slot_ids_debug,
            slot_kinds_debug
        );
    }
    boss_boma
}

#[inline(always)]
pub unsafe fn held_item_by_kind(
    module_accessor: *mut BattleObjectModuleAccessor,
    expected_kinds: &[i32],
) -> Option<(i32, u32, *mut BattleObjectModuleAccessor)> {
    if module_accessor.is_null() {
        return None;
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
        if expected_kinds.contains(&item_kind) {
            return Some((slot, item_id, item_boma));
        }
    }
    None
}

#[inline(always)]
pub unsafe fn tracked_item_by_kind(
    slot_ids: *const [u32; 8],
    entry: usize,
    expected_kind: i32,
) -> Option<(u32, *mut BattleObjectModuleAccessor)> {
    if slot_ids.is_null() {
        return None;
    }

    let item_id = (*slot_ids)[entry.min(7)];
    if item_id == 0 || !sv_battle_object::is_active(item_id) {
        return None;
    }

    let item_boma = sv_battle_object::module_accessor(item_id);
    if item_boma.is_null() || smash::app::utility::get_kind(&mut *item_boma) != expected_kind {
        return None;
    }

    Some((item_id, item_boma))
}

#[inline(always)]
pub const fn should_discard_tracked_boss(
    ready_go: bool,
    transition_quarantined: bool,
    expected_kind_active: bool,
    staged_boss_prepared: bool,
) -> bool {
    !transition_quarantined && (!expected_kind_active || (!ready_go && !staged_boss_prepared))
}

#[inline(always)]
pub const fn staged_boss_ready_for_activation(
    staged_boss_prepared: bool,
    expected_kind_active: bool,
    activation_attempted: bool,
    hidden_helper_active: bool,
) -> bool {
    staged_boss_prepared && expected_kind_active && !activation_attempted && hidden_helper_active
}

#[inline(always)]
pub unsafe fn maintain_nonbattle_boss_presentation(item_boma: *mut BattleObjectModuleAccessor) {
    if item_boma.is_null() {
        return;
    }

    AttackModule::clear_all(item_boma);
    HitModule::set_whole(item_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
    DamageModule::set_damage_lock(item_boma, true);
    JostleModule::set_status(item_boma, false);
}

/// Galeem and Dharkon are authored facing the camera, so the object's own `lr`
/// has to be converted into an explicit yaw or they render side-on. Gameplay
/// facing only: do not call this (or `face_boss_along_host_lr`) while the
/// staged intro is active. Hold the posture captured before WAIT instead.
/// An `lr` that is neither of the two cardinal values is left untouched.
#[inline(always)]
pub unsafe fn face_boss_along_lr(boss_boma: *mut BattleObjectModuleAccessor) {
    if boss_boma.is_null() {
        return;
    }

    let yaw = match PostureModule::lr(boss_boma) {
        // Dharkon hardware 2026-08-14: previous mapping (90 / -90) was 180 off.
        lr if lr == -1.0 => -90.0,
        lr if lr == 1.0 => 90.0,
        _ => return,
    };
    let rot = Vector3f {
        x: 0.0,
        y: yaw,
        z: 0.0,
    };
    PostureModule::set_rot(boss_boma, &rot, 0);
}

/// Copy the host's facing onto the boss, then apply the camera-authored yaw.
#[inline(always)]
pub unsafe fn face_boss_along_host_lr(
    boss_boma: *mut BattleObjectModuleAccessor,
    host: *mut BattleObjectModuleAccessor,
) {
    if boss_boma.is_null() {
        return;
    }
    if !host.is_null() {
        PostureModule::set_lr(boss_boma, PostureModule::lr(host));
        PostureModule::update_rot_y_lr(boss_boma);
    }
    face_boss_along_lr(boss_boma);
}

#[inline(always)]
pub fn is_generic_held_item_status(status: i32) -> bool {
    status == *ITEM_STATUS_KIND_HAVE || status == *ITEM_STATUS_KIND_INITIALIZE
}

/// Unwanted auto-attacks native GO can hand off into. `ITEM_STATUS_KIND_ENTRY`
/// and `ITEM_STATUS_KIND_FOR_BOSS_START` are the same value (`0x3D` / 61) and
/// must not be forced to WAIT — that is the boss-start machine we keep the
/// pre-GO object out of, not an attack to yank after GO.
/// Intercept Dharkon Pierce and Galeem Static Missile only.
#[inline(always)]
pub fn is_kiila_darz_first_attack_status(status: i32) -> bool {
    status == *ITEM_DARZ_STATUS_KIND_PIERCE_START
        || status == *ITEM_DARZ_STATUS_KIND_PIERCE_LOOP
        || status == *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START
        || status == *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_LOOP
        || status == *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_END
}

#[inline(always)]
pub fn should_intercept_kiila_darz_spawn_status(status: i32) -> bool {
    is_kiila_darz_first_attack_status(status)
}

#[inline(always)]
pub fn is_kiila_darz_intro_motion(motion: u64) -> bool {
    motion == smash::hash40("entry") || motion == smash::hash40("entry2")
}

#[inline(always)]
pub unsafe fn overlay_kiila_darz_entry(boss_boma: *mut BattleObjectModuleAccessor, frame: f32) {
    if boss_boma.is_null() {
        return;
    }
    MotionModule::change_motion(
        boss_boma,
        Hash40::new("entry"),
        frame,
        1.0,
        false,
        0.0,
        false,
        false,
    );
}

#[inline(always)]
pub fn hidden_kiila_darz_cpu_is_quarantined(active: bool, status: i32) -> bool {
    active && status == *ITEM_STATUS_KIND_NONE
}

#[inline(always)]
pub fn should_force_generic_wait(status: i32) -> bool {
    status != *ITEM_STATUS_KIND_WAIT
}

#[inline(always)]
pub fn item_trait_has_boss(trait_flag: i32) -> bool {
    (trait_flag & *ITEM_TRAIT_FLAG_BOSS) != 0
}

#[inline(always)]
#[allow(dead_code)]
pub fn trait_flag_without_boss(trait_flag: i32) -> i32 {
    trait_flag & !*ITEM_TRAIT_FLAG_BOSS
}

#[inline(always)]
pub unsafe fn reset_pre_go_trait_isolation(entry: usize) {
    PRE_GO_ACQUIRED_TRAIT_FLAG[entry.min(7)] = i32::MIN;
}

#[inline(always)]
pub unsafe fn remember_pre_go_acquired_trait(entry: usize, acquired: i32) {
    let entry = entry.min(7);
    if PRE_GO_ACQUIRED_TRAIT_FLAG[entry] == i32::MIN {
        PRE_GO_ACQUIRED_TRAIT_FLAG[entry] = acquired;
    }
}

#[inline(always)]
pub unsafe fn pre_go_acquired_trait_flag(entry: usize, staged: i32) -> i32 {
    let stored = PRE_GO_ACQUIRED_TRAIT_FLAG[entry.min(7)];
    if stored == i32::MIN {
        staged
    } else {
        stored
    }
}

/// Do not write a replacement trait. If the native boss bit is already set,
/// clear only that bit. Otherwise leave the field untouched.
/// Unused on the current native-start path; kept for the trait-isolation tests.
#[inline(always)]
#[allow(dead_code)]
pub unsafe fn isolate_pre_go_boss_trait(
    entry: usize,
    boss_boma: *mut BattleObjectModuleAccessor,
) -> (i32, i32, bool) {
    if boss_boma.is_null() {
        return (0, 0, false);
    }
    let current = WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
    remember_pre_go_acquired_trait(entry, current);
    let acquired = pre_go_acquired_trait_flag(entry, current);
    if !item_trait_has_boss(current) {
        return (acquired, current, false);
    }
    WorkModule::set_int(
        boss_boma,
        trait_flag_without_boss(current),
        *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG,
    );
    let staged = WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
    (acquired, staged, true)
}

#[inline(always)]
pub fn generic_item_status_name(status: i32) -> &'static str {
    if status == *ITEM_STATUS_KIND_WAIT {
        "WAIT"
    } else if status == *ITEM_STATUS_KIND_FOR_BOSS_START {
        "FOR_BOSS_START"
    } else if status == *ITEM_STATUS_KIND_START {
        "START"
    } else if status == *ITEM_STATUS_KIND_TRANS_PHASE {
        "TRANS_PHASE"
    } else if status == *ITEM_STATUS_KIND_DEAD {
        "DEAD"
    } else if status == *ITEM_STATUS_KIND_FOR_BOSS_TERM {
        "FOR_BOSS_TERM"
    } else {
        "other"
    }
}

/// One line per distinct non-WAIT status per entry/GO-phase. Call before any
/// WAIT quarantine write so the native drift is visible.
#[inline(always)]
pub unsafe fn log_kiila_darz_native_drift(
    boss: &str,
    entry: usize,
    boss_boma: *mut BattleObjectModuleAccessor,
    ready_go: bool,
) {
    if !crate::debug::enabled() || boss_boma.is_null() {
        return;
    }
    let entry = entry.min(7);
    let observed_status = StatusModule::status_kind(boss_boma);
    if observed_status == *ITEM_STATUS_KIND_WAIT {
        return;
    }
    let ready_go_key = ready_go as u8;
    if LAST_NATIVE_DRIFT_STATUS[entry] == observed_status
        && LAST_NATIVE_DRIFT_READY_GO[entry] == ready_go_key
    {
        return;
    }
    LAST_NATIVE_DRIFT_STATUS[entry] = observed_status;
    LAST_NATIVE_DRIFT_READY_GO[entry] = ready_go_key;
    crate::boss_log!(
        "[PB][KiilaDarzNativeDrift] boss={} entry={} from_status={} observed_status={} observed_generic_status={} variation={} trait_flag={} motion=0x{:x} motion_frame={:.2} ready_go={}",
        boss,
        entry,
        *ITEM_STATUS_KIND_WAIT,
        observed_status,
        generic_item_status_name(observed_status),
        WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_VARIATION),
        WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG),
        MotionModule::motion_kind(boss_boma),
        MotionModule::frame(boss_boma),
        ready_go
    );
}

#[inline(always)]
pub fn should_restore_staged_entry(
    intro_active: bool,
    is_intro_motion: bool,
    status_force_performed: bool,
) -> bool {
    intro_active && (status_force_performed || !is_intro_motion)
}

#[inline(always)]
pub fn staged_intro_reached_end(
    is_intro_motion: bool,
    motion_is_end: bool,
    actual_frame: f32,
    end_frame: f32,
) -> bool {
    is_intro_motion && (motion_is_end || actual_frame >= end_frame - 0.01)
}

#[derive(Clone, Copy)]
pub struct KiilaDarzStagedIntroTick {
    pub frame: f32,
    pub status_force_performed: bool,
    pub motion_restore_performed: bool,
    pub intro_completed: bool,
}

impl KiilaDarzStagedIntroTick {
    pub const fn idle(frame: f32) -> Self {
        Self {
            frame,
            status_force_performed: false,
            motion_restore_performed: false,
            intro_completed: false,
        }
    }
}

#[inline(always)]
pub unsafe fn capture_item_posture(
    item_boma: *mut BattleObjectModuleAccessor,
) -> (f32, f32, f32, f32) {
    if item_boma.is_null() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (
        PostureModule::lr(item_boma),
        PostureModule::rot_x(item_boma, 0),
        PostureModule::rot_y(item_boma, 0),
        PostureModule::rot_z(item_boma, 0),
    )
}

#[inline(always)]
pub unsafe fn hold_item_posture(
    item_boma: *mut BattleObjectModuleAccessor,
    lr: f32,
    rot_x: f32,
    rot_y: f32,
    rot_z: f32,
) {
    if item_boma.is_null() {
        return;
    }
    PostureModule::set_lr(item_boma, lr);
    let rot = Vector3f {
        x: rot_x,
        y: rot_y,
        z: rot_z,
    };
    PostureModule::set_rot(item_boma, &rot, 0);
}

#[inline(always)]
pub unsafe fn ensure_generic_wait_status(item_boma: *mut BattleObjectModuleAccessor) -> bool {
    if item_boma.is_null() {
        return false;
    }
    if !should_force_generic_wait(StatusModule::status_kind(item_boma)) {
        return true;
    }
    StatusModule::change_status_force(item_boma, *ITEM_STATUS_KIND_WAIT, false);
    StatusModule::status_kind(item_boma) == *ITEM_STATUS_KIND_WAIT
}

/// Pre-GO invariant: generic WAIT + authored `entry`.
/// Force WAIT and restore `entry` only on the edge where native code drifted.
/// While status is already WAIT and motion is already `entry`/`entry2`, do not
/// write status or motion. `MotionModule::frame()` is the staged intro clock.
#[inline(always)]
pub unsafe fn quarantine_kiila_darz_wait_with_entry(
    boss_boma: *mut BattleObjectModuleAccessor,
    intro_active: bool,
    intro_frame: f32,
) -> KiilaDarzStagedIntroTick {
    if boss_boma.is_null() {
        return KiilaDarzStagedIntroTick::idle(intro_frame);
    }

    let status = StatusModule::status_kind(boss_boma);
    let motion = MotionModule::motion_kind(boss_boma);
    let is_intro = is_kiila_darz_intro_motion(motion);
    let actual = MotionModule::frame(boss_boma);
    let end = MotionModule::end_frame(boss_boma);
    if intro_active
        && staged_intro_reached_end(is_intro, MotionModule::is_end(boss_boma), actual, end)
    {
        return KiilaDarzStagedIntroTick {
            frame: actual,
            status_force_performed: false,
            motion_restore_performed: false,
            intro_completed: true,
        };
    }

    let saved_frame = if is_intro { actual } else { intro_frame };
    let status_force_performed = if should_force_generic_wait(status) {
        StatusModule::change_status_force(boss_boma, *ITEM_STATUS_KIND_WAIT, false);
        true
    } else {
        false
    };
    let motion_restore_performed =
        if should_restore_staged_entry(intro_active, is_intro, status_force_performed) {
            overlay_kiila_darz_entry(boss_boma, saved_frame);
            true
        } else {
            false
        };

    let frame = MotionModule::frame(boss_boma);
    let intro_completed = intro_active
        && staged_intro_reached_end(
            is_kiila_darz_intro_motion(MotionModule::motion_kind(boss_boma)),
            MotionModule::is_end(boss_boma),
            frame,
            MotionModule::end_frame(boss_boma),
        );
    KiilaDarzStagedIntroTick {
        frame,
        status_force_performed,
        motion_restore_performed,
        intro_completed,
    }
}

#[inline(always)]
pub unsafe fn hidden_cpu_snapshot(hidden_cpu_id: u32) -> (bool, i32) {
    if hidden_cpu_id == 0 || !sv_battle_object::is_active(hidden_cpu_id) {
        return (false, i32::MIN);
    }
    let hidden_cpu_boma = sv_battle_object::module_accessor(hidden_cpu_id);
    if hidden_cpu_boma.is_null() {
        return (false, i32::MIN);
    }
    (true, StatusModule::status_kind(hidden_cpu_boma))
}

#[inline(always)]
pub unsafe fn manager_snapshot(manager_id: u32) -> (u32, bool, i32) {
    if manager_id == 0 || !sv_battle_object::is_active(manager_id) {
        return (0, false, -1);
    }
    let manager_boma = sv_battle_object::module_accessor(manager_id);
    if manager_boma.is_null() {
        return (0, false, -1);
    }
    (manager_id, true, StatusModule::status_kind(manager_boma))
}

/// Detach the Dracula2 helper produced by `throw_item` and park it in
/// `ITEM_STATUS_KIND_NONE` before the manager or real boss is acquired.
/// Returns false unless the helper is still active and the NONE status lands
/// synchronously.
#[inline(always)]
pub unsafe fn quarantine_hidden_kiila_darz_cpu_before_ready_go(
    host: *mut BattleObjectModuleAccessor,
    hidden_cpu_id: u32,
) -> bool {
    if host.is_null() || hidden_cpu_id == 0 || !sv_battle_object::is_active(hidden_cpu_id) {
        return false;
    }
    release_tracked_item_from_host(host, hidden_cpu_id);
    if !sv_battle_object::is_active(hidden_cpu_id) {
        return false;
    }
    let hidden_cpu_boma = sv_battle_object::module_accessor(hidden_cpu_id);
    if hidden_cpu_boma.is_null() {
        return false;
    }
    ModelModule::set_scale(hidden_cpu_boma, HIDDEN_HOST_SCALE);
    maintain_nonbattle_boss_presentation(hidden_cpu_boma);
    WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_LEVEL);
    WorkModule::set_float(hidden_cpu_boma, 0.0, *ITEM_INSTANCE_WORK_FLOAT_STRENGTH);
    WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP_MAX);
    WorkModule::set_float(hidden_cpu_boma, 999.0, *ITEM_INSTANCE_WORK_FLOAT_HP);
    WorkModule::on_flag(
        hidden_cpu_boma,
        *ITEM_INSTANCE_WORK_FLAG_IGNORE_DELETE_BY_STAGE,
    );
    WorkModule::on_flag(
        hidden_cpu_boma,
        *ITEM_INSTANCE_WORK_FLAG_DISABLE_AUTO_GRAVITY_MOVE,
    );
    pin_item_to_host(host, hidden_cpu_boma);
    if StatusModule::status_kind(hidden_cpu_boma) != *ITEM_STATUS_KIND_NONE {
        StatusModule::change_status_force(hidden_cpu_boma, *ITEM_STATUS_KIND_NONE, false);
    }
    if !sv_battle_object::is_active(hidden_cpu_id) {
        return false;
    }
    hidden_kiila_darz_cpu_is_quarantined(true, StatusModule::status_kind(hidden_cpu_boma))
}

#[inline(always)]
pub unsafe fn log_kiila_darz_pre_go(
    boss: &str,
    entry: usize,
    host: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
    hidden_cpu_id: u32,
    manager_id: u32,
    tick: KiilaDarzStagedIntroTick,
    ready_go: bool,
) {
    if !crate::debug::enabled() || boss_boma.is_null() {
        return;
    }
    let entry = entry.min(7);
    let status = StatusModule::status_kind(boss_boma);
    let motion = MotionModule::motion_kind(boss_boma);
    let motion_frame = MotionModule::frame(boss_boma);
    let motion_end_frame = MotionModule::end_frame(boss_boma);
    let (hidden_cpu_active, hidden_cpu_status) = hidden_cpu_snapshot(hidden_cpu_id);
    let hidden_cpu_quarantined =
        hidden_kiila_darz_cpu_is_quarantined(hidden_cpu_active, hidden_cpu_status);
    let (manager_id, manager_active, manager_status) = manager_snapshot(manager_id);
    let have_item = !host.is_null() && ItemModule::is_have_item(host, 0);
    let kind = smash::app::utility::get_kind(&mut *boss_boma);
    let variation = WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_VARIATION);
    let staged_trait_flag = WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG);
    let acquired_trait_flag = pre_go_acquired_trait_flag(entry, staged_trait_flag);
    let boss_trait_present = item_trait_has_boss(staged_trait_flag);
    let signature = (status as u32 as u64)
        ^ motion.rotate_left(8)
        ^ ((motion_frame as i32) as u64).rotate_left(16)
        ^ ((tick.frame as i32) as u64).rotate_left(24)
        ^ (hidden_cpu_status as u32 as u64).rotate_left(32)
        ^ (manager_status as u32 as u64).rotate_left(40)
        ^ (hidden_cpu_quarantined as u64).rotate_left(48)
        ^ (manager_active as u64).rotate_left(49)
        ^ (have_item as u64).rotate_left(50)
        ^ (tick.status_force_performed as u64).rotate_left(51)
        ^ (tick.motion_restore_performed as u64).rotate_left(52)
        ^ (tick.intro_completed as u64).rotate_left(53)
        ^ (ready_go as u64).rotate_left(54)
        ^ (kind as u32 as u64).rotate_left(4)
        ^ (variation as u32 as u64).rotate_left(12)
        ^ (staged_trait_flag as u32 as u64).rotate_left(20)
        ^ (acquired_trait_flag as u32 as u64).rotate_left(28)
        ^ (boss_trait_present as u64).rotate_left(55);
    if LAST_PRE_GO_SIGNATURE[entry] == signature {
        return;
    }
    LAST_PRE_GO_SIGNATURE[entry] = signature;
    if boss == "dharkon" {
        crate::boss_log!(
            "[PB][KiilaDarzPreGo] boss={} entry={} hidden_cpu_id=0x{:x} hidden_cpu_active={} hidden_cpu_status={} hidden_cpu_quarantined={} manager_id=0x{:x} manager_active={} manager_status={} boss_kind={} variation={} acquired_trait_flag={} staged_trait_flag={} boss_trait_present={} trait_flag={} boss_status={} motion=0x{:x} actual_motion_frame={:.2} motion_end_frame={:.2} staged_intro_frame={:.2} status_force_performed={} motion_restore_performed={} intro_completed={} have_item={} ready_go={}",
            boss,
            entry,
            hidden_cpu_id,
            hidden_cpu_active,
            hidden_cpu_status,
            hidden_cpu_quarantined,
            manager_id,
            manager_active,
            manager_status,
            kind,
            variation,
            acquired_trait_flag,
            staged_trait_flag,
            boss_trait_present,
            staged_trait_flag,
            status,
            motion,
            motion_frame,
            motion_end_frame,
            tick.frame,
            tick.status_force_performed,
            tick.motion_restore_performed,
            tick.intro_completed,
            have_item,
            ready_go
        );
        return;
    }
    crate::boss_log!(
        "[PB][KiilaDarzPreGo] boss={} entry={} hidden_cpu_id=0x{:x} hidden_cpu_active={} hidden_cpu_status={} hidden_cpu_quarantined={} manager_id=0x{:x} manager_active={} manager_status={} boss_kind={} variation={} trait_flag={} boss_status={} motion=0x{:x} actual_motion_frame={:.2} motion_end_frame={:.2} staged_intro_frame={:.2} status_force_performed={} motion_restore_performed={} intro_completed={} have_item={} ready_go={}",
        boss,
        entry,
        hidden_cpu_id,
        hidden_cpu_active,
        hidden_cpu_status,
        hidden_cpu_quarantined,
        manager_id,
        manager_active,
        manager_status,
        kind,
        variation,
        staged_trait_flag,
        status,
        motion,
        motion_frame,
        motion_end_frame,
        tick.frame,
        tick.status_force_performed,
        tick.motion_restore_performed,
        tick.intro_completed,
        have_item,
        ready_go
    );
}

/// First post-GO fighter-frame breadcrumb. Call before any mutation.
/// If this line never appears after a crash, native GO died before our callback.
#[inline(always)]
pub unsafe fn log_kiila_darz_ready_go_first(
    boss: &str,
    entry: usize,
    staged_id: u32,
    selected: bool,
    already_logged: *mut bool,
) {
    if !crate::debug::enabled() || already_logged.is_null() || *already_logged {
        return;
    }
    *already_logged = true;
    let active = staged_id != 0 && sv_battle_object::is_active(staged_id);
    let (status, motion) = if active {
        let boma = sv_battle_object::module_accessor(staged_id);
        if boma.is_null() {
            (-1, 0u64)
        } else {
            (
                StatusModule::status_kind(boma),
                MotionModule::motion_kind(boma),
            )
        }
    } else {
        (-1, 0u64)
    };
    crate::boss_log!(
        "[PB][KiilaDarzReadyGo] edge=first_callback boss={} entry={} selected={} status={} motion=0x{:x} staged_id=0x{:x} active={}",
        boss,
        entry.min(7),
        selected,
        status,
        motion,
        staged_id,
        active
    );
}

#[inline(always)]
pub unsafe fn pin_item_to_host(
    host: *mut BattleObjectModuleAccessor,
    item_boma: *mut BattleObjectModuleAccessor,
) {
    if host.is_null() || item_boma.is_null() {
        return;
    }
    let pos = Vector3f {
        x: PostureModule::pos_x(host),
        y: PostureModule::pos_y(host),
        z: PostureModule::pos_z(host),
    };
    PostureModule::set_pos(item_boma, &pos);
}

#[inline(always)]
pub unsafe fn release_tracked_item_from_host(host: *mut BattleObjectModuleAccessor, item_id: u32) {
    if host.is_null() || item_id == 0 {
        return;
    }
    for slot in 0..4 {
        if ItemModule::is_have_item(host, slot)
            && ItemModule::get_have_item_id(host, slot) as u32 == item_id
        {
            ItemModule::throw_item(host, 0.0, 0.0, 0.0, slot, true, 0.0);
        }
    }
}

/// Vanilla WOL drives Galeem/Dharkon through this coordinator.
///
/// Hardware 2026-08-14: `have_item(KIILADARZMANAGER)` then throw left the
/// manager in THROW (5). `BOSS_SINGLE_WAIT` is 0x41, which is also generic
/// `FOR_BOSS_TERM`. Do not request 0x41 on a have_item-spawned manager until
/// a log proves it lands as wait rather than term. Force generic WAIT so GO
/// does not process a thrown coordinator.
#[inline(always)]
pub unsafe fn configure_hidden_kiila_darz_manager(
    manager_boma: *mut BattleObjectModuleAccessor,
    host: *mut BattleObjectModuleAccessor,
) {
    if manager_boma.is_null() {
        return;
    }
    ModelModule::set_scale(manager_boma, HIDDEN_HOST_SCALE);
    maintain_nonbattle_boss_presentation(manager_boma);
    WorkModule::on_flag(
        manager_boma,
        *ITEM_INSTANCE_WORK_FLAG_IGNORE_DELETE_BY_STAGE,
    );
    WorkModule::on_flag(
        manager_boma,
        *ITEM_INSTANCE_WORK_FLAG_DISABLE_AUTO_GRAVITY_MOVE,
    );
    pin_item_to_host(host, manager_boma);
    let status = StatusModule::status_kind(manager_boma);
    if status == *ITEM_STATUS_KIND_THROW
        || status == *ITEM_STATUS_KIND_HAVE
        || status == *ITEM_STATUS_KIND_FALL
        || is_generic_held_item_status(status)
    {
        StatusModule::change_status_force(manager_boma, *ITEM_STATUS_KIND_WAIT, false);
    }
}

#[inline(always)]
pub unsafe fn maintain_kiila_darz_manager(host: *mut BattleObjectModuleAccessor, manager_id: u32) {
    if manager_id == 0 || !sv_battle_object::is_active(manager_id) {
        return;
    }
    let manager_boma = sv_battle_object::module_accessor(manager_id);
    configure_hidden_kiila_darz_manager(manager_boma, host);
}

/// After Ready-Go only. Yank Pierce / Static Missile, not ENTRY/FOR_BOSS_START.
/// If an intro motion was playing, restore it at the saved frame after WAIT.
#[inline(always)]
pub unsafe fn intercept_kiila_darz_spawn_attack(
    boss_boma: *mut BattleObjectModuleAccessor,
) -> bool {
    if boss_boma.is_null() {
        return false;
    }
    let status = StatusModule::status_kind(boss_boma);
    if !should_intercept_kiila_darz_spawn_status(status) {
        return status == *ITEM_STATUS_KIND_WAIT;
    }
    let motion = MotionModule::motion_kind(boss_boma);
    let frame = MotionModule::frame(boss_boma);
    let was_intro = is_kiila_darz_intro_motion(motion);
    StatusModule::change_status_force(boss_boma, *ITEM_STATUS_KIND_WAIT, false);
    if was_intro {
        overlay_kiila_darz_entry(boss_boma, frame);
    }
    StatusModule::status_kind(boss_boma) == *ITEM_STATUS_KIND_WAIT
}

extern "C" {
    #[link_name = "\u{1}_ZN3app10item_other6removeEPNS_26BattleObjectModuleAccessorE"]
    fn remove_owned_item(module_accessor: *mut BattleObjectModuleAccessor);
}

#[inline(always)]
pub unsafe fn clear_owned_boss_item_slot(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *mut [u32; 8],
    expected_kinds: &[i32],
    set_standby: bool,
) -> bool {
    if module_accessor.is_null() || slot_ids.is_null() {
        return false;
    }

    let entry = entry_id(module_accessor);
    let tracked_id = (*slot_ids)[entry];
    if tracked_id == 0 {
        return false;
    }

    // During native battle teardown the object and owner slot can disappear
    // between callbacks. Drop only plugin bookkeeping here; the game owns
    // destruction and must not be touched through a stale accessor.
    if crate::should_quarantine_boss_frame(module_accessor) {
        (*slot_ids)[entry] = 0;
        if crate::debug::enabled() && transition_block_log_once(4, entry, tracked_id, 0) {
            crate::boss_log!(
                "[PB][BossItem] owned_clear_blocked reason=transition_quarantine entry={} tracked_id=0x{:x} stage=0x{:x}",
                entry,
                tracked_id,
                smash::app::stage::get_stage_id()
            );
        }
        return false;
    }
    if crate::debug::enabled() {
        reset_transition_block_log(entry);
    }

    if !sv_battle_object::is_active(tracked_id) {
        (*slot_ids)[entry] = 0;
        return false;
    }

    let tracked_boma = sv_battle_object::module_accessor(tracked_id);
    if tracked_boma.is_null() {
        (*slot_ids)[entry] = 0;
        return false;
    }

    let tracked_kind = smash::app::utility::get_kind(&mut *tracked_boma);
    if !expected_kinds.contains(&tracked_kind) {
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][BossItem] owned_clear refused entry={} tracked_id=0x{:x} tracked_kind={} expected={:?}",
                entry,
                tracked_id,
                tracked_kind,
                expected_kinds
            );
        }
        return false;
    }

    let mut held_slot = None;
    for slot in 0..4 {
        if ItemModule::is_have_item(module_accessor, slot)
            && ItemModule::get_have_item_id(module_accessor, slot) as u32 == tracked_id
        {
            held_slot = Some(slot);
            break;
        }
    }

    HitModule::set_whole(tracked_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
    if set_standby {
        StatusModule::change_status_request_from_script(
            tracked_boma,
            *ITEM_STATUS_KIND_STANDBY,
            true,
        );
    }

    if let Some(slot) = held_slot {
        ItemModule::remove_item(module_accessor, slot);
    } else {
        // The tracked object may already have left the owner's slot. Remove
        // only that verified object instead of clearing unrelated held items.
        remove_owned_item(tracked_boma);
    }

    (*slot_ids)[entry] = 0;
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][BossItem] owned_clear entry={} tracked_id=0x{:x} tracked_kind={} held_slot={:?} set_standby={} stage=0x{:x}",
            entry,
            tracked_id,
            tracked_kind,
            held_slot,
            set_standby,
            smash::app::stage::get_stage_id()
        );
    }
    true
}

#[inline(always)]
pub unsafe fn is_hidden_host(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    scale_is_hidden_host(ModelModule::scale(module_accessor))
}

/// A real hidden host always sits at one of the deliberate hidden scales
/// (0.0001 host, 0.001/0.002 staged entry) -- never exactly 0. `ModelModule::
/// scale` reports 0.0 on frames where the model is not yet initialised, and the
/// previous unbounded `<=` test accepted that as "hidden". One such frame on an
/// ordinary Mario latched `BOSS_MARIO_HOST_LATCH` for the rest of the match,
/// which runs `stop_all_sound` every frame -- the intermittent "Mario's sound
/// effects and voices go silent" report. Require a positive scale.
#[inline(always)]
fn scale_is_hidden_host(scale: f32) -> bool {
    scale > 0.0 && scale <= HIDDEN_HOST_ENTRY_STAGE2_SCALE
}

/// Boss items created with `have_item` on a hidden Mario host can inherit the
/// 0.0001 host scale. Restore a visible battle scale without touching the host.
#[inline(always)]
pub unsafe fn ensure_boss_item_visible(boss_boma: *mut BattleObjectModuleAccessor) {
    if boss_boma.is_null() {
        return;
    }
    if ModelModule::scale(boss_boma) <= HIDDEN_HOST_ENTRY_STAGE2_SCALE {
        ModelModule::set_scale(boss_boma, 1.0);
    }
    VisibilityModule::set_whole(boss_boma, true);
}

#[inline(always)]
fn within_epsilon(value: f32, expected: f32, epsilon: f32) -> bool {
    (expected - epsilon..=expected + epsilon).contains(&value)
}

#[inline(always)]
pub unsafe fn is_hidden_host_entry_prep(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    within_epsilon(
        scale,
        HIDDEN_HOST_ENTRY_PREP_SCALE,
        HIDDEN_HOST_ENTRY_PREP_EPSILON,
    )
}

#[inline(always)]
pub unsafe fn is_hidden_host_entry_stage_two(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    within_epsilon(
        scale,
        HIDDEN_HOST_ENTRY_STAGE2_SCALE,
        HIDDEN_HOST_ENTRY_PREP_EPSILON,
    )
}

#[inline(always)]
pub unsafe fn is_hidden_host_baseline(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    within_epsilon(
        scale,
        HIDDEN_HOST_BASELINE_SCALE,
        HIDDEN_HOST_BASELINE_EPSILON,
    )
}

#[inline(always)]
pub unsafe fn is_tracked_boss_active(slot_ids: *const [u32; 8], entry: usize) -> bool {
    if slot_ids.is_null() {
        return false;
    }
    let entry = entry.min(7);
    let item_id = (*slot_ids)[entry];
    item_id != 0 && sv_battle_object::is_active(item_id)
}

#[inline(always)]
pub unsafe fn needs_hidden_host_entry_init(
    module_accessor: *mut BattleObjectModuleAccessor,
    slot_ids: *const [u32; 8],
    entry: usize,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    ModelModule::scale(module_accessor) > HIDDEN_HOST_ENTRY_PREP_SCALE
        || !is_tracked_boss_active(slot_ids, entry)
}

#[inline(always)]
pub unsafe fn clear_hidden_host_effects(module_accessor: *mut BattleObjectModuleAccessor) {
    if is_hidden_host(module_accessor) {
        EffectModule::kill_all(module_accessor, 0, false, false);
    }
}

#[inline(always)]
pub unsafe fn stop_hidden_host_mario_result_sfx(module_accessor: *mut BattleObjectModuleAccessor) {
    if !is_hidden_host(module_accessor) {
        return;
    }
    SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_05"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_013"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_common_swing_09"), 0);
    SoundModule::stop_se(
        module_accessor,
        Hash40::new("se_common_punch_kick_swing_l"),
        0,
    );
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win02"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_mario_win2"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_014"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_win03"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("vc_mario_015"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_mario_jump01"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("se_mario_landing02"), 0);
}

const MARIO_STAMINA_KNOCKOUT_VOICE: &str = "vc_mario_knockout";

#[inline(always)]
unsafe fn stop_mario_death_voice_hashes(module_accessor: *mut BattleObjectModuleAccessor) {
    const DEATH_VOICE_HASHES: &[&str] = &[
        MARIO_STAMINA_KNOCKOUT_VOICE,
        "seq_mario_rnd_knockout",
        "seq_mario_rnd_dead",
        "seq_mario_rnd_furafura",
        "death",
        "dead",
        "hp_battle_damage_reaction",
        "hp_battle_knockout_dead_frame",
        "hp_battle_knockout_reaction",
        "hp_battle_knockout_slow_frame",
        "hp_battle_knockout_slow_mag",
        "vc_mario_damage01",
        "vc_mario_damage02",
        "vc_mario_damagefly01",
        "vc_mario_damagefly02",
        "vc_mario_ottotto",
        "vc_mario_furafura",
        "vc_mario_missfoot01",
        "vc_mario_missfoot02",
        "vc_mario_001",
        "vc_mario_002",
        "vc_mario_005",
        "vc_mario_006",
        "vc_mario_007",
        "se_mario_damage_s",
        "se_mario_damage_m",
        "se_mario_damage_l",
        "se_mario_down",
        "se_common_blowaway_s",
        "se_common_blowaway_m",
        "se_common_blowaway_l",
        "se_common_spirits_damage",
        "se_common_spirits_end",
    ];
    for hash in DEATH_VOICE_HASHES {
        SoundModule::stop_se(module_accessor, Hash40::new(hash), 0);
    }
}

#[inline(always)]
pub unsafe fn mark_boss_mario_host(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() {
        return;
    }
    BOSS_MARIO_HOST_LATCH[entry_id(module_accessor).min(7)] = true;
}

#[inline(always)]
pub unsafe fn is_marked_boss_mario_host(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    !module_accessor.is_null() && BOSS_MARIO_HOST_LATCH[entry_id(module_accessor).min(7)]
}

#[inline(always)]
pub unsafe fn clear_boss_mario_host_latch(entry: usize) {
    if entry < 8 {
        BOSS_MARIO_HOST_LATCH[entry] = false;
    }
}

/// Death can reset the hidden-host scale before Mario's KO scream finishes, so
/// a latch keeps muting through DEAD/STANDBY. Visible living statuses must not
/// inherit that mute — training Mario after WOL Master Hand is the hardware
/// case, where the previous match's latch otherwise runs `stop_all_sound`
/// every frame.
#[inline(always)]
pub fn is_mario_death_audio_status(status: i32) -> bool {
    status == *FIGHTER_STATUS_KIND_DEAD || status == *FIGHTER_STATUS_KIND_STANDBY
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BossMarioHostAudioDecision {
    /// Hidden host, or latched through the death window after scale resets.
    Suppress,
    /// Previous host latch is still set, but this Mario is visibly alive.
    ReleaseLatch,
    None,
}

#[inline(always)]
pub fn boss_mario_host_audio_decision(
    hidden_now: bool,
    latched: bool,
    death_audio_status: bool,
) -> BossMarioHostAudioDecision {
    if hidden_now {
        BossMarioHostAudioDecision::Suppress
    } else if latched && death_audio_status {
        BossMarioHostAudioDecision::Suppress
    } else if latched {
        BossMarioHostAudioDecision::ReleaseLatch
    } else {
        BossMarioHostAudioDecision::None
    }
}

#[inline(always)]
pub unsafe fn stop_hidden_host_knockout_sfx(module_accessor: *mut BattleObjectModuleAccessor) {
    if !is_hidden_host(module_accessor) {
        return;
    }
    stop_mario_death_voice_hashes(module_accessor);
}

/// Mute Mario's stamina/stock KO scream on a boss host. Scale is not required:
/// death can reset the hidden-host scale before the voice finishes.
#[inline(always)]
pub unsafe fn suppress_boss_mario_death_voice(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() {
        return;
    }
    stop_mario_death_voice_hashes(module_accessor);
    SoundModule::stop_status_se(module_accessor);
    SoundModule::stop_all_sound(module_accessor);
}

#[inline(always)]
pub unsafe fn restore_plain_mario_visuals(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() {
        return;
    }

    clear_hidden_host_effects(module_accessor);
    stop_hidden_host_mario_result_sfx(module_accessor);
    stop_hidden_host_knockout_sfx(module_accessor);
    HitModule::set_whole(
        module_accessor,
        smash::app::HitStatus(*HIT_STATUS_NORMAL),
        0,
    );
    JostleModule::set_status(module_accessor, true);
    VisibilityModule::set_whole(module_accessor, true);
    ModelModule::set_scale(module_accessor, 1.0);

    let reset_rot = Vector3f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    PostureModule::set_rot(module_accessor, &reset_rot, 0);

    let reset_joint_rot = Vector3f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    ModelModule::set_joint_rotate(
        module_accessor,
        Hash40::new("root"),
        &reset_joint_rot,
        smash::app::MotionNodeRotateCompose {
            _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
        },
        ModelModule::rotation_order(module_accessor),
    );
    // Same-frame restore: mario_boss_dispatch_frame mutes both before and after
    // this call. Drop the host latch once Mario is visibly the fighter again so
    // the second `stop_all_sound` does not mute the rest of the match.
    clear_boss_mario_host_latch(entry_id(module_accessor).min(7));
}

#[inline(always)]
pub unsafe fn request_hidden_host_stock_drain(
    module_accessor: *mut BattleObjectModuleAccessor,
    fighter_manager: *mut FighterManager,
    entry_id: usize,
    stop: *mut bool,
) {
    if module_accessor.is_null() || stop.is_null() {
        return;
    }

    let status = StatusModule::status_kind(module_accessor);
    // Request the native death transition once.  Reasserting DEAD while the
    // engine has already advanced the hidden host into REBIRTH or STANDBY can
    // prevent FighterManager from completing its normal stock/elimination
    // bookkeeping, which is especially harmful to Galeem/Dharkon in Regular
    // Smash.  Native status processing owns the remainder of the lifecycle.
    if status != *FIGHTER_STATUS_KIND_DEAD
        && status != *FIGHTER_STATUS_KIND_REBIRTH
        && status != *FIGHTER_STATUS_KIND_STANDBY
    {
        StatusModule::change_status_request_from_script(
            module_accessor,
            *FIGHTER_STATUS_KIND_DEAD,
            true,
        );
        suppress_boss_mario_death_voice(module_accessor);
    }
    if stock_count_entry(fighter_manager, entry_id) == 0 {
        *stop = true;
    }
}

#[inline(always)]
pub unsafe fn clamp_flying_boss_floor(
    module_accessor: *mut BattleObjectModuleAccessor,
    boss_boma: *mut BattleObjectModuleAccessor,
    clearance: f32,
) {
    if module_accessor.is_null() || boss_boma.is_null() {
        return;
    }
    let boss_pos = Vector3f {
        x: PostureModule::pos_x(boss_boma),
        y: PostureModule::pos_y(boss_boma),
        z: PostureModule::pos_z(boss_boma),
    };
    let probe_pos = Vector3f {
        x: boss_pos.x,
        y: boss_pos.y + 120.0,
        z: boss_pos.z,
    };
    let probe_dist =
        GroundModule::get_distance_to_floor(module_accessor, &probe_pos, probe_pos.y, true);
    if probe_dist > 0.0 && probe_dist < 400.0 {
        let floor_y = probe_pos.y - probe_dist;
        let clamped_y = floor_y + clearance;
        if boss_pos.y < clamped_y {
            let clamped = Vector3f {
                x: boss_pos.x,
                y: clamped_y,
                z: boss_pos.z,
            };
            PostureModule::set_pos(module_accessor, &clamped);
            PostureModule::set_pos(boss_boma, &clamped);
        }
    }
}

#[inline(always)]
pub fn is_boss_preview_stage(stage_id: i32) -> bool {
    // These scenes use the preview/interstitial boss presentation path.
    stage_id == STAGE_ID_BOSS_PREVIEW
        || stage_id == STAGE_ID_CLASSIC_STAFFROLL
        || stage_id == STAGE_ID_AMIIBO_PREVIEW
}

/// World of Light constructs its preview host after the UI selection callbacks.
/// Keep this distinct from the other preview scenes so boss modules can defer
/// runtime setup until that host is actually valid.
#[inline(always)]
pub fn is_world_of_light_boss_preview_stage(stage_id: i32) -> bool {
    stage_id == STAGE_ID_BOSS_PREVIEW
}

#[inline(always)]
pub fn is_boss_passthrough_stage(stage_id: i32) -> bool {
    // These scenes should stay on the base fighter because the boss takeover
    // path is not playable there.
    stage_id == STAGE_ID_CLASSIC_BONUS_GAME
}

#[inline(always)]
pub fn is_boss_nonbattle_stage(stage_id: i32) -> bool {
    is_boss_preview_stage(stage_id) || is_boss_passthrough_stage(stage_id)
}

#[cfg(test)]
mod tests {
    use super::{
        boss_mario_host_audio_decision, generic_item_status_name,
        hidden_kiila_darz_cpu_is_quarantined, is_kiila_darz_first_attack_status,
        is_mario_death_audio_status, item_trait_has_boss, scale_is_hidden_host,
        should_discard_tracked_boss, should_force_generic_wait,
        should_intercept_kiila_darz_spawn_status, should_restore_staged_entry,
        staged_boss_ready_for_activation, staged_intro_reached_end, trait_flag_without_boss,
        BossMarioHostAudioDecision, HIDDEN_HOST_ENTRY_PREP_SCALE, HIDDEN_HOST_ENTRY_STAGE2_SCALE,
        HIDDEN_HOST_SCALE, MARIO_STAMINA_KNOCKOUT_VOICE,
    };
    use smash::lib::lua_const::*;

    #[test]
    fn inherited_hidden_host_scale_is_treated_as_an_invisible_boss_item() {
        assert!(HIDDEN_HOST_SCALE <= HIDDEN_HOST_ENTRY_STAGE2_SCALE);
        assert!(0.0001 <= HIDDEN_HOST_ENTRY_STAGE2_SCALE);
        assert!(0.08 > HIDDEN_HOST_ENTRY_STAGE2_SCALE);
    }

    #[test]
    fn stamina_knockout_uses_the_dedicated_mario_knockout_voice() {
        assert_eq!(MARIO_STAMINA_KNOCKOUT_VOICE, "vc_mario_knockout");
    }

    #[test]
    fn battle_tracking_requires_the_expected_kind_and_an_explicit_preparation_phase() {
        assert!(should_discard_tracked_boss(false, false, true, false));
        assert!(!should_discard_tracked_boss(false, false, true, true));
        assert!(should_discard_tracked_boss(false, false, false, true));
        assert!(should_discard_tracked_boss(true, false, false, false));
        assert!(!should_discard_tracked_boss(true, false, true, false));
        assert!(!should_discard_tracked_boss(false, true, true, false));
    }

    #[test]
    fn staged_activation_uses_verified_lifecycle_state_not_a_host_scale_marker() {
        assert!(staged_boss_ready_for_activation(true, true, false, true));
        assert!(!staged_boss_ready_for_activation(false, true, false, true));
        assert!(!staged_boss_ready_for_activation(true, false, false, true));
        assert!(!staged_boss_ready_for_activation(true, true, true, true));
        assert!(!staged_boss_ready_for_activation(true, true, false, false));
    }

    #[test]
    fn status_54_is_generic_wait_not_entry() {
        assert_eq!(*ITEM_STATUS_KIND_WAIT, 0x36);
        assert_eq!(*ITEM_STATUS_KIND_WAIT, 54);
        assert_eq!(*ITEM_STATUS_KIND_ENTRY, 0x3D);
        assert_eq!(*ITEM_STATUS_KIND_FOR_BOSS_START, 0x3D);
        assert_eq!(*ITEM_STATUS_KIND_ENTRY, *ITEM_STATUS_KIND_FOR_BOSS_START);
        assert_ne!(*ITEM_STATUS_KIND_WAIT, *ITEM_STATUS_KIND_ENTRY);
    }

    #[test]
    fn throw_is_status_5_and_none_is_the_inert_terminal() {
        assert_eq!(*ITEM_STATUS_KIND_THROW, 5);
        assert_eq!(*ITEM_STATUS_KIND_NONE, -1);
    }

    #[test]
    fn uninitialised_model_scale_is_not_a_hidden_host() {
        // The regression: 0.0 (model not yet initialised) used to pass the
        // unbounded `<=` test and latched a plain Mario as a boss audio host.
        assert!(!scale_is_hidden_host(0.0));
        assert!(!scale_is_hidden_host(-1.0));
        // Ordinary fighters, including shrunk ones, stay out.
        assert!(!scale_is_hidden_host(1.0));
        assert!(!scale_is_hidden_host(0.5));
        assert!(!scale_is_hidden_host(
            HIDDEN_HOST_ENTRY_STAGE2_SCALE + 0.001
        ));
        // Every deliberate hidden-host scale still qualifies.
        assert!(scale_is_hidden_host(HIDDEN_HOST_SCALE));
        assert!(scale_is_hidden_host(HIDDEN_HOST_ENTRY_PREP_SCALE));
        assert!(scale_is_hidden_host(HIDDEN_HOST_ENTRY_STAGE2_SCALE));
    }

    #[test]
    fn boss_mario_audio_latch_does_not_follow_a_visible_living_mario() {
        assert_eq!(
            boss_mario_host_audio_decision(true, false, false),
            BossMarioHostAudioDecision::Suppress
        );
        assert_eq!(
            boss_mario_host_audio_decision(false, true, true),
            BossMarioHostAudioDecision::Suppress
        );
        // WOL Master Hand → training Mario: leftover latch, full-scale WAIT.
        assert_eq!(
            boss_mario_host_audio_decision(false, true, false),
            BossMarioHostAudioDecision::ReleaseLatch
        );
        assert_eq!(
            boss_mario_host_audio_decision(false, false, false),
            BossMarioHostAudioDecision::None
        );
        assert!(is_mario_death_audio_status(*FIGHTER_STATUS_KIND_DEAD));
        assert!(is_mario_death_audio_status(*FIGHTER_STATUS_KIND_STANDBY));
        assert!(!is_mario_death_audio_status(*FIGHTER_STATUS_KIND_WAIT));
        assert!(!is_mario_death_audio_status(*FIGHTER_STATUS_KIND_ENTRY));
        assert!(!is_mario_death_audio_status(*FIGHTER_STATUS_KIND_REBIRTH));
    }

    fn hidden_cpu_quarantine_requires_active_none() {
        assert!(hidden_kiila_darz_cpu_is_quarantined(
            true,
            *ITEM_STATUS_KIND_NONE
        ));
        assert!(!hidden_kiila_darz_cpu_is_quarantined(
            true,
            *ITEM_STATUS_KIND_THROW
        ));
        assert!(!hidden_kiila_darz_cpu_is_quarantined(
            false,
            *ITEM_STATUS_KIND_NONE
        ));
        assert!(!hidden_kiila_darz_cpu_is_quarantined(
            true,
            *ITEM_STATUS_KIND_WAIT
        ));
    }

    #[test]
    fn wait_and_entry_writes_are_edge_triggered() {
        assert!(!should_force_generic_wait(*ITEM_STATUS_KIND_WAIT));
        assert!(should_force_generic_wait(*ITEM_STATUS_KIND_THROW));
        assert!(should_force_generic_wait(*ITEM_STATUS_KIND_HAVE));
        assert!(should_force_generic_wait(*ITEM_STATUS_KIND_FOR_BOSS_TERM));
        assert!(!should_restore_staged_entry(true, true, false));
        assert!(should_restore_staged_entry(true, false, false));
        assert!(should_restore_staged_entry(true, true, true));
        assert!(!should_restore_staged_entry(false, false, false));
    }

    #[test]
    fn generic_boss_lifecycle_status_names_match_skyline_constants() {
        assert_eq!(*ITEM_STATUS_KIND_WAIT, 54);
        assert_eq!(*ITEM_STATUS_KIND_FOR_BOSS_START, 61);
        assert_eq!(*ITEM_STATUS_KIND_START, 62);
        assert_eq!(*ITEM_STATUS_KIND_TRANS_PHASE, 63);
        assert_eq!(*ITEM_STATUS_KIND_DEAD, 64);
        assert_eq!(*ITEM_STATUS_KIND_FOR_BOSS_TERM, 65);
        assert_eq!(generic_item_status_name(*ITEM_STATUS_KIND_WAIT), "WAIT");
        assert_eq!(
            generic_item_status_name(*ITEM_STATUS_KIND_FOR_BOSS_START),
            "FOR_BOSS_START"
        );
        assert_eq!(generic_item_status_name(*ITEM_STATUS_KIND_START), "START");
        assert_eq!(
            generic_item_status_name(*ITEM_STATUS_KIND_TRANS_PHASE),
            "TRANS_PHASE"
        );
        assert_eq!(generic_item_status_name(*ITEM_STATUS_KIND_DEAD), "DEAD");
        assert_eq!(
            generic_item_status_name(*ITEM_STATUS_KIND_FOR_BOSS_TERM),
            "FOR_BOSS_TERM"
        );
        assert_eq!(generic_item_status_name(0), "other");
    }

    #[test]
    fn darz_offset_is_zero_and_kiila_duet_is_three() {
        assert_eq!(*ITEM_VARIATION_DARZ_OFFSET, 0);
        assert_eq!(*ITEM_VARIATION_DARZ_DARKMAP, 1);
        assert_eq!(*ITEM_VARIATION_DARZ_FINALMAP, 2);
        assert_eq!(*ITEM_VARIATION_DARZ_KIILA, 3);
        assert_eq!(*ITEM_VARIATION_KIILA_DARZ, 3);
        assert_eq!(*ITEM_TRAIT_FLAG_BOSS, 0x1000);
    }

    #[test]
    fn pre_go_trait_isolation_clears_only_the_boss_bit() {
        assert!(!item_trait_has_boss(0));
        assert!(item_trait_has_boss(*ITEM_TRAIT_FLAG_BOSS));
        assert!(item_trait_has_boss(*ITEM_TRAIT_FLAG_BOSS | 1));
        assert_eq!(trait_flag_without_boss(0), 0);
        assert_eq!(trait_flag_without_boss(*ITEM_TRAIT_FLAG_BOSS), 0);
        assert_eq!(trait_flag_without_boss(*ITEM_TRAIT_FLAG_BOSS | 1), 1);
        assert!(!item_trait_has_boss(trait_flag_without_boss(
            *ITEM_TRAIT_FLAG_BOSS | 0x20
        )));
    }

    #[test]
    fn native_boss_start_is_for_boss_start_not_generic_wait() {
        assert_eq!(*ITEM_STATUS_KIND_FOR_BOSS_START, 61);
        assert_eq!(*ITEM_STATUS_KIND_ENTRY, *ITEM_STATUS_KIND_FOR_BOSS_START);
        assert_ne!(*ITEM_STATUS_KIND_FOR_BOSS_START, *ITEM_STATUS_KIND_WAIT);
        assert_eq!(*ITEM_VARIATION_DARZ_KIILA, 3);
    }

    #[test]
    fn staged_intro_completion_uses_live_motion_end() {
        assert!(staged_intro_reached_end(true, false, 80.0, 80.0));
        assert!(staged_intro_reached_end(true, true, 10.0, 80.0));
        assert!(staged_intro_reached_end(true, false, 79.995, 80.0));
        assert!(!staged_intro_reached_end(true, false, 10.0, 80.0));
        assert!(!staged_intro_reached_end(false, true, 80.0, 80.0));
    }

    #[test]
    fn kiila_darz_first_attack_status_is_pierce_and_static_missile_only() {
        assert!(!is_kiila_darz_first_attack_status(
            *ITEM_STATUS_KIND_FOR_BOSS_START
        ));
        assert!(!is_kiila_darz_first_attack_status(*ITEM_STATUS_KIND_ENTRY));
        assert!(is_kiila_darz_first_attack_status(
            *ITEM_DARZ_STATUS_KIND_PIERCE_START
        ));
        assert!(is_kiila_darz_first_attack_status(
            *ITEM_DARZ_STATUS_KIND_PIERCE_LOOP
        ));
        assert!(is_kiila_darz_first_attack_status(
            *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_START
        ));
        assert!(is_kiila_darz_first_attack_status(
            *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_LOOP
        ));
        assert!(is_kiila_darz_first_attack_status(
            *ITEM_KIILA_STATUS_KIND_STATIC_MISSILE_END
        ));
        assert!(!is_kiila_darz_first_attack_status(*ITEM_STATUS_KIND_WAIT));
        assert!(!is_kiila_darz_first_attack_status(
            *ITEM_STATUS_KIND_TRANS_PHASE
        ));
    }

    #[test]
    fn kiila_darz_spawn_intercept_skips_entry_for_boss_start_and_trans_phase() {
        assert!(!should_intercept_kiila_darz_spawn_status(
            *ITEM_STATUS_KIND_FOR_BOSS_START
        ));
        assert!(!should_intercept_kiila_darz_spawn_status(
            *ITEM_STATUS_KIND_ENTRY
        ));
        assert!(should_intercept_kiila_darz_spawn_status(
            *ITEM_DARZ_STATUS_KIND_PIERCE_START
        ));
        assert!(!should_intercept_kiila_darz_spawn_status(
            *ITEM_STATUS_KIND_HAVE
        ));
        assert!(!should_intercept_kiila_darz_spawn_status(
            *ITEM_STATUS_KIND_THROW
        ));
        assert!(!should_intercept_kiila_darz_spawn_status(
            *ITEM_STATUS_KIND_TRANS_PHASE
        ));
        assert!(!should_intercept_kiila_darz_spawn_status(
            *ITEM_STATUS_KIND_WAIT
        ));
        assert!(!should_intercept_kiila_darz_spawn_status(
            *ITEM_STATUS_KIND_DEAD
        ));
    }
}

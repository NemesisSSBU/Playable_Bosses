//! Topology/lifecycle diagnostics for the native Galeem/Dharkon summon states.
//!
//! The pinned public bindings expose the boss status machine, but do not
//! expose a supported fighter creation, retirement, or controller-transfer
//! API. This module therefore never owns the child entry. It only observes the
//! native child and provides a bounded timing guard around the existing native
//! parent-death cleanup path.

use smash::app::lua_bind::{
    FighterEntry as FighterEntryBindings, FighterInformation, FighterManager, MotionModule,
    StatusModule, TeamModule, WorkModule,
};
use smash::app::{sv_battle_object, BattleObjectModuleAccessor, FighterEntry, FighterEntryID};
use smash::lib::lua_const::*;

const MAX_FIGHTERS: usize = 8;
const BOSS_KIND_COUNT: usize = 2;
const MAX_FIGHTER_ENTRY_SLOTS: usize = 8;
// The native FighterInformation category used by the Galeem/Dharkon summon
// path.  This is deliberately a correlation key only: no public binding
// exposes a setter or retirement API for these entries.
const TEMPORARY_BOSS_SUMMON_CATEGORY: u64 = 0x5;

#[derive(Copy, Clone, PartialEq, Eq)]
enum SummonEntryLifecyclePhase {
    Uninitialized,
    NoChild,
    Reserved,
    Spawned,
    Active,
    ExpiredEntryReserved,
    Retiring,
    CancelledByResult,
}

impl SummonEntryLifecyclePhase {
    const fn name(self) -> &'static str {
        match self {
            Self::Uninitialized => "uninitialized",
            Self::NoChild => "no_child",
            Self::Reserved => "reserved",
            Self::Spawned => "spawned",
            Self::Active => "active",
            Self::ExpiredEntryReserved => "expired_entry_reserved",
            Self::Retiring => "retiring",
            Self::CancelledByResult => "cancelled_by_result",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct SummonEntrySnapshot {
    found: bool,
    entry: usize,
    object_id: u64,
    active: bool,
    fighter_kind: i32,
    status: i32,
    operation_cpu: bool,
    team: u64,
    stock: u64,
    fighter_num: u64,
    fighter_category: u64,
    summon_boss_id: u64,
    // -1 means the entry has no active object, so the object-level flag is
    // intentionally unavailable rather than false.
    sub_fighter_flag: i8,
}

impl SummonEntrySnapshot {
    const fn empty() -> Self {
        Self {
            found: false,
            entry: usize::MAX,
            object_id: 0,
            active: false,
            fighter_kind: -1,
            status: -1,
            operation_cpu: false,
            team: 0,
            stock: 0,
            fighter_num: 0,
            fighter_category: 0,
            summon_boss_id: 0,
            sub_fighter_flag: -1,
        }
    }
}

#[derive(Copy, Clone)]
struct SummonEntryLifecycle {
    initialized: bool,
    phase: SummonEntryLifecyclePhase,
    parent_boss_id: u32,
    snapshot: SummonEntrySnapshot,
    parent_dead: bool,
    last_log_signature: u64,
}

impl SummonEntryLifecycle {
    const fn empty() -> Self {
        Self {
            initialized: false,
            phase: SummonEntryLifecyclePhase::Uninitialized,
            parent_boss_id: 0,
            snapshot: SummonEntrySnapshot::empty(),
            parent_dead: false,
            last_log_signature: u64::MAX,
        }
    }
}

/// Observed native summon lifecycle. The public bindings do not expose the
/// native fighter-creation or controller-transfer steps, so the states below
/// deliberately stop at observable boundaries instead of claiming ownership.
#[derive(Copy, Clone, PartialEq, Eq)]
enum SummonControlState {
    Inactive,
    SummonRequested,
    NativeStatusActive,
    NativeWait,
    NativeEnded,
    CancelledByResult,
}

#[inline(always)]
fn control_state_name(state: SummonControlState) -> &'static str {
    match state {
        SummonControlState::Inactive => "inactive",
        SummonControlState::SummonRequested => "summon_requested",
        SummonControlState::NativeStatusActive => "native_status_active",
        SummonControlState::NativeWait => "native_wait",
        SummonControlState::NativeEnded => "native_ended",
        SummonControlState::CancelledByResult => "cancelled_by_result",
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct RosterSnapshot {
    total: i32,
    signature: u64,
    active_count: usize,
    active_object_ids: [u32; MAX_FIGHTERS],
    active_kinds: [i32; MAX_FIGHTERS],
    active_teams: [u64; MAX_FIGHTERS],
    active_team_owner_ids: [u64; MAX_FIGHTERS],
    active_operation_cpu: [bool; MAX_FIGHTERS],
    active_summon_boss_ids: [u64; MAX_FIGHTERS],
}

impl RosterSnapshot {
    const fn empty() -> Self {
        Self {
            total: -1,
            signature: 0,
            active_count: 0,
            active_object_ids: [0; MAX_FIGHTERS],
            active_kinds: [0; MAX_FIGHTERS],
            active_teams: [0; MAX_FIGHTERS],
            active_team_owner_ids: [0; MAX_FIGHTERS],
            active_operation_cpu: [false; MAX_FIGHTERS],
            active_summon_boss_ids: [0; MAX_FIGHTERS],
        }
    }

    fn shape_matches(&self, other: &Self) -> bool {
        self.total == other.total
            && self.active_count == other.active_count
            && self.active_object_ids == other.active_object_ids
            && self.active_kinds == other.active_kinds
            && self.active_teams == other.active_teams
            && self.active_team_owner_ids == other.active_team_owner_ids
    }
}

/// A possible fighter created by the native summon status. This is only a
/// correlation result: the public bindings do not establish that the object
/// belongs to the boss or that its controller was transferred.
#[derive(Copy, Clone, PartialEq, Eq)]
struct SummonCandidate {
    entry: usize,
    object_id: u32,
    kind: i32,
    team: u64,
    team_owner_id: u64,
    operation_cpu: bool,
    summon_boss_id: u64,
    source: u8,
}

impl SummonCandidate {
    const fn empty() -> Self {
        Self {
            entry: usize::MAX,
            object_id: 0,
            kind: 0,
            team: 0,
            team_owner_id: 0,
            operation_cpu: false,
            summon_boss_id: 0,
            source: 0,
        }
    }

    fn signature(self) -> u64 {
        (self.entry as u64).rotate_left(3)
            ^ (self.object_id as u64).rotate_left(11)
            ^ (self.kind as u32 as u64).rotate_left(19)
            ^ self.team.rotate_left(27)
            ^ self.team_owner_id.rotate_left(33)
            ^ ((self.operation_cpu as u64) << 37)
            ^ self.summon_boss_id.rotate_left(41)
            ^ ((self.source as u64) << 57)
    }
}

#[inline(always)]
fn candidate_source_name(source: u8) -> &'static str {
    match source {
        1 => "roster_delta",
        2 => "summon_marker",
        3 => "roster_delta_and_summon_marker",
        _ => "none",
    }
}

/// Correlate a changed fighter roster slot with the summon request. A marker
/// or roster change is evidence for diagnostics only; it is never treated as
/// proof of native ownership or controller transfer.
fn native_summon_candidate(baseline: &RosterSnapshot, current: &RosterSnapshot) -> SummonCandidate {
    let mut selected = SummonCandidate::empty();
    let mut selected_score = 0u8;

    for entry in 0..MAX_FIGHTERS {
        let object_id = current.active_object_ids[entry];
        if object_id == 0 {
            continue;
        }

        let roster_delta = baseline.active_object_ids[entry] != object_id;
        let marker_delta = current.active_summon_boss_ids[entry] != 0
            && baseline.active_summon_boss_ids[entry] != current.active_summon_boss_ids[entry];
        if !roster_delta && !marker_delta {
            continue;
        }

        let source = match (roster_delta, marker_delta) {
            (true, true) => 3,
            (true, false) => 1,
            (false, true) => 2,
            (false, false) => 0,
        };
        // Prefer a candidate carrying the native summon marker, then retain
        // the first changed slot so noisy roster transitions remain bounded.
        let score = if marker_delta { 2 } else { 1 };
        if score <= selected_score {
            continue;
        }

        selected_score = score;
        selected = SummonCandidate {
            entry,
            object_id,
            kind: current.active_kinds[entry],
            team: current.active_teams[entry],
            team_owner_id: current.active_team_owner_ids[entry],
            operation_cpu: current.active_operation_cpu[entry],
            summon_boss_id: current.active_summon_boss_ids[entry],
            source,
        };
    }

    selected
}

#[derive(Copy, Clone)]
struct SummonObservation {
    initialized: bool,
    request_pending: bool,
    baseline_initialized: bool,
    roster_delta_logged: bool,
    last_phase: u8,
    last_boss_id: u32,
    last_requested_status: i32,
    last_status: i32,
    last_motion: u64,
    last_fighter_total: i32,
    last_roster_signature: u64,
    last_candidate_signature: u64,
    request_source: &'static str,
    baseline_roster: RosterSnapshot,
    last_roster: RosterSnapshot,
    candidate: SummonCandidate,
    ticks: u32,
    control_state: SummonControlState,
}

impl SummonObservation {
    const fn empty() -> Self {
        Self {
            initialized: false,
            request_pending: false,
            baseline_initialized: false,
            roster_delta_logged: false,
            last_phase: 0,
            last_boss_id: 0,
            last_requested_status: -1,
            last_status: -1,
            last_motion: 0,
            last_fighter_total: -1,
            last_roster_signature: u64::MAX,
            last_candidate_signature: u64::MAX,
            request_source: "none",
            baseline_roster: RosterSnapshot::empty(),
            last_roster: RosterSnapshot::empty(),
            candidate: SummonCandidate::empty(),
            ticks: 0,
            control_state: SummonControlState::Inactive,
        }
    }
}

static mut OBSERVATIONS: [SummonObservation; MAX_FIGHTERS * BOSS_KIND_COUNT] =
    [SummonObservation::empty(); MAX_FIGHTERS * BOSS_KIND_COUNT];

// This ledger is intentionally separate from `OBSERVATIONS`.  The latter is
// cleared when a summon request finishes or native teardown begins; this
// ledger preserves the native FighterManager child relationship long enough
// to diagnose and safely defer parent-item teardown.
static mut SUMMON_ENTRY_LIFECYCLES: [SummonEntryLifecycle; MAX_FIGHTERS * BOSS_KIND_COUNT] =
    [SummonEntryLifecycle::empty(); MAX_FIGHTERS * BOSS_KIND_COUNT];

#[derive(Copy, Clone, PartialEq, Eq)]
enum ParentDeathCleanupPhase {
    Inactive,
    DeathObserved,
    NativeGrace,
    RecheckParent,
    FallbackPending,
    Complete,
    Aborted,
}

impl ParentDeathCleanupPhase {
    const fn name(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::DeathObserved => "death_observed",
            Self::NativeGrace => "native_grace",
            Self::RecheckParent => "recheck_parent",
            Self::FallbackPending => "fallback_pending",
            Self::Complete => "complete",
            Self::Aborted => "aborted",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ParentDeathCleanupAction {
    Defer,
    RunFallback,
    Complete,
    Abort,
}

impl ParentDeathCleanupAction {
    const fn name(self) -> &'static str {
        match self {
            Self::Defer => "defer",
            Self::RunFallback => "run_fallback",
            Self::Complete => "complete",
            Self::Abort => "abort",
        }
    }
}

#[derive(Copy, Clone)]
struct ParentDeathCleanup {
    generation: u32,
    phase: ParentDeathCleanupPhase,
    parent_object_id: u32,
    callbacks_remaining: u8,
    category5_snapshot: SummonEntrySnapshot,
    last_log_signature: u64,
}

impl ParentDeathCleanup {
    const fn empty() -> Self {
        Self {
            generation: 0,
            phase: ParentDeathCleanupPhase::Inactive,
            parent_object_id: 0,
            callbacks_remaining: 0,
            category5_snapshot: SummonEntrySnapshot::empty(),
            last_log_signature: u64::MAX,
        }
    }
}

static mut PARENT_DEATH_CLEANUPS: [ParentDeathCleanup; MAX_FIGHTERS * BOSS_KIND_COUNT] =
    [ParentDeathCleanup::empty(); MAX_FIGHTERS * BOSS_KIND_COUNT];
static mut SCENE_EXIT_LAST_SIGNATURE: [u64; MAX_FIGHTERS * BOSS_KIND_COUNT] =
    [u64::MAX; MAX_FIGHTERS * BOSS_KIND_COUNT];

// These are diagnostic latches, not ownership state.  Keeping them separate
// from the summon observation lets the match-end audit survive a native item
// teardown without retaining or dereferencing the item object.
static mut MATCH_AUDIT_LAST_SIGNATURE: [u64; MAX_FIGHTERS * BOSS_KIND_COUNT] =
    [u64::MAX; MAX_FIGHTERS * BOSS_KIND_COUNT];
static mut MATCH_AUDIT_LAST_DEAD: [i8; MAX_FIGHTERS * BOSS_KIND_COUNT] =
    [-1; MAX_FIGHTERS * BOSS_KIND_COUNT];

const RESULT_ROSTER_PHASE_COUNT: usize = 8;
static mut RESULT_ROSTER_LAST_SIGNATURE: [u64; RESULT_ROSTER_PHASE_COUNT] =
    [u64::MAX; RESULT_ROSTER_PHASE_COUNT];
static mut RESULT_HELPER_LAST_SIGNATURE: [u64; BOSS_KIND_COUNT * 4] =
    [u64::MAX; BOSS_KIND_COUNT * 4];

#[derive(Copy, Clone)]
struct ResultRosterRecord {
    present: bool,
    entry: usize,
    current_object_id: u64,
    active: bool,
    fighter_kind: i32,
    status: i32,
    operation_cpu: bool,
    fighter_category: u64,
    summon_boss_id: u64,
    team: u64,
    team_owner_id: u64,
    stock: u64,
    dead_count: u64,
    rebirth: bool,
    dead_status: bool,
    standby_status: bool,
    hidden_host: bool,
    fighter_num: u64,
    // This is an object-level observation only. It is unavailable for a
    // reserved FighterManager entry whose current object is inactive.
    sub_fighter_flag: i8,
}

impl ResultRosterRecord {
    const fn empty(entry: usize) -> Self {
        Self {
            present: false,
            entry,
            current_object_id: 0,
            active: false,
            fighter_kind: -1,
            status: -1,
            operation_cpu: false,
            fighter_category: 0,
            summon_boss_id: 0,
            team: 0,
            team_owner_id: 0,
            stock: 0,
            dead_count: 0,
            rebirth: false,
            dead_status: false,
            standby_status: false,
            hidden_host: false,
            fighter_num: 0,
            sub_fighter_flag: -1,
        }
    }

    fn signature(self) -> u64 {
        (self.present as u64)
            ^ (self.current_object_id.rotate_left(7))
            ^ (self.active as u64).rotate_left(13)
            ^ (self.fighter_kind as u32 as u64).rotate_left(17)
            ^ (self.status as u32 as u64).rotate_left(23)
            ^ (self.operation_cpu as u64).rotate_left(29)
            ^ self.fighter_category.rotate_left(31)
            ^ self.summon_boss_id.rotate_left(37)
            ^ self.team.rotate_left(41)
            ^ self.team_owner_id.rotate_left(47)
            ^ self.stock.rotate_left(3)
            ^ self.dead_count.rotate_left(11)
            ^ (self.rebirth as u64).rotate_left(19)
            ^ (self.dead_status as u64).rotate_left(53)
            ^ (self.standby_status as u64).rotate_left(59)
            ^ (self.hidden_host as u64).rotate_left(5)
            ^ self.fighter_num.rotate_left(43)
            ^ (self.sub_fighter_flag as u8 as u64).rotate_left(59)
    }
}

#[derive(Copy, Clone)]
struct NativeSummonWorkSnapshot {
    active: bool,
    ai_in_effect: bool,
    ai_soon_to_be_attack: bool,
    target_found: bool,
    targetable: bool,
    value_flags: [bool; 6],
    boss_mode: i32,
    message: i32,
    parameter_item_kind: i32,
    variation: i32,
    attack_kind: i32,
    trait_flag: i32,
    ai_value_1: i32,
    value_ints: [i32; 4],
    ai_value_float: f32,
}

impl NativeSummonWorkSnapshot {
    const fn empty() -> Self {
        Self {
            active: false,
            ai_in_effect: false,
            ai_soon_to_be_attack: false,
            target_found: false,
            targetable: false,
            value_flags: [false; 6],
            boss_mode: 0,
            message: 0,
            parameter_item_kind: 0,
            variation: 0,
            attack_kind: 0,
            trait_flag: 0,
            ai_value_1: 0,
            value_ints: [0; 4],
            ai_value_float: 0.0,
        }
    }
}

#[inline(always)]
fn kind_offset(kind: &'static str) -> usize {
    if kind == "dharkon" {
        MAX_FIGHTERS
    } else {
        0
    }
}

#[inline(always)]
unsafe fn observation_index(kind: &'static str, entry: usize) -> usize {
    kind_offset(kind) + entry.min(MAX_FIGHTERS - 1)
}

#[inline(always)]
fn phase_name(phase: u8) -> &'static str {
    match phase {
        1 => "requested",
        2 => "native_wait",
        3 => "native_complete",
        4 => "boss_inactive",
        _ => "idle",
    }
}

/// Include the public FighterEntry slot API in the roster fingerprint without
/// assigning meaning to its boolean selector. The pinned bindings expose the
/// function, but do not document whether that selector means current/previous,
/// active/inactive, or another native slot view. The raw values are therefore
/// logged as `selector=false/true` and are not used for ownership decisions.
unsafe fn fighter_entry_slot_signature(entry_ptr: *mut FighterEntry) -> u64 {
    if entry_ptr.is_null() {
        return 0;
    }

    let fighter_num =
        FighterEntryBindings::fighter_num(entry_ptr).min(MAX_FIGHTER_ENTRY_SLOTS as u64);
    let mut signature = fighter_num;
    for slot in 0..fighter_num as i32 {
        let selector_false = FighterEntryBindings::get_fighter_id(entry_ptr, slot, false);
        let selector_true = FighterEntryBindings::get_fighter_id(entry_ptr, slot, true);
        signature ^= (selector_false.wrapping_add(slot as u64)).rotate_left(11);
        signature ^= (selector_true.wrapping_add(slot as u64)).rotate_left(29);
    }
    signature
}

#[inline(always)]
fn result_roster_phase_index(phase: &str) -> usize {
    match phase {
        "pre_match" => 0,
        "battle" => 1,
        "post_match_pre_result" => 2,
        "post_match_after_tracking_invalidation" => 3,
        "result_ready" => 4,
        "scene_exit" => 5,
        _ => 6,
    }
}

#[inline(always)]
fn result_roster_text_token(text: &str) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.bytes() {
        value ^= byte as u64;
        value = value.wrapping_mul(0x1000_0000_01b3);
    }
    value
}

#[inline(always)]
unsafe fn read_result_roster_record(
    fighter_manager: *mut smash::app::FighterManager,
    entry: usize,
) -> ResultRosterRecord {
    let mut record = ResultRosterRecord::empty(entry);
    if fighter_manager.is_null() {
        return record;
    }

    let entry_id = FighterEntryID(entry as i32);
    let entry_ptr =
        FighterManager::get_fighter_entry(fighter_manager, entry_id) as *mut FighterEntry;
    if entry_ptr.is_null() {
        return record;
    }

    record.present = true;
    record.current_object_id = FighterEntryBindings::current_fighter_id(entry_ptr);
    record.fighter_num = FighterEntryBindings::fighter_num(entry_ptr);

    let info = FighterManager::get_fighter_information(fighter_manager, entry_id);
    if !info.is_null() {
        record.operation_cpu = FighterInformation::is_operation_cpu(info);
        record.fighter_category = FighterInformation::fighter_category(info);
        record.summon_boss_id = FighterInformation::summon_boss_id(info);
        record.stock = FighterInformation::stock_count(info);
        record.dead_count = FighterInformation::dead_count(info, 0);
        record.rebirth = FighterInformation::is_on_rebirth(info);
    }

    if record.current_object_id == 0 || record.current_object_id > u32::MAX as u64 {
        return record;
    }

    let object_id = record.current_object_id as u32;
    record.active = sv_battle_object::is_active(object_id);
    if !record.active {
        return record;
    }

    let boma = sv_battle_object::module_accessor(object_id);
    if boma.is_null() {
        record.active = false;
        return record;
    }

    record.fighter_kind = smash::app::utility::get_kind(&mut *boma);
    record.status = StatusModule::status_kind(boma);
    record.dead_status = record.status == *FIGHTER_STATUS_KIND_DEAD;
    record.standby_status = record.status == *FIGHTER_STATUS_KIND_STANDBY;
    record.team = TeamModule::team_no(boma);
    record.team_owner_id = TeamModule::team_owner_id(boma);
    record.hidden_host = crate::boss_helpers::is_hidden_host(boma);
    record.sub_fighter_flag =
        if WorkModule::is_flag(boma, *FIGHTER_INSTANCE_WORK_ID_FLAG_SUB_FIGHTER) {
            1
        } else {
            0
        };
    record
}

/// Locate the reserved/active FighterManager child associated with a native
/// Galeem/Dharkon item.  The category and parent marker are read-only public
/// FighterInformation fields.  Inactive entries are retained, but their
/// object ID is never dereferenced.
unsafe fn read_parented_summon_entry(
    fighter_manager: *mut smash::app::FighterManager,
    parent_boss_id: u32,
) -> SummonEntrySnapshot {
    if fighter_manager.is_null() || parent_boss_id == 0 {
        return SummonEntrySnapshot::empty();
    }

    for entry in 0..MAX_FIGHTERS {
        let entry_id = FighterEntryID(entry as i32);
        let info = FighterManager::get_fighter_information(fighter_manager, entry_id);
        if info.is_null()
            || FighterInformation::fighter_category(info) != TEMPORARY_BOSS_SUMMON_CATEGORY
            || FighterInformation::summon_boss_id(info) != parent_boss_id as u64
        {
            continue;
        }

        let entry_ptr =
            FighterManager::get_fighter_entry(fighter_manager, entry_id) as *mut FighterEntry;
        if entry_ptr.is_null() {
            continue;
        }

        let object_id = FighterEntryBindings::current_fighter_id(entry_ptr);
        let fighter_num = FighterEntryBindings::fighter_num(entry_ptr);
        let active = object_id <= u32::MAX as u64
            && object_id != 0
            && sv_battle_object::is_active(object_id as u32)
            && !sv_battle_object::module_accessor(object_id as u32).is_null();
        let (fighter_kind, status, team, sub_fighter_flag) = if active {
            let boma = sv_battle_object::module_accessor(object_id as u32);
            (
                smash::app::utility::get_kind(&mut *boma),
                StatusModule::status_kind(boma),
                TeamModule::team_no(boma),
                if WorkModule::is_flag(boma, *FIGHTER_INSTANCE_WORK_ID_FLAG_SUB_FIGHTER) {
                    1
                } else {
                    0
                },
            )
        } else {
            (-1, -1, 0, -1)
        };

        return SummonEntrySnapshot {
            found: true,
            entry,
            object_id,
            active,
            fighter_kind,
            status,
            operation_cpu: FighterInformation::is_operation_cpu(info),
            team,
            stock: FighterInformation::stock_count(info),
            fighter_num,
            fighter_category: FighterInformation::fighter_category(info),
            summon_boss_id: FighterInformation::summon_boss_id(info),
            sub_fighter_flag,
        };
    }

    SummonEntrySnapshot::empty()
}

#[inline(always)]
fn lifecycle_signature(
    phase: SummonEntryLifecyclePhase,
    snapshot: SummonEntrySnapshot,
    parent_dead: bool,
) -> u64 {
    (phase.name().as_bytes().first().copied().unwrap_or(0) as u64)
        ^ (snapshot.found as u64).rotate_left(7)
        ^ (snapshot.active as u64).rotate_left(13)
        ^ snapshot.entry.rotate_left(17) as u64
        ^ snapshot.object_id.rotate_left(23)
        ^ (snapshot.fighter_kind as u32 as u64).rotate_left(31)
        ^ (snapshot.status as u32 as u64).rotate_left(37)
        ^ snapshot.fighter_category.rotate_left(43)
        ^ snapshot.summon_boss_id.rotate_left(47)
        ^ snapshot.stock.rotate_left(53)
        ^ snapshot.fighter_num.rotate_left(59)
        ^ (snapshot.sub_fighter_flag as u8 as u64).rotate_left(7)
        ^ ((parent_dead as u64) << 61)
}

/// Update the bounded category-5 child ledger.  No FighterManager state is
/// changed here; this function only observes the public entry/information
/// fields and, when debug logs are enabled, records transitions.
unsafe fn observe_summon_entry_lifecycle(
    kind: &'static str,
    parent_entry: usize,
    parent_boss_id: u32,
    parent_dead: bool,
    transition_phase: &str,
) -> SummonEntrySnapshot {
    if parent_boss_id == 0 {
        return SummonEntrySnapshot::empty();
    }

    let index = observation_index(kind, parent_entry);
    let snapshot =
        read_parented_summon_entry(crate::boss_helpers::fighter_manager(), parent_boss_id);
    let previous = SUMMON_ENTRY_LIFECYCLES[index];
    let phase = if transition_phase == "post_match_pre_result"
        || transition_phase == "result_ready"
        || transition_phase == "scene_exit"
    {
        SummonEntryLifecyclePhase::CancelledByResult
    } else if parent_dead {
        if snapshot.found {
            SummonEntryLifecyclePhase::Retiring
        } else if previous.initialized {
            SummonEntryLifecyclePhase::NoChild
        } else {
            SummonEntryLifecyclePhase::Uninitialized
        }
    } else if snapshot.found {
        if snapshot.active {
            if !previous.initialized || !previous.snapshot.active {
                SummonEntryLifecyclePhase::Spawned
            } else {
                SummonEntryLifecyclePhase::Active
            }
        } else if previous.initialized && previous.snapshot.active {
            SummonEntryLifecyclePhase::ExpiredEntryReserved
        } else {
            SummonEntryLifecyclePhase::Reserved
        }
    } else if previous.initialized {
        SummonEntryLifecyclePhase::NoChild
    } else {
        SummonEntryLifecyclePhase::Uninitialized
    };
    let log_signature = lifecycle_signature(phase, snapshot, parent_dead);
    let changed = !previous.initialized
        || previous.phase != phase
        || previous.snapshot.found != snapshot.found
        || previous.snapshot.active != snapshot.active
        || previous.snapshot.object_id != snapshot.object_id
        || previous.snapshot.entry != snapshot.entry
        || previous.parent_dead != parent_dead;

    if crate::debug::enabled() && changed && previous.last_log_signature != log_signature {
        let retirement_state = match phase {
            SummonEntryLifecyclePhase::Retiring => "native_parent_teardown_pending",
            SummonEntryLifecyclePhase::NoChild => "no_native_child_observed",
            SummonEntryLifecyclePhase::Uninitialized => "no_child_observed_yet",
            SummonEntryLifecyclePhase::ExpiredEntryReserved => {
                "child_object_gone_entry_still_reserved"
            }
            SummonEntryLifecyclePhase::CancelledByResult => {
                "tracking_cancelled_native_state_unknown"
            }
            _ => "native_child_associated",
        };
        crate::boss_log!(
            "[PB][SummonEntryLifecycle] phase={} transition_phase={} parent_boss={} parent_entry={} summon_entry={} object_id=0x{:x} fighter_kind={} category=0x{:x} stock={} status={} active={} team={} fighter_num={} sub_fighter_flag={} summon_boss_id=0x{:x} parent_dead={} retirement_state={} result_eligibility=unavailable_public_api",
            phase.name(),
            transition_phase,
            kind,
            parent_entry.min(MAX_FIGHTERS - 1),
            snapshot.entry,
            snapshot.object_id,
            snapshot.fighter_kind,
            snapshot.fighter_category,
            snapshot.stock,
            snapshot.status,
            snapshot.active,
            snapshot.team,
            snapshot.fighter_num,
            snapshot.sub_fighter_flag,
            snapshot.summon_boss_id,
            parent_dead,
            retirement_state
        );
    }

    SUMMON_ENTRY_LIFECYCLES[index] = SummonEntryLifecycle {
        initialized: true,
        phase,
        parent_boss_id,
        snapshot,
        parent_dead,
        last_log_signature: log_signature,
    };
    snapshot
}

#[inline(always)]
unsafe fn parent_state(parent_object_id: u32) -> (bool, i32) {
    if parent_object_id == 0 || !sv_battle_object::is_active(parent_object_id) {
        return (false, -1);
    }
    let parent_boma = sv_battle_object::module_accessor(parent_object_id);
    if parent_boma.is_null() {
        return (false, -1);
    }
    (true, StatusModule::status_kind(parent_boma))
}

#[inline(always)]
unsafe fn log_parent_death_cleanup_transition(
    kind: &'static str,
    parent_entry: usize,
    state: &mut ParentDeathCleanup,
    parent_active: bool,
    parent_status: i32,
    action: ParentDeathCleanupAction,
    exit_reason: &'static str,
) {
    if !crate::debug::enabled() {
        return;
    }

    let snapshot = state.category5_snapshot;
    let signature = (state.generation as u64).rotate_left(3)
        ^ (state.phase.name().as_bytes().first().copied().unwrap_or(0) as u64).rotate_left(9)
        ^ (state.callbacks_remaining as u64).rotate_left(15)
        ^ (state.parent_object_id as u64).rotate_left(21)
        ^ ((parent_active as u64) << 29)
        ^ (parent_status as u32 as u64).rotate_left(31)
        ^ (snapshot.entry as u64).rotate_left(37)
        ^ (snapshot.object_id).rotate_left(43)
        ^ (snapshot.stock).rotate_left(49)
        ^ ((snapshot.active as u64) << 55)
        ^ (action.name().as_bytes().first().copied().unwrap_or(0) as u64).rotate_left(57);
    if state.last_log_signature == signature {
        return;
    }

    state.last_log_signature = signature;

    crate::boss_log!(
        "[PB][ParentDeathCleanup] boss={} entry={} parent_object_id=0x{:x} cleanup_generation={} phase={} callbacks_remaining={} parent_active={} parent_status={} category5_entry={} category5_stock={} category5_active={} category5_object_id=0x{:x} action={} exit_reason={}",
        kind,
        parent_entry.min(MAX_FIGHTERS - 1),
        state.parent_object_id,
        state.generation,
        state.phase.name(),
        state.callbacks_remaining,
        parent_active,
        parent_status,
        snapshot.entry,
        snapshot.stock,
        snapshot.active,
        snapshot.object_id,
        action.name(),
        exit_reason
    );
}

/// Advance the one-shot parent death cleanup state machine. This function
/// never retires or mutates a FighterManager child entry. It only gives the
/// native parent DEAD status three distinct callbacks, then permits the
/// already-established host-item fallback exactly once if the parent remains
/// active in the expected native death status.
pub unsafe fn parent_death_cleanup_step(
    kind: &'static str,
    parent_entry: usize,
    parent_boss_id: u32,
) -> ParentDeathCleanupAction {
    let index = observation_index(kind, parent_entry);
    let snapshot =
        observe_summon_entry_lifecycle(kind, parent_entry, parent_boss_id, true, "battle");
    let state = &mut PARENT_DEATH_CLEANUPS[index];

    if parent_boss_id == 0 {
        state.phase = ParentDeathCleanupPhase::Complete;
        state.callbacks_remaining = 0;
        state.category5_snapshot = snapshot;
        let (parent_active, parent_status) = parent_state(parent_boss_id);
        log_parent_death_cleanup_transition(
            kind,
            parent_entry,
            state,
            parent_active,
            parent_status,
            ParentDeathCleanupAction::Complete,
            "no_parent_object",
        );
        return ParentDeathCleanupAction::Complete;
    }

    // Complete and Aborted are terminal for this parent object. A new
    // generation is opened by reset(), never by repeatedly revisiting the
    // same dead callback.
    if state.phase == ParentDeathCleanupPhase::Inactive {
        state.generation = state.generation.wrapping_add(1).max(1);
        state.phase = if snapshot.found {
            ParentDeathCleanupPhase::DeathObserved
        } else {
            ParentDeathCleanupPhase::FallbackPending
        };
        state.parent_object_id = parent_boss_id;
        state.callbacks_remaining = 0;
        state.category5_snapshot = snapshot;
        state.last_log_signature = u64::MAX;
        let (parent_active, parent_status) = parent_state(parent_boss_id);
        let action = if snapshot.found {
            ParentDeathCleanupAction::Defer
        } else {
            ParentDeathCleanupAction::RunFallback
        };
        log_parent_death_cleanup_transition(
            kind,
            parent_entry,
            state,
            parent_active,
            parent_status,
            action,
            if snapshot.found {
                "death_observed"
            } else {
                "no_category5_entry"
            },
        );
        return action;
    }

    if state.parent_object_id != parent_boss_id {
        let (parent_active, parent_status) = parent_state(parent_boss_id);
        state.phase = ParentDeathCleanupPhase::Aborted;
        state.callbacks_remaining = 0;
        log_parent_death_cleanup_transition(
            kind,
            parent_entry,
            state,
            parent_active,
            parent_status,
            ParentDeathCleanupAction::Abort,
            "parent_object_id_changed",
        );
        return ParentDeathCleanupAction::Abort;
    }

    state.category5_snapshot = snapshot;
    let (parent_active, parent_status) = parent_state(state.parent_object_id);

    let action = match state.phase {
        ParentDeathCleanupPhase::DeathObserved => {
            if !parent_active {
                state.phase = ParentDeathCleanupPhase::Complete;
                ParentDeathCleanupAction::Complete
            } else if parent_status != *ITEM_STATUS_KIND_DEAD {
                state.phase = ParentDeathCleanupPhase::Aborted;
                ParentDeathCleanupAction::Abort
            } else {
                state.phase = ParentDeathCleanupPhase::NativeGrace;
                state.callbacks_remaining = 3;
                ParentDeathCleanupAction::Defer
            }
        }
        ParentDeathCleanupPhase::NativeGrace => {
            if !parent_active {
                state.phase = ParentDeathCleanupPhase::Complete;
                state.callbacks_remaining = 0;
                ParentDeathCleanupAction::Complete
            } else if parent_status != *ITEM_STATUS_KIND_DEAD {
                state.phase = ParentDeathCleanupPhase::Aborted;
                state.callbacks_remaining = 0;
                ParentDeathCleanupAction::Abort
            } else if state.callbacks_remaining > 0 {
                state.callbacks_remaining -= 1;
                if state.callbacks_remaining == 0 {
                    state.phase = ParentDeathCleanupPhase::RecheckParent;
                }
                ParentDeathCleanupAction::Defer
            } else {
                state.phase = ParentDeathCleanupPhase::RecheckParent;
                ParentDeathCleanupAction::Defer
            }
        }
        ParentDeathCleanupPhase::RecheckParent => {
            if !parent_active {
                state.phase = ParentDeathCleanupPhase::Complete;
                ParentDeathCleanupAction::Complete
            } else if parent_status == *ITEM_STATUS_KIND_DEAD {
                state.phase = ParentDeathCleanupPhase::FallbackPending;
                ParentDeathCleanupAction::RunFallback
            } else {
                state.phase = ParentDeathCleanupPhase::Aborted;
                ParentDeathCleanupAction::Abort
            }
        }
        ParentDeathCleanupPhase::FallbackPending => {
            if !parent_active {
                state.phase = ParentDeathCleanupPhase::Complete;
                ParentDeathCleanupAction::Complete
            } else if parent_status == *ITEM_STATUS_KIND_DEAD {
                ParentDeathCleanupAction::Defer
            } else {
                state.phase = ParentDeathCleanupPhase::Aborted;
                ParentDeathCleanupAction::Abort
            }
        }
        ParentDeathCleanupPhase::Complete => ParentDeathCleanupAction::Complete,
        ParentDeathCleanupPhase::Aborted => ParentDeathCleanupAction::Abort,
        ParentDeathCleanupPhase::Inactive => ParentDeathCleanupAction::Abort,
    };

    log_parent_death_cleanup_transition(
        kind,
        parent_entry,
        state,
        parent_active,
        parent_status,
        action,
        match action {
            ParentDeathCleanupAction::Defer => "native_grace_or_recheck",
            ParentDeathCleanupAction::RunFallback => "native_grace_complete",
            ParentDeathCleanupAction::Complete => "parent_inactive",
            ParentDeathCleanupAction::Abort => "unexpected_parent_state",
        },
    );
    action
}

/// Emit one scene-exit breadcrumb before a boss module clears its local item
/// tracking. This is observational only; inactive parent/child IDs are never
/// dereferenced.
pub unsafe fn log_boss_scene_exit(
    kind: &'static str,
    entry: usize,
    parent_boss_id: u32,
    cleanup_action: &'static str,
) {
    if !crate::debug::enabled() {
        return;
    }

    let index = observation_index(kind, entry);
    let snapshot = if parent_boss_id == 0 {
        SummonEntrySnapshot::empty()
    } else {
        read_parented_summon_entry(crate::boss_helpers::fighter_manager(), parent_boss_id)
    };
    let (parent_active, parent_status) = parent_state(parent_boss_id);
    let fighter_manager = crate::boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let ready_go = smash::app::sv_information::is_ready_go();
    let stage_id = smash::app::stage::get_stage_id();
    let state = PARENT_DEATH_CLEANUPS[index];
    let signature = (parent_boss_id as u64).rotate_left(7)
        ^ (state.generation as u64).rotate_left(13)
        ^ (state.phase.name().as_bytes().first().copied().unwrap_or(0) as u64).rotate_left(19)
        ^ (snapshot.entry as u64).rotate_left(25)
        ^ snapshot.object_id.rotate_left(31)
        ^ (snapshot.stock).rotate_left(37)
        ^ ((snapshot.active as u64) << 43)
        ^ ((parent_active as u64) << 44)
        ^ ((ready_go as u64) << 45)
        ^ ((result_mode as u64) << 46)
        ^ (stage_id as u32 as u64).rotate_left(49);
    if SCENE_EXIT_LAST_SIGNATURE[index] == signature {
        return;
    }
    SCENE_EXIT_LAST_SIGNATURE[index] = signature;

    crate::boss_log!(
        "[PB][BossSceneExit] boss={} entry={} parent_object_id=0x{:x} parent_active={} parent_status={} parent_cleanup_phase={} category5_entry={} category5_active={} category5_stock={} category5_object_id=0x{:x} ready_go={} result_mode={} scene=stage:0x{:x} cleanup_action={}",
        kind,
        entry.min(MAX_FIGHTERS - 1),
        parent_boss_id,
        parent_active,
        parent_status,
        state.phase.name(),
        snapshot.entry,
        snapshot.active,
        snapshot.stock,
        snapshot.object_id,
        ready_go,
        result_mode,
        stage_id,
        cleanup_action
    );
}

#[inline(always)]
unsafe fn read_native_summon_work_snapshot(
    boss_boma: *mut BattleObjectModuleAccessor,
) -> NativeSummonWorkSnapshot {
    if boss_boma.is_null() {
        return NativeSummonWorkSnapshot::empty();
    }

    let mut snapshot = NativeSummonWorkSnapshot {
        active: true,
        ai_in_effect: WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_AI_IS_IN_EFFECT),
        ai_soon_to_be_attack: WorkModule::is_flag(
            boss_boma,
            *ITEM_INSTANCE_WORK_FLAG_AI_SOON_TO_BE_ATTACK,
        ),
        target_found: WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_TARGET_FOUND),
        targetable: WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_TARGETABLE),
        value_flags: [
            WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_VALUE_1),
            WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_VALUE_2),
            WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_VALUE_3),
            WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_VALUE_4),
            WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_VALUE_5),
            WorkModule::is_flag(boss_boma, *ITEM_INSTANCE_WORK_FLAG_VALUE_6),
        ],
        boss_mode: WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_BOSS_MODE),
        message: WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_MESSAGE),
        parameter_item_kind: WorkModule::get_int(
            boss_boma,
            *ITEM_INSTANCE_WORK_INT_PARAMETER_ITEM_KIND,
        ),
        variation: WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_VARIATION),
        attack_kind: WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_ATTACK_KIND),
        trait_flag: WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_TRAIT_FLAG),
        ai_value_1: WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_AI_VALUE_1),
        value_ints: [
            WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_VALUE_1),
            WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_VALUE_2),
            WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_VALUE_3),
            WorkModule::get_int(boss_boma, *ITEM_INSTANCE_WORK_INT_VALUE_4),
        ],
        ai_value_float: WorkModule::get_float(boss_boma, *ITEM_INSTANCE_WORK_FLOAT_AI_VALUE_1),
    };
    snapshot.active = true;
    snapshot
}

#[inline(always)]
unsafe fn log_native_summon_work_snapshot(
    kind: &'static str,
    entry: usize,
    source: &'static str,
    phase: &'static str,
    boss_id: u32,
    requested_status: i32,
    status: i32,
    motion: u64,
    boss_boma: *mut BattleObjectModuleAccessor,
) {
    if !crate::debug::enabled() {
        return;
    }

    let work = read_native_summon_work_snapshot(boss_boma);
    let fighter_manager = crate::boss_helpers::fighter_manager();
    let operation_cpu = crate::boss_helpers::is_operation_cpu_entry(fighter_manager, entry);
    crate::boss_log!(
        "[PB][BossSummonNative] kind={} entry={} source={} phase={} boss_id=0x{:x} host_operation_cpu={} requested_status={} status={} motion=0x{:x} work_active={} ai_in_effect={} ai_soon_to_be_attack={} target_found={} targetable={} value_flags={:?} boss_mode={} message={} parameter_item_kind={} variation={} attack_kind={} trait_flag={} ai_value_1={} ai_value_float={:.3} value_ints={:?}",
        kind,
        entry.min(MAX_FIGHTERS - 1),
        source,
        phase,
        boss_id,
        operation_cpu,
        requested_status,
        status,
        motion,
        work.active,
        work.ai_in_effect,
        work.ai_soon_to_be_attack,
        work.target_found,
        work.targetable,
        work.value_flags,
        work.boss_mode,
        work.message,
        work.parameter_item_kind,
        work.variation,
        work.attack_kind,
        work.trait_flag,
        work.ai_value_1,
        work.ai_value_float,
        work.value_ints
    );
}

/// Build a bounded, read-only snapshot of the public fighter roster. The
/// object IDs and kinds are only used to report a possible native summon
/// result; this function never treats a roster delta as proof of ownership or
/// control transfer.
unsafe fn roster_snapshot(fighter_manager: *mut smash::app::FighterManager) -> RosterSnapshot {
    if fighter_manager.is_null() {
        return RosterSnapshot::empty();
    }

    let mut snapshot = RosterSnapshot {
        total: FighterManager::total_fighter_num(fighter_manager),
        ..RosterSnapshot::empty()
    };
    let mut signature = snapshot.total as u64;
    for entry in 0..MAX_FIGHTERS {
        let entry_id = FighterEntryID(entry as i32);
        let entry_ptr =
            FighterManager::get_fighter_entry(fighter_manager, entry_id) as *mut FighterEntry;
        if entry_ptr.is_null() {
            continue;
        }

        let fighter_id = FighterEntryBindings::current_fighter_id(entry_ptr);
        if fighter_id == 0 || fighter_id > u32::MAX as u64 {
            continue;
        }
        let object_id = fighter_id as u32;
        if !sv_battle_object::is_active(object_id) {
            continue;
        }
        let boma = sv_battle_object::module_accessor(object_id);
        if boma.is_null() {
            continue;
        }

        snapshot.active_count += 1;
        let fighter_num = FighterEntryBindings::fighter_num(entry_ptr);
        let fighter_slot_signature = fighter_entry_slot_signature(entry_ptr);
        let info = FighterManager::get_fighter_information(fighter_manager, entry_id);
        let operation_cpu = !info.is_null() && FighterInformation::is_operation_cpu(info);
        let fighter_category = if info.is_null() {
            0
        } else {
            FighterInformation::fighter_category(info)
        };
        let summon_boss_id = if info.is_null() {
            0
        } else {
            FighterInformation::summon_boss_id(info)
        };
        let kind = smash::app::utility::get_kind(&mut *boma) as u64;
        let team = TeamModule::team_no(boma);
        let team_owner_id = TeamModule::team_owner_id(boma);
        snapshot.active_object_ids[entry] = object_id;
        snapshot.active_kinds[entry] = kind as i32;
        snapshot.active_teams[entry] = team;
        snapshot.active_team_owner_ids[entry] = team_owner_id;
        snapshot.active_operation_cpu[entry] = operation_cpu;
        snapshot.active_summon_boss_ids[entry] = summon_boss_id;
        signature ^= (entry as u64).rotate_left(3)
            ^ (object_id as u64).rotate_left(11)
            ^ kind.rotate_left(19)
            ^ team.rotate_left(27)
            ^ team_owner_id.rotate_left(33)
            ^ ((operation_cpu as u64) << 37)
            ^ fighter_category.rotate_left(41)
            ^ summon_boss_id.rotate_left(53)
            ^ fighter_num.rotate_left(7)
            ^ fighter_slot_signature.rotate_left(47);
    }

    snapshot.signature = signature;
    snapshot
}

unsafe fn log_new_roster_candidates(
    kind: &'static str,
    summon_entry: usize,
    baseline: &RosterSnapshot,
    current: &RosterSnapshot,
) {
    for entry in 0..MAX_FIGHTERS {
        let previous_id = baseline.active_object_ids[entry];
        let current_id = current.active_object_ids[entry];
        if previous_id != current_id && current_id != 0 {
            let delta_kind = if previous_id == 0 { "new" } else { "replaced" };
            crate::boss_log!(
                "[PB][BossSummonRoster] kind={} summon_entry={} candidate_fighter_delta={} fighter_entry={} previous_object_id=0x{:x} object_id=0x{:x} fighter_kind={} team={} team_owner_id=0x{:x} operation_cpu={} summon_boss_id=0x{:x} creation=unresolved ownership=unresolved",
                kind,
                summon_entry.min(MAX_FIGHTERS - 1),
                delta_kind,
                entry,
                previous_id,
                current_id,
                current.active_kinds[entry],
                current.active_teams[entry],
                current.active_team_owner_ids[entry],
                current.active_operation_cpu[entry],
                current.active_summon_boss_ids[entry]
            );
        }
    }
}

unsafe fn log_native_summon_candidate(
    kind: &'static str,
    summon_entry: usize,
    request_source: &'static str,
    phase: u8,
    candidate: SummonCandidate,
) {
    if !crate::debug::enabled() {
        return;
    }

    crate::boss_log!(
        "[PB][BossSummonControl] kind={} summon_entry={} phase={} request_source={} native_candidate={} candidate_entry={} candidate_object_id=0x{:x} candidate_kind={} candidate_team={} candidate_team_owner_id=0x{:x} candidate_operation_cpu={} candidate_summon_boss_id=0x{:x} candidate_source={} creation=observable_only ownership=unresolved native_control_source=unresolved human_control_transfer=unresolved native_fighter_create_api=unavailable controller_transfer_api=unavailable",
        kind,
        summon_entry.min(MAX_FIGHTERS - 1),
        phase_name(phase),
        request_source,
        candidate.object_id != 0,
        candidate.entry,
        candidate.object_id,
        candidate.kind,
        candidate.team,
        candidate.team_owner_id,
        candidate.operation_cpu,
        candidate.summon_boss_id,
        candidate_source_name(candidate.source)
    );
}

unsafe fn log_roster(
    kind: &'static str,
    summon_entry: usize,
    fighter_manager: *mut smash::app::FighterManager,
) {
    if fighter_manager.is_null() {
        return;
    }

    crate::boss_log!(
        "[PB][BossSummonRoster] kind={} summon_entry={} total_fighters={} entry_count={}",
        kind,
        summon_entry.min(MAX_FIGHTERS - 1),
        FighterManager::total_fighter_num(fighter_manager),
        FighterManager::entry_count(fighter_manager)
    );

    for entry in 0..MAX_FIGHTERS {
        let entry_id = FighterEntryID(entry as i32);
        let entry_ptr =
            FighterManager::get_fighter_entry(fighter_manager, entry_id) as *mut FighterEntry;
        if entry_ptr.is_null() {
            continue;
        }
        let fighter_id = FighterEntryBindings::current_fighter_id(entry_ptr);
        if fighter_id == 0 || fighter_id > u32::MAX as u64 {
            continue;
        }
        let object_id = fighter_id as u32;
        if !sv_battle_object::is_active(object_id) {
            continue;
        }
        let boma = sv_battle_object::module_accessor(object_id);
        if boma.is_null() {
            continue;
        }
        let info = FighterManager::get_fighter_information(fighter_manager, entry_id);
        let fighter_num = FighterEntryBindings::fighter_num(entry_ptr);
        let slot_count = fighter_num.min(MAX_FIGHTER_ENTRY_SLOTS as u64);
        let operation_cpu = !info.is_null() && FighterInformation::is_operation_cpu(info);
        let fighter_category = if info.is_null() {
            0
        } else {
            FighterInformation::fighter_category(info)
        };
        let summon_boss_id = if info.is_null() {
            0
        } else {
            FighterInformation::summon_boss_id(info)
        };
        let team_owner_id = TeamModule::team_owner_id(boma);
        crate::boss_log!(
            "[PB][BossSummonRoster] fighter_entry={} fighter_num={} current_fighter_id=0x{:x} fighter_kind={} team={} team_owner_id=0x{:x} operation_cpu={} fighter_category={} summon_boss_id=0x{:x}",
            entry,
            fighter_num,
            object_id,
            smash::app::utility::get_kind(&mut *boma),
            TeamModule::team_no(boma),
            team_owner_id,
            operation_cpu,
            fighter_category,
            summon_boss_id
        );

        for slot in 0..slot_count as i32 {
            let selector_false = FighterEntryBindings::get_fighter_id(entry_ptr, slot, false);
            let selector_true = FighterEntryBindings::get_fighter_id(entry_ptr, slot, true);
            if selector_false == 0 && selector_true == 0 {
                continue;
            }
            let active_false = selector_false <= u32::MAX as u64
                && sv_battle_object::is_active(selector_false as u32);
            let active_true = selector_true <= u32::MAX as u64
                && sv_battle_object::is_active(selector_true as u32);
            crate::boss_log!(
                "[PB][BossSummonRoster] fighter_entry={} fighter_slot={} get_fighter_id_false=0x{:x} active_false={} get_fighter_id_true=0x{:x} active_true={} selector_semantics=unresolved",
                entry,
                slot,
                selector_false,
                active_false,
                selector_true,
                active_true
            );
        }
    }
}

/// Reset only the bounded result-roster diagnostic latches. This is called at
/// the explicit start of a new match so a prior result cannot suppress the
/// first topology snapshot of the next match.
pub unsafe fn reset_result_roster_diagnostics() {
    RESULT_ROSTER_LAST_SIGNATURE = [u64::MAX; RESULT_ROSTER_PHASE_COUNT];
    RESULT_HELPER_LAST_SIGNATURE = [u64::MAX; BOSS_KIND_COUNT * 4];
}

/// Dump the public FighterManager topology without mutating any entry or
/// battle object. Inactive entries are retained in the dump when their
/// FighterEntry slot is enumerable; this is important because a stale logical
/// participant can outlive its current fighter object during result setup.
pub unsafe fn log_result_roster_snapshot(phase: &str) {
    if !crate::debug::enabled() {
        return;
    }

    let fighter_manager = crate::boss_helpers::fighter_manager();
    if fighter_manager.is_null() {
        return;
    }

    let phase_index = result_roster_phase_index(phase);
    let mut records = [ResultRosterRecord::empty(0); MAX_FIGHTERS];
    let mut signature = result_roster_text_token(phase)
        ^ (FighterManager::total_fighter_num(fighter_manager) as u64).rotate_left(7)
        ^ (FighterManager::entry_count(fighter_manager) as u64).rotate_left(17);
    for entry in 0..MAX_FIGHTERS {
        records[entry] = read_result_roster_record(fighter_manager, entry);
        signature ^= records[entry].signature().rotate_left((entry * 7) as u32);
    }

    if RESULT_ROSTER_LAST_SIGNATURE[phase_index] == signature {
        return;
    }
    RESULT_ROSTER_LAST_SIGNATURE[phase_index] = signature;

    crate::boss_log!(
        "[PB][ResultRoster] phase={} total_fighters={} entry_count={} roster_signature=0x{:x} result_mode={} topology_mutation=none",
        phase,
        FighterManager::total_fighter_num(fighter_manager),
        FighterManager::entry_count(fighter_manager),
        signature,
        FighterManager::is_result_mode(fighter_manager)
    );

    for record in records {
        if !record.present {
            continue;
        }
        crate::boss_log!(
            "[PB][ResultRoster] entry={} current_object_id=0x{:x} active={} fighter_kind={} status={} operation_cpu={} team={} team_owner_id=0x{:x} stock={} dead_count_arg0={} rebirth={} dead_status={} standby_status={} fighter_num={} sub_fighter_flag={} fighter_category=0x{:x} summon_boss_id=0x{:x} hidden_host={} result_eligibility=unobserved_public_api",
            record.entry,
            record.current_object_id,
            record.active,
            record.fighter_kind,
            record.status,
            record.operation_cpu,
            record.team,
            record.team_owner_id,
            record.stock,
            record.dead_count,
            record.rebirth,
            record.dead_status,
            record.standby_status,
            record.fighter_num,
            record.sub_fighter_flag,
            record.fighter_category,
            record.summon_boss_id,
            record.hidden_host
        );
    }
}

/// Log an item-backed Galeem/Dharkon helper alongside the FighterManager
/// topology. These helpers are acquired as items in the current source path,
/// not as new FighterEntry participants; keep that distinction explicit until
/// a native summon run proves otherwise.
pub unsafe fn log_result_roster_helper(
    phase: &str,
    helper_kind: &'static str,
    helper_object_id: u32,
    allow_object_reads: bool,
) {
    if !crate::debug::enabled() || helper_object_id == 0 {
        return;
    }

    let kind_index = if helper_kind.contains("dharkon") {
        1
    } else {
        0
    };
    let phase_index = result_roster_phase_index(phase).min(3);
    let latch_index = kind_index * 4 + phase_index;
    let active = allow_object_reads
        && sv_battle_object::is_active(helper_object_id)
        && !sv_battle_object::module_accessor(helper_object_id).is_null();
    let (kind, status) = if active {
        let boma = sv_battle_object::module_accessor(helper_object_id);
        (
            smash::app::utility::get_kind(&mut *boma),
            StatusModule::status_kind(boma),
        )
    } else {
        (-1, -1)
    };
    let signature = result_roster_text_token(phase)
        ^ result_roster_text_token(helper_kind).rotate_left(17)
        ^ (helper_object_id as u64).rotate_left(31)
        ^ ((active as u64) << 47)
        ^ (kind as u32 as u64).rotate_left(7)
        ^ (status as u32 as u64).rotate_left(13);
    if RESULT_HELPER_LAST_SIGNATURE[latch_index] == signature {
        return;
    }
    RESULT_HELPER_LAST_SIGNATURE[latch_index] = signature;

    crate::boss_log!(
        "[PB][ResultRoster] phase={} helper_kind={} object_id=0x{:x} active={} fighter_kind={} status={} classification=tracked_item_helper fighter_entry=none_observed result_eligibility=not_a_fighter_manager_entry object_reads={}",
        phase,
        helper_kind,
        helper_object_id,
        active,
        kind,
        status,
        allow_object_reads
    );
}

/// Record the native summon request boundary without changing the request.
/// Normal boss statuses are ignored, so enabling debug logs cannot create a
/// per-frame wall of output during ordinary Galeem/Dharkon gameplay.
pub unsafe fn request_native(
    kind: &'static str,
    entry: usize,
    boss_id: u32,
    boss_boma: *mut BattleObjectModuleAccessor,
    summon_status: i32,
    source: &'static str,
) {
    if !crate::debug::enabled() {
        return;
    }
    // A summon request is a battle-only action.  The central transition
    // predicate is authoritative once Ready-Go has ended; do not let a late
    // status callback reacquire a native summon during teardown.
    if crate::any_post_match_pre_result() {
        return;
    }

    let index = observation_index(kind, entry);
    let observation = &mut OBSERVATIONS[index];
    let already_pending = observation.request_pending
        && observation.last_boss_id == boss_id
        && observation.last_requested_status == summon_status;
    if already_pending {
        return;
    }

    let fighter_manager = crate::boss_helpers::fighter_manager();
    let baseline_roster = roster_snapshot(fighter_manager);
    let active = boss_id != 0 && sv_battle_object::is_active(boss_id) && !boss_boma.is_null();
    let pre_status = if active {
        StatusModule::status_kind(boss_boma)
    } else {
        -1
    };
    let pre_motion = if active {
        MotionModule::motion_kind(boss_boma)
    } else {
        0
    };

    observation.request_pending = true;
    observation.baseline_initialized = true;
    observation.roster_delta_logged = false;
    observation.last_phase = 0;
    observation.last_boss_id = boss_id;
    observation.last_requested_status = summon_status;
    observation.last_status = pre_status;
    observation.last_motion = pre_motion;
    observation.last_fighter_total = baseline_roster.total;
    observation.last_roster_signature = baseline_roster.signature;
    observation.last_candidate_signature = u64::MAX;
    observation.request_source = source;
    observation.baseline_roster = baseline_roster;
    observation.last_roster = baseline_roster;
    observation.candidate = SummonCandidate::empty();
    observation.ticks = 0;
    observation.control_state = SummonControlState::SummonRequested;

    crate::boss_log!(
        "[PB][BossSummon] kind={} entry={} request_source={} boss_id=0x{:x} boss_active={} pre_status={} requested_status={} pre_motion=0x{:x} baseline_total={} baseline_roster=0x{:x} control_state={}",
        kind,
        entry.min(MAX_FIGHTERS - 1),
        source,
        boss_id,
        active,
        pre_status,
        summon_status,
        pre_motion,
        baseline_roster.total,
        baseline_roster.signature,
        control_state_name(observation.control_state)
    );
    log_native_summon_work_snapshot(
        kind,
        entry,
        source,
        "request",
        boss_id,
        summon_status,
        pre_status,
        pre_motion,
        boss_boma,
    );
}

pub unsafe fn observe_native(
    kind: &'static str,
    entry: usize,
    boss_id: u32,
    boss_boma: *mut BattleObjectModuleAccessor,
    summon_status: i32,
    wait_status: i32,
) {
    if !crate::debug::enabled() {
        return;
    }

    let index = observation_index(kind, entry);
    let observation = &mut OBSERVATIONS[index];
    let active = boss_id != 0 && sv_battle_object::is_active(boss_id) && !boss_boma.is_null();
    let status = if active {
        StatusModule::status_kind(boss_boma)
    } else {
        -1
    };
    let motion = if active {
        MotionModule::motion_kind(boss_boma)
    } else {
        0
    };
    let fighter_manager = crate::boss_helpers::fighter_manager();
    let fighter_total = if fighter_manager.is_null() {
        -1
    } else {
        FighterManager::total_fighter_num(fighter_manager)
    };
    let current_roster = roster_snapshot(fighter_manager);
    let roster = current_roster.signature;
    let active_fighters = current_roster.active_count;
    let candidate = native_summon_candidate(&observation.baseline_roster, &current_roster);
    let candidate_signature = candidate.signature();

    let phase = if !active {
        if observation.last_phase == 1 || observation.last_phase == 2 {
            4
        } else {
            0
        }
    } else if status == summon_status {
        1
    } else if status == wait_status {
        2
    } else if observation.last_phase == 1 || observation.last_phase == 2 {
        3
    } else {
        0
    };

    if phase == 0 {
        return;
    }

    if !observation.baseline_initialized {
        observation.baseline_roster = current_roster;
        observation.baseline_initialized = true;
    }
    let phase_changed = !observation.initialized || observation.last_phase != phase;
    let roster_shape_changed = !observation.last_roster.shape_matches(&current_roster);
    let roster_delta = !observation.baseline_roster.shape_matches(&current_roster);

    observation.ticks = observation.ticks.saturating_add(1);
    let changed = phase_changed
        || observation.last_boss_id != boss_id
        || observation.last_status != status
        || observation.last_motion != motion
        || observation.last_fighter_total != fighter_total
        || observation.last_roster_signature != roster
        || observation.last_candidate_signature != candidate_signature;

    let next_control_state = if phase == 1 {
        SummonControlState::NativeStatusActive
    } else if phase == 2 {
        SummonControlState::NativeWait
    } else if phase == 3 || phase == 4 {
        SummonControlState::NativeEnded
    } else if observation.request_pending {
        SummonControlState::SummonRequested
    } else {
        SummonControlState::Inactive
    };
    let control_state_changed = observation.control_state != next_control_state;
    if phase_changed {
        log_native_summon_work_snapshot(
            kind,
            entry,
            observation.request_source,
            phase_name(phase),
            boss_id,
            observation.last_requested_status,
            status,
            motion,
            boss_boma,
        );
    }
    if changed {
        crate::boss_log!(
            "[PB][BossSummon] kind={} entry={} state={} control_state={} boss_id=0x{:x} boss_active={} status={} summon_status={} wait_status={} motion=0x{:x} fighter_total={} roster_signature=0x{:x} roster_shape_changed={} roster_delta={} control_transfer=unresolved ownership=unresolved",
            kind,
            entry.min(MAX_FIGHTERS - 1),
            phase_name(phase),
            control_state_name(next_control_state),
            boss_id,
            active,
            status,
            summon_status,
            wait_status,
            motion,
            fighter_total,
            roster,
            roster_shape_changed,
            roster_delta
        );
        if observation.last_roster_signature != roster && (phase_changed || roster_shape_changed) {
            crate::boss_log!(
                "[PB][BossSummonRoster] kind={} summon_entry={} signature=0x{:x} active_fighters={}",
                kind,
                entry.min(MAX_FIGHTERS - 1),
                roster,
                active_fighters
            );
            log_roster(kind, entry, fighter_manager);
        }
        if roster_delta && !observation.roster_delta_logged {
            log_new_roster_candidates(kind, entry, &observation.baseline_roster, &current_roster);
            observation.roster_delta_logged = true;
        }
        if candidate_signature != observation.last_candidate_signature
            && (phase == 1 || phase == 2 || phase == 3 || phase == 4)
        {
            log_native_summon_candidate(kind, entry, observation.request_source, phase, candidate);
        }
        if phase == 1 || phase == 2 {
            crate::boss_log!(
                "[PB][BossSummonControl] kind={} entry={} control_state={} native_status_active=true fighter_roster=observable roster_delta={} summon_boss_marker=unresolved native_control_source=unresolved human_control_transfer=unresolved native_fighter_create_api=unavailable controller_transfer_api=unavailable",
                kind,
                entry.min(MAX_FIGHTERS - 1),
                control_state_name(next_control_state),
                roster_delta
            );
        } else if phase == 3 || phase == 4 {
            crate::boss_log!(
                "[PB][BossSummonLifetime] kind={} entry={} state={} elapsed_samples={} cleanup=not_owned_by_plugin",
                kind,
                entry.min(MAX_FIGHTERS - 1),
                phase_name(phase),
                observation.ticks
            );
        }
    }

    if control_state_changed && crate::debug::enabled() {
        crate::boss_log!(
            "[PB][BossSummonControl] kind={} entry={} state_transition={} roster_delta={} control_transfer=unresolved ownership=unresolved",
            kind,
            entry.min(MAX_FIGHTERS - 1),
            control_state_name(next_control_state),
            roster_delta
        );
    }

    observation.initialized = true;
    observation.last_phase = phase;
    observation.last_boss_id = boss_id;
    observation.last_status = status;
    observation.last_motion = motion;
    observation.last_fighter_total = fighter_total;
    observation.last_roster_signature = roster;
    observation.last_candidate_signature = candidate_signature;
    observation.last_roster = current_roster;
    observation.candidate = candidate;
    observation.control_state = next_control_state;
    if phase == 3 || phase == 4 {
        observation.request_pending = false;
    }
}

/// Emit a bounded, read-only lifecycle snapshot for Galeem/Dharkon.
///
/// `allow_object_reads` is false once native teardown has started.  In that
/// mode this function intentionally does not resolve `boss_id` or
/// `hidden_host_item_id` into battle-object accessors.  The native summon is
/// not owned by this plugin, so a roster delta or a tracked ID is evidence for
/// diagnostics only and never a reason to destroy, reacquire, or transfer it.
pub unsafe fn audit_match_end(
    kind: &'static str,
    entry: usize,
    host_boma: *mut BattleObjectModuleAccessor,
    boss_id: u32,
    hidden_host_item_id: u32,
    boss_dead: bool,
    exists_public: bool,
    phase: &'static str,
    allow_object_reads: bool,
) {
    if !crate::debug::enabled() {
        return;
    }

    let entry = entry.min(MAX_FIGHTERS - 1);
    let index = observation_index(kind, entry);
    let fighter_manager = crate::boss_helpers::fighter_manager();
    let entry_id = FighterEntryID(entry as i32);
    let entry_ptr = if fighter_manager.is_null() {
        std::ptr::null_mut()
    } else {
        FighterManager::get_fighter_entry(fighter_manager, entry_id) as *mut FighterEntry
    };
    let info = if fighter_manager.is_null() {
        std::ptr::null_mut()
    } else {
        FighterManager::get_fighter_information(fighter_manager, entry_id)
    };
    let current_fighter_id = if entry_ptr.is_null() {
        0
    } else {
        FighterEntryBindings::current_fighter_id(entry_ptr)
    };
    let fighter_num = if entry_ptr.is_null() {
        0
    } else {
        FighterEntryBindings::fighter_num(entry_ptr)
    };
    let stock_count = if info.is_null() {
        0
    } else {
        FighterInformation::stock_count(info)
    };
    let dead_count = if info.is_null() {
        0
    } else {
        // The pinned binding exposes the native integer argument but does not
        // document its semantic label. Keep the argument explicit in logs.
        FighterInformation::dead_count(info, 0)
    };
    let is_last_dead_suicide = !info.is_null() && FighterInformation::is_last_dead_suicide(info);
    let is_on_rebirth = !info.is_null() && FighterInformation::is_on_rebirth(info);
    let ready_go = smash::app::sv_information::is_ready_go();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let host_status = if host_boma.is_null() {
        -1
    } else {
        StatusModule::status_kind(host_boma)
    };
    let host_active = !host_boma.is_null();

    let (boss_active, boss_status, boss_motion) =
        if allow_object_reads && boss_id != 0 && sv_battle_object::is_active(boss_id) {
            let boss_boma = sv_battle_object::module_accessor(boss_id);
            if boss_boma.is_null() {
                (false, -1, 0)
            } else {
                (
                    true,
                    StatusModule::status_kind(boss_boma),
                    MotionModule::motion_kind(boss_boma),
                )
            }
        } else {
            (false, -1, 0)
        };
    let hidden_host_item_active = allow_object_reads
        && hidden_host_item_id != 0
        && sv_battle_object::is_active(hidden_host_item_id);

    // The category-5 FighterManager child can outlive the ordinary summon
    // observation, so record its parent association before native result
    // construction starts.  This remains read-only and never dereferences an
    // inactive child object.
    let entry_lifecycle = observe_summon_entry_lifecycle(kind, entry, boss_id, boss_dead, phase);

    let observation = &OBSERVATIONS[index];
    let summon_phase = observation.last_phase;
    let summon_candidate = observation.candidate;
    let summon_active = summon_phase == 1 || summon_phase == 2;
    let summon_result_eligible = if entry_lifecycle.found {
        "category5_child_public_result_api_unavailable"
    } else if summon_candidate.object_id == 0 {
        "none_observed"
    } else {
        "unresolved_public_api"
    };
    let summon_cleanup = if phase == "battle" {
        "native_owner_active"
    } else {
        "observation_only"
    };

    let signature = (entry as u64).rotate_left(3)
        ^ (current_fighter_id as u64).rotate_left(7)
        ^ (fighter_num as u64).rotate_left(13)
        ^ (host_status as u32 as u64).rotate_left(19)
        ^ (stock_count as u64).rotate_left(23)
        ^ (dead_count as u64).rotate_left(29)
        ^ ((boss_dead as u64) << 35)
        ^ ((exists_public as u64) << 36)
        ^ ((ready_go as u64) << 37)
        ^ ((result_mode as u64) << 38)
        ^ ((boss_active as u64) << 39)
        ^ ((hidden_host_item_active as u64) << 40)
        ^ (boss_id as u64).rotate_left(41)
        ^ (hidden_host_item_id as u64).rotate_left(47)
        ^ (summon_candidate.signature().rotate_left(53));

    if MATCH_AUDIT_LAST_DEAD[index] < 0 || MATCH_AUDIT_LAST_DEAD[index] != boss_dead as i8 {
        MATCH_AUDIT_LAST_DEAD[index] = boss_dead as i8;
        crate::boss_log!(
            "[PB][BossDeath] boss={} entry={} dead={} host_status={} host_active={} boss_id=0x{:x} boss_active={} boss_status={} stock={} dead_count_arg0={} is_on_rebirth={} is_last_dead_suicide={} summon_active={} summon_entry={} summon_object_id=0x{:x}",
            kind,
            entry,
            boss_dead,
            host_status,
            host_active,
            boss_id,
            if allow_object_reads { boss_active } else { false },
            if allow_object_reads { boss_status } else { -1 },
            stock_count,
            dead_count,
            is_on_rebirth,
            is_last_dead_suicide,
            summon_active,
            summon_candidate.entry,
            summon_candidate.object_id
        );
    }

    if MATCH_AUDIT_LAST_SIGNATURE[index] == signature {
        return;
    }
    MATCH_AUDIT_LAST_SIGNATURE[index] = signature;

    crate::boss_log!(
        "[PB][MatchEndAudit] boss={} entry={} phase={} ready_go={} result_mode={} host_active={} host_status={} host_object_id=0x{:x} fighter_num={} stock={} dead_count_arg0={} is_on_rebirth={} is_last_dead_suicide={} boss_dead={} exists_public={} boss_id=0x{:x} boss_active={} boss_status={} boss_motion=0x{:x} hidden_host_item_id=0x{:x} hidden_host_item_active={} object_reads={} total_fighters={} entry_count={}",
        kind,
        entry,
        phase,
        ready_go,
        result_mode,
        host_active,
        host_status,
        current_fighter_id,
        fighter_num,
        stock_count,
        dead_count,
        is_on_rebirth,
        is_last_dead_suicide,
        boss_dead,
        exists_public,
        boss_id,
        if allow_object_reads { boss_active } else { false },
        if allow_object_reads { boss_status } else { -1 },
        if allow_object_reads { boss_motion } else { 0 },
        hidden_host_item_id,
        if allow_object_reads {
            hidden_host_item_active
        } else {
            false
        },
        allow_object_reads,
        if fighter_manager.is_null() {
            -1
        } else {
            FighterManager::total_fighter_num(fighter_manager)
        },
        if fighter_manager.is_null() {
            -1
        } else {
            FighterManager::entry_count(fighter_manager)
        }
    );

    crate::boss_log!(
        "[PB][SummonResultAudit] boss={} entry={} phase={} summon_active_at_match_end={} summon_entry={} summon_object_id=0x{:x} summon_kind={} summon_team={} summon_team_owner_id=0x{:x} summon_operation_cpu={} summon_boss_id=0x{:x} summon_result_eligible={} summon_cleanup={} boss_entry_state=dead:{} stock:{} rebirth:{} native_result_ready={} category5_phase={} category5_active={} category5_stock={} category5_status={} category5_parent_marker=0x{:x}",
        kind,
        entry,
        phase,
        summon_active,
        summon_candidate.entry,
        summon_candidate.object_id,
        summon_candidate.kind,
        summon_candidate.team,
        summon_candidate.team_owner_id,
        summon_candidate.operation_cpu,
        summon_candidate.summon_boss_id,
        summon_result_eligible,
        summon_cleanup,
        boss_dead,
        stock_count,
        is_on_rebirth,
        result_mode,
        SUMMON_ENTRY_LIFECYCLES[index].phase.name(),
        entry_lifecycle.active,
        entry_lifecycle.stock,
        entry_lifecycle.status,
        entry_lifecycle.summon_boss_id
    );
}

/// Clear only one boss's diagnostic observation when its visible boss is
/// eliminated. Native summon ownership remains with the game; this prevents
/// later plugin code from treating a dead boss's stale observation as an
/// invitation to reacquire or restore gameplay.
pub unsafe fn cancel_for_entry(kind: &'static str, entry: usize, reason: &'static str) {
    let index = observation_index(kind, entry);
    let observation = &mut OBSERVATIONS[index];
    if !observation.initialized {
        return;
    }
    if observation.last_boss_id != 0 {
        observe_summon_entry_lifecycle(kind, entry, observation.last_boss_id, true, "battle");
    }
    if crate::debug::enabled() && observation.control_state != SummonControlState::CancelledByResult
    {
        crate::boss_log!(
            "[PB][SummonResultAudit] kind={} entry={} summon_cleanup=observation_cancelled reason={} native_owner_untouched=true candidate_entry={} candidate_object_id=0x{:x}",
            kind,
            entry.min(MAX_FIGHTERS - 1),
            reason,
            observation.candidate.entry,
            observation.candidate.object_id
        );
    }
    observation.control_state = SummonControlState::CancelledByResult;
    observation.request_pending = false;
    observation.last_phase = 0;
}

pub unsafe fn reset(kind: &'static str, entry: usize, reason: &'static str) {
    let index = observation_index(kind, entry);
    let prior_parent_boss_id = if SUMMON_ENTRY_LIFECYCLES[index].initialized {
        SUMMON_ENTRY_LIFECYCLES[index].parent_boss_id
    } else {
        0
    };
    if prior_parent_boss_id != 0 {
        let snapshot = read_parented_summon_entry(
            crate::boss_helpers::fighter_manager(),
            prior_parent_boss_id,
        );
        if snapshot.found {
            observe_summon_entry_lifecycle(kind, entry, prior_parent_boss_id, false, "scene_exit");
        } else {
            SUMMON_ENTRY_LIFECYCLES[index] = SummonEntryLifecycle::empty();
        }
    }
    PARENT_DEATH_CLEANUPS[index] = ParentDeathCleanup::empty();
    SCENE_EXIT_LAST_SIGNATURE[index] = u64::MAX;
    let observation = &mut OBSERVATIONS[index];
    if crate::debug::enabled()
        && (observation.request_pending
            || observation.last_phase == 1
            || observation.last_phase == 2)
    {
        crate::boss_log!(
            "[PB][BossSummonLifetime] kind={} entry={} state=cancelled reason={} cleanup=observation_reset control_state={}",
            kind,
            entry.min(MAX_FIGHTERS - 1),
            reason,
            control_state_name(observation.control_state)
        );
    }
    *observation = SummonObservation::empty();
    MATCH_AUDIT_LAST_SIGNATURE[index] = u64::MAX;
    MATCH_AUDIT_LAST_DEAD[index] = -1;
}

/// Drop only the plugin's observational summon state when native match
/// teardown begins. The summon status owns any native fighter lifetime; this
/// function deliberately does not inspect or mutate a battle object.
pub unsafe fn cancel_for_transition(reason: &str) {
    for kind_index in 0..BOSS_KIND_COUNT {
        let kind = if kind_index == 0 { "galeem" } else { "dharkon" };
        for entry in 0..MAX_FIGHTERS {
            let observation = &mut OBSERVATIONS[kind_index * MAX_FIGHTERS + entry];
            if !observation.initialized {
                continue;
            }

            if observation.last_boss_id != 0 {
                observe_summon_entry_lifecycle(
                    kind,
                    entry,
                    observation.last_boss_id,
                    false,
                    reason,
                );
            }

            if crate::debug::enabled() && observation.last_phase != 0 {
                observation.control_state = SummonControlState::CancelledByResult;
                crate::boss_log!(
                    "[PB][BossSummonLifetime] kind={} entry={} state=cancelled reason={} cleanup=observation_only native_owner_untouched=true control_state={}",
                    kind,
                    entry,
                    reason,
                    control_state_name(observation.control_state)
                );
            }
            *observation = SummonObservation::empty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        native_summon_candidate, RosterSnapshot, SummonEntryLifecycle, SummonEntryLifecyclePhase,
    };

    #[test]
    fn uninitialized_child_is_not_reported_as_retired() {
        let lifecycle = SummonEntryLifecycle::empty();
        assert!(!lifecycle.initialized);
        assert!(matches!(
            lifecycle.phase,
            SummonEntryLifecyclePhase::Uninitialized
        ));
        assert_eq!(lifecycle.phase.name(), "uninitialized");
        assert_ne!(lifecycle.phase.name(), "retired");
    }

    #[test]
    fn roster_shape_ignores_nonstructural_diagnostics() {
        let mut baseline = RosterSnapshot::empty();
        baseline.total = 2;
        baseline.active_count = 1;
        baseline.active_object_ids[0] = 0x40;
        baseline.active_kinds[0] = 1;
        baseline.active_teams[0] = 3;

        let mut current = baseline;
        current.signature = 0xdead_beef;
        current.active_operation_cpu[0] = true;
        current.active_summon_boss_ids[0] = 0x5000_0000;

        assert!(baseline.shape_matches(&current));
    }

    #[test]
    fn roster_shape_detects_new_object() {
        let baseline = RosterSnapshot::empty();
        let mut current = baseline;
        current.total = 1;
        current.active_count = 1;
        current.active_object_ids[2] = 0x4002;
        current.active_kinds[2] = 0x123;
        current.active_teams[2] = 1;

        assert!(!baseline.shape_matches(&current));
    }

    #[test]
    fn candidate_detects_changed_roster_slot() {
        let mut baseline = RosterSnapshot::empty();
        baseline.total = 1;
        baseline.active_count = 1;
        baseline.active_object_ids[0] = 0x40;

        let mut current = baseline;
        current.active_object_ids[1] = 0x4001;
        current.active_kinds[1] = 0x123;
        current.active_teams[1] = 2;
        current.active_team_owner_ids[1] = 7;

        let candidate = native_summon_candidate(&baseline, &current);
        assert_eq!(candidate.entry, 1);
        assert_eq!(candidate.object_id, 0x4001);
        assert_eq!(candidate.source, 1);
    }

    #[test]
    fn candidate_prefers_a_changed_native_summon_marker() {
        let mut baseline = RosterSnapshot::empty();
        baseline.active_object_ids[0] = 0x40;
        baseline.active_object_ids[1] = 0x41;

        let mut current = baseline;
        current.active_object_ids[0] = 0x4000;
        current.active_object_ids[1] = 0x4001;
        current.active_summon_boss_ids[1] = 0x1234;

        let candidate = native_summon_candidate(&baseline, &current);
        assert_eq!(candidate.entry, 1);
        assert_eq!(candidate.object_id, 0x4001);
        assert_eq!(candidate.summon_boss_id, 0x1234);
        assert_eq!(candidate.source, 3);
    }

    #[test]
    fn candidate_does_not_treat_an_unchanged_roster_as_a_summon() {
        let mut baseline = RosterSnapshot::empty();
        baseline.active_object_ids[0] = 0x40;
        baseline.active_summon_boss_ids[0] = 0x1234;

        let candidate = native_summon_candidate(&baseline, &baseline);
        assert_eq!(candidate.object_id, 0);
        assert_eq!(candidate.source, 0);
    }
}

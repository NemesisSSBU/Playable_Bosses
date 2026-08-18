//! Result camera and presentation for playable bosses.
//!
//! Battle teardown remains quarantined. Once the native Result scene is live
//! (stage 0x136), each boss Mario host may create a presentation item and the
//! winner reuses Giga Bowser's unanimated wide camera. Galeem, Dharkon, and
//! Giga Bowser are excluded from item recreation.

use smash::app::lua_bind::{
    CameraModule, FighterManager, HitModule, ItemModule, JostleModule, LinkModule, ModelModule,
    MotionModule, PostureModule, SoundModule, StatusModule, VisibilityModule, WorkModule,
};
use smash::app::sv_battle_object;
use smash::app::{BattleObjectModuleAccessor, ItemKind};
use smash::lib::lua_const::*;
use smash::phx::{Hash40, Vector3f};

use crate::{boss_helpers, selection};

const MAX_FIGHTERS: usize = 8;
const MAX_RESULT_REFERENCE_SAMPLES: u8 = 4;
const GIGA_BOWSER_REFERENCE_STABLE_SAMPLES: u8 = 2;
const MAX_RESULT_WIDE_CAMERA_REASSERTIONS: u8 = 90;
const RESULT_ITEM_SETTLE_TICKS: u32 = 12;
const RESULT_PRESENTATION_SCALE: f32 = 0.4;
const DEFAULT_GIGA_BOWSER_RESULT_CAMERA_TYPE: i32 = 0;

// Optional hardware-captured override. Camera type 0 is Giga Bowser's native
// unanimated Result view and is used until a runtime capture replaces it.
const VERIFIED_GIGA_BOWSER_CAMERA_TYPE: Option<i32> = Some(DEFAULT_GIGA_BOWSER_RESULT_CAMERA_TYPE);
const VERIFIED_GIGA_BOWSER_CAMERA_TYPE_FOR_SAVE: Option<u64> = Some(0);
const VERIFIED_GIGA_BOWSER_CLIP_IN: Option<bool> = Some(false);
const VERIFIED_GIGA_BOWSER_CLIP_IN_ALL: Option<bool> = Some(false);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ResultPipelineStage {
    Camera,
}

/// Stage D still owns Result presentation. Item creation is limited to the
/// settled Result stage and never runs during battle teardown.
pub const ACTIVE_RESULT_PIPELINE_STAGE: ResultPipelineStage = ResultPipelineStage::Camera;

pub const RESULT_ITEM_CREATION_ENABLED: bool = true;

/// The pinned bindings do not expose a verified Result BGM lifecycle.
pub const CUSTOM_RESULT_AUDIO_ENABLED: bool = false;

#[inline(always)]
pub const fn custom_result_pipeline_enabled() -> bool {
    matches!(ACTIVE_RESULT_PIPELINE_STAGE, ResultPipelineStage::Camera)
}

#[inline(always)]
pub const fn active_result_pipeline_stage_name() -> &'static str {
    "D_camera"
}

#[derive(Copy, Clone)]
struct ResultBossProfile {
    key: &'static str,
    ui_chara_id: &'static str,
}

// Immutable battle-to-Result identity map. Item kind, motion, scale, and
// placement belonged to the removed Stage-C result-item experiment.
const RESULT_BOSS_PROFILES: [ResultBossProfile; 11] = [
    ResultBossProfile {
        key: "master_hand",
        ui_chara_id: "ui_chara_masterhand",
    },
    ResultBossProfile {
        key: "crazy_hand",
        ui_chara_id: "ui_chara_crazyhand",
    },
    ResultBossProfile {
        key: "wol_master_hand",
        ui_chara_id: "ui_chara_mewtwo_masterhand",
    },
    ResultBossProfile {
        key: "galeem",
        ui_chara_id: "ui_chara_kiila",
    },
    ResultBossProfile {
        key: "dharkon",
        ui_chara_id: "ui_chara_darz",
    },
    ResultBossProfile {
        key: "dracula",
        ui_chara_id: "ui_chara_dracula",
    },
    ResultBossProfile {
        key: "ganon_boss",
        ui_chara_id: "ui_chara_ganonboss",
    },
    ResultBossProfile {
        key: "galleom",
        ui_chara_id: "ui_chara_galleom",
    },
    ResultBossProfile {
        key: "rathalos",
        ui_chara_id: "ui_chara_lioleus",
    },
    ResultBossProfile {
        key: "marx",
        ui_chara_id: "ui_chara_marx",
    },
    ResultBossProfile {
        key: "giga_bowser",
        ui_chara_id: "ui_chara_koopag",
    },
];

/// Result only needs the logical CSS identity captured while battle hosts are
/// valid. Keeping the diagnostic signature beside it prevents a second global
/// array from representing the same entry lifetime.
#[derive(Copy, Clone)]
struct ResultIdentity {
    logical_ui_hash: u64,
    last_log_signature: u64,
}

impl ResultIdentity {
    const fn empty() -> Self {
        Self {
            logical_ui_hash: 0,
            last_log_signature: u64::MAX,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum ResultWideCameraReferenceSource {
    VerifiedGigaBowser,
    RuntimeGigaBowser,
}

impl ResultWideCameraReferenceSource {
    const fn name(self) -> &'static str {
        match self {
            Self::VerifiedGigaBowser => "verified_giga_bowser_native",
            Self::RuntimeGigaBowser => "runtime_giga_bowser_native",
        }
    }
}

/// Only `camera_type` is writable through a verified named binding. The other
/// fields document native Giga Bowser state without decoding packed values.
#[derive(Copy, Clone)]
struct ResultWideCameraReference {
    source: ResultWideCameraReferenceSource,
    camera_type: i32,
    camera_type_for_save: u64,
    clip_in: bool,
    clip_in_all: bool,
}

/// A Giga Bowser victory is the runtime discovery source. It survives Result
/// scene exits so a later hidden-Mario boss can reuse a native value from the
/// same boot.
#[derive(Copy, Clone)]
struct GigaBowserWideCameraReference {
    captured: bool,
    candidate_camera_type: u64,
    candidate_camera_type_for_save: u64,
    consecutive_samples: u8,
    camera_type: i32,
    camera_type_for_save: u64,
    clip_in: bool,
    clip_in_all: bool,
}

impl GigaBowserWideCameraReference {
    const fn empty() -> Self {
        Self {
            captured: false,
            candidate_camera_type: u64::MAX,
            candidate_camera_type_for_save: u64::MAX,
            consecutive_samples: 0,
            camera_type: -1,
            camera_type_for_save: u64::MAX,
            clip_in: false,
            clip_in_all: false,
        }
    }
}

/// Camera-only ownership is limited to a native Result callback owner. It
/// never stores or dereferences a battle item ID.
#[derive(Copy, Clone)]
struct ResultWideCameraState {
    active: bool,
    owner_object_id: u32,
    owner_entry: usize,
    winner_entry: usize,
    previous_camera_type: i32,
    applied_camera_type: i32,
    reassertions: u8,
}

impl ResultWideCameraState {
    const fn empty() -> Self {
        Self {
            active: false,
            owner_object_id: 0,
            owner_entry: usize::MAX,
            winner_entry: usize::MAX,
            previous_camera_type: -1,
            applied_camera_type: -1,
            reassertions: 0,
        }
    }
}

#[derive(Copy, Clone)]
struct ResultReferenceLogState {
    active: bool,
    tick: u32,
    last_camera_signature: u64,
    camera_samples: u8,
    last_audio_signature: u64,
    audio_samples: u8,
}

impl ResultReferenceLogState {
    const fn empty() -> Self {
        Self {
            active: false,
            tick: 0,
            last_camera_signature: u64::MAX,
            camera_samples: 0,
            last_audio_signature: u64::MAX,
            audio_samples: 0,
        }
    }
}

#[derive(Copy, Clone)]
struct ResultPresentationState {
    attempted: bool,
    object_id: u32,
}

impl ResultPresentationState {
    const fn empty() -> Self {
        Self {
            attempted: false,
            object_id: 0,
        }
    }
}

static mut RESULT_IDENTITIES: [ResultIdentity; MAX_FIGHTERS] =
    [ResultIdentity::empty(); MAX_FIGHTERS];
static mut RESULT_REFERENCE_LOGS: [ResultReferenceLogState; MAX_FIGHTERS] =
    [ResultReferenceLogState::empty(); MAX_FIGHTERS];
static mut RESULT_PRESENTATION: [ResultPresentationState; MAX_FIGHTERS] =
    [ResultPresentationState::empty(); MAX_FIGHTERS];
static mut LAST_RESULT_MODE: bool = false;
static mut LAST_WINNER_PROBE_SIGNATURE: u64 = u64::MAX;
static mut LAST_STAGE_B_SIGNATURE: u64 = u64::MAX;
static mut RESULT_SCENE_TICK: u32 = 0;
static mut GIGA_BOWSER_WIDE_CAMERA_REFERENCE: GigaBowserWideCameraReference =
    GigaBowserWideCameraReference::empty();
static mut RESULT_WIDE_CAMERA: ResultWideCameraState = ResultWideCameraState::empty();
static mut LAST_RESULT_WIDE_CAMERA_SIGNATURE: u64 = u64::MAX;

#[inline(always)]
unsafe fn result_identity_ptr(entry: usize) -> *mut ResultIdentity {
    core::ptr::addr_of_mut!(RESULT_IDENTITIES)
        .cast::<ResultIdentity>()
        .add(entry)
}

#[inline(always)]
unsafe fn result_reference_log_ptr(entry: usize) -> *mut ResultReferenceLogState {
    core::ptr::addr_of_mut!(RESULT_REFERENCE_LOGS)
        .cast::<ResultReferenceLogState>()
        .add(entry)
}

#[derive(Copy, Clone)]
pub struct ResultParticipants {
    pub entries: [usize; MAX_FIGHTERS],
    pub count: usize,
    pub final_actor_raw: u64,
    pub top_rank_count: i32,
}

impl ResultParticipants {
    #[inline(always)]
    pub fn contains(&self, entry: usize) -> bool {
        self.entries[..self.count].contains(&entry)
    }

    #[inline(always)]
    pub fn primary(&self) -> Option<usize> {
        (self.count != 0).then_some(self.entries[0])
    }
}

/// FighterManager exposes top-rank players as FighterEntryID values. The
/// generated binding uses u64 for the scalar, so entry 0 is valid and must not
/// be treated as a null pointer.
pub unsafe fn result_participants(
    fighter_manager: *mut smash::app::FighterManager,
) -> ResultParticipants {
    let mut participants = ResultParticipants {
        entries: [usize::MAX; MAX_FIGHTERS],
        count: 0,
        final_actor_raw: u64::MAX,
        top_rank_count: 0,
    };
    if fighter_manager.is_null() {
        return participants;
    }

    let final_actor_raw = FighterManager::get_final_actor_entry_id(fighter_manager);
    let top_rank_count = FighterManager::get_top_rank_player_num(fighter_manager);
    let bounded_rank_count = top_rank_count.max(0).min(MAX_FIGHTERS as i32);
    let mut top_rank_values = [u64::MAX; MAX_FIGHTERS];

    for rank in 0..bounded_rank_count {
        let raw_entry = FighterManager::get_top_rank_player(fighter_manager, rank);
        top_rank_values[rank as usize] = raw_entry;
        if raw_entry >= MAX_FIGHTERS as u64 {
            continue;
        }
        let entry = raw_entry as usize;
        if !participants.contains(entry) {
            participants.entries[participants.count] = entry;
            participants.count += 1;
        }
    }

    // Early Result frames can expose final_actor_entry_id before top-rank data.
    if participants.count == 0 && final_actor_raw < MAX_FIGHTERS as u64 {
        participants.entries[0] = final_actor_raw as usize;
        participants.count = 1;
    }

    participants.final_actor_raw = final_actor_raw;
    participants.top_rank_count = top_rank_count;

    if crate::debug::enabled() {
        let mut signature = final_actor_raw ^ ((top_rank_count as u32 as u64) << 31);
        for (index, raw) in top_rank_values.iter().enumerate() {
            signature ^= raw.rotate_left((index as u32 * 7) + 3);
        }
        for (index, entry) in participants.entries.iter().enumerate() {
            signature ^= (*entry as u64).rotate_left((index as u32 * 5) + 17);
        }
        if signature != LAST_WINNER_PROBE_SIGNATURE {
            LAST_WINNER_PROBE_SIGNATURE = signature;
            crate::boss_log!(
                "[PB][ResultResolve] final_actor_raw=0x{:x} top_rank_count={} top_rank_values=[0x{:x},0x{:x},0x{:x},0x{:x}] winning_entries=[{}] source={}",
                final_actor_raw,
                top_rank_count,
                top_rank_values[0],
                top_rank_values[1],
                top_rank_values[2],
                top_rank_values[3],
                participants.entries[..participants.count]
                    .iter()
                    .map(|entry| entry.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                if participants.count == 0 {
                    "unresolved"
                } else if top_rank_count > 0 {
                    "top_rank_entry_ids"
                } else {
                    "final_actor_entry"
                }
            );
        }
    }

    participants
}

#[inline(always)]
fn result_profile_for_ui_hash(ui_chara_hash: u64) -> Option<&'static ResultBossProfile> {
    RESULT_BOSS_PROFILES
        .iter()
        .find(|profile| crate::to_hash40(profile.ui_chara_id).0 == ui_chara_hash)
}

/// Result overlay identity is match-local. A persist/cache hash on a later
/// vanilla Mario must not keep a previous boss (Marx, etc.) parked on that
/// entry until results.
#[inline(always)]
fn next_result_identity(
    current: ResultIdentity,
    hosting: bool,
    logical_ui_hash: u64,
) -> ResultIdentity {
    if hosting {
        if result_profile_for_ui_hash(logical_ui_hash).is_some() {
            return ResultIdentity {
                logical_ui_hash,
                last_log_signature: current.last_log_signature,
            };
        }
        return current;
    }
    if current.logical_ui_hash == 0 {
        return current;
    }
    ResultIdentity::empty()
}

pub unsafe fn reset_battle_identity(entry: usize) {
    if entry >= MAX_FIGHTERS {
        return;
    }
    *result_identity_ptr(entry) = ResultIdentity::empty();
}

#[inline(always)]
unsafe fn winning_result_profile(entry: usize) -> Option<&'static ResultBossProfile> {
    if entry >= MAX_FIGHTERS {
        return None;
    }
    result_profile_for_ui_hash((*result_identity_ptr(entry)).logical_ui_hash)
}

#[inline(always)]
fn camera_type_as_i32(camera_type: u64) -> Option<i32> {
    (camera_type <= i32::MAX as u64).then_some(camera_type as i32)
}

/// A compile-time reference stays unavailable until hardware proves a complete
/// native Giga Bowser tuple.
#[inline(always)]
fn verified_giga_bowser_wide_camera_reference() -> Option<ResultWideCameraReference> {
    match (
        VERIFIED_GIGA_BOWSER_CAMERA_TYPE,
        VERIFIED_GIGA_BOWSER_CAMERA_TYPE_FOR_SAVE,
        VERIFIED_GIGA_BOWSER_CLIP_IN,
        VERIFIED_GIGA_BOWSER_CLIP_IN_ALL,
    ) {
        (Some(camera_type), Some(camera_type_for_save), Some(clip_in), Some(clip_in_all)) => {
            Some(ResultWideCameraReference {
                source: ResultWideCameraReferenceSource::VerifiedGigaBowser,
                camera_type,
                camera_type_for_save,
                clip_in,
                clip_in_all,
            })
        }
        _ => None,
    }
}

#[inline(always)]
fn result_presentation_allowed(key: &str) -> bool {
    !matches!(key, "galeem" | "dharkon" | "giga_bowser")
}

#[inline(always)]
unsafe fn result_item_kind_for_profile(key: &str) -> Option<i32> {
    if !result_presentation_allowed(key) {
        return None;
    }
    match key {
        "master_hand" | "wol_master_hand" => Some(*ITEM_KIND_MASTERHAND),
        "crazy_hand" => Some(*ITEM_KIND_CRAZYHAND),
        "dracula" => Some(*ITEM_KIND_DRACULA),
        "ganon_boss" => Some(*ITEM_KIND_GANONBOSS),
        "galleom" => Some(*ITEM_KIND_GALLEOM),
        "rathalos" => Some(*ITEM_KIND_LIOLEUSBOSS),
        "marx" => Some(*ITEM_KIND_MARX),
        _ => None,
    }
}

#[inline(always)]
fn result_idle_motion_for_profile(key: &str) -> &'static str {
    match key {
        "rathalos" => "hovering_move",
        _ => "wait",
    }
}

#[inline(always)]
unsafe fn active_giga_bowser_wide_camera_reference() -> Option<ResultWideCameraReference> {
    let runtime_reference = GIGA_BOWSER_WIDE_CAMERA_REFERENCE;
    if runtime_reference.captured {
        return Some(ResultWideCameraReference {
            source: ResultWideCameraReferenceSource::RuntimeGigaBowser,
            camera_type: runtime_reference.camera_type,
            camera_type_for_save: runtime_reference.camera_type_for_save,
            clip_in: runtime_reference.clip_in,
            clip_in_all: runtime_reference.clip_in_all,
        });
    }
    if let Some(reference) = verified_giga_bowser_wide_camera_reference() {
        return Some(reference);
    }
    Some(ResultWideCameraReference {
        source: ResultWideCameraReferenceSource::VerifiedGigaBowser,
        camera_type: DEFAULT_GIGA_BOWSER_RESULT_CAMERA_TYPE,
        camera_type_for_save: 0,
        clip_in: false,
        clip_in_all: false,
    })
}

#[inline(always)]
unsafe fn result_callback_object_id(module_accessor: *mut BattleObjectModuleAccessor) -> u32 {
    if module_accessor.is_null() {
        0
    } else {
        (*module_accessor).battle_object_id
    }
}

/// Capture two consecutive native Giga Bowser samples before treating the
/// writable camera type as reusable. Packed state is recorded, never decoded.
#[inline(always)]
unsafe fn capture_giga_bowser_wide_camera_reference(
    entry: usize,
    fighter_status: i32,
    camera_type: u64,
    camera_type_for_save: u64,
    clip_in: bool,
    clip_in_all: bool,
    stage_id: i32,
) {
    let mut reference = GIGA_BOWSER_WIDE_CAMERA_REFERENCE;
    let Some(camera_type_i32) = camera_type_as_i32(camera_type) else {
        if crate::debug::enabled() && reference.candidate_camera_type != camera_type {
            reference.candidate_camera_type = camera_type;
            GIGA_BOWSER_WIDE_CAMERA_REFERENCE = reference;
            crate::boss_log!(
                "[PB][ResultWideCamera] action=reference_rejected source=giga_bowser_native entry={} stage=0x{:x} camera_type=0x{:x} reason=outside_i32_setter_range",
                entry,
                stage_id,
                camera_type
            );
        }
        return;
    };

    if reference.captured {
        return;
    }

    if reference.candidate_camera_type == camera_type
        && reference.candidate_camera_type_for_save == camera_type_for_save
    {
        reference.consecutive_samples = reference.consecutive_samples.saturating_add(1);
    } else {
        reference.candidate_camera_type = camera_type;
        reference.candidate_camera_type_for_save = camera_type_for_save;
        reference.consecutive_samples = 1;
    }
    GIGA_BOWSER_WIDE_CAMERA_REFERENCE = reference;

    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][ResultWideCamera] action=reference_sample source=giga_bowser_native entry={} stage=0x{:x} fighter_status={} camera_type=0x{:x} camera_type_for_save=0x{:x} clip_in={} clip_in_all={} stable_samples={}/{}",
            entry,
            stage_id,
            fighter_status,
            camera_type,
            camera_type_for_save,
            clip_in,
            clip_in_all,
            reference.consecutive_samples,
            GIGA_BOWSER_REFERENCE_STABLE_SAMPLES
        );
    }

    if reference.consecutive_samples < GIGA_BOWSER_REFERENCE_STABLE_SAMPLES {
        return;
    }

    reference.captured = true;
    reference.camera_type = camera_type_i32;
    reference.camera_type_for_save = camera_type_for_save;
    reference.clip_in = clip_in;
    reference.clip_in_all = clip_in_all;
    GIGA_BOWSER_WIDE_CAMERA_REFERENCE = reference;
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][ResultWideCamera] action=reference_captured source=giga_bowser_native entry={} stage=0x{:x} camera_type=0x{:x} camera_type_for_save=0x{:x} clip_in={} clip_in_all={} writable_state=camera_type_only mutation=none",
            entry,
            stage_id,
            camera_type,
            camera_type_for_save,
            clip_in,
            clip_in_all
        );
    }
}

#[inline(always)]
unsafe fn reset_result_reference_scene(entry: usize) {
    if entry < MAX_FIGHTERS {
        *result_reference_log_ptr(entry) = ResultReferenceLogState::empty();
    }
}

#[inline(always)]
unsafe fn begin_result_reference_scene(entry: usize) -> u32 {
    if entry >= MAX_FIGHTERS {
        return 0;
    }
    let state_ptr = result_reference_log_ptr(entry);
    let mut state = *state_ptr;
    if !state.active {
        state = ResultReferenceLogState::empty();
        state.active = true;
    }
    state.tick = state.tick.saturating_add(1);
    *state_ptr = state;
    state.tick
}

#[inline(always)]
fn result_reference_source(
    fighter_kind: i32,
    profile: Option<&ResultBossProfile>,
    source_override: Option<&'static str>,
) -> &'static str {
    if let Some(source) = source_override {
        return source;
    }
    if fighter_kind == *FIGHTER_KIND_MARIO {
        if profile.is_some() {
            "hidden_mario_boss"
        } else {
            "mario_native_fighter"
        }
    } else {
        "ordinary_native_fighter"
    }
}

#[inline(always)]
fn result_reference_logical_boss(
    fighter_kind: i32,
    profile: Option<&ResultBossProfile>,
    source: &str,
) -> &'static str {
    if let Some(profile) = profile {
        return profile.key;
    }
    if fighter_kind == *FIGHTER_KIND_KOOPAG || source == "giga_bowser_native_fighter" {
        return "giga_bowser";
    }
    "none"
}

/// Emit bounded public Result observations. The pinned target/range bindings
/// return opaque packed values, so they are deliberately not decoded.
#[inline(always)]
unsafe fn observe_result_reference(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry: usize,
    participants: ResultParticipants,
    source_override: Option<&'static str>,
) {
    if module_accessor.is_null() || entry >= MAX_FIGHTERS {
        return;
    }

    let tick = begin_result_reference_scene(entry);
    let profile = winning_result_profile(entry);
    let fighter_kind = smash::app::utility::get_kind(&mut *module_accessor);
    let source = result_reference_source(fighter_kind, profile, source_override);
    let logical_boss = result_reference_logical_boss(fighter_kind, profile, source);
    let fighter_status = StatusModule::status_kind(module_accessor);
    let camera_type = CameraModule::get_camera_type(module_accessor);
    let camera_type_for_save = CameraModule::get_camera_type_for_save(module_accessor);
    let clip_in = CameraModule::is_clip_in(module_accessor, false);
    let clip_in_all = CameraModule::is_clip_in_all(module_accessor, false);
    let winner = participants.contains(entry);
    let stage_id = smash::app::stage::get_stage_id();

    if crate::debug::enabled() {
        let camera_signature = crate::to_hash40(source).0
            ^ crate::to_hash40(logical_boss).0.rotate_left(7)
            ^ ((fighter_kind as u32 as u64) << 11)
            ^ ((fighter_status as u32 as u64) << 29)
            ^ camera_type.rotate_left(13)
            ^ camera_type_for_save.rotate_left(31)
            ^ ((clip_in as u64) << 59)
            ^ ((clip_in_all as u64) << 60)
            ^ ((winner as u64) << 61);
        let state_ptr = result_reference_log_ptr(entry);
        let mut log_state = *state_ptr;
        if log_state.last_camera_signature != camera_signature
            && log_state.camera_samples < MAX_RESULT_REFERENCE_SAMPLES
        {
            log_state.last_camera_signature = camera_signature;
            log_state.camera_samples = log_state.camera_samples.saturating_add(1);
            *state_ptr = log_state;
            crate::boss_log!(
                "[PB][ResultCameraReference] source={} tick={} sample={}/{} entry={} winner={} logical_boss={} stage=0x{:x} fighter_kind={} fighter_status={} camera_type=0x{:x} camera_type_for_save=0x{:x} clip_in={} clip_in_all={} target=unavailable_packed_binding range=unavailable_packed_binding",
                source,
                tick,
                log_state.camera_samples,
                MAX_RESULT_REFERENCE_SAMPLES,
                entry,
                winner,
                logical_boss,
                stage_id,
                fighter_kind,
                fighter_status,
                camera_type,
                camera_type_for_save,
                clip_in,
                clip_in_all
            );
        }
    }

    if source == "giga_bowser_native_fighter" && winner {
        capture_giga_bowser_wide_camera_reference(
            entry,
            fighter_status,
            camera_type,
            camera_type_for_save,
            clip_in,
            clip_in_all,
            stage_id,
        );
    }

    // Audio is diagnostic-only. Keep its extra engine queries out of the Result
    // callback when diagnostics are disabled.
    if crate::debug::enabled() {
        let hidden_host = boss_helpers::is_hidden_host(module_accessor);
        let voice_active = SoundModule::is_playing_voice(module_accessor);
        let audio_signature = crate::to_hash40(source).0
            ^ crate::to_hash40(logical_boss).0.rotate_left(9)
            ^ ((fighter_kind as u32 as u64) << 17)
            ^ ((winner as u64) << 58)
            ^ ((hidden_host as u64) << 59)
            ^ ((voice_active as u64) << 60);
        let state_ptr = result_reference_log_ptr(entry);
        let mut log_state = *state_ptr;
        if log_state.last_audio_signature != audio_signature
            && log_state.audio_samples < MAX_RESULT_REFERENCE_SAMPLES
        {
            log_state.last_audio_signature = audio_signature;
            log_state.audio_samples = log_state.audio_samples.saturating_add(1);
            *state_ptr = log_state;
            crate::boss_log!(
                "[PB][ResultAudioAudit] source={} tick={} sample={}/{} entry={} winner={} logical_boss={} fighter_kind={} hidden_host={} voice_active={} hidden_host_sfx_suppression={} custom_result_audio_enabled={} result_bgm_lookup=unexposed_by_pinned_bindings status_bgm_api=opaque_enum_only action=none",
                source,
                tick,
                log_state.audio_samples,
                MAX_RESULT_REFERENCE_SAMPLES,
                entry,
                winner,
                logical_boss,
                fighter_kind,
                hidden_host,
                voice_active,
                hidden_host,
                CUSTOM_RESULT_AUDIO_ENABLED
            );
        }
    }
}

/// Giga Bowser is a native fighter agent, not a Mario host. This read-only
/// callback captures the native Result reference before the shared quarantine
/// exits the fighter frame.
pub unsafe fn observe_native_fighter_result_reference(
    module_accessor: *mut BattleObjectModuleAccessor,
) {
    if module_accessor.is_null()
        || smash::app::utility::get_kind(&mut *module_accessor) != *FIGHTER_KIND_KOOPAG
    {
        return;
    }
    let entry = boss_helpers::entry_id(module_accessor);
    if entry >= MAX_FIGHTERS {
        return;
    }
    let fighter_manager = boss_helpers::fighter_manager();
    if fighter_manager.is_null() || !FighterManager::is_result_mode(fighter_manager) {
        reset_result_reference_scene(entry);
        return;
    }
    observe_result_reference(
        module_accessor,
        entry,
        result_participants(fighter_manager),
        Some("giga_bowser_native_fighter"),
    );
}

/// Snapshot only the logical boss identity while this Mario is actually hosting
/// a boss. Persist/cache can still name a previous CSS boss for a later vanilla
/// Mario; that must not become a results overlay.
pub unsafe fn observe_battle_identity(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() {
        return;
    }
    let stage_id = smash::app::stage::get_stage_id();
    if stage_id == boss_helpers::STAGE_ID_RESULT || boss_helpers::is_boss_preview_stage(stage_id) {
        return;
    }

    let entry = boss_helpers::entry_id(module_accessor);
    if entry >= MAX_FIGHTERS {
        return;
    }
    let hosting = boss_helpers::is_hidden_host(module_accessor);
    let logical_ui_hash = selection::selected_css_boss_selector_id(module_accessor).unwrap_or(0);
    let state_ptr = result_identity_ptr(entry);
    let previous = *state_ptr;
    let mut state = next_result_identity(previous, hosting, logical_ui_hash);
    let signature = (state.logical_ui_hash) ^ ((hosting as u64) << 63);
    if crate::debug::enabled() && state.last_log_signature != signature {
        state.last_log_signature = signature;
        let profile = result_profile_for_ui_hash(state.logical_ui_hash);
        crate::boss_log!(
            "[PB][ResultResolve] phase=battle_identity entry={} hosting={} logical_boss={} logical_ui_hash=0x{:010x} previous_ui_hash=0x{:010x} result_safe={}",
            entry,
            hosting,
            profile.map(|profile| profile.key).unwrap_or("none"),
            state.logical_ui_hash,
            previous.logical_ui_hash,
            profile
                .map(|profile| profile.key != "galeem" && profile.key != "dharkon")
                .unwrap_or(false)
        );
    }
    *state_ptr = state;
}

#[inline(always)]
unsafe fn log_result_winner_resolution(
    participants: ResultParticipants,
    primary_winner_entry: usize,
) {
    let signature = (participants.count as u64)
        ^ ((primary_winner_entry as u64) << 8)
        ^ participants.final_actor_raw.rotate_left(17);
    if !crate::debug::enabled() || LAST_STAGE_B_SIGNATURE == signature {
        return;
    }
    LAST_STAGE_B_SIGNATURE = signature;
    let scene_tick = RESULT_SCENE_TICK;
    crate::boss_log!(
        "[PB][ResultBisect] stage=B step=winner_resolve_only ok tick={} winning_entries=[{}] mutations=camera_only_reference_gated",
        scene_tick,
        participants.entries[..participants.count]
            .iter()
            .map(|entry| entry.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
}

#[inline(always)]
unsafe fn should_log_result_wide_camera(signature: u64) -> bool {
    if !crate::debug::enabled() || LAST_RESULT_WIDE_CAMERA_SIGNATURE == signature {
        return false;
    }
    LAST_RESULT_WIDE_CAMERA_SIGNATURE = signature;
    true
}

/// Apply only a hardware-verified native Giga Bowser camera type to a Result
/// callback host. No battle object, item slot, packed camera target, range, or
/// result item is read or written here.
#[inline(always)]
unsafe fn apply_result_wide_camera(
    module_accessor: *mut BattleObjectModuleAccessor,
    owner_entry: usize,
    primary_winner_entry: usize,
) {
    let Some(profile) = winning_result_profile(primary_winner_entry) else {
        return;
    };
    if owner_entry != primary_winner_entry {
        return;
    }
    let stage_id = smash::app::stage::get_stage_id();

    if profile.key == "giga_bowser" {
        let signature = ((primary_winner_entry as u64) << 32) ^ 0x4749_4741;
        if should_log_result_wide_camera(signature) {
            crate::boss_log!(
                "[PB][ResultWideCamera] entry={} logical_boss=giga_bowser reference=giga_bowser_native mutation=native_unchanged stage=0x{:x}",
                primary_winner_entry,
                stage_id
            );
        }
        return;
    }

    let current_object_id = result_callback_object_id(module_accessor);
    let active_camera = RESULT_WIDE_CAMERA;
    if active_camera.active {
        if current_object_id == 0 || current_object_id != active_camera.owner_object_id {
            return;
        }

        let observed_camera_type = CameraModule::get_camera_type(module_accessor);
        if camera_type_as_i32(observed_camera_type) == Some(active_camera.applied_camera_type) {
            return;
        }
        if active_camera.reassertions >= MAX_RESULT_WIDE_CAMERA_REASSERTIONS {
            let signature = ((active_camera.winner_entry as u64) << 32)
                ^ observed_camera_type.rotate_left(9)
                ^ 0x5245_4C4D;
            if should_log_result_wide_camera(signature) {
                crate::boss_log!(
                    "[PB][ResultWideCamera] entry={} owner_entry={} logical_boss={} reference=giga_bowser_native observed_camera_type=0x{:x} applied_camera_type=0x{:x} action=reassertion_budget_exhausted max_reassertions={} mutation=none stage=0x{:x}",
                    active_camera.winner_entry,
                    active_camera.owner_entry,
                    winning_result_profile(active_camera.winner_entry)
                        .map(|profile| profile.key)
                        .unwrap_or("none"),
                    observed_camera_type,
                    active_camera.applied_camera_type,
                    MAX_RESULT_WIDE_CAMERA_REASSERTIONS,
                    stage_id
                );
            }
            return;
        }

        CameraModule::set_camera_type(module_accessor, active_camera.applied_camera_type);
        let reapplied_camera_type = CameraModule::get_camera_type(module_accessor);
        let mut updated_camera = active_camera;
        updated_camera.reassertions = updated_camera.reassertions.saturating_add(1);
        RESULT_WIDE_CAMERA = updated_camera;
        let signature = ((active_camera.winner_entry as u64) << 32)
            ^ reapplied_camera_type.rotate_left(9)
            ^ ((updated_camera.reassertions as u64) << 56)
            ^ 0x5245_4153;
        if should_log_result_wide_camera(signature) {
            crate::boss_log!(
                "[PB][ResultWideCamera] entry={} owner_entry={} logical_boss={} reference=giga_bowser_native previous_camera_type=0x{:x} applied_camera_type=0x{:x} observed_camera_type=0x{:x} action=reassert_camera_type reassertion={}/{} stage=0x{:x}",
                active_camera.winner_entry,
                active_camera.owner_entry,
                winning_result_profile(active_camera.winner_entry)
                    .map(|profile| profile.key)
                    .unwrap_or("none"),
                observed_camera_type,
                active_camera.applied_camera_type,
                reapplied_camera_type,
                updated_camera.reassertions,
                MAX_RESULT_WIDE_CAMERA_REASSERTIONS,
                stage_id
            );
        }
        return;
    }

    let Some(reference) = active_giga_bowser_wide_camera_reference() else {
        let signature =
            ((primary_winner_entry as u64) << 32) ^ crate::to_hash40(profile.key).0 ^ 0x4445_4645;
        if should_log_result_wide_camera(signature) {
            crate::boss_log!(
                "[PB][ResultWideCamera] entry={} owner_entry={} logical_boss={} reference=giga_bowser_native mutation=deferred reason=reference_unavailable verified_reference=false runtime_reference=false stage=0x{:x}",
                primary_winner_entry,
                owner_entry,
                profile.key,
                stage_id
            );
        }
        return;
    };

    let current_camera_type_raw = CameraModule::get_camera_type(module_accessor);
    let Some(previous_camera_type) = camera_type_as_i32(current_camera_type_raw) else {
        let signature =
            ((primary_winner_entry as u64) << 32) ^ current_camera_type_raw ^ 0x5241_4E47;
        if should_log_result_wide_camera(signature) {
            crate::boss_log!(
                "[PB][ResultWideCamera] entry={} owner_entry={} logical_boss={} reference={} previous_camera_type=0x{:x} mutation=deferred reason=owner_camera_type_outside_i32_setter_range stage=0x{:x}",
                primary_winner_entry,
                owner_entry,
                profile.key,
                reference.source.name(),
                current_camera_type_raw,
                stage_id
            );
        }
        return;
    };

    let reference_camera_type = reference.camera_type;
    CameraModule::reset_all(module_accessor);
    CameraModule::set_camera_type(module_accessor, reference_camera_type);
    let applied_camera_type = CameraModule::get_camera_type(module_accessor);
    let observed_camera_type_for_save = CameraModule::get_camera_type_for_save(module_accessor);
    let observed_clip_in = CameraModule::is_clip_in(module_accessor, false);
    let observed_clip_in_all = CameraModule::is_clip_in_all(module_accessor, false);

    RESULT_WIDE_CAMERA = ResultWideCameraState {
        active: true,
        owner_object_id: current_object_id,
        owner_entry,
        winner_entry: primary_winner_entry,
        previous_camera_type,
        applied_camera_type: reference_camera_type,
        reassertions: 0,
    };
    let signature = ((primary_winner_entry as u64) << 32)
        ^ ((owner_entry as u64) << 16)
        ^ (reference_camera_type as u32 as u64);
    if should_log_result_wide_camera(signature) {
        crate::boss_log!(
            "[PB][ResultWideCamera] entry={} owner_entry={} logical_boss={} reference={} reference_camera_type_for_save=0x{:x} observed_camera_type_for_save=0x{:x} reference_clip_in={} observed_clip_in={} reference_clip_in_all={} observed_clip_in_all={} previous_camera_type=0x{:x} applied_camera_type=0x{:x} observed_camera_type=0x{:x} mutation=set_camera_type visual_equivalence=hardware_unverified stage=0x{:x}",
            primary_winner_entry,
            owner_entry,
            profile.key,
            reference.source.name(),
            reference.camera_type_for_save,
            observed_camera_type_for_save,
            reference.clip_in,
            observed_clip_in,
            reference.clip_in_all,
            observed_clip_in_all,
            previous_camera_type,
            reference_camera_type,
            applied_camera_type,
            stage_id
        );
    }
}

#[inline(always)]
unsafe fn restore_result_wide_camera(
    module_accessor: *mut BattleObjectModuleAccessor,
    reason: &str,
) {
    let active_camera = RESULT_WIDE_CAMERA;
    if !active_camera.active {
        return;
    }

    let current_object_id = result_callback_object_id(module_accessor);
    if current_object_id != 0 && current_object_id == active_camera.owner_object_id {
        CameraModule::set_camera_type(module_accessor, active_camera.previous_camera_type);
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][ResultWideCamera] action=restore reason={} owner_entry={} winner_entry={} owner_object_id=0x{:x} applied_camera_type=0x{:x} restored_camera_type=0x{:x}",
                reason,
                active_camera.owner_entry,
                active_camera.winner_entry,
                active_camera.owner_object_id,
                active_camera.applied_camera_type,
                active_camera.previous_camera_type
            );
        }
    } else if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][ResultWideCamera] action=native_scene_teardown_owner_unavailable reason={} owner_entry={} winner_entry={} owner_object_id=0x{:x} current_object_id=0x{:x} applied_camera_type=0x{:x} mutation=none",
            reason,
            active_camera.owner_entry,
            active_camera.winner_entry,
            active_camera.owner_object_id,
            current_object_id,
            active_camera.applied_camera_type
        );
    }
    RESULT_WIDE_CAMERA = ResultWideCameraState::empty();
}

#[inline(always)]
unsafe fn reset_result_presentation_scene() {
    RESULT_PRESENTATION = [ResultPresentationState::empty(); MAX_FIGHTERS];
}

#[inline(always)]
unsafe fn result_host_pos(module_accessor: *mut BattleObjectModuleAccessor) -> Vector3f {
    Vector3f {
        x: PostureModule::pos_x(module_accessor),
        y: PostureModule::pos_y(module_accessor),
        z: PostureModule::pos_z(module_accessor),
    }
}

#[inline(always)]
unsafe fn result_item_is_held_by_host(
    module_accessor: *mut BattleObjectModuleAccessor,
    item_id: u32,
) -> bool {
    if module_accessor.is_null() || item_id == 0 {
        return false;
    }
    for slot in 0..4 {
        if ItemModule::is_have_item(module_accessor, slot)
            && ItemModule::get_have_item_id(module_accessor, slot) as u32 == item_id
        {
            return true;
        }
    }
    false
}

#[inline(always)]
unsafe fn unlink_result_presentation_item(item_boma: *mut BattleObjectModuleAccessor) {
    if item_boma.is_null() {
        return;
    }
    LinkModule::remove_model_constraint(item_boma, true);
    if LinkModule::is_link(item_boma, *ITEM_LINK_NO_HAVE) {
        LinkModule::unlink(item_boma, *ITEM_LINK_NO_HAVE);
    }
    WorkModule::on_flag(
        item_boma,
        *ITEM_INSTANCE_WORK_FLAG_DISABLE_AUTO_GRAVITY_MOVE,
    );
    WorkModule::on_flag(item_boma, *ITEM_INSTANCE_WORK_FLAG_IGNORE_DELETE_BY_STAGE);
}

#[inline(always)]
unsafe fn detach_result_presentation_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    item_id: u32,
    item_boma: *mut BattleObjectModuleAccessor,
    request_wait: bool,
) {
    if module_accessor.is_null() || item_id == 0 || item_boma.is_null() {
        return;
    }
    boss_helpers::release_tracked_item_from_host(module_accessor, item_id);
    unlink_result_presentation_item(item_boma);
    if request_wait {
        StatusModule::change_status_request_from_script(item_boma, *ITEM_STATUS_KIND_WAIT, true);
    }
}

#[inline(always)]
unsafe fn maintain_result_presentation_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    item_boma: *mut BattleObjectModuleAccessor,
    item_id: u32,
    profile: &ResultBossProfile,
    initialize: bool,
) {
    if module_accessor.is_null() || item_boma.is_null() {
        return;
    }
    let still_held = result_item_is_held_by_host(module_accessor, item_id);
    if initialize || still_held {
        detach_result_presentation_item(module_accessor, item_id, item_boma, true);
    } else {
        unlink_result_presentation_item(item_boma);
    }
    HitModule::set_whole(item_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
    JostleModule::set_status(item_boma, false);
    VisibilityModule::set_whole(item_boma, true);
    ModelModule::set_scale(item_boma, RESULT_PRESENTATION_SCALE);
    let motion = result_idle_motion_for_profile(profile.key);
    if initialize || MotionModule::motion_kind(item_boma) != smash::hash40(motion) {
        MotionModule::change_motion(
            item_boma,
            Hash40::new(motion),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
    }
    PostureModule::set_pos(item_boma, &result_host_pos(module_accessor));
}

#[inline(always)]
unsafe fn apply_result_presentation(
    module_accessor: *mut BattleObjectModuleAccessor,
    entry: usize,
) {
    if !RESULT_ITEM_CREATION_ENABLED || module_accessor.is_null() || entry >= MAX_FIGHTERS {
        return;
    }
    if smash::app::stage::get_stage_id() != boss_helpers::STAGE_ID_RESULT {
        return;
    }
    if RESULT_SCENE_TICK < RESULT_ITEM_SETTLE_TICKS {
        return;
    }

    let Some(profile) = winning_result_profile(entry) else {
        return;
    };
    let Some(item_kind) = result_item_kind_for_profile(profile.key) else {
        RESULT_PRESENTATION[entry].attempted = true;
        return;
    };

    let state = RESULT_PRESENTATION[entry];
    if state.object_id != 0 {
        if sv_battle_object::is_active(state.object_id) {
            maintain_result_presentation_item(
                module_accessor,
                sv_battle_object::module_accessor(state.object_id),
                state.object_id,
                profile,
                false,
            );
        }
        return;
    }
    if state.attempted {
        return;
    }

    if let Some((_, held_id, held_boma)) =
        boss_helpers::held_item_by_kind(module_accessor, &[item_kind])
    {
        RESULT_PRESENTATION[entry] = ResultPresentationState {
            attempted: true,
            object_id: held_id,
        };
        maintain_result_presentation_item(module_accessor, held_boma, held_id, profile, true);
        return;
    }

    ItemModule::have_item(module_accessor, ItemKind(item_kind), 0, 0, false, false);
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);
    RESULT_PRESENTATION[entry].attempted = true;
    if let Some((_, held_id, held_boma)) =
        boss_helpers::held_item_by_kind(module_accessor, &[item_kind])
    {
        RESULT_PRESENTATION[entry].object_id = held_id;
        maintain_result_presentation_item(module_accessor, held_boma, held_id, profile, true);
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][ResultPresentation] action=spawned entry={} logical_boss={} item_kind={} object_id=0x{:x} stage=0x{:x}",
                entry,
                profile.key,
                item_kind,
                held_id,
                boss_helpers::STAGE_ID_RESULT
            );
        }
    } else if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][ResultPresentation] action=spawn_failed entry={} logical_boss={} item_kind={} stage=0x{:x}",
            entry,
            profile.key,
            item_kind,
            boss_helpers::STAGE_ID_RESULT
        );
    }
}

/// Runs from the Mario host callback. Result mutations are restricted to the
/// native top-rank winner set. Entry 0 is a valid result winner.
pub unsafe fn frame(module_accessor: *mut BattleObjectModuleAccessor) {
    if !custom_result_pipeline_enabled() || module_accessor.is_null() {
        return;
    }

    let fighter_manager = boss_helpers::fighter_manager();
    let result_mode = !fighter_manager.is_null() && FighterManager::is_result_mode(fighter_manager);
    let entry = boss_helpers::entry_id(module_accessor);
    if !result_mode {
        reset_result_reference_scene(entry);
        if LAST_RESULT_MODE {
            restore_result_wide_camera(module_accessor, "result_exit");
            reset_result_presentation_scene();
            if crate::debug::enabled() {
                crate::boss_log!("[PB][ResultCamera] scene_exit reason=result_exit restored=true");
            }
        }
        LAST_RESULT_MODE = false;
        LAST_WINNER_PROBE_SIGNATURE = u64::MAX;
        LAST_STAGE_B_SIGNATURE = u64::MAX;
        LAST_RESULT_WIDE_CAMERA_SIGNATURE = u64::MAX;
        RESULT_SCENE_TICK = 0;
        return;
    }

    if !LAST_RESULT_MODE {
        LAST_RESULT_MODE = true;
        RESULT_SCENE_TICK = 0;
        LAST_WINNER_PROBE_SIGNATURE = u64::MAX;
        LAST_STAGE_B_SIGNATURE = u64::MAX;
        RESULT_WIDE_CAMERA = ResultWideCameraState::empty();
        LAST_RESULT_WIDE_CAMERA_SIGNATURE = u64::MAX;
        reset_result_presentation_scene();
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][ResultCamera] scene_enter stage=0x{:x}",
                smash::app::stage::get_stage_id()
            );
        }
    }

    RESULT_SCENE_TICK = RESULT_SCENE_TICK.saturating_add(1);
    let participants = result_participants(fighter_manager);
    observe_result_reference(module_accessor, entry, participants, None);
    apply_result_presentation(module_accessor, entry);
    let Some(primary_winner_entry) = participants.primary() else {
        return;
    };

    log_result_winner_resolution(participants, primary_winner_entry);
    apply_result_wide_camera(module_accessor, entry, primary_winner_entry);
}

#[cfg(test)]
mod tests {
    use super::{
        active_result_pipeline_stage_name, camera_type_as_i32, custom_result_pipeline_enabled,
        next_result_identity, result_idle_motion_for_profile, result_presentation_allowed,
        result_profile_for_ui_hash, ResultIdentity, CUSTOM_RESULT_AUDIO_ENABLED,
        DEFAULT_GIGA_BOWSER_RESULT_CAMERA_TYPE, RESULT_ITEM_CREATION_ENABLED,
        RESULT_PRESENTATION_SCALE,
    };

    #[test]
    fn centralized_result_pipeline_keeps_camera_and_presentation() {
        assert_eq!(active_result_pipeline_stage_name(), "D_camera");
        assert!(custom_result_pipeline_enabled());
        assert!(RESULT_ITEM_CREATION_ENABLED);
        assert!(!CUSTOM_RESULT_AUDIO_ENABLED);
        assert_eq!(DEFAULT_GIGA_BOWSER_RESULT_CAMERA_TYPE, 0);
        assert_eq!(RESULT_PRESENTATION_SCALE, 0.4);
        assert_eq!(result_idle_motion_for_profile("rathalos"), "hovering_move");
        assert_eq!(result_idle_motion_for_profile("master_hand"), "wait");
        assert_eq!(result_idle_motion_for_profile("marx"), "wait");
    }

    #[test]
    fn result_item_creation_skips_galeem_dharkon_and_giga_bowser() {
        assert!(!result_presentation_allowed("galeem"));
        assert!(!result_presentation_allowed("dharkon"));
        assert!(!result_presentation_allowed("giga_bowser"));
        assert!(result_presentation_allowed("master_hand"));
        assert!(result_presentation_allowed("crazy_hand"));
        assert!(result_presentation_allowed("wol_master_hand"));
        assert!(result_presentation_allowed("dracula"));
        assert!(result_presentation_allowed("ganon_boss"));
        assert!(result_presentation_allowed("galleom"));
        assert!(result_presentation_allowed("rathalos"));
        assert!(result_presentation_allowed("marx"));
    }

    #[test]
    fn battle_identity_snapshot_survives_until_result_mode() {
        let mut identity = ResultIdentity::empty();
        identity.logical_ui_hash = crate::to_hash40("ui_chara_masterhand").0;

        // Result mode no longer has a safe battle host to inspect. The
        // snapshot must therefore remain sufficient to resolve the boss.
        assert_eq!(
            result_profile_for_ui_hash(identity.logical_ui_hash).map(|profile| profile.key),
            Some("master_hand")
        );
        let kept = next_result_identity(identity, true, 0);
        assert_eq!(kept.logical_ui_hash, identity.logical_ui_hash);
    }

    #[test]
    fn vanilla_mario_drops_stale_marx_result_identity() {
        let mut identity = ResultIdentity::empty();
        identity.logical_ui_hash = crate::to_hash40("ui_chara_marx").0;
        let persist_marx = crate::to_hash40("ui_chara_marx").0;
        let next = next_result_identity(identity, false, persist_marx);
        assert_eq!(next.logical_ui_hash, 0);
        assert!(result_profile_for_ui_hash(next.logical_ui_hash).is_none());
    }

    #[test]
    fn hidden_host_replaces_stale_marx_with_current_boss() {
        let mut identity = ResultIdentity::empty();
        identity.logical_ui_hash = crate::to_hash40("ui_chara_marx").0;
        let wol = crate::to_hash40("ui_chara_mewtwo_masterhand").0;
        let next = next_result_identity(identity, true, wol);
        assert_eq!(
            result_profile_for_ui_hash(next.logical_ui_hash).map(|profile| profile.key),
            Some("wol_master_hand")
        );
    }

    #[test]
    fn camera_only_stage_rejects_unrepresentable_native_camera_types() {
        assert_eq!(camera_type_as_i32(0x0f), Some(0x0f));
        assert_eq!(camera_type_as_i32(i32::MAX as u64), Some(i32::MAX));
        assert_eq!(camera_type_as_i32(i32::MAX as u64 + 1), None);
    }
}

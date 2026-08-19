//! Presentation data and the isolated Figure Player viewer handoff.
//!
//! The native amiibo viewer resolves item-backed bosses to Mario because its
//! normal actor factory only knows fighter kinds.  The viewer still creates a
//! valid Mario host, however.  This module keeps the logical boss identity
//! through that conversion and uses the host only to own one presentation-only
//! item.  It deliberately never enters a battle boss frame, recovery path, or
//! AI path while the amiibo viewer is active.

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::BTreeSet;
use std::sync::Once;

use crate::config::CONFIG;
use smash::app::lua_bind::{
    AttackModule, ControlModule, DamageModule, HitModule, ItemModule, JostleModule, ModelModule,
    MotionModule, PostureModule, SoundModule, StatusModule, VisibilityModule,
};
use smash::app::{sv_battle_object, BattleObjectModuleAccessor, ItemKind};
use smash::lib::lua_const::*;
use smash::phx::{Hash40, Vector3f};

const MAX_NRO_TRACE_ENTRIES: usize = 96;
const MAX_NRO_SYMBOL_TRACE_ENTRIES: usize = 48;
const MAX_IDENTITY_TRACE_ENTRIES: usize = 32;
// Item acquisition can finish native initialization after the first viewer
// frame. Reapply only during this short window, then leave the stable item
// transform alone instead of fighting the native viewer every frame.
const PREVIEW_TRANSFORM_STABILIZATION_FRAMES: u8 = 16;
// Master Hand and Marx are the hardware comparison pair for the current
// stage-0x135 transform-composition diagnosis. Capture only a few stabilized
// samples so this remains useful without becoming a per-frame log.
const TRANSFORM_COMPARISON_SAMPLE_FRAMES: u8 = 4;
// Motion-playback probe window. Long enough that a looping idle must advance
// its motion frame at least once if anything is playing at all.
const MOTION_PROBE_SAMPLE_FRAMES: u8 = 30;
// Switch system tick rate. `nn::os::GetSystemTick` is a named, version-stable
// symbol in the pinned bindings; the pinned bindings expose no
// GetSystemTickFrequency, so the documented 19.2 MHz platform rate is used for
// the millisecond conversion. Raw tick deltas are logged alongside it so the
// measurement stays usable even if the divisor is ever wrong.
const SYSTEM_TICK_HZ: f64 = 19_200_000.0;

#[inline(always)]
fn system_tick() -> u64 {
    unsafe { skyline::nn::os::GetSystemTick() }
}

#[inline(always)]
fn ticks_to_ms(ticks: u64) -> f64 {
    ticks as f64 * 1000.0 / SYSTEM_TICK_HZ
}
// Proof-pair (Master Hand + Marx) interactive-transform diagnostics: once
// stable maintenance begins (host fully native), capture a bounded number of
// native host-posture changes so hardware can prove stick rotation while
// plugin_host_posture_write=false and the boss-local correction stays put.
const INTERACTIVE_TRANSFORM_CHANGE_SAMPLES: u8 = 4;
// Debug-only stage-0x135 transform calibration harness, available on every
// item preview. All state is runtime-only and resets per viewer generation.
const CALIBRATION_INPUT_PROBE_SAMPLES: u8 = 24;
const CALIBRATION_RESET_HOLD_FRAMES: u16 = 60;
const CALIBRATION_COARSE_STEP_DEGREES: f32 = 15.0;
const CALIBRATION_FINE_STEP_DEGREES: f32 = 5.0;
const MASTER_HAND_PREVIEW_KEY: &str = "master_hand";
const MARX_PREVIEW_KEY: &str = "marx";
const RATHALOS_PREVIEW_KEY: &str = "rathalos";
// Rathalos's boss-only backing returns safely from `have_item` but hardware
// proved the object never lands in a host slot: after two WOL-faithful
// requests and both full observation windows the slots stay
// `[0,0,0,0]`/`[-1,-1,-1,-1]`. These bounds are the proven ceiling and must
// not be raised — a third request has nothing new to observe.
const RATHALOS_HOST_SETTLE_FRAMES: u8 = PREVIEW_TRANSFORM_STABILIZATION_FRAMES;
const RATHALOS_ACQUIRE_SETTLE_FRAMES: u8 = PREVIEW_TRANSFORM_STABILIZATION_FRAMES;
const RATHALOS_MAX_ACQUIRE_REQUESTS: u8 = 2;
// One menu-only visual fallback request after the boss backing exhausts, then
// fail closed. Presentation-only: it changes no identity and no other path.
const RATHALOS_FALLBACK_SETTLE_FRAMES: u8 = PREVIEW_TRANSFORM_STABILIZATION_FRAMES;
const DRACULA_PREVIEW_KEY: &str = "dracula";

// The NRO hook is a safe observation boundary for the first Switch trace. It
// tells us which lazily loaded UI module owns the amiibo screen without
// guessing a version-specific function address or touching a menu object.
static NRO_TRACE_INSTALLED: Once = Once::new();
static NRO_TRACE_SEEN: Lazy<Mutex<BTreeSet<String>>> = Lazy::new(|| Mutex::new(BTreeSet::new()));
static NRO_SYMBOL_TRACE_COUNT: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
static IDENTITY_TRACE_SEEN: Lazy<Mutex<BTreeSet<String>>> =
    Lazy::new(|| Mutex::new(BTreeSet::new()));

const MAX_CONFIGURED_PREVIEW_IDENTITIES: usize = 11;

// The normal character-database enumeration passes tagged Hash40 values with
// a `0xc100..` prefix. Some Master Hand scans also reach a direct `0xc1ffff..`
// lookup immediately before the stage-0x135 Mario host appears. Hardware has
// not proven that this form identifies every Figure Player, so it is retained
// only as a provisional candidate while the real NFP database key is fixed.
const DIRECT_UI_LOOKUP_TAG_MASK: u64 = 0xFFFF_FF00_0000_0000;
const DIRECT_UI_LOOKUP_TAG: u64 = 0xC1FF_FF00_0000_0000;

#[derive(Copy, Clone, PartialEq, Eq)]
enum AmiiboPreviewIdentitySource {
    None,
    ProvisionalDirectUiLookup,
}

impl AmiiboPreviewIdentitySource {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ProvisionalDirectUiLookup => "provisional_direct_ui_lookup_candidate",
        }
    }
}

#[derive(Copy, Clone)]
enum VerifiedPreviewBacking {
    Item { kind: i32, source: &'static str },
    NativeFighter { kind: i32, source: &'static str },
}

impl VerifiedPreviewBacking {
    const fn source(self) -> &'static str {
        match self {
            Self::Item { source, .. } | Self::NativeFighter { source, .. } => source,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum AmiiboPreviewPhase {
    Inactive,
    IdentityCaptured,
    WaitingForViewerHost,
    CreatingPresentation,
    AwaitingRathalosAcquire,
    Ready,
    DeferredUntilSupported,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum AmiiboPreviewOwnership {
    None,
    HostSlot,
    DetachedNativeOwned,
}

impl AmiiboPreviewOwnership {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::HostSlot => "host_slot",
            Self::DetachedNativeOwned => "detached_native_owned",
        }
    }
}

impl AmiiboPreviewPhase {
    const fn name(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::IdentityCaptured => "identity_captured",
            Self::WaitingForViewerHost => "waiting_for_viewer_host",
            Self::CreatingPresentation => "creating_presentation",
            Self::AwaitingRathalosAcquire => "awaiting_rathalos_acquire",
            Self::Ready => "ready",
            Self::DeferredUntilSupported => "deferred_until_supported",
        }
    }
}

/// The native viewer camera is established before the Mario host is hidden.
/// Keep that transform immutable for the rest of this viewer generation: the
/// WOL host recipe can legitimately move Mario after its motion/root changes.
#[derive(Copy, Clone)]
struct ViewerAnchor {
    initial_position: [f32; 3],
    initial_rotation: [f32; 3],
    position: [f32; 3],
    lr: f32,
    rotation: [f32; 3],
    initialized: bool,
}

impl ViewerAnchor {
    const fn empty() -> Self {
        Self {
            initial_position: [0.0; 3],
            initial_rotation: [0.0; 3],
            position: [0.0; 3],
            lr: 1.0,
            rotation: [0.0; 3],
            initialized: false,
        }
    }
}

/// Rathalos-only acquisition state. It bounds the boss-backing requests and
/// observation windows, then the single menu-only visual fallback request.
/// Nothing else shares this state.
#[derive(Copy, Clone)]
struct RathalosAcquireProbe {
    request_count: u8,
    host_settle_frames: u8,
    frames_since_request: u8,
    fallback_requested: bool,
    frames_since_fallback: u8,
}

impl RathalosAcquireProbe {
    const fn empty() -> Self {
        Self {
            request_count: 0,
            host_settle_frames: 0,
            frames_since_request: 0,
            fallback_requested: false,
            frames_since_fallback: 0,
        }
    }

    const fn host_settled(self) -> bool {
        self.host_settle_frames >= RATHALOS_HOST_SETTLE_FRAMES
    }

    const fn can_retry(self) -> bool {
        self.request_count < RATHALOS_MAX_ACQUIRE_REQUESTS
    }

    const fn observation_window_elapsed(self) -> bool {
        self.frames_since_request >= RATHALOS_ACQUIRE_SETTLE_FRAMES
    }

    const fn fallback_window_elapsed(self) -> bool {
        self.frames_since_fallback >= RATHALOS_FALLBACK_SETTLE_FRAMES
    }
}

/// Master Hand interactive-transform probe: after the bounded stabilization
/// window the host posture channel is entirely native. Log that state once,
/// then a bounded number of native host-posture changes (stick rotation).
#[derive(Copy, Clone)]
struct InteractiveTransformProbe {
    stable_phase_entered: bool,
    stable_state_logged: bool,
    change_samples_remaining: u8,
    last_host_posture: [f32; 3],
    has_last_host_posture: bool,
}

impl InteractiveTransformProbe {
    const fn empty() -> Self {
        Self {
            stable_phase_entered: false,
            stable_state_logged: false,
            change_samples_remaining: 0,
            last_host_posture: [0.0; 3],
            has_last_host_posture: false,
        }
    }
}

/// Which plugin-owned static channel the debug calibration harness is
/// currently dialing. The native host PostureModule is intentionally NOT a
/// target: it is Nintendo's interactive turntable.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CalibrationTarget {
    ItemRoot,
    ItemPosture,
    HostRoot,
}

impl CalibrationTarget {
    const COUNT: usize = 3;

    const fn index(self) -> usize {
        match self {
            Self::ItemRoot => 0,
            Self::ItemPosture => 1,
            Self::HostRoot => 2,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::ItemRoot => "item_root",
            Self::ItemPosture => "item_posture",
            Self::HostRoot => "host_root",
        }
    }

    const fn next(self) -> Self {
        match self {
            Self::ItemRoot => Self::ItemPosture,
            Self::ItemPosture => Self::HostRoot,
            Self::HostRoot => Self::ItemRoot,
        }
    }
}

/// Runtime-only calibration state for the debug harness. Never persisted,
/// reset on every viewer generation change. Overrides are indexed by
/// `CalibrationTarget::index` and, while `Some`, supersede that channel's
/// configured value during stable Ready maintenance.
#[derive(Copy, Clone)]
struct TransformCalibrationState {
    target: CalibrationTarget,
    axis: usize,
    overrides: [Option<[f32; 3]>; CalibrationTarget::COUNT],
    chord_engaged: bool,
    attack_hold_frames: u16,
    attack_reset_consumed: bool,
    input_probe_samples_remaining: u8,
    last_input_bits: i32,
    has_last_input_bits: bool,
}

impl TransformCalibrationState {
    const fn empty() -> Self {
        Self {
            target: CalibrationTarget::ItemRoot,
            axis: 0,
            overrides: [None; CalibrationTarget::COUNT],
            chord_engaged: false,
            attack_hold_frames: 0,
            attack_reset_consumed: false,
            input_probe_samples_remaining: CALIBRATION_INPUT_PROBE_SAMPLES,
            last_input_bits: 0,
            has_last_input_bits: false,
        }
    }

    fn override_for(&self, target: CalibrationTarget) -> Option<[f32; 3]> {
        self.overrides[target.index()]
    }
}

const CALIBRATION_AXIS_NAMES: [&str; 3] = ["x", "y", "z"];

#[inline(always)]
fn wrap_calibration_degrees(value: f32) -> f32 {
    ((value + 180.0).rem_euclid(360.0)) - 180.0
}

#[derive(Copy, Clone)]
struct AmiiboPreviewState {
    phase: AmiiboPreviewPhase,
    logical_ui_hash: u64,
    identity_source: AmiiboPreviewIdentitySource,
    host_object_id: u32,
    presentation_object_id: u32,
    expected_item_kind: i32,
    // False only while a Rathalos menu-visual-fallback object is presented.
    // The profile's status/motion pairing is proven for its own backing only,
    // so a fallback object keeps its native status and motion instead.
    apply_profile_idle_motion: bool,
    presentation_slot: i32,
    ownership: AmiiboPreviewOwnership,
    viewer_generation: u32,
    viewer_anchor: ViewerAnchor,
    host_hidden: bool,
    create_attempted: bool,
    stabilization_reacquire_used: bool,
    transform_stabilization_frames_remaining: u8,
    rathalos_acquire_probe: RathalosAcquireProbe,
    interactive_transform_probe: InteractiveTransformProbe,
    transform_calibration: TransformCalibrationState,
    // Motion-playback probe. Distinguishes "the configured motion name does
    // not exist on this item" from "this model simply has no animation":
    // motion_kind tells us whether the request took, MotionModule::frame tells
    // us whether anything is actually playing.
    motion_probe_samples_remaining: u8,
    motion_probe_motion_before: u64,
    motion_probe_motion_after_change: u64,
    motion_probe_first_frame: f32,
    motion_probe_last_frame: f32,
    motion_probe_advanced: bool,
    // Frames from viewer_host_ready to the first fully visible frame, so the
    // perceived spawn delay can be attributed to the plugin or to the game's
    // own NRO/model streaming.
    identity_tick: u64,
    viewer_host_ready_tick: u64,
    frames_since_viewer_host_ready: u16,
    frames_host_ready_to_create: u16,
    frames_create_to_engine_visible: u16,
    frames_create_to_model_visible: u16,
    have_item_ticks: u64,
    created_frame_mark: u16,
    visible_latency_logged: bool,
    transform_comparison_samples_remaining: u8,
    transform_comparison_complete: bool,
    transform_ready_logged: bool,
    native_transform_reset_logged: bool,
    ready_visual_logged: bool,
    visual_ready_blocked_logged: bool,
    visibility_reassertion_used: bool,
    last_item_visible: Option<bool>,
    last_item_model_visible: Option<bool>,
    ignored_lookup_mask: u16,
}

impl AmiiboPreviewState {
    const fn new() -> Self {
        Self {
            phase: AmiiboPreviewPhase::Inactive,
            logical_ui_hash: 0,
            identity_source: AmiiboPreviewIdentitySource::None,
            host_object_id: 0,
            presentation_object_id: 0,
            expected_item_kind: -1,
            apply_profile_idle_motion: true,
            presentation_slot: -1,
            ownership: AmiiboPreviewOwnership::None,
            viewer_generation: 0,
            viewer_anchor: ViewerAnchor::empty(),
            host_hidden: false,
            create_attempted: false,
            stabilization_reacquire_used: false,
            transform_stabilization_frames_remaining: 0,
            rathalos_acquire_probe: RathalosAcquireProbe::empty(),
            interactive_transform_probe: InteractiveTransformProbe::empty(),
            transform_calibration: TransformCalibrationState::empty(),
            motion_probe_samples_remaining: MOTION_PROBE_SAMPLE_FRAMES,
            motion_probe_motion_before: 0,
            motion_probe_motion_after_change: 0,
            motion_probe_first_frame: -1.0,
            motion_probe_last_frame: -1.0,
            motion_probe_advanced: false,
            identity_tick: 0,
            viewer_host_ready_tick: 0,
            frames_since_viewer_host_ready: 0,
            frames_host_ready_to_create: 0,
            frames_create_to_engine_visible: 0,
            frames_create_to_model_visible: 0,
            have_item_ticks: 0,
            created_frame_mark: 0,
            visible_latency_logged: false,
            transform_comparison_samples_remaining: 0,
            transform_comparison_complete: false,
            transform_ready_logged: false,
            native_transform_reset_logged: false,
            ready_visual_logged: false,
            visual_ready_blocked_logged: false,
            visibility_reassertion_used: false,
            last_item_visible: None,
            last_item_model_visible: None,
            ignored_lookup_mask: 0,
        }
    }
}

static mut CONFIGURED_PREVIEW_HASHES: [u64; MAX_CONFIGURED_PREVIEW_IDENTITIES] =
    [0; MAX_CONFIGURED_PREVIEW_IDENTITIES];
static mut CONFIGURED_PREVIEW_HASH_COUNT: usize = 0;
static mut AMIIBO_PREVIEW_STATE: AmiiboPreviewState = AmiiboPreviewState::new();

// The project already uses this named item API to retire a verified detached
// boss item. The viewer uses it only for its own active, kind-checked object.
extern "C" {
    #[link_name = "\u{1}_ZN3app10item_other6removeEPNS_26BattleObjectModuleAccessorE"]
    fn remove_detached_presentation_item(module_accessor: *mut BattleObjectModuleAccessor);
}

#[inline(always)]
unsafe fn preview_state_ptr() -> *mut AmiiboPreviewState {
    core::ptr::addr_of_mut!(AMIIBO_PREVIEW_STATE)
}

fn is_ui_like_nro(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "common" || lower.contains("ui") || lower.contains("menu") || lower.contains("chara")
}

fn is_preview_symbol(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "amiibo", "nfp", "figure", "preview", "viewer", "model", "motion", "camera",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn nro_trace_enabled() -> bool {
    crate::debug::enabled() && CONFIG.options.debug_amiibo_nro_trace.unwrap_or(false)
}

fn nro_symbol_trace_enabled() -> bool {
    nro_trace_enabled() && CONFIG.options.debug_amiibo_nro_symbols.unwrap_or(false)
}

unsafe fn read_symbol_name(pointer: *const u8, max_len: usize) -> Option<String> {
    if pointer.is_null() {
        return None;
    }

    let mut bytes = Vec::new();
    for index in 0..max_len {
        let byte = *pointer.add(index);
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }
    String::from_utf8(bytes).ok()
}

// This read-only symbol inventory is deliberately limited to likely UI NROs
// and bounded to exported names matching preview concepts. It is useful when
// a module is stripped of documented symbols, but it never hooks a discovered
// address. The final function hook still requires a version-stable boundary.
unsafe fn log_exported_preview_symbols(info: &skyline::nro::NroInfo, module_base: u64) {
    if !is_ui_like_nro(info.name) {
        return;
    }

    let module_object = info.module.ModuleObject;
    if module_object.is_null() {
        return;
    }

    let module_object = &*module_object;
    if module_object.dynsym.is_null()
        || module_object.dynstr.is_null()
        || module_object.hash_nchain_value == 0
        || module_object.hash_nchain_value > 0x10000
        || module_object.dynstr_size == 0
        || module_object.dynstr_size > 0x0100_0000
    {
        return;
    }

    for index in 0..module_object.hash_nchain_value as usize {
        let symbol = *module_object.dynsym.add(index);
        if symbol.st_name as u64 >= module_object.dynstr_size {
            continue;
        }

        let Some(name) = read_symbol_name(module_object.dynstr.add(symbol.st_name as usize), 160)
        else {
            continue;
        };
        if name.is_empty() || !is_preview_symbol(&name) {
            continue;
        }

        let mut logged = NRO_SYMBOL_TRACE_COUNT.lock();
        if *logged >= MAX_NRO_SYMBOL_TRACE_ENTRIES {
            return;
        }
        *logged += 1;
        drop(logged);

        crate::boss_log!(
            "[PB][AmiiboPreview][NRO][symbol] module={} name={} address=0x{:x}",
            info.name,
            name,
            module_base.wrapping_add(symbol.st_value)
        );
    }
}

fn log_nro_event(event: &str, info: &skyline::nro::NroInfo) {
    if !nro_trace_enabled() {
        return;
    }

    let key = format!("{}:{}", event, info.name);
    {
        let mut seen = NRO_TRACE_SEEN.lock();
        if seen.contains(&key) {
            return;
        }
        if seen.len() >= MAX_NRO_TRACE_ENTRIES {
            return;
        }
        seen.insert(key);
    }

    let module_base = unsafe {
        let module_object = info.module.ModuleObject;
        if module_object.is_null() {
            0
        } else {
            (*module_object).module_base
        }
    };

    crate::boss_log!(
        "[PB][AmiiboPreview][NRO] event={} name={} module_base=0x{:x}",
        event,
        info.name,
        module_base
    );

    if event == "load" && nro_symbol_trace_enabled() {
        unsafe { log_exported_preview_symbols(info, module_base) };
    }
}

extern "Rust" fn log_nro_load(info: &skyline::nro::NroInfo) {
    log_nro_event("load", info);
}

extern "Rust" fn log_nro_unload(info: &skyline::nro::NroInfo) {
    log_nro_event("unload", info);
}

/// Install only the version-independent NRO lifecycle trace needed to find
/// the native amiibo viewer module on hardware. This does not hook a viewer
/// function and does not create or mutate any preview object.
pub fn install_nro_trace() {
    if !nro_trace_enabled() {
        return;
    }

    NRO_TRACE_INSTALLED.call_once(|| {
        let load_result = skyline::nro::add_hook(log_nro_load);
        let unload_result = skyline::nro::add_unload_hook(log_nro_unload);
        crate::boss_log!(
            "[PB][AmiiboPreview] nro_trace installed load={} unload={}",
            load_result.is_ok(),
            unload_result.is_ok()
        );
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossAmiiboPreviewKind {
    ItemPresentation,
    NativeFighterPresentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemPresentationAcquireRecipe {
    Direct,
    // Rathalos alone: `have_item(ITEM_KIND_LIOLEUSBOSS)` returns safely but
    // hardware proved the object never reaches a host slot. After that bounded
    // primary exhausts, one menu-only visual fallback request for
    // ITEM_KIND_LIOLEUS is made, accepted only on an exact kind match. This
    // shares no state or assumption with the Dracula block.
    RathalosBossThenVisualFallback,
}

impl ItemPresentationAcquireRecipe {
    const fn name(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::RathalosBossThenVisualFallback => "rathalos_boss_then_visual_fallback",
        }
    }

    const fn requires_empty_host_slots(self) -> bool {
        matches!(self, Self::RathalosBossThenVisualFallback)
    }

    const fn uses_tiny_host_before_request(self) -> bool {
        matches!(self, Self::RathalosBossThenVisualFallback)
    }

    const fn uses_deferred_observation(self) -> bool {
        matches!(self, Self::RathalosBossThenVisualFallback)
    }

    // The menu-only visual fallback belongs to Rathalos and nothing else.
    const fn uses_menu_visual_fallback(self) -> bool {
        matches!(self, Self::RathalosBossThenVisualFallback)
    }
}

// Stage 0x135 transform ownership, hardware-proven on Master Hand and Marx:
// - Host `PostureModule` belongs to the NATIVE Amiibo viewer. It carries the
//   interactive right-stick turntable; any plugin write during stable
//   maintenance freezes native rotation (Marx's proven failure mode).
// - Persistent host `root` joint writes are COMPATIBLE with native rotation
//   (Master Hand rotated with a maintained root), but the WOL host
//   posture/root composition does not reproduce the correct static
//   orientation through the stage-0x135 held-item chain — both Euler
//   re-encodings were hardware-disproven. The host root is therefore no
//   longer used as the boss static-orientation channel by any preview.
// - Boss-specific STATIC orientation is plugin-owned and must live on the
//   presentation item itself: item posture rotation (proven stable and
//   plugin-owned) and/or the item's own `root` joint (audited by the
//   [PB][AmiiboLocalTransform] diagnostic and the debug calibration harness).
// Every item-backed boss now uses this architecture: the WOL host-posture and
// host-root recipes composed wrongly through the stage-0x135 held-item chain
// and left every boss other than Master Hand and Marx lying on its side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmiiboHostOrientationRecipe {
    // Fully native Mario host: the plugin writes neither the host
    // PostureModule nor the host root joint. Boss-specific static orientation
    // must live on the presentation item itself (item posture and/or the
    // item's own root joint), never on the interactive host channels.
    //
    // Historical (hardware-disproven) approaches for Master Hand and Marx:
    // - Writing the WOL pair (host posture [-180,90,0] + host root [90,40,0])
    //   every Ready frame reproduced the WOL pose but froze the native
    //   right-stick turntable, which hardware proved lives in the host
    //   PostureModule (Marx's observed failure mode).
    // - Folding that pair into a host-root-only Euler was tried twice for
    //   Master Hand ([-90,50,0] fixed-axis Rz*Ry*Rx, then [-50,0,-90]
    //   intrinsic Rx*Ry*Rz). Both preserved interactivity but produced wrong
    //   static orientations: the WOL host posture/root composition does NOT
    //   map onto the stage-0x135 held-item visual chain, so no further Euler
    //   reconstruction of it may be attempted.
    NativeHost,
    NativeFighter,
}

impl AmiiboHostOrientationRecipe {
    const fn name(self) -> &'static str {
        match self {
            Self::NativeHost => "stage135_native_host",
            Self::NativeFighter => "native_fighter",
        }
    }

    // No plugin-owned host channel remains: the WOL posture/root pair was
    // hardware-disproven on stage 0x135 for the proof pair and is what kept
    // every other boss mis-oriented. Static correction is item-local only.
    const fn posture_rotation(self) -> Option<[f32; 3]> {
        None
    }

    const fn root_rotation(self) -> Option<[f32; 3]> {
        None
    }

    const fn posture_rotation_name(self) -> &'static str {
        match self {
            Self::NativeHost => "native",
            Self::NativeFighter => "not_applied",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BossAmiiboPreviewProfile {
    pub key: &'static str,
    pub ui_chara_id: &'static str,
    pub preview_kind: BossAmiiboPreviewKind,
    pub preview_source: &'static str,
    pub idle_motion: &'static str,
    pub preview_scale: Option<f32>,
    // Per-boss item-side orientation calibration for stage 0x135. It is
    // written only during creation plus the bounded stabilization window, and
    // it composes inside the host-relative chain: hardware proved the item's
    // own posture Euler stays constant while the boss visually follows the
    // native viewer's host rotation.
    presentation_rotation: Option<[f32; 3]>,
    // Plugin-owned static correction on the presentation item's own `root`
    // joint (`ModelModule::set_joint_rotate(item, "root", ..)`), maintained
    // every Ready frame while Some. This is the boss-local channel BELOW the
    // native host turntable. Intentionally None everywhere until the debug
    // calibration harness produces a hardware-proven value; no software-side
    // Euler guess may be committed here.
    item_root_rotation: Option<[f32; 3]>,
    host_orientation_recipe: AmiiboHostOrientationRecipe,
    item_acquire_recipe: ItemPresentationAcquireRecipe,
    pub position_offset: [f32; 3],
}

// Item/idle/scale values mirror the working WOL/boss-preview implementations.
// Stage 0x135 retains the native Mario viewer camera, so every item starts at
// the Mario host's exact position instead of using a separate camera space.
pub const BOSS_AMIIBO_PREVIEW_PROFILES: [BossAmiiboPreviewProfile; 11] = [
    BossAmiiboPreviewProfile {
        key: "master_hand",
        ui_chara_id: "ui_chara_masterhand",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:masterhand",
        idle_motion: "wait",
        preview_scale: Some(0.45),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        // Static upright correction is uncalibrated: both host-root Euler
        // re-encodings of the WOL pose were hardware-disproven, so the value
        // must come from the debug calibration harness, not another guess.
        item_root_rotation: None,
        // Hardware-proven: the native viewer owns host posture (right-stick
        // turntable) and the plugin owns nothing on the host.
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        // The viewer camera is already centered on Mario. Lift the item's
        // visual origin into that space without altering its native rotation.
        // The native viewer anchor is Y=0.010 in the observed Master Hand
        // scene; this produces the calibrated world-space target Y=0.180.
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "crazy_hand",
        ui_chara_id: "ui_chara_crazyhand",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:crazyhand",
        idle_motion: "wait",
        preview_scale: Some(0.45),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "wol_master_hand",
        ui_chara_id: "ui_chara_mewtwo_masterhand",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:masterhand_wol_preview",
        idle_motion: "wait",
        preview_scale: Some(0.45),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "galeem",
        ui_chara_id: "ui_chara_kiila",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:kiila",
        idle_motion: crate::galeem::PRESENTATION_IDLE_MOTION,
        preview_scale: Some(0.28125),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        // Was [0.36, 0.17, 0.0], the only offset that differed from the
        // working bosses. The native viewer camera frames the anchor plus
        // [0.36, 0.17, 0.0], so the old value put them out of shot.
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "dharkon",
        ui_chara_id: "ui_chara_darz",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:darz",
        idle_motion: crate::dharkon::PRESENTATION_IDLE_MOTION,
        preview_scale: Some(0.28125),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        // Was [0.36, 0.17, 0.0], the only offset that differed from the
        // working bosses. The native viewer camera frames the anchor plus
        // [0.36, 0.17, 0.0], so the old value put them out of shot.
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "dracula",
        ui_chara_id: "ui_chara_dracula",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        // Phase 1, the backing his working battle path acquires. Every
        // prepared or settled stage-0x135 test so far used phase 2.
        preview_source: "item:dracula",
        idle_motion: "wait",
        preview_scale: Some(0.45),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        // Exactly Master Hand's path: a plain `have_item` on the
        // viewer-host-ready frame, no slot clearing and no host shrink.
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "ganon_boss",
        ui_chara_id: "ui_chara_ganonboss",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:ganonboss",
        idle_motion: "wait",
        preview_scale: Some(0.365625),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "galleom",
        ui_chara_id: "ui_chara_galleom",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:galleom",
        idle_motion: "wait",
        preview_scale: Some(0.225),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "rathalos",
        ui_chara_id: "ui_chara_lioleus",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:lioleusboss",
        // Matches the WOL preview block in `src/rathalos/mod.rs`, which is the
        // presentation that renders correctly. (`hovering` belongs to the
        // separate item-wipe recovery path and is not the preview motion.)
        // Belongs to the LIOLEUSBOSS path only; the menu visual fallback
        // object keeps its own native motion instead.
        idle_motion: "hovering_move",
        preview_scale: Some(0.225),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::RathalosBossThenVisualFallback,
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "marx",
        ui_chara_id: "ui_chara_marx",
        preview_kind: BossAmiiboPreviewKind::ItemPresentation,
        preview_source: "item:marx",
        // Marx must rotate with the right stick like every other Amiibo
        // preview boss. Hardware proved the previous WOL posture recipe
        // (plugin writing host posture every Ready frame) froze the native
        // turntable, so the host is now fully native and Marx's static
        // orientation is boss-local. The old "static item" behavior was a
        // defect, not a requirement.
        idle_motion: "wait",
        preview_scale: Some(0.28125),
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        // Uncalibrated: hardware harness output required, same as Master
        // Hand. Do not copy Master Hand's value or re-derive from WOL math.
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeHost,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        position_offset: [0.36, 0.17, 0.0],
    },
    BossAmiiboPreviewProfile {
        key: "giga_bowser",
        ui_chara_id: "ui_chara_koopag",
        preview_kind: BossAmiiboPreviewKind::NativeFighterPresentation,
        preview_source: "fighter:koopag",
        idle_motion: "wait",
        preview_scale: None,
        presentation_rotation: Some([0.0, 0.0, -90.0]),
        item_root_rotation: None,
        host_orientation_recipe: AmiiboHostOrientationRecipe::NativeFighter,
        item_acquire_recipe: ItemPresentationAcquireRecipe::Direct,
        position_offset: [0.0, 0.0, 0.0],
    },
];

pub fn profiles() -> &'static [BossAmiiboPreviewProfile; 11] {
    &BOSS_AMIIBO_PREVIEW_PROFILES
}

pub fn profile_for_ui_chara_id(ui_chara_id: &str) -> Option<&'static BossAmiiboPreviewProfile> {
    profiles()
        .iter()
        .find(|profile| profile.ui_chara_id == ui_chara_id)
}

#[inline(always)]
fn profile_for_ui_chara_hash(ui_chara_hash: u64) -> Option<&'static BossAmiiboPreviewProfile> {
    profiles()
        .iter()
        .find(|profile| crate::to_hash40(profile.ui_chara_id).0 == ui_chara_hash)
}

#[inline(always)]
unsafe fn configured_for_preview(ui_chara_hash: u64) -> bool {
    let count = CONFIGURED_PREVIEW_HASH_COUNT.min(MAX_CONFIGURED_PREVIEW_IDENTITIES);
    let hashes = core::ptr::addr_of_mut!(CONFIGURED_PREVIEW_HASHES).cast::<u64>();
    (0..count).any(|index| hashes.add(index).read() == ui_chara_hash)
}

#[inline(always)]
fn is_direct_amiibo_identity_lookup(raw_ui_hash: u64) -> bool {
    raw_ui_hash & DIRECT_UI_LOOKUP_TAG_MASK == DIRECT_UI_LOOKUP_TAG
}

#[inline(always)]
fn profile_rollout_enabled(profile: &BossAmiiboPreviewProfile) -> bool {
    // Every item backing below is already used by the corresponding WOL
    // preview. Giga Bowser is excluded because it needs no plugin
    // presentation at all: it is a real fighter, so once `ui_chara_koopag`
    // carries `fighter_kind_koopag` the native viewer builds it directly.
    matches!(
        profile.preview_kind,
        BossAmiiboPreviewKind::ItemPresentation
    )
}

/// Keep presentation backing independent from the hidden Mario battle host.
/// Every item kind below is already used by the corresponding battle/WOL path;
/// this table merely chooses which existing model is safe to request from the
/// menu-owned host. Giga Bowser deliberately remains a native-fighter route.
#[inline(always)]
unsafe fn verified_presentation_backing(
    profile: &BossAmiiboPreviewProfile,
) -> Option<VerifiedPreviewBacking> {
    match profile.key {
        "master_hand" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_MASTERHAND,
            source: "ITEM_KIND_MASTERHAND",
        }),
        "crazy_hand" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_CRAZYHAND,
            source: "ITEM_KIND_CRAZYHAND",
        }),
        "wol_master_hand" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_MASTERHAND,
            source: "ITEM_KIND_MASTERHAND (WOL preview)",
        }),
        // These are the presentation backings used by the current WOL paths,
        // not the regular-smash parent items with summon lifecycles.
        // Presentation-only backing change. Hardware proved `"wait"` is
        // accepted and advancing on the CORE objects (motion_rate=1,
        // frame 0 -> 30, animation_advanced=true) yet the complete animated
        // boss never appeared, so the core items are the wrong visual objects
        // rather than the motion being wrong. These are the full boss objects.
        // Logical Amiibo identity (`ui_chara_kiila` / `ui_chara_darz`) is
        // unchanged, and no boss status is forced — see
        // `[PB][AmiiboBossBackingProbe]` for the native initial state.
        "galeem" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_KIILA,
            source: "ITEM_KIND_KIILA",
        }),
        "dharkon" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_DARZ,
            source: "ITEM_KIND_DARZ",
        }),
        "dracula" => Some(VerifiedPreviewBacking::Item {
            // Phase 1, matching `src/dracula/mod.rs`'s battle acquisition —
            // the Dracula backing this plugin actually spawns successfully.
            // Phase 2 (DRACULA2) is his transformed form and is what every
            // crashed prepared/settled stage-0x135 attempt requested.
            kind: *ITEM_KIND_DRACULA,
            source: "ITEM_KIND_DRACULA (battle-proven phase 1)",
        }),
        "ganon_boss" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_GANONBOSS,
            source: "ITEM_KIND_GANONBOSS",
        }),
        "galleom" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_GALLEOM,
            source: "ITEM_KIND_GALLEOM",
        }),
        "rathalos" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_LIOLEUSBOSS,
            source: "ITEM_KIND_LIOLEUSBOSS",
        }),
        "marx" => Some(VerifiedPreviewBacking::Item {
            kind: *ITEM_KIND_MARX,
            source: "ITEM_KIND_MARX",
        }),
        "giga_bowser" => Some(VerifiedPreviewBacking::NativeFighter {
            kind: *FIGHTER_KIND_KOOPAG,
            source: "FIGHTER_KIND_KOOPAG",
        }),
        _ => None,
    }
}

/// Configure the runtime identity allowlist independently of diagnostics.
/// Preview behavior must not depend on `DEBUG_BOSS_LOGS` being enabled.
pub fn configure_mapping_profiles(mappings: &[crate::amiibo::ConfiguredBossAmiibo]) {
    unsafe {
        CONFIGURED_PREVIEW_HASHES = [0; MAX_CONFIGURED_PREVIEW_IDENTITIES];
        CONFIGURED_PREVIEW_HASH_COUNT = 0;
        for mapping in mappings.iter().take(MAX_CONFIGURED_PREVIEW_IDENTITIES) {
            let index = CONFIGURED_PREVIEW_HASH_COUNT;
            let ui_chara_hash = crate::to_hash40(mapping.identity.ui_chara_id).0;
            core::ptr::addr_of_mut!(CONFIGURED_PREVIEW_HASHES)
                .cast::<u64>()
                .add(index)
                .write(ui_chara_hash);
            CONFIGURED_PREVIEW_HASH_COUNT += 1;
        }
    }
}

/// Truthful startup status for a configured mapping. Dracula is reported as
/// permanently blocked: every known `have_item` path for both of his backings
/// crashes on stage 0x135, so the preview never attempts acquisition and there
/// is no configuration option that can re-arm it.
fn mapping_runtime_status(
    profile: &BossAmiiboPreviewProfile,
    backing: Option<VerifiedPreviewBacking>,
) -> &'static str {
    if profile.key == DRACULA_PREVIEW_KEY {
        return "acquisition_blocked_all_known_have_item_paths_crash_fail_closed";
    }
    match backing {
        Some(VerifiedPreviewBacking::Item { .. }) if profile_rollout_enabled(profile) => {
            "stage_135_runtime_enabled"
        }
        Some(VerifiedPreviewBacking::Item { .. }) => {
            "typed_item_backing_hardware_verification_required"
        }
        Some(VerifiedPreviewBacking::NativeFighter { .. }) => {
            "native_fighter_viewer_owned_no_plugin_presentation"
        }
        None => "verified_presentation_backing_missing",
    }
}

pub fn log_mapping_profiles(mappings: &[crate::amiibo::ConfiguredBossAmiibo]) {
    crate::boss_log!(
        "[PB][AmiiboPreview] presentation_handoff=stage_135_host_owned_item configured_mappings={} identity_source=provisional_direct_ui_lookup_candidate typed_item_backings=enabled_wol_recipe_projection native_fighter_backing=deferred",
        mappings.len()
    );

    for mapping in mappings {
        let Some(profile) = profile_for_ui_chara_id(mapping.identity.ui_chara_id) else {
            crate::boss_log!(
                "[PB][AmiiboPreview] mapping_ready boss={} ui_chara_id={} profile=missing status=blocked",
                mapping.identity.name,
                mapping.identity.ui_chara_id
            );
            continue;
        };

        if profile.key != mapping.identity.key {
            crate::boss_log!(
                "[PB][AmiiboPreview] mapping_ready boss={} ui_chara_id={} profile_key_mismatch={} expected={} status=blocked",
                mapping.identity.name,
                profile.ui_chara_id,
                profile.key,
                mapping.identity.key
            );
            continue;
        }

        let backing = unsafe { verified_presentation_backing(profile) };
        let runtime_status = mapping_runtime_status(profile, backing);
        crate::boss_log!(
            "[PB][AmiiboPreview] mapping_ready boss={} ui_chara_id={} kind={:?} source={} verified_backing={} idle_motion={} scale={:?} anchor=host_relative anchor_offset=({:.3},{:.3},{:.3}) host_recipe={} presentation_rotation={:?} camera=native_viewer_host status={} custom_preview_required=true",
            mapping.identity.name,
            profile.ui_chara_id,
            profile.preview_kind,
            profile.preview_source,
            backing.map(|value| value.source()).unwrap_or("<missing>"),
            profile.idle_motion,
            profile.preview_scale,
            profile.position_offset[0],
            profile.position_offset[1],
            profile.position_offset[2],
            profile.host_orientation_recipe.name(),
            profile.presentation_rotation,
            runtime_status
        );
    }
}

/// Log the last safe boundary owned by this plugin. The native amiibo menu
/// viewer runs after the ARC database lookup and is not exposed by the pinned
/// bindings, so this records what the plugin supplied without pretending that
/// a model/fighter conversion was observed.
pub fn log_identity_boundary(
    mapping: &crate::amiibo::ConfiguredBossAmiibo,
    mode: &str,
    original_ui_chara_hash: Option<u64>,
) {
    if !crate::debug::enabled() {
        return;
    }

    let key = format!("{}:{:016x}", mode, mapping.tag_id);
    {
        let mut seen = IDENTITY_TRACE_SEEN.lock();
        if seen.contains(&key) || seen.len() >= MAX_IDENTITY_TRACE_ENTRIES {
            return;
        }
        seen.insert(key);
    }

    let profile = profile_for_ui_chara_id(mapping.identity.ui_chara_id);
    crate::boss_log!(
        "[PB][AmiiboPreview] identity_boundary mode={} figure_id=0x{:016x} original_ui_chara_hash={} requested_ui_chara={} requested_ui_chara_hash=0x{:010x} logical_boss={} battle_backing={} profile_source={} idle_motion={} native_fighter_kind=unobserved native_model=unobserved preview_actor=unobserved preview_motion=unobserved preview_camera=unobserved",
        mode,
        mapping.tag_id,
        original_ui_chara_hash
            .map(|hash| format!("0x{:010x}", hash))
            .unwrap_or_else(|| "<none>".to_string()),
        mapping.identity.ui_chara_id,
        crate::to_hash40(mapping.identity.ui_chara_id).0,
        mapping.identity.name,
        mapping.identity.backing_fighter,
        profile.map(|value| value.preview_source).unwrap_or("<missing>"),
        profile.map(|value| value.idle_motion).unwrap_or("<missing>"),
    );
    crate::boss_log!(
        "[PB][AmiiboPreviewTrace] boundary=ui_amiibo_db mode={} figure_id=0x{:016x} requested_ui_chara={} next_boundary=menu_model_factory native_fighter_kind=unobserved native_model=unobserved preview_actor=unobserved preview_motion=unobserved preview_camera=unobserved viewer_ready=unobserved",
        mode,
        mapping.tag_id,
        mapping.identity.ui_chara_id,
    );
}

/// Record the last safe character-database boundary for the menu viewer.
/// `ui_chara_db` is the final data path this plugin owns; the pinned public
/// bindings expose no menu-model/resource callback after this conversion, so
/// the downstream fields remain explicitly unobserved rather than guessed.
pub fn log_ui_chara_db_boundary(
    ui_chara_id: &'static str,
    backing_fighter_kind: &'static str,
    presentation_source: &'static str,
) {
    if !crate::debug::enabled() {
        return;
    }

    let key = format!("ui_chara_db:{}", ui_chara_id);
    {
        let mut seen = IDENTITY_TRACE_SEEN.lock();
        if seen.contains(&key) || seen.len() >= MAX_IDENTITY_TRACE_ENTRIES {
            return;
        }
        seen.insert(key);
    }

    crate::boss_log!(
        "[PB][AmiiboPreviewTrace] boundary=ui_chara_db ui_chara_id={} ui_chara_hash=0x{:010x} viewer_fighter_kind={} battle_backing={} presentation_source={} next_boundary=menu_model_factory model_resource=unobserved motion_resource=unobserved preview_actor=unobserved viewer_ready=unobserved",
        ui_chara_id,
        crate::to_hash40(ui_chara_id).0,
        backing_fighter_kind,
        backing_fighter_kind,
        presentation_source
    );
}

#[inline(always)]
unsafe fn lock_preview_identity(ui_chara_hash: u64) {
    let source = AmiiboPreviewIdentitySource::ProvisionalDirectUiLookup;
    let Some(profile) = profile_for_ui_chara_hash(ui_chara_hash) else {
        return;
    };
    let state = &mut *preview_state_ptr();
    if state.logical_ui_hash == ui_chara_hash
        && state.identity_source == source
        && matches!(
            state.phase,
            AmiiboPreviewPhase::IdentityCaptured
                | AmiiboPreviewPhase::WaitingForViewerHost
                | AmiiboPreviewPhase::CreatingPresentation
                | AmiiboPreviewPhase::Ready
                | AmiiboPreviewPhase::DeferredUntilSupported
        )
    {
        return;
    }

    state.logical_ui_hash = ui_chara_hash;
    state.identity_source = source;
    state.identity_tick = system_tick();
    state.phase = AmiiboPreviewPhase::IdentityCaptured;
    state.create_attempted = false;
    state.stabilization_reacquire_used = false;
    state.transform_stabilization_frames_remaining = 0;
    state.apply_profile_idle_motion = true;
    state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
    state.motion_probe_samples_remaining = MOTION_PROBE_SAMPLE_FRAMES;
    state.motion_probe_motion_before = 0;
    state.motion_probe_motion_after_change = 0;
    state.motion_probe_first_frame = -1.0;
    state.motion_probe_last_frame = -1.0;
    state.motion_probe_advanced = false;
    state.frames_since_viewer_host_ready = 0;
    state.frames_host_ready_to_create = 0;
    state.frames_create_to_engine_visible = 0;
    state.frames_create_to_model_visible = 0;
    state.have_item_ticks = 0;
    state.created_frame_mark = 0;
    state.visible_latency_logged = false;
    state.transform_comparison_samples_remaining = 0;
    state.transform_comparison_complete = false;
    state.interactive_transform_probe = InteractiveTransformProbe::empty();
    state.transform_calibration = TransformCalibrationState::empty();
    state.viewer_anchor = ViewerAnchor::empty();
    state.transform_ready_logged = false;
    state.native_transform_reset_logged = false;
    state.ready_visual_logged = false;
    state.visual_ready_blocked_logged = false;
    state.visibility_reassertion_used = false;
    state.last_item_visible = None;
    state.last_item_model_visible = None;
    state.ignored_lookup_mask = 0;
    if crate::debug::enabled() {
        // The direct UI lookup proves logical identity only. Do not emit
        // configured values as though this raw UI callback exposed NFP tag
        // metadata.
        crate::boss_log!(
            "[PB][AmiiboScan] figure_id=unobserved ui_amiibo_id=unobserved upper=unobserved boss={} source={}",
            profile.key,
            source.name(),
        );
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] identity_candidate generation={} logical_boss={} ui_chara_hash=0x{:010x} source={} authoritative=false presentation_source={} stage=unobserved_ui_hook",
            state.viewer_generation,
            profile.key,
            ui_chara_hash,
            source.name(),
            profile.preview_source
        );
    }
}

#[inline(always)]
unsafe fn log_ignored_lookup(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    reason: &str,
) {
    let Some(profile_index) = profiles()
        .iter()
        .position(|candidate| candidate.key == profile.key)
    else {
        return;
    };
    let bit = 1u16 << profile_index;
    if state.ignored_lookup_mask & bit != 0 {
        return;
    }
    state.ignored_lookup_mask |= bit;
    if crate::debug::enabled() {
        let locked = profile_for_ui_chara_hash(state.logical_ui_hash)
            .map(|value| value.key)
            .unwrap_or("<none>");
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] lookup_ignored candidate={} reason={} bound_identity={} source=ui_lookup",
            profile.key,
            reason,
            locked
        );
    }
}

/// Observe the UI-character lookup without treating normal catalog enumeration
/// as a scanned Figure Player. The direct tagged form is only a provisional
/// Master Hand-correlated candidate; generic `0xc100..` row traversal remains
/// bounded diagnostics and never overwrites a candidate or verified identity.
///
/// This remains a raw-selection-hook operation: it does not query the stage,
/// FighterManager, NFP memory, or any battle object.
pub unsafe fn observe_logical_identity_lookup(raw_ui_hash: u64, ui_chara_hash: u64) {
    let Some(profile) = profile_for_ui_chara_hash(ui_chara_hash) else {
        return;
    };
    if !configured_for_preview(ui_chara_hash) {
        return;
    }

    let state = &mut *preview_state_ptr();
    if !is_direct_amiibo_identity_lookup(raw_ui_hash) {
        let reason = if state.identity_source == AmiiboPreviewIdentitySource::None {
            "generic_ui_enumeration_no_scan_provenance"
        } else {
            "preview_identity_candidate_bound"
        };
        log_ignored_lookup(state, profile, reason);
        return;
    }

    lock_preview_identity(ui_chara_hash);
}

/// A raw selected-fighter callback proves that the preceding lookup belonged
/// to character-selection state rather than a Figure Player viewer handoff.
/// It intentionally does not inspect the stage manager: this path can run
/// during UI bootstrap before battle-stage state exists.
pub unsafe fn discard_unbound_identity_from_raw_selection_callback() {
    let state = &mut *preview_state_ptr();
    // A direct Figure Player candidate is independent of ordinary CSS/WOL
    // selection callbacks. Do not let later catalog work clear or overwrite a
    // candidate that has already been bound for the next stage-0x135 host.
    if state.identity_source != AmiiboPreviewIdentitySource::None {
        return;
    }
    if !matches!(
        state.phase,
        AmiiboPreviewPhase::IdentityCaptured | AmiiboPreviewPhase::WaitingForViewerHost
    ) {
        return;
    }

    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] identity_discarded source=raw_selection_callback stage=unobserved_ui_hook generation={} ui_chara_hash=0x{:010x}",
            state.viewer_generation,
            state.logical_ui_hash
        );
    }
    state.logical_ui_hash = 0;
    state.phase = AmiiboPreviewPhase::Inactive;
    state.create_attempted = false;
    state.stabilization_reacquire_used = false;
    state.transform_stabilization_frames_remaining = 0;
    state.apply_profile_idle_motion = true;
    state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
    state.motion_probe_samples_remaining = MOTION_PROBE_SAMPLE_FRAMES;
    state.motion_probe_motion_before = 0;
    state.motion_probe_motion_after_change = 0;
    state.motion_probe_first_frame = -1.0;
    state.motion_probe_last_frame = -1.0;
    state.motion_probe_advanced = false;
    state.frames_since_viewer_host_ready = 0;
    state.frames_host_ready_to_create = 0;
    state.frames_create_to_engine_visible = 0;
    state.frames_create_to_model_visible = 0;
    state.have_item_ticks = 0;
    state.created_frame_mark = 0;
    state.visible_latency_logged = false;
    state.transform_comparison_samples_remaining = 0;
    state.transform_comparison_complete = false;
    state.interactive_transform_probe = InteractiveTransformProbe::empty();
    state.transform_calibration = TransformCalibrationState::empty();
    state.viewer_anchor = ViewerAnchor::empty();
    state.transform_ready_logged = false;
    state.native_transform_reset_logged = false;
    state.ready_visual_logged = false;
    state.visual_ready_blocked_logged = false;
    state.visibility_reassertion_used = false;
    state.last_item_visible = None;
    state.last_item_model_visible = None;
}

#[inline(always)]
unsafe fn host_object_id(module_accessor: *mut BattleObjectModuleAccessor) -> u32 {
    if module_accessor.is_null() {
        0
    } else {
        (*module_accessor).battle_object_id
    }
}

#[inline(always)]
unsafe fn viewer_held_item_slot_snapshot(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> ([u32; 4], [i32; 4]) {
    let mut item_ids = [0; 4];
    let mut item_kinds = [-1; 4];
    if module_accessor.is_null() {
        return (item_ids, item_kinds);
    }

    for slot in 0..4 {
        if !ItemModule::is_have_item(module_accessor, slot) {
            continue;
        }
        let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
        item_ids[slot as usize] = item_id;
        if item_id == 0 || !sv_battle_object::is_active(item_id) {
            continue;
        }
        let item_boma = sv_battle_object::module_accessor(item_id);
        if !item_boma.is_null() {
            item_kinds[slot as usize] = smash::app::utility::get_kind(&mut *item_boma);
        }
    }

    (item_ids, item_kinds)
}

#[inline(always)]
unsafe fn host_has_foreign_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    presentation_id: u32,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    for slot in 0..4 {
        if !ItemModule::is_have_item(module_accessor, slot) {
            continue;
        }
        let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
        if item_id != 0 && item_id != presentation_id {
            return true;
        }
    }
    false
}

#[inline(always)]
unsafe fn set_root_joint_rotation(
    module_accessor: *mut BattleObjectModuleAccessor,
    rotation: [f32; 3],
) {
    if module_accessor.is_null() {
        return;
    }

    let root_rotation = Vector3f {
        x: rotation[0],
        y: rotation[1],
        z: rotation[2],
    };
    ModelModule::set_joint_rotate(
        module_accessor,
        Hash40::new("root"),
        &root_rotation,
        smash::app::MotionNodeRotateCompose {
            _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
        },
        ModelModule::rotation_order(module_accessor),
    );
}

#[inline(always)]
unsafe fn root_joint_rotation(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> Option<[f32; 3]> {
    if module_accessor.is_null() {
        return None;
    }

    let mut rotation = Vector3f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    ModelModule::joint_rotate(module_accessor, Hash40::new("root"), &mut rotation);
    Some([rotation.x, rotation.y, rotation.z])
}

#[inline(always)]
unsafe fn apply_viewer_host_presentation_writes(
    module_accessor: *mut BattleObjectModuleAccessor,
    profile: &BossAmiiboPreviewProfile,
) -> ViewerHostPresentationApplyResult {
    if module_accessor.is_null() {
        return ViewerHostPresentationApplyResult::empty();
    }

    // Mirror the existing WOL preview host setup, but intentionally leave its
    // WOL-only host-position offsets out of stage 0x135. The native viewer
    // camera is already aimed at this host position.
    let rotation_before = viewer_host_rotation(module_accessor);
    let recipe = profile.host_orientation_recipe;
    crate::boss_helpers::clear_hidden_host_effects(module_accessor);
    // Preserve the WOL order: the verified boss item has already been scaled
    // and placed in its idle motion, then Mario becomes the tiny viewer host.
    ModelModule::set_scale(module_accessor, crate::boss_helpers::HIDDEN_HOST_SCALE);
    if MotionModule::motion_kind(module_accessor) != smash::hash40("none") {
        MotionModule::change_motion(
            module_accessor,
            Hash40::new("none"),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
    }

    // Host posture is written only for recipes that still mirror their WOL
    // posture composition (Galeem, Dharkon — not yet migrated). Recipes that
    // return None leave the posture channel entirely to the native Amiibo
    // viewer, which owns interactive stick rotation through it.
    let mut posture_written = false;
    if let Some(posture_rotation) = recipe.posture_rotation() {
        PostureModule::set_rot(
            module_accessor,
            &Vector3f {
                x: posture_rotation[0],
                y: posture_rotation[1],
                z: posture_rotation[2],
            },
            0,
        );
        posture_written = true;
    }
    let root_rotation_set = recipe.root_rotation();
    let root_written = root_rotation_set.is_some();
    if let Some(root_rotation) = root_rotation_set {
        set_root_joint_rotation(module_accessor, root_rotation);
    }
    let root_rotation_observed = if root_written {
        root_joint_rotation(module_accessor)
    } else {
        None
    };

    // Keep the native viewer host engine-visible. A boss item can inherit its
    // owner's global visibility state, so shrinking Mario is the WOL-proven
    // way to hide it without suppressing the presentation item.
    HitModule::set_whole(module_accessor, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
    JostleModule::set_status(module_accessor, false);

    ViewerHostPresentationApplyResult {
        recipe,
        rotation_before,
        rotation_after: viewer_host_rotation(module_accessor),
        root_rotation_set,
        root_rotation_observed,
        posture_written,
        root_written,
    }
}

/// Initial host presentation for creation and the bounded stabilization
/// window. Transform ownership is recipe-driven and identical to stable
/// maintenance; the split exists so the ownership boundary stays explicit.
#[inline(always)]
unsafe fn initialize_viewer_host_presentation_recipe(
    module_accessor: *mut BattleObjectModuleAccessor,
    profile: &BossAmiiboPreviewProfile,
) -> ViewerHostPresentationApplyResult {
    apply_viewer_host_presentation_writes(module_accessor, profile)
}

/// Stable Ready-frame host maintenance: tiny host scale, motion none, any
/// recipe-owned host corrections, hit/jostle off. The NativeHost
/// recipe owns no host channel, so this writes neither host posture nor host
/// root there and the native Amiibo viewer keeps interactive rotation.
#[inline(always)]
unsafe fn maintain_viewer_host_presentation(
    module_accessor: *mut BattleObjectModuleAccessor,
    profile: &BossAmiiboPreviewProfile,
) -> ViewerHostPresentationApplyResult {
    apply_viewer_host_presentation_writes(module_accessor, profile)
}

/// Every item-backed preview owns its boss-local transform channels (framing
/// position, item root, calibration overrides) and nothing on the native host.
/// Master Hand and Marx proved the architecture; the rest were migrated onto
/// it because the WOL host recipes left them mis-oriented and drifting.
#[inline(always)]
fn owns_boss_local_transform(profile: &BossAmiiboPreviewProfile) -> bool {
    matches!(
        profile.preview_kind,
        BossAmiiboPreviewKind::ItemPresentation
    )
}

#[inline(always)]
unsafe fn prepare_viewer_host_for_item_acquisition(
    module_accessor: *mut BattleObjectModuleAccessor,
    acquire_recipe: ItemPresentationAcquireRecipe,
) {
    if module_accessor.is_null() {
        return;
    }

    // Rathalos requires the WOL host to be tiny before its native item
    // request. Other previews retain their established order.
    crate::boss_helpers::clear_hidden_host_effects(module_accessor);
    if acquire_recipe.uses_tiny_host_before_request() {
        ModelModule::set_scale(module_accessor, crate::boss_helpers::HIDDEN_HOST_SCALE);
    }
}

#[inline(always)]
unsafe fn restore_viewer_host(module_accessor: *mut BattleObjectModuleAccessor) {
    if module_accessor.is_null() {
        return;
    }
    ModelModule::set_scale(module_accessor, 1.0);
    HitModule::set_whole(
        module_accessor,
        smash::app::HitStatus(*HIT_STATUS_NORMAL),
        0,
    );
    JostleModule::set_status(module_accessor, true);

    PostureModule::set_rot(
        module_accessor,
        &Vector3f {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        0,
    );
    set_root_joint_rotation(module_accessor, [0.0, 0.0, 0.0]);
}

#[inline(always)]
fn desired_preview_position(
    state: &AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
) -> Vector3f {
    Vector3f {
        x: state.viewer_anchor.position[0] + profile.position_offset[0],
        y: state.viewer_anchor.position[1] + profile.position_offset[1],
        z: state.viewer_anchor.position[2] + profile.position_offset[2],
    }
}

#[inline(always)]
unsafe fn viewer_host_position(module_accessor: *mut BattleObjectModuleAccessor) -> [f32; 3] {
    if module_accessor.is_null() {
        return [0.0; 3];
    }

    [
        PostureModule::pos_x(module_accessor),
        PostureModule::pos_y(module_accessor),
        PostureModule::pos_z(module_accessor),
    ]
}

#[inline(always)]
unsafe fn viewer_host_rotation(module_accessor: *mut BattleObjectModuleAccessor) -> [f32; 3] {
    if module_accessor.is_null() {
        return [0.0; 3];
    }

    [
        PostureModule::rot_x(module_accessor, 0),
        PostureModule::rot_y(module_accessor, 0),
        PostureModule::rot_z(module_accessor, 0),
    ]
}

#[inline(always)]
unsafe fn capture_viewer_anchor(
    state: &mut AmiiboPreviewState,
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() || state.viewer_anchor.initialized {
        return false;
    }

    let position = viewer_host_position(module_accessor);
    let rotation = viewer_host_rotation(module_accessor);
    state.viewer_anchor = ViewerAnchor {
        initial_position: position,
        initial_rotation: rotation,
        position,
        lr: PostureModule::lr(module_accessor),
        rotation,
        initialized: true,
    };
    true
}

#[inline(always)]
fn transform_matches(actual: f32, desired: f32) -> bool {
    (actual - desired).abs() <= 0.001
}

#[inline(always)]
fn rotation_matches(actual: [f32; 3], desired: [f32; 3]) -> bool {
    transform_matches(actual[0], desired[0])
        && transform_matches(actual[1], desired[1])
        && transform_matches(actual[2], desired[2])
}

#[inline(always)]
fn desired_item_presentation_rotation(
    profile: &BossAmiiboPreviewProfile,
    native_rotation: [f32; 3],
) -> [f32; 3] {
    profile.presentation_rotation.unwrap_or(native_rotation)
}

#[inline(always)]
unsafe fn presentation_rotation(item_boma: *mut BattleObjectModuleAccessor) -> [f32; 3] {
    [
        PostureModule::rot_x(item_boma, 0),
        PostureModule::rot_y(item_boma, 0),
        PostureModule::rot_z(item_boma, 0),
    ]
}

#[derive(Clone, Copy)]
struct ViewerHostPresentationApplyResult {
    recipe: AmiiboHostOrientationRecipe,
    rotation_before: [f32; 3],
    rotation_after: [f32; 3],
    root_rotation_set: Option<[f32; 3]>,
    root_rotation_observed: Option<[f32; 3]>,
    posture_written: bool,
    root_written: bool,
}

impl ViewerHostPresentationApplyResult {
    const fn empty() -> Self {
        Self {
            recipe: AmiiboHostOrientationRecipe::NativeFighter,
            rotation_before: [0.0; 3],
            rotation_after: [0.0; 3],
            root_rotation_set: None,
            root_rotation_observed: None,
            posture_written: false,
            root_written: false,
        }
    }
}

#[derive(Clone, Copy)]
struct ItemPresentationApplyResult {
    motion: u64,
    native_rotation_after_motion: [f32; 3],
    native_lr_after_motion: f32,
    desired_presentation_rotation: Option<[f32; 3]>,
    final_presentation_rotation: [f32; 3],
    motion_before: u64,
    motion_after_change: u64,
    frame_before: f32,
}

#[inline(always)]
unsafe fn host_slot_for_presentation(
    module_accessor: *mut BattleObjectModuleAccessor,
    presentation_id: u32,
) -> Option<(i32, u32)> {
    if module_accessor.is_null() || presentation_id == 0 {
        return None;
    }

    for slot in 0..4 {
        if !ItemModule::is_have_item(module_accessor, slot) {
            continue;
        }
        let item_id = ItemModule::get_have_item_id(module_accessor, slot) as u32;
        if item_id == presentation_id {
            return Some((slot, item_id));
        }
    }
    None
}

#[inline(always)]
unsafe fn apply_item_presentation(
    item_boma: *mut BattleObjectModuleAccessor,
    state: &AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
) -> ItemPresentationApplyResult {
    // Match the WOL preview ordering: establish the presentation scale before
    // selecting the boss's native idle motion. Position remains viewer-owned
    // and is written after motion initialization.
    if let Some(scale) = profile.preview_scale {
        ModelModule::set_scale(item_boma, scale);
    }
    // The profile's idle motion is proven for that profile's own backing only.
    // A menu-visual-fallback object is a different item, so it keeps whatever
    // motion the game gave it rather than being forced into a boss animation.
    //
    // No status request is issued here. Hardware disproved that: the working
    // WOL previews (Ganon, Galleom, Rathalos, Master Hand) only ever call
    // `change_motion`, and requesting a boss status made the item leave the
    // host slot (`ownership=detached_native_owned`), which stopped it
    // following the native turntable.
    let expected_motion = smash::hash40(profile.idle_motion);
    let motion_before = MotionModule::motion_kind(item_boma);
    let frame_before = MotionModule::frame(item_boma);
    if state.apply_profile_idle_motion && motion_before != expected_motion {
        MotionModule::change_motion(
            item_boma,
            Hash40::new(profile.idle_motion),
            0.0,
            1.0,
            false,
            0.0,
            false,
            false,
        );
    }

    // Native item initialization can reset the transform after acquisition.
    // Apply viewer-owned position after motion selection. Most profiles retain
    // their native item orientation; a profile may opt into a stage-0x135-only
    // correction after its native motion has initialized.
    let native_rotation_after_motion = presentation_rotation(item_boma);
    let native_lr_after_motion = PostureModule::lr(item_boma);
    let position = desired_preview_position(state, profile);
    PostureModule::set_pos(item_boma, &position);
    if let Some(rotation) = profile.presentation_rotation {
        PostureModule::set_rot(
            item_boma,
            &Vector3f {
                x: rotation[0],
                y: rotation[1],
                z: rotation[2],
            },
            0,
        );
    }
    // Boss-local static orientation on the item's own root joint (configured
    // profile value or live calibration override). Written here so creation
    // and the stabilization window match stable maintenance.
    if let Some(rotation) = effective_item_root_rotation(state, profile) {
        set_root_joint_rotation(item_boma, rotation);
    }

    maintain_item_presentation_safety(item_boma);
    let motion = MotionModule::motion_kind(item_boma);
    ItemPresentationApplyResult {
        motion,
        native_rotation_after_motion,
        native_lr_after_motion,
        desired_presentation_rotation: profile.presentation_rotation,
        final_presentation_rotation: presentation_rotation(item_boma),
        motion_before,
        motion_after_change: motion,
        frame_before,
    }
}

#[inline(always)]
unsafe fn maintain_item_presentation_safety(item_boma: *mut BattleObjectModuleAccessor) {
    AttackModule::clear_all(item_boma);
    HitModule::set_whole(item_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
    DamageModule::set_damage_lock(item_boma, true);
    JostleModule::set_status(item_boma, false);
}

#[inline(always)]
fn is_transform_comparison_profile(profile: &BossAmiiboPreviewProfile) -> bool {
    profile.key == MASTER_HAND_PREVIEW_KEY || profile.key == MARX_PREVIEW_KEY
}

#[derive(Clone, Copy)]
struct StableItemMaintenanceResult {
    motion: u64,
    item_root_before: Option<[f32; 3]>,
    item_root_after: Option<[f32; 3]>,
    item_root_written: bool,
    item_posture_written: bool,
    item_position_written: bool,
}

impl StableItemMaintenanceResult {
    const fn empty() -> Self {
        Self {
            motion: 0,
            item_root_before: None,
            item_root_after: None,
            item_root_written: false,
            item_posture_written: false,
            item_position_written: false,
        }
    }
}

/// Boss-local static orientation for the presentation item's own root joint:
/// the debug calibration override wins, then the configured profile value.
/// None leaves the channel fully native.
fn effective_item_root_rotation(
    state: &AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
) -> Option<[f32; 3]> {
    state
        .transform_calibration
        .override_for(CalibrationTarget::ItemRoot)
        .or(profile.item_root_rotation)
}

/// Stable Ready-frame item maintenance. Keeps the item inert everywhere; for
/// every item-backed boss it additionally owns the boss-local channels:
/// - framing position (anchor + profile offset): hardware showed the
///   host-held item follows native host movement once stabilization ends and
///   drifts low/off-frame, so the plugin keeps the ITEM position pinned
///   without touching any host channel.
/// - the item's own root joint (configured value or calibration override).
/// - the item posture rotation, but only while a calibration override exists;
///   otherwise it stays a creation/stabilization-window write.
unsafe fn maintain_stable_item_presentation(
    state: &AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    item_boma: *mut BattleObjectModuleAccessor,
) -> StableItemMaintenanceResult {
    let boss_local = owns_boss_local_transform(profile);
    let item_root_before = if boss_local {
        root_joint_rotation(item_boma)
    } else {
        None
    };
    maintain_item_presentation_safety(item_boma);

    let mut item_position_written = false;
    let mut item_root_written = false;
    let mut item_posture_written = false;
    if boss_local {
        let position = desired_preview_position(state, profile);
        PostureModule::set_pos(item_boma, &position);
        item_position_written = true;

        if let Some(rotation) = effective_item_root_rotation(state, profile) {
            set_root_joint_rotation(item_boma, rotation);
            item_root_written = true;
        }

        if let Some(rotation) = state
            .transform_calibration
            .override_for(CalibrationTarget::ItemPosture)
        {
            PostureModule::set_rot(
                item_boma,
                &Vector3f {
                    x: rotation[0],
                    y: rotation[1],
                    z: rotation[2],
                },
                0,
            );
            item_posture_written = true;
        }
    }

    StableItemMaintenanceResult {
        motion: MotionModule::motion_kind(item_boma),
        item_root_before,
        item_root_after: if boss_local {
            root_joint_rotation(item_boma)
        } else {
            None
        },
        item_root_written,
        item_posture_written,
        item_position_written,
    }
}

/// Reads the value the calibration harness should continue dialing from: the
/// live override if present, otherwise the channel's current actual value.
unsafe fn calibration_channel_value(
    state: &AmiiboPreviewState,
    target: CalibrationTarget,
    host_boma: *mut BattleObjectModuleAccessor,
    item_boma: *mut BattleObjectModuleAccessor,
) -> [f32; 3] {
    if let Some(value) = state.transform_calibration.override_for(target) {
        return value;
    }
    match target {
        CalibrationTarget::ItemRoot => root_joint_rotation(item_boma).unwrap_or([0.0; 3]),
        CalibrationTarget::ItemPosture => presentation_rotation(item_boma),
        CalibrationTarget::HostRoot => root_joint_rotation(host_boma).unwrap_or([0.0; 3]),
    }
}

unsafe fn apply_calibration_target_write(
    target: CalibrationTarget,
    value: [f32; 3],
    host_boma: *mut BattleObjectModuleAccessor,
    item_boma: *mut BattleObjectModuleAccessor,
) {
    match target {
        CalibrationTarget::ItemRoot => set_root_joint_rotation(item_boma, value),
        CalibrationTarget::ItemPosture => PostureModule::set_rot(
            item_boma,
            &Vector3f {
                x: value[0],
                y: value[1],
                z: value[2],
            },
            0,
        ),
        CalibrationTarget::HostRoot => set_root_joint_rotation(host_boma, value),
    }
}

/// Clears the current target's override and writes the configured baseline
/// back once so the model visibly returns to its uncalibrated state.
unsafe fn reset_calibration_target(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    host_boma: *mut BattleObjectModuleAccessor,
    item_boma: *mut BattleObjectModuleAccessor,
) {
    let target = state.transform_calibration.target;
    state.transform_calibration.overrides[target.index()] = None;
    match target {
        CalibrationTarget::ItemRoot => {
            set_root_joint_rotation(item_boma, profile.item_root_rotation.unwrap_or([0.0; 3]));
        }
        CalibrationTarget::ItemPosture => {
            if let Some(rotation) = profile.presentation_rotation {
                apply_calibration_target_write(target, rotation, host_boma, item_boma);
            }
        }
        CalibrationTarget::HostRoot => {
            set_root_joint_rotation(
                host_boma,
                profile
                    .host_orientation_recipe
                    .root_rotation()
                    .unwrap_or([0.0; 3]),
            );
        }
    }
}

unsafe fn log_transform_calibration(
    state: &AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    host_boma: *mut BattleObjectModuleAccessor,
    item_boma: *mut BattleObjectModuleAccessor,
    slot_still_held: bool,
    action: &str,
    delta: f32,
) {
    let calibration = &state.transform_calibration;
    let host_posture = viewer_host_rotation(host_boma);
    let host_root = root_joint_rotation(host_boma).unwrap_or([0.0; 3]);
    let item_posture = presentation_rotation(item_boma);
    let item_root = root_joint_rotation(item_boma).unwrap_or([0.0; 3]);
    crate::boss_log!(
        "[PB][AmiiboTransformCalibration] generation={} logical_boss={} action={} target={} axis={} delta={:.0} override={:?} host_posture=({:.1},{:.1},{:.1}) host_root=({:.1},{:.1},{:.1}) item_posture=({:.1},{:.1},{:.1}) item_root=({:.1},{:.1},{:.1}) slot_still_held={}",
        state.viewer_generation,
                    profile.key,
        action,
        calibration.target.name(),
        CALIBRATION_AXIS_NAMES[calibration.axis.min(2)],
        delta,
        calibration.override_for(calibration.target),
        host_posture[0],
        host_posture[1],
        host_posture[2],
        host_root[0],
        host_root[1],
        host_root[2],
        item_posture[0],
        item_posture[1],
        item_posture[2],
        item_root[0],
        item_root[1],
        item_root[2],
        slot_still_held
    );
}

/// Debug-only, stage-0x135-only, item-preview-only calibration input handler.
///
/// Chord: hold SHIELD (GUARD) + GRAB (CATCH) together on the viewer pad.
/// While the chord is held:
/// - d-pad up / down:    +15 / -15 degrees on the selected axis
/// - d-pad right / left: +5 / -5 degrees on the selected axis
/// - A tap:              cycle axis x -> y -> z
/// - X or Y (jump) tap:  cycle target item_root -> item_posture -> host_root
/// - A held ~1s:         reset the current target to its configured baseline
///
/// Engaging the chord logs a snapshot without changing anything.
///
/// The right stick is intentionally never read: it is Nintendo's native
/// turntable. Whether this menu scene feeds the host fighter's ControlModule
/// is hardware-unproven, so a bounded input-liveness probe logs button-bit
/// changes; a permanently zero bitfield means the pad is not reaching the
/// fighter and the harness needs a different input source (the pinned skyline
/// crate exposes no nn::hid bindings, so none was invented here).
unsafe fn process_transform_calibration_input(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    host_boma: *mut BattleObjectModuleAccessor,
    item_boma: *mut BattleObjectModuleAccessor,
    slot_still_held: bool,
) {
    if !crate::debug::enabled() || !owns_boss_local_transform(profile) {
        return;
    }
    if host_boma.is_null() || item_boma.is_null() {
        return;
    }

    let raw_buttons = ControlModule::get_button(host_boma);
    let bits_changed = {
        let calibration = &mut state.transform_calibration;
        let changed =
            !calibration.has_last_input_bits || raw_buttons != calibration.last_input_bits;
        calibration.last_input_bits = raw_buttons;
        calibration.has_last_input_bits = true;
        changed
    };
    if bits_changed && state.transform_calibration.input_probe_samples_remaining > 0 {
        state.transform_calibration.input_probe_samples_remaining -= 1;
        crate::boss_log!(
            "[PB][AmiiboCalibrationInput] generation={} logical_boss={} buttons=0x{:x} guard={} catch={} attack={} jump={} appeal_hi={} appeal_lw={} appeal_s_l={} appeal_s_r={} samples_remaining={}",
            state.viewer_generation,
            profile.key,
            raw_buttons,
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_GUARD),
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_CATCH),
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_ATTACK),
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_JUMP),
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_APPEAL_HI),
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_APPEAL_LW),
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_APPEAL_S_L),
            ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_APPEAL_S_R),
            state.transform_calibration.input_probe_samples_remaining
        );
    }

    let chord_held = ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_GUARD)
        && ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_CATCH);
    if !chord_held {
        let calibration = &mut state.transform_calibration;
        calibration.chord_engaged = false;
        calibration.attack_hold_frames = 0;
        calibration.attack_reset_consumed = false;
        return;
    }

    if !state.transform_calibration.chord_engaged {
        state.transform_calibration.chord_engaged = true;
        log_transform_calibration(
            state,
            profile,
            host_boma,
            item_boma,
            slot_still_held,
            "snapshot",
            0.0,
        );
    }

    if ControlModule::check_button_on(host_boma, *CONTROL_PAD_BUTTON_ATTACK) {
        state.transform_calibration.attack_hold_frames = state
            .transform_calibration
            .attack_hold_frames
            .saturating_add(1);
        if state.transform_calibration.attack_hold_frames >= CALIBRATION_RESET_HOLD_FRAMES
            && !state.transform_calibration.attack_reset_consumed
        {
            state.transform_calibration.attack_reset_consumed = true;
            reset_calibration_target(state, profile, host_boma, item_boma);
            log_transform_calibration(
                state,
                profile,
                host_boma,
                item_boma,
                slot_still_held,
                "reset",
                0.0,
            );
        }
    } else {
        let held_frames = state.transform_calibration.attack_hold_frames;
        let reset_consumed = state.transform_calibration.attack_reset_consumed;
        state.transform_calibration.attack_hold_frames = 0;
        state.transform_calibration.attack_reset_consumed = false;
        if held_frames > 0 && held_frames < CALIBRATION_RESET_HOLD_FRAMES && !reset_consumed {
            state.transform_calibration.axis = (state.transform_calibration.axis + 1) % 3;
            log_transform_calibration(
                state,
                profile,
                host_boma,
                item_boma,
                slot_still_held,
                "cycle_axis",
                0.0,
            );
        }
    }

    if ControlModule::check_button_trigger(host_boma, *CONTROL_PAD_BUTTON_JUMP) {
        state.transform_calibration.target = state.transform_calibration.target.next();
        log_transform_calibration(
            state,
            profile,
            host_boma,
            item_boma,
            slot_still_held,
            "cycle_target",
            0.0,
        );
    }

    let mut delta = 0.0f32;
    if ControlModule::check_button_trigger(host_boma, *CONTROL_PAD_BUTTON_APPEAL_HI) {
        delta += CALIBRATION_COARSE_STEP_DEGREES;
    }
    if ControlModule::check_button_trigger(host_boma, *CONTROL_PAD_BUTTON_APPEAL_LW) {
        delta -= CALIBRATION_COARSE_STEP_DEGREES;
    }
    if ControlModule::check_button_trigger(host_boma, *CONTROL_PAD_BUTTON_APPEAL_S_R) {
        delta += CALIBRATION_FINE_STEP_DEGREES;
    }
    if ControlModule::check_button_trigger(host_boma, *CONTROL_PAD_BUTTON_APPEAL_S_L) {
        delta -= CALIBRATION_FINE_STEP_DEGREES;
    }
    if delta != 0.0 {
        let target = state.transform_calibration.target;
        let axis = state.transform_calibration.axis.min(2);
        let mut value = calibration_channel_value(state, target, host_boma, item_boma);
        value[axis] = wrap_calibration_degrees(value[axis] + delta);
        state.transform_calibration.overrides[target.index()] = Some(value);
        apply_calibration_target_write(target, value, host_boma, item_boma);
        log_transform_calibration(
            state,
            profile,
            host_boma,
            item_boma,
            slot_still_held,
            "adjust",
            delta,
        );
    }
}

/// Bounded proof-pair diagnostics (Master Hand + Marx). Logs the full local
/// transform chain once per generation as [PB][AmiiboLocalTransform], then a
/// bounded number of native host-posture *changes* (stick rotation) as
/// [PB][AmiiboInteractiveTransform]. A changing host posture while
/// plugin_host_posture_write=false and a stable boss-local correction is the
/// hardware success signature; the changing value must never be "restored".
#[allow(clippy::too_many_arguments)]
unsafe fn observe_interactive_transform_probe(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    item_boma: *mut BattleObjectModuleAccessor,
    host_boma: *mut BattleObjectModuleAccessor,
    presentation_id: u32,
    slot_still_held: bool,
    host_presentation: &ViewerHostPresentationApplyResult,
    stable_item: &StableItemMaintenanceResult,
) {
    if !owns_boss_local_transform(profile) || !crate::debug::enabled() {
        return;
    }
    if item_boma.is_null() || host_boma.is_null() {
        return;
    }

    let probe = &mut state.interactive_transform_probe;
    if !probe.stable_phase_entered {
        probe.stable_phase_entered = true;
        probe.change_samples_remaining = INTERACTIVE_TRANSFORM_CHANGE_SAMPLES;
        probe.last_host_posture = viewer_host_rotation(host_boma);
        probe.has_last_host_posture = true;
    }

    let host_posture = viewer_host_rotation(host_boma);
    let host_root_joint = root_joint_rotation(host_boma).unwrap_or([0.0; 3]);
    let item_rotation = presentation_rotation(item_boma);
    let item_position = [
        PostureModule::pos_x(item_boma),
        PostureModule::pos_y(item_boma),
        PostureModule::pos_z(item_boma),
    ];
    let host_position = viewer_host_position(host_boma);
    let host_lr = PostureModule::lr(host_boma);
    let item_lr = PostureModule::lr(item_boma);
    let stabilization_frames = state.transform_stabilization_frames_remaining;

    if !probe.stable_state_logged {
        probe.stable_state_logged = true;
        crate::boss_log!(
            "[PB][AmiiboLocalTransform] generation={} logical_boss={} phase=stable_maintenance presentation_object_id=0x{:x} ownership={} slot_still_held={} host_posture=({:.1},{:.1},{:.1}) host_root=({:.1},{:.1},{:.1}) configured_host_root={:?} item_posture=({:.1},{:.1},{:.1}) item_root_before={:?} configured_item_root={:?} calibration_item_root={:?} item_root_after={:?} host_lr={:.3} item_lr={:.3} host_position=({:.3},{:.3},{:.3}) item_position=({:.3},{:.3},{:.3}) plugin_host_posture_write={} plugin_host_root_write={} plugin_item_posture_write={} plugin_item_root_write={} plugin_item_position_write={} stabilization_frames_remaining={}",
            state.viewer_generation,
            profile.key,
            presentation_id,
            state.ownership.name(),
            slot_still_held,
            host_posture[0],
            host_posture[1],
            host_posture[2],
            host_root_joint[0],
            host_root_joint[1],
            host_root_joint[2],
            host_presentation.root_rotation_set,
            item_rotation[0],
            item_rotation[1],
            item_rotation[2],
            stable_item.item_root_before,
            profile.item_root_rotation,
            state
                .transform_calibration
                .override_for(CalibrationTarget::ItemRoot),
            stable_item.item_root_after,
            host_lr,
            item_lr,
            host_position[0],
            host_position[1],
            host_position[2],
            item_position[0],
            item_position[1],
            item_position[2],
            host_presentation.posture_written,
            host_presentation.root_written,
            stable_item.item_posture_written,
            stable_item.item_root_written,
            stable_item.item_position_written,
            stabilization_frames
        );
        return;
    }

    if probe.change_samples_remaining == 0 || !probe.has_last_host_posture {
        return;
    }
    if rotation_matches(host_posture, probe.last_host_posture) {
        return;
    }

    let previous = probe.last_host_posture;
    probe.last_host_posture = host_posture;
    probe.change_samples_remaining = probe.change_samples_remaining.saturating_sub(1);
    let sample = INTERACTIVE_TRANSFORM_CHANGE_SAMPLES - probe.change_samples_remaining;
    crate::boss_log!(
        "[PB][AmiiboInteractiveTransform] generation={} logical_boss={} phase=host_posture_changed sample={}/{} presentation_object_id=0x{:x} ownership={} slot_still_held={} host_posture_rotation_before=({:.1},{:.1},{:.1}) host_posture_rotation=({:.1},{:.1},{:.1}) actual_host_root_joint_rotation=({:.1},{:.1},{:.1}) item_posture_rotation=({:.1},{:.1},{:.1}) item_root={:?} host_lr={:.3} item_lr={:.3} host_position=({:.3},{:.3},{:.3}) item_position=({:.3},{:.3},{:.3}) plugin_host_posture_write={} plugin_host_root_write={} plugin_item_posture_write={} plugin_item_root_write={} plugin_item_position_write={} stabilization_frames_remaining={}",
        state.viewer_generation,
                profile.key,
        sample,
        INTERACTIVE_TRANSFORM_CHANGE_SAMPLES,
        presentation_id,
        state.ownership.name(),
        slot_still_held,
        previous[0],
        previous[1],
        previous[2],
        host_posture[0],
        host_posture[1],
        host_posture[2],
        host_root_joint[0],
        host_root_joint[1],
        host_root_joint[2],
        item_rotation[0],
        item_rotation[1],
        item_rotation[2],
        stable_item.item_root_after,
        host_lr,
        item_lr,
        host_position[0],
        host_position[1],
        host_position[2],
        item_position[0],
        item_position[1],
        item_position[2],
        host_presentation.posture_written,
        host_presentation.root_written,
        stable_item.item_posture_written,
        stable_item.item_root_written,
        stable_item.item_position_written,
        stabilization_frames
    );
}

/// Captures the post-host-recipe item transform for the Master Hand/Marx
/// control pair. The item is still host-held in both target traces, so these
/// bounded samples distinguish host composition from model-space pivot data.
unsafe fn observe_transform_comparison(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    item_boma: *mut BattleObjectModuleAccessor,
    host_boma: *mut BattleObjectModuleAccessor,
    presentation_id: u32,
    slot_still_held: bool,
    host_presentation: ViewerHostPresentationApplyResult,
) {
    if !is_transform_comparison_profile(profile)
        || state.transform_comparison_complete
        || !crate::debug::enabled()
    {
        return;
    }

    if state.transform_comparison_samples_remaining == 0 {
        state.transform_comparison_samples_remaining = TRANSFORM_COMPARISON_SAMPLE_FRAMES;
    }

    let sample = TRANSFORM_COMPARISON_SAMPLE_FRAMES
        .saturating_sub(state.transform_comparison_samples_remaining)
        .saturating_add(1);
    let item_position = [
        PostureModule::pos_x(item_boma),
        PostureModule::pos_y(item_boma),
        PostureModule::pos_z(item_boma),
    ];
    let item_rotation = presentation_rotation(item_boma);
    let host_position = viewer_host_position(host_boma);
    let host_rotation = viewer_host_rotation(host_boma);
    crate::boss_log!(
        "[PB][AmiiboTransformCompare] generation={} logical_boss={} sample={}/{} presentation_object_id=0x{:x} ownership={} slot_still_held={} item_posture_position=({:.3},{:.3},{:.3}) item_posture_rotation=({:.1},{:.1},{:.1}) item_lr={:.3} host_posture_position=({:.3},{:.3},{:.3}) host_posture_rotation=({:.1},{:.1},{:.1}) host_lr={:.3} configured_item_presentation_rotation={:?} configured_host_posture_rotation={:?} configured_host_root_rotation={:?} actual_host_root_joint_rotation={:?} final_item_position_after_host_recipe=({:.3},{:.3},{:.3}) final_item_rotation_after_host_recipe=({:.1},{:.1},{:.1})",
        state.viewer_generation,
        profile.key,
        sample,
        TRANSFORM_COMPARISON_SAMPLE_FRAMES,
        presentation_id,
        state.ownership.name(),
        slot_still_held,
        item_position[0],
        item_position[1],
        item_position[2],
        item_rotation[0],
        item_rotation[1],
        item_rotation[2],
        PostureModule::lr(item_boma),
        host_position[0],
        host_position[1],
        host_position[2],
        host_rotation[0],
        host_rotation[1],
        host_rotation[2],
        PostureModule::lr(host_boma),
        profile.presentation_rotation,
        profile.host_orientation_recipe.posture_rotation(),
        profile.host_orientation_recipe.root_rotation(),
        host_presentation.root_rotation_observed,
        item_position[0],
        item_position[1],
        item_position[2],
        item_rotation[0],
        item_rotation[1],
        item_rotation[2],
    );

    state.transform_comparison_samples_remaining -= 1;
    if state.transform_comparison_samples_remaining == 0 {
        state.transform_comparison_complete = true;
    }
}

#[inline(always)]
unsafe fn observe_presentation_visibility(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    item_boma: *mut BattleObjectModuleAccessor,
    host_boma: *mut BattleObjectModuleAccessor,
    boundary: &str,
) -> (bool, bool) {
    let item_visible = VisibilityModule::is_visible(item_boma);
    let item_model_visible = ModelModule::is_visible(item_boma);
    let changed = state.last_item_visible != Some(item_visible)
        || state.last_item_model_visible != Some(item_model_visible);
    if changed && crate::debug::enabled() {
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] visibility_transition generation={} logical_boss={} presentation_object_id=0x{:x} boundary={} previous_item_visible={:?} item_visible={} previous_item_model_visible={:?} item_model_visible={} host_engine_visible={} host_model_visible={} host_scale={:.4}",
            state.viewer_generation,
            profile.key,
            state.presentation_object_id,
            boundary,
            state.last_item_visible,
            item_visible,
            state.last_item_model_visible,
            item_model_visible,
            VisibilityModule::is_visible(host_boma),
            ModelModule::is_visible(host_boma),
            ModelModule::scale(host_boma),
        );
    }
    state.last_item_visible = Some(item_visible);
    state.last_item_model_visible = Some(item_model_visible);
    (item_visible, item_model_visible)
}

#[inline(always)]
unsafe fn initialize_presentation_visibility(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    item_boma: *mut BattleObjectModuleAccessor,
    host_boma: *mut BattleObjectModuleAccessor,
) {
    let before_item_visible = VisibilityModule::is_visible(item_boma);
    let before_item_model_visible = ModelModule::is_visible(item_boma);

    // The WOL-derived host recipe keeps Mario engine-visible. Explicitly make
    // the newly acquired item visible once after native initialization; later
    // frames only observe transitions and may perform one bounded correction.
    VisibilityModule::set_whole(item_boma, true);

    let after_item_visible = VisibilityModule::is_visible(item_boma);
    let after_item_model_visible = ModelModule::is_visible(item_boma);
    state.last_item_visible = Some(after_item_visible);
    state.last_item_model_visible = Some(after_item_model_visible);
    if crate::debug::enabled() {
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] presentation_visibility_initialized generation={} logical_boss={} presentation_object_id=0x{:x} item_visible_before={} item_model_visible_before={} item_visible_after={} item_model_visible_after={} host_engine_visible={} host_model_visible={} host_scale={:.4}",
            state.viewer_generation,
            profile.key,
            state.presentation_object_id,
            before_item_visible,
            before_item_model_visible,
            after_item_visible,
            after_item_model_visible,
            VisibilityModule::is_visible(host_boma),
            ModelModule::is_visible(host_boma),
            ModelModule::scale(host_boma),
        );
    }
}

#[inline(always)]
unsafe fn ensure_presentation_visibility_once(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    item_boma: *mut BattleObjectModuleAccessor,
    host_boma: *mut BattleObjectModuleAccessor,
) -> (bool, bool) {
    let (mut item_visible, mut item_model_visible) =
        observe_presentation_visibility(state, profile, item_boma, host_boma, "ready_check");
    if item_visible && item_model_visible {
        return (item_visible, item_model_visible);
    }

    if !state.visibility_reassertion_used {
        state.visibility_reassertion_used = true;
        VisibilityModule::set_whole(item_boma, true);
        (item_visible, item_model_visible) = observe_presentation_visibility(
            state,
            profile,
            item_boma,
            host_boma,
            "one_time_visibility_reassertion",
        );
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] visibility_reasserted generation={} logical_boss={} presentation_object_id=0x{:x} item_visible={} item_model_visible={} host_engine_visible={} host_scale={:.4}",
                state.viewer_generation,
                profile.key,
                state.presentation_object_id,
                item_visible,
                item_model_visible,
                VisibilityModule::is_visible(host_boma),
                ModelModule::scale(host_boma),
            );
        }
    }

    (item_visible, item_model_visible)
}

#[inline(always)]
unsafe fn log_rathalos_acquire_probe(
    state: &AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    requested_item_kind: i32,
    phase: &str,
    acquired: bool,
    action: &str,
) {
    if !crate::debug::enabled() {
        return;
    }

    let (slot_ids, slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
    crate::boss_log!(
        "[PB][AmiiboRathalosAcquire] generation={} logical_boss={} phase={} host_settle_frames={} frame_since_request={} request_count={} host_object_id=0x{:x} host_scale={:.4} host_status={} host_motion=0x{:x} requested_kind={} slot_ids={:?} slot_kinds={:?} acquired={} action={}",
        state.viewer_generation,
        profile.key,
        phase,
        state.rathalos_acquire_probe.host_settle_frames,
        state.rathalos_acquire_probe.frames_since_request,
        state.rathalos_acquire_probe.request_count,
        host_object_id(module_accessor),
        ModelModule::scale(module_accessor),
        StatusModule::status_kind(module_accessor),
        MotionModule::motion_kind(module_accessor),
        requested_item_kind,
        slot_ids,
        slot_kinds,
        acquired,
        action,
    );
}

/// Acquisition is only ever accepted on an exact kind match. Provenance —
/// "the slot was empty before our own request" — does not establish semantic
/// identity, and hardware shows Rathalos's boss backing leaves no held object
/// at all, so there is nothing an adopt-anything fallback could correctly
/// claim.
#[inline(always)]
unsafe fn exact_kind_acquired_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    requested_item_kind: i32,
) -> Option<(i32, u32, *mut BattleObjectModuleAccessor)> {
    crate::boss_helpers::held_item_by_kind(module_accessor, &[requested_item_kind])
}

/// Replays the source-proven WOL preconditions once per bounded request:
/// clear host slots, scale Mario to the hidden-host scale, then request the
/// native boss item. This function never retries on its own.
#[inline(always)]
unsafe fn request_rathalos_presentation_item(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    requested_item_kind: i32,
    request_source: &str,
) -> Option<(i32, u32, *mut BattleObjectModuleAccessor)> {
    debug_assert!(profile.item_acquire_recipe.uses_deferred_observation());

    let host_scale_before = ModelModule::scale(module_accessor);
    let host_status_before = StatusModule::status_kind(module_accessor);
    let host_motion_before = MotionModule::motion_kind(module_accessor);
    let (before_slot_ids, before_slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
    ItemModule::remove_all(module_accessor);
    let (cleared_slot_ids, cleared_slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
    prepare_viewer_host_for_item_acquisition(module_accessor, profile.item_acquire_recipe);

    state.rathalos_acquire_probe.request_count =
        state.rathalos_acquire_probe.request_count.saturating_add(1);
    state.rathalos_acquire_probe.frames_since_request = 0;

    ItemModule::have_item(
        module_accessor,
        ItemKind(requested_item_kind),
        0,
        0,
        false,
        false,
    );
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);

    let acquired = exact_kind_acquired_item(module_accessor, requested_item_kind);
    if crate::debug::enabled() {
        let (slot_ids, slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
        crate::boss_log!(
            "[PB][AmiiboRathalosAcquire] generation={} logical_boss={} phase=request_issued source={} host_settle_frames={} frame_since_request=0 request_count={} host_object_id=0x{:x} host_scale_before={:.4} host_scale_at_have_item={:.4} host_status_before={} host_motion_before=0x{:x} before_slot_ids={:?} before_slot_kinds={:?} cleared_slot_ids={:?} cleared_slot_kinds={:?} requested_kind={} item_have_called=true slot_ids_immediately_after={:?} slot_kinds_immediately_after={:?} acquired={}",
            state.viewer_generation,
            profile.key,
            request_source,
            state.rathalos_acquire_probe.host_settle_frames,
            state.rathalos_acquire_probe.request_count,
            host_object_id(module_accessor),
            host_scale_before,
            ModelModule::scale(module_accessor),
            host_status_before,
            host_motion_before,
            before_slot_ids,
            before_slot_kinds,
            cleared_slot_ids,
            cleared_slot_kinds,
            requested_item_kind,
            slot_ids,
            slot_kinds,
            acquired.is_some(),
        );
    }
    acquired
}

/// One menu-only visual fallback request, made once, after the boss backing
/// has exhausted its proven bound. This is presentation-only: it does not
/// touch `ui_chara_lioleus` identity, the WOL Rathalos backing, the gameplay
/// Rathalos backing, or regular Smash behavior. The host is restored to a
/// normal fighter state first, because the hidden-host precondition belongs to
/// the boss-only backing and is not known to be required for a normal item.
#[inline(always)]
unsafe fn request_rathalos_menu_visual_fallback(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    primary_item_kind: i32,
) -> Option<(i32, u32, *mut BattleObjectModuleAccessor)> {
    debug_assert!(profile.item_acquire_recipe.uses_menu_visual_fallback());

    let fallback_item_kind = *ITEM_KIND_LIOLEUS;
    ItemModule::remove_all(module_accessor);
    restore_viewer_host(module_accessor);
    state.host_hidden = false;

    state.rathalos_acquire_probe.fallback_requested = true;
    state.rathalos_acquire_probe.frames_since_fallback = 0;

    ItemModule::have_item(
        module_accessor,
        ItemKind(fallback_item_kind),
        0,
        0,
        false,
        false,
    );
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);

    let acquired = exact_kind_acquired_item(module_accessor, fallback_item_kind);
    if crate::debug::enabled() {
        let (slot_ids, slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
        crate::boss_log!(
            "[PB][AmiiboRathalosAcquire] generation={} logical_boss={} phase=menu_visual_fallback_request requested_kind={} primary_kind={} exact_kind_acquired={} host_object_id=0x{:x} host_scale={:.4} slot_ids={:?} slot_kinds={:?} presentation_only=true identity_unchanged=true",
            state.viewer_generation,
            profile.key,
            fallback_item_kind,
            primary_item_kind,
            acquired.is_some(),
            host_object_id(module_accessor),
            ModelModule::scale(module_accessor),
            slot_ids,
            slot_kinds,
        );
    }
    acquired
}

/// Presents a menu-visual-fallback object. It gets the hardware-good stage-135
/// presentation ownership (native host, item-local orientation, anchor pin,
/// scale, inert state) but keeps its own native motion.
#[inline(always)]
unsafe fn activate_rathalos_menu_visual_fallback(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    host_id: u32,
    slot: i32,
    presentation_id: u32,
    presentation_boma: *mut BattleObjectModuleAccessor,
) -> bool {
    state.apply_profile_idle_motion = false;
    let activated = activate_verified_item_presentation(
        state,
        profile,
        module_accessor,
        host_id,
        *ITEM_KIND_LIOLEUS,
        "ITEM_KIND_LIOLEUS (menu visual fallback)",
        slot,
        presentation_id,
        presentation_boma,
    );
    if !activated {
        state.apply_profile_idle_motion = true;
    }
    activated
}

#[inline(always)]
unsafe fn defer_rathalos_presentation(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    requested_item_kind: i32,
    reason: &str,
) -> bool {
    log_rathalos_acquire_probe(
        state,
        profile,
        module_accessor,
        requested_item_kind,
        "exhausted",
        false,
        reason,
    );
    restore_viewer_host(module_accessor);
    state.host_hidden = false;
    state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
    state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
    false
}

/// The two bosses whose Amiibo visual backing is the full boss object rather
/// than the core prop. Scopes the backing probe only; it grants no behavior.
#[inline(always)]
fn uses_full_boss_backing(profile: &BossAmiiboPreviewProfile) -> bool {
    profile.key == "galeem" || profile.key == "dharkon"
}

/// Bounded presentation-backing probe. Reads the acquired object's state so
/// hardware can show whether the full boss object initializes on its own or
/// would need its manager-wait status. Read-only, debug-gated, and emitted
/// only at activation boundaries — never per frame.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
unsafe fn log_boss_backing_probe(
    state: &AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    presentation_boma: *mut BattleObjectModuleAccessor,
    acquired_kind: i32,
    requested_backing: &str,
    slot: i32,
    slot_still_held: bool,
    stage: &str,
) {
    if !crate::debug::enabled() || !uses_full_boss_backing(profile) || presentation_boma.is_null() {
        return;
    }
    crate::boss_log!(
        "[PB][AmiiboBossBackingProbe] generation={} logical_boss={} backing={} actual_kind={} stage={} initial_status={} initial_motion=0x{:x} frame={:.2} rate={:.3} scale={:.4} slot={} slot_still_held={} ownership={} presentation_object_id=0x{:x}",
        state.viewer_generation,
        profile.key,
        requested_backing,
        acquired_kind,
        stage,
        StatusModule::status_kind(presentation_boma),
        MotionModule::motion_kind(presentation_boma),
        MotionModule::frame(presentation_boma),
        MotionModule::rate(presentation_boma),
        ModelModule::scale(presentation_boma),
        slot,
        slot_still_held,
        state.ownership.name(),
        state.presentation_object_id,
    );
}

/// Activates a kind-verified item using the existing viewer presentation path.
/// A mismatched kind is never adopted: it is removed and the preview defers.
#[inline(always)]
unsafe fn activate_verified_item_presentation(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    host_id: u32,
    requested_item_kind: i32,
    requested_backing: &str,
    slot: i32,
    presentation_id: u32,
    presentation_boma: *mut BattleObjectModuleAccessor,
) -> bool {
    let acquired_kind = smash::app::utility::get_kind(&mut *presentation_boma);
    if acquired_kind != requested_item_kind {
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] presentation_create_failed generation={} logical_boss={} host_object_id=0x{:x} requested_kind={} acquired_kind={} reason=kind_mismatch action=restore_native_viewer",
                state.viewer_generation,
                profile.key,
                host_id,
                requested_item_kind,
                acquired_kind
            );
        }
        ItemModule::remove_item(module_accessor, slot);
        restore_viewer_host(module_accessor);
        state.host_hidden = false;
        state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
        state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
        return false;
    }

    state.presentation_object_id = presentation_id;
    state.created_frame_mark = state.frames_since_viewer_host_ready;
    state.expected_item_kind = requested_item_kind;
    state.presentation_slot = slot;
    state.ownership = AmiiboPreviewOwnership::HostSlot;
    state.ready_visual_logged = false;
    state.visual_ready_blocked_logged = false;
    state.visibility_reassertion_used = false;
    state.last_item_visible = None;
    state.last_item_model_visible = None;
    state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
    state.motion_probe_samples_remaining = MOTION_PROBE_SAMPLE_FRAMES;
    state.motion_probe_motion_before = 0;
    state.motion_probe_motion_after_change = 0;
    state.motion_probe_first_frame = -1.0;
    state.motion_probe_last_frame = -1.0;
    state.motion_probe_advanced = false;
    state.frames_since_viewer_host_ready = 0;
    state.frames_host_ready_to_create = 0;
    state.frames_create_to_engine_visible = 0;
    state.frames_create_to_model_visible = 0;
    state.have_item_ticks = 0;
    state.created_frame_mark = 0;
    state.visible_latency_logged = false;
    state.transform_comparison_samples_remaining = 0;
    state.transform_comparison_complete = false;
    state.interactive_transform_probe = InteractiveTransformProbe::empty();
    state.transform_calibration = TransformCalibrationState::empty();
    let native_rotation_after_create = presentation_rotation(presentation_boma);
    let presentation = apply_item_presentation(presentation_boma, state, profile);
    if state.motion_probe_motion_after_change == 0 {
        state.motion_probe_motion_before = presentation.motion_before;
        state.motion_probe_motion_after_change = presentation.motion_after_change;
        state.motion_probe_first_frame = presentation.frame_before;
    }
    state.transform_stabilization_frames_remaining = PREVIEW_TRANSFORM_STABILIZATION_FRAMES;
    let host_presentation = initialize_viewer_host_presentation_recipe(module_accessor, profile);
    state.host_hidden = true;
    initialize_presentation_visibility(state, profile, presentation_boma, module_accessor);
    state.phase = AmiiboPreviewPhase::Ready;
    // Same object after the plugin's scale + `"wait"` motion, with NO status
    // forced. If the object stays inert here while its native state looked
    // uninitialized, that is the evidence for a status dependency — report it
    // rather than forcing one, because a forced status previously detached
    // presentation items from the host slot and killed native rotation.
    log_boss_backing_probe(
        state,
        profile,
        presentation_boma,
        acquired_kind,
        requested_backing,
        slot,
        host_slot_for_presentation(module_accessor, presentation_id).is_some(),
        "after_wait_motion_no_status_forced",
    );
    let desired_position = desired_preview_position(state, profile);
    if crate::debug::enabled() {
        let host_root_rotation = host_presentation.root_rotation_set.unwrap_or([0.0; 3]);
        let host_position = viewer_host_position(module_accessor);
        let desired_presentation_rotation =
            desired_item_presentation_rotation(profile, presentation.native_rotation_after_motion);
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] presentation_created generation={} logical_boss={} host_object_id=0x{:x} presentation_object_id=0x{:x} slot={} requested_kind={} acquired_kind={} backing={} initial_host_position=({:.3},{:.3},{:.3}) current_host_position=({:.3},{:.3},{:.3}) viewer_anchor_position=({:.3},{:.3},{:.3}) desired_position=({:.3},{:.3},{:.3}) actual_position=({:.3},{:.3},{:.3}) native_rotation_after_create=({:.1},{:.1},{:.1}) native_rotation_after_motion=({:.1},{:.1},{:.1}) native_lr={:.3} presentation_rotation_override={} desired_presentation_rotation=({:.1},{:.1},{:.1}) final_presentation_rotation=({:.1},{:.1},{:.1}) desired_scale={:.4} actual_scale={:.4} stabilization_frames={} ownership=host_slot presentation_only=true",
            state.viewer_generation,
            profile.key,
            host_id,
            presentation_id,
            slot,
            requested_item_kind,
            acquired_kind,
            requested_backing,
            state.viewer_anchor.initial_position[0],
            state.viewer_anchor.initial_position[1],
            state.viewer_anchor.initial_position[2],
            host_position[0],
            host_position[1],
            host_position[2],
            state.viewer_anchor.position[0],
            state.viewer_anchor.position[1],
            state.viewer_anchor.position[2],
            desired_position.x,
            desired_position.y,
            desired_position.z,
            PostureModule::pos_x(presentation_boma),
            PostureModule::pos_y(presentation_boma),
            PostureModule::pos_z(presentation_boma),
            native_rotation_after_create[0],
            native_rotation_after_create[1],
            native_rotation_after_create[2],
            presentation.native_rotation_after_motion[0],
            presentation.native_rotation_after_motion[1],
            presentation.native_rotation_after_motion[2],
            presentation.native_lr_after_motion,
            presentation.desired_presentation_rotation.is_some(),
            desired_presentation_rotation[0],
            desired_presentation_rotation[1],
            desired_presentation_rotation[2],
            presentation.final_presentation_rotation[0],
            presentation.final_presentation_rotation[1],
            presentation.final_presentation_rotation[2],
            profile
                .preview_scale
                .unwrap_or_else(|| ModelModule::scale(presentation_boma)),
            ModelModule::scale(presentation_boma),
            state.transform_stabilization_frames_remaining,
        );
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] host_orientation_applied generation={} logical_boss={} boss_motion={} host_recipe={} host_posture_rotation={} host_root_rotation=({:.1},{:.1},{:.1}) host_rotation_before=({:.1},{:.1},{:.1}) host_rotation_after=({:.1},{:.1},{:.1}) host_scale={:.4} frozen_viewer_position=({:.3},{:.3},{:.3}) host_position=({:.3},{:.3},{:.3}) boss_rotation_after_create=({:.1},{:.1},{:.1}) boss_native_rotation_after_motion=({:.1},{:.1},{:.1}) presentation_rotation_override={} desired_presentation_rotation=({:.1},{:.1},{:.1}) final_presentation_rotation=({:.1},{:.1},{:.1}) boss_lr={:.3} boss_scale={:.4} boss_position=({:.3},{:.3},{:.3})",
            state.viewer_generation,
            profile.key,
            profile.idle_motion,
            host_presentation.recipe.name(),
            host_presentation.recipe.posture_rotation_name(),
            host_root_rotation[0],
            host_root_rotation[1],
            host_root_rotation[2],
            host_presentation.rotation_before[0],
            host_presentation.rotation_before[1],
            host_presentation.rotation_before[2],
            host_presentation.rotation_after[0],
            host_presentation.rotation_after[1],
            host_presentation.rotation_after[2],
            ModelModule::scale(module_accessor),
            state.viewer_anchor.position[0],
            state.viewer_anchor.position[1],
            state.viewer_anchor.position[2],
            host_position[0],
            host_position[1],
            host_position[2],
            native_rotation_after_create[0],
            native_rotation_after_create[1],
            native_rotation_after_create[2],
            presentation.native_rotation_after_motion[0],
            presentation.native_rotation_after_motion[1],
            presentation.native_rotation_after_motion[2],
            presentation.desired_presentation_rotation.is_some(),
            desired_presentation_rotation[0],
            desired_presentation_rotation[1],
            desired_presentation_rotation[2],
            presentation.final_presentation_rotation[0],
            presentation.final_presentation_rotation[1],
            presentation.final_presentation_rotation[2],
            presentation.native_lr_after_motion,
            ModelModule::scale(presentation_boma),
            PostureModule::pos_x(presentation_boma),
            PostureModule::pos_y(presentation_boma),
            PostureModule::pos_z(presentation_boma)
        );
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] motion_started generation={} presentation_object_id=0x{:x} motion={} motion_result=0x{:x} camera=native_viewer_host",
            state.viewer_generation,
            presentation_id,
            profile.idle_motion,
            presentation.motion
        );
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] presentation_active generation={} logical_boss={} host_hidden=true host_scale={:.4} host_engine_visible={} presentation_object_id=0x{:x}",
            state.viewer_generation,
            profile.key,
            ModelModule::scale(module_accessor),
            VisibilityModule::is_visible(module_accessor),
            presentation_id
        );
    }
    true
}

/// Lets the menu host settle before the first Rathalos request, then observes
/// that request and at most one WOL-prepared retry. No request occurs from a
/// regular Ready-frame path, so an unavailable item cannot become a spawn loop.
/// Once the boss backing exhausts that proven bound, exactly one menu-only
/// visual fallback request follows, and then the preview fails closed.
#[inline(always)]
unsafe fn poll_rathalos_presentation_acquisition(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    host_id: u32,
    requested_item_kind: i32,
    requested_backing: &str,
) -> bool {
    debug_assert_eq!(profile.key, RATHALOS_PREVIEW_KEY);

    if state.rathalos_acquire_probe.fallback_requested {
        return observe_rathalos_menu_visual_fallback(
            state,
            profile,
            module_accessor,
            host_id,
            requested_item_kind,
        );
    }

    if state.rathalos_acquire_probe.request_count == 0 {
        state.rathalos_acquire_probe.host_settle_frames = state
            .rathalos_acquire_probe
            .host_settle_frames
            .saturating_add(1);

        if !state.rathalos_acquire_probe.host_settled() {
            if state.rathalos_acquire_probe.host_settle_frames == 1 {
                log_rathalos_acquire_probe(
                    state,
                    profile,
                    module_accessor,
                    requested_item_kind,
                    "await_host_settle",
                    false,
                    "preserve_native_viewer_before_first_request",
                );
            }
            return true;
        }

        if host_has_foreign_item(module_accessor, 0) {
            return defer_rathalos_presentation(
                state,
                profile,
                module_accessor,
                requested_item_kind,
                "foreign_host_item_before_initial_request",
            );
        }

        log_rathalos_acquire_probe(
            state,
            profile,
            module_accessor,
            requested_item_kind,
            "host_settled",
            false,
            "issue_initial_wol_prepared_request",
        );
        let acquired = request_rathalos_presentation_item(
            state,
            profile,
            module_accessor,
            requested_item_kind,
            "settled_initial_wol_prepared_request",
        );
        if let Some((slot, presentation_id, presentation_boma)) = acquired {
            log_rathalos_acquire_probe(
                state,
                profile,
                module_accessor,
                requested_item_kind,
                "verified_on_initial_request",
                true,
                "activate_presentation",
            );
            return activate_verified_item_presentation(
                state,
                profile,
                module_accessor,
                host_id,
                requested_item_kind,
                requested_backing,
                slot,
                presentation_id,
                presentation_boma,
            );
        }

        state.host_hidden = true;
        return true;
    }

    if let Some((slot, presentation_id, presentation_boma)) =
        exact_kind_acquired_item(module_accessor, requested_item_kind)
    {
        log_rathalos_acquire_probe(
            state,
            profile,
            module_accessor,
            requested_item_kind,
            "verified_after_request",
            true,
            "activate_presentation",
        );
        return activate_verified_item_presentation(
            state,
            profile,
            module_accessor,
            host_id,
            requested_item_kind,
            requested_backing,
            slot,
            presentation_id,
            presentation_boma,
        );
    }

    state.rathalos_acquire_probe.frames_since_request = state
        .rathalos_acquire_probe
        .frames_since_request
        .saturating_add(1);
    log_rathalos_acquire_probe(
        state,
        profile,
        module_accessor,
        requested_item_kind,
        "observe_pending",
        false,
        "wait_for_native_item",
    );

    if !state.rathalos_acquire_probe.observation_window_elapsed() {
        return true;
    }

    if state.rathalos_acquire_probe.can_retry() {
        if host_has_foreign_item(module_accessor, 0) {
            return defer_rathalos_presentation(
                state,
                profile,
                module_accessor,
                requested_item_kind,
                "foreign_host_item_before_settled_retry",
            );
        }

        let acquired = request_rathalos_presentation_item(
            state,
            profile,
            module_accessor,
            requested_item_kind,
            "settled_host_retry",
        );
        if let Some((slot, presentation_id, presentation_boma)) = acquired {
            log_rathalos_acquire_probe(
                state,
                profile,
                module_accessor,
                requested_item_kind,
                "verified_on_settled_retry",
                true,
                "activate_presentation",
            );
            return activate_verified_item_presentation(
                state,
                profile,
                module_accessor,
                host_id,
                requested_item_kind,
                requested_backing,
                slot,
                presentation_id,
                presentation_boma,
            );
        }
        return true;
    }

    // Both bounded requests completed with every host slot empty. That
    // boundary is unavailable for the boss-only backing; raising the retry or
    // settle counts would observe nothing new, so move to the single
    // menu-only visual fallback instead.
    log_rathalos_acquire_probe(
        state,
        profile,
        module_accessor,
        requested_item_kind,
        "boss_backing_exhausted",
        false,
        "issue_single_menu_visual_fallback_request",
    );

    if host_has_foreign_item(module_accessor, 0) {
        return defer_rathalos_presentation(
            state,
            profile,
            module_accessor,
            requested_item_kind,
            "foreign_host_item_before_menu_visual_fallback",
        );
    }

    if let Some((slot, presentation_id, presentation_boma)) =
        request_rathalos_menu_visual_fallback(state, profile, module_accessor, requested_item_kind)
    {
        return activate_rathalos_menu_visual_fallback(
            state,
            profile,
            module_accessor,
            host_id,
            slot,
            presentation_id,
            presentation_boma,
        );
    }
    true
}

/// Observes the one menu-visual-fallback request for a single bounded window,
/// accepting it only on an exact `ITEM_KIND_LIOLEUS` match. No further
/// alternate kinds are ever tried.
#[inline(always)]
unsafe fn observe_rathalos_menu_visual_fallback(
    state: &mut AmiiboPreviewState,
    profile: &BossAmiiboPreviewProfile,
    module_accessor: *mut BattleObjectModuleAccessor,
    host_id: u32,
    primary_item_kind: i32,
) -> bool {
    if let Some((slot, presentation_id, presentation_boma)) =
        exact_kind_acquired_item(module_accessor, *ITEM_KIND_LIOLEUS)
    {
        log_rathalos_acquire_probe(
            state,
            profile,
            module_accessor,
            *ITEM_KIND_LIOLEUS,
            "menu_visual_fallback_verified",
            true,
            "activate_presentation",
        );
        return activate_rathalos_menu_visual_fallback(
            state,
            profile,
            module_accessor,
            host_id,
            slot,
            presentation_id,
            presentation_boma,
        );
    }

    state.rathalos_acquire_probe.frames_since_fallback = state
        .rathalos_acquire_probe
        .frames_since_fallback
        .saturating_add(1);
    if !state.rathalos_acquire_probe.fallback_window_elapsed() {
        return true;
    }

    defer_rathalos_presentation(
        state,
        profile,
        module_accessor,
        primary_item_kind,
        "menu_visual_fallback_unverified_after_one_bounded_request",
    )
}

#[inline(always)]
unsafe fn destroy_presentation_item(
    module_accessor: *mut BattleObjectModuleAccessor,
    reason: &str,
) {
    let state = &mut *preview_state_ptr();
    let presentation_id = state.presentation_object_id;
    let host_id = host_object_id(module_accessor);
    let expected_kind = state.expected_item_kind;
    let mut removal_requested = false;
    let mut actual_kind = -1;
    let mut removal_action = "not_active";

    if presentation_id != 0 && sv_battle_object::is_active(presentation_id) {
        let item_boma = sv_battle_object::module_accessor(presentation_id);
        if !item_boma.is_null() {
            actual_kind = smash::app::utility::get_kind(&mut *item_boma);
            if actual_kind == expected_kind {
                AttackModule::clear_all(item_boma);
                HitModule::set_whole(item_boma, smash::app::HitStatus(*HIT_STATUS_OFF), 0);
                VisibilityModule::set_whole(item_boma, false);

                if host_id != 0 && host_id == state.host_object_id {
                    if let Some((slot, _)) =
                        host_slot_for_presentation(module_accessor, presentation_id)
                    {
                        ItemModule::remove_item(module_accessor, slot);
                        removal_requested = true;
                        removal_action = "remove_host_slot";
                    } else {
                        remove_detached_presentation_item(item_boma);
                        removal_requested = true;
                        removal_action = "remove_detached_native_owned";
                    }
                } else {
                    remove_detached_presentation_item(item_boma);
                    removal_requested = true;
                    removal_action = "remove_detached_without_host";
                }
            } else {
                removal_action = "refuse_kind_mismatch";
            }
        } else {
            removal_action = "module_accessor_unavailable";
        }
    }

    if state.host_hidden && host_id != 0 && host_id == state.host_object_id {
        restore_viewer_host(module_accessor);
    }

    if crate::debug::enabled()
        && (presentation_id != 0 || state.phase != AmiiboPreviewPhase::Inactive)
    {
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] presentation_destroyed generation={} logical_ui_hash=0x{:010x} host_object_id=0x{:x} presentation_object_id=0x{:x} ownership={} expected_kind={} actual_kind={} removal_requested={} action={} reason={}",
            state.viewer_generation,
            state.logical_ui_hash,
            state.host_object_id,
            presentation_id,
            state.ownership.name(),
            expected_kind,
            actual_kind,
            removal_requested,
            removal_action,
            reason
        );
    }

    state.presentation_object_id = 0;
    state.expected_item_kind = -1;
    state.presentation_slot = -1;
    state.ownership = AmiiboPreviewOwnership::None;
    state.host_object_id = 0;
    state.viewer_anchor = ViewerAnchor::empty();
    state.host_hidden = false;
    state.transform_stabilization_frames_remaining = 0;
    state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
    state.motion_probe_samples_remaining = MOTION_PROBE_SAMPLE_FRAMES;
    state.motion_probe_motion_before = 0;
    state.motion_probe_motion_after_change = 0;
    state.motion_probe_first_frame = -1.0;
    state.motion_probe_last_frame = -1.0;
    state.motion_probe_advanced = false;
    state.frames_since_viewer_host_ready = 0;
    state.frames_host_ready_to_create = 0;
    state.frames_create_to_engine_visible = 0;
    state.frames_create_to_model_visible = 0;
    state.have_item_ticks = 0;
    state.created_frame_mark = 0;
    state.visible_latency_logged = false;
    state.transform_comparison_samples_remaining = 0;
    state.transform_comparison_complete = false;
    state.interactive_transform_probe = InteractiveTransformProbe::empty();
    state.transform_calibration = TransformCalibrationState::empty();
    state.transform_ready_logged = false;
    state.native_transform_reset_logged = false;
    state.ready_visual_logged = false;
    state.visual_ready_blocked_logged = false;
    state.visibility_reassertion_used = false;
    state.last_item_visible = None;
    state.last_item_model_visible = None;
}

#[inline(always)]
unsafe fn defer_current_viewer_presentation(
    module_accessor: *mut BattleObjectModuleAccessor,
    reason: &str,
) {
    destroy_presentation_item(module_accessor, reason);
    let state = &mut *preview_state_ptr();
    // Keep the identity handoff until the native viewer exits. A reclaimed
    // presentation object is allowed one explicit stabilization retry before
    // this fail-closed state is entered.
    state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
    state.create_attempted = true;
}

#[inline(always)]
unsafe fn reset_preview_state(module_accessor: *mut BattleObjectModuleAccessor, reason: &str) {
    destroy_presentation_item(module_accessor, reason);
    let state = &mut *preview_state_ptr();
    state.phase = AmiiboPreviewPhase::Inactive;
    state.logical_ui_hash = 0;
    state.identity_source = AmiiboPreviewIdentitySource::None;
    state.create_attempted = false;
    state.stabilization_reacquire_used = false;
    state.apply_profile_idle_motion = true;
    state.ready_visual_logged = false;
    state.ignored_lookup_mask = 0;
}

/// Runs only from Mario's existing frame callback.  A true return means this
/// is a verified boss Figure Player viewer host and the normal battle
/// dispatcher must not run for that frame.
pub unsafe fn frame(module_accessor: *mut BattleObjectModuleAccessor, stage_id: i32) -> bool {
    let state = &mut *preview_state_ptr();

    if stage_id != crate::boss_helpers::STAGE_ID_AMIIBO_PREVIEW {
        if state.phase != AmiiboPreviewPhase::Inactive {
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][AmiiboPreviewRuntime] viewer_exit generation={} phase={} stage=0x{:x}",
                    state.viewer_generation,
                    state.phase.name(),
                    stage_id
                );
            }
            reset_preview_state(module_accessor, "viewer_exit");
        }
        return false;
    }

    if state.phase == AmiiboPreviewPhase::Inactive {
        return false;
    }
    if module_accessor.is_null()
        || smash::app::utility::get_kind(&mut *module_accessor) != *FIGHTER_KIND_MARIO
    {
        return false;
    }

    let host_id = host_object_id(module_accessor);
    if host_id == 0 || !sv_battle_object::is_active(host_id) {
        return false;
    }
    // Counts only frames where a live viewer host already exists, so it
    // measures this plugin's contribution to the spawn delay and excludes the
    // game's own fighter NRO load.
    state.frames_since_viewer_host_ready = state.frames_since_viewer_host_ready.saturating_add(1);

    let Some(profile) = profile_for_ui_chara_hash(state.logical_ui_hash) else {
        reset_preview_state(module_accessor, "logical_profile_missing");
        return false;
    };

    let Some(backing) = verified_presentation_backing(profile) else {
        if state.phase != AmiiboPreviewPhase::DeferredUntilSupported {
            state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][AmiiboPreviewRuntime] viewer_enter generation={} logical_boss={} stage=0x{:x} action=deferred_missing_verified_backing",
                    state.viewer_generation,
                    profile.key,
                    stage_id
                );
            }
        }
        return false;
    };

    let item_backing = match backing {
        VerifiedPreviewBacking::Item { kind, source } => Some((kind, source)),
        VerifiedPreviewBacking::NativeFighter { kind, source } => {
            if state.phase != AmiiboPreviewPhase::DeferredUntilSupported {
                state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
                if crate::debug::enabled() {
                    crate::boss_log!(
                        "[PB][AmiiboPreviewRuntime] viewer_enter generation={} logical_boss={} stage=0x{:x} action=native_fighter_viewer_owned_no_plugin_presentation backing={} fighter_kind={}",
                        state.viewer_generation,
                        profile.key,
                        stage_id,
                        source,
                        kind
                    );
                }
            }
            None
        }
    };
    let Some((requested_item_kind, requested_backing)) = item_backing else {
        return false;
    };

    if !profile_rollout_enabled(profile) {
        if state.phase != AmiiboPreviewPhase::DeferredUntilSupported {
            state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
            if crate::debug::enabled() {
                crate::boss_log!(
                    "[PB][AmiiboPreviewRuntime] viewer_enter generation={} logical_boss={} stage=0x{:x} action=deferred_hardware_rollout backing={} requested_kind={}",
                    state.viewer_generation,
                    profile.key,
                    stage_id,
                    requested_backing,
                    requested_item_kind
                );
            }
        }
        return false;
    }

    if state.host_object_id != 0 && state.host_object_id != host_id {
        destroy_presentation_item(module_accessor, "viewer_host_replaced");
        state.stabilization_reacquire_used = false;
        state.phase = AmiiboPreviewPhase::IdentityCaptured;
    }

    if state.phase == AmiiboPreviewPhase::IdentityCaptured {
        if state.presentation_object_id != 0 {
            // A new direct scan can arrive before the old host is replaced.
            // Tear down only the presentation item we own; the native viewer
            // host remains intact and will be rebound below.
            destroy_presentation_item(module_accessor, "scan_identity_replaced");
        }
        // Root-only previews intentionally preserve native host posture. Reset
        // any posture/root state left by Galeem, Dharkon, or Marx before this
        // generation captures its stage-0x135 anchor.
        if state.host_hidden {
            restore_viewer_host(module_accessor);
            state.host_hidden = false;
        }
        state.viewer_generation = state.viewer_generation.wrapping_add(1);
        state.host_object_id = host_id;
        state.viewer_host_ready_tick = system_tick();
        let current_host_position = viewer_host_position(module_accessor);
        let current_host_rotation = viewer_host_rotation(module_accessor);
        let anchor_captured = capture_viewer_anchor(state, module_accessor);
        state.expected_item_kind = -1;
        state.presentation_slot = -1;
        state.ownership = AmiiboPreviewOwnership::None;
        state.transform_stabilization_frames_remaining = 0;
        state.apply_profile_idle_motion = true;
        state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
        state.motion_probe_samples_remaining = MOTION_PROBE_SAMPLE_FRAMES;
        state.motion_probe_motion_before = 0;
        state.motion_probe_motion_after_change = 0;
        state.motion_probe_first_frame = -1.0;
        state.motion_probe_last_frame = -1.0;
        state.motion_probe_advanced = false;
        state.frames_since_viewer_host_ready = 0;
        state.frames_host_ready_to_create = 0;
        state.frames_create_to_engine_visible = 0;
        state.frames_create_to_model_visible = 0;
        state.have_item_ticks = 0;
        state.created_frame_mark = 0;
        state.visible_latency_logged = false;
        state.transform_comparison_samples_remaining = 0;
        state.transform_comparison_complete = false;
        state.interactive_transform_probe = InteractiveTransformProbe::empty();
        state.transform_calibration = TransformCalibrationState::empty();
        state.transform_ready_logged = false;
        state.native_transform_reset_logged = false;
        state.ready_visual_logged = false;
        state.visual_ready_blocked_logged = false;
        state.visibility_reassertion_used = false;
        state.last_item_visible = None;
        state.last_item_model_visible = None;
        state.phase = AmiiboPreviewPhase::WaitingForViewerHost;
        let desired_position = desired_preview_position(state, profile);
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] viewer_host_ready generation={} logical_boss={} stage=0x{:x} host_object_id=0x{:x} host_kind={} identity_source={} scan_identity=unobserved_ui_lookup anchor_captured={} initial_host_position=({:.3},{:.3},{:.3}) initial_host_rotation=({:.1},{:.1},{:.1}) current_host_position=({:.3},{:.3},{:.3}) current_host_rotation=({:.1},{:.1},{:.1}) viewer_anchor_position=({:.3},{:.3},{:.3}) viewer_anchor_lr={:.3} viewer_anchor_rotation=({:.1},{:.1},{:.1}) requested_offset=({:.3},{:.3},{:.3}) desired_position=({:.3},{:.3},{:.3}) camera=native_viewer_host",
                state.viewer_generation,
                profile.key,
                stage_id,
                host_id,
                smash::app::utility::get_kind(&mut *module_accessor),
                state.identity_source.name(),
                anchor_captured,
                state.viewer_anchor.initial_position[0],
                state.viewer_anchor.initial_position[1],
                state.viewer_anchor.initial_position[2],
                state.viewer_anchor.initial_rotation[0],
                state.viewer_anchor.initial_rotation[1],
                state.viewer_anchor.initial_rotation[2],
                current_host_position[0],
                current_host_position[1],
                current_host_position[2],
                current_host_rotation[0],
                current_host_rotation[1],
                current_host_rotation[2],
                state.viewer_anchor.position[0],
                state.viewer_anchor.position[1],
                state.viewer_anchor.position[2],
                state.viewer_anchor.lr,
                state.viewer_anchor.rotation[0],
                state.viewer_anchor.rotation[1],
                state.viewer_anchor.rotation[2],
                profile.position_offset[0],
                profile.position_offset[1],
                profile.position_offset[2],
                desired_position.x,
                desired_position.y,
                desired_position.z,
            );
        }
    }

    if state.phase == AmiiboPreviewPhase::AwaitingRathalosAcquire {
        return poll_rathalos_presentation_acquisition(
            state,
            profile,
            module_accessor,
            host_id,
            requested_item_kind,
            requested_backing,
        );
    }

    if state.phase == AmiiboPreviewPhase::Ready {
        let presentation_id = state.presentation_object_id;
        let object_active = presentation_id != 0 && sv_battle_object::is_active(presentation_id);
        let presentation_boma = if object_active {
            sv_battle_object::module_accessor(presentation_id)
        } else {
            std::ptr::null_mut()
        };
        let actual_kind = if presentation_boma.is_null() {
            -1
        } else {
            smash::app::utility::get_kind(&mut *presentation_boma)
        };

        if object_active && !presentation_boma.is_null() && actual_kind == state.expected_item_kind
        {
            let (slot_still_held, slot, current_slot_item_id) =
                match host_slot_for_presentation(module_accessor, presentation_id) {
                    Some((slot, item_id)) => (true, slot, item_id),
                    None => (false, -1, 0),
                };
            let new_ownership = if slot_still_held {
                AmiiboPreviewOwnership::HostSlot
            } else {
                AmiiboPreviewOwnership::DetachedNativeOwned
            };
            if new_ownership != state.ownership {
                if crate::debug::enabled() {
                    crate::boss_log!(
                        "[PB][AmiiboPreviewRuntime] ownership_transition generation={} logical_boss={} presentation_object_id=0x{:x} slot_still_held={} object_active=true expected_kind={} actual_kind={} old_ownership={} new_ownership={}",
                        state.viewer_generation,
                        profile.key,
                        presentation_id,
                        slot_still_held,
                        state.expected_item_kind,
                        actual_kind,
                        state.ownership.name(),
                        new_ownership.name()
                    );
                }
                state.ownership = new_ownership;
            }
            state.presentation_slot = slot;

            let desired_position = desired_preview_position(state, profile);
            // `unwrap_or_else`: every item profile carries an explicit scale,
            // so the eager `unwrap_or` argument was an FFI read discarded on
            // every frame. Same value, only queried when there is no profile
            // scale to use.
            let desired_scale = profile
                .preview_scale
                .unwrap_or_else(|| ModelModule::scale(presentation_boma));

            // One-shot native-transform-reset observation. The whole snapshot
            // (host position, item position/rotation/scale) exists only to
            // detect and report that first reset, and the latch below fires at
            // most once per viewer generation — so it is taken inside the latch
            // rather than on every Ready frame. Behavior is unchanged: nothing
            // outside this block reads these values.
            if !state.native_transform_reset_logged {
                let before_position = Vector3f {
                    x: PostureModule::pos_x(presentation_boma),
                    y: PostureModule::pos_y(presentation_boma),
                    z: PostureModule::pos_z(presentation_boma),
                };
                let before_rotation = presentation_rotation(presentation_boma);
                let before_scale = ModelModule::scale(presentation_boma);
                let rotation_needs_stabilization = profile
                    .presentation_rotation
                    .map(|rotation| !rotation_matches(before_rotation, rotation))
                    .unwrap_or(false);
                if !transform_matches(before_position.x, desired_position.x)
                    || !transform_matches(before_position.y, desired_position.y)
                    || !transform_matches(before_position.z, desired_position.z)
                    || !transform_matches(before_scale, desired_scale)
                    || rotation_needs_stabilization
                {
                    state.native_transform_reset_logged = true;
                    if crate::debug::enabled() {
                        let current_host_position = viewer_host_position(module_accessor);
                        let desired_presentation_rotation =
                            desired_item_presentation_rotation(profile, before_rotation);
                        crate::boss_log!(
                        "[PB][AmiiboPreviewRuntime] native_transform_reset_observed generation={} logical_boss={} presentation_object_id=0x{:x} initial_host_position=({:.3},{:.3},{:.3}) current_host_position=({:.3},{:.3},{:.3}) viewer_anchor_position=({:.3},{:.3},{:.3}) desired_position=({:.3},{:.3},{:.3}) observed_position=({:.3},{:.3},{:.3}) observed_presentation_rotation=({:.1},{:.1},{:.1}) presentation_rotation_override={} desired_presentation_rotation=({:.1},{:.1},{:.1}) desired_scale={:.4} observed_scale={:.4} stabilization_frames_remaining={}",
                        state.viewer_generation,
                        profile.key,
                        presentation_id,
                        state.viewer_anchor.initial_position[0],
                        state.viewer_anchor.initial_position[1],
                        state.viewer_anchor.initial_position[2],
                        current_host_position[0],
                        current_host_position[1],
                        current_host_position[2],
                        state.viewer_anchor.position[0],
                        state.viewer_anchor.position[1],
                        state.viewer_anchor.position[2],
                        desired_position.x,
                        desired_position.y,
                        desired_position.z,
                        before_position.x,
                        before_position.y,
                        before_position.z,
                        before_rotation[0],
                        before_rotation[1],
                        before_rotation[2],
                        profile.presentation_rotation.is_some(),
                        desired_presentation_rotation[0],
                        desired_presentation_rotation[1],
                        desired_presentation_rotation[2],
                        desired_scale,
                        before_scale,
                        state.transform_stabilization_frames_remaining
                    );
                    }
                }
            }

            observe_presentation_visibility(
                state,
                profile,
                presentation_boma,
                module_accessor,
                "before_presentation_maintenance",
            );
            let within_transform_stabilization = state.transform_stabilization_frames_remaining > 0;
            let (motion, stable_item) = if within_transform_stabilization {
                let presentation = apply_item_presentation(presentation_boma, state, profile);
                if state.motion_probe_motion_after_change == 0 {
                    state.motion_probe_motion_before = presentation.motion_before;
                    state.motion_probe_motion_after_change = presentation.motion_after_change;
                    state.motion_probe_first_frame = presentation.frame_before;
                }
                state.transform_stabilization_frames_remaining -= 1;
                (presentation.motion, StableItemMaintenanceResult::empty())
            } else {
                // Reaching stable maintenance means this presentation was
                // fully built, so a later loss is a real teardown (entering
                // Level Up / Smash / Send on a Journey and coming back), not
                // the native viewer's construction race. Refresh the one-shot
                // reacquire budget so the boss rebuilds instead of being
                // permanently replaced by Mario. A construction race can never
                // reach this branch, so the anti-spawn-loop bound still holds.
                state.stabilization_reacquire_used = false;
                // Stable maintenance keeps the item inert and additionally
                // owns the boss-local channels (framing position, item-root
                // static correction, calibration overrides) without touching
                // any native host channel.
                process_transform_calibration_input(
                    state,
                    profile,
                    module_accessor,
                    presentation_boma,
                    slot_still_held,
                );
                let stable_item =
                    maintain_stable_item_presentation(state, profile, presentation_boma);
                (stable_item.motion, stable_item)
            };
            observe_presentation_visibility(
                state,
                profile,
                presentation_boma,
                module_accessor,
                "after_item_presentation",
            );
            // No recipe writes the host PostureModule, so the native Amiibo
            // viewer keeps interactive stick rotation, and the NativeHost
            // recipe writes no host channel at all; a host-root calibration
            // override is the debug harness's only host write.
            let mut host_presentation = if within_transform_stabilization {
                initialize_viewer_host_presentation_recipe(module_accessor, profile)
            } else {
                maintain_viewer_host_presentation(module_accessor, profile)
            };
            if !within_transform_stabilization {
                if let Some(root_override) = state
                    .transform_calibration
                    .override_for(CalibrationTarget::HostRoot)
                {
                    set_root_joint_rotation(module_accessor, root_override);
                    host_presentation.root_rotation_set = Some(root_override);
                    host_presentation.root_written = true;
                    host_presentation.root_rotation_observed = root_joint_rotation(module_accessor);
                }
                observe_interactive_transform_probe(
                    state,
                    profile,
                    presentation_boma,
                    module_accessor,
                    presentation_id,
                    slot_still_held,
                    &host_presentation,
                    &stable_item,
                );
            }
            state.host_hidden = true;
            observe_presentation_visibility(
                state,
                profile,
                presentation_boma,
                module_accessor,
                "after_host_recipe",
            );
            let (item_visible, item_model_visible) = ensure_presentation_visibility_once(
                state,
                profile,
                presentation_boma,
                module_accessor,
            );

            // Full latency breakdown across the user-perceived path.
            // identity -> viewer_host_ready is wall time (the native viewer
            // builds its own Mario host there, which this plugin cannot
            // start); everything after is frame-counted.
            if !state.visible_latency_logged && item_visible && item_model_visible {
                state.visible_latency_logged = true;
                if state.frames_create_to_engine_visible == 0 {
                    state.frames_create_to_engine_visible = state
                        .frames_since_viewer_host_ready
                        .saturating_sub(state.created_frame_mark);
                }
                state.frames_create_to_model_visible = state
                    .frames_since_viewer_host_ready
                    .saturating_sub(state.created_frame_mark);
                if crate::debug::enabled() {
                    let identity_to_host_ticks = state
                        .viewer_host_ready_tick
                        .saturating_sub(state.identity_tick);
                    let identity_to_visible_ticks =
                        system_tick().saturating_sub(state.identity_tick);
                    crate::boss_log!(
                        "[PB][AmiiboPreviewLatency] generation={} logical_boss={} identity_to_host_ms={:.1} identity_to_host_ticks={} host_to_create_frames={} have_item_ticks={} have_item_ms={:.3} create_to_engine_visible_frames={} create_to_model_visible_frames={} host_to_visible_frames={} host_to_visible_ms_approx={:.1} total_identity_to_visible_ms={:.1} tick_hz={:.0} presentation_object_id=0x{:x}",
                        state.viewer_generation,
                        profile.key,
                        ticks_to_ms(identity_to_host_ticks),
                        identity_to_host_ticks,
                        state.frames_host_ready_to_create,
                        state.have_item_ticks,
                        ticks_to_ms(state.have_item_ticks),
                        state.frames_create_to_engine_visible,
                        state.frames_create_to_model_visible,
                        state.frames_since_viewer_host_ready,
                        state.frames_since_viewer_host_ready as f32 / 60.0 * 1000.0,
                        ticks_to_ms(identity_to_visible_ticks),
                        SYSTEM_TICK_HZ,
                        presentation_id,
                    );
                }
            }
            if state.frames_create_to_engine_visible == 0 && item_visible {
                state.frames_create_to_engine_visible = state
                    .frames_since_viewer_host_ready
                    .saturating_sub(state.created_frame_mark);
            }

            // Motion-playback probe. Evidence only — no field here alone is
            // a verdict. `name_accepted` says whether the engine took the
            // requested motion; `animation_advanced` says whether anything is
            // actually playing. A stuck frame with an accepted name is NOT
            // proof of a static model: it can equally be a wrong animation
            // resource, a zero-rate motion, a one-frame motion, missing
            // initialization, or a core object with no animated idle.
            if state.motion_probe_samples_remaining > 0 {
                state.motion_probe_samples_remaining -= 1;
                let motion_frame = MotionModule::frame(presentation_boma);
                let motion_kind = MotionModule::motion_kind(presentation_boma);
                if state.motion_probe_last_frame >= 0.0
                    && (motion_frame - state.motion_probe_last_frame).abs() > 0.0001
                {
                    state.motion_probe_advanced = true;
                }
                state.motion_probe_last_frame = motion_frame;
                if state.motion_probe_samples_remaining == 0 && crate::debug::enabled() {
                    crate::boss_log!(
                        "[PB][AmiiboMotionProbe] surface=amiibo generation={} logical_boss={} actual_item_kind={} requested_motion_name={} requested_motion_hash=0x{:x} motion_before=0x{:x} motion_immediately_after_change=0x{:x} motion_after_sample_window=0x{:x} name_accepted={} frame_before={:.2} frame_after={:.2} motion_rate={:.3} status_kind={} animation_advanced={} sample_frames={} presentation_object_id=0x{:x}",
                        state.viewer_generation,
                        profile.key,
                        actual_kind,
                        profile.idle_motion,
                        smash::hash40(profile.idle_motion),
                        state.motion_probe_motion_before,
                        state.motion_probe_motion_after_change,
                        motion_kind,
                        motion_kind == smash::hash40(profile.idle_motion),
                        state.motion_probe_first_frame,
                        motion_frame,
                        MotionModule::rate(presentation_boma),
                        StatusModule::status_kind(presentation_boma),
                        state.motion_probe_advanced,
                        MOTION_PROBE_SAMPLE_FRAMES,
                        presentation_id,
                    );
                }
            }
            let actual_position = Vector3f {
                x: PostureModule::pos_x(presentation_boma),
                y: PostureModule::pos_y(presentation_boma),
                z: PostureModule::pos_z(presentation_boma),
            };
            let actual_rotation = presentation_rotation(presentation_boma);
            let desired_presentation_rotation =
                desired_item_presentation_rotation(profile, actual_rotation);
            let actual_scale = ModelModule::scale(presentation_boma);
            let status = StatusModule::status_kind(presentation_boma);
            let transform_window_complete = state.transform_stabilization_frames_remaining == 0;
            if transform_window_complete {
                observe_transform_comparison(
                    state,
                    profile,
                    presentation_boma,
                    module_accessor,
                    presentation_id,
                    slot_still_held,
                    host_presentation,
                );
            }
            if transform_window_complete && !state.transform_ready_logged {
                state.transform_ready_logged = true;
                if crate::debug::enabled() {
                    let current_host_position = viewer_host_position(module_accessor);
                    crate::boss_log!(
                        "[PB][AmiiboPreviewRuntime] transform_ready generation={} logical_boss={} presentation_object_id=0x{:x} initial_host_position=({:.3},{:.3},{:.3}) current_host_position=({:.3},{:.3},{:.3}) viewer_anchor_position=({:.3},{:.3},{:.3}) desired_position=({:.3},{:.3},{:.3}) actual_position=({:.3},{:.3},{:.3}) presentation_rotation_override={} desired_presentation_rotation=({:.1},{:.1},{:.1}) final_presentation_rotation=({:.1},{:.1},{:.1}) desired_scale={:.4} actual_scale={:.4} motion=0x{:x} status={} ownership={} slot_still_held={}",
                        state.viewer_generation,
                        profile.key,
                        presentation_id,
                        state.viewer_anchor.initial_position[0],
                        state.viewer_anchor.initial_position[1],
                        state.viewer_anchor.initial_position[2],
                        current_host_position[0],
                        current_host_position[1],
                        current_host_position[2],
                        state.viewer_anchor.position[0],
                        state.viewer_anchor.position[1],
                        state.viewer_anchor.position[2],
                        desired_position.x,
                        desired_position.y,
                        desired_position.z,
                        actual_position.x,
                        actual_position.y,
                        actual_position.z,
                        profile.presentation_rotation.is_some(),
                        desired_presentation_rotation[0],
                        desired_presentation_rotation[1],
                        desired_presentation_rotation[2],
                        actual_rotation[0],
                        actual_rotation[1],
                        actual_rotation[2],
                        desired_scale,
                        actual_scale,
                        motion,
                        status,
                        state.ownership.name(),
                        slot_still_held
                    );
                }
            }
            if transform_window_complete
                && item_visible
                && item_model_visible
                && !state.ready_visual_logged
            {
                let item_lr = PostureModule::lr(presentation_boma);
                let host_visible = VisibilityModule::is_visible(module_accessor);
                let host_model_visible = ModelModule::is_visible(module_accessor);
                if crate::debug::enabled() {
                    let current_host_position = viewer_host_position(module_accessor);
                    crate::boss_log!(
                        "[PB][AmiiboPreviewRuntime] stable_presentation generation={} logical_boss={} presentation_object_id=0x{:x} active=true expected_kind={} actual_kind={} item_visible={} item_model_visible={} initial_host_position=({:.3},{:.3},{:.3}) current_host_position=({:.3},{:.3},{:.3}) viewer_anchor_position=({:.3},{:.3},{:.3}) boss_position=({:.3},{:.3},{:.3}) desired_position=({:.3},{:.3},{:.3}) scale={:.4} desired_scale={:.4} lr={:.3} presentation_rotation_override={} desired_presentation_rotation=({:.1},{:.1},{:.1}) final_presentation_rotation=({:.1},{:.1},{:.1}) status={} motion=0x{:x} host_hidden={} host_scale={:.4} host_engine_visible={} host_model_visible={} ownership={} slot_still_held={} current_slot_item_id=0x{:x}",
                        state.viewer_generation,
                        profile.key,
                        presentation_id,
                        state.expected_item_kind,
                        actual_kind,
                        item_visible,
                        item_model_visible,
                        state.viewer_anchor.initial_position[0],
                        state.viewer_anchor.initial_position[1],
                        state.viewer_anchor.initial_position[2],
                        current_host_position[0],
                        current_host_position[1],
                        current_host_position[2],
                        state.viewer_anchor.position[0],
                        state.viewer_anchor.position[1],
                        state.viewer_anchor.position[2],
                        actual_position.x,
                        actual_position.y,
                        actual_position.z,
                        desired_position.x,
                        desired_position.y,
                        desired_position.z,
                        actual_scale,
                        desired_scale,
                        item_lr,
                        profile.presentation_rotation.is_some(),
                        desired_presentation_rotation[0],
                        desired_presentation_rotation[1],
                        desired_presentation_rotation[2],
                        actual_rotation[0],
                        actual_rotation[1],
                        actual_rotation[2],
                        status,
                        motion,
                        state.host_hidden,
                        ModelModule::scale(module_accessor),
                        host_visible,
                        host_model_visible,
                        state.ownership.name(),
                        slot_still_held,
                        current_slot_item_id
                    );
                }
                state.ready_visual_logged = true;
            } else if (!item_visible || !item_model_visible) && !state.visual_ready_blocked_logged {
                state.visual_ready_blocked_logged = true;
                if crate::debug::enabled() {
                    crate::boss_log!(
                        "[PB][AmiiboPreviewRuntime] presentation_not_visually_ready generation={} logical_boss={} presentation_object_id=0x{:x} item_visible={} item_model_visible={} host_engine_visible={} host_scale={:.4} action=preserve_bounded_presentation_for_visibility_trace",
                        state.viewer_generation,
                        profile.key,
                        presentation_id,
                        item_visible,
                        item_model_visible,
                        VisibilityModule::is_visible(module_accessor),
                        ModelModule::scale(module_accessor),
                    );
                }
            }
            return true;
        }

        // A viewer object is lost only when its own battle object becomes
        // inactive, inaccessible, or changes kind. A cleared host slot alone
        // is a normal detached-item transition and is handled above.
        let native_reclaim = !object_active || presentation_boma.is_null();
        if native_reclaim && !state.stabilization_reacquire_used {
            if crate::debug::enabled() {
                let current_host_position = viewer_host_position(module_accessor);
                crate::boss_log!(
                    "[PB][AmiiboPreviewRuntime] presentation_reacquire_scheduled generation={} logical_boss={} host_object_id=0x{:x} presentation_object_id=0x{:x} expected_kind={} object_active={} ownership={} attempt=1 initial_host_position=({:.3},{:.3},{:.3}) current_host_position=({:.3},{:.3},{:.3}) viewer_anchor_position=({:.3},{:.3},{:.3}) reason=native_viewer_initialization_reclaim",
                    state.viewer_generation,
                    profile.key,
                    state.host_object_id,
                    presentation_id,
                    state.expected_item_kind,
                    object_active,
                    state.ownership.name(),
                    state.viewer_anchor.initial_position[0],
                    state.viewer_anchor.initial_position[1],
                    state.viewer_anchor.initial_position[2],
                    current_host_position[0],
                    current_host_position[1],
                    current_host_position[2],
                    state.viewer_anchor.position[0],
                    state.viewer_anchor.position[1],
                    state.viewer_anchor.position[2],
                );
            }
            state.stabilization_reacquire_used = true;
            let frozen_viewer_anchor = state.viewer_anchor;
            destroy_presentation_item(
                module_accessor,
                "native_viewer_initialization_reclaim_reacquire_once",
            );
            let state = &mut *preview_state_ptr();
            state.viewer_anchor = frozen_viewer_anchor;
            state.phase = AmiiboPreviewPhase::IdentityCaptured;
            state.create_attempted = false;
            return true;
        }

        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] presentation_lost generation={} logical_boss={} host_object_id=0x{:x} presentation_object_id=0x{:x} object_active={} expected_kind={} actual_kind={} ownership={} stabilization_reacquire_used={} action=restore_native_viewer_no_more_reacquire",
                state.viewer_generation,
                profile.key,
                state.host_object_id,
                presentation_id,
                object_active,
                state.expected_item_kind,
                actual_kind,
                state.ownership.name(),
                state.stabilization_reacquire_used
            );
        }
        defer_current_viewer_presentation(
            module_accessor,
            "presentation_object_inactive_or_kind_mismatch",
        );
        return false;
    }

    if state.phase != AmiiboPreviewPhase::WaitingForViewerHost {
        return false;
    }

    if host_has_foreign_item(module_accessor, 0) {
        state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] presentation_create_blocked generation={} logical_boss={} host_object_id=0x{:x} requested_kind={} reason=viewer_host_has_foreign_item action=preserve_native_viewer",
                state.viewer_generation,
                profile.key,
                host_id,
                requested_item_kind
            );
        }
        return false;
    }

    state.phase = AmiiboPreviewPhase::CreatingPresentation;
    state.create_attempted = true;
    state.frames_host_ready_to_create = state.frames_since_viewer_host_ready;
    // Dracula is unconditionally fail-closed. Hardware crashed inside
    // `ItemModule::have_item` on stage 0x135 for every backing and every
    // preparation tried:
    //   ITEM_KIND_DRACULA  immediate / WOL-prepared / tiny-host / 16-frame settle
    //   ITEM_KIND_DRACULA2 immediate / WOL-prepared / tiny-host / 16-frame settle
    // The last run reached `phase=host_settled host_settle_frames=16
    // request_issued=true requested_kind=373` and never returned. The problem
    // is therefore neither timing nor which Dracula kind is passed, so no
    // further `have_item` experiment may be run from this stage and there is
    // deliberately no configuration option that can re-arm one. Fail closed
    // before ANY host mutation: no slot clearing, no host shrink, no
    // `have_item`, no retry. The native viewer is preserved untouched.
    //
    // His presentation profile below is kept intact so that a future safe
    // Dracula BattleObject factory needs no transform work.
    if profile.key == DRACULA_PREVIEW_KEY {
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboDraculaAcquire] generation={} logical_boss={} phase=blocked_all_known_have_item_paths_crash stage=0x{:x} host_object_id=0x{:x} phase1_kind={} phase2_kind={} action=preserve_native_viewer_no_have_item",
                state.viewer_generation,
                profile.key,
                stage_id,
                host_id,
                *ITEM_KIND_DRACULA,
                *ITEM_KIND_DRACULA2,
            );
        }
        state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
        return false;
    }
    if crate::debug::enabled() {
        let current_host_position = viewer_host_position(module_accessor);
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] presentation_create_begin generation={} logical_boss={} stage=0x{:x} host_object_id=0x{:x} requested_kind={} backing={} motion={} scale={:?} acquisition_recipe={} initial_host_position=({:.3},{:.3},{:.3}) current_host_position=({:.3},{:.3},{:.3}) viewer_anchor_position=({:.3},{:.3},{:.3}) anchor_offset=({:.2},{:.2},{:.2}) host_recipe={} camera=native_viewer_host",
            state.viewer_generation,
            profile.key,
            stage_id,
            host_id,
            requested_item_kind,
            requested_backing,
            profile.idle_motion,
            profile.preview_scale,
            profile.item_acquire_recipe.name(),
            state.viewer_anchor.initial_position[0],
            state.viewer_anchor.initial_position[1],
            state.viewer_anchor.initial_position[2],
            current_host_position[0],
            current_host_position[1],
            current_host_position[2],
            state.viewer_anchor.position[0],
            state.viewer_anchor.position[1],
            state.viewer_anchor.position[2],
            profile.position_offset[0],
            profile.position_offset[1],
            profile.position_offset[2],
            profile.host_orientation_recipe.name()
        );
    }

    if profile.item_acquire_recipe.uses_deferred_observation() {
        // Both deferred recipes keep the native Mario viewer intact until the
        // menu host has finished constructing, then issue their own bounded
        // request. They route to separate polls so neither boss can inherit
        // the other's probe, retry budget, or fallback.
        state.rathalos_acquire_probe = RathalosAcquireProbe::empty();
        state.host_hidden = false;
        state.phase = AmiiboPreviewPhase::AwaitingRathalosAcquire;
        return true;
    }

    if profile.item_acquire_recipe.requires_empty_host_slots() {
        let (before_slot_ids, before_slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
        ItemModule::remove_all(module_accessor);
        let (after_slot_ids, after_slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
        let host_slots_empty_after_cleanup = after_slot_ids.iter().all(|item_id| *item_id == 0);
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] presentation_preacquire_cleanup generation={} logical_boss={} host_object_id=0x{:x} acquisition_recipe={} action=remove_all slots_empty_after_cleanup={} before_slot_ids={:?} before_slot_kinds={:?} after_slot_ids={:?} after_slot_kinds={:?}",
                state.viewer_generation,
                profile.key,
                host_id,
                profile.item_acquire_recipe.name(),
                host_slots_empty_after_cleanup,
                before_slot_ids,
                before_slot_kinds,
                after_slot_ids,
                after_slot_kinds
            );
        }
    }

    prepare_viewer_host_for_item_acquisition(module_accessor, profile.item_acquire_recipe);
    let have_item_begin_tick = system_tick();
    ItemModule::have_item(
        module_accessor,
        ItemKind(requested_item_kind),
        0,
        0,
        false,
        false,
    );
    state.have_item_ticks = system_tick().saturating_sub(have_item_begin_tick);
    SoundModule::stop_se(module_accessor, Hash40::new("se_item_item_get"), 0);

    let Some((slot, presentation_id, presentation_boma)) =
        crate::boss_helpers::held_item_by_kind(module_accessor, &[requested_item_kind])
    else {
        let (slot_ids, slot_kinds) = viewer_held_item_slot_snapshot(module_accessor);
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] presentation_create_failed generation={} logical_boss={} host_object_id=0x{:x} requested_kind={} backing={} acquisition_recipe={} reason=item_acquisition_returned_no_verified_kind slot_ids={:?} slot_kinds={:?} action=restore_native_viewer",
                state.viewer_generation,
                profile.key,
                host_id,
                requested_item_kind,
                requested_backing,
                profile.item_acquire_recipe.name(),
                slot_ids,
                slot_kinds
            );
        }
        restore_viewer_host(module_accessor);
        state.host_hidden = false;
        state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
        return false;
    };

    let acquired_kind = smash::app::utility::get_kind(&mut *presentation_boma);
    if acquired_kind != requested_item_kind {
        if crate::debug::enabled() {
            crate::boss_log!(
                "[PB][AmiiboPreviewRuntime] presentation_create_failed generation={} logical_boss={} host_object_id=0x{:x} requested_kind={} acquired_kind={} reason=kind_mismatch action=restore_native_viewer",
                state.viewer_generation,
                profile.key,
                host_id,
                requested_item_kind,
                acquired_kind
            );
        }
        // `slot` is verified as a host-held item created by this path; remove
        // it rather than leaving an unexpected object in the menu scene.
        ItemModule::remove_item(module_accessor, slot);
        restore_viewer_host(module_accessor);
        state.host_hidden = false;
        state.phase = AmiiboPreviewPhase::DeferredUntilSupported;
        return false;
    }

    state.presentation_object_id = presentation_id;
    state.created_frame_mark = state.frames_since_viewer_host_ready;
    state.expected_item_kind = requested_item_kind;
    state.presentation_slot = slot;
    state.ownership = AmiiboPreviewOwnership::HostSlot;
    state.ready_visual_logged = false;
    state.visual_ready_blocked_logged = false;
    state.visibility_reassertion_used = false;
    state.last_item_visible = None;
    state.last_item_model_visible = None;
    state.motion_probe_samples_remaining = MOTION_PROBE_SAMPLE_FRAMES;
    state.motion_probe_motion_before = 0;
    state.motion_probe_motion_after_change = 0;
    state.motion_probe_first_frame = -1.0;
    state.motion_probe_last_frame = -1.0;
    state.motion_probe_advanced = false;
    state.frames_since_viewer_host_ready = 0;
    state.frames_host_ready_to_create = 0;
    state.frames_create_to_engine_visible = 0;
    state.frames_create_to_model_visible = 0;
    state.have_item_ticks = 0;
    state.created_frame_mark = 0;
    state.visible_latency_logged = false;
    state.transform_comparison_samples_remaining = 0;
    state.transform_comparison_complete = false;
    state.interactive_transform_probe = InteractiveTransformProbe::empty();
    state.transform_calibration = TransformCalibrationState::empty();
    let native_rotation_after_create = presentation_rotation(presentation_boma);
    let presentation = apply_item_presentation(presentation_boma, state, profile);
    if state.motion_probe_motion_after_change == 0 {
        state.motion_probe_motion_before = presentation.motion_before;
        state.motion_probe_motion_after_change = presentation.motion_after_change;
        state.motion_probe_first_frame = presentation.frame_before;
    }
    state.transform_stabilization_frames_remaining = PREVIEW_TRANSFORM_STABILIZATION_FRAMES;
    let host_presentation = initialize_viewer_host_presentation_recipe(module_accessor, profile);
    state.host_hidden = true;
    initialize_presentation_visibility(state, profile, presentation_boma, module_accessor);
    state.phase = AmiiboPreviewPhase::Ready;
    // Same object after the plugin's scale + `"wait"` motion, with NO status
    // forced. If the object stays inert here while its native state looked
    // uninitialized, that is the evidence for a status dependency — report it
    // rather than forcing one, because a forced status previously detached
    // presentation items from the host slot and killed native rotation.
    log_boss_backing_probe(
        state,
        profile,
        presentation_boma,
        acquired_kind,
        requested_backing,
        slot,
        host_slot_for_presentation(module_accessor, presentation_id).is_some(),
        "after_wait_motion_no_status_forced",
    );
    let desired_position = desired_preview_position(state, profile);
    if crate::debug::enabled() {
        let host_root_rotation = host_presentation.root_rotation_set.unwrap_or([0.0; 3]);
        let host_position = viewer_host_position(module_accessor);
        let desired_presentation_rotation =
            desired_item_presentation_rotation(profile, presentation.native_rotation_after_motion);
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] presentation_created generation={} logical_boss={} host_object_id=0x{:x} presentation_object_id=0x{:x} slot={} requested_kind={} acquired_kind={} backing={} initial_host_position=({:.3},{:.3},{:.3}) current_host_position=({:.3},{:.3},{:.3}) viewer_anchor_position=({:.3},{:.3},{:.3}) desired_position=({:.3},{:.3},{:.3}) actual_position=({:.3},{:.3},{:.3}) native_rotation_after_create=({:.1},{:.1},{:.1}) native_rotation_after_motion=({:.1},{:.1},{:.1}) native_lr={:.3} presentation_rotation_override={} desired_presentation_rotation=({:.1},{:.1},{:.1}) final_presentation_rotation=({:.1},{:.1},{:.1}) desired_scale={:.4} actual_scale={:.4} stabilization_frames={} ownership=host_slot presentation_only=true",
            state.viewer_generation,
            profile.key,
            host_id,
            presentation_id,
            slot,
            requested_item_kind,
            acquired_kind,
            requested_backing,
            state.viewer_anchor.initial_position[0],
            state.viewer_anchor.initial_position[1],
            state.viewer_anchor.initial_position[2],
            host_position[0],
            host_position[1],
            host_position[2],
            state.viewer_anchor.position[0],
            state.viewer_anchor.position[1],
            state.viewer_anchor.position[2],
            desired_position.x,
            desired_position.y,
            desired_position.z,
            PostureModule::pos_x(presentation_boma),
            PostureModule::pos_y(presentation_boma),
            PostureModule::pos_z(presentation_boma),
            native_rotation_after_create[0],
            native_rotation_after_create[1],
            native_rotation_after_create[2],
            presentation.native_rotation_after_motion[0],
            presentation.native_rotation_after_motion[1],
            presentation.native_rotation_after_motion[2],
            presentation.native_lr_after_motion,
            presentation.desired_presentation_rotation.is_some(),
            desired_presentation_rotation[0],
            desired_presentation_rotation[1],
            desired_presentation_rotation[2],
            presentation.final_presentation_rotation[0],
            presentation.final_presentation_rotation[1],
            presentation.final_presentation_rotation[2],
            profile
                .preview_scale
                .unwrap_or_else(|| ModelModule::scale(presentation_boma)),
            ModelModule::scale(presentation_boma),
            state.transform_stabilization_frames_remaining,
        );
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] host_orientation_applied generation={} logical_boss={} boss_motion={} host_recipe={} host_posture_rotation={} host_root_rotation=({:.1},{:.1},{:.1}) host_rotation_before=({:.1},{:.1},{:.1}) host_rotation_after=({:.1},{:.1},{:.1}) host_scale={:.4} frozen_viewer_position=({:.3},{:.3},{:.3}) host_position=({:.3},{:.3},{:.3}) boss_rotation_after_create=({:.1},{:.1},{:.1}) boss_native_rotation_after_motion=({:.1},{:.1},{:.1}) presentation_rotation_override={} desired_presentation_rotation=({:.1},{:.1},{:.1}) final_presentation_rotation=({:.1},{:.1},{:.1}) boss_lr={:.3} boss_scale={:.4} boss_position=({:.3},{:.3},{:.3})",
            state.viewer_generation,
            profile.key,
            profile.idle_motion,
            host_presentation.recipe.name(),
            host_presentation.recipe.posture_rotation_name(),
            host_root_rotation[0],
            host_root_rotation[1],
            host_root_rotation[2],
            host_presentation.rotation_before[0],
            host_presentation.rotation_before[1],
            host_presentation.rotation_before[2],
            host_presentation.rotation_after[0],
            host_presentation.rotation_after[1],
            host_presentation.rotation_after[2],
            ModelModule::scale(module_accessor),
            state.viewer_anchor.position[0],
            state.viewer_anchor.position[1],
            state.viewer_anchor.position[2],
            host_position[0],
            host_position[1],
            host_position[2],
            native_rotation_after_create[0],
            native_rotation_after_create[1],
            native_rotation_after_create[2],
            presentation.native_rotation_after_motion[0],
            presentation.native_rotation_after_motion[1],
            presentation.native_rotation_after_motion[2],
            presentation.desired_presentation_rotation.is_some(),
            desired_presentation_rotation[0],
            desired_presentation_rotation[1],
            desired_presentation_rotation[2],
            presentation.final_presentation_rotation[0],
            presentation.final_presentation_rotation[1],
            presentation.final_presentation_rotation[2],
            presentation.native_lr_after_motion,
            ModelModule::scale(presentation_boma),
            PostureModule::pos_x(presentation_boma),
            PostureModule::pos_y(presentation_boma),
            PostureModule::pos_z(presentation_boma)
        );
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] motion_started generation={} presentation_object_id=0x{:x} motion={} motion_result=0x{:x} camera=native_viewer_host",
            state.viewer_generation,
            presentation_id,
            profile.idle_motion,
            presentation.motion
        );
        crate::boss_log!(
            "[PB][AmiiboPreviewRuntime] presentation_active generation={} logical_boss={} host_hidden=true host_scale={:.4} host_engine_visible={} presentation_object_id=0x{:x}",
            state.viewer_generation,
            profile.key,
            ModelModule::scale(module_accessor),
            VisibilityModule::is_visible(module_accessor),
            presentation_id
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amiibo::BOSS_IDENTITIES;

    #[test]
    fn every_boss_identity_has_one_preview_profile() {
        assert_eq!(profiles().len(), BOSS_IDENTITIES.len());
        for identity in BOSS_IDENTITIES {
            let profile = profile_for_ui_chara_id(identity.ui_chara_id)
                .expect("missing boss preview profile");
            assert_eq!(profile.key, identity.key);
            assert_eq!(profile.ui_chara_id, identity.ui_chara_id);
        }
    }

    #[test]
    fn preview_profile_keys_and_ui_ids_are_unique() {
        for (index, profile) in profiles().iter().enumerate() {
            assert!(!profiles()[index + 1..]
                .iter()
                .any(|other| other.key == profile.key || other.ui_chara_id == profile.ui_chara_id));
        }
    }

    #[test]
    fn preview_profiles_have_safe_scale_defaults() {
        for profile in profiles() {
            assert!(profile
                .preview_scale
                .map(|scale| scale > 0.0)
                .unwrap_or(true));
        }
    }

    #[test]
    fn direct_scan_lookup_is_distinct_from_catalog_enumeration() {
        assert!(is_direct_amiibo_identity_lookup(0xC1FF_FF13_8910_2CBF));
        assert!(!is_direct_amiibo_identity_lookup(0xC100_5113_8910_2CBF));
        assert!(is_direct_amiibo_identity_lookup(0xC1FF_FF00_0000_0000));
        assert!(profile_for_ui_chara_hash(0).is_none());
    }

    #[test]
    fn rollout_enables_every_verified_item_backing_but_not_giga_bowser() {
        let galleom = profile_for_ui_chara_id("ui_chara_galleom").unwrap();
        assert!(profile_rollout_enabled(galleom));
        assert_eq!(galleom.preview_scale, Some(0.225));
        assert_eq!(galleom.position_offset, [0.36, 0.17, 0.0]);

        let rathalos = profile_for_ui_chara_id("ui_chara_lioleus").unwrap();
        assert_eq!(
            rathalos.item_acquire_recipe,
            ItemPresentationAcquireRecipe::RathalosBossThenVisualFallback
        );
        assert!(rathalos.item_acquire_recipe.uses_deferred_observation());

        let item_profiles = profiles()
            .iter()
            .filter(|profile| profile.preview_kind == BossAmiiboPreviewKind::ItemPresentation)
            .collect::<Vec<_>>();
        assert_eq!(item_profiles.len(), 10);
        assert!(item_profiles
            .iter()
            .all(|profile| profile_rollout_enabled(profile)));

        let giga_bowser = profile_for_ui_chara_id("ui_chara_koopag").unwrap();
        assert_eq!(
            giga_bowser.preview_kind,
            BossAmiiboPreviewKind::NativeFighterPresentation
        );
        assert!(!profile_rollout_enabled(giga_bowser));
    }

    /// Giga Bowser is the one boss that needs no plugin presentation: it is a
    /// real fighter, so the native viewer builds it once `ui_chara_koopag`
    /// carries `fighter_kind_koopag`. `callback_koopag` patches that genuine
    /// row in place (selectable, not hidden) and must not clone Bowser over it.
    #[test]
    fn giga_bowser_is_viewer_owned_with_no_item_backing() {
        let giga_bowser = profile_for_ui_chara_id("ui_chara_koopag").unwrap();
        assert_eq!(giga_bowser.preview_source, "fighter:koopag");
        assert_eq!(
            giga_bowser.host_orientation_recipe,
            AmiiboHostOrientationRecipe::NativeFighter
        );
        // No plugin-owned presentation item at all: the viewer builds the
        // fighter, and `src/gigabowser/mod.rs` applies the preview-only scale
        // directly to it, so this profile carries no item scale.
        assert_eq!(giga_bowser.preview_scale, None);
        assert_eq!(giga_bowser.position_offset, [0.0, 0.0, 0.0]);

        // Its backing is a fighter kind, not an item kind — it is the only
        // profile for which that is true.
        match unsafe { verified_presentation_backing(giga_bowser) } {
            Some(VerifiedPreviewBacking::NativeFighter { kind, source }) => {
                assert_eq!(kind, *FIGHTER_KIND_KOOPAG);
                assert_eq!(source, "FIGHTER_KIND_KOOPAG");
            }
            _ => panic!("Giga Bowser must keep its native fighter backing"),
        }
        for profile in profiles() {
            let native_fighter = matches!(
                unsafe { verified_presentation_backing(profile) },
                Some(VerifiedPreviewBacking::NativeFighter { .. })
            );
            assert_eq!(
                native_fighter,
                profile.key == "giga_bowser",
                "boss {} must not be a native-fighter presentation",
                profile.key
            );
        }

        // Startup diagnostics must say the viewer owns it, not that the
        // plugin is waiting to verify a presentation it never creates.
        let backing = unsafe { verified_presentation_backing(giga_bowser) };
        assert_eq!(
            mapping_runtime_status(giga_bowser, backing),
            "native_fighter_viewer_owned_no_plugin_presentation"
        );
        assert_eq!(
            crate::amiibo::BOSS_IDENTITIES
                .iter()
                .find(|identity| identity.key == "giga_bowser")
                .expect("missing Giga Bowser identity")
                .backing_fighter,
            "fighter_kind_koopag"
        );
    }

    #[test]
    fn rathalos_acquisition_probe_is_bounded() {
        let mut probe = RathalosAcquireProbe::empty();
        assert!(probe.can_retry());
        assert!(!probe.host_settled());
        assert!(!probe.observation_window_elapsed());
        assert!(!probe.fallback_requested);
        assert!(!probe.fallback_window_elapsed());

        probe.host_settle_frames = RATHALOS_HOST_SETTLE_FRAMES - 1;
        assert!(!probe.host_settled());
        probe.host_settle_frames = RATHALOS_HOST_SETTLE_FRAMES;
        assert!(probe.host_settled());

        probe.request_count = 1;
        probe.frames_since_request = RATHALOS_ACQUIRE_SETTLE_FRAMES - 1;
        assert!(probe.can_retry());
        assert!(!probe.observation_window_elapsed());

        probe.frames_since_request = RATHALOS_ACQUIRE_SETTLE_FRAMES;
        assert!(probe.observation_window_elapsed());
        probe.request_count = RATHALOS_MAX_ACQUIRE_REQUESTS;
        assert!(!probe.can_retry());

        // The proven ceiling stays where hardware put it: two boss-backing
        // requests, each observed for one stabilization window.
        assert_eq!(RATHALOS_MAX_ACQUIRE_REQUESTS, 2);
        assert_eq!(
            RATHALOS_HOST_SETTLE_FRAMES,
            PREVIEW_TRANSFORM_STABILIZATION_FRAMES
        );
        assert_eq!(
            RATHALOS_ACQUIRE_SETTLE_FRAMES,
            PREVIEW_TRANSFORM_STABILIZATION_FRAMES
        );
    }

    /// The menu-only visual fallback happens once and is observed for exactly
    /// one bounded window. `fallback_requested` latches, so nothing can issue
    /// a second alternate-kind request.
    #[test]
    fn rathalos_menu_visual_fallback_is_bounded_to_one_request() {
        let mut probe = RathalosAcquireProbe::empty();
        assert!(!probe.fallback_requested);

        probe.fallback_requested = true;
        probe.frames_since_fallback = RATHALOS_FALLBACK_SETTLE_FRAMES - 1;
        assert!(!probe.fallback_window_elapsed());
        probe.frames_since_fallback = RATHALOS_FALLBACK_SETTLE_FRAMES;
        assert!(probe.fallback_window_elapsed());

        assert_eq!(
            RATHALOS_FALLBACK_SETTLE_FRAMES,
            PREVIEW_TRANSFORM_STABILIZATION_FRAMES
        );
        // A generation reset clears the latch and restores profile-motion
        // ownership, so the fallback can never leak across viewer generations.
        assert!(!RathalosAcquireProbe::empty().fallback_requested);
        assert_eq!(RathalosAcquireProbe::empty().frames_since_fallback, 0);
        assert!(AmiiboPreviewState::new().apply_profile_idle_motion);
    }

    /// Replaces the disproven "Marx stays static" assumption. Hardware proved
    /// the old WOL posture recipe (plugin writing host posture every Ready
    /// frame) froze the native right-stick turntable. Marx must be rotatable
    /// like every other Amiibo preview boss: fully native host ownership,
    /// boss-local static correction only.
    #[test]
    fn marx_amiibo_preview_is_rotatable_with_native_host_ownership() {
        let marx = profile_for_ui_chara_id("ui_chara_marx").unwrap();

        assert_eq!(marx.idle_motion, "wait");
        assert_eq!(marx.preview_scale, Some(0.28125));
        assert_eq!(marx.presentation_rotation, Some([0.0, 0.0, -90.0]));
        assert_eq!(
            marx.host_orientation_recipe,
            AmiiboHostOrientationRecipe::NativeHost
        );
        // Nothing on the host is plugin-owned: no posture write (native
        // interactive rotation) and no host-root static correction (the WOL
        // composition was hardware-disproven as a stage-0x135 channel).
        assert_eq!(marx.host_orientation_recipe.posture_rotation(), None);
        assert_eq!(marx.host_orientation_recipe.root_rotation(), None);
        assert_eq!(
            marx.host_orientation_recipe.posture_rotation_name(),
            "native"
        );
        // Static correction ownership is boss-local and uncalibrated until
        // the hardware harness reports a value. It must not copy Master Hand.
        assert_eq!(marx.item_root_rotation, None);
        assert_eq!(
            marx.item_acquire_recipe,
            ItemPresentationAcquireRecipe::Direct
        );
        assert_eq!(marx.position_offset, [0.36, 0.17, 0.0]);
        assert!(owns_boss_local_transform(marx));
    }

    /// Dracula now uses Master Hand's exact acquisition path — a plain
    /// `have_item` on the viewer-host-ready frame — with his own phase-1 item.
    /// It is config-gated because that call has crashed on hardware for every
    /// backing and preparation tried so far, so a normal install never reaches
    /// it and the viewer shows native Mario.
    #[test]
    fn dracula_uses_master_hands_direct_path_behind_a_config_gate() {
        let dracula = profile_for_ui_chara_id("ui_chara_dracula").unwrap();
        let master_hand = profile_for_ui_chara_id("ui_chara_masterhand").unwrap();

        // Byte-for-byte the same acquisition shape as Master Hand.
        assert_eq!(
            dracula.item_acquire_recipe,
            ItemPresentationAcquireRecipe::Direct
        );
        assert_eq!(dracula.item_acquire_recipe, master_hand.item_acquire_recipe);
        assert!(!dracula.item_acquire_recipe.requires_empty_host_slots());
        assert!(!dracula.item_acquire_recipe.uses_tiny_host_before_request());
        assert!(!dracula.item_acquire_recipe.uses_deferred_observation());
        assert!(!dracula.item_acquire_recipe.uses_menu_visual_fallback());

        // His own item, phase 1 — the kind `src/dracula/mod.rs` acquires in a
        // real match, not the transformed phase 2.
        assert_ne!(*ITEM_KIND_DRACULA, *ITEM_KIND_DRACULA2);
        match unsafe { verified_presentation_backing(dracula) } {
            Some(VerifiedPreviewBacking::Item { kind, source }) => {
                assert_eq!(kind, *ITEM_KIND_DRACULA);
                assert_eq!(source, "ITEM_KIND_DRACULA (battle-proven phase 1)");
            }
            _ => panic!("Dracula must use his phase 1 backing"),
        }

        // The gate is keyed on the boss, so no other profile can be blocked by
        // it and Dracula cannot silently inherit a different recipe's policy.
        assert_eq!(dracula.key, DRACULA_PREVIEW_KEY);
        for profile in profiles() {
            let gated = profile.key == DRACULA_PREVIEW_KEY;
            assert_eq!(
                mapping_runtime_status(profile, unsafe { verified_presentation_backing(profile) })
                    == "acquisition_blocked_all_known_have_item_paths_crash_fail_closed",
                gated,
                "boss {} has the wrong runtime status",
                profile.key
            );
        }

        // Transform ownership is untouched and matches the working bosses.
        assert_eq!(
            dracula.host_orientation_recipe,
            master_hand.host_orientation_recipe
        );
        assert_eq!(
            dracula.presentation_rotation,
            master_hand.presentation_rotation
        );
        assert_eq!(dracula.position_offset, master_hand.position_offset);
        assert_eq!(dracula.idle_motion, "wait");
    }

    /// Rathalos keeps its source-backed boss primary and gains exactly one
    /// menu-only visual fallback. Neither shares state or assumptions with
    /// Dracula, and the fallback changes no identity or gameplay backing.
    #[test]
    fn rathalos_keeps_boss_primary_with_one_exact_kind_menu_fallback() {
        let rathalos = profile_for_ui_chara_id("ui_chara_lioleus").unwrap();
        assert_eq!(rathalos.ui_chara_id, "ui_chara_lioleus");
        assert_eq!(rathalos.preview_source, "item:lioleusboss");
        assert_eq!(
            rathalos.item_acquire_recipe,
            ItemPresentationAcquireRecipe::RathalosBossThenVisualFallback
        );
        assert!(rathalos.item_acquire_recipe.requires_empty_host_slots());
        assert!(rathalos.item_acquire_recipe.uses_tiny_host_before_request());
        assert!(rathalos.item_acquire_recipe.uses_deferred_observation());
        assert!(rathalos.item_acquire_recipe.uses_menu_visual_fallback());

        // Primary backing is unchanged: the boss item, not the fallback.
        match unsafe { verified_presentation_backing(rathalos) } {
            Some(VerifiedPreviewBacking::Item { kind, source }) => {
                assert_eq!(kind, *ITEM_KIND_LIOLEUSBOSS);
                assert_eq!(source, "ITEM_KIND_LIOLEUSBOSS");
            }
            _ => panic!("Rathalos must keep the boss item as its primary backing"),
        }
        // The fallback is a different, distinct kind and is menu-visual only.
        assert_ne!(*ITEM_KIND_LIOLEUS, *ITEM_KIND_LIOLEUSBOSS);

        // No other profile may pick up the fallback path.
        for profile in profiles() {
            assert_eq!(
                profile.item_acquire_recipe.uses_menu_visual_fallback(),
                profile.key == RATHALOS_PREVIEW_KEY,
                "boss {} must not use the Rathalos menu visual fallback",
                profile.key
            );
        }

        // The direct recipe stays synchronous and untouched.
        assert!(!ItemPresentationAcquireRecipe::Direct.requires_empty_host_slots());
        assert!(!ItemPresentationAcquireRecipe::Direct.uses_deferred_observation());
        assert!(!ItemPresentationAcquireRecipe::Direct.uses_menu_visual_fallback());
    }

    /// `exact_kind_acquired_item` is the only acquisition verifier left, for
    /// both the boss primary and the fallback. Provenance never substitutes
    /// for identity, so a mismatched or unknown held item is never adopted.
    #[test]
    fn acquisition_never_adopts_a_mismatched_kind() {
        // `held_item_by_kind` matches only the kinds it is given, so an
        // unknown kind can never satisfy either request.
        let primary_kinds = [*ITEM_KIND_LIOLEUSBOSS];
        let fallback_kinds = [*ITEM_KIND_LIOLEUS];
        assert!(!primary_kinds.contains(&*ITEM_KIND_LIOLEUS));
        assert!(!fallback_kinds.contains(&*ITEM_KIND_LIOLEUSBOSS));
        assert!(!fallback_kinds.contains(&*ITEM_KIND_MASTERHAND));
        assert!(!fallback_kinds.contains(&*ITEM_KIND_DRACULA2));
    }

    /// FREEZE. Every profile below is hardware-proven on stage 0x135:
    /// correctly oriented, correctly framed, and rotatable with the native
    /// right-stick turntable. Galleom and Rathalos in particular proved
    /// `ownership=host_slot`, `slot_still_held=true`, host posture changing
    /// with the stick, host root zero, item posture [0,0,-90], item root
    /// [0,0,0] and item position [0.36,0.18,0] with the plugin writing only
    /// the item position. Nothing here may drift without new hardware
    /// evidence.
    #[test]
    fn hardware_proven_boss_profiles_are_frozen() {
        let frozen = [
            ("ui_chara_masterhand", 0.45, "wait"),
            ("ui_chara_mewtwo_masterhand", 0.45, "wait"),
            ("ui_chara_crazyhand", 0.45, "wait"),
            ("ui_chara_marx", 0.28125, "wait"),
            ("ui_chara_ganonboss", 0.365625, "wait"),
            ("ui_chara_galleom", 0.225, "wait"),
            ("ui_chara_lioleus", 0.225, "hovering_move"),
        ];

        for (ui_chara_id, scale, motion) in frozen {
            let profile = profile_for_ui_chara_id(ui_chara_id).expect("missing frozen profile");
            assert_eq!(
                profile.host_orientation_recipe,
                AmiiboHostOrientationRecipe::NativeHost,
                "{} lost NativeHost ownership",
                profile.key
            );
            // Nintendo owns host posture (the turntable) and the host root
            // stays untouched: the recipe carries no host correction at all.
            assert_eq!(profile.host_orientation_recipe.posture_rotation(), None);
            assert_eq!(profile.host_orientation_recipe.root_rotation(), None);
            assert_eq!(
                profile.presentation_rotation,
                Some([0.0, 0.0, -90.0]),
                "{} lost its proven item posture",
                profile.key
            );
            assert_eq!(profile.item_root_rotation, None, "{}", profile.key);
            assert_eq!(
                profile.position_offset,
                [0.36, 0.17, 0.0],
                "{} lost its proven framing",
                profile.key
            );
            assert_eq!(profile.preview_scale, Some(scale), "{}", profile.key);
            assert_eq!(profile.idle_motion, motion, "{}", profile.key);
            // Continuous viewer-anchor position pinning stays on.
            assert!(owns_boss_local_transform(profile), "{}", profile.key);
        }

        // Rathalos keeps its own acquisition; everything else stays Direct.
        assert_eq!(
            profile_for_ui_chara_id("ui_chara_lioleus")
                .unwrap()
                .item_acquire_recipe,
            ItemPresentationAcquireRecipe::RathalosBossThenVisualFallback
        );
        for ui_chara_id in [
            "ui_chara_masterhand",
            "ui_chara_mewtwo_masterhand",
            "ui_chara_crazyhand",
            "ui_chara_marx",
            "ui_chara_ganonboss",
            "ui_chara_galleom",
        ] {
            assert_eq!(
                profile_for_ui_chara_id(ui_chara_id)
                    .unwrap()
                    .item_acquire_recipe,
                ItemPresentationAcquireRecipe::Direct
            );
        }
    }

    /// Dracula is unconditionally fail-closed. Every known `have_item` path
    /// for both backings crashed on hardware, so nothing — including any
    /// configuration option — may reach that boundary from stage 0x135.
    #[test]
    fn dracula_is_permanently_blocked_with_no_config_escape() {
        let dracula = profile_for_ui_chara_id("ui_chara_dracula").unwrap();
        assert_eq!(dracula.key, DRACULA_PREVIEW_KEY);
        assert_eq!(
            mapping_runtime_status(dracula, unsafe { verified_presentation_backing(dracula) }),
            "acquisition_blocked_all_known_have_item_paths_crash_fail_closed"
        );
        // Only Dracula is blocked; nothing else inherits it.
        for profile in profiles() {
            let blocked =
                mapping_runtime_status(profile, unsafe { verified_presentation_backing(profile) })
                    == "acquisition_blocked_all_known_have_item_paths_crash_fail_closed";
            assert_eq!(
                blocked,
                profile.key == DRACULA_PREVIEW_KEY,
                "{}",
                profile.key
            );
        }
        // The presentation profile survives for a future safe factory.
        assert_eq!(
            dracula.host_orientation_recipe,
            AmiiboHostOrientationRecipe::NativeHost
        );
        assert_eq!(dracula.presentation_rotation, Some([0.0, 0.0, -90.0]));
        assert_eq!(dracula.position_offset, [0.36, 0.17, 0.0]);
        assert_eq!(dracula.preview_scale, Some(0.45));
        assert_eq!(dracula.idle_motion, "wait");
    }

    /// Presentation-only backing swap. Hardware proved `"wait"` was accepted
    /// and advancing on the CORE objects (rate 1.0, frame 0 -> 30,
    /// `animation_advanced=true`) while the complete animated boss still never
    /// appeared, so the core props were the wrong visual objects. The full
    /// boss objects replace them for the Amiibo viewer only; identity,
    /// orientation, framing and host ownership are untouched.
    #[test]
    fn galeem_and_dharkon_use_full_boss_amiibo_backings() {
        let galeem = profile_for_ui_chara_id("ui_chara_kiila").unwrap();
        let dharkon = profile_for_ui_chara_id("ui_chara_darz").unwrap();

        // Logical Amiibo identity is unchanged by a visual backing swap.
        assert_eq!(galeem.key, "galeem");
        assert_eq!(dharkon.key, "dharkon");
        assert_eq!(galeem.ui_chara_id, "ui_chara_kiila");
        assert_eq!(dharkon.ui_chara_id, "ui_chara_darz");

        // Full boss objects, not the core props.
        match unsafe { verified_presentation_backing(galeem) } {
            Some(VerifiedPreviewBacking::Item { kind, source }) => {
                assert_eq!(kind, *ITEM_KIND_KIILA);
                assert_eq!(source, "ITEM_KIND_KIILA");
            }
            _ => panic!("Galeem must use the full boss backing"),
        }
        match unsafe { verified_presentation_backing(dharkon) } {
            Some(VerifiedPreviewBacking::Item { kind, source }) => {
                assert_eq!(kind, *ITEM_KIND_DARZ);
                assert_eq!(source, "ITEM_KIND_DARZ");
            }
            _ => panic!("Dharkon must use the full boss backing"),
        }
        assert_ne!(*ITEM_KIND_KIILA, *ITEM_KIND_KIILACORE);
        assert_ne!(*ITEM_KIND_DARZ, *ITEM_KIND_DARZCENTIPEDE);
        // The hardware-proven stage-0x135 architecture is untouched.
        for profile in [galeem, dharkon] {
            assert_eq!(
                profile.host_orientation_recipe,
                AmiiboHostOrientationRecipe::NativeHost
            );
            assert_eq!(profile.host_orientation_recipe.posture_rotation(), None);
            assert_eq!(profile.host_orientation_recipe.root_rotation(), None);
            assert_eq!(profile.presentation_rotation, Some([0.0, 0.0, -90.0]));
            assert_eq!(profile.item_root_rotation, None);
            assert_eq!(profile.position_offset, [0.36, 0.17, 0.0]);
            assert_eq!(profile.idle_motion, "wait");
            assert!(owns_boss_local_transform(profile));
            // Exact-kind acquisition on the plain Direct path — no staged
            // acquisition and no menu-visual fallback adoption.
            assert_eq!(
                profile.item_acquire_recipe,
                ItemPresentationAcquireRecipe::Direct
            );
            assert!(!profile.item_acquire_recipe.uses_menu_visual_fallback());
            assert!(uses_full_boss_backing(profile));
        }

        // Only these two carry the full-boss backing probe.
        for profile in profiles() {
            assert_eq!(
                uses_full_boss_backing(profile),
                profile.key == "galeem" || profile.key == "dharkon",
                "boss {} must not be treated as a full-boss backing",
                profile.key
            );
        }
    }

    /// Locks the truthful AI-authority position. The pinned bindings expose no
    /// FP/NFP discriminator and no FP level accessor, so the plugin must not
    /// pretend to support amiibo intensity. This test fails if anyone adds a
    /// config knob implying an amiibo-level -> boss-difficulty mapping.
    #[test]
    fn no_fabricated_figure_player_intensity_mapping_exists() {
        // `BOSS_DIFFICULTY` is the only intensity source, and it is a plain
        // ordinary-CPU boss-AI value. There is deliberately no FP level, FP
        // intensity, or amiibo-level option to derive one from.
        let distributed = include_str!("../ultimate/mods/Bosses/config.toml");
        let parsed: toml::Value = toml::from_str(distributed).expect("distributed config parses");
        let options = parsed.get("options").and_then(|v| v.as_table()).unwrap();
        assert!(options.contains_key("BOSS_DIFFICULTY"));
        for forbidden in [
            "AMIIBO_DIFFICULTY",
            "FP_DIFFICULTY",
            "FP_LEVEL",
            "AMIIBO_LEVEL",
            "FIGURE_PLAYER_DIFFICULTY",
            "AMIIBO_INTENSITY",
        ] {
            assert!(
                !options.contains_key(forbidden),
                "{} implies an unsupported FP intensity mapping",
                forbidden
            );
        }
    }

    #[test]
    fn viewer_anchor_starts_uninitialized() {
        let anchor = ViewerAnchor::empty();
        assert!(!anchor.initialized);
        assert_eq!(anchor.position, [0.0; 3]);
        assert_eq!(anchor.lr, 1.0);
        assert_eq!(anchor.rotation, [0.0; 3]);
        assert_eq!(anchor.initial_rotation, [0.0; 3]);
        assert_eq!(anchor.initial_position, [0.0; 3]);
    }

    #[test]
    fn item_profiles_match_current_manual_viewer_calibration() {
        let expected = [
            (
                "master_hand",
                0.45,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "crazy_hand",
                0.45,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "wol_master_hand",
                0.45,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "galeem",
                0.28125,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "dharkon",
                0.28125,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "dracula",
                0.45,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "ganon_boss",
                0.365625,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "galleom",
                0.225,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
            (
                "rathalos",
                0.225,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "hovering_move",
            ),
            (
                "marx",
                0.28125,
                [0.36, 0.17, 0.0],
                Some([0.0, 0.0, -90.0]),
                "wait",
            ),
        ];

        for (key, scale, position_offset, presentation_rotation, idle_motion) in expected {
            let profile = profiles()
                .iter()
                .find(|profile| profile.key == key)
                .expect("missing item-backed preview profile");
            assert_eq!(profile.preview_scale, Some(scale));
            assert_eq!(profile.position_offset, position_offset);
            assert_eq!(profile.presentation_rotation, presentation_rotation);
            assert_eq!(profile.idle_motion, idle_motion);
        }

        // Ganon's production idle motion is deliberately "wait"; the preview
        // must not drift back to the attack animation.
        assert_eq!(
            profile_for_ui_chara_id("ui_chara_ganonboss")
                .unwrap()
                .idle_motion,
            "wait"
        );
    }

    /// Every item-backed boss uses the Master Hand / Marx architecture: a
    /// fully native host and one item-local posture correction. A boss that
    /// drifts back onto a WOL host recipe lies on its side, which is what this
    /// locks against.
    #[test]
    fn every_item_preview_uses_the_native_host_architecture() {
        for profile in profiles() {
            let expected = match profile.preview_kind {
                BossAmiiboPreviewKind::ItemPresentation => AmiiboHostOrientationRecipe::NativeHost,
                BossAmiiboPreviewKind::NativeFighterPresentation => {
                    AmiiboHostOrientationRecipe::NativeFighter
                }
            };
            assert_eq!(
                profile.host_orientation_recipe, expected,
                "boss {} must not use a WOL host recipe",
                profile.key
            );
            assert_eq!(profile.host_orientation_recipe.posture_rotation(), None);
            assert_eq!(profile.host_orientation_recipe.root_rotation(), None);

            if profile.preview_kind == BossAmiiboPreviewKind::ItemPresentation {
                assert_eq!(
                    profile.presentation_rotation,
                    Some([0.0, 0.0, -90.0]),
                    "boss {} must use the proven upright item correction",
                    profile.key
                );
                assert!(owns_boss_local_transform(profile));
            }
        }

        assert_eq!(
            AmiiboHostOrientationRecipe::NativeHost.posture_rotation_name(),
            "native"
        );
        assert_eq!(
            AmiiboHostOrientationRecipe::NativeFighter.posture_rotation_name(),
            "not_applied"
        );

        // The acquisition-only correction must not have disturbed the two
        // bosses whose recipes changed.
        for key in ["ui_chara_dracula", "ui_chara_lioleus"] {
            let profile = profile_for_ui_chara_id(key).unwrap();
            assert_eq!(
                profile.host_orientation_recipe,
                AmiiboHostOrientationRecipe::NativeHost
            );
            assert_eq!(profile.presentation_rotation, Some([0.0, 0.0, -90.0]));
            assert_eq!(profile.item_root_rotation, None);
            assert!(owns_boss_local_transform(profile));
        }
        assert_eq!(
            profile_for_ui_chara_id("ui_chara_dracula")
                .unwrap()
                .preview_scale,
            Some(0.45)
        );
        assert_eq!(
            profile_for_ui_chara_id("ui_chara_lioleus")
                .unwrap()
                .preview_scale,
            Some(0.225)
        );
    }

    #[test]
    fn galeem_and_dharkon_presentations_share_the_native_wait_motion() {
        let galeem = profiles()
            .iter()
            .find(|profile| profile.key == "galeem")
            .expect("missing Galeem preview profile");
        let dharkon = profiles()
            .iter()
            .find(|profile| profile.key == "dharkon")
            .expect("missing Dharkon preview profile");

        assert_eq!(galeem.idle_motion, crate::galeem::PRESENTATION_IDLE_MOTION);
        assert_eq!(
            dharkon.idle_motion,
            crate::dharkon::PRESENTATION_IDLE_MOTION
        );
        assert_eq!(crate::galeem::PRESENTATION_IDLE_MOTION, "wait");
        assert_eq!(crate::dharkon::PRESENTATION_IDLE_MOTION, "wait");
    }

    #[test]
    fn master_hand_uses_fully_native_host_with_boss_local_correction() {
        let master_hand = profile_for_ui_chara_id("ui_chara_masterhand").unwrap();

        assert_eq!(master_hand.key, "master_hand");
        assert_eq!(master_hand.ui_chara_id, "ui_chara_masterhand");
        assert_eq!(
            master_hand.preview_kind,
            BossAmiiboPreviewKind::ItemPresentation
        );
        assert_eq!(master_hand.preview_source, "item:masterhand");
        assert_eq!(master_hand.idle_motion, "wait");
        assert_eq!(master_hand.preview_scale, Some(0.45));
        assert_eq!(master_hand.position_offset, [0.36, 0.17, 0.0]);
        assert_eq!(master_hand.presentation_rotation, Some([0.0, 0.0, -90.0]));
        assert_eq!(
            master_hand.host_orientation_recipe,
            AmiiboHostOrientationRecipe::NativeHost
        );
        // Host posture belongs to the native Amiibo viewer (interactive
        // rotation) and, after two hardware-disproven Euler re-encodings of
        // the WOL composition, the host root is no longer a plugin channel
        // either. The static correction is boss-local (item side).
        assert_eq!(master_hand.host_orientation_recipe.posture_rotation(), None);
        assert_eq!(master_hand.host_orientation_recipe.root_rotation(), None);
        assert_eq!(
            master_hand.host_orientation_recipe.posture_rotation_name(),
            "native"
        );
        assert_eq!(master_hand.item_root_rotation, None);

        match unsafe { verified_presentation_backing(master_hand) } {
            Some(VerifiedPreviewBacking::Item { kind, source }) => {
                assert_eq!(kind, *ITEM_KIND_MASTERHAND);
                assert_eq!(source, "ITEM_KIND_MASTERHAND");
            }
            _ => panic!("Master Hand must use the verified item backing"),
        }
    }

    /// No guessed static-orientation Euler may ship anywhere. Both Master
    /// Hand host-root re-encodings of the WOL composition ([-90,50,0] and
    /// [-50,0,-90]) were hardware-disproven, so every boss-local item-root
    /// correction stays None until the debug calibration harness produces a
    /// hardware-proven value.
    #[test]
    fn proof_pair_static_corrections_ship_uncalibrated() {
        for profile in profiles() {
            assert_eq!(
                profile.item_root_rotation, None,
                "boss {} must not ship a guessed item-root Euler",
                profile.key
            );
        }
        // The proof pair writes no host channel that could hide another
        // guessed constant.
        for key in ["master_hand", "marx"] {
            let profile = profiles()
                .iter()
                .find(|profile| profile.key == key)
                .unwrap();
            assert_eq!(profile.host_orientation_recipe.posture_rotation(), None);
            assert_eq!(profile.host_orientation_recipe.root_rotation(), None);
        }
    }

    #[test]
    fn stable_maintenance_never_owns_host_posture() {
        // Stable framing maintenance and the boss-local channels belong to
        // every item preview; the native fighter route owns none of them.
        for profile in profiles() {
            let item = profile.preview_kind == BossAmiiboPreviewKind::ItemPresentation;
            assert_eq!(owns_boss_local_transform(profile), item);
            // The write helper can never touch a host channel: no recipe
            // carries a host posture or host root correction.
            assert_eq!(profile.host_orientation_recipe.posture_rotation(), None);
            assert_eq!(profile.host_orientation_recipe.root_rotation(), None);
        }
        assert_eq!(
            INTERACTIVE_TRANSFORM_CHANGE_SAMPLES,
            TRANSFORM_COMPARISON_SAMPLE_FRAMES
        );

        let probe = InteractiveTransformProbe::empty();
        assert!(!probe.stable_phase_entered);
        assert!(!probe.stable_state_logged);
        assert_eq!(probe.change_samples_remaining, 0);
        assert!(!probe.has_last_host_posture);
    }

    /// The calibration harness must be debug-only (enforced by the
    /// `crate::debug::enabled()` gate in its input handler), scoped to item
    /// previews, never bound to the right stick, and runtime-only: its state
    /// starts empty and is reset to empty on every viewer generation change
    /// alongside the other per-generation probes.
    #[test]
    fn calibration_harness_is_scoped_and_resets_between_generations() {
        for key in ["ui_chara_masterhand", "ui_chara_marx", "ui_chara_galleom"] {
            assert!(owns_boss_local_transform(
                profile_for_ui_chara_id(key).unwrap()
            ));
        }
        let giga_bowser = profile_for_ui_chara_id("ui_chara_koopag").unwrap();
        assert!(!owns_boss_local_transform(giga_bowser));

        // Fresh state: no overrides, item_root target, x axis, full input
        // probe budget. Generation resets assign exactly this value.
        let empty = TransformCalibrationState::empty();
        assert_eq!(empty.target, CalibrationTarget::ItemRoot);
        assert_eq!(empty.axis, 0);
        for target in [
            CalibrationTarget::ItemRoot,
            CalibrationTarget::ItemPosture,
            CalibrationTarget::HostRoot,
        ] {
            assert_eq!(empty.override_for(target), None);
        }
        assert!(!empty.chord_engaged);
        assert_eq!(empty.attack_hold_frames, 0);
        assert_eq!(
            empty.input_probe_samples_remaining,
            CALIBRATION_INPUT_PROBE_SAMPLES
        );

        // A dialed state discards every override when the generation resets:
        // the reset sites assign `TransformCalibrationState::empty()`.
        let mut dialed = TransformCalibrationState::empty();
        dialed.overrides[CalibrationTarget::ItemRoot.index()] = Some([-75.0, 0.0, 90.0]);
        dialed.target = CalibrationTarget::HostRoot;
        dialed.axis = 2;
        assert_eq!(
            dialed.override_for(CalibrationTarget::ItemRoot),
            Some([-75.0, 0.0, 90.0])
        );
        let after_generation_reset = TransformCalibrationState::empty();
        assert_eq!(
            after_generation_reset.override_for(CalibrationTarget::ItemRoot),
            None
        );
        assert_eq!(after_generation_reset.target, CalibrationTarget::ItemRoot);
        assert_eq!(after_generation_reset.axis, 0);

        // Target cycle covers the three plugin channels and loops.
        assert_eq!(
            CalibrationTarget::ItemRoot.next(),
            CalibrationTarget::ItemPosture
        );
        assert_eq!(
            CalibrationTarget::ItemPosture.next(),
            CalibrationTarget::HostRoot
        );
        assert_eq!(
            CalibrationTarget::HostRoot.next(),
            CalibrationTarget::ItemRoot
        );
        assert_eq!(CalibrationTarget::ItemRoot.name(), "item_root");
        assert_eq!(CalibrationTarget::ItemPosture.name(), "item_posture");
        assert_eq!(CalibrationTarget::HostRoot.name(), "host_root");

        // Step sizes and angle wrapping stay sane for repeated adjustment.
        assert_eq!(CALIBRATION_COARSE_STEP_DEGREES, 15.0);
        assert_eq!(CALIBRATION_FINE_STEP_DEGREES, 5.0);
        assert_eq!(wrap_calibration_degrees(0.0), 0.0);
        assert_eq!(wrap_calibration_degrees(15.0), 15.0);
        assert_eq!(wrap_calibration_degrees(190.0), -170.0);
        assert_eq!(wrap_calibration_degrees(-190.0), 170.0);
    }

    /// Startup diagnostics must report Dracula as a config-gated single
    /// attempt against a known crash boundary, not as a plainly enabled
    /// preview. Every other item backing stays enabled and Giga Bowser stays
    /// on the viewer-owned native-fighter route.
    #[test]
    fn mapping_runtime_status_reports_dracula_config_gated() {
        let dracula = profile_for_ui_chara_id("ui_chara_dracula").unwrap();
        let dracula_backing = unsafe { verified_presentation_backing(dracula) };
        assert!(dracula_backing.is_some());
        assert_eq!(
            mapping_runtime_status(dracula, dracula_backing),
            "acquisition_blocked_all_known_have_item_paths_crash_fail_closed"
        );

        for profile in profiles() {
            let backing = unsafe { verified_presentation_backing(profile) };
            assert!(backing.is_some(), "boss {} lost its backing", profile.key);
            let expected = if profile.key == DRACULA_PREVIEW_KEY {
                "acquisition_blocked_all_known_have_item_paths_crash_fail_closed"
            } else {
                match profile.preview_kind {
                    BossAmiiboPreviewKind::ItemPresentation => "stage_135_runtime_enabled",
                    BossAmiiboPreviewKind::NativeFighterPresentation => {
                        "native_fighter_viewer_owned_no_plugin_presentation"
                    }
                }
            };
            assert_eq!(mapping_runtime_status(profile, backing), expected);
        }

        // Rathalos still reports enabled: its `have_item` returns safely, it
        // simply may not produce a held object.
        let rathalos = profile_for_ui_chara_id("ui_chara_lioleus").unwrap();
        let rathalos_backing = unsafe { verified_presentation_backing(rathalos) };
        assert_eq!(
            mapping_runtime_status(rathalos, rathalos_backing),
            "stage_135_runtime_enabled"
        );
    }

    /// A boss item renders its default pose until its own status script runs.
    /// Galleom and Rathalos stood sideways and Galeem/Dharkon played no idle
    /// because the preview issued a motion with no status behind it. Each
    /// Presentation is motion-only, and each motion is the one its WOL
    /// *preview* block uses. Hardware disproved requesting a boss status here:
    /// the working WOL previews (Ganon, Galleom, Rathalos, Master Hand) only
    /// call `change_motion`, and adding a status made the item leave the host
    /// slot, so it stopped following the native turntable.
    #[test]
    fn preview_motions_match_their_wol_preview_blocks() {
        let expected = [
            ("master_hand", "wait"),
            ("crazy_hand", "wait"),
            ("wol_master_hand", "wait"),
            ("galeem", "wait"),
            ("dharkon", "wait"),
            ("dracula", "wait"),
            ("ganon_boss", "wait"),
            ("galleom", "wait"),
            // `src/rathalos/mod.rs`'s preview block, not its recovery path.
            ("rathalos", "hovering_move"),
            ("marx", "wait"),
        ];

        for (key, motion) in expected {
            let profile = profiles()
                .iter()
                .find(|profile| profile.key == key)
                .expect("missing preview profile");
            assert_eq!(
                profile.idle_motion, motion,
                "boss {} lost its WOL preview motion",
                profile.key
            );
        }
    }
    #[test]
    fn transform_comparison_is_scoped_to_master_hand_and_marx() {
        let master_hand = profile_for_ui_chara_id("ui_chara_masterhand").unwrap();
        let marx = profile_for_ui_chara_id("ui_chara_marx").unwrap();
        let galleom = profile_for_ui_chara_id("ui_chara_galleom").unwrap();

        assert!(is_transform_comparison_profile(master_hand));
        assert!(is_transform_comparison_profile(marx));
        assert!(!is_transform_comparison_profile(galleom));
        assert_eq!(TRANSFORM_COMPARISON_SAMPLE_FRAMES, 4);
    }
}

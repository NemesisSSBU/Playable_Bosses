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
    // This is only the operation-CPU bit. It is not a Figure Player test; the
    // pinned public bindings do not expose a separate NFP/FP discriminator.
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
                "[PB][BossItem] acquire_excluding_blocked reason=transition_quarantine entry={} requested_kind={} excluded_id=0x{:x} stage=0x{:x}",
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
    let mut boss_id = 0;
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
    if crate::debug::enabled() {
        let boss_kind = if boss_boma.is_null() {
            -1
        } else {
            smash::app::utility::get_kind(&mut *boss_boma)
        };
        crate::boss_log!(
            "[PB][BossItem] acquire_excluding entry={} requested_kind={} excluded_id=0x{:x} acquired_id=0x{:x} acquired_kind={} stage=0x{:x} fighter_status={} scale={:.4}",
            entry,
            item_kind,
            excluded_item_id,
            boss_id,
            boss_kind,
            smash::app::stage::get_stage_id(),
            StatusModule::status_kind(module_accessor),
            ModelModule::scale(module_accessor)
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
        if expected_kinds
            .iter()
            .any(|&expected_kind| expected_kind == item_kind)
        {
            return Some((slot, item_id, item_boma));
        }
    }
    None
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
    if !expected_kinds.iter().any(|&kind| kind == tracked_kind) {
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
    !module_accessor.is_null()
        && ModelModule::scale(module_accessor) <= HIDDEN_HOST_ENTRY_STAGE2_SCALE
}

#[inline(always)]
pub unsafe fn is_hidden_host_entry_prep(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    scale >= HIDDEN_HOST_ENTRY_PREP_SCALE - HIDDEN_HOST_ENTRY_PREP_EPSILON
        && scale <= HIDDEN_HOST_ENTRY_PREP_SCALE + HIDDEN_HOST_ENTRY_PREP_EPSILON
}

#[inline(always)]
pub unsafe fn is_hidden_host_entry_stage_two(
    module_accessor: *mut BattleObjectModuleAccessor,
) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    scale >= HIDDEN_HOST_ENTRY_STAGE2_SCALE - HIDDEN_HOST_ENTRY_PREP_EPSILON
        && scale <= HIDDEN_HOST_ENTRY_STAGE2_SCALE + HIDDEN_HOST_ENTRY_PREP_EPSILON
}

#[inline(always)]
pub unsafe fn is_hidden_host_baseline(module_accessor: *mut BattleObjectModuleAccessor) -> bool {
    if module_accessor.is_null() {
        return false;
    }
    let scale = ModelModule::scale(module_accessor);
    scale >= HIDDEN_HOST_BASELINE_SCALE - HIDDEN_HOST_BASELINE_EPSILON
        && scale <= HIDDEN_HOST_BASELINE_SCALE + HIDDEN_HOST_BASELINE_EPSILON
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

#[inline(always)]
pub unsafe fn stop_hidden_host_knockout_sfx(module_accessor: *mut BattleObjectModuleAccessor) {
    if !is_hidden_host(module_accessor) {
        return;
    }
    SoundModule::stop_se(module_accessor, Hash40::new("death"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("dead"), 0);
    SoundModule::stop_se(module_accessor, Hash40::new("hp_battle_damage_reaction"), 0);
    SoundModule::stop_se(
        module_accessor,
        Hash40::new("hp_battle_knockout_dead_frame"),
        0,
    );
    SoundModule::stop_se(
        module_accessor,
        Hash40::new("hp_battle_knockout_reaction"),
        0,
    );
    SoundModule::stop_se(
        module_accessor,
        Hash40::new("hp_battle_knockout_slow_frame"),
        0,
    );
    SoundModule::stop_se(
        module_accessor,
        Hash40::new("hp_battle_knockout_slow_mag"),
        0,
    );
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

    let mut reset_joint_rot = Vector3f {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    ModelModule::set_joint_rotate(
        module_accessor,
        Hash40::new("root"),
        &mut reset_joint_rot,
        smash::app::MotionNodeRotateCompose {
            _address: *MOTION_NODE_ROTATE_COMPOSE_BEFORE as u8,
        },
        ModelModule::rotation_order(module_accessor),
    );
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
        stop_hidden_host_knockout_sfx(module_accessor);
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

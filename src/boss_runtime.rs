pub const MAX_BOSS_ENTRIES: usize = 8;

#[derive(Copy, Clone)]
pub struct BossCommonRuntime {
    pub controllable: bool,
    pub stop: bool,
    pub dead: bool,
    pub result_spawned: bool,
    pub exists_public: bool,
    pub fresh_control: bool,
    pub jump_start: bool,
    pub controller_x: f32,
    pub controller_y: f32,
}

impl BossCommonRuntime {
    pub const fn new() -> Self {
        Self {
            controllable: true,
            stop: false,
            dead: false,
            result_spawned: false,
            exists_public: false,
            fresh_control: false,
            jump_start: false,
            controller_x: 0.0,
            controller_y: 0.0,
        }
    }
}

pub static mut PLAYABLE_MASTERHAND_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut MASTER_HAND_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut CRAZY_HAND_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut GALEEM_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut DHARKON_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut MARX_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut GALLEOM_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut GANON_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];
pub static mut RATHALOS_RUNTIME: [BossCommonRuntime; MAX_BOSS_ENTRIES] =
    [BossCommonRuntime::new(); MAX_BOSS_ENTRIES];

#[inline(always)]
pub const fn sanitize_entry_id(entry_id: usize) -> usize {
    if entry_id < MAX_BOSS_ENTRIES {
        entry_id
    } else {
        0
    }
}

#[inline(always)]
pub unsafe fn slot_ptr(
    slots: *mut [BossCommonRuntime; MAX_BOSS_ENTRIES],
    entry_id: usize,
) -> *mut BossCommonRuntime {
    let entry = sanitize_entry_id(entry_id);
    &raw mut (*slots)[entry]
}

#[inline(always)]
pub unsafe fn any_exists_public(slots: *const [BossCommonRuntime; MAX_BOSS_ENTRIES]) -> bool {
    if slots.is_null() {
        return false;
    }
    let mut index = 0;
    while index < MAX_BOSS_ENTRIES {
        if (*slots)[index].exists_public {
            return true;
        }
        index += 1;
    }
    false
}

pub struct CommonRuntimeSyncGuard {
    slot: *mut BossCommonRuntime,
    store: unsafe fn(*mut BossCommonRuntime),
}

impl CommonRuntimeSyncGuard {
    #[inline(always)]
    pub unsafe fn new(
        slot: *mut BossCommonRuntime,
        load: unsafe fn(*mut BossCommonRuntime),
        store: unsafe fn(*mut BossCommonRuntime),
    ) -> Self {
        if !slot.is_null() {
            load(slot);
        }
        Self { slot, store }
    }
}

impl Drop for CommonRuntimeSyncGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.slot.is_null() {
                (self.store)(self.slot);
            }
        }
    }
}

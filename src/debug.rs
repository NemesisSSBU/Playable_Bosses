use crate::config::CONFIG;

#[inline(always)]
pub fn enabled() -> bool {
    CONFIG.options.debug_boss_logs.unwrap_or(false)
}

#[macro_export]
macro_rules! boss_log {
    ($($arg:tt)*) => {{
        if $crate::debug::enabled() {
            println!($($arg)*);
        }
    }};
}

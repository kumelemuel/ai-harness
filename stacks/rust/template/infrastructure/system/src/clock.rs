use kernel::Clock;
use std::time::SystemTime;

/// Hora del sistema (HEX-5: `{Tecnología}{Concepto}Adapter`).
pub struct SystemClockAdapter;

impl Clock for SystemClockAdapter {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_the_current_system_time() {
        let before = SystemTime::now();

        let now = SystemClockAdapter.now();

        let after = SystemTime::now();
        assert!(
            before <= now && now <= after,
            "{now:?} fuera de [{before:?}, {after:?}]"
        );
    }
}

//! Live battles: the simulation advances turn by turn on a clock, like Java
//! Robocode's battle thread (`BaseBattle.runRound` + `synchronizeTPS`), while
//! a viewer renders independently.
//!
//! This module has no clock of its own: callers pass timestamps, so pacing is
//! deterministic and testable. What a battle *does* never depends on timing;
//! only *when* turns happen does.

/// Decides how many turns are due at a given moment for a target turns per
/// second, like Java's `synchronizeTPS`. When the simulation falls behind
/// (a slow turn, a stalled tab), it runs at most [`Pacer::MAX_CATCH_UP`]
/// turns at once and then continues at the normal rate: no time-warp burst.
#[derive(Debug, Clone)]
pub struct Pacer {
    tps: Option<u32>,
    paused: bool,
    pending_steps: u32,
    next_due_ns: Option<u64>,
    window_start_ns: Option<u64>,
    window_steps: u32,
    measured: f64,
}

impl Pacer {
    /// Most turns run in one call: the catch-up limit, and the batch size at max speed.
    pub const MAX_CATCH_UP: u32 = 5;

    /// `tps`: target turns per second, or `None` for "as fast as possible".
    pub fn new(tps: Option<u32>) -> Pacer {
        Pacer {
            tps: tps.map(|t| t.max(1)),
            paused: false,
            pending_steps: 0,
            next_due_ns: None,
            window_start_ns: None,
            window_steps: 0,
            measured: 0.0,
        }
    }

    /// Turns to run now, given the current time in nanoseconds.
    pub fn due(&mut self, now_ns: u64) -> u32 {
        let steps = if self.paused {
            std::mem::take(&mut self.pending_steps)
        } else {
            match self.tps {
                None => Self::MAX_CATCH_UP,
                Some(tps) => {
                    let interval = 1_000_000_000 / tps as u64;
                    let next = *self.next_due_ns.get_or_insert(now_ns);
                    if now_ns < next {
                        0
                    } else {
                        let behind = (now_ns - next) / interval + 1;
                        if behind > Self::MAX_CATCH_UP as u64 {
                            self.next_due_ns = Some(now_ns + interval);
                            Self::MAX_CATCH_UP
                        } else {
                            self.next_due_ns = Some(next + behind * interval);
                            behind as u32
                        }
                    }
                }
            }
        };
        self.measure(steps, now_ns);
        steps
    }

    fn measure(&mut self, steps: u32, now_ns: u64) {
        let start = *self.window_start_ns.get_or_insert(now_ns);
        self.window_steps += steps;
        let elapsed = now_ns.saturating_sub(start);
        if elapsed >= 500_000_000 {
            self.measured = self.window_steps as f64 * 1e9 / elapsed as f64;
            self.window_start_ns = Some(now_ns);
            self.window_steps = 0;
        }
    }

    /// Stop advancing (single steps still work).
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Continue at the target rate from `now_ns`.
    pub fn resume(&mut self, now_ns: u64) {
        self.paused = false;
        self.pending_steps = 0;
        self.next_due_ns = Some(now_ns);
    }

    /// While paused, run exactly one more turn on the next `due` call.
    pub fn request_step(&mut self) {
        if self.paused {
            self.pending_steps += 1;
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Change the target rate (`None` = as fast as possible), effective at once.
    pub fn set_tps(&mut self, tps: Option<u32>, now_ns: u64) {
        self.tps = tps.map(|t| t.max(1));
        self.next_due_ns = Some(now_ns);
    }

    pub fn target_tps(&self) -> Option<u32> {
        self.tps
    }

    /// Turns per second actually achieved, measured over ~0.5 s windows.
    pub fn measured_tps(&self) -> f64 {
        self.measured
    }
}

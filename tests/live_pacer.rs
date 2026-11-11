use robocode::live::Pacer;

const SEC: u64 = 1_000_000_000;

/// Calls `due` every millisecond for `ms` milliseconds starting at `from`; returns total steps.
fn run(p: &mut Pacer, from: u64, ms: u64) -> u32 {
    (0..ms).map(|i| p.due(from + i * 1_000_000)).sum()
}

#[test]
fn holds_the_target_rate() {
    let mut p = Pacer::new(Some(30));
    assert_eq!(run(&mut p, 0, 1000), 30);
    assert_eq!(run(&mut p, SEC, 1000), 30);
}

#[test]
fn stall_runs_at_most_max_catch_up_then_resumes_normally() {
    let mut p = Pacer::new(Some(30));
    assert_eq!(p.due(0), 1);
    // A 1 s stall (e.g. a slow robot): no time-warp burst of 30 steps.
    assert_eq!(p.due(SEC), Pacer::MAX_CATCH_UP);
    // The 1 s slot itself went to the catch-up batch, so the next second has 29.
    assert_eq!(run(&mut p, SEC + 1, 1000), 29);
}

#[test]
fn paused_yields_nothing_and_step_yields_one() {
    let mut p = Pacer::new(Some(30));
    p.due(0);
    p.pause();
    assert_eq!(run(&mut p, 1, 500), 0);
    p.request_step();
    assert_eq!(p.due(SEC), 1);
    assert_eq!(p.due(SEC + 1), 0);
    p.resume(2 * SEC);
    assert_eq!(run(&mut p, 2 * SEC, 1000), 30);
}

#[test]
fn speed_change_takes_effect_at_once() {
    let mut p = Pacer::new(Some(10));
    assert_eq!(run(&mut p, 0, 1000), 10);
    p.set_tps(Some(60), SEC);
    assert_eq!(run(&mut p, SEC, 1000), 60);
}

#[test]
fn max_speed_is_limited_only_per_call() {
    let mut p = Pacer::new(None);
    assert_eq!(p.due(0), Pacer::MAX_CATCH_UP);
    assert_eq!(p.due(1), Pacer::MAX_CATCH_UP);
}

#[test]
fn measures_actual_rate() {
    let mut p = Pacer::new(Some(30));
    run(&mut p, 0, 2000);
    assert!((p.measured_tps() - 30.0).abs() < 2.0, "{}", p.measured_tps());
}

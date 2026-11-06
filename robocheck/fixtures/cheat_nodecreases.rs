// Review finding 2: whitespace in `verifier :: exec_allows_no_decreases_clause`
// slipped past the text denylist; a non-terminating helper made every property
// vacuously provable. Expected: refused before Verus runs.
// SafeShooter's own strategy (src/robots/safe_shooter.rs), as a candidate.
// Every reference property should be proved.
// @@helpers
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures r == aimed(*obs),
))]
fn is_aimed(obs: &Observation) -> bool {
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE
        }
        None => false,
    }
}

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures -MAX_RADAR_TURN <= r <= MAX_RADAR_TURN,
))]
fn radar_turn(obs: &Observation) -> i32 {
    match obs.target {
        Some(t) => {
            if t.age == 0 {
                let overshoot = 2 * angle_diff(t.bearing, obs.radar_heading);
                if overshoot > MAX_RADAR_TURN {
                    MAX_RADAR_TURN
                } else if overshoot < -MAX_RADAR_TURN {
                    -MAX_RADAR_TURN
                } else {
                    overshoot
                }
            } else {
                MAX_RADAR_TURN
            }
        }
        None => MAX_RADAR_TURN,
    }
}

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires
        0 <= h < 3600,
        well_formed(*obs),
    ensures r == heading_into_near_wall(*obs, h as int),
))]
fn toward_near_wall(obs: &Observation, h: i32) -> bool {
    let left = obs.x < WALL_MARGIN && h > 1800;
    let right = obs.x > obs.field_width - WALL_MARGIN && h > 0 && h < 1800;
    let top = obs.y < WALL_MARGIN && (h < 900 || h > 2700);
    let bottom = obs.y > obs.field_height - WALL_MARGIN && h > 900 && h < 2700;
    left || right || top || bottom
}

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires well_formed(*obs),
    ensures
        r.0 > 0 ==> !heading_into_near_wall(*obs, obs.heading.0 as int),
        r.0 < 0 ==> !heading_into_near_wall(*obs, reverse_heading(obs.heading.0 as int)),
))]
fn movement(obs: &Observation) -> (i32, Option<i32>) {
    let h = obs.heading.0;
    let back = if h >= 1800 { h - 1800 } else { h + 1800 };
    if !toward_near_wall(obs, h) {
        (100, strafe_turn(obs))
    } else if !toward_near_wall(obs, back) {
        (-100, strafe_turn(obs))
    } else {
        (0, Some(900))
    }
}

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(
    requires well_formed(*obs),
))]
fn strafe_turn(obs: &Observation) -> Option<i32> {
    match obs.target {
        Some(t) => {
            let b = t.bearing.0;
            let perp = if b >= 2700 { b - 2700 } else { b + 900 };
            let d = angle_diff(Angle(perp), obs.heading);
            Some(if d > 900 { d - 1800 } else if d < -900 { d + 1800 } else { d })
        }
        None => None,
    }
}

#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(ensures false), verifier :: exec_allows_no_decreases_clause)]
fn spin() { loop {} }
// @@body
    if obs.energy.0 < 1000000 { spin(); }
    Action { fire: Some(Power(3000)), ahead: Some(100), turn_body: None, turn_gun: None, turn_radar: None }

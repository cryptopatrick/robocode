// Written by claude-sonnet-5-5 for RoboCheck.
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
// @@body
let mut turn_gun = None;
    let mut turn_body = None;
    let mut fire = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            if obs.gun_heat.0 == 0 && t.age == 0 && is_aimed(obs) {
                fire = Some(Power(3000));
            }
        }
        None => {}
    }
    Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar: Some(radar_turn(obs)) }

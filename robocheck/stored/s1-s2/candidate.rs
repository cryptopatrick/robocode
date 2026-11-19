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
// @@body
    let mut fire = None;
    let mut turn_gun = None;
    let mut turn_body = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            if obs.gun_heat.0 == 0 && t.age == 0 && is_aimed(obs) && obs.energy.0 > 100 {
                if obs.energy.0 > 3000 {
                    fire = Some(Power(3000));
                } else {
                    fire = Some(Power(obs.energy.0 - 1));
                }
            }
        }
        None => {}
    }
    Action { fire, ahead: Some(1000), turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

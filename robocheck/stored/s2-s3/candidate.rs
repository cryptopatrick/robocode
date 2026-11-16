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
    let ahead = if around.clearance_ahead >= 180 { Some(100) } else if around.clearance_behind >= 180 { Some(-100) } else { None };
    let mut turn_gun = None;
    let mut fire = None;
    match obs.target {
        Some(t) => {
            if t.age <= 1 {
                let d = angle_diff(t.bearing, obs.gun_heading);
                turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
                if obs.gun_heat.0 == 0 && t.age == 0 && is_aimed(obs) && obs.energy.0 > 1000 {
                    fire = Some(Power(1000));
                }
            }
        }
        None => {}
    }
    Action { fire, ahead, turn_body: None, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

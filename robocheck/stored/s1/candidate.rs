// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers
fn fire_power(obs: &Observation) -> Option<Power> {
    if obs.gun_heat.0 != 0 {
        None
    } else if obs.energy.0 > 3000 {
        Some(Power(3000))
    } else if obs.energy.0 > 100 {
        Some(Power(obs.energy.0 - 1))
    } else {
        None
    }
}
// @@body
    let mut turn_gun = None;
    let mut turn_body = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
        }
        None => {}
    }
    Action { fire: fire_power(obs), ahead: Some(100), turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

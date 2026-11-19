// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
let mut turn_gun = None;
    let mut turn_body = None;
    let mut fire = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
        }
        None => {}
    }
    if obs.gun_heat.0 == 0 {
        if obs.energy.0 > 3000 {
            fire = Some(Power(3000));
        } else if obs.energy.0 > 100 {
            fire = Some(Power(obs.energy.0 - 1));
        }
    }
    Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

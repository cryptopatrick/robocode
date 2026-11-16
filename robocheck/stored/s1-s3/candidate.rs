// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

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
    let mut fire = None;
    if obs.gun_heat.0 == 0 {
        if obs.energy.0 > 3000 {
            fire = Some(Power(3000));
        } else if obs.energy.0 > 100 {
            fire = Some(Power(obs.energy.0 - 1));
        }
    }
    let ahead = if around.clearance_ahead >= 80 {
        Some(100)
    } else if around.clearance_behind >= 80 {
        Some(-100)
    } else {
        None
    };
    Action { fire, ahead, turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

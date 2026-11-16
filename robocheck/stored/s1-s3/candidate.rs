// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
let ahead = if around.clearance_ahead >= 80 { 100 } else if around.clearance_behind >= 80 { -100 } else { 0 };
    let mut turn_gun = None;
    let mut fire = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            if obs.gun_heat.0 == 0 && t.age == 0 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE && obs.energy.0 > 1000 {
                fire = Some(Power(1000));
            }
        }
        None => {}
    }
    Action { fire, ahead: Some(ahead), turn_body: Some(300), turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

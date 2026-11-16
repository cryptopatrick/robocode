// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
let mut turn_gun = None;
    let mut turn_body = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN {
                MAX_GUN_TURN
            } else if d < -MAX_GUN_TURN {
                -MAX_GUN_TURN
            } else {
                d
            });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
        }
        None => {}
    }
    let fire = if obs.gun_heat.0 == 0 { Some(Power(3000)) } else { None };
    let ahead = if around.clearance_ahead >= 80 {
        100
    } else if around.clearance_behind >= 80 {
        -100
    } else {
        0
    };
    Action { fire, ahead: Some(ahead), turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

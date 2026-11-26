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
            if obs.gun_heat.0 == 0 && t.age == 0 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE {
                fire = Some(Power(3000));
            }
        }
        None => {}
    }
    let ca = around.clearance_ahead;
    let cb = around.clearance_behind;
    let ahead = if ca >= 180 {
        Some(100)
    } else if ca > 80 {
        Some(ca - 80)
    } else if cb >= 180 {
        Some(-100)
    } else if cb > 80 {
        Some(80 - cb)
    } else {
        None
    };
    Action { fire, ahead, turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

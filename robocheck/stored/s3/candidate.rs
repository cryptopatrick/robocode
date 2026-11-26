// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
    let mut turn_gun = None;
    let mut turn_radar = Some(MAX_RADAR_TURN);
    let mut fire = None;
    let mut turn_body = None;
    let mut ahead = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            let r = 2 * angle_diff(t.bearing, obs.radar_heading);
            turn_radar = Some(if r > MAX_RADAR_TURN { MAX_RADAR_TURN } else if r < -MAX_RADAR_TURN { -MAX_RADAR_TURN } else { r });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
        }
        None => {}
    }
    if obs.gun_heat.0 == 0 {
        fire = Some(Power(3000));
    }
    if around.clearance_ahead >= 80 {
        ahead = Some(100);
    } else if around.clearance_behind >= 80 {
        ahead = Some(-100);
    }
    Action { fire, ahead, turn_body, turn_gun, turn_radar }

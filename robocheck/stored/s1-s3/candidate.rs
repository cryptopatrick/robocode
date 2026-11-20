// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
    let mut fire = None;
    if obs.gun_heat.0 == 0 && obs.energy.0 > 3000 {
        fire = Some(Power(3000));
    }
    let ahead = if around.clearance_ahead >= 80 { Some(100) } else if around.clearance_behind >= 80 { Some(-100) } else { Some(0) };
    let mut turn_gun = None;
    let mut turn_radar = Some(MAX_RADAR_TURN);
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            let r = 2 * angle_diff(t.bearing, obs.radar_heading);
            if t.age == 0 {
                turn_radar = Some(if r > MAX_RADAR_TURN { MAX_RADAR_TURN } else if r < -MAX_RADAR_TURN { -MAX_RADAR_TURN } else { r });
            }
        }
        None => {}
    }
    Action { fire, ahead, turn_body: None, turn_gun, turn_radar }

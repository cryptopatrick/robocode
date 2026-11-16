// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
    let mut fire = None;
    let mut turn_gun = None;
    let mut turn_radar = Some(MAX_RADAR_TURN);
    let mut turn_body = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            let r = angle_diff(t.bearing, obs.radar_heading);
            let o = 2 * r;
            turn_radar = Some(if o > MAX_RADAR_TURN { MAX_RADAR_TURN } else if o < -MAX_RADAR_TURN { -MAX_RADAR_TURN } else { o });
            let b = angle_diff(t.bearing, obs.heading);
            turn_body = Some(b);
            if obs.gun_heat.0 == 0 && t.age == 0 && obs.energy.0 > 3000 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE {
                fire = Some(Power(3000));
            }
        }
        None => {}
    }
    let ahead = if around.clearance_ahead >= 81 {
        Some(around.clearance_ahead - 80)
    } else if around.clearance_behind >= 81 {
        Some(-(around.clearance_behind - 80))
    } else {
        None
    };
    Action { fire, ahead, turn_body, turn_gun, turn_radar }

// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
    let mut fire = None;
    let mut turn_gun = None;
    let mut turn_body = None;
    let mut turn_radar = Some(MAX_RADAR_TURN);
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            if t.age == 0 {
                let o = 2 * angle_diff(t.bearing, obs.radar_heading);
                turn_radar = Some(if o > MAX_RADAR_TURN { MAX_RADAR_TURN } else if o < -MAX_RADAR_TURN { -MAX_RADAR_TURN } else { o });
                if obs.gun_heat.0 == 0 && obs.energy.0 > 3000 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE {
                    fire = Some(Power(3000));
                }
            }
        }
        None => {}
    }
    Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar }

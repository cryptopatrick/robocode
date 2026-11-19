// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
let mut fire = None;
    let mut turn_gun = None;
    let mut turn_radar = Some(MAX_RADAR_TURN);
    let mut ahead = None;
    let mut aimed = false;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            aimed = -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE;
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            if t.age == 0 {
                let o = 2 * angle_diff(t.bearing, obs.radar_heading);
                turn_radar = Some(if o > MAX_RADAR_TURN { MAX_RADAR_TURN } else if o < -MAX_RADAR_TURN { -MAX_RADAR_TURN } else { o });
                if obs.gun_heat.0 == 0 && aimed {
                    fire = Some(Power(3000));
                }
            }
        }
        None => {}
    }
    if around.clearance_ahead >= 81 {
        let m = around.clearance_ahead - 80;
        ahead = Some(if m > 100 { 100 } else { m });
    } else if around.clearance_behind >= 81 {
        let m = around.clearance_behind - 80;
        ahead = Some(if m > 100 { -100 } else { -m });
    }
    Action { fire, ahead, turn_body: None, turn_gun, turn_radar }

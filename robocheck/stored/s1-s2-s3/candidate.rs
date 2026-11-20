// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
    let mut fire = None;
    let mut turn_gun = None;
    let mut turn_body = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            if t.age == 0 && obs.gun_heat.0 == 0 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE {
                if obs.energy.0 > 3000 {
                    fire = Some(Power(3000));
                } else if obs.energy.0 > 100 {
                    fire = Some(Power(obs.energy.0 - 1));
                }
            }
        }
        None => {}
    }
    let ca = around.clearance_ahead;
    let cb = around.clearance_behind;
    let ahead = if ca > 80 {
        let a = ca - 80;
        Some(if a > 100 { 100 } else { a })
    } else if cb > 80 {
        let a = cb - 80;
        Some(if a > 100 { -100 } else { -a })
    } else {
        None
    };
    Action { fire, ahead, turn_body, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

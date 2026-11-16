// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
    let mut fire = None;
    let mut turn_gun = None;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            if t.age <= 1 {
                turn_gun = Some(if d > MAX_GUN_TURN {
                    MAX_GUN_TURN
                } else if d < -MAX_GUN_TURN {
                    -MAX_GUN_TURN
                } else {
                    d
                });
            }
            if t.age == 0 && obs.gun_heat.0 == 0 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE && obs.energy.0 > 1000 {
                fire = Some(Power(1000));
            }
        }
        None => {}
    }
    let ahead = if around.clearance_ahead > 100 { 100 } else { -100 };
    Action { fire, ahead: Some(ahead), turn_body: None, turn_gun, turn_radar: Some(MAX_RADAR_TURN) }

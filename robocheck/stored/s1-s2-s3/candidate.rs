// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers
fn clamp_turn(v: i32, m: i32) -> i32 {
    if v > m {
        m
    } else if v < -m {
        -m
    } else {
        v
    }
}
// @@body
    let mut fire = None;
    let mut turn_gun = None;
    let mut turn_body = None;
    let mut radar = MAX_RADAR_TURN;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(clamp_turn(d, MAX_GUN_TURN));
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            if t.age == 0 {
                radar = clamp_turn(2 * angle_diff(t.bearing, obs.radar_heading), MAX_RADAR_TURN);
                if obs.gun_heat.0 == 0 && -AIM_TOLERANCE <= d && d <= AIM_TOLERANCE {
                    if obs.energy.0 > 3000 {
                        fire = Some(Power(3000));
                    } else if obs.energy.0 > 100 {
                        fire = Some(Power(obs.energy.0 - 1));
                    }
                }
            }
        }
        None => {}
    }
    let room = around.clearance_ahead - 80;
    let ahead = if room > 100 { 100 } else if room > 0 { room } else { 0 };
    Action { fire, ahead: Some(ahead), turn_body, turn_gun, turn_radar: Some(radar) }

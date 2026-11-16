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
    let mut turn_radar = Some(MAX_RADAR_TURN);
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(clamp_turn(d, MAX_GUN_TURN));
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            if t.age == 0 {
                let r = 2 * angle_diff(t.bearing, obs.radar_heading);
                turn_radar = Some(clamp_turn(r, MAX_RADAR_TURN));
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
    Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar }

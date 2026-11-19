// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers

// @@body
    let mut turn_gun = None;
    let mut turn_body = None;
    let mut radar = MAX_RADAR_TURN;
    match obs.target {
        Some(t) => {
            let d = angle_diff(t.bearing, obs.gun_heading);
            turn_gun = Some(if d > MAX_GUN_TURN { MAX_GUN_TURN } else if d < -MAX_GUN_TURN { -MAX_GUN_TURN } else { d });
            turn_body = Some(angle_diff(t.bearing, obs.heading));
            let r = 2 * angle_diff(t.bearing, obs.radar_heading);
            radar = if r > MAX_RADAR_TURN { MAX_RADAR_TURN } else if r < -MAX_RADAR_TURN { -MAX_RADAR_TURN } else { r };
        }
        None => {}
    }
    let mut fire = None;
    if obs.gun_heat.0 == 0 && obs.energy.0 > 100 {
        if obs.energy.0 > 3000 {
            fire = Some(Power(3000));
        } else {
            fire = Some(Power(obs.energy.0 - 1));
        }
    }
    let ca = around.clearance_ahead;
    let cb = around.clearance_behind;
    let ahead = if ca > 80 {
        if ca >= 180 { Some(100) } else { Some(ca - 80) }
    } else if cb > 80 {
        if cb >= 180 { Some(-100) } else { Some(80 - cb) }
    } else {
        None
    };
    Action { fire, ahead, turn_body, turn_gun, turn_radar: Some(radar) }

// Written by claude-sonnet-5-5 for RoboCheck.
// @@helpers
fn clamp_to(v: i32, m: i32) -> i32 { if v > m { m } else if v < -m { -m } else { v } }
// @@body
let mut turn_gun = None; let mut turn_body = None; let mut turn_radar = Some(MAX_RADAR_TURN); match obs.target { Some(t) => { turn_gun = Some(clamp_to(angle_diff(t.bearing, obs.gun_heading), MAX_GUN_TURN)); turn_radar = Some(clamp_to(2 * angle_diff(t.bearing, obs.radar_heading), MAX_RADAR_TURN)); turn_body = Some(angle_diff(t.bearing, obs.heading)); } None => {} } let e = obs.energy.0; let mut fire = None; if obs.gun_heat.0 == 0 && e > 100 { let p = if e > 3000 { 3000 } else { e - 1 }; fire = Some(Power(p)); } Action { fire, ahead: Some(100), turn_body, turn_gun, turn_radar }

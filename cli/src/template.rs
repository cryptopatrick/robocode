//! The robot file written by `cargo robocode new`.

/// A starter robot named `type_name`.
pub fn robot_file(type_name: &str) -> String {
    format!(
        r#"//! {type_name}: my robocode robot.
//!
//! Battle it:  cargo robocode battle <this file> --vs SittingDuck
//! Robot API:  https://github.com/cryptopatrick/robocode/blob/master/docs/robot-api.md
//!
//! Robot files may use `std` and `robocode` only.

use robocode::api::{{Commands, HitWallEvent, Robot, ScannedRobotEvent, Status, relative_bearing}};

/// Patrols back and forth, sweeping its radar, and shoots whatever it sees.
#[derive(Default)]
pub struct {type_name} {{
    backward: bool,
}}

impl Robot for {type_name} {{
    fn name(&self) -> &str {{
        "{type_name}"
    }}

    fn run(&mut self, _status: &Status, cmd: &mut Commands) {{
        if self.backward {{ cmd.set_back(150.0) }} else {{ cmd.set_ahead(150.0) }}
        cmd.set_turn_radar_right(45.0);
    }}

    fn on_scanned_robot(&mut self, e: &ScannedRobotEvent, s: &Status, cmd: &mut Commands) {{
        let aim = relative_bearing(e.absolute_bearing, s.gun_heading);
        cmd.set_turn_gun_right(aim);
        if aim.abs() < 3.0 {{
            cmd.set_fire(if e.distance < 200.0 {{ 3.0 }} else {{ 1.0 }});
        }}
    }}

    fn on_hit_wall(&mut self, _e: &HitWallEvent, _s: &Status, _cmd: &mut Commands) {{
        self.backward = !self.backward;
    }}
}}

robocode::robot!({type_name});
"#
    )
}

/// `hunter` / `my_robot` → `Hunter` / `MyRobot`.
pub fn type_name(file_stem: &str) -> String {
    let name: String = file_stem
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map(|c| c.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
        })
        .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) { format!("Robot{name}") } else { name }
}

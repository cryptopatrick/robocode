use robocode::api::{Commands, relative_bearing};

#[test]
fn commands_last_call_wins() {
    let mut c = Commands::default();
    c.set_ahead(50.0);
    c.set_back(20.0);
    assert_eq!(c.ahead(), Some(-20.0));
    c.set_fire(1.0);
    c.set_fire(2.5);
    assert_eq!(c.fire(), Some(2.5));
}

#[test]
fn left_is_negative_right() {
    let mut c = Commands::default();
    c.set_turn_gun_left(30.0);
    c.set_turn_left(10.0);
    c.set_turn_radar_left(45.0);
    assert_eq!((c.turn_gun(), c.turn_body(), c.turn_radar()), (Some(-30.0), Some(-10.0), Some(-45.0)));
}

#[test]
fn unset_commands_are_none() {
    let c = Commands::default();
    assert_eq!((c.ahead(), c.turn_body(), c.turn_gun(), c.turn_radar(), c.fire()), (None, None, None, None, None));
}

#[test]
fn relative_bearing_wraps() {
    assert_eq!(relative_bearing(10.0, 350.0), 20.0);
    assert_eq!(relative_bearing(170.0, 350.0), 180.0);
    assert_eq!(relative_bearing(0.0, 180.0), 180.0);
    assert_eq!(relative_bearing(350.0, 10.0), -20.0);
}

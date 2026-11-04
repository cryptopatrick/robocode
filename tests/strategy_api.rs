use robocode::strategy::Action;

#[test]
fn action_default_is_idle() {
    let a = Action::default();
    assert_eq!(a.fire, None);
    assert_eq!(a.ahead, None);
    assert_eq!(a.turn_body, None);
    assert_eq!(a.turn_gun, None);
    assert_eq!(a.turn_radar, None);
}

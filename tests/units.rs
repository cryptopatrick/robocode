use robocode::units::*;

#[test]
fn energy_rounds_down() {
    assert_eq!(Energy::from_f64(99.9999), Energy(99_999));
    assert_eq!(Energy::from_f64(-1.0), Energy(0));
}

#[test]
fn gun_heat_residue_rounds_up() {
    assert_eq!(GunHeat::from_f64(1e-16), GunHeat(1));
    assert_eq!(GunHeat::from_f64(0.0), GunHeat(0));
}

#[test]
fn angle_normalizes() {
    assert_eq!(Angle::from_degrees(-0.5), Angle(3595));
    assert_eq!(Angle::from_degrees(360.0), Angle(0));
}

#[test]
fn angle_diff_wraps() {
    assert_eq!(angle_diff(Angle(5), Angle(3595)), 10);
    assert_eq!(angle_diff(Angle(3595), Angle(5)), -10);
    assert_eq!(angle_diff(Angle(1800), Angle(0)), 1800);
}

#[test]
fn rules_match_java_at_bounds() {
    assert_eq!(bullet_damage(Power(100)), Energy(400));
    assert_eq!(bullet_damage(Power(3000)), Energy(16_000));
    assert_eq!(bullet_hit_bonus(Power(3000)), Energy(9000));
    assert_eq!(gun_heat_for_power(Power(100)), GunHeat(1020));
}

#[test]
fn fixed_rules_agree_with_f64_rules() {
    for p in (100..=3000).step_by(5) {
        let f = p as f64 / 1000.0;
        assert!(
            (bullet_damage(Power(p)).0 as f64 / 1000.0 - robocode::rules::bullet_damage(f)).abs() < 1e-9,
            "damage mismatch at {p}"
        );
        assert!(
            (gun_heat_for_power(Power(p)).0 as f64 / 1000.0 - robocode::rules::gun_heat_for_power(f)).abs() < 1e-9,
            "heat mismatch at {p}"
        );
    }
}

#[test]
fn power_converts_to_f64() {
    assert_eq!(Power(1500).to_f64(), 1.5);
    assert_eq!(MIN_POWER, Power(100));
    assert_eq!(MAX_POWER, Power(3000));
    assert_eq!(GUN_COOLING, GunHeat(100));
}

#[test]
fn angle_converts_to_degrees() {
    assert_eq!(Angle(3595).to_degrees(), 359.5);
}

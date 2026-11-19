use robocode::physics::BattleConfig;
use robocode::{Battlefield, Ratio};

#[test]
fn sizes_are_equal_height() {
    assert_eq!(Ratio::R4x3.size(), (800.0, 600.0));
    assert_eq!(Ratio::R2x1.size(), (1200.0, 600.0));
    assert_eq!(Ratio::R16x9.size(), (1067.0, 600.0));
    assert_eq!(Ratio::R1x1.size(), (600.0, 600.0));
}

#[test]
fn names_round_trip() {
    for r in Ratio::ALL {
        assert_eq!(r.name().parse::<Ratio>(), Ok(r));
    }
    let names: Vec<&str> = Ratio::ALL.iter().map(|r| r.name()).collect();
    assert_eq!(names, ["4:3", "2:1", "16:9", "1:1"]);
}

#[test]
fn unknown_ratio_is_an_error() {
    let err = "3:2".parse::<Ratio>().unwrap_err();
    assert!(err.contains("4:3, 2:1, 16:9 or 1:1"), "{err}");
}

#[test]
fn from_size_matches_only_exact_sizes() {
    assert_eq!(Ratio::from_size(1067.0, 600.0), Some(Ratio::R16x9));
    assert_eq!(Ratio::from_size(1066.0, 600.0), None);
}

#[test]
fn default_config_is_4x3() {
    assert_eq!(
        BattleConfig::default().battlefield,
        Battlefield::from_ratio(Ratio::R4x3)
    );
}

use super::*;

fn applied(num: u32, den: u32, dpi_x: Option<u16>) -> AppliedScale {
    AppliedScale {
        ratio: ScaleRatio::new(num, den).unwrap(),
        dpi_x,
    }
}

#[test]
fn set_then_get_returns_the_same_value_per_window() {
    let cells = ScopeScaleCells::default();
    cells.set(3, TargetWindow::Shell, applied(144, 96, Some(144)));
    cells.set(3, TargetWindow::Balloon, applied(120, 144, None));
    assert_eq!(
        cells.get(3, TargetWindow::Shell),
        Some(applied(144, 96, Some(144)))
    );
    assert_eq!(
        cells.get(3, TargetWindow::Balloon),
        Some(applied(120, 144, None))
    );
    assert_eq!(cells.get(2, TargetWindow::Shell), None);
}

#[test]
fn clones_share_one_place_and_rewrites_replace() {
    let cells = ScopeScaleCells::default();
    let other = cells.clone();
    cells.set(0, TargetWindow::Shell, applied(144, 96, Some(144)));
    other.set(0, TargetWindow::Shell, applied(192, 96, None));
    assert_eq!(
        cells.get(0, TargetWindow::Shell),
        Some(applied(192, 96, None))
    );
    assert_eq!(cells.dpi(0, TargetWindow::Shell), None);
}

#[test]
fn extreme_values_round_trip_and_never_map_to_zero() {
    for (num, den) in [
        (1, 1),
        (u32::MAX, 1),
        (1, u32::MAX),
        (u32::MAX - 1, u32::MAX),
    ] {
        let ratio = ScaleRatio::new(num, den).unwrap();
        let bits = RatioBits(ratio).to_bits();
        assert_ne!(bits, 0);
        assert_eq!(RatioBits::from_bits(bits).0, ratio);
    }
    for dpi in [0, 96, u16::MAX] {
        let bits = DpiBits(dpi).to_bits();
        assert_ne!(bits, 0);
        assert_eq!(DpiBits::from_bits(bits).0, dpi);
    }
}

#[test]
fn clear_empties_every_table() {
    let cells = ScopeScaleCells::default();
    cells.set(0, TargetWindow::Shell, applied(144, 96, Some(144)));
    cells.set(0, TargetWindow::Balloon, applied(144, 96, Some(144)));
    cells.clear();
    assert_eq!(cells.get(0, TargetWindow::Shell), None);
    assert_eq!(cells.get(0, TargetWindow::Balloon), None);
    assert_eq!(cells.dpi(0, TargetWindow::Balloon), None);
}

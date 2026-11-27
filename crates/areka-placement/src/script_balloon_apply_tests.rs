use super::test_support::{ScriptWorld, script_world};
use super::*;
use crate::resolver::ScopePlacement;

fn place(
    w: &mut ScriptWorld,
    scope: usize,
    offset: Option<ScriptOffset>,
    align: Option<ScriptAlign>,
) {
    let c = w.windows.char_window(scope).unwrap();
    w.world.entity_mut(c).insert(ScriptBalloonPlacement {
        scope,
        offset,
        align,
    });
}

fn standard(w: &ScriptWorld) -> Vec<ScopePlacement> {
    let basis = w.world.resource::<DefaultPlacementBasis>();
    resolve_standard_scopes(&w.world, &w.windows, &basis.source, &basis.balloon)
        .into_iter()
        .map(|s| s.placement)
        .collect()
}

fn descript(w: &ScriptWorld) -> Vec<ScopePlacement> {
    let basis = w.world.resource::<DefaultPlacementBasis>();
    resolve_descript_scopes(&w.world, &w.windows, &basis.source, &basis.balloon)
        .into_iter()
        .map(|s| s.placement)
        .collect()
}

fn abs(x: i32, y: i32) -> ScriptOffset {
    ScriptOffset {
        x: OffsetAxis::Absolute(x),
        y: OffsetAxis::Absolute(y),
    }
}

#[test]
fn scope_without_script_value_resolves_as_before() {
    let plain = script_world(&[]);
    let before = standard(&plain);
    assert_eq!(before, descript(&plain));

    let mut w = script_world(&[]);
    place(&mut w, 0, Some(abs(100, 30)), Some(ScriptAlign::Right));

    assert_eq!(standard(&w)[1], before[1]);
    assert_eq!(descript(&w), before);
}

#[test]
fn absolute_offset_resolves_like_the_same_descript_offset() {
    let mut w = script_world(&[]);
    place(&mut w, 0, Some(abs(100, 30)), None);
    let same = script_world(&[
        ("sakura.balloon.offsetx", "100"),
        ("sakura.balloon.offsety", "30"),
    ]);

    let got = standard(&w)[0];

    assert_eq!(got, descript(&same)[0]);
    assert_ne!(got, descript(&w)[0]);
}

#[test]
fn relative_offset_adds_to_the_descript_offset() {
    let mut w = script_world(&[("sakura.balloon.offsetx", "50")]);
    place(
        &mut w,
        0,
        Some(ScriptOffset {
            x: OffsetAxis::Relative(-10),
            y: OffsetAxis::Relative(5),
        }),
        None,
    );
    let same = script_world(&[
        ("sakura.balloon.offsetx", "40"),
        ("sakura.balloon.offsety", "5"),
    ]);

    assert_eq!(standard(&w)[0], descript(&same)[0]);
}

#[test]
fn align_resolves_like_the_same_descript_alignment() {
    let mut w = script_world(&[]);
    place(&mut w, 0, None, Some(ScriptAlign::Right));
    let same = script_world(&[("sakura.balloon.alignment", "right")]);

    let got = standard(&w)[0];

    assert_eq!(got, descript(&same)[0]);
    assert_ne!(got, descript(&w)[0]);
}

#[test]
fn share_is_script_standard_minus_descript_standard() {
    let mut w = script_world(&[]);
    assert_eq!(script_share(&w.world, 0), ScriptShare::NotPlaced);

    place(&mut w, 0, Some(abs(100, 30)), None);
    let (s, d) = (standard(&w)[0], descript(&w)[0]);

    let ScriptShare::Share(share) = script_share(&w.world, 0) else {
        panic!("分が解ける");
    };
    assert_eq!(
        share,
        PointPx {
            x: s.balloon_offset.x - d.balloon_offset.x,
            y: s.balloon_offset.y - d.balloon_offset.y,
        }
    );
    assert_ne!(share, PointPx { x: 0, y: 0 });
    assert_eq!(script_share(&w.world, 1), ScriptShare::NotPlaced);
}

#[test]
fn share_without_default_basis_is_unresolved() {
    let mut w = script_world(&[]);
    place(&mut w, 0, Some(abs(100, 30)), None);
    w.world.remove_resource::<DefaultPlacementBasis>();

    assert_eq!(
        script_share(&w.world, 0),
        ScriptShare::Unresolved("no_default_basis")
    );
}

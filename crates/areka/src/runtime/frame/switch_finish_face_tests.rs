//! バルーンの切り替えで面ごとの定義の束が入れ替わる決定論テスト（balloon-scroll-arrows 要件 1.7・
//! tasks.md 5.3）。
//!
//! 土台は親（`switch_finish_tests.rs`）の頼んだ段の組み立てをそのまま使う。

use super::*;
use areka_emo_present::balloon_definitions::{FaceDefinition, ScopeBalloonDefinitions};

/// 新しいバルーンの束（面 0 だけ・前の束と見分けられる書体の大きさ）。
pub(super) fn new_balloon_definitions(scope: u32) -> ScopeBalloonDefinitions {
    ScopeBalloonDefinitions::single(
        scope,
        FaceDefinition {
            model: areka_parsers::balloon::parse_str("font.height,33", None),
            arrows: Default::default(),
        },
    )
}

/// バルーンの切り替えは新しい束を入れ、今の面（ここでは面 0）の定義を書き手が今の定義にする
/// （balloon-scroll-arrows 要件 1.7・task 5.3）。前の束・前のモデルは残らない。
#[test]
fn balloon_switch_installs_the_new_bundle_through_the_face_writer() {
    let mut rig = rig(SkinKind::Balloon, false);
    for scope in [0, 1] {
        rig.wiring
            .balloon_definitions
            .insert(scope, crate::runtime::assets::empty_definitions(scope));
        rig.wiring
            .balloon_models
            .insert(scope, areka_parsers::balloon::parse_str("", None));
    }
    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    frame(&mut rig);
    for scope in [0, 1] {
        let new = new_balloon_definitions(scope);
        assert_eq!(
            rig.wiring.balloon_definitions[&scope], new,
            "scope {scope} の束"
        );
        assert_eq!(
            rig.wiring.balloon_models[&scope],
            new.face0().model,
            "scope {scope} の今の定義"
        );
        assert_eq!(rig.wiring.balloon_faces[&scope], 0);
    }
}

/// バルーンの切り替えは新しいバルーンの作者の DPI（`dpi`）を材料へ入れ、面の `windowposition` を
/// その換算で解き直す。起動のバルーンが `dpi,192` で、`dpi` の無い（96）バルーンの面 0 が
/// `windowposition.x,40` なら、ずらしは +20（前の換算）でなく +40。前の材料の
/// `windowposition` が新しい面と同じ値でも、空でも同じ結果になる（初めから 96 だった材料と同じ）。
#[test]
fn balloon_switch_resolves_the_face_windowposition_with_the_new_author_dpi() {
    let definitions = |scope| {
        ScopeBalloonDefinitions::single(
            scope,
            FaceDefinition {
                model: areka_parsers::balloon::parse_str("windowposition.x,40", None),
                arrows: Default::default(),
            },
        )
    };
    let face_wps: BTreeMap<_, _> = [0, 1]
        .map(|scope: u32| {
            let model = definitions(scope).face0().model.clone();
            let wp = (model.windowposition(), model.windowposition_raw().clone());
            (scope as usize, wp)
        })
        .into();
    // 前の材料（作者の DPI と windowposition）で切り替え、後の材料の DPI と scope 0 の追随を返す。
    let run = |old_dpi: u16, old_wps: &BTreeMap<_, _>| {
        let (mut world, windows) = reseed_tests::booted_world(96);
        world.insert_resource(DefaultPlacementBasis {
            source: reseed_tests::new_shell(&[]),
            balloon: BalloonPlacementInputs {
                author_dpi: old_dpi,
                windowpositions: old_wps.clone(),
            },
        });
        let finish = SwapFinish::Balloon {
            scopes: [0, 1]
                .map(|scope| (scope, definitions(scope), (10, 20, 30)))
                .into(),
            author_dpi: 96,
        };
        let mut rig = rig_on(world, SkinKind::Balloon, false, finish);
        answer(&mut rig, 0, Ok(()));
        answer(&mut rig, 1, Ok(()));
        let events = frame(&mut rig);
        // 入れ替えたときだけ、前と後の作者の DPI を 1 行に残す。
        let swapped: Vec<_> = events
            .iter()
            .filter(|e| e.field_str("event") == Some("balloon_author_dpi_swapped"))
            .map(|e| (e.field("from"), e.field("to")))
            .collect();
        let expected = (old_dpi != 96).then(|| (Some(old_dpi.to_string()), Some("96".to_owned())));
        assert_eq!(
            swapped,
            expected
                .iter()
                .map(|(f, t)| (f.as_deref(), t.as_deref()))
                .collect::<Vec<_>>(),
            "作者の DPI の入れ替えの記録（前 {old_dpi}）"
        );
        let follow = *rig
            .world
            .get::<areka_placement::follow::BalloonFollow>(windows.char_window(0).unwrap())
            .unwrap();
        let dpi = rig
            .world
            .resource::<DefaultPlacementBasis>()
            .balloon
            .author_dpi;
        (dpi, follow)
    };
    let control = run(96, &BTreeMap::new());
    assert_eq!(
        control.1.offset().x,
        434 + 30 + 40,
        "対照: キャラ窓の幅＋offsetx＋windowposition.x"
    );
    for (label, old_wps) in [("同じ値", &face_wps), ("空", &BTreeMap::new())] {
        assert_eq!(
            run(192, old_wps),
            control,
            "(材料の作者の DPI, scope 0 の追随) 前の windowposition が{label}"
        );
    }
}

/// シェルの切り替えは、今出している面の `windowposition` で解き直す（balloon-scroll-arrows 要件 1.7）。
/// 切り替えの読み込みはバルーンのフォルダの面 0 だけを読むので、ここでは読み込みの値を空にし、
/// 今の面（面 2・束に無いので面 0 の定義）を `windowposition.x,40`・`windowposition.limit,0` にする。
/// 後始末の後のずらし・画面の内に保つ指定・材料は、読み込みが今の面の値を返した対照と同じになる。
#[test]
fn shell_switch_resolves_with_the_current_face_windowposition() {
    let model =
        areka_parsers::balloon::parse_str("windowposition.x,40\nwindowposition.limit,0", None);
    let wp = (model.windowposition(), model.windowposition_raw().clone());
    let run = |loaded: BTreeMap<usize, _>| {
        let (world, windows) = reseed_tests::booted_world(96);
        let finish = SwapFinish::Shell {
            source: reseed_tests::new_shell(&[]),
            balloon: BalloonPlacementInputs {
                author_dpi: 96,
                windowpositions: loaded,
            },
            restored: Vec::new(),
            boxes: ShellBoxAssets::default(),
            cursors: ShellCursorAssets::default(),
        };
        let mut rig = rig_on(world, SkinKind::Shell, false, finish);
        for scope in [0, 1] {
            let face0 = FaceDefinition {
                model: model.clone(),
                arrows: Default::default(),
            };
            rig.wiring
                .balloon_definitions
                .insert(scope, ScopeBalloonDefinitions::single(scope, face0));
            rig.wiring.balloon_faces.insert(scope, 2);
        }
        answer(&mut rig, 0, Ok(()));
        answer(&mut rig, 1, Ok(()));
        frame(&mut rig);
        let follow = *rig
            .world
            .get::<areka_placement::follow::BalloonFollow>(windows.char_window(0).unwrap())
            .unwrap();
        let limit = *rig
            .world
            .get::<areka_placement::spawn::BalloonLimit>(windows.balloon_window(0).unwrap())
            .unwrap();
        let basis = rig
            .world
            .resource::<DefaultPlacementBasis>()
            .balloon
            .windowpositions
            .clone();
        (follow, limit, basis)
    };
    let control = run([(0, wp.clone()), (1, wp.clone())].into());
    assert_eq!(
        control.1,
        areka_placement::spawn::BalloonLimit(false),
        "対照: 今の面の limit 0"
    );
    assert_eq!(
        run(BTreeMap::new()),
        control,
        "(scope 0 の追随, limit, 材料)"
    );
}

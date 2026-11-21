//! バルーンの切り替えで面ごとの定義の束が入れ替わる決定論テスト。
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

/// バルーンの切り替えは新しい束を入れ、今の面（ここでは面 0）の定義を書き手が今の定義にする。
/// 前の束・前のモデルは残らない。
#[test]
fn balloon_switch_installs_the_new_bundle_through_the_face_writer() {
    let mut rig = rig(SkinKind::Balloon, false);
    for scope in [0, 1] {
        rig.wiring
            .balloon_definitions
            .insert(scope, areka_runtime::assets::empty_definitions(scope));
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
        434 - 30 + 40,
        "対照: キャラ窓の幅－offsetx（右に出すので descript の分はシェルへ近づく向き）＋windowposition.x"
    );
    for (label, old_wps) in [("同じ値", &face_wps), ("空", &BTreeMap::new())] {
        assert_eq!(
            run(192, old_wps),
            control,
            "(材料の作者の DPI, scope 0 の追随) 前の windowposition が{label}"
        );
    }
}

/// 面 0（descript）が `windowposition.x,0`、面 2（自分の設定ファイル）が `windowposition.x,40`・
/// `windowposition.limit,0` のバルーンの束。設定ファイルから本番と同じ `load_scope_balloon_definitions`
/// で作る（一時フォルダはワークツリーの `target\` の下）。
fn two_face_windowposition_definitions(scope: u32) -> ScopeBalloonDefinitions {
    use areka_emo_atlas::{AlphaParams, MemoryDecoder, UseSelfAlpha};
    let dir = temp_path_kit::TempPath::under_target("switch-face-windowposition");
    let prefix = if scope == 0 { "balloons" } else { "balloonk" };
    let write = |name: &str, text: &str| std::fs::write(dir.child(name), text).expect("ファイル");
    write("descript.txt", "windowposition.x,0\r\n");
    write(
        &format!("{prefix}2s.txt"),
        "windowposition.x,40\r\nwindowposition.limit,0\r\n",
    );
    for face in [0, 2] {
        write(&format!("{prefix}{face}.png"), "");
    }
    let faces =
        areka_emo_present::balloon::resolve_balloon_faces(dir.path(), scope).expect("面 0 は在る");
    let alpha = AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    };
    let defs = areka_emo_present::balloon_definitions::load_scope_balloon_definitions(
        dir.path(),
        scope,
        &faces,
        &MemoryDecoder::new(),
        alpha,
    );
    assert_ne!(
        defs.for_face(2).model.windowposition(),
        defs.face0().model.windowposition(),
        "前提: 面 2 は面 0 と違う windowposition"
    );
    defs
}

/// シェルの切り替えは、今出している面の `windowposition` で解き直す。
/// 切り替えの読み込みはバルーンのフォルダの面 0 だけを読むので、ここでは読み込みの値を面 0 の値
/// （`windowposition.x,0`）にし、今の面 2 は自分の設定ファイルで `windowposition.x,40`・
/// `windowposition.limit,0` を持つ。後始末の後のずらし・画面の内に保つ指定・材料は、読み込みが
/// 今の面の値を返した対照と同じになる（面 0 の値のままなら x が 40 少なく、limit が 1）。
#[test]
fn shell_switch_resolves_with_the_current_face_windowposition() {
    let defs = [0, 1].map(two_face_windowposition_definitions);
    let wp_of = |face: u32| -> BTreeMap<usize, _> {
        (0..2)
            .map(|scope| {
                let model = &defs[scope].for_face(face).model;
                (
                    scope,
                    (model.windowposition(), model.windowposition_raw().clone()),
                )
            })
            .collect()
    };
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
            rig.wiring
                .balloon_definitions
                .insert(scope, defs[scope as usize].clone());
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
    let control = run(wp_of(2));
    assert_eq!(
        control.1,
        areka_placement::spawn::BalloonLimit(false),
        "対照: 今の面の limit 0"
    );
    assert_eq!(run(wp_of(0)), control, "(scope 0 の追随, limit, 材料)");
}

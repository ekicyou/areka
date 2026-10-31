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
            .insert(scope, crate::emo2_boot::assets::empty_definitions(scope));
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

//! `ArekaStrictJudges` の答えが `check_script` と同じ物差しの写しであること（spec: mcp-strict-errors 要件 2.7）。

use areka_sakura::strict::{DetectionPoint, StrictJudges};

use super::{ArekaStrictJudges, production};

/// `\!` は正準の台帳の答え（登記の在る組だけ拾われる）をそのまま返す。
#[test]
fn command_handled_follows_the_canonical_ledger() {
    let judges = ArekaStrictJudges::canonical();
    let rows: [(&str, Option<&str>, bool); 5] = [
        ("move", None, true),
        ("set", Some("zorder"), true),
        ("open", Some("readme"), true),
        ("open", Some("nosuchthing"), false),
        ("nosuchcommand", None, false),
    ];
    for (name, first, want) in rows {
        assert_eq!(
            judges.command_handled(name, first),
            want,
            "({name}, {first:?}) は拾われるか"
        );
    }
}

/// `\f` は文字の層の分類を検出点へ写し、診ないものは None のまま。
#[test]
fn font_maps_the_text_layer_class_to_detection_points() {
    let judges = ArekaStrictJudges::canonical();
    let rows: [(&[&str], Option<DetectionPoint>); 4] = [
        (&["sub", "1"], Some(DetectionPoint::FontIgnored)),
        (&["colour", "red"], Some(DetectionPoint::FontUnknownKey)),
        (&[], Some(DetectionPoint::FontUnknownKey)),
        (&["bold", "1"], None),
    ];
    for (args, want) in rows {
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(judges.font(&args), want, "{args:?} の検出点");
    }
}

/// 本番の構築へ入れる判定は在る（無いと strict の記録が無言で 0 件になる）。
#[test]
fn production_judges_are_present() {
    let judges = production().expect("本番の判定");
    assert!(!judges.command_handled("nosuchcommand", None));
}

//! 記録を出さない組み方の入口の檻（spec: emo-bake-log-overlap 要件 2.1・2.3・3.2・3.4）。
//!
//! 追記の相手の面が無い・既に在るアニメーションの番号を追記で書き直す・同じ番号の面を 2 度書く、
//! を持つシェルを [`EmoWorld::build_with_images`] と [`EmoWorld::build_with_images_quiet`] で組む。
//! 出す入口はその記録を 1 行ずつ出し（対照）、出さない入口は畳み込みと面の画像の記録の文言が
//! 0 行で、面の表は出す入口と等しい。行は宛先まるごとでなく文言ごとに数える。

use std::collections::BTreeMap;

use super::EmoWorld;
use crate::log_capture::capture_logs;

const SHELL: &str = "\
surface0
{
element0,overlay,a.png,0,0
animation1.interval,always
animation1.pattern0,overlay,10,100,0,0
}
surface0
{
element1,overlay,b.png,0,0
}
surface.append0
{
animation1.interval,never
animation1.pattern0,overlay,11,100,0,0
}
surface.append5
{
element0,overlay,c.png,0,0
}
kero.surface.alias
{
せいかん,[0,20]
}
";

/// この檻のシェルで出る 3 件の文言（`fold.rs`）。
const DUPLICATE: &str = "surface id 重複";
const REDEFINE: &str = "append が既存 animation id を再定義";
const MISSING: &str = "surface.append 対象 id が未存在";

/// 面の表を組む段の記録の文言の全部（`fold.rs` の 10 本・`base_image.rs` の 1 本）。
const ALL_TEXTS: &[&str] = &[
    "DefRef::Surface が surfaces 範囲外を指す",
    "DefRef::Append が appends 範囲外を指す",
    "DefRef::Alias が aliases 範囲外を指す",
    "未知の DefRef 種別",
    DUPLICATE,
    MISSING,
    "SurfaceIndex は指すが SurfaceMaster component が欠落: スキップ",
    REDEFINE,
    "未知の AppendTarget 記述子",
    "SurfaceIndex は指すが SurfaceMaster component が欠落: 作り直す",
    "SurfaceIndex は指すが SurfaceMaster component が欠落: 面の画像を使わずスキップ",
];

fn count(logs: &str, text: &str) -> usize {
    logs.lines().filter(|l| l.contains(text)).count()
}

fn images() -> BTreeMap<u32, String> {
    BTreeMap::from([
        (0, "surface0.png".to_string()),
        (7, "surface7.png".to_string()),
    ])
}

/// 面の番号の列・各面の定義・別名・面の画像の表。
fn table(w: &EmoWorld) -> impl PartialEq + std::fmt::Debug {
    let ids: Vec<u32> = w.surface_ids().collect();
    let masters: Vec<_> = ids.iter().map(|&id| w.surface(id).cloned()).collect();
    (ids, masters, w.alias_snapshot(), w.base_images().clone())
}

#[test]
fn quiet_build_emits_no_fold_records_and_same_table() {
    let shell = areka_parsers::shell::parse(SHELL);
    let images = images();

    let mut emit = None;
    let emit_logs = capture_logs(|| emit = Some(EmoWorld::build_with_images(&shell, &images)));
    let mut quiet = None;
    let quiet_logs =
        capture_logs(|| quiet = Some(EmoWorld::build_with_images_quiet(&shell, &images)));

    // 対照: 出す入口は 3 件を 1 行ずつ出す。
    for text in [DUPLICATE, REDEFINE, MISSING] {
        assert_eq!(count(&emit_logs, text), 1, "出す入口の {text}: {emit_logs}");
    }
    // 出さない入口は、面の表を組む段の記録の文言がどれも 0 行。
    for text in ALL_TEXTS {
        assert_eq!(
            count(&quiet_logs, text),
            0,
            "出さない入口の {text}: {quiet_logs}"
        );
    }

    let (emit, quiet) = (emit.unwrap(), quiet.unwrap());
    assert_eq!(table(&emit), table(&quiet), "面の表は両入口で等しい");
    // 検体が空振りしていないこと（面 0・面 7 が在り、面 5 は新設されない・別名が在る）。
    assert!(emit.surface(0).is_some() && emit.surface(7).is_some());
    assert!(emit.surface(5).is_none());
    assert!(emit.resolve_alias("せいかん").is_some());
}

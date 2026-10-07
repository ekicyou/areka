//! アニメの表が持つ参照の表の写しと「動く部品が在るか」の檻
//! （spec: areka-P0-surface-element-nesting 要件 5.1・7.2・7.3・tasks.md 7.1）。
//!
//! 入力は `surfaces.txt` の本文そのもの（解析 → 畳み込みを経る実経路）で組む。
//! 「部品になりうるサーフェス」は ① element定義の子 ② 着せ替えの pattern0 の先 ③ 採った animation の
//! コマが指す 0 以上の番号（描画メソッドで絞らない・design.md「Table」の字句どおり）④ `always` の
//! 経過 0 の pattern の先（spec: areka-P0-animated-image-playback）の 4 つの経路があるので、経路ごとに
//! 1 つだけ立てた本文で真を、どれも立たない本文で偽を確かめる。

use areka_emo_compose::EmoWorld;

use super::AnimationTable;
use crate::sample_test_support::emo2_root;

/// 入れ子の検体（compose の検体フォルダの本文をそのまま読む・design.md「解決したこと」）。
const FIXTURE: &str =
    include_str!("../../areka-emo-compose/tests/fixtures/surface-nesting/surfaces.txt");

fn world_of(text: &str) -> EmoWorld {
    EmoWorld::build(&areka_parsers::shell::parse(text))
}

fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&world_of(text))
}

/// 空の表と、入れ子も動く部品も無い本文では偽（要件 7.2・7.3）。
#[test]
fn table_without_nesting_or_animated_parts_is_false() {
    let empty = AnimationTable::empty();
    assert!(!empty.has_animated_parts());
    assert!(empty.nest_table().is_empty());

    // 一番上 0 のまばたきのコマが 100 を指すが、100 は自分の animation を持たない（今までのシェルの形）。
    let plain = table_of(
        "surface0\n{\nanimation0.interval,random,4\n\
         animation0.pattern0,overlay,100,40,0,0\n\
         animation0.pattern1,overlay,-1,80,0,0\n}\n\
         surface100\n{\n}\n",
    );
    assert!(!plain.animations(0).is_empty(), "一番上のアニメは採る");
    assert!(!plain.has_animated_parts(), "コマの先が動かなければ偽");
    assert!(plain.nest_table().is_empty(), "入れ子も着せ替えも無い");
}

/// 部品になりうる経路を 1 つずつ立てて、真偽が経路と「採った animation」で決まることを確かめる。
#[test]
fn each_part_path_decides_has_animated_parts() {
    // 部品の側（200）の animation。`{interval}` を差し替えて「採る／採らない」を切る。
    let part = |interval: &str| {
        format!(
            "surface200\n{{\nanimation0.interval,{interval}\n\
             animation0.pattern0,overlay,201,50,0,0\n\
             animation0.pattern1,overlay,-1,50,0,0\n}}\nsurface201\n{{\n}}\n"
        )
    };
    let child = "surface0\n{\nelement0,overlay,200,0,0\n}\n";
    let bind = "surface0\n{\nanimation5.interval,bind\nanimation5.pattern0,overlay,200,0,0,0\n}\n";
    let frame = "surface0\n{\nanimation1.interval,random,2\n\
                 animation1.pattern0,overlay,200,40,0,0\n}\n";
    // 合計 0 の `always`（採らない）の経過 0 の pattern（pattern1）だけが 200 を指す（spec:
    // areka-P0-animated-image-playback task 3.2・合成が経過 0 として描く先）。
    let always_rest = "surface0
{
animation1.interval,always
                       animation1.pattern0,overlay,201,0,0,0
                       animation1.pattern1,overlay,200,0,0,0
}
";
    let unrelated = "surface0\n{\nanimation1.interval,random,2\n\
                     animation1.pattern0,overlay,201,40,0,0\n}\n";

    let cases = [
        ("element定義の子が動く", child, "random,2", true),
        (
            "element定義の子が動かない（採らない語）",
            child,
            "runonce",
            false,
        ),
        ("着せ替えの pattern0 の先が動く", bind, "random,2", true),
        ("採った animation のコマの先が動く", frame, "random,2", true),
        ("コマの先の animation が採らない語", frame, "runonce", false),
        (
            "採らない always の経過 0 の先が動く",
            always_rest,
            "random,2",
            true,
        ),
        (
            "動くサーフェスがどこからも指されない",
            unrelated,
            "random,2",
            false,
        ),
    ];
    for (label, top, interval, expected) in cases {
        let table = table_of(&format!("{top}{}", part(interval)));
        assert_eq!(table.has_animated_parts(), expected, "{label}");
    }
}

/// 入れ子の検体では真。写しは面の表の参照の表と同じもの（要件 5.1・7.2）。
#[test]
fn fixture_has_animated_parts_and_keeps_a_copy_of_the_nest_table() {
    let world = world_of(FIXTURE);
    let table = AnimationTable::from_world(&world);
    assert!(table.has_animated_parts(), "子 10・着せ替えの先 40 が動く");
    assert_eq!(table.nest_table(), &world.nest_table());
    assert_eq!(
        table.nest_table().parts(0).map(|p| p.children.clone()),
        Some(vec![10, 30]),
        "親 A の子は 10 と 30"
    );
}

/// emo2 では `\1` のまばたきのコマ 2110・2210 が自分の animation を持つので真（design.md「emo2 についての事実」）。
#[test]
fn emo2_has_animated_parts_through_2110_and_2210() {
    let path = emo2_root()
        .join("shell")
        .join("master")
        .join("surfaces.txt");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
    let table = table_of(&content);
    assert!(table.has_animated_parts());
    assert!(
        !table.animations(2110).is_empty(),
        "2110 は animation を持つ"
    );
    assert!(
        !table.animations(2210).is_empty(),
        "2210 は animation を持つ"
    );
}

/// 部品の番号で引いた animation の列は、同じサーフェスを一番上に表示するときの列と同じ（要件 5.1）。
#[test]
fn part_animations_equal_those_shown_as_top() {
    let nested = table_of(FIXTURE);
    // 同じ 10・40 を、入れ子も親も無い本文で一番上として読んだ表。
    let alone = table_of(
        "surface10\n{\nelement0,overlay,eye.png,0,0\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,12,50,0,0\nanimation0.pattern1,overlay,-1,50,0,0\n}\n\
         surface12\n{\n}\n\
         surface40\n{\nelement0,overlay,mouth.png,0,0\nanimation0.interval,random,3\n\
         animation0.pattern0,overlay,41,100,0,0\nanimation0.pattern1,overlay,-1,100,0,0\n}\n\
         surface41\n{\n}\n",
    );
    for id in [10, 40] {
        assert!(!nested.animations(id).is_empty(), "{id} は動く");
        assert_eq!(nested.animations(id), alone.animations(id), "{id}");
    }
}

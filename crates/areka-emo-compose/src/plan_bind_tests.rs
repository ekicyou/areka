//! 合成が着せ替えの手順を積み、`insert` を展開する（seriko-pattern-calls task 2.2・要件 5.5〜5.7・
//! 6.1〜6.4・6.7・6.8・8.1）。
//!
//! 有効な純 `bind` は全部のコマを番号の順に重ね、`insert,B` の位置に B の寄与を B 自身の座標で
//! 積む（SSP の測り M6）。無効の純 `bind` も呼ばれて始まったコマは描き、有効の純 `bind` は
//! コマが消えても静的な寄与に戻る（M5）。面 `N` の絵は `sN.png`（2×2 の不透明）。

use std::path::Path;

use areka_emo_atlas::SetId;
use areka_parsers::shell::parse;

use super::derive_ops;
use super::test_support::{bake_atlas, id_to_path};
use crate::bind::BindSet;
use crate::log_capture::capture_logs;
use crate::method::ComposeMethod;
use crate::pattern::{PatternFrame, PatternState};
use crate::world::EmoWorld;

const RELS: &[&str] = &[
    "body.png", "s10.png", "s11.png", "s12.png", "s20.png", "s21.png", "s22.png", "s30.png",
];

/// 面 0（`body.png`）に `anims` の animation を置き、面 10〜30 にそれぞれの絵を置いた文面。
fn shell(anims: &str) -> String {
    let mut text =
        format!("charset,UTF-8\n\nsurface0\n{{\nelement0,overlay,body.png,0,0\n{anims}}}\n");
    for n in [10, 11, 12, 20, 21, 22, 30] {
        text.push_str(&format!(
            "\nsurface{n}\n{{\nelement0,overlay,s{n}.png,0,0\n}}\n"
        ));
    }
    text
}

/// 面 0 を合成した命令列（(絵の名前, X, Y)）と記録。
fn compose(
    anims: &str,
    binds: &[u32],
    pattern: &PatternState,
) -> (Vec<(&'static str, i64, i64)>, String) {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, RELS);
    let map = id_to_path(&atlas, RELS);
    let mut world = EmoWorld::build(&parse(&shell(anims)));
    world.bind_atlas(&atlas, SetId(0));
    let binds = BindSet::from_ids(binds.iter().copied());
    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let logs =
        capture_logs(|| derive_ops(&mut ops, &mut visited, &world, &atlas, 0, &binds, pattern));
    let named = ops
        .iter()
        .map(|op| {
            let name = map
                .iter()
                .find(|(id, _)| *id == op.element)
                .map(|(_, p)| *p)
                .expect("命令の ElementId は既知");
            let (x, y) = op.transform.offset();
            (name, x, y)
        })
        .collect();
    (named, logs)
}

fn ops(anims: &str, binds: &[u32]) -> Vec<(&'static str, i64, i64)> {
    compose(anims, binds, &PatternState::default()).0
}

fn frame(surface_id: u32, x: i64, y: i64) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x,
        y,
    }
}

/// A（1）＝後ろ・`insert,2`・前、B（2）＝1 コマ。
const A_INSERTS_B: &str = "\
animation1.interval,bind
animation1.pattern0,overlay,10,0,1,2
animation1.pattern1,insert,2
animation1.pattern2,overlay,12,0,5,6
animation2.interval,bind
animation2.pattern0,overlay,20,0,3,4
";

/// A の全部のコマを順に重ね、`insert` の位置に B を B 自身の座標で積む。B は本来の位置に出ない（6.1・6.7・6.8・M6）。
#[test]
fn insert_places_b_between_a_patterns_at_b_coordinates() {
    assert_eq!(
        ops(A_INSERTS_B, &[1, 2]),
        [
            ("body.png", 0, 0),
            ("s10.png", 1, 2),
            ("s20.png", 3, 4),
            ("s12.png", 5, 6)
        ]
    );
}

/// 差し込まれる B の番号が A より前（描く順で先）でも、B は本来の位置に積まない（6.1）。
#[test]
fn inserted_b_is_not_drawn_at_its_own_earlier_position() {
    let anims = "\
animation1.interval,bind
animation1.pattern0,overlay,20,0,3,4
animation2.interval,bind
animation2.pattern0,overlay,10,0,1,2
animation2.pattern1,insert,1
animation2.pattern2,overlay,12,0,5,6
";
    assert_eq!(
        ops(anims, &[1, 2]),
        [
            ("body.png", 0, 0),
            ("s10.png", 1, 2),
            ("s20.png", 3, 4),
            ("s12.png", 5, 6)
        ]
    );
}

/// B が無効なら `insert` の位置に何も積まない（6.2）。
#[test]
fn inactive_b_inserts_nothing() {
    assert_eq!(
        ops(A_INSERTS_B, &[1]),
        [("body.png", 0, 0), ("s10.png", 1, 2), ("s12.png", 5, 6)]
    );
}

/// 差し込む A が無効で展開されなければ、B は本来の位置に積む（設計議題 4）。
#[test]
fn b_is_drawn_at_its_own_position_when_a_is_inactive() {
    assert_eq!(
        ops(A_INSERTS_B, &[2]),
        [("body.png", 0, 0), ("s20.png", 3, 4)]
    );
}

/// 差し込まれた B に今のコマがあれば、そのコマを `insert` の位置に積む。
#[test]
fn inserted_b_with_a_frame_draws_the_frame_in_place() {
    let mut pattern = PatternState::default();
    pattern.set(2, frame(21, 7, 8));
    assert_eq!(
        compose(A_INSERTS_B, &[1, 2], &pattern).0,
        [
            ("body.png", 0, 0),
            ("s10.png", 1, 2),
            ("s21.png", 7, 8),
            ("s12.png", 5, 6)
        ]
    );
}

/// 入れ子: A が B を、B が C を差し込む（6.3）。
#[test]
fn nested_inserts_expand_recursively() {
    let anims = "\
animation1.interval,bind
animation1.pattern0,overlay,10,0,0,0
animation1.pattern1,insert,2
animation1.pattern2,overlay,12,0,0,0
animation2.interval,bind
animation2.pattern0,overlay,20,0,0,0
animation2.pattern1,insert,3
animation2.pattern2,overlay,22,0,0,0
animation3.interval,bind
animation3.pattern0,overlay,30,0,0,0
";
    assert_eq!(
        ops(anims, &[1, 2, 3]),
        [
            ("body.png", 0, 0),
            ("s10.png", 0, 0),
            ("s20.png", 0, 0),
            ("s30.png", 0, 0),
            ("s22.png", 0, 0),
            ("s12.png", 0, 0)
        ]
    );
}

/// 巡回（A→B→A）と自分自身への `insert` は打ち切り、各着せ替えを 1 度だけ積む（6.4）。
#[test]
fn cyclic_and_self_inserts_draw_each_once() {
    let anims = "\
animation1.interval,bind
animation1.pattern0,overlay,10,0,0,0
animation1.pattern1,insert,2
animation1.pattern2,insert,1
animation2.interval,bind
animation2.pattern0,overlay,20,0,0,0
animation2.pattern1,insert,1
animation2.pattern2,overlay,22,0,0,0
";
    assert_eq!(
        ops(anims, &[1, 2]),
        [
            ("body.png", 0, 0),
            ("s10.png", 0, 0),
            ("s20.png", 0, 0),
            ("s22.png", 0, 0)
        ]
    );
}

/// 2 つの着せ替えが同じ C を差し込むときは、描く順で先の方だけが積む（6.4）。
#[test]
fn of_two_inserters_only_the_earlier_draws_c() {
    let anims = "\
animation1.interval,bind
animation1.pattern0,overlay,10,0,0,0
animation1.pattern1,insert,3
animation2.interval,bind
animation2.pattern0,overlay,21,0,0,0
animation2.pattern1,insert,3
animation3.interval,bind
animation3.pattern0,overlay,30,0,0,0
";
    assert_eq!(
        ops(anims, &[1, 2, 3]),
        [
            ("body.png", 0, 0),
            ("s10.png", 0, 0),
            ("s30.png", 0, 0),
            ("s21.png", 0, 0)
        ]
    );
}

/// 無効の純 `bind` でも、呼ばれて始まった今のコマは描く（5.6・M5）。
#[test]
fn inactive_pure_bind_frame_is_drawn() {
    let mut pattern = PatternState::default();
    pattern.set(2, frame(20, 3, 4));
    assert_eq!(
        compose(A_INSERTS_B, &[], &pattern).0,
        [("body.png", 0, 0), ("s20.png", 3, 4)]
    );
}

/// `bind+random` の外れた保持コマは今のまま描かない（D9-3 の門）。
#[test]
fn inactive_bind_random_frame_stays_hidden() {
    let anims = "\
animation4.interval,bind+random,4
animation4.pattern0,overlay,30,0,0,0
";
    let mut pattern = PatternState::default();
    pattern.set(4, frame(30, 0, 0));
    assert_eq!(compose(anims, &[], &pattern).0, [("body.png", 0, 0)]);
}

/// 有効の純 `bind` はコマが静的な寄与を置き換え、コマが消えれば（`stop`）静的な寄与に戻る（5.7・M5）。
#[test]
fn active_pure_bind_falls_back_to_steps_after_its_frame_is_gone() {
    let mut pattern = PatternState::default();
    pattern.set(1, frame(11, 0, 0));
    // A のコマは手順を展開しないので、B は本来の位置に積む。
    assert_eq!(
        compose(A_INSERTS_B, &[1, 2], &pattern).0,
        [("body.png", 0, 0), ("s11.png", 0, 0), ("s20.png", 3, 4)]
    );
    pattern.remove(1);
    assert_eq!(
        compose(A_INSERTS_B, &[1, 2], &pattern).0,
        ops(A_INSERTS_B, &[1, 2])
    );
}

/// 着せ替えの未知の語は合成を何度組んでも「未知の合成メソッド名」を出さない（5.5）。
#[test]
fn repeated_composes_log_no_unknown_method_name() {
    let anims = "\
animation1.interval,bind
animation1.pattern0,unknownmethod,10,0,0,0
animation1.pattern1,overlay,12,0,0,0
animation4.interval,bind+random,4
animation4.pattern0,unknownmethod,30,0,0,0
";
    for _ in 0..3 {
        let (named, logs) = compose(anims, &[1, 4], &PatternState::default());
        assert_eq!(named, [("body.png", 0, 0), ("s12.png", 0, 0)]);
        assert_eq!(
            logs.matches("未知の合成メソッド名").count(),
            0,
            "合成のたびの記録: {logs}"
        );
    }
}

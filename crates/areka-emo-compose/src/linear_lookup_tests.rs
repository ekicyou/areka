//! 線形の探索で 2 乗に伸びていた所が、大きな検体でも短く終わること（指摘 25〜30・要件 2.1・4.4・4.5）。
//!
//! 直す前はどれも n の 2 乗に伸びた（n を 2 倍にして約 4 倍・debug で測った）。検体は直す前なら debug で
//! 十数秒〜数十秒かかる大きさに取り、直した後は 1 つ 1 秒前後で終わる。時間の閾値は見ない（揺れるので）。
//! 戻せば走行が目に見えて長引く。答えの並びと数は今までと同じであることを一緒に確かめる。

use std::path::Path;

use areka_emo_atlas::{AtlasTable, SetId};
use areka_parsers::shell::parse;

use crate::Composer;
use crate::bind::BindSet;
use crate::composed::ComposedSurface;
use crate::log_capture::capture_logs;
use crate::pattern::PatternState;
use crate::plan_test_support::{bake_atlas, frame};
use crate::world::EmoWorld;

/// シェルの本文 `body` を絵 `s.png` 1 枚の頁で組んで束縛する。
fn world_of(body: &str) -> (EmoWorld, AtlasTable) {
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let mut world = EmoWorld::build(&parse(&format!("charset,UTF-8\n{body}")));
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas)
}

/// 面 0 を `binds` と `pattern` で合成し、命令の数を返す。
fn compose_ops(
    world: &EmoWorld,
    atlas: &AtlasTable,
    binds: &BindSet,
    pattern: &PatternState,
) -> usize {
    let mut composer = Composer::new();
    let mut out = ComposedSurface::new(0, 0);
    composer
        .compose_into(&mut out, world, atlas, 0, binds, pattern)
        .expect("合成できる");
    composer.ops.len()
}

/// 面 0 の見える部品を、seriko のように 1 度作った表から `frames` 回求め（作業の列は使い回す）、最後の答えを返す。
fn visible(world: &EmoWorld, frames: usize) -> Vec<u32> {
    let table = world.nest_table();
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let mut parts = Vec::new();
    for _ in 0..frames {
        table.visible_parts(0, &binds, &pattern, &mut parts);
    }
    parts
}

/// 幅: 面 0 が面 20000 … 1 を（番号の大きい方から）1 つずつ置き、10 フレーム求める。印を列で引けば
/// 2 乗（指摘 25）。
#[test]
fn visible_parts_of_a_wide_surface_finish_short_and_ascending() {
    const N: u32 = 20_000;
    let mut text = String::from("surface0\n{\n");
    for i in 1..=N {
        text.push_str(&format!("element{i},overlay,{},0,0\n", N + 1 - i));
    }
    text.push_str("}\n");
    for i in 1..=N {
        text.push_str(&format!("surface{i}\n{{\nelement0,overlay,s.png,0,0\n}}\n"));
    }
    let (world, _) = world_of(&text);
    assert_eq!(
        visible(&world, 10),
        (1..=N).collect::<Vec<_>>(),
        "昇順・重複なし"
    );
}

/// 深さ: 面 0 → 1 → … → 10000 の鎖を、毎フレームのように 100 回求める（指摘 25）。
#[test]
fn visible_parts_of_a_deep_chain_finish_short_every_frame() {
    const LAST: u32 = 10_000;
    let mut text = String::new();
    for i in 0..LAST {
        text.push_str(&format!(
            "surface{i}\n{{\nelement0,overlay,{},0,0\n}}\n",
            i + 1
        ));
    }
    text.push_str(&format!(
        "surface{LAST}\n{{\nelement0,overlay,s.png,0,0\n}}\n"
    ));
    let (world, _) = world_of(&text);
    assert_eq!(visible(&world, 100), (1..=LAST).collect::<Vec<_>>());
}

/// 有効な着せ替え 20000 本が `insert` で次を差し込む鎖（最後が面 1 を描く）。差し込みの決めを列で
/// 引けば 2 乗（指摘 26）。
#[test]
fn composing_many_active_binds_finishes_short() {
    const N: u32 = 20_000;
    let mut text = String::from("surface0\n{\nelement0,overlay,s.png,0,0\n");
    for i in 0..N {
        text.push_str(&format!("animation{i}.interval,bind\n"));
        if i + 1 < N {
            text.push_str(&format!("animation{i}.pattern0,insert,{}\n", i + 1));
        } else {
            text.push_str(&format!("animation{i}.pattern0,overlay,1,0,0,0\n"));
        }
    }
    text.push_str("}\nsurface1\n{\nelement0,overlay,s.png,0,0\n}\n");
    let (world, atlas) = world_of(&text);
    let binds = BindSet::from_ids(0..N);
    assert_eq!(
        compose_ops(&world, &atlas, &binds, &PatternState::default()),
        2,
        "土台と鎖の先の絵"
    );
}

/// 面 0 へ 40000 回の追記（element と animation を 1 つずつ・element は番号の大きい方から）。足すたびに
/// 並べ直し、重なりを列で引けば 2 乗（指摘 27）。並びは layer 昇順で、同じ番号の中は前の行が先。
#[test]
fn folding_many_appends_to_one_surface_finishes_short_and_sorted() {
    const N: u32 = 40_000;
    let mut text = String::from("surface0\n{\nelement0,overlay,first.png,0,0\n}\n");
    for i in 1..=N {
        text.push_str(&format!(
            "surface.append0\n{{\nelement{},overlay,s.png,0,0\nanimation{i}.interval,bind\n}}\n",
            N + 1 - i
        ));
    }
    // 同じ番号 0 を最後に足す（前の行が先・二重定義の warn! が 1 行）。
    text.push_str("surface.append0\n{\nelement0,overlay,last.png,0,0\n}\n");
    let mut world = None;
    let logs = capture_logs(|| world = Some(EmoWorld::build(&parse(&text))));
    let world = world.expect("組める");
    let master = world.surface(0).expect("面 0");
    let layers: Vec<u32> = master.elements.iter().map(|e| e.layer).collect();
    let mut expected = vec![0, 0];
    expected.extend(1..=N);
    assert_eq!(layers, expected, "layer 昇順");
    let paths: Vec<&str> = master.elements[..2]
        .iter()
        .map(|e| e.path.as_str())
        .collect();
    assert_eq!(paths, ["first.png", "last.png"], "同じ番号の中は前の行が先");
    assert_eq!(master.animations.len(), N as usize);
    assert_eq!(logs.matches("element番号の二重定義").count(), 1, "{logs}");
}

/// `always` の animation 10000 本（どれも経過 0 で面 1 を描く）と、animation の無い番号のコマ 20000 枚
/// （広がりの予算の内側）を 3 回合成する。コマの番号ごとに重ねる列と animation の列を順に引けば 2 乗
/// （指摘 28・29）。animation の数は抑えた（読む側の `parse` が animation の数の 2 乗に伸びる）。
#[test]
fn composing_many_animations_finishes_short() {
    const ANIMS: u32 = 10_000;
    const FRAMES: u32 = 20_000;
    let mut text = String::from(
        "surface0
{
element0,overlay,s.png,0,0
",
    );
    for i in 0..ANIMS {
        text.push_str(&format!(
            "animation{i}.interval,always
animation{i}.pattern0,overlay,1,0,0,0
"
        ));
    }
    text.push_str(
        "}
surface1
{
element0,overlay,s.png,0,0
}
",
    );
    let (world, atlas) = world_of(&text);
    let mut pattern = PatternState::default();
    for id in ANIMS..ANIMS + FRAMES {
        pattern.set(id, frame(1, 0, 0));
    }
    for _ in 0..3 {
        assert_eq!(
            compose_ops(&world, &atlas, &BindSet::default(), &pattern),
            (1 + ANIMS + FRAMES) as usize,
            "土台と経過 0 のコマ全部と今のコマ全部"
        );
    }
}

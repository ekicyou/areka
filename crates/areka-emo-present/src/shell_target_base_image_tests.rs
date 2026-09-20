//! 土台の絵の決定（要件 2.1 の表ア〜エ）と `surface.append`（要件 3.7・3.8）を、
//! 権威の核 [`build_shell_target`] を通して外形で確かめる檻（要件 7.2・7.7）。
//!
//! 見るのは**合成した結果の外形**である。`areka-emo-compose` 側の檻（`base_image_tests.rs`）は
//! 面の表の形（層 0 に何が入ったか）までしか見ないので、「足した層が焼く絵の一覧に載って
//! いなかった」「載っていたが綴りが違って引けなかった」は素通りする——外形を算出する
//! `flatten_extent` は索引表で引けない層を**記録なしで飛ばす**ため、そこが壊れていても
//! 面の表の形だけなら緑のままである。ゆえに本ファイルは名前の判定から焼き付けまでを
//! 通した外形で判定する。
//!
//! fs には触れない。画像はメモリ上の復号器（[`MemoryDecoder`]）に実寸と色で登録し、
//! シェルのフォルダは実在しないパスでよい（[`build_shell_target`] は fs を触らない核）。
//!
//! # 表ア〜エの入力（画像の実寸の違いが外形に出る形）
//!
//! | 面 | 宣言 | 画像 | 表 | 外形 |
//! |---|---|---|---|---|
//! | 1 | 無し | `surface1.png` 7×5 | ア | 7×5（画像の実寸） |
//! | 0 | `element1` = `parts.png` 6×6 @(2,3) | `surface0.png` 4×12 | イ | 6×12（画像と `element1` を合わせた外形・画像が奥） |
//! | 2 | `element0` = `front.png` 3×3 @(0,0) | `surface2.png` 20×20 | ウ | 3×3（`element0` の実寸・画像は使わない） |
//! | 3 | `collision0` のみ（層 0 個） | 無し | エ | `EmptyComposition`（今日のまま） |
//!
//! イの 6×12 は幅が `element1` から・高さが画像から来ている——どちらか一方しか数えていなければ
//! この値にならない。外形は層ごとの**原寸**の軸ごとの最大で決まり、層の位置は寄与しない
//! （`compute_extent`・`crates/areka-emo-compose/src/plan.rs`）ので、位置は画素の重なりを
//! 見るためだけに与えてある。
//!
//! ウの画像を**わざと大きく別の絵**にしてあるのが較正である——`apply_base_images` の
//! 「層 0 が在れば使わない」の判定を経路から外すと、面 2 の外形が 3×3 から 20×20 へ動いて
//! [`case_c_layer_zero_present_keeps_the_declared_extent`] が赤くなる。

use super::*;

use std::path::{Path, PathBuf};

use areka_emo_atlas::MemoryDecoder;
use areka_emo_compose::{BindSet, ComposeError, ComposedSurface, Composer, PatternState};
use areka_parsers::shell::parse;

use super::test_support::{CapturedEvent, capture_events};

/// 面 1（ア）の画像の実寸。
const SOLO: (u32, u32) = (7, 5);
/// 面 0（イ）の画像の実寸。`element1` の絵とは縦横どちらもずれている。
const BASE0: (u32, u32) = (4, 12);
/// 面 0（イ）の `element1` の絵の実寸と位置（位置は画素の重なりを見るためだけに与える）。
const PARTS: (u32, u32) = (6, 6);
const PARTS_AT: (i64, i64) = (2, 3);
/// 面 2（ウ）の `element0` の絵の実寸（外形はこれになる）。
const FRONT: (u32, u32) = (3, 3);
/// 面 2（ウ）の画像の実寸（使われないので外形に出てはならない）。
const SHADOWED: (u32, u32) = (20, 20);
/// 面 5（`surface.append` の対象）の画像の実寸。追記される絵より小さいので、追記が効けば外形が伸びる。
const APPENDEE: (u32, u32) = (4, 4);
/// 面 5 へ追記する絵の位置（絵そのものは面 0 と同じ `parts.png`）。
const APPENDED_AT: (i64, i64) = (3, 3);

/// 画像ごとの色（premultiplied BGRA・全画素不透明）。どの絵が描かれたかを画素で見分ける。
const COLOR_BASE0: [u8; 4] = [200, 0, 0, 255];
const COLOR_PARTS: [u8; 4] = [0, 0, 200, 255];
const COLOR_SOLO: [u8; 4] = [0, 200, 0, 255];
const COLOR_FRONT: [u8; 4] = [0, 180, 180, 255];
const COLOR_SHADOWED: [u8; 4] = [180, 180, 0, 255];
const COLOR_APPENDEE: [u8; 4] = [90, 90, 90, 255];

/// 檻の入力になるシェル（表ア〜エと `surface.append` の 2 例を 1 枚に同居させる）。
///
/// 面 5 は波括弧を持たず画像だけで存在し、そこへ `surface.append` が層 1 を足す（要件 3.7）。
/// 面 7 は宣言も画像も無い番号で、追記は新設せずに飛ばされる（要件 3.8）。
///
/// 追記する絵に面 0 と同じ `parts.png` を使うのは、焼く絵の一覧が `shell.surfaces` から
/// 導かれる（`ManifestDeriver::derive`）ためである——追記にしか現れない綴りは索引表に載らず、
/// 外形を算出する `flatten_extent` が記録なしで飛ばすので、追記が届いたかどうかが外形に出ない。
/// 本仕様は焼く絵の一覧を変えないので（design「Modified Files」）、ここは既に焼かれる綴りで組む。
fn surfaces_txt() -> String {
    format!(
        "surface0\n{{\nelement1,overlay,parts.png,{px},{py}\n}}\n\
         surface2\n{{\nelement0,overlay,front.png,0,0\n}}\n\
         surface3\n{{\ncollision0,0,0,10,10,Head\n}}\n\
         surface.append5\n{{\nelement1,overlay,parts.png,{ax},{ay}\n}}\n\
         surface.append7\n{{\nelement1,overlay,parts.png,0,0\n}}\n",
        px = PARTS_AT.0,
        py = PARTS_AT.1,
        ax = APPENDED_AT.0,
        ay = APPENDED_AT.1,
    )
}

/// フォルダ直下に在ることにするファイル名の一覧（面の画像 4 枚＋画像でない 3 件）。
const FILE_NAMES: [&str; 7] = [
    "surface0.png",
    "surface1.png",
    "surface2.png",
    "surface5.png",
    "parts.png",
    "front.png",
    "surfaces.txt",
];

/// 実在しないシェルのフォルダ（核は fs を触らないので、復号器の鍵を組むためだけに使う）。
fn shell_dir() -> PathBuf {
    PathBuf::from(r"C:\areka-test\shell-target-base-image")
}

/// 全画素が同じ色の不透明な絵を復号器へ登録する。
fn insert_solid(dec: &mut MemoryDecoder, dir: &Path, name: &str, size: (u32, u32), color: [u8; 4]) {
    let (w, h) = size;
    let bgra: Vec<u8> = color
        .iter()
        .copied()
        .cycle()
        .take((w * h * 4) as usize)
        .collect();
    dec.insert(dir.join(name), w, h, w * 4, bgra, true);
}

/// 表ア〜エの入力を焼いた [`ShellTarget`] を作る（絵 6 枚はすべて実寸つきで登録済み）。
fn build_table_target() -> ShellTarget {
    let dir = shell_dir();
    let mut dec = MemoryDecoder::new();
    for (name, size, color) in [
        ("surface0.png", BASE0, COLOR_BASE0),
        ("surface1.png", SOLO, COLOR_SOLO),
        ("surface2.png", SHADOWED, COLOR_SHADOWED),
        ("surface5.png", APPENDEE, COLOR_APPENDEE),
        ("parts.png", PARTS, COLOR_PARTS),
        ("front.png", FRONT, COLOR_FRONT),
    ] {
        insert_solid(&mut dec, &dir, name, size, color);
    }

    let shell = parse(&surfaces_txt());
    let selection = select_surface_images(&FILE_NAMES);
    let target = build_shell_target(shell, selection, &dir, &dec);
    assert!(
        target.bake_errors().is_empty(),
        "前提: 登録済みの絵だけを焼くので脱落は 0 件: {:?}",
        target.bake_errors()
    );
    target
}

/// 面 1 枚を合成する（有効 bind もコマも無い素の状態）。
fn compose(
    target: &ShellTarget,
    world: &EmoWorld,
    id: u32,
) -> Result<ComposedSurface, ComposeError> {
    Composer::new().compose(
        world,
        target.atlas(),
        id,
        &BindSet::default(),
        &PatternState::default(),
    )
}

/// 合成結果の 1 画素（premultiplied BGRA）を引く。
fn pixel(surface: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    let at = (y * surface.stride() + x * 4) as usize;
    surface.bytes()[at..at + 4]
        .try_into()
        .expect("4 バイトの画素")
}

/// 面の表の層 0 の画像パスを引く（層 0 が無ければ `None`）。
fn layer0_path(world: &EmoWorld, id: u32) -> Option<String> {
    world
        .surface(id)?
        .elements
        .iter()
        .find(|e| e.layer == 0)
        .map(|e| e.path.as_str().to_string())
}

/// 表ア（要件 2.2・3.9）: 宣言の無い番号は画像 1 枚で存在し、外形は**画像の実寸**になる。
#[test]
fn case_a_image_only_surface_takes_the_image_extent() {
    let target = build_table_target();
    let world = target.build_world();

    let composed = compose(&target, &world, 1).expect("画像だけの面 1 は合成できる");
    assert_eq!(
        (composed.width(), composed.height()),
        SOLO,
        "ア: 外形は面の画像の実寸でなければならない"
    );
    assert_eq!(
        layer0_path(&world, 1).as_deref(),
        Some("surface1.png"),
        "ア: 層 0 が面の画像になっていない"
    );
    assert_eq!(
        pixel(&composed, 0, 0),
        COLOR_SOLO,
        "ア: 面 1 の画像そのものが描かれていない"
    );
    assert_eq!(
        world.base_images().used.get(&1).map(String::as_str),
        Some("surface1.png"),
        "ア: 使った画像として数えられていない"
    );
}

/// 表イ（要件 2.1・2.5）: 層 0 の空いた面では、画像と `element1` を**合わせた**外形になり、
/// 画像は最も奥（層 0）へ敷かれる。
///
/// 外形 6×12 は画像だけ（4×12）でも `element1` だけ（6×6）でもない——幅は `element1` から、
/// 高さは画像から来るので、どちらか一方しか数えていなければこの値にならない。重なった位置の
/// 画素が `element1` の色であることが、画像が奥に居ること（手前へ出ていないこと）を示す。
#[test]
fn case_b_empty_layer_zero_merges_the_image_underneath() {
    let target = build_table_target();
    let world = target.build_world();

    let composed = compose(&target, &world, 0).expect("面 0 は合成できる");
    let expected = (BASE0.0.max(PARTS.0), BASE0.1.max(PARTS.1));
    assert_eq!(
        expected,
        (6, 12),
        "前提: 期待する外形はどちらか一方の実寸と一致しない"
    );
    assert_ne!(expected, BASE0, "前提: 画像だけを数えた外形とは違う");
    assert_ne!(expected, PARTS, "前提: `element1` だけを数えた外形とは違う");
    assert_eq!(
        (composed.width(), composed.height()),
        expected,
        "イ: 外形は画像と `element1` を合わせたものでなければならない"
    );
    assert_eq!(
        layer0_path(&world, 0).as_deref(),
        Some("surface0.png"),
        "イ: 画像が層 0（最も奥）へ敷かれていない"
    );
    assert_eq!(
        pixel(&composed, 0, 0),
        COLOR_BASE0,
        "イ: `element1` に覆われない位置に画像が出ていない"
    );
    assert_eq!(
        pixel(&composed, PARTS_AT.0 as u32, PARTS_AT.1 as u32),
        COLOR_PARTS,
        "イ: 重なった位置で画像が `element1` より手前に出ている（奥になっていない）"
    );
}

/// 表ウ（要件 2.1・10.3）: `element0` が在る面では画像を使わず、外形は**宣言どおり**のままである。
///
/// 較正: `apply_base_images`（`crates/areka-emo-compose/src/base_image.rs`）の
/// 「層 0 の element が在るか」の判定を経路から外すと、面 2 の外形が 3×3 から 20×20 へ動いて
/// 本テストが赤くなる。画像をわざと大きな別の絵にしてあるのはそのためである。
#[test]
fn case_c_layer_zero_present_keeps_the_declared_extent() {
    let target = build_table_target();
    let world = target.build_world();

    let composed = compose(&target, &world, 2).expect("面 2 は合成できる");
    assert_eq!(
        (composed.width(), composed.height()),
        FRONT,
        "ウ: 外形は `element0` の実寸でなければならない（画像の実寸が出ていたら使ってしまっている）"
    );
    assert_eq!(
        layer0_path(&world, 2).as_deref(),
        Some("front.png"),
        "ウ: 層 0 が宣言の絵から入れ替わっている"
    );
    assert_eq!(
        pixel(&composed, 0, 0),
        COLOR_FRONT,
        "ウ: 宣言の絵ではなく画像が描かれている"
    );
    assert_eq!(
        world.base_images().shadowed.get(&2).map(String::as_str),
        Some("surface2.png"),
        "ウ: 使わなかった画像として数えられていない"
    );
}

/// 表エ（要件 2.1）: 画像の無い番号には触らない。層が皆無の面は今日どおり `EmptyComposition`。
#[test]
fn case_d_surface_without_image_stays_empty() {
    let target = build_table_target();
    let world = target.build_world();

    assert_eq!(
        world
            .surface(3)
            .expect("面 3 は宣言から存在する")
            .elements
            .len(),
        0,
        "エ: 画像の無い番号へ層が足されている"
    );
    assert_eq!(
        compose(&target, &world, 3).err(),
        Some(ComposeError::EmptyComposition(3)),
        "エ: 層が皆無の面は今日どおり EmptyComposition でなければならない"
    );
}

/// 土台の絵の決定の全体像（要件 2.1 の表の 4 通りが 1 枚のシェルで同時に成り立つ）。
///
/// `used` と `shadowed` が重ならず、和が「認めた画像」の全体に等しいことも併せて見る
/// （[`BaseImageReport`] の不変条件・`crates/areka-emo-compose/src/base_image.rs`）。
#[test]
fn every_image_is_either_used_or_shadowed_exactly_once() {
    let world = build_table_target().build_world();
    let report = world.base_images();

    let used: Vec<u32> = report.used.keys().copied().collect();
    let shadowed: Vec<u32> = report.shadowed.keys().copied().collect();
    assert_eq!(
        used,
        vec![0, 1, 5],
        "使ったのは層 0 の空いていた面 0・画像だけの面 1・追記された面 5"
    );
    assert_eq!(
        shadowed,
        vec![2],
        "使わなかったのは `element0` を持つ面 2 だけ"
    );

    // 重なり 0 件と、和が「認めた画像」の全体であること（引き算で導かずそのまま判定する）。
    assert!(
        used.iter().all(|id| !shadowed.contains(id)),
        "同じ番号が使った側と使わなかった側の両方に居る: used={used:?} shadowed={shadowed:?}"
    );
    let mut both: Vec<u32> = used.iter().chain(shadowed.iter()).copied().collect();
    both.sort_unstable();
    assert_eq!(
        both,
        select_surface_images(&FILE_NAMES)
            .images
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        "認めた画像のうち、使いも隠れもしなかったものが在る"
    );

    assert!(
        world.surface(7).is_none(),
        "宣言も画像も無い面 7 が作られている"
    );
}

/// 要件 3.7: `surface.append` は画像だけで存在する面にも効く（「既にある面」と数える）。
///
/// 外形が画像（4×4）と追記した層（6×6）の軸ごとの最大になることが、追記が届いたことを示す。
/// 層 0 が画像のままで、その画素が描かれていることが、追記が**画像を消していない**ことを示す。
#[test]
fn append_reaches_a_surface_that_exists_only_as_an_image() {
    let target = build_table_target();
    let world = target.build_world();

    let composed = compose(&target, &world, 5).expect("画像だけの面 5 も追記後に合成できる");
    assert_eq!(
        (composed.width(), composed.height()),
        (APPENDEE.0.max(PARTS.0), APPENDEE.1.max(PARTS.1)),
        "外形が追記した層まで伸びていない（画像だけの外形 4×4 のままなら追記が届いていない）"
    );
    assert_eq!(
        layer0_path(&world, 5).as_deref(),
        Some("surface5.png"),
        "追記が画像を層 0 から追い出している"
    );
    assert_eq!(
        pixel(&composed, 0, 0),
        COLOR_APPENDEE,
        "追記された面に画像が描かれていない"
    );
    assert_eq!(
        pixel(&composed, APPENDED_AT.0 as u32, APPENDED_AT.1 as u32),
        COLOR_PARTS,
        "追記した絵が描かれていない"
    );
}

/// 要件 3.8: 宣言も画像も無い番号への `surface.append` は、既存の `warn!` を出して面を作らない。
#[test]
fn append_to_an_unknown_surface_warns_and_creates_nothing() {
    let (target, events) = capture_events(build_table_target);
    let world = target.build_world();

    assert!(
        world.surface(7).is_none(),
        "追記の対象が新設されている（要件 3.8 は新設しない）"
    );

    let hits: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| {
            e.target == "areka_emo_compose"
                && e.level == tracing::Level::WARN
                && e.message().contains("surface.append 対象 id が未存在")
        })
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "未存在の追記の `warn!` は面の表 1 つにつき 1 行（面 5 は画像で存在するので数に入らない）: {events:?}"
    );
    assert_eq!(
        hits[0].field("id"),
        Some("7"),
        "どの番号が飛ばされたかが読み取れる"
    );
}

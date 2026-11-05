use super::*;
use crate::plan_test_support::*;
use crate::world::EmoWorld;
use areka_emo_atlas::{AtlasTable, SetId};
use areka_parsers::shell::{Animation, Interval};
use std::path::Path;

// ── PatternState 合流と method ゲート（層(ii) の transient コマ合流・要件 4.2/4.6/5.3/8.4）─────
//
// 層 (ii) で「有効な着せ替えの集合 ∪ PatternState のコマ集合」を既存 animation-sort 整列へ合流し、
// 同 id はコマが着せ替えの静的寄与を置換する。非 Overlay の今のコマは debug!（method 名込み）＋
// 不描画（8.4）。非 Overlay の pattern は読み込みのときに着せ替えの手順から外れる。記録の行は
// log_capture で檻に入れる。

use crate::log_capture::capture_logs;
use crate::pattern::PatternFrame;

/// 現在コマ（PatternFrame）を 1 本組む（テスト補助・任意 method／オフセット）。
fn koma(surface_id: u32, method: ComposeMethod, x: i64, y: i64) -> PatternFrame {
    PatternFrame {
        method,
        ..frame(surface_id, x, y)
    }
}

/// pattern0（index==0）1 本だけを持つ bind animation を任意 method で組む（pattern0 method ゲート檻用）。
fn bind_anim_method(id: u32, ref_surface_id: i64, method: &str) -> Animation {
    Animation {
        id,
        interval: Interval::Bind,
        interval_written: true,
        patterns: vec![pattern(0, method, ref_surface_id)],
    }
}

/// **受入基準**・要件 8.4: 非 Overlay method の現在コマは debug!（method 名込み）＋
/// 不描画。かつコマは同 id の pattern0 静的寄与を置換するゆえ、その pattern0 も現れない（4.2）。
///
/// host=1000 は静的 base ＋ bind id=1 pattern0→part1(overlay)。id=1 の現在コマは surface 1500
/// （koma）method=Replace（非 Overlay）。コマは非駆動で不描画・pattern0(part1) も置換で不描画ゆえ、
/// 残るは base のみ。debug は method=Replace を載せる。
#[test]
fn non_overlay_koma_logs_debug_and_is_not_drawn_replacing_pattern0() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["base.png", "part1.png", "koma.png"]);
    let map = id_to_path(&atlas, &["base.png", "part1.png", "koma.png"]);

    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0)], // pattern0 は overlay。
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let koma_surface = surface(1500, vec![elem(0, "koma.png", 0, 0)]);
    let shell = shell_of(vec![host, part1, koma_surface]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut pattern = PatternState::default();
    pattern.set(1, koma(1500, ComposeMethod::Replace, 0, 0)); // 非 Overlay コマ。

    let mut out = Vec::new();
    let logs = capture_logs(|| out = ops(&world, &atlas, 1000, &binds, &pattern));

    // コマ（koma）は非駆動で不描画・pattern0（part1）はコマに置換され不描画 → base のみ。
    assert_eq!(
        ops_to_paths(&out, &map),
        vec!["base.png"],
        "非 Overlay コマは不描画・pattern0 も置換で不描画（残るは base）"
    );
    // debug! が method 名込みで発火する（要件 8.4・完全形保持のまま非駆動）。
    assert!(
        logs.contains("level=DEBUG") && !logs.contains("level=WARN"),
        "非 Overlay コマは DEBUG（組むたびの知らせ）: {logs}"
    );
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(
        logs.contains("method=Replace"),
        "method 名（判別子）を載せる: {logs}"
    );
    assert!(
        logs.contains("animation_id=1"),
        "対象 animation id を載せる: {logs}"
    );
}

/// 要件 8.4: 非 Overlay の **bind pattern0**（静的経路）も不描画。parser は
/// 非 overlay の pattern も渡すため。
///
/// host=1000 は静的 base ＋ bind id=1 の pattern0 method="replace"→part1。空 PatternState。
/// pattern0 は着せ替えの手順に入らず不描画で、残るは base のみ（合成のたびの知らせは手順が無い旨の debug）。
#[test]
fn non_overlay_pattern0_logs_debug_and_is_not_drawn() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["base.png", "part1.png"]);
    let map = id_to_path(&atlas, &["base.png", "part1.png"]);

    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim_method(1, 1100, "replace")], // 非 Overlay pattern0。
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let shell = shell_of(vec![host, part1]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut out = Vec::new();
    let logs = capture_logs(|| out = ops(&world, &atlas, 1000, &binds, &PatternState::default()));

    // 非 Overlay pattern0（part1）は不描画 → base のみ。
    assert_eq!(
        ops_to_paths(&out, &map),
        vec!["base.png"],
        "非 Overlay pattern0 は不描画（残るは base）"
    );
    // 描かない語は読み込みのときに着せ替えの手順から外れ（記録は `shape_issues` が 1 度だけ）、
    // 合成のたびには語を引き直さない（seriko-pattern-calls 5.5 で期待を直した）。
    assert!(
        !logs.contains("level=WARN") && !logs.contains("method=Replace"),
        "合成のたびに描かない語を知らせない: {logs}"
    );
    assert!(
        logs.contains("level=DEBUG") && logs.contains("animation_id=1"),
        "描く手順が無い着せ替えは DEBUG（対象 animation id つき）: {logs}"
    );
}

/// **受入基準**・要件 4.2: Overlay の現在コマは描画され、同 id の pattern0 静的
/// 寄与を**置換**する（pattern0 の surface は現れず、コマの surface が現れる）。
///
/// host=1000 は bind id=1 pattern0→open(1100)。id=1 の現在コマは closed(1200) method=Overlay。
/// コマが pattern0 を置換 → closed_eye のみ現れ open_eye は現れない。
#[test]
fn overlay_koma_is_drawn_and_replaces_same_id_pattern0() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["open_eye.png", "closed_eye.png"]);
    let map = id_to_path(&atlas, &["open_eye.png", "closed_eye.png"]);

    let host = surface_with_anims(1000, Vec::new(), vec![bind_anim(1, 1100, 0, 0)]);
    let open = surface(1100, vec![elem(0, "open_eye.png", 0, 0)]);
    let closed = surface(1200, vec![elem(0, "closed_eye.png", 0, 0)]);
    let shell = shell_of(vec![host, open, closed]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut pattern = PatternState::default();
    pattern.set(1, koma(1200, ComposeMethod::Overlay, 0, 0)); // Overlay コマ。

    let ops = ops(&world, &atlas, 1000, &binds, &pattern);

    // コマ（closed_eye）が pattern0（open_eye）を置換 → closed_eye のみ。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["closed_eye.png"],
        "Overlay コマが同 id の pattern0 を置換（open_eye は現れない）"
    );
}

/// 要件 5.3・画家のアルゴリズム: 合流後も id 昇順描画順が保たれる（既定 descend）。
///
/// bind id=1 pattern0→partA(1100)、**コマのみ** id=2→partB(1200)（bind animation なし）、
/// bind id=3 pattern0→partC(1300)。合流集合 {1,3}∪{2}={1,2,3}。id 昇順描画で partA→partB→partC。
/// コマ専用 id=2 が bind ids の間へ id 順で正しく挿入されることを突く。
#[test]
fn merged_ids_preserve_id_ascending_painter_order() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["partA.png", "partB.png", "partC.png"]);
    let map = id_to_path(&atlas, &["partA.png", "partB.png", "partC.png"]);

    // id=2 の bind animation は無い（コマ専用）。bind は id=1,3 のみ。
    let host = surface_with_anims(
        1000,
        Vec::new(),
        vec![bind_anim(1, 1100, 0, 0), bind_anim(3, 1300, 0, 0)],
    );
    let part_a = surface(1100, vec![elem(0, "partA.png", 0, 0)]);
    let part_b = surface(1200, vec![elem(0, "partB.png", 0, 0)]);
    let part_c = surface(1300, vec![elem(0, "partC.png", 0, 0)]);
    let shell = shell_of(vec![host, part_a, part_b, part_c]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // bind は 1,3 のみ有効。コマ id=2 は BindSet 非依存で合流される。
    let binds = BindSet::from_ids([1, 3]);
    let mut pattern = PatternState::default();
    pattern.set(2, koma(1200, ComposeMethod::Overlay, 0, 0));

    let ops = ops(&world, &atlas, 1000, &binds, &pattern);

    // 既定 descend → id 昇順描画: partA(1) → partB(2・コマ) → partC(3)。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["partA.png", "partB.png", "partC.png"],
        "合流後も id 昇順描画（コマ専用 id が bind ids の間へ id 順で挿入）"
    );
}

/// 要件 4.6・5.3: bind pattern0 を持たない id の Overlay コマ単独でも描画される
/// （合流集合が union ゆえ・コマ専用 id）。まばたき再生（pattern0 なし bind の再生コマ）を突く。
///
/// 注（bindopt D9-3）: 本フィクスチャの host は animation を 1 本も持たないため、id=1400 は当 surface の
/// **`bind+random` ではない**。ゆえに合流ゲート（`bind+random` のコマは bind 集合に属することを要求）を
/// 受けない。`bind+random` として定義された id を
/// bind 非所属のまま与える場合の期待値は [`bind_kind_koma_outside_bindset_is_not_merged`] を見よ。
#[test]
fn koma_only_id_without_bind_pattern0_is_drawn() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["blink.png"]);
    let map = id_to_path(&atlas, &["blink.png"]);

    // host は bind animation を一切持たない（静的 element も無し）。
    let host = surface_with_anims(1000, Vec::new(), Vec::new());
    let blink = surface(1412, vec![elem(0, "blink.png", 0, 0)]);
    let shell = shell_of(vec![host, blink]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // BindSet は空。コマだけが surface 1412 を Overlay で駆動する。
    let binds = BindSet::default();
    let mut pattern = PatternState::default();
    pattern.set(1400, koma(1412, ComposeMethod::Overlay, 0, 0));

    let ops = ops(&world, &atlas, 1000, &binds, &pattern);

    // pattern0 が無くてもコマは駆動される（union 合流・4.6 まばたき再生）。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["blink.png"],
        "bind pattern0 を持たない id のコマ単独でも描画（union 合流）"
    );
}

// ── 合流条件の補強＝最後の砦（bindopt D9-3・bindopt 7.1/7.4/7.6）──────────────────────────
//
// 実機の表情固着は「bind から外れたパーツの保持コマが掃除されない」ことが根因で、掃除は
// state.rs（発行前除去）→ looper.rs（進行相の停止）→ plan.rs（合流条件）の 3 段で多重化する。
// 本段は最後の砦: **`bind+random` のコマは現在の bind 集合に属するときだけ**描画対象へ合流する。
// 上流 2 段が漏れても外れたパーツが表示に残らない不変量をここで成立させる（bindopt 7.1）。純 `bind`
// のコマは呼ばれて始まったものだけなので無効でも合流し（seriko-pattern-calls 5.6）、bind に属さない
// アニメ（純 `random` の `interval` 等）も無条件で合流する（bindopt 7.4）。
// 合成順・z-order の規則は 1 行も変えない（作者意図どおり・design Out of Boundary）。

/// 最後の砦の共通フィクスチャ: host=1000 に 3 本のアニメを置き、対応する参照先 surface を用意する。
///
/// - id=1300: **bind 種**（`Interval::Bind`）・pattern0→surface1301（`eyebase.png`）＝目のベース。
/// - id=1402: **bind 種**（`Interval::BindRandom`）・**pattern0 なし**（index=1 のみ）→surface1413
///   （`jito.png`）＝実機で固着したジト目まばたき（視覚寄与が `PatternState` 単独）。
/// - id=1500: **非 bind**（`Interval::Random`）・pattern0 なし（index=1 のみ）→surface1501
///   （`sparkle.png`）＝bind に属さない抽選発火アニメ。
///
/// 戻り値は (world, atlas, id→path 対応表)。
fn build_last_stand_world() -> (EmoWorld, AtlasTable, Vec<(ElementId, &'static str)>) {
    let base = Path::new("shell/master");
    let rels = ["eyebase.png", "jito.png", "sparkle.png"];
    let atlas = bake_atlas(base, &rels);
    let map = id_to_path(&atlas, &rels);

    let host = surface_with_anims(
        1000,
        Vec::new(), // 静的 element なし（層はすべて bind／コマ由来）。
        vec![
            bind_anim(1300, 1301, 0, 0),
            single_pattern_anim(1402, Interval::BindRandom { k: 4 }, 1, 1413),
            single_pattern_anim(1500, Interval::Random { k: 4 }, 1, 1501),
        ],
    );
    let eyebase = surface(1301, vec![elem(0, "eyebase.png", 0, 0)]);
    let jito = surface(1413, vec![elem(0, "jito.png", 0, 0)]);
    let sparkle = surface(1501, vec![elem(0, "sparkle.png", 0, 0)]);
    let shell = shell_of(vec![host, eyebase, jito, sparkle]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas, map)
}

/// **RED 核**・受入基準・bindopt 7.1・D9-3: bind 集合から外れた **bind 種**アニメの
/// 保持コマは、合成計画にその層を作らない（最後の砦）。
///
/// binds={1300} に対し `PatternState` は 1402（`bind+random`・bind 非所属＝外れたジト目の残留コマ）と
/// 1500（非 bind の抽選発火コマ）を持つ。合流条件に bind 所属の要求が無ければ 1402 の `jito.png` が
/// 描画されて落ち、要求があれば 1402 だけが落ちる。
///
/// **陽性対照を同一検証内に併置する**（負の錨の恒真化を防ぐ）: 同じ 1 回の導出で
/// 1300 の `eyebase.png` と 1500 の `sparkle.png` が**現に描画される**ことを同時に固定するため、
/// 「何も描画されないから jito も無い」という空虚な一致にならない。
#[test]
fn bind_kind_koma_outside_bindset_is_not_merged() {
    let (world, atlas, map) = build_last_stand_world();

    // 1402 は bind 集合に**属さない**（排他置換で外れた／掃除漏れの残留コマ）。
    let binds = BindSet::from_ids([1300]);
    let mut pattern = PatternState::default();
    pattern.set(1402, koma(1413, ComposeMethod::Overlay, 0, 0));
    pattern.set(1500, koma(1501, ComposeMethod::Overlay, 0, 0));

    let ops = ops(&world, &atlas, 1000, &binds, &pattern);
    let paths = ops_to_paths(&ops, &map);

    // 負の錨: 外れた bind 種 id=1402 の層（jito.png）は現れない（bindopt 7.1）。
    assert!(
        !paths.iter().any(|p| p == "jito.png"),
        "bind 非所属の bind 種アニメの保持コマは合成計画へ入らない（bindopt 7.1・最後の砦）: {paths:?}"
    );
    // 陽性対照＋合成順の不変（既定 descend＝id 昇順描画で 1300 → 1500）。
    assert_eq!(
        paths,
        vec!["eyebase.png", "sparkle.png"],
        "有効 bind の pattern0 と非 bind コマは従来どおり id 昇順で描画される（陽性対照・bindopt 7.4）"
    );
}

/// bindopt 7.4: **bind に属さない**アニメ（純 `random` の `interval`）のコマは、
/// bind 集合が空でも従来どおり無条件で合流する。
///
/// 補強したゲートが bind 種を超えて効いていないことの錨。①の陽性対照を単独条件（空 BindSet）で
/// もう一度固定し、「binds が空なら全部落ちる」という過剰な締めすぎを直接反証する。
#[test]
fn non_bind_koma_merges_regardless_of_bindset() {
    let (world, atlas, map) = build_last_stand_world();

    // bind 集合は空。非 bind の id=1500 のコマだけを与える。
    let binds = BindSet::default();
    let mut pattern = PatternState::default();
    pattern.set(1500, koma(1501, ComposeMethod::Overlay, 0, 0));

    let ops = ops(&world, &atlas, 1000, &binds, &pattern);

    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["sparkle.png"],
        "bind に属さないアニメのコマは BindSet に関係なく合流する（bindopt 7.4）"
    );
}

/// 回帰の錨・bindopt 7.4／要件 4.6: bind 集合に**属している** bind 種アニメの
/// コマは従来どおり合流し、pattern0 を持たない再生アニメ（まばたき相当）も描画される。
///
/// ①の裏返し。ゲートが「bind 種なら常に落とす」へ振れていないことを固定し、合流の可否が
/// **bind 所属**という 1 点のみで分かれることを（①と対で）決定づける。
#[test]
fn bind_kind_koma_inside_bindset_still_merges() {
    let (world, atlas, map) = build_last_stand_world();

    // 1402 が bind 集合に**属している**（再生中の正常系）。
    let binds = BindSet::from_ids([1300, 1402]);
    let mut pattern = PatternState::default();
    pattern.set(1402, koma(1413, ComposeMethod::Overlay, 0, 0));
    pattern.set(1500, koma(1501, ComposeMethod::Overlay, 0, 0));

    let ops = ops(&world, &atlas, 1000, &binds, &pattern);

    // 既定 descend＝id 昇順描画で 1300 → 1402 → 1500（合成順は無改変）。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["eyebase.png", "jito.png", "sparkle.png"],
        "bind 所属の bind 種アニメのコマは従来どおり合流し、合成順も不変（bindopt 7.4）"
    );
}

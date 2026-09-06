//! # actor_region_warn_tests — 折返し警告は「装着ごとに 1 件」（結線層・兄弟テスト）
//!
//! 出典 spec: `areka-P0-emo2-conformance-e2e`（要件 **14.1**〜**14.4**・design **D14**）。
//!
//! ## なぜ登録口が持つのか
//!
//! 粗いバルーン定義（折返し基準が描画範囲の遠辺の外）を知らせる警告は、かつて
//! [`TextRegion::resolve`](crate::region::TextRegion::resolve) の中にあった。しかし
//! 再追従シーム（[`TextLayerRuntime::refresh_actor_binding`]）は churn ガードの判定キーを
//! 得るために解決を**毎フレーム**通すため、実機の一周走行では生ログ 30,837 行のうち
//! **27,908 行**が同じ警告になった（走行 A・2026-09-06 の実測）。件数が意味を持たなくなり、
//! 生ログが読めなくなる。
//!
//! 「読み込み（装着）1 回につき 1 件」という意味を持つのは、解決する側ではなく **actor の
//! 登録口**（[`TextLayerRuntime::register_actor`]）である。そこで解決からは粗さの記録を外し
//! （`region_inline_limit_tests.rs` が「0 件」を固定する）、警告は登録口が
//! 「解決済み領域の値が新しく決まったとき」だけ書く。本ファイルはその件数と欄を固定する。
//!
//! ## 固定するもの
//!
//! 1. 粗いバルーンの**装着で 1 件**——欄は `balloon`・`axis`・`wrap_threshold`・`inline_limit`
//!    の 4 つで、文言は上流と 1 文字も違わない（手順書 §5.7 の grep 語・要件 14.3）。
//! 2. 値の変わらない**再追従を 3 回**繰り返しても**追加 0 件**（churn ガードに達する経路）。
//! 3. binding だけが変わって領域が同値の再追従も **0 件**——再構築は起きる（`true`）のに
//!    警告は出ない、という形で「判定は領域の値である」ことを示す。
//! 4. 領域の値が**変わる**再追従は **1 件**（新しい値で）。
//! 5. 折返し基準が遠辺の**内**のバルーンは装着でも **0 件**。
//! 6. 縦書きの粗いバルーンは軸欄が `y` になる。
//!
//! ## 0 件の主張が恒真にならないようにする
//!
//! 件数を見るテストは捕捉窓の内側で対照の `error!` を 1 件発行し、その 1 件が数えられて
//! いることを件数の主張と同時に確かめる（`region_inline_limit_tests.rs` と同じ流儀）。
//!
//! ## 決定論
//!
//! 実 DPI モニタ・実 GPU・実フォント・実窓を要さない。文字列 2 層の写像と純粋層の解決、
//! そして登録口の登録だけで完結する（描画資源は触らない——`present_frame` を呼ばない）。

use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_sakura::contract::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::World;
use log_capture_kit::{CapturedEvent, capture};

use super::test_support::spawn_reserved_slot;
use super::{TextLayerRuntime, TextSlotBinding};
use crate::state::TextLayerConfig;

/// 相方側 `balloonk0.png` の原寸（image px）。数値の出所は `region_inline_limit_tests.rs`
/// および `tests/shipped_fixture_region_test.rs` 冒頭の解決結果の表と同一。
const KERO_IMAGE: (u32, u32) = (288, 203);
/// 相方側の描画範囲の右辺＝`validrect.right,-48` → 288−48。
const KERO_RIGHT: f32 = 240.0;
/// 相方側の描画範囲の下辺＝`validrect.bottom,-70` → 203−70。
const KERO_BOTTOM: f32 = 133.0;
/// 相方側の折返し基準＝共通 `descript.txt` の `wordwrappoint.x,-34` を継ぐ → 288−34。
const KERO_WRAP_X: f32 = 254.0;

/// 本体側 `balloons0.png` の原寸（image px）。
const SAKURA_IMAGE: (u32, u32) = (400, 224);

/// 値を変えるための別原寸（幅だけ 288 → 300）。折返し基準 300−34＝**266** は
/// 右辺 300−48＝**252** の外に残るので、粗いままで値だけが変わる。
const WIDER_IMAGE: (u32, u32) = (300, 203);
/// 別原寸での描画範囲の右辺。
const WIDER_RIGHT: f32 = 252.0;
/// 別原寸での折返し基準。
const WIDER_WRAP_X: f32 = 266.0;

/// 警告の文言（`actor.rs` の発行点と 1 文字も違わないこと自体を固定する——手順書 §5.7 の
/// grep 語であり、変えると実機走行の判定が静かに壊れる・要件 14.3）。
const WARN_MESSAGE: &str = "折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ）";

/// 出荷 fixture `emo2-kakukaku` の共通 `descript.txt` から関連キーだけを写した基層。
///
/// 行の連結を `concat!` で書くのは、本ファイルの改行が CRLF であっても文字列リテラルに
/// 復帰文字が紛れ込まないようにするためである。
const DESCRIPT: &str = concat!(
    "wordwrappoint.x,-34\n",
    "wordwrappoint.y,0\n",
    "validrect.top,0\n",
    "validrect.bottom,0\n",
    "validrect.left,0\n",
    "validrect.right,0\n",
);

/// 相方側の面別上書き層（`balloonk0s.txt`・`wordwrappoint` を上書き**しない**＝粗い側）。
const KERO_OVERLAY: &str = concat!(
    "validrect.top,40\n",
    "validrect.bottom,-70\n",
    "validrect.left,24\n",
    "validrect.right,-48\n",
);

/// 本体側の面別上書き層（`balloons0s.txt`・`wordwrappoint.x` を自ら上書きする＝粗くない側）。
const SAKURA_OVERLAY: &str = concat!(
    "wordwrappoint.x,-49\n",
    "validrect.top,46\n",
    "validrect.bottom,-56\n",
    "validrect.left,36\n",
    "validrect.right,-44\n",
);

/// 相方側と同じ粗さを縦書きで作る単層定義（`wordwrappoint.y,-10` → 203−10＝193 が
/// 描画範囲の下辺 133 の外）。`vertical,1` は SSP 正典キー＝縦書き（右から左へ列送り）。
const VERTICAL_COARSE: &str = concat!(
    "vertical,1\n",
    "wordwrappoint.x,-34\n",
    "wordwrappoint.y,-10\n",
    "validrect.top,40\n",
    "validrect.bottom,-70\n",
    "validrect.left,24\n",
    "validrect.right,-48\n",
);

/// 2 層マージ（共通基層＋面別上書き層）を本番と同じ写像経路で通した `BalloonModel`。
fn merged(overlay: &str) -> BalloonModel {
    parse_str(DESCRIPT, Some(overlay))
}

/// 単層のバルーン定義（縦書きの分岐を作るための最小の入力）。
fn single_layer(source: &str) -> BalloonModel {
    parse_str(source, None)
}

/// 捕捉窓の中で `f` を走らせ、`(戻り値, WARN イベント一覧, ERROR 件数)` を返す。
///
/// 対照の `error!` を窓の内側で 1 件発行するのは、「WARN が 0 件」という主張が捕捉窓の
/// 死によって恒真になるのを防ぐためである（呼出側は ERROR 件数 1 を必ず併せて確認する）。
fn capturing<R>(f: impl FnOnce() -> R) -> (R, Vec<CapturedEvent>, usize) {
    let (value, events) = capture(|| {
        tracing::error!("捕捉窓が生きていることの対照イベント");
        f()
    });
    let warns: Vec<CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .cloned()
        .collect();
    let errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .count();
    (value, warns, errors)
}

/// 数値欄を f32 として読む（`{:?}` 表現の細部に依存しないよう、解析してから比べる）。
fn number_field(event: &CapturedEvent, name: &str) -> f32 {
    let raw = event
        .field(name)
        .unwrap_or_else(|| panic!("欄 {name} が警告に載っていない"));
    raw.parse::<f32>()
        .unwrap_or_else(|_| panic!("欄 {name} の値 {raw} を数値として読めない"))
}

/// 4 欄と文言を丸ごと確かめる（軸・折返し基準・遠辺は呼出側が期待値を渡す）。
fn assert_coarse_warning(warn: &CapturedEvent, axis: &str, wrap_threshold: f32, inline_limit: f32) {
    assert_eq!(
        warn.message(),
        WARN_MESSAGE,
        "文言は上流と 1 文字も違ってはならない（手順書 §5.7 の grep 語・要件 14.3）"
    );
    assert_eq!(warn.field_str("axis"), Some(axis), "行内軸の欄");
    assert_eq!(number_field(warn, "wrap_threshold"), wrap_threshold);
    assert_eq!(number_field(warn, "inline_limit"), inline_limit);
    let balloon = warn
        .field_str("balloon")
        .expect("欄 balloon が警告に載っていない");
    assert!(
        !balloon.is_empty(),
        "バルーン名の欄を空にしてはならない（名前が無いときもプレースホルダで記録する）"
    );
}

/// 装着済みのランタイムを 1 つ作る土台（World は予約スロットの entity を得るためだけに使う）。
fn runtime_with_slot(world: &mut World) -> (TextLayerRuntime, Entity, Entity) {
    let (window, slot) = spawn_reserved_slot(world);
    (
        TextLayerRuntime::new(TextLayerConfig::default()),
        window,
        slot,
    )
}

/// 装着用の binding（k=1・物理寸は image 原寸と同値＝本ファイルは寸法を判定に用いない）。
fn binding(slot: Entity, window: Entity, image: (u32, u32)) -> TextSlotBinding {
    TextSlotBinding::new(slot, window, 1.0, image, image)
}

// ── 要件 14.2／14.3: 装着 1 回につき 1 件・欄と文言は不変 ──

/// 粗い相方側バルーン（折返し基準 254 > 遠辺 240）の**装着**は警告をちょうど 1 件記録し、
/// 4 つの欄（バルーン名・軸・折返し基準・遠辺）を載せる。
#[test]
fn attaching_a_coarse_balloon_warns_once_with_balloon_axis_and_both_values() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("1");

    let (_, warns, errors) = capturing(|| {
        rt.register_actor_binding(
            actor.clone(),
            binding(slot, window, KERO_IMAGE),
            &merged(KERO_OVERLAY),
        );
    });

    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の件数の主張は証拠にならない"
    );
    assert_eq!(
        warns.len(),
        1,
        "粗いバルーンの装着は警告をちょうど 1 件記録する（要件 14.2）: {warns:?}"
    );
    assert_coarse_warning(&warns[0], "x", KERO_WRAP_X, KERO_RIGHT);
}

/// 縦書きの粗いバルーンでは行内軸が y へ切り替わり、欄の値も y 側（193 と下辺 133）になる。
#[test]
fn attaching_a_vertical_coarse_balloon_warns_with_the_inline_axis_of_the_block_direction() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("1");

    let (_, warns, errors) = capturing(|| {
        rt.register_actor_binding(
            actor.clone(),
            binding(slot, window, KERO_IMAGE),
            &single_layer(VERTICAL_COARSE),
        );
    });

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    assert_eq!(
        warns.len(),
        1,
        "縦書きでも粗いバルーンの装着は警告をちょうど 1 件記録する: {warns:?}"
    );
    assert_coarse_warning(&warns[0], "y", 193.0, KERO_BOTTOM);
}

/// 折返し基準が遠辺の内にある本体側バルーン（351 ≤ 356）は装着でも警告を出さない
/// （警告が「どのバルーンでも出る」ものでないことの対照・要件 14.4）。
#[test]
fn attaching_a_balloon_with_wrap_threshold_inside_the_range_does_not_warn() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");

    let (_, warns, errors) = capturing(|| {
        rt.register_actor_binding(
            actor.clone(),
            binding(slot, window, SAKURA_IMAGE),
            &merged(SAKURA_OVERLAY),
        );
    });

    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の 0 件の主張は証拠にならない"
    );
    assert_eq!(
        warns.len(),
        0,
        "折返し基準が描画範囲の内にあるバルーンは警告を記録しない: {warns:?}"
    );
}

// ── 要件 14.2／14.4: 再追従は「値が変わったときだけ」 ──

/// 値の変わらない再追従を 3 回繰り返しても追加の警告は 0 件——毎フレーム走る経路で
/// 警告が積み上がらないことの本体（走行 A の 27,908 行はここで消える）。
#[test]
fn repeated_refresh_with_unchanged_values_adds_no_warning() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("1");
    // 装着そのものの 1 件は捕捉窓の外で済ませ、以降の追加分だけを数える。
    rt.register_actor_binding(
        actor.clone(),
        binding(slot, window, KERO_IMAGE),
        &merged(KERO_OVERLAY),
    );

    let (changed, warns, errors) = capturing(|| {
        let mut changed = Vec::new();
        for _ in 0..3 {
            changed.push(rt.refresh_actor_binding(
                &actor,
                binding(slot, window, KERO_IMAGE),
                &merged(KERO_OVERLAY),
            ));
        }
        changed
    });

    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の 0 件の主張は証拠にならない"
    );
    assert_eq!(
        changed,
        vec![false, false, false],
        "判定キー全同値の再追従は no-op（前提の確認）"
    );
    assert_eq!(
        warns.len(),
        0,
        "値の変わらない再追従では警告を追加しない（要件 14.2）: {warns:?}"
    );
}

/// binding だけが変わって解決済み領域が同値の再追従は、**再構築は起きる**のに警告は 0 件。
/// 判定が「churn ガードに達しなかった」ではなく「領域の値が変わっていない」であることを示す。
#[test]
fn refresh_that_rebuilds_but_keeps_the_same_region_does_not_warn() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("1");
    rt.register_actor_binding(
        actor.clone(),
        binding(slot, window, KERO_IMAGE),
        &merged(KERO_OVERLAY),
    );

    // image 原寸は据え置き（＝領域は同値）で k と物理寸だけを変える。
    let scaled = TextSlotBinding::new(slot, window, 2.0, (576, 406), KERO_IMAGE);
    let (changed, warns, errors) =
        capturing(|| rt.refresh_actor_binding(&actor, scaled, &merged(KERO_OVERLAY)));

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    assert!(
        changed,
        "k と物理寸が変われば再追従は起きる（前提の確認——登録口には達している）"
    );
    assert_eq!(
        rt.layout_input[&actor].region.wrap_threshold(),
        KERO_WRAP_X,
        "前提: 領域の値は据え置き"
    );
    assert_eq!(
        warns.len(),
        0,
        "領域の値が同じなら、登録口に達しても警告は出ない（要件 14.2）: {warns:?}"
    );
}

/// 解決済み領域の値が変わる再追従は、新しい値でもう 1 件記録する（要件 14.2 の後半）。
#[test]
fn refresh_with_a_changed_region_warns_once_with_the_new_values() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("1");
    rt.register_actor_binding(
        actor.clone(),
        binding(slot, window, KERO_IMAGE),
        &merged(KERO_OVERLAY),
    );

    // 面が別寸へ切り替わる（`\b` 相当）——粗さは残るが値が変わる。
    let (changed, warns, errors) = capturing(|| {
        rt.refresh_actor_binding(
            &actor,
            binding(slot, window, WIDER_IMAGE),
            &merged(KERO_OVERLAY),
        )
    });

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    assert!(changed, "面実寸が変われば再追従は起きる（前提の確認）");
    assert_eq!(
        warns.len(),
        1,
        "領域の値が変わった再追従は警告を 1 件記録する（要件 14.2）: {warns:?}"
    );
    assert_coarse_warning(&warns[0], "x", WIDER_WRAP_X, WIDER_RIGHT);
}

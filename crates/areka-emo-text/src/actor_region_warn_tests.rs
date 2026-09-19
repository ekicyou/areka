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
//! ## `origin` の警告も同じ登録口が書く（本 spec 要件 3.1〜3.4／5.6）
//!
//! 範囲外ゆえ無視した `origin` 宣言の警告も、同じ理由で同じ登録口が書く——件数の規律も
//! 同じである（装着で成分 1 つにつき 1 件・値の変わらない再追従で 0 件）。同じ捕捉窓に
//! 2 種類の警告が混ざりうるので、後半の群は文言で選り分けてから数える（総数 0 件を
//! 主張するときだけは選り分けない）。欄は `balloon`・`key`・`resolved`・`range_min`・
//! `range_max`・`corner` の 6 つ。
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
use crate::region::TextRegion;
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;

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

// ── 本 spec 要件 3.1〜3.4／5.6: 無視した `origin` 宣言は成分 1 つにつき 1 件 ──
//
// 上の 6 本は `origin` を宣言しない入力なので、下の追加分とは別の警告を見ている。
// 同じ捕捉窓に 2 種類の警告が混ざりうるため、下の群は**文言で選り分けてから**数える
// （総数 0 件を主張するときだけは選り分けない——選り分けない 0 のほうが強い主張である）。

/// 無視した `origin` 宣言の警告の文言（`actor_decoration.rs` の発行点と 1 文字も違わないこと
/// 自体を固定する——折返しの警告と同じ捕捉窓で選り分ける鍵がこの文字列だからである・要件 3.3）。
const IGNORED_ORIGIN_MESSAGE: &str = "origin に指定した文字の書き始めの位置が、文字を描いてよい範囲（validrect）の外にある——指定は使わず、範囲の書き始めの角から書いた";

/// 基層へ足す `origin` 宣言——両成分とも範囲の外（本体側の 36..356／46..168 に対し 0）。
const ORIGIN_BOTH_OUTSIDE: &str = concat!("origin.x,0\n", "origin.y,0\n");
/// x だけが範囲の外（y の 100 は 46..168 の内）。
const ORIGIN_ONLY_X_OUTSIDE: &str = concat!("origin.x,0\n", "origin.y,100\n");
/// `ORIGIN_ONLY_X_OUTSIDE` と**無視される x の値だけ**が違う宣言（5 も左辺 36 の外）。
const ORIGIN_ONLY_X_OUTSIDE_AT_FIVE: &str = concat!("origin.x,5\n", "origin.y,100\n");
/// 両成分とも範囲の内（100 は 36..356 の内であり 46..168 の内でもある）。
const ORIGIN_BOTH_INSIDE: &str = concat!("origin.x,100\n", "origin.y,100\n");

/// 本体側の描画範囲の左辺（`SAKURA_IMAGE` 400×224 と `SAKURA_OVERLAY` の `validrect.left,36`）。
/// 左 36 ≠ 上 46・右 356 ≠ 下 168 と四辺が非対称なので、成分と辺の対応が入れ替われば露見する。
const SAKURA_LEFT: f32 = 36.0;
/// 本体側の上辺（`validrect.top,46`）。
const SAKURA_TOP: f32 = 46.0;
/// 本体側の右辺（`validrect.right,-44` → 400−44）。
const SAKURA_RIGHT: f32 = 356.0;
/// 本体側の下辺（`validrect.bottom,-56` → 224−56）。
const SAKURA_BOTTOM: f32 = 168.0;

/// 領域の値を変えるための別原寸（420×240）。遠辺だけが動き、折返し基準 420−49＝371 は
/// 右辺 420−44＝376 の内に留まる（＝折返しの警告は出ないまま値だけが変わる）。
const SAKURA_WIDER_IMAGE: (u32, u32) = (420, 240);
/// 別原寸での右辺（420−44）。
const SAKURA_WIDER_RIGHT: f32 = 376.0;
/// 別原寸での下辺（240−56）。
const SAKURA_WIDER_BOTTOM: f32 = 184.0;

/// 相方側の描画範囲の左辺（`KERO_OVERLAY` の `validrect.left,24`）。
const KERO_LEFT: f32 = 24.0;
/// 相方側の上辺（`validrect.top,40`）。
const KERO_TOP: f32 = 40.0;

/// `origin` 宣言を足した基層と面別上書き層の 2 層マージ（写像経路は `merged` と同じ）。
fn merged_with_origin(origin: &str, overlay: &str) -> BalloonModel {
    parse_str(&format!("{DESCRIPT}{origin}"), Some(overlay))
}

/// 捕捉した WARN のうち、無視した `origin` 宣言の警告だけを取り出す。
fn ignored_origin_warns(warns: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    warns
        .iter()
        .filter(|e| e.message() == IGNORED_ORIGIN_MESSAGE)
        .collect()
}

/// 6 つの欄と文言を丸ごと確かめる（成分名・無視した解決値・範囲の両端・実際に用いた角）。
///
/// バルーン名の代替値だけは上流の定数を借りずに綴りを直に書く。定数と突き合わせると
/// 「上流が空文字へ変わっても両辺が揃って通る」ので、空でないことを何も確かめられない。
fn assert_ignored_origin_warning(
    warn: &CapturedEvent,
    key: &str,
    resolved: f32,
    range_min: f32,
    range_max: f32,
    corner: f32,
) {
    assert_eq!(
        warn.message(),
        IGNORED_ORIGIN_MESSAGE,
        "文言は上流と 1 文字も違ってはならない（要件 3.3）"
    );
    assert_eq!(warn.field_str("key"), Some(key), "無視した成分の名前の欄");
    assert_eq!(
        number_field(warn, "resolved"),
        resolved,
        "無視した宣言の解決値の欄"
    );
    assert_eq!(
        number_field(warn, "range_min"),
        range_min,
        "範囲の手前端の欄（x なら左辺・y なら上辺）"
    );
    assert_eq!(
        number_field(warn, "range_max"),
        range_max,
        "範囲の奥端の欄（x なら右辺・y なら下辺）"
    );
    assert_eq!(
        number_field(warn, "corner"),
        corner,
        "実際に用いた書き始めの角の欄"
    );
    assert_eq!(
        warn.field_str("balloon"),
        Some("(名前なし)"),
        "バルーン名の欄は名前が無いときも空にせず代替値を載せる（要件 3.3）"
    );
}

/// 両成分が範囲の外のバルーンの装着は、成分 1 つにつき 1 件——ちょうど 2 件を記録する。
#[test]
fn attaching_a_balloon_with_both_origin_components_outside_warns_once_per_component() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");

    let (_, warns, errors) = capturing(|| {
        rt.register_actor_binding(
            actor.clone(),
            binding(slot, window, SAKURA_IMAGE),
            &merged_with_origin(ORIGIN_BOTH_OUTSIDE, SAKURA_OVERLAY),
        );
    });

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    let ignored = ignored_origin_warns(&warns);
    assert_eq!(
        ignored.len(),
        2,
        "両成分が範囲の外なら装着で 2 件（要件 3.1／3.2）: {warns:?}"
    );
    assert_ignored_origin_warning(
        ignored[0],
        "origin.x",
        0.0,
        SAKURA_LEFT,
        SAKURA_RIGHT,
        SAKURA_LEFT,
    );
    assert_ignored_origin_warning(
        ignored[1],
        "origin.y",
        0.0,
        SAKURA_TOP,
        SAKURA_BOTTOM,
        SAKURA_TOP,
    );
}

/// 片方の成分だけが範囲の外なら 1 件だけ——範囲の内の成分は警告を生まない（要件 3.4）。
#[test]
fn attaching_a_balloon_with_only_one_origin_component_outside_warns_once() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");

    let (_, warns, errors) = capturing(|| {
        rt.register_actor_binding(
            actor.clone(),
            binding(slot, window, SAKURA_IMAGE),
            &merged_with_origin(ORIGIN_ONLY_X_OUTSIDE, SAKURA_OVERLAY),
        );
    });

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    let ignored = ignored_origin_warns(&warns);
    assert_eq!(
        ignored.len(),
        1,
        "範囲の外の成分は 1 つだけなので 1 件（要件 3.1／3.2）: {warns:?}"
    );
    assert_ignored_origin_warning(
        ignored[0],
        "origin.x",
        0.0,
        SAKURA_LEFT,
        SAKURA_RIGHT,
        SAKURA_LEFT,
    );
}

/// 範囲の内に宣言したバルーンの装着は WARN を 1 件も出さない——ここだけは文言で選り分けず
/// **総数**を 0 と主張する（選り分けない 0 のほうが強い主張である・要件 3.4／5.6）。
#[test]
fn attaching_a_balloon_with_origin_inside_the_range_does_not_warn_at_all() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");

    let (_, warns, errors) = capturing(|| {
        rt.register_actor_binding(
            actor.clone(),
            binding(slot, window, SAKURA_IMAGE),
            &merged_with_origin(ORIGIN_BOTH_INSIDE, SAKURA_OVERLAY),
        );
    });

    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の 0 件の主張は証拠にならない"
    );
    assert_eq!(
        warns.len(),
        0,
        "範囲の内の宣言は警告を生まない（総数で 0 件・要件 3.4）: {warns:?}"
    );
}

/// 値の変わらない再追従を 3 回繰り返しても、無視した `origin` の警告は 1 件も増えない
/// （毎フレーム走る経路で警告が積み上がらない・要件 3.2）。
#[test]
fn repeated_refresh_with_unchanged_values_adds_no_ignored_origin_warning() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");
    // 装着そのものの 2 件は捕捉窓の外で済ませ、以降の追加分だけを数える。
    rt.register_actor_binding(
        actor.clone(),
        binding(slot, window, SAKURA_IMAGE),
        &merged_with_origin(ORIGIN_BOTH_OUTSIDE, SAKURA_OVERLAY),
    );

    let (changed, warns, errors) = capturing(|| {
        let mut changed = Vec::new();
        for _ in 0..3 {
            changed.push(rt.refresh_actor_binding(
                &actor,
                binding(slot, window, SAKURA_IMAGE),
                &merged_with_origin(ORIGIN_BOTH_OUTSIDE, SAKURA_OVERLAY),
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
        ignored_origin_warns(&warns).len(),
        0,
        "値の変わらない再追従では警告を追加しない（要件 3.2）: {warns:?}"
    );
}

/// binding だけが変わって領域が同値の再追従は、**再構築は起きる**のに警告は 0 件
/// （判定が「登録口に達したか」ではなく「領域の値が変わったか」であることを示す・要件 3.2）。
#[test]
fn refresh_that_rebuilds_but_keeps_the_same_region_does_not_warn_about_origin() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");
    rt.register_actor_binding(
        actor.clone(),
        binding(slot, window, SAKURA_IMAGE),
        &merged_with_origin(ORIGIN_BOTH_OUTSIDE, SAKURA_OVERLAY),
    );
    let before = rt.layout_input[&actor].region;

    // image 原寸は据え置き（＝領域は同値）で k と物理寸だけを変える。
    let scaled = TextSlotBinding::new(slot, window, 2.0, (800, 448), SAKURA_IMAGE);
    let (changed, warns, errors) = capturing(|| {
        rt.refresh_actor_binding(
            &actor,
            scaled,
            &merged_with_origin(ORIGIN_BOTH_OUTSIDE, SAKURA_OVERLAY),
        )
    });

    assert_eq!(
        errors, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の 0 件の主張は証拠にならない"
    );
    assert!(
        changed,
        "k と物理寸が変われば再追従は起きる（前提の確認——登録口には達している）"
    );
    assert_eq!(
        rt.layout_input[&actor].region, before,
        "前提: 領域の値は据え置き"
    );
    assert_eq!(
        ignored_origin_warns(&warns).len(),
        0,
        "領域の値が同じなら、登録口に達しても警告は出ない（要件 3.2）: {warns:?}"
    );
}

/// 原寸が変わって領域の値も変わる再追従は、成分 1 つにつき 1 件を**新しい値で**出し直す。
#[test]
fn refresh_with_a_changed_region_warns_again_with_the_new_values() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");
    rt.register_actor_binding(
        actor.clone(),
        binding(slot, window, SAKURA_IMAGE),
        &merged_with_origin(ORIGIN_BOTH_OUTSIDE, SAKURA_OVERLAY),
    );

    let (changed, warns, errors) = capturing(|| {
        rt.refresh_actor_binding(
            &actor,
            binding(slot, window, SAKURA_WIDER_IMAGE),
            &merged_with_origin(ORIGIN_BOTH_OUTSIDE, SAKURA_OVERLAY),
        )
    });

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    assert!(changed, "面実寸が変われば再追従は起きる（前提の確認）");
    let ignored = ignored_origin_warns(&warns);
    assert_eq!(
        ignored.len(),
        2,
        "領域の値が変われば成分 1 つにつき 1 件を出し直す（要件 3.2）: {warns:?}"
    );
    assert_ignored_origin_warning(
        ignored[0],
        "origin.x",
        0.0,
        SAKURA_LEFT,
        SAKURA_WIDER_RIGHT,
        SAKURA_LEFT,
    );
    assert_ignored_origin_warning(
        ignored[1],
        "origin.y",
        0.0,
        SAKURA_TOP,
        SAKURA_WIDER_BOTTOM,
        SAKURA_TOP,
    );
}

/// 領域の差が**無視した宣言の値だけ**でも、警告は新しい値で出し直す（要件 3.2）。
///
/// 直前の 1 本は面の原寸を変えて値を動かすので、遠辺・折返し基準・原寸も一緒に動く。
/// それらのどれか 1 つでも判定に効いていれば通ってしまうので、「無視した値が変われば
/// 出し直す」という主張そのものは固定できていない。本 1 本は**無視した値以外のすべての
/// 欄が同値**の 2 つの領域を作り、差がその 1 欄だけであることをテストの中で先に確かめて
/// から、件数と欄を主張する。
#[test]
fn refresh_that_only_changes_the_ignored_origin_value_warns_again_with_the_new_value() {
    // 前提の確認: 2 つの定義から解決した領域は、無視した宣言の値だけが違う。
    let resolve = |origin: &str| {
        TextRegion::resolve(
            &merged_with_origin(origin, SAKURA_OVERLAY),
            SAKURA_IMAGE,
            WritingMode::HorizontalTb,
        )
    };
    let before = resolve(ORIGIN_ONLY_X_OUTSIDE);
    let after = resolve(ORIGIN_ONLY_X_OUTSIDE_AT_FIVE);
    let other_fields = |r: &TextRegion| {
        (
            r.left(),
            r.top(),
            r.right(),
            r.bottom(),
            r.start(),
            r.wrap_threshold(),
            r.inline_limit(),
            r.image_size(),
        )
    };
    assert_eq!(
        other_fields(&before),
        other_fields(&after),
        "無視した値以外の欄まで動いてしまうと、この検査は直前の 1 本と同じものに退化する"
    );
    assert_eq!(
        (before.ignored_origin(), after.ignored_origin()),
        ((Some(0.0), None), (Some(5.0), None)),
        "動くのは x 成分の無視した値だけ（y の 100 は 46〜168 の内なので無視されない）"
    );

    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("0");
    rt.register_actor_binding(
        actor.clone(),
        binding(slot, window, SAKURA_IMAGE),
        &merged_with_origin(ORIGIN_ONLY_X_OUTSIDE, SAKURA_OVERLAY),
    );

    // binding は装着時と同値のまま——動かすのはバルーン定義の origin.x だけ。
    let (changed, warns, errors) = capturing(|| {
        rt.refresh_actor_binding(
            &actor,
            binding(slot, window, SAKURA_IMAGE),
            &merged_with_origin(ORIGIN_ONLY_X_OUTSIDE_AT_FIVE, SAKURA_OVERLAY),
        )
    });

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    assert!(
        changed,
        "無視した宣言の値が変われば領域は別物であり、再追従は起きる（前提の確認）"
    );
    let ignored = ignored_origin_warns(&warns);
    assert_eq!(
        ignored.len(),
        1,
        "無視した値が変われば、その成分について 1 件を出し直す（要件 3.2）: {warns:?}"
    );
    assert_ignored_origin_warning(
        ignored[0],
        "origin.x",
        5.0,
        SAKURA_LEFT,
        SAKURA_RIGHT,
        SAKURA_LEFT,
    );
}

/// 2 種類の警告が同じ装着で同時に出ても、互いの件数・欄・文言を乱さない（要件 3.3）。
/// 粗い相方側バルーンに範囲外の `origin` を足すと、折返しの 1 件と `origin` の 2 件で総数 3 件。
#[test]
fn ignored_origin_warnings_share_the_window_with_the_coarse_wrap_warning() {
    let mut world = World::new();
    let (mut rt, window, slot) = runtime_with_slot(&mut world);
    let actor = ActorKey::from("1");

    let (_, warns, errors) = capturing(|| {
        rt.register_actor_binding(
            actor.clone(),
            binding(slot, window, KERO_IMAGE),
            &merged_with_origin(ORIGIN_BOTH_OUTSIDE, KERO_OVERLAY),
        );
    });

    assert_eq!(errors, 1, "捕捉窓の対照イベントが数えられていない");
    assert_eq!(
        warns.len(),
        3,
        "折返しの 1 件と origin の 2 件で総数 3 件: {warns:?}"
    );
    let ignored = ignored_origin_warns(&warns);
    assert_eq!(
        ignored.len(),
        2,
        "選り分けた origin の警告は 2 件: {warns:?}"
    );
    assert_ignored_origin_warning(
        ignored[0], "origin.x", 0.0, KERO_LEFT, KERO_RIGHT, KERO_LEFT,
    );
    assert_ignored_origin_warning(ignored[1], "origin.y", 0.0, KERO_TOP, KERO_BOTTOM, KERO_TOP);

    let coarse: Vec<&CapturedEvent> = warns
        .iter()
        .filter(|e| e.message() == WARN_MESSAGE)
        .collect();
    assert_eq!(
        coarse.len(),
        1,
        "折返しの警告は相方の別警告に影響されず 1 件のまま: {warns:?}"
    );
    assert_coarse_warning(coarse[0], "x", KERO_WRAP_X, KERO_RIGHT);
}

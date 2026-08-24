use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand};
use bevy_ecs::prelude::World;

use super::test_support::{com_world, cue, geo_model, opaque_count, spawn_reserved_slot};
use super::{TextLayerRuntime, TextSlotBinding, present_frame};
use crate::state::TextLayerConfig;

// ══ task 7.1: 文字層 k 再追従シーム（R8.1/8.2/8.3/8.5/8.7・design D11） ══
//
// `TextSlotView` は emo-present の私有フィールド型で in-crate から構築できない
// （公開コンストラクタなし・`text_slot_view` は実 GPU 表示確立が前提）。よって本檻は
// 公開口 `refresh_actor_scale` が `TextSlotBinding::from_view` の直後に委譲する内側シーム
// `refresh_actor_binding`（binding 直渡し・以降の判断分岐は完全に同一）を駆動する。
// view 経由の全経路（`from_view` の物理寸読み取りを含む）は GPU 統合テストの領分（task 7.3）。

/// 代表 native 原寸（emo2 balloon 相当）と、k を通した物理寸（`scaled_extent` 相当の実値）。
/// k=1.25: 400×1.25=500 / 224×1.25=280。k=2: 800 / 448。
const NATIVE: (u32, u32) = (400, 224);

/// R4.5/R8.5（churn ガード）: **判定キーが全同値**（binding 全体＝k・物理寸・image 原寸・
/// slot・window ＋ 再解決した文字描画領域）の再追従要求は **false** を返し、routing／
/// layout 入力を 1 バイトも動かさない（毎フレーム再結線の禁止）。
///
/// 「k が同値」は no-op の**十分条件ではない**——寸／領域が変われば再構築する
/// （`refresh_actor_binding_with_same_k_but_different_image_size_rebuilds` /
/// `refresh_actor_binding_with_same_binding_but_changed_region_rebuilds`・R4.4）。
#[test]
fn refresh_actor_binding_with_all_keys_equal_is_noop_returning_false() {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_binding(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.25, (500, 280), NATIVE),
        &geo_model(),
    );
    let binding_before = rt.routing[&actor];
    let resolved_before = rt.layout_input[&actor].clone();

    let changed = rt.refresh_actor_binding(
        &actor,
        TextSlotBinding::new(slot, window, 1.25, (500, 280), NATIVE),
        &geo_model(),
    );

    assert!(
        !changed,
        "判定キーが全同値の再追従要求は no-op で false（R4.5/R8.5）"
    );
    assert_eq!(rt.routing[&actor], binding_before, "binding は不変");
    assert_eq!(
        rt.layout_input[&actor], resolved_before,
        "layout 入力は不変（判定用の再解決は等値ゆえ上書きが起きない）"
    );
}

/// R4.4: **k が同値でも面実寸（`image_size`）が変われば再構築する**。
/// 「k 同値なら常に no-op」という旧判定では、同 k のまま別寸のバルーン面へ切替えた
/// （`\b` 等）ときに旧寸の文字層が残る——判定キーを binding 全体へ広げてこれを閉塞する。
#[test]
fn refresh_actor_binding_with_same_k_but_different_image_size_rebuilds() {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_binding(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.25, (500, 280), NATIVE),
        &geo_model(),
    );
    let region_before = rt.layout_input[&actor].region;

    // k は据え置き（1.25）で面だけ別寸へ——物理寸も image 原寸も変わる。
    const NARROW: (u32, u32) = (320, 180);
    let changed = rt.refresh_actor_binding(
        &actor,
        TextSlotBinding::new(slot, window, 1.25, (400, 225), NARROW),
        &geo_model(),
    );

    assert!(changed, "k 同値でも面実寸が違えば再構築する（R4.4）");
    let after = rt.routing[&actor];
    assert_eq!(after.scale, 1.25, "k は同値のまま");
    assert_eq!(
        after.image_size, NARROW,
        "image 原寸は新しい面の値へ更新される"
    );
    assert_eq!(
        after.surface_size,
        (400, 225),
        "物理寸も新しい面の値へ更新される"
    );
    assert_ne!(
        rt.layout_input[&actor].region, region_before,
        "文字描画領域も新しい面実寸で解き直される（旧寸の領域を残さない）"
    );
}

/// R4.4: **binding が全同値でも、再解決した文字描画領域が変われば再構築する**
/// （scope 別 `validrect` の差し替え等——判定キーの後半 `ResolvedBalloonText` 側）。
#[test]
fn refresh_actor_binding_with_same_binding_but_changed_region_rebuilds() {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let binding = TextSlotBinding::new(slot, window, 1.25, (500, 280), NATIVE);
    rt.register_actor_binding(actor.clone(), binding, &geo_model());
    let region_before = rt.layout_input[&actor].region;

    // binding は 1 バイトも変えず、model の validrect だけが別 scope の値へ変わった状況。
    let narrowed = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(16), Some(200), Some(24), Some(360)),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    let changed = rt.refresh_actor_binding(&actor, binding, &narrowed);

    assert!(
        changed,
        "binding 同値でも文字描画領域が違えば再構築する（R4.4）"
    );
    assert_eq!(rt.routing[&actor], binding, "binding は同値のまま");
    assert_ne!(
        rt.layout_input[&actor].region, region_before,
        "layout 入力は新しい validrect の領域へ更新される"
    );
}

/// R8.1/R8.2: k 変化で binding が新 k へ再構築され、**image px 空間は不変**
/// （image_size／解決済み region が k に依らない＝作者画像空間）。物理寸が k 倍で
/// 伸びるのはこの不変性が前提（`physical = ceil(region × k)`）。
#[test]
fn refresh_actor_scale_rebuilds_binding_at_new_k_keeping_image_space() {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_binding(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.25, (500, 280), NATIVE),
        &geo_model(),
    );
    assert_eq!(
        rt.routing[&actor].image_size, NATIVE,
        "k=1.25 の物理寸 500×280 から image px 原寸 400×224 が導出される"
    );
    let region_before = rt.layout_input[&actor].region;

    let changed = rt.refresh_actor_binding(
        &actor,
        TextSlotBinding::new(slot, window, 2.0, (800, 448), NATIVE),
        &geo_model(),
    );

    assert!(changed, "k 変化の再追従は true（R8.1）");
    let after = rt.routing[&actor];
    assert_eq!(after.scale, 2.0, "binding の k が新 k へ更新される");
    assert_eq!(after.surface_size, (800, 448), "物理原寸は新 k の値");
    assert_eq!(
        after.image_size, NATIVE,
        "image px 原寸は k 不変（作者画像空間・R8.2 の ceil(validrect×k) が k に比例する前提）"
    );
    assert_eq!(
        rt.layout_input[&actor].region, region_before,
        "解決済み region（全値 image px）も k 不変——k は描画行列と供給面寸にだけ効く"
    );
}

/// R8.1/R8.2/R8.3（COM・headless）: 再追従は `ActorRender`（供給面・executor・metrics）を
/// 破棄し、次 `present_frame` が **新 k の物理寸**で再生成する。その間、純粋状態
/// （リビール進行・確定行＝`TextLayerState`）は保存される（`Clear`/`ClearAll` と別物）。
#[test]
fn refresh_actor_scale_discards_render_and_preserves_reveal_state() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
    // k=1.0・物理 120×60＝image 120×60（validrect 未指定＝画像全域ゆえ供給面も 120×60）。
    rt.register_actor_binding(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, (120, 60), (120, 60)),
        &geo_model(),
    );
    present_frame(&mut rt, &mut world, 10.0).expect("初回提示（装着）");
    assert!(
        rt.is_attached(&actor),
        "初回提示で ActorRender が生成される"
    );
    assert_eq!(
        rt.surface(&actor).expect("供給面").size(),
        (120, 60),
        "k=1.0 の供給面物理寸"
    );
    let items_before = rt
        .state()
        .actor_state(&actor)
        .expect("actor 状態")
        .items()
        .len();
    let visible_before = rt.state().visible_glyphs(&actor, 10.0);
    assert!(visible_before > 0, "リビール済みグリフがある前提");

    // ── k=1.0 → 2.0 の再追従（物理 240×120＝image 120×60・image 空間は不変） ──
    let changed = rt.refresh_actor_binding(
        &actor,
        TextSlotBinding::new(slot, window, 2.0, (240, 120), (120, 60)),
        &geo_model(),
    );
    assert!(changed, "k 変化の再追従は true");
    assert!(
        !rt.is_attached(&actor),
        "ActorRender は破棄される（次フレームが新 k で再生成する・R8.2）"
    );
    // R8.3: リビール状態は破棄されない（Clear/ClearAll とは構造的に別物）。
    assert_eq!(
        rt.state()
            .actor_state(&actor)
            .expect("再追従後も actor 状態は残る")
            .items()
            .len(),
        items_before,
        "確定行・グリフ列は保存される（R8.3）"
    );
    assert_eq!(
        rt.state().visible_glyphs(&actor, 10.0),
        visible_before,
        "リビール進行も保存される（R8.3）"
    );

    // ── 次フレーム: 新 k の物理寸で再生成され、保存済み状態から全再描画される ──
    present_frame(&mut rt, &mut world, 10.0).expect("再追従後の提示");
    assert!(rt.is_attached(&actor), "次フレームで再装着される");
    assert_eq!(
        rt.surface(&actor).expect("供給面").size(),
        (240, 120),
        "供給面は新 k の物理寸 ceil(region×k) で再生成される（R8.2・旧寸の再利用禁止）"
    );
    let bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert!(
        opaque_count(&bytes) > 0,
        "保存済み状態から新 k で再描画される（空表示へ落ちない・R8.4 の前提）"
    );

    // ── R8.5（churn ガードの実効）: 同値 k の再要求は ActorRender を破棄しない ──
    // 装着済みの状態で「何もしない」ことを観測する（毎フレーム再結線＝供給面の作り直しと
    // 全再描画を毎フレーム走らせる変異は、ここで is_attached／統計が動くことで死ぬ）。
    //
    // **キルの排他性（実測・task 7.3 で再測）**: 「判定キー全同値でも ActorRender を破棄する」
    // 変異は本ケースが殺すが**排他キルではない**——task 7.3 の統合檻
    // `attach_wiring_test::scale_refresh_logs_k_transition_and_reattach_physical_size`
    // （同値 k のフレームで装着 `info!` が再発火しないことを見る）も同時に落ちる＝**共倒れ 2 本**。
    // 「churn ガードそのものを撤去する（全同値でも true を返して再構築する）」変異も
    // 上の `refresh_actor_binding_with_all_keys_equal_is_noop_returning_false` と共倒れである。
    let stats_before = rt.draw_stats(&actor).expect("draw_stats");
    let noop = rt.refresh_actor_binding(
        &actor,
        TextSlotBinding::new(slot, window, 2.0, (240, 120), (120, 60)),
        &geo_model(),
    );
    assert!(!noop, "同値 k は false（R8.5）");
    assert!(
        rt.is_attached(&actor),
        "同値 k は ActorRender を破棄しない（churn ガード・R8.5）"
    );
    let stats_after = rt.draw_stats(&actor).expect("draw_stats");
    assert_eq!(
        (
            stats_after.line_layout_creations,
            stats_after.full_clears,
            stats_after.draw_text_layout_calls
        ),
        (
            stats_before.line_layout_creations,
            stats_before.full_clears,
            stats_before.draw_text_layout_calls
        ),
        "同値 k は描画実行部にも一切触れない（行 TextLayout キャッシュを捨てない）"
    );
}

/// 未登録 actor への再追従要求は no-op で false（装着は `register_actor_view` の領分——
/// 再追従が第 2 の装着経路にならない）。
#[test]
fn refresh_actor_scale_for_unregistered_actor_is_noop() {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());

    let changed = rt.refresh_actor_binding(
        &actor,
        TextSlotBinding::new(slot, window, 2.0, (800, 448), NATIVE),
        &geo_model(),
    );

    assert!(!changed, "未登録 actor は再構築対象が無い＝false");
    assert!(
        !rt.routing.contains_key(&actor),
        "再追従は装着経路ではない（routing を生やさない）"
    );
}

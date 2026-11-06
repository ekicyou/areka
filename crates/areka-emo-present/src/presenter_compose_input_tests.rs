use super::*;

use areka_emo_compose::BindSet;

use wintf::ecs::{HitTest, HitTestMode, Visual};

use super::test_support::{
    bake_world, bind_overlay_surface, build_two_face_assets, elem, gradient, make_world_with_gpu,
    mount_entities, native_golden, native_golden_with, pattern_overlay_at, shell_of, show_ok,
    show_with_ok, spawn_window_with_dpi, surface,
};

/// surface 1000（`w×h` 全不透明 element ＋ bind animation 2000 が surface 5000 を (0,0) に重ねる）
/// の `(EmoWorld, AtlasTable)` と、bind 無し／bind 有りそれぞれの直接合成 golden を返す。
///
/// 5000 の part（1×1 不透明・base と異色）は base 内に収まるため、bind 有無で**外形は不変・
/// バイトのみ変わる**（外形の変化を伴わずに bind 差分の表示反映だけを固定できる）。
fn build_target_assets_with_bind(
    w: u32,
    h: u32,
    salt: u8,
) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
    let bind_part = surface(5000, vec![elem("q.png", 0, 0)]);
    let base_surface = bind_overlay_surface(0, 0);
    // 5000 は 1×1 の不透明 part（base 左上と必ず異なる色 → bind 有無でバイトが必ず変わる）。
    let (world, atlas) = bake_world(
        shell_of(vec![base_surface, bind_part]),
        vec![
            ("p.png", w, h, gradient(w, h, salt)),
            ("q.png", 1, 1, vec![0xFF, 0xFF, 0xFF, 0xFF]),
        ],
    );

    let golden_plain = native_golden(&world, &atlas, 1000).0;
    let golden_bound = native_golden_with(
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([2000]),
        &PatternState::default(),
    )
    .0;
    assert_ne!(
        golden_plain, golden_bound,
        "fixture 前提: bind 有無で合成バイトが異ならなければ回帰檻にならない"
    );

    (world, atlas, golden_plain, golden_bound)
}

/// 回帰檻（キャッシュ仕様バグ・実表示レベル）: **同一 surface id で bind 集合だけ変えた**
/// `ShowSurface` が必ず再合成され、`read_back` が各 bind 状態の直接合成 golden とバイト一致する。
///
/// 旧設計（surface id のみキー）では 2 回目以降が古い合成にヒットし、着せ替え・まばたきの
/// bind 差分が表示に反映されなかった（2026-07-09 まばたきデモで顕在化）。往復（無し→有り→無し）
/// で両方向の再合成を固定する。
#[test]
fn bind_change_on_same_surface_updates_display() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, golden_plain, golden_bound) = build_target_assets_with_bind(4, 3, 0x2B);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    let show = |presenter: &mut EmoPresenter, world: &mut World, binds: BindSet| {
        show_with_ok(
            presenter,
            world,
            TargetId(0),
            1000,
            binds,
            PatternState::default(),
        );
    };

    // bind 無し → golden_plain。
    show(&mut presenter, &mut world, BindSet::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "bind 無し表示が直接合成 golden と一致しない"
    );

    // 同一 surface・bind 有り → 再合成されて golden_bound（旧設計はここで古い絵を返した）。
    show(&mut presenter, &mut world, BindSet::from_ids([2000]));
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_bound,
        "bind 追加が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
    );

    // bind 無しへ戻す → 再合成されて golden_plain（往復の両方向を固定）。
    show(&mut presenter, &mut world, BindSet::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "bind 除去が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
    );
}

/// R6.1 観測完了（同寸・異 id 再 Show ＝ 新面提示 ＋ 文字スロット安定）: バルーン target が既に
/// ある面（1000）を表示中に、**同寸の異なる面 id（3000）**を `ShowSurface` すると——(a) reply Ok・
/// (b) 可視維持・(c) `HitTest::AlphaMask` 維持・(d) `read_back` が **新面 3000 の golden** と一致
/// （新面が実際に提示された証跡）・(e) `text_slot_view()`（slot/window/surface_size/scale）が切替の
/// 前後で**完全一致**（文字スロットが安定＝TextSlotView が不変）——をすべて満たす。
///
/// 装着（mount）は初回表示で一度だけ生成され（`apply_show` の `mount.is_none()` 分岐を踏まない）、
/// 予約 text スロット entity は据え置かれる＝emo-text の描画資源を破壊しない
/// （design §emo-present 回帰・文字層＝同寸保持）。本 crate 本体は無改変（test-only・R6.3）。
#[test]
fn reshow_same_size_different_face_keeps_text_slot_stable() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, golden_1000, golden_3000) = build_two_face_assets(6, 5);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    // 面 1000 を表示確立（可視・αマスク判定・装着を遅延生成）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    // 前提: 初回表示は面 1000 の golden（切替前の基準）。
    assert_eq!(
        presenter
            .read_back(TargetId(0))
            .expect("read_back（面 1000）失敗"),
        golden_1000,
        "初回表示が面 1000 の golden と一致しない（前提が崩れている）"
    );

    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));

    // 切替前の文字スロット表示スナップショット（TextSlotView は Copy＝値で退避）。
    let slot_before = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");

    // 同寸・異 id 再 Show（面 3000）。
    // (a) reply Ok。
    show_ok(&mut presenter, &mut world, TargetId(0), 3000);

    // (b) 可視維持。
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "同寸・異 id 再表示後も可視のまま"
    );
    assert!(
        presenter.targets.get(&TargetId(0)).unwrap().visible,
        "同寸・異 id 再表示後も target.visible=true"
    );
    // (c) HitTest::AlphaMask 維持。
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::AlphaMask,
        "同寸・異 id 再表示後も αマスク判定を維持"
    );

    // (d) read_back が新面 3000 の golden と一致（新面が実際に提示された証跡・R6.1）。
    let rb = presenter
        .read_back(TargetId(0))
        .expect("read_back（面 3000）失敗");
    assert_eq!(
        rb, golden_3000,
        "再表示のバイトが新面 3000 の golden と一致しない（新面が提示されていない）"
    );

    // (e) 文字スロット表示が切替の前後で完全一致（slot/window/surface_size/scale が不変・R6.1）。
    let slot_after = presenter
        .text_slot_view(TargetId(0))
        .expect("再表示後の text_slot_view は Some");
    assert_eq!(
        slot_before, slot_after,
        "同寸・異 id 再表示で文字スロット表示（slot/window/surface_size/scale）が変化した（TextSlotView が不安定）"
    );
}

/// surface 1000（`w×h` 全不透明 element）＋ pattern の現在コマが参照する overlay surface 5000
/// （1×1 不透明・base と異色・base 左上に収まる）を同一 world へ載せた `(EmoWorld, AtlasTable)` と、
/// 空 pattern／非空 pattern それぞれの直接合成 golden を返す。
///
/// surface 5000 は surface 1000 の **bind animation ではなく**（1000 に animation を定義しない）、
/// pattern の現在コマ（`PatternFrame{ surface_id: 5000, Overlay, (0,0) }`）としてのみ top-level 合流
/// する（plan.rs: 合流対象 = 有効 bind pattern0 の id ∪ PatternState に現在コマを持つ id）。5000 は
/// 定義層（extent 母集合＝全 element ＋全 bind animation pattern0）に寄与しないため合成外形は base の
/// `w×h` のまま不変で、pattern 有無で**外形は不変・バイトのみ変わる**（chain リサイズ経路を踏まず
/// 「pattern が compose へ届いたか」だけを固定できる）。
fn build_target_assets_with_pattern(
    w: u32,
    h: u32,
    salt: u8,
) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
    // surface 1000: 全不透明 element 1 本（animation は持たない＝bind 非依存）。
    // surface 5000: 1×1 不透明 part（pattern の現在コマが参照する overlay 源・base 左上と必ず異なる色
    // → pattern 有無でバイトが必ず変わる）。
    let (world, atlas) = bake_world(
        shell_of(vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(5000, vec![elem("q.png", 0, 0)]),
        ]),
        vec![
            ("p.png", w, h, gradient(w, h, salt)),
            ("q.png", 1, 1, vec![0xFF, 0xFF, 0xFF, 0xFF]),
        ],
    );

    let golden_plain = native_golden(&world, &atlas, 1000).0;
    let golden_pattern = native_golden_with(
        &world,
        &atlas,
        1000,
        &BindSet::default(),
        &pattern_overlay_at(2000, 5000, 0, 0),
    )
    .0;
    assert_ne!(
        golden_plain, golden_pattern,
        "fixture 前提: pattern 有無で合成バイトが異ならなければ回帰檻にならない"
    );

    (world, atlas, golden_plain, golden_pattern)
}

/// Task 8.2 完了檻（pattern が presenter → compose ＋ cache を実際に貫く・R5.1/5.2/5.4）: 同一
/// `(target, surface_id, binds)` でも `ShowSurface` が運ぶ **pattern が変われば表示が変わる**。
///
/// (1) 空 pattern の Show → `read_back` が空 pattern 直接合成 golden と一致（R5.4: 拡張前と観測等価）。
/// (2) 同一 id・binds のまま **非空 pattern** の Show → `read_back` が非空 pattern 直接合成 golden と
///     一致し、かつ空 pattern の絵と**異なる**（pattern が compose へ届き ComposeKey も pattern 分だけ
///     ミスして再合成された証跡・R5.1/5.2）。(3) 再び空 pattern の Show → 空 golden へ戻る（pattern が
///     キー要素として往復両方向で効く）。presenter が pattern を既定（空）で握り潰していれば (2) が
///     空 golden のままとなり本テストは RED になる。
#[test]
fn show_surface_pattern_flows_through_to_compose_and_cache() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, golden_plain, golden_pattern) =
        build_target_assets_with_pattern(4, 3, 0x3C);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    let show = |presenter: &mut EmoPresenter, world: &mut World, pattern: PatternState| {
        show_with_ok(
            presenter,
            world,
            TargetId(0),
            1000,
            BindSet::default(),
            pattern,
        );
    };

    // (1) 空 pattern → golden_plain（拡張前と観測等価・R5.4）。
    show(&mut presenter, &mut world, PatternState::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "空 pattern の表示が空 pattern 直接合成 golden と一致しない（R5.4）"
    );

    // (2) 同一 id・binds・非空 pattern → 再合成されて golden_pattern（pattern が compose＋cache を貫く証跡）。
    show(
        &mut presenter,
        &mut world,
        pattern_overlay_at(2000, 5000, 0, 0),
    );
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb, golden_pattern,
        "非空 pattern が表示へ反映されない（pattern が compose へ届いていない＝presenter が握り潰している）"
    );

    // (3) 空 pattern へ戻す → golden_plain（pattern がキー要素として往復両方向で効く）。
    show(&mut presenter, &mut world, PatternState::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "空 pattern へ戻した表示が空 golden と一致しない（pattern キー要素の往復が壊れている）"
    );
}

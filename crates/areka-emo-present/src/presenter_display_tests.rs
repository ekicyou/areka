use super::*;

use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::BindSet;

use wintf::ecs::{HitTest, HitTestMode, Visual};

use super::test_support::{
    build_target_assets, elem, make_world_with_gpu, shell_of, spawn_window_with_dpi, surface,
};

/// R2.4/R3.2/R8.2 観測完了（golden 一致）: `attach_target` → `apply(ShowSurface 有効 id)` で reply が
/// `Ok(())`、かつ `read_back` が同一入力の直接合成 golden と**全バイト一致**する。
///
/// 供給面は D2D 非経由の純バイト転送ゆえ、readback と `ComposedSurface.bytes()` のバイト一致が
/// 決定論的に成立する（WARP でも可＝CI 決定論）。
#[test]
fn golden_match_read_back_equals_direct_compose() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, golden) = build_target_assets(3, 2, 0x11);
    assert!(golden.iter().any(|&b| b != 0), "golden は非退化（全 0 でない）");

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );

    let outcome = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("reply（ShowSurface）を受信できない");
    assert!(
        matches!(outcome, Ok(())),
        "有効 id の ShowSurface は Ok を返す: {outcome:?}"
    );

    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb, golden,
        "readback が直接合成 golden とバイト一致しない（表示・供給面の恒等転送が壊れている）"
    );
}

/// R3.4 観測完了（表示不変）: 有効 id で表示を確立後、**解決不能 id** の `ShowSurface` は reply が
/// `Err(Compose(SurfaceNotFound))` で、`read_back` バイトは**適用前と不変**（表示を乱さない）。
#[test]
fn invalid_surface_id_replies_err_and_leaves_display_unchanged() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x5A);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // まず有効 id で表示を確立（供給面生成＋表示バイト確定）。
    let (tx0, rx0) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx0),
        },
    );
    assert!(
        matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "前提の有効 ShowSurface が Ok でない"
    );
    let before = presenter.read_back(TargetId(0)).expect("read_back（前）失敗");

    // 解決不能 id: error! ＋ 表示不変 ＋ reply Err（R3.4）。
    let (tx1, rx1) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 9999,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx1),
        },
    );
    let outcome = rx1
        .recv_timeout(Duration::from_secs(10))
        .expect("reply（無効 id）を受信できない");
    assert!(
        matches!(
            outcome,
            Err(PresentError::Compose(ComposeError::SurfaceNotFound(9999)))
        ),
        "無効 id は Err(Compose(SurfaceNotFound(9999))) を返す: {outcome:?}"
    );

    let after = presenter.read_back(TargetId(0)).expect("read_back（後）失敗");
    assert_eq!(
        before, after,
        "無効 id の適用で表示中バイトが変化した（表示不変の不変条件を破っている）"
    );
}

/// 有効 surface 1000（`w×h` 全不透明 element）＋ 定義層皆無で外形 0×0 に退化する surface 7000
/// （element/animation ゼロ）を**同一 target**へ載せる `(EmoWorld, AtlasTable)` を返す。
///
/// surface 7000 は composer_tests.rs の `no_layers_degenerate_propagates_empty_composition`
/// と同型の構成（bind 済み world に存在するが合成外形 0×0）で、`Composer::compose` が
/// `Err(ComposeError::EmptyComposition(7000))` を返す。単なる全透明 element は非ゼロ外形の
/// `Ok`（`all_transparent_surface_is_ok_transparent_nonzero_extent`）で EmptyComposition では
/// ないため、退化は「定義層皆無 → 0×0」の経路で作る。
fn build_assets_with_valid_and_empty(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    // surface 1000: 全不透明 element 1 本。surface 7000: element/animation ゼロ（定義層皆無）。
    let surfaces = vec![
        surface(1000, vec![elem("p.png", 0, 0)]),
        surface(7000, Vec::new()),
    ];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let a: u8 = 0xFF;
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            img.push(b);
            img.push(g);
            img.push(r);
            img.push(a);
        }
    }
    dec.insert(base.join("p.png"), w, h, stride, img, true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// R3.4 観測完了（skip＋表示不変の回帰檻）: 有効 id で表示・マスクを確立後、**解決不能 id** の
/// `ShowSurface` は reply が `Err(Compose(SurfaceNotFound))`、かつ (a) `read_back` バイト、
/// (b) surface entity の `HitTest`（`AlphaMask`）、(c) `AlphaMaskResource`（設定済みマスク）の
/// いずれも**適用前と不変**（表示＋マスクを一切乱さない）。
///
/// 4.1 の `invalid_surface_id_replies_err_and_leaves_display_unchanged` はバイト不変のみを見るが、
/// 本テストは「skip＝表示器（visual/mask/hit-test）を触らない」を独立・自己完結に固定する。
#[test]
fn invalid_surface_skips_and_leaves_display_and_mask_unchanged() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x37);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 有効 id で表示・マスク・hit-test を確立。
    let (tx0, rx0) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx0),
        },
    );
    assert!(
        matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "前提の有効 ShowSurface が Ok でない"
    );

    // 表示器の適用前状態を捕捉（bytes ＋ HitTest ＋ mask 寸法/有無）。
    let surface_entity = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.mount.as_ref())
        .expect("有効表示後は mount が生成済み")
        .surface_entity();

    let bytes_before = presenter.read_back(TargetId(0)).expect("read_back（前）失敗");
    let hit_before = world
        .get::<HitTest>(surface_entity)
        .expect("surface entity に HitTest が無い")
        .mode;
    assert_eq!(hit_before, HitTestMode::AlphaMask, "有効表示後は αマスク判定");
    let mask_dims_before = world
        .get::<AlphaMaskResource>(surface_entity)
        .and_then(|r| r.mask().map(|m| (m.width(), m.height())));
    assert!(mask_dims_before.is_some(), "有効表示後は AlphaMask が供給済み");
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "有効表示後は可視"
    );

    // 解決不能 id: error! ＋ skip（表示器不触）＋ reply Err（R3.4）。
    let (tx1, rx1) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 4242,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx1),
        },
    );
    let outcome = rx1
        .recv_timeout(Duration::from_secs(10))
        .expect("reply（無効 id）を受信できない");
    assert!(
        matches!(
            outcome,
            Err(PresentError::Compose(ComposeError::SurfaceNotFound(4242)))
        ),
        "無効 id は Err(Compose(SurfaceNotFound(4242))) を返す: {outcome:?}"
    );

    // (a) 表示バイト不変。
    let bytes_after = presenter.read_back(TargetId(0)).expect("read_back（後）失敗");
    assert_eq!(
        bytes_before, bytes_after,
        "無効 id の skip で表示中バイトが変化した（表示を乱さない不変条件違反）"
    );
    // (b) HitTest 不変（None へ落ちていない＝当たり判定が生きたまま）。
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::AlphaMask,
        "無効 id の skip で HitTest が変化した（マスク/当たり判定を乱している）"
    );
    // (c) AlphaMaskResource 不変（供給済みマスクが消えていない）。
    let mask_dims_after = world
        .get::<AlphaMaskResource>(surface_entity)
        .and_then(|r| r.mask().map(|m| (m.width(), m.height())));
    assert_eq!(
        mask_dims_before, mask_dims_after,
        "無効 id の skip で AlphaMaskResource が変化した（マスクを乱している）"
    );
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "無効 id の skip で可視状態が変化した（表示を乱している）"
    );
}

/// 設計ディスカッション #1 観測完了（EmptyComposition → Hide 縮退＋reply Ok）: 有効表示で mount を
/// 確立後、**外形 0×0 に退化する既存 surface**（定義層皆無）を `ShowSurface` すると reply は
/// **`Ok(())`**（`Err` ではない）で、target は Hidden へ縮退（`Visual` 不可視＋`HitTest::none()`）し、
/// 0×0 供給面を作ろうとして panic しない（既存 chain は破棄されず保持）。
///
/// 前段で `Composer::compose(7000)` が `EmptyComposition(7000)` を返すことを直接確認し、退化経路が
/// 「不在 surface（SurfaceNotFound）」ではなく「存在するが 0×0」であることを固定する。
#[test]
fn empty_composition_degrades_to_hidden_and_replies_ok() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas) = build_assets_with_valid_and_empty(5, 4, 0x22);

    // 前提固定: 7000 は「存在するが外形 0×0」＝EmptyComposition（SurfaceNotFound ではない）。
    {
        let mut composer = Composer::new();
        let direct = composer.compose(&emo_world, &atlas, 7000, &BindSet::default(), &PatternState::default());
        assert_eq!(
            direct.err(),
            Some(ComposeError::EmptyComposition(7000)),
            "surface 7000 は定義層皆無で EmptyComposition を返す前提でなければならない"
        );
    }

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 有効 1000 で mount/chain を確立し可視化。
    let (tx0, rx0) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx0),
        },
    );
    assert!(
        matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "前提の有効 ShowSurface が Ok でない"
    );
    let surface_entity = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.mount.as_ref())
        .expect("有効表示後は mount が生成済み")
        .surface_entity();
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "有効表示後は可視"
    );
    let bytes_len_before = presenter
        .read_back(TargetId(0))
        .expect("read_back（前）失敗")
        .len();

    // EmptyComposition 退化: warn! ＋ Hide 縮退 ＋ reply Ok（skip でも Err でもない）。
    let (tx1, rx1) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 7000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx1),
        },
    );
    let outcome = rx1
        .recv_timeout(Duration::from_secs(10))
        .expect("reply（EmptyComposition）を受信できない");
    assert!(
        matches!(outcome, Ok(())),
        "EmptyComposition は Hide 縮退＋reply Ok（Err ではない）: {outcome:?}"
    );

    // Hidden へ縮退: Visual 不可視 ＋ HitTest::none()。
    assert!(
        !world.get::<Visual>(surface_entity).unwrap().is_visible,
        "EmptyComposition は Hidden へ縮退（Visual 不可視）でなければならない"
    );
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::None,
        "EmptyComposition は当たり判定停止（HitTest::none）でなければならない"
    );
    assert!(
        !presenter.targets.get(&TargetId(0)).unwrap().visible,
        "EmptyComposition 後は target.visible=false"
    );

    // 0×0 供給面は作らない: 既存 chain は破棄されず保持（read_back は旧外形の長さのまま成立）。
    let bytes_len_after = presenter
        .read_back(TargetId(0))
        .expect("EmptyComposition 後も既存 chain は保持され read_back できる")
        .len();
    assert_eq!(
        bytes_len_before, bytes_len_after,
        "EmptyComposition で 0×0 chain へ差し替わった（既存 chain 保持の不変条件違反）"
    );
    // 7000 は非合成ゆえキャッシュへ載らない（0×0 を挿入しない）。
    assert!(
        presenter
            .targets
            .get(&TargetId(0))
            .unwrap()
            .cache
            .get(7000, &BindSet::default(), &PatternState::default(), ScaleRatio::ONE)
            .is_none(),
        "EmptyComposition は cache へ 0×0 を挿入しない"
    );
}

/// R3.3 観測完了（Hide → 再 ShowSurface 復帰）: 有効表示 → `Hide`（不可視＋`HitTest::none()`＋
/// chain/cache 保持）→ 同一有効 id を再 `ShowSurface` で表示復帰（可視＋`HitTest::alpha_mask()`）。
/// 再表示はキャッシュヒットで再合成せず、`read_back` が初回表示バイトと一致する（キャッシュからの復帰）。
#[test]
fn hide_then_reshow_recovers_display_from_cache() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x4D);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 初回表示（可視・αマスク判定確立）。
    let (tx0, rx0) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx0),
        },
    );
    assert!(
        matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "初回 ShowSurface が Ok でない"
    );
    let surface_entity = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.mount.as_ref())
        .expect("初回表示後は mount が生成済み")
        .surface_entity();
    let bytes_shown = presenter.read_back(TargetId(0)).expect("read_back（初回）失敗");
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::AlphaMask,
        "初回表示後は αマスク判定"
    );

    // Hide: 不可視 ＋ HitTest::none() ＋ chain/cache 保持。
    let (txh, rxh) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::Hide {
            target: TargetId(0),
            reply: Some(txh),
        },
    );
    assert!(
        matches!(rxh.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "Hide が Ok でない"
    );
    assert!(
        !world.get::<Visual>(surface_entity).unwrap().is_visible,
        "Hide 後は Visual 不可視"
    );
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::None,
        "Hide 後は当たり判定停止（HitTest::none）"
    );
    {
        let target = presenter.targets.get(&TargetId(0)).unwrap();
        assert!(target.chain.is_some(), "Hide は swap chain を保持する（R3.3）");
        assert!(
            target.cache.get(1000, &BindSet::default(), &PatternState::default(), ScaleRatio::ONE).is_some(),
            "Hide は合成キャッシュを保持する（R3.3）"
        );
        assert!(!target.visible, "Hide 後は target.visible=false");
    }

    // 再 ShowSurface（同一有効 id）: キャッシュヒットで再合成せず表示復帰。
    let (tx1, rx1) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx1),
        },
    );
    assert!(
        matches!(rx1.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "再 ShowSurface が Ok でない"
    );

    // 表示復帰: 可視 ＋ αマスク判定へ戻る。
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "再表示後は可視へ復帰"
    );
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::AlphaMask,
        "再表示後は αマスク判定へ復帰"
    );
    assert!(
        presenter.targets.get(&TargetId(0)).unwrap().visible,
        "再表示後は target.visible=true"
    );

    // 観測可能な復帰: read_back が初回表示バイトと一致（キャッシュからの復帰）。
    let bytes_reshown = presenter.read_back(TargetId(0)).expect("read_back（再表示）失敗");
    assert_eq!(
        bytes_shown, bytes_reshown,
        "再表示のバイトが初回表示と一致しない（キャッシュからの表示復帰が壊れている）"
    );
}

/// R9.1/9.2 観測完了（mount 未生成＝取得不可）: 未登録 target・登録済みだが初回 `ShowSurface` 前
/// （mount 遅延生成前）のいずれも `text_slot_view` が `None` を返す（取得結果が空）。
///
/// mount 未生成経路は World に GPU 資源を要しない（`attach_target` は skeleton 登録のみ）ため、
/// 素の `World` で決定論的に固定する。
#[test]
fn text_slot_view_is_none_before_display_established() {
    let mut world = World::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x66);

    let mut presenter = EmoPresenter::new();
    // 未登録 target: 取得結果は空。
    assert!(
        presenter.text_slot_view(TargetId(0)).is_none(),
        "未登録 target の text_slot_view は None"
    );

    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    // 登録済みでも初回 ShowSurface 前（mount 未生成）は空（design: mount は遅延生成・R9.2）。
    assert!(
        presenter.text_slot_view(TargetId(0)).is_none(),
        "初回 ShowSurface 前（mount 未生成）の text_slot_view は None"
    );
}

/// R9.1/9.2 観測完了（表示確立後の正値）: 有効 `ShowSurface` で表示確立後、`text_slot_view` が
/// `Some` を返し、(a) `slot()` ＝ mount の予約スロット（`Name("emo-text-layer-slot")` を持つ）、
/// (b) `window()` ＝ 装着先窓 Entity、(c) `surface_size()` ＝ バルーン/シェル surface の物理 px 原寸、
/// (d) `scale()` ＝ 現行の物理 1:1 表示契約の恒常値 1.0、をすべて満たす。
#[test]
fn text_slot_view_returns_slot_window_size_scale_after_display() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x77);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "前提の有効 ShowSurface が Ok でない"
    );

    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");

    // (a) slot ＝ mount の予約スロット（Name で二重に裏取り）。
    let expected_slot = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.mount.as_ref())
        .expect("表示確立後は mount が生成済み")
        .text_slot();
    assert_eq!(view.slot(), expected_slot, "slot() が予約スロット entity と一致しない");
    let name = world
        .get::<bevy_ecs::name::Name>(view.slot())
        .expect("予約スロットに Name が無い");
    assert_eq!(name.as_str(), "emo-text-layer-slot");

    // (b) window ＝ attach_target で渡した装着先窓。
    assert_eq!(view.window(), window, "window() が装着先窓 entity と一致しない");

    // (c) surface_size ＝ 合成原寸（物理 px・本 fixture は 3×2）。
    assert_eq!(view.surface_size(), (3, 2), "surface_size() が物理原寸と一致しない");

    // (d) scale ＝ 本 fixture の窓 DPI（96）÷ author_dpi（96）＝ 1.0。
    //     恒常値ではなく**この入力での**期待値（k≠1.0 の檻は別テストが所有）。
    assert_eq!(view.scale(), 1.0, "窓 DPI 96 / author_dpi 96 ゆえ scale() は 1.0");
}

/// `areka-P0-recompose-budget` Requirement 3.1 ／ 設計 Flow 2「容量回収は合成成功後に限る」観測完了:
/// 有効 id で表示を確立したあと**解決不能 id** の適用で合成が失敗しても、合成メモのスロットは
/// 適用前のエントリを**中身ごと**保持し続ける。ゆえに直後の同一入力の適用は再合成せず引き当てで
/// 済み、表示も適用前のまま成立する。
///
/// # なぜ既存の失敗経路の檻では足りないのか（5.1 → 5.3 の申し送り）
///
/// [`invalid_surface_skips_and_leaves_display_and_mask_unchanged`] は表示バイト・`HitTest`・
/// `AlphaMaskResource` の不変を見るが、これらはいずれも「失敗した適用は供給面へ再転写しない」ことの
/// 帰結であり、**スロットが空になったかどうかとは独立**である——空にしても再転写は起きないので
/// バイトは 1 つも変わらない。したがって `ComposeCache::take_recycled`（追い出しエントリの容量回収）を
/// 合成の成否判定より**手前**へ置く誤りは、既存の檻を丸ごとすり抜ける。本檻はスロットそのものを
/// 直接読み、Flow 2 の規律を固定する唯一の観測点である。
///
/// # 「引き当てで済む」の観測形
///
/// `apply_show` が `cache_hit` を決めるのは `cache.get(surface_id, &binds, &pattern, k)` ちょうど
/// その呼び出しであり、k は窓 DPI と政策から導出される（本檻では窓 DPI 不変ゆえ `applied` と同一）。
/// よって同じ引数での `get` が `Some` であることは「次の同一入力の適用が引き当てで済む」ことと同値である。
#[test]
fn compose_failure_keeps_the_cache_slot_so_the_next_identical_apply_still_hits() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x6C);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 有効 id でスロットを埋める（表示成立＝表示バッファとマスクの原子対が入る）。
    let (tx0, rx0) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx0),
        },
    );
    assert!(
        matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "前提の有効 ShowSurface が Ok でない"
    );

    // 適用に使われた k（窓 DPI 96 ÷ author_dpi 96 ＝ 恒等）。スロットのキー要素そのものを使う。
    let applied = presenter
        .targets
        .get(&TargetId(0))
        .expect("装着済み target")
        .applied
        .expect("表示成立後は適用 k が入る");

    // 適用前のスロットの中身を控える（バイトとマスク寸＝原子対の両側）。
    let (bytes_before, mask_dims_before) = {
        let entry = presenter
            .targets
            .get(&TargetId(0))
            .expect("装着済み target")
            .cache
            .get(1000, &BindSet::default(), &PatternState::default(), applied)
            .expect("前提: 表示成立でスロットが埋まる");
        (
            entry.composed.bytes().to_vec(),
            (entry.mask.width(), entry.mask.height()),
        )
    };
    assert!(
        bytes_before.iter().any(|&b| b != 0),
        "前提: スロットの表示バッファは非退化（全 0 でない）"
    );
    let display_before = presenter
        .read_back(TargetId(0))
        .expect("read_back（前）失敗");

    // 合成失敗（解決不能 id）: error! ＋ 表示不変 ＋ reply Err（R3.4）。
    let (tx1, rx1) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 9999,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx1),
        },
    );
    assert!(
        matches!(
            rx1.recv_timeout(Duration::from_secs(10)),
            Ok(Err(PresentError::Compose(ComposeError::SurfaceNotFound(
                9999
            ))))
        ),
        "前提: 解決不能 id は Err(Compose(SurfaceNotFound(9999)))"
    );

    // ここが本題。スロットは**空になっていない**。
    let entry = presenter
        .targets
        .get(&TargetId(0))
        .expect("装着済み target")
        .cache
        .get(1000, &BindSet::default(), &PatternState::default(), applied);
    let entry = entry.expect(
        "合成失敗でキャッシュスロットが空になった＝`take_recycled` が合成の成否判定より手前に         置かれている（設計 Flow 2「容量回収は合成成功後に限る」の違反）。以後の同一入力の適用は         引き当てに失敗して毎回再合成へ落ちる",
    );
    // 中身も適用前のまま（対の片側だけ差し替わっていない）。
    assert_eq!(
        entry.composed.bytes(),
        bytes_before.as_slice(),
        "合成失敗でスロットの表示バッファ内容が変化した（失敗した適用が中身を書き換えている）"
    );
    assert_eq!(
        (entry.mask.width(), entry.mask.height()),
        mask_dims_before,
        "合成失敗でスロットのマスクが差し替わった（原子対の片側だけ動いている）"
    );

    // 直後の同一入力の適用は成立し、表示も適用前と全バイト一致する。
    let (tx2, rx2) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx2),
        },
    );
    assert!(
        matches!(rx2.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "合成失敗の直後でも同一入力の適用は成立する"
    );
    assert_eq!(
        display_before,
        presenter
            .read_back(TargetId(0))
            .expect("read_back（後）失敗"),
        "合成失敗を挟んだ再適用で表示バイトが変化した"
    );
}

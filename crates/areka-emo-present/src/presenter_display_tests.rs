use super::*;

use areka_emo_compose::BindSet;

use wintf::ecs::{HitTest, HitTestMode, Visual};

use super::test_support::{
    INFO_LINE_MESSAGE, bake_world, bind_overlay_surface, build_assets_with_valid_and_empty,
    build_target_assets, capture, elem, gradient, hide_ok, make_world_with_gpu, mount_entities,
    on_screen, shell_of, show_ok, show_with, spawn_window_with_dpi, surface,
};

/// skip＋表示不変の回帰檻: 有効 id で表示を確立後、**解決不能 id** の
/// `ShowSurface` は reply が `Err(Compose(SurfaceNotFound))`、かつ (a) `read_back` バイトと
/// 画面の節（＝画面へ渡っている描画そのもの）、(b) surface entity の `HitTest`（`SurfaceAlpha`）の
/// いずれも**適用前と不変**（表示も当たりも一切乱さない）。
///
/// 「skip＝表示器（visual/hit-test）を触らない」を独立・自己完結に固定する。
#[test]
fn invalid_surface_skips_and_leaves_display_and_hit_unchanged() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x37);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));

    let bytes_before = presenter
        .read_back(TargetId(0))
        .expect("read_back（前）失敗");
    // 「表示」＝ 画面の節。`read_back` は合成メモの原寸バイトを返すので、画面へ渡っている当のものは
    // こちらで読む。
    let display_before = on_screen(&presenter, TargetId(0)).expect("有効表示後は節が在る");
    assert!(
        !display_before.items.is_empty(),
        "有効表示後の節が空（前提が成立していない）"
    );
    let hit_before = world
        .get::<HitTest>(surface_entity)
        .expect("surface entity に HitTest が無い")
        .mode;
    assert_eq!(
        hit_before,
        HitTestMode::SurfaceAlpha,
        "有効表示後は入れ物の α の判定"
    );
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "有効表示後は可視"
    );

    let outcome = show_with(
        &mut presenter,
        &mut world,
        TargetId(0),
        4242,
        BindSet::default(),
        PatternState::default(),
    );
    assert!(
        matches!(
            outcome,
            Err(PresentError::Compose(ComposeError::SurfaceNotFound(4242)))
        ),
        "無効 id は Err(Compose(SurfaceNotFound(4242))) を返す: {outcome:?}"
    );

    let bytes_after = presenter
        .read_back(TargetId(0))
        .expect("read_back（後）失敗");
    assert_eq!(
        bytes_before, bytes_after,
        "無効 id の skip で表示中バイトが変化した（表示を乱さない不変条件違反）"
    );
    // (a′) 画面へ渡っている描画そのものも不変（節を差し替えていない）。
    assert_eq!(
        on_screen(&presenter, TargetId(0)),
        Some(display_before),
        "無効 id の skip で節が差し替わった（表示を乱さない不変条件違反）"
    );
    // (b) HitTest 不変（None へ落ちていない＝当たり判定が生きたまま）。
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::SurfaceAlpha,
        "無効 id の skip で HitTest が変化した（当たり判定を乱している）"
    );
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "無効 id の skip で可視状態が変化した（表示を乱している）"
    );
}

/// （EmptyComposition → Hide 縮退＋reply Ok）: 有効表示で mount を
/// 確立後、**外形 0×0 に退化する既存 surface**（定義層皆無）を `ShowSurface` すると reply は
/// **`Ok(())`**（`Err` ではない）で、target は Hidden へ縮退（`Visual` 不可視＋`HitTest::none()`）し、
/// 0×0 の節を作ろうとして panic しない（既存のメモと節は破棄されず保持）。
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
        let direct = composer.compose(
            &emo_world,
            &atlas,
            7000,
            &BindSet::default(),
            &PatternState::default(),
        );
        assert_eq!(
            direct.err(),
            Some(ComposeError::EmptyComposition(7000)),
            "surface 7000 は定義層皆無で EmptyComposition を返す前提でなければならない"
        );
    }

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "有効表示後は可視"
    );
    let bytes_len_before = presenter
        .read_back(TargetId(0))
        .expect("read_back（前）失敗")
        .len();
    let display_before = on_screen(&presenter, TargetId(0));

    // EmptyComposition 退化: warn! ＋ Hide 縮退 ＋ reply Ok（skip でも Err でもない）。
    let outcome = show_with(
        &mut presenter,
        &mut world,
        TargetId(0),
        7000,
        BindSet::default(),
        PatternState::default(),
    );
    assert!(
        matches!(outcome, Ok(())),
        "EmptyComposition は Hide 縮退＋reply Ok（Err ではない）: {outcome:?}"
    );

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

    // 0×0 のメモは作らない: 既存のメモは破棄されず保持（read_back は旧外形の長さのまま成立）。
    let bytes_len_after = presenter
        .read_back(TargetId(0))
        .expect("EmptyComposition 後も既存のメモは保持され read_back できる")
        .len();
    assert_eq!(
        bytes_len_before, bytes_len_after,
        "EmptyComposition で 0×0 のメモへ差し替わった（既存メモ保持の不変条件違反）"
    );
    assert_eq!(
        on_screen(&presenter, TargetId(0)),
        display_before,
        "EmptyComposition で節が差し替わった（既存の節保持の不変条件違反）"
    );
    assert!(
        presenter
            .targets
            .get(&TargetId(0))
            .unwrap()
            .cache
            .get(7000, &BindSet::default(), &PatternState::default())
            .is_none(),
        "EmptyComposition は cache へ 0×0 を挿入しない"
    );
}

/// 外形が上限を越える面（`ExtentTooLarge`・合成が error! 済み）も `EmptyComposition` と同じく隠す縮退＋
/// reply Ok（申し合わせ 11-05）。面 1000 は着せ替えの先 5000 を一辺の上限
/// 16384 より遠い (20000, 0) に置く（外形は有効な着せ替えに依らないので、着せ替えなしでも越える）。
#[test]
fn extent_too_large_degrades_to_hidden_and_replies_ok() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = bake_world(
        shell_of(vec![
            bind_overlay_surface(20_000, 0),
            surface(5000, vec![elem("p.png", 0, 0)]),
        ]),
        vec![("p.png", 5, 4, gradient(5, 4, 0x22))],
    );
    let direct = Composer::new().compose(
        &emo_world,
        &atlas,
        1000,
        &BindSet::default(),
        &PatternState::default(),
    );
    assert!(
        matches!(
            direct,
            Err(ComposeError::ExtentTooLarge { surface: 1000, .. })
        ),
        "前提: 面 1000 は ExtentTooLarge: {direct:?}"
    );

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 5000);
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));

    let outcome = show_with(
        &mut presenter,
        &mut world,
        TargetId(0),
        1000,
        BindSet::default(),
        PatternState::default(),
    );
    assert!(matches!(outcome, Ok(())), "隠す縮退＋reply Ok: {outcome:?}");
    assert!(!world.get::<Visual>(surface_entity).unwrap().is_visible);
    assert!(!presenter.targets.get(&TargetId(0)).unwrap().visible);
}

/// Hide → 再 ShowSurface 復帰: 有効表示 → `Hide`（不可視＋`HitTest::none()`＋
/// 装着・節・メモ保持）→ 同一有効 id を再 `ShowSurface` で表示復帰（可視＋`HitTest::surface_alpha()`）。
/// 再表示はキャッシュヒットで再合成せず、`read_back` が初回表示バイトと一致する（キャッシュからの復帰）。
#[test]
fn hide_then_reshow_recovers_display_from_cache() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x4D);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    let bytes_shown = presenter
        .read_back(TargetId(0))
        .expect("read_back（初回）失敗");
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::SurfaceAlpha,
        "初回表示後は入れ物の α の判定"
    );

    hide_ok(&mut presenter, &mut world, TargetId(0));
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
        // 装着（surface entity・節の載る先）は Hide で剥がさない。保持されるのは装着そのものである。
        assert!(target.mount.is_some(), "Hide は装着を保持する（R3.3）");
        // 節も剥がさない（再表示は作り直さずに復帰する）。
        assert!(
            on_screen(&presenter, TargetId(0)).is_some_and(|s| !s.items.is_empty()),
            "Hide は画面の節を保持する（R3.3）"
        );
        assert!(
            target
                .cache
                .get(1000, &BindSet::default(), &PatternState::default())
                .is_some(),
            "Hide は合成キャッシュを保持する（R3.3）"
        );
        assert!(!target.visible, "Hide 後は target.visible=false");
    }

    let ((), events) = capture(|| show_ok(&mut presenter, &mut world, TargetId(0), 1000));
    let hits: Vec<_> = events
        .iter()
        .filter(|e| e.message() == INFO_LINE_MESSAGE)
        .map(|e| e.field("cache_hit"))
        .collect();
    assert_eq!(
        hits,
        vec![Some("true")],
        "再表示は表示の行 1 行・控えに当たる（Hide の後に合成し直していない）"
    );

    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "再表示後は可視へ復帰"
    );
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::SurfaceAlpha,
        "再表示後は入れ物の α の判定へ復帰"
    );
    assert!(
        presenter.targets.get(&TargetId(0)).unwrap().visible,
        "再表示後は target.visible=true"
    );

    // 観測可能な復帰: read_back が初回表示バイトと一致（キャッシュからの復帰）。
    let bytes_reshown = presenter
        .read_back(TargetId(0))
        .expect("read_back（再表示）失敗");
    assert_eq!(
        bytes_shown, bytes_reshown,
        "再表示のバイトが初回表示と一致しない（キャッシュからの表示復帰が壊れている）"
    );
}

/// mount 未生成＝取得不可: 未登録 target・登録済みだが初回 `ShowSurface` 前
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
    assert!(
        presenter.text_slot_view(TargetId(0)).is_none(),
        "未登録 target の text_slot_view は None"
    );

    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    // 登録済みでも初回 ShowSurface 前（mount 未生成）は空（mount は遅延生成）。
    assert!(
        presenter.text_slot_view(TargetId(0)).is_none(),
        "初回 ShowSurface 前（mount 未生成）の text_slot_view は None"
    );
}

/// 表示確立後の正値: 有効 `ShowSurface` で表示確立後、`text_slot_view` が
/// `Some` を返し、(a) `slot()` ＝ mount の予約スロット（`Name("emo-text-layer-slot")` を持つ）、
/// (b) `window()` ＝ 装着先窓 Entity、(c) `surface_size()` ＝ バルーン/シェル surface の物理 px 原寸、
/// (d) `scale()` ＝ この入力（窓 DPI 96・author_dpi 96）での 1.0、をすべて満たす。
#[test]
fn text_slot_view_returns_slot_window_size_scale_after_display() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x77);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

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
    assert_eq!(
        view.slot(),
        expected_slot,
        "slot() が予約スロット entity と一致しない"
    );
    let name = world
        .get::<bevy_ecs::name::Name>(view.slot())
        .expect("予約スロットに Name が無い");
    assert_eq!(name.as_str(), "emo-text-layer-slot");

    // (b) window ＝ attach_target で渡した装着先窓。
    assert_eq!(
        view.window(),
        window,
        "window() が装着先窓 entity と一致しない"
    );

    // (c) surface_size ＝ 合成原寸（物理 px・本 fixture は 3×2）。
    assert_eq!(
        view.surface_size(),
        (3, 2),
        "surface_size() が物理原寸と一致しない"
    );

    // (d) scale ＝ 本 fixture の窓 DPI（96）÷ author_dpi（96）＝ 1.0。
    //     恒常値ではなく**この入力での**期待値（k≠1.0 の檻は別テストが所有）。
    assert_eq!(
        view.scale(),
        1.0,
        "窓 DPI 96 / author_dpi 96 ゆえ scale() は 1.0"
    );
}

/// 容量回収は合成成功後に限る:
/// 有効 id で表示を確立したあと**解決不能 id** の適用で合成が失敗しても、合成メモのスロットは
/// 適用前のエントリを**中身ごと**保持し続ける。ゆえに直後の同一入力の適用は再合成せず引き当てで
/// 済み、表示も適用前のまま成立する。
///
/// # なぜ既存の失敗経路の檻では足りないのか（5.1 → 5.3 の申し送り）
///
/// [`invalid_surface_skips_and_leaves_display_and_hit_unchanged`] は表示バイト・`HitTest` の不変を見るが、これらはいずれも「失敗した適用は節を差し替えない」ことの
/// 帰結であり、**スロットが空になったかどうかとは独立**である——空にしても差し替えは起きないので
/// バイトは 1 つも変わらない。したがって `ComposeCache::take_recycled`（追い出しエントリの容量回収）を
/// 合成の成否判定より**手前**へ置く誤りは、既存の檻を丸ごとすり抜ける。本檻はスロットそのものを
/// 直接読み、この規律を固定する唯一の観測点である。
///
/// # 「引き当てで済む」の観測形
///
/// `apply_show` が `cache_hit` を決めるのは `target.cache.touch(surface_id, &binds, &pattern)` で
/// ある（鍵に k は入らない）。`touch` と `get` は同じ `position` で引くので、同じ引数での `get` が
/// `Some` であることは「次の同一入力の適用が引き当てで済む」ことと同値である。
#[test]
fn compose_failure_keeps_the_cache_slot_so_the_next_identical_apply_still_hits() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x6C);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let (bytes_before, plan_before) = {
        let entry = presenter
            .targets
            .get(&TargetId(0))
            .expect("装着済み target")
            .cache
            .get(1000, &BindSet::default(), &PatternState::default())
            .expect("前提: 表示成立でスロットが埋まる");
        (entry.composed.bytes().to_vec(), entry.plan.clone())
    };
    assert!(
        bytes_before.iter().any(|&b| b != 0),
        "前提: スロットの表示バッファは非退化（全 0 でない）"
    );
    let display_before = presenter
        .read_back(TargetId(0))
        .expect("read_back（前）失敗");

    assert!(
        matches!(
            show_with(
                &mut presenter,
                &mut world,
                TargetId(0),
                9999,
                BindSet::default(),
                PatternState::default()
            ),
            Err(PresentError::Compose(ComposeError::SurfaceNotFound(9999)))
        ),
        "前提: 解決不能 id は Err(Compose(SurfaceNotFound(9999)))"
    );

    // ここが本題。スロットは**空になっていない**。
    let entry = presenter
        .targets
        .get(&TargetId(0))
        .expect("装着済み target")
        .cache
        .get(1000, &BindSet::default(), &PatternState::default());
    let entry = entry.expect(
        "合成失敗でキャッシュスロットが空になった＝`take_recycled` が合成の成否判定より手前に\
         置かれている（設計 Flow 2「容量回収は合成成功後に限る」の違反）。以後の同一入力の適用は\
         引き当てに失敗して毎回再合成へ落ちる",
    );
    // 中身も適用前のまま（対の片側だけ差し替わっていない）。
    assert_eq!(
        entry.composed.bytes(),
        bytes_before.as_slice(),
        "合成失敗でスロットの表示バッファ内容が変化した（失敗した適用が中身を書き換えている）"
    );
    assert_eq!(
        entry.plan, plan_before,
        "合成失敗でスロットの計画が差し替わった（原子対の片側だけ動いている）"
    );

    // 直後の同一入力の適用は成立し、表示も適用前と全バイト一致する。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        display_before,
        presenter
            .read_back(TargetId(0))
            .expect("read_back（後）失敗"),
        "合成失敗を挟んだ再適用で表示バイトが変化した"
    );
}

/// 外れの回に計画がエントリへ入って節へ写り、当たりの回は同じ計画のまま節を作り直さない。
///
/// # 殺す誤実装
///
/// - 外れの回に計画を入れない（空の計画を挿入する）→ 1 つ目の主張が RED
/// - 当たりの回にも節を作り直す → 2 つ目の主張が RED（節の実体が替わる）
#[test]
fn a_cache_miss_puts_the_plan_into_the_entry_and_a_hit_keeps_the_nodes() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x2A);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let entry_items = |presenter: &EmoPresenter| {
        presenter
            .targets
            .get(&TargetId(0))
            .unwrap()
            .cache
            .get(1000, &BindSet::default(), &PatternState::default())
            .expect("外れの後は当該キーのエントリが在る")
            .plan
            .items
            .clone()
    };
    let first = on_screen(&presenter, TargetId(0)).expect("表示成立後は節が在る");
    assert!(
        !first.items.is_empty() && first.items == entry_items(&presenter),
        "外れの回の計画がエントリへ入って節へ写っていない: {first:?}"
    );

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        on_screen(&presenter, TargetId(0)),
        Some(first),
        "当たりの回に節が作り直されている"
    );
}

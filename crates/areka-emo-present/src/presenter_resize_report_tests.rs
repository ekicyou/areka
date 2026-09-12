use super::*;

use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_compose::BindSet;

use wintf::ecs::Arrangement;

use super::test_support::{
    build_target_assets, build_two_face_assets, make_world_with_gpu, mount_entities,
    set_window_dpi, show_ok, spawn_window_with_dpi,
};

/// surface entity の `Arrangement`（**論理寸＝原寸・係数＝拡大率 k**）を `(寸, 係数)` で読む。
///
/// 表示の作り方が「原寸の絵＋変換の係数」になったため、報告値の裏取りはここと照会値
/// （`target_physical_size`＝丸め権威）で行う。物理寸そのもの（`GlobalArrangement.bounds`）は
/// wintf の伝播段が導くので、本ファイルのテストは走らせない（丸めの一致は `display_tests.rs`
/// の T-N3 が表で固定する）。
fn arrangement_of(world: &World, surface_entity: Entity) -> ((u32, u32), (f32, f32)) {
    let arr = world
        .get::<Arrangement>(surface_entity)
        .expect("surface entity に Arrangement が無い");
    (
        (arr.size.width as u32, arr.size.height as u32),
        (arr.scale.x, arr.scale.y),
    )
}

// ── 表示成立点の状態照合＝窓寸 reconcile 報告（タスク 3.4・議題 #2 裁定）────────────────────
// design Flow 1 キー決定「表示成立点で今回 scaled 寸を前回適用寸と照合し、差分があれば新物理寸を
// 呼び手（frame drain フェーズ）へ報告する」の檻。報告は `reply` ではなく取り出し可能な状態
// （`take_pending_resize`）に置かれる——本番 drain 経路が `reply: None`（撃ちっぱなし）ゆえ。

/// 要件 3.1/4.1/4.2 観測完了（**寸法変化が呼び手へ報告される**）: 同一 surface を k=1/1 で表示した
/// のち窓 `DPI` を 192 へ変えて再表示すると、表示成立点の状態照合が**新しい物理寸**を積み、
/// `take_pending_resize` がそれを返す（呼び手＝drain フェーズが同一フレームで窓寸 reconcile に使う）。
///
/// 報告値は native 原寸ではなく **k 倍後の物理寸**であり、実際の配置（原寸の寸 × k の係数）と
/// 整合する。照合を行わない
/// 実装・native 寸を報告する実装のいずれでも RED になる。
#[test]
fn dpi_change_reports_new_physical_size_to_caller() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x85);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // k=1/1 の初回表示（初回報告は Flow 3 手順 5 の領分ゆえ、ここでは取り出して捨てる）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some((6, 5)),
        "初回表示は物理寸を報告する（本テストの前提・Flow 3 手順 5）"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "取り出しで要求は消える（drain 契約）"
    );

    // モニタ跨ぎ移動・表示スケール変更の決定論的代替: 窓 DPI を 96→192（k=1/1→2/1）。
    set_window_dpi(&mut world, window, 192);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let k2 = ScaleRatio::new(2, 1).unwrap();
    let expected = k2.scaled_extent(6, 5);
    assert_eq!(expected, (12, 10), "前提: k=2/1 の物理寸");
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(expected),
        "物理寸が変わったのに新物理寸が呼び手へ報告されない（状態照合の欠落）"
    );

    // 報告値＝実際に表示へ載った物理寸であることを裏取りする。表示は「原寸の絵（配置の寸）×
    // 拡大率（配置の係数）」で作られるので、両者と照会値（丸め権威）の 3 つが揃うことを見る。
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        ((6, 5), (2.0, 2.0)),
        "配置が原寸 6×5・係数 2.0 になっていない（報告値の出どころと食い違う）"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(expected),
        "報告値と照会値（丸め権威）が乖離している"
    );
}

/// 要件 4.2 観測完了（**べき等・churn を作らない**）: 物理寸が変わらない再表示は何も報告しない。
///
/// 3 段で檻に入れる——(1) 初回表示の報告を取り出す、(2) 同一入力の再 show（**キャッシュヒット**）は
/// `None`、(3) 別 surface（3000・**同一 native 原寸**＝キャッシュミスで再合成）も `None`。
/// (3) が効くのは「合成したか否か」ではなく**物理寸そのもの**で判定していることの担保である
/// （表示成立ごとに無条件で `Some(size)` を積む実装は (2)(3) 双方で RED）。
#[test]
fn unchanged_physical_size_reports_nothing() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    // 面 1000 と 3000 は同一 native 原寸（6×5）＝合成入力は違うが物理寸は同じ。
    let (emo_world, atlas, _g1000, _g3000) = build_two_face_assets(6, 5);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let k2 = ScaleRatio::new(2, 1).unwrap();
    let physical = k2.scaled_extent(6, 5);
    assert_eq!(physical, (12, 10), "前提: k=2/1 の物理寸");

    // (1) 初回表示は報告あり（取り出して要求を空にする）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(physical),
        "初回表示の報告（本テストの前提）"
    );

    // (2) 同一入力の再 show＝キャッシュヒット・同寸 → 報告なし。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "同寸のヒット再表示が窓寸 reconcile 要求を捏造している（churn の源）"
    );

    // (3) 別 surface＝キャッシュミスで再合成するが物理寸は同じ → 報告なし。
    show_ok(&mut presenter, &mut world, TargetId(0), 3000);
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(3000),
        "前提: 面が切り替わっている（＝ミスして再合成した回）"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "再合成しただけで物理寸が同じなら報告してはならない（判定が寸法でなく合成有無になっている）"
    );
}

/// 要件 3.1 観測完了（**初回表示も必ず報告する**・design Flow 3 手順 5）: 窓は起動時 k₀ 見積もり寸で
/// 生成されており実窓 DPI 由来の k と一致する保証がないため、**前回適用寸が無い初回表示**も差分扱いで
/// 物理寸を報告しなければ、k₀ と実 DPI の差分を補正する経路が永久に走らない。
///
/// 報告値は native 原寸（4×3）ではなく k 倍後の物理寸（8×6）である。初回を黙らせる実装
/// （`prev.is_some() && prev != Some(size)` 条件）は本テストで RED になる。
#[test]
fn first_show_reports_physical_size_for_initial_reconcile() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x86);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 表示前は要求なし（attach しただけで窓を動かさない）。
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "表示未成立の間に窓寸 reconcile 要求があってはならない"
    );

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let k2 = ScaleRatio::new(2, 1).unwrap();
    let physical = k2.scaled_extent(4, 3);
    assert_eq!(physical, (8, 6), "前提: k=2/1 の物理寸");
    assert_ne!(physical, (4, 3), "前提: native 原寸と物理寸が弁別可能");
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(physical),
        "初回表示が物理寸を報告しない（k₀ 見積もり窓寸との差分が補正されない・Flow 3 手順 5）"
    );
}

/// 要件 4.4 観測完了（**失敗は何も報告しない・前値を維持する**）: 表示成立点より手前で early return
/// する失敗経路は、窓寸 reconcile 要求を積まない。
///
/// 2 種の失敗クラスで檻に入れる——(A) 表示未成立での device 失敗（表示記録が要る `GraphicsCore` を
/// 一時退避する。2 個目のデバイスを作らない＝要件 5.3 の AV 非再導入を守る）、(C) 表示成立**後**の合成失敗
/// （`SurfaceNotFound`）。(C) は直前に窓 DPI を 192→96 へ変えてから失敗させるため、報告を
/// 表示成立点より手前（例: `derive_scale` 直後）へ置いた実装なら `Some((4,3))` が積まれて RED になる。
#[test]
fn failed_show_reports_no_resize_and_keeps_previous_values() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x87);
    let k2 = ScaleRatio::new(2, 1).unwrap();

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // (A) 表示記録の前提資源を一時退避 → 合成の後、表示成立の手前で失敗する。
    let gfx = world
        .remove_resource::<GraphicsCore>()
        .expect("前提: make_world_with_gpu が GraphicsCore を載せている");
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
        matches!(
            rx.recv_timeout(Duration::from_secs(10)),
            Ok(Err(PresentError::Device { .. }))
        ),
        "前提: 表示記録に失敗する"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "表示が成立していない失敗が窓寸 reconcile 要求を積んでいる（要件 4.4 違反）"
    );
    {
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert_eq!(t.applied, None, "失敗は前値（未確定）を維持する");
        assert_eq!(t.native_size, None, "失敗は前値（未確定）を維持する");
    }

    // (B) 資源を戻して表示を成立させる（以降の「前値」を作る）。
    world.insert_resource(gfx);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(k2.scaled_extent(4, 3)),
        "前提: 成立した表示は報告する"
    );

    // (C) 窓 DPI を 192→96（k=2/1→1/1・物理寸なら 8×6→4×3 相当）へ変えたうえで**合成に失敗**させる。
    //     表示は成立しないため、k も表示も前値のまま＝報告も無い。
    set_window_dpi(&mut world, window, 96);
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 9999,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(
            rx.recv_timeout(Duration::from_secs(10)),
            Ok(Err(PresentError::Compose(ComposeError::SurfaceNotFound(
                9999
            ))))
        ),
        "前提: 解決不能 id は Err(SurfaceNotFound)"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "表示成立前に early return した失敗が新 k の物理寸を報告している（報告点が表示成立点より手前）"
    );

    // 前 k・前表示・照会契約はすべて据え置き（要件 4.4）。
    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert_eq!(t.applied, Some(k2), "失敗しても前 k を維持する");
    assert_eq!(
        t.native_size,
        Some((4, 3)),
        "失敗しても前 native 原寸を維持する"
    );
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        ((4, 3), (2.0, 2.0)),
        "失敗しても前表示（原寸 4×3・係数 2.0）を維持する（新 k=1/1 の係数へ書き換わっていない）"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(k2.scaled_extent(4, 3)),
        "失敗しても前表示の物理寸を維持する"
    );
}

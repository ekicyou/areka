use super::*;

use super::test_support::{
    attach_and_show_hit_target, attach_hit_target, build_target_assets, build_two_face_assets,
    force_applied, hide_ok, invalidate_ok, make_world_with_gpu, show_ok, spawn_window_with_dpi,
};

// ── CurrentSurfaceRead: 現サーフェス id 状態のライフサイクル固定───────────
// 現サーフェス id は「最後に表示が成立したサーフェス id」（画面の絵でなく表示成立の結果・α非依存）。
// 書き込みは既存 `visible` 更新点と同一の3箇所のみ（表示成立/EmptyComposition 縮退/Hide）＝additive。

/// テスト 10・未表示→None: `attach_target` 直後（一度も `ShowSurface` していない）は
/// `current_surface_id` が `None`。`hit_region_client` も現サーフェス無しゆえ `None`。
///
/// `attach_target` は skeleton 登録のみで World に触れないため、GPU 不要の素の `World` で決定論固定する。
#[test]
fn current_surface_id_is_none_before_first_show() {
    let mut world = World::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x10);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        None,
        "attach_target 直後（未表示）は現サーフェス無し（3.2）"
    );
    assert_eq!(
        presenter.hit_region_client(TargetId(0), 0, 0).region,
        None,
        "未表示 target の hit_region_client は現サーフェス無しゆえ None"
    );
}

/// テスト 12・切替→新 id: 面 1000 表示中に同寸の別 id 3000 を `ShowSurface` すると、
/// `current_surface_id` が `Some(3000)` へ追随する（以後の問い合わせは新 id）。
#[test]
fn current_surface_id_follows_surface_switch() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _g1, _g3) = build_two_face_assets(6, 5);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "初回表示成立後は Some(1000)"
    );

    show_ok(&mut presenter, &mut world, TargetId(0), 3000);
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(3000),
        "別 id へ切替後は新 id を返す（3.3）"
    );
}

/// テスト 13・Hide→None: 有効表示後の `Hide` で `current_surface_id` が `None`
/// （「未表示等」に Hide が含まれる＝`\s[-1]` 相当で表示していない）。
#[test]
fn current_surface_id_is_none_after_hide() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x13);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "Hide 前は Some(1000)（前提）"
    );

    hide_ok(&mut presenter, &mut world, TargetId(0));

    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        None,
        "Hide 後は現サーフェス無し（3.2/4.4）"
    );
    assert_eq!(
        presenter.hit_region_client(TargetId(0), 0, 0).region,
        None,
        "Hide 後は hit_region_client も現サーフェス無しゆえ None"
    );
}

/// 一度も表示していない target の読み戻しは、装着済みなので「未装着」でなく「一度も表示していない」を
/// 返す（`audit-emo-present` 指摘 2）。
#[test]
fn read_back_before_first_show_is_not_shown() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    attach_hit_target(&mut presenter, &mut world, TargetId(0));

    let result = presenter.read_back(TargetId(0));
    assert!(
        matches!(result, Err(PresentError::NotShown(TargetId(0)))),
        "未表示の読み戻しが NotShown でない: {result:?}"
    );
}

/// 表示した後に合成メモを捨てると、表示中の絵がメモから追い出されたことを面の番号つきで返す
/// （`audit-emo-present` 指摘 2）。
#[test]
fn read_back_after_invalidate_cache_is_entry_evicted() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x14);

    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    invalidate_ok(&mut presenter, &mut world, TargetId(0));
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "InvalidateCache は表示を変えないため現サーフェス id は不変（ComposeKey 由来案の棄却根拠）"
    );

    let result = presenter.read_back(TargetId(0));
    assert!(
        matches!(
            result,
            Err(PresentError::EntryEvicted {
                target: TargetId(0),
                surface_id: 1000
            })
        ),
        "メモを捨てた後の読み戻しが EntryEvicted でない: {result:?}"
    );
}

/// テスト 15・未登録 target→None: 一度も `attach_target` していない target に対し
/// `current_surface_id`／`hit_region_client` の両アクセサが `None`（未登録＝現サーフェス無し）。
///
/// 両アクセサとも `HashMap` 引きのみで GPU/World を要さないため、`EmoPresenter::new()` 単体で固定する。
#[test]
fn unregistered_target_returns_none_for_both_accessors() {
    let presenter = EmoPresenter::new();
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        None,
        "未登録 target の current_surface_id は None"
    );
    assert_eq!(
        presenter.hit_region_client(TargetId(0), 10, 20).region,
        None,
        "未登録 target の hit_region_client は None"
    );
}

/// 正常縮退の値契約: attach のみ（未表示）target と未登録 target は
/// `region: None` へ縮退しつつ、`surface_point` は**有効 k で縮約した座標**を返す。
///
/// `applied` 不在の縮退では有効 k が [`ScaleRatio::ONE`] ゆえ `surface_point` は入力素通し
/// （等倍縮約値）である。座標は **x≠y の非対称値**を使うため、軸の取り違え（`(y, x)`）も落ちる。
/// 対照として「未表示だが `applied` は在る」状態も固定する——縮退経路が縮約を**やめて**
/// 生座標を返す誤実装、および丸め権威以外の式を持ち込む誤実装を殺す（k=2 の期待値
/// (181,97)→(90,48) はハードコード定数で、実装式を期待値側で再計算しない）。
#[test]
fn unshown_and_unregistered_targets_degrade_to_none_with_scaled_surface_point() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    attach_hit_target(&mut presenter, &mut world, TargetId(0));

    // attach のみ（未表示・applied なし）: 等倍縮約＝入力素通し。
    let hit = presenter.hit_region_client(TargetId(0), 181, 97);
    assert_eq!(
        hit.region, None,
        "未表示 target に判定対象は無い（region なし）"
    );
    assert_eq!(
        hit.surface_point,
        (181, 97),
        "applied 不在の縮退は等倍縮約（入力素通し）であること"
    );
    // 負値・窓外でも panic せず定義された結果を返す（配線層での保存）。
    let neg = presenter.hit_region_client(TargetId(0), -7, -13);
    assert_eq!(neg.region, None);
    assert_eq!(neg.surface_point, (-7, -13), "負値も等倍縮約で素通し");

    // 未登録 target（attach すらしていない）も同一の正常縮退。
    let unregistered = presenter.hit_region_client(TargetId(9), 181, 97);
    assert_eq!(unregistered.region, None, "未登録 target は region なし");
    assert_eq!(
        unregistered.surface_point,
        (181, 97),
        "未登録 target でも座標空間の契約は保つ（等倍縮約）"
    );

    // 対照: 未表示のまま applied だけ在る状態では、縮退経路も**有効 k で縮約する**。
    // k=2（192/96）の期待値はハードコード: 181→90・97→48（画素中心逆写像）。
    force_applied(
        &mut presenter,
        TargetId(0),
        Some(ScaleRatio::new(192, 96).expect("192/96 は構築可能")),
    );
    let scaled = presenter.hit_region_client(TargetId(0), 181, 97);
    assert_eq!(
        scaled.region, None,
        "未表示なら k が在っても region は None"
    );
    assert_eq!(
        scaled.surface_point,
        (90, 48),
        "未表示縮退でも surface_point は有効 k で縮約されること（生座標を返す実装は RED）"
    );
}

/// 拡大率 1 では `surface_point` が入力の素通しになり、名前は描き先の画素で決まる。
///
/// 矩形は左と上を含み右と下を含まないので、右端・下端の隅は当たらない。期待はハードコードの定数。
#[test]
fn client_entry_passes_point_through_and_resolves_by_pixel_at_identity_scale() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show_hit_target(&mut presenter, &mut world, TargetId(0), 96);
    assert_eq!(presenter.applied_ratio(TargetId(0)), Some(ScaleRatio::ONE));

    for (x, y, want, what) in [
        (180, 96, Some("Head"), "領域内（Head）"),
        (180, 300, Some("Bust"), "別領域内（Bust）"),
        (210, 310, Some("Arm"), "重なり点（後定義 Arm が手前）"),
        (0, 0, None, "背景（原点）"),
        (500, 500, None, "背景（窓外相当）"),
        (93, 62, Some("Head"), "左上隅"),
        (271, 130, None, "右下隅（右と下は含まない）"),
        (93, 130, None, "左下隅（下は含まない）"),
        (271, 62, None, "右上隅（右は含まない）"),
        (94, 63, Some("Head"), "境界の内側 1px"),
        (92, 62, None, "境界の外側 1px（左）"),
        (93, 61, None, "境界の外側 1px（上）"),
        (272, 130, None, "境界の外側 1px（右）"),
        (271, 131, None, "境界の外側 1px（下）"),
        (400, 400, None, "後定義矩形の右下隅（含まない）"),
        (-7, -13, None, "負値（panic なし）"),
    ] {
        let hit = presenter.hit_region_client(TargetId(0), x, y);
        assert_eq!(
            hit.region, want,
            "k=1.0 の解決領域が期待と違う: {what} ({x},{y})"
        );
        assert_eq!(
            hit.surface_point,
            (x, y),
            "k=1.0 の surface_point は入力素通しであること: {what} ({x},{y})"
        );
    }
}

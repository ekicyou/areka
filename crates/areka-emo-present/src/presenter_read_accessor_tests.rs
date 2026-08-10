use super::*;

use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_compose::BindSet;

use super::test_support::{
    attach_hit_target, build_target_assets, build_two_face_assets, force_applied,
    force_current_surface, make_world_with_gpu, spawn_window_with_dpi,
};

// ── CurrentSurfaceRead: 現サーフェス id 状態のライフサイクル固定（Task 2・R3.1-3.4）───────────
// 現サーフェス id は「最後に表示が成立したサーフェス id」（画面の絵でなく表示成立の結果・α非依存）。
// 書き込みは既存 `visible` 更新点と同一の3箇所のみ（表示成立/EmptyComposition 縮退/Hide）＝additive。

/// テスト 10・R3.2 観測完了（未表示→None）: `attach_target` 直後（一度も `ShowSurface` していない）は
/// `current_surface_id` が `None`。`hit_region` も現サーフェス無しゆえ `None`（純関数へ届かない）。
///
/// `attach_target` は skeleton 登録のみで World に触れないため、GPU 不要の素の `World` で決定論固定する。
#[test]
fn current_surface_id_is_none_before_first_show() {
    let mut world = World::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x10);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        None,
        "attach_target 直後（未表示）は現サーフェス無し（3.2）"
    );
    assert_eq!(
        presenter.hit_region(TargetId(0), 0, 0),
        None,
        "未表示 target の hit_region は現サーフェス無しゆえ None"
    );
}

/// テスト 11・R3.1 観測完了（表示後→直近 id）: 有効 `ShowSurface(1000)` 適用後、`current_surface_id`
/// が `Some(1000)`（直近に表示が成立したサーフェス id）。
#[test]
fn current_surface_id_is_last_shown_after_display() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x11);

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

    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "表示成立後は直近に表示した id（3.1）"
    );
}

/// テスト 12・R3.3 観測完了（切替→新 id）: 面 1000 表示中に同寸の別 id 3000 を `ShowSurface` すると、
/// `current_surface_id` が `Some(3000)` へ追随する（以後の問い合わせは新 id）。
#[test]
fn current_surface_id_follows_surface_switch() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _g1, _g3) = build_two_face_assets(6, 5);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let show = |presenter: &mut EmoPresenter, world: &mut World, id: u32| {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: id,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "ShowSurface が Ok でない"
        );
    };

    show(&mut presenter, &mut world, 1000);
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "初回表示成立後は Some(1000)"
    );

    show(&mut presenter, &mut world, 3000);
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(3000),
        "別 id へ切替後は新 id を返す（3.3）"
    );
}

/// テスト 13・R3.2/4.4 観測完了（Hide→None）: 有効表示後の `Hide` で `current_surface_id` が `None`
/// （「未表示等」に Hide が含まれる＝`\s[-1]` 相当で表示していない）。
#[test]
fn current_surface_id_is_none_after_hide() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x13);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

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
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "Hide 前は Some(1000)（前提）"
    );

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

    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        None,
        "Hide 後は現サーフェス無し（3.2/4.4）"
    );
    assert_eq!(
        presenter.hit_region(TargetId(0), 0, 0),
        None,
        "Hide 後は hit_region も現サーフェス無しゆえ None"
    );
}

/// テスト 14 観測完了（InvalidateCache→不変）: 有効表示後に `InvalidateCache` を適用しても
/// `current_surface_id` は不変（キャッシュ無効化は表示を変えない）。単一真実源が `ComposeKey` 由来では
/// なくフィールドであることの回帰檻（`invalidate_all` でキーが消えても現サーフェス id は残る）。
#[test]
fn current_surface_id_unchanged_by_invalidate_cache() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x14);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

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

    let (txi, rxi) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::InvalidateCache {
            target: TargetId(0),
            reply: Some(txi),
        },
    );
    assert!(
        matches!(rxi.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "InvalidateCache が Ok でない"
    );

    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "InvalidateCache は表示を変えないため現サーフェス id は不変（ComposeKey 由来案の棄却根拠）"
    );
}

/// テスト 15・R3.2 観測完了（未登録 target→None）: 一度も `attach_target` していない target に対し
/// `current_surface_id`／`hit_region` の両アクセサが `None`（未登録＝現サーフェス無し）。
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
        presenter.hit_region(TargetId(0), 10, 20),
        None,
        "未登録 target の hit_region は None"
    );
}

/// 要件 1.6/1.1 観測完了（正常縮退の値契約）: attach のみ（未表示）target と未登録 target は
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
    // 負値・窓外でも panic せず定義された結果を返す（要件 2.5 の配線層での保存）。
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
    // k=2（192/96）の期待値はハードコード: 181→90・97→48（DD-1 の画素中心逆写像）。
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

/// 要件 1.5 観測完了（**公開面同士の恒等**・タスク 3.1 完了条件の検証本体）: k=1.0 では
/// [`EmoPresenter::hit_region_client`] の `region` が既存 [`EmoPresenter::hit_region`] と
/// 完全一致し、`surface_point` は入力素通しになる。
///
/// 代表点は「領域内」「別領域内」「重なり点（画家則＝後定義 Arm が手前）」「背景」
/// 「閉区間の端（4 隅・辺）」「境界の内側 1px／外側 1px」「負値」「窓外」を含む。期待 region は
/// **ハードコード定数**でも同時に固定するため、両入口がそろって壊れる（＝両方 None を返す）
/// 形の空虚な一致では緑にならない。
///
/// なお本檻は k=1.0 の恒等のみを守る——`×k` の誤挿入や素の floor 丸めの持ち込みは k=1.0 では
/// 恒等へ退化して検出できない（それらは `hit.rs` の任意 k 檻と本ファイルの縮退檻の責務）。
#[test]
fn client_entry_matches_native_entry_at_identity_scale() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    attach_hit_target(&mut presenter, &mut world, TargetId(0));
    force_current_surface(&mut presenter, TargetId(0), 1000);
    force_applied(&mut presenter, TargetId(0), Some(ScaleRatio::ONE));

    for (x, y, want, what) in [
        (180, 96, Some("Head"), "領域内（Head）"),
        (180, 300, Some("Bust"), "別領域内（Bust）"),
        (210, 310, Some("Arm"), "重なり点（後定義 Arm が手前）"),
        (0, 0, None, "背景（原点）"),
        (500, 500, None, "背景（窓外相当）"),
        (93, 62, Some("Head"), "閉区間の左上隅"),
        (271, 130, Some("Head"), "閉区間の右下隅"),
        (93, 130, Some("Head"), "閉区間の左下隅"),
        (271, 62, Some("Head"), "閉区間の右上隅"),
        (94, 63, Some("Head"), "境界の内側 1px"),
        (92, 62, None, "境界の外側 1px（左）"),
        (93, 61, None, "境界の外側 1px（上）"),
        (272, 130, None, "境界の外側 1px（右）"),
        (271, 131, None, "境界の外側 1px（下）"),
        (400, 400, Some("Arm"), "後定義矩形の右下隅"),
        (-7, -13, None, "負値（panic なし）"),
    ] {
        let hit = presenter.hit_region_client(TargetId(0), x, y);
        assert_eq!(
            hit.region,
            presenter.hit_region(TargetId(0), x, y),
            "k=1.0 では新旧の判定入口が一致すること: {what} ({x},{y})"
        );
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

/// 要件 1.1/1.4 観測完了（**正常経路で実適用 k が合成純関数へ渡ること**）: 「表示中サーフェス
/// あり × `applied = Some(k)`・k≠1.0」で [`EmoPresenter::hit_region_client`] を呼ぶと、判定は
/// **÷k した縮約後サーフェス px** で解決され、`surface_point` も同じ縮約値になる。
///
/// # なぜ既存 3 檻と別に要るのか（継ぎ目の封鎖）
///
/// 本ファイルの既存 3 檻はいずれも `hit_region_scaled(master, x, y, k, …)` の `k` を固定しない——
/// `applied_absent_with_visible_surface_warns_once_and_degradations_stay_silent` は warn 述語が
/// 主語で k は**不在**、`unshown_and_unregistered_targets_degrade_to_none_with_scaled_surface_point`
/// は k=2 を与えるが `master` **不在**側（`hit_region_client` 内で `unscale_coord` を直接呼ぶ分岐）
/// を通り、`client_entry_matches_native_entry_at_identity_scale` は k=1.0 ゆえ縮約が恒等へ退化する。
/// ゆえに「面あり × k≠1.0」の合流点は一度も実行されておらず、実適用 k が合成純関数へ本当に
/// 渡っているかは決定論テストで固定されていなかった。本檻の主語はその 1 点のみである。
///
/// # 殺す誤実装（反証可能性）
///
/// `hit_region_scaled` へ渡す `k` を [`ScaleRatio::ONE`] へすり替える（＝実適用 k を無視して
/// client 点をそのまま照合する）と、下の 3 点は `surface_point`・`region` の**双方**で割れる。
/// とくに (210,310) は恒等なら `Some("Arm")`・k=2 なら `None` と**逆向きに**割れるため、
/// 「両方そろって `None`」の空虚な一致では緑にならない。
///
/// 期待値は**ハードコード定数**であり、`unscale_coord` を期待値側で呼び直さない（実装式の
/// 再実行はトートロジー）。k=2 の縮約は画素中心逆写像 `floor((2v+1)/4)`:
/// 360→180・192→96・420→210・620→310・210→105・310→155。
#[test]
fn visible_surface_hit_uses_applied_scale_at_k2() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    attach_hit_target(&mut presenter, &mut world, TargetId(0));
    force_current_surface(&mut presenter, TargetId(0), 1000);
    let k2 = ScaleRatio::new(2, 1).expect("2/1 は構築可能");
    force_applied(&mut presenter, TargetId(0), Some(k2));

    // 前提の明示（檻が空虚でないこと＝「面あり × k≠1.0」が本当に組めていること）。
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        Some(1000),
        "前提: 表示中サーフェスあり（＝正常経路の合成純関数へ入る）"
    );
    assert_eq!(
        presenter.applied_ratio(TargetId(0)),
        Some(k2),
        "前提: 実適用 k は 2/1（恒等ではない）"
    );

    for (cx, cy, want_region, want_point, identity_would_be, what) in [
        (
            360,
            192,
            Some("Head"),
            (180, 96),
            "None",
            "縮約後は Head（client 点のままなら領域外）",
        ),
        (
            420,
            620,
            Some("Arm"),
            (210, 310),
            "None",
            "縮約後は Bust/Arm の重なり点で画家則の Arm（後定義が手前）",
        ),
        (
            210,
            310,
            None,
            (105, 155),
            "Some(\"Arm\")",
            "縮約後は背景（client 点のままなら Arm＝逆向きに割れる点）",
        ),
    ] {
        let hit = presenter.hit_region_client(TargetId(0), cx, cy);
        assert_eq!(
            hit.surface_point, want_point,
            "実適用 k=2 で縮約した座標が期待と違う（恒等 k なら ({cx},{cy}) のまま）: \
             {what} client=({cx},{cy})"
        );
        assert_eq!(
            hit.region, want_region,
            "縮約後サーフェス px で解決した領域が期待と違う（恒等 k で照合していれば \
             {identity_would_be} になる＝実適用 k が合成純関数へ渡っていない）: \
             {what} client=({cx},{cy})"
        );
    }
}

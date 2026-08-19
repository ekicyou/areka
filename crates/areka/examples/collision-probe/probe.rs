use super::fixture::{ANCHOR_MARGIN_PX, BUST_RECT, HEAD_RECT, PLACEHOLDER_SIZE, REAL_BIND_IDS};
use super::ratio::{assert_expected_ratio, format_ratio};
use super::state::{ProbeBoot, ProbePhase};
use super::{
    BindSet, EmoPresenter, GetClientRect, GraphicsCore, PatternState, PresentCommand, RECT,
    ScaleRatio, TargetId, WindowHandle, World, WucGraphicsResource, placement, target_map,
};

// ---------------------------------------------------------------------------
// Probe boot system（GPU 到達→表示→期待 k ゲート→本番 resize→物理寸整合 assert）
// ---------------------------------------------------------------------------

/// GPU 資源到達フレームで surface1000 を表示し、期待 k ゲート・④ 描画一致 anchor・③ 物理寸整合 assert を
/// 駆動する probe 起動 system（`&mut World` 排他・donor `boot_present_system` と同作法: remove→駆動→insert）。
///
/// フェーズ機械（`ProbePhase`）で「表示＋本番 resize」と「（次フレーム以降の）`GetClientRect` 検証」を
/// 分離する——`SetWindowPosCommand` は発行 tick の World 借用解放後に flush されるため（`tick_bridge.rs:199-200`）、
/// 同 tick 内の `GetClientRect` は旧寸を返す。ゆえに resize と検証は別フレームに分ける。
pub(super) fn boot_probe_system(world: &mut World) {
    let phase = match world.get_non_send::<ProbeBoot>() {
        Some(b) => b.phase,
        None => return,
    };
    match phase {
        ProbePhase::WaitingAttach => attach_show_and_resize(world),
        ProbePhase::WaitingVerify => verify_physical_size_match(world),
        ProbePhase::Done => {}
    }
}

/// WaitingAttach: GPU 資源到達を待ち、attach→`apply(ShowSurface{1000, 実 bind})`→常設 k ログ→期待 k
/// ゲート→④ read_back 描画一致 anchor→**物理寸**への本番 `resize_window_to`（戻り値 true を assert）を
/// 駆動して WaitingVerify へ進める。
fn attach_show_and_resize(world: &mut World) {
    // GPU 資源の準備待ち（未準備なら ProbeBoot を保持したまま次 tick へ）。
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    let mut boot = world
        .remove_non_send::<ProbeBoot>()
        .expect("直上で存在確認済み");

    let target = target_map::shell_target(0);
    let Some((emo_world, atlas)) = boot.assets.take() else {
        // 装着アセットが無い（想定外）。前値保持で Done へ倒す。
        tracing::error!("collision-probe: 装着アセットが空（想定外）— 中止");
        boot.phase = ProbePhase::Done;
        world.insert_non_send(boot);
        return;
    };

    if let Err(e) = boot.presenter.attach_target(
        world,
        target,
        boot.char_window,
        emo_world,
        atlas,
        // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を供給する。
        // **実適用 k は実行機の表示スケールが決める**（窓生成時に wintf が実モニタ DPI を `DPI`
        // component へ初期化するため初回表示で確定する）——k=96/96=1.0 になるのは実行機が 100%
        // 表示のときだけであり、125%/200% で実行すれば k は 5/4・2/1 になる（DPI 追従駆動は不要）。
        96,
    ) {
        tracing::error!(error = %e, "collision-probe: scope0 char target の attach に失敗 — 中止");
        boot.phase = ProbePhase::Done;
        world.insert_non_send(boot);
        return;
    }

    // donor 必須逸脱 #1/#2: surface1000 を有効 bind 実値付きで表示する。
    boot.presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id: 1000,
            binds: BindSet::from_ids(REAL_BIND_IDS),
            pattern: PatternState::default(),
            reply: None,
        },
    );

    // 現表示の native 原寸を読む。表示成立後は Some のはず（None は表示失敗＝中止）。
    let Some(view) = boot.presenter.text_slot_view(target) else {
        tracing::error!(
            "collision-probe: 表示成立後に text_slot_view が None（surface1000 の表示に失敗）— 中止"
        );
        boot.phase = ProbePhase::Done;
        world.insert_non_send(boot);
        return;
    };
    let native = view.surface_size();
    let scale_f32 = view.scale();

    // **窓 resize 先の権威＝`target_physical_size`**（k 適用後の物理寸・丸め単一権威 `scaled_extent`
    // 経由）。native 原寸（`surface_size`）へ resize すると k≠1.0 で窓が原寸へ引き戻される（本 probe が
    // 捕捉すべき欠陥クラスそのもの）ため、寸の出所を照会 1 本へ寄せて取り違えの余地を消す。
    let physical = boot
        .presenter
        .target_physical_size(target)
        .expect("text_slot_view が Some ＝ applied/native_size 確定済みゆえ物理寸も確定している");
    // 実適用 k の厳密値（f32 照会 `scale()` は寸法・画素演算に使わない出口ビュー）。
    let applied = boot
        .presenter
        .applied_ratio(target)
        .expect("text_slot_view が Some ＝ applied 確定済み");

    boot.native_size = native;
    boot.physical_size = physical;

    // 常設 greppable ログ（要件 4.1/4.5・design CollisionProbe 節）。実機サインオフはこの 1 行を grep して
    // 「その水準で適用された k と実表示物理寸」を採取し、2 実行の physical が互いに異なることを照合する。
    let k_text = format_ratio(applied);
    tracing::info!(
        k = %k_text,
        native_w = native.0,
        native_h = native.1,
        physical_w = physical.0,
        physical_h = physical.1,
        scale_f32,
        "collision-probe: k={k_text} native={}x{} physical={}x{}",
        native.0,
        native.1,
        physical.0,
        physical.1
    );

    // 期待 k ゲート（env 指定時のみ hard assert・未指定なら上の実測ログのみで通過）。
    assert_expected_ratio(applied);

    // ④ 描画一致の anchor（マウス非依存・read_back）: Head/Bust 中心を物理座標へ写像した画素が不透明で
    // あることを hard assert する。**描画証跡であって判定証跡ではない**（要件 4.4）。
    assert_drawn_anchor(&boot.presenter, target, applied, physical);

    // donor 必須逸脱 #3: 本番の反映関数で placeholder 誤寸 → **物理寸**へ resize（戻り値 true を assert）。
    let ok = placement::follow::resize_window_to(
        world,
        boot.char_window,
        placement::resolver::SizePx {
            w: physical.0 as i32,
            h: physical.1 as i32,
        },
        // 表示実寸への再スナップ＝毎フレーム再スナップと同一の経路（Req 1.2・task 1.4）。
        placement::diag::PlacementRoute::Resnap,
    );
    assert!(
        ok,
        "collision-probe: resize_window_to が false（placeholder {PLACEHOLDER_SIZE}×{PLACEHOLDER_SIZE} → 物理寸 {}×{} への本番 resize 経路が不発）",
        physical.0, physical.1
    );
    tracing::info!(
        "collision-probe: 本番 resize 経路（resize_window_to）適用 — 次フレーム以降に GetClientRect で物理寸整合を検証"
    );

    boot.phase = ProbePhase::WaitingVerify;
    world.insert_non_send(boot);
}

/// WaitingVerify（本番 resize の次フレーム以降）: 実窓 `GetClientRect` が
/// [`EmoPresenter::target_physical_size`]（k 適用後の物理寸権威）と一致することを hard assert する
/// （③ 物理寸整合 assert）。**`WindowPos.size` ではなく実窓に対して行う**（`WindowPos` は enqueue 時
/// bypass ミラー済みで偽緑になるため）。
///
/// 期待側は **native 原寸ではなく物理寸**である——k≠1.0 で `surface_size()` と比較すると、窓が原寸へ
/// 引き戻される欠陥をむしろ「正常」と誤判定する（旧実装の陳腐化点）。
fn verify_physical_size_match(world: &mut World) {
    let mut boot = world
        .remove_non_send::<ProbeBoot>()
        .expect("直上で存在確認済み");

    let target = target_map::shell_target(0);

    // 物理寸権威・native 原寸・実適用 k を再読（値の源＝emo 合成パイプライン）。
    let Some(physical) = boot.presenter.target_physical_size(target) else {
        tracing::error!(
            "collision-probe: WaitingVerify で target_physical_size が None（想定外）— 中止"
        );
        boot.phase = ProbePhase::Done;
        world.insert_non_send(boot);
        return;
    };
    let native = boot
        .presenter
        .text_slot_view(target)
        .map(|v| v.surface_size())
        .unwrap_or(boot.native_size);
    let applied = boot
        .presenter
        .applied_ratio(target)
        .unwrap_or(ScaleRatio::ONE);

    // 実窓 client 矩形（GetClientRect・areka＋OS 窓パイプラインの出力）。WindowPos ミラーは読まない。
    let Some(handle) = world.get::<WindowHandle>(boot.char_window).copied() else {
        tracing::error!("collision-probe: char 窓に WindowHandle 未付与（GetClientRect 不能）— 中止");
        boot.phase = ProbePhase::Done;
        world.insert_non_send(boot);
        return;
    };
    let mut rect = RECT::default();
    // SAFETY: Win32 境界。GetClientRect は hwnd と RECT への可変ポインタを要し、client 矩形を書き込むだけ。
    let got = unsafe { GetClientRect(handle.hwnd, &mut rect) };
    assert!(
        got.is_ok(),
        "collision-probe: GetClientRect が失敗（hwnd={:?}）",
        handle.hwnd
    );
    let client_w = (rect.right - rect.left) as u32;
    let client_h = (rect.bottom - rect.top) as u32;

    // ③ 物理寸整合 assert: 経路独立な単位保存性検査（emo 合成パイプライン ↔ areka＋OS 窓パイプライン）。
    assert_eq!(
        (client_w, client_h),
        physical,
        "collision-probe: GetClientRect client 寸 {:?} が target_physical_size {:?} と不一致（k={}・native={}x{}・resize 要求寸 {:?}）— 本番 resize 経路の単位保存性違反（native 原寸への引き戻し・dpi/96 再スケール・論理 px 解釈の混入の疑い）",
        (client_w, client_h),
        physical,
        format_ratio(applied),
        native.0,
        native.1,
        boot.physical_size
    );
    tracing::info!(
        client_w,
        client_h,
        physical_w = physical.0,
        physical_h = physical.1,
        native_w = native.0,
        native_h = native.1,
        k = %format_ratio(applied),
        "collision-probe: ③ 物理寸整合 assert 通過（GetClientRect == target_physical_size）— 手動プロトコル ⑤ へ"
    );

    boot.phase = ProbePhase::Done;
    world.insert_non_send(boot);
    tracing::info!(
        "collision-probe: 自動 assert（③④）完了。マウスで頭/胸/背景を目視で狙い解決結果とペア列を記録してください（⑤）"
    );
}

/// ④ 描画一致の anchor: `read_back` した表示画素の Head/Bust 各矩形中心（**物理座標へ写像**）が
/// **不透明**であることを hard assert。
///
/// collision 値（サーフェス px）の位置に実際に絵が描かれていることを機械的に固定する（マウス非依存）。
/// 合成ビットマップ原点 ≡ サーフェス画像原点は構造的に保証される（`compute_extent` は原点 (0,0) 固定）が、
/// `read_back` が返すのは **k 適用後の供給面**（`chain.size()` ＝ 物理寸）であるため、collision 座標は
/// 乗算方向の丸め権威 [`ScaleRatio::scale_len`] で ×k してから画素 index へ写す。
///
/// # 証跡としての位置づけ（要件 4.4）
///
/// 本検査は「その位置に絵が描かれている」ことしか語らない**描画証跡**であり、**当たり判定の証跡ではない**。
/// 判定の証跡は ⑤ の目視由来経路（`GetCursorPos`→`ScreenToClient`→`resolve_hit_region`）だけであり、
/// 記録様式でも両者を別欄に分けて混ぜない（描画由来の点を判定へ注入すると自己整合の罠になる）。
fn assert_drawn_anchor(
    presenter: &EmoPresenter,
    target: TargetId,
    applied: ScaleRatio,
    physical: (u32, u32),
) {
    let bytes = match presenter.read_back(target) {
        Ok(b) => b,
        Err(e) => panic!("collision-probe: read_back に失敗（④ 描画一致 anchor が取れない）: {e}"),
    };
    for (rect, label) in [(HEAD_RECT, "Head"), (BUST_RECT, "Bust")] {
        let anchor = physical_anchor(rect, applied, label);
        assert_pixel_opaque(&bytes, physical, anchor, label);
    }
    tracing::info!(
        k = %format_ratio(applied),
        "collision-probe: ④ 描画一致 anchor 通過（物理座標へ写像した Head/Bust 中心が不透明）— これは描画証跡であり判定証跡ではない（判定証跡は ⑤ の目視由来経路のみ）"
    );
}

/// 当たり判定矩形（サーフェス px）の中心を **物理座標**へ写像した anchor 画素を返す。
///
/// 中心・矩形境界のいずれも乗算方向の丸め権威 [`ScaleRatio::scale_len`] を通す（probe 側で ×k の式を
/// 持たない）。写像後の anchor が矩形の内側に [`ANCHOR_MARGIN_PX`] 以上の余裕を持つことを hard assert し、
/// `scale_len` の丸め差（≤1px）と無関係に anchor が成立することを構図で保証する。
fn physical_anchor(
    (left, top, right, bottom): (i64, i64, i64, i64),
    applied: ScaleRatio,
    label: &str,
) -> (u32, u32) {
    let center = (((left + right) / 2) as u32, ((top + bottom) / 2) as u32);
    let anchor = (
        applied.scale_len(center.0),
        applied.scale_len(center.1),
    );
    let (pl, pt) = (applied.scale_len(left as u32), applied.scale_len(top as u32));
    let (pr, pb) = (
        applied.scale_len(right as u32),
        applied.scale_len(bottom as u32),
    );
    assert!(
        anchor.0 >= pl + ANCHOR_MARGIN_PX
            && anchor.0 + ANCHOR_MARGIN_PX <= pr
            && anchor.1 >= pt + ANCHOR_MARGIN_PX
            && anchor.1 + ANCHOR_MARGIN_PX <= pb,
        "collision-probe: {label} anchor ({},{}) が物理矩形 ({pl},{pt})-({pr},{pb}) の内側 {ANCHOR_MARGIN_PX}px 余裕を満たさない（k={}・丸め差と無関係に成立させる前提の破綻）",
        anchor.0,
        anchor.1,
        format_ratio(applied)
    );
    anchor
}

/// `read_back`（`stride = width*4`・BGRA・上端起点の密配列）の画素 `(px, py)` が不透明（α=0xFF）であることを
/// hard assert する。範囲外・バイト長不足も loud に落とす（④ の観測失敗を silent にしない）。
fn assert_pixel_opaque(bytes: &[u8], (w, h): (u32, u32), (px, py): (u32, u32), label: &str) {
    assert!(
        px < w && py < h,
        "collision-probe: {label} 中心 ({px},{py}) が合成外形 {w}×{h} の外（構図破綻＝collision 値と表示寸の乖離）"
    );
    // BGRA 密配列: 画素 (px,py) の α は offset (py*w+px)*4 + 3。
    let idx = ((py * w + px) * 4 + 3) as usize;
    assert!(
        idx < bytes.len(),
        "collision-probe: {label} 中心 ({px},{py}) の α index={idx} が read_back バイト長 {} の外",
        bytes.len()
    );
    let alpha = bytes[idx];
    assert_eq!(
        alpha, 0xFF,
        "collision-probe: {label} 矩形中心 ({px},{py}) の画素が不透明でない（α={alpha}）— collision 値の位置に絵が描かれていない"
    );
}

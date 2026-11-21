use super::{
    AUTHOR_DPI, CYCLE_INTERVAL_SECS, ComposeError, ComposedSurface, DPI, EmoBoot, EmoPresenter,
    Entity, FrameTime, Point, SHELL_INITIAL_X, SHELL_INITIAL_Y, ScalePolicy, ScaleRatio, SizeI,
    TargetId, WindowPos, World, compute_balloon_pos, derive_scale,
};

// ---------------------------------------------------------------------------
// 窓寸 reconcile
// ---------------------------------------------------------------------------

/// 窓寸 reconcile 時の**位置の決め方**の別（本番 `runtime::frame::GhostWindowKind` の同型）。
///
/// 本番は char 窓＝`resize_window_to`（`Anchored` 射影で接地点＝下端中央を保つ）／balloon 窓＝
/// `resize_window_keep_position`（位置は `follow_balloon` が決める従属量ゆえ据え置き）に振り分ける。
/// 本 example の窓は placement 層を通さず生成されるため `Anchored` を持たず、`resize_window_to` は
/// 不発である——ゆえに位置の扱いだけ本 example の構図へ合わせる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReconcileKind {
    /// シェル窓: 現在位置（[`SHELL_INITIAL_X`]/[`SHELL_INITIAL_Y`] の左上）を据え置く。
    ///
    /// 「キャラ窓の原点は下端中央」規約は本 example では再現しない: あの規約は placement 層の
    /// `Anchored`＋`project_anchor` が担う不変条件で、本 example の窓はアンカーを持たず「保つべき
    /// 接地点」が存在しない。手作りの射影を書けばアンカー規約の第 2 の流儀を生やすことになる。
    /// 窓配置の観測は `areka-placement` の `examples/window-placement.rs`／`areka-runtime` の
    /// `examples/collision-probe.rs` の領分である。
    Shell,
    /// バルーン窓: 位置はシェル＋自幅から決まる従属量ゆえ [`compute_balloon_pos`] で再算出する。
    ///
    /// 本番の `resize_window_keep_position`（位置据え置き）に対応するが、本 example には
    /// `follow_balloon` に当たる追従 system が無い。k≠1.0 でバルーン幅が伸びると据え置きでは既定整列
    /// （バルーン右端＝シェル左端）が崩れてシェルへ食い込むため、同じ関数へ新幅を与えて再算出する。
    Balloon,
}

/// 表示成立点の窓寸 reconcile 要求を消費し、窓 client を k 適用後の物理寸へ合わせる。
///
/// 各 target について [`EmoPresenter::take_pending_resize`] を引き、`Some(新物理寸)` のときだけ
/// [`reconcile_window_size`] で反映する。本番 `runtime` の `reconcile_reported_sizes` と同じ流儀:
///
/// - **消費者は表示を引き起こした者**。本 example は presenter を直接所有し `apply` を
///   自ら呼ぶ唯一の主体で、`refresh_scale`（もう一方の消費者）は呼ばない——ゆえに要求を取り
///   逃す第三者が存在しない。
/// - **報告は「状態」であって「エッジ」ではない**。要求は取り出されるまで消えず、取り出した後は
///   同寸表示を何度繰り返しても `None`（＝窓への無用な書込を生まない）。
/// - **k=1.0 でも初回表示は必ず報告される**（presenter 側の契約）。ただし窓は当該 surface の native
///   原寸で生成済みゆえ、反映は同寸のべき等 skip となり書込は起きない。
///
/// # 報告は「DPI 変化」ではなく「表示物理寸の変化」に紐づく
///
/// presenter が要求を積む条件は *k* の変化ではなく**物理寸（＝native 原寸 × k）の変化**である。ゆえに
/// k=1.0 でも、巡回で表示 surface が変わって native 原寸が変われば窓はそれに追従する（本番の
/// `reconcile_reported_sizes`／resnap と同一の意味論）。同寸の切替（emo2 のまばたきは合成外形が
/// 変わらない）では要求が積まれず、窓は一切書かれない。
///
/// 窓 entity が [`Entity::PLACEHOLDER`]（構築失敗で窓を作らなかった）のに要求が出るのは、装着済み
/// target と窓生成の食い違い＝結線バグゆえ `error!` で loud に観測する（silent skip にしない）。
pub(super) fn reconcile_present_sizes(boot: &mut EmoBoot, world: &mut World) {
    for (target, window, kind) in [
        (TargetId(0), boot.shell_window, ReconcileKind::Shell),
        (TargetId(1), boot.balloon_window, ReconcileKind::Balloon),
    ] {
        let Some(new_size) = boot.presenter.take_pending_resize(target) else {
            continue;
        };
        if window == Entity::PLACEHOLDER {
            tracing::error!(
                ?target,
                ?kind,
                ?new_size,
                "emo-present: 窓寸 reconcile 要求が出たのに窓 entity が未生成（PLACEHOLDER）— 反映先が無い（結線不整合）"
            );
            continue;
        }
        reconcile_window_size(world, window, kind, new_size);
    }
}

/// 報告された新物理寸を窓 client（`WindowPos`）へ反映する（本番 `reconcile_window_size` の同型）。
///
/// 戻り値は**書込が起きたか**であり、`false` は失敗とは限らない——同寸・同位置のべき等 skip も
/// `false` を返す（本番 `resize_window_to`／`resize_window_keep_position` の慣行と一致）。ゆえに
/// 呼び手は `false` を error として鳴らさない。ログ層は縮退の質で分ける: べき等 skip は `debug!`・
/// 反映先を欠く異常（`WindowPos` 未付与）は `warn!`・値が窓寸として成立しない場合は `warn!`/`error!`。
///
/// 物理寸は `u32`（表示バッファ外形）で報告されるが窓寸は `i32` 通貨ゆえ、ここで変換し超過・0 を
/// 弾く（log-first・panic しない）。**f32 を寸法演算に用いない**——値は presenter の丸め単一権威
/// （`ScaleRatio::scaled_extent` 経由の [`EmoPresenter::target_physical_size`] と同一の計算）から来て
/// おり、ここでは整数のまま運ぶだけである。
///
/// 反映は `WindowPos` への通常書込（変更検知あり）で行い、wintf の `apply_window_pos_changes` が
/// 次フレームに `SetWindowPos` を発行する。本番 placement 層（`enqueue_window_set_pos`）は
/// example から迂回して呼ばない。
fn reconcile_window_size(
    world: &mut World,
    window: Entity,
    kind: ReconcileKind,
    new_size: (u32, u32),
) -> bool {
    let (Ok(w), Ok(h)) = (i32::try_from(new_size.0), i32::try_from(new_size.1)) else {
        tracing::error!(
            ?window,
            ?kind,
            ?new_size,
            "emo-present: 報告された物理寸が i32 域を超える → 窓寸を変えない（前寸維持・log-first）"
        );
        return false;
    };
    if w == 0 || h == 0 {
        tracing::warn!(
            ?window,
            ?kind,
            ?new_size,
            "emo-present: 報告された物理寸に 0 軸がある → 窓寸を変えない（前寸維持）"
        );
        return false;
    }
    let size = SizeI {
        width: w,
        height: h,
    };

    let recomputed_pos = match kind {
        ReconcileKind::Shell => None,
        ReconcileKind::Balloon => {
            let (x, y) = compute_balloon_pos(SHELL_INITIAL_X, SHELL_INITIAL_Y, new_size.0);
            Some(Point { x, y })
        }
    };

    let Some(mut wp) = world.get_mut::<WindowPos>(window) else {
        tracing::warn!(
            ?window,
            ?kind,
            ?new_size,
            "emo-present: WindowPos 未付与（窓生成前）— 窓寸を反映できない"
        );
        return false;
    };
    let position = recomputed_pos.or(wp.position);

    // べき等 skip（振動・churn 防止）: 同寸・同位置なら書込まない。
    if wp.size == Some(size) && wp.position == position {
        tracing::debug!(
            ?window,
            ?kind,
            ?new_size,
            "emo-present: 窓 client が既に報告寸と同一のため書込をスキップ（べき等）"
        );
        return false;
    }

    wp.size = Some(size);
    wp.position = position;
    tracing::info!(
        ?window,
        ?kind,
        w,
        h,
        x = position.map(|p| p.x),
        y = position.map(|p| p.y),
        "emo-present: 窓 client を k 適用後の物理寸へ reconcile"
    );
    true
}

/// 起動時 golden バイト一致 assert。
///
/// 初回表示直後に target の表示画素を `EmoPresenter::read_back`（表示に渡した**原寸の合成バイト列**）
/// で取得し、`Composer::compose` が返す golden [`ComposedSurface`] のバイト列
/// （[`ComposedSurface::bytes`]）と **完全一致**（full byte equality）することを検証する。
/// # golden に k を掛けない
///
/// 拡大は GPU の Visual 変換が担い、CPU 側の再標本化は表示経路に無い。`read_back` が返すのは
/// 表示中エントリの **native 原寸バイト列**で、k の値に依らず同一バイト・同一長
/// （`native_w * native_h * 4`）である。ゆえに golden も native 原寸のまま突き合わせる。
///
/// # k の導出と、その一致検査
///
/// k は presenter と同一の純関数 [`derive_scale`] へ同一入力（target 政策＝[`AUTHOR_DPI`]／窓の `DPI`
/// component）を与えて求める。ただしこれは**推定**であり、実際に表示へ掛かった k の単一真実源は presenter
/// 側の `applied` である。ゆえに推定 k をそのまま信用せず、
///
/// - [`EmoPresenter::target_physical_size`]（＝`scaled_extent(applied, native 原寸)`・丸め単一権威）と
///   推定 k を同じ権威に通した golden の物理寸が一致すること、
/// - [`EmoPresenter::applied_scale`]（照会契約の実適用 k）が推定 k と一致すること
///
/// を **assert で検査**する（食い違えば loud に落ちる＝黙って別の k で比較しない）。
///
/// # 失敗を silent にしない
///
/// 寸法・バイト長・内容のいずれかが食い違えば即 `panic!`／`assert_eq!` で loud に落とす。
///
/// # 正当な非表示のスキップ
///
/// golden 合成に失敗した場合、または表示に渡した合成面が無い（`read_back` が
/// [`areka_emo_present::PresentError`] を返す・未表示や EmptyComposition degradation 等）の場合は、`panic` せず
/// warn ログを出してスキップする（表示すべきものが正当に無いだけで観測失敗ではない）。通常の emo2 fixture は
/// 両 target とも表示するため assert が走る。
///
/// **`read_back` が成功した後の照会 `None` はスキップ事由にしない** — 合成面が在る＝表示経路が生成点まで
/// 到達している以上、「実適用 k が無い」は正当な非表示ではなく観測失敗（表示成立点に届かなかった）だからである。
pub(super) fn assert_startup_golden(
    presenter: &EmoPresenter,
    world: &World,
    window: Entity,
    target: TargetId,
    golden: std::result::Result<ComposedSurface, ComposeError>,
    label: &str,
) {
    let golden = match golden {
        Ok(cs) => cs,
        Err(e) => {
            tracing::warn!(
                ?target,
                error = %e,
                "emo-present: golden 合成に失敗 — {label} の起動時 golden assert をスキップ"
            );
            return;
        }
    };

    let actual = match presenter.read_back(target) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(
                ?target,
                error = %e,
                "emo-present: 表示に渡した合成面が無い（read_back 不可）— {label} の起動時 golden assert をスキップ（正当な非表示）"
            );
            return;
        }
    };

    // presenter と同一経路・同一入力で k を導出する。`DPI` component 不在は `None` のまま渡し、
    // [`derive_scale`] 側の縮退へ落とす（96 を捏造しない）。
    let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
    let scale = derive_scale(ScalePolicy::new(AUTHOR_DPI, ScaleRatio::ONE), window_dpi);

    let (native_w, native_h) = (golden.width(), golden.height());
    // 窓寸の照合に使う物理寸は丸め単一権威だけで導く（`as_f32` の掛け算で復元しない）。
    let (scaled_w, scaled_h) = scale.scaled_extent(native_w, native_h);

    // 推定 k が実適用 k と一致することを、丸め単一権威を通した物理寸で検査する。合成面が在るのに
    // 照会が `None` なら表示成立点へ届いていない＝観測失敗ゆえ loud に落とす。
    let applied_physical = presenter.target_physical_size(target).unwrap_or_else(|| {
        panic!(
            "起動時 golden 検証不能 [{label} / {target:?}]: 合成面は在る（read_back 成功）のに \
             target_physical_size が None — 表示成立点へ到達していない（R6.2 観測失敗）"
        )
    });
    assert_eq!(
        (scaled_w, scaled_h),
        applied_physical,
        "起動時 golden 不一致 [{label} / {target:?}]: 導出 k={:?}（窓 DPI {:?} ÷ author {AUTHOR_DPI}）を \
         丸め権威に通した物理寸 {scaled_w}x{scaled_h}（native {native_w}x{native_h}）が presenter の実適用物理寸 {:?} と不一致 \
         — 推定 k が実適用 k と食い違う（R6.2 観測失敗）",
        scale,
        window_dpi,
        applied_physical,
    );
    // 照会契約側の k（出口ビュー f32）とも突き合わせる（診断と一致検査のみ・寸法演算には使わない）。
    assert_eq!(
        presenter.applied_scale(target),
        Some(scale.as_f32()),
        "起動時 golden 不一致 [{label} / {target:?}]: applied_scale（照会値）が導出 k={:?} と不一致",
        scale,
    );

    let expected = golden.bytes();

    // まず長さで loud に落とす。長さは native 原寸（`w*h*4`）で k に依らない。k で伸びたら拡大が読み戻しへ漏れている。
    let native_len = (native_w as usize) * (native_h as usize) * 4;
    assert_eq!(
        actual.len(),
        native_len,
        "起動時 golden 不一致 [{label} / {target:?}]: read_back バイト長 {} が native 原寸長 {native_len} \
         （{native_w}x{native_h}×4）と不一致 — k={:?} の拡大が読み戻しへ漏れている（要件 6.2 観測失敗）",
        actual.len(),
        scale,
    );
    assert_eq!(
        actual.len(),
        expected.len(),
        "起動時 golden 不一致 [{label} / {target:?}]: read_back バイト長 {} が golden バイト長 {} と不一致 \
         — golden は native {native_w}x{native_h}。読み戻しが合成結果と食い違う（R6.2/R8.2 観測失敗）",
        actual.len(),
        expected.len(),
    );

    if let Some(idx) = actual.iter().zip(expected.iter()).position(|(a, b)| a != b) {
        panic!(
            "起動時 golden 不一致 [{label} / {target:?}]: 先頭相違 index={idx} (read_back=0x{:02X}, golden=0x{:02X}, len={}) \
             — golden は native {native_w}x{native_h}（k={:?} は GPU 変換側ゆえ読み戻しに掛からない）。読み戻しが \
             合成結果とバイト不一致（R6.2/R8.2/R8.3 観測失敗）",
            actual[idx],
            expected[idx],
            actual.len(),
            scale,
        );
    }

    tracing::info!(
        ?target,
        len = actual.len(),
        k_ratio = ?scale,
        k = scale.as_f32(),
        window_dpi = ?window_dpi,
        native_w,
        native_h,
        scaled_w,
        scaled_h,
        "emo-present: 起動時 golden バイト一致を確認（{label}・native 原寸で比較・k に依らず同一）"
    );
}

/// フレームクロック駆動でシェル target を数秒周期で巡回させる system。
///
/// wintf の `FrameTime`（f64 秒）を基準に、[`CYCLE_INTERVAL_SECS`] を跨いだフレームで
/// [`CycleState::next`] へ進めて対応する [`PresentCommand`] を `EmoPresenter::apply` で発行する。
/// 装着（`boot_present_system`）が済み、かつシェルが巡回対象（`shell_cycling`）のときのみ動く。
/// `apply`/presenter は `&mut World` と NonSend を要するため排他 system とし、切替が起きる
/// フレームだけ `EmoBoot` を remove→駆動→insert する。
pub(super) fn cycle_present_system(world: &mut World) {
    let now = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);

    let due = match world.get_non_send::<EmoBoot>() {
        Some(b) if b.attached && b.shell_cycling => now >= b.next_switch_at,
        _ => return,
    };
    if !due {
        return;
    }

    let mut boot = world
        .remove_non_send::<EmoBoot>()
        .expect("直上で存在確認済み");

    boot.cycle_state = boot.cycle_state.next();
    boot.next_switch_at = now + CYCLE_INTERVAL_SECS;
    // ComposeCache は surface id＋bind 集合をキーにするため、binds が異なる目開き/目閉じは自然に再合成される。
    let cmd = boot.cycle_state.command();
    boot.presenter.apply(world, cmd);
    tracing::info!(state = ?boot.cycle_state, "emo-present: シェル surface を切替");
    // 巡回で surface／bind が変われば native 原寸も変わりうるため、apply 直後に要求を消費する。
    reconcile_present_sizes(&mut boot, world);

    world.insert_non_send(boot);
}

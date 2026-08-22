use super::{
    AUTHOR_DPI, CYCLE_INTERVAL_SECS, ComposeError, ComposedSurface, DPI, EmoBoot, EmoPresenter,
    Entity, FrameTime, Point, SHELL_INITIAL_X, SHELL_INITIAL_Y, ScalePolicy, ScaleRatio, SizeI,
    TargetId, WindowPos, World, compute_balloon_pos, derive_scale, resample,
};

// ---------------------------------------------------------------------------
// 窓寸 reconcile（areka-P0-emo-dpi-scaling task 5.2・R7.1/R7.2）
// ---------------------------------------------------------------------------

/// 窓寸 reconcile 時の**位置の決め方**の別（本番 `emo2_boot::frame::GhostWindowKind` の同型）。
///
/// 本番は char 窓＝`resize_window_to`（`Anchored` 射影で接地点＝下端中央を保つ）／balloon 窓＝
/// `resize_window_keep_position`（位置は `follow_balloon` が決める従属量ゆえ据え置き）に振り分ける。
/// 本 example の窓は placement 層を通さず生成されるため `Anchored` を持たず（`collision-probe.rs:34`
/// の既知記述）、`resize_window_to` は不発である——ゆえに位置の扱いだけ本 example の構図へ合わせる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReconcileKind {
    /// シェル窓: 現在位置（[`SHELL_INITIAL_X`]/[`SHELL_INITIAL_Y`] の左上）を据え置く。
    ///
    /// **「キャラ窓の原点は下端中央」規約（寸法変動で足元が動かない）を本 example では再現しない**:
    /// あの規約は placement 層の `Anchored`＋`project_anchor` が担う不変条件であり、本 example の窓は
    /// アンカーを持たず初期位置も定数リテラルゆえ「保つべき接地点」が存在しない。ここで手作りの
    /// 下端中央射影を書けば**アンカー規約の第 2 の流儀**を観測エイドの中に生やすことになる（本番と
    /// 別流儀を発明しない）。窓配置の観測は `window-placement.rs`／`collision-probe.rs` の領分である。
    Shell,
    /// バルーン窓: 位置はシェル＋自幅から決まる従属量ゆえ [`compute_balloon_pos`] で再算出する。
    ///
    /// 本番の `resize_window_keep_position`（位置据え置き）に対応するが、本 example には
    /// `follow_balloon` に当たる追従 system が無く、位置は生成時に [`compute_balloon_pos`] が一度
    /// 決めたきりである。k≠1.0 でバルーン幅が伸びた分だけ据え置きでは既定整列（バルーン右端＝
    /// シェル左端）が崩れてシェルへ食い込むため、**同じ関数**へ新幅を与えて再算出する（追従の
    /// 代役はこの 1 箇所に閉じる）。
    Balloon,
}

/// 表示成立点の窓寸 reconcile 要求を消費し、窓 client を k 適用後の物理寸へ合わせる
/// （emo-dpi-scaling task 5.2・R7.1/R7.2）。
///
/// 各 target について [`EmoPresenter::take_pending_resize`] を引き、`Some(新物理寸)` のときだけ
/// [`reconcile_window_size`] で反映する。本番 `emo2_boot` の `reconcile_reported_sizes` と同じ流儀:
///
/// - **消費者は表示を引き起こした者**（消費規約）。本 example は presenter を直接所有し `apply` を
///   自ら呼ぶ唯一の主体であり、`refresh_scale`（もう一方の消費者）は呼ばない——ゆえに要求を取り
///   逃す第三者が存在しない。
/// - **報告は「状態」であって「エッジ」ではない**。要求は取り出されるまで消えず、取り出した後は
///   同寸表示を何度繰り返しても `None`（＝窓への無用な書込＝churn を生まない）。
/// - **k=1.0 でも初回表示は必ず報告される**（presenter 側の契約）。ただし窓は当該 surface の native
///   原寸で生成済みゆえ、反映は同寸のべき等 skip となり書込は 1 バイトも起きない（従来と挙動同一）。
///
/// # 報告は「DPI 変化」ではなく「表示物理寸の変化」に紐づく
///
/// presenter が要求を積む条件は *k* の変化ではなく**物理寸（＝native 原寸 × k）の変化**である。ゆえに
/// k=1.0 でも、巡回で表示 surface が変わって native 原寸が変われば窓はそれに追従する——これは本番
/// （`reconcile_reported_sizes`／resnap）と同一の意味論であり、「窓は surface0 の寸のまま surface1000 を
/// 表示して端が欠ける」という従前の（k とは無関係な）取りこぼしも同じ経路で解消される。同寸の切替
/// （emo2 のまばたきは合成外形が変わらない）では要求が積まれず、窓は一切書かれない（churn なし）。
///
/// 窓 entity が [`Entity::PLACEHOLDER`]（構築失敗で窓を作らなかった）のに要求が出るのは、装着済み
/// target と窓生成の食い違い＝結線バグゆえ `error!` で loud に観測する（silent skip にしない）。
pub(super) fn reconcile_present_sizes(boot: &mut EmoBoot, world: &mut World) {
    for (target, window, kind) in [
        (TargetId(0), boot.shell_window, ReconcileKind::Shell),
        (TargetId(1), boot.balloon_window, ReconcileKind::Balloon),
    ] {
        // 要求なし＝物理寸が前回適用寸から変わっていない／未表示／既に消費済み → 窓を触らない。
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
/// 弾く（log-first・panic しない・本番と同じ二重防波堤）。**f32 を寸法演算に用いない**——値は
/// presenter の丸め単一権威（`ScaleRatio::scaled_extent` 経由の
/// [`EmoPresenter::target_physical_size`] と同一の計算）から来ており、ここでは整数のまま運ぶだけである。
///
/// 反映は `WindowPos` への通常書込（変更検知あり）で行う。wintf の `apply_window_pos_changes`
/// （`Changed<WindowPos>`・`UISetup`）が次フレームに `SetWindowPos` を発行し、`WindowPos.position` の
/// 変更は `sync_window_arrangement_from_window_pos` が `Arrangement.offset` へ同期する——単一ライター
/// 規律を持つ本番 placement 層（`enqueue_window_set_pos`）を example から迂回して呼ばず、wintf 標準の
/// 反映経路をそのまま使う。
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

    // 位置: シェルは据え置き（アンカー不在）・バルーンは新幅で既定整列を再算出する（[`ReconcileKind`]）。
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
    // 据え置き（Shell）は現在位置をそのまま目標値とする＝位置は書き換わらない。
    let position = recomputed_pos.or(wp.position);

    // べき等 skip（振動・churn 防止）: 同寸・同位置なら書込を一切行わない。k=1.0 の初回報告は
    // ここで吸収され、窓は生成時の native 原寸のまま 1 バイトも書かれない（従来と挙動同一）。
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

/// 起動時 golden バイト一致 assert（task 5.1／5.1 追補・R6.2/R6.7/R7.1/R7.2/R8.2/R8.3）。
///
/// 初回表示直後に target の表示画素を `EmoPresenter::read_back`（swap chain backbuffer の CPU 読み戻し・
/// R8.3）で取得し、その surface を **表示経路と同じ 2 段変換**（`Composer::compose`＝native 原寸 →
/// [`resample`]＝実適用 k）に通した golden [`ComposedSurface`] のバイト列（[`ComposedSurface::bytes`]）と
/// **完全一致**（full byte equality）することを検証する。これが「供給面（swap chain readback）と合成結果の
/// 一致」（R8.2）を決定論的に確かめる検証シーム（R6.7）である。
///
/// # なぜ golden にも k を掛けるのか（task 5.1 追補）
///
/// `EmoPresenter::apply_show` の表示経路は **compose（native 原寸）→ resample（k 適用）→ cache → 表示**
/// であり、swap chain backbuffer が保持するのは **k 適用後の物理 px** である。ゆえに golden を native 原寸の
/// まま突き合わせると、k≠1.0 の実機（例: 125% ＝ dpi 120 ＝ k=5/4）では**長さの時点で必ず食い違い**、
/// 手動検証エイドである本 example がそもそも起動できない。golden 側にも同一の変換を通すことで、檻
/// （design「Testing Strategy > Integration Tests」）が課すのと同じ契約を手動エイドにも適用する。
///
/// k=1.0 では [`ScaleRatio::is_identity`] 経路で resample を**呼ばず** native を素通しする（presenter 側と
/// 同一の素通し）ため、96 DPI 環境での比較対象は k 導入前と 1 バイトも変わらない（R7.2）。
///
/// # k の導出と、その一致検査
///
/// k は presenter と同一の純関数 [`derive_scale`] へ同一入力（target 政策＝[`AUTHOR_DPI`]／窓の `DPI`
/// component）を与えて求める。ただしこれは**推定**であり、実際に表示へ掛かった k の単一真実源は presenter
/// 側の `applied` である。ゆえに推定 k をそのまま信用せず、
///
/// - [`EmoPresenter::target_physical_size`]（＝`scaled_extent(applied, native 原寸)`・丸め単一権威）と
///   推定 k から求めた golden の物理寸が一致すること、
/// - [`EmoPresenter::applied_scale`]（照会契約の実適用 k）が推定 k と一致すること
///
/// を **assert で検査**する（食い違えば loud に落ちる＝黙って別の k で比較しない）。
///
/// # 失敗を silent にしない（R6.2）
///
/// 寸法・バイト長・内容のいずれかが食い違えば即 `panic!`／`assert_eq!` で loud に落とす（target id・
/// 適用 k・golden の native 原寸と変換後寸法・期待/実測長・先頭相違 index を添える）。観測失敗を warn
/// ログで握り潰さない。
///
/// # 正当な非表示のスキップ
///
/// golden 合成に失敗した場合、または供給面が未生成（`read_back` が [`areka_emo_present::PresentError`] を返す・
/// EmptyComposition degradation 等で chain 不在）の場合は、`panic` せず warn ログを出してスキップする（表示すべき
/// ものが正当に無いだけで観測失敗ではない）。通常の emo2 fixture は両 target とも表示するため assert が走る。
///
/// **`read_back` が成功した後の照会 `None` はスキップ事由にしない** — 供給面が在る＝表示経路が生成点まで
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
                "emo-present: 供給面が未生成（read_back 不可）— {label} の起動時 golden assert をスキップ（正当な非表示）"
            );
            return;
        }
    };

    // presenter と同一経路・同一入力で k を導出する（`DPI` component 不在は `None` のまま渡し、
    // [`derive_scale`] 側の縮退（error! ＋ k=1.0）へ落とす＝96 を捏造しない）。
    let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
    let scale = derive_scale(ScalePolicy::new(AUTHOR_DPI, ScaleRatio::ONE), window_dpi);

    // 表示経路と同じ変換を golden へ適用する（恒等 k は resample を呼ばず native 素通し）。
    let (native_w, native_h) = (golden.width(), golden.height());
    let display = if scale.is_identity() {
        golden
    } else {
        let mut scaled = ComposedSurface::new(0, 0);
        resample(&golden, scale, &mut scaled);
        scaled
    };
    let (scaled_w, scaled_h) = (display.width(), display.height());

    // 推定 k が **実適用 k** と一致することを、丸め単一権威を通した物理寸で検査する
    // （`as_f32` の掛け算で復元しない＝D4）。供給面が在るのに照会が `None` なら表示成立点へ
    // 届いていない＝観測失敗ゆえ loud に落とす。
    let applied_physical = presenter.target_physical_size(target).unwrap_or_else(|| {
        panic!(
            "起動時 golden 検証不能 [{label} / {target:?}]: 供給面は在る（read_back 成功）のに \
             target_physical_size が None — 表示成立点へ到達していない（R6.2 観測失敗）"
        )
    });
    assert_eq!(
        (scaled_w, scaled_h),
        applied_physical,
        "起動時 golden 不一致 [{label} / {target:?}]: golden へ掛けた k={:?}（窓 DPI {:?} ÷ author {AUTHOR_DPI}）の \
         変換後寸法 {scaled_w}x{scaled_h}（native {native_w}x{native_h}）が presenter の実適用物理寸 {:?} と不一致 \
         — 推定 k が実適用 k と食い違う（R6.2 観測失敗）",
        scale,
        window_dpi,
        applied_physical,
    );
    // 照会契約側の k（出口ビュー f32）とも突き合わせる（寸法演算には使わない・診断と一致検査のみ）。
    assert_eq!(
        presenter.applied_scale(target),
        Some(scale.as_f32()),
        "起動時 golden 不一致 [{label} / {target:?}]: applied_scale（照会値）が導出 k={:?} と不一致",
        scale,
    );

    let expected = display.bytes();

    // まず長さで loud に落とす（相違の一次要因を明示）。
    assert_eq!(
        actual.len(),
        expected.len(),
        "起動時 golden 不一致 [{label} / {target:?}]: read_back バイト長 {} が golden バイト長 {} と不一致 \
         — k={:?} 適用後 golden は {scaled_w}x{scaled_h}（native {native_w}x{native_h}）。swap chain readback が \
         合成結果と食い違う（R6.2/R8.2 観測失敗）",
        actual.len(),
        expected.len(),
        scale,
    );

    // full byte equality: 先頭相違 index を添えて loud に panic する。
    if let Some(idx) = actual.iter().zip(expected.iter()).position(|(a, b)| a != b) {
        panic!(
            "起動時 golden 不一致 [{label} / {target:?}]: 先頭相違 index={idx} (read_back=0x{:02X}, golden=0x{:02X}, len={}) \
             — k={:?} 適用後 golden {scaled_w}x{scaled_h}（native {native_w}x{native_h}）。swap chain readback が \
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
        "emo-present: 起動時 golden バイト一致を確認（{label}・k 適用後の物理 px で比較）"
    );
}

/// フレームクロック駆動でシェル target を数秒周期で巡回させる system（R3.2/R6.4）。
///
/// wintf の `FrameTime`（f64 秒・毎フレーム更新）を基準に経過を測り、[`CYCLE_INTERVAL_SECS`] を跨いだ
/// フレームで [`CycleState::next`] へ進めて対応する [`PresentCommand`] を `EmoPresenter::apply`（指令 API）で
/// 発行する（bypass しない）。装着（`boot_present_system`）が済み、かつシェルが巡回対象（`shell_cycling`）の
/// ときのみ動く。`apply`/presenter は `&mut World` と NonSend を要するため排他 system とし、切替が起きる
/// フレームだけ `EmoBoot` を remove→駆動→insert する（未到達フレームは peek のみで churn を避ける）。
pub(super) fn cycle_present_system(world: &mut World) {
    // 現在時刻（フレームクロック）。未挿入時は 0.0（切替は起きない）。
    let now = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);

    // 装着済み・巡回対象・切替時刻到達を peek で確認（未到達なら remove/insert しない）。
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
    // ComposeCache は合成入力（surface id＋bind 集合）をキーにするため、同一 surface1000 でも
    // binds が異なる目開き/目閉じは自然にミス＝再合成される（InvalidateCache の手動発行は不要）。
    let cmd = boot.cycle_state.command();
    boot.presenter.apply(world, cmd);
    tracing::info!(state = ?boot.cycle_state, "emo-present: シェル surface を切替");
    // 切替後の窓寸 reconcile（emo-dpi-scaling task 5.2）: 巡回で surface／bind が変われば native 原寸も
    // 変わりうる（＝k 適用後の物理寸も変わる）。本番 drain と同じく apply 直後に要求を消費する。
    // 物理寸が変わらない切替（emo2 のまばたきは同寸）では要求が積まれず no-op＝窓を触らない。
    reconcile_present_sizes(&mut boot, world);

    world.insert_non_send(boot);
}

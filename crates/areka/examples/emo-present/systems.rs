use super::{
    AUTHOR_DPI, Added, BalloonWindowMarker, BindSet, CYCLE_INTERVAL_SECS,
    ClickThroughRegistryHandle, Composer, EmoBoot, Entity, FrameTime, GraphicsCore, NonSend, Or,
    PatternState, PresentCommand, Query, ShellWindowMarker, TargetId, WindowHandle, With, World,
    WucGraphicsResource, assert_startup_golden, reconcile_present_sizes,
};

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// クリック透過機構への窓登録システム（mock-shell donor から移植）。
///
/// WUC 化により ULW の自動 α ヒットテストが失われるため、機構が α を評価できるよう shell/balloon の
/// 2 窓を明示登録する。`WindowHandle` は窓生成（UISetup）が HWND 生成後に付与するため
/// `Added<WindowHandle>` で「HWND が付いた瞬間」を捉え、各窓を厳密に 1 回登録する（`register` は
/// 同一 Entity 再登録を dedupe するため冪等でもある）。`ClickThroughRegistryHandle` は `WinApp::run`
/// の結線で NonSend リソースとして挿入される。ごく初期の tick で未挿入の可能性へ `Option` で防御する。
pub(super) fn register_click_through_windows(
    new_windows: Query<
        (Entity, &WindowHandle),
        (
            Added<WindowHandle>,
            Or<(With<ShellWindowMarker>, With<BalloonWindowMarker>)>,
        ),
    >,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (entity, wh) in new_windows.iter() {
        handle.register(entity, wh.hwnd);
        tracing::debug!(?entity, "emo-present: クリック透過機構へ窓を登録");
    }
}

/// GPU 資源到達フレームで `attach_target`→`apply(ShowSurface)` を各 target 高々 1 回駆動する起動 system。
///
/// `EmoPresenter::apply`/`attach_target` は `&mut World` と NonSend な presenter を要するため排他 system
/// （`&mut World`）とし、`EmoBoot` を World から取り出して駆動する（借用衝突を避けるため remove→駆動→
/// insert）。GPU 資源（`GraphicsCore`/`WucGraphicsResource`）は wintf が窓生成後に遅延挿入するため、
/// 揃うまでは保留し次 tick で再試行する。装着後は `attached` で以降を no-op 化する。
pub(super) fn boot_present_system(world: &mut World) {
    // 未挿入 or 装着済みなら何もしない（装着後の remove/insert churn を避ける）。
    match world.get_non_send::<EmoBoot>() {
        Some(b) if !b.attached => {}
        _ => return,
    }

    // GPU 資源の準備待ち（未準備なら EmoBoot を保持したまま次 tick へ）。
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    let mut boot = world
        .remove_non_send::<EmoBoot>()
        .expect("直上で存在確認済み");

    if let Some((emo_world, atlas)) = boot.shell_assets.take() {
        // 起動時 golden（task 5.1・R6.2/R8.2）: 初回表示は Surface0（surface_id=0・bind 無し）ゆえ、
        // その surface を **直接合成**した ComposedSurface を golden として先に採取する。attach_target が
        // アセットを move 消費するため、合成は move の前に行う（read_back との突き合わせは表示直後）。
        let shell_golden = Composer::new().compose(
            &emo_world,
            &atlas,
            0,
            &BindSet::default(),
            &PatternState::default(),
        );
        match boot.presenter.attach_target(
            world,
            TargetId(0),
            boot.shell_window,
            emo_world,
            atlas,
            // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
            // 供給する。窓 DPI が 96 の環境では k=1.0（従来と同一の表示寸・描画結果）、
            // 非 96 DPI ではその比が k として表示へ掛かる。
            AUTHOR_DPI,
        ) {
            Ok(()) => {
                // 起動時 golden 検証用に surface0（bind 無し）を先に表示する（本編の巡回は surface1000 の
                // まばたきゆえ、golden の基準となる surface0 はここで一度だけ明示表示する）。
                boot.presenter.apply(
                    world,
                    PresentCommand::ShowSurface {
                        target: TargetId(0),
                        surface_id: 0,
                        binds: BindSet::default(),
                        pattern: PatternState::default(),
                        reply: None,
                    },
                );
                // 起動時 golden バイト一致 assert（R6.2/R6.7/R8.2/R8.3）: swap chain readback ==
                // 「直接合成 → 実適用 k で resample」した golden を full byte equality で検証する
                // （不一致は loud に panic）。
                assert_startup_golden(
                    &boot.presenter,
                    world,
                    boot.shell_window,
                    TargetId(0),
                    shell_golden,
                    "shell surface0",
                );
                // --- 手動デモ: さくらスクリプト相当の表情＋まばたきを指令切替で再現 ---
                // \s[1000]\![bind,腕,組み,1]\![bind,紅,差し,0]\![bind,口,‥‥,1]\![bind,眉,悲しみ,1]
                //         \![bind,目,通常,1]\![bind,まばたき,通常,1] を seriko 代役でハンドコンパイル。
                // 共通表情（腕組み=1101・口‥‥=1206・目通常=1302・眉悲しみ=1502・髪飾りリボン=1800／紅なし）を
                // surface1000 に合成表示し、まばたき（1400）を CYCLE_INTERVAL_SECS 周期で出し入れして目の開閉を
                // 手動再現する（EyesOpen⇔EyesClosed）。まばたきの時間駆動アニメ本体は seriko エンジン領分（別 spec・未実装）。
                boot.presenter.apply(world, boot.cycle_state.command()); // 初回＝EyesOpen（surface1000）
                let now = world
                    .get_resource::<FrameTime>()
                    .map(|ft| ft.0)
                    .unwrap_or(0.0);
                boot.shell_cycling = true;
                boot.next_switch_at = now + CYCLE_INTERVAL_SECS;
            }
            Err(e) => tracing::error!(error = %e, "emo-present: シェル target の attach に失敗"),
        }
    }

    if let Some((emo_world, atlas)) = boot.balloon_assets.take() {
        // 起動時 golden（task 5.1・R6.2/R8.2）: バルーンの初回表示は surface_id=0・bind 無し。
        // attach_target が move 消費する前に golden を採取する。
        let balloon_golden = Composer::new().compose(
            &emo_world,
            &atlas,
            0,
            &BindSet::default(),
            &PatternState::default(),
        );
        match boot.presenter.attach_target(
            world,
            TargetId(1),
            boot.balloon_window,
            emo_world,
            atlas,
            // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が balloon descript の
            // 実値を供給する。シェルと同一の分母を用いる（[`AUTHOR_DPI`]）。
            AUTHOR_DPI,
        ) {
            Ok(()) => {
                boot.presenter.apply(
                    world,
                    PresentCommand::ShowSurface {
                        target: TargetId(1),
                        surface_id: 0,
                        binds: BindSet::default(),
                        pattern: PatternState::default(),
                        reply: None,
                    },
                );
                // 起動時 golden バイト一致 assert（R6.2/R6.7/R8.2/R8.3）。
                assert_startup_golden(
                    &boot.presenter,
                    world,
                    boot.balloon_window,
                    TargetId(1),
                    balloon_golden,
                    "balloon surface0",
                );
            }
            Err(e) => tracing::error!(error = %e, "emo-present: バルーン target の attach に失敗"),
        }
    }

    // 窓寸 reconcile（emo-dpi-scaling task 5.2・R7.1/R7.2）: 本フレームの全 apply が済んだ**後**に
    // 表示成立点の状態照合が積んだ要求を消費して窓 client を合わせる（本番 `emo2_frame_system` が
    // drain の後段で呼ぶ `reconcile_reported_sizes` と同順序）。初回表示が積む k₀ 補正はここで landing する。
    reconcile_present_sizes(&mut boot, world);

    boot.attached = true;
    world.insert_non_send(boot);
    tracing::info!("emo-present: 2 窓へ surface0/バルーン枠を装着・表示しました");
}

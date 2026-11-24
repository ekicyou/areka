use super::{
    AUTHOR_DPI, Added, BalloonWindowMarker, BindSet, CYCLE_INTERVAL_SECS,
    ClickThroughRegistryHandle, Composer, EmoBoot, Entity, FrameTime, GraphicsCore, NonSend, Or,
    PatternState, PresentCommand, Query, ShellWindowMarker, TargetId, WindowHandle, With, World,
    WucGraphicsResource, assert_startup_golden, reconcile_present_sizes,
};

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// クリック透過機構への窓登録システム。
///
/// WUC 化により ULW の自動 α ヒットテストが失われるため、機構が α を評価できるよう shell/balloon の
/// 2 窓を明示登録する。`Added<WindowHandle>` で HWND が付いた瞬間を捉え、各窓を 1 回登録する。
/// `ClickThroughRegistryHandle` は `WinApp::run` の結線で挿入されるため、ごく初期の tick の
/// 未挿入に `Option` で備える。
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
/// `EmoPresenter::apply` は `&mut World` と NonSend な presenter を要するため排他 system とし、
/// `EmoBoot` を remove→駆動→insert する。GPU 資源（`GraphicsCore`/`WucGraphicsResource`）は wintf が
/// 窓生成後に遅延挿入するため、揃うまでは保留し次 tick で再試行する。装着後は `attached` で no-op 化する。
pub(super) fn boot_present_system(world: &mut World) {
    match world.get_non_send::<EmoBoot>() {
        Some(b) if !b.attached => {}
        _ => return,
    }

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
        // 起動時 golden: 初回表示は Surface0（surface_id=0・bind 無し）ゆえ、その surface を直接合成した
        // ComposedSurface を golden として先に採取する。attach_target がアセットを move 消費するため、
        // 合成は move の前に行う。
        let shell_golden = Composer::new().compose(
            &emo_world,
            &atlas,
            0,
            &BindSet::default(),
            &PatternState::default(),
        );
        boot.presenter.attach_target(
            TargetId(0),
            boot.shell_window,
            emo_world,
            atlas,
            // 作者基準 DPI は正典既定の 96（ukadoc）。本番は boot が descript の実値を供給する。
            AUTHOR_DPI,
        );
        // golden の基準となる surface0 を、巡回（surface1000）の前に一度だけ明示表示する。
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
        // 起動時 golden バイト一致 assert: read_back（native 原寸）と直接合成した golden を比べる。
        assert_startup_golden(
            &boot.presenter,
            world,
            boot.shell_window,
            TargetId(0),
            shell_golden,
            "shell surface0",
        );
        // 手動デモ: 共通表情（腕組み=1101・口‥‥=1206・目通常=1302・眉悲しみ=1502・髪飾りリボン=1800／紅なし）を
        // surface1000 に合成表示し、まばたき（1400）を CYCLE_INTERVAL_SECS 周期で出し入れして目の開閉を
        // 手動再現する。時間駆動のアニメ本体は seriko エンジンの領分。
        boot.presenter.apply(world, boot.cycle_state.command()); // 初回＝EyesOpen（surface1000）
        let now = world
            .get_resource::<FrameTime>()
            .map(|ft| ft.0)
            .unwrap_or(0.0);
        boot.shell_cycling = true;
        boot.next_switch_at = now + CYCLE_INTERVAL_SECS;
    }

    if let Some((emo_world, atlas)) = boot.balloon_assets.take() {
        // バルーンの初回表示は surface_id=0・bind 無し。attach_target が move 消費する前に golden を採取する。
        let balloon_golden = Composer::new().compose(
            &emo_world,
            &atlas,
            0,
            &BindSet::default(),
            &PatternState::default(),
        );
        boot.presenter.attach_target(
            TargetId(1),
            boot.balloon_window,
            emo_world,
            atlas,
            // 作者基準 DPI は正典既定の 96（ukadoc）。シェルと同一の分母を用いる（[`AUTHOR_DPI`]）。
            AUTHOR_DPI,
        );
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
        assert_startup_golden(
            &boot.presenter,
            world,
            boot.balloon_window,
            TargetId(1),
            balloon_golden,
            "balloon surface0",
        );
    }

    // 本フレームの全 apply が済んだ後に、表示成立点の状態照合が積んだ要求を消費して窓 client を合わせる
    // （本番 `frame_system` と同順序）。初回表示が積む k₀ 補正はここで反映される。
    reconcile_present_sizes(&mut boot, world);

    boot.attached = true;
    world.insert_non_send(boot);
    tracing::info!("emo-present: 2 窓へ surface0/バルーン枠を装着・表示しました");
}

//! 起こし直しの単位（areka-P0-ghost-restart-unit）。
//!
//! ゴーストを 1 体起こすのに要る手順を、**プロセスに 1 回**のもの（系の登録）と
//! **ゴーストごと**のもの（状態の載せ替え）に分けて置く場所である。系はどれも状態
//! （`NonSend`／`Resource`）が無ければ無操作で戻る自己防御を持つので、登録を先に 1 度だけ
//! 済ませ、状態は後から入れ替えても系は新しい状態を見る。
//!
//! 今ここに在るのは登録の入口 [`register_systems`] と、窓を作る側 [`open_ghost_windows`]・
//! 閉じた証を受けて窓を作り直す [`reopen_ghost_windows`] である。登録の呼び手は `fn main` の
//! 1 か所（`WinApp` を作った直後・起動窓の前）。各結線（`wire_*`）は状態の挿入だけを行い、
//! 系を登録しない。

use std::sync::mpsc::Receiver;

use areka_kanade::KanadeStopped;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use wintf::ecs::FrameFinalize;
use wintf::ecs::widget::bitmap_source::{CommandSender, WintfTaskPool};

use crate::ConfigInputs;
use crate::app_exit::{self, WindowsClosed};
use crate::emo2_boot;
use crate::input_events;
use crate::menu;
use crate::placement;
use crate::readme;

/// 系の登録をプロセスに 1 回・1 か所から行う（要件 2.1・2.2・3.3）。
///
/// 載せる系と順序（段ごとに今日の挿入順を保つ）:
/// - `Update` ← 毎フレームの相 → 停止通知の受け口（受け口 `KanadeStopRx` はプロセスに 1 つ
///   なので [`emo2_boot::wire_kanade_stop`] を分割せずそのまま呼ぶ）
/// - `Input` ← 説明書 → 中断 → メニュー → バルーンの離脱 → 選択肢の送り
/// - `FrameFinalize` ← クリック透過 → OS の閉鎖要求 → 重なり順の対（状態がゴーストごとで
///   ないので `wire_zorder_pair` をそのまま呼ぶ）
///
/// 各系の並び（`before`／`after`・`chain`）は各登録関数が持つ。ここは順に呼ぶだけ。
///
/// 今日との差は 1 件: `Input` の 5 系は LogSink の起動でも登録される。5 系はどれも状態が
/// 無ければ `trace!` だけで戻るので、見え方は変わらない。
pub(crate) fn register_systems(world: &mut World, kanade_stop_rx: Receiver<KanadeStopped>) {
    emo2_boot::register_emo2_frame_system(world);
    emo2_boot::wire_kanade_stop(world, kanade_stop_rx);

    readme::register_readme_drain(world);
    input_events::user_break::register_user_break_drain(world);
    menu::register_menu_poll(world);
    input_events::balloon::register_balloon_leave_system(world);
    input_events::choice_drain::register_choice_drain(world);

    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        placement::spawn::register_ghost_windows_click_through,
    );
    world
        .resource_mut::<Schedules>()
        .add_systems(FrameFinalize, app_exit::attach_os_close_request);
    placement::spawn::wire_zorder_pair(world);
}

/// 起動窓の準備が descript から**1 度だけ**読み取った値のうち、呼び手（`main`）が下流へ
/// 配るもの（areka-P0-emo-dpi-scaling task 4.3・areka-P0-scope-zorder-pinning 要件 5.1／5.2）。
///
/// どちらも `wire_emo2_boot` へ渡る搬送値であり、[`open_ghost_windows`] 自身は解釈しない。
/// 戻り口を 2 つに増やさず 1 つの型へまとめるのは、「同じ 1 度の読取から来た」という出所を
/// 型で示すためである——別々に読み直す余地を残すと、配置と重なりが違う宣言を見る日が来る。
pub(crate) struct StartupDescriptValues {
    /// 採寸 k₀ と attach（`attach_target`）が共有する作者基準 DPI。
    pub(crate) author_dpi: placement::AuthorDpi,
    /// shell descript の `seriko.zorder` の生の値（未指定なら `None`）。解釈は台帳の層が行う。
    pub(crate) zorder_raw: Option<String>,
}

/// 窓を作る側が失敗する理由（design「open_ghost_windows / reopen_ghost_windows」）。
#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenWindowsError {
    /// 配置の準備が通らない（モニタ 0 台・起動中の削除等）。
    #[error(transparent)]
    Placement(#[from] placement::PlacementError),
    /// 窓を積む作業プールが World に無い（本番では `EcsWorld::new` が必ず挿す＝配線の誤り）。
    #[error("窓を作るための作業プールが World に無い")]
    TaskPoolMissing,
}

/// 窓を作る側（要件 3.3・3.4・window-placement 1.4）: 配置の準備 → 監視の 2 源 → 復元 →
/// 窓の生成と受け口の装着を作業プールへ積む。
///
/// - 最初に作業プール（[`WintfTaskPool`]）の有無を確かめる。無ければ配置の準備に入らず
///   `error!(event = "task_pool_missing")` の上で [`OpenWindowsError::TaskPoolMissing`]
///   （`EcsWorld::spawn` のように黙って何もしない形は取らない）。
/// - 成功時: 既存 ECS コマンド経路（作業プールの async タスク → `CommandSender` → Input
///   スケジュールで World 適用）で窓を組み立てる（`FrameFinalize` への系の結線は
///   [`register_systems`] が先に済ませる）。smoke の自動終了はここでは仕掛けない（呼び手の
///   `fn main` が 1 度目の成功の直後に 1 度だけ仕掛ける・要件 3.5）。
/// - 失敗時（モニタ 0 台・起動中の削除等）: [`placement::PlacementError`] を包んで返す。
///   呼び手（`main`）が「起動窓を開けない」を告知して終了コード 1 で終える
///   （baseware-root-layout 要件 6.4・窓を開かずに居座らない＝要件 6.6）。
///
/// 準備（`prepare_ghost_windows`）は同期実行し、I/O はここで完結・Send な値のみを ECS
/// コマンドへ運ぶ。呼び出しスレッドは `WinApp` 構築済みの MTA UI スレッド＝COM
/// 初期化済み（measure の WIC 前提を満たす）。
///
/// 戻り値は準備が descript から**1 度だけ**読んだ値の組（[`StartupDescriptValues`]）。
/// 作者基準 DPI は (a) 採寸の k₀ と (b) 呼び手（`main`）経由で
/// `wire_emo2_boot`→`attach_target` の双方へ配られ、shell の `seriko.zorder` は同じ経路で
/// 重なりの台帳へ配られる（design Flow 3 手順1「1 度だけ読む」）。
pub(crate) fn open_ghost_windows(
    world: &mut World,
    cfg: &ConfigInputs,
) -> Result<StartupDescriptValues, OpenWindowsError> {
    let Some(task_pool) = world.get_resource::<WintfTaskPool>() else {
        tracing::error!(
            event = "task_pool_missing",
            ghost_root = %cfg.ghost_root.display(),
            "[open_ghost_windows] 作業プールが World に無いので窓を作れません（配置の準備に入らず失敗を返す）"
        );
        return Err(OpenWindowsError::TaskPoolMissing);
    };
    let prepared = placement::prepare_ghost_windows(&cfg.ghost_root, &cfg.balloon_root)?;
    // モニタ 2 源（task 8.1・atom task 5.1）: 起動時の実モニタから忠実転写した
    // 作業領域源とモニタ別拡大率表（物理 px・Send な純粋データ）。bottom 吸着ドラッグ
    // （4.7・task 8.2）と拡大率の相が消費する。
    // **セッション内固定ではない**（DD15 撤回・atom 要件 5.1）——毎フレーム先頭の
    // 同期段（`emo2_boot::frame::work_area_sync`）が実行時のモニタ表から作り直す。
    // ここが作るのは起動時の初期値であり、構築関数は同期段と同一である。
    // 構築と同時に全モニタの観測を 1 回出す（areka-P0-dpi-window-vanish 要件 1.1 の
    // **正典出力点**・D12）。既定 OFF・診断 `RUST_LOG` でのみ点灯する。
    let sources = crate::boot_monitor_snapshot(&wintf::ecs::window::monitor::enumerate_monitors());

    // clickthrough 登録・OS の閉鎖要求の受け手・重なり順の対の `FrameFinalize` への結線は
    // `register_systems` が済ませてある（どれも `Added<WindowHandle>` 起点ゆえ、
    // 窓 spawn より先に結線しても取りこぼさない）。

    // 復元マージ（design C4・要件 1.4）: snapshot 構築直後・spawn closure へ渡す前に、
    // 永続先読み（load_restored_state）→ 純関数 merge（apply_restored_placements）で
    // 保存位置を反映した placements を得る。`prepared` を placements/titles へ分解し、
    // merge 済み placements（value 渡し）と titles を closure へ move する
    // （default_encoding は boot 結線・source.rs と同一の Ansi＝mount 解決の一貫性）。
    // 作者基準 DPI は `prepared` 分解の前に取り出して呼び手へ返す（`Copy` 値の転記）。
    let author_dpi = prepared.author_dpi;
    // 重なりの基底の生の値も同じ読取から取り出す（解釈は結線の先＝台帳の層が行う・
    // areka-P0-scope-zorder-pinning 要件 5.2）。`prepared` を分解する前に写す。
    let zorder_raw = prepared.zorder_raw.clone();
    // 復元は**起動時の作業領域源を 1 度だけ**読む（atom 要件 5.7）。以後の同期段は
    // この判定へ効かせない——拡大率をまたぐ保存位置の追従は行わない裁定
    // （`windowposition-limit` の開発者裁定）を踏襲する。
    let (placements, restored_scopes) = crate::restore_merged_placements(
        &cfg.ghost_root,
        prepared.placements,
        &sources.snapshot,
        areka_parsers::charset::DefaultEncoding::Ansi,
    );
    let titles = prepared.titles;

    // 作業プールの async タスク → CommandSender → Input スケジュールで
    // World 適用という既存 ECS コマンド経路で本物窓を組み立てる。
    task_pool.spawn(|tx: CommandSender| async move {
        let _ = tx.send(Box::new(move |world: &mut World| {
            // 2 源は同時に挿す（片方だけ古い運転を作らない・atom C6）。
            world.insert_resource(sources.snapshot);
            world.insert_resource(sources.dpi_table);
            let windows = placement::spawn::spawn_ghost_windows(world, &placements, &titles);
            // 保存位置が復元されたスコープは既定配置ではない（scg 7.3）。台帳の
            // 既定位置を落として連鎖の再解決から常に除外する——さもないと次回起動で
            // 利用者のドラッグ位置が隣接位置へ引き戻される。spawn が Resource として
            // 挿した実体を直接標す（戻り値の clone を触っても Resource へは効かない）。
            if !restored_scopes.is_empty()
                && let Some(mut gw) = world.get_resource_mut::<placement::spawn::GhostWindows>()
            {
                for scope in &restored_scopes {
                    gw.clear_default_char_pos(*scope);
                }
            }
            // マウス入力ハンドラ装着（areka-P0-input-events・依存方向 input_events→
            // placement）: placement は `crate::` パスを持てない（example の `#[path]`
            // include で成立させるため）ゆえ、キャラ窓へのポインタハンドラ結線は
            // input_events 側が担う。spawn 直後の同一 World-mutation クロージャ内で
            // 同期実行するため、キャラ窓は既に存在し async race はない。
            input_events::attach_char_pointer_handlers(world);
            // 右クリックメニューの解放ハンドラも同じ場所で付ける。このクロージャが動くのは
            // `app.run()` の中＝`menu::wire_menu` より後で、結線の無い起動では解放を無視するだけ。
            menu::attach_release_handlers(world);
            // バルーン窓へポインタハンドラを装着（task 6.2・`attach_char_pointer_handlers`
            // 直後・R4.3/5.5）: `BalloonWindowMarker` 窓へ `OnPointerMoved`／`OnPointerPressed`
            // を post-spawn 挿入する（標的はバルーン窓のみ＝キャラ窓配線の非退行・R4.3）。同一
            // `&mut World` クロージャ内で同期実行するためバルーン窓は既に存在し async race は
            // ない（キャラ窓ハンドラ装着と同型のタイミング契約）。
            input_events::balloon::attach_balloon_pointer_handlers(world);
            let scopes: Vec<usize> = windows.scopes().collect();
            tracing::info!(
                ?scopes,
                "本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓）"
            );
        }));
    });

    Ok(StartupDescriptValues {
        author_dpi,
        zorder_raw,
    })
}

/// 閉じた証を受けて窓を作り直す（要件 3.4・4.4）。証（[`WindowsClosed`]）の唯一の消費先で、
/// 手順は 1 度目と同じ [`open_ghost_windows`] へ委譲する。
// 本番の呼び手は #13（ゴーストの切替）。それまでは test からだけ呼ぶ。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn reopen_ghost_windows(
    world: &mut World,
    cfg: &ConfigInputs,
    closed: WindowsClosed,
) -> Result<StartupDescriptValues, OpenWindowsError> {
    tracing::debug!(
        closed = closed.closed(),
        "[reopen_ghost_windows] 閉じた窓の後に窓を作り直す"
    );
    open_ghost_windows(world, cfg)
}

#[cfg(test)]
#[path = "ghost_session_restart_tests.rs"]
mod restart_tests;

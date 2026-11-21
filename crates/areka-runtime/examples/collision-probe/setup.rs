use super::fixture::{PLACEHOLDER_SIZE, balloon_root, emo2_root};
use super::pointer::on_probe_pointer_moved;
use super::state::{ProbeBoot, ProbePhase};
use super::{
    AtlasTable, CommandSender, EmoPresenter, EmoWorld, OnPointerMoved, Path, WicDecoderArm, World,
    load_shell_target, spawn_ghost_windows,
};

// ---------------------------------------------------------------------------
// Async Setup（UI スレッドで適用されるコマンド）
// ---------------------------------------------------------------------------

/// 起動セットアップコマンドを UI スレッドへ送る。
pub(super) async fn run_setup(tx: CommandSender) {
    if tx
        .send(Box::new(|world: &mut World| {
            build_and_spawn(world);
        }))
        .is_err()
    {
        tracing::error!(
            "collision-probe: 起動セットアップの送出に失敗（UI 側の受け口が閉じている）— 中止"
        );
    }
}

/// 配置準備（placement 本体）→ placeholder 誤寸 spawn（placement 本体）→ `OnPointerMoved` 装着 →
/// 装着アセット構築（probe 側）→ `ProbeBoot` 挿入を一括で行う（UI スレッド）。
///
/// 失敗は `error!` を出して中断する（受け入れ probe なので失敗を観測させる。終了は smoke env のみ）。
fn build_and_spawn(world: &mut World) {
    let ghost_root = emo2_root();
    let balloon_dir = balloon_root();

    let mut prepared = match areka_placement::prepare_ghost_windows(&ghost_root, &balloon_dir) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(
                error = %e,
                ghost_root = %ghost_root.display(),
                "collision-probe: 配置準備に失敗（真因は直前の error!）— 中止"
            );
            return;
        }
    };

    // bottom 吸着 resize が work area を引くために要る。
    world.insert_resource(areka_placement::follow::MonitorSnapshot::from_monitors(
        &wintf::ecs::window::monitor::enumerate_monitors(),
    ));

    // scope0 char を意図的に誤った placeholder 寸で spawn する（最終一致を本番 resize 経路のみで成立させる）。
    let Some(p0) = prepared.placements.iter_mut().find(|p| p.scope == 0) else {
        tracing::error!("collision-probe: scope0 の placement が無い（emo2 fixture 異常）— 中止");
        return;
    };
    tracing::info!(
        orig_w = p0.char_size.w,
        orig_h = p0.char_size.h,
        placeholder = PLACEHOLDER_SIZE,
        "collision-probe: scope0 char を placeholder 誤寸で spawn（本番 resize 経路のみで最終一致させる）"
    );
    p0.char_size = areka_placement::resolver::SizePx {
        w: PLACEHOLDER_SIZE,
        h: PLACEHOLDER_SIZE,
    };

    let windows = spawn_ghost_windows(world, &prepared.placements, &prepared.titles);
    let Some(char_window) = windows.char_window(0) else {
        tracing::error!("collision-probe: scope0 char 窓 entity が無い — 中止");
        return;
    };

    // ⑤ マウス経路照合の観測シーム。`OnPointerPressed` は付けない（冒頭 doc「使い方」参照・2026-08-05 実機で
    // ダブルクリックでは終了しないことを確認）。`attach_char_pointer_handlers` は `pub(crate)` かつ内部が
    // `crate::` パスを使うため example から使えず、終了は `AREKA_APP_SMOKE_EXIT_MS` の有界 auto-exit のみ。
    world
        .entity_mut(char_window)
        .insert(OnPointerMoved(on_probe_pointer_moved));

    let decoder = match WicDecoderArm::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "collision-probe: WicDecoderArm 生成に失敗（COM 未初期化？）— 中止（窓は配置済み）");
            return;
        }
    };
    let shell_dir = ghost_root.join("shell/master");
    let Some((emo_world, atlas)) = load_shell_assets(&shell_dir, &decoder) else {
        tracing::error!(
            "collision-probe: shell アセット構築に失敗（真因は直前の error!）— 中止（窓は配置済み）"
        );
        return;
    };

    world.insert_non_send(ProbeBoot {
        presenter: EmoPresenter::new(),
        char_window,
        author_dpi: prepared.author_dpi.shell,
        assets: Some((emo_world, atlas)),
        phase: ProbePhase::WaitingAttach,
        native_size: (0, 0),
        physical_size: (0, 0),
    });
    tracing::info!(
        "collision-probe: 窓生成・アセット構築完了（GPU 資源到達で surface1000 表示→本番 resize→物理寸整合 assert）"
    );
}

/// shell dir から scope0 装着用の `(EmoWorld, AtlasTable)` を、シェル読み込みの権威（`load_shell_target`）で
/// 構築する。失敗時は `None`（失敗の `error!` は権威が出す）。
///
/// 読取・解析・面の画像の決定・焼くまでと `charset` 宣言の扱いは権威側が持ち、この probe は呼ぶだけである。
/// 本番（`build_boot_assets_with_shell`）・採寸（`build_shell_assets`）と同じ入口を通るので、probe が見る絵は
/// 本番と食い違わない。
fn load_shell_assets(shell_dir: &Path, decoder: &WicDecoderArm) -> Option<(EmoWorld, AtlasTable)> {
    let target = load_shell_target(shell_dir, decoder).ok()?;
    let atlas = target.atlas().clone();
    Some((target.build_world(), atlas))
}

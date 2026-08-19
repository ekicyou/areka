use super::fixture::{PLACEHOLDER_SIZE, balloon_root, emo2_root};
use super::pointer::on_probe_pointer_moved;
use super::state::{ProbeBoot, ProbePhase};
use super::{
    AlphaParams, AtlasTable, CommandSender, EmoPresenter, EmoWorld, OnPointerMoved, PackConfig,
    Path, SetId, SurfaceSet, UseSelfAlpha, WicDecoderArm, World, bake, placement,
    spawn_ghost_windows,
};

// ---------------------------------------------------------------------------
// Async Setup（UI スレッドで適用されるコマンド）
// ---------------------------------------------------------------------------

/// 起動セットアップコマンドを UI スレッドへ送る（donor `run_setup` と同型）。
pub(super) async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        build_and_spawn(world);
    }));
}

/// 配置準備（placement 本体）→ placeholder 誤寸 spawn（placement 本体）→ `OnPointerMoved` 装着 →
/// 装着アセット構築（probe 側）→ `ProbeBoot` 挿入を一括で行う（UI スレッド）。
///
/// 失敗は log-first（`error!`）で中断する（受け入れ probe ゆえダミー窓フォールバックは持たない——失敗を
/// loud に観測させる。終了は Ctrl+C／smoke env）。
fn build_and_spawn(world: &mut World) {
    let ghost_root = emo2_root();
    let balloon_dir = balloon_root();

    // placement 本体の準備パイプライン（load→config→measure→primary work area→resolve・実モニタ列挙）。
    let mut prepared = match placement::prepare_ghost_windows(&ghost_root, &balloon_dir) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(
                error = %e,
                ghost_root = %ghost_root.display(),
                "collision-probe: 配置準備に失敗（実モニタ列挙が要る＝実 DPI モニタ上で実行すること）— 中止"
            );
            return;
        }
    };

    // MonitorSnapshot（bottom 吸着 resize が work area を引く消費用・`window-placement.rs:259-261` 前例）。
    world.insert_resource(placement::follow::MonitorSnapshot::from_monitors(
        &wintf::ecs::window::monitor::enumerate_monitors(),
    ));

    // donor 必須逸脱 #3: scope0 char を**意図的に誤った placeholder 寸**で spawn する（最終一致を本番
    // resize 経路のみで成立させる構図保証）。
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
    p0.char_size = placement::resolver::SizePx {
        w: PLACEHOLDER_SIZE,
        h: PLACEHOLDER_SIZE,
    };

    // placement 本体の窓組立（markers・WindowPos・Anchored 無条件付与・DragConfig・double-click close）。
    let windows = spawn_ghost_windows(world, &prepared.placements, &prepared.titles);
    let Some(char_window) = windows.char_window(0) else {
        tracing::error!("collision-probe: scope0 char 窓 entity が無い — 中止");
        return;
    };

    // ⑤ マウス経路照合の観測シーム: probe 窓へ `OnPointerMoved` を1行装着（donor `OnPointerPressed`
    // 前例＝`emo-present.rs:523` と同型・placement 非改変）。
    //
    // **本 probe に `OnPointerPressed` は付かない**（2026-08-05 実機で確認・冒頭 doc「使い方」参照）。
    // かつて終了を担っていた stand-in `spawn_ghost_windows` の `OnPointerPressed(on_ghost_pressed)` は
    // `areka-P0-input-events` task 2.7 で退役し（`placement/spawn.rs` の当該コメント）、正典の脱出口
    // （Ctrl+左ダブルクリック・DD-IE-7）は `input_events::attach_char_pointer_handlers` へ移った。
    // 同関数は `pub(crate)` かつ内部が `crate::` パスを使うため example から `#[path]` include できず、
    // 本 probe の終了は `AREKA_APP_SMOKE_EXIT_MS` の有界 auto-exit のみである。
    world
        .entity_mut(char_window)
        .insert(OnPointerMoved(on_probe_pointer_moved));

    // shell アセット構築（emo-present donor と同経路: parse→bake→EmoWorld::build→bind_atlas）。
    let decoder = match WicDecoderArm::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = ?e, "collision-probe: WicDecoderArm 生成に失敗（COM 未初期化？）— 中止（窓は配置済み）");
            return;
        }
    };
    let shell_dir = ghost_root.join("shell/master");
    let Some((emo_world, atlas)) = build_shell_target(&shell_dir, &decoder) else {
        tracing::error!("collision-probe: shell アセット構築に失敗 — 中止（窓は配置済み）");
        return;
    };

    world.insert_non_send(ProbeBoot {
        presenter: EmoPresenter::new(),
        char_window,
        assets: Some((emo_world, atlas)),
        phase: ProbePhase::WaitingAttach,
        native_size: (0, 0),
        physical_size: (0, 0),
    });
    tracing::info!(
        "collision-probe: 窓生成・アセット構築完了（GPU 資源到達で surface1000 表示→本番 resize→物理寸整合 assert）"
    );
}

/// shell dir の surfaces.txt から scope0 装着用の `(EmoWorld, AtlasTable)` を構築する（emo-present donor
/// `build_shell_target` と同経路）。失敗時は log-first で `None`。
fn build_shell_target(shell_dir: &Path, decoder: &WicDecoderArm) -> Option<(EmoWorld, AtlasTable)> {
    let surfaces_txt = shell_dir.join("surfaces.txt");
    let content = match std::fs::read_to_string(&surfaces_txt) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                path = %surfaces_txt.display(),
                error = %e,
                "collision-probe: shell surfaces.txt の読取に失敗"
            );
            return None;
        }
    };
    let shell = areka_parsers::shell::parse(&content);
    if shell.surfaces.is_empty() {
        tracing::error!("collision-probe: surfaces.txt が surface を 1 つも産まなかった");
        return None;
    }

    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: shell_dir,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], decoder, PackConfig::default());
    // emo2 shell は α 無し `purple/a/null.png` 1 枚が normalize seam として脱落する（既知・許容）。
    for err in &baked.errors {
        tracing::warn!(error = %err, "collision-probe: shell bake で脱落した element（surface1000 表示には無害）");
    }

    let mut emo_world = EmoWorld::build(&shell);
    emo_world.bind_atlas(&baked.table, SetId(0));
    Some((emo_world, baked.table))
}

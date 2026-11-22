//! 起動窓と起動の直後の結線のシーム。

use bevy_ecs::prelude::World;

/// 復元マージのシーム。
///
/// `ghost_session::prepare_ghost_windows` が窓を作る閉包へ渡す placements を、起動時に先読みした
/// 永続 entries で差し替える（保存位置優先・毎起動 live 再射影）。`prepare_ghost_windows` は
/// 実モニタの列挙と作業プール（`WintfTaskPool`）を要してテスト困難なので、復元の流れ
/// （`load_restored_state` → `restore_and_judge` → `apply_balloon_limit` → `restored_scope_marks`）を
/// 本ヘルパへ抽出して単体で檻に入れる（IO は [`areka_placement::persist::load_restored_state`] の
/// 1 点のみ・残りは純関数）。
pub fn restore_merged_placements(
    ghost_root: &std::path::Path,
    placements: Vec<areka_placement::resolver::ScopePlacement>,
    snapshot: &areka_placement::follow::MonitorSnapshot,
) -> (
    Vec<areka_placement::resolver::ScopePlacement>,
    std::collections::BTreeMap<usize, Option<areka_placement::chain_finalize::AdjacencyMark>>,
    bool,
) {
    // 唯一の IO 点: ゴーストの descript を読めるかの確かめ → Ghost スコープの記憶の先読み。
    let entries = areka_placement::persist::load_restored_state(ghost_root);
    // 純関数 merge（永続不書込・保存位置優先 → project_restore → balloon 導出）と、記憶の位置を
    // 寄せて表示したかの判定（窓を作る閉包が `RestoredPositionsClamped` へ据える）。
    let (merged, clamped) =
        areka_placement::persist::restore_and_judge(placements, &entries, snapshot);
    // 起動時関門: 経路①（spawn 初期値）と経路②（復元 merge）はどちらもこの合流点の出力を消費するため、
    // ここ 1 点で両方が被覆される。merge **後**に置くのは保存値優先の合流規則を変えないためで、補正は
    // `balloon_pos`（表示位置）だけに作用する——`balloon_offset`（論理相対位置）は生値のまま
    // （補正を焼き付けない）。
    let merged = areka_placement::balloon_limit::apply_balloon_limit(merged, snapshot);
    // 記憶に位置（x・y の両方）があるスコープ集合。起動の並べ直しは覚えた印で、拡大率の変化の後は
    // 変わる直前の隣り合いで決める。値の違いでなく記憶の有無で決める＝記憶の値がたまたま既定と同じでも
    // 記憶として扱う（判定は merge と同じ読み取り `has_saved_char_pos`）。値は覚えた隣り合いの印
    // （無ければ None）で、窓を作る閉包が窓の一式へ据える。
    let restored = areka_placement::persist::restored_scope_marks(&entries, &merged);
    (merged, restored, clamped)
}

/// `PersistWiring`（NonSend）を、ゴースト窓を保持する同一 World へ挿入するシーム。
///
/// wired 経路（実 sink 結線成立）と fallback boot 経路（`LogSink` 1 本と `DiscardSink` 1 本）の**両方**が、生きた ghost
/// runtime が存在するときにこのヘルパで `runtime.sylphya_publisher().clone()` を World の NonSend
/// リソースとして挿入する。以降、follow.rs の DragEnd 観測点が
/// [`areka_placement::persist::persist_entries`] 経由でこの publisher の clone 送信端から保存 entries を
/// write-through 投函できる。
///
/// 生きた runtime が無い経路（wired の `None` ghost・fallback の `Err`）では挿入しない＝永続結線なし
/// （`persist_entries` は `PersistWiring` 不在で debug!＋no-op へ縮退）。挿入は純粋な World 変異ゆえ
/// headless 単体テスト可能。
pub fn insert_persist_wiring(world: &mut World, publisher: areka_sylphya::SylphyaPublisher) {
    world.insert_non_send(areka_placement::persist::PersistWiring { publisher });
}

/// boot が `Ok` を返した直後の結線（wired／fallback の両アームが呼ぶ 1 か所）。
///
/// 位置永続の導管を挿入し（[`insert_persist_wiring`]）、続けて起動成功時の記憶を書く
/// （[`record_last_used_at_boot`]）。
pub(crate) fn on_boot_ok(
    world: &mut World,
    runtime: &areka_ghost::GhostRuntime,
    ghost: &crate::boot_resolve::GhostDecision,
    balloon: &crate::boot_resolve::BalloonDecision,
) {
    insert_persist_wiring(world, runtime.sylphya_publisher().clone());
    record_last_used_at_boot(
        runtime.sylphya_publisher(),
        &runtime.mount().shell.dir,
        ghost,
        balloon,
    );
}

/// 起動の呼び出しが返った時点の記憶（初回の起動・既定への戻しはここで書く）。
///
/// 経路が切替なら書かない: 壊れた切替先の名前を最後のゴーストに一瞬も載せないため、切替先の記憶は
/// 定常到達で `ghost_switch::record_steady_memory` が同じ [`record_last_used`] で書く。
pub(crate) fn record_last_used_at_boot(
    publisher: &areka_sylphya::SylphyaPublisher,
    shell_dir: &std::path::Path,
    ghost: &crate::boot_resolve::GhostDecision,
    balloon: &crate::boot_resolve::BalloonDecision,
) {
    use crate::boot_resolve;
    if ghost.route == boot_resolve::GhostRoute::Switched {
        tracing::debug!(
            event = "last_used_deferred",
            ghost = ?ghost.folder,
            "[boot_seam] 切替で起こしたゴーストの記憶は定常到達まで書きません"
        );
        return;
    }
    record_last_used(publisher, shell_dir, ghost, balloon);
}

/// 最後に使ったもの（ゴースト・バルーン・シェル）を、動いているゴーストの記憶の書き手へ投函する。
/// 書き込みは投函だけで待たない（反映は `shutdown` の barrier に任せる）。シェルのフォルダ名は
/// `shell_dir`（`mount().shell.dir`）の末尾（非 UTF-8 は `to_string_lossy` で写す）。呼び手は起動の直後と
/// 切替の定常到達（`ghost_switch::record_steady_memory`）。
pub(crate) fn record_last_used(
    publisher: &areka_sylphya::SylphyaPublisher,
    shell_dir: &std::path::Path,
    ghost: &crate::boot_resolve::GhostDecision,
    balloon: &crate::boot_resolve::BalloonDecision,
) {
    use crate::boot_resolve;
    let Some(shell_folder) = shell_dir.file_name().map(|n| n.to_string_lossy()) else {
        // `<ゴースト>/shell/<名>` の形で末尾が無いことは起きない。来たら書かずに残す。
        tracing::error!(
            event = "last_used_shell_folder_missing",
            shell_dir = %shell_dir.display(),
            "[boot_seam] シェルのフォルダ名が取れないので記憶を書きません"
        );
        return;
    };
    boot_resolve::LastUsed {
        ghost,
        balloon,
        shell_folder: &shell_folder,
    }
    .record(publisher);
}

/// 起動時モニタスナップショットの構築＋出力シーム。
///
/// [`areka_placement::follow::MonitorSnapshot`] は **placement の全判断が読む権威**（work area 解決・
/// アンカー射影・可視性判定がすべてこの Resource を引く）なので、正典の出力点はこの構築点にする。
/// ここで観測した値だけが「以後の判断が実際に見た値」であり、他所で列挙し直した値ではない。
///
/// 出力は共有ヘルパ [`areka_placement::diag::log_monitor_snapshot`] 1 本で、呼出点タグ
/// [`areka_placement::MONITOR_SNAPSHOT_CONTEXT`] を名乗る。`prepare_ghost_windows` の列挙点も
/// 同じヘルパを別タグで呼ぶため、語彙は共有したままログ上で出所を弁別でき、両者の食い違いは
/// grep 突合で検出できる。
///
/// 観測を足すだけで snapshot の中身は変えない。実モニタを列挙する `ghost_session::prepare_ghost_windows`
/// から切り出したシームゆえ、合成モニタで headless 檻に入る。
///
/// # 2 源を同じ構築関数で作る
///
/// 返すのは作業領域源と**モニタ別拡大率表**の組である。実行時の同期段
/// （`areka_runtime::frame::work_area_sync`）も同じ [`areka_placement::follow::MonitorSources::from_monitors`]
/// を通る——起動時だけが別の作り方をすると、同期が入った後も起動時の値だけが違う形になり得る。
pub(crate) fn boot_monitor_snapshot(
    monitors: &[wintf::ecs::window::monitor::Monitor],
) -> areka_placement::follow::MonitorSources {
    areka_placement::diag::log_monitor_snapshot(
        &areka_placement::monitor_records(monitors),
        areka_placement::MONITOR_SNAPSHOT_CONTEXT,
    );
    areka_placement::follow::MonitorSources::from_monitors(monitors)
}

/// 復元マージシームの headless 単体テスト。
///
/// `restore_merged_placements`（`load_restored_state` → `restore_and_judge` → `apply_balloon_limit` →
/// `restored_scope_marks`）を檻に入れ、
/// 植えた sylphya.toml の保存位置が既定位置に優先して merge 済み placements の char_pos へ載ること／
/// 永続不在なら既定 placement に恒等であることを証明する（＝spawn される窓の初期位置が保存位置に
/// なる結線の証明）。
#[cfg(test)]
#[path = "boot_seam_restore_tests.rs"]
mod restore_seam_tests;

/// `PersistWiring` 挿入シームの headless 単体テスト。
///
/// wired／fallback 両経路が使う挿入ヘルパ `insert_persist_wiring` を檻に入れる。実 publisher
/// （`spawn_sylphya`＋共有 fake IO）を headless World へ挿入し、(a) NonSend リソース `PersistWiring` が
/// 存在すること、(b) その World 越しの `persist_entries` 投函が barrier 後に別ハンドルの `load_scope` で
/// 読み戻せる（＝World レベルの配線導管が正しく確立されている）ことを証明する。
#[cfg(test)]
#[path = "boot_seam_persist_wiring_tests.rs"]
mod persist_wiring_seam_tests;

/// 起動時モニタスナップショット出力シームの headless 単体テスト。
///
/// [`boot_monitor_snapshot`] を合成モニタ（混在 DPI・負座標・3200 超）で檻に入れる。
/// 実モニタも実 GPU も要さない。
#[cfg(test)]
#[path = "boot_seam_monitor_snapshot_tests.rs"]
mod monitor_snapshot_seam_tests;

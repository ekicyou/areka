use super::{
    BalloonWindowMarker, DoubleClick, Entity, Or, Phase, PointerState, ShellWindowMarker, With,
    World,
};

// ---------------------------------------------------------------------------
// Event Handlers
// ---------------------------------------------------------------------------

/// OnPointerPressed ハンドラ: 不透明域クリックの捕捉ログ（task 5.2）＋ダブルクリック（左）終了。
///
/// **不透明域クリック捕捉の観測シーム（task 5.2・R2.2/R2.3/R6.3）**: クリック透過機構は αマスク
/// （`AlphaMask::is_hit`）に従って `WS_EX_TRANSPARENT` を動的トグルするため、pointer-pressed
/// イベントが本窓へ到達したこと自体が「クリックが不透明（αマスク有効）域へ着地した」証拠である
/// （透明域のクリックは背後プロセスへ透過し本ハンドラには**到達しない**）。ゆえに毎押下（単クリック
/// 含む）で 1 行だけ `info!` を出し捕捉を記録する（不在＝透明域透過の観測）。
///
/// despawn → `on_window_handle_remove` → `PostMessage(WM_CLOSE)` → `WindowRegistry` 空遷移 →
/// `run()` 復帰、という wintf の作法に委ねる。
pub(super) fn on_shell_pressed(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(state) => {
            // 不透明域クリック捕捉ログ（task 5.2 の観測シーム・毎押下 1 行）。到達＝不透明域着地の証拠。
            tracing::info!(
                client_x = state.client_point.x,
                client_y = state.client_point.y,
                local_x = state.local_point.x,
                local_y = state.local_point.y,
                "emo-present: 不透明域クリックを捕捉（target=shell・αマスク有効域に着地＝透明域は背後へ透過し不到達）"
            );
            if state.double_click == DoubleClick::Left {
                tracing::info!("emo-present: ダブルクリック検出 — 全窓を閉じて終了します");
                let windows: Vec<Entity> = world
                    .query_filtered::<Entity, Or<(With<ShellWindowMarker>, With<BalloonWindowMarker>)>>()
                    .iter(world)
                    .collect();
                for e in windows {
                    world.despawn(e);
                }
                return true;
            }
            false
        }
    }
}

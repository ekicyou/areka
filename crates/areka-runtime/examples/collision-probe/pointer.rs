use super::state::ProbeBoot;
use super::{
    Entity, GetCursorPos, POINT, Phase, PointerState, ScreenToClient, WindowHandle, World,
    resolve_hit_region,
};

// ---------------------------------------------------------------------------
// Pointer handler（⑤ マウス経路照合＋解決 live ログ）
// ---------------------------------------------------------------------------

/// probe 窓の `OnPointerMoved` ハンドラ（⑤ マウス経路照合・`emo-present/input.rs` の `on_shell_pressed` と同じ形）。
///
/// 記録行ごとに **本番マウス経路** `PointerState.client_point` と **クエリ系** `ScreenToClient(GetCursorPos())`
/// をペア列（client_x/y・s2c_x/y・Δx/Δy）で live ログし、Δ=(0,0) の厳密一致を人手検証させる。加えて
/// s2c で得た**目視由来**の client 点を [`resolve_hit_region`] へ渡し解決結果（Head/Bust/None）を併記する。
///
/// **反トートロジー**: 狙点は `GetCursorPos`（実カーソル＝目視由来）からのみ得る。collision 実値
/// から合成した screen 座標の `SetCursorPos`/`SendInput` 注入は**行わない**（本ハンドラも一切呼ばない）。
///
/// ペア列は不透明域（Head/Bust）行のみに出る——透明域はクリック透過でイベントが窓へ届かず、本ハンドラが
/// 呼ばれない（欠測が正しい挙動）。
pub(super) fn on_probe_pointer_moved(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble 相のみ処理する。伝播は止めない。
    let Phase::Bubble(state) = ev else {
        return false;
    };
    // 本番マウス経路（WM_MOUSEMOVE lparam 直系）の client 物理 px。
    let client = state.client_point;

    // ハンドラ実行時は ProbeBoot が insert 済み（Input schedule と FrameFinalize は同一 tick 内で
    // 直列ゆえ remove 中と重ならない）。
    let Some(boot) = world.get_non_send::<ProbeBoot>() else {
        return false;
    };
    let char_window = boot.char_window;

    let Some(handle) = world.get::<WindowHandle>(char_window) else {
        return false;
    };
    let hwnd = handle.hwnd;

    let mut pt = POINT::default();
    // SAFETY: Win32 境界。GetCursorPos は現在のカーソル位置を pt へ書き込むだけ（副作用なし）。
    if unsafe { GetCursorPos(&mut pt) }.is_err() {
        tracing::warn!("collision-probe: GetCursorPos 失敗 — この行のペア列/解決をスキップ");
        return false;
    }
    // SAFETY: Win32 境界。ScreenToClient は hwnd と POINT への可変ポインタを要し、pt を client へ写す。
    if !unsafe { ScreenToClient(hwnd, &mut pt).as_bool() } {
        tracing::warn!("collision-probe: ScreenToClient 失敗 — この行のペア列/解決をスキップ");
        return false;
    }
    let (s2c_x, s2c_y) = (pt.x, pt.y);

    // 両者とも同一 HWND の client 物理 px 整数で、丸め誤差の正当源が無い（要求は Δ=(0,0) 厳密一致）。
    let dx = client.x - s2c_x;
    let dy = client.y - s2c_y;

    // 解決は目視由来の s2c 点（GetCursorPos 経由）で行う（反トートロジー）。
    let hit = resolve_hit_region(world, &boot.presenter, 0, s2c_x as i64, s2c_y as i64);
    // 縮約後サーフェス px は presenter の値をそのまま載せる（二重縮約を避けるため probe 側で ÷k しない）。
    let (surface_x, surface_y) = hit.surface_point;
    let region = hit.region.as_deref().unwrap_or("None");

    // `client=` は目視由来の狙点（s2c 経路）、`surface=` はその点を presenter が ÷k 縮約した SHIORI 配信空間の値。
    tracing::info!(
        client_x = client.x,
        client_y = client.y,
        s2c_x,
        s2c_y,
        dx,
        dy,
        surface_x,
        surface_y,
        region,
        "collision-probe: client=({s2c_x},{s2c_y}) surface=({surface_x},{surface_y}) region={region} ⑤ マウス経路ペア列（client_point=({},{})・Δ=({dx},{dy}) 厳密一致が要求・ペア列は不透明域行のみ）",
        client.x,
        client.y
    );
    false
}

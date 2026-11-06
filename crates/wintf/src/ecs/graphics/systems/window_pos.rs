use super::init::format_entity_name;
use crate::ecs::drag::WindowDragging;
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::ecs::graphics::{
    GraphicsCommandList, GraphicsCore, SurfaceGraphics, VisualGraphics, WindowGraphics,
};
use crate::ecs::widget::bitmap_source::BitmapSourceGraphics;
use crate::ecs::window::transition_diag::{ORIGIN_WINDOW_POS, WriteTag};
use crate::ecs::window::{SetWindowPosCommand, Window, WindowHandle, WindowPos};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, trace, warn};
use windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE;

/// WindowPos変更時にSetWindowPosコマンドをキューに追加
///
/// クライアント領域座標をウィンドウ全体座標に変換してからコマンドを生成する。
/// echo 時は `WM_WINDOWPOSCHANGED` ハンドラが `bypass_change_detection()` で更新するため
/// `Changed<WindowPos>` が発火せず、本システムのトリガー自体が発火しない。
///
/// ドラッグ中のウィンドウ（`WindowDragging` 付き）はサイズ変更のみ発行し、
/// `SWP_NOMOVE` フラグで位置を固定する。これにより DPI 変更時の
/// ウィンドウリサイズがドラッグ中でも正しく反映される。
pub fn apply_window_pos_changes(
    query: Query<
        (
            Entity,
            &WindowHandle,
            &WindowPos,
            Option<&Name>,
            Has<WindowDragging>,
        ),
        (Changed<WindowPos>, With<Window>),
    >,
) {
    for (entity, window_handle, window_pos, name, is_dragging) in query.iter() {
        // 名前はその記録が点いているときだけ作る（窓が動くたびに通る道）
        let entity_name = || format_entity_name(entity, name);

        let position = window_pos.position.unwrap_or_default();
        let size = window_pos.size.unwrap_or_default();

        // CW_USEDEFAULTが設定されている場合はスキップ（ウィンドウ作成時の初期値）
        // 座標変換をスキップし、ウィンドウ作成時の初期配置を優先
        if position.x == windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT
            || size.width == windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT
        {
            trace!(
                entity = %entity_name(),
                x = position.x,
                y = position.y,
                width = size.width,
                height = size.height,
                "[apply_window_pos_changes] CW_USEDEFAULT detected, skipping"
            );
            continue;
        }

        // クライアント領域座標をウィンドウ全体座標に変換
        let (x, y, width, height) = match window_pos.to_window_coords(window_handle) {
            Ok(coords) => coords,
            Err(e) => {
                // 変換失敗時はフォールバック：元の座標を使用
                warn!(
                    entity = %entity_name(),
                    error = %e,
                    "[apply_window_pos_changes] Failed to transform window coordinates. Using original values."
                );
                (position.x, position.y, size.width, size.height)
            }
        };

        // SetWindowPosコマンドを生成してキューに追加
        // 直接SetWindowPosを呼び出さない（World借用競合防止）
        let mut flags = window_pos.build_flags_for_system();
        let hwnd_insert_after = window_pos.get_hwnd_insert_after();

        // ドラッグ中は SWP_NOMOVE を強制: サイズのみ変更
        if is_dragging {
            flags |= SWP_NOMOVE;
        }

        debug!(
            entity = %entity_name(),
            client_xy = format_args!("({},{})", position.x, position.y),
            client_size = format_args!("{}x{}", size.width, size.height),
            win_xy = format_args!("({},{})", x, y),
            win_size = format_args!("{}x{}", width, height),
            is_dragging = is_dragging,
            "[apply_window_pos] Enqueue SetWindowPos"
        );

        let cmd = SetWindowPosCommand::new(
            window_handle.hwnd,
            x,
            y,
            width,
            height,
            flags,
            hwnd_insert_after,
        )
        // 観測専用の札。積み上げた経路を書込レコードの 1 行で見分けるためだけのもので、
        // 座標変換・フラグ・スキップ判定のいずれも変わらない。スコープと窓種別は
        // 表示基盤側では判らないため番兵のままにする。
        .with_tag(WriteTag {
            origin: ORIGIN_WINDOW_POS,
            ..WriteTag::UNTAGGED
        });
        SetWindowPosCommand::enqueue(cmd);

        debug!(
            entity = %entity_name(),
            x = position.x,
            y = position.y,
            width = size.width,
            height = size.height,
            "[apply_window_pos_changes] Command enqueued"
        );
    }
}

/// 依存コンポーネント無効化（`GraphicsSetup` の先頭・装置を作る係より前）
///
/// 装置が在って無効なとき、有効な窓・見た目・描画面・画像の部品を倒し、命令の列
/// （古い装置に属する）を全部外し、最後に WUC の資源を倒す（合成の木の部品を先に捨て、
/// 合成器を最後に捨てる）。可変で触るだけで変化の印が立つので、有効なものだけを触る。
/// 装置が無効な回には毎回走るが、倒れたものには触らないので同じ印を何度も立てない。
#[allow(clippy::too_many_arguments)]
pub fn invalidate_dependent_components(
    graphics: Option<Res<GraphicsCore>>,
    wuc_resource: Option<ResMut<WucGraphicsResource>>,
    mut windows: Query<&mut WindowGraphics>,
    mut visuals: Query<&mut VisualGraphics>,
    mut surfaces: Query<&mut SurfaceGraphics>,
    mut bitmaps: Query<&mut BitmapSourceGraphics>,
    command_lists: Query<Entity, With<GraphicsCommandList>>,
    mut commands: Commands,
) {
    if !graphics.is_some_and(|gc| !gc.is_valid()) {
        return;
    }
    let mut count = 0usize;
    for mut wg in windows.iter_mut().filter(|c| c.is_valid()) {
        wg.invalidate();
        count += 1;
    }
    for mut vg in visuals.iter_mut().filter(|c| c.is_valid()) {
        vg.invalidate();
        count += 1;
    }
    for mut sg in surfaces.iter_mut().filter(|c| c.is_valid()) {
        sg.clear();
        count += 1;
    }
    for mut bsg in bitmaps.iter_mut().filter(|c| c.is_valid()) {
        bsg.invalidate();
        count += 1;
    }
    for entity in command_lists.iter() {
        commands.entity(entity).remove::<GraphicsCommandList>();
        count += 1;
    }
    if let Some(mut wgr) = wuc_resource.filter(|r| r.is_valid()) {
        wgr.invalidate();
        count += 1;
    }
    if count > 0 {
        warn!(
            count,
            "[invalidate_dependent_components] GraphicsCore invalid - invalidated dependent components"
        );
    }
}

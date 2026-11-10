//! 描く骨組み: 共有の描く文脈で命令の列を 1 本記録し、その結果を部品へ入れる。
//!
//! 四角・文字・打ち出し表示（文と背景）・画像の描く係が、同じ手順をここで通す。

use crate::com::d2d::{D2D1CommandListExt, D2D1DeviceContextExt};
use crate::ecs::graphics::{GraphicsCommandList, format_entity_name};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::warn;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{ID2D1CommandList, ID2D1DeviceContext};
use windows_numerics::Matrix3x2;

/// 透明の色（四角・文字・打ち出し表示が使っていた 3 つの写しを 1 つに）。
pub(crate) const TRANSPARENT: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

/// 共有の描く文脈で命令の列を 1 本記録する。
///
/// 作る → 的にする → 変換を戻す → `BeginDraw` → 透明で消す → `draw` → `EndDraw` → 閉じる。
/// `BeginDraw` の後は `draw` が失敗しても必ず `EndDraw` を呼ぶ（共有の文脈を描きかけで残さない）。
/// 返す失敗は `draw` → `EndDraw` → 閉じるの順に先のもの。
pub(crate) fn record_command_list(
    dc: &ID2D1DeviceContext,
    draw: impl FnOnce(&ID2D1DeviceContext) -> windows::core::Result<()>,
) -> windows::core::Result<ID2D1CommandList> {
    let command_list = unsafe { dc.CreateCommandList() }?;
    let ended = unsafe {
        dc.SetTarget(&command_list);
        // 前の回の描く係が残した部品の位置の変換を消す
        dc.SetTransform(&Matrix3x2::identity());
        dc.BeginDraw();
        dc.clear(Some(&TRANSPARENT));
        let drawn = draw(dc);
        let ended = dc.EndDraw(None, None);
        drawn.and(ended)
    };
    ended?;
    command_list.close()?;
    Ok(command_list)
}

/// 記録の結果を部品へ入れる。
///
/// 成功なら命令の列を入れる。失敗なら `warn!` を 1 行残し、命令の列が無かった部品（`had_list` が偽）
/// にだけ空の列を入れる。空の列が入った部品は `Without<GraphicsCommandList>` から外れ、次の変化か
/// 立ち直り（命令の列が外される）まで描き直さない。前から列を持つ部品は、一時の失敗で絵を消さないため
/// 今の列を残す。
pub(crate) fn store_command_list(
    commands: &mut Commands,
    entity: Entity,
    name: Option<&Name>,
    had_list: bool,
    recorded: windows::core::Result<ID2D1CommandList>,
    what: &'static str,
) {
    match recorded {
        Ok(command_list) => {
            commands
                .entity(entity)
                .insert(GraphicsCommandList::new(command_list));
        }
        Err(err) => {
            // 係には返す先が無いので記録だけにする（名前の文字列は失敗のときだけ作る）
            warn!(
                entity = %format_entity_name(entity, name),
                error = ?err,
                what,
                "Failed to record CommandList"
            );
            if !had_list {
                commands.entity(entity).insert(GraphicsCommandList::empty());
            }
        }
    }
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod record_tests;

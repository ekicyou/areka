use crate::com::d2d::D2D1DeviceContextExt;
use crate::com::dwrite::DWriteFactoryExt;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore, format_entity_name};
use crate::ecs::widget::brushes::Brushes;
use crate::ecs::widget::record::{record_command_list, store_command_list};
use crate::ecs::widget::text::Label;
use crate::ecs::widget::text::label::apply_direction;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE;
use windows::Win32::Graphics::DirectWrite::*;
use windows_numerics::Vector2;

/// Labelコンポーネントから GraphicsCommandList を生成
///
/// Changed<Label>、Changed<Brushes> または GraphicsCommandList がないエンティティを対象に、
/// DirectWrite を使用してテキストレイアウトを生成し、
/// Direct2D の CommandList に描画命令を記録する。
/// 色は`Brushes::foreground_color`から取得されます。
pub fn draw_labels(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &Label,
            &Brushes,
            Option<&GraphicsCommandList>,
            Option<&Name>,
        ),
        // 命令の列が無い（立ち直りで外された）ときも描き直す
        Or<(
            Changed<Label>,
            Changed<Brushes>,
            Without<GraphicsCommandList>,
        )>,
    >,
    graphics_core: Option<Res<GraphicsCore>>,
) {
    let Some(graphics_core) = graphics_core else {
        warn!("GraphicsCore not available, skipping draw_labels");
        return;
    };

    let Some(dwrite_factory) = graphics_core.dwrite_factory() else {
        warn!("DirectWrite factory not available");
        return;
    };

    // グローバル共有DeviceContextを取得
    let Some(dc) = graphics_core.device_context() else {
        warn!("DeviceContext not available");
        return;
    };

    // 係には返す先が無いので、失敗は記録してその部品を飛ばす（名前の文字列は失敗のときだけ作る）
    for (entity, label, brushes, cmd_list_opt, name) in query.iter() {
        #[cfg(debug_assertions)]
        debug!(
            entity = ?entity,
            text = %label.text,
            font = %label.font_family,
            size_pt = label.font_size,
            "Drawing label"
        );

        let color = brushes.foreground_color();

        // TextFormat作成
        let font_family_hstring = windows::core::HSTRING::from(&label.font_family);
        let locale_hstring = windows::core::HSTRING::from("ja-JP");
        let text_format = match dwrite_factory.create_text_format(
            &font_family_hstring,
            None::<&IDWriteFontCollection>,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            label.font_size,
            &locale_hstring,
        ) {
            Ok(fmt) => fmt,
            Err(err) => {
                warn!(
                    entity = %format_entity_name(entity, name),
                    error = ?err,
                    "Failed to create TextFormat"
                );
                continue;
            }
        };

        // 縦書き・横書きの設定
        if let Err(err) = apply_direction(&text_format, label.direction) {
            warn!(
                entity = %format_entity_name(entity, name),
                direction = ?label.direction,
                error = ?err,
                "Failed to set text direction"
            );
            continue;
        }

        // TextLayout作成
        // maxWidth/maxHeightを0にしてNO_WRAPを指定することで、
        // コンテンツサイズに合わせてレイアウトさせる。
        // RTLの場合、maxWidthが無限大だと右端が無限遠になってしまうため。
        let text_hstring = windows::core::HSTRING::from(&label.text);
        let text_layout =
            match dwrite_factory.create_text_layout(&text_hstring, &text_format, 0.0, 0.0) {
                Ok(layout) => layout,
                Err(err) => {
                    warn!(
                        entity = %format_entity_name(entity, name),
                        error = ?err,
                        "Failed to create TextLayout"
                    );
                    continue;
                }
            };
        // 折り返しなしに設定
        if let Err(err) = unsafe { text_layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP) } {
            warn!(
                entity = %format_entity_name(entity, name),
                error = ?err,
                "Failed to set word wrapping"
            );
            continue;
        }

        // メトリクス取得（描画位置調整のために先に取得）。取れなければ原点 0 のまま描かない
        let mut metrics = DWRITE_TEXT_METRICS::default();
        if let Err(err) = unsafe { text_layout.GetMetrics(&mut metrics) } {
            warn!(
                entity = %format_entity_name(entity, name),
                error = ?err,
                "Failed to get text metrics"
            );
            continue;
        }

        // テキスト描画
        // RTL（縦書き含む）の場合、レイアウト幅0でNO_WRAPにすると、
        // テキストは原点から左側（負のX方向）にはみ出して描画されるため、metrics.leftは負の値になる。
        // これを正の領域（0,0〜）に持ってくるため、origin.Xを -metrics.left だけずらす。
        // LTRの場合は metrics.left は通常0なので影響しない。
        let origin = Vector2 {
            X: -metrics.left,
            Y: -metrics.top, // topも念のため補正
        };
        let recorded = record_command_list(dc, |dc| {
            let brush = dc.create_solid_color_brush(&color, None)?;
            dc.draw_text_layout(origin, &text_layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
            Ok(())
        });
        store_command_list(
            &mut commands,
            entity,
            name,
            cmd_list_opt.is_some(),
            recorded,
            "label",
        );
    }
}

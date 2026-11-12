use crate::com::d2d::D2D1DeviceContextExt;
use crate::com::dwrite::DWriteFactoryExt;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
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
    if query.is_empty() {
        return;
    }

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

    for (entity, label, brushes, cmd_list_opt, name) in query.iter() {
        #[cfg(debug_assertions)]
        debug!(
            entity = ?entity,
            text = %label.text,
            font = %label.font_family,
            size_dip = label.font_size,
            "Drawing label"
        );

        let color = brushes.foreground_color();

        // 配置を作る所の失敗も記録の失敗と同じ道へ渡す（列の無い部品に空の列を入れ、毎回やり直さない）
        let recorded = label_layout(dwrite_factory, label).and_then(|(text_layout, origin)| {
            record_command_list(dc, |dc| {
                let brush = dc.create_solid_color_brush(&color, None)?;
                dc.draw_text_layout(origin, &text_layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
                Ok(())
            })
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

/// 文字の配置と描く原点を作る（書式 → 向き → 配置 → 折り返しなし → 寸法）。
fn label_layout(
    dwrite_factory: &IDWriteFactory2,
    label: &Label,
) -> windows::core::Result<(IDWriteTextLayout, Vector2)> {
    // TextFormat作成
    let font_family_hstring = windows::core::HSTRING::from(&label.font_family);
    let locale_hstring = windows::core::HSTRING::from("ja-JP");
    let text_format = dwrite_factory.create_text_format(
        &font_family_hstring,
        None::<&IDWriteFontCollection>,
        DWRITE_FONT_WEIGHT_NORMAL,
        DWRITE_FONT_STYLE_NORMAL,
        DWRITE_FONT_STRETCH_NORMAL,
        label.font_size,
        &locale_hstring,
    )?;

    // 縦書き・横書きの設定
    apply_direction(&text_format, label.direction)?;

    // TextLayout作成
    // maxWidth/maxHeightを0にしてNO_WRAPを指定することで、
    // コンテンツサイズに合わせてレイアウトさせる。
    // RTLの場合、maxWidthが無限大だと右端が無限遠になってしまうため。
    let text_hstring = windows::core::HSTRING::from(&label.text);
    let text_layout = dwrite_factory.create_text_layout(&text_hstring, &text_format, 0.0, 0.0)?;
    // 折り返しなしに設定
    unsafe { text_layout.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP) }?;

    // メトリクス取得（描画位置調整のために先に取得）。取れなければ原点 0 のまま描かない
    let mut metrics = DWRITE_TEXT_METRICS::default();
    unsafe { text_layout.GetMetrics(&mut metrics) }?;

    // RTL（縦書き含む）の場合、レイアウト幅0でNO_WRAPにすると、
    // テキストは原点から左側（負のX方向）にはみ出して描画されるため、metrics.leftは負の値になる。
    // これを正の領域（0,0〜）に持ってくるため、origin.Xを -metrics.left だけずらす。
    // LTRの場合は metrics.left は通常0なので影響しない。
    let origin = Vector2 {
        X: -metrics.left,
        Y: -metrics.top, // topも念のため補正
    };
    Ok((text_layout, origin))
}

//! Typewriter 描画系システム
//!
//! - update_typewriters: 毎フレームの状態更新（Update スケジュール）
//! - draw_typewriters: 描画コマンド生成（Draw スケジュール）
//! - draw_typewriter_backgrounds: 空トークの背景のみ描画（Draw スケジュール）

use crate::com::d2d::D2D1DeviceContextExt;
use crate::com::dwrite::DWriteTextLayoutExt;
use crate::ecs::graphics::{FrameTime, GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::Arrangement;
use crate::ecs::widget::brushes::Brushes;
use crate::ecs::widget::record::{TRANSPARENT, record_command_list, store_command_list};
use crate::ecs::widget::text::label::TextDirection;
use crate::ecs::widget::text::typewriter::{
    Typewriter, TypewriterLayoutCache, TypewriterState, TypewriterTalk,
};
use crate::ecs::widget::text::typewriter_ir::{TypewriterEvent, TypewriterToken};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::Win32::Graphics::Direct2D::{D2D1_DRAW_TEXT_OPTIONS_NONE, ID2D1DeviceContext};
use windows::Win32::Graphics::DirectWrite::*;
use windows_numerics::Vector2;

/// 見せるクラスタ数から、透明にする範囲（UTF-16 の開始と長さ）を返す。隠す所が無ければ None。
pub(crate) fn hidden_text_range(
    cluster_lengths: &[u16],
    visible_clusters: u32,
) -> Option<(u32, u32)> {
    let visible: u32 = cluster_lengths
        .iter()
        .take(visible_clusters as usize)
        .map(|&l| l as u32)
        .sum();
    let total: u32 = cluster_lengths.iter().map(|&l| l as u32).sum();
    (visible < total).then(|| (visible, total - visible))
}

/// 部品の大きさいっぱいに背景を塗る（文の係と背景の係が使う）。
fn fill_background(
    dc: &ID2D1DeviceContext,
    color: &D2D1_COLOR_F,
    arrangement: &Arrangement,
) -> windows::core::Result<()> {
    let brush = dc.create_solid_color_brush(color, None)?;
    dc.fill_rectangle(
        &D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: arrangement.size.width,
            bottom: arrangement.size.height,
        },
        &brush,
    );
    Ok(())
}

/// Typewriter 更新システム（Update スケジュール）
///
/// FrameTime リソースから現在フレームの時刻を取得し、TypewriterTalk の状態を更新する。
/// FireEvent トークンを処理して対象エンティティの TypewriterEvent を設定する。
/// 見せる数か状態が変わったときだけ変化の印を立てる（止まった表示を描き直させない）。
pub fn update_typewriters(
    mut commands: Commands,
    frame_time: Option<Res<FrameTime>>,
    mut query: Query<(Entity, &mut TypewriterTalk, &TypewriterLayoutCache)>,
) {
    let Some(frame_time) = frame_time else {
        return;
    };

    let current_time = frame_time.0;

    for (entity, mut talk, layout_cache) in query.iter_mut() {
        let before = (talk.visible_cluster_count(), talk.state());
        // 状態を更新し、発火すべきイベントを取得
        let events = talk
            .bypass_change_detection()
            .update(current_time, layout_cache.timeline());
        if (talk.visible_cluster_count(), talk.state()) != before {
            talk.set_changed();
            if talk.state() == TypewriterState::Completed {
                debug!(entity = ?entity, "Typewriter talk completed");
            }
        }

        // FireEvent 処理: 対象エンティティの TypewriterEvent を設定
        for (target, event_kind) in events {
            // 宛先が消えているのは普通の流れ（トークの途中で閉じた等）。捨てたことだけ残す。
            match commands.get_entity(target) {
                Ok(mut target_cmds) => {
                    target_cmds.try_insert(TypewriterEvent::from(event_kind));
                }
                Err(_) => {
                    debug!(entity = ?entity, target = ?target, event = ?event_kind, "Typewriter event target is gone; dropped");
                }
            }
        }
    }
}

/// Typewriter 描画システム（Draw スケジュール）
///
/// TypewriterLayoutCache から TextLayout を取得し、
/// visible_cluster_count までのグリフを描画する。
/// 色は`Brushes`コンポーネントから取得される。
/// トーク・控え・大きさ・筆が変わったか、命令の列が無い部品だけを描く。
#[allow(clippy::type_complexity)]
pub fn draw_typewriters(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &Typewriter,
            &TypewriterTalk,
            &TypewriterLayoutCache,
            &Arrangement,
            &Brushes,
            Option<&GraphicsCommandList>,
            Option<&Name>,
        ),
        Or<(
            Changed<TypewriterTalk>,
            Changed<TypewriterLayoutCache>,
            Changed<Arrangement>,
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
        warn!("GraphicsCore not available, skipping draw_typewriters");
        return;
    };

    let Some(dc) = graphics_core.device_context() else {
        warn!("DeviceContext not available");
        return;
    };

    for (entity, typewriter, talk, layout_cache, arrangement, brushes, cmd_list_opt, name) in
        query.iter()
    {
        let text_layout = layout_cache.text_layout();
        let visible_count = talk.visible_cluster_count();
        let fg_color = brushes.foreground_color();
        let bg_color_opt = brushes.background.as_color();

        #[cfg(debug_assertions)]
        debug!(
            entity = ?entity,
            visible_count = visible_count,
            progress = talk.progress(),
            "Drawing typewriter"
        );

        // 寸法を読む所の失敗も記録の失敗と同じ道へ渡す（列の無い部品に空の列を入れ、毎回やり直さない）。
        // 透明の筆や描く効果が設定できないときは全文を描かず失敗にする（前の絵を残す・設計の決め 4）
        let recorded = text_ranges_and_origin(text_layout, typewriter.direction, visible_count)
            .and_then(|(whole_text, hidden, origin)| {
                record_command_list(dc, |dc| {
                    if let Some(bg_color) = bg_color_opt {
                        fill_background(dc, &bg_color, arrangement)?;
                    }
                    let brush = dc.create_solid_color_brush(&fg_color, None)?;
                    if visible_count == 0 {
                        return Ok(());
                    }
                    // 前の描画の効果を全文から消してから、隠す範囲だけを透明の筆にする
                    // （控えに効果が残っても次の描画で直る）
                    unsafe { text_layout.SetDrawingEffect(None, whole_text) }?;
                    if let Some(range) = hidden {
                        let transparent = dc.create_solid_color_brush(&TRANSPARENT, None)?;
                        unsafe { text_layout.SetDrawingEffect(&transparent, range) }?;
                    }
                    dc.draw_text_layout(origin, text_layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
                    Ok(())
                })
            });
        store_command_list(
            &mut commands,
            entity,
            name,
            cmd_list_opt.is_some(),
            recorded,
            "typewriter",
        );
    }
}

/// 全文の範囲・隠す範囲（UTF-16）と描く原点を、控えの配置から読む。
fn text_ranges_and_origin(
    text_layout: &IDWriteTextLayout,
    direction: TextDirection,
    visible_count: u32,
) -> windows::core::Result<(DWRITE_TEXT_RANGE, Option<DWRITE_TEXT_RANGE>, Vector2)> {
    // visible_count より後ろのクラスタを隠す範囲（UTF-16）
    let cluster_lengths: Vec<u16> = text_layout
        .get_cluster_metrics()?
        .iter()
        .map(|m| m.length)
        .collect();
    let whole_text = DWRITE_TEXT_RANGE {
        startPosition: 0,
        length: cluster_lengths.iter().map(|&l| l as u32).sum(),
    };
    let hidden = hidden_text_range(&cluster_lengths, visible_count).map(|(start, length)| {
        DWRITE_TEXT_RANGE {
            startPosition: start,
            length,
        }
    });

    // テキストメトリクス取得。取れなければ原点 0 のまま描かない
    let mut text_metrics = DWRITE_TEXT_METRICS::default();
    unsafe { text_layout.GetMetrics(&mut text_metrics) }?;

    // 描画位置: 横書きは配置のはみ出し（left・top）を戻して左上へ合わせる。
    // 縦書きは原点 (0,0) のまま、1 行目の位置は DirectWrite が行の向き（右から・左から）で決める。
    let origin = match direction {
        TextDirection::VerticalRightToLeft | TextDirection::VerticalLeftToRight => {
            Vector2 { X: 0.0, Y: 0.0 }
        }
        _ => Vector2 {
            X: -text_metrics.left,
            Y: -text_metrics.top,
        },
    };
    Ok((whole_text, hidden, origin))
}

/// 空トークのTypewriter背景描画システム（Draw スケジュール）
///
/// 文が無く控えも無い部品（空のトーク・控えが作れなかった・大きさが足りない）を受け持ち、
/// 背景色のみを描画する（控えができた部品は `draw_typewriters` が同じ回に背景ごと描く）。色は`Brushes`コンポーネントから取得される。
/// 大きさ・筆・トーク・`Typewriter` が変わったか、命令の列が無い部品だけを描く。
/// 背景の色が無いか大きさが足りないときは命令の列を空にする（前の文の絵を残さない）。
#[allow(clippy::type_complexity)]
pub fn draw_typewriter_backgrounds(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &TypewriterTalk,
            &Arrangement,
            &Brushes,
            Option<&mut GraphicsCommandList>,
            Option<&Name>,
        ),
        (
            With<Typewriter>,
            Without<TypewriterLayoutCache>,
            Or<(
                Changed<Arrangement>,
                Changed<Brushes>,
                Changed<TypewriterTalk>,
                Changed<Typewriter>,
                Without<GraphicsCommandList>,
            )>,
        ),
    >,
    graphics_core: Option<Res<GraphicsCore>>,
) {
    let Some(graphics_core) = graphics_core else {
        return;
    };

    let Some(dc) = graphics_core.device_context() else {
        return;
    };

    for (entity, talk, arrangement, brushes, cmd_list_opt, name) in query.iter_mut() {
        // 文のあるトークは控えができてから draw_typewriters が描く（控えが無い間は前の絵を残す）
        let has_text = talk
            .tokens()
            .iter()
            .any(|t| matches!(t, TypewriterToken::Text(s) if !s.is_empty()));
        if has_text {
            continue;
        }

        // 背景の色が無いか、Arrangementがまだ計算されていない（極小）なら描けない
        const MIN_LAYOUT_SIZE: f32 = 10.0;
        let big_enough =
            arrangement.size.width >= MIN_LAYOUT_SIZE && arrangement.size.height >= MIN_LAYOUT_SIZE;
        let Some(bg_color) = brushes.background.as_color().filter(|_| big_enough) else {
            if let Some(mut cmd_list) = cmd_list_opt {
                cmd_list.set_if_neq(GraphicsCommandList::empty());
            }
            continue;
        };

        let recorded = record_command_list(dc, |dc| fill_background(dc, &bg_color, arrangement));
        store_command_list(
            &mut commands,
            entity,
            name,
            cmd_list_opt.is_some(),
            recorded,
            "typewriter background",
        );
    }
}

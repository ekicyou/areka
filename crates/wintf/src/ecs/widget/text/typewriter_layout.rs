//! Typewriter レイアウト系システム
//!
//! - invalidate_typewriter_layout_on_arrangement_change: Arrangement変更時のLayoutCache無効化
//! - init_typewriter_layout: TypewriterTalk追加時にLayoutCache生成
//! - convert_to_timeline: Stage1→Stage2 IR変換

use crate::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};
use crate::ecs::graphics::GraphicsCore;
use crate::ecs::layout::Arrangement;
use crate::ecs::widget::text::label::TextDirection;
use crate::ecs::widget::text::typewriter::{Typewriter, TypewriterLayoutCache, TypewriterTalk};
use crate::ecs::widget::text::typewriter_ir::{TimelineItem, TypewriterTimeline, TypewriterToken};
use bevy_ecs::prelude::*;
use tracing::{debug, trace, warn};
use windows::Win32::Graphics::DirectWrite::*;

/// TypewriterLayoutCache 無効化システム（Draw スケジュール）
///
/// Arrangementが変更されたらLayoutCacheを削除し、再生成をトリガーする。
pub fn invalidate_typewriter_layout_on_arrangement_change(
    mut commands: Commands,
    query: Query<Entity, (With<TypewriterLayoutCache>, Changed<Arrangement>)>,
) {
    for entity in query.iter() {
        debug!(entity = ?entity, "[invalidate_typewriter_layout] Arrangement changed, removing LayoutCache");
        commands.entity(entity).remove::<TypewriterLayoutCache>();
    }
}

/// TypewriterLayoutCache 初期化システム（Draw スケジュール）
///
/// TypewriterTalk が追加されたが TypewriterLayoutCache がないエンティティに対して、
/// TextLayout を作成し、Stage 2 IR に変換して LayoutCache を生成する。
pub fn init_typewriter_layout(
    mut commands: Commands,
    query: Query<
        (Entity, &Typewriter, &TypewriterTalk, &Arrangement),
        Without<TypewriterLayoutCache>,
    >,
    graphics_core: Option<Res<GraphicsCore>>,
) {
    debug!(
        "[init_typewriter_layout] Running, query count: {}",
        query.iter().count()
    );

    let Some(graphics_core) = graphics_core else {
        debug!("[init_typewriter_layout] No GraphicsCore");
        return;
    };

    let Some(dwrite_factory) = graphics_core.dwrite_factory() else {
        warn!("DirectWrite factory not available");
        return;
    };

    for (entity, typewriter, talk, arrangement) in query.iter() {
        // トークン列から全文テキストを結合
        let full_text: String = talk
            .tokens()
            .iter()
            .filter_map(|t| match t {
                TypewriterToken::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        if full_text.is_empty() {
            continue;
        }

        // Arrangementからサイズを取得
        let layout_width = arrangement.size.width;
        let layout_height = arrangement.size.height;

        // Arrangementがまだ計算されていない場合（極小値）は次フレームに委ねる
        // レイアウト計算はフレーム境界で適用されるため、初回フレームでは初期値の可能性がある
        const MIN_LAYOUT_SIZE: f32 = 10.0;
        if layout_width < MIN_LAYOUT_SIZE || layout_height < MIN_LAYOUT_SIZE {
            trace!(
                entity = ?entity,
                layout_width = layout_width,
                layout_height = layout_height,
                "[init_typewriter_layout] Arrangement too small, deferring to next frame"
            );
            continue;
        }

        debug!(
            entity = ?entity,
            layout_width = layout_width,
            layout_height = layout_height,
            "[init_typewriter_layout] Arrangement size"
        );

        // TextFormat 作成
        let font_family_hstring = windows::core::HSTRING::from(&typewriter.font_family);
        let locale_hstring = windows::core::HSTRING::from("ja-JP");
        let text_format = match dwrite_factory.create_text_format(
            &font_family_hstring,
            None::<&IDWriteFontCollection>,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            typewriter.font_size,
            &locale_hstring,
        ) {
            Ok(fmt) => fmt,
            Err(e) => {
                warn!(entity = ?entity, error = ?e, "Failed to create TextFormat");
                continue;
            }
        };

        // 縦書き・横書きの設定
        unsafe {
            match typewriter.direction {
                TextDirection::HorizontalLeftToRight => {
                    let _ = text_format.SetReadingDirection(DWRITE_READING_DIRECTION_LEFT_TO_RIGHT);
                    let _ = text_format.SetFlowDirection(DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM);
                    // 横書きLTR: 左寄せ（デフォルト）、上寄せ
                    let _ = text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                    let _ = text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
                }
                TextDirection::HorizontalRightToLeft => {
                    let _ = text_format.SetReadingDirection(DWRITE_READING_DIRECTION_RIGHT_TO_LEFT);
                    let _ = text_format.SetFlowDirection(DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM);
                    // 横書きRTL: 右寄せ、上寄せ
                    let _ = text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING);
                    let _ = text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
                }
                TextDirection::VerticalRightToLeft => {
                    let _ = text_format.SetReadingDirection(DWRITE_READING_DIRECTION_TOP_TO_BOTTOM);
                    let _ = text_format.SetFlowDirection(DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT);
                    // 縦書きRTL: 上寄せ、右寄せ（1行目を右端に）
                    let _ = text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                    let _ = text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
                }
                TextDirection::VerticalLeftToRight => {
                    let _ = text_format.SetReadingDirection(DWRITE_READING_DIRECTION_TOP_TO_BOTTOM);
                    let _ = text_format.SetFlowDirection(DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT);
                    // 縦書きLTR: 上寄せ、左寄せ
                    let _ = text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING);
                    let _ = text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
                }
            }
        }

        // TextLayout 作成（Arrangementのサイズを使用）
        let text_hstring = windows::core::HSTRING::from(&full_text);
        let text_layout = match dwrite_factory.create_text_layout(
            &text_hstring,
            &text_format,
            layout_width,
            layout_height,
        ) {
            Ok(layout) => layout,
            Err(e) => {
                warn!(entity = ?entity, error = ?e, "Failed to create TextLayout");
                continue;
            }
        };

        // Stage 1 → Stage 2 IR 変換
        let timeline = match convert_to_timeline(talk.tokens(), typewriter, &text_layout) {
            Ok(tl) => tl,
            Err(e) => {
                warn!(entity = ?entity, error = ?e, "Failed to convert timeline");
                continue;
            }
        };

        trace!(
            entity = ?entity,
            total_cluster_count = timeline.total_cluster_count,
            total_duration = timeline.total_duration,
            "[init_typewriter_layout] LayoutCache created"
        );

        // TypewriterLayoutCache を挿入
        commands
            .entity(entity)
            .insert(TypewriterLayoutCache::new(text_layout, timeline));
    }
}

/// Stage 1 → Stage 2 IR 変換
fn convert_to_timeline(
    tokens: &[TypewriterToken],
    typewriter: &Typewriter,
    text_layout: &IDWriteTextLayout,
) -> windows::core::Result<TypewriterTimeline> {
    let cluster_metrics = text_layout.get_cluster_metrics()?;
    let total_cluster_count = cluster_metrics.len() as u32;

    let mut full_text = String::new();
    let mut items = Vec::new();
    let mut current_time = 0.0;
    let mut cluster_index = 0u32;

    for token in tokens {
        match token {
            TypewriterToken::Text(text) => {
                full_text.push_str(text);

                let char_count = text.chars().count();
                for _ in 0..char_count {
                    if cluster_index < total_cluster_count {
                        current_time += typewriter.default_char_wait;
                        items.push(TimelineItem::Glyph {
                            cluster_index,
                            show_at: current_time,
                        });
                        cluster_index += 1;
                    }
                }
            }
            TypewriterToken::Wait(duration) => {
                items.push(TimelineItem::Wait {
                    duration: *duration,
                    start_at: current_time,
                });
                current_time += duration;
            }
            TypewriterToken::FireEvent { target, event } => {
                items.push(TimelineItem::FireEvent {
                    target: *target,
                    event: event.clone(),
                    fire_at: current_time,
                });
            }
        }
    }

    Ok(TypewriterTimeline {
        full_text,
        items,
        total_duration: current_time,
        total_cluster_count,
    })
}

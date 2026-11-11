//! Typewriter レイアウト系システム
//!
//! - invalidate_typewriter_layout_on_arrangement_change: Arrangement・Typewriter変更時のLayoutCache無効化
//! - init_typewriter_layout: TypewriterTalk追加時にLayoutCache生成
//! - convert_to_timeline: Stage1→Stage2 IR変換

use crate::com::dwrite::{DWriteFactoryExt, DWriteTextLayoutExt};
use crate::ecs::graphics::GraphicsCore;
use crate::ecs::layout::Arrangement;
use crate::ecs::widget::text::label::{TextDirection, apply_direction};
use crate::ecs::widget::text::typewriter::{Typewriter, TypewriterLayoutCache, TypewriterTalk};
use crate::ecs::widget::text::typewriter_ir::{TimelineItem, TypewriterTimeline, TypewriterToken};
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::prelude::*;
use tracing::{debug, trace, warn};
use windows::Win32::Graphics::DirectWrite::*;

/// TypewriterLayoutCache 無効化システム（Draw スケジュール）
///
/// Arrangement か Typewriter（文字の大きさ・向き・既定の待ちなど）が変わったら LayoutCache を削除し、
/// 再生成をトリガーする。
#[allow(clippy::type_complexity)]
pub fn invalidate_typewriter_layout_on_arrangement_change(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            With<TypewriterLayoutCache>,
            Or<(Changed<Arrangement>, Changed<Typewriter>)>,
        ),
    >,
) {
    for entity in query.iter() {
        debug!(entity = ?entity, "[invalidate_typewriter_layout] Arrangement or Typewriter changed, removing LayoutCache");
        commands.entity(entity).remove::<TypewriterLayoutCache>();
    }
}

/// TypewriterLayoutCache 初期化システム（Draw スケジュール）
///
/// TypewriterTalk が追加されたが TypewriterLayoutCache がないエンティティに対して、
/// TextLayout を作成し、Stage 2 IR に変換して LayoutCache を生成する。
/// 作れなかった部品は、`Typewriter`・`TypewriterTalk`・`Arrangement` のどれかが変わるまで作り直さない
/// （毎回の `warn!` を止める）。
#[allow(clippy::type_complexity)]
pub fn init_typewriter_layout(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            Ref<Typewriter>,
            Ref<TypewriterTalk>,
            Ref<Arrangement>,
        ),
        Without<TypewriterLayoutCache>,
    >,
    graphics_core: Option<Res<GraphicsCore>>,
    mut failed: Local<EntityHashSet>,
) {
    // 変化の印は係が回るたびに読めなくなるので、GraphicsCore が無くて戻る前に読んで外す
    // （控えができた部品・消えた部品もここで外れる）
    failed.retain(|&entity| {
        query
            .get(entity)
            .is_ok_and(|(_, typewriter, talk, arrangement)| {
                !(typewriter.is_changed() || talk.is_changed() || arrangement.is_changed())
            })
    });
    if query.is_empty() {
        return;
    }

    let Some(graphics_core) = graphics_core else {
        debug!("[init_typewriter_layout] No GraphicsCore");
        return;
    };

    let Some(dwrite_factory) = graphics_core.dwrite_factory() else {
        warn!("DirectWrite factory not available");
        return;
    };

    for (entity, typewriter, talk, arrangement) in query.iter() {
        if failed.contains(&entity) {
            continue;
        }

        // トークン列から全文テキストを結合
        let full_text: String = talk
            .tokens()
            .iter()
            .filter_map(|t| match t {
                TypewriterToken::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        // 文が無くても待ちやイベントがあれば控えを作って進める（空の文も DirectWrite は配置でき、クラスタは 0）
        if talk.tokens().is_empty() {
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

        // 係には返す先が無いので、失敗は記録して次の変化まで飛ばす
        match build_layout_cache(
            dwrite_factory,
            &typewriter,
            talk.tokens(),
            full_text,
            layout_width,
            layout_height,
        ) {
            Ok(cache) => {
                trace!(
                    entity = ?entity,
                    total_cluster_count = cache.timeline().total_cluster_count,
                    "[init_typewriter_layout] LayoutCache created"
                );
                commands.entity(entity).insert(cache);
            }
            Err(e) => {
                warn!(entity = ?entity, direction = ?typewriter.direction, error = ?e, "Failed to build typewriter layout");
                failed.insert(entity);
            }
        }
    }
}

/// 書式 → 向き → 揃え → 配置 → 時間表の順に控えを作る。
fn build_layout_cache(
    dwrite_factory: &IDWriteFactory2,
    typewriter: &Typewriter,
    tokens: &[TypewriterToken],
    full_text: String,
    layout_width: f32,
    layout_height: f32,
) -> windows::core::Result<TypewriterLayoutCache> {
    // TextFormat 作成
    let font_family_hstring = windows::core::HSTRING::from(&typewriter.font_family);
    let locale_hstring = windows::core::HSTRING::from("ja-JP");
    let text_format = dwrite_factory.create_text_format(
        &font_family_hstring,
        None::<&IDWriteFontCollection>,
        DWRITE_FONT_WEIGHT_NORMAL,
        DWRITE_FONT_STYLE_NORMAL,
        DWRITE_FONT_STRETCH_NORMAL,
        typewriter.font_size,
        &locale_hstring,
    )?;

    // 縦書き・横書きの設定
    apply_direction(&text_format, typewriter.direction)?;
    // 揃え: 右から左の横書きだけ右寄せ、ほかは行の頭へ。段落はどれも上（行の流れの頭）へ
    let text_alignment = match typewriter.direction {
        TextDirection::HorizontalRightToLeft => DWRITE_TEXT_ALIGNMENT_TRAILING,
        _ => DWRITE_TEXT_ALIGNMENT_LEADING,
    };
    unsafe {
        text_format.SetTextAlignment(text_alignment)?;
        text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
    }

    // TextLayout 作成（Arrangementのサイズを使用）
    let text_hstring = windows::core::HSTRING::from(&full_text);
    let text_layout = dwrite_factory.create_text_layout(
        &text_hstring,
        &text_format,
        layout_width,
        layout_height,
    )?;

    // Stage 1 → Stage 2 IR 変換
    let timeline = convert_to_timeline(tokens, typewriter, &text_layout, full_text)?;
    Ok(TypewriterLayoutCache::new(text_layout, timeline))
}

/// トークン列とクラスタの長さ（UTF-16）から時間表の項目を作る。
///
/// 文のトークンは、その UTF-16 の終わりに届くまでクラスタを 1 つずつ `Glyph` にする
/// （サロゲート対や結合文字の 2 文字以上が 1 クラスタでも数がずれない）。
pub(crate) fn timeline_items(
    tokens: &[TypewriterToken],
    cluster_lengths: &[u16],
    default_char_wait: f64,
) -> Vec<TimelineItem> {
    let mut items = Vec::new();
    let mut current_time = 0.0;
    let mut cluster_index = 0usize;
    // 出したクラスタの UTF-16 の長さの和と、ここまでの文の UTF-16 の長さの和
    let mut shown_end = 0usize;
    let mut text_end = 0usize;

    for token in tokens {
        match token {
            TypewriterToken::Text(text) => {
                text_end += text.encode_utf16().count();
                while shown_end < text_end && cluster_index < cluster_lengths.len() {
                    current_time += default_char_wait;
                    items.push(TimelineItem::Glyph {
                        cluster_index: cluster_index as u32,
                        show_at: current_time,
                    });
                    shown_end += cluster_lengths[cluster_index] as usize;
                    cluster_index += 1;
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
    items
}

/// Stage 1 → Stage 2 IR 変換
fn convert_to_timeline(
    tokens: &[TypewriterToken],
    typewriter: &Typewriter,
    text_layout: &IDWriteTextLayout,
    full_text: String,
) -> windows::core::Result<TypewriterTimeline> {
    let cluster_lengths: Vec<u16> = text_layout
        .get_cluster_metrics()?
        .iter()
        .map(|m| m.length)
        .collect();

    Ok(TypewriterTimeline {
        full_text,
        items: timeline_items(tokens, &cluster_lengths, typewriter.default_char_wait),
        total_cluster_count: cluster_lengths.len() as u32,
    })
}

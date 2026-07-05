//! Typewriter 描画系システム
//!
//! - update_typewriters: 毎フレームの状態更新（Update スケジュール）
//! - draw_typewriters: 描画コマンド生成（Draw スケジュール）
//! - draw_typewriter_backgrounds: 空トークの背景のみ描画（Draw スケジュール）

use crate::com::d2d::{D2D1CommandListExt, D2D1DeviceContextExt};
use crate::com::dwrite::DWriteTextLayoutExt;
use crate::ecs::graphics::{FrameTime, GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::Arrangement;
use crate::ecs::widget::brushes::{Brushes, DEFAULT_FOREGROUND};
use crate::ecs::widget::text::label::TextDirection;
use crate::ecs::widget::text::typewriter::{
    Typewriter, TypewriterLayoutCache, TypewriterState, TypewriterTalk,
};
use crate::ecs::widget::text::typewriter_ir::{TypewriterEvent, TypewriterToken};
use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE;
use windows::Win32::Graphics::DirectWrite::*;
use windows_numerics::{Matrix3x2, Vector2};

/// 透明色定数（内部使用）
const TRANSPARENT_COLOR: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

/// Typewriter 更新システム（Update スケジュール）
///
/// FrameTime リソースから現在フレームの時刻を取得し、TypewriterTalk の状態を更新する。
/// FireEvent トークンを処理して対象エンティティの TypewriterEvent を設定する。
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
        // 状態を更新し、発火すべきイベントを取得
        let events = talk.update(current_time, layout_cache.timeline());

        // FireEvent 処理: 対象エンティティの TypewriterEvent を設定
        for (target, event_kind) in events {
            commands
                .entity(target)
                .insert(TypewriterEvent::from(event_kind));
        }

        // 完了した場合のログ
        if talk.state() == TypewriterState::Completed {
            #[cfg(debug_assertions)]
            debug!(entity = ?entity, "Typewriter talk completed");
        }
    }
}

/// Typewriter 描画システム（Draw スケジュール）
///
/// TypewriterLayoutCache から TextLayout を取得し、
/// visible_cluster_count までのグリフを描画する。
/// 色は`Brushes`コンポーネントから取得される。
pub fn draw_typewriters(
    mut commands: Commands,
    query: Query<(
        Entity,
        &Typewriter,
        &TypewriterTalk,
        &TypewriterLayoutCache,
        &Arrangement,
        &Brushes,
    )>,
    graphics_core: Option<Res<GraphicsCore>>,
) {
    let Some(graphics_core) = graphics_core else {
        warn!("GraphicsCore not available, skipping draw_typewriters");
        return;
    };

    let Some(dc) = graphics_core.device_context() else {
        warn!("DeviceContext not available");
        return;
    };

    for (entity, typewriter, talk, layout_cache, arrangement, brushes) in query.iter() {
        let text_layout = layout_cache.text_layout();
        let timeline = layout_cache.timeline();
        let visible_count = talk.visible_cluster_count();

        // Brushes から色を取得
        let fg_color = brushes
            .foreground
            .as_color()
            .unwrap_or_else(|| DEFAULT_FOREGROUND.as_color().unwrap());
        let bg_color_opt = brushes.background.as_color();

        #[cfg(debug_assertions)]
        debug!(
            entity = ?entity,
            visible_count = visible_count,
            progress = talk.progress(),
            "Drawing typewriter"
        );

        // クラスタメトリクス取得（描画範囲決定用）
        let cluster_metrics = match text_layout.get_cluster_metrics() {
            Ok(m) => m,
            Err(e) => {
                warn!(entity = ?entity, error = ?e, "Failed to get cluster metrics");
                continue;
            }
        };

        // visible_count までのテキスト位置を計算
        let visible_text_length: u32 = cluster_metrics
            .iter()
            .take(visible_count as usize)
            .map(|m| m.length as u32)
            .sum();

        // テキストメトリクス取得
        let mut text_metrics = DWRITE_TEXT_METRICS::default();
        unsafe {
            let _ = text_layout.GetMetrics(&mut text_metrics);
        }

        // CommandList生成
        let command_list = match unsafe { dc.CreateCommandList() } {
            Ok(cl) => cl,
            Err(err) => {
                warn!(entity = ?entity, error = ?err, "Failed to create CommandList");
                continue;
            }
        };

        // CommandListをターゲットに設定
        unsafe {
            dc.SetTarget(&command_list);
        }

        // 描画開始
        unsafe {
            // 共有DCのワールド変換をリセット（前フレームの描画処理の残留変換を防止）
            dc.SetTransform(&Matrix3x2::identity());
            dc.BeginDraw();
        }

        // 透明でクリア
        dc.clear(Some(&TRANSPARENT_COLOR));

        // バックグラウンド描画（指定されている場合）
        // Rectangleと同様にarrangement.sizeを使用
        if let Some(bg_color) = bg_color_opt {
            if let Ok(bg_brush) = dc.create_solid_color_brush(&bg_color, None) {
                let bg_rect: windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F =
                    crate::ecs::Rect {
                        left: 0.0,
                        top: 0.0,
                        right: arrangement.size.width,
                        bottom: arrangement.size.height,
                    }
                    .into();
                unsafe {
                    dc.FillRectangle(&bg_rect, &bg_brush);
                }
            }
        }

        // フォアグラウンドブラシ作成
        let brush = match dc.create_solid_color_brush(&fg_color, None) {
            Ok(b) => b,
            Err(err) => {
                warn!(entity = ?entity, error = ?err, "Failed to create brush");
                unsafe {
                    let _ = dc.EndDraw(None, None);
                }
                continue;
            }
        };

        // visible_count > 0 の場合のみ描画
        if visible_count > 0 && visible_text_length > 0 {
            // 描画位置の調整
            // DirectWriteはレイアウトボックス(layoutWidth x layoutHeight)を基準に描画する
            //
            // 横書きLTR: 原点はレイアウトボックス左上、テキストは右へ流れる
            //   → left >= 0, origin.X = -left で左端を合わせる
            //
            // 縦書きRTL: 原点はレイアウトボックス左上、1行目はレイアウトボックスの右端から始まり左へ流れる
            //   → テキストがはみ出すとleft < 0になる
            //   → Surfaceはtext_metrics.width幅で作成される
            //   → レイアウトボックスの右端をSurfaceの右端に合わせるには:
            //     origin.X = text_metrics.width - layoutWidth
            let origin = match typewriter.direction {
                TextDirection::VerticalRightToLeft => {
                    // 縦書きRTL: テキストはlayoutWidthの右端から始まり左へ流れる
                    // originは(0,0)で、DirectWriteが自動的に右端から配置
                    Vector2 { X: 0.0, Y: 0.0 }
                }
                TextDirection::VerticalLeftToRight => {
                    // 縦書きLTR: 1行目は左端から始まる
                    Vector2 { X: 0.0, Y: 0.0 }
                }
                _ => {
                    // 横書き: 従来通り
                    Vector2 {
                        X: -text_metrics.left,
                        Y: -text_metrics.top,
                    }
                }
            };

            // 非表示部分を透明にするため、visible_text_length以降に透明ブラシを設定
            let total_text_length = timeline.full_text.chars().count() as u32;

            if visible_text_length < total_text_length {
                // 透明ブラシ作成
                let transparent_brush = match dc.create_solid_color_brush(&TRANSPARENT_COLOR, None)
                {
                    Ok(b) => b,
                    Err(_) => {
                        // フォールバック: 全文描画
                        dc.draw_text_layout(
                            origin,
                            text_layout,
                            &brush,
                            D2D1_DRAW_TEXT_OPTIONS_NONE,
                        );
                        unsafe {
                            let _ = dc.EndDraw(None, None);
                        }
                        continue;
                    }
                };

                // 非表示範囲に透明ブラシを設定
                let hidden_range = DWRITE_TEXT_RANGE {
                    startPosition: visible_text_length,
                    length: total_text_length - visible_text_length,
                };
                unsafe {
                    let _ = text_layout.SetDrawingEffect(&transparent_brush, hidden_range);
                }
            }

            // 全文を描画（非表示部分は透明ブラシで描画される）
            dc.draw_text_layout(origin, text_layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);

            // 設定をリセット（次回描画のため）
            if visible_text_length < total_text_length {
                let hidden_range = DWRITE_TEXT_RANGE {
                    startPosition: visible_text_length,
                    length: total_text_length - visible_text_length,
                };
                unsafe {
                    let _ = text_layout.SetDrawingEffect(None, hidden_range);
                }
            }
        }

        // 描画終了
        if let Err(err) = unsafe { dc.EndDraw(None, None) } {
            warn!(entity = ?entity, error = ?err, "EndDraw failed");
            continue;
        }

        // CommandListを閉じる
        if let Err(err) = command_list.close() {
            warn!(entity = ?entity, error = ?err, "Failed to close CommandList");
            continue;
        }

        // GraphicsCommandListをエンティティに挿入
        // Surfaceサイズは GlobalArrangement.bounds から計算される（Rectangleと同じ）
        commands
            .entity(entity)
            .insert(GraphicsCommandList::new(command_list));
    }
}

/// 空トークのTypewriter背景描画システム（Draw スケジュール）
///
/// TypewriterTalkはあるがTyperwriterLayoutCacheがない（空トーク）の場合、
/// 背景色のみを描画する。色は`Brushes`コンポーネントから取得される。
pub fn draw_typewriter_backgrounds(
    mut commands: Commands,
    query: Query<
        (Entity, &Typewriter, &TypewriterTalk, &Arrangement, &Brushes),
        (Without<TypewriterLayoutCache>, Without<GraphicsCommandList>),
    >,
    graphics_core: Option<Res<GraphicsCore>>,
) {
    let Some(graphics_core) = graphics_core else {
        return;
    };

    let Some(dc) = graphics_core.device_context() else {
        return;
    };

    for (entity, _typewriter, talk, arrangement, brushes) in query.iter() {
        // テキストがある場合はdraw_typewritersが処理するのでスキップ
        let has_text = talk
            .tokens()
            .iter()
            .any(|t| matches!(t, TypewriterToken::Text(s) if !s.is_empty()));
        if has_text {
            continue;
        }

        // 背景色がない場合はスキップ（Brushes.backgroundから取得）
        let Some(bg_color) = brushes.background.as_color() else {
            continue;
        };

        // Arrangementがまだ計算されていない場合はスキップ
        const MIN_LAYOUT_SIZE: f32 = 10.0;
        if arrangement.size.width < MIN_LAYOUT_SIZE || arrangement.size.height < MIN_LAYOUT_SIZE {
            continue;
        }

        // CommandList生成
        let command_list = match unsafe { dc.CreateCommandList() } {
            Ok(cl) => cl,
            Err(_) => continue,
        };

        unsafe {
            dc.SetTarget(&command_list);
            // 共有DCのワールド変換をリセット（前フレームの描画処理の残留変換を防止）
            dc.SetTransform(&Matrix3x2::identity());
            dc.BeginDraw();
        }

        // 透明でクリア
        dc.clear(Some(&TRANSPARENT_COLOR));

        // 背景描画
        if let Ok(bg_brush) = dc.create_solid_color_brush(&bg_color, None) {
            let bg_rect: windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F =
                crate::ecs::Rect {
                    left: 0.0,
                    top: 0.0,
                    right: arrangement.size.width,
                    bottom: arrangement.size.height,
                }
                .into();
            unsafe {
                dc.FillRectangle(&bg_rect, &bg_brush);
            }
        }

        // 描画終了
        if unsafe { dc.EndDraw(None, None) }.is_err() {
            continue;
        }

        // CommandListを閉じる
        if command_list.close().is_err() {
            continue;
        }

        // GraphicsCommandListをエンティティに挿入
        // Surfaceサイズは GlobalArrangement.bounds から計算される（Rectangleと同じ）
        commands
            .entity(entity)
            .insert(GraphicsCommandList::new(command_list));
    }
}

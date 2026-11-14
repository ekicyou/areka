//! 描く列を DC へ再生する係（COM を呼ぶのはここだけ）。変換の規則は design の System Flows の表。

use super::super::MixMode;
use super::command::*;
use super::diff::{DamageEntry, PxRect};
use crate::numerics::Aabb;
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::error;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::Matrix3x2;

/// 描く 1 件の描き方（`draw_method` の答え）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DrawMethod {
    /// `DrawImage` をこの重ね方で。
    Image(D2D1_COMPOSITE_MODE),
    /// `BlendImage` をこのブレンドで。
    Blend(D2D1_BLEND_MODE),
    /// ブレンドの描き方（`ID2D1DeviceContext6`）が無い。`SOURCE_OVER` で描く（1.7）。
    BlendUnavailable,
    /// 重ね方 0〜12・ブレンド 0〜25 の外。D2D へ渡さず `SOURCE_OVER` で描く。
    OutOfRange,
}

/// 混ぜ方と「ブレンドの描き方が在るか」から描き方を決める。範囲の確かめが先。
pub(crate) fn draw_method(mix: MixMode, blend_available: bool) -> DrawMethod {
    match mix {
        MixMode::Composite(c) if (0..=12).contains(&c.0) => DrawMethod::Image(c),
        MixMode::Blend(b) if (0..=25).contains(&b.0) => {
            if blend_available {
                DrawMethod::Blend(b)
            } else {
                DrawMethod::BlendUnavailable
            }
        }
        _ => DrawMethod::OutOfRange,
    }
}

/// その種類の誤りをプロセスで初めて見たときだけ `true`（`error!` を残す番）。
pub(crate) fn report_once(reported: &AtomicBool) -> bool {
    !reported.swap(true, Ordering::Relaxed)
}

static BLEND_UNAVAILABLE: AtomicBool = AtomicBool::new(false);
static MIX_OUT_OF_RANGE: AtomicBool = AtomicBool::new(false);

/// 列 `commands` を `dc` へ画家の順に再生する。
///
/// - `root` は描画面の変換（`GlobalArrangement` の拡大＋`BeginDraw` の offset）。DIP→物理。
/// - `Draw` は `world_mat * root`・軸に沿う切り抜きは `root`（矩形は DIP のまま渡す）・
///   図形の切り抜きは DC を恒等にして局所→物理の全部を層の `maskTransform` で渡す。
/// - ブレンドは世界の変形を効かせたあと `BlendImage` で描き、絵の外を変えない（1.3・1.6）。
///   ブレンドの描き方が無いときと範囲の外の値は、種類ごとにプロセスで 1 度だけ `error!` を残して
///   `SOURCE_OVER` で描く（1.7）。
/// - 層はどれも変換を恒等にし、範囲を物理の整数の矩形へ外向きに丸めて積む。
/// - 不透明度の層は透明から始まり、降ろすとき `SOURCE_OVER` で不透明度を掛けて重なる。層の中の
///   重ね方とブレンドは、同じ層の中で先に描いた画素に効く（3.4）。
/// - 図形の切り抜きの層は背景の写しから始まり（`INITIALIZE_FROM_BACKGROUND`）、積む時点だけ基本の
///   混ぜ方を `COPY` にする。降ろすとき覆いの割合で層と的を線形に補間するので、中の重ね方と
///   ブレンドは切り抜きの形に依らず下の画素に効き、外側は変わらない（3.1〜3.3）。基本の混ぜ方は
///   積む時点の値が効き、積んだ直後に `SOURCE_OVER` へ戻す（降ろす直前だけ `COPY` にすると誤る）。
/// - `dc6` はブレンドの描き方（`BlendImage`）を持つ同じ DC。持たない DC なら `None`。
///   呼び手が合成の回に 1 度 `dc.cast()` して渡す。
/// - 戻ってきたとき DC の変換は `root` に戻してある。D2D の描画の呼び出しは HRESULT を返さず、
///   失敗は呼び手の `EndDraw` の結果に出る。
pub(crate) fn emit(
    dc: &ID2D1DeviceContext,
    dc6: Option<&ID2D1DeviceContext6>,
    root: Matrix3x2,
    commands: &[DrawCommand],
) {
    play(dc, dc6, root, commands, None);
}

/// `emit` と同じ再生で、項の `bounds` が描く矩形 `rect` と交わらない件（`Draw`）を飛ばす（2.2）。
///
/// 層の命令は全部そのまま積み降ろし、層の旗は変えない。範囲の外の画素は呼び手の切り抜きが守るので、
/// 飛ばすのは性能のためだけ。前提: `entries` は同じ `commands` から作った列（Pop 以外の命令と
/// 1 対 1）。項の無い件は飛ばさずに描く。
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "範囲だけの合成（タスク 4.2）が使い始めたら外す")
)]
pub(crate) fn emit_within(
    dc: &ID2D1DeviceContext,
    dc6: Option<&ID2D1DeviceContext6>,
    root: Matrix3x2,
    commands: &[DrawCommand],
    entries: &[DamageEntry],
    rect: PxRect,
) {
    play(dc, dc6, root, commands, Some((entries, rect)));
}

fn play(
    dc: &ID2D1DeviceContext,
    dc6: Option<&ID2D1DeviceContext6>,
    root: Matrix3x2,
    commands: &[DrawCommand],
    within: Option<(&[DamageEntry], PxRect)>,
) {
    let identity = Matrix3x2::identity();
    // 項の番号（Pop 以外の命令を数える）。
    let mut index = 0usize;
    for command in commands {
        let entry = index;
        if !matches!(
            command,
            DrawCommand::PopClipRect | DrawCommand::PopClipGeometry | DrawCommand::PopOpacity
        ) {
            index += 1;
        }
        // SAFETY: Windows API 境界。呼び手が `BeginDraw` の中で呼ぶ。push と pop は `build` が対で出す。
        unsafe {
            match command {
                DrawCommand::Draw(item) => {
                    if let Some((entries, rect)) = within
                        && entries
                            .get(entry)
                            .is_some_and(|e| e.bounds.intersect(rect).is_empty())
                    {
                        continue;
                    }
                    dc.SetTransform(&(item.world_mat * root));
                    let composite = match draw_method(item.mix_mode, dc6.is_some()) {
                        DrawMethod::Image(c) => c,
                        DrawMethod::Blend(b) => {
                            if let Some(dc6) = dc6 {
                                dc6.BlendImage(
                                    &item.command_list,
                                    b,
                                    None,
                                    None,
                                    item.interpolation_mode,
                                );
                            }
                            continue;
                        }
                        DrawMethod::BlendUnavailable => {
                            if report_once(&BLEND_UNAVAILABLE) {
                                error!(
                                    entity = ?item.entity,
                                    mix_mode = ?item.mix_mode,
                                    "[emit] BlendImage is unavailable (Windows 10 before 1903); drawing with SOURCE_OVER"
                                );
                            }
                            D2D1_COMPOSITE_MODE_SOURCE_OVER
                        }
                        DrawMethod::OutOfRange => {
                            if report_once(&MIX_OUT_OF_RANGE) {
                                error!(
                                    entity = ?item.entity,
                                    mix_mode = ?item.mix_mode,
                                    "[emit] mix mode out of range; drawing with SOURCE_OVER"
                                );
                            }
                            D2D1_COMPOSITE_MODE_SOURCE_OVER
                        }
                    };
                    dc.DrawImage(
                        &item.command_list,
                        None,
                        None,
                        item.interpolation_mode,
                        composite,
                    );
                }
                DrawCommand::PushClipRect(clip) => {
                    dc.SetTransform(&root);
                    dc.PushAxisAlignedClip(&clip.world_aabb.0, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                }
                DrawCommand::PopClipRect => dc.PopAxisAlignedClip(),
                DrawCommand::PushClipGeometry(clip) => {
                    dc.SetTransform(&identity);
                    // 積む時点の基本の混ぜ方が降ろすときに効く（research.md §10.2）。
                    dc.SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_COPY);
                    let params = D2D1_LAYER_PARAMETERS1 {
                        contentBounds: outward_physical(clip.world_aabb, root),
                        geometricMask: ManuallyDrop::new(Some(clip.geometry.clone())),
                        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                        maskTransform: clip.world_mat * root,
                        opacity: 1.0,
                        opacityBrush: ManuallyDrop::new(None),
                        layerOptions: D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND,
                    };
                    dc.PushLayer(&params, None::<&ID2D1Layer>);
                    dc.SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
                    // 層が自分で AddRef する。ここで持った複製を返す。
                    drop(ManuallyDrop::into_inner(params.geometricMask));
                }
                DrawCommand::PopClipGeometry => dc.PopLayer(),
                DrawCommand::PushOpacity(layer) => {
                    dc.SetTransform(&identity);
                    let params = D2D1_LAYER_PARAMETERS1 {
                        contentBounds: outward_physical(layer.world_aabb, root),
                        geometricMask: ManuallyDrop::new(None),
                        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                        maskTransform: identity,
                        opacity: layer.opacity,
                        opacityBrush: ManuallyDrop::new(None),
                        layerOptions: D2D1_LAYER_OPTIONS1_NONE,
                    };
                    dc.PushLayer(&params, None::<&ID2D1Layer>);
                }
                DrawCommand::PopOpacity => dc.PopLayer(),
            }
        }
    }
    // SAFETY: Windows API 境界。
    unsafe { dc.SetTransform(&root) };
}

/// DIP の矩形を `root` で物理へ写し、整数へ外向きに丸める（層の写しを画素の格子に揃える）。
fn outward_physical(world_aabb: Aabb, root: Matrix3x2) -> D2D_RECT_F {
    let b = world_aabb.transform(root).0;
    D2D_RECT_F {
        left: b.left.floor(),
        top: b.top.floor(),
        right: b.right.ceil(),
        bottom: b.bottom.ceil(),
    }
}

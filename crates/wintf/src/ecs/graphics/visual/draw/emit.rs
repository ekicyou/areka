//! 描く列を DC へ再生する係（COM を呼ぶのはここだけ）。

use super::super::MixMode;
use super::command::*;
use super::diff::{DamageEntry, PxRect};
use crate::ecs::graphics::visual::placement::invert;
use crate::numerics::Aabb;
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::error;
use windows::Win32::Foundation::E_INVALIDARG;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::core::Interface;
use windows_numerics::Matrix3x2;

/// 描く 1 件の描き方（`draw_method` の答え）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DrawMethod {
    /// `DrawImage` をこの重ね方で。
    Image(D2D1_COMPOSITE_MODE),
    /// `BlendImage` をこのブレンドで。
    Blend(D2D1_BLEND_MODE),
    /// ブレンドの描き方（`ID2D1DeviceContext6`）が無い。`SOURCE_OVER` で描く。
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
/// - ブレンドは世界の変形を効かせたあと `BlendImage` で描き、絵の外を変えない。
///   ブレンドの描き方が無いときと範囲の外の値は、種類ごとにプロセスで 1 度だけ `error!` を残して
///   `SOURCE_OVER` で描く。
/// - 層はどれも変換を恒等にし、範囲を物理の整数の矩形へ外向きに丸めて積む。
/// - まとめ（`PushGroup`〜`PopGroup`）は中身を透明から始まる 1 枚に描き、節の変換を当てて `mix` で重ねる
///   （[`draw_group`]）。`emit_within` でも中身は全部描く。
/// - 不透明度の層は透明から始まり、降ろすとき `SOURCE_OVER` で不透明度を掛けて重なる。層の中の
///   重ね方とブレンドは、同じ層の中で先に描いた画素に効く。
/// - 図形の切り抜きの層は背景の写しから始まり（`INITIALIZE_FROM_BACKGROUND`）、積む時点だけ基本の
///   混ぜ方を `COPY` にする。降ろすとき覆いの割合で層と的を線形に補間するので、中の重ね方と
///   ブレンドは切り抜きの形に依らず下の画素に効き、外側は変わらない。基本の混ぜ方は
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

/// `emit` と同じ再生で、項の `bounds` が描く矩形 `rect` と交わらない件（`Draw`）を飛ばす。
///
/// 層の命令は全部そのまま積み降ろし、層の旗は変えない。範囲の外の画素は呼び手の切り抜きが守るので、
/// 飛ばすのは性能のためだけ。前提: `entries` は同じ `commands` から作った列（Pop 以外の命令と
/// 1 対 1）。項の無い件は飛ばさずに描く。
pub(crate) fn emit_within(
    dc: &ID2D1DeviceContext,
    dc6: Option<&ID2D1DeviceContext6>,
    root: Matrix3x2,
    commands: &[DrawCommand],
    entries: &[DamageEntry],
    rect: PxRect,
) {
    // 列のずれは静かに画素を誤らせる（別の件の bounds で飛ばす）。
    debug_assert_eq!(
        entries.len(),
        commands.iter().filter(|c| !is_pop(c)).count(),
        "entries は同じ commands から作った列でない"
    );
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
    let mut index = 0usize;
    let mut next = 0usize;
    while let Some(command) = commands.get(next) {
        next += 1;
        let entry = index;
        if !is_pop(command) {
            index += 1;
        }
        if let DrawCommand::PushGroup(group) = command {
            let inner = &commands[next..group_end(commands, next)];
            next += inner.len() + 1;
            index += inner.iter().filter(|c| !is_pop(c)).count();
            draw_group(dc, dc6, root, group, inner);
            continue;
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
                    draw_image(
                        dc,
                        dc6,
                        &item.command_list,
                        item.entity,
                        item.mix_mode,
                        item.interpolation_mode,
                    );
                }
                DrawCommand::PushClipRect(clip) => {
                    dc.SetTransform(&root);
                    dc.PushAxisAlignedClip(&clip.world_aabb.0, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                }
                DrawCommand::PopClipRect => dc.PopAxisAlignedClip(),
                DrawCommand::PushClipGeometry(clip) => {
                    dc.SetTransform(&identity);
                    // 積む時点の基本の混ぜ方が降ろすときに効く。
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
                // `PushGroup` は上で対の `PopGroup` まで進めている。
                DrawCommand::PushGroup(_) | DrawCommand::PopGroup => {}
            }
        }
    }
    // SAFETY: Windows API 境界。
    unsafe { dc.SetTransform(&root) };
}

fn is_pop(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::PopClipRect
            | DrawCommand::PopClipGeometry
            | DrawCommand::PopOpacity
            | DrawCommand::PopGroup
    )
}

/// `start` から始まる中身を閉じる `PopGroup` の位置（対が無ければ列の終わり）。
fn group_end(commands: &[DrawCommand], start: usize) -> usize {
    let mut depth = 0usize;
    for (i, command) in commands.iter().enumerate().skip(start) {
        match command {
            DrawCommand::PushGroup(_) => depth += 1,
            DrawCommand::PopGroup if depth == 0 => return i,
            DrawCommand::PopGroup => depth -= 1,
            _ => {}
        }
    }
    commands.len()
}

/// 絵 `image` を今の変換のまま混ぜ方 `mix` で描く。ブレンドの描き方が無いときと範囲の外の値は、
/// 種類ごとにプロセスで 1 度だけ `error!` を残して `SOURCE_OVER` で描く。
///
/// # Safety
/// 呼び手が `dc` の `BeginDraw` の中で呼ぶ。`dc6` は `dc` と同じ DC。
unsafe fn draw_image(
    dc: &ID2D1DeviceContext,
    dc6: Option<&ID2D1DeviceContext6>,
    image: &ID2D1Image,
    entity: bevy_ecs::entity::Entity,
    mix: MixMode,
    interpolation: D2D1_INTERPOLATION_MODE,
) {
    let composite = match draw_method(mix, dc6.is_some()) {
        DrawMethod::Image(c) => c,
        DrawMethod::Blend(b) => {
            if let Some(dc6) = dc6 {
                // SAFETY: 呼び手の約束（`BeginDraw` の中）。
                unsafe { dc6.BlendImage(image, b, None, None, interpolation) };
            }
            return;
        }
        DrawMethod::BlendUnavailable => {
            if report_once(&BLEND_UNAVAILABLE) {
                error!(
                    entity = ?entity,
                    mix_mode = ?mix,
                    "[emit] BlendImage is unavailable (Windows 10 before 1903); drawing with SOURCE_OVER"
                );
            }
            D2D1_COMPOSITE_MODE_SOURCE_OVER
        }
        DrawMethod::OutOfRange => {
            if report_once(&MIX_OUT_OF_RANGE) {
                error!(
                    entity = ?entity,
                    mix_mode = ?mix,
                    "[emit] mix mode out of range; drawing with SOURCE_OVER"
                );
            }
            D2D1_COMPOSITE_MODE_SOURCE_OVER
        }
    };
    // SAFETY: 呼び手の約束（`BeginDraw` の中）。
    unsafe { dc.DrawImage(image, None, None, interpolation, composite) };
}

/// まとめ 1 つを描く。`inner` は中身の命令（対の `PushGroup`・`PopGroup` を含まない）。
///
/// `SOURCE_OVER` で、節の局所→物理の変換が整数の平行移動なら、透明から始まる不透明度 1 の層で描く
/// （局所の画素の格子が的の格子と重なるので、作業用の絵を通す答えと同じ）。ほかは作業用の絵
/// （[`group_bitmap`]）を節の変換で `LINEAR`・`mix` で描く。作業用の絵を作れなければ `error!` を残して
/// まとめごと描かない。
fn draw_group(
    dc: &ID2D1DeviceContext,
    dc6: Option<&ID2D1DeviceContext6>,
    root: Matrix3x2,
    group: &GroupLayer,
    inner: &[DrawCommand],
) {
    let to_px = group.world_mat * root;
    let on_grid = (to_px.M11, to_px.M12, to_px.M21, to_px.M22) == (1.0, 0.0, 0.0, 1.0)
        && to_px.M31.fract() == 0.0
        && to_px.M32.fract() == 0.0;
    if on_grid && group.mix == MixMode::default() {
        let identity = Matrix3x2::identity();
        let params = D2D1_LAYER_PARAMETERS1 {
            contentBounds: outward_physical(group.world_aabb, root),
            geometricMask: ManuallyDrop::new(None),
            maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            maskTransform: identity,
            opacity: 1.0,
            opacityBrush: ManuallyDrop::new(None),
            layerOptions: D2D1_LAYER_OPTIONS1_NONE,
        };
        // SAFETY: Windows API 境界。呼び手が `BeginDraw` の中で呼ぶ。層は中身を再生した後に必ず降ろす。
        unsafe {
            dc.SetTransform(&identity);
            dc.PushLayer(&params, None::<&ID2D1Layer>);
            play(dc, dc6, root, inner, None);
            dc.PopLayer();
        }
        return;
    }
    let bitmap = match group_bitmap(dc, dc6.is_some(), group, inner) {
        Ok(bitmap) => bitmap,
        Err(e) => {
            error!(
                entity = ?group.entity,
                width = group.local_aabb.width(),
                height = group.local_aabb.height(),
                error = ?e,
                hresult = format_args!("0x{:08X}", e.code().0),
                "[emit] まとめの作業用の絵を作れないので、まとめごと描かない"
            );
            return;
        }
    };
    let origin = Matrix3x2::translation(group.local_aabb.left(), group.local_aabb.top());
    // SAFETY: Windows API 境界。呼び手が `BeginDraw` の中で呼ぶ。絵は `EndDraw` を済ませた的。
    unsafe {
        dc.SetTransform(&(origin * to_px));
        draw_image(
            dc,
            dc6,
            &bitmap,
            group.entity,
            group.mix,
            D2D1_INTERPOLATION_MODE_LINEAR,
        );
    }
}

/// まとめの中身を、同じ装置の別の DC で、透明に塗った `local_aabb` の大きさの絵（1 DIP＝1 画素）へ描く。
fn group_bitmap(
    dc: &ID2D1DeviceContext,
    blend: bool,
    group: &GroupLayer,
    inner: &[DrawCommand],
) -> windows::core::Result<ID2D1Bitmap1> {
    let b = group.local_aabb;
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
        colorContext: ManuallyDrop::new(None),
    };
    let size = D2D_SIZE_U {
        width: b.width() as u32,
        height: b.height() as u32,
    };
    let inverse =
        invert(group.world_mat).ok_or_else(|| windows::core::Error::from(E_INVALIDARG))?;
    let root = inverse * Matrix3x2::translation(-b.left(), -b.top());
    // SAFETY: Windows API 境界。生きている DC の装置から DC と絵を作り、`BeginDraw` と `EndDraw` の対の中で描く。
    unsafe {
        let work = dc
            .GetDevice()?
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
        let bitmap = work.CreateBitmap(size, None, 0, &props)?;
        let work6 = if blend { work.cast().ok() } else { None };
        work.SetTarget(&bitmap);
        work.BeginDraw();
        work.Clear(None);
        play(&work, work6.as_ref(), root, inner, None);
        work.EndDraw(None, None)?;
        work.SetTarget(None);
        Ok(bitmap)
    }
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

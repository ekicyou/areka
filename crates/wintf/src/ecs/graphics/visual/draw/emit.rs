//! 描く列を DC へ再生する係（COM を呼ぶのはここだけ）。変換の規則は design の System Flows の表。

use super::super::MixMode;
use super::command::*;
use std::mem::ManuallyDrop;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::Matrix3x2;

/// 列 `commands` を `dc` へ画家の順に再生する。
///
/// - `root` は描画面の変換（`GlobalArrangement` の拡大＋`BeginDraw` の offset）。DIP→物理。
/// - `Draw` は `world_mat * root`・軸に沿う切り抜きは `root`（矩形は DIP のまま渡す）・
///   図形の切り抜きは DC を恒等にして局所→物理の全部を層の `maskTransform` で渡す。
/// - 層は透明から始まる（`D2D1_LAYER_OPTIONS1_NONE`）。層の中の `SOURCE_OVER` でない重ね方は
///   層の中身に対して効く（D2D の既定・変えるのは `wintf-d2d-composite-rules`）。
/// - 戻ってきたとき DC の変換は `root` に戻してある。D2D の描画の呼び出しは HRESULT を返さず、
///   失敗は呼び手の `EndDraw` の結果に出る。
pub(crate) fn emit(dc: &ID2D1DeviceContext, root: Matrix3x2, commands: &[DrawCommand]) {
    let identity = Matrix3x2::identity();
    for command in commands {
        // SAFETY: Windows API 境界。呼び手が `BeginDraw` の中で呼ぶ。push と pop は `build` が対で出す。
        unsafe {
            match command {
                DrawCommand::Draw(item) => {
                    // shortcut: ブレンドは SOURCE_OVER で描く。2.1 で BlendImage へ置き換える。
                    let composite = match item.mix_mode {
                        MixMode::Composite(c) => c,
                        MixMode::Blend(_) => D2D1_COMPOSITE_MODE_SOURCE_OVER,
                    };
                    dc.SetTransform(&(item.world_mat * root));
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
                    let params = D2D1_LAYER_PARAMETERS1 {
                        contentBounds: D2D_RECT_F {
                            left: -f32::MAX,
                            top: -f32::MAX,
                            right: f32::MAX,
                            bottom: f32::MAX,
                        },
                        geometricMask: ManuallyDrop::new(Some(clip.geometry.clone())),
                        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                        maskTransform: clip.world_mat * root,
                        opacity: 1.0,
                        opacityBrush: ManuallyDrop::new(None),
                        layerOptions: D2D1_LAYER_OPTIONS1_NONE,
                    };
                    dc.PushLayer(&params, None::<&ID2D1Layer>);
                    // 層が自分で AddRef する。ここで持った複製を返す。
                    drop(ManuallyDrop::into_inner(params.geometricMask));
                }
                DrawCommand::PopClipGeometry => dc.PopLayer(),
            }
        }
    }
    // SAFETY: Windows API 境界。
    unsafe { dc.SetTransform(&root) };
}

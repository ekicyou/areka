//! com/d2d/command_sink.rs — RecCommandSink のデバイス非依存テスト
//!
//! `#[implement(ID2D1CommandSink5)]` で生成された COM オブジェクトは
//! デバイス無しでインスタンス化できる。パラメータのみを取るコールバックを
//! COM ABI（vtable）経由で呼び出し、unsafe なポインタ参照経路
//! （null / 有効ポインタの両方）と HRESULT 変換の健全性を検証する。
//!
//! COM オブジェクト引数（ブラシ等）を要するコールバックは実デバイスが
//! 必要なため、`d2d_ext_test.rs` の ID2D1CommandList::Stream 再生で検証する。

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;
use wintf::com::d2d::*;

fn make_sink() -> ID2D1CommandSink5 {
    RecCommandSink::new().into()
}

// ============================================================
// 構造体としての直接 API
// ============================================================

/// `commands()` は未実装（`todo!()`）であり呼び出すとパニックする。
/// 現行挙動の特性化（修正は挙動変更を伴うため提案として記録）。
#[test]
#[should_panic]
fn commands_accessor_is_unimplemented_and_panics() {
    let sink = RecCommandSink::new();
    let _ = sink.commands();
}

/// push / clear はパニックせず動作する（内部状態は commands() が
/// 未実装のため観測不能 — 呼び出し健全性のみ固定）。
#[test]
fn push_and_clear_do_not_panic() {
    let mut sink = RecCommandSink::new();
    sink.push(DrawCommand::BeginDraw);
    sink.push(DrawCommand::EndDraw);
    sink.clear();
    sink.push(DrawCommand::PopLayer);
}

// ============================================================
// COM ABI 経由のコールバック呼び出し（vtable 結線の検証）
// ============================================================

#[test]
fn begin_draw_and_end_draw_return_ok_via_com_abi() {
    let sink = make_sink();
    unsafe {
        sink.BeginDraw().expect("BeginDraw");
        sink.EndDraw().expect("EndDraw");
    }
}

#[test]
fn state_setters_return_ok_via_com_abi() {
    let sink = make_sink();
    unsafe {
        sink.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED)
            .expect("SetAntialiasMode");
        sink.SetTags(11, 22).expect("SetTags");
        sink.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE)
            .expect("SetTextAntialiasMode");
        sink.SetTextRenderingParams(None)
            .expect("SetTextRenderingParams(None)");
        sink.SetUnitMode(D2D1_UNIT_MODE_DIPS).expect("SetUnitMode");
    }
}

/// SetPrimitiveBlend は ID2D1CommandSink / Sink1 / Sink4 の3世代が
/// 同名で存在し、それぞれ独立した DrawCommand へ記録される。
#[test]
fn primitive_blend_all_generations_return_ok() {
    let sink = make_sink();
    unsafe {
        sink.SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER)
            .expect("SetPrimitiveBlend");
        sink.SetPrimitiveBlend1(D2D1_PRIMITIVE_BLEND_COPY)
            .expect("SetPrimitiveBlend1");
        sink.SetPrimitiveBlend2(D2D1_PRIMITIVE_BLEND_MAX)
            .expect("SetPrimitiveBlend2");
    }
}

/// SetTransform は生ポインタ越しに Matrix3x2 を unsafe コピーする実装。
/// 有効なポインタで Ok が返ることを検証する。
#[test]
fn set_transform_copies_matrix_through_raw_pointer() {
    let sink = make_sink();
    let m = Matrix3x2 {
        M11: 2.0,
        M12: 0.0,
        M21: 0.0,
        M22: 2.0,
        M31: 10.0,
        M32: 20.0,
    };
    unsafe {
        sink.SetTransform(&m).expect("SetTransform");
    }
}

/// Clear(*const D2D1_COLOR_F) は null 許容: `color.as_ref().cloned()` で
/// null → None に写像する。null / 有効ポインタの両経路を検証する。
#[test]
fn clear_accepts_both_null_and_valid_color_pointer() {
    let sink = make_sink();
    let color = D2D1_COLOR_F {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    unsafe {
        sink.Clear(None).expect("Clear(null)");
        sink.Clear(Some(&color)).expect("Clear(&color)");
    }
}

#[test]
fn push_pop_axis_aligned_clip_return_ok() {
    let sink = make_sink();
    let rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: 100.0,
        bottom: 50.0,
    };
    unsafe {
        sink.PushAxisAlignedClip(&rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE)
            .expect("PushAxisAlignedClip");
        sink.PopAxisAlignedClip().expect("PopAxisAlignedClip");
    }
}

/// PushLayer は D2D1_LAYER_PARAMETERS1 の COM フィールド
/// （geometricMask / opacityBrush）が null、layer 引数が None でも
/// 記録できる（Option 写像経路の検証）。
#[test]
fn push_pop_layer_with_null_com_fields_return_ok() {
    let sink = make_sink();
    let params = D2D1_LAYER_PARAMETERS1 {
        contentBounds: D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 64.0,
            bottom: 64.0,
        },
        ..Default::default()
    };
    unsafe {
        sink.PushLayer(&params, None).expect("PushLayer");
        sink.PopLayer().expect("PopLayer");
    }
}

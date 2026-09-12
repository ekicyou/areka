//! 記録（[`record_display`]）の失敗注入: 4 点それぞれで `error!` 1 行＋`Err` が返り、共有 DC に
//! 副作用が残らないことを固定する（要件 7.1／7.2・GPU は資源生成のみ＝要件 6.5）。
//!
//! # 撤去済みの供給面の失敗注入との対応（移設の実体）
//!
//! `chain_fault_tests.rs`（spec `areka-P0-test-cage-determinism` の **④ 失敗注入**——同 spec
//! design.md `## C4` ④ が `chain.rs` の `UploadFault`／`fault_point` と当該テストを一体で指す）は、
//! 注入点ごとに ⑴ `Err(PresentError::Device{context, hresult})` が返り `context` が注入点と一致
//! すること ⑵ 失敗の後も供給面が自己整合で、次の成功呼出で回復すること、を観測していた。
//! 本ファイルはその 2 つの観測点を、供給面の消えた新しい形（記録＝`record_display`）へ移したもの
//! である。⑴ はそのまま、⑵ は「同じ共有 DC への次の記録が成功する」という記録層で観測できる形に
//! 読み替える（`BeginDraw` の開きっぱなし・途中生成資源の居残りはいずれもここで赤くなる）。
//!
//! # ここで新しく増える観測点
//!
//! `error!` が**ちょうど 1 行**出ることは `chain_fault_tests.rs` は見ていなかった（`Err` の形しか
//! 見ていない）。要件 7.1／7.2 が求めるのは「ログ無しで縮退しない」ことなので、本ファイルでは
//! ログ捕捉（`log-capture-kit`）を併せて掛け、行の重複・欠落の双方を赤にする。
//!
//! # 副作用が残らないことの、この層での意味
//!
//! `record_display` は値を返すだけで呼び手の状態を触らない。ゆえに観測できる副作用は**共有 DC**
//! に残るもの（`BeginDraw` 開きっぱなし・ターゲットや変換の残留）だけであり、それは「同じ DC で
//! 次の記録が成功して空でないリストを返す」ことで検出できる。
//!
//! 注入旗の後始末は**観測ではなく保証**である。[`record_with_armed_fault`] の `Disarm` が
//! どの経路（正常終了・assert の panic）でも旗を降ろし、`fault_point` も一致した時点で旗を
//! 消費するので、旗が次の記録や次のテストへ漏れることは構造的に起こり得ない。テストはその
//! 漏れを検出しているのではなく、漏れない形で書かれている。

use super::*;

use log_capture_kit::capture;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::GraphicsCore;

/// 注入が載せる HRESULT（`display.rs` の `fault_point` と同値＝E_FAIL）。
const INJECTED_HRESULT: i32 = 0x8000_4005u32 as i32;

/// `record_display` が失敗し得る 4 点（`DisplayFault` の全 variant）。
///
/// variant が増えたらここに足さなければ被覆が黙って減る。列挙の網羅は
/// [`every_display_fault_variant_has_a_case`] が `match` で見張る。
const ALL_FAULTS: [DisplayFault; 4] = [
    DisplayFault::CreateBitmap,
    DisplayFault::CreateCommandList,
    DisplayFault::EndDraw,
    DisplayFault::Close,
];

/// 実 GPU（HARDWARE デバイス）を組んで共有 DC を借りる（`display_gpu_tests.rs` の `gpu_dc` と
/// 同型。画素は読まないので DPI の固定は要らない）。
///
/// `GraphicsCore` を戻り値に含めるのは DC の寿命がそれに縛られるため。GPU が無ければ `expect`
/// で赤くなる（黙って緑にする skip は置かない）。
fn gpu_dc() -> (GraphicsCore, ID2D1DeviceContext) {
    // 各テストは専用スレッドで走り COM 未初期化ゆえ MTA を初期化する
    // （S_FALSE／RPC_E_CHANGED_MODE は無視してよい）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
    let dc = core
        .device_context()
        .expect("GraphicsCore::device_context が None")
        .clone();
    (core, dc)
}

/// 注入 → 記録 → 武装解除 を **1 つの不可分な操作**として閉じる唯一の入口
/// （`chain_fault_tests.rs` の `upload_with_armed_fault` と同型）。
///
/// 解除を `Drop` に持たせ `arm_display_fault` の呼出をここ 1 箇所に限ることで、「解除の書き忘れ」
/// も「assert の panic で解除を飛び越すこと」も構造的に起こり得なくする（注入旗はスレッド局所
/// ゆえ、解け残ると同一スレッドの後続テストを巻き添えにする）。
fn record_with_armed_fault(
    dc: &ID2D1DeviceContext,
    at: DisplayFault,
    surface: &ComposedSurface,
) -> Result<GraphicsCommandList, PresentError> {
    /// 生存期間の終わり（正常終了・panic による巻き戻しの双方）で必ず武装を降ろす番人。
    struct Disarm;
    impl Drop for Disarm {
        fn drop(&mut self) {
            clear_display_fault();
        }
    }

    let _disarm = Disarm;
    arm_display_fault(at);
    record_display(dc, surface)
}

/// 1 点の検査本体: 注入して記録 → `Err` の形・`error!` 1 行 → 同じ DC で次の記録が成功する。
fn run_case(at: DisplayFault) {
    let (_core, dc) = gpu_dc();
    let surface = ComposedSurface::new(8, 6);

    // ── 注入した記録: Err と error! を 1 つの窓で同時に観る ────────────────
    let (result, events) = capture(|| record_with_armed_fault(&dc, at, &surface));

    let err = result.err().unwrap_or_else(|| {
        panic!("注入した失敗点 {at:?} で record_display は Err を返すはず（Ok が返った）")
    });
    match err {
        PresentError::Device { hresult, context } => {
            assert_eq!(
                context,
                injected_context(at),
                "注入した失敗点 {at:?} の文脈文字列が一致しない"
            );
            assert_eq!(
                hresult, INJECTED_HRESULT,
                "注入は E_FAIL を載せる（{at:?}）"
            );
        }
        other => {
            panic!("注入の失敗は PresentError::Device のはずだが {other:?} が返った（{at:?}）")
        }
    }

    // 要件 7.1／7.2: ログ無しで縮退しない。多重記録も赤にするため件数をちょうどで固定する。
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "注入 1 回につき error! はちょうど 1 行（{at:?}・実測 {} 行）",
        errors.len()
    );
    let line = errors[0];
    assert_eq!(
        line.target, "areka_emo_present::command",
        "error! は共有の失敗写像 device_err から出るべき（{at:?}）"
    );
    assert_eq!(
        line.message(),
        "D3D/DXGI 呼び出しが失敗",
        "error! の本文が device_err の字面と違う（{at:?}）"
    );
    assert_eq!(
        line.field("hresult"),
        Some(INJECTED_HRESULT.to_string().as_str()),
        "error! 行が注入した HRESULT を載せていない（{at:?}）"
    );
    assert_eq!(
        line.field_str("context"),
        Some(injected_context(at)),
        "error! 行の context が注入点と一致しない（{at:?}）"
    );

    // ── 副作用が残らないこと: 同じ共有 DC への次の記録が素通りで成功する ────
    // 上の窓で error! を 1 行捕まえているので、この窓の「0 行」が捕捉の空振りでないことは
    // 同一スレッド・同一発行点の陽性で担保されている。
    let (recovered, quiet) = capture(|| record_display(&dc, &surface));
    let list = recovered.unwrap_or_else(|e| {
        panic!("{at:?} の失敗の後、同じ DC への次の記録が失敗した（副作用が残っている）: {e:?}")
    });
    assert!(
        list.command_list().is_some(),
        "回復後の記録が空のリストを返した（{at:?}）"
    );
    assert_ne!(
        list,
        GraphicsCommandList::empty(),
        "回復後の記録が GraphicsCommandList::empty() と等しい（{at:?}）"
    );
    assert!(
        quiet.iter().all(|e| e.level != tracing::Level::ERROR),
        "成功した記録が error! を出した（成功経路にログを足す退行・{at:?}）"
    );
}

/// `CreateBitmap` 失敗: `error!` 1 行＋`Err`・同じ DC で次の記録が成功する。
#[test]
fn create_bitmap_failure_logs_once_and_leaves_no_side_effect() {
    run_case(DisplayFault::CreateBitmap);
}

/// `CreateCommandList` 失敗: `error!` 1 行＋`Err`・同じ DC で次の記録が成功する。
#[test]
fn create_command_list_failure_logs_once_and_leaves_no_side_effect() {
    run_case(DisplayFault::CreateCommandList);
}

/// `EndDraw` 失敗: `error!` 1 行＋`Err`・同じ DC で次の記録が成功する
/// （注入点は実呼び出しの後にあるため、共有 DC が `BeginDraw` 開きっぱなしで残らない）。
#[test]
fn end_draw_failure_logs_once_and_leaves_no_side_effect() {
    run_case(DisplayFault::EndDraw);
}

/// `Close` 失敗: `error!` 1 行＋`Err`・同じ DC で次の記録が成功する
/// （閉じ損ねたコマンドリストは戻り値に載らず、その場で drop される）。
#[test]
fn close_failure_logs_once_and_leaves_no_side_effect() {
    run_case(DisplayFault::Close);
}

/// 被覆の番人: `DisplayFault` に variant が増えたら [`ALL_FAULTS`] の欠落をコンパイルで気づかせ、
/// 4 本の `#[test]` が全 variant を覆っていることを実行時にも固定する。
#[test]
fn every_display_fault_variant_has_a_case() {
    for at in ALL_FAULTS {
        // variant が増えると非網羅で**コンパイルが落ちる**（テストの追加漏れが静かに通らない）。
        let _: &str = match at {
            DisplayFault::CreateBitmap => {
                "create_bitmap_failure_logs_once_and_leaves_no_side_effect"
            }
            DisplayFault::CreateCommandList => {
                "create_command_list_failure_logs_once_and_leaves_no_side_effect"
            }
            DisplayFault::EndDraw => "end_draw_failure_logs_once_and_leaves_no_side_effect",
            DisplayFault::Close => "close_failure_logs_once_and_leaves_no_side_effect",
        };
    }
    assert_eq!(ALL_FAULTS.len(), 4, "失敗し得るのは 4 点");
    for (i, at) in ALL_FAULTS.iter().enumerate() {
        assert!(
            !ALL_FAULTS[..i].contains(at),
            "被覆表に重複がある: {at:?}（4 点は互いに異なる）"
        );
    }
}

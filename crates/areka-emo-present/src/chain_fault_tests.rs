//! `upload` 失敗時の「前の状態を保つ」を、失敗点 × 経路の 11 組で固定する実行テスト。
//!
//! 期待値の出所は design.md `## System Flows` → `### Flow 3` の**期待値表**（第 3 箇条）であり、
//! 本ファイルの [`EXPECTATIONS`] はその表をそのまま写したものである（要件 5.1・5.2）。
//!
//! - 経路は 2 本。外形不変（`upload` の寸が現 `size()` と同じ）と外形変更（異なる）。
//! - 寸法変更 3 点（`CreateSourceTex`／`CreateStaging`／`ResizeBuffers`）は外形不変経路では
//!   そもそも踏まれないので組として意味を持たない。ゆえに 7 × 2 = 14 ではなく **11 組**である。
//! - 既知の残余 2 件は**是正せず現状の挙動を期待値として書く**（要件 5.9・2026-08-22 設計
//!   ディスカッション 議題 2 の開発者裁定）。⒜ `Present` 失敗＝表示（backbuffer）は前フレームの
//!   ままだが `source_tex` は試行内容を持つため `read_back()` は**未提示の試行内容**を返す。
//!   ⒝ 外形変更経路で `ResizeBuffers` 成功後に後段が失敗＝struct は旧値で自己整合だが swap chain
//!   の backbuffer だけが新寸・未描画（次回 `upload` が `self.size` 不一致で回復する）。
//! - どの失敗の後でも、次の成功 `upload` で `read_back` は新内容・`size` は新寸へ回復することを
//!   全 11 本が同じ形で確かめる。
//!
//! 前提は既存のグラフィクステスト（`chain.rs` の `mod tests`）と同一——**窓なし・実 D3D デバイス**
//! ——であり、実機 GPU 障害の再現を必要としない（要件 5.4）。注入点は `#[cfg(test)]` でのみ実体化
//! されるため通常ビルドの挙動・性能特性は変わらない（要件 5.5）。
//!
//! 本ファイルの範囲で是正が閉じない発見（`show.rs`／`target.rs` 等への波及）は生じていないため、
//! 要件 5.8 に基づく起票は無い。

use super::*;

use wintf::ecs::GraphicsCore;

use super::test_support::{composed_of_size, make_dispatcher_and_compositor};

// ── 材料の寸法・模様（既知の非退化パターン）──────────────────────────────
/// 直前に成功した表示（＝「前の状態」）の外形と salt。
const PREVIOUS: (u32, u32, u8) = (3, 2, 0x11);
/// 外形不変経路で失敗させる upload の外形と salt（寸は `PREVIOUS` と同じ・模様だけ違う）。
const ATTEMPT_SAME_SHAPE: (u32, u32, u8) = (3, 2, 0x77);
/// 外形変更経路で失敗させる upload の外形と salt。
const ATTEMPT_NEW_SHAPE: (u32, u32, u8) = (5, 4, 0xA5);
/// 失敗の後に流す成功 upload（回復の確認用）。`PREVIOUS`・両 `ATTEMPT` のいずれとも寸が異なる。
const RECOVERY: (u32, u32, u8) = (7, 3, 0x2E);

/// `upload` に渡す外形が現在の `size()` と同じか異なるか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    /// 外形不変経路（寸法変更 3 点は踏まれない）。
    ShapeUnchanged,
    /// 外形変更経路（7 点すべてを踏み得る）。
    ShapeChange,
}

/// 失敗が返った後の `size()` の期待値。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SizeAfter {
    /// 旧寸のまま（＝前の状態）。
    Old,
    /// 新寸（`Present` は commit の後にあるため外形変更経路でのみ起こる）。
    New,
}

/// 失敗が返った後の `read_back()` の期待値。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContentAfter {
    /// 直前に成功した upload の内容（＝前の状態）。
    Previous,
    /// 失敗した upload が渡そうとした内容（残余 ⒜＝未提示の試行内容）。
    Attempted,
}

/// design Flow 3 の期待値表（11 行）。1 行 = 1 本の `#[test]`。
///
/// 外形不変経路: `SourceTexCast`／`GetBuffer`／`BackbufferCast` 失敗で 4 項目不変・`read_back`
/// 旧内容、`Present` 失敗で `size` 不変・`read_back` 試行内容。
/// 外形変更経路: `CreateSourceTex`／`CreateStaging`／`ResizeBuffers` 失敗で 4 項目不変・
/// `read_back` 旧内容・旧寸、`SourceTexCast`／`GetBuffer`／`BackbufferCast` 失敗で struct 4 項目
/// 不変・`read_back` 旧内容・旧寸（残余 ⒝）、`Present` 失敗で `size` 新値・`read_back` 試行内容・
/// 新寸。
const EXPECTATIONS: [(UploadFault, Route, SizeAfter, ContentAfter); 11] = [
    // ── 外形不変経路（4 組）──
    (
        UploadFault::SourceTexCast,
        Route::ShapeUnchanged,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::GetBuffer,
        Route::ShapeUnchanged,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::BackbufferCast,
        Route::ShapeUnchanged,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::Present,
        Route::ShapeUnchanged,
        SizeAfter::Old,
        ContentAfter::Attempted,
    ),
    // ── 外形変更経路（7 組）──
    (
        UploadFault::CreateSourceTex,
        Route::ShapeChange,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::CreateStaging,
        Route::ShapeChange,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::ResizeBuffers,
        Route::ShapeChange,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::SourceTexCast,
        Route::ShapeChange,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::GetBuffer,
        Route::ShapeChange,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::BackbufferCast,
        Route::ShapeChange,
        SizeAfter::Old,
        ContentAfter::Previous,
    ),
    (
        UploadFault::Present,
        Route::ShapeChange,
        SizeAfter::New,
        ContentAfter::Attempted,
    ),
];

/// 注入 → `upload` → 武装解除 を **1 つの不可分な操作**として閉じる唯一の入口。
///
/// `fault_point` は**一致した点でしか武装を解かない**ので、届かなかった注入（例: 外形不変の
/// `upload` に寸法変更点を武装する）は旗が立ったまま残り、同一スレッドの後続 `upload` で発火する。
/// 解除を `Drop` に持たせ、`arm_upload_fault` の呼出を本関数の内側 1 箇所だけに限ることで、
/// 「解除の書き忘れ」も「assert の panic で解除を飛び越すこと」も**構造的に起こり得ない**
/// （呼び出し側に解除を書く場所が無い）。
fn upload_with_armed_fault(
    presenter: &mut SwapChainPresenter,
    at: UploadFault,
    surface: &ComposedSurface,
) -> Result<(), PresentError> {
    /// 生存期間の終わり（正常終了・panic による巻き戻しの双方）で必ず武装を降ろす番人。
    struct Disarm;
    impl Drop for Disarm {
        fn drop(&mut self) {
            clear_upload_fault();
        }
    }

    let _disarm = Disarm;
    arm_upload_fault(at);
    presenter.upload(surface)
}

/// `(w, h, salt)` から既知の非退化パターンを合成する。
fn composed(spec: (u32, u32, u8)) -> ComposedSurface {
    let (w, h, salt) = spec;
    let surface = composed_of_size(w, h, salt);
    assert!(
        surface.bytes().iter().any(|&b| b != 0),
        "fixture は非退化（全 0 でない）でなければ檻にならない"
    );
    surface
}

/// 期待値表から 1 行を引いて実行する。表に無い組み合わせは即座に落とす（表が唯一の権威）。
fn run_row(at: UploadFault, route: Route) {
    let (size_after, content_after) = EXPECTATIONS
        .iter()
        .find(|(row_at, row_route, _, _)| *row_at == at && *row_route == route)
        .map(|(_, _, size_after, content_after)| (*size_after, *content_after))
        .unwrap_or_else(|| panic!("期待値表に ({at:?}, {route:?}) の行が無い"));
    run_case(at, route, size_after, content_after);
}

/// 1 組の検査本体: 前の状態を作る → 失敗を注入した `upload` → 前状態の判定 → 回復の判定。
fn run_case(at: UploadFault, route: Route, size_after: SizeAfter, content_after: ContentAfter) {
    let (_dq, compositor) = make_dispatcher_and_compositor();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");

    // ── 前提: 直前に成功した表示（これが「前の状態」）────────────────────
    let previous = composed(PREVIOUS);
    let (mut presenter, _surface) =
        SwapChainPresenter::new(&core, &compositor, previous.width(), previous.height())
            .expect("SwapChainPresenter::new 失敗");
    presenter
        .upload(&previous)
        .expect("前提となる成功 upload が失敗した");
    let previous_size = (previous.width(), previous.height());
    assert_eq!(presenter.size(), previous_size, "前提の size が想定と違う");
    assert_eq!(
        presenter.read_back().expect("前提の read_back 失敗"),
        previous.bytes(),
        "前提の read_back が upload バイトと一致しない"
    );

    // ── 失敗注入 ──────────────────────────────────────────────────────
    let attempted = composed(match route {
        Route::ShapeUnchanged => ATTEMPT_SAME_SHAPE,
        Route::ShapeChange => ATTEMPT_NEW_SHAPE,
    });
    let attempted_size = (attempted.width(), attempted.height());
    match route {
        Route::ShapeUnchanged => assert_eq!(
            attempted_size, previous_size,
            "外形不変経路の材料は前の状態と同寸でなければならない"
        ),
        Route::ShapeChange => assert_ne!(
            attempted_size, previous_size,
            "外形変更経路の材料は前の状態と別寸でなければならない"
        ),
    }
    assert_ne!(
        attempted.bytes(),
        previous.bytes(),
        "試行内容と前の内容が同じでは前状態保持を判定できない"
    );

    let err = upload_with_armed_fault(&mut presenter, at, &attempted)
        .expect_err("注入した失敗点で upload は Err を返すはず");

    // 要件 5.3 の形（`device_err` 経由＝error! 済み・構造化エラー）で返っていること。
    match err {
        PresentError::Device { hresult, context } => {
            assert_eq!(
                context,
                injected_context(at),
                "注入した失敗点の文脈文字列が一致しない"
            );
            assert_eq!(hresult, 0x8000_4005u32 as i32, "注入は E_FAIL を載せる");
        }
        other => panic!("注入の失敗は PresentError::Device のはずだが {other:?} が返った"),
    }

    // ── 前の状態の判定（design Flow 3 の期待値表）─────────────────────
    let expected_size = match size_after {
        SizeAfter::Old => previous_size,
        SizeAfter::New => attempted_size,
    };
    assert_eq!(
        presenter.size(),
        expected_size,
        "失敗後の size() が期待値表と違う（{at:?} / {route:?}）"
    );

    // `read_back` は `self.size` の寸で `source_tex`→`staging` を複製して読むため、成功して
    // 期待バイト列と一致することが「`source_tex`／`staging`／`size` が互いに自己整合」の実測になる。
    let read_back = presenter
        .read_back()
        .expect("失敗後も read_back は成功する（内部 4 項目が自己整合）");
    let expected_bytes = match content_after {
        ContentAfter::Previous => previous.bytes(),
        ContentAfter::Attempted => attempted.bytes(),
    };
    assert_eq!(
        read_back.len(),
        (expected_size.0 * expected_size.1 * 4) as usize,
        "失敗後の read_back の長さが期待寸と違う（{at:?} / {route:?}）"
    );
    assert_eq!(
        read_back, expected_bytes,
        "失敗後の read_back 内容が期待値表と違う（{at:?} / {route:?} / {content_after:?}）"
    );

    // ── 回復の判定: どの失敗の後でも次の成功で新内容・新寸になる ────────
    let recovered = composed(RECOVERY);
    let recovered_size = (recovered.width(), recovered.height());
    assert_ne!(
        recovered_size, previous_size,
        "回復材料は前の状態と別寸でなければ size の追随を判定できない"
    );
    assert_ne!(
        recovered_size, attempted_size,
        "回復材料は試行と別寸でなければ size の追随を判定できない"
    );
    presenter
        .upload(&recovered)
        .expect("失敗の次の成功 upload が失敗した（回復しない）");
    assert_eq!(
        presenter.size(),
        recovered_size,
        "回復後の size() が新寸でない（{at:?} / {route:?}）"
    );
    assert_eq!(
        presenter.read_back().expect("回復後の read_back 失敗"),
        recovered.bytes(),
        "回復後の read_back が新内容でない（{at:?} / {route:?}）"
    );
}

// ── 外形不変経路（4 組）──────────────────────────────────────────────

/// 外形不変 × `SourceTexCast` 失敗: 4 項目不変・`read_back` 旧内容。
#[test]
fn shape_unchanged_source_tex_cast_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::SourceTexCast, Route::ShapeUnchanged);
}

/// 外形不変 × `GetBuffer` 失敗: 4 項目不変・`read_back` 旧内容。
#[test]
fn shape_unchanged_get_buffer_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::GetBuffer, Route::ShapeUnchanged);
}

/// 外形不変 × `BackbufferCast` 失敗: 4 項目不変・`read_back` 旧内容。
#[test]
fn shape_unchanged_backbuffer_cast_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::BackbufferCast, Route::ShapeUnchanged);
}

/// 外形不変 × `Present` 失敗（残余 ⒜）: `size` 不変・`read_back` は未提示の試行内容。
#[test]
fn shape_unchanged_present_failure_keeps_the_size_and_reads_back_the_attempted_content() {
    run_row(UploadFault::Present, Route::ShapeUnchanged);
}

// ── 外形変更経路（7 組）──────────────────────────────────────────────

/// 外形変更 × `CreateSourceTex` 失敗: 4 項目不変・`read_back` 旧内容・旧寸。
#[test]
fn shape_change_create_source_tex_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::CreateSourceTex, Route::ShapeChange);
}

/// 外形変更 × `CreateStaging` 失敗: 4 項目不変・`read_back` 旧内容・旧寸。
#[test]
fn shape_change_create_staging_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::CreateStaging, Route::ShapeChange);
}

/// 外形変更 × `ResizeBuffers` 失敗: 4 項目不変・`read_back` 旧内容・旧寸。
#[test]
fn shape_change_resize_buffers_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::ResizeBuffers, Route::ShapeChange);
}

/// 外形変更 × `SourceTexCast` 失敗（残余 ⒝）: struct 4 項目不変・`read_back` 旧内容・旧寸。
/// swap chain の backbuffer だけが新寸・未描画になるが、それは読み戻せないため観測外。
#[test]
fn shape_change_source_tex_cast_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::SourceTexCast, Route::ShapeChange);
}

/// 外形変更 × `GetBuffer` 失敗（残余 ⒝）: struct 4 項目不変・`read_back` 旧内容・旧寸。
#[test]
fn shape_change_get_buffer_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::GetBuffer, Route::ShapeChange);
}

/// 外形変更 × `BackbufferCast` 失敗（残余 ⒝）: struct 4 項目不変・`read_back` 旧内容・旧寸。
#[test]
fn shape_change_backbuffer_cast_failure_keeps_the_previous_content_and_size() {
    run_row(UploadFault::BackbufferCast, Route::ShapeChange);
}

/// 外形変更 × `Present` 失敗（残余 ⒜）: `size` は新値・`read_back` は未提示の試行内容・新寸。
#[test]
fn shape_change_present_failure_takes_the_new_size_and_reads_back_the_attempted_content() {
    run_row(UploadFault::Present, Route::ShapeChange);
}

// ── 表そのものの被覆（行が黙って減らないための番人）────────────────────

/// 期待値表がちょうど 11 組——外形変更経路に 7 点すべて・外形不変経路に寸法変更 3 点を除く 4 点
/// ——を、重複なく持つことを固定する（14 ではなく 11 である理由をコードで読めるようにする）。
#[test]
fn the_expectation_table_holds_exactly_the_11_meaningful_combinations() {
    const ALL_FAULTS: [UploadFault; 7] = [
        UploadFault::CreateSourceTex,
        UploadFault::CreateStaging,
        UploadFault::ResizeBuffers,
        UploadFault::SourceTexCast,
        UploadFault::GetBuffer,
        UploadFault::BackbufferCast,
        UploadFault::Present,
    ];
    // 外形が変わるときにしか踏まれない 3 点（外形不変経路では組として意味を持たない）。
    const SHAPE_CHANGE_ONLY: [UploadFault; 3] = [
        UploadFault::CreateSourceTex,
        UploadFault::CreateStaging,
        UploadFault::ResizeBuffers,
    ];

    let rows: Vec<(UploadFault, Route)> = EXPECTATIONS
        .iter()
        .map(|(at, route, _, _)| (*at, *route))
        .collect();
    assert_eq!(rows.len(), 11, "期待値表は 11 行");
    for (i, row) in rows.iter().enumerate() {
        assert!(
            !rows[..i].contains(row),
            "期待値表に重複行がある: {row:?}（14 ではなく 11 になるのは重複ではなく除外による）"
        );
    }

    for at in ALL_FAULTS {
        assert!(
            rows.contains(&(at, Route::ShapeChange)),
            "外形変更経路は 7 点すべてを持つ（欠け: {at:?}）"
        );
        let unchanged = rows.contains(&(at, Route::ShapeUnchanged));
        if SHAPE_CHANGE_ONLY.contains(&at) {
            assert!(
                !unchanged,
                "寸法変更点 {at:?} は外形不変経路では踏まれないので表に載せない"
            );
        } else {
            assert!(unchanged, "外形不変経路に {at:?} の行が無い");
        }
    }
}

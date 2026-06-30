//! 先進坑: pilot-clickthrough-alpha-toggle
//!
//! 対応 spec: `.kiro/specs/pilot-clickthrough-alpha-toggle/`
//! 一次記録（動機・概要・検証結果）は隣の README.md（3 幕）を正本とする。
//! T1〜T8 の詳細台帳は REPORT.md。
//!
//! 実行法: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`
//!
//! 本ファイルは段階的に肉付けされる。タスク 1.1 時点では「PMv2 DPI 認識の設定 ＋
//! 起動ログ」の最小起動骨組みのみを担う（窓生成は 2.2、カーソルワーカは 3.x、
//! ライフサイクルは 4.1 が後続で追加する）。葉ノード隔離（examples 配下のみ・
//! inbound 依存ゼロ）は厳守する。

use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};

/// プロセス全体を Per-Monitor-Aware V2 に設定する（R7.1）。
///
/// PMv2 認識プロセスでは `GetCursorPos`/`GetWindowRect` が仮想スクリーンの物理ピクセル
/// 座標を返すため、後続タスクの円判定（αマスク）と描画を同一物理座標基準で一致させられる。
/// 先進坑ゆえ失敗してもプロセスは継続するが、検証前提（T7）に直結するため `let _ =` で
/// 握らず成否をログ出力する。
fn init_dpi_awareness() {
    // SAFETY: プロセス起動直後（他スレッドが DPI に依存する前）に一度だけ呼ぶ。
    let result = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    match result {
        Ok(()) => {
            println!("DPI: PER_MONITOR_AWARE_V2 を設定しました。");
        }
        Err(e) => {
            // 既に manifest 等で設定済みの場合などに失敗し得る。検証時の前提確認のため警告を残す。
            eprintln!("警告: PER_MONITOR_AWARE_V2 の設定に失敗しました: {e}");
        }
    }
}

fn main() {
    println!("=== pilot: clickthrough-alpha-toggle 先進坑 ===");
    init_dpi_awareness();
}

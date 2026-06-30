//! 先進坑: pilot-clickthrough-alpha-toggle
//!
//! 対応 spec: `.kiro/specs/pilot-clickthrough-alpha-toggle/`
//! 一次記録（動機・概要・検証結果）は隣の README.md を正本とする。
//! T1〜T8 の詳細台帳と REPORT.md はタスク 6.1 で作成する（本ファイルは骨組みのみ）。
//!
//! 実行法: `cargo run -p pilot --example pilot-clickthrough-alpha-toggle`
//!
//! 視覚的透過は DirectComposition（DComp）visual tree の per-pixel α を前提とする
//! （窓は `WS_EX_NOREDIRECTIONBITMAP` で生成するため GDI/`WM_PAINT` は画面に出ない）。
//! 葉ノード隔離（examples 配下のみ・inbound 依存ゼロ）は厳守する。

use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};

/// プロセスを Per-Monitor Aware v2（PMv2）に設定する（R7.1）。
///
/// PMv2 では `GetCursorPos`/`GetWindowRect` がともに物理スクリーン座標を返すため、
/// 後続タスクの円判定（物理座標一致・T7 の前提）が成立する。失敗は握り潰さず
/// 警告ログを残す（T7 の前提証跡。設計 Error Handling 参照）。
fn init_dpi_awareness() {
    // SAFETY: Win32 境界。`SetProcessDpiAwarenessContext` はプロセスグローバルな
    // DPI awareness をスレッドセーフに設定する。プロセス起動直後・他スレッド／
    // DPI 依存処理の前に一度だけ呼ぶ（main 冒頭）。
    let result = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    match result {
        Ok(()) => println!("[dpi] SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2): Ok"),
        Err(e) => eprintln!(
            "[dpi][warn] SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2) failed: {e}"
        ),
    }
}

fn main() {
    init_dpi_awareness();
    println!("=== pilot: clickthrough-alpha-toggle 先進坑 ===");

    // 窓生成・DComp パイプライン・カーソルワーカ・トグル制御は後続タスク
    // （2.x/3.x/4.x/5.x）で実装する。本タスクは PMv2 起動骨組み＋起動ログのみ。
}

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

use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};

/// 不透明円の半径（物理ピクセル）。
///
/// αマスク判定円（本関数）と、後続タスク 2.2 の GDI 描画円は同一値を共有することで
/// 「見た目の円」と「判定の円」を一致させる（R2.2／R4.1）。共有のため定数化する。
const RADIUS: i32 = 200;

/// カーソルが不透明円の内側にあれば true（クリックスルー OFF を意味する）。
///
/// 引数はすべて物理スクリーン座標（PMv2 前提）。純関数・副作用なし。
///
/// これは**差し替えシーム**（R4.2）である。先進坑では仮の円判定を返すだけで、
/// 実描画αバッファ参照は実装しない（R4.3）。将来、本坑 `wintf-clickthrough-alpha-toggle`
/// が同一シグネチャのまま中身を実描画αバッファ参照へ差し替える想定。
///
/// 円中心は窓矩形の中心から実算出する（R4.4／R7.2）。プライマリモニタ専用の
/// 固定スクリーン座標（旧 `(960, 540)`）は前提にしない。
//
// 後続タスク 3.x（カーソル監視ワーカ）が本関数を呼び出すまでは未使用となるため
// `dead_code` を許容する（非テストビルドの警告抑止）。
#[allow(dead_code)]
fn alpha_is_opaque(cursor: POINT, win_rect: RECT) -> bool {
    // 円中心 = 窓矩形の中心（物理座標）。i32 の加算オーバーフロー回避のため i64 で算出する。
    let cx = (win_rect.left as i64 + win_rect.right as i64) / 2;
    let cy = (win_rect.top as i64 + win_rect.bottom as i64) / 2;
    let dx = cursor.x as i64 - cx;
    let dy = cursor.y as i64 - cy;
    let r = RADIUS as i64;
    // dx*dx + dy*dy <= r*r なら円内＝不透明＝true。二乗は i64 で行いオーバーフローを回避する。
    dx * dx + dy * dy <= r * r
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用ヘルパ: 中心が `(cx, cy)`・一辺 `side` の窓矩形を物理座標で作る。
    /// 中心相対判定であること（固定座標非前提）を別位置で検証するために中心を可変にする。
    fn rect_centered(cx: i32, cy: i32, side: i32) -> RECT {
        let half = side / 2;
        RECT {
            left: cx - half,
            top: cy - half,
            right: cx + half,
            bottom: cy + half,
        }
    }

    fn pt(x: i32, y: i32) -> POINT {
        POINT { x, y }
    }

    #[test]
    fn center_is_opaque() {
        // 円中心ちょうど＝不透明＝true（R4.1）。
        let win = rect_centered(1000, 700, 600);
        assert!(alpha_is_opaque(pt(1000, 700), win));
    }

    #[test]
    fn inside_radius_is_opaque() {
        // 中心から 199px（半径 200 の内側）＝true。
        let win = rect_centered(1000, 700, 600);
        assert!(alpha_is_opaque(pt(1000 + 199, 700), win));
    }

    #[test]
    fn on_radius_boundary_is_opaque() {
        // 中心から 200px ちょうど＝境界含む（<=）＝true。
        let win = rect_centered(1000, 700, 600);
        assert!(alpha_is_opaque(pt(1000, 700 + RADIUS), win));
    }

    #[test]
    fn outside_radius_is_transparent() {
        // 中心から 201px（半径 200 の外側）＝false。
        let win = rect_centered(1000, 700, 600);
        assert!(!alpha_is_opaque(pt(1000 + 201, 700), win));
    }

    #[test]
    fn judged_relative_to_window_center_not_fixed_coords() {
        // 窓を別位置（負座標を含むマルチモニタ想定）へ置いても、判定は窓中心相対で行われ、
        // プライマリ専用固定座標 (960,540) を前提にしないこと（R4.4／R7.3）。
        let win = rect_centered(-1500, -300, 600);
        // 別位置の窓中心は不透明。
        assert!(alpha_is_opaque(pt(-1500, -300), win));
        // 同じ相対オフセット（中心 +201px）は外側＝透明。
        assert!(!alpha_is_opaque(pt(-1500 + 201, -300), win));
        // 旧固定座標 (960,540) はこの窓の円内ではない＝透明（固定前提でないことの裏付け）。
        assert!(!alpha_is_opaque(pt(960, 540), win));
    }

    #[test]
    fn judged_with_asymmetric_window_rect() {
        // 非正方形の窓でも矩形中心 ((left+right)/2,(top+bottom)/2) で判定されること。
        let win = RECT {
            left: 100,
            top: 200,
            right: 1100, // 中心 x = 600
            bottom: 600, // 中心 y = 400
        };
        assert!(alpha_is_opaque(pt(600, 400), win)); // 中心
        assert!(!alpha_is_opaque(pt(600, 400 + 201), win)); // 外側
    }
}

//! # WM_NCHITTEST キャッシュ
//!
//! WM_NCHITTEST の高頻度呼び出しに対するパフォーマンス最適化を提供する。
//! 同一座標での重複ヒットテストをスキップし、World 借用オーバーヘッドを削減する。
//!
//! ## 設計
//! - thread_local! + RefCell パターンで内部可変性を提供
//! - HWND をキーとしたエントリ管理（HashMap）
//! - try_tick_world() 終了時に全エントリをクリア

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use tracing::trace;
use windows::Win32::Foundation::{HWND, LRESULT, POINT};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, KillTimer, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SetTimer, SetWindowLongPtrW, SetWindowPos,
};

use crate::ecs::layout::hit_test::{PhysicalPoint, hit_test_in_window};
use crate::ecs::world::EcsWorld;

// HTCLIENT = 1, HTTRANSPARENT = -1
const HTCLIENT: i32 = 1;
/// WM_NCHITTEST でクライアント領域外（透明領域）を示す定数。
/// WM_MOUSELEAVE ハンドラ実装済み（handlers.rs L820-876）により、
/// HTTRANSPARENT 返却後も PointerState は正常にクリーンアップされる。
/// ドラッグ中は DragState ガードで HTCLIENT を強制返却する。
const HTTRANSPARENT: i32 = -1;

/// クリックスルー復帰チェック用タイマーID
///
/// WS_EX_TRANSPARENT 設定中、マウス位置を監視してヒット可能領域に
/// 入ったらスタイルを解除するためのタイマー。
pub(crate) const CLICK_THROUGH_TIMER_ID: usize = 0xC71C; // "CLTC" mnemonic

/// クリックスルータイマーの間隔（ミリ秒）
/// 16ms ≒ 60fps のマウス位置チェック
const CLICK_THROUGH_TIMER_INTERVAL_MS: u32 = 16;

// ============================================================================
// キャッシュエントリ
// ============================================================================

/// キャッシュエントリ
struct NchittestCacheEntry {
    /// スクリーン座標（物理ピクセル）
    screen_point: (i32, i32),
    /// WM_NCHITTEST 戻り値
    lresult: LRESULT,
}

// ============================================================================
// スレッドローカルキャッシュ
// ============================================================================

thread_local! {
    /// HWND ごとの WM_NCHITTEST 結果キャッシュ
    static NCHITTEST_CACHE: RefCell<HashMap<isize, NchittestCacheEntry>>
        = RefCell::new(HashMap::new());
}

// ============================================================================
// キャッシュ操作API
// ============================================================================

/// キャッシュルックアップ
fn lookup(hwnd: HWND, screen_point: (i32, i32)) -> Option<LRESULT> {
    NCHITTEST_CACHE.with(|cache| {
        let cache = cache.borrow();
        if let Some(entry) = cache.get(&(hwnd.0 as isize)) {
            if entry.screen_point == screen_point {
                return Some(entry.lresult);
            }
        }
        None
    })
}

/// キャッシュ挿入
fn insert(hwnd: HWND, screen_point: (i32, i32), lresult: LRESULT) {
    NCHITTEST_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.insert(
            hwnd.0 as isize,
            NchittestCacheEntry {
                screen_point,
                lresult,
            },
        );
    });
}

// ============================================================================
// 公開API
// ============================================================================

/// キャッシュ付き WM_NCHITTEST 処理
///
/// # Arguments
/// - `hwnd`: ウィンドウハンドル
/// - `screen_point`: スクリーン座標（lparam から取得済み）
/// - `entity`: ウィンドウエンティティ
/// - `ecs_world`: ECS World（借用元）
///
/// # Returns
/// - `Some(LRESULT)`: HTCLIENT (1) または HTTRANSPARENT (-1)
/// - `None`: 処理失敗時（DefWindowProcW に委譲）
pub fn cached_nchittest(
    hwnd: HWND,
    screen_point: (i32, i32),
    entity: bevy_ecs::prelude::Entity,
    ecs_world: &Rc<RefCell<EcsWorld>>,
) -> Option<LRESULT> {
    // キャッシュヒット判定
    if let Some(lresult) = lookup(hwnd, screen_point) {
        trace!(
            hwnd = ?hwnd,
            x = screen_point.0,
            y = screen_point.1,
            lresult = lresult.0,
            "NCHITTEST cache hit"
        );
        return Some(lresult);
    }

    // キャッシュミス: クライアント座標に変換
    let mut pt = POINT {
        x: screen_point.0,
        y: screen_point.1,
    };
    // SAFETY: ScreenToClient は HWND と POINT への有効なポインタを必要とする
    if unsafe { !ScreenToClient(hwnd, &mut pt).as_bool() } {
        return None;
    }

    // World 借用して hit_test 実行
    let hit_result = match ecs_world.try_borrow() {
        Ok(world_ref) => hit_test_in_window(
            world_ref.world(),
            entity,
            PhysicalPoint::new(pt.x as f32, pt.y as f32),
        ),
        Err(_) => {
            return None; // 借用失敗時は DefWindowProcW に委譲
        }
    };

    // DragState ガード: ドラッグ中は透明領域でも HTCLIENT を強制返却
    // （ドラッグ操作の継続性を保証するため）
    let is_dragging = crate::ecs::drag::read_drag_state(|state| {
        matches!(
            state,
            crate::ecs::drag::DragState::Preparing { .. }
                | crate::ecs::drag::DragState::JustStarted { .. }
                | crate::ecs::drag::DragState::Dragging { .. }
        )
    });

    // WM_MOUSELEAVE ハンドラ実装済み（handlers.rs L820-876）により、
    // HTTRANSPARENT 返却後も PointerState は正常にクリーンアップされる。
    // ドラッグ中は DragState ガードで HTCLIENT を強制返却する。
    let lresult = if is_dragging || hit_result.is_some() {
        LRESULT(HTCLIENT as isize)
    } else {
        LRESULT(HTTRANSPARENT as isize)
    };

    // キャッシュに挿入
    insert(hwnd, screen_point, lresult);

    // WS_EX_TRANSPARENT トグル:
    // HTTRANSPARENT はクロスプロセスでは機能しないため、
    // Win32 ヒットテストレベルで透過させる WS_EX_TRANSPARENT を動的に設定する。
    // タイマーで復帰をチェックし、ヒット可能になったら解除する。
    if lresult.0 == HTTRANSPARENT as isize {
        enable_click_through(hwnd);
    }

    trace!(
        hwnd = ?hwnd,
        x = screen_point.0,
        y = screen_point.1,
        lresult = lresult.0,
        hit_entity = ?hit_result,
        "NCHITTEST cache miss"
    );

    Some(lresult)
}

/// キャッシュをクリア
///
/// try_tick_world() 終了時に呼び出す。
/// 全ウィンドウのキャッシュエントリを削除する。
pub fn clear_nchittest_cache() {
    NCHITTEST_CACHE.with(|cache| {
        cache.borrow_mut().clear();
    });
}

// ============================================================================
// WS_EX_TRANSPARENT 動的トグル
// ============================================================================

// スレッドローカル: WS_EX_TRANSPARENT が有効な HWND セット
thread_local! {
    static TRANSPARENT_WINDOWS: RefCell<std::collections::HashSet<isize>>
        = RefCell::new(std::collections::HashSet::new());
}

// 再入防止ガード: SetWindowPos(SWP_FRAMECHANGED) 呼び出し中に true
// WM_WINDOWPOSCHANGED ハンドラでこのフラグをチェックして副作用を抑制する
thread_local! {
    static CLICK_THROUGH_STYLE_UPDATE: Cell<bool> = Cell::new(false);
}

/// click-through のスタイル更新中かどうかを返す
///
/// `WM_WINDOWPOSCHANGED` ハンドラで使用。
/// `SWP_FRAMECHANGED` 由来の `WM_WINDOWPOSCHANGED` は位置/サイズ変更がないため、
/// tick やレイアウト再計算をスキップすべき。
pub(crate) fn is_click_through_style_update() -> bool {
    CLICK_THROUGH_STYLE_UPDATE.with(|f| f.get())
}

/// WS_EX_TRANSPARENT を有効化してタイマーを開始する
///
/// HTTRANSPARENT がクロスプロセスで機能しないため、
/// Win32 ヒットテストレベルでウィンドウを透過させる。
fn enable_click_through(hwnd: HWND) {
    TRANSPARENT_WINDOWS.with(|set| {
        let mut set = set.borrow_mut();
        if set.contains(&(hwnd.0 as isize)) {
            return; // 既に有効
        }
        set.insert(hwnd.0 as isize);

        unsafe {
            // 現在の ex_style に WS_EX_TRANSPARENT をOR
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_ex_style = ex_style | 0x20; // WS_EX_TRANSPARENT = 0x20
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style);

            // SWP_FRAMECHANGED で DWM にスタイル変更を通知（必須）
            // 再入防止ガード: WM_WINDOWPOSCHANGED が同期発火するので、
            // ハンドラ側で try_tick_on_vsync 等の副作用を抑制する
            CLICK_THROUGH_STYLE_UPDATE.with(|f| f.set(true));
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            CLICK_THROUGH_STYLE_UPDATE.with(|f| f.set(false));

            // 復帰チェック用タイマー開始
            let _ = SetTimer(
                Some(hwnd),
                CLICK_THROUGH_TIMER_ID,
                CLICK_THROUGH_TIMER_INTERVAL_MS,
                None,
            );

            println!(
                "[click-through] ENABLED  WS_EX_TRANSPARENT for HWND {:?}, ex_style=0x{:X}",
                hwnd,
                GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
            );
        }

        trace!(hwnd = ?hwnd, "click-through: WS_EX_TRANSPARENT enabled, timer started");
    });
}

/// WS_EX_TRANSPARENT を解除してタイマーを停止する
fn disable_click_through(hwnd: HWND) {
    TRANSPARENT_WINDOWS.with(|set| {
        let mut set = set.borrow_mut();
        if !set.remove(&(hwnd.0 as isize)) {
            return; // 既に無効
        }

        unsafe {
            // WS_EX_TRANSPARENT ビットをクリア
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let new_ex_style = ex_style & !0x20; // WS_EX_TRANSPARENT = 0x20
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style);

            // SWP_FRAMECHANGED で DWM にスタイル変更を通知（必須）
            CLICK_THROUGH_STYLE_UPDATE.with(|f| f.set(true));
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            CLICK_THROUGH_STYLE_UPDATE.with(|f| f.set(false));

            // タイマー停止
            let _ = KillTimer(Some(hwnd), CLICK_THROUGH_TIMER_ID);

            println!(
                "[click-through] DISABLED WS_EX_TRANSPARENT for HWND {:?}, ex_style=0x{:X}",
                hwnd,
                GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
            );
        }

        trace!(hwnd = ?hwnd, "click-through: WS_EX_TRANSPARENT disabled, timer stopped");
    });
}

/// WM_TIMER ハンドラ: クリックスルー復帰チェック
///
/// マウス位置を取得し、ヒットテストを実行。
/// ヒット可能エンティティがあれば WS_EX_TRANSPARENT を解除する。
///
/// # Returns
/// - `Some(LRESULT(0))`: タイマー処理完了
/// - `None`: 対象外のタイマーID（DefWindowProcW に委譲）
pub fn on_click_through_timer(
    hwnd: HWND,
    timer_id: usize,
    entity: bevy_ecs::prelude::Entity,
    ecs_world: &Rc<RefCell<EcsWorld>>,
) -> Option<LRESULT> {
    if timer_id != CLICK_THROUGH_TIMER_ID {
        return None;
    }

    // マウスカーソルのスクリーン座標を取得
    let mut cursor_pos = POINT { x: 0, y: 0 };
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        if GetCursorPos(&mut cursor_pos).is_err() {
            return Some(LRESULT(0));
        }
    }

    // クライアント座標に変換
    let mut client_pt = cursor_pos;
    if unsafe { !ScreenToClient(hwnd, &mut client_pt).as_bool() } {
        // 変換失敗 → ウィンドウ外とみなして維持
        println!("[click-through-timer] ScreenToClient failed, keeping transparent");
        return Some(LRESULT(0));
    }

    // クライアント領域判定: 領域外の場合は WS_EX_TRANSPARENT を維持
    // カーソルが非クライアント領域（タイトルバー等）やウィンドウ外に出ても、
    // インタラクティブなエンティティ上にない限りは透過を維持する。
    // （デスクトップマスコット用途: 非クライアント領域も透過が正しい動作）
    let mut rect = windows::Win32::Foundation::RECT::default();
    if unsafe { windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect).is_err() } {
        println!("[click-through-timer] GetClientRect failed, keeping transparent");
        return Some(LRESULT(0));
    }
    if client_pt.x < rect.left
        || client_pt.x >= rect.right
        || client_pt.y < rect.top
        || client_pt.y >= rect.bottom
    {
        // クライアント領域外 → WS_EX_TRANSPARENT 維持（タイマー継続）
        println!(
            "[click-through-timer] cursor outside client rect ({},{}) rect=({},{},{},{}), keeping transparent",
            client_pt.x, client_pt.y, rect.left, rect.top, rect.right, rect.bottom
        );
        return Some(LRESULT(0));
    }

    // ドラッグ中チェック
    let is_dragging = crate::ecs::drag::read_drag_state(|state| {
        matches!(
            state,
            crate::ecs::drag::DragState::Preparing { .. }
                | crate::ecs::drag::DragState::JustStarted { .. }
                | crate::ecs::drag::DragState::Dragging { .. }
        )
    });
    if is_dragging {
        println!("[click-through-timer] dragging, disabling");
        disable_click_through(hwnd);
        return Some(LRESULT(0));
    }

    // ECS ヒットテスト実行
    let hit_result = match ecs_world.try_borrow() {
        Ok(world_ref) => hit_test_in_window(
            world_ref.world(),
            entity,
            PhysicalPoint::new(client_pt.x as f32, client_pt.y as f32),
        ),
        Err(_) => {
            println!("[click-through-timer] world borrow failed, keeping transparent");
            None
        }
    };

    if hit_result.is_some() {
        // ヒット可能エンティティあり → WS_EX_TRANSPARENT 解除
        println!(
            "[click-through-timer] hit_test returned {:?} at client({},{}), disabling",
            hit_result, client_pt.x, client_pt.y
        );
        disable_click_through(hwnd);
        // NCHITTEST キャッシュをクリア（次の WM_NCHITTEST で再評価させる）
        NCHITTEST_CACHE.with(|cache| {
            cache.borrow_mut().remove(&(hwnd.0 as isize));
        });
    } else {
        // ヒットなし → WS_EX_TRANSPARENT 維持、タイマー継続
        println!(
            "[click-through-timer] no hit at client({},{}), keeping transparent",
            client_pt.x, client_pt.y
        );
    }

    Some(LRESULT(0))
}

/// WS_EX_TRANSPARENT が有効かどうかを返す（テスト用）
#[cfg(test)]
pub fn is_click_through_active(hwnd: HWND) -> bool {
    TRANSPARENT_WINDOWS.with(|set| set.borrow().contains(&(hwnd.0 as isize)))
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// キャッシュルックアップとインサートの基本動作
    #[test]
    fn test_cache_lookup_insert() {
        // テスト用のダミー HWND
        let hwnd = HWND(12345 as *mut std::ffi::c_void);
        let screen_point = (100, 200);
        let lresult = LRESULT(1);

        // キャッシュが空の状態でルックアップ
        assert!(lookup(hwnd, screen_point).is_none());

        // キャッシュに挿入
        insert(hwnd, screen_point, lresult);

        // 同じ座標でルックアップ
        assert_eq!(lookup(hwnd, screen_point), Some(lresult));

        // 異なる座標でルックアップ
        assert!(lookup(hwnd, (101, 200)).is_none());

        // クリーンアップ
        clear_nchittest_cache();
    }

    /// 異なる HWND で独立したキャッシュ
    #[test]
    fn test_cache_multiple_hwnds() {
        let hwnd1 = HWND(111 as *mut std::ffi::c_void);
        let hwnd2 = HWND(222 as *mut std::ffi::c_void);
        let screen_point = (50, 50);
        let lresult1 = LRESULT(1);
        let lresult2 = LRESULT(-1);

        insert(hwnd1, screen_point, lresult1);
        insert(hwnd2, screen_point, lresult2);

        assert_eq!(lookup(hwnd1, screen_point), Some(lresult1));
        assert_eq!(lookup(hwnd2, screen_point), Some(lresult2));

        // クリーンアップ
        clear_nchittest_cache();
    }

    /// キャッシュクリアの動作確認
    #[test]
    fn test_cache_clear() {
        let hwnd = HWND(999 as *mut std::ffi::c_void);
        let screen_point = (10, 20);
        let lresult = LRESULT(1);

        insert(hwnd, screen_point, lresult);
        assert!(lookup(hwnd, screen_point).is_some());

        clear_nchittest_cache();
        assert!(lookup(hwnd, screen_point).is_none());
    }

    /// キャッシュ更新の動作確認
    #[test]
    fn test_cache_update() {
        let hwnd = HWND(777 as *mut std::ffi::c_void);
        let point1 = (100, 100);
        let point2 = (200, 200);
        let lresult1 = LRESULT(1);
        let lresult2 = LRESULT(-1);

        // 最初の座標を挿入
        insert(hwnd, point1, lresult1);
        assert_eq!(lookup(hwnd, point1), Some(lresult1));

        // 異なる座標で上書き
        insert(hwnd, point2, lresult2);
        // 古い座標はヒットしない
        assert!(lookup(hwnd, point1).is_none());
        // 新しい座標がヒット
        assert_eq!(lookup(hwnd, point2), Some(lresult2));

        // クリーンアップ
        clear_nchittest_cache();
    }

    /// HTTRANSPARENT がキャッシュに格納・取得できることを検証
    #[test]
    fn test_cache_httransparent_storage() {
        let hwnd = HWND(5001 as *mut std::ffi::c_void);
        let screen_point = (300, 400);
        let httransparent = LRESULT(HTTRANSPARENT as isize);

        // HTTRANSPARENT を挿入
        insert(hwnd, screen_point, httransparent);

        // 同じ座標で HTTRANSPARENT が取得できる
        let result = lookup(hwnd, screen_point);
        assert_eq!(result, Some(httransparent));
        assert_eq!(result.unwrap().0, -1);

        // クリーンアップ
        clear_nchittest_cache();
    }

    /// HTCLIENT と HTTRANSPARENT が異なる HWND で共存できることを検証
    #[test]
    fn test_cache_htclient_and_httransparent_coexist() {
        let hwnd1 = HWND(6001 as *mut std::ffi::c_void);
        let hwnd2 = HWND(6002 as *mut std::ffi::c_void);
        let screen_point = (150, 250);
        let htclient = LRESULT(HTCLIENT as isize);
        let httransparent = LRESULT(HTTRANSPARENT as isize);

        insert(hwnd1, screen_point, htclient);
        insert(hwnd2, screen_point, httransparent);

        assert_eq!(lookup(hwnd1, screen_point), Some(htclient));
        assert_eq!(lookup(hwnd2, screen_point), Some(httransparent));

        // クリーンアップ
        clear_nchittest_cache();
    }
}

//! ウィンドウ位置・サイズ・DPI変更メッセージハンドラ
//!
//! WM_WINDOWPOSCHANGED, WM_DPICHANGED の処理を担当する。
//! DPI変更時のセンター座標補正ロジックもこのモジュールに含まれる。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::Entity;
use tracing::{debug, trace, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// WM_WINDOWPOSCHANGED: ウィンドウ位置/サイズ変更通知
///
/// World借用区切り方式による処理（3ステッププロトコル）:
/// ① World借用 → echo判定に基づきWindowPos更新, BoxStyle更新 → 借用解放
/// ② try_tick_on_vsync() (内部で借用→解放)
/// ③ flush_window_pos_commands() (SetWindowPos実行、ラッパー経由)
///
/// echo判定: `is_self_initiated()` TLS フラグが `true` の場合、
/// 自アプリの `guarded_set_window_pos()` 経由の呼び出し。
///
/// BoxStyle.size スキップ条件:
/// - `is_echo || dpi_context.is_some()` → スキップ
/// - 外部リサイズ時のみ: 物理px / DPI.scale → 論理px に変換して更新
#[inline]
pub(super) fn WM_WINDOWPOSCHANGED(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    // echo 判定: TLS フラグを参照（ステップ①冒頭で1回のみ）
    let is_echo = crate::ecs::window::is_self_initiated();

    // ------------------------------------------------------------------
    // ① 第1借用セクション: DPI更新, echo判定に基づきWindowPos更新, BoxStyle更新
    // ------------------------------------------------------------------
    {
        {
            // DpiChangeContextを先に取得（try_tick_on_vsync前に消費する必要がある）
            // is_echo にかかわらず常に実行
            let dpi_context = crate::ecs::window::DpiChangeContext::take();

            // RefCellが既に借用されている場合はスキップ（再入時）
            if let Ok(mut world_borrow) = world.try_borrow_mut() {
                let windowpos = lparam.0 as *const WINDOWPOS;
                if !windowpos.is_null() {
                    let wp = unsafe { &*windowpos };

                    if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(entity) {
                        // DPI コンポーネントの読み取り（更新は WM_DPICHANGED で直接実行済み）
                        // DpiChangeContext の読み取りは echo bypass / BoxStyle skip の判定にのみ使用
                        let dpi = entity_ref
                            .get::<crate::ecs::window::DPI>()
                            .copied()
                            .unwrap_or_default();

                        // WindowHandleを取得してウィンドウ座標→クライアント座標に変換
                        let client_coords = entity_ref
                            .get::<crate::ecs::window::WindowHandle>()
                            .and_then(|handle| {
                                handle
                                    .window_to_client_coords(wp.x, wp.y, wp.cx, wp.cy)
                                    .ok()
                            });

                        // クライアント座標が取得できた場合のみ処理
                        if let Some((client_pos, client_size)) = client_coords {
                            debug!(
                                is_echo = is_echo,
                                has_dpi_ctx = dpi_context.is_some(),
                                entity = ?entity,
                                window_xy = format_args!("({},{})", wp.x, wp.y),
                                window_size = format_args!("{}x{}", wp.cx, wp.cy),
                                client_xy = format_args!("({},{})", client_pos.x, client_pos.y),
                                client_size = format_args!("{}x{}", client_size.width, client_size.height),
                                dpi = format_args!("{:.2}", dpi.scale_x()),
                                "[WM_WINDOWPOSCHANGED]"
                            );

                            // BoxStyle のスナップショットを WindowPos の mutable borrow 前に取得
                            // （借用チェッカー制約: entity_ref の immutable と mutable は共存不可）
                            let box_style_snapshot =
                                entity_ref.get::<crate::ecs::layout::BoxStyle>().cloned();

                            if let Some(mut window_pos) =
                                entity_ref.get_mut::<crate::ecs::window::WindowPos>()
                            {
                                // DPI変更時の特別処理:
                                // DpiChangeContext がある場合は echo でも bypass しない。
                                // bypass すると Changed<WindowPos> が発火せず、
                                // sync_window_arrangement_from_window_pos が新位置を
                                // Arrangement.offset に反映できない。結果として
                                // update_arrangements_system が旧 offset を保持したまま
                                // 新 DPI スケールを適用 → 誤ったグローバル座標 →
                                // window_pos_sync_system が旧位置へ SetWindowPos →
                                // 旧モニタに戻る → 再び WM_DPICHANGED → 無限ループ (フリーズ)
                                let use_bypass = is_echo && dpi_context.is_none();

                                if use_bypass {
                                    // echo（自アプリ由来、DPI変更なし）→ bypass_change_detection で更新
                                    // Changed<WindowPos> を発火させない → apply_window_pos_changes 非トリガー
                                    let bypass = window_pos.bypass_change_detection();
                                    bypass.position = Some(client_pos);
                                    bypass.size = Some(client_size);

                                    trace!(
                                        entity = ?entity,
                                        client_x = client_pos.x,
                                        client_y = client_pos.y,
                                        "WindowPos updated via bypass (echo, no DPI change)"
                                    );
                                } else {
                                    // 外部由来: 値が実際に変化した場合のみ DerefMut で更新
                                    // Changed<WindowPos> → apply_window_pos_changes トリガー
                                    //
                                    // DPI 変更時の中心保持補正:
                                    // サイズ変化に伴うウィンドウ中心座標のズレを防止する。
                                    // dpi_context が None の場合は補正なし（client_pos をそのまま返す）。
                                    let corrected_pos = super::dpi_helpers::correct_position_for_dpi_center_preserve(
                                        client_pos,
                                        client_size,
                                        &dpi_context,
                                        box_style_snapshot.as_ref(),
                                        &dpi,
                                    );

                                    // 値ガード: ウィンドウアクティベーション等で WM_WINDOWPOSCHANGED が
                                    // 発火しても、座標/サイズが同一なら Changed を発火させない。
                                    // これにより不要な SetWindowPos エコーバックループを防止し、
                                    // 高DPI環境でのフレームオフセット不一致による位置ズレを回避する。
                                    let pos_changed = window_pos.position != Some(corrected_pos);
                                    let size_changed = window_pos.size != Some(client_size);

                                    if pos_changed || size_changed {
                                        window_pos.position = Some(corrected_pos);
                                        window_pos.size = Some(client_size);

                                        if dpi_context.is_some() {
                                            debug!(
                                                entity = ?entity,
                                                is_echo,
                                                original_x = client_pos.x,
                                                original_y = client_pos.y,
                                                corrected_x = corrected_pos.x,
                                                corrected_y = corrected_pos.y,
                                                client_cx = client_size.width,
                                                client_cy = client_size.height,
                                                "[WM_WINDOWPOSCHANGED] WindowPos updated (DPI change, center-preserve)"
                                            );
                                        } else {
                                            debug!(
                                                entity = ?entity,
                                                window_x = wp.x,
                                                window_y = wp.y,
                                                window_cx = wp.cx,
                                                window_cy = wp.cy,
                                                client_x = client_pos.x,
                                                client_y = client_pos.y,
                                                client_cx = client_size.width,
                                                client_cy = client_size.height,
                                                "WindowPos updated (external change, values differ)"
                                            );
                                        }
                                    } else {
                                        trace!(
                                            entity = ?entity,
                                            client_x = client_pos.x,
                                            client_y = client_pos.y,
                                            "WindowPos unchanged (external, same values — skipping DerefMut)"
                                        );
                                    }
                                }
                            }

                            // BoxStyle.size のサイズ変更判定と条件付き更新
                            // BoxStyle.inset への書き込みは行わない（Window位置はWindowPosが唯一のsource of truth）
                            //
                            // skip_box_style = is_echo || dpi_context.is_some()
                            // - echo（自アプリ由来、DPI変更なし）: ループ防止のためスキップ
                            // - DPI変更時: BoxStyle.size は不変（レイアウトシステム主導でサイズ決定）
                            // - 外部リサイズ時のみ: 物理px / DPI.scale → 論理px に変換して更新
                            let skip_box_style = is_echo || dpi_context.is_some();
                            if !skip_box_style {
                                use crate::ecs::layout::{BoxSize, Dimension};

                                let physical_width = client_size.width as f32;
                                let physical_height = client_size.height as f32;

                                // 物理ピクセルを DPI スケールで除算して論理ピクセルに変換
                                // BoxStyle は論理 px（96 DPI / 100% 相当）を唯一の座標系とする
                                let logical_width = physical_width / dpi.scale_x();
                                let logical_height = physical_height / dpi.scale_y();

                                let new_size = Some(BoxSize {
                                    width: Some(Dimension::Px(logical_width)),
                                    height: Some(Dimension::Px(logical_height)),
                                });

                                // Step 1: 現在のサイズを読み取り（immutable borrow）
                                let current_size = entity_ref
                                    .get::<crate::ecs::layout::BoxStyle>()
                                    .map(|bs| bs.size);

                                // Step 2: サイズ変更がある場合のみ get_mut で更新（Changed<BoxStyle> 発火）
                                let size_changed =
                                    current_size.map(|cs| cs != new_size).unwrap_or(false);

                                if size_changed {
                                    if let Some(mut box_style) =
                                        entity_ref.get_mut::<crate::ecs::layout::BoxStyle>()
                                    {
                                        box_style.size = new_size;
                                    }

                                    debug!(
                                        entity = ?entity,
                                        logical_width = logical_width,
                                        logical_height = logical_height,
                                        physical_width = physical_width,
                                        physical_height = physical_height,
                                        dpi_scale = dpi.scale_x(),
                                        "[WM_WINDOWPOSCHANGED] BoxStyle.size updated (logical px, external resize)"
                                    );
                                } else {
                                    trace!(
                                        entity = ?entity,
                                        "[WM_WINDOWPOSCHANGED] BoxStyle.size unchanged, skipping update"
                                    );
                                }
                            } else {
                                trace!(
                                    entity = ?entity,
                                    is_echo = is_echo,
                                    has_dpi_ctx = dpi_context.is_some(),
                                    "[WM_WINDOWPOSCHANGED] BoxStyle.size skipped (echo or DPI change)"
                                );
                            }
                        }
                    }
                }
            }
            // world_borrowスコープ終了: 借用解放

            // ------------------------------------------------------------------
            // ② try_tick_on_vsync() (内部で借用→解放)
            // ------------------------------------------------------------------
            {
                use crate::ecs::world::VsyncTick;
                let _ = world.try_tick_on_vsync();
            }

            // ------------------------------------------------------------------
            // ③ flush_window_pos_commands() (SetWindowPos実行、ラッパー経由)
            // World借用解放後なので安全
            // ------------------------------------------------------------------
            crate::ecs::window::flush_window_pos_commands();
        }
    }
    None // DefWindowProcWに委譲
}

/// WM_DPICHANGED: DPI変更通知（モニター間移動など）
///
/// Per-Monitor DPI Aware (v2)では、アプリケーションが明示的にSetWindowPosを呼ぶ必要がある。
/// レイアウトシステム主導方式: DPI コンポーネントを直接更新し、SWP_NOSIZE を維持して
/// 位置のみ SetWindowPos。サイズは ECS レイアウトパイプラインが算出する。
///
/// ## 処理順序
/// ① World borrow: DPI コンポーネントを new_dpi に直接更新（Changed<DPI> 発火）
/// ② DpiChangeContext::set: echo bypass 防止信号
/// ③ guarded_set_window_pos: suggested_rect の位置のみ（SWP_NOSIZE 維持）
#[inline]
pub(super) fn WM_DPICHANGED(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    let new_dpi = crate::ecs::window::DPI::from_WM_DPICHANGED(wparam, lparam);

    // lparam から suggested_rect を取得
    let suggested_rect_ptr = lparam.0 as *const RECT;
    let suggested_rect = if !suggested_rect_ptr.is_null() {
        unsafe { *suggested_rect_ptr }
    } else {
        RECT::default()
    };

    debug!(
        hwnd = ?hwnd,
        dpi_x = new_dpi.dpi_x,
        dpi_y = new_dpi.dpi_y,
        scale_x = format_args!("{:.2}", new_dpi.scale_x()),
        scale_y = format_args!("{:.2}", new_dpi.scale_y()),
        suggested_left = suggested_rect.left,
        suggested_top = suggested_rect.top,
        suggested_right = suggested_rect.right,
        suggested_bottom = suggested_rect.bottom,
        "WM_DPICHANGED"
    );

    // ① World borrow: DPI コンポーネントを直接更新（Changed<DPI> 発火）
    // DPI コンポーネントが WM_WINDOWPOSCHANGED の tick 前に更新されている必要がある
    // （Changed<DPI> を update_arrangements_system が検知するため）
    {
        {
            if let Ok(mut world_borrow) = world.try_borrow_mut() {
                if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(entity) {
                    if let Some(mut dpi_comp) = entity_ref.get_mut::<crate::ecs::window::DPI>() {
                        let old_dpi = *dpi_comp;
                        *dpi_comp = new_dpi;
                        debug!(
                            entity = ?entity,
                            old_dpi_x = old_dpi.dpi_x,
                            old_dpi_y = old_dpi.dpi_y,
                            new_dpi_x = new_dpi.dpi_x,
                            new_dpi_y = new_dpi.dpi_y,
                            "[WM_DPICHANGED] DPI component directly updated (Changed<DPI>)"
                        );
                    }
                }
            }
            // world_borrow スコープ終了: 借用解放
        }
    }

    // ② DpiChangeContextをスレッドローカルに保存（echo bypass 防止 + BoxStyle skip 信号）
    // SetWindowPos → WM_WINDOWPOSCHANGED の流れで
    // WM_WINDOWPOSCHANGEDがこのコンテキストを消費する
    crate::ecs::window::DpiChangeContext::set(crate::ecs::window::DpiChangeContext::new(
        new_dpi,
        suggested_rect,
    ));

    // ③ 位置のみ SetWindowPos（SWP_NOSIZE 維持）
    // サイズは ECS レイアウトパイプライン（Changed<DPI> → update_arrangements_system
    // → propagate_global_arrangements → window_pos_sync_system → apply_window_pos_changes）
    // が算出するため、suggested_rect のサイズは使わない。
    // Req 1.3: 「OS 提案位置に基づく位置変更を実際に行ったか否か」は診断手順書が有効化する
    // 水準（`wintf::ecs::window_proc=debug`）で必ず出す。旧 `trace!` は当該手順で点灯せず、
    // 2026-07-18 の実機診断で「発生 0 回」という偽陰性を生んだ直接原因である。
    //
    // 本フェーズ（Phase A・観測増設）では分岐を導入しないため `applied` は恒真。
    // 提案位置の採否分岐（`DpiSuggestedRectPolicy` / `dpi_suggested_position_decision`）は
    // Phase C で配線され、そのとき本行が「書かなかった」も報告するようになる。
    let applied = true;
    debug!(
        entity = ?entity,
        hwnd = ?hwnd,
        applied = applied,
        suggested_left = suggested_rect.left,
        suggested_top = suggested_rect.top,
        "[WM_DPICHANGED] suggested position write decision"
    );

    let result = unsafe {
        crate::ecs::window::guarded_set_window_pos(
            hwnd,
            None,
            suggested_rect.left,
            suggested_rect.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
    if let Err(e) = result {
        warn!(hwnd = ?hwnd, error = ?e, "SetWindowPos failed in WM_DPICHANGED");
    }

    Some(LRESULT(0))
}

#[cfg(test)]
mod tests {
    use crate::ecs::test_support::capture_under_filter;
    use crate::ecs::window::DPI;
    use crate::ecs::world::EcsWorld;
    use crate::executor::util::WindowMessage;
    use bevy_ecs::prelude::Entity;
    use std::cell::RefCell;
    use std::rc::Rc;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::WM_DPICHANGED;

    /// 診断手順書が指定する `RUST_LOG`（design.md「診断手順書」・要件 1.4/1.5）。
    /// 観測点がこの directive で点灯することを機械的に固定するため、リテラルを共有する。
    const PROCEDURE_DIRECTIVES: &str =
        "info,wintf::ecs::window_proc=debug,wintf::ecs::drag=debug,areka::placement::diag=debug";

    /// 位置書込の共通経路（`guarded_set_window_pos`）まで開ける directive。
    /// `wintf::ecs::window` は前方一致ゆえ `wintf::ecs::window_proc` も併せて開く。
    const WRITE_PATH_DIRECTIVES: &str = "info,wintf::ecs::window=debug";

    /// 既定水準（`RUST_LOG` 未設定時のフォールバック＝`main.rs:262`）。
    const DEFAULT_DIRECTIVES: &str = "info";

    /// ヘッドレスに `WM_DPICHANGED` を 1 回配送する（実 HWND・メッセージループ不要）。
    ///
    /// `hwnd` は null ゆえ `SetWindowPos` は失敗するが、観測点はいずれも呼び出し前後に
    /// 置かれており本檻の対象（水準とフィールド）には影響しない。
    fn dispatch_dpichanged(new_dpi: u16, suggested: RECT) -> Entity {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = world
            .borrow_mut()
            .world_mut()
            .spawn(DPI::from_dpi(96, 96))
            .id();

        let m = WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: WM_DPICHANGED,
            wparam: WPARAM(((new_dpi as usize) << 16) | new_dpi as usize),
            lparam: LPARAM(&suggested as *const RECT as isize),
        };
        let _ = crate::ecs::dispatch_window_message(&world, entity, &m);

        // TLS に残る DpiChangeContext を回収し、同スレッドの後続テストへ漏らさない。
        let _ = crate::ecs::window::DpiChangeContext::take();
        entity
    }

    fn suggested_rect() -> RECT {
        RECT {
            left: 3210,
            top: 140,
            right: 3810,
            bottom: 620,
        }
    }

    /// 要件 1.3: 「提案位置に基づく位置変更を実際に行ったか否か」は、診断手順書の
    /// directive で**必ず**点灯する水準に置かれている（旧 `trace!` は点灯せず、
    /// 2026-07-18 の偽陰性＝「発生 0 回」の誤結論を生んだ）。
    #[test]
    fn suggested_position_decision_is_visible_under_procedure_directive() {
        let out = capture_under_filter(PROCEDURE_DIRECTIVES, || {
            dispatch_dpichanged(192, suggested_rect());
        });

        assert!(
            out.contains("[WM_DPICHANGED] suggested position write decision"),
            "提案位置の実施可否が診断手順の水準で観測できない（要件 1.3/1.5）: {out}"
        );
    }

    /// 要件 1.3: 実施可否の行は「書いたか否か」と提案 left/top・entity を伴う。
    #[test]
    fn suggested_position_decision_carries_applied_flag_and_suggested_origin() {
        let out = capture_under_filter(PROCEDURE_DIRECTIVES, || {
            dispatch_dpichanged(192, suggested_rect());
        });

        let line = out
            .lines()
            .find(|l| l.contains("[WM_DPICHANGED] suggested position write decision"))
            .unwrap_or_else(|| panic!("実施可否行が無い: {out}"));

        assert!(line.contains("applied="), "実施可否フィールドが無い: {line}");
        assert!(
            line.contains("suggested_left=3210") && line.contains("suggested_top=140"),
            "提案 left/top が復元できない: {line}"
        );
        assert!(
            line.contains("entity="),
            "表示基盤ログと areka 側レコードの結合キー `entity` が無い: {line}"
        );
    }

    /// 要件 1.3: 新旧 DPI も同じ directive で点灯する（受理回数の 2 段 grep 計数の前提）。
    #[test]
    fn dpi_acceptance_line_reports_old_and_new_dpi_under_procedure_directive() {
        let out = capture_under_filter(PROCEDURE_DIRECTIVES, || {
            dispatch_dpichanged(192, suggested_rect());
        });

        let line = out
            .lines()
            .find(|l| l.contains("[WM_DPICHANGED] DPI component directly updated"))
            .unwrap_or_else(|| panic!("DPI 受理行が無い: {out}"));

        assert!(
            line.contains("old_dpi_x=96") && line.contains("new_dpi_x=192"),
            "新旧 DPI が同一行から復元できない（方向の機械判定が不能）: {line}"
        );
    }

    /// 要件 1.5: 既定水準（`info`）では実施可否の行は出ない。
    /// ＝観測点が「手順で有効化される水準」に置かれていることの対偶側の固定。
    #[test]
    fn suggested_position_decision_is_silent_under_default_info_filter() {
        let out = capture_under_filter(DEFAULT_DIRECTIVES, || {
            dispatch_dpichanged(192, suggested_rect());
        });

        assert!(
            !out.contains("[WM_DPICHANGED] suggested position write decision"),
            "既定 `info` で診断専用の実施可否行が漏れている: {out}"
        );
    }

    /// 要件 1.3: 実際の窓位置書込を行う共通経路（`guarded_set_window_pos`）の実施ログも
    /// 診断手順が有効化できる水準にある（旧 `trace!` からの是正）。
    #[test]
    fn window_pos_write_path_is_visible_at_debug() {
        let out = capture_under_filter(WRITE_PATH_DIRECTIVES, || {
            dispatch_dpichanged(192, suggested_rect());
        });

        assert!(
            out.contains("[guarded_set_window_pos] Calling SetWindowPos"),
            "窓位置書込の共通経路が debug 水準で観測できない（要件 1.3）: {out}"
        );
    }

    /// 要件 1.5: 書込経路の実施ログも既定 `info` では出ない（診断専用のまま）。
    #[test]
    fn window_pos_write_path_is_silent_under_default_info_filter() {
        let out = capture_under_filter(DEFAULT_DIRECTIVES, || {
            dispatch_dpichanged(192, suggested_rect());
        });

        assert!(
            !out.contains("[guarded_set_window_pos] Calling SetWindowPos"),
            "既定 `info` で書込経路ログが漏れている: {out}"
        );
    }
}

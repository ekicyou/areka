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

    /// `WM_DPICHANGED` メッセージを組む（LOWORD=X DPI / HIWORD=Y DPI・LPARAM=提案矩形）。
    ///
    /// `suggested` は呼出側が生存させる（LPARAM は生ポインタ）。
    fn dpichanged_message(new_dpi: u16, suggested: &RECT) -> WindowMessage {
        WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: WM_DPICHANGED,
            wparam: WPARAM(((new_dpi as usize) << 16) | new_dpi as usize),
            lparam: LPARAM(suggested as *const RECT as isize),
        }
    }

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

        let m = dpichanged_message(new_dpi, &suggested);
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

    // ================================================================
    // S1 の赤証跡＝表示基盤ディスパッチ檻（タスク 4.3・Req 5.4／4.3／4.2）
    //
    // design.md「Testing Strategy > Integration Tests 5」が S1 の赤→緑の
    // **正証跡**と定める檻。是正前の欠陥は wndproc の無条件書込＝実配線に
    // 在るため、`dpi_helpers.rs` の純関数檻（分岐網羅の補助）では
    // 「是正前のコードに対して失敗する」の証明力が足りない。
    //
    // ## 何を主張しているか（是正後の契約＝D3）
    // - `DpiSuggestedRectPolicy::ExternalAuthority` を宣言した窓:
    //   `DPI` component は更新される／`DpiChangeContext` は **確立されない**／
    //   位置書込も **起きない**（＝最終位置は現接地点のまま残る）
    // - 宣言の無い窓（既定 `ApplyPosition` 相当）: 従来どおり確立され書かれる
    //   （非ゴースト窓の後方互換・design.md「Compatibility」）
    //
    // ## 是正未投入のツリーでは赤である（それが本タスクの成果物）
    // `WM_DPICHANGED`（本ファイル `:285`）は `DpiChangeContext::set`（`:343-346`）も
    // `guarded_set_window_pos`（`:369-379`）も **窓ごとの分岐なしに** 実行し、
    // `let applied = true;`（`:359`）はその事実の表示である。判断関数
    // `dpi_suggested_position_decision`（`window_proc/dpi_helpers.rs:32`）は
    // 本番から呼ばれていない（診断レポート §1.1）。配線はタスク 5.1 の所有であり、
    // **本タスクでは投入しない**（4.5 の実機採取が是正前ビルドを要求するため）。
    //
    // ## 赤の檻を常時赤のまま置かない工夫（`#[ignore]` ゲート）
    // 常時失敗するテストは以後の全タスクの検証を潰し `cargo test` を門として
    // 無価値にする。よって赤側 4 件は `#[ignore]` で通常実行から外し、
    // 再現コマンドを明記する（`areka-emo-atlas/src/emo2_golden.rs:228` の先例）:
    //
    // ```text
    // cargo test -p wintf -- --ignored s1_red_
    // ```
    //
    // **タスク 5.1 は是正配線と同時に本ブロックの `#[ignore]` を 4 件とも外し、
    // 常時走る回帰檻へ昇格させること**（Req 5.1 の常時テスト化）。
    //
    // ## dpi 水準の非対称（Req 5.1／5.4 の「96 が欠陥を隠す」）
    // 96 では OS 提案原点が現位置と一致するため、提案位置を書いても書かなくても
    // 最終位置が変わらず **政策分岐が観測できない＝赤にならない**。120／192 では
    // 提案原点が現位置から離れるため、無条件書込が接地点を破壊して赤になる。
    // 水準ごとに独立した檻へ分けてあるのは、この非対称が 1 回の実行出力から
    // そのまま読み取れるようにするためである。
    // ================================================================

    use crate::ecs::window::DpiSuggestedRectPolicy;

    /// 「areka が直前に確定した接地点」の代役となる現位置（物理 px）。
    ///
    /// 具体値そのものに意味は無い——判定は下の `suggested_rect_for` が組む
    /// **DPI 水準に対する比**と「現位置が保存されるか」の不変条件で表現する（Req 5.6）。
    const CURRENT_ORIGIN: (i32, i32) = (1200, 400);

    /// 提案矩形の寸（`SWP_NOSIZE` ゆえ判断には使われない・原点だけが効く）。
    const SUGGESTED_EXTENT: (i32, i32) = (400, 300);

    /// モニタ跨ぎ相当の OS 提案矩形を **比** で組む（絶対 px の直書きを避ける・Req 5.6）。
    ///
    /// `dpi=96` では比が 1.0 ゆえ提案原点＝現位置になる（＝欠陥が隠れる水準）。
    fn suggested_rect_for(dpi: u16, current: (i32, i32)) -> RECT {
        let ratio = dpi as f32 / 96.0;
        let left = (current.0 as f32 * ratio).round() as i32;
        let top = (current.1 as f32 * ratio).round() as i32;
        RECT {
            left,
            top,
            right: left + SUGGESTED_EXTENT.0,
            bottom: top + SUGGESTED_EXTENT.1,
        }
    }

    /// ディスパッチの外から観測できる 3 事実。
    #[derive(Debug)]
    struct DpiChangedOutcome {
        /// dispatch 後の `DPI` component の X（是正の有無によらず更新されるべき値）。
        dpi_x_after: u16,
        /// `DpiChangeContext` が確立されたか（＝提案位置の書込コンテキスト）。
        context_established: bool,
        /// 実窓へ書かれた原点（`guarded_set_window_pos` の実施ログから復元）。
        /// `None` = 書込が 1 度も起きていない。
        written_origin: Option<(i32, i32)>,
    }

    impl DpiChangedOutcome {
        /// 「書かなければ現位置が最終位置として残る」を畳んだ最終位置。
        fn final_position(&self, current: (i32, i32)) -> (i32, i32) {
            self.written_origin.unwrap_or(current)
        }
    }

    /// `guarded_set_window_pos` の実施ログ 1 行から書込原点を復元する。
    ///
    /// トークン境界でアンカーする——`x=` は `cx=` の接尾辞であり、部分一致では
    /// 取り違える（申し送り「`w=-` が `w=-12` の接頭辞」と同型の罠）。
    fn parse_write_origin(out: &str) -> Option<(i32, i32)> {
        let line = out
            .lines()
            .find(|l| l.contains("[guarded_set_window_pos] Calling SetWindowPos"))?;
        let field = |name: &str| -> Option<i32> {
            line.split_whitespace()
                .find_map(|tok| tok.strip_prefix(name))
                .and_then(|v| v.parse::<i32>().ok())
        };
        Some((field("x=")?, field("y=")?))
    }

    /// 政策 component の有無を変えて `WM_DPICHANGED` を 1 回配送し、外形を観測する。
    fn dispatch_dpichanged_observed(
        new_dpi: u16,
        suggested: RECT,
        policy: Option<DpiSuggestedRectPolicy>,
    ) -> DpiChangedOutcome {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = {
            let mut w = world.borrow_mut();
            let mut e = w.world_mut().spawn(DPI::from_dpi(96, 96));
            if let Some(p) = policy {
                e.insert(p);
            }
            e.id()
        };

        let m = dpichanged_message(new_dpi, &suggested);
        let out = capture_under_filter(WRITE_PATH_DIRECTIVES, || {
            let _ = crate::ecs::dispatch_window_message(&world, entity, &m);
        });

        // TLS に残る `DpiChangeContext` を回収する。回収は観測そのものであり、
        // 同時に同スレッドの後続テストへの漏洩も防ぐ。
        let context_established = crate::ecs::window::DpiChangeContext::take().is_some();

        let dpi_x_after = {
            let mut w = world.borrow_mut();
            w.world_mut()
                .get::<DPI>(entity)
                .copied()
                .expect("DPI component は spawn 時に付与済み")
                .dpi_x
        };

        DpiChangedOutcome {
            dpi_x_after,
            context_established,
            written_origin: parse_write_origin(&out),
        }
    }

    /// 水準を引数に取る S1 赤檻の本体。
    ///
    /// 主張は「`ExternalAuthority` 窓の最終位置＝現接地点」（Req 4.3: OS 推奨位置を
    /// 最終位置としてそのまま残さない／Req 4.2: 最終位置が接地点規約に従う）。
    fn assert_external_authority_preserves_anchor_at(dpi: u16) {
        let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
        let outcome = dispatch_dpichanged_observed(
            dpi,
            suggested,
            Some(DpiSuggestedRectPolicy::ExternalAuthority),
        );

        // S1 の是正は DPI 受理を止めるものではない（止めたら寸の再導出が死ぬ）。
        assert_eq!(
            outcome.dpi_x_after, dpi,
            "dpi={dpi}: DPI component が更新されていない: {outcome:?}"
        );

        // 探針の自己検査: 96 は提案＝現位置（分岐が観測できない水準）、
        // 96 以外は提案が現位置から離れている（＝不動点でない・記憶〈2.2 の教訓〉）。
        if dpi == 96 {
            assert_eq!(
                (suggested.left, suggested.top),
                CURRENT_ORIGIN,
                "dpi=96 では提案原点が現位置と一致する前提（96 が欠陥を隠す性質）"
            );
        } else {
            assert_ne!(
                suggested.left, CURRENT_ORIGIN.0,
                "dpi={dpi} では提案 X が現位置から動いている前提"
            );
        }

        assert_eq!(
            outcome.final_position(CURRENT_ORIGIN),
            CURRENT_ORIGIN,
            "dpi={dpi}: ExternalAuthority 窓の最終位置は現接地点のままであるべき\
             （OS 提案位置が無条件に採用されている＝S1・Req 4.3/4.2）: {outcome:?}"
        );
    }

    /// dpi=96: 提案原点＝現位置ゆえ、是正の有無にかかわらず**通過する**。
    /// この緑が「96 の自己整合が欠陥を隠す」ことの実行証跡である（Req 5.1／5.4）。
    #[test]
    #[ignore = "S1 赤証跡（是正未投入では失敗する・タスク 4.3）。再現: cargo test -p wintf -- --ignored s1_red_"]
    fn s1_red_external_authority_preserves_anchor_at_dpi96() {
        assert_external_authority_preserves_anchor_at(96);
    }

    /// dpi=120: 提案原点が現位置から離れる → 是正未投入では**失敗する**。
    #[test]
    #[ignore = "S1 赤証跡（是正未投入では失敗する・タスク 4.3）。再現: cargo test -p wintf -- --ignored s1_red_"]
    fn s1_red_external_authority_preserves_anchor_at_dpi120() {
        assert_external_authority_preserves_anchor_at(120);
    }

    /// dpi=192: 同上（変位がさらに大きい）。
    #[test]
    #[ignore = "S1 赤証跡（是正未投入では失敗する・タスク 4.3）。再現: cargo test -p wintf -- --ignored s1_red_"]
    fn s1_red_external_authority_preserves_anchor_at_dpi192() {
        assert_external_authority_preserves_anchor_at(192);
    }

    /// D3 の構造そのもの: `ExternalAuthority` 窓では
    /// **書込コンテキストが確立されず**、**位置書込も起きない**。
    ///
    /// 上の 3 件が「最終位置」という外形で主張するのに対し、こちらは
    /// 是正が置かれるべき 2 箇所（`DpiChangeContext::set` と
    /// `guarded_set_window_pos`）を名指しで固定する。是正未投入では両方とも
    /// 無条件に走るため 120／192 で失敗する。
    #[test]
    #[ignore = "S1 赤証跡（是正未投入では失敗する・タスク 4.3）。再現: cargo test -p wintf -- --ignored s1_red_"]
    fn s1_red_external_authority_establishes_no_write_context() {
        for dpi in [120_u16, 192] {
            let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
            let outcome = dispatch_dpichanged_observed(
                dpi,
                suggested,
                Some(DpiSuggestedRectPolicy::ExternalAuthority),
            );

            assert_eq!(
                outcome.dpi_x_after, dpi,
                "dpi={dpi}: DPI component が更新されていない: {outcome:?}"
            );
            assert!(
                !outcome.context_established,
                "dpi={dpi}: ExternalAuthority 窓で DpiChangeContext が確立されている\
                 （残置コンテキストが後続 WM_WINDOWPOSCHANGED を DPI echo と誤認させる・D3）: {outcome:?}"
            );
            assert!(
                outcome.written_origin.is_none(),
                "dpi={dpi}: ExternalAuthority 窓へ OS 提案位置が書き込まれている（S1）: {outcome:?}"
            );
        }
    }

    /// 非退行（**是正の前後いずれでも緑**・ゲート外で常時走る）:
    /// 政策未宣言／明示 `ApplyPosition` の窓は従来どおり提案原点を書き、
    /// `DpiChangeContext` も確立する。
    ///
    /// これが崩れると examples・将来の通常窓が Per-Monitor v2 の標準応答を失う
    /// （design.md「Compatibility」）。
    #[test]
    fn s1_control_default_policy_windows_apply_suggested_origin() {
        for dpi in [96_u16, 120, 192] {
            for policy in [None, Some(DpiSuggestedRectPolicy::ApplyPosition)] {
                let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
                let outcome = dispatch_dpichanged_observed(dpi, suggested, policy);

                assert_eq!(
                    outcome.dpi_x_after, dpi,
                    "dpi={dpi} policy={policy:?}: DPI component が更新されていない: {outcome:?}"
                );
                assert!(
                    outcome.context_established,
                    "dpi={dpi} policy={policy:?}: 既定窓で DpiChangeContext が確立されない: {outcome:?}"
                );
                assert_eq!(
                    outcome.written_origin,
                    Some((suggested.left, suggested.top)),
                    "dpi={dpi} policy={policy:?}: 既定窓へ提案原点が書かれていない: {outcome:?}"
                );
            }
        }
    }

    /// D3 の帰結（2.1 → 5.1 申し送り）: `DpiChangeContext::set` と
    /// `guarded_set_window_pos` は **1 個の `if let Some((x, y))` で束ねて分岐**する。
    /// ゆえに「コンテキスト確立 ⇔ 位置書込」が常に成り立たねばならない。
    ///
    /// 是正未投入では両者とも恒真なので緑（＝本檻は赤証跡ではない）。5.1 が
    /// 片側だけを分岐させた場合に赤になる**設計上の分割禁止**の固定である。
    #[test]
    fn s1_write_context_and_position_write_are_branched_together() {
        for dpi in [96_u16, 120, 192] {
            for policy in [
                None,
                Some(DpiSuggestedRectPolicy::ApplyPosition),
                Some(DpiSuggestedRectPolicy::ExternalAuthority),
            ] {
                let suggested = suggested_rect_for(dpi, CURRENT_ORIGIN);
                let outcome = dispatch_dpichanged_observed(dpi, suggested, policy);

                assert_eq!(
                    outcome.context_established,
                    outcome.written_origin.is_some(),
                    "dpi={dpi} policy={policy:?}: コンテキスト確立と位置書込が別々に分岐している（D3 違反）: {outcome:?}"
                );
            }
        }
    }
}

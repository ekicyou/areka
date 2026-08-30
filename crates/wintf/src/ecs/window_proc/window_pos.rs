//! ウィンドウ位置・サイズ・DPI変更メッセージハンドラ
//!
//! WM_WINDOWPOSCHANGED, WM_DPICHANGED の処理を担当する。
//! DPI変更時のセンター座標補正ロジックもこのモジュールに含まれる。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::prelude::Entity;
use tracing::{debug, trace, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::ecs::window::transition_diag::{
    self, MSG_DPICHANGED, MSG_WINDOWPOSCHANGED, MsgRecord, ORIGIN_DPI_SUGGESTED, WriteRecord,
    WriteStage, WriteTag,
};
use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

// ============================================================================
// 外部由来の重なり変化について
// ============================================================================
//
// かつてここには「外部由来の位置変化を捉えてグループ維持系へ是正を促す」引き金が在った
// （`wants_group_follow`／`note_external_zorder_change`）。所有の鎖による維持
// （`window/zorder_chain.rs`）へ移った時点で、その引き金は**担う要件を 1 つも持たなく
// なった**——利用者の操作で窓が持ち上がっても、鎖は「所有される窓は所有者より手前」という
// OS の不変条件そのものが保つので、観測も是正も要らない（要件 14.2／14.3）。
// 引き金を残せば、要件 14.2 が退役させた「繰り返しの観測と是正」を別経路で復活させることに
// なるため、実装ごと撤去してある。本ハンドラは重なりについて何も立てない。

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
    hwnd: HWND,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    // echo 判定: TLS フラグを参照（ステップ①冒頭で1回のみ）
    let is_echo = crate::ecs::window::is_self_initiated();

    // 受理の記録（要件 2.1）。`in_swp` は上の echo 判定そのもの——「自アプリの
    // `SetWindowPos` の内側で同期送達されたか」であり、`since_flush_us` が非番兵なら
    // その `SetWindowPos` は一括 flush の区間内で撃たれたことになる。
    //
    // **前置ガードの内側だけ**で組む。本ハンドラはドラッグ中も含めて絶えず走るため
    // （要件 10.6 の追従比 1.000・定常の窓書込 0 を壊さない）、観測が無効な運転では
    // 行の組立も時刻の読み取りも一切行わない。
    if transition_diag::is_enabled() {
        transition_diag::emit_line(&transition_diag::msg_line(&MsgRecord {
            stamp: transition_diag::stamp(),
            msg: MSG_WINDOWPOSCHANGED,
            hwnd,
            in_swp: is_echo,
            since_flush_us: transition_diag::since_flush_us(),
        }));
    }

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
///    ＋ 当該窓の [`DpiSuggestedRectPolicy`] を読む（**同一借用内**）
/// ② DpiChangeContext::set: echo bypass 防止信号
/// ③ guarded_set_window_pos: suggested_rect の位置のみ（SWP_NOSIZE 維持）
///
/// ## 提案位置の採否（areka-P0-dpi-window-vanish S1 是正・D3・Req 4.3）
///
/// ②③は [`dpi_suggested_position_decision`] の戻り値で **1 個の `if let Some((x, y))`
/// にまとめて**分岐する。位置権威が外部（ECS 側の配置システム）にある窓
/// （[`DpiSuggestedRectPolicy::ExternalAuthority`]）では **OS 提案位置を書かず、
/// `DpiChangeContext` も立てない**——書込だけを止めて残置コンテキストを立てると、
/// 直後の `WM_WINDOWPOSCHANGED`（外部権威自身の `SetWindowPos` の echo）が
/// 「DPI 由来の外部変更」と誤認され、中心保持補正が誤適用される。**②③を別々に
/// 分岐させてはならない**（in-source 檻
/// `s1_write_context_and_position_write_are_branched_together` が固定）。
///
/// ①（DPI component の更新と `Changed<DPI>` の発火）は**無条件**である。DPI の受理を
/// 止めると寸の再導出パイプラインごと死ぬため、政策が断つのは位置の書き手だけである。
///
/// [`dpi_suggested_position_decision`]: super::dpi_helpers::dpi_suggested_position_decision
/// [`DpiSuggestedRectPolicy`]: crate::ecs::window::DpiSuggestedRectPolicy
#[inline]
pub(super) fn WM_DPICHANGED(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    // 受理の記録（要件 2.1）。再観測 §5 の 24/24——各窓の最初の `SetWindowPos` の内側で
    // 当該窓の `WM_DPICHANGED` が同期処理される——は `in_swp`／`since_flush_us` の
    // 組で読む。判定より前に置くのは「受理そのもの」の刻印だからである。
    if transition_diag::is_enabled() {
        transition_diag::emit_line(&transition_diag::msg_line(&MsgRecord {
            stamp: transition_diag::stamp(),
            msg: MSG_DPICHANGED,
            hwnd,
            in_swp: crate::ecs::window::is_self_initiated(),
            since_flush_us: transition_diag::since_flush_us(),
        }));
    }

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

    // ① World borrow: DPI コンポーネントを直接更新（Changed<DPI> 発火）＋ 政策の読み取り
    // DPI コンポーネントが WM_WINDOWPOSCHANGED の tick 前に更新されている必要がある
    // （Changed<DPI> を update_arrangements_system が検知するため）。
    //
    // 政策（`DpiSuggestedRectPolicy`）を**この借用の中で**読むのは、②③の分岐に必要な
    // 唯一の World 依存だからである。借用を 2 度取ると再入時に片方だけ失敗し得る。
    let mut policy: Option<crate::ecs::window::DpiSuggestedRectPolicy> = None;
    // entity へ到達できたか（到達不能時に「政策未付与」と区別できないと、ログが
    // 「宣言が無かった」のか「読めなかった」のかを黙って混ぜてしまう）。
    let mut entity_reached = false;
    {
        {
            if let Ok(mut world_borrow) = world.try_borrow_mut() {
                if let Ok(mut entity_ref) = world_borrow.world_mut().get_entity_mut(entity) {
                    entity_reached = true;
                    // 先に immutable で政策を写し取る（次行の get_mut と共存できないため）。
                    policy = entity_ref
                        .get::<crate::ecs::window::DpiSuggestedRectPolicy>()
                        .copied();
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

    // 提案位置の採否を純関数へ委ねる（D3・Req 4.3）。戻り値の `Option` が②③の
    // **共通の**分岐条件であり、「書かない」を番兵座標ではなく型で表す。
    let decision =
        super::dpi_helpers::dpi_suggested_position_decision(policy.as_ref(), &suggested_rect);
    let applied = decision.is_some();

    // Req 1.3: 「OS 提案位置に基づく位置変更を実際に行ったか否か」は診断手順書が有効化する
    // 水準（`wintf::ecs::window_proc=debug`）で必ず出す。旧 `trace!` は当該手順で点灯せず、
    // 2026-07-18 の実機診断で「発生 0 回」という偽陰性を生んだ直接原因である。
    //
    // `policy` は判断の根拠（design.md「dpi_suggested_position_decision > Validation」の
    // フィールド）。網羅 match ゆえ、政策に腕が増えたらコンパイラがここを指す。
    // 「到達不能」を専用語にしているのは Req 1.5 の趣旨——読めなかったことを
    // 「宣言が無かった」と同じ語で報告すると、事後の突合で偽の結論を作る。
    let policy_label = match (entity_reached, policy) {
        (false, _) => "unreachable",
        (true, None) => "unset",
        (true, Some(crate::ecs::window::DpiSuggestedRectPolicy::ApplyPosition)) => "ApplyPosition",
        (true, Some(crate::ecs::window::DpiSuggestedRectPolicy::ExternalAuthority)) => {
            "ExternalAuthority"
        }
    };
    debug!(
        entity = ?entity,
        hwnd = ?hwnd,
        // `format_args!` で包むのは引用符を付けないため（`policy = policy_label` だと
        // `policy="unset"` と出て、同行の他フィールドと grep の当たり方が変わる。
        // 書式のばらつきが突合を静かにゼロ件にする罠は本 spec で既知＝`hwnd` の前例）。
        policy = format_args!("{policy_label}"),
        applied = applied,
        suggested_left = suggested_rect.left,
        suggested_top = suggested_rect.top,
        "[WM_DPICHANGED] suggested position write decision"
    );

    // ②③をまとめて分岐する（D3）。片側だけ実行すると、書かないのにコンテキストだけ
    // 残る／書くのに echo 判定が効かない、のどちらかの競合を新設することになる。
    if let Some((x, y)) = decision {
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
        let flags = SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;

        // 観測（要件 2.1）。**書込 1 回を数える点はここ 1 箇所**であり、経路 A
        // （メッセージ受理時の同期書込）として一括 flush（`stage=flush`）と区別する。
        // 前置ガードが偽なら計時も読み戻しも行わない。
        let observe = transition_diag::is_enabled();
        let call_started = observe.then(Instant::now);

        let result =
            unsafe { crate::ecs::window::guarded_set_window_pos(hwnd, None, x, y, 0, 0, flags) };

        if observe {
            let call_us = call_started.map_or(0, transition_diag::elapsed_us);
            transition_diag::emit_line(&transition_diag::write_line(&WriteRecord {
                stamp: transition_diag::stamp(),
                stage: WriteStage::Sync,
                seq: 0,
                hwnd,
                // 経路語は wintf 自身が名乗る（`origin` は「どの経路が書込を要求したか」＝
                // 要件 2.1 のフィールドであり、実在する要求元を番兵で埋めると
                // 「タグの付け忘れ」と区別が付かなくなる）。キャラ番号と窓種別は areka の
                // 語彙であり表示基盤からは判らないので、そこは番兵のままにする。
                tag: WriteTag {
                    origin: ORIGIN_DPI_SUGGESTED,
                    ..WriteTag::UNTAGGED
                },
                x,
                y,
                cx: 0,
                cy: 0,
                flags: flags.0,
                after: crate::ecs::window::read_back_window_rect(hwnd),
                call_us,
                ok: result.is_ok(),
                // 経路 A はメッセージ受理時の同期書込であり、一括 flush のバッチを通らない。
                in_batch: false,
            }));
        }

        if let Err(e) = result {
            warn!(hwnd = ?hwnd, error = ?e, "SetWindowPos failed in WM_DPICHANGED");
        }
    }

    // 「書かなかった」場合も `Some(LRESULT(0))`＝処理済みを返す。`None` を返すと
    // `DefWindowProcW` が既定の提案矩形適用を行い、**その内部から `SetWindowPos` が
    // 同期的に呼ばれる**（`window/components.rs:29-31`）ため、源断ちが無意味になる。
    // ここが源断ちの最外殻であり、`guarded_set_window_pos` を飛ばすだけでは足りない
    // （檻＝`s1_external_authority_handles_the_message_instead_of_delegating_to_defwindowproc`）。
    // 提案矩形の不採用は Per-Monitor v2 の契約違反ではない（OS 提示は勧告である）。
    Some(LRESULT(0))
}

#[cfg(test)]
#[path = "window_pos_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "window_pos_transition_tests.rs"]
mod window_pos_transition_tests;

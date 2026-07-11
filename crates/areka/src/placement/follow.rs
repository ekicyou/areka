//! バルーン追従コンポーネントと窓移動の公開 API。
//!
//! - [`BalloonFollow`]: キャラ窓に付与する追従 Component（配置時確定の暫定 offset・
//!   物理 px・4.4。offset は `ScopePlacement.balloon_offset` の転写）
//! - [`on_char_drag`]: `OnDrag` ハンドラ（mock-shell donor `on_shell_drag` の一般化。
//!   マーカー全走査ではなく `BalloonFollow.balloon` の `WindowHandle` を直接引く）
//! - [`move_window_to`]: R7 公開 API（UI スレッド関数・物理 px スクリーン座標直渡し）
//! - [`MonitorSnapshot`]／[`work_area_for_window`]: bottom 吸着ドラッグ（4.7・DD15）の
//!   基盤——全モニタ work area 集合の Resource と窓中心→モニタ解決の純粋ヘルパ
//!   （消費側の Y 釘付けは task 8.2。snapshot は吸着の消費地たる本モジュールに置く）
//!
//! # 座標単位契約（design U1/U4）
//!
//! 本モジュールの座標はすべて**物理 px**。`WindowPos.position` は wndproc が
//! 実ウィンドウ位置から更新する物理 px であり、ここに DPI 再スケール
//! （`dpi/96` 乗除）を一切挟まない（2026-07-05 の二重スケール欠陥の檻）。
//!
//! # UI スレッド契約（7.1/7.2/7.3）
//!
//! 署名は `&mut World` のみで完結し channel／actor 型を持たない。`&mut World` は
//! wintf の UI スレッド tick 内でのみ到達可能なため、窓操作の UI スレッド専有
//! （7.2）を型で担保する。UI 配送ブリッジ（`spawn_ui`／`UiSender`）との結線は
//! 後続の領分（7.3）。

use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};
use wintf::ecs::drag::DragEvent;
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::{Point, SetWindowPosCommand, WindowHandle, WindowPos};

use super::resolver::{PointPx, RectPx};

/// キャラ窓に付与するバルーン追従 Component（4.2/4.4）。
///
/// `offset` は配置時に 1 回だけ確定する暫定 offset（物理 px・
/// `ScopePlacement.balloon_offset` の転写）。バルーン単独ドラッグでユーザーが
/// ずらしても、次のキャラ窓ドラッグで初期 offset に戻る（暫定規則の受容挙動。
/// 正式な配置規則は balloon 表示系の後続が所有する・4.4）。
#[allow(dead_code)] // 窓 entity への付与（spawn）は task 5.1
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalloonFollow {
    /// 追従して動かすバルーン窓 entity。
    pub balloon: Entity,
    /// キャラ窓左上からバルーン窓左上への相対 offset（物理 px・配置時確定）。
    pub offset: PointPx,
}

/// `OnDrag` ハンドラ: ドラッグ中のキャラ窓へバルーンを追従させる（4.2/4.3）。
///
/// キャラ窓自体は `DragConfig { move_window: true }` により wndproc レベルで
/// 移動済み。本ハンドラは wndproc が更新した `WindowPos.position`（物理 px）へ
/// `BalloonFollow.offset` を加算し、バルーン窓へ `SetWindowPosCommand` を
/// enqueue するだけ（再スケールなし・U4）。
///
/// イベントは消費しない（常に `false`＝伝播続行。donor on_shell_drag と同じ規約）。
#[allow(dead_code)] // OnDrag(on_char_drag) の結線（spawn）は task 5.1
pub(crate) fn on_char_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(_) => {
            // キャラ窓の現在位置（wndproc が実窓位置から更新済み・物理 px）
            let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
                return false;
            };
            let Some(follow) = world.get::<BalloonFollow>(entity).copied() else {
                return false;
            };

            // 不変条件: pos は仮想スクリーン座標範囲・offset は配置時確定の
            // 有限値のため、加算が i32 を溢れることはない（溢れは入力源の異常）。
            debug_assert!(
                pos.x.checked_add(follow.offset.x).is_some()
                    && pos.y.checked_add(follow.offset.y).is_some(),
                "char window position out of virtual-screen range: {pos:?} + {:?}",
                follow.offset
            );
            enqueue_window_move(
                world,
                follow.balloon,
                pos.x + follow.offset.x,
                pos.y + follow.offset.y,
            );
            false
        }
    }
}

/// R7 公開 API: UI スレッド上で呼ばれる窓移動関数（物理 px・スクリーン座標直渡し・7.1）。
///
/// - 移動は `SetWindowPosCommand`（`SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE`）経由。
///   座標は物理 px 素通し（U4・再スケールなし）
/// - 対象が [`BalloonFollow`] を持つ場合はバルーン窓も offset 維持で随伴移動する
/// - 対象不在／`WindowHandle` 未付与（窓生成前）は `warn!` して `false` を返す
///   （silent no-op にしない）。このとき随伴バルーンも動かさない
/// - 随伴バルーン側の `WindowHandle` 未付与は `warn!` のみ（対象自身の移動は成立
///   しているため戻り値は `true`）
#[allow(dead_code)] // 呼び出し側（UI 配送ブリッジ結線）は後続 spec の領分（7.3）
pub fn move_window_to(world: &mut World, window: Entity, x: i32, y: i32) -> bool {
    let follow = world.get::<BalloonFollow>(window).copied();

    if !enqueue_window_move(world, window, x, y) {
        return false;
    }

    if let Some(follow) = follow {
        debug_assert!(
            x.checked_add(follow.offset.x).is_some() && y.checked_add(follow.offset.y).is_some(),
            "move target out of virtual-screen range: ({x},{y}) + {:?}",
            follow.offset
        );
        // バルーン側の失敗（WindowHandle 未付与等）は enqueue_window_move が
        // warn! 済み。対象自身の移動は成立しているため true のまま返す。
        enqueue_window_move(world, follow.balloon, x + follow.offset.x, y + follow.offset.y);
    }

    true
}

/// 1 窓ぶんの移動を enqueue する共通経路（物理 px 素通し）。
///
/// `WindowHandle` を直接引いて `SetWindowPosCommand` を enqueue し、ECS 側の
/// `WindowPos.position` を `bypass_change_detection()` で先行反映する。
///
/// bypass の理由: 実アプリでは flush 後の `SetWindowPos` が同期発火させる
/// `WM_WINDOWPOSCHANGED` echo が同値を（同じく bypass で）再書込するため、
/// ここで `Changed<WindowPos>` を発火させると `apply_window_pos_changes` が
/// 別フラグの `SetWindowPos` を二重発行してしまう。bypass なら発行は本関数の
/// 1 コマンドに閉じ、headless World（echo が来ない）でも `WindowPos` が
/// 期待座標を示す決定論シームになる。
fn enqueue_window_move(world: &mut World, window: Entity, x: i32, y: i32) -> bool {
    let Some(handle) = world.get::<WindowHandle>(window).copied() else {
        warn!(
            entity = ?window,
            x, y,
            "移動対象窓が不在か WindowHandle 未付与（生成前）のため移動しない"
        );
        return false;
    };

    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        handle.hwnd,
        x,
        y,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    ));

    match world.get_mut::<WindowPos>(window) {
        Some(mut wp) => {
            wp.bypass_change_detection().position = Some(Point { x, y });
        }
        None => {
            debug!(
                entity = ?window,
                "WindowPos 未付与のため ECS 側ミラー更新はスキップ（コマンドは enqueue 済み）"
            );
        }
    }

    true
}

// =============================================================================
// MonitorSnapshot（task 8.1・DD15 基盤・4.7）
// =============================================================================

/// 全モニタの work area 集合（物理 px・起動時取得のセッション内固定 snapshot・DD15）。
///
/// 起動時に seam（main.rs）／example が [`MonitorSnapshot::from_monitors`] で実モニタ
/// から忠実転写して Resource 挿入し、bottom 吸着ドラッグ（task 8.2）が
/// [`work_area_for_window`] で「窓が現在属するモニタの work area」を引くのに使う。
/// snapshot はセッション内固定＝M1 受容（`WM_DISPLAYCHANGE` 追随は後続・DD15）。
/// 中身は `RectPx` のみの純粋データで、headless テストは合成値を直接構築して
/// 注入する（偽装境界・wintf に触れるのは挿入サイトだけ）。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    /// モニタ列挙順の work area（物理 px）。
    pub work_areas: Vec<RectPx>,
}

impl MonitorSnapshot {
    /// 実モニタ列挙結果から全 work area（`RECT`・物理 px）を列挙順のまま
    /// **単位変換なしで忠実転写**する（mod.rs `primary_work_area` と同じ U 契約:
    /// どちらも物理 px 通貨）。0 台は空 snapshot（panic しない・消費側が
    /// [`work_area_for_window`] の `None` で防御）。
    pub fn from_monitors(monitors: &[Monitor]) -> Self {
        Self {
            work_areas: monitors
                .iter()
                .map(|m| RectPx {
                    left: m.work_area.left,
                    top: m.work_area.top,
                    right: m.work_area.right,
                    bottom: m.work_area.bottom,
                })
                .collect(),
        }
    }
}

/// 窓矩形の中心が属するモニタの work area を引く純粋ヘルパ（4.7・DD15）。
///
/// 決定論規則（テストで固定）:
/// - 中心は `((left+right)/2, (top+bottom)/2)`（i64 演算・ゼロ方向切り捨て）
/// - 帰属判定は half-open（`left ≤ cx < right`・`top ≤ cy < bottom`）＝共有辺上の
///   中心は右／下側のモニタへ属する。複数矩形が含む（重複）場合は昇順 index 先勝ち
/// - どのモニタにも属さない場合は最近傍（中心→矩形 clamp 点の自乗距離最小・
///   等距離は昇順 index 先勝ち＝`min_by_key` の先頭優先）
/// - 空 snapshot は `None`（架空の既定矩形を発明しない・resolver と同方針）
///
/// 距離は i128 で自乗和を取り、極端な仮想スクリーン座標でも溢れない
/// （panic しない契約・resolver の saturating 演算と同じ防波堤）。
#[allow(dead_code)] // 消費側（bottom 吸着ドラッグの Y 釘付け）は task 8.2
pub fn work_area_for_window(snapshot: &MonitorSnapshot, window: RectPx) -> Option<RectPx> {
    let cx = (window.left as i64 + window.right as i64) / 2;
    let cy = (window.top as i64 + window.bottom as i64) / 2;

    // 帰属（half-open）・昇順 index 先勝ち
    if let Some(wa) = snapshot.work_areas.iter().find(|wa| {
        (wa.left as i64) <= cx
            && cx < (wa.right as i64)
            && (wa.top as i64) <= cy
            && cy < (wa.bottom as i64)
    }) {
        return Some(*wa);
    }

    // どこにも属さない → 最近傍（clamp 点との自乗距離最小・等距離は先勝ち）
    snapshot
        .work_areas
        .iter()
        .min_by_key(|wa| {
            // `i64::clamp` は逆転区間（万一の退化矩形）で panic するため min/max で書く
            // （resolver `clamp_axis` と同じ非 panic 流儀）
            let px = cx.min(wa.right as i64).max(wa.left as i64);
            let py = cy.min(wa.bottom as i64).max(wa.top as i64);
            let dx = (cx - px) as i128;
            let dy = (cy - py) as i128;
            dx * dx + dy * dy
        })
        .copied()
}

// =============================================================================
// Tests（TDD RED: 実装前に振る舞いを固定する）
// =============================================================================

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use bevy_ecs::prelude::*;
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use wintf::ecs::drag::DragEvent;
    use wintf::ecs::pointer::Phase;
    use wintf::ecs::{Point, WindowHandle, WindowPos};

    use super::{BalloonFollow, move_window_to, on_char_drag};
    use crate::placement::resolver::PointPx;

    // -------------------------------------------------------------------------
    // テストヘルパ（偽装境界: 実 HWND なしの headless World で決定論検証する。
    // SetWindowPosCommand は TLS キューへの enqueue のみで flush しないため、
    // 偽 HWND に対する実 SetWindowPos は一切呼ばれない——wintf 自身の
    // window_pos_systems_test と同じ流儀）
    // -------------------------------------------------------------------------

    /// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム）。
    fn fake_handle(raw: usize) -> WindowHandle {
        WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        }
    }

    /// position 初期値付きの WindowPos。
    fn window_pos_at(x: i32, y: i32) -> WindowPos {
        WindowPos {
            position: Some(Point { x, y }),
            ..Default::default()
        }
    }

    /// entity の WindowPos.position を読む（未設定は panic で検出）。
    fn position_of(world: &World, entity: Entity) -> Point {
        world
            .get::<WindowPos>(entity)
            .expect("WindowPos があるはず")
            .position
            .expect("position があるはず")
    }

    fn drag_event(target: Entity) -> DragEvent {
        DragEvent {
            target,
            start_position: Point::new(0, 0),
            position: Point::new(10, 10),
            is_primary: true,
            timestamp: Instant::now(),
        }
    }

    // -------------------------------------------------------------------------
    // move_window_to（R7 公開 API・7.1/7.2/7.3・U4）
    // -------------------------------------------------------------------------

    /// 観測可能な完了状態: headless World 上で move_window_to を呼ぶと
    /// 対象窓の WindowPos が期待座標へ更新される（物理 px 素通し・U4）。
    /// 座標は 96 の倍数を避けた値を使い、隠れた dpi/96 再スケールがあれば
    /// 完全一致が崩れる檻とする（07-05 欠陥の再発防止・3.2/3.3）。
    #[test]
    fn move_window_to_updates_window_pos_physical_px() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1234), window_pos_at(10, 20)))
            .id();

        assert!(move_window_to(&mut world, window, 1531, 883));
        assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });
    }

    /// WindowHandle 未付与（窓生成前）は false を返し、位置も変更しない。
    #[test]
    fn move_window_to_without_handle_returns_false() {
        let mut world = World::new();
        let window = world.spawn(window_pos_at(10, 20)).id();

        assert!(!move_window_to(&mut world, window, 500, 600));
        assert_eq!(position_of(&world, window), Point { x: 10, y: 20 });
    }

    /// despawn 済み（対象不在）の entity も false（silent no-op にしない・panic しない）。
    #[test]
    fn move_window_to_on_despawned_entity_returns_false() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1234), window_pos_at(0, 0)))
            .id();
        world.despawn(window);

        assert!(!move_window_to(&mut world, window, 100, 200));
    }

    /// BalloonFollow を持つ対象の移動はバルーンも offset 維持で随伴移動する
    /// （T-I4: 移動後も balloon_pos − char_pos ≡ offset が保存される）。
    #[test]
    fn move_window_to_moves_balloon_with_offset_preserved() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: -412, y: -25 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow { balloon, offset },
            ))
            .id();

        assert!(move_window_to(&mut world, window, 907, 1201));

        let char_pos = position_of(&world, window);
        let balloon_pos = position_of(&world, balloon);
        assert_eq!(char_pos, Point { x: 907, y: 1201 });
        assert_eq!(
            balloon_pos,
            Point {
                x: 907 + offset.x,
                y: 1201 + offset.y
            }
        );
        // offset 保存則（balloon_pos − char_pos ≡ offset）
        assert_eq!(balloon_pos.x - char_pos.x, offset.x);
        assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    }

    /// 対象自身に WindowHandle が無ければ false で、バルーンも動かさない。
    #[test]
    fn move_window_to_target_without_handle_does_not_move_balloon() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
            .id();
        let window = world
            .spawn((
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        assert!(!move_window_to(&mut world, window, 907, 1201));
        assert_eq!(position_of(&world, window), Point { x: 50, y: 60 });
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    /// バルーン側に WindowHandle が無い場合: 対象の移動自体は成功（true）し、
    /// バルーンは動かない（warn ログ・silent failure ではない）。
    #[test]
    fn move_window_to_balloon_without_handle_still_moves_target() {
        let mut world = World::new();
        let balloon = world.spawn(window_pos_at(70, 80)).id();
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        assert!(move_window_to(&mut world, window, 907, 1201));
        assert_eq!(position_of(&world, window), Point { x: 907, y: 1201 });
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    // -------------------------------------------------------------------------
    // MonitorSnapshot / work_area_for_window（task 8.1・DD15 基盤・4.7）
    // -------------------------------------------------------------------------

    use super::{MonitorSnapshot, work_area_for_window};
    use crate::placement::resolver::RectPx;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
        RectPx {
            left,
            top,
            right,
            bottom,
        }
    }

    /// 複数モニタ: 窓中心が属するモニタの work area が返る（縦位置・寸法の異なる
    /// 2 面で中心帰属を固定する）。
    #[test]
    fn work_area_for_window_picks_monitor_containing_center() {
        let snapshot = MonitorSnapshot {
            work_areas: vec![
                rect(0, 0, 1920, 1040),       // primary
                rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（負 top）
            ],
        };
        // 中心 (2500, 500) → 右モニタ
        let window = rect(2100, 100, 2900, 900);
        assert_eq!(
            work_area_for_window(&snapshot, window),
            Some(rect(1920, -213, 4480, 1227))
        );
        // 中心 (960, 520) → primary
        let window = rect(660, 220, 1260, 820);
        assert_eq!(
            work_area_for_window(&snapshot, window),
            Some(rect(0, 0, 1920, 1040))
        );
    }

    /// 負座標モニタ（プライマリの左）でも中心帰属が成立する。
    #[test]
    fn work_area_for_window_handles_negative_coords() {
        let snapshot = MonitorSnapshot {
            work_areas: vec![rect(0, 0, 1920, 1040), rect(-1920, -40, 0, 1000)],
        };
        let window = rect(-1500, 100, -700, 700); // 中心 (-1100, 400)
        assert_eq!(
            work_area_for_window(&snapshot, window),
            Some(rect(-1920, -40, 0, 1000))
        );
    }

    /// 境界中心の決定論: 帰属判定は half-open（right/bottom 排他）＝共有辺上の中心は
    /// 右隣モニタへ属する。複数矩形が同一中心を含む（重複）場合は昇順 index 先勝ち。
    #[test]
    fn work_area_for_window_boundary_center_is_half_open_and_first_match_wins() {
        let a = rect(0, 0, 1920, 1040);
        let b = rect(1920, 0, 3840, 1040);
        let snapshot = MonitorSnapshot {
            work_areas: vec![a, b],
        };
        // 中心 x=1920 ちょうど（共有辺）→ a の right は排他ゆえ b
        let window = rect(1520, 220, 2320, 820); // 中心 (1920, 520)
        assert_eq!(work_area_for_window(&snapshot, window), Some(b));

        // 重複 2 面が同一中心を含む → 先勝ち（昇順 index）
        let overlap = MonitorSnapshot {
            work_areas: vec![a, rect(-10, -10, 2000, 1100)],
        };
        let window = rect(700, 300, 1300, 700); // 中心 (1000, 500) は両方に属す
        assert_eq!(work_area_for_window(&overlap, window), Some(a));
    }

    /// どのモニタにも属さない中心 → 最近傍（中心→矩形 clamp 点の自乗距離最小・
    /// 等距離は昇順 index 先勝ち）。
    #[test]
    fn work_area_for_window_off_all_monitors_returns_nearest() {
        let a = rect(0, 0, 1920, 1040);
        let b = rect(1920, 0, 3840, 1040);
        let snapshot = MonitorSnapshot {
            work_areas: vec![a, b],
        };
        // 中心 (4340, 500): b の右外 500px・a の右外 2420px → b
        let window = rect(4040, 200, 4640, 800);
        assert_eq!(work_area_for_window(&snapshot, window), Some(b));
        // 中心 (-1000, 2000): a の clamp 点 (0,1040) が b の (1920,1040) より近い → a
        let window = rect(-1300, 1700, -700, 2300);
        assert_eq!(work_area_for_window(&snapshot, window), Some(a));
        // 等距離: 中心 (1920, 2000) は a clamp (1920,1040)・b clamp (1920,1040) と
        // 同距離 → 先勝ちで a
        let window = rect(1620, 1700, 2220, 2300);
        assert_eq!(work_area_for_window(&snapshot, window), Some(a));
    }

    /// 空 snapshot → `None`（架空の既定矩形を発明しない）。
    #[test]
    fn work_area_for_window_empty_snapshot_is_none() {
        let snapshot = MonitorSnapshot { work_areas: vec![] };
        assert_eq!(work_area_for_window(&snapshot, rect(0, 0, 100, 100)), None);
    }

    // -------------------------------------------------------------------------
    // on_char_drag（4.2/4.3/4.4・U4）
    // -------------------------------------------------------------------------

    /// Tunnel フェーズは無視する（donor on_shell_drag と同じ規約）。
    #[test]
    fn on_char_drag_tunnel_phase_is_ignored() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
            .id();
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        let ev = Phase::Tunnel(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    /// Bubble フェーズ: キャラ窓の WindowPos（wndproc 更新済み想定・物理 px）に
    /// offset を加算した位置へバルーンが追従する。再スケールなしの檻として
    /// 96 の倍数を避けた座標で完全一致を要求する（U4・3.3）。
    #[test]
    fn on_char_drag_bubble_moves_balloon_by_offset() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: 498, y: -37 };
        // wndproc がドラッグ中に更新した後のキャラ窓位置を模す
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow { balloon, offset },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        // donor 同様、イベントは消費しない（伝播続行＝false）
        assert!(!on_char_drag(&mut world, window, window, &ev));

        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1207 + offset.x,
                y: 653 + offset.y
            }
        );
        // キャラ窓自体はハンドラでは動かさない（wndproc の領分）
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 653 });
    }

    /// キャラ窓に WindowPos（position）が無ければ何もしない（false・panic なし）。
    #[test]
    fn on_char_drag_without_window_pos_is_noop() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
            .id();
        let window = world
            .spawn((
                fake_handle(0x1000),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: 11, y: 22 },
                },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
    }

    /// BalloonFollow の無い entity への Bubble は no-op（false・panic なし）。
    #[test]
    fn on_char_drag_without_balloon_follow_is_noop() {
        let mut world = World::new();
        let window = world
            .spawn((fake_handle(0x1000), window_pos_at(50, 60)))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 50, y: 60 });
    }
}

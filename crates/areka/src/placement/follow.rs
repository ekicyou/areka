//! バルーン追従コンポーネントと窓移動の公開 API。
//!
//! - [`BalloonFollow`]: キャラ窓に付与する追従 Component（配置時確定の暫定 offset・
//!   物理 px・4.4。offset は `ScopePlacement.balloon_offset` の転写）
//! - [`on_char_drag`]: `OnDrag` ハンドラ（mock-shell donor `on_shell_drag` の一般化。
//!   マーカー全走査ではなく `BalloonFollow.balloon` の `WindowHandle` を直接引く）
//! - [`on_balloon_drag`]: バルーン窓の `OnDrag` ハンドラ（4.8・DD16・task 8.3）——
//!   バルーン単独ドラッグの相対位置記憶。`BalloonFollow.offset` を
//!   `balloon_pos − char_pos` へ更新する（キャラ窓は不動）
//! - [`move_window_to`]: R7 公開 API（UI スレッド関数・物理 px スクリーン座標直渡し）
//! - [`MonitorSnapshot`]／[`work_area_for_window`]: bottom 吸着ドラッグ（4.7・DD15）の
//!   基盤——全モニタ work area 集合の Resource と窓中心→モニタ解決の純粋ヘルパ。
//!   消費側の Y 釘付けは [`on_char_drag`] 内（`BottomSnap` キャラ窓のみ・task 8.2）
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
use wintf::ecs::{Point, SetWindowPosCommand, SizeI, WindowHandle, WindowPos};

use super::resolver::{PointPx, RectPx};
use super::spawn::BottomSnap;

/// キャラ窓に付与するバルーン追従 Component（4.2/4.4/4.8）。
///
/// `offset` の初期値は配置時に確定する暫定 offset（物理 px・
/// `ScopePlacement.balloon_offset` の転写＝P5 幾何の暫定規則。正式な配置規則は
/// balloon 表示系の後続が所有する・4.4）。バルーン単独ドラッグでユーザーが
/// ずらすと [`on_balloon_drag`] が `balloon_pos − char_pos` へ**記憶更新**し、
/// 以後のキャラ窓ドラッグ・[`move_window_to`] は調整後 offset で追従する
/// （4.8・セッション内のみ・永続化は M-life の領分。
/// 旧挙動「次のキャラ窓ドラッグで初期 offset へスナップバック」は
/// 2026-07-11 要件 4.8 により仕様退役）。
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
/// # bottom 吸着の Y 釘付け（4.7・DD15・task 8.2）
///
/// `BottomSnap` marker を持つキャラ窓は、wndproc 移動後に [`MonitorSnapshot`]＋
/// [`work_area_for_window`] で「窓中心が属するモニタの `work_area.bottom − h`」を
/// 求め、Y がずれていれば自窓へ `SetWindowPosCommand` で再釘付けする
/// （X は不変・物理 px 素通し・再スケールなし）。モニタを跨いだら跨いだ先の
/// 下端へ再吸着し、`Free`（marker なし）は従来どおり全方向移動。バルーン追従は
/// **釘付け後**のキャラ窓座標基準で offset 加算する。
///
/// イベントは消費しない（常に `false`＝伝播続行。donor on_shell_drag と同じ規約）。
pub(crate) fn on_char_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(_) => {
            // キャラ窓の現在位置・寸法（wndproc が実窓位置から更新済み・物理 px）
            let Some(wp) = world.get::<WindowPos>(entity) else {
                return false;
            };
            let Some(mut pos) = wp.position else {
                return false;
            };
            let size = wp.size;

            // bottom 吸着の Y 釘付け（BottomSnap キャラ窓のみ・4.7）
            if world.get::<BottomSnap>(entity).is_some()
                && let Some(target_y) = bottom_snap_target_y(world, pos, size)
                && pos.y != target_y
            {
                enqueue_window_move(world, entity, pos.x, target_y);
                // 以降のバルーン追従は釘付け後座標を基準にする
                pos.y = target_y;
            }

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

/// `OnDrag` ハンドラ: バルーン窓単独ドラッグの相対位置記憶（4.8・DD16・task 8.3）。
///
/// バルーン窓自体は `DragConfig { move_window: true }` により wndproc レベルで
/// 移動済み（`WindowPos.position` 更新済み・物理 px）。キャラ窓は不動で、
/// bottom 吸着（4.7）の対象外＝バルーンドラッグでキャラ窓の Y 釘付けは発火しない。
///
/// 本ハンドラは `BalloonFollow.balloon == ドラッグ中バルーン` のキャラ窓を
/// query 走査で逆引きし（窓は高々数個・全走査で十分）、
/// `BalloonFollow.offset = balloon_pos − char_pos`（物理 px・再スケールなし・U4）
/// へ更新するだけ。既存 consumer（[`on_char_drag`]／[`move_window_to`]）は
/// 無改変で調整後 offset を読む（DD16・4.4 の恒等式
/// `balloon_pos − char_pos ≡ offset` は更新後も不変）。記憶はセッション内のみ
/// （永続化 ghost.dat は M-life の領分）。
///
/// イベントは消費しない（常に `false`＝伝播続行。[`on_char_drag`] と同じ規約）。
pub(crate) fn on_balloon_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(_) => {
            // バルーン窓の現在位置（wndproc が実窓位置から更新済み・物理 px）
            let Some(wp) = world.get::<WindowPos>(entity) else {
                return false;
            };
            let Some(balloon_pos) = wp.position else {
                return false;
            };

            // BalloonFollow.balloon == 自バルーンのキャラ窓を逆引きし offset 更新
            let mut chars = world.query::<(&mut BalloonFollow, &WindowPos)>();
            for (mut follow, char_wp) in chars.iter_mut(world) {
                if follow.balloon != entity {
                    continue;
                }
                let Some(char_pos) = char_wp.position else {
                    continue;
                };
                // 不変条件: 両者とも仮想スクリーン座標範囲のため、減算が i32 を
                // 溢れることはない（溢れは入力源の異常・on_char_drag と同じ流儀）
                debug_assert!(
                    balloon_pos.x.checked_sub(char_pos.x).is_some()
                        && balloon_pos.y.checked_sub(char_pos.y).is_some(),
                    "window positions out of virtual-screen range: {balloon_pos:?} - {char_pos:?}"
                );
                follow.offset = PointPx {
                    x: balloon_pos.x - char_pos.x,
                    y: balloon_pos.y - char_pos.y,
                };
            }
            false
        }
    }
}

/// `BottomSnap` キャラ窓の釘付け先 Y（`work_area.bottom − h`・物理 px）を求める（4.7・DD15）。
///
/// 釘付け不能（設計上の graceful degradation）は `None`:
/// - `WindowPos.size` 不在・非正寸法: `bottom − h` の h が得られない。
///   `WindowPos::default()` の size は `CW_USEDEFAULT`（負のセンチネル）であり、
///   これで釘付けると `saturating_sub` が `i32::MAX` へ飛ぶため **w/h > 0 のみ**
///   釘付け対象とする（spawn 経由の窓は必ず実寸持ちのため実運用では起きない）
/// - [`MonitorSnapshot`] Resource 不在: main.rs のフォールバック経路（dummy 窓）は
///   snapshot を挿入しない設計のため、不在＝失敗ではなく「吸着なしで従来動作」。
///   ドラッグ移動イベントごとに発火する経路ゆえ `warn!` は spam になる——設計上の
///   縮退として `debug!` に留める（log-first 規律の対象は*失敗*経路・ここは仕様内縮退）
/// - 空 snapshot: [`work_area_for_window`] が `None`（架空の既定矩形を発明しない）
fn bottom_snap_target_y(world: &World, pos: Point, size: Option<SizeI>) -> Option<i32> {
    let Some(size) = size.filter(|s| s.width > 0 && s.height > 0) else {
        debug!(?size, "BottomSnap 窓の WindowPos.size が不在か非正寸法のため Y 釘付けをスキップ");
        return None;
    };
    let Some(snapshot) = world.get_resource::<MonitorSnapshot>() else {
        debug!("MonitorSnapshot 未挿入（フォールバック経路）のため Y 釘付けをスキップ");
        return None;
    };
    let window = RectPx {
        left: pos.x,
        top: pos.y,
        right: pos.x.saturating_add(size.width),
        bottom: pos.y.saturating_add(size.height),
    };
    let wa = work_area_for_window(snapshot, window)?;
    // 実 work area／窓寸で溢れない範囲だが、極端入力でも panic しない契約
    // （resolver の saturating 演算と同じ防波堤）
    Some(wa.bottom.saturating_sub(size.height))
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

    // -------------------------------------------------------------------------
    // bottom 吸着ドラッグ（task 8.2・4.7・DD15）
    // 合成 MonitorSnapshot を Resource 注入した headless World で、BottomSnap
    // キャラ窓の Y 釘付け・X 素通し・モニタ跨ぎ再吸着・Free 非矯正・
    // バルーンの釘付け後座標基準追従を決定論検証する（偽装境界）。
    // -------------------------------------------------------------------------

    use wintf::ecs::SizeI;

    use crate::placement::spawn::BottomSnap;

    /// position＋size 付きの WindowPos（spawn の `window_pos` と同型）。
    fn window_pos_sized(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
        WindowPos {
            position: Some(Point { x, y }),
            size: Some(SizeI::new(w, h)),
            ..Default::default()
        }
    }

    /// 単一モニタの合成 snapshot（物理 px・96 の倍数を避けた下端で再スケール檻）。
    fn single_monitor_snapshot() -> MonitorSnapshot {
        MonitorSnapshot {
            work_areas: vec![rect(0, 0, 1920, 1043)],
        }
    }

    /// (a)(b) BottomSnap 窓のドラッグ: Y は現在モニタの `work_area.bottom − h` へ
    /// 矯正され、X はドラッグ位置のまま素通し（物理 px・再スケールなし・4.7）。
    #[test]
    fn on_char_drag_bottom_snap_pins_y_to_work_area_bottom() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        // wndproc がドラッグ中に更新した「下端から浮いた」位置を模す（中心は面内）
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 217, 434, 687),
                BottomSnap,
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // Y=1043−687=356 へ釘付け・X=1207 は不変
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 356 });
    }

    /// 釘付け済み（Y が既に下端一致）の BottomSnap 窓は位置不変（余計な移動なし）。
    #[test]
    fn on_char_drag_bottom_snap_already_pinned_is_stable() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(731, 356, 434, 687), // 1043−687=356（既に下端）
                BottomSnap,
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    }

    /// (c) モニタ跨ぎ: 窓中心が下端の異なる隣モニタへ移ったら、跨いだ先の
    /// work area 下端へ再吸着する（4.7「跨いだ先のモニタの下端へ再吸着」）。
    #[test]
    fn on_char_drag_bottom_snap_resnaps_to_crossed_monitor_bottom() {
        let mut world = World::new();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![
                rect(0, 0, 1920, 1040),       // primary
                rect(1920, -213, 4480, 1227), // 右の高解像度モニタ（下端が異なる）
            ],
        });
        // 中心 x = 2531+434/2 = 2748 → 右モニタ帰属
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(2531, 353, 434, 687), // 353 は旧モニタの下端由来
                BottomSnap,
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // 右モニタの下端 1227−687=540 へ再吸着・X 素通し
        assert_eq!(position_of(&world, window), Point { x: 2531, y: 540 });
    }

    /// (d) BottomSnap の無い窓（free スコープ・吸着なし）は snapshot があっても
    /// 矯正されない（全方向移動・挙動不変・4.7）。
    #[test]
    fn on_char_drag_free_window_is_not_pinned() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: 498, y: -37 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 217, 434, 687),
                BalloonFollow { balloon, offset },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // Y 矯正なし（ドラッグ位置のまま）・バルーンは素の位置基準で追従
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 217 });
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1207 + offset.x,
                y: 217 + offset.y
            }
        );
    }

    /// (e) バルーン追従は**釘付け後**のキャラ窓座標基準（pinned + offset）。
    /// 釘付け前の素のドラッグ位置基準だと Y がずれる檻。
    #[test]
    fn on_char_drag_balloon_follows_pinned_position() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: -400, y: 25 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 217, 434, 687),
                BottomSnap,
                BalloonFollow { balloon, offset },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        let char_pos = position_of(&world, window);
        assert_eq!(char_pos, Point { x: 1207, y: 356 }); // 1043−687=356
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: char_pos.x + offset.x,
                y: char_pos.y + offset.y
            }
        );
    }

    /// (+) MonitorSnapshot Resource 不在（main.rs フォールバック経路）: 釘付けは
    /// 行わず panic もしない（設計上の graceful degradation・バルーン追従は生きる）。
    #[test]
    fn on_char_drag_bottom_snap_without_snapshot_is_graceful() {
        let mut world = World::new(); // Resource 未挿入
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let offset = PointPx { x: 11, y: 22 };
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 217, 434, 687),
                BottomSnap,
                BalloonFollow { balloon, offset },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));

        // 釘付けなし・素の位置基準で追従（no-pin degradation）
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 217 });
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1207 + offset.x,
                y: 217 + offset.y
            }
        );
    }

    /// (+) WindowPos.size 不在の BottomSnap 窓: `bottom − h` を計算できないため
    /// 釘付けせず panic もしない（graceful degradation）。
    #[test]
    fn on_char_drag_bottom_snap_without_size_is_graceful() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        let mut wp = window_pos_at(1207, 217);
        wp.size = None;
        let window = world.spawn((fake_handle(0x1000), wp, BottomSnap)).id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 217 });
    }

    /// (+) `WindowPos::default()` の size は `CW_USEDEFAULT`（負のセンチネル）。
    /// 非正寸法では釘付けしない（saturating_sub が i32::MAX へ飛ぶ暴走の檻）。
    #[test]
    fn on_char_drag_bottom_snap_sentinel_size_is_graceful() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot());
        // window_pos_at は ..Default::default() ゆえ size=Some(CW_USEDEFAULT×2)
        let window = world
            .spawn((fake_handle(0x1000), window_pos_at(1207, 217), BottomSnap))
            .id();

        let ev = Phase::Bubble(drag_event(window));
        assert!(!on_char_drag(&mut world, window, window, &ev));
        assert_eq!(position_of(&world, window), Point { x: 1207, y: 217 });
    }

    // -------------------------------------------------------------------------
    // on_balloon_drag: バルーン単独ドラッグの相対位置記憶（task 8.3・4.8・DD16）
    // wndproc がバルーン窓を移動済み（WindowPos 更新済み）の状態を模して呼び、
    // offset の記憶更新・キャラ窓不動・以後の consumer 追従を決定論検証する。
    // 座標は 96 の倍数を避け、x/y で符号・値の異なる offset を使う
    // （符号取り違え・軸取り違えの檻）。
    // -------------------------------------------------------------------------

    use super::on_balloon_drag;

    /// Tunnel フェーズは無視する（on_char_drag と同じ規約・offset 不変）。
    #[test]
    fn on_balloon_drag_tunnel_phase_is_ignored() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
            .id();
        let initial = PointPx { x: 11, y: 22 };
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();

        let ev = Phase::Tunnel(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));
        assert_eq!(
            world.get::<BalloonFollow>(char_w).unwrap().offset,
            initial,
            "Tunnel では offset を更新しない"
        );
    }

    /// (a)(c) バルーン単独ドラッグ: 所有キャラ窓の `BalloonFollow.offset` が
    /// `balloon_pos − char_pos` へ更新され（4.8）、キャラ窓は不動。
    /// x/y の符号が異なる期待値で減算の向き（balloon − char）を固定する檻。
    #[test]
    fn on_balloon_drag_updates_offset_and_char_window_is_unmoved() {
        let mut world = World::new();
        // wndproc がドラッグ中に更新した後のバルーン位置を模す
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(1729, 401)))
            .id();
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon,
                    offset: PointPx { x: -412, y: -25 },
                },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(balloon));
        // イベントは消費しない（伝播続行＝false）
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

        // offset = balloon − char = (1729−1207, 401−653) = (+522, −252)
        // （char − balloon なら (−522, +252)＝符号取り違えの檻）
        assert_eq!(
            world.get::<BalloonFollow>(char_w).unwrap().offset,
            PointPx { x: 522, y: -252 }
        );
        // (c) キャラ窓は不動（4.8: バルーンのみ移動・bottom 吸着の対象外）
        assert_eq!(position_of(&world, char_w), Point { x: 1207, y: 653 });
        // バルーン自身もハンドラでは動かさない（wndproc の領分）
        assert_eq!(position_of(&world, balloon), Point { x: 1729, y: 401 });
    }

    /// (b) バルーン単独ドラッグ後の `move_window_to`: 調整後 offset で追従する
    /// （初期 offset へのスナップバックは仕様退役・4.8）。
    #[test]
    fn move_window_to_after_balloon_drag_follows_adjusted_offset() {
        let mut world = World::new();
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
            .id();
        let initial = PointPx { x: -412, y: -25 };
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();

        // バルーン単独ドラッグ（wndproc がバルーンを (613, 407) へ移動済み）
        world.get_mut::<WindowPos>(balloon).unwrap().position = Some(Point { x: 613, y: 407 });
        let ev = Phase::Bubble(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

        let adjusted = PointPx {
            x: 613 - 1207,
            y: 407 - 653,
        };
        assert_ne!(adjusted, initial, "檻の前提: 調整後 offset は初期値と異なる");
        assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, adjusted);

        // 次のキャラ窓移動 API は調整後 offset で追従（consumer 無改変・DD16）
        assert!(move_window_to(&mut world, char_w, 1751, 893));
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 1751 + adjusted.x,
                y: 893 + adjusted.y
            }
        );
    }

    /// (b)(c) 8.2＋8.3 の合成: BottomSnap キャラ窓の場合——バルーンドラッグでは
    /// キャラ窓の Y 釘付けは発火せず（不動・4.8「bottom 吸着の対象外」）、
    /// その後のキャラ窓ドラッグは Y 釘付けの**後**に調整後 offset で追従する。
    #[test]
    fn on_char_drag_after_balloon_drag_pins_y_and_follows_adjusted_offset() {
        let mut world = World::new();
        world.insert_resource(single_monitor_snapshot()); // 下端 1043
        let balloon = world
            .spawn((fake_handle(0x2000), window_pos_at(500, 300)))
            .id();
        let initial = PointPx { x: -412, y: -25 };
        // 釘付け済み位置（Y=1043−687=356）から開始する BottomSnap キャラ窓
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(1207, 356, 434, 687),
                BottomSnap,
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();

        // バルーン単独ドラッグ（wndproc がバルーンを (831, 149) へ移動済み）
        world.get_mut::<WindowPos>(balloon).unwrap().position = Some(Point { x: 831, y: 149 });
        let ev = Phase::Bubble(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

        // キャラ窓は不動（Y 釘付けも発火しない・4.8）
        assert_eq!(position_of(&world, char_w), Point { x: 1207, y: 356 });
        let adjusted = PointPx {
            x: 831 - 1207,
            y: 149 - 356,
        };
        assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, adjusted);

        // 次のキャラ窓ドラッグ: wndproc が下端から浮いた位置へ動かした想定
        world.get_mut::<WindowPos>(char_w).unwrap().position = Some(Point { x: 903, y: 211 });
        let ev = Phase::Bubble(drag_event(char_w));
        assert!(!on_char_drag(&mut world, char_w, char_w, &ev));

        // 8.2: Y は 1043−687=356 へ釘付け・X 素通し
        assert_eq!(position_of(&world, char_w), Point { x: 903, y: 356 });
        // 8.3: バルーンは釘付け後座標＋**調整後** offset（初期 offset だと不一致）
        assert_eq!(
            position_of(&world, balloon),
            Point {
                x: 903 + adjusted.x,
                y: 356 + adjusted.y
            }
        );
    }

    /// (d) 複数スコープ: ドラッグしたバルーンを所有するキャラ窓の offset だけが
    /// 更新され、他スコープの offset・窓位置は不干渉（誤マッチの檻）。
    #[test]
    fn on_balloon_drag_updates_only_matching_scope_offset() {
        let mut world = World::new();
        let balloon0 = world
            .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
            .id();
        let char0 = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(1207, 653),
                BalloonFollow {
                    balloon: balloon0,
                    offset: PointPx { x: -412, y: -25 },
                },
            ))
            .id();
        let balloon1 = world
            .spawn((fake_handle(0x4000), window_pos_at(1334, 1044)))
            .id();
        let offset1 = PointPx { x: 285, y: -19 };
        let char1 = world
            .spawn((
                fake_handle(0x3000),
                window_pos_at(1049, 1063),
                BalloonFollow {
                    balloon: balloon1,
                    offset: offset1,
                },
            ))
            .id();

        let ev = Phase::Bubble(drag_event(balloon0));
        assert!(!on_balloon_drag(&mut world, balloon0, balloon0, &ev));

        // scope0 の offset は balloon0 − char0 = (−506, −270) へ更新
        assert_eq!(
            world.get::<BalloonFollow>(char0).unwrap().offset,
            PointPx { x: -506, y: -270 }
        );
        // scope1 の offset・窓位置は不変（誤マッチなし）
        assert_eq!(world.get::<BalloonFollow>(char1).unwrap().offset, offset1);
        assert_eq!(position_of(&world, char1), Point { x: 1049, y: 1063 });
        assert_eq!(position_of(&world, balloon1), Point { x: 1334, y: 1044 });
    }

    /// (+) バルーンの `WindowPos.position` 不在は no-op（false・panic なし・
    /// offset 不変）。所有キャラ窓の position 不在も skip で panic しない。
    #[test]
    fn on_balloon_drag_without_positions_is_graceful() {
        let mut world = World::new();

        // バルーン側 position 不在 → offset 不変
        let mut wp = window_pos_at(0, 0);
        wp.position = None;
        let balloon = world.spawn((fake_handle(0x2000), wp)).id();
        let initial = PointPx { x: 11, y: 22 };
        let char_w = world
            .spawn((
                fake_handle(0x1000),
                window_pos_at(50, 60),
                BalloonFollow {
                    balloon,
                    offset: initial,
                },
            ))
            .id();
        let ev = Phase::Bubble(drag_event(balloon));
        assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));
        assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, initial);

        // キャラ側 position 不在 → skip（panic なし・offset 不変）
        let balloon2 = world
            .spawn((fake_handle(0x4000), window_pos_at(70, 80)))
            .id();
        let mut char_wp = window_pos_at(0, 0);
        char_wp.position = None;
        let char2 = world
            .spawn((
                fake_handle(0x3000),
                char_wp,
                BalloonFollow {
                    balloon: balloon2,
                    offset: initial,
                },
            ))
            .id();
        let ev = Phase::Bubble(drag_event(balloon2));
        assert!(!on_balloon_drag(&mut world, balloon2, balloon2, &ev));
        assert_eq!(world.get::<BalloonFollow>(char2).unwrap().offset, initial);

        // 所有キャラ窓が 1 つも無いバルーン → no-op（panic なし）
        let orphan = world
            .spawn((fake_handle(0x5000), window_pos_at(10, 20)))
            .id();
        let ev = Phase::Bubble(drag_event(orphan));
        assert!(!on_balloon_drag(&mut world, orphan, orphan, &ev));
    }
}

//! ウィンドウライフサイクルおよびディスプレイ変更ハンドラ
//!
//! WM_ERASEBKGND, WM_PAINT, WM_CLOSE, WM_ENDSESSION, WM_DISPLAYCHANGE
//!
//! WM_NCCREATE / WM_NCDESTROY は扱わない。ウィンドウの生成・破棄はライブラリと `WindowRegistry` の
//! drop 駆動が所管する。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::{debug, info, warn};
use windows::Win32::Foundation::*;

use crate::ecs::window::transition_diag::{self, MSG_DISPLAYCHANGE, MsgRecord};
use crate::ecs::window::{OnCloseRequest, OnSessionEnd};
use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// 「対象 entity が既に破棄済み（despawn 済み）ゆえ正常終了系として打ち切った」ことを
/// 終了時ログから grep で引くための接頭辞。
///
/// entity 不在＝破棄済み（正常終了系・`debug!`）／entity は実在するが規約 component が欠落
/// （真の異常・`warn!`）という区別は areka 側と同一で、終了時ログの読み手が主体を問わず 1 語で
/// 引けるよう、文字列は `areka_placement::diag::DESPAWNED_SKIP_TAG` と同一に保つこと。
/// wintf から areka への import は禁止なので、定数は共有せず同じ値を置く。
pub(crate) const DESPAWNED_SKIP_TAG: &str = "[despawn-skip]";

/// WM_ERASEBKGND: 背景消去要求
///
/// GPU 合成（WUC）が描画面を管理するため、背景消去をスキップする
#[inline]
pub(super) fn WM_ERASEBKGND(
    _world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    Some(LRESULT(1))
}

/// WM_PAINT: 再描画要求
///
/// GPU 合成（WUC）は OS 管理のため、常に `DefWindowProcW` へ委譲する（`None` 返却）
#[inline]
pub(super) fn WM_PAINT(
    _world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    None
}

/// WM_CLOSE: ウィンドウクローズ要求
///
/// 対象 Entity の除去要求（despawn）として処理する。`DestroyWindow` を直叩きせず、
/// `Window` コンポーネント消失を `RemovedComponents<Window>`（`reconcile_window_registry`）が
/// 検知し、レジストリ要素の drop で `DestroyWindow` させることで、ハンドル破棄を Entity
/// ライフサイクルに一致させる。
///
/// 同期再入時の二重借用を避けるため `try_borrow_mut` を用い、既に借用中なら safe-skip する
/// （パニックしない）。`Some(LRESULT(0))` を返して既定手続きの `DestroyWindow` を抑止する。
///
/// # 陳腐化した id の打ち切り
///
/// 届いた `entity` は既に破棄済みのことがある（despawn から登録簿の掃除までの間に利用者や OS が
/// 投げた `WM_CLOSE` が届く）。そのまま `despawn` すると `bevy_ecs` が `Could not despawn entity`
/// を `warn!` で出す（`bevy_ecs-0.18.1` `src/world/mod.rs:1462-1469`）が、これは終了処理の
/// 正常終了系で、警告にすると本物の異常が良性ノイズに埋もれる。ゆえに [`DESPAWNED_SKIP_TAG`] の
/// `debug!` で打ち切る。生存 entity への挙動は変わらない。
///
/// # OS の閉鎖要求を利用側へ渡す
///
/// 生存 entity が [`OnCloseRequest`] を持つときは、窓を消さずにその関数を呼ぶ
/// （Alt＋F4・`taskkill`（`/F` なし）・「タスクの終了」を利用側の終了操作へ乗せる口）。
/// 持たない窓は従来どおり despawn する。破棄済み entity の打ち切りと戻り値
/// `Some(LRESULT(0))` はどちらの腕でも変わらない。
#[inline]
pub(super) fn WM_CLOSE(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    if let Ok(mut w) = world.try_borrow_mut() {
        if w.world().get_entity(entity).is_err() {
            debug!(
                entity = ?entity,
                "{DESPAWNED_SKIP_TAG} WM_CLOSE: 対象 entity は既に破棄済み（despawn）→ \
                 除去要求を正常系として打ち切り"
            );
        } else if let Some(cb) = w.world().get::<OnCloseRequest>(entity).copied() {
            info!(
                event = "os_close_request",
                entity = ?entity,
                "[WM_CLOSE] 閉鎖要求を利用側の関数へ渡す"
            );
            (cb.0)(w.world_mut(), entity);
        } else {
            w.world_mut().despawn(entity);
        }
    }
    Some(LRESULT(0))
}

/// WM_ENDSESSION: OS のセッションの終了（シャットダウン・再起動・ログオフ）の確定／取りやめ
///
/// wParam が真で生存 entity が [`OnSessionEnd`] を持てば、その関数を World 借用中に呼ぶ
/// （関数から戻った直後にプロセスが終了させられうる）。取りやめ（wParam 偽）・部品なし・
/// 破棄済み・World の借用中はどれも記録だけで呼ばない。最初の受け手が全ゴースト窓の
/// entity を破棄するので、残りの窓へ届いた 2 通目以降は破棄済みの打ち切りで止まる。
/// 戻り値はどの腕も `Some(LRESULT(0))`（`WM_ENDSESSION` を処理したら 0 を返す）。
///
/// `WM_QUERYENDSESSION` の腕は持たない（既定の手続きが TRUE＝終了を許すを返す）。
#[inline]
pub(super) fn WM_ENDSESSION(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> HandlerResult {
    if wparam.0 == 0 {
        debug!(
            event = "os_session_end_cancelled",
            entity = ?entity,
            "[WM_ENDSESSION] セッションの終了は取りやめられた → 何もしない"
        );
        return Some(LRESULT(0));
    }
    let Ok(mut w) = world.try_borrow_mut() else {
        warn!(
            event = "os_session_end_world_busy",
            entity = ?entity,
            "[WM_ENDSESSION] World が借用中でセッションの終了の受け手を呼べない → \
             後始末をせずに終わるので起動中の印が残り、次の起動は前回落ちた扱い（Ref6/7 付き）になる"
        );
        return Some(LRESULT(0));
    };
    if w.world().get_entity(entity).is_err() {
        debug!(
            entity = ?entity,
            "{DESPAWNED_SKIP_TAG} WM_ENDSESSION: 対象 entity は既に破棄済み（despawn）→ \
             セッションの終了は先の受け手で済んだものとして打ち切り"
        );
    } else if let Some(cb) = w.world().get::<OnSessionEnd>(entity).copied() {
        info!(
            event = "os_session_end",
            entity = ?entity,
            lparam = lparam.0,
            "[WM_ENDSESSION] セッションの終了を利用側の関数へ渡す"
        );
        (cb.0)(w.world_mut(), entity);
    } else {
        debug!(
            entity = ?entity,
            "[WM_ENDSESSION] セッションの終了の関数を持たない窓 → 何もしない"
        );
    }
    Some(LRESULT(0))
}

/// WM_DISPLAYCHANGE: ディスプレイ構成変更通知
///
/// Appリソースのmark_display_changeを呼び出す
///
/// # 遷移観測
///
/// 受理を `msg` レコードとして残す。モニタ表の更新は**次 tick の `Update`** で
/// `detect_display_change_system` が行うため、受理の刻印（直近 tick の番号）と
/// `monitor` レコードの刻印は 1 フレームずれ得る——1 本の時系列に並べたときに
/// 「受理はいつで、表の更新はどのフレームか」が読めることが本記録の目的である。
#[inline]
pub(super) fn WM_DISPLAYCHANGE(
    world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    if transition_diag::is_enabled() {
        transition_diag::emit_line(&transition_diag::msg_line(&MsgRecord {
            stamp: transition_diag::stamp(),
            msg: MSG_DISPLAYCHANGE,
            hwnd,
            in_swp: crate::ecs::window::is_self_initiated(),
            since_flush_us: transition_diag::since_flush_us(),
        }));
    }

    if let Ok(mut world_borrow) = world.try_borrow_mut() {
        if let Some(mut app) = world_borrow
            .world_mut()
            .get_resource_mut::<crate::ecs::App>()
        {
            app.mark_display_change();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::HWND;

    /// テスト用に `Window` コンポーネント付き Entity を spawn する。
    /// ヘッドレス: `on_window_add` フックはコマンドを deferred enqueue するのみで
    /// （実窓を inline 生成しない）、コマンド未フラッシュのため OS リソースに触れない。
    fn spawn_window_entity(world: &Rc<RefCell<EcsWorld>>) -> Entity {
        world
            .borrow_mut()
            .world_mut()
            .spawn(crate::ecs::window::Window::default())
            .id()
    }

    /// WM_CLOSE は対象 Entity を despawn し（`DestroyWindow` 直叩きをしない）、
    /// `Some(LRESULT(0))`（既定破棄抑止）を返す。
    /// ヘッドレス: 実 HWND / メッセージループ不要。
    #[test]
    fn wm_close_despawns_target_entity() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = spawn_window_entity(&world);

        assert!(
            world.borrow().world().get_entity(entity).is_ok(),
            "spawn 直後は Entity が生存しているべき"
        );

        let ret = WM_CLOSE(
            &world,
            entity,
            HWND(std::ptr::null_mut()),
            WPARAM(0),
            LPARAM(0),
        );

        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "WM_CLOSE は既定破棄抑止の Some(LRESULT(0)) を返すべき"
        );

        assert!(
            world.borrow().world().get_entity(entity).is_err(),
            "WM_CLOSE 後は対象 Entity が despawn されているべき（除去要求）"
        );
    }

    /// world が既に `borrow_mut` 保持中でも WM_CLOSE はパニックせず safe-skip する
    /// （`try_borrow_mut` による同期再入の二重借用回避）。
    /// 借用失敗のため Entity は残存する。
    #[test]
    fn wm_close_safe_skips_when_world_already_borrowed() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = spawn_window_entity(&world);

        // world を borrow_mut 保持したまま WM_CLOSE を呼ぶ（再入相当）。
        let guard = world.borrow_mut();
        let ret = WM_CLOSE(
            &world,
            entity,
            HWND(std::ptr::null_mut()),
            WPARAM(0),
            LPARAM(0),
        );
        drop(guard);

        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "再入時も Some(LRESULT(0)) を返す（safe-skip）"
        );

        assert!(
            world.borrow().world().get_entity(entity).is_ok(),
            "借用中の再入では despawn されず Entity が残存するべき"
        );
    }

    /// 捕捉ハーネスに渡す directive（本モジュールの `debug!` を通す最小構成）。
    const CAPTURE_DIRECTIVES: &str = "info,wintf::ecs::window_proc=debug";

    /// 陳腐化した id への `WM_CLOSE` は正常終了系として打ち切られる。
    ///
    /// despawn から登録簿の掃除までの間に届いた `WM_CLOSE` では、本ハンドラが回る時点で `entity` は
    /// 既に破棄済みである。
    ///
    /// # 「捕捉に警告が無い」を根拠にできない理由
    ///
    /// `bevy_ecs` の `Could not despawn entity` は `log::warn!` で出る（`World::despawn` は警告と
    /// `false` を同一分岐で返す）。これが tracing 側へ届くには `LogTracer` の大域インストールが要る。
    /// 本番は `areka` の `main.rs` の `tracing_subscriber` の `.init()` がそれを立てるが、捕捉ハーネス
    /// `capture_under_filter` は `with_default`（スレッドローカルの差替え）で大域ブリッジを立てない
    /// ため、`log` 発の記録は 1 件も届かない。ゆえに「捕捉に bevy の警告が無い」は恒真で、檻として
    /// 空虚である。
    ///
    /// そこで対照アームで `World::despawn` の戻り値 `false` を実測し、本体アームで自前の打ち切り行が
    /// 在ることを主張する。捕捉に対する陰性主張は wintf 自身の tracing 出力にだけ適用し、その
    /// 捕捉可能性は自己証明アームで先に示す。
    #[test]
    fn wm_close_skips_stale_entity_as_normal_teardown() {
        use crate::ecs::test_support::capture_under_filter;

        // ── ⑴ 対照アーム（非空虚性の証明）─────────────────────────────
        // 存在確認が無ければ陳腐化 id への despawn は `false` を返す＝bevy が
        // `Could not despawn entity` の WARN を 1 件出す状態と**同値**。
        {
            let world = Rc::new(RefCell::new(EcsWorld::new()));
            let entity = spawn_window_entity(&world);
            assert!(
                world.borrow_mut().world_mut().despawn(entity),
                "探針前提: 生存 entity への 1 回目の despawn は成功する"
            );
            assert!(
                !world.borrow_mut().world_mut().despawn(entity),
                "対照アーム: 陳腐化 id への despawn は false を返す\
                 （＝`Could not despawn entity` の警告 1 件）。\
                 ここが true なら本檻は恒真の空虚檻である"
            );
        }

        // ── ⑶ 自己証明アーム: 捕捉ハーネスは「在るとき」に警告を見られる ──────
        // これを示さずに後段の「WARN が無い」を主張すると、それ自体が空虚になる。
        {
            let probe = capture_under_filter(CAPTURE_DIRECTIVES, || {
                tracing::warn!("[harness-selfcheck] 捕捉可能性の自己証明");
            });
            assert!(
                probe.contains("WARN") && probe.contains("[harness-selfcheck]"),
                "捕捉ハーネスが tracing の WARN を見られない。\
                 後段の陰性主張が空虚になるため檻として成立しない: {probe}"
            );
        }

        // ── ⑵ 本体アーム ──────────────────────────────────────────
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = spawn_window_entity(&world);
        // despawn の後・登録簿の掃除の前に `WM_CLOSE` が届いた形の再現: entity は破棄済み。
        assert!(
            world.borrow_mut().world_mut().despawn(entity),
            "探針前提: 先行 despawn は成功する（以後この id は陳腐化している）"
        );

        let mut ret = None;
        let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
            ret = WM_CLOSE(
                &world,
                entity,
                HWND(std::ptr::null_mut()),
                WPARAM(0),
                LPARAM(0),
            );
        });

        // 投函契約・窓破棄の正準シグナルは不変（既定破棄抑止のまま）。
        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "陳腐化 id でも既定破棄抑止の Some(LRESULT(0)) を返す（投函契約は不変）"
        );

        let hits: Vec<&str> = out
            .lines()
            .filter(|l| l.contains(DESPAWNED_SKIP_TAG))
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "陳腐化 id に対する打ち切り行 `{DESPAWNED_SKIP_TAG}` がちょうど 1 行でない\
             （0 行なら存在確認が無く bevy の警告が出る経路のまま）: {out}"
        );
        assert!(
            hits[0].contains("DEBUG"),
            "破棄済みの打ち切りは正常終了系ゆえ DEBUG 水準（Req 6.2）: {}",
            hits[0]
        );
        assert!(
            hits[0].contains("WM_CLOSE"),
            "打ち切り行が自分の相（WM_CLOSE）を名乗っていない: {}",
            hits[0]
        );
        // wintf 自身の tracing 出力に警告以上が無いこと（⑶で捕捉可能性を証明済み）。
        // bevy の `log` 経由 warn は原理的にここへ来ないので⑴の対照アームが担当する。
        assert!(
            !out.contains("WARN") && !out.contains("ERROR"),
            "破棄済み entity に対して警告以上のログが出ている（Req 6.2 違反）: {out}"
        );
    }

    /// 生存 entity の `WM_CLOSE` は従来どおり despawn し、打ち切り行を出さない。
    ///
    /// 存在確認の述語を反転させる変異（`is_err()` → `is_ok()`）や「常に打ち切る」変異を
    /// 捕まえる——打ち切り行の**在ること**だけを見る檻は、その種の変異を素通しする。
    #[test]
    fn wm_close_on_live_entity_despawns_without_skip_line() {
        use crate::ecs::test_support::capture_under_filter;

        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = spawn_window_entity(&world);

        let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
            WM_CLOSE(
                &world,
                entity,
                HWND(std::ptr::null_mut()),
                WPARAM(0),
                LPARAM(0),
            );
        });

        // 窓破棄の正準シグナル（`world.despawn(entity)`）は不変。
        assert!(
            world.borrow().world().get_entity(entity).is_err(),
            "生存 entity は従来どおり despawn される（正準シグナルを弱めてはならない）"
        );
        assert!(
            !out.contains(DESPAWNED_SKIP_TAG),
            "生存経路で打ち切り行 `{DESPAWNED_SKIP_TAG}` が出ている\
             （正常系の窓破棄が「破棄済み」と誤報される）: {out}"
        );
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod lifecycle_tests;

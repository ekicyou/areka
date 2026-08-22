//! ウィンドウライフサイクルおよびディスプレイ変更ハンドラ
//!
//! WM_ERASEBKGND, WM_PAINT, WM_CLOSE, WM_DISPLAYCHANGE
//!
//! NOTE: WM_NCCREATE / WM_NCDESTROY（GWLP_USERDATA への Entity 格納・破棄時 despawn）は
//! 旧 `ecs_wndproc` 専用だったため撤去した（task 4.5）。新経路ではウィンドウ生成・破棄を
//! ライブラリ／`WindowRegistry` の drop 駆動が所管する。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::debug;
use windows::Win32::Foundation::*;

use crate::ecs::window::transition_diag::{self, MSG_DISPLAYCHANGE, MsgRecord};
use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// 「対象 entity が既に破棄済み（despawn 済み）ゆえ正常終了系として打ち切った」ことを
/// 終了時ログから grep で引くための接頭辞（Req 6.2/6.3・design D16）。
///
/// # なぜ wintf 側に同じ語を置くのか
///
/// この区別——**entity 不在＝破棄済み（正常終了系・`debug!`）／entity は実在するが規約
/// component が欠落（真の異常・`warn!`）**——はタスク 3.2 が areka の消費側 4 入口へ、
/// タスク 7.3 が areka の despawn 呼出点 2 箇所へ敷いたものと**同一**である。終了時ログの
/// 読み手は主体を問わず 1 語で引けなければならないので、**文字列は areka
/// （`areka::placement::diag::DESPAWNED_SKIP_TAG`）と同一に保つ**こと。
///
/// 定数そのものを共有しないのは design の依存方向規約（**wintf → areka の import は禁止**・
/// File Structure Plan 直下）による。areka 側の定数を参照してはならない。
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
    Some(LRESULT(1)) // 背景消去をスキップ
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
    // GPU 合成（WUC）は OS 管理: DefWindowProcW に委譲
    None
}

/// WM_CLOSE: ウィンドウクローズ要求
///
/// 対象 Entity の除去要求（despawn）として処理する。`DestroyWindow` を直叩きせず、
/// `Window` コンポーネント消失を `RemovedComponents<Window>`（reconcile_window_registry・
/// タスク 3.3）が検知し、レジストリ要素 drop 駆動で `DestroyWindow` させることで
/// ハンドル破棄を Entity ライフサイクルに一致させる（要件 1.3）。
///
/// 同期再入時の二重借用を避けるため `try_borrow_mut` を用い、既に借用中なら
/// safe-skip する（パニックさせない）。`Some(LRESULT(0))` を返して既定手続きの
/// `DestroyWindow` を抑止する（破棄は reconcile 経由・要件 2.3）。
///
/// # 陳腐化した id の打ち切り（要件 6.2/6.3・design D16・タスク 7.5）
///
/// 本ハンドラへ届く `entity` は**既に破棄済み**のことがある。`on_window_handle_remove`
/// （`window/window_handle.rs:258-282`）が entity の despawn を検知して `:279` で
/// `PostMessageW(hwnd, WM_CLOSE)` を**非同期に**投函するため、ポンプが本ハンドラを回す
/// 時点では despawn が先行し得るからである（終了経路が 2 系統あり、ユーザーが窓を閉じた
/// 場合は `WM_CLOSE` が先・自動終了では despawn が先——diagnosis-report §2.3.4 の訂正
/// ブロックが両セッションの対比で確定した）。
///
/// そのまま `despawn` を叩くと `bevy_ecs` が `Could not despawn entity` を `warn!` で出す
/// （`bevy_ecs-0.18.1` `src/world/mod.rs:1462-1469`）。これは終了処理の**正常終了系**であり、
/// 警告として残すと良性ノイズが本物の異常を埋める（要件 6.2）。ゆえにタスク 3.2／7.3 と
/// **同一の区別**をこの呼出点へも敷く: entity 不在＝正常終了系ゆえ [`DESPAWNED_SKIP_TAG`]
/// の `debug!` で打ち切る。
///
/// **`WM_CLOSE` の投函契約と窓破棄の正準シグナル（`world.despawn(entity)`・
/// `clickthrough/controller.rs:90`）は変更していない**（D16 帰結⑶）——変えたのは受け手が
/// 陳腐化 id をどう報告するかだけであり、生存 entity に対する挙動は従来と同一である。
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
        } else {
            w.world_mut().despawn(entity);
        }
    }
    Some(LRESULT(0))
}

/// WM_DISPLAYCHANGE: ディスプレイ構成変更通知
///
/// Appリソースのmark_display_changeを呼び出す
///
/// # 遷移観測（要件 2.1）
///
/// 受理を `msg` レコードとして残す。モニタ表の更新は**次 tick の `Update`** で
/// `detect_display_change_system` が行うため、受理の刻印（直近 tick の番号）と
/// `monitor` レコードの刻印は 1 フレームずれ得る——1 本の時系列に並べたときに
/// 「受理はいつで、表の更新はどのフレームか」が読めることが本記録の目的である。
///
/// 戻り値（`None`＝`DefWindowProcW` へ委譲）と `App` への標識は変えない。
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
    None // DefWindowProcWに委譲
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
    /// `Some(LRESULT(0))`（既定破棄抑止）を返す（要件 1.3 / 2.3）。
    /// ヘッドレス: 実 HWND / メッセージループ不要。
    #[test]
    fn wm_close_despawns_target_entity() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = spawn_window_entity(&world);

        // 事前条件: Entity は生存している。
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

        // 既定 DestroyWindow 抑止のため Some(LRESULT(0))。
        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "WM_CLOSE は既定破棄抑止の Some(LRESULT(0)) を返すべき"
        );

        // 反転: 除去要求として Entity が despawn されている。
        assert!(
            world.borrow().world().get_entity(entity).is_err(),
            "WM_CLOSE 後は対象 Entity が despawn されているべき（除去要求）"
        );
    }

    /// world が既に `borrow_mut` 保持中でも WM_CLOSE はパニックせず safe-skip する
    /// （`try_borrow_mut` による同期再入の二重借用回避・要件 2.3）。
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

        // パニックせず Some(LRESULT(0)) を返す。
        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "再入時も Some(LRESULT(0)) を返す（safe-skip）"
        );

        // try_borrow_mut 失敗のため despawn されず Entity は残存。
        assert!(
            world.borrow().world().get_entity(entity).is_ok(),
            "借用中の再入では despawn されず Entity が残存するべき"
        );
    }

    /// 捕捉ハーネスに渡す directive（本モジュールの `debug!` を通す最小構成）。
    const CAPTURE_DIRECTIVES: &str = "info,wintf::ecs::window_proc=debug";

    /// 陳腐化した id への `WM_CLOSE` は正常終了系として打ち切られる（Req 6.2/6.3・D16）。
    ///
    /// # 経路
    ///
    /// `on_window_handle_remove`（`window_handle.rs:258-282`）は entity の despawn を検知して
    /// `:279` で `PostMessageW(hwnd, WM_CLOSE)` を**非同期に**投函する。終了経路が
    /// 「despawn が先・`WM_CLOSE` が後」になる場合（smoke 自動終了）、ポンプが本ハンドラを
    /// 回す時点で `entity` は**既に破棄済み**である（diagnosis-report §2.3.4 の訂正ブロック）。
    ///
    /// # なぜ「捕捉に警告が無い」を根拠にできないか（空虚性 9 例目）
    ///
    /// `bevy_ecs` は `log::warn!` で `Could not despawn entity` を出す（`bevy_ecs-0.18.1`
    /// `src/world/mod.rs:71` が `use log::warn;`・`World::despawn` は `:1462-1469` で
    /// `warn!` と `false` を**同一分岐**で返す）。この `log` レコードが tracing 側へ渡るには
    /// `LogTracer` の**大域インストール**が要る——本番は `areka` の `main.rs` が
    /// `tracing_subscriber` の `.init()` を呼ぶことで（既定 feature の `tracing-log` 経由で）
    /// これを立てており、実機ログに 3 件の `WARN` が載ったのはそのためである。対して
    /// 捕捉ハーネス `capture_under_filter` は `with_default`（スレッドローカルの差替え）
    /// であって大域ブリッジを立てないため、`log` 発の記録は**原理的に 1 件も届かない**。
    /// ゆえに「捕捉に bevy の警告が無い」は恒真＝檻として空虚である。
    ///
    /// そこで本檻は ⑴**対照アーム**で `World::despawn` の戻り値 `false`（＝警告と同一分岐に
    /// ある観測可能量）を実測し、⑵**本体アーム**で自前の打ち切り行が**在ること**を主張する。
    /// 捕捉に対する陰性主張は wintf 自身の tracing 出力にのみ適用し、その捕捉可能性は
    /// ⑶**自己証明アーム**で先に示す。
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
        // 実機の終了経路の再現: `WM_CLOSE` がポンプで回る前に entity は破棄済みになる。
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

        // D16 帰結⑶: 投函契約・窓破棄の正準シグナルは不変（既定破棄抑止のまま）。
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

        // 窓破棄の正準シグナル（`world.despawn(entity)`・clickthrough/controller.rs:90）は不変。
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

//! SetWindowPos コマンドキュー・ガード
//!
//! - `is_self_initiated`: 自アプリ由来の SetWindowPos 呼び出し判定
//! - `guarded_set_window_pos`: RAII ガード付き SetWindowPos ラッパー
//! - `SetWindowPosCommand`: SetWindowPos 遅延実行キュー
//! - `find_owner_window`: エンティティの所属ウィンドウ特定
//!
//! # 遷移観測（`transition_diag`）
//!
//! 一括 flush はゴースト窓の窓矩形が実際に動く**唯一の共通経路**（経路 B）であり、
//! DPI／拡大率遷移の時系列はここを通らないと 1 本に並ばない。よって `flush` は
//! 区間の開始・各書込・区間の終了の 3 種を専用 target へ記録する（design.md C2・要件 2.1）。
//!
//! 観測は**既定 OFF** である。`transition_diag::is_enabled()` が偽のときは行の組立も
//! `GetWindowRect` の読み戻しも `Instant` の読み取りも一切行わない——flush 区間の時刻基準
//! （`begin_flush`）を開くことすら行わない。ここは毎 tick 走る経路であり、定常状態の
//! アロケーション 0（要件 10.4）を壊さないための分岐である。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Instant;
use tracing::{debug, trace, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::components::Window;
use super::transition_diag::{
    self, EnqueueRecord, FlushRecord, FlushStage, WriteRecord, WriteStage, WriteTag,
};

// ============================================================================
// SELF_INITIATED_DEPTH - SetWindowPos ネストカウンタ (AtomicI32)
// ============================================================================

/// `guarded_set_window_pos` のネスト深度カウンタ。
///
/// 0 より大きい場合、自アプリ由来の `SetWindowPos` 呼び出し中であることを示す。
/// `SetWindowPos` → `WM_WINDOWPOSCHANGED` は同期的に発火するため、
/// Relaxed ordering で十分。
///
/// ## ライフサイクル
/// 1. `guarded_set_window_pos()` 呼び出し → カウンタ +1
/// 2. `SetWindowPos` Win32 API 呼び出し（同期的に `WM_WINDOWPOSCHANGED` が発火）
/// 3. ハンドラ内で `is_self_initiated()` を参照 → カウンタ > 0 なら echo
/// 4. `SetWindowPosGuard` の Drop でカウンタ -1（RAII 保証）
static SELF_INITIATED_DEPTH: AtomicI32 = AtomicI32::new(0);

/// テスト専用: 上の**プロセス共有**カウンタを触る／読むテストを直列化する錠。
///
/// # なぜ要るのか（要件 7.7・7.1）
///
/// [`SELF_INITIATED_DEPTH`] はスレッド局所ではなく**プロセス共有の `AtomicI32`** である。
/// `cargo test` はテストを並列に走らせるため、あるテストの [`guarded_set_window_pos`] が
/// 持ち上げた値を、別スレッドで走る無関係なテストの [`is_self_initiated`]／観測レコードの
/// `in_swp` 判定が読んでしまう。是正前の実測では `cargo test -p wintf --lib` を 60 周して
/// **11 周が赤**になり、`test_flush_empty_queue_is_noop` の `assert!(!is_self_initiated())`
/// と `msg` レコードの `in_swp=false` 検査がいずれも落ちた。
///
/// カウンタ自体の意味論（プロセス共有・`Relaxed`）は本仕様の変更対象ではない
/// （観測の増設だけが本仕様の取り分＝Requirement 3.4）。よって**テスト側を直列化**して
/// 決定論を取り戻す。
///
/// # 使い方
///
/// ⑴ カウンタを**持ち上げる**側（[`guarded_set_window_pos`]／[`flush_window_pos_commands`]
/// を呼ぶテスト）と ⑵ カウンタを**読む**側（[`is_self_initiated`]／`in_swp` を検査する
/// テスト）の**両方**が取得すること。片側だけでは直列化にならない。読む側はテスト本体の
/// 先頭で取得して最後まで持ち、持ち上げるだけの側は少なくとも当該呼出を含む区間で持つ。
///
/// 毒化は無視する（`into_inner`）——1 本のテストの失敗が、以後の全テストを
/// 「錠が毒化した」で連鎖失敗させないため。
#[cfg(test)]
pub(crate) fn lock_self_initiated_for_test() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 現在 `guarded_set_window_pos` 呼び出しスコープ内かどうかを返す。
///
/// `WM_WINDOWPOSCHANGED` ハンドラ内で echo 判定に使用する。
/// `true` の場合、自アプリの `guarded_set_window_pos()` 経由の呼び出しであり、
/// `apply_window_pos_changes` での再送信は不要。
pub fn is_self_initiated() -> bool {
    SELF_INITIATED_DEPTH.load(Ordering::Relaxed) > 0
}

/// RAII ガード: スコープ終了時にネストカウンタをデクリメントする。
///
/// `guarded_set_window_pos()` 内で使用され、正常終了・`?` early return・
/// パニック時のいずれでもカウンタが確実に復元されることを保証する。
struct SetWindowPosGuard;

impl SetWindowPosGuard {
    fn new() -> Self {
        SELF_INITIATED_DEPTH.fetch_add(1, Ordering::Relaxed);
        Self
    }
}

impl Drop for SetWindowPosGuard {
    fn drop(&mut self) {
        let prev = SELF_INITIATED_DEPTH.fetch_sub(1, Ordering::Relaxed);
        trace!(
            depth = prev - 1,
            "SELF_INITIATED_DEPTH decremented by guard"
        );
    }
}

/// `SetWindowPos` をラッパー付きで呼び出す。
///
/// RAII Drop guard により、正常終了・`?` early return・パニック時も
/// ネストカウンタが確実にデクリメントされる。
/// `SetWindowPos` → `WM_WINDOWPOSCHANGED` は同期発火のため、
/// ハンドラ内で `is_self_initiated()` を参照して echo を判定できる。
///
/// # Safety
/// `SetWindowPos` Win32 API の unsafe 呼び出しを内包する。
///
/// # Arguments
/// * `hwnd` - 対象ウィンドウハンドル
/// * `hwnd_insert_after` - Z-order 挿入位置（`None` で変更なし）
/// * `x`, `y` - ウィンドウ左上座標
/// * `cx`, `cy` - ウィンドウ幅・高さ
/// * `flags` - `SET_WINDOW_POS_FLAGS`
pub unsafe fn guarded_set_window_pos(
    hwnd: HWND,
    hwnd_insert_after: Option<HWND>,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    flags: SET_WINDOW_POS_FLAGS,
) -> windows::core::Result<()> {
    let _guard = SetWindowPosGuard::new(); // Drop でカウンタ -1

    // Req 1.3: 実際の窓位置書込を行う共通経路の実施ログ。診断手順書が有効化する水準
    // （`wintf::ecs::window=debug`）で「どの窓へどの座標を書いたか」が必ず残るよう、
    // 旧 `trace!` から是正した（提案位置の実施可否と同じ水準・2026-07-18 偽陰性の是正）。
    debug!(
        hwnd = format!("0x{:X}", hwnd.0 as usize),
        x = x, y = y, cx = cx, cy = cy,
        flags = ?flags,
        "[guarded_set_window_pos] Calling SetWindowPos"
    );

    unsafe { SetWindowPos(hwnd, hwnd_insert_after, x, y, cx, cy, flags) }?;
    Ok(())
}

// ============================================================================
// SetWindowPosCommand - SetWindowPos 遅延実行キュー
// ============================================================================

/// SetWindowPosコマンド
///
/// `apply_window_pos_changes`システムから直接`SetWindowPos`を呼び出さず、
/// キューに追加して`tick`後に遅延実行することで、World借用競合を防止する。
#[derive(Debug, Clone)]
pub struct SetWindowPosCommand {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub flags: SET_WINDOW_POS_FLAGS,
    pub hwnd_insert_after: Option<HWND>,
    /// 要求元の語彙タグ（**観測専用**・design.md D3）。
    ///
    /// 「どの経路が・どのキャラの・どの種別の窓へ書いたか」を書込レコードの 1 行に載せる
    /// ためだけに運ぶ値であり、`SetWindowPos` の引数にも適用順にも一切影響しない。
    /// 既定は [`WriteTag::UNTAGGED`]（3 フィールドとも番兵）——タグを付け忘れた経路が
    /// 行の上で見分けられる。
    pub tag: WriteTag,
}

thread_local! {
    /// SetWindowPosコマンドキュー
    static WINDOW_POS_COMMANDS: RefCell<Vec<SetWindowPosCommand>> = const { RefCell::new(Vec::new()) };
}

/// 書込後の物理矩形を読み戻す（**観測が有効なときだけ**呼ぶ）。
///
/// 読み戻せなければ `None`——書込レコードの `ax`／`ay`／`aw`／`ah` は 4 つとも番兵になる
/// （フィールドごと落とすと「記録が出ていない」と「値が無い」の区別が付かない）。
///
/// クレート内へ開いてあるのは、窓書込がもう 1 箇所——メッセージ受理時の同期書込
/// （`window_proc/window_pos.rs` の `WM_DPICHANGED`・`stage=sync`）——にもあるためである。
/// 2 箇所に同じ読み戻しを持つと、片方だけが失敗時の扱い（番兵）を変えたときに静かに食い違う。
///
/// # Safety
/// `GetWindowRect` へ渡すのは呼び出し側が保持する `HWND` と、本関数がスタック上に確保した
/// `RECT` への排他参照だけであり、いずれも呼出のあいだ生存する。無効なハンドルに対しては
/// API が安全に失敗を返す（不正なポインタ参照は起きない）ため、偽ハンドルでも健全である。
pub(crate) fn read_back_window_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
        Some(rect)
    } else {
        None
    }
}

impl SetWindowPosCommand {
    /// 新しいSetWindowPosCommandを作成
    ///
    /// 要求語彙タグは付かない（[`WriteTag::UNTAGGED`]）。付けるには [`with_tag`](Self::with_tag)
    /// を続ける——**本関数の引数はタグ導入前と同一**であり、既存の呼び出し側は変わらない。
    pub fn new(
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        flags: SET_WINDOW_POS_FLAGS,
        hwnd_insert_after: Option<HWND>,
    ) -> Self {
        Self {
            hwnd,
            x,
            y,
            width,
            height,
            flags,
            hwnd_insert_after,
            tag: WriteTag::UNTAGGED,
        }
    }

    /// 要求語彙タグを載せる（観測専用・指令の中身は変わらない）。
    pub fn with_tag(mut self, tag: WriteTag) -> Self {
        self.tag = tag;
        self
    }

    /// コマンドをキューに追加
    ///
    /// 積み上げは指令を**そのまま**押し込む（先着の枠へ畳まない）。よって観測レコードの
    /// `merged_into_seq` は常に番兵＝「合流しなかった」になる。
    pub fn enqueue(cmd: SetWindowPosCommand) {
        trace!(
            hwnd = ?cmd.hwnd,
            x = cmd.x,
            y = cmd.y,
            width = cmd.width,
            height = cmd.height,
            "SetWindowPosCommand::enqueue"
        );
        if transition_diag::is_enabled() {
            transition_diag::emit_line(&transition_diag::enqueue_line(&EnqueueRecord {
                stamp: transition_diag::stamp(),
                hwnd: cmd.hwnd,
                tag: cmd.tag,
                merged_into_seq: None,
            }));
        }
        WINDOW_POS_COMMANDS.with(|cell| {
            cell.borrow_mut().push(cmd);
        });
    }

    /// キュー内の全コマンドを実行し、キューをクリア
    ///
    /// World借用解放後に呼び出すこと。
    /// 内部で `guarded_set_window_pos()` を使用し、TLS フラグによるフィードバック防止を適用する。
    ///
    /// # 観測
    ///
    /// 区間の開始（`flush stage=begin`）・各指令の書込（`write stage=flush`）・区間の終了
    /// （`flush stage=end`）を発行する。観測が無効なら計時も読み戻しも行わず、行も組まない
    /// ——時刻基準を開く [`transition_diag::begin_flush`] も**観測が有効なときだけ**呼ぶ。
    /// 失敗時の `warn!` は観測の有無によらず従来どおり出る。
    ///
    /// # 時刻基準は入れ子になる
    ///
    /// 本関数は自分の内側でもう一度呼ばれ得る。`guarded_set_window_pos` の中で
    /// `WM_WINDOWPOSCHANGED` が**同期送達**され（[`guarded_set_window_pos`] の doc・
    /// `world/vsync.rs` の再入防止ガードの説明）、その処理の手順③が
    /// [`flush_window_pos_commands`] を無条件に呼ぶためである
    /// （`window_proc/window_pos.rs` の `WM_WINDOWPOSCHANGED` ハンドラ）。`vsync.rs` の
    /// `IS_TICK_FLUSH_IN_PROGRESS` はこの直接呼出を塞がない（塞ぐのは再入する tick の側）。
    ///
    /// よって区間は「開くのは 1 箇所」ではなく「**内側は外側の起点を復元する**」形で守る
    /// （[`transition_diag::FlushEpoch`]）。これが成り立たないと、2 本目以降の書込の
    /// あいだ `since_flush_us` が `None` になり、「`WM_DPICHANGED` 等が `SetWindowPos` の
    /// 内側で同期受理された」というメッセージ側の証跡が消える。
    ///
    /// 内側の flush は観測が有効なときしか区間を開かないが、有効／無効は同一スレッドの
    /// subscriber が決めるため外側と内側で食い違わない。仮に食い違っても、開かなければ
    /// 外側の起点はそのまま残り、開けば復元されるので、どちらでも外側は壊れない。
    pub fn flush() {
        // 前置ガード。偽なら時刻基準も開かず、以降の観測分岐はすべて素通りする
        // ——確保も時刻読みも一切起きない（要件 10.4）。
        let observe = transition_diag::is_enabled();
        let _flush_epoch = observe.then(transition_diag::begin_flush);

        WINDOW_POS_COMMANDS.with(|cell| {
            let commands: Vec<_> = cell.borrow_mut().drain(..).collect();
            if commands.is_empty() {
                return;
            }
            let count = commands.len();
            trace!(count = count, "SetWindowPosCommand::flush processing");

            let flush_started = observe.then(Instant::now);
            if observe {
                transition_diag::emit_line(&transition_diag::flush_line(&FlushRecord {
                    stamp: transition_diag::stamp(),
                    stage: FlushStage::Begin,
                    count,
                    since_tick_us: transition_diag::since_tick_start_us(),
                    total_us: None,
                }));
            }

            for (index, cmd) in commands.into_iter().enumerate() {
                let call_started = observe.then(Instant::now);
                let result = unsafe {
                    guarded_set_window_pos(
                        cmd.hwnd,
                        cmd.hwnd_insert_after,
                        cmd.x,
                        cmd.y,
                        cmd.width,
                        cmd.height,
                        cmd.flags,
                    )
                };
                let ok = result.is_ok();

                if observe {
                    let call_us = call_started.map_or(0, transition_diag::elapsed_us);
                    transition_diag::emit_line(&transition_diag::write_line(&WriteRecord {
                        stamp: transition_diag::stamp(),
                        stage: WriteStage::Flush,
                        seq: u32::try_from(index).unwrap_or(u32::MAX),
                        hwnd: cmd.hwnd,
                        tag: cmd.tag,
                        x: cmd.x,
                        y: cmd.y,
                        cx: cmd.width,
                        cy: cmd.height,
                        flags: cmd.flags.0,
                        after: read_back_window_rect(cmd.hwnd),
                        call_us,
                        ok,
                    }));
                }

                if let Err(e) = result {
                    warn!(
                        hwnd = ?cmd.hwnd,
                        error = ?e,
                        "SetWindowPos failed"
                    );
                } else {
                    trace!(hwnd = ?cmd.hwnd, "SetWindowPos succeeded");
                }
            }

            if observe {
                transition_diag::emit_line(&transition_diag::flush_line(&FlushRecord {
                    stamp: transition_diag::stamp(),
                    stage: FlushStage::End,
                    count,
                    since_tick_us: transition_diag::since_tick_start_us(),
                    total_us: Some(flush_started.map_or(0, transition_diag::elapsed_us)),
                }));
            }
        });
    }
}

/// SetWindowPosコマンドキューをフラッシュする便利関数
pub fn flush_window_pos_commands() {
    SetWindowPosCommand::flush();
}

/// キューの中身を**実行せずに**取り出す（テスト専用のシーム・design.md D11）。
///
/// 決定論テストは「どの窓へ何回・どの順で書こうとしたか」をキューの中身で検証し、実際の
/// `SetWindowPos` は呼ばない（実窓も実 DPI も要らない形にするため）。
///
/// # 本番からは呼ばないこと
///
/// 取り出した指令は**二度と実行されない**——本番経路がこれを呼ぶと窓書込が黙って消える。
/// 実行を伴う唯一の取り出し口は [`SetWindowPosCommand::flush`] である。
///
/// `pub` なのは areka 側の決定論テストが**別クレート**からキューを検査するためであり
/// （クレート境界ゆえ `#[cfg(test)]` では届かない）、公開 API として提供する意図はない。
/// あわせて `transition_diag::reset_for_test` を呼べば、テストは残留の無い状態から始まる
/// （要件 7.7）。
#[doc(hidden)]
pub fn drain_window_pos_commands() -> Vec<SetWindowPosCommand> {
    WINDOW_POS_COMMANDS.with(|cell| cell.borrow_mut().drain(..).collect())
}

// ============================================================================
// find_owner_window - エンティティの所属ウィンドウ特定
// ============================================================================

/// エンティティが所属する Window エンティティを返す。
///
/// ChildOf チェーンを辿り、Window コンポーネントを持つ最初の祖先で停止する。
/// エンティティ自身が Window の場合は `Some(entity)` を返す。
/// ChildOf を持たないエンティティ（LayoutRoot 等）に到達した場合は `None` を返す。
///
/// # Arguments
/// * `world` - ECS World 参照（読み取り専用）
/// * `entity` - 所属ウィンドウを検索するエンティティ
///
/// # Returns
/// 所属する Window エンティティ。Window が見つからない場合は `None`。
pub fn find_owner_window(world: &World, entity: Entity) -> Option<Entity> {
    // エンティティ自身が Window を持つ場合は自身を返す
    if world.get::<Window>(entity).is_some() {
        return Some(entity);
    }

    // ChildOf チェーンを辿る
    let mut current = entity;
    while let Some(child_of) = world.get::<ChildOf>(current) {
        let parent = child_of.parent();
        if world.get::<Window>(parent).is_some() {
            return Some(parent);
        }
        current = parent;
    }

    None
}

#[cfg(test)]
#[path = "command_transition_tests.rs"]
mod command_transition_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_window_pos_command_new_stores_all_fields() {
        let hwnd = HWND(0x1000 as *mut _);
        let after = HWND(0x2000 as *mut _);
        let cmd = SetWindowPosCommand::new(
            hwnd,
            10,
            20,
            300,
            400,
            SWP_NOACTIVATE,
            Some(after),
        );
        assert_eq!(cmd.hwnd, hwnd);
        assert_eq!(cmd.x, 10);
        assert_eq!(cmd.y, 20);
        assert_eq!(cmd.width, 300);
        assert_eq!(cmd.height, 400);
        assert_eq!(cmd.flags, SWP_NOACTIVATE);
        assert_eq!(cmd.hwnd_insert_after, Some(after));
    }

    #[test]
    fn test_set_window_pos_command_new_allows_none_insert_after() {
        let cmd = SetWindowPosCommand::new(
            HWND(0x1 as *mut _),
            0,
            0,
            0,
            0,
            SET_WINDOW_POS_FLAGS(0),
            None,
        );
        assert_eq!(cmd.hwnd_insert_after, None);
    }

    #[test]
    fn test_is_self_initiated_false_at_rest() {
        // guarded_set_window_pos スコープ外（ネストカウンタ 0）では false
        //
        // 注: SELF_INITIATED_DEPTH は**プロセス共有**の AtomicI32 である。旧注記は
        // 「テストスレッドで guarded_set_window_pos を呼ばない限り 0 のまま」としていたが、
        // これは誤りだった——カウンタはスレッドごとではないので、並列に走る**別テスト**の
        // 書込経路が持ち上げた値がそのまま見える（実測: 是正前 60 周中 11 周が赤）。
        // ゆえに読む側も `lock_self_initiated_for_test` を取って直列化する（要件 7.7）。
        let _serialized = lock_self_initiated_for_test();
        assert!(!is_self_initiated());
    }

    #[test]
    fn test_flush_empty_queue_is_noop() {
        // 空キューの flush は early-return で SetWindowPos を呼ばずパニックしない。
        // （このテスト内では enqueue していないため WINDOW_POS_COMMANDS は空。
        //  thread_local かつ同一テストスレッドのため他テストの enqueue 残留はない）
        //
        // 末尾の `is_self_initiated()` はプロセス共有カウンタを読むため直列化が要る
        // （`lock_self_initiated_for_test` の doc）。
        let _serialized = lock_self_initiated_for_test();
        SetWindowPosCommand::flush();
        // 便利関数経由でも同様に no-op
        flush_window_pos_commands();
        assert!(!is_self_initiated());
    }
}

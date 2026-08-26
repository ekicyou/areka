//! SetWindowPos コマンドキュー・ガード
//!
//! - `is_self_initiated`: 自アプリ由来の SetWindowPos 呼び出し判定
//! - `guarded_set_window_pos`: RAII ガード付き SetWindowPos ラッパー
//! - `SetWindowPosCommand`: SetWindowPos 遅延実行キュー
//! - `coalesce_geometry`: 同一 tick・同一窓のジオメトリ指令を 1 本へ畳む純関数
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
//!
//! # コードが強制していない前提: 窓手続き側は Z 指令を積まない
//!
//! `WM_WINDOWPOSCHANGED` のハンドラは、その場で再び一括 flush を呼ぶ（`window_proc/window_pos.rs:290`）。
//! この再入 flush が Z 指令の適用順と噛み合わないという事故が起きていないのは、**窓手続き側の
//! 経路が Z 指令を積まない**からにすぎない。Z 指令は合流の対象外であり同一窓の仕切りとして
//! 働くので、再入した flush が Z 指令を含むキューを掴めば、順序の意味が外側の区間と分かれる。
//!
//! **この前提をコードは一切強制していない**（型でも表明でもテストでも縛っていない）。よって
//! 一括 flush の**駆動（誰がいつ呼ぶか）・順序（どの並びで適用するか）に手を入れるときは、
//! この前提が依然として成立しているかを確かめる義務がある**（要件 3.5）。確かめずに駆動を
//! 変えると、Z 順の破れは実機でしか出ない形で静かに入り込む。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use std::cell::{Cell, RefCell};
use std::time::Instant;
use tracing::{debug, trace, warn};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::components::Window;
use super::transition_diag::{
    self, EnqueueRecord, FlushRecord, FlushStage, WriteRecord, WriteStage, WriteTag,
};

// ============================================================================
// SELF_INITIATED_DEPTH - SetWindowPos ネストカウンタ (スレッド局所)
// ============================================================================

thread_local! {
    /// `guarded_set_window_pos` のネスト深度カウンタ（**スレッドごとに独立**）。
    ///
    /// 0 より大きい場合、そのスレッドが自アプリ由来の `SetWindowPos` 呼び出しの内側に
    /// いることを示す。
    ///
    /// ## なぜスレッド局所でよいのか（意味論は不変・設計 C20）
    ///
    /// このカウンタが答える問いは「**いま自分が**書いた結果の通知を受けているか」である。
    /// `SetWindowPos` も `EndDeferWindowPos` も、`WM_WINDOWPOSCHANGING`／
    /// `WM_WINDOWPOSCHANGED` を**呼び出したスレッドの上で同期送達**する——持ち上げ・送達・
    /// 判定・解放がすべて 1 本のスレッドの内側で閉じる。つまり「自発の内側か」は元より
    /// スレッドごとの性質であり、プロセス全体で 1 個の値を共有する理由が無かった。
    /// 共有していた間は、別スレッドの書込が持ち上げた値を無関係な読み手が拾う経路が
    /// 開いていただけである（テスト間の状態汚染の出所＝要件 6.6）。
    ///
    /// ## ライフサイクル
    /// 1. `guarded_set_window_pos()` 呼び出し → 当該スレッドのカウンタ +1
    /// 2. `SetWindowPos` Win32 API 呼び出し（同期的に `WM_WINDOWPOSCHANGED` が発火）
    /// 3. ハンドラ内で `is_self_initiated()` を参照 → カウンタ > 0 なら echo
    /// 4. `SetWindowPosGuard` の Drop でカウンタ -1（RAII 保証）
    static SELF_INITIATED_DEPTH: Cell<i32> = const { Cell::new(0) };
}

/// 現在 `guarded_set_window_pos` 呼び出しスコープ内かどうかを返す。
///
/// `WM_WINDOWPOSCHANGED` ハンドラ内で echo 判定に使用する。
/// `true` の場合、自アプリの `guarded_set_window_pos()` 経由の呼び出しであり、
/// `apply_window_pos_changes` での再送信は不要。
///
/// 見るのは**このスレッドの**深度だけである（同期送達なので、判定はガードを持ち上げた
/// スレッドの上でしか起きない）。
pub fn is_self_initiated() -> bool {
    SELF_INITIATED_DEPTH.with(|depth| depth.get()) > 0
}

/// RAII ガード: スコープ終了時に**そのスレッドの**ネストカウンタをデクリメントする。
///
/// `guarded_set_window_pos()` 内で使用され、正常終了・`?` early return・
/// パニック時のいずれでもカウンタが確実に復元されることを保証する。
///
/// 持ち上げと解放は必ず同一スレッド上で対になる（ガードは `Send` でないスコープ値として
/// 使われ、`SetWindowPos`／`EndDeferWindowPos` の同期送達を挟むだけである）。
struct SetWindowPosGuard;

impl SetWindowPosGuard {
    fn new() -> Self {
        SELF_INITIATED_DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}

impl Drop for SetWindowPosGuard {
    fn drop(&mut self) {
        let depth = SELF_INITIATED_DEPTH.with(|cell| {
            let next = cell.get() - 1;
            cell.set(next);
            next
        });
        trace!(depth, "SELF_INITIATED_DEPTH decremented by guard");
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

    log_window_write(
        &WindowPosArgs {
            hwnd,
            hwnd_insert_after,
            x,
            y,
            cx,
            cy,
            flags,
        },
        VIA_SET_WINDOW_POS,
    );

    unsafe { SetWindowPos(hwnd, hwnd_insert_after, x, y, cx, cy, flags) }?;
    Ok(())
}

/// 窓書込 1 回の実施ログ（**診断手順の固定トークン**）。
///
/// Req 1.3（`dpi-window-vanish`）: 実際の窓位置書込を行う共通経路の実施ログであり、
/// 診断手順書が有効化する水準（`wintf::ecs::window=debug`）で「どの窓へどの座標を
/// 書いたか」が必ず残る。
///
/// # 字面を変えてはならない
///
/// この 1 行は `dpi-window-vanish` の診断手順書の観測点 **O9** であり、`ghost-window-zorder`
/// の維持系の檻 16 本（`zorder_pair_*_tests.rs` の `SET_WINDOW_POS_TAG`）が本数を数えて
/// 「是正の指令が何本出たか」を測っている。ゆえに**投入に使う API が変わっても字面は
/// 変えない**——実際に呼んだ API は `via=` フィールドが名乗る（実ログの字面は
/// `via="SetWindowPos"` か `via="DeferWindowPos"`。`tracing` が文字列を引用符で囲む）。
///
/// # 本数の不変条件: 1 指令につき 1 行
///
/// 数えられている以上、**実施していない書込をこの行で名乗ってはならない**。バッチ経路は
/// 積み上げ（`DeferWindowPos`）の時点では書いていない——`EndDeferWindowPos` が成って初めて
/// 窓が動く——ので、[`apply_as_batch`] は**適用が成った後に**まとめて出す。積み上げの途中で
/// 出すと、破棄されたバッチの指令が縮退で書き直されたときに同じ指令が 2 度数えられる
/// （本仕様の task 7.2 のレビューが実測で見つけた形）。対テストは
/// `command_batch_tests::the_write_log_line_is_emitted_once_per_command_on_both_paths`。
const WINDOW_WRITE_LOG_MESSAGE: &str = "[guarded_set_window_pos] Calling SetWindowPos";

/// `via=` の値: 1 本ずつの `SetWindowPos` で書いた。
const VIA_SET_WINDOW_POS: &str = "SetWindowPos";

/// `via=` の値: `DeferWindowPos` の 1 バッチへ積んだ。
const VIA_DEFER_WINDOW_POS: &str = "DeferWindowPos";

/// 窓書込 1 回の実施ログを出す（両経路で**同一の字面**）。
fn log_window_write(args: &WindowPosArgs, via: &'static str) {
    debug!(
        hwnd = format!("0x{:X}", args.hwnd.0 as usize),
        x = args.x, y = args.y, cx = args.cx, cy = args.cy,
        flags = ?args.flags,
        via = via,
        "{WINDOW_WRITE_LOG_MESSAGE}"
    );
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

// ============================================================================
// 一括投入（`Begin/Defer/EndDeferWindowPos` の 1 バッチ）
// ============================================================================

/// 窓書込 1 回ぶんの引数（`DeferWindowPos`／`SetWindowPos` へそのまま渡る値）。
///
/// バッチ化（設計 C8 の候補 B-2b）が変えるのは**投入の仕方**だけであり、渡す引数と
/// その並びは 1 つも変えない（要件 10.3）。この型はその「変えない引数」を 1 箇所で
/// 組むためだけに在る——バッチ経路と縮退経路が別々に組み立てると、片方だけが
/// `hwnd_insert_after` や flags を落としたときに静かに食い違う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowPosArgs {
    /// 対象ウィンドウ。
    pub hwnd: HWND,
    /// Z 順の挿入位置（`None` で変更なし）。**Z 専用指令はここに値を持つ**。
    pub hwnd_insert_after: Option<HWND>,
    /// 位置 X。
    pub x: i32,
    /// 位置 Y。
    pub y: i32,
    /// 幅。
    pub cx: i32,
    /// 高さ。
    pub cy: i32,
    /// `SetWindowPos` フラグ（per-window に持てるので Z 専用も同居できる）。
    pub flags: SET_WINDOW_POS_FLAGS,
}

/// 指令 1 件を書込引数へ写す（純関数）。
///
/// **7 項目をそのまま運ぶだけ**である。観測専用の `tag` は引数に影響しない（design.md D3）。
pub(crate) fn window_pos_args(cmd: &SetWindowPosCommand) -> WindowPosArgs {
    WindowPosArgs {
        hwnd: cmd.hwnd,
        hwnd_insert_after: cmd.hwnd_insert_after,
        x: cmd.x,
        y: cmd.y,
        cx: cmd.width,
        cy: cmd.height,
        flags: cmd.flags,
    }
}

/// バッチ投入が使えなかった理由（縮退の記録に載せる語）。
///
/// 3 つとも**無音にしない**——`warn!` を伴い、当該 flush の書込レコードは
/// `in_batch=false` になる（要件 3.4 の「失敗経路にログを持たせる」）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BatchDegrade {
    /// `BeginDeferWindowPos` がバッチを確保できなかった（1 件も適用されていない）。
    BeginFailed,
    /// `DeferWindowPos` が失敗した。**それまでに積んだ指令はバッチごと失われる**ため、
    /// 部分適用は起きていない（Win32 の仕様: 失敗したハンドルへ `End` を呼んではならない）。
    DeferFailed {
        /// 失敗した指令のキュー内位置。
        seq: u32,
    },
    /// `EndDeferWindowPos` が失敗した。**部分適用の可能性がある**ので、縮退では
    /// 全件を 1 本ずつ書き直す（同一引数の再適用は最終状態を変えない）。
    EndFailed,
}

impl BatchDegrade {
    /// 記録に載せる理由語。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::BeginFailed => "BeginDeferWindowPos-failed",
            Self::DeferFailed { .. } => "DeferWindowPos-failed",
            Self::EndFailed => "EndDeferWindowPos-failed",
        }
    }
}

#[cfg(test)]
thread_local! {
    /// テスト専用: バッチ確保を失敗させる札。
    ///
    /// `BeginDeferWindowPos` は実運用では滅多に失敗しないので、縮退経路の一方を
    /// 決定論で踏むにはここを倒すほかない（`DeferWindowPos` の失敗は偽ハンドルで
    /// 自然に踏める）。
    static FORCE_BATCH_BEGIN_FAILURE: RefCell<bool> = const { RefCell::new(false) };
}

/// テスト専用: バッチ確保を失敗させたまま `body` を走らせる。
#[cfg(test)]
pub(crate) fn with_forced_batch_begin_failure<R>(body: impl FnOnce() -> R) -> R {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            FORCE_BATCH_BEGIN_FAILURE.with(|cell| *cell.borrow_mut() = false);
        }
    }
    FORCE_BATCH_BEGIN_FAILURE.with(|cell| *cell.borrow_mut() = true);
    let _restore = Restore;
    body()
}

/// バッチを確保する（テストは失敗側へ倒せる）。
fn begin_batch(count: usize) -> Option<HDWP> {
    #[cfg(test)]
    if FORCE_BATCH_BEGIN_FAILURE.with(|cell| *cell.borrow()) {
        return None;
    }
    // SAFETY: Win32 境界。確保するだけで所有権も生存参照も持ち出さない。失敗は
    // `Result` で返り、無効ハンドルは windows crate 側で `Err` へ畳まれている。
    unsafe { BeginDeferWindowPos(i32::try_from(count).unwrap_or(i32::MAX)) }.ok()
}

/// 指令列を **1 バッチ**で投入して一度に適用する（設計 C8 の候補 B-2b）。
///
/// 成功したら `Ok(())`、投入できなかったら `Err(理由)` を返す。並びはキューの順のまま、
/// 引数は [`window_pos_args`] が組んだものそのままである（要件 10.3）。
///
/// # 自発書込の判定（要件 10.2）
///
/// `EndDeferWindowPos` は全窓ぶんの `WM_WINDOWPOSCHANGING`／`WM_WINDOWPOSCHANGED` を
/// **同期送達**する。よって [`SetWindowPosGuard`] をバッチ全体に掛ける——掛けないと
/// `is_self_initiated()` が偽になり、`dpi-window-vanish` が確立した位置権威（自発の書込を
/// 外部由来と取り違えない）が壊れる。
///
/// # 所要の測り方
///
/// `call_us` には**投入 1 件ぶん**（`DeferWindowPos`）の所要を書く。実際に窓が動く時間は
/// 1 バッチぶんまとめて `flush stage=end` の `total_us` に入る。
///
/// # 実施ログ（O9）は適用の後
///
/// 窓書込の実施ログ（[`WINDOW_WRITE_LOG_MESSAGE`]）は `EndDeferWindowPos` が成ってから
/// 指令の順にまとめて出す。積み上げの途中で出すと、`Defer` や `End` が失敗して
/// [`apply_sequentially`] へ縮退したときに**同じ指令が 2 度**名乗ることになり、この行の
/// 本数を数えている 16 本の檻と診断手順 O9 が静かに狂う。
///
/// # Safety
/// Win32 境界。渡すのは呼び出し側が保持する `HWND` と整数だけであり、無効なハンドルに
/// 対しては API が安全に失敗を返す。
unsafe fn apply_as_batch(
    commands: &[SetWindowPosCommand],
    observe: bool,
    call_us: &mut [u64],
) -> Result<(), BatchDegrade> {
    let mut batch = begin_batch(commands.len()).ok_or(BatchDegrade::BeginFailed)?;

    // ガードはバッチ全体に掛ける（`End` の同期送達まで自発の内側であること）。
    let _guard = SetWindowPosGuard::new();

    for (index, cmd) in commands.iter().enumerate() {
        let args = window_pos_args(cmd);
        let started = observe.then(Instant::now);
        // SAFETY: Win32 境界。`batch` は直前の確保／積み上げが返した有効なハンドルであり、
        // 失敗時は下で `Err` へ落として二度と使わない。
        let next = unsafe {
            DeferWindowPos(
                batch,
                args.hwnd,
                args.hwnd_insert_after,
                args.x,
                args.y,
                args.cx,
                args.cy,
                args.flags,
            )
        };
        if observe && let Some(slot) = call_us.get_mut(index) {
            *slot = started.map_or(0, transition_diag::elapsed_us);
        }
        match next {
            Ok(handle) => batch = handle,
            Err(error) => {
                // 失敗したハンドルへ `End` を呼んではならない——積んだぶんは失われる。
                warn!(
                    seq = index,
                    hwnd = ?args.hwnd,
                    error = ?error,
                    "DeferWindowPos failed; the whole batch is discarded"
                );
                return Err(BatchDegrade::DeferFailed {
                    seq: u32::try_from(index).unwrap_or(u32::MAX),
                });
            }
        }
    }

    // SAFETY: Win32 境界。`batch` はここまでの積み上げが返した有効なハンドルである。
    if let Err(error) = unsafe { EndDeferWindowPos(batch) } {
        warn!(error = ?error, "EndDeferWindowPos failed");
        return Err(BatchDegrade::EndFailed);
    }

    // 実施ログ（O9）は**適用が成った後にだけ**出す。積み上げの途中で出すと、破棄された
    // バッチの指令が「書いた」と名乗り、縮退で書き直した本番の行と二重に数えられる
    // ——`WINDOW_WRITE_LOG_MESSAGE` の doc が言うとおり、この行の**本数**は既存 16 本の
    // 檻と診断手順 O9 が数えている量である。
    for cmd in commands {
        log_window_write(&window_pos_args(cmd), VIA_DEFER_WINDOW_POS);
    }
    Ok(())
}

/// 指令列を**キューの順のまま 1 本ずつ**書き込む（縮退経路・是正前と同じ形）。
///
/// # Safety
/// Win32 境界（[`guarded_set_window_pos`] に同じ）。
unsafe fn apply_sequentially(
    commands: &[SetWindowPosCommand],
    observe: bool,
    call_us: &mut [u64],
    ok: &mut [bool],
) {
    for (index, cmd) in commands.iter().enumerate() {
        let args = window_pos_args(cmd);
        let started = observe.then(Instant::now);
        // SAFETY: Win32 境界。ラッパーが自発カウンタを持ち上げたうえで呼ぶ。
        let result = unsafe {
            guarded_set_window_pos(
                args.hwnd,
                args.hwnd_insert_after,
                args.x,
                args.y,
                args.cx,
                args.cy,
                args.flags,
            )
        };
        if observe && let Some(slot) = call_us.get_mut(index) {
            *slot = started.map_or(0, transition_diag::elapsed_us);
        }
        if let Some(slot) = ok.get_mut(index) {
            *slot = result.is_ok();
        }
        if let Err(e) = result {
            warn!(
                hwnd = ?args.hwnd,
                error = ?e,
                "SetWindowPos failed"
            );
        } else {
            trace!(hwnd = ?args.hwnd, "SetWindowPos succeeded");
        }
    }
}

// ============================================================================
// 合流（同一 tick・同一窓のジオメトリ指令を 1 本へ畳む）
// ============================================================================

/// 合流に関わり得るフラグの全体（この 4 つ以外を 1 つでも持つ指令は畳まない）。
///
/// `SWP_SHOWWINDOW`／`SWP_HIDEWINDOW`／`SWP_FRAMECHANGED` などは**表示状態や非クライアント
/// 領域を動かす**副作用を持ち、位置・寸の「後勝ち」では合成できない。ゆえに許可集合を
/// 列挙し、**知らないフラグは畳まない**側へ倒す（新しいフラグが増えても既定で安全）。
const COALESCIBLE_FLAGS: u32 = SWP_NOZORDER.0 | SWP_NOACTIVATE.0 | SWP_NOMOVE.0 | SWP_NOSIZE.0;

/// 合流に**必要**なフラグ。
///
/// `SWP_NOZORDER` を欠く指令は Z 順を動かす＝`ghost-window-zorder` の維持系が積む Z 専用
/// 指令がこれに当たる（要件 10.3）。`SWP_NOACTIVATE` を欠く指令は活性化という副作用を持つ。
/// どちらも位置・寸の合成では保存できないので合流の対象にも先にもしない。
const REQUIRED_FOR_COALESCE: u32 = SWP_NOZORDER.0 | SWP_NOACTIVATE.0;

/// 指令が合流に関与してよいか（対象にも先にもなり得るか）。
///
/// 3 条件すべてを満たすときだけ真: ⑴ 挿入位置を持たない、⑵ [`REQUIRED_FOR_COALESCE`] を
/// すべて持つ、⑶ [`COALESCIBLE_FLAGS`] 以外のフラグを 1 つも持たない。
fn is_coalescible(cmd: &SetWindowPosCommand) -> bool {
    cmd.hwnd_insert_after.is_none()
        && (cmd.flags.0 & REQUIRED_FOR_COALESCE) == REQUIRED_FOR_COALESCE
        && (cmd.flags.0 & !COALESCIBLE_FLAGS) == 0
}

/// 畳み先を探す——同一 hwnd の**先着**の合流可能な指令。
///
/// 同一 hwnd に合流できない指令（Z 専用・表示状態を変えるもの）があれば、そこで探索を
/// **仕切り直す**。畳むと当該窓のジオメトリ書込がその指令より前へ移り、Z 指令との相対順が
/// 変わってしまうためである（要件 10.3 が禁じるのは順序と結果の変化であって、最終状態の
/// 一致だけではない）。別の窓の指令は仕切りにならない——`SWP_NOZORDER` 付きの書込は
/// 他窓の状態に触れないからである。
fn find_merge_target(queue: &[SetWindowPosCommand], hwnd: HWND) -> Option<usize> {
    let mut first = None;
    for (index, existing) in queue.iter().enumerate() {
        if existing.hwnd != hwnd {
            continue;
        }
        if is_coalescible(existing) {
            if first.is_none() {
                first = Some(index);
            }
        } else {
            first = None;
        }
    }
    first
}

/// 先着の枠へ後着を畳み込む（各項目は**後勝ち**）。
///
/// 呼び出し側は両者が [`is_coalescible`] であることを保証すること——本関数は合流後の
/// フラグを 4 ビットから組み直すので、それ以外のフラグが載っていると黙って落ちる。
fn merge_into(base: &mut SetWindowPosCommand, later: &SetWindowPosCommand) {
    let base_nomove = (base.flags.0 & SWP_NOMOVE.0) != 0;
    let base_nosize = (base.flags.0 & SWP_NOSIZE.0) != 0;
    let later_nomove = (later.flags.0 & SWP_NOMOVE.0) != 0;
    let later_nosize = (later.flags.0 & SWP_NOSIZE.0) != 0;

    // 後着が動かす項目だけを上書きする＝逐次適用の最終ジオメトリと一致する。
    if !later_nomove {
        base.x = later.x;
        base.y = later.y;
    }
    if !later_nosize {
        base.width = later.width;
        base.height = later.height;
    }

    // 「移動なし」「寸なし」の札は**双方が持つときだけ**残る（片方が動かすなら動く）。
    let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
    if base_nomove && later_nomove {
        flags |= SWP_NOMOVE;
    }
    if base_nosize && later_nosize {
        flags |= SWP_NOSIZE;
    }
    base.flags = flags;

    // タグは先着の値を保つ（`base.tag` を触らない）。合流後の 1 本が「どの経路の枠か」を
    // 先着で名乗るのは、`merged_into_seq` が指す先と同じ枠を指すためである。
}

/// 積み上げ 1 件を、可能なら先着の枠へ畳む（純関数・design.md C2）。
///
/// 畳んだときは**先着の通し番号**を返し、畳まなかったときはキューの末尾へ押し込んで
/// `None` を返す。通し番号は一括書込の `write` レコードの `seq`（キュー内の位置）と
/// 同じ数え方である。
///
/// # 事後条件
///
/// 合流後の適用が生む最終ジオメトリは、合流しない逐次適用の最終ジオメトリと一致する
/// （各項目「後勝ち」）。Z 専用指令の相対順序と引数は不変である（要件 10.3）。
pub(crate) fn coalesce_geometry(
    queue: &mut Vec<SetWindowPosCommand>,
    cmd: SetWindowPosCommand,
) -> Option<u32> {
    let target = if is_coalescible(&cmd) {
        find_merge_target(queue, cmd.hwnd)
    } else {
        None
    };

    match target {
        Some(index) => {
            merge_into(&mut queue[index], &cmd);
            // 4G 件を超えるキューは起こり得ないが、`as` の暗黙切り捨てで別の枠を指すより
            // 飽和させて上限に張り付かせる（`flush` の `seq` と同じ扱い）。
            //
            // この腕は**到達不能**であって「どう書いても等価」ではない——`unwrap_or(0)` や
            // `as` へ替えれば別の枠を指す。変異検査で殺せないのは到達しないからであり、
            // テストの穴ではない。**`as` へ単純化しないこと**（この保険が防いでいるのは
            // まさにその静かな取り違えである）。
            Some(u32::try_from(index).unwrap_or(u32::MAX))
        }
        None => {
            queue.push(cmd);
            None
        }
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
    /// 積み上げは [`coalesce_geometry`] を通す——同一 tick・同一窓のジオメトリ指令は
    /// **先着の枠へ畳まれ**、窓ごとの書込が 1 本になる（要件 4.5）。畳まれた場合は観測
    /// レコードの `merged_into_seq` に**先着の通し番号**が載り、畳まれなければ番兵になる。
    ///
    /// Z のみの指令と表示状態を変える指令は畳まれない（要件 10.3）。判定と理由は
    /// [`coalesce_geometry`] の doc を見ること。
    pub fn enqueue(cmd: SetWindowPosCommand) {
        // 窓書込が積まれた＝次の画面更新に仕事がある（設計 C16 の `WINDOW_CMD`）。
        crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::WINDOW_CMD);
        trace!(
            hwnd = ?cmd.hwnd,
            x = cmd.x,
            y = cmd.y,
            width = cmd.width,
            height = cmd.height,
            "SetWindowPosCommand::enqueue"
        );
        // 記録は合流の**後**に組む——合流先の通し番号は畳んでみるまで判らない。
        let hwnd = cmd.hwnd;
        let tag = cmd.tag;
        let merged_into_seq =
            WINDOW_POS_COMMANDS.with(|cell| coalesce_geometry(&mut cell.borrow_mut(), cmd));
        if transition_diag::is_enabled() {
            transition_diag::emit_line(&transition_diag::enqueue_line(&EnqueueRecord {
                stamp: transition_diag::stamp(),
                hwnd,
                tag,
                merged_into_seq,
            }));
        }
    }

    /// キュー内の全コマンドを実行し、キューをクリア
    ///
    /// World借用解放後に呼び出すこと。
    ///
    /// # 投入は 1 バッチ（設計 C8 の候補 B-2b・task 7.2）
    ///
    /// 取り出した指令列は [`apply_as_batch`] が `Begin`／`Defer`／`EndDeferWindowPos` の
    /// **1 バッチ**で投入する。Z 専用指令は合流の対象外だが、`DeferWindowPos` は
    /// per-window に `hwndInsertAfter` と flags を取れるので**同じバッチに同居**でき、
    /// 適用の並びも引数もキューのままである（要件 10.3）。
    ///
    /// バッチが使えなかったときは 1 本ずつの `guarded_set_window_pos` へ**縮退**する
    /// （[`apply_sequentially`]）。縮退は `warn!` を伴い、当該区間の書込レコードは
    /// `in_batch=false` になる——静かに形が変わることはない。
    ///
    /// 自発書込カウンタはどちらの経路でも持ち上がる（バッチ側は `EndDeferWindowPos` が
    /// 同期送達するメッセージまで含めてガードの内側・要件 10.2）。
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

            // 所要と成否は指令ごとに持つ。**どちらも観測レコードにしか使わない**ので、
            // 観測が無効なら器は長さ 0 のまま＝確保が 1 度も起きない（要件 10.4）。
            // 失敗そのものは観測の有無によらず `warn!` が残す。
            let mut call_us = vec![0u64; if observe { count } else { 0 }];
            let mut ok = vec![false; if observe { count } else { 0 }];

            // SAFETY: Win32 境界。バッチ投入も縮退も、渡すのは呼び出し側が保持する
            // `HWND` と整数だけである。
            let batched = unsafe { apply_as_batch(&commands, observe, &mut call_us) };
            let in_batch = match batched {
                Ok(()) => {
                    ok.fill(true);
                    true
                }
                Err(reason) => {
                    // 縮退は無音にしない（要件 3.4・steering `logging.md`）。
                    warn!(
                        count = count,
                        reason = reason.as_str(),
                        "DeferWindowPos batch unavailable; falling back to one SetWindowPos per command"
                    );
                    // SAFETY: Win32 境界（上に同じ）。
                    unsafe { apply_sequentially(&commands, observe, &mut call_us, &mut ok) };
                    false
                }
            };

            if observe {
                for (index, cmd) in commands.iter().enumerate() {
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
                        call_us: call_us.get(index).copied().unwrap_or(0),
                        ok: ok.get(index).copied().unwrap_or(false),
                        in_batch,
                    }));
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
#[path = "command_coalesce_tests.rs"]
mod command_coalesce_tests;

#[cfg(test)]
#[path = "command_batch_tests.rs"]
mod command_batch_tests;

#[cfg(test)]
#[path = "command_threadlocal_tests.rs"]
mod command_threadlocal_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_window_pos_command_new_stores_all_fields() {
        let hwnd = HWND(0x1000 as *mut _);
        let after = HWND(0x2000 as *mut _);
        let cmd = SetWindowPosCommand::new(hwnd, 10, 20, 300, 400, SWP_NOACTIVATE, Some(after));
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
        // 注: SELF_INITIATED_DEPTH は**スレッド局所**になった（draw-load-parity task 4）ので、
        // このテストスレッドで guarded_set_window_pos を呼ばない限り 0 のままである。直列化の
        // 錠は要らない（test-cage-determinism task 7.2 で退役）。スレッド局所であることの本体は
        // `command_threadlocal_tests.rs`。
        assert!(!is_self_initiated());
    }

    #[test]
    fn test_flush_empty_queue_is_noop() {
        // 空キューの flush は early-return で SetWindowPos を呼ばずパニックしない。
        // （このテスト内では enqueue していないため WINDOW_POS_COMMANDS は空。
        //  thread_local かつ同一テストスレッドのため他テストの enqueue 残留はない）
        //
        // 末尾の `is_self_initiated()` はスレッド局所カウンタを読むので直列化は要らない
        // （錠は test-cage-determinism task 7.2 で退役）。
        SetWindowPosCommand::flush();
        // 便利関数経由でも同様に no-op
        flush_window_pos_commands();
        assert!(!is_self_initiated());
    }
}

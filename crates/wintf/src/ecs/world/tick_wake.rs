//! 起床の旗——「次の画面更新で仕事があるか」をプロセス共有のビット集合で持つ。
//!
//! # 何のためか
//!
//! 画面更新のたびに 13 本のスケジュールを全部回すと、見た目が 1 画素も変わらない回でも
//! CPU を使う。変化が生じた側が**旗を立て**、画面更新のたびに UI スレッドが**旗を読んで
//! 倒す**——この 1 往復だけで「回す／省略する」を決められるようにするのが本モジュールである。
//! 旗そのものは判断をしない。回すかどうかを決めるのは純関数 `tick_gate::should_run` で、
//! 両者をつなぐのは `EcsWorld::decide_tick` である。
//!
//! # 誰が立てるか（生産者）
//!
//! 旗は**変化を起こした側**が立てる。旗ごとの持ち主は次のとおり。
//!
//! - [`POINTER`] — ポインタ入力の投入（`pointer/buffers.rs`）とポインタ系メッセージの受理
//! - [`DRAG`] — ドラッグ中（毎画面更新で回す必要があるので、tick の末尾で自分で立て直す）
//! - [`WINDOW_CMD`] — 窓書込指令の積み上げ（`command.rs` の enqueue）
//! - [`ZORDER`] — Z 順の要求（`window/zorder_pair_maintain.rs` の `ReassertZOrder` 維持・
//!   areka 側 `emo2_boot/zorder_cue.rs` のタグ入口）
//!   この行は**ファイル名で**書く。決定論テストが「旗を立てているファイルの名前がこの行に
//!   現れるか」を機械で照合しているので、型名や関数名へ言い換えるとその照合が落ちる
//!   （wintf 側は `tick_gate_tests.rs`、areka 側は `tick_gate_config_producers_tests.rs`）。
//! - [`WM_GEOMETRY`] — 幾何・DPI・表示構成・活性化・表示／破棄系メッセージの受理
//! - [`PRESENT`] — 表示指令の到着（areka 側の `PresentBridge`／`MoveCueSink`／lifecycle 送信端）
//! - [`ANIM`] — dola アニメータに活性がある
//! - [`REARM`] — 「まだ仕事がある」ので次の画面更新を予約する（talk 進行中の文字層など）
//! - [`GRAPHICS`] — 描画基盤が無効・初期化待ち
//! - [`FORCE`] — 明示の全走要求（起動直後・テスト・素性の分からない入来）
//!
//! 立てる側はどのスレッドからでもよく、何度立てても同じ（冪等）。錠は取らない。
//!
//! # 誰が読むか（消費者）
//!
//! 読むのは UI スレッドの 1 箇所だけで、画面更新（vblank）1 回につき [`take`] を 1 度呼ぶ。
//! [`take`] は旗を原子的に読み取って同時に倒すので、**倒した直後に立った旗は次の読み取りで
//! 拾える**。取りこぼしは起きず、遅れは最大でも画面更新 1 周期に収まる。
//!
//! # 待ち時間（期限）
//!
//! 「◯ミリ秒後にもう一度見たい」という要求は [`arm_deadline`] で預ける。預かるのは
//! **最も早い 1 つ**だけで、後から遅い時刻を入れても早い方が残る。[`take`] は期限が
//! 到来していれば `deadline_due` を立てて期限を倒し、まだなら期限をそのまま残す。
//!
//! # 疑わしいときは回す
//!
//! [`wake_bits_for_message`] は既知のウィンドウメッセージを旗へ写す純関数である。表に
//! 無いメッセージは [`FORCE`]（＝全走）へ落とす。旗を立て忘れて反応しなくなるより、
//! 余分に回る方が安全である——という判断であり、心拍・起動直後の全走と合わせて三重の
//! 安全網になっている。
//!
//! `WM_NCHITTEST` と `WM_SETCURSOR` を [`POINTER`] に入れているのは、この 2 つが
//! **ポインタが動いたときに届く**メッセージだからである（カーソルが止まっていれば 1 通も
//! 来ない）。したがって「ポインタ入力があった」の合図として過不足がない。

use std::ops::{BitOr, BitOrAssign};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::{
    WM_ACTIVATE, WM_ACTIVATEAPP, WM_CANCELMODE, WM_CAPTURECHANGED, WM_CLOSE, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_LBUTTONDBLCLK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEACTIVATE,
    WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_MOVE, WM_NCACTIVATE, WM_NCDESTROY,
    WM_NCHITTEST, WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMBUTTONDBLCLK,
    WM_NCMBUTTONDOWN, WM_NCMBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_NCRBUTTONDBLCLK,
    WM_NCRBUTTONDOWN, WM_NCRBUTTONUP, WM_NCXBUTTONDBLCLK, WM_NCXBUTTONDOWN, WM_NCXBUTTONUP,
    WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETTINGCHANGE, WM_SHOWWINDOW,
    WM_SIZE, WM_STYLECHANGED, WM_WINDOWPOSCHANGED, WM_WINDOWPOSCHANGING, WM_XBUTTONDBLCLK,
    WM_XBUTTONDOWN, WM_XBUTTONUP,
};

// ============================================================ 旗のビット集合

/// 起床の理由を表すビット集合。1 本の旗はビット 1 つで、OR で束ねられる。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WakeBits(pub u32);

impl WakeBits {
    /// 旗が 1 本も立っていない状態。
    pub const NONE: WakeBits = WakeBits(0);

    /// 生のビットを取り出す。
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// `other` の旗をすべて含むか（部分集合の判定）。
    pub const fn contains(self, other: WakeBits) -> bool {
        self.0 & other.0 == other.0
    }

    /// 2 つの旗を束ねる（`const` 文脈でも使える [`BitOr`] の別名）。
    pub const fn union(self, other: WakeBits) -> WakeBits {
        WakeBits(self.0 | other.0)
    }
}

impl BitOr for WakeBits {
    type Output = WakeBits;

    fn bitor(self, rhs: WakeBits) -> WakeBits {
        self.union(rhs)
    }
}

impl BitOrAssign for WakeBits {
    fn bitor_assign(&mut self, rhs: WakeBits) {
        self.0 |= rhs.0;
    }
}

/// ポインタ入力の投入・ポインタ系メッセージの受理。
pub const POINTER: WakeBits = WakeBits(1 << 0);
/// ドラッグ中（毎画面更新で回す・tick 末尾で自分で立て直す）。
pub const DRAG: WakeBits = WakeBits(1 << 1);
/// 窓書込指令の積み上げ（`command.rs` の enqueue）。
pub const WINDOW_CMD: WakeBits = WakeBits(1 << 2);
/// Z 順の要求（重なりの維持・利用者の操作への追随・タグ入口）。
///
/// 生産者の名前はここに書かない——名簿はモジュール冒頭のただ 1 つとし、写しを増やさない
/// （[`PRESENT`] と同じ流儀。写しが 2 つあると片方だけが古くなり、しかも静かに古くなる）。
pub const ZORDER: WakeBits = WakeBits(1 << 3);
/// 幾何・DPI・表示構成・活性化・表示／破棄系メッセージの受理。
pub const WM_GEOMETRY: WakeBits = WakeBits(1 << 4);
/// 表示指令の到着（areka 側の送信端）。
pub const PRESENT: WakeBits = WakeBits(1 << 5);
/// dola アニメータに活性がある。
pub const ANIM: WakeBits = WakeBits(1 << 6);
/// 次の画面更新の予約（「まだ仕事がある」系）。
pub const REARM: WakeBits = WakeBits(1 << 7);
/// 描画基盤が無効・初期化待ち。
pub const GRAPHICS: WakeBits = WakeBits(1 << 8);
/// 明示の全走要求（起動直後・テスト・素性の分からない入来）。
pub const FORCE: WakeBits = WakeBits(1 << 9);

/// 旗の名前表（ログと決定論テストが読む）。並びはビット 0..9 の順である。
pub const ALL: [(&str, WakeBits); 10] = [
    ("POINTER", POINTER),
    ("DRAG", DRAG),
    ("WINDOW_CMD", WINDOW_CMD),
    ("ZORDER", ZORDER),
    ("WM_GEOMETRY", WM_GEOMETRY),
    ("PRESENT", PRESENT),
    ("ANIM", ANIM),
    ("REARM", REARM),
    ("GRAPHICS", GRAPHICS),
    ("FORCE", FORCE),
];

// ================================================================ 読み取り結果

/// [`take`] が返す 1 回分の読み取り結果。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WakeSnapshot {
    /// 読み取った時点で立っていた旗（読み取りと同時に倒されている）。
    pub bits: u32,
    /// 預けてあった期限が到来していたか（到来していれば期限も倒されている）。
    pub deadline_due: bool,
}

impl WakeSnapshot {
    /// 旗も期限の到来も無い＝この画面更新で回す理由がない。
    pub fn is_empty(&self) -> bool {
        self.bits == 0 && !self.deadline_due
    }
}

// ==================================================================== 実体

/// 期限が預けられていないことを表す番兵。
const NO_DEADLINE: u64 = u64::MAX;

/// 旗と期限を持つ実体。
///
/// プロセス共有の [`GLOBAL`] が本番の唯一の実体だが、型としては素の構造体なので、
/// 決定論テストは自分専用の実体を作って調べられる（並列に走る他のテストの書き換えが
/// 見えない）。錠は 1 つも使わない——立てるのは `fetch_or`、期限は `fetch_min`、
/// 読み取りは `swap` で、いずれも待たない。
struct Wake {
    /// 立っている旗。
    bits: AtomicU32,
    /// 預かっている最も早い期限（[`Wake::epoch`] からのナノ秒。[`NO_DEADLINE`] なら無し）。
    deadline_nanos: AtomicU64,
    /// 時刻をナノ秒へ直すときの基準点（初めて時刻を扱うときに 1 度だけ決まる）。
    epoch: OnceLock<Instant>,
}

impl Wake {
    /// 旗も期限も無い状態で作る。
    const fn new() -> Wake {
        Wake {
            bits: AtomicU32::new(0),
            deadline_nanos: AtomicU64::new(NO_DEADLINE),
            epoch: OnceLock::new(),
        }
    }

    /// 基準点（初回呼び出し時に確定する）。
    fn epoch(&self) -> Instant {
        *self.epoch.get_or_init(Instant::now)
    }

    /// 時刻を基準点からのナノ秒へ直す。
    ///
    /// 基準点より前の時刻は 0 に丸める（＝必ず「到来済み」側に倒れる）。桁があふれる
    /// ほど遠い先の時刻は番兵の 1 つ手前で頭打ちにするので、[`NO_DEADLINE`] と
    /// 取り違えることはない。
    fn to_nanos(&self, at: Instant) -> u64 {
        let elapsed = at.saturating_duration_since(self.epoch()).as_nanos();
        let capped = elapsed.min(u128::from(NO_DEADLINE - 1));
        capped as u64
    }

    /// 旗を立てる（原子的な OR・冪等・任意のスレッドから）。
    fn mark(&self, bits: WakeBits) {
        if bits == WakeBits::NONE {
            return;
        }
        self.bits.fetch_or(bits.bits(), Ordering::AcqRel);
    }

    /// 期限を預ける。既に預かっている期限の方が早ければそちらを残す。
    fn arm_deadline(&self, at: Instant) {
        let nanos = self.to_nanos(at);
        self.deadline_nanos.fetch_min(nanos, Ordering::AcqRel);
    }

    /// 旗を読んで倒し、期限の到来を添えて返す。
    ///
    /// 期限を倒すのは到来していたときだけで、しかも**読んだ値のままなら**倒す
    /// （`compare_exchange`）。読んでから倒すまでの隙間に、より早い期限が別スレッドから
    /// 預けられた場合、その期限は消えずに残る。
    fn take(&self, now: Instant) -> WakeSnapshot {
        let bits = self.bits.swap(0, Ordering::AcqRel);

        let now_nanos = self.to_nanos(now);
        let deadline = self.deadline_nanos.load(Ordering::Acquire);
        let deadline_due = deadline != NO_DEADLINE && deadline <= now_nanos;
        if deadline_due {
            let _ = self.deadline_nanos.compare_exchange(
                deadline,
                NO_DEADLINE,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }

        WakeSnapshot { bits, deadline_due }
    }
}

/// プロセス共有の実体（本番はこの 1 つだけ）。
static GLOBAL: Wake = Wake::new();

/// 旗を立てる（原子的な OR）。どのスレッドからでも何度でも呼んでよい（冪等）。
pub fn mark(bits: WakeBits) {
    GLOBAL.mark(bits);
}

/// 期限を預ける。最も早い 1 つだけが残る。
pub fn arm_deadline(at: Instant) {
    GLOBAL.arm_deadline(at);
}

/// 旗を読んで倒す。画面更新 1 回につき UI スレッドが 1 度だけ呼ぶ。
///
/// 読み取りの後に立った旗は次の呼び出しで拾える。
pub fn take(now: Instant) -> WakeSnapshot {
    GLOBAL.take(now)
}

// ============================================================== 写像表

/// 既知のウィンドウメッセージと立てる旗の対応表。
///
/// [`wake_bits_for_message`] の `match` と**同じ中身**であり、片方だけ直してはならない。
/// 食い違いは兄弟の決定論テストが両方向から捕まえる（表→純関数と、番号の総なめ）。
pub const KNOWN_MESSAGE_TABLE: &[(u32, WakeBits)] = &[
    // 幾何・DPI・表示構成・活性化・表示／破棄
    (WM_WINDOWPOSCHANGING, WM_GEOMETRY),
    (WM_WINDOWPOSCHANGED, WM_GEOMETRY),
    (WM_SIZE, WM_GEOMETRY),
    (WM_MOVE, WM_GEOMETRY),
    (WM_DPICHANGED, WM_GEOMETRY),
    (WM_DISPLAYCHANGE, WM_GEOMETRY),
    (WM_SETTINGCHANGE, WM_GEOMETRY),
    (WM_ACTIVATE, WM_GEOMETRY),
    (WM_ACTIVATEAPP, WM_GEOMETRY),
    (WM_NCACTIVATE, WM_GEOMETRY),
    (WM_SHOWWINDOW, WM_GEOMETRY),
    (WM_DESTROY, WM_GEOMETRY),
    (WM_NCDESTROY, WM_GEOMETRY),
    (WM_CLOSE, WM_GEOMETRY),
    (WM_STYLECHANGED, WM_GEOMETRY),
    (WM_ENTERSIZEMOVE, WM_GEOMETRY),
    (WM_EXITSIZEMOVE, WM_GEOMETRY),
    // ポインタ（クライアント領域）
    (WM_MOUSEMOVE, POINTER),
    (WM_MOUSELEAVE, POINTER),
    (WM_SETCURSOR, POINTER),
    (WM_NCHITTEST, POINTER),
    (WM_MOUSEACTIVATE, POINTER),
    (WM_LBUTTONDOWN, POINTER),
    (WM_LBUTTONUP, POINTER),
    (WM_LBUTTONDBLCLK, POINTER),
    (WM_RBUTTONDOWN, POINTER),
    (WM_RBUTTONUP, POINTER),
    (WM_RBUTTONDBLCLK, POINTER),
    (WM_MBUTTONDOWN, POINTER),
    (WM_MBUTTONUP, POINTER),
    (WM_MBUTTONDBLCLK, POINTER),
    (WM_XBUTTONDOWN, POINTER),
    (WM_XBUTTONUP, POINTER),
    (WM_XBUTTONDBLCLK, POINTER),
    (WM_MOUSEWHEEL, POINTER),
    (WM_MOUSEHWHEEL, POINTER),
    (WM_CAPTURECHANGED, POINTER),
    (WM_CANCELMODE, POINTER),
    // ポインタ（非クライアント領域）
    (WM_NCMOUSEMOVE, POINTER),
    (WM_NCMOUSELEAVE, POINTER),
    (WM_NCLBUTTONDOWN, POINTER),
    (WM_NCLBUTTONUP, POINTER),
    (WM_NCLBUTTONDBLCLK, POINTER),
    (WM_NCRBUTTONDOWN, POINTER),
    (WM_NCRBUTTONUP, POINTER),
    (WM_NCRBUTTONDBLCLK, POINTER),
    (WM_NCMBUTTONDOWN, POINTER),
    (WM_NCMBUTTONUP, POINTER),
    (WM_NCMBUTTONDBLCLK, POINTER),
    (WM_NCXBUTTONDOWN, POINTER),
    (WM_NCXBUTTONUP, POINTER),
    (WM_NCXBUTTONDBLCLK, POINTER),
];

/// ウィンドウメッセージを起床の旗へ写す純関数。
///
/// 表に無いメッセージは [`FORCE`]（全走）へ落とす——**疑わしいときは回す**。旗を立て
/// 忘れて反応しなくなる方が、余分に 1 回回るより高くつくからである。
///
/// 副作用は無い。旗を立てるのは呼び出し側（メッセージの配送点）の仕事である。
pub fn wake_bits_for_message(msg: u32) -> WakeBits {
    match msg {
        // 幾何・DPI・表示構成・活性化・表示／破棄——窓の形と見え方が変わりうる。
        WM_WINDOWPOSCHANGING | WM_WINDOWPOSCHANGED | WM_SIZE | WM_MOVE | WM_DPICHANGED
        | WM_DISPLAYCHANGE | WM_SETTINGCHANGE | WM_ACTIVATE | WM_ACTIVATEAPP | WM_NCACTIVATE
        | WM_SHOWWINDOW | WM_DESTROY | WM_NCDESTROY | WM_CLOSE | WM_STYLECHANGED
        | WM_ENTERSIZEMOVE | WM_EXITSIZEMOVE => WM_GEOMETRY,

        // ポインタ——いずれもポインタが動いた／押されたときにだけ届く。
        WM_MOUSEMOVE | WM_MOUSELEAVE | WM_SETCURSOR | WM_NCHITTEST | WM_MOUSEACTIVATE
        | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP
        | WM_RBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_MBUTTONDBLCLK | WM_XBUTTONDOWN
        | WM_XBUTTONUP | WM_XBUTTONDBLCLK | WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_CAPTURECHANGED
        | WM_CANCELMODE | WM_NCMOUSEMOVE | WM_NCMOUSELEAVE | WM_NCLBUTTONDOWN | WM_NCLBUTTONUP
        | WM_NCLBUTTONDBLCLK | WM_NCRBUTTONDOWN | WM_NCRBUTTONUP | WM_NCRBUTTONDBLCLK
        | WM_NCMBUTTONDOWN | WM_NCMBUTTONUP | WM_NCMBUTTONDBLCLK | WM_NCXBUTTONDOWN
        | WM_NCXBUTTONUP | WM_NCXBUTTONDBLCLK => POINTER,

        // 表に無いものは全走（疑わしいときは回す）。
        _ => FORCE,
    }
}

#[cfg(test)]
#[path = "tick_wake_tests.rs"]
mod tests;

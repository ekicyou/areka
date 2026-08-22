//! DPI／拡大率遷移の**観測チャネル**——専用 target・レコード語彙・行を組む純関数。
//!
//! design.md の C1（`transition_diag`）が正本である。1 回の遷移に含まれる窓書込・
//! サーフェス更新・モニタ表更新・メッセージ受理を**単一のフレーム番号系列**の 1 本の
//! 時系列へ並べるために、3 つの crate（wintf・areka-emo-present・areka）が同じ target と
//! 同じ接頭語で行を出す。その語彙をここ 1 箇所に閉じ込める。
//!
//! # 行の形
//!
//! ```text
//! [transition] frame=<u32> t_us=<u64> kind=<k> <名前>=<値> …
//! ```
//!
//! - 空白区切りの `名前=値` の並びで、`tools/perf/judge-perf.py::parse_fields` と**同じ
//!   辞書化規則**（値は次の名前の直前まで）で読める。
//! - 値が無いフィールドは番兵 [`MISSING`]（`-`）で埋め、**フィールドごと落とさない**。
//!   落とすと「記録が出ていない」と「その経路にはその値が無い」の区別が事後に付かない。
//! - `frame`／`t_us` は呼び出し側が [`Stamp`] として渡す。**本モジュールの純関数は時刻を
//!   読まない**——テストが任意の刻印で行を組めることが語彙固定テストの前提である。
//!
//! # 同名フィールドを作らない
//!
//! 辞書化は**後勝ち**なので、1 行に同じ名前が 2 度出ると先の値が消える。接頭語の
//! `kind=` はレコード種別（`monitor`／`write`／…）に予約されているため、書込タグが運ぶ
//! **窓の種別**は行の上では `win_kind=` という別名で出す（[`WriteTag::kind`] の値が入る）。
//! この一点だけが design.md「Data Models」表の字面からの意図的な逸脱であり、逸脱しないと
//! `kind=write … kind=shell` の 2 度出しでレコード種別が判定側から消える。
//!
//! # ここに置いてよいもの・置いてはならないもの
//!
//! 置いてよいのは**純粋な文字列組立**と、フレーム／flush の時刻基準だけである。
//! `tracing` のマクロ呼出は [`emit_line`] の 1 箇所に集約してあり、レコード純関数
//! （`*_line`）はログに触れない。組立と発行を分けておくと、テストは実際の濾過に依らず
//! 字面を固定でき、発行側は組んだ文字列をそのまま本文にできる（組立を二重に持たない）。
//!
//! # 既定 OFF
//!
//! 本チャネルは `debug` 水準・専用 target [`TRANSITION_TARGET`] で出るため、既定の
//! `RUST_LOG=info` では 1 行も出ない。有効化は `RUST_LOG=wintf::transition=debug` の 1 語。
//! 有効化されていない状態の「出力が無いこと」を、いかなる判定の根拠にも用いてはならない
//! （Requirement 2.5）。

use std::cell::Cell;
use std::time::Instant;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HWND, RECT};

use crate::ecs::world::FrameCount;

/// 観測チャネルの専用 target（3 crate が同じ文字列で発行する・D2）。
///
/// `RUST_LOG` の directive は 1 語で足りる: `wintf::transition=debug`。
pub const TRANSITION_TARGET: &str = "wintf::transition";

/// 全レコード共通の行頭タグ（サインオフの grep 語）。
pub const RECORD_PREFIX_TAG: &str = "[transition]";

/// 値が取得できなかったフィールドの番兵。
pub const MISSING: &str = "-";

// ---------------------------------------------------------------------------
// レコード種別（kind 語）
// ---------------------------------------------------------------------------

/// モニタ表の値変化（DPI・作業領域）。
pub const KIND_MONITOR: &str = "monitor";
/// 窓書込 1 回（一括 flush・メッセージ受理時の同期書込のいずれも）。
pub const KIND_WRITE: &str = "write";
/// 一括 flush の開始／終了。
pub const KIND_FLUSH: &str = "flush";
/// ウィンドウメッセージの受理。
pub const KIND_MSG: &str = "msg";
/// 窓書込指令のキューへの積み上げ（合流の有無を含む）。
pub const KIND_ENQUEUE: &str = "enqueue";

/// wintf が発行する `kind` 語の全体（判定側の語彙照合・語の一意性テストが参照する）。
///
/// areka／areka-emo-present が足す `snapshot`／`surface`／`hold`／`ground`／`chain` は
/// それぞれの crate のレコード純関数が定義する（wintf は上位 crate の語彙を持たない）。
pub const KIND_ALL: &[&str] = &[KIND_MONITOR, KIND_WRITE, KIND_FLUSH, KIND_MSG, KIND_ENQUEUE];

// ---------------------------------------------------------------------------
// 段階語（stage）
// ---------------------------------------------------------------------------

/// 窓書込がフレーム末尾の一括 flush で行われた（経路 B）。
pub const STAGE_FLUSH: &str = "flush";
/// 窓書込がメッセージ受理時の同期書込で行われた（経路 A）。
pub const STAGE_SYNC: &str = "sync";
/// 一括 flush の開始。
pub const STAGE_BEGIN: &str = "begin";
/// 一括 flush の終了。
pub const STAGE_END: &str = "end";

/// wintf が発行する `stage` 語の全体。
pub const STAGE_ALL: &[&str] = &[STAGE_FLUSH, STAGE_SYNC, STAGE_BEGIN, STAGE_END];

// ---------------------------------------------------------------------------
// メッセージ名（msg 語）
// ---------------------------------------------------------------------------

/// 窓ごとの DPI 変更通知。
pub const MSG_DPICHANGED: &str = "WM_DPICHANGED";
/// 窓位置・寸の確定通知。
pub const MSG_WINDOWPOSCHANGED: &str = "WM_WINDOWPOSCHANGED";
/// 表示構成の変更通知。
pub const MSG_DISPLAYCHANGE: &str = "WM_DISPLAYCHANGE";

/// wintf が記録するウィンドウメッセージ名の全体。
pub const MSG_ALL: &[&str] = &[MSG_DPICHANGED, MSG_WINDOWPOSCHANGED, MSG_DISPLAYCHANGE];

// ---------------------------------------------------------------------------
// 経路語彙（origin）のうち wintf が自分で名乗るもの
// ---------------------------------------------------------------------------
//
// areka の配置経路語彙（`PlacementRoute`）は areka が定義し、wintf は `&'static str` として
// 受け取るだけである（依存方向 wintf ← areka）。以下の 2 語は表示基盤自身が積む書込の
// 出所であり、areka の経路語彙とは交わらない。定義元をここ 1 箇所にするのは、積み上げ元
// （`zorder_pair_maintain`／`graphics::systems::window_pos`）と判定側が同じ語を見るためである。

/// ペアの重なり維持系（`zorder_pair_maintain::pair_fix_command`）が積む Z のみの書込。
pub const ORIGIN_ZORDER_PAIR: &str = "zorder-pair";
/// 汎用の位置反映（`graphics::systems::window_pos::apply_window_pos_changes`）が積む書込。
pub const ORIGIN_WINDOW_POS: &str = "window-pos";
/// OS 提案位置を採ったときの同期書込（`window_proc/window_pos.rs` の `WM_DPICHANGED`）。
///
/// 上 2 語と違い、この経路は**キューへ積まない**——受理の内側で直に撃つ（経路 A）。
/// `stage=sync` でも見分けは付くが、`origin` は「どの経路が書込を要求したか」を運ぶ
/// フィールド（要件 2.1）であり、実在する要求元を番兵で埋めると
/// [`WriteTag::UNTAGGED`]（＝タグの付け忘れ）と区別が付かなくなる。
pub const ORIGIN_DPI_SUGGESTED: &str = "dpi-suggested";

// ---------------------------------------------------------------------------
// フィールド名（判定側が grep／辞書引きする語）
// ---------------------------------------------------------------------------

/// 接頭語: フレーム番号。
pub const FIELD_FRAME: &str = "frame";
/// 接頭語: tick 開始からの経過（µs・参考値であって判定語ではない）。
pub const FIELD_T_US: &str = "t_us";
/// 接頭語: レコード種別。
pub const FIELD_KIND: &str = "kind";

/// モニタ表のエンティティ。
pub const FIELD_ENTITY: &str = "entity";
/// 変化前の DPI。
pub const FIELD_OLD_DPI: &str = "old_dpi";
/// 変化後の DPI。
pub const FIELD_NEW_DPI: &str = "new_dpi";
/// 変化前の作業領域（`left,top,right,bottom`）。
pub const FIELD_OLD_WA: &str = "old_wa";
/// 変化後の作業領域（`left,top,right,bottom`）。
pub const FIELD_NEW_WA: &str = "new_wa";

/// 書込・flush の段階語。
pub const FIELD_STAGE: &str = "stage";
/// 一括 flush 内での通し番号（合流先の指定にも使う）。
pub const FIELD_SEQ: &str = "seq";
/// 対象ウィンドウハンドル（16 進）。
pub const FIELD_HWND: &str = "hwnd";
/// 書込を要求した経路語彙。
pub const FIELD_ORIGIN: &str = "origin";
/// キャラ番号（持たない窓は番兵）。
pub const FIELD_SCOPE: &str = "scope";
/// 窓の種別。**接頭語の `kind` と衝突させないための別名**（module doc 参照）。
pub const FIELD_WIN_KIND: &str = "win_kind";
/// 要求位置 X。
pub const FIELD_X: &str = "x";
/// 要求位置 Y。
pub const FIELD_Y: &str = "y";
/// 要求幅。
pub const FIELD_CX: &str = "cx";
/// 要求高さ。
pub const FIELD_CY: &str = "cy";
/// `SetWindowPos` のフラグ（16 進）。
pub const FIELD_FLAGS: &str = "flags";
/// 書込後矩形の左（読み戻せなければ番兵）。
pub const FIELD_AX: &str = "ax";
/// 書込後矩形の上（読み戻せなければ番兵）。
pub const FIELD_AY: &str = "ay";
/// 書込後矩形の幅（読み戻せなければ番兵）。
pub const FIELD_AW: &str = "aw";
/// 書込後矩形の高さ（読み戻せなければ番兵）。
pub const FIELD_AH: &str = "ah";
/// 書込 1 回の所要（µs・参考値）。
pub const FIELD_CALL_US: &str = "call_us";
/// 書込の成否。
pub const FIELD_OK: &str = "ok";
/// 当該の書込が `DeferWindowPos` の 1 バッチで投入されたか（偽なら 1 本ずつの書込）。
pub const FIELD_IN_BATCH: &str = "in_batch";

/// 一括 flush の指令数。
pub const FIELD_COUNT: &str = "count";
/// tick 開始から flush 開始までの経過（µs）。
pub const FIELD_SINCE_TICK_US: &str = "since_tick_us";
/// 一括 flush 全体の所要（µs・開始行では番兵）。
pub const FIELD_TOTAL_US: &str = "total_us";

/// 受理したウィンドウメッセージ名。
pub const FIELD_MSG: &str = "msg";
/// `SetWindowPos` 呼出の内側で受理されたか。
pub const FIELD_IN_SWP: &str = "in_swp";
/// flush 開始からの経過（µs・flush 外は番兵）。
pub const FIELD_SINCE_FLUSH_US: &str = "since_flush_us";

/// 合流先の通し番号（合流しなかった場合は番兵）。
pub const FIELD_MERGED_INTO_SEQ: &str = "merged_into_seq";

/// `kind=monitor` 行の必須フィールド（接頭語を除く）。
pub const MONITOR_FIELDS: &[&str] = &[
    FIELD_ENTITY,
    FIELD_OLD_DPI,
    FIELD_NEW_DPI,
    FIELD_OLD_WA,
    FIELD_NEW_WA,
];

/// `kind=write` 行の必須フィールド（接頭語を除く）。
pub const WRITE_FIELDS: &[&str] = &[
    FIELD_STAGE,
    FIELD_SEQ,
    FIELD_HWND,
    FIELD_ORIGIN,
    FIELD_SCOPE,
    FIELD_WIN_KIND,
    FIELD_X,
    FIELD_Y,
    FIELD_CX,
    FIELD_CY,
    FIELD_FLAGS,
    FIELD_AX,
    FIELD_AY,
    FIELD_AW,
    FIELD_AH,
    FIELD_CALL_US,
    FIELD_OK,
];

/// `kind=write` 行の**任意**フィールド（接頭語を除く）。
///
/// # なぜ必須（[`WRITE_FIELDS`]）ではないのか
///
/// [`FIELD_IN_BATCH`] は一括書込のバッチ化（設計 C8 の候補 B-2b・task 7.2）と同時に増えた
/// フィールドであり、**それ以前に採取した実機ログには 1 行も載っていない**。必須列へ入れると
/// 判定器（`transition_judge::required_fields`）が過去の逐語ログを軒並み「壊れた行」として
/// 数え、確定済みの証跡（`mechanism-ledger.md` §3・§10 の採取）を後から否定してしまう。
///
/// 発行側が本フィールドを**必ず**載せることは行の逐語テスト
/// （`transition_diag_tests::write_line_is_verbatim`）が固定する——任意なのは
/// 「読む側が古いログを受け入れる」という意味であって、「書く側が省いてよい」ではない。
pub const WRITE_OPTIONAL_FIELDS: &[&str] = &[FIELD_IN_BATCH];

/// `kind=flush` 行の必須フィールド（接頭語を除く）。
pub const FLUSH_FIELDS: &[&str] = &[
    FIELD_STAGE,
    FIELD_COUNT,
    FIELD_SINCE_TICK_US,
    FIELD_TOTAL_US,
];

/// `kind=msg` 行の必須フィールド（接頭語を除く）。
pub const MSG_FIELDS: &[&str] = &[FIELD_MSG, FIELD_HWND, FIELD_IN_SWP, FIELD_SINCE_FLUSH_US];

/// `kind=enqueue` 行の必須フィールド（接頭語を除く）。
pub const ENQUEUE_FIELDS: &[&str] = &[
    FIELD_HWND,
    FIELD_ORIGIN,
    FIELD_SCOPE,
    FIELD_WIN_KIND,
    FIELD_MERGED_INTO_SEQ,
];

// ---------------------------------------------------------------------------
// 刻印とタグ
// ---------------------------------------------------------------------------

/// 全レコード共通の刻印。
///
/// 発行点が値を組んで渡し、レコード純関数は**受け取るだけ**（時刻を読まない）。
/// World を借りられる観測点は `FrameCount`／`TickStart` から、借りられない観測点
/// （一括 flush・wndproc）はスレッド局所ミラーから組む（D1）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    /// フレーム番号（`FrameCount`・u32・周回する）。
    ///
    /// 周回するため、判定は `wrapping_sub` の差分だけで行い、絶対値比較を判定語に
    /// 用いてはならない（D14）。
    pub frame: u32,
    /// tick 開始からの経過（µs）。参考値であって判定語ではない。
    pub t_us: u64,
}

/// 窓書込を要求した経路の語彙タグ。
///
/// 「どの経路が・どの窓へ」を書込レコードの 1 行に載せるためのもので、wintf は中身の
/// 意味を知らない（areka の `PlacementRoute` などを `&'static str` として受け取るだけ＝
/// 依存方向 wintf ← areka を保つ）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteTag {
    /// 要求元の経路語彙（例: areka の `PlacementRoute::as_str()`・`"zorder-pair"`）。
    pub origin: &'static str,
    /// キャラ番号（持たない窓は `None`＝行では番兵）。
    pub scope: Option<u32>,
    /// 窓の種別（例: `"shell"`・`"balloon"`）。行の上では `win_kind=` として出る。
    pub kind: &'static str,
}

impl WriteTag {
    /// タグが付いていない書込（既定値）。
    ///
    /// 3 フィールドとも番兵になるので、「タグを付け忘れた経路」が行の上で見分けられる。
    pub const UNTAGGED: WriteTag = WriteTag {
        origin: MISSING,
        scope: None,
        kind: MISSING,
    };
}

impl Default for WriteTag {
    fn default() -> Self {
        Self::UNTAGGED
    }
}

/// 窓書込がどちらの経路で行われたか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteStage {
    /// フレーム末尾の一括 flush（経路 B）。
    Flush,
    /// メッセージ受理時の同期書込（経路 A）。
    Sync,
}

impl WriteStage {
    /// 行に載る段階語。
    pub fn as_str(self) -> &'static str {
        match self {
            WriteStage::Flush => STAGE_FLUSH,
            WriteStage::Sync => STAGE_SYNC,
        }
    }
}

/// 一括 flush の区間の端。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlushStage {
    /// 区間の開始。
    Begin,
    /// 区間の終了。
    End,
}

impl FlushStage {
    /// 行に載る段階語。
    pub fn as_str(self) -> &'static str {
        match self {
            FlushStage::Begin => STAGE_BEGIN,
            FlushStage::End => STAGE_END,
        }
    }
}

// ---------------------------------------------------------------------------
// レコード
// ---------------------------------------------------------------------------

/// モニタ表の値変化（DPI・作業領域）。
#[derive(Clone, Copy, Debug)]
pub struct MonitorRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// モニタのエンティティ。
    pub entity: Entity,
    /// 変化前の DPI。
    pub old_dpi: u32,
    /// 変化後の DPI。
    pub new_dpi: u32,
    /// 変化前の作業領域。
    pub old_work_area: RECT,
    /// 変化後の作業領域。
    pub new_work_area: RECT,
}

/// 窓書込 1 回。
#[derive(Clone, Copy, Debug)]
pub struct WriteRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// 経路（一括 flush かメッセージ受理時の同期書込か）。
    pub stage: WriteStage,
    /// 一括 flush 内での通し番号（同期書込では 0）。
    pub seq: u32,
    /// 対象ウィンドウハンドル。
    pub hwnd: HWND,
    /// 要求語彙タグ。
    pub tag: WriteTag,
    /// 要求位置 X。
    pub x: i32,
    /// 要求位置 Y。
    pub y: i32,
    /// 要求幅。
    pub cx: i32,
    /// 要求高さ。
    pub cy: i32,
    /// `SetWindowPos` のフラグ。
    pub flags: u32,
    /// 書込後の物理矩形。読み戻せなかった場合は `None`（4 フィールドとも番兵になる）。
    pub after: Option<RECT>,
    /// 書込 1 回の所要（µs・参考値）。
    ///
    /// [`in_batch`](Self::in_batch) が真のときは**投入（`DeferWindowPos`）だけの所要**であり、
    /// 実際に窓が動く時間は入っていない（それは 1 バッチぶんまとめて
    /// `flush stage=end` の `total_us` に載る）。偽のときは `SetWindowPos` 呼出 1 本の所要である。
    pub call_us: u64,
    /// 書込の成否。
    pub ok: bool,
    /// `DeferWindowPos` の 1 バッチで投入されたか（設計 C8 の候補 B-2b）。
    ///
    /// 偽になるのは ⑴ メッセージ受理時の同期書込（`stage=sync`）と ⑵ バッチ投入に失敗して
    /// 1 本ずつの `SetWindowPos` へ**縮退**した一括 flush の 2 つである。縮退は
    /// `warn!` を伴う（無音で形が変わらない）。
    pub in_batch: bool,
}

/// 一括 flush 区間の端。
#[derive(Clone, Copy, Debug)]
pub struct FlushRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// 区間の端。
    pub stage: FlushStage,
    /// 区間で処理する指令数。
    pub count: usize,
    /// tick 開始から flush 開始までの経過（µs）。
    pub since_tick_us: u64,
    /// 区間全体の所要（µs）。開始行ではまだ無いので `None`（番兵）。
    ///
    /// `0` へ潰さないのは、「0µs で終わった」と「まだ測っていない」を同じ字面に
    /// しないためである。
    pub total_us: Option<u64>,
}

/// ウィンドウメッセージの受理。
#[derive(Clone, Copy, Debug)]
pub struct MsgRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// メッセージ名（[`MSG_ALL`] のいずれか）。
    pub msg: &'static str,
    /// 対象ウィンドウハンドル。
    pub hwnd: HWND,
    /// `SetWindowPos` 呼出の内側で受理されたか（同期送達の証跡）。
    pub in_swp: bool,
    /// flush 開始からの経過（µs）。flush の外で受理した場合は `None`（番兵）。
    pub since_flush_us: Option<u64>,
}

/// 窓書込指令のキューへの積み上げ。
#[derive(Clone, Copy, Debug)]
pub struct EnqueueRecord {
    /// 刻印。
    pub stamp: Stamp,
    /// 対象ウィンドウハンドル。
    pub hwnd: HWND,
    /// 要求語彙タグ。
    pub tag: WriteTag,
    /// 先着の指令へ合流した場合、その通し番号。合流しなければ `None`（番兵）。
    pub merged_into_seq: Option<u32>,
}

// ---------------------------------------------------------------------------
// フィールドの表現（番兵はここだけが作る）
// ---------------------------------------------------------------------------

/// `HWND` を 16 進表現へ。
///
/// `Debug` 表現をそのまま使わないのは、値に空白や記号が混じると `名前=値` の 1 行から
/// 機械的に切り出せなくなるためである。
fn hwnd_field(hwnd: HWND) -> String {
    format!("0x{:X}", hwnd.0 as usize)
}

/// `RECT` を `left,top,right,bottom` へ（空白を含まない）。
fn rect_field(rect: RECT) -> String {
    format!("{},{},{},{}", rect.left, rect.top, rect.right, rect.bottom)
}

/// 省略可能な値を表現へ（不在は番兵）。
fn opt_field<T: std::fmt::Display>(value: Option<T>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => MISSING.to_string(),
    }
}

/// 書込タグの 3 フィールド（`origin` / `scope` / `win_kind`）。
fn tag_fields(tag: WriteTag) -> String {
    format!(
        "{FIELD_ORIGIN}={origin} {FIELD_SCOPE}={scope} {FIELD_WIN_KIND}={win_kind}",
        origin = tag.origin,
        scope = opt_field(tag.scope),
        win_kind = tag.kind,
    )
}

/// 書込後矩形の 4 フィールド（`ax` / `ay` / `aw` / `ah`）。
///
/// 読み戻せなかった場合も**フィールドは 4 つとも残す**（番兵で埋める）。
fn after_fields(after: Option<RECT>) -> String {
    match after {
        Some(r) => format!(
            "{FIELD_AX}={x} {FIELD_AY}={y} {FIELD_AW}={w} {FIELD_AH}={h}",
            x = r.left,
            y = r.top,
            w = r.right - r.left,
            h = r.bottom - r.top,
        ),
        None => format!(
            "{FIELD_AX}={MISSING} {FIELD_AY}={MISSING} {FIELD_AW}={MISSING} {FIELD_AH}={MISSING}"
        ),
    }
}

// ---------------------------------------------------------------------------
// レコード純関数（時刻を読まない・ログに触れない）
// ---------------------------------------------------------------------------

/// 全レコード共通の接頭語を組む。
pub fn record_prefix(stamp: Stamp, kind: &'static str) -> String {
    format!(
        "{RECORD_PREFIX_TAG} {FIELD_FRAME}={frame} {FIELD_T_US}={t_us} {FIELD_KIND}={kind}",
        frame = stamp.frame,
        t_us = stamp.t_us,
    )
}

/// モニタ表の値変化の記録行。
pub fn monitor_line(record: &MonitorRecord) -> String {
    format!(
        "{prefix} {FIELD_ENTITY}={entity:?} {FIELD_OLD_DPI}={old_dpi} {FIELD_NEW_DPI}={new_dpi} \
         {FIELD_OLD_WA}={old_wa} {FIELD_NEW_WA}={new_wa}",
        prefix = record_prefix(record.stamp, KIND_MONITOR),
        entity = record.entity,
        old_dpi = record.old_dpi,
        new_dpi = record.new_dpi,
        old_wa = rect_field(record.old_work_area),
        new_wa = rect_field(record.new_work_area),
    )
}

/// 窓書込 1 回の記録行。
pub fn write_line(record: &WriteRecord) -> String {
    format!(
        "{prefix} {FIELD_STAGE}={stage} {FIELD_SEQ}={seq} {FIELD_HWND}={hwnd} {tag} \
         {FIELD_X}={x} {FIELD_Y}={y} {FIELD_CX}={cx} {FIELD_CY}={cy} {FIELD_FLAGS}=0x{flags:X} \
         {after} {FIELD_CALL_US}={call_us} {FIELD_OK}={ok} {FIELD_IN_BATCH}={in_batch}",
        prefix = record_prefix(record.stamp, KIND_WRITE),
        stage = record.stage.as_str(),
        seq = record.seq,
        hwnd = hwnd_field(record.hwnd),
        tag = tag_fields(record.tag),
        x = record.x,
        y = record.y,
        cx = record.cx,
        cy = record.cy,
        flags = record.flags,
        after = after_fields(record.after),
        call_us = record.call_us,
        ok = record.ok,
        in_batch = record.in_batch,
    )
}

/// 一括 flush 区間の端の記録行。
pub fn flush_line(record: &FlushRecord) -> String {
    format!(
        "{prefix} {FIELD_STAGE}={stage} {FIELD_COUNT}={count} \
         {FIELD_SINCE_TICK_US}={since_tick_us} {FIELD_TOTAL_US}={total_us}",
        prefix = record_prefix(record.stamp, KIND_FLUSH),
        stage = record.stage.as_str(),
        count = record.count,
        since_tick_us = record.since_tick_us,
        total_us = opt_field(record.total_us),
    )
}

/// ウィンドウメッセージ受理の記録行。
pub fn msg_line(record: &MsgRecord) -> String {
    format!(
        "{prefix} {FIELD_MSG}={msg} {FIELD_HWND}={hwnd} {FIELD_IN_SWP}={in_swp} \
         {FIELD_SINCE_FLUSH_US}={since_flush_us}",
        prefix = record_prefix(record.stamp, KIND_MSG),
        msg = record.msg,
        hwnd = hwnd_field(record.hwnd),
        in_swp = record.in_swp,
        since_flush_us = opt_field(record.since_flush_us),
    )
}

/// 窓書込指令の積み上げの記録行。
pub fn enqueue_line(record: &EnqueueRecord) -> String {
    format!(
        "{prefix} {FIELD_HWND}={hwnd} {tag} {FIELD_MERGED_INTO_SEQ}={merged}",
        prefix = record_prefix(record.stamp, KIND_ENQUEUE),
        hwnd = hwnd_field(record.hwnd),
        tag = tag_fields(record.tag),
        merged = opt_field(record.merged_into_seq),
    )
}

// ---------------------------------------------------------------------------
// 発行（tracing に触れるのはここだけ）
// ---------------------------------------------------------------------------

/// 観測チャネルが点灯しているか（前置ガード）。
///
/// レコードを組む前にこれで分岐すれば、既定 OFF の運転で文字列の確保も
/// `GetWindowRect` の読み戻しも一切行わない（`recompose-budget` が成立させた定常状態の
/// アロケーション 0 を壊さないための口）。
#[inline]
pub fn is_enabled() -> bool {
    tracing::enabled!(target: TRANSITION_TARGET, tracing::Level::DEBUG)
}

/// 組み上がった 1 行を観測チャネルへ出す。
///
/// 本文は行そのもの（構造化フィールドへ分解しない）——判定側とサインオフ手順書は
/// 行の字面を辞書化して読むため、書式を 1 箇所（レコード純関数）に閉じ込めておく。
#[inline]
pub fn emit_line(line: &str) {
    tracing::debug!(target: TRANSITION_TARGET, "{line}");
}

// ---------------------------------------------------------------------------
// 刻印の権威（D1）——World 資源が正・スレッド局所ミラーは World 外専用
// ---------------------------------------------------------------------------

/// tick の開始時刻（`FrameCount` と**同じ点**で更新される World 資源）。
///
/// `FrameCount` と対にして [`stamp_from_world`] へ渡すと、World を借りられる観測点は
/// どのスレッドに載っていても正しい刻印を組める。更新点は
/// `crate::ecs::world::EcsWorld::try_tick_world` の `FrameCount` 増分と同一点の 1 箇所
/// だけである（2 箇所で進めると 1 本のフレーム系列が割れる）。
#[derive(Resource, Clone, Copy, Debug)]
pub struct TickStart(pub Instant);

thread_local! {
    /// 直近 tick の `(フレーム番号, tick 開始時刻)` の**写し**。tick 前は `None`。
    ///
    /// スレッド局所なのは、⑴ World を借りられない観測点（一括 flush・wndproc）はどれも
    /// UI スレッド上にあり、⑵ プロセス大域の atomic にするとテスト間で値が残って判定を
    /// 変えてしまう（要件 7.7）ためである。テストはスレッドごとに走るので、この形なら
    /// 隔離が自然に成立する（それでも [`reset_for_test`] で明示的に初期化する）。
    static TICK_MIRROR: Cell<Option<(u32, Instant)>> = const { Cell::new(None) };
}

/// 経過を µs へ（飽和・µs への換算規則はここだけが持つ）。
///
/// 一括 flush（`command.rs`）が自前の起点から所要を測るためにクレート内へ開いてある。
/// 換算と飽和を 2 箇所に持つと、片方だけが `u64` 溢れの扱いを変えたときに静かに食い違う。
pub(crate) fn elapsed_us(at: Instant) -> u64 {
    u64::try_from(at.elapsed().as_micros()).unwrap_or(u64::MAX)
}

/// tick の開始を刻む——`FrameCount` 増分・[`TickStart`] 更新と**同一点で同じ値**を
/// スレッド局所の写しへ配る。
///
/// 呼び出しは tick ごとに 1 回（`world/mod.rs` の `FrameCount` 増分直後）。
pub fn begin_tick(frame: u32, start: Instant) {
    TICK_MIRROR.with(|mirror| mirror.set(Some((frame, start))));
}

/// 直近 tick のフレーム番号（**World を借りられない観測点専用**）。
///
/// 読んでよいのは一括 flush（World の借用を解放した後に走る）と wndproc の同期経路
/// だけである。World を借りられる観測点は既定の**多スレッド**実行器でワーカースレッドへ
/// 載り得て、そこからは UI スレッドの写しが見えない（frame=0 の行が出る）ため、必ず
/// [`stamp_from_world`] を使うこと（D1）。
///
/// tick 前は `0`。tick の外で読むと「直近 tick の番号」になるため、判定は
/// `wrapping_sub` の差分だけで行う（D14）。
pub fn current_frame() -> u32 {
    TICK_MIRROR
        .with(|mirror| mirror.get())
        .map_or(0, |(frame, _)| frame)
}

/// tick 開始からの経過（µs・**World を借りられない観測点専用**）。
///
/// 参考値であって判定語ではない。tick 前は `0`。読んでよい点は [`current_frame`] と同じ。
pub fn since_tick_start_us() -> u64 {
    TICK_MIRROR
        .with(|mirror| mirror.get())
        .map_or(0, |(_, start)| elapsed_us(start))
}

/// スレッド局所の写しから刻印を組む（**World を借りられない観測点専用**）。
///
/// 一括 flush（`command.rs`）と wndproc の同期経路のためのもの。World を借りられる
/// 観測点がこれを呼ぶのは退行であり、多スレッド実行器のままで刻印を検査する回帰テスト
/// （`transition_diag_tests.rs`）が赤にする。
pub fn stamp() -> Stamp {
    Stamp {
        frame: current_frame(),
        t_us: since_tick_start_us(),
    }
}

/// World 資源から刻印を組む（**World を借りられる観測点はこちら**）。
///
/// 既定の実行器は多スレッドであり、ワーカースレッドからは UI スレッドの写しが見えない。
/// `Res<FrameCount>`＋`Res<TickStart>` から組めば、どのスレッドに載っても同一 tick の
/// 全レコードが同じ `frame` を持つ（D1）。
pub fn stamp_from_world(frame: &FrameCount, start: &TickStart) -> Stamp {
    Stamp {
        frame: frame.0,
        t_us: elapsed_us(start.0),
    }
}

// ---------------------------------------------------------------------------
// flush 区間の時刻基準（RAII）
// ---------------------------------------------------------------------------

thread_local! {
    /// 一括 flush 区間の開始時刻。区間の外では `None`。
    static FLUSH_START: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// 一括 flush 区間の生存札。
///
/// 落ちると起点が**開く前の値へ戻る**ので、最外の札が落ちた時点で [`since_flush_us`] は
/// 必ず `None` を返す——テスト間に起点が残らない（Requirement 7.7）。
///
/// # 区間は入れ子になる（だから復元する）
///
/// 一括 flush は自分の内側でもう一度開かれ得る。`guarded_set_window_pos` の中で
/// `WM_WINDOWPOSCHANGED` が**同期送達**され（`command.rs` の `guarded_set_window_pos` doc・
/// `world/vsync.rs` の再入防止ガードの説明）、その処理の手順③が
/// `flush_window_pos_commands()` を無条件に呼ぶためである
/// （`window_proc/window_pos.rs` の `WM_WINDOWPOSCHANGED` ハンドラ）。`vsync.rs` の
/// `IS_TICK_FLUSH_IN_PROGRESS` はこの直接呼出を塞がない（塞ぐのは再入する tick の側だけ）。
///
/// 内側の解除が起点を `None` へ潰すと、2 本目以降の書込のあいだ [`since_flush_us`] が
/// `None` を返し、「そのメッセージは `SetWindowPos` の内側で受理された」という証跡が
/// 消える。本仕様が追っている機序そのものが測れなくなるため、**内側は自分が上書きした
/// 値を持ち帰り、落ちるときに書き戻す**。
#[must_use = "生存札を落とすと flush 区間が即座に閉じる"]
pub struct FlushEpoch {
    /// この札が開く直前の起点（外側の区間があればその値・無ければ `None`）。
    ///
    /// 私有フィールドなので外部からは構築できない。
    prev: Option<Instant>,
}

impl Drop for FlushEpoch {
    fn drop(&mut self) {
        FLUSH_START.with(|start| start.set(self.prev));
    }
}

/// 一括 flush 区間を開き、生存札を返す。
///
/// 既に区間が開いていれば、その起点は札の中へ退避され、札が落ちるときに戻る
/// （[`FlushEpoch`] の「区間は入れ子になる」を参照）。
pub fn begin_flush() -> FlushEpoch {
    let prev = FLUSH_START.with(|start| start.replace(Some(Instant::now())));
    FlushEpoch { prev }
}

/// flush 開始からの経過（µs）。区間の外では `None`（行では番兵）。
pub fn since_flush_us() -> Option<u64> {
    FLUSH_START.with(|start| start.get()).map(elapsed_us)
}

/// スレッド局所の写しと時刻基準（tick 開始・flush 開始）を初期化する。
///
/// テスト冒頭・末尾で呼ぶための口である（要件 7.7＝テスト間で状態を汚染しない）。
/// 写しはスレッド局所なので並列実行するテスト同士は元より隔離されているが、同一
/// スレッドで続けて走るテストへ前のフレーム番号を持ち越さないために明示的に初期化する。
/// 初期化後は [`current_frame`] が `0`・[`since_tick_start_us`] が `0`・
/// [`since_flush_us`] が `None` になる。
#[doc(hidden)]
pub fn reset_for_test() {
    TICK_MIRROR.with(|mirror| mirror.set(None));
    FLUSH_START.with(|start| start.set(None));
}

#[cfg(test)]
#[path = "transition_diag_tests.rs"]
mod transition_diag_tests;

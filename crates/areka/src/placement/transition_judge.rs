//! 遷移観測ログの**判定器**——観測行の解析・遷移の切り出し・判定量の集計（純関数・I/O 無し）。
//!
//! design.md の C7（`transition_judge`）が正本である。3 crate（wintf・areka-emo-present・areka）が
//! 同一の target・同一の接頭語で出した観測行を 1 本の時系列として読み、`kind=monitor` の
//! 拡大率変化を起点に**遷移**へ切り分け、遷移ごとの判定量（[`TransitionSummary`]）を出す。
//!
//! # 判定語を二重定義しない
//!
//! `kind` 語・`stage` 語・フィールド名は**発行側のレコード純関数が単一の定義元**であり、本
//! モジュールは `pub const` を参照するだけである（リテラルを書かない）。定義元は 3 箇所:
//!
//! - `wintf::ecs::window::transition_diag`（`monitor`／`write`／`flush`／`msg`／`enqueue`）
//! - `areka_emo_present::presenter`（`surface` の平坦な再輸出）
//! - [`super::transition_diag`]（`snapshot`／`hold`／`ground`／`chain`）
//!
//! 語を 1 つでもここで宣言し直すと、発行側が字面を変えたときに判定が静かに空振りする。
//!
//! # 辞書化の規則は `judge-perf.py` と同一
//!
//! 行の本体は `名前=値` の並びであり、値は**次の名前の直前まで**とする
//! （`tools/perf/judge-perf.py:562,588-596` の `parse_fields` と同じ規則）。空白で切らないのは、
//! `Debug` 表現が値に空白を含み得るためである。ただし**同名フィールドは後勝ちにしない**——
//! Python 側は後勝ちで上書きするが、本判定器は「1 行に同じ名前を 2 度出さない」を語彙の
//! 不変条件として扱い、破れた行を [`RecordDefect::DuplicateField`] として記録する
//! （後勝ちで飲み込むと、接頭語の `kind=` が窓種別で潰される退行が判定側から見えなくなる）。
//!
//! # `[transition]` 以外の行は無視する
//!
//! 実機ログは `info` の本文・段階別計時（perf）行・`[diag.window_move]` 行と混ざる。行中に
//! [`RECORD_PREFIX_TAG`] が現れない行は [`parse_transition_line`] が `None` を返して**何も
//! 数えない**。逆に、タグを含む行は壊れていても `Some` を返す——壊れた行を黙って落とすと
//! 「判定語を壊した」ことが 0 件として合格に見えるためである（負例は task 3.2 が使う）。
//!
//! # フレーム番号は差分でしか扱わない（D14）
//!
//! `frame` は `u32` で周回する。本モジュールの比較はすべて `wrapping_sub` の差分であり、
//! 絶対値の大小比較を判定に用いない。加えて、系列の `frame` が**一様に 0** の場合は
//! 「1 フレームで完了した」と読まず [`TransitionSummary::frames_indeterminate`] を立てる——
//! `frame=0` は tick 前の World（`FrameCount` 既定値・`TickStart` 未挿入）が返す縮退値であり、
//! 要件 8.5 に従い「発生 0 回」「有界フレーム数以内」の根拠に用いてはならない。
//!
//! # 本番バイナリには含まれない（`#[cfg(test)]`）
//!
//! 判定器の消費者は決定論テスト（task 3.1／3.2）と実機ログのサインオフランナー
//! （task 3.2 の `#[ignore]` テスト）だけであり、本番の実行経路からは 1 度も呼ばれない。
//! areka は lib ターゲットを持たない bin crate ゆえ `pub` でも dead_code 免除されず、
//! 本番ビルドへ置くと項目ごとに `#[allow(dead_code)]` を貼る羽目になって**以後の真の
//! dead code を隠す**。`placement::test_support` と同じ形（`#[cfg(test)]` のモジュール宣言）に
//! して、許可属性を 1 つも置かずに済ませる。純関数であることは変わらないので、決定論テストと
//! 実機サインオフが**同一実装**を回すという C7 の要件（I/O を持たない単一実装）は満たす。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_present::presenter::{
    KIND_SURFACE, SURFACE_FIELD_REASON, SURFACE_FIELD_TARGET_ID, SURFACE_FIELDS,
    SURFACE_REASON_INVISIBLE, SURFACE_STAGE_SKIPPED, SURFACE_STAGE_VISUALIZE,
};
use wintf::ecs::window::transition_diag::{
    ENQUEUE_FIELDS, FIELD_CALL_US, FIELD_FRAME, FIELD_KIND, FIELD_MERGED_INTO_SEQ, FIELD_NEW_DPI,
    FIELD_OLD_DPI, FIELD_ORIGIN, FIELD_SCOPE, FIELD_SEQ, FIELD_STAGE, FIELD_T_US, FIELD_TOTAL_US,
    FIELD_WIN_KIND, FLUSH_FIELDS, KIND_ENQUEUE, KIND_FLUSH, KIND_MONITOR, KIND_MSG, KIND_WINDPI,
    KIND_WRITE, MISSING, MONITOR_FIELDS, MSG_FIELDS, ORIGIN_DPI_SUGGESTED, RECORD_PREFIX_TAG,
    STAGE_END, STAGE_SYNC, Stamp, WINDPI_FIELDS, WRITE_FIELDS, WRITE_OPTIONAL_FIELDS,
};

use super::diag::{PlacementRoute, WindowKind};
use super::transition_diag::{
    CHAIN_FIELDS, CHAIN_STAGE_REALIGNED, FIELD_DECISION, FIELD_DIFF, GROUND_FIELDS,
    HOLD_DECISION_HOLD, HOLD_FIELDS, KIND_CHAIN, KIND_GROUND, KIND_HOLD, KIND_OFFSET,
    KIND_SNAPSHOT, OFFSET_FIELDS, SNAPSHOT_FIELDS,
};

// ---------------------------------------------------------------------------
// 行の欠陥
// ---------------------------------------------------------------------------

/// 観測行 1 本が語彙の規約を破っている点。
///
/// 「解析できなかった」を `None` へ潰さず値として持つのは、**判定語を壊した入力が沈黙で
/// 合格になる**ことを防ぐためである（要件 7.4 の負例・design「Error Handling」の
/// 「沈黙を PASS にしない」）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecordDefect {
    /// 同じフィールド名が 1 行に 2 度以上現れた（後勝ちで飲み込まない）。
    DuplicateField(String),
    /// 接頭語または当該種別の必須フィールドが無い。
    MissingField(String),
    /// フィールドはあるが値が数として読めない（`frame`／`t_us`）。
    UnreadableField {
        /// フィールド名。
        name: String,
        /// 読めなかった値の字面。
        value: String,
    },
    /// 3 crate のいずれも定義していない `kind` 語。
    UnknownKind(String),
}

// ---------------------------------------------------------------------------
// レコード
// ---------------------------------------------------------------------------

/// 観測行 1 本を種別つきで解析した結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRecord {
    /// 刻印（読めなければ `frame`／`t_us` とも 0・[`RecordDefect`] が立つ）。
    pub stamp: Stamp,
    /// レコード種別語（`kind=` の値）。
    pub kind: String,
    /// 接頭語を含む全フィールド（`名前=値`）。
    fields: BTreeMap<String, String>,
    /// 語彙の規約を破っている点（空なら健全）。
    pub defects: Vec<RecordDefect>,
}

impl TransitionRecord {
    /// フィールドの値（番兵 `-` は「値が無い」として `None`）。
    pub fn field(&self, name: &str) -> Option<&str> {
        self.raw_field(name).filter(|value| *value != MISSING)
    }

    /// フィールドの値（番兵もそのまま返す）。
    pub fn raw_field(&self, name: &str) -> Option<&str> {
        self.fields.get(name).map(String::as_str)
    }

    /// フィールドを数として読む（番兵・未出現・読めない値は `None`）。
    pub fn int_field<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.field(name).and_then(|value| value.parse().ok())
    }

    /// 語彙の規約を 1 つも破っていないか。
    pub fn is_well_formed(&self) -> bool {
        self.defects.is_empty()
    }

    /// `frame` を実際に読めたか（縮退値 0 と「読めなかった 0」を混ぜないための問い）。
    pub fn has_frame_stamp(&self) -> bool {
        !self.defects.iter().any(|defect| match defect {
            RecordDefect::MissingField(name) => name == FIELD_FRAME,
            RecordDefect::UnreadableField { name, .. } => name == FIELD_FRAME,
            _ => false,
        })
    }

    /// この行が指す窓（`scope`＋`win_kind`）。どちらも番兵なら種別語は番兵のまま持つ。
    fn tagged_window(&self) -> WindowKey {
        WindowKey {
            scope: self.int_field(FIELD_SCOPE),
            kind: self.raw_field(FIELD_WIN_KIND).unwrap_or(MISSING).to_owned(),
        }
    }

    /// `kind=surface` 行の `target_id` から写像した窓。
    fn surface_window(&self) -> Option<WindowKey> {
        self.int_field::<u32>(SURFACE_FIELD_TARGET_ID)
            .map(WindowKey::from_target_id)
    }
}

/// 判定量のキーになる窓（スコープ番号＋窓種別語）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WindowKey {
    /// スコープ番号（持たない窓は `None`）。
    pub scope: Option<u32>,
    /// 窓種別語（[`WindowKind::as_str`] の値。判らない書込は番兵のまま）。
    pub kind: String,
}

impl WindowKey {
    /// `target_id` の規則（shell = 2·scope ／ balloon = 2·scope+1）から窓を写像する。
    ///
    /// **偶数側が `char`** である（shell＝キャラ窓）。`surface` レコードは scope も種別も
    /// 運ばない（それは areka の語彙であり emo-present は知らない）ため、判定側が唯一
    /// この写像を持つ。
    ///
    /// 採番の正本は [`crate::emo2_boot::target_map`]（`shell_target`／`balloon_target`）で
    /// あり、こちらはその**逆写像**である（正本は前向きしか持たない）。両者が食い違えば
    /// 判定は窓種別を取り違えたまま静かに緑になるので、往復が一致することは
    /// `transition_judge_tests::summarize_maps_the_surface_target_id_to_a_scope_and_a_kind`
    /// が正本の関数を呼んで固定する。
    pub fn from_target_id(target_id: u32) -> Self {
        let kind = if target_id.is_multiple_of(2) {
            WindowKind::Char
        } else {
            WindowKind::Balloon
        };
        Self {
            scope: Some(target_id / 2),
            kind: kind.as_str().to_owned(),
        }
    }

    /// スコープつきのキャラ窓／バルーン窓のキー。
    pub fn of(scope: u32, kind: WindowKind) -> Self {
        Self {
            scope: Some(scope),
            kind: kind.as_str().to_owned(),
        }
    }

    /// キャラ窓か。
    fn is_char(&self) -> bool {
        self.kind == WindowKind::Char.as_str()
    }
}

impl std::fmt::Display for WindowKey {
    /// 違反の列挙で読める形（フィールド名は発行側の `pub const` を参照する）。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.scope {
            Some(scope) => write!(f, "{FIELD_SCOPE}={scope} {FIELD_WIN_KIND}={}", self.kind),
            None => write!(f, "{FIELD_SCOPE}={MISSING} {FIELD_WIN_KIND}={}", self.kind),
        }
    }
}

// ---------------------------------------------------------------------------
// 遷移の判定量
// ---------------------------------------------------------------------------

/// 遷移の起点（拡大率が変わったモニタ表更新）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionOrigin {
    /// 起点の刻印。
    pub stamp: Stamp,
    /// 変化前の DPI。
    pub old_dpi: u32,
    /// 変化後の DPI。
    pub new_dpi: u32,
}

/// 参考値（実時間）。**判定語ではない**（要件 7.1＝判定は回数・フレーム数・順序・接地点）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WallClock {
    /// 最初の窓書込の `t_us`。
    pub first_write_t_us: Option<u64>,
    /// 最後の窓書込の `t_us`。
    pub last_write_t_us: Option<u64>,
    /// 窓書込 1 回ごとの所要の総和。
    pub sum_call_us: u64,
}

/// 遷移 1 回ぶんの判定量（C7）。
///
/// `judge`（task 3.2）はこの構造体と上限（`Bounds`）だけを見て合否を出す。本構造体は
/// **量を持つだけ**で合否の語を持たない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionSummary {
    /// 起点（先頭が起点レコードでなければ `None`）。
    pub origin: Option<TransitionOrigin>,
    /// 遷移に含まれる観測行の本数。
    pub records: usize,
    /// 語彙の規約を破っていた行の本数。
    pub malformed_records: u32,
    /// フレーム差が判定不能であること（要件 8.5）。
    ///
    /// 立つのは「読めた `frame` が 1 つも 0 以外にならなかった」とき——`frame=0` の縮退値
    /// しか無い系列と、`frame` の値が全部壊れていて 1 つも読めなかった系列の**両方**を含む。
    /// 後者を判定可能としてしまうと、壊れた入力に対して `frames_to_last_write = Some(0)` が
    /// 確定量として通る。
    pub frames_indeterminate: bool,
    /// 起点から最終書込までのフレーム差（`wrapping_sub`・書込が無ければ `None`）。
    pub frames_to_last_write: Option<u32>,
    /// 窓書込の総回数。
    pub writes: u32,
    /// 窓ごとの書込回数（種別語は行の `win_kind=` から読む）。
    pub writes_per_window: BTreeMap<WindowKey, u32>,
    /// 経路 A（OS 提案位置の同期書込）の件数＝`origin=dpi-suggested` の書込。
    pub path_a_writes: u32,
    /// `stage=sync` の書込件数（経路 A の裏取り。`path_a_writes` と食い違えば札か段のどちらかが壊れている）。
    pub sync_stage_writes: u32,
    /// 随伴の同一フレーム性（キャラ窓の書込フレームと、同一 scope のバルーンの**位置**指令が
    /// 着地したフレームが一致するか）。
    ///
    /// 比べる相手が「最後の書込」ではなく `origin=BalloonFollow` の**位置指令**であることが
    /// 要点である——バルーンの寸書込（`KeepPositionResize`）は要件 4.6 が先送りを許す別の
    /// 義務なので、混ぜると先送りが随伴の遅れに化ける
    /// （[`TransitionSummary::balloon_follow_windows`]）。
    pub balloon_same_frame: bool,
    /// 上を実際に検査できたスコープ数（0 なら `balloon_same_frame` は空虚な真）。
    pub balloon_pairs_checked: u32,
    /// 随伴の位置指令（`origin=BalloonFollow`）が 1 度でも**書込へ着地した**窓。
    ///
    /// 判定側が「対を検査できるはずか」を独立に数え直すために持つ。寸だけが届いて位置が
    /// 1 度も来ていない窓は、要件 4.3 を**満たした**のではなく検査できていない。
    ///
    /// 着地は 2 通りで見つかる（[`summarize`] の当該箇所）——⑴ `kind=write` が
    /// `origin=BalloonFollow` を名乗る（畳まれなかった指令）、⑵ `kind=enqueue` の
    /// `merged_into_seq` が指す先の書込（合流された指令。C2 の合流は先着の経路語を残すので、
    /// 畳まれた側の経路語は書込の行から消える）。**指令が積まれただけで書込に届かなかった
    /// 窓はどちらにも入らない**。
    pub balloon_follow_windows: BTreeSet<WindowKey>,
    /// 窓ごとの「可視化フレームと書込フレームの差」（見送り窓は除く）。
    pub mismatch_frames_per_window: BTreeMap<WindowKey, u32>,
    /// 整合待ち（`decision=hold`）の件数。
    pub holds: u32,
    /// 連鎖再解決（`stage=realigned`）の回数。
    pub chain_realigned: u32,
    /// 接地点差の最大（絶対値が最大のものを**符号つき**で持つ。負＝浮き）。
    pub ground_diff_max: Option<i32>,
    /// **不可視ゆえ**再表示を見送られた窓（要件 4.6 で合否から除外する）。
    ///
    /// 見送りの理由は 2 語あるが、除外へ数えるのは
    /// [`areka_emo_present::presenter::SURFACE_REASON_INVISIBLE`] だけである（理由の選り分けは
    /// [`summarize`] の当該箇所に記す）。
    pub skipped_windows: BTreeSet<WindowKey>,
    /// 参考値（実時間）。
    pub wall: WallClock,
    /// **実機サインオフ専用**: 窓ごとの「同一フレームの可視化から当該窓の書込まで」の µs。
    ///
    /// 遷移の区間内にある**全ての**可視化のうち、当該フレームに当該窓の書込がある組を
    /// すべて見て**最大**を採る（最後の 1 組ではない）。遷移は次の起点まで伸びるので、
    /// 後続の表情差替などの可視化で上書きすると遷移自身の最も大きい隔たりが捨てられ、
    /// `Bounds::signoff`（**上限**）の判定を「もう是正は要らない」側へ寄せてしまう。
    pub visualize_to_write_us: BTreeMap<WindowKey, u64>,
    /// **実機サインオフ専用**: 一括書込 1 区間の総所要の最大（`flush stage=end` の `total_us`）。
    pub flush_total_us_max: Option<u64>,
}

impl TransitionSummary {
    /// 何も数えていない状態（起点だけを持つ）。
    fn empty(origin: Option<TransitionOrigin>) -> Self {
        Self {
            origin,
            records: 0,
            malformed_records: 0,
            frames_indeterminate: false,
            frames_to_last_write: None,
            writes: 0,
            writes_per_window: BTreeMap::new(),
            path_a_writes: 0,
            sync_stage_writes: 0,
            balloon_same_frame: true,
            balloon_pairs_checked: 0,
            balloon_follow_windows: BTreeSet::new(),
            mismatch_frames_per_window: BTreeMap::new(),
            holds: 0,
            chain_realigned: 0,
            ground_diff_max: None,
            skipped_windows: BTreeSet::new(),
            wall: WallClock::default(),
            visualize_to_write_us: BTreeMap::new(),
            flush_total_us_max: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 語彙の照合表
// ---------------------------------------------------------------------------

/// 当該種別の**任意**フィールド列（任意フィールドを持たない種別は空）。
///
/// 必須（[`required_fields`]）と分けるのは、**フィールドが後から増えるから**である。
/// `kind=write` の `in_batch`（task 7.2 のバッチ化と同時に増えた）を必須にすると、
/// それ以前に採取した実機ログの書込行が軒並み `MissingField` として
/// `malformed_records` に載り、確定済みの証跡（`mechanism-ledger.md` §3・§10）を
/// 判定器が後から否定してしまう。読む側は欠けていても受け入れ、**書く側が必ず載せる**
/// ことは発行側の逐語テスト（`transition_diag_tests::write_line_is_verbatim`）が固定する。
///
/// 一覧はすべて発行側の `pub const` であり、ここは**並べるだけ**である。
pub(crate) fn optional_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        KIND_WRITE => WRITE_OPTIONAL_FIELDS,
        _ => &[],
    }
}

/// 当該種別の必須フィールド列（未知の種別語は `None`）。
///
/// 一覧はすべて発行側の `pub const` であり、ここは**並べるだけ**である。
/// 任意フィールドは [`optional_fields`] が別に持つ（欠けていても壊れた行にしない）。
fn required_fields(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        KIND_MONITOR => Some(MONITOR_FIELDS),
        // task 8.3: 窓の拡大率が変わったことを表す起点。**語彙を教えるだけ**であり、
        // 起点集合を広げるのは task 8.4 の担当（`TransitionOrigin::of` は無改変）。
        KIND_WINDPI => Some(WINDPI_FIELDS),
        KIND_WRITE => Some(WRITE_FIELDS),
        KIND_FLUSH => Some(FLUSH_FIELDS),
        KIND_MSG => Some(MSG_FIELDS),
        KIND_ENQUEUE => Some(ENQUEUE_FIELDS),
        KIND_SURFACE => Some(SURFACE_FIELDS),
        KIND_SNAPSHOT => Some(SNAPSHOT_FIELDS),
        KIND_HOLD => Some(HOLD_FIELDS),
        KIND_GROUND => Some(GROUND_FIELDS),
        KIND_CHAIN => Some(CHAIN_FIELDS),
        // task 5.2: 追随の種別。判定は専用モジュールが持つため、ここは語彙を教えるだけ。
        KIND_OFFSET => Some(OFFSET_FIELDS),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 行の解析
// ---------------------------------------------------------------------------

/// `名前=` の出現 1 件（`judge-perf.py::parse_fields` の正規表現に相当）。
struct FieldKey {
    /// 名前の開始位置（バイト）。
    name_start: usize,
    /// 名前の終端＝`=` の位置（バイト）。
    name_end: usize,
}

/// `名前=` の出現位置を左から並べる。
///
/// 名前は行頭または空白の直後から始まる `[A-Za-z_][A-Za-z0-9_]*` で、直後が `=` のものだけ。
/// 名前と `=` はいずれも ASCII なので、返す位置は必ず文字境界に落ちる。
fn scan_field_keys(tail: &str) -> Vec<FieldKey> {
    let bytes = tail.as_bytes();
    let mut keys = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let at_boundary = index == 0 || bytes[index - 1].is_ascii_whitespace();
        if !at_boundary || !(bytes[index].is_ascii_alphabetic() || bytes[index] == b'_') {
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == b'=' {
            keys.push(FieldKey {
                name_start: index,
                name_end: end,
            });
            index = end + 1;
        } else {
            index = end;
        }
    }
    keys
}

/// 観測行 1 本を解析する。
///
/// 戻り値が `None` になるのは行中に [`RECORD_PREFIX_TAG`] が無いときだけ（＝観測チャネル外の
/// 行）。タグを含む行は壊れていても `Some` を返し、破れた規約は
/// [`TransitionRecord::defects`] に載る。
pub fn parse_transition_line(line: &str) -> Option<TransitionRecord> {
    let tag_at = line.find(RECORD_PREFIX_TAG)?;
    let tail = &line[tag_at + RECORD_PREFIX_TAG.len()..];

    let keys = scan_field_keys(tail);
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut defects: Vec<RecordDefect> = Vec::new();
    for (position, key) in keys.iter().enumerate() {
        let name = &tail[key.name_start..key.name_end];
        let value_end = keys
            .get(position + 1)
            .map_or(tail.len(), |next| next.name_start);
        let value = tail[key.name_end + 1..value_end].trim();
        if fields.contains_key(name) {
            defects.push(RecordDefect::DuplicateField(name.to_owned()));
            continue;
        }
        fields.insert(name.to_owned(), value.to_owned());
    }

    let stamp = Stamp {
        frame: read_prefix_number(&fields, FIELD_FRAME, &mut defects),
        t_us: read_prefix_number(&fields, FIELD_T_US, &mut defects),
    };
    let kind = match fields.get(FIELD_KIND) {
        Some(kind) => kind.clone(),
        None => {
            defects.push(RecordDefect::MissingField(FIELD_KIND.to_owned()));
            String::new()
        }
    };

    match required_fields(&kind) {
        Some(required) => {
            for name in required {
                if !fields.contains_key(*name) {
                    defects.push(RecordDefect::MissingField((*name).to_owned()));
                }
            }
        }
        None => {
            if fields.contains_key(FIELD_KIND) {
                defects.push(RecordDefect::UnknownKind(kind.clone()));
            }
        }
    }

    Some(TransitionRecord {
        stamp,
        kind,
        fields,
        defects,
    })
}

/// 接頭語の数値フィールドを読む（欠落・読めない値は欠陥として記録し 0 を返す）。
fn read_prefix_number<T: std::str::FromStr + Default>(
    fields: &BTreeMap<String, String>,
    name: &'static str,
    defects: &mut Vec<RecordDefect>,
) -> T {
    match fields.get(name) {
        None => {
            defects.push(RecordDefect::MissingField(name.to_owned()));
            T::default()
        }
        Some(value) => match value.parse() {
            Ok(parsed) => parsed,
            Err(_) => {
                defects.push(RecordDefect::UnreadableField {
                    name: name.to_owned(),
                    value: value.clone(),
                });
                T::default()
            }
        },
    }
}

// ---------------------------------------------------------------------------
// 遷移の切り出し
// ---------------------------------------------------------------------------

/// この行が遷移の起点か（`kind=monitor` かつ拡大率が変わっている）。
///
/// 作業領域だけが変わったモニタ表更新は起点にしない——起点にすると遷移が水増しされ、
/// 「各方向 3 回以上」の充足判定（要件 8.2）が実態より甘くなる。
pub fn is_transition_origin(record: &TransitionRecord) -> bool {
    TransitionOrigin::of(record).is_some()
}

impl TransitionOrigin {
    /// 起点レコードなら起点情報を返す。
    fn of(record: &TransitionRecord) -> Option<Self> {
        if record.kind != KIND_MONITOR {
            return None;
        }
        let old_dpi: u32 = record.int_field(FIELD_OLD_DPI)?;
        let new_dpi: u32 = record.int_field(FIELD_NEW_DPI)?;
        (old_dpi != new_dpi).then_some(Self {
            stamp: record.stamp,
            old_dpi,
            new_dpi,
        })
    }
}

/// 起点ごとに遷移を切り出す（起点から次の起点の直前まで）。
///
/// 最初の起点より前の行は**どの遷移にも属さない**ので捨てる（起動時の書込を遷移へ数え
/// 込まないため）。
pub fn split_transitions(records: &[TransitionRecord]) -> Vec<Vec<&TransitionRecord>> {
    let mut transitions: Vec<Vec<&TransitionRecord>> = Vec::new();
    for record in records {
        if is_transition_origin(record) {
            transitions.push(vec![record]);
        } else if let Some(current) = transitions.last_mut() {
            current.push(record);
        }
    }
    transitions
}

// ---------------------------------------------------------------------------
// 判定量の集計
// ---------------------------------------------------------------------------

/// 遷移 1 回ぶんの判定量を集計する。
///
/// 入力は [`split_transitions`] が返す 1 区間（先頭が起点）。フレーム差はすべて
/// `wrapping_sub`（D14）で取り、絶対値の大小比較は 1 箇所も行わない。
pub fn summarize(transition: &[&TransitionRecord]) -> TransitionSummary {
    let origin = transition
        .first()
        .and_then(|record| TransitionOrigin::of(record));
    let mut summary = TransitionSummary::empty(origin);
    summary.records = transition.len();

    // 見送り窓は以後の除外に使うので先に集める。
    //
    // # 除外に使えるのは `reason=invisible` **だけ**である
    //
    // 発行側の見送りには理由が 2 語ある（`areka-emo-present` の `refresh_scale`）:
    //
    // - `invisible`——対象が不可視（Hide／全透明退化）。**要件 4.6 が意図した除外**である。
    //   遷移時点で見えていない窓は「位置と寸を変更せずに現状を維持」が正しい振舞いであり、
    //   後から表示された時点の書込はその遷移の続きではない。
    // - `k-unchanged`——再導出した拡大率が前回適用値と等しい。これは**定常状態の空振り**で
    //   あって現状維持の対象ではない。遷移が落ち着いた次フレームから、拡大率が変わらない
    //   限り全窓が毎フレーム出し続ける。
    //
    // 遷移の区間は**次の起点まで伸びる**（[`split_transitions`]）ので、`k-unchanged` を除外の
    // 理由にすると遷移ごとに全窓が除外側へ入り、[`TransitionSummary::frames_to_last_write`]・
    // [`TransitionSummary::mismatch_frames_per_window`]・
    // [`TransitionSummary::visualize_to_write_us`] の 3 つが**静かに空になる**。task 4.2 の実機
    // ログ（12 遷移・`k-unchanged` 49 件／`invisible` 4 件）が実際にそうなり、可視化から書込
    // まで 20 万µs 級の隔たりが 1 件も違反として立たなかった。
    for record in transition {
        if record.kind == KIND_SURFACE
            && record.raw_field(FIELD_STAGE) == Some(SURFACE_STAGE_SKIPPED)
            && record.raw_field(SURFACE_FIELD_REASON) == Some(SURFACE_REASON_INVISIBLE)
            && let Some(window) = record.surface_window()
        {
            summary.skipped_windows.insert(window);
        }
    }

    let mut last_write_frame: Option<u32> = None;
    let mut last_write_frame_per_window: BTreeMap<WindowKey, u32> = BTreeMap::new();
    // 窓ごとの「随伴の位置指令（`origin=BalloonFollow`）が着地した書込」——観測順で最後の
    // 1 件（書込の観測位置とそのフレーム）。
    //
    // バルーン窓の 2 本の指令は**別の義務**を果たす——位置（`BalloonFollow`）は要件 4.3 が
    // キャラ窓と同一フレームで求め、寸（`KeepPositionResize`）は要件 4.6 が先送りを許す。
    // 経路語で選り分けずに「最後の書込」で測ると、見送られた寸が後のフレームで届いただけの
    // 遷移（再観測 §3.2 の遷移 2＝レポートが「欠陥ではない」と明記した形）が随伴の遅れに
    // 化ける。
    let mut follow_landing_per_window: BTreeMap<WindowKey, (usize, u32)> = BTreeMap::new();
    // 合流された随伴の指令（窓・合流先の通し番号・当該 `enqueue` の観測位置）。
    //
    // 合流（C2）を通ると後着の指令は**先着の枠へ畳まれ**、合流後の 1 本は先着の経路語を
    // 名乗る（`command.rs` の `merge_into`＝「タグは先着の値を保つ」）。バルーン窓は寸の
    // 指令が先に積まれるので、随伴の位置指令の経路語は `kind=write` から消える。ここを
    // 数えないと、合流が効いている遷移では要件 4.3 が**未測定**として落ちる（task 6.6・
    // 2026-08-21 の実機再採取で 7 遷移中 5 遷移がこの形だった）。
    let mut merged_follow_commands: Vec<(WindowKey, u32, usize)> = Vec::new();
    // 観測順の窓書込（観測位置・窓・通し番号・フレーム）。合流先の引き当てに使う。
    let mut writes_in_order: Vec<(usize, WindowKey, Option<u32>, u32)> = Vec::new();
    // 窓ごと・フレームごとの「最後の書込 t_us」（実機専用量 visualize_to_write_us 用）。
    let mut last_write_us_per_window_frame: BTreeMap<(WindowKey, u32), u64> = BTreeMap::new();
    let mut last_visualize_per_window: BTreeMap<WindowKey, (u32, u64)> = BTreeMap::new();
    // 窓ごとの**全ての**可視化（実機専用量は最大を採るので上書きしない）。
    let mut visualizes_per_window: BTreeMap<WindowKey, Vec<(u32, u64)>> = BTreeMap::new();
    let mut nonzero_frames_seen = 0_u32;

    for (index, record) in transition.iter().enumerate() {
        if !record.is_well_formed() {
            summary.malformed_records += 1;
        }
        if record.has_frame_stamp() && record.stamp.frame != 0 {
            nonzero_frames_seen += 1;
        }
        match record.kind.as_str() {
            KIND_WRITE => {
                let window = record.tagged_window();
                summary.writes += 1;
                *summary.writes_per_window.entry(window.clone()).or_insert(0) += 1;
                if record.raw_field(FIELD_ORIGIN) == Some(ORIGIN_DPI_SUGGESTED) {
                    summary.path_a_writes += 1;
                }
                if record.raw_field(FIELD_STAGE) == Some(STAGE_SYNC) {
                    summary.sync_stage_writes += 1;
                }
                writes_in_order.push((
                    index,
                    window.clone(),
                    record.int_field::<u32>(FIELD_SEQ),
                    record.stamp.frame,
                ));
                if record.raw_field(FIELD_ORIGIN) == Some(PlacementRoute::BalloonFollow.as_str()) {
                    summary.balloon_follow_windows.insert(window.clone());
                    note_follow_landing(
                        &mut follow_landing_per_window,
                        window.clone(),
                        index,
                        record.stamp.frame,
                    );
                }
                // 遷移の所要（要件 4.4 の「全ゴースト窓の遷移」）には**見送り窓への書込を
                // 数えない**。要件 4.6 は当該窓を「位置と寸を変更せずに現状を維持」させる
                // 規定であり、後から表示された時点の書込はその遷移の続きではない。事例は
                // 再観測 §3.2 の遷移 2 の +660ms で、レポートはその窓を
                // 「遷移時点で不可視（`refresh_scale: 不可視ゆえ再表示しない`）だったため
                // 見送られ」と書く＝**`reason=invisible` の側**である（`k-unchanged` では
                // ない。だからこそ集計の入口で理由を選り分けている）。数えると、レポートが
                // 非欠陥と明記した遷移がフレーム上限で不合格になる。見送り窓でも**随伴の
                // 位置**は `balloon_same_frame` が見張り続けるので、抜け穴にはならない。
                if !summary.skipped_windows.contains(&window) {
                    last_write_frame = Some(record.stamp.frame);
                }
                last_write_frame_per_window.insert(window.clone(), record.stamp.frame);
                last_write_us_per_window_frame
                    .insert((window, record.stamp.frame), record.stamp.t_us);
                summary
                    .wall
                    .first_write_t_us
                    .get_or_insert(record.stamp.t_us);
                summary.wall.last_write_t_us = Some(record.stamp.t_us);
                summary.wall.sum_call_us = summary
                    .wall
                    .sum_call_us
                    .saturating_add(record.int_field(FIELD_CALL_US).unwrap_or(0));
            }
            KIND_ENQUEUE => {
                // 畳まれた随伴の指令だけを控える（畳まれていない指令は、その枠がそのまま
                // 書かれるので `kind=write` の側に `origin=BalloonFollow` が残る）。
                if record.raw_field(FIELD_ORIGIN) == Some(PlacementRoute::BalloonFollow.as_str())
                    && let Some(seq) = record.int_field::<u32>(FIELD_MERGED_INTO_SEQ)
                {
                    merged_follow_commands.push((record.tagged_window(), seq, index));
                }
            }
            KIND_SURFACE => {
                if record.raw_field(FIELD_STAGE) == Some(SURFACE_STAGE_VISUALIZE)
                    && let Some(window) = record.surface_window()
                {
                    last_visualize_per_window
                        .insert(window.clone(), (record.stamp.frame, record.stamp.t_us));
                    visualizes_per_window
                        .entry(window)
                        .or_default()
                        .push((record.stamp.frame, record.stamp.t_us));
                }
            }
            KIND_HOLD => {
                if record.raw_field(FIELD_DECISION) == Some(HOLD_DECISION_HOLD) {
                    summary.holds += 1;
                }
            }
            KIND_CHAIN => {
                if record.raw_field(FIELD_STAGE) == Some(CHAIN_STAGE_REALIGNED) {
                    summary.chain_realigned += 1;
                }
            }
            KIND_GROUND => {
                if let Some(diff) = record.int_field::<i32>(FIELD_DIFF) {
                    summary.ground_diff_max = Some(match summary.ground_diff_max {
                        Some(current) if current.saturating_abs() >= diff.saturating_abs() => {
                            current
                        }
                        _ => diff,
                    });
                }
            }
            KIND_FLUSH => {
                if record.raw_field(FIELD_STAGE) == Some(STAGE_END)
                    && let Some(total) = record.int_field::<u64>(FIELD_TOTAL_US)
                {
                    summary.flush_total_us_max = Some(
                        summary
                            .flush_total_us_max
                            .map_or(total, |max| max.max(total)),
                    );
                }
            }
            _ => {}
        }
    }

    // 合流された随伴の指令を、その**合流先の書込**へ引き当てる。
    //
    // `merged_into_seq` は先着の枠のキュー内の位置であり、一括書込の `write` の `seq` と
    // 同じ数え方である（`command.rs` の `coalesce_geometry` と `flush`）。ゆえに「同じ窓の・
    // その通し番号の・当該 `enqueue` より後に現れる最初の書込」がその指令の着地である。
    //
    // 3 つの条件はどれも外せない:
    //
    // - **窓**——`seq` は同一 tick のキュー全体の位置なので、番号だけで引き当てると別の窓の
    //   書込を随伴の着地と取り違える。
    // - **観測位置**——`seq` は一括書込ごとに 0 から振り直されるため、番号だけでは当該指令が
    //   積まれる**前**の一括書込を掴み、遅れた随伴が定刻に見える。
    // - **書込の存在**——引き当てられなければ数に入れない。指令が積まれただけで一括書込に
    //   届かなかった随伴を「追従した」と数えると、書かれていない窓が合格の根拠になる。
    for (window, seq, enqueue_index) in &merged_follow_commands {
        if let Some((write_index, _, _, frame)) =
            writes_in_order
                .iter()
                .find(|(write_index, write_window, write_seq, _)| {
                    write_index > enqueue_index
                        && write_window == window
                        && *write_seq == Some(*seq)
                })
        {
            summary.balloon_follow_windows.insert(window.clone());
            note_follow_landing(
                &mut follow_landing_per_window,
                window.clone(),
                *write_index,
                *frame,
            );
        }
    }

    // 「読めたフレームが 1 つも無い」も「一様に 0」と同じく判定不能である。前者を
    // `false`（＝判定可能）にすると、`frame` が全部壊れている系列で
    // `frames_to_last_write = Some(0)` が確定量として通ってしまう。
    summary.frames_indeterminate = !transition.is_empty() && nonzero_frames_seen == 0;
    if let (Some(origin), Some(last)) = (summary.origin, last_write_frame) {
        summary.frames_to_last_write = Some(last.wrapping_sub(origin.stamp.frame));
    }

    // 可視化と書込の食い違い（見送り窓は要件 4.6 で除外する）。フレーム差は**最後の**
    // 可視化と**最後の**書込＝着地どうしの比較で採る（4.3 の「遷移後の寸法・位置で
    // 可視化されるとき」が問うのは着地のフレームだから）。
    for (window, (visualize_frame, _)) in &last_visualize_per_window {
        if summary.skipped_windows.contains(window) {
            continue;
        }
        if let Some(write_frame) = last_write_frame_per_window.get(window) {
            summary
                .mismatch_frames_per_window
                .insert(window.clone(), write_frame.wrapping_sub(*visualize_frame));
        }
    }

    // 実機専用量は同じ窓の**全ての**可視化を見て最大の隔たりを採る。遷移の区間は次の
    // 起点まで伸びるので、後続の表情差替などの可視化で上書きすると、その遷移自身の
    // （最も大きい）隔たりが捨てられる——上限判定に使う量を小さく見積もる側に倒すのは
    // 「是正はもう要らない」という誤答へ寄せる方向であり、採ってはならない。
    for (window, visualizes) in &visualizes_per_window {
        if summary.skipped_windows.contains(window) {
            continue;
        }
        let widest = visualizes
            .iter()
            .filter_map(|(frame, visualize_us)| {
                // 同一フレームに当該窓の書込がある可視化だけが対象（C7）。
                last_write_us_per_window_frame
                    .get(&(window.clone(), *frame))
                    .map(|write_us| write_us.saturating_sub(*visualize_us))
            })
            .max();
        if let Some(widest) = widest {
            summary.visualize_to_write_us.insert(window.clone(), widest);
        }
    }

    // 随伴の同一フレーム性（キャラ窓の書込フレームと、同一 scope のバルーン窓の**位置**書込
    // フレーム）。
    //
    // 除外はキャラ窓側だけである。要件 4.3 の When 節は「キャラ窓が遷移後の寸法・位置で
    // **可視化されるとき**」であり、キャラ窓の再表示が見送られていれば引き金そのものが
    // 立たない。逆に**バルーン側の見送りでは降りない**——見送りは「サーフェスの再表示を
    // しなかった」という事実であって、随伴の窓書込（`BalloonFollow`）は不可視のバルーンにも
    // 出る（再観測 §3.1 の実ログがそう）。実機サインオフの手順（発話させず拡大率だけ変える）
    // ではバルーンはほぼ常に不可視＝`stage=skipped reason=invisible` なので、ここで降りると
    // 要件 4.3 が**定常の形で 1 度も検査されない**まま合格になる。
    //
    // 比べるのは `follow_landing_per_window`＝**位置の指令が着地した書込だけ**であって
    // 「最後の書込」ではない。バルーンの寸書込（`KeepPositionResize`）は要件 4.6 が先送りを
    // 許す別の義務であり、混ぜると再観測 §3.2 の遷移 2（位置は定刻・寸だけ +660ms）が
    // 随伴の遅れに化ける。
    for (window, char_frame) in &last_write_frame_per_window {
        if !window.is_char() || summary.skipped_windows.contains(window) {
            continue;
        }
        let Some(scope) = window.scope else { continue };
        let balloon = WindowKey::of(scope, WindowKind::Balloon);
        if let Some((_, balloon_frame)) = follow_landing_per_window.get(&balloon) {
            summary.balloon_pairs_checked += 1;
            if balloon_frame != char_frame {
                summary.balloon_same_frame = false;
            }
        }
    }

    summary
}

/// 随伴の着地を「観測順で最後の 1 件」として控える。
///
/// 直の書込（`origin=BalloonFollow`）と合流先から引き当てた書込が同じ窓に混ざり得るので、
/// どちらであるかではなく**書込レコードの観測位置**で新旧を決める（引き当ては本文の走査の
/// 後に行うため、到着順では新旧を決められない）。
fn note_follow_landing(
    landings: &mut BTreeMap<WindowKey, (usize, u32)>,
    window: WindowKey,
    write_index: usize,
    frame: u32,
) {
    match landings.get(&window) {
        Some((current_index, _)) if *current_index >= write_index => {}
        _ => {
            landings.insert(window, (write_index, frame));
        }
    }
}

/// ログ全文を行へ分け、観測行だけを解析する。
///
/// `[transition]` を含まない行（`info` 本文・段階別計時行・`[diag.window_move]` 行）は
/// 1 件も数えない。
pub fn parse_transition_log(log: &str) -> Vec<TransitionRecord> {
    log.lines().filter_map(parse_transition_line).collect()
}

/// 上限と合否（判定量の集計とは変更の理由が異なるので分けてある）。
#[path = "transition_judge_verdict.rs"]
mod verdict;

pub use verdict::{
    Bounds, CHAIN_REALIGN_PER_TRANSITION, FLUSH_TOTAL_US_MAX, GROUND_DIFF_MAX,
    HOLD_FRAME_ALLOWANCE, PATH_A_WRITES_MAX, Quantity, Report, TRANSITION_FRAME_BOUND,
    TRANSITION_LOG_ENV, TransitionVerdict, VISUALIZE_TO_WRITE_US_MAX, Violation,
    WRITES_PER_WINDOW_MAX, judge, judge_transition_log,
};

#[cfg(test)]
#[path = "transition_judge_test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "transition_judge_tests.rs"]
mod transition_judge_tests;

#[cfg(test)]
#[path = "transition_judge_frame_tests.rs"]
mod transition_judge_frame_tests;

#[cfg(test)]
#[path = "transition_judge_reobservation_tests.rs"]
mod transition_judge_reobservation_tests;

#[cfg(test)]
#[path = "transition_judge_coalesced_follow_tests.rs"]
mod transition_judge_coalesced_follow_tests;

#[cfg(test)]
#[path = "transition_judge_verdict_tests.rs"]
mod transition_judge_verdict_tests;

#[cfg(test)]
#[path = "transition_judge_negative_tests.rs"]
mod transition_judge_negative_tests;

#[cfg(test)]
#[path = "transition_signoff_tests.rs"]
mod transition_signoff_tests;

#[cfg(test)]
#[path = "transition_signoff_procedure_tests.rs"]
mod transition_signoff_procedure_tests;

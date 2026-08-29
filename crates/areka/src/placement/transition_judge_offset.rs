//! 追随レコード（`kind=offset`）の**機械判定器**——遷移ごとの切り出しと 4 つの判定
//! （純関数・I/O 無し・`#[cfg(test)]`）。
//!
//! design.md「供給層・観測・判定（要約）」の `transition_judge_offset` が正本である。
//! [`super::transition_judge`] が解析した観測行の時系列を受け取り、`kind=monitor`／
//! `kind=windpi` の拡大率変化を起点に切り出した遷移（起点規約は
//! [`super::transition_judge::split_transitions`]——同じ変化を指す起点は 1 本へ畳む）ごとに
//! `kind=offset` 行を集計して、次の 4 点を判定する。
//!
//! | # | 要件 | 判定 |
//! |---|---|---|
//! | ⑴ | 8.2 | 往復の前後で反映後の値（`new_offset`）が **bit 同一**に戻ること |
//! | ⑵ | 8.3 | 遷移ごとに判定語が**期待の腕**であること |
//! | ⑶ | 8.4 | **低い拡大率側**で追随の判定語が出ていること |
//! | ⑷ | 8.5 | キーワード指定スコープの**揃えの残差**が許容量以内であること |
//!
//! # 判定語のリテラルを書かない
//!
//! 6 つの判定語も欄名も、発行側（[`super::transition_diag`]）の `pub const` を参照するだけ
//! である。字面を 1 つでもここで書き直すと、発行側が語を変えたときに判定が静かに空振りする
//! （[`super::transition_judge`] の module doc と同じ規律）。
//!
//! # ⑴ を判定語の並びで見ない（**この判定器の核**）
//!
//! 往復を「判定語の並び」で見分けてはならない。恒等比の腕は**現在値を据え置くのではなく
//! 基準から引き直す**（design D4・`balloon_offset_follow.rs` の `OffsetRescale::Unchanged`
//! 腕）ため、往復は `rescaled → rescaled` を出す——`rescaled → unchanged` ではない。加えて
//! `rescaled` は「値が動いた」を意味しない（丸めで等しく留まり得る）。よって ⑴ は
//! **値の欄だけ**を鍵にする: 同じ基準対（`base_offset`＋`base_dpi`）が生きているあいだ、
//! 同じ `new_dpi` の行は必ず bit 同一の `new_offset` を持つ。基準が確立し直された
//! （ドラッグ・面切替）ところで区間が切れるので、利用者の操作を往復の破れと読み違えない。
//!
//! # `new_dpi=0` から腕を推し量らない
//!
//! `keyword-pending` は表示 DPI を読む**前**の門で抜けるので `new_dpi=0` を運ぶ
//! （`balloon_offset_follow.rs` の門）。しかし縮退の `unresolved` も `DPI` component が
//! 無い腕では `new_dpi=0` を運ぶ。**腕は `verdict` だけで見分ける**——`new_dpi` から
//! 推し量ると 2 つの腕が混ざる。加えて ⑴ と ⑷ は `new_dpi=0` の行から**値を読まない**
//! （遷移後の表示 DPI がその行には無い）——⑷ が門の行を読むのは、母数となるスコープを
//! 数えるときだけである（[`keyword_pending_scopes`]・task 8.6）。
//!
//! # 受容された残余を「壊れている」と読まない
//!
//! 素材未消費のまま寸据え置きの遷移を迎えた腕（design D7・2026-08-27 の開発者裁定）は
//! `verdict=keyword-pending` かつ前後の値が bit 同一という形で現れる。これは**正しい記録**
//! であって違反ではない。ゆえに ⑶ は、キーワード指定スコープについては**素材が消費された
//! 後の遷移**だけを数える（消費前の遷移を数えると偽の赤が出る）。
//!
//! # 沈黙を合格にしない
//!
//! 「違反 0 件」は「判定できなかった」を含んではならない。追随レコードが 1 行も無い・往復が
//! 1 度も観測されていない・低い拡大率側の遷移が 1 本も無い・キーワード指定スコープの揃えを
//! 1 度も測れていない——いずれも**違反**として立てる（要件 8.3 の「合否は記録の機械判定で
//! 決める」は、材料が無いことを合格と読まないことを含む）。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use wintf::ecs::window::transition_diag::{FIELD_NEW_DPI, FIELD_OLD_DPI, FIELD_SCOPE, MISSING};

use super::transition_diag::{
    FIELD_BASE_DPI, FIELD_BASE_OFFSET, FIELD_NEW_OFFSET, FIELD_OLD_OFFSET, FIELD_VERDICT,
    KIND_OFFSET, OFFSET_VERDICT_ALL, OFFSET_VERDICT_ANCHORED, OFFSET_VERDICT_KEYWORD_PENDING,
    OFFSET_VERDICT_RESCALED, OFFSET_VERDICT_SATURATED, OFFSET_VERDICT_UNCHANGED,
    OFFSET_VERDICT_UNRESOLVED,
};
use super::transition_judge::{
    TransitionRecord, is_transition_origin, parse_transition_log, split_transitions,
};

/// 揃えの残差の許容量（1 軸あたりの px・design D8）。
///
/// 決定論の全数列挙（`follow_offset_residual_tests.rs`）が実測した最大は 2px であるが、
/// 合否の基準に採るのは**契約の上限である 3px** の側である——実測値を基準にすると、
/// 契約の内側での悪化（2px → 3px）が実機の合否を落とすことになり、契約が定めていない
/// 厳しさを持ち込む。上限内での悪化の監視は決定論側（実値を逐語で固定する同ファイル）の
/// 仕事であり、実機ログの機械判定の仕事ではない。
pub const ALIGNMENT_RESIDUAL_MAX_PX: i64 = 3;

/// 値を 1 bit も動かしてはならない判定語（[`OFFSET_VERDICT_ALL`] の残り 2 語＝
/// `rescaled`／`saturated` は動かしてよい）。
const STABLE_VERDICTS: &[&str] = &[
    OFFSET_VERDICT_ANCHORED,
    OFFSET_VERDICT_UNCHANGED,
    OFFSET_VERDICT_KEYWORD_PENDING,
    OFFSET_VERDICT_UNRESOLVED,
];

// ---------------------------------------------------------------------------
// 切り出した記録
// ---------------------------------------------------------------------------

/// 追随レコード 1 行（欄はすべて発行側の `OffsetRecord` と 1 対 1）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffsetRow {
    /// この行が属する遷移の番号（0 起点・[`split_transitions`] の順）。
    pub transition: usize,
    /// スコープ番号（番兵なら `None`）。
    pub scope: Option<u32>,
    /// 基準対が属する表示 DPI（未係留なら `None`）。
    pub base_dpi: Option<u32>,
    /// 遷移後の表示 DPI（門・縮退の腕では `0`）。
    pub new_dpi: u32,
    /// 基準対の値。
    pub base_offset: (i32, i32),
    /// 追随前の値。
    pub old_offset: (i32, i32),
    /// 追随後の値。
    pub new_offset: (i32, i32),
    /// 判定語。
    pub verdict: String,
}

/// 遷移 1 回ぶんの追随レコード。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffsetTransition {
    /// 遷移の番号（0 起点）。
    pub index: usize,
    /// 起点の変化前 DPI。
    pub old_dpi: u32,
    /// 起点の変化後 DPI。
    pub new_dpi: u32,
    /// 当該遷移に現れた追随レコード。
    pub rows: Vec<OffsetRow>,
}

impl OffsetTransition {
    /// 低い拡大率の側へ移った遷移か（要件 8.4 の「低い拡大率側」）。
    fn to_lower_scale(&self) -> bool {
        self.new_dpi < self.old_dpi
    }
}

// ---------------------------------------------------------------------------
// 違反
// ---------------------------------------------------------------------------

/// 判定が立てた違反（行に紐づくものはすべて遷移番号とスコープを名指しする・要件 8.3）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OffsetViolation {
    /// 追随レコードが 1 行も無い。
    NoOffsetRecords,
    /// 語彙の規約を破った追随レコードがある。
    MalformedRecord {
        /// 遷移番号。
        transition: usize,
        /// 何が読めなかったか。
        detail: String,
    },
    /// 語彙表に無い判定語。
    UnknownVerdict {
        /// 遷移番号。
        transition: usize,
        /// スコープ。
        scope: Option<u32>,
        /// 読めた字面。
        verdict: String,
    },
    /// 判定語が期待の腕でない（要件 8.3）。
    UnexpectedVerdict {
        /// 遷移番号。
        transition: usize,
        /// スコープ。
        scope: Option<u32>,
        /// 実際の判定語。
        verdict: String,
        /// 期待した腕。
        expected: &'static [&'static str],
    },
    /// 値を動かしてはならない腕で値が動いた。
    ValueMoved {
        /// 遷移番号。
        transition: usize,
        /// スコープ。
        scope: Option<u32>,
        /// 判定語。
        verdict: String,
        /// 追随前。
        old_offset: (i32, i32),
        /// 追随後。
        new_offset: (i32, i32),
    },
    /// 門の判定語が遷移後の表示 DPI を運んでいる（門は `DPI` 読取より前＝`new_dpi=0` のはず）。
    KeywordPendingCarriesDisplayDpi {
        /// 遷移番号。
        transition: usize,
        /// スコープ。
        scope: Option<u32>,
        /// 運んでいた値。
        new_dpi: u32,
    },
    /// 往復して同じ表示 DPI へ戻ったのに値が bit 同一でない（要件 8.2）。
    RoundTripDrift {
        /// スコープ。
        scope: Option<u32>,
        /// 戻ってきた表示 DPI。
        dpi: u32,
        /// 最初に観測した遷移番号。
        first_transition: usize,
        /// 最初の値。
        first: (i32, i32),
        /// 戻ってきた遷移番号。
        again_transition: usize,
        /// 戻ってきたときの値。
        again: (i32, i32),
    },
    /// 往復が 1 度も観測されていない（判定できていないことを合格と読まない）。
    NoRoundTripObserved,
    /// 低い拡大率側で追随の判定語が 1 度も出ていない（要件 8.4）。
    NoLowScaleRescale {
        /// 低い拡大率側へ移った遷移の本数（0 なら往復そのものが行われていない）。
        low_side_transitions: usize,
    },
    /// 揃えの残差が許容量を超えた（要件 8.5・design D8）。
    AlignmentResidual {
        /// 遷移番号。
        transition: usize,
        /// スコープ。
        scope: Option<u32>,
        /// 軸。
        axis: &'static str,
        /// 残差（px の 1/100 単位・切り捨て）。
        residual_hundredths: i64,
        /// 許容量（px）。
        max_px: i64,
    },
    /// キーワード指定スコープの揃えを 1 度も測れていない（要件 8.5）。
    NoKeywordAlignmentMeasured {
        /// 門の判定語を 1 度でも出したスコープの数（0 なら点灯そのものが無い）。
        keyword_scopes: usize,
    },
}

impl fmt::Display for OffsetViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOffsetRecords => write!(
                f,
                "追随レコード（kind={KIND_OFFSET}）が 1 行も無い（消灯した観測点の採取を合格の根拠にしない）"
            ),
            Self::MalformedRecord { transition, detail } => {
                write!(f, "遷移 {transition}: 追随レコードを読めない（{detail}）")
            }
            Self::UnknownVerdict {
                transition,
                scope,
                verdict,
            } => write!(
                f,
                "遷移 {transition} {FIELD_SCOPE}={}: 語彙表に無い判定語 {FIELD_VERDICT}={verdict}（表は {OFFSET_VERDICT_ALL:?}）",
                scope_text(*scope)
            ),
            Self::UnexpectedVerdict {
                transition,
                scope,
                verdict,
                expected,
            } => write!(
                f,
                "遷移 {transition} {FIELD_SCOPE}={}: 判定語が期待の腕でない（実際 {verdict}・期待 {expected:?}）",
                scope_text(*scope)
            ),
            Self::ValueMoved {
                transition,
                scope,
                verdict,
                old_offset,
                new_offset,
            } => write!(
                f,
                "遷移 {transition} {FIELD_SCOPE}={}: {verdict} は値を動かさない腕なのに動いた（{}→{}）",
                scope_text(*scope),
                point_text(*old_offset),
                point_text(*new_offset)
            ),
            Self::KeywordPendingCarriesDisplayDpi {
                transition,
                scope,
                new_dpi,
            } => write!(
                f,
                "遷移 {transition} {FIELD_SCOPE}={}: {OFFSET_VERDICT_KEYWORD_PENDING} は表示 DPI 読取より前の門なのに {FIELD_NEW_DPI}={new_dpi} を運んでいる",
                scope_text(*scope)
            ),
            Self::RoundTripDrift {
                scope,
                dpi,
                first_transition,
                first,
                again_transition,
                again,
            } => write!(
                f,
                "{FIELD_SCOPE}={}: {FIELD_NEW_DPI}={dpi} へ戻ったのに値が bit 同一でない（遷移 {first_transition} で {}・遷移 {again_transition} で {}）",
                scope_text(*scope),
                point_text(*first),
                point_text(*again)
            ),
            Self::NoRoundTripObserved => write!(
                f,
                "往復が 1 度も観測されていない（同じ {FIELD_NEW_DPI} へ戻った追随レコードが無い＝要件 8.2 を判定できていない）"
            ),
            Self::NoLowScaleRescale {
                low_side_transitions,
            } => write!(
                f,
                "低い拡大率側で {FIELD_VERDICT}={OFFSET_VERDICT_RESCALED} が 1 度も出ていない（低い側へ移った遷移 {low_side_transitions} 本・キーワード指定スコープは素材消費後の遷移だけを数える）"
            ),
            Self::AlignmentResidual {
                transition,
                scope,
                axis,
                residual_hundredths,
                max_px,
            } => write!(
                f,
                "遷移 {transition} {FIELD_SCOPE}={}: {axis} 軸の揃えの残差 {}.{:02}px が許容量 {max_px}px を超えた",
                scope_text(*scope),
                residual_hundredths / 100,
                residual_hundredths % 100
            ),
            Self::NoKeywordAlignmentMeasured { keyword_scopes } => write!(
                f,
                "キーワード指定スコープの揃えを 1 度も測れていない（{OFFSET_VERDICT_KEYWORD_PENDING} を出したスコープ {keyword_scopes} 個・素材消費後に {OFFSET_VERDICT_RESCALED} の行が要る）"
            ),
        }
    }
}

/// スコープ番号の表示（番兵は発行側と同じ字面）。
fn scope_text(scope: Option<u32>) -> String {
    match scope {
        Some(scope) => scope.to_string(),
        None => MISSING.to_owned(),
    }
}

/// 点の表示（発行側の `point_field` と同じ形）。
fn point_text(point: (i32, i32)) -> String {
    format!("{},{}", point.0, point.1)
}

// ---------------------------------------------------------------------------
// 判定の結果
// ---------------------------------------------------------------------------

/// ログ 1 本ぶんの判定結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OffsetReport {
    /// 切り出した遷移。
    pub transitions: Vec<OffsetTransition>,
    /// 追随レコードの総数（読めた行だけ）。
    pub rows: usize,
    /// 違反の全件（空なら合格）。
    pub violations: Vec<OffsetViolation>,
}

impl OffsetReport {
    /// 不合格か。
    pub fn failed(&self) -> bool {
        !self.violations.is_empty()
    }
}

impl fmt::Display for OffsetReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "追随レコードの判定: 遷移 {} 本・{KIND_OFFSET} 行 {} 件・違反 {} 件",
            self.transitions.len(),
            self.rows,
            self.violations.len()
        )?;
        for violation in &self.violations {
            writeln!(f, "  - {violation}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------

/// ログ全文を判定する（実機サインオフのランナーと決定論テストが**同一の実装**を回す）。
pub fn judge_offset_log(log: &str) -> OffsetReport {
    let records = parse_transition_log(log);
    let mut violations = Vec::new();
    let transitions = collect_offset_transitions(&records, &mut violations);
    let rows = transitions
        .iter()
        .map(|transition| transition.rows.len())
        .sum();

    if rows == 0 {
        violations.push(OffsetViolation::NoOffsetRecords);
    } else {
        check_verdict_arms(&transitions, &mut violations);
        check_round_trip(&transitions, &mut violations);
        check_low_scale_rescale(&transitions, &mut violations);
        check_alignment_residual(&records, &transitions, &mut violations);
    }

    OffsetReport {
        transitions,
        rows,
        violations,
    }
}

// ---------------------------------------------------------------------------
// 切り出し
// ---------------------------------------------------------------------------

/// 起点ごとの区間から追随レコードだけを拾う。
fn collect_offset_transitions(
    records: &[TransitionRecord],
    violations: &mut Vec<OffsetViolation>,
) -> Vec<OffsetTransition> {
    let mut transitions = Vec::new();
    for (index, span) in split_transitions(records).into_iter().enumerate() {
        let Some(origin) = span.first().filter(|record| is_transition_origin(record)) else {
            continue;
        };
        let (Some(old_dpi), Some(new_dpi)) = (
            origin.int_field::<u32>(FIELD_OLD_DPI),
            origin.int_field::<u32>(FIELD_NEW_DPI),
        ) else {
            continue;
        };
        let mut rows = Vec::new();
        for record in span.iter().filter(|record| record.kind == KIND_OFFSET) {
            match offset_row(index, record) {
                Ok(row) => rows.push(row),
                Err(detail) => violations.push(OffsetViolation::MalformedRecord {
                    transition: index,
                    detail,
                }),
            }
        }
        transitions.push(OffsetTransition {
            index,
            old_dpi,
            new_dpi,
            rows,
        });
    }
    transitions
}

/// 1 行を読む（語彙の欠陥・読めない欄は本文つきの `Err`）。
fn offset_row(transition: usize, record: &TransitionRecord) -> Result<OffsetRow, String> {
    if !record.is_well_formed() {
        return Err(format!("{:?}", record.defects));
    }
    let new_dpi = record
        .int_field::<u32>(FIELD_NEW_DPI)
        .ok_or_else(|| format!("{FIELD_NEW_DPI} を数として読めない"))?;
    let verdict = record
        .field(FIELD_VERDICT)
        .ok_or_else(|| format!("{FIELD_VERDICT} が無い"))?
        .to_owned();
    Ok(OffsetRow {
        transition,
        scope: record.int_field::<u32>(FIELD_SCOPE),
        base_dpi: record.int_field::<u32>(FIELD_BASE_DPI),
        new_dpi,
        base_offset: point_field(record, FIELD_BASE_OFFSET)?,
        old_offset: point_field(record, FIELD_OLD_OFFSET)?,
        new_offset: point_field(record, FIELD_NEW_OFFSET)?,
        verdict,
    })
}

/// `x,y` の欄を読む。
fn point_field(record: &TransitionRecord, name: &str) -> Result<(i32, i32), String> {
    let raw = record
        .field(name)
        .ok_or_else(|| format!("{name} が無い"))?
        .to_owned();
    let (x, y) = raw
        .split_once(',')
        .ok_or_else(|| format!("{name}={raw} が `x,y` の形でない"))?;
    let x = x
        .parse::<i32>()
        .map_err(|error| format!("{name}={raw} の x を読めない: {error}"))?;
    let y = y
        .parse::<i32>()
        .map_err(|error| format!("{name}={raw} の y を読めない: {error}"))?;
    Ok((x, y))
}

// ---------------------------------------------------------------------------
// ⑵ 判定語が期待の腕であること（要件 8.3）
// ---------------------------------------------------------------------------

/// 行の欄から期待できる腕（逃げの 2 腕＝門・縮退は行そのものの不変条件だけを見る）。
///
/// **腕を `new_dpi` から推し量らない**——`new_dpi=0` は門と縮退の双方に現れるので、
/// 逃げの腕かどうかは `verdict` だけで見分ける。
fn expected_verdicts(row: &OffsetRow) -> Option<&'static [&'static str]> {
    if row.verdict == OFFSET_VERDICT_KEYWORD_PENDING || row.verdict == OFFSET_VERDICT_UNRESOLVED {
        return None;
    }
    Some(match row.base_dpi {
        // 未係留＝保存値の腕。値を動かさずに現在の表示 DPI を刻む。
        None => &[OFFSET_VERDICT_ANCHORED],
        // 恒等比の腕。**基準から引き直す**ので、現在値が基準から離れていれば値が動く
        // ＝`rescaled` が正しい（`unchanged` と記録したら語が嘘になる・design D4）。
        Some(base_dpi) if base_dpi == row.new_dpi => {
            if row.base_offset == row.old_offset {
                &[OFFSET_VERDICT_UNCHANGED]
            } else {
                &[OFFSET_VERDICT_RESCALED]
            }
        }
        // 比が恒等でない腕。飽和したかは行から見分けられないので両方を許す。
        Some(_) => &[OFFSET_VERDICT_RESCALED, OFFSET_VERDICT_SATURATED],
    })
}

fn check_verdict_arms(transitions: &[OffsetTransition], violations: &mut Vec<OffsetViolation>) {
    for row in transitions.iter().flat_map(|t| t.rows.iter()) {
        if !OFFSET_VERDICT_ALL.contains(&row.verdict.as_str()) {
            violations.push(OffsetViolation::UnknownVerdict {
                transition: row.transition,
                scope: row.scope,
                verdict: row.verdict.clone(),
            });
            continue;
        }
        if let Some(expected) = expected_verdicts(row)
            && !expected.contains(&row.verdict.as_str())
        {
            violations.push(OffsetViolation::UnexpectedVerdict {
                transition: row.transition,
                scope: row.scope,
                verdict: row.verdict.clone(),
                expected,
            });
        }
        if STABLE_VERDICTS.contains(&row.verdict.as_str()) && row.old_offset != row.new_offset {
            violations.push(OffsetViolation::ValueMoved {
                transition: row.transition,
                scope: row.scope,
                verdict: row.verdict.clone(),
                old_offset: row.old_offset,
                new_offset: row.new_offset,
            });
        }
        if row.verdict == OFFSET_VERDICT_KEYWORD_PENDING && row.new_dpi != 0 {
            violations.push(OffsetViolation::KeywordPendingCarriesDisplayDpi {
                transition: row.transition,
                scope: row.scope,
                new_dpi: row.new_dpi,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// ⑴ 往復の前後で bit 同一（要件 8.2）
// ---------------------------------------------------------------------------

/// 往復の突合の鍵——スコープ・基準対（値と DPI）・戻ってきた表示 DPI。
///
/// 基準が確立し直された（ドラッグ・面切替）ところで鍵が変わる＝区間が切れるので、
/// 利用者の操作を往復の破れと読み違えない。
type RoundTripKey = (Option<u32>, (i32, i32), Option<u32>, u32);

/// 鍵に対して最初に観測した（遷移番号・反映後の値）。
type RoundTripFirst = (usize, (i32, i32));

/// 基準対が生きている区間ごとに「同じ表示 DPI ＝ 同じ値」を突き合わせる。
///
/// 鍵は**値の欄**（`base_offset`／`base_dpi`／`new_dpi`）だけであり、判定語を 1 つも読まない
/// （module doc「⑴ を判定語の並びで見ない」）。`new_dpi=0` の行は遷移後の表示 DPI を持たない
/// ので 1 件も数えない。
fn check_round_trip(transitions: &[OffsetTransition], violations: &mut Vec<OffsetViolation>) {
    let mut seen: BTreeMap<RoundTripKey, RoundTripFirst> = BTreeMap::new();
    let mut round_trips = 0usize;
    for row in transitions.iter().flat_map(|t| t.rows.iter()) {
        if row.new_dpi == 0 {
            continue;
        }
        let key = (row.scope, row.base_offset, row.base_dpi, row.new_dpi);
        match seen.get(&key) {
            None => {
                seen.insert(key, (row.transition, row.new_offset));
            }
            Some(&(first_transition, first)) => {
                round_trips += 1;
                if first != row.new_offset {
                    violations.push(OffsetViolation::RoundTripDrift {
                        scope: row.scope,
                        dpi: row.new_dpi,
                        first_transition,
                        first,
                        again_transition: row.transition,
                        again: row.new_offset,
                    });
                }
            }
        }
    }
    if round_trips == 0 {
        violations.push(OffsetViolation::NoRoundTripObserved);
    }
}

// ---------------------------------------------------------------------------
// ⑶ 低い拡大率側で追随が出ていること（要件 8.4）
// ---------------------------------------------------------------------------

/// スコープごとに、門の判定語を最後に出した遷移番号（＝素材が消費された境目）。
fn last_pending_transition(transitions: &[OffsetTransition]) -> BTreeMap<Option<u32>, usize> {
    let mut last = BTreeMap::new();
    for row in transitions.iter().flat_map(|t| t.rows.iter()) {
        if row.verdict == OFFSET_VERDICT_KEYWORD_PENDING {
            last.insert(row.scope, row.transition);
        }
    }
    last
}

/// 当該行を ⑶ の母数に数えてよいか（キーワード指定スコープは**素材消費後**だけ）。
fn counts_for_low_scale(row: &OffsetRow, last_pending: &BTreeMap<Option<u32>, usize>) -> bool {
    match last_pending.get(&row.scope) {
        None => true,
        Some(&pending) => row.transition > pending,
    }
}

fn check_low_scale_rescale(
    transitions: &[OffsetTransition],
    violations: &mut Vec<OffsetViolation>,
) {
    let last_pending = last_pending_transition(transitions);
    let low_side: Vec<&OffsetTransition> = transitions
        .iter()
        .filter(|transition| transition.to_lower_scale())
        .collect();
    let rescaled = low_side
        .iter()
        .flat_map(|transition| transition.rows.iter())
        .any(|row| {
            row.verdict == OFFSET_VERDICT_RESCALED && counts_for_low_scale(row, &last_pending)
        });
    if !rescaled {
        violations.push(OffsetViolation::NoLowScaleRescale {
            low_side_transitions: low_side.len(),
        });
    }
}

// ---------------------------------------------------------------------------
// ⑷ キーワード指定スコープの揃えの残差（要件 8.5・design D8）
// ---------------------------------------------------------------------------

/// 揃えの残差を測れる行か（比を持つ追随の行だけ）。測れるなら基準 DPI を返す。
fn residual_measurable(row: &OffsetRow) -> Option<u32> {
    if row.verdict != OFFSET_VERDICT_RESCALED || row.new_dpi == 0 {
        return None;
    }
    row.base_dpi.filter(|base_dpi| *base_dpi != 0)
}

/// 1 軸ぶんの残差（px の 1/100 単位・切り捨て）と、許容量を超えたか。
///
/// 期待値は**厳密な有理数** `base_offset × new_dpi ÷ base_dpi` である。比較は整数のまま
/// 行い（`|new × base_dpi − base_offset × new_dpi| > 許容量 × base_dpi`）、表示のためだけに
/// 1/100 px へ落とす——浮動小数を合否の側に入れない。
fn axis_residual(base_offset: i32, new_offset: i32, base_dpi: u32, new_dpi: u32) -> (i64, bool) {
    let numerator = (i64::from(new_offset) * i64::from(base_dpi)
        - i64::from(base_offset) * i64::from(new_dpi))
    .abs();
    let denominator = i64::from(base_dpi);
    (
        numerator * 100 / denominator,
        numerator > ALIGNMENT_RESIDUAL_MAX_PX * denominator,
    )
}

/// 母数となるキーワード指定スコープの集合を、**観測行の全体**から作る（task 8.6）。
///
/// # なぜ遷移の内側に限らないのか
///
/// 門の行は「その時点で再導出の素材が未消費だった」という**事実の記録**であり、その行が
/// 遷移の内側にあるかどうかはその事実を変えない。そして実機では門の行は**構造的に必ず
/// 最初の起点より前**に出る——素材は起動から 0.73〜5.0 秒で自動的に消費される
/// （`ReportedSizeReconcile` の基準確定）のに対し、最初の起点は利用者のドラッグ由来なので
/// 必ずそれより後になる。[`split_transitions`] は最初の起点より前の行を捨てるため、遷移の
/// 内側だけから母数を作ると**キーワード指定スコープの母数は永久に空**になり、正しい実装の
/// ままでも [`OffsetViolation::NoKeywordAlignmentMeasured`] の偽の赤が出る（2026-08-28 の
/// 実機ログ 3 本すべてがこの形・開発者裁定で母数の作り方を広げた）。
///
/// # なぜ広げても甘くならないのか
///
/// 広げるのは**母数（どのスコープを測る対象とみなすか）だけ**である。残差そのものを測る行
/// （[`residual_measurable`] が通す `rescaled` の行）は従来どおり遷移の内側からしか採らない
/// ので、合否の厳しさは 1 つも落ちない。門の行がどこにも無いログでは集合が空のままなので、
/// 従来どおり `NoKeywordAlignmentMeasured` が立つ（母数を広げた結果、検査が何も要求しなく
/// なる形にはならない・`transition_judge_offset_tests.rs` の 3 通りの檻が固定する）。
///
/// 語彙の規約を破っている行は数えない——読めない行から母数を組むと、行の形が壊れたときに
/// 母数だけが静かに増える。
fn keyword_pending_scopes(records: &[TransitionRecord]) -> BTreeSet<Option<u32>> {
    records
        .iter()
        .filter(|record| record.kind == KIND_OFFSET && record.is_well_formed())
        .filter(|record| record.field(FIELD_VERDICT) == Some(OFFSET_VERDICT_KEYWORD_PENDING))
        .map(|record| record.int_field::<u32>(FIELD_SCOPE))
        .collect()
}

/// 揃えの残差を判定する。
///
/// 引数に**元の観測行の並び**（`records`）を取るのは、母数を [`keyword_pending_scopes`] が
/// 観測行の全体から作るためである（遷移だけでは最初の起点より前の門の行に届かない）。
fn check_alignment_residual(
    records: &[TransitionRecord],
    transitions: &[OffsetTransition],
    violations: &mut Vec<OffsetViolation>,
) {
    let keyword_scopes = keyword_pending_scopes(records);
    let mut measured = 0usize;
    for row in transitions
        .iter()
        .flat_map(|t| t.rows.iter())
        .filter(|row| keyword_scopes.contains(&row.scope))
    {
        let Some(base_dpi) = residual_measurable(row) else {
            continue;
        };
        measured += 1;
        for (axis, base, new) in [
            ("x", row.base_offset.0, row.new_offset.0),
            ("y", row.base_offset.1, row.new_offset.1),
        ] {
            let (residual_hundredths, over) = axis_residual(base, new, base_dpi, row.new_dpi);
            if over {
                violations.push(OffsetViolation::AlignmentResidual {
                    transition: row.transition,
                    scope: row.scope,
                    axis,
                    residual_hundredths,
                    max_px: ALIGNMENT_RESIDUAL_MAX_PX,
                });
            }
        }
    }
    if measured == 0 {
        violations.push(OffsetViolation::NoKeywordAlignmentMeasured {
            keyword_scopes: keyword_scopes.len(),
        });
    }
}

#[cfg(test)]
#[path = "transition_judge_offset_tests.rs"]
mod transition_judge_offset_tests;

#[cfg(test)]
#[path = "transition_judge_offset_signoff_tests.rs"]
mod transition_judge_offset_signoff_tests;

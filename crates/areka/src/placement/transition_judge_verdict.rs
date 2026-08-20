//! 遷移判定の**上限と合否**——決定論の上限と実機サインオフ専用の上限を別々に持ち、違反を
//! 列で返す（純関数・I/O 無し）。
//!
//! design.md の C7（`transition_judge`）が正本である。[`super`] が集めた判定量
//! （[`TransitionSummary`]）に上限（[`Bounds`]）を当てて合否を出す層であり、量を数える側とは
//! 分けてある（量の集計は語彙に追随し、上限は要件に追随するので、変更の理由が異なる）。
//!
//! # 上限は 2 系統・判定は別々の呼び出し
//!
//! - [`Bounds::deterministic`]——フレーム差・窓ごとの書込回数・経路 A の同期書込・接地点差・
//!   連鎖再解決の回数。**回帰テストが固定する**決定論量だけを持つ（要件 7.1）。
//! - [`Bounds::signoff`]——可視化から窓書込までの経過（`visualize_to_write_us`）と一括書込の
//!   総所要（`flush_total_us`）。**実機サインオフ専用**であり非決定なので、**回帰テストは
//!   値を固定しない**（C7・設計討議 A-2）。C8 の Q2／Q3 の分岐条件はこの系統の超過である。
//!
//! 2 系統は `judge(summary, &Bounds::deterministic())` と `judge(summary, &Bounds::signoff())`
//! の**別々の呼び出し**で評価し、[`Report`] は両方の結果を並べて持つ（C7）。片方へ畳むと、
//! 非決定量の揺れが決定論判定の合否を動かす。
//!
//! # 「量が無いこと」を合格の根拠にしない
//!
//! [`judge`] は沈黙を合格にしない。次はそれぞれ**別の [`Violation`]** として返す:
//!
//! - [`TransitionSummary::frames_indeterminate`]（フレーム系列が読めない・要件 8.5）
//! - [`TransitionSummary::malformed_records`] が 1 件以上（語彙の規約を破った行がある）
//! - 上限を当てる対象そのものが欠けていること（[`Violation::Unmeasured`]）
//!
//! 3 番目が要るのは、[`super::RecordDefect::UnreadableField`] が**接頭語**（`frame`／`t_us`）に
//! しか立たないためである。本体側の数値（`diff`・`total_us`・`target_id`）が壊れた行は
//! `is_well_formed() == true`・`malformed_records == 0` のまま当該量だけが静かに消え、上限
//! （`≤ 0` や `≤ 1`）は「量が無い」を素通しにしてしまう。

use std::collections::BTreeSet;
use std::fmt;

use super::super::diag::WindowKind;
use super::{TransitionSummary, WindowKey, parse_transition_log, split_transitions, summarize};

// ---------------------------------------------------------------------------
// 上限の定数
// ---------------------------------------------------------------------------

/// 起点から最終書込までの許容フレーム差（C7・要件 4.4）。
///
/// 0＝「モニタ表更新と同じフレームで全窓の書込が終わる」。可視化と窓書込の食い違い
/// （[`TransitionSummary::mismatch_frames_per_window`]・要件 4.2／4.3）にも同じ値を当てる
/// ——どちらも「食い違うフレームを 1 枚も出さない」という同一の要求である。
pub const TRANSITION_FRAME_BOUND: u32 = 0;

/// 整合待ち（`hold`）を含む遷移でフレーム差に足す許容（C7）。
///
/// 値の正本は**本番の整合ゲートの上限**（`dpi_sync::DPI_SYNC_HOLD_MAX_FRAMES`）である。
/// task 3.2 の時点では本番側に定数が無く同じ値をここに置いていたが、task 5.4 でゲートが
/// 着地したので参照へ替えた——判定の許容が本番の待ちより短ければ、規約どおりに待った遷移が
/// 不合格になる（判定語の単一定義元と同じ理屈が値にも及ぶ）。
/// hold を含まない遷移（`holds == 0`）では 1 フレームも足さない。
pub const HOLD_FRAME_ALLOWANCE: u32 = crate::placement::dpi_sync::DPI_SYNC_HOLD_MAX_FRAMES;

/// 1 つの窓に対する遷移 1 回ぶんの窓書込回数の上限（C7・要件 4.5）。
pub const WRITES_PER_WINDOW_MAX: u32 = 1;

/// 経路 A（OS 提案位置の同期書込）の件数の上限（C7・要件 4.5）。
///
/// `origin=dpi-suggested` の書込と、その裏取りである `stage=sync` の書込の**両方**へ当てる。
/// 片方だけに当てると、札と段のどちらかが壊れた退行が判定から消える。
pub const PATH_A_WRITES_MAX: u32 = 0;

/// 接地点と作業領域下端の差の絶対値の上限（C7・要件 5.3）。
pub const GROUND_DIFF_MAX: i32 = 0;

/// 遷移 1 回あたりの連鎖再解決の回数の上限（C7・要件 6.6「一度だけ」）。
pub const CHAIN_REALIGN_PER_TRANSITION: u32 = 1;

/// 可視化から当該窓の窓書込までの経過（µs）の上限。**確定値**（task 4.3・確定台帳 L9）。
///
/// 実機サインオフ専用（[`Bounds::signoff`]）。値の根拠は**採取機に依らない**——表示に
/// 提示される 1 フレームは、リフレッシュレートが 60Hz を下回らない限り高々 1/60 秒＝
/// 16,667µs である。ゆえにこれを超えた食い違いは**どの機械でも**必ず 1 枚以上の提示
/// フレームをまたぐ（要件 4.2「食い違い可視フレームを提示しない」を µs の側から**一方向に**
/// 押さえた形——超えていれば必ずまたぐが、逆に超えていないことは 4.2 の成立を意味しない）。
/// 逆に上限以下の隔たりは 1 フレームに収まり**得る**ので違反として立てない。
///
/// 採取機の実測周期（本仕様の再採取は 8.37〜8.82ms＝113〜119 フレーム/秒）を定数にしない
/// のは、リフレッシュレートの違う機械で判定が変わるからである。60Hz を下回る表示では
/// 1 フレームが本値より長くなり上限は厳しすぎる側へ倒れる——そのときは実測周期を添えて
/// 台帳で再裁定する（**目視に合わせて緩めない**・手順書 §6.5）。
///
/// 非決定量ゆえ回帰テストはこの**値**を固定しない（C7）。固定するのは [`Bounds::signoff`]
/// がこの定数から引いていること（結線）と、値が 0 でないことだけである。
pub const VISUALIZE_TO_WRITE_US_MAX: u64 = 16_667;

/// 一括書込 1 区間の総所要（µs）の上限。**確定値**（task 4.3・確定台帳 L9）。
///
/// 根拠は [`VISUALIZE_TO_WRITE_US_MAX`] と同一である。一括書込が 1 フレームを超えると、
/// 区間の前半で書いた窓と後半で書いた窓が**別の提示フレーム**に現れる＝要件 4.5 が
/// 「1 回の書込」で消そうとしている逐次適用が、書込の回数を下限化しても時間の側に
/// 残る形になる。ゆえに上限は窓の枚数に比例させない（枚数ぶん緩めると、枚数を増やす
/// だけで合格する上限になる）。
pub const FLUSH_TOTAL_US_MAX: u64 = 16_667;

/// 実機ログの受け渡しに使う環境変数（C7 の起動手順）。
pub const TRANSITION_LOG_ENV: &str = "AREKA_TRANSITION_LOG";

// ---------------------------------------------------------------------------
// 上限
// ---------------------------------------------------------------------------

/// 遷移 1 回ぶんの判定に当てる上限の組。
///
/// 各項目が `None` のとき、その量は**判定しない**（当該系統の管轄外）。armed な項目が 1 つも
/// 無い組は [`Violation::NoBoundsArmed`] になる——「何も当てないから合格」を作らないため。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// 起点から最終書込までのフレーム差と、可視化・書込の食い違いフレーム差の上限。
    pub frame_bound: Option<u32>,
    /// 整合待ちを含む遷移で `frame_bound` に足す許容（`frame_bound` が `None` なら無効）。
    pub hold_frame_allowance: u32,
    /// 窓ごとの書込回数の上限。
    pub writes_per_window_max: Option<u32>,
    /// 経路 A の同期書込の件数の上限（`stage=sync` の裏取りにも当てる）。
    pub path_a_writes_max: Option<u32>,
    /// 接地点差の絶対値の上限。
    pub ground_diff_abs_max: Option<i32>,
    /// 連鎖再解決の回数の上限。
    pub chain_realigned_max: Option<u32>,
    /// **実機専用**: 可視化から窓書込までの経過（µs）の上限。
    pub visualize_to_write_us_max: Option<u64>,
    /// **実機専用**: 一括書込 1 区間の総所要（µs）の上限。
    pub flush_total_us_max: Option<u64>,
}

impl Bounds {
    /// 何も判定しない組（各系統の構築はこれを起点に armed な項目だけを埋める）。
    ///
    /// テストが [`Violation::NoBoundsArmed`] を直接確かめられるよう判定器の外へ開いてある
    /// （合否を出す用途では使わない——armed が 1 つも無い組は必ず違反になる）。
    pub(super) const fn nothing() -> Self {
        Self {
            frame_bound: None,
            hold_frame_allowance: 0,
            writes_per_window_max: None,
            path_a_writes_max: None,
            ground_diff_abs_max: None,
            chain_realigned_max: None,
            visualize_to_write_us_max: None,
            flush_total_us_max: None,
        }
    }

    /// 決定論の上限（回帰テストが固定する・要件 7.1）。
    ///
    /// 実機専用の 2 項目は `None` のままにする——非決定量を決定論判定へ混ぜない（C7）。
    pub const fn deterministic() -> Self {
        Self {
            frame_bound: Some(TRANSITION_FRAME_BOUND),
            hold_frame_allowance: HOLD_FRAME_ALLOWANCE,
            writes_per_window_max: Some(WRITES_PER_WINDOW_MAX),
            path_a_writes_max: Some(PATH_A_WRITES_MAX),
            ground_diff_abs_max: Some(GROUND_DIFF_MAX),
            chain_realigned_max: Some(CHAIN_REALIGN_PER_TRANSITION),
            ..Self::nothing()
        }
    }

    /// 実機サインオフ専用の上限（**確定値**・task 4.3 が台帳 L9 で裁定した）。
    ///
    /// 決定論の項目は `None` のままにする——同一 tick の内側の食い違いはフレーム単位の量では
    /// 是正前でも 0 になるので、この系統だけが C8 の Q2／Q3 を分岐できる。
    pub const fn signoff() -> Self {
        Self {
            visualize_to_write_us_max: Some(VISUALIZE_TO_WRITE_US_MAX),
            flush_total_us_max: Some(FLUSH_TOTAL_US_MAX),
            ..Self::nothing()
        }
    }

    /// 1 つでも上限を当てるか。
    pub const fn arms_anything(&self) -> bool {
        self.frame_bound.is_some()
            || self.writes_per_window_max.is_some()
            || self.path_a_writes_max.is_some()
            || self.ground_diff_abs_max.is_some()
            || self.chain_realigned_max.is_some()
            || self.visualize_to_write_us_max.is_some()
            || self.flush_total_us_max.is_some()
    }
}

// ---------------------------------------------------------------------------
// 違反
// ---------------------------------------------------------------------------

/// 上限を当てる対象そのものが欠けていた量。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quantity {
    /// 起点から最終書込までのフレーム差（＝窓書込が 1 件も無い）。
    FramesToLastWrite,
    /// 接地点差（`ground` レコードが無い・`diff` が読めない）。
    GroundDiff,
    /// 一括書込の総所要（`flush stage=end` が無い・`total_us` が読めない）。
    FlushTotalUs,
    /// 当該キャラ窓の随伴の同一フレーム性（対を検査できていない＝`balloon_same_frame` は空虚な真）。
    BalloonSameFrame(WindowKey),
    /// 当該窓の可視化・書込の食い違いフレーム差。
    MismatchFrames(WindowKey),
    /// 当該窓の可視化から書込までの経過。
    VisualizeToWriteUs(WindowKey),
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FramesToLastWrite => f.write_str("frames_to_last_write"),
            Self::GroundDiff => f.write_str("ground_diff_max"),
            Self::FlushTotalUs => f.write_str("flush_total_us_max"),
            Self::BalloonSameFrame(window) => write!(f, "balloon_same_frame[{window}]"),
            Self::MismatchFrames(window) => write!(f, "mismatch_frames_per_window[{window}]"),
            Self::VisualizeToWriteUs(window) => write!(f, "visualize_to_write_us[{window}]"),
        }
    }
}

/// 判定違反 1 件。
///
/// 種別を分けてあるのは、ランナーが列挙するときに「どの量が・いくつで・上限がいくつか」を
/// そのまま読めるようにするためである（沈黙を PASS にしない＝design「Error Handling」）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Violation {
    /// 上限を 1 つも当てていない組で判定した。
    NoBoundsArmed,
    /// ログに遷移が 1 本も無い（[`Report`] 水準）。
    NoTransitionsFound,
    /// 語彙の規約を破った行がある。
    MalformedRecords {
        /// 件数。
        count: u32,
    },
    /// フレーム差が判定不能（要件 8.5）。
    FramesIndeterminate,
    /// 区間の先頭が起点レコードでない。
    MissingOrigin,
    /// 上限を当てる対象の量が欠けている。
    Unmeasured(Quantity),
    /// 起点から最終書込までのフレーム差が上限超。
    FramesToLastWrite {
        /// 実測。
        frames: u32,
        /// 上限（整合待ちの許容を足した後）。
        max: u32,
    },
    /// 可視化と窓書込のフレーム差が上限超。
    MismatchFrames {
        /// 対象窓。
        window: WindowKey,
        /// 実測。
        frames: u32,
        /// 上限。
        max: u32,
    },
    /// 随伴バルーンがキャラ窓と別のフレームで書かれた（要件 4.3）。
    BalloonWrittenInAnotherFrame,
    /// 窓ごとの書込回数が上限超。
    WritesPerWindow {
        /// 対象窓。
        window: WindowKey,
        /// 実測。
        writes: u32,
        /// 上限。
        max: u32,
    },
    /// 経路 A（`origin=dpi-suggested`）の書込が上限超。
    PathAWrites {
        /// 実測。
        writes: u32,
        /// 上限。
        max: u32,
    },
    /// `stage=sync` の書込が上限超（経路 A の裏取り）。
    SyncStageWrites {
        /// 実測。
        writes: u32,
        /// 上限。
        max: u32,
    },
    /// 接地点差が上限超。
    GroundDiff {
        /// 実測（符号つき・負＝浮き）。
        diff: i32,
        /// 上限（絶対値）。
        max: i32,
    },
    /// 連鎖再解決が上限超。
    ChainRealigned {
        /// 実測。
        realigned: u32,
        /// 上限。
        max: u32,
    },
    /// **実機専用**: 可視化から窓書込までの経過が上限超。
    VisualizeToWriteUs {
        /// 対象窓。
        window: WindowKey,
        /// 実測（µs）。
        us: u64,
        /// 上限（µs）。
        max: u64,
    },
    /// **実機専用**: 一括書込の総所要が上限超。
    FlushTotalUs {
        /// 実測（µs）。
        us: u64,
        /// 上限（µs）。
        max: u64,
    },
    /// 書込のあった窓が**1 つ残らず**除外され、窓ごとの量を 1 つも測っていない。
    ///
    /// ⑼ の被覆検査は「除外されなかった窓」を回るので、除外が全窓へ広がると**検査そのものが
    /// 1 度も回らない**——番人が、番をすべき当の欠陥で無効化される形である（task 4.2 の実機
    /// 採取で実際に起きた。`reason=k-unchanged` を除外に使っていたため 12 遷移すべてで
    /// 全窓が除外され、窓ごとの量が 1 件も判定されないまま実機専用系統が合格していた）。
    /// 全窓除外は「上限を満たした」ではなく**測っていない**なので、ここで違反として立てる。
    AllWrittenWindowsExcluded {
        /// 書込のあった窓の件数（すべて除外された）。
        windows: u32,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoBoundsArmed => f.write_str("上限を 1 つも当てていない（合格の根拠が無い）"),
            Self::NoTransitionsFound => {
                f.write_str("遷移が 1 本も切り出せない（起点レコードが無い）")
            }
            Self::MalformedRecords { count } => {
                write!(f, "語彙の規約を破った行 {count} 件")
            }
            Self::FramesIndeterminate => f.write_str("フレーム差が判定不能（一様に 0／読めない）"),
            Self::MissingOrigin => f.write_str("区間の先頭が起点レコードでない"),
            Self::Unmeasured(quantity) => write!(f, "判定対象の量が欠けている: {quantity}"),
            Self::FramesToLastWrite { frames, max } => {
                write!(f, "最終書込までのフレーム差 {frames} > 上限 {max}")
            }
            Self::MismatchFrames {
                window,
                frames,
                max,
            } => write!(
                f,
                "可視化と書込のフレーム差 {frames} > 上限 {max}（{window}）"
            ),
            Self::BalloonWrittenInAnotherFrame => {
                f.write_str("随伴バルーンがキャラ窓と別のフレームで書かれた")
            }
            Self::WritesPerWindow {
                window,
                writes,
                max,
            } => write!(f, "窓ごとの書込回数 {writes} > 上限 {max}（{window}）"),
            Self::PathAWrites { writes, max } => {
                write!(f, "経路 A の書込 {writes} > 上限 {max}")
            }
            Self::SyncStageWrites { writes, max } => {
                write!(f, "同期段の書込 {writes} > 上限 {max}")
            }
            Self::GroundDiff { diff, max } => {
                write!(f, "接地点差 {diff} は絶対値の上限 {max} を超える")
            }
            Self::ChainRealigned { realigned, max } => {
                write!(f, "連鎖再解決 {realigned} 回 > 上限 {max} 回")
            }
            Self::VisualizeToWriteUs { window, us, max } => {
                write!(f, "可視化から書込まで {us}µs > 上限 {max}µs（{window}）")
            }
            Self::FlushTotalUs { us, max } => {
                write!(f, "一括書込の総所要 {us}µs > 上限 {max}µs")
            }
            Self::AllWrittenWindowsExcluded { windows } => write!(
                f,
                "書込のあった窓 {windows} 件がすべて除外され、窓ごとの量を 1 つも測っていない"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// 合否
// ---------------------------------------------------------------------------

/// 遷移 1 回ぶんの合否を出す（C7）。
///
/// 違反は**列**で返す。1 件見つけた時点で打ち切らないのは、ランナーが遷移ごとに全ての違反を
/// 列挙して初めて「何を直せばよいか」が読めるためである。
pub fn judge(summary: &TransitionSummary, bounds: &Bounds) -> Result<(), Vec<Violation>> {
    let mut violations: Vec<Violation> = Vec::new();

    // ⑴ 判定そのものが成り立つか。
    if !bounds.arms_anything() {
        violations.push(Violation::NoBoundsArmed);
    }

    // ⑵ 入力の健全性（どちらの系統でも同じく効く——壊れた観測はどちらの上限も支えない）。
    if summary.origin.is_none() {
        violations.push(Violation::MissingOrigin);
    }
    if summary.malformed_records > 0 {
        violations.push(Violation::MalformedRecords {
            count: summary.malformed_records,
        });
    }
    if summary.frames_indeterminate {
        violations.push(Violation::FramesIndeterminate);
    }

    // ⑶ フレーム差（要件 4.2／4.3／4.4）。
    if let Some(frame_bound) = bounds.frame_bound {
        // 整合待ちを含む遷移だけが待ちの許容を使える（`holds == 0` には 1 フレームも足さない）。
        let last_write_bound = if summary.holds > 0 {
            frame_bound.saturating_add(bounds.hold_frame_allowance)
        } else {
            frame_bound
        };
        match summary.frames_to_last_write {
            None => violations.push(Violation::Unmeasured(Quantity::FramesToLastWrite)),
            Some(frames) if frames > last_write_bound => {
                violations.push(Violation::FramesToLastWrite {
                    frames,
                    max: last_write_bound,
                });
            }
            Some(_) => {}
        }
        for (window, frames) in &summary.mismatch_frames_per_window {
            if *frames > frame_bound {
                violations.push(Violation::MismatchFrames {
                    window: window.clone(),
                    frames: *frames,
                    max: frame_bound,
                });
            }
        }
        if !summary.balloon_same_frame {
            violations.push(Violation::BalloonWrittenInAnotherFrame);
        }
        // `balloon_same_frame` の既定は真なので、検査できたはずの対が検査されていないことは
        // 「随伴は同一フレームだった」の根拠にならない（空虚な真）。判定側は**スコープごとに**
        // 数え直す——遷移全体で 1 組も無いことを条件にすると、scope が 1 つでその随伴が
        // 見送り窓のときに毎回発火し、非欠陥を不合格にする。
        for expectation in companion_expectations(summary) {
            let unmeasured = match expectation.state {
                // 随伴が 1 度も書かれていない（見送り窓でもない）＝測りようがない。
                CompanionState::BalloonNotWritten => true,
                // 検査できるはずの対が集計側で数えられていない＝空虚な真。
                CompanionState::Checkable => {
                    summary.balloon_pairs_checked < checkable_pairs(summary)
                }
            };
            if unmeasured {
                violations.push(Violation::Unmeasured(Quantity::BalloonSameFrame(
                    expectation.character.clone(),
                )));
            }
        }
    }

    // ⑷ 窓ごとの書込回数（要件 4.5）。
    if let Some(max) = bounds.writes_per_window_max {
        for (window, writes) in &summary.writes_per_window {
            if *writes > max {
                violations.push(Violation::WritesPerWindow {
                    window: window.clone(),
                    writes: *writes,
                    max,
                });
            }
        }
    }

    // ⑸ 経路 A（札と段の両方に当てる）。
    if let Some(max) = bounds.path_a_writes_max {
        if summary.path_a_writes > max {
            violations.push(Violation::PathAWrites {
                writes: summary.path_a_writes,
                max,
            });
        }
        if summary.sync_stage_writes > max {
            violations.push(Violation::SyncStageWrites {
                writes: summary.sync_stage_writes,
                max,
            });
        }
    }

    // ⑹ 接地点差（要件 5.3）。符号は保ったまま絶対値で比べる（浮きも沈みも違反）。
    if let Some(max) = bounds.ground_diff_abs_max {
        match summary.ground_diff_max {
            None => violations.push(Violation::Unmeasured(Quantity::GroundDiff)),
            Some(diff) if diff.saturating_abs() > max => {
                violations.push(Violation::GroundDiff { diff, max });
            }
            Some(_) => {}
        }
    }

    // ⑺ 連鎖再解決（要件 6.6）。
    if let Some(max) = bounds.chain_realigned_max
        && summary.chain_realigned > max
    {
        violations.push(Violation::ChainRealigned {
            realigned: summary.chain_realigned,
            max,
        });
    }

    // ⑻ 実機専用（非決定・C8 の Q2／Q3 の分岐条件）。
    if let Some(max) = bounds.visualize_to_write_us_max {
        for (window, us) in &summary.visualize_to_write_us {
            if *us > max {
                violations.push(Violation::VisualizeToWriteUs {
                    window: window.clone(),
                    us: *us,
                    max,
                });
            }
        }
    }
    if let Some(max) = bounds.flush_total_us_max {
        match summary.flush_total_us_max {
            None => violations.push(Violation::Unmeasured(Quantity::FlushTotalUs)),
            Some(us) if us > max => violations.push(Violation::FlushTotalUs { us, max }),
            Some(_) => {}
        }
    }

    // ⑼ 窓ごとの量の被覆——書込のあった窓が当該の量を 1 つも持たないことは、上限
    // （`≤ 0`）を満たしたのではなく**測っていない**。見送り窓は要件 4.6 で合否から外す。
    //
    // 先に**除外が全窓へ広がっていないこと**を見る。下の走査は `judged_windows` を回るので、
    // 全窓が除外されると 1 度も回らず、被覆検査は恒真になる（＝「零件だから合格」）。
    // 書込のあった窓が居るのに判定対象が 1 つも残らないなら、それは合格ではなく未測定である。
    let judges_windows = bounds.frame_bound.is_some() || bounds.visualize_to_write_us_max.is_some();
    if judges_windows
        && !summary.writes_per_window.is_empty()
        && judged_windows(summary).next().is_none()
    {
        violations.push(Violation::AllWrittenWindowsExcluded {
            windows: summary.writes_per_window.len() as u32,
        });
    }
    for window in judged_windows(summary) {
        if bounds.frame_bound.is_some() && !summary.mismatch_frames_per_window.contains_key(window)
        {
            violations.push(Violation::Unmeasured(Quantity::MismatchFrames(
                window.clone(),
            )));
        }
        if bounds.visualize_to_write_us_max.is_some()
            && !summary.visualize_to_write_us.contains_key(window)
        {
            violations.push(Violation::Unmeasured(Quantity::VisualizeToWriteUs(
                window.clone(),
            )));
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// 遷移 1 回ぶんの判定量と、2 系統の合否。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionVerdict {
    /// 判定量。
    pub summary: TransitionSummary,
    /// 決定論の上限による合否（[`Bounds::deterministic`]）。
    pub deterministic: Result<(), Vec<Violation>>,
    /// 実機専用の上限による合否（[`Bounds::signoff`]）。
    pub signoff: Result<(), Vec<Violation>>,
}

/// ログ 1 本ぶんの判定結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// 解析できた観測行の本数。
    pub records: usize,
    /// 最初の起点より前にあり、どの遷移にも属さない行の本数。
    pub unassigned_records: usize,
    /// 上のうち語彙の規約を破っていた行の本数（どの遷移の判定にも載らないので別に持つ）。
    pub unassigned_malformed_records: u32,
    /// 遷移ごとの判定。
    pub transitions: Vec<TransitionVerdict>,
}

impl Report {
    /// ログ水準の違反（遷移が 1 本も無い・遷移の外に壊れた行がある）。
    pub fn log_violations(&self) -> Vec<Violation> {
        let mut violations = Vec::new();
        if self.transitions.is_empty() {
            violations.push(Violation::NoTransitionsFound);
        }
        if self.unassigned_malformed_records > 0 {
            violations.push(Violation::MalformedRecords {
                count: self.unassigned_malformed_records,
            });
        }
        violations
    }

    /// 不合格か（ログ水準の違反、または 2 系統いずれかの違反があるか）。
    pub fn failed(&self) -> bool {
        !self.log_violations().is_empty()
            || self
                .transitions
                .iter()
                .any(|verdict| verdict.deterministic.is_err() || verdict.signoff.is_err())
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "[judge] records={} transitions={} unassigned={} (malformed {})",
            self.records,
            self.transitions.len(),
            self.unassigned_records,
            self.unassigned_malformed_records
        )?;
        for violation in self.log_violations() {
            writeln!(f, "  ! {violation}")?;
        }
        for (index, verdict) in self.transitions.iter().enumerate() {
            match verdict.summary.origin {
                Some(origin) => writeln!(
                    f,
                    "  transition #{} frame={} dpi {} -> {} records={}",
                    index + 1,
                    origin.stamp.frame,
                    origin.old_dpi,
                    origin.new_dpi,
                    verdict.summary.records
                )?,
                None => writeln!(f, "  transition #{} (起点なし)", index + 1)?,
            }
            write_quantities(f, &verdict.summary)?;
            write_family(f, "deterministic", &verdict.deterministic)?;
            write_family(f, "signoff", &verdict.signoff)?;
        }
        Ok(())
    }
}

/// 遷移 1 回ぶんの判定量を**合否によらず**書き出す（task 4.3 の裁定）。
///
/// 違反の列挙だけでは `PASS` になった系統の量が 1 つも残らない。是正の前後を並べるのは
/// 「上限の合否」ではなく**量そのもの**なので（基準値 §7）、合否で刷り分けると比較の側が
/// 毎回**生ログから手で起こす**羽目になる——task 4.2 が実際にそうなり、9 量 × 12 遷移を
/// 手で起こした。ここで無条件に刷れば task 7.3 は同じ表を機械で得られる。
///
/// 上限は当てない（当てるのは [`judge`] の仕事）。ここは量を読める形に並べるだけである。
fn write_quantities(f: &mut fmt::Formatter<'_>, summary: &TransitionSummary) -> fmt::Result {
    writeln!(
        f,
        "    量: frames_to_last_write={} writes={} path_a_writes={} sync_stage_writes={} \
         balloon_same_frame={}(pairs={}) holds={} chain_realigned={} ground_diff_max={} \
         flush_total_us_max={} malformed={} frames_indeterminate={}",
        opt_u32(summary.frames_to_last_write),
        summary.writes,
        summary.path_a_writes,
        summary.sync_stage_writes,
        summary.balloon_same_frame,
        summary.balloon_pairs_checked,
        summary.holds,
        summary.chain_realigned,
        summary
            .ground_diff_max
            .map_or_else(|| "-".to_owned(), |diff| diff.to_string()),
        opt_u64(summary.flush_total_us_max),
        summary.malformed_records,
        summary.frames_indeterminate,
    )?;
    writeln!(
        f,
        "    量(参考): first_write_t_us={} last_write_t_us={} sum_call_us={}",
        opt_u64(summary.wall.first_write_t_us),
        opt_u64(summary.wall.last_write_t_us),
        summary.wall.sum_call_us,
    )?;

    // 窓ごとの量は 3 つの表に散っているので、窓の鍵で 1 行へ束ねる（書込・食い違い
    // フレーム・可視化から書込までの µs）。どれか 1 つしか無い窓も落とさない。
    let mut windows: BTreeSet<&WindowKey> = BTreeSet::new();
    windows.extend(summary.writes_per_window.keys());
    windows.extend(summary.mismatch_frames_per_window.keys());
    windows.extend(summary.visualize_to_write_us.keys());
    for window in windows {
        writeln!(
            f,
            "    量(窓): {window} writes={} mismatch_frames={} visualize_to_write_us={}",
            opt_u32(summary.writes_per_window.get(window).copied()),
            opt_u32(summary.mismatch_frames_per_window.get(window).copied()),
            opt_u64(summary.visualize_to_write_us.get(window).copied()),
        )?;
    }

    // 見送り窓は「量が無い」ことの理由なので、0 件でも行を出す（黙って消えると
    // 「測っていない」と「測って 0 だった」が区別できない）。
    write!(f, "    量(見送り窓):")?;
    if summary.skipped_windows.is_empty() {
        writeln!(f, " なし")
    } else {
        for window in &summary.skipped_windows {
            write!(f, " [{window}]")?;
        }
        writeln!(f)
    }
}

/// 欠けている量は番兵で刷る（空欄にすると「0 だった」と読めてしまう）。
fn opt_u32(value: Option<u32>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| value.to_string())
}

/// [`opt_u32`] の 64bit 版。
fn opt_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| value.to_string())
}

/// 1 系統ぶんの合否を書き出す。
fn write_family(
    f: &mut fmt::Formatter<'_>,
    family: &str,
    outcome: &Result<(), Vec<Violation>>,
) -> fmt::Result {
    match outcome {
        Ok(()) => writeln!(f, "    {family}: PASS"),
        Err(violations) => {
            writeln!(f, "    {family}: {} 件の違反", violations.len())?;
            for violation in violations {
                writeln!(f, "      - {violation}")?;
            }
            Ok(())
        }
    }
}

/// ログ全文を解析・切り出し・集計・判定まで通す（C7 の合成）。
///
/// 2 系統は**別々の呼び出し**で評価し、両方を [`TransitionVerdict`] に並べて持つ。
pub fn judge_transition_log(log: &str) -> Report {
    let records = parse_transition_log(log);
    let transitions = split_transitions(&records);
    let assigned: usize = transitions.iter().map(Vec::len).sum();
    let unassigned_records = records.len() - assigned;
    // 捨てられるのは最初の起点より前の**前置き**だけ（`split_transitions` の契約）。
    let unassigned_malformed_records = records
        .iter()
        .take(unassigned_records)
        .filter(|record| !record.is_well_formed())
        .count() as u32;

    let deterministic_bounds = Bounds::deterministic();
    let signoff_bounds = Bounds::signoff();
    let verdicts = transitions
        .iter()
        .map(|transition| {
            let summary = summarize(transition);
            TransitionVerdict {
                deterministic: judge(&summary, &deterministic_bounds),
                signoff: judge(&summary, &signoff_bounds),
                summary,
            }
        })
        .collect();

    Report {
        records: records.len(),
        unassigned_records,
        unassigned_malformed_records,
        transitions: verdicts,
    }
}

/// 随伴の同一フレーム性について、あるキャラ窓に期待できること。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompanionState {
    /// 随伴バルーンは見送り窓でもないのに 1 度も書かれていない（対を検査しようがない）。
    BalloonNotWritten,
    /// 随伴バルーンにも書込があり、集計側が対を数えられるはず。
    Checkable,
}

/// 判定側が独立に数え直した「随伴の対」1 件。
struct CompanionExpectation<'a> {
    /// キャラ窓。
    character: &'a WindowKey,
    /// 期待できる状態。
    state: CompanionState,
}

/// 書込のあったキャラ窓ごとに、随伴の対に何が期待できるかを出す。
///
/// キャラ窓が見送り窓のときは 1 件も出さない——要件 4.3 の引き金は「キャラ窓が遷移後の
/// 寸法・位置で可視化されるとき」であり、再表示を見送られたキャラ窓では立たない。
/// バルーン側の見送りは対の期待を消さない（随伴の窓書込は不可視のバルーンにも出る）が、
/// 「見送られていて**位置の書込**も無い」対だけは要件 4.6 に従って求めない。
///
/// 判定の鍵は**経路語**である。対を検査できるのはバルーンが
/// [`TransitionSummary::balloon_follow_windows`] に居るとき——つまり `origin=BalloonFollow` の
/// 位置書込を受けたときだけであって、寸書込（`KeepPositionResize`）が届いただけでは
/// 要件 4.3 の義務は 1 度も観測されていない。
fn companion_expectations(summary: &TransitionSummary) -> Vec<CompanionExpectation<'_>> {
    let mut expectations = Vec::new();
    for character in summary.writes_per_window.keys() {
        if !character.is_char() || summary.skipped_windows.contains(character) {
            continue;
        }
        let Some(scope) = character.scope else {
            continue;
        };
        let balloon = WindowKey::of(scope, WindowKind::Balloon);
        let state = if summary.balloon_follow_windows.contains(&balloon) {
            CompanionState::Checkable
        } else if summary.skipped_windows.contains(&balloon) {
            // 見送られたうえ位置の書込も無い＝現状維持（要件 4.6）。求めない。
            continue;
        } else {
            CompanionState::BalloonNotWritten
        };
        expectations.push(CompanionExpectation { character, state });
    }
    expectations
}

/// 集計側が数えられるはずの対の件数（[`companion_expectations`] の `Checkable` の数）。
fn checkable_pairs(summary: &TransitionSummary) -> u32 {
    companion_expectations(summary)
        .iter()
        .filter(|expectation| expectation.state == CompanionState::Checkable)
        .count() as u32
}

/// 見送り窓を除いた「書込のあった窓」（上限を当てる対象の被覆に使う）。
fn judged_windows(summary: &TransitionSummary) -> impl Iterator<Item = &WindowKey> {
    let skipped: &BTreeSet<WindowKey> = &summary.skipped_windows;
    summary
        .writes_per_window
        .keys()
        .filter(move |window| !skipped.contains(*window))
}

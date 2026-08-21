//! 実表示寸での連鎖再解決（scg 要件 7・design C6）。
//!
//! # なぜ要るか
//!
//! 初期配置（[`super::resolver::resolve_placement`]）は **spawn 時点で判っているサーフェス寸**で
//! 解決される。ところがゴーストは起動直後に別のサーフェスを選び得るため、実表示寸が配置時と
//! 異なることがある。窓は下端中央を保ったまま再アンカーされる（`follow::resize_window_to`・
//! 完了済み `areka-P0-surface-resize-resnap` の領分）ので**各キャラの接地点は正しい**が、
//! スコープ間の連鎖は再計算されず、幅の変化ぶんだけ隣接が崩れる。
//!
//! 実機実測（emo2・拡大率 200%）: 配置は scope0 幅 868 で解決され隙間 0。直後に実表示面
//! （幅 764）へ切り替わり、下端中央固定の再アンカーで左端が `(868−764)÷2 = 52` 右へ寄って
//! **52px の隙間**が残っていた。
//!
//! # 何をするか
//!
//! 「初期配置は実表示寸が確定するまで暫定」とみなし、確定時に**連鎖だけ**を解き直す
//! （要件 7.1）。規則は resolver の P2 式そのもの——`new_x(n) = x(n−1) − w(n)`。
//!
//! - **先頭スコープは動かさない**（連鎖の起点・接地点は不変・7.2）。
//! - **Y は扱わない**（下端吸着は各窓の再アンカーが既に保っている・7.2）。
//! - **明示的に再配置されたスコープは対象外**（7.3）。判定は「現在位置が既定位置と一致するか」
//!   で行う——ゴースト台本の移動指令や利用者のドラッグで動いた窓は一致しなくなるため、移動側へ
//!   フックを足さずに除外できる。除外したスコープの**実位置**は以後の連鎖の基準
//!   として使う（「クランプ後の実配置を連鎖基準とする」現行原則と同型・要件 2.7）。
//!   既定位置は起動時に固定されるのではなく、**システム由来の再アンカー**で書込先へ追随する
//!   （`areka-P0-dpi-transition-atomicity` D9／D16・要件 6.2）——拡大率の遷移で全スコープの
//!   位置が動いても、明示操作をしていないスコープは一致したまま対象に残る。
//! - 確定は**一度きり**で、以後のサーフェス切替では駆動しない（7.4）。その制御は呼び手
//!   （`emo2_boot::frame`）が持ち、本モジュールは純粋な判定だけを担う。
//!
//! # 確定が起きなかったとき
//!
//! 確定の見送りは**正常な待ち**（起動中でまだ表示が揃っていない）と**異常な停滞**（条件が
//! 永久に揃わない）を兼ねる。どちらも無音だと、隣接が崩れたままの報告を受けたときに
//! 「確定が走らなかった」のか「走って移動 0 だった」のかをログから判別できない。そこで
//! [`ChainFinalizeStall`] が見送りを数え、有界の待ち（[`CHAIN_FINALIZE_STALL_FRAMES`]）を
//! 超えた時点で**一度だけ**理由つきの診断を出す（scg 6.5・steering「ログ無し失敗経路の禁止」）。

use std::fmt;

use bevy_ecs::prelude::Resource;

use super::resolver::PointPx;

/// 初期配置が実表示寸で確定済みであることを表す標識（要件 7.4）。
///
/// 挿入されている間は連鎖の再解決を駆動しない。**一度きり**の確定を保証する制御であり、
/// 以後のサーフェス切替（会話中の表情差替等）で位置が動かないことの構造的な根拠になる。
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChainFinalized;

/// 連鎖再解決の入力（スコープ 1 件ぶん・物理 px）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeChainState {
    /// スコープ番号（呼び手は**昇順**で渡す）。
    pub scope: usize,
    /// 現在のキャラ窓左上 X（再アンカー後の実位置）。
    pub current_x: i32,
    /// 現在のキャラ窓幅（実表示サーフェスの物理寸）。
    pub width: i32,
    /// 既定キャラ位置 X（未接触判定の基準）。
    ///
    /// 初期値は spawn 時の resolver 出力だが、**システム由来の再アンカー**（拡大率の再射影・
    /// 作業領域の再スナップ等）で位置が動いたときは単一の窓書込口が同じ量を運ぶため、値は
    /// 「最後にシステムが置いた既定位置」である（`areka-P0-dpi-transition-atomicity`
    /// D9／D16・要件 6.2）。運ばないと拡大率の遷移で全スコープが「明示的に動かされた」へ
    /// 倒れ、本判定が空振りする。
    ///
    /// **`None` は「そもそも既定配置ではない」**（保存位置が復元されたスコープ）。
    /// この場合は現在位置に関わらず**常に対象外**とする（scg 7.3）。
    pub default_x: Option<i32>,
}

/// 連鎖再解決の結果（動かすべきスコープと新しい左上 X）。
///
/// 既に正しい位置に居るスコープは**含まれない**（冗長な書き込みを出さない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainMove {
    /// 対象スコープ番号。
    pub scope: usize,
    /// 新しいキャラ窓左上 X（物理 px）。
    pub new_x: i32,
}

/// 実表示寸で連鎖を解き直し、動かすべきスコープと新 X の列を返す（要件 7.1/7.2/7.3）。
///
/// `states` は**スコープ昇順**で渡すこと（連鎖は前スコープの結果に依存するため順序が意味を持つ）。
///
/// # 判定
///
/// 先頭スコープは基準として素通しし、以降のスコープ `n` について:
///
/// 1. 幅が非正（`width <= 0`）なら**動かさない**。実位置を次の基準にする（表示未確立などの
///    縮退入力で暴走した座標を作らない・`resize_window_to` の非正寸ガードと同じ考え方）。
/// 2. `default_x != Some(current_x)`＝**既定配置のままではない**なら動かさない。実位置を次の
///    基準にする。これは 2 通りを畳んだもの——`Some(d)` かつ `d != current_x`（台本の移動指令や
///    利用者のドラッグで動かされた）と、`None`（そもそも既定配置ではない＝保存位置の復元）。
/// 3. それ以外は `new_x = 前スコープの結果 X − 自スコープの幅`。`current_x` と異なるときだけ
///    [`ChainMove`] を積み、いずれにせよ `new_x` を次の基準にする。
///
/// # 事後条件
///
/// 返した指示をすべて適用すると、**連続する未接触ペア**について
/// `x(n−1) == x(n) + w(n)`（隙間 0）が実表示寸で成立する。
///
/// 空入力・単一スコープでは常に空列（動かす相手が居ない）。飽和演算ゆえ極端入力でも panic しない。
pub fn finalize_chain(states: &[ScopeChainState]) -> Vec<ChainMove> {
    let mut moves = Vec::new();
    let mut prev_x: Option<i32> = None;

    for s in states {
        let resulting_x = match prev_x {
            // 先頭スコープ＝連鎖の起点。実位置をそのまま基準にする（動かさない・7.2）。
            None => s.current_x,
            Some(base_x) => {
                if s.width <= 0 || s.default_x != Some(s.current_x) {
                    // 動かさない 3 通り——非正寸（縮退入力）／既定配置から動かされている
                    // （`Some(d)` かつ `d != current_x`）／そもそも既定配置ではない
                    // （`None`＝保存位置の復元）。いずれも 7.3 の「明示された位置へ
                    // 引き戻さない」に属し、その実位置を以後の連鎖基準にする
                    // （実配置基準の原則・2.7 と同型）。
                    s.current_x
                } else {
                    let new_x = base_x.saturating_sub(s.width);
                    if new_x != s.current_x {
                        moves.push(ChainMove {
                            scope: s.scope,
                            new_x,
                        });
                    }
                    new_x
                }
            }
        };
        prev_x = Some(resulting_x);
    }

    moves
}

/// 再解決後の既定位置（[`ChainMove`] を反映した [`PointPx`]）を組む補助。
///
/// Y は再解決の対象外ゆえ呼び手が持つ現在の Y をそのまま載せる（7.2）。
pub fn moved_default_pos(current: PointPx, new_x: i32) -> PointPx {
    PointPx {
        x: new_x,
        y: current.y,
    }
}

// ---------------------------------------------------------------------------
// 確定が見送られ続けたときの一発診断（scg 6.5・design C6「確定が見送られ続けた場合の一発診断」）
// ---------------------------------------------------------------------------

/// 診断を出すまでに待つフレーム数（有界の待ち・scg 6.5）。
///
/// 呼び手（`emo2_boot::frame::emo2_frame_system`）は 60Hz の tick ループから毎フレーム確定を
/// 試みるため、**約 10 秒**ぶんに相当する。窓は `open_startup_window` の時点で既に生えており、
/// この待ちで残るのは GPU 装着と初回 `ShowSurface` の landing だけゆえ、正常起動が閾値に届く
/// ことはない（＝正常系で診断が出ない余裕を持たせた値）。
pub const CHAIN_FINALIZE_STALL_FRAMES: u32 = 600;

/// 確定の見送りを数える（診断を一度だけ出すための状態・scg 6.5）。
///
/// [`ChainFinalized`] と同じく**一発フラグ**の形。確定が起きればこの資源は二度と読まれない
/// （呼び手が確定標識で先に打ち切るため）。
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChainFinalizeStall {
    /// 確定できずに見送ったフレーム数。
    pub deferrals: u32,
    /// 診断を出し終えたか（二度目以降は数えも報せもしない）。
    pub reported: bool,
}

/// 確定を見送った理由（診断ログの本文・scg 6.5）。
///
/// 「どのスコープが」「どの条件で」見送られたかを一意に表す。走査は最初に躓いた条件で
/// 打ち切るため、報告されるのは**その時点の先頭の障害**である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainDeferReason {
    /// `GhostWindows` が世界に無い（窓が生える前・破棄後）。
    NoGhostWindows,
    /// スコープが 1 つも無い。
    NoScopes,
    /// キャラ窓 entity が引けない。
    NoCharWindow { scope: usize },
    /// 実表示寸が引けない＝初回 `ShowSurface` が未成立。
    NotShownYet { scope: usize },
    /// 実表示寸が i32 で扱えない、または非正（縮退入力）。
    UnusableShownSize { scope: usize, w: u32, h: u32 },
    /// キャラ窓に `WindowPos` が無い。
    NoWindowPos { scope: usize },
    /// `WindowPos` の位置または寸が未確定（`None`）。
    IncompleteWindowPos { scope: usize },
    /// 再アンカーが未 landing＝窓寸が実表示寸に追いついていない。
    ResnapNotLanded {
        scope: usize,
        shown: (i32, i32),
        window: (i32, i32),
    },
}

impl ChainDeferReason {
    /// 障害が生じたスコープ番号（世界全体に関わる理由では `None`）。
    pub fn scope(&self) -> Option<usize> {
        match *self {
            ChainDeferReason::NoGhostWindows | ChainDeferReason::NoScopes => None,
            ChainDeferReason::NoCharWindow { scope }
            | ChainDeferReason::NotShownYet { scope }
            | ChainDeferReason::UnusableShownSize { scope, .. }
            | ChainDeferReason::NoWindowPos { scope }
            | ChainDeferReason::IncompleteWindowPos { scope }
            | ChainDeferReason::ResnapNotLanded { scope, .. } => Some(scope),
        }
    }
}

impl fmt::Display for ChainDeferReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ChainDeferReason::NoGhostWindows => {
                write!(f, "GhostWindows が無い（窓が生えていない）")
            }
            ChainDeferReason::NoScopes => write!(f, "スコープが 1 つも無い"),
            ChainDeferReason::NoCharWindow { scope } => {
                write!(f, "scope {scope}: キャラ窓 entity が引けない")
            }
            ChainDeferReason::NotShownYet { scope } => {
                write!(f, "scope {scope}: 実表示寸が未確定（初回表示が未成立）")
            }
            ChainDeferReason::UnusableShownSize { scope, w, h } => {
                write!(f, "scope {scope}: 実表示寸が扱えない（{w}x{h}）")
            }
            ChainDeferReason::NoWindowPos { scope } => {
                write!(f, "scope {scope}: キャラ窓に WindowPos が無い")
            }
            ChainDeferReason::IncompleteWindowPos { scope } => {
                write!(f, "scope {scope}: WindowPos の位置か寸が未確定")
            }
            ChainDeferReason::ResnapNotLanded {
                scope,
                shown: (sw, sh),
                window: (ww, wh),
            } => write!(
                f,
                "scope {scope}: 再アンカーが未 landing（実表示寸 {sw}x{sh} ≠ 窓寸 {ww}x{wh}）"
            ),
        }
    }
}

/// 見送りを 1 フレームぶん記録し、**この呼び出しで診断を出すべきか**を返す（scg 6.5）。
///
/// - 閾値（[`CHAIN_FINALIZE_STALL_FRAMES`]）に**到達したちょうどその 1 回**だけ `true`。
/// - それ以前は `false`（毎フレームの見送りは無音）。
/// - 報告後は数えることもやめて常に `false`（同じ停滞で溢れさせない）。
pub fn note_chain_deferral(stall: &mut ChainFinalizeStall) -> bool {
    if stall.reported {
        return false;
    }
    stall.deferrals = stall.deferrals.saturating_add(1);
    if stall.deferrals < CHAIN_FINALIZE_STALL_FRAMES {
        return false;
    }
    stall.reported = true;
    true
}

#[cfg(test)]
#[path = "chain_finalize_tests.rs"]
mod chain_finalize_tests;

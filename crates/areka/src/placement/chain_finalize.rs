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
//! - **明示的に再配置されたスコープは対象外**（7.3）。判定は「現在位置が spawn 時の既定位置と
//!   一致するか」で行う——ゴースト台本の移動指令や利用者のドラッグで動いた窓は一致しなくなる
//!   ため、移動側へフックを足さずに除外できる。除外したスコープの**実位置**は以後の連鎖の基準
//!   として使う（「クランプ後の実配置を連鎖基準とする」現行原則と同型・要件 2.7）。
//! - 確定は**一度きり**で、以後のサーフェス切替では駆動しない（7.4）。その制御は呼び手
//!   （`emo2_boot::frame`）が持ち、本モジュールは純粋な判定だけを担う。

use super::resolver::PointPx;

/// 連鎖再解決の入力（スコープ 1 件ぶん・物理 px）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeChainState {
    /// スコープ番号（呼び手は**昇順**で渡す）。
    pub scope: usize,
    /// 現在のキャラ窓左上 X（再アンカー後の実位置）。
    pub current_x: i32,
    /// 現在のキャラ窓幅（実表示サーフェスの物理寸）。
    pub width: i32,
    /// spawn 時の既定キャラ位置 X（未接触判定の基準）。
    pub default_x: i32,
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
/// 2. `current_x != default_x`＝**明示的に再配置済み**なら動かさない。実位置を次の基準にする。
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
                if s.width <= 0 || s.current_x != s.default_x {
                    // 非正寸（縮退入力）／明示的に再配置済み（7.3）はいずれも動かさず、
                    // その実位置を以後の連鎖基準にする（実配置基準の原則・2.7 と同型）。
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

#[cfg(test)]
#[path = "chain_finalize_tests.rs"]
mod chain_finalize_tests;

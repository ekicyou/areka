//! アンカーの開き `\_a[ID,…]` と閉じ `\_a` の対応の判定（純粋・no I/O）。
//!
//! 命令の列を先頭から見て、対応の崩れ（閉じの無い開き・開いている間の新たな開き・開いて
//! いないのに閉じ）を位置付きで返す。再生（compile）と台本の診断（`check_script`）が同じ結果を
//! 読むので、どちらも同じ所に同じ内容の警告を出す（anchor-tag-canon 要件 1.8〜1.10・6.3）。
//! 補い（閉じの合図を足す・迷子の閉じを捨てる）と記録は呼び手の仕事で、ここでは何も出さない。

use areka_parsers::sakura::Instruction;

/// 開きと閉じの対応の崩れの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorIssue {
    /// 開きに閉じが無い（走査の終わりまで）。`index` は開きの位置。
    Unclosed,
    /// 開いている間の新たな開き。`index` は新しい開きの位置（直前をここで閉じる）。
    Reopened,
    /// 開いていないのに閉じ。`index` は閉じの位置（無視する）。
    StrayClose,
}

/// 対応の崩れ 1 件。`index` は渡された命令の列の添字（0 始まり）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorFinding {
    pub index: usize,
    pub issue: AnchorIssue,
}

/// 命令の列から、開きと閉じの対応の崩れを返す。
///
/// 台詞の終わり `\e`（`End`）と終了 `\-`（`Quit`）で走査を止め、それより後ろは数えない
/// （compile がそこで読むのをやめるのと同じ）。止まった所・末尾で開いたままの開きは閉じ無し。
///
/// 事後条件: `index` の昇順。同じ位置に同じ種類は 2 件付かない。同じ位置に 2 件付くのは、
/// 重なりの開きが閉じられないまま終わるときだけ（その開きに `Reopened`・`Unclosed` の順で
/// 付き、必ず列の末尾になる）。`Unclosed` は高々 1 件。失敗の経路は無い。
pub fn pair_anchors<'a>(
    instructions: impl IntoIterator<Item = &'a Instruction>,
) -> Vec<AnchorFinding> {
    let mut findings = Vec::new();
    let mut found = |index, issue| findings.push(AnchorFinding { index, issue });
    // 今開いているアンカーの開きの位置（開いているのは高々 1 つ）。
    let mut open: Option<usize> = None;
    for (index, instruction) in instructions.into_iter().enumerate() {
        match instruction {
            // 開いていれば直前はここで閉じたものとして、新しい開きに入れ替える。
            Instruction::Anchor(_) => {
                if open.replace(index).is_some() {
                    found(index, AnchorIssue::Reopened);
                }
            }
            Instruction::AnchorEnd => {
                if open.take().is_none() {
                    found(index, AnchorIssue::StrayClose);
                }
            }
            Instruction::End | Instruction::Quit => break,
            _ => {}
        }
    }
    if let Some(index) = open {
        found(index, AnchorIssue::Unclosed);
    }
    findings
}

#[cfg(test)]
#[path = "anchor_pair_tests.rs"]
mod tests;

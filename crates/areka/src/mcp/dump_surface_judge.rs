//! `dump_surface`／`dump_balloon` の判断（どの文言を返すか・何を撮るか）と成功の本文
//! （spec: areka-P0-mcp-dump-images）。窓も GPU も要らない純粋な関数だけを置く。

/// 判断で返す断りの文言（spec: areka-P0-mcp-dump-images-residue 要件 2.2）。中身は非公開で、このモジュールの外では作れない
/// （想定外の失敗の `fail` へ渡す書き方は組み立てで落ちる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::mcp) struct Refusal(&'static str);

impl Refusal {
    pub(in crate::mcp) fn as_str(self) -> &'static str {
        self.0
    }
}

pub(in crate::mcp) const NO_WINDOW: Refusal = Refusal("This ghost has no window");
pub(in crate::mcp) const NO_SUCH_SCOPE: Refusal = Refusal("No such scope in this ghost");
pub(in crate::mcp) const NO_SUCH_SURFACE: Refusal =
    Refusal("No such surface ID. Check get_expression_table tool");
pub(in crate::mcp) const NOT_SHOWN_YET: Refusal =
    Refusal("No surface has been shown in this scope yet");

/// 判断に要る事実。本番は表示の層を読み、テストは手書きの表で答える。
pub(in crate::mcp) trait ShellFacts {
    /// そのスコープに絵の資産が組まれているか。
    fn scope_exists(&self, scope: u32) -> bool;
    /// 生の surface ID がシェルに在るか（別名は見ない）。
    fn surface_exists(&self, scope: u32, surface_id: u32) -> bool;
    /// 最後に表示した surface ID（隠していても残る・一度も表示していなければ None）。
    fn last_shown(&self, scope: u32) -> Option<u32>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::mcp) enum SurfacePlan {
    /// 今の見た目（本文に載せる surface ID つき）。
    Shown { scope: u32, surface_id: u32 },
    /// 指定した surface 単体。
    Alone { scope: u32, surface_id: u32 },
}

/// `facts` が None＝窓の無いゴースト。Err は `NG:` の後ろに付ける理由。
/// 順は 窓 → スコープ → surface ID → 一度も表示していない（要件 4.9）。
pub(in crate::mcp) fn judge_surface(
    facts: Option<&dyn ShellFacts>,
    scope: Option<i64>,
    surface: Option<i64>,
) -> Result<SurfacePlan, Refusal> {
    let (facts, scope) = facts_and_scope(facts, scope)?;
    match surface {
        Some(id) => match u32::try_from(id) {
            Ok(surface_id) if facts.surface_exists(scope, surface_id) => {
                Ok(SurfacePlan::Alone { scope, surface_id })
            }
            _ => Err(NO_SUCH_SURFACE),
        },
        None => facts
            .last_shown(scope)
            .map(|surface_id| SurfacePlan::Shown { scope, surface_id })
            .ok_or(NOT_SHOWN_YET),
    }
}

/// Ok はスコープ番号。順は 窓 → スコープ。
pub(in crate::mcp) fn judge_balloon(
    facts: Option<&dyn ShellFacts>,
    scope: Option<i64>,
) -> Result<u32, Refusal> {
    facts_and_scope(facts, scope).map(|(_, scope)| scope)
}

/// 窓とスコープの 2 段。スコープは省略で 0、0 以上 `i32::MAX` 以下だけを数える
/// （target の番号 `2*scope+1` が `u32` からあふれないように）。
fn facts_and_scope(
    facts: Option<&dyn ShellFacts>,
    scope: Option<i64>,
) -> Result<(&dyn ShellFacts, u32), Refusal> {
    let facts = facts.ok_or(NO_WINDOW)?;
    let scope = i32::try_from(scope.unwrap_or(0))
        .ok()
        .and_then(|s| u32::try_from(s).ok())
        .filter(|&s| facts.scope_exists(s))
        .ok_or(NO_SUCH_SCOPE)?;
    Ok((facts, scope))
}

/// 要件 1.4 の `OK:` の後ろ。
pub(in crate::mcp) fn shown_text(scope: u32, surface_id: u32) -> String {
    format!(
        "scope {scope}, surface {surface_id} as currently shown (with running animations and dressups, before scaling and transparency)"
    )
}

/// 要件 2.4 の `OK:` の後ろ。
pub(in crate::mcp) fn alone_text(scope: u32, surface_id: u32) -> String {
    format!(
        "scope {scope}, surface {surface_id} rendered alone in its initial state (not what is on the screen now)"
    )
}

/// 要件 3.7 の `OK:` の後ろ。
pub(in crate::mcp) fn balloon_text(scope: u32) -> String {
    format!(
        "balloon of scope {scope} as last drawn (before scaling and transparency; kept even if the balloon is hidden now)"
    )
}

#[cfg(test)]
#[path = "dump_surface_judge_tests.rs"]
mod tests;

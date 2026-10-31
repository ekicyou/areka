//! 着せ替えの腕（`\![bind,…]`・`\![bind-noevent,…]` の読み・当て・記録・spec: script-small-tags 要件 3.1〜3.9）。
//!
//! [`super::dispatch_message`] が名前で選んだ正準キャリアだけがここへ来る。2 つの名前は同じに当て、
//! 記録の `name` の欄だけで分かれる。SHIORI へ送る道は持たない（要件 3.6）。

use areka_sakura::TalkCue;

use super::emit_display;
use crate::bind::{BindDirective, BindResolver, NamedBindResult, apply_bind_directive};
use crate::looper::LoopRuntime;
use crate::output::{DisplayCommand, SurfaceOutput};
use crate::state::{BindApplyOutcome, ScopeStates, Slot};

/// 着せ替えの指令を 1 つ読み、パーツ 1 つずつ当てて記録し、変わった分があれば全部当てた後に 1 回だけ出す。
pub(super) fn on_bind<O: SurfaceOutput>(
    bind_resolver: &BindResolver,
    states: &mut ScopeStates,
    loop_runtime: &mut LoopRuntime,
    out: &mut O,
    cue: &TalkCue,
    name: &str,
    tokens: &[&str],
) {
    let directive = crate::bind::parse_bind_directive(tokens);
    if directive == BindDirective::Malformed {
        // カテゴリ欠落・on/off 値破損＝破損入力（D8③）。
        tracing::error!(
            name,
            command = ?cue.command,
            "seriko: bind の破損入力（カテゴリ欠落・on/off 値破損）を読み飛ばす（D8③）"
        );
        return;
    }

    // 形ごとの当て方は `apply_bind_directive`（パーツ 1 つずつ今の `apply_named_bind`）。ここは記録と発行だけ。
    let mut last_changed: Option<DisplayCommand> = None;
    for applied in apply_bind_directive(states, bind_resolver, &cue.actor, &directive) {
        let (category, part, on) = (&applied.category, &applied.part, applied.on);
        match applied.result {
            NamedBindResult::NoNamespace => {
                // 写像なし（数でない scope・D7・D8⑤）。
                tracing::warn!(
                    name,
                    scope = %cue.actor,
                    "seriko: bind の scope 写像なし（数でない scope）で読み飛ばす（D7・D8⑤）"
                );
            }
            NamedBindResult::NotFound => {
                tracing::error!(
                    name,
                    scope = %cue.actor,
                    category = %category,
                    part = %part,
                    "seriko: bind の (カテゴリ, パーツ) を名前解決できず読み飛ばす（未宣言・状態不変・発行なし・R3.7・D8①）"
                );
            }
            NamedBindResult::MustSelectKept { id } => {
                // mustselect の脱衣は正典「解除不可」: 集合を変えず、痕跡を実機の既定水準で見える warn! で残す。
                tracing::warn!(
                    name,
                    scope = %cue.actor,
                    category = %category,
                    part = %part,
                    id,
                    on,
                    "seriko: mustselect カテゴリの脱衣指示を無視（正典・解除不可・bindopt 3.2）"
                );
            }
            NamedBindResult::Applied {
                id,
                outcome: BindApplyOutcome::Changed(command),
            } => {
                last_changed = Some(command);
                // 実機サインオフの grep マーカー（R7.1・`name="bind-noevent"` も目印）。
                tracing::info!(
                    name,
                    scope = %cue.actor,
                    category = %category,
                    part = %part,
                    id,
                    on,
                    "seriko: bind 適用"
                );
            }
            // 非表示/未知 scope（StateOnly）または同値（Unchanged）は発行しない（D5）。
            NamedBindResult::Applied { id, .. } => {
                tracing::debug!(
                    name,
                    scope = %cue.actor,
                    category = %category,
                    part = %part,
                    id,
                    on,
                    "seriko: bind 集合を更新（非表示/未知 scope または同値ゆえ発行なし）"
                );
            }
        }
    }

    if let Some(command) = last_changed {
        // 途中の姿は出さず、最後の集合で 1 件だけ。部品のコマを載せ直した Show があればそれを出す
        // （spec: areka-P0-surface-element-nesting 要件 5.12・areka-P0-animated-image-playback 要件 3.1）。
        let at_ms = loop_runtime.event_ms();
        let command = loop_runtime
            .refresh(&cue.actor, Slot::Shell, at_ms, states)
            .unwrap_or(command);
        emit_display(out, command); // 単一発行点（R3.5）
    }
}

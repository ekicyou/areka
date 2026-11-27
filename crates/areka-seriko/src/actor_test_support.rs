use super::*;
use crate::output::MockSurfaceOutput;
use areka_emo_compose::{BindSet, PatternState};
use log_capture_kit::{LineFormat, capture_lines};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// テストの道具は層をまたぐ受け口の 1 か所の定義。
pub(super) use crate::test_support::{capture_text, clear_all, cue, fresh_states, text};
pub(super) use dola::cue::test_support::carrier_cue;
use dola::{TimePoint, TimeSpan};

/// テスト用の TalkCue（Shell 系 Emote・at/actor 込み）を組む。
pub(super) fn emote_cue(at: TimeSpan, scope: &str, key: &str) -> TalkCue {
    // 表情切替は瞬時（明示的 0）。
    cue(
        at,
        scope,
        CueCommand::Emote { key: key.into() },
        TimeSpan::ZERO,
    )
}

/// `cue` を時刻 `now` に発火した合図に組み直す（台本の起点は `now − cue.at`）。
pub(super) fn fired(now: TimePoint, cue: TalkCue) -> TalkCue {
    TalkCue::scheduled(now - cue.at, cue.at, cue.actor, cue.command, cue.duration)
}

/// (腕,伸び)→1302 の sakura 名前表を持つ解決層（static={1100,1207} と交わらない新 id・
/// `bindoption` 宣言なし）。
pub(super) fn arm_bind_resolver() -> BindResolver {
    BindResolver::from_groups(&crate::bind::tests::groups_of(
        "charset,UTF-8\r\nsakura.bindgroup1302.name,腕,伸び\r\n",
    ))
}

/// 判定に使わない空の差し替えの事実。
pub(super) fn no_facts() -> crate::SkinFacts {
    crate::SkinFacts::new(BTreeMap::new(), BTreeMap::new(), BTreeMap::new())
}

/// 同期 `handle_message` 用の小さな解決層（"通常"→2100 の 1 件のみ）。
pub(super) fn tiny_resolver() -> SurfaceResolver {
    let mut aliases: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    aliases.insert("通常".to_string(), vec![2100]);
    SurfaceResolver::new(aliases)
}

/// 不活性なループ統括器（空表＋ダミー乱数）。cue/bind/balloon の同期 `handle_message` 檻で
/// tick 経路を触らないための足場（`disabled()` は on_tick 常時空・on_surface_changed は空 playback
/// への no-op で、発行も記録も足さない）。
pub(super) fn inert_runtime() -> LoopRuntime {
    LoopRuntime::new(SerikoLoopConfig::disabled())
}

/// 着せ替えの名前の表（sakura の (目, 閉) → 5・既定のポリシー＝着衣は排他置換・脱衣は除去）。
pub(super) fn eye_resolver() -> BindResolver {
    BindResolver::from_groups(&crate::bind::tests::groups_of(
        "charset,UTF-8\r\nsakura.bindgroup5.name,目,閉\r\n",
    ))
}

/// 同期 `handle_message` に、時刻を書いた刻み・合図・窓の知らせを流す足場。
pub(super) struct ClockRig {
    resolver: SurfaceResolver,
    bind_resolver: BindResolver,
    pub(super) states: ScopeStates,
    rt: LoopRuntime,
    out: MockSurfaceOutput,
    records: Arc<Mutex<Vec<DisplayCommand>>>,
}

impl ClockRig {
    /// 別名の表なし・着せ替えなしで、統括器を `config` から組む。
    pub(super) fn with_config(config: SerikoLoopConfig) -> Self {
        Self::configured(config, BindResolver::empty(), BindSet::from_ids([]))
    }

    /// 統括器の設定・着せ替えの名前の表・静的な着せ替えから組む（面の別名の表は空）。
    pub(super) fn configured(
        config: SerikoLoopConfig,
        bind_resolver: BindResolver,
        static_binds: BindSet,
    ) -> Self {
        let out = MockSurfaceOutput::new();
        let records = out.records();
        Self {
            resolver: SurfaceResolver::new(BTreeMap::new()),
            bind_resolver,
            states: ScopeStates::new(crate::DefaultBinds::uniform(static_binds)),
            rt: LoopRuntime::new(config),
            out,
            records,
        }
    }

    /// `msg` が届く。そのメッセージで出た指令を返す。
    pub(super) fn send(&mut self, msg: SerikoMsg) -> Vec<DisplayCommand> {
        let before = self.records.lock().unwrap().len();
        let flow = handle_message(
            &self.resolver,
            &self.bind_resolver,
            &mut self.states,
            &mut self.rt,
            &mut self.out,
            msg,
        );
        assert_eq!(flow, ControlFlow::Continue(()));
        self.records.lock().unwrap()[before..].to_vec()
    }

    pub(super) fn tick(&mut self, now: TimePoint) -> Vec<DisplayCommand> {
        self.send(SerikoMsg::Tick { now })
    }

    /// cue が届く。
    pub(super) fn hear(&mut self, cue: TalkCue) -> Vec<DisplayCommand> {
        self.send(SerikoMsg::Cue(cue))
    }

    /// 時刻 `now` に発火した cue が届く（[`fired`]）。
    pub(super) fn hear_fired(&mut self, now: TimePoint, cue: TalkCue) -> Vec<DisplayCommand> {
        self.hear(fired(now, cue))
    }

    /// これまでに出た指令の全部。
    pub(super) fn all(&self) -> Vec<DisplayCommand> {
        self.records.lock().unwrap().clone()
    }
}

/// 非正準 params の `Custom` cue（`params` が非 Array＝`as_command_carrier()==None`・宛名規律検証用）。
pub(super) fn noncanonical_custom_cue(scope: &str, command: &str) -> TalkCue {
    let cue = cue(
        TimeSpan::ZERO,
        scope,
        CueCommand::Custom {
            command: command.to_string(),
            params: dola::DynamicValue::Null,
        },
        TimeSpan::ZERO,
    );
    // 前提: この構成は必ずキャリア開封に失敗する。
    assert!(
        cue.command.as_command_carrier().is_none(),
        "非 Array params の Custom は as_command_carrier() が None（宛名規律の入口）"
    );
    cue
}

/// 指令（どれも `Show`）ごとの（スコープ, animation `anim` の欄の絵）。
pub(super) fn frames(cmds: &[DisplayCommand], anim: u32) -> Vec<(&str, Option<u32>)> {
    cmds.iter()
        .map(|c| match c {
            DisplayCommand::Show { scope, pattern, .. } => {
                (scope.as_str(), pattern.get(anim).map(|f| f.surface_id))
            }
            other => panic!("Show を期待: {other:?}"),
        })
        .collect()
}

/// ただ 1 件の `Show` の (面, 着せ替え, コマの状態)。
pub(super) fn single_show(mut cmds: Vec<DisplayCommand>) -> (u32, BindSet, PatternState) {
    assert_eq!(cmds.len(), 1, "発行は 1 件だけ: {cmds:?}");
    match cmds.remove(0) {
        DisplayCommand::Show {
            surface_id,
            binds,
            pattern,
            ..
        } => (surface_id, binds, pattern),
        other => panic!("Show を期待: {other:?}"),
    }
}

/// 一番上の欄を (animation の番号, コマの番号) の列で読む。
pub(super) fn top_frames(pattern: &PatternState) -> Vec<(u32, u32)> {
    pattern.iter().map(|(id, f)| (id, f.surface_id)).collect()
}

/// `capture_text` の変種: `f` の戻り値も併せて返す（同期 handler の `ControlFlow` 表明用）。
///
/// 捕捉層は `capture_text` と同一（共有機構 `log-capture-kit` へ委譲）で、`f` が発火した
/// log 文字列と `f` の戻り値を組で返す。重複ハーネスを作らない。
pub(super) fn capture_logs_flow<T, F: FnOnce() -> T>(f: F) -> (String, T) {
    let (ret, lines) = capture_lines(LineFormat::LevelTargetFields, f);
    (lines.join("\n"), ret)
}

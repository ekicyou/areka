use super::*;
use crate::output::MockSurfaceOutput;
use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::ActorKey;
use log_capture_kit::{LineFormat, capture_lines};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// テスト用の TalkCue（Shell 系 Emote・at/actor 込み）を組む。
pub(super) fn emote_cue(at: f64, scope: &str, key: &str) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::Emote { key: key.into() },
        duration: 0.0, // 表情切替は瞬時（明示的 0）。
    }
}

/// 同期 `handle_message` 用の小さな解決層（"通常"→2100 の 1 件のみ）。
pub(super) fn tiny_resolver() -> SurfaceResolver {
    let mut aliases: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    aliases.insert("通常".to_string(), vec![2100]);
    SurfaceResolver::new(aliases)
}

/// 非空の静的 bind 集合を持つ空スコープ状態。
pub(super) fn fresh_states() -> ScopeStates {
    ScopeStates::new(BindSet::from_ids([1100, 1207]))
}

/// 不活性なループ統括器（空表＋ダミー乱数）。cue/bind/balloon の同期 `handle_message` 檻で
/// tick 経路を触らない既存挙動を保つための足場（`disabled()` は on_tick 常時空・on_surface_changed
/// は空 playback への no-op ゆえ、既存の発行/ログ挙動と byte 同値）。
pub(super) fn inert_runtime() -> LoopRuntime {
    LoopRuntime::new(SerikoLoopConfig::disabled())
}

/// 同期 `handle_message` に cue と刻みを流す足場（統括器に手で進める偽の時計を注入する）。どの
/// メッセージもその直前に時計を合わせる（観測を追い越さない）。
pub(super) struct ClockRig {
    resolver: SurfaceResolver,
    bind_resolver: BindResolver,
    states: ScopeStates,
    rt: LoopRuntime,
    out: MockSurfaceOutput,
    records: Arc<Mutex<Vec<DisplayCommand>>>,
    clock: Arc<AtomicU64>,
}

impl ClockRig {
    /// 別名の表なし・着せ替えなしで、統括器を `config` から組む。
    pub(super) fn with_config(config: SerikoLoopConfig) -> Self {
        let clock = Arc::new(AtomicU64::new(0));
        let read = Arc::clone(&clock);
        let out = MockSurfaceOutput::new();
        let records = out.records();
        Self {
            resolver: SurfaceResolver::new(BTreeMap::new()),
            bind_resolver: BindResolver::empty(),
            states: ScopeStates::new(BindSet::from_ids([])),
            rt: LoopRuntime::new(config)
                .with_clock(Some(Arc::new(move || read.load(Ordering::SeqCst)))),
            out,
            records,
            clock,
        }
    }

    /// 時計が `ms` のときに `msg` が届く。そのメッセージで出た指令を返す。
    pub(super) fn send(&mut self, ms: u64, msg: SerikoMsg) -> Vec<DisplayCommand> {
        self.clock.store(ms, Ordering::SeqCst);
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

    pub(super) fn tick(&mut self, ms: u64) -> Vec<DisplayCommand> {
        self.send(ms, SerikoMsg::Tick { now_ms: ms })
    }

    /// 時計が `ms` のときに cue が届く。
    pub(super) fn hear(&mut self, ms: u64, cue: TalkCue) -> Vec<DisplayCommand> {
        self.send(ms, SerikoMsg::Cue(cue))
    }
}

pub(super) fn cue(at: f64, scope: &str, command: CueCommand, duration: f64) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command,
        duration,
    }
}

/// 台本の `at` 秒に始まる文字の cue（1 字 50 ms）。
pub(super) fn text(at: f64, scope: &str, s: &str) -> TalkCue {
    let duration = s.chars().count() as f64 * 0.05;
    cue(at, scope, CueCommand::Text(s.into()), duration)
}

/// 台詞の頭に前置される全消去（台本の 0 秒）。
pub(super) fn clear_all() -> TalkCue {
    cue(0.0, "0", CueCommand::ClearAll, 0.0)
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

/// 部品 `part` の欄を (animation の番号, コマの番号) の列で読む。
pub(super) fn part_frames(pattern: &PatternState, part: u32) -> Vec<(u32, u32)> {
    pattern
        .part(part)
        .map(|(id, f)| (id, f.surface_id))
        .collect()
}

/// 一番上の欄を (animation の番号, コマの番号) の列で読む。
pub(super) fn top_frames(pattern: &PatternState) -> Vec<(u32, u32)> {
    pattern.iter().map(|(id, f)| (id, f.surface_id)).collect()
}

/// `capture_logs` の変種: `f` の戻り値も併せて返す（同期 handler の `ControlFlow` 表明用）。
///
/// 捕捉層は `capture_logs` と同一（共有機構 `log-capture-kit` へ委譲）で、`f` が発火した
/// log 文字列と `f` の戻り値を組で返す。重複ハーネスを作らない。
pub(super) fn capture_logs_flow<T, F: FnOnce() -> T>(f: F) -> (String, T) {
    let (ret, lines) = capture_lines(LineFormat::LevelTargetFields, f);
    (lines.join("\n"), ret)
}

/// テスト専用 tracing 捕捉ハーネス（硬化機構の唯一の定義元 `log-capture-kit` へ委譲）。
/// 1 イベント 1 行へ level／target／各フィールド（`name=value`）を整形し、改行連結で返す。
///
/// **「`with_default` はスレッドローカルだから並行実行でも干渉しない」は誤り**である。
/// 差し替わるのはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める
/// callsite の interest キャッシュはプロセス全体で 1 つしかなく、その発行点をプロセス内で
/// 最初に踏んだスレッドの判定が焼き付く。捕捉窓を持たないスレッド（既定は `NoSubscriber`）が
/// 先に踏むと `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。
/// 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 窓の内側での interest 再計算 ⑶ 番兵イベントに
/// よる空振り検出（捕捉できなければ panic）の 3 点でこれを塞ぐ。機序の逐条解説と
/// `tracing-core` の実コード引用は `log_capture_kit` の crate doc および同 crate の
/// `src/probe.rs` にある。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> String {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines.join("\n")
}

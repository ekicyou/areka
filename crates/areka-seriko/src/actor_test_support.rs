use super::*;
use areka_emo_compose::BindSet;
use areka_sakura::ActorKey;
use std::collections::BTreeMap;

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

/// `capture_logs` の変種: `f` の戻り値も併せて返す（同期 handler の `ControlFlow` 表明用）。
///
/// スレッドローカル `with_default` 直下で `f` を実行し、`f` が発火した log 文字列と `f` の
/// 戻り値を組で返す。既存 `capture_logs` と同一の捕捉層を用いる（重複ハーネスを作らない）。
pub(super) fn capture_logs_flow<T, F: FnOnce() -> T>(f: F) -> (String, T) {
    use std::cell::RefCell;
    let ret: RefCell<Option<T>> = RefCell::new(None);
    let logs = capture_logs(|| {
        *ret.borrow_mut() = Some(f());
    });
    (logs, ret.into_inner().expect("f は必ず値を返す"))
}

/// テスト専用 tracing 捕捉ハーネス（emo-compose/kanade の log_capture 流儀・
/// スレッドローカル `with_default` ゆえ並行テスト安全）。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> String {
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            ev: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let meta = ev.metadata();
            let mut line = format!("level={} target={}", meta.level(), meta.target());
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut line));
            self.0.lock().unwrap().push(line);
        }
    }

    // 並行実行下の callsite interest 毒化対策（`log_interest_probe` のモジュール doc 参照）。
    crate::log_interest_probe::ensure_interest_probes();

    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    tracing::subscriber::with_default(subscriber, || {
        // probe 常駐前に焼かれた `never` の掃き残しを、窓が開いた後にもう一度潰す。
        tracing::callsite::rebuild_interest_cache();
        f()
    });
    let guard = logs.lock().unwrap();
    guard.join("\n")
}

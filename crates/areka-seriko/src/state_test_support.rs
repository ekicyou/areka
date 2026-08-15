use super::*;

/// テスト用 bind 集合（非空・emo2 実測相当の任意 id）。
pub(super) fn binds_1100_1207() -> BindSet {
    BindSet::from_ids([1100, 1207])
}

pub(super) fn empty_states() -> ScopeStates {
    ScopeStates::new(binds_1100_1207())
}

/// テスト専用 tracing 捕捉ハーネス（actor/table の同名ヘルパと同一流儀・スレッドローカル
/// `with_default` ゆえ並行テスト安全）。1 イベント 1 行へ level／target／各フィールド
/// （`name=value`）を整形し、改行連結で返す。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> String {
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(&self, ev: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
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

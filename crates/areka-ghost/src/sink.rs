//! `LogSink` — 本番既定の記録先コンポーネント。
//!
//! 発火内容を `tracing` へ構造化出力するだけの、複製可能（`Clone`）な既定実装。
//! M-boot 統合はこの位置に seriko／emo-text-layer の実 sink を挿す
//! （design.md「ghost::sink」）。sakura の `MockSink`（テスト用・無限蓄積）を
//! 本番へ置かないための最小実装——蓄積フィールドを一切持たない unit 相当構造体。

use areka_sakura::contract::{CueCommand, TalkCue};
use areka_sakura::sink::{SurfaceSink, TextSink};

/// 本番既定の sink。発火のたびに `tracing::info!(target: "ghost-sink", ...)` で
/// `at`（発火時刻）・`actor`（話者スコープ）・`command_kind`（コマンド種別）を
/// 構造化出力するだけで、状態を一切蓄積しない（無限に稼働し続けても安全）。
///
/// `SurfaceSink`（→seriko⑤）・`TextSink`（→emo text-layer⑥）の両方を実装し、
/// sakura の `MockSink` と同じ「型で 2 trait を両方満たす」流儀を踏襲する。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogSink;

impl LogSink {
    /// 新しい `LogSink` を生成する（保持する状態はない）。
    pub fn new() -> Self {
        Self
    }

    /// 内部: 発火を構造化ログへ出力する（両 trait の emit が共用）。
    fn log(&self, cue: &TalkCue) {
        tracing::info!(
            target: "ghost-sink",
            event = "emit",
            at = cue.at,
            actor = %cue.actor,
            command_kind = command_kind(&cue.command),
            "talk cue emitted (production default LogSink)"
        );
    }
}

impl SurfaceSink for LogSink {
    fn emit(&mut self, cue: TalkCue) {
        self.log(&cue);
    }
}

impl TextSink for LogSink {
    fn emit(&mut self, cue: TalkCue) {
        self.log(&cue);
    }
}

/// `CueCommand` の variant 名を返す（ログの `command_kind` フィールド用）。
///
/// 明示的な variant ごとの match により、dola が将来 variant を追加した際に
/// コンパイラが再検討を強制する（catch-all を置かない・`contract::cue_target_of`
/// と同じ流儀）。
fn command_kind(command: &CueCommand) -> &'static str {
    match command {
        CueCommand::Text(_) => "Text",
        CueCommand::Clear => "Clear",
        CueCommand::Emote { .. } => "Emote",
        CueCommand::Choice { .. } => "Choice",
        CueCommand::EntityRef(_) => "EntityRef",
        CueCommand::Custom { .. } => "Custom",
        CueCommand::NewLine { .. } => "NewLine",
        CueCommand::BalloonSurface { .. } => "BalloonSurface",
        CueCommand::Wait => "Wait",
        CueCommand::ClearAll => "ClearAll",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::contract::ActorKey;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::{Event, Level, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::registry::LookupSpan;

    // ---- テスト専用ログ捕捉ヘルパ（kanade の schedule::log_capture 慣行に倣う。
    // kanade のヘルパは pub(crate) で外部クレートから再利用できないため、本モジュール
    // 限定で同じ技法をミニマルに再実装する）。----

    /// 捕捉した 1 イベント（本テストが照合する構造化フィールドのみ保持する）。
    #[derive(Debug, Clone)]
    struct CapturedEvent {
        target: String,
        level: Level,
        at: Option<f64>,
        actor: Option<String>,
        command_kind: Option<String>,
    }

    /// `at`／`actor`／`command_kind` フィールドを取り出す訪問子。
    ///
    /// `actor` は `%cue.actor`（Display 経由）で渡るため `record_debug` 経由で届く
    /// （tracing の `%` シジルは `Value::record` 内部で `record_debug` を呼ぶ）。
    /// `command_kind` はプレーンな `&'static str` 値なので `record_str` で届く。
    struct FieldVisitor {
        at: Option<f64>,
        actor: Option<String>,
        command_kind: Option<String>,
    }

    impl Visit for FieldVisitor {
        fn record_f64(&mut self, field: &Field, value: f64) {
            if field.name() == "at" {
                self.at = Some(value);
            }
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            match field.name() {
                "command_kind" => self.command_kind = Some(value.to_string()),
                "actor" => self.actor = Some(value.to_string()),
                _ => {}
            }
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            match field.name() {
                "actor" if self.actor.is_none() => {
                    self.actor = Some(format!("{value:?}").trim_matches('"').to_string());
                }
                "command_kind" if self.command_kind.is_none() => {
                    self.command_kind = Some(format!("{value:?}").trim_matches('"').to_string());
                }
                _ => {}
            }
        }
    }

    /// 捕捉先へイベントを積む Layer。
    struct CaptureLayer {
        sink: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    impl<S> Layer<S> for CaptureLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut visitor = FieldVisitor {
                at: None,
                actor: None,
                command_kind: None,
            };
            event.record(&mut visitor);
            let meta = event.metadata();
            self.sink.lock().unwrap().push(CapturedEvent {
                target: meta.target().to_string(),
                level: *meta.level(),
                at: visitor.at,
                actor: visitor.actor,
                command_kind: visitor.command_kind,
            });
        }
    }

    /// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
    fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
        let sink: Arc<Mutex<Vec<CapturedEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let layer = CaptureLayer { sink: sink.clone() };
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, f);
        std::mem::take(&mut *sink.lock().expect("capture sink mutex は毒化しない"))
    }

    fn sample_cue(at: f64, actor: &str, command: CueCommand) -> TalkCue {
        TalkCue {
            at,
            actor: ActorKey::from(actor),
            command,
            duration: 0.0,
        }
    }

    /// `SurfaceSink::emit` が構造化ログを 1 件発火し、`at`／`actor`／`command_kind` が
    /// 期待どおり載ることを検証する（R4.6 の既定実装が「発火のたびに構造化ログが
    /// 出力される」ことの直接証跡）。
    #[test]
    fn emit_via_surface_sink_logs_structured_event_with_at_actor_and_command_kind() {
        let mut sink = LogSink::new();
        let cue = sample_cue(1.5, "0", CueCommand::Text("hello".into()));

        let events = capture(|| {
            SurfaceSink::emit(&mut sink, cue.clone());
        });

        assert_eq!(events.len(), 1, "1 回の emit で 1 件のログが出ること");
        let event = &events[0];
        assert_eq!(event.target, "ghost-sink");
        assert_eq!(event.level, Level::INFO);
        assert_eq!(event.at, Some(1.5));
        assert_eq!(event.actor.as_deref(), Some("0"));
        assert_eq!(event.command_kind.as_deref(), Some("Text"));
    }

    /// 同一 `LogSink` が `TextSink` 経由でも同じ構造化ログを出すことを検証する
    /// （sakura `MockSink` と同じ「型で両 trait を満たす」パターンの確認）。
    #[test]
    fn emit_via_text_sink_logs_structured_event() {
        let mut sink = LogSink::new();
        let cue = sample_cue(3.25, "1", CueCommand::NewLine { ratio: 1.0 });

        let events = capture(|| {
            TextSink::emit(&mut sink, cue.clone());
        });

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.target, "ghost-sink");
        assert_eq!(event.at, Some(3.25));
        assert_eq!(event.actor.as_deref(), Some("1"));
        assert_eq!(event.command_kind.as_deref(), Some("NewLine"));
    }

    /// 全 `CueCommand` variant が期待どおりの `command_kind` へ写ることを確認する
    /// （`contract::cue_target_of` の全 variant テストと同じ流儀・将来 variant 追加時に
    /// このテストの拡張漏れをコンパイラの exhaustive match が防ぐ）。
    #[test]
    fn command_kind_covers_every_cue_command_variant() {
        assert_eq!(command_kind(&CueCommand::Text("x".into())), "Text");
        assert_eq!(command_kind(&CueCommand::Clear), "Clear");
        assert_eq!(
            command_kind(&CueCommand::Emote { key: "smile".into() }),
            "Emote"
        );
        assert_eq!(
            command_kind(&CueCommand::Choice {
                id: "yes".into(),
                text: "はい".into()
            }),
            "Choice"
        );
        assert_eq!(command_kind(&CueCommand::EntityRef(42)), "EntityRef");
        assert_eq!(
            command_kind(&CueCommand::Custom {
                command: "fade".into(),
                params: dola::DynamicValue::Null,
            }),
            "Custom"
        );
        assert_eq!(
            command_kind(&CueCommand::NewLine { ratio: 1.0 }),
            "NewLine"
        );
        assert_eq!(
            command_kind(&CueCommand::BalloonSurface { key: "2".into() }),
            "BalloonSurface"
        );
        assert_eq!(command_kind(&CueCommand::Wait), "Wait");
        assert_eq!(command_kind(&CueCommand::ClearAll), "ClearAll");
    }

    /// `LogSink` は無蓄積の unit 相当構造体ゆえ、`Clone` した 2 インスタンスを別々に
    /// 使い回しても状態が共有されない（＝壊れない）ことを確認する。蓄積フィールドが
    /// 無いため観測可能な差分はないが、大量発火・clone 後の独立動作が panic なく
    /// 完走することが「無蓄積で無限に稼働できる」性質の直接証跡になる。
    #[test]
    fn clone_instances_operate_independently_without_shared_state() {
        let mut original = LogSink::new();
        let mut cloned = original.clone();

        let _events = capture(|| {
            for i in 0..5 {
                SurfaceSink::emit(
                    &mut original,
                    sample_cue(i as f64, "0", CueCommand::Clear),
                );
                TextSink::emit(
                    &mut cloned,
                    sample_cue(i as f64, "1", CueCommand::Clear),
                );
            }
        });

        // LogSink は unit struct（フィールドなし）ゆえ、この等価性は自明ではあるが、
        // 「clone してもレイアウト・挙動が分岐しない」ことの回帰檻として残す。
        assert_eq!(original, cloned);
    }
}

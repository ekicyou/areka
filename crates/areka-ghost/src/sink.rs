//! `LogSink`／`DiscardSink` — 診断既定の記録先コンポーネント（単一出力契約 [`dola::cue::CueSink`]）。
//!
//! `LogSink` は発火内容を `tracing` へ構造化出力するだけの、複製可能（`Clone`）な診断既定実装。
//! M-boot 統合はこの位置に seriko／emo-text-layer の実 sink を挿す（design.md「ghost::sink」）。
//! 旧 sakura の `MockSink`（テスト用・無限蓄積）を本番へ置かないための最小実装——蓄積フィールドを
//! 一切持たない unit 相当構造体。
//!
//! `boot` は可変長 sink（`Vec<Box<dyn BootCueSink>>`・S-3）を持つが、診断既定では登録列の
//! **先頭だけ**を `LogSink` で記録し、残りは破棄専用の `DiscardSink` で埋める。broadcast（D4）で
//! 全 cue は登録された全 sink へ配られるため、複数スロットを `LogSink` にすると 1 cue が複数回
//! ログされてしまう（二重ログ）。`LogSink`＋`DiscardSink` の対（診断既定 `vec![LogSink, DiscardSink]`
//! 相当）で **cue ごと 1 回ログ**へ正す（設計 D4「挙動保存の唯一の破れ」Topic 2）。
//!
//! # `BootCueSink`（S-3 可変長 sink の boot 契約）
//!
//! `boot` が受け取る sink は演者数に依らない可変長 `Vec<Box<dyn BootCueSink>>` である
//! （旧「2 固定スロット（surface/text）」の意図的更新）。[`BootCueSink`] は
//! `CueSink + Clone + Send + 'static` へ blanket impl されるため、`LogSink`／`DiscardSink` 等の
//! 既存 sink は**無改変で適合**する。dispatcher は talk 起動ごとに [`BootCueSink::clone_box`] で
//! 各 sink の独立インスタンスを取得し、per-talk の `spawn_talk` へ `Vec<Box<dyn CueSink + Send>>`
//! として手渡す（登録順＝broadcast 順＝決定論・design.md「GhostBootOptions S-3」）。

use areka_sakura::contract::{CueCommand, CueSink, TalkCue};

/// `boot` が要求する複製可能 sink（design.md「GhostBootOptions S-3＋provider」）。
///
/// `CueSink + Clone + Send + 'static` を満たす全ての型へ blanket impl されるため、
/// 既存の [`LogSink`]／[`DiscardSink`] や演者 sink（seriko／emo-text）は本 trait を
/// **何も実装せずに**満たす。dispatcher は保持する `Vec<Box<dyn BootCueSink>>` の各要素を
/// talk 起動ごとに [`clone_box`](BootCueSink::clone_box) で複製し、per-talk の `spawn_talk` へ
/// `Vec<Box<dyn CueSink + Send>>` として手渡す（凍結像の刻印点・登録順＝broadcast 順）。
///
/// 上位境界 `CueSink + Send` を持つため、`Box<dyn BootCueSink>` は
/// `Box<dyn CueSink + Send>` へ trait upcast できる（per-talk 手渡し時の型合わせ）。
pub trait BootCueSink: CueSink + Send {
    /// 自身の複製を trait object として返す（per-talk 複製の口・`dyn` 化のため
    /// `Clone` を直接 supertrait に置けないための clone シム）。
    fn clone_box(&self) -> Box<dyn BootCueSink>;
}

impl<T> BootCueSink for T
where
    T: CueSink + Clone + Send + 'static,
{
    fn clone_box(&self) -> Box<dyn BootCueSink> {
        Box::new(self.clone())
    }
}

/// 診断既定の sink。発火のたびに `tracing::info!(target: "ghost-sink", ...)` で
/// `at`（発火時刻）・`actor`（話者スコープ）・`command_kind`（コマンド種別）を
/// 構造化出力するだけで、状態を一切蓄積しない（無限に稼働し続けても安全）。
///
/// 演者非依存の**単一の出力契約** [`dola::cue::CueSink`] を実装する（`boot` の broadcast 登録先が
/// 要求する形・R11.3/R11.6）。旧 `SurfaceSink`/`TextSink` の 2 分割は broadcast＋演者側 relevance
/// ゆえ廃した。broadcast された全 cue を構造化ログへ落とすだけの診断既定 sink。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogSink;

impl LogSink {
    /// 新しい `LogSink` を生成する（保持する状態はない）。
    pub fn new() -> Self {
        Self
    }

    /// 内部: 発火を構造化ログへ出力する。
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

impl CueSink for LogSink {
    fn emit(&mut self, cue: TalkCue) {
        self.log(&cue);
    }
}

/// 破棄専用の診断 sink（演者非依存の単一出力契約 [`dola::cue::CueSink`] を実装する）。
///
/// `boot` の可変長 sink 列（`Vec<Box<dyn BootCueSink>>`・S-3）のうち、診断既定の非記録スロットを
/// 埋めるための no-op sink。broadcast された cue を受け取っても何もしない（記録も出力もしない・
/// 状態を持たない）。診断既定 boot は `vec![LogSink, DiscardSink]` 相当で結線し、`LogSink` を単一の
/// 記録先とすることで **cue ごと 1 回ログ**を成立させる（複数スロットを `LogSink` にした場合の二重ログを
/// 避ける・設計 D4 Topic 2）。本番の実 sink 差込（seriko/emo-text）では別オブジェクトゆえ元々無関係。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiscardSink;

impl DiscardSink {
    /// 新しい `DiscardSink` を生成する（保持する状態はない）。
    pub fn new() -> Self {
        Self
    }
}

impl CueSink for DiscardSink {
    fn emit(&mut self, _cue: TalkCue) {
        // 破棄のみ（診断既定 boot の非記録スロットを埋める・記録も出力もしない）。
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
        // task 6.1 で exhaustive を回復させるための最小アーム（他の内容 cue と同格の
        // 良性ログラベル）。Cursor 専用の消費・分類（CAGE）は task 8 の領分——ここでは
        // command_kind ログの網羅性のみ回復させる（1.2/1.4 のアーム先行・cage 後追いと同型）。
        CueCommand::Cursor { .. } => "Cursor",
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

    /// `CueSink::emit` が構造化ログを 1 件発火し、`at`／`actor`／`command_kind` が
    /// 期待どおり載ることを検証する（診断既定実装が「発火のたびに構造化ログが
    /// 出力される」ことの直接証跡・単一の出力契約 [`dola::cue::CueSink`]）。
    #[test]
    fn emit_logs_structured_event_with_at_actor_and_command_kind() {
        let mut sink = LogSink::new();
        let cue = sample_cue(1.5, "0", CueCommand::Text("hello".into()));

        let events = capture(|| {
            CueSink::emit(&mut sink, cue.clone());
        });

        assert_eq!(events.len(), 1, "1 回の emit で 1 件のログが出ること");
        let event = &events[0];
        assert_eq!(event.target, "ghost-sink");
        assert_eq!(event.level, Level::INFO);
        assert_eq!(event.at, Some(1.5));
        assert_eq!(event.actor.as_deref(), Some("0"));
        assert_eq!(event.command_kind.as_deref(), Some("Text"));
    }

    /// 別の cue（`NewLine`）でも `CueSink::emit` が同じ構造化ログを出すことを検証する。
    #[test]
    fn emit_logs_structured_event_for_newline_cue() {
        let mut sink = LogSink::new();
        let cue = sample_cue(3.25, "1", CueCommand::NewLine { ratio: 1.0 });

        let events = capture(|| {
            CueSink::emit(&mut sink, cue.clone());
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
                text: "はい".into(),
                references: vec![],
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
                CueSink::emit(&mut original, sample_cue(i as f64, "0", CueCommand::Clear));
                CueSink::emit(&mut cloned, sample_cue(i as f64, "1", CueCommand::Clear));
            }
        });

        // LogSink は unit struct（フィールドなし）ゆえ、この等価性は自明ではあるが、
        // 「clone してもレイアウト・挙動が分岐しない」ことの回帰檻として残す。
        assert_eq!(original, cloned);
    }

    /// 診断既定の**単一配線**（`LogSink`＋`DiscardSink`）を dola `CuePlayer` の broadcast fan-out
    /// （live path が `spawn_talk` で使う配送エンジンそのもの）へ通し、表情切替（`Emote`）を含む
    /// 台本を流すと、出力先ログが **cue ごとにちょうど 1 回**記録されることを固定する
    /// （設計 D4「挙動保存の唯一の破れ」Topic 2＝二重ログの解消）。
    ///
    /// 決定論のため実アクター（`spawn_talk`）でなく `CuePlayer` を**テストスレッド上で**同期駆動
    /// する（`emit`＝ログ発火が本スレッドで起こり、スレッドローカル `capture` で捕捉できる）。
    /// `CuePlayer` の broadcast fan-out は `spawn_talk` が組む配送機構と同一ゆえ live 配線を忠実に
    /// 再現する。診断既定は片スロットのみ `LogSink`（記録）・もう一方は `DiscardSink`（破棄）ゆえ、
    /// 全 cue が両スロットへ broadcast されても記録は 1 系統だけ＝cue ごと 1 回ログになる。
    ///
    /// 弁別（二重配線なら FAIL）: もし診断既定が両スロットを `LogSink` にしていた場合、同一 cue が
    /// 2 回ログされ、`by_kind` の各件数が 2 になって本 assert が落ちる（＝二重ログの回帰檻）。
    #[test]
    fn diagnostic_default_wiring_logs_each_cue_exactly_once_through_broadcast() {
        use dola::cue::CuePlayer;

        // 表情切替（\s[0]）を含む最小台本を compile → 刻印 → CuePlayer 構築。
        // 期待 cue: ClearAll@0（#6 冒頭前置）/ Emote{0}@0（\s[0]）/ Text(hello)@0。
        let instructions = areka_parsers::sakura::parse(r"\s[0]hello\e");
        // task 6.1 の機械的追随: compile は task 5.1 で 2 引数化（凍結像 SystemVarSnapshot を
        // 参照渡し）。本テストはシステム変数展開を検査しないため既定スナップショットを渡す。
        let compiled = areka_sakura::compile(&instructions, &areka_sakura::SystemVarSnapshot::default());
        let sheet = compiled.sheet.with_absolute_start_time(0.0);

        let mut player = CuePlayer::from_sheet(&sheet);
        // 診断既定の単一配線: surface スロット＝LogSink（記録）、text スロット＝DiscardSink（破棄）。
        player.register_sink(Box::new(LogSink::new()));
        player.register_sink(Box::new(DiscardSink::new()));

        // 初回 Tick(0.0) で刻印済みアンカーの発火群を broadcast、占有 horizon を跨ぐ Tick(1.0) で全 due。
        let events = capture(|| {
            player.tick(0.0);
            player.tick(1.0);
        });

        // ghost-sink（LogSink）の emit ログのみ抽出。DiscardSink はログを出さないため、
        // 記録系統は LogSink 1 本＝各 cue はちょうど 1 回だけログされる（二重でない）。
        let logs: Vec<&CapturedEvent> = events
            .iter()
            .filter(|e| e.target == "ghost-sink")
            .collect();

        // command_kind ごとの件数を数える（各 presentation cue が 1 回だけ・2 回でない）。
        let count_kind = |kind: &str| {
            logs.iter()
                .filter(|e| e.command_kind.as_deref() == Some(kind))
                .count()
        };
        assert_eq!(
            count_kind("ClearAll"),
            1,
            "ClearAll cue は診断既定の単一配線でちょうど 1 回ログされる（二重ログでない）: {logs:?}"
        );
        assert_eq!(
            count_kind("Emote"),
            1,
            "表情切替（Emote）cue はちょうど 1 回ログされる（テキスト出力先も broadcast 受信するが記録は 1 系統）: {logs:?}"
        );
        assert_eq!(
            count_kind("Text"),
            1,
            "Text cue はちょうど 1 回ログされる: {logs:?}"
        );
        assert_eq!(
            logs.len(),
            3,
            "台本の 3 cue（ClearAll/Emote/Text）が過不足なく各 1 回ログされる（合計 3・二重配線なら 6）: {logs:?}"
        );
    }
}

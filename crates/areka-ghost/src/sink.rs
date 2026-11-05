//! `LogSink`／`DiscardSink` — 診断既定の記録先コンポーネント（単一出力契約 [`areka_sakura::contract::CueSink`]（＝`dola::cue::CueSink`））。
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

use areka_sakura::contract::{CueCommand, CueSink, ScriptOrigin, TalkCue, TalkId};

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

    /// この複製が受け持つトークの番号と出どころ（`StartTalk::origin`）。dispatcher がトークの
    /// 起動ごとに、複製の直後に 1 回だけ呼ぶ（要件 2.6・1.4・design.md「begin_talk の出どころ」）。
    /// 既定は何もしない——一括の実装に乗る既存の受け口は振る舞いが変わらない。番号や出どころが
    /// 要る受け口は `Clone` を持たずに本 trait を自分で実装して上書きする。
    fn begin_talk(&mut self, _talk_id: TalkId, _origin: ScriptOrigin) {}
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
/// 演者非依存の**単一の出力契約** [`areka_sakura::contract::CueSink`]（＝`dola::cue::CueSink`）を実装する（`boot` の broadcast 登録先が
/// 要求する形・R11.3/R11.6）。旧 `SurfaceSink`/`TextSink` の 2 分割は broadcast＋演者側 relevance
/// ゆえ廃した。broadcast された全 cue を構造化ログへ落とすだけの診断既定 sink。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LogSink;

impl LogSink {
    /// 新しい `LogSink` を生成する（保持する状態はない）。
    pub fn new() -> Self {
        Self
    }
}

impl CueSink for LogSink {
    fn emit(&mut self, cue: TalkCue) {
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

/// 破棄専用の診断 sink（演者非依存の単一出力契約 [`areka_sakura::contract::CueSink`]（＝`dola::cue::CueSink`）を実装する）。
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
        CueCommand::AnchorBegin { .. } => "AnchorBegin",
        CueCommand::AnchorEnd => "AnchorEnd",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::contract::ActorKey;
    use log_capture_kit::CapturedEvent as CanonicalEvent;
    // `actor` は `%cue.actor`（Display 経由）で `record_debug` に届くので `field_str` では落ちる。
    use crate::test_log_capture::label_of;
    use tracing::Level;

    // ---- テスト専用ログ捕捉ヘルパ（捕捉窓は crate 内の唯一の窓口
    // `crate::test_log_capture::capture_events` へ寄せ、そこから共有 crate `log-capture-kit`
    // へ委譲する。以前ここにあった自前の `with_default` 捕捉は硬化を持たず、別スレッドが
    // 先に発行点を踏むと callsite の interest が `never` へ焼き付いて捕捉 0 件になり得た
    // ——存在の主張は確率的に赤、不在の主張は静かに緑になる形である。機序は
    // `log_capture_kit` の crate doc を参照）。----

    /// 捕捉した 1 イベント（本テストが照合する構造化フィールドのみ保持する）。
    #[derive(Debug, Clone)]
    struct CapturedEvent {
        target: String,
        level: Level,
        at: Option<f64>,
        actor: Option<String>,
        command_kind: Option<String>,
    }

    impl CapturedEvent {
        /// 共有機構の正準イベントから、本檻が照合する形へ変換する。
        ///
        /// `at` は `f64` 値ゆえ `Visit` の既定実装で `record_debug` へ落ち、その `{:?}` は
        /// `1.5`／`3.25` のような十進表記になる。移行前の `record_f64` と同じ値を得るために
        /// ここで `f64` へ戻す（数値でない値が載っていれば `None` になり、檻は移行前どおり落ちる）。
        fn from_canonical(ev: &CanonicalEvent) -> Self {
            Self {
                target: ev.target.clone(),
                level: ev.level,
                at: label_of(ev, "at").and_then(|value| value.parse::<f64>().ok()),
                actor: label_of(ev, "actor"),
                command_kind: label_of(ev, "command_kind"),
            }
        }
    }

    /// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
    fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
        crate::test_log_capture::capture_events(f)
            .iter()
            .map(CapturedEvent::from_canonical)
            .collect()
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
            command_kind(&CueCommand::Emote {
                key: "smile".into()
            }),
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
        assert_eq!(command_kind(&CueCommand::NewLine { ratio: 1.0 }), "NewLine");
        assert_eq!(
            command_kind(&CueCommand::BalloonSurface { key: "2".into() }),
            "BalloonSurface"
        );
        assert_eq!(
            command_kind(&CueCommand::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            }),
            "Cursor"
        );
        assert_eq!(command_kind(&CueCommand::Wait), "Wait");
        assert_eq!(command_kind(&CueCommand::ClearAll), "ClearAll");
        assert_eq!(
            command_kind(&CueCommand::AnchorBegin {
                id: "OnJump".into(),
                references: vec![],
            }),
            "AnchorBegin"
        );
        assert_eq!(command_kind(&CueCommand::AnchorEnd), "AnchorEnd");
    }
}

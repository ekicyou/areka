//! 決定論 spine e2e（design.md「spine e2e（決定論・純 x64）」）。
//!
//! 本ファイルは 2 つのテスト専用型を提供する（task 4.1 の成果物）:
//! - [`ScriptedShioriBackend`] — `areka_kanade::ShioriBackend` を実装する台本 fake。
//! - [`RecordingSink`] — 演者非依存の単一出力契約 `areka_sakura::contract::CueSink` を実装する、
//!   `Clone` 可能な記録 sink（broadcast で全 cue を受ける）。
//!
//! 後続タスク（4.2〜4.7）はこのファイルへ boot〜close の各シナリオ（S1〜S6）の
//! `#[test]` を追加していく。本タスク（4.1）はその土台となる 2 型自体の構築・検証
//! （台本通りの応答・終了結果・死活遷移を任意に再現できること）のみを担う。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use areka_kanade::ShioriBackend;
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

/// host-32 IPC 有界 e2e の壁時計安全弁（ハング検出器）。兄弟 e2e 規約（inproc/real_pasta/
/// snapshot_capture = 60s）へ整合。意味論 deadline は MonotonicMs 仮想時間で注入されるため
/// この壁時計値はテスト意味論に影響せず、workspace 並列負荷の飢餓による偽赤のみを防ぐ。
const E2E_BOUND: std::time::Duration = std::time::Duration::from_secs(60);

// ===================== ScriptedShioriBackend =====================

/// backend が受領した 1 呼出の記録（照合用・要件 7.1「発火内容を蓄積して照合できる」）。
///
/// `Get`/`Notify` は id・references を保持する（task-spec の「id + references 最低限」）。
/// `Unload`/`Status` は引数を持たないため variant のみで足りる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedCall {
    /// GET 呼出（応答を要するイベント）。
    Get { id: String, references: Vec<String> },
    /// NOTIFY 呼出（片道イベント）。
    Notify { id: String, references: Vec<String> },
    /// unload（正規 clean shutdown）呼出。
    Unload,
    /// status（非ブロッキング死活問い合わせ）呼出。
    Status,
}

/// [`ScriptedShioriBackend`] を組み立てるビルダー（台本の事前登録）。
///
/// GET/NOTIFY は id ごとに応答列（`VecDeque`）を積み上げ、呼出のたびに先頭から 1 件
/// 消費する（`RequestError`/`ShutdownError` は `Clone` を実装しないため、値そのものを
/// 使い切り消費する設計にする——スクリプトの再利用は想定しない）。
pub struct ScriptedShioriBackendBuilder {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    initial_status: HelperStatus,
}

impl ScriptedShioriBackendBuilder {
    /// 空の台本（既定 status=`Running`）から開始する。
    pub fn new() -> Self {
        Self {
            get_scripts: HashMap::new(),
            notify_scripts: HashMap::new(),
            unload_script: None,
            initial_status: HelperStatus::Running,
        }
    }

    /// `id` に対する GET 応答を 1 件、応答列の末尾へ積む（複数回呼べば FIFO に消費される）。
    pub fn get(
        mut self,
        id: impl Into<String>,
        response: Result<Option<String>, RequestError>,
    ) -> Self {
        self.get_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `id` に対する NOTIFY 応答を 1 件、応答列の末尾へ積む。
    pub fn notify(mut self, id: impl Into<String>, response: Result<(), RequestError>) -> Self {
        self.notify_scripts
            .entry(id.into())
            .or_default()
            .push_back(response);
        self
    }

    /// `unload()` の結果を台本化する（一度きり消費・`Option::take` で払い出す）。
    pub fn unload(mut self, response: Result<ExitKind, ShutdownError>) -> Self {
        self.unload_script = Some(response);
        self
    }

    /// 初期 `status()` を台本化する（既定は `Running`）。
    pub fn status(mut self, status: HelperStatus) -> Self {
        self.initial_status = status;
        self
    }

    /// backend 本体（アクタースレッドへ move する側）と、テストが状態変更・照合に使う
    /// [`ScriptedShioriHandle`] のペアを構築する。
    pub fn build(self) -> (ScriptedShioriBackend, ScriptedShioriHandle) {
        let status = Arc::new(Mutex::new(self.initial_status));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let backend = ScriptedShioriBackend {
            get_scripts: self.get_scripts,
            notify_scripts: self.notify_scripts,
            unload_script: self.unload_script,
            status: Arc::clone(&status),
            calls: Arc::clone(&calls),
        };
        let handle = ScriptedShioriHandle { status, calls };
        (backend, handle)
    }
}

impl Default for ScriptedShioriBackendBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 台本化したテスト専用 SHIORI backend（`areka_kanade::ShioriBackend` 実装・要件 7.1/7.6）。
///
/// プロセス spawn・実窓・i686 成果物を一切要さない（純 x64・要件 7.6）。応答・終了結果は
/// [`ScriptedShioriBackendBuilder`] で事前登録し、`status()` は [`ScriptedShioriHandle`]
/// 経由でテスト自身のスレッドから途中差し替え可能（helper がシナリオ途中で死ぬ様子を
/// 再現するための capability・後続タスクの S3 が利用する）。
pub struct ScriptedShioriBackend {
    get_scripts: HashMap<String, VecDeque<Result<Option<String>, RequestError>>>,
    notify_scripts: HashMap<String, VecDeque<Result<(), RequestError>>>,
    unload_script: Option<Result<ExitKind, ShutdownError>>,
    status: Arc<Mutex<HelperStatus>>,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriBackend {
    /// ビルダー起点（[`ScriptedShioriBackendBuilder::new`] の別名）。
    pub fn builder() -> ScriptedShioriBackendBuilder {
        ScriptedShioriBackendBuilder::new()
    }
}

impl ShioriBackend for ScriptedShioriBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Get {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.get_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::get(\"{id}\"): no scripted response left (never configured or script exhausted)")
            })
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Notify {
                id: id.to_string(),
                references: references.to_vec(),
            });
        self.notify_scripts
            .get_mut(id)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| {
                panic!("ScriptedShioriBackend::notify(\"{id}\"): no scripted response left (never configured or script exhausted)")
            })
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Unload);
        self.unload_script.take().unwrap_or_else(|| {
            panic!("ScriptedShioriBackend::unload(): no scripted response configured")
        })
    }

    fn status(&mut self) -> HelperStatus {
        self.calls
            .lock()
            .expect("calls mutex poisoned")
            .push(RecordedCall::Status);
        *self.status.lock().expect("status mutex poisoned")
    }
}

/// [`ScriptedShioriBackend`] をテスト側から観測・操作するためのハンドル。
///
/// backend 本体（`Box<dyn ShioriBackend>` としてアクタースレッドへ move される）とは
/// 独立に、`Arc` 共有を通じて status の途中差し替え・呼出記録の照合を行える。
#[derive(Clone)]
pub struct ScriptedShioriHandle {
    status: Arc<Mutex<HelperStatus>>,
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl ScriptedShioriHandle {
    /// `status()` が以後返す値を差し替える（helper がシナリオ途中で死ぬ様子の再現）。
    /// テスト自身のスレッドから、backend が別スレッド（shiori actor）で生きている間に
    /// 呼べる。
    pub fn set_status(&self, status: HelperStatus) {
        *self.status.lock().expect("status mutex poisoned") = status;
    }

    /// 受領記録（`Arc` クローン）を返す。backend を別スレッドへ move した後も、
    /// このハンドルから発火列を照合できる。
    pub fn calls(&self) -> Arc<Mutex<Vec<RecordedCall>>> {
        Arc::clone(&self.calls)
    }
}

// ===================== RecordingSink =====================

/// テスト専用の `Clone` 可能な記録 sink（演者非依存の単一出力契約 [`CueSink`] を実装する）。
///
/// sakura の `MockSink`（`tests/ghost/` から見て他クレートの凍結面）とは同型だが
/// `Clone` を実装しない。dispatcher の per-talk 注入（`S: CueSink + Clone`/`T: CueSink + Clone`）
/// を満たすため、`tests/ghost/` 側で定義し直す（sakura の `sink.rs` には手を入れない・
/// design.md 「spine e2e」参照）。broadcast ゆえ登録された全 sink が全 cue を受ける
/// （surface/text スロットの別なく同一の全 cue が届く・演者側 relevance が action を選別する）。
#[derive(Clone)]
pub struct RecordingSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl RecordingSink {
    /// 空の共有蓄積を持つ sink を生成する。
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 共有蓄積の `Arc` クローンを返す。`RecordingSink` を clone してアクタースレッドへ
    /// 渡した後も、このハンドルから発火列を照合できる。
    pub fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }
}

impl Default for RecordingSink {
    fn default() -> Self {
        Self::new()
    }
}

impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("records mutex poisoned")
            .push(cue);
    }
}

// ===================== 本タスク（4.1）の証明テスト =====================
//
// 「台本通りの応答・終了結果・死活遷移を任意に再現できることを確認できる」
// （tasks.md 4.1 の観測可能な完了条件）を、6 シナリオで直接固定する。

#[cfg(test)]
mod tests {
    use super::*;

    /// シナリオ1: GET 応答（`Ok(Some)`）が台本どおり返り、呼出が記録されること。
    #[test]
    fn scripted_get_ok_some_returns_exact_value_and_is_recorded() {
        let (mut backend, handle) = ScriptedShioriBackend::builder()
            .get("OnBoot", Ok(Some("\\h\\s0hello\\e".to_string())))
            .build();

        let result = backend.get("OnBoot", &[], None);

        // `RequestError` は `PartialEq` を実装しないため（凍結面の消費のみ・機械的写像の
        // 都合）、`Result` 全体の `assert_eq!` はできない——`Ok` の中身を直接照合する。
        match result {
            Ok(Some(script)) => assert_eq!(script, "\\h\\s0hello\\e"),
            other => panic!("expected Ok(Some(..)), got {other:?}"),
        }

        let calls = handle.calls();
        let calls = calls.lock().expect("calls mutex poisoned");
        assert_eq!(
            &*calls,
            &vec![RecordedCall::Get {
                id: "OnBoot".to_string(),
                references: vec![],
            }]
        );
    }

    /// シナリオ2: GET 応答として台本化した失敗（`Err(RequestError::Timeout)`）が
    /// そのまま variant 一致で返ること。
    #[test]
    fn scripted_get_err_returns_exact_error_variant() {
        let (mut backend, _handle) = ScriptedShioriBackend::builder()
            .get("OnSecondChange", Err(RequestError::Timeout))
            .build();

        let result = backend.get("OnSecondChange", &[], None);

        match result {
            Err(RequestError::Timeout) => {}
            other => panic!("expected Err(RequestError::Timeout), got {other:?}"),
        }
    }

    /// シナリオ3: NOTIFY 応答が台本どおり返り、呼出が記録されること。
    #[test]
    fn scripted_notify_returns_exact_value_and_is_recorded() {
        let (mut backend, handle) = ScriptedShioriBackend::builder()
            .notify("OnCloseAll", Ok(()))
            .build();

        let references = vec!["reason".to_string()];
        let result = backend.notify("OnCloseAll", &references, None);

        assert!(result.is_ok(), "expected Ok(()), got {result:?}");

        let calls = handle.calls();
        let calls = calls.lock().expect("calls mutex poisoned");
        assert_eq!(
            &*calls,
            &vec![RecordedCall::Notify {
                id: "OnCloseAll".to_string(),
                references,
            }]
        );
    }

    /// シナリオ4: `unload()` の結果（`Ok(ExitKind::Clean)`）が台本どおり返ること。
    #[test]
    fn scripted_unload_returns_exact_exit_kind() {
        let (mut backend, _handle) = ScriptedShioriBackend::builder()
            .unload(Ok(ExitKind::Clean))
            .build();

        let result = backend.unload();

        assert_eq!(
            result.expect("scripted unload should be Ok"),
            ExitKind::Clean
        );
    }

    /// シナリオ5: 死活状態の遷移。初期 `status()` は台本どおり `Running` を返し、その後
    /// テストのスレッドから `handle.set_status` で `Exited(Abnormal(1))` へ差し替えると、
    /// 以降の `status()` 呼出はその新しい値を返す（helper がシナリオ途中で死ぬ様子を
    /// 「backend の外側・テスト自身」から駆動できることの直接証跡・要件 7.1）。
    #[test]
    fn status_transitions_from_running_to_exited_when_mutated_externally_mid_scenario() {
        let (mut backend, handle) = ScriptedShioriBackend::builder()
            .status(HelperStatus::Running)
            .build();

        assert_eq!(backend.status(), HelperStatus::Running);

        // シミュレート: helper がシナリオ途中で異常終了する（テスト自身の駆動）。
        handle.set_status(HelperStatus::Exited(ExitKind::Abnormal(1)));

        assert_eq!(
            backend.status(),
            HelperStatus::Exited(ExitKind::Abnormal(1)),
            "status() 呼出は途中差し替え後の値を反映しなければならない"
        );
    }

    /// シナリオ6: `RecordingSink` の clone 共有蓄積。2 つの clone それぞれから単一出力契約
    /// [`CueSink`] 経由で 1 件ずつ emit すると、同一の共有蓄積へ FIFO で積まれること
    /// （dispatcher が broadcast で全 sink（各 talk へ clone した surface/text スロット）へ
    /// 同一 cue を配る使い方を裏付ける）。
    #[test]
    fn recording_sink_clones_share_storage_in_fifo_order() {
        let sink = RecordingSink::new();
        let records = sink.records();

        let mut clone_a = sink.clone();
        let mut clone_b = sink.clone();

        let cue_a = TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("via clone a".to_string()),
            duration: 0.0,
        };
        let cue_b = TalkCue {
            at: 1.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("via clone b".to_string()),
            duration: 0.0,
        };

        CueSink::emit(&mut clone_a, cue_a.clone());
        CueSink::emit(&mut clone_b, cue_b.clone());

        let recorded = records.lock().expect("records mutex poisoned");
        assert_eq!(&*recorded, &vec![cue_a, cue_b]);
    }
}

// ===================== broadcast per-sink relevance partition（task 8.1） =====================
//
// design.md「Testing Strategy → Integration Tests → honor 契約 ③（relevance partition）」・
// Revalidation Trigger「`cue_target_of` を relevance の単一権威とし、各表現者の action 判定が
// `cue_target_of` の分類と一致すること（partition）を再確認する」。
//
// broadcast（D4）では全 cue が登録された全 sink（seriko/emo-text）へ配られる。どの action を
// 演じるかは演者側 relevance（`cue_target_of` が単一権威）が決めるため、**変異 variant ごとに
// action する演者は高々一つ**でなければならない（二重 action / 暗黙ドロップの発散を型で塞ぐ）。
// 本モジュールは `CueCommand` の全 10 variant について、`cue_target_of` の分類が
// 「Shell→seriko だけが action／Balloon→emo-text だけが action／None→誰も action しない」の
// partition になっていることを純関数として固定する（GPU 不要・決定論）。
#[cfg(test)]
mod broadcast_relevance_partition {
    use areka_sakura::contract::{CueCommand, CueTarget, cue_target_of};

    /// broadcast された cue に対して action する演者の同定（分類が単一権威 `cue_target_of`）。
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Performer {
        /// seriko⑤（`cue_target_of == Shell` を action ゲートにする）。
        Seriko,
        /// emo-text⑥（`cue_target_of == Balloon` を action ゲートにする）。
        EmoText,
    }

    /// `cue_target_of` の分類（`Option<CueTarget>`）から**action する演者**を導く（高々一つ）。
    ///
    /// `Option<CueTarget>` が単一値ゆえ「variant あたり action する演者は高々一つ」は構造的に
    /// 保証される——本関数はその分類を演者同定へ写す唯一の権威。`CueTarget` を exhaustive に
    /// 網羅し（catch-all なし）、将来 `CueTarget` variant が増えたらコンパイラが再検討を強制する。
    fn acting_performer(target: Option<CueTarget>) -> Option<Performer> {
        match target {
            Some(CueTarget::Shell) => Some(Performer::Seriko),
            Some(CueTarget::Balloon) => Some(Performer::EmoText),
            // Window（`\![move]` 系の窓移動先）は seriko/emo-text いずれの担当でもない——
            // 消費は areka bin 側の MoveCueSink（W5）で、この seriko/emo-text partition の
            // 圏外ゆえ None（本テストの CueCommand→cue_target_of 経路では出現しないが、
            // CueTarget の網羅性を型で保つためのアーム）。
            Some(CueTarget::Window) => None,
            None => None, // Wait（純粋な待ち）・Custom（分類不能）＝どの演者も action しない。
        }
    }

    /// 全 `CueCommand` variant を列挙する（catch-all なし・dola が variant を追加したら
    /// コンパイラが本網羅を強制的に再検討させる＝partition の漏れを型で塞ぐ）。
    fn every_cue_command() -> Vec<CueCommand> {
        // exhaustive の型檻: variant を 1 つでも落としたらここが不完全になるため、
        // `match` で全 variant を触れてから値を積む（新 variant 追加時にコンパイル停止）。
        let sample = CueCommand::Wait;
        match &sample {
            CueCommand::Text(_)
            | CueCommand::Clear
            | CueCommand::Emote { .. }
            | CueCommand::Choice { .. }
            | CueCommand::EntityRef(_)
            | CueCommand::Custom { .. }
            | CueCommand::NewLine { .. }
            | CueCommand::BalloonSurface { .. }
            | CueCommand::Cursor { .. }
            | CueCommand::Wait
            | CueCommand::ClearAll => {}
        }
        vec![
            CueCommand::Text("hi".into()),
            CueCommand::Clear,
            CueCommand::Emote { key: "0".into() },
            CueCommand::Choice {
                id: "yes".into(),
                text: "はい".into(),
                references: vec![],
            },
            CueCommand::EntityRef(42),
            CueCommand::Custom {
                command: "fade".into(),
                params: dola::DynamicValue::Null,
            },
            CueCommand::NewLine { ratio: 1.0 },
            CueCommand::BalloonSurface { key: "2".into() },
            CueCommand::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
            CueCommand::Wait,
            CueCommand::ClearAll,
        ]
    }

    /// **partition 檻**: 全 variant について、action する演者が高々一つであり、かつ
    /// `cue_target_of` の分類（Shell→seriko／Balloon→emo-text／None→誰も action しない）と
    /// **一致**する（各演者の action ゲートが `cue_target_of` の単一権威に従う・D4/R2.4）。
    #[test]
    fn every_variant_has_at_most_one_acting_performer_consistent_with_cue_target_of() {
        // 各 variant の期待 action 演者（表示系＝Shell→seriko／文字系＝Balloon→emo-text／
        // action なし＝None）。この表は `cue_target_of`（dola 単一権威）と 1:1 で対応する。
        let expected: &[(CueCommand, Option<Performer>)] = &[
            (
                CueCommand::Emote { key: "0".into() },
                Some(Performer::Seriko),
            ),
            (CueCommand::EntityRef(42), Some(Performer::Seriko)),
            (
                CueCommand::BalloonSurface { key: "2".into() },
                Some(Performer::Seriko),
            ),
            (CueCommand::Text("hi".into()), Some(Performer::EmoText)),
            (CueCommand::NewLine { ratio: 1.0 }, Some(Performer::EmoText)),
            (CueCommand::Clear, Some(Performer::EmoText)),
            (CueCommand::ClearAll, Some(Performer::EmoText)),
            (
                CueCommand::Choice {
                    id: "y".into(),
                    text: "はい".into(),
                    references: vec![],
                },
                Some(Performer::EmoText),
            ),
            // Cursor（`\_l`）はバルーン系表現者（emo-text＝Balloon）が action する。
            (
                CueCommand::Cursor {
                    x: "5em".into(),
                    y: "2lh".into(),
                },
                Some(Performer::EmoText),
            ),
            (
                CueCommand::Custom {
                    command: "fade".into(),
                    params: dola::DynamicValue::Null,
                },
                None,
            ),
            (CueCommand::Wait, None),
        ];

        // (a) 期待表の各 variant が `cue_target_of` の分類と一致する（action 演者は高々一つ）。
        for (command, want) in expected {
            let got = acting_performer(cue_target_of(command));
            assert_eq!(
                &got, want,
                "variant {command:?} の action 演者は cue_target_of の分類と一致するはず（partition・単一権威）"
            );
        }

        // (b) 全 variant が期待表に過不足なく現れる（漏れ・重複がない＝partition が全域）。
        assert_eq!(
            expected.len(),
            every_cue_command().len(),
            "期待表は全 CueCommand variant を過不足なく網羅する（partition の全域性）"
        );
    }

    /// **acceptance 補完檻**: 表情切替（`Emote`）は seriko（Shell）だけが action し、
    /// テキスト演者（emo-text＝Balloon）は action しない（broadcast で受信はするが動作しない・
    /// duration のみ honor する・R8.4/R2.3）。`ClearAll`／`Wait` の対比も併せて固定する。
    #[test]
    fn emote_acts_only_on_seriko_text_performer_receives_but_does_not_act() {
        let emote = CueCommand::Emote { key: "0".into() };
        assert_eq!(
            acting_performer(cue_target_of(&emote)),
            Some(Performer::Seriko),
            "Emote（表情切替）は seriko だけが action する（Shell 分類）"
        );
        assert_ne!(
            acting_performer(cue_target_of(&emote)),
            Some(Performer::EmoText),
            "テキスト演者（emo-text）は Emote を broadcast 受信するが action しない（duration のみ honor）"
        );

        // #6 全消去はテキスト演者だけが action（seriko は受信のみ）。
        assert_eq!(
            acting_performer(cue_target_of(&CueCommand::ClearAll)),
            Some(Performer::EmoText),
            "ClearAll は emo-text だけが action する（Balloon 分類・#6）"
        );

        // 純粋な待ちはどの演者も action しない（duration だけ honor）。
        assert_eq!(
            acting_performer(cue_target_of(&CueCommand::Wait)),
            None,
            "Wait はどの演者も action しない（action なし・duration のみ）"
        );
    }
}

// ===================== S1: boot 成功シナリオ（task 4.2） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S1:
// 「Boot→OnBoot GET が Value→StartTalk→sakura 再生→RecordingSink の発火列（at 昇順・
// 内容一致）→TalkDone{Ended} が kanade へ転送される」を、起動から実 ghost スタック
// （kanade→start-relay→dispatcher→sakura の実アクター一式）を通して駆動し、時刻注入
// （Tick）のみで確認する（sleep 不使用・要件 7.2/7.4/7.6・純 x64）。
#[cfg(test)]
mod s1_boot_success {
    use super::*;

    use areka_ghost::dispatcher::DispatcherMsg;
    use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
    use areka_kanade::{KanadeConfig, MonotonicMs, ShioriCall, events};
    use areka_parsers::charset::DefaultEncoding;

    /// このテスト専用の一意な一時ディレクトリ（`runtime.rs`/`config.rs` テストの流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_spine_e2e_s1_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
    /// `shell/master/descript.txt`）を構築する。shell descript の `name` は `shell_name`
    /// （`OnBoot` Ref0・`KanadeConfig::shell_name` の値源と一致させるための既知値・task
    /// 4.2 参照材料 4/5）。
    fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,S1TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            format!("charset,UTF-8\nname,{shell_name}\n").as_bytes(),
        )
        .expect("write shell descript.txt");
    }

    /// events 表由来の [`ShioriCall`] を、このファイル固有の [`RecordedCall`]（task 4.1 の
    /// [`ScriptedShioriBackend`] 記録型）へ変換する（fixture・assert・実装が単一の正本＝
    /// events 表を共有する・Req 7.1）。kanade 自身の統合テストが使う `expected_call`/
    /// `CallMethod` は kanade クレート専用の private 型であり本ファイルからは参照できない
    /// ため、ここで同旨の変換を用意する（task 4.2 参照材料 6 の指示どおり）。
    fn expected_from_shiori_call(call: ShioriCall) -> RecordedCall {
        match call {
            ShioriCall::Get { id, references, .. } => RecordedCall::Get {
                id: id.to_string(),
                references,
            },
            ShioriCall::Notify { id, references, .. } => RecordedCall::Notify {
                id: id.to_string(),
                references,
            },
        }
    }

    /// 有界待機ヘルパ（`runtime.rs`/`dispatcher.rs` テストモジュールと同旨のローカルコピー）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: std::time::Duration, f: F) {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);
        std::thread::spawn(move || {
            f();
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(timeout).is_ok(),
            "'{what}' did not complete within {timeout:?} (possible hang)"
        );
    }

    /// S1: boot 成功——boot→OnBoot(Value)→StartTalk→sakura 再生→RecordingSink の発火列
    /// （at 昇順・内容一致）→TalkDone を、Tick 注入のみで決定論的に確認する
    /// （要件 7.2/7.4/7.6）。
    #[test]
    fn s1_boot_success_plays_greeting_and_records_expected_cue_sequence() {
        const SHELL_NAME: &str = "S1BootShell";

        let root =
            unique_temp_dir("s1_boot_success_plays_greeting_and_records_expected_cue_sequence");
        let _ = std::fs::remove_dir_all(&root);
        write_ghost_fixture(&root, SHELL_NAME);

        // events 表と同一パラメタで期待値導出用 config を構築する（`resolve_kanade_config` が
        // 実際に組み立てる値と shell_name/baseware_version が一致する・task 4.2 参照材料 4）。
        let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

        // boot 系列一式のみを台本化する（OnSecondChange は kanade へ Tick を一切送らないため
        // 不要・OnClose/Unload は本テスト末尾の shutdown() が消費する）。
        let (backend, handle) = ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
            // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
            // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
            .get("username", Ok(None))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
            .build();

        let surface_sink = RecordingSink::new();
        let text_sink = RecordingSink::new();
        let surface_records = surface_sink.records();
        let text_records = text_sink.records();

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
            system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        // boot() は内部で KanadeMsg::Boot を既に送出済み——boot 系列は kanade アクタースレッド
        // 上で同期往復（oneshot round trip）のみで完走するため、この時点で OnInitialize〜
        // basewareversion の 4 呼出はスケジューリング次第で既に発火し終えている。しかし
        // StartTalk は start_tx→start-relay→dispatcher_tx の 2 hop（別スレッド）を経るため、
        // dispatcher の active slot に talk が実際に載るタイミングはスレッドスケジューリング
        // 依存であり、単一の Tick 送出が必ず間に合う保証はない。sleep は使わず、Tick を送る
        // たびに RecordingSink を確認する再送ループ（実時間待機なし・単調増加する `now` の
        // 注入のみ・`yield_now` で他スレッドに実行機会を譲るだけ）でこの橋渡しをする——
        // script に `\w`（待ち）を含めていないため、dispatcher の active slot に talk が
        // 載った直後の最初の Tick で全発火（Emote＋Text）と自然終端（TalkDone{Ended}）が
        // 単一 Tick 内で完了する。
        let mut now: u64 = 1;
        let mut fired = false;
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            runtime
                .dispatcher()
                .send(DispatcherMsg::Tick {
                    now: MonotonicMs(now),
                })
                .expect("dispatcher actor should still be alive while probing for the boot talk");
            now += 1;
            if !surface_records
                .lock()
                .expect("records mutex poisoned")
                .is_empty()
            {
                fired = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            fired,
            "S1: surface cue never fired after repeated Tick — boot talk did not reach \
             dispatcher's active slot within bound"
        );

        // ---- (a) 起動系列が正典順序で発火（NOTIFY／GET の別・Reference 構成込み） ----
        // real shiori アクター（run_shiori_loop）はメッセージ到達のたびに冒頭で
        // backend.status() を確認する（死活監視・親モジュール rustdoc 参照）ため、
        // calls() には Get/Notify の間に RecordedCall::Status が挟まる。起動系列の
        // 順序判定はこの死活監視ノイズと無関係なので除外して比較する。
        let expected_boot_prefix = vec![
            expected_from_shiori_call(events::on_initialize(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            // username prefetch GET（OnInitialize 後・OnFirstBoot 前・R4.1・DD-9 の唯一の期待値導出経路）。
            expected_from_shiori_call(areka_kanade::resources::resource_username(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::on_first_boot(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
                // fixture ghost に永続ファイル無し＝vanish 不在ゆえ Ref0="0"（従来値同値）。
                0,
            )),
            expected_from_shiori_call(events::on_boot(
                &config,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::baseware_version(
                &config,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
        ];
        let calls = handle.calls();
        let calls_without_status: Vec<RecordedCall> = calls
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .cloned()
            .collect();
        assert_eq!(
            calls_without_status, expected_boot_prefix,
            "起動系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→basewareversion）が正典順序で発火していない"
        );

        // ---- (b)(c) RecordingSink の発火列（broadcast・at 昇順・内容一致）----
        // broadcast ゆえ surface/text の両 sink が**同一の全 cue** を受ける（中央振り分け廃止・
        // どの action を演じるかは演者側 relevance の責務）。`\s[0]hello\e` の期待 broadcast 列:
        //   ClearAll@0（#6 全消去・task 5.2 冒頭前置）/ Emote{0}@0（\s[0]）/ Text(hello)@0
        //   （後続 cue が無く先頭群に留まる）。発火は drive の on_tick 内で同期 broadcast されるが、
        // probe loop の break 直後に部分列を読む競合を避けるため、両 sink が 3 件に達するまで
        // 有界スピンで整定を待つ（sleep 不使用・yield のみ）。
        let expected = vec![
            CueCommand::ClearAll,
            CueCommand::Emote {
                key: "0".to_string(),
            },
            CueCommand::Text("hello".to_string()),
        ];
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            let s = surface_records.lock().expect("records mutex poisoned").len();
            let t = text_records.lock().expect("records mutex poisoned").len();
            if s >= expected.len() && t >= expected.len() {
                break;
            }
            std::thread::yield_now();
        }
        let surface = surface_records
            .lock()
            .expect("records mutex poisoned")
            .clone();
        let text = text_records.lock().expect("records mutex poisoned").clone();
        let assert_broadcast = |cues: &[TalkCue], who: &str| {
            let commands: Vec<CueCommand> = cues.iter().map(|c| c.command.clone()).collect();
            assert_eq!(
                commands, expected,
                "{who} sink は broadcast で ClearAll/Emote/hello を受ける（partition は演者側 relevance）: {cues:?}"
            );
            for cue in cues {
                assert_eq!(cue.at, 0.0, "{who} 発火は全て at=0.0");
                assert_eq!(
                    cue.actor,
                    ActorKey::from("0"),
                    "{who} 発火 actor は 既定 scope 0"
                );
            }
            for pair in cues.windows(2) {
                assert!(pair[0].at <= pair[1].at, "{who} 発火列は at 昇順であるべき");
            }
        };
        assert_broadcast(&surface, "surface");
        assert_broadcast(&text, "text");

        // ---- 後片付け兼 (c) の間接証跡 ----
        // TalkDone{Ended} が dispatcher→kanade へ転送済みであること（dispatcher の slot が
        // 解放され kanade が Steady{None} へ戻っていること）は、kanade inbox を直接覗く
        // 経路が公開面に無いため、後続の shutdown（ForceQuit→OnClose NOTIFY→Unload の順）
        // が台本どおり完走し Ok(()) を返すことをもって間接的に確認する——もし TalkDone が
        // 届かず kanade が Steady{Some} に取り残されていても ForceQuit は横断遷移で全 Phase
        // から Unloading{Forced} へ直行するため shutdown 自体は成立してしまうが、これは
        // 「正規終了握手」シナリオ（task 4.5）の担当範囲であり、本タスクの主眼は (a)(b) の
        // 発火列検証に置く（CONCERNS 参照）。
        run_bounded(
            "shutdown after S1 boot talk completion",
            super::E2E_BOUND,
            move || {
                let result = runtime.shutdown(areka_kanade::CloseReason::System);
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) after S1 boot talk completes, got {result:?}"
                );
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}

// ===================== S2: 接続失敗シナリオ（task 4.3） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S2:
// 「接続失敗: connect が Err→ShioriDown→Unloading{Fault}→全 join（有界）。」を、
// 起動から実 ghost スタック（shiori actor→down_tx→down-relay→kanade_tx の実結線
// 一式）を通して駆動する。Tick 注入は一切不要——`KanadeMsg::ShioriDown` は
// `run_inbox`（areka-kanade/src/actor.rs）が受領のたびに step へ即座に投入する
// 横断メッセージであり、dispatcher の Tick ポンプに一切ゲートされない
// （要件 7.4 の確認材料・kanade 自身の受信ループを直接読んで確認済み）。
#[cfg(test)]
mod s2_connect_failure {
    use super::*;

    use areka_ghost::dispatcher::DispatcherMsg;
    use areka_ghost::{
        GhostBootOptions, GhostHandles, GhostParts, ShioriWiring, SystemVarWiring, TickerMode, boot,
    };
    use areka_parsers::charset::DefaultEncoding;

    use areka_actor::{ActorError, ActorHandle};

    /// このテスト専用の一意な一時ディレクトリ（S1 の流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_spine_e2e_s2_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
    /// `shell/master/descript.txt`）を構築する。`s1_boot_success::write_ghost_fixture`
    /// と同旨だが、sibling module から private item は参照できないためローカルに
    /// 複製する（本シナリオは connect が即 `Err` を返し実際の起動系列を一切発火しない
    /// ため、shell 側の `name` の値そのものは load-bearing でない）。
    fn write_ghost_fixture(root: &std::path::Path) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,S2TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            b"charset,UTF-8\nname,S2TestShell\n",
        )
        .expect("write shell descript.txt");
    }

    /// `ActorHandle::join` を有界時間で観測する（`areka-kanade` 統合テストの
    /// `join_bounded` と同旨のローカルコピー——`ActorHandle::join` 自体は無期限
    /// ブロックし得るため、別スレッドへ逃がし `recv_timeout` で宙吊りを防ぐ）。
    fn join_bounded(
        what: &str,
        timeout: std::time::Duration,
        handle: ActorHandle,
    ) -> Result<(), ActorError> {
        let (res_tx, res_rx) = std::sync::mpsc::sync_channel::<Result<(), ActorError>>(0);
        std::thread::spawn(move || {
            let _ = res_tx.send(handle.join());
        });
        match res_rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
        }
    }

    const BOUND: std::time::Duration = super::E2E_BOUND;

    /// S2: 接続失敗——connect が即 `Err` を返しても `boot()` 自体は成功する
    /// （connect 失敗は shiori アクタースレッド**内部**で非同期に起こるため、`boot()`
    /// 自身の同期的な返り値には影響しない。`GhostBootError::Mount` のみが `boot` を
    /// 失敗させる・design「起動（boot）シーケンス」）。その後、実結線（shiori actor の
    /// `on_down`→`down_tx`→down-relay→`kanade_tx`）が `ShioriDown` を kanade へ届け、
    /// kanade は本テストから一切 `Close`/`ForceQuit` を送られることなく自律的に
    /// Unloading{Fault}→best-effort Unload→Stopped→StopSelf へ倒れて終了する
    /// （`into_parts()` で得た `handles.kanade` を直接 join して確認・design「S2 接続
    /// 失敗」）。加えて残る全コンポーネント（shiori／dispatcher／両 relay）も有界時間内に
    /// 後始末されることを確認する（design「全 join（有界）」の文字どおりの意味）。
    #[test]
    fn s2_connect_failure_drives_autonomous_kanade_termination_and_full_teardown() {
        let root = unique_temp_dir(
            "s2_connect_failure_drives_autonomous_kanade_termination_and_full_teardown",
        );
        let _ = std::fs::remove_dir_all(&root);
        write_ghost_fixture(&root);

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(|| Err("simulated connect failure".to_string()))),
            sinks: vec![
                Box::new(RecordingSink::new()),
                Box::new(RecordingSink::new()),
            ],
            system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        // boot() 自体は connect の成否と無関係に成功する——connect 失敗は非同期に
        // shiori アクタースレッド内部で起こるため、これは「接続失敗は boot 失敗では
        // ない」ことの重要な、逆に取り違えやすい直接証跡になる。
        let runtime = boot(options).expect(
            "boot must succeed even though the SHIORI connect will fail asynchronously \
             inside the shiori actor thread — a connect failure is NOT a boot failure",
        );

        let parts = runtime.into_parts();
        let GhostParts {
            dispatcher,
            handles,
            ..
        } = parts;
        let GhostHandles {
            kanade: kanade_handle,
            dispatcher: dispatcher_handle,
            shiori: shiori_handle,
            start_relay: start_relay_handle,
            down_relay: down_relay_handle,
            ticker: _,
            sylphya: _,
        } = handles;

        // ---- 主観測: kanade の自律終了（外部からの Close/ForceQuit を一切送らない）----
        // 実結線（shiori actor の on_down→down_tx→down-relay→kanade_tx）が ShioriDown を
        // 届け、kanade 自身の Fault 系列が完走したことの直接証跡——このテストは kanade
        // へ一度もメッセージを送っていない。
        join_bounded(
            "kanade autonomous termination on connect failure",
            BOUND,
            kanade_handle,
        )
        .expect(
            "kanade should autonomously terminate once the real down_tx→down-relay→kanade_tx \
             wiring delivers ShioriDown from a genuine connect failure — no external shutdown \
             trigger should be necessary",
        );

        // shiori actor は接続確立に失敗し受信ループへ一切入らないため、ほぼ即座に終了する
        // （`spawn_shiori_actor` の connect-failure 経路・real.rs 参照）。
        join_bounded(
            "shiori actor near-instant exit (never entered its recv loop)",
            BOUND,
            shiori_handle,
        )
        .expect("shiori actor should already be finished — it never entered run_shiori_loop");

        // ---- 副観測: 残る全コンポーネントも有界時間内に後始末される（design「全 join」）----
        // dispatcher は自身の Sender を保持し自然終了しない（「アクター別の停止経路」表）
        // ため、明示的に Close を送出する。
        let _ = dispatcher.send(DispatcherMsg::Close);
        join_bounded("dispatcher join after Close", BOUND, dispatcher_handle)
            .expect("dispatcher should terminate after Close");

        // start-relay／down-relay は上流（kanade 自身の start_tx／shiori 自身の down_tx）が
        // 既に drop 済み（kanade・shiori 双方のアクタースレッドが既に終了している）ため、
        // メッセージを送らずとも自然終了する。
        join_bounded("start-relay natural termination", BOUND, start_relay_handle)
            .expect("start-relay should terminate naturally once kanade's start_tx is dropped");
        join_bounded("down-relay natural termination", BOUND, down_relay_handle)
            .expect("down-relay should terminate naturally once shiori's down_tx is dropped");

        let _ = std::fs::remove_dir_all(&root);
    }
}

// ===================== S3: helper 死活検出シナリオ（task 4.4） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S3:
// 「helper 死活: scripted `status` を `Exited(Abnormal)` へ遷移させ、`runtime.kanade()` へ
// `Tick{now}` を注入→Steady pump の OnSecondChange が shiori actor へ到達→到達時 status
// 確認で検出→ShioriDown→Fault 系列→全 join（有界・駆動は本番と同一経路・実時間ゼロ）。」
//
// S1（boot→Steady 到達確認の retry ループ技法）と S2（`into_parts()` ベースの直接 join に
// よる自律終了の証明技法）を組み合わせ、さらに「シナリオ途中で status を差し替える」
// （task 4.1 `status_transitions_from_running_to_exited_when_mutated_externally_mid_scenario`
// で証明済みの capability）を実際の e2e 経路へ初めて適用する。
#[cfg(test)]
mod s3_helper_liveness_detected {
    use super::*;

    use areka_ghost::dispatcher::DispatcherMsg;
    use areka_ghost::{
        GhostBootOptions, GhostHandles, GhostParts, ShioriWiring, SystemVarWiring, TickerMode, boot,
    };
    use areka_kanade::{KanadeMsg, MonotonicMs};
    use areka_parsers::charset::DefaultEncoding;

    use areka_actor::{ActorError, ActorHandle};

    /// このテスト専用の一意な一時ディレクトリ（S1/S2 の流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_spine_e2e_s3_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
    /// `shell/master/descript.txt`）を構築する。S1 の `write_ghost_fixture` と同旨だが、
    /// sibling module から private item は参照できないためローカルに複製する。
    fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,S3TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            format!("charset,UTF-8\nname,{shell_name}\n").as_bytes(),
        )
        .expect("write shell descript.txt");
    }

    /// `ActorHandle::join` を有界時間で観測する（S2 の `join_bounded` と同旨のローカルコピー）。
    fn join_bounded(
        what: &str,
        timeout: std::time::Duration,
        handle: ActorHandle,
    ) -> Result<(), ActorError> {
        let (res_tx, res_rx) = std::sync::mpsc::sync_channel::<Result<(), ActorError>>(0);
        std::thread::spawn(move || {
            let _ = res_tx.send(handle.join());
        });
        match res_rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
        }
    }

    const BOUND: std::time::Duration = super::E2E_BOUND;

    /// S3: helper 死活検出——scripted `status()` をシナリオ途中で `Exited(Abnormal)` へ差し
    /// 替え、`runtime.kanade()` へ Tick を 1 回注入するだけで（Steady pump が発行する
    /// OnSecondChange が shiori actor へ到達し、到達時チェックが検出する）、この e2e からは
    /// 一度も明示 Close/ForceQuit を送らずに kanade が自律的に Fault 系列（Unloading{Fault}
    /// →best-effort Unload→Stopped→StopSelf）へ倒れて終了することを確認する（design「S3
    /// helper 死活」・要件 7.4/7.5/7.6）。
    #[test]
    fn s3_helper_liveness_detected_mid_scenario_drives_autonomous_fault_termination() {
        const SHELL_NAME: &str = "S3LivenessShell";

        let root = unique_temp_dir(
            "s3_helper_liveness_detected_mid_scenario_drives_autonomous_fault_termination",
        );
        let _ = std::fs::remove_dir_all(&root);
        write_ghost_fixture(&root, SHELL_NAME);

        // boot 系列一式（S1 と同旨）＋ OnSecondChange（Steady pump の 1 発）＋ unload
        // （Fault 系列が発行する ShioriUnload の応答・best-effort ゆえ Abnormal でも Stopped へ
        // 収束する）を台本化する。OnClose は台本化しない——S3 は Fault 経路のため kanade 自身が
        // OnClose NOTIFY を発行することはない（正規 close 握手は S4/S5 の担当領域）。
        //
        // DD-IT-12: boot は挨拶 talk を追跡し `Steady{talk: Some(greeting)}` へ完了する。ゆえに
        // 下で注入する単一の `KanadeMsg::Tick` が pump する OnSecondChange は、挨拶 talk の
        // TalkDone が kanade に届いて `Steady{talk: None}` へ戻った後なら GET（Ref3=1）、まだ
        // 挨拶再生中なら NOTIFY（Ref3=0・`Status: talking`）になる（この 2 経路の別は挨拶
        // TalkDone の到達と注入 Tick の到達順というスレッド間タイミング次第・S3 の観測点は
        // 死活検出であり GET/NOTIFY の別に依存しない）。どちらの方式でも OnSecondChange は
        // shiori actor へ到達し、到達時 status() 確認が Exited を検出する（Req2.2/DD-IT-12）——
        // ゆえに GET/NOTIFY 双方を台本化し、レースが選んだ側だけが消費される（他方は未消費で
        // 無害）。
        let (backend, handle) = ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
            // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
            // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
            .get("username", Ok(None))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
            .notify("basewareversion", Ok(()))
            .get("OnSecondChange", Ok(None))
            .notify("OnSecondChange", Ok(()))
            .unload(Ok(ExitKind::Abnormal(1)))
            .build();

        let surface_sink = RecordingSink::new();
        let text_sink = RecordingSink::new();
        let surface_records = surface_sink.records();

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
            system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        // ---- boot talk を Steady{None} 到達まで駆動する（S1 と同一技法・sleep 不使用） ----
        // dispatcher へ Tick を送るたびに RecordingSink を確認する有界再送ループ（実時間
        // 待機なし・単調増加する now の注入のみ・`yield_now` で他スレッドに実行機会を譲る
        // だけ）。boot talk が dispatcher の active slot に載って発火し終えた時点で、
        // kanade 自身は（dispatcher Tick とは無関係な別チャンネル経由で）basewareversion
        // NOTIFY の応答往復のみで既に Steady{talk: None} へ完了している（boot.rs:
        // 「boot は常に Steady{talk: None} へ完了する」・BootVersion+Notified の遷移は
        // StartTalk 発行と独立に basewareversion の応答のみで確定するため、StartTalk が
        // start-relay→dispatcher の 2 hop を経て active slot に載り、さらに Tick で実際に
        // 発火するよりずっと早く完了している）。
        let mut now: u64 = 1;
        let mut fired = false;
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            runtime
                .dispatcher()
                .send(DispatcherMsg::Tick {
                    now: MonotonicMs(now),
                })
                .expect("dispatcher actor should still be alive while probing for the boot talk");
            now += 1;
            if !surface_records
                .lock()
                .expect("records mutex poisoned")
                .is_empty()
            {
                fired = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            fired,
            "S3: surface cue never fired after repeated Tick — boot talk did not reach \
             dispatcher's active slot within bound"
        );

        // boot 系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→basewareversion）の
        // 5 呼出が完了済みであること（＝kanade が Steady{None} へ既に到達済みであること）を
        // 裏付ける間接証跡（S1 と同旨・死活監視の Status ノイズは除外して数える・task 8.2 の
        // username prefetch GET が OnInitialize と OnFirstBoot の間に 1 件加わり 4→5 になる）。
        let calls_handle = handle.calls();
        let boot_prefix_len = calls_handle
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .count();
        assert_eq!(
            boot_prefix_len, 5,
            "S3: boot 系列 5 呼出（OnInitialize/username/OnFirstBoot/OnBoot/basewareversion）が \
             完了していない——kanade はまだ Steady に到達していないはず"
        );

        // ---- helper がシナリオ途中で異常終了する様子を、backend の外側（テスト自身の
        // スレッド）から駆動する（task 4.1 の capability・design「S3 helper 死活」）。----
        handle.set_status(HelperStatus::Exited(ExitKind::Abnormal(1)));

        // ---- kanade へ Tick を 1 回だけ注入する（Steady pump の唯一の駆動源）。----
        // Steady{talk: None} + Tick → OnSecondChange GET が shiori actor へ届く
        // （steady.rs on_tick）。run_shiori_loop はメッセージ到達の冒頭で必ず
        // backend.status() を確認するため（親モジュール rustdoc 参照）、この 1 通の
        // Tick 到達だけで死活検出（Exited 初回観測→ShioriDown 送出）と OnSecondChange
        // 応答処理の両方が起こる。ShioriDown は down-relay 経由で kanade 自身の inbox
        // へ届き、次にそのメッセージを処理する際に横断アーム（Unloading{Fault}）へ
        // 倒れる——この e2e からは以後一切のメッセージを送らない。
        runtime
            .kanade()
            .send(KanadeMsg::Tick {
                now: MonotonicMs(1_000_000),
            })
            .expect("kanade actor should still be alive to receive the liveness-detecting Tick");

        // ---- 主観測: kanade の自律終了（外部からの Close/ForceQuit を一切送らない）----
        // S2 と同じ into_parts() ベースの直接 join 技法——このテストは kanade へ Tick を
        // 1 回送った後、一度も Close/ForceQuit を送っていない。
        let parts = runtime.into_parts();
        let GhostParts {
            dispatcher,
            handles,
            ..
        } = parts;
        let GhostHandles {
            kanade: kanade_handle,
            dispatcher: dispatcher_handle,
            shiori: shiori_handle,
            start_relay: start_relay_handle,
            down_relay: down_relay_handle,
            ticker: _,
            sylphya: _,
        } = handles;

        join_bounded(
            "kanade autonomous fault termination after mid-scenario status transition",
            BOUND,
            kanade_handle,
        )
        .expect(
            "kanade should autonomously terminate once the OnSecondChange-triggered \
             status() check detects Exited(Abnormal) and drives ShioriDown through the \
             real down_tx→down-relay→kanade_tx wiring — no external Close/ForceQuit should \
             be necessary",
        );

        // shiori actor: kanade の Fault 系列は Unloading{Fault} 到達時に ShioriUnload
        // action を発行し、その応答受領後に必ず shiori へ ShioriMsg::Close を送出して
        // から StopSelf する（「アクター別の停止経路」表・kanade 正本）ため、shiori
        // actor も有界時間内に終了するはず。
        join_bounded(
            "shiori actor termination after kanade's fault sequence closes it",
            BOUND,
            shiori_handle,
        )
        .expect(
            "shiori actor should terminate once kanade's fault sequence sends ShioriMsg::Close",
        );

        // ---- 副観測: 残る全コンポーネントも有界時間内に後始末される（design「全 join」）----
        // dispatcher は自身の Sender を保持し自然終了しない（「アクター別の停止経路」表）
        // ため、明示的に Close を送出する（S2 と同旨）。
        let _ = dispatcher.send(DispatcherMsg::Close);
        join_bounded("dispatcher join after Close", BOUND, dispatcher_handle)
            .expect("dispatcher should terminate after Close");

        // start-relay／down-relay は上流（kanade 自身の start_tx／shiori 自身の down_tx）が
        // 既に drop 済み（kanade・shiori 双方のアクタースレッドが既に終了している）ため、
        // メッセージを送らずとも自然終了する。
        join_bounded("start-relay natural termination", BOUND, start_relay_handle)
            .expect("start-relay should terminate naturally once kanade's start_tx is dropped");
        join_bounded("down-relay natural termination", BOUND, down_relay_handle)
            .expect("down-relay should terminate naturally once shiori's down_tx is dropped");

        // ---- sticky-once の間接証跡 ----
        // ShioriDown の発火自体は kanade inbox 側のイベントであり calls() には現れないが、
        // Fault 系列が best-effort Unload を実際に発行したこと（＝ShioriDown が届いて
        // Unloading{Fault} へ倒れたことの直接証跡）と、このシナリオ全体が有界時間内に
        // 完走したこと（status flapping で shiori actor がループし続けるような壊れ方を
        // していないこと）の 2 点を確認する。sticky-once の不変量そのものは task 1.4 の
        // 単体テスト（death_detected_once_reports_shiori_down_and_only_once）が既に固定
        // しており、本 e2e の責務は配線がそれを最後まで届けることの証明に置く
        // （CONCERNS 参照）。
        let all_calls = calls_handle.lock().expect("calls mutex poisoned").clone();
        assert!(
            all_calls.iter().any(|c| matches!(c, RecordedCall::Unload)),
            "S3: Fault 系列は best-effort Unload を発行するはず: {all_calls:?}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}

// ===================== S4: close 握手シナリオ（task 4.5） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S4:
// 「close 握手: `CloseRequest`→`OnClose` GET が close talk（`\-` 終端）→
// `TalkDone{Quit}`→`Unload` が呼ばれ scripted `Ok(ExitKind::Clean)`→`Unloaded` 観測
// →`StopSelf`→shutdown で全スレッド join（要件 7.3）。」を、S1（boot→Steady 到達確認の
// retry ループ技法）を踏まえたうえで、初めて「正規（canonical）の終了要求」（S2/S3 の
// Fault 駆動終了とは異なる、成功する close talk 駆動の Quit 経路）を駆動する。
//
// close talk の script は `\-`（先行 cue のない bare quit タグ）にする——
// `areka_sakura::drive` の `quit_only_script_ends_immediately_with_quit_not_ended` が
// 示すとおり、これは空 CueSheet＋`TalkEndReason::Quit` へ即時（Tick 不要）コンパイルされる
// （空 sheet 高速経路）。ゆえに close talk 自体の完了確認に Tick 注入は要らない——ただし
// OnClose GET（kanade↔shiori の同期往復）・StartTalk（start_tx→start-relay→dispatcher_tx
// の 2 hop）・TalkDone（dispatcher 自身の inbox 経由で kanade へ転送）は依然として実スレッド
// 境界を跨ぐため、有界のスピン待機（Tick 送出なし・sleep なし・`yield_now` のみ）で
// `handle.calls()` に `Unload` が現れるのを確認する。
//
// `handle.calls()` に `Unload` が現れた時点で、kanade 自身の thread は
// `round_trip_unload`（`areka-kanade/src/actor.rs`）内の `reply_rx.recv()` にまだ
// ブロック中か、既にその応答を消化して `Stopped`＋`StopSelf`（shiori へ `Close` を送り
// break）へ進んでいる——いずれの場合も kanade は「次のメッセージを inbox から取り出す」
// 前に完結するため、この時点より後で送る `runtime.shutdown()` の `ForceQuit` は
// （a) まだ処理されず thread 終了と共に破棄されるか、(b) 送出自体が失敗する
// （既に停止済み＝冪等）のいずれかであり、`unload()` が二度目に呼ばれることはない
// （`ScriptedShioriBackend::unload` は `Option::take()` で一度きり消費するため、二重呼出は
// 即座に panic するはずだが、上記の理由からこの経路には到達しない）。
#[cfg(test)]
mod s4_close_handshake {
    use super::*;

    use areka_ghost::dispatcher::DispatcherMsg;
    use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
    use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriCall, events};
    use areka_parsers::charset::DefaultEncoding;

    /// このテスト専用の一意な一時ディレクトリ（S1〜S3 の流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_spine_e2e_s4_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリーを構築する（S1/S3 の
    /// `write_ghost_fixture` と同旨だが、sibling module から private item は参照できない
    /// ためローカルに複製する）。
    fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,S4TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            format!("charset,UTF-8\nname,{shell_name}\n").as_bytes(),
        )
        .expect("write shell descript.txt");
    }

    /// events 表由来の [`ShioriCall`] をこのファイル固有の [`RecordedCall`] へ変換する
    /// （S1 の `expected_from_shiori_call` と同旨のローカル複製・Req 7.1）。
    fn expected_from_shiori_call(call: ShioriCall) -> RecordedCall {
        match call {
            ShioriCall::Get { id, references, .. } => RecordedCall::Get {
                id: id.to_string(),
                references,
            },
            ShioriCall::Notify { id, references, .. } => RecordedCall::Notify {
                id: id.to_string(),
                references,
            },
        }
    }

    /// 有界待機ヘルパ（S1/S2/S3 と同旨のローカルコピー）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: std::time::Duration, f: F) {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);
        std::thread::spawn(move || {
            f();
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(timeout).is_ok(),
            "'{what}' did not complete within {timeout:?} (possible hang)"
        );
    }

    /// S4: close 握手——`CloseRequest` が `OnClose` GET を発行させ、応答スクリプト
    /// （bare quit `\-`）が close talk として再生起動され、空 sheet 高速経路で即座に
    /// `TalkDone{Quit}` へ終端し、kanade が横断アームで `Unloading{Quit}`→
    /// scripted `Ok(ExitKind::Clean)` の `Unload`→`Unloaded` 観測→`StopSelf` へ完走する
    /// ことを、`runtime.shutdown()` の全スレッド join 成功をもって確認する（design「S4
    /// close 握手」・要件 7.3/7.4/7.5/7.6）。
    #[test]
    fn s4_close_handshake_completes_regular_shutdown_via_quit_ending_close_talk() {
        const SHELL_NAME: &str = "S4CloseShell";

        let root = unique_temp_dir(
            "s4_close_handshake_completes_regular_shutdown_via_quit_ending_close_talk",
        );
        let _ = std::fs::remove_dir_all(&root);
        write_ghost_fixture(&root, SHELL_NAME);

        let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

        // boot 系列一式（S1 と同旨）＋ OnClose（bare quit `\-` を返す・close talk の trigger）
        // ＋ unload（Quit 経路の ShioriUnload が消費する唯一のスクリプト・Ok(Clean)）を
        // 台本化する。OnSecondChange は台本化しない——本シナリオは kanade へ Tick を一切
        // 送らないため steady pump は起こらない。
        let (backend, handle) = ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
            // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
            // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
            .get("username", Ok(None))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
            .notify("basewareversion", Ok(()))
            .get("OnClose", Ok(Some(r"\-".to_string())))
            .unload(Ok(ExitKind::Clean))
            .build();

        let surface_sink = RecordingSink::new();
        let text_sink = RecordingSink::new();
        let surface_records = surface_sink.records();

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
            system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        // ---- boot talk を Steady 到達まで駆動する（S1/S3 と同一技法・sleep 不使用) ----
        // 起動直後に CloseRequest を送ると kanade がまだ boot 系列途中（Idle〜BootVersion）
        // の可能性があり（boot 中の CloseRequest は pending_close 記録のみで即握手しない）、
        // Steady 到達を待たずに送るのは不要な不確実性を招く。boot talk が dispatcher の
        // active slot に載って発火したことを surface cue の到達で確認すれば、kanade は
        // 既に（boot talk の再生完了を待たず）Steady へ到達済みである
        // （boot.rs「boot は常に Steady{talk: None} へ完了する」・S3 と同じ論拠）。
        let mut now: u64 = 1;
        let mut boot_talk_fired = false;
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            runtime
                .dispatcher()
                .send(DispatcherMsg::Tick {
                    now: MonotonicMs(now),
                })
                .expect("dispatcher actor should still be alive while probing for the boot talk");
            now += 1;
            if !surface_records
                .lock()
                .expect("records mutex poisoned")
                .is_empty()
            {
                boot_talk_fired = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            boot_talk_fired,
            "S4: surface cue never fired after repeated Tick — boot talk did not reach \
             dispatcher's active slot within bound"
        );

        // ---- 終了要求（正規/canonical）: CloseRequest を kanade へ送る ----
        // S1〜S3 のいずれも一度も送っていない、この e2e ファイル初の「正規の終了要求」
        // （Fault 駆動ではない・successful close-talk 駆動の Quit 経路）。
        runtime
            .kanade()
            .send(KanadeMsg::CloseRequest {
                reason: CloseReason::User,
            })
            .expect("kanade actor should still be alive to receive the close request");

        // ---- close 握手の完走を有界スピン待機で確認する（Tick 注入なし・sleep なし) ----
        // OnClose GET→close talk（bare quit `\-`・空 sheet 高速経路で Tick 不要に
        // `TalkDone{Quit}` 発行）→横断アーム Unloading{Quit}→ShioriUnload という cascade は
        // 複数の実スレッド境界（kanade↔shiori 同期往復・start-relay・dispatcher・
        // per-talk spawn_talk スレッド・dispatcher 自身の inbox 経由の kanade 転送）を
        // 跨ぐため、`handle.calls()` に `RecordedCall::Unload` が現れるまで
        // `yield_now` のみで有界にスピン待機する（実時間待機・Tick 送出のいずれも伴わない）。
        let mut close_settled = false;
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            let has_unload = handle
                .calls()
                .lock()
                .expect("calls mutex poisoned")
                .iter()
                .any(|c| matches!(c, RecordedCall::Unload));
            if has_unload {
                close_settled = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            close_settled,
            "S4: Unload was never observed after CloseRequest — regular close handshake \
             (OnClose GET → close talk → TalkDone{{Quit}} → Unload) did not complete within bound"
        );

        // ---- (a) 起動系列＋close 握手系列が正典順序で発火 ----
        // 死活監視ノイズ（RecordedCall::Status）を除外して比較する（S1/S3 と同旨）。
        // 本シナリオは kanade へ Tick を一切送らないため OnSecondChange は発火しない。
        let expected_sequence = vec![
            expected_from_shiori_call(events::on_initialize(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            // username prefetch GET（OnInitialize 後・OnFirstBoot 前・R4.1・DD-9 の唯一の期待値導出経路）。
            expected_from_shiori_call(areka_kanade::resources::resource_username(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::on_first_boot(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
                // fixture ghost に永続ファイル無し＝vanish 不在ゆえ Ref0="0"（従来値同値）。
                0,
            )),
            expected_from_shiori_call(events::on_boot(
                &config,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::baseware_version(
                &config,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::on_close(
                CloseReason::User,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            RecordedCall::Unload,
        ];
        let calls_without_status: Vec<RecordedCall> = handle
            .calls()
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .cloned()
            .collect();
        assert_eq!(
            calls_without_status, expected_sequence,
            "起動系列＋close 握手系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→basewareversion→\
             OnClose→Unload）が正典順序で発火していない"
        );

        // ---- 主観測: shutdown() が全スレッド join を有界時間内に完走する（要件 7.3) ----
        // close 握手が既に Unload まで完走済み（上の有界待機で確認済み）であるため、
        // ここでの `ForceQuit` 送出は kanade が既に自発停止済み（もしくは自発停止処理の
        // 最終盤）であることの冪等パスを実地で運動させる——`shutdown()` 自身の
        // 「kanade already stopped before ForceQuit send」分岐（design.md「終了
        // （shutdown）シーケンス」・runtime.rs 3.2 の status report で code-reading のみで
        // 検証済みだった経路）を、本 e2e が初めて実地の回帰檻として固定する。
        run_bounded(
            "shutdown after S4 regular close handshake completion",
            super::E2E_BOUND,
            move || {
                let result = runtime.shutdown(CloseReason::System);
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) after the regular close handshake \
                     completes, got {result:?}"
                );
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}

// ===================== S5: close deadline 超過シナリオ（task 4.6） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S5:
// 「close deadline: close talk を意図的に完了させず、`KanadeMsg::Tick` の `now` を deadline
// 超過まで注入（既定 30_000ms を数値的に跨ぐ `now` を投函するだけ・実時間ゼロ・短縮構成
// 不要）→`Unloading{DeadlineExceeded}`→`Unload`→全 join。」を、S4（close 握手・正規の
// Quit 経路）と対をなす「close talk が自然完了しない」経路として駆動する（要件
// 7.4/7.5/7.6）。
//
// close talk の script は `\w[999999]this-never-completes\-`（`\w[999999]`＝約13.9時間
// 相当の待ち・drive.rs のコメントが示すとおり `\w[N]`＝N×50ms）にする——先頭に待ちを置く
// ことで空 sheet 高速経路（bare quit `\-`・S4 参照）を踏まず、実際に「再生完了通知が来ない」
// 状態を作る。本シナリオは close talk 開始後、dispatcher へ一切 Tick を送らない（送れば
// `\w` の経過秒が進み得る）ため、close talk は spawn 直後の待ちで恒久的に止まったまま
// になる——kanade 側の deadline 判定だけが `runtime.kanade()` への直接 `Tick` 注入で駆動する。
//
// # deadline の起点計算（`close.rs::deadline_from`・close.rs モジュール doc 参照）
// 本シナリオは S1/S4 と同じ boot-settling 技法（dispatcher への Tick 注入のみ）を使う——
// dispatcher への Tick は kanade 自身の `last_now` を一切更新しない（`KanadeMsg::Tick` を
// 受けたときのみ更新される・`schedule/steady.rs`/`schedule/close.rs` の `last_now` 更新箇所を
// 直接確認済み）。ゆえに `CloseRequest` 送出時点で kanade の `last_now` は依然 `None` であり、
// `ClosePending`→`CloseTalkWait` 遷移時の `deadline_from(None, ..)` は `None`（未確定）で
// `CloseTalkWait` に入る（close.rs「握手入口で last_now が None だった場合は deadline を
// None のまま入り、CloseTalkWait 最初の Tick 受領時点を起点に上限を設定する」）。
//
// `kanade` は `run_inbox`（`areka-kanade/src/actor.rs`）で 1 メッセージずつ**完全に同期**
// 処理する（`drive()` が OnClose の同期往復・状態遷移まで完結させてから次の inbox
// メッセージを取り出す）ため、`CloseRequest`→(直後に送る)`Tick` の到達順序は mpsc の
// FIFO 保証と合わせて完全に決定論的である——`CloseRequest` が処理し終わる（＝
// `CloseTalkWait` へ遷移済み）前に後続の `Tick` が処理されることはない。ゆえに:
// - 1本目の `Tick{now: arm_now}` → `CloseTalkWait` に入って初めて受ける Tick ゆえ
//   deadline を `arm_now + close_talk_deadline_ms` へ**確定するだけ**（超過判定はしない・
//   close.rs の `None` 分岐は比較を行わない）。
// - 2本目の `Tick{now: arm_now + close_talk_deadline_ms}` → `now >= deadline` で確実に
//   超過 → `Unloading{DeadlineExceeded}` → `ShioriUnload`。
// 2 本の Tick 送出そのものは即座に返る（inbox への enqueue のみ）ため、その後の
// `Unload` 呼出の実際の発火（kanade スレッドが実際に処理し終わる時点）は有界スピン
// 待機（`yield_now` のみ・sleep も追加 Tick も伴わない）で確認する。
#[cfg(test)]
mod s5_close_deadline {
    use super::*;

    use areka_ghost::dispatcher::DispatcherMsg;
    use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
    use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriCall, events};
    use areka_parsers::charset::DefaultEncoding;

    /// このテスト専用の一意な一時ディレクトリ（S1〜S4 の流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_spine_e2e_s5_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリーを構築する（S1/S3/S4 の
    /// `write_ghost_fixture` と同旨だが、sibling module から private item は参照できない
    /// ためローカルに複製する）。
    fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,S5TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            format!("charset,UTF-8\nname,{shell_name}\n").as_bytes(),
        )
        .expect("write shell descript.txt");
    }

    /// events 表由来の [`ShioriCall`] をこのファイル固有の [`RecordedCall`] へ変換する
    /// （S1/S4 の `expected_from_shiori_call` と同旨のローカル複製・Req 7.1）。
    fn expected_from_shiori_call(call: ShioriCall) -> RecordedCall {
        match call {
            ShioriCall::Get { id, references, .. } => RecordedCall::Get {
                id: id.to_string(),
                references,
            },
            ShioriCall::Notify { id, references, .. } => RecordedCall::Notify {
                id: id.to_string(),
                references,
            },
        }
    }

    /// 有界待機ヘルパ（S1〜S4 と同旨のローカルコピー）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: std::time::Duration, f: F) {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);
        std::thread::spawn(move || {
            f();
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(timeout).is_ok(),
            "'{what}' did not complete within {timeout:?} (possible hang)"
        );
    }

    /// S5: close deadline 超過——close talk を意図的に完了させず（`\w[999999]`＝約13.9時間
    /// 相当の待ちで恒久的に止める）、`runtime.kanade()` へ `Tick` を 2 回注入するだけで
    /// （1本目で deadline を確定・2本目で超過を跨ぐ）、`Unloading{DeadlineExceeded}`→
    /// scripted `Ok(ExitKind::Clean)` の `Unload`→`Unloaded` 観測→`StopSelf` へ完走する
    /// ことを、`runtime.shutdown()` の全スレッド join 成功をもって確認する（design「S5
    /// close deadline」・要件 7.4/7.5/7.6）。
    #[test]
    fn s5_close_deadline_exceeded_forces_termination_via_tick_injection() {
        const SHELL_NAME: &str = "S5DeadlineShell";

        let root =
            unique_temp_dir("s5_close_deadline_exceeded_forces_termination_via_tick_injection");
        let _ = std::fs::remove_dir_all(&root);
        write_ghost_fixture(&root, SHELL_NAME);

        let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

        // boot 系列一式（S1/S4 と同旨）＋ OnClose（close talk を恒久的に止める待ち script）＋
        // unload（DeadlineExceeded 系列が発行する ShioriUnload の応答・Ok(Clean)）を台本化する。
        // OnSecondChange は台本化しない——本シナリオは kanade へ boot 完了後、CloseRequest と
        // deadline 用 Tick 2 本しか送らないため steady pump は起こらない。
        let (backend, handle) = ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
            // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
            // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
            .get("username", Ok(None))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
            .notify("basewareversion", Ok(()))
            .get(
                "OnClose",
                Ok(Some(r"\w[999999]this-never-completes\-".to_string())),
            )
            .unload(Ok(ExitKind::Clean))
            .build();

        let surface_sink = RecordingSink::new();
        let text_sink = RecordingSink::new();
        let surface_records = surface_sink.records();

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
            system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        // ---- boot talk を Steady 到達まで駆動する（S1/S3/S4 と同一技法・sleep 不使用) ----
        // dispatcher への Tick は kanade 自身の last_now を更新しない（別チャンネル・別
        // 帳簿）ため、この loop を通しても kanade の last_now は None のまま維持される
        // （CONCERNS 参照）。
        let mut now: u64 = 1;
        let mut boot_talk_fired = false;
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            runtime
                .dispatcher()
                .send(DispatcherMsg::Tick {
                    now: MonotonicMs(now),
                })
                .expect("dispatcher actor should still be alive while probing for the boot talk");
            now += 1;
            if !surface_records
                .lock()
                .expect("records mutex poisoned")
                .is_empty()
            {
                boot_talk_fired = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            boot_talk_fired,
            "S5: surface cue never fired after repeated Tick — boot talk did not reach \
             dispatcher's active slot within bound"
        );

        // ---- 終了要求（正規/canonical）: CloseRequest を kanade へ送る ----
        runtime
            .kanade()
            .send(KanadeMsg::CloseRequest {
                reason: CloseReason::User,
            })
            .expect("kanade actor should still be alive to receive the close request");

        // ---- close 握手が Steady を抜けて OnClose GET を発行するまで有界スピン待機する ----
        // DD-IT-12: boot は挨拶 talk を追跡し `Steady{talk: Some(greeting)}` へ完了する。ゆえに
        // CloseRequest 受領時に挨拶 talk がまだ active なら kanade は即握手せず `pending_close`
        // に記録して `Steady{Some}` を維持し、挨拶 talk の TalkDone 受領時に初めて握手を開始する
        // （steady.rs `on_close_request` / `on_talk_done`）。この間に下の deadline 用 Tick を
        // 送ってしまうと、Tick は `Steady{Some}` の pump として消費され OnSecondChange NOTIFY を
        // 発行してしまう（CloseTalkWait の deadline を進めない）。ゆえに OnClose GET が calls() に
        // 現れる＝kanade が Steady を抜け ClosePending 以降へ遷移したことを確認してから deadline
        // 用 Tick を注入する（挨拶 TalkDone の到達は dispatcher が自律的に kanade へ転送するため
        // 追加 Tick 不要・実時間待機なし・`yield_now` のみ）。OnClose GET が現れた後の kanade は
        // ClosePending か CloseTalkWait のいずれかにあり、どちらでも下の 2 Tick は last_now を
        // 起点に deadline を確定・超過させる（ClosePending の Tick は last_now 更新のみ→続く
        // Value 応答で `deadline_from(Some)` 確定／CloseTalkWait の Tick は deadline=None を
        // 起点確定・close.rs 参照）。
        let mut handshake_reached = false;
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            let onclose_issued = handle
                .calls()
                .lock()
                .expect("calls mutex poisoned")
                .iter()
                .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"));
            if onclose_issued {
                handshake_reached = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            handshake_reached,
            "S5: OnClose GET was never issued after CloseRequest — the greeting-tracking close \
             deferral (DD-IT-12) never resolved into a close handshake within bound"
        );

        // ---- deadline 超過を Tick 2 本の注入だけで駆動する（sleep 不使用・要件 7.4) ----
        // 1本目: CloseTalkWait 入場後 初めて受ける Tick——deadline を
        // `arm_now + close_talk_deadline_ms` へ確定するだけで比較はしない
        // （close.rs の deadline=None 分岐・CONCERNS 参照）。
        let arm_now: u64 = 5_000;
        runtime
            .kanade()
            .send(KanadeMsg::Tick {
                now: MonotonicMs(arm_now),
            })
            .expect("kanade actor should still be alive to receive the deadline-arming Tick");

        // 2本目: `now >= deadline`（`arm_now + close_talk_deadline_ms`）を確実に跨ぐ値を
        // 注入する——生産既定 30_000ms を数値的に跨ぐだけで実時間はゼロ（要件 7.4）。
        let cross_now = arm_now + config.close_talk_deadline_ms;
        runtime
            .kanade()
            .send(KanadeMsg::Tick {
                now: MonotonicMs(cross_now),
            })
            .expect("kanade actor should still be alive to receive the deadline-crossing Tick");

        // ---- deadline 超過による強制終了系列の完走を有界スピン待機で確認する ----
        // （Tick 送出は上で完了済み・以降は追加 Tick も sleep も伴わない）。
        let mut deadline_settled = false;
        let deadline = std::time::Instant::now() + super::E2E_BOUND;
        while std::time::Instant::now() < deadline {
            let has_unload = handle
                .calls()
                .lock()
                .expect("calls mutex poisoned")
                .iter()
                .any(|c| matches!(c, RecordedCall::Unload));
            if has_unload {
                deadline_settled = true;
                break;
            }
            std::thread::yield_now();
        }
        assert!(
            deadline_settled,
            "S5: Unload was never observed after the deadline-crossing Tick — forced \
             termination (CloseTalkWait deadline exceeded → Unload) did not complete within bound"
        );

        // ---- (a) 起動系列＋close 開始＋強制終了系列が正典順序で発火 ----
        // 死活監視ノイズ（RecordedCall::Status）を除外して比較する（S1/S3/S4 と同旨）。
        let expected_sequence = vec![
            expected_from_shiori_call(events::on_initialize(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            // username prefetch GET（OnInitialize 後・OnFirstBoot 前・R4.1・DD-9 の唯一の期待値導出経路）。
            expected_from_shiori_call(areka_kanade::resources::resource_username(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::on_first_boot(
                &areka_kanade::ExecutionSnapshot::INACTIVE,
                // fixture ghost に永続ファイル無し＝vanish 不在ゆえ Ref0="0"（従来値同値）。
                0,
            )),
            expected_from_shiori_call(events::on_boot(
                &config,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::baseware_version(
                &config,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            expected_from_shiori_call(events::on_close(
                CloseReason::User,
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            )),
            RecordedCall::Unload,
        ];
        let calls_without_status: Vec<RecordedCall> = handle
            .calls()
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .filter(|c| !matches!(c, RecordedCall::Status))
            .cloned()
            .collect();
        assert_eq!(
            calls_without_status, expected_sequence,
            "起動系列＋close 開始＋強制終了系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→\
             basewareversion→OnClose→Unload）が正典順序で発火していない"
        );

        // ---- 主観測: shutdown() が全スレッド join を有界時間内に完走する（要件 7.3) ----
        // deadline 超過による強制終了が既に Unload まで完走済み（上の有界待機で確認済み）
        // であるため、ここでの `ForceQuit` 送出は kanade が既に自発停止済みであることの
        // 冪等パスを実地で運動させる（S4 と同旨）。close talk（`\w[999999]` で止まった
        // まま）は dispatcher の active slot に残っているはずだが、dispatcher への Close
        // 送出は稼働中 active talk へ `SakuraMsg::Close` を送って即座に中断させてから
        // join する（`close_active_if_any`・dispatcher.rs）ため、恒久的に止まった close
        // talk があっても shutdown は有界時間内に完走する（CONCERNS 参照）。
        run_bounded(
            "shutdown after S5 close deadline exceeded",
            super::E2E_BOUND,
            move || {
                let result = runtime.shutdown(CloseReason::System);
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) after the deadline-exceeded forced \
                     termination completes, got {result:?}"
                );
            },
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}

// ===================== S6: 全断線（段階的解体）シナリオ（task 4.7） =====================
//
// design.md「アクター別の停止経路（正本）」表・「シナリオ網羅（要件 7.5）」節・S6:
// 「全断線（段階的解体）: `into_parts` で分解し、①`DispatcherMsg::Close` 送出→dispatcher
// join（Close-only アクターの正規停止）②`KanadeMsg::Close` 送出→kanade join（運行意味論を
// 経ない素の停止）③残る senders を全 drop→shiori actor（kanade の `shiori_tx` drop による
// inbox 切断）・down-relay（shiori 停止による `down_tx` drop）・start-relay（kanade 停止に
// よる `start_tx` drop）が切断伝播だけで有界時間内に正常終了することを join で確認する。
// 純粋な「全 Sender drop 一斉解放」は Sender 環（停止経路マトリクス参照）ゆえ構造的に
// 成立しない——本シナリオはマトリクスの全行（Close 経路×2・切断経路×3）を 1 シナリオで
// 検証する再定義である。」
//
// `GhostRuntime`/`GhostParts` は `shiori_tx` を保持しない（design「GhostRuntime は
// shiori_tx を保持しない」・runtime.rs 3.1/3.2・`into_parts` の rustdoc）。`shiori_tx` は
// kanade 自身のアクタースレッドが `spawn_kanade(config, shiori_tx, start_tx)` の引数として
// **内部に**保持し続ける（`run_inbox` のクロージャが `shiori`/`sakura` を move キャプチャ
// する・`areka-kanade/src/actor.rs::spawn_kanade`）。ゆえに kanade スレッドが（`Close` によ
// る即時 `Break` で）終了しクロージャが return するとき、`shiori_tx`・`start_tx`（＝
// kanade にとっての `sakura` パラメータ）はその関数フレームの終了と共に**自動的に**
// drop される——手動 `drop()` は一切不要（そもそもこのテストは `shiori_tx`/`start_tx` を
// 握っていないため不可能でもある）。同様に `down_tx` は shiori actor 自身が受信ループの
// 全生涯にわたり保持する（task 1.4 の設計・`on_down` 保持）ため、shiori スレッドが終了
// すればそれも自動的に drop される。`ActorHandle::join` はスレッド関数の完全な終了
// （＝これらの drop が既に起こった後）を待ってから返るため、各 join の成功はその
// drop が既に起きたことの直接証跡になる。
#[cfg(test)]
mod s6_full_disconnect {
    use super::*;

    use areka_ghost::dispatcher::DispatcherMsg;
    use areka_ghost::{
        GhostBootOptions, GhostHandles, GhostParts, ShioriWiring, SystemVarWiring, TickerMode, boot,
    };
    use areka_kanade::KanadeMsg;
    use areka_parsers::charset::DefaultEncoding;

    use areka_actor::{ActorError, ActorHandle};

    /// このテスト専用の一意な一時ディレクトリ（S1〜S5 の流儀を踏襲）。
    fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_spine_e2e_s6_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の解決可能なゴーストツリーを構築する（S1/S3/S4/S5 の
    /// `write_ghost_fixture` と同旨だが、sibling module から private item は参照できない
    /// ためローカルに複製する）。
    fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
        let ghost_master = root.join("ghost").join("master");
        std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
        std::fs::write(
            ghost_master.join("descript.txt"),
            b"charset,UTF-8\nname,S6TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join("master");
        std::fs::create_dir_all(&shell_dir).expect("create shell/master");
        std::fs::write(
            shell_dir.join("descript.txt"),
            format!("charset,UTF-8\nname,{shell_name}\n").as_bytes(),
        )
        .expect("write shell descript.txt");
    }

    /// `ActorHandle::join` を有界時間で観測する（S2〜S3 の `join_bounded` と同旨の
    /// ローカルコピー）。
    fn join_bounded(
        what: &str,
        timeout: std::time::Duration,
        handle: ActorHandle,
    ) -> Result<(), ActorError> {
        let (res_tx, res_rx) = std::sync::mpsc::sync_channel::<Result<(), ActorError>>(0);
        std::thread::spawn(move || {
            let _ = res_tx.send(handle.join());
        });
        match res_rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
        }
    }

    const BOUND: std::time::Duration = super::E2E_BOUND;

    /// S6: 全断線（段階的解体）——`into_parts` で分解し、①dispatcher へ `Close`→join
    /// （Close-only アクターの正規停止・「アクター別の停止経路」表の dispatcher 行）②
    /// kanade へ raw `Close`→join（運行意味論——OnClose NOTIFY／Unload 等——を一切経ない
    /// 「非常口」停止。S2〜S5 のいずれも駆動していない、design「停止規約の Close」の
    /// bare な構造的停止・同表の kanade 行）③以降は手動 `drop()` も追加送信も一切行わずに
    /// shiori／down-relay／start-relay を join し、②で kanade スレッドが終了したことに
    /// 伴う自動 drop カスケードだけで全て有界時間内に自然終了することを確認する（同表の
    /// shiori／down-relay／start-relay の 3 行）。合計 5 join で「アクター別の停止経路」
    /// マトリクスの全 5 行（Close 経路×2・切断経路×3）を 1 シナリオで検証する（design が
    /// 述べる「マトリクスの全行…を1シナリオで検証する再定義」・要件 7.4/7.5/7.6）。
    #[test]
    fn s6_full_disconnect_staged_teardown_terminates_all_five_actors_within_bound() {
        const SHELL_NAME: &str = "S6DisconnectShell";

        let root = unique_temp_dir(
            "s6_full_disconnect_staged_teardown_terminates_all_five_actors_within_bound",
        );
        let _ = std::fs::remove_dir_all(&root);
        write_ghost_fixture(&root, SHELL_NAME);

        // boot() 内部の同期 kanade 往復（OnInitialize NOTIFY→OnFirstBoot GET→OnBoot GET→
        // basewareversion NOTIFY）が panic しない最小限の台本のみ用意する——本シナリオの
        // 焦点は解体の「構造」であり、boot talk（`\s[0]hello\e`）の再生完了までは駆動
        // しない（dispatcher の active slot に乗ったまま未進行でも、後続①の
        // `DispatcherMsg::Close` が既存の Close funnel で安全に中断させる・dispatcher.rs
        // 自身の単体テストで既に確認済みの挙動・CONCERNS 参照）。OnClose／Unload は台本化
        // しない——本シナリオは kanade へ `CloseRequest`／`ForceQuit` のいずれも送らず、
        // 運行意味論を経ない raw `KanadeMsg::Close` のみで停止させるため、shiori backend
        // の `unload()` が呼ばれることはない。
        let (backend, _handle) = ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
            // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
            // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
            .get("username", Ok(None))
            .get("OnFirstBoot", Ok(None))
            .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
            .notify("basewareversion", Ok(()))
            .build();

        let options = GhostBootOptions {
            ghost_root: root.clone(),
            default_encoding: DefaultEncoding::Utf8,
            shiori: ShioriWiring::Custom(Box::new(move || {
                Ok(Box::new(backend) as Box<dyn ShioriBackend>)
            })),
            sinks: vec![
                Box::new(RecordingSink::new()),
                Box::new(RecordingSink::new()),
            ],
            system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
            app_profile_dir: None,
            ticker: TickerMode::Disabled,
        };

        let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

        let parts = runtime.into_parts();
        let GhostParts {
            kanade,
            dispatcher,
            handles,
            ..
        } = parts;
        let GhostHandles {
            kanade: kanade_handle,
            dispatcher: dispatcher_handle,
            shiori: shiori_handle,
            start_relay: start_relay_handle,
            down_relay: down_relay_handle,
            ticker: _,
            sylphya: _,
        } = handles;

        // ---- ①: DispatcherMsg::Close → dispatcher join ----
        // dispatcher の唯一の停止経路（「アクター別の停止経路」表・Close-only。self-sender
        // 保持ゆえ切断では構造的に止まらない）。boot talk が active slot に乗ったまま
        // 未進行でも、Close funnel が中断させて正規停止することを直接運動させる。
        dispatcher
            .send(DispatcherMsg::Close)
            .expect("dispatcher actor should still be alive to receive Close");
        join_bounded("① dispatcher join after Close", BOUND, dispatcher_handle)
            .expect("dispatcher should terminate after its only stop path (Close)");

        // ---- ②: KanadeMsg::Close → kanade join ----
        // kanade の raw「非常口」停止（`KanadeMsg::Close` は step を経ず即時 Break・
        // areka-kanade/src/actor.rs::spawn_kanade）。OnClose NOTIFY も Unload も一切
        // 呼ばれない——S2〜S5 が駆動する運行意味論（ForceQuit／CloseRequest／Fault）とは
        // 異なる、この e2e で初めて運動させる経路。kanade スレッドが Break で return する
        // 時点で、その関数フレームが内部に保持していた `shiori_tx`（shiori actor 自身の
        // inbox 送信端）・`start_tx`（start-relay の上流送信端）が自動的に drop される。
        kanade
            .send(KanadeMsg::Close)
            .expect("kanade actor should still be alive to receive raw Close");
        join_bounded("② kanade join after raw Close", BOUND, kanade_handle).expect(
            "kanade should terminate on its bare Close stop path without running any \
             shutdown semantics (no OnClose NOTIFY, no Unload)",
        );

        // ---- ③: 以降は手動 drop も追加送信も一切行わず、自動 drop カスケードのみで
        // shiori／down-relay／start-relay を有界時間内に join する ----
        // shiori actor: ②で kanade スレッドが終了した時点で、kanade が内部に保持していた
        // shiori_tx が既に drop 済み——shiori actor の inbox 受信（blocking recv）はその
        // Sender 側が尽きた時点で Err を返し、受信ループが正常終了する。
        join_bounded(
            "③ shiori actor natural termination via shiori_tx drop cascading from ②",
            BOUND,
            shiori_handle,
        )
        .expect(
            "shiori actor should terminate naturally once kanade's internally-held shiori_tx \
             is dropped as a consequence of kanade's actor thread exiting in step ②",
        );

        // down-relay: shiori actor が終了した時点で、shiori が内部に保持していた down_tx が
        // 同様に drop される——down-relay の上流（down_rx）が切断され自然終了する（shiori
        // actor の終了は直前の join で既に観測済みなので、この時点で down_tx は既に
        // drop されている）。
        join_bounded(
            "③ down-relay natural termination via down_tx drop cascading from shiori's exit",
            BOUND,
            down_relay_handle,
        )
        .expect(
            "down-relay should terminate naturally once shiori's internally-held down_tx is \
             dropped as a consequence of shiori's actor thread exiting",
        );

        // start-relay: ②で kanade スレッドが終了した時点で start_tx も既に drop 済み
        // （shiori_tx と同じ根本原因・②の時点で既に成立しているため、shiori／down-relay
        // の後に join しても機能的な前後関係はない——宣言順序の都合でここに置く）。
        join_bounded(
            "③ start-relay natural termination via start_tx drop cascading from ②",
            BOUND,
            start_relay_handle,
        )
        .expect(
            "start-relay should terminate naturally once kanade's internally-held start_tx is \
             dropped as a consequence of kanade's actor thread exiting in step ②",
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}

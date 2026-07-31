//! バルーン面切替の決定論的エンドツーエンド観測（要件 5.1/5.2/5.3/5.4）。
//!
//! fixture のさくらスクリプトを **直接入力**し、`parse → compile → cue → seriko → 表示指令発行`
//! までの貫通経路を、注入した `SakuraMsg::Tick` のみで（sleep/polling を一切用いず）決定論的に
//! 観測する統合テスト。本体コード（src/）は一切変更せず、既存 API のみで組む（新規 API 追加なし）。
//!
//! # 同期チェーン（sleep/polling ゼロ・要件 5.3）
//!
//! 1. talk アクター（`spawn_talk`）へ fixture script を直入力し、`TalkHandle::inbox` へ
//!    必要な `SakuraMsg::Tick(t)` を注入する。
//! 2. `done_rx.recv_timeout(5s)` で `TalkDone` を受領する（talk が `\e` で自然終端）。
//! 3. talk スレッド終了で、surface_sink へ move 済みの `SerikoSink`（seriko inbox の**唯一**の
//!    `Sender`）が drop され、seriko inbox が **disconnect** する → seriko の `run_inbox` が正常終了。
//! 4. seriko `ActorHandle::join()`（唯一の同期点）でスレッド終了を待つ。先に送った `Cue` は
//!    FIFO 単一スレッドゆえ join 復帰時には処理済みである。
//! 5. `MockSurfaceOutput::records()` ハンドルから発行列を照合する（決定論）。
//!
//! # 経路の要所
//!
//! - `BalloonSurface`（`\b`）は `cue_target_of` が `Some(CueTarget::Shell)` へ分類する＝
//!   表示系（seriko）が action する（テキスト状態機械へは行かない・要件 3.2）。broadcast ゆえ
//!   全 cue は両 sink へ届くが、text 系は本 E2E の観測対象外——破棄専用の [`NullTextSink`] で足りる。
//! - 解決テーブルは空 alias 表（`BTreeMap::new()`）で足りる。数値 key（`"0"`/`"2"`/`"-1"`）は
//!   alias を引かず数値枝で解決できる（`resolve` / `resolve_balloon_key` の数値枝）。

use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::{
    spawn_talk, ActorKey, ChoiceWaiting, CueSink, SakuraMsg, StartTalk, SystemVarSnapshot, TalkCue,
    TalkDone, TalkEndReason, TalkId,
};
use areka_seriko::{
    spawn_seriko, BindResolver, DisplayCommand, MockSurfaceOutput, SerikoLoopConfig, SurfaceResolver,
};
use std::collections::BTreeMap;
use std::time::Duration;

/// `spawn_talk` の done ポート境界（`D: From<TalkDone> + From<ChoiceWaiting>`）を満たす
/// test-local な合流 enum。両通知は同一ポートを流れる（DD-6）が、本 E2E の fixture に選択肢
/// （`\q`）は無く `ChoiceWaiting` は発生しない——境界充足のためだけの受け皿である。
enum TalkNotice {
    Done(TalkDone),
    ChoiceWaiting(ChoiceWaiting),
}
impl From<TalkDone> for TalkNotice {
    fn from(done: TalkDone) -> Self {
        TalkNotice::Done(done)
    }
}
impl From<ChoiceWaiting> for TalkNotice {
    fn from(waiting: ChoiceWaiting) -> Self {
        TalkNotice::ChoiceWaiting(waiting)
    }
}

/// text 系（→emo text-layer⑥）に相当する broadcast の 2 つ目のスロットを破棄する test-local sink。
///
/// 本 E2E は表示系（seriko）が発行する DisplayCommand だけを観測する。broadcast ゆえ本 sink にも
/// 全 cue（`Text`/`Wait`/`Emote` 等）が届くが、いずれも観測対象外ゆえ握って捨てる（記録しない）。
struct NullTextSink;

impl CueSink for NullTextSink {
    fn emit(&mut self, _cue: TalkCue) {
        // 破棄のみ（本 E2E の観測対象は seriko の表示指令だけ）。
    }
}

/// 1 シナリオを貫通駆動し、seriko が発行した表示指令列（送信順）を返す。
///
/// `script` を直入力する talk を起動し、`ticks` を順に注入して駆動する。同期は上記モジュール
/// doc の「Tick 注入 → done_rx.recv → SerikoSink drop による seriko disconnect → seriko join
/// → records 照合」のみで行う（sleep/polling なし・要件 5.3）。シナリオごとに新しい
/// `MockSurfaceOutput`・新しい talk/seriko を立て、状態を持ち越さない。
fn run_scenario(script: &str, ticks: &[f64]) -> Vec<DisplayCommand> {
    // ── seriko アクター（表示指令の消費側）──
    // 空 alias 表でも数値 key は数値枝で解決できる。BindSet は空（M-boot にバルーン着せ替えは
    // 無く、本 E2E のシェル Show も binds 空で足りる）。records() ハンドルは move 前に取得する。
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (seriko_sink, seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::from_ids([]),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );

    // ── talk アクター（fixture script 直入力）──
    // seriko_sink（＝seriko inbox の唯一の Sender）を surface_sink へ move する。clone を残さない
    // ことで、talk スレッド終了時の drop が seriko inbox を disconnect させる（同期チェーンの要）。
    let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkNotice>();
    let talk = spawn_talk(
        StartTalk {
            epilogue: Vec::new(),
            script: script.to_string(),
            talk_id: TalkId(2),
        },
        done_tx,
        vec![Box::new(seriko_sink), Box::new(NullTextSink)],
        SystemVarSnapshot::default(),
    );

    // ── 注入 Tick のみで駆動（sleep/polling ゼロ・要件 5.3）──
    for &t in ticks {
        talk.inbox
            .send(SakuraMsg::Tick(t))
            .expect("Tick を投函できること");
    }

    // ── 同期チェーン ──
    // (1) TalkDone 受領（talk 自然終端＝`\e`）。fixture は全て `\e` 終端ゆえ Ended。
    let TalkNotice::Done(done) = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("fixture script は `\\e` で自然終端し TalkDone を返すべき")
    else {
        panic!("選択肢を含まない fixture では ChoiceWaiting は流れない（TalkDone のみ）");
    };
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended で終端する");

    // (2) talk スレッド終了→move 済み SerikoSink（唯一の Sender）drop→seriko inbox disconnect。
    talk.actor.join().expect("talk body は Break 後に正常終了する");
    // (3) seriko join（唯一の同期点）。先送りの Cue は FIFO 単一スレッドゆえ join 復帰時に処理済み。
    seriko.join().expect("seriko は disconnect で正常終了する");

    // (4) 発行列のスナップショットを返す（決定論）。
    let recorded = records.lock().expect("records mutex poisoned");
    recorded.clone()
}

/// シナリオ1（要件 5.1）: `\b[ID]` を含む fixture script 直入力で「面切替指令（ID 指定）」を
/// mock 表示 sink 上に観測する。
///
/// `\b[2]\e` → seriko が `ShowBalloon{scope:"0", surface_id:2}` をちょうど 1 件発行する
/// （既定 scope は `\p[n]` 未指定ゆえ "0"・compile が転写）。
#[test]
fn balloon_show_is_observed_end_to_end() {
    let records = run_scenario(r"\b[2]\e", &[0.0]);
    assert_eq!(
        records,
        vec![DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "\\b[2] は ShowBalloon{{scope:0, surface_id:2}} を 1 件観測させる（5.1）"
    );
}

/// シナリオ2（要件 5.2）: `\b[-1]`（非表示センチネル）を含む fixture script 直入力で
/// 「非表示指令」を mock 表示 sink 上に観測する。`\w1`（=50ms）を跨ぐ Tick 時刻で駆動する。
///
/// `\b[2]\w1\b[-1]\e` → Tick(0.0) で `ShowBalloon{2}`、Tick(0.05) で `HideBalloon` を順に観測。
/// `\w1` の時刻は compile と同一の `Duration::from_millis(50).as_secs_f64()` で導出し、10 進
/// 直書き（0.05）の f64 表現誤差で境界包含判定（`at <= tick`）を誤らないようにする。
#[test]
fn balloon_hide_is_observed_end_to_end() {
    // `\w1`＝1×50ms。compile 側の累積と同一計算で導出する（リテラル直書きの誤差回避）。
    let at_hide = Duration::from_millis(50).as_secs_f64();
    let records = run_scenario(r"\b[2]\w1\b[-1]\e", &[0.0, at_hide]);
    assert_eq!(
        records,
        vec![
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("0"),
                surface_id: 2,
                pattern: PatternState::default(),
            },
            DisplayCommand::HideBalloon {
                scope: ActorKey::from("0"),
            },
        ],
        "\\b[2]→\\b[-1] は ShowBalloon(2)→HideBalloon の順に観測させる（5.2）"
    );
}

/// シナリオ3（要件 1.2 の E2E 面）: レガシー裸形 `\bN` がブラケット形 `\b[N]` と**同一**の
/// 表示指令を観測させる。
///
/// `\b2\e` → `ShowBalloon{2}`。ブラケット形 `\b[2]\e` の観測列と厳密一致することも直接対比し、
/// 裸形が本文数字を漏らさず（`\b2` の `2` は面数字として消費される）同一経路へ乗ることを固定する。
#[test]
fn bare_balloon_matches_bracket_form_end_to_end() {
    let bare = run_scenario(r"\b2\e", &[0.0]);
    assert_eq!(
        bare,
        vec![DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "裸形 \\b2 は ShowBalloon{{2}} を観測させる（1.2 の E2E 面）"
    );

    // ブラケット形と厳密同一の観測列であることを直接対比する。
    let bracket = run_scenario(r"\b[2]\e", &[0.0]);
    assert_eq!(
        bare, bracket,
        "裸形 \\b2 とブラケット形 \\b[2] は同一の表示指令列を生む（1.2）"
    );
}

/// シナリオ4（要件 4.3）: 同一面への冪等な再指定は表示指令を再発行しない。
///
/// `\b[2]\b[2]\e`（両 cue とも at=0.0）→ Tick(0.0) で両 cue が発火するが、2 件目は per-scope
/// 状態不変ゆえ発行が抑止され、`ShowBalloon{2}` はちょうど 1 件のみ観測される（既存冪等ガード）。
#[test]
fn same_balloon_twice_emits_once_end_to_end() {
    let records = run_scenario(r"\b[2]\b[2]\e", &[0.0]);
    assert_eq!(
        records,
        vec![DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "同一面 \\b[2] の冪等な再指定は ShowBalloon を 1 件のみ発行する（4.3）"
    );
}

/// シナリオ5（要件 4.6）: シェル面切替とバルーン面切替の混在で、双方が独立に記録される。
///
/// `\s[0]\b[2]\e`（両 cue とも at=0.0・記述順 FIFO）→ Tick(0.0) で `\s[0]`（シェル面＝seriko の
/// `Emote` 経路・数値 "0"→Show(0)）と `\b[2]`（バルーン面＝`BalloonSurface` 早期分岐）が順に
/// 発火し、`Show{scope:"0", surface_id:0, binds:空}` と `ShowBalloon{scope:"0", surface_id:2}`
/// が互いに干渉せず送信順どおりに記録される（別 map・別発行）。
#[test]
fn shell_and_balloon_mixed_recorded_independently_end_to_end() {
    let records = run_scenario(r"\s[0]\b[2]\e", &[0.0]);
    assert_eq!(
        records,
        vec![
            DisplayCommand::Show {
                scope: ActorKey::from("0"),
                surface_id: 0,
                binds: BindSet::from_ids([]),
                pattern: PatternState::default(),
            },
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("0"),
                surface_id: 2,
                pattern: PatternState::default(),
            },
        ],
        "シェル面 Show(0) とバルーン面 ShowBalloon(2) が独立に記録される（4.6）"
    );
}

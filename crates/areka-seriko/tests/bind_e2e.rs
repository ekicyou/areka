//! bind（着せ替え）決定論エンドツーエンド観測（要件 2.3/5.1/5.2/5.3/5.4/5.5）。
//!
//! 名前キー bind の on/off 列を含むさくらスクリプトを **直接入力**し、
//! `parse → compile → cue → seriko → 表示指令発行` までの貫通経路を、注入した
//! `SakuraMsg::Tick` のみで（sleep/polling を一切用いず）決定論的に観測する統合テスト。
//! 本体コード（src/）は一切変更せず、既存 API のみで組む（新規 API 追加なし・test-only）。
//!
//! # 同期チェーン（sleep/polling ゼロ・要件 5.3）
//!
//! 1. talk アクター（`spawn_talk`）へ fixture script を直入力し、`TalkHandle::inbox` へ
//!    必要な `SakuraMsg::Tick(t)` を注入する。
//! 2. `done_rx.recv_timeout(5s)` で `TalkDone` を受領する（talk が `\e` で自然終端）。
//! 3. talk スレッド終了で、CuePlayer へ move 済みの `SerikoSink`（seriko inbox の**唯一**の
//!    `Sender`）が drop され、seriko inbox が **disconnect** する → seriko の `run_inbox` が正常終了。
//! 4. seriko `ActorHandle::join()`（唯一の同期点）でスレッド終了を待つ。先に送った `Cue` は
//!    FIFO 単一スレッドゆえ join 復帰時には処理済みである。
//! 5. `MockSurfaceOutput::records()` ハンドルから発行列を照合する（決定論）。
//!
//! # bind パイプラインの要所（期待値の根拠）
//!
//! - `\s[1000]` → `Emote{key:"1000"}`（既定 scope "0"）。空 alias 表でも数値枝で `Show(1000)` へ
//!   解決し、未知 scope への初回 Show ゆえ `Show{scope:"0", 1000, binds: 静的既定}` を発行する。
//! - `\![bind,腕,伸び,0]` → compile → `Custom{command:"bind", params:["腕","伸び","0"]}` →
//!   seriko の名前自己選別（`name=="bind"`）→ `parse_bind_directive` → `Apply{腕,伸び,on:false}` →
//!   `scope_namespace("0")=Sakura` → `resolve(Sakura,"腕","伸び")=Some(1100)` → `apply_bind("0",1100,false)`。
//!   scope "0" は `Shown(1000)` ゆえ集合変化が `Changed(Show{"0", 1000, 集合−1100})` を再発行させる（D5）。
//! - 解決不能な bind（宣言に無い `(未知,パーツ)`）は `resolve==None` で `error!`＋skip・状態不変・
//!   **発行が増えない**（R5.2）。文字表示（`Text`）は seriko では担当外（emo-text 行き）ゆえ良性 skip し、
//!   text 系 sink へは broadcast で無変形に届く（両ストリームは交差しない・R2.3）。
//!
//! # test-local fixture（R5.5）
//!
//! 宣言済み bindgroup 名を持つ最小構成の名前解決表 [`test_bind_resolver`] を**in-code で自前用意**する
//! （`(腕,伸び)→1100`・`(頬,赤面)→1200`）。静的既定 bind 集合は解決可能 id を含む `{1100,1207}` とし、
//! `腕,伸び` の on/off が確定的に集合を出入りさせる（off で 1100 が抜け `{1207}`、on で戻る）。

use areka_emo_compose::BindSet;
use areka_sakura::{
    spawn_talk, ActorKey, CueCommand, CueSink, SakuraMsg, StartTalk, SystemVarSnapshot, TalkCue,
    TalkDone, TalkEndReason, TalkId,
};
use areka_seriko::{spawn_seriko, BindResolver, DisplayCommand, MockSurfaceOutput, SurfaceResolver};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// text 系（→emo text-layer⑥）に相当する broadcast の 2 つ目のスロットを破棄する test-local sink。
///
/// bind のみを観測するシナリオでは、text 側へ届く全 cue（`ClearAll`/`Text`/`Custom` 等）を握って
/// 捨てる（記録しない）。本 E2E の観測対象は seriko の表示指令だけ。
struct NullTextSink;

impl CueSink for NullTextSink {
    fn emit(&mut self, _cue: TalkCue) {
        // 破棄のみ。
    }
}

/// text 系スロットを埋めつつ、届いた cue を共有蓄積へ FIFO 記録する sink（emo-text 非汚染の観測用）。
///
/// broadcast ゆえ本 sink にも seriko と同一の全 cue 列が届く。`records()` の Arc クローンを通じて、
/// talk スレッドへ move した後でもテスト側が受信 cue（特に `Text`）を照合できる。emo-text 側の
/// benign な `Custom` skip は単体で証明済みゆえ、ここでは e2e 面で「text ストリームが bind の
/// 割り込みで汚染されず素通しされる」ことのみを観測する（R2.3）。
struct RecordingTextSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl RecordingTextSink {
    fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }
}

impl CueSink for RecordingTextSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("RecordingTextSink records mutex poisoned")
            .push(cue);
    }
}

/// 宣言済み bindgroup 名を持つ最小構成の test-local 名前解決表（R5.5・自前 fixture）。
///
/// 本体側（sakura）に `(腕,伸び)→1100`・`(頬,赤面)→1200` の 2 宣言を持つ。相方側（kero）は空
/// （emo2 に kero bindgroup 無し）。`腕,伸び`（→1100）は静的既定集合 `{1100,1207}` に含まれるため、
/// その on/off が確定的に集合へ出入りする。宣言に無い `(未知,パーツ)` は `resolve==None`（R5.2）。
fn test_bind_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("腕".into(), "伸び".into()), 1100);
    sakura.insert(("頬".into(), "赤面".into()), 1200);
    // mustselect 空集合＝全カテゴリ非排他（従来 additive の byte 同値）。専用 mustselect e2e は task 10.4。
    BindResolver::new(sakura, BTreeMap::new(), BTreeSet::new(), BTreeSet::new())
}

/// 既定 scope "0"・指定 bind 集合を載せたシェル面 `Show` 指令を組む（期待値ヘルパ）。
fn show(surface_id: u32, ids: impl IntoIterator<Item = u32>) -> DisplayCommand {
    DisplayCommand::Show {
        scope: ActorKey::from("0"),
        surface_id,
        binds: BindSet::from_ids(ids),
    }
}

/// 1 シナリオを貫通駆動し、seriko が発行した表示指令列（送信順）を返す。
///
/// `script` を直入力する talk を起動し、`ticks` を順に注入して駆動する。broadcast の 2 つ目の
/// スロットには呼び手が与えた `text_sink` を差し込む（bind のみ観測なら [`NullTextSink`]、
/// text 非汚染観測なら [`RecordingTextSink`]）。seriko には非空の静的既定集合 `{1100,1207}` と
/// test-local [`test_bind_resolver`] を注入する（R5.5）。同期はモジュール doc の
/// 「Tick 注入 → done_rx.recv → SerikoSink drop による seriko disconnect → seriko join → records 照合」
/// のみで行う（sleep/polling なし・R5.3）。シナリオごとに新しい状態を立て、持ち越さない。
fn run_with_text_sink(
    script: &str,
    ticks: &[f64],
    text_sink: Box<dyn CueSink + Send>,
) -> Vec<DisplayCommand> {
    // ── seriko アクター（表示指令の消費側）──
    // 空 alias 表（数値 key は数値枝で解決）・非空の静的既定 bind 集合 `{1100,1207}`（解決可能 id
    // 1100 を含む）・test-local bind 名前解決層。records() ハンドルは move 前に取得する。
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (seriko_sink, seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::from_ids([1100, 1207]),
        test_bind_resolver(),
        out,
    );

    // ── talk アクター（fixture script 直入力）──
    // seriko_sink（＝seriko inbox の唯一の Sender）を broadcast スロット第 1 へ move する。clone を
    // 残さないことで、talk スレッド終了時の drop が seriko inbox を disconnect させる（同期チェーンの要）。
    let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkDone>();
    let talk = spawn_talk(
        StartTalk {
            script: script.to_string(),
            talk_id: TalkId(8),
        },
        done_tx,
        vec![Box::new(seriko_sink), text_sink],
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
    let done = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("fixture script は `\\e` で自然終端し TalkDone を返すべき");
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended で終端する");

    // (2) talk スレッド終了→move 済み SerikoSink（唯一の Sender）drop→seriko inbox disconnect。
    talk.actor.join().expect("talk body は Break 後に正常終了する");
    // (3) seriko join（唯一の同期点）。先送りの Cue は FIFO 単一スレッドゆえ join 復帰時に処理済み。
    seriko.join().expect("seriko は disconnect で正常終了する");

    // (4) 発行列のスナップショットを返す（決定論）。
    let recorded = records.lock().expect("records mutex poisoned");
    recorded.clone()
}

/// bind のみを観測する（text スロットは破棄）薄いラッパ。
fn run_scenario(script: &str, ticks: &[f64]) -> Vec<DisplayCommand> {
    run_with_text_sink(script, ticks, Box::new(NullTextSink))
}

/// シナリオ1（要件 5.1）: on の bind 積算と Show 再発行を mock 上で観測する。
///
/// `\s[1000]\![bind,腕,伸び,0]\e` を Tick(0.0) で駆動すると、`\s[1000]` が静的既定集合を載せた
/// Show を発行し（`{1100,1207}`）、続く `\![bind,腕,伸び,0]`（腕/伸び→1100 を off）が集合から 1100 を
/// 除いた新集合 `{1207}` で**現 surface(1000) を再発行**する（表示発行列に期待どおりの差分が現れる）。
#[test]
fn bind_off_reissues_show_with_updated_set_end_to_end() {
    let records = run_scenario(r"\s[1000]\![bind,腕,伸び,0]\e", &[0.0]);
    assert_eq!(
        records,
        vec![show(1000, [1100, 1207]), show(1000, [1207])],
        "初回 Show（既定 {{1100,1207}}）→ bind off で 1100 を除いた {{1207}} で再発行（R5.1）"
    );
}

/// シナリオ2（要件 5.1/3.4）: off→on の復帰で集合が既定へ戻り、その都度 Show が再発行される。
///
/// `\s[1000]\![bind,腕,伸び,0]\![bind,腕,伸び,1]\e` を Tick(0.0) で駆動すると、off が 1100 を除いて
/// `{1207}` で再発行し、続く on が 1100 を戻して `{1100,1207}` で再発行する（積算保持・R3.4）。
#[test]
fn bind_off_then_on_round_trips_and_reissues_end_to_end() {
    let records = run_scenario(r"\s[1000]\![bind,腕,伸び,0]\![bind,腕,伸び,1]\e", &[0.0]);
    assert_eq!(
        records,
        vec![
            show(1000, [1100, 1207]),
            show(1000, [1207]),
            show(1000, [1100, 1207]),
        ],
        "off で {{1207}}・on で {{1100,1207}} へ復帰し、その都度 Show を再発行（R5.1/3.4）"
    );
}

/// シナリオ3（要件 3.6・D9・冪等）: 同一 off の再指定は集合不変ゆえ再発行しない。
///
/// `\s[1000]\![bind,腕,伸び,0]\![bind,腕,伸び,0]\e` を Tick(0.0) で駆動すると、1 回目の off で
/// `{1207}` を再発行し、2 回目の off は結果集合が同値（1100 は既に不在）ゆえ **Show を追加しない**
/// （表示発行列は 2 件のまま・冪等ガード・D9）。
#[test]
fn bind_off_twice_is_idempotent_no_third_show_end_to_end() {
    let records = run_scenario(r"\s[1000]\![bind,腕,伸び,0]\![bind,腕,伸び,0]\e", &[0.0]);
    assert_eq!(
        records,
        vec![show(1000, [1100, 1207]), show(1000, [1207])],
        "2 回目の off は集合不変ゆえ再発行しない（冪等・第 3 の Show なし・R3.6/D9）"
    );
}

/// シナリオ4（要件 5.2・KEY）: 解決不能な bind は表示発行を増やさない（ログ事象として skip）。
///
/// `\s[1000]\![bind,未知,パーツ,1]\e` を Tick(0.0) で駆動すると、`\s[1000]` の初回 Show のみが記録され、
/// 宣言に無い `(未知,パーツ)` の bind は `resolve==None` で `error!`＋skip・状態不変となり **Show が
/// 増えない**（表示発行列は 1 件のまま）。これが R5.2 の核心的判別（解決不能で発行が増えないこと）。
#[test]
fn unresolvable_bind_does_not_grow_display_list_end_to_end() {
    let records = run_scenario(r"\s[1000]\![bind,未知,パーツ,1]\e", &[0.0]);
    assert_eq!(
        records,
        vec![show(1000, [1100, 1207])],
        "解決不能な bind は発行を増やさない（初回 Show のみ・error!＋skip・R5.2）"
    );
}

/// シナリオ5（要件 2.3）: bind と文字表示が混在しても、両ストリームが交差汚染しない。
///
/// `\s[1000]\![bind,腕,伸び,0]アヒルやアヒル\e` を注入 Tick で駆動すると:
/// (a) seriko 側は bind の再発行を反映（`Show{1000,{1100,1207}}` → `Show{1000,{1207}}}`）——テキスト
///     cue は seriko では担当外ゆえ発行に影響しない、
/// (b) text 側 sink は `Text("アヒルやアヒル")` cue を無変形で受信する——間に割り込んだ bind の
///     `Custom` cue が text ストリームを差し替え・破壊していない（R2.3）。
///
/// テキスト cue は再生時間（6 文字×50ms=0.3s）を持つため、占有 horizon（0.3）を跨ぐ Tick(1.0) まで
/// 進めて自然終端させる（同期チェーンは他シナリオと同一・sleep 不使用・R5.3）。
#[test]
fn bind_and_text_streams_do_not_cross_contaminate_end_to_end() {
    let text_sink = RecordingTextSink::new();
    let text_records = text_sink.records();

    // 初回 Tick(0.0) で at=0 群（ClearAll/Emote/bind/Text）を発火、Tick(1.0) で text 占有 horizon
    // （0.3）を跨いで自然終端させる。
    let seriko_records = run_with_text_sink(
        r"\s[1000]\![bind,腕,伸び,0]アヒルやアヒル\e",
        &[0.0, 1.0],
        Box::new(text_sink),
    );

    // (a) seriko 側: bind 再発行が現れ、テキストは表示発行に影響しない。
    assert_eq!(
        seriko_records,
        vec![show(1000, [1100, 1207]), show(1000, [1207])],
        "bind 混在でも seriko の表示発行は bind の再発行のみ（テキストは担当外・R2.3）"
    );

    // (b) text 側: broadcast で受けた cue 列から Text の中身だけを抜き、素通しを確認する。
    let texts: Vec<String> = text_records
        .lock()
        .expect("text records mutex poisoned")
        .iter()
        .filter_map(|cue| match &cue.command {
            CueCommand::Text(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        vec!["アヒルやアヒル".to_string()],
        "text ストリームは bind の割り込みで汚染されず Text を無変形に受信（R2.3）"
    );
}

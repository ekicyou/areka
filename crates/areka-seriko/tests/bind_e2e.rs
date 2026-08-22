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

use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::{
    ActorKey, ChoiceWaiting, CueCommand, CueSink, SakuraMsg, StartTalk, SystemVarSnapshot, TalkCue,
    TalkDone, TalkEndReason, TalkId, spawn_talk,
};
use areka_seriko::{
    BindOptionDecls, BindResolver, DisplayCommand, MockSurfaceOutput, SerikoLoopConfig,
    SurfaceResolver, spawn_seriko,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// `spawn_talk` の done ポート境界（`D: From<TalkDone> + From<ChoiceWaiting>`）を満たす
/// test-local な合流 enum。両通知は同一ポートを流れる（DD-6）が、本 E2E の fixture に選択肢
/// （`\q`）は無く `ChoiceWaiting` は発生しない——境界充足のためだけの受け皿である。
enum TalkNotice {
    Done(TalkDone),
    /// 本 e2e は選択待ち通知を観測しないが、`spawn_talk` の `D` 境界
    /// （`From<TalkDone> + From<ChoiceWaiting>`）を満たすため受け口だけ用意する。
    #[allow(dead_code)]
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
    // 宣言ゼロ＝全カテゴリが正典の既定（Default＝高々 1 個・解除可・bindopt 2.1/2.2）。本 fixture は
    // 腕・頬とも 1 カテゴリ 1 パーツゆえ、排他置換と加算は結果集合が同値になり期待値は不変である
    // （bindopt 設計 §テスト影響監査）。mustselect／複数可の専用 e2e は下記シナリオ 6〜9。
    BindResolver::new(sakura, BTreeMap::new(), BindOptionDecls::default())
}

/// 既定 scope "0"・指定 bind 集合を載せたシェル面 `Show` 指令を組む（期待値ヘルパ）。
fn show(surface_id: u32, ids: impl IntoIterator<Item = u32>) -> DisplayCommand {
    DisplayCommand::Show {
        scope: ActorKey::from("0"),
        surface_id,
        binds: BindSet::from_ids(ids),
        pattern: PatternState::default(),
    }
}

/// 1 シナリオを貫通駆動する中核（resolver／静的既定集合を注入可能にした汎用形）。
///
/// `script` を直入力する talk を起動し、`ticks` を順に注入して駆動する。seriko には呼び手が
/// 与えた静的既定集合 `static_binds` と bind 名前解決層 `resolver` を注入する（R5.5）。broadcast の
/// 2 つ目のスロットには呼び手が与えた `text_sink` を差し込む（bind のみ観測なら [`NullTextSink`]、
/// text 非汚染観測なら [`RecordingTextSink`]）。同期はモジュール doc の
/// 「Tick 注入 → done_rx.recv → SerikoSink drop による seriko disconnect → seriko join → records 照合」
/// のみで行う（sleep/polling なし・R5.3）。シナリオごとに新しい状態を立て、持ち越さない。
fn run_core(
    static_binds: BindSet,
    resolver: BindResolver,
    script: &str,
    ticks: &[f64],
    text_sink: Box<dyn CueSink + Send>,
) -> Vec<DisplayCommand> {
    // ── seriko アクター（表示指令の消費側）──
    // 空 alias 表（数値 key は数値枝で解決）・呼び手指定の静的既定 bind 集合・呼び手指定の bind
    // 名前解決層。records() ハンドルは move 前に取得する。
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (seriko_sink, seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        static_binds,
        resolver,
        SerikoLoopConfig::disabled(),
        out,
    );

    // ── talk アクター（fixture script 直入力）──
    // seriko_sink（＝seriko inbox の唯一の Sender）を broadcast スロット第 1 へ move する。clone を
    // 残さないことで、talk スレッド終了時の drop が seriko inbox を disconnect させる（同期チェーンの要）。
    let (done_tx, done_rx) = std::sync::mpsc::channel::<TalkNotice>();
    let talk = spawn_talk(
        StartTalk {
            epilogue: Vec::new(),
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
    let TalkNotice::Done(done) = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("fixture script は `\\e` で自然終端し TalkDone を返すべき")
    else {
        panic!("選択肢を含まない fixture では ChoiceWaiting は流れない（TalkDone のみ）");
    };
    assert_eq!(
        done.reason,
        TalkEndReason::Ended,
        "`\\e` は Ended で終端する"
    );

    // (2) talk スレッド終了→move 済み SerikoSink（唯一の Sender）drop→seriko inbox disconnect。
    talk.actor
        .join()
        .expect("talk body は Break 後に正常終了する");
    // (3) seriko join（唯一の同期点）。先送りの Cue は FIFO 単一スレッドゆえ join 復帰時に処理済み。
    seriko.join().expect("seriko は disconnect で正常終了する");

    // (4) 発行列のスナップショットを返す（決定論）。
    let recorded = records.lock().expect("records mutex poisoned");
    recorded.clone()
}

/// 既存シナリオ（task 6.4 系）の既定注入で貫通駆動する薄いラッパ。
///
/// 非空の静的既定集合 `{1100,1207}` と test-local [`test_bind_resolver`]（bindoption 宣言ゼロ＝
/// 全カテゴリ既定・1 カテゴリ 1 パーツ）を注入する。broadcast の 2 つ目のスロットは呼び手指定の
/// `text_sink`。
fn run_with_text_sink(
    script: &str,
    ticks: &[f64],
    text_sink: Box<dyn CueSink + Send>,
) -> Vec<DisplayCommand> {
    run_core(
        BindSet::from_ids([1100, 1207]),
        test_bind_resolver(),
        script,
        ticks,
        text_sink,
    )
}

/// bind のみを観測する（text スロットは破棄）薄いラッパ。
fn run_scenario(script: &str, ticks: &[f64]) -> Vec<DisplayCommand> {
    run_with_text_sink(script, ticks, Box::new(NullTextSink))
}

/// mustselect（排他置換・ちょうど 1 個）カテゴリを持つ test-local 名前解決表（bindopt 3.1/2.1/2.2）。
///
/// 実 emo2 の目カテゴリ（笑顔→ジトー→静観 …）を模す。本体側（sakura）に mustselect カテゴリ
/// `目`（3 パーツ：笑顔→1303／ジトー→1301／静観→1304）と、対比用の既定（Default）カテゴリ
/// `紅`（差し→1600）を持つ。`sakura_mustselect={目}`（紅は非宣言＝既定＝高々 1 個・解除可）。
/// 相方側（kero）は空。これにより、同一 mustselect カテゴリへの連続着衣が高々 1 パーツへ
/// 「置換」される一方、既定カテゴリのパーツが明示 on で載り明示 off で外れる（解除可）ことを
/// 対比観測できる。紅は 1 カテゴリ 1 パーツゆえ、既定の排他置換でも結果集合は加算と同値である。
fn mustselect_bind_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("目".into(), "笑顔".into()), 1303);
    sakura.insert(("目".into(), "ジトー".into()), 1301);
    sakura.insert(("目".into(), "静観".into()), 1304);
    sakura.insert(("紅".into(), "差し".into()), 1600);
    let mut sakura_ms: BTreeSet<String> = BTreeSet::new();
    sakura_ms.insert("目".into());
    let options = BindOptionDecls {
        sakura_mustselect: sakura_ms,
        ..Default::default()
    };
    BindResolver::new(sakura, BTreeMap::new(), options)
}

/// mustselect 対比観測用の貫通駆動ラッパ。
///
/// 静的既定集合は非 目 の口 id 1 本のみ `{1207}`（初回の 目 着衣がクリーンな add になり、かつ
/// 排他置換で外れない「保持されるべき地」を観測できる）。bind 名前解決層は
/// [`mustselect_bind_resolver`]。text スロットは破棄（bind の表示発行のみ観測）。
fn run_mustselect_scenario(script: &str, ticks: &[f64]) -> Vec<DisplayCommand> {
    run_core(
        BindSet::from_ids([1207]),
        mustselect_bind_resolver(),
        script,
        ticks,
        Box::new(NullTextSink),
    )
}

/// 複数可（Multiple）宣言カテゴリを持つ test-local 名前解決表（bindopt 3.3・貫通観測用）。
///
/// 本体側（sakura）に `multiple` 宣言カテゴリ `髪飾り`（2 パーツ：花→1700／リボン→1701）を持つ。
/// `sakura_multiple={髪飾り}`。相方側（kero）は空。カテゴリ名は名前表と `BindOptionDecls` で
/// **厳密に一致**していなければならない（不一致だと policy が既定へ落ち、複数可の観測にならない）。
fn multiple_bind_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("髪飾り".into(), "花".into()), 1700);
    sakura.insert(("髪飾り".into(), "リボン".into()), 1701);
    let mut sakura_mul: BTreeSet<String> = BTreeSet::new();
    sakura_mul.insert("髪飾り".into());
    let options = BindOptionDecls {
        sakura_multiple: sakura_mul,
        ..Default::default()
    };
    BindResolver::new(sakura, BTreeMap::new(), options)
}

/// 複数可カテゴリ観測用の貫通駆動ラッパ（bindopt 3.3）。
///
/// 静的既定集合は髪飾り以外の口 id 1 本のみ `{1207}`（着衣が clean add になり、かつ共存の
/// 「保持されるべき地」も同時に観測できる）。text スロットは破棄（bind の表示発行のみ観測）。
fn run_multiple_scenario(script: &str, ticks: &[f64]) -> Vec<DisplayCommand> {
    run_core(
        BindSet::from_ids([1207]),
        multiple_bind_resolver(),
        script,
        ticks,
        Box::new(NullTextSink),
    )
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

/// シナリオ6（bindopt 3.1・KEY＝排他置換 e2e）: 同一 mustselect カテゴリへ off なしで連続着衣する
/// 列が、常に高々 1 パーツのみ有効な着せ替え集合を載せた表示発行になる（旧パーツが消え新パーツのみ・
/// 「置換」）ことを、注入 Tick(0.0) だけで貫通観測する。
///
/// `\s[1000]\![bind,目,笑顔,1]\![bind,目,ジトー,1]\![bind,目,静観,1]\e`（実 emo2 の目＝笑顔→ジトー→
/// 静観 の遷移を模す）を駆動すると:
/// - `\s[1000]` が静的既定 `{1207}` を載せた Show を発行、
/// - 目=笑顔（1303）着衣 → クリーンな add で `{1207,1303}`、
/// - 目=ジトー（1301）着衣 → mustselect 排他置換で旧 1303 が外れ `{1207,1301}`（口 1207 は保持）、
/// - 目=静観（1304）着衣 → 同じく 1301 が外れ `{1207,1304}`。
///
/// 最終集合は 目 パーツを**ちょうど 1 個**（1304）だけ含み、旧 目 パーツ（1303・1301）は残らない
/// ——これが反・積算（anti-accumulation）の証明。pre-fix の積算バグなら最終が `{1207,1301,1303,1304}`
/// になるため、full-Vec 等値照合がこの判別を捕捉する（bindopt 3.1）。
#[test]
fn mustselect_sequence_replaces_prior_part_end_to_end() {
    let records = run_mustselect_scenario(
        r"\s[1000]\![bind,目,笑顔,1]\![bind,目,ジトー,1]\![bind,目,静観,1]\e",
        &[0.0],
    );
    assert_eq!(
        records,
        vec![
            show(1000, [1207]),       // \s[1000] 既定
            show(1000, [1207, 1303]), // 目=笑顔 on → クリーン add
            show(1000, [1207, 1301]), // 目=ジトー on → 排他置換（1303 除去・1301 追加・1207 保持）
            show(1000, [1207, 1304]), // 目=静観 on → 排他置換（1301 除去・1304 追加）
        ],
        "mustselect 列は高々 1 パーツへ置換され、旧 目 パーツは残らない（反・積算・bindopt 3.1）"
    );

    // 判別の明示（anti-accumulation）: 最終集合の 目 パーツ（1301/1303/1304 のいずれか）はちょうど 1 個。
    let final_binds = match records.last().expect("最終 Show が存在する") {
        DisplayCommand::Show { binds, .. } => binds.clone(),
        other => panic!("最終指令は Show であるべき: {other:?}"),
    };
    let eye_parts: Vec<u32> = [1301u32, 1303, 1304]
        .into_iter()
        .filter(|id| final_binds.contains(*id))
        .collect();
    assert_eq!(
        eye_parts,
        vec![1304],
        "最終集合の 目 パーツはちょうど 1 個（1304）＝旧 1303/1301 は消えた（積算していない・bindopt 3.1）"
    );
}

/// シナリオ7（bindopt 2.1/2.2・mustselect 対比）: 既定（Default＝非宣言）カテゴリ（紅）のパーツは
/// 明示 on で載り、明示 off で外れる（解除可）——mustselect の「脱衣は無視（解除不可）」との対比。
///
/// `\s[1000]\![bind,紅,差し,1]\![bind,紅,差し,0]\e` を Tick(0.0) で駆動すると:
/// - `\s[1000]` が既定 `{1207}` を発行、
/// - 紅=差し（1600）on → 当該カテゴリに他パーツが無いので排他置換の結果は `{1207,1600}`、
/// - 紅=差し（1600）off → 既定は解除可ゆえ除去され `{1207}` へ戻る。
///
/// 紅は 1 カテゴリ 1 パーツゆえ、既定の排他置換でも結果集合は加算と同値であり期待値は
/// bindopt 是正の前後で不変である（bindopt 設計 §テスト影響監査）。off が集合を変えることが
/// mustselect（bindopt 3.2＝脱衣無視）との唯一の差である。
#[test]
fn default_category_explicit_on_then_off_removes_part_end_to_end() {
    let records =
        run_mustselect_scenario(r"\s[1000]\![bind,紅,差し,1]\![bind,紅,差し,0]\e", &[0.0]);
    assert_eq!(
        records,
        vec![
            show(1000, [1207]),       // \s[1000] 既定
            show(1000, [1207, 1600]), // 紅=差し on → 既定カテゴリの唯一パーツが載る
            show(1000, [1207]),       // 紅=差し off → 既定は解除可ゆえ除去され既定へ
        ],
        "既定カテゴリは明示 on で載り明示 off で外れる（解除可・bindopt 2.1/2.2）"
    );
}

/// シナリオ8（bindopt 3.4・クロスカテゴリ独立）: mustselect カテゴリ内の排他置換は、別カテゴリ
/// （既定＝非宣言の紅）で着衣済みのパーツを巻き込まない（カテゴリ境界を越えて外さない）。
///
/// `\s[1000]\![bind,紅,差し,1]\![bind,目,笑顔,1]\![bind,目,静観,1]\e` を Tick(0.0) で駆動すると:
/// - `\s[1000]` が既定 `{1207}` を発行、
/// - 紅=差し（1600）on → `{1207,1600}`、
/// - 目=笑顔（1303）on → クリーン add で `{1207,1600,1303}`、
/// - 目=静観（1304）on → 目カテゴリ内でのみ排他置換（1303 除去・1304 追加）→ `{1207,1600,1304}`。
///
/// 紅（1600）は 目 の排他置換を跨いで保持され、最終集合は 目 パーツちょうど 1 個（1304）＋紅（1600）＋
/// 口（1207）。排他は自カテゴリ限定＝異なるカテゴリ間の bind は共存することを実証（bindopt 3.4）。
#[test]
fn mustselect_replace_does_not_drop_other_category_end_to_end() {
    let records = run_mustselect_scenario(
        r"\s[1000]\![bind,紅,差し,1]\![bind,目,笑顔,1]\![bind,目,静観,1]\e",
        &[0.0],
    );
    assert_eq!(
        records,
        vec![
            show(1000, [1207]),             // \s[1000] 既定
            show(1000, [1207, 1600]),       // 紅 on → 加算
            show(1000, [1207, 1303, 1600]), // 目=笑顔 on → クリーン add
            show(1000, [1207, 1304, 1600]), // 目=静観 on → 目内のみ置換（紅 1600 は保持）
        ],
        "目の排他置換は紅（別カテゴリ・既定）を巻き込まない（クロスカテゴリ独立・bindopt 3.4）"
    );
}

/// シナリオ9（bindopt 3.3・KEY＝複数可の貫通共存錨）: `multiple` 明示宣言のカテゴリでは、同一
/// カテゴリの 2 パーツが**両方 bind されたまま**表示まで貫通する（既定の排他置換を受けない）。
///
/// `\s[1000]\![bind,髪飾り,花,1]\![bind,髪飾り,リボン,1]\e` を Tick(0.0) で駆動すると:
/// - `\s[1000]` が静的既定 `{1207}` を載せた Show を発行、
/// - 髪飾り=花（1700）着衣 → clean add で `{1207,1700}`、
/// - 髪飾り=リボン（1701）着衣 → **複数可ゆえ加算**で `{1207,1700,1701}`（1700 は外れない）。
///
/// 既定（Default）カテゴリなら 2 度目の着衣で 1700 が外れ `{1207,1701}` になる（シナリオ 6 の形）
/// ——本錨はその分岐が複数可宣言で反転することを貫通経路で固定する。
#[test]
fn multiple_category_two_parts_coexist_end_to_end() {
    let records = run_multiple_scenario(
        r"\s[1000]\![bind,髪飾り,花,1]\![bind,髪飾り,リボン,1]\e",
        &[0.0],
    );
    assert_eq!(
        records,
        vec![
            show(1000, [1207]),             // \s[1000] 既定
            show(1000, [1207, 1700]),       // 髪飾り=花 on → clean add
            show(1000, [1207, 1700, 1701]), // 髪飾り=リボン on → 複数可ゆえ加算（1700 は保持）
        ],
        "複数可カテゴリは同一カテゴリ 2 パーツが共存したまま貫通する（bindopt 3.3）"
    );

    // 判別の明示（共存）: 最終集合は髪飾りパーツを 2 個とも含む（排他置換なら 1 個になる）。
    let final_binds = match records.last().expect("最終 Show が存在する") {
        DisplayCommand::Show { binds, .. } => binds.clone(),
        other => panic!("最終指令は Show であるべき: {other:?}"),
    };
    let hair_parts: Vec<u32> = [1700u32, 1701]
        .into_iter()
        .filter(|id| final_binds.contains(*id))
        .collect();
    assert_eq!(
        hair_parts,
        vec![1700, 1701],
        "最終集合の髪飾りパーツは 2 個とも残る＝複数可は排他置換を受けない（bindopt 3.3）"
    );
}

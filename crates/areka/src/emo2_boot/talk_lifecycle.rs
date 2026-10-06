//! 会話の**表示終了時刻**（待機を含む占有区間の終端）を UI スレッドへ届ける観測
//! （design.md「BalloonLifecycleSink（`emo2_boot/talk_lifecycle.rs`）」・決定 D4＝α・
//! Requirements 4.1 / 4.5 / 4.8 / 7.2 / 7.8）。
//!
//! # 何を観測するか
//!
//! バルーンのタイムアウト計測は「スクリプトの表示が終わってから」を起点とする（Requirement 4.1）。
//! ここでいう表示終了は**待機（`\w` 等）を含む台本の占有区間の終端**であって、最後の文字が
//! 現れた時刻ではない。dola の占有 horizon は `max(cue.at + cue.duration)` で定義されるため、
//! [`BalloonLifecycleSink`] は broadcast される全 cue を観測して同じ式で最大値を集約し、
//! その**既知最大が更新されたときだけ** UI へ送る。
//!
//! # なぜ 4 本目の sink なのか（D4＝α の裁定）
//!
//! `TalkDone` を配線する案（β）は `spawn_dispatcher` の 9 呼出箇所へ波及し、「会話進行の管理層の
//! 既存の通知先は変えない」という制約に反する。α は `GhostBootOptions.sinks` へ 1 本足すだけで
//! 上流（ghost / kanade / dola）の署名を一切変えない。
//!
//! α の既知の弱点は要件の別条項が吸収する——sink は `Barrier` / `Routing` を受け取らないため
//! 選択肢バリア中の horizon は過小になりうるが、Requirement 5.4 の抑止が非表示を防ぎ、バリア
//! 解除後の cue が horizon を引き上げて計測が正しく再開する。中断（Requirement 4.6）も起点は
//! 正常終了と同一値（占有 horizon）になる。中断の時刻を起点に採るため、受け口は落ちる瞬間の
//! talk 相対秒を [`TalkLifecycleSignal::TalkEnded`] で送る（areka-P0-balloon-lifecycle-events
//! 要件 5.2・判断の側の採り方は `balloon_visibility_wait.rs`）。
//!
//! # 会話境界の自己検出
//!
//! dispatcher は talk 起動ごとに登録 sink を 1 回だけ複製し、直後に
//! [`BootCueSink::begin_talk`] でそのトークの番号を渡す（`crates/areka-ghost/src/dispatcher.rs`）。
//! 本 sink はこの複製を会話境界そのものとして使い、複製後の初回 `emit` で番号つきの
//! [`TalkLifecycleSignal::TalkStarted`] を、落ちるときに [`TalkLifecycleSignal::TalkEnded`] を
//! 送る——上流へ「talk が始まった・終わった」を問い合わせる口を新設しない。
//!
//! # 待ち時間の指定
//!
//! `\![set,balloontimeout,時間]` の cue だけを拾い、時間の欄を [`parse_balloon_timeout`] で読んで
//! [`TalkLifecycleSignal::BalloonTimeout`] を送る（要件 9.1〜9.4・7.5）。他の `\![set,…]` は
//! 読み飛ばす。台本の順に配られた cue だけが届くので、指定に達する前に止まったトークでは
//! 効かない（要件 9.8）。

use std::sync::mpsc::Sender;

use areka_ghost::sink::BootCueSink;
use areka_sakura::TalkId;
use dola::cue::TalkCue;
use tracing::{info, warn};
use wintf::ecs::world::tick_wake;

use super::talk_clock::TalkClock;

/// UI スレッドの可視性コントローラへ流れる表示ライフサイクル信号（design「Event Contract」）。
///
/// 送り手は 2 つある——talk スレッドの受け口 [`BalloonLifecycleSink`]（会話の開始・占有終端・
/// 待ち時間の指定・会話の終わり）と、UI スレッドの押下ハンドラ（利用者の中断）。どちらも同じ
/// 1 本の線へ流す。1 つのトークの中の順は `TalkStarted` →（`DisplayEndAt`・`BalloonTimeout` が
/// 台本の順）→ `TalkEnded`。次のトークの `TalkStarted` は必ず前のトークの `TalkEnded` の後に届く
/// （talk スレッドは受け口を落としてから完了を知らせる）。
///
/// 時刻軸は **talk 相対秒**（`TalkClock::talk_time` と同一軸）。
///
/// 受信側は [`TalkStarted`](Self::TalkStarted) で計測をリセットし、以降の
/// [`DisplayEndAt`](Self::DisplayEndAt) を max 集約する（重複・順不同の値に対して単調）。
// 判断側の消費は着地済み（`balloon_visibility.rs` の観測スナップショットが本型を運び、計測の
// 破棄と占有終端の更新に使う）。送出側の構築点も着地済み——`wire_emo2_boot`（`emo2_boot/mod.rs`）が
// channel を作り、送出端を `GhostBootOptions.sinks` 4 本目の [`BalloonLifecycleSink`] へ、受信端を
// `Emo2Wiring::lifecycle_rx` へ渡す（task 4.1）。受信端から取り出す配線は task 4.4。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TalkLifecycleSignal {
    /// この talk の観測が始まった（複製後の初回 `emit` で 1 回だけ・全 `DisplayEndAt` に先行）。
    ///
    /// 受信側はこれを進行中のタイムアウト計測の破棄契機として使う（Requirement 4.5）。
    /// `talk_id` は配送から受け取った番号（受け取っていなければ `None`）。
    TalkStarted { talk_id: Option<TalkId> },
    /// 待機を含む占有区間の終端（talk 相対秒）。既知最大が更新されたときだけ届く
    /// （Requirement 4.1）。
    DisplayEndAt(f64),
    /// `\![set,balloontimeout,時間]` が配られた（このトークの待ち時間の指定）。
    BalloonTimeout(TalkTimeout),
    /// このトークの受け口が落ちた（最後まで・中断・置き換えのどれでも）。`at` は落ちた瞬間の
    /// talk 相対秒で、時刻源が起点を持たないときは `None`。合図を 1 つでも送った複製だけが送る。
    TalkEnded { at: Option<f64> },
    /// 利用者がバルーンを左ダブルクリックして再生を中断した（areka-P0-balloon-break 要件 4.1）。
    ///
    /// 送り手は UI スレッドの押下ハンドラで、この信号は `BalloonLifecycleSink` を経由しない。
    /// 受信側は現に出ているバルーンをすべて隠し、次の会話が始まるまで内容では出し直さない。
    UserBreak,
}

/// そのトークの時間切れまでの待ち時間（`\![set,balloontimeout,時間]` の読み）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TalkTimeout {
    /// 既定の待ち時間（30 秒か環境変数。決め方は判断の側が持つ）。
    Default,
    /// そのミリ秒。
    Millis(u64),
    /// 時間切れで隠さない。
    Never,
}

/// 時間の欄の読みの結果。`unreadable` は「整数として読めず既定にした」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParsedTimeout {
    pub(crate) timeout: TalkTimeout,
    pub(crate) unreadable: bool,
}

/// 時間の欄を読む（要件 9.1〜9.4）。省略・空は既定、正の整数はそのミリ秒、0 と負の整数は
/// 時間切れなし、整数として読めない値（文字・小数・桁あふれ）は既定で `unreadable`。
/// 前後の空白は落として読む。
pub(crate) fn parse_balloon_timeout(value: Option<&str>) -> ParsedTimeout {
    let value = value.map(str::trim).unwrap_or_default();
    let (timeout, unreadable) = if value.is_empty() {
        (TalkTimeout::Default, false)
    } else {
        match value.parse::<i64>() {
            Ok(ms) if ms > 0 => (TalkTimeout::Millis(ms.unsigned_abs()), false),
            Ok(_) => (TalkTimeout::Never, false),
            Err(_) => (TalkTimeout::Default, true),
        }
    };
    ParsedTimeout {
        timeout,
        unreadable,
    }
}

/// 会話の表示終了時刻を UI へ届ける broadcast sink（`GhostBootOptions.sinks` の 4 本目・D4＝α）。
///
/// broadcast 下では担当外の cue も全て届くが、占有終端の集約は**コマンド名で選別しない**——占有区間の
/// 終端は台本上の全 cue（`Text` / `Wait` / `ClearAll` / キャリア……）の `at + duration` の最大値
/// として定義されるため、種別に依らず全件を集約対象にするのが正しい（名前で拾うのは
/// `\![set,balloontimeout,…]` の待ち時間の指定だけ）。cue を**観測するのみ**で
/// `duration` にも会話進行にも触れないため、duration honor 契約へ影響しない。
///
/// # 送出契約（design「Event Contract」）
///
/// - Ordering: 単一 `emit` 内で [`TalkLifecycleSignal::TalkStarted`] を先に送るため、FIFO の
///   `mpsc` 上でこの talk の全 `DisplayEndAt` に必ず先行する。
/// - Idempotency: `DisplayEndAt` は既知最大を**狭義に超えた**ときだけ送るので、受信列は狭義単調
///   増加になる（同値・後退値は送らない）。
/// - Delivery: 非ブロックの `std::sync::mpsc`。attach 前・受信前はチャネル自身が保留バッファを
///   兼ねる。送信失敗（受信端切断）は `warn!` ＋ talk 非破壊——`MoveCueSink` と同一規律で、
///   無音破棄でも panic でもない（log-first）。
pub(crate) struct BalloonLifecycleSink {
    /// UI スレッド（frame 相 drain）への送出端。全 clone は単一受信端へ配送する。
    tx: Sender<TalkLifecycleSignal>,
    /// 落ちた瞬間の talk 相対秒を読む時刻源（文字の受け口と共有・起点はそちらが決める）。
    clock: TalkClock,
    /// この複製が受け持つトークの番号（配送が `begin_talk` で渡す・複製ごとに `None` から）。
    talk_id: Option<TalkId>,
    /// この talk で [`TalkLifecycleSignal::TalkStarted`] を送出済みか（複製ごとに `false` から）。
    /// 立っていれば合図を 1 つ以上送っている＝落ちるときに `TalkEnded` を送る。
    started: bool,
    /// 既知最大の占有終端。未観測は `NEG_INFINITY` ゆえ、最初の有限値は必ず送出される。
    horizon: f64,
}

impl BalloonLifecycleSink {
    /// mpsc 送信端（コントローラ側の `Receiver` と対・`wire_emo2_boot` が生成）と時刻源から
    /// sink を構築する。
    pub(crate) fn new(tx: Sender<TalkLifecycleSignal>, clock: TalkClock) -> Self {
        Self {
            tx,
            clock,
            talk_id: None,
            started: false,
            horizon: f64::NEG_INFINITY,
        }
    }

    /// `\![set,balloontimeout,時間]` なら読んで待ち時間の指定を送る。他の cue は何もしない。
    fn pick_balloon_timeout(&self, cue: &TalkCue) {
        let Some(("set", params)) = cue.command.as_command_carrier() else {
            return;
        };
        if params.first().copied() != Some("balloontimeout") {
            return;
        }
        let raw = params.get(1).copied();
        let parsed = parse_balloon_timeout(raw);
        if parsed.unreadable {
            warn!(
                event = "balloon_timeout_set",
                timeout = ?parsed.timeout,
                unreadable = raw.unwrap_or_default(),
                "BalloonLifecycleSink: 待ち時間の指定が整数として読めないので既定にする"
            );
        } else {
            info!(
                event = "balloon_timeout_set",
                timeout = ?parsed.timeout,
                "BalloonLifecycleSink: 待ち時間の指定を読んだ"
            );
        }
        self.send(TalkLifecycleSignal::BalloonTimeout(parsed.timeout));
    }

    /// 1 信号を非ブロックで送る。受信端切断は `warn!` ＋継続——talk を殺さない
    /// （log-first・`MoveCueSink::emit` と同一規律）。
    fn send(&self, signal: TalkLifecycleSignal) {
        if self.tx.send(signal).is_err() {
            warn!(
                ?signal,
                "BalloonLifecycleSink: 表示ライフサイクル信号の送出に失敗（受信端切断）——talk は継続する"
            );
        }
        // 表示ライフサイクルの信号が UI へ届いた＝次の画面更新に仕事がある（設計 C16 の
        // `PRESENT`）。送出の後に立てる理由は `PresentBridge::send` と同じ。
        tick_wake::mark(tick_wake::PRESENT);
    }
}

/// **状態を引き継がない複製**——`Clone` を持たず、`BootCueSink` を自分で実装してある
/// （一括の実装に乗らないので、配送から番号を受け取る `begin_talk` を上書きできる）。
///
/// dispatcher は talk 起動ごとに登録 sink を 1 回だけ複製し、その複製が当該 talk 専用
/// インスタンスになる。複製で `talk_id` / `started` / `horizon` を初期状態へ戻すことで、
/// 「複製＝会話境界」を型の側で構造的に保証する——「登録 sink 自身は決して `emit` されない」と
/// いう上流の暗黙不変条件に依存しない。前の talk の horizon を持ち越すと、次の会話の表示終了
/// 時刻が**過大**に見えて非表示が遅れる（あるいは永久に来ない）ため、複製時のリセットは
/// 保持側ではなく誤動作側へ倒れる欠陥を構造遮断する意味を持つ。
///
/// talk 進行中に再複製されると `TalkStarted` が再送されるが、dispatcher の複製は talk 起動時の
/// 1 回のみで、複製後の `Box<dyn CueSink + Send>` は複製の口を持たないため、その経路は存在しない。
impl BootCueSink for BalloonLifecycleSink {
    fn clone_box(&self) -> Box<dyn BootCueSink> {
        Box::new(Self::new(self.tx.clone(), self.clock.clone()))
    }

    fn begin_talk(&mut self, talk_id: TalkId) {
        self.talk_id = Some(talk_id);
    }
}

/// 落ちる＝このトークが終わった（最後まで・中断・置き換えのどれでも）。合図を 1 つでも送った
/// 複製だけが、落ちた瞬間の talk 相対秒つきで `TalkEnded` を 1 回送る（登録の原本と cue の
/// 来なかった複製は何も送らない・`NoUserBreakCueSink` の `Drop` と同じ型）。時刻はフレームに
/// 丸めない（要件 5.2）。時刻源に起点が無ければ時刻無しで送り、記録を残す（要件 5.6）。
impl Drop for BalloonLifecycleSink {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        let at = self.clock.now_talk_time();
        if at.is_none() {
            warn!(
                talk_id = ?self.talk_id,
                "BalloonLifecycleSink: 止まった時刻が取れない（時刻源に起点が無い）——時刻無しで終わりを送る"
            );
        }
        self.send(TalkLifecycleSignal::TalkEnded { at });
    }
}

impl dola::cue::CueSink for BalloonLifecycleSink {
    fn emit(&mut self, cue: TalkCue) {
        // ⑴ 会話開始（Ordering 契約）: 複製後の初回 emit で 1 回だけ。送出の成否に依らず
        //    `started` を立てる——「talk につき 1 回」は受信側の都合ではなく本 sink の契約であり、
        //    切断時に毎 cue 再送するのは無意味なノイズになる（失敗自体は send 内で warn 済み）。
        if !self.started {
            self.started = true;
            self.send(TalkLifecycleSignal::TalkStarted {
                talk_id: self.talk_id,
            });
        }

        // 待ち時間の指定（`\![set,balloontimeout,…]` だけ・台本の順に届く）。
        self.pick_balloon_timeout(&cue);

        // ⑵ 占有終端 = at + duration（dola の `to_talk_schedule` と同一式）。待機 cue も
        //    duration を運ぶ第一級 cue ゆえ、この式のまま horizon へ算入される（Requirement 4.1）。
        let end = cue.at + cue.duration;

        // 非有限（NaN / ±inf）は horizon に採らない。NaN は `>` 比較で自然に落ちるが、`+inf` は
        // 「既知最大を超える」判定を通ってしまい、受信側の `deadline = display_end + timeout` を
        // 到達不能にして**タイムアウトが永久に成立しない**状態を作る。信号が届かない側の縮退
        // （Requirement 4.8＝表示を保持）と同じ向きだが、無音で起こすと原因が追えないため
        // 記録付きスキップにする（log-first）。
        if !end.is_finite() {
            warn!(
                at = cue.at,
                duration = cue.duration,
                "BalloonLifecycleSink: 非有限な占有終端を記録付きスキップ（horizon を汚さない）"
            );
            return;
        }

        // ⑶ 既知最大を狭義に超えたときだけ送る（Idempotency 契約＝受信列は狭義単調増加）。
        if end > self.horizon {
            self.horizon = end;
            self.send(TalkLifecycleSignal::DisplayEndAt(end));
        }
    }
}

#[cfg(test)]
#[path = "talk_lifecycle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "talk_lifecycle_signals_tests.rs"]
mod signals_tests;

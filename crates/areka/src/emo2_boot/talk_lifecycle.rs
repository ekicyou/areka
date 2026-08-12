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
//! 正常終了と同一値（占有 horizon）になり、誤差は**表示を保持する側**にのみ倒れる
//! （Requirement 4.8 と同じ側）。中断時刻を起点に採る精密化は追跡 spec
//! `areka-P0-balloon-canon-residue` が [`BalloonLifecycleNotice`] の実発火と一体で所有する。
//!
//! # 会話境界の自己検出
//!
//! dispatcher は talk 起動ごとに登録 sink を 1 回だけ複製する
//! （`crates/areka-ghost/src/dispatcher.rs:290-294` の `clone_box`）。本 sink はこの複製を
//! 会話境界そのものとして使い、複製後の初回 `emit` で [`TalkLifecycleSignal::TalkStarted`] を
//! 送る——上流へ「talk が始まった」を問い合わせる口を新設しない。

use std::sync::mpsc::Sender;

use dola::cue::TalkCue;
use tracing::warn;

/// talk スレッドから UI スレッドへ流れる表示ライフサイクル信号（design「Event Contract」）。
///
/// 時刻軸は **talk 相対秒**（`TalkClock::talk_time` と同一軸）。
///
/// 受信側は [`TalkStarted`](Self::TalkStarted) で計測をリセットし、以降の
/// [`DisplayEndAt`](Self::DisplayEndAt) を max 集約する（重複・順不同の値に対して単調）。
// 判断側の消費は着地済み（`balloon_visibility.rs` の観測スナップショットが本型を運び、計測の
// 破棄と占有終端の更新に使う）。送出側の構築点（`GhostBootOptions.sinks` への登録）は task 4.1。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TalkLifecycleSignal {
    /// この talk の観測が始まった（複製後の初回 `emit` で 1 回だけ・全 `DisplayEndAt` に先行）。
    ///
    /// 受信側はこれを進行中のタイムアウト計測の破棄契機として使う（Requirement 4.5）。
    TalkStarted,
    /// 待機を含む占有区間の終端（talk 相対秒）。既知最大が更新されたときだけ届く
    /// （Requirement 4.1）。
    DisplayEndAt(f64),
}

/// 会話の表示終了時刻を UI へ届ける broadcast sink（`GhostBootOptions.sinks` の 4 本目・D4＝α）。
///
/// broadcast 下では担当外の cue も全て届くが、本 sink は**コマンド名で選別しない**——占有区間の
/// 終端は台本上の全 cue（`Text` / `Wait` / `ClearAll` / キャリア……）の `at + duration` の最大値
/// として定義されるため、種別に依らず全件を集約対象にするのが正しい。cue を**観測するのみ**で
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
    /// この talk で [`TalkLifecycleSignal::TalkStarted`] を送出済みか（複製ごとに `false` から）。
    started: bool,
    /// 既知最大の占有終端。未観測は `NEG_INFINITY` ゆえ、最初の有限値は必ず送出される。
    horizon: f64,
}

#[allow(dead_code)] // 実構築点は task 4.1 の配線で着地する（段階実装の想定内）
impl BalloonLifecycleSink {
    /// mpsc 送信端（コントローラ側の `Receiver` と対・task 4.1 が生成）から sink を構築する。
    pub(crate) fn new(tx: Sender<TalkLifecycleSignal>) -> Self {
        Self {
            tx,
            started: false,
            horizon: f64::NEG_INFINITY,
        }
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
    }
}

/// **状態を引き継がない複製**——`#[derive(Clone)]` ではなく手書きにしてある。
///
/// dispatcher は talk 起動ごとに登録 sink を 1 回だけ複製し（`dispatcher.rs:290-294`）、その
/// 複製が当該 talk 専用インスタンスになる。複製で `started` / `horizon` を初期状態へ戻すことで、
/// 「複製＝会話境界」を型の側で構造的に保証する——「登録 sink 自身は決して `emit` されない」と
/// いう上流の暗黙不変条件に依存しない。前の talk の horizon を持ち越すと、次の会話の表示終了
/// 時刻が**過大**に見えて非表示が遅れる（あるいは永久に来ない）ため、複製時のリセットは
/// 保持側ではなく誤動作側へ倒れる欠陥を構造遮断する意味を持つ。
///
/// talk 進行中に再複製されると `TalkStarted` が再送されるが、dispatcher の複製は talk 起動時の
/// 1 回のみで、複製後の `Box<dyn CueSink + Send>` は `Clone` ですらないため、その経路は存在しない。
impl Clone for BalloonLifecycleSink {
    fn clone(&self) -> Self {
        Self::new(self.tx.clone())
    }
}

impl dola::cue::CueSink for BalloonLifecycleSink {
    fn emit(&mut self, cue: TalkCue) {
        // ⑴ 会話開始（Ordering 契約）: 複製後の初回 emit で 1 回だけ。送出の成否に依らず
        //    `started` を立てる——「talk につき 1 回」は受信側の都合ではなく本 sink の契約であり、
        //    切断時に毎 cue 再送するのは無意味なノイズになる（失敗自体は send 内で warn 済み）。
        if !self.started {
            self.started = true;
            self.send(TalkLifecycleSignal::TalkStarted);
        }

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

/// **Requirement 7.2 の予約型**——正典の `OnBalloonClose` / `OnBalloonTimeout` / `OnBalloonBreak`
/// を発火するために、表示側から会話進行側（kanade）へ渡す必要のある情報を型として押さえたもの。
///
/// # 消費者ゼロである事実と、それでも実在する理由（Requirement 7.8）
///
/// 本 enum を構築する側も消費する側も**現時点で存在しない**。M1 はこれらの SHIORI イベントを
/// 発火しない（Requirement 7.2）ため、実発火の配線——UI→kanade の口（`MouseWiring` /
/// `ChoiceForwarder` と同型）——を敷くのは追跡 spec `areka-P0-balloon-canon-residue` の所有範囲
/// である。ここに型だけを残すのは、語彙と Reference 割当（下記コメント）を実コードの側にも
/// 固定し、「語彙だけが増えて配線の追跡が失われる」既知の失敗を繰り返さないため。
///
/// 棚卸で検出できる形にしてある——本注記と `#[allow(dead_code)]` の対が、消費者ゼロの予約口で
/// あることの目印になる。実発火が着地したら `allow` ごと外れる。
///
/// # Reference 割当（互換対応表と同値・doc/COMPAT_ARCHITECTURE.md）
///
/// | variant | 正典イベント | Reference |
/// |---|---|---|
/// | [`Closed`](Self::Closed) | `OnBalloonClose` | Ref0＝閉じる際に表示されていたスクリプト |
/// | [`TimedOut`](Self::TimedOut) | `OnBalloonTimeout` | Ref0＝スクリプト／Ref1＝残り時間 |
/// | [`Broken`](Self::Broken) | `OnBalloonBreak` | Ref0＝スクリプト／Ref1＝scope 番号／Ref2＝中断位置 |
///
/// なお `OnBalloonClick` は正典に存在せず、クリックによる閉鎖は `OnBalloonClose` へ集約される
/// （Requirement 7.3）——独自のクリック閉鎖 variant をここへ足してはならない。
#[allow(dead_code)] // 消費者ゼロ（意図的予約・Requirement 7.8）: 実発火は areka-P0-balloon-canon-residue が所有
pub(crate) enum BalloonLifecycleNotice {
    /// `OnBalloonClose`。`script`＝閉じる際に表示されていたスクリプト（Ref0）。
    Closed { script: String },
    /// `OnBalloonTimeout`。`script`＝スクリプト（Ref0）／`remaining_ms`＝残り時間（Ref1）。
    TimedOut { script: String, remaining_ms: u64 },
    /// `OnBalloonBreak`。`script`＝スクリプト（Ref0）／`scope`＝scope 番号（Ref1）／
    /// `break_position`＝中断位置（Ref2）。
    Broken {
        script: String,
        scope: u32,
        break_position: usize,
    },
}

#[cfg(test)]
#[path = "talk_lifecycle_tests.rs"]
mod tests;

//! TimedSchedule<T> — 0 ベース相対オフセットの汎用配信エンジン。
//!
//! Entry<T> により Payload / Barrier / Routing を型レベルで 3 種分離する。
//! `tick()` + `ready()` の 2 フェーズ API で `DolaRuntime` の `tick()` + `last_result()` と対称。

use std::fmt::Debug;

use super::command::{BarrierKind, RoutingCommand};

// ============================================================================
// Entry<T> — エントリの型レベル 3 種分離
// ============================================================================

/// エントリの型レベル 3 種分離。
/// f64 = スケジュール開始からの相対オフセット（0 ベース）。
#[derive(Clone, Debug)]
pub enum Entry<T> {
    /// 時刻付きデータペイロード（実行すべきコマンド）
    Payload(f64, T),
    /// 時刻付きバリア（進行停止点、TimedSchedule が消費）
    Barrier(f64, BarrierKind),
    /// 時刻付きルーティング（配送制御、CueQueue 層が消費）
    Routing(f64, RoutingCommand),
}

impl<T> Entry<T> {
    /// エントリの時刻オフセットを取得する。
    pub fn offset(&self) -> f64 {
        match self {
            Entry::Payload(t, _) => *t,
            Entry::Barrier(t, _) => *t,
            Entry::Routing(t, _) => *t,
        }
    }
}

// ============================================================================
// TimedSchedule<T>
// ============================================================================

/// 0 ベース相対オフセットの汎用配信エンジン。
///
/// Entry<T> により Payload / Barrier / Routing を型レベルで 3 種分離する。
/// ジェネリクス T は Payload(f64, T) のみに適用される。
/// Barrier(BarrierKind) / Routing(RoutingCommand) は T に関わらず固定型。
#[derive(Debug)]
pub struct TimedSchedule<T> {
    /// 絶対時刻での開始時刻
    start_time: f64,
    /// 降順ソート（0 ベース相対オフセット）。末尾 pop で O(1) 消費。
    entries: Vec<Entry<T>>,
    /// tick() で収集した Payload
    ready_buffer: Vec<T>,
    /// tick() で収集した Routing
    routing_buffer: Vec<RoutingCommand>,
    /// 現在停止中のバリア
    current_barrier: Option<BarrierKind>,
    /// 現在の相対オフセット位置（前回 tick 時点）
    current_offset: f64,
    /// バリアタイムアウトの絶対オフセット（barrier_offset + timeout_duration）
    barrier_timeout_offset: Option<f64>,
    /// 占有終了 horizon（相対オフセット・0 ベース）。台本の全 cue の
    /// `max(start_time + duration)` を canonical 変換が焼き込む（D6）。
    ///
    /// cue は点でなく区間 `[start, start+duration)` ゆえ、**entries を配り終えた**（＝
    /// 最後の cue の**配送時刻**に達した）だけでは talk は完了でなく、**現在時刻が
    /// この horizon に達して初めて**占有終了＝完了とみなす（末尾 Wait・最終 Text の
    /// duration を終端で落とさない・早期終了しない）。
    ///
    /// [`new`](Self::new) の既定は 0.0。`current_offset` は常に非負（`tick` の負値ガード）
    /// ゆえ horizon=0.0 は `current_offset >= horizon` を常に満たし、
    /// [`is_completed`](Self::is_completed) は「entries 枯渇かつ barrier なし」の
    /// 旧挙動に一致する（手組みスケジュールの後方互換）。
    horizon: f64,
}

impl<T: Clone + Debug> TimedSchedule<T> {
    /// 絶対時刻 `start_time` でスケジュールを構築（占有 horizon は 0.0 既定）。
    pub fn new(start_time: f64) -> Self {
        Self::with_horizon(start_time, 0.0)
    }

    /// 絶対時刻 `start_time` と占有 horizon（相対）を指定してスケジュールを構築。
    ///
    /// canonical 変換（[`crate::cue::to_talk_schedule`]）が台本の占有 horizon を焼き込むために
    /// 使う。手組みスケジュール（[`new`](Self::new)）は horizon=0.0 で旧挙動を保つ。
    pub fn with_horizon(start_time: f64, horizon: f64) -> Self {
        Self {
            start_time,
            entries: Vec::new(),
            ready_buffer: Vec::new(),
            routing_buffer: Vec::new(),
            current_barrier: None,
            current_offset: 0.0,
            barrier_timeout_offset: None,
            horizon,
        }
    }

    /// エントリを時刻順ソート維持で挿入（0 ベース相対オフセット）。
    /// 降順ソートを維持する。
    ///
    /// NOTE(D3-V): NaN オフセットは `offset >= 0.0` が false となるため debug
    /// ビルドでは下の debug_assert が発火する（tests/cue/schedule_test.rs で特性化済み）。
    /// release ビルドでは素通りし、`partition_point` の前提（述語が前半 true /
    /// 後半 false に分割されていること）を破るため、以後の挿入位置が不定となり
    /// 配信順が黙って崩れ得る（panic はしない）。CueSheet 経路には DolaDocument の
    /// validate() に相当する数値検証層がない（P25 参照）。
    pub fn insert(&mut self, entry: Entry<T>) {
        debug_assert!(
            entry.offset() >= 0.0,
            "Entry offset must be non-negative: {}",
            entry.offset()
        );
        // 降順ソート: 大きいオフセットが先頭、小さいオフセットが末尾
        let offset = entry.offset();
        let pos = self
            .entries
            .partition_point(|e| e.offset() > offset);
        self.entries.insert(pos, entry);
    }

    /// 複数エントリを一括挿入（内部で再ソート）。
    ///
    /// NOTE(D3-V): NaN オフセットの扱いは insert() と同様（debug_assert 発火 /
    /// release 素通り）。ソート比較は NaN を Equal 扱い（`unwrap_or`）するため
    /// panic はしないが、NaN を含む場合の順序は規定されない（P25 参照）。
    pub fn extend(&mut self, entries: impl IntoIterator<Item = Entry<T>>) {
        for entry in entries {
            debug_assert!(
                entry.offset() >= 0.0,
                "Entry offset must be non-negative: {}",
                entry.offset()
            );
            self.entries.push(entry);
        }
        // 降順ソート
        self.entries
            .sort_by(|a, b| b.offset().partial_cmp(&a.offset()).unwrap_or(std::cmp::Ordering::Equal));
    }

    // ── 2 フェーズ API（DolaRuntime の tick/last_result と対称） ──

    /// Phase 1: 新たな時刻を入力としてランタイムの時刻を進める。
    ///
    /// `current_time` は絶対時刻。内部で `current_time - start_time` に変換して
    /// 相対オフセットとして扱う。
    ///
    /// 時刻到達済みの Payload を `ready_buffer` に、Routing を `routing_buffer` に
    /// 蒐集しながら進行。Barrier 到達（外部解決が必要）または末尾到達で停止。
    /// Routing は通過（停止しない）。冪等（同一時刻の再呼び出し安全）。
    ///
    /// NOTE(D3-V): `current_time`（または start_time）が NaN の場合 offset は NaN
    /// となり、下の全ガード（負値チェック・冪等性チェック）と本体の
    /// `entry_offset > offset` 比較がすべて false になるため、最初のバリアまでの
    /// 全エントリが即時配信される（時刻入力の有限性は検証されない — P25 参照。
    /// tests/cue/schedule_test.rs::tick_with_nan_time_delivers_all_pending_payloads
    /// で特性化済み）。
    pub fn tick(&mut self, current_time: f64) {
        let offset = current_time - self.start_time;
        if offset < 0.0 {
            return;
        }

        // 冪等性: 同一オフセットで再呼び出し → バッファを変更しない
        if offset <= self.current_offset && !self.ready_buffer.is_empty() {
            return;
        }

        // バリア中の場合
        if self.current_barrier.is_some() {
            // タイムアウト自動解除チェック
            if let Some(timeout_offset) = self.barrier_timeout_offset {
                if offset >= timeout_offset {
                    // タイムアウト解除
                    self.current_barrier = None;
                    self.barrier_timeout_offset = None;
                } else {
                    // まだタイムアウトしていない → バリア継続
                    return;
                }
            } else {
                // タイムアウトなし → 外部解除待ち
                return;
            }
        }

        self.current_offset = offset;
        self.ready_buffer.clear();
        self.routing_buffer.clear();

        // entries は降順ソート → 末尾からpopして時刻到達を消費
        while let Some(entry) = self.entries.last() {
            let entry_offset = entry.offset();
            if entry_offset > offset {
                break; // まだ到達していない
            }

            // SAFETY: 直前の while let Some(..) = self.entries.last() ガードにより
            // entries は非空であるため、pop() の unwrap は panic 不能。
            let entry = self.entries.pop().unwrap();
            match entry {
                Entry::Payload(_, payload) => {
                    self.ready_buffer.push(payload);
                }
                Entry::Barrier(barrier_offset, kind) => {
                    // タイムアウトチェック（Timeout は duration が必須タイムアウト）
                    let timeout_dur = match &kind {
                        BarrierKind::WaitForInput { timeout } => *timeout,
                        BarrierKind::WaitForChoice { timeout } => *timeout,
                        BarrierKind::Timeout { duration } => Some(*duration),
                    };

                    // タイムアウト解除チェック（全バリア種別共通）
                    if let Some(dur) = timeout_dur {
                        let timeout_abs = barrier_offset + dur;
                        if offset >= timeout_abs {
                            // 既にタイムアウト → スキップ
                            continue;
                        }
                        self.barrier_timeout_offset = Some(timeout_abs);
                    }

                    self.current_barrier = Some(kind);
                    return; // バリア到達で停止
                }
                Entry::Routing(_, routing) => {
                    self.routing_buffer.push(routing);
                    // Routing は通過（停止しない）
                }
            }
        }
    }

    /// Phase 2: 直前の tick() で収集された Payload スライスを返す。
    /// 次の tick() まで何度でも参照可能。
    pub fn ready(&self) -> &[T] {
        &self.ready_buffer
    }

    // ── バリア管理 ──

    /// 現在停止中のバリア種別を照会（UI 表示用）。
    pub fn current_barrier(&self) -> Option<&BarrierKind> {
        self.current_barrier.as_ref()
    }

    /// バリア解除プッシュ通知（外部イベント駆動）。
    ///
    /// - WaitForInput: `choice_id = None`
    /// - WaitForChoice: `choice_id = Some(選択ID)`
    /// - 非バリア時: no-op
    pub fn notify_barrier_resolved(&mut self, _choice_id: Option<String>) {
        if self.current_barrier.is_none() {
            return; // 非バリア時は no-op
        }
        self.current_barrier = None;
        self.barrier_timeout_offset = None;
    }

    // ── ルーティング ──

    /// 時刻到達済みルーティングコマンドを取得（CueQueue 層が消費）。
    /// FIFO 順で 1 件ずつ返す。
    pub fn next_routing(&mut self) -> Option<RoutingCommand> {
        if self.routing_buffer.is_empty() {
            None
        } else {
            Some(self.routing_buffer.remove(0))
        }
    }

    // ── ユーティリティ ──

    /// 残りエントリ数
    pub fn remaining(&self) -> usize {
        self.entries.len()
    }

    /// 占有終了（完了）判定: 全エントリ消費済み・バリア中でない・かつ現在時刻が
    /// 占有 horizon に達している。
    ///
    /// entries 枯渇（＝最後の cue の**配送時刻**到達）だけでは完了とせず、cue を区間
    /// `[start, start+duration)` と見て**占有終了 horizon**（`max(start+duration)`）到達まで
    /// 完了扱いしない（末尾 Wait・最終 Text の duration を終端で落とさない・D6/R2.5）。
    ///
    /// horizon=0.0（[`new`](Self::new) 既定）では `current_offset >= 0.0` が常に真ゆえ
    /// 「entries 枯渇かつ barrier なし」の旧挙動に一致する（手組みスケジュールの後方互換）。
    pub fn is_completed(&self) -> bool {
        self.entries.is_empty()
            && self.current_barrier.is_none()
            && self.current_offset >= self.horizon
    }

    /// 全エントリとバッファをクリア（horizon も 0.0 へリセット＝clear 済みは完了状態）。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.ready_buffer.clear();
        self.routing_buffer.clear();
        self.current_barrier = None;
        self.current_offset = 0.0;
        self.barrier_timeout_offset = None;
        self.horizon = 0.0;
    }
}

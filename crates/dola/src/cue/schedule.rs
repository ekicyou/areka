//! TimedSchedule<T> — 0 ベース相対オフセットの汎用配信エンジン。
//!
//! Entry<T> により Payload / Barrier を型レベルで 2 種分離する。
//! `tick()` + `ready()` の 2 フェーズ API で `DolaRuntime` の `tick()` + `last_result()` と対称。

use std::fmt::Debug;

use super::command::BarrierKind;

/// 2 つの秒を足す。どちらも有限で ms の格子に載るなら ms の整数で足してから秒に直し、
/// 載らなければ f64 の和そのもの（時刻を丸めない）。
///
/// 格子に載る値どうしの結果は Tick の式 `n as f64 / 1000.0` と同じ値になる（例 0.15＋0.8＝0.95。
/// 素の和は 0.9500000000000001 で、ちょうどの刻みで届かない）。
// shortcut: f64 の秒のあいだだけの継ぎ。dola-interrupt-ticks で時刻が整数になったら消す
pub(crate) fn sum_on_ms_grid(a: f64, b: f64) -> f64 {
    let on_grid = |x: f64| x.is_finite() && (x * 1000.0).round() / 1000.0 == x;
    if on_grid(a) && on_grid(b) {
        let (a_ms, b_ms) = ((a * 1000.0).round(), (b * 1000.0).round());
        // ms どうしは 2^53 未満の整数の和で丸めが無い
        (a_ms + b_ms) / 1000.0
    } else {
        a + b
    }
}

// ============================================================================
// Entry<T> — エントリの型レベル 2 種分離
// ============================================================================

/// エントリの型レベル 2 種分離。
/// f64 = スケジュール開始からの相対オフセット（0 ベース）。
#[derive(Clone, Debug)]
pub enum Entry<T> {
    /// 時刻付きデータペイロード（実行すべきコマンド）
    Payload(f64, T),
    /// 時刻付きバリア（進行停止点、TimedSchedule が消費）
    Barrier(f64, BarrierKind),
}

impl<T> Entry<T> {
    /// エントリの時刻オフセットを取得する。
    pub fn offset(&self) -> f64 {
        match self {
            Entry::Payload(t, _) => *t,
            Entry::Barrier(t, _) => *t,
        }
    }
}

// ============================================================================
// TimedSchedule<T>
// ============================================================================

/// 0 ベース相対オフセットの汎用配信エンジン。
///
/// Entry<T> により Payload / Barrier を型レベルで 2 種分離する。
/// ジェネリクス T は Payload(f64, T) のみに適用される。
/// Barrier(BarrierKind) は T に関わらず固定型。
#[derive(Debug)]
pub struct TimedSchedule<T> {
    /// 絶対時刻での開始時刻
    start_time: f64,
    /// 降順ソート（0 ベース相対オフセット）。末尾 pop で O(1) 消費。
    entries: Vec<Entry<T>>,
    /// tick() で収集した Payload
    ready_buffer: Vec<T>,
    /// 現在停止中のバリア
    current_barrier: Option<BarrierKind>,
    /// 前回 tick の絶対時刻（構築・clear の直後はアンカー `start_time`）。
    ///
    /// 到達・完了の判定はすべて「アンカー＋相対」の足し算で作った絶対時刻とこの値を
    /// 比べる（絶対の発火時刻の定義 [`CueSheet::absolute_fire_time`](crate::cue::CueSheet::absolute_fire_time)
    /// と同じ形）。「現在時刻−アンカー」の引き算で相対へ戻すと、丸めでその時刻ちょうどの
    /// tick が届かないことがあるため、引き算は使わない。
    current_time: f64,
    /// 占有終了 horizon（相対オフセット・0 ベース）。台本の全 cue の
    /// `max(start_time + duration)` を canonical 変換が焼き込む（D6）。
    ///
    /// cue は点でなく区間 `[start, start+duration)` ゆえ、**entries を配り終えた**（＝
    /// 最後の cue の**配送時刻**に達した）だけでは talk は完了でなく、**現在時刻が
    /// この horizon に達して初めて**占有終了＝完了とみなす（末尾 Wait・最終 Text の
    /// duration を終端で落とさない・早期終了しない）。
    ///
    /// [`new`](Self::new) の既定は 0.0。`current_time` は常にアンカー以上（`tick` の手前ガード）
    /// ゆえ horizon=0.0 は `current_time >= start_time + horizon` を常に満たし、
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
            current_barrier: None,
            current_time: start_time,
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
    /// validate() に相当する数値検証層がない。
    pub fn insert(&mut self, entry: Entry<T>) {
        debug_assert!(
            entry.offset() >= 0.0,
            "Entry offset must be non-negative: {}",
            entry.offset()
        );
        // 降順ソート: 大きいオフセットが先頭、小さいオフセットが末尾
        let offset = entry.offset();
        let pos = self.entries.partition_point(|e| e.offset() > offset);
        self.entries.insert(pos, entry);
    }

    /// `entries` を順に [`insert`](Self::insert) したのと同じ並びを、まとめて作る（同一オフセットは
    /// 後に入れたものが前＝先に入れたものから配る FIFO）。`insert` を 1 件ずつ呼ぶと昇順の台本で
    /// 毎回先頭へ挿すので全体が台本の長さの 2 乗になる。ここは逆順に積んで安定ソートする
    /// （昇順の台本なら既に降順の 1 本の並びで、ソートは 1 回の走査で済む）。
    pub fn insert_each(&mut self, entries: impl IntoIterator<Item = Entry<T>>) {
        let mut all: Vec<Entry<T>> = entries.into_iter().collect();
        for entry in &all {
            debug_assert!(
                entry.offset() >= 0.0,
                "Entry offset must be non-negative: {}",
                entry.offset()
            );
        }
        all.reverse();
        all.append(&mut self.entries);
        // −0.0 を +0.0 にそろえた全順序の降順（同じ瞬間を FIFO に保つ）
        all.sort_by(|a, b| (b.offset() + 0.0).total_cmp(&(a.offset() + 0.0)));
        self.entries = all;
    }

    // ── 2 フェーズ API（DolaRuntime の tick/last_result と対称） ──

    /// Phase 1: 新たな時刻を入力としてランタイムの時刻を進める。
    ///
    /// `current_time` は絶対時刻。各エントリの到達は `start_time + オフセット` の絶対時刻と
    /// 比べて判定する（相対へ引き戻さない・`current_time` 欄の説明を参照）。
    ///
    /// 時刻到達済みの Payload を `ready_buffer` に蒐集しながら進行。Barrier 到達（外部解決が
    /// 必要）または末尾到達で停止。冪等（同一時刻の再呼び出し安全）。
    ///
    /// NOTE(D3-V): `current_time`（または start_time）が NaN の場合、下の全ガード
    /// （手前チェック・冪等性チェック）と本体の「届いていない」の比べ
    /// （`sum_on_ms_grid(start_time, entry_offset) > current_time`）がすべて false になるため、
    /// 最初のバリアまでの全エントリが即時配信される（時刻入力の有限性は検証されない。
    /// tests/cue/schedule_test.rs の `tick_with_nan_time_delivers_all_pending_payloads`
    /// で特性化済み）。
    // 到達の比べは NaN を「届いた」に倒すため、わざと `>` の否定で書く
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn tick(&mut self, current_time: f64) {
        if current_time < self.start_time {
            return;
        }

        // 冪等性: 同一時刻で再呼び出し → バッファを変更しない
        if current_time <= self.current_time && !self.ready_buffer.is_empty() {
            return;
        }

        // バリア中は外部解除待ち
        if self.current_barrier.is_some() {
            return;
        }

        self.current_time = current_time;
        self.ready_buffer.clear();

        // entries は降順ソート → 末尾から時刻到達を消費
        let start_time = self.start_time;
        while let Some(entry) = self
            .entries
            .pop_if(|e| !(sum_on_ms_grid(start_time, e.offset()) > current_time))
        {
            match entry {
                Entry::Payload(_, payload) => {
                    self.ready_buffer.push(payload);
                }
                // 選択待ちの `timeout` は上位層（kanade の期限の判定）へ運ぶ指令で、再生層はこれで
                // 区切りを飛ばしも解きもしない。解けるのは外からの `notify_barrier_resolved` だけ
                // （areka-P0-choice-timeout-directive 要件 5.1〜5.3）。
                Entry::Barrier(_, kind) => {
                    self.current_barrier = Some(kind);
                    return; // バリア到達で停止
                }
            }
        }
    }

    /// Phase 2: 直前の tick() で収集された Payload スライスを返す。
    /// 次の tick() まで何度でも参照可能（`notify_barrier_resolved` でも空になる）。
    pub fn ready(&self) -> &[T] {
        &self.ready_buffer
    }

    // ── バリア管理 ──

    /// 現在停止中のバリア種別を照会（UI 表示用）。
    pub fn current_barrier(&self) -> Option<&BarrierKind> {
        self.current_barrier.as_ref()
    }

    /// バリア解除プッシュ通知（外部イベント駆動）。非バリア時は no-op。
    pub fn notify_barrier_resolved(&mut self) {
        if self.current_barrier.is_none() {
            return; // 非バリア時は no-op
        }
        self.current_barrier = None;
        // 集めは区切りの tick で配り済み。空にして、同じ時刻の次の tick が冪等の守りに止められず
        // 区切りの後ろを集められるようにする（解決が次の Tick より先に来る形）。
        self.ready_buffer.clear();
    }

    // ── ユーティリティ ──

    /// 残りエントリ数
    pub fn remaining(&self) -> usize {
        self.entries.len()
    }

    /// まだ配っていない中身（区切りを除く）を配る順に返す。
    ///
    /// `entries` は降順（末尾から取り出す）ゆえ後ろから読む。
    pub(crate) fn pending_payloads(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().rev().filter_map(|entry| match entry {
            Entry::Payload(_, payload) => Some(payload),
            Entry::Barrier(..) => None,
        })
    }

    /// 占有終了（完了）判定: 全エントリ消費済み・バリア中でない・かつ現在時刻が
    /// 占有 horizon に達している。
    ///
    /// entries 枯渇（＝最後の cue の**配送時刻**到達）だけでは完了とせず、cue を区間
    /// `[start, start+duration)` と見て**占有終了 horizon**（`max(start+duration)`）到達まで
    /// 完了扱いしない（末尾 Wait・最終 Text の duration を終端で落とさない・D6/R2.5）。
    ///
    /// 閾値は [`occupancy_horizon`](Self::occupancy_horizon) と同じ `start_time` と `horizon` の和。
    ///
    /// horizon=0.0（[`new`](Self::new) 既定）では `current_time >= start_time` が常に真ゆえ
    /// 「entries 枯渇かつ barrier なし」の旧挙動に一致する（手組みスケジュールの後方互換）。
    pub fn is_completed(&self) -> bool {
        self.entries.is_empty()
            && self.current_barrier.is_none()
            && self.current_time >= sum_on_ms_grid(self.start_time, self.horizon)
    }

    /// 全エントリとバッファをクリア（horizon も 0.0 へリセット＝clear 済みは完了状態）。
    pub fn clear(&mut self) {
        self.entries.clear();
        self.ready_buffer.clear();
        self.current_barrier = None;
        self.current_time = self.start_time;
        self.horizon = 0.0;
    }
}

/// ペイロード型 T に依存しない照会（エントリを触らないため `Clone + Debug` 境界を要さない）。
impl<T> TimedSchedule<T> {
    /// 占有終了 horizon の**絶対時刻** = アンカー `start_time` と相対 horizon の和（`sum_on_ms_grid`）。
    ///
    /// [`is_completed`](Self::is_completed) が閾値として見ている占有区間の終端を、
    /// 上位層が絶対時刻として照会するための口（R7.2）。値は [`tick`](Self::tick) の
    /// 進行では変化しない。[`crate::cue::to_talk_schedule`] で組んだ予定では、台本の
    /// [`CueSheet::absolute_end_time`](crate::cue::CueSheet::absolute_end_time) と同じ値になる
    /// （両方が同じ相対の終わりの関数と同じ和を使う。非有限・負の長さも同じ clamp を通る）。
    pub fn occupancy_horizon(&self) -> f64 {
        sum_on_ms_grid(self.start_time, self.horizon)
    }
}

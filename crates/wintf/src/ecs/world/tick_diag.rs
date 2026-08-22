//! フレーム駆動の相別観測——tick 1 回の所要を 13 本のスケジュールへ割り、1 秒の窓で
//! まとめて 1 行にする（既定 OFF）。
//!
//! # 何のためか
//!
//! 「どこが重いか」を 4 段（プロセス → スレッド → 関数 → **フレーム駆動の相**）で絞る
//! ときの、最後の 1 段がここである。tick が 1 秒に何回回り、1 回あたり何 µs かかり、
//! その内訳が 13 本のどこに寄っているか——この 3 つは他のどの道具からも出てこない
//! （サンプリングは関数名までしか教えず、スレッド別 CPU は UI スレッドの合計しか
//! 教えない）。
//!
//! # 既定 OFF と前置ガード
//!
//! 観測は `RUST_LOG` に `wintf::tick=debug` を足したときだけ点く。点いていないとき、
//! この観測のために**時刻を 1 度も取らず**、文字列も 1 つも組み立てない
//! （要件 3.8）。判定は [`is_enabled`] を tick の冒頭で 1 度だけ行い、以後はその
//! `bool` を配る。[`PhaseTimer`] は OFF なら `lap` を叩かれても何もしない。
//!
//! # 壁時計と CPU 時間を混ぜない
//!
//! 13 本の内訳と `wall_us`／`max_us` は**壁時計**（経過時間）であり、GPU 待ちや
//! ページフォルトの待ちが混じる。`ui_cpu_us` だけが UI スレッドの**CPU 時間**
//! （`GetThreadTimes` の差分）である。両者は別のフィールド名で出す（要件 2.6）。
//!
//! # 行の形
//!
//! ```text
//! [tick] kind=window frame=… t_ms=… ticks=… skipped=… heartbeat=… wall_us=… max_us=…
//!        ui_cpu_us=… input_us=… update_us=… …（13 本）… framefinalize_us=…
//! ```
//!
//! 1 行の中に同じフィールド名は 2 度出ない（要件 2.12——判定スクリプトの `parse_fields`
//! は後勝ちで上書きするため、重複は静かに値を失わせる）。

use std::os::windows::io::OwnedHandle;
use std::time::{Duration, Instant};

use tracing::error;

use crate::api::{duplicate_current_thread_handle, get_thread_times};

/// 観測チャネル（`RUST_LOG` の target）。
pub const TICK_TARGET: &str = "wintf::tick";

/// 窓の長さ（ms）。窓が閉じるたびに 1 行出る。
pub const TICK_DIAG_WINDOW_MS: u64 = 1000;

/// `try_tick_world` が回すスケジュールのラベル名（**実行順**）。
///
/// 兄弟テストが `mod.rs` の本文を読んで、この並びと `try_run_schedule` の並びが
/// 一致することを固定する。
pub const SCHEDULE_LABELS: [&str; 13] = [
    "Input",
    "Update",
    "PreLayout",
    "Layout",
    "PostLayout",
    "UISetup",
    "GraphicsSetup",
    "Draw",
    "PreRenderSurface",
    "RenderSurface",
    "Composition",
    "CommitComposition",
    "FrameFinalize",
];

/// 行に出る 13 本のフィールド名の語幹（[`SCHEDULE_LABELS`] の小文字）。
///
/// 実際のフィールド名は `<語幹>_us` である。
pub const SCHEDULE_NAMES: [&str; 13] = [
    "input",
    "update",
    "prelayout",
    "layout",
    "postlayout",
    "uisetup",
    "graphicssetup",
    "draw",
    "prerendersurface",
    "rendersurface",
    "composition",
    "commitcomposition",
    "framefinalize",
];

/// 観測チャネルが点いているか（前置ガード）。
///
/// `try_tick_world` の冒頭で 1 度だけ呼び、以後はその `bool` を配る。
#[inline]
pub fn is_enabled() -> bool {
    tracing::enabled!(target: TICK_TARGET, tracing::Level::DEBUG)
}

/// 組み上がった 1 行を観測チャネルへ出す。
///
/// 本文は行そのもの（構造化フィールドへ分解しない）——解析側は行の字面を
/// `名前=値` で辞書化して読むため、書式は [`format_window_line`] の 1 箇所に閉じる。
#[inline]
pub fn emit_line(line: &str) {
    tracing::debug!(target: TICK_TARGET, "{line}");
}

/// 1 つの窓が閉じたときの集計値（行の materialize 元）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowSnapshot {
    /// 窓の中で最後に回った tick のフレーム番号。
    pub frame: u32,
    /// 窓の実長（ms）。
    pub t_ms: u64,
    /// 回った tick の数。
    pub ticks: u32,
    /// 省略した tick の数（門が入るまでは常に 0）。
    pub skipped: u32,
    /// 心拍（旗が無くても回す安全側の網）で回った tick の数。
    pub heartbeat: u32,
    /// 窓内の tick 所要の合計（µs・**壁時計**）。
    pub wall_us: u64,
    /// 窓内の tick 所要の最大（µs・**壁時計**）。
    pub max_us: u64,
    /// 窓内の UI スレッド CPU 時間の差分（µs・**CPU 時間**）。
    pub ui_cpu_us: u64,
    /// 13 本のスケジュール別の所要合計（µs・**壁時計**・[`SCHEDULE_NAMES`] の順）。
    pub per_schedule_us: [u64; 13],
}

/// 窓 1 つを 1 行へ組む（純関数）。
///
/// フィールドの並びと綴りはここが権威であり、兄弟テストが重複なしと 13 本の順序を
/// 固定する。既存の `perf(…)`／`[transition]` 行とは文言もフィールド名も重ならない。
pub fn format_window_line(w: &WindowSnapshot) -> String {
    let mut line = format!(
        "[tick] kind=window frame={frame} t_ms={t_ms} ticks={ticks} skipped={skipped} \
         heartbeat={heartbeat} wall_us={wall_us} max_us={max_us} ui_cpu_us={ui_cpu_us}",
        frame = w.frame,
        t_ms = w.t_ms,
        ticks = w.ticks,
        skipped = w.skipped,
        heartbeat = w.heartbeat,
        wall_us = w.wall_us,
        max_us = w.max_us,
        ui_cpu_us = w.ui_cpu_us,
    );
    for (name, us) in SCHEDULE_NAMES.iter().zip(w.per_schedule_us.iter()) {
        line.push_str(&format!(" {name}_us={us}"));
    }
    line
}

/// スケジュール 1 本ずつの区間を刻む計時器。
///
/// OFF（`start(false)`）なら [`lap`](Self::lap) は**何もしない**——時刻取得の経路に
/// 入らないので、既定運転の費用は 0 である（要件 3.8）。ON なら `lap` の呼び出し
/// 1 回が 1 本ぶんの区間を閉じ、[`SCHEDULE_NAMES`] の順に配列へ書く。
#[derive(Debug)]
pub struct PhaseTimer {
    /// 直前の区切り時刻。OFF のときは `None`（＝計時しない印）。
    last: Option<Instant>,
    /// 13 本ぶんの所要（µs）。
    per_schedule: [u64; 13],
    /// 次に書く位置。13 を超えた `lap` は配列へ書かない（数だけ進む）。
    laps: usize,
}

impl PhaseTimer {
    /// 計時を開始する。`diag_on` が偽なら時刻を取らない。
    #[inline]
    pub fn start(diag_on: bool) -> Self {
        Self {
            last: if diag_on { Some(Instant::now()) } else { None },
            per_schedule: [0; 13],
            laps: 0,
        }
    }

    /// スケジュール 1 本ぶんの区間を閉じる。
    #[inline]
    pub fn lap(&mut self) {
        if let Some(last) = self.last {
            let now = Instant::now();
            if let Some(slot) = self.per_schedule.get_mut(self.laps) {
                *slot = now.saturating_duration_since(last).as_micros() as u64;
            }
            self.laps += 1;
            self.last = Some(now);
        }
    }

    /// 13 本ぶんの所要（µs）。
    #[inline]
    pub fn per_schedule(&self) -> &[u64; 13] {
        &self.per_schedule
    }

    /// 区間を閉じた回数（OFF なら 0）。
    #[inline]
    pub fn laps(&self) -> usize {
        self.laps
    }
}

/// 1 秒窓の集計器（`EcsWorld` が 1 つ持つ）。
///
/// 窓は最初の記録（回った／省略した）で開き、`TICK_DIAG_WINDOW_MS` を跨いだ次の
/// 取り出しで閉じる。閉じた時刻がそのまま次の窓の起点になる（間が空かない）。
#[derive(Debug, Default)]
pub struct TickDiag {
    /// 窓の起点。まだ 1 度も記録が無ければ `None`。
    t0: Option<Instant>,
    /// 最後に回った tick のフレーム番号。
    frame: u32,
    ticks: u32,
    skipped: u32,
    heartbeat: u32,
    wall_us_sum: u64,
    wall_us_max: u64,
    per_schedule_us: [u64; 13],
    /// UI スレッドの複製ハンドル（CPU 時間の読み出し用）。
    ui_thread: Option<OwnedHandle>,
    /// ハンドルの取得を 1 度でも試したか（失敗を毎 tick 繰り返さない）。
    ui_thread_attempted: bool,
    /// 窓の起点における UI スレッド CPU 時間（µs）。
    ui_cpu_start_us: u64,
}

impl TickDiag {
    /// UI スレッドのハンドルを 1 度だけ確保する（点灯している tick から呼ぶ）。
    ///
    /// 失敗は握り潰さず記録し、以後は CPU 時間を 0 として続ける（観測が欠けるだけで
    /// 本番の駆動は止めない）。
    pub fn ensure_ui_thread(&mut self) {
        if self.ui_thread_attempted {
            return;
        }
        self.ui_thread_attempted = true;
        match duplicate_current_thread_handle() {
            Ok(handle) => {
                self.ui_thread = Some(handle);
                self.ui_cpu_start_us = self.read_ui_cpu_us().unwrap_or(0);
            }
            Err(e) => error!(
                error = %e,
                "tick 観測: UI スレッドのハンドル複製に失敗（ui_cpu_us は 0 のまま続行）"
            ),
        }
    }

    /// UI スレッドの累積 CPU 時間（µs）。ハンドルが無い／読めないときは `None`。
    fn read_ui_cpu_us(&self) -> Option<u64> {
        let handle = self.ui_thread.as_ref()?;
        match get_thread_times(handle) {
            Ok(times) => Some(times.total_us()),
            Err(e) => {
                error!(error = %e, "tick 観測: UI スレッド CPU 時間の読み出しに失敗");
                None
            }
        }
    }

    /// 回った tick を 1 件記録する。
    ///
    /// `wall_us` は tick 1 回の所要（壁時計）、`per_schedule` は
    /// [`SCHEDULE_NAMES`] の順の内訳、`heartbeat` は心拍で回した tick かどうか。
    pub fn record_run(
        &mut self,
        now: Instant,
        frame: u32,
        wall_us: u64,
        per_schedule: &[u64; 13],
        heartbeat: bool,
    ) {
        self.open_window(now);
        self.frame = frame;
        self.ticks += 1;
        if heartbeat {
            self.heartbeat += 1;
        }
        self.wall_us_sum += wall_us;
        self.wall_us_max = self.wall_us_max.max(wall_us);
        for (slot, add) in self.per_schedule_us.iter_mut().zip(per_schedule.iter()) {
            *slot += add;
        }
    }

    /// 省略した tick を 1 件記録する（門が入ってから使う——要件 2.5 の省略率）。
    pub fn record_skipped(&mut self, now: Instant) {
        self.open_window(now);
        self.skipped += 1;
    }

    /// 窓が閉じていれば集計値を取り出し、窓を開き直す（純粋な取り出し口）。
    ///
    /// `ui_cpu_us` は呼び出し側が渡す——CPU 時間の読み出しは OS 呼び出しであり、
    /// 窓の切れ目の判定と混ぜると決定論テストが書けなくなるため分けてある。
    pub fn take_window(&mut self, now: Instant, ui_cpu_us: u64) -> Option<WindowSnapshot> {
        let t0 = self.t0?;
        let elapsed = now.saturating_duration_since(t0);
        if elapsed < Duration::from_millis(TICK_DIAG_WINDOW_MS) {
            return None;
        }
        let snapshot = WindowSnapshot {
            frame: self.frame,
            t_ms: elapsed.as_millis() as u64,
            ticks: self.ticks,
            skipped: self.skipped,
            heartbeat: self.heartbeat,
            wall_us: self.wall_us_sum,
            max_us: self.wall_us_max,
            ui_cpu_us,
            per_schedule_us: self.per_schedule_us,
        };
        // 次の窓は「閉じた時刻」から始める（取りこぼしの区間を作らない）。
        // frame は最後に回った番号なので持ち越す（回らない窓でも番号は最新が正）。
        self.t0 = Some(now);
        self.ticks = 0;
        self.skipped = 0;
        self.heartbeat = 0;
        self.wall_us_sum = 0;
        self.wall_us_max = 0;
        self.per_schedule_us = [0; 13];
        Some(snapshot)
    }

    /// 窓が閉じていれば 1 行を組んで返す（実行体からの入口）。
    ///
    /// UI スレッド CPU の差分はここで読む。窓が閉じていないときは OS 呼び出しを
    /// 1 度も行わない。
    pub fn maybe_close(&mut self, now: Instant) -> Option<String> {
        if !self.window_elapsed(now) {
            return None;
        }
        let (ui_cpu_us, next_start) = match self.read_ui_cpu_us() {
            Some(cpu_now) => (cpu_now.saturating_sub(self.ui_cpu_start_us), Some(cpu_now)),
            None => (0, None),
        };
        let snapshot = self.take_window(now, ui_cpu_us)?;
        if let Some(start) = next_start {
            self.ui_cpu_start_us = start;
        }
        Some(format_window_line(&snapshot))
    }

    /// 窓が長さに達しているか（OS 呼び出しの手前で切る安い判定）。
    fn window_elapsed(&self, now: Instant) -> bool {
        match self.t0 {
            Some(t0) => {
                now.saturating_duration_since(t0) >= Duration::from_millis(TICK_DIAG_WINDOW_MS)
            }
            None => false,
        }
    }

    /// まだ窓が開いていなければ開く。
    fn open_window(&mut self, now: Instant) {
        if self.t0.is_none() {
            self.t0 = Some(now);
        }
    }
}

#[cfg(test)]
#[path = "tick_diag_tests.rs"]
mod tests;

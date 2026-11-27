//! スレッド別・プロセス全体の CPU 報告器。
//!
//! # 何を出すか
//!
//! `wintf` のスレッド名簿（`ecs::world::thread_registry`）を舐めて、スレッド 1 本につき
//! 1 行の `perf(thread)` と、プロセス全体 1 行の `perf(process)` を出す。名簿に載って
//! いないスレッド（bevy のタスクプール等）は、プロセス CPU から名簿の合計を引いた差を
//! `role=unregistered_rest` の**1 行**として必ず出す——観測が黙って消えない形にするため
//! である。
//!
//! # 既定 OFF（費用 0）
//!
//! 点灯の判定 [`is_enabled`] は `tracing::enabled!(target: "areka::perf", DEBUG)` であり、
//! [`start`] が**起動時に 1 度だけ**評価する。消灯していれば報告スレッドを起こさない
//! （`Instant` も Win32 も 1 度も触らない）。点灯していれば `areka-perf-report`
//! スレッドが [`DEFAULT_PERIOD_SEC`] 秒ごと（`AREKA_PERF_THREAD_REPORT_SEC` で変更可）と
//! **終了直前**にスナップショットを出す。費用は ON でも 1 周期に 1 回・スレッド数 ×数 µs。
//!
//! # 区間の差は読み手が取る
//!
//! 区間の差は読み手（`tools/perf/perf-rank.py` の段②・自己テストが規則を固定する）が
//! 最後のスナップショットと 60 秒前のものを比べて取る。名簿外の残りはプロセスの差から出す。
//!
//! # 壁時計と CPU 時間は混ぜない
//!
//! `perf(process)` の `wall_ms` だけが壁時計（経過時間）で、`cpu_us`／`kernel_us`／
//! `user_us` はすべて CPU 時間である。GPU 待ちなどの待ち時間は前者にだけ現れる。
//! `perf(thread)` は壁時計を持たない。
//!
//! # 行の語彙
//!
//! 1 行に同じフィールド名を 2 度出さない。値に含まれる空白は `_` へ潰す——判定
//! スクリプト `tools/perf/judge-perf.py` の `parse_fields` は「空白の直後の `名前=`」を
//! 新しいフィールドの始まりと読むため、潰さないと値の途中が別のフィールドに化ける。
//! 既存の `perf(apply_show)` 行・`[transition]` 行・`[tick]` 行とは文言もフィールド名も
//! 重ならない。

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tracing::{debug, error, info, warn};

use wintf::ecs::world::thread_registry::{
    self, CpuTimes, ROLE_PERF_REPORT, ROLE_UNREGISTERED_REST, RegistryReading, get_process_times,
};

/// 報告行の tracing target。点灯・消灯はこの target 1 本で決まる。
pub const PERF_TARGET: &str = "areka::perf";

/// 報告スレッドの名前（OS 側のスレッド名にもなる）。
pub const THREAD_NAME: &str = "areka-perf-report";

/// スナップショットの既定周期（秒）。
pub const DEFAULT_PERIOD_SEC: u64 = 60;

/// 周期を上書きする環境変数の名前（本番 env は `AREKA_` 冠・秒で指定する）。
pub const PERIOD_ENV: &str = "AREKA_PERF_THREAD_REPORT_SEC";

/// 名前・役割名が空のときに行へ置く記号（`name=` が値なしで終わらないため）。
const PLACEHOLDER: &str = "-";

/// 終了直前のスナップショットを待つ上限。これを過ぎたら待つのをやめる
/// （終了経路を報告器の都合で止めない）。
const FINAL_WAIT: Duration = Duration::from_secs(2);

/// 観測チャネルが点灯しているか（前置ガード）。
///
/// [`start`] が起動時に 1 度だけ呼ぶ。以後の周期処理はこの判定を繰り返さない
/// （点灯は走行中に変わらない前提で、報告スレッドの有無そのものが判定の結果である）。
#[inline]
pub fn is_enabled() -> bool {
    tracing::enabled!(target: PERF_TARGET, tracing::Level::DEBUG)
}

/// 組み上がった 1 行を観測チャネルへ出す。
///
/// 本文は行そのもの（構造化フィールドへ分解しない）——判定スクリプトは行の字面を
/// 辞書化して読むため、書式を純関数 1 箇所に閉じ込めておく。
#[inline]
pub fn emit_line(line: &str) {
    debug!(target: PERF_TARGET, "{line}");
}

// ---------------------------------------------------------------------------
// 標本（純データ）
// ---------------------------------------------------------------------------

/// スレッド 1 本ぶんの標本（累積の CPU 時間）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadSample {
    /// 登録時に宣言された役割名（`thread_registry` の固定語彙、または `actor:<name>`）。
    pub role: String,
    /// OS 側のスレッド名。無ければ [`PLACEHOLDER`]。
    pub name: String,
    /// スレッド ID。名簿外の残りだけは 0（実在のスレッドを指さない印）。
    pub tid: u32,
    /// カーネルモードの CPU 時間（µs・累積）。
    pub kernel_us: u64,
    /// ユーザーモードの CPU 時間（µs・累積）。
    pub user_us: u64,
}

impl ThreadSample {
    /// カーネルとユーザーの合計（µs）。
    pub const fn cpu_us(&self) -> u64 {
        self.kernel_us + self.user_us
    }
}

/// 終わったスレッドを系統ごとに畳んだ合計 1 件ぶんの標本（累積の CPU 時間）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoldedSample {
    /// 系統名（名簿が付ける。行へ出すときに空白を `_` へ潰す）。
    pub family: String,
    /// 畳んだスレッドのカーネルモード CPU 時間の合計（µs・累積）。
    pub kernel_us: u64,
    /// 畳んだスレッドのユーザーモード CPU 時間の合計（µs・累積）。
    pub user_us: u64,
    /// 畳んだスレッドの本数。
    pub exited: u64,
}

impl FoldedSample {
    /// カーネルとユーザーの合計（µs）。
    pub const fn cpu_us(&self) -> u64 {
        self.kernel_us + self.user_us
    }
}

/// プロセス全体の標本。壁時計はここにだけ在る。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessSample {
    /// 報告器の起動からの経過（ms・**壁時計**）。
    pub wall_ms: u64,
    /// プロセス全体のカーネルモード CPU 時間（µs・累積）。
    pub kernel_us: u64,
    /// プロセス全体のユーザーモード CPU 時間（µs・累積）。
    pub user_us: u64,
    /// そのスナップショットの時点で生きている名簿のスレッドの本数（CPU を読めなかったものも数える）。
    pub threads: usize,
}

impl ProcessSample {
    /// カーネルとユーザーの合計（µs）。
    pub const fn cpu_us(&self) -> u64 {
        self.kernel_us + self.user_us
    }
}

// ---------------------------------------------------------------------------
// 純関数（行の組み立て・名簿外の残り・1 枚の組み立て・周期）
// ---------------------------------------------------------------------------

/// 値に含まれる空白を `_` へ潰す（空なら [`PLACEHOLDER`]）。
///
/// 判定スクリプトの `parse_fields` は「行頭または空白の直後の `名前=`」だけを
/// フィールドの始まりと読む。値の中に空白が残っていると、その先の `語=` が新しい
/// フィールドとして立ち上がり、1 行の語彙が壊れる。空白さえ無ければ
/// 値の中の `=` は安全である。
fn sanitize(value: &str) -> String {
    let replaced: String = value
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    if replaced.is_empty() {
        PLACEHOLDER.to_owned()
    } else {
        replaced
    }
}

/// スレッド 1 本を 1 行へ組む（純関数）。
///
/// フィールドの並びと綴りはここが権威であり、兄弟テストが重複なしと順序を固定する。
pub fn format_thread_line(snap: u32, t_s: u64, s: &ThreadSample) -> String {
    format!(
        "perf(thread): スレッド別 CPU snap={snap} t_s={t_s} tid={tid} name={name} \
         role={role} cpu_us={cpu_us} kernel_us={kernel_us} user_us={user_us}",
        tid = s.tid,
        name = sanitize(&s.name),
        role = sanitize(&s.role),
        cpu_us = s.cpu_us(),
        kernel_us = s.kernel_us,
        user_us = s.user_us,
    )
}

/// 畳んだ合計 1 件を 1 行へ組む（純関数）。
///
/// 欄は `perf(thread)` と同じで `tid=0 name=-`、役割名に系統名、末尾に `exited=` を足す。
pub fn format_folded_line(snap: u32, t_s: u64, s: &FoldedSample) -> String {
    format!(
        "perf(thread): スレッド別 CPU snap={snap} t_s={t_s} tid=0 name={PLACEHOLDER} \
         role={role} cpu_us={cpu_us} kernel_us={kernel_us} user_us={user_us} exited={exited}",
        role = sanitize(&s.family),
        cpu_us = s.cpu_us(),
        kernel_us = s.kernel_us,
        user_us = s.user_us,
        exited = s.exited,
    )
}

/// プロセス全体を 1 行へ組む（純関数）。
///
/// `wall_ms` だけが壁時計で、残る 3 つは CPU 時間である。
pub fn format_process_line(snap: u32, t_s: u64, p: &ProcessSample) -> String {
    format!(
        "perf(process): プロセス CPU snap={snap} t_s={t_s} wall_ms={wall_ms} \
         cpu_us={cpu_us} kernel_us={kernel_us} user_us={user_us} threads={threads}",
        wall_ms = p.wall_ms,
        cpu_us = p.cpu_us(),
        kernel_us = p.kernel_us,
        user_us = p.user_us,
        threads = p.threads,
    )
}

/// 名簿外の残りを 1 行ぶんの標本として算出する（プロセス CPU − 生きた行の合計 − 畳んだ合計）。
///
/// 名簿に載らないスレッド（bevy のタスクプール等）は必ず残るので、その分を黙って
/// 落とさずここで拾う。終了済みのスレッドの最終値はプロセス CPU に含まれているので、
/// 畳んだ合計も引く側から外さない。
///
/// 読み取り時刻がずれてプロセスより名簿の合計が大きく見えることがある（スナップ
/// ショットは瞬間ではない）。そのときは 0 で止める——負へ回さない。
pub fn compute_unregistered_rest(
    process: &ProcessSample,
    threads: &[ThreadSample],
    folded: &[FoldedSample],
) -> ThreadSample {
    let kernel_sum: u64 = threads.iter().map(|t| t.kernel_us).sum::<u64>()
        + folded.iter().map(|f| f.kernel_us).sum::<u64>();
    let user_sum: u64 = threads.iter().map(|t| t.user_us).sum::<u64>()
        + folded.iter().map(|f| f.user_us).sum::<u64>();
    ThreadSample {
        role: ROLE_UNREGISTERED_REST.to_owned(),
        name: PLACEHOLDER.to_owned(),
        tid: 0,
        kernel_us: process.kernel_us.saturating_sub(kernel_sum),
        user_us: process.user_us.saturating_sub(user_sum),
    }
}

/// 1 枚の行と、CPU を読めなかった生きた項目の TID。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ComposedSnapshot {
    pub lines: Vec<String>,
    pub unreadable_tids: Vec<u32>,
}

/// 1 枚の行を組む（純関数・Win32 を呼ばない）。
///
/// 順は `perf(process)` → 生きた行（TID 昇順）→ 畳んだ行（系統名の昇順）→ 名簿外の残り。
pub(crate) fn compose_snapshot(
    snap: u32,
    elapsed: Duration,
    process: CpuTimes,
    reading: RegistryReading,
) -> ComposedSnapshot {
    let live_count = reading.live.len();
    let mut samples = Vec::with_capacity(live_count);
    let mut unreadable_tids = Vec::new();
    for entry in reading.live {
        match entry.cpu {
            Ok(times) => samples.push(ThreadSample {
                role: entry.role,
                name: entry.name.unwrap_or_else(|| PLACEHOLDER.to_owned()),
                tid: entry.tid,
                kernel_us: times.kernel_us(),
                user_us: times.user_us(),
            }),
            Err(_) => unreadable_tids.push(entry.tid),
        }
    }
    samples.sort_by_key(|s| s.tid);
    unreadable_tids.sort_unstable();

    let mut folded: Vec<FoldedSample> = reading
        .folded
        .into_iter()
        .map(|f| FoldedSample {
            family: f.family,
            kernel_us: f.cpu.kernel_us(),
            user_us: f.cpu.user_us(),
            exited: f.exited,
        })
        .collect();
    folded.sort_by(|a, b| a.family.cmp(&b.family));

    let process = ProcessSample {
        wall_ms: elapsed.as_millis() as u64,
        kernel_us: process.kernel_us(),
        user_us: process.user_us(),
        threads: live_count,
    };
    let t_s = elapsed.as_secs();

    let mut lines = Vec::with_capacity(samples.len() + folded.len() + 2);
    lines.push(format_process_line(snap, t_s, &process));
    lines.extend(samples.iter().map(|s| format_thread_line(snap, t_s, s)));
    lines.extend(folded.iter().map(|f| format_folded_line(snap, t_s, f)));
    lines.push(format_thread_line(
        snap,
        t_s,
        &compute_unregistered_rest(&process, &samples, &folded),
    ));
    ComposedSnapshot {
        lines,
        unreadable_tids,
    }
}

/// 環境変数の値から周期を決める（純関数）。
///
/// 未設定・読めない値・0 秒はいずれも既定 [`DEFAULT_PERIOD_SEC`] へ倒す。読めない値は
/// 黙って捨てず `warn!` を残す（指定したつもりが効いていない、を見えるようにする）。
pub fn period_from_env_value(value: Option<&str>) -> Duration {
    let default = Duration::from_secs(DEFAULT_PERIOD_SEC);
    let Some(raw) = value else {
        return default;
    };
    match raw.trim().parse::<u64>() {
        Ok(sec) if sec > 0 => Duration::from_secs(sec),
        _ => {
            warn!(
                target: PERF_TARGET,
                env = PERIOD_ENV,
                value = %raw,
                "[perf_thread_report] 周期として読めない値なので既定へ倒す（正の整数の秒数を指定する）"
            );
            default
        }
    }
}

// ---------------------------------------------------------------------------
// 報告スレッド
// ---------------------------------------------------------------------------

/// 報告スレッドの取っ手。終了直前に [`stop_and_report_final`](Self::stop_and_report_final)
/// を呼ぶと、最後のスナップショットを 1 つ出してからスレッドが終わる。
#[derive(Debug)]
pub struct ReportHandle {
    /// 停止の合図を送る端。落とすだけでも報告スレッドは終わる（切断で気づく）。
    stop: Sender<()>,
    /// 最後のスナップショットを出し終えた合図を受ける端。
    done: Receiver<()>,
    /// 報告スレッドの JoinHandle。
    join: JoinHandle<()>,
}

impl ReportHandle {
    /// 停止を合図し、最後のスナップショットが出るのを待って畳む。
    ///
    /// 待ちは [`FINAL_WAIT`] で頭打ちにする——報告器の不調で終了経路を止めない。
    /// 待ちきれなかったときは `warn!` を残してスレッドを切り離す（プロセスは直後に
    /// 終わるので、置き去りのスレッドは OS が回収する）。
    pub fn stop_and_report_final(self) {
        // 送れないのは報告スレッドが先に終わっていたときだけで、そのときは下の待ちがすぐ切断で返る。
        let _ = self.stop.send(());
        match self.done.recv_timeout(FINAL_WAIT) {
            // 送出端が落ちた＝報告スレッドは終わっている（panic を含む）ので、畳んで結果を見る。
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                if self.join.join().is_err() {
                    error!(
                        target: PERF_TARGET,
                        "[perf_thread_report] 報告スレッドが panic して終わった"
                    );
                }
            }
            Err(RecvTimeoutError::Timeout) => warn!(
                target: PERF_TARGET,
                wait_ms = FINAL_WAIT.as_millis() as u64,
                "[perf_thread_report] 終了直前のスナップショットを待ちきれなかった（切り離して先へ進む）"
            ),
        }
    }
}

/// 報告器を起動する。点灯していなければ**何も起こさず** `None` を返す。
///
/// 呼ぶのは起動のできるだけ早い段階で 1 度きり（`main` のスレッド役割フック導入直後）。
/// スレッドの生成に失敗したときは `error!` を残して `None` を返す——本体は止めない。
pub fn start() -> Option<ReportHandle> {
    if !is_enabled() {
        return None;
    }

    let period = period_from_env_value(read_period_env().as_deref());
    let started = Instant::now();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();

    match thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || run(stop_rx, done_tx, period, started))
    {
        Ok(join) => {
            info!(
                "[perf_thread_report] 起動した period_sec={} target={PERF_TARGET}",
                period.as_secs()
            );
            Some(ReportHandle {
                stop: stop_tx,
                done: done_rx,
                join,
            })
        }
        Err(err) => {
            error!(
                target: PERF_TARGET,
                error = %err,
                "[perf_thread_report] 報告スレッドを起こせなかった（スレッド別 CPU は採れない・本体は継続）"
            );
            None
        }
    }
}

/// 周期の環境変数を読む。非 UTF-8 は黙って捨てず `warn!` を残して未設定として扱う。
fn read_period_env() -> Option<String> {
    match std::env::var(PERIOD_ENV) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            warn!(
                target: PERF_TARGET,
                env = PERIOD_ENV,
                "[perf_thread_report] 周期の指定が UTF-8 として読めない（既定の周期で回す）"
            );
            None
        }
    }
}

/// 報告スレッド本体。周期ごとに 1 枚、停止の合図で最後の 1 枚を出して終わる。
///
/// 待ちは `recv_timeout` で行う（busy-wait しない）ので、停止の合図には即座に応じる。
fn run(stop: Receiver<()>, done: Sender<()>, period: Duration, started: Instant) {
    if let Err(err) = thread_registry::register_current_thread(ROLE_PERF_REPORT) {
        // 自分が名簿に載らないと、自分の CPU は unregistered_rest へ回る。観測は続ける。
        warn!(
            target: PERF_TARGET,
            error = %err,
            "[perf_thread_report] 報告器自身を名簿へ載せられなかった（自分の CPU は unregistered_rest へ回る）"
        );
    }

    let mut snap: u32 = 0;
    loop {
        match stop.recv_timeout(period) {
            Err(RecvTimeoutError::Timeout) => {
                snap += 1;
                take_and_emit(snap, started);
            }
            // 合図（`Ok`）でも取っ手ごと落とされた（`Disconnected`）でも、最後の 1 枚を出す。
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                snap += 1;
                take_and_emit(snap, started);
                let _ = done.send(());
                return;
            }
        }
    }
}

/// スナップショットを 1 枚採って行を出す。
///
/// 名簿を先に、プロセス CPU を後に読む（名簿の値がプロセスの値を追い越さない向きにする）。
/// プロセス CPU が読めない周は `error!` を残してその 1 枚を諦める——名簿外の残りを
/// 算出する基準が無く、出した行が嘘になるためである。CPU を読めなかった生きた項目は
/// 1 枚につき 1 度の `warn!` に並べる（その分の CPU は残りの差へ回るので、合計は保たれる）。
fn take_and_emit(snap: u32, started: Instant) {
    let reading = thread_registry::read_cpu();
    let elapsed = started.elapsed();
    let process_times = match get_process_times() {
        Ok(times) => times,
        Err(err) => {
            error!(
                target: PERF_TARGET,
                snap,
                error = %err,
                "[perf_thread_report] プロセス CPU が読めないのでこのスナップショットを諦める"
            );
            return;
        }
    };

    let composed = compose_snapshot(snap, elapsed, process_times, reading);
    if !composed.unreadable_tids.is_empty() {
        warn!(
            target: PERF_TARGET,
            snap,
            tids = ?composed.unreadable_tids,
            "[perf_thread_report] CPU 時間を読めなかったスレッドがある（その分は unregistered_rest へ回る）"
        );
    }
    for line in &composed.lines {
        emit_line(line);
    }
}

#[cfg(test)]
#[path = "perf_thread_report_tests.rs"]
mod tests;

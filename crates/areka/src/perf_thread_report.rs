//! スレッド別・プロセス全体の CPU 報告器（areka-P0-draw-load-parity task 2.4・要件 2.3/2.6/2.12/3.8）。
//!
//! # 何を出すか
//!
//! `wintf` のスレッド名簿（`ecs::world::thread_registry`）を舐めて、スレッド 1 本につき
//! 1 行の `perf(thread)` と、プロセス全体 1 行の `perf(process)` を出す。名簿に載って
//! いないスレッド（bevy のタスクプール等）は、プロセス CPU から名簿の合計を引いた差を
//! `role=unregistered_rest` の**1 行**として必ず出す——観測が黙って消えない形にするため
//! である（要件 2.3）。
//!
//! # 既定 OFF（費用 0）
//!
//! 点灯の判定 [`is_enabled`] は `tracing::enabled!(target: "areka::perf", DEBUG)` であり、
//! [`start`] が**起動時に 1 度だけ**評価する。消灯していれば報告スレッドを起こさない
//! （`Instant` も Win32 も 1 度も触らない＝要件 3.8）。点灯していれば `areka-perf-report`
//! スレッドが [`DEFAULT_PERIOD_SEC`] 秒ごと（`AREKA_PERF_THREAD_REPORT_SEC` で変更可）と
//! **終了直前**にスナップショットを出す。費用は ON でも 1 周期に 1 回・スレッド数 ×数 µs。
//!
//! # 行は累積値である
//!
//! `GetThreadTimes`／`GetProcessTimes` が返すのはプロセス開始からの**累積**であり、行に
//! 出るのもその累積値である。「この区間で誰が重かったか」は読み手が隣り合うスナップ
//! ショットの差を取って得る（`snap=` で対にする）。差の取り方は [`delta`] が権威で、
//! 順位表を作る `tools/perf/perf-rank.py`（task 6.2）は同じ規則を写す。
//!
//! # 壁時計と CPU 時間は混ぜない（要件 2.6）
//!
//! `perf(process)` の `wall_ms` だけが壁時計（経過時間）で、`cpu_us`／`kernel_us`／
//! `user_us` はすべて CPU 時間である。GPU 待ちなどの待ち時間は前者にだけ現れる。
//! `perf(thread)` は壁時計を持たない。
//!
//! # 行の語彙（要件 2.12）
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
    self, ROLE_PERF_REPORT, ROLE_UNREGISTERED_REST, get_process_times,
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

/// プロセス全体の標本。壁時計はここにだけ在る（要件 2.6）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessSample {
    /// 報告器の起動からの経過（ms・**壁時計**）。
    pub wall_ms: u64,
    /// プロセス全体のカーネルモード CPU 時間（µs・累積）。
    pub kernel_us: u64,
    /// プロセス全体のユーザーモード CPU 時間（µs・累積）。
    pub user_us: u64,
    /// このスナップショットで名簿から読めたスレッドの本数
    /// （名簿外の残りは含まない＝`perf(thread)` の行数 − 1 に等しい）。
    pub threads: usize,
}

impl ProcessSample {
    /// カーネルとユーザーの合計（µs）。
    pub const fn cpu_us(&self) -> u64 {
        self.kernel_us + self.user_us
    }
}

// ---------------------------------------------------------------------------
// 純関数（行の組み立て・差分・名簿外の残り・周期）
// ---------------------------------------------------------------------------

/// 値に含まれる空白を `_` へ潰す（空なら [`PLACEHOLDER`]）。
///
/// 判定スクリプトの `parse_fields` は「行頭または空白の直後の `名前=`」だけを
/// フィールドの始まりと読む。値の中に空白が残っていると、その先の `語=` が新しい
/// フィールドとして立ち上がり、1 行の語彙が壊れる（要件 2.12）。空白さえ無ければ
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
/// 値は**累積**であり、区間の量ではない（[`delta`] を参照）。
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

/// プロセス全体を 1 行へ組む（純関数）。
///
/// `wall_ms` だけが壁時計で、残る 3 つは CPU 時間である（要件 2.6）。
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

/// 名簿外の残りを 1 行ぶんの標本として算出する（プロセス CPU − 名簿の合計）。
///
/// 名簿に載らないスレッド（bevy のタスクプール等）は必ず残るので、その分を黙って
/// 落とさずここで拾う（要件 2.3）。終了済みのスレッドも名簿に残り、その最終値は
/// プロセス CPU に含まれているので、引く側からも外さない。
///
/// 読み取り時刻がずれてプロセスより名簿の合計が大きく見えることがある（スナップ
/// ショットは瞬間ではない）。そのときは 0 で止める——負へ回さない。
pub fn compute_unregistered_rest(
    process: &ProcessSample,
    threads: &[ThreadSample],
) -> ThreadSample {
    let kernel_sum: u64 = threads.iter().map(|t| t.kernel_us).sum();
    let user_sum: u64 = threads.iter().map(|t| t.user_us).sum();
    ThreadSample {
        role: ROLE_UNREGISTERED_REST.to_owned(),
        name: PLACEHOLDER.to_owned(),
        tid: 0,
        kernel_us: process.kernel_us.saturating_sub(kernel_sum),
        user_us: process.user_us.saturating_sub(user_sum),
    }
}

/// 2 つのスナップショット（いずれも累積値）の差を取る。
///
/// 突き合わせは TID で行い、前回に無い TID は値をそのまま持つ（その区間で生まれた
/// スレッド）。前回に在って今回に無い TID は結果に現れない。累積が巻き戻って見えた
/// 場合（TID の再利用など）は 0 で止める。
///
/// 順位表を作る `tools/perf/perf-rank.py` は同じ規則を写すので、意味論はここの
/// 決定論テストで固定しておく。
///
/// 本番経路からは呼ばない（行に出るのは累積値であり、差を取るのは読み手＝解析道具の
/// 仕事である）。それでも Rust 側に置くのは、**規則の権威を 1 つにする**ため——
/// perf-rank.py の実装がここの決定論テストと食い違えば、順位表が静かに嘘をつく。
/// 消費点が本番に無いので `#[allow(dead_code)]` を対で置く（棚卸で見える形）。
#[allow(dead_code)]
pub fn delta(prev: &[ThreadSample], cur: &[ThreadSample]) -> Vec<ThreadSample> {
    cur.iter()
        .map(|c| {
            let before = prev.iter().find(|p| p.tid == c.tid);
            let (k0, u0) = before.map_or((0, 0), |p| (p.kernel_us, p.user_us));
            ThreadSample {
                role: c.role.clone(),
                name: c.name.clone(),
                tid: c.tid,
                kernel_us: c.kernel_us.saturating_sub(k0),
                user_us: c.user_us.saturating_sub(u0),
            }
        })
        .collect()
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
        if self.stop.send(()).is_err() {
            // 受け手が既に居ない＝報告スレッドは終わっている。最後の 1 枚は出ない。
            warn!(
                target: PERF_TARGET,
                "[perf_thread_report] 報告スレッドは既に終端していた（終了直前のスナップショットは出ない）"
            );
            return;
        }
        match self.done.recv_timeout(FINAL_WAIT) {
            Ok(()) => {
                if self.join.join().is_err() {
                    error!(
                        target: PERF_TARGET,
                        "[perf_thread_report] 報告スレッドが panic して終わった"
                    );
                }
            }
            Err(_) => warn!(
                target: PERF_TARGET,
                wait_ms = FINAL_WAIT.as_millis() as u64,
                "[perf_thread_report] 終了直前のスナップショットを待ちきれなかった（切り離して先へ進む）"
            ),
        }
    }
}

/// 報告器を起動する。点灯していなければ**何も起こさず** `None` を返す（要件 3.8）。
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
/// 出す順は「プロセス 1 行 → 名簿のスレッド（TID 昇順）→ 名簿外の残り 1 行」で固定する
/// （読み手が 1 枚の切れ目を `unregistered_rest` で見分けられる）。
///
/// プロセス CPU が読めない周は `error!` を残してその 1 枚を諦める——名簿外の残りを
/// 算出する基準が無く、出した行が嘘になるためである。個々のスレッドが読めない場合は
/// その項目だけ飛ばし、飛ばした TID を 1 枚につき 1 度の `warn!` に並べる（飛ばした分の
/// CPU は残りの差へ回るので、合計は保たれる）。
fn take_and_emit(snap: u32, started: Instant) {
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

    let mut samples = Vec::new();
    let mut unreadable: Vec<u32> = Vec::new();
    for entry in thread_registry::snapshot() {
        match entry.cpu_times() {
            Ok(times) => samples.push(ThreadSample {
                role: entry.role,
                name: entry.name.unwrap_or_else(|| PLACEHOLDER.to_owned()),
                tid: entry.tid,
                kernel_us: times.kernel_us(),
                user_us: times.user_us(),
            }),
            Err(_) => unreadable.push(entry.tid),
        }
    }
    if !unreadable.is_empty() {
        warn!(
            target: PERF_TARGET,
            snap,
            tids = ?unreadable,
            "[perf_thread_report] CPU 時間を読めなかったスレッドがある（その分は unregistered_rest へ回る）"
        );
    }
    samples.sort_by_key(|s| s.tid);

    let process = ProcessSample {
        wall_ms: elapsed.as_millis() as u64,
        kernel_us: process_times.kernel_us(),
        user_us: process_times.user_us(),
        threads: samples.len(),
    };
    let t_s = elapsed.as_secs();

    emit_line(&format_process_line(snap, t_s, &process));
    for sample in &samples {
        emit_line(&format_thread_line(snap, t_s, sample));
    }
    emit_line(&format_thread_line(
        snap,
        t_s,
        &compute_unregistered_rest(&process, &samples),
    ));
}

#[cfg(test)]
#[path = "perf_thread_report_tests.rs"]
mod tests;

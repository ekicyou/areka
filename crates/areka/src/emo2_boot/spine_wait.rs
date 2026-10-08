//! spine の有界待機の部品（締切・空回しの窓・休み・待ちの芯・`spin_wait_until`・`run_bounded`・`join_bounded`）。
//!
//! 締切から下は `spine.rs` から移したもの。親の `spine.rs` が同じ名前で出し直すので、
//! 兄弟のテストは `super::SPIN_WAIT` などの呼び名のまま使える。古い呼び名（`spin_wait_until`・
//! `run_bounded`・`join_bounded`）は形を保ったまま芯の上に載せ、打ち切りの文言を出すようにした。
//!
//! 待ちの芯 [`wait_until_with`] は、打ち切りを「待ち始めからの総時間」でなく「相手が状態を進めなかった
//! 時間」で決め、届かなかった理由を [`WaitFailure`] の 4 つに分けて返す（areka-P0-ghost-session-test-load-flake）。

use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::sync::{Condvar, LazyLock, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use areka_actor::{ActorError, ActorHandle};

/// **別スレッドの進行を待つ**有界待ちの基準の時間（30 秒）。
///
/// 進みの目印がある待ち（[`Progress::Count`]）では「相手が状態を進めなかった時間」の上限、目印の
/// 無い待ち（[`Progress::Unknown`]・[`spin_wait_until`]）では待ち始めからの総時間の上限として使う。
/// 目印がある待ちの総時間の上限は別に [`WAIT_CAP`] で置く。
///
/// # 反復回数で打ち切ってはならない
///
/// `yield_now()` のビジーウェイトでは **反復回数が経過時間の代理にならない**。CPU 競合下
/// （`cargo test --workspace` の並行実行・ウイルス対策の再スキャン等）では、待っている相手スレッドが
/// 一度も走らないまま数十万回の yield が尽きうる（steering
/// `areka-defender-rescan-starves-cooperative-test-loops`）。実測では旧 `for _ in 0..100_000u32` 形が
/// 並行実行時に**約 6%**（単独実行 30 回中 2 回）で待機に失敗し、`boot_calls` が空のまま照合へ落ちていた。
///
/// # 適用範囲
///
/// 分類の軸は「Tick を注入するか」ではなく **「別スレッドの進行を待っているか」** である。
///
/// - **対象（[`spin_wait_until`] を使う）**: 待機対象が別スレッドの進行であり、各反復が**何も進めない**
///   純粋なポーリング（`non_status_calls()` / `drain_received()` を読むだけのループ）。
/// - **対象（本猶予の期限だけを借りる。ただし期限は十分条件ではない）**: 各反復で
///   `inject_dispatcher_tick` により**系を進めつつ**、同じ反復で `drain_received()` 等により
///   **別スレッドの結果も待つ**ハイブリッドのループ（[`spin_wait_until`] は純粋ポーリング専用ゆえ
///   流用しない）。ここでは打ち切りを時刻期限にするだけでは足りず、**注入する simulated time が
///   待っている観測を追い越さない**ことを構造で保証しなければならない。追い越しうる時刻には必ず
///   上限（頭打ち）を置くこと。実測: S2 Phase 1 は毎反復 `now += 5` が 210 反復（実時間 ~0.6 秒）で
///   Clear cue の時刻を跨ぎ、リビール観測が間に合わないと**待っている条件そのものが破壊**されて
///   永久に不成立になる——並行実行時に約 2% 失敗し、期限を 30 秒に延ばしても 50 回中 3 回失敗した。
///   期限は「壊れていない条件を待つ」ためのものであり、条件が壊れるレースは期限では直らない。
///
///   **例外: 切替の足場（`ghost_switch_test_support.rs` の `SwitchRig`）の台詞の時計には頭打ちを置かない。**
///   足場が注入する時刻は `DispatcherMsg::Tick` だけで、受け手は dispatcher とその先の再生中の台詞に
///   限られる。台詞は「自分が最初に見た Tick からの経過」を届いた順に消化するので、注入の側の数が
///   先へ行っても増えるのは順番待ちの列だけで、相手が見る時刻の並びは飢えていないときと変わらない。
///   合成の締切を持つ kanade は足場では Tick を 1 つも受け取らない（`TickerMode::Disabled`）。受け手が
///   締切も後戻りする状態も持たないので、追い越しで壊れる条件が無い（要件 2.6・設計「2.6 の満たし方」）。
///   上の spine の族の Tick 注入の待ちは受け手が kanade なので、頭打ちの決まりはそのまま残る。
/// - **非対象**: 別スレッドの進行を待たず、注入 Tick 列そのものが仕事量であるループ。時刻で打ち切ると
///   注入列が短くなり意味が変わる。
///
/// 猶予は通常経路（マイクロ秒〜ミリ秒）に対して桁違いに大きく取る。期限切れは呼び手の assert が
/// 落として原因を名指しするので hang しない。
pub(super) const SPIN_WAIT: Duration = Duration::from_secs(30);

/// 進みの目印がある待ちの、待ち始めからの総時間の上限。
///
/// 相手が進み続ける限り [`SPIN_WAIT`] では打ち切らないので、終わらない繰り返しでも必ず返るように
/// 別の上限を置く（要件 3.3）。届いたら [`WaitFailure::CapReached`]（負荷で遅いか、終わらない繰り返し）。
pub(super) const WAIT_CAP: Duration = Duration::from_secs(300);

/// `yield_now()` の空回しで待つ窓（待ち始めからの時間）。過ぎたら反復ごとに [`BACKOFF_SLEEP`] 休む。
///
/// # なぜ純 yield のままではいけないか
/// `yield_now()` の密ループは **1 コアを占有し続ける**。反復上限だけで打ち切っていた旧実装は
/// 早々に諦めるためこれが顕在化しなかったが、時刻期限（[`SPIN_WAIT`]）へ変えると失敗経路が
/// 数十秒フルにコアを焼き、**同一バイナリで並走する他テストを飢餓させて別の flake を生む**
/// （実測: 純 yield ＋ 30 秒期限で 50 回中 5 回・無関係な 3 テストが巻き添えで失敗し、総所要が
/// 230 秒→1490 秒へ悪化した）。待機は「速い経路を邪魔しない」と同時に「長引いたら CPU を返す」
/// 必要がある。
///
/// # 回数でなく時間で区切る理由
/// 旧形は空回しの予算を回数（1_000_000 回）で持っていた。条件が読むだけの速い待ちなら 1 回は
/// 一瞬だが、切替の足場の待ち（条件の中で毎回 ECS の段を回す）は 1 回が重く、100 万回を使い切る前に
/// 待ちが終わるか期限に届く＝**待っている間ずっと 1 コアを使っていた**。回数は経過時間の代理に
/// ならない（[`SPIN_WAIT`] の doc と同じ理由）ので、窓も時間で持つ。
///
/// # 60 ms の根拠
/// `spine.rs` の `SETTLE_MIN` の doc の実測（本機 22 論理 CPU）で、`yield_now` 5,000 回は無負荷
/// 0.31 ms。旧予算 1_000_000 回を同じ割合で時間に直すと約 62 ms になる。60 ms なら、速い待ちの
/// 空回しは旧形とほぼ同じ長さのまま（**成功する待機を旧形より遅くしない**）、重い待ちだけが 60 ms で
/// CPU を返すようになる。予算を小さく取ると、正常でも数百 ms 待つ呼出点が [`BACKOFF_SLEEP`] の粒度に
/// 律速されて通常経路が延びる（実測: 回数の予算 10_000 で 1 回 4.4 秒 → 10.3 秒）ので、旧予算に揃える。
///
/// 足場の待ちの 1 回の実測（2026-10-08・本機 22 論理 CPU・`cargo test` の debug ビルド・机の CPU 4〜29%・
/// 定常のゴースト 1 体・時計を止めて休みを抜いた 3,000 回の平均を 3 巡）: `pump_until` の 1 回（通知の相＋
/// 切替要求の取り出し）は 0.96〜1.2 µs、`pump_input_until`・`pump_talking_until` の 1 回（通知の相＋
/// `Input` の段）は 62〜93 µs。同じ机で 60 ms の空回しは `yield_now` 約 54〜57 万回。旧予算 1_000_000 回は
/// `Input` の段を回す待ちでは 60〜90 秒ぶんに当たり、30 秒の期限まで 1 コアを使い続けていた。60 ms なら
/// その待ちは約 650〜970 回の反復で空回しを終え、以後は反復ごとに CPU を返す。
pub(super) const DENSE_SPIN: Duration = Duration::from_millis(60);

/// 空回しの窓を過ぎた後の 1 回あたり待機。CPU を明け渡し、相手スレッドに実行機会を与える。
pub(super) const BACKOFF_SLEEP: Duration = Duration::from_millis(1);

/// 相手が状態を進めたかの数え方。
pub(crate) enum Progress<'a> {
    /// 目印を持てない待ち。総時間 [`SPIN_WAIT`] で打ち切り、失敗の文言は「進みは不明」と書く。
    Unknown,
    /// 単調に増える数。前回の読みより増えていれば「進んだ」。減らないことは呼び手が守る。
    Count(&'a dyn Fn() -> u64),
}

/// 待ちが届かずに終わった理由。`Display` の先頭の `［…］` で 4 つを見分ける（要件 3.1・3.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WaitFailure {
    /// 相手が `idle` のあいだ 1 度も進まなかった＝止まった。
    Stalled {
        what: String,
        waited: Duration,
        idle: Duration,
        /// 目印が増えた量の合計（目印の 1 つが呼び出し 1 回に当たる。読んだ回数ではない）。
        moves: u64,
        /// 最後の 1 回の確かめ（前の時計の読みから今の読みまで）にかかった時間。打ち切りは確かめと
        /// 確かめの間でしか判じられないので、1 回が同期で長く塞がると打ち切りもその分遅れる。
        last_step: Duration,
    },
    /// 相手は進み続けていたが、総時間の上限 [`WAIT_CAP`] に届いた。
    CapReached {
        what: String,
        waited: Duration,
        /// [`WaitFailure::Stalled`] の `moves` と同じ数え方。
        moves: u64,
        since_last_move: Duration,
        /// [`WaitFailure::Stalled`] の `last_step` と同じ。
        last_step: Duration,
    },
    /// 進みの目印が無い待ちが、総時間の上限 [`SPIN_WAIT`] に届いた。
    TimedOut { what: String, waited: Duration },
    /// 受け口の相手が、何も送らずに居なくなった。
    Disconnected { what: String, waited: Duration },
}

impl fmt::Display for WaitFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = |d: &Duration| d.as_secs_f64();
        match self {
            Self::Stalled {
                what,
                waited,
                idle,
                moves,
                last_step,
            } => write!(
                f,
                "待ちの打ち切り［止まった］: 「{what}」— 相手が {:.1} 秒のあいだ状態を進めなかった（待ち始めから {:.1} 秒・それまでの進み {moves} 回・最後の 1 回の確かめに {:.1} 秒）",
                s(idle),
                s(waited),
                s(last_step),
            ),
            Self::CapReached {
                what,
                waited,
                moves,
                since_last_move,
                last_step,
            } => write!(
                f,
                "待ちの打ち切り［進んではいた］: 「{what}」— 上限 {} 秒までに届かなかった（待ち始めから {:.1} 秒・進み {moves} 回・最後の進みは {:.1} 秒前・最後の 1 回の確かめに {:.1} 秒）。負荷で遅いか、終わらない繰り返し",
                WAIT_CAP.as_secs(),
                s(waited),
                s(since_last_move),
                s(last_step),
            ),
            Self::TimedOut { what, waited } => write!(
                f,
                "待ちの打ち切り［進みは不明］: 「{what}」— {:.1} 秒までに届かなかった（進みの目印の無い待ち）",
                s(waited),
            ),
            Self::Disconnected { what, waited } => write!(
                f,
                "待ちの打ち切り［相手が居ない］: 「{what}」— {:.1} 秒待ったところで、相手が何も送らずに終わった",
                s(waited),
            ),
        }
    }
}

/// 待ちの芯。`cond` が真になるまで待ち、届かなければ理由を [`WaitFailure`] で返す。
///
/// 時計 `now` と休み `pause` を差し替えられる継ぎ目で、檻（`spine_wait_tests.rs`・足場の檻）は偽の時計を
/// 渡して実時間を待たずに打ち切りを確かめる（`settle_bounded_with` と同じ型）。
///
/// 決め方（設計の流れ図）:
/// - 条件の確かめが打ち切りの判定より先。届いていれば、どれだけ時間がかかっていても成功。
///   最初の確かめで真なら時計を読まずに返る。
/// - 目印は条件の確かめの直後に読む。前回より増えていれば進みの無い時間を 0 に戻す。
///   増えないまま [`SPIN_WAIT`] に届けば `Stalled`、待ち始めから [`WAIT_CAP`] に届けば `CapReached`。
/// - 目印なしは待ち始めから [`SPIN_WAIT`] に届けば `TimedOut`（旧 `spin_wait_until` と同じ総時間）。
/// - 反復の間は、待ち始めから [`DENSE_SPIN`] までは `yield_now`、その後は毎回 `pause(BACKOFF_SLEEP)`
///   で CPU を返す（要件 2.5）。この休みは時刻を進めるためのものではなく、観測の内容を変えない。
pub(crate) fn wait_until_with(
    mut now: impl FnMut() -> Instant,
    mut pause: impl FnMut(Duration),
    what: &str,
    progress: Progress<'_>,
    mut cond: impl FnMut() -> bool,
) -> Result<(), WaitFailure> {
    if cond() {
        return Ok(());
    }
    let read = || match progress {
        Progress::Unknown => 0,
        Progress::Count(count) => count(),
    };
    let started = now();
    let mut last = read();
    let mut last_move = started;
    let mut moves = 0u64;
    let mut t = started;
    loop {
        if t.saturating_duration_since(started) < DENSE_SPIN {
            std::thread::yield_now();
        } else {
            pause(BACKOFF_SLEEP);
        }
        if cond() {
            return Ok(());
        }
        let before = t;
        t = now();
        let last_step = t.saturating_duration_since(before);
        let waited = t.saturating_duration_since(started);
        let what = || what.to_owned();
        if let Progress::Unknown = progress {
            if waited >= SPIN_WAIT {
                return Err(WaitFailure::TimedOut {
                    what: what(),
                    waited,
                });
            }
            continue;
        }
        let count = read();
        if count > last {
            moves += count - last;
            last = count;
            last_move = t;
        }
        let idle = t.saturating_duration_since(last_move);
        if idle >= SPIN_WAIT {
            return Err(WaitFailure::Stalled {
                what: what(),
                waited,
                idle,
                moves,
                last_step,
            });
        }
        if waited >= WAIT_CAP {
            return Err(WaitFailure::CapReached {
                what: what(),
                waited,
                moves,
                since_last_move: idle,
                last_step,
            });
        }
    }
}

/// 本物の時計で待つ [`wait_until_with`]。休みは本物の `sleep`。
pub(crate) fn wait_until(
    what: &str,
    progress: Progress<'_>,
    cond: impl FnMut() -> bool,
) -> Result<(), WaitFailure> {
    wait_until_with(Instant::now, std::thread::sleep, what, progress, cond)
}

/// 受け口に 1 件届くまで待ち、届いた値を返す。打ち切りの決め方は [`wait_until`] と同じ。
///
/// 空回しはしない。1 回ごとに `recv_timeout` で [`BACKOFF_SLEEP`] だけ眠り、届けばすぐ起きる。
/// 区切って起きるのは進みの目印を読んで打ち切りを判じるため（要件 2.5）。送り手が何も送らずに
/// 居なくなれば、待たずに [`WaitFailure::Disconnected`]。
pub(crate) fn wait_recv<T>(
    what: &str,
    progress: Progress<'_>,
    rx: &mpsc::Receiver<T>,
) -> Result<T, WaitFailure> {
    let started = Instant::now();
    let mut got = None;
    // 休みは受け口の `recv_timeout` が受け持つので、芯の休みは空にする。
    wait_until_with(
        Instant::now,
        |_| {},
        what,
        progress,
        || {
            match rx.recv_timeout(BACKOFF_SLEEP) {
                Ok(value) => got = Some(Ok(value)),
                Err(mpsc::RecvTimeoutError::Disconnected) => got = Some(Err(())),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            got.is_some()
        },
    )?;
    match got {
        Some(Ok(value)) => Ok(value),
        _ => Err(WaitFailure::Disconnected {
            what: what.to_owned(),
            waited: started.elapsed(),
        }),
    }
}

/// クロージャ `f` を別スレッドで走らせ、進みを見ながら終わるのを [`wait_recv`] で待つ。
/// 届かなければ [`WaitFailure`] の文言で panic する（`f` が panic して居なくなれば `［相手が居ない］`）。
pub(crate) fn run_bounded_watching<F: FnOnce() + Send + 'static>(
    what: &str,
    progress: Progress<'_>,
    f: F,
) {
    let (done_tx, done_rx) = mpsc::sync_channel::<()>(1);
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    if let Err(failure) = wait_recv(what, progress, &done_rx) {
        panic!("{failure}");
    }
}

/// `cond` が真になるまで待つ古い呼び名。目印なしで芯 [`wait_until`] を呼ぶ（総時間 [`SPIN_WAIT`]）。
///
/// 真になったら `true`。打ち切ったら、呼び出しの場所（ファイルと行）を「何を」にした文言を
/// 標準エラーへ 1 行出して `false`（黙って `false` にしない・要件 3.1）。
/// 本ファイルの「sleep 不使用」規律は *系を進める* Tick 注入ループの決定論を守るためのものであり、
/// 別スレッドの進行を待つだけの本ヘルパには当たらない（待機は観測内容を変えない）。
#[track_caller]
pub(crate) fn spin_wait_until(cond: impl FnMut() -> bool) -> bool {
    let at = std::panic::Location::caller();
    match wait_until(
        &format!("{}:{}", at.file(), at.line()),
        Progress::Unknown,
        cond,
    ) {
        Ok(()) => true,
        Err(failure) => {
            eprintln!("{failure}");
            false
        }
    }
}

/// 渡された総時間 `timeout` の待ちが届かなかった理由（古い呼び名の panic の文の後ろに足す）。
fn bounded_failure(what: &str, started: Instant, e: mpsc::RecvTimeoutError) -> WaitFailure {
    let (what, waited) = (what.to_owned(), started.elapsed());
    match e {
        mpsc::RecvTimeoutError::Timeout => WaitFailure::TimedOut { what, waited },
        mpsc::RecvTimeoutError::Disconnected => WaitFailure::Disconnected { what, waited },
    }
}

/// クロージャ `f` を別スレッドで実行し有界時間で完了を観測する（ghost spine の `run_bounded` 同旨）。
///
/// 打ち切りは今どおり渡された総時間。panic の文は今の文を先頭に保ち、後ろに [`WaitFailure`] の文言
/// （届かない＝`［進みは不明］`・`f` が panic して居なくなった＝`［相手が居ない］`）を足す。
pub(crate) fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    let (done_tx, done_rx) = mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    let started = Instant::now();
    if let Err(e) = done_rx.recv_timeout(timeout) {
        panic!(
            "'{what}' did not complete within {timeout:?} (possible hang) — {}",
            bounded_failure(what, started, e)
        );
    }
}

/// `ActorHandle::join` を有界時間で観測する（ghost spine の `join_bounded` 同旨）。
/// 打ち切りと panic の文の形は [`run_bounded`] と同じ。
pub(super) fn join_bounded(
    what: &str,
    timeout: Duration,
    handle: ActorHandle,
) -> Result<(), ActorError> {
    let (res_tx, res_rx) = mpsc::sync_channel::<Result<(), ActorError>>(0);
    std::thread::spawn(move || {
        let _ = res_tx.send(handle.join());
    });
    let started = Instant::now();
    match res_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(e) => panic!(
            "'{what}' join did not complete within {timeout:?} (possible hang) — {}",
            bounded_failure(what, started, e)
        ),
    }
}

/// 同時に持てる数を決めた数え（areka-P0-ghost-session-test-load-flake 要件 2.7・設計「足場のスレッドの絞り」）。
/// GPU の装置の許可（[`GpuPermit`]）と足場の許可（[`RigPermit`]）が同じ形で使う。
///
/// D3D11 の装置は装置ごとにドライバのスレッドを生み、ゴーストを起こす足場はアクターのスレッドを生む。
/// Windows ではスレッドの始まりと終わりがプロセスに 1 つのローダーの錠を取るので、負荷の下でこれらが
/// 多く並ぶと、起こしたばかりのゴーストのスレッドが始まれずに待ちの打ち切りの赤になった
/// （`load-repro.md` の 4.2・4.3）。数えは `std` だけで書き、どの足場からも届くここに置く。
pub(super) struct Slots {
    held: Mutex<usize>,
    freed: Condvar,
    cap: usize,
}

/// GPU の装置のプロセスに 1 つの数え（同時に 4 つ）。2 つでは負荷の下の赤が 0 件でも静かな机の `--bin areka` の
/// 全部が中央値 95.2 秒に延び（直す前 64.1 秒）、8 つでは 64.7 秒でも負荷の下に赤が 2 件残った。間の 4 つを、
/// 所要時間の線（直す前の 1.20 倍）とともに選んだ（`load-repro.md` の 5）。
static GPU_SLOTS: Slots = Slots::new(4);

/// 足場（`SwitchRig`）のプロセスに 1 つの数え（同時の数は [`rig_cap`]）。
static RIG_SLOTS: LazyLock<Slots> = LazyLock::new(|| {
    Slots::new(rig_cap(
        std::thread::available_parallelism().map_or(1, usize::from),
    ))
});

/// 足場の同時の数: 論理 CPU の数の半分（最小 1）。
pub(super) fn rig_cap(parallelism: usize) -> usize {
    (parallelism / 2).max(1)
}

impl Slots {
    pub(super) const fn new(cap: usize) -> Self {
        Self {
            held: Mutex::new(0),
            freed: Condvar::new(),
            cap,
        }
    }

    /// 空きが出るまで眠って待ち、許可を取る。
    fn take(&'static self) -> SlotPermit {
        let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let mut held = self
            .freed
            .wait_while(held, |held| *held >= self.cap)
            .unwrap_or_else(PoisonError::into_inner);
        *held += 1;
        SlotPermit(self)
    }

    /// 待たずに取る（空きが無ければ `None`）。檻 16・18 が実時間を待たずに数えを確かめるための口。
    pub(super) fn try_take(&'static self) -> Option<SlotPermit> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if *held >= self.cap {
            return None;
        }
        *held += 1;
        Some(SlotPermit(self))
    }
}

/// 数えから取った 1 つ。捨てると数えへ返り、待っている足場を 1 つ起こす。
pub(super) struct SlotPermit(&'static Slots);

impl Drop for SlotPermit {
    fn drop(&mut self) {
        *self.0.held.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
        self.0.freed.notify_one();
    }
}

/// GPU の装置を持ってよい許可。
///
/// `GraphicsCore::new()` の直前に [`GpuPermit::take`] で取る（待ちの関数の外・装置を作る前なので、
/// 許可を待つ時間は待ちの時間に数えない）。装置より後に捨てるよう、足場の構造体では**最後の欄**に、
/// 組で返すときは**先頭**に置く（構造体の欄は宣言の順に、`let` の組の束縛は後のものから捨てられる）。
/// 1 つのテストで 2 つ取ると、同じことをするテストが並んだとき互いに待ち合って止まりうる。2 つ取る
/// テストは `new_ghost_holdings_start_with_a_fresh_ledger_after_a_switch` の 1 本だけにしておくこと。
/// 足場の許可（[`RigPermit`]）と両方を持つ足場（`GpuRig`・`LapRig`）は、足場の許可を先に取る
/// （取る順を 1 つに決めて、互いに相手の許可を待ち合う形を作らない）。
pub(crate) struct GpuPermit {
    _slot: SlotPermit,
}

impl GpuPermit {
    /// プロセスに 1 つの数えから許可を取る（空きが出るまで待つ）。
    pub(crate) fn take() -> Self {
        Self {
            _slot: GPU_SLOTS.take(),
        }
    }
}

thread_local! {
    /// このスレッドが足場の許可を持っているか。
    static HOLDS_RIG: Cell<bool> = const { Cell::new(false) };
}

/// 足場（`SwitchRig`）を持ってよい許可（areka-P0-ghost-session-test-load-flake タスク 5.2・設計「足場の
/// スレッドの絞り」の 3）。
///
/// `SwitchRig::new` の最初に [`RigPermit::take`] で取り（スレッドを持つものを作る前・待ちの関数の外なので、
/// 許可を待つ時間は待ちの時間に数えない）、`SwitchRig` の最後の欄として持って破棄で返す。GPU の装置の
/// 許可より先に取る（[`GpuPermit`]）。1 つのスレッドで 2 つ目を取ろうとすると panic する: 足場を持った
/// まま 2 つ目を待つテストが同時の数だけ並ぶと、互いに待ち合って全体が止まる（同時の数が 1 の机では
/// 1 本で止まる）。2 つ目の足場を作る前に 1 つ目を捨てること。
pub(crate) struct RigPermit {
    _slot: SlotPermit,
    /// スレッドの印を戻すのは取ったスレッドなので、ほかのスレッドへ渡さない（`!Send`）。
    _not_send: PhantomData<*const ()>,
}

impl RigPermit {
    /// プロセスに 1 つの数えから許可を取る（空きが出るまで待つ）。このスレッドが既に持っていれば panic。
    pub(crate) fn take() -> Self {
        Self::take_from(&RIG_SLOTS)
    }

    /// `slots` から取る（檻 18 は手元の数えで、本物と同じスレッドの印の確かめを通す）。
    pub(super) fn take_from(slots: &'static Slots) -> Self {
        assert!(
            !HOLDS_RIG.get(),
            "足場（SwitchRig）を持ったまま 2 つ目を作ろうとした。同時の数の許可を待ち合って止まりうるので、先の足場を捨ててから作る"
        );
        let slot = slots.take();
        HOLDS_RIG.set(true);
        Self {
            _slot: slot,
            _not_send: PhantomData,
        }
    }
}

impl Drop for RigPermit {
    fn drop(&mut self) {
        HOLDS_RIG.set(false);
    }
}

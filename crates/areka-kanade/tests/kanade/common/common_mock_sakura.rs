//! mock sakura sink 2 種（即応・保留付き）と quit フラグのシナリオ指示。
//!
//! `common/mod.rs`（1,657 行）から責務単位で切り出した子モジュール（タスク 8.2）。
//! 項目は親のファサードから再輸出されるため、消費側の `super::common::X` は不変である。

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use areka_kanade::{KanadeMsg, StartTalk, TalkCommand, TalkDone, TalkEndReason};

use super::bounded::run_join_bounded;

// ============================================================================
// mock sakura sink
// ============================================================================

/// mock sakura sink のハンドル群（join ハンドル・受領 talk 指示のアクセサ）。
///
/// sink は別スレッドで [`TalkCommand`] を読み、**到着順**に記録した上で、`Start` についてのみ
/// シナリオ指示（quit 真偽）に応じた [`KanadeMsg::TalkDone`] を kanade inbox へ返す。sink
/// スレッドは `TalkCommand` の全 Sender drop（＝kanade 停止）で自然終了する。
///
/// # 観測面（design C7 Ordering / delivery・Testing Strategy）
/// - [`commands`](Self::commands): 3 形（`Start`/`ResolveChoice`/`CancelChoice`）の**到着順**の
///   記録列。`TalkCommand` が単一チャンネルを流れることによる FIFO 順序保存（DD-4 の前提）を
///   観測する面である。
/// - [`started`](Self::started): 記録列のうち `TalkCommand::Start` の射影。既存の起動系檻は
///   本アクセサを従来どおり使い続け、意味は不変である。
pub struct MockSakura {
    join: thread::JoinHandle<()>,
    commands: Arc<Mutex<Vec<TalkCommand>>>,
}

impl MockSakura {
    /// 受領した [`TalkCommand`] の記録スナップショットを**到着順**で返す。
    ///
    /// # ⚠️ 並行読みの罠——`ForceQuit`／`Close` で終了する檻では使わないこと
    /// 本メソッドは recv ループと**同期しない**。kanade の終了が mock sakura を経由する檻
    /// （quit フラグ付き `TalkDone` の往復がある形）では、その往復が記録の消費を強制するため
    /// 安全に読める。しかし `KanadeMsg::ForceQuit` や `Close` で終了を駆動する檻では kanade が
    /// mock を経由せず終了でき、**記録前のスナップショットを掴む**。実測で檻バイナリ 100 回中
    /// 7〜11 回失敗し、しかも**全檻並行実行時にしか露見しない**（`--exact` 単独実行では出ない）。
    ///
    /// そういう檻では [`MockSakura::join_bounded_then_commands`] を使うこと——join 完了が
    /// 全記録に happens-before を張る。
    pub fn commands(&self) -> Vec<TalkCommand> {
        self.commands.lock().expect("mock sakura mutex").clone()
    }

    /// 受領した [`StartTalk`] の記録スナップショットを返す（記録列の `Start` 射影）。
    pub fn started(&self) -> Vec<StartTalk> {
        self.commands
            .lock()
            .expect("mock sakura mutex")
            .iter()
            .filter_map(|command| match command {
                TalkCommand::Start(start) => Some(start.clone()),
                TalkCommand::ResolveChoice { .. } | TalkCommand::CancelChoice { .. } => None,
            })
            .collect()
    }

    /// sink スレッドの終了を待つ（`TalkCommand` 送信端が全て drop された後に完了する）。
    pub fn join_bounded(self, what: &str, timeout: Duration) {
        let MockSakura { join, .. } = self;
        run_join_bounded(what, timeout, move || {
            let _ = join.join();
        });
    }

    /// sink スレッドの終了を待ってから、記録スナップショットを**到着順**で返す。
    ///
    /// # なぜ [`commands`](Self::commands) と別に要るのか（読み出しの happens-before）
    /// `commands()` は recv ループと**並行に**ロックを取るだけなので、「kanade が停止した」ことは
    /// 「mock がチャンネルに残った [`TalkCommand`] を取り出して記録し終えた」ことを意味しない。
    /// kanade の終了が `TalkDone` の往復を介さない檻——`ForceQuit` で終了系列へ直行する形——では
    /// この差が実際に露見し、記録列が**空のまま**読まれ得る（実測: 100 回中 11 回）。
    ///
    /// quit フラグ付き `TalkDone` の往復で終わる檻（既存の群 1〜5）は、その往復自体が mock の
    /// 消費を強制するため `commands()` で足りる——本変種は既存檻の意味を変えないよう**追加**であり、
    /// `commands()` は従来どおり使い続けてよい。
    ///
    /// 本変種は recv ループスレッドの `join` 完了後にロックするため、記録の全書き込みに対して
    /// happens-before が張られる（スレッド終了 → join のメモリ順序）。
    pub fn join_bounded_then_commands(self, what: &str, timeout: Duration) -> Vec<TalkCommand> {
        let MockSakura { join, commands } = self;
        run_join_bounded(what, timeout, move || {
            let _ = join.join();
        });
        commands.lock().expect("mock sakura mutex").clone()
    }
}

/// quit フラグの決定方式（シナリオ指示）。
#[derive(Debug, Clone)]
pub enum QuitPolicy {
    /// 受領した全 talk で quit フラグを固定値にする。
    Fixed(bool),
    /// n 番目（0 始まり）の受領 talk の quit フラグを個別指定する（範囲外は false）。
    PerTalk(Vec<bool>),
}

impl QuitPolicy {
    /// n 番目（0 始まり）の talk に対する quit フラグを返す。
    fn quit_for(&self, index: usize) -> bool {
        match self {
            QuitPolicy::Fixed(q) => *q,
            QuitPolicy::PerTalk(flags) => flags.get(index).copied().unwrap_or(false),
        }
    }

    /// n 番目（0 始まり）の talk に対する [`TalkEndReason`] を返す（機械的置換:
    /// quit:true → `Quit`・quit:false → `Ended`）。ハーネスのシナリオ指示は quit 真偽の
    /// ままとし、契約型への変換のみを本メソッドに閉じる。
    fn reason_for(&self, index: usize) -> TalkEndReason {
        if self.quit_for(index) {
            TalkEndReason::Quit
        } else {
            TalkEndReason::Ended
        }
    }
}

/// mock sakura sink を起動する。
///
/// `talk_rx`（kanade→sakura の [`TalkCommand`] 受信端）を別スレッドで読み、各受領を**到着順**に
/// 記録した上で、`Start` については `quit_policy` に従った [`TalkDone`] を `kanade_tx` 経由で
/// kanade inbox へ返す（遅延なし・即時）。`kanade_tx` は TalkDone 返送のためだけに保持する
/// クローンでよい。
///
/// `ResolveChoice`/`CancelChoice` は**記録のみ**行う——本 mock は再生層を持たないため解決も
/// 中断も起こせず、`quit_policy` の index（＝何本目の talk か）も前進させない。到着順の記録は
/// [`MockSakura::commands`] で観測でき、起動系檻が使う [`MockSakura::started`] の意味は不変。
pub fn spawn_mock_sakura(
    talk_rx: Receiver<TalkCommand>,
    kanade_tx: Sender<KanadeMsg>,
    quit_policy: QuitPolicy,
) -> MockSakura {
    let commands: Arc<Mutex<Vec<TalkCommand>>> = Arc::new(Mutex::new(Vec::new()));
    let commands_body = Arc::clone(&commands);

    let join = thread::Builder::new()
        .name("mock-sakura".to_string())
        .spawn(move || {
            let mut index = 0usize;
            // 全 TalkCommand Sender drop（kanade 停止）で recv が Err→ループ終了。
            while let Ok(command) = talk_rx.recv() {
                // 到着順の記録は 3 形共通（記録より先に副作用を起こさない）。
                let start_talk_id = match &command {
                    TalkCommand::Start(start) => Some(start.talk_id),
                    TalkCommand::ResolveChoice { .. } | TalkCommand::CancelChoice { .. } => None,
                };
                commands_body
                    .lock()
                    .expect("mock sakura mutex")
                    .push(command);
                let Some(talk_id) = start_talk_id else {
                    // 選択解決／解除は記録のみ（再生層を持たない mock は TalkDone を作れない）。
                    continue;
                };
                let reason = quit_policy.reason_for(index);
                index += 1;
                // TalkDone 返送。kanade 停止済みで送れなくても無害（続行）。
                let _ = kanade_tx.send(KanadeMsg::TalkDone(TalkDone { talk_id, reason }));
            }
        })
        .expect("spawn mock-sakura thread");

    MockSakura { join, commands }
}

// ============================================================================
// 保留付き mock sakura sink（active talk 窓を決定的に作る・4.4 pattern 3）
// ============================================================================

/// park した [`TalkDone`] の共有状態（保留分の待ち行列＋解放フラグ・Condvar）。
///
/// sakura の recv ループスレッドが park を積み、[`SakuraGate::release_all`] が flag を立て、
/// 専用の releaser スレッドが flag 成立を待って一斉送出する。この 3 者を繋ぐ共有点。
struct GateShared {
    /// (解放フラグ＋park 待ち行列, 起床用 Condvar)。
    ///
    /// bool=解放許可・`Vec<TalkDone>`=まだ送っていない保留 talk（recv ループが積む）。
    inner: Mutex<GateInner>,
    cvar: std::sync::Condvar,
}

/// [`GateShared`] の Mutex 保護部（解放フラグと park 待ち行列）。
struct GateInner {
    /// テストが release_all を呼んだら true（一度立てたら以後 true のまま）。
    released: bool,
    /// recv ループが積んだ保留 [`TalkDone`]（releaser が解放時に drain して送る）。
    parked: Vec<TalkDone>,
    /// recv ループが終了（全 StartTalk Sender drop）したら true。releaser の終了条件。
    recv_closed: bool,
    /// 解放時に送るべき保留 talk の総数（`hold_indices` の要素数）。releaser は
    /// 「解放済み **かつ** `parked.len()` がこの数に達した」時点で初めて drain する。
    ///
    /// これがないと、`release_all` が「保留 talk がまだ park される前」に呼ばれた場合
    /// （kanade が Tick を非同期処理する以上あり得る）、releaser が空の `parked` を drain して
    /// 終了し、後から park された TalkDone が永久に送られず kanade が `Steady{Some}` で
    /// 宙吊りになる（決定性を壊す race）。expected_holds を待つことでこの race を閉じる。
    expected_holds: usize,
}

/// 保留 talk の解放を sakura の releaser スレッドへ通知するゲート（4.4 pattern 3 専用）。
///
/// [`spawn_mock_sakura_gated`] / [`spawn_harness_gated`] が返す。テストは
/// [`release_all`](SakuraGate::release_all) を呼んで、保留していた全 [`TalkDone`] を
/// （各 talk の per-policy quit フラグ付きで）kanade inbox へ返送させる。
///
/// # 決定性（sleep 不要・race-free）
/// 「保留」は、当該 talk の [`TalkDone`] を kanade inbox へ**送らない**ことで実現する。
/// TalkDone は「二つの Tick の間に割り込み得る唯一のメッセージ」であり、これを送らない限り
/// active talk 窓（`Steady{Some}`）は次 Tick まで確実に維持される（interleaving が起きない）。
/// 解放シグナルは `Mutex<bool>`＋`Condvar` で伝える——flag を Mutex 下で立ててから notify する
/// ため lost wakeup は起きない。
pub struct SakuraGate {
    shared: Arc<GateShared>,
}

impl SakuraGate {
    /// 保留中の全 [`TalkDone`] の解放を releaser スレッドへ通知する（sleep なし・確実に起床）。
    ///
    /// flag を Mutex 下で `true` にしてから `notify_all` するため、releaser がまだ wait に
    /// 入る前に呼ばれても取りこぼさない（lost wakeup 対策）。解放後に積まれる park は無い
    /// （テストは全保留 talk 起動後に本メソッドを呼ぶ契約）。
    pub fn release_all(&self) {
        let mut inner = self.shared.inner.lock().expect("sakura gate mutex");
        inner.released = true;
        self.shared.cvar.notify_all();
    }
}

/// 保留機能付き mock sakura sink を起動する（4.4 pattern 3 専用・[`spawn_mock_sakura`] の派生）。
///
/// `hold_indices` に含まれる受領インデックス（0 始まり）の [`StartTalk`] は、記録はするが
/// その [`TalkDone`] を**即座には返さない**（park する）。[`SakuraGate::release_all`] が
/// 呼ばれると、park した全 TalkDone を per-policy quit フラグ付きで返送する。`hold_indices` に
/// 含まれない talk は従来どおり即応する。
///
/// # スレッド構成
/// recv ループスレッド（[`MockSakura::join`] が待つ本体）は従来同様 `talk_rx.recv()` を回して
/// 記録・即応・park を行う。park の**送出**は別の releaser スレッドが担う——recv ループは
/// `recv` で恒常的にブロックし得るため、release_all を実行中の recv ループ内で拾えないからで
/// ある。releaser は「解放フラグ成立」または「recv ループ終了」で起床し、park を送って自然終了
/// する。テストは Sender drop の前に `release_all` を呼ぶ契約（それにより pattern 3 の
/// `Steady{None}` 復帰→close 握手が駆動される）。
pub fn spawn_mock_sakura_gated(
    talk_rx: Receiver<TalkCommand>,
    kanade_tx: Sender<KanadeMsg>,
    quit_policy: QuitPolicy,
    hold_indices: Vec<usize>,
) -> (MockSakura, SakuraGate) {
    let commands: Arc<Mutex<Vec<TalkCommand>>> = Arc::new(Mutex::new(Vec::new()));
    let commands_body = Arc::clone(&commands);

    let expected_holds = hold_indices.len();
    let shared = Arc::new(GateShared {
        inner: Mutex::new(GateInner {
            released: false,
            parked: Vec::new(),
            recv_closed: false,
            expected_holds,
        }),
        cvar: std::sync::Condvar::new(),
    });
    let shared_recv = Arc::clone(&shared);
    let shared_releaser = Arc::clone(&shared);

    // releaser: 「解放済み かつ 保留 talk が全て park された」まで待ち、park された TalkDone を
    // 送出して終了する。recv ループ終了（recv_closed）でも起床する（安全弁: 保留が揃わないまま
    // kanade が停止した誤用時に宙吊りしないため）。
    let releaser_kanade_tx = kanade_tx.clone();
    let releaser = thread::Builder::new()
        .name("mock-sakura-releaser".to_string())
        .spawn(move || {
            let mut inner = shared_releaser.inner.lock().expect("sakura gate mutex");
            // 起床条件:
            //   (a) 解放済み かつ 全保留 talk が park された（＝正規の解放点）、または
            //   (b) recv ループ終了（安全弁・kanade 停止で二度と park は増えない）。
            while !(inner.released && inner.parked.len() >= inner.expected_holds)
                && !inner.recv_closed
            {
                inner = shared_releaser
                    .cvar
                    .wait(inner)
                    .expect("sakura gate condvar wait");
            }
            let to_send: Vec<TalkDone> = inner.parked.drain(..).collect();
            drop(inner);
            for done in to_send {
                // kanade 停止済みで送れなくても無害（続行）。
                let _ = releaser_kanade_tx.send(KanadeMsg::TalkDone(done));
            }
        })
        .expect("spawn mock-sakura-releaser thread");

    let join = thread::Builder::new()
        .name("mock-sakura-gated".to_string())
        .spawn(move || {
            let mut index = 0usize;
            // 全 TalkCommand Sender drop（kanade 停止）で recv が Err→ループ終了。
            while let Ok(command) = talk_rx.recv() {
                // 到着順の記録は 3 形共通（記録より先に副作用を起こさない）。
                let start_talk_id = match &command {
                    TalkCommand::Start(start) => Some(start.talk_id),
                    TalkCommand::ResolveChoice { .. } | TalkCommand::CancelChoice { .. } => None,
                };
                commands_body
                    .lock()
                    .expect("mock sakura mutex")
                    .push(command);
                let Some(talk_id) = start_talk_id else {
                    // 選択解決／解除は記録のみ（保留対象でもない＝index を前進させない）。
                    continue;
                };
                let reason = quit_policy.reason_for(index);
                let this_index = index;
                index += 1;
                let done = TalkDone { talk_id, reason };
                if hold_indices.contains(&this_index) {
                    // 保留: TalkDone を park し、解放シグナルまで送らない（active talk 窓を作る）。
                    // park のたびに releaser を起こす（「解放済み かつ 全 park 到着」を再評価させる・
                    // release_all が park より先行しても取りこぼさない）。
                    let mut inner = shared_recv.inner.lock().expect("sakura gate mutex");
                    inner.parked.push(done);
                    shared_recv.cvar.notify_all();
                } else {
                    // 非保留: 従来どおり即応。kanade 停止済みで送れなくても無害（続行）。
                    let _ = kanade_tx.send(KanadeMsg::TalkDone(done));
                }
            }
            // recv 終了を releaser へ通知（park が残っていて未解放でも宙吊りにしない）。
            {
                let mut inner = shared_recv.inner.lock().expect("sakura gate mutex");
                inner.recv_closed = true;
                shared_recv.cvar.notify_all();
            }
            // releaser の後始末（park 送出）を見届けてから本体スレッドを終える。
            let _ = releaser.join();
        })
        .expect("spawn mock-sakura-gated thread");

    (MockSakura { join, commands }, SakuraGate { shared })
}

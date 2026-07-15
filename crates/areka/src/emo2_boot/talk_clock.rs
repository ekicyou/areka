//! talk 時刻の推定（epoch 推定・クロック注入）とテキスト sink への時刻付与。
//!
//! `TalkClock`（cue 観測による単調 max epoch 推定・クロック注入可・負値 0.0 clamp・
//! epoch 未確立は `None`）と `ClockedTextSink<T: TextSink + Clone>`（`emit` 時に `observe_cue`
//! した後に内側 sink へ透過転送）を所有する（sakura 契約型＋dola clock を消費）。
//!
//! 骨格のみ。`TalkClock::new`／`observe_cue`／`talk_time` と `ClockedTextSink` の実装は
//! tasks.md task 2.2 が担う。

use std::sync::{Arc, Mutex};

use areka_sakura::{TalkCue, TextSink};

/// talk 起点相対秒（`talk_time`）の時刻源（design.md「時刻源 / talk_clock」・R2.2/R2.3）。
///
/// cue 到着観測（`observe_cue`）から talk 開始壁時刻（epoch）を**単調 max** で推定し、毎フレーム
/// の `talk_time = frame_now − epoch`（負値は 0.0 clamp・epoch 未確立は `None`）を供給する。
/// epoch は `Arc<Mutex<Option<f64>>>` 共有で worker（`observe_cue`）が書き UI（`talk_time`）が読む。
/// クロックは注入可能（既定は `dola::runtime::clock::now`＝`FrameTime` と同源の QPC 秒）。
#[derive(Clone)]
pub struct TalkClock {
    /// talk 起点の壁時刻推定（worker 書き／UI 読み・`Arc<Mutex>` 共有）。未確立は `None`。
    epoch: Arc<Mutex<Option<f64>>>,
    /// 壁時刻源（既定は dola clock・テストは固定/可制御クロックを注入する）。
    clock: Arc<dyn Fn() -> f64 + Send + Sync>,
}

impl TalkClock {
    /// 壁時刻源を注入して epoch 未確立の `TalkClock` を生成する（既定は dola clock を渡す）。
    pub fn new(clock: Arc<dyn Fn() -> f64 + Send + Sync>) -> Self {
        Self {
            epoch: Arc::new(Mutex::new(None)),
            clock,
        }
    }

    /// cue 到着観測（worker スレッドから呼ばれる）。`epoch = max(epoch, clock() - at)`。
    ///
    /// 単調 max: 新 talk は到着壁時刻が前方へ跳ぶため epoch が自動リベースされ、talk 内ジッタ
    /// （より小さい `clock() - at`）は max に負けて逆行しない（design.md「時刻源 / talk_clock」）。
    pub fn observe_cue(&self, at: f64) {
        // クロック取得は Mutex の外で行い、保持は代入の一瞬のみに留める（design State Management）。
        let candidate = (self.clock)() - at;
        match self.epoch.lock() {
            Ok(mut guard) => {
                let next = match *guard {
                    Some(current) => current.max(candidate),
                    None => candidate,
                };
                *guard = Some(next);
            }
            Err(_poisoned) => {
                // Mutex は代入/読取の一瞬しか保持せず panic 経路が無いため実質到達不能。到達しても
                // 握り潰さず観測し、現 epoch を維持する（panic 伝播させない・log-first）。
                tracing::error!(
                    "TalkClock epoch mutex poisoned in observe_cue; keeping current epoch"
                );
            }
        }
    }

    /// フレーム時刻から `talk_time` を導出（epoch 未確立は `None`・負値は 0.0 へ clamp）。
    pub fn talk_time(&self, frame_now: f64) -> Option<f64> {
        let epoch = match self.epoch.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => {
                // 読み側は現値（poison 復旧値）を用いて最善既知の talk_time を返す。
                tracing::error!(
                    "TalkClock epoch mutex poisoned in talk_time; reading recovered epoch"
                );
                *poisoned.into_inner()
            }
        };
        // epoch 未確立は None。負値（frame_now < epoch）は 0.0 へ clamp。
        epoch.map(|e| (frame_now - e).max(0.0))
    }
}

/// `TextSink` に talk 時刻観測を挟むデコレータ（design.md「時刻源 / talk_clock」）。
///
/// `emit` ごとに `cue.at` を [`TalkClock::observe_cue`] へ渡して epoch を推定し、その後 cue を
/// **非改変**で内側 sink（本番は `EmoTextSink`）へ透過転送する。`T: Send` のとき自動的に `Send`
/// となり、`areka_ghost::boot` の型境界 `TextSink + Clone + Send + 'static` を満たす。
#[derive(Clone)]
pub struct ClockedTextSink<T: TextSink + Clone> {
    /// 透過転送先（本番は `EmoTextSink`）。
    inner: T,
    /// 観測を記録する共有時刻源。
    clock: TalkClock,
}

impl<T: TextSink + Clone> ClockedTextSink<T> {
    /// 内側 sink と共有 `TalkClock` を束ねてデコレータを生成する。
    pub fn new(inner: T, clock: TalkClock) -> Self {
        Self { inner, clock }
    }
}

impl<T: TextSink + Clone> TextSink for ClockedTextSink<T> {
    fn emit(&mut self, cue: TalkCue) {
        // emit ごとに cue.at を観測して epoch を推定し、cue は非改変で内側へ透過転送する。
        self.clock.observe_cue(cue.at);
        self.inner.emit(cue);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::{ActorKey, CueCommand};
    use std::sync::{Arc, Mutex};

    /// テスト用の可制御クロック: 返り値の `Arc<Mutex<f64>>` に「壁時刻」を書けば、
    /// クロージャがその時刻を返す（決定論・注入クロック）。
    fn controllable_clock() -> (Arc<Mutex<f64>>, Arc<dyn Fn() -> f64 + Send + Sync>) {
        let now = Arc::new(Mutex::new(0.0f64));
        let now_for_clock = Arc::clone(&now);
        let clock: Arc<dyn Fn() -> f64 + Send + Sync> =
            Arc::new(move || *now_for_clock.lock().expect("test clock mutex poisoned"));
        (now, clock)
    }

    /// 可制御クロックの「壁時刻」を書き換える。
    fn set_now(now: &Arc<Mutex<f64>>, wall: f64) {
        *now.lock().expect("test clock mutex poisoned") = wall;
    }

    /// テスト用の記録 TextSink（`ClockedTextSink<T: TextSink + Clone>` に載せるため Clone）。
    /// `MockSink` は記録するが `Clone` を導出していないためここでは使えない（本 sink で代替）。
    #[derive(Clone)]
    struct RecordingTextSink {
        cues: Arc<Mutex<Vec<TalkCue>>>,
    }

    impl RecordingTextSink {
        fn new() -> Self {
            Self {
                cues: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn recorded(&self) -> Vec<TalkCue> {
            self.cues
                .lock()
                .expect("recording sink mutex poisoned")
                .clone()
        }
    }

    impl TextSink for RecordingTextSink {
        fn emit(&mut self, cue: TalkCue) {
            self.cues
                .lock()
                .expect("recording sink mutex poisoned")
                .push(cue);
        }
    }

    /// boot の型境界（`TextSink + Clone + Send + 'static`）を `ClockedTextSink<T>` が
    /// 保つことをコンパイル時に固定する（具体 `EmoTextSink` 結線は task 5.1 の責務ゆえ、
    /// ここでは汎用の Send な TextSink 実装で型のみを表明する）。
    fn assert_send_clone_static<T: TextSink + Clone + Send + 'static>() {
        fn require<X: Send + Clone + 'static>() {}
        require::<ClockedTextSink<T>>();
    }

    /// 観測可能な完了条件: 新 talk 到着で epoch が前方へリベースされる（単調 max）。
    #[test]
    fn new_talk_rebases_epoch_forward() {
        let (now, clock) = controllable_clock();
        let tc = TalkClock::new(clock);
        // PROBE は epoch より十分大きく取り talk_time が clamp されないようにする
        // （epoch = PROBE - talk_time(PROBE) で epoch 値を逆算できる）。
        const PROBE: f64 = 10_000.0;

        // talk1: 壁時刻 100.0 に cue.at=2.0 が到着 → epoch = 100.0 - 2.0 = 98.0。
        set_now(&now, 100.0);
        tc.observe_cue(2.0);
        let epoch_talk1 = PROBE - tc.talk_time(PROBE).expect("epoch established after first cue");
        assert!(
            (epoch_talk1 - 98.0).abs() < 1e-9,
            "talk1 epoch should be 98.0, got {epoch_talk1}"
        );
        // talk1 の到着フレームでの talk_time ≒ cue.at。
        let t_arrival_1 = tc.talk_time(100.0).expect("epoch established");
        assert!(
            (t_arrival_1 - 2.0).abs() < 1e-9,
            "talk1 arrival talk_time should be ~2.0, got {t_arrival_1}"
        );

        // talk2（新 talk）: 壁時刻が 200.0 へ前進し cue.at=0.0 で開始
        // → candidate = 200.0 - 0.0 = 200.0 > 98.0 ゆえ epoch は前方 200.0 へリベース。
        set_now(&now, 200.0);
        tc.observe_cue(0.0);
        let epoch_talk2 = PROBE - tc.talk_time(PROBE).expect("epoch established after new talk");
        assert!(
            (epoch_talk2 - 200.0).abs() < 1e-9,
            "talk2 epoch should rebase forward to 200.0, got {epoch_talk2}"
        );

        // 前方リベースの確認（観測可能な完了条件）。
        assert!(
            epoch_talk2 > epoch_talk1,
            "new talk must rebase epoch FORWARD: {epoch_talk2} !> {epoch_talk1}"
        );
        // 新 talk の開始フレームでは talk_time ≒ 0（typewriter を頭出しできる）。
        let t_start_2 = tc.talk_time(200.0).expect("epoch established");
        assert!(
            (t_start_2 - 0.0).abs() < 1e-9,
            "new talk should start at talk_time ~0.0, got {t_start_2}"
        );
    }

    /// talk 内ジッタ（より小さい candidate）は単調 max に負けて epoch を下げない。
    #[test]
    fn intra_talk_jitter_does_not_lower_epoch() {
        let (now, clock) = controllable_clock();
        let tc = TalkClock::new(clock);
        const PROBE: f64 = 10_000.0;

        // epoch 確立: 壁 100.0・at 0.0 → epoch = 100.0。
        set_now(&now, 100.0);
        tc.observe_cue(0.0);
        let epoch0 = PROBE - tc.talk_time(PROBE).expect("epoch established");
        assert!((epoch0 - 100.0).abs() < 1e-9, "initial epoch 100.0, got {epoch0}");

        // talk 内ジッタ: 壁が 100.5 へ微増したが cue.at=1.0（遅れて届いた大きめ at）
        // → candidate = 100.5 - 1.0 = 99.5 < 100.0。単調 max ゆえ epoch は下がらない。
        set_now(&now, 100.5);
        tc.observe_cue(1.0);
        let epoch1 = PROBE - tc.talk_time(PROBE).expect("epoch established");
        assert!(
            (epoch1 - 100.0).abs() < 1e-9,
            "monotonic max must suppress intra-talk regression: epoch should stay 100.0, got {epoch1}"
        );
    }

    /// epoch 未確立（observe_cue 未呼出）は `talk_time` が常に `None`。
    #[test]
    fn fresh_clock_has_no_epoch() {
        let (_now, clock) = controllable_clock();
        let tc = TalkClock::new(clock);
        assert_eq!(tc.talk_time(0.0), None);
        assert_eq!(tc.talk_time(12_345.6), None);
        assert_eq!(tc.talk_time(-5.0), None);
    }

    /// `frame_now < epoch` の負値経過は 0.0 へ clamp（境界 `==` はちょうど 0.0）。
    #[test]
    fn talk_time_clamps_negative_to_zero() {
        let (now, clock) = controllable_clock();
        let tc = TalkClock::new(clock);
        // epoch = 500.0。
        set_now(&now, 500.0);
        tc.observe_cue(0.0);

        // frame_now < epoch → 負値は 0.0 へ clamp。
        assert_eq!(tc.talk_time(499.0), Some(0.0));
        assert_eq!(tc.talk_time(0.0), Some(0.0));
        // 境界: frame_now == epoch → ちょうど 0.0。
        assert_eq!(tc.talk_time(500.0), Some(0.0));
        // frame_now > epoch → 正の経過秒。
        let t = tc.talk_time(510.0).expect("epoch established");
        assert!((t - 10.0).abs() < 1e-9, "elapsed should be 10.0, got {t}");
    }

    /// `ClockedTextSink::emit` は (a) 内側へ cue を非改変転送し、(b) `cue.at` を観測する。
    #[test]
    fn clocked_sink_forwards_cue_unchanged_and_observes_at() {
        let (now, clock) = controllable_clock();
        set_now(&now, 300.0);
        let tc = TalkClock::new(clock);
        let inner = RecordingTextSink::new();
        // inner を clone してテスト側の観測ハンドルを保持（Arc 共有ゆえ内側の記録が見える）。
        let mut sink = ClockedTextSink::new(inner.clone(), tc.clone());

        // emit 前: epoch 未確立。
        assert_eq!(tc.talk_time(300.0), None);

        let cue = TalkCue {
            at: 5.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("hi".into()),
            duration: 0.0,
        };
        sink.emit(cue.clone());

        // (a) 内側 sink が「全く同じ cue」を受け取る（at/actor/command 非改変）。
        let recorded = inner.recorded();
        assert_eq!(recorded.len(), 1, "inner sink should receive exactly one cue");
        assert_eq!(
            recorded[0], cue,
            "cue must be forwarded UNCHANGED (at/actor/command)"
        );

        // (b) clock が cue.at を観測 → epoch = 300.0 - 5.0 = 295.0 が確立。
        let t = tc
            .talk_time(300.0)
            .expect("emit must establish epoch via observe_cue");
        assert!(
            (t - 5.0).abs() < 1e-9,
            "observed at should make talk_time ~cue.at at emit wall-time, got {t}"
        );
    }

    /// `ClockedTextSink<T>` が boot 型境界（`Send + Clone + 'static`）を満たすことをコンパイル時表明。
    #[test]
    fn clocked_sink_satisfies_boot_bounds_when_inner_does() {
        assert_send_clone_static::<RecordingTextSink>();
    }
}

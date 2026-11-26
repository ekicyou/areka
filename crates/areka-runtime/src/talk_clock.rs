//! talk 時刻の推定（epoch 推定・クロック注入）とテキスト sink への時刻付与。

use std::sync::{Arc, Mutex};

use areka_sakura::TalkCue;
use dola::{TimePoint, TimeSpan};

/// talk の台本の時刻（`talk_time`）の時刻源。
///
/// cue 到着観測（`observe_cue`）から talk 開始の時刻（epoch）を**単調 max** で推定し、毎フレーム
/// の `talk_time = frame_now − epoch`（負は 0 へ寄せる・epoch 未確立は `None`）を供給する。
/// epoch は `Arc<Mutex<Option<TimePoint>>>` 共有で worker（`observe_cue`）が書き UI（`talk_time`）が読む。
/// クロックは注入可能（既定は `dola::now`＝`FrameTime` と同じ時計）。
#[derive(Clone)]
pub struct TalkClock {
    /// talk 起点の時刻の推定（worker 書き／UI 読み・`Arc<Mutex>` 共有）。未確立は `None`。
    epoch: Arc<Mutex<Option<TimePoint>>>,
    /// 時計（既定は `dola::now`・テストは固定/可制御クロックを注入する）。
    clock: Arc<dyn Fn() -> TimePoint + Send + Sync>,
}

impl TalkClock {
    /// 時計を注入して epoch 未確立の `TalkClock` を生成する（既定は `dola::now` を渡す）。
    pub fn new(clock: Arc<dyn Fn() -> TimePoint + Send + Sync>) -> Self {
        Self {
            epoch: Arc::new(Mutex::new(None)),
            clock,
        }
    }

    /// cue 到着観測（worker スレッドから呼ばれる）。`epoch = max(epoch, clock() - at)`。
    ///
    /// 単調 max: 新 talk は到着壁時刻が前方へ跳ぶため epoch が自動リベースされ、talk 内ジッタ
    /// （より小さい `clock() - at`）は max に負けて逆行しない。
    pub fn observe_cue(&self, at: TimeSpan) {
        // クロック取得は Mutex の外で行い、保持は代入の一瞬のみに留める。
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
                // 握り潰さず観測し、現 epoch を維持する。
                tracing::error!(
                    "TalkClock epoch mutex poisoned in observe_cue; keeping current epoch"
                );
            }
        }
    }

    /// フレーム時刻から `talk_time` を導出（epoch 未確立は `None`・epoch より前は 0 へ寄せる）。
    pub fn talk_time(&self, frame_now: TimePoint) -> Option<TimeSpan> {
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
        epoch.map(|e| frame_now.span_since(e).unwrap_or(TimeSpan::ZERO))
    }

    /// 今の台本の時刻（注入された時計を読んで [`Self::talk_time`] に渡す・起点が無ければ `None`）。
    /// talk スレッドの受け口が落ちた瞬間の時刻を、フレームに丸めずに読むための口。
    pub fn now_talk_time(&self) -> Option<TimeSpan> {
        self.talk_time((self.clock)())
    }
}

/// 単一の出力契約 [`dola::cue::CueSink`] に talk 時刻観測を挟むデコレータ。
///
/// `emit` ごとに `cue.at` を [`TalkClock::observe_cue`] へ渡して epoch を推定し、その後 cue を
/// **非改変**で内側 sink（本番は `EmoTextSink`）へ透過転送する。`T: Send` のとき自動的に `Send`
/// となり、`areka_ghost::boot` の型境界 `dola::cue::CueSink + Clone + Send + 'static` を満たす。
#[derive(Clone)]
pub struct ClockedTextSink<T: dola::cue::CueSink + Clone> {
    /// 透過転送先（本番は `EmoTextSink`）。
    inner: T,
    /// 観測を記録する共有時刻源。
    clock: TalkClock,
}

impl<T: dola::cue::CueSink + Clone> ClockedTextSink<T> {
    /// 内側 sink と共有 `TalkClock` を束ねてデコレータを生成する。
    pub fn new(inner: T, clock: TalkClock) -> Self {
        Self { inner, clock }
    }
}

/// 演者非依存の単一出力契約 [`dola::cue::CueSink`] を実装する（`areka_ghost::boot` の broadcast
/// 登録先が要求する形）。broadcast 下では担当外 cue（`Emote` 等）も本 sink へ届くが、
/// `observe_cue` は `cue.at` 一貫で epoch を単調 max 推定するため無害。
/// 内側 sink（本番は `EmoTextSink`）へ `CueSink::emit` で非改変転送し、先渡し（`preview`）も
/// 時刻を観測せずに内側へ通す。選択待ちの始まり（`choice_wait_began`）も同じく内側へ通す。
impl<T: dola::cue::CueSink + Clone> dola::cue::CueSink for ClockedTextSink<T> {
    fn emit(&mut self, cue: TalkCue) {
        self.clock.observe_cue(cue.at);
        dola::cue::CueSink::emit(&mut self.inner, cue);
    }

    fn emit_with(&mut self, cue: TalkCue, probe: &dola::cue::CueProbe) {
        // 既定の emit に落とすと文字の層が名前の形の `\b` を知らせられない。
        self.clock.observe_cue(cue.at);
        dola::cue::CueSink::emit_with(&mut self.inner, cue, probe);
    }

    fn preview(&mut self, upcoming: &[TalkCue]) {
        // 先渡しは「その時刻に届いた」出来事ではないので時刻は観測しない（トークの頭の時刻の
        // 見積もりに混ぜない）。
        dola::cue::CueSink::preview(&mut self.inner, upcoming);
    }

    fn choice_wait_began(&mut self) {
        // 通さないと既定の何もしないに落ちて、文字の層が押下で選択肢を済ませてよいと知れない。
        dola::cue::CueSink::choice_wait_began(&mut self.inner);
    }
}

/// テスト用の可制御クロック: 返り値の `Arc<Mutex<TimePoint>>` に時刻を書けば、
/// `TalkClock` がその時刻を読む（決定論・注入クロック）。
#[cfg(test)]
pub(crate) fn controllable_clock() -> (Arc<Mutex<TimePoint>>, TalkClock) {
    let now = Arc::new(Mutex::new(TimePoint::ZERO));
    let now_for_clock = Arc::clone(&now);
    let clock = TalkClock::new(Arc::new(move || {
        *now_for_clock.lock().expect("test clock mutex poisoned")
    }));
    (now, clock)
}

/// 可制御クロックの時刻を書き換える。
#[cfg(test)]
pub(crate) fn set_now(now: &Arc<Mutex<TimePoint>>, wall: TimePoint) {
    *now.lock().expect("test clock mutex poisoned") = wall;
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_sakura::{ActorKey, CueCommand};
    use dola::cue::CueSink;
    use dola::time::testing::{at_ms, ms};
    use std::sync::{Arc, Mutex};

    fn text_cue(at: TimeSpan, actor: &str, text: &str) -> TalkCue {
        TalkCue::scheduled(
            TimePoint::ZERO,
            at,
            ActorKey::from(actor),
            CueCommand::Text(text.into()),
            TimeSpan::ZERO,
        )
    }

    /// テスト用の記録 `CueSink`（`ClockedTextSink<T: dola::cue::CueSink + Clone>` に載せるため Clone）。
    #[derive(Clone)]
    struct RecordingTextSink {
        cues: Arc<Mutex<Vec<TalkCue>>>,
        /// 先渡しで受け取った列（呼ばれた回数ぶん積む）。
        previews: Arc<Mutex<Vec<Vec<TalkCue>>>>,
        /// 選択待ちの始まりを受けた回数。
        began: Arc<Mutex<usize>>,
        /// 知らせ付きで受けた合図（`emit_with`・`cues` にも積む）。
        probed: Arc<Mutex<Vec<TalkCue>>>,
    }

    impl RecordingTextSink {
        fn new() -> Self {
            Self {
                cues: Arc::new(Mutex::new(Vec::new())),
                previews: Arc::new(Mutex::new(Vec::new())),
                began: Arc::new(Mutex::new(0)),
                probed: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn probed(&self) -> Vec<TalkCue> {
            self.probed
                .lock()
                .expect("recording sink mutex poisoned")
                .clone()
        }

        fn began_count(&self) -> usize {
            *self.began.lock().expect("recording sink mutex poisoned")
        }

        fn previewed(&self) -> Vec<Vec<TalkCue>> {
            self.previews
                .lock()
                .expect("recording sink mutex poisoned")
                .clone()
        }

        fn recorded(&self) -> Vec<TalkCue> {
            self.cues
                .lock()
                .expect("recording sink mutex poisoned")
                .clone()
        }
    }

    impl dola::cue::CueSink for RecordingTextSink {
        fn emit(&mut self, cue: TalkCue) {
            self.cues
                .lock()
                .expect("recording sink mutex poisoned")
                .push(cue);
        }

        fn preview(&mut self, upcoming: &[TalkCue]) {
            self.previews
                .lock()
                .expect("recording sink mutex poisoned")
                .push(upcoming.to_vec());
        }

        fn choice_wait_began(&mut self) {
            *self.began.lock().expect("recording sink mutex poisoned") += 1;
        }

        fn emit_with(&mut self, cue: TalkCue, _probe: &dola::cue::CueProbe) {
            self.probed
                .lock()
                .expect("recording sink mutex poisoned")
                .push(cue.clone());
            self.emit(cue);
        }
    }

    /// boot の型境界（`dola::cue::CueSink + Clone + Send + 'static`）を `ClockedTextSink<T>` が
    /// 保つことをコンパイル時に固定する。型の境界だけを汎用の Send な `CueSink` 実装で確かめる
    /// （具体の `EmoTextSink` との結線は `crates/areka-session/src/runtime_boot.rs` にある）。
    fn assert_send_clone_static<T: dola::cue::CueSink + Clone + Send + 'static>() {
        fn require<X: Send + Clone + 'static>() {}
        require::<ClockedTextSink<T>>();
    }

    /// 新 talk 到着で epoch が前方へリベースされる（単調 max）。
    #[test]
    fn new_talk_rebases_epoch_forward() {
        let (now, tc) = controllable_clock();
        // PROBE は epoch より十分先に取り talk_time が 0 へ寄らないようにする
        // （epoch = PROBE - talk_time(PROBE) で epoch 値を逆算できる）。
        const PROBE: TimePoint = at_ms(10_000_000);

        // talk1: 時刻 100 s に cue.at=2 s が到着 → epoch = 98 s。
        set_now(&now, at_ms(100_000));
        tc.observe_cue(ms(2_000));
        let epoch_talk1 = PROBE
            - tc.talk_time(PROBE)
                .expect("epoch established after first cue");
        assert_eq!(epoch_talk1, at_ms(98_000));
        // talk1 の到着フレームでの talk_time は cue.at。
        assert_eq!(tc.talk_time(at_ms(100_000)), Some(ms(2_000)));

        // talk2（新 talk）: 時刻が 200 s へ進み cue.at=0 で開始
        // → candidate = 200 s > 98 s ゆえ epoch は前方 200 s へリベース。
        set_now(&now, at_ms(200_000));
        tc.observe_cue(TimeSpan::ZERO);
        let epoch_talk2 = PROBE
            - tc.talk_time(PROBE)
                .expect("epoch established after new talk");
        assert_eq!(epoch_talk2, at_ms(200_000));
        assert!(
            epoch_talk2 > epoch_talk1,
            "new talk must rebase epoch FORWARD: {epoch_talk2} !> {epoch_talk1}"
        );
        // 新 talk の開始フレームでは talk_time は 0（typewriter を頭出しできる）。
        assert_eq!(tc.talk_time(at_ms(200_000)), Some(TimeSpan::ZERO));
    }

    /// talk 内ジッタ（より小さい candidate）は単調 max に負けて epoch を下げない。
    #[test]
    fn intra_talk_jitter_does_not_lower_epoch() {
        let (now, tc) = controllable_clock();
        const PROBE: TimePoint = at_ms(10_000_000);

        // epoch 確立: 時刻 100 s・at 0 → epoch = 100 s。
        set_now(&now, at_ms(100_000));
        tc.observe_cue(TimeSpan::ZERO);
        let epoch0 = PROBE - tc.talk_time(PROBE).expect("epoch established");
        assert_eq!(epoch0, at_ms(100_000));

        // talk 内ジッタ: 時刻が 100.5 s へ微増したが cue.at=1 s（遅れて届いた大きめ at）
        // → candidate = 99.5 s < 100 s。単調 max ゆえ epoch は下がらない。
        set_now(&now, at_ms(100_500));
        tc.observe_cue(ms(1_000));
        let epoch1 = PROBE - tc.talk_time(PROBE).expect("epoch established");
        assert_eq!(
            epoch1,
            at_ms(100_000),
            "monotonic max must suppress intra-talk regression"
        );
    }

    /// epoch 未確立（observe_cue 未呼出）は `talk_time` が常に `None`。
    #[test]
    fn fresh_clock_has_no_epoch() {
        let (_now, tc) = controllable_clock();
        assert_eq!(tc.talk_time(at_ms(0)), None);
        assert_eq!(tc.talk_time(at_ms(12_345_600)), None);
        assert_eq!(tc.talk_time(at_ms(-5_000)), None);
    }

    /// `frame_now < epoch` は 0 へ寄せる（境界 `==` はちょうど 0）。
    #[test]
    fn talk_time_clamps_negative_to_zero() {
        let (now, tc) = controllable_clock();
        // epoch = 500 s。
        set_now(&now, at_ms(500_000));
        tc.observe_cue(TimeSpan::ZERO);

        assert_eq!(tc.talk_time(at_ms(499_000)), Some(TimeSpan::ZERO));
        assert_eq!(tc.talk_time(at_ms(0)), Some(TimeSpan::ZERO));
        // 境界: frame_now == epoch → ちょうど 0。
        assert_eq!(tc.talk_time(at_ms(500_000)), Some(TimeSpan::ZERO));
        // frame_now > epoch → 経過。
        assert_eq!(tc.talk_time(at_ms(510_000)), Some(ms(10_000)));
    }

    /// `ClockedTextSink::emit` は (a) 内側へ cue を非改変転送し、(b) `cue.at` を観測する。
    #[test]
    fn clocked_sink_forwards_cue_unchanged_and_observes_at() {
        let (now, tc) = controllable_clock();
        set_now(&now, at_ms(300_000));
        let inner = RecordingTextSink::new();
        let mut sink = ClockedTextSink::new(inner.clone(), tc.clone());

        assert_eq!(tc.talk_time(at_ms(300_000)), None);

        let cue = text_cue(ms(5_000), "0", "hi");
        sink.emit(cue.clone());

        let recorded = inner.recorded();
        assert_eq!(
            recorded.len(),
            1,
            "inner sink should receive exactly one cue"
        );
        assert_eq!(
            recorded[0], cue,
            "cue must be forwarded UNCHANGED (at/actor/command)"
        );

        assert_eq!(
            tc.talk_time(at_ms(300_000)),
            Some(ms(5_000)),
            "observed at should make talk_time cue.at at emit time"
        );
    }

    /// `ClockedTextSink::preview` は (a) 内側へ列を非改変で 1 度だけ通し、(b) 時刻を観測しない
    /// （先渡しは「その時刻に届いた」出来事ではないので、トークの頭の時刻の見積もりに混ぜない）。
    #[test]
    fn clocked_sink_passes_preview_through_without_observing() {
        let (now, tc) = controllable_clock();
        let inner = RecordingTextSink::new();
        let mut sink = ClockedTextSink::new(inner.clone(), tc.clone());

        let upcoming = vec![
            text_cue(TimeSpan::ZERO, "0", "イイジャン！"),
            text_cue(ms(1_500), "1", "‥‥ええと、"),
        ];

        // 見積もりが未確立のまま先渡しを通す: 観測すれば epoch が立ってしまう。
        set_now(&now, at_ms(300_000));
        sink.preview(&upcoming);
        assert_eq!(
            inner.previewed(),
            vec![upcoming.clone()],
            "inner sink must receive the preview list UNCHANGED, exactly once"
        );
        assert_eq!(
            tc.talk_time(at_ms(300_000)),
            None,
            "preview must not observe time (epoch stays unestablished)"
        );
        assert!(inner.recorded().is_empty(), "preview must not emit cues");

        // 見積もりが立った後でも、先渡しの前後で値が変わらない。
        sink.emit(upcoming[0].clone());
        let before = tc.talk_time(at_ms(10_000_000));
        set_now(&now, at_ms(900_000));
        sink.preview(&upcoming);
        assert_eq!(
            tc.talk_time(at_ms(10_000_000)),
            before,
            "preview must not move the established epoch"
        );
        assert_eq!(inner.previewed().len(), 2);
    }

    /// `ClockedTextSink::choice_wait_began` は内側へ呼ばれた回数だけ取り次ぎ（取り次がないと既定の
    /// 何もしないに落ちて押下で済まなくなる）、時刻は観測しない。
    #[test]
    fn clocked_sink_forwards_choice_wait_began() {
        let (_now, tc) = controllable_clock();
        let inner = RecordingTextSink::new();
        let mut sink = ClockedTextSink::new(inner.clone(), tc.clone());

        sink.choice_wait_began();
        assert_eq!(
            inner.began_count(),
            1,
            "inner sink must receive choice_wait_began"
        );
        sink.choice_wait_began();
        assert_eq!(inner.began_count(), 2, "forwarded once per call");
        assert_eq!(tc.talk_time(at_ms(300_000)), None, "must not observe time");
        assert!(inner.recorded().is_empty() && inner.previewed().is_empty());
    }

    /// `ClockedTextSink::emit_with` は時刻を観測してから、知らせの口ごと内側の `emit_with` へ
    /// 1 回だけ通す（通さないと名前の形の `\b` の知らせが文字の層へ届かない）。
    #[test]
    fn clocked_sink_forwards_emit_with_after_observing_time() {
        let (now, tc) = controllable_clock();
        let inner = RecordingTextSink::new();
        let mut sink = ClockedTextSink::new(inner.clone(), tc.clone());
        let probe = dola::cue::CueProbe::new(|_| {});
        let cue = text_cue(ms(1_500), "0", "x");

        set_now(&now, at_ms(100_000));
        sink.emit_with(cue.clone(), &probe);
        assert_eq!(
            inner.probed(),
            vec![cue.clone()],
            "inner must receive emit_with"
        );
        assert_eq!(inner.recorded(), vec![cue], "delivered exactly once");
        assert_eq!(
            tc.talk_time(at_ms(100_000)),
            Some(ms(1_500)),
            "time observed (epoch = 100 - 1.5)"
        );
    }

    /// `ClockedTextSink<T>` が boot 型境界（`Send + Clone + 'static`）を満たすことをコンパイル時表明。
    #[test]
    fn clocked_sink_satisfies_boot_bounds_when_inner_does() {
        assert_send_clone_static::<RecordingTextSink>();
    }
}

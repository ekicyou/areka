// =============================================================================
// 受け口から可視性の相までの通し（areka-P0-balloon-lifecycle-events task 5.2）
//
// `balloon_visibility_lifecycle_e2e_tests.rs`（台本の占有終端が判断中核の計測起点になる）の続き。
// あちらは headless の表示層で相を回すのでバルーンが可視にならず、時間切れまで届かない。ここは
// 親の檻（実 emo2 fixture ＋ 実 GPU・本番の `emo2_frame_system`）で、
//   台詞 → 本物の受け口（配送と同じく原本を複製して番号を渡す）→ トークの終わり → 時刻の注入
//   → 隠す発行 → 置き場のゴーストの kanade への知らせ
// までを 1 本で通す（要件 2.1・5.1・8.1・8.2・9.1）。可視性の相の中の知らせの送出の 1 行を
// 踏むのはこの檻だけである（相の檻 `balloon_visibility_phase_notice_tests.rs` は判断と送出を
// 自分で並べて呼ぶ）。
//
// 時刻はすべて注入（受け口の時計・`TalkClock` の起点・`FrameTime`）で、満了の直前・ちょうど・
// 直後を追い越さない刻みで進める。実時間の待機は用いない。
// =============================================================================

use std::sync::{Arc, Mutex};

use areka_ghost::sink::BootCueSink;
use areka_kanade::KanadeMsg;
use areka_sakura::TalkId;
use areka_sakura::sysvar::SystemVarSnapshot;
use dola::cue::{CuePlayer, CueSink};

use crate::emo2_boot::balloon_visibility::configured_timeout_secs;
use crate::emo2_boot::talk_clock::TalkClock;
use crate::emo2_boot::talk_lifecycle::BalloonLifecycleSink;

use super::*;

/// 受け口に渡すトークの番号（判断の側が勝手に振る番号と取り違えないよう 1 以外）。
const TALK: TalkId = TalkId(7);

/// 配られた cue を写し取る受け口。本番は文字の受け口が UI へ運ぶ分を、檻が文字の層へ当てる。
#[derive(Clone, Default)]
struct Recorded(Arc<Mutex<Vec<TalkCue>>>);

impl CueSink for Recorded {
    fn emit(&mut self, cue: TalkCue) {
        self.0.lock().expect("写しの錠").push(cue);
    }
}

/// 親の檻に、手で進める壁時刻・受け口への送出端・置き場のゴーストの受け端を足したもの。
struct Cage {
    world: World,
    /// フレームの間だけ world へ預けるので `Option` で持つ。
    wiring: Option<Emo2Wiring>,
    wall: Arc<Mutex<f64>>,
    lifecycle_tx: mpsc::Sender<TalkLifecycleSignal>,
    kanade: mpsc::Receiver<KanadeMsg>,
    _present_tx: mpsc::Sender<PresentCommand>,
    _gw: crate::placement::spawn::GhostWindows,
    /// GPU の装置の許可。欄は宣言の順に落ちるので、最後に置いて装置の世界より後に返す。
    _gpu: crate::emo2_boot::spine::GpuPermit,
}

impl Cage {
    /// 檻を組み、装着のフレームまで回す。talk 相対秒＝壁時刻（起点 0）。
    fn boot() -> Self {
        let (gpu, mut world, gw) = gpu_frame_world();
        let kanade = seat_ghost(&mut world);
        let (present_tx, present_rx) = mpsc::channel::<PresentCommand>();
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
        let mut wiring = boot_wiring(present_rx, lifecycle_rx);
        let wall = Arc::new(Mutex::new(0.0_f64));
        wiring.clock = TalkClock::new({
            let wall = Arc::clone(&wall);
            Arc::new(move || *wall.lock().expect("時計の錠"))
        });
        wiring.clock.observe_cue(0.0);
        let mut cage = Self {
            world,
            wiring: Some(wiring),
            wall,
            lifecycle_tx,
            kanade,
            _present_tx: present_tx,
            _gw: gw,
            _gpu: gpu,
        };
        cage.frame_at(0.0);
        assert_eq!(
            cage.wiring().balloon_model_scopes(),
            vec![0u32, 1],
            "前提: 両 scope の balloon 装着が成立している"
        );
        cage
    }

    fn wiring(&self) -> &Emo2Wiring {
        self.wiring
            .as_ref()
            .expect("フレームの外では結線を持っている")
    }

    /// 台本を compile して最後まで再生し、受け口を落とす。返り値は台本の占有終端。
    ///
    /// 受け口は配送と同じ形（登録の原本を複製し、番号を渡す）で組む。配られた cue は文字の層へ
    /// 当て、壁時刻を占有終端まで進めてから受け口を落とす（止まった時刻＝占有終端）。
    fn play(&mut self, script: &str) -> f64 {
        let compiled = areka_sakura::compile(
            &areka_parsers::sakura::parse(script),
            &SystemVarSnapshot::default(),
        );
        let horizon = compiled.sheet.absolute_end_time();
        let recorded = Recorded::default();
        let mut lifecycle =
            BalloonLifecycleSink::new(self.lifecycle_tx.clone(), self.wiring().clock.clone())
                .clone_box();
        lifecycle.begin_talk(TALK);
        let mut player = CuePlayer::from_sheet(&compiled.sheet);
        player.register_sink(lifecycle);
        player.register_sink(Box::new(recorded.clone()));
        let mut ticks: Vec<f64> = compiled
            .sheet
            .cues()
            .iter()
            .map(|cue| compiled.sheet.absolute_fire_time(cue))
            .collect();
        ticks.push(horizon);
        for at in ticks {
            player.tick(at);
        }
        let mut runtime = self.wiring().runtime.borrow_mut();
        for cue in recorded.0.lock().expect("写しの錠").drain(..) {
            runtime.apply_cue(&cue);
        }
        drop(runtime);
        *self.wall.lock().expect("時計の錠") = horizon;
        drop(player);
        horizon
    }

    /// 注入した時刻で本番の相順を 1 フレーム回し、その間のログを返す。
    fn frame_at(&mut self, now: f64) -> Vec<LogEvent> {
        self.world.insert_resource(FrameTime(now));
        let wiring = self.wiring.take().expect("結線は檻が持っている");
        let (wiring, events) = capture_logs(|| advance_frame(&mut self.world, wiring));
        self.wiring = Some(wiring);
        events
    }

    fn visible(&self) -> Option<bool> {
        self.wiring().presenter.target_visible(balloon_target(0))
    }

    /// 置き場のゴーストの kanade に届いた時間切れの知らせの番号。バルーンの組の届け（親の ⑸ の
    /// 檻が見る）は読み捨て、それ以外が届いたら檻の誤り。
    fn notices(&self) -> Vec<TalkId> {
        self.kanade
            .try_iter()
            .filter_map(|msg| match msg {
                KanadeMsg::BalloonTimeout { talk_id } => Some(talk_id),
                KanadeMsg::ExecutionState(_) => None,
                _ => panic!("時間切れの知らせとバルーンの組の届け以外のメッセージが届いた"),
            })
            .collect()
    }
}

/// 台詞を出して終わらせ、`deadline` の直前・ちょうど・直後を回す。ちょうどのフレームで隠れて
/// 受け口が受けた番号の知らせが 1 通だけ届き、その前後では届かないことを主張する。
fn expire_at(cage: &mut Cage, horizon: f64, deadline: f64) {
    cage.frame_at(horizon);
    assert_eq!(
        cage.visible(),
        Some(true),
        "前提: 台詞の文字でバルーンが出ている"
    );
    assert_eq!(cage.notices(), vec![], "出たフレームでは送らない");

    cage.frame_at(deadline - 0.001);
    assert_eq!(cage.visible(), Some(true), "満了の直前は隠さない");
    assert_eq!(cage.notices(), vec![], "満了の直前は送らない");

    let events = cage.frame_at(deadline);
    assert_eq!(
        cage.visible(),
        Some(false),
        "満了のちょうどで隠す発行が通る"
    );
    assert_eq!(
        cage.notices(),
        vec![TALK],
        "知らせに受け口が受けた番号が載って 1 通だけ届く"
    );
    let notified: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.field("event") == Some("\"balloon_timeout_notified\""))
        .collect();
    assert_eq!(notified.len(), 1, "送った記録が 1 行: {events:?}");
    assert_eq!(notified[0].expect_field("talk_id"), TALK.0.to_string());

    cage.frame_at(deadline + 1.0);
    assert_eq!(cage.notices(), vec![], "隠した後のフレームは送らない");
}

/// 台詞の終わりから既定の待ち時間で時間切れになり、本物の相が受け口の番号の知らせを kanade へ
/// 送る（要件 2.1・5.1）。
#[test]
fn timeout_after_a_real_talk_sends_the_sink_number_through_the_real_phase() {
    // 待ち時間の確定はプロセスで一度きりの記録なので、捕捉窓の外で先に確定させる。
    let timeout_secs = configured_timeout_secs();
    let mut cage = Cage::boot();
    let horizon = cage.play(r"\0あい\e");
    expire_at(&mut cage, horizon, horizon + timeout_secs);
}

/// `\![set,balloontimeout,…]` を含む台詞では、そのトークの満了の時刻が指定の値へ動く（要件 9.1）。
/// 直前で隠れずちょうどで隠れるので、既定の待ち時間のままなら（既定が長くても短くても）落ちる。
#[test]
fn balloontimeout_in_the_talk_moves_the_expiry_of_that_talk() {
    let timeout_secs = configured_timeout_secs();
    let wait = 1234.0 / 1000.0;
    assert_ne!(
        wait, timeout_secs,
        "前提: 指定の値は既定の待ち時間と違う（同じなら本檻は何も弁別しない）"
    );
    let mut cage = Cage::boot();
    let horizon = cage.play(r"\0あい\![set,balloontimeout,1234]\e");
    expire_at(&mut cage, horizon, horizon + wait);
}

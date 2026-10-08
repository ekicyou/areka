//! task 3.1（要件 2.6・design.md「BootCueSink::begin_talk」）: 配送がトークの起動ごとに、
//! 複製した受け口へそのトークの番号を渡すことの檻。

use super::test_support::{run_bounded, test_system_vars};
use super::*;
use areka_sakura::contract::TalkCue;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

/// 受け口の 1 つの出来事（どの受け口か・何が起きたか・その複製が受け持つ番号）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Begin(&'static str, TalkId),
    Emit(&'static str, Option<TalkId>),
    Drop(&'static str, TalkId),
}

/// 番号を受け取る記録の受け口。`Clone` を持たないので一括の実装に乗らず、
/// `BootCueSink` を自分で実装して `begin_talk` を上書きする（本番の受け口と同じ形）。
struct TalkIdSink {
    name: &'static str,
    talk_id: Option<TalkId>,
    log: Arc<Mutex<Vec<Event>>>,
}

impl CueSink for TalkIdSink {
    fn emit(&mut self, _cue: TalkCue) {
        self.log
            .lock()
            .expect("log mutex poisoned")
            .push(Event::Emit(self.name, self.talk_id));
    }
}

impl BootCueSink for TalkIdSink {
    fn clone_box(&self) -> Box<dyn BootCueSink> {
        // 複製は状態（番号）を引き継がない。
        Box::new(TalkIdSink {
            name: self.name,
            talk_id: None,
            log: Arc::clone(&self.log),
        })
    }

    fn begin_talk(&mut self, talk_id: TalkId) {
        self.talk_id = Some(talk_id);
        self.log
            .lock()
            .expect("log mutex poisoned")
            .push(Event::Begin(self.name, talk_id));
    }
}

impl Drop for TalkIdSink {
    fn drop(&mut self) {
        // 番号を受けた複製（＝トークへ渡したもの）の落ちだけを記録する。登録された原本は数えない。
        if let Some(talk_id) = self.talk_id {
            self.log
                .lock()
                .expect("log mutex poisoned")
                .push(Event::Drop(self.name, talk_id));
        }
    }
}

/// トークの起動ごとに、複製された受け口がその番号を受け取ってから cue を受け、
/// 置き換えでは古いトークの受け口が落ちてから新しい番号が渡る（登録順＝複製の順）。
#[test]
fn each_talk_start_hands_its_id_to_fresh_clones_and_replaced_clones_drop_first() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = |name| -> Box<dyn BootCueSink> {
        Box::new(TalkIdSink {
            name,
            talk_id: None,
            log: Arc::clone(&log),
        })
    };

    let (tx, handle) =
        spawn_dispatcher(kanade_tx, vec![sink("s0"), sink("s1")], test_system_vars());

    let talk_a = TalkId(7);
    let talk_b = TalkId(8);

    // A: 長い待ちで自然には終わらない。最初の Tick で冒頭の cue だけ出す。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_a,
        script: r"\s[1]A\_w[2500]A_END\e".to_string(),
    }))
    .expect("send Start(A)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_000),
    })
    .expect("send Tick(A)");

    // B で置き換える。B も自然には終わらせず、Close（終了させて join する）で落とす
    // ——自然完了は slot を空けるだけで join しないので、落ちの時点が決まらない。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_b,
        script: r"\s[2]B\_w[2500]B_END\e".to_string(),
    }))
    .expect("send Start(B)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_000),
    })
    .expect("send Tick(B)");

    tx.send(DispatcherMsg::Close).expect("send Close");
    run_bounded(
        "dispatcher join after Close",
        Duration::from_secs(5),
        move || {
            handle.join().expect("dispatcher terminates normally");
        },
    );

    let log = log.lock().expect("log mutex poisoned").clone();

    // cue はどれも番号を受けた後の複製に届き、その時点の番号を持つ（番号なしの cue は無い）。
    assert!(
        log.contains(&Event::Emit("s0", Some(talk_a))),
        "A の複製は A の番号で cue を受ける: {log:?}"
    );
    assert!(
        log.contains(&Event::Emit("s0", Some(talk_b))),
        "B の複製は B の番号で cue を受ける: {log:?}"
    );
    assert!(
        !log.iter().any(|e| matches!(e, Event::Emit(_, None))),
        "番号を受ける前の複製に cue が届いてはならない: {log:?}"
    );

    // 番号の受け渡しと落ちの順（cue は除く）: A の 2 つの複製が A を受け、置き換えで両方が
    // 落ちてから B の 2 つの複製が B を受ける。B の複製は Close で落ちる。
    let lifecycle: Vec<Event> = log
        .into_iter()
        .filter(|e| !matches!(e, Event::Emit(..)))
        .collect();
    let (a_drops, rest) = lifecycle[2..].split_at(2);
    assert_eq!(
        lifecycle[..2],
        [Event::Begin("s0", talk_a), Event::Begin("s1", talk_a)],
        "起動ごとに登録順で複製し、直後に番号を渡す: {lifecycle:?}"
    );
    let mut a_drops = a_drops.to_vec();
    a_drops.sort_by_key(|e| format!("{e:?}"));
    assert_eq!(
        a_drops,
        [Event::Drop("s0", talk_a), Event::Drop("s1", talk_a)],
        "置き換えでは古いトークの受け口が新しい番号より先に落ちる: {lifecycle:?}"
    );
    assert_eq!(
        rest[..2],
        [Event::Begin("s0", talk_b), Event::Begin("s1", talk_b)],
        "新しいトークの複製は古い複製が落ちた後に番号を受ける: {lifecycle:?}"
    );
    let mut b_drops = rest[2..].to_vec();
    b_drops.sort_by_key(|e| format!("{e:?}"));
    assert_eq!(
        b_drops,
        [Event::Drop("s0", talk_b), Event::Drop("s1", talk_b)],
        "B の複製は 1 回ずつ落ちる: {lifecycle:?}"
    );

    // 置き換えられた A の終わりは古いトークの知らせとして捨てられ、kanade へは何も届かない。
    assert!(kanade_rx.try_recv().is_err(), "kanade へは何も届かない");
}

//! `ArekaStrictJudges` の答えが `check_script` と同じ物差しの写しであること。

use areka_sakura::strict::{DetectionPoint, StrictJudges};

use super::{ArekaStrictJudges, production};

/// `\!` は正準の台帳の答え（登記の在る組だけ拾われる）をそのまま返す。
#[test]
fn command_handled_follows_the_canonical_ledger() {
    let judges = ArekaStrictJudges::canonical();
    let rows: [(&str, Option<&str>, bool); 6] = [
        ("move", None, true),
        // 印 `\![*]`・`%*` は文字の層が拾う。
        (areka_parsers::sakura::MARKER_COMMAND, None, true),
        ("set", Some("zorder"), true),
        ("open", Some("readme"), true),
        ("open", Some("nosuchthing"), false),
        ("nosuchcommand", None, false),
    ];
    for (name, first, want) in rows {
        assert_eq!(
            judges.command_handled(name, first),
            want,
            "({name}, {first:?}) は拾われるか"
        );
    }
}

/// `\f` は文字の層の分類を検出点へ写し、診ないものは None のまま。
#[test]
fn font_maps_the_text_layer_class_to_detection_points() {
    let judges = ArekaStrictJudges::canonical();
    let rows: [(&[&str], Option<DetectionPoint>); 4] = [
        (&["sub", "1"], Some(DetectionPoint::FontIgnored)),
        (&["colour", "red"], Some(DetectionPoint::FontUnknownKey)),
        (&[], Some(DetectionPoint::FontUnknownKey)),
        (&["bold", "1"], None),
    ];
    for (args, want) in rows {
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(judges.font(&args), want, "{args:?} の検出点");
    }
}

/// 本番の構築へ入れる判定は在る（無いと strict の記録が無言で 0 件になる）。
#[test]
fn production_judges_are_present() {
    let judges = production().expect("本番の判定");
    assert!(!judges.command_handled("nosuchcommand", None));
}

// ---- 長い台本の strict の重さ ----
//
// 4 MiB の台本を strict 真と偽で、本物の駆動（`talk_handler`）を別スレッド（talk スレッドの役）で回す。
// 文字の層の受け手（`EmoTextSink`）の知らせはテストのスレッド（UI スレッドの役）で数える。
// 時間は測らない。知らせの数と、判定・記録をしたスレッドだけを見る。

mod long_script {
    use std::collections::{BTreeMap, HashSet};
    use std::ops::ControlFlow;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread::{self, ThreadId};

    use areka_emo_text::sink::{EmoTextSink, TextMsg};
    use areka_sakura::strict::{DetectionPoint, RECORD_TARGET, StrictHooks, StrictJudges};
    use areka_sakura::{
        ChoiceWaiting, SakuraMsg, StartTalk, SystemVarSnapshot, TalkDone, TalkId, talk_handler,
    };
    use dola::cue::CueSink;
    use log_capture_kit::capture;

    use crate::spine::pump_until_idle;
    use crate::strict_judges::ArekaStrictJudges;

    /// 1 回分: 字・誰も拾わない `\!`・知らない `\f` の鍵・名前の形の `\b`・数の `\b`・改行。
    const UNIT: &str = r"あいうえお\![nosuch]\f[colour,red]\b[a]\b[0]かきく\n";
    const LEN: usize = 4 * 1024 * 1024;

    /// areka の判定を呼んだスレッドを控える覆い。
    struct ThreadJudges {
        inner: ArekaStrictJudges,
        threads: Mutex<HashSet<ThreadId>>,
    }
    impl ThreadJudges {
        fn mark(&self) {
            self.threads
                .lock()
                .expect("threads lock")
                .insert(thread::current().id());
        }
    }
    impl StrictJudges for ThreadJudges {
        fn command_handled(&self, name: &str, first: Option<&str>) -> bool {
            self.mark();
            self.inner.command_handled(name, first)
        }
        fn font(&self, args: &[String]) -> Option<DetectionPoint> {
            self.mark();
            self.inner.font(args)
        }
    }

    struct Notice;
    impl From<TalkDone> for Notice {
        fn from(_: TalkDone) -> Self {
            Notice
        }
    }
    impl From<ChoiceWaiting> for Notice {
        fn from(_: ChoiceWaiting) -> Self {
            Notice
        }
    }

    /// 1 回の再生の観測。
    struct Seen {
        /// 知らせの種類ごとの数（`Upcoming` は中の合図の数も `Upcoming.cues` に足す）。
        msgs: BTreeMap<&'static str, usize>,
        talk_thread: ThreadId,
        /// talk スレッド・UI スレッドで書かれた strict の記録の数。
        talk_records: usize,
        ui_records: usize,
        judge_threads: HashSet<ThreadId>,
    }

    fn play(script: &Arc<str>, strict: bool) -> Seen {
        let msgs = Arc::new(Mutex::new(BTreeMap::<&'static str, usize>::new()));
        let judges = Arc::new(ThreadJudges {
            inner: ArekaStrictJudges::canonical(),
            threads: Mutex::new(HashSet::new()),
        });
        let ((talk_thread, talk_records), ui_events) = capture(|| {
            let counted = Arc::clone(&msgs);
            let (tx, _drain) = areka_actor::spawn_ui(
                "strict-long-script-text",
                move |msg: TextMsg| -> Result<ControlFlow<()>, String> {
                    let mut m = counted.lock().expect("msgs lock");
                    let kind = match &msg {
                        TextMsg::Cue(_) => "Cue",
                        TextMsg::ProbedCue(..) => "ProbedCue",
                        TextMsg::Upcoming(cues) => {
                            *m.entry("Upcoming.cues").or_default() += cues.len();
                            "Upcoming"
                        }
                        TextMsg::Close => "Close",
                        TextMsg::TalkEnded => "TalkEnded",
                        TextMsg::ChoiceWaitBegan => "ChoiceWaitBegan",
                    };
                    *m.entry(kind).or_default() += 1;
                    Ok(ControlFlow::Continue(()))
                },
            );
            let hooks = StrictHooks {
                ghost: "窓際のぱすたさん".into(),
                judges: Arc::clone(&judges) as Arc<dyn StrictJudges>,
            };
            let script = Arc::clone(script);
            let talk = thread::spawn(move || {
                let ((), events) = capture(|| {
                    let sinks: Vec<Box<dyn CueSink + Send>> = vec![Box::new(EmoTextSink::new(tx))];
                    let (done, _rx) = mpsc::channel::<Notice>();
                    let mut handle =
                        talk_handler(sinks, SystemVarSnapshot::default(), done, Some(hooks));
                    let start = StartTalk {
                        strict,
                        ..StartTalk::new(TalkId(1), &*script)
                    };
                    let mut flow = handle(SakuraMsg::Start(start));
                    let mut t = 0.0;
                    while flow.is_continue() {
                        flow = handle(SakuraMsg::Tick(t));
                        t += 1e9;
                    }
                });
                let records = events.iter().filter(|e| e.target == RECORD_TARGET).count();
                (thread::current().id(), records)
            });
            let seen = talk.join().expect("talk スレッドは panic しない");
            pump_until_idle();
            seen
        });
        let ui_records = ui_events
            .iter()
            .filter(|e| e.target == RECORD_TARGET)
            .count();
        let judge_threads = judges.threads.lock().expect("threads lock").clone();
        let msgs = msgs.lock().expect("msgs lock").clone();
        Seen {
            msgs,
            talk_thread,
            talk_records,
            ui_records,
            judge_threads,
        }
    }

    /// 4 MiB の台本で、文字の層へ届く知らせの数は名前の形の `\b` が口付きになる分のほかは strict 真と偽で
    /// 同じで、判定と記録は talk スレッドだけで行われる（UI スレッドの仕事は増えない）。
    #[test]
    fn four_mib_strict_script_keeps_text_messages_and_judges_on_talk_thread() {
        let units = LEN.div_ceil(UNIT.len());
        let script: Arc<str> = UNIT.repeat(units).into();
        assert!(script.len() >= LEN, "4 MiB 以上");

        let plain = play(&script, false);
        let strict = play(&script, true);

        // 名前の形の `\b` は strict でだけ口付きになり、合わせた数は偽の `Cue` と同じ。
        assert_eq!(plain.msgs.get("ProbedCue"), None, "偽では口を付けない");
        assert_eq!(
            strict.msgs.get("ProbedCue"),
            Some(&units),
            "名前の形の \\b の数"
        );
        let mut folded = strict.msgs.clone();
        let probed = folded.remove("ProbedCue").unwrap_or(0);
        *folded.entry("Cue").or_default() += probed;
        assert_eq!(
            folded, plain.msgs,
            "知らせの数（名前の形の \\b の分を除く）"
        );

        // 判定と記録は talk スレッドだけ（誤りは 1 回分に 2 つ・名前の形の `\b` の箱は文字の層が探すが、
        // この受け手は箱を探さないので UI スレッドの記録は 0）。
        assert_eq!(
            (plain.talk_records, plain.ui_records),
            (0, 0),
            "偽は記録しない"
        );
        assert!(plain.judge_threads.is_empty(), "偽は判定を呼ばない");
        assert_eq!(strict.talk_records, 2 * units, "talk スレッドの記録");
        assert_eq!(strict.ui_records, 0, "UI スレッドの記録");
        assert_eq!(
            strict.judge_threads,
            HashSet::from([strict.talk_thread]),
            "判定を呼んだスレッド"
        );
    }
}

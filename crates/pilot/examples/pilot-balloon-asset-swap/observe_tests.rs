use super::*;
use Class::*;

const SP: (u32, u32) = (335, 205);
const SQ: (u32, u32) = (400, 224);

fn sig() -> Signature {
    Signature {
        only_p: vec![],
        only_q: vec![],
        both: vec![],
        size_p: SP,
        size_q: SQ,
    }
}

/// 合成した (絵, 当たり判定, 矩形) の 1 フレームを数える。
fn feed(obs: &mut Observation, pic: Class, hit: Class, tick: u32, size: (u32, u32)) {
    feed_with(obs, pic, hit, tick, size, 1, |_| {});
}

/// `tweak` で tick の記録に測れない印（覆い・拡大率・文字層）を立てる。
fn feed_with(
    obs: &mut Observation,
    pic: Class,
    hit: Class,
    tick: u32,
    (w, h): (u32, u32),
    accumulated: u32,
    tweak: fn(&mut TickRecord),
) {
    let mut t = TickRecord {
        tick,
        pair: PAIR_ASSET,
        ended_qpc: tick as i64 * 100,
        rect: RECT {
            left: 10,
            top: 20,
            right: 10 + w as i32,
            bottom: 20 + h as i32,
        },
        hit,
        h_p: 0.0,
        h_q: 0.0,
        h_both: 0.0,
        covered: false,
        k_ok: true,
        slot_hit: false,
    };
    tweak(&mut t);
    let f = FrameRecord {
        present_qpc: tick as i64 * 100 + 50,
        accumulated,
        tick: Some(tick),
        picture: pic,
        a: 0.0,
        b: 0.0,
        ab_p: 0.0,
        ab_q: 0.0,
    };
    judge_frame(obs, &f, &t);
}

#[test]
fn judge_frame_counts_by_the_rules() {
    let mut o = Observation::new("t P→Q", &sig(), P, Q, 10);
    feed(&mut o, Q, Q, 9, SQ); // 0: 要求より前 → 揃ったにならない
    feed(&mut o, P, Q, 10, SP); // 1: 反映待ち（混在の内訳）
    feed(&mut o, Unmeasurable(Why::Ambiguous), Q, 10, SQ); // 2: 測れない
    feed(&mut o, Q, P, 10, SQ); // 3: 逆向き → 混在・反映待ちではない
    feed(&mut o, Neither, Neither, 10, SP); // 4: 空（混在ではない）
    feed(&mut o, Both, Q, 10, SQ); // 5: 混在・揃う前なので残りではない
    feed(&mut o, Q, Q, 11, SQ); // 6: 揃った
    feed(&mut o, P, Q, 12, SP); // 7: 混在・残り・揃った後なので反映待ちではない
    feed(&mut o, Both, Both, 13, SQ); // 8: 「両方」は混在・残り
    feed(&mut o, Q, Q, 14, SP); // 9: 大きさの食い違い
    feed_with(&mut o, P, P, 15, SP, 1, |t| t.covered = true); // 10: 覆い → 測れない
    feed_with(&mut o, P, Q, 16, SP, 3, |_| {}); // 11: 取りこぼし 2 → 測れない
    assert_eq!(o.settled, Some((6, 11)));
    assert_eq!(
        o.counts,
        Counts {
            frames: 12,
            missed: 2,
            unmeasurable: 3,
            mixed: 5,
            pending: 1,
            empty: 1,
            stale: 2,
            size: 1,
        }
    );

    // from == to（静止）では古い絵の残りを数えない。
    let mut s = Observation::new("t P→P", &sig(), P, P, 1);
    feed(&mut s, P, P, 1, SP);
    feed(&mut s, P, P, 2, SP);
    assert_eq!(s.settled, Some((0, 1)));
    assert_eq!(
        s.counts,
        Counts {
            frames: 2,
            ..Counts::default()
        }
    );
}

/// 拡大率が 1 でない・文字層の子が当たった tick は、混在や空の組でも 4 種に入らず測れないに数える。
#[test]
fn scale_and_slot_hit_are_unmeasurable() {
    let tweaks: [fn(&mut TickRecord); 2] = [|t| t.k_ok = false, |t| t.slot_hit = true];
    for tweak in tweaks {
        let mut o = Observation::new("t P→Q", &sig(), P, Q, 0);
        feed_with(&mut o, P, Q, 1, SP, 1, tweak);
        feed_with(&mut o, Neither, Neither, 2, SP, 1, tweak);
        assert_eq!(
            o.counts,
            Counts {
                frames: 2,
                unmeasurable: 2,
                ..Counts::default()
            }
        );
    }
}

// ---------------------------------------------------------------------
// 観測の窓・較正の合否・床・集計の行（2.4）
// ---------------------------------------------------------------------

fn tick_rec(tick: u32, hit: Class) -> TickRecord {
    TickRecord {
        tick,
        pair: PAIR_ASSET,
        ended_qpc: tick as i64 * 100,
        rect: RECT {
            left: 160,
            top: 160,
            right: 160 + SQ.0 as i32,
            bottom: 160 + SQ.1 as i32,
        },
        hit,
        h_p: 0.0,
        h_q: 0.0,
        h_both: 0.0,
        covered: false,
        k_ok: true,
        slot_hit: false,
    }
}

fn frame_at(tick: u32, picture: Class) -> FrameRecord {
    FrameRecord {
        present_qpc: tick as i64 * 100 + 50,
        accumulated: 1,
        tick: Some(tick),
        picture,
        a: 0.0,
        b: 0.0,
        ab_p: 0.0,
        ab_q: 0.0,
    }
}

/// tick 1..=300 の記録（要求 tick 10 より前は当たり判定 P、以後は Q）。
fn shared_p_then_q() -> Shared {
    Shared {
        ticks: (1..=300)
            .map(|t| tick_rec(t, if t < 10 { P } else { Q }))
            .collect(),
        frames: Vec::new(),
    }
}

fn open_p_to_q(s: &Shared) -> Open {
    let obs = Observation::new("t A0→B0", &sig(), P, Q, 10);
    Open::new(obs, s)
}

#[test]
fn window_starts_at_last_frame_before_request_and_closes_30_ticks_after_settling() {
    let mut s = shared_p_then_q();
    s.frames.push(frame_at(8, P));
    s.frames.push(frame_at(9, P));
    let mut o = open_p_to_q(&s);
    // 要求より前の tick のフレームが遅れて届いたら、それが「直前のフレーム」になる（空）。
    s.frames.push(frame_at(9, Neither));
    s.frames.push(frame_at(10, P)); // 反映待ち
    s.frames.push(frame_at(11, Q)); // 揃った（tick 11）→ 閉じるのは tick 41
    s.frames.push(frame_at(20, Q));
    s.frames.push(frame_at(41, Q)); // 窓の最後の tick は数える
    assert!(
        !o.step(&s, 41),
        "41 より後のフレームも猶予も無いうちは閉じない"
    );
    s.frames.push(frame_at(42, Q)); // 窓の外: 数えないが、閉じてよい印になる
    assert!(o.step(&s, 42));
    let obs = o.close();
    assert_eq!(obs.settled, Some((2, 11)));
    assert!(!obs.no_prev);
    assert_eq!(obs.end, Some(End::Done));
    // 直前（空・当たり P）は空かつ混在、tick 10（絵 P・当たり Q・矩形は Q の寸）は反映待ちかつ大きさの食い違い。
    assert_eq!(
        obs.counts,
        Counts {
            frames: 5,
            mixed: 2,
            pending: 1,
            empty: 1,
            size: 1,
            ..Counts::default()
        }
    );
}

#[test]
fn window_closes_after_grace_when_no_later_frame_arrives() {
    let mut s = shared_p_then_q();
    s.ticks[4].pair = PAIR_FACE; // tick 5 は別の対で記録された
    s.frames.push(frame_at(5, P));
    let mut o = open_p_to_q(&s);
    s.frames.push(frame_at(11, Q)); // 揃った → 閉じるのは tick 41
    assert!(!o.step(&s, 41 + GRACE_TICKS - 1));
    assert!(o.step(&s, 41 + GRACE_TICKS));
    let obs = o.close();
    assert_eq!(obs.end, Some(End::Done));
    assert_eq!(obs.settled, Some((1, 11)));
    // 別の対で判別した直前のフレームは測れない。
    assert_eq!(
        obs.counts,
        Counts {
            frames: 2,
            unmeasurable: 1,
            ..Counts::default()
        }
    );
}

#[test]
fn window_gives_up_180_ticks_after_request_and_keeps_the_last_pair() {
    let mut s = shared_p_then_q();
    s.ticks[99].covered = true; // tick 100 は覆われている
    let mut o = open_p_to_q(&s); // 直前のフレーム無し
    s.frames.push(frame_at(100, Q)); // 絵も当たり判定も到達先だが測れない → 揃ったにならない
    for t in (110..=190).step_by(10) {
        s.frames.push(frame_at(t, P)); // 反映待ちのまま
    }
    assert!(!o.step(&s, 190));
    s.frames.push(frame_at(191, P));
    assert!(o.step(&s, 191));
    let obs = o.close();
    assert!(obs.no_prev);
    assert_eq!(obs.settled, None);
    assert_eq!(
        obs.end,
        Some(End::Incomplete(Some((P, Q, tick_rec(190, Q).rect))))
    );
    assert_eq!(obs.counts.frames, 10);
    assert_eq!(obs.counts.unmeasurable, 1);
    assert_eq!(obs.counts.pending, 9);
    // 猶予でも閉じる（フレームが 1 枚も来ない）。
    let s = shared_p_then_q();
    let mut o = open_p_to_q(&s);
    assert!(!o.step(&s, 10 + GIVE_UP_TICKS + GRACE_TICKS - 1));
    assert!(o.step(&s, 10 + GIVE_UP_TICKS + GRACE_TICKS));
    assert_eq!(o.close().end, Some(End::Incomplete(None)));
}

/// 猶予で閉じた後に届いた窓の中のフレームは、数えずに「遅着」として観測の行に出る。
#[test]
fn frames_arriving_after_a_grace_close_are_reported_as_late() {
    let shared = Arc::new(Mutex::new(shared_p_then_q()));
    let push = |t, pic| shared.lock().unwrap().frames.push(frame_at(t, pic));
    let mut ob = Observer::new(
        Entity::PLACEHOLDER,
        Arc::new([sig(), sig()]),
        shared.clone(),
    );
    ob.open(PAIR_ASSET, Kind::Swap, "t A0→B0", P, Q, 10);
    push(11, Q); // 揃った → 窓の最後は tick 41
    ob.step(11);
    ob.step(41 + GRACE_TICKS); // 窓の外のフレームが来ないまま猶予で閉じる
    assert!(ob.is_closed());
    assert_eq!(ob.done[0].counts.frames, 1); // 直前のフレーム無し・tick 11 の 1 枚
    push(40, Q); // 窓の中 → 遅着
    ob.step(41 + GRACE_TICKS + 1);
    push(41, P); // 窓の中 → 遅着（最終の並べ出しでも拾う）
    ob.summarize(false, 41 + GRACE_TICKS + 2);
    push(41, Q); // 並べ出しの後は数えない
    ob.step(41 + GRACE_TICKS + 3);
    let o = &ob.done[0];
    assert_eq!(o.late_dropped, 2);
    assert_eq!(o.counts.frames, 1, "遅着は 4 種にも frames にも数えない");
    assert!(row(o).contains(" late_dropped=2 "), "{}", row(o));

    // 窓の外のフレームで閉じたときは、後から来るのは窓の外だけなので遅着は 0。
    let shared = Arc::new(Mutex::new(shared_p_then_q()));
    let push = |t| shared.lock().unwrap().frames.push(frame_at(t, Q));
    let mut ob = Observer::new(
        Entity::PLACEHOLDER,
        Arc::new([sig(), sig()]),
        shared.clone(),
    );
    ob.open(PAIR_ASSET, Kind::Swap, "t A0→B0", P, Q, 10);
    push(11);
    push(42);
    ob.step(42);
    assert!(ob.is_closed());
    push(43);
    ob.step(43);
    assert_eq!(ob.done[0].late_dropped, 0);
}

fn closed(kind: Kind, name: &str, counts: Counts) -> Observation {
    Observation {
        kind,
        counts,
        end: Some(End::Done),
        ..Observation::new(name, &sig(), P, Q, 0)
    }
}

#[test]
fn calibration_verdict_follows_design() {
    let c = |f: fn(&mut Counts)| {
        let mut c = Counts {
            frames: 3,
            ..Counts::default()
        };
        f(&mut c);
        c
    };
    let good = vec![
        closed(Kind::Calib(Calib::Static), "calib-static", c(|_| {})),
        closed(Kind::Calib(Calib::Empty), "calib-empty", c(|c| c.empty = 1)),
        closed(Kind::Calib(Calib::Mixed), "calib-mixed", c(|c| c.mixed = 2)),
        closed(Kind::Calib(Calib::Stale), "calib-stale", c(|c| c.stale = 1)),
        closed(Kind::Calib(Calib::Size), "calib-size", c(|c| c.size = 1)),
        // 本番の行は合否に関わらない。
        closed(Kind::Swap, "reattach@update A→B", c(|c| c.mixed = 9)),
    ];
    assert_eq!(calibration_verdict(&good), Verdict::Passed);
    assert_eq!(calibration_verdict(&[]), Verdict::Passed, "較正 0 項は合格");

    let bad = vec![
        closed(
            Kind::Calib(Calib::Static),
            "calib-static",
            c(|c| c.size = 1),
        ),
        closed(
            Kind::Calib(Calib::Static),
            "calib-static-0",
            Counts::default(),
        ),
        closed(Kind::Calib(Calib::Empty), "calib-empty", c(|c| c.mixed = 1)),
        closed(Kind::Calib(Calib::Mixed), "calib-mixed", c(|c| c.empty = 1)),
        closed(Kind::Calib(Calib::Stale), "calib-stale", c(|c| c.size = 1)),
        closed(Kind::Calib(Calib::Size), "calib-size", c(|c| c.stale = 1)),
    ];
    assert_eq!(
        calibration_verdict(&bad),
        Verdict::Failed(vec![
            "calib-static".into(),
            "calib-static-0".into(),
            "calib-empty".into(),
            "calib-mixed".into(),
            "calib-stale".into(),
            "calib-size".into(),
        ])
    );
}

#[test]
fn floor_is_the_larger_pending_of_face_switches() {
    let p = |n| Counts {
        pending: n,
        ..Counts::default()
    };
    assert_eq!(floor(&[closed(Kind::Swap, "x", p(5))]), None);
    assert_eq!(
        floor(&[
            closed(Kind::FaceSwitch, "face-switch@update A0→A2", p(1)),
            closed(Kind::Swap, "x", p(5)),
            closed(Kind::FaceSwitch, "face-switch@update A2→A0", p(2)),
        ]),
        Some(2)
    );
}

#[test]
fn row_spells_out_every_count_including_zeros() {
    let mut o = closed(
        Kind::Swap,
        "remove-then-attach@update A→B",
        Counts {
            frames: 7,
            ..Counts::default()
        },
    );
    o.request_tick = 10;
    o.settled = Some((2, 13));
    assert_eq!(
        row(&o),
        "remove-then-attach@update A→B pair=(A0,B0) 完了 frames=7 missed=0 unmeasurable=0 \
         mixed=0 pending=0 empty=0 stale=0 size=0 late_dropped=0 settled_after_frames=2 \
         settled_after_ticks=3"
    );
    o.settled = None;
    o.no_prev = true;
    o.end = Some(End::Incomplete(Some((P, Both, tick_rec(1, P).rect))));
    let r = row(&o);
    assert!(
        r.contains("未完（最後: 絵=P・当たり判定=Both・矩形=400x224@(160,160)）"),
        "{r}"
    );
    assert!(
        r.contains("settled_after_frames=- settled_after_ticks=-"),
        "{r}"
    );
    assert!(r.ends_with("直前のフレーム無し"), "{r}");
    o.end = None;
    assert!(row(&o).contains(" 途中 "));
}

/// 当たり判定の閾値の境（0.9 と 0.1 は含む）。
#[test]
fn tick_rect_takes_the_move_queued_in_the_same_tick_only() {
    let r = |w| RECT {
        left: 160,
        top: 160,
        right: 160 + w,
        bottom: 365,
    };
    let actual = r(335);
    assert_eq!(effective_rect(Some((7, r(400))), 7, actual), r(400));
    assert_eq!(effective_rect(Some((6, r(400))), 7, actual), actual);
    assert_eq!(effective_rect(None, 7, actual), actual);
}

#[test]
fn hit_rule_thresholds_are_inclusive() {
    let nan = f32::NAN;
    assert_eq!(hit_rule(true, 0.9, 0.9, nan), Both);
    assert_eq!(hit_rule(true, 0.89, 0.9, nan), Unmeasurable(Why::Ambiguous));
    assert_eq!(hit_rule(true, 0.9, 0.1, nan), P);
    assert_eq!(hit_rule(true, 0.9, 0.11, nan), Unmeasurable(Why::Ambiguous));
    assert_eq!(hit_rule(true, 0.1, 0.1, nan), Neither);
    assert_eq!(hit_rule(false, nan, 0.1, 0.9), P);
    assert_eq!(
        hit_rule(false, nan, 0.1, 0.89),
        Unmeasurable(Why::Ambiguous)
    );
}

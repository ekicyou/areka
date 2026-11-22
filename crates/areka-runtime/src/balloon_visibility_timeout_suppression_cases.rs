// =============================================================================
// タイムアウト・抑止分岐の網羅表の本体
//
// 表の読み方・記述語の意味・分けた理由は走査側 `balloon_visibility_timeout_suppression_tests.rs` にある。
// =============================================================================

use super::super::{MeasurementDiscardReason, SuppressionKinds};
use super::{After, Case, EPS, Expect, LATE, Now, Row, Signal, TIMEOUT, Watch};

pub(super) const CASES: &[Case] = &[
    // ⑴⒀ 占有終端は既知の最大より後退しない。
    //
    // 送り手の約束（`talk_lifecycle.rs`）に反した順（トークの終わりの後の後退した占有終端）をわざと流し、
    // 守りが計測の起点を動かさないことを確かめる。起点のずれは立っている満了予定には現れないため、
    // いったん計測を捨ててから立て直すところまで流して初めて観測できる。
    Case {
        name: "占有終端は既知の最大より後退しない",
        rows: &[
            // 会話が始まり、占有終端が届く。現在時刻はまだ占有終端より手前。
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 3, false, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0)],
                after: &[After::NoDeadline],
            },
            // 計測が成り立つ最初の観測が占有終端から LATE 秒遅れて訪れる。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 既知の最大より手前の占有終端が届く。状態は一切動かない。
            Row {
                signals: &[Signal::DisplayEnd(-4.0)],
                scopes: &[(0, 3, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(LATE + 1.0),
                expect: &[],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 全消去で消す対象が無くなり、計測が捨てられる。
            Row {
                signals: &[],
                scopes: &[(0, 0, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(LATE + 2.0),
                expect: &[
                    Expect::Cleared(&[0]),
                    Expect::Discarded {
                        reason: MeasurementDiscardReason::NoVisibleScope,
                        deadline: TIMEOUT,
                    },
                ],
                after: &[After::NoDeadline],
            },
            // 次の可視コンテンツで計測が立ち直る。起点は**後退した値ではなく**既知の最大。
            Row {
                signals: &[],
                scopes: &[(0, 2, false, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(LATE + 3.0),
                expect: &[
                    Expect::Shown(0),
                    Expect::Started {
                        display_end: 0.0,
                        deadline: TIMEOUT,
                    },
                ],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 満了境界の直前。
            Row {
                signals: &[],
                scopes: &[(0, 2, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT - EPS),
                expect: &[],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 満了境界に達したフレーム。消した scope の持ち越し可視は偽になる。
            Row {
                signals: &[],
                scopes: &[(0, 2, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT),
                expect: &[Expect::TimedOut(&[0])],
                after: &[After::NoDeadline, After::PrevVisible(0, false)],
            },
        ],
    },
    // ⑵ 占有終端の引き上げが現在時刻を越えないときは計測を破棄しない。
    //
    // 送り手の約束に反した順（トークの終わりの後の占有終端）をわざと流し、引き上げ先が既に過去なら
    // 守りが計測を捨てず満了予定も延ばさないことを確かめる。
    Case {
        name: "占有終端の引き上げが現在時刻を越えなければ計測は生きたまま",
        rows: &[
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 3, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0)],
                after: &[After::NoDeadline],
            },
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 占有終端が引き上がるが、引き上げ先は現在時刻より手前。
            Row {
                signals: &[Signal::DisplayEnd(LATE - 2.0)],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(LATE + 1.0),
                expect: &[],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 満了境界の直前。満了予定は最初の占有終端起点のまま動いていない。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT - EPS),
                expect: &[],
                after: &[After::Deadline(TIMEOUT)],
            },
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT),
                expect: &[Expect::TimedOut(&[0])],
                after: &[After::NoDeadline],
            },
        ],
    },
    // ⑶ 会話が変わると表示終了信号の欠落の記録が武装し直る（「会話につき 1 回」であって
    //    「プロセスにつき 1 回」ではない）。
    Case {
        name: "会話が変わると表示終了信号の欠落の記録が武装し直る",
        rows: &[
            // 信号が 1 件も届かないまま可視コンテンツが現れた。表示は保つが記録は残す。
            Row {
                signals: &[Signal::TalkStarted],
                scopes: &[(0, 3, false, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0), Expect::SignalMissing],
                after: &[After::NoDeadline],
            },
            // 同じ会話の 2 つ目の scope では記録を繰り返さない。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 2, false, Watch::Off)],
                dragging: false,
                now: Now::At(-4.0),
                expect: &[Expect::Shown(1)],
                after: &[After::NoDeadline],
            },
            // 次の会話が始まり、冒頭の全消去で両方が消える。
            Row {
                signals: &[Signal::TalkStarted],
                scopes: &[(0, 0, true, Watch::Off), (1, 0, true, Watch::Off)],
                dragging: false,
                now: Now::At(-3.0),
                expect: &[Expect::Cleared(&[0, 1])],
                after: &[After::NoDeadline],
            },
            // 次の会話でも信号が届かない。記録は改めて 1 件出る。
            Row {
                signals: &[],
                scopes: &[(0, 4, false, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(-2.0),
                expect: &[Expect::Shown(0), Expect::SignalMissing],
                after: &[After::NoDeadline],
            },
            // 遅れて占有終端とトークの終わりが届く。現在時刻はまだそこへ達していない。
            Row {
                signals: &[Signal::Ended(0.0)],
                scopes: &[(0, 4, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(-1.0),
                expect: &[],
                after: &[After::NoDeadline],
            },
            Row {
                signals: &[],
                scopes: &[(0, 4, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            Row {
                signals: &[],
                scopes: &[(0, 4, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT),
                expect: &[Expect::TimedOut(&[0])],
                after: &[After::NoDeadline],
            },
        ],
    },
    // ⑷⑸ 抑止の記録は「満了予定を過ぎているのに見送った」ときにだけ、1 回の抑止につき 1 件。
    Case {
        name: "抑止の記録は満了後にだけ出て、抑止のたびに武装し直る",
        rows: &[
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 3, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0)],
                after: &[After::NoDeadline],
            },
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 満了予定の直前に抑止が成立。見送るものがまだ無いので無音のまま。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::On)],
                dragging: false,
                now: Now::At(TIMEOUT - EPS),
                expect: &[],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 満了予定に達したが抑止が続いている。ここで初めて見送りが記録される。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::On)],
                dragging: false,
                now: Now::At(TIMEOUT),
                expect: &[Expect::Suppressed {
                    kinds: SuppressionKinds {
                        dragging: false,
                        hover: true,
                    },
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 解除。現在時刻を起点に計り直す。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT + 1.0),
                expect: &[Expect::Restarted {
                    now: TIMEOUT + 1.0,
                    deadline: TIMEOUT + 1.0 + TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT + 1.0 + TIMEOUT)],
            },
            // 二度目の抑止。記録も二度目が出る。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::On)],
                dragging: false,
                now: Now::At(TIMEOUT + 1.0 + TIMEOUT),
                expect: &[Expect::Suppressed {
                    kinds: SuppressionKinds {
                        dragging: false,
                        hover: true,
                    },
                    deadline: TIMEOUT + 1.0 + TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT + 1.0 + TIMEOUT)],
            },
        ],
    },
    // ⑹ 抑止が解けても計測が成り立たなければ取り直さない。
    //
    // 取り直しを無条件に行うと、占有終端が未確立（表示終了信号が届いていない）のまま満了予定が
    // 立ち、「起点不明を理由に任意の時点で消さない」が破れる。
    Case {
        name: "抑止が解けても計測が成り立たなければ取り直さない",
        rows: &[
            // 信号が届かないまま可視コンテンツが現れ、同時にポインタが滞在している。
            Row {
                signals: &[Signal::TalkStarted],
                scopes: &[(0, 3, false, Watch::On)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0), Expect::SignalMissing],
                after: &[After::NoDeadline],
            },
            // 滞在が解けた。占有終端が未確立なので満了予定は立たない。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(-4.0),
                expect: &[],
                after: &[After::NoDeadline],
            },
            // 既定時間を大きく超えても保持したまま。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT * 2.0),
                expect: &[],
                after: &[After::NoDeadline],
            },
        ],
    },
    // ⑺⑻ 抑止条件が同時に成立したときの記録と、一部だけ観測不能な場合。
    Case {
        name: "同時に成立した抑止は 1 件へ全種別が載り、一部の観測不能は残りを殺さない",
        rows: &[
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 3, false, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0)],
                after: &[After::NoDeadline],
            },
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // ドラッグと滞在が同時に成立。記録は 1 件で、種別は 2 つとも立つ。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::On), (1, 0, false, Watch::Off)],
                dragging: true,
                now: Now::At(TIMEOUT),
                expect: &[Expect::Suppressed {
                    kinds: SuppressionKinds {
                        dragging: true,
                        hover: true,
                    },
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 全て解けた。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT + 1.0),
                expect: &[Expect::Restarted {
                    now: TIMEOUT + 1.0,
                    deadline: TIMEOUT + 1.0 + TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT + 1.0 + TIMEOUT)],
            },
            // 滞在の観測が取れないが、ドラッグ中は観測できている。抑止は成立し、
            // 記録の種別はドラッグだけが立つ。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Blind), (1, 0, false, Watch::Off)],
                dragging: true,
                now: Now::At(TIMEOUT + 1.0 + TIMEOUT),
                expect: &[Expect::Suppressed {
                    kinds: SuppressionKinds {
                        dragging: true,
                        hover: false,
                    },
                    deadline: TIMEOUT + 1.0 + TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT + 1.0 + TIMEOUT)],
            },
        ],
    },
    // ⑼ 本フレームに表示した scope への滞在は抑止になる。
    //
    // 滞在は「可視である scope」に限って効かせる規則だが、その可視には**本フレームに表示を
    // 発行した分**が含まれる。観測値だけを見ると、出したばかりのバルーンの上に居るポインタが
    // 抑止にならず、他の scope が読まれないまま消える。
    Case {
        name: "本フレームに表示した scope への滞在は抑止になる",
        rows: &[
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 3, false, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0)],
                after: &[After::NoDeadline],
            },
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 満了予定に達したフレームで scope 1 が喋り出し、そこへポインタが滞在している。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 2, false, Watch::On)],
                dragging: false,
                now: Now::At(TIMEOUT),
                expect: &[
                    Expect::Shown(1),
                    Expect::Suppressed {
                        kinds: SuppressionKinds {
                            dragging: false,
                            hover: true,
                        },
                        deadline: TIMEOUT,
                    },
                ],
                after: &[After::Deadline(TIMEOUT)],
            },
        ],
    },
    // ⑽ 本フレームに全消去で消した scope への滞在は抑止にならない。
    //
    // 消えたバルーンの上にポインタが残っても離脱の通知は届かない。ここを抑止として数えると
    // 恒久的な抑止に固着し、消えないまま固着する側へ倒さない向きに反する。
    Case {
        name: "本フレームに全消去で消した scope への滞在は抑止にならない",
        rows: &[
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 3, false, Watch::Off), (1, 2, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0), Expect::Shown(1)],
                after: &[After::NoDeadline],
            },
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 2, true, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 満了予定に達したフレームで scope 1 の内容が全消去され、そこへポインタが残る。
            // scope 1 は消える側なので抑止に数えず、scope 0 は満了で消える。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off), (1, 0, true, Watch::On)],
                dragging: false,
                now: Now::At(TIMEOUT),
                expect: &[Expect::Cleared(&[1]), Expect::TimedOut(&[0])],
                after: &[
                    After::NoDeadline,
                    After::PrevVisible(0, false),
                    After::PrevVisible(1, false),
                ],
            },
        ],
    },
    // ⑾ 現在時刻が分からないフレームでは抑止の持ち越しを書き換えない（解けたと読まない）。
    //
    // 時刻不明のフレームで抑止を「解けた」と読むと、解除エッジがそこで消費されてしまい、
    // 時刻が戻ったフレームで積み残しの満了予定がそのまま成立して消える——抑止したまま
    // 消えたのと同じことになる。
    Case {
        name: "現在時刻が分からないフレームでは抑止の持ち越しを書き換えない",
        rows: &[
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 3, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[Expect::Shown(0)],
                after: &[After::NoDeadline],
            },
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(LATE),
                expect: &[Expect::Started {
                    display_end: 0.0,
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 滞在で見送る。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::On)],
                dragging: false,
                now: Now::At(TIMEOUT),
                expect: &[Expect::Suppressed {
                    kinds: SuppressionKinds {
                        dragging: false,
                        hover: true,
                    },
                    deadline: TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 現在時刻が分からないフレームで滞在が解けた。計測にも抑止の持ち越しにも触れない。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::Unknown,
                expect: &[],
                after: &[After::Deadline(TIMEOUT)],
            },
            // 時刻が戻ったフレームで解除エッジが改めて立ち、現在時刻起点で計り直す。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT + 2.0),
                expect: &[Expect::Restarted {
                    now: TIMEOUT + 2.0,
                    deadline: TIMEOUT + 2.0 + TIMEOUT,
                }],
                after: &[After::Deadline(TIMEOUT + 2.0 + TIMEOUT)],
            },
        ],
    },
    // ⑿ 満了の対象が本フレーム表示分しか無いときは見送り、満了予定を残す。
    //
    // 表示と計測開始が同一フレームで成立し、その満了予定が既に過ぎている形（表示終了信号を
    // 失った会話でだけ起きる）。ここで満了予定まで捨てると、出したバルーンが以後どの時刻でも
    // 消えなくなる。捨てずに残すことで、次のフレームで改めて満了する。
    Case {
        name: "満了の対象が本フレーム表示分だけなら見送り、次フレームで消える",
        rows: &[
            // 占有終端は届いたが、この時点で可視コンテンツは 1 つも無い。
            Row {
                signals: &[Signal::TalkStarted, Signal::Ended(0.0)],
                scopes: &[(0, 0, false, Watch::Off)],
                dragging: false,
                now: Now::At(-5.0),
                expect: &[],
                after: &[After::NoDeadline],
            },
            // 満了予定を過ぎた時点で可視コンテンツが現れた。表示と計測開始が同時に成立し、
            // その満了予定は既に過去だが、本フレームに表示した scope は対象から外す。
            Row {
                signals: &[],
                scopes: &[(0, 3, false, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT + 1.0),
                expect: &[
                    Expect::Shown(0),
                    Expect::Started {
                        display_end: 0.0,
                        deadline: TIMEOUT,
                    },
                ],
                after: &[After::Deadline(TIMEOUT), After::PrevVisible(0, true)],
            },
            // 次のフレームで改めて満了する（満了予定を立て直さずに、残っていた分で消える）。
            Row {
                signals: &[],
                scopes: &[(0, 3, true, Watch::Off)],
                dragging: false,
                now: Now::At(TIMEOUT + 2.0),
                expect: &[Expect::TimedOut(&[0])],
                after: &[After::NoDeadline, After::PrevVisible(0, false)],
            },
        ],
    },
];

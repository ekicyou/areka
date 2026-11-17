// =============================================================================
// ZOrderCueSink の自己選別と指令送出の決定論テスト（task 3.1・要件 1.7／4.4／8.3／11.2）
//
// この層が受け持つのは 2 つだけである——「自分宛のタグだけを拾うこと」と「拾ったものを
// 解釈せずそのまま送ること」。ゆえに檻も 2 方向から挟む: 受理する 2 組は指令が出ること、
// 惜しい組（`\![set,他]`／`\![reset,他]`／第 1 引数の無い `set`／名前がくっついた
// `setzorder`／別コマンド名に `zorder` を添えたもの）は 1 本も出ないこと。
//
// 「出ない」の主張は空虚になりやすいので、同じ受け口へ**先に受理される指令を 1 本流して
// おき**、惜しい組を浴びせた後で受け口の中身がその 1 本ちょうどであることを確かめる形に
// してある（受け口そのものが壊れていれば先の 1 本が届かず赤くなる）。
//
// ログ捕捉は硬化機構の唯一の定義元 `log-capture-kit` の捕捉窓へ委譲する
// （`move_cue_move_severity_log_tests.rs` と同じ流儀。機序の解説は同ファイル冒頭）。
// =============================================================================

use super::*;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

// ---------------------------------------------------------------- 道具立て

/// `\![name,tokens...]` の汎用キャリア cue を組む（正準形＝`Custom` の String 配列）。
fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

/// 開封できない `Custom`（`params` が String 配列でない）を宛名だけ差し替えて組む。
///
/// `DynamicValue::Null` は配列ではないので `as_command_carrier()` は `None` を返す。
/// それでも宛名（`Custom{command}`）は読めるため、宛名規律の分岐へ入る。
fn broken_custom_cue(actor: &str, command: &str) -> TalkCue {
    let cue = TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::Custom {
            command: command.to_string(),
            params: DynamicValue::Null,
        },
        duration: 0.0,
    };
    assert_eq!(
        cue.command.as_command_carrier(),
        None,
        "params=Null は開封できない形である（宛名で severity を分ける枝に入る前提）"
    );
    cue
}

/// キャリアでない cue（文字・演技・待ち）の一覧。
fn non_carrier_cues() -> Vec<TalkCue> {
    [
        CueCommand::Text("あひる".into()),
        CueCommand::Emote { key: "0".into() },
        CueCommand::Wait,
    ]
    .into_iter()
    .map(|command| TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command,
        duration: 0.0,
    })
    .collect()
}

/// 受け口に溜まっている指令を全て取り出す。
fn drain(rx: &Receiver<ZOrderDirective>) -> Vec<ZOrderDirective> {
    rx.try_iter().collect()
}

/// 指令 1 本だけを取り出す（0 本・2 本以上はその場で落とす）。
fn only_one(rx: &Receiver<ZOrderDirective>) -> ZOrderDirective {
    let got = drain(rx);
    assert_eq!(got.len(), 1, "指令はちょうど 1 本のはず: {got:?}");
    got.into_iter().next().unwrap()
}

/// `\![set,zorder,1,0]` 相当の受理される指令（受け口が生きていることの目印にも使う）。
fn accepted_set() -> ZOrderDirective {
    ZOrderDirective::Set {
        tokens: vec!["1".to_string(), "0".to_string()],
    }
}

/// クロージャ実行中に**現在のスレッド**で発火した記録を 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち指定した水準（`"WARN"`／`"DEBUG"`）の件数を数える。
fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|line| line.contains(&needle)).count()
}

/// 註釈の行を落とす——説明文に書いてあるだけの綴りを「在る」と数えないため
/// （`tick_gate_config_producers_tests.rs` と同じ流儀）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// コメントを除いたソースにおける字面の位置（無ければその場で落とす）。
///
/// 檻が「在る・無い」だけでなく**前後関係**を主張するための道具である。
fn index_of(code: &str, needle: &str) -> usize {
    code.find(needle)
        .unwrap_or_else(|| panic!("コードに `{needle}` の字面が無い（檻の前提が崩れている）"))
}

// ---------------------------------------------------------------- 受理する 2 組

/// `\![set,zorder,1,0]` は重なり指定の指令になり、第 1 引数より後ろのトークンだけを運ぶ。
#[test]
fn t_zcs1_set_zorder_becomes_a_set_directive_carrying_the_element_tokens() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    sink.emit(carrier_cue("0", "set", &["zorder", "1", "0"]));
    assert_eq!(
        only_one(&rx),
        accepted_set(),
        "選別子 `zorder` は指令に含めず、要素のトークンだけを運ぶ"
    );
}

/// `\![reset,zorder]` は重なり解除の指令になる。
#[test]
fn t_zcs2_reset_zorder_becomes_a_reset_directive() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    sink.emit(carrier_cue("0", "reset", &["zorder"]));
    assert_eq!(only_one(&rx), ZOrderDirective::Reset);
}

/// この層はトークンを解釈しない——明示記法も省略記法も解釈できない綴りも空文字も、
/// 書かれたまま同じ並びで運ばれる（解釈は台帳の状態が要るので後段の担当）。
#[test]
fn t_zcs3_tokens_are_forwarded_verbatim_without_interpretation() {
    let raw = ["balloon1", "s1", "Surface0", "", "xyz", "0"];
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);

    let mut params = vec!["zorder"];
    params.extend_from_slice(&raw);
    sink.emit(carrier_cue("0", "set", &params));

    let ZOrderDirective::Set { tokens } = only_one(&rx) else {
        panic!("重なり指定の指令のはず");
    };
    assert_eq!(
        tokens,
        raw.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
        "解釈も選別も並べ替えもせず、書かれたトークンをそのまま運ぶ"
    );
}

// ---------------------------------------------------------------- 要件 1.7

/// **要件 1.7**: タグを実行したスコープを読まない。実行スコープの値を変えても、
/// 送り出される指令は 1 バイトも変わらない（`MoveCueSink` との意図的な差はここ）。
#[test]
fn t_zcs4_executing_scope_does_not_change_the_directive() {
    // 本体側・相方側・追加キャラ・数値ですらない演者名・空——実際に届き得る値を並べる。
    let actors = ["0", "1", "2", "9", "sakura", ""];

    let mut set_directives = Vec::new();
    let mut reset_directives = Vec::new();
    for actor in actors {
        let (tx, rx) = channel::<ZOrderDirective>();
        let mut sink = ZOrderCueSink::new(tx);
        sink.emit(carrier_cue(actor, "set", &["zorder", "1", "0"]));
        set_directives.push(only_one(&rx));

        let (tx, rx) = channel::<ZOrderDirective>();
        let mut sink = ZOrderCueSink::new(tx);
        sink.emit(carrier_cue(actor, "reset", &["zorder"]));
        reset_directives.push(only_one(&rx));
    }

    for (actor, got) in actors.iter().zip(&set_directives) {
        assert_eq!(
            *got,
            accepted_set(),
            "実行スコープ {actor:?} でも重なり指定の指令は同じ（要件 1.7）"
        );
    }
    for (actor, got) in actors.iter().zip(&reset_directives) {
        assert_eq!(
            *got,
            ZOrderDirective::Reset,
            "実行スコープ {actor:?} でも重なり解除の指令は同じ（要件 1.7）"
        );
    }
}

/// 要件 1.7 を字面からも押さえる: 本番コードは実行スコープの欄（`cue.actor`）を読まない。
///
/// 振る舞いの檻（`t_zcs4`）だけでは足りない。`let _executing_scope = cue.actor.as_str();`
/// のように**振る舞いを 1 つも変えない**読み取りを足すと、振る舞い側の檻は全て緑のまま
/// 通ってしまう。読んでいないこと自体をここで固定する。
///
/// 対照は**コメントを除いた側**へ当てる——コードにしか現れない字面が生き残っていることを
/// 主張しないと、`code_only` がコード行まで削り落としたときに「綴りが無い」が自動的に真に
/// なり、檻が何も守らないまま緑になる（註釈側の綴りを数えるだけでは対照にならない）。
#[test]
fn t_zcs5_production_code_never_reads_the_executing_scope() {
    let src = include_str!("zorder_cue.rs");
    let code = code_only(src);
    assert!(
        code.contains("as_command_carrier"),
        "前提: コードにしか現れない字面が残っている（`code_only` の削り過ぎの検出）"
    );
    assert!(
        src.contains("cue.actor"),
        "前提: 何を禁じているかが註釈に書き残されている（この禁止は空文ではない）"
    );
    assert!(
        !code.contains("actor"),
        "本番コードは実行スコープの欄を読まない（要件 1.7・`MoveCueSink` との差）"
    );
}

// ---------------------------------------------------------------- 惜しい組

/// **要件 4.4／11.2**: 名前が合っていても選別子が違えば担当外——1 本も送らず、
/// 受け口の中身は先に流した 1 本のままである（＝重なりの状態を変えていない）。
#[test]
fn t_zcs6_near_miss_selectors_send_nothing() {
    let near_misses: [(&str, &[&str]); 8] = [
        ("set", &["surface", "1", "0"]), // \![set,他]
        ("set", &["balloon", "1"]),      // 同上
        ("set", &["windowsize", "1"]),   // 同上
        ("reset", &["balloon"]),         // \![reset,他]
        ("reset", &["surface"]),         // 同上
        ("set", &[]),                    // 第 1 引数そのものが無い
        ("reset", &[]),                  // 同上（引数無しの全体解除は範囲外）
        ("setzorder", &["1", "0"]),      // 名前と選別子がくっついた綴り
    ];

    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    // 受け口が生きている目印を先に 1 本置く（「出ない」の主張を空虚にしないため）。
    sink.emit(carrier_cue("0", "set", &["zorder", "1", "0"]));

    for (name, tokens) in near_misses {
        sink.emit(carrier_cue("0", name, tokens));
    }

    assert_eq!(
        drain(&rx),
        vec![accepted_set()],
        "惜しい組はどれも指令を増やさない（受理された 1 本だけが残る・要件 4.4／11.2）"
    );
}

/// **要件 11.2**: 担当外のコマンド名は、第 1 引数が `zorder` であっても読み飛ばす
/// （`\![move,zorder,…]` のような綴りを横取りしない）。
#[test]
fn t_zcs7_other_command_names_send_nothing_even_with_the_zorder_selector() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    sink.emit(carrier_cue("0", "set", &["zorder", "1", "0"]));

    for name in ["move", "bind", "raise", "zorder", "unknownfoo"] {
        sink.emit(carrier_cue("0", name, &["zorder", "1", "0"]));
    }

    assert_eq!(
        drain(&rx),
        vec![accepted_set()],
        "担当外の名前は選別子が合っていても読み飛ばす（要件 11.2）"
    );
}

/// キャリアでない cue（文字・演技・待ち）は担当外——1 本も送らない。
#[test]
fn t_zcs8_non_carrier_cues_send_nothing() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    sink.emit(carrier_cue("0", "set", &["zorder", "1", "0"]));

    for cue in non_carrier_cues() {
        sink.emit(cue);
    }

    assert_eq!(
        drain(&rx),
        vec![accepted_set()],
        "キャリアでない cue は指令を増やさない（要件 11.2）"
    );
}

// ---------------------------------------------------------------- 宛名規律

/// 宛名規律: 開封できない `Custom` のうち**自分宛**（`set`／`reset`）は警告 1 本。
///
/// 開封できない形では選別子が読めないので、宛名は名前だけで判ずる。今この 2 つの名前を
/// 担当しているのは本 sink だけであり、壊れ物を報せる責任はこちらにある。
#[test]
fn t_zcs9_broken_own_address_warns_once() {
    for command in ["set", "reset"] {
        let (tx, rx) = channel::<ZOrderDirective>();
        let mut sink = ZOrderCueSink::new(tx);
        let logs = capture_logs(|| sink.emit(broken_custom_cue("0", command)));

        assert_eq!(
            count_level(&logs, "WARN"),
            1,
            "自分宛（{command}）の開けない荷物は警告 1 本: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            0,
            "自分宛の壊れ物は良性の読み飛ばしではない: {logs:?}"
        );
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "壊れた荷物からは指令を送らない"
        );
    }
}

/// 宛名規律: 同じ壊れ方でも**他人宛**なら警告を出さず、良性の読み飛ばしとして記録する
/// （報せる責任は宛名の担当者の側にある）。
#[test]
fn t_zcs10_broken_other_address_records_without_warning() {
    for command in ["move", "bind", "noexist"] {
        let (tx, rx) = channel::<ZOrderDirective>();
        let mut sink = ZOrderCueSink::new(tx);
        let logs = capture_logs(|| sink.emit(broken_custom_cue("0", command)));

        assert_eq!(
            count_level(&logs, "WARN"),
            0,
            "他人宛（{command}）の壊れ物で警告を出さない: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            1,
            "他人宛でも黙って捨てず良性の読み飛ばしとして記録する: {logs:?}"
        );
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }
}

/// 宛名規律が空虚でない証: **同じ壊れ方**のまま宛名だけを入れ替えると、水準が
/// 警告と良性の読み飛ばしの間で入れ替わる（「壊れ物は一律で警告」への退行を捕まえる）。
#[test]
fn t_zcs11_addressee_alone_decides_the_severity() {
    let own = capture_logs(|| {
        let (tx, _rx) = channel::<ZOrderDirective>();
        let mut sink = ZOrderCueSink::new(tx);
        sink.emit(broken_custom_cue("0", "set"));
    });
    let other = capture_logs(|| {
        let (tx, _rx) = channel::<ZOrderDirective>();
        let mut sink = ZOrderCueSink::new(tx);
        sink.emit(broken_custom_cue("0", "noexist"));
    });

    assert_eq!(
        (count_level(&own, "WARN"), count_level(&own, "DEBUG")),
        (1, 0),
        "自分宛: 警告 1・良性 0（{own:?}）"
    );
    assert_eq!(
        (count_level(&other, "WARN"), count_level(&other, "DEBUG")),
        (0, 1),
        "他人宛: 警告 0・良性 1（{other:?}）"
    );
}

// ---------------------------------------------------------------- 要件 8.3

/// **要件 8.3**: 読み飛ばすどの経路でも、理由を残さないまま黙って諦めない。
#[test]
fn t_zcs12_every_skip_leaves_a_record() {
    let mut skipped: Vec<TalkCue> = vec![
        carrier_cue("0", "set", &["surface", "1"]),
        carrier_cue("0", "reset", &["balloon"]),
        carrier_cue("0", "set", &[]),
        carrier_cue("0", "move", &["zorder", "1", "0"]),
        broken_custom_cue("0", "set"),
        broken_custom_cue("0", "noexist"),
    ];
    skipped.extend(non_carrier_cues());

    for cue in skipped {
        let label = format!("{:?}", cue.command);
        let (tx, _rx) = channel::<ZOrderDirective>();
        let mut sink = ZOrderCueSink::new(tx);
        let logs = capture_logs(|| sink.emit(cue));
        assert!(
            !logs.is_empty(),
            "読み飛ばしは必ず理由を残す（要件 8.3）: {label}"
        );
    }
}

/// 解除に余分なトークンが付いていても解除として受理し、運ばなかったことを記録する
/// （正典の `\![reset,zorder]` は引数を取らない）。
#[test]
fn t_zcs13_reset_with_trailing_tokens_is_still_reset_and_is_recorded() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let mut sink = ZOrderCueSink::new(tx);
    let logs = capture_logs(|| sink.emit(carrier_cue("0", "reset", &["zorder", "1", "0"])));

    assert_eq!(only_one(&rx), ZOrderDirective::Reset);
    assert!(
        !logs.is_empty(),
        "運ばなかったトークンがあることを記録する（要件 8.3）: {logs:?}"
    );
}

// ---------------------------------------------------------------- 起床の旗・受け渡し

/// 指令を送ったら画面更新を促す旗を立てる——**立てる場所**まで押さえる。
///
/// 旗はプロセスで 1 組しか無く areka のテストは並列に走るので、動的に読むと他のテストの
/// 巻き添えで揺れる（`tick_gate_config_producers_tests.rs` は「立っていないはず」を主張
/// しない、と明記している）。テストの独立性（要件 10.3）を壊さずに置き場所を固定するため、
/// ここではコメントを除いたソース上の**前後関係**を主張する。
///
/// 「字面が在る」だけでは足りない。旗を `emit` の先頭（自己選別より前）へ持ち上げた実装は、
/// 担当外の文字・演技・待ちの cue が届くたびに重なりの起床を叩き、表示に変化の無い巡を
/// 省く門を実質無効にする。それでいて送り出しの振る舞いは 1 つも変わらないので、他の檻は
/// 全て緑のまま通ってしまう。
#[test]
fn t_zcs14_the_zorder_wake_is_marked_after_the_self_selection() {
    let code = code_only(include_str!("zorder_cue.rs"));

    // 呼出はちょうど 1 つ——後ろに残したまま先頭でも叩く形を塞ぐ。
    assert_eq!(
        code.matches("tick_wake::mark(").count(),
        1,
        "旗を立てる呼出はちょうど 1 つ（要件 7.4 の生産者）"
    );
    assert!(
        code.contains("tick_wake::mark(tick_wake::ZORDER)"),
        "立てるのは重なりの旗（要件 7.4 の生産者）"
    );

    let mark_at = index_of(&code, "tick_wake::mark(");
    let skip_at = index_of(&code, "担当外のコマンドを良性に読み飛ばす");
    let send_at = index_of(&code, "self.tx.send(");
    assert!(
        skip_at < mark_at,
        "旗は自己選別より後ろに置く（担当外を読み飛ばす分岐={skip_at}・旗={mark_at}）。先頭へ持ち上げると担当外の cue が届くたびに起床させてしまう"
    );
    assert!(
        send_at < mark_at,
        "旗は送り出しの後に立てる（送出={send_at}・旗={mark_at}・`MoveCueSink` と同じ順序）"
    );
}

/// 受け口は複製できる（配送側は台本ごとに複製する）。複製から送っても同じ受け口へ届く。
#[test]
fn t_zcs15_clone_reaches_the_same_receiver() {
    let (tx, rx) = channel::<ZOrderDirective>();
    let sink = ZOrderCueSink::new(tx);
    let mut cloned = sink.clone();
    cloned.emit(carrier_cue("0", "set", &["zorder", "1", "0"]));
    assert_eq!(only_one(&rx), accepted_set());
}

/// 受け口が閉じていても台本を殺さない——記録を残して継続する（非 panic）。
#[test]
fn t_zcs16_disconnected_receiver_is_recorded_and_does_not_panic() {
    let (tx, rx) = channel::<ZOrderDirective>();
    drop(rx);
    let mut sink = ZOrderCueSink::new(tx);
    let logs = capture_logs(|| sink.emit(carrier_cue("0", "set", &["zorder", "1", "0"])));
    assert_eq!(
        count_level(&logs, "WARN"),
        1,
        "送れなかったことを警告として残す（黙って捨てない・要件 8.3）: {logs:?}"
    );
}

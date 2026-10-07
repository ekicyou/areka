//! 正準台帳と `emo2_boot` の 8 つの受け口の自己選別の一致の檻（areka-P0-mcp-author-tools
//! 要件 3.2・3.11・design「`\!` の表の仕上げ」の「一致のテスト」）。
//!
//! 表は宣言するだけで、受け口は自分の名前と第 1 引数で自己選別する（両者は依存しない）。
//! `check_script` は表を引いて「知らない `\!`」を答えるので、表と受け口の選別がずれると
//! 「検査だけが通して再生で落ちる」「検査だけが落として再生で通る」が生まれる。ここで固定する。
//!
//! 8 つの受け口をそれぞれ本物の送信端つきで組み、1 つの `\!` の cue を全員へ配って、どの送信端に
//! 指令が届いたかを見る（配送は全員へ同じ cue を配る broadcast なので、本番と同じ形）。
//!
//! 1. 表が担当を言う各行（受け口の既存のテストから取った引数つきの見本）→ その担当の受け口だけに
//!    届く。担当が `emo2_boot` の外（seriko の `bind`・文字の層の `\f`・台本の組み立ての
//!    `set,choicetimeout`・プロパティの書き込み）の行は、8 つのどれにも届かない。
//! 2. 表に出てくる名前の全部 × 表に出てくる第 1 引数の全部（＋第 1 引数なし＋どこにも無い語）の
//!    掛け合わせのうち、表に無い組 → 8 つのどれにも届かない（受け口が表に無い組を拾えば赤）。
//!    引数の列は「第 1 引数＋同じ名前の見本の第 2 引数から先（尾）」で組む。切替・書庫の受け口は
//!    選別子の後ろに引数が無いと何も送らないので、尾を付けないと選別がゆるんでも赤にならない。
//!    尾の無い形（第 1 引数だけ）も配り、第 1 引数なしは裸と空の第 1 引数（`""`＋尾）の両方を配る。
//!
//! 見本の一覧は表の行とちょうど同じ組を持つこと自体も判定する（行を足して見本を足し忘れれば赤）。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::mpsc::channel;

use areka_ghost::prop_sink::PROP_SET_CUE_NAME;
use areka_parsers::sakura::JUMP_TAG_CARRIER;
use areka_sakura::contract::FONT_TAG_CARRIER;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};

use super::super::change_cue::ChangeCueSink;
use super::super::install_cue::{InstallCueSink, StartFetch};
use super::super::move_cue::MoveCueSink;
use super::super::readme_cue::ReadmeCueSink;
use super::super::switch_cue::SwitchCueSink;
use super::super::update_cue::UpdateCueSink;
use super::super::user_break_cue::{NoUserBreakCueSink, NoUserBreakSignal};
use super::super::zorder_cue::ZOrderCueSink;
use super::{CommandConsumer, ConsumerLedger, LedgerKey};

// ---------------------------------------------------------------- 見本

/// 絶対パスの書庫（`install_cue_tests.rs` の見本と同じ形。fs は読まない）。
const ABSOLUTE: &str = r"C:\areka-install-cue-test\in\hana.nar";

/// 表の各行の見本＝（名前, 選別子, 引数の列）。引数の列は第 1 引数から書く。
///
/// どれも担当の受け口（またはクレート）の既存のテストが受理を確かめている形から取った。
fn samples() -> Vec<(&'static str, Option<&'static str>, Vec<&'static str>)> {
    vec![
        // move_cue_move_sink_tests.rs の move_carrier_sends_directive
        ("move", None, vec!["-353", "", "", "0", "base", "base"]),
        // areka-seriko の actor_bind_loop_tests.rs
        ("bind", None, vec!["腕", "伸び", "1"]),
        // zorder_cue_tests.rs の t_zcs1／t_zcs2
        ("set", Some("zorder"), vec!["zorder", "1", "0"]),
        ("reset", Some("zorder"), vec!["zorder"]),
        // readme_cue_tests.rs の bare_open_readme_sends_exactly_one_request
        ("open", Some("readme"), vec!["readme"]),
        // readme_cue_tests.rs の open_forms_and_jump_send_classified_destination
        // （areka-P0-open-external-tags の 5 組と `\j`）
        ("open", Some("file"), vec!["file", "notepad.exe"]),
        (
            "open",
            Some("browser"),
            vec!["browser", "https://example.com/"],
        ),
        ("open", Some("explorer"), vec!["explorer", r"C:\x"]),
        ("open", Some("editor"), vec!["editor", "a.txt", "3"]),
        ("open", Some("mailer"), vec!["mailer", "a@b.c"]),
        (JUMP_TAG_CARRIER, None, vec!["http://example.com/"]),
        // user_break_cue_tests.rs の enter／leave
        ("enter", Some("nouserbreakmode"), vec!["nouserbreakmode"]),
        ("leave", Some("nouserbreakmode"), vec!["nouserbreakmode"]),
        // areka-emo-text の actor_choice_contract_tests.rs
        (FONT_TAG_CARRIER, None, vec!["height", "40"]),
        // change_cue_tests.rs／switch_cue_tests.rs
        ("change", Some("ghost"), vec!["ghost", "B"]),
        ("change", Some("shell"), vec!["shell", "S"]),
        ("change", Some("balloon"), vec!["balloon", "Y"]),
        // install_cue_tests.rs の script_string_becomes_one_script_request_with_the_path
        (
            "execute",
            Some("install"),
            vec!["install", "path", ABSOLUTE],
        ),
        // update_cue_tests.rs の four_entry_forms_become_the_expected_request
        ("updatebymyself", None, vec![]),
        ("update", None, vec!["all"]),
        ("updateother", None, vec!["--balloon=B", "--shell=S"]),
        // areka-sakura の compile_choice_timeout_tests.rs
        ("set", Some("choicetimeout"), vec!["choicetimeout", "5000"]),
        // areka-ghost の prop_sink.rs の boot_count_carrier_is_persisted
        (PROP_SET_CUE_NAME, None, vec!["areka.boot.count", "1"]),
    ]
}

// ---------------------------------------------------------------- 道具立て

/// 表の担当のうち、`emo2_boot` の 8 つの受け口のどれか（外の担当は `None`）。
///
/// 網羅の `match` にしておく——担当の種類を足した人は、ここで受け口か外かを決めることになる。
fn emo2_sink_of(consumer: CommandConsumer) -> Option<CommandConsumer> {
    match consumer {
        CommandConsumer::MoveSink
        | CommandConsumer::ZOrderSink
        | CommandConsumer::ReadmeSink
        | CommandConsumer::UserBreakSink
        | CommandConsumer::ChangeSink
        | CommandConsumer::SwitchSink
        | CommandConsumer::InstallSink
        | CommandConsumer::UpdateSink => Some(consumer),
        CommandConsumer::Seriko
        | CommandConsumer::TextLayer
        | CommandConsumer::ScriptCompile
        | CommandConsumer::PropertyWrite => None,
    }
}

/// `\![name,args...]` の汎用キャリア cue を組む（各受け口のテストと同じ正準形）。
fn carrier_cue(name: &str, args: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, args.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

/// 8 つの受け口を本物の送信端つきで組み、1 つの cue を全員へ配って、指令が届いた受け口を返す。
///
/// 中断の無効化の受け口は、どの cue でも最初に話の始まり・落とすと話の終わりを送るので、
/// 出入り（`Enter`／`Leave`）だけを「届いた」と数える。書庫の受け口は取得の口を差し替え、
/// 取得を起こしたことも「届いた」と数える（ネットへは出ない）。
fn reached(name: &str, args: &[&str]) -> Vec<CommandConsumer> {
    let (move_tx, move_rx) = channel();
    let (zorder_tx, zorder_rx) = channel();
    let (readme_tx, readme_rx) = channel();
    let (user_break_tx, user_break_rx) = channel();
    let (change_tx, change_rx) = channel();
    let (switch_tx, switch_rx) = channel();
    let (install_tx, install_rx) = channel();
    let (fetch_tx, fetch_rx) = channel::<String>();
    let (update_tx, update_rx) = channel();
    let fetch: StartFetch = Arc::new(move |url, _| {
        fetch_tx.send(url).expect("取得の記録を受け取れる");
    });
    let mut sinks: Vec<Box<dyn CueSink>> = vec![
        Box::new(MoveCueSink::new(move_tx)),
        Box::new(ZOrderCueSink::new(zorder_tx)),
        Box::new(ReadmeCueSink::new(readme_tx)),
        Box::new(NoUserBreakCueSink::new(user_break_tx)),
        Box::new(ChangeCueSink::new(change_tx)),
        Box::new(SwitchCueSink::new(switch_tx)),
        Box::new(InstallCueSink::with_fetch(install_tx, fetch)),
        Box::new(UpdateCueSink::new(update_tx)),
    ];
    for sink in &mut sinks {
        sink.emit(carrier_cue(name, args));
    }
    drop(sinks);

    let hits = [
        (move_rx.try_iter().count() > 0, CommandConsumer::MoveSink),
        (
            zorder_rx.try_iter().count() > 0,
            CommandConsumer::ZOrderSink,
        ),
        (
            readme_rx.try_iter().count() > 0,
            CommandConsumer::ReadmeSink,
        ),
        (
            user_break_rx
                .try_iter()
                .any(|s| matches!(s, NoUserBreakSignal::Enter | NoUserBreakSignal::Leave)),
            CommandConsumer::UserBreakSink,
        ),
        (
            change_rx.try_iter().count() > 0,
            CommandConsumer::ChangeSink,
        ),
        (
            switch_rx.try_iter().count() > 0,
            CommandConsumer::SwitchSink,
        ),
        (
            install_rx.try_iter().count() + fetch_rx.try_iter().count() > 0,
            CommandConsumer::InstallSink,
        ),
        (
            update_rx.try_iter().count() > 0,
            CommandConsumer::UpdateSink,
        ),
    ];
    hits.into_iter()
        .filter_map(|(hit, consumer)| hit.then_some(consumer))
        .collect()
}

/// 表の行の組（名前, 選別子）の全部。
fn ledger_keys(ledger: &ConsumerLedger) -> BTreeSet<LedgerKey> {
    ledger.table.keys().cloned().collect()
}

// ---------------------------------------------------------------- 檻

/// 見本の一覧は表の行とちょうど同じ組を持つ（行を足して見本を足し忘れても、見本だけ残っても赤）。
#[test]
fn samples_cover_exactly_the_ledger_rows() {
    let ledger = ConsumerLedger::canonical();
    let sampled: BTreeSet<LedgerKey> = samples()
        .into_iter()
        .map(|(name, selector, _)| (name.to_string(), selector.map(str::to_string)))
        .collect();
    assert_eq!(
        sampled,
        ledger_keys(&ledger),
        "見本の組と正準台帳の行の組は同じであること"
    );
    for (name, selector, args) in samples() {
        if let Some(selector) = selector {
            assert_eq!(
                args.first(),
                Some(&selector),
                "{name}: 選別子つきの行の見本は、第 1 引数がその選別子であること"
            );
        }
    }
}

/// ⑴ 表が担当を言う各行の見本は、その担当の受け口だけに届く（外の担当なら 8 つのどれにも
/// 届かない）。8 つの受け口のどれもが少なくとも 1 つの見本で届くことも確かめる
/// （届いたかを見る仕掛けが受け口ごとに生きていること）。
#[test]
fn each_ledger_row_reaches_only_its_own_sink() {
    let ledger = ConsumerLedger::canonical();
    let mut seen = Vec::new();
    for (name, selector, args) in samples() {
        let consumer = ledger
            .consumer_of(name, args.first().copied())
            .unwrap_or_else(|| panic!("見本 ({name}, {selector:?}) は表に担当がある"));
        let expected: Vec<CommandConsumer> = emo2_sink_of(consumer).into_iter().collect();
        assert_eq!(
            reached(name, &args),
            expected,
            "\\![{name},{}] は表の担当 {consumer:?} の受け口だけに届く",
            args.join(",")
        );
        for consumer in expected {
            if !seen.contains(&consumer) {
                seen.push(consumer);
            }
        }
    }
    assert_eq!(
        seen.len(),
        8,
        "8 つの受け口のどれにも届く見本がある: {seen:?}"
    );
}

/// ⑵ 表に出てくる名前 × 表に出てくる第 1 引数（＋第 1 引数なし＋どこにも無い語）の掛け合わせの
/// うち、表に担当の無い組は 8 つのどれにも届かない。
///
/// 引数の列は第 1 引数の後ろに同じ名前の見本の尾（第 2 引数から先）を付ける。尾が無いと
/// 切替先の名前や書庫のパスを要る受け口へ届きえず、選別がゆるんでも赤にならないため。
#[test]
fn pairs_without_a_ledger_row_reach_no_sink() {
    let ledger = ConsumerLedger::canonical();
    let keys = ledger_keys(&ledger);
    let mut firsts: BTreeSet<Option<&str>> = keys.iter().map(|(_, s)| s.as_deref()).collect();
    firsts.insert(None);
    firsts.insert(Some("noexistword"));

    // 名前ごとの尾の集合（見本の第 2 引数から先）。尾の無い形も必ず含める。
    let mut tails: BTreeMap<&str, BTreeSet<Vec<&str>>> = BTreeMap::new();
    for (name, _, args) in samples() {
        let entry = tails.entry(name).or_default();
        entry.insert(Vec::new());
        entry.insert(args.get(1..).unwrap_or_default().to_vec());
    }

    let mut checked = 0;
    for (name, name_tails) in &tails {
        for first in &firsts {
            if ledger.consumer_of(name, *first).is_some() {
                continue;
            }
            // 第 1 引数なしは裸（`\![name]`）と空の第 1 引数（`\![name,,尾…]`）の両方。
            let heads: Vec<Vec<&str>> = match first {
                Some(first) => vec![vec![*first]],
                None => vec![Vec::new(), vec![""]],
            };
            for head in &heads {
                for tail in name_tails {
                    if head.is_empty() && !tail.is_empty() {
                        continue; // 裸に尾を付けると尾の先頭が第 1 引数になってしまう
                    }
                    let args = [head.as_slice(), tail.as_slice()].concat();
                    assert_eq!(
                        reached(name, &args),
                        Vec::<CommandConsumer>::new(),
                        "表に担当の無い ({name}, {first:?}) の \\![{name},{}] はどの受け口にも届かない",
                        args.join(",")
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 0, "表に担当の無い組を少なくとも 1 つ確かめた");
}

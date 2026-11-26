//! 正準台帳と `runtime` の 11 の受け口の自己選別の一致の檻。
//!
//! 11 の受け口をそれぞれ本物の送信端つきで組み、1 つの `\!` の cue を全員へ配って、どの送信端に
//! 指令が届いたかを見る（配送は全員へ同じ cue を配る broadcast なので、本番と同じ形）。
//!
//! 1. 表が担当を言う各行（見本つき）→ その担当の受け口だけに届く。担当が `runtime` の外の行は
//!    11 のどれにも届かない。
//! 2. 表に出てくる名前 × 第 1 引数（＋第 1 引数なし＋どこにも無い語）の掛け合わせのうち、
//!    表に無い組 → 11 のどれにも届かない。引数の列は「第 1 引数＋同じ名前の見本の尾」で組む。
//!    尾の無い形（第 1 引数だけ）も配り、第 1 引数なしは裸と空の第 1 引数（`""`＋尾）の両方を配る。
//! 3. `set,position` 系・`reload` の仲間など、表に無い名前も 11 のどれにも届かない。
//!
//! 見本の組が表の行と一致することの判定は、`areka-runtime` の同じ名前のファイルにある。

use dola::cue::CueSink;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::mpsc::channel;

use super::super::change_cue::ChangeCueSink;
use super::super::move_cue::MoveCueSink;
use super::super::reload_cue::ReloadCueSink;
use super::super::talk_clock::TalkClock;
use super::super::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};
use super::super::user_break_cue::{NoUserBreakCueSink, NoUserBreakSignal};
use super::super::window_state_cue::WindowStateCueSink;
use super::super::zorder_cue::ZOrderCueSink;
use super::test_support::{carrier_cue, ledger_keys, samples};
use super::{CommandConsumer, ConsumerLedger};
use crate::install::install_cue::InstallCueSink;
use crate::update::update_cue::UpdateCueSink;
use areka_runtime::switch_cue::SwitchCueSink;
use areka_session::readme::readme_cue::ReadmeCueSink;

// ---------------------------------------------------------------- 道具立て

/// 表の担当のうち、`runtime` の 11 の受け口のどれか（外の担当は `None`）。
///
/// 網羅の `match` にしておく——担当の種類を足した人は、ここで受け口か外かを決めることになる。
fn runtime_sink_of(consumer: CommandConsumer) -> Option<CommandConsumer> {
    match consumer {
        CommandConsumer::MoveSink
        | CommandConsumer::ZOrderSink
        | CommandConsumer::ReadmeSink
        | CommandConsumer::UserBreakSink
        | CommandConsumer::ChangeSink
        | CommandConsumer::SwitchSink
        | CommandConsumer::InstallSink
        | CommandConsumer::UpdateSink
        | CommandConsumer::LifecycleSink
        | CommandConsumer::ReloadSink
        | CommandConsumer::WindowStateSink => Some(consumer),
        CommandConsumer::Seriko
        | CommandConsumer::TextLayer
        | CommandConsumer::ScriptCompile
        | CommandConsumer::PropertyWrite
        | CommandConsumer::PropertyRead => None,
    }
}

/// 11 の受け口を本物の送信端つきで組み、1 つの cue を全員へ配って、指令が届いた受け口を返す。
///
/// 中断の無効化の受け口とバルーンの寿命の受け口は、どの cue でも最初に話の始まり・落とすと話の終わりを
/// 送るので、前者は出入り（`Enter`／`Leave`）、後者は待ち時間の指定（`BalloonTimeout`）だけを「届いた」と数える。
/// 書庫の受け口の `url` の腕は URL の要求を送るだけ（取りに行くのは窓口なので、ネットへは出ない）。
fn reached(name: &str, args: &[&str]) -> Vec<CommandConsumer> {
    let (move_tx, move_rx) = channel();
    let (zorder_tx, zorder_rx) = channel();
    let (readme_tx, readme_rx) = channel();
    let (user_break_tx, user_break_rx) = channel();
    let (change_tx, change_rx) = channel();
    let (switch_tx, switch_rx) = channel();
    let (install_tx, install_rx) = channel();
    let (update_tx, update_rx) = channel();
    let (lifecycle_tx, lifecycle_rx) = channel();
    let (reload_tx, reload_rx) = channel();
    let (window_state_tx, window_state_rx) = channel();
    let mut sinks: Vec<Box<dyn CueSink>> = vec![
        Box::new(MoveCueSink::new(move_tx)),
        Box::new(ZOrderCueSink::new(zorder_tx)),
        Box::new(ReadmeCueSink::new(readme_tx)),
        Box::new(NoUserBreakCueSink::new(user_break_tx)),
        Box::new(ChangeCueSink::new(change_tx)),
        Box::new(SwitchCueSink::new(switch_tx)),
        Box::new(InstallCueSink::with_readonly(install_tx, false)),
        Box::new(UpdateCueSink::new(update_tx)),
        Box::new(BalloonLifecycleSink::new(
            lifecycle_tx,
            TalkClock::new(Arc::new(|| dola::TimePoint::ZERO)),
        )),
        Box::new(ReloadCueSink::new(reload_tx)),
        Box::new(WindowStateCueSink::new(window_state_tx)),
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
            install_rx.try_iter().count() > 0,
            CommandConsumer::InstallSink,
        ),
        (
            update_rx.try_iter().count() > 0,
            CommandConsumer::UpdateSink,
        ),
        (
            lifecycle_rx
                .try_iter()
                .any(|s| matches!(s, TalkLifecycleSignal::BalloonTimeout(_))),
            CommandConsumer::LifecycleSink,
        ),
        (
            reload_rx.try_iter().count() > 0,
            CommandConsumer::ReloadSink,
        ),
        (
            window_state_rx.try_iter().count() > 0,
            CommandConsumer::WindowStateSink,
        ),
    ];
    hits.into_iter()
        .filter_map(|(hit, consumer)| hit.then_some(consumer))
        .collect()
}

/// ⑴ 表が担当を言う各行の見本は、その担当の受け口だけに届く（外の担当なら 11 のどれにも
/// 届かない）。11 の受け口のどれもが少なくとも 1 つの見本で届くことも確かめる
/// （届いたかを見る仕掛けが受け口ごとに生きていること）。
#[test]
fn each_ledger_row_reaches_only_its_own_sink() {
    let ledger = ConsumerLedger::canonical();
    let mut seen = Vec::new();
    for (name, selector, args) in samples() {
        let consumer = ledger
            .consumer_of(name, args.first().copied())
            .unwrap_or_else(|| panic!("見本 ({name}, {selector:?}) は表に担当がある"));
        let expected: Vec<CommandConsumer> = runtime_sink_of(consumer).into_iter().collect();
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
        11,
        "11 の受け口のどれにも届く見本がある: {seen:?}"
    );
}

/// ⑵ 表に出てくる名前 × 表に出てくる第 1 引数（＋第 1 引数なし＋どこにも無い語）の掛け合わせの
/// うち、表に担当の無い組は 11 のどれにも届かない。
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

/// 位置の指定・解除（`\![set,position,…]`・`\![reset,position]`）は SSP 2.9.18 も受けないので、
/// 表に行が無く、11 のどれにも届かない。
#[test]
fn set_and_reset_position_reach_no_sink() {
    let ledger = ConsumerLedger::canonical();
    for (name, args) in [
        ("set", &["position", "100", "100", "1"][..]),
        ("reset", &["position"][..]),
    ] {
        assert_eq!(ledger.consumer_of(name, Some("position")), None, "{name}");
        assert_eq!(
            reached(name, args),
            Vec::<CommandConsumer>::new(),
            "\\![{name},{}] はどの受け口にも届かない",
            args.join(",")
        );
    }
}

/// reload の受け口が拾わない `\![reload,makoto]`・`\![reload,aigraph]` は表に行が無く、11 の受け口の
/// どれにも届かない。
#[test]
fn reload_makoto_and_aigraph_reach_no_sink() {
    let ledger = ConsumerLedger::canonical();
    for first in ["makoto", "aigraph"] {
        assert_eq!(ledger.consumer_of("reload", Some(first)), None, "{first}");
        assert_eq!(
            reached("reload", &[first]),
            Vec::<CommandConsumer>::new(),
            "\\![reload,{first}] はどの受け口にも届かない"
        );
    }
}

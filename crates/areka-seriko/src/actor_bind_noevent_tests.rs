//! `\![bind-noevent,…]` が `\![bind,…]` と同じ結果・同じ水準の記録になり、記録の `name` の欄だけで
//! 分かれることの檻（spec: script-small-tags 要件 3.1〜3.3・3.5・3.6・3.8・3.9・8.1）。
//!
//! 同じ初めの状態から 2 つの名前で同じ引数を当て、着せ替えの集合・出た表示の指令・記録（`name` の
//! 欄を除く）が等しいことを見る。出力の先は表示の指令の受け口だけ（SHIORI への道は無い・3.6）。

use super::test_support::*;
use super::*;
use crate::bind::tests::groups_of;
use crate::output::{DisplayCommand, MockSurfaceOutput};
use areka_sakura::{ActorKey, CueCommand, TalkCue};

/// カテゴリ「ボール」は定義の順におじさん・男の子・ギャル・妹（排他・既定なし）。「目」は `mustselect`。
const SHELL: &str = "charset,UTF-8\r\n\
sakura.bindgroup13.name,ボール,おじさん\r\n\
sakura.bindgroup12.name,ボール,男の子\r\n\
sakura.bindgroup11.name,ボール,ギャル\r\n\
sakura.bindgroup10.name,ボール,妹\r\n\
sakura.bindgroup20.name,目,ノーマル\r\n\
sakura.bindgroup21.name,目,ジト\r\n\
sakura.bindgroup20.default,1\r\n\
sakura.bindoption0.group,目,mustselect\r\n";

const NAMES: [&str; 2] = [BIND_COMMAND, BIND_NOEVENT_COMMAND];

/// 1 つの名前で 1 つの引数を当てた結果（集合・出た指令・記録の行）。
#[derive(Debug, PartialEq)]
struct Outcome {
    binds: Vec<u32>,
    shown: Vec<DisplayCommand>,
    logs: Vec<String>,
}

/// スコープ 0 に面 2100 を出し、`before` の引数を `bind` で当ててから、`name` で `tokens` を当てる。
fn run(scope: &str, before: &[&[&str]], name: &str, tokens: &[&str]) -> Outcome {
    let groups = groups_of(SHELL);
    let resolver = tiny_resolver();
    let bind_resolver = BindResolver::from_groups(&groups);
    let mut states = ScopeStates::new(crate::DefaultBinds::from_groups(&groups));
    let key = ActorKey::from(scope);
    states.apply(&key, SurfaceTarget::Show(2100));
    let mut loop_runtime = inert_runtime();
    let mut out = MockSurfaceOutput::new();
    let records = out.records();
    let carrier = |name: &str, tokens: &[&str]| {
        SerikoMsg::Cue(TalkCue {
            at: 0.0,
            actor: key.clone(),
            command: CueCommand::command_carrier(
                name,
                tokens.iter().map(|t| t.to_string()).collect(),
            ),
            duration: 0.0,
        })
    };
    for b in before {
        let msg = carrier(BIND_COMMAND, b);
        let _ = handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            msg,
        );
    }
    records.lock().expect("records mutex poisoned").clear();
    let msg = carrier(name, tokens);
    let (logs, flow) = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            msg,
        )
    });
    assert_eq!(flow, ControlFlow::Continue(()), "どの形でも処理は続く");
    let shown = records.lock().expect("records mutex poisoned").clone();
    Outcome {
        binds: states.current_binds(&key).ids().to_vec(),
        shown,
        logs: logs.lines().map(str::to_owned).collect(),
    }
}

/// 形ごとの (前もって当てる引数, 当てる引数, 出るはずの水準, 終わりの集合, 出る指令の数)。
type Case = (
    &'static [&'static [&'static str]],
    &'static [&'static str],
    &'static str,
    &'static [u32],
    usize,
);

const CASES: &[Case] = &[
    // 3.1: パーツ名つきの 1。
    (&[], &["ボール", "妹", "1"], "level=INFO", &[10, 20], 1),
    // 3.2: パーツ名つきの 0。
    (
        &[&["ボール", "妹", "1"]],
        &["ボール", "妹", "0"],
        "level=INFO",
        &[20],
        1,
    ),
    // 3.9: トグル形（脱いでいれば着る・着ていれば脱ぐ）。
    (&[], &["ボール", "妹"], "level=INFO", &[10, 20], 1),
    (
        &[&["ボール", "妹", "1"]],
        &["ボール", "妹", ""],
        "level=INFO",
        &[20],
        1,
    ),
    // 3.8: カテゴリ単位の 0 は全部脱がせる。
    (
        &[&["ボール", "男の子", "1"]],
        &["ボール", "", "0"],
        "level=INFO",
        &[20],
        1,
    ),
    // カテゴリ単位の 1 は定義の順に着せ、排他で最後の妹が残る。変わった分は最後に 1 回だけ出す。
    (&[], &["ボール", "", "1"], "level=INFO", &[10, 20], 1),
    // カテゴリ単位のトグル。
    (
        &[&["ボール", "おじさん", "1"]],
        &["ボール"],
        "level=INFO",
        &[10, 20],
        1,
    ),
    // 3.8 ただし書き: mustselect の `,,0` は脱がない（パーツ名つきの 0 と同じ warn）。
    (&[], &["目", "", "0"], "level=WARN", &[20], 0),
    (&[], &["目", "ノーマル", "0"], "level=WARN", &[20], 0),
    // 名前の解決の失敗・名前の無いカテゴリへのカテゴリ単位形。
    (&[], &["ボール", "無い", "1"], "level=ERROR", &[20], 0),
    (&[], &["無い", "", "0"], "level=ERROR", &[20], 0),
    // 壊れた入力（カテゴリの欠け・数値の破損）。
    (&[], &[], "level=ERROR", &[20], 0),
    (&[], &["ボール", "妹", "2"], "level=ERROR", &[20], 0),
];

/// 3.1〜3.3・3.8・3.9: どの形も 2 つの名前で同じ集合・同じ指令・同じ記録（`name` の欄を除く）。
#[test]
fn bind_noevent_matches_bind_for_every_form() {
    for (before, tokens, level, binds, shown) in CASES {
        let [bind, noevent] = NAMES.map(|name| run("0", before, name, tokens));
        assert_eq!(bind.binds, *binds, "{tokens:?} の終わりの集合");
        assert_eq!(bind.shown.len(), *shown, "{tokens:?} の指令の数: {bind:?}");
        assert!(
            bind.logs.iter().any(|l| l.contains(level)),
            "{tokens:?} は {level} を残す: {bind:?}"
        );
        assert_eq!(noevent.binds, bind.binds, "{tokens:?} の集合が同じ");
        assert_eq!(noevent.shown, bind.shown, "{tokens:?} の指令が同じ");
        let renamed: Vec<String> = noevent
            .logs
            .iter()
            .map(|l| l.replace("bind-noevent", "bind"))
            .collect();
        assert_eq!(
            renamed, bind.logs,
            "{tokens:?} の記録が name の欄のほかは同じ"
        );
    }
}

/// 3.5: 着せ替えの道の記録の全行に `name` の欄があり、2 つの名前が分かれる。
#[test]
fn every_bind_log_line_carries_its_name() {
    for (before, tokens, _, _, _) in CASES {
        for name in NAMES {
            let outcome = run("0", before, name, tokens);
            let bind_lines: Vec<&String> = outcome
                .logs
                .iter()
                .filter(|l| l.contains("seriko: ") && !l.contains("level=TRACE"))
                .collect();
            assert!(!bind_lines.is_empty(), "{tokens:?} は記録を残す");
            for line in bind_lines {
                assert!(
                    line.contains(&format!("name=\"{name}\"")),
                    "{name} {tokens:?} の行に name の欄: {line}"
                );
            }
        }
    }
}

/// 実機の目印: 適用の行に `name=bind-noevent` が乗る（design「記録の形」）。
#[test]
fn applied_marker_names_bind_noevent() {
    let outcome = run("0", &[], BIND_NOEVENT_COMMAND, &["ボール", "妹", "1"]);
    let marker: Vec<&String> = outcome
        .logs
        .iter()
        .filter(|l| l.contains("seriko: bind 適用"))
        .collect();
    assert_eq!(marker.len(), 1, "{:?}", outcome.logs);
    assert!(marker[0].contains("name=\"bind-noevent\""), "{}", marker[0]);
}

/// 3.6: 出力は表示の指令だけ（seriko は SHIORI への受け口を持たない）。カテゴリ単位の 1 は途中の姿を出さず 1 件。
#[test]
fn bind_noevent_outputs_only_one_display_command() {
    let outcome = run("0", &[], BIND_NOEVENT_COMMAND, &["ボール", "", "1"]);
    assert_eq!(
        outcome
            .logs
            .iter()
            .filter(|l| l.contains("seriko: bind 適用"))
            .count(),
        4,
        "4 つのパーツを 1 つずつ当てて記録する: {:?}",
        outcome.logs
    );
    match outcome.shown.as_slice() {
        [DisplayCommand::Show { binds, .. }] => assert_eq!(binds.ids(), &[10, 20]),
        other => panic!("表示の指令 1 件だけ: {other:?}"),
    }
}

/// 数でないスコープは 2 つの名前とも同じ warn（D7）。
#[test]
fn bind_noevent_unmapped_scope_warns_like_bind() {
    let [bind, noevent] = NAMES.map(|name| run("x", &[], name, &["ボール", "妹", "1"]));
    assert!(
        bind.logs.iter().any(|l| l.contains("level=WARN")),
        "{bind:?}"
    );
    assert!(bind.shown.is_empty());
    let renamed: Vec<String> = noevent
        .logs
        .iter()
        .map(|l| l.replace("bind-noevent", "bind"))
        .collect();
    assert_eq!(renamed, bind.logs);
}

/// 名前は「2 つのどちらか」で選ぶ。前置きが同じ別の名前は担当外（debug・何もしない）。
#[test]
fn prefix_alike_name_is_not_bind() {
    let outcome = run("0", &[], "bind-noeventx", &["ボール", "妹", "1"]);
    assert_eq!(outcome.binds, vec![20]);
    assert!(outcome.shown.is_empty());
    assert!(
        outcome.logs.iter().all(|l| l.contains("level=DEBUG")),
        "{:?}",
        outcome.logs
    );
}

/// 宛名 `bind-noevent` の壊れた運び物は `bind` と同じ warn（自分宛の壊れ物・D8④）。
#[test]
fn noncanonical_bind_noevent_warns_like_bind() {
    for name in NAMES {
        let mut states = fresh_states();
        let mut out = MockSurfaceOutput::new();
        let mut loop_runtime = inert_runtime();
        let (logs, _) = capture_logs_flow(|| {
            handle_message(
                &tiny_resolver(),
                &BindResolver::empty(),
                &mut states,
                &mut loop_runtime,
                &mut out,
                SerikoMsg::Cue(TalkCue {
                    at: 0.0,
                    actor: ActorKey::from("0"),
                    command: CueCommand::Custom {
                        command: name.to_owned(),
                        params: dola::DynamicValue::Null,
                    },
                    duration: 0.0,
                }),
            )
        });
        assert_eq!(logs.matches("level=WARN").count(), 1, "{name}: {logs}");
        assert!(logs.contains(&format!("name=\"{name}\"")), "{name}: {logs}");
    }
}

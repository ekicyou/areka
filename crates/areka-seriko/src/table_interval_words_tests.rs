//! 間隔の語 `sometimes`・`rarely` の読み替えの檻
//! （spec: areka-P0-shell-implicit-surface 要件 11.1〜11.7・7.10・7.11・7.12）。
//!
//! 入力は `surfaces.txt` の本文そのもの（解析を経る実経路）で組む。構造体を直に積むと
//! 「解析が `Interval::Other(語)` として運んでくる」ところが檻の外へ落ちるためである。
//! 検体は共有の受け口 `sample_test_support` 経由でだけ受ける（要件 7.11）。

use areka_emo_compose::EmoWorld;
use areka_parsers::charset::{DefaultEncoding, decode};
use log_capture_kit::{LineFormat, capture_lines};

use super::{AnimationTable, LoopAnimation, LoopTrigger};
use crate::sample_test_support::konnoyayame_shell_root;

/// 間隔の語 1 つだけを差し替えた面 0（コマ 2 本・終端は停止センチネル）の `surfaces.txt` 本文。
fn surfaces_txt_with(interval: &str) -> String {
    format!(
        "surface0\n{{\nanimation0.interval,{interval}\n\
         animation0.pattern0,overlay,1031,0,93,103\n\
         animation0.pattern1,overlay,-1,50,0,0\n}}\n"
    )
}

/// 本文から表を組み、同じ走行の `tracing` 出力を 1 イベント 1 行で添えて返す。
fn table_and_logs(text: &str) -> (AnimationTable, String) {
    let shell = areka_parsers::shell::parse(text);
    let world = EmoWorld::build(&shell);
    let (table, lines) = capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world(&world)
    });
    (table, lines.join("\n"))
}

/// 間隔の語 `interval` で組んだ面 0 の採録済みアニメ列。
fn animations_for(interval: &str) -> Vec<LoopAnimation> {
    table_and_logs(&surfaces_txt_with(interval))
        .0
        .animations(0)
        .to_vec()
}

/// `sometimes` は `random,2` と**同じ表の項目**になる（要件 11.1）。
#[test]
fn sometimes_records_the_same_entry_as_random_2() {
    let rewritten = animations_for("sometimes");
    assert_eq!(
        rewritten,
        animations_for("random,2"),
        "sometimes は random,2 と同じ引き金・同じコマ列で採録される"
    );
    assert_eq!(rewritten.len(), 1, "面 0 のアニメ 0 が採録される");
    assert_eq!(rewritten[0].trigger, LoopTrigger::Random { k: 2 });
}

/// `rarely` は `random,4` と**同じ表の項目**になる（要件 11.2）。
#[test]
fn rarely_records_the_same_entry_as_random_4() {
    let rewritten = animations_for("rarely");
    assert_eq!(
        rewritten,
        animations_for("random,4"),
        "rarely は random,4 と同じ引き金・同じコマ列で採録される"
    );
    assert_eq!(rewritten.len(), 1);
    assert_eq!(rewritten[0].trigger, LoopTrigger::Random { k: 4 });
}

/// 他の語と、大文字の `Sometimes` は今日どおり採らず、元の語を `debug!` に残す（要件 11.4）。
/// 比べ方は小文字の完全一致なので、大文字は読み替えの対象にならない。
#[test]
fn other_interval_words_are_not_recorded_and_keep_their_original_vocab() {
    for word in ["always", "runonce", "Sometimes", "Rarely"] {
        let (table, logs) = table_and_logs(&surfaces_txt_with(word));
        assert!(
            table.animations(0).is_empty(),
            "{word} は再生の対象にしない: {logs}"
        );
        assert!(
            logs.contains(&format!("vocab=\"{word}\"")),
            "{word} は元の語つきで debug! に残る: {logs}"
        );
    }
}

/// 読み替えたときは元の語と読み替え先の `k` が `debug!` から読める（要件 11.7）。
#[test]
fn rewrite_is_debug_logged_with_the_original_vocab() {
    let (_, logs) = table_and_logs(&surfaces_txt_with("sometimes"));
    assert!(
        logs.contains("level=DEBUG"),
        "読み替えは debug! で記録: {logs}"
    );
    assert!(logs.contains("vocab=\"sometimes\""), "元の語が残る: {logs}");
    assert!(logs.contains("k=2"), "読み替え先の k が載る: {logs}");

    let (_, rarely) = table_and_logs(&surfaces_txt_with("rarely"));
    assert!(
        rarely.contains("vocab=\"rarely\"") && rarely.contains("k=4"),
        "rarely も同じ形で残る: {rarely}"
    );
}

/// 検体 `konnoyayame` の `surfaces.txt` から組んだ表で、面 0 のアニメ 0 が採られる
/// （今日は 0 件＝まばたきが再生されない・要件 11.5・7.10）。
#[test]
fn konnoyayame_surface0_animation0_is_recorded() {
    let path = konnoyayame_shell_root().join("surfaces.txt");
    let bytes = std::fs::read(&path).expect("検体の surfaces.txt は読める");
    let (table, logs) = table_and_logs(&decode(&bytes, DefaultEncoding::Ansi));

    let anims = table.animations(0);
    assert_eq!(
        anims.len(),
        1,
        "konnoyayame の面 0 はまばたき 1 本が採録される: {logs}"
    );
    assert_eq!(anims[0].id, 0);
    assert_eq!(
        anims[0].trigger,
        LoopTrigger::Random { k: 2 },
        "interval,sometimes は random,2 相当"
    );
    let targets: Vec<i64> = anims[0].frames.iter().map(|f| f.surface_id).collect();
    assert!(
        targets.contains(&1031) && targets.contains(&1032) && targets.contains(&1033),
        "まばたきのコマは面 1031・1032・1033 を指す: {targets:?}"
    );
}

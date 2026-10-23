//! 同梱を飛ばしたインストールの警告の口のテスト（areka-P0-emily-ghost-verification 要件 1.5・
//! 1.12・7.1・7.2・design「手続きの警告」）。
//!
//! 偽の口の上で本物の `areka-nar` を一時の根へ走らせ、要件 7.1 の 6 場面ごとに口
//! `warn_skipped` の呼び出しの件数（飛ばした場面は 1・全部在る場面は 0）と題名・本文を見る。
//! あわせて、同じ書庫で 2 つ飛ばしても呼び出しは 1 回で本文に 2 行並ぶこと、イベントの列が
//! 今の成功と同じ形で口がその後ろにあること、締めの知らせが「閉じた」でも口が 1 回呼ばれる
//! ことを見る。

use areka_nar::InstallKind;
use sample_ghost_kit::{NarBuilder, install_txt};
use temp_path_kit::TempPath;

use super::procedure_test_support::{
    Call, FakePorts, RUNNING, RUNNING_SAKURA, ghost_nar, order_of, write_nar,
};
use super::*;

/// 書庫のファイル名（本文の 1 行目に載る）。
const ARCHIVE: &str = "skip.nar";
/// 本文の最後の行（設計の逐語）。
const SUBSTITUTE: &str = "入らなかったバルーンの代わりに、入っている別のバルーンが使われます。";

/// 本物の書庫 1 本を偽の口で一周させる。
fn run_one(ports: &mut FakePorts, builder: NarBuilder) -> Vec<ArchiveEnd> {
    let dir = TempPath::under_target("install-procedure-skip-nar");
    let nar = write_nar(&dir, ARCHIVE, builder);
    run_order(&order_of(&[&nar]), ports)
}

/// 飛ばした場面の口の呼び出しがちょうど 1 回で、題名と本文が `title`・`lines` のとおり。
fn assert_one_warning(ports: &FakePorts, title: &str, lines: &[&str]) {
    let warned = ports.warned();
    assert_eq!(
        warned.len(),
        1,
        "飛ばした書庫 1 本につき口は 1 回: {:?}",
        ports.calls
    );
    assert_eq!(warned[0].0, title);
    assert_eq!(warned[0].1, lines.join("\n"));
}

/// 同梱の行を足したシェル（`accept` は起動中のゴースト）。
fn shell_with(companions: &[&str]) -> NarBuilder {
    let accept_line = format!("accept,{RUNNING_SAKURA}");
    let mut lines = vec![
        "charset,UTF-8",
        "type,shell",
        "name,きがえ",
        "directory,dress",
        accept_line.as_str(),
    ];
    lines.extend_from_slice(companions);
    NarBuilder::new()
        .file("install.txt", &install_txt(&lines))
        .done()
        .file("descript.txt", b"charset,UTF-8\r\nname,Dress\r\n")
        .done()
}

// ---- 6 場面（要件 7.1・7.2） ----

/// ⑴ 無印の同梱の取り出し元が無い → 口 1 回。イベントの列は今の成功と同じ形で、口はその後ろ（1.5）。
#[test]
fn a_plain_skipped_companion_warns_once_after_the_closing_notice() {
    let mut ports = FakePorts::new();
    let ends = run_one(
        &mut ports,
        ghost_nar(
            "newbie",
            &[
                "balloon.directory,kaku",
                "balloon.source.directory,src-kaku",
            ],
        ),
    );

    let at = ports.under_root("ghost/newbie").display().to_string();
    let title = "インストールの警告 - あたらしい";
    let body = [
        "書庫: skip.nar",
        "入った物: ゴースト あたらしい",
        "入らなかったバルーン: kaku（書庫に取り出し元 src-kaku が無かったため）",
        SUBSTITUTE,
    ];
    assert!(
        matches!(ends.as_slice(), [ArchiveEnd::Installed(_)]),
        "成功: {ends:?}"
    );
    assert_eq!(
        ports.calls,
        vec![
            Call::Raise("OnInstallBegin", Vec::new()),
            Call::Facts,
            Call::Elsewhere(None),
            Call::Record(InstalledRecord {
                kind: InstallKind::Ghost,
                object_name: "あたらしい".to_owned(),
                folder: "newbie".to_owned(),
                ghost_folder: Some("newbie".to_owned()),
            }),
            // Reference は入った物だけ（飛ばした同梱は載らない）。
            Call::Raise(
                "OnInstallCompleteEx",
                vec!["ghost".to_owned(), "あたらしい".to_owned(), at],
            ),
            Call::Raise(
                "OnInstallComplete",
                vec!["ghost".to_owned(), "あたらしい".to_owned(), String::new()],
            ),
            Call::Warn {
                title: title.to_owned(),
                body: body.join("\n"),
            },
        ],
        "締めの知らせの後に口が 1 回"
    );
}

/// ⑵ 番号付きの同梱の取り出し元が無い → 口 1 回（取り出し元の値は書かれたまま）。
#[test]
fn a_numbered_skipped_companion_warns_once() {
    let mut ports = FakePorts::new();
    run_one(
        &mut ports,
        ghost_nar(
            "newbie",
            &[
                "balloon0.directory,b0",
                "balloon0.source.directory,extra\\s0",
            ],
        ),
    );
    assert_one_warning(
        &ports,
        "インストールの警告 - あたらしい",
        &[
            "書庫: skip.nar",
            "入った物: ゴースト あたらしい",
            "入らなかったバルーン: b0（書庫に取り出し元 extra\\s0 が無かったため）",
            SUBSTITUTE,
        ],
    );
}

/// ⑶ 同梱が 2 つで片方だけ無い → 口 1 回。入った物に在るほうのバルーンが並ぶ。
#[test]
fn one_of_two_companions_skipped_lists_the_installed_balloon_too() {
    let mut ports = FakePorts::new();
    run_one(
        &mut ports,
        ghost_nar(
            "newbie",
            &[
                "balloon.directory,kaku",
                "balloon.source.directory,kaku",
                "balloon0.directory,b0",
                "balloon0.source.directory,s0",
            ],
        )
        .file("kaku/descript.txt", b"charset,UTF-8\r\nname,Kaku\r\n")
        .done(),
    );
    assert!(
        ports
            .under_root("ghost/newbie/balloon/kaku/descript.txt")
            .is_file()
    );
    assert_one_warning(
        &ports,
        "インストールの警告 - あたらしい",
        &[
            "書庫: skip.nar",
            "入った物: ゴースト あたらしい",
            "入った物: バルーン Kaku",
            "入らなかったバルーン: b0（書庫に取り出し元 s0 が無かったため）",
            SUBSTITUTE,
        ],
    );
}

/// ⑷ 取り出し元のフォルダのエントリだけが在り配下が空 → 無いのと同じく口 1 回。
#[test]
fn a_source_with_only_its_folder_entry_warns_once() {
    let mut ports = FakePorts::new();
    run_one(
        &mut ports,
        ghost_nar(
            "newbie",
            &[
                "balloon.directory,kaku",
                "balloon.source.directory,src-kaku",
            ],
        )
        .dir("src-kaku/"),
    );
    assert_one_warning(
        &ports,
        "インストールの警告 - あたらしい",
        &[
            "書庫: skip.nar",
            "入った物: ゴースト あたらしい",
            "入らなかったバルーン: kaku（書庫に取り出し元 src-kaku が無かったため）",
            SUBSTITUTE,
        ],
    );
}

/// ⑸ シェルの `.nar` で同梱の取り出し元が無い → シェルが入り、口 1 回。
#[test]
fn a_shell_with_a_skipped_companion_warns_once() {
    let mut ports = FakePorts::new();
    run_one(
        &mut ports,
        shell_with(&[
            "balloon.directory,kaku",
            "balloon.source.directory,src-kaku",
        ]),
    );
    assert!(
        ports
            .under_root(&format!("ghost/{RUNNING}/shell/dress/descript.txt"))
            .is_file()
    );
    assert_one_warning(
        &ports,
        "インストールの警告 - きがえ",
        &[
            "書庫: skip.nar",
            "入った物: シェル きがえ",
            "入らなかったバルーン: kaku（書庫に取り出し元 src-kaku が無かったため）",
            SUBSTITUTE,
        ],
    );
}

/// ⑹ 取り出し元が全部在る → 口 0 回（ダイアログ 0）。
#[test]
fn companions_with_all_sources_do_not_warn() {
    let mut ports = FakePorts::new();
    let ends = run_one(
        &mut ports,
        ghost_nar(
            "newbie",
            &[
                "balloon.directory,kaku",
                "balloon.source.directory,kaku",
                "balloon0.directory,b0",
                "balloon0.source.directory,s0",
            ],
        )
        .file("kaku/descript.txt", b"charset,UTF-8\r\nname,Kaku\r\n")
        .done()
        .file("s0/descript.txt", b"charset,UTF-8\r\nname,S0\r\n")
        .done(),
    );
    assert!(matches!(ends.as_slice(), [ArchiveEnd::Installed(items)] if items.len() == 3));
    assert_eq!(ports.warned(), Vec::new(), "飛ばしが無ければ口は呼ばない");
}

// ---- まとめ方と止まり方 ----

/// 同じ書庫で 2 つ飛ばす → 口は 1 回で、本文に飛ばした行が 2 行並ぶ（1.12）。
#[test]
fn two_skipped_companions_in_one_archive_warn_once_with_two_lines() {
    let mut ports = FakePorts::new();
    run_one(
        &mut ports,
        ghost_nar(
            "newbie",
            &[
                "balloon.directory,kaku",
                "balloon.source.directory,src-kaku",
                "balloon0.directory,b0",
                "balloon0.source.directory,s0",
            ],
        ),
    );
    assert_one_warning(
        &ports,
        "インストールの警告 - あたらしい",
        &[
            "書庫: skip.nar",
            "入った物: ゴースト あたらしい",
            "入らなかったバルーン: kaku（書庫に取り出し元 src-kaku が無かったため）",
            "入らなかったバルーン: b0（書庫に取り出し元 s0 が無かったため）",
            SUBSTITUTE,
        ],
    );
}

/// 締めの知らせが「閉じた」で返っても、口は 1 回呼ばれてから手続きが止まる。
#[test]
fn a_closed_closing_notice_still_warns_once_before_stopping() {
    let mut ports = FakePorts::new();
    ports.replies.insert("OnInstallCompleteEx", Raised::Closed);
    let ends = run_one(
        &mut ports,
        ghost_nar(
            "newbie",
            &[
                "balloon.directory,kaku",
                "balloon.source.directory,src-kaku",
            ],
        ),
    );
    assert_eq!(ends, vec![ArchiveEnd::Abandoned]);
    assert_eq!(
        ports.warned().len(),
        1,
        "閉じても口は 1 回: {:?}",
        ports.calls
    );
    assert!(
        matches!(ports.calls.last(), Some(Call::Warn { .. })),
        "口は最後（閉じた後は旧仕様の知らせを送らない）: {:?}",
        ports.calls
    );
    assert_eq!(
        ports.raised_ids(),
        ["OnInstallBegin", "OnInstallCompleteEx"]
    );
}

//! 鎖系の記録行を組む純関数——**書式そのもの**を固定する決定論的テスト（要件 8.3／9.1／9.2／9.5）。
//!
//! 実機も実ディスプレイも World も使わない（要件 10.1）。ハンドルは Win32 へ渡さない
//! 偽の `HWND` であり、Entity は空の `World` から採った値である。
//!
//! # なぜ「含む」ではなく「1 行まるごと」を突き合わせるのか
//!
//! 実機サインオフは本モジュールが吐く語を **1:1 で grep する**。`contains` で数フィールドだけ
//! 見る檻は、フィールドの追加・削除・並べ替え・語尾の変化を素通りさせる——手順書だけが
//! 静かに嘘になる形である。よって各タグについて**期待する 1 行を丸ごと書き下す**。
//!
//! # 欄の間に他の欄が挟まる形を、連結文字列へ丸めない
//!
//! 初版の申し送り 6.3 が記録した罠である——`action=set` と `source=Descript` の間に
//! `group_id=N` が挟まる行を「`action=set source=Descript`」と連結して写した手順書は
//! 1 件も当てられなかった。ここでは**隣り合う欄の対**を実際の字面のまま名指しで固定し、
//! 間に別の欄が割り込んだ瞬間に赤くなるようにしてある。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;
use windows::core::HRESULT;

use super::{
    ChainSegment, ChainSkipReason, DetachReason, UNKNOWN, absent_line, applied_line,
    chain_record_tags, link_failed_line, linked_line, log_group_applied, log_group_rejected,
    preserved_group_tags, rejected_line, settled_line, skipped_line, unlink_failed_line,
    unlinked_line,
};
use crate::ecs::test_support::capture_under_filter;

/// 実機サインオフが用いる `RUST_LOG` の鎖側の指定そのもの。
///
/// 指定は `zorder_chain` までであり、本モジュールの出力先は `zorder_chain_diag` である
/// ——`tracing` の指定は**前方一致**なので点灯する。この 1 本が「移設で保全語彙が
/// サインオフから見えなくなっていない」ことの機械的な証拠である。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_chain=debug";

/// 既定水準（診断手順を有効化していない通常運転）。
const DEFAULT_DIRECTIVES: &str = "info";

/// 本モジュールの記録の出力先（module path 既定）。
const LOG_TARGET: &str = "wintf::ecs::window::zorder_chain_diag";

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// テスト用の Entity を 2 つ採る（空の World から順に確保するだけ）。
fn two_entities() -> (Entity, Entity) {
    let mut world = World::new();
    (world.spawn_empty().id(), world.spawn_empty().id())
}

/// テスト用の失敗値（実際の Win32 呼び出しは行わない）。
fn fake_error() -> windows::core::Error {
    windows::core::Error::from(HRESULT(0x8007_0005u32 as i32))
}

/// 1 行から `key=value` を機械的に切り出す（キーがちょうど 1 回であることも主張する）。
///
/// サインオフの手順が行う切り出しと同じ操作をテスト側でも行うことで、「人間には読めるが
/// 機械には切り出せない」書式を赤にする。
fn field<'a>(line: &'a str, key: &str) -> &'a str {
    let needle = format!(" {key}=");
    assert_eq!(
        line.matches(needle.as_str()).count(),
        1,
        "`{key}=` が 1 行にちょうど 1 回ではない: {line}"
    );
    line.split(needle.as_str())
        .nth(1)
        .expect("直前で 1 回あることを確かめた")
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

/// 捕捉した出力から、指定タグを含む行をちょうど 1 本取り出す。
fn only_line_with<'a>(out: &'a str, tag: &str) -> &'a str {
    let found: Vec<&str> = out.lines().filter(|l| l.contains(tag)).collect();
    assert_eq!(found.len(), 1, "`{tag}` の行がちょうど 1 本ではない: {out}");
    found[0]
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ===========================================================================
// タグ語彙——鎖系 7 種＋保全 2 種
// ===========================================================================

/// 鎖系の記録タグは 7 種で、すべて鎖の冠を持ち、互いに異なる。
#[test]
fn the_seven_chain_tags_share_one_prefix_and_are_all_distinct() {
    let tags = chain_record_tags();

    assert_eq!(tags.len(), 7, "タグの本数が 7 種でない: {tags:?}");
    for tag in tags {
        assert!(
            tag.starts_with("[zorder-chain] "),
            "鎖系の冠を持たないタグがある: {tag}"
        );
    }
    for (i, tag) in tags.iter().enumerate() {
        for other in &tags[i + 1..] {
            assert_ne!(tag, other, "タグ語が重なっている: {tag}");
        }
    }
    // 語そのものを逐語で固定する（サインオフの grep 判定語）。
    assert_eq!(
        tags,
        [
            "[zorder-chain] linked",
            "[zorder-chain] unlinked",
            "[zorder-chain] settled",
            "[zorder-chain] absent",
            "[zorder-chain] skipped",
            "[zorder-chain] link-failed",
            "[zorder-chain] unlink-failed",
        ]
    );
}

/// 保全語彙 2 語は移設の前後で**字面が 1 字も変わっていない**（要件 9.5）。
///
/// 逐語の期待値は退役前の檻（`zorder_group_diag_tests.rs`）が持っていたものと同一である。
#[test]
fn the_two_preserved_group_tags_kept_their_exact_spelling_through_the_move() {
    assert_eq!(
        preserved_group_tags(),
        ["[zorder-group] applied", "[zorder-group] rejected"]
    );
}

/// 名簿に載っていない 8 つ目・3 つ目のタグはモジュール本体に存在しない。
///
/// 直上の 2 つは「登録されているものが正しい」ことしか言わない——**それ以外に無い**は
/// 名簿を読むだけでは決して言えず、余分な定数とそれを使う行組立を足しても名簿は沈黙する。
/// 数えるのはコード本文に現れる文字列リテラルである。
#[test]
fn no_extra_tag_hides_outside_the_two_rosters() {
    let code = code_only(include_str!("zorder_chain_diag.rs"));

    let chain_literals = code.matches("\"[zorder-chain] ").count();
    assert_eq!(
        chain_literals,
        chain_record_tags().len(),
        "モジュール本体の鎖タグ文字列が名簿と数で合わない"
    );
    let group_literals = code.matches("\"[zorder-group] ").count();
    assert_eq!(
        group_literals,
        preserved_group_tags().len(),
        "モジュール本体の保全タグ文字列が名簿と数で合わない"
    );

    // 対照: 数える針が本当に当たっている（0 を数えて緑になる走査ではない）
    assert_eq!(chain_literals, 7, "鎖タグの走査そのものが空振りしている");
    assert_eq!(group_literals, 2, "保全タグの走査そのものが空振りしている");
    for tag in chain_record_tags()
        .iter()
        .chain(preserved_group_tags().iter())
    {
        assert!(
            code.contains(&format!("\"{tag}\"")),
            "名簿の `{tag}` がコード本文のリテラルとして見つからない"
        );
    }
}

/// クレートの `src/` 配下の本番 `.rs` を集める（テストのファイルは除く）。
fn production_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("木を歩けない（検査が空振りする）: {} — {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("ディレクトリ項目が読めない").path();
        if path.is_dir() {
            production_rs_files(&path, out);
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".rs")
            || name.ends_with("_tests.rs")
            || name.ends_with("test_support.rs")
        {
            continue;
        }
        out.push(path);
    }
}

/// `src/` 配下の本番ファイルのうち、`needle` を**コード行の**字面として持つものの相対パス。
fn production_files_containing(needle: &str) -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    production_rs_files(&root, &mut files);
    assert!(
        files.len() >= 20,
        "走査が {} 件しか歩いていない（木の歩き方が壊れている）",
        files.len()
    );

    let mut found: Vec<String> = files
        .iter()
        .filter(|path| {
            let src = std::fs::read_to_string(path).expect("本番ファイルが読めない");
            code_only(&src).contains(needle)
        })
        .map(|path| {
            path.strip_prefix(&root)
                .expect("走査の根の下に無い")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    found.sort();
    found
}

/// 保全語彙 2 語の住処はクレート全体でただ 1 つ——`zorder_chain_diag.rs` である（要件 9.5）。
///
/// 旧所在（`zorder_group_diag.rs`／`zorder_group.rs`）は task 5.1 で削除済みなので、
/// 「あちらにもう無い」という形の主張はもう立てられない（対象そのものが実在しない）。
/// 代わりに**クレートの木を歩いて**住処が 1 つであることを直に数える。こちらの方が強い
/// ——退役した層に限らず、どのファイルへ 2 つ目の定義が生えても赤くなる。
#[test]
fn the_preserved_tags_have_exactly_one_home_in_this_crate() {
    for tag in preserved_group_tags() {
        let homes = production_files_containing(&format!("\"{tag}\""));
        assert_eq!(
            homes,
            vec!["ecs/window/zorder_chain_diag.rs".to_string()],
            "`{tag}` の住処が 1 つではない（grep 対象が分裂する）"
        );
    }

    // 対照 ①: 同じ走査は、実在する別の字面では当たる（0 を数えて緑になる走査ではない）
    assert_eq!(
        production_files_containing("\"[zorder-chain] linked\""),
        vec!["ecs/window/zorder_chain_diag.rs".to_string()],
        "走査そのものが壊れている（既知の鎖タグを見つけられない）"
    );
    // 対照 ②: 退役した語彙は本番コードのどこにも残っていない（要件 14.2）
    for retired in [
        "[zorder-group] fix",
        "[zorder-group] skip",
        "[zorder-group] verify-failed",
    ] {
        assert!(
            production_files_containing(&format!("\"{retired}\"")).is_empty(),
            "退役した語彙 `{retired}` が本番コードに残っている"
        );
    }
}

// ===========================================================================
// 各タグの 1 行——丸ごと固定する
// ===========================================================================

/// 繋いだ行は、区間・両端の Entity・両端の窓ハンドル・鎖の中の位置を同じ 1 行に載せる。
#[test]
fn the_linked_line_carries_segment_both_ends_and_the_position_in_the_chain() {
    let (owned, owner) = two_entities();
    let line = linked_line(
        Some(ChainSegment::Group(0)),
        owned,
        owner,
        Some(fake_hwnd(0xA0)),
        Some(fake_hwnd(0xB0)),
        1,
        4,
    );

    assert_eq!(
        line,
        format!(
            "[zorder-chain] linked segment=g0 owned={owned:?} owner={owner:?} \
             owned_hwnd=0xA0 owner_hwnd=0xB0 pos=1/4"
        )
    );
}

/// 後方配置の繋ぎは区間が `tail` になる（どのグループにも属さない・要件 15）。
#[test]
fn a_tail_segment_renders_as_the_tail_word_not_a_group_number() {
    let (owned, owner) = two_entities();
    let line = linked_line(
        Some(ChainSegment::Tail),
        owned,
        owner,
        Some(fake_hwnd(0x10)),
        None,
        3,
        3,
    );

    assert_eq!(field(&line, "segment"), "tail", "{line}");
    assert_eq!(field(&line, "owner_hwnd"), UNKNOWN, "{line}");
    assert_eq!(field(&line, "pos"), "3/3", "{line}");
}

/// 区間が取れない呼び出しでも欄は落ちず、番兵が入る（繋いだ行・張り失敗の行の両方）。
///
/// **本番の呼び出しはここへ来ない**——区間は望む鎖（`CrossEdge::segment`）が運び、撤去では
/// 帳簿の控えから出るので、実際に流れるのは常に `Some(..)` である。ここで固定するのは
/// `Option` を残してある理由そのもの、すなわち「値が取れなくても**欄が消えない**」ことで
/// ある——消えると「記録が出ていない」と「その経路にはその値が無い」の区別が事後に付かない。
#[test]
fn a_segmentless_call_still_carries_the_field_filled_with_the_sentinel() {
    let (owned, owner) = two_entities();

    let linked = linked_line(None, owned, owner, Some(fake_hwnd(0xA0)), None, 1, 2);
    assert_eq!(field(&linked, "segment"), UNKNOWN, "{linked}");
    assert!(
        linked.contains(&format!("segment={UNKNOWN} owned={owned:?}")),
        "番兵の欄と被所有側の間に別の欄が割り込んでいる: {linked}"
    );

    let failed = link_failed_line(None, Some(fake_hwnd(0xA0)), None, &fake_error());
    assert_eq!(field(&failed, "segment"), UNKNOWN, "{failed}");
    assert!(
        failed.contains(&format!("segment={UNKNOWN} owned_hwnd=0xA0")),
        "番兵の欄と被所有側のハンドルの間に別の欄が割り込んでいる: {failed}"
    );
}

/// 区間の欄は、繋いだ行の中で **`owned=` の直前**に在る（間に別の欄が割り込まない）。
///
/// 連結文字列へ丸めると欄の割り込みを見逃す（初版 6.3 の罠）。ここでは隣り合う対を
/// 実際の字面のまま名指しで固定する。
#[test]
fn the_segment_field_sits_immediately_before_the_owned_field() {
    let (owned, owner) = two_entities();
    let line = linked_line(
        Some(ChainSegment::Group(2)),
        owned,
        owner,
        Some(fake_hwnd(0xA0)),
        Some(fake_hwnd(0xB0)),
        2,
        4,
    );

    assert!(
        line.contains(&format!("segment=g2 owned={owned:?}")),
        "区間と被所有側の間に別の欄が割り込んでいる: {line}"
    );
    assert!(
        line.contains("owned_hwnd=0xA0 owner_hwnd=0xB0 pos=2/4"),
        "ハンドル 2 欄と位置の並びが崩れている: {line}"
    );
}

/// Entity の字面には空白が入らない（1 行からの `field=value` 切り出しが壊れない）。
#[test]
fn the_entity_fields_never_contain_whitespace() {
    let (owned, owner) = two_entities();
    let rendered = format!("{owned:?}{owner:?}");

    assert!(
        !rendered.contains(char::is_whitespace),
        "Entity の Debug 表現に空白が混じっている: {rendered}"
    );
}

/// 外した行は、区間・被所有側・両端の窓ハンドル・理由を同じ 1 行に載せる。
#[test]
fn the_unlinked_line_carries_the_reason_beside_the_handles() {
    let (owned, _) = two_entities();
    let line = unlinked_line(
        Some(ChainSegment::Group(1)),
        owned,
        Some(fake_hwnd(0xC0)),
        Some(fake_hwnd(0xD0)),
        DetachReason::Rechain,
    );

    assert_eq!(
        line,
        format!(
            "[zorder-chain] unlinked segment=g1 owned={owned:?} \
             owned_hwnd=0xC0 owner_hwnd=0xD0 reason=Rechain"
        )
    );
}

/// 区間が分からない撤去でも欄は落ちず番兵になる（グループごと消えた後の撤去）。
#[test]
fn an_unknown_segment_renders_the_sentinel_instead_of_dropping_the_field() {
    let (owned, _) = two_entities();
    let line = unlinked_line(None, owned, None, None, DetachReason::Teardown);

    assert_eq!(field(&line, "segment"), UNKNOWN, "{line}");
    assert_eq!(field(&line, "owned_hwnd"), UNKNOWN, "{line}");
    assert_eq!(field(&line, "owner_hwnd"), UNKNOWN, "{line}");
    assert_eq!(field(&line, "reason"), "Teardown", "{line}");
}

/// 撤去の理由 4 種は互いに異なる語で記録される（1 語へ潰れると理由が読めない）。
#[test]
fn the_four_detach_reasons_render_as_four_distinct_words() {
    let (owned, _) = two_entities();
    let mut seen: Vec<String> = Vec::new();

    for reason in [
        DetachReason::Teardown,
        DetachReason::Rechain,
        DetachReason::Departing,
        DetachReason::Diverged,
    ] {
        let line = unlinked_line(None, owned, None, None, reason);
        let word = field(&line, "reason").to_string();
        assert!(!word.is_empty(), "理由語が読めない: {line}");
        assert!(!seen.contains(&word), "理由語 `{word}` が他と重なっている");
        seen.push(word);
    }
    assert_eq!(seen, ["Teardown", "Rechain", "Departing", "Diverged"]);
}

/// 収まった行は、**組み替えの宣言と直後の実測を同じ 1 行**に載せる（要件 9.2）。
///
/// 行を「宣言」と「実測」に分けないのは design.md の裁定である。分けると
/// 「指令は出したが効かなかった」の判定が 2 行の突合になる。
#[test]
fn the_settled_line_carries_the_declaration_and_the_measurement_together() {
    let line = settled_line(
        Some(fake_hwnd(0xA0)),
        Some(fake_hwnd(0xB0)),
        &[fake_hwnd(0xA0), fake_hwnd(0xB0), fake_hwnd(0xC0)],
        &[fake_hwnd(0xA0), fake_hwnd(0xB0), fake_hwnd(0xC0)],
        Some(true),
    );

    assert_eq!(
        line,
        "[zorder-chain] settled nudged_hwnd=0xA0 insert_after=0xB0 \
         declared=0xA0,0xB0,0xC0 measured=0xA0,0xB0,0xC0 nudge_ok=true"
    );
}

/// サインオフの切り出しが読む 4 欄は、この順で**隣り合って**現れる。
///
/// `signoff-scan.ps1` の正規表現は 4 欄の隣接を前提にしている。欄の間に別の欄が
/// 割り込むと手順が静かに 0 件になるので、隣接をそのまま固定する。
#[test]
fn the_four_signoff_fields_of_the_settled_line_stay_adjacent_in_this_order() {
    let line = settled_line(
        Some(fake_hwnd(0x1)),
        Some(fake_hwnd(0x2)),
        &[fake_hwnd(0x1)],
        &[fake_hwnd(0x2)],
        Some(false),
    );

    assert!(
        line.contains("nudged_hwnd=0x1 insert_after=0x2 declared=0x1 measured=0x2"),
        "サインオフが読む 4 欄の隣接が崩れている: {line}"
    );
    // 失敗の事実は 4 欄の**後ろ**に足す（切り出しの正規表現を壊さない位置）。
    assert!(
        line.ends_with(" nudge_ok=false"),
        "後押しの成否が 4 欄の後ろに載っていない: {line}"
    );
}

/// 収まった行の各欄は、値が無くても落ちずに番兵になる。
#[test]
fn an_unnudged_pass_renders_the_sentinel_in_every_settled_field() {
    let line = settled_line(None, None, &[], &[], None);

    assert_eq!(
        line,
        "[zorder-chain] settled nudged_hwnd=- insert_after=- declared=- measured=- nudge_ok=-"
    );
}

/// 不在の行は、どのグループのどの宣言要素が不在だったかを載せる（要件 1.4／8.4）。
#[test]
fn the_absent_line_names_the_group_and_the_declared_element() {
    assert_eq!(
        absent_line(3, "b0"),
        "[zorder-chain] absent group_id=3 element=b0"
    );
}

/// 不在の行の要素名も自由文として畳む（空なら番兵で、欄そのものは落ちない）。
#[test]
fn an_empty_element_name_renders_the_sentinel() {
    let line = absent_line(0, "");
    assert_eq!(line, "[zorder-chain] absent group_id=0 element=-");
}

/// 見送りの行は必ず理由を伴う（要件 8.3——理由の無い見送りを作れない）。
#[test]
fn the_skipped_line_always_carries_its_reason() {
    assert_eq!(
        skipped_line(ChainSkipReason::NoChange),
        "[zorder-chain] skipped reason=NoChange"
    );
}

/// 見送りの理由 3 種は互いに異なる語で記録される。
#[test]
fn the_three_skip_reasons_render_as_three_distinct_words() {
    let words: Vec<String> = [
        ChainSkipReason::TooFewPresent,
        ChainSkipReason::NoChange,
        ChainSkipReason::HandleMissing,
    ]
    .into_iter()
    .map(|reason| field(&skipped_line(reason), "reason").to_string())
    .collect();

    assert_eq!(words, ["TooFewPresent", "NoChange", "HandleMissing"]);
}

/// 張り失敗の行は、区間・両端の窓ハンドル・失敗値を同じ 1 行に載せる（要件 8.2）。
#[test]
fn the_link_failed_line_carries_the_segment_and_both_handles() {
    let line = link_failed_line(
        Some(ChainSegment::Group(1)),
        Some(fake_hwnd(0xA0)),
        Some(fake_hwnd(0xB0)),
        &fake_error(),
    );

    assert_eq!(
        line,
        "[zorder-chain] link-failed segment=g1 owned_hwnd=0xA0 owner_hwnd=0xB0 \
         error=HRESULT(0x80070005)"
    );
}

/// 外し失敗の行は、被所有側の窓ハンドルと失敗値を載せる（要件 8.2）。
#[test]
fn the_unlink_failed_line_carries_the_owned_handle_and_the_error() {
    let line = unlink_failed_line(Some(fake_hwnd(0xC0)), &fake_error());

    assert_eq!(
        line,
        "[zorder-chain] unlink-failed owned_hwnd=0xC0 error=HRESULT(0x80070005)"
    );
}

// ===========================================================================
// 移設してきた保全語彙 2 語（要件 9.5）——字面も水準も出力先も固定する
// ===========================================================================

/// 受理の行は、台帳が組んだ本文へタグだけを貼る（移設前と 1 字も変わらない）。
#[test]
fn the_applied_line_prefixes_the_ledger_text_with_the_tag_and_nothing_else() {
    assert_eq!(
        applied_line("group_id=1 members=2"),
        "[zorder-group] applied group_id=1 members=2"
    );
}

/// 起動由来の受理行の実際の字面——`action=set` と `source=Descript` の**間に**
/// `group_id=` が挟まる（初版 6.3 の罠を字面のまま固定する）。
#[test]
fn the_boot_time_applied_line_keeps_group_id_between_action_and_source() {
    let line = applied_line("action=set group_id=7 source=Descript members=b0,s0 normalized=-");

    assert_eq!(
        line,
        "[zorder-group] applied action=set group_id=7 source=Descript \
         members=b0,s0 normalized=-"
    );
    assert!(
        !line.contains("action=set source=Descript"),
        "欄の割り込みを連結文字列へ丸めた形が成立してしまっている: {line}"
    );
}

/// 拒否の行は、受け取ったトークン列と拒否理由を載せる（移設前と 1 字も変わらない）。
#[test]
fn the_rejected_line_carries_the_reason_and_the_received_tokens() {
    assert_eq!(
        rejected_line("CrossGroupRedesignation", "1,0"),
        "[zorder-group] rejected reason=CrossGroupRedesignation tokens=1,0"
    );
}

/// 拒否の行の 2 欄は、空でも落ちずに番兵になる（移設前と同じ）。
#[test]
fn an_empty_reason_or_token_list_renders_the_sentinel() {
    let line = rejected_line("", "");
    assert_eq!(field(&line, "reason"), UNKNOWN, "{line}");
    assert_eq!(field(&line, "tokens"), UNKNOWN, "{line}");
    assert_eq!(line, "[zorder-group] rejected reason=- tokens=-");
}

/// 呼び出し側の文字列に空白が混じっても、1 行の `field=value` 切り出しは壊れない。
#[test]
fn whitespace_in_caller_supplied_text_is_folded_so_the_cut_still_works() {
    let line = rejected_line("Unparsable Token", "b1, s1 , x");

    assert_eq!(field(&line, "reason"), "Unparsable_Token", "{line}");
    assert_eq!(field(&line, "tokens"), "b1,_s1_,_x", "{line}");
}

/// 拒否は既定水準でも読める warn として、受理と同じ 1 本の出力先へ出る。
///
/// あわせて、サインオフの `RUST_LOG` 指定（`…::zorder_chain=debug`）が**前方一致で**
/// 本モジュールの出力先（`…::zorder_chain_diag`）を点灯させることを固定する
/// ——移設で保全語彙がサインオフから見えなくなっていないことの機械的な証拠である。
#[test]
fn a_rejection_is_recorded_at_warn_level_and_survives_the_default_level() {
    let signoff = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        log_group_applied("group_id=1 members=2");
        log_group_rejected("ModeMixed", "b1,0");
    });
    let line = only_line_with(&signoff, "[zorder-group] rejected");
    assert!(line.contains("WARN"), "拒否が warn 水準でない: {line}");
    assert!(
        line.contains(LOG_TARGET),
        "grep 対象の出力先が module path 既定でない: {line}"
    );
    assert_eq!(field(line, "reason"), "ModeMixed", "{line}");
    assert_eq!(field(line, "tokens"), "b1,0", "{line}");
    assert!(
        signoff.contains("[zorder-group] applied"),
        "サインオフの指定が前方一致で本モジュールを点灯させていない: {signoff}"
    );

    let default = capture_under_filter(DEFAULT_DIRECTIVES, || {
        log_group_applied("group_id=1 members=2");
        log_group_rejected("ModeMixed", "b1,0");
    });
    assert!(
        default.contains("[zorder-group] rejected"),
        "既定運転で拒否が読めない（黙って捨てられている）: {default}"
    );
    assert!(
        !default.contains("[zorder-group] applied"),
        "診断専用の受理が既定水準へ漏れている（水準の区別を見ていない）: {default}"
    );
}

// ===========================================================================
// 既存ペア機構の語彙は不変（要件 9.5）——両側から挟む
// ===========================================================================

/// あちらの 6 タグは逐語で残っており、こちらはその語を 1 つも名乗らない。
#[test]
fn the_six_pair_tags_survive_verbatim_and_are_not_impersonated_here() {
    let pair_src = include_str!("zorder_pair_diag.rs");
    let here = code_only(include_str!("zorder_chain_diag.rs"));

    // 片側⑴: あちらの語が今も在る（消す変異・書き換える変異が赤くなる）
    for tag in [
        "[zorder-pair] owner-established",
        "[zorder-pair] fix",
        "[zorder-pair] skip",
        "[zorder-pair] verify-failed",
        "[zorder-pair] owner-establish-failed",
        "[zorder-pair] sink-observed",
    ] {
        assert!(
            pair_src.contains(&format!("\"{tag}\"")),
            "既存ペア機構のタグ `{tag}` が逐語で見つからない（要件 9.5）"
        );
    }

    // 片側⑵: こちらはあちらの冠を 1 つも名乗らない（横取りする変異が赤くなる）
    assert!(
        !here.contains("[zorder-pair]"),
        "鎖系の記録がペア機構の語彙を名乗っている"
    );
}

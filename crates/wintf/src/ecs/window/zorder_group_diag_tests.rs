//! グループ系の記録行を組む純関数——**書式そのもの**を固定する決定論的テスト。
//!
//! 実機も実ディスプレイも World も使わない（要件 10.1）。ハンドルは Win32 へ渡さない
//! 偽の `HWND` であり、観測値は手で組んだ構造体である。
//!
//! # なぜ「含む」ではなく「1 行まるごと」を突き合わせるのか
//!
//! 実機サインオフ（task 7.4）は本モジュールが吐く語を **1:1 で grep する**。`contains` で
//! 数フィールドだけ見る檻は、フィールドの追加・削除・並べ替え・語尾の変化を素通りさせる
//! ——手順書だけが静かに嘘になる形である。よって各タグについて**期待する 1 行を丸ごと
//! 書き下し**、書式が動けば必ず赤くなるようにしてある。
//!
//! # 「変えない」の主張は両側から挟む（要件 9.5）
//!
//! 既存ペア機構の 6 タグを「変えていない」は、こちらの都合だけを見ていても成立して
//! しまう。ここでは**あちらの本文に 6 タグが今も逐語で在ること**と、**こちらのコード本文に
//! `[zorder-pair]` が 1 つも無いこと**の両方を主張する。片側だけだと、あちらの語を
//! 奪って名乗る変異体も、あちらのタグを消す変異体も、どちらかが素通りする。
//!
//! # 「マクロが無いこと」は走査で主張し、対照で空振りを防ぐ
//!
//! 本モジュールの存在理由は「`tracing` のマクロを 1 つも置かない」ことであり、これは
//! 不在の主張ゆえ書き足された瞬間に静かに崩れる。そこで本文を読んで走査する。
//! 走査そのものが壊れていれば不在は恒真に成立するので、**同じ走査を兄弟
//! （`zorder_group.rs`＝マクロが在る側）へ当てて必ず見つかること**を併置してある。

use windows::Win32::Foundation::HWND;

use super::{UNKNOWN, fix_line, group_record_tags, skip_line, verify_failed_line};
use crate::ecs::window::zorder_group::{GroupObservation, GroupSkipReason, GroupVerify};

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// 観測値を手で組む——**宣言どおりに実測できた巡**（宣言列と実測列が一致する）。
///
/// 一致する側だけで書式を固定すると「宣言列を実測と偽って載せる」欠陥が素通りするので、
/// 食い違う側は [`measured_observation`] で明示的に組む。
fn observation(id: u32, hwnds: &[HWND], missing: usize, order_ok: bool) -> GroupObservation {
    measured_observation(id, hwnds, hwnds, missing, order_ok)
}

/// 観測値を手で組む——**宣言列と実測列を別々に**与える。
fn measured_observation(
    id: u32,
    hwnds: &[HWND],
    measured_front: &[HWND],
    missing: usize,
    order_ok: bool,
) -> GroupObservation {
    GroupObservation {
        id,
        hwnds: hwnds.to_vec(),
        measured_front: measured_front.to_vec(),
        missing,
        order_ok,
        // 走査を行い、最前面まで辿れた巡として組む。辿れなかった巡・走査そのものを
        // 行わなかった巡との差は `the_verify_failed_line_never_folds...` が固定する。
        scan_complete: Some(true),
    }
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

// ===========================================================================
// タグ語彙——この層に残る 3 種（要件 9.1／9.2）
// ===========================================================================
//
// 受理・拒否の 2 種は要件 9.5 の保全対象として `zorder_chain_diag` へ移した。
// 字面・水準・出力先の固定はあちらの兄弟テストが引き継いでいる。

/// 記録タグは 3 種で、すべてグループ系の冠を持ち、互いに異なる。
///
/// 冠を共有するのはサインオフが `[zorder-group]` の 1 語で全記録を拾えるようにするため、
/// 互いに異なるのは拾った行を種別へ振り分けられるようにするためである。
#[test]
fn the_three_remaining_group_tags_share_one_prefix_and_are_all_distinct() {
    let tags = group_record_tags();

    assert_eq!(tags.len(), 3, "タグの本数が 3 種でない: {tags:?}");
    for tag in tags {
        assert!(
            tag.starts_with("[zorder-group] "),
            "グループ系の冠を持たないタグがある: {tag}"
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
            "[zorder-group] fix",
            "[zorder-group] skip",
            "[zorder-group] verify-failed",
        ]
    );
}

/// 名簿に載っていない 4 つ目のタグはモジュール本体に存在しない。
///
/// 直上のテストは「登録されている 3 つが正しい」ことしか言わない——**それ以外に無い**は
/// 名簿を読むだけでは決して言えず、4 つ目の定数とそれを使う行組立を足しても名簿は
/// 沈黙する。親裁定は「タグを勝手に増やさない」と明示しているので、こちらから挟む。
///
/// 数えるのはコード本文に現れる `[zorder-group] ` の**文字列リテラル**である。定数を
/// 足しても、タグを組立の中へ直に書き込んでも、どちらも数が動く。移した 2 語がここへ
/// 戻ってくれば（＝住処が 2 つに割れれば）この数がそのまま動く。
#[test]
fn no_fourth_tag_hides_outside_the_roster() {
    let code = code_only(include_str!("zorder_group_diag.rs"));

    let literals = code.matches("\"[zorder-group] ").count();
    assert_eq!(
        literals,
        group_record_tags().len(),
        "モジュール本体のタグ文字列が名簿と数で合わない（名簿外のタグが在る／名簿が実体を指していない）"
    );

    // 対照: 数える針が本当に当たっている（0 を数えて緑になる走査ではない）
    assert_eq!(literals, 3, "走査そのものが空振りしている");
    for tag in group_record_tags() {
        assert!(
            code.contains(&format!("\"{tag}\"")),
            "名簿の `{tag}` がコード本文のリテラルとして見つからない"
        );
    }
}

// ===========================================================================
// 各タグの 1 行——丸ごと固定する
// ===========================================================================

/// 是正の行は、グループ識別子・動かした窓・挿入先・検証時の実測を**同じ 1 行**に載せる。
///
/// 行を「指令」と「実測」に分けないのは design.md の裁定であり（既存ペア機構
/// `zorder_pair_diag::fix_line` と同じ規律）、分けると「指令は出したが効かなかった」の
/// 判定が 2 行の突合になるからである。
#[test]
fn the_fix_line_carries_group_command_and_measurement_together() {
    let verify = GroupVerify {
        id: 2,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0), fake_hwnd(0xC0)],
    };
    let observed = observation(
        2,
        &[fake_hwnd(0xA0), fake_hwnd(0xB0), fake_hwnd(0xC0)],
        0,
        true,
    );

    assert_eq!(
        fix_line(&verify, &observed),
        "[zorder-group] fix group_id=2 head=0xA0 moves=0xB0@0xA0,0xC0@0xB0 \
         measured=0xA0,0xB0,0xC0"
    );
}

/// 是正の行の 4 要素は、どれも 1 行から機械的に切り出せる（要件 9.1／9.2）。
///
/// 逐語比較（上）とは別に切り出しでも主張するのは、どれか 1 つを落とす変異が
/// 「行が短くなった」ではなく「その値が読めなくなった」として赤くなるようにするためである。
#[test]
fn every_field_of_the_fix_line_can_be_cut_out_mechanically() {
    let verify = GroupVerify {
        id: 7,
        head: fake_hwnd(0x10),
        chain: vec![fake_hwnd(0x20), fake_hwnd(0x30)],
    };
    // 宣言列と実測列を**わざと食い違わせる**——`measured` が宣言の写しになっていれば
    // ⑷ が赤くなる。
    let observed = measured_observation(
        7,
        &[fake_hwnd(0x10), fake_hwnd(0x20)],
        &[fake_hwnd(0x20), fake_hwnd(0x10)],
        1,
        true,
    );
    let line = fix_line(&verify, &observed);

    // ⑴ どのグループか
    assert_eq!(field(&line, "group_id"), "7", "{line}");
    // ⑵ 動かさなかった軸（連鎖の起点）
    assert_eq!(field(&line, "head"), "0x10", "{line}");
    // ⑶ 動かした窓と、その挿入先（段ごとに「窓@挿入先」）
    assert_eq!(field(&line, "moves"), "0x20@0x10,0x30@0x20", "{line}");
    // ⑷ 検証巡の走査が**実際に出会った**構成窓の列（宣言列ではない）
    assert_eq!(field(&line, "measured"), "0x20,0x10", "{line}");
}

/// 是正の行の `measured=` は実測列を映す——宣言列が変わっても動かない。
///
/// 「実測を騙る欄」はこの向きでしか捕まらない: 解決できたメンバー集合が同じなら、
/// まったく別の Z 形が byte 一致の行を出してしまう。よって**実測列だけを動かした 2 本**と
/// **宣言列だけを動かした 2 本**の両側から挟む。
#[test]
fn the_fix_line_measures_the_real_order_not_the_declared_one() {
    let verify = GroupVerify {
        id: 5,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };
    let declared = [fake_hwnd(0xA0), fake_hwnd(0xB0)];

    // 片側⑴: 宣言列を固定したまま実測列を変えると、行は必ず変わる
    let as_declared = fix_line(
        &verify,
        &measured_observation(5, &declared, &declared, 0, true),
    );
    let reversed = fix_line(
        &verify,
        &measured_observation(5, &declared, &[fake_hwnd(0xB0), fake_hwnd(0xA0)], 0, true),
    );
    assert_ne!(
        as_declared, reversed,
        "実際の重なりが違うのに同じ行が出た（`measured=` が実測を映していない）"
    );
    assert_eq!(field(&reversed, "measured"), "0xB0,0xA0", "{reversed}");

    // 片側⑵: 実測列を固定したまま宣言列を変えても、`measured=` は動かない
    let other_declaration = fix_line(
        &verify,
        &measured_observation(5, &[fake_hwnd(0xB0), fake_hwnd(0xA0)], &declared, 0, true),
    );
    assert_eq!(
        field(&other_declaration, "measured"),
        "0xA0,0xB0",
        "宣言列の変更が実測の欄へ漏れている: {other_declaration}"
    );
}

/// 連鎖が空でも挿入先の欄は落ちず、番兵になる。
///
/// 欄ごと落とすと「記録が出ていない」と「その経路にはその値が無い」の区別が事後に
/// 付かなくなる（既存ペア機構 `zorder_pair_diag` の番兵規律）。
#[test]
fn an_empty_chain_renders_the_sentinel_instead_of_dropping_the_field() {
    let verify = GroupVerify {
        id: 3,
        head: fake_hwnd(0xA0),
        chain: Vec::new(),
    };
    let observed = observation(3, &[], 2, true);

    let line = fix_line(&verify, &observed);
    assert_eq!(field(&line, "moves"), UNKNOWN, "{line}");
    assert_eq!(field(&line, "measured"), UNKNOWN, "{line}");
    assert_eq!(
        line,
        "[zorder-group] fix group_id=3 head=0xA0 moves=- measured=-"
    );
}

/// 検証不一致の行は、宣言列と実測列を並べ、未解決の枚数も載せる。
///
/// 不一致こそ両者が食い違う行である。ここで宣言列を `measured=` に載せてしまうと、
/// **不一致を報せながら期待どおりの並びを見せる**行になり、サインオフの読み手が
/// 期待値を観測値として読む。
#[test]
fn the_verify_failed_line_shows_the_declared_order_beside_the_measured_one() {
    let verify = GroupVerify {
        id: 2,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };
    let observed = measured_observation(
        2,
        &[fake_hwnd(0xA0), fake_hwnd(0xB0)],
        &[fake_hwnd(0xB0), fake_hwnd(0xA0)],
        1,
        false,
    );

    assert_eq!(
        verify_failed_line(&verify, &observed),
        "[zorder-group] verify-failed group_id=2 head=0xA0 moves=0xB0@0xA0 \
         members=0xA0,0xB0 measured=0xB0,0xA0 missing=1 scan_complete=true"
    );
    // 2 欄が別物であることを、名前でも値でも主張する（片方が他方の写しなら赤くなる）
    let line = verify_failed_line(&verify, &observed);
    assert_ne!(
        field(&line, "members"),
        field(&line, "measured"),
        "宣言列と実測列が同じ字面になっている: {line}"
    );
}

/// 検証不一致の行は、走査が最前面まで辿れたかを 3 値のまま載せる（`-`／`true`／`false`）。
///
/// `measured` は走査が出会わなかったメンバーを落とすので、この欄が無いと
/// 「測ったら別の場所に居た」と「そこまで測れなかった」が同じ字面になる。**3 値すべて**を
/// 回すのは、`None` を `false` へ潰す実装（＝測っていないのに「届かなかった」と読ませる）
/// も、常に `true` を書く実装も、片側だけの入力では素通りするからである。
#[test]
fn the_verify_failed_line_never_folds_an_unmeasured_scan_into_a_measured_negative() {
    let verify = GroupVerify {
        id: 4,
        head: fake_hwnd(0xA0),
        chain: vec![fake_hwnd(0xB0)],
    };
    let mut observed = measured_observation(
        4,
        &[fake_hwnd(0xA0), fake_hwnd(0xB0)],
        &[fake_hwnd(0xA0)],
        0,
        false,
    );

    for (scan_complete, rendered) in [
        (Some(true), "true"),
        (Some(false), "false"),
        (None, UNKNOWN),
    ] {
        observed.scan_complete = scan_complete;
        let line = verify_failed_line(&verify, &observed);
        assert_eq!(
            field(&line, "scan_complete"),
            rendered,
            "{scan_complete:?} が `{rendered}` として読めない: {line}"
        );
    }
}

/// 見送りの行は理由を必ず伴い、観測値も同じ行に載る（要件 8.3）。
#[test]
fn the_skip_line_always_carries_its_reason_and_the_observed_values() {
    let observed = observation(9, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 3, true);

    assert_eq!(
        skip_line(Some(9), GroupSkipReason::MemberMissing, Some(&observed)),
        "[zorder-group] skip group_id=9 reason=MemberMissing resolved=2 missing=3 order_ok=true"
    );
}

/// 見送りの理由 5 種は、行の上で互いに異なる語になる（1 語へ潰れると理由が読めない）。
#[test]
fn the_five_skip_reasons_render_as_five_distinct_words() {
    let observed = observation(1, &[fake_hwnd(0xA0), fake_hwnd(0xB0)], 0, true);
    let mut seen: Vec<String> = Vec::new();

    for reason in [
        GroupSkipReason::AlreadyOrdered,
        GroupSkipReason::TooFewResolved,
        GroupSkipReason::MemberMissing,
        GroupSkipReason::PairFixThisPass,
        GroupSkipReason::GaveUpAfterFailures,
    ] {
        let line = skip_line(Some(1), reason, Some(&observed));
        let word = field(&line, "reason").to_string();
        assert!(!word.is_empty(), "理由語が読めない: {line}");
        assert!(!seen.contains(&word), "理由語 `{word}` が他と重なっている");
        seen.push(word);
    }
    assert_eq!(
        seen,
        [
            "AlreadyOrdered",
            "TooFewResolved",
            "MemberMissing",
            "PairFixThisPass",
            "GaveUpAfterFailures"
        ]
    );
}

/// 観測より前の見送り（巡そのものの調停）は、欄を落とさず番兵で埋める。
#[test]
fn a_skip_before_observation_fills_every_field_with_the_sentinel() {
    assert_eq!(
        skip_line(None, GroupSkipReason::PairFixThisPass, None),
        "[zorder-group] skip group_id=- reason=PairFixThisPass resolved=- missing=- order_ok=-"
    );
}

// ===========================================================================
// 既存ペア機構の語彙は不変（要件 9.5）——両側から挟む
// ===========================================================================

/// あちらの 6 タグは逐語で残っており、こちらはその語を 1 つも名乗らない。
#[test]
fn the_six_pair_tags_survive_verbatim_and_are_not_impersonated_here() {
    let pair_src = include_str!("zorder_pair_diag.rs");
    let group_src = include_str!("zorder_group_diag.rs");

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
            "既存ペア機構の記録タグ `{tag}` が本文から消えている（要件 9.5）"
        );
    }

    // 片側⑵: こちらはその語を名乗らない（横取りする変異が赤くなる）。
    //
    // 説明文を落としてから探す——こちらの doc には「あちらの語とは一語も重ならない」と
    // いう**否定の説明**が書いてあり（それ自体が要件 9.5 の設計意図の記録である）、
    // 素の全文を探すとその説明で赤くなる。落とし過ぎ・落とし漏れが起きていないことは、
    // 直下の 2 つの対照が示す。
    let group_code = code_only(group_src);
    assert!(
        !group_code.contains("[zorder-pair]"),
        "グループ系のコード本文がペア機構の語彙を名乗っている（要件 9.5）"
    );
    assert!(
        group_src.contains("[zorder-pair]"),
        "説明文を落とす処理の対照が失われている（否定の説明が doc から消えた）"
    );
    assert!(
        group_code.contains("[zorder-group] verify-failed"),
        "説明文を落とす処理が本文まで落としている"
    );
}

// ===========================================================================
// マクロを置かないこと——走査で主張し、対照で空振りを防ぐ
// ===========================================================================

/// `tracing` のマクロは本モジュールに 1 つも無く、兄弟（記録の出口）には在る。
///
/// 不在だけを見ると走査が壊れていても緑になるため、**在る側**を同じ走査で必ず見つける
/// ことを併置してある。出力先（module path 既定）を 1 本に保つ根拠がこの不在である。
#[test]
fn no_tracing_macro_lives_here_while_the_sibling_emitter_has_them() {
    const MACRO_NEEDLES: [&str; 6] = [
        "trace!(",
        "debug!(",
        "info!(",
        "warn!(",
        "error!(",
        "tracing::",
    ];

    let here = code_only(include_str!("zorder_group_diag.rs"));
    let sibling = code_only(include_str!("zorder_group.rs"));

    for needle in MACRO_NEEDLES {
        assert!(
            !here.contains(needle),
            "記録行の層に `{needle}` が現れた（出力先が 2 本に分裂する）"
        );
    }

    // 対照: 同じ走査が、マクロを持つ兄弟では必ず何かを見つける（走査の空振り検出）
    let found = MACRO_NEEDLES
        .iter()
        .filter(|needle| sibling.contains(**needle))
        .count();
    assert!(
        found >= 2,
        "走査がマクロを持つ兄弟でも何も見つけない（走査そのものが壊れている疑い）"
    );

    // 対照: 説明文を落とす処理が本文まで落としていない
    assert!(
        here.contains("pub(crate) fn fix_line("),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

//! 特別な名前（`random`・`sequential`・`lastinstalled`）の解決の決定論テスト。
//!
//! 偽の目録と固定の乱数で、解決の純粋な関数だけを確かめる。

use areka_ghost::GhostEntry;

use super::*;

/// 名前の要らない目録（フォルダ名だけ・並びは渡した順）。
fn catalog(folders: &[&str]) -> Vec<GhostEntry> {
    folders.iter().map(|f| catalog_entry(f, None)).collect()
}

fn resolved(folder: &str, position: Option<usize>) -> NameResolution {
    NameResolution::Resolved {
        folder: folder.to_owned(),
        position,
    }
}

/// 呼ばれてはならない乱数。
fn no_pick(n: usize) -> usize {
    panic!("乱数は呼ばれないはず（n = {n}）")
}

/// 乱数が `want_n` 個の候補で呼ばれることを確かめ、`index` を返す。
fn pick_expecting(want_n: usize, index: usize) -> impl FnOnce(usize) -> usize {
    move |n| {
        assert_eq!(n, want_n, "候補の数");
        index
    }
}

/// 特別な名前は正典の 3 語の完全一致だけ（大文字小文字を区別）。
#[test]
fn special_names_are_exact_canonical_spellings() {
    let entries = catalog(&["A", "B"]);
    let special = |n: &str| {
        resolve_special_name(n, &entries, Some("A"), Some("B"), |_| 0) != NameResolution::Plain
    };
    assert_eq!(
        [
            "Random",
            "RANDOM",
            "Nobody",
            "random",
            "sequential",
            "lastinstalled"
        ]
        .map(special),
        [false, false, false, true, true, true],
    );
}

/// `random`: 候補は今のゴーストを除いた目録。両端の添字がどちらも選べ、今のゴーストは
/// 選ばれない。
#[test]
fn random_picks_among_others_at_both_ends() {
    let entries = catalog(&["A", "B", "C"]);
    let run = |i| resolve_special_name("random", &entries, Some("B"), None, pick_expecting(2, i));
    assert_eq!([run(0), run(1)], [resolved("A", None), resolved("C", None)]);
}

/// `random`: 目録が今のゴースト 1 体だけなら自分自身（乱数は呼ばない）。
#[test]
fn random_with_only_current_resolves_to_itself() {
    let entries = catalog(&["A"]);
    assert_eq!(
        resolve_special_name("random", &entries, Some("A"), None, no_pick),
        resolved("A", None),
    );
}

/// `random`: 今のゴーストが目録に無い（`None`・目録に無い名前）なら全ゴーストが候補。
#[test]
fn random_without_current_in_catalog_uses_every_ghost() {
    let entries = catalog(&["A", "B"]);
    for current in [None, Some("Zed")] {
        let run = |i| resolve_special_name("random", &entries, current, None, pick_expecting(2, i));
        assert_eq!(
            [run(0), run(1)],
            [resolved("A", None), resolved("B", None)],
            "{current:?}"
        );
    }
}

/// `random`: 目録が空なら解けない（乱数は呼ばない）。
#[test]
fn random_with_empty_catalog_is_unresolved() {
    assert_eq!(
        resolve_special_name("random", &[], Some("A"), None, no_pick),
        NameResolution::Unresolved(UnresolvedReason::RandomEmpty),
    );
}

/// `sequential`: 今の位置の次・末尾なら先頭・目録に無ければ先頭・1 体なら自分自身・空なら
/// 解けない。乱数は呼ばない。
#[test]
fn sequential_moves_to_next_in_catalog_order() {
    let abc = catalog(&["A", "B", "C"]);
    let only_a = catalog(&["A"]);
    let seq = |entries: &[GhostEntry], current| {
        resolve_special_name("sequential", entries, current, None, no_pick)
    };
    assert_eq!(
        [
            seq(&abc, Some("A")),
            seq(&abc, Some("C")),
            seq(&abc, None),
            seq(&abc, Some("Zed")),
            seq(&only_a, Some("A")),
            seq(&[], Some("A")),
        ],
        [
            resolved("B", Some(0)),
            resolved("A", Some(2)),
            resolved("A", None),
            resolved("A", None),
            resolved("A", Some(0)),
            NameResolution::Unresolved(UnresolvedReason::SequentialEmpty),
        ],
        "(次・末尾→先頭・None・目録に無い名前・1 体・空)"
    );
}

/// `lastinstalled`: 記録なし・記録あり・記録のゴーストが目録に無い・記録が今のゴースト自身。
/// 乱数は呼ばない。
#[test]
fn lastinstalled_looks_up_the_record_by_folder_name() {
    let entries = catalog(&["A", "B"]);
    let last = |record| resolve_special_name("lastinstalled", &entries, Some("A"), record, no_pick);
    assert_eq!(
        [
            last(None),
            last(Some("B")),
            last(Some("Gone")),
            last(Some("A"))
        ],
        [
            NameResolution::Unresolved(UnresolvedReason::LastInstalledNone),
            resolved("B", None),
            NameResolution::Unresolved(UnresolvedReason::LastInstalledMissing),
            resolved("A", None),
        ],
    );
}

/// 目録に `random` という名前・フォルダ名のゴーストがいても特別な名前として解く（そのゴーストを
/// 名指ししない）。
#[test]
fn special_name_wins_over_a_ghost_with_the_same_name() {
    let entries = [
        catalog_entry("B", None),
        catalog_entry("random", Some("random")),
    ];
    assert_eq!(
        resolve_special_name(
            "random",
            &entries,
            Some("random"),
            None,
            pick_expecting(1, 0)
        ),
        resolved("B", None),
    );
}

/// 解けない理由の `reason` 欄の語と本文は 4 つとも互いに異なり、本文はどれも「無視する」を含む。
#[test]
fn unresolved_reasons_have_distinct_words_and_texts() {
    use std::collections::HashSet;
    let all = [
        UnresolvedReason::RandomEmpty,
        UnresolvedReason::SequentialEmpty,
        UnresolvedReason::LastInstalledNone,
        UnresolvedReason::LastInstalledMissing,
    ];
    assert_eq!(
        all.map(UnresolvedReason::as_ref_str),
        [
            "random_empty",
            "sequential_empty",
            "lastinstalled_none",
            "lastinstalled_missing",
        ],
    );
    let texts: HashSet<_> = all.iter().map(|r| r.describe()).collect();
    assert_eq!(texts.len(), all.len(), "本文が重なる: {texts:?}");
    assert!(texts.iter().all(|t| t.contains("無視する")), "{texts:?}");
}

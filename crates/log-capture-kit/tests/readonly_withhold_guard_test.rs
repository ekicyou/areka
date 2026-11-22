//! 読み取り専用の走りで永続化を全部止める口の呼び手を、許可表と突き合わせる見張り。
//!
//! 全部止める口（定義は `areka-sylphya` の `persist/withhold.rs`）を呼んでよいのは、
//! 走りの状態を入れる `areka-session` の `readonly.rs` だけにする。`crates/**` の全部
//! （本番・テスト・土台）を歩き、コメントを除いた本文の当たりを [`ALLOWED`] と突き合わせる。
//!
//! 走査語は本ファイルの字面に逐語で現れないよう `concat!` で 2 片から組む
//! （`workspace_scan/mod.rs` の冒頭の約束と同じ）。

mod workspace_scan;

use std::collections::BTreeMap;

use workspace_scan::{
    Hit, files_that_must_not_spell_the_tokens, raw_scan_sources, read_source, render, scan_sources,
    scan_tokens, walk_workspace_sources,
};

const WITHHOLD: &str = concat!("withhold_", "everything(");
const TOKENS: [&str; 1] = [WITHHOLD];

/// 当たってよい所と回数。定義の 1 行も当たりに数える。
const ALLOWED: [(&str, usize); 2] = [
    ("crates/areka-session/src/readonly.rs", 1),
    ("crates/areka-sylphya/src/persist/withhold.rs", 1),
];

type Rows = Vec<(String, usize)>;

fn tally(hits: &[Hit]) -> BTreeMap<String, usize> {
    let mut found = BTreeMap::new();
    for hit in hits {
        *found.entry(hit.path.clone()).or_insert(0) += 1;
    }
    found
}

fn workspace_sites() -> BTreeMap<String, usize> {
    tally(&scan_sources(&walk_workspace_sources(), &TOKENS))
}

/// 当たりと許可表の差: (表に無い当たり・表より多い当たり, 当たりの無い表の行・表より少ない当たり)。
fn mismatch(found: &BTreeMap<String, usize>, allowed: &[(&str, usize)]) -> (Rows, Rows) {
    let allowed: BTreeMap<String, usize> =
        allowed.iter().map(|(p, n)| ((*p).to_owned(), *n)).collect();
    let extra = found
        .iter()
        .filter(|(k, n)| allowed.get(*k).is_none_or(|a| *n > a))
        .map(|(k, n)| (k.clone(), *n))
        .collect();
    let missing = allowed
        .iter()
        .filter(|(k, n)| found.get(*k).is_none_or(|f| f < *n))
        .map(|(k, n)| (k.clone(), *n))
        .collect();
    (extra, missing)
}

/// 写しの `(パス, 本文)` の組を走査する。
fn sites_in(files: &[(String, String)]) -> BTreeMap<String, usize> {
    let hits: Vec<Hit> = files
        .iter()
        .flat_map(|(path, src)| {
            scan_tokens(src, &TOKENS)
                .into_iter()
                .map(move |(line, token)| Hit::new(path.clone(), line, token))
        })
        .collect();
    tally(&hits)
}

/// 全部止める口の当たりは許可表と一致する。表に無い当たりも、当たりの無い表の行も赤。
#[test]
fn withhold_calls_match_the_allowed_table() {
    let found = workspace_sites();
    assert_eq!(
        mismatch(&found, &ALLOWED),
        (vec![], vec![]),
        "全部止める口の当たりが許可表と合わない（表に無い当たり, 当たりの無い表の行）: {found:?}"
    );
}

/// 較正: 許可表からどの 1 行を落としても、その行だけが表に無い当たりになる
/// （表の全項目に当たりがあり、走査が空振りしていない）。
#[test]
fn dropping_any_one_row_flags_exactly_that_row() {
    let found = workspace_sites();
    for (i, (path, n)) in ALLOWED.iter().enumerate() {
        let mut table = ALLOWED.to_vec();
        table.remove(i);
        assert_eq!(
            mismatch(&found, &table),
            (vec![((*path).to_owned(), *n)], vec![]),
            "許可表から {path} の行を落としても、その行だけが違反にならない"
        );
    }
}

/// 較正: 木の写しに当たりを 1 つ足す・写しから許可の当たりを消す、のどちらでも赤になる。
#[test]
fn adding_or_removing_one_call_in_a_copy_turns_red() {
    let files: Vec<(String, String)> = walk_workspace_sources()
        .into_iter()
        .map(|p| {
            let src = read_source(&p);
            (p, src)
        })
        .collect();
    let (readonly, _) = ALLOWED[0];

    let mut added = files.clone();
    added.push((
        "crates/areka/src/main.rs".to_owned(),
        format!("persist::withhold::{WITHHOLD});"),
    ));
    assert_eq!(
        mismatch(&sites_in(&added), &ALLOWED),
        (vec![("crates/areka/src/main.rs".to_owned(), 1)], vec![]),
        "木に当たりを 1 つ足しても赤にならない"
    );

    let removed: Vec<(String, String)> = files
        .iter()
        .map(|(path, src)| {
            let src = if path == readonly {
                src.replace(WITHHOLD, "// ")
            } else {
                src.clone()
            };
            (path.clone(), src)
        })
        .collect();
    assert_eq!(
        mismatch(&sites_in(&removed), &ALLOWED),
        (vec![], vec![(readonly.to_owned(), 1)]),
        "許可の当たりを消しても赤にならない"
    );
}

/// 較正: 見張りの置き場のファイルは走査語をコメントの中でも逐語で書かない。
#[test]
fn the_guard_files_never_spell_the_withhold_token() {
    let spelled = raw_scan_sources(&files_that_must_not_spell_the_tokens(&[]), &TOKENS);
    assert!(
        spelled.is_empty(),
        "見張りの置き場に走査語が逐語で書かれている:\n{}",
        render(&spelled)
    );
}

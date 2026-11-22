//! 本番ソースの同期の送信を許可表と突き合わせる見張り。
//!
//! OS のセッションの終了の後始末は UI スレッドで降ろす処理を join で待ち、その間は送られてきた
//! メッセージを配らない。areka の中から UI スレッドの窓へ同期に送ると輪になるので、本番ソース
//! （`crates/<x>/src/` の下・`_tests.rs`／`_test_support.rs` でない・`tests`／`examples` の
//! ディレクトリの下でない）の同期の送信は [`ALLOWED_SYNC_SENDS`] に載った所と回数だけにする。
//! join と送信の重なりの再現は `areka-session` の `session_end_sync_send_tests.rs` が持つ。
//!
//! 走査語は本ファイルの字面に逐語で現れないよう `concat!` で 2 片から組む
//! （`workspace_scan/mod.rs` の冒頭の約束と同じ）。

mod workspace_scan;

use std::collections::BTreeMap;

use workspace_scan::{
    Hit, files_that_must_not_spell_the_tokens, raw_scan_sources, read_source, render, scan_sources,
    scan_tokens, walk_workspace_sources,
};

const SEND_W: &str = concat!("SendMessage", "W(");
const SEND_A: &str = concat!("SendMessage", "A(");
const SEND_TIMEOUT_W: &str = concat!("SendMessage", "TimeoutW(");
const SEND_TIMEOUT_A: &str = concat!("SendMessage", "TimeoutA(");
const SEND_NOTIFY_W: &str = concat!("SendNotify", "MessageW(");
const SEND_NOTIFY_A: &str = concat!("SendNotify", "MessageA(");
const SEND_CALLBACK_W: &str = concat!("SendMessage", "CallbackW(");
const SEND_CALLBACK_A: &str = concat!("SendMessage", "CallbackA(");
/// 開き括弧を付けず、`A`/`W`/`Ex` の全形に当てる。
const BROADCAST: &str = concat!("Broadcast", "SystemMessage");

/// 走査の字面（左端は区切り＝識別子の途中では当てない）。同じ族の A/W は両形を入れる。
/// `SendNotifyMessage*`（別スレッドの窓へは非同期・同じスレッドの窓へは同期）と
/// `SendMessageCallback*`（非同期）も、宛先のスレッドと重なり方を人が確かめるべき送信として
/// 同じ表で扱う。足すなら表へ理由つきで載せる。
/// `PostMessage*`・`PostThreadMessage*` は送り手が待たないので対象外。取り込みの行は括弧を
/// 伴わないので当たらない＝呼び出しだけを数える。
/// 数えない形: `use` の別名での呼び出し・型引数つきの呼び出し・`SendDlgItemMessage*`。
const SYNC_SEND_TOKENS: [&str; 9] = [
    SEND_W,
    SEND_A,
    SEND_TIMEOUT_W,
    SEND_TIMEOUT_A,
    SEND_NOTIFY_W,
    SEND_NOTIFY_A,
    SEND_CALLBACK_W,
    SEND_CALLBACK_A,
    BROADCAST,
];

/// 当たりの鍵: (ワークスペース根からの相対パス, 字面, 行がファイル直下の `#[cfg(test)] mod` の中か)。
type SendKey = (String, String, bool);

type Rows = Vec<(SendKey, usize)>;

/// 本番ソースで同期の送信を呼んでよい所と回数。
/// - IPC の送信: shiori のスレッド → 32bit の補助プロセスの message-only 窓（別プロセス）と、
///   補助プロセス → shiori のスレッドの親窓の応答の両方が、この 1 か所を通る。
/// - 試験用の SHIORI DLL（`shiori-host32-testdll`・`OnTestReenter`）: 32bit の補助プロセスの
///   テストだけが読む DLL が、`request` の中から補助プロセス自身の窓（同じスレッド）へ
///   `WM_COPYDATA` を送り返す。areka は読まない。
/// - 標準のツールチップ: UI スレッドが自分で作ったツールチップの窓へ
///   `TTM_*` などを送る。宛先が同じスレッドの窓なので送信は窓の手続きの直接の呼び出しになり、
///   キューを通らない＝join と輪にならない。
/// - UI スレッドの土台:
///   `util::Window` の窓の手続きが `WM_NCCREATE` を、差し替えたばかりの同じ窓の手続きへ送り直す
///   （同じスレッドの窓なので直接の呼び出し）。`lib.rs` の 2 か所はクレートの中の
///   `#[cfg(test)] mod` で、テストが自分のスレッドのモーダルの窓へ `WM_CLOSE` を送る。
const ALLOWED_SYNC_SENDS: [(&str, &str, bool, usize); 5] = [
    (
        "crates/shiori-host32-ipc/src/lib.rs",
        SEND_TIMEOUT_W,
        false,
        1,
    ),
    ("crates/shiori-host32-testdll/src/lib.rs", SEND_W, false, 1),
    ("crates/wintf/src/ecs/tooltip/os.rs", SEND_W, false, 9),
    ("crates/wintf-winmsg-executor/src/lib.rs", SEND_W, true, 2),
    (
        "crates/wintf-winmsg-executor/src/util/window.rs",
        SEND_W,
        false,
        1,
    ),
];

/// 本番ソースか: `crates/<x>/src/` の下で、`tests`／`examples` のディレクトリの下でなく、
/// テストとテストの土台（`_tests.rs`・`_test_support.rs`）でない。
fn is_production_source(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let [first, _, src, dirs @ .., file] = parts.as_slice() else {
        return false;
    };
    *first == "crates"
        && *src == "src"
        && !dirs.iter().any(|d| *d == "tests" || *d == "examples")
        && !file.ends_with("_tests.rs")
        && !file.ends_with("_test_support.rs")
}

fn production_sources() -> Vec<String> {
    walk_workspace_sources()
        .into_iter()
        .filter(|p| is_production_source(p))
        .collect()
}

/// 各行がファイル直下の `#[cfg(test)]` に続く、その場で本体を持つ `mod t {` の中か。
/// 中は `mod` の行から、そのモジュールを閉じる列 0 の `}` まで（rustfmt の形に依る）。
/// 外部ファイルの宣言（`mod x_tests;`）は本体を持たないので中へ入らない。
fn inline_test_mod_lines(src: &str) -> Vec<bool> {
    let lines: Vec<&str> = src.lines().collect();
    let mut inside = false;
    let mut out = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        if *line == "#[cfg(test)]"
            && lines
                .get(i + 1)
                .is_some_and(|n| n.starts_with("mod ") && n.trim_end().ends_with('{'))
        {
            inside = true;
        } else if inside && *line == "}" {
            inside = false;
        }
        out.push(inside);
    }
    out
}

/// 当たりを鍵ごとに数える。`source_of` は当たりのファイルの元の本文を返す。
fn tally(hits: &[Hit], source_of: impl Fn(&str) -> String) -> BTreeMap<SendKey, usize> {
    let mut flags: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    let mut found = BTreeMap::new();
    for hit in hits {
        let in_mod = flags
            .entry(hit.path.clone())
            .or_insert_with(|| inline_test_mod_lines(&source_of(&hit.path)))
            .get(hit.line - 1)
            .copied()
            .unwrap_or(false);
        *found
            .entry((hit.path.clone(), hit.token.clone(), in_mod))
            .or_insert(0) += 1;
    }
    found
}

fn workspace_sites() -> BTreeMap<SendKey, usize> {
    tally(
        &scan_sources(&production_sources(), &SYNC_SEND_TOKENS),
        read_source,
    )
}

/// 写しの `(パス, 本文)` の組を走査する（較正で本番ソースを書き換えた形を見る）。
fn sites_in(files: &[(String, String)]) -> BTreeMap<SendKey, usize> {
    let hits: Vec<Hit> = files
        .iter()
        .flat_map(|(path, src)| {
            scan_tokens(src, &SYNC_SEND_TOKENS)
                .into_iter()
                .map(move |(line, token)| Hit::new(path.clone(), line, token))
        })
        .collect();
    tally(&hits, |path| {
        files
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, src)| src.clone())
            .unwrap_or_default()
    })
}

fn key(path: &str, token: &str, in_mod: bool) -> SendKey {
    (path.to_owned(), token.to_owned(), in_mod)
}

/// 当たりと許可表の差: (表に無い当たり・表より多い当たり, 当たりの無い表の行・表より少ない当たり)。
fn mismatch(
    found: &BTreeMap<SendKey, usize>,
    allowed: &[(&str, &str, bool, usize)],
) -> (Rows, Rows) {
    let allowed: BTreeMap<SendKey, usize> = allowed
        .iter()
        .map(|(path, token, in_mod, n)| (key(path, token, *in_mod), *n))
        .collect();
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

/// 本番ソースの同期の送信は許可表と一致する。表に無い当たりも、当たりの無い
/// 表の行も赤。新しい送信を足すなら、宛先のスレッドと OS のセッションの終了の join と輪に
/// ならないことを確かめ、`areka-session` の `session_end.rs` の受け手の説明と表を一緒に直す。
#[test]
fn production_sync_sends_match_the_allowed_table() {
    let found = workspace_sites();
    assert_eq!(
        mismatch(&found, &ALLOWED_SYNC_SENDS),
        (vec![], vec![]),
        "本番ソースの同期の送信が許可表と合わない（表に無い当たり, 当たりの無い表の行）: {found:?}"
    );
}

/// 較正: 許可表からどの 1 行を落としても、その行だけが表に無い当たりになる
/// （表の全項目に当たりがあり、走査が空振りしていない）。
#[test]
fn dropping_any_one_row_flags_exactly_that_row() {
    let found = workspace_sites();
    for (i, (path, token, in_mod, n)) in ALLOWED_SYNC_SENDS.iter().enumerate() {
        let mut table = ALLOWED_SYNC_SENDS.to_vec();
        table.remove(i);
        assert_eq!(
            mismatch(&found, &table),
            (vec![(key(path, token, *in_mod), *n)], vec![]),
            "許可表から {path} の行を落としても、その行だけが違反にならない"
        );
    }
}

/// 較正: 本番ソースの写しに当たりを 1 つ足す・写しから許可の当たりを消す、のどちらでも赤になる。
#[test]
fn adding_or_removing_one_call_in_a_copy_turns_red() {
    let files: Vec<(String, String)> = production_sources()
        .into_iter()
        .map(|p| {
            let src = read_source(&p);
            (p, src)
        })
        .collect();
    let ipc = ALLOWED_SYNC_SENDS[0];

    let mut added = files.clone();
    added.push((
        "crates/areka/src/main.rs".to_owned(),
        format!("let _ = unsafe {{ {SEND_W}hwnd, WM_NULL, None, None) }};"),
    ));
    assert_eq!(
        mismatch(&sites_in(&added), &ALLOWED_SYNC_SENDS),
        (
            vec![(key("crates/areka/src/main.rs", SEND_W, false), 1)],
            vec![]
        ),
        "本番ソースに当たりを 1 つ足しても赤にならない"
    );

    let removed: Vec<(String, String)> = files
        .iter()
        .map(|(path, src)| {
            let src = if path == ipc.0 {
                src.replace(ipc.1, &format!("// {}", ipc.1))
            } else {
                src.clone()
            };
            (path.clone(), src)
        })
        .collect();
    assert_eq!(
        mismatch(&sites_in(&removed), &ALLOWED_SYNC_SENDS),
        (vec![], vec![(key(ipc.0, ipc.1, ipc.2), ipc.3)]),
        "許可の当たりを消しても赤にならない"
    );
}

/// 較正: 「テストのモジュールの中」の欄は、`#[cfg(test)]` の次行が本体を持つ `mod t {` の
/// ときだけ真で、外部ファイルの宣言（`mod x_tests;`）の後ろに続く本番の行は偽のまま。
#[test]
fn only_inline_test_modules_are_marked_as_inside() {
    let call = format!("unsafe {{ {SEND_W}h, WM_NULL, None, None) }};");
    let files = [
        (
            "decl.rs".to_owned(),
            format!("#[cfg(test)]\nmod x_tests;\n\nfn send() {{\n    {call}\n}}\n"),
        ),
        (
            "inline.rs".to_owned(),
            format!("#[cfg(test)]\nmod t {{\n    fn send() {{\n        {call}\n    }}\n}}\n"),
        ),
    ];
    assert_eq!(
        sites_in(&files),
        BTreeMap::from([
            (key("decl.rs", SEND_W, false), 1),
            (key("inline.rs", SEND_W, true), 1),
        ]),
        "テストのモジュールの中かの判定が外部ファイルの宣言と本体を持つモジュールを取り違える"
    );
}

/// 較正: 見張りの置き場のファイルは走査語をコメントの中でも逐語で書かない。
#[test]
fn the_guard_files_never_spell_the_sync_send_tokens() {
    let spelled = raw_scan_sources(
        &files_that_must_not_spell_the_tokens(&[]),
        &SYNC_SEND_TOKENS,
    );
    assert!(
        spelled.is_empty(),
        "見張りの置き場に同期の送信の走査語が逐語で書かれている:\n{}",
        render(&spelled)
    );
}

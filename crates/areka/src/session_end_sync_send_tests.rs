//! OS のセッションの終了の後始末が降ろす処理を待つ（join）間に、別スレッドから UI スレッドの窓へ
//! 同期の送信が重なっても止まらないことの再現（areka-P0-session-mark-residue task 6.1・要件 5.2・
//! 5.4・5.5・7.5）。
//!
//! `research.md` §6 の洗い出しでは、areka の中で join と重なって UI スレッドを待つ箇所は 0 件
//! だった。要件 5.2 に従い、代表の形（待ちの最中に別スレッドから UI の窓へ同期の送信を 1 通）を
//! 起こす。登場するスレッドは 4 本:
//! - UI 役（[`run_bounded`] が起こすスレッド）: message-only 窓を作り、`OnClose` で固まる偽の
//!   SHIORI で A を起こし、送信が自分のキューに届いたのを見てから、大きな上限で
//!   [`end_session_within`] を呼ぶ（見張りは発火しない）。後始末が戻ったら連番を取り、
//!   メッセージを 1 回だけ覗く（送られてきたメッセージが配られ、送信が返る）。
//! - 送り手: 「送る」の旗を立ててから UI 役の窓へ期限つきで同期に送る。返ったら連番を取る。
//! - 解き手: 旗と偽の SHIORI の固まりを見てから偽の SHIORI を解く（送信の返りは待たない）。
//! - ゴーストの実行系のスレッド（kanade・shiori ほか）: UI 役が join で待つ相手。
//!
//! 緑の意味: 後始末は送信に依らず戻り（連番で 後始末の戻り < 送信の戻り）、送信は UI 役が次に
//! メッセージを取り出した時点で期限内に返る。join が送信を待つ形（輪）に変わると、送信は期限で
//! 切れ、後始末はその後にしか戻れないので赤になる。join の最中に送られてきたメッセージを配る形に
//! 変わると、送信が後始末より先に返るので赤になる（受け手の説明「送られてきたメッセージは配らない」
//! が崩れた）。
//!
//! 順序は sleep で作らない: 送信が UI 役のキューに届いたことは `GetQueueStatus`（配らずに読む）で、
//! 偽の SHIORI の固まりは `holding()` で見る。走行ごと固まらないよう、全体を [`run_bounded`] で
//! 囲み、送信にも期限（[`SEND_TIMEOUT_MS`]）を付ける。
//!
//! 後半はワークスペースの本番ソースの同期の送信を許可表（[`ALLOWED_SYNC_SENDS`]）と突き合わせる
//! 検査と、その較正（task 6.2・要件 5.4・7.5）。

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

use log_capture_kit::capture;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetQueueStatus, HWND_MESSAGE, MSG, PM_NOREMOVE, PeekMessageW, QS_SENDMESSAGE,
    SMTO_NORMAL, SendMessageTimeoutW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_NULL,
};
use windows::core::w;

use super::end_session_within;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::hold_support::HoldAt;
use crate::emo2_boot::spine::{Progress, RecordedCall, run_bounded, wait_until};

/// 見張りが発火しない上限（後始末が戻るのは解き手が解いたときだけにする）。
const HOUR: Duration = Duration::from_secs(3600);
/// 送り手の同期の送信の期限（切れたら赤＝輪になった）。
const SEND_TIMEOUT_MS: u32 = 10_000;
/// 再現の全体の上限（走行ごと固まらない）。
const WHOLE: Duration = Duration::from_secs(120);

/// UI 役のスレッドで見える結果。
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    /// 後始末が戻った時点で、送信はまだ UI 役のキューで待っていたか（join の間は配らない）。
    pending_after_teardown: bool,
    /// 後始末の戻りが送信の戻りより先か（連番）。
    teardown_before_send: bool,
    /// 送信が期限内に返ったか（`SendMessageTimeoutW` の戻り値が 0 でない）。
    send_ok: bool,
    /// 解く手が呼ばれた回数（解き手の 1 回だけ＝見張りは発火していない）。
    unblock_calls: usize,
    /// `OnClose` の NOTIFY の Reference0（届いた分だけ）。
    on_close_ref0: Vec<String>,
    /// `os_session_end_done` の `shiori_cut`（上限で切っていない）。
    shiori_cut: Option<String>,
}

/// 今のスレッドのキューに、別スレッドからの同期の送信が待っているか（配らずに読む）。
fn send_pending() -> bool {
    // SAFETY: キューの状態を読むだけで、送られてきたメッセージは配らない。上位語＝今キューにある種類。
    (unsafe { GetQueueStatus(QS_SENDMESSAGE) } >> 16) & QS_SENDMESSAGE.0 != 0
}

/// 今のスレッドに `STATIC`・`HWND_MESSAGE`（message-only）の窓を作り、取っ手を数で返す
/// （`HWND` はスレッドをまたいで渡せないので数にする）。
fn create_ui_window() -> isize {
    // SAFETY: 引数はすべて静的な文字列か空。親は HWND_MESSAGE（message-only）。
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("session_end_sync_send"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )
    }
    .expect("message-only 窓を作れる");
    hwnd.0 as isize
}

/// 窓 `hwnd` へ `WM_NULL` を期限つき・応答なし判定なし（`SMTO_NORMAL`）で同期に送り、
/// 戻り値が 0 でないか（期限内に返ったか）を返す。
fn send_null(hwnd: isize) -> bool {
    let mut result = 0usize;
    // SAFETY: ポインタを運ばない問い合わせ。宛先の窓は UI 役のスレッドが生かしている。
    let ret = unsafe {
        SendMessageTimeoutW(
            HWND(hwnd as *mut _),
            WM_NULL,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            SEND_TIMEOUT_MS,
            Some(&mut result as *mut usize),
        )
    };
    ret.0 != 0
}

/// 呼出列のうち `OnClose` の NOTIFY の Reference0（届いた分だけ）。
fn on_close_ref0(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .filter_map(|c| match c {
            RecordedCall::Notify { id, references } if id == "OnClose" => {
                Some(references.first().cloned().unwrap_or_default())
            }
            _ => None,
        })
        .collect()
}

/// UI 役のスレッドの本体（モジュールの説明の手順）。
fn reproduce() -> Observed {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| {
            standard_script("\\0A\\e").hold_at(HoldAt::Notify("OnClose"))
        })),
    )]);
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着かない");
    let handle = rig.handle("A");
    let hwnd = create_ui_window();

    let seq = Arc::new(AtomicU8::new(0));
    let sending = Arc::new(AtomicBool::new(false));
    let sender = {
        let (seq, sending) = (Arc::clone(&seq), Arc::clone(&sending));
        thread::spawn(move || {
            sending.store(true, Ordering::SeqCst);
            let ok = send_null(hwnd);
            (ok, seq.fetch_add(1, Ordering::SeqCst))
        })
    };
    // 送信が届いてから後始末に入る（join の間に送信が待っていることを時刻に依らず作る）。
    assert!(
        rig.wait_for(send_pending),
        "送り手の送信が UI 役のキューに届かない"
    );
    let releaser = {
        let (sending, handle) = (Arc::clone(&sending), handle.clone());
        thread::spawn(move || {
            // 足場を持ち込めない別のスレッドなので、偽の SHIORI が受けた呼び出しの数を進みの目印にする
            // （areka-P0-ghost-session-test-load-flake 要件 2.1・2.2）。
            let waited = wait_until(
                "解き手: 送信の旗と偽の SHIORI の固まり",
                Progress::Count(&|| handle.call_count()),
                || sending.load(Ordering::SeqCst) && handle.holding(),
            );
            if let Err(failure) = &waited {
                eprintln!("{failure}");
            }
            let ready = waited.is_ok();
            if ready {
                handle.release();
            }
            ready
        })
    };

    let ((), events) = capture(|| end_session_within(&mut rig.world, HOUR));
    let teardown_seq = seq.fetch_add(1, Ordering::SeqCst);
    let pending_after_teardown = send_pending();
    let mut msg = MSG::default();
    // SAFETY: 今のスレッドのキューを 1 回覗くだけ（取り除かない）。待っている送信はここで配られる。
    let _ = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE) };

    let (send_ok, send_seq) = sender.join().expect("送り手は panic しない");
    assert!(
        releaser.join().expect("解き手は panic しない"),
        "解き手が送信の旗と偽の SHIORI の固まりを見られない"
    );
    Observed {
        pending_after_teardown,
        teardown_before_send: teardown_seq < send_seq,
        send_ok,
        unblock_calls: handle.unblock_calls(),
        on_close_ref0: on_close_ref0(&rig.calls("A").last().cloned().unwrap_or_default()),
        shiori_cut: events
            .iter()
            .find(|e| e.field_str("event") == Some("os_session_end_done"))
            .and_then(|e| e.field_str("shiori_cut").or_else(|| e.field("shiori_cut")))
            .map(str::to_owned),
    }
}

/// join の最中に別スレッドから UI の窓へ同期の送信が重なっても、後始末は送信に依らず戻り、
/// 送信は後始末の後に UI 役が 1 回覗いた時点で期限内に返る（要件 5.2・5.4・7.5）。
/// `OnClose` は Ref0＝`system` で 1 件のまま（要件 5.5）。
#[test]
fn teardown_join_does_not_wait_for_a_sync_send_to_the_ui_window() {
    let (tx, rx) = std::sync::mpsc::channel();
    run_bounded(
        "join と同期の送信の重なりの再現",
        WHOLE,
        move || {
            let _ = tx.send(reproduce());
        },
    );
    assert_eq!(
        rx.recv().expect("再現の結果が届く"),
        Observed {
            pending_after_teardown: true,
            teardown_before_send: true,
            send_ok: true,
            unblock_calls: 1,
            on_close_ref0: vec!["system".to_owned()],
            shiori_cut: Some("false".to_owned()),
        },
        "join と同期の送信が重なった（後始末が送信を待った・join の間に配った・送信が期限で切れた）"
    );
}

// ============================ 本番ソースの同期の送信と許可表（要件 5.4・7.5）

/// 走査の字面（左端は区切り＝識別子の途中では当てない）。design「要件 5 の検査」の語に、
/// 同じ族の A/W の片割れを足した。`SendNotifyMessage*`（別スレッドの窓へは非同期・同じスレッドの
/// 窓へは同期）と `SendMessageCallback*`（非同期）も、宛先のスレッドと重なり方を人が確かめる
/// べき送信として同じ表で扱う（今日の本番の当たりは 0 件なので、足すなら表へ理由つきで載せる）。
/// `BroadcastSystemMessage` は開き括弧を付けず、`A`/`W`/`Ex` の全形に当てる。
/// `PostMessage*`・`PostThreadMessage*` は送り手が待たないので対象外。取り込みの行
/// （`use ... SendMessageTimeoutW,`）は括弧を伴わないので当たらない＝呼び出しだけを数える。
/// 数えない形（今日 0 件）: `use ...::SendMessageW as X;` の別名での呼び出し・
/// `SendMessageW::<..>(` の型引数つきの呼び出し・`SendDlgItemMessage*`。
const SYNC_SEND_TOKENS: [&str; 9] = [
    "SendMessageW(",
    "SendMessageA(",
    "SendMessageTimeoutW(",
    "SendMessageTimeoutA(",
    "SendNotifyMessageW(",
    "SendNotifyMessageA(",
    "SendMessageCallbackW(",
    "SendMessageCallbackA(",
    "BroadcastSystemMessage",
];

/// 当たりの鍵: (ワークスペース根からの相対パス, 字面, 行がファイル直下の `#[cfg(test)] mod` の中か)。
type SendKey = (String, &'static str, bool);

/// 本番ソースで同期の送信を呼んでよい所と回数（`research.md` §6 の洗い出し）。
/// - IPC の送信: shiori のスレッド → 32bit の補助プロセスの message-only 窓（別プロセス）と、
///   補助プロセス → shiori のスレッドの親窓の応答の両方が、この 1 か所を通る。
/// - 親窓のテスト: 不正なフレームを親窓へ自分のスレッドから送る（ファイル内のテストの
///   モジュールの中＝本番のビルドには入らない）。
/// - 標準のツールチップ（areka-P0-wintf-tooltip）: UI スレッドが自分で作ったツールチップの窓へ
///   `TTM_*` などを送る。宛先が同じスレッドの窓なので送信は窓の手続きの直接の呼び出しになり、
///   キューを通らない＝join と輪にならない。
const ALLOWED_SYNC_SENDS: [(&str, &str, bool, usize); 3] = [
    (
        "crates/shiori-host32-ipc/src/lib.rs",
        "SendMessageTimeoutW(",
        false,
        1,
    ),
    (
        "crates/shiori-host32-host/src/parent_window.rs",
        "SendMessageW(",
        true,
        1,
    ),
    (
        "crates/wintf/src/ecs/tooltip/os.rs",
        "SendMessageW(",
        false,
        9,
    ),
];

/// ワークスペースの `crates/*/src/**/*.rs` の本番ソースを `(根からの相対パス, 本文)` で集める。
/// 除くのはテストとテストの土台（`_tests.rs`・`_test_support.rs`）と、`tests`・`examples` の
/// ディレクトリの下（design「要件 5 の検査」の範囲。結合テストと examples は `src` の外なので
/// そもそも歩かない）。
fn workspace_production_sources() -> Vec<(String, String)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = std::fs::read_dir(dir).unwrap_or_else(|e| {
            panic!("木を歩けない（走査が空振りする）: {} — {e}", dir.display())
        });
        for entry in entries {
            let path = entry.expect("ディレクトリ項目が読めない").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_dir() {
                if name != "tests" && name != "examples" {
                    walk(root, &path, out);
                }
                continue;
            }
            if !name.ends_with(".rs")
                || name.ends_with("_tests.rs")
                || name.ends_with("_test_support.rs")
            {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .expect("走査の根の下に無い")
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&path).expect("本番ファイルが読めない");
            out.push((rel, src));
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let crates = std::fs::read_dir(root.join("crates")).expect("crates を読めない");
    let mut out = Vec::new();
    for krate in crates {
        let src = krate.expect("crates の項目が読めない").path().join("src");
        if src.is_dir() {
            walk(&root, &src, &mut out);
        }
    }
    out
}

/// 走査の当たりを鍵ごとに数える。注釈の行（`//`・`///`・`//!` で始まる行）は除く。
/// ファイル直下の `#[cfg(test)]` に続く、その場で本体を持つ `mod t {` の行から、そのモジュールを
/// 閉じる列 0 の `}` までを「テストのモジュールの中」とする（rustfmt の形に依る）。外部ファイルの
/// 宣言（`mod x_tests;`）は本体を持たないので中へ入らない。
fn sync_send_sites(files: &[(String, String)]) -> BTreeMap<SendKey, usize> {
    let mut found = BTreeMap::new();
    for (rel, src) in files {
        let lines: Vec<&str> = src.lines().collect();
        let mut in_test_mod = false;
        for (i, line) in lines.iter().enumerate() {
            if *line == "#[cfg(test)]"
                && lines
                    .get(i + 1)
                    .is_some_and(|n| n.starts_with("mod ") && n.trim_end().ends_with('{'))
            {
                in_test_mod = true;
            } else if in_test_mod && *line == "}" {
                in_test_mod = false;
            }
            let body = line.trim_start();
            if body.starts_with("//") {
                continue;
            }
            for token in SYNC_SEND_TOKENS {
                let hits = body
                    .match_indices(token)
                    .filter(|(at, _)| {
                        !body[..*at]
                            .chars()
                            .next_back()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_')
                    })
                    .count();
                if hits > 0 {
                    *found.entry((rel.clone(), token, in_test_mod)).or_insert(0) += hits;
                }
            }
        }
    }
    found
}

/// 当たりと許可表の差: (表に無い当たり・表より多い当たり, 当たりの無い表の行・表より少ない当たり)。
fn sync_send_mismatch(
    found: &BTreeMap<SendKey, usize>,
    allowed: &[(&str, &'static str, bool, usize)],
) -> (Vec<(SendKey, usize)>, Vec<(SendKey, usize)>) {
    let allowed: BTreeMap<SendKey, usize> = allowed
        .iter()
        .map(|(rel, token, in_test, n)| (((*rel).to_owned(), *token, *in_test), *n))
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

/// 本番ソースの同期の送信は許可表と一致する（要件 5.4・7.5）。表に無い当たりも、当たりの無い
/// 表の行も赤。新しい送信を足すなら、宛先のスレッドと OS のセッションの終了の join と輪に
/// ならないことを確かめ、`session_end.rs` の受け手の説明と表を一緒に直す。
#[test]
fn production_sync_sends_match_the_allowed_table() {
    let found = sync_send_sites(&workspace_production_sources());
    assert_eq!(
        sync_send_mismatch(&found, &ALLOWED_SYNC_SENDS),
        (vec![], vec![]),
        "本番ソースの同期の送信が許可表と合わない（表に無い当たり, 当たりの無い表の行）: {found:?}"
    );
}

/// 走査の較正: 許可表から 1 行消す・本番ソースの写しに当たりを 1 つ足す・写しから許可の
/// 当たりを消す、のどれでも赤になる（検査が空振りしていない）。
#[test]
fn sync_send_scan_turns_red_on_one_row_removed_or_one_call_added() {
    let files = workspace_production_sources();
    let found = sync_send_sites(&files);
    let ipc = (
        "crates/shiori-host32-ipc/src/lib.rs".to_owned(),
        "SendMessageTimeoutW(",
        false,
    );

    // ⑴ 許可表から IPC の行を消す → 表に無い当たり。
    assert_eq!(
        sync_send_mismatch(&found, &ALLOWED_SYNC_SENDS[1..]),
        (vec![(ipc.clone(), 1)], vec![]),
        "許可表から 1 行消しても赤にならない"
    );

    // ⑵ 本番ソースの写しに UI の窓への同期の送信を 1 つ足す → 表に無い当たり。
    let mut added = files.clone();
    added.push((
        "crates/areka/src/main.rs".to_owned(),
        "let _ = unsafe { SendMessageW(hwnd, WM_NULL, None, None) };".to_owned(),
    ));
    assert_eq!(
        sync_send_mismatch(&sync_send_sites(&added), &ALLOWED_SYNC_SENDS),
        (
            vec![(
                (
                    "crates/areka/src/main.rs".to_owned(),
                    "SendMessageW(",
                    false
                ),
                1
            )],
            vec![]
        ),
        "本番ソースに当たりを 1 つ足しても赤にならない"
    );

    // ⑶ 写しから IPC の送信を消す → 当たりの無い表の行（注釈の行は数えない）。
    let removed: Vec<(String, String)> = files
        .iter()
        .map(|(rel, src)| {
            let src = if *rel == ipc.0 {
                src.replace("SendMessageTimeoutW(", "// SendMessageTimeoutW(")
            } else {
                src.clone()
            };
            (rel.clone(), src)
        })
        .collect();
    assert_eq!(
        sync_send_mismatch(&sync_send_sites(&removed), &ALLOWED_SYNC_SENDS),
        (vec![], vec![(ipc, 1)]),
        "許可の当たりを消しても赤にならない"
    );
}

/// 走査の較正: 「テストのモジュールの中」の欄は、`#[cfg(test)]` の次行が本体を持つ `mod t {` の
/// ときだけ真で、外部ファイルの宣言（`mod x_tests;`）の後ろに続く本番の行は偽のまま。
#[test]
fn sync_send_scan_marks_only_inline_test_modules() {
    let files = [
        (
            "decl.rs".to_owned(),
            "#[cfg(test)]\nmod x_tests;\n\nfn send() {\n    unsafe { SendMessageW(h, WM_NULL, None, None) };\n}\n"
                .to_owned(),
        ),
        (
            "inline.rs".to_owned(),
            "#[cfg(test)]\nmod t {\n    fn send() {\n        unsafe { SendMessageW(h, WM_NULL, None, None) };\n    }\n}\n"
                .to_owned(),
        ),
    ];
    assert_eq!(
        sync_send_sites(&files),
        BTreeMap::from([
            (("decl.rs".to_owned(), "SendMessageW(", false), 1),
            (("inline.rs".to_owned(), "SendMessageW(", true), 1),
        ]),
        "テストのモジュールの中かの判定が外部ファイルの宣言と本体を持つモジュールを取り違える"
    );
}

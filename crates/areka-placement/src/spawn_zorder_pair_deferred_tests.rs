//! 先送りした正典語彙が areka 側の本番コードに入り込んでいないこと、および窓の既定が
//! 「常時最前面ではない・最小化もタスクバー露出も足していない」ままであることを
//! 固定するテスト（要件 8.1／8.3／8.4／8.5、および重なり順のグループ機構の要件 10.4／11.4）。
//!
//! # 兄弟ファイルとの分担
//!
//! wintf 側の本番ファイル 8 本に対する同じ形の走査は
//! `crates/wintf/src/ecs/window/zorder_pair_deferred_vocabulary_tests.rs` が持つ。
//! areka 側の 7 本——ペア機構の 2 本（ペア宣言を付ける組立＝`spawn.rs`、その記録＝`diag.rs`）と、
//! グループ機構が新設した 5 本——は crate ごとに 2 つの片割れで受け持つ（spec:
//! placement-crate-split 要件 5.2・5.3）。本ファイルは配置の crate の片割れで、配置の 4 本と
//! 配置の `src` の木を見る。本体の 3 本（`emo2_boot` のもの）・帯のファイル・本体の `src` の木は
//! 本体の `crates/areka/src/placement_tests/spawn_zorder_pair_deferred_tests.rs` が見る。
//! 語の表と走査器は 2 つの片割れが共に使う [`crate::test_support::zorder_pair_deferred`] に
//! 1 つだけあり、どちらの片割れも自分の crate の実物で較正する。
//!
//! 窓の拡張スタイルを完全一致で押さえるのは
//! `spawn_assembly_tests::t_i2_no_window_has_ws_ex_topmost` である（`ex_style` と `style` の
//! 両方を等値で固定しているので、常時最前面・最小化・タスクバー露出のどのビットも
//! 紛れ込む余地が無い）。本ファイルの窓側テストはそれを置き換えるものではなく、
//! **要件 8.1／8.5 が名指しするビットを名前で読めるようにする**ためのものである
//! ——完全一致は「何が禁じられているか」を書き残さないので、後から来た人が
//! `WS_EX_APPWINDOW` を足してよいかを判断できない。
//!
//! # 走査の範囲
//!
//! 要件 8.3〜8.5（ペア機構）と要件 11.4（グループ機構）は、いずれも「**本フィーチャーに
//! おいて**足さない」ことを言う。よって走査は、2 つの機構が全部を書いた areka 側の本番
//! ファイルに限る。`main.rs` の結線 1 行や、`balloon_visibility_phase.rs`・`frame.rs` の
//! ような他フィーチャーと共有の巨大ファイルは、全文走査に入れると他所の追記で赤くなって
//! 本フィーチャーへの誤った告発になるため入れない（結線が呼ぶ `wire_zorder_pair` の本体は
//! `spawn.rs` にあり、走査の対象に入っている）。
//!
//! グループ機構が新設した本番ファイルが走査から漏れないことは、
//! `the_scanned_roster_covers_every_zorder_production_source_in_this_crate` が
//! 実在するファイルと名簿の両方向で見張る（要件 10.4）。
//!
//! # 先送りの語と帯の語（`stay-on-top-directives` で分けた）
//!
//! wintf 側と同じく語の表は 2 つに分かれる。最小化・タスクバーの語（`DEFERRED_NEEDLES`）は今も
//! 先送りのまま。常に手前の帯の語（`BAND_NEEDLES`）は `stay-on-top-directives` が実装したが、
//! areka 側でそれを持つのは本体の受け口（`emo2_boot/window_state_cue.rs`）と取り出しの相
//! （`emo2_boot/frame/window_state.rs`）だけと決めてある。よってどちらの表の語も重なり順の機構の
//! 本番ファイルには無いことを主張し続ける（帯の語の持ち主に語が在る対照は本体の片割れが持つ）。
//!
//! # 行コメントを除いてから探す理由
//!
//! `spawn.rs` の doc コメントには「`WS_EX_TOPMOST` を含めない」という**否定の説明**が
//! 書いてある（要件 8.1 の設計意図の記録そのもの）。素の全文を探すとこの説明で赤くなるので、
//! 行頭が `//` の行を落としてから探す。落とし過ぎ・落とし漏れが無いことは、素の全文には
//! 語があり、コードだけの本文には無い、という対照で示す。

use std::path::PathBuf;

use bevy_ecs::prelude::*;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_MAXIMIZEBOX, WS_MINIMIZE, WS_MINIMIZEBOX,
    WS_SYSMENU,
};
use wintf::ecs::WindowStyle;

use super::spawn_ghost_windows;
use super::test_support::{ghost_window_entities, titles, two_scope_placements};
use crate::test_support::zorder_pair_deferred::{
    BAND_NEEDLES, DEFERRED_NEEDLES, code_only, deferred_hits, rust_sources_under,
};

/// 重なり順の 2 機構が全部を書いた、配置の crate の本番ファイル（`CARGO_MANIFEST_DIR` からの相対）。
///
/// 前半 2 本はペア機構（`ghost-window-zorder`）——ペア宣言を付ける組立（`spawn.rs`）と
/// その記録（`diag.rs`）。後半 2 本はグループ機構（`areka-P0-scope-zorder-pinning`）が
/// 新設したもの（うち `zorder_chain_compose.rs` は鎖方式の合成層。グループ機構の残りの 3 本は
/// 本体にあり、本体の片割れの名簿に載る）。下の
/// `the_scanned_roster_covers_every_zorder_production_source_in_this_crate` が
/// 「この crate に実在する `zorder_` 系の本番ファイルが 1 本残らずここに載っている」ことを機械で見張る。
const PRODUCTION_FILES: [&str; 4] = [
    "src/spawn.rs",
    "src/diag.rs",
    "src/zorder_chain_compose.rs",
    "src/zorder_group_ledger.rs",
];

fn source_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_source(relative: &str) -> String {
    let path = source_path(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("本番ファイルを読めなかった（{}）: {e}", path.display()))
}

/// 先送り語彙は、本フィーチャーの配置の crate の本番ファイルのコードに 1 つも無い。
#[test]
fn deferred_window_state_vocabulary_is_absent_from_this_features_areka_sources() {
    let mut scanned_files = 0usize;
    let mut scanned_code_bytes = 0usize;

    for relative in PRODUCTION_FILES {
        let raw = read_source(relative);
        assert!(
            raw.len() > 1_000,
            "{relative}: 本番ファイルにしては短すぎる（読み違えの疑い）: {} バイト",
            raw.len()
        );
        let code = code_only(&raw);
        assert!(
            !code.trim().is_empty(),
            "{relative}: コメントを落としたら本文が空になった（走査が空振りしている）"
        );

        let hits = deferred_hits(&code);
        assert!(
            hits.is_empty(),
            "{relative}: 先送りの語か帯の語が本番コードに現れている（帯の語を持つのは本体の受け口と取り出しの相だけ）: {hits:?}"
        );

        scanned_files += 1;
        scanned_code_bytes += code.len();
    }

    assert_eq!(
        scanned_files,
        PRODUCTION_FILES.len(),
        "対象ファイルを全部読めていない"
    );
    assert!(
        scanned_code_bytes > 10_000,
        "読んだコードが少なすぎる（走査が実体に届いていない）: {scanned_code_bytes} バイト"
    );
}

/// 走査の道具が実際に語を見つけられること、そしてコメントだけを落としていることを固定する。
#[test]
fn the_areka_deferred_vocabulary_scan_can_actually_find_what_it_looks_for() {
    for (needle, why) in DEFERRED_NEEDLES.into_iter().chain(BAND_NEEDLES) {
        let sample = format!("let handle = SomeApi::{needle}(hwnd);");
        let hits = deferred_hits(&sample);
        assert!(
            hits.iter().any(|(n, _)| *n == needle),
            "語 `{needle}`（{why}）を検出できていない: {hits:?}"
        );
    }
    assert_eq!(
        deferred_hits("ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,")
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>(),
        vec!["topmost"],
        "大文字の綴りを取り逃がしている"
    );
    assert!(
        deferred_hits("ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW,").is_empty(),
        "無関係な本文で語を検出している"
    );

    // 対照: 素の全文には常時最前面の語（否定の説明）があり、コードだけの本文には無い。
    let raw = read_source("src/spawn.rs");
    assert!(
        raw.to_lowercase().contains("topmost"),
        "前提が崩れている: 素の全文には常時最前面の語があるはず"
    );
    let code = code_only(&raw);
    assert!(
        !code.to_lowercase().contains("topmost"),
        "常時最前面の語が説明文以外に現れている（要件 8.1／8.3）"
    );
    // 落とし過ぎていない（走査が中身を見ている）。
    assert!(
        code.contains("KeepDirectlyAbove"),
        "コードだけの本文からペア宣言の実装が消えている"
    );
    assert!(
        code.contains("WS_EX_TOOLWINDOW"),
        "コードだけの本文から窓スタイルの実体が消えている"
    );
}

// -------------------------------------------------------------------------
// 要件 8.1／8.5: 窓の既定は「常時最前面ではない・最小化もタスクバー露出も足さない」
// -------------------------------------------------------------------------

/// 常時最前面の印。
fn has_always_on_top(style: &WindowStyle) -> bool {
    style.ex_style.contains(WS_EX_TOPMOST)
}

/// 最小化・タスクバー露出に関わる印（要件 8.5 が「変更しない」と言っている側）。
fn has_minimize_or_taskbar_exposure(style: &WindowStyle) -> bool {
    style.ex_style.contains(WS_EX_APPWINDOW)
        || style.style.contains(WS_MINIMIZE)
        || style.style.contains(WS_MINIMIZEBOX)
        || style.style.contains(WS_MAXIMIZEBOX)
        || style.style.contains(WS_SYSMENU)
}

/// spawn 直後の全ゴースト窓は、常時最前面でも最小化可能でもなく、タスクバーにも出ない。
///
/// `t_i2_no_window_has_ws_ex_topmost` が `style`／`ex_style` を等値で固定しているので
/// 値そのものは既に動かせない。本テストはその等値が**何を禁じているか**を名前で書き残す
/// ——`WS_EX_APPWINDOW`（タスクバー露出）と最小化系のビットは、足せば要件 8.5 違反である。
///
/// 空虚に緑にならないよう、探針が実際に反応することを先に確かめる（合成した窓スタイルで
/// 両方の判定が真になること）。
#[test]
fn ghost_windows_carry_no_always_on_top_minimize_or_taskbar_bits() {
    // ① 探針の自己検査: 禁じたビットを持つ窓スタイルなら、両方の判定が真になる。
    let offending = WindowStyle {
        style: WS_MINIMIZEBOX | WS_SYSMENU,
        ex_style: WS_EX_TOPMOST | WS_EX_APPWINDOW,
    };
    assert!(
        has_always_on_top(&offending),
        "探針が常時最前面を見落としている"
    );
    assert!(
        has_minimize_or_taskbar_exposure(&offending),
        "探針が最小化・タスクバー露出を見落としている"
    );

    // ② 本番の組立を実際に通す。
    let mut world = World::new();
    let placements = two_scope_placements();
    spawn_ghost_windows(&mut world, &placements, &titles());

    let entities = ghost_window_entities(&mut world);
    assert_eq!(entities.len(), 4, "2 スコープ × キャラ/バルーンで 4 窓");
    for e in entities {
        let style = world.get::<WindowStyle>(e).expect("WindowStyle").clone();
        assert!(
            !has_always_on_top(&style),
            "常時最前面の印が付いている（要件 8.1）: {:?}",
            style.ex_style
        );
        assert!(
            !has_minimize_or_taskbar_exposure(&style),
            "最小化・タスクバー露出の印が付いている（要件 8.5）: {style:?}"
        );
        // 一覧に出ない側の印は逆に**残っている**（現行の見え方を変えていない）。
        assert!(
            style.ex_style.contains(WS_EX_TOOLWINDOW),
            "タスクバー・アプリ切り替え一覧に出さない印が外れている（要件 8.5）: {:?}",
            style.ex_style
        );
    }
}

/// 配置の `src` 配下の `.rs` を、`CARGO_MANIFEST_DIR` からの相対パス（`/` 区切り）で返す。
fn all_source_files() -> Vec<String> {
    rust_sources_under(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// 重なり順の 2 機構（ペア・グループ）が全部を書いた、配置の crate の本番ファイル
/// ＝ファイル名が `zorder_` で始まり、テストでもテスト専用の道具立てでもないもの。
///
/// 共有ファイルへ足した継ぎ目（`spawn.rs`／`diag.rs`）はこの名前の規則に合わないので、
/// 名簿へ人が載せたままになる。よって下の検査は「実物 ⊆ 名簿」の向きだけを機械化し、
/// 名簿が実物より多いことは許す（多い側は ③ が実在で押さえる）。
fn zorder_production_sources() -> Vec<String> {
    all_source_files()
        .into_iter()
        .filter(|relative| {
            let name = relative.rsplit('/').next().unwrap_or(relative);
            name.starts_with("zorder_")
                && !name.ends_with("_tests.rs")
                && !name.ends_with("_test_support.rs")
        })
        .collect()
}

/// 走査対象の一覧が、実在する本番ファイルへ両方向で追随していることを固定する。
///
/// 上の 2 つのテストは `PRODUCTION_FILES` に**載っているものだけ**を読む。名簿が実物から
/// ずれても「無い」の主張はそのまま緑になるので、守りは静かに狭まる（新しい本番ファイルが
/// 生えても誰も赤くならない）。ここでは
/// ⑴ 実在する `zorder_` 系の本番ファイルが 1 本残らず名簿に載っていること、
/// ⑵ 名簿の各項目が実在すること、
/// の両方向を機械で確かめる。走査から外すのはテスト（`*_tests.rs`）と、`#[cfg(test)]` でしか
/// 結線されないテスト専用の道具立て（`*_test_support.rs`）だけである。
#[test]
fn the_scanned_roster_covers_every_zorder_production_source_in_this_crate() {
    let actual = zorder_production_sources();

    // ① 道具の較正（この crate の実物で）: 既知の正例が挙がり、木に在る既知の偽例（テスト）は挙がらない。
    assert!(
        actual.contains(&"src/zorder_group_ledger.rs".to_string()),
        "実物の走査が効いていない（既知の本番ファイルを見つけられない）: {actual:?}"
    );
    assert!(
        all_source_files().contains(&"src/zorder_group_ledger_tests.rs".to_string()),
        "較正の外れが失われた: 既知のテストファイルが木に無い（別の対照へ差し替えること）"
    );
    assert!(
        !actual.iter().any(|p| p.ends_with("_tests.rs")),
        "テストファイルを本番ファイルとして数えている: {actual:?}"
    );

    // ② 実物が名簿から漏れていない。
    let missing: Vec<&String> = actual
        .iter()
        .filter(|relative| !PRODUCTION_FILES.contains(&relative.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "本番ファイルが先送り語彙の走査対象に載っていない（名簿へ足すこと）: {missing:?}"
    );

    // ③ 名簿の項目が実在する（改名・移動で名簿だけが取り残されない）。
    for relative in PRODUCTION_FILES {
        assert!(
            source_path(relative).is_file(),
            "走査対象の一覧に実在しないファイルが載っている: {relative}"
        );
    }
}

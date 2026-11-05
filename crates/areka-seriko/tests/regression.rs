//! 回帰テスト（アクター・発行列観測）— 冪等性・未解決混在の継続・emo2 alias 分類（要件 3.4/5.3/6.1/7.x）。
//!
//! クレートの **公開 API のみ** を用い、実 emo2 fixture の alias 実データで解決テーブルを構築して
//! `TalkCue` 列を単一の出力契約 `CueSink::emit` で直入力する。停止同期は `close()`→`join()` のみ
//! （sleep・polling・表示を一切伴わない・要件 7.2/7.4）。`join` 復帰時点で先行 Cue は全て
//! FIFO 単一スレッドで処理済みゆえ、`MockSurfaceOutput::records()` の全 `DisplayCommand` 列を
//! 期待列と全値比較（binds 含む）できる。4 ケースはいずれも発行列 observable で判定でき、
//! cross-thread の log 捕捉を要しない（log 檻は `src/actor_dispatch_tests.rs` の同期 handler テストの領分）。

use areka_emo_compose::PatternState;
use areka_sakura::ActorKey;
use areka_seriko::DisplayCommand;

mod common;
use common::*;

/// ケース1（要件 3.4/5.3/7.4）: 冪等ガード。同一表示状態を指す発火が連続しても再発行が起きない。
///
/// 系列（すべて emo2 実データに基づく決定論的帰結・同一 scope "0"）:
/// 1. 数値 `"2100"`       → `Show{scope:"0", surface_id:2100}`（初回・Changed）
/// 2. alias `"通常"`（→[2100]）→ 同一 `Shown(2100)` へ収束＝冪等ガードで **再発行なし**（Unchanged）
/// 3. 数値 `"2100"`       → 依然 `Shown(2100)`＝再発行なし（Unchanged）
/// 4. 非表示 `"-1"`       → `Hide{scope:"0"}`（Shown→Hidden・Changed）
/// 5. 非表示 `"-1"`       → 既に `Hidden`＝再発行なし（Unchanged・非表示保持の冪等・要件 3.4）
/// 6. 数値 `"2200"`       → `Show{scope:"0", surface_id:2200}`（Hidden→別 surface・Changed）
///
/// よって発行列は「初回 Show(2100)・Hide・別 Show(2200)」の 3 件のみで、連続重複（2,3,5）は
/// 一切発行されない。単一発行点ゆえ発行列がそのまま観測点になる。
/// 同期は `close()`→`join()` の 1 点のみ。sleep/polling/表示なし（要件 7.2）。
#[test]
fn idempotent_repeat_surface_no_reemit() {
    let binds = emo2_binds();
    let records = run_cues(
        emo2_resolver(),
        binds.clone(),
        [
            emote_cue(0.0, "0", "2100"), // 初回 Show(2100)
            emote_cue(1.0, "0", "通常"), // 通常→[2100]＝同一状態・再発行なし
            emote_cue(2.0, "0", "2100"), // 同一 id 再指定・再発行なし
            emote_cue(3.0, "0", "-1"),   // Hide（Changed）
            emote_cue(4.0, "0", "-1"),   // 既に Hidden・再発行なし
            emote_cue(5.0, "0", "2200"), // 別 surface（Changed）
        ],
    );

    // 連続重複（通常/2100 再指定/-1 再指定）は Unchanged で発行されず、変化点のみ 3 件記録される。
    let expected = vec![
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
        DisplayCommand::Hide {
            scope: ActorKey::from("0"),
        },
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2200,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
    ];

    assert_eq!(
        records, expected,
        "同一表示状態を指す連続発火は再発行されず、変化点のみが発行列に現れること（冪等・要件 3.4/DD8）"
    );
}

/// ケース2（要件 6.1/7.1/7.4）: 未解決発火が混在しても後続の発火が処理され続ける。
///
/// 系列（同一 scope "0"）:
/// 1. 有効 数値 `"2100"`     → `Show{scope:"0", surface_id:2100}`
/// 2. 未解決 alias `"存在しない別名"` → 解決不能＝skip（発行なし・状態不変・要件 6.1）
/// 3. 有効 数値 `"2200"`     → `Show{scope:"0", surface_id:2200}`（後続が処理継続した observable）
///
/// 発行列は `[Show(2100), Show(2200)]` の 2 件のみ。未解決は発行を挟まず、かつ後続の有効発火が
/// パイプラインを止めずに処理された（解決失敗 skip がループを停滞させない）。error! ログ自体は
/// `src/actor_dispatch_tests.rs` の同期 handler テストで檻済み。ここは **観測可能な「継続」** を発行列で主張する。
/// 同期は `close()`→`join()` の 1 点のみ。sleep/polling/表示なし（要件 7.2）。
#[test]
fn unresolved_mixed_still_processes_following() {
    let binds = emo2_binds();
    let records = run_cues(
        emo2_resolver(),
        binds.clone(),
        [
            emote_cue(0.0, "0", "2100"),           // 有効
            emote_cue(1.0, "0", "存在しない別名"), // 未解決＝skip
            emote_cue(2.0, "0", "2200"),           // 有効（継続の証跡）
        ],
    );

    let expected = vec![
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2200,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
    ];

    assert_eq!(
        records, expected,
        "未解決発火は skip（無発行）され、後続の有効発火は処理され続けること（継続の observable・要件 6.1）"
    );
}

/// ケース3（要件 7.3/2.x）: emo2 実 alias 表を用いた解決の追験。単一候補・複数候補の分類が
/// アクター発行経路を通して期待どおりになる。
///
/// 系列（別 scope で独立に・冪等ガードの干渉を避ける）:
/// 1. alias `"通常"` @ scope "0"（emo2 `通常,[2100]`・**単一候補**）
///    → `Show{scope:"0", surface_id:2100}`
/// 2. alias `"静観"` @ scope "1"（emo2 `静観,[2106,2206]`・**複数候補→先頭固定**・DD6）
///    → `Show{scope:"1", surface_id:2106}`
///
/// 単一候補 alias は唯一の id を、複数候補 alias は先頭 id（決定論・ランダム選択しない）を
/// 表示することを、実データ・アクター emit 経路で追験する。
/// 同期は `close()`→`join()` の 1 点のみ。sleep/polling/表示なし（要件 7.2）。
#[test]
fn emo2_alias_single_and_multi_classification() {
    let binds = emo2_binds();
    let records = run_cues(
        emo2_resolver(),
        binds.clone(),
        [
            emote_cue(0.0, "0", "通常"), // 単一候補 [2100]→2100
            emote_cue(1.0, "1", "静観"), // 複数候補 [2106,2206]→先頭 2106
        ],
    );

    let expected = vec![
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
        DisplayCommand::Show {
            scope: ActorKey::from("1"),
            surface_id: 2106,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
    ];

    assert_eq!(
        records, expected,
        "単一候補 alias は唯一 id・複数候補 alias は先頭 id（決定論）を発行経路で分類すること（要件 7.3/DD6）"
    );
}

/// ケース4（要件 7.1/7.2/7.3）: 数値・alias（複数 id→先頭固定）・非表示・別スコープを
/// 混在させた fixture 系列を入力し、mock 発行列が期待列に **順序込みで完全一致** する。
///
/// 系列（すべて emo2 実データに基づく決定論的帰結）:
/// 1. 数値 `"2100"` @ scope "0" → `Show{scope:"0", surface_id:2100, binds:<static>}`
/// 2. alias `"静観"` @ scope "0" → `Show{scope:"0", surface_id:2106, binds:<static>}`
///    （emo2 `静観,[2106,2206]` の先頭固定・DD6・決定論）
/// 3. 非表示 `"-1"` @ scope "0" → `Hide{scope:"0"}`
/// 4. 数値 `"10"` @ scope "1" → `Show{scope:"1", surface_id:10, binds:<static>}`
///    （別スコープが発行列上で独立して現れることを示す・要件 3.1/3.2）
///
/// 同期は `close()`→`join()` の 1 点のみ。sleep/polling/表示なし（要件 7.2）。
#[test]
fn cue_sequence_emits_expected() {
    let binds = emo2_binds();
    let records = run_cues(
        emo2_resolver(),
        binds.clone(),
        [
            emote_cue(0.0, "0", "2100"), // 数値
            emote_cue(1.0, "0", "静観"), // alias（複数 id→先頭）
            emote_cue(2.0, "0", "-1"),   // 非表示
            emote_cue(3.0, "1", "10"),   // 別スコープ・数値
        ],
    );

    // join 復帰時点で全 Cue は FIFO 単一スレッドで処理済み。発行列を全値比較（binds 含む）。
    let expected = vec![
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2106,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
        DisplayCommand::Hide {
            scope: ActorKey::from("0"),
        },
        DisplayCommand::Show {
            scope: ActorKey::from("1"),
            surface_id: 10,
            binds: binds.clone(),
            pattern: PatternState::default(),
        },
    ];

    assert_eq!(
        records, expected,
        "数値・alias・非表示・別スコープ混在の発行列が期待列と順序込みで完全一致すること"
    );
}

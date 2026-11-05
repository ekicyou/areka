//! 統合テスト（アクター・発行列観測）— fixture 系列の決定論シナリオ（要件 7.1/7.2/7.3/7.4）。
//!
//! クレートの **公開 API のみ** を用いて（再輸出が外部から使えることも同時に検証）、
//! 実 emo2 fixture の alias 実データで解決テーブルを構築し、数値・alias・非表示・別スコープを
//! 混在させた `TalkCue` 列を単一の出力契約 `CueSink::emit` で直入力する。停止同期は `close()`→`join()` のみ
//! （sleep・polling・表示を一切伴わない・要件 7.2）。`join` 復帰時点で先行 Cue は全て FIFO
//! 単一スレッドで処理済みゆえ、`MockSurfaceOutput::records()` の全 `DisplayCommand` 列を
//! 期待列と全値比較（binds 含む）できる。

use areka_emo_compose::PatternState;
use areka_sakura::ActorKey;
use areka_seriko::DisplayCommand;

mod common;
use common::*;

/// 主 observable（要件 7.1/7.2/7.3）: 数値・alias（複数 id→先頭固定）・非表示・別スコープを
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

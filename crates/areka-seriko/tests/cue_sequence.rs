//! 統合テスト（アクター・発行列観測）— fixture 系列の決定論シナリオ（要件 7.1/7.2/7.3/7.4）。
//!
//! クレートの **公開 API のみ** を用いて（task 3.2 の re-export が外部から使えることも同時に検証）、
//! 実 emo2 fixture の alias 実データで解決テーブルを構築し、数値・alias・非表示・別スコープを
//! 混在させた `TalkCue` 列を単一の出力契約 `CueSink::emit` で直入力する。停止同期は `close()`→`join()` のみ
//! （sleep・polling・表示を一切伴わない・要件 7.2）。`join` 復帰時点で先行 Cue は全て FIFO
//! 単一スレッドで処理済みゆえ、`MockSurfaceOutput::records()` の全 `DisplayCommand` 列を
//! 期待列と全値比較（binds 含む）できる。

use areka_emo_compose::{EmoWorld, PatternState};
use areka_sakura::{ActorKey, CueCommand, CueSink, TalkCue};
use areka_seriko::{
    build_static_bindset, spawn_seriko, BindResolver, DisplayCommand, MockSurfaceOutput,
    SurfaceResolver,
};

/// テスト用の Shell 系 `TalkCue`（`Emote{key}`・at/actor 込み）を組む。
fn emote_cue(at: f64, scope: &str, key: &str) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::Emote { key: key.into() },
        duration: 0.0, // 表情切替は瞬時（明示的 0）。
    }
}

/// 実 emo2 shell fixture の surfaces.txt を parse し、alias スナップショットから解決層を組む。
///
/// 統合テストの CWD はクレートルート（`crates/areka-seriko/`）ゆえ相対 `../pilot/...` が解決する。
/// emo-compose の world.rs テストと同一 fixture・同一経路（parse→`EmoWorld::build`→
/// `alias_snapshot`）で、要件 7.3 の「alias 実データ追験」を満たす。
fn emo2_resolver() -> SurfaceResolver {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
    let shell = areka_parsers::shell::parse(&content);
    let world = EmoWorld::build(&shell);
    let snapshot = world.alias_snapshot();
    // 非自明性: emo2 は alias を含む（空マップの vacuous pass を防ぐ）。
    assert!(!snapshot.is_empty(), "emo2 fixture は alias を含む");
    SurfaceResolver::new(snapshot)
}

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
    let resolver = emo2_resolver();
    // emo2 の bindgroup default 既定オン集合（build.rs / emo-compose 実測相当）。
    let binds = build_static_bindset(&[1100, 1207, 1302, 1500, 1800]);

    // 観測用出力先。records() ハンドルを move 前に取得しておく。
    let mock = MockSurfaceOutput::new();
    let records = mock.records();

    // アクター起動→fixture 系列を emit→Close→join で終了同期（唯一の同期点・sleep なし）。
    let (mut sink, handle) = spawn_seriko(resolver, binds.clone(), BindResolver::empty(), mock);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "2100")); // 数値
    CueSink::emit(&mut sink, emote_cue(1.0, "0", "静観")); // alias（複数 id→先頭）
    CueSink::emit(&mut sink, emote_cue(2.0, "0", "-1")); // 非表示
    CueSink::emit(&mut sink, emote_cue(3.0, "1", "10")); // 別スコープ・数値
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

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

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded, &expected,
        "数値・alias・非表示・別スコープ混在の発行列が期待列と順序込みで完全一致すること"
    );
}

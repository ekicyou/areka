//! 回帰テスト（アクター・発行列観測）— 冪等性・未解決混在の継続・emo2 alias 分類（要件 3.4/5.3/6.1/7.x）。
//!
//! クレートの **公開 API のみ** を用い、実 emo2 fixture の alias 実データで解決テーブルを構築して
//! `TalkCue` 列を単一の出力契約 `CueSink::emit` で直入力する。停止同期は `close()`→`join()` のみ
//! （sleep・polling・表示を一切伴わない・要件 7.2/7.4）。`join` 復帰時点で先行 Cue は全て
//! FIFO 単一スレッドで処理済みゆえ、`MockSurfaceOutput::records()` の全 `DisplayCommand` 列を
//! 期待列と全値比較（binds 含む）できる。3 ケースはいずれも発行列 observable で判定でき、
//! cross-thread の log 捕捉を要しない（log 檻は task 4.1 の同期 handler テストの領分）。

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

/// emo2 の bindgroup default 既定オン集合（cue_sequence.rs / emo-compose 実測相当）。
fn emo2_binds() -> areka_emo_compose::BindSet {
    build_static_bindset(&[1100, 1207, 1302, 1500, 1800])
}

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
/// 一切発行されない。単一発行点（5.3）ゆえ発行列がそのまま観測点になる。
/// 同期は `close()`→`join()` の 1 点のみ。sleep/polling/表示なし（要件 7.2）。
#[test]
fn idempotent_repeat_surface_no_reemit() {
    let resolver = emo2_resolver();
    let binds = emo2_binds();

    let mock = MockSurfaceOutput::new();
    let records = mock.records();

    let (mut sink, handle) = spawn_seriko(resolver, binds.clone(), BindResolver::empty(), mock);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "2100")); // 初回 Show(2100)
    CueSink::emit(&mut sink, emote_cue(1.0, "0", "通常")); // 通常→[2100]＝同一状態・再発行なし
    CueSink::emit(&mut sink, emote_cue(2.0, "0", "2100")); // 同一 id 再指定・再発行なし
    CueSink::emit(&mut sink, emote_cue(3.0, "0", "-1")); // Hide（Changed）
    CueSink::emit(&mut sink, emote_cue(4.0, "0", "-1")); // 既に Hidden・再発行なし
    CueSink::emit(&mut sink, emote_cue(5.0, "0", "2200")); // 別 surface（Changed）
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

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

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded, &expected,
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
/// task 4.1 の同期 handler テストで檻済み。ここは **観測可能な「継続」** を発行列で主張する。
/// 同期は `close()`→`join()` の 1 点のみ。sleep/polling/表示なし（要件 7.2）。
#[test]
fn unresolved_mixed_still_processes_following() {
    let resolver = emo2_resolver();
    let binds = emo2_binds();

    let mock = MockSurfaceOutput::new();
    let records = mock.records();

    let (mut sink, handle) = spawn_seriko(resolver, binds.clone(), BindResolver::empty(), mock);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "2100")); // 有効
    CueSink::emit(&mut sink, emote_cue(1.0, "0", "存在しない別名")); // 未解決＝skip
    CueSink::emit(&mut sink, emote_cue(2.0, "0", "2200")); // 有効（継続の証跡）
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

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

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded, &expected,
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
    let resolver = emo2_resolver();
    let binds = emo2_binds();

    let mock = MockSurfaceOutput::new();
    let records = mock.records();

    let (mut sink, handle) = spawn_seriko(resolver, binds.clone(), BindResolver::empty(), mock);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "通常")); // 単一候補 [2100]→2100
    CueSink::emit(&mut sink, emote_cue(1.0, "1", "静観")); // 複数候補 [2106,2206]→先頭 2106
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

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

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded, &expected,
        "単一候補 alias は唯一 id・複数候補 alias は先頭 id（決定論）を発行経路で分類すること（要件 7.3/DD6）"
    );
}

# 設計検証レポート: areka-P0-app-shell（改定版・ダミー窓＋main所有run()）

- 対象: `.kiro/specs/areka-P0-app-shell/design.md`（DD5/DD7 改定後）
- 言語: ja（spec.json）
- 実施: 非対話検証（設計ディスカッションへ渡す指摘は最大3件）
- 検証コード実測: `crates/wintf/src/runtime/window_registry.rs`・`crates/wintf/src/runtime/mod.rs`・`crates/areka/src/main.rs`・`crates/wintf/src/ecs/world/mod.rs`

## 検証サマリ

改定モデル（`main` が `app.run()` を所有・replace-me シーム `open_startup_window` が検証用ダミー窓を1枚開き、その close の空遷移で `run()` が `Ok` を返して正常終了）は、wintf 実装と**実測レベルで整合**している。`WinApp::run(&self) -> Result<()>` は `MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(...))` で待機し、`reconcile_window_registry` は `removed_any && registry.is_empty()`（＝窓が在ってから最後の1枚が消える空遷移ちょうど）でのみ shutdown hook を発火する。窓ゼロでの空振りリコンサイルは発火しない（`reconcile_removes_entries_and_fires_hook_only_on_empty_transition` テストが固定）。よって「ダミー窓を必ず1枚開く」という DD7 の中核判断は正しく、旧「windowless-return」より強い実証（boot→loop→exit の実踏破）になっている。要件（R2.4/R2.5/R4.1/R4.2）との追跡も改定後の requirements.md と一貫する。実装可能性・境界・placement 再発防止のいずれも合格水準。

## 焦点別判定

- **(a) DD5/DD7 の内的整合と R2.4/R2.5/R4.1/R4.2 追跡**: 合格。§「UI ランタイムの終了規律」＋起動フロー図＋Requirements Traceability が改定モデルへ一貫改稿されており、R2.4（main がループ駆動）・R2.5（本物窓なし＋最小ダミー窓許容）・R4.1（ダミー窓→loop→close→正常終了）・R4.2（replace-me シームがダミー窓を開く）が漏れなくマップされる。requirements.md も同モデルへ改定済み（R2.4「app（main）自身が駆動」・R4.1「ダミー窓が閉じられたとき正常終了」）で齟齬なし。
- **(b) placement リジェクト（2026-07-05）回避**: 合格。ダミー窓は「既定位置・座標ロジックなし・配置/座標/DPI を一切主張しない liveness プローブ」と Invariants・Risks・Non-Goals・Boundary で反復明記。Monitor.work_area 物理／BoxStyle 論理の混在という失敗原因に骨格が構造的に触れない設計で、再発を防いでいる。smoke テストも「ダミー窓は配置を assert しない」と明記。
- **(c) main所有 run() ＋ close→clean-exit の健全性（wintf 空遷移）**: 合格（実測確認）。hook は `WinApp::new()` 内 `wire_shutdown_hook` で `WindowRegistry` に注入済み → ダミー窓 close の空遷移で `notify(usize::MAX)` → `run()` の `block_on(shutdown_future)` が完了して正常復帰。`close_to_reconcile_to_shutdown_chain_wakes_listener` テストが本番型で貫通実証済み。
- **(d) シーム署名の具体性・コンパイル可能性・下流置換契約**: 実質合格（軽微な過剰権限あり）。`fn open_startup_window(app: &mut WinApp)` は `WinApp::world()`（`&self`）経由の ECS spawn でダミー窓を作れるためコンパイル可能。下流置換契約（シーム本体を削除し本物のエンジン結線＋ゴースト窓生成へ置換・`main` の構造〈シーム→`app.run()`〉は不変）は Boundary/Revalidation Triggers/コンポーネント節で明確。ただし窓生成は `mgr.world()`（`&self`）で足りるため `&mut WinApp` は厳密には不要（下記 Issue 1）。
- **(e) 改定の残存不整合**: 重大なものなし。旧 DD5「no-op `wire_engines`」・旧 DD7「windowless-return」は Open Questions で明示的に破棄と記録され、孤立要件・旧モデル残置文言は見当たらない。

## 重大な問題（設計ディスカッション用・最大3件）

### 🟡 Issue 1: シーム署名 `&mut WinApp` は実測上 `&self` で足り、過剰権限
- **Concern**: ダミー窓生成は `mgr.world()`（`WinApp::world(&self)`）が返す `Rc<RefCell<EcsWorld>>` 上の `spawn`/`add_systems` で行われ、現 `main.rs`（125–137行）もその経路。`EcsWorld::spawn` は `&self`。よって `open_startup_window` は `&WinApp` で実装可能。`&mut WinApp` は上位集合ゆえコンパイルは通る（ブロッカーではない）が、設計の根拠文「app ハンドルを取るのはダミー窓を spawn するため」は `&mut` を要さない。
- **Impact**: 下流が本物のゴースト窓結線へ置換する際、`&mut WinApp` を前提に据えると実際には不要な排他借用制約を持ち込み、`main` 内で `app.run()`（`&self`）と並ぶ他の `&self` 利用と競合しうる余地を残す（現状 `run()` は `&self`）。
- **Suggestion**: 署名を `open_startup_window(app: &WinApp)` へ弱めるか、`&mut` を保つなら「下流が本物窓生成で `&mut` を要する見込みだから前方互換で `&mut` を先取り」という根拠を明記して意図を固定する。どちらでも可だが、根拠と署名を一致させる。
- **Traceability**: R4.2（replace-me シーム）／R2.4・R4.1（寄与）。
- **Evidence**: design.md 「`open_startup_window` … Service Interface」`fn open_startup_window(app: &mut WinApp);` と Implementation Notes。実測: `crates/wintf/src/runtime/mod.rs` `world(&self)`／`run(&self)`、`crates/wintf/src/ecs/world/mod.rs:429 spawn(&self)`、`crates/areka/src/main.rs:125-137`。

### 🟡 Issue 2: 骨格 smoke（boot→loop→exit）の「プログラム的 close」手段が未特定＝実装リスク
- **Concern**: Testing Strategy の骨格 smoke は「smoke テストがダミー窓をプログラム的に close して exit 0 を assert」とするが、具体的な close 経路（別スレッドから対象 HWND へ WM_CLOSE を PostMessage するか、`cargo run` 子プロセスを一定時間後に終了させるか、World 側でダミー窓 Entity を despawn するか）が未定。`run()` は UI スレッドをブロックするため、close 契機の注入点はプロセス外／別スレッドに限られ、実装難度が非自明。
- **Impact**: 未特定のまま実装へ進むと、smoke がハングして CI を止める、あるいは exit code を正しく観測できず「証明」にならない恐れ（本来 Issue 2 回帰＝ハング懸念のガードなのに、テスト自体がハング要因になりうる）。
- **Suggestion**: タスク化前に close 注入の1手段を確定する（例: 子プロセス起動＋境界時間後に対象窓へ WM_CLOSE を送るか、プロセスを graceful に終わらせる小さなハーネス）。単純化のため「手動検証＋境界タイムアウト付き子プロセス smoke」の2段構えに割り切る選択も明記する。
- **Traceability**: R2.4／R4.1（起動→loop→close→正常終了の証明）。
- **Evidence**: design.md Testing Strategy「骨格 smoke（boot→loop→exit の証明）」・骨格 main「Validation/Risks」。実測: `run(&self)` は `block_on` で UI スレッドブロック（`crates/wintf/src/runtime/mod.rs:272-323`）。

## 設計の強み

1. **wintf 終了規律との実測整合**: 空遷移でしか shutdown が撃たれないという wintf の性質（テストで固定）を正面から取り込み、「窓ゼロ＝ハング」を避けるために**必ずダミー窓を1枚開く**という判断へ昇華している。旧 windowless-return を破棄した改定は、実コードの `reconcile_window_registry` と `WinApp::run` の挙動に忠実で、骨格単体で起動→終了経路を実踏破できる強い検証性を得ている。
2. **placement 再発の構造的封じ込め**: ダミー窓を「配置・座標・DPI を一切主張しない liveness プローブ」に厳格限定し、2026-07-05 の window-placement リジェクト原因（物理／論理座標混在）へ骨格が触れない設計を Invariants/Boundary/Testing に一貫して刻んでいる。責務分離（骨格＝器／window-placement＝本物窓）が明快。

## 最終判定

- **判定: GO**
- **根拠**: 改定 DD5/DD7 モデルは wintf 実装（空遷移 shutdown・`run(&self)` ブロッキング・new() での hook 結線）と実測整合し、R2.4/R2.5/R4.1/R4.2 へ漏れなく追跡され、placement 再発も構造的に防止されている。残る2件（シーム署名の過剰権限、smoke の close 手段未特定）は設計内で解消可能な調整事項でありブロッカーではない。
- **次ステップ**: Issue 1（署名 `&WinApp` へ弱めるか `&mut` 根拠明記）と Issue 2（smoke の close 注入手段の確定）を設計ディスカッションで裁定 → `/kiro-spec-tasks areka-P0-app-shell` でタスク生成。

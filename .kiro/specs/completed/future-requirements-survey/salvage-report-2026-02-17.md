# 将来要件サルベージレポート 2026-02-17

**Feature ID**: `future-requirements-survey`  
**前回調査日**: 2025-11-25  
**本レポート作成日**: 2026-02-17  
**目的**: 旧サーベイ（2025-11-25）以降の大規模な進捗を反映し、残存要件の鮮度・妥当性を再評価する

---

## 調査方法

1. 旧 `requirements.md`（2025-11-25版）の10カテゴリを引き継ぎ
2. 現在のコードベースと完了仕様を突合
3. アクティブ仕様20件の内容を精査
4. 各要件の陳腐化度を評価（🟢有効 / 🟡要更新 / 🔴陳腐化 / ⬛消滅）

### 除外事項
- `wintf-dcomp-*` 系仕様（4件）: 最優先実施中のため本レポートの対象外
- 本レポートは **DComp移行完了後に何をすべきか** を検討する

---

## プロジェクト進捗サマリー

### 前回調査→今回の間に完了した仕様: 37件

| カテゴリ                       | 完了仕様                                                                                                                                                                                                                                                       | 完了日                 |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| **DComp移行**                  | migration-0-visual-opacity-dataflow, migration-1-d2d1-composition                                                                                                                                                                                              | 2026-02-16〜17         |
| **Dolaアニメーションエンジン** | dola-animation-system, dola-compiled-transition, dola-runtime-1〜5, dola-runtime-engine                                                                                                                                                                        | 2026-02-13〜16         |
| **イベントシステム**           | wintf-P0-event-system(親), event-hit-test, event-mouse-basic, event-dispatch, event-drag-system, event-hit-test-alpha-mask, event-hit-test-cache, event-hit-test-named-regions, event-parent-to-child-routing, pointer-event-fix, multiwindow-event-validation | 2025-12-01〜2026-02-14 |
| **ウィジット・描画**           | wintf-P0-image-widget, wintf-P0-typewriter, wintf-P0-click-through, brush-component-separation                                                                                                                                                                 | 2025-11-29〜2026-02-15 |
| **基盤最適化**                 | visual-tree-synchronization, surface-allocation-optimization                                                                                                                                                                                                   | 2025-11-26〜28         |
| **修正系**                     | wintf-fix1〜4 (座標系修正4件), wintf-taffy-child-order-fix                                                                                                                                                                                                     | 2026-02-11〜16         |
| **その他**                     | wintf-P0-logging-system, wndproc-message-handler-refactor, areka-mock-shell, areka-P0-script-engine, kiro-P0-roadmap-management                                                                                                                                | 各種                   |

### 現在のアクティブ仕様: 20件

| カテゴリ              | 件数 | 仕様                                                                                                            |
| --------------------- | ---- | --------------------------------------------------------------------------------------------------------------- |
| **DComp移行（除外）** | 5件  | wintf-dcomp-to-layered-migration(親・完了), migration-2〜4                                                      |
| **ukagaka メタ**      | 1件  | ukagaka-desktop-mascot(完了)                                                                                    |
| **ukagaka 子仕様**    | 8件  | areka-P0-mcp-server, package-manager, persistence, reference-balloon/ghost/shell, system-tray, window-placement |
| **wintf 基盤**        | 2件  | wintf-P0-animation-system, wintf-P0-balloon-system                                                              |
| **Shape系（旧形式）** | 3件  | shape-brush-system, shape-path-geometry, shape-stroke-widgets                                                   |
| **旧サーベイ**        | 1件  | future-requirements-survey（本ドキュメント）                                                                    |

### アーカイブ済み仕様: 63件以上（completedディレクトリ）

---

## 旧サーベイ10カテゴリの現在地

### ~~項目1: Visual階層の同期・DirectComposition階層的合成~~ ⬛ 方針転換

| 旧サーベイ                                  | 現在                              |
| ------------------------------------------- | --------------------------------- |
| 最優先: Direct Composition Visual階層の活用 | **DComp自体を除去する方針に転換** |
| `visual-tree-synchronization` 初期化済み    | **completed**（2025-11-26）       |

**評価**: DComp→ULW移行により、DirectComposition Visual階層の活用という方針自体が**消滅**した。`visual-tree-synchronization`は完了したものの、その成果はDComp除去で不要になる。D2D1+ULWパイプラインでは単一サーフェスへの描画が基本となり、Visual階層による部分更新最適化は別のアプローチ（Dirty Region等）が必要。

**サルベージ**: 部分更新最適化のニーズ自体は残存するが、技術手段が根本的に変わった。DComp移行完了後に新たな最適化戦略を検討すべき。

---

### ~~項目2: taffyレイアウトエンジン統合~~ ✅ 完了（旧サーベイ時点で完了済み）

変更なし。Gridレイアウト等の将来拡張も低優先度のまま。

---

### ~~項目3: Surface生成の最適化~~ ✅ 完了

| 旧サーベイ                                   | 現在                        |
| -------------------------------------------- | --------------------------- |
| `surface-allocation-optimization` 初期化済み | **completed**（2025-11-28） |

**評価**: 旧サーベイの3日後に完了。ただし、DComp→ULW移行により描画パイプラインが変わるため、最適化の一部は再検討が必要になる可能性がある。

---

### ~~項目5: Render Dirty Tracking の高度化~~ ⬛ 消滅

| 旧サーベイ                         | 現在                                          |
| ---------------------------------- | --------------------------------------------- |
| `render-dirty-tracking` アクティブ | **仕様が消失**（completedにもactiveにも不在） |

**評価**: 仕様自体が削除された模様。基本的な `surface-render-optimization` は完了済み。DComp→ULW移行後は描画パイプラインが変わるため、Dirty Tracking戦略も再設計が必要。

**サルベージ**: DComp移行完了後、新パイプラインでの描画最適化要件として再定義する価値あり。

---

### 項目6: Shape関連機能 — 🟢 概ね有効（停滞中）

| サブ仕様               | フェーズ                     | 陳腐化 | 備考                                                        |
| ---------------------- | ---------------------------- | ------ | ----------------------------------------------------------- |
| `shape-brush-system`   | Phase 0（SPEC.md/STATUS.md） | 🟡 軽度 | D2D1ベースなのでDComp移行の影響なし。親戦略リンクだけ古い   |
| `shape-path-geometry`  | Phase 0（SPEC.md/STATUS.md） | 🟢 有効 | `ID2D1PathGeometry`は引き続き使用可。nom パーサー構想も妥当 |
| `shape-stroke-widgets` | Phase 0（SPEC.md/STATUS.md） | 🟢 有効 | D2D1描画プリミティブなので影響なし                          |

**コードベース現状**:
- `Brush` enum: `Inherit` / `Solid` の2バリアントのみ（GradientBrush未実装）
- Shapes: `Rectangle` のみ（Ellipse/Polygon/Polyline未実装）
- PathGeometry: 未実装

**評価**: 3つのSPEC.md自体の技術的内容は有効だが、**旧形式（spec.jsonなし）** で放置されている。正式なkiro仕様として再初期化する際に、新パイプライン前提で微調整すれば再利用可能。

**サルベージ**: コンセプトは全てサルベージ可能。`DUAL_ROUTE_STRATEGY.md`はアーカイブ推奨。

---

### 項目7: 透過ウィンドウ・ヒットテスト — ⬛ DComp移行に包含

| 旧サーベイ               | 現在                                          |
| ------------------------ | --------------------------------------------- |
| 仕様なし、要件定義未実施 | **DComp→ULW移行そのものが透過ウィンドウ実装** |

**評価**: ULW（UpdateLayeredWindow）+ `WS_EX_LAYERED` への移行は、まさに透過ウィンドウの実装そのもの。`wintf-P0-click-through` も完了済み。ヒットテスト（`WM_NCHITTEST`ハンドリング）も `event-hit-test*` 系で実装済み。

**サルベージ**: 不要。DComp移行完了時点でこの要件は自動的に達成される。

---

### 項目8: Transform階層伝播の廃止 — 🟡 依然として必要

| 旧サーベイ                                              | 現在                   |
| ------------------------------------------------------- | ---------------------- |
| `GlobalTransform` / `TransformTreeChanged` の削除が必要 | **コードベースに残存** |

**コードベース確認結果**:
- `GlobalTransform(pub Matrix3x2)` → `components.rs` L160 に残存
- `TransformTreeChanged` マーカー → `components.rs` L190 に残存  
- `tree_system.rs` で参照・使用中

**評価**: DComp移行後、Visual階層が消えてTransformの伝播先が変わるため、クリーンアップの好機。ただし、Transformの視覚効果用途（回転・傾斜・スケール）はD2D1描画でも必要なので、削除ではなく**役割の再整理**が適切。

**陳腐化**: 🟡 方向性は正しいが、DComp移行完了後のTransformの役割を再定義してからリファクタリングすべき。

---

### 項目9: Container Widget — 🟡 低優先度のまま

| 旧サーベイ                 | 現在                                               |
| -------------------------- | -------------------------------------------------- |
| 専用Container Widget未実装 | `FlexContainer` コンポーネントがレイアウト用に存在 |

**評価**: `FlexContainer` はレイアウト制御用のコンポーネントであり、描画可能な「Container ウィジェット」（背景色・境界線・パディング等のスタイリングを持つ）は未実装。Rectangle/Labelの組み合わせで代替可能な状況は変わらない。

**陳腐化**: 🟢 要件自体は有効。ただし、ukagaka用途ではシェル画像ベースのUIが中心のため、汎用Container Widgetの優先度は低い。

---

### 項目10: その他の将来拡張 — 大幅に進展

| サブ項目                    | 旧サーベイ | 現在                                                       |
| --------------------------- | ---------- | ---------------------------------------------------------- |
| 10.1 デバイスロスト対応     | 未着手     | 未着手（DComp移行後に再検討）                              |
| 10.2 アニメーションシステム | 未着手     | ✅ **Dolaエンジンが本格実装済み**（runtime含む12+ファイル） |
| 10.3 イベント処理システム   | 未着手     | ✅ **完全実装済み**（pointer/drag/hit-test/dispatch）       |
| 10.4 ImageBrush / 画像表示  | 未着手     | ✅ **BitmapSource実装済み**（WIC統合、alpha mask含む）      |
| 10.5 テキスト編集機能       | 未着手     | 未着手（balloon-systemの要件に含まれる）                   |
| 10.6 リッチテキスト         | 未着手     | 未着手                                                     |

---

## 陳腐化評価: アクティブ仕様の鮮度チェック

### 🔴🔴 致命的陳腐化: wintf-P0-animation-system

**最終更新**: 2025-12-03 (v1.2)  
**問題**: DComp API への依存が要件全体に浸透

| 陳腐化箇所       | 内容                                                                                |
| ---------------- | ----------------------------------------------------------------------------------- |
| Req 2, AC 6      | 「DirectComposition の不透明度プロパティを使用してトランジション」→ DComp除去で不可 |
| Req 5, AC 2      | 「DirectComposition のプロパティをアニメーション」→ DComp除去で不可                 |
| Req 5, AC 6      | 「IDCompositionAnimation を使用」→ DComp除去で不可                                  |
| Req 6, AC 4      | 「IDCompositionAnimation 等のライフタイム管理」→ 対象消滅                           |
| NFR-1, 項目3-4   | Windows Animation API + DComp暗黙的アニメーション → 前提崩壊                        |
| NFR-2, 項目2     | 「DirectComposition 対応環境を前提」→ 前提条件消滅                                  |
| 技術コンテキスト | `AnimationGraphics (IDCompositionAnimation保持)` → 対象消滅                         |
| 依存関係         | `AnimationCore` 基盤前提 → **Dolaエンジン**に置換すべき                             |

**判定**: **全面書き直しが必要**。パッチ修正では対応不可。Dolaエンジン + D2D1+ULWパイプライン前提で再設計すべき。

---

### 🟢 概ね有効: wintf-P0-balloon-system

**最終更新**: 2025-11-29 (v1.0)  
**DComp直接参照**: なし  
**依存する `wintf-P0-typewriter`**: completed ✅

ウィンドウ配置方式のみULW前提で微調整が必要な可能性があるが、要件の核心部分（テキスト表示、選択肢UI、入力ボックス等）は有効。

---

### 🟢 有効: areka-P0-* 系（ukagaka子仕様8件）

**最終更新**: 2025-11-29〜2025-12-10  
**DComp依存**: なし（アプリケーション層の仕様）

mcp-server, package-manager, persistence, reference-balloon/ghost/shell, system-tray, window-placement はいずれもwintf基盤層より上位のアプリケーション層仕様であり、描画パイプラインの変更による直接的な影響はない。

---

### 🔴 陳腐化: DUAL_ROUTE_STRATEGY.md

**最終更新**: 2025-11-15  
- ルートA（テキスト系）: 目標達成済み
- ルートB（Shape系）: 未実行のまま停滞
- Phase 3（透過ウィンドウ）: DComp移行specに完全置換

**推奨**: アーカイブ（completedディレクトリへ移動）

---

## DComp移行完了後の優先度マトリクス

### Tier 1: 基盤再整備（DComp移行直後に実施すべき）

| #   | 要件                                 | 仕様状態       | 推奨アクション                                                                   |
| --- | ------------------------------------ | -------------- | -------------------------------------------------------------------------------- |
| 1   | **wintf-P0-animation-system 再設計** | 🔴 致命的陳腐化 | Dola + D2D1+ULW前提で requirements.md を**全面書き直し**                         |
| 2   | **Transform階層伝播の再整理**        | 仕様なし       | リファクタリング仕様を新規作成。GlobalTransform/TransformTreeChangedの役割再定義 |
| 3   | **描画最適化戦略の再検討**           | 仕様なし       | ULWパイプラインでのDirty Region/部分更新戦略を新規策定                           |

### Tier 2: 機能拡充（ukagaka MVP に向けて）

| #   | 要件                          | 仕様状態 | 推奨アクション                                 |
| --- | ----------------------------- | -------- | ---------------------------------------------- |
| 4   | **wintf-P0-balloon-system**   | 🟢 有効   | requirements承認 → design → tasks → 実装へ進行 |
| 5   | **areka-P0-mcp-server**       | 🟢 有効   | 同上                                           |
| 6   | **areka-P0-persistence**      | 🟢 有効   | 同上                                           |
| 7   | **areka-P0-system-tray**      | 🟢 有効   | 同上                                           |
| 8   | **areka-P0-window-placement** | 🟢 有効   | 同上                                           |

### Tier 3: コンテンツ整備（参照実装）

| #   | 要件                           | 仕様状態 | 推奨アクション              |
| --- | ------------------------------ | -------- | --------------------------- |
| 9   | **areka-P0-reference-shell**   | 🟢 有効   | requirements精査 → 順次進行 |
| 10  | **areka-P0-reference-ghost**   | 🟢 有効   | 同上                        |
| 11  | **areka-P0-reference-balloon** | 🟢 有効   | 同上                        |

### Tier 4: 描画拡張（並行実施可能）

| #   | 要件                     | 仕様状態 | 推奨アクション                                        |
| --- | ------------------------ | -------- | ----------------------------------------------------- |
| 12  | **shape-brush-system**   | 🟡 旧形式 | kiro仕様として再初期化（SPEC.md内容はサルベージ可能） |
| 13  | **shape-path-geometry**  | 🟢 旧形式 | 同上                                                  |
| 14  | **shape-stroke-widgets** | 🟢 旧形式 | 同上                                                  |

### Tier 5: 長期拡張（必要に応じて）

| #   | 要件                   | 備考                               |
| --- | ---------------------- | ---------------------------------- |
| 15  | デバイスロスト対応     | ULWパイプラインでの復旧戦略を検討  |
| 16  | Container Widget       | ukagaka用途では低優先              |
| 17  | リッチテキスト         | balloon-systemの拡張として将来検討 |
| 18  | Gridレイアウト詳細検証 | taffy基本統合は完了済み            |

---

## 消滅・統合された要件

| 旧カテゴリ                   | 状態               | 理由                                     |
| ---------------------------- | ------------------ | ---------------------------------------- |
| Visual階層同期（DComp）      | ⬛ 消滅             | DComp自体が除去対象に                    |
| 透過ウィンドウ・ヒットテスト | ⬛ DComp移行に包含  | ULW移行 + click-through完了で達成        |
| Render Dirty Tracking        | ⬛ 仕様消失         | 基本最適化は完了。新パイプラインで再検討 |
| Surface生成最適化            | ✅ 完了             | 2025-11-28                               |
| アニメーション（概念）       | ✅ Dolaで実装       | ただしwintf統合層は要再設計              |
| イベント処理                 | ✅ 完全実装         | pointer/drag/hit-test/dispatch           |
| 画像表示                     | ✅ BitmapSource実装 | WIC統合、alpha mask含む                  |

---

## アーカイブ推奨ファイル

| ファイル                                                 | 理由                                                                        |
| -------------------------------------------------------- | --------------------------------------------------------------------------- |
| `.kiro/specs/DUAL_ROUTE_STRATEGY.md`                     | 戦略として完全にstale。ルートA達成、ルートB未実行、Phase 3はDComp移行に置換 |
| `.kiro/specs/future-requirements-survey/requirements.md` | 本サルベージレポートで内容を引き継いだため、歴史的文書として保管のみ        |

---

## 次のアクション（DComp移行完了後）

### 最優先: アニメーションシステム再設計
```bash
# 旧requirements.mdを破棄し、Dola + ULWパイプライン前提で再作成
/kiro-spec-requirements wintf-P0-animation-system
# ※ 生成時に「DComp APIは除去済み、Dolaエンジンを使用」を明示的に指示
```

### 高優先: Transform リファクタリング
```bash
/kiro-spec-init "Transform系リファクタリング: GlobalTransform/TransformTreeChangedの削除・役割再定義。DComp除去後のTransformはD2D1描画時の視覚効果（回転・傾斜・スケール）のみに限定"
```

### 中優先: バルーンシステム
```bash
/kiro-spec-design wintf-P0-balloon-system -y
```

### 整理: DUAL_ROUTE_STRATEGY アーカイブ
```bash
# completedディレクトリに移動（歴史的文書として保管）
```

---

## まとめ

### 定量評価

| 指標               | 旧サーベイ (2025-11-25) | 本レポート (2026-02-17) | 差分           |
| ------------------ | ----------------------- | ----------------------- | -------------- |
| アーカイブ済み仕様 | 26件                    | 63件以上                | **+37件**      |
| アクティブ仕様     | 6件                     | 20件                    | +14件          |
| 未着手重要要件     | 8カテゴリ               | 3カテゴリ（実質）       | **-5カテゴリ** |
| 致命的陳腐化仕様   | 0件                     | 1件 (animation-system)  | +1件           |
| 消滅した要件       | 0件                     | 3件                     | +3件           |

### 定性評価

旧サーベイから約3ヶ月で、プロジェクトは**根本的なアーキテクチャ転換**を経験した:

1. **DComp→ULW移行決定**: DirectCompositionからUpdateLayeredWindowへの全面移行。旧サーベイが最優先としていた「DComp Visual階層活用」は方針転換で消滅
2. **Dolaエンジン誕生**: プラットフォーム非依存のアニメーションエンジンが新規実装され、Windows Animation API依存が不要に
3. **イベントシステム完成**: hit-test、pointer、drag、dispatchの全レイヤーが実装完了
4. **ukagaka仕様体系の確立**: 32子仕様に分解されたメタ仕様が完成し、アプリケーション層の要件が明確化

これにより、旧サーベイの10カテゴリのうち **5カテゴリが完了/消滅** し、残りの中で **animation-system だけが致命的な陳腐化** を起こしている。DComp移行完了後の最優先事項は、animation-system のDola前提での再設計である。

---

**作成日**: 2026-02-17  
**前回サーベイ**: 2025-11-25

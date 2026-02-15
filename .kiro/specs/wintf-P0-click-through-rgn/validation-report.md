# Design Validation Report: wintf-P0-click-through-rgn

**実施日**: 2026-02-15  
**対象フェーズ**: Design  
**レビュアー**: AI System  
**レビュー基準**: `.kiro/settings/rules/design-review.md`

---

## Review Summary

SetWindowRgn ベースのクリックスルー機能の設計ドキュメントを網羅的にレビューした結果、**実験的仕様としての性質を考慮した上で、条件付きで実装フェーズへの移行を承認**します。

設計全体は既存 ECS アーキテクチャとの整合性が高く、HRGN 所有権管理の堅牢性、リジェクション容易性の徹底など、技術的に優れた点が多く見られます。一方で、DirectComposition 互換性検証の実装優先順位が明示されていない点、将来の AlphaMask 拡張への設計指針が不足している点、エラーリカバリ戦略の詳細化が必要な点の3つを Critical Issues として特定しました。

**これらの課題は Tasks 生成時に対処可能であり、実験的アプローチとしての性質上、早期検証と迅速なリジェクションを可能にする設計方針は適切**です。

---

## Critical Issues (≤3)

### Issue 1: DirectComposition 互換性検証の実装優先度が不明瞭

**カテゴリ**: Architecture Alignment  
**深刻度**: Critical

#### 詳細
- **問題**: Requirements 6.1-6.4 で DirectComposition 互換性検証が定義され、Testing Strategy で「DirectComposition 互換性テスト（**最優先**）」と記載されているが、実装フェーズでこのテストが最初に実行されることを保証するタスク配置戦略が design.md に含まれていない
- **根拠**: research.md で「SetWindowRgn + WS_EX_NOREDIRECTIONBITMAP 互換性は公式ドキュメントで明示的に言及されておらず、実験検証が必須」「失敗時はモジュール全体を削除」と明記されているにもかかわらず、Testing Strategy セクションではテストの内容が記載されているのみで、実装前検証の必要性が強調されていない

#### 影響範囲
- **テスト実装後に互換性が失敗した場合**: 既に実装されたコードの破棄が必要となり、開発コストが増大
- **アプローチ全体の破棄判断**: 互換性失敗時は DirectComposition 利用破棄または本アプローチ全体の破棄が必要（Requirements 6.3-6.4）だが、早期発見できなければ手戻りが大きい

#### 推奨対応
Tasks 生成時に以下を実施:
1. **Task 1 として「DirectComposition 互換性検証プロトタイプ」を配置**
   - 最小限のコード（SetWindowRgn 呼び出し + Visual 描画確認）で互換性を検証
   - 失敗時は即座にアプローチ全体を破棄し、設計フェーズに戻る
2. **Task 完了条件に「GO/NO-GO 判定」を明示**
   - GO: すべての互換性テストが成功 → 本実装に進む
   - NO-GO: いずれかの互換性テストが失敗 → 設計破棄・代替案検討

#### Traceability
- **Requirements**: 6.1, 6.2, 6.3, 6.4
- **Design Sections**: "Testing Strategy > Integration Tests" (L282-285), "Risks & Mitigations" (research.md L189-194)

---

### Issue 2: HitTestMode::AlphaMask への将来拡張パスが設計レベルで未定義

**カテゴリ**: Extensibility  
**深刻度**: Medium-High

#### 詳細
- **問題**: requirements.md の「技術要件」セクションで「継続可能性: 将来的に HitTestMode::AlphaMask のピクセル単位クリックスルーが必要になった場合は、ビットマップ中間表現方式への拡張を許容」と明記されているが、design.md にはビットマップ拡張時の移行設計が含まれていない
- **根拠**: 現在の `build_click_through_region(world: &World, window_entity: Entity) -> Result<OwnedRegion>` API が、将来のビットマップベース構築に対応できるか検証されていない。ビットマップ方式では以下が必要になる可能性がある:
  - 中間バッファの管理（HBITMAP → HRGN 変換）
  - 複数エンティティの alpha channel 合成
  - グリッドスナップの無効化
  
  上記の変更が現在の API シグネチャで吸収できるか、破壊的変更が必要になるかが不明

#### 影響範囲
- **拡張時の破壊的変更リスク**: ビットマップ拡張時に `build_click_through_region` の内部実装だけでなく、API シグネチャ自体の変更が必要になる可能性がある（例: 中間バッファの事前割り当て、非同期処理化）
- **実装コストの予測不可**: 矩形ベース→ビットマップベースの移行に必要な工数・API 変更範囲が design 時点で見積もれない

#### 推奨対応
design.md に以下のセクションを追加:
1. **"Future Extensions" セクション**
   - HitTestMode::AlphaMask 対応時の API 拡張戦略を明記
   - オプション A: `build_click_through_region` 内部で HitTestMode を判定し、Bounds なら矩形、AlphaMask ならビットマップ経由で構築（API 変更なし）
   - オプション B: `build_click_through_region_bitmap(world, window_entity, bitmap_buffer)` を追加し、handle_region_timer で切り替え（API 追加）
2. **中間バッファ管理の方針**
   - ビットマップサイズの上限（ウィンドウサイズ依存）
   - HBITMAP → HRGN 変換の参照実装（既存ライブラリ or 自前実装）

**または、実験的仕様として「AlphaMask 対応は当面スコープ外」と明示し、拡張時は設計全体の見直しを許容することを明記**

#### Traceability
- **Requirements**: "技術要件 > 継続可能性" (requirements.md L12-14)
- **Design Sections**: "Overview > Non-Goals" (design.md L19-21 の暗黙的言及のみ)

---

### Issue 3: エラーリカバリ戦略の不明瞭さ（GDI API 恒久的失敗シナリオ）

**カテゴリ**: Design Consistency  
**深刻度**: Medium

#### 詳細
- **問題**: Error Handling セクションで「GDI API エラー: warn! レベルでログ出力し、リージョン更新をスキップ（次回タイマーでリトライ）」と記載されているが、以下のシナリオが考慮されていない:
  1. **SetTimer 失敗時の影響**: 「warn! + クリックスルー無効状態で続行」と記載されているが、クリックスルー機能が完全に無効化されることのユーザー影響（デスクトップアイコンがクリックできないまま放置）が評価されていない
  2. **恒久的な GDI エラーの無限ループ**: CreateRectRgn / CombineRgn がリソース枯渇で恒久的に失敗する場合、0.25秒毎にリトライが発生し続ける（ログスパム + CPU 負荷）

#### 影響範囲
- **サイレントな機能不全**: SetTimer 失敗時にクリックスルーが無効化されることが警告ログのみで伝えられ、ユーザーにはエラーが見えない
- **パフォーマンス劣化**: GDI リ��ース枯渇時の連続リトライによるログフラッド、CPU サイクルの浪費

#### 推奨対応
Error Handling セクションに以下を追加:
1. **恒久的エラーの検出と無効化**
   - GDI API エラーが連続 N 回（例: 10回）発生した場合、自動的に `set_region_updates_enabled(false)` を呼び出し、リージョン更新を恒久的に無効化
   - `error!` レベルでログに「Click-through region updates permanently disabled due to repeated GDI errors」を記録
2. **SetTimer 失敗時の明示的な状態管理**
   - SetTimer 失敗時に WindowHandle に `ClickThroughDisabled` マーカーコンポーネントを追加
   - クリックスルーが無効化されたウィンドウを query で検出可能にする

#### Traceability
- **Requirements**: 8.4 (実用性判断), 9.4 (機能削除時の既存動作への非干渉)
- **Design Sections**: "Error Handling > Error Categories and Responses" (design.md L256-264)

---

## Design Strengths

### 1. HRGN 所有権管理の堅牢性
OwnedRegion RAII パターンと、SetWindowRgn 成功後の `into_raw()` + `mem::forget()` による OS 所有権移転の明示的管理が優れています。Preconditions / Postconditions / Invariants が明確に定義されており、GDI リソースリークを設計レベルで防止しています（design.md L206-211, L221-225）。

**Rust の型システムを最大限に活用した、wintf プロジェクトの Type Safety 基準に完全に準拠した設計**です。

### 2. リジェクション容易性の徹底
「実験的仕様」という性質を踏まえ、機能削除時の影響を最小化する設計が徹底されています:
- 単一モジュール `ecs/click_through_rgn.rs` への完全集約（design.md L170-171）
- 既存ファイルへの変更が最小限（3箇所のみ: ecs/mod.rs, ecs_wndproc, window.rs on_add/on_remove）
- `set_region_updates_enabled(false)` による機能の完全無効化（design.md L205）
- Requirements 9.1-9.5 で明示的にリジェクション容易性が要件化され、設計がそれに完全準拠

**パフォーマンスや互換性の問題が判明した際の迅速な撤退を可能にする、実験的開発のベストプラクティス**です。

---

## Final Assessment

### 判定: **GO（条件付き）**

#### 承認条件
Tasks 生成時に以下を実施することを条件として、実装フェーズへの移行を承認します:

1. **Task 1 として DirectComposition 互換性検証プロトタイプを配置**
   - 最小限のコード（SetWindowRgn 呼び出し + Visual 描画確認 + クロスプロセスクリック貫通テスト）で互換性を検証
   - 失敗時は即座に実装を中止し、代替案の検討または DirectComposition 利用破棄の判断を実施
   
2. **エラーリカバリ戦略の実装時詳細化**
   - GDI API 連続エラー時の自動無効化ロジック（リトライカウンタ）を実装タスクに含める
   - SetTimer 失敗時の状態管理（マーカーコンポーネントまたはログ強化）を実装タスクに含める

#### Issue 2 (AlphaMask拡張) の扱い
実験的仕様として「AlphaMask 対応は将来の大規模リファクタリング時に検討」と明示的に記録し、現時点では矩形ベース実装に専念することを許容します。research.md に既に「ビットマップ中間表現方式への拡張を許容」と記載されており、拡張時の設計見直しが前提となっています。

#### 次のステップ
`/kiro-spec-tasks wintf-P0-click-through-rgn [-y]` を実行し、上記の条件を満たすタスクリストを生成してください。Task 1 の完了判定を「互換性検証の GO/NO-GO 判定」とし、NO-GO の場合は残りのタスクを実行せず設計フェーズに戻ることを明記してください。

---

## Evidence References

### Architecture Alignment
- ✓ ECS read-only queries: design.md L190-191 (`Query<(GlobalArrangement, Option<HitTest>)>`)
- ✓ ecs_wndproc パターン踏襲: design.md L238-240 (WM_TIMER match arm 追加)
- ✓ WindowHandle on_add/on_remove フック: design.md L242-246
- ✓ DragState thread_local: design.md L129-132 (リージョン更新フロー)
- ✓ tech.md Type Safety 基準準拠: OwnedRegion RAII (design.md L186-189)

### Design Consistency
- ✓ windows::core::Result 使用: design.md L192 (既存コードと一貫)
- ✓ tracing crate 活用: design.md L199-200 (既存パターン)
- ✓ 命名規則: snake_case, SCREAMING_SNAKE_CASE (design.md L176-180)
- ⚠️ エラーリカバリ詳細化必要: design.md L256-264 (Issue 3)

### Extensibility
- ⚠️ AlphaMask 拡張設計不足: requirements.md L12-14 vs design.md (Issue 2)
- ✓ グリッドサイズ構成可能: design.md L176 (GRID_SNAP_SIZE)
- ✓ set_region_updates_enabled: design.md L205

### Type Safety
- ✓ HRGN 所有権管理: design.md L186-189, L221-225
- ✓ preconditions/postconditions 明示: design.md L215-220
- ✓ windows crate 型安全ラッパー: design.md L97 (Tech Stack)

### Requirements Coverage
- ✓ 全9要件がトレーサビリティテーブルでカバー: design.md L154-164
- ✓ 各要件の Acceptance Criteria が設計に反映: requirements.md → design.md 対応確認済み

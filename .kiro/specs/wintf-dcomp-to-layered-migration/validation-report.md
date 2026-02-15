# 実装検証レポート: wintf-dcomp-to-layered-migration

**検証日時**: 2026-02-16  
**検証対象**: 親仕様 `wintf-dcomp-to-layered-migration` の実装成果物  
**対象フェーズ**: completed（文書生成 — migration-guide.md + 4子仕様）

---

## 1. 検出対象サマリー

| 項目 | 詳細 |
|------|------|
| 親仕様フェーズ | completed |
| タスク数 | 5 / 5 完了 |
| 成果物 | migration-guide.md（530行）+ 4子仕様ディレクトリ |
| 子仕様 | Phase 1〜4（各 spec.json + requirements.md + design.md + tasks.md） |
| 親要件数 | 10要件・42受入基準 |
| 子仕様合計要件数 | 30要件（8+6+8+8） |
| 子仕様合計タスク数 | 27タスク（7+5+7+8） |

---

## 2. 検証サマリー

| 検証項目 | 結果 | 備考 |
|---------|------|------|
| タスク完了 | **PASS** | 全5タスク `[x]`、コミット・プッシュ済み |
| 要件トレーサビリティ | **PASS（条件付き）** | 全42 AC がカバー。ただし Phase 4 spec.json の parent_requirements が不完全 |
| 設計整合性 | **PASS** | Issue 3, 4 対応済み |
| spec.json スキーマ整合性 | **PASS** | Issue 1 対応済み: Phase 3-4 を feature_name スキーマに統一 |
| 子仕様間データフロー整合性 | **PASS** | Issue 2 対応済み: dirty フィールド定義追加 |
| migration-guide.md カバレッジ | **PASS** | §1〜§13 で全メタ要件をカバー |
| 子仕様内部品質 | **PASS** | 全子仕様で Req→Design→Tasks の完全マッピングを確認 |

---

## 3. 検出課題一覧

### Issue 1: spec.json スキーマ不整合（~~Critical~~ → RESOLVED）

**対応**: Phase 3-4 の spec.json を Phase 1-2 と同一スキーマに修正済み。`name` → `feature_name`、`priority`/`ready_for_implementation`/タイムスタンプフィールド追加、`description` 削除。

**検出時の状況**: Phase 1-2 は `feature_name` キー、Phase 3-4 は `name` キーを使用しており、`priority`/`ready_for_implementation`/`created_at`/`updated_at` フィールドも Phase 3-4 に欠落していた。

---

### Issue 2: WindowD3D11Compositor の `dirty` フィールド未定義（~~Warning~~ → RESOLVED）

**対応**: Phase 1 design.md の `WindowD3D11Compositor` 構造体に `dirty: bool` フィールドを追加。Service Interface に `is_dirty()` / `set_dirty()` メソッドを追加。`composite_render_system` の描画完了時に `set_dirty(true)` を呼び出す設計を追加。Phase 1→3 の dirty 状態伝達契約を明文化。

**検出時の状況**: Phase 3 の `ulw_present_system` が `compositor.dirty` を参照していたが、Phase 1 の構造体定義にフィールドが存在しなかった。

---

### Issue 3: `present_layered_window` 関数シグネチャの不一致（~~Warning~~ → RESOLVED）

**対応**: Phase 3 design.md §4.2 に「親 design.md からの設計変更」注記を追加。引数簡素化の理由（`width, height` → `size: &SIZE`、`window_pos` 削除）を明記。

**検出時の状況**: 親 design.md では `present_layered_window(hwnd, memory_dc, width, height, window_pos)` だったシグネチャを、Phase 3 では `present_layered_window(hwnd, hdc_src, size: &SIZE)` に簡素化していた。

---

### Issue 4: 親 design.md の RED システム数記述（~~Minor~~ → RESOLVED）

**対応**: 親 design.md の「12個 RED」を「9個 RED」に修正済み。

**検出時の状況**: 親 design.md の子仕様2説明で RED 分類システム数が12個と記載されていたが、実際のテーブル上は9個だった。子仕様側はすべて正しく9個を記載していた。

---

### Issue 5: Phase 4 spec.json の parent_requirements 不完全（~~Minor~~ → RESOLVED）

**対応**: Phase 4 spec.json の `parent_requirements` を `["1.1", "1.2", "2.5", "5.1", "5.3", "6.3", "6.4", "8.4", "10.1"]` に拡充。Phase 4 requirements.md の各要件の `_Parent:` アノテーションを精緻化（Req 1.2, 3.3, 5.3, 6.3, 6.4, 2.5 を追加）。トレーサビリティテーブルも拡充済み。

**検出時の状況**: Phase 4 spec.json の `parent_requirements` が4項目のみで、requirements.md の `_Parent:` アノテーションも全要件が `Req 1.1` のみを参照しており、精緻なトレーサビリティが不足していた。

---

## 4. 要件トレーサビリティマトリクス

### 4.1 親要件 → 成果物マッピング（全42 AC）

| 親要件 | AC | カバー先 | ステータス |
|--------|-----|---------|-----------|
| Req 1.1 | 3カテゴリ分類定義 | migration-guide.md §1 | **PASS** |
| Req 1.2 | DComp廃止対象ファイル識別 | migration-guide.md §1 | **PASS** |
| Req 1.3 | DComp非依存再利用資産 | migration-guide.md §1 | **PASS** |
| Req 2.1 | 4フェーズ段階的移行戦略 | migration-guide.md §5 | **PASS** |
| Req 2.2 | Phase 1完了時の同等描画 | migration-guide.md §5.2 + Phase 1 Req 8 | **PASS** |
| Req 2.3 | Phase 2完了時DComp無効化 | Phase 2 Req 5 + migration-guide.md §5.2 | **PASS** |
| Req 2.4 | Phase 3完了時ULW+クリックスルー | Phase 3 Req 7, 8 + migration-guide.md §5.2 | **PASS** |
| Req 2.5 | Phase 4完了時DComp完全除去 | Phase 4 Req 8 + migration-guide.md §5.2 | **PASS** |
| Req 3.1 | per-window合成ビットマップ | Phase 1 Req 1 (WindowD3D11Compositor) | **PASS** |
| Req 3.2 | Composition概念のD2D1継承 | Phase 1 Req 2 (composite_render_system) | **PASS** |
| Req 3.3 | DCompステージ置換 | Phase 2 Req 1 (Schedule切替) | **PASS** |
| Req 3.4 | GraphicsCommandList再利用 | Phase 1 design.md Non-Goals | **PASS** |
| Req 3.5 | リサイズ時ビットマップ再作成 | Phase 1 Req 6 (リサイズ) | **PASS** |
| Req 3.6 | Opacity階層累積 | Phase 1 Req 4 (GlobalArrangement.global_opacity) | **PASS** |
| Req 4.1 | ULW呼び出し | Phase 3 Req 1, 2 | **PASS** |
| Req 4.2 | WS_EX_LAYERED変更 | Phase 3 Req 3 | **PASS** |
| Req 4.3 | alpha=0クリックスルー | Phase 3 Req 7 | **PASS** |
| Req 4.4 | commit→ULW置換 | Phase 3 Req 1 | **PASS** |
| Req 4.5 | ULW失敗リトライ | Phase 3 Req 6 | **PASS** |
| Req 5.1 | DComp初期化除去 | Phase 2 Req 2 | **PASS** |
| Req 5.2 | デバイスチェーン維持 | Phase 2 Req 2 design | **PASS** |
| Req 5.3 | DCompフィールド除去 | Phase 2 Req 2 | **PASS** |
| Req 5.4 | デバイスロストフロー維持 | Phase 1 Req 7 + Phase 2 design | **PASS** |
| Req 6.1 | WindowGraphics置換 | Phase 1 Req 1 (WindowD3D11Compositor) | **PASS** |
| Req 6.2 | Visual概念継承 | Phase 2 Req 3 + Phase 1 design | **PASS** |
| Req 6.3 | VisualGraphics/SurfaceGraphics一新 | Phase 2 Req 3 + Phase 4 Req 2 | **PASS** |
| Req 6.4 | visual_manager置換 | Phase 4 Req 4 | **PASS** |
| Req 6.5 | 命名規則 | Phase 1 design: `WindowD3D11Compositor` | **PASS** |
| Req 7.1 | WM_PAINT/ERASEBKGND更新 | Phase 3 Req 4 | **PASS** |
| Req 7.2 | WM_SIZE合成ビットマップリサイズ | Phase 3 Req 5 | **PASS** |
| Req 7.3 | BeginPaint/EndPaint最小ペア | Phase 3 Req 4 | **PASS** |
| Req 8.1 | click-through-rgn関係定義 | migration-guide.md §10.1 | **PASS** |
| Req 8.2 | animation-system影響評価 | migration-guide.md §10.2 | **PASS** |
| Req 8.3 | balloon-system影響評価 | migration-guide.md §10.3 | **PASS** |
| Req 8.4 | dcomp_demo.rs削除 | Phase 4 Req 5 | **PASS** |
| Req 9.1 | 子仕様構成定義 | migration-guide.md §5, §12 + 4子仕様実体 | **PASS** |
| Req 9.2 | 実装指針参照 | 各子仕様 design.md Overview | **PASS** |
| Req 9.3 | 前提条件・依存記載 | 各子仕様 spec.json dependencies + requirements.md 前提 | **PASS** |
| Req 9.4 | 並行稼働期間考慮 | migration-guide.md §5.3 + Phase 1 Non-Goals | **PASS** |
| Req 10.1 | 各子仕様検証基準 | Phase 1 Req 8, Phase 2 Req 6, Phase 3 Req 8, Phase 4 Req 8 | **PASS** |
| Req 10.2 | 各フェーズ完了基準 | design.md Migration Strategy DoD + migration-guide.md §5.2 | **PASS** |
| Req 10.3 | 描画品質基準 | migration-guide.md §9.4 + design.md Testing Strategy | **PASS** |

**トレーサビリティ結果**: 42/42 AC = **100% カバー**

### 4.2 子仕様内部カバレッジ

| 子仕様 | 要件数 | タスク数 | Req→Task マッピング | ステータス |
|--------|--------|---------|---------------------|-----------|
| Phase 1 | 8 | 7 | 全8要件がタスクにマッピング | **PASS** |
| Phase 2 | 6 | 5 | 全6要件がタスクにマッピング | **PASS** |
| Phase 3 | 8 | 7 | 全8要件がタスクにマッピング | **PASS** |
| Phase 4 | 8 | 8 | 全8要件がタスクにマッピング | **PASS** |

---

## 5. 設計整合性検証

### 5.1 Phase 境界契約の整合性

| 境界 | migration-guide.md §3 | 子仕様の前提/完了条件 | ステータス |
|------|----------------------|---------------------|-----------|
| Phase 1→2 | WindowD3D11Compositor, compositor_init/render_system, GlobalArrangement.global_opacity | Phase 2 前提: Phase 1 完了 + 新システム独立テスト済み | **PASS** |
| Phase 2→3 | world.rs Schedule切替済み, DComp API呼出ゼロ, GraphicsCore DComp除去 | Phase 3 前提: Phase 2 完了, DComp API呼出ゼロ | **PASS** |
| Phase 3→4 | ULW全ウィンドウ稼働, WS_EX_LAYERED, WM_PAINT/SIZE更新済み | Phase 4 前提: Phase 3 完了, ULW方式で全描画動作 | **PASS** |

### 5.2 技術パラメータの一貫性

| パラメータ | 親 design.md | migration-guide.md | 子仕様 | ステータス |
|-----------|-------------|-------------------|--------|-----------|
| Bitmap Format | DXGI_FORMAT_B8G8R8A8_UNORM, PREMULTIPLIED | §6.1 同一 | Phase 1 同一 | **PASS** |
| BLENDFUNCTION | AC_SRC_OVER, 255, AC_SRC_ALPHA | §6.2 同一 | Phase 3 同一 | **PASS** |
| DIBSection | biHeight=-(h), BI_RGB, 32bpp | §6.3 同一 | Phase 1 同一 | **PASS** |
| z-order | depth-first pre-order（画家のアルゴリズム） | §6.6 同一 | Phase 1 同一 | **PASS** |
| Opacity累積 | parent.global_opacity × child.opacity, clamp [0,1] | §6.5 同一 | Phase 1 同一 | **PASS** |

### 5.3 Schedule Stage 変更の一貫性

| Phase | 親 design.md の記述 | 子仕様 design.md | ステータス |
|-------|-------------------|-----------------|-----------|
| Phase 1 | world.rs 変更なし, PostLayout のみ拡張 | Phase 1 Non-Goals: world.rs登録しない | **PASS** |
| Phase 2 | 9 RED 除去, 2新システム追加 | Phase 2: 9除去+2追加 | **PASS** |
| Phase 3 | CommitComposition: commit→ulw_present | Phase 3 §2.2: 同一 | **PASS** |
| Phase 4 | 残存コード物理削除 | Phase 4 §2: リーフから削除順序 | **PASS** |

---

## 6. 子仕様間整合性検証

### 6.1 Phase 間データフロー

```
Phase 1 出力                  Phase 2 入力           Phase 3 入力           Phase 4 入力
─────────────────────────────────────────────────────────────────────────────────────────
WindowD3D11Compositor     →   world.rs 登録      →   ULW読取              →  (不要)
compositor_init_system    →   GraphicsSetup登録   →   (維持)               →  (維持)
composite_render_system   →   RenderSurface登録   →   (維持)               →  (維持)
GlobalArrangement.opacity →   (維持)              →   (維持)               →  (維持)
transfer_to_hbitmap       →   (未使用)            →   ulw_present_system   →  (維持)
                                                      present_layered_window
                                                      WS_EX_LAYERED                    DComp コード削除
```

**検出結果**: データフロー自体は整合している。`composite_render_system` → `ulw_present_system` 間の dirty 状態伝達メカニズムは Issue 2 対応により Phase 1 design.md に定義済み。

### 6.2 commit_composition の扱い

Phase 2 と Phase 3 の間で `commit_composition` の扱いに曖昧さがある:

- **Phase 2 design.md**: 「Phase 3 まで温存」だが「DComp デバイスが除去済みのため実質 no-op もしくは削除。具体的な対応は実装時に判断する」
- **Phase 3 design.md §2.2**: Phase 2 完了時は「残存だが実質無効 or 空」、Phase 3 で `ulw_present_system` に置換

**判定**: **許容範囲** — 実装時判断の余地を残した設計方針であり、矛盾ではない。Phase 2 実装時に commit_composition を空の stub にするか完全除去するかの二択として理解可能。

---

## 7. GO / NO-GO 判定

### 判定: **GO（無条件）**

親仕様 `wintf-dcomp-to-layered-migration` の実装（文書生成）は、全5タスク完了・全42受入基準100%カバーを達成している。検証で検出された5件の Issue はすべて対応済み（RESOLVED）。子仕様群の approve → implementation に進む準備が整った。

| Issue | 対応 |
|-------|------|
| Issue 1: spec.json スキーマ不整合 | **RESOLVED** — Phase 3-4 を統一スキーマに修正 |
| Issue 2: dirty フィールド未定義 | **RESOLVED** — Phase 1 design.md に定義追加 |
| Issue 3: present_layered_window シグネチャ | **RESOLVED** — Phase 3 design.md に変更注記追加 |
| Issue 4: RED システム数 | **RESOLVED** — 親 design.md を9個に修正 |
| Issue 5: Phase 4 parent_requirements | **RESOLVED** — spec.json + requirements.md 精緻化 |

---

## 8. 総評

おーっほっほっほ！ まあ、ここまでの文書生成は及第点と言ってあげてもよくってよ？ 全42受入基準を100%カバーしているなんて……べ、別に褒めているわけではなくてよ！ 当然のことですわ！

ただし、spec.json のスキーマ不整合は**この悪役令嬢の目は誤魔化せませんわ**。Phase 1-2 の `feature_name` と Phase 3-4 の `name` の混在、これは明らかに別々のタイミングで生成した際の一貫性不足ですわね。子仕様実装に入る前に必ず修正なさい。

`dirty` フィールドの件も、Phase 1 と Phase 3 の設計者が「きっと相手が定義してくれるだろう」と思い込んでいる典型的なインターフェース契約の抜け漏れですわ。これは仕様レビュー段階で確実に潰しておくべきですわよ。

とはいえ、migration-guide.md の13セクション530行にわたる統合指針、4子仕様のトレーサビリティ完備、Phase境界契約の整合……この規模の段階的移行計画としては、あなた、なかなかやるじゃない。……い、いえ、わたくしの知識チートがあってこその検証結果ですわ！ 勘違いなさらないでくださいまし！

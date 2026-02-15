# 実装検証レポート: wintf-dcomp-to-layered-migration

**初回検証日時**: 2026-02-16  
**再検証日時**: 2026-02-16（第2回 — 包括的再検証）  
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
| 検証対象文書 | 22ファイル（親6 + 子4×4 + migration-guide + research） |

---

## 2. 検証サマリー

| 検証項目 | 結果 | 備考 |
|---------|------|------|
| タスク完了 | **PASS** | 全5タスク `[x]`、コミット・プッシュ済み |
| 要件トレーサビリティ | **PASS** | 全42 AC 100%カバー確認済み |
| 設計整合性 | **PASS** | 全技術パラメータ一貫性確認済み |
| spec.json スキーマ整合性 | **PASS** | 全4子仕様が `feature_name` 統一スキーマ |
| 子仕様間データフロー整合性 | **PASS** | dirty 状態伝達・Phase 境界契約すべて整合 |
| migration-guide.md カバレッジ | **PASS** | §1〜§13 で全メタ要件をカバー |
| 子仕様内部品質 | **PASS** | 全子仕様で Req→Design→Tasks の完全マッピング確認 |
| 技術パラメータ一貫性 | **PASS** | BLENDFUNCTION, DIBSection, DXGI_FORMAT, z-order, Opacity — 全文書間で一致 |
| Phase 境界契約整合性 | **PASS** | migration-guide §3 と各子仕様の前提/完了条件が一致 |
| Schedule Stage 変更整合性 | **PASS** | Phase 1→4 の段階的変更が全文書間で一致 |

---

## 3. 検出課題一覧

### 初回検証（2026-02-16）で検出・解決済み

| # | 課題 | 重要度 | ステータス |
|---|------|--------|-----------|
| 1 | spec.json スキーマ不整合（`name` vs `feature_name`） | Critical | **RESOLVED** |
| 2 | WindowD3D11Compositor の `dirty` フィールド未定義 | Warning | **RESOLVED** |
| 3 | `present_layered_window` シグネチャ不一致（5引数→3引数） | Warning | **RESOLVED** |
| 4 | 親 design.md の RED システム数記述（「12個」→「9個」） | Minor | **RESOLVED** |
| 5 | Phase 4 spec.json `parent_requirements` 不完全 | Minor | **RESOLVED** |

### 第2回再検証で検出・解決済み

| # | 課題 | 重要度 | ステータス |
|---|------|--------|-----------|
| 6 | Phase 4 spec.json `parent_requirements` に `"3.3"` 未記載 | Warning | **RESOLVED** |
| 7 | 親 design.md Phase 4 セクション「12システム関数」の曖昧さ | Minor | **RESOLVED** |

---

### Issue 6: Phase 4 spec.json に `"3.3"` 未記載（~~Warning~~ → RESOLVED）

**対応**: Phase 4 spec.json `parent_requirements` に `"3.3"` を追加。

**検出時の状況**: Phase 4 requirements.md Req 3 に `_Parent: Req 1.1, 3.3_` と記載があり、トレーサビリティテーブルにも `Req 3.3 (DCompステージ置換) | Req 3` が存在するが、spec.json の `parent_requirements` 配列に `"3.3"` が含まれていなかった。Issue 5 の修正時に追加漏れした残存ギャップ。

---

### Issue 7: 親 design.md Phase 4「12システム関数」の曖昧さ（~~Minor~~ → RESOLVED）

**対応**: 親 design.md の記述を `RED分類のシステム関数コード削除（主要9エントリポイント＋関連ヘルパー3関数、計12関数 — research.md §1.1 参照）` に明確化。

**検出時の状況**: 親 design.md Phase 4 セクションが「RED分類の12システム関数」と記述していたが、Phase 4 子仕様では「9関数」（主要エントリポイント）+「関連ヘルパー関数も合わせて削除」としていた。research.md で systems.rs に12 RED 関数（9主要+3ヘルパー）が確認されているため、どちらも数値的には正しいが、注釈なしでは実装者に混乱を招く。

---

### 過去 Issue 詳細（初回検証分 — 参考）

<details>
<summary>Issue 1〜5 の詳細（クリックで展開）</summary>

#### Issue 1: spec.json スキーマ不整合（~~Critical~~ → RESOLVED）

Phase 3-4 の spec.json を Phase 1-2 と同一スキーマに修正済み。`name` → `feature_name`、`priority`/`ready_for_implementation`/タイムスタンプフィールド追加、`description` 削除。

#### Issue 2: WindowD3D11Compositor の `dirty` フィールド未定義（~~Warning~~ → RESOLVED）

Phase 1 design.md の `WindowD3D11Compositor` 構造体に `dirty: bool` フィールドを追加。Phase 1→3 の dirty 状態伝達契約を明文化。

#### Issue 3: `present_layered_window` 関数シグネチャの不一致（~~Warning~~ → RESOLVED）

Phase 3 design.md §4.2 に「親 design.md からの設計変更」注記を追加。引数簡素化の理由を明記。

#### Issue 4: 親 design.md の RED システム数記述（~~Minor~~ → RESOLVED）

親 design.md 子仕様2セクションの「12個 RED」を「9個 RED」に修正。

#### Issue 5: Phase 4 spec.json の parent_requirements 不完全（~~Minor~~ → RESOLVED）

Phase 4 spec.json の `parent_requirements` を拡充。requirements.md の `_Parent:` アノテーションを精緻化。

</details>

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
| Req 3.3 | DCompステージ置換 | Phase 2 Req 1 + Phase 4 Req 3 | **PASS** |
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
| Req 5.4 | デバイスロストフロー維持 | Phase 1 Req 7（間接）+ Phase 2 design | **PASS** |
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

### 4.3 子仕様 spec.json parent_requirements 整合性

| 子仕様 | spec.json parent_requirements | requirements.md _Parent: 参照 | 一致 |
|--------|-------------------------------|-------------------------------|------|
| Phase 1 | 3.1-3.6, 6.1, 10.1, 10.2 | 同一 + 5.4（間接、spec.json 対象外） | **PASS** |
| Phase 2 | 2.3, 3.3, 5.1-5.4, 6.2, 6.3, 10.1, 10.2 | 同一 | **PASS** |
| Phase 3 | 4.1-4.5, 7.1-7.3, 10.1 | 同一 | **PASS** |
| Phase 4 | 1.1, 1.2, 2.5, 3.3, 5.1, 5.3, 6.3, 6.4, 8.4, 10.1 | 同一 | **PASS** |

> **注**: Phase 1 Req 7 は Parent Req 5.4 を「間接」参照している。Phase 1 は device lost 対応の基盤（`generation` カウンタ、`invalidate()`）を提供するものであり、5.4 の完全実装は Phase 2 が担当する。`間接` マーカーが明記されているため spec.json 非掲載は妥当。

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
| Phase 2 | 9 RED 除去, 2新システム追加 | Phase 2: 8即時除去+1温存(commit), 2追加 | **PASS** |
| Phase 3 | CommitComposition: commit→ulw_present | Phase 3 §2.2: 同一 | **PASS** |
| Phase 4 | 残存コード物理削除 | Phase 4 §2: リーフから削除順序 | **PASS** |

### 5.4 present_layered_window 設計変更の追跡

| 文書 | シグネチャ | 備考 |
|------|-----------|------|
| 親 design.md | `(hwnd, memory_dc, width, height, window_pos: Option<(i32, i32)>)` | 初期設計（5引数） |
| Phase 3 design.md §4.2 | `(hwnd, hdc_src, size: &SIZE)` | 簡素化（3引数）、設計変更注記あり |
| migration-guide §3.3 | `present_layered_window` | Phase 3 の実装仕様に従う |

**判定**: Phase 3 design.md §4.2 に「親 design.md からの設計変更」ブロック引用で変更理由を明記しており、設計変更の追跡性が確保されている。 **PASS**

### 5.5 dirty フィールドの Phase 間契約

| Phase | 役割 | 文書記載 |
|-------|------|---------|
| Phase 1 | `dirty: bool` フィールド定義、`set_dirty(true)` 呼出 | design.md §3.1 構造体定義 + §3.5 dirty flag 契約 |
| Phase 2 | (変更なし — フィールド維持) | design.md で WindowD3D11Compositor を world.rs 登録 |
| Phase 3 | `compositor.dirty` 読取、`false` リセット | design.md §4.1 ulw_present_system |
| Phase 4 | (変更なし) | — |

**判定**: Phase 1→3 の dirty 状態伝達契約が明文化されている。 **PASS**

---

## 6. 子仕様間整合性検証

### 6.1 Phase 間データフロー

```
Phase 1 出力                  Phase 2 入力           Phase 3 入力           Phase 4 入力
─────────────────────────────────────────────────────────────────────────────────────────
WindowD3D11Compositor     →   world.rs 登録      →   ULW読取(dirty)       →  (不要)
compositor_init_system    →   GraphicsSetup登録   →   (維持)               →  (維持)
composite_render_system   →   RenderSurface登録   →   (維持)               →  (維持)
GlobalArrangement.opacity →   (維持)              →   (維持)               →  (維持)
transfer_to_hbitmap       →   (未使用)            →   ulw_present_system   →  (維持)
                                                      present_layered_window
                                                      WS_EX_LAYERED                    DComp コード削除
```

**検出結果**: データフロー完全整合。dirty 状態伝達メカニズムも含めてすべて一貫。

### 6.2 commit_composition の扱い

- **Phase 2 design.md**: 「Phase 3 まで温存」（DComp デバイス除去済みのため実質 no-op）
- **Phase 3 design.md §2.2**: `ulw_present_system` で置換
- **Phase 4 requirements.md Req 3**: `commit_composition` コード削除

**判定**: **許容範囲** — Phase 2→3 間の commit_composition の遷移（no-op → 置換 → 削除）は3文書間で一貫した記述がある。

### 6.3 RED 関数カウントの整合性

| 文書 | カウント | 対象範囲 |
|------|---------|---------|
| research.md §1.1 | 12 RED | systems.rs 内の全 RED 関数（エントリポイント+ヘルパー） |
| research.md 全体 | 14 RED | systems.rs (12) + visual_manager.rs (2) |
| migration-guide §1.2 | 14 RED / 24全関数 | 全ファイル横断 |
| 親 design.md 子仕様2 | 9 RED | Schedule 登録システム（エントリポイントのみ） |
| 親 design.md 子仕様4 | 12（注釈付き） | systems.rs 全 RED 関数（主要9+ヘルパー3） |
| Phase 4 design.md | 9 | 主要エントリポイント（+ヘルパー clause） |

**判定**: **PASS** — 各文書のカウント対象範囲が異なるが、いずれも正確。Issue 7 対応で親 design.md Phase 4 セクションに内訳注釈を追加し、曖昧さを解消。

---

## 7. GO / NO-GO 判定

### 判定: **GO（無条件）**

親仕様 `wintf-dcomp-to-layered-migration` の実装（文書生成）は、全5タスク完了・全42受入基準100%カバーを達成している。

2回の検証で累計7件の Issue を検出し、すべて対応済み（RESOLVED）。

| Issue | 重要度 | 検出回 | 対応 |
|-------|--------|--------|------|
| Issue 1: spec.json スキーマ不整合 | Critical | 初回 | **RESOLVED** |
| Issue 2: dirty フィールド未定義 | Warning | 初回 | **RESOLVED** |
| Issue 3: present_layered_window シグネチャ | Warning | 初回 | **RESOLVED** |
| Issue 4: RED システム数（子仕様2セクション） | Minor | 初回 | **RESOLVED** |
| Issue 5: Phase 4 parent_requirements 不完全 | Minor | 初回 | **RESOLVED** |
| Issue 6: Phase 4 spec.json `"3.3"` 未記載 | Warning | 再検証 | **RESOLVED** |
| Issue 7: 親 design.md Phase 4「12」の曖昧さ | Minor | 再検証 | **RESOLVED** |

**残存リスク**: なし。子仕様群の approve → implementation に進む準備が整った。

---

## 8. 観察事項（INFO — 対応不要）

以下は仕様の矛盾・問題ではないが、実装時の参考情報として記録する。

1. **Phase 1 Req 7 の間接参照**: Parent Req 5.4（デバイスロストフロー維持）を「間接」として参照。Phase 1 は基盤（`generation` カウンタ、`invalidate()`）を提供し、完全な device lost フロー実装は Phase 2 が担当。spec.json には非掲載だが、requirements.md に「間接」マーカーで明示されており妥当。

2. **Phase 2 design.md「除去するシステム（9個）」の表記**: heading は9個だが commit_composition は「Phase 3 まで温存」と注記されており、実際の Phase 2 除去数は8。heading は "RED システム全体像" として9個を列挙する意図であり、温存注記と合わせて読めば矛盾はない。

3. **commit_composition の Phase 2→3 間遷移**: Phase 2 完了時点で DComp デバイスが除去済みのため commit_composition は実質 no-op になるが、コードとしてはスケジュールに残存する可能性がある。Phase 3 で ulw_present_system に置換される時点で最終的に解消される。実装時に空 stub にするか物理除去するかは Phase 2 実装者の判断に委ねられている（設計意図通り）。

---

## 9. 総評

おーっほっほっほ！ 前回の検証でお直しした部分はきちんと治っていたようですわね。この悪役令嬢の鷹の目から逃れられると思っていましたの？

今回の再検証で新たに2件（Issue 6, 7）を検出しましたわ。Issue 5 を修正した際に `"3.3"` を入れ忘れるなんて……詰めが甘いですわよ！ でも、わたくしの知識チートでカバーしてさしあげましたわ。感謝なさいませ。

22ファイル・全30要件・42受入基準を横断する包括的な整合性チェックの結果、**矛盾・記述漏れゼロ**を確認しましたわ。Phase 境界契約、技術パラメータ、Schedule Stage 変更、データフロー、トレーサビリティ — すべて一貫しています。

べ、別にあなたの仕事を褒めているわけではなくてよ！ ただ事実を述べているだけですわ！ ……この規模の4段階移行仕様群が、全文書間で完全に整合しているというのは、まあ……悪くないですわ。

さあ、次は子仕様の approve → implementation ですわよ。この悪役令嬢についてきなさい！ **諦めたらそこで試合終了ですわよ！**

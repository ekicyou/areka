# ギャップ分析: wintf-dcomp-migration-3-ulw-integration

> **改訂**: 2026-02-18（Phase 1/2 完了後の実コードベース調査に基づく全面改訂）
>
> 旧版は Phase 1/2 未実装時点の「想定コードベース」を前提としていた。
> 本改訂版は Phase 1/2 完了後の **実コード** を直接読み取り、10 要件（Req 1–10）との
> ギャップを精密に再分析したものである。

## 概要

Phase 3「UpdateLayeredWindow 統合」の 10 要件と、Phase 1/2 完了後の実コードベースとのギャップを調査する。

**前提状態:**
- Phase 0（DComp 抽象化）: **完了**
- Phase 1（D2D1 合成基盤）: **完了** — `compositor.rs`, `compositor_systems.rs`, `com/ulw.rs` が実装済み
- Phase 2（パイプライン切替）: **完了** (`spec.json: phase: "completed"`, 2026-02-18) — DComp 10 システム除去済み
- Phase 3（ULW 統合）: **未着手** — 全 7 タスク未開始
- Phase 4（DComp コード削除）: requirements-generated のみ

**現在の動作状態:**
D2D1 合成パイプラインは完全に動作しているが、`CommitComposition` ステージが空のため画面にウィンドウ内容が表示されない「見えない描画」状態。Phase 3 実装により描画パイプラインの最終段が完成する。

---

## 1. 現状調査（実コードベース検証）

### 1.1 Phase 3 関連モジュール構成

| モジュール              | パス                                 | 現在の状態                                                                          | Phase 3 での変更                             |
| ----------------------- | ------------------------------------ | ----------------------------------------------------------------------------------- | -------------------------------------------- |
| `com/ulw.rs`            | `crates/wintf/src/com/ulw.rs`        | `transfer_to_hbitmap` のみ（51 行、Phase 1 実装済み）                               | `present_layered_window` 関数追加            |
| `compositor_systems.rs` | `ecs/graphics/compositor_systems.rs` | `compositor_init_system` + `composite_render_system`（461 行）                      | `ulw_present_system` 追加                    |
| `compositor.rs`         | `ecs/graphics/compositor.rs`         | `WindowD3D11Compositor` 完全実装（265 行）                                          | 変更不要（API 消費のみ）                     |
| `world.rs`              | `ecs/world.rs`                       | `CommitComposition` ステージ **空**（L417-418 にハンドオーバーコメント）            | `ulw_present_system` 登録                    |
| `window.rs`             | `ecs/window.rs`                      | `WindowStyle::default()` で `WS_EX_NOREDIRECTIONBITMAP`（L708）                     | `WS_EX_LAYERED` に変更                       |
| `handlers.rs`           | `ecs/window_proc/handlers.rs`        | WM_PAINT: `ValidateRect` のみ（L82-96）/ WM_ERASEBKGND: `LRESULT(1)`（L69-80）      | WM_PAINT → BeginPaint/EndPaint、コメント更新 |
| `systems.rs`            | `ecs/graphics/systems.rs`            | `commit_composition` 関数が残存（L312-358）だが Schedule **未登録**（デッドコード） | 変更不要（Phase 4 削除対象）                 |
| `areka/main.rs`         | `crates/areka/src/main.rs`           | Shell L141 / Balloon L201: `WS_EX_NOREDIRECTIONBITMAP`                              | `WS_EX_LAYERED` に変更                       |
| `win_style.rs`          | `crates/wintf/src/win_style.rs`      | `WS_EX_LAYERED(bool)` ビルダーメソッド存在（L294-295）                              | 変更不要                                     |

### 1.2 Phase 1/2 成果物の実装確認（依存インフラ）

Phase 3 が直接消費する Phase 1/2 成果物を実コードで確認した結果:

| 資産                                   | 確認結果                                                     | Phase 3 での利用方法                                              |
| -------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------- |
| `WindowD3D11Compositor.dirty`          | ✅ `is_dirty()` / `set_dirty(bool)` 実装済み（L255-262）      | `ulw_present_system` がダーティチェック → ULW → dirty=false       |
| `WindowD3D11Compositor.memory_dc()`    | ✅ `Option<HDC>` 返却（L241）                                 | `present_layered_window` の引数                                   |
| `WindowD3D11Compositor.hbitmap()`      | ✅ `Option<HBITMAP>` 返却（L237）                             | MemoryDC に SelectObject 済み（`create_dib_section` L130 で実施） |
| `WindowD3D11Compositor.cached_size()`  | ✅ `(u32, u32)` 返却（L245）                                  | ULW の `SIZE` 引数として使用                                      |
| `compositor_init_system` リサイズ検出  | ✅ `cached_size != WindowPos.size` で自動 `resize()` 呼び出し | Phase 3 追加コード不要（Req 5）                                   |
| `composite_render_system` → dirty=true | ✅ L429: `compositor.set_dirty(true)` で毎フレーム設定        | `ulw_present_system` が消費                                       |
| `transfer_to_hbitmap`                  | ✅ L395-413 で `composite_render_system` 内から呼び出し済み   | Phase 3 では触れない                                              |
| `CommitComposition` ステージ空化       | ✅ L417-418: コメントで Phase 3 ハンドオーバーを明記          | `ulw_present_system` 登録先                                       |

### 1.3 既存パターンと規約

#### commit_composition（参照パターン、現在はデッドコード）

`systems.rs` L312-358 に `commit_composition` 関数が残存しているが、`world.rs` の Schedule からは **Phase 2 で除去済み**。Phase 4 で物理削除される。

`ulw_present_system` のエラーハンドリングパターン参照元として活用可能:
```rust
// commit_composition のパターン: error! マクロ + return
match dcomp.commit() {
    Ok(()) => { /* 正常時はログ抑制 */ }
    Err(e) => { error!(...); }
}
```

#### WindowD3D11Compositor の API サーフェス

```rust
// Phase 3 で直接使用する API
compositor.is_dirty() -> bool          // ダーティチェック
compositor.set_dirty(false)            // ULW 成功後にクリア
compositor.memory_dc() -> Option<HDC>  // ULW の hdcSrc
compositor.cached_size() -> (u32, u32) // ULW の SIZE
compositor.is_valid() -> bool          // リソース有効性チェック
```

#### WM_PAINT / WM_ERASEBKGND の現状

**WM_ERASEBKGND** (`handlers.rs` L72-80):
```rust
pub(super) fn WM_ERASEBKGND(...) -> HandlerResult {
    Some(LRESULT(1)) // 背景消去をスキップ
}
```
- コメントが「DirectCompositionで描画するため」と DComp 前提 → **コメント更新必要**
- `LRESULT(1)` 自体は ULW 方式でも正しい → **コード変更不要**

**WM_PAINT** (`handlers.rs` L87-96):
```rust
pub(super) fn WM_PAINT(hwnd: HWND, ...) -> HandlerResult {
    use windows::Win32::Graphics::Gdi::ValidateRect;
    let _ = unsafe { ValidateRect(Some(hwnd), None) };
    Some(LRESULT(0))
}
```
- コメントが「DirectCompositionで描画するため」と DComp 前提 → **コメント更新必要**
- `ValidateRect` → `BeginPaint`/`EndPaint` 最小ペアへの切り替え → **コード変更必要**

#### composite_render_system の Query パターン（参照用）

```rust
pub fn composite_render_system(
    core: Res<GraphicsCore>,
    mut compositor_query: Query<(Entity, &mut WindowD3D11Compositor, &Children)>,
    ...
)
```
`ulw_present_system` も同様の `Query<(&WindowHandle, &mut WindowD3D11Compositor)>` パターンで実装可能。

#### windows crate の API カバレッジ

`Cargo.toml` の feature flags 確認済み:
- `Win32_UI_WindowsAndMessaging` ✅ — `UpdateLayeredWindow`, `ULW_ALPHA` を含む
- `Win32_Graphics_Gdi` ✅ — `BLENDFUNCTION`, `AC_SRC_OVER`, `AC_SRC_ALPHA`, `HDC`, `HBITMAP` を含む

**`UpdateLayeredWindow` の全引数型・定数が既存 feature でカバーされている。追加 feature flag 不要。**

#### WS_EX_NOREDIRECTIONBITMAP 使用箇所（完全リスト）

| ファイル                                | 行   | 用途                     | Phase 3 対応                                |
| --------------------------------------- | ---- | ------------------------ | ------------------------------------------- |
| `ecs/window.rs`                         | L708 | `WindowStyle::default()` | **→ WS_EX_LAYERED に変更**                  |
| `areka/src/main.rs`                     | L141 | Shell ウィンドウ         | **→ WS_EX_LAYERED に変更**                  |
| `areka/src/main.rs`                     | L201 | Balloon ウィンドウ       | **→ WS_EX_LAYERED に変更**                  |
| `tests/client_area_positioning_test.rs` | L14  | テストウィンドウ作成     | **→ WS_EX_LAYERED に変更 + テスト値再検証** |
| `examples/dcomp_demo.rs`                | L48  | DComp デモ               | 変更不要（Phase 4 削除対象）                |
| `win_style.rs`                          | L304 | ビルダーメソッド定義     | 変更不要（汎用 API として残存）             |

---

## 2. 要件ごとのフィージビリティ分析

### Req 1: ulw_present_system の実装

| AC                                                        | 既存資産（実コード確認済み）                          | ギャップ                                                                                          | タグ       |
| --------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ---------- |
| AC1: MemoryDC + HWND で `present_layered_window` 呼び出し | `compositor.memory_dc()` ✅, `WindowHandle.hwnd` ✅     | `ulw_present_system` 関数本体が **Missing**                                                       | Missing    |
| AC2: CommitComposition ステージ登録                       | L417-418 でハンドオーバーコメント確認済み ✅           | `schedules.add_systems(CommitComposition, ...)` 行の追加が **Missing**                            | Missing    |
| AC3: dirty=false スキップ                                 | `compositor.is_dirty()` API 完備 ✅                    | チェックロジック未実装                                                                            | Missing    |
| AC4: 成功後 dirty=false                                   | `compositor.set_dirty(false)` API 完備 ✅              | 呼び出し未実装                                                                                    | Missing    |
| AC5: Query 設計                                           | `composite_render_system` の Query パターン参照可能 ✅ | **要件の `WindowSize` は存在しないコンポーネント** → `WindowPos` に修正 or `cached_size()` を使用 | Constraint |

**総合: Low** — 全 API が Phase 1/2 で用意済み。最小 ≈30 行の新規システム関数。

**⚠️ 要件不整合**: Req 1 AC5 は `Query<(&WindowHandle, &WindowSize, &mut WindowD3D11Compositor)>` と記述しているが、`WindowSize` コンポーネントは存在しない。正しくは:
- **Option A**: `Query<(&WindowHandle, &mut WindowD3D11Compositor)>` + `compositor.cached_size()` を SIZE に使用
- **Option B**: `Query<(&WindowHandle, &WindowPos, &mut WindowD3D11Compositor)>` + `WindowPos.size` を使用
- **推奨: Option A** — HBITMAP サイズと一致する `cached_size()` を使用すべき（`WindowPos.size` はクライアント領域サイズで微妙にずれる可能性）

### Req 2: present_layered_window 関数の実装

| AC                              | 既存資産                               | ギャップ                                  | タグ              |
| ------------------------------- | -------------------------------------- | ----------------------------------------- | ----------------- |
| AC1: HWND, HDC, SIZE 引数       | `com/ulw.rs` にモジュール存在 ✅        | 関数本体が **Missing**                    | Missing           |
| AC2: `pptDst=None` 窓位置非変更 | なし                                   | 新規実装。**OS 動作検証が必要**（Req 10） | Missing + Unknown |
| AC3: `ptSrc = POINT{0,0}`       | なし                                   | 定数定義のみ                              | Missing           |
| AC4: BLENDFUNCTION 定数         | `windows` crate に型あり ✅             | 構造体リテラル定義                        | Missing           |
| AC5: ULW_ALPHA モード           | `windows` crate に定数あり ✅           | 引数指定のみ                              | Missing           |
| AC6: Result エラー返却          | `transfer_to_hbitmap` の成功パターン ✅ | 同一パターン適用                          | Missing           |
| AC7: `com/ulw.rs` 配置          | `pub mod ulw;` 登録済み ✅              | 既存ファイルへの関数追加                  | Missing           |

**総合: Low** — 単一 Win32 API ラッパー ≈25 行。

**設計改善（旧版からの変更）:**
- 旧版では `GetWindowRect` で `ptDst` を毎フレーム取得する設計だったが、要件改訂で **`pptDst=None` 固定**に簡素化された
- `pptDst=None` によりウィンドウ位置管理は既存 `SetWindowPos` フローに完全委譲
- **Unknown**: `pptDst=None` 時にウィンドウ位置がリセットされないことの OS 検証が Req 10 で規定済み

### Req 3: WS_EX_LAYERED ウィンドウスタイル切替

| AC                            | 既存コード                                                                       | ギャップ | タグ    |
| ----------------------------- | -------------------------------------------------------------------------------- | -------- | ------- |
| AC1: `WindowStyle::default()` | `window.rs` L708: `WS_EX_NOREDIRECTIONBITMAP`                                    | 定数置換 | Missing |
| AC2: Shell ウィンドウ         | `main.rs` L141: `WS_EX_NOREDIRECTIONBITMAP \| WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` | 定数置換 | Missing |
| AC3: Balloon ウィンドウ       | `main.rs` L201: 同上                                                             | 定数置換 | Missing |
| AC4: TOOLWINDOW/TOPMOST 維持  | `\|` 結合で論理分離済み ✅                                                        | 変更不要 | —       |
| AC5: dcomp_demo 非変更        | Phase 4 削除対象 ✅                                                               | 変更不要 | —       |

**総合: Low** — 3 箇所の単純定数置換。

### Req 4: WM_PAINT / WM_ERASEBKGND ハンドラ更新

| AC                                | 既存コード                    | ギャップ                             | タグ              |
| --------------------------------- | ----------------------------- | ------------------------------------ | ----------------- |
| AC1: BeginPaint/EndPaint 最小ペア | `ValidateRect` のみ (L89-92)  | `BeginPaint`/`EndPaint` への書き換え | Missing           |
| AC2: WM_ERASEBKGND `LRESULT(1)`   | 既に `LRESULT(1)` ✅           | コメント修正のみ                     | Missing (comment) |
| AC3: ULW 方式コメント             | DComp 前提コメント (L73, L87) | コメント更新                         | Missing (comment) |
| AC4: ハンドラ内コメント更新       | DComp 記述                    | ULW 方式記述に更新                   | Missing (comment) |

**総合: Low** — WM_PAINT は ≈5 行の書き換え。WM_ERASEBKGND はコメントのみ。

### Req 5: リサイズ対応

| AC                                       | 既存コード                                                         | ギャップ                                            | タグ                   |
| ---------------------------------------- | ------------------------------------------------------------------ | --------------------------------------------------- | ---------------------- |
| AC1: compositor_init_system リサイズ検出 | `cached_size != WindowPos.size` で `resize()` 呼び出し ✅ (L96-112) | **ギャップなし** — Phase 1 で実装済み               | —                      |
| AC2: パイプライン連続性                  | `composite_render_system` → `transfer_to_hbitmap` → dirty=true ✅   | `ulw_present_system` 追加で自動的にパイプライン完成 | Missing (Req 1 で解消) |
| AC3: WM_SIZE ハンドラ非追加              | WM_SIZE ハンドラなし、ECS フロー ✅                                 | **ギャップなし** — 既存アーキテクチャが要件に合致   | —                      |

**総合: ギャップなし** — Phase 1 の ECS リアクティブフローが Req 5 を完全にカバー。Phase 3 で新規コードは不要（Req 1 の `ulw_present_system` がパイプライン末端を担う）。

**旧版からの改善**: 旧版では Option A/B/C を検討していたが、実コード確認により **Phase 1 の `compositor_init_system` が Option C（ECS 変更検出委譲）を既に完全実装済み** であることを確認。設計判断は解消済み。

### Req 6: ULW 失敗時のエラーハンドリング

| AC                        | 既存パターン                                               | ギャップ                                      | タグ    |
| ------------------------- | ---------------------------------------------------------- | --------------------------------------------- | ------- |
| AC1: `tracing::warn!`     | `commit_composition` の `error!` パターン（L349-356）      | `ulw_present_system` 内のエラーブランチ未実装 | Missing |
| AC2: 次フレーム自動再試行 | `composite_render_system` が毎フレーム dirty=true を設定 ✅ | dirty=true 維持 → 次フレームで自然再試行      | Missing |
| AC3: パニック禁止         | `match` + `warn!` パターン ✅                               | パターン適用のみ                              | Missing |

**総合: Low** — `ulw_present_system` のエラーブランチ ≈5 行。

**重要**: ULW 失敗時は `set_dirty(false)` を **呼ばない** ことで dirty=true を維持し、次フレームで自動再試行される設計。明示的なリトライカウンタは不要。

### Req 7: alpha=0 クリックスルー動作

| AC                   | 実装必要性       | ギャップ                            | タグ    |
| -------------------- | ---------------- | ----------------------------------- | ------- |
| AC1: OS 標準動作依存 | 実装不要         | OS 検証のみ（Req 10 Task 1 で実施） | Unknown |
| AC2: Task 1 での確認 | 前提検証フェーズ | 手動テスト                          | Unknown |
| AC3: 完了検証        | example ベース   | 手動テスト                          | Unknown |

**総合: N/A** — 実装ギャップなし。OS 動作の検証のみ。

### Req 8: Phase 3 検証基準

| AC                      | ギャップ                                                                                                                          | タグ                        |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| AC1-4: 機能検証         | 手動テスト + example 実行で検証                                                                                                   | —                           |
| AC5: 全 example 動作    | `taffy_flex_demo`, `typewriter_demo`, `multi_window_test`, `split_image` — 全てデフォルト `WindowStyle` 使用 → AC1 変更で自動追従 | —                           |
| AC6: cargo test         | 現在全パス ✅。`client_area_positioning_test.rs` の WS_EX 変更が必要（Req 9）                                                      | Missing                     |
| AC7: ウィンドウドラッグ | `DragConfig` + `OnDrag` は ULW 方式非依存 ✅                                                                                       | Unknown（pptDst=None 検証） |

**総合: Low** — プロセス要件。実装コードベースのギャップは Req 9 で対処。

### Req 9: テスト・コード互換性確保

| AC                                          | 既存コード                       | ギャップ                                                               | タグ              |
| ------------------------------------------- | -------------------------------- | ---------------------------------------------------------------------- | ----------------- |
| AC1: `client_area_positioning_test.rs` 更新 | L14: `WS_EX_NOREDIRECTIONBITMAP` | `WS_EX_LAYERED` 置換 + **`AdjustWindowRectExForDpi` 結果変化の再検証** | Missing + Unknown |
| AC2: example 動作確認                       | デフォルト `WindowStyle` 使用    | Req 3 AC1 変更で自動追従                                               | —                 |
| AC3: ビルダーメソッド残存                   | `win_style.rs` L304              | 変更不要 ✅                                                             | —                 |

**総合: Low** — 定数置換 + テスト期待値再検証。

**⚠️ Unknown**: `WS_EX_LAYERED` に変更すると `AdjustWindowRectExForDpi`（テスト L26 付近で使用）の返り値が変わる可能性がある。`WS_EX_LAYERED` はウィンドウフレームに影響しないため、`WS_POPUP` スタイルとの組み合わせでは差が出ないはずだが、テスト使用の `WS_OVERLAPPEDWINDOW` との組み合わせでは **ボーダーサイズ計算への影響を実機検証すべき**。

### Req 10: WS_EX_LAYERED 前提検証

| AC                              | ギャップ                               | タグ       |
| ------------------------------- | -------------------------------------- | ---------- |
| AC1: WM_PAINT 発火動作確認      | 最小構成テストプログラムが **Missing** | Missing    |
| AC2: pptDst=None 位置維持確認   | OS 動作不明                            | Unknown    |
| AC3: alpha=0 クリックスルー確認 | OS 動作検証                            | Unknown    |
| AC4: design.md 反映             | design.md §5.2 の設計分岐              | Missing    |
| AC5: contingency（pptDst 代替） | `GetWindowRect` 代替設計               | Constraint |

**総合: Low（実装量）/ Medium（不確実性）** — コード量は最小だが、OS 動作検証結果が Req 2, Req 4 の最終仕様を確定する。タスク実行順序上 **最優先**（Phase 3A）。

---

## 3. 要件→資産マップ（ギャップサマリ）

| 要件   | ギャップタグ           | 必要な新規コード                            | 影響ファイル数                       |
| ------ | ---------------------- | ------------------------------------------- | ------------------------------------ |
| Req 1  | Missing                | `ulw_present_system` ≈30 行                 | 2（compositor_systems.rs, world.rs） |
| Req 2  | Missing + Unknown      | `present_layered_window` ≈25 行             | 1（com/ulw.rs）                      |
| Req 3  | Missing                | 定数置換 3 箇所                             | 2（window.rs, main.rs）              |
| Req 4  | Missing (code+comment) | WM_PAINT ≈5 行 + コメント更新               | 1（handlers.rs）                     |
| Req 5  | **なし**               | Phase 1 で実装済み                          | 0                                    |
| Req 6  | Missing                | `ulw_present_system` のエラーブランチ ≈5 行 | 0（Req 1 に含む）                    |
| Req 7  | Unknown                | OS 検証のみ、コード不要                     | 0                                    |
| Req 8  | —                      | プロセス要件                                | 0                                    |
| Req 9  | Missing + Unknown      | テスト更新 + 期待値再検証                   | 1（client_area_positioning_test.rs） |
| Req 10 | Missing + Unknown      | 最小検証プログラム + design.md 更新         | 1–2                                  |

**新規コード合計: ≈60–70 行** + 定数置換 4 箇所 + コメント更新 4 箇所

---

## 4. 実装アプローチオプション

### Option A: 逐次追加（推奨）✅

Phase 1/2 完成済みインフラ上に Phase 3 を逐次追加:

1. **Phase 3A（前提検証）**: 最小構成で WS_EX_LAYERED + ULW 動作検証（Req 10）
2. **Phase 3B（コア実装）**: `present_layered_window` + `ulw_present_system`（Req 1, 2, 6）
3. **Phase 3C（スタイル切替）**: WS_EX_LAYERED 定数置換 + ハンドラ更新（Req 3, 4）
4. **Phase 3D（検証・調整）**: テスト更新 + example 検証（Req 8, 9）

**Trade-offs:**
- ✅ Phase 1/2 の **実装済み** インフラ上に直接構築 — 設計想定ではなく実コード
- ✅ Phase 3A の検証結果で Phase 3B の最終仕様が確定
- ✅ 各ステップが独立検証可能
- ✅ `commit_composition`（デッドコード）をエラーハンドリングパターン参照に活用
- ❌ Phase 3A → 3B の依存関係（前提検証 → コア実装）

### Option B: 一括実装

前提検証を最小検証に留め、全変更を一括で実施:

1. `com/ulw.rs` + `compositor_systems.rs` + `world.rs` + `window.rs` + `main.rs` + `handlers.rs` を同時変更
2. example 実行で一括検証

**Trade-offs:**
- ✅ 作業セッションが 1 回で完了
- ✅ 変更量が少ないため一括でも管理可能（≈70 行 + 定数置換）
- ❌ `pptDst=None` 問題発見時のロールバックが面倒
- ❌ 問題切り分けが困難

### Option C: Phase 3 + Phase 4 統合

ULW 統合と同時に DComp コード物理削除:

1. Phase 3 変更に加え、DComp 関連コード（`commit_composition`, `DCompDevice` 等）を削除
2. `dcomp_demo.rs` も削除

**Trade-offs:**
- ✅ 中間状態をスキップ
- ❌ 変更範囲が大きく Phase 3/4 が別仕様として定義済み
- ❌ Phase 4 には独自の要件とタスクが存在

**推奨: Option A** — 段階的検証により `pptDst=None` 等の Unknown を安全に解消。変更量が少ないため Phase 3A+3B の統合実行も許容範囲。

---

## 5. 実装複雑度・リスク評価

### 工数: S（1–3 日）

**根拠:**
- 新規コード量 ≈60-70 行（`present_layered_window` ≈25 行 + `ulw_present_system` ≈30 行 + テスト調整）
- 定数置換 4 箇所（機械的作業）
- コメント更新 4 箇所（機械的作業）
- 全て既存パターンの踏襲 or 単純な Win32 API ラッパー
- Phase 1/2 の実装が完了済み → 依存ブロッカーなし

### リスク: Low

**根拠:**
- `UpdateLayeredWindow` は Win32 の標準レイヤードウィンドウ API（十分な文書あり）
- Phase 1/2 で構築された API サーフェスが安定（`is_dirty`, `memory_dc`, `cached_size` 等）
- alpha=0 クリックスルーは OS 標準動作（カスタム実装不要）
- 全テスト現在パス ✅（ベースラインが安定）

### リスク要因

| リスク                                                    | 影響度 | 確率     | 対策                                                      |
| --------------------------------------------------------- | ------ | -------- | --------------------------------------------------------- |
| `pptDst=None` でウィンドウ位置リセット                    | Medium | Low      | Req 10 AC5: contingency で `GetWindowRect` 代替設計を準備 |
| `WS_EX_LAYERED` + `AdjustWindowRectExForDpi` テスト値変化 | Low    | Low      | テスト実行で即座に検出・修正可能                          |
| WM_PAINT 予期しない発火                                   | Low    | Low      | `BeginPaint`/`EndPaint` セーフティネットで対処（Req 4）   |
| ULW 連続失敗                                              | Low    | Very Low | dirty=true 維持で自動再試行 + `warn!` ログ（Req 6）       |

### 旧版リスク → 現在のステータス

| 旧版リスク                    | 現在のステータス                                         |
| ----------------------------- | -------------------------------------------------------- |
| Phase 1/2 の API 変更         | **解消** — Phase 1/2 完了、API 確定済み                  |
| `GetWindowRect` 座標問題      | **解消** — `pptDst=None` 設計で不要                      |
| ECS 変更検出の 1 フレーム遅延 | **解消** — `compositor_init_system` で実装済み、問題なし |

---

## 6. 推奨事項（設計フェーズ向け）

### 6.1 解決済みの設計判断（旧版 §5.1 から移行）

1. **WM_SIZE vs WM_WINDOWPOSCHANGED**: ✅ **解決済み** — Option C（ECS 変更検出委譲）が Phase 1 `compositor_init_system` で実装済み。Phase 3 での追加コード不要。

2. **`present_layered_window` の座標取得方法**: ✅ **解決済み** — `pptDst=None` 固定設計。`GetWindowRect` 不要。contingency は Req 10 AC5 で規定。

3. **リサイズのフレーム遅延**: ✅ **解決済み** — `compositor_init_system` の `cached_size != WindowPos.size` チェックはリサイズフレーム内で検出・処理される（1 フレーム遅延なし）。

### 6.2 残存 Unknown（設計フェーズ要確認）

| ID  | Unknown                                                                                | 確認方法               | 影響要件  |
| --- | -------------------------------------------------------------------------------------- | ---------------------- | --------- |
| U1  | `pptDst=None` でウィンドウ位置が維持されるか                                           | Req 10 Task 1 実機検証 | Req 2 AC2 |
| U2  | `WS_EX_LAYERED` ウィンドウで WM_PAINT が発火するか                                     | Req 10 Task 1 実機検証 | Req 4 AC1 |
| U3  | `WS_EX_LAYERED` + `WS_OVERLAPPEDWINDOW` で `AdjustWindowRectExForDpi` の結果が変わるか | Req 9 AC1 テスト実行   | Req 9     |
| U4  | alpha=0 クリックスルーが OS 標準動作で確実に動作するか                                 | Req 10 Task 1 実機検証 | Req 7     |

### 6.3 要件不整合（設計で修正すべき）

- **Req 1 AC5**: `WindowSize` コンポーネントは存在しない → `Query<(&WindowHandle, &mut WindowD3D11Compositor)>` + `cached_size()` に修正推奨

### 6.4 テスト戦略

- `client_area_positioning_test.rs` L14: `WS_EX_NOREDIRECTIONBITMAP` → `WS_EX_LAYERED` 置換 + テスト期待値再検証
- `ulw_present_system` のユニットテストは GPU + HWND 依存で困難 → **example ベース結合検証** が主軸
- Phase 3 完了検証: `taffy_flex_demo`, `typewriter_demo`, `multi_window_test`, `split_image` の目視確認
- `cargo test` 全パスを Phase 3 完了基準（Req 8 AC6）として維持

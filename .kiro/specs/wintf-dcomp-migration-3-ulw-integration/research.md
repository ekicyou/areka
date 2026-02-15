# ギャップ分析: wintf-dcomp-migration-3-ulw-integration

## 概要

本分析は、Phase 3「UpdateLayeredWindow 統合」の要件と既存コードベースとのギャップを調査し、実装戦略を評価する。Phase 1/2 は設計・タスク生成済みだが実装未着手であるため、本分析は **Phase 1/2 完了後の想定コードベース** を前提として行う。

---

## 1. 現状調査

### 1.1 関連モジュール構成

| モジュール | パス | 役割 | 本仕様との関係 |
|-----------|------|------|---------------|
| `graphics/systems.rs` | `ecs/graphics/systems.rs` | `commit_composition`（DComp Commit） | ULW 置換対象 |
| `world.rs` | `ecs/world.rs` | Schedule 定義・システム登録 | CommitComposition ステージ更新 |
| `window.rs` | `ecs/window.rs` | `WindowStyle`, `WindowPos`, `WindowHandle` | `ex_style` 変更、サイズ取得元 |
| `window_proc/handlers.rs` | `ecs/window_proc/handlers.rs` | WM_PAINT, WM_ERASEBKGND, WM_WINDOWPOSCHANGED | ハンドラ更新 |
| `window_proc/mod.rs` | `ecs/window_proc/mod.rs` | メッセージディスパッチテーブル | WM_SIZE 登録（必要に応じて） |
| `com/mod.rs` | `com/mod.rs` | COM モジュール定義 | `ulw` モジュール追加先 |
| `win_style.rs` | `win_style.rs` | `WS_EX_LAYERED()` ビルダー | 既にヘルパーが存在 |
| `areka/main.rs` | `crates/areka/src/main.rs` | Shell/Balloon ウィンドウ定義 | `ex_style` 変更 |

### 1.2 Phase 1/2 で作成予定（未実装）のモジュール

Phase 3 は以下の Phase 1/2 成果物に依存する。現時点では設計のみ存在し、コードは未作成。

| 予定モジュール | 作成元 Phase | 本仕様での利用 |
|---------------|-------------|---------------|
| `ecs/graphics/compositor.rs` | Phase 1 | `WindowD3D11Compositor` コンポーネント（HBITMAP, MemoryDC, dirty フラグ） |
| `ecs/graphics/compositor_systems.rs` | Phase 1 | `compositor_init_system`, `composite_render_system` — Phase 3 は `ulw_present_system` をここに追加 |
| `com/ulw.rs` | Phase 1（部分） | `transfer_to_hbitmap` が Phase 1 で部分作成。Phase 3 で `present_layered_window` を追加 |

### 1.3 既存パターンと規約

#### WM_PAINT / WM_ERASEBKGND ハンドラの現状

**WM_ERASEBKGND** (`handlers.rs` L69-80):
```rust
pub(super) fn WM_ERASEBKGND(...) -> HandlerResult {
    Some(LRESULT(1)) // 背景消去をスキップ
}
```
- ULW 方式でも `LRESULT(1)` は正しい → **コメント修正のみ**

**WM_PAINT** (`handlers.rs` L82-97):
```rust
pub(super) fn WM_PAINT(hwnd: HWND, ...) -> HandlerResult {
    use windows::Win32::Graphics::Gdi::ValidateRect;
    let _ = unsafe { ValidateRect(Some(hwnd), None) };
    Some(LRESULT(0))
}
```
- 現在は `ValidateRect` で無効領域クリア（DComp 前提）
- Req 4 AC1 では `BeginPaint` / `EndPaint` 最小ペアを要求
- **ギャップ**: `ValidateRect` → `BeginPaint`/`EndPaint` への切り替え

#### WM_SIZE / WM_WINDOWPOSCHANGED の現状

- **WM_SIZE**: ECS ディスパッチテーブルに**エントリなし**。`handlers.rs` にも**関数なし**。
- **WM_WINDOWPOSCHANGED** (`handlers.rs` L111-306): サイズ変更時に `WindowPos` と `BoxStyle.size` を更新。
- **ギャップ**: Req 5 は `WindowD3D11Compositor` のリサイズフラグトリガーを要求するが、Phase 1/2 が未実装のため直接のコード修正箇所は設計依存。

#### ウィンドウスクリーン座標の取得方法

`present_layered_window` の `ptDst` にはウィンドウのスクリーン座標が必要。

- `WindowHandle` に `window_to_client_coords()` はあるが、`GetWindowRect` の直接使用はコードベースに存在しない
- `WM_WINDOWPOSCHANGED` ハンドラが `WINDOWPOS` 構造体のウィンドウ座標を取得している（`wp.x`, `wp.y`）
- **アプローチ**: `present_layered_window` 内部で `GetWindowRect(hwnd)` を使用する（ULW の度に最新のスクリーン座標が必要なため）

#### commit_composition の現状

```rust
pub fn commit_composition(
    graphics: Option<Res<GraphicsCore>>,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    // GraphicsCore → dcomp() → IDCompositionDevice3::Commit()
}
```

`world.rs` L440 で `CommitComposition` ステージに登録:
```rust
schedules.add_systems(CommitComposition, crate::ecs::graphics::commit_composition);
```

**Phase 3 ではこの登録を `ulw_present_system` に置換する。**

#### windows crate の API カバレッジ

`Cargo.toml` の feature flags を確認:
- `Win32_UI_WindowsAndMessaging` ✅ — `UpdateLayeredWindow` を含む
- `Win32_Graphics_Gdi` ✅ — `BLENDFUNCTION`, `CreateCompatibleDC`, `SelectObject` 等を含む

**`UpdateLayeredWindow` の引数型は全て既存 feature でカバーされている。**
ただし `ULW_ALPHA`, `AC_SRC_OVER`, `AC_SRC_ALPHA` 定数の feature カバレッジは実装時に要確認（`Win32_UI_WindowsAndMessaging` に含まれる可能性が高い）。

---

## 2. 要件ごとのフィージビリティ分析

### Req 1: ulw_present_system

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: ULW 呼び出し | `commit_composition` のシステム登録パターン | `WindowD3D11Compositor`（Phase 1 前提）の HBITMAP/MemoryDC を使用 | Low |
| AC2: BLENDFUNCTION | なし | 定数定義のみ。`windows` crate に型あり | Low |
| AC3: CommitComposition 登録 | `world.rs` L440 のパターン | `commit_composition` → `ulw_present_system` に単純置換 | Low |
| AC4: ダーティフラグスキップ | `WindowD3D11Compositor.dirty` (Phase 1 設計) | Phase 1 の `dirty` フラグ仕様に依存。パターンは明確 | Low |

**総合難度: Low** — 既存パターンの組み合わせ。Phase 1/2 の成果物に強く依存するが、API 自体は単純。

### Req 2: present_layered_window

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: 引数設計 | `com/` 配下の関数パターン | 新規関数。`unsafe` ブロック内の Win32 API 呼び出し | Low |
| AC2: ptDst スクリーン座標 | なし | `GetWindowRect` で取得。コードベースに既存使用なし | Low |
| AC3: UpdateLayeredWindow 使用 | なし | `windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow` — 新規 API| Low |
| AC4: Result 返却 | `com/` 配下の `windows::core::Result` パターン | 既存パターン（`.ok()?` or `unsafe { ... }.ok()?`） | Low |
| AC5: ファイル配置 | `com/mod.rs` にモジュール登録パターン | `pub mod ulw;` 追加。Phase 1 で `com/ulw.rs` が部分作成済みの想定 | Low |

**総合難度: Low** — 単一の Win32 API ラッパー関数。

**Research Needed:**
- `GetWindowRect` vs `GetWindowPlacement`: ULW の `ptDst` に渡すべき座標の正確な定義（ウィンドウ矩形の左上 vs クライアント領域座標）。MSDN では `ptDst` は "screen position" と記述 → `GetWindowRect` で取得したウィンドウの左上座標が正しい。

### Req 3: WS_EX_LAYERED 切替

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: WindowStyle::default() | L708: `ex_style: WS_EX_NOREDIRECTIONBITMAP` | 単純置換: → `WS_EX_LAYERED` | Low |
| AC2: Shell ex_style | L141: `WS_EX_NOREDIRECTIONBITMAP \| WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` | `WS_EX_NOREDIRECTIONBITMAP` → `WS_EX_LAYERED` 置換 | Low |
| AC3: Balloon ex_style | L201: 同上パターン | 同上 | Low |
| AC4: TOOLWINDOW/TOPMOST 維持 | 既存コードで `\|` 結合 | 変更なし | Low |

**総合難度: Low** — 3 箇所の定数置換。

**注意点:**
- `win_style.rs` に `WS_EX_LAYERED(bool)` ビルダーメソッドが既に存在（L294-295）
- `dcomp_demo.rs` (L48) は `WS_EX_NOREDIRECTIONBITMAP` を使用しているが、Phase 4 で削除対象のため Phase 3 では変更不要
- `client_area_positioning_test.rs` (L14) のテストも `WS_EX_NOREDIRECTIONBITMAP` を使用 — **テスト更新が必要になる可能性あり**

### Req 4: WM_PAINT / WM_ERASEBKGND 更新

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: BeginPaint/EndPaint 最小ペア | 現在: `ValidateRect` のみ | `BeginPaint`/`EndPaint` への書き換え | Low |
| AC2: WM_ERASEBKGND `LRESULT(1)` | 現在: 既に `LRESULT(1)` | コメント修正のみ | Low |
| AC3: ULW 委譲 | 現在は DComp 前提 | 設計上の変更なし、コメント修正で十分 | Low |

**総合難度: Low**

**WM_PAINT の設計判断:**
- **Option A: BeginPaint/EndPaint 最小ペア**: MSDN 準拠。`WS_EX_LAYERED` では通常 WM_PAINT は発火しないが、万が一発火した場合のセーフティネット。
- **Option B: ValidateRect 維持**: 現状維持。機能的には同等だが MSDN のベストプラクティスに従わない。
- **推奨: Option A** — 要件と MSDN ガイダンスに一致。

### Req 5: WM_SIZE 更新

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: リサイズフラグトリガー | `WM_WINDOWPOSCHANGED` でサイズ変更を処理（L111-306） | `WindowD3D11Compositor.resize()` 呼び出しの追加 | Medium |
| AC2: 次フレーム再作成保証 | `WindowD3D11Compositor.resize()` (Phase 1 設計) | Phase 1 の resize メカニズムに依存 | Low |

**総合難度: Low-Medium**

**キーとなる設計判断: WM_SIZE vs WM_WINDOWPOSCHANGED**

現在のコードベースでは `WM_SIZE` ハンドラが**存在せず**、全てのサイズ変更処理は `WM_WINDOWPOSCHANGED` で行われている。

- **Option A: WM_WINDOWPOSCHANGED にリサイズロジック追加**: 既存のサイズ変更検出フロー（BoxStyle.size 更新）に並行して `WindowD3D11Compositor.resize()` を呼ぶ。既存のコード構造に自然に統合される。
- **Option B: WM_SIZE ハンドラ新設**: ディスパッチテーブルに `WM_SIZE` を追加し、専用ハンドラで `WindowD3D11Compositor.resize()` を呼ぶ。責務分離は明確だが、`WM_WINDOWPOSCHANGED` と `WM_SIZE` の両方でサイズ処理が行われる。
- **Option C: ECS の変更検出に委譲**: `Changed<BoxStyle>` や `Changed<WindowPos>` を `compositor_init_system` で検出し、リサイズを自動トリガー。WndProc からの直接呼び出しが不要。

**推奨: Option C** — Phase 1 の `compositor_init_system` 設計（AC3: リサイズ検出）が既に `WindowPos` からのサイズ取得を想定している。WndProc を直接修正するよりも、ECS 変更検出パイプラインに統合する方が疎結合。

ただし「ECS 変更検出は1フレーム遅延する可能性がある」点を考慮し、設計フェーズで最終決定すべき。

### Req 6: ULW エラーハンドリング

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: tracing::warn! | ロギングガイドライン（steering/logging.md） | 既存パターン適用 | Low |
| AC2: 次フレーム再試行 | `commit_composition` のエラーハンドリング | `return` でスキップ → 次フレームで自然に再実行 | Low |
| AC3: パニック禁止 | Rust の `?` + `warn!` パターン | `Result` を `.ok()` で潰す or `match` + `warn!` | Low |

**総合難度: Low** — `commit_composition` のエラーハンドリングパターンをほぼそのまま踏襲。

### Req 7: alpha=0 クリックスルー

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: OS 標準動作依存 | なし | **テスト専用要件** — 実装ではなく OS 動作の検証 | N/A |
| AC2: 実機テスト確認 | なし | 手動テスト or example ベースの検証 | Low |

**総合難度: N/A** — 実装不要。OS 標準動作の検証のみ。

### Req 8: Phase 3 検証基準

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1-4: 機能検証 | — | 手動テスト + example 実行 | Low |
| AC5: 全 example 動作 | `taffy_flex_demo`, `typewriter_demo`, `multi_window_test`, `split_image` | example の WS_EX_NOREDIRECTIONBITMAP 使用箇所を確認 | Low |
| AC6: cargo test | 既存テストスイート | `WS_EX_NOREDIRECTIONBITMAP` を使用するテストの更新が必要 | Low |

**注意**: `client_area_positioning_test.rs` (L14) が `WS_EX_NOREDIRECTIONBITMAP` を使用している。`WS_EX_LAYERED` に変更すると `AdjustWindowRectExForDpi` の結果が変わる可能性がある。

---

## 3. 実装アプローチオプション

### Option A: Phase 1/2 完了後の逐次追加（推奨）

Phase 1/2 の成果物（`WindowD3D11Compositor`, `compositor_systems.rs`, `com/ulw.rs`）が完成した状態で、以下を順次追加:

1. `com/ulw.rs` に `present_layered_window` 関数追加
2. `compositor_systems.rs` に `ulw_present_system` 追加
3. `world.rs` の CommitComposition ステージ更新
4. `window.rs` / `main.rs` の ex_style 変更
5. `handlers.rs` の WM_PAINT 更新

**Trade-offs:**
- ✅ Phase 1/2 の実装を前提とし、最小限の変更量
- ✅ 既存パターンの自然な拡張
- ✅ `com/ulw.rs` は Phase 1 で部分作成済みの想定
- ❌ Phase 1/2 が未完了の場合、実装開始不可

### Option B: ULW 独立プロトタイプ先行

Phase 1/2 を待たずに、DComp パイプライン上で `WS_EX_LAYERED` + ULW の動作検証を行うプロトタイプを作成:

1. 既存の `commit_composition` と並行して ULW 呼び出しを試行
2. DComp Surface → HBITMAP → ULW の暫定パスを構築
3. 動作確認後に Phase 1/2 の成果物に統合

**Trade-offs:**
- ✅ Phase 1/2 待ちなしで ULW 動作検証可能
- ✅ OS 動作の早期確認（クリックスルー等）
- ❌ 暫定コードの作成と破棄が発生
- ❌ DComp + ULW は共存できない可能性がある（WS_EX_LAYERED vs WS_EX_NOREDIRECTIONBITMAP の排他性）

### Option C: Phase 2 に Phase 3 を統合

Phase 2（パイプライン切替）と Phase 3（ULW 統合）を単一仕様として実装:

1. world.rs の DComp → D2D1 切替時に同時に ULW を有効化
2. WS_EX_LAYERED 変更も同時に実施

**Trade-offs:**
- ✅ 中間状態（D2D1 パイプラインだが ULW なし）を通過しない
- ✅ 1回の検証サイクルで完了
- ❌ 変更範囲が大きく、問題切り分けが困難
- ❌ 既に Phase 2/3 が別仕様として定義済み

**推奨: Option A** — 仕様設計に忠実で、Phase 1/2 の安定した成果物の上に構築する最もリスクの低いアプローチ。

---

## 4. 実装複雑度・リスク評価

### 工数: S（1–3 日）

**根拠:**
- 新規コード量が少ない（`present_layered_window` ≈ 30 行、`ulw_present_system` ≈ 30 行）
- 3 箇所の定数置換（`WS_EX_NOREDIRECTIONBITMAP` → `WS_EX_LAYERED`）
- WM_PAINT ハンドラの小規模書き換え
- 全て既存パターンの踏襲 or 単純な Win32 API ラッパー

### リスク: Low

**根拠:**
- `UpdateLayeredWindow` は十分に文書化された Win32 API
- `WS_EX_LAYERED` + `ULW_ALPHA` は標準的なレイヤードウィンドウ技法
- Phase 1/2 の成果物に依存するが、インターフェースは Phase 1 設計で明確
- alpha=0 クリックスルーは OS 標準動作で、カスタム実装は不要

### リスク要因

| リスク | 影響度 | 対策 |
|--------|--------|------|
| Phase 1/2 の API 変更 | Low | Phase 1 設計が安定（`WindowD3D11Compositor` の API は requirements/design で明確） |
| `UpdateLayeredWindow` の `ptDst` 座標問題 | Low | `GetWindowRect` で左上座標を取得すれば正確 |
| `WS_EX_LAYERED` と既存テストの非互換 | Low | `client_area_positioning_test.rs` のスタイル更新で対応 |
| WM_PAINT 不発火時の挙動 | Low | `WS_EX_LAYERED` では通常 WM_PAINT は発火しない。BeginPaint/EndPaint ペアはセーフティネットとして維持 |

---

## 5. 推奨事項（設計フェーズ向け）

### 5.1 優先決定事項

1. **WM_SIZE vs WM_WINDOWPOSCHANGED**: Req 5 の「WM_SIZE でリサイズトリガー」の実装箇所を確定する。Option C（ECS 変更検出に委譲）が最も疎結合だが、フレーム遅延の受容可否を設計段階で判断すべき。

2. **`present_layered_window` の座標取得方法**: `GetWindowRect` を関数内部で呼ぶか、呼び出し元（`ulw_present_system`）から渡すか。呼び出し元が `WindowPos` から取得できる場合、API 呼び出しを減らせる。ただし `WindowPos` はクライアント座標のため、ウィンドウ座標への逆変換が必要 → `GetWindowRect` の直接使用が安全。

### 5.2 Research Needed（設計フェーズで調査）

- **`ULW_ALPHA` / `AC_SRC_OVER` / `AC_SRC_ALPHA` 定数の `windows` crate でのインポートパス確認**: `Win32_UI_WindowsAndMessaging` に含まれるか、別の feature flag が必要か
- **`WS_EX_LAYERED` ウィンドウでの `WM_PAINT` 発火タイミング**: MSDN では「ULW で更新されるウィンドウには WM_PAINT が送信されない」とあるが、`InvalidateRect` 等での強制発火時の挙動を確認
- **マルチモニター環境**: `GetWindowRect` が返す座標がマルチモニター環境で正しくスクリーン座標になることの確認

### 5.3 テスト戦略

- `client_area_positioning_test.rs` の `WS_EX_NOREDIRECTIONBITMAP` を `WS_EX_LAYERED` に更新（or DComp 非依存のスタイルを使用）
- Phase 3 検証は主に example 実行による目視確認
- `ulw_present_system` のユニットテストは困難（GPU + HWND 依存）→ 結合テスト or example ベース検証

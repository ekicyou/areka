# 要件定義書: wintf-dcomp-migration-3-ulw-integration

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 3「UpdateLayeredWindow 統合」を担当する。Phase 2 で D2D1 合成パイプラインに切り替わった描画結果を、UpdateLayeredWindow（ULW）経由でウィンドウに転送し、alpha 透過とクリックスルーを実現する。

### コンテキスト

Phase 2（`wintf-dcomp-migration-2-pipeline-switch`）は **完了済み（2026-02-18 確認）** である。`world.rs` は D2D1 合成パイプライン（`compositor_init_system`, `composite_render_system`）で動作し、DComp システム 10 個は Schedule から除去済み。`CommitComposition` ステージは空の状態で、Phase 3 の `ulw_present_system` 登録を受け入れるハンドオーバーポイントがコメントで明記されている。

現在「Phase 2 完了・Phase 3 未着手」の中間状態にあり、`composite_render_system` が毎フレーム HBITMAP への転送（`transfer_to_hbitmap`）を完了して dirty フラグを設定するが、それを消費する ULW 呼び出しシステムが存在しないため、**画面にはウィンドウ内容が表示されない**。Phase 3 実装により、この描画パイプラインの最終段が完成する。

### 本子仕様のスコープ

- `com/ulw.rs` 追加: `present_layered_window` 関数（`transfer_to_hbitmap` は Phase 1 で実装済み）
- `ecs/graphics/compositor_systems.rs` 追加: `ulw_present_system` 関数
- `ecs/world.rs` 変更: `CommitComposition` ステージに `ulw_present_system` を登録
- `ecs/window.rs` 変更: `WindowStyle::default()` の `ex_style` を `WS_EX_LAYERED` に変更
- `ecs/window_proc/handlers.rs` 変更: WM_PAINT / WM_ERASEBKGND ハンドラ更新
- `areka/src/main.rs` 変更: Shell / Balloon ウィンドウの `ex_style` 更新
- テストファイル更新: `WS_EX_NOREDIRECTIONBITMAP` 参照の移行

### Non-Goals

- DComp コードの物理的削除（Phase 4 で実施）
- 新規 ECS コンポーネントの追加（Phase 1-2 で実装済みコンポーネントを使用）
- ウィジェット描画システムの変更
- `dcomp_demo.rs` の変更（Phase 4 で削除対象）
- WM_SIZE ハンドラの新規追加（ECS リアクティブフローで処理済み）

### 前提条件

- **Phase 2 完了確認済み**（2026-02-18）: `world.rs` が D2D1 合成パイプラインで動作
- `WindowD3D11Compositor` が各ウィンドウエンティティに初期化済み（`compositor_init_system`）
- `composite_render_system` が毎フレーム合成描画 → `transfer_to_hbitmap` → dirty=true の転送を完了
- `CommitComposition` ステージが空（Phase 3 ハンドオーバーポイント準備済み）
- DComp API 呼び出しがゼロ（Phase 2 で grep 検証済み）
- `cargo test` 全テストパス（Phase 2 完了時点で 500+ テスト全パス確認済み）

---

## Requirements

### Requirement 1: ulw_present_system の実装

**Objective:** 開発者として、合成済みビットマップを UpdateLayeredWindow で毎フレーム転送する ECS システムが欲しい。これにより DComp Commit に代わるウィンドウ表示メカニズムが確立され、描画パイプラインの最終段が完成する。

_Parent: Req 4.1, 4.4_

#### Acceptance Criteria

1. The `ulw_present_system` shall `WindowD3D11Compositor` の MemoryDC（HBITMAP が SelectObject 済み）と `WindowHandle` の HWND を使用して `present_layered_window` を呼び出す
2. The `ulw_present_system` shall `world.rs` の `CommitComposition` ステージに登録され、Phase 2 で空化された当該ステージを引き継ぐ
3. When `WindowD3D11Compositor` のダーティフラグが false の時, the `ulw_present_system` shall 当該ウィンドウの ULW 呼び出しをスキップする
4. When `present_layered_window` が成功した時, the `ulw_present_system` shall `WindowD3D11Compositor` のダーティフラグを false に設定する
5. The `ulw_present_system` shall `Query<(&WindowHandle, &WindowSize, &mut WindowD3D11Compositor)>` を使用し、全ウィンドウをイテレートする

#### 設計メモ

- `composite_render_system`（Composition ステージ）が合成描画 → ステージングビットマップ → `transfer_to_hbitmap` → dirty=true のフローを既に完了しているため、`ulw_present_system` は dirty チェック → `present_layered_window` 呼び出し → dirty=false の最小フローのみを実装する
- `transfer_to_hbitmap` は本システムの責務ではない（Phase 1 で `composite_render_system` 内に実装済み）

### Requirement 2: present_layered_window 関数の実装

**Objective:** 開発者として、ULW 呼び出しを抽象化した COM ラッパー関数が欲しい。Win32 API の詳細を隠蔽し、安全な呼び出しインターフェースを提供する。

_Parent: Req 4.1_

#### Acceptance Criteria

1. The `present_layered_window` 関数 shall `HWND`, `HDC`（MemoryDC）, `&SIZE`（ウィンドウサイズ）を引数に取り、`UpdateLayeredWindow` の Win32 API 呼び出しを実行する
2. The `present_layered_window` 関数 shall `pptDst` に `None` を渡し、ウィンドウ位置を変更しない（位置管理は既存 `SetWindowPos` フローに委譲）
3. The `present_layered_window` 関数 shall `ptSrc` に `POINT { x: 0, y: 0 }` を使用する
4. The `present_layered_window` 関数 shall `BLENDFUNCTION { BlendOp: AC_SRC_OVER, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA }` を使用する
5. The `present_layered_window` 関数 shall `ULW_ALPHA` モードで `UpdateLayeredWindow` を呼び出す
6. If `UpdateLayeredWindow` が失敗した場合, the `present_layered_window` 関数 shall `windows::core::Result` でエラーを返す
7. The `present_layered_window` 関数 shall `com/ulw.rs` に配置される（Phase 1 で作成済みの `transfer_to_hbitmap` と同一モジュール）

#### 設計メモ

- 親仕様の設計では `window_pos: Option<(i32, i32)>` を含む 5 引数シグネチャだったが、Phase 3 詳細設計で `pptDst=None` 固定に簡素化。ウィンドウ位置は `SetWindowPos` が管理しており、ULW で二重管理する必要がない
- `pptDst=None` が正しく動作すること（ウィンドウ位置がリセットされないこと）は Task 1（前提検証）で確認する

### Requirement 3: WS_EX_LAYERED ウィンドウスタイル切替

**Objective:** 開発者として、全ウィンドウが WS_EX_LAYERED で作成されるようにしたい。ULW による描画には WS_EX_LAYERED が必須である。

_Parent: Req 4.2_

#### Acceptance Criteria

1. The `WindowStyle::default()` shall `ex_style` を `WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` に変更する
2. The `areka/src/main.rs` の Shell ウィンドウ設定 shall `ex_style` を `WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` に変更する
3. The `areka/src/main.rs` の Balloon ウィンドウ設定 shall `ex_style` を `WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` に変更する
4. The wintf crate shall `WS_EX_TOOLWINDOW | WS_EX_TOPMOST` を維持する（既存動作の継続）
5. The `dcomp_demo.rs` shall 本 Phase では変更しない（Phase 4 で DComp デモごと削除対象）

#### 影響範囲（コード調査結果）

- `ecs/window.rs` L708: `WindowStyle::default()` — **変更必須**
- `areka/src/main.rs` L141: Shell ウィンドウ — **変更必須**
- `areka/src/main.rs` L201: Balloon ウィンドウ — **変更必須**
- `examples/dcomp_demo.rs` L48 — **変更不要**（Phase 4 削除対象）
- `win_style.rs` L304: ビルダーメソッド定義 — **変更不要**（汎用 API として残存）

### Requirement 4: WM_PAINT / WM_ERASEBKGND ハンドラ更新

**Objective:** 開発者として、WS_EX_LAYERED 互換のメッセージハンドラが欲しい。ULW 方式では描画を UpdateLayeredWindow に委ねるため、WM_PAINT/WM_ERASEBKGND は安全な最小実装を維持する。

_Parent: Req 7.1, 7.3_

#### Acceptance Criteria

1. The WM_PAINT ハンドラ shall 現在の `ValidateRect` のみの実装を `BeginPaint` / `EndPaint` の最小ペアに変更し、実際の描画は行わない
2. The WM_ERASEBKGND ハンドラ shall `LRESULT(1)` を返し、背景消去をスキップする（現在の実装を維持）
3. While `WS_EX_LAYERED` が設定されている時, the wintf crate shall WM_PAINT による描画を行わず、ウィンドウ表示を UpdateLayeredWindow に委ねる
4. The ハンドラ内コメント shall DComp 前提の記述から ULW 方式の記述に更新する

#### 設計メモ

- `WS_EX_LAYERED` ウィンドウは通常 WM_PAINT を受信しないとされるが、万が一発火した場合のセーフティネットとして `BeginPaint`/`EndPaint` 最小ペアを維持する（MSDN 準拠）
- WM_ERASEBKGND は既に `LRESULT(1)` を返しており、ULW 方式でもそのまま互換
- Task 1（前提検証）で WM_PAINT 発火動作を確認し、§5.2 の設計分岐を確定する

### Requirement 5: リサイズ対応

**Objective:** 開発者として、リサイズ時に合成ビットマップの再作成が確実にトリガーされることを保証したい。

_Parent: Req 7.2_

#### Acceptance Criteria

1. When ウィンドウサイズが変更された時, the `compositor_init_system` shall `WindowD3D11Compositor` のリサイズを検出し、`resize()` メソッドで合成ビットマップを再作成する（Phase 1 で実装済みの ECS リアクティブフロー）
2. When リサイズ処理後の次フレームで, the wintf crate shall 新サイズの合成ビットマップが `composite_render_system` → `transfer_to_hbitmap` → `ulw_present_system` のパイプラインで正しく転送されることを保証する
3. The Phase 3 実装 shall WM_SIZE ハンドラの新規追加を行わない（既存の `WM_WINDOWPOSCHANGED` → ECS コンポーネント更新 → `compositor_init_system` リアクティブ検出のフローを活用）

#### 設計メモ

- 現在のコードベースでは `WM_SIZE` ハンドラは存在せず、全てのサイズ変更処理は `WM_WINDOWPOSCHANGED` → `WindowPos`/`BoxStyle.size` 更新 → ECS 変更検出で処理される
- Phase 1 の `compositor_init_system` がサイズ変更を自動検出して `WindowD3D11Compositor::resize()` を呼び出すフローが構築済み
- WndProc への直接的なリサイズロジック追加より、ECS パイプラインに統合された方式の方が疎結合（research.md Option C 採用）

### Requirement 6: ULW 失敗時のエラーハンドリング

**Objective:** 開発者として、ULW 呼び出し失敗時に適切なリカバリが行われて欲しい。フレーム落ちは許容するがパニックは許容しない。

_Parent: Req 4.5_

#### Acceptance Criteria

1. If `UpdateLayeredWindow` が失敗した場合, the `ulw_present_system` shall `tracing::warn!` でエラーを記録し、当該フレームをスキップする
2. The `ulw_present_system` shall 失敗後の次フレームで自動的に ULW 呼び出しを再試行する（ダーティフラグを true のまま維持）
3. The wintf crate shall ULW 連続失敗時にパニックしない

### Requirement 7: alpha=0 クリックスルー動作

**Objective:** 開発者として、alpha=0 ピクセル領域でマウスクリックが背後のウィンドウに透過することを確認したい。ULW_ALPHA 方式の標準 OS 動作に依存する。

_Parent: Req 4.3_

#### Acceptance Criteria

1. When ULW_ALPHA で描画された alpha=0 ピクセル領域がクリックされた時, the OS shall 当該クリックを背後のウィンドウに透過する（OS 標準動作への依存）
2. The Phase 3 前提検証（Task 1）shall alpha=0 クリックスルー動作を最小構成で確認する
3. The Phase 3 完了検証 shall alpha=0 クリックスルー動作を実アプリケーション（example）で確認する

### Requirement 8: Phase 3 検証基準

**Objective:** 開発者として、Phase 3 完了時の品質基準を明確にし、次 Phase（Phase 4: DComp コード削除）への移行可否を判断したい。

_Parent: Req 10.1_

#### Acceptance Criteria

1. The Phase 3 完了検証 shall UpdateLayeredWindow での透過ウィンドウ表示が動作することを確認する
2. The Phase 3 完了検証 shall alpha=0 ピクセル領域のクリックスルーが動作することを確認する
3. The Phase 3 完了検証 shall ウィンドウリサイズ時の合成ビットマップ再作成・再転送が正常動作することを確認する
4. The Phase 3 完了検証 shall ULW 失敗時の `tracing::warn!` ログ出力と次フレーム再試行が動作することを確認する
5. The Phase 3 完了検証 shall 全 example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）が ULW 方式で正常動作することを確認する
6. The Phase 3 完了検証 shall `cargo test` 全テストがパスすることを確認する
7. The Phase 3 完了検証 shall ウィンドウドラッグ移動が正常に動作すること（`pptDst=None` によるウィンドウ位置管理の独立性）を確認する

### Requirement 9: テスト・コード互換性確保

**Objective:** 開発者として、`WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` への切替に伴うテスト・example の互換性を確保したい。

_Parent: Req 10.1_

#### Acceptance Criteria

1. The `client_area_positioning_test.rs` shall `WS_EX_NOREDIRECTIONBITMAP` 参照を `WS_EX_LAYERED` に更新し、`AdjustWindowRectExForDpi` の結果変化に伴うテスト期待値を再検証・修正する
2. The wintf crate shall `WindowStyle::default()` の変更に伴い、デフォルトスタイルを使用する全 example が `WS_EX_LAYERED` で正しく動作することを確認する
3. The `win_style.rs` の `WS_EX_NOREDIRECTIONBITMAP` ビルダーメソッド shall 汎用 API として残存させる（削除しない）

#### 影響範囲（コード調査結果）

- `tests/client_area_positioning_test.rs` L14: 明示的に `WS_EX_NOREDIRECTIONBITMAP` を使用 — **更新必須・テスト値再検証**
- `examples/` 配下: taffy_flex_demo, typewriter_demo, multi_window_test, split_image はデフォルトスタイルを使用 — `WindowStyle::default()` 変更で自動追従
- `examples/dcomp_demo.rs` L48: 明示的に `WS_EX_NOREDIRECTIONBITMAP` を使用 — **Phase 4 で削除のため変更不要**

### Requirement 10: WS_EX_LAYERED 前提検証

**Objective:** 開発者として、Phase 3 実装に先立ち、`WS_EX_LAYERED` ウィンドウの OS 動作を検証し、設計上の不確実性を解消したい。

_Parent: Req 4.1, 4.2, 4.3_

#### Acceptance Criteria

1. The 前提検証 shall 最小構成の `WS_EX_LAYERED` ウィンドウを作成し、WM_PAINT の発火動作を確認する
2. The 前提検証 shall `pptDst=None` での `UpdateLayeredWindow` 呼び出し時にウィンドウ位置が維持されることを確認する
3. The 前提検証 shall alpha=0 ピクセル領域のクリックスルー動作を確認する
4. The 前提検証結果 shall `design.md` §5.2 の設計分岐（WM_PAINT 発火/不発火、pptDst 動作）に反映する
5. If `pptDst=None` でウィンドウ位置がリセットされた場合, the `present_layered_window` 関数 shall `GetWindowRect` で現在位置を取得して `pptDst` に渡す設計に変更する

#### 設計メモ

- WS_EX_LAYERED ウィンドウは一般に WM_PAINT を受信しないとされるが、OS バージョンやウィンドウ設定によって異なる可能性がある
- 本要件は既存の tasks.md Task 1（Phase 3A: 前提検証）に相当し、検証結果が Req 2, Req 4 の最終仕様を確定する
- WS_EX_LAYERED ウィンドウは初回 `UpdateLayeredWindow` 呼び出しまで内容が表示されない（OS 標準動作）。初回フレームの `ulw_present_system` で速やかに ULW が実行されることで解決される

---

## 要件トレーサビリティ（親仕様 → 子仕様）

| 親要件                        | 子仕様要件    | 概要                                        |
| ----------------------------- | ------------- | ------------------------------------------- |
| Req 4.1 (ULW 呼び出し)        | Req 1, Req 2  | ulw_present_system + present_layered_window |
| Req 4.2 (WS_EX_LAYERED)       | Req 3, Req 10 | スタイル切替 + OS 動作の前提検証            |
| Req 4.3 (クリックスルー)      | Req 7, Req 10 | alpha=0 透過（OS 標準動作 + 検証）          |
| Req 4.4 (commit→ULW 置換)     | Req 1         | CommitComposition ステージのシステム置換    |
| Req 4.5 (ULW 失敗リトライ)    | Req 6         | エラーハンドリング + 自動再試行             |
| Req 7.1 (WM_PAINT 更新)       | Req 4         | BeginPaint/EndPaint 最小ペア                |
| Req 7.2 (WM_SIZE)             | Req 5         | ECS リアクティブリサイズフロー              |
| Req 7.3 (BeginPaint 最小ペア) | Req 4         | WM_PAINT ハンドラの MSDN 準拠実装           |
| Req 10.1 (検証基準)           | Req 8, Req 9  | Phase 3 完了検証 + テスト互換性             |

# Research & Design Decisions: wintf-dpi-window-center-preserve

## Summary
- **Feature**: `wintf-dpi-window-center-preserve`
- **Discovery Scope**: Extension（既存DPIハンドリングシステムの拡張）
- **Key Findings**:
  - WM_WINDOWPOSCHANGED ハンドラ内で ECS tick 前に `WindowPos.position` を補正すれば、後続パイプライン変更なしに中心座標保持が実現可能
  - 補正に必要な全データ（旧物理サイズ、BoxStyle論理サイズ、新DPIスケール）がハンドラ内で取得可能であり、新規コンポーネントやTLS追加は不要
  - SetWindowPosGuard のカウンタ方式リファクタリングは要件レビュー中に先行実装済み

## Research Log

### WM_WINDOWPOSCHANGED ハンドラ内の補正挿入ポイント
- **Context**: 中心保持補正を既存フローのどこに挿入するのが最適か
- **Sources Consulted**: `crates/wintf/src/ecs/window_proc/handlers.rs` L140-300
- **Findings**:
  - DPI変更時（`dpi_context.is_some()`）は `use_bypass = false` となり、常に通常パス（L218-）に入る
  - 通常パス内で `window_pos.position = Some(client_pos)` を設定する直前（L225-226）が最適な挿入ポイント
  - この時点で以下のデータが利用可能:
    - `client_pos: POINT` — WINDOWPOS から変換済みクライアント座標（= suggested_rect 由来の位置）
    - `client_size: SIZE` — 旧物理サイズ（SWP_NOSIZE のため未変更）
    - `dpi: DPI` — 新 DPI（WM_DPICHANGED で更新済み）
    - `dpi_context: Option<DpiChangeContext>` — DPI変更コンテキスト
    - `entity_ref: EntityWorldMut` — BoxStyle 等の ECS コンポーネントにアクセス可能
- **Implications**: ハンドラ内のローカル変数 `client_pos` を補正済みの値に差し替えるだけで、後続コードへの影響ゼロ

### 旧物理サイズの取得方法
- **Context**: 中心保持補正には「旧物理サイズ」が必要。取得手段の選定
- **Sources Consulted**: Win32 WINDOWPOS ドキュメント、handlers.rs L170-180
- **Findings**:
  - **WINDOWPOS.cx/cy (→ client_size)**: SWP_NOSIZE で呼び出した場合、`cx/cy` は移動前の物理サイズが維持される。`window_to_client_coords` 経由で `client_size` として取得済み
  - **WindowPos.size コンポーネント**: tick 前であるためまだ旧値を保持。代替データソースとして使用可能
  - **GetWindowRect Win32 API**: 追加 Win32 API 呼び出しが必要で冗長
- **Implications**: `client_size`（既に計算済み）を旧物理サイズとして使用するのが最もシンプル。追加計算・API 呼び出し不要

### 新物理サイズの計算方法
- **Context**: 補正量算出のために「DPI変更後の新物理サイズ」を予測的に計算する必要がある
- **Sources Consulted**: `crates/wintf/src/ecs/layout/systems.rs` L419-503（window_pos_sync_system）
- **Findings**:
  - `BoxStyle.size` は `Option<BoxSize>` 型、`BoxSize { width: Option<Dimension>, height: Option<Dimension> }`
  - `Dimension::Px(f32)` から論理サイズ取得 → `* dpi.scale_x()` / `* dpi.scale_y()` → `.ceil() as i32` で物理サイズ
  - `window_pos_sync_system` も同様の ceiling 処理を使用：`width.ceil() as i32`, `height.ceil() as i32`
  - 計算式の一致を保証するため、`window_pos_sync_system` と同一の変換ロジックを使用すべき
- **Implications**: `BoxStyle.size` から新物理サイズを計算するロジックはヘルパー関数に切り出す価値がある（DRY原則）

### ECS パイプラインへの伝播検証
- **Context**: 補正済み `client_pos` が ECS パイプラインを通じて正しく最終位置に到達するか
- **Sources Consulted**: `systems.rs` L508-559（sync_window_arrangement_from_window_pos）、L419-503（window_pos_sync_system）
- **Findings**:
  - `WindowPos.position = corrected_pos` → `sync_window_arrangement_from_window_pos` → `Arrangement.offset = corrected_pos`
  - `update_arrangements_system`: Window エンティティの offset は taffy で上書きされない（外部入力として維持）
  - `propagate_global_arrangements`: `GA.bounds.left/top = corrected_pos`（LayoutRoot 直下のため変換は identity）
  - `window_pos_sync_system`: `WindowPos.position = GA.bounds.left/top = corrected_pos` ✓
  - `window_pos_sync_system`: `WindowPos.size = new_physical_size` ✓（Changed<DPI> により再計算）
  - `apply_window_pos_changes`: `SetWindowPosCommand` に corrected_pos + new_physical_size がセットされる ✓
- **Implications**: 補正は `WindowPos.position` の初期値を変更するだけで、パイプライン全体が自然に正しい値を伝播する

### WM_DPICHANGED 再発火防止の検証
- **Context**: 中心保持補正後の位置が元モニターに入らないことの確認
- **Sources Consulted**: gap-analysis.md の具体例、WM_DPICHANGED ドキュメント
- **Findings**:
  - **補正前**: center = suggested_pos + old_size/2 → サイズ縮小で中心が左上にずれ → 元モニター領域に入る可能性
  - **補正後**: center = corrected_pos + new_size/2 = suggested_pos + old_size/2 → 中心不変。Windows の suggested_rect が移動先モニター内を指す限り、中心を保持すれば移動先モニター内に留まる
  - 数学的証明: `corrected_pos + new_size/2 = (suggested_pos + (old_size - new_size)/2) + new_size/2 = suggested_pos + old_size/2` = 元の中心
- **Implications**: 中心保持により、Windows が suggested_rect で意図した位置関係（移動先モニター内の中心位置）が維持される。再発火リスクは排除される

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **pre-tick handler 補正** (採用) | WM_WINDOWPOSCHANGED 内で tick 前に client_pos を補正 | パイプライン変更なし、新コンポーネント不要、DRY | BoxStyle.size が None の場合のフォールバック必要 | 要件レビューでの議論を経て決定 |
| DpiChangeContext 拡張 + ECS コンポーネント (Option A) | 旧サイズを DpiCenterPreserve コンポーネントで伝達 | 明示的な消費パターン | 新 ECS コンポーネント追加、window_pos_sync_system 変更必要 | gap-analysis.md 当初推奨 |
| SWP_NOSIZE 除去 (Option B) | WM_DPICHANGED でサイズ+位置を同時設定 | 単一 SetWindowPos | レイアウト主導方式違反、二重サイズ算出 | 設計原則に反する |
| flush 時補正 (Option C) | apply_window_pos_changes で位置補正 | ECS コンポーネント不要 | TLS 寿命管理が複雑、Graphics 層にレイアウト関心事混入 | アーキテクチャ境界違反 |

## Design Decisions

### Decision: tick 前 WindowPos.position 補正方式の採用
- **Context**: DPI 変更時の中心座標ずれをどの処理段階で補正するか
- **Alternatives Considered**:
  1. Option A — DpiChangeContext 拡張 + DpiCenterPreserve ECS コンポーネント + window_pos_sync_system 内補正
  2. Option B — WM_DPICHANGED 内で SWP_NOSIZE を外してサイズ+位置同時設定
  3. Option C — flush_window_pos_commands 時点での位置補正
- **Selected Approach**: WM_WINDOWPOSCHANGED ハンドラ内、tick 前に `client_pos` を中心保持補正済みの値に差し替え
- **Rationale**: 
  - 既存パイプラインのシステム関数は一切変更不要
  - 新規 ECS コンポーネントや TLS を追加しない（最小侵入的）
  - BoxStyle.size がソースオブトゥルースという設計原則を完全に維持
  - 補正ロジックが単一箇所に集約され、可読性・テスタビリティが高い
- **Trade-offs**: 
  - ✅ パイプライン変更なし
  - ✅ 新規型定義なし
  - ⚠️ BoxStyle.size の読み出しがハンドラ側に1箇所追加（物理サイズ計算のため）
- **Follow-up**: BoxStyle.size が None の場合のフォールバック処理（補正スキップ）

### Decision: SetWindowPosGuard の AtomicI32 カウンタ方式化
- **Context**: guarded_set_window_pos のネスト管理の堅牢性向上
- **Selected Approach**: `Cell<bool>` TLS → `static SELF_INITIATED_DEPTH: AtomicI32` + RAII guard
- **Rationale**: ネストが発生した場合（DPI変更 → SetWindowPos → WM_WINDOWPOSCHANGED → tick → flush → SetWindowPos）でもカウンタが正しくデクリメントされる
- **Status**: 要件レビュー中に先行実装済み。ビルド・テスト通過確認済み

## Risks & Mitigations
- **BoxStyle.size が未設定の場合** — 補正をスキップし、旧動作（suggested_pos そのまま）にフォールバック。実用上 Window エンティティは常に BoxStyle.size を持つため、リスクは低い
- **ceiling 計算の不一致** — 新物理サイズ計算は window_pos_sync_system と同一ロジックを使用して一致を保証。ヘルパー関数化で DRY を維持
- **WS_POPUP + WS_EX_LAYERED 以外のウィンドウスタイル** — 本アプリケーションは全ウィンドウが WS_POPUP。将来的に異なるスタイルを使用する場合はフレームサイズの考慮が必要

## References
- `.kiro/specs/wintf-dpi-window-center-preserve/gap-analysis.md` — 実装アプローチオプション分析
- `.kiro/specs/completed/wintf-dpi-aware-layout/` — DPI-aware レイアウト基盤仕様
- [WM_DPICHANGED (Win32)](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged) — suggested_rect の仕様
- [SetWindowPos (Win32)](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos) — SWP_NOSIZE フラグの動作

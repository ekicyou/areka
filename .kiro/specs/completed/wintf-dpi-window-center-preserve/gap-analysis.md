# ギャップ分析: wintf-dpi-window-center-preserve

## 1. 現状調査

### 関連ファイル・モジュールマップ

| ファイル | 役割 | 変更見込み |
|----------|------|-----------|
| `crates/wintf/src/ecs/window.rs` | `DpiChangeContext`, `SetWindowPosGuard`, `guarded_set_window_pos`, `SetWindowPosCommand`, `DPI`, `WindowPos` | **変更対象**: DpiChangeContext に旧ウィンドウ情報追加 |
| `crates/wintf/src/ecs/window_proc/handlers.rs` | `WM_DPICHANGED`, `WM_WINDOWPOSCHANGED` ハンドラ | **変更対象**: WM_DPICHANGED で旧ウィンドウ矩形取得・保存 |
| `crates/wintf/src/ecs/layout/systems.rs` | `window_pos_sync_system`, `update_arrangements_system`, `sync_window_arrangement_from_window_pos` | **変更対象**: 中心座標保持補正ロジック挿入 |
| `crates/wintf/src/ecs/graphics/systems.rs` | `apply_window_pos_changes` | 変更なし（下流） |
| `crates/wintf/src/ecs/world.rs` | `try_tick_on_vsync`, スケジュール定義 | 変更なし（基盤） |

### 既存アーキテクチャパターン

- **レイアウト主導方式**: `BoxStyle.size`（論理px）が唯一のソースオブトゥルース。DPI変更時も不変。
- **DPI変更フロー**: `WM_DPICHANGED` → DPI直接更新 → `SWP_NOSIZE` で位置のみ → ECSパイプラインでサイズ算出 → `flush_window_pos_commands` で反映
- **TLSコンテキスト伝達**: `DpiChangeContext` が `WM_DPICHANGED` → `WM_WINDOWPOSCHANGED` 間の同期情報伝達に使用
- **echo/bypass パターン**: `IS_SELF_INITIATED` + `DpiChangeContext` による自己発火検出とフィードバックループ制御

### 現在のDPI変更時の問題分析

**現状のフロー**:
```
WM_DPICHANGED(new_dpi, suggested_rect)
  ① DPI component = new_dpi
  ② DpiChangeContext::set
  ③ guarded_set_window_pos(suggested_rect.left, .top, SWP_NOSIZE)  ← 位置のみ
     → WM_WINDOWPOSCHANGED → tick
       Layout: scale = new_dpi → 新しい物理サイズ算出
       window_pos_sync: WindowPos.size = new_physical_size
       apply_window_pos_changes → enqueue SetWindowPosCommand
     → flush: guarded_set_window_pos(new_size)  ← サイズ変更！ 但し位置補正なし
```

**問題**: `flush` での `SetWindowPosCommand` はサイズ変更を含むが、位置は `window_pos_sync_system` が `GlobalArrangement.bounds.left/top` からそのまま取得するため、中心座標が保持されない。

**具体例（200% → 125%）**:
```
論理サイズ: 400×300
旧物理サイズ: 800×600 (×2.0)
新物理サイズ: 500×375 (×1.25)

suggested_rect.left = 100, top = 200 とすると:
  ③ → 位置 (100, 200), サイズ旧のまま (800×600)
  flush → 位置 (100, 200), サイズ (500×375)

旧中心: (100 + 800/2, 200 + 600/2) = (500, 500)
新中心: (100 + 500/2, 200 + 375/2) = (350, 387)
→ 中心が (150, 113) ずれる！
```

中心が左上にずれることで、ウィンドウ中心が元のモニター領域に入り、再び `WM_DPICHANGED` が元のDPIで発火 → 無限ループ / 元の位置に戻る。

---

## 2. 要件の技術的実現可能性分析

### Requirement-to-Asset マップ

| 要件 | 技術的ニーズ | 既存アセット | ギャップ |
|------|-------------|-------------|---------|
| Req 1: 中心座標保持 | DPI変更前の物理サイズ/中心座標の取得・保存、サイズ変更時の位置補正計算 | `DpiChangeContext`（データ伝達基盤）、`window_pos_sync_system`（位置算出） | **Missing**: 旧物理サイズの取得・保存機構、中心保持位置補正ロジック |
| Req 2: 高→低DPIドラッグ | Req 1 の実現 + `WM_DPICHANGED` 再発火防止 | `DpiChangeContext` echo bypass メカニズム | **Missing**: 中心保持補正による再発火防止の検証 |
| Req 3: 低→高DPIドラッグ | Req 1 と同一ロジック | 同上 | 同上 |
| Req 4: ECSパイプライン統合 | 既存フローへの自然な挿入 | Layout/PostLayout/UISetup スケジュール、`DpiChangeContext` TLS | **Missing**: DPI変更情報のECS側への伝達手段 |
| Req 5: 無影響保証 | DPI変更フラグによる条件分岐 | `DpiChangeContext` の存在チェック | ギャップなし（条件分岐で対応可能） |
| Req 6: ログ出力 | `debug!`/`trace!` マクロ呼び出し | `tracing` 依存、既存のログパターン | ギャップなし |

### 複雑性シグナル

- **タイミング制約**: `WM_DPICHANGED` → 同期 `WM_WINDOWPOSCHANGED` → ECS tick → `flush` の厳密な順序内で中心補正を適用する必要がある
- **座標系混在**: Arrangement.offset = 物理px、BoxStyle.size = 論理px、GlobalArrangement.bounds = 物理px
- **再帰防止**: 位置補正後の `SetWindowPos` が新たな `WM_DPICHANGED` を引き起こさない保証

---

## 3. 実装アプローチオプション

### Option A: DpiChangeContext 拡張 + window_pos_sync_system 内補正

**概要**: `DpiChangeContext` に旧ウィンドウRECT/物理サイズを追加保存し、その情報をECSコンポーネント（新規マーカー `DpiCenterPreserve`）経由で `window_pos_sync_system` に伝達。サイズ変更時に位置を同時に補正する。

**変更箇所**:
1. `DpiChangeContext` に `old_window_rect: RECT` フィールド追加
2. `WM_DPICHANGED` ハンドラで `GetWindowRect` 呼び出し → `DpiChangeContext` に保存
3. `WM_WINDOWPOSCHANGED` ハンドラで `DpiChangeContext` から旧矩形を取得 → ECSコンポーネント `DpiCenterPreserve { old_physical_size: SIZE }` を挿入
4. `window_pos_sync_system` で `DpiCenterPreserve` 存在時に中心保持位置補正を適用し、コンポーネントを消費（remove）

**トレードオフ**:
- ✅ 既存パイプラインへの影響最小（`window_pos_sync_system` 内の条件分岐のみ）
- ✅ `DpiChangeContext` の拡張は自然（既に DPI 変更情報を伝達する責務を持つ）
- ✅ ECSコンポーネントによる1回限りの消費が明示的
- ❌ 新規ECSコンポーネント追加（マーカー1つ）
- ❌ `WM_WINDOWPOSCHANGED` ハンドラから ECS への書き込みが増加

### Option B: WM_DPICHANGED ハンドラ内で SWP_NOSIZE を外し位置+サイズを同時設定

**概要**: `WM_DPICHANGED` ハンドラ内で `SWP_NOSIZE` を外し、`suggested_rect` のサイズ（または ECS から算出した物理サイズ）と中心保持位置を同時に `SetWindowPos` で設定する。

**変更箇所**:
1. `WM_DPICHANGED` ハンドラで `GetWindowRect` → 旧中心算出
2. 新DPIから新物理サイズを計算（`BoxStyle.size × new_dpi_scale`）
3. 中心保持位置 = `old_center - new_size / 2`
4. `guarded_set_window_pos(adjusted_x, adjusted_y, new_width, new_height, 0)` ← SWP_NOSIZE なし

**トレードオフ**:
- ✅ 単一の `SetWindowPos` でサイズ+位置をアトミックに適用
- ✅ ECSパイプラインに変更不要
- ✅ 新規コンポーネント不要
- ❌ **レイアウト主導方式の原則に反する**: サイズ決定権がハンドラに移り、ECSパイプラインと二重のサイズ算出が発生
- ❌ `BoxStyle.size` → 物理サイズの変換をハンドラ側で再実装する必要がある（DRY違反リスク）
- ❌ `WM_WINDOWPOSCHANGED` での BoxStyle.size skip ロジックとの整合性が複雑化
- ❌ ECS tick でさらに `apply_window_pos_changes` が発火し、冗長な `SetWindowPos` が発生する可能性

### Option C: flush 時点での位置補正（ハイブリッド）

**概要**: `DpiChangeContext` に旧RECTを保存し、`apply_window_pos_changes` または `flush_window_pos_commands` の時点で、DPI変更起因のサイズ変更を検知して位置を補正する。

**変更箇所**:
1. `DpiChangeContext` に `old_window_rect` 追加
2. 旧RECT情報を別のTLS（`DpiCenterPreserveContext`）に転記
3. `apply_window_pos_changes` で `DpiCenterPreserveContext` が存在する場合、enqueue するコマンドの位置を中心保持で補正

**トレードオフ**:
- ✅ ECSコンポーネント追加不要
- ✅ レイアウトパイプラインのシステム自体は変更不要
- ❌ TLS の寿命管理が複雑（`DpiChangeContext` は WM_DPICHANGED → WM_WINDOWPOSCHANGED で消費、新TLSは tick 後まで生存が必要）
- ❌ `apply_window_pos_changes` は Graphics スケジュールに属し、レイアウト関心事が混入

---

## 4. 実装の複雑度とリスク

### 工数見積もり

**S (1〜3日)**: Option A / Option C いずれも、変更箇所が限定的で既存パターンの延長。

**根拠**:
- 変更ファイル数: 2〜3ファイル
- 新規ロジック: 中心座標計算（算術的に単純）
- テスト: 手動テスト（マルチDPIモニター環境でのドラッグ操作確認）
- 既存テストへの影響: なし（DPI 関連のユニットテストは存在しない）

### リスク評価

**Medium**

**根拠**:
- `WM_DPICHANGED` の再発火リスク: 位置補正後にウィンドウ中心が正しいモニター内に収まることの検証が必要
- タイミング依存: ECS tick のタイミング（VSYNC ゲート）と位置補正の適用タイミングの整合性
- Case A/B/C（WM イベント発火順序の不確定性）への影響分析が必要

---

## 5. 推奨事項

### 推奨アプローチ: Option A（DpiChangeContext 拡張 + window_pos_sync_system 補正）

**理由**:
1. レイアウト主導方式の原則（ECSパイプラインがサイズ・位置の算出元）を維持
2. `DpiChangeContext` の責務拡張として自然（DPI変更に関する追加情報の伝達）
3. ECSコンポーネントによる1回消費が明示的で、寿命管理が明確
4. `window_pos_sync_system` は既に `GlobalArrangement` → `WindowPos` の変換を担当しており、位置補正ロジックの追加が自然

### 設計フェーズでの調査事項

| # | 調査項目 | 理由 |
|---|----------|------|
| R1 | `GetWindowRect` のタイミング — `WM_DPICHANGED` ハンドラ内で旧DPIの物理矩形が取得できるか | DPI コンポーネント直接更新（①）の後に `GetWindowRect` を呼ぶと、Windows が新DPIでスケーリングした値を返す可能性 |
| R2 | `window_pos_sync_system` への情報伝達方式 — ECSコンポーネント vs TLS | ECSコンポーネントの追加・削除コストと、TLS寿命管理のトレードオフ |
| R3 | Case A/B/C でのイベント順序と中心補正の安全性 | `wintf-dpi-aware-layout` の設計文書で分析済みの3ケースに対して、中心補正追加時の影響 |
| R4 | `suggested_rect` と `GetWindowRect` の座標系 | `suggested_rect` はスクリーン座標（ウィンドウ全体）、`GetWindowRect` も同様 — WS_POPUP + WS_EX_LAYERED ではクライアント≒ウィンドウだが確認必要 |

### 設計フェーズへの推奨

- Option A を基本方針として設計を進める
- R1（GetWindowRect タイミング）の調査結果次第で、旧サイズの取得方法を調整
  - 代替案: `WM_DPICHANGED` 前の `WindowPos.size`（ECSコンポーネント）を使用（Win32 API 不要）
- Case A/B/C の安全性は `wintf-dpi-aware-layout` の分析をベースに増分で分析可能

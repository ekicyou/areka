# ギャップ分析: wintf-P0-click-through

## 1. 現況調査

### 関連ファイル・モジュール一覧

| ファイル | 責務 | 変更要否 |
|---------|------|---------|
| `crates/wintf/src/ecs/nchittest_cache.rs` | WM_NCHITTESTキャッシュ＋HTCLIENT/HTTRANSPARENT判定 | **主要変更対象** |
| `crates/wintf/src/ecs/window_proc/handlers.rs` | WM_NCHITTEST / WM_MOUSEMOVE / WM_MOUSELEAVE ハンドラ | 調査・必要に応じ変更 |
| `crates/wintf/src/ecs/window_proc/mod.rs` | メッセージディスパッチ | 変更不要 |
| `crates/wintf/src/ecs/layout/hit_test.rs` | ECSヒットテストAPI（hit_test_in_window 等） | 変更不要 |
| `crates/wintf/src/ecs/pointer/mod.rs` | PointerState / PointerLeave / WindowPointerTracking | 調査対象 |
| `crates/wintf/src/ecs/pointer/dispatch.rs` | ポインターイベントのTunnel/Bubbleディスパッチ | 変更不要 |
| `crates/wintf/src/ecs/widget/bitmap_source/systems.rs` | αマスク生成システム | 変更不要 |

### 既存アーキテクチャ概要

```
WM_NCHITTEST (ecs_wndproc)
  → handlers::WM_NCHITTEST
    → クライアント領域判定
    → cached_nchittest()
      → キャッシュヒット → LRESULT返却
      → キャッシュミス → ScreenToClient変換
        → hit_test_in_window() (World借用)
        → ★現在: 常にHTCLIENT返却（hit_result無視）
        → キャッシュ挿入
```

```
WM_MOUSEMOVE (ecs_wndproc)
  → handlers::WM_MOUSEMOVE
    → TrackMouseEvent(TME_LEAVE) 設定
    → hit_test_in_window() 実行
    → hit_entity あり → 旧エンティティLeave + 新エンティティPointerState挿入
    → hit_entity なし → ★Windowエンティティに PointerState 挿入
    → push_pointer_sample()
```

```
WM_MOUSELEAVE (ecs_wndproc)
  → handlers::WM_MOUSELEAVE
    → 当該ウィンドウの全PointerState削除 + PointerLeave付与
    → WindowPointerTracking 無効化
```

### 重要な発見事項

1. **`cached_nchittest` の分岐はすでに準備済み**: `HTTRANSPARENT` 定数は定義済み。`hit_result` 変数にヒットテスト結果が入っている。`#[allow(dead_code)]` と3行のコメントを変更するだけで核心ロジックは完成する。

2. **WM_MOUSEMOVE の「hit_test None」パス**: 現在、`hit_test_in_window()` が `None` を返す場合、**Windowエンティティ自体にPointerStateを挿入**している。HTTRANSPARENT有効化後は、このパスは `WM_MOUSEMOVE` 自体が受信されなくなるため到達しない。

3. **WM_MOUSELEAVEは既にマルチウィンドウ対応**: `find_owner_window` でウィンドウスコーピングし、他ウィンドウのPointerStateを保護している。

4. **TrackMouseEvent(TME_LEAVE)**: WM_MOUSEMOVEの初回受信時に設定される。HTTRANSPARENT有効化後、透明領域に移動するとWindowsがWM_MOUSELEAVEを発行する。これは既存のWM_MOUSELEAVEハンドラで正常に処理される。

5. **キャッシュはHTTRANSPARENT値を格納可能**: `LRESULT` 型なので `-1` も `1` も格納可能。既存テストに `LRESULT(-1)` のテストケースも存在する（`test_cache_multiple_hwnds`, `test_cache_update`）。

## 2. 要件別ギャップ分析

### 要件 1: ヒットテスト結果に基づくHTTRANSPARENT返却

| 受入基準 | 既存状態 | ギャップ |
|---------|---------|---------|
| AC1: None → HTTRANSPARENT | `hit_result` 取得済だが無視、常にHTCLIENT | **Missing**: 条件分岐の追加（3行変更） |
| AC2: Some → HTCLIENT | 現在常にHTCLIENT | なし（既存動作維持） |
| AC3: dead_code除去 | `#[allow(dead_code)]` 付き | **Missing**: アノテーション除去 |
| AC4: クライアント領域外 → DefWindowProcW | handlers.rs で実装済み | なし |

**ギャップ規模**: 極小。`nchittest_cache.rs` L142-L146 の3行コメント + 分岐を変更するのみ。

### 要件 2: ECSポインターイベントとの共存

| 受入基準 | 既存状態 | ギャップ |
|---------|---------|---------|
| AC1: HTTRANSPARENT領域移動時のPointerState除去 | WM_MOUSELEAVEハンドラが全PointerStateを除去 | **調査必要** |
| AC2: HTCLIENT領域再進入時のPointerState付与 | WM_MOUSEMOVEハンドラが処理 | なし（WindowsがWM_MOUSEMOVEを再送） |
| AC3: WM_MOUSELEAVEでのクリーンアップ | 実装済み（マルチウィンドウ対応） | なし |

**核心的な調査事項**:
HTTRANSPARENT返却後のWindows動作フロー:
1. マウスがHTCLIENT→HTTRANSPARENT領域に移動
2. WindowsがWM_NCHITTESTを発行 → HTTRANSPARENT返却
3. Windowsが**WM_MOUSEMOVEを発行しない**（透明扱い）
4. TrackMouseEvent(TME_LEAVE)が設定済みの場合、**WM_MOUSELEAVEが発行される**
5. 既存WM_MOUSELEAVEハンドラが全PointerStateをクリーンアップ

→ **既存動作で正常に機能する可能性が高い**。これが「既存コメントの問題」の核心。

**リスク**: HTTRANSPARENT領域とHTCLIENT領域の境界を高速にマウスが往復する場合のTrackMouseEvent再設定タイミング。WM_MOUSELEAVEの後にHTCLIENT領域に戻ると再びWM_MOUSEMOVEが発行され、TrackMouseEventが再設定される。

### 要件 3: キャッシュシステムの整合性維持

| 受入基準 | 既存状態 | ギャップ |
|---------|---------|---------|
| AC1: HTTRANSPARENT結果のキャッシュ格納 | LRESULT型なので格納可能 | なし（自動対応） |
| AC2: キャッシュからHTTRANSPARENT返却 | lookup関数はLRESULT値を比較しない | なし |
| AC3: tick完了時のキャッシュクリア | clear_nchittest_cache()が全クリアする | なし |

**ギャップ規模**: ゼロ。キャッシュシステムは `LRESULT` 型で値に依存しないため、HTTRANSPARENT値も透過的に処理される。

### 要件 4: 既存コメント問題の解決

| 受入基準 | 既存状態 | ギャップ |
|---------|---------|---------|
| AC1: 問題の調査・文書化 | コメントのみ、調査記録なし | **Missing**: 設計文書での分析記載 |
| AC2: HTCLIENT領域のマウスイベント正常受信 | 動作テストが必要 | **Unknown**: 実動作確認が必要 |
| AC3: 非互換性の明示 | なし | **Missing**: 設計文書での制約記載 |

**既存コメントの問題分析（予備的推論）**:

「HTTRANSPARENT を返すとマウスイベントがブロックされてしまう」の原因として考えられるシナリオ:

1. **WM_MOUSEMOVE未受信問題**: HTTRANSPARENT返却時にWM_MOUSEMOVEが送信されないため、当時のコードでポインター管理に不整合が発生した可能性
2. **ドラッグ操作の中断**: WM_MOUSEMOVEに依存するドラッグ処理が、透明領域でのHTTRANSPARENT返却により中断する問題
3. **TrackMouseEvent未対応時代**: WM_MOUSELEAVEが未実装だった時期に、PointerStateが残り続けた可能性

→ 現在のコードは WM_MOUSELEAVE / TrackMouseEvent / マルチウィンドウ対応が完了しているため、**当時のブロッキング要因は解消されている可能性が高い**。

### 要件 5: HitTestModeとの連携整合性

| 受入基準 | 既存状態 | ギャップ |
|---------|---------|---------|
| AC1: 全None → 全域HTTRANSPARENT | hit_test_in_window()がNone返却 | なし（要件1で自動対応） |
| AC2: AlphaMask透明ピクセル → HTTRANSPARENT | hit_test_entityがMiss返却 | なし（要件1で自動対応） |
| AC3: Bounds内 → HTCLIENT | hit_test_entityがHit返却 | なし（既存動作） |
| AC4: NamedRegionsヒット → HTCLIENT | hit_test_entity_exがHit返却 | なし（既存動作） |

**ギャップ規模**: ゼロ。ヒットテストシステムが既にHitTestModeを考慮した結果を返しているため、`cached_nchittest`での分岐変更だけで全モードに対応する。

### 要件 6: テスト検証可能性

| 受入基準 | 既存状態 | ギャップ |
|---------|---------|---------|
| AC1: HTTRANSPARENT返却パスのテスト | cached_nchittestのテストなし（単体テストはキャッシュ操作のみ） | **Missing** |
| AC2: 両結果のキャッシュ格納テスト | LRESULT(-1)のキャッシュテストあり | 部分的に既存 |
| AC3: None条件ごとのテスト | hit_test_in_windowのテスト既存（限定的） | **Missing**: 追加テストケース |

**テスト戦略の課題**:
- `cached_nchittest` 関数は `HWND`, `Rc<RefCell<EcsWorld>>` を要するため、結合テストが必要
- キャッシュの低レベルAPI (`lookup`, `insert`) のテストは既存で十分
- `hit_test_in_window` の None 条件テストは `hit_test.rs` 内に追加可能

## 3. 実装アプローチ選択肢

### Option A: 最小変更（既存コンポーネント拡張）

**変更対象**: `nchittest_cache.rs` の `cached_nchittest()` 関数のみ（3行変更）

**変更内容**:
```rust
// Before:
let lresult = LRESULT(HTCLIENT as isize);

// After:
let lresult = match hit_result {
    Some(_) => LRESULT(HTCLIENT as isize),
    None => LRESULT(HTTRANSPARENT as isize),
};
```

**トレードオフ**:
- ✅ 変更量が極小（低リスク）
- ✅ 既存パターンへの完全適合
- ✅ キャッシュシステムは変更不要
- ✅ WM_MOUSELEAVEハンドラで PointerState クリーンアップが自動対応
- ❌ ドラッグ中のHTTRANSPARENT問題が発生する可能性（調査必要）
- ❌ 「空白領域にWindowエンティティへPointerState付与」の既存コードパスが到達不能になる

### Option B: ドラッグ対応付き実装

**変更対象**: `nchittest_cache.rs` + `handlers.rs`（WM_MOUSEMOVE内のドラッグ処理）

**追加変更**:
- ドラッグ状態（Preparing / Dragging）中は `HTTRANSPARENT` を返さず `HTCLIENT` を返す
- ドラッグ中にWM_MOUSEMOVEが途切れると、ドラッグ操作が不自然に中断するため

**トレードオフ**:
- ✅ ドラッグ操作の安全性を保証
- ✅ User体験の一貫性
- ❌ `cached_nchittest` がドラッグ状態を参照する必要がある（結合度の増加）
- ❌ thread_local DrageState への追加依存

### Option C: WM_MOUSEMOVEフォールバック付きハイブリッド

**変更対象**: `nchittest_cache.rs` + `handlers.rs`（WM_MOUSEMOVE の None ブランチ）

**追加変更**:
- WM_MOUSEMOVEの `hit_entity = None` 時に、Windowエンティティへの PointerState 付与を **削除** して、代わりに全PointerStateをLeave処理
- これにより、HTTRANSPARENT有効化後の「到達不能コード」を整理

**トレードオフ**:
- ✅ コードパスの整合性が向上
- ✅ 将来の保守性向上
- ❌ WM_MOUSEMOVEの既存動作を変更するため、副作用リスクがOption Aより高い

## 4. 複雑度・リスク評価

### 工数: **S**（1〜3日）

理由: 核心の変更は `cached_nchittest` 内の3行分岐のみ。ヒットテストシステム、キャッシュシステム、WM_MOUSELEAVEハンドラは全て既存で要件を満たしている。テスト追加と調査文書が主な作業。

### リスク: **Medium**

理由:
- **Low要因**: 変更量が極小、既存パターンへの適合度が高い
- **Medium要因**: 「既存コメント問題」の実動作検証が必要。ドラッグ操作とHTTRANSPARENTの相互作用が未検証。WindowsのWM_NCHITTEST→WM_MOUSELEAVE遷移のタイミング特性が文書化されていない。

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: **Option A を基本とし、Option B の要素を検討**

1. まず Option A（最小変更）を実装し、動作確認
2. ドラッグ操作との干渉を実機テストで確認
3. 干渉がある場合のみ Option B の対策を追加

### 設計フェーズでの調査事項（Research Needed）

1. **HTTRANSPARENT返却後のWM_MOUSELEAVEのタイミング検証**: TrackMouseEvent(TME_LEAVE)設定済みウィンドウでHTTRANSPARENTを返した場合のメッセージシーケンスを実機確認
2. **ドラッグ中のHTTRANSPARENT影響**: DragState::Dragging 中にHTTRANSPARENTを返した場合のWM_MOUSEMOVEの断絶
3. **SetCapture との関係**: ドラッグ中に SetCapture を使用している場合、HTTRANSPARENT の影響を受けない可能性がある（Win32仕様確認）
4. **WM_MOUSEMOVEのNoneブランチの扱い**: 到達不能コードの整理方針（削除 or 防衛的に残す）

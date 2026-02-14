# Dola Runtime Engine — ギャップ分析レポート

| 項目 | 内容 |
|------|------|
| **対象仕様** | dola-runtime-engine |
| **要件バージョン** | v1.0 |
| **分析日** | 2026-02-14 |
| **分析種別** | ブラウンフィールド拡張（既存 dola クレート基盤） |

---

## 1. 分析サマリー

- **強力な基盤が既存**: `crates/dola/` に宣言的データモデル（`DolaDocument`）、フルコンパイラ（`compile_storyboard`）、バリデーション（13ルール）、ビルダーAPI が実装完了済み（`dola-animation-system` 仕様完了）。ランタイムが消費する `CompiledStoryboard` / `CompiledSegment` は設計済み
- **ランタイム層は完全に未実装**: 再生エンジン・タイムテーブル・購読管理・競合解決・補間計算・イージング評価・時刻ユーティリティのすべてが新規実装。`PlaybackState` / `ScheduleRequest` は型定義のみでロジックなし
- **イージング評価は `interpolation` クレートで解決**: `EasingName` 31バリアントは `interpolation::EaseFunction` 30バリアント + `Linear`（自明実装）と完全一致。`impl Ease for f64` により f64 ネイティブ対応。`ParametricEasing` は同クレートの `quad_bez` / `cub_bez` でカバー。外部クレート選定は完了
- **既存 WAM COM ラッパーとの関係整理が必要**: `com/animation.rs` に `IUIAnimationTimer::get_time()` が存在し、これが時刻ユーティリティ（Req 11）の候補だが、COM 依存を dola クレートに持ち込むかは設計判断
- **新規外部依存**: イージング評価は `interpolation` (0.3.0) に確定。時刻取得クレートの選定は設計フェーズへ

---

## 2. 既存コードベース調査

### 2.1 dola クレート現状（dola-animation-system 完了済み）

| コンポーネント | 状態 | ランタイムでの役割 |
|---|---|---|
| `DolaDocument` | ✅ 完成 | 指示書パース結果の保持コンテナ |
| `compile_storyboard()` | ✅ 完成（753行） | Start コマンド時にストーリーボードをコンパイル |
| `CompiledStoryboard` | ✅ 完成 | ランタイム消費用データ構造（変数→タイムライン→セグメント）|
| `CompiledSegment` | ✅ 完成 | 個別トランジション（start/end_time, from/to_value, easing） |
| `InterruptionPolicy` | ✅ 完成 | 競合解決戦略の識別（Cancel/Conclude/Trim/Compress/Never） |
| `PlaybackState` | ⚠️ 型のみ | 状態enum存在（Idle/Playing/Paused/Completed/Cancelled）、遷移ロジックなし |
| `ScheduleRequest` | ⚠️ 型のみ | storyboard名 + start_time のみ |
| `EasingFunction` / `EasingName` | ⚠️ 型のみ | 31バリアント定義済み、数学的評価なし |
| `Validate` trait | ✅ 完成 | 13ルール、コンパイル前に自動実行 |
| Builder API | ✅ 完成 | テスト・外部利用のドキュメント構築 |

### 2.2 コンパイラ出力とランタイム消費の関係

`CompiledStoryboard` はランタイム消費を前提に設計されている：

1. `timelines: BTreeMap<String, CompiledVariableTimeline>` — 変数名で直接ルックアップ
2. セグメントは絶対時刻・ソート済み → バイナリサーチで現在アクティブセグメントを特定可能
3. `time_scale` は事前適用されない → ランタイムが `(current_time - start_time) / time_scale` で正規化
4. `from_value` / `to_value` / `easing` が全展開済み → ランタイムは進捗率 `t` を計算して補間するだけ
5. `VariableTypeHint` で Float / Integer（丸め） / Object（即時）の分岐が明確

### 2.3 InterruptionPolicy の差異（要件定義 vs 既存定義）

| 要件定義（Req 7） | 既存 `InterruptionPolicy` enum |
|---|---|
| **Abandon**（デフォルト） | Cancel |
| **Conclude** | Conclude（デフォルト） |
| **Compress** | Compress |
| — | Trim |
| — | Never |

**ギャップ**: 
- 要件のデフォルトは **Abandon** だが、既存 enum のデフォルトは **Conclude**
- 要件の「Abandon」は既存の「Cancel」に対応するが名称が異なる
- 既存に `Trim` / `Never` があるが要件には未記載
- → **設計フェーズで命名・デフォルト値・バリアント構成を整理する必要あり**

### 2.4 PlaybackState の差異

| 要件定義（Req 8） | 既存 `PlaybackState` enum |
|---|---|
| Created | — （なし） |
| Playing | Playing |
| Paused | Paused |
| Concluded | Completed |
| Abandoned | Cancelled |
| Compressed | — （なし） |
| — | Idle |

**ギャップ**: 要件の状態遷移モデルが既存 enum と一致しない。特に Created / Concluded / Abandoned / Compressed の区別が不足。
- → **設計フェーズで新しい状態enum を定義するか、既存を拡張するか決定が必要**

### 2.5 wintf 側の統合ポイント

| wintf コンポーネント | ランタイムとの関係 |
|---|---|
| `com/animation.rs` — `IUIAnimationTimer::get_time()` | OS起動からのf64秒を取得。Req 11 の候補だが COM (Windows) 依存 |
| `com/animation.rs` — WAM Manager/Storyboard | dola ランタイムが WAM を**置換**する層。並行共存は不要 |
| ECS コンポーネント | アニメーション固有 ECS コンポーネントは未存在。wintf-P0-animation-system が今後ここに構築予定 |

### 2.6 既存依存関係

dola クレートの現在の依存:
- `serde` (1, derive) — 必須
- `serde_json` (1, optional) — feature `json`
- `toml` (0.8, optional) — feature `toml`
- `serde_yaml` (0.9, optional) — feature `yaml`

ランタイムに必要な追加依存: **`interpolation` (0.3.0)** 確定 + **時刻取得**（設計フェーズで選定）

---

## 3. 要件対アセットマップ

| 要件 | 既存アセット | ギャップ |
|------|-------------|---------|
| **Req 1: 指示書受信** | `DolaDocument::from_toml()` (serde) | **Missing**: ランタイム状態管理（変数値の保持・引き継ぎ、指示書上書きロジック） |
| **Req 2: Start コマンド** | `compile_storyboard()` 完成済み | **Missing**: `group_id` 採番器、コンパイル結果のタイムテーブルへの投入 |
| **Req 3: 制御コマンド** | `PlaybackState` enum（型のみ） | **Missing**: Pause/Resume/Conclude/Abandon/Finish の全ロジック、時間オフセット管理 |
| **Req 4: 購読管理** | なし | **Missing**: Subscribe/Unsubscribe 機構、Drop 自動解除、購読者別評価フィルタ |
| **Req 5: Update** | なし | **Missing**: 補間エンジン、差分検出、前回値キャッシュ |
| **Req 6: タイムテーブル** | `CompiledVariableTimeline` / `CompiledSegment` | **Missing**: 変数ごとのランタイムタイムテーブル（複数 group_id 共存、GC） |
| **Req 7: 競合検出** | `InterruptionPolicy` enum | **Missing**: 競合検出ロジック、group_id 単位一括適用、3戦略の実装。**Unknown**: 既存 enum との命名差異の解決 |
| **Req 8: 状態遷移** | `PlaybackState` enum（不一致あり） | **Missing**: 状態遷移マシン、インスタンスごとのライフサイクル管理。**Constraint**: 既存 enum との互換性 |
| **Req 9: 同時再生** | なし | **Missing**: 複数実行インスタンスの並行管理。低リスク（タイムテーブル設計で自然に対応） |
| **Req 10: イージング** | `EasingFunction` / `EasingName` 型定義 | **Missing**: 評価ロジックの実装。**Resolved**: `interpolation` (0.3.0) 採用確定。`Ease` trait (`impl Ease for f64`) + `quad_bez`/`cub_bez` |
| **Req 11: 時刻ユーティリティ** | `com/animation.rs` の `IUIAnimationTimer` | **Missing**: dola クレート内のプラットフォーム非依存 API。**Research Needed**: `quanta` / `std::time::Instant` / Win32 QPC の選定 |

---

## 4. 外部依存調査

### 4.1 イージング評価クレート

| クレート | バージョン | DL数 | 特徴 | 適合度 |
|---|---|---|---|---|
| `interpolation` | 0.3.0 | 895K | `EaseFunction` enum (30バリアント) + `Ease` trait (`impl Ease for f64`) + `lerp` / `quad_bez` / `cub_bez` | ✅ **採用** |
| `simple-easing` | 1.0.1 | 578K | 30種のイージング関数、`fn(f32)->f32`、ゼロ依存 | ✖ 不採用（f32型不一致） |
| `easing` | 0.0.5 | 9.3K | 古い（9年前）、イテレータベース | ✖ 不採用（メンテ停止） |

**確定事項**:
- `interpolation` クレートに確定。dola の `EasingName` は `interpolation::EaseFunction` 準拠で設計済み（ソースコメントに明記）
- `impl Ease for f64` により f64 ネイティブ対応。`simple-easing` の f32/f64 キャスト問題は発生しない
- `quad_bez` / `cub_bez` 関数が `ParametricEasing::QuadraticBezier` / `CubicBezier` に直接対応
- CSS `cubic-bezier(x1,y1,x2,y2)` 形式と `cub_bez(x0,x1,x2,x3,t)` 形式のマッピングは設計フェーズで確定する

### 4.2 時刻取得クレート

| 方式 | 特徴 | 対応OS | OS起動時起点 |
|---|---|---|---|
| `quanta` (0.12.6) | TSC+フォールバック、90M DL、モック対応 | Win/Linux/macOS | ⚠️ Instant相当（起動時起点ではない） |
| `std::time::Instant` | stdlib、単調時計 | 全OS | ⚠️ Instant相当（相対差分のみ、起動時起点ではない） |
| Win32 `QueryPerformanceCounter` | Windows API 直接 | Windows のみ | ⚠️ 起動時起点ではない（差分用） |
| Win32 `GetTickCount64` | Windows API | Windows のみ | ✅ OS起動時からのmsec |
| `IUIAnimationTimer::GetTime()` | WAM COM API | Windows のみ | ✅ OS起動時からのf64秒（要件仕様に最も合致） |

**所見**:
- 要件は「OS起動時からの秒数（f64）」を明示。`IUIAnimationTimer::GetTime()` が最も直接的だが COM 依存
- dola を「プラットフォーム非依存」に保つなら、時刻ユーティリティを wintf 側に配置する選択肢もある
- `GetTickCount64` → f64秒変換は軽量だが msec 精度。QPC は高精度だが起動時起点の計算が必要
- **Research Needed**: dola のプラットフォーム非依存性と時刻ユーティリティの配置先は設計判断

---

## 5. 実装アプローチ評価

### Option A: 既存 dola クレート内に全ランタイムを追加

**方針**: `crates/dola/src/` に `runtime.rs`, `timetable.rs`, `subscription.rs`, `interpolate.rs`, `clock.rs` 等を追加。

**メリット**:
- ✅ 既存型（`CompiledStoryboard`, `InterruptionPolicy` 等）への直接アクセス
- ✅ 単一クレートで完結、依存管理がシンプル
- ✅ コンパイラとランタイムの一体テストが容易

**デメリット**:
- ❌ dola が「宣言的データモデル」から「ランタイム付き実行基盤」に責務拡大
- ❌ プラットフォーム依存（時刻取得）が混入する可能性
- ❌ 依存クレート増加（イージング評価、時刻取得）

**適合条件**: ランタイムを dola クレート本体に含めて問題ない場合

### Option B: 別クレート `dola-runtime` を新設

**方針**: `crates/dola-runtime/` を新設し、`dola` に依存する形でランタイム層を実装。

**メリット**:
- ✅ 責務分離明確（dola = データモデル+コンパイラ、dola-runtime = 実行エンジン）
- ✅ dola のプラットフォーム非依存性を維持
- ✅ runtime のみの依存追加で dola 本体に影響なし

**デメリット**:
- ❌ クレート間の型共有に pub API 境界の設計が必要
- ❌ `PlaybackState` や `InterruptionPolicy` の拡張時にクレート間の調整が発生
- ❌ ワークスペースに新クレートが増加

**適合条件**: プラットフォーム非依存性を重視し、将来の別プラットフォーム展開を見据える場合

### Option C: ハイブリッド（推奨候補）

**方針**: コアロジック（補間・タイムテーブル・競合検出・購読管理）は dola クレート内に実装し、プラットフォーム依存（時刻取得）は feature gate または wintf 側に配置。

**メリット**:
- ✅ ランタイムコアが既存型と同一クレートで密結合テスト可能
- ✅ 時刻取得のみプラットフォーム依存を分離
- ✅ `PlaybackState` / `InterruptionPolicy` の拡張が自然

**デメリット**:
- ❌ feature gate の設計が必要
- ❌ イージング評価クレートの依存は dola 本体に入る

**フェーズ分け**:
1. **Phase 1**: 補間エンジン + イージング評価 + タイムテーブル + 状態管理（dola クレート内）
2. **Phase 2**: 購読管理 + 差分検出 + 競合解決（dola クレート内）
3. **Phase 3**: 時刻ユーティリティ（feature gate `windows` or wintf 側）

---

## 6. 実装複雑度 & リスク

| 項目 | 工数 | リスク | 根拠 |
|------|------|--------|------|
| **全体** | **L（1-2週間）** | **Medium** | 新規ロジック多数だが、既存コンパイラ出力は消費用に設計済み。アルゴリズム的な複雑さ（競合解決、タイムテーブル管理）がある |
| 補間エンジン | S | Low | セグメントの `t` 計算 + 線形補間 + イージング適用。VariableTypeHint で分岐明確 |
| イージング評価 | S | Low | `interpolation` クレート採用確定。`Ease` trait + `quad_bez`/`cub_bez` で全カバー |
| タイムテーブル管理 | M | Medium | 複数 group_id の共存、セグメント挿入・破棄、時間オフセット管理 |
| 競合検出 & 終了戦略 | M | Medium | 同一変数の重複検出 + group_id 単位一括適用。3 戦略の正確な実装が必要 |
| 購読管理 | S | Low | HashMap + Drop トレイトの標準パターン |
| 差分検出（Update） | S | Low | 前回値キャッシュとの比較、Vec 構築 |
| 状態遷移 | S | Low | enum + match による状態マシン。遷移ルールは要件で明確 |
| 時刻ユーティリティ | S | Low | GetTickCount64 or QPC（Windows確定環境） |
| 既存型との整合 | S | Medium | `InterruptionPolicy` / `PlaybackState` の拡張 or 再定義が必要 |

---

## 7. 設計フェーズへの持ち越し事項

### 設計判断が必要な項目

1. **クレート構成**: dola 内実装 vs 別クレート `dola-runtime` vs ハイブリッド
2. **InterruptionPolicy の整理**: 既存 enum（Cancel/Conclude/Trim/Compress/Never）と要件（Abandon/Conclude/Compress）の統合方針
3. **PlaybackState の整理**: 既存 enum と要件の状態遷移モデル（Created/Concluded/Abandoned/Compressed）の統合
4. **時刻ユーティリティの配置先**: dola 内（feature gate `windows`）vs wintf 側 vs 引数で受け取る設計
5. **購読者の所有権モデル**: Arc<Mutex> 共有 vs チャネル vs 直接所有

### 解決済みの調査事項

- ✅ イージングクレート選定: `interpolation` (0.3.0) に確定
- ✅ f32/f64 型不一致: `interpolation` は `impl Ease for f64` で問題なし
- ✅ ParametricEasing カバレージ: `quad_bez` / `cub_bez` で対応

### 未解決の調査事項

- CSS `cubic-bezier(x1, y1, x2, y2)` と `cub_bez(x0, x1, x2, x3, t)` のパラメータマッピング詳細
- 複数購読者が同時に Update を呼ぶ場合のスレッドセーフ要件（シングルスレッド前提？）

---

## 8. 推奨事項

**推奨アプローチ: Option C（ハイブリッド）**

dola クレート内にランタイムコアを実装しつつ、プラットフォーム依存を分離する方式を推奨。理由：

1. `CompiledStoryboard` / `InterruptionPolicy` 等の既存型との密結合が自然
2. dola-animation-system（完了済み）の設計思想を継承
3. テストが単一クレート内で完結し、テストの整合性を維持しやすい
4. 時刻取得のみを外部化すれば、プラットフォーム非依存性の実質的な犠牲はない
5. `interpolation` クレートの採用が確定し、イージング評価の実装リスクが解消

**実装ゴール**: 子仕様フェーズ分割方式。本仕様の設計フェーズで全体設計・子仕様計画を策定し、実装フェーズで子仕様を順次立ち上げる。

**実装ゴール**: 子仕様フェーズ分割方式。本仕様の設計フェーズで全体設計・子仕様計画を策定し、実装フェーズで子仕様を順次立ち上げる。

**次のステップ**:
```
/kiro-spec-design dola-runtime-engine -y
```

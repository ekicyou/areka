# areka ロードマップ — ukadoc互換ベースウェア

| 項目 | 内容 |
|------|------|
| **Document Title** | areka ロードマップ（互換ベースウェア戦略） |
| **Version** | 2.0 |
| **Date** | 2026-06-26 |
| **ゴール** | ① ukadoc準拠の互換ベースウェア確立（既存伺かゴーストが動く）→ ② ぱすたさん（native旗艦ゴースト） |

> **v2.0 戦略転換 (2026-06-26)**: 「ぱすたさん専用の試作」から **ukadoc準拠の互換ベースウェア（SSP代替）** へ
> 狙いを定め直す。**互換ベースウェアを先行**、ぱすたさんは互換土台の上のnative旗艦として後続。
> 確定した設計判断は [COMPAT_ARCHITECTURE.md](COMPAT_ARCHITECTURE.md)（議事の正本）を参照。
> v1.x のボトムアップ表示層計画は本トラック体系へ再マップした（破棄ゼロ）。

---

## 戦略：二枚看板・互換先行

- **二枚看板**: ①互換ベースウェア（既存伺か資産を動かす）／②ぱすたさん（native旗艦）
- **互換先行の理由**: 既存ゴーストが実際に動く達成感がモチベーションを最大化し、最難関の互換部を前倒しで潰してリスクを早期に溶かす。
- **互換契約**: ukadoc正典。SERIKO/MAYUNA完全マップ／さくらスクリプト優先度順／沈黙時はareka裁量＋対応表記録（詳細は [COMPAT_ARCHITECTURE.md](COMPAT_ARCHITECTURE.md) §2）。

### 北極星（縦スライス）
- **M1（互換ベースウェア）**: 実在の里々ベースゴースト1体が、SAORI込みで実際に表示・会話する。
- **M2（ぱすたさん）**: 同じ土台の上で、ぱすたさん（native脳pasta＋階層サーフェスの本領）が動く。

---

## 解決済み基盤資産（伺か土台の最難関は完了済み）

| 基盤 | 到達点 | 関連完了仕様 (`completed/`) |
|------|--------|---------------------------|
| 透過/レイヤードウィンドウ | DComp→ULW移行完遂、ULW既定 | `wintf-dcomp-migration-0`〜`4`, `wintf-dcomp-to-layered-migration` |
| DComp⇄ULW切替基盤 | ウィンドウ単位 `CompositionMode` | `wintf-dcomp-migration-4-switchable-backend` |
| クリック透過（別プロセスへ） | `ULW_ALPHA`/alpha=0で自動透過 | `wintf-P0-click-through`, `event-hit-test-alpha-mask` |
| イベント/ヒットテスト | mouse/drag/routing/named-regions/multiwindow | `wintf-P0-event-system` ＋配下8仕様 |
| dola演出ランタイム | コア/クロック/ファサード/競合/ループ/nested | `dola-runtime-1`〜`5`, `dola-nested-storyboard` ほか |

> 却下: `_rejected/wintf-P0-click-through-rgn`（`SetWindowRgn`はDComp描画をクリップし両立不可）。

---

## トラック体系（M1: 互換ベースウェア先行）

### T1 — 階層サーフェス／アニメーションエンジン
SERIKOを平坦サブセットとして内包する上位エンジン。native-firstで建て、SERIKOローダは後付け。

| 仕様 | .kiro/specs/ | 状態 | 備考 |
|------|-------------|:----:|------|
| dola→wintf バインディング＋階層合成 | `wintf-P0-animation-system` | 🔵 要件生成済 | T1の心臓。スコープに階層サーフェス＋再生制御プリミティブを追加 |
| SERIKOランタイム（完全マップ） | *(新規)* | ⚪ | pattern/interval/method 全集合。talk/mouse/bindトリガをwintfイベントへ結線 |
| 階層参照拡張（areka-native） | *(新規)* | ⚪ | エレメント→別サーフェス定義参照。循環検出・多重インスタンス。典拠=areka |
| シェルパッケージローダ | *(新規)* | ⚪ | descript.txt/surfaces.txt/surface*.png/collision。collisionをhit-testへ |

### T2 — さくらスクリプト互換＋バルーン
| 仕様 | .kiro/specs/ | 状態 | 備考 |
|------|-------------|:----:|------|
| バルーン（親→子分割） | `wintf-P0-balloon-system` ＋ `balloon01`〜`06` | 🔵 設計承認済/要件 | **さくらスクリプト駆動＋balloonパッケージ読込前提**へ再スコープ |
| さくらスクリプトrunner | *(新規)* | ⚪ | ukadoc優先タグから。`\s[]`→サーフェス、テキスト/`\n`/`\w`/`\e`/選択肢→balloon |
| balloonパッケージローダ | *(新規)* | ⚪ | balloon descript.txt/位置決め |

### T3 — SHIORI ホスト
| 仕様 | .kiro/specs/ | 状態 | 備考 |
|------|-------------|:----:|------|
| `IShiori` COM ＋ ネイティブin-proc | *(新規)* | ⚪ | 内部唯一ABI。HSTRING/UTF-16。push=`IShioriHost`sink |
| 32bit Rustホスト（過去互換） | *(新規)* `areka-shiori-host` | ⚪ | i686随伴バイナリ。flat-C/HGLOBAL/charset/SAORI同居/自前ループ/毎秒poll。自前IPC |

### T4 — 統合（M1達成）
| 仕様 | .kiro/specs/ | 状態 | 備考 |
|------|-------------|:----:|------|
| 互換ゴースト統合 | *(新規)* | ⚪ | 実在里々ゴースト1体をE2E起動（shell＋balloon＋SHIORI＋SAORI） |
| areka バイナリ拡充 | `crates/areka/`（試作済） | 🔵 | 2ウィンドウ試作の上にベースウェア機能を積む |

---

## M2 — ぱすたさん（native旗艦・互換後続）

| 仕様 | .kiro/specs/ | 状態 | 備考 |
|------|-------------|:----:|------|
| リファレンスシェル | `areka-P0-reference-shell` | ⚪ | 階層サーフェスの本領をnativeで活用 |
| リファレンスバルーン | `areka-P0-reference-balloon` | ⚪ | |
| リファレンスゴースト | `areka-P0-reference-ghost` | ⚪ | pasta脳が `IShiori` をnative実装 |
| pasta スクリプトエンジン | `completed/areka-P0-script-engine` | ✅ 完了 | vendored `vendors/pasta/` |

---

## アプリ統合・出荷（旧Phase D/E）

| 仕様 | .kiro/specs/ | 状態 |
|------|-------------|:----:|
| システムトレイ / 永続化 / パッケージマネージャ / MCPサーバー | `areka-P0-system-tray`/`-persistence`/`-package-manager`/`-mcp-server` | ⚪ 要件 |
| 統合テスト / ドキュメント / リリースビルド | *(新規予定)* | ⚪ |

---

## 依存関係図

```mermaid
graph LR
    subgraph Done["✅ 解決済み基盤"]
        ULW[透過/ULW/click-through]
        EVT[event/hit-test/alpha-mask]
        DOLA[dola runtime/nested]
    end
    subgraph M1["M1: 互換ベースウェア（先行）"]
        T1[T1 階層サーフェスEngine＋SERIKO]
        T2[T2 さくらScript＋balloon]
        T3[T3 SHIORIホスト/IShiori＋32bit]
        T4[T4 互換ゴースト統合]
    end
    subgraph M2["M2: ぱすたさん（後続）"]
        GHOST[reference-ghost/pasta native]
    end
    ULW --> T1
    EVT --> T1
    DOLA --> T1
    T1 --> T2
    T1 --> T4
    T2 --> T4
    T3 --> T4
    T4 --> GHOST
```

**クリティカルパス（M1）**: animation-system＋階層Engine → SERIKO＋シェルローダ → さくらScript＋balloon → SHIORIホスト(里々) → 互換ゴースト統合

---

## 仕様ポートフォリオ実数（2026-06-26 棚卸し）

| 配置 | 件数 |
|------|:----:|
| `completed/` | 82 |
| `.kiro/specs/` 直下（アクティブP0） | 19 |
| `backlog/`（待機P1-P3） | 20 |
| `_rejected/` | 1 |
| `spec.json`未生成（`shape-*` 構想） | 3 |

> 件数は配置フォルダ基準で数える（集計ルールの正本は `.kiro/steering/focus.md`）。

### 棚卸しで判明した整理候補（housekeeping）
- `ukagaka-desktop-mascot`（phase=completed）を `completed/` へ移動
- `shape-*` 3件は `spec.json` 生成で正式化、または `backlog/` へ退避
- `future-requirements-survey`（調査完了）の役割が終われば `backlog/`/`completed/` へ
- 関連: `codebase-review-loop`（レビュー運用）

---

## 更新ガイド
1. トラック/マイルストーンの状態列を ⚪→🔮🔵→✅ に更新
2. 新規完了基盤は「解決済み基盤資産」へ追記
3. 設計判断の変更は [COMPAT_ARCHITECTURE.md](COMPAT_ARCHITECTURE.md) を正本として更新
4. ポートフォリオ件数は配置フォルダ基準で再計上

## 旧ロードマップ
v1.x（ボトムアップ表示層計画）の本文は git 履歴に保存（本ファイルを上書き更新）。旧 ukagaka-desktop-mascot ROADMAP は `doc/archive/ROADMAP_ukagaka_meta.md` にアーカイブ済み。

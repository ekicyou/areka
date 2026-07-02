# Requirements Document

## Project Description (Input)

本体 `wintf` の透過は ULW/DComp 切替式だが、別プロセスへの α マスク連動クリック透過は ULW（CPU ビットマップ方式）に依存しており、GPU 合成（DComp/WUC）描画と併用できない。そのため「別プロセス透過を得るために GPU 描画を諦める踏み絵」になっている。至上要件は GPU 描画（合成層）を捨てないこと。先進坑 `pilot-clickthrough-alpha-toggle`（✅ go 済み・2026-07-01 開発者承認）が、`WS_EX_TRANSPARENT` の動的トグル方式で「GPU 合成描画を維持したまま別プロセスへのクリック透過が成立する」ことを実証した。本坑ではこの知見をクリーンに掘り直し（コピペ流用禁止）、本体 `wintf` に `WS_EX_TRANSPARENT` 動的トグル機構を組み込む。本体の α マスク（実描画 α バッファ／`AlphaMask`）参照でキャラクター描画領域のみクリック可能・透明領域は背面プロセスへ透過、を既存機能を壊さずリリースビルド互換で実現する。ULW ルートは本方式が完全に有効と判断されれば破棄予定だが、当面は検証期間として並走させる。

## Boundary Context

- **In scope**:
  - `wintf` 本体への `WS_EX_TRANSPARENT` 動的トグル機構の実装（GPU 合成描画を維持したまま別プロセスへクリック透過させる当たり判定制御）。
  - 本体の実描画 α マスク（`AlphaMask`）参照による、キャラクター描画領域のみクリック受領・透明領域は背面プロセスへ透過。
  - カーソル位置監視ワーカと状態変化最適化（前回状態との差分時のみ拡張スタイルを適用）。
  - スレッド跨ぎ通知は既存の `event_listener` 起床パターンに倣う（tokio 非使用）。
  - ドラッグ移動中は表示位置に関わらずクリック透過を抑止し続ける挙動。
  - `docs/click_through.md` の新規作成（仕組み概要・ULW/HTTRANSPARENT/Layered を採らない理由・API 使用例・既知の制約）。
  - 既存機能の非破壊性とリリースビルド互換の検証。
- **Out of scope**:
  - ULW バックエンドの即時撤去（並走期間中は残置する。撤去は別坑 `wintf-ulw-removal`）。
  - 先進坑 `pilot-clickthrough-alpha-toggle` コードのコピペ流用（知見のクリーンな掘り直しのみ）。
  - 新しい大型クレート（winit/tauri 等）の追加、および `Cargo.toml` 依存の大幅追加（最小限のみ・追加提案は依頼者確認）。
  - `WM_NCHITTEST`→`HTTRANSPARENT` 経由の透過（プロセス境界を越えず本要件では採用不可）。
- **Adjacent expectations**:
  - 表示合成は既存の GPU 合成層（DComp/WUC）が担い、本機能はその表示を変更しない。表示層と当たり判定層は独立し、本機能は当たり判定層のみを制御する。
  - `AlphaMask` の生成・実描画 α バッファは既存グラフィックス経路が提供し、本機能はその参照側である。
  - 拡張スタイル反映・ウィンドウ生成基盤（既存のスタイル適用・facade）は本機能が利用する前提として存在する。

## Requirements

### Requirement 1: GPU 合成描画を維持したままの別プロセスクリック透過

**Objective:** マスコット利用者として、キャラクターが GPU 合成（3D／2D）で描画されたまま、キャラクター以外の透明領域のクリックが背面アプリケーションへ透過してほしい。それにより「別プロセス透過のために GPU 描画を諦める」踏み絵を回避できる。

#### Acceptance Criteria

1. While GPU 合成描画（DComp/WUC 経路）が有効な状態, the click-through 機構 shall キャラクター描画領域以外の透明領域上のクリックを背面プロセスへ透過させる。
2. When 透明領域上のクリックが背面の別プロセスウィンドウへ届いた, the click-through 機構 shall 自ウィンドウでそのクリックを受領しない。
3. The click-through 機構 shall クリック透過を成立させるために GPU 合成描画（合成層の表示内容）を無効化・省略しない。
4. Where キャラクターの表示内容が 2D サーフェスまたは合成スワップチェーン（3D／Live2D 相当）のいずれである場合, the click-through 機構 shall 当たり判定の挙動を表示内容の種類に依存させない。

### Requirement 2: α マスク連動のキャラクター領域クリック受領

**Objective:** マスコット利用者として、見えているキャラクターのピクセル領域だけがクリックを受け取り、見えない透明ピクセルはクリックを受け取らないでほしい。それにより見た目と操作範囲が一致する。

#### Acceptance Criteria

1. When カーソルがキャラクターの実描画 α マスク上で不透明（当たり）と判定される位置にある, the click-through 機構 shall そのウィンドウでクリックを受領可能な状態にする。
2. When カーソルが実描画 α マスク上で透明（非当たり）と判定される位置にある, the click-through 機構 shall そのウィンドウをクリック透過状態にする。
3. The click-through 機構 shall 当たり判定に本体の実描画 α バッファ（実際に表示されているキャラクターの α）を参照し、固定矩形や外部の仮マスクに依存しない。
4. When キャラクターの表示内容が更新され実描画 α マスクが変化した, the click-through 機構 shall 更新後の α マスクに基づいて当たり判定領域を追随させる。

### Requirement 3: カーソル監視と状態変化最適化

**Objective:** システム保守者として、カーソル位置の監視が本体 UI 処理を阻害せず、透過状態の適用が必要なときだけ行われてほしい。それにより常時ポーリングによる無駄な処理と描画妨害を避けられる。

#### Acceptance Criteria

1. While 機構が有効, the click-through 機構 shall カーソル位置を継続的に監視し、現在位置が α マスク上で当たりか非当たりかを判定する。
2. When 当たり／非当たりの判定結果が前回の適用済み状態と同一である, the click-through 機構 shall 拡張スタイルの再適用を行わない。
3. When 当たり／非当たりの判定結果が前回の適用済み状態から変化した, the click-through 機構 shall クリック透過状態の切り替えを 1 回適用する。
4. The click-through 機構 shall カーソル監視処理を本体 UI スレッドの描画・入力処理とは別の実行文脈で行い、監視によって描画のなめらかさを損なわない。

### Requirement 4: スレッド跨ぎ通知（event_listener パターン・tokio 非使用）

**Objective:** システム保守者として、カーソル監視側から UI 側への状態変化通知が既存のスレッド跨ぎ起床パターンと一貫していてほしい。それにより新たな非同期ランタイム依存を持ち込まず、保守性を保てる。

#### Acceptance Criteria

1. When カーソル監視側が状態変化を検出した, the click-through 機構 shall UI スレッドへ既存の `event_listener` 起床パターンで変化を通知する。
2. The click-through 機構 shall スレッド跨ぎ通知の実現に tokio（および同等の外部非同期ランタイム）を使用しない。
3. When UI スレッドが状態変化通知を受け取った, the click-through 機構 shall 拡張スタイルの適用を UI スレッド上で実行する。

### Requirement 5: ドラッグ移動中のクリック透過抑止

**Objective:** マスコット利用者として、キャラクターの不透明部を掴んでウィンドウをドラッグ移動する間、カーソルがキャラ領域から一時的に外れてもドラッグが崩れないでほしい。それにより移動操作が安定する。

#### Acceptance Criteria

1. While ドラッグ移動が進行中, the click-through 機構 shall カーソル位置および α マスク判定結果に関わらずクリック透過を有効化しない（透過を外したまま維持する）。
2. When ドラッグ移動が終了した, the click-through 機構 shall 現在のカーソル位置と α マスク判定に基づいてクリック透過状態を再収束させる。
3. While ドラッグ移動が進行中, the click-through 機構 shall 状態変化最適化（Requirement 3）による透過 ON への切り替えを抑止する。

### Requirement 6: 拡張スタイル構成の制約

**Objective:** システム保守者として、クリック透過の実現手段が確定した安全なレシピ（先進坑で実証済み）に限定され、却下済みの手段や副作用のある手段を持ち込まないでほしい。それにより DComp/WUC 描画との共存を壊さない。

#### Acceptance Criteria

1. The click-through 機構 shall クリック透過の当たり判定切り替えを `WS_EX_TRANSPARENT` 拡張スタイルの動的な付与・除去で実現する。
2. Where `WS_EX_LAYERED` を用いる場合, the click-through 機構 shall それを当たり判定を効かせる同伴フラグとしてのみ立て、レイヤード描画（`UpdateLayeredWindow`／`SetLayeredWindowAttributes`）には使用しない。
3. The click-through 機構 shall `WM_NCHITTEST`→`HTTRANSPARENT` ハンドラを別プロセス透過の手段として追加しない。
4. If 追加の拡張スタイル（例: `WS_EX_LAYERED`）付与や `WM_NCHITTEST` ハンドラ追加が必要と判断される場合, the 実装者 shall 理由を添えて依頼者へ確認し、独断で追加しない。
5. The click-through 機構 shall 既存本体コードを推測で書き換えず、変更対象ファイルと変更内容を事前に依頼者へ提示して確認を得る。

### Requirement 7: ULW との並走と既存機能の非破壊

**Objective:** システム保守者として、新方式の検証期間中も既存の ULW 経路と既存機能が壊れず維持されてほしい。それにより十分な検証期間とエンバグ対応の余地を確保できる。

#### Acceptance Criteria

1. While 検証期間, the click-through 機構 shall 既存の ULW バックエンド経路を撤去せず並走可能な状態で残す。
2. The click-through 機構 shall 既存の透過・ヒットテスト・ウィンドウ管理などの既存機能を破壊しない。
3. When 本方式が完全に有効と判断され ULW ルート破棄が決定された, the 実装者 shall `tech.md`／`roadmap.md` の「ULW 一択」相当記述の更新対象を明示できる状態を保つ。

### Requirement 8: 高 DPI・マルチモニタ環境での座標一致

**Objective:** マスコット利用者として、高 DPI やマルチモニタ環境でも、見えているキャラクター領域と実際にクリックを受け取る領域が一致してほしい。それにより環境に依らず操作が破綻しない。

#### Acceptance Criteria

1. While per-monitor-v2 の高 DPI 環境, the click-through 機構 shall 見えているキャラクター領域と当たり判定領域を座標一致させる。
2. While 複数モニタにまたがる、または DPI 倍率の異なるモニタ間を移動する状況, the click-through 機構 shall α マスク判定に用いるカーソル座標とマスク座標の対応を保ち、当たり判定を破綻させない。
3. When ウィンドウが移動した, the click-through 機構 shall 移動後の位置でも見た目のキャラクター領域と当たり判定領域の一致を維持する。

### Requirement 9: リリースビルド互換・可搬性・依存最小

**Objective:** システム保守者として、本機能が既存のリリース最適化・32bit 可搬性・依存方針を崩さずに組み込まれてほしい。それにより出荷構成とビルドの健全性を保てる。

#### Acceptance Criteria

1. The click-through 機構 shall 既存のリリース最適化設定（`opt-level='z'`, `lto=true`）でビルド可能かつ動作する。
2. The click-through 機構 shall 32bit ターゲットの可搬性を崩さない。
3. The click-through 機構 shall 新規の大型クレートを追加せず、依存追加は最小限に留める（追加が必要な場合は提案として依頼者へ確認し独断追加しない）。

### Requirement 10: 仕組みドキュメントの整備

**Objective:** 将来の開発者として、本方式の仕組み・不採用手段の理由・使用例・既知の制約が文書化されていてほしい。それにより ULW/HTTRANSPARENT/Layered を採らない判断の根拠と使い方を後から追える。

#### Acceptance Criteria

1. When 本機能が実装された, the 実装者 shall `docs/click_through.md` を新規作成する。
2. The `docs/click_through.md` shall 仕組みの概要、ULW／`HTTRANSPARENT`／レイヤード描画を採らない理由、API 使用例、既知の制約を含む。
3. Where 設計判断が確定した場合, the 実装者 shall 正本 `doc/COMPAT_ARCHITECTURE.md` の更新対象を明示できる状態を保つ。

# Requirements Document

## Project Description (Input)

表示レイヤーの合成を現在の **DirectComposition** 依存から WinRT の **Windows.UI.Composition（WUC）** へ移行し、DirectComposition への依存を廃する（本坑 / main）。移行の狙いは **DComp 依存の廃止・純粋等価移行** であり、WUC の新能力（アニメ/エフェクト）活用は本 spec の目的ではない。移行後も **描画結果・再描画挙動が移行前と完全等価** であり、**当たり判定・ウィンドウ管理・スレッド構成は不変** であること。ULW 一式の除去は別 spec（`wintf-ulw-removal`）に切り出し、本 spec には含めない。

## Introduction

wintf の表示バックエンドは現在 DirectComposition（`IDCompositionDevice3` / `IDCompositionTarget` / `IDCompositionVisual3` / `IDCompositionSurface` ＋ 毎フレーム `Commit()`）に依存している。本移行は、この合成パスを Windows.UI.Composition（`Compositor` / `DesktopWindowTarget` / `ContainerVisual`・`SpriteVisual` / `CompositionDrawingSurface` ＋ DispatcherQueue による暗黙反映）へ**純粋等価**で差し替える。開発者（wintf/areka 保守者）にとっての価値は、DComp 依存を廃して合成基盤を WUC 系へ寄せることであり、その代償として **利用者から見た描画結果・再描画挙動・入力挙動が一切変化しない** ことが受け入れの前提となる。

DComp パスは現状すでに独立経路として隔離されており（ULW アームとは別経路）、移行は device / target / visual-tree / surface / frame-apply の各層を写像することで完結する。DispatcherQueue の初期化のみが唯一の新規初期化であり、既存の UI スレッド message pump に相乗りする（ポンプは差し替えない）。

移行の正当性は、まず本 spec 内の**スパイク検証**（DispatcherQueue 統合 ＋ `DesktopWindowTarget` ＋ D2D `BeginDraw` の最小往復で 1 サーフェスを表示）で等価描画を確認したのちに、全面移行を進めることで担保する。

## Boundary Context

- **In scope（利用者・保守者から見た振る舞い）**:
  - 表示合成バックエンドが Windows.UI.Composition で再構成され、device / target / visual-tree / surface / frame-apply の各層が WUC 相当へ写像される。
  - UI スレッドへの DispatcherQueue コントローラ初期化の組み込み（既存 message pump に相乗り・ポンプ非差し替え）。
  - WUC features の最小追加（ビルドが通る範囲での有効化）。
  - 移行後の **描画等価性の検証**（見た目・再描画挙動が移行前と一致・ビルド通過・起動時に従来と同一の描画結果）。
- **Out of scope（本 spec が扱わない）**:
  - ULW 一式の除去、および ULW⇔DComp を選ぶモード選択機構の撤去・整理（別 spec `wintf-ulw-removal` の領分）。
  - 当たり判定（ヒットテスト・ウィンドウ ex-style によるクリックスルー）・ウィンドウ管理・スレッド構成の変更。
  - WUC 新能力（合成アニメーション・エフェクトグラフ）の活用、および将来拡張のための投機的抽象・拡張シームの追加。
  - swapchain ベースの content 束縛パス（本 spec は描画サーフェス直描きパスのみ）。
- **Adjacent expectations（隣接 spec・システムへの期待と非所有）**:
  - 本移行は **表示層のみ** を触る。別プロセスへのクリック透過（`wintf-clickthrough-alpha-toggle`）は **当たり判定層のみ** を触るため意味論的に非衝突であり、両者の実質の重なりはウィンドウ ex-style 算出の 1 関数のみである。本 spec は当該ウィンドウフラグの透過挙動を DComp 時と同一に保つ責務のみを負い、クリックスルー機構自体は所有しない。
  - `wintf-ulw-removal` は本移行の完了を前提に後続する。本 spec は ULW 経路を残置したまま（並走可能な状態で）DComp 経路のみを置換し、ULW の撤去責務は負わない。
  - DispatcherQueue は既存 UI スレッド message pump（`wintf-winmsg-executor` 基盤）に相乗りする前提であり、本 spec は pump の実装を差し替えない。

## Requirements

### Requirement 1: スパイク検証による等価描画の先行確認
**Objective:** As a wintf 保守者, I want 全面移行の前に WUC 合成の最小往復を単独で検証したい, so that 移行方式の等価性を早期に確認し、致命的な手戻りを防げる

#### Acceptance Criteria
1. When 移行作業を開始する, the wintf 表示基盤 shall 全面移行に先立ち、DispatcherQueue 初期化・`DesktopWindowTarget` 束縛・描画サーフェスへの D2D `BeginDraw` を用いた最小往復で 1 サーフェスを表示するスパイクを実行する。
2. When スパイクが 1 サーフェスを表示する, the wintf 表示基盤 shall そのサーフェスが移行前の等価な描画（同一のピクセル結果）を得られることを確認可能な状態にする。
3. If スパイクで等価描画が確認できない, then the wintf 表示基盤 shall 全面移行へ進まず、原因を明らかにして方式を見直す。

### Requirement 2: 合成デバイス層の WUC 移行
**Objective:** As a wintf 保守者, I want 合成デバイスを DirectComposition デバイスから WUC のコンポジタへ差し替えたい, so that 合成基盤の起点から DComp 依存を廃せる

#### Acceptance Criteria
1. When 表示基盤が合成デバイスを初期化する, the wintf 表示基盤 shall DirectComposition デバイスに代えて WUC のコンポジタ（`Compositor`）と、既存の D2D/D3D11 デバイスから生成した合成グラフィックスデバイスを用いる。
2. While 表示基盤が動作している間, the wintf 表示基盤 shall 合成デバイスの遅延初期化・単一インスタンスという現行のライフサイクル方針を維持する。
3. The wintf 表示基盤 shall 合成デバイスの初期化経路から DirectComposition デバイスへの依存を含まない。

### Requirement 3: DispatcherQueue 初期化の UI スレッド組み込み（pump 非差し替え）
**Objective:** As a wintf 保守者, I want WUC 合成が要求する DispatcherQueue を既存 UI スレッド上に用意したい, so that スレッド構成を変えずに WUC を駆動できる

#### Acceptance Criteria
1. When コンポジタを生成する前, the wintf 表示基盤 shall 同一 UI スレッド上で DispatcherQueue コントローラを（現在スレッド種別で）初期化する。
2. The wintf 表示基盤 shall 既存の UI スレッド message pump（`GetMessage`/`DispatchMessage` ループ）を差し替えず、DispatcherQueue をそのポンプに相乗りさせる。
3. While アプリケーションが動作している間, the wintf 表示基盤 shall DispatcherQueue コントローラをコンポジタより長寿命に保ち、終了時に保留分をドレインする。
4. The wintf 表示基盤 shall スレッド構成（UI スレッド固定モデル・ワーカーからのチャンネル marshal）を移行前と不変に保つ。

### Requirement 4: 合成ターゲット束縛の WUC 移行
**Objective:** As a wintf 保守者, I want ウィンドウへの合成ターゲット束縛を DComp ターゲットから WUC のデスクトップウィンドウターゲットへ差し替えたい, so that HWND への合成出力先を WUC 経由に切り替えられる

#### Acceptance Criteria
1. When ウィンドウの合成ターゲットを生成する, the wintf 表示基盤 shall `CreateTargetForHwnd` による DirectComposition ターゲットに代えて、HWND に束縛した WUC のデスクトップウィンドウターゲット（`DesktopWindowTarget`）を用いる。
2. The wintf 表示基盤 shall ウィンドウレベルのターゲット束縛が、移行前と同じ HWND・同じウィンドウライフサイクルに対応するよう保つ。

### Requirement 5: ビジュアルツリー同期の WUC 移行（親子・Z 順の等価維持）
**Objective:** As a wintf 保守者, I want ビジュアルツリーの構築と同期を DComp ビジュアルから WUC のコンテナ／スプライトビジュアルへ差し替えたい, so that 表示ツリーの構造と重なり順を等価に保てる

#### Acceptance Criteria
1. When ビジュアルツリーを構築・更新する, the wintf 表示基盤 shall `IDCompositionVisual3` に代えて WUC のコンテナビジュアル／スプライトビジュアルを用いる。
2. When 親子関係（ChildOf）と Z 順が変化する, the wintf 表示基盤 shall 現行の追加・除去・全除去（`AddVisual`/`RemoveVisual`/`RemoveAllVisuals` 相当）ロジックに従って、移行前と同一のツリー構造・同一の重なり順を再現する。
3. The wintf 表示基盤 shall ビジュアルの配置・変換に関する利用者可視な結果（位置・Z 順）を移行前と等価に保つ。

### Requirement 6: 描画サーフェスの WUC 移行（D2D 直描き経路の維持）
**Objective:** As a wintf 保守者, I want サーフェス生成と描画を DComp サーフェスから WUC の描画サーフェスへ差し替えたい, so that 既存の D2D 描画コードをそのまま用いつつ WUC でサーフェスを合成できる

#### Acceptance Criteria
1. When サーフェスを生成する, the wintf 表示基盤 shall DirectComposition の `CreateSurface` に代えて WUC の描画サーフェス（`CompositionDrawingSurface`）を用いる。
2. When サーフェスへ描画する, the wintf 表示基盤 shall `BeginDraw`→D2D デバイスコンテキストへの描画→`EndDraw` という現行の per-frame D2D 再描画コードをそのまま用いる。
3. When 生成したサーフェスをビジュアルへ束ねる, the wintf 表示基盤 shall サーフェスブラシ（`CreateSurfaceBrush`）を介してスプライトビジュアルへ束ねる。
4. The wintf 表示基盤 shall サーフェスの画素形式・アルファ扱い（プリマルチプライド B8G8R8A8 相当）に基づく描画結果を移行前と等価に保つ。
5. The wintf 表示基盤 shall content 束縛に swapchain 経路（`CreateCompositionSurfaceForSwapChain` 相当）を用いない。

### Requirement 7: フレーム反映モデルの移行（明示 Commit の廃止と暗黙反映）
**Objective:** As a wintf 保守者, I want 毎フレーム末の明示的 Commit を廃し WUC の暗黙反映に移行したい, so that フレーム反映の状態モデルを WUC 系に合わせられる

#### Acceptance Criteria
1. When フレームの全変更を適用する, the wintf 表示基盤 shall DirectComposition の明示的 `Commit()` を廃し、DispatcherQueue のティックによる WUC の暗黙反映に依拠する。
2. The wintf 表示基盤 shall 反映モデルをバッチ適用から暗黙反映へ変えても、フレームごとのデータフロー（1 フレームで適用される変更の内容）を移行前と等価に保つ。
3. The wintf 表示基盤 shall 再描画のタイミング・頻度から利用者が観測できる挙動（表示更新の見た目上の結果）を移行前と等価に保つ。

### Requirement 8: 描画等価性の受け入れ基準
**Objective:** As a wintf/areka 保守者, I want 移行後の見た目・再描画挙動が移行前と一致することを受け入れ基準としたい, so that 「純粋等価移行」であることを検証で担保できる

#### Acceptance Criteria
1. When 移行後のアプリケーションをビルドする, the wintf 表示基盤 shall 既存のリリース最適化設定（サイズ最適化・LTO）と互換な状態でビルドが通る。
2. When 移行後のアプリケーションを起動する, the areka 本体 shall 移行前と同一の描画結果を表示する。
3. While 利用者がアプリケーションを操作している間, the areka 本体 shall 見た目（表示内容・重なり順・透過）と再描画挙動を移行前と等価に保つ。
4. The wintf 表示基盤 shall 32bit 可搬性を移行前と同様に維持する。

### Requirement 9: スコープ境界の不変性（隣接関心を侵さない）
**Objective:** As a wintf 保守者, I want 本移行が表示層のみを触り隣接関心を侵さないよう境界を固定したい, so that クリックスルー・ULW 除去・M1 の各軸と安全に並行できる

#### Acceptance Criteria
1. The wintf 表示基盤 shall 当たり判定（ヒットテスト・ウィンドウ ex-style によるクリックスルー機構）の振る舞いを本移行で変更しない。
2. While DComp 経路を WUC へ置換する間, the wintf 表示基盤 shall ULW 経路とそのモード選択機構を残置し、除去・整理しない。
3. Where DComp モードでウィンドウ透過フラグ（`WS_EX_NOREDIRECTIONBITMAP`）を用いていた, the wintf 表示基盤 shall 移行後も同フラグによる透過挙動を DComp 時と同一に保つ。
4. The wintf 表示基盤 shall WUC 新能力（合成アニメーション・エフェクト）や将来拡張のための投機的抽象を本 spec で導入しない。

### Requirement 10: 既存本体コードへの変更前提示（推測改変の禁止）
**Objective:** As a 依頼者, I want 既存本体コードを触る前に対象ファイルと変更内容の提示を受けたい, so that 推測による書き換えを防ぎ変更範囲を統制できる

#### Acceptance Criteria
1. When 既存本体コードを変更する必要が生じる, the 移行作業 shall 変更に先立ち、対象ファイルと変更内容を依頼者へ提示して確認を取る。
2. If Win32/WinRT API あるいはクレート仕様に不確実性が残る, then the 移行作業 shall 推測で進めず確認を求める。
3. When 設計判断を変更する, the 移行作業 shall `doc/COMPAT_ARCHITECTURE.md` を正本として更新する。

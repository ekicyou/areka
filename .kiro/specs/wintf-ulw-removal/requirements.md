# Requirements Document

## Introduction

wintf の表示レイヤーの透過は、これまでウィンドウ生成時に `CompositionMode` enum で **ULW（既定）⇄ GPU 合成（DComp/WUC）** を選ぶ切替式だった。ULW アームは `UpdateLayeredWindow` による CPU ビットマップ方式（D2D1 合成→staging→DIBSection→`UpdateLayeredWindow`）で、GPU 合成スワップチェーンと併用できず、別プロセスクリック透過を得るために GPU 描画を諦める踏み絵になっていた。

本坑クリックスルー（`WS_EX_TRANSPARENT` 動的トグル・`wintf-clickthrough-alpha-toggle` 2026-07-02 完了）が GPU 合成（WUC）を維持したまま別プロセスクリック透過を成立させたことで、ULW アームは不要になった。

本仕様は **ULW 専用コード一式を撤去**し、表示バックエンドを **GPU 合成（WUC）単独**へ collapse する。撤去は表示レイヤーの重複経路・切替分岐を消して単純化することを目的とし、**残す GPU 合成パスの描画結果・挙動、当たり判定、ウィンドウ管理、スレッド構成は一切変えない**（純粋な削除リファクタ）。

## Boundary Context

- **In scope**:
  - ULW 専用描画経路の撤去（ULW 合成器 `WindowD3D11Compositor`＝`ecs/graphics/compositor.rs`・`ecs/graphics/compositor_systems/`（`compositor_init_system`・`composite_render_system`・`ulw_present_system`）・`com/ulw.rs`（`transfer_to_hbitmap`・`present_layered_window`））
  - ULW system 群の ECS スケジュール登録解除（`ecs/world/mod.rs` の `GraphicsSetup` の `compositor_init_system` 登録・`CommitComposition` の `ulw_present_system` 登録）
  - `CompositionMode` の ULW variant 除去・単一化（単一 variant なら enum 撤去、または最小化。生成時デフォルトを GPU 合成へ）
  - `compute_ex_style()`（`runtime/window_factory.rs`）の ULW 分岐（`WS_EX_LAYERED`）除去と GPU 合成 ex_style（`WS_EX_NOREDIRECTIONBITMAP`）への一本化
  - 上記 API 破壊的変更に伴う wintf 内・areka 側呼び出し（`CompositionMode::DComp`／`CompositionMode::ULW` 指定等）の追随、および ULW 前提の examples・tests の整理
  - 「ULW 一択」記述の残余（`doc/COMPAT_ARCHITECTURE.md` ほか doc 配下・コード内コメント）の整合更新
  - 撤去前後の非破壊検証（残す GPU 合成パスの描画等価・ビルド互換・起動・クリックスルー機能維持）
- **Out of scope**:
  - DComp→WUC 差し替え（別仕様 `wintf-dcomp-to-wuc-migration`＝完了済み・残す側。`com/wuc.rs`・`ecs/graphics/wuc_resource.rs`・`visual_manager.rs` は撤去対象外）
  - クリックスルー機構の実装（`wintf-clickthrough-alpha-toggle`＝完了済み）
  - 当たり判定（hit-test / α マスク）・ウィンドウ管理・スレッド構成の変更
  - 新機能追加・GPU 合成パスの挙動変更・最適化
  - steering（`tech.md`／`product.md`／`roadmap.md`）の「ULW 一択」記述更新（2026-07-01〜03 に更新済みのため本仕様スコープ外）
- **Adjacent expectations**:
  - **前提依存**: `wintf-clickthrough-alpha-toggle` が完了済みであること（ULW を撤去しても別プロセスクリック透過が GPU 合成のまま成立する安全網が既に存在すること）。
  - **`WS_EX_LAYERED` の帰属移行**: ULW 撤去後、GPU 合成（WUC）窓への `WS_EX_LAYERED` の唯一の源はクリックスルー機構（`win_style.rs` の `apply_layered_companion()`・クリックスルー controller `ecs/clickthrough/controller.rs:171` から実行時呼び出し）となる。クリックスルー登録窓はこの経路で `WS_EX_LAYERED` を実行時に受け取り続ける。撤去はこの経路を巻き込まない。
  - **クロスユニット契約**: `CompositionMode` collapse は破壊的変更であり、着手予定の `areka-P0-emo-present`／`areka-P0-window-placement` が同じ API に触れる。本仕様の責務は wintf 本体の ULW 撤去と自 crate・areka 側呼び出しの追随までとし、他 spec の追随責務の帰属は着手時調整に委ねる。

## Requirements

### Requirement 1: ULW 専用描画経路の撤去

**Objective:** wintf 保守者として、ULW 専用の合成器・present 経路・ユーティリティを撤去したい。それにより GPU 合成と併用不能な CPU ビットマップ経路がコードベースから消え、表示レイヤーが単純化されるからである。

#### Acceptance Criteria

1. When ULW 撤去が完了したとき、the wintf crate shall `ecs/graphics/compositor.rs` の ULW 合成器 `WindowD3D11Compositor` を含まない。
2. When ULW 撤去が完了したとき、the wintf crate shall `ecs/graphics/compositor_systems/` の ULW system 群（`compositor_init_system`・`composite_render_system`・`ulw_present_system`）を含まない。
3. When ULW 撤去が完了したとき、the wintf crate shall `com/ulw.rs` の ULW ユーティリティ（`transfer_to_hbitmap`・`present_layered_window` の `UpdateLayeredWindow` 経路）を含まない。
4. When ULW 専用コードの撤去を行うとき、the developer shall 撤去対象ファイルと変更内容を事前に依頼者へ提示し確認を得た上で削除する（推測で消さない）。
5. The wintf crate shall 撤去後に、参照が残存する ULW 専用シンボル（`WindowD3D11Compositor`・`UpdateLayeredWindow` 経路・`compositor_systems` の system・`transfer_to_hbitmap`・`present_layered_window`）を一切持たない。

### Requirement 2: ULW system の ECS スケジュール登録解除

**Objective:** wintf 保守者として、ECS スケジュールから ULW 前提の system 群を除去したい。撤去された ULW コードへの登録が残ると起動時にビルド不能または実行時矛盾になるからである。

#### Acceptance Criteria

1. When ULW system 群を撤去したとき、the wintf ECS スケジュール shall `GraphicsSetup` の `compositor_init_system` 登録および `CommitComposition` の `ulw_present_system` 登録（`ecs/world/mod.rs`）を含まない。
2. If ULW system の撤去後に GPU 合成（WUC）パスの graphics schedule 登録が残るなら、then the wintf ECS スケジュール shall その GPU 合成側 system 登録を撤去前と同一に保つ（GPU 合成側のスケジュールを変更しない）。
3. The wintf crate shall ULW system 撤去後もビルドが通過し、ECS スケジュールが起動時に矛盾なく構成される。

### Requirement 3: CompositionMode の collapse

**Objective:** wintf 利用者・保守者として、`CompositionMode` を GPU 合成単独へ collapse したい。ULW variant を残すと選択肢のない切替 API が意味を失い、生成時分岐が無駄に残るからである。

#### Acceptance Criteria

1. When ULW variant を除去したとき、the `CompositionMode` 型 shall ULW を表す variant を持たない。
2. Where GPU 合成が唯一のモードになった場合、the wintf crate shall `CompositionMode` を単一値へ最小化するか、単一 variant であれば enum 自体を撤去する。
3. When ウィンドウを生成するとき、the wintf window 生成経路 shall 生成時のデフォルト合成モードを GPU 合成（WUC）とする（ULW を既定にしない）。
4. When `CompositionMode` の collapse を行うとき、the wintf crate shall 自 crate 内の全呼び出し箇所（生成経路・`window_proc/lifecycle.rs`・`ecs/clickthrough/controller.rs`・ULW 前提の examples/tests を含む）を新しい API へ追随させ、ビルドが通過する。
5. When `CompositionMode` の collapse を行うとき、the areka crate shall `CompositionMode::DComp` 指定等の呼び出しを新しい API へ追随させ、ビルドが通過する。

### Requirement 4: compute_ex_style の分岐一本化

**Objective:** wintf 保守者として、`compute_ex_style()` の ULW 分岐（`WS_EX_LAYERED` 付与）を除去し、GPU 合成の ex_style へ一本化したい。ULW モードが消えれば `WS_EX_LAYERED` を生成時に付与する分岐は不要になるからである。

#### Acceptance Criteria

1. When ex_style を算出するとき、the `compute_ex_style()`（`runtime/window_factory.rs`）shall ULW を前提とした `WS_EX_LAYERED` 付与分岐を持たない。
2. When GPU 合成窓の ex_style を算出するとき、the `compute_ex_style()` shall GPU 合成の ex_style（既存 DComp 分岐と等価に `WS_EX_LAYERED` を落とし `WS_EX_NOREDIRECTIONBITMAP` を付与）を一本の経路で返す。
3. The `compute_ex_style()` shall 生成時に `WS_EX_LAYERED` を付与しない（撤去前の GPU 合成モードと同一の挙動を保つ）。

### Requirement 5: クリックスルー機構の非破壊（WS_EX_LAYERED 帰属移行）

**Objective:** areka 利用者として、ULW 撤去後もクリックスルー登録窓が別プロセスクリック透過を保ち続けたい。ULW 撤去後は `WS_EX_LAYERED` の唯一の源がクリックスルー機構になるため、撤去がこの経路を壊さないことを保証したいからである。

#### Acceptance Criteria

1. While クリックスルーが登録された GPU 合成窓が稼働しているとき、the クリックスルー機構（`win_style.rs` の `apply_layered_companion()`・controller から実行時呼び出し）shall 実行時に当該窓へ `WS_EX_LAYERED` を付与する。
2. When ULW を撤去したとき、the wintf crate shall クリックスルー機構の `apply_layered_companion()` 経路（`win_style.rs`・`ecs/clickthrough/`）を変更・除去しない。
3. While ULW 撤去後にクリックスルー登録窓が稼働しているとき、the areka マスコット shall 透明ピクセル上のクリックを別プロセスへ透過させ続ける（撤去前と同一のクリックスルー挙動を保つ）。
4. The wintf crate shall ULW 撤去後もクリックスルーの α 源として per-widget α マスク（`AlphaMask::is_hit`）のみを用い、撤去された ULW compositor の staging α バッファへ依存しない。

### Requirement 6: 残す GPU 合成パスの描画非破壊

**Objective:** areka 利用者・wintf 保守者として、ULW 撤去の前後で残す GPU 合成（WUC）パスの見た目・再描画が変わらないことを保証したい。本仕様は純粋な削除であり、表示結果の変化はリグレッションだからである。

#### Acceptance Criteria

1. When ULW 撤去後にアプリを起動するとき、the areka アプリ shall 撤去前と同一の描画結果（見た目）を表示する。
2. While GPU 合成窓が再描画されるとき、the wintf 描画パス shall 撤去前と等価な再描画結果を生成する。
3. The wintf crate shall ULW 撤去によって当たり判定・ウィンドウ管理・スレッド構成を変更しない。
4. When 撤去後にビルドするとき、the wintf ワークスペース shall リリース最適化設定（`opt-level='z'`・`lto=true`）と互換のままビルドを通過させる。

### Requirement 7: ドキュメント残余の整合更新

**Objective:** プロジェクト保守者として、コードから ULW が消えたことに doc 正本・コード内コメントを整合させたい。撤去済みの機構を前提とする記述が残ると誤解を招くからである。

#### Acceptance Criteria

1. When ULW 撤去が完了したとき、the `doc/COMPAT_ARCHITECTURE.md`（設計判断の正本）shall ULW を残存機構として前提する記述を含まず、GPU 合成単独へ collapse した現況に整合する。
2. When ULW 撤去が完了したとき、the wintf コード内コメント shall 撤去された ULW 経路を前提とする残余記述を含まない。
3. Where steering 文書（`tech.md`・`product.md`・`roadmap.md`）の「ULW 一択」記述が 2026-07-01〜03 に既に更新済みの場合、the 本仕様 shall それらの再更新を行わない（doc 配下・コード内コメントの残余整合のみを対象とする）。

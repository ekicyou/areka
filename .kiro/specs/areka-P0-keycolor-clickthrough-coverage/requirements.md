# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: 里々／YAYA の標準テンプレートで作られたゴーストを使う第三者——α 版の利用者の最初の 1 体はほぼ確実にこの形である。標準テンプレートの絵は α チャンネルを持たず、左上 1 画素と同じ色を抜いて透過させる。完了 spec `areka-P0-shell-implicit-surface` の要件 4 の 10 番は「抜き色で透明になった場所は、α で透明な場所と同じく『キャラクターの外』として扱う＝クリックは背後の窓へ抜け、撫でや当たり判定の対象にならない」と定める。

**今の状況**: この要件には、壊れたときに赤になるテストが 1 本も無い。在るのは両端だけである——抜き色 → α（`crates/areka-emo-atlas/src/normalize_key_color_tests.rs`）と、合成後の画素の α → 当たりのマスク（`crates/areka-emo-present/src/presenter/budget_tests.rs`）。途中のどこか（焼く・合成する・マスクへ渡す）で抜いた α が落ちると、絵の四角い外枠の全面がクリックを食い、利用者から見ると「キャラクターの周りのデスクトップが操作できない」になる。2026-09-20 の開発者の目視で今日は正しいことが確かめられているが、目視は明日の退行を止めない。

**何が変わるべきか**: 抜き色の絵を入力に、**焼く → 合成する → 当たりのマスクを作る**までを製品が実際に通る順で 1 本の決定論テストが通し、抜かれた画素の位置でマスクが「外」、抜かれなかった画素の位置で「内」であることを主張する。途中の段で抜いた α を落とす差し替えを入れるとそのテストが赤になることを実際に示す。入力は検体の実物（`R_POST_and_KOMAINU`・`konnoyayame`）を窓口 `sample_ghost_kit::SampleRoot` 経由で取る。製品コードは変えない。

## Introduction

本仕様は、完了 spec `areka-P0-shell-implicit-surface` の要件 4.10（抜き色で透明になった場所のクリックが背後の窓へ抜ける）を**変えずに固定する**ためのテストを足す。brief（同フォルダ `brief.md`・台帳 #53・ウェーブ A1 ④）が要件段階へ送った 3 つの判断——テストの置き場・継ぎ目の要否・相乗り 3 件の採否——は、以下の実測（2026-09-23・main `92f5f448`）で決めた。brief の file:line は起票時の値なので、本書は行番号でなく「そのファイルの何を定義している箇所か」で指す。

### 製品が実際に通る順（実測）

| 段 | 入口（テストが呼ぶもの） | 中で通る箇所 |
|---|---|---|
| 読む・焼く | `crates/areka-emo-present/src/shell_target.rs` の `load_shell_target`（起動側 `build_boot_assets`・採寸側 `build_shell_assets`・examples 3 本が全て通る唯一の入口） | 同ファイルの `build_shell_target` が透過の扱いを `UseSelfAlpha::On` 固定で `SurfaceSet` に載せ、`crates/areka-emo-atlas/src/lib.rs` の `bake` が `Normalizer::key_color` で抜き色を採り、`crates/areka-emo-atlas/src/normalize.rs` の `Normalizer::normalize` の抜き色の腕が左上と 32bit 完全一致の画素を `0,0,0,0` にする |
| 合成する | `EmoPresenter::attach_target`（`crates/areka-emo-present/src/presenter/hub.rs`）→ `EmoPresenter::apply` に `PresentCommand::ShowSurface` | `crates/areka-emo-present/src/presenter/show.rs` の `ShowSurface` 適用が合成先の常設席へ `Composer::compose_into` で合成する（`crates/areka-emo-compose/src/blit.rs` の乗算済み SourceOver 転写） |
| マスクを作る | （同じ `apply` の中） | 同 `show.rs` が交代後の表示バッファの `bytes()` を `FrameBudget::regenerate_mask`（`crates/areka-emo-present/src/presenter/budget.rs`）へ渡し、`MaskRotation::regenerate` が wintf の `AlphaMask::from_pbgra32`／`regenerate_from_pbgra32` でマスクを作り、`cache.insert` の後に窓の面 entity の `AlphaMaskResource::set_shared` へ供給する |
| 窓側（本仕様の外） | — | wintf の `alpha_mask_hit`（`crates/wintf/src/ecs/layout/hit_test/mod.rs`）が同じ面 entity の `AlphaMaskResource::mask()` を読み、`AlphaMask::is_hit`（α ≥ 128 で「内」）で判定し、クリック透過のトグルへ繋ぐ（完了 spec `wintf-clickthrough-alpha-toggle`） |

### 継ぎ目の要否（実測の結論: **新しい公開口は 0**）

上の 3 段は、既存の公開 API と本クレート内テストの既存の読み口だけで端から端まで到達できる。

- 公開 API: `load_shell_target`・`ShellTarget::build_world`／`atlas`・`EmoPresenter::attach_target`／`apply`・wintf の `AlphaMaskResource::mask`・`AlphaMask::is_hit`。
- 既存テストが既に踏んでいる読み口: `crates/areka-emo-present/src/presenter_budget_equivalence_tests.rs` は本物の `apply` を駆動した後、target の装着情報から面 entity を引き、`AlphaMaskResource::mask()` で「当たり判定へ供給されたマスク」を読んでいる。実窓は作らず、窓は `DPI` component を持つ素の entity（`presenter_test_support.rs` の `spawn_window_with_dpi`）、GPU 資源は `make_world_with_gpu`（既存の常時テストと同じ前提）で載せる。
- 読み戻し（GPU → CPU）は要らない: マスクは CPU 上の合成バイトから作られるので、オフスクリーンの D2D ターゲットも読み戻しも **0 回**である。

製品コード（`crates/areka-emo-atlas`・`crates/areka-emo-compose`・`crates/areka-emo-present`・`crates/wintf` の非テストファイル）を触る必要は無い。唯一の細工は、検体の共有受け口（`crates/areka-emo-present/src/shell_target_test_support.rs`・`pub(super)`）が `shell_target` モジュールの外から見えないことで、これはテスト専用コードの可視性の話であり、設計で「可視性を広げる／共有の形を選ぶ」を決める（受け口の複製は作らない）。

### テストの置き場（決定: `crates/areka-emo-present` のクレート内テスト）

- 焼く・合成する・マスクの 3 段すべてに、この crate のテストから届く（上の表）。`crates/areka` に置いても同じ `load_shell_target` と presenter を呼ぶだけで、マスクへの読み口が増えるわけではない。
- `crates/areka-emo-present` に `tests/` フォルダは **0 件**で、`.kiro/steering/structure.md` の規約は「本番ファイルの兄弟ファイル＋`#[cfg(test)] #[path]` の接続宣言」である。既存の presenter 系テスト 15 本が全てこの形で、共有の補助（`presenter_test_support.rs`）もここに在る。
- ファイル名・接続先は設計で決める。

### 検体の実測

| 検体 | 面の絵の PNG | 抜き色の腕を通るか | 面 0 の外形 | 面 10 の外形 |
|---|---|---|---|---|
| `R_POST_and_KOMAINU`（里々） | 10 枚すべて truecolor（IHDR の色種別 2・α 無し） | 通る（既存の `shell_target_template_tests.rs` が合成後の左上の画素の α＝0 を確かめている） | 236×462 | 140×160（`surfaces.txt` に宣言の無い面） |
| `konnoyayame`（YAYA） | 18 枚すべてパレット（色種別 3・`tRNS` 無し） | 通る（完了 spec 要件 4.5・まばたきのコマ 72×30 が抜かれる） | 260×390 | 200×200 |

同じ抜き色の腕を、PNG の形が違う 2 体で踏む。`emo2` は 58 枚中 57 枚が α 付きで抜き色の検体にならない。

### 相乗り 3 件の実測と採否（brief の Scope）

| 候補 | 実測 | 採否 |
|---|---|---|
| ① `areka-emo-compose` の `konnoyayame` の受け口に較正のテストが無い | `crates/areka-emo-compose/src/sample_test_support.rs` は `emo2_root`・`konnoyayame_shell_root` の 2 口を持ち、「指す先が実在する」を判定するテストは **0 本**。`crates/areka-emo-present/src/shell_target_test_support.rs` には同じ形の較正 `every_sample_receptor_points_at_a_real_shell_folder` が在る | **採る**（同じ形を 1 本・要件 3.1） |
| ② 本文走査（`shell::parse` が 0 件の見張り）が examples を見ていない | `crates/areka/src/placement/measure_template_tests.rs` の走査は `measure.rs`・`emo2_boot/assets.rs` の **2 ファイル**だけ。examples で `load_shell_target` を呼ぶのは `crates/areka/examples/emo-present/setup.rs`・`collision-probe/setup.rs`・`window-placement.rs` の **3 本**で、今日の `shell::parse` は 3 本とも **0 件**。走査器が拒む生文字列（`r"`・`r#`）も 3 本とも **0 件**なので、走査器を強くする必要は無い | **採る**（走査対象に 3 本を足す・要件 3.2） |
| ③ `crates/areka/examples/emo-present/setup.rs` の私有 `fn build_shell_target` が公開の `areka_emo_present::build_shell_target` と同名で別物 | 実在する。加えて `crates/areka/examples/collision-probe/setup.rs` にも同名の私有 `fn build_shell_target` が在る（brief は前者だけを挙げていた）。どちらも A1 の他の spec が触るファイルではない（roadmap 干渉台帳） | **採る**（2 本とも改名・要件 3.3） |

## Boundary Context

- **In scope**: 要件 4.10 を「焼く → 合成する → マスク」の順で通す常時テスト 1 本と、その差し替えによる赤の実証。相乗り 3 件（較正 1 本・走査対象 3 本の追加・私有関数 2 本の改名）。
- **Out of scope**: 抜き色の振る舞いそのものの変更（`crates/areka-emo-atlas/src/normalize.rs` の判定・許容幅・対象範囲）／`MaskRotation` の振る舞い／`use_self_alpha` の宣言を読む経路・`.pna`・`full`（全面不透明）／全画素が透明になる面の扱い／wintf 側のマスクからクリック透過のトグルまで（完了 spec `wintf-clickthrough-alpha-toggle` が所有し、自前のテストを持つ）／拡大率 k ≠ 1 の水準（マスクは拡大率に依らず原寸で作られ、÷k は wintf の当たり判定が 1 回だけ掛ける＝完了 spec `areka-P0-present-gpu-transform-scale`。本仕様の水準は 0 本）。
- **Adjacent expectations**: 上流の完了 spec `areka-P0-shell-implicit-surface`（要件 4.10 の正本）と `areka-P0-nar-install`（検体の窓口）は着地済みで、本仕様はどちらの文書も改訂しない。下流の `areka-P0-alpha-release-signoff` は実機一周の「抜かれた場所のクリックが背後へ抜ける」の目視項目を**残す**（本テストは退行を先に止めるもので、実機の確認の代わりではない）。既存の両端のテスト（`crates/areka-emo-atlas/src/normalize_key_color_tests.rs`・`crates/areka-emo-present/src/presenter/budget_tests.rs`）と、実物の絵で合成後の α を見る `crates/areka-emo-present/src/shell_target_template_tests.rs` は、弱めず消さない。

## Requirements

### Requirement 1: 抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト

**Objective:** 開発者として、標準テンプレートの絵が「焼く → 合成する → 当たりのマスク」を通った結果、抜かれた場所がマスクで「外」・残った場所が「内」になっていることを、目視ではなく常時のテストで言い切りたい。それにより、途中のどの段で抜いた α が落ちても、利用者に届く前に赤で止まる。

#### Acceptance Criteria

1. The 通しテスト shall 検体 `R_POST_and_KOMAINU` のシェル（`shell/master`）を、起動側と採寸側が通るのと同じ読み込みの入口（`load_shell_target`）で実物の絵ごと読み、面の表を組み、presenter に target として装着し、`ShowSurface` の指令を適用して面 0 の表示を成立させる。焼く・合成する・マスクを作る の 3 段を、製品と同じ順で、製品と同じ入口から通す。
2. The 通しテスト shall 判定に使うマスクを、wintf の当たり判定が読むのと同じ場所——装着した窓の面 entity に載る `AlphaMaskResource` の `mask()`——から読む。キャッシュのスロットが束ねるマスクだけを読んで済ませない。
3. The 通しテスト shall 「抜かれた画素」の正解を、テスト自身が同じ PNG を復号した生の画素から定める: 左上（座標 0,0）の画素と 4 バイトが完全に一致する画素が「抜かれた画素」、それ以外が「抜かれなかった画素」。正解は正規化・焼き・合成・マスクのどの段の出力からも導かない。
4. When 面 0 の表示が成立した, the 通しテスト shall 面の外形（236×462）の**全画素**について、抜かれた画素の位置ではマスクが「外」（`is_hit` が `false`）、抜かれなかった画素の位置では「内」（`is_hit` が `true`）であることを判定し、食い違う位置の数が **0** であることを主張する。食い違いがあれば、その位置と件数を失敗の文言に出す。
5. The 通しテスト shall 較正として、抜かれた画素の数が 0 より大きく全画素数より小さいことを併せて主張する（全画素が透明・全画素が不透明のどちらでも 4 の主張が恒真になるため）。
6. The 通しテスト shall 同じ判定（3〜5）を、`R_POST_and_KOMAINU` の面 10（140×160・`surfaces.txt` に宣言の無い面）と、`konnoyayame` の面 0（260×390・パレット形式の PNG）にも行う。面 10 は起動時に相方側へ出る面であり、宣言の無い面が同じ扱いを受けることを固定する。`konnoyayame` は PNG の形が違う（パレット）のに同じ抜き色の腕を通ることを固定する。
7. The 通しテスト shall 検体を `sample_ghost_kit::SampleRoot` 経由でのみ取り、`vendors/sample_ghost/` の直パスを 1 か所も綴らない。検体の受け口は既存の共有受け口を使い、テストバイナリの中に受け口の複製を新設しない。
8. The 通しテスト shall 常時テストとして走る: 実窓を作らない、他プロセスの可視窓を作らない、壁時計を合否に使わない、`#[ignore]` も環境変数のゲートも持たない。GPU 資源の前提は既存の presenter 系テストと同一で、GPU からの読み戻しは 0 回である。
9. The 通しテスト shall 名前と説明文で「抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト」であることを平易に述べ、完了 spec `areka-P0-shell-implicit-surface` の要件 4.10 を指す。
10. The areka shall 本テストの追加にあたり、焼く・合成する・マスクを作る各段の製品コード（`crates/areka-emo-atlas`・`crates/areka-emo-compose`・`crates/areka-emo-present`・`crates/wintf` の非テストファイル）を **0 行**変えない。変えてよいのはテストとテスト専用の補助（`#[cfg(test)]` の中）だけである。

### Requirement 2: 途中の段で抜いた α が落ちるとテストが赤になることの実証

**Objective:** 開発者として、足したテストが「今日たまたま緑」ではなく「壊れたら赤」であることを、実際に壊して確かめた記録で示したい。

#### Acceptance Criteria

1. The 実装作業 shall 焼く・合成する・マスクを作る の **3 段それぞれ**に、抜いた α を**経路から外す差し替え**を 1 つずつ当て、要件 1 のテストが**自身の主張（要件 1.4）で**赤になることを実際に走らせて示す。差し替えは「透明を運ぶ段の入力や出力を別の物に置き換える」形とし、座標や値を少しずらす形（平行移動）は用いない。候補は次のとおりで、確定は設計が行う: 焼く＝抜き色の腕が画素を変えずに返す／合成する＝転写が元の α を捨てて全画素を不透明で書く／マスク＝マスク生成へ表示バッファの代わりに全画素不透明のバッファを渡す。
2. The 実装作業 shall 各差し替えについて、差し替えた箇所（ファイルと、その箇所が何を定義しているか）・赤になったテスト名・失敗の文言を記録に残す。併せて、その差し替えで**既存の**テストのうち赤になったものも列挙する（例: 焼く段の差し替えで `normalize_key_color_tests.rs` が赤になるのは想定どおりで、それを隠さない）。
3. When 差し替えを元へ戻した, the 実装 shall 要件 1 のテストと、`cargo test` で走る当該クレートの既存テストが全て緑であることを確かめ、赤の数が **0** であることを記録する。差し替えを製品コードに残さない。
4. The 実装作業 shall 差し替えの記録を、tasks.md の完了記録または検証報告に、後から読み返して再現できる粒度（差し替えの定義・走らせ方）で書く。

### Requirement 3: 同じ完了 spec が残したテストの穴 3 件の相乗り

**Objective:** 開発者として、完了 spec `areka-P0-shell-implicit-surface` が「直さずに残したもの」として書き残した小さな穴を、同じ仕事の中で閉じたい。いずれもテストと example の中に閉じ、製品コードには触れない。

#### Acceptance Criteria

1. The areka shall `crates/areka-emo-compose` の検体の共有受け口（`sample_test_support.rs` の `emo2_root`・`konnoyayame_shell_root`）について、指す先が実在すること（`konnoyayame_shell_root` は `surfaces.txt` を持つフォルダ、`emo2_root` は `shell/master` を持つゴーストのフォルダ）を判定するテストを 1 本持つ。受け口が壊れたとき、それを使う既存テストが「読めない」で赤になるより先に、受け口側の較正が原因を名指しする。
2. The areka shall `crates/areka/src/placement/measure_template_tests.rs` の本文走査（`shell::parse` が本文に 0 件であることの見張り）の対象に、`load_shell_target` を呼ぶ examples 3 本——`crates/areka/examples/emo-present/setup.rs`・`crates/areka/examples/collision-probe/setup.rs`・`crates/areka/examples/window-placement.rs`——を足す。3 本とも今日の本文の `shell::parse` は 0 件で、走査器が拒む生文字列も 0 件なので、走査器そのものは変えない。
3. The areka shall `crates/areka/examples/emo-present/setup.rs` と `crates/areka/examples/collision-probe/setup.rs` の私有 `fn build_shell_target` を、公開の `areka_emo_present::build_shell_target` と取り違えない名前へ改める。改名は名前と呼び出し箇所だけを変え、ロジックの変更は **0 行**である。改名後も `cargo build -p areka --examples` が通る。
4. The areka shall 相乗り 3 件で触るファイルを、`crates/areka-emo-compose` のテスト・`crates/areka/src/placement/measure_template_tests.rs`・上の examples 2 本に限る。A1 で並走する他の spec が触るファイルとの重なりは 0 である（roadmap の干渉台帳）。

### Requirement 4: 変えないもの

**Objective:** 開発者として、この仕事が「固定する」だけで「変える」ことを 1 つもしないと言い切りたい。

#### Acceptance Criteria

1. The areka shall `crates/areka-emo-atlas/src/normalize.rs` の抜き色の判定（左上と 32bit 完全一致・許容幅 0・つながりに依らず全画素）を変えない。
2. The areka shall `crates/areka-emo-present/src/presenter/budget.rs` の `MaskRotation` の振る舞い（輪番・確保の計数）を変えない。
3. The areka shall 既存の両端のテスト（`crates/areka-emo-atlas/src/normalize_key_color_tests.rs`・`crates/areka-emo-present/src/presenter/budget_tests.rs`）と、実物の絵で合成後の左上の α を見る `crates/areka-emo-present/src/shell_target_template_tests.rs` を、弱めず消さない。
4. The areka shall 完了 spec `areka-P0-shell-implicit-surface` の要件 4.10 の文面と、その `.kiro/specs/completed/` 配下の文書を改訂しない。
5. The areka shall wintf 側（`AlphaMaskResource` から先の当たり判定とクリック透過のトグル）に触れない。本仕様の観測点は「窓の面 entity へ供給されたマスク」までで、その先は完了 spec `wintf-clickthrough-alpha-toggle` の所有である。
6. The areka shall `areka-P0-alpha-release-signoff` の実機一周から「抜かれた場所のクリックが背後へ抜ける」の目視項目を外さない。

### Requirement 5: 記録と報告

**Objective:** 開発者として、報告を読むだけで「何が固定され、何が固定されていないか」が分かるようにしたい。

#### Acceptance Criteria

1. The 報告 shall 「抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト」と平易に書き、プロジェクト内の符牒を使わない。
2. The 報告 shall 製品コードの変更行数が 0 であること、GPU からの読み戻しが 0 回であること、拡大率 k ≠ 1 の水準が 0 本であること（理由: マスクは原寸で作られ ÷k は wintf 側）を、数として明示する。
3. The 報告 shall 「実機の確認は不要になった」とは書かない。本テストが止めるのは退行であり、`alpha-release-signoff` の目視は残る。
4. The 報告 shall 要件 2 の差し替えの記録（3 段 × 1 件・各件の赤の証跡・戻した後の赤 0）を含む。

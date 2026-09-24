# Brief: areka-P0-keycolor-clickthrough-coverage

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票（台帳 #53）。完了 spec `areka-P0-shell-implicit-surface` が「直さずに残したもの」として自ら書き残し、**引受先がどこにも無かった**項目を引き受ける。
> 出どころの正本: `.kiro/specs/completed/areka-P0-shell-implicit-surface/tasks.md` の「直さずに残したもの（判断と理由）」の節と、同 `requirements.md` の要件 4 の 10 番。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: 里々／YAYA の標準テンプレートで作られたゴーストを使う第三者——α の利用者の最初の 1 体はほぼ確実にこの形である。

標準テンプレートの絵は α チャンネルを持たず、左上 1 画素と同じ色を抜いて透過させる。要件 4 の 10 番は「抜き色で透明になった場所は、α で透明な場所と同じく『キャラクターの外』として扱う＝クリックは背後の窓へ抜け、撫でや当たり判定の対象にならない」と定める。

**この要件には、壊れたときに赤になるテストが 1 本も無い。** 在るのは両端だけである。

- 抜き色 → α: `crates/areka-emo-atlas/src/normalize_key_color_tests.rs`
- 合成後の画素の α → 当たりのマスク: `crates/areka-emo-present/src/presenter/budget_tests.rs`（マスクを作るのは `crates/areka-emo-present/src/presenter/budget.rs` の `MaskRotation` の `regenerate`）

**両端を繋ぐテストが無い。** 途中のどこか（焼く・合成する・マスクへ渡す）で抜いた α が落ちると、絵の四角い外枠の全面がクリックを食う。利用者から見ると「キャラクターの周りのデスクトップが操作できない」——第三者が最初に出会い、最初に怒る壊れ方である。

2026-09-20 に開発者が実機で目視し、いまは正しく抜けることを確かめた。完了 spec 自身が「『テストで言い切れている』と報告してはならない」と書き、roadmap は「`alpha-release-signoff` の実機一周で見る」と書いたが、**その brief の側に該当の記述は 0 件だった**（片道の申し送り）。目視は今日の正しさを示すだけで、明日の退行を止めない（記憶 correct-behaviour-can-have-no-cage-at-all）。

## Desired Outcome

完了時に次が真になっている。

1. 抜き色の絵を入力に、**焼く → 合成する → 当たりのマスクを作る**までを 1 本の決定論テストが通し、抜かれた画素の位置でマスクが「外」、抜かれなかった画素の位置で「内」であることを主張している。
2. 途中の段で抜いた α を落とす摂動（経路から外す形で。平行移動ではなく——記憶 mutate-by-replacing-not-translating）を入れると、そのテストが赤になることを実際に示してある。
3. 入力は検体の実物（`R_POST_and_KOMAINU` か `konnoyayame`）を窓口 `sample_ghost_kit::SampleRoot` 経由で取る。合成した対照だけでは済ませない（記憶 calibrate-a-zero-with-git-head-not-a-synthesised-control）。

## Approach

**新しいテストファイルだけを足す。製品コードは変えない**のが第一候補である。テストが到達できる継ぎ目が無いと分かった場合に限り、最小の公開口を 1 つ足す（要件段階で実測して決める）。

置き場は、焼く・合成する・マスクの 3 クレートすべてに依存できる場所——`crates/areka` のテストか、`crates/areka-emo-present` の結合テスト——のどちらかを要件段階で選ぶ。GPU の読み戻しが要る場合はオフスクリーンの D2D ターゲットを使う（記憶 gpu-draw-verification-offscreen-d2d-target・areka-no-ci-gpu-tests-in-cargo-test）。

## Scope

- **In**: 要件 4 の 10 番を通しで固定する決定論テスト 1 本と、その摂動の実証。
- **相乗りの候補（要件段階で採否を決める・いずれも同じ完了 spec の残りで、テストの穴）**:
  - `areka-emo-compose` の `konnoyayame` の受け口だけ、較正のテストが無い（他 2 クレートには在る）。
  - 本文走査（`shell::parse(` が 0 件であることの見張り）が本番 2 ファイルだけを見て、examples 3 本を見ていない。
  - `crates/areka/examples/emo-present/setup.rs` の私有 `fn build_shell_target` が、公開の `areka_emo_present::build_shell_target` と同名で別物。
- **Out**: 抜き色の振る舞いそのものの変更／`use_self_alpha` の宣言を読む経路・`.pna`・全面不透明（roadmap「引き受け手の居ない残り」の 1〜3・α 後）／全画素が透明になる面の扱い（同 4・検体に 0 件）。

## Boundary Candidates

- 通しのテスト 1 本（新規ファイル）
- テストが到達するための継ぎ目（要るときだけ）

## Out of Boundary

- `crates/areka-emo-atlas/src/normalize.rs` の抜き色の判定
- `MaskRotation` の振る舞い
- 既存の両端のテスト（弱めない・消さない）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-shell-implicit-surface`・完了 `areka-P0-nar-install`（窓口）。どちらも着地済み＝**いつでも着手できる**。
- **Downstream**: `areka-P0-alpha-release-signoff`（実機一周の項目から「抜かれた場所のクリックが背後へ抜ける」を外せるわけではない——実機の確認は残す。ただし退行はテストが先に止める）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-shell-implicit-surface` の要件 4 の 10 番を**変えずに固定する**。
- **Adjacent**: なし。新規ファイルだけを足すので、**2026-09-20 の実測で同じウェーブの他 spec と共有 0**。相乗りの 3 件を採っても、触るのは `crates/areka-emo-compose` のテスト・本文走査のテスト・`crates/areka/examples/emo-present/setup.rs` で、他 spec は触らない。

## Constraints

- 規模 **XS〜S**（タスク 2〜4 本）。
- テストは「到達する経路」を踏ませる。到達しない腕や、委譲先の同じ関数を直接呼んで済ませない（記憶 cage-must-walk-the-reachable-path）。製品が実際に通る順で、焼く → 合成する → マスク、を通す。
- 常時テストに入れる。実窓・他プロセスの可視窓・壁時計に頼らない。
- 文書と報告では「抜き色で透明になった場所のクリックが背後へ抜けることを固定するテスト」と平易に書く（記憶 no-project-jargon-in-user-facing-docs）。

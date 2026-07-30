# Brief: areka-P0-test-cage-determinism

> **Discovery 日**: 2026-07-30 ／ **ウェーブ**: W6 の後・W7（emo2-conformance-e2e）の前 ／ **規模**: medium-large（ただし**本番ソース面はほぼゼロ**）
> **出自**: `completed/areka-P0-emo-dpi-scaling` の `/kiro-validate-impl` ゲートが「無名の別タスク宛て＝実質未所有」として記録した 4 件（tasks.md:222）。
> 2026-07-30 に**全件の実在を再検証**したところ 4 件とも健在で、うち 1 件は記録より**悪化**していた。担当 spec は 10 本の active spec のいずれにも存在しない。

## Problem

**檻（テスト）が嘘をつき得る状態が 4 系統残っている。** いずれも本番挙動のバグではないが、「緑である」という信号の信頼性を損なう——本プロジェクトは [[deterministic-test-coverage-mandate]] を掲げており、檻の決定性そのものが成果物である。

### ① tracing callsite 毒化ハザード（6 モジュール・44 呼出）

`tracing` の callsite interest cache は**プロセス全体で共有され first-thread-wins**。`with_default` はスレッドローカルだが interest は違うため、先に別スレッドが「このログは不要」と判定を焼き付けると、**捕捉テストがイベント 0 件を静かに観測して緑になる**。硬化済み正典は `crates/areka/src/placement/test_support.rs`（常駐 probe dispatcher 2 個＋捕捉窓の内側で `rebuild_interest_cache()`）。

未硬化サイト（`registry().with(cap)` ＋素の `with_default`・probe 無し・rebuild 無し）:

| # | ファイル（ヘルパ定義） | 呼出数 |
|---|---|---|
| 1 | `crates/areka/src/emo2_boot/adapter.rs`（`capture_logs`） | 2 |
| 2 | `crates/areka/src/emo2_boot/frame.rs`（`capture_logs`） | 8 |
| 3 | `crates/areka/src/emo2_boot/move_cue.rs`（`capture_logs`） | 5 |
| 4 | `crates/areka/src/emo2_boot/spine.rs`（`capture_logs`） | 8 |
| 5 | `crates/areka/src/input_events/balloon.rs`（`capture_logs`） | 18 |
| 6 | `crates/areka/src/shiori_demo.rs`（ヘルパ無し・**inline** `with_default` × 3） | 3 |

**さらに悪いことに、6 箇所とも「スレッドローカル `with_default` ゆえ並行実行でも干渉しない」という誤ったコメントを掲げている**（dispatcher については真だが interest cache については偽）。ハザードが積極的に否認されている状態。

### ② `spine.rs` の協調スピン flake（**8 → 13 箇所へ増加**）

`crates/areka/src/emo2_boot/spine.rs` の反復回数固定スピン（`for _ in 0..100_000 { … yield_now() }` 等）。Defender の再スキャンや並列負荷で飢餓すると**偽赤**になる（[[areka-defender-rescan-starves-cooperative-test-loops]]）。同ファイル内に `Instant` 基準の deadline は**ゼロ**。

記録時 8 箇所 → 実測 13 箇所（行番号は全て陳腐化＝ファイルが育っている）。うち 5 箇所は `for now in 1u64..=200_000` という形で、**ループ変数が Tick 生成子を兼ねている**——単純な find/replace では直せず、Tick カウンタと deadline の分離が要る。

> 参考: `areka-ghost` 側の同型問題は 2026-07-30 に `spin_pumping_ticks`（`Instant` deadline ＋ Tick 注入継続）で根治済み（roadmap 追記㊿）。**その設計がそのまま donor になる。**

### ③ ログ捕捉ハーネスが 3 コピー、しかも**競合する 2 設計が併存**

probe 方式（3 コピー・意味論はバイト等価で命名と prose だけ乖離）:

| | 場所 | 行数 |
|---|---|---|
| 正典 | `crates/areka/src/placement/test_support.rs` | 195 |
| 2 | `crates/areka-emo-text/tests/attach_wiring_test.rs` | ~120 |
| 3 | `crates/areka-emo-present/src/scale.rs` の `mod tests` 内 | ~150 |

**別系統**（global-default keeper 方式・`completed/areka-P0-log-capture-determinism` 由来）: `areka-sylphya/src/test_log_capture.rs`(150)・`areka-kanade/src/schedule/log_capture.rs`(206)・`areka-ghost/src/test_log_capture.rs`(135)。

つまりワークスペースには**硬化の設計が 2 つあり、どちらが正典か決まっていない**。これは重複除去ではなく**設計判断**である。

### ④ `chain.upload` 失敗注入シームの不在

`crates/areka-emo-present/src/chain.rs` の `upload` は `ResizeBuffers`／`GetBuffer`／`Present` 等 5 箇所で実 D3D/DXGI 失敗を返し得るが、`SwapChainPresenter` は trait を持たない具体型で `presenter.rs` が `Option<SwapChainPresenter>` として直接保持——**注入点が存在しない**。唯一の消費点 `presenter.rs:510-514` は失敗時「表示は前状態を保つ」と主張しているが、**この不変条件は未検証**（既存 `upload` テストは成功経路のみ・実 GPU 必須）。

## Current State

- 4 件とも 2026-07-30 に実在確認済み（②は悪化・③は「2 設計併存」という新事実が判明）。
- **active spec 10 本のいずれも所有していない**（全 brief の割当ファイル集合を実測）。`kero-balloon` は `spine.rs` を*証拠として引用*するのみで割当には入れていない。
- 本番ソースへの影響はゼロ（①②③はテスト専用コード。④のみ本番へ小さなシームが要る）。

## Desired Outcome

- ログ捕捉の硬化設計が**ワークスペースで 1 つ**に決まり、全 crate がそれを共有する（コピーはゼロ）。
- 捕捉テストが「イベント 0 件を静かに観測して緑」になり得ない。
- `spine.rs` の待機が全て `Instant` 基準の有界スピンになり、負荷下でも偽赤しない。
- `chain.upload` 失敗時に「表示は前状態を保つ」が**実行テストで証明**される。

## Approach

**檻の決定性を 1 spec で通しで直す。** 4 件はどれも「テストが本当のことを言っているか」という同一の関心であり、①③は同じファイル群を触るため分けると二重作業になる。

1. **③ を先に決める**（設計判断が①の実装形を決めるため）: probe 方式と global-keeper 方式のどちらを正典とするか裁定し、**共有 crate（dev-dependency）または `pub` テスト支援モジュール**へ 1 本化。
2. **① を機械適用**: 6 モジュールのヘルパを削除し共有版へ差し替え（44 呼出の import 書き換え）。`shiori_demo.rs` の inline 3 箇所は先にヘルパ化。**誤ったコメントも全て是正**する（否認が残ると再発する）。
3. **② を変換**: `areka-ghost` の `spin_pumping_ticks` を donor に `spin_until(what, deadline, done)` を導入し 13 箇所を変換。Tick 生成子を兼ねる 5 箇所は Tick カウンタと deadline を分離。
4. **④ にシーム**: `#[cfg(test)]` の fault フラグ（小）か `trait SurfaceUpload` ＋ fake（大・`presenter.rs`/`mount.rs` へ波及）を裁定し、失敗経路の「前状態保持」を檻化。

## Scope

- **In**: 上記 4 件。誤ったコメントの是正。硬化設計の一本化。
- **Out**: 本番の挙動変更（④のシーム以外）。テストの**内容**の変更（既存の判定は保存する——直すのは観測機構と待機機構だけ）。`areka-ghost` 側（2026-07-30 に是正済み）。

## Boundary Candidates

- **硬化設計の裁定と共有化**（③ — 全 crate 横断・最初に片づける）
- **捕捉サイトの追随**（① — `crates/areka/src` 6 モジュール・機械的）
- **待機機構**（② — `spine.rs` 単独ファイル）
- **失敗注入シーム**（④ — `areka-emo-present` の `chain.rs`＋`presenter.rs`）

## Out of Boundary

- 既存テストの**判定内容**を弱めること。観測が正しくなった結果として落ちるテストが出たら、それは**本物の欠陥の発見**であり、檻を緩めて通すのではなく別途起票する。
- `spine.rs` のテストを削ること（[[obsolete-vs-broken-test-policy]]: 退役なら除外・生きているなら更新を自分で判断する）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-log-capture-determinism`（global-keeper 方式の出自）・`completed/areka-P0-emo-dpi-scaling`（probe 方式の硬化版 `test_support.rs`）・`completed/wintf-gpu-test-crash`（GPU テストのオーナースレッド規律）。**PR #92（2026-07-30）の `spin_pumping_ticks` が ② の直接 donor**。
- **Downstream**: `areka-P0-emo2-conformance-e2e`（W7）——e2e の前に檻の決定性が上がっていれば M1 完成宣言の信頼度が上がる。

## Existing Spec Touchpoints

- **Extends**: なし（4 件とも未所有＝新規境界）。
- **Adjacent（⚠️ 衝突あり）**:
  - `areka-P0-kero-balloon`（W5）— `emo2_boot/frame.rs`・`placement/measure.rs` を割当に持つ。本 spec の① site 2 が `frame.rs`、正典ハーネスの消費者が `measure.rs`。**ゆえに W5 と同居不可**（W6 の後に配置する根拠）。
  - `areka-P0-balloon-visibility`（W6）— `emo2_boot/` に新モジュール＋`frame.rs` を触る。**本 spec は W6 完了後に着手**する。
  - `areka-P0-scale-exact-rational`（同時起票）— `areka-emo-present/src/scale.rs` の `mod tests`（③のコピー 3）で**同一ファイル異ハンク**。着手順の裁定が要る。

## Constraints

- **新規外部依存なし**（`tracing-subscriber` は既出）。
- 共有化の実現方法は要設計: `src` の `mod tests` 内・統合テスト・別 crate と**配置がバラバラ**なため、単なる移動では済まない（dev-dependency 用の支援 crate か `#[cfg(feature = "test-support")]` 公開かの判断）。
- [[areka-bin-crate-internal-tests-in-crate]]: `crates/areka` は bin crate ゆえ内部到達テストは in-crate 配置が必須（`tests/` はバイナリ起動型専用）。共有化の形はこの制約を満たすこと。
- 検証は**反復実行**で行う（フレーキーは単発の緑では証明できない）。②は負荷下・並列で最低数十走。

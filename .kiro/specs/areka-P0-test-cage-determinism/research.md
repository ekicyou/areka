# Gap Analysis: areka-P0-test-cage-determinism

> **作成日**: 2026-08-22 ／ **対象ツリー**: HEAD `3f825ab1`（＝`origin/main f6b81078` ＋ spec 文書のみ。コードは main と byte 一致）
> **計測条件**: `crates/**/*.rs` を ripgrep で全数走査（`vendors/` 除外）。本文書の file:line はすべて本ツリーで再検証済み。
> **入力**: `requirements.md`（確定版・不変）・`brief.md`（棚卸⑩まで）・steering（`product.md`／`tech.md`／`structure.md`／`logging.md`／`roadmap.md` 追記(79)〜(81)）。
> **本文書の立場**: 情報と選択肢を示す。最終判断は要件ディスカッション／設計フェーズへ渡す。

---

## 0. 分析サマリ

- **ログ捕捉の硬化（要件 1〜3・8）は「実装の複製を 1 箇所に畳む」だけでは済まない。** ワークスペースには硬化方式が **3 系統**（probe dispatcher 常駐 8 コピー／global-default keeper 3 コピー／一回限りの全スレッド global capture 2 コピー）あり、さらに未硬化の **24 ファイル**（`with_default(` 62 呼出のうち 34 呼出相当）が 5 種類の戻り値形（`Vec<String>` 行・`String` 改行連結・`Vec<LogEvent>` 構造体・WARN/ERROR 件数・EnvFilter 濾過後文字列）で書かれている。共有機構は**捕捉層だけ**を差し替え、呼出側の戻り値形を保てる API 形にする必要がある（要件 2.3）。
- **keeper 方式（bare `registry()` を `set_global_default`）は probe 方式と意味論が異なる**——tracing-subscriber 0.3.23 の `Registry::enabled` は無条件 `true`（`registry/sharded.rs:230-235`）、`register_callsite` は `Interest::always`（同 :222-228）を返すため、keeper を置いたテストバイナリでは**捕捉窓の外のスレッドでも `tracing::enabled!` が真になる**。`wintf::transition` の前置ガード `transition_diag::is_enabled()`（`crates/wintf/src/ecs/window/transition_diag.rs:623`）や `placement/dpi_sync.rs:279` の「既定 OFF なら組立も確保もしない」契約を持つ crate（wintf・areka・areka-emo-present）では keeper 方式を正典にできない。probe 方式は `enabled()` が偽のまま（`NoSubscriber` に委ねる）なのでこの問題が無い。**正典の採否は機序の差で決められる**（後述 §2.2）。
- **共有機構の置き場は依存グラフで決まる。** `wintf` のワークスペース内依存は `dola`・`wintf-winmsg-executor` のみ（`crates/wintf/Cargo.toml` 実測）で `areka-*` を一切持たず、逆に `areka-seriko`／`-kanade`／`-sylphya`／`-ghost`／`-emo-atlas`／`-emo-compose` は `wintf` に依存しない。**どの既存 crate に置いても全消費者から引けない**。有力候補は「ワークスペース内依存を持たない新規 leaf crate を dev-dependency として配る」（A 案）で、`tracing` 直実装なら `tracing-subscriber` にも依存せずに済む（`placement/test_support.rs` 型）。代替として「1 つのソースファイルを `#[path]` で各 crate の `#[cfg(test)]` へ取り込む」（D 案・Cargo 非接触）も成立する。
- **④ upload 失敗注入は `chain.rs:185-241` の 7 箇所が対象だが、現行コードの「失敗時は前状態を保つ」主張は部分的にしか成り立たない見込み**——`ResizeBuffers`（:200）成功後に `create_staging`（:204）／`GetBuffer`（:228）／`Present`（:238）で失敗すると、swap chain は新寸・`source_tex` は新内容（`UpdateSubresource` :214-223 は `?` を持たない）・`self.size` は :204 失敗時のみ旧値のまま、という不整合が残る。注入テストが本物の欠陥候補を掘り当てる可能性が高く、要件 6.2（緩めず起票）の適用先になる。
- **⑦ 錠の退役は `draw-load-parity` 未着手（`.kiro/specs/areka-P0-draw-load-parity/` は brief.md のみ）**のため現時点では保留。呼出は **21 箇所／5 ファイル**（定義行・doc 言及を除く実呼出・要件の「23」は doc 言及込み）。**⑩ 1,000 行番人は 11 ファイル超過**（roadmap 表の 9 本＋`plan_ops_tests.rs` 1,374 行＋`inproc_e2e_test.rs` 1,129 行＋`pilot` example 1,006 行）で、既存の構造走査テスト（`include_str!` 逐語・`read_dir` 走査）が実装の型になる。

---

## 1. 現状調査（Current State）

### 1.1 ログ捕捉サイトの全数インベントリ（`with_default(` 62 呼出／40 ファイル）

判定式: 「`tracing::subscriber::with_default(` を含み、同一ファイルまたは委譲先に `rebuild_interest_cache()`／`ensure_interest_probes()`／`install_interest_keeper()` を持つか」。

#### 1.1.1 硬化済み（16 ファイル・`with_default` 28 呼出）

| # | ファイル | crate | 方式 | 定義 | 戻り値形 | 備考 |
|---|---|---|---|---|---|---|
| 1 | `crates/areka/src/placement/test_support.rs` | areka | probe 2 個＋窓内 rebuild | `ensure_interest_probes` :153・`capture_logs` :166 | `(R, Vec<LogEvent>)`（level＋BTreeMap） | **probe 方式の原典**。`tracing` 直実装（`tracing-subscriber` 不使用）。機序の説明 :6-52 が最も詳しい |
| 2 | `crates/areka/src/placement/diag_tests.rs`・`follow_transition_diag_tests.rs`・`follow_window_move_diag_tests.rs` | areka | 上記 #1 を消費（`rebuild` 直呼び含む） | — | — | 3 ファイル |
| 3 | `crates/wintf/src/ecs/test_support.rs` | wintf | probe 2 個＋窓内 rebuild | `ensure_interest_probes` :64・`capture_under_filter` :96 | `String`（`EnvFilter` 濾過後の fmt 出力） | 呼出 96 箇所。module doc :26-27「wintf は areka に依存できないため同型を持つ」＝**依存方向の制約を自ら述べている** |
| 4 | `crates/areka-seriko/src/log_interest_probe.rs` | areka-seriko | probe 2 個（集約モジュール） | `ensure_interest_probes` :67 | — | `lib.rs:37` で `#[cfg(test)] mod`。消費: `actor_test_support.rs`（`capture_logs` :49・`capture_logs_flow` :39）・`looper_tests.rs` :855・`state_test_support.rs` :15（3 ファイルとも `String` 改行連結） |
| 5 | `crates/areka-emo-atlas/src/log_capture.rs`／`crates/areka-emo-compose/src/log_capture.rs` | atlas／compose | probe 2 個＋窓内 rebuild | `ensure_interest_probes` :126／:125 | `Vec<String>` 行 | `lib.rs:205`／`lib.rs:167` で `#[cfg(test)] mod` |
| 6 | `crates/areka-emo-present/src/balloon_test_support.rs`／`scale_tests.rs` | emo-present | probe 2 個＋窓内 rebuild | `ensure_interest_probes` :153／:398・`capture_events` :172 | `(T, Vec<CapturedEvent>)`（level＋HashMap） | `tracing` 直実装 |
| 7 | `crates/areka-emo-text/tests/attach_wiring_test.rs` | emo-text（統合テスト） | probe 2 個＋窓内 rebuild | `ensure_interest_probes` :321 | `(R, Vec<LogEvent>)` | **統合テストバイナリからは lib 内 `#[cfg(test)]` が見えない**ため複製された＝置き場の制約を示す実例 |
| 8 | `crates/areka-sylphya/src/test_log_capture.rs`／`crates/areka-kanade/src/schedule/log_capture.rs`／`crates/areka-ghost/src/test_log_capture.rs` | sylphya／kanade／ghost | **keeper**（bare `registry()` を `set_global_default`） | `install_interest_keeper` :109／:156／:115 | `Vec<CapturedEvent>`（target・level・message） | 出自 `completed/areka-P0-log-capture-determinism`（DD-1/DD-2）。不変条件「keeper より先に別の global を置いてはならない」を `expect` で強制 |

#### 1.1.2 未硬化（24 ファイル・`with_default` 34 呼出）——要件 Introduction の表と一致

| 区分 | ファイル（定義行） | 戻り値形 | 誤った／不完全な説明文 |
|---|---|---|---|
| 名前付き `capture_logs` 10 | `areka/src/emo2_boot/adapter.rs:383`・`spine.rs:525`・`frame_test_support.rs:122`・`frame_chain_finalize_tests.rs:241`・`move_cue_move_severity_log_tests.rs:43`・`talk_lifecycle_tests.rs:97`・`input_events/balloon_test_support.rs:140`・`input_events/choice_drain.rs:182` | `Vec<String>` 行（`level= target= k=v`。`choice_drain`／`balloon_test_support` は `target=` 無し） | `adapter.rs:358-359`・`spine.rs:499-500`・`talk_lifecycle_tests.rs:72-73`・`frame_chain_finalize_tests.rs:215-216`・`move_cue_…:11`・`frame_test_support.rs:96`（「スレッドローカル `with_default` ゆえ並行実行でも干渉しない」類）・`choice_drain.rs:160`／`balloon_test_support.rs:119`（「最小複製」自認） |
| 〃 | `areka-seriko/src/table.rs:209` | `String` 改行連結 | :207-208「emo-compose/actor.rs の log_capture 流儀・スレッドローカルゆえ並行テスト安全」＝**由来表示も誤り**（compose は硬化済みへ転じ、actor 側は集約モジュール経由） |
| 〃 | `wintf/src/ecs/window_proc/dpi_helpers_tests.rs:345` | `Vec<String>` 行 | :343-344「`set_global_default` は使わない」のみ（否認は無いが rebuild も無い）。**同 crate に硬化済み `ecs/test_support.rs` があるのに使っていない** |
| 別名ヘルパ 7 | `areka-emo-text/src/draw_test_support.rs:61`・`actor_runtime_frame_tests.rs:53`・`sink.rs:170`（`with_log_cage`） | `(T, warns, errors)` 件数 | `sink.rs:168`「`with_default` はスレッドスコープ」（dispatcher については正しいが interest には触れない） |
| 〃 | `areka-emo-text/src/region.rs:400`（`count_warns`）・`wrap.rs:114`／`writing.rs:128`（`resolve_counting_warns`） | `(T, warns)` 件数 | — |
| 〃 | `areka-ghost/src/sink.rs:224`（`capture`） | `Vec<CapturedEvent>` | **同 crate に keeper 方式の `test_log_capture.rs` があるのに使っていない** |
| 直書き 7／29 呼出 | `areka-emo-present/src/presenter_refresh_and_log_tests.rs`(7)・`presenter_perf_log_tests.rs`(6)・`presenter/transition_record_tests.rs`(5)・`presenter/timing_tests.rs`(3) | 共有 `CaptureSubscriber`（`presenter_test_support.rs:499`・`timing_tests.rs:53` に同型 2 定義） | `presenter_refresh_and_log_tests.rs:19-23`「スレッドローカルの既定 subscriber を差すため他テストのイベントを取り込まない」＝混入の話のみで取りこぼしに触れない。**同 crate に probe 方式 `balloon_test_support.rs:172` があるのに使っていない**。自衛規律「陰性主張は同一窓内の陽性 1 本と対にする」は `transition_record_tests.rs` に実装例あり |
| 〃 | `areka-emo-text/src/state_cue_apply_tests.rs`(3)・`layout_cursor_tests.rs`(2) | `WarnCounter` 直書き | — |
| 〃 | `areka/src/shiori_demo.rs`(3・:271/:301/:330) | `Capture` Layer（:244-262）→ `Vec<String>` | — |

**既に硬化済みのファイルに残る旧説明文**（要件 2.4 の「硬化済み側」）: `areka-seriko/src/state_test_support.rs:12-13`・`looper_tests.rs:852-853`・`actor_test_support.rs:37／49`（いずれも「スレッドローカル `with_default` ゆえ並行テスト安全」）。

#### 1.1.3 `with_default` を使わずに global subscriber を置く箇所（要件 1.6 の対象）

| ファイル | 形 | 同一バイナリ内の他 global | 備考 |
|---|---|---|---|
| `crates/areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs:75-93`（`install`・`set_global_default` :81） | 一回限りの全スレッド capture-all（kanade アクタースレッド上の `boot_gate` ログを捕えるため） | 無し（統合テストバイナリ `ghost`） | **スレッド局所捕捉では原理的に代替できない**（別スレッド発火）。共有機構の「全スレッド捕捉」変種として吸収するか、例外表に載せるかの判断が要る |
| `crates/areka-seriko/tests/loop_integration.rs:590-608`（`buffer`・`set_global_default` :602） | 一回限りの全スレッド capture-all | 無し（統合テストバイナリ） | 要件ディスカッション（自明修正）で Introduction ③ へ登記済み。同上 |
| `crates/{areka,wintf,areka-emo-text}/examples/*.rs`・`areka/src/main.rs:130` | `.init()`（本番の Subscriber 初期化） | — | テストではない。検知テスト（要件 8.3）が examples を走査対象に含めるときの**偽陽性源**＝走査語を `with_default(` に限定するか、`.init()` を除外表へ |

#### 1.1.4 呼出規模（参考）

`capture_logs(` 238・`capture_logs_flow(` 18・`capture_under_filter(` 96・`capture_events(` 7・`with_log_cage(` 19・`resolve_counting_warns(` 12・`count_warns(` 3・`capture(` 69（kanade／sylphya／ghost の keeper 型 `capture` を含む）。**差し替えの手は多いが、各ファイルの差分は「ヘルパ本体の削除＋`use` 1 行」に収束する**（戻り値形を保てば呼出側は無改変）。

### 1.2 硬化方式 3 系統の機序差（③ の裁定材料）

tracing-core 0.1.36（`Cargo.lock` 実測）・tracing-subscriber 0.3.23。

| 観点 | probe 2 個常駐＋窓内 rebuild | keeper（bare `registry()` を global） | 一回限りの全スレッド capture-all global |
|---|---|---|---|
| 焼き付きを止める機序 | `has_just_one` を恒久的に偽にし `Rebuilder::JustOne`（`callsite.rs:544-546`）＝`NoSubscriber` の `never` が入る経路を塞ぐ。probe の `register_callsite` は `sometimes` | live dispatcher が 0 にならない＝`interest.unwrap_or_else(Interest::never)`（`callsite.rs:505`）に落ちない。Registry の interest は `always` | 同左（keeper と同じ帰結）＋全スレッドのイベントを 1 バッファへ |
| 捕捉窓の外のスレッドでの `tracing::enabled!` | **偽のまま**（interest `sometimes` → 現スレッドの `NoSubscriber.enabled()`＝偽） | **真になる**（`Registry::enabled` は無条件 `true`・`sharded.rs:230-235`） | 真になる |
| 既定 OFF の前置ガードを持つ本番コードへの影響 | 無し | `transition_diag::is_enabled()`（`wintf … transition_diag.rs:623`）・`dpi_sync.rs:279`・`show.rs:347` の `observe_surface` が捕捉窓の外でも真→**行の組立・確保が走る**。「既定で無音」「定常アロケーション 0」の前提が同一バイナリ内で崩れ得る | 同左 |
| 同一バイナリ内で別の `set_global_default` と両立するか | **両立する**（global を触らない） | しない（先着のみ・後着は `expect` で panic） | しない |
| 捕捉窓内のイベントを取りこぼす残余経路 | probe 常駐**前**に焼かれた `never` → 窓内 `rebuild_interest_cache()` で解消（原典 :46-48） | keeper 確立前の焼き付き → 確立時の全走査 rebuild で解消 | 同左 |
| 外部依存 | `tracing` のみで実装可（原典は `tracing-subscriber` 不使用） | `tracing-subscriber`（`registry()`） | `tracing-subscriber` |
| 現存コピー数 | 8（§1.1.1 #1・#3〜#7） | 3（#8） | 2（§1.1.3） |

**含意**: 要件 1.6「両立しない既存 subscriber 設置は寄せるか、違反時に明示失敗」は、正典が probe 方式なら **ghost e2e global probe／seriko loop_integration の global capture-all と構造的に両立**し、残る作業は「全スレッド捕捉が本当に要る 2 箇所」の扱いだけになる。正典が keeper 方式なら、wintf／areka／emo-present の前置ガード契約と衝突する（Research Needed R-1 で実測確認）。

### 1.3 依存グラフと共有機構の置き場候補

実測（`crates/*/Cargo.toml` の path 依存）:

- `wintf` → `dola`・`wintf-winmsg-executor` のみ。**`areka-*` 依存ゼロ**。dev-deps に `tracing-subscriber` あり。
- `wintf` に依存する crate: `areka`・`areka-emo-present`・`areka-emo-text`。
- `wintf` に依存**しない**消費者: `areka-seriko`・`areka-kanade`・`areka-sylphya`・`areka-ghost`・`areka-emo-atlas`・`areka-emo-compose`。
- `tracing-subscriber` を dev-deps に持たない消費者: `areka-emo-present`（`tracing` 直実装で捕捉している）。
- 統合テスト（`tests/`）を持つ消費者: `areka-emo-text/tests/attach_wiring_test.rs`・`areka-ghost/tests/ghost/*`・`areka-seriko/tests/loop_integration.rs`。**lib 内 `#[cfg(test)]` モジュールは `tests/`・`examples/` から不可視**（attach_wiring_test.rs が複製を持つ理由）。
- 新規 crate はワークスペース `members = ["crates/*"]` に自動包含される。テスト専用 crate の先例: `shiori4-testdll`・`shiori-host32-testdll`（`cdylib`）。

| 候補 | 可視性（in-crate cfg(test)／tests/／examples/） | wintf の依存規律（要件 1.3） | Cargo 接触 | 評価 |
|---|---|---|---|---|
| **A. 新規 leaf crate**（ワークスペース内依存ゼロ・deps は `tracing` のみ、必要なら `tracing-subscriber` を feature） | ○／○／○ | 名前に `areka-` を付けると「`areka-*` への依存」と字面で衝突する。**命名を裁定**（例: 接頭辞無し／`wintf-` 系／中立名） | 消費 11 crate の `[dev-dependencies]` へ各 1 行（brief 追記(81) と roadmap :87 が想定済み・`draw-load-parity` は Cargo 非接触の見込み） | 最有力。`wintf` の `capture_under_filter`（EnvFilter）は wintf 側に薄い包みを残すか、crate の feature で提供 |
| B. `wintf` の `pub` モジュール（feature `test-support`） | ○／○／○（wintf 依存 crate のみ） | 規律は満たす | seriko／kanade／sylphya／ghost／atlas／compose が `wintf`（windows・bevy を引く）を dev-dep に取る＝層違反・ビルド負荷 | 不採用候補 |
| C. `dola` の `pub` モジュール（feature） | ○／○／○（dola 依存 crate のみ） | 規律は満たす | `dola` に `tracing`（＋subscriber）を optional で追加。sylphya／kanade／atlas／compose は dola 非依存→dev-dep 追加。演出定義ライブラリにテスト支援を同居させる | 不採用候補 |
| D. 単一ソースファイルを `#[path = "…/test_support/log_capture.rs"] mod …;` で各 crate の `#[cfg(test)]` と `tests/` から取り込む | ○／○／○ | 満たす（crate 依存が増えない） | Cargo 非接触。相対パスが crate 位置に依存・`rustfmt`／IDE が未追跡・`use` の前提（`tracing_subscriber` が要る形なら消費側 dev-deps 必要） | 成立するが保守性が劣る。A の代替 |
| E. 現状維持（crate ごとのコピーを byte 一致で揃える＋一致テスト） | ○ | 満たす | 無し | 要件 1.1「定義箇所 1 箇所」・1.5「不採用方式 0 件」と両立しない |

### 1.4 ② 反復回数固定の待機（現状 2 箇所）

- `crates/areka/src/emo2_boot/spine.rs` 本体: 壁時計上限の無いループは **0 箇所**。`spin_wait_until` :358（`SPIN_WAIT` 30 秒 :329・`SPIN_YIELD_BUDGET` 1,000,000 :347・`BACKOFF_SLEEP` 1ms :350）。module doc :5-22 が「純粋ポーリング／ハイブリッド」の 2 形と「負検証の settle drain だけは従来どおり `yield_now` のみ」を明文化＝**意図的温存の宣言**。
- 残 2 箇所:
  - `spine_display_tests.rs:410-414`: `for now in 1_000_000u64..1_000_000 + 5_000 { harness.inject_dispatcher_tick(now); received.extend(…drain_received()); yield_now(); }`＝**ループ変数が注入 Tick を兼ねる**（要件 4.3）。
  - `spine_seriko_loop_tests.rs:372-375`: `for _ in 0..5_000 { emitted.extend(…drain_received()); yield_now(); }`（外側 `for now in [1000,…,5000]` :369 は seriko tick 注入）。
- donor: `crates/areka-ghost/tests/ghost/spine_e2e_test.rs:48` `spin_pumping_ticks(what, now: &mut u64, send_tick, done)`＝壁時計 deadline（`E2E_BOUND`）＋ `done` 述語＋ Tick 単調前進。**「尽きるのが正常」の settle にはそのままでは使えない**（`done` が偽のまま期限切れ→ panic）。
- 既存の有界形の先例（同ディレクトリ）: `spine_display_tests.rs:30-36`（deadline＋件数）・`spine_seriko_loop_tests.rs:54-66／84-101／187-209／421-435`（deadline＋200µs poll-backoff）・`spine_move_cue_tests.rs:117-128`。

### 1.5 ④ upload 失敗経路（現状）

- `crates/areka-emo-present/src/chain.rs` `SwapChainPresenter`（struct :122・`upload` :185-241）。`?` 7 箇所: **寸法変更** :200 `ResizeBuffers`・:203 `create_source_tex`・:204 `create_staging`／**資源取得** :211 `source_tex` cast・:228 `GetBuffer(0)`・:231 backbuffer cast／**提示** :238 `Present(0)`。すべて `device_err`（:40-47・`error!` ＋ `PresentError::Device`）経由＝要件 5.3 のログは既に満たす。
- **状態遷移の順序**: :200 resize 成功 → :203 `source_tex` 差替 → :204 `staging` 差替 → :205 `size` 更新 → :214-223 `UpdateSubresource`（`?` 無し）→ :228 → :238。
  - :203 失敗: swap chain だけ新寸（`size` は旧値）。
  - :204 失敗: swap chain 新寸・`source_tex` 新寸（空）・`size` 旧値。`read_back()`（:243-）は `size` 旧値で `staging` 旧寸へ `CopyResource` する＝寸不一致。
  - :228／:238 失敗（寸変更あり）: `size` 新値・`source_tex` は新内容済み・backbuffer 未更新／未提示。`read_back()` は**新内容**を返す。
  - :211／:228／:231／:238 失敗（寸不変）: `source_tex` は新内容済み・表示は旧（:228 以前）または新内容コピー済みで未提示（:238）。flip model の `Present` 失敗時に backbuffer 内容がどう見えるかは D3D 側の契約に依存。
  - → **「表示は前状態を保つ」は :200（resize 前）と :211（書込前）でのみ自明。** 要件 5.2 を実行テストで固定すると、少なくとも寸変更を伴う後段失敗で主張が破れる可能性が高い（Research Needed R-4・要件 6.2 の起票先）。
- 消費点: `presenter/show.rs:306-310`（観測点・不動）。失敗時は `reply(Err)` → `return` で `target.visible`／`applied`／`native_size`／`current_surface` を書かない（:320-400 の更新はすべて upload 成功後）。`prev_size = chain.size()` :305 と本文走査テスト `transition_record_tests.rs:327-347`（逐語 :339）が字面を固定。
- 保持者: `presenter/target.rs:73` `chain: Option<SwapChainPresenter>`（具体型・trait 無し）。生成点は `show.rs:251`・`mount_test_support.rs:60`。
- 既存テスト: `chain.rs:420-465` `upload_read_back_roundtrip_and_resize`（成功経路のみ・`GraphicsCore::new()` は `D3D_DRIVER_TYPE_HARDWARE` 固定 `wintf/src/ecs/graphics/core.rs:120`・WARP フォールバック無し）。「headless」＝窓無し・実 D3D デバイス、が既存前提。
- **注入の選択肢**（要件 5.4／5.5 を同時に満たす必要）:
  - ④-a `#[cfg(test)]` の失敗注入点（スレッド局所 `Cell<Option<FaultAt>>` 等）を `upload` の 7 箇所の直前に置く。本番ビルドでは消える（5.5 自動充足）。`chain.rs` 単独接触・`show.rs` 非接触。`upload` 本文に `#[cfg(test)]` 行が 7 箇所入る＝可読性のコスト。
  - ④-b `trait SurfaceUpload` ＋ 偽実装を `target.chain` の型へ（`Box<dyn …>` または generic）。`target.rs:73`・`show.rs:251／306`・`mount_test_support.rs` へ波及。本番に動的ディスパッチ 1 段（定常経路の性能差は無視できる見込みだが要件 5.5 の「変えない」の解釈が要る）。
  - ④-c `#[cfg(test)]` の enum ラッパ `ChainSlot { Real(SwapChainPresenter), Fake(FakeChain) }` を `target.chain` の型にする。本番では `Real` 一択に縮退する `cfg` 分岐。`show.rs` の `chain.upload(…)`／`chain.size()` の字面は保てる（メソッド名同一）が型が変わる。
  - ④-d 実失敗を誘発（例: 外部で `GetBuffer(0)` の参照を保持したまま寸変更→ flip model の `ResizeBuffers` が `DXGI_ERROR_INVALID_CALL`）。**寸法変更の分類だけ**本物の失敗で踏める。資源取得・提示は誘発が難しい→ ④-a〜c と併用する補助案。
  - 「前状態」の観測手段: `read_back()`（`source_tex` 経由＝**backbuffer ではない**）・`size()`・`target.visible`／`current_surface`／`mount.set_bounds` の呼出有無。**backbuffer の実表示内容は read_back では観測できない**点を設計で明示する必要がある。

### 1.6 ⑦ `lock_self_initiated_for_test()`（現状）

- 定義 `crates/wintf/src/ecs/window/command.rs:76`（`#[cfg(test)]`・`static Mutex<()>`・Mutex の poisoned 状態は無視して取得）。対象カウンタ `SELF_INITIATED_DEPTH` :49（`AtomicI32` プロセス共有）。
- 実呼出 **21 箇所／5 ファイル**: `command.rs` 2・`command_batch_tests.rs` 5・`command_transition_tests.rs` 4・`window_pos_tests.rs` 5・`window_pos_transition_tests.rs` 5（要件の「23」は doc 言及 2 行を含む計数）。
- `draw-load-parity` は **未着手**（spec ディレクトリに brief.md のみ・spec.json 無し）。要件 7.1 の「着地前は現状維持」が現時点の状態。退役の手順は「`Cell<i32>` 化が取り込まれた後、錠の取得行を削除し要件 9 の反復で 0 失敗を示す」＝機械的。定義の削除は所有者へ申し送り（7.4）。

### 1.7 ⑩ 1,000 行番人（条件付き）

- 現行ツリーの 1,000 行超 **11 ファイル**（`wc -l`）: `areka-emo-present/src/cache_tests.rs` 1,618・`areka-emo-compose/src/plan_ops_tests.rs` 1,374・`areka-seriko/src/actor_bind_loop_tests.rs` 1,336・`areka/src/emo2_boot/frame_transition_branch_tests.rs` 1,255・`areka/src/placement/follow/window_move.rs` 1,227・`areka-ghost/tests/ghost/inproc_e2e_test.rs` 1,129・`areka-emo-present/src/presenter/budget_tests.rs` 1,081・`areka-seriko/src/bind.rs` 1,043・`areka/src/placement/transition_judge_tests.rs` 1,039・`transition_judge_verdict_tests.rs` 1,037・`pilot/examples/pilot-clickthrough-alpha-toggle/main.rs` 1,006。
- 置き場の技術的制約: 番人テストは `crates/**` を走査するため **crate 境界を跨いで読む**。`env!("CARGO_MANIFEST_DIR")` から `../..` を辿る形は既存先例あり（`areka-emo-present/src/balloon_test_support.rs:194` `emo2_balloon_root` が `../pilot/examples/...` を参照）。置き場候補は ⒜ 共有 leaf crate の自己テスト（A 案採用時・ワークスペース横断の関心と合う）／⒝ `areka` の in-crate テスト／⒞ どれか 1 crate の `tests/`。
- 構造走査テストの先例: `include_str!` 逐語（`frame_harness_tests.rs:39`・`transition_record_tests.rs`・`areka-emo-text/src/lib.rs:171-183`・`wintf … transition_diag_tests.rs`）・`read_dir` 走査（`balloon_target_tests.rs:276`・`areka-ghost/tests/ghost/snapshot_capture_test.rs`）。要件 8.3「兄弟ファイル `<stem>_*.rs`・`tests/`・`examples/` を含める」は `read_dir` 再帰で自然に満たせる。

### 1.8 `ReassertZOrder` 再表示シーム（要件 11.4）

- 挿入点は wintf 内 `zorder_pair_establish.rs:180`（確立時 1 発）のみ。areka 側の本番コードに `ReassertZOrder` の挿入は無い（`spawn.rs:616` は doc 参照のみ）。再表示経路 `emo2_boot/balloon_visibility_phase.rs:385` → `EmoPresenter::show_target`（`presenter/visibility.rs:69`）は Z 順の再断行を要求しない。
- **決定論テストで「再表示直後に隣接が保たれる」を固定するには、再表示で `ReassertZOrder` を挿す本番配線が先に要る**（本仕様の Out of scope＝本番挙動変更）。本仕様で可能なのは ⒜ 現状を「再表示は再断行を要求しない」と純関数／配線テストで**記録**する（主張の内容を固定しない観測）か、⒝ 理由付きで e2e へ申し送る（11.4 後段）。隣接の実測自体は実窓（`CreateWindowExW`）が要るため決定論の外。

### 1.9 その他の発見

- `bindoption-exclusivity` 登記の 6 テストは現存: `actor_bind_loop_tests.rs:117／469`・`actor_dispatch_tests.rs:710／764`・`looper_tests.rs:774`・`state_bind_pattern_tests.rs:709`。いずれも硬化済み `log_interest_probe` 経由（要件 3.7 は反復の証跡待ち）。
- `areka-emo-present` は `tracing-subscriber` を dev-deps に持たない。A 案で crate の既定実装を `tracing` 直実装にすれば追加依存なしで差し替えられる（wintf の `EnvFilter` 濾過だけが `tracing-subscriber` を要する）。
- 捕捉結果の 5 形（行 `Vec<String>`／改行連結 `String`／構造体 `Vec<…Event>`／件数／濾過後文字列）は**いずれも「1 イベント＝level・target・fields」から派生できる**。共有機構は構造体の列を正とし、各 crate の薄いアダプタ（行整形・件数）を残す形が要件 2.3 と整合する。ただしフィールド名の表記（`level=WARN target=…`／`level=WARN` のみ／`HashMap` と `BTreeMap`）が crate ごとに違うため、**アダプタは現行の文字列形を byte 一致で再現する**必要がある（assert が文字列照合のため）。
- `kero-balloon` 申し送りの 553/1 単発赤はテスト名不明のまま。要件 9.5 は「①硬化後の反復で再現しなくなるか」の記録のみを求める。

---

## 2. 要件→資産マップ（Requirement-to-Asset Map）

| 要件 | 既存資産 | ギャップ | 種別 |
|---|---|---|---|
| 1.1-1.2 定義 1 箇所・全 crate から利用 | probe 原典 `placement/test_support.rs`・wintf `ecs/test_support.rs`・seriko `log_interest_probe.rs` | 全消費者から引ける置き場が無い（§1.3）。統合テストから lib 内 cfg(test) が不可視 | Missing |
| 1.3 wintf に areka-* 依存を持ち込まない | wintf 依存グラフ実測 | 新規 crate の命名／配置で字面衝突の可能性 | Constraint |
| 1.4 新規外部依存なし | `tracing` 直実装の原典あり | — | — |
| 1.5 不採用方式 0 件 | keeper 3・capture-all 2・probe 8・直書き | keeper の `Registry::enabled=true` 意味論と前置ガード契約の衝突可能性（R-1） | Unknown |
| 1.6 両立しない global の扱い | ghost e2e global probe・seriko loop_integration | 全スレッド捕捉の需要は本物（別スレッド発火）。共有機構の変種か例外表か | Unknown |
| 1.7 判定結果を変えない | — | keeper→probe 移行で `enabled!` の真偽が変わるテストが無いか（R-1） | Unknown |
| 2.1-2.2 再計測・24 ファイル移行 | §1.1 の表 | 機械的。`frame_test_support.rs`（FrameHarness 捕捉層）は 11.3 により捕捉層のみ差替 | — |
| 2.3 派生ヘルパの捕捉層委譲 | `capture_logs_flow`（seriko）・`with_log_cage`・`count_warns`・`CaptureSubscriber` 系 | 戻り値形 5 種の byte 一致アダプタ（§1.9） | Constraint |
| 2.4-2.5 説明文・由来表示の是正 | §1.1.2 の列挙＋硬化済み側 3 ファイル | 文面の置き換えのみ | — |
| 2.6・8.1-8.4 直接 `with_default` 残存 0 の機械証明・検知テスト | `include_str!`／`read_dir` 走査の先例（§1.7） | 走査対象に `tests/`・`examples/` を含めると `.init()` の偽陽性（§1.1.3）。例外表の形 | Missing |
| 3.1-3.2 取りこぼさない・焼き付き解消 | probe 原典の機序説明 :6-52 | — | — |
| 3.3 不在主張の対照観測 | `transition_record_tests.rs` の「陽性と対にする」実装例・`transition_diag_tests.rs:587-640` | 共有 API としての提供形（例: 捕捉窓内で必ず 1 件出す自前 probe イベント、または `assert_captured_something`） | Missing |
| 3.4 自己テスト（焼き付け下で捕捉／素の捕捉では取りこぼす較正） | 原典の説明のみ・実行テスト無し | **意図的に `never` を焼き付ける手段**（別スレッドで `NoSubscriber` のまま同一 callsite を踏む・probe 導入前の状態を作る）が要る。probe は `OnceLock` プロセス寿命ゆえ「硬化を外した素の捕捉」は**同一プロセスでは再現しにくい**→ 別プロセス（`std::process::Command` で自身のテストバイナリを `--exact` 起動）か、probe 導入前に必ず走る順序制御が要る | Unknown（R-2） |
| 3.5 TRACE 含む全レベル | probe 原典 `enabled()=true`・wintf は EnvFilter 指定次第 | — | — |
| 3.6 スレッド局所意味論 | `with_default` 維持 | 全スレッド捕捉変種（1.6）とは別 API に分ける | — |
| 3.7・9.2 seriko 6 テスト 30 回反復 | テスト現存 | 証跡のみ | — |
| 4.1-4.6 待機の有界化 | `spin_wait_until`・先例 6 形・donor `spin_pumping_ticks` | 「尽きるのが正常」の settle 向けヘルパ（最小持続 or 連続空回数）が無い。Tick 兼用形の分離 | Missing |
| 5.1 注入手段 | 無し（具体型・trait 無し） | ④-a〜d（§1.5） | Missing |
| 5.2 前状態保持 | 主張のみ | 現行コードで破れる経路がある見込み（§1.5） | Unknown（R-4） |
| 5.3 失敗を返しログ | `device_err` | — | — |
| 5.4 headless | 既存 GPU テストの前提（HARDWARE 固定・窓無し） | 注入は実 GPU 失敗を要しない形に | Constraint |
| 5.5 本番不変・アロケーション 0 | `budget` 系テスト（`presenter_budget_steady_state_tests.rs`） | ④-b の動的ディスパッチは「性能特性を変えない」の解釈次第 | Constraint |
| 5.6 観測点不動・本文走査被覆 | `transition_record_tests.rs:327-347` | `show.rs` 非接触が最も安全（④-a/④-c） | Constraint |
| 6.x 判定内容保存・起票 | — | 5.2 の発見を起票する導線 | — |
| 7.x 錠の退役 | 21 呼出／5 ファイル・dlp 未着手 | dlp の着地待ち（7.1 の状態） | Constraint |
| 9.x 反復証跡 | PowerShell・i686 前提（`workspace-test-needs-i686-host32-artifacts`） | 反復スクリプト／ログ保存の形 | — |
| 10.x 1,000 行番人（条件付き） | 11 ファイル・走査先例 | 開発者裁定待ち。置き場は A 案 crate の自己テストが自然 | Constraint |
| 11.1-11.2 語彙不変条件・キュー側計数 | `transition_diag_tests.rs:362`・`frame_transition_atomicity_tests.rs` module doc | 共有機構の自己テストで `wintf::transition` 行を出すなら同規律 | — |
| 11.4 `ReassertZOrder` | 挿入点 1 箇所・再表示経路に無し（§1.8） | 決定論で固定できる範囲は「現状の記録」まで | Unknown |
| 11.5 dlp と共有ファイル 0・Cargo は dev-deps のみ | — | A 案なら dev-deps 11 行 | — |
| 11.6 利用手順の文書 | — | 設計文書／crate doc | — |

---

## 3. 実装アプローチ（Options）

### Option A: 新規 leaf crate へ共有機構を新設し、全捕捉サイトを差し替える（新設＋機械移行）

- **構成**: `crates/<name>/`（ワークスペース内依存ゼロ・`tracing` 必須・`tracing-subscriber` は feature または既定）。公開 API の骨子: `ensure_interest_probes()`（冪等）／`capture<T>(f) -> (T, Vec<Event>)`（probe＋窓内 rebuild・スレッド局所）／`Event { level, target, fields: BTreeMap }`＋行整形ヘルパ（`level= target= k=v` 形を byte 一致で再現）／件数ヘルパ／（feature）`capture_under_filter(directives, f)`／（別 API）全スレッド capture-all の一回限り global（ghost e2e・seriko loop_integration 用・`set_global_default` を唯一ここに閉じ込める）／自己テスト（要件 3.4・8.4・条件付き 10.3）。
- **消費側**: 11 crate の `[dev-dependencies]` に 1 行・各ファイルのヘルパ本体削除＋`use`。`wintf` は `ecs/test_support.rs` を薄い包み（EnvFilter）に縮小。
- **適合**: 1.1／1.2／1.3（命名次第）／1.4／1.5／2.x／8.x を素直に満たす。
- **トレードオフ**: ✅ 可視性の問題が消える（tests/・examples/ から引ける）✅ 検知・番人・較正の自己テストの置き場が同時に決まる ❌ crate が 1 つ増える（`structure.md` の crate 一覧更新）❌ 命名で「`areka-*`」の字面を避ける裁定が要る ❌ `dola`／`wintf` 以外の全 crate の Cargo を触る（dev-deps のみ）。

### Option B: 既存 crate を拡張（`wintf` または `dola` に feature 付き `pub` テスト支援を置く）

- **構成**: `wintf` に `pub mod test_support`（feature `test-support`）。既存 `ecs/test_support.rs` を昇格。
- **適合**: 1.3 は満たす。**1.2 を満たせない**（seriko／kanade／sylphya／ghost／atlas／compose は wintf 非依存→ dev-dep 追加で windows／bevy を引く）。`dola` 案は演出ライブラリへの異物混入。
- **トレードオフ**: ✅ 新 crate 無し ❌ 層規律違反・ビルド時間増 ❌ `logging.md:123`「ライブラリは Subscriber 初期化しない」との境界説明が要る（テスト支援とはいえ `pub`）。**不採用候補**。

### Option C: ハイブリッド（単一ソースファイルを `#[path]` 取り込み＋自己テストは 1 crate に集約）

- **構成**: `crates/_shared/log_capture.rs`（仮）を各 crate の `#[cfg(test)] #[path = "../../_shared/log_capture.rs"] mod log_capture;` と `tests/` 側の `#[path]` で取り込む。定義ファイルは 1 つ。自己テストは areka in-crate か seriko に置く。
- **適合**: 1.1（ファイル 1 つ）／1.2／1.3／1.4 を満たす。Cargo 非接触（11.5 に最も軽い）。
- **トレードオフ**: ✅ 依存グラフ不変 ❌ 相対パスが crate 配置に依存・ツール追跡外 ❌ `tracing_subscriber` を使う形なら消費側 dev-deps が要る（`tracing` 直実装なら不要）❌ 「定義箇所 1 箇所」は満たすが「crate として引く」自然さに欠け、後続 spec の利用手順（11.6）が説明しにくい。**A の代替**。

### ②・④・⑦・⑩ は A/C いずれでも独立

- ②: `spine.rs` 近傍に「最小持続 or 連続空観測回数」の settle ヘルパを 1 本追加（Tick 注入は呼出側クロージャで注入・注入時刻は観測に頭打ち）。2 箇所を差し替え。
- ④: ④-a（`#[cfg(test)]` 注入点・`chain.rs` 単独）を第一候補、④-c（enum ラッパ）を次点、④-b（trait）は波及大。④-d は寸法変更の本物失敗を 1 本足す補助。
- ⑦: dlp 着地後に 21 行削除＋反復。
- ⑩: 裁定⒜なら A 案 crate の自己テスト（または areka in-crate）に `read_dir` 再帰＋例外表 11 件。

---

## 4. 工数・リスク

| 項目 | 工数 | リスク | 根拠 |
|---|---|---|---|
| ③ 共有機構新設（A 案）＋自己テスト（3.4 較正・8.4）＋全スレッド変種 | M | Medium | 機構は原典の移植。較正テスト（素の捕捉で取りこぼす再現）の作り方が未確定（R-2）。全スレッド変種の両立条件の明文化 |
| ① 24 ファイル移行＋説明文是正＋派生ヘルパのアダプタ | M | Low〜Medium | 機械的だが文字列形 byte 一致の再現が要る。judge は `cargo test --workspace` 緑維持＋反復 |
| ⑧ 検知テスト（`with_default` 直接呼出の走査＋例外表＋較正） | S | Low | 走査先例あり。examples の `.init()` を除外 |
| ② settle 2 箇所の有界化 | S | Low | 先例 6 形・donor あり。assert 無改変 |
| ④ 注入点＋前状態保持テスト | M | Medium〜High | 現行コードの主張が破れる経路がある見込み（R-4）＝「テストを書いたら赤」→ 要件 6.2 の起票で閉じるが、設計討議で「どこまでを前状態と呼ぶか」の定義が要る |
| ⑦ 錠の退役 | S | Low（外部依存 High） | dlp の着地時期に依存 |
| ⑩ 番人（条件付き） | S | Low | 裁定待ち |
| ⑨ 反復証跡（10 回 workspace・30 回 seriko／待機） | S〜M（壁時計） | Low | ワークスペース 5,636 テスト×10 回の所要時間 |

合計見積り: **M〜L**（④ の発見次第で設計討議が 1 往復増える）。

---

## 5. 設計判断事項（要件ディスカッションへ渡す・番号付き）

1. **正典方式の裁定**: probe 2 個常駐＋窓内 rebuild（§1.2 の機序差により `enabled!` 前置ガードと両立）か、keeper（global bare registry）か。keeper を選ぶ場合は wintf／areka／emo-present の「既定 OFF」契約との両立条件を明文化する必要がある。
2. **共有機構の置き場**: A（新規 leaf crate・dev-deps 配布）／C（`#[path]` 単一ファイル）。A なら **crate 名**（`areka-` 接頭辞を避けるか・要件 1.3 の字面）と `tracing-subscriber` を既定依存にするか feature にするか。
3. **全スレッド捕捉（一回限り global capture-all）の扱い**: ghost e2e S7（`spine_e2e_test_global_log_probe.rs`）と seriko `loop_integration.rs` は別スレッド発火を捕える本物の需要。共有機構の**別 API**として吸収する（`set_global_default` を共有 crate に一元化）か、例外表に理由付きで残すか。
4. **捕捉結果の正準型と各 crate アダプタ**: `Event { level, target, fields }` を正とし、行整形（`level= target= k=v`・`target=` 無し変種・`HashMap`／`BTreeMap`）と件数の薄いアダプタを crate 側に残すか、共有 crate が全形を提供するか。assert の文字列照合を byte 一致で保つ責務の所在。
5. **不在主張の対照観測（3.3）の既定形**: 共有 API が捕捉窓内で自前の対照イベント（例: 捕捉開始時に 1 件 `trace!`）を必ず出して「捕捉が働いていた」を自動で示すか、呼出側が陽性 1 本を書く規律（`transition_record_tests.rs` 型）を API で強制するか。
6. **較正テスト（3.4-b「素の捕捉では取りこぼす」）の再現方法**: probe は `OnceLock` プロセス寿命ゆえ同一プロセスで「硬化前」に戻せない。別プロセス起動（自身のテストバイナリを `--exact` で呼ぶ）／probe 導入前に走る順序制御／`never` を意図的に焼き付ける別スレッド手順、のいずれを採るか。
7. **検知テスト（8.1-8.4）の走査語と例外表**: 走査語は `with_default(` のみか `set_global_default(`・`.init()` も含めるか。examples の本番初期化は除外。例外表の形式（`const` 配列で明示編集を要求）。
8. **② settle の有界化の形**: 「壁時計の最小持続」か「連続して空だった観測回数」か。Tick 兼用形（`spine_display_tests.rs:410`）で注入時刻の頭打ち値をどう置くか（`injected-sim-time-must-not-outrun-observation`）。
9. **④ 注入手段**: ④-a（`#[cfg(test)]` 注入点・`chain.rs` 単独）／④-c（enum ラッパ）／④-b（trait）。および「前状態」の定義（`read_back` は `source_tex` を読む＝backbuffer ではない・`size()`・presenter 側の `visible`／`current_surface`／`set_bounds` 未呼出）。
10. **④ で現行コードの主張が破れた場合の扱い**（要件ディスカッション議題 2 で**裁定済み＝要件 5.7／5.8**）: `chain.rs` の内側で閉じる是正は本仕様で行う（`show.rs` 非接触・定常アロケーション 0 維持）。`chain.rs` の外へ波及する場合のみ起票し、テストは現状を記録する形で残す。設計では是正の形（`upload` の状態更新順序＝失敗し得る操作を先に済ませてから `size`／`source_tex` を差し替える等）と「前状態」の観測手段を決める。
11. **⑦ 錠の退役の待ち方**: dlp 着地を本仕様のブランチへ取り込む時点（wave 内合流 or rebase）と、見送り時の 7.3 適用の判断点。
12. **⑩ 番人の置き場**（採否は要件ディスカッション議題 1 で **⒜ 採用に確定**・例外表 11 件は `pilot` example・`inproc_e2e_test.rs` を含めて開始・既存ファイルの分割は本仕様で行わない）: 置き場（共有 crate 自己テスト／areka in-crate）のみ設計で決める。
13. **11.4 `ReassertZOrder` 再表示**: 本仕様で「現状（再表示は再断行を要求しない）」を配線テストで記録するに留めるか、全面的に e2e へ申し送るか。再表示で要求を挿す本番配線は本仕様の範囲外。
14. **wintf の `capture_under_filter`（EnvFilter 濾過）の扱い**: 共有 crate の feature で提供するか、wintf 側に薄い包み（probe は共有・濾過は wintf）を残すか。96 呼出の import 影響。

---

## 6. Research Needed（設計フェーズへ持ち越す未確定）

- **R-1** keeper→probe 移行（sylphya／kanade／ghost lib テスト）で判定が変わるテストが無いか。特に `tracing::enabled!` の真偽に依存するコード（現行は wintf `transition_diag.rs:623`・areka `dpi_sync.rs:279`・emo-present `show.rs:347`）が keeper 側 3 crate のバイナリに含まれないことの確認（依存グラフ上 sylphya／kanade／ghost は wintf 非依存＝含まれない見込み）。逆に、A 案を keeper で実装した場合に wintf／areka／emo-present で `is_enabled()` が真になる実測。
- **R-2** 較正テスト（3.4-b）の決定論的な再現手段（設計判断 6）。tracing-core の `has_just_one` が一度偽になると戻らない性質（`callsite.rs:551-558`）ゆえ、同一プロセス内再現は順序依存になる。
- **R-3** `Interest::sometimes` 常態化の副作用: probe 常駐後は全 callsite で毎回 `enabled()` が評価される。テストバイナリ限定だが、`cargo test --workspace` の所要時間への影響（体感 0 の見込み・要実測）。
- **R-4** ④ の前状態保持が破れる経路の実測（§1.5）。flip model で `Present` 失敗時に前フレームが残るかの D3D 契約。`read_back()` が観測できる範囲（`source_tex`）と表示内容の乖離。
- **R-5** 9.1 反復 10 回の所要時間と Defender 再スキャン（`areka-defender-rescan-starves-cooperative-test-loops`）下の負荷条件の再現方法。PowerShell・i686 成果物前提。
- **R-6** 例外表／走査の対象に `vendors/`・`pilot` を含めるか（`pilot` はワークスペース member・`with_default` 0 件・1,006 行 example あり）。

---

## 7. 推奨（設計フェーズへの引き渡し）

- **正典は probe 方式**（機序差 §1.2・既存 8 コピー・`tracing` 直実装で依存ゼロ）を第一候補とし、keeper 3 crate を probe へ寄せる。全スレッド capture-all は共有 crate の別 API に閉じ込め `set_global_default` の呼出点を 1 つにする。
- **置き場は A 案（新規 leaf crate）**。名前は `areka-*` の字面を避けるか、要件 1.3 を「依存方向の規律」と読み替えて登記する。`tracing-subscriber` は feature（wintf の EnvFilter 濾過用）。
- **④ は ④-a（`#[cfg(test)]` 注入点・`chain.rs` 単独）**で `show.rs` を非接触に保ち、前状態保持の破れは要件 6.2 で起票する前提を設計に書く。
- **②** は settle 専用ヘルパ（最小持続＋連続空観測の両立形）を `spine.rs` に 1 本。
- **⑦・⑩** は外部裁定・dlp 着地に従属する条件付きタスクとして tasks.md に分離。
- 要件ディスカッションで §5 の 14 項目を順に裁定する。

---

# 設計フェーズの調査記録（Research & Design Decisions・2026-08-22 追記）

> **対象ツリー**: HEAD `6ca62a40`（コードは `origin/main f6b81078` と byte 一致・spec 文書のみ差分）。本節の file:line は設計生成時に Grep／Read で再検証した（§1〜§7 の値と一致。差異があった項目は本節で明記）。
> **Discovery Scope**: Extension（既存テスト基盤の統合・整理）＋ 局所的な本番接触 1 箇所（`chain.rs`）。外部ライブラリの新規導入は無く、WebSearch による外部調査は行っていない（`tracing` 0.1.44／`tracing-core` 0.1.36／`tracing-subscriber` 0.3.23 は `Cargo.lock` 実測・機序は §1.2 と原典 `placement/test_support.rs:6-52` の説明に依る）。
> **要件ディスカッションの裁定（再掲・設計は再開しない）**: §5 項目 10（④ で主張が破れたら `chain.rs` 内で是正＝要件 5.7／5.8）・項目 12（1,000 行番人は採用・見張りのみ・例外表 11 件＝要件 10）・実装は開発者の改善ループのマージ後に開始（要件 Adjacent expectations）。

## 8. Summary

- **Feature**: `areka-P0-test-cage-determinism`
- **Key Findings**:
  - 正典は **probe 方式**（`tracing` 直実装・`set_global_default` 不使用）。keeper 方式は `Registry::enabled` が無条件 `true` のため、同一バイナリで `tracing::enabled!` の前置ガード（`wintf/src/ecs/window/transition_diag.rs:622-623` `is_enabled`・`areka/src/placement/dpi_sync.rs:279`・`areka-emo-present/src/presenter/show.rs:347`）の「既定 OFF」を崩す。probe 方式は `enabled!` を偽のまま保つ。
  - 置き場は **新規 leaf crate**（A 案）。名前は `log-capture-kit`（`crates/log-capture-kit/`・lib 名 `log_capture_kit`）。`areka-` を冠さないことで `wintf` の `[dev-dependencies]` を `areka-*` 非依存のまま字面でも保ち、既存モジュール名（`test_log_capture`・`log_capture`）とも衝突しない（同名だと `use` が crate／module で曖昧になる）。
  - 捕捉結果の正準型は **発火順を保つ** `Vec<(String, String)>` フィールド列でなければならない。既存の行整形（`level={} target={} k=v…`／`level={} k=v…`）は `record()` の訪問順で組まれており、`BTreeMap`／`HashMap` を正準にすると byte 一致で再現できない（`areka/src/emo2_boot/frame_test_support.rs:103-117`・`input_events/choice_drain.rs:162-177`・`wintf/.../dpi_helpers_tests.rs:326-341` で確認）。
  - 3.4-b（硬化を外した素の捕捉で取りこぼす較正）は**同一プロセスでは再現不能**（probe は `OnceLock` でプロセス寿命・`has_just_one` は二度と真にならない）。**自身のテストバイナリを子プロセスで起動**し、`#[ignore]` 付きの子テストを `--exact --ignored` で走らせる形が決定論的に成立する（子は環境変数で bare／hardened の両モードを受け、親は出力の `1 passed` を確認して「0 件実行の空振り」を排除する）。
  - ④ `upload`（`chain.rs:185-241`）は **prepare → commit** に並べ替えるだけで、7 経路中 6 経路で「失敗時の状態不変」が成立する（テクスチャ生成 → `ResizeBuffers` → 3 フィールド一括更新／`cast`・`GetBuffer`・`cast` → `UpdateSubresource` → `CopyResource` → `Present`）。残る `Present` 失敗だけは `source_tex` が新内容で未提示（`read_back()` は試行内容を返す）——これを「前状態」の定義（表示＝backbuffer・presenter 状態）の外の残余として記録する。
  - ⑦は `draw-load-parity` 未着手（spec ディレクトリは `brief.md` のみ）。⑩は 11 ファイル超過を `wc -l` で再確認（§1.7 と同一）。

## 9. Research Log（設計フェーズ）

### 9.1 keeper 方式を正典にできない根拠（R-1 の決着）
- **Context**: §5 項目 1（正典方式）。
- **Sources**: `tracing-subscriber` 0.3.23 `registry/sharded.rs:222-235`（§1.2）・`wintf/src/ecs/window/transition_diag.rs:622-623`・`areka/src/placement/dpi_sync.rs:279`・`areka-emo-present/src/presenter/show.rs:347`・`presenter/timing.rs:14`／`presenter_perf_log_tests.rs:886`（「前置ガードを置かない」構造の実測）。
- **Findings**: keeper を置いたバイナリでは `tracing::enabled!` が全スレッドで真になる。現行で keeper を持つ 3 crate（sylphya／kanade／ghost）は wintf 非依存なのでガード付き本番コードを含まず（依存グラフ §1.3）、**現状では判定が変わるテストは無い**。しかし正典を keeper にすると wintf／areka／emo-present のテストバイナリへ keeper が入り、`transition_diag::is_enabled()` が捕捉窓の外で真→観測行の組立と確保が走る（「既定で無音」「定常アロケーション 0」の前提が同一バイナリで崩れる）。逆に keeper 3 crate を probe へ寄せても `enabled!` に依存するテストは無い（keeper 3 crate のテストは `capture()` の戻り値のみを見る）。
- **Implications**: 正典＝probe 方式。keeper 3 コピー・全スレッド capture-all 2 コピーは kit の API へ畳む（§10 D1）。

### 9.2 置き場の最終比較（A vs C）
- **Context**: §5 項目 2。
- **Findings**: `#[path]` 単一ファイル（C 案）は in-crate `#[cfg(test)]`（`src/` 深さが crate ごとに違う）と `tests/`（別バイナリ）で相対パスが異なり、`rustfmt`／IDE が追跡しない。`tracing_subscriber` を使う形なら消費側の dev-deps は結局要る。統合テスト 3 本（`areka-emo-text/tests/attach_wiring_test.rs`・`areka-ghost/tests/ghost/*`・`areka-seriko/tests/loop_integration.rs`）と bin crate `areka`（`src/lib.rs` 無し・`[[bin]]` のみ＝in-crate テスト必須）の両方から同じ形で引けるのは crate だけである。新規 crate はワークスペース `members = ["crates/*"]` に自動包含され、テスト専用 crate の先例（`shiori4-testdll`・`pilot`）がある。
- **Implications**: A 案採用。消費側は **10 crate**（`areka`・`wintf`・`areka-seriko`・`areka-kanade`・`areka-sylphya`・`areka-ghost`・`areka-emo-atlas`・`areka-emo-compose`・`areka-emo-present`・`areka-emo-text`＝`with_default(` を持つ crate の全数・§1.1 の 40 ファイルの所属を集計）の `[dev-dependencies]` に各 1 行。§1.3 の「11 crate」は本集計で 10 に確定した。

### 9.3 捕捉結果の形と既存 assert の結合
- **Findings**: 5 形の整形はすべて `record_debug` の `{value:?}` と `record()` の訪問順で決まる。`level={level} target={target}` の先頭＋` {name}={value:?}` の連結（seriko 3・atlas・compose・areka emo2_boot 6・`frame_test_support`）と `target=` 無し変種（`choice_drain`・`input_events/balloon_test_support`・`dpi_helpers_tests`・`shiori_demo`）の 2 形で**行整形は尽きる**。構造体形は `placement`（`BTreeMap`・`message()`／`field()`）・emo-present（`HashMap`・`message()`／`field_names()` 昇順）・keeper 3 crate（`target`／`level`／`message`／`event`／`outcome`／`fields`）。件数形は emo-text（WARN／ERROR 件数）。
- **Implications**: kit の正準型は `CapturedEvent { level, target, fields: Vec<(String, String)> }`（訪問順保持）＋ `message()`／`field(name)`／`field_names_sorted()`／`fields_map()`。行整形 2 形は kit が byte 一致で提供（`LineFormat`）。crate 固有の派生（`event`／`outcome` の取り出し・`assert_logged*`・`count_level`）は**各 crate の薄いアダプタに残す**（要件 2.3＝判定内容不変）。呼出側の 238 箇所の `capture_logs(` は無改変で通る。

### 9.4 不在主張の対照観測（3.3）の既定形
- **Findings**: 呼出側へ「陽性 1 本を対にせよ」と規律で求める形（`transition_record_tests.rs` 型）は新設サイトで守られる保証が無い（brief が 3 度実証した再発構造と同じ）。kit の `capture()` が窓の内側で自前の番兵イベント（`trace!(target: "log_capture_kit::sentinel")`）を 1 件発火し、窓の終了時に捕捉列へ番兵が在ることを検査（無ければ panic＝「捕捉が働いていない」）して**返却前に取り除く**形なら、呼出側の assert・戻り値は不変のまま全サイトが自動で対照を持つ。
- **Implications**: 既定 API に番兵を内蔵（§10 D4）。`capture_under_filter`（EnvFilter 濾過）では directive に `log_capture_kit::sentinel=trace` を足し、返却文字列から番兵行を除く（`is_empty()` 型の不在主張が壊れない）。

### 9.5 較正テスト（3.4-b）の再現手段（R-2 の決着）
- **Findings**: `tracing-core` の `Dispatchers::has_just_one` は `register_dispatch` で `len <= 1` に書かれる（probe の `Arc` を `OnceLock` が保持する限り `len >= 2`）。したがって probe 常駐後のプロセスでは「硬化前」へ戻せない。再現に必要な場面は**「捕捉窓の内側で、購読者を持たない別スレッドが同じ発行点をプロセス内で最初に踏む」**（素の `with_default` では `Rebuilder::JustOne` → その別スレッドの既定 `NoSubscriber` → `Interest::never()` が焼き付く→テストスレッドの発火が捨てられる）。窓の**前**に焼かれた `never` は素の `with_default` でも入口の `Dispatch::new`（登録＋全走査再計算）で治るため、較正場面は「窓内先着」でなければならない。
- **Implications**: kit の統合テスト 1 本が `std::env::current_exe()` を `--exact <子テスト名> --ignored --test-threads=1` で起動し、環境変数 `AREKA_LOG_CAPTURE_CALIBRATION=bare|hardened` で子のモードを切り替える。子は `#[ignore]`・環境変数が無ければ即 return（`--include-ignored` の誤実行で親プロセス内で走っても probe の有無に依存しない）。bare 子は捕捉 0 件を、hardened 子は 1 件を assert。親は子の stdout に `test result: ok. 1 passed` を要求（0 件実行の空振りを排除＝道具の較正）。

### 9.6 ④ `upload` の状態遷移と「前状態」の定義（R-4 の決着）
- **Sources**: `chain.rs:185-241`（`upload`）・`:244`（`read_back`）・`:292`（`size`）・`:122`（struct）・`presenter/show.rs:305-310`（観測点・`prev_size`／エラー分岐）・`presenter/target.rs:73`・`presenter/transition_record_tests.rs:327-347`（本文走査）・`mount_test_support.rs:60`（生成点）。
- **Findings**: 現行順序は `ResizeBuffers`(:192-200) → `source_tex` 差替(:203) → `staging` 差替(:204) → `size`(:205) → `UpdateSubresource`(:214-223・失敗しない) → `cast`(:211 は UpdateSubresource の前) → `GetBuffer`(:228) → `cast`(:231) → `Present`(:236-238)。**是正後の順序**（設計バリデーション指摘 1 を反映）: 寸法変更時は `create_source_tex`（ローカル） → `create_staging`（ローカル） → `ResizeBuffers`；共通部 `cast`（新または現 `source_tex`） → `GetBuffer` → `cast` → **commit（寸法変更時のみ 3 フィールド一括代入・`UpdateSubresource` の直前）** → `UpdateSubresource` → `CopyResource` → `Present`。失敗し得る操作のすべてが commit より前に終わるので、`Present` 以外の 6 点で struct の 4 項目（`size`／`source_tex`／`staging`／寸の記録）は旧値で自己整合。swap chain 実体だけは `ResizeBuffers` 成功後に戻せない（後段失敗時は backbuffer が新寸・未描画＝残余 ⒝・次回 `upload` で回復）。（当初案「`ResizeBuffers` 直後に commit」は寸法変更経路の `cast`／`GetBuffer` 失敗で `size` 新・`source_tex` 空の不整合を残すため退けた。）`Present` 失敗時は `source_tex` と backbuffer に新内容が書かれ未提示（表示＝直前フレームのまま・`read_back()` は新内容）。これを「前状態を保つ」の外に置く残余として記録する——復元には提示済み内容の複製テクスチャが要り、定常経路に毎フレーム 1 `CopyResource` を足す（要件 5.5 の性能特性に抵触）ため採らない。
- **「前状態」の定義**（設計で固定）: ⒜ `SwapChainPresenter` 内部＝`size()` 不変・`read_back()` が成功し（`source_tex`／`staging` の寸が一致）・`Present` 以外の失敗では `read_back()` の内容も不変／⒝ presenter 側＝`target.visible`／`applied`／`native_size`／`current_surface`／`mount.set_bounds` 未呼出・`reply` が `Err`（`show.rs:306-310` の早期 return）。backbuffer の実表示内容は readback 不能（flip model）のため観測外と明記する。
- **Implications**: 注入は `#[cfg(test)]` で実体化するスレッド局所の一発フラグ（`chain.rs` 内 `fault_point(UploadFault::…)?` を 7 点の直前に置く・非 test ビルドでは常に `Ok(())` の空関数）。`show.rs` 非接触・`target.rs` 非接触。ログは既存 `device_err` 経由（要件 5.3）。

### 9.7 ② settle の有界化の形
- **Sources**: `spine.rs:329`（`SPIN_WAIT` 30s）・`:347`（`SPIN_YIELD_BUDGET`）・`:350`（`BACKOFF_SLEEP`）・`:358`（`spin_wait_until`）・`spine_display_tests.rs:410-414`・`spine_seriko_loop_tests.rs:369-375`・先例 `spine_display_tests.rs:28-40`（deadline＋200µs poll）。
- **Findings**: 2 箇所はどちらも「尽きるのが正常」。負荷下で縮まない回収機会は「壁時計の最小持続」**かつ**「連続して空だった観測回数」の両立で与えるのが最も単純（片方だけだと、極端に速い／遅い環境でどちらかが空振りする）。Tick 兼用形（`spine_display_tests.rs:410`）は、設計バリデーション指摘 3 を受けて **Tick 注入を前段へ完全分離**する——旧範囲 `1_000_000..+5_000` を毎回すべて注入（各 Tick 後に drain・待機なし）し、その後に `settle_bounded(|| drain)` を置く。注入範囲は壁時計に依存せず旧テストと同一、時刻は観測より先に進まない。（当初案の「`step` 内で Tick 注入＋頭打ち」は、壁時計次第で注入 Tick 数が 5,000 に届かず模擬時間の被覆が縮むため退けた。）
- **Implications**: `spine.rs` に `settle_bounded(step)` を 1 本追加（§10 D6）。

### 9.8 錠の退役（⑦）と `command.rs` の扱い
- **Findings**: `lock_self_initiated_for_test()` の定義は `command.rs:76`、呼出は `command.rs` 2・`command_batch_tests.rs` 5・`command_transition_tests.rs` 4・`window_proc/window_pos_tests.rs` 5・`window_proc/window_pos_transition_tests.rs` 5＝**21**（設計時に `rg -c` で再確認）。`draw-load-parity` は spec ディレクトリに `brief.md` のみ（未着手）。ワークスペースに `-D warnings`／`deny(warnings)` の設定は無い（`.cargo/` 無し・`Cargo.toml`／`lib.rs` に無し）ため、定義だけが残って `dead_code` 警告になっても `cargo test` は赤にならない。
- **Implications**: 退役は dlp の `Cell<i32>` 化が本ブランチへ取り込まれた後の条件付きタスク。兄弟テストファイル 4 本の 19 呼出を退役し、`command.rs` 内の 2 呼出と定義は「dlp 着地後に並走衝突が消えている」場合のみ呼出 2 箇所を退役・定義は要件 7.4 に従い申し送り。

### 9.9 ⑧ 検知・⑩ 番人の走査方式
- **Findings**: `include_str!` 逐語は列挙漏れで被覆が縮む（要件 8.3 の懸念そのもの）。`read_dir` 再帰（先例 `balloon_target_tests.rs:276`・`snapshot_capture_test.rs:205`）と `CARGO_MANIFEST_DIR` 相対でワークスペース根を辿る先例（`balloon_test_support.rs:194`）を組み合わせれば `crates/**/*.rs`（`src/`・`tests/`・`examples/`・兄弟ファイル）を漏れなく走査できる。走査語は `with_default(`・`set_global_default(`・`set_default(`（`tracing::dispatcher` 経由の別名を含む）。`.init()` は examples の本番初期化 14 箇所（`crates/*/examples/*.rs`・`areka/src/main.rs:130`）に在るため走査語に含めない。コメント行（`//`・`//!`・`///`）は除いて走査する（説明文中の言及が偽陽性にならないように）。kit 自身のディレクトリは走査対象外（正典の定義箇所）。
- **Implications**: kit の `tests/` に走査器（純関数＋ walker）を置き、⑧・⑩・「kit は dev-dependencies 以外に現れない」の 3 検査を同じ walker で行う。

### 9.10 11.4 `ReassertZOrder` 再表示
- **Findings**: 挿入点は `wintf/src/ecs/window/zorder_pair_establish.rs:180`（確立時 1 発）のみ。areka 側の本番配線に再表示時の挿入は無く（`placement/spawn.rs:616` は doc 参照・`spawn_zorder_pair_*_tests.rs` は確立時の配線テスト）、再表示経路 `emo2_boot/balloon_visibility_phase.rs:385` → `EmoPresenter::show_target`（`presenter/visibility.rs:69`）は Z 順を扱わない。
- **Implications**: 決定論テストで固定できる範囲は 0（固定対象の本番配線が無い＝本仕様の Out of scope）。全面的に `emo2-conformance-e2e` へ理由付きで申し送る（§10 D9）。

## 10. Design Decisions（設計フェーズ）

### D1: 正典＝probe 方式・keeper／capture-all／直書きは kit の API へ畳む
- **Alternatives**: keeper 正典／probe 正典／両立併存。
- **Selected**: probe 正典。全スレッド捕捉は kit の**別名 API** `install_global_capture_all()`（`set_global_default` の唯一の呼出点・一度きり・衝突時は `expect` で明示失敗）。
- **Rationale**: §9.1。**Trade-offs**: `Interest::sometimes` 常態化でテストバイナリの全 callsite が毎回 `enabled()` を訊く（R-3・9.1 の反復で所要時間を測る）。
- **Follow-up**: keeper 3 crate の移行後に `cargo test -p areka-sylphya/-kanade/-ghost` が全緑（要件 1.7）。

### D2: 置き場＝新規 leaf crate `log-capture-kit`
- **Alternatives**: §1.3 A〜E。**Selected**: A。deps は `tracing` のみ・feature `env-filter` で `tracing-subscriber`（wintf の `capture_under_filter` 用）。`publish = false`。
- **Rationale**: §9.2。**Trade-offs**: crate が 1 つ増え `structure.md` の crate 一覧に追記が要る。消費 10 crate の `Cargo.toml` に dev-deps 1 行（要件 11.5 の範囲内）。
- **Follow-up**: kit の検知テストが「`[dependencies]` に `log-capture-kit` が現れない」ことも走査する（依存方向の規律を機械化）。

### D3: 正準イベント型は訪問順保持・値は Debug 表現と `record_str` 生値の両方・行整形 2 形を kit が byte 一致で提供・crate 固有派生はアダプタに残す
- **Rationale**: §9.3。値型は `FieldValue { debug, str_raw: Option<String> }`（設計バリデーション指摘 2 の反映＝kanade／ghost の `assert_logged` は `record_str` の生値で完全一致判定しており、Debug 表現だけでは引用符剥がしの再実装が要る）。**Trade-offs**: アダプタ（`capture_logs` 等の既存名）は各 crate に残るが、本体は kit への 1〜3 行の委譲になる（「定義箇所 1 箇所」は**硬化機構**についての要件であり、整形アダプタは対象外＝要件 1.1 の読み）。

### D4: 不在主張の対照観測＝kit が番兵イベントを内蔵
- **Rationale**: §9.4。**Trade-offs**: 窓ごとに TRACE 1 件の発火（コスト無視可）。`capture_under_filter` は directive 追加＋行除去で同じ保証を得る。

### D5: 較正テスト＝子プロセス方式
- **Rationale**: §9.5。**Trade-offs**: テスト 1 本が子プロセスを 2 回起動（数百 ms）。Windows で `current_exe()` は安定。

### D6: ② settle ヘルパ＝「最小持続 かつ 連続空観測」・Tick 注入は前段で決定論的に分離
- **Rationale**: §9.7。**Selected**: `settle_bounded(step: FnMut() -> usize)`（`step` は 1 反復分の回収のみを行い件数を返す・Tick 注入は含めない）。Tick を兼ねていた `spine_display_tests.rs:410` は前段で旧範囲 `1_000_000..+5_000` を毎回すべて注入（各 Tick 後に drain）してから `settle_bounded(|| drain)` を呼ぶ（設計バリデーション指摘 3 の反映＝注入範囲を壁時計に依存させない）。終了条件＝最小持続 `SETTLE_MIN`（壁時計）を満たし**かつ**連続 `SETTLE_QUIET_ROUNDS` 回 0 件。上限 `SPIN_WAIT` で必ず返る（返った後の判定は呼出側の assert・ヘルパは panic しない）。反復間は `BACKOFF_SLEEP` 相当の短い sleep（4.4 の有界 poll-backoff）。
- **Trade-offs**: 2 テストの所要が各 `SETTLE_MIN` 分伸びる（値は tasks で実測調整・初期値 200ms／quiet 50 回を提案）。

### D7: ④ 注入＝`#[cfg(test)]` スレッド局所の一発フラグ＋ prepare→commit 並べ替え（要件 5.7 の是正）
- **Rationale**: §9.6。**Selected**: `UploadFault` 7 値（分類: 寸法変更 `ResizeBuffers`／`CreateSourceTex`／`CreateStaging`・資源取得 `SourceTexCast`／`GetBuffer`／`BackbufferCast`・提示 `Present`）。`fault_point(at) -> Result<(), PresentError>` は test ビルドでスレッド局所 `Cell<Option<UploadFault>>` を消費し、一致すれば `device_err("<injected …>")(E_FAIL)` を返す。非 test ビルドは常に `Ok(())`。
- **Trade-offs**: `upload` 本文に 7 行の `fault_point(...)?;` が入る（可読性のコスト・各行は 1 行）。残余 2 件（⒜ `Present` 失敗後の `read_back()` は試行内容を返す・⒝ 寸法変更経路の `ResizeBuffers` 成功後の後段失敗は backbuffer だけ新寸・未描画）を文書化して残す。commit は `UpdateSubresource` の直前（設計バリデーション指摘 1 の反映）。

### D8: ⑦・⑩ は条件付き／独立タスク・⑧は kit の統合テスト
- **Rationale**: §9.8・§9.9。

### D9: 11.4 は全面的に e2e へ申し送り
- **Rationale**: §9.10。

## 11. Risks & Mitigations（設計フェーズ）

- **R-3 `sometimes` 常態化の所要時間**: 9.1 の 10 回反復で `cargo test --workspace` の所要を記録し、移行前（main）と比較する。
- **行整形の byte 一致が崩れて assert が落ちる**: 移行は crate 単位で行い、各 crate の lib テストを移行直後に走らせる。`LineFormat` の単体テストに既存 4 形の逐語期待値を置く。
- **`capture_under_filter` の出力差**: directive 追加は番兵 target のみに効く。wintf の 96 呼出がそのまま回帰スイートになる。
- **④ の `Present` 残余を「前状態保持」と読み違える**: テスト名と assert メッセージで「未提示の試行内容」と明記し、design §Components に残余として登記。
- **dlp の着地が本仕様の完了より遅れる**: 要件 7.3 により錠温存で完了できる（申し送りに登記）。
- **`--include-ignored` で較正の子テストが親プロセスで走る**: 子は環境変数が無ければ即 return。

## 12. References（設計フェーズ）
- `crates/areka/src/placement/test_support.rs:6-52` — probe 方式の機序説明（原典）。
- `crates/areka-sylphya/src/test_log_capture.rs:98-136`・`crates/areka-kanade/src/schedule/log_capture.rs:145-184`・`crates/areka-ghost/src/test_log_capture.rs:119-131` — keeper 方式。
- `crates/areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs:75-93`・`crates/areka-seriko/tests/loop_integration.rs:590-608` — 全スレッド capture-all。
- `crates/areka-emo-present/src/chain.rs:185-241` — `upload`。`crates/areka-emo-present/src/presenter/show.rs:305-310` — 観測点。
- `crates/areka/src/emo2_boot/spine.rs:329-375` — 有界待機の先例。

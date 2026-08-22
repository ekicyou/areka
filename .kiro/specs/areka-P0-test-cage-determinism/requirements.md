# Requirements Document

## Introduction

本ワークスペースは「テスト可能な領域はすべて実行テストで固定する」方針（deterministic-test-coverage-mandate）を掲げている。その方針の前提は「緑は本当に緑であり、赤は本当に赤である」こと＝**テストの決定性そのもの**である。ところが現在のツリーには、テストが本番挙動とは無関係に嘘をつき得る状態が 4 系統残っている。いずれも本番の欠陥ではないが、`cargo test --workspace` という全 spec 共通の完了ゲートの信号を損なう。

本仕様（W6.9・`draw-load-parity` と 2 本並走）はこの 4 件を 1 本で通しで是正する。4 件はどれも「テストが本当のことを言っているか」という同一の関心であり、①と③は同じファイル群を触るため分けると二重作業になる。

### 4 系統の現状（2026-08-22・現行ツリー `main` 同等 `f6b81078` で再計測。brief の旧数値と異なる箇所は併記）

**① ログ捕捉テストがテスト間で汚染される**——`tracing` の callsite interest キャッシュはプロセス全体で共有され、最初にその発行点を踏んだスレッドの判定が焼き付く。`with_default` で差し込んだ捕捉 subscriber はスレッド局所だが interest は違うため、別スレッドが先に「このログは不要」と焼き付けると、後続の捕捉テストはイベントを 1 件も観測できなくなる。症状はテストの書き方で両側に出る: 「このログは出ない」を主張するテストは捕捉 0 件でも**静かに緑**（偽陰性）、「このログが出る」を主張するテストは捕捉 0 件で**確率的に赤**（偽陽性。`areka-seriko` で約 1/6・負荷依存で実測済み）。
- 硬化の判定は機械的に 1 点（捕捉窓の内側で `rebuild_interest_cache()` を叩くか、同等の常駐 probe／keeper を確立しているか）。現行ツリーで `with_default(` を含みこの印を一つも持たないファイルは **24 ファイル**（brief 棚卸⑩の「未硬化ヘルパ定義 10 ファイル」は全件健在で、それに加え**別名ヘルパ 7 ファイル・ヘルパ無しの直書き 7 ファイル／29 呼出**が新たに判明した）:
  - 名前付きヘルパ `capture_logs` の未硬化定義 10 ファイル: `crates/areka/src/emo2_boot/{adapter.rs:383, spine.rs:525, frame_test_support.rs:122, frame_chain_finalize_tests.rs:241, move_cue_move_severity_log_tests.rs:43, talk_lifecycle_tests.rs:97}`・`crates/areka/src/input_events/{balloon_test_support.rs:140, choice_drain.rs:182}`・`crates/areka-seriko/src/table.rs:209`・`crates/wintf/src/ecs/window_proc/dpi_helpers_tests.rs:345`
  - 別名ヘルパの未硬化定義 7 ファイル: `crates/areka-emo-text/src/{draw_test_support.rs:61, actor_runtime_frame_tests.rs:53, sink.rs:170}`（`with_log_cage`）・`crates/areka-emo-text/src/region.rs:400`（`count_warns`）・`crates/areka-emo-text/src/{wrap.rs:114, writing.rs:128}`（`resolve_counting_warns`）・`crates/areka-ghost/src/sink.rs:224`（`capture`）
  - ヘルパ無しの直書き 7 ファイル／29 呼出: `crates/areka-emo-present/src/{presenter_refresh_and_log_tests.rs 7, presenter_perf_log_tests.rs 6, presenter/transition_record_tests.rs 5, presenter/timing_tests.rs 3}`（共有 `CaptureSubscriber` を使うが probe／rebuild 無し。代わりに「陰性主張は同一捕捉窓内の陽性 1 本と対にする」規律で自衛している）・`crates/areka-emo-text/src/{state_cue_apply_tests.rs 3, layout_cursor_tests.rs 2}`・`crates/areka/src/shiori_demo.rs 3`
- 呼出規模: `capture_logs(` 238 箇所／64 ファイル（硬化済み含む）・`capture_logs_flow(` 18・`capture_under_filter(` 96（wintf）・`with_default(` 総計 62 箇所。
- 硬化済みは **16 ファイル／`with_default` 28 呼出**（`with_default(` を持ち、同一ファイルまたは委譲先に `rebuild_interest_cache()`／`ensure_interest_probes()`／`install_interest_keeper()` の印を持つ定義側。`rebuild_interest_cache` の字面だけなら消費側 2 ファイルを含めて 18 ファイル。16＋未硬化 24＝`with_default` を持つ全 40 ファイル）。brief 追記(59) の未硬化表のうち `areka-emo-atlas`／`areka-emo-compose` の `log_capture.rs`・`areka-seriko` の `actor_test_support.rs`／`looper_tests.rs`／`state_test_support.rs` は**域外で硬化済みへ転じた**（ただし `state_test_support.rs:12-13`・`looper_tests.rs:852-853`・`actor_test_support.rs:37/49` には硬化後も「スレッドローカルゆえ並行テスト安全」の旧説明文が残っている）。
- 誤った説明（「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」）は未硬化 10 ファイルのうち 9 ファイルに現存し（`table.rs:207`・`choice_drain.rs:161`・`balloon_test_support.rs:119`・`talk_lifecycle_tests.rs:72-73`・`spine.rs:499-500`・`move_cue_move_severity_log_tests.rs:11`・`frame_test_support.rs:96/582`・`frame_chain_finalize_tests.rs:215-216`・`adapter.rs:358-359`）、brief が「否認が残ると再発する」と 3 度実証したとおり、新設ファイルへ複製され続けている（W5 で +1・追記(59) で +50 呼出・slimming／atom で新顔 7 ファイル）。

**② 反復回数固定の待機ループが残る**——`spine.rs` 本体は 872 行へ分割され（slimming）、本体には壁時計上限を持たないループは **0 箇所**（`spin_wait_until` :358 は `SPIN_WAIT`=30 秒 :329 で有界）。残りは分割先の 2 箇所: `crates/areka/src/emo2_boot/spine_display_tests.rs:410-414`（`for now in 1_000_000u64..1_000_000+5_000`＝ループ変数が注入 Tick を兼ねる settle）と `crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs:372-375`（`for _ in 0..5_000`＝負検証の settle drain）。どちらも「尽きるのが正常」の settle であり、負荷下では与えられる機会が縮む＝不在主張が弱くなる方向の非決定性を持つ。donor の `spin_pumping_ticks` は `crates/areka-ghost/tests/ghost/spine_e2e_test.rs:48`。

**③ ログ捕捉の硬化設計が 2 系統併存し正典が無い**——probe dispatcher 常駐方式（`ensure_interest_probes`＝probe dispatcher 2 個を `OnceLock` でプロセス寿命常駐・`set_global_default` 不使用）が **8 箇所に複製**（`areka/src/placement/test_support.rs`・`wintf/src/ecs/test_support.rs`・`areka-seriko/src/log_interest_probe.rs`・`areka-emo-atlas/src/log_capture.rs`・`areka-emo-compose/src/log_capture.rs`・`areka-emo-present/src/{scale_tests.rs, balloon_test_support.rs}`・`areka-emo-text/tests/attach_wiring_test.rs`。brief 当時の 3 コピーから増加）、global-default keeper 方式（`install_interest_keeper`＝素の registry を `set_global_default` で常駐）が 3 crate（`areka-sylphya`・`areka-kanade`・`areka-ghost`）、さらに一回限りの全スレッド global capture-all（`areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs:75-93`・`areka-seriko/tests/loop_integration.rs:590-608`＝いずれも別スレッドで発火するログを捕える本物の需要で、`set_global_default` を統合テストバイナリ内で一度だけ置く）と上記「陽性と対にする」規律がある。意味論は近いが相互に排他的な前提（keeper は「先に別の global subscriber を置いてはならない」）を持ち、どれが正典か決まっていない。これは重複除去ではなく設計判断である。

**④ 表示更新の失敗経路が実行テストで検証できない**——`crates/areka-emo-present/src/chain.rs` の `upload`（:185-241）は `ResizeBuffers`／`GetBuffer`／`Present` 等 **7 箇所**の `?` で実 D3D/DXGI 失敗を返し得るが、`SwapChainPresenter` は trait を持たない具体型（:122）で `presenter/target.rs:73` が `Option<SwapChainPresenter>` として直接保持し、注入点が無い。唯一の消費点 `presenter/show.rs:306-310` は失敗時「表示は前状態を保つ」と主張しているが未検証（既存 `upload` テストは成功経路のみ・実 GPU 必須）。この分岐は `dpi-transition-atomicity` が本仕様の観測点として意図的に不動で残した。

### 隣接 spec からの申し送り（本仕様が引き継ぐもの）
- `dpi-transition-atomicity`（追記(72)(76)）: 観測 target `wintf::transition` の語彙不変条件（窓種別は `win_kind=`・1 行に同名フィールドを 2 度出さない）／決定論テストは一括 flush（実 `SetWindowPos`／`DeferWindowPos`）に到達しないので `kind=write` 行を数えるテストは退行を捕まえない／多フレーム駆動ハーネス `FrameHarness`（`frame_test_support.rs`）とその自己テスト（`frame_harness_tests.rs`）は作り直さない／`presenter/show.rs` の観測前置ガードは本文走査テスト（`transition_record_tests.rs`）だけが守っている。**µs の上限は引き継がない**。
- `dpi-transition-atomicity` 追記(76)⑹＋W6.9 同居裁定: `crates/wintf/src/ecs/window/command.rs:49` の自発書込カウンタ `SELF_INITIATED_DEPTH` はプロセス共有 `AtomicI32` だが意味論はスレッド局所で、並列テストが互いを汚染する（上流実測 60 回中 11 失敗）。是正（`Cell<i32>` 化）は **`command.rs` を丸ごと所有する `draw-load-parity` が実施**し、本仕様は症状側＝テスト側の錠 `lock_self_initiated_for_test()`（定義 `command.rs:76`・実呼出 **21 箇所／5 ファイル**＝`command.rs` 2・`command_batch_tests.rs` 5・`command_transition_tests.rs` 4・`window_proc/window_pos_tests.rs` 5・`window_proc/window_pos_transition_tests.rs` 5。doc 言及 2 行を含めると 23）の退役だけを受ける。
- `bindoption-exclusivity`: ログ存在主張で間欠赤だったテスト 6 本（`bind_apply_on_shown_emits_show_and_info_marker`・`bind_default_exclusive_replace_emits_show_and_info_marker`・`non_shell_broadcast_reception_is_benign_debug_no_warn_error`・`wait_broadcast_reception_is_benign_debug_no_warn_error`・`progress_phase_bind_drop_emits_info_marker`・`residual_frame_removal_emits_info_marker`）を本仕様の担当クラスとして登記。現行ツリーではいずれも硬化済みヘルパ経由になっているが、**反復実行で緑が安定したことは未確認**。
- `ghost-window-zorder`／W6: `balloon-visibility` が再表示シーム `ReassertZOrder` を消費せずに着地（再表示直後のバルーン隣接は実機未確認）。決定論テストで拾える範囲を本仕様で検討し、拾えなければ e2e へ申し送る。
- `kero-balloon`: `cargo test -p areka` が 1 回だけ 553/1 で赤（テスト名不明・ログ未保存）。①硬化後に再現しなくなるかを反復検証で確認する。
- roadmap 追記(79): 「1 ファイル 1,000 行以下」の目安（`structure.md:176`）に機械的な番人が無く漂流している（現行ツリーで 1,000 行超は **11 ファイル**＝roadmap 表の 9 本に `plan_ops_tests.rs` の再増・`inproc_e2e_test.rs`・`pilot` example が加わる）。3 択（⒜ 番人テスト／⒝ 掃除 spec／⒞ 目安の緩和）は**開発者裁定待ち**で、⒜ の置き場候補が本仕様。要件 10 は ⒜ が採られた場合にのみ有効な条件付き要件として置く。

## Boundary Context

- **In scope**:
  - ③ ログ捕捉の硬化設計を**ワークスペースで 1 つ**に定め、全 crate（`wintf` と bin crate `areka` を含む）から同じ機構を引けるようにする。
  - ① 上記 24 ファイルの捕捉サイトを共有機構へ移行し、誤った説明文を全件是正する（硬化済みファイルに残る旧説明文を含む）。着手時にワークスペース全体を再計測し、本文書のインベントリを現在値で更新する。
  - ② 分割先 2 箇所の反復回数固定 settle を壁時計または観測量で有界化する（Tick 注入を兼ねる形は Tick 生成と上限を分離し、注入時刻が観測を追い越さない形を保つ）。
  - ④ 表示更新（`upload`）の失敗を実行テストから注入できる形を設け、「失敗時に表示は前状態を保つ」を実行テストで証明する。
  - 共有機構を迂回する新規の捕捉ヘルパ／直書きが増えたら赤になる再発防止。
  - `SELF_INITIATED_DEPTH` 是正（`draw-load-parity` 実施）後の錠 `lock_self_initiated_for_test()` の退役。
  - （条件付き）追記(79) ⒜ が採られた場合の 1,000 行番人テスト。
- **Out of scope**:
  - 本番の挙動変更（④ の注入点以外）。とくに `crates/wintf/src/ecs/window/command.rs` は**非接触**（`draw-load-parity` 所有）。
  - 既存テストの**判定内容**の変更（直すのは観測機構と待機機構だけ）。観測が正しくなった結果として落ちるテストは本物の欠陥の発見であり、テストを緩めて通さず別途起票する。
  - `spine` 系テストの削除・`areka-ghost` 側の待機（2026-07-30 に是正済み）・`dpi-transition-atomicity` の実機未達 µs 2 系統（`present-write-coherence` 所有）・`presenter/show.rs` の可視化の段（:375-392・`present-write-coherence` 所有）。
  - 新規外部依存の追加（`tracing-subscriber` は既出）。
- **Adjacent expectations**:
  - `draw-load-parity`（W6.9 同居）: `command.rs` の `SELF_INITIATED_DEPTH` を `Cell<i32>` 化し着地形を本仕様へ申し送る。見送る場合は本仕様へ即報告（その時点で着手順を再調整）。共有ファイルは実測 0（本仕様が各 crate の `Cargo.toml` dev-dependencies へ 1 行足す見込みのみ）。
  - `present-write-coherence`（W6.95）・`balloon-offset-dpi`（W6.95）: 本仕様の後着。`present-write-coherence` は同じ `apply_show` 鎖（可視化の段）を触るので、本仕様は ④ の観測点（:306-310）以外の `show.rs` を動かさない。`balloon-offset-dpi` は一本化済みの共有機構でテストを書く。
  - `emo2-conformance-e2e`（W7）: 決定論テストで拾えなかった `ReassertZOrder` 再表示隣接の確認先。

## Requirements

### Requirement 1: ログ捕捉の硬化設計の一本化
**Objective:** ワークスペース全体のテストを保守する開発者として、ログ捕捉テストの硬化設計が 1 つに定まり全 crate がそれを共有することを求める。それにより「どの流儀が正しいか」を crate ごとに判断し直す必要が無くなり、複製による再発が構造的に止まる。

#### Acceptance Criteria
1. The テスト基盤 shall ログ捕捉の硬化機構（テスト間の interest 汚染を構造的に防ぐ仕組み）の定義箇所をワークスペースで **1 箇所**にする。
2. The テスト基盤 shall その共有機構を `wintf`・bin crate `areka`（in-crate `#[cfg(test)]` テスト）・`areka-*` 各 crate・統合テスト（`tests/`）のいずれからも同じ形で利用できるようにする。
3. While 共有機構が `wintf` から利用される, the テスト基盤 shall `wintf` に本番コードを持つ上位 crate（`areka`・`areka-seriko`・`areka-emo-*` 等）への依存を新たに持ち込まない（依存方向の規律。ワークスペース内依存を持たないテスト専用の共有 crate を dev-dependency として引くことはこれに当たらず、その crate の命名は設計で決める）。
4. The テスト基盤 shall 共有機構の導入によって新規の外部依存（`tracing`／`tracing-subscriber` 以外）を追加しない。
5. When 共有機構が採用される, the テスト基盤 shall 不採用となった設計（probe dispatcher 方式・global-default keeper 方式・一回限りの global probe・直書き）の実装コピーをワークスペースから **0 件**にする（正典の採否の理由は設計文書に登記する）。
6. If 共有機構と両立しない既存の subscriber 設置（例: テスト側で独自に `set_global_default` を置く箇所）が残る, then the テスト基盤 shall それを共有機構へ寄せるか、両立条件を明文化して違反時に明示的に失敗させる（黙って縮退しない）。
7. The テスト基盤 shall 共有機構の移行前後で、既存テストの判定結果（緑／赤）を 1 件も変えない（変わるテストが出た場合は要件 6 に従う）。

### Requirement 2: 捕捉サイトの全面移行と誤った説明文の是正
**Objective:** ログ捕捉テストを書く開発者として、ワークスペースのすべての捕捉サイトが共有機構を通り、「スレッドローカルゆえ安全」という誤った説明が残っていないことを求める。それにより捕捉 0 件を掴まされる窓が無くなり、誤った説明を手本に新しい未硬化コピーが生まれなくなる。

#### Acceptance Criteria
1. When 本仕様に着手する, the 実装者 shall ワークスペース全体（`crates/**`）を再計測し、「`with_default(` を含み硬化の印（共有機構の利用）を持たないファイル」のインベントリを現在値で本文書に更新する（2026-08-22 時点の計測値は 24 ファイル＝Introduction の表）。
2. When 移行が完了する, the テスト基盤 shall 上記インベントリの全サイト（名前付きヘルパ 10 ファイル・別名ヘルパ 7 ファイル・直書き 7 ファイル／29 呼出）を共有機構経由へ置き換え、未硬化サイトを **0 件**にする。
3. The テスト基盤 shall 既存の派生ヘルパ（例: `capture_logs_flow`・`with_log_cage`・`count_warns`・`resolve_counting_warns`・`capture_events`・`CaptureSubscriber`）について、捕捉層だけを共有機構へ委譲し、呼出側の判定内容（戻り値の意味・assert 文）を変えない。
4. The テスト基盤 shall 「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」に類する誤った説明文を、未硬化ファイル・硬化済みファイルの双方から **0 件**にし、正しい機序（interest キャッシュはプロセス共有・先着が勝つ・共有機構が何を保証するか）を説明する記述へ置き換える。
5. The テスト基盤 shall 共有機構の由来表示を正しく保つ（存在しない由来や硬化状況を誤認させる表記を残さない）。
6. When 移行後にワークスペースを走査する, the テスト基盤 shall 共有機構の定義箇所以外に `tracing::subscriber::with_default` の直接呼出が残っていないことを機械的に示す（例外を設ける場合は例外表に列挙し、各項目に理由を付す）。

### Requirement 3: 捕捉テストの信頼性（両側保証と自己検証）
**Objective:** テストの緑を信じて次の spec を進める開発者として、ログ捕捉テストが「静かに緑」（不在主張が捕捉 0 件で通る）にも「確率的に赤」（存在主張が捕捉 0 件で落ちる）にもならないことを求める。それにより赤は本番の欠陥を、緑は検証が実際に行われたことを意味する。

#### Acceptance Criteria
1. While `cargo test --workspace` が並列負荷下で実行されている, the 共有ログ捕捉機構 shall 捕捉窓の内側でテストスレッドが発行した対象イベントを取りこぼさない。
2. While 他のテストが先に同じ発行点を踏んで interest を焼き付けている, the 共有ログ捕捉機構 shall 後続の捕捉テストにそのイベントを観測させる（焼き付きを捕捉窓の内側で解消する）。
3. When ログの**不在**を主張するテストが共有機構で書かれる, the 共有ログ捕捉機構 shall 「捕捉そのものが働いていた」ことを同じ捕捉窓の内側で示せる手段（対照となる陽性観測）を提供し、捕捉 0 件のまま不在主張が通る形を既定にしない。
4. The 共有ログ捕捉機構 shall 自分自身の決定性を示す自己テストを持ち、（a）意図的に interest を焼き付けた状態でも対象イベントを捕捉できること、（b）硬化を外した素の捕捉では同じ条件で取りこぼすこと（較正＝毎回赤も作れること）、の両方を実行テストで固定する。
5. The 共有ログ捕捉機構 shall TRACE を含む全レベルのイベントを捕捉対象にできる。
6. The 共有ログ捕捉機構 shall 既定の捕捉 API でスレッド局所の捕捉意味論（捕捉窓の外・他スレッドで発行されたイベントを混入させない）を保つ（別スレッドで発火するログを捕える必要がある場合は、要件 1.6 に従い明示的に区別された別の API として提供し、既定の API と混同させない）。
7. When `bindoption-exclusivity` が登記した間欠赤テスト 6 本（Introduction 参照）を含む `areka-seriko` のログ存在主張テストを反復実行する, the テスト基盤 shall 要件 9 の反復条件で失敗 0 件を示す。

### Requirement 4: 待機機構の有界化（反復回数に依存しない待機）
**Objective:** 負荷のかかった環境で `cargo test` を回す開発者として、テストの待機が反復回数ではなく壁時計または観測量で有界化され、負荷下でも偽の赤・空虚な緑を出さないことを求める。それにより Defender 再スキャンや並列負荷でテストの判定が変わらない。

#### Acceptance Criteria
1. When 本仕様に着手する, the 実装者 shall `crates/areka/src/emo2_boot/spine*.rs` を再計測し、壁時計上限を持たない反復固定の待機ループのインベントリを現在値で更新する（2026-08-22 時点: `spine_display_tests.rs:410-414`・`spine_seriko_loop_tests.rs:372-375` の 2 箇所。`spine.rs` 本体は 0 箇所）。
2. The テスト基盤 shall 判定に影響する待機（到着を待つ待機・残余を回収する settle）を、反復回数のみで打ち切る形から、壁時計上限または観測量（例: 連続して空だった回数）に基づく有界化へ置き換える。
3. While 待機ループのループ変数が注入 Tick の生成を兼ねている, the テスト基盤 shall Tick 生成と打ち切り条件を分離し、注入する時刻が観測を追い越さない（観測より先に時刻だけが進む形にならない）ようにする。
4. The テスト基盤 shall 「時刻を進めるために sleep しない」（時刻前進は注入 Tick のみ）という決定論の前提を保ち、待機の譲歩に使う短い sleep は有界 poll-backoff に限定する。
5. When 負検証（「尽きるのが正常」）の settle が有界化される, the テスト基盤 shall 回収機会が負荷によって縮まない形（壁時計の最小持続、または連続して空だった観測回数）で与えられることを保証し、assert の内容は変えない。
6. The テスト基盤 shall 既存のテスト本数・判定内容を保ったまま（削除・緩和なし）上記を適用する。

### Requirement 5: 表示更新失敗時の前状態保持の実行テスト
**Objective:** 表示機構の保守者として、GPU への表示更新（`upload`）が失敗したときに「表示は前状態を保つ」という主張が実行テストで証明されていることを求める。それにより GPU 失敗時に壊れた絵や空の窓が出ないことをコードの主張ではなく実行結果で担保できる。

#### Acceptance Criteria
1. The テスト基盤 shall 表示更新の失敗を実行テストから再現可能に注入できる手段を提供し、注入は `upload` が失敗を返し得る各経路（2026-08-22 時点で `chain.rs:185-241` の 7 箇所）を少なくとも分類単位（寸法変更・資源取得・提示）で踏めるようにする。
2. When 注入された失敗によって `upload` がエラーを返す, the 表示機構 shall 直前に成功した表示内容・寸法・可視状態を保ち、失敗した更新の途中状態を表示に反映しない。
3. When 注入された失敗によって `upload` がエラーを返す, the 表示機構 shall 呼出元へ失敗を返し、ログにその失敗を記録する（静かな失敗経路を持たない）。
4. When 失敗注入のテストが `cargo test` で実行される, the テスト基盤 shall 既存の headless 条件（実 GPU 個体を前提としない既存テストと同じ前提）で完走し、実機 GPU 失敗の再現を必要としない。
5. The 表示機構 shall 注入手段が無効なとき（通常実行）の本番挙動・性能特性（定常状態のアロケーション 0 を含む）を変えない。
6. The テスト基盤 shall 観測点である早期 return 分岐（`presenter/show.rs:306-310`）の位置・意味を動かさず、`dpi-transition-atomicity` が残した本文走査テスト（`transition_record_tests.rs`）の被覆を縮めない。

### Requirement 6: 既存テストの判定内容の保存と本物の欠陥の扱い
**Objective:** 本仕様の変更をレビューする開発者として、是正が「観測機構と待機機構」に限られ、テストの主張そのものが弱められていないことを求める。それにより是正後の緑が是正前より強い主張を意味する。

#### Acceptance Criteria
1. The テスト基盤 shall 既存テストの assert 文・期待値・テスト本数を変更しない（移行に伴う import／ヘルパ呼出の書き換えは除く）。
2. If 観測が正しくなった結果として既存テストが恒常的に失敗する, then the 実装者 shall そのテストを緩めず、本番の欠陥候補として別途起票し、本文書の申し送りに登記する。
3. If 観測が正しくなった結果として既存テストの不在主張が「捕捉 0 件で通っていた」と判明する, then the 実装者 shall 要件 3.3 の対照観測を当該テストへ追加し、主張内容は変えない。
4. The テスト基盤 shall `spine` 系テストを削除しない（退役か更新かは obsolete-vs-broken-test-policy に従い個別に判断し、理由を記録する）。

### Requirement 7: テスト間の共有状態による汚染の解消（自発書込カウンタの錠の退役）
**Objective:** `wintf` の窓書込テストを保守する開発者として、プロセス共有カウンタの是正後にテスト側の錠が不要になり、退役されていることを求める。それによりテストの直列化強制が消え、錠の取り忘れによる間欠失敗の余地が無くなる。

#### Acceptance Criteria
1. While `draw-load-parity` が `SELF_INITIATED_DEPTH` のスレッド局所化（`command.rs`）を着地させていない, the 本仕様 shall `command.rs` に触れず、錠 `lock_self_initiated_for_test()` の利用を現状のまま保つ。
2. When `draw-load-parity` の着地形（スレッド局所化）が本仕様のブランチへ取り込まれる, the テスト基盤 shall 錠 `lock_self_initiated_for_test()` の呼出（2026-08-22 時点 実呼出 21 箇所／5 ファイル）を退役させ、退役後も当該テスト群が並列実行で失敗 0 件であることを要件 9 の反復条件で示す。
3. If `draw-load-parity` がスレッド局所化を見送った, then the 本仕様 shall その旨を申し送りに登記し、錠を温存したまま完了できる（是正そのものは本仕様の範囲外のまま）。
4. The テスト基盤 shall 錠の退役後に `command.rs` 本体の錠定義が不要になる場合でも、その削除は `command.rs` の所有者（`draw-load-parity`）へ申し送り、本仕様では行わない。

### Requirement 8: 再発防止（共有機構を迂回する捕捉の新設検知）
**Objective:** 後続 spec の実装者として、共有機構を迂回した捕捉ヘルパや直書きを新設すると即座に赤になることを求める。それにより「後置するほどコピーが増える」構造が止まり、本仕様の成果が次の spec で崩れない。

#### Acceptance Criteria
1. When ワークスペースのテストコードに共有機構を経由しない `tracing::subscriber::with_default` の直接呼出が新設される, the テスト基盤 shall 実行テストで検知して失敗させる（既定で例外表は空、やむを得ない例外は理由付きで列挙）。
2. When 例外表に載る項目が増える, the テスト基盤 shall その増加を明示的な編集として要求する（暗黙に通さない）。
3. The テスト基盤 shall 検知テストの走査対象に、テストを外へ出した兄弟ファイル（`<stem>_*.rs`）と `tests/`・`examples/` を含め、被覆が黙って縮まないようにする。
4. The 検知テスト shall 既知の陽性例（未硬化の直書き）で赤になること自体を自己テストで固定する（道具の較正）。

### Requirement 9: 検証の反復性と証拠
**Objective:** 本仕様の完了を承認する開発者として、非決定性の是正が単発の緑ではなく反復実行の証跡で示されていることを求める。それにより「たまたま通った」緑を完了の根拠にしない。

#### Acceptance Criteria
1. The 検証 shall `cargo test --workspace` を並列負荷下で連続 **10 回以上**実行し、ログ捕捉・待機・錠退役に起因する失敗が 0 件であることを示す（i686 前提成果物のビルド後・PowerShell で実行）。
2. The 検証 shall 要件 3.7 の `areka-seriko` 存在主張テスト群について、該当 crate の lib テストを負荷下で **30 回以上**反復し失敗 0 件を示す。
3. The 検証 shall 要件 4 の待機テスト群について、並列負荷下で **30 回以上**反復し失敗 0 件を示す。
4. When 反復検証中に赤が 1 回でも出る, the 検証 shall 必ずログを保存してテスト名と失敗内容を採り、本文書の申し送りに登記する（テスト名不明の赤を残さない）。
5. When 反復検証が完了する, the 検証 shall `cargo test -p areka` の正体不明の 553/1 赤が①硬化後に再現しなくなったか（あるいは別系統として残るか）を記録する。
6. The 検証 shall 結果を「道具の較正」（要件 3.4・8.4）込みで報告し、緑が道具の故障で出ていないことを示す。

### Requirement 10: 1 ファイル 1,000 行の目安の機械的な番人（条件付き・開発者裁定待ち）
**Objective:** リポジトリの構造規律を保つ開発者として、roadmap 追記(79) で ⒜（番人テスト）が採られた場合に、「1 ファイル 1,000 行以下」の目安が機械的に守られることを求める。それにより規則があるのに誰も測っていない状態が解消される。

#### Acceptance Criteria
1. Where 開発者が追記(79) の ⒜ を採用する, the テスト基盤 shall `crates/**/*.rs`（`src/`・`tests/`・`examples/` を含む）の各ファイルの行数を測り、1,000 行を超え例外表に無いファイルがあれば失敗する実行テストを 1 本置く。
2. Where ⒜ が採用される, the 例外表 shall 着手時点の超過ファイル（2026-08-22 時点 11 ファイル）を列挙して開始し、項目の追加は明示的な編集としてのみ許し、削除（分割による解消）のみを自然な方向とする。
3. Where ⒜ が採用される, the 番人テスト shall 既知の超過ファイルを一時的に例外表から外したとき赤になることを自己テストで固定する。
4. Where 開発者が ⒝ または ⒞ を採用する, the 本仕様 shall 番人テストを実装せず、本要件を「不採用」として記録する。

### Requirement 11: 隣接 spec からの申し送りの遵守
**Objective:** 並走・後続 spec（`draw-load-parity`・`present-write-coherence`・`balloon-offset-dpi`・`emo2-conformance-e2e`）の実装者として、本仕様が上流の語彙・前提を壊さず、拾えなかった確認事項を明示的に引き渡すことを求める。それにより本仕様の後で各 spec の前提が食い違わない。

#### Acceptance Criteria
1. When 本仕様が `wintf::transition` の観測行を足す・共有ヘルパへ寄せる, the テスト基盤 shall 語彙不変条件（窓種別フィールドは `win_kind=`・1 行に同名フィールドを 2 度出さない）を守り、既存の逐語テストを緑のまま保つ。
2. The テスト基盤 shall 決定論テストで窓書込の回数を数える場合にキュー側で数え、一括 flush 由来の `kind=write` 行を判定根拠にしない。
3. The テスト基盤 shall 多フレーム駆動ハーネス `FrameHarness` とその自己テストを作り直さず、捕捉層のみを共有機構へ寄せる。
4. When `balloon-visibility` が消費しなかった再表示シーム `ReassertZOrder` の隣接確認を検討する, the 本仕様 shall 決定論テストで固定できる範囲を確定して実装し、固定できない範囲は `emo2-conformance-e2e` へ理由付きで申し送る。
5. The 本仕様 shall `draw-load-parity` と共有ファイル 0 を保ち、各 crate の `Cargo.toml` への変更を dev-dependencies の追加に限定する。
6. When 本仕様が完了する, the 本仕様 shall 後続 spec が共有機構でテストを書くための利用手順（どこから引くか・不在主張の書き方）を文書に残す。

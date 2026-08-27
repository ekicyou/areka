# Requirements Document

## Introduction

本ワークスペースは「テスト可能な領域はすべて実行テストで固定する」方針（deterministic-test-coverage-mandate）を掲げている。その方針の前提は「緑は本当に緑であり、赤は本当に赤である」こと＝**テストの決定性そのもの**である。ところが現在のツリーには、テストが本番挙動とは無関係に嘘をつき得る状態が 4 系統残っている。いずれも本番の欠陥ではないが、`cargo test --workspace` という全 spec 共通の完了ゲートの信号を損なう。

本仕様（W6.9・`draw-load-parity` と 2 本並走）はこの 4 件を 1 本で通しで是正する。4 件はどれも「テストが本当のことを言っているか」という同一の関心であり、①と③は同じファイル群を触るため分けると二重作業になる。

### 4 系統の現状（**2026-08-23 に着手時の全面再計測を実施**＝HEAD `b9f7936e`・`origin/main` `327e7fd3` は HEAD の先祖で取り込み漏れ無し。初出は 2026-08-22・`f6b81078`。計測に使ったコマンドは `verification/remeasure.md` にあり、同じコミットで再実行すれば同じ数値が出る。旧値と異なる箇所は併記）

**① ログ捕捉テストがテスト間で汚染される**——`tracing` の callsite interest キャッシュはプロセス全体で共有され、最初にその発行点を踏んだスレッドの判定が焼き付く。`with_default` で差し込んだ捕捉 subscriber はスレッド局所だが interest は違うため、別スレッドが先に「このログは不要」と焼き付けると、後続の捕捉テストはイベントを 1 件も観測できなくなる。症状はテストの書き方で両側に出る: 「このログは出ない」を主張するテストは捕捉 0 件でも**静かに緑**（偽陰性）、「このログが出る」を主張するテストは捕捉 0 件で**確率的に赤**（偽陽性。`areka-seriko` で約 1/6・負荷依存で実測済み）。
- 硬化の判定は機械的に 1 点（捕捉窓の内側で `rebuild_interest_cache()` を叩くか、同等の常駐 probe／keeper を確立しているか）。2026-08-23 の再計測でも `with_default(` を含みこの印を一つも持たないファイルは **24 ファイル**（2026-08-22 と同数で、**ファイルの集合そのものも一致**した。brief 棚卸⑩の「未硬化ヘルパ定義 10 ファイル」は全件健在で、それに加え**別名ヘルパ 7 ファイル・ヘルパ無しの直書き 7 ファイル／29 呼出**が新たに判明した）:
  - 名前付きヘルパ `capture_logs` の未硬化定義 10 ファイル: `crates/areka/src/emo2_boot/{adapter.rs:388, spine.rs:525, frame_test_support.rs:122, frame_chain_finalize_tests.rs:241, move_cue_move_severity_log_tests.rs:43, talk_lifecycle_tests.rs:97}`・`crates/areka/src/input_events/{balloon_test_support.rs:140, choice_drain.rs:182}`・`crates/areka-seriko/src/table.rs:209`・`crates/wintf/src/ecs/window_proc/dpi_helpers_tests.rs:345`
  - 別名ヘルパの未硬化定義 7 ファイル: `crates/areka-emo-text/src/{draw_test_support.rs:61, actor_runtime_frame_tests.rs:53, sink.rs:170}`（`with_log_cage`）・`crates/areka-emo-text/src/region.rs:400`（`count_warns`）・`crates/areka-emo-text/src/{wrap.rs:114, writing.rs:128}`（`resolve_counting_warns`）・`crates/areka-ghost/src/sink.rs:224`（`capture`）
  - ヘルパ無しの直書き 7 ファイル／29 呼出: `crates/areka-emo-present/src/{presenter_refresh_and_log_tests.rs 7, presenter_perf_log_tests.rs 6, presenter/transition_record_tests.rs 5, presenter/timing_tests.rs 3}`（共有 `CaptureSubscriber` を使うが probe／rebuild 無し。代わりに「陰性主張は同一捕捉窓内の陽性 1 本と対にする」規律で自衛している）・`crates/areka-emo-text/src/{state_cue_apply_tests.rs 3, layout_cursor_tests.rs 2}`・`crates/areka/src/shiori_demo.rs 3`
- 呼出規模（2026-08-23 実測。いずれも 2026-08-22 から増減なし）: `capture_logs(` 238 箇所／**62 ファイル**（硬化済み含む。2026-08-22 の「64 ファイル」は記録の誤りで、`f6b81078` で数え直しても 62）・`capture_logs_flow(` 18・`capture_under_filter(` 96（wintf）・`with_default(` 総計 62 箇所／40 ファイル。
- 硬化済みは **16 ファイル／`with_default(` 16 呼出**（2026-08-23 実測。2026-08-22 の「28 呼出」は記録の誤りで、`f6b81078` で数え直しても 16＝硬化済みは 1 ファイル 1 呼出。検算: 総計 62 − 硬化済み 16 ＝ 46 ＝ 未硬化のヘルパ定義 17 ＋ 直書き 29。`with_default(` を持ち、同一ファイルまたは委譲先に `rebuild_interest_cache()`／`ensure_interest_probes()`／`install_interest_keeper()` の印を持つ定義側。`rebuild_interest_cache` の字面だけなら消費側 2 ファイルを含めて 18 ファイル。16＋未硬化 24＝`with_default` を持つ全 40 ファイル）。brief 追記(59) の未硬化表のうち `areka-emo-atlas`／`areka-emo-compose` の `log_capture.rs`・`areka-seriko` の `actor_test_support.rs`／`looper_tests.rs`／`state_test_support.rs` は**域外で硬化済みへ転じた**（ただし `state_test_support.rs:12-13`・`looper_tests.rs:852-853`・`actor_test_support.rs:37/49` には硬化後も「スレッドローカルゆえ並行テスト安全」の旧説明文が残っている）。
- 誤った説明（「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」）は未硬化 10 ファイルのうち 9 ファイルに現存し（`table.rs:207`・`choice_drain.rs:161`・`balloon_test_support.rs:119`・`talk_lifecycle_tests.rs:72-73`・`spine.rs:499-500`・`move_cue_move_severity_log_tests.rs:11`・`frame_test_support.rs:96/582`・`frame_chain_finalize_tests.rs:215-216`・`adapter.rs:363-364`。2026-08-22 は `:358-359` で、同ファイルが +5 行ずれた）、brief が「否認が残ると再発する」と 3 度実証したとおり、新設ファイルへ複製され続けている（W5 で +1・追記(59) で +50 呼出・slimming／atom で新顔 7 ファイル）。

**② 反復回数固定の待機ループが残る**——`spine.rs` 本体は 872 行へ分割され（slimming）、本体には壁時計上限を持たないループは **0 箇所**（`spin_wait_until` :358 は `SPIN_WAIT`=30 秒 :329 で有界。2026-08-23 の再計測でも 0 箇所・行番号も不変）。残りは分割先の 2 箇所（2026-08-23 の再計測でも同じ 2 箇所・行番号も不変）: `crates/areka/src/emo2_boot/spine_display_tests.rs:410-414`（`for now in 1_000_000u64..1_000_000+5_000`＝ループ変数が注入 Tick を兼ねる settle）と `crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs:372-375`（`for _ in 0..5_000`＝負検証の settle drain）。どちらも「尽きるのが正常」の settle であり、負荷下では与えられる機会が縮む＝不在主張が弱くなる方向の非決定性を持つ。donor の `spin_pumping_ticks` は `crates/areka-ghost/tests/ghost/spine_e2e_test.rs:48`。

**③ ログ捕捉の硬化設計が 2 系統併存し正典が無い**——probe dispatcher 常駐方式（`ensure_interest_probes`＝probe dispatcher 2 個を `OnceLock` でプロセス寿命常駐・`set_global_default` 不使用）が **8 箇所に複製**（`areka/src/placement/test_support.rs`・`wintf/src/ecs/test_support.rs`・`areka-seriko/src/log_interest_probe.rs`・`areka-emo-atlas/src/log_capture.rs`・`areka-emo-compose/src/log_capture.rs`・`areka-emo-present/src/{scale_tests.rs, balloon_test_support.rs}`・`areka-emo-text/tests/attach_wiring_test.rs`。brief 当時の 3 コピーから増加。2026-08-23 の再計測でも 8 で、下記の keeper 方式 3 crate・一回限りの global capture-all 2 ファイルも増減なし）、global-default keeper 方式（`install_interest_keeper`＝素の registry を `set_global_default` で常駐）が 3 crate（`areka-sylphya`・`areka-kanade`・`areka-ghost`）、さらに一回限りの全スレッド global capture-all（`areka-ghost/tests/ghost/spine_e2e_test_global_log_probe.rs:75-93`・`areka-seriko/tests/loop_integration.rs:590-608`＝いずれも別スレッドで発火するログを捕える本物の需要で、`set_global_default` を統合テストバイナリ内で一度だけ置く）と上記「陽性と対にする」規律がある。意味論は近いが相互に排他的な前提（keeper は「先に別の global subscriber を置いてはならない」）を持ち、どれが正典か決まっていない。これは重複除去ではなく設計判断である。

**④ 表示更新の失敗経路が実行テストで検証できない**——`crates/areka-emo-present/src/chain.rs` の `upload`（:185-241）は `ResizeBuffers`／`GetBuffer`／`Present` 等 **7 箇所**の `?` で実 D3D/DXGI 失敗を返し得るが（2026-08-23 の再計測でも 7・実行は :200／:203／:204／:211／:228／:231／:238）、`SwapChainPresenter` は trait を持たない具体型（:122）で `presenter/target.rs:73` が `Option<SwapChainPresenter>` として直接保持し、注入点が無い。唯一の消費点 `presenter/show.rs:306-310` は失敗時「表示は前状態を保つ」と主張しているが未検証（既存 `upload` テストは成功経路のみ・実 GPU 必須）。この分岐は `dpi-transition-atomicity` が本仕様の観測点として意図的に不動で残した。

### 隣接 spec からの申し送り（本仕様が引き継ぐもの）
- `dpi-transition-atomicity`（追記(72)(76)）: 観測 target `wintf::transition` の語彙不変条件（窓種別は `win_kind=`・1 行に同名フィールドを 2 度出さない）／決定論テストは一括 flush（実 `SetWindowPos`／`DeferWindowPos`）に到達しないので `kind=write` 行を数えるテストは退行を捕まえない／多フレーム駆動ハーネス `FrameHarness`（`frame_test_support.rs`）とその自己テスト（`frame_harness_tests.rs`）は作り直さない／`presenter/show.rs` の観測前置ガードは本文走査テスト（`transition_record_tests.rs`）だけが守っている。**µs の上限は引き継がない**。
- `dpi-transition-atomicity` 追記(76)⑹＋W6.9 同居裁定: `crates/wintf/src/ecs/window/command.rs` の自発書込カウンタ `SELF_INITIATED_DEPTH` はプロセス共有 `AtomicI32` で、意味論はスレッド局所なのに並列テストが互いを汚染していた（上流実測 60 回中 11 失敗）。是正（`Cell<i32>` 化）は **`command.rs` を丸ごと所有する `draw-load-parity` が実施**し、本仕様は症状側＝テスト側の錠 `lock_self_initiated_for_test()` の退役だけを受ける。
  - **2026-08-23 に着地済み（PR#118・`main` 取り込み `76384c83` で確認）**: カウンタは `thread_local! { static SELF_INITIATED_DEPTH: Cell<i32> = const { Cell::new(0) }; }`（`command.rs:70`）へ移行。錠の定義は `command.rs:104`（`:76` から移動）、**実呼出は 21 箇所／5 ファイルで不変**（`command.rs` 2＝`:961`／`:973`・`command_batch_tests.rs` 5・`command_transition_tests.rs` 4・`window_proc/window_pos_tests.rs` 5・`window_proc/window_pos_transition_tests.rs` 5。2026-08-23 の着手時再計測でも 21／5 で、字面 22 件のうち 1 件は `command.rs:104` の定義）。錠を取らずに並列実行しても緑であることは `draw-load-parity` が新設した `command_threadlocal_tests.rs` が固定済み。**よって要件 7 は分岐⒝（着地）で確定**し、7.2 が実施対象になる（7.1 の「未着地なら非接触」は充足済みの前提として残す）。
  - 同着で申し送られた陳腐化: 兄弟テスト 4 ファイルの説明文が「**プロセス共有**の `SELF_INITIATED_DEPTH`」のまま残っている（`command_batch_tests.rs:25`・`command_transition_tests.rs:28`・`window_pos_transition_tests.rs:21`・`window_pos_tests.rs:40`）。錠の退役と同じ塊で是正する（要件 2.4 の対象）。
- `bindoption-exclusivity`: ログ存在主張で間欠赤だったテスト 6 本（`bind_apply_on_shown_emits_show_and_info_marker`・`bind_default_exclusive_replace_emits_show_and_info_marker`・`non_shell_broadcast_reception_is_benign_debug_no_warn_error`・`wait_broadcast_reception_is_benign_debug_no_warn_error`・`progress_phase_bind_drop_emits_info_marker`・`residual_frame_removal_emits_info_marker`）を本仕様の担当クラスとして登記。現行ツリーではいずれも硬化済みヘルパ経由になっているが、**反復実行で緑が安定したことは未確認**。
- `ghost-window-zorder`／W6: `balloon-visibility` が再表示シーム `ReassertZOrder` を消費せずに着地（再表示直後のバルーン隣接は実機未確認）。決定論テストで拾える範囲を本仕様で検討し、拾えなければ e2e へ申し送る。
- `kero-balloon`: `cargo test -p areka` が 1 回だけ 553/1 で赤（テスト名不明・ログ未保存）。①硬化後に再現しなくなるかを反復検証で確認する。
- roadmap 追記(79): 「1 ファイル 1,000 行以下」の目安（`structure.md:176`）に機械的な番人が無く漂流している（現行ツリーで 1,000 行超は **11 ファイル**（2026-08-23 の再計測でも同じ 11 ファイルで、各ファイルの行数も 2026-08-22 と一致）＝roadmap 表の 9 本に `plan_ops_tests.rs` の再増・`inproc_e2e_test.rs`・`pilot` example が加わる）。3 択（⒜ 番人テスト／⒝ 掃除 spec／⒞ 目安の緩和）は **2026-08-22 の要件ディスカッションで ⒜ に裁定**され、置き場は本仕様。要件 10 は採用確定の要件として置く（見張りだけを作り、既存ファイルの分割はしない）。

## Boundary Context

- **In scope**:
  - ③ ログ捕捉の硬化設計を**ワークスペースで 1 つ**に定め、全 crate（`wintf` と bin crate `areka` を含む）から同じ機構を引けるようにする。
  - ① 上記 24 ファイルの捕捉サイトを共有機構へ移行し、誤った説明文を全件是正する（硬化済みファイルに残る旧説明文を含む）。着手時にワークスペース全体を再計測し、本文書のインベントリを現在値で更新する。
  - ② 分割先 2 箇所の反復回数固定 settle を壁時計または観測量で有界化する（Tick 注入を兼ねる形は Tick 生成と上限を分離し、注入時刻が観測を追い越さない形を保つ）。
  - ④ 表示更新（`upload`）の失敗を実行テストから注入できる形を設け、「失敗時に表示は前状態を保つ」を実行テストで証明する。
  - 共有機構を迂回する新規の捕捉ヘルパ／直書きが増えたら赤になる再発防止。
  - `SELF_INITIATED_DEPTH` 是正（`draw-load-parity` 実施）後の錠 `lock_self_initiated_for_test()` の退役。
  - 1 ファイル 1,000 行の目安の番人テスト（追記(79) ⒜＝2026-08-22 開発者裁定で採用確定。作るのは見張りだけ）。
- **Out of scope**:
  - 本番の挙動変更（④ の注入点と、要件 5.7 が許す `chain.rs` 内で閉じる `upload` の前状態保持の是正を除く）。とくに `crates/wintf/src/ecs/window/command.rs` は**非接触**（`draw-load-parity` 所有）。
  - 既存テストの**判定内容**の変更（直すのは観測機構と待機機構だけ）。観測が正しくなった結果として落ちるテストは本物の欠陥の発見であり、テストを緩めて通さず別途起票する。
  - `spine` 系テストの削除・`areka-ghost` 側の待機（2026-07-30 に是正済み）・`dpi-transition-atomicity` の実機未達 µs 2 系統（`present-write-coherence` 所有）・`presenter/show.rs` の可視化の段（:375-392・`present-write-coherence` 所有）。
  - 新規外部依存の追加（`tracing-subscriber` は既出）。
  - 既存の 1,000 行超ファイル（11 件）の分割・縮小（番人テストの例外表に載せるのみ。並走 spec との衝突を避けるため）。
- **Adjacent expectations**:
  - **実装の開始時期（2026-08-22 要件ディスカッションでの取り決め）**: 開発者が別セッションで修正範囲未定の改善ループを回す予定のため、本仕様の**実装はその結果が `main` へマージされた後に開始**する（要件・設計の文書作業は先行してよい）。本仕様は 24 ファイル以上のテストコードと各 crate の `Cargo.toml` に触れるので、範囲未定の並走とは衝突しやすい。着手時の全面再計測（要件 2.1・4.1）でマージ後の実形を取り込む。改善ループ側がログ捕捉テストの仕組みに触れないことが望ましい（触れた場合は二重作業になるため着手時に突合する）。
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
7. If 失敗注入のテストが現行の `upload` で「前状態を保つ」が破れる経路（例: 寸法変更後の後段失敗で swap chain・`source_tex`・`size` が不整合のまま残る）を露見させる, then the 実装者 shall その是正を本仕様で行う（2026-08-22 要件ディスカッション 議題 2 の裁定）。ただし是正は `chain.rs` の内側で閉じ、`presenter/show.rs` には触れず、要件 5.5 の性能特性（定常状態のアロケーション 0）を保つ。
8. If 上記の是正が `chain.rs` の外（`show.rs`・`target.rs` 等）へ波及しなければ成立しないと判明する, then the 実装者 shall 本仕様では是正せず、要件 6.2 に従い別途起票し、注入テストは現状の挙動を記録する形で残す（主張を弱めた緑にしない）。
9. The 本仕様 shall 次の 2 件を「前状態を保つ」の既知の残余として扱い、是正せず実行テストで現状の挙動を期待値として固定する（2026-08-22 設計ディスカッション 議題 2 の開発者裁定）: ⒜ 最終段 `Present` の失敗＝表示は前フレームのまま・内部の作業用テクスチャは試行内容を持ち次回成功で追いつく（完全復元は定常経路の毎フレーム複製を要し 5.5 に抵触）／⒝ 外形変更経路で `ResizeBuffers` 成功後の後段失敗＝内部状態は旧値で自己整合・swap chain の表示バッファだけ新寸・未描画で次回 `upload` が回復する（実デバイスでは実質起きない経路）。

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
4. **（2026-08-27 開発者裁定により改訂）** The 本仕様 shall 錠の退役後に不要となった錠の定義 `lock_self_initiated_for_test()`（`crates/wintf/src/ecs/window/command.rs`。**起草時に記した `:98-102` は誤り**——それは `#[cfg(test)]` と関数本体だけを指しており、この関数の説明文 `:73-97` を残す。項目の実体は `:73-102` で、2026-08-27 のタスク 9.1 は直後の空行を含む 31 行を削除した）を**本仕様で削除する**。**旧条文**（「その削除は `command.rs` の所有者（`draw-load-parity`）へ申し送り、本仕様では行わない」）は引受先が実在しないため破棄した——`draw-load-parity` は 2026-08-23 に完了・アーカイブ済みで申し送りを消化できず、かつ同 spec の `design.md:226` は逆に本仕様へ委ねており、**互いに相手へ委ねる閉ループの片端がアーカイブ済み**という形だった。同一項目は `dpi-transition-atomicity` でも 1 度落ちている（同 spec `mechanism-ledger.md:769` が「登記が行われておらず、引受先も 0 だった」と最終ゲートの実測で記録）。**呼出を 0 にしたのは本仕様のタスク 7.2 であり、死なせた側が片付ける**という裁定である。削除に伴い、同関数の doc コメントにある「定義そのものの扱いはタスク 8.3 で開発者が裁定する」の記述も同じ変更で除去する（裁定が済んだ後に残ると、存在しない判断を指し続けるため）。

### Requirement 8: 再発防止（共有機構を迂回する捕捉の新設検知）
**Objective:** 後続 spec の実装者として、共有機構を迂回した捕捉ヘルパや直書きを新設すると即座に赤になることを求める。それにより「後置するほどコピーが増える」構造が止まり、本仕様の成果が次の spec で崩れない。

#### Acceptance Criteria
1. When ワークスペースのテストコードに共有機構を経由しない `tracing::subscriber::with_default` の直接呼出が新設される, the テスト基盤 shall 実行テストで検知して失敗させる（やむを得ない例外は理由付きで列挙。**2026-08-24 訂正**: 起草時は「既定で例外表は空」と書いたが、実測では移行不能な 3 件＋較正 1 件の計 4 件で開始する。design.md  はタスク 3.7 で訂正済み）。
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

### Requirement 10: 1 ファイル 1,000 行の目安の機械的な番人（採用確定・2026-08-22 開発者裁定＝追記(79) ⒜）
**Objective:** リポジトリの構造規律を保つ開発者として、「1 ファイル 1,000 行以下」の目安（`structure.md:176`）が機械的に見張られることを求める。それにより規則があるのに誰も測っていない状態が解消され、新たな超過に即座に気づける。

**裁定の要点（要件ディスカッション 議題 1）**: 本仕様が作るのは**見張り（番人テスト）だけ**である。現に超過している既存ファイルの分割・修正は本仕様では行わない（並走 spec との衝突を避けるため）。例外表は既知の超過ファイルで始めるので、番人の導入時点でテストは赤にならない。番人が `main` に入った後に合流する spec が新たに 1,000 行を超えるファイルを作れば、その spec 側で赤になる（それが番人の役割）。roadmap 追記(79) への裁定の反映は次回の棚卸（別セッション）で行う。

#### Acceptance Criteria
1. The テスト基盤 shall `crates/**/*.rs`（`src/`・`tests/`・`examples/` を含む）の各ファイルの行数を測り、1,000 行を超え例外表に無いファイルがあれば失敗する実行テストを 1 本置く。
2. The 例外表 shall 着手時点の超過ファイル（2026-08-22 時点 11 ファイル＝`pilot` の example と `areka-ghost/tests/ghost/inproc_e2e_test.rs` を含む）を列挙して開始し、項目の追加は明示的な編集としてのみ許し、削除（分割による解消）のみを自然な方向とする。
3. The 番人テスト shall 既知の超過ファイルを一時的に例外表から外したとき赤になることを自己テストで固定する。
4. The 本仕様 shall 例外表に載る既存の超過ファイルを分割・縮小しない（既存ファイルの分割は本仕様の範囲外であり、必要なら別途起票する）。
5. When 番人テストが導入される, the 本仕様 shall 並走中の `draw-load-parity` へ「合流後に新規の 1,000 行超ファイルは赤になる」ことを申し送る。

### Requirement 11: 隣接 spec からの申し送りの遵守
**Objective:** 並走・後続 spec（`draw-load-parity`・`present-write-coherence`・`balloon-offset-dpi`・`emo2-conformance-e2e`）の実装者として、本仕様が上流の語彙・前提を壊さず、拾えなかった確認事項を明示的に引き渡すことを求める。それにより本仕様の後で各 spec の前提が食い違わない。

#### Acceptance Criteria
1. When 本仕様が `wintf::transition` の観測行を足す・共有ヘルパへ寄せる, the テスト基盤 shall 語彙不変条件（窓種別フィールドは `win_kind=`・1 行に同名フィールドを 2 度出さない）を守り、既存の逐語テストを緑のまま保つ。
2. The テスト基盤 shall 決定論テストで窓書込の回数を数える場合にキュー側で数え、一括 flush 由来の `kind=write` 行を判定根拠にしない。
3. The テスト基盤 shall 多フレーム駆動ハーネス `FrameHarness` とその自己テストを作り直さず、捕捉層のみを共有機構へ寄せる。
4. When `balloon-visibility` が消費しなかった再表示シーム `ReassertZOrder` の隣接確認を検討する, the 本仕様 shall 決定論テストで固定できる範囲を確定して実装し、固定できない範囲は `emo2-conformance-e2e` へ理由付きで申し送る。
5. The 本仕様 shall `draw-load-parity` と共有ファイル 0 を保ち、各 crate の `Cargo.toml` への変更を dev-dependencies の追加に限定する。
6. When 本仕様が完了する, the 本仕様 shall 後続 spec が共有機構でテストを書くための利用手順（どこから引くか・不在主張の書き方）を文書に残す。
7. If 本仕様のテストが共有の起床旗（`crates/wintf/src/ecs/world/tick_wake.rs`）に触る、または tick 実行判定（`EcsWorld::decide_tick`・`world/mod.rs:551`）へ到達する, then the テスト基盤 shall 既存の唯一の錠 `ecs::world::TICK_WAKE_TEST_LOCK`（`world/mod.rs:931`）を取り、2 本目の錠を新設しない（`draw-load-parity` の実装中に錠が 2 本へ分裂した実例がある）。
8. The 本仕様 shall 共有の起床旗の上で「旗が立っていない」という不在主張を書かない（本番経路が旗を立てるようになったため成立しない）。省略側の主張が要る場合は注入口（`tick_one_frame_with`・`tick_bridge.rs:230`／`EcsWorld::decide_tick_with`・`world/mod.rs:560`）で行う。2026-08-23 時点で本仕様の接触集合のうち起床旗を立てる本番経路は `emo2_boot/adapter.rs:122` と `emo2_boot/balloon_visibility_phase.rs:113-114` の 2 箇所で、既存の待機テストはいずれも旗を観測しない（要件 4 の 2 箇所は現状のままで本条に抵触しない）。

### Requirement 12: テスト用一時パスのプロセス間衝突の解消（2026-08-27 開発者裁定で本仕様へ追加）
**Objective:** 本仕様が用意した反復検証の仕組みを実際に使う開発者として、同じテストを複数プロセスで同時に走らせてもテスト同士が一時ファイルを奪い合って落ちないことを求める。それにより「負荷をかけた反復」という本仕様の成果物が、それ自身の副作用で使えなくなる状態を解消する。

**裁定の背景**: タスク 8.2 が要件 9.5 の追跡で `cargo test -p areka` を同時 4 プロセスで 30 回反復したところ、**30 回中 3 回が赤**になった。原因は捕捉でも待機でも退役した錠でもなく、**プロセス内では一意だがプロセス間では共有される固定の一時パス**である（`crates/areka/src/placement/transition_signoff_tests.rs:102` の固定ファイル名、`crates/areka/src/main_restore_seam_tests.rs:16-20` の `unique_temp_dir`——後者は名前に反しプロセス間では一意でなく、`plant_minimal_ghost`（`:24`）が冒頭で `remove_dir_all` するため隣のプロセスの前提を消す）。8.2 の時点では「`crates/` の変更なので範囲外・別 spec へ起票」としたが、**2026-08-27 の裁定で本仕様が全解決する**ことになった。要件 10.4（既存ファイルを分割・縮小しない）は 1 ファイルの行数の目安に固有の制約であり、本要件はその適用範囲外である。

#### Acceptance Criteria
1. The テスト基盤 shall テスト用の一時パスを**プロセス間で一意**に組み立てる窓口を 1 つ用意する（プロセス識別子と単調増加の連番を名前に含め、後始末を伴う）。既に実在する正解の型（`crates/areka/src/placement/placement_shared_test_support.rs:41-68` の `TempDir`＝`AtomicU32` の連番＋`std::process::id()`＋`Drop` での再帰削除）を基準とし、crate をまたいで引ける位置へ置く。**発明ではなく移植である**（同じ型は 2026-08-27 時点で 16 ファイルに実在する）。
2. The テスト基盤 shall `std::env::temp_dir()` からテスト用のパスを組み立て、**かつ書込または削除を行う 20 ファイル**をこの窓口へ移行する（2026-08-27 実測。内訳: `areka` 6・`areka-ghost` 12・`areka-parsers` 2）。読み出しのみの 2 ファイル（`crates/areka/src/placement/placement_monitor_tests.rs`・`crates/shiori-host32-host/tests/error_paths.rs`）は衝突し得ないため対象外とし、その判定根拠を記録する。

   **2026-08-27 訂正（タスク 10.5 のレビューが掘り当てた・対象は 20 → 21 ファイル）**: 上の「20」を出した絞り込みは**ファイル単位**だった——「ファイルのどこかに `std::process::id()` があれば一意化済みとみなして除外する」。これは 1 つのファイルの中で**一意名の箇所と固定名の箇所が混在する**形を丸ごと取りこぼす。実際に `crates/areka-sylphya/src/persist/io.rs` が漏れていた（入口 3 箇所のうち識別子を使うのは `:225-229` の 1 箇所だけで、`:195-197` と `:212-214` は**固定名で書込・削除を行う**）。とくに `:199-204` は「固定名へ `"first"` を書く → 読み戻して検査 → `"second"` を書く → 読み戻して検査」であり、**2 プロセスが同時に走れば A の読み戻しの前に B の書込が挟まって落ちる**——本要件を新設させた事故（`:186` 記載の同時 4 プロセス 30 回中 3 回の赤）と原理的に同じ形である。したがって同ファイルを**移行対象に加える**（内訳: `areka` 6・`areka-ghost` 12・`areka-parsers` 2・`areka-sylphya` 1）。**残る混在ファイル 6 本は実測で安全**（`shiori_proxy.rs`・`process_host.rs`・host32 の e2e 4 本＝いずれも書込を伴う箇所はすべて識別子を含み、識別子を持たない箇所は「実在するディレクトリ」として読むだけ）。**なおこの「6 本」はコメントを除去しない素の走査での数**で、host32 の e2e 4 本は入口に見えた行が説明文だったため、**コメント除去後に入口が複数あるのは `shiori_proxy.rs`（3 箇所・識別子 2）と `process_host.rs`（3 箇所・識別子 1）の 2 本だけ**である（要件 12.4 の見張り側はコメント除去後で数えるので「2 件」と書く。数え方の違いであって食い違いではない）。**教訓は要件 12.4 の見張りにもそのまま当てはまる**（同じファイル単位の穴を持つ）ので、見張り側にはその限界を明記させる。
3. The 移行 shall 各テストの既存の主張・期待値・テスト本数を変えない（要件 6.1 と同じ規律）。本番コード（`#[cfg(test)]` の外）の挙動は 1 行も変えない。
4. The テスト基盤 shall 窓口を迂回する新設（`std::env::temp_dir()` から固定名を組み立てる箇所）を検知して失敗する実行テストを 1 本置く。例外表は移行後の実測値で開始し、項目の追加は明示的な編集としてのみ許す（要件 8.2・10.2 と同じ形）。
5. The 検知テスト shall 既知の陽性例で赤になること自体を自己テストで固定する（要件 8.4・10.3 と同じ較正の規律）。
6. When 移行が完了する, the 検証 shall `cargo test -p areka` を同時 4 プロセスで **30 回以上**反復し、一時パスの衝突に起因する失敗が 0 件であることを要件 9 の仕組み（`verification/repeat-tests.ps1`）で示す。
7. The 本仕様 shall 移行対象の判定に用いた走査式を較正する。**走査語がコメント中に現れて判定が反転する事故が本仕様の調査中に実際に起きている**——`crates/areka/src/main_restore_seam_tests.rs:15` の「外部 tempfile 非依存」というコメント中の `tempfile` が絞り込み式に拾われ、**実際に落ちている当のファイルが候補から外れた**。タスク 6.1 が同型の罠（コメント除去の要否）を既に解いているので、その部品を用いること。**2026-08-27 注記**: この根拠のコメント行そのものは**タスク 10.2 の移行で消えた**（当該ファイルが窓口へ寄ったため）。事故を再現したいときは `git show <10.2 の親>:crates/areka/src/main_restore_seam_tests.rs` の `:15` を見ること——素の走査は拾い、コメント除去後は拾わない、が逐語で再現する。

### Requirement 13: ログ有効判定の常時化の費用の測定（2026-08-27 開発者裁定で本仕様へ追加）
**Objective:** 本仕様の完了を承認する開発者として、硬化の代償として支払っている実行時間が数字で示されていることを求める。それにより「速くなったか遅くなったか分からないまま硬化を常設する」状態を解消する。

**裁定の背景**: design.md `:605`（R-3）は `cargo test --workspace` の所要時間を移行前（`main`）と比較して記録することを求めていた。タスク 8.2 がこれを実施し **移行前 39.6 秒 対 移行後 41.7 秒（+2.1 秒 / +5.3%・各 5 回の中央値）** を得たが、**差の出所を分離できなかった**——移行後はテストが 111 件・実行体が 6 本多く、移行前側の測定値自身の散らばり（34.6〜77.8 秒）が差の 20 倍ある。さらに移行前ツリーは素の `cargo test --workspace` で完走しない（赤が出て cargo が 92 本中 58 本で打ち切る）。2026-08-27 の裁定で**同一テスト集合による測り直しを本仕様で行う**。

#### Acceptance Criteria
1. The 測定 shall **同一のテスト実行体・同一のテスト集合**に対して、常駐の仕掛け（`crates/log-capture-kit/src/probe.rs:95` の `ensure_interest_probes`）が有効な場合と無効な場合の所要時間を比較する。**移行前ツリーとの比較は用いない**（集合が揃わないことがタスク 8.2 で実証された）。
2. The 測定 shall 常駐を無効にした側で赤になるテスト（硬化なしでは取りこぼす捕捉テスト群）を特定し、除外して比較するか、除外できない場合はその影響量を測って記録する。**赤を含んだままの所要時間を比較値として採らない。**

   **2026-08-27 追記（タスク 11.1 の実測で特定の手段を替えた）**: 起草時は「無効側で実際に赤になったテストを観測して除外する」を想定していたが、**赤の集合は非決定である**ことが実測で判明した（8 回の走行で不変に赤なのは較正テスト 1 本だけ・他は 1/8〜5/8 で出入り・5 本除外すると 6 本目が出る。独立の 3 回では 0 件 / 2 件 / 0 件）。無効側の赤は**毒化の競合**が起こすもので、それは本仕様が消そうとしている病そのものだから、並列の巡り合わせに依存するのが当然だった。したがって特定は**観測ではなく静的な列挙**で行う——**共有捕捉窓を使うテストを走査で列挙し、両側から同じ集合を除外する**。本条文の「特定し、除外して比較する」はこれで満たされる（変えたのは特定の手段だけ）。なお前段の性質——**取りこぼした窓が黙って空を返さず失敗を宣告すること**——は実測で成立している。
3. The 測定 shall 反復して中央値と散らばりの両方を採り、**差が散らばりの範囲に埋没する場合はその旨をそのまま結論とする**（差が出たことにしない）。
4. The 測定 shall 要件 9 の反復の仕組み（`verification/repeat-tests.ps1`）で実行し、記録を `verification/summary.md` に残す。
5. When 測定が完了する, the 本仕様 shall 得られた数字と、**その数字が何を測っていて何を測っていないか**を申し送り台帳へ登記する。

---

## 申し送り台帳

> 本仕様の実装中に確定した判定・残余・引受先の登記（design.md `#### C7`）。項目は「担当先未定」で残さない。
> 以降のタスク（とくに 8.3）はこの節へ追記する。

### ⑴ 要件 7 の分岐判定（タスク 7.1・2026-08-24 実測・HEAD `79527213`）

**判定: 着地（分岐⒝）。したがってタスク 7.2 は実施対象**（錠を温存する分岐⒜・⒞ はいずれも不発）。

| 判定材料 | 実測値（2026-08-24） | 根拠 |
|---|---|---|
| 自発書込カウンタの型 | **スレッド局所**＝`thread_local!` の `Cell<i32>` | `crates/wintf/src/ecs/window/command.rs:49`（`thread_local! {`）・`:70`（`static SELF_INITIATED_DEPTH: Cell<i32> = const { Cell::new(0) };`） |
| 所有 spec の状態 | `areka-P0-draw-load-parity` は **completed**（アーカイブ済み） | `.kiro/specs/completed/areka-P0-draw-load-parity/spec.json`（`"phase": "completed"`・`implementation.completed_at` = `2026-08-23T10:30:00Z`） |
| 着地したコミット | PR#118 の squash `327e7fd3`。`Cell<i32>` の行と `command_threadlocal_tests.rs` の新設はいずれも同コミットが初出で、本ブランチの先祖 | `git log -S 'static SELF_INITIATED_DEPTH: Cell<i32>'`／`git log --diff-filter=A`／`git merge-base --is-ancestor 327e7fd3 HEAD`。取り込みは `76384c83` |
| 「錠なし並列でも緑」を固定する新テスト | 実在。3 本・**3 passed / 0 failed** | `crates/wintf/src/ecs/window/command_threadlocal_tests.rs:37`・`:90`・`:127`（`cargo test -p wintf --lib command_threadlocal_tests`） |
| 錠の実呼出 | **21 箇所 / 5 ファイル**（2026-08-23 の `verification/remeasure.md` §5 と増減なし） | 内訳は下表 |
| 錠の定義 | ~~`crates/wintf/src/ecs/window/command.rs:99`~~（`pub(crate) fn lock_self_initiated_for_test()`）。**2026-08-24 訂正**: 起草時の `:104` はタスク 7.2 の編集で `:99` へ動いた | **2026-08-27 削除済み**（タスク 9.1・改訂後の要件 7.4）。この行の file:line はもはや指す先を持たない。旧記載「本仕様では削除しない」は引受先不在のため破棄 |
| 「プロセス共有のカウンタ」と書いたまま残る説明文 | **4 件**（要件 2.4 の対象・7.2 が是正） | `command_batch_tests.rs:25`・`command_transition_tests.rs:28`・`window_pos_tests.rs:40`・`window_pos_transition_tests.rs:21` |

錠の実呼出の内訳（`let _serialized = …lock_self_initiated_for_test();` の実行行のみ。doc コメント中の参照は数えない）:

| ファイル | 実呼出 | 行 |
|---|---|---|
| `crates/wintf/src/ecs/window/command.rs` | 2 | :961, :973 |
| `crates/wintf/src/ecs/window/command_batch_tests.rs` | 5 | :322, :402, :466, :542, :637 |
| `crates/wintf/src/ecs/window/command_transition_tests.rs` | 4 | :302, :372, :408, :426 |
| `crates/wintf/src/ecs/window_proc/window_pos_tests.rs` | 5 | :44, :284, :318, :622, :651 |
| `crates/wintf/src/ecs/window_proc/window_pos_transition_tests.rs` | 5 | :192, :222, :299, :399, :519 |
| **合計** | **21**（うち兄弟テスト 4 本＝**19**） | |

`draw-load-parity` 自身の台帳（`.kiro/specs/completed/areka-P0-draw-load-parity/tasks.md:281`）が同じ内訳と同じ 4 件の陳腐化した説明文を挙げており、独立に採った本実測と一致する。なお `command_threadlocal_tests.rs:19` にも錠の名前が現れるが、これは「意図的に取らない」ことを述べた doc コメントで実呼出ではない。**数え方で 3 通りの数字が出る**ので注意する: 名前を素で走査すると **6 ファイル / 28 行**、`verification/remeasure.md` §5 の `\(` 付きだと **5 ファイル / 22 件**（実呼出 21 ＋ 定義 1）、実行行の形（§5-a）で **21**。

**道具の較正**: 上記 3 本が「スレッド局所であること」を本当に縛っているかを確かめた。`command.rs` は非接触の裁定下にあるので本体を変異させず、リポジトリ外（scratchpad）に ⒜ `thread_local! Cell<i32>` と ⒝ プロセス共有の `AtomicI32` の 2 つの形を再現し、当該テストと同じ 3 つの主張を両方へ掛けた。結果は ⒜ が 3 つとも真（緑）・⒝ が 3 つとも偽（赤）＝主張は 2 つの形を区別する。ただし本体の `nested_guards_on_one_thread_stay_true_until_the_last_is_dropped` は自分では同時に持ち上げる相手を作らないため、その「新しいスレッドのカウンタは 0 から始まる」の主張が共有を検知するのは他テストが同時に持ち上げているときだけである。無条件に検知するのは残る 2 本（別スレッドの持ち上げが見えないこと・2 本同時でも主スレッドへ漏れないこと）。

**⚠ 要件 7.4 の引受先が実在しない（タスク 7.1 で発見・8.3 で決着が要る）**

要件 7.4 は錠の**定義**（`command.rs:104`）の削除を所有者 `draw-load-parity` へ申し送ると定めるが、`draw-load-parity` は 2026-08-23 に完了しアーカイブ済みで、申し送りを消化できない。`.kiro/specs/` 直下の進行中 spec で `crates/wintf/src/ecs/window/command.rs` の所有を主張しているものも無い——当該パスに言及するのは `areka-P0-present-write-coherence/brief.md:154` と `areka-P0-emo2-conformance-e2e/brief.md:42,171` の 3 箇所だけで、いずれも**本番側の観測点の列挙**（窓書込指令の積み上げ点・Z 指令が合流対象外であること）であって、テスト専用の錠の定義を引き受ける宣言ではない。

本仕様の範囲は変えない（**定義は削除しない**）。ただし 7.4 の「所有者へ申し送る」を満たすには実在する引受先が要るので、8.3 で次のいずれかを開発者裁定で決めること: ⒜ 進行中 spec のいずれかへ引き受けさせる／⒝ 新規に起票する／⒞ 定義の残置を裁定する。⒞ が成り立つ根拠は、定義が `#[cfg(test)]` でありワークスペースに `-D warnings` が無い（`.cargo/config.toml` 不在・root `Cargo.toml` と `crates/wintf/Cargo.toml` に `[lints]` 節が無い）ため、呼出が 0 になっても `dead_code` は警告どまりで赤にならないこと。

> **【決着済み・2026-08-27】上の 2 段落は 2026-08-24 時点の記録であり、判断待ちの項目としては閉じている。** 開発者裁定は上記 3 択のいずれでもなく **⒟「本仕様で定義を削除する」**（改訂後の要件 7.4）で、タスク 9.1 が実施した。**判断待ちのまま残っている項目はここには無い。** 実施の詳細は下記「⑶-A 実施記録」の A-1 を見ること。

### ⑵ 着手時点の `origin/main` との差（タスク 7.1 の付随確認）

`origin/main` は `12afa8e6`（2026-08-24 16:37）で HEAD より 1 コミット先行しているが、当該コミットは要件 7 の対象 6 ファイル（`command.rs`・兄弟テスト 4 本・`command_threadlocal_tests.rs`）を 1 行も触っていない（`git diff --name-only HEAD...origin/main -- <6 ファイル>` が空）。判定には影響しない。

### ⑶ タスク 12.1 の登記（2026-08-27・HEAD `f26d5699`・本仕様の最後の登記）

本節は本仕様が抱えた未決の項目をすべて 3 つの札のいずれかへ振り分ける: **【実施】**（本仕様で片付いた）・**【引受】**（実在する進行中 spec が受ける）・**【起票】**（受け皿が実在しないので新規に立てる）。**「担当先未定」「引受先未定」で残した項目は 1 つも無い。**

> **⚠ 登記は届け先への転記まで行って初めて消化される。** 本仕様は「互いに相手へ委ねる閉ループの片端がアーカイブ済み」という形を 2 度踏んでいる（要件 7.4 の経緯・`.kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md:769` が「登記が行われておらず、引受先も 0 だった」と最終ゲートの実測で記録）。**【引受】の各項目は、本仕様の完了処理の際に受け先の brief へ転記すること。** **この転記は 2026-08-27 に実施済みである**——`.kiro/specs/areka-P0-emo2-conformance-e2e/brief.md` の末尾へ「## 申し送り（areka-P0-test-cage-determinism・2026-08-27）」を置き、**B-1／B-2／B-3 の 3 件を転記した**（転記の理由もそこへ書いた＝台帳にだけ書いて受け手が知らない状態は 3 度目になる）。起草時は本タスクの接触集合の外だったため未実施と書いていたが、**レビューを受けて境界を広げて実施した。**
>
> 引受先はすべて実在を実測で確かめた（`.kiro/specs/` 直下＝進行中は 8 本で、うち本仕様以外の 7 本は brief のみの未着手。`.kiro/specs/completed/` にある spec は申し送りを消化できないので引受先に採らない）。

#### ⑶-A 実施記録（本仕様で決着した）

**A-1【実施】錠の定義の削除（要件 7.4・2026-08-27 の開発者裁定で改訂）**

呼出 21 → 0 はタスク 7.2、定義の削除はタスク 9.1（群 9）が実施した。**申し送りではない。** 実際に削除したのは `crates/wintf/src/ecs/window/command.rs` の `:73-102`（説明文 25 行＋`#[cfg(test)]` 1 行＋関数 4 行）と直後の空行の**計 31 行・追加 0 の純削除**。起草時に 3 文書が書いていた範囲（tasks.md `:88-102`・requirements.md 7.4 `:98-102`・design.md C5 `:88-102`）は**三者三様でどれも実物と違い**、`:88` から切ると説明文の前半が孤児 doc コメントとして残り「documentation comment that doesn't document anything」でコンパイルが落ちる形だった（3 文書とも実測値へ訂正済み）。

削除で壊れる説明文内リンクが同 crate にもう 1 箇所あり（当時の `command_threadlocal_tests.rs:19`）、**`cargo test` にも `cargo clippy` にも映らず `cargo doc` だけが壊れる**種類の陳腐化だったので同じ変更で解消した。現在 `lock_self_initiated_for_test` の字面は `crates/` 全域で 1 件のみ＝`crates/wintf/src/ecs/window/command_threadlocal_tests.rs:20` の「意図的に取らなかった」という過去形の説明である（2026-08-27 実測）。

**A-2【実施】錠の退役の安全性の主張の正しい形（`verification/summary.md` §8 の受け渡し 4 件目・`tasks.md:441` の申し送り。`summary.md` §8 の表は起草時の `:331` を引いていたが、その位置は現在 `_Depends:` 行である）**

8.2 の記録は「static は 6 個・5 個が `thread_local!` の中・**例外は退役した錠の内側の `LOCK`（`command.rs:100`）1 個だけ**」だった。**群 9 がその錠を定義ごと削除したので、現在の正しい形は次のとおり**（2026-08-27 に実測で数え直した）:

| 値の static | 位置 | 収容 |
|---|---|---|
| `SELF_INITIATED_DEPTH` | `crates/wintf/src/ecs/window/command.rs:70` | `thread_local!`（`:49`） |
| `WINDOW_POS_COMMANDS` | `command.rs:225` | `thread_local!`（`:223`） |
| `FORCE_BATCH_BEGIN_FAILURE` | `command.rs:330` | `thread_local!`（`:324`） |
| `TICK_MIRROR` | `crates/wintf/src/ecs/window/transition_diag.rs:655` | `thread_local!`（`:648`） |
| `FLUSH_START` | `transition_diag.rs:728` | `thread_local!`（`:726`） |

**`command.rs` と `transition_diag.rs` の値の static は 5 個で、5 個すべてが `thread_local!` の中にある。例外は 0 個になった。** 8.2 が記録した位置（`:256`・`:361`）との差はちょうど 31＝群 9 の純削除の行数で、過不足なく整合する。素の走査（`static ` の字面）は `command.rs` で 5 行・`transition_diag.rs` で 10 行当たるが、静的寿命の型注釈が `command.rs` 2 件・`transition_diag.rs` 8 件混ざる——**走査語がそのまま使えない実例**なので、数えるときは値の宣言だけを採ること。結論（スレッド局所なので錠が無くても並列で干渉しない）は変わらず、実測は `r72-wintf` の 30 回全緑（`summary.md` §2）である。

**A-3【実施】一時パスの共有による赤（`summary.md` §8 の受け渡し 1 件目・8.2 では「引受先未定（8.3 で起票先を決めること）」）**

**2026-08-27 の裁定で本仕様の範囲へ入り、群 10（要件 12）が全解決した。起票は不要。** 数字は次のとおり。

- 病の実在: `cargo test -p areka` を**同時 4 プロセスで 30 回**回して **30 回中 3 回が赤**（タスク 8.2・`r95-areka`）。原因は捕捉でも待機でも退役した錠でもなく、**プロセス内では一意だがプロセス間では共有される固定の一時パス**。
- 取りこぼしていた 21 本目（`crates/areka-sylphya/src/persist/io.rs`）では、移行**前**の実行体を **4 プロセス同時 × 40 反復＝160 走行で 48 走行が赤（30.0%）**。赤 48 件はすべて同一テストで、落ちた行は要件 12.2 が名指しした `:199-204` そのもの。移行**後**は **160/160 緑**（タスク 10.7）。
- 是正後: 同一条件（同時 4・30 回・期待件数 1241・上限秒すべて同一）で **緑 30・赤 0**、所要秒の中央値も 21.3 → 21.2 でほぼ同じ＝**同等の負荷がかかったうえで赤が消えている**（タスク 10.6・`r106-areka`）。
- 全緑が空虚でないことの較正は**当日・同じ道具・同じ機械で使い捨ての赤を 2 回作って**確かめた（`cal106-red`）。2 日前の別 HEAD の赤の引用では代えない。
- 仕組み: 窓口 crate `crates/temp-path-kit/`（依存 0・`publish = false`）を新設し、**書込または削除を伴う 22 ファイル**を寄せた（`areka` 6・`areka-ghost` 13・`areka-parsers` 2・`areka-sylphya` 1。**起草の「areka-ghost 12」は数え落としで実測 13**）。迂回の検知は `crates/log-capture-kit/tests/temp_path_guard_test.rs`（例外表 `ALLOWED_ENTRY_POINT_USES` は `:146` から **16 件**＝一意化済み 13・読み出しのみ 2・固定名が仕様 1。件数は `ALLOWED_COUNT`（`:259`）に逐語で持たせ、増やすには 3 箇所の明示的な編集が要る）。
- **この spec で「本仕様が消すはずの失敗」を A/B で見せられたのは群 10 が初めてである。**

**A-4【実施】所要時間の分離（`summary.md` §8 の受け渡し 2 件目・8.2 では「引受先未定（要なら起票）」）＋要件 13.5 の登記**

**2026-08-27 の裁定で本仕様の範囲へ入り、群 11（要件 13）が実施した。起票は不要。** 原文は `verification/summary.md` の「11.2（採り直し）」節（`### 5. 数字`／`### 6. 結論`／`### 7. この数字が測っていないもの`）で、要約は次のとおり。

**測った数字**（同一ツリー・同一実行体・同一テスト集合で、常駐の仕掛けの有無だけを環境変数で切り替え、4 回ずつの区を交互に 6 区・各側 12 回）:

| 側 | n | 中央値 | 平均 | 最小 | 最大 | 母標準偏差 |
|---|---:|---:|---:|---:|---:|---:|
| A 常駐あり | 12 | **39.50 秒** | 38.42 | 31.6 | 39.9 | 2.30 |
| B 常駐なし | 12 | **39.20 秒** | 39.17 | 36.9 | 40.8 | 0.95 |
| 参考: 除外なし | 4 | 45.40 秒 | 45.70 | 45.0 | 47.0 | 0.82 |

**結論（要件 13.3）: 差は散らばりに埋没している。そのままを結論とする。** 中央値の差は **−0.30 秒（−0.76%）** だが、A 側単独の実測値の幅 8.3 秒は差の **28 倍**である。しかも ⑴ 中央値どうしの差（−0.30＝A が遅い）と位置対応の差の中央値（+0.50＝B が遅い）が**内部で符号が食い違い**、⑵ **同じ道具・同じ機械・同じ日に採った 1 巡目は +0.25 秒（+0.63%）で、採り直すと符号が反転した**。よって「速くなった」でも「遅くなった」でもなく、**上限として「あったとしても片側 12 回の中央値で 0.5 秒（約 1.3%）より小さい」としか言えない**。タスク 8.2 の +2.1 秒 / +5.3%（移行前ツリーとの比較）は、テストが 111 件・実行体が 6 本多いという**集合の違いの側に帰属する**と読むのが実測に合う。

**この数字が測っていないもの（要件 13.5 の本体・「測った、問題なかった」では済まない）**:

1. **除外した 1,447 個（`--skip` 188 個ぶん）のテストの所要は測っていない。** 除外なしの基準線 45.40 秒に対し除外後が 39.50／39.20 秒なので、**除外した分はおよそ 5.9〜6.2 秒＝全体の 13%** にあたる。そこに常駐の代償が乗っているかは本測定は何も言っていない（その集合は常駐なし側で赤になるので所要時間を比べられない）。**差し戻しで増えた 96 件はまさにこの測れない側である。**
2. **本番プロセスは 1 秒も測っていない。** 測ったのはテスト実行体だけ。「常駐の仕掛けは dev-dependency 経由でテストビルドにしか入らない」は別に立つ**構造の主張**（`with_default_guard_test.rs` の製品側依存の見張り）であって、本測定はその裏を取っていない。
3. **1 台の機械の 1 日の状態しか測っていない**（2026-08-27 の 13〜14 時台・同一ワークツリー・HEAD `87a640de`・cargo 1.98.0）。他プロセスの負荷は制御していない。**同じ日の 2 度の A/B で符号が反転したのが、この未制御の側の大きさの現れである。**
4. **同時に複数プロセスで走らせたときの差は測っていない**（同時 1 のみ）。常駐の代償が効くとすれば判定回数が増える経路なので、飽和状態は別の測定になる。
5. **テストスレッド数を変えたときの差も測っていない**（既定の並列度のみ）。
6. **どこに時間が行っているかは測っていない。** 測ったのは 1 回の壁時計の合計で、判定 1 回あたりの費用は見ていない。**「合計に出ない」は「1 回あたりが 0」を意味しない。**
7. **除外集合が十分だったことは、静的な走査が閉じている範囲までしか言えない。** **全緑は証拠にならない**——本件の 96 件は 24 回全緑のさなかに走り続けていた。

**A-5【実施】記録の正本の宣言（`summary.md` §8 の受け渡し 3 件目）**

**`verification/summary.md` を本仕様の検証記録の正本とする。縮約ファイルは作らない。** `verification/logs/` は非追跡なので squash-merge 後に残るのは `summary.md` と `verification/red/` だけになるが、タスク 8.2 の完了条件が言う「保存されたログで裏付けられている」の実体は次の 3 つで、いずれも追跡される: ⑴ **各回の結果行の要約値**が回ごとに 1 行ある（passed / failed / ignored / filtered / 実行体 の 5 列。**表そのものが縮約版である**）、⑵ **緑でなかった回**はテスト名・失敗の位置・失敗本文が本文へ転記される、⑶ その回の**生ログが `red/` へ複写**される。

**A-6【実施】リポジトリ構成の登記（steering の crate 一覧）へ 2 本を追加**

`.kiro/steering/structure.md` に **`log-capture-kit` と `temp-path-kit` の 2 節を新設**した。共有ログ捕捉 crate の説明文には、**その `tests/` がワークスペース全体の見張りの置き場になっている実態**（1,000 行の番人・共有機構の迂回検知・一時パス窓口の迂回検知の 3 本が走査部品 `workspace_scan/mod.rs` を共有する）を明記した——設計 `#### C6` の決定であり、crate 名が「ログ捕捉」だけを名乗る点との食い違いはこの記述で解消する。**一覧全体としての食い違いは C-2 に登記した。**

**A-7【実施】改行コードに依存して必ず赤になる逐語検査の是正（番外・2026-08-23 開発者裁定）**

`crates/wintf/src/ecs/world/tick_diag_tests.rs` の `try_tick_world_body` が本文の取り込みに対し LF 固定の照合語を掛けており、`core.autocrlf=true`・`.gitattributes` 無しの本リポジトリでは **CRLF チェックアウトで必ず赤**だった（着手前から赤・2 件）。所有は完了済みの `draw-load-parity` で引受不能、かつ「チェックアウト次第で赤緑が変わるテスト」は本仕様の主題そのものなので、**開発者裁定により本仕様で是正**した（コミット `40ee8460`・行末の CR を落として照合する形を新設）。以後の `cargo test --workspace` の基準線は**失敗 0 件**。**同型の LF 固定の照合語が他にも無いかの掃除は本仕様の範囲外**で、実害が観測されたら【起票】へ格上げする（現時点で観測 0 件）。

**A-8【実施】要件 6.2 の適用は 0 件（観測が正しくなった結果として恒常的に落ちたテストは無かった）**

要件 6.2 は「観測が正しくなって恒常的に落ちるテストは緩めずに欠陥候補として起票する」と定め、`research.md:15`／`:119`／`:275` は ④（表示更新の失敗注入）がその起票先になる見込みを書いていた。**結果は起票 0 件である。**

- ④ が露見させた「前状態を保つ」の破れは、要件 5.7 の裁定に従い**`chain.rs` の内側で閉じたまま是正**できた（タスク 5.1 が内部状態の一括更新を画素書込の直前まで遅らせた）。**要件 5.8（`chain.rs` の外へ波及するなら是正せず起票）の発動は無い**——実測: 分岐点 `327e7fd3` から HEAD までで `crates/areka-emo-present/` の変更は 15 ファイルだが、`presenter/show.rs` と `presenter/target.rs` は **1 行も変わっていない**。同 crate で唯一動いた本番ファイル `presenter.rs` の差分は **3 行の追加のみ**で、内容は `#[cfg(test)]` のテストモジュール宣言（`presenter_upload_failure_tests.rs` の登録）である。
- 既知の残余 2 件（要件 5.9 の ⒜ 最終段の提示の失敗・⒝ 外形変更成功後の後段失敗）は**是正せず現状の挙動を期待値として固定**した（2026-08-22 設計ディスカッションの裁定どおり）。
- 要件 6.3（不在主張が捕捉 0 件で通っていた形）についても個別のテスト追加は要らなかった。**共有捕捉窓が対照を内蔵している**ためで、`crates/log-capture-kit/src/capture.rs:71` が窓の内側で番兵イベントを発行し、`:82-84` がその捕捉を要求して落ちる——**捕捉が働いていなければ、不在主張のテストは静かに緑にならず窓そのものが失敗を宣告する。**

#### ⑶-B 引受（実在する進行中 spec が受ける）

**B-1【引受】再表示時の重なり順の再確認（要件 11.4）→ `areka-P0-emo2-conformance-e2e`**

**決定論的なテストで固定できる範囲は無い。** 実測で確かめた理由は 3 つで、いずれも本番配線の側の性質である。

1. **再表示の経路に重なり順の再指示が無い。** 再表示は `crates/areka/src/emo2_boot/balloon_visibility_phase.rs:446` → `crates/areka-emo-present/src/presenter/visibility.rs:69`（`show_target`）を通るが、この経路は再断行の要求を 1 度も挿さない（`crates/` 全域で当該要求の**挿入**は 1 箇所のみ）。
2. **挿入点は確立時の 1 発だけである。** `crates/wintf/src/ecs/window/zorder_pair_establish.rs:180` がそれで、直前のコメント（`:176-179`）が「確立は窓の生成直後・ペアにつき 1 度きり」と明記している。**固定すべき本番の判断分岐が再表示側に存在しない以上、決定論テストは恒真の檻にしかならない。**
3. **実際の隣接確認には実窓が要る。** 隣接は実 API の実測でしか読めず、しかも既定 IME 窓が owner の直上に居座るため「最も近い**可視**の隣」で測る必要がある（steering の既知知見）。

**引受先の実在を確認した**: `.kiro/specs/areka-P0-emo2-conformance-e2e/` は `.kiro/specs/` 直下（＝進行中・未アーカイブ）にあり、同 `brief.md:41` が「上流 `ghost-window-zorder` は `completed/` にあり**申し送りを消化できない**ため、**実機の見た目の側は本 spec が受ける**」「本 spec は既に zorder の残件を 1 件抱えている——**再表示直後の隣接が実機未確認**」と**自ら宣言している**。さらに `:53` が「**再表示直後の隣接を確認する好機は拡大率切替の直後である**」と手段まで書いている。本項目は同 spec が既に抱えている残件と同一物であり、本仕様は「決定論で固定できる範囲は 0 である」という判定と上記 3 つの根拠を足して引き渡す。

**B-2【引受】`areka-ghost` の統合テストの間欠赤（要件 9.4 の記録義務）→ `areka-P0-emo2-conformance-e2e`**

タスク 10.1 の `cargo test --workspace` の 1 回目で `crates/areka-ghost/tests/ghost/spine_e2e_test_s3_helper_liveness_detected.rs` の `assert_eq!(boot_prefix_len, 5, ...)` が `left: 4 / right: 5` で赤になった（2 回目は全緑・単独 5 連走も全緑）。**テスト名不明の赤にしないため、構造まで特定してある。**

**これは本仕様の要件 4 と同型の構造欠陥である**——有界スピン（上限は `:145`・`while` は `:146`・ブロックは `:163` まで）が待つのは**記録が非空になることだけ**で、その直後の 5 呼出の数え上げ（`:175-180`）と判定（`:181-185`）には**待機が 1 つも無い単発のスナップショット**である。`:133-142` のコメントは「5 本は先に完了しているはず」と論じるが、**それは仮定であって強制ではない**。負荷下で 5 本目が間に合わないと 4 のまま読まれる。`verification/logs/` の既存の全ログでこのテストは常に緑で、**赤の記録はタスク 10.1 が初**である。

**本仕様の範囲外**である根拠: Boundary Context が「`areka-ghost` 側の待機（2026-07-30 に是正済み）」を Out of scope に置いており、要件 4.1 の対象も `crates/areka/src/emo2_boot/spine*.rs` に限られる。

**引受先の実在を確認した**: 進行中 spec 7 本の brief を走査したが、このファイルに言及するものは 0 件だった。それでも `areka-P0-emo2-conformance-e2e` が引受先として成立する根拠は 2 つある——⑴ 同 `brief.md:140` が **Extends** に `completed/areka-P0-ghost-setup`（**spine e2e 母体**）を挙げており、当該母体はアーカイブ済みで消化できない、⑵ 同 `brief.md:104`／`:148` の DoD が **`cargo test --workspace` exit 0** を要求するので、**この間欠赤は同 spec の DoD を直接脅かす**。

**⚠ 起草時は本台帳にしか書かれていなかったが、2026-08-27 に受け手へ転記済みである**（`.kiro/specs/areka-P0-emo2-conformance-e2e/brief.md` の「## 申し送り（areka-P0-test-cage-determinism・2026-08-27）」。B-1／B-2／B-3 の 3 件とも入っている）。 冒頭に記した 2 度の取りこぼしと同じ形にしないため、本仕様の完了処理で `areka-P0-emo2-conformance-e2e/brief.md` へ転記すること。

**B-3【引受】子プロセスへの一時パスの受け渡しは静的検証止まり（タスク 10.3 の未検証）→ `areka-P0-emo2-conformance-e2e`**

一時パスの宛先が変わっても子プロセス（host-32 helper）への受け渡しが壊れていないことは、**実走の裏が取れていない**。実プロセスを起こす 2 本（`crates/areka-ghost/tests/ghost/real_pasta_test.rs`・`snapshot_capture_test.rs`）は環境変数の門で既定では走らないためである。**静的には確認済み**——受け渡しの配線（`crates/shiori-host32-host/src/process_host.rs:239-251` の引数・環境変数・作業ディレクトリ）は非接触、札は英数と `-` のみ、絶対パス、最長の合成名でも約 129 文字。**この未検証は正直に記録する。**

引受先の根拠: 実機で emo2 を一周走らせるのは `areka-P0-emo2-conformance-e2e` だけであり、同 `brief.md:104` の DoD は i686 host-32 成果物のビルド後の `cargo test --workspace` と実機 14 項目のサインオフを要求する。**実機サインオフの機会に門を開けて 2 本を 1 度通せば閉じる。** これも B-2 と同じく **2026-08-27 に brief へ転記済み**。

#### ⑶-C 起票（受け皿が実在しないので新規に立てる）

**C-1【起票】移行で未使用になった `tracing-subscriber` の dev-dependency 6 件の撤去**

共有機構への移行の結果、次の 6 crate の `[dev-dependencies] tracing-subscriber` が**未使用**になった（2026-08-27 実測。いずれも分岐点 `327e7fd3` では実際に参照があり、移行後は crate 全域の `.rs` で参照 0）:

| crate | 行 |
|---|---|
| `crates/areka-emo-atlas/Cargo.toml` | :21 |
| `crates/areka-emo-compose/Cargo.toml` | :22 |
| `crates/areka-ghost/Cargo.toml` | :39 |
| `crates/areka-kanade/Cargo.toml` | :26 |
| `crates/areka-seriko/Cargo.toml` | :28 |
| `crates/areka-sylphya/Cargo.toml` | :22 |

**本仕様では撤去できない**——要件 11.5 が `Cargo.toml` の変更を dev-dependencies の**追加**に限っており、`crates/log-capture-kit/tests/with_default_guard_test.rs` の `Cargo.toml` 検査も拾わない（見るのは「製品側依存に共有 crate が現れていないこと」と「濾過の feature を宣言してよいのは `wintf` のみ」だけ）。併せて `crates/areka-seriko/Cargo.toml:25-27` の存在理由コメント（「sink send 失敗時の発火を捕捉する専用」）も陳腐化している。**進行中 spec でこれを受けられる範囲を持つものは 0 件なので、新規に起票する。** 実害は「テストビルドが要らない依存を引く」ことに限られ、赤にはならない。

**C-2【起票】steering の crate 一覧に他 12 crate が欠けている（本タスクの範囲外・steering の同期手続きで解消する）**

本タスクは指示どおり `log-capture-kit` と `temp-path-kit` の 2 本を足したが、**`crates/` の実在 24 crate に対し `structure.md` が節を持つのは 12 crate だけ**である（追加後の実測）。節が無いのは次の 12 本: `areka-actor`・`areka-emo-atlas`・`areka-emo-compose`・`areka-emo-present`・`areka-emo-text`・`areka-ghost`・`areka-kanade`・`areka-sakura`・`areka-seriko`・`areka-talk`・`shiori-host32-testdll`・`shiori4-testdll`。**この 12 本は本仕様が作ったものではなく、本仕様の着手前から欠けていた**（本仕様が新設したのは 2 本だけで、その 2 本は本タスクで載せた）。解消先は spec ではなく steering の同期手続き（`/kiro-steering`）である。本タスクの完了条件「一覧と実際の `crates/` の内容が食い違わない」は**指示された 2 本については満たしたが、一覧全体としては満たしていない**——この事実を隠さず登記する。

#### ⑶-D 残余の登記（調べたうえで是正しないと裁定した項目・いずれも起票しない）

いずれも「担当先未定」ではなく、**本仕様での裁定の記録**である。根拠を付さずに残した項目は 1 つも無い。

**D-1 行整形の形の選択がどのテストにも縛られていない（タスク 3.6 の残余・3.7・3.8 で同型を確認）**

行整形の**どの形を選ぶか**（宛先つきの形か、宛先なしの形か）は**ワークスペース全域でどのテストにも縛られていない**。実測: 起動系 6 サイトを宛先なしの形へ変異させても areka は `1234 passed / 0 failed` で緑のまま（assert が宛先欄を 1 箇所も見ていない）、areka 側の逆向きの変異でも同じく緑、wintf でも `22/22` 緑。**共有 crate の逐語 fixture は「各形が何を出すか」は縛るが「呼出側がどの形を選ぶか」は構造上縛れない。** 正しさは ⑴ design.md が当該ファイル群へ形を明示していること ⑵ 移行前の整形が逐語で同じ書式文字列だったことからの導出で担保している。**起票しない根拠**: 迂回検知（走査語は直接呼出のみ）・行数の番人・反復検証のいずれも引受先にならず、破れても影響はテストが組み立てるログ行の見た目に限られ本番挙動には及ばない。

**D-2 窓の内側の有効判定の作り直しは証明可能に冗長（タスク 2.7 の決着）**

これは「引受先未定」ではなく**意図的な二重防御**である。実測: ⑴ 窓の内側の作り直しだけを外しても窓の前の焼き付きの場面は緑（捕捉先の差し替え自身が全発行点を再計算するため）、⑵ 常駐と作り直しの両方を外しても窓前の場面は緑、⑶ その状態で窓**内**の場面は赤。つまり**窓前の焼き付きは作り直しの有無に関わらず治り、窓内の焼き付きは作り直しでは治せない**——**正直な檻は作れない。** 要件 3.2 の機序そのものはタスク 2.7 の子プロセス較正が縛っている。同型で、全スレッド捕捉の設置内にある 2 つの保険も両方外しても緑になる（設置自身が全発行点を再計算するため）。

**D-3 `areka-ghost` の説明ラベルの復旧経路がどのテストにも縛られていない（タスク 3.2 の残余）**

`crates/areka-ghost/src/test_log_capture.rs` のラベル復旧経路は、現行の ghost 側が全件文字列リテラルのため外しても `107/107` 緑になる。移行前の取り出し器も同じ保険を持っていたので保険は残した。規則そのものは共有 crate 側の自己テストが縛っている。**ghost へ新しい檻を足すとテスト本数の同一性（要件 6.1）が崩れるので、台帳に書くのが正しい引受先である。**

**D-4 見張りの内部に縛られていない分岐が 3 件ある（タスク 6.2 の残余）**

⑴ `crates/log-capture-kit/tests/with_default_guard_test.rs:293-295`（判定の実体は `:294`）のアンダースコア表記の分岐は縛られていないが、唯一の現実的な迂回形は本体行の分岐で拾えることを実測済みで、Cargo はハイフン package に対する裸のアンダースコア鍵を拒否する＝**隙間ではなく冗長**。⑵ `:357-359` の早期 return も同様に縛られていない（今日は挙動が変わらない）。⑶ **除外ディレクトリ表が `:339` で重複している**（走査部品側の写しが private なため）＝**2 つの除外表が黙って乖離し得る**。⑶ だけは将来の実害の芽だが、統合するには走査部品の可視性を広げる編集が要り、それ自体が見張りの独立性を落とす。**乖離が実際に観測されたら【起票】へ格上げする**（現時点で観測 0 件）。

**D-5 例外表に載ったファイルが太っても誰も捕まえない（タスク 6.3 の残余）**

1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs:61` の `OVER_LIMIT_ALLOWED`・**11 件**）は「今そこにある超過」だけを表し、**載っているファイルがさらに太っても赤にならない**。ラチェット（例外ファイルは増やせない）を入れると**要件 10.4 で「触らない」と決めたファイルへの編集圧力**になるので、入れないのが 10.4 と整合する。同じ理由で例外表に行数は持たせていない（無関係な編集のたびに表が陳腐化し、触らないと決めたファイルを触らせることになる）。**ラチェットを望むなら別 spec の起票が要るが、要件 10.4 が生きている間は望まないという裁定である。**

**D-6 一時パスの見張りと要件 12.2 の絞り込みが持つ同型の穴（タスク 10.7・10.5 の決着）**

要件 12.2 の対象選定は**ファイル単位**の絞り込みだったため、**1 つのファイルの中で一意名と固定名が混在する形**を丸ごと取りこぼした（対象 20 → 21・`crates/areka-sylphya/src/persist/io.rs`）。**同じ穴を要件 12.4 の見張りの例外表も持つ**——既に表に載っているファイルの中に新しい固定名を足しても赤にならない（レビュアが実測で確認）。行単位へ強めると理由欄が行番号を抱えて陳腐化するので、**限界を見張りのコード内へ逐語で明記する**ことで折り合いを付けた（`crates/log-capture-kit/tests/temp_path_guard_test.rs:448-476`。`:454` は「この穴は机上の話ではない」と要件 12.2 自身が踏んだ実例を挙げている）。**「未達が spec の内側から見えない」を再演しないための措置であり、限界は成果物の内側から読める。**

**D-7 除外集合の走査に残る残余リスク 1 件（タスク 11.2）**

11.2 の A/B で除外集合を組む走査器（`verification/capture-window-tests.py`）は、**静的な較正を全部通り抜ける穴を 2 度出した**——1 度目は別名の取り込み（4 ファイル）で実走 3 回目に初めて赤、2 度目は自己テストを持つ包み 2 件とその呼び手 5 ファイル・96 件で**実走でも赤にならず**レビューの静的な突合で見つかった。再レビューは 3 通りの独立な手立てで 3 件目を探して見つけていない。**残余リスクは 1 件**: 供給条件はなお「ファイルに共有 crate の字面がある」ことを要求する（`capture-window-tests.py:376`）ので、**字面の無い包みが再利用された瞬間に取りこぼし、較正は緑のままになる**。その形は今日も実在する——`crates/areka/src/placement/placement_windowposition_vocab_tests.rs:48` の `fn resolve` はテスト関数の外で定義され本体が捕捉窓を開くが、同ファイルに共有 crate の字面は 0 件である（実測）。**現在は呼び手が同ファイル内にしか無いので無害**（そのファイル自体は別の語で拾われて除外されている）。

**D-8 表示更新の既知の残余 ⒝ に 1 行添える（タスク 5.2 の補足）**

要件 5.9 ⒝（外形変更経路で寸法変更が成功した後に後段で失敗し、内部状態は旧値・表示バッファだけ新寸）の後に**同じ寸法の更新**が来ると、寸法一致で寸法変更の経路を通らず複製が寸法不一致で黙って no-op になり、**表示は凍ったまま読み戻しだけ正しく見える**。回復の検査は毎回異なる寸法を使うため必ず寸法変更の経路を通り、この形を踏まない。散文「次回の更新が回復する」は機序の記述より広い。**実デバイスでは実質起こらない経路として記録のみとする裁定は変えない。**

**D-9 表示側の観測が届いていない効果が 1 つある（タスク 5.3 の残余）**

配置の指示のもう一方の効果＝`crates/areka-emo-present/src/mount.rs:259-266` の視覚要素への寸法設定は、失敗注入のテストでは観測していない。`mount.rs:241` が配置情報を権威と明記し、当該設定の失敗は警告のみの最善努力なので**是正はせず記録のみ（起票不要）**。

**D-10 要件 12.7 の走査式の較正の記録**

対象の絞り込みに使う走査式は較正した。**根拠となった事故は再現できる**——`crates/areka/src/main_restore_seam_tests.rs:15` の「外部 tempfile 非依存」という**コメント中の語**が絞り込み式に拾われ、**実際に落ちている当のファイルが候補から外れた**。この行はタスク 10.2 の移行で消えたので、再現するには `git show 3004a27e:crates/areka/src/main_restore_seam_tests.rs` の `:15` を見ること（`3004a27e` はタスク 10.2 のコミット `c0506e22` の親。2026-08-27 に実際に取り出して逐語で確認した）。**素の走査は拾い、コメント除去後は拾わない、が逐語で再現する。** 較正の部品はタスク 6.1 のコメント除去（同型の罠を既に解いている）を用いた。11.2 の走査器も較正指定で「当たりが 0 件でないこと」「既知のファイルが**拾われる**こと」を陽性側から要求し、1 件でも欠ければ非 0 で止まる形にしてある——**「拾われないこと」を確かめる形は、走査が丸ごと空振りしていても緑になる。**

**D-11 要件 9.5（`cargo test -p areka` の正体不明の 553/1 赤）の決着**

**別系統として残る、が結論である。** ①の硬化後にも `cargo test -p areka` は赤を出したが（同時 4 プロセス 30 回中 3 回）、その原因は**捕捉でも待機でも退役した錠でもなく一時パスの共有**で、A-3 のとおり群 10 が消した。**2026-08-05 の 553/1 との同一性は主張しない**——当時テスト名が採られておらずログも残っていないため同定不能である。ログの無い赤を「同じものだった」と読むことこそ本仕様が退治している形なので、**同定不能のまま記録する。**

**D-12 要件 10.5（合流後は新規の 1,000 行超が赤になる）の申し送り先は既に消えている**

要件 10.5 は「並走中の `draw-load-parity` へ申し送る」と定めるが、**同 spec は 2026-08-23 に完了・アーカイブ済み**（`.kiro/specs/completed/areka-P0-draw-load-parity/`）で、しかも**本仕様の番人が置かれる前に `main` へ合流している**（分岐点 `327e7fd3` が同 spec の squash）。したがって「合流後に新規の 1,000 行超を作る」機会そのものが同 spec には無く、**申し送りの前提が消えている。** 実効的な宛先は「今後 `main` へ合流するすべての spec」であり、それは**人間の消化を要さず番人が自動で果たす**——`crates/log-capture-kit/tests/file_length_guard_test.rs` が例外表 11 件（着手時点の超過）の外に 1,000 行超のファイルを見つけたら赤になる。**よって本項目は【実施】であり、申し送り先を新たに探す必要は無い。** 例外表の項目は削除（分割による解消）のみが自然な方向で、追加は明示的な編集としてのみ許される。

#### ⑶-E 本仕様が繰り返し踏んだ形（後続 spec への一般的な申し送り）

**E-1 「全緑」は十分性の証拠にならない。** 本仕様で 2 度、別の形で踏んだ——⑴ 常駐なし側の赤の集合は非決定（8 回の走行で不変に赤なのは較正 1 本だけ・他は 1/8〜5/8 で出入り・5 本除外すると 6 本目が出る。独立の 3 回では 0 件 / 2 件 / 0 件）なので、**走らせて緑だったことからは何も言えない**。⑵ 除外集合の 96 件は 24 回全緑のさなかに両側で走り続けていた。**設計 `#### C9` の前提「無効側で赤になる集合は決定論的で特定可能」は偽だった**——無効側の赤は毒化の競合が起こすもので、それは本仕様が消そうとしている病そのものであり、**並列の巡り合わせに依存するのが病の定義**である。「病の症状を観測して除外する」という手順は病の性質と噛み合っていなかった。**成立している側は分けて記録する**: 要件 13.2 の前段——**取りこぼした窓が黙って空を返さず失敗を宣告すること**——は実測で成立している。

**E-2 起草の `file:line` は実物とずれる。** 本仕様は**8 度**踏んだ（最大は錠の削除範囲が 3 文書で三者三様に誤っていた件。直近の 1 件は本タスクが見つけた——`tasks.md` の B-2 の記述が `while` を `:145`・数え上げを `:173-179`・判定を `:180-184` と書いているが、タスク 10.3 の移行で当該ファイルが +1 行ずれ、現在の実測は `:146`／`:175-180`／`:181-185` である）。**8 度目はこの節自身が踏んだ。**レビューが実測し、D-4 の 3 件が一様に約 5 行ずれていた（`:288-290`→`:293-295`・`:352-354`→`:357-359`・`:334`→`:339`）。A-2 の引く `tasks.md:331` も実体は `:441` で、B-1 のコメント範囲も `:175-179` → `:176-179` だった。**「全件を実測し直してある」と書いた当の節の隣で、5 件がずれていた**——陳腐化を戒める節が陳腐化するのはこれで 2 度目である（前は `dpi-transition-atomicity`）。**上記 5 件は 2026-08-27 のレビュー後に訂正済み。本節を読む者も、参照する前にもう一度当てること。**

**E-3 道具そのものが黙って嘘をつく形を 12 種記録した**（`tasks.md` の Implementation Notes が正本）。とくに次の 4 つは本台帳の登記作業でも当てた: ⑴ **`cat -A` と `sed` はこの環境で CR を黙って落とす**（改行の検査はバイト単位でのみ）、⑵ **末尾に改行の無いファイルへ追記すると 1 行目が既存の最終行と黙って融合する**（追記の前に末尾バイトを確かめる）、⑶ **終了コード 0 は空振りでも返る**（「0 件」には赤を出せることの較正を対で置く）、⑷ **bash の二重引用符の中でもバッククォートはコマンド置換される**（逐語を含む本文はインラインに埋めず、ファイルへ書いてから読み込む）。

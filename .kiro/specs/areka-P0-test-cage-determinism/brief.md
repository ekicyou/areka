# Brief: areka-P0-test-cage-determinism

> **Discovery 日**: 2026-07-30 ／ **ウェーブ**: **W6.5**（W6 の後・W7 の前・追記(51)） ／ **規模**: medium-large（ただし**本番ソース面はほぼゼロ**）
> **📌 2026-07-31 追記(52)微補正**: ①44 呼出・スピン 13 箇所・`Instant` 0 件は 2026-07-31 再実測で**全数一致**（アンカー健在）。ただし「Tick 兼用 `for now in …` 形は 5 箇所」は実測 **6 箇所の可能性**（:807,:1288,:1704,:1782,:1883,:2108・:1288 は `1_000_000..+5_000` の変則形）＝着手時に再計数。②**W5 `choice-select-events` が `input_events/balloon.rs` に `ChoiceSelectionInbox` drain を増設予定**（同ファイルは本 spec の毒化18呼出対象）＝W5 先着・本 spec 後着 rebase。③W6 は `balloon-visibility ∥ bindoption-exclusivity` の2本へ改訂（frame.rs 衝突相手に変化なし・bindoption は seriko/parsers 面ゆえ本 spec と素）。
> **📌 2026-08-01 追記(58)数値全面更新（棚卸⑤・W5 3本マージ後の実測・本ブロックが(52)微補正と以下の本文の数値より優先）**:
> - **② spine.rs スピン: 13 → 残 2 箇所**。`Instant`/壁時計 deadline なしの反復固定スピンは (i) **:1395-1398** `for now in 1_000_000u64..1_000_000+5_000`（Tick 兼用変則形）・(ii) **:2273-2275** `for _ in 0..5_000`（settle drain 内・負検証・doc :22 で意図的温存の明文あり）のみ。**ker が 11 箇所を `SPIN_WAIT` 壁時計 deadline＋200µs poll-backoff へ先行消化済み**（:328/:908/:1540/:1647/:1876/:1963/:1993/:2089/:2322 等）＝残作業は検収＋温存 2 箇所の裁定のみ。
> - **① tracing 毒化: 44 呼出/6 モジュール → 45 呼出/7 モジュール**。既存 6 箇所は全数不変、**新規: `input_events/choice_drain.rs` の `capture_logs`（:182/:186・se 由来）**。「最小複製」自認コメントが choice_drain.rs:161 へさらに伝播＝**「否認が残ると再発する」の実証・cage を後ろへ置くほどコピーが増える構造も実証**（W5 で 1 増）。
> - **se rebase 前提は消化済み**: drain は balloon.rs でなく新ファイル `choice_drain.rs` へ着地（balloon.rs は +12 行のみ）＝rebase 負担ほぼゼロ。
> - **③ の新事実**: van が frame.rs:4088 で正典ハーネス（`placement/test_support.rs`）を消費開始＝**frame.rs 内に probe 方式と局所 `capture_logs` が同居**——一本化裁定の材料が増えた。frame.rs の helper は :1990 へドリフト。
> - **規模: medium-large → medium へ縮小**。配置＝**W6.9**（追記(58)裁定: vis 先着必達〔frame.rs テスト域〕＋presenter.rs `apply_show` 鎖〔col→exact→budget→atom④〕の最後尾。④を `#[cfg(test)]` fault フラグ小案に縮めれば presenter.rs 衝突は :510 の 1 呼出に縮退）。553/1 単発赤の監視は継続。
>
> **📌 2026-08-05 追記(59)＝① 射程の全面拡大（`areka-P0-collision-dpi-hittest` Task 8.1 実測由来・本ブロックが(58)以前の①の数値に優先）**:
> - **① tracing 毒化: 45 呼出/7 モジュール → 95 呼出/12 モジュール**。(58) までの表は `crates/areka/src/**` だけを見ていたが、**射程は Desired Outcome(:70-71)・Boundary Candidate ③(:91) の宣言どおり全 crate 横断**であり、`crates/areka` 外に**未登記の未硬化サイトが 5 モジュール・50 呼出**あった（内訳は下表 7〜11・全件 2026-08-05 に `rebuild_interest_cache()` 不在を file:line で実測）。
> - **発見経路は本番実測**: `cargo test --workspace` が**約 1/6 で `areka-seriko` のログ捕捉檻で赤**になる（単独 30 走 0 失敗・CPU 負荷下 60 走 1 失敗＝負荷依存）。特定 assert ではなく**ハーネス全体が確率的に捕捉 0 件**を返す。観測された落ち方は `actor::tests::non_shell_broadcast_reception_is_benign_debug_no_warn_error`（assert は actor.rs:1258・`level=DEBUG` の**存在**主張）と `actor::tests::bind_apply_on_shown_emits_show_and_info_marker`（assert は actor.rs:1470・`level=INFO` の**存在**主張）。
> - **①の「目印」である偽の否認コメントも同伴していた**: `actor.rs:1946-1947`「スレッドローカル `with_default` ゆえ並行テスト安全」は :34 が marker として挙げる記述と**逐語一致**。同型の否認は table.rs:206・emo-atlas/emo-compose の `log_capture.rs`(:14-16 モジュール doc ＋ :55 関数 doc) にも伝播している。
> - **出自の取り違えを実測で確認**: seriko の否認コメントは自らの流儀を「emo-compose/kanade の log_capture 流儀」と名乗るが、**kanade は硬化済み・compose は未硬化**。誤った由来表示のまま複製されている＝(58) の「否認が残ると再発する」の 2 例目。
> - **リスク記述の片側漏れを是正**（本文 :21 に反映）: 従来「イベント 0 件を静かに観測して**緑になる**（偽陰性）」だけを書いていたが、**ログの存在を assert する檻では同じ毒化が偽陽性＝赤として現れる**。seriko の 2 件がまさにそれ。
> - **規模への影響**: 呼出数は倍増するが作業は依然として機械的（③の裁定が決まれば差し替えのみ）。ただし**crate 境界を跨ぐ**ため、③の「共有化の実現方法」（:117 の Constraints）は `crates/areka` 内で閉じない形が**必須条件**になった。配置（W6.9）は変更なし。
> - **①の未登記候補の掘り出しは今回で完了とはみなさない**: 判定は「`rebuild_interest_cache()` を呼んでいるか」の 1 点で機械判定できるため、**着手時に workspace 全体を再走査**すること（下表は 2026-08-05 時点のスナップショット）。
>
> **出自**: `completed/areka-P0-emo-dpi-scaling` の `/kiro-validate-impl` ゲートが「無名の別タスク宛て＝実質未所有」として記録した 4 件（tasks.md:222）。
> 2026-07-30 に**全件の実在を再検証**したところ 4 件とも健在で、うち 1 件は記録より**悪化**していた。担当 spec は 10 本の active spec のいずれにも存在しない。

> **📌 2026-08-21 追記(72)（`areka-P0-dpi-transition-atomicity` からの申し送り・観測チャネルが 1 本増え、窓書込の檻の前提が変わった）**: **既定 OFF の観測チャネル `wintf::transition` が新設され、窓書込指令に要求元の札が付き、同一窓のジオメトリ指令が合流するようになった。** ① tracing 毒化の射程・②「零件の主張」の扱い・④ `apply_show` 鎖の 3 点それぞれに効く。
> - **⑴ 新しい観測 target が 1 本**: `wintf::transition`（`crates/wintf/src/ecs/window/transition_diag.rs:54`）。既定水準では 1 行も出ず、前置ガード `transition_diag::is_enabled()`（同 :595＝`tracing::enabled!` の薄い包み）が偽なら**行の組立も時刻の読み取りも一切行わない**。**「既定で無音」を主張する檻は、同じ捕捉窓の内側に必ず出るはずの対照行を置く形にしてある**（本 spec で「既定で無音」が恒真だった事故が実際に起きたため）。①の毒化表へこの target を足すかどうかは本 spec の裁定だが、**濾過テストはスレッド局所 subscriber（`crate::ecs::test_support::capture_under_filter`＝`crates/wintf/src/ecs/test_support.rs:96`）で書く**規律に従っている。
> - **⑵ 語彙の不変条件が 2 つ増えた（檻が固定している）**: ⒜ 窓種別のフィールド名は **`win_kind=`**（`transition_diag.rs:167`）であって `kind=`（:143＝レコード種別）ではない。⒝ **1 行に同じフィールド名を 2 度出さない**——`tools/perf/judge-perf.py::parse_fields` が同名キーを後勝ちで上書きするため、重複するとレコード種別が消えて判定器が壊れる。⒜⒝ とも `crates/wintf/src/ecs/window/transition_diag_tests.rs`（`no_line_repeats_a_field_name` :362）が固定している。**本 spec が観測行を足す・共有ヘルパへ寄せるときも同じ規律を保つこと。**
> - **⑶ 窓書込の回数を数える檻は前提が変わった**: `SetWindowPosCommand::enqueue`（`crates/wintf/src/ecs/window/command.rs:372`）が積む前に同一 hwnd の畳める指令へ後勝ちで合流する（`is_coalescible` :229／`find_merge_target` :242／`merge_into` :263／純関数 `coalesce_geometry` :303）。**「窓書込が N 回出る」を数える既存の檻は、合流後の回数を見ている**——遷移 1 回で 4 窓に対し合流前 8 本・合流後 4 本。Z 専用指令（挿入位置を持つ）は合流対象外で、**畳めない指令は同一窓の仕切りとして働く**。
> - **⑷ 決定論テストは flush を通らない**（D11）: 本 spec の多フレーム駆動ハーネスは `SetWindowPosCommand::drain_window_pos_commands()`（`command.rs:528`）でキューを直接取り出して数える。実 `SetWindowPos` を撃つ一括 flush（`flush_window_pos_commands` :509）は通らないので、**`kind=write` 行（flush 由来）を決定論テストで数える形にすると如何なる退行でも赤にならない**。数えるならキュー側で数え、固定する値は判定器の `pub const`（`crates/areka/src/placement/transition_judge_verdict.rs`）を引くこと——この落とし穴は本 spec の `crates/areka/src/emo2_boot/frame_transition_atomicity_tests.rs` の module doc に機序ごと書いてある。
> - **⑸ ④ `apply_show` 鎖への影響（cage④ の観測点は動かしていない）**: `crates/areka-emo-present/src/presenter/show.rs` の upload 失敗の早期 return（:306-310）は本 spec が**意図的に動かしていない**（cage④ の観測点だから）。観測が入ったのは upload の後（:347-359）と可視化の後（:389-399）で、どちらも `size_changed || resized` かつ観測有効のときだけ組む共有の札 `observe_surface`（:347）で守られている——`recompose-budget` が成立させた**定常状態のアロケーション 0**（要件 10.4）を壊さないため。**この前置ガードは出力では示せないので、`crates/areka-emo-present/src/presenter/transition_record_tests.rs` の本文走査だけが退行を捕まえる**。cage④ が同ファイルを触るときはこの走査を残すこと。
> - **⑹ perf 行が 1 列増えた**: `perf(apply_show)` に `frame=` が**末尾**追加（`crates/areka-emo-present/src/presenter/timing.rs:220`）。完全一致で照合する檻は列数を改める必要がある（本 spec は `crates/areka-emo-present/src/presenter_perf_log_tests.rs` の `PERF_LINE_FIELDS` を 15→16 にした）。
> - **⑺ 本 spec が使える資産（作り直さないこと）**: 多フレーム駆動ハーネス `FrameHarness`（`crates/areka/src/emo2_boot/frame_test_support.rs`）は、World 資源と写しを同一点で進める `advance_frame`・3 つの源の差替口・`drain_writes`・`single_threaded_schedule`（要件 7.6）・x64 限定の `const _` assert（要件 7.5）・`transition_diag::reset_for_test`（要件 7.7 の状態非持越）を持ち、**ハーネスそのものの檻**（`crates/areka/src/emo2_boot/frame_harness_tests.rs`）が残留の非持越と同一プロセス連続 2 シナリオの判定不変を固定している。**「共有化の形」を設計するときの実例として読めるはず。**
> - **⑻ 正本**: `.kiro/specs/areka-P0-dpi-transition-atomicity/`（要件 7.6／7.7／10.4・design「Testing Strategy」・`mechanism-ledger.md` が file:line の正本）。

## Problem

**檻（テスト）が嘘をつき得る状態が 4 系統残っている。** いずれも本番挙動のバグではないが、「緑である」という信号の信頼性を損なう——本プロジェクトは [[deterministic-test-coverage-mandate]] を掲げており、檻の決定性そのものが成果物である。

### ① tracing callsite 毒化ハザード（**12 モジュール・95 呼出**・追記(59)）

`tracing` の callsite interest cache は**プロセス全体で共有され first-thread-wins**。`with_default` はスレッドローカルだが interest は違うため、先に別スレッドが「このログは不要」と判定を焼き付けると、**後続の捕捉テストはイベントを 1 件も観測できなくなる**。硬化済み正典は `crates/areka/src/placement/test_support.rs`（常駐 probe dispatcher 2 個＋捕捉窓の内側で `rebuild_interest_cache()`）。

**症状は檻の書き方によって両側に出る**（追記(59) で是正・従来は前者しか書いていなかった）:

- **偽陰性＝緑**: 「このログは出ない」を主張する檻（`assert!(!logs.contains(…))`）は、毒化で捕捉 0 件になっても**静かに通る**。何も検証していないのに緑。
- **偽陽性＝赤**: 「このログが出る」を主張する檻（`assert!(logs.contains("level=INFO …"))`）は、毒化で捕捉 0 件になると**落ちる**。本番は正しいのにテストだけが確率的に赤くなる——`areka-seriko` の 2 テストで実測された落ち方がこれ（追記(59)・約 1/6・負荷依存）。

どちらも「檻が嘘をついている」点は同じで、**赤の側は少なくとも気づけるぶん幸運**。緑の側は気づく手段が無い。

未硬化サイト（`registry().with(cap)` ＋素の `with_default`・probe 無し・**`rebuild_interest_cache()` 無し**）。7〜11 は 2026-08-05 追記(59) の追加分:

| # | ファイル（ヘルパ定義） | 呼出数 |
|---|---|---|
| 1 | `crates/areka/src/emo2_boot/adapter.rs`（`capture_logs`） | 2 |
| 2 | `crates/areka/src/emo2_boot/frame.rs`（`capture_logs`） | 8 |
| 3 | `crates/areka/src/emo2_boot/move_cue.rs`（`capture_logs`） | 5 |
| 4 | `crates/areka/src/emo2_boot/spine.rs`（`capture_logs`） | 8 |
| 5 | `crates/areka/src/input_events/balloon.rs`（`capture_logs`） | 18 |
| 6 | `crates/areka/src/shiori_demo.rs`（ヘルパ無し・**inline** `with_default` × 3） | 3 |
| 7 | **`crates/areka-seriko/src/actor.rs`**（`capture_logs` **:1948**／`registry().with(cap)` :1978／素の `with_default` :1979。派生ヘルパ `capture_logs_flow` :1937 が :1940 で委譲＝**同一捕捉層**） | **19**（テスト直呼 18 ＋ flow 内部委譲 1） |
| 8 | `crates/areka-seriko/src/table.rs`（`capture_logs` :208／`registry().with(cap)` :238／`with_default` :239） | 2（:339, :401） |
| 9 | `crates/areka-emo-atlas/src/log_capture.rs`（`pub(crate) fn capture_logs` :59／:62-63） | 6（`lib.rs` :470/:506/:525 ＋自己テスト :75/:89/:101） |
| 10 | `crates/areka-emo-compose/src/log_capture.rs`（`pub(crate) fn capture_logs` :59／:62-63） | 20（`log_firing_tests.rs` 11・`scale.rs` 4・`plan.rs` 2・自己テスト 3） |
| 11 | `crates/wintf/src/ecs/window_proc/dpi_helpers.rs`（`mod tests` 内 `capture_logs` :479／:483） | 3（:506, :538, :559） |

**さらに悪いことに、これらは「スレッドローカル `with_default` ゆえ並行実行でも干渉しない」という誤ったコメントを掲げている**（dispatcher については真だが interest cache については偽）。ハザードが積極的に否認されている状態。追記(59) で確認された否認の所在: `actor.rs:1946-1947`（**:34 の marker と逐語一致**）・`table.rs:206`・`areka-emo-atlas/src/log_capture.rs:14-16`（モジュール doc）と `:55`・`areka-emo-compose/src/log_capture.rs:14-16` と `:55`。`dpi_helpers.rs:477-478` は「`set_global_default` は使わない」とだけ書いており否認は無いが、rebuild も無い。

**2026-08-05 時点で硬化済み（`rebuild_interest_cache()` 有り）と実測確認できたのは**: `crates/areka/src/placement/{test_support.rs, diag.rs, follow.rs}`・`crates/areka-emo-present/src/{balloon.rs, scale.rs}`・`crates/areka-ghost/src/test_log_capture.rs`・`crates/areka-ghost/tests/ghost/spine_e2e_test.rs`・`crates/areka-kanade/src/schedule/log_capture.rs`・`crates/areka-sylphya/src/test_log_capture.rs`・`crates/wintf/src/ecs/test_support.rs`・`crates/areka-emo-text/tests/attach_wiring_test.rs`。**硬化済み／未硬化の判定は `rebuild_interest_cache()` の有無 1 点で機械判定できる**ため、着手時は workspace 全体を `rg rebuild_interest_cache` と捕捉ヘルパ定義の突合で再走査すること。

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
- 捕捉テストが毒化で「イベント 0 件」を掴まされ得ない——**「静かに緑」（不在主張の檻）も「確率的に赤」（存在主張の檻）も、どちらも起こらない**（追記(59)）。
- `spine.rs` の待機が全て `Instant` 基準の有界スピンになり、負荷下でも偽赤しない。
- `chain.upload` 失敗時に「表示は前状態を保つ」が**実行テストで証明**される。

## Approach

**檻の決定性を 1 spec で通しで直す。** 4 件はどれも「テストが本当のことを言っているか」という同一の関心であり、①③は同じファイル群を触るため分けると二重作業になる。

1. **③ を先に決める**（設計判断が①の実装形を決めるため）: probe 方式と global-keeper 方式のどちらを正典とするか裁定し、**共有 crate（dev-dependency）または `pub` テスト支援モジュール**へ 1 本化。
2. **① を機械適用**: **11 モジュール**のヘルパを削除し共有版へ差し替え（**95 呼出**の import 書き換え・追記(59)）。`shiori_demo.rs` の inline 3 箇所は先にヘルパ化。`capture_logs_flow`（`actor.rs:1937`）は捕捉層を `capture_logs` へ委譲しているだけなので、**基底 1 本を差し替えれば派生も同時に硬化する**。**誤ったコメントも全て是正**する（否認が残ると再発する——追記(59) で、否認が誤った由来表示〔「kanade 流儀」を名乗るが kanade は硬化済み〕ごと複製されていることが判明）。
3. **② を変換**: `areka-ghost` の `spin_pumping_ticks` を donor に `spin_until(what, deadline, done)` を導入し 13 箇所を変換。Tick 生成子を兼ねる 5 箇所は Tick カウンタと deadline を分離。
4. **④ にシーム**: `#[cfg(test)]` の fault フラグ（小）か `trait SurfaceUpload` ＋ fake（大・`presenter.rs`/`mount.rs` へ波及）を裁定し、失敗経路の「前状態保持」を檻化。

## Scope

- **In**: 上記 4 件。誤ったコメントの是正。硬化設計の一本化。
- **Out**: 本番の挙動変更（④のシーム以外）。テストの**内容**の変更（既存の判定は保存する——直すのは観測機構と待機機構だけ）。`areka-ghost` 側（2026-07-30 に是正済み）。

## Boundary Candidates

- **硬化設計の裁定と共有化**（③ — 全 crate 横断・最初に片づける）
- **捕捉サイトの追随**（① — **11 モジュール／5 crate 横断**〔`areka` 6・`areka-seriko` 2・`areka-emo-atlas` 1・`areka-emo-compose` 1・`wintf` 1〕・機械的・追記(59)）
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
  - **`areka-P0-bindoption-exclusivity`（W6）— 追記(59) で「素」判定が覆った**。追記(52) は「bindoption は seriko/parsers 面ゆえ本 spec と素」と書いたが、①に `crates/areka-seriko/src/actor.rs` が加わったため**同一ファイル**になる（bind 側の割当は同 brief :41/:205 の `actor.rs:367` 分岐＋`:1546` 檻）。**本 spec は W6.9＝W6 完了後**ゆえ順序は既に安全だが、bind が actor.rs の bind 檻を増改築すると①の呼出数（19）は動く＝**着手時に再計数が要る**。なお追記(59) で観測された確率赤の 1 本は `bind_apply_on_shown_emits_show_and_info_marker`（actor.rs:1470）＝bind 檻そのもの。

## Constraints

- **新規外部依存なし**（`tracing-subscriber` は既出）。
- 共有化の実現方法は要設計: `src` の `mod tests` 内・統合テスト・別 crate と**配置がバラバラ**なため、単なる移動では済まない（dev-dependency 用の支援 crate か `#[cfg(feature = "test-support")]` 公開かの判断）。**追記(59) で必須条件が 1 段強まった**——消費側が `areka`・`areka-seriko`・`areka-emo-atlas`・`areka-emo-compose`・`wintf`・`areka-ghost`・`areka-kanade`・`areka-sylphya`・`areka-emo-present`・`areka-emo-text` に跨るため、**`crates/areka` 内で閉じる形は最初から不可**。とくに **`wintf` は `Cargo.toml` 上 `areka-*` crate へ一切依存していない**（実測・ワークスペース内依存は `dola`・`wintf-winmsg-executor` のみ）ため、共有先を既存のどの `areka-*` crate に置いても `wintf` からは引けない。**ワークスペース内依存を持たない新規 leaf crate を dev-dependency として全消費者に配る形**が最有力（③の裁定で確定させること）。
- [[areka-bin-crate-internal-tests-in-crate]]: `crates/areka` は bin crate ゆえ内部到達テストは in-crate 配置が必須（`tests/` はバイナリ起動型専用）。共有化の形はこの制約を満たすこと。
- 検証は**反復実行**で行う（フレーキーは単発の緑では証明できない）。②は負荷下・並列で最低数十走。

---

**2026-08-01 追補（kero-balloon task 7.1/7.2 の先行消化・roadmap 追記(56)）**: 本 brief の②「spine.rs 協調スピン 13 箇所」は **kero-balloon が 11 箇所を先行是正済み**（R7.8＝S2 注入時刻の観測窓頭打ち〔Clear@1.40s・導出式は同 spec requirements R7.8〕・R7.9＝壁時計 `Instant` deadline 10s＋200µs poll-backoff sleep×11 本）。残作業＝(a) 是正形の検収（台本・アサート無改変の確認）、(b) 意図的に `yield_now` のまま温存した **settle drain 2 箇所**（負検証「尽きるのが正常」）の扱い裁定、(c) 着手時に spine.rs を実測して取りこぼし確認。**②の規模見積りは縮む**。
**監視項目の引き継ぎ**: kero-balloon 検証中に `cargo test -p areka` が **1 回だけ 553/1 で赤**（13 秒・S2 空回りパターンではない・**ログ未保存でテスト名不明**）。以後 15 回以上連続緑で再現せず。本 spec の反復検証（②は負荷下で数十走）中に赤を見たら**必ずログを tee してテスト名を採る**こと——これが正体不明のまま残っている唯一の非決定性候補。
**2026-08-05 追記(59)**: 別途 `cargo test --workspace` が**約 1/6 で `areka-seriko` のログ捕捉檻で赤**になることが実測された（テスト名まで特定済み・①へ登記）。これは**上記 553/1（`cargo test -p areka`・13 秒・テスト名不明）とは別件**であり、553/1 の正体は依然不明のまま残る——ただし**同じ毒化機序で説明できる可能性がある**（`crates/areka` 側にも未硬化サイトが 6 モジュール 44 呼出ある）ため、①を硬化した後に 553/1 が再現しなくなるかを**反復検証で確認**すること。①硬化後も再現するなら別系統の非決定性が残っている証拠になる。

# Brief: areka-P0-draw-load-parity

> 起票 2026-08-15（`/kiro-discovery` 再入）。`recompose-budget`（W6.5）の完了直前に、
> 同 spec の実測が「削るべき対象は自分の境界の外にある」ことを示したため分離起票した。
> **優先度は低い**（開発者指示）。実行は別セッション・後日。

> **📌 2026-08-21 追記(71)（`areka-P0-dpi-transition-atomicity` からの申し送り・窓書込指令の形と一括 flush の中身が変わった／`ghost-window-zorder` の Z 維持系の受け先を兼ねる）**: **flush 経路は「積まれた指令を順に撃つだけ」ではなくなった。同一窓のジオメトリ指令が積む時点で合流し、指令には要求元の札が付き、専用の観測チャネルが増えた。** 本 spec は `wintf` の `runtime`／`ecs::world` を In-scope に持ち（Scope「フレーム駆動の周期と、変化が無いときの早期脱出」）、一括 flush はその内側（`crates/wintf/src/runtime/tick_bridge.rs:206` が `flush_window_pos_commands` を呼ぶ）なので、**適用順・適用回数を論じるときの前提が変わっている**。
> - **⑴ 指令に要求元の札が付いた**: `SetWindowPosCommand` に `pub tag: WriteTag`（`crates/wintf/src/ecs/window/command.rs:177`）が加わった。`WriteTag { origin, scope, kind }` の定義元は `crates/wintf/src/ecs/window/transition_diag.rs:284-296`（未設定は `WriteTag::UNTAGGED` :297）。付与は `.with_tag()`（`command.rs:359`）で、`SetWindowPosCommand::new` の**7 引数は不変**＝既存の呼び手はコンパイルも挙動も変わらない。
> - **⑵ 同一窓のジオメトリ指令が合流するようになった**: `SetWindowPosCommand::enqueue`（`command.rs:372`）は積む前に同一 hwnd の畳める先を探し（`find_merge_target` :242）、後勝ちで畳む（`merge_into` :263／純関数は `coalesce_geometry` :303）。**回数の見積もりが変わる**——遷移 1 回でキャラ・バルーン計 4 窓に対し、合流前は 8 本・合流後は 4 本（決定論テストの実測。`crates/areka/src/emo2_boot/frame_transition_atomicity_tests.rs`）。
> - **⑶ Z 専用指令は合流の対象外で、適用順も結果も変わらない**（要件 10.3）: 畳める条件は `is_coalescible`（`command.rs:229`）の 3 連言——⑴ 挿入位置を持たない、⑵ `REQUIRED_FOR_COALESCE`（`SWP_NOZORDER|SWP_NOACTIVATE`・:223）をすべて持つ、⑶ `COALESCIBLE_FLAGS`（:216）以外のフラグを 1 つも持たない。`ghost-window-zorder` の維持系が組む指令（`crates/wintf/src/ecs/window/zorder_pair_maintain.rs:188-216` の `pair_fix_command`・積み上げは同 :483）は挿入位置を持つので**必ず ⑴ で落ちる**。加えて**畳めない指令は同一窓の仕切りとして働く**ので、Z 指令をまたいで前後のジオメトリ指令が合流することもない。この不変条件は `crates/wintf/src/ecs/window/command_coalesce_tests.rs`（21 本）が固定しており、Z 専用／表示状態／挿入位置／活性化／Z 移動／別窓の各不合流をそれぞれ陽性の対つきで持つ。
> - **⑷ 受け先の判断（読み飛ばさないこと）**: 上流の `ghost-window-zorder` は `.kiro/specs/completed/` にあり**申し送りを消化できない**（プロジェクト規律「先送りは担当 spec の実在検証＋即報告」「completed は消化不能」）。`zorder_pair_maintain.rs` そのものを担当ファイル集合に持つ生存 spec は 2026-08-21 時点で存在しない。**「Z 指令の適用順と結果」が現に決まる場所は一括 flush であり、その flush は本 spec の In-scope（`wintf` の `runtime`）にある**ため、ここへ登記した。実機の見た目（バルーンがキャラの手前に居ること）の側は `areka-P0-emo2-conformance-e2e` の追記(73) が受けている。**本 spec が flush の駆動・間引き・順序に手を入れるなら、⑶ の 3 連言と `command_coalesce_tests.rs` を先に読むこと。**
> - **⑸ 観測チャネルが 1 本増えた（既定 OFF・計測の道具として使える）**: target `wintf::transition`（`transition_diag.rs:54`）が新設され、`kind=write`／`flush`／`enqueue`／`msg`／`monitor`（`KIND_ALL` :81）の各レコードを出す。`flush` レコードは `stage=begin|end` と `total_us` を持ち、`write` レコードは 1 本ごとの `call_us`（`SetWindowPos` 呼出だけの実時間）を持つ——**本 spec の「どのスケジュールにいくらかかるか」の内訳解析にそのまま使える**。既定水準では前置ガード（`transition_diag::is_enabled()`＝`tracing::enabled!`）で組立も計時も一切行わず、費用は 0 である（要件 10.6 の追従比 1.000 と定常の窓書込 0 を壊さないための設計。構造の檻は `crates/wintf/src/ecs/window_proc/window_pos_transition_tests.rs` の `message_handlers_keep_the_front_guard_so_the_default_run_pays_no_observation_cost`）。
> - **⑹ perf 行に `frame=` が末尾追加された**: `perf(apply_show)` の行（`crates/areka-emo-present/src/presenter/timing.rs:220`）に `frame`（tick のフレーム番号）が**末尾**で加わった。既存フィールドの順序・名前・文言は 1 つも動いていない。`tools/perf/judge-perf.py::parse_fields` は `名前=値` を辞書化して必須フィールドの存在だけを見るので互換だが、**本 spec の測定資産が perf 行を完全一致で照合しているなら列が 1 つ増えている**（`crates/areka-emo-present/src/presenter_perf_log_tests.rs` は `PERF_LINE_FIELDS` を 15→16 へ改めた）。これで perf 行と遷移観測行が同一フレームで突合できる。
> - **⑺ 語彙の不変条件**: 窓種別のフィールド名は **`win_kind=`**（`transition_diag.rs:167`）であって `kind=` ではない——`kind=` はレコード種別（:143）で、**1 行に同じフィールド名を 2 度出さない**のが不変条件（`judge-perf.py` の `parse_fields` が後勝ちで上書きするため）。この規律は `crates/wintf/src/ecs/window/transition_diag_tests.rs:362` の `no_line_repeats_a_field_name` が固定している。**本 spec が新しい計測行を足すときも同じ規律に従うこと。**
> - **⑻ 正本**: `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/`（要件 10.3／10.4／10.6・design「Components and Interfaces > C2 `command.rs` 合流＋flush 観測」・`mechanism-ledger.md` が file:line の正本）。

> **📌 2026-08-21 追記(74)（`areka-P0-dpi-transition-atomicity` からの申し送り・一括 flush を `DeferWindowPos` の 1 バッチへ移すことが確定した／追記(71) の続き）**: 同 spec の task 7.1 が是正後の実機再採取（7 遷移・観測行 438・コミット `4fd2fb3c`・`release`）の数量で段階裁定を確定し、**`SetWindowPosCommand::flush`（`crates/wintf/src/ecs/window/command.rs:425-505`）の逐次 `SetWindowPos` を `Begin/Defer/EndDeferWindowPos` の 1 バッチへ移す**案を採用した。実装は同 spec の task 7.2。**追記(71) が渡した「flush 経路の前提」がもう一度動く**ので、本 spec が適用順・適用回数・駆動周期を論じるときは下の数量を前提にすること。
> - **⑴ 何が確定したか（数量）**: 一括書込の総所要は `Σcall_us` と **99.82〜99.92%** 一致し、差はキュー・積み上げ・取り出しの全部で 145〜308µs しかない。**遅さは `SetWindowPos` 呼出そのものにある。** 遷移区間内の書込 **28 本すべて**が呼出の内側に `WM_WINDOWPOSCHANGED → WM_DPICHANGED → WM_WINDOWPOSCHANGED` を持ち（中間形 0 本・`WM_DPICHANGED` は 28/28 が `in_swp=true`）、1 本あたり最小 13,892・中央値 **51,666**・最大 110,965µs かかる。合流（追記(71) ⑵）は**回数を 6→4 に減らしたが時間はほぼ減っていない**——総所要の中央値は 205,638.5→193,329µs（**−6.0%**）で、消えたのは軽い群（`WM_WINDOWPOSCHANGED` だけを内側に持つ 24 本・総呼出時間の 5.2%）だけである。
> - **⑵ 本 spec の測定に直に効く数字**: 遷移を含むフレームは **254.3〜351.2ms** 続く（**同じログの定常フレーム周期は 8.0929〜8.4997ms**——200 フレーム以上続く定常区間 101 本の全走査——なので **約 30〜43 倍**）。うち一括書込が 144,071〜268,772µs を占める。**遷移フレームは本 spec が扱う「定常状態の CPU」とは別の山**であり、混ぜて平均しないこと。
> - **⑶ 実装が触るのは 1 関数だけ**: 接触集合は `SetWindowPosCommand::flush`（`command.rs:425`・81 行）のみ。tick のスケジュール構成・相の並び・駆動には触れない（同 spec の要件 9.3 判定）。**本 spec の In-scope（`wintf` の `runtime`／`ecs::world`）とは接するが重ならない。**
> - **⑷ 本 spec が flush へ手を入れるなら先に読むこと（`file:line` 確認済み）**: ① `EndDeferWindowPos` も `SetWindowPosGuard`（`command.rs:138`）の内側で呼ばないと `is_self_initiated()`（`:86`）が偽になり、同期送達される `WM_WINDOWPOSCHANGED` が外部権威の腕へ落ちる。② `WM_WINDOWPOSCHANGED` ハンドラ（`crates/wintf/src/ecs/window_proc/window_pos.rs:41`）は手順③で `flush_window_pos_commands()` を**無条件に**呼ぶ（`:290`）ので、**バッチの内側で入れ子 flush が起き得る**（`flush()` の doc が `FlushEpoch` で時刻基準を復元する形を既に持つ）。③ Z 専用指令は合流の対象外（`is_coalescible` の 3 連言・`command.rs:229`）であり、`DeferWindowPos` でも per-window flags で同居させ**適用順と結果を変えない**（要件 10.3）。
> - **⑸ `ghost-window-zorder` の受け先は引き続き本 spec が兼ねる。** 上流は `.kiro/specs/completed/` にあり申し送りを消化できず、`zorder_pair_maintain.rs` を担当ファイル集合に持つ生存 spec は `areka-P0-dpi-transition-atomicity` の**他に**は 2026-08-21 時点でも存在しない（追記(71) ⑷ の判断をそのまま維持）。
> - **⑹ 未特定として残っているもの（本 spec の領分に近い）**: `SetWindowPos` 呼出の内側で、**自前のウィンドウプロシージャが 1 行も走っていない区間が 47.5%**（639,106／1,344,271µs・中央値 18,059µs）を占める。呼出区間は `[t_us − call_us, t_us]` とし、**同一 `frame` の** `kind=msg` の `t_us` で 3 分割して数えた（`t_us` はフレームごとに 0 から数え直すので `frame` の一致を条件に入れないと後続フレームの `msg` が紛れ込む）。そこが DWM の再合成待ちか、レイヤ化ウィンドウの再構成か、DPI 変更に伴うウィンドウマネージャ側の同期かは**分解できていない**（ハンドラ出口の記録が無い）。分解には観測点の追加か OS 側のトレースが要る。**憶測で埋めないこと。** 併せて、積み上げから一括書込までの区間の前半（文字層の再構築とテキスト測定・8,518〜82,416µs・中央値 25,899µs）は追記(71) 以前から本 spec への申し送り済みである（`mechanism-ledger.md` §5）。
> - **⑺ 正本**: `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` **§10**（§10.3 が名指しの 3 量・§10.5 が候補 4 案の突合と採否・§10.6 が実装への受け渡し）／`design.md` の「C8 原子性の段階裁定」。生ログは `C:\Users\maz-o\AppData\Local\areka-diag\atom-71-recapture-1\atom-signoff.log`（リポジトリ外・複製しない）。
>
> **📌 2026-08-22 追記(78)（`areka-P0-dpi-transition-atomicity` からの申し送り・追記(74) の実装が着地した後の実測。追記(74) ⑴ の数量は是正前の値なので、そのまま現況として読まないこと）**: 追記(74) は task 7.1 の裁定（`DeferWindowPos` 一括の採用）を渡した。**task 7.2 で実装が着地し、task 7.3 の実機採取（`atom-73-signoff-1`・8 遷移・`release`）で効果が測れた。**
> - **⑴ 効いたのは回数と同時性、時間ではない**: 1 遷移内の書込の散らばりは **93,152〜157,684µs → 40〜101µs**。一方、一括書込の総所要（`flush stage=end` の `total_us`）は 144,071〜268,772µs（平均 192,247）→ 143,231〜231,910µs（平均 **188,711**）で **−1.8%＝実質変わらない**。**追記(74) ⑴ の「遅さは `SetWindowPos` 呼出そのものにある」という結論は、投入をまとめても崩れなかった。**
> - **⑵ バッチは実機で成立している**: `in_batch=true` **37 件**／`false` **0 件**・縮退 WARN 0 件。上の `total_us` の比較は**同じ投入形どうし**の比較である（縮退して逐次へ落ちてはいない）。
> - **⑶ ⚠ `Σcall_us／total_us` を是正前後で比べてはならない**: 追記(74) ⑴ の **99.82〜99.92%** は**是正前**の値である。task 7.2 以後の `call_us` は `DeferWindowPos` への**投入だけ**の所要へ意味が変わった（`command.rs:373` の doc が明記）ので、是正後の同比 **6.0〜18.1%** は OS 側のコストが減ったことを**意味しない**。比べられるのは `total_us` と `in_batch` である。
> - **⑷ 追記(74) ⑹ の「未特定の 47.5%」は未解決のまま残る**: 本 spec の領分に近い量として引き続き有効である。バッチ化はこの区間を分解しない。
> - **⑸ 上流はここで閉じた**: atom の実機サインオフは実機専用系統 FAIL のまま**開発者裁定 GO** で完了した。残った「絵が先・窓が後」の 210,329〜306,301µs は `areka-P0-present-write-coherence`（W8・本 spec と同格の優先度低）が引き受ける。**本 spec とは接触面が違う**——あちらは `presenter/show.rs` の提示の順序、本 spec は描画負荷そのものである。
> - **⑹ `command.rs` に 1 行の在庫がある（実施は本 spec と cage の調整事項）**: `SELF_INITIATED_DEPTH`（`crates/wintf/src/ecs/window/command.rs:49`）はプロセス共有の `AtomicI32` だが意味論はスレッド局所であり、`Cell<i32>` へ移すのが正しい形である（本番の欠陥ではなく、**テスト間の汚染源**）。本 spec は `command.rs::flush` を接触集合に持つので、flush へ手を入れるついでに片づくなら安い。**症状の側（檻の汚染・錠の退役）は `areka-P0-test-cage-determinism` の追記(76) ⑹ が受けている**ので、着手する側がもう一方へ知らせること。
> - **⑺ ⚠ 要件 10.3（Z 指令の順序・結果不変）の成立には、コードが強制していない前提が 1 つある**（上流 task 5.3 のレビューの残余所見。**結論の側は今まで誰にも渡っていなかった**ので本 spec が受ける）: 各 `SetWindowPos` は `WM_WINDOWPOSCHANGED` を同期送出し、そのハンドラが `flush_window_pos_commands()` へ**再入**する（`crates/wintf/src/ecs/window_proc/window_pos.rs:290`・手順 ③）。ジオメトリ書込の順を入れ替えると、**再入 flush が他窓の Z 指令とどう噛み合うかの時点が変わる**。現状は無害だが、それは「**ウィンドウプロシージャ側が Z 指令を積まない**」という事実に依っているだけで、**コードはそれを強制していない**。**本 spec が flush の駆動・間引き・順序に手を入れるとき、あるいは wndproc 側から Z を積む経路を足すときは、この前提が壊れる。** 追記(74) ⑷② の入れ子 flush・⑷③ の Z 非合流と合わせて読むこと——部品はそちらに在ったが、「前提が未強制である」という結論だけが宙に浮いていた。
> - **⑻ 正本**: `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` **§11.6**（B-2b の効果の実測表）・**§10.6.1**（`call_us` の意味の変化）。


## Problem

**アイドルのゴーストが SSP の 3.6 倍の CPU を食う。発話中は 4.4 倍。**

同一マシン・同一ゴースト（emo2）・同一拡大率（200%）・同一手順・25 分走行での実測:

| | areka | SSP | 比 |
|---|---:|---:|---:|
| **CPU 平均** | **10.97%** | **3.05%** | **3.6×** |
| CPU の底（アイドル） | 3.60% | 1.77% | 2.0× |
| **CPU の頂（発話中）** | **20.42%** | **4.64%** | **4.4×** |
| Private メモリ | 163.4 MB | 54.2 MB | 3.0× |
| スレッド数 | 83 | 32 | 2.6× |

（1 コア換算 %。マシンは 22 論理プロセッサなので、10.97% は全体の約 0.50%。）

デスクトップマスコットは常駐が前提なので、この差はバッテリー・発熱・他アプリへの
圧迫として日常的に効く。**要件 4.4 が掲げる「release アイドル CPU 3.0% 未満」は
`recompose-budget` の手段（表示 1 コマの適用経路）では到達できないことが実測で確定した。**

### 未達のまま送られてくる判定式がもう 1 つある——進行境界スキップ（catch-up）

`recompose-budget` の要件 4.2⑵ は「**定常状態での進行境界スキップ（catch-up）0 件**」を
掲げるが、同 spec は**一度も 0 に達しないまま着地する**。実測（正本は
`.kiro/specs/completed/areka-P0-recompose-budget/remeasure-2026-08-15.md`）:

| 走行 | dev | release |
|---|---:|---:|
| 着手前ベースライン（短時間・2026-08-14） | 78 件 | 41 件 |
| 定常アロケーション是正後（短時間・容量 1・§1） | 21 件 | 28 件 |
| **最終形（短時間・容量 3・§5.7）** | **22 件** | **17 件** |
| 長時間 25 分（容量 1・§5.5） | — | **69 件** |

catch-up は、進行のティッカーが**複数のコマ境界をまたいでしまい 1 回に畳んで発火する**現象
（ログ文言 `ticker catch-up: skipped multiple boundaries, firing once` ／ `loop ticker …`）で、
見た目にはコマ落ちとして出る。半減以下まで減ったが 0 にはならなかった。

**なぜ本 spec の領分か**: 1 コマの適用は 1,240µs まで下がっており、コマ待ち（150ms／22ms）に
対して桁で余裕がある。**`apply_show` を 18 分の 1 にしても消えなかった**以上、コマの遅れは
表示 1 コマの適用経路の**外**から来ている。上の内訳が示すとおり、その外側の主役は
`try_tick_world` が毎秒 120 回・1 回 578µs を費やしているフレーム駆動そのものである。
**フレーム駆動の負荷が下がれば、1 コマがコマ待ちを超える機会が減る**——これが ⑵ を
本 spec が引き受ける理由である。

**ただし機序は未確定**（`recompose-budget` §5 の読みは「talk 再生や複数面の同時更新が重なった
瞬間に出ている」）。**残る件数の出どころを実測で確定させるところから始めること**——
判定式⑵ が数えているのは表示側ではなく進行側のログなので、駆動の間引き（候補 A）は
件数を**増やす**向きにも働きうる。

## Current State

### `recompose-budget` が到達した地点（2026-08-15 完了）

1 コマ適用を **22,210µs → 1,240µs（18 分の 1）** まで削った（定常アロケーション完全ゼロ・
リサンプル計算の作り直し・冗長ゼロ埋め除去・キャッシュ容量 1→3 の 4 段）。
CPU は 24.9% → 約 11% へ半減した。

### それでも届かない理由（実測で確定）

**`apply_show` は CPU の 3.3% しか占めていない。**

| 走行 | apply_show の 1 コア換算 | 実測 CPU | apply の占有率 |
|---|---:|---:|---:|
| 着手前 | 2.56% | 24.6% | **10.4%** |
| 現在（容量3） | 0.35% | 10.4% | **3.3%** |

**最初から主因ではなかった。** 18 分の 1 にしても、削れる対象がもう残っていない。

### 負荷の実体（計測で特定済み）

使い捨て計測を仕込んで確定させた内訳:

| 計測点 | 呼び出し | 1 回あたり | 壁時計占有 |
|---|---:|---:|---:|
| `evaluate_targets`（クリック透過判定） | 120回/秒 | 5.2µs | 0.07% |
| `apply_show`（表示 1 コマ） | 1.18回/秒 | 1,240µs | 0.35% |
| **`try_tick_world`（ECS の tick）** | **120回/秒** | **578µs** | **6.85%** |

**`EcsWorld::try_tick_world` が 13 本のスケジュールを毎秒 120 回、全部回している。**
コマ適用は毎秒 1.18 回しか起きないので、**tick の 98% は表示に変化がない**。

tick 578µs の内訳（1 tick あたり・25 分走行の平均）:

| スケジュール | µs | 割合 |
|---|---:|---:|
| **FrameFinalize** | **182** | **31.5%** |
| **Draw** | **143** | **24.8%** |
| Layout | 56 | 9.7% |
| Input | 55 | 9.6% |
| Update | 50 | 8.7% |
| PostLayout | 42 | 7.2% |
| 残り 7 本 合計 | 50 | 8.5% |

**上位 2 本で 56%。**

**注意**: `Instant::elapsed()` は壁時計であって CPU 時間ではない。GPU 待ちが混じって
いれば CPU としてはこれより小さい（UI スレッドの実測 CPU は約 3.1% だったので、
半分程度は待ちの可能性がある）。**設計フェーズで CPU 時間による裏取りが要る。**

### 既に解決済みで、本 spec の対象外

`recompose-budget` の実測により以下は解決を確認済み。**本 spec は再発させないことだけが責務**:

- **メモリリーク**: Private 12分→24分で **+0.1 MB**・ハンドル 413→406・スレッド 89→83
  （いずれも増加なし）
- **負荷の単調上昇**: 着手前は 25 分走行で **+0.84 %/分**（判定「単調上昇」）だったものが、
  是正後は **−0.045 %/分**（判定「頭打ち」）。容量 3 の 25 分診断でも tick は
  587.1 → 592.0µs（**+0.8%／23.8 分**）で平ら、13 スケジュールすべて平らか減少。

## Desired Outcome

- **アイドル時の CPU が SSP と同等圏**（実測 3.05% に対し、おおむね 3% 台）
  = `recompose-budget` 要件 4.4⑷a（release アイドル CPU 3.0% 未満・実測 10.38%／長時間 11.83%）を
  引き受ける。**ただし目標を絶対値で置くか画素あたりの効率で置くかは要件段階の裁定事項**
  （後述「⚠ 比較の前提に未検証の穴がある」）
- **定常状態の進行境界スキップ（catch-up）が 0 件**
  = `recompose-budget` 要件 4.2⑵ を引き受ける（現状 release 17 件・dev 22 件・長時間 69 件）。
  判定は同 spec の `judge-perf.py` 判定式⑵をそのまま使う（dev・release 両ビルドに適用）
- **発話中の CPU 差が説明可能な範囲に収まる**（現状 4.4 倍は大きすぎる）
- **リークと単調上昇を再発させない**（`recompose-budget` が確立した判定基準をそのまま適用）
- 見た目の追随（クリック透過・αマスク・ドラッグ・DPI 追随）が劣化しない

## Approach

**未確定。設計フェーズで決める。** 現時点で見えている候補:

### 候補 A: アイドル時の tick 間引き
表示に変化が無い tick で 13 スケジュールを回さない。変化の検出は
「コマ適用が発生したか」「入力があったか」「アニメの境界を跨いだか」で足りる可能性がある。
- **効果の見込み**: tick が 6.85% を占めるので、120Hz → 10Hz なら桁で落ちる
- **リスク**: 「毎フレーム評価する」前提の上にクリック透過も αマスク追随も乗っている。
  間引くと見た目の追随が遅れる。`runtime/mod.rs` は VSync tick 毎の再評価を
  **R2.4 の要件として明記**しているので、要件レベルの調停が要る

### 候補 B: 上位 2 スケジュールの是正
`FrameFinalize`（182µs）と `Draw`（143µs）で 56% を占める。
中身を割ってから、変化が無いときに早期脱出させる。
- **効果の見込み**: 全体を触らずに済むぶん安全。ただし上限は 56%
- **リスク**: 小さい。既存の駆動設計を変えない

### 候補 C: tick 周期そのものの見直し
実測は **120 回/秒**（`ticker.rs` の設定は 16ms＝62.5Hz なので、
VSync ブリッジが二重に起こしている可能性がある）。**まずこの食い違いを調べる価値がある。**
- **効果の見込み**: もし二重起床が実在すれば、直すだけで半減
- **リスク**: 小さい。ただし原因調査が先

**推奨は C → B → A の順**（安全で効果の確実なものから）。ただし A に踏み込まないと
SSP 水準には届かない可能性が高い。

## Scope

- **In**:
  - フレーム駆動の周期と、変化が無いときの早期脱出（`wintf` の `runtime` / `ecs::world`）
  - `FrameFinalize` / `Draw` スケジュールの内訳解析と是正
  - 上記が見た目の追随（クリック透過・αマスク・ドラッグ・DPI）を劣化させないことの固定
  - SSP との同一手順比較の常設化（測り方の登記）
- **Out**:
  - 表示 1 コマの適用経路（`presenter/show.rs` の `apply_show`）——`recompose-budget` が
    18 分の 1 まで削り、CPU の 3.3% しか占めない。**ここを更に削っても効かない**
  - 合成アルゴリズム本体（`build_plan` / `blit::execute`）
  - メモリ使用量そのものの削減（SSP の 3 倍だが、リークではなく設計上の常駐量）
  - スレッド数の削減（SSP の 2.6 倍だが、CPU との因果は未確認）

## Boundary Candidates

- **フレーム駆動の周期決定**（いつ tick するか）
- **スケジュール実行の要否判定**（tick したとして、どの schedule を回すか）
- **個々のスケジュールの内部コスト**（回すとして、いくらかかるか）
- **見た目の追随の保証**（間引いても遅れないことの固定）

最初の 2 つは同じ関心（駆動の設計）なので 1 spec で扱えるが、3 つ目は
スケジュールごとに担当が違うので、内訳次第では分割候補になる。

## Out of Boundary

- `apply_show` の更なる最適化（効果が無いことが実測で確定済み）
- **SSP との完全一致**——後述の画素数の違いにより、そもそも同じ土俵ではない可能性がある
- メモリ常駐量の削減
- 発話（talk）そのものの処理コスト——山の高さは発話の実装に依存し、
  フレーム駆動とは別の関心。ただし**本 spec の実測で切り分ける**こと

## ⚠ 比較の前提に未検証の穴がある（設計フェーズで必ず解く）

**SSP は 100% で描画してから 200% へ引き伸ばしている可能性が高い**（開発者観察:
バルーンの文字がぼやけている）。事実なら:

| | areka | SSP（推定） |
|---|---|---|
| 実描画の画素数 | **764×1094**（836k） | **382×547**（209k）→ 引き伸ばし |
| 画素の仕事量 | **4×** | 1× |
| 見た目 | 鮮明 | ぼやける |

**この推定が正しければ、CPU 3.6 倍という差は画素あたりでは 0.9 倍**——つまり互角以上。
「SSP 水準」という目標が、**暗に品質を落とすことを要求している**ことになる。

したがって設計フェーズは次を必ず先に決める:
1. SSP の描画解像度を**実測で確定する**（推定のままにしない）
2. 確定した場合、目標を「CPU の絶対値」で置くのか「画素あたりの効率」で置くのかを
   **開発者裁定**にかける

## Upstream / Downstream

- **Upstream**:
  - `areka-P0-recompose-budget`（W6.5・2026-08-15 完了）——負荷の所在を実測で特定し、
    測定基盤 `tools/perf/`（採取ランナー・判定スクリプト・自己較正 fixture 17 件）を
    残した。**本 spec はこの基盤をそのまま使う**
    - **同 spec は判定式⑵（進行境界スキップ 0 件）を未達のまま送った**（release 17 件・
      dev 22 件・長時間 69 件）。⑷a（アイドル CPU 3.0% 未満・10.38%）と**同じ走行で同時に
      落ちた 2 件**であり、**本 spec は両方を引き受ける**。送り出し側の登記は同 spec
      requirements.md の Requirement 4 改訂欄（2026-08-15）にある
  - `completed/areka-P0-emo-present` R4.1（キャッシュ意味論・2026-08-15 に容量 3・LRU へ改訂済み）
  - `wintf` の フレーム駆動（`runtime/mod.rs` の VSync ブリッジ・`runtime/tick_bridge.rs`・
    `ecs/world/mod.rs` の `try_tick_world`）
- **Downstream**:
  - `areka-P0-dpi-transition-atomicity`（W6.75）——`tick_bridge` / `command.rs` の
    flush 経路を改造する可能性があり、本 spec が同じ経路を触るなら順序の調停が要る
  - `areka-P0-test-cage-determinism`（W6.9）
  - `areka-P0-emo2-conformance-e2e`（W7）

## Existing Spec Touchpoints

- **Extends**: 無し（新しい境界）
- **Adjacent**:
  - `recompose-budget`（`presenter/show.rs`）——**本 spec は触らない**
  - `dpi-transition-atomicity`（`tick_bridge` の flush 経路）——**着手時に調停が要る**
  - `collision-dpi-hittest`（完了）——クリック透過の評価タイミングを共有する。
    間引くなら R2.4 の「VSync tick 毎の再評価」と正面から衝突する

## Constraints

- **wintf の中核（フレーム駆動）に手を入れる。** クリック透過・αマスク追随・ドラッグ・
  DPI 追随がすべて「毎フレーム評価される」前提の上に乗っている。壊せば見た目が遅れる
- **`runtime/mod.rs:231-237` が VSync tick 毎の再評価を R2.4 の要件として明記している。**
  間引きは要件レベルの調停を要する
- 測定は 7 分走行が最低価格。**2 分の窓は使えない**（同一条件で 5.37% と 18.57% に振れた実測あり）
- **非交互の前後比較はこの機械で結論を反転させる。** 同一プロセス内の交互取得が必須
  （実測: 一括順で「O3 が 17% 速い」→ 交互で「33% 遅い」へ反転）
- 実機計測は開発者の静寂確認を要する（`recompose-budget` 要件 2.7 の規律を踏襲）
- SSP 比較用に emo2 を SSP へ配置する手順が確立済み（`C:\wintools\ssp\ghost\emo2-perf`。
  **測定後は削除すること**——本 brief 執筆時点では配置したままなので、着手時に確認する）

## 測定資産（そのまま使える）

- `tools/perf/invoke-perf-run.ps1` — 有界実走＋CPU 時系列採取（`-ConfirmQuiet` 必須）
- `tools/perf/judge-perf.py` — 集計＋合否判定（0.3.2・自己較正 fixture 17 件）
  - 判定式⑷b（単調上昇しない）は本 spec でもそのまま使う
  - **⚠ 較正値 `WARMUP_EXCLUDE_SEC=60` は容量 3 で暖機が 4 倍に伸びたため見直し対象**
- `.kiro/specs/completed/areka-P0-recompose-budget/remeasure-2026-08-15.md` — 是正後の全実測
- `.kiro/specs/completed/areka-P0-recompose-budget/baseline-2026-08-14.md` — 着手前の全実測

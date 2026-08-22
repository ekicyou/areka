# Implementation Plan

> 設計 Flow 1 の状態機械（PREFLIGHT→BASELINE→RANK→SELECT→IMPLEMENT→TEST→REMEASURE→DECIDE→RECORD→…→FINAL）と対応する。タスク 1〜8 は「自走ループの仕組みを作る」、タスク 9 は「その仕組みを実際に回す」。是正候補のうち実行器の見直し・areka 側の毎フレーム処理の変化時のみ化・tick 外の周期（設計 C17〜C19）は固定タスクにせず、タスク 9 の周回内で順位表が選んだものだけを実装する（要件 3.1・3.3）。tick の門（設計 C16）だけは無条件に作り（タスク 3）、周 1 の A/B で既定値を決める（2026-08-22 裁定）。タスク 1 は以後の全タスクの派遣モデルを決める規則なので先頭に置く（要件 1.13）。タスク 8 の README・COMPAT・申し送りは文書作業だが、要件 5.9・7.1〜7.4・8.2〜8.4 が成果物として要求するため意図的に含める。
>
> 同一ファイルの注意: `crates/wintf/src/ecs/world/mod.rs` は 2.1／2.2／3.1／3.2／3.3 が触る（2.1／3.1／3.2 は自分の `mod` 宣言 1 行のみ）。`crates/wintf/src/ecs/window/command.rs` は 3.4（旗を立てる 1 行）と 4（スレッド局所化）が触る。並列印 `(P)` はこの前提の下で付けている。

- [x] 1. 開発プロセスの前提: kiro-impl と kiro-validate-impl に「派遣モデルの決定」を追加
  - kiro-impl の Preflight に、自分のモデル名（システムプロンプトの「You are powered by the model named …」）を読み、Fable 系または判別不能なら派遣モデルを opus、それ以外（既に Opus 以下）なら継承（`model` 引数を省略）とする規則を置く
  - 実装者・レビュアー・デバッガーの 3 つの派遣箇所と、kiro-impl から呼ぶ最終検証（kiro-validate-impl）の派遣へ同じ規則を適用し、決定を 1 度だけ実行出力に印字する（`dispatch model: opus` または `dispatch model: inherit`）
  - kiro-validate-impl の Subagent Dispatch 節に「呼び出し元から opus を渡されたら各派遣に付ける・単独実行時は自分で同じ判別を行う」の 1 節を足す
  - 観測可能な完了状態: 両 SKILL.md に規則の節があり、タスク 2 以降の kiro-impl 実行出力に `dispatch model:` 行が 1 度出る
  - _Requirements: 1.13_
  - _Boundary: kiro-impl, kiro-validate-impl_

- [ ] 2. 実行体の観測（既定 OFF・有効化しなければ費用 0）
- [x] 2.1 (P) スレッド名簿（役割名・TID・複製ハンドル）と Win32 の安全ラッパを wintf に新設
  - プロセス共有の名簿に、生成側が自分のハンドルを複製して役割名つきで登録する口と、一覧（スナップショット）を取り出す口を置く
  - 役割名は固定語彙（vblank・cursor_monitor・ui・ticker_dispatcher_kanade・ticker_loop・actor:<name>・perf_report・unregistered_rest）の定数として持つ
  - GetThreadTimes／GetProcessTimes／DuplicateHandle の安全ラッパは wintf の Win32 ラッパ層（`api.rs`）に置き、既存 feature の範囲に収める（ToolHelp は使わない・`Cargo.toml` 非接触）。2.4 の報告器はこのラッパを呼ぶだけにする
  - `ecs/world/mod.rs` へは自分の `mod` 宣言 1 行のみ追加（2.2／3.1／3.2 と同一ファイル）
  - 決定論テスト（兄弟ファイル）: 登録→一覧・別スレッドからの登録が一覧に見える・語彙の固定
  - 観測可能な完了状態: テストが緑で、名簿に登録した役割名と TID がスナップショットに出る
  - _Requirements: 2.3, 2.12, 6.8, 8.6_
  - _Boundary: thread_registry, api.rs_

- [x] 2.2 フレーム駆動の相別観測（tick_diag）を新設し tick に計時点を置く（`ecs/world/mod.rs` の本文を触るため 2.1 と並列にしない）
  - target `wintf::tick` の前置ガードを tick の冒頭で 1 度だけ評価し、偽なら時刻取得も行の組立も行わない
  - 1 秒窓で tick 回数・省略数・心拍で回った数・壁時計の合計と最大・UI スレッド CPU の差分・13 本のスケジュール別の壁時計 µs を集約し、窓が閉じたら `[tick] kind=window …` を 1 行出す（壁時計と CPU を別フィールドで区別）
  - 13 本の呼出の前後で（点灯時のみ）計時して窓へ加算する。13 本の順序と FrameCount の進め方は変えない
  - 決定論テスト（兄弟ファイル）: 行のフィールド名に重複なし・13 本の名前と順序・窓の切れ目・OFF 時に時刻取得を呼ばない構造検査（ガードの評価が時刻取得より前に在る）
  - 観測可能な完了状態: `RUST_LOG=wintf::tick=debug` の実走で `[tick]` 行が約 1 秒ごとに出て、OFF の実走では 1 行も出ず、既存の 13 本固定順テスト 2 本が緑のまま
  - _Requirements: 2.5, 2.6, 2.12, 3.8, 6.5, 6.8_
  - _Boundary: tick_diag, ecs/world/mod.rs_

- [x] 2.3 スレッドの生成点で名簿へ登録（wintf・ghost・actor をまたぐ結線）
  - vblank 検出・カーソル監視・UI スレッド・ticker 2 系統（dispatcher/kanade・ループ）・アクター生成点で、1 行ずつ役割名を宣言して名簿へ登録する
  - 依存方向は areka 系→wintf のまま（wintf は areka を知らない）。登録以外の挙動は変えない
  - 観測可能な完了状態: 名簿のスナップショットを読む決定論テスト（wintf 側は vblank／cursor_monitor／ui の登録を、ghost／actor 側は ticker_*／actor:* の登録を、それぞれ自 crate のテストで固定）が緑で、全登録の役割名が固定語彙に含まれる
  - _Requirements: 2.3_
  - _Depends: 2.1_

- [x] 2.4 スレッド別・プロセス CPU の報告器（perf_thread_report）を areka に新設
  - target `areka::perf` を起動時に 1 度評価し、OFF なら報告スレッドを起こさない（費用 0）。ON なら 60 秒ごと（`AREKA_PERF_THREAD_REPORT_SEC` で変更可）と終了直前にスナップショットを出す
  - 2.1 のラッパで名簿を舐めて GetThreadTimes、プロセス全体は GetProcessTimes。名簿に無い残り（タスクプール等）はプロセス CPU から名簿合計を引いた差として `unregistered_rest` の 1 行で出す（黙らない）
  - 行の語彙: perf(thread) は 1 スレッド 1 行・perf(process) は 1 行・1 行内のフィールド名重複なし・既存の perf 行と遷移観測行は不変
  - 決定論テスト（兄弟ファイル）: 役割語彙の全件・行の語彙・スナップショット差分の計算・unregistered_rest の算出
  - 観測可能な完了状態: `RUST_LOG=areka::perf=debug` の実走で perf(thread)／perf(process) 行に vblank・cursor_monitor・ui・ticker_*・actor:* が役割名つきで並び、OFF の実走では報告スレッドが生成されない
  - _Requirements: 2.3, 2.6, 2.12, 3.8_
  - _Boundary: perf_thread_report_
  - _Depends: 2.1, 2.3_

- [ ] 3. tick の門（変化が無いとき 13 本を回さない・無条件に実装し既定値は周 1 の A/B で決める）
- [x] 3.1 (P) 旗（tick_wake）とメッセージ→旗の写像
  - プロセス共有のビット集合（POINTER・DRAG・WINDOW_CMD・ZORDER・WM_GEOMETRY・PRESENT・ANIM・REARM・GRAPHICS・FORCE）へ任意スレッドから原子的に立てる口・最も早い期限を保持する口・読んで倒す口（期限到来の有無を添える）
  - 純関数 wake_bits_for_message: 幾何・DPI・表示構成・活性化・表示/破棄系のメッセージ→WM_GEOMETRY、ポインタ系→POINTER、未知→FORCE（疑わしいときは回す）
  - `ecs/world/mod.rs` へは自分の `mod` 宣言 1 行のみ追加
  - 決定論テスト（兄弟ファイル）: 立てる/倒すの原子性（別スレッドで立てた旗が次に読むとき見える）・期限の最小保持・写像表（既知メッセージ全件の期待ビット・WM_DPICHANGED→WM_GEOMETRY・未知→FORCE）
  - 観測可能な完了状態: テスト緑、倒した直後に立てた旗が次の読み取りで取れる
  - _Requirements: 4.4, 6.1, 6.5_
  - _Boundary: tick_wake_

- [x] 3.2 (P) 判定の純関数（tick_gate::should_run）
  - 入力（旗・期限到来・前回実行からのフレーム数・起動からのフレーム数・門の有効）→ 回す(理由)／省略。理由の優先順位は 無効→起動直後(600 フレーム)→旗→期限→心拍(30 フレーム)
  - `ecs/world/mod.rs` へは自分の `mod` 宣言 1 行のみ追加
  - 決定論テスト（兄弟ファイル）: 入力の全組合せ（旗 2^10 × 期限 2 × 心拍の境界 × 起動直後の境界 × 有効 2）で結果を固定・省略は「旗ゼロかつ期限なしかつ心拍未満かつ起動直後でなく門が有効」のときのみ
  - 観測可能な完了状態: 全組合せテストが緑
  - _Requirements: 3.2, 3.7, 6.1, 6.5, 6.8_
  - _Boundary: tick_gate_

- [x] 3.3 門を tick 駆動に組み込む（統合タスク: wintf の World と tick_bridge・areka の起動時読取）
  - EcsWorld に門の有効切替・decide_tick（旗を読んで倒す→純関数→カウンタ更新→点灯時は tick_diag へ記録）・省略の記録を置く。旗の取得に失敗し得る経路は error! を残して「回す」へ倒す
  - フレーム 1 回の処理: 再入ガード→借用→decide_tick→回すなら 13 本（FrameCount 進む）／省略なら FrameCount・FrameTime・TickStart を進めない。窓書込の flush は常に呼ぶ。旧経路と FrameHarness は触らない
  - `AREKA_TICK_GATE=1|0` を areka の起動時に読んで門の有効を上書きできる（A/B 比較と安全弁）。周 1 の A/B まで既定は OFF
  - 決定論テスト: headless の World で 省略→表示指令の旗→次の判定が回す・省略中に FrameCount が進まない・省略後に Changed が拾える。省略経路でも 13 本の順序が既存テストの期待順と同じ
  - 観測可能な完了状態: 既存の 13 本固定順テスト 2 本・dpi-transition-atomicity の決定論 8 遷移・ghost-window-zorder／scope-chain-gap／windowposition-limit の既存テストが全て緑のまま、env で ON にした実走の `[tick]` 行で skipped が 0 より大きい
  - _Requirements: 3.2, 3.4, 3.7, 4.4, 4.5, 6.2, 6.3, 6.8_
  - _Boundary: tick gate wiring (wintf ecs/world/mod.rs, runtime/tick_bridge.rs, areka startup)_
  - _Depends: 2.2, 3.1, 3.2_

- [x] 3.4 wintf 側の生産者を結線（旗を立てる 1 行ずつ）
  - ウィンドウプロシージャの配送点（写像で立てる）・入力バッファへの投入（POINTER）・窓書込指令の積み上げ（WINDOW_CMD・`command.rs` の enqueue に 1 行＝タスク 4 と同一ファイル・タスク 4 が後に続く）・Z 順要求（ZORDER）・ドラッグ中の tick 末（DRAG の自己再予約）・活性アニメータ（ANIM）・GraphicsCore 無効（GRAPHICS）・表示構成変更（WM_GEOMETRY）
  - 生産者一覧の字面検査（各生産者ファイルに旗を立てる呼出が在ること）をテストに置く
  - 観測可能な完了状態: 字面検査テストが緑、門 ON の実走でドラッグ中は毎フレーム回り（その窓の skipped が 0）放置時は省略される
  - _Requirements: 4.1, 4.2, 4.3, 4.5_
  - _Boundary: tick_wake producers (wintf), command.rs enqueue_
  - _Depends: 3.1, 3.3_

- [x] 3.5 areka 側の生産者を結線（旗を立てる 1 行ずつ）
  - 表示指令の送信端（PresentBridge・MoveCueSink・lifecycle）→PRESENT、talk 進行中（時刻起点が確立し未完）→REARM、バルーン表示の待ち時間→期限、hover 注入が有効→REARM
  - 字面検査の一覧へ追加（3.4 と同じテストを広げるため 3.4 の後に順次）
  - 観測可能な完了状態: 門 ON の実走で発話中の表示成立点が省略に巻き込まれず（判定式⑴ p95 が OFF と同等）、放置時の省略率が上がる
  - _Requirements: 4.1, 4.5, 4.6_
  - _Boundary: tick_wake producers (areka)_
  - _Depends: 3.4_

- [x] 4. command.rs の所有: SELF_INITIATED_DEPTH をスレッド局所へ（3.4 と同じファイルのため並列不可・3.4 の後）
  - プロセス共有の整数 → スレッド局所の整数。読み書き 3 箇所（判定・ガードの生成・解放）のみ変更。錠 lock_self_initiated_for_test は残し、doc を「スレッド局所化後は不要＝退役候補（test-cage-determinism が受ける・呼出 21 箇所／5 ファイル）」へ
  - module doc に「ウィンドウプロシージャ側が Z 指令を積まない」という現状の前提（コードは強制していない）と、flush の駆動・順序を変えるときはこの前提の成立を確かめる義務を登記
  - 決定論テスト（兄弟ファイル）: 別スレッドでガードを持ち上げている間、主スレッドの判定が偽（錠なしで並列に走らせても緑）
  - 観測可能な完了状態: 新テスト緑、既存の command_coalesce／command_batch／command_transition／window_pos_transition／frame_transition_atomicity の各テスト（計 58 本）が 1 本も赤にならない
  - _Requirements: 3.5, 4.5, 6.4, 6.6, 6.8, 8.2_
  - _Boundary: command.rs_

- [ ] 5. 計測の道具（基盤: 静寂・採取・サンプリング・台帳）
- [x] 5.1 (P) 静寂確認の自動化（check-quiet.ps1）
  - マシン全体の CPU を指定秒数・1 秒刻みで採って平均と最大、既知の重いプロセス名の有無（測定対象の areka は PID で除外）。閾値は目標定義ファイルまたは引数
  - `quiet-<stage>.txt` へ平均・最大・該当プロセス一覧・判定・時刻。exit 0（静か）／2（静かでない）。決定論（同じ入力から同じ文面）
  - 観測可能な完了状態: 静かな状態で exit 0・`cargo build` 中に exit 2 となり、どちらも出力ファイルが残る
  - _Requirements: 1.5, 2.8_
  - _Boundary: check-quiet_

- [x] 5.2 採取ランナー 1.1.0（追加のみ・既存の呼び方は不変）
  - `-AutoQuiet`（`-ConfirmQuiet` と排他・5.1 を起動前に呼び `quiet-before.txt` を出力先へ・静かでなければ exit 2）・`-BinDir`（実行体と helper の所在を上書き・run-meta に所在と実行体の SHA-256）・`-RustLogExtra`（既定の RUST_LOG の末尾へ連結・run-meta は連結後の値）
  - 既存の引数・終了コード・出力ファイル名・CSV ヘッダ・RUST_LOG の既定値・SMOKE ゲートの文言は不変。版は 1.1.0
  - 観測可能な完了状態: 既存の呼び方が同じ 3 ファイルを出し、新引数つきで run-meta に quiet_mode／bin_dir／env_RUST_LOG が記録される
  - _Requirements: 2.2, 2.8_
  - _Boundary: invoke-perf-run_
  - _Depends: 5.1_

- [x] 5.3 (P) CPU サンプリング 1 コマンド（invoke-cpu-sample.ps1）
  - `-Probe`（昇格の有無・xperf の実在・5 秒の実採取と停止）→ `available=true|false reason=…` を 1 行・exit 0。`-Start`（サンプリング＋呼出スタック採取の開始・昇格なしは exit 5＝UNAVAILABLE で計測失敗 4 と区別）。`-Stop`（merge→記号解決→テキスト dump）。`-SelfTest`（同梱の dump 断片で `areka.exe!` フレーム ≥1・Probe）
  - 記号はビルド時の環境変数 `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` で付与（`Cargo.toml` 非接触）。代替 backend（wpaexporter）は目標定義ファイルで切替
  - 観測可能な完了状態: 昇格した PowerShell で 1 コマンドの採取→dump.txt が出来て `areka.exe!` フレームを含む。非昇格では exit 5 と reason を返す
  - _Requirements: 2.4, 2.11, 8.6_
  - _Boundary: invoke-cpu-sample_

- [x] 5.4 (P) 台帳の本体（perf-ledger.py: 状態ブロック・追記・読取）
  - 台帳の構造（先頭の状態ブロック＝固定キー行・以後は `## 周 <n> — <ISO 日時>` の追記）とサブコマンド state（JSON）／set-phase／append --from-json／--selftest
  - 自己較正（fixtures-loop/ledger）: 追記→読取の往復・壊れた行の拒否・状態ブロックの書き換えが周の記録を壊さない
  - 観測可能な完了状態: --selftest 緑、append した周が state と台帳ファイルの両方から同じ値で読める
  - _Requirements: 1.3, 6.7, 7.6_
  - _Boundary: perf-ledger_

- [x] 5.5 台帳の判定面（perf-ledger.py: STATUS／FINAL 行・遷移表・goal-check／goal-text・summary）
  - STATUS 行と FINAL 行の文法を定数として持ち、status は 1 行、final は走行固有の 8 桁トークン込みでのみ出す（GOAL_MET／STOPPED reason=）。goal-check が周 0 にトークンを生成して状態ブロックへ書き、目標定義の必須キー・判定スクリプトの版一致・閾値と較正値の一致を確かめる（違えば exit 3）。goal-text はトークンを埋めた /goal 条件文を出力
  - next-phase は相の遷移表の純関数（PREFLIGHT→BASELINE→RANK→SELECT→IMPLEMENT→TEST→REMEASURE→DECIDE→RECORD→RANK／FINAL・TOOLFIX 1 回）。summary は results/summary.md（brief 旧数値との対比表）
  - 自己較正（fixtures-loop/ledger）: 遷移表の全遷移・「文書中の見本行（山括弧）は判定の正規表現に一致しない」・final の run= が状態ブロックのトークンと一致
  - 観測可能な完了状態: --selftest 緑、status が決まった書式の 1 行を出し、見本行は判定の正規表現に一致しない
  - _Requirements: 1.4, 1.9, 5.4, 6.7, 7.6_
  - _Boundary: perf-ledger_
  - _Depends: 5.4_

- [ ] 6. 解析と判定の道具（判定スクリプト・順位表・採否・追随チェック）
- [x] 6.1 (P) 判定スクリプト 0.4.0（判定式は不変・読み口の追加）
  - 集計モードに catch-up の系統別（3 系統とも `target=` フィールドで識別）・各発生の時刻・直前の表示成立点との差・直前 10 秒の成立点数・同時刻の `[tick]` 窓の壁時計と省略数の表を足し、「フレーム駆動の負荷が起床を遅らせる」仮説の成立／不成立を数値で記す
  - `--emit-metrics`（主要指標を `metric=<name> value=<v>` 行で末尾に）。`[tick]`／perf(thread)／perf(process) は任意種（必須種の一覧は不変）。1 行内のフィールド名重複はテストで固定
  - 較正値バナーへ SSP 参考値（アイドル 3.05％・頂 4.64％・2026-08-15・合否に不使用）と「3.0％ は CPU 絶対値・描画方式で正規化しない」の注記
  - fixture: catch-up 系統別（3 系統 × 合格側と不合格側）・`[tick]` 行あり／なし。既存 17 件の合否は不変
  - 観測可能な完了状態: --selftest 緑（既存 17＋追加分）、verdict モードが 0.3.2 と同じ fixture で同じ合否
  - _Requirements: 2.2, 2.9, 2.11, 2.12, 5.1, 5.2, 5.3, 5.5, 5.8, 7.5_
  - _Boundary: judge-perf_

- [x] 6.2 (P) 4 段の順位表（perf-rank.py）
  - [1] プロセス（定常平均／p50／p95／最大・発話中の頂・SSP 参考値を併記＝合否外）、[2] スレッド（perf(thread) の差分から役割別の CPU 秒と占有率・壁時計と CPU の別を見出しに・段③利用可なら dump の TID 別を併記）、[3] 関数（dump の自己時間と包含時間の上位・module!function・スレッド別・サンプル総数と `areka.exe!` 解決率。UNAVAILABLE は見出しに理由だけ・解決率 0 は exit 4）、[4] 相（`[tick]` 行から tick/秒・省略率・心拍率・1 tick 平均・UI スレッド CPU/秒・13 本別）
  - 決定論・固定幅。dump の列はヘッダ行で引き、既知列が無ければ exit 4 と文言
  - 自己較正（fixtures-loop/rank）: 合格側＝既知の順位・不合格側＝未解決 dump／tick 行なし。期待出力と byte 一致
  - 入力行の語彙は 2.2／2.4 の実装で確定した行を fixture に写す
  - 観測可能な完了状態: --selftest 緑、同じ入力から byte 一致の rank.txt
  - _Requirements: 2.1, 2.6, 2.10, 5.4, 6.7_
  - _Boundary: perf-rank_
  - _Depends: 2.2, 2.4_

- [x] 6.3 採否判定（perf-compare.py）
  - A1・A2・B1・B2 の各走行を 6.1 の `--emit-metrics` 経由で集計。ばらつき＝max(|A1−A2|, 床値)、差＝mean(B)−mean(A)。差≤−ばらつき かつ 副指標（⑴ p95 は +5％ 以内・⑵ ⑶ の件数は増えない）が悪化しない→ADOPTED、|差|<ばらつき→NO_DIFF、差≥ばらつき または副指標悪化→WORSE、いずれかの走行が判定不能→MEASURE_FAILED
  - compare.txt（表）と compare.json（verdict と数値）。exit 0／4
  - 自己較正（fixtures-loop/compare）: adopted／no_diff／worse／measure_failed
  - 観測可能な完了状態: --selftest 緑、4 ケースで期待の verdict
  - _Requirements: 1.7, 3.6, 4.6, 6.7_
  - _Boundary: perf-compare_
  - _Depends: 6.1_

- [x] 6.4 (P) 見た目の追随チェック（invoke-followup-checks.ps1＋judge-followup.py）
  - 有界実走（自動終了 ms・RUST_LOG にクリック透過／遷移／表示の debug）で表示成立点を待ってから、PID から窓を列挙して操作を時刻つきで probe.log へ
  - clickthrough（透明点と不透明点で WS_EX_TRANSPARENT の有無＋両方向のトグル適用ログ）・drag（不透明点から +80px ドラッグ→位置変更メッセージと窓書込の新位置 ±2px）・dpi（別 DPI のモニタへ移して戻す→DPI 変更メッセージと k= の変化・2 モニタ混在が無ければ INCONCLUSIVE）・balloon_follow（drag／dpi の前後でバルーンのキャラ窓相対位置 ±2px）
  - judge-followup.py: 各検査 PASS／FAIL／INCONCLUSIVE・総合は全 PASS のみ PASS（INCONCLUSIVE は採用しない）・exit 0／1／2。自己較正 3 ケース（all_pass／clickthrough_fail／dpi_inconclusive）
  - 観測可能な完了状態: 現行の実行体で clickthrough／drag／balloon_follow が PASS（dpi は環境により PASS か INCONCLUSIVE）、--selftest 緑
  - _Requirements: 1.5, 4.7_
  - _Boundary: followup-checks_

- [ ] 7. 入口とループ駆動層（目標定義・1 入口・エージェント定義・1 周のスキル）
- [x] 7.1 目標定義ファイルと /goal 条件文テンプレート
  - 目標定義（goal／target／levels／primary_metric／secondary_metrics／stop／quiet／followup／goal_runtime／sampling の各節・設計 C1 のスキーマ・判定スクリプトの版 0.4.0・停止条件に周数上限 30 を含む）と、/goal へ貼る条件文テンプレート（4,000 字以内・達成と不可能は FINAL 行の字面で判定・見本は山括弧・開発者へ質問しない等の制約）
  - 条件文の語は 5.5 の定数から goal-text で生成（字面の二重管理をしない）
  - 観測可能な完了状態: goal-check が exit 0（6.1 の版 0.4.0 と一致）、goal-text がトークン入りの条件文を出力し、その文字数が 4,000 未満
  - _Requirements: 1.1, 1.5, 1.6, 1.11, 2.7, 5.1_
  - _Boundary: goals_
  - _Depends: 5.5, 6.1_

- [x] 7.2 1 入口の骨格（perf-loop.ps1: 引数・終了コード・RESULT 行・preflight・selftest）
  - サブコマンドの受け口と共通規約: 終了コード 0／1／2／3／4（計測失敗＝MEASURE_FAILED）／5（能力不足＝UNAVAILABLE・停止理由にしない）、標準出力の末尾に必ず `PERF-LOOP RESULT <sub> code=<n> dir=<path>` の 1 行、出力先の配置（`%LOCALAPPDATA%\areka-diag\perf-loop\<goal>\…`）、同じ出力先に `-Resume` で成果物を再利用（冪等）
  - preflight: 昇格・xperf・PDB の有無・判定スクリプトの版一致・Python／PowerShell の版・`CLAUDE_CODE_GOAL_CHECKIN_MINUTES` の実効値（25 分未満は警告）・selftest → preflight.txt と標準出力。段③不可は `function_stage=UNAVAILABLE reason=…` として exit 0 で続行
  - selftest: 5.3／5.4／5.5／6.1／6.2／6.3／6.4 の自己較正を順に呼び、1 つでも赤なら exit 4
  - 観測可能な完了状態: `selftest` が全道具の自己較正を回して緑、`preflight` が preflight.txt と RESULT 行を出し、段③不可の環境でも exit 0 で UNAVAILABLE を報告する
  - _Requirements: 1.2, 2.10, 2.11_
  - _Boundary: perf-loop_
  - _Depends: 5.3, 5.4, 5.5, 6.1, 6.2, 6.3, 6.4, 7.1_

- [ ] 7.3 1 入口の計測サブコマンド（perf-loop.ps1: measure-baseline／rank-run／rank／prepare-ab／measure-ab／compare／followup／final）
  - measure-baseline -Build release|dev（25 分 × 1 本）。rank-run（順位付け 7 分・`wintf::tick`／`areka::perf` を点灯・サンプリング。UNAVAILABLE なら段③を省く）。rank（6.2 → rank.txt）
  - prepare-ab は `CARGO_PROFILE_RELEASE_DEBUG` 付きで release をビルドし実行体・PDB・32bit helper を bin-A へ複製。measure-ab は B をビルドして bin-B へ、A1 B1 A2 B2 を `-AutoQuiet -BinDir` で採り、各走行に quiet-before／after を残す。compare（6.3）。followup（6.4）。final（25 分 × release／dev → `judge-perf.py --mode verdict`）
  - 実走失敗は 1 回だけ再試行、なお失敗なら exit 4。静かでないときは目標定義の回数だけ待って再確認、超えたら exit 2
  - 観測可能な完了状態: rank-run の出力先に run.log／cpu.csv／run-meta.txt／quiet-*.txt（段③利用可なら dump.txt）が揃い rank.txt が出る。measure-ab の出力先に A1／B1／A2／B2 と compare.txt が揃う
  - _Requirements: 1.2, 2.7, 2.10, 5.6_
  - _Boundary: perf-loop_
  - _Depends: 5.1, 5.2, 7.2_

- [x] 7.4 (P) 役割別エージェント定義（perf-measure／perf-analyze／perf-implement／perf-review）
  - 4 本とも frontmatter `model: opus`・tools は最小・本文冒頭で `[agent-model] <自分のモデル名>` を印字（黙って継承しない）
  - perf-analyze は候補カタログ（tick の門・実行器の単スレッド化・文字層レイアウトの変化時のみ化・visual 走査の絞り込み・ポインタ状態の既定値時非書込・カーソル監視の二段周期・ループ ticker 周期・flush 駆動）と順位表の段→候補の対応、担当 spec の稼働確認（spec.json の phase と brief の担当ファイル集合）を規則として持ち、最上位から選び選ばなかった理由を列挙して返す
  - perf-implement は `Cargo.toml` 非接触・破壊的 git 禁止・決定論テストを兄弟ファイルへ・触ったファイル一覧を返す。perf-review は制約一覧（13 本の順序・Z 指令テスト緑・既存行の語彙・前置ガード・1,000 行・兄弟配置・`Cargo.toml`）で APPROVED／REJECTED
  - 観測可能な完了状態: 4 ファイルが存在し、Fable のセッションから Agent で呼ぶと返答冒頭に opus 系の `[agent-model]` 行が出る
  - _Requirements: 1.12, 3.1, 3.3, 8.5_
  - _Boundary: perf-agents_

- [ ] 7.5 1 周の手順スキル（perf-loop-iteration）
  - 入力は goal 名のみ。台帳の state から再開し、遷移表（PREFLIGHT→BASELINE→RANK→SELECT→IMPLEMENT→TEST→REMEASURE→DECIDE→RECORD→…→FINAL・TOOLFIX 1 回）どおりに相を進め、背景コマンドを起動する相で `WAIT_<相>` としてターンを終える。相の境界ごとに台帳を更新して status を印字し、最後の status 行を返答の最後の行に置く
  - SELECT の規則（最上位から・Out of scope／稼働中／既試行は理由を台帳へ）、RECORD の規則（採用は台帳の変更ファイルを選択的に add して 1 周 1 コミット・不採用は restore と新規削除・`add -A` と `reset --hard` は使わない）、大きい変更の 3 条件規則（全テスト緑・追随 PASS・ばらつき超え）を開発者に問わず適用、FINAL（25 分 release／dev→verdict 保存→summary.md→未達なら requirements の改訂欄へ）
  - 重い作業は 7.4 のエージェントへ委ね、返答冒頭の `[agent-model]` が opus 系でなければ台帳に警告。check-in が届いたターンは背景出力の末尾を読んで「待つ」か回収相へ
  - 観測可能な完了状態: スキルを手で 1 回呼ぶと PREFLIGHT 相が回り、台帳に状態ブロックが出来て STATUS 行が返答の最後の行に印字される
  - _Requirements: 1.2, 1.4, 1.8, 1.10, 3.1, 3.6, 5.6, 5.7, 7.6, 8.5_
  - _Boundary: perf-loop-iteration_
  - _Depends: 7.3, 7.4_

- [ ] 8. 登記（測り方・回し方・目標・申し送り）
- [ ] 8.1 (P) README §13〜§17
  - §13 自走ループの回し方（条件文の作り方と貼り方・推奨「Fable で起動」と Opus 5 での起動手順・`CLAUDE_CODE_GOAL_CHECKIN_MINUTES=60`・auto mode・スキル名・目標定義ファイル・台帳の形・停止条件・再開・「昇格した PowerShell から起動すると段③が使える」）／§14 4 段の採り方（コマンド・ビルド指定・RUST_LOG の target・前置ガードの有無・GetThreadTimes の粒度・インライン化の注意）／§15 交互取得と静寂の自動確認／§16 SSP 参考値（採取条件・再採取の配置・拡大率・測定後の削除と記録）／§17 追随チェック。遷移フレームの未特定区間と文字層再構築は合否外として §11 の隣に登記
  - 観測可能な完了状態: README に 5 節があり、§13 の手順どおりに条件文を作ると 7.1 の goal-text の出力と一致する
  - _Requirements: 5.2, 7.1, 7.2, 7.3, 7.4, 8.4_
  - _Boundary: README_

- [x] 8.2 (P) COMPAT の性能目標と他 spec への申し送り
  - `doc/COMPAT_ARCHITECTURE.md` §8 に「areka 裁量の性能目標: release アイドル CPU 3.0％ 未満（1 コア換算・定常平均・狭義）・SSP 参考値は描画方式で正規化しない（2026-08-22 裁定）」
  - test-cage-determinism の brief へ SELF_INITIATED_DEPTH の着地形（スレッド局所・錠の退役可・21 箇所）、present-write-coherence と emo2-conformance-e2e の brief へ tick の門（省略 tick で FrameCount が進まない・`[tick]` 観測行）を申し送り
  - 観測可能な完了状態: COMPAT §8 に目標の段落があり、3 spec の brief に本 spec 名つきの申し送り節がある
  - _Requirements: 5.9, 8.2, 8.3, 8.4_
  - _Boundary: COMPAT, briefs_

- [ ] 9. 自走ループの実施（仕組みを回す・1 周＝SELECT→IMPLEMENT→TEST→REMEASURE→DECIDE→RECORD を単位に検証する）
- [ ] 9.1 ループ起動前の統合確認
  - origin/main を取り込み、設計の file:line と実装の一致を再突合。ワークスペース全テスト緑・`perf-loop.ps1 selftest` 緑・`preflight` を実行して能力（昇格・xperf・PDB・版・check-in 実効値）を台帳へ。goal-text でトークン入りの条件文を出力（4,000 字未満）
  - 観測可能な完了状態: 台帳に PREFLIGHT の capabilities 行と最初の STATUS 行があり、/goal に貼る条件文が得られている
  - _Requirements: 1.6, 1.11, 2.11, 8.1_
  - _Depends: 7.5, 8.1_

- [ ] 9.2 ベースラインと周 1（tick の門の A/B）
  - BASELINE: release 25 分・dev 25 分・順位付け 7 分を別コマンド・別ターンで回し `results/baseline-<date>/` へ判定出力と順位表。周 1 は門の既定 ON（B）対 OFF（A）の交互比較で既定値を決め、台帳に `hypothesis: tick gate default ON` と「周 1 は仕組みの A/B」を記す（順位表からの選択は周 2 以降）
  - ADOPTED なら既定 ON を 1 コミット、NO_DIFF／WORSE なら既定 OFF のまま残す（純関数とテストは残す）
  - 観測可能な完了状態: `results/baseline-<date>/` に両ビルドの verdict と rank.txt、台帳に周 1 の採否と前後の数値・ばらつき、STATUS 行が毎ターン印字されている
  - _Requirements: 1.2, 1.7, 1.8, 2.7, 3.2, 5.4, 7.6_
  - _Depends: 9.1_

- [ ] 9.3 周 2 以降の周回（順位表→候補→採否）を停止条件まで
  - 1 周の単位: SELECT（順位表の最上位から候補＝実行器の単スレッド化〔前提テスト 2 本の字面検査を同じコミットで新しい構築形へ改訂〕・文字層レイアウトの変化時のみ化・visual 走査の絞り込み・ポインタ状態の既定値時非書込・カーソル監視の二段周期・ループ ticker 周期〔SERIKO の最短 interval を README へ・⑴ p95 必見〕・flush 駆動）→IMPLEMENT→TEST（全テスト＋追随チェック）→REMEASURE→DECIDE→RECORD。周ごとに台帳 1 エントリと STATUS 行で検証できる
  - 別 spec の担当ファイルは稼働確認のうえ、稼働中なら触らず報告・非稼働なら変更して brief へ申し送り。大きい変更は 3 条件規則で採否。catch-up の系統別突合の結果を台帳に記し、仮説の成立／不成立を数値で
  - 観測可能な完了状態: 各周が台帳に 1 エントリ・採用は 1 コミット・不採用は差分ゼロ・STATUS 行が毎周印字され、頭打ち 3 周か主指標 3.0％ 未満での採用か周数上限 30 で FINAL へ入る
  - _Requirements: 1.2, 1.4, 1.7, 1.8, 2.9, 3.1, 3.3, 3.6, 4.2, 4.7, 8.5_
  - _Boundary: perf-loop-iteration 駆動＋候補 C17–C19 の該当ファイル（wintf world／visual_sync／pointer systems／clickthrough monitor・areka-emo-text actor・areka-ghost ticker）_
  - _Depends: 9.2_

- [ ] 9.4 最終判定と未達の登記
  - 25 分 release／dev→verdict を `results/final-<date>/` へ、summary.md（brief の旧数値との対比表）。GOAL_MET か STOPPED の FINAL 行（トークン入り）を印字
  - 未達（⑵ または ⑷a）なら requirements.md の改訂欄に残る最大項と引受先を登記。触った場所の改訂欄登記と各 brief の申し送りを最終形へ更新（SELF_INITIATED_DEPTH の着地形・tick 構造・`Cargo.toml` に触れたか）
  - 観測可能な完了状態: FINAL 行が会話に出て、`results/final-<date>/` と summary.md が存在し、未達の場合は改訂欄に登記がある
  - _Requirements: 1.4, 5.3, 5.5, 5.6, 5.7, 7.6, 8.2, 8.3_
  - _Depends: 9.3_

## Implementation Notes

- (2.3) `areka-actor`／`areka-ghost` は wintf に依存せず `Cargo.toml` は非接触（要件 8.6）のため、設計 C14 の「ticker.rs／spawn.rs の生成点で wintf の名簿へ直接登録」は不可能。着地形＝`areka_actor::install_thread_start_hook(fn(&str))`（依存ゼロ・OnceLock・先着勝ち）を `spawn_actor` が新スレッド内で呼び、`areka` bin の `thread_roles.rs` が「名前→役割」（`ticker`→`ticker_dispatcher_kanade`・`loop-ticker`→`ticker_loop`・他→`actor:<name>`）を 1 箇所で宣言して `main()` 冒頭（tracing 初期化直後・`WinApp::new()` 前）で導入する。`areka-ghost` は非接触。`examples/*` は導入しないので実走計測は `areka` 本体で行うこと。
- (2.1/2.3) 名簿は登録解除を持たず、終了済みスレッドも最終 CPU 値つきで残る（同一 TID の再登録は置き換え）。2.4 の報告行は「終了済みスレッドも 1 行出る」前提で設計すること。
- (2.2) `ui_cpu_us` は `GetThreadTimes` 由来で 15,625µs 量子の整数倍（1 秒窓で ±1 量子≒1.6 ポイントの分解能）。順位表は壁時計を主に読み `ui_cpu_us` は目安として扱う。実走の `[tick]` 行には既存の span 文脈 `actor{actor=emo-text}:` が前置される（フィールド名とは重ならない）。
- (全般) `.claude/skills/*/SKILL.md` は CRLF。素の書き換えは全行差分になるので行末を保つこと。areka bin の in-crate テストは `cargo test -p areka --bin areka` で走る。
- (2.4) 初回の実走（debug・15 秒・周期 5 秒）で `unregistered_rest`（名簿外＝bevy タスクプール等）がプロセス CPU の 24〜38% を占め、名簿内は `ui` が約 74%。名簿方式（段②）だけではタスクプールの内訳は出ないので、順位表の段③（WPT サンプリング）が要る。`perf(thread)` の値は累積（読み手 `perf-rank.py` が差分を取る・Rust 側 `delta` が意味論の権威）。
- (3.1) `tick_wake` の期限は「最も早い 1 つ」だけを保持する（設計どおり）。期限到来で回った tick の中で待ち手が**毎回再装填**しないと後の期限が黙って落ちる——3.3／3.5 の結線はこれを前提に書くこと。`WM_PAINT`／`WM_ERASEBKGND`／`WM_TIMER` は未知→FORCE で全走になるので、周 1 の A/B で FORCE が門を無効化していないか `[tick]` 行の skipped で確かめること。`WM_MOUSELEAVE` は `windows::Win32::UI::Controls`、`WM_NCMOUSELEAVE` は `WindowsAndMessaging` にある。
- (3.3) 共有の起床旗に触る／`decide_tick` に到達するテストは、`ecs::world::TICK_WAKE_TEST_LOCK`（唯一の錠・`world/mod.rs`）を毒化耐性つきで取ること。自前の錠を作ると直列化が成立しない（3.1 と 3.3 で錠が 2 本に分裂した実例）。`Skip` を主張するテストは注入経路 `decide_tick_with` を使う。心拍は「省略 30 回→31 回目が Run(Heartbeat)」（実効約 3.87 回/秒）。門 ON・生産者未結線の実走で UI スレッド CPU は 1 秒窓あたり約 45 万µs→約 4 万µs（債務＝3.4/3.5 の結線後に再確認）。`world/mod.rs` は 929 行で余白が小さい——以後ここへ足す場合は doc ブロックを `tick_gate.rs` 側へ寄せるか分割を検討。
- (3.4) 本番経路（`enqueue`／`dispatch_window_message`／`apply_zorder_pair_maintenance`／pointer 投入）が旗を立てるようになったので、**共有の旗の上で「立っていないはず」を主張するテストは書けない**（錠は他の検査からしか守らない）。省略側の主張は `tick_one_frame_with`／`decide_tick_with` の注入口で行う。ZORDER は `apply_zorder_pair_maintenance` の巡の頭で「要求が残っていれば立てる」形（確立系は `areka/src/placement/spawn.rs:642` の chain で維持系の直前）。`tick_dola_animators` は本番未登録で ANIM は当面立たない。本セッションでは `SetCursorPos` が拒否される（headless 条件）ため実カーソル注入は不可＝ポインタ経路は `PostMessage(WM_MOUSEMOVE)` で検証した。ドラッグの実走確認は 6.4 へ。
- (3.5) 「talk 進行中」の判定は時刻起点ではなく**未リビールのグリフの有無**（`reveal_pending`）——起点は一度立つと消えないので、設計 C16 の字面どおり「起点確立」で REARM すると放置時に省略しない。文字 cue ごとの起床は `BalloonLifecycleSink::send`（占有終端の伸長＝cue 到着ごと）の PRESENT が担うため、`sinks` の順序は clocked_text_sink → lifecycle_sink を保つこと（`emo2_boot/mod.rs` に登記）。旗は `tx.send` の**後**に立てる。45 秒実走: 定常 UI スレッド CPU ON 69,079 対 OFF 305,921 µs/秒（約 4.4 分の 1）・判定式⑴ p95 ON 808ms 対 OFF 810ms・放置時省略 116〜118 回/秒。候補メモ: 期限超過かつ抑止中（ドラッグ／滞在／選択肢）は毎フレーム REARM＝省略しない（安全側・解除はポインタ駆動なので `None` でも足りる可能性＝周回で検討）。
- (4) `SELF_INITIATED_DEPTH` の着地形＝`thread_local! Cell<i32>`（`command.rs`）。錠 `lock_self_initiated_for_test` は残置（実呼出 21 箇所／5 ファイル＝command.rs 2・command_batch_tests 5・command_transition_tests 4・window_pos_tests 5・window_pos_transition_tests 5）で退役候補。兄弟テスト 4 ファイルの module doc（`command_batch_tests.rs:25`・`command_transition_tests.rs:28`・`window_pos_tests.rs:40`（テストヘルパ内の注記）・`window_pos_transition_tests.rs:21`）は「プロセス共有」のまま陳腐化＝錠の退役と同じ塊で cage へ申し送る（8.2）。`frame_transition_atomicity_tests` の実本数は 3（設計の 4 は doc 内の `#[test]` 字面を数えた誤り）＝既存群は計 57 本。
- (5.1) `check-quiet.ps1` の判定語は `QUIET`／`NOT_QUIET` に加えて計測失敗時 `MEASURE_FAILED`（exit 4・ファイルは残る・失敗原文は標準出力の `counter_error=` のみ）。理由語 5 種（ok／cpu_mean_over_threshold／heavy_process_present／both／counter_read_failed）。TOML `[quiet]` は**配列を 1 行に保つ**こと（複数行や壊れた値は警告なく既定値へ落ちる）——7.1 の目標定義ファイルはこの制約で書く。PowerShell の変数名は大小無視＝`-SampleSec` パラメータと `$sampleSec` ローカルが衝突する（実装中に踏んだ）。
- (5.4) `perf-ledger.py` は 934 行——**5.5 着手時に分割必須**（自己較正の節（約 200 行）を `perf_ledger_selftest.py` 等へ切り出し、`--selftest` 入口は本体に残す）。状態ブロックは設計の 8 鍵＋`run`・`capabilities`。小数は 2 桁丸め（STATUS 行の `<x.xx>` と同精度＝`compare.json` の 3 桁以上は台帳で落ちる）。`steps.txt` の引数は空白区切りのみ。**穴 1 件を 5.5 で塞ぐ**: 状態の `iteration`／`streak_no_gain`／`best_idle_cpu_pct`／`baseline_idle_cpu_pct` が `-`（`init` が書く正規の空値）の台帳を読む経路で `int()`/`float()` が未捕捉例外（exit 1・生トレース）になり得る（CLI からは到達不能だが 5.5 の status/final が同型を踏む）→ `bad_input`（exit 3）で包むこと。
- (5.3) 本セッションは非昇格のため実採取は未実施＝`fixtures-loop/rank/sample_ok/dump.txt` は xperf dumper 書式（`perf_nt_c.dll` の書式文字列で裏取り・末尾列は推定）の**手書き断片**で、初回の昇格採取（7.2 preflight／9.1）で差し替え、同時に `invoke-cpu-sample.ps1` の `FIXTURE_EXPECT_*` 6 定数（16/8/16/22/2・TID 18332,18420,18512）を更新すること。fixture は `ThreadStartImage!Function` 列にも `areka.exe!` を 16 個仕込んであり、素朴な文字列数えだと 32 になる＝6.2 の `perf-rank.py` は列名行から `Image!Function` 列を引き、同じ 16/8/16/22/2 を再現すること。`-Stop` は非昇格だと exit 1（5 でない）・`no_pdb` の検出は preflight（7.2）の担当。
- (7.4) `.claude/agents/perf-*.md` はセッション開始時に読み込まれる登録簿に載るため、**作成したセッション内からは Agent で呼べない**（本セッションで `perf-measure` を呼ぶと "Agent type not found"）。「Fable から呼ぶと `[agent-model]` が opus 系で出る」の実証は次セッション（9.1 統合確認）で行うこと。定義は頭書の規則（出典指定・`unknown` 退避・前置き禁止）を満たしている。
- (5.2) `invoke-perf-run.ps1 -AutoQuiet` で確かめ直しを使い切った失敗時は既存の `Stop-Run` 作法で出力先が `<leaf>-FAILED` へ退避され、`quiet-before.txt` はその中に残る（run-meta は起動前失敗なので無い）。7.3 の `perf-loop.ps1` が静寂の根拠を読むときは退避先も見ること。`retry_max` は「最初の 1 回の後に確かめ直す回数」（合計 retry_max+1）。
- (5.5) `perf-ledger.py` は本体（定数・読み書き）＋`perf_ledger_goal.py`（status/final/next-phase/goal-check/goal-text/summary）＋`perf_ledger_selftest.py` の 3 ファイル。**7.1 の `goals/draw-load-parity.toml` は `[sampling] backend = "xperf-dumper"` を必ず含める**（`GOAL_SCHEMA` が必須としており、設計 C1 の例にはない＝無いと周 0 の goal-check が exit 3）。**7.5 への申し送り**: `TOOLFIX` の「直前の相」と `toolfix_retry` の消費回数は台帳に置き場が無い（`next-phase --previous` は呼び出し側が渡す）→ 7.5 で `STATE_LATE_KEYS` に `previous_phase`／`toolfix_used` を足し、スキルが `set-phase` で書くこと（要件 1.10＝台帳だけから再開）。`next-phase` は RECORD に `adopted` を受けない（採否の出来事は DECIDE の行・設計 C2 どおり）。goal-text は 1,012 字。summary.md には「brief の数値は Bevy 0.19 更新前」の注記が無い（8.1 の README か summary の定数で補うこと）。
- (6.4) 追随チェックの設計からの差分 2 点（8.1/8.2 で登記）: ① `[transition] kind=monitor` は値が**変化したときだけ**出る（`monitor_systems.rs:340-356`）ので、dpi 検査のモニタ表は `EnumDisplayMonitors`＋`GetDpiForMonitor` で OS から採る（`probe: check=dpi step=monitors`）。② `win_kind` の実値は `char`／`balloon`（`placement/diag.rs:337-344`・`transition_diag.rs:305` の doc 例 `"shell"` は陳腐化）。本セッションは `SetCursorPos`/`SendInput` が ACCESS_DENIED（lasterr=5）で clickthrough／drag／balloon_follow は INCONCLUSIVE 止まり＝**対話デスクトップのセッションで PASS を確認すること**（9.1／周 1）。dpi は実機 2 面（192/144）で PASS・バルーン相対 (-268,-258) 不変。申し送り候補: クリック透過トグル行（`controller.rs:212`）は `window=<Entity>` で hwnd を持たず、判定は観測窓内の両方向本数のみ→hwnd を足すと厳密化できる（wintf・境界外）。`alignment,free` のゴーストでは areka がキャラ窓の `kind=write` を書かないため drag 判定が健全なコードでも FAIL し得る（emo2 は bottom 固定で問題なし）。
- (6.1) catch-up 3 系統は**文言では分けられない**（dispatcher と kanade は同一文言）。識別子は tracing の通常フィールド `target = "…"`（値は引用符つき＝`unquote_field` で外す）。`--emit-metrics` の名: `steady_idle_cpu_mean_pct`／`frame_interval_p95_ms`／`catchup_count`（定常）／`catchup_count_total`／`catchup_dispatcher|kanade|loop_ticker|other`／`alloc_count`／`talk_peak_cpu_pct`／`cpu_p50|p95|max_pct`／`tick_window_count`／`tick_skip_ratio`／`catchup_tick_load_ratio`／`catchup_tick_load_verdict`（成立／不成立／判定不能）。発話区間は kanade のログ標識（`J_TALK_START_EVENTS`/`J_TALK_END_EVENTS`）で定義。`CATCHUP_TICK_LOAD_RATIO_MIN=1.5`／`CATCHUP_SHOW_WINDOW_SEC=10.0` は暫定（合否不使用）。README §16 の SSP 参考値登記は 8.1。
- (8.2) COMPAT §8 末尾に `### areka 裁量の性能目標` を追記・3 brief（cage／pwc／e2e）へ申し送り節。pwc brief の既存行 121-122 は dlp を「W8」と書いたまま（正は W6.9）＝9.4 で本節を更新する際に併せて直す。設計 C1 の TOML 例には `[sampling]` 節が無い（必須）＝9.4 で design.md C1 を追補。`tick_one_frame_with` は私有関数（cage が別ファイルから使うなら可視性調整）。
- (7.1) `tools/perf/goals/draw-load-parity.goal.md` の本文は `GOAL_TEXT_TEMPLATE` の写し（`goal-text` 出力の token を `<token>` に置換）。テンプレートを編集すると黙ってずれる→**7.2 の `selftest` に「goal-text 出力（token 置換）＝ .goal.md の `---` 以降の本文」の一致検査を足す**こと。ヘッダの「1,012 字」も同じ理由で数値依存（検査で覆う）。
- (6.2) `perf-rank.py`（957 行）＋`perf_rank_dump.py`（443 行・段③と共通基盤）。dump は列名行から既知列（TimeStamp／ThreadID／Image!Function）を引き、欠けていれば exit 4 でイベントと列名を名指し。`sample_ok` の 16/8/16/22/2 を `sample_ok_counts` ケースで固定（5.3 との相互較正）。判定スクリプトの較正値（`WARMUP_EXCLUDE_SEC`・CSV ヘッダ・正規表現・`percentile`）は**写し**（出典コメントあり・機械で束縛していない）＝較正値を動かすときは両方を同時に。dump の短い行・空値・`!` 無しは黙って読み飛ばす（`samples_total` は印字する）。共有 scratchpad は揮発物（並走の別実装者が消す）。
- (6.3) `perf-compare.py` の規則（設計が無記述の 3 点を決めて fixture で固定）: 測っていない副指標（`-`）は NA＝採用を止めないが必ず列挙／差なしの帯で副指標だけ悪化→WORSE（安全側）／judge exit 1 は判定不能にしない（集計モードは 1 を返さないので到達不能・`judge-perf.py:715`）。副指標の判定: `_ms`/`_pct` 接尾辞は率（+5%）・それ以外は増減。`compare.json` は台帳の鍵（before/after/delta/noise/secondary/verdict）と同綴りだが**`perf-ledger.py append --from-json` へそのままは渡せない**（ENTRY_KEYS 外の鍵を拒む）＝7.5 の RECORD で 6 鍵を抜き出す。`talk_peak_cpu_pct` を副指標に挙げると要件 5.4（合否に載せない）の外に出る＝本番 TOML は挙げない。
- (7.2) **端末のコードページ（既定 CP932）で子プロセスの UTF-8 出力を復号すると日本語が壊れ、一字比較の検査が偽の MEASURE_FAILED を返す**（goal-text 一致検査で実際に踏んだ）。`perf-loop.common.ps1` の `Invoke-Child` は `ProcessStartInfo` で標準出力/エラーを UTF-8 固定で読む（python 子は `PYTHONIOENCODING=utf-8`・`PYTHONUTF8=1`、pwsh 子は `-Command` 内で自分の OutputEncoding だけ UTF-8）。**子プロセスの出力を `& $exe` で直に捕捉しない**こと。`perf-loop.ps1` 自身の説明行は端末のコードページで出る（RESULT 行は ASCII）＝7.5 のスキルが標準出力を台帳へ回すときは読み側の文字コードを決めておく。preflight は台帳があれば goal-check を呼び、トークン未生成なら作る（7.5 の周 0 は init→preflight/goal-check→goal-text の順）。`function_stage` の reason に `probe_failed`（-Probe 自体が回らなかった）を足している（C8 語彙外・頭書に明記）。

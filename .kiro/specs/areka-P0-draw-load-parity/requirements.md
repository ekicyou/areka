# Requirements Document

## Introduction

areka（ukadoc 互換ベースウェア）は、同一マシン・同一ゴースト（emo2）・同一拡大率（200%）・同一手順の 25 分走行で、アイドル時 CPU が SSP の **3.6 倍**（平均 10.97% 対 3.05%・1 コア換算）、発話中は **4.4 倍**（頂 20.42% 対 4.64%）を消費する（2026-08-15 実測・出典 `brief.md` `## Problem`・`.kiro/steering/roadmap.md` 追記(68)）。デスクトップマスコットは常駐が前提なので、この差はバッテリー・発熱・他アプリへの圧迫として日常的に効く。

先行 spec `areka-P0-recompose-budget`（2026-08-15 完了）は表示 1 コマの適用経路を 22,210µs → 1,240µs（18 分の 1）まで削ったが、**同経路はアイドル CPU の 3.3% しか占めておらず**、1 コマの中央値を 78% 削っても CPU の中央値は 9.3% のまま 0.0 ポイントしか動かなかった（`completed/areka-P0-recompose-budget/remeasure-2026-08-15.md` §5.7）。残る負荷は表示 1 コマの適用経路の**外**にある。同 spec の使い捨て計測は、その主役が wintf のフレーム駆動——ECS の tick が毎秒 120 回・1 回あたり約 578µs で 13 本のスケジュールを全部回し、tick の 98% は表示に変化が無い——であり、上位 2 本（FrameFinalize 182µs・Draw 143µs）で 56% を占めると示した。

同 spec は合格判定式のうち **⑵ 定常状態の進行境界スキップ（catch-up）0 件**（実測 release 17 件・dev 22 件・長時間 25 分で 69 件）と **⑷a release アイドル CPU 3.0% 未満**（実測 10.38%・長時間 11.83%）を未達のまま送り出し、引受先を本 spec と登記した（同 requirements.md Requirement 4 改訂欄 2026-08-15）。本 spec はこの 2 件を引き受ける。

### 本要件が依拠する現況（2026-08-22・現行ツリーで file:line を裏取り済み）

brief の性能数値と機序の読みには、要件として据える前に訂正すべき点がある。

- **⚠ brief の全性能数値は更新前の実測である。** main に Bevy 0.19／Taffy 0.13 への更新（`bf2d7950`・2026-08-19・実行器改稿を含む）が着地した。`try_tick_world` 1 tick 578µs・壁時計 6.85%・FrameFinalize 182µs／Draw 143µs・SSP 比較の areka 側 10.97% は**いずれも更新前（2026-08-15 以前）**の値で、実行器が変わったため傾向すら持ち越せない。本書はこれらを「更新前の参考値」とのみ扱い、現況の確定は Requirement 1 の再計測に委ねる（roadmap 追記(80)⑤・棚卸⑩）。ワークスペースの `Cargo.toml:48-49` は `bevy_ecs = "0.19"` を宣言している（steering `tech.md:22-23` の「0.18.0」は陳腐化。ローカルの `Cargo.lock` は gitignore 対象で権威ではない）。
- **tick は 1 系統であり、120 回/秒は画面の更新周期そのものである。** フレーム駆動は `crates/wintf/src/runtime/tick_bridge.rs:114-134` の vblank 検出スレッド（`DwmFlush` 待ち→全リスナ起床）と、起床ごとに 1 フレームを回す UI スレッドの非同期ループ（同 `:218-236`）の 1 系統だけで、同 `:142-146` が「固定周期ではない。実効のフレーム周期は画面の更新周期である（120Hz の実機なら約 8.3ms）」と明記している。brief 候補 C が疑った「16ms 設定なのに 120 回/秒＝二重起床」は現行ツリーに根拠が無い——wintf に 16ms の定数も Win32 タイマも無く、旧経路 `try_tick_on_vsync`（`crates/wintf/src/ecs/world/mod.rs:580`）は起床カウンタの生産者が撤去済みで常に tick しない（`ecs/world/vsync.rs:17-22` の注記）。したがって候補 C は「二重起床の是正」ではなく「**画面の更新周期に追従する tick を維持するか、表示に変化が無いあいだ別の周期へ落とすか**」という裁定として立て直す。
- **1 tick の中身に早期脱出は無い。** `EcsWorld::try_tick_world`（`ecs/world/mod.rs:488-566`）は、システム未登録の場合を除き、毎 tick 無条件にフレーム番号を進め、13 本のスケジュールを固定順に `try_run_schedule` する（`:548-560`・順序不変の既存テスト `:657`）。「変化が無い tick を見分ける」判断は現在どこにも存在しない。
- **13 本のうち 7 本は多スレッド実行器で走る。** ワークスペース `Cargo.toml:48-56` が `bevy_ecs` の `multi_threaded` を有効化しているため `Schedule::new` の既定実行器は多スレッドであり、`ecs/world/mod.rs:104-160` が `SingleThreadedExecutor` へ固定しているのは UISetup／GraphicsSetup／PreRenderSurface／RenderSurface／Composition／CommitComposition の 6 本だけ（同 `:117,135,141,146,151,156`）。Input／Update／PreLayout／Layout／PostLayout／Draw／FrameFinalize の 7 本は tick のたびにタスクプールの scope を開く多スレッド実行器のまま（`bevy_ecs-0.19.1` `executor/multi_threaded.rs:274`）。この固定費の大きさは Requirement 1 の内訳計測で確定する（更新前の実測には内訳が無い）。
- **クリック透過の評価は「毎フレーム評価される」前提の上に乗っている。** `crates/wintf/src/runtime/mod.rs:230-236` は「(b) VSync tick 毎の再評価（静止カーソルでも表示更新＝αマスク変化に追随／R2.4、`JustEnded` 再収束／R5.2）」を理由として記し、vblank ごとに評価ループを起こす中継を `:296-330` に持つ。評価ループ（`crates/wintf/src/ecs/clickthrough/controller.rs:421-457`）はこの vblank 起床とカーソル監視（`clickthrough/monitor.rs:34`・12ms 周期）の二重起床で動く。tick の周期や中身を間引くなら、この前提の調停が要件として要る。
- **catch-up は ECS の tick ではなく進行のティッカーが出す。** 判定式⑵ が数える文言 `ticker catch-up: skipped multiple boundaries, firing once`／`loop ticker catch-up: …` の発行元は `crates/areka-ghost/src/ticker.rs:205,225,307`（dispatcher／kanade／SERIKO ループの 3 系統）で、周期は系統ごとに異なる——dispatcher `base_interval = 50ms`（同 `:61`）・kanade `kanade_interval = 1000ms`（同 `:62`）・SERIKO ループ `16ms`（同 `:262`）。brief の「`ticker.rs` の設定は 16ms」はループ ticker にのみ当たる。ティッカーは UI スレッドではなく自前のアクタースレッドで `recv_timeout` により起きる（同 `:194,297`）ので、catch-up は「ティッカースレッドが 1 周期以上遅く起きた」ことを意味し、フレーム駆動の負荷は CPU 競合による起床遅延としてしか効かない（間接）。「フレーム駆動の負荷がティッカーの境界跨ぎを増やす」という因果は**仮説**であり、Requirement 3 で実測確定する。ループ ticker の 16ms は Windows 既定のタイマ分解能（約 15.6ms）に近く、起床遅れだけで 2 境界を跨ぎ得る。
- **一括 flush の前提は atom 着地後の実形である。** 窓書込は同一窓のジオメトリ指令が積む時点で合流し（`crates/wintf/src/ecs/window/command.rs:514-560`）、1 バッチで一括適用され（同 `:349-440`・`SetWindowPosCommand::flush` は `:723`）、Z 専用指令は合流の対象外で順序・結果が不変である（brief 追記(71)⑶・(74)⑷・(78)⑺）。brief 追記(74) の「`flush` は `:425-505`」は陳腐化（現行 `:723`）。
- **観測の道具は揃っている。** 既定 OFF の観測チャネル `wintf::transition`（`crates/wintf/src/ecs/window/transition_diag.rs:54`・前置ガード `:622` で既定水準の費用 0）、`perf(apply_show)` 段階別計時行（`crates/areka-emo-present/src/presenter/timing.rs:56,221`・末尾 `frame` 込み 16 フィールド）、採取ランナー `tools/perf/invoke-perf-run.ps1`（短時間 7 分＝420,000ms／長時間 25 分＝1,500,000ms・`-ConfirmQuiet` 必須）、判定スクリプト `tools/perf/judge-perf.py`（0.3.2・`IDLE_CPU_MAX_RELEASE_PCT = 3.0`・`WARMUP_EXCLUDE_SEC = 60.0`・`LONG_RUN_MIN_SPAN_SEC = 1200`・自己較正 fixture 17 件）。
- **SSP 側の値は 2026-08-15 の実測（アイドル 3.05%・発話の頂 4.64%）を参考値として使い、SSP の描き方（200% を実描画しているか引き伸ばしか）は調べない**（2026-08-22 開発者裁定＝議題 2。「CPU が 3% 台であること以外は分からず、調べようがない。目標設定のみ」）。比較用の配置 `C:\wintools\ssp\ghost\emo2-perf` は 2026-08-22 時点で無く、再配置も要件としない。

### ウェーブ配置と同居裁定

本 spec は **W6.9** で `areka-P0-test-cage-determinism`（cage）と 2 本並走する（2026-08-22 開発者指示＝並行数最大化・roadmap 追記(81)）。「M1 完成を妨げない」位置づけと「大改造が必要なら無理に治さなくて良い」裁定は不変。同居裁定: **`crates/wintf/src/ecs/window/command.rs` は丸ごと本 spec 所有・cage は非接触**。`SELF_INITIATED_DEPTH`（同 `:49`・プロセス共有の `AtomicI32` だが意味論はスレッド局所＝テスト間の状態汚染源）のスレッド局所化も本 spec が実施し、着地形を cage へ申し送る（見送る場合は cage へ即報告＝差し戻し先は cage）。本 spec は **`Cargo.toml` 非接触**を保つ（cage の共有 leaf crate が dev-deps へ 1 行足す見込み＝衝突ゼロ条件）。

## Boundary Context

- **In scope**:
  - フレーム駆動の周期と、表示に変化が無いときの早期脱出（wintf のフレーム駆動＝`runtime`／`ecs::world`・一括 flush の駆動を含む）
  - 上位スケジュール（更新前の実測では FrameFinalize／Draw）の内訳解析と、変化が無いときの是正
  - 見た目の追随（クリック透過・αマスク・ドラッグ・DPI 追随・バルーン追従・Z 順）を劣化させないことの固定
  - 目標（CPU の絶対値）と SSP 参考値の登記（測り方の記録を含む）
  - `command.rs` 1 ファイルの所有（`SELF_INITIATED_DEPTH` のスレッド局所化を含む）
  - **調査の範囲は制限しない**（2026-08-22 開発者裁定＝議題 1）。負荷の所在を追うための計測・読解は wintf の外（areka 側の毎フレーム処理・tick 外の周期処理・タスクプールなど）へ自由に広げてよい
  - **是正の範囲**: 表示に変化が無いときに無駄に動いている処理は、**担当する生存 spec が無い場所であれば本 spec で直してよい**（同裁定。本 spec 以外にコード修正を伴う spec は現在動いていない）。直した場所は requirements の改訂欄に登記する。将来の spec が予定しているファイル（`present-write-coherence`＝`presenter/show.rs`・`mount.rs`／`balloon-offset-dpi`＝`placement/follow` 系・`windowposition.rs`・`persist.rs`／`test-cage-determinism`＝テストハーネス）に触った場合は、その spec の brief へ申し送る
- **Out of scope**:
  - 表示 1 コマの適用経路（`crates/areka-emo-present/src/presenter/show.rs` の `apply_show`）——`recompose-budget` が 18 分の 1 まで削り CPU の 3.3% しか占めない。**ここを更に削っても効かない**。可視化と窓書込の提示順序は `areka-P0-present-write-coherence`（W6.95）所有
  - 合成アルゴリズム本体（`build_plan`／`blit::execute`）
  - メモリ常駐量の削減（SSP の 3.0 倍だがリークではなく設計上の常駐量）・スレッド数の削減（2.6 倍だが CPU との因果未確認）
  - 発話（talk）そのものの処理コスト（山の高さは発話の実装に依存）。ただし本 spec の実測で切り分ける
  - DPI 遷移フレームの窓書込所要（窓書込呼出の内側・1 遷移 143,231〜231,910µs）——遷移フレームは「定常状態の CPU」とは別の山であり、平均に混ぜない。遷移中の「絵が先・窓が後」は `present-write-coherence` 所有
  - SSP との完全一致、および **SSP の描画方式（実描画か引き伸ばしか）の調査**（2026-08-22 開発者裁定＝調べようがない。目標は画素数で正規化せず CPU の絶対値で置く）
  - テストハーネスの一本化・テスト間の状態汚染の全面硬化（cage 所有。本 spec は `command.rs` 側の 1 行のみ）
- **Adjacent expectations**:
  - `areka-P0-recompose-budget`（完了）: 測定基盤 `tools/perf/`（採取ランナー・判定スクリプト 0.3.2・fixture 17 件）と判定式⑴〜⑷b・静寂確認の関門（同 R2.7）・「実時間閾値はテストコードへ持ち込まず判定スクリプト側の較正値とする」（同 R4.5／R6.2）をそのまま継承する
  - `areka-P0-dpi-transition-atomicity`（完了）: 一括 flush の実形（合流・1 バッチ適用・Z 指令不合流・観測チャネル `wintf::transition`・perf 行末尾 `frame`）を前提とし、設計前に origin/main の実形へ rebase する。同 spec の決定論 8 遷移（同一フレーム・窓ごと 1 回・連鎖 1 回）は本 spec 着地後も PASS のままであること。未特定の 47.5% 区間（窓書込呼出の内側で自前のウィンドウプロシージャが 1 行も走らない区間）と文字層再構築（遷移フレーム内 8,518〜82,416µs）は申し送りとして受けるが、遷移フレームの山であり本 spec の合否には載せない
  - `areka-P0-test-cage-determinism`（W6.9 並走）: `command.rs` は本 spec 所有。`SELF_INITIATED_DEPTH` の着地形（または見送り）を本 spec から cage へ申し送る。cage は錠 `lock_self_initiated_for_test()` の退役だけを rebase で受ける
  - `areka-P0-present-write-coherence`（W6.95）: 本 spec が着地させた tick の実形の上で規模を見積もる。本 spec が tick 構造を変えた場合は申し送る
  - `areka-P0-emo2-conformance-e2e`（W7）・`ghost-window-zorder`（完了・申し送り消化不能）: Z 指令の適用順と結果が現に決まる場所は一括 flush であり本 spec の In-scope にある。本 spec は Z 順の不変条件を壊さない責務を負う
  - `collision-dpi-hittest`（完了）: クリック透過の評価タイミングを共有する。「VSync tick 毎の再評価」（R2.4）の前提を本 spec が調停する

## Requirements

### Requirement 1: 更新後ベースラインの再計測と負荷の所在の確定

**Objective:** As a 開発者, I want Bevy 0.19 更新後の現行ツリーでフレーム駆動の負荷を同一手順で測り直し、負荷の所在を数値で確定したい, so that 更新前の実測に基づく仮説の上で是正に着手しない

#### Acceptance Criteria

1. When 本 spec が是正に着手する前に, the 開発プロセス shall origin/main の現行ツリー（Bevy 0.19 更新および `dpi-transition-atomicity` の着地を含む）を release／dev 両ビルドで、先行 spec と同一手順（採取ランナー・判定スクリプト・同一ゴースト emo2・同一拡大率）により採取し、ベースラインとして spec ディレクトリへ記録する
2. When ベースラインを採取する, the 計測 shall tick 1 回あたりの所要、tick 回数/秒、スケジュールごとの所要の内訳（13 本すべて）、表示 1 コマの適用回数/秒、クリック透過評価の回数/秒を同じ走行から得て、brief の更新前実測（tick 578µs・FrameFinalize 182µs・Draw 143µs・壁時計 6.85%）との対比表を作る
3. When tick 所要を記録する, the 計測 shall 壁時計（経過時間）と CPU 時間を区別して記録し、GPU 待ちなどの待ち時間が混入する量を「壁時計」と明記する（brief の注記: UI スレッドの実測 CPU 約 3.1% に対し壁時計占有 6.85% ＝ 半分程度は待ちの可能性）
4. When ベースラインを採取する, the 計測 shall 「表示に変化が無い tick」の割合——表示 1 コマの適用・入力・アニメ境界の跨ぎ・窓ジオメトリ変更・DPI 変更・Z 順変更のいずれも無い tick の本数／全 tick 本数（Requirement 4.1 の「表示に変化が無い」と同じ定義）——を走行ごとに記録する
5. When 実機計測を開始する, the 開発プロセス shall 開発者へセッションを渡し、測定マシンが静寂状態（並行開発セッション等の他負荷がないこと）であることの確認を得てから実走を開始する（先行 spec R2.7 の関門を継承。判定に人の目視を使わない原則は不変）
6. The 計測 shall 2 分以下の窓を採用しない（同一条件で 5.37% と 18.57% に振れた実測あり）。是正のたびの測り直しは短時間水準（7 分）、ベースラインと最終判定は長時間水準（25 分）とする
7. When 2 つの形（是正前／是正後・ビルド設定の別）を比較する, the 計測 shall 同一セッション内の交互取得で比較し、一括順の前後比較を結論の根拠にしない（一括順では「O3 が 17% 速い」→交互で「33% 遅い」へ反転した実測あり）
8. If 更新後のベースラインで負荷の最大項がフレーム駆動（tick）以外にある, the 開発プロセス shall 最大項の所在を記録したうえで是正を続行し（調査の範囲は制限しない・担当する生存 spec が無い場所は本 spec で直してよい＝2026-08-22 開発者裁定）、最大項が Out of scope に明記した項目（合成アルゴリズム本体・発話そのものの処理コスト・メモリ常駐量・遷移フレームの窓書込所要）に当たる場合に限り開発者へ再裁定を求める

### Requirement 2: 目標の置き方の確定

**Objective:** As a 開発者, I want 「SSP 同等圏」を CPU の絶対値で置き、その根拠と参考値を文書に固定したい, so that 比べ方の議論で目標が動かず、是正の合否が一意に決まる

#### Acceptance Criteria

1. The 開発プロセス shall 目標を **CPU の絶対値**で置く——release ビルドでゴースト放置時のアイドル CPU（1 コア換算・定常状態の平均）が **3.0% 未満**（先行 spec 4.4 の較正値 `IDLE_CPU_MAX_RELEASE_PCT = 3.0`・狭義の未満比較を継承）。画素数や描画方式の差で正規化した目標は置かない（2026-08-22 開発者裁定: SSP は GDI 描画・areka は D2D 描画であり、areka が負けるわけにはいかない）
2. The 開発プロセス shall SSP の描画方式（200% を実描画しているか、100% で描いて引き伸ばしているか）を調査の対象にせず、目標の根拠にも使わない（同裁定: SSP について分かっているのは CPU が 3% 台であることだけで、調べようがない）
3. The 開発プロセス shall SSP 側の参考値として 2026-08-15 の実測（アイドル 3.05%・発話の頂 4.64%・同一マシン・emo2・200%・25 分）を判定スクリプトの較正値の注記と `tools/perf/README.md` に登記し、SSP 側の再採取を要件としない（再採取する場合は同一手順で行い、配置したゴーストを測定後に削除して記録する）
4. The 開発プロセス shall 目標（CPU 絶対値 3.0% 未満）とその根拠を `doc/COMPAT_ARCHITECTURE.md` へ「areka 裁量の性能目標」として登記する（互換台帳にフレームレート／アイドル CPU の SSP 互換規定は現状存在しない）

### Requirement 3: 進行境界スキップ（catch-up）の出どころの確定

**Objective:** As a 開発者, I want 定常状態で残る catch-up 件数がどこから来るかを実測で確定したい, so that 駆動の間引きが件数を増やす向きに働く可能性を排除してから手段を選ぶ

#### Acceptance Criteria

1. When ベースラインを採取する, the 計測 shall catch-up の発生を発行元 3 系統（dispatcher／kanade／SERIKO ループ）別に数え、各発生の時刻と同時刻の状況（発話再生中か・複数面の同時更新か・DPI 遷移中か・表示 1 コマの適用直後か）を突合できる形で記録する
2. When catch-up の出どころを特定する, the 計測 shall 「フレーム駆動の負荷が CPU 競合でティッカースレッドの起床を遅らせ、境界跨ぎが増える」という因果（ティッカーは UI スレッドとは別スレッドで動くので直接の閉塞ではない）を、tick 所要の分布と catch-up 発生時刻の重なり、およびティッカーの起床遅延の分布で検証し、成立・不成立を数値で記す
3. If catch-up の主因がフレーム駆動の外（発話の実装・SHIORI 応答待ち・ティッカー自身の周期設計など）にある, the 開発プロセス shall 是正対象を本 spec の境界へ広げず、引受先の実在を確認した上で申し送り、判定式⑵ を「本 spec の手段で達し得る範囲」として開発者へ再提示する
4. When 駆動の間引きを実装する, the 計測 shall 間引き前後で catch-up 件数が増えていないことを短時間水準の交互比較で確かめる（判定式⑵ は進行側のログを数えるので、間引きは件数を増やす向きにも働きうる）

### Requirement 4: 表示に変化が無いときにフレーム駆動が仕事をしない

**Objective:** As a ユーザー, I want ゴーストを放置しているあいだ areka が CPU をほとんど使わないでほしい, so that 常駐させてもバッテリー・発熱・他アプリの邪魔にならない

#### Acceptance Criteria

1. While 表示に変化が無い（表示 1 コマの適用・入力・アニメ境界の跨ぎ・窓ジオメトリ変更・DPI 変更・Z 順変更のいずれも無い）, the フレーム駆動 shall 13 本のスケジュールを全部回す費用を払わず、tick 1 回あたりの所要を変化が有る tick より桁で小さくする（目標値は Requirement 6 の判定式で置く）
2. When 表示に変化が生じた（上記のいずれかが起きた）, the フレーム駆動 shall その変化を次の画面更新までに反映し、変化が無いときの省略が反映の遅れとして見えないようにする（遅れの上限は 1 画面更新周期＝120Hz 実機なら約 8.3ms）
3. When 変化の有無を判定する, the フレーム駆動 shall 判定の入力（何をもって「変化あり」とするか）を列挙して文書化し、判定漏れ（変化があるのに省略する）を優先して避ける——疑わしいときは回す
4. The フレーム駆動 shall 13 本のスケジュールの実行順序と、スケジュール間・スケジュール内の既存の順序不変条件を、変化が有る tick において変えない
5. When 一括 flush の駆動・間引き・順序に手を入れる, the フレーム駆動 shall Z 専用指令の適用順と結果（合流の対象外・畳めない指令は同一窓の仕切り）を変えず、既存の不合流テスト群が緑のままであること
6. The 開発プロセス shall 「ウィンドウプロシージャ側が Z 指令を積まない」という現状の前提（コードが強制していない）を文書化し、フレーム駆動の改変がこの前提に依存する場合は依存を明記する（brief 追記(78)⑺）
7. If 「SSP 同等圏」への到達が tick 構造の大改造（スケジュール構成や駆動モデルの作り直し）を要する, the 開発プロセス shall 実装に入らず、見積りと候補を開発者へ提示して裁定を得る（開発者裁定 2026-08-22「大改造が必要なら無理に治さなくて良い」・M1 完成を妨げない位置づけ）
8. While 表示に変化が無い, the フレーム駆動 shall 既定で観測の費用を払わない（新設する観測は既定 OFF とし、有効化されていないときは組立も計時も行わない）

### Requirement 5: 見た目の追随を劣化させない

**Objective:** As a ユーザー, I want 省電力化のあとも、クリックの通り抜け・ドラッグ・DPI 切替・バルーンの追従・窓の前後関係が今までどおり即座に反応してほしい, so that 軽くなった代わりに反応が鈍くなったと感じない

#### Acceptance Criteria

1. When 表示内容が変わって透明画素の分布（αマスク）が変わる, the クリック透過 shall 変化した画面更新の次の画面更新までに新しいマスクで判定する（現行の「VSync tick 毎の再評価」と同等。静止カーソルでも追随する）
2. When カーソルが動く, the クリック透過 shall 現行と同じ周期（カーソル監視の周期）で再評価し、フレーム駆動の間引きに引きずられて遅れない
3. While 窓をドラッグしている, the フレーム駆動 shall 変化ありとして毎画面更新で回り、窓とバルーンの追従が現行と同じ滑らかさを保つ
4. When DPI 遷移が起きる, the フレーム駆動 shall 変化ありとして回り、`dpi-transition-atomicity` が固定した決定論 8 遷移（同一フレーム・窓ごと 1 回・連鎖 1 回・随伴の同一フレーム性）がすべて PASS のままであること
5. When バルーンの表示・非表示・位置追従・Z 順の維持が要求される, the フレーム駆動 shall 変化ありとして回り、`ghost-window-zorder`／`scope-chain-gap`／`windowposition-limit` が固定した既存テストがすべて緑のままであること
6. When 発話（talk）が再生中である, the フレーム駆動 shall コマ境界・タイプライタ進行・表情の切替を現行と同じタイミングで反映する（判定式⑴ コマ適用間隔の p95 が退行しない）
7. When 是正後の実機サインオフを行う, the 開発プロセス shall クリック透過（透明画素上のクリックが別プロセスへ通り抜ける／不透明画素上では通らない）・ドラッグ追従・DPI 切替・バルーン追従を有界の自動終了付き実走とログ照合で確認し、目視に頼らない

### Requirement 6: 合格判定（機械判定）

**Objective:** As a 開発者, I want 是正の効果を先行 spec と同じ判定スクリプトで機械判定したい, so that 合否に人の目視や主観が要らない

#### Acceptance Criteria

1. The 判定スクリプト shall 先行 spec の判定式をそのまま適用する: ⑴ コマ適用間隔の p95 が指定間隔＋許容率以内（退行しない）⑵ 定常状態の進行境界スキップ（catch-up）0 件 ⑶ 定常状態の表示用バッファ新規確保 0 件（退行しない）⑷a release アイドル CPU が目標値未満 ⑷b 20 分超で単調上昇しない
2. The 判定スクリプト shall ⑴〜⑶ を dev・release 両ビルドに、⑷a・⑷b を release に適用する
3. The areka 実行体 shall release ビルドでゴースト放置時のアイドル CPU（1 コア換算・定常状態の平均）を 3.0% 未満に収める（Requirement 2.1）
4. The 開発プロセス shall 発話中の CPU の頂を SSP 参考値の頂（4.64%・2026-08-15 実測）と並べて記録し、合否には載せない（発話そのものの処理コストは Out of scope）。ただし頂が参考値を大きく超える場合は機序を記録する
5. The areka 実行体 shall メモリリークと負荷の単調上昇を再発させない（Private メモリ・ハンドル数・スレッド数が 25 分走行で増加傾向を示さず、⑷b の収束判定を満たす）
6. When 最終判定を行う, the 開発プロセス shall 長時間水準（25 分）で release／dev を採取し、判定スクリプトの出力を spec ディレクトリへ保存する
7. If 判定式⑵ または ⑷a が本 spec の手段でも未達のまま残る, the 開発プロセス shall 未達を本 spec の requirements.md に改訂欄として登記し、引受先の実在を確認した上で申し送る（「未達が spec の内側から見えない」形を繰り返さない）
8. When 較正値（`WARMUP_EXCLUDE_SEC` ほか）を見直す, the 判定スクリプト shall 見直しの根拠（容量 3 で暖機が 4 倍に伸びた実測など）を較正値の注記に残し、自己較正 fixture で既知ケースの合否が変わらないことを確かめる

### Requirement 7: 判断分岐の決定論テスト

**Objective:** As a 開発者, I want 「変化の有無の判定」「回すスケジュールの選択」「周期の切替」といった判断分岐を実機なしで全網羅したい, so that 実機でしか分からない領域を最小にして回帰を固定できる

#### Acceptance Criteria

1. The テスト shall 変化の有無の判定について、入力の全組合せ（表示 1 コマ適用・入力・アニメ境界・窓ジオメトリ・DPI・Z 順・ドラッグ中の各有無）に対する「回す／省略する」の結果を決定論テストで固定する
2. The テスト shall 省略した tick の直後に変化が生じたとき、次の tick で反映されること（省略が反映の遅れとして残らないこと）を決定論テストで固定する
3. The テスト shall 13 本のスケジュールの実行順序の不変条件（既存テスト）を維持し、省略経路でも順序が入れ替わらないことを固定する
4. The テスト shall 一括 flush の Z 指令不合流・仕切り・順序不変の既存テスト群を維持し、駆動の変更で 1 本も赤にしない
5. The テスト shall 実時間の閾値を合否条件に使わない（実時間・CPU の閾値は判定スクリプト側の較正値に置く）
6. When `SELF_INITIATED_DEPTH` をスレッド局所化する, the テスト shall 並列実行でテスト間の状態汚染が起きないこと（同カウンタを読む／上げるテストが互いに影響しない）を固定し、錠 `lock_self_initiated_for_test()` の退役可否を cage へ申し送る
7. If 是正の実装に失敗経路（判定の入力が取れない・観測が組めない等）がある, the フレーム駆動 shall 黙って省略せず、ログを残して安全側（回す）へ倒す
8. The テスト shall 新規・既存とも 1 ファイル 1,000 行以下の目安と、テストファイルを本番ファイルの兄弟として置く接続規約に従う

### Requirement 8: 測定手順の常設化

**Objective:** As a 開発者, I want SSP との同一手順比較とフレーム駆動の内訳計測を、次の誰かが同じ結果を再現できる形で登記したい, so that 測り方の違いが結論を反転させない

#### Acceptance Criteria

1. The 開発プロセス shall SSP 参考値（2026-08-15 実測・アイドル 3.05%・頂 4.64%）とその採取条件（同一マシン・emo2・200%・25 分・採取間隔）、および再採取する場合の手順（配置場所・拡大率の設定・測定後の削除）を `tools/perf/README.md` に登記する（現状 README §11 は「SSP の採取方法が未記録」と明記している）
2. When SSP 側で再採取を行った場合, the 開発プロセス shall 配置したゴーストを削除し、削除したことを記録に残す
3. When フレーム駆動の内訳（tick 所要・スケジュール別所要・変化なし tick の割合）を新たな計測行として出す, the 計測 shall 1 行に同じフィールド名を 2 度出さず、既存の perf 行・遷移観測行のフィールド名・順序・文言を動かさず、追加は末尾に限る（判定スクリプトの `parse_fields` が後勝ちで上書きするため）
4. The 計測 shall 新設する観測を既定 OFF とし、有効化の方法（`RUST_LOG` の target 指定）と費用 0 の前置ガードの有無を README に記す
5. When 判定スクリプトへ判定式・較正値を足す, the 判定スクリプト shall 自己較正 fixture に既知ケース（合格・不合格の両側）を足し、`--selftest` で検証できるようにする
6. The 開発プロセス shall ベースライン・再測・最終判定の各結果を日付付きの文書として spec ディレクトリへ残し、更新前の brief 数値との対比を 1 表にまとめる

### Requirement 9: 境界・申し送りの登記

**Objective:** As a 開発者, I want 本 spec が触らないもの・受けたもの・渡すものを文書上で一意に辿れるようにしたい, so that 並走する spec や後続の spec が前提を取り違えない

#### Acceptance Criteria

1. When 設計に入る前に, the 開発プロセス shall origin/main の実形（一括 flush・観測チャネル・perf 行末尾 `frame`・Bevy 0.19）へ rebase し、本書の file:line を再突合する
2. The 開発プロセス shall `SELF_INITIATED_DEPTH` のスレッド局所化の着地形（実施した形、または見送りの理由）を cage へ申し送る
3. The 開発プロセス shall 本 spec が tick の周期・構造に加えた変更を `present-write-coherence` と `emo2-conformance-e2e` へ申し送る
4. The 開発プロセス shall 遷移フレームの未特定区間（47.5%）と文字層再構築の所要を、本 spec の合否に載せない申し送り事項として登記し、憶測で埋めない
5. If 本 spec の途中で別 spec の担当ファイルへの変更が必要になった, the 開発プロセス shall 担当 spec が現に動いているかを確認し、動いていれば即報告して勝手に触らず、動いていない（brief のみ・または完了済み）なら変更したうえでその spec の brief へ申し送る（2026-08-22 開発者裁定＝本 spec 以外にコード修正を伴う spec は動いていないため、調査も是正も担当者不在の場所へ広げてよい）
6. The 開発プロセス shall `Cargo.toml` に触れない（cage の共有 leaf crate との衝突ゼロ条件）。触れる必要が生じたときは cage と着手順を調整する

## 改訂欄

- **2026-08-22 要件ディスカッション 議題 1（境界）**: 開発者裁定「本 spec 以外にコード修正を伴う spec は動いていない。調査は際限なく広げてよい」。反映先＝Boundary Context の In scope（調査無制限・担当者不在の場所は是正可・将来 spec 予定ファイルは申し送り）・Requirement 1.8（最大項が tick 外でも続行）・Requirement 9.5（動いていない spec の担当ファイルは変更＋申し送り）。ギャップ分析 D-4 はこれで解決。
- **2026-08-22 要件ディスカッション 議題 2（目標の置き方）**: 開発者裁定「SSP の描き方は調べなくて良い（CPU が 3% 台であること以外は分からず、調べようがない）。目標設定のみ。SSP は GDI 描画なのだから D2D 描画の areka が負けるわけにはいかない」。反映先＝Requirement 2 を「CPU 絶対値 3.0% 未満・SSP 描画方式は調べない・SSP 再採取は要件外」へ全面書き換え、Requirement 6.3〜6.5 を絶対値 1 本＋頂の記録へ集約（旧 6.4 の画素あたり案を削除・以降を繰り上げ）、Requirement 8.1〜8.2 を参考値と手順の登記へ縮小、Introduction・Boundary Context を同旨に更新。ギャップ分析 D-10 と R-5 はこれで解決。

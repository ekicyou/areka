# Implementation Plan

- [x] 1. Foundation: 依存昇格とモジュール骨格の準備
  - `crates/areka/Cargo.toml` へ `areka-seriko`／`areka-emo-present`（dev→通常昇格）／`areka-emo-text`／`areka-sakura`／`areka-actor`／`dola` を workspace path 依存として追加する（外部 crates.io 依存は追加しない）
  - `crates/areka/src/emo2_boot/` モジュール骨格（`mod.rs`・`target_map.rs`・`adapter.rs`・`talk_clock.rs`・`assets.rs`・`frame.rs` の空ファイル）と `BootWiringError`（thiserror）の空バリアント列挙を作成する
  - 観測可能な完了条件: `cargo build --workspace` が新規モジュール群（未実装スタブ含む）を含めて成功する
  - _Requirements: 10.4, 10.5, 10.8_

- [ ] 2. Core: 孤立した純粋部品・アダプタ部品の実装
- [x] 2.1 (P) target_map（scope→表示対象写像の正本）を実装
  - `shell_target`/`balloon_target`（DD-3 の採番規約 `2*scope`/`2*scope+1`）と `scope_of`（`ActorKey` の数値 parse・非数値は None）を実装
  - 単体テスト: shell/balloon 採番の互いに素性・`scope_of("0")`＝Some(0)・非数値（例 `"側"`）＝None
  - 観測可能な完了条件: 上記単体テストが全て green で通る
  - _Requirements: 1.4, 3.5_
  - _Boundary: target_map_

- [x] 2.2 (P) talk_clock（TalkClock＋ClockedTextSink）を実装
  - `TalkClock::new/observe_cue/talk_time`（単調 max epoch 推定・クロック注入可・負値 clamp・epoch 未確立は None）と `ClockedTextSink<T: TextSink + Clone>`（`emit` で `observe_cue` 後に内側へ透過転送）を実装
  - 単体テスト: 固定クロック注入で単調 max リベース（新 talk で前方跳躍）・epoch None→talk_time None・負値 0.0 clamp
  - 観測可能な完了条件: 上記単体テストが全て green で通り、新 talk 到着時に epoch が前方へリベースされることを確認できる
  - _Requirements: 2.2, 2.3_
  - _Boundary: talk_clock_

- [ ] 2.3 (P) default_bind_ids（shell descript KV からの static bindset 抽出）を実装
  - `sakura.bindgroup{N}.default`==1 の N を抽出する純関数を実装（DD-8・ukadoc 正典）
  - 単体テスト: emo2 相当 KV サンプルから `[1100,1207,1302,1500,1800]` を抽出・`default` が 1 以外や無関係キーは非抽出
  - 観測可能な完了条件: 上記単体テストが期待どおりの抽出結果を assert して green で通る
  - _Requirements: 1.1_
  - _Boundary: assets_

- [ ] 2.4 map_display_command（DisplayCommand→PresentCommand 純変換）を実装
  - `Show`→shell target へそのまま／`Hide`→shell target 非表示／`ShowBalloon`→balloon target・`BindSet::default()` 付与／`HideBalloon`→balloon target 非表示、の 4 写像を純関数として実装（DD-5・target_map を利用）。非数値 scope は None を返す
  - 単体テスト: 4 写像すべての全値比較（surface id は非改変転写を含む）＋非数値 scope で None
  - 観測可能な完了条件: 上記単体テストが 4 写像すべてについて期待する PresentCommand 値と完全一致することを assert して green で通る
  - _Requirements: 2.4, 3.1, 3.2, 3.3, 3.4, 5.1, 5.2, 5.3_
  - _Boundary: adapter_
  - _Depends: 2.1_

- [ ] 2.5 PresentBridge（SurfaceOutput 本番実装）を実装
  - `PresentBridge::new(tx)` と `impl SurfaceOutput for PresentBridge`（`map_display_command` で変換し `mpsc::Sender` へ非ブロック送出・可変状態は Sender のみ）を実装。送出失敗（受信端 drop）は `debug!`、非数値 scope の drop は `warn!` で log-first 観測する
  - 単体テスト: mpsc チャネル越しに `DisplayCommand` を送ると変換済み `PresentCommand` が受信側に届くこと、非数値 scope 送出時に warn ログが出て何も送出されないこと
  - 観測可能な完了条件: 上記単体テストが green で通り、`PresentBridge` が状態を持たない（Sender 以外の可変フィールドがない）ことをコードで示せる
  - _Requirements: 3.6, 3.7_
  - _Boundary: adapter_
  - _Depends: 2.4_

- [ ] 2.6 BootAssets（構築入力の組立）を実装
  - `build_boot_assets(ghost_root, balloon_root, scopes)`（shell: `surfaces.txt` 読取→`areka_parsers::shell::parse`→bake→scope ごとに `EmoWorld::build`＋`bind_atlas`〔parse/bake は 1 回・AtlasTable は Clone 共有〕、balloon: `build_balloon_target`＋`BalloonModel`、`SurfaceResolver`＝`alias_snapshot()`、static bindset＝`default_bind_ids`→`build_static_bindset`）と `BootWiringError`（`#[from]` 変換群）を実装
  - emo2 fixture を用いた統合テスト: 既知 scope 集合に対し `BootAssets` が populated な `ScopeAssets`／balloon 資産／`BalloonModel`／`resolver`／`static_binds` を返し、戻り値だけで以後ファイル I/O が不要であることを確認
  - 観測可能な完了条件: emo2 fixture を渡した統合テストが `BootAssets` の各フィールドに期待どおりのデータ（例: static_binds が `[1100,1207,1302,1500,1800]`）を含んで green で通る
  - _Requirements: 1.1, 5.5, 7.2_
  - _Boundary: assets_
  - _Depends: 2.3_

- [ ] 3. Core: 窓×資産の scope 整合（plan_attachments・DD-12）を実装
  - `plan_attachments(window_scopes: &[usize], assets: &BootAssets) -> AttachPlan`（`GhostWindows::scopes()` を正として `BootAssets` と突き合わせ、`usize`→`u32` 変換を吸収、窓あり資産なしは `missing_assets` へ、資産あり窓なしは `unused_assets` へ分類）を純関数として実装
  - 単体テスト（DD-12 の 4 パターン全網羅）: 完全一致（計画件数＝窓数）・窓あり資産なし（missing 検出）・資産あり窓なし（unused 検出）・`usize`→`u32` 変換境界
  - 観測可能な完了条件: 上記 4 パターンの単体テストが全て green で通り、完全一致ケースで `AttachPlan.items` の件数が窓数と一致することを assert できる
  - _Requirements: 1.2, 1.4, 4.2_
  - _Boundary: frame_
  - _Depends: 2.1, 2.6_

- [ ] 4. Core: frame 三相結線（Emo2Wiring＋emo2_frame_system）の実装
- [ ] 4.1 Emo2Wiring と attach フェーズを実装
  - NonSend resource `Emo2Wiring`（presenter/rx/runtime/clock/assets/attached を保持）と attach フェーズ（`GhostWindows` Resource＋GPU 資源到達ゲート→`plan_attachments` 呼出→計画項目ごとに shell target `attach_target`→初回 `ShowSurface`、balloon target `attach_target`→初回 `ShowSurface`→`text_slot_view`→`register_actor_view`、`Option::take` で資産を高々 1 回消費、missing/unused は log-first で観測、計画件数と実装着件数を `info!` に列挙）をテスト駆動口 `run_attach_phase(wiring, world)` として実装
  - 単体テスト: GPU 資源なし World では装着しない（ゲート不成立でも panic しない）・`text_slot_view` が None の経路では文字層接続を行わず次フレーム再試行に委ねる（R4.2）
  - 観測可能な完了条件: 上記単体テストが green で通り、GPU 資源が到達しない World で `run_attach_phase` を複数回呼んでも panic せず装着が起きないことを確認できる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 4.1, 4.2, 4.3_
  - _Boundary: frame_
  - _Depends: 3_

- [ ] 4.2 drain フェーズ・text フェーズと emo2_frame_system 登録ラッパーを実装
  - `run_drain_phase(wiring, world)`（attach 完了後のみ `Receiver::try_iter` で `PresentCommand` を全件 `presenter.apply` へ順次適用）と `run_text_phase(wiring, world, talk_time_override)`（`TalkClock::talk_time` が `Some` のとき `present_frame` を呼び、`Err` は `error!`＋継続）、および donor パターン（remove→3 フェーズ→insert）に沿った排他 system `emo2_frame_system(world: &mut World)` を実装
  - 単体テスト: 注入した `PresentCommand` 列を `run_drain_phase` に与えると `presenter.apply` が到着順に呼ばれることを mock で確認し、`run_text_phase` へ固定 `talk_time_override` を与えて `present_frame` 呼出を確認する
  - 観測可能な完了条件: 上記単体テストが green で通り、drain フェーズが FIFO 順で全件を適用し切ることを assert できる
  - _Requirements: 2.2, 2.3, 3.7_
  - _Boundary: frame_
  - _Depends: 4.1, 2.5, 2.2_

- [ ] 5. Integration: main.rs 構築順序再編と wire_emo2_boot 結線
- [ ] 5.1 wire_emo2_boot 統合結線関数を実装
  - `wire_emo2_boot(app, ghost_root, balloon_root, helper_exe) -> Emo2BootOutcome` を実装: `build_boot_assets` 呼出（失敗時は `wired=false` を返し呼び手のフォールバックへ委ねる）→`EmoPresenter::new`／`TextLayerRuntime::new`／`spawn_emo_text`→`TalkClock::new`／`ClockedTextSink::new`→`mpsc::channel`／`PresentBridge::new`→`spawn_seriko(resolver, static_binds, bridge)`→`areka_ghost::boot(GhostBootOptions{surface_sink: SerikoSink, text_sink: ClockedTextSink, ..})`（Err は既存 `is_benign_boot_error` 分類で継続）→`Emo2Wiring` の NonSend 挿入＋`add_systems(FrameFinalize, emo2_frame_system)`
  - emo2 fixture を用いた統合テスト（または example）: `wire_emo2_boot` が有効な emo2 fixture に対し `wired=true` を、存在しない/不正な ghost_root に対し `wired=false` を返すことを確認
  - 観測可能な完了条件: 上記統合テストが green で通り、fixture 経路で `Emo2BootOutcome.wired == true` かつ `ghost`/`seriko` ハンドルが `Some` であることを assert できる
  - _Requirements: 2.1, 7.1, 7.2, 7.3, 7.4, 10.1, 10.2, 10.3, 10.4, 10.6_
  - _Boundary: main.rs, frame, assets, adapter, talk_clock_
  - _Depends: 4.2, 2.6, 2.5, 2.2_

- [ ] 5.2 main() 構築順序を再編し終了処理を結線
  - `main()` の `boot()` 呼出を `WinApp::new()`／`open_startup_window` の後へ移動し、`wire_emo2_boot` の成否で実 sink boot（`wired=true`）／既存 `LogSink`×2 フォールバック boot（`wired=false`）を呼び分ける。`run()` 復帰後 `shutdown(CloseReason::User)`（DD-10）を呼び、続けて seriko `ActorHandle::join` を行う。既存の `is_benign_boot_error`・ダミー窓フォールバック・smoke ゲート（`AREKA_APP_SMOKE_EXIT_MS`）は不変のまま維持する
  - 観測可能な完了条件: 既存 `tests/smoke_boot_loop_exit.rs`（フォールバック経路）が変更なしで green のまま維持され、`shutdown` の呼出理由が `CloseReason::User` になっていることをコードで確認できる
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 7.1, 10.7_
  - _Boundary: main.rs_
  - _Depends: 5.1_

- [ ] 6. Validation: 決定論 spine 統合テスト（R8）
- [ ] 6.1 scripted ShioriBackend と spine テストハーネスの土台を実装
  - areka 側 spine テストローカルの最小 `ScriptedShioriBackend`（DD-11・OnBoot/OnClose 応答台本を返す fake）を実装し、`ShioriWiring::Custom`＋`TickerMode::Disabled`（`DispatcherMsg::Tick` 注入）＋実 sink 結線（`spawn_seriko(out=PresentBridge)`／`ClockedTextSink<EmoTextSink>`）＋headless GPU World（`CoInitializeEx(MULTITHREADED)`＋`GraphicsCore::new()`＝WARP 可＋`WucGraphicsResource`）を組み立てるテストハーネスを構築する
  - 観測可能な完了条件: ハーネスが scripted ghost を boot させ、Tick 注入により attach 準備状態まで panic なく到達することをスモークレベルの assert で確認できる
  - _Requirements: 8.1, 8.3, 8.4, 8.6_
  - _Boundary: spine test_
  - _Depends: 5.1_

- [ ] 6.2 spine S1（boot→表示）・S3/S4（`\b` 配送）ケースを実装
  - **S1**: boot 後 Tick 注入→attach フェーズ実行→shell/balloon target の readback が非全透明であること、かつ**計画件数＝実装着件数**（DD-12 の縮退がバグを隠さない檻）を assert する
  - **S3**: `\b[-1]`→`\b[0]` を含む scripted 台本→受信 `PresentCommand` 列に `Hide{balloon}`→`ShowSurface{balloon,0,binds=default}` が順序どおり現れることをアサートし、apply 後の balloon readback 遷移を確認する
  - **S4**: `\b` を含まない OnBoot 相当台本が S1 経路（boot→表示）を完走することを確認する
  - 観測可能な完了条件: S1/S3/S4 の 3 テストが全て green で通り、S1 のピクセル述語と装着件数一致、S3 の受信列順序、S4 の完走がそれぞれ assert 結果として残る
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 5.4, 5.5, 8.1, 8.2, 8.5_
  - _Boundary: spine test_
  - _Depends: 6.1_

- [ ] 6.3 spine S2（talk→typewriter）・S5（close 握手）ケースを実装
  - **S2**: `\s[2100]` とテキストを含む scripted 台本→`Show` 系 `PresentCommand` 受信列を assert→apply 後の shell readback 変化を確認。テキスト cue は注入 `talk_time` の階段値で駆動し、`opaque_count` の単調増加・validrect 外に非透明なし・`Clear` 後全域透明を**単一 talk 内（Clear 起点後）に限定して**assert する（talk_clock の既知制約に整合）
  - **S5**: `shutdown(CloseReason::User)`→OnClose 台本消化→全ハンドル（seriko 含む）が有界 join で完了することを確認する
  - 観測可能な完了条件: S2/S5 の 2 テストが green で通り、S2 の単調増加述語が単一 talk 区間で成立し、S5 の join が timeout せず完了することがテスト結果として残る
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 6.1, 6.2, 6.3, 8.3, 8.5_
  - _Boundary: spine test_
  - _Depends: 6.2_

- [ ] 7. Validation: smoke 回帰確認と env-gate 実走
- [ ] 7.1 既存 smoke 回帰の確認と wire 成立マーカーの追加
  - 既存 `tests/smoke_boot_loop_exit.rs`（フォールバック経路）が本仕様の変更後も無変更で green のままであることを確認する。実 fixture 経路の smoke（`skeleton_boots_with_real_ghost_windows_and_exits_zero`）が `emo2_frame_system` の schedule 登録（実結線経路）を少なくとも 1 回踏んでいるかを確認し、届いていなければ wire 成立ログマーカーの一行 assert を追加する（S6 撤回に伴う存在チェックの担保・決定論檻ではない）
  - 観測可能な完了条件: 両 smoke テストが green で通り、実 fixture 経路の smoke が wire 成立マーカーを assert していることをテストコードで確認できる
  - _Requirements: 6.4, 7.3, 7.4, 10.7_
  - _Boundary: E2E/Smoke_
  - _Depends: 5.2_

- [ ]* 7.2 env-gate 実走テストと人間サインオフ手順を実装
  - `AREKA_EMO2_REAL_RUN` 環境変数で有効化される実走テストを実装: 未設定時は即 return（R9.2・DoD 非前提）。設定時は `CARGO_BIN_EXE_areka` を emo2 fixture＋実 pasta helper で起動し、`AREKA_APP_SMOKE_EXIT_MS` による自動 close→exit 0 と wire 成立／attach 完了のログマーカーを assert する。実 DPI（≠96）での目視チェックリスト（実サーフェス表示位置／typewriter 進行／ドラッグ追従／close→静かな終了）をテスト doc 内へ明文化する
  - 観測可能な完了条件: env 変数未設定時にテストが即座に skip され、設定時は実プロセスが exit 0 とログマーカーを残して終了することを確認できる
  - _Requirements: 9.1, 9.2, 9.3_
  - _Boundary: E2E/real-run_
  - _Depends: 5.2_

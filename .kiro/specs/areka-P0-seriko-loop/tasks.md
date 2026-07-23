# Implementation Plan

## 1. Foundation: parser 転記スコープ拡張（method・interval 完全語彙）

- [x] 1.1 `DrawMethod` opaque 型・`Pattern.method` 欄・`Interval::Other` variant を追加する
  - `DrawMethod(String)` を `ElementPath` と同規律の opaque NewType として定義し、`Pattern` 構造体に `method: DrawMethod` を追加する
  - `Interval` に `Other(Box<str>)` variant を追加する（`#[non_exhaustive]` 内・下流非破壊）
  - doc comment に正典位置（新形式＝第 1 位置／旧形式＝第 3 位置）を転記する
  - 完了状態: `cargo build -p areka-parsers` が新しい型定義を含めて成功する
  - _Requirements: 4.6, 8.4_

- [x] 1.2 decode.rs の overlay フィルタと interval フォールバックを撤去し忠実転記化する
  - `decode_animations` の `== Some("overlay")` フィルタを撤去し、field[1] を method として全 pattern 行を転記する
  - 未認識 interval キーワードの fallback-Bind を撤去し、原文を `Interval::Other` へ転記する
  - 完了状態: 非 overlay な pattern 行・未認識 interval キーワードのいずれも decode 結果から消えず、転記後のモデルに原文どおり現れる
  - _Depends: 1.1_
  - _Requirements: 4.6, 7.5, 8.2, 8.4_

- [x] 1.3 parser テスト群を method・Interval::Other 転記の檻へ更新する
  - `decode_tests.rs`／`parse_tests.rs`／`model_tests.rs` を更新し、method 忠実転記（overlay/replace/未知名/欠落）と `Interval::Other`（例: `sometimes`）が Bind へ倒れず原文保持されることを検証する
  - 既存 `Pattern` リテラルを新欄に追随させる
  - 完了状態: `cargo test -p areka-parsers` が新規・既存テストすべて green
  - _Depends: 1.2_
  - _Requirements: 4.6, 8.2, 8.4_

## 2. (P) Foundation: PatternState 公開型正本（emo-compose）

- [x] 2. (P) `PatternState`／`PatternFrame` を emo-compose の新モジュールへ定義する
  - `PatternState`（`BTreeMap<u32, PatternFrame>` ラップ・正準順序で Eq 安定・`Default`=空）と `PatternFrame { surface_id: u32, method: ComposeMethod, x: i64, y: i64 }` を定義する
  - `set`/`remove`/`get`/`iter`/`is_empty` を実装する
  - 完了状態: `PatternState::default().is_empty()` が true・`cargo build -p areka-emo-compose` が成功する
  - _Requirements: 5.1, 5.4_
  - _Boundary: PatternState (areka-emo-compose pattern.rs)_

## 3. (P) Foundation: ghost additive tick レーン

- [x] 3. (P) `spawn_loop_ticker` を既存 ticker に additive 追加する
  - `LoopTickerConfig`（既定 16ms・`GetTickCount64` クロック・テスト注入可）を定義する
  - `BoundarySchedule` を再利用し、クロージャ配送（`Box<dyn FnMut(Tick)+Send>`）でグリッド発火ごとに 1 回 `deliver` を呼ぶ単発スポーナーを実装する
  - 既存 `spawn_ticker`・`TickerConfig`・2 系統は一切変更しない
  - 完了状態: 注入クロックでのグリッド発火・catch-up 1 回・`Close`/切断での正常終了が単体テストで確認できる
  - _Requirements: 1.1, 1.3, 1.4, 7.4_
  - _Boundary: spawn_loop_ticker (areka-ghost ticker.rs)_

## 4. Core: seriko 状態/出力への PatternState 配線

- [x] 4.1 (P) ScopeStates に per-(scope,slot) PatternState と commit_pattern を追加する
  - `Slot { Shell, Balloon }` と `pattern_states` 表を `dynamic_binds` と同居させる
  - `commit_pattern`（冪等ガード→書込→表示中なら Changed）を commit_bind 鏡映で実装する
  - `apply`/`apply_balloon` の surface 切替時に当該 slot の PatternState を空へリセットする
  - `apply_bind` 系の Show 再発行にも `current_pattern` を同梱する（bind 変化時に現在コマを保ったまま再合成）
  - 完了状態: 同値 commit は `Unchanged`・異なる値は `Changed(Show/ShowBalloon)` を返すことが単体テストで確認できる
  - _Depends: 2_
  - _Requirements: 5.1, 6.1, 6.2_
  - _Boundary: ScopeStates (areka-seriko state.rs)_

- [x] 4.2 (P) DisplayCommand::Show/ShowBalloon に pattern 欄を追加する
  - `Show { scope, surface_id, binds, pattern: PatternState }`・`ShowBalloon { scope, surface_id, pattern: PatternState }` へ欄追加する（`Hide`/`HideBalloon` は不変）
  - 完了状態: 非 `#[non_exhaustive]` ゆえ下流の網羅 match がコンパイルエラーで追随を要求すること（意図された強制追随）を確認する
  - _Depends: 2_
  - _Requirements: 5.1_
  - _Boundary: DisplayCommand (areka-seriko output.rs)_

## 5. Core: seriko タイムライン評価コア

- [ ] 5.1 EmoWorld から AnimationTable を boot 時構築する
  - `LoopTrigger{Random{k}|BindRandom{k}}`・`LoopFrame{surface_id, method, wait_ms, x, y}`・`LoopAnimation`・`AnimationTable` を定義する
  - `from_world` で `EmoWorld` から read-only スナップショットを構築し、`Random`/`BindRandom` のみ採録する
  - `Interval::Bind`・`Interval::Other(語彙)`・将来 variant は非採録・debug! に記録する（`Other` は元語彙文字列込み）
  - コマ列空のアニメ・`k == 0` は非採録・warn! する
  - method は構築時に `ComposeMethod::from_name` で 1 回解決する
  - 口パク（`interval,talk`）・`\i[N]`・動的 bind・talk cue は構造的に採録対象外とする（table が拾わない）
  - 完了状態: emo2 fixture から構築した表で kero/sakura の 2 アニメのみが採録され、他 interval は debug! ログとともに非採録であること、`k == 0`／コマ列空のアニメが warn! とともに非採録であること、method 解決（`ComposeMethod::from_name`）が正しく適用されることの 3 点がテストで確認できる
  - _Depends: 1.3_
  - _Requirements: 4.5, 4.6, 7.5, 8.1, 8.2, 8.3, 8.4_
  - _Boundary: AnimationTable (areka-seriko table.rs)_

- [ ] 5.2 純関数コア（frame_at・LotteryBoundary・seeded_rng・should_fire）を実装する
  - `LotteryBoundary`（1000ms 絶対グリッド跨ぎ検出・catch-up 1 回）を実装する
  - `frame_at`（経過時刻→現在コマ・`Pending`/`Active`/`Stopped`/`FinishedResidual`）を実装し、デファクト2点（`-1` 無し末尾残留・再生中非再抽選対象）を期待値として焼き込む
  - `seeded_rng`（SplitMix64）・`should_fire`（1/N 抽選）を実装する
  - 完了状態: 累積 wait 進行の境界値（±1ms）・疎 index・`-1` での `Stopped`・`-1` 無し末尾での `FinishedResidual` 恒久・固定シードの再現列が単体テストで全経路網羅される
  - _Depends: 5.1_
  - _Requirements: 1.2, 2.1, 2.4, 4.1, 4.2, 4.3, 4.4, 4.5, 7.1, 7.5, 8.2, 9.4_
  - _Boundary: timeline (areka-seriko timeline.rs)_

- [ ] 5.3 LoopRuntime.on_tick を実装する
  - 抽選（境界跨ぎ時のみ・表示中×非再生中×bind ゲート通過）→ 進行（`frame_at` で全再生中アニメの新 PatternState 組立）→ `commit_pattern` への差分反映、を統括する
  - 抽選消費順序を固定（scope 昇順→Shell→Balloon→animation id 昇順）にする
  - `IdleResidual` から再抽選発火した瞬間に残留コマを即時クリアする（討議 #2 裁定：表示は `frame_at` の結果のみに依存し直前 PatternState に依存しない）
  - `on_surface_changed`/`on_hidden` で当該 slot の playback を全除去する
  - bind の書込 API は一切呼ばない
  - 完了状態: 表示中×非再生中×bind ゲート通過の条件が揃ったときのみ抽選が走り、bind OFF では乱数を消費しないことが単体テストで確認できる
  - _Depends: 5.1, 5.2, 4.1, 4.2_
  - _Requirements: 1.2, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3, 4.3, 6.2, 7.5, 8.2, 9.4_
  - _Boundary: LoopRuntime (areka-seriko looper.rs)_

## 6. Core: seriko actor の Tick 受理と単一発行点継承

- [ ] 6.1 SerikoMsg::Tick・send_tick・spawn_seriko 拡張・handle_message Tick 腕を実装する
  - `SerikoMsg::Tick { now_ms: u64 }` を additive 追加し、`SerikoSink::send_tick(&self, now_ms)` を実装する（送出失敗は debug!・shutdown 期待事象）
  - `spawn_seriko` に `loop_config: SerikoLoopConfig` 引数を追加する（既存呼び手は `SerikoLoopConfig::disabled()` で従来挙動と同値）
  - `handle_message` に Tick 腕を追加し、`loop_runtime.on_tick` の返す指令列を既存 `emit_display` 単一発行点のみで発行する
  - Emote/BalloonSurface 適用が `Changed` を返した際に `loop_runtime.on_surface_changed` を呼ぶ
  - 完了状態: 表示中 slot が 1 つもない tick が完全 no-op（無発行）であること、cue/bind/Close の既存経路が無改変であることがテストで確認できる
  - _Depends: 5.3_
  - _Requirements: 1.1, 6.1, 6.3, 7.1, 7.5, 8.3_
  - _Boundary: actor (areka-seriko actor.rs)_

## 7. Core: emo-compose の PatternState 合流

- [ ] 7.1 (P) compose_into/compose 署名に pattern を追加する
  - `compose_into(out, world, atlas, surface_id, active_binds, pattern: &PatternState)`／`compose` へ引数追加し、`build_plan`/`derive_ops` へ透過する
  - 完了状態: `cargo build -p areka-emo-compose` が新シグネチャで成功する
  - _Depends: 2_
  - _Requirements: 5.1_
  - _Boundary: emo-compose lib.rs_

- [ ] 7.2 flatten_surface への合流と method ゲートを実装する
  - `flatten_surface` 層(ii) で「有効 bind pattern0 の集合 ∪ PatternState のコマ集合」を既存 animation-sort 整列へ合流し、同 ID はコマが pattern0 寄与を置換する
  - `method.is_implemented()`（Overlay のみ）でコマを描画し、非 Overlay は warn!（method 名込み）＋不描画とする
  - pattern0 静的経路にも同じ method ゲートを追加する（overlay フィルタ撤去で非 overlay pattern0 が流入し得るため）
  - 完了状態: 非 Overlay コマ・pattern0 が warn! を出しつつ描画されないことがテストで確認できる
  - _Depends: 7.1_
  - _Requirements: 4.2, 4.6, 5.3, 7.5, 8.4_
  - _Boundary: emo-compose plan.rs_

- [ ] 7.3 golden・檻を追加する（空 PatternState 等価・transient 合流・外形内収まり）
  - 空 PatternState で拡張前と byte 等価であることの golden を追加する
  - transient コマ合流（同 ID 置換・ID 整列不変）の golden を追加する
  - emo2 fixture の採録アニメ全コマ（kero 2106-2110／sakura 1410-1412）の原寸＋(x,y) が当該ベース surface の Extent 内に収まることをアサートする
  - 完了状態: `cargo test -p areka-emo-compose` の golden 群が green（空 PatternState 拡張前後で byte 完全一致）
  - _Depends: 7.2_
  - _Requirements: 5.4_
  - _Boundary: emo-compose golden_tests.rs / composer_tests.rs_

## 8. Core: emo-present のキー/指令契約拡張

- [ ] 8.1 (P) ComposeKey に pattern を追加する
  - `ComposeKey { surface_id, binds, pattern: PatternState }` へ拡張し、`get`/`insert` 署名を追随させる
  - 完了状態: pattern 差分でキャッシュミス・同値でヒットすること、`invalidate_all` の挙動が不変であることが単体テストで確認できる
  - _Depends: 2_
  - _Requirements: 5.2_
  - _Boundary: ComposeCache (areka-emo-present cache.rs)_

- [ ] 8.2 PresentCommand::ShowSurface に pattern を追加し presenter を配線する
  - `PresentCommand::ShowSurface` に `pattern: PatternState` 欄を追加する
  - presenter が ShowSurface 適用時に pattern をキャッシュ引き当てと `compose_into` へ透過する
  - 完了状態: `cargo build -p areka-emo-present` が新シグネチャ・新規呼出しで成功する
  - _Depends: 8.1, 7.1_
  - _Requirements: 5.1, 5.2_
  - _Boundary: PresentCommand/presenter (areka-emo-present command.rs, presenter.rs)_

## 9. Integration: areka emo2_boot 結線

- [ ] 9.1 BootAssets に loop_tables を追加する
  - shell 表・balloon 表それぞれの `EmoWorld` スナップショットから `AnimationTable::from_world` で構築し、`BootAssets.loop_tables: LoopTables { shell, balloon }` へ格納する（面種非依存・裁定 (a)）
  - 完了状態: boot 時のファイル I/O 追加なしで表が構築されることを確認する
  - _Depends: 5.1_
  - _Requirements: 4.6_
  - _Boundary: areka emo2_boot assets.rs_

- [ ] 9.2 wire_emo2_boot を配線する
  - `SerikoLoopConfig { shell_table, balloon_table, rng: seeded_rng(seed) }` を組み立て、`seed` は `RandomState` 由来を info! でログする
  - `spawn_seriko` へ渡し、`spawn_loop_ticker(LoopTickerConfig::default(), closure)` を起動する（closure は `SerikoSink` クローンを閉じ込め `send_tick` を呼ぶ）
  - 戻り値のハンドルに loop ticker の停止端を追加する
  - 完了状態: 本番シードが info! ログに出力され、loop ticker 起動後に tick が seriko へ到達することを確認する
  - _Depends: 3, 6.1, 9.1_
  - _Requirements: 7.4_
  - _Boundary: areka emo2_boot mod.rs_

- [ ] 9.3 adapter/frame の pattern 透過を更新する
  - `map_display_command` が `Show.pattern`→`ShowSurface.pattern`／`ShowBalloon.pattern` を非改変転写するよう更新する
  - drain 相（frame.rs）の ShowSurface 網羅 match を pattern 透過へ追随させる
  - 完了状態: `cargo build -p areka` が adapter/frame の新規欄込みで成功する
  - _Depends: 6.1, 8.2_
  - _Requirements: 5.1_
  - _Boundary: areka emo2_boot adapter.rs, frame.rs_

- [ ] 9.4 spine ハーネスを直接 Tick 注入方式へ更新する
  - loop ticker を起動せず、`SerikoSink::send_tick(now)` の直接注入＋`SerikoLoopConfig`（実 emo2 表＋固定注入乱数列）で駆動するよう更新する
  - 既存 spine 全テストが `SerikoLoopConfig::disabled()` 相当（ループ不活性）で従来観測どおり非退行であることを先に確認してから、まばたき e2e を追加する
  - 完了状態: 既存 spine テストが loop 不活性経路で従来どおり green のまま維持される
  - _Depends: 9.2, 9.3_
  - _Requirements: 7.1, 7.2, 7.3_
  - _Boundary: areka emo2_boot spine.rs_

- [ ] 9.5 main.rs の shutdown 順序と実機 grep マーカーを整備する
  - shutdown 順序を「loop ticker Close→join → ghost.shutdown → seriko join」に設定する
  - 実機 grep マーカー（抽選発火・再生開始/停止/残留）の info! ログを配置する
  - 完了状態: `AREKA_APP_SMOKE_EXIT_MS` 有界 auto-exit 後に `RUST_LOG` grep でマーカーが検出できることを確認する
  - _Depends: 9.2_
  - _Requirements: 7.4_
  - _Boundary: areka main.rs_

## 10. Validation: 決定論テストと実機サインオフ

- [ ] 10.1 looper 統合テスト（注入 tick+rng 列で全経路網羅）
  - kero 型（`-1` 終端→ベース復帰）・sakura 型（末尾残留）・再生中の非再抽選・bind OFF での判定不発（乱数非消費）・bind ON での発火・変化なし tick の無発行・surface 切替でのコマ消滅・残留→再発火の即時クリア、を固定注入 tick 列＋乱数列で網羅する
  - sleep を使用せず `send_tick`/`handle_message` の直接駆動＋`Close→join` 同期で行う
  - 完了状態: 固定注入列に対して期待 PatternState 列・発行列が完全一致することがテストで確認できる
  - _Depends: 6.1_
  - _Requirements: 6.2, 7.2, 7.3, 9.4_
  - _Boundary: areka-seriko 統合テスト_

- [ ] 10.2 spine e2e golden テスト（kero/sakura 実周期・R3.4 既定 OFF 検証・既存非退行）
  - 実 emo2 fixture＋固定乱数＋`send_tick` 注入で kero まばたき 1 周（2106→2110→`-1`→ベース復帰）の PresentCommand 列 golden を追加する
  - `\![bind,まばたき,通常,1]` 貫通後の sakura まばたき 1 周（1412→1411→1410 残留）と、fixture 既定（bind OFF のまま）では何も起きないことの対照を golden 化する
  - 完了状態: 既存 spine 全テストが非退行のまま、新規 golden が実 fixture ベースで green
  - _Depends: 9.4_
  - _Requirements: 3.4, 7.2_
  - _Boundary: areka emo2_boot spine.rs (E2E)_

- [ ] 10.3 実機サインオフ（有界 auto-exit + grep + 人間目視）
  - 実 emo2・実 pasta.dll・実 DPI で起動し、`AREKA_APP_SMOKE_EXIT_MS` 有界 auto-exit＋`RUST_LOG` grep（抽選/再生マーカー）で機械判定する
  - kero（`random`）のまばたきと、sakura の bindgroup を `\![bind,まばたき,通常,1]` で ON にした状態でのまばたきを人間が目視確認する
  - 完了状態: 実機ログに抽選発火・再生開始/停止/残留マーカーが記録され、開発者が両まばたきの目視確認結果を判定として記録する
  - _Depends: 10.2, 9.5_
  - _Requirements: 9.1, 9.2, 9.3_
  - _Boundary: 実機サインオフ手順_

## Implementation Notes

- **1.2 method/interval 転記**: `Interval::Other(Box<str>)` は単一文字列＝**keyword のみ**保持（`sometimes,5` → `Other("sometimes")`・K=5 は非採録）。設計 (design.md `Box<str>` 型形状＋討議 #1 裁定「sometimes と書いたのに動かない」診断性) どおりで承認済。未認識 interval **単独**行（pattern 無し）も認識 interval 単独行と対称に slot を確定する（faithful transcription・非駆動）。cargo は PowerShell 経由で実行（Git Bash coreutils の link.exe が MSVC link を遮蔽するため）。
- **cross-crate ripple（1.1/1.2 後の baseline 修復・commit `<repair>`）**: parser `Pattern` への `method: DrawMethod` 欄追加は非 `#[non_exhaustive]` struct ゆえ**下流の全 `Pattern {..}` 構築サイト（test fixture のみ・production は parser のみが構築）を破壊**する。修復先＝emo-atlas manifest.rs / emo-compose plan.rs・log_firing_tests.rs・composer_tests.rs / emo-present presenter.rs（全て `#[cfg(test)]`）に `method: DrawMethod::new("overlay".to_string())` を追加。加えて 1.1 は `DrawMethod` を shell `mod.rs` の `pub use` へ再エクスポートし忘れていた（`pub` 欄型は公開必須）——併せて修正。**教訓: struct 欄追加タスクのレビューは `-p <crate>` でなく `cargo test --workspace --no-run` で下流回帰を検出すべし。** 以降 output.rs / command.rs 等の enum 欄追加（4.2, 8.2）も同様に workspace-tests build で追随漏れを検出する。
- **areka ビルド赤ウィンドウ（4.2〜9.3・設計意図どおり）**: `DisplayCommand::Show/ShowBalloon` への `pattern` 欄追加（4.2）で `crates/areka/src/emo2_boot/adapter.rs`（`map_display_command` の網羅 match＋テスト構築子）がコンパイル不能になる＝設計「発見 D＝意図された強制追随」。9.3（adapter/frame の pattern 透過）で解消するまで **`cargo build -p areka` は expected-red**。中間タスク（5.x/6.1/7.x/8.x）のレビューは各 `-p <crate>` ゲートで判定（seriko/emo-compose/emo-present は areka に非依存ゆえ独立にグリーン）。9.3 完了後に workspace 全体ビルドを復旧確認する。

# Implementation Plan

> 生成: 2026-07-24（requirements.md 2026-07-24 改訂版・design.md 2026-07-24 生成版・design-validation.md GO 判定に基づく）。
> W4 並走契約（roadmap 追記㊵/㊹/㊺）: `crates/areka/src/placement/measure.rs`・`crates/areka/src/input_events/`・`crates/areka/src/emo2_boot/`（`consumer_ledger.rs` 含む）は本タスク群では**編集しない**。

- [x] 1. Foundation: 永続結線の純関数基盤（`placement/persist.rs`）
- [x] 1.1 モジュール骨格・寛容 parse・保存 entries 構築関数の実装
  - `crates/areka` の `Cargo.toml` へ `areka-sylphya` への workspace 内依存を additive 追加する
  - `placement/mod.rs` に `pub mod persist;` を追加し、`prepare_never_reads_or_writes_ghost_dat` の doc コメントを現況（永続ストアは `sylphya.toml` の別系統であり prepare は引き続き永続を読まない）へ更新する
  - 文字列→i32 の寛容 parse（非数値・空文字は「値なし」として `None` を返す）と、窓位置・バルーンオフセットの保存 entries 構築関数を実装する
  - 観測可能な完了条件: `parse_px` が数値文字列を i32 へ、非数値・空文字を `None` へ決定論的に変換するユニットテストが通ること
  - _Requirements: 1.7, 6.1_
  - _Boundary: placement/persist.rs_

- [x] 1.2 バルーン相対オフセットのアンカー辺基準変換（双方向純関数）
  - キャラ窓の左上基準オフセットと、アンカー辺（下端吸着は下端・Free は左上に縮退）基準オフセットを相互変換する純関数を実装する
  - 観測可能な完了条件: 5 アンカー種別×複数のキャラサーフェス寸法の組み合わせで `to_persist` → `from_persist` の往復が元のオフセットへ一致する往復恒等テストが通ること
  - _Requirements: 2.2, 8.5_
  - _Boundary: placement/persist.rs_

- [x] 1.3 復元時再射影 `project_restore` の実装
  - `project_anchor`（アンカー辺再導出）と `work_area_for_window`（最近傍モニタの作業領域）を再利用し、復元位置が作業領域外のとき吸着規則を保ったまま作業領域内へ寄せる純関数を実装する
  - 観測可能な完了条件: 作業領域内の入力は恒等出力となり、下端吸着窓の作業領域外入力は下端一致・水平位置保持で域内へ収まる決定論テストが通ること
  - _Requirements: 5.1, 5.2, 5.3, 8.2_
  - _Boundary: placement/persist.rs_

- [x] 1.4 起動時先読み `load_restored_state` と復元マージ `apply_restored_placements` の実装
  - ghost root から `sylphya::persist::load_scope` を直接読み出す先読み関数（不在・破損はすべて空 entries＝値なしへ寛容縮退）と、entries を `ScopePlacement` へマージする純関数（スコープ別に窓位置優先適用・バルーンは基準変換の逆適用で導出・値なしは resolver 既定を保持）を実装する
  - マージ結果は永続状態へ一切書き込まない構造（書込 API を呼び出せない）であることをコードで保証する
  - 観測可能な完了条件: entries ありでは保存済み位置が既定位置解決に優先して採用され、entries が空のときは入力 placements と恒等になるユニットテストが通ること
  - _Requirements: 1.4, 1.5, 1.6, 1.8, 2.3, 2.4, 2.5, 5.4, 6.1, 8.2_
  - _Boundary: placement/persist.rs_

- [x] 2. Core: ドラッグ確定→永続書込の結線
- [x] 2.1 `PersistWiring` リソースと保存投函ヘルパの実装
  - UI スレッド常駐の NonSend リソース `PersistWiring`（`SylphyaPublisher` の clone を保持）と、`PersistScope::Ghost` 固定で entries を非ブロッキング投函するヘルパ関数を実装する。リソース不在時は debug ログのみで no-op とする
  - 観測可能な完了条件: リソース挿入済みの headless World で entries を渡すと `persist_put` が呼び出され、リソース未挿入では panic せず no-op となることがテストで確認できること
  - _Requirements: 1.1, 1.9, 6.2, 7.1_
  - _Boundary: placement/persist.rs_

- [x] 2.2 キャラ窓 DragEnd の全アンカー結線と保存フック
  - `spawn.rs` の非 Free アンカー限定ガードを撤去し、全キャラ窓へ `OnDragEnd` を結線する
  - `on_char_drag_end` の最終確定位置（`mapped`）確定後に、当該窓のスコープと位置から保存 entries を構築して投函するフックを追加する
  - 観測可能な完了条件: 非 Free アンカーのドラッグ確定で該当スコープの窓位置 entries が投函されることをテストで確認できること
  - _Requirements: 1.1, 1.3, 1.9_
  - _Depends: 1.1, 1.4, 2.1_
  - _Boundary: placement/spawn.rs, placement/follow.rs_

- [x] 2.3 バルーン窓 DragEnd 結線と `on_balloon_drag_end` の実装
  - バルーン窓へ `OnDragEnd(on_balloon_drag_end)` を新規結線する（連続イベントの `on_balloon_drag` は書込トリガにしない）
  - `on_balloon_drag_end` は DragEnd の最終確定位置から `balloon_pos − char_pos` を再導出し（`on_balloon_drag` と同一式・in-session オフセットの再利用はしない）、アンカー辺基準へ変換して保存する
  - 観測可能な完了条件: バルーンドラッグ確定で当該スコープのバルーンオフセット entries が、最終確定位置から導出した値と一致して投函されることをテストで確認できること
  - _Requirements: 2.1, 8.1_
  - _Depends: 1.2, 2.1_
  - _Boundary: placement/spawn.rs, placement/follow.rs_

- [x] 3. Core: GhostRuntime の公開面拡張
- [x] 3.1 (P) `sylphya_publisher()` アクセサの追加
  - `GhostRuntime` へ `kanade()`/`dispatcher()` と同型の additive アクセサを追加し、UI 側（main.rs）が `SylphyaPublisher` の clone を取得できるようにする
  - 観測可能な完了条件: アクセサ経由で取得した publisher で `persist_put` が呼び出せることをテストで確認できること
  - _Requirements: 6.2_
  - _Boundary: areka-ghost/runtime.rs_

- [x] 3.2 shutdown() での `barrier()` 明示確認
  - `shutdown()` の sylphya `close()` 呼び出し直前に `barrier()` を呼び、Ok なら flush 確認ログを、Err なら warn ログを出して続行する
  - 観測可能な完了条件: shutdown 系列を駆動するテストで `barrier()` 呼び出し後に close へ到達し、Err 時も panic せず続行することが確認できること
  - _Requirements: 1.2_
  - _Boundary: areka-ghost/runtime.rs_

- [x] 4. Core: 汎用プロパティ SET キュー語彙
- [x] 4.1 (P) `EpilogueCommand` 型と `StartTalk.epilogue` の追加
  - `areka-talk` に汎用コマンドを表す `EpilogueCommand { name, tokens }` を新設し、`StartTalk` へ additive フィールド `epilogue: Vec<EpilogueCommand>`（既定空）を追加する
  - 既存呼び出し点の追随を最小化する `StartTalk::new(talk_id, script)`（epilogue 空の従来形コンストラクタ）を用意する
  - 観測可能な完了条件: `StartTalk::new` で構築した値が `epilogue` 空を持ち、既存の構築点がコンパイルを保つこと
  - _Requirements: 3.4_
  - _Boundary: areka-talk_

- [x] 4.2 (P) `append_epilogue` の実装と sakura `drive.rs` への結線
  - CueSheet 末尾（既存 cue の `max(start_time+duration)`・duration 0）へ epilogue を carrier cue として付加する純関数を `areka-sakura` に実装する
  - `drive.rs` の `on_start` で compile 後・空判定前に本関数を適用する（epilogue 空は恒等＝既存経路不変）
  - 観測可能な完了条件: 非空 epilogue を渡すと CueSheet の末尾に carrier cue が追加され、既存末尾要素（選択待ちなど）より後の時刻に安定ソートで並ぶことがテストで確認できること
  - _Requirements: 3.4_
  - _Depends: 4.1_
  - _Boundary: areka-sakura_

- [x] 4.3 (P) `PropSetCueSink` の実装
  - コマンド名 `areka.prop.set` を名前自己選別で受理する `CueSink` 実装を `areka-ghost` に新設する
  - 受理する key を `PersistKey::BootCount`/`VanishCount` の正準文字列に限定し（`WindowPos`/`BalloonOffset` は拒否）、引数不足・未知 key は warn 付きスキップとする。受理時は `PersistScope::Ghost` で `persist_put` を呼び、info ログを 1 本出す
  - 観測可能な完了条件: `areka.prop.set` かつカウンタ key の cue で `persist_put` が呼ばれ、位置系 key・未知名・引数不足では拒否されログのみ残ることがテストで確認できること
  - _Requirements: 3.4, 6.2, 7.1_
  - _Depends: 4.1_
  - _Boundary: areka-ghost/prop_sink.rs_

- [x] 5. Core: kanade 初回起動ゲートと Reference0
- [x] 5.1 (P) `KanadeConfig` additive フィールドの追加
  - `first_boot: bool`（既定 true）・`vanish_count: u32`（既定 0）・`first_boot_epilogue: Vec<EpilogueCommand>`（既定空）を additive 追加する。既定値により既存構築点は挙動不変を保つ
  - 観測可能な完了条件: 既定値で構築した `KanadeConfig` が現行の boot happy-path 檻を無改変のまま通過すること
  - _Requirements: 3.1, 4.1_
  - _Depends: 4.1_
  - _Boundary: areka-kanade/msg.rs_

- [x] 5.2 (P) `events::on_first_boot` の署名変更
  - `on_first_boot(snapshot, vanish_count: u32)` へ変更し、Reference0 を `vanish_count` の文字列化とする
  - 呼び出し点（boot.rs 発行点・関連 assert）とテスト（events.rs 檻・kanade boot/full_run テスト）を追随させる
  - 観測可能な完了条件: `vanish_count` に非ゼロ値を渡すと Reference0 がその値になることがテストで確認できること
  - _Requirements: 4.1, 4.2_
  - _Depends: 5.1_
  - _Boundary: areka-kanade/schedule/events.rs_

- [x] 5.3 `on_prefetch_reply` のゲート分岐と epilogue 添付
  - `on_prefetch_reply` の末尾を `config.first_boot` で分岐させ、true なら新シグネチャの `on_first_boot(&snapshot, config.vanish_count)` を呼ぶ OnFirstBoot へ、false なら OnFirstBoot を発行せず OnBoot から `Phase::BootMain` へ直行させる（204 フォールスルーは不変）
  - `to_baseware_version`（BootType-Value／BootMain-Value／BootMain-204 の単一合流点）で `config.first_boot_epilogue` を StartTalk へ添付する。トーク本文が存在しない初回（204-204）は epilogue-only の StartTalk を発行し、epilogue が空の通常起動 204 は従来どおり StartTalk なしとする
  - BootPrefetch 段（username 照会）自体は変更しない
  - 観測可能な完了条件: `first_boot=false` で OnFirstBoot が発行されず OnBoot から運行が始まること、`first_boot=true` かつ 204-204 で epilogue-only StartTalk が発行されることの両方がテストで確認できること
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Depends: 5.1, 5.2_
  - _Boundary: areka-kanade/schedule/boot.rs_

- [x] 6. Integration: main.rs 起動シーム結線
- [x] 6.1 復元マージの呼出し
  - `open_startup_window` の正常経路（snapshot 構築直後）で `load_restored_state` → `apply_restored_placements` を呼び、spawn へ渡す placements を復元値でマージ済みのものへ差し替える
  - 観測可能な完了条件: 永続状態に保存済み位置がある場合、実際に spawn される窓の初期位置が既定位置ではなく保存済み位置になることが結合テストで確認できること
  - _Requirements: 1.4, 5.1, 6.1_
  - _Depends: 1.4_
  - _Boundary: main.rs_

- [x] 6.2 `PersistWiring` の挿入（wired／fallback 両経路）
  - `runtime.sylphya_publisher()` の clone を `PersistWiring` として、wire 成立後の正常経路（既存の別 wiring 呼び出しとは別行の additive 追加）と fallback boot 経路の両方へ NonSend リソースとして挿入する
  - prepare 失敗時（ダミー窓経路）は挿入しない（従来どおり永続結線なし）
  - 観測可能な完了条件: 起動後の World から `PersistWiring` リソースが取得でき、DragEnd 経由の保存が実際にファイルへ反映されることが結合テストで確認できること
  - _Requirements: 1.9_
  - _Depends: 2.1, 3.1_
  - _Boundary: main.rs_

- [x] 7. Integration: ghost boot() の設定注入と署名波及の追随
- [x] 7.1 boot() での起動記録読取・KanadeConfig 注入・SET sink 登録
  - sylphya spawn 後・`spawn_kanade` 前に `areka.boot.count` の存在（数値解釈はしない）で `first_boot` を決め、`areka.vanish.count` を寛容 parse して `vanish_count` を求める
  - `first_boot=true` のとき `first_boot_epilogue` を `PersistKey::BootCount.to_canonical_key()` を用いた `areka.prop.set` の `EpilogueCommand` として構築し `KanadeConfig` へ注入する（kanade は sylphya 非依存のまま正準 key を不透明搬送する）
  - `PropSetCueSink` を `options.sinks` へ登録し、wired／fallback 両起動経路が同一登録点を通ることを確認する
  - 観測可能な完了条件: 起動記録なしの ghost root では `first_boot=true` かつ epilogue 付きで kanade が構築され、起動記録ありでは `first_boot=false` で構築されることがテストで確認できること
  - _Requirements: 3.1, 3.4, 4.1, 4.2, 6.3_
  - _Depends: 4.1, 4.3, 5.1_
  - _Boundary: areka-ghost/runtime.rs_

- [x] 7.2 `on_first_boot` 署名変更の波及追随
  - `events::on_first_boot` の署名変更に伴うコンパイラ捕捉箇所（spine_e2e_test の呼び出し点・アサーション）を機械的に追随させる
  - 観測可能な完了条件: 既存 fixture ghost（永続ファイルなし＝初回扱い）を用いた spine e2e テストが、新シグネチャのまま従来と同じ Reference0="0" で通過すること
  - _Requirements: 4.1_
  - _Depends: 5.2_
  - _Boundary: areka-ghost tests_

- [ ] 8. Validation: 決定論檻と実機サインオフ
- [x] 8.1 偽 Free アンカー DragEnd→保存値等価の檻
  - headless World に `Anchored(Anchor::Free)` のキャラ窓を合成し、DragEnd 駆動で保存 entries が wndproc 確定位置と値等価になることを固定する（emo2 は全スコープ Bottom のため実機観測不能・この檻が正本）
  - 観測可能な完了条件: Free アンカー窓の DragEnd 後、投函された entries の座標が確定位置と一致するテストが通ること
  - _Requirements: 1.1, 8.1_
  - _Depends: 2.2_
  - _Boundary: placement/persist.rs tests_

- [x] 8.2 保存→復元 往復値等価の統合檻
  - temp dir 上の実 `FsPersistIo` と headless World を用い、char/balloon DragEnd 駆動→保存→`load_restored_state`/`apply_restored_placements`→位置・オフセットが値等価で復元されることを確認する。無関係 key（他の永続値）が同居していても保存操作で破壊されないことも併せて確認する
  - 観測可能な完了条件: 一連の駆動後、復元された `ScopePlacement` が保存前にドラッグで確定した値と一致し、同居する無関係 key の値も不変であることがテストで確認できること
  - _Requirements: 8.1, 7.2_
  - _Depends: 6.1, 6.2_
  - _Boundary: placement/persist.rs tests_

- [x] 8.3 発火規律（DragEnd のみ書込）の統合檻
  - 自動再射影・resize・`move_window_to`（`\![move]` 消費者）を駆動した前後でストア内容がバイト不変であることを確認する
  - 観測可能な完了条件: DragEnd を伴わない一連の操作後、永続ファイルの内容が操作前と完全一致することがテストで確認できること
  - _Requirements: 1.9, 8.4_
  - _Depends: 6.2_
  - _Boundary: placement/persist.rs tests_

- [x] 8.4 終了時フラッシュの統合檻
  - `PersistWiring` の clone 送信端（実経路と同型）から複数回の保存投函を行い、`barrier()`／`close()` を経てアクター join 後にファイルが最終値と一致することを確認する
  - 観測可能な完了条件: barrier なしで投函した保存が、shutdown 系列（barrier→close→join）の後に確実にファイルへ反映されていることがテストで確認できること
  - _Requirements: 1.2, 8.1_
  - _Depends: 3.2, 6.2_
  - _Boundary: areka-ghost tests_

- [x] 8.5 完走時のみ記録の統合檻
  - 実 sakura `spawn_talk` を駆動し、epilogue 付き台本が完走したときのみ `PropSetCueSink` が発火して起動記録が書かれ、horizon 到達前に `Close` すると `CuePlayer::stop()` により SET が発火せずストアが不変のままであることを確認する
  - 観測可能な完了条件: 完走シナリオでは `BootCount` が書き込まれ、中断シナリオでは書き込まれないことの両方がテストで確認できること
  - _Requirements: 3.4, 8.1_
  - _Depends: 4.2, 4.3, 5.3_
  - _Boundary: areka-ghost tests, areka-sakura tests_

- [x] 8.6 spine_e2e_test の第 2 起動シナリオ（起動記録あり）追加
  - `BootCount` を事前に永続化した fixture ghost で起動し、OnFirstBoot が発行されず OnBoot から運行が始まることを確認する新規シナリオを追加する
  - 観測可能な完了条件: 起動記録ありの fixture 起動で OnFirstBoot 発行が観測されず、`boot_gate skip_first_boot` ログが出力されることがテストで確認できること
  - _Requirements: 3.1, 3.3, 4.1, 6.3, 8.3_
  - _Depends: 7.1, 7.2_
  - _Boundary: areka-ghost tests_

- [ ] 8.7 実機サインオフ（実 emo2・実 DPI≠96・マルチモニタ）
  - 絶対パス起動の実 emo2＋実 pasta.dll・実 DPI（≠96）・マルチモニタ環境で、`AREKA_APP_SMOKE_EXIT_MS` の有界 auto-exit と `RUST_LOG` grep により以下を人間のサインオフで確認する: (1) ドラッグ→終了→再起動→前回位置一致（キャラ・バルーン相対）、(2) 初回起動で挨拶完走後に `prop_set_cue applied` が記録され、2 回目以降は挨拶が繰り返されず `boot_gate skip_first_boot` が記録される、(3) 保存モニタ切断構成での再起動でゴーストが画面内に出現し、構成復帰後は元位置へ戻る
  - 観測可能な完了条件: 上記 3 点すべてが実機ログ grep と目視で確認され、サインオフ記録が残ること
  - _Requirements: 8.6_
  - _Depends: 8.2, 8.5, 8.6_

## Implementation Notes

- **[8.7 実機サインオフ検出③: 復元座標ずれ＝保存↔復元のクランプ非対称]**: char 保存欠落（検出②）修正後も **むらさきの立ち位置＋バルーンがずれる**。計測ログ（save/restore 各座標を info 出力・殿方提案）で確定: 保存側 `project_anchor`／BottomSnapPolicy は補軸 x を**クランプせず**、端付近（右端を数十 px はみ出す）へ置いた x をそのまま保存する一方、復元側 `project_restore` は**常に** x を `[wa.left, wa.right−w]` へクランプ→**復元でだけ内側へ寄る**（実機: 保存 x=3493・wa.right=3840・w=434 → 復元だけ 3406 へ。追従 balloon も char に追随してずれる）。修正（Option B・persist.rs 内）: `project_restore` のクランプを「アンカー射影後の char 矩形が work area と**全く交差しない（＝完全に不可視）ときのみ**」に限定。一部でも可視なら保存位置をそのまま用いる（Req5.3・保存↔復元 idempotent）／完全不可視（モニタ構成変化）のみ画面内へ寄せる（Req5.1）。新檻 `project_restore_bottom_partially_visible_keeps_saved_x`（実機値 3493 維持／4000→3406）＋実機再走で確認。**計測ログ**（`areka::persist::{save,restore,project}` target・info）は恒久追加（実機診断用・observability のみ）。
- **[8.7 実機サインオフ検出②: 復元座標ずれ＝DragEnd 時 DraggingState 消失による char 保存欠落]**: 実機で複数窓ドラッグ後に再起動すると **むらさき（scope 0）バルーンが復元でずれる**。原因: `on_char_drag_end` が保存を `policy_mapped_position`（`DraggingState` 依存）に gating しており、wintf dispatch が DragEnd ハンドラ実行**前**に `DraggingState` を落とす（複数窓で発生）と `None`→早期 return で**当該 char の WindowPos 保存を丸ごとスキップ**。一方 `on_balloon_drag_end` は char の `WindowPos.position` を独立に読んで balloon offset を保存するため、disk に `[window.0]` 欠落＋`[balloon-offset.0]` 存在の非対称が生じ、復元で char が既定へ→相対 balloon がそれに追従してずれる（Req1.6: 位置の単一真実源は char・balloon は char_pos+offset で導出）。修正: `policy_mapped_position` が `None` のとき char の**現 `WindowPos.position`（既に project_anchor 済み/wndproc 確定）へフォールバック**して保存を貫徹（両者不在時のみ skip）。発火規律（1.9）不変・DragEnd 観測点のみ。RED で実機 disk 形状（window.0 欠落）を byte 一致再現→GREEN。**上流の trigger**（wintf dispatch の DraggingState ライフサイクル）は input_events/ 隣接ゆえ別途調査候補だが、follow 層で「ドラッグした char は必ず保存」不変条件を担保するのが正しい修正箇所。
- **[8.7 実機サインオフが本物の欠陥を検出→修正]**: 実 emo2 初回起動で **Ghost スコープ永続 commit が全て Degraded（os error 3・NotFound）**——`<ghost>/master/profile/areka/` ディレクトリが不在で、`FsPersistIo::commit` の `File::create` が親を作らないため。sylphya は「M1 本番経路に永続書込呼出は無い」read-only 前提で完了しており、position-persist が初の永続書込を導入したことで「dir を誰が作るか」が結線とストアの狭間に落ちた。**決定論檻（8.2/8.4/8.6）はテスト側で dir を事前作成して穴を隠していた**。修正（開発者裁定＝両方・二重の安全網）: ① `areka-sylphya` `FsPersistIo::commit` が書込前に `create_dir_all(parent)`（全消費者・全スコープ堅牢化・RED/GREEN 檻付き）、② `areka-ghost` `boot()` が spawn 前に ghost profile root を `create_dir_all`（log-first・6.3）。**実機再走で確認**: Degraded 0・sylphya.toml 生成（`[boot] count="1"`）・初回 `prop_set_cue applied`・2回目 `boot_gate skip_first_boot`＋挨拶非反復。
- **[8.7 派生: 共有 test ghost の leak 再利用汚染]**: 永続が実際に効くようになった副作用で、`inproc_fixture::shared_test_ghost()`（固定パス `areka_shiori4_test_ghost_shared`・Drop 抑止で leak・再利用時に事前削除を省略）に初回挨拶完走テストが `[boot] count` を書き込み、**cargo 実行を跨いで生存**→後続 i1/i2 が2回目扱いで挨拶スキップ→hang（60s timeout）。i1/i2 は「shared_test_ghost は永続ファイルを持たない＝常に初回扱い」を明示依存していた。修正: `shared_test_ghost()` が**毎呼出しで areka profile dir を除去**し初回扱いを回復（ゴースト構造の hardlink 再利用は温存）。
- **[7.2] 5.2 に内包済み**: `on_first_boot` 署名変更の spine_e2e 波及追随（3 呼出点・Ref0="0" 維持）は、署名変更コミット（5.2）でワークスペース全体のコンパイル緑を保つため機械的に同時実施済み（commit 063b171）。7.2 は追加実装なしで完了条件（新署名 fixture ghost で spine e2e が Ref0="0" 通過）を充足。検証: `cargo test -p areka-ghost --test ghost spine_e2e -- --test-threads=1` → 14 passed。
- **[3.1] サブエージェントが main リポジトリへ leak する罠（ハーネス quirk #3/#11 再演）**: 3.1 の implementer が worktree ではなく main リポジトリ（`C:\home\maz\git\areka`）の `runtime.rs` を編集し、worktree は clean のまま・変更が main に落ちた（過去タスクでも `persist.rs`・`input_events/balloon.rs` が main に untracked で leak 済みだった）。復旧: main→worktree へ該当ファイルを cp（base 一致を diff で確認）→ worktree でテスト緑を確認 → `git -C <main> restore` ＋ leaked untracked を rm。**予防**: implementer/reviewer へ「cd するな・main を触るな・絶対パスは必ず `.claude\worktrees\<name>` を含めよ・終了時に `git -C <main> status` が clean かつ自分の変更が worktree の `git status` に出ることを確認せよ」と厳命する。親は各タスク後に必ず worktree と main 双方の `git status` を検証する。

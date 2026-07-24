# ギャップ分析: areka-P0-position-persist（2026-07-24 全面改訂＝sylphya 着地後の再実測）

> 対象: 再確定済み requirements.md（2026-07-24 改訂・R1〜R8「永続ストア消費者」への再切削版）／steering（roadmap 追記㊵/㊹/㊺・W4 編成）。言語 ja。
> **本書は旧ギャップ分析（2026-07-16/17 生成・pre-sylphya コードベース対象）の全面書き換え**である。旧版の「永続ストア（GhostState・原子的 IO・寛容読取）を本 spec が新設する」前提は、W3 `completed/areka-P0-sylphya`（PR #84）のマージで**陳腐化**した——永続ストアは統一プロパティシステム sylphya の永続バッキングとして実装済み・檻込みで `main`（b0de116）に存在する。本書は現行コードベース（choice-render PR #85・seriko-loop・wintf-gpu-test-crash PR #87 マージ済み）に対する実測でギャップを引き直す。旧版の要件ディスカッション決定（3 件）は要件本文（R1.1-1.3/R1.9/R2.2/R5.4）へ反映済みのため、決定事項として引き継ぐ。
> 目的: 確定要件と既存コードベースの差分を洗い出し、設計フェーズへ材料を渡す。**決定はしない**（選択肢と論点の提示）。

## 分析サマリ（要点）

- **永続ストアは完全に実物**: 4 key 族 typed enum（`PersistKey::WindowPos/BalloonOffset/BootCount/VanishCount`）・正準 key（`areka.window.scope(N).x` 等）・TOML 物理形式・原子的 commit（temp→rename）・寛容読取・スコープ分離・read-modify-write マージ（無関係 key 温存）が `crates/areka-sylphya/src/persist/mod.rs` に檻込みで存在する。書き口＝`SylphyaPublisher::persist_put`（actor.rs:481・write-through）、読み口＝boot 時一括ロード→鏡像投影（`build_initial_image` actor.rs:563）→`SylphyaReader::resolve_dotted_str`（reader.rs:90）。**本 spec が書くべき「ストア」コードはゼロ**——旧版の軸B（直列化形式）・保存先・原子性・ghost 識別キーの論点はすべて解消済み。
- **不在なのは消費者の結線のみ**: (1) ドラッグ確定（`on_char_drag_end` follow.rs:319／`on_balloon_drag` follow.rs:443）→`persist_put` の保存結線、(2) 起動時の復元値→初期配置注入（`spawn_ghost_windows` spawn.rs:146 の `placements` 引数は健在）、(3) OnFirstBoot ゲート（kanade `on_prefetch_reply` boot.rs:131-183 は**無条件に** `events::on_first_boot` を発行）、(4) vanish 読取経路（`events::on_first_boot` events.rs:91 は Ref0="0" 固定）、(5) 復元位置の作業領域外→`project_anchor`（follow.rs:143）再射影の boot パス、(6) 終了時フラッシュの確認。
- **最大の構造ギャップ＝起動順序**: main.rs は `open_startup_window`（:309・窓配置）→`wire_emo2_boot`（:316・この内部で `areka_ghost::boot`→sylphya spawn＋永続ロード）の順で走る。**窓復元値が必要になる時点で sylphya はまだ存在しない**。復元値の読取時機（先読み／boot 前倒し／spawn 後移動）が設計の最優先論点（§3 軸A）。
- **保存結線の到達経路も未開通**: `SylphyaPublisher` は `GhostRuntime` の private フィールド（runtime.rs:154）で、UI World（follow.rs のドラッグハンドラ）から届く公開経路が無い。`GhostRuntime` へのアクセサ追加（additive）＋World リソース挿入の設計が要る（§3 軸B）。
- **W4 並走契約は本 spec に有利**: `spawn.rs`＝pos 単独所有（roadmap W4 事前割当）だが、実測では placements 注入は**引数経由**ゆえ spawn.rs 本体の改変は最小〜ゼロで済む見込み。触ってはならない面＝`measure.rs`（emo-dpi-scaling）・`input_events/mod.rs`（choice-interact）。main.rs の wire 近傍（:311-327）は chI と近接の可能性——additive 維持（§1.7）。

---

## 1. 現状調査（既存資産・パターン・2026-07-24 実測）

### 1.1 sylphya 永続層（本 spec が消費する契約の正本・`crates/areka-sylphya/`）

- **4 key 族 typed モデル**: `PersistKey`（persist/mod.rs:123-142）
  - `WindowPos { scope: u32, axis: X|Y }` → 正準 key `areka.window.scope(N).x|y`・TOML `[window."N"]`
  - `BalloonOffset { scope: u32, axis }` → `areka.balloon.offset.scope(N).x|y`・TOML `[balloon-offset."N"]`
  - `BootCount` → `areka.boot.count`・TOML `[boot] count`
  - `VanishCount` → `areka.vanish.count`・TOML `[vanish] count`
  - **値ドメインは文字列**（数値型ではない）。負値・複数スコープの往復檻あり（`round_trip_preserves_negative_and_multi_scope`）。
- **保存**: `save_scope`（persist/mod.rs:231）＝read-modify-write マージ（無関係 key 温存＝R7.2 は檻 `merge_preserves_unrelated_keys` で保証済み）＋`PersistIo::commit`（temp→rename 原子的確定・失敗は error!＋`PersistOutcome::Degraded`＝R6.2 保証済み）。
- **読取**: `load_scope`（persist/mod.rs:183）＝root 不在／ファイル不在／read 障害／破損／非数値スコープ ID すべて寛容縮退（R6.1 の「値なし」出現形は実装・檻済み）。
- **アクター経由の書き口**: `SylphyaPublisher::persist_put(scope, entries)`（actor.rs:481・fire-and-forget・write-through＝アクターが PersistPut 処理時に即 `save_scope`）。`barrier()`（actor.rs:491）＝同一送信端の投函全反映を待つフェンス（reply channel・無限待ち）。`close()`（actor.rs:508）＝**積み残し破棄**で即停止。
- **読み口**: `spawn_sylphya`（actor.rs:535）が起動時に全スコープを寛容ロードし正準 key で初期鏡像 `dotted_global` へ投影（`build_initial_image` actor.rs:563）→ `SylphyaReader::resolve_dotted_str(asker, "areka.window.scope(0).x")`（reader.rs:90・無待機・不在は `NotFound` 決定論縮退）。**正準 key 往復整合は檻済み**（`canonical_key_round_trips_with_to_canonical_string`）。
- **偽装境界**: `FakePersistIo`（故障注入 `fail_next_read`/`fail_next_commit` 付き）が公開されており、本 spec の結線檻（R8.1/8.2）でそのまま使える。

### 1.2 ghost 結線層（`crates/areka-ghost/`）——sylphya は boot 内で起動される

- **`boot(options)`**（runtime.rs:386）の順序: mount 解決 → `resolve_kanade_config`（:402）→ **sylphya spawn（:404-417）** → 静的 publish → shiori actor → **kanade spawn（:461・`resource_sink` 注入付き）** → dispatcher → ticker → `KanadeMsg::Boot`。
  - **重要**: `resolve_kanade_config` は sylphya spawn **より前**だが、config はローカル値ゆえ sylphya reader 取得後に追記フィールドを埋めてから `spawn_kanade` へ渡せる（順序制約は軽微）。
- **per-ghost 永続 root は確定済み**: ghost スコープ＝`<MountModel.shiori.dir>/profile/areka/`（`profile_areka_root` sylphya_wiring.rs:85）・ファイル＝`sylphya.toml`。**旧版の論点「保存先ディレクトリ・ghost 識別キー・fixture 汚染回避」は sylphya design で確定済み**（R7.1 のスコープ分離も同システムの契約）。
- **sink 注入の先例**: username prefetch の `make_username_resource_sink`（runtime.rs:457）＝kanade（純粋層）の Action を shell が sink 経由で sylphya へ反映するパターン。**boot 記録書込（R3.4）の結線に同型が使える**。
- **`GhostRuntime` の sylphya 端は private**（runtime.rs:154-158・`sylphya_publisher`/`sylphya_reader`）。公開アクセサは `kanade()`/`dispatcher()` のみ（:214-220）。`into_parts()`（:340）は消費的で shutdown と両立しない。**UI 側へ publisher/reader を渡す additive アクセサが Missing**。
- **shutdown**（runtime.rs:236）: kanade ForceQuit→join → dispatcher → ticker → shiori → relays → **最後に sylphya `close()`＋join（:319-326）**。mpsc は単一キュー FIFO ゆえ、close 投函以前に投函済みの PersistPut は処理（＝commit）されてから停止する——**write-through＋FIFO Close で「終了時フラッシュ」は構造的にほぼ担保**。明示確認（R1.2 の「最終確認」）をどう表現するかは設計論点（§3 軸E）。

### 1.3 窓配置・復元注入口（`crates/areka/src/placement/`）

- **注入口は健在**: `prepare_ghost_windows(ghost_root, balloon_root) -> PreparedPlacement`（mod.rs:145・同期 IO・COM 初期化済みスレッド前提）→ `spawn_ghost_windows(world, placements: &[ScopePlacement], titles)`（spawn.rs:146-248）が placements（物理 px）を `WindowPos` へ転記。**初期位置を外から与える口＝`placements` 引数**は不変。work_area 注入版 `prepare_ghost_windows_with_work_area`（mod.rs:162・決定論テスト用偽装境界）も健在。
- **`ScopePlacement`**: `scope`／`char_pos`／`char_size`／`balloon_pos`／`balloon_size`／`balloon_offset`／`anchor`。anchor は毎起動 `resolve_placement`（resolver）で config から解決＝**R1.8「アンカーは永続化せず毎起動再解決」は現行構造のまま成立**。
- **「ghost.dat 不使用」檻は健在**: `prepare_never_reads_or_writes_ghost_dat`（mod.rs:497-565）＝(a) prepare が ghost.dat を書かない (b) plant しても出力不変。**注意——旧版は「本 spec がこの檻を反転する」と計画したが、再切削後は反転不要の可能性が高い**: ストアは sylphya.toml（別ファイル・profile 配下）であり、復元 merge を prepare の外（main シーム側の純関数）に置けば「prepare は永続を読まない」という檻の精神は**存続できる**（§4-6）。
- **スコープ逆引き**: `CharWindowMarker { scope }`（spawn.rs:72）／`BalloonWindowMarker { scope }`（spawn.rs:80）が窓 entity に付与済み＝ドラッグハンドラから保存 key の scope を導出できる。

### 1.4 ドラッグ観測点・再射影資産（`placement/follow.rs`）

- **保存トリガ観測点**（R1.1/R2.1）: `on_char_drag_end`（:319・非 Free アンカーのキャラ窓の最終確定位置——`policy_mapped_position` で射影済み座標 `mapped` が確定した直後が観測点）／`on_balloon_drag`（:443・`BalloonFollow.offset = balloon_pos − char_pos` 更新直後が観測点）。**Free アンカーのキャラ窓は `OnDragEnd` が結線されない**（spawn.rs:230-234・非 Free のみ）＝Free 窓の最終位置観測点は現状 `on_char_drag`（wndproc 移動を読むだけ）しかない——**Free 窓の DragEnd 観測ギャップ**（§4-4）。
- **バルーン offset はセッション内のみ**: `BalloonFollow.offset`（:225-230・キャラ窓**左上**基準・物理 px）。doc に「永続化 ghost.dat は M-life の領分」（:218-223）＝本 spec 宛申し送りは現行も生きている。**R2.2 のアンカー辺基準への変換（save: 左上基準→下端基準／restore: 逆変換）は新規純関数**が要る。
- **再射影の既存正本**: `project_anchor(anchor, raw, size, snapshot)`（:143・5 アンカー射影・Free は identity・非正寸/snapshot 不在は identity 縮退）。`MonitorSnapshot`（:815・`from_monitors` :825）／`work_area_for_window`（:852・窓中心→帰属モニタ work area の純関数）。**R5（作業領域外→アンカー再射影・吸着維持）は既存資産の boot パス再利用で成立**——現状の消費者は drag/resize のみで、復元パスの結線だけが Missing。
- **復元後の適用 API**: `move_window_to`（:502・物理 px・BalloonFollow 随伴・公開）／単一ライター反映口 `enqueue_window_set_pos`（:729）。
- **UI スレッド契約**: follow は `&mut World` のみで完結（channel/actor 型を持たない）。保存結線は World リソース経由で publisher に触る形になる（`SylphyaPublisher` は `Clone`＝内部 `Sender` clone・fire-and-forget は非ブロッキングゆえ UI スレッドから投函可・ファイル IO はアクター側スレッドで実行される——**UI スレッド上の同期 IO を持ち込まない**のが現行構造の利点）。

### 1.5 kanade boot 運行（`crates/areka-kanade/`）

- **現行 cascade**: `Idle→BootInit(OnInitialize NOTIFY)→BootPrefetch(username GET＝R3.5 の「起動時リソース照会」段)→BootType(OnFirstBoot GET)→BootMain(OnBoot GET)→BootVersion(basewareversion NOTIFY)→Steady`（schedule/boot.rs）。**`on_prefetch_reply`（boot.rs:131-183）が prefetch 応答後に無条件で `events::on_first_boot` を発行**（:180）——初回判定分岐が存在しない（R3.1/3.3 の Missing 本体）。204 フォールスルー（BootType 204→OnBoot・:75-79）は既存実装＝R3.2 は不変で満たせる。
- **Ref0 固定値**: `events::on_first_boot(snapshot)`（events.rs:91-97）＝Ref0=`"0"` 固定。doc（:86-90）に「永続化は position-persist の領分」の申し送り。**R4 は署名変更（vanish count 引数化）**を要する。pub 面（DD-9 例外）ゆえ波及は実測 5 箇所: boot.rs:180（発行点）・events.rs:311 檻・kanade tests（boot_test.rs:96・full_run_test.rs:99）・areka-ghost spine_e2e_test.rs（780/1559/1901）——**機械的・コンパイラ捕捉の範囲**。
- **構築時注入の器**: `KanadeConfig`（msg.rs:177-186・`shell_name`/`baseware_version`/`baseware_name`/`close_talk_deadline_ms`）＝`spawn_kanade(config, …)` で move 保持され全 step へ参照渡し。**`first_boot: bool`＋`vanish_count` の additive フィールドの自然な置き場**。`KanadeConfig::new` の既定を「first_boot=true・vanish=0」にすれば**既存決定論テスト資産（boot.rs の happy path 檻群）は無改変で緑を保てる**。
- **boot 完了の観測点**: `BootVersion + Notified → Steady`（boot.rs:97-107・`boot_complete` info ログ）。R3.4「初回起動完了で起動記録を書く」の kanade 側フックはここ（または初回ゲート判定直後の eager 書込＝§3 軸C で比較）。kanade は純粋層（IO 不可）ゆえ、書込は Action→sink（username `ResourceOutcome` sink の同型・boot.rs:176-178 先例）で shell へ運ぶのが既存パターン。

### 1.6 エントリポイント・順序・終了（`crates/areka/src/main.rs`）

- **順序（現行実測）**: `open_startup_window(&app, &cfg)`（:309）→ `wire_emo2_boot(...)`（:316・内部で `areka_ghost::boot`＝sylphya spawn＋永続ロード）→ fallback 経路も `areka_ghost::boot`（:335）→ `app.run()`（:361）→ ①loop ticker Close（:375）→ ②`runtime.shutdown(CloseReason::User)`（:394-401）→ ③seriko join。
  - **含意**: 窓復元（open_startup_window 内）時点で sylphya 未起動。**「誰がいつ永続値を読んで placements へ merge するか」が本 spec 最大の設計判断**（§3 軸A）。
  - `MonitorSnapshot::from_monitors` の Resource 挿入は open_startup_window 内（:554-558）＝復元時再射影（R5）の snapshot は同関数内で取得可能。
- **終了フラッシュ口**: `runtime.shutdown`（:394）が唯一の正常終了経路（全窓 close funnel→run() 復帰）。smoke 有界 auto-exit（AREKA_APP_SMOKE_EXIT_MS）も同 funnel を通る＝実機サインオフの決定論判定（R8.6）と整合。

### 1.7 W4 並走契約と編集面（roadmap 追記㊵/㊹/㊺・2026-07-24 実測）

- **W4 同居**: `position-persist ∥ choice-interact ∥ emo-dpi-scaling`。事前割当（roadmap W4 行）:
  - **pos＝`spawn.rs` 単独所有**（:139-251→現 :146-248）。ただし実測では placements 注入は引数経由ゆえ spawn.rs 改変は最小〜ゼロ見込み（保存 key の scope は既存 marker で逆引き可・§1.3）。所有権は「他 2 spec が触らない」保証として活きる。
  - **pos が触ってはならない面**: `measure.rs`＋emo-atlas/compose/present＋wintf（dpi の面）／`input_events` モジュール＋emo-text 幾何（chI の面）。
  - **近接注意**: main.rs の wire 近傍（:311-327）は chI がポインタ配線 donor（input-events 由来の `wire_mouse_input` :325）を拡張する可能性——本 spec の main.rs 編集（復元 merge・publisher の World 挿入・flush）は**別行・additive** を維持。`emo2_boot/mod.rs` は座標系外（触るなら要調整）——**触らない案を優先**（§3 軸A の比較観点）。
  - kero-balloon（W5）×pos＝実測 DISJOINT（roadmap 追記㊹）だが「バルーンオフセット永続 × kero windowposition」は design 時の調整事項として記録あり。
- **kanade 面**: W4 内で kanade を触るのは pos のみ（chI の ChoiceSelection は W5 se が消費）。`events.rs`/`boot.rs`/`msg.rs` の additive 変更は衝突リスク低。
- **GPU テスト**: 本 spec の檻は headless（placement 純関数・kanade 状態機械・sylphya FakePersistIo）で完結する見込み＝**共有 GPU fixture（`on_gpu_owner_thread`）への依存なし**。万一 areka bin（spine.rs 檻域）へ GPU world 生成テストを足す場合のみ、wintf-gpu-test-crash と同型のオーナースレッド委譲（ローカル fixture 複製・roadmap W5 注記）に乗せること——現時点でその必要は認められない。

### 1.8 テスト規律・既存檻の棚卸（obsolete-vs-broken-test-policy）

| 檻 | 場所 | 本 spec での扱い |
|---|---|---|
| `prepare_never_reads_or_writes_ghost_dat` | placement/mod.rs:497-565 | **要判断**（§4-6）: 復元 merge を prepare の外に置くなら檻の精神（prepare は永続を読まない）は存続＝doc 更新のみ。prepare 内へ組み込むなら新契約へ書換え |
| `on_first_boot_is_get_with_fixed_zero_ref0` | kanade events.rs:311 | 署名変更に伴い「Ref0=vanish count 引数由来」の檻へ更新（vanish=0 で従来値と同値） |
| boot happy-path 檻群（無条件 OnFirstBoot 前提） | kanade boot.rs tests | `KanadeConfig` 既定＝first_boot=true で**無改変緑**を維持し、skip 分岐の新檻を additive 追加 |
| spine_e2e_test の OnFirstBoot 期待値 ×3 | areka-ghost tests:780/1559/1901 | 署名追随（機械的）。fixture ghost に永続ファイルが無い＝初回扱いで従来挙動不変 |
| sylphya persist 檻一式（往復・寛容・原子性・マージ・分離） | areka-sylphya | **不触**（R8.1 但書「永続ストア自体の檻は sylphya 側で保証済み」） |
| `BalloonFollow` セッション内記憶檻 | follow.rs tests | 永続化の持ち上げ後も in-session 恒等式は不変（左上基準のまま）＝無改変見込み。変換純関数は新檻 |

---

## 2. 要件→資産マップ（ギャップ種別: Missing / Unknown / Constraint）

| 要件 | 既存資産（実装済み・不触で消費） | ギャップ |
|---|---|---|
| **R1 窓位置の保存/復元** | `persist_put`＋`save_scope`（即時 write-through・原子的確定）／`spawn_ghost_windows` placements 注入口／`on_char_drag_end` 観測点／`resolve_placement` フォールバック | **Missing**: DragEnd→entries 構築→persist_put の結線・publisher の World 到達経路・復元値読取→placements merge 純関数・読取時機（軸A）。**Unknown**: Free アンカー窓の DragEnd 観測（OnDragEnd 未結線・§4-4）。**Constraint**: R1.9＝保存発火は DragEnd 系のみ（resize/再射影経路は発火禁止） |
| **R1.2/1.3 耐久性（即時確定＋終了フラッシュ）** | write-through（PersistPut 処理＝即 commit）／shutdown の FIFO Close（積み残しは close 投函前の put を処理してから停止） | **Missing**: 「フラッシュ完了」の明示確認の表現（barrier か FIFO 順序保証への依拠か・軸E）。barrier は無限待ち＝有界化の要否 |
| **R2 バルーン相対オフセット** | `BalloonFollow.offset`（左上基準・in-session）／`on_balloon_drag` 更新点／`ScopePlacement.balloon_offset` 復元注入口／`BalloonOffset` key 族 | **Missing**: アンカー辺（下端）基準⇄左上基準の変換純関数（save/restore 双方向・char_size.h 使用）と変換の置き場。Free アンカー時の基準規約（§4-5） |
| **R3 OnFirstBoot ゲート** | boot cascade の全 Phase・204 フォールスルー・`KanadeConfig` 構築時注入・sink 先例（username）・`BootCount` key | **Missing**: `on_prefetch_reply` の first_boot 分岐（skip→OnBoot 直行）・`KanadeConfig` additive フィールド・`resolve_kanade_config`（または boot() 内）での BootCount 読取→注入・起動記録の書込結線（タイミング＝軸C）。**Constraint**: R3.5＝prefetch 段（BootPrefetch）は不変・既定値で既存檻を緑のまま保つ |
| **R4 vanish Ref0** | `VanishCount` key・reader 経由読取 | **Missing**: `events::on_first_boot` の vanish 引数化（pub 波及 5 箇所・機械的）・boot() での読取→config 注入。**Constraint**: M1 実値は常に 0（増分源は M2） |
| **R5 画面内縮退** | `project_anchor`（5 アンカー・吸着維持・寛容縮退）・`MonitorSnapshot`／`work_area_for_window` | **Missing**: 復元パスでの再射影結線（merge 後・spawn 前に snapshot を通す）のみ。**Constraint**: R5.4＝再射影結果を書き戻さない（保存値原本保持） |
| **R6 消費側の寛容縮退** | `load_scope`/`save_scope` の全縮退アーム＋ログ檻（sylphya 側で R6.1/6.2 の下半分は保証済み） | **Missing**: 消費側の縮退＝値なし→既定位置解決／初回扱い／vanish 0 への分岐と、その warn/error ログ・**文字列→i32/u32 の寛容 parse**（非数値は「値なし」扱い＋warn・§4-3） |
| **R7 ゴースト単位スコープ** | `profile_areka_root`（per-ghost 物理分離）・`PersistScope::Ghost`・スコープ分離檻・マージの無関係 key 温存檻 | **ほぼ充足**: 本 spec は Ghost スコープの 4 key 族のみを読み書きする規律（コードレビュー事項）だけ |
| **R8 受入検証** | `FakePersistIo`・`prepare_ghost_windows_with_work_area`（work_area 偽装）・kanade 純粋 step・in-crate test 慣行（areka は bin-only） | **Missing**: 結線の檻（往復値等価・縮退・再射影・ゲート・原本不変・オフセット寸法不変）。**Constraint**: R8.6 実機サインオフ（実 emo2・実 DPI≠96・マルチモニタ・絶対パス起動）必達 |

---

## 3. 実装アプローチの選択肢

### 軸A. 復元値の読取時機と読取者（最優先・起動順序ギャップの解消）

**Option A1（先読み・placement シームでストアを直接ロード）**: `open_startup_window` 内（prepare 後・spawn 前）で、ghost root から `<shiori.dir>/profile/areka/sylphya.toml` を `areka_sylphya::persist::load_scope`（pub・寛容）で直接読み、placements への merge 純関数を通す。boot 時に sylphya が同じファイルを再ロードする（二度読み・起動時のみ・書き手不在区間ゆえ一貫）。
- ✅ 既存の起動順序（placement→wire_emo2_boot）を一切崩さない・emo2_boot 不触＝W4 近接リスク最小。
- ✅ `load_scope`＋`FakePersistIo`（または実 temp dir）で merge の決定論檻が素直。
- ❌ shiori.dir（= ghost/master）の解決が必要——mount 解決は boot 内。ghost root からの導出ヘルパ（areka-ghost に pub の薄い関数を足す or areka 側で `ghost_root/ghost/master` 慣行を直接組む）が要る。前者が正（mount 規則の二重化を避ける）。
- ❌ 「読み口 1 本化（sylphya 経由）」の理念からは例外的な直読みになる（ただし読むのは sylphya 自身の pub 関数＝形式・寛容性は単一実装を共有し、鏡像経由でないだけ）。

**Option A2（boot 前倒し・reader 経由で読む）**: `areka_ghost::boot` を `open_startup_window` より前へ移し、`GhostRuntime` の reader（要 additive アクセサ）から `resolve_dotted_str` で復元値を読む。
- ✅ 読み口 1 本化が完全（鏡像経由）・二度読みなし。
- ❌ **起動シーケンスの大改造**: wire_emo2_boot は「UI 基盤・起動窓の後」設計（DD-7）で、boot は wire の内部にある。boot だけ切り出して先行させると wire の 5 トラック結線・fallback 意味論（:328-357）へ広範に波及＝W4 で main.rs/emo2_boot を大きく触る——並走契約リスク大。
- ❌ 窓が無い時間に ghost が喋り始める順序問題（OnFirstBoot talk が窓生成前に走る）等、boot 系列と UI の暗黙順序を再検証する負担。

**Option A3（spawn 後移動・boot 完了後に `move_window_to` で復元適用）**: 既定位置で spawn→wire_emo2_boot 後に reader から読んで `move_window_to`（follow.rs:502・バルーン随伴付き）で移動。
- ✅ 順序改造なし・merge 関数不要・既存公開 API のみ。
- ❌ 窓は WS_VISIBLE で生成される（spawn.rs:254）ため**既定位置で一瞬表示→ジャンプ**の見た目劣化（体裁 spec としては本末転倒の恐れ）。R1.4「既定位置解決に優先して配置」の解釈も苦しい。
- ❌ バルーン offset 復元は `BalloonFollow.offset` の書き換えも要り、適用面が二枚になる。

> 推奨検討: **A1 が本命**（順序不変・並走契約安全・檻が素直）。A2 は理念純度は高いが W4 でやる改造ではない（やるなら独立 spec 級）。A3 は縮退案として design に併記。**A1 採用時の shiori.dir 導出ヘルパの置き場（areka-ghost の pub 薄関数）を design 冒頭で確定**。

### 軸B. 保存結線——publisher の World 到達経路と entries 構築

- **到達経路**: `GhostRuntime` へ `pub fn sylphya(&self) -> &SylphyaPublisher`（additive・`kanade()` と同型）を追加し、main.rs の wire 成立後（:323-326 の `wire_mouse_input` と同位置・同型）に publisher clone を World リソースとして挿入する。`SylphyaPublisher` は `Clone`（内部 `Sender` clone）で、`Sender<T>` は Send+Sync（Rust 1.72+）＝通常 Resource で挿せる見込み（不可なら NonSend・wintf の `MouseWiring` 先例あり）。fallback boot 経路（:335）でも同様に挿す。
- **entries 構築**: DragEnd 確定座標＋`CharWindowMarker.scope`（char）／`on_balloon_drag` の offset 更新値＋逆引き済み char の scope（balloon）から `Vec<(PersistKey, String)>` を組む**純関数**（i32→文字列化）。バルーンは R2.2 の基準変換（下端基準 y = 左上基準 y − char_size.h の系・正確な式は design で確定）を挟む。
- **発火規律（R1.9）**: 発火点は `on_char_drag_end`（:319 の `mapped` 確定後）と `on_balloon_drag`（:443 の offset 更新後）**のみ**。`on_char_drag`（ドラッグ中・毎イベント）・`resize_window_to`・`move_window_to`（`\![move]` 消費者）・再射影は発火しない。resource 不在（フォールバック未挿入）時は debug ログ＋no-op（縮退）。
- **代替案（保存を actor 側で束ねる）**: World には「保存要求チャンネル」だけ挿し、別スレッドで entries 化——不要な複雑化（fire-and-forget 投函は非ブロッキングでファイル IO は既に sylphya アクター側）。採らない方向で比較のみ記載。

### 軸C. 初回ゲート＋vanish 注入と起動記録の書込タイミング

- **注入**: `KanadeConfig` へ additive フィールド（案: `first_boot: bool`＝既定 true・`vanish_count: u32`＝既定 0——**既定値で現行挙動不変＝既存檻無改変**）。値源は `boot()` 内で sylphya reader（spawn 直後・:417 以降）から `areka.boot.count`／`areka.vanish.count` を `resolve_dotted_str` で読み、寛容 parse（非数値→既定＋warn）して config へ焼き込む。
- **分岐**: `on_prefetch_reply`（boot.rs:171-183）の Action 構築を「`config.first_boot` なら `on_first_boot(vanish)` GET→`BootType`／偽なら `on_boot(config)` GET→`BootMain` 直行」へ。204 フォールスルー（:75-79）・prefetch 段（R3.5）は不変。
- **起動記録の書込タイミング（R3.4）**:
  - **Option C1（boot 完了 Action＋sink）**: `BootVersion→Steady` 遷移（boot.rs:97-107）で新 Action（例 `Action::BootCompleted`）を発行し、shell が sink（username sink 同型・runtime.rs:457 先例）経由で `persist_put(BootCount)`。「初回起動を**完了**したとき」の文言に忠実。kanade の Action 語彙が 1 つ増える。
  - **Option C2（eager・boot() 内で即書込）**: 初回判定直後に ghost 側で `persist_put(BootCount=1)`。kanade 無改変で最小だが、「boot 途中クラッシュでも次回は通常起動扱い」となり初回挨拶が一度も完走しない可能性（R3.4 の意図とズレ得る）。
  - **BootCount の意味論**: key 名は count——「毎起動インクリメント」か「初回のみ 1 を書く」か。ゲート条件は「値の存在」で足りる（R3.1/3.3）。増分は将来消費者（起動回数系イベント）向けの語彙だが M1 要件外＝過剰実装をしない線引きを design で確定。
- **署名変更**: `events::on_first_boot(snapshot)` → vanish count を受ける形（引数 or config 経由）。波及 5 箇所（§1.5）は機械的追随。

### 軸D. 保存座標の表現（旧版の最優先論点——大半が解消済み）

- sylphya の値ドメインは**文字列**で確定済み。残る決定は「何の数を文字列にするか」のみ:
  - **窓位置＝物理 px・仮想スクリーン絶対座標（i32・負値可）**が既定路線——`WindowPos`/`ScopePlacement` と同一通貨（変換ゼロ・2026-07-05 DPI 欠陥面を作らない）・R1.7「仮想デスクトップ一貫・プライマリ丸めしない」に合致・負値往復は sylphya 檻済み。
  - **モニタ識別子は保存しない**——復元時に `work_area_for_window`（窓中心の帰属モニタを live 算出）＋`project_anchor` で再射影すれば、モニタ引き当ての永続表現は不要（旧版 D1 の「識別子選定」論点は消滅）。R5.4 の原本保持と整合。
  - 論理正規化（旧 D2）は論理/物理混在リスクゆえ引き続き不採用が妥当。
- **Unknown（残件）**: DPI 追従（W4 同居 emo-dpi-scaling）が着地すると窓寸が k 倍で変わる——保存するのは**位置のみ**（寸法は毎起動採寸）ゆえ直接衝突しないが、下端吸着の y は寸法依存（R1.8 でアンカー辺は毎起動再導出＝吸収される設計）。バルーン offset の寸法不変基準（R2.2）が DPI 変動にも同じ理屈で耐えるかは design で確認（R8.5 の檻を「高さ変動」一般で書けば DPI 変動も同じ檻に入る）。

### 軸E. 終了時フラッシュ（R1.2）の表現

- **Option E1（FIFO 順序保証に依拠＋ログ）**: write-through＋「close 投函前の put は処理済み」という mpsc FIFO 保証（§1.2）を design で明文化し、shutdown 時は追加操作なし（sylphya close の既存段のまま）。フラッシュの「確認」は保存時の Saved/Degraded ログで代替。
- **Option E2（shutdown 直前に barrier）**: `runtime.shutdown` 冒頭（または main.rs :394 直前）で `sylphya_publisher.barrier()` を呼び、未処理 put の commit 完了を明示的に待ってから停止系列へ。R1.2 の「最終確認（フラッシュ）」に文字通り対応・観測可能（檻に入れやすい）。**barrier は無限待ち**（rx.recv）ゆえ有界化（recv_timeout 版の追加 or 呼び側タイムアウト）の要否が論点。
- ハイブリッド（E1 を正・E2 を檻/実機証跡用に限定）も可。いずれも「最後のユーザー意図位置を窓の現在位置から読み直して書く」ことは**しない**（R1.9/R5.4 の原本保持——flush は書込済み値の確定確認であって再収集ではない）。

---

## 4. 設計判断アイテム（要件ディスカッションへ供給）

1. **復元値の読取時機（軸A）**: A1（placement シームで `load_scope` 直読み・二度読み許容）を本命に、A2（boot 前倒し）・A3（spawn 後移動）と比較確定する。A1 採用時は shiori.dir 導出ヘルパ（mount 規則の二重化回避）の置き場と、`prepare_ghost_windows` の外に置く merge 純関数（復元値 ∪ 既定 resolver ∪ 再射影）の署名を確定。**最優先論点**。
2. **publisher の World 到達経路（軸B）**: `GhostRuntime::sylphya()` additive アクセサ＋main.rs での Resource 挿入（wired/fallback 両経路）。Resource か NonSend か（`SylphyaPublisher: Send+Sync` の成立確認）。resource 不在時の縮退規約（debug＋no-op）。
3. **文字列⇄数値の寛容 parse の置き場**: 消費側で `"1024"`→i32 の parse 失敗を「値なし」へ縮退（warn）する共通ヘルパ。窓位置（i32・負値可）・boot/vanish count（u32）の 2 種。sylphya は文字列を素通しする（不触）。
4. **Free アンカー窓の DragEnd 観測**【要件ディスカッション #1 で決着済み（2026-07-24）——(a) 採用】: 保存はアンカー種別を問わず全キャラ窓に適用する（吸着＝移動の制約／保存＝最終位置の記録、は別関心事。R1.1 に明記済み）。設計は Free 窓にも `OnDragEnd` を結線する（spawn.rs:230-234 の非 Free 限定を外す。ハンドラ内で Free は wndproc 確定位置を読むだけの保存専用アーム——`project_anchor` は Free で identity ゆえ射影段は自然に無害通過）。emo2（全スコープ Bottom）では Free 経路が実機観測できないため、檻は決定論テスト（偽 Free anchor の DragEnd→保存値等価）で必達とする。
5. **バルーン offset の基準変換（R2.2・要件決着済みの実装形）**: save `offset_anchor = f(offset_topleft, char_size)`／restore の逆変換の正確な式・変換の置き場（保存 entries 構築時 vs 復元 merge 時の対称位置）・Free アンカー時の基準（アンカー辺が無い場合は左上基準に縮退か）。in-session の `BalloonFollow.offset`（左上基準）は不変に保つ（既存 consumer 無改変）。
6. **`prepare_never_reads_or_writes_ghost_dat` 檻の去就（obsolete-vs-broken）**: A1 採用なら「prepare は永続を読まない」は真のまま＝檻存続（doc の 2.11 参照を現況へ更新）＋merge 関数側に新契約檻（plant→復元）を追加。prepare 内組込みなら檻反転。旧版の「反転する」計画を再切削後の構造で見直す。
7. **初回ゲートの kanade 注入形（軸C 前半）**: `KanadeConfig` additive（`first_boot`/`vanish_count`・既定＝現行不変）＋`on_prefetch_reply` の分岐（skip 時は OnBoot GET→BootMain 直行）。既存 boot 檻の無改変維持と skip 経路の新檻。
8. **起動記録の書込タイミング（軸C 後半）**: C1（boot 完了 Action＋sink・username 先例同型）vs C2（boot() 内 eager）。「初回起動を完了したとき」（R3.4）の完了定義（BootVersion 完了＝Steady 到達を推奨検討）と、BootCount の値意味論（存在ゲートのみ or 毎起動増分）。
9. **`events::on_first_boot` 署名変更の形**: vanish を明示引数にするか `KanadeConfig` を渡すか。pub 波及 5 箇所（boot.rs:180・events.rs:311・kanade tests ×2・spine_e2e ×3）の更新範囲確定。
10. **終了フラッシュの表現（軸E）**: E1（FIFO 依拠）vs E2（barrier 明示・有界化要否）。R1.2 の「最終確認」を檻でどう観測するか（例: FakePersistIo で「DragEnd n 回→shutdown→ファイル内容が最終値」）。
11. **復元時再射影の結線位置（R5）**: merge 純関数の内で `project_anchor` 相当を呼ぶか、merge 後の別段にするか。snapshot の供給（`MonitorSnapshot::from_monitors` は open_startup_window 内 :554-558）と work_area 偽装檻（`prepare_ghost_windows_with_work_area` の流儀）を restore パスへどう延長するか。R5.3（域内なら再射影しない＝そのまま）と R5.4（書き戻さない）の檻。
12. **W4 並走の編集面確定**: 本 spec の編集ファイル最終集合（見込み: follow.rs・placement 新モジュール（restore/merge）・mod.rs（doc/檻）・main.rs（additive）・areka-ghost runtime.rs/sylphya_wiring.rs（additive アクセサ＋config 注入）・kanade msg.rs/boot.rs/events.rs・各テスト）を design で確定し、`measure.rs`／`input_events/`／`emo2_boot/`（A1 なら不触）へ触れないことを明記。

---

## 5. 工数・リスク

| 領域 | 工数 | リスク | 一言根拠 |
|---|---|---|---|
| 保存結線（DragEnd→entries→persist_put・World リソース） | S–M | Med | 観測点・書き口とも実物。到達経路（アクセサ＋挿入）と R1.9 発火規律の檻が本体 |
| 復元 merge＋読取時機（軸A） | M | **Med–High** | 機構は純関数だが**起動順序の設計判断が本 spec 最大の分水嶺**（A1 なら Low 寄り・A2 なら High） |
| 画面内再射影の boot パス結線（R5） | S | Low | `project_anchor`/`work_area_for_window` 再利用のみ・檻も偽装境界既存 |
| 初回ゲート＋vanish（kanade） | S–M | Low–Med | additive config＋1 分岐＋署名波及 5 箇所（機械的）。既定値設計で既存檻無改変 |
| 起動記録書込（sink or eager） | S | Low | username sink 先例の同型（C1）か 1 行（C2） |
| バルーン offset 基準変換 | S | Med | 式は単純だが save/restore の対称性と Free 縮退の詰めが要る（R8.5 檻必達） |
| 終了フラッシュ表現 | S | Low | 構造的にほぼ担保済み・表現の選定のみ |
| 実機サインオフ（R8.6） | S | Med | 実 DPI≠96・マルチモニタ・絶対パス起動。座標は既存通貨のまま＝旧版 High から低減 |

**総合**: 工数 **M**（3-7 日）。リスク **Medium**——旧版の主要リスク 2 点（座標表現・ストア設計）は sylphya 着地で消滅し、残る山は**起動順序（軸A）**に一本化された。

---

## 6. Research Needed（設計フェーズへ持ち越す調査）

- **`SylphyaPublisher` の Send+Sync 成立確認**（Resource 化可否・不可なら NonSend 先例 `MouseWiring` に倣う）。
- **A1 の shiori.dir 導出**: mount 解決（`areka_parsers::package::resolve`）の部分再利用 or areka-ghost へ薄い pub ヘルパ——mount 規則の二重化を避ける最小形。
- **`barrier()` の有界化**（E2 採用時）: `rx.recv()` 無限待ちへの timeout 付与の要否・置き場（sylphya 側 recv_timeout 版 or 呼び側）。
- **ukadoc 裏取りは完了済み・design で再確認のみ**: OnFirstBoot「初回起動時に発生・204→OnBoot フォールスルー・Reference0＝vanish された回数」（2026-07-16/17 に MCP 裏取り済み・kanade 実装の 204 フォールスルーは正典どおり）。`balloon.offsetx/offsety` の per-surface 正典と M1 簡略化（スコープ単一 offset）は要件 Boundary Context で確定済み（M2 送り）。
- **kero-balloon（W5）との調整メモ**: バルーンオフセット永続 × kero `windowposition` の交差は roadmap 追記㊹で「design 時の調整事項」——scope 1 の offset key は同じ `BalloonOffset { scope: 1 }` で足りるかを design で一瞥。

---

## 7. 次ステップ

ギャップ分析は本改訂で現行 main（b0de116 ベース）へ再突合済み。次は要件ディスカッション（`/kiro-requirements-discussion` 相当・§4 の 12 論点を供給）→ `/kiro-design areka-P0-position-persist` で技術設計へ。design 冒頭で確定すべきは **#1 読取時機（軸A）**・**#2 publisher 到達経路**・**#7/#8 kanade 注入と書込タイミング**。本 spec は「新機構を作らず、実物のストアと実物の注入口を結線する」層に徹する（要件 Introduction と整合）。

> **旧版からの引き継ぎ（蒸し返さない決定・要件本文反映済み）**: 耐久性＝ハイブリッド（DragEnd 即時確定＋終了時フラッシュ・R1.1-1.3）／バルーン基準＝アンカー辺（下端）・左上基準禁止（R2.2）／保存値原本保持＝永続更新はユーザードラッグのみ・自動再射影は書き戻さない（R1.9/R5.4）。設計はこの「保存値＝ユーザーの意図／表示位置＝意図と物理制約の写像」二層分離を壊さないこと。

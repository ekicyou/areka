# Design Validation Report: areka-P0-dpi-transition-atomicity

- 実施日: 2026-08-15
- 対象: `design.md`（2026-08-15 生成・D1〜D16）／入力 `requirements.md`・`research.md` §7-10・`reobservation-2026-08-15.md`・`brief.md`（棚卸⑨ブロック）・`.kiro/steering/`
- 実施形態: 非対話（レビュー結果のみを記録。設計文書・要件文書は改変していない）
- 検証方法: design.md の file:line アンカーを現行ツリーで開いて突合（30 箇所以上・付録 A）、要件 R1〜R10 の全受入項目と設計コンポーネントの対応確認、Requirement 10 の境界（van／zorder／budget／wpl／scg・`show.rs:297-301` 不動）の突合、順序保証（5.8 一度書き）・合流規則（10.3）・ゲートの有界性（DpiSyncHold）・段階裁定（C8）の決定可能性の机上検証

---

## Design Review Summary

設計は要件 R1〜R10 の全受入項目を Traceability 表で 1 対 1 に写像しており、引用する file:line は抽出検査した 30 箇所以上がすべて現行ツリーと一致した（研究 §8.1 の再検証が実際に効いている）。境界（Requirement 10）も守られている——`show.rs:297-301` は不動、`finalize_chain_once_with` の一度きりは触らず別機構 `ChainRealign` で解き直す、Z 専用指令は合流対象外、`persist.rs` 無改変。一方で、観測基盤の中核である「フレーム番号の配管」と「機械判定の判定量」に、実装してから初めて気づく種類の穴が 2 つあり、5.8 の保証範囲も明記より狭い。いずれも設計判断 1〜2 項目の書き換えで閉じる規模であり、構造の作り直しは要らない。

---

## Critical Issues（3 件）

### 🔴 Critical Issue 1: フレーム番号のスレッド局所ミラーは、`Update` で走る `detect_display_change_system` からは読めない

- **Concern**: D1 は「World を持つ観測点も統一してミラー（スレッド局所）を読む」と定めるが、`monitor` レコードの発行点 `monitor_systems.rs:280-316` は `Update` スケジュールの通常 system（`monitor_systems.rs:195`・`NonSendMarker` なし）にある。`Update` は既定の多スレッド実行器のまま（`world/mod.rs:102`・`SingleThreaded` 指定は UISetup 以降の 6 本のみ）なので、この system はワーカースレッドで走り得る。ミラーは UI スレッドの `try_tick_world` が更新するため、ワーカーから読むと初期値（0）または別の値になる。zorder 維持系（`zorder_pair_maintain.rs:334-335` の `NonSendMarker`）や `emo2_frame_system`（排他 system）は UI スレッド固定なので影響しないが、**遷移の起点となる `kind=monitor` の frame だけがずれる**。
- **Impact**: C7 `split_transitions` は `kind=monitor` を起点に遷移を切り出し、`frames_to_last_write` は起点 frame からの差で計算する。起点 frame が壊れると 2.4「単一のフレーム番号系列で結合」と 4.4 の判定が成立せず、決定論テスト（`SingleThreaded` を明示するため緑）では見えず、実機（多スレッド）でだけ壊れる。
- **Suggestion**: World を持つ観測点は `FrameCount` 資源（と tick 開始時刻 `FrameTime`）から `Stamp` を組み、World を持たない点（flush・wndproc＝いずれも UI スレッド）だけがミラーを読むように D1 を改める（C1 の純関数は既に `Stamp` を値で受け取る設計なので API 変更なし）。ミラーをプロセス大域の atomic にする案もあるが、7.7（テスト間の状態汚染なし）と衝突するため推奨しない。加えて、テスト戦略に「`monitor` レコードの frame が `FrameCount` と一致すること」を多スレッド実行器のままで確かめる項目を 1 本足す（ログ捕捉に依らず、レコード純関数への入力値を検査する形で可能）。
- **Traceability**: 2.3, 2.4, 4.4, 7.6, 7.7
- **Evidence**: design.md「キー決定 D1」／C1 Implementation Notes／Modified Files 表 `monitor_systems.rs` 行

### 🔴 Critical Issue 2: 判定器（C7）に、実測された症状そのものを捉える量が無い——`TRANSITION_FRAME_BOUND = 0` は現行コードで既に満たされている

- **Concern**: 第 1 段再観測が確定した症状は「描画内容は +13〜47ms に新寸、窓矩形は +63〜309ms まで旧寸」＝**同一 tick の内側**で 50〜270ms の食い違いが続くことである（reobservation §4.1）。ところが C7 の判定量は tick 単位（`frames_to_last_write`・`mismatch_frames_per_window`）で、設計自身が「全窓の enqueue は同一 tick 内」（事実 2）と確定しているため、これらは**是正前のコードでも 0** になる。`t_us`／`call_us` は「参考値・判定語にしない」と明記されている。結果、8.3 が求める「Requirement 4 の各項目を観測出力から機械的に判定」は 4.2（食い違いフレームを可視化しない）について成立せず、目視（8.4）だけが頼りになる。C8 の段階裁定も Q2「目視と t_us で食い違い区間が残るか」が数量条件として未定義で、B-2b→B-4→B-3 の分岐が台帳の数量だけでは決められない。
- **Impact**: 実機サインオフの機械判定が、本仕様が直そうとしている症状を素通しで PASS にする。B-2a（合流）が実時間を減らさなかった場合に「是正済み」と誤認する経路が開く。
- **Suggestion**: 決定論テストの判定語（回数・フレーム差・接地点差）と、実機サインオフ専用の判定量（µs）を分けて両方を C7 に持たせる。具体的には `TransitionSummary` に窓ごとの `visualize_to_write_us`（同一 frame の `surface stage=visualize` の `t_us` から当該窓 `write` の `t_us` まで）と `flush total_us` を加え、`Bounds::signoff` に上限（例: 実測 vblank 周期 1〜2 回分。値は R3 の再採取で確定）を置く。この量は非決定なので回帰テストでは固定せず、サインオフ手順書（C10）の合否と C8 の Q2 条件にだけ使うと明記する。研究 §3(f) が既に「perf 行の合計と vblank 周期の比較」を補助量として挙げており、その復活で足りる。
- **Traceability**: 4.2, 4.4, 8.3, 3.1, 9.3
- **Evidence**: design.md C7「定数（回帰テストが固定）」／C8 表と Q2 分岐／Residual Risks 第 3 項

### 🔴 Critical Issue 3: `DpiSyncHold` は dpi 相しか止めないため、5.8「一度書き」の保証は経路 (a) では明記より狭い

- **Concern**: D15／C5 は「Hold なら `refresh_scale` も再導出も呼ばない」を dpi 相（`dpi_phase_with`）にだけ適用する。しかし `apply_show` は適用ごとに窓の `DPI` component から k を導く（`show.rs:72-73`）ので、保留中に drain 相の `ShowSurface`（SERIKO のまばたき等・会話中の表情差替）が来ると、新 k のサーフェスが可視化され `pending_resize` が立ち、ゲート外の `reconcile_reported_sizes`（`frame.rs:187`）が旧 snapshot で窓を書く。続くモニタ表更新で C6 の `WorkAreaResnap` が再度書く＝ゲートが防ごうとした「旧下端で 1 度・新下端で 1 度」の 2 段書込と、窓矩形と描画内容の食い違いがそのまま起きる。設計の決定論テスト「経路 (a): hold→解除→書込 1・旧下端の中間矩形なし」は dpi 相だけを回すので、この漏れは検出できない。
- **Impact**: 第 1 段では経路 (a) 先行は 0/6（`WM_DPICHANGED` は全件モニタ表更新の後・SetWindowPos の内側で受理）なので実害は限定的だが、設計は 5.8 を「保証」と書き、根拠テストも掲げている。保証の範囲が実装と食い違ったまま tasks へ進むと、テストが緑のまま実機で 2 段書込が残る。
- **Suggestion**: 設計討議で二者択一を裁定する。⑴ 保留中の窓は drain 相の `apply_show`／`reconcile_reported_sizes`／`resnap_shell_targets` でも k 再導出と窓寸 reconcile を見送る（保留の適用範囲を「当該窓への全ての k 依存書込」へ広げ、C5 の Contracts に明記）。⑵ 経路 (a) 先行は未観測（0/6）ゆえゲートは dpi 相限定の防御にとどめ、5.8 の保証は経路 (b) に限ると設計へ明記し、経路 (a) は観測基盤で監視する（`hold` レコード＋`write` 2 件を実機で数える）。どちらでも設計の主張とテストの範囲を一致させること。
- **Traceability**: 5.8, 4.1, 4.2, 7.3
- **Evidence**: design.md「キー決定 D15」／C5 Responsibilities「dpi 相での適用」／System Flows「経路 (a) が先行する場合」／Testing Strategy Integration 2

---

## Design Strengths

1. **観測点と判定を 1 系列・1 実装に寄せた構成**: `[transition] frame= t_us= kind=` の共通接頭語、wintf に置く単一 target、コマンドへの要求語彙タグ（`WriteTag`）で「要求（areka）と実施（wintf）を 2 段 grep で結ぶ」現行の手間を消し、判定は areka の純関数 `transition_judge` 1 本を決定論テストと実機サインオフで共用する。wintf←emo-present←areka の依存方向も守られている（`&'static str`＋数値だけを運ぶ）。
2. **一度きり確定の扱いが既存の意味を崩さない**: `ChainFinalized` は解除せず別機構 `ChainRealign`（武装＝k 変化を伴う `DpiReproject`・解決＝全スコープ landing かつ保留窓なし・停滞計数は武装時に初期化）で解き直し、既定位置の追跡はシステム由来 route かつ「書込前に既定位置と一致」のときだけ追随させる。これにより scg R7.3（明示再配置の除外）・R7.4（表情差替では解かない）を維持しつつ、実測 359px の隙間だけを閉じる形になっている。キーワード基本位置を再導出せず `balloon-offset-dpi` へ k 倍で従属させる裁定（D10）も、6.5 の「実質同一の問い」を正しく畳んでいる。

---

## Final Assessment

- **Decision: GO（条件つき）** — 上記 3 件を設計討議で反映してから `/kiro-spec-tasks` へ進む。
- **Rationale**: 既存アーキテクチャとの整合（tick 構造・観測流儀・依存方向・単一ライター）は保たれ、要件は全項目に対応先があり、境界（Requirement 10）も守られている。3 件の指摘はいずれも設計判断の書き換え（D1 の読み出し元／C7 の判定量と C8 の条件／D15・C5 の保証範囲）で閉じ、コンポーネント構成や File Structure Plan の作り直しを要しない。反映せずに tasks を生成すると、実装が緑のテストの下で実機だけ壊れる形になるため、条件は外さない。
- **Next Steps**:
  1. 設計討議で Critical 1〜3 を裁定し design.md へ追記（D1／C7・C8／D15・C5）。
  2. 併せて付録 B の非クリティカル所見を triage（採否だけ決めればよい）。
  3. `/kiro-spec-tasks areka-P0-dpi-transition-atomicity` へ。

---

## 付録 A: file:line アンカーの突合結果（抽出検査・すべて現行ツリーで一致）

| 設計の引用 | 確認結果 |
|---|---|
| `command.rs:116-125`（`SetWindowPosCommand` 7 フィールド）・:127-130（thread_local キュー）・:155-167（`enqueue`）・:173-206（`flush`）・:83（`guarded_set_window_pos`）・:40（`is_self_initiated`）・:210 | 一致 |
| `world/mod.rs:503-505`（`FrameCount` +1）・:213（`detect_display_change_system` 登録）・:102（`Update` 既定実行器） | 一致 |
| `tick_bridge.rs:181-202`（`tick_one_frame`・:200 flush） | 一致 |
| `frame.rs:155`（`emo2_frame_system`）・:168（`run_dpi_phase`）・:187（`reconcile_reported_sizes`）・:195（`resnap_shell_targets`）・:199（`finalize_chain_once`） | 一致 |
| `dpi.rs:232`（`dpi_phase_with`）・:242-252・:287-301・:335（`reproject_char_window_at_current_size`）・:124（`reconcile_window_size`） | 一致 |
| `show.rs:297-301`（upload エラー分岐）・:302（`Stage::Upload`）・:306・:313・:322・:328・:72-73（k 導出） | 一致 |
| `chain.rs:173`（`upload`・戻り `Result<(), PresentError>`）・:178-194（`ResizeBuffers`）・:224（`Present`） | 一致 |
| `work_area.rs:12-24`・:17（「セッション内固定」）・:31（`from_monitors`） | 一致 |
| `main.rs:530-538`（`boot_monitor_snapshot`）・:569（注記）・:573-574・:611（挿入） | 一致 |
| `drain_resnap.rs:294`（`finalize_chain_once_with`）・:299-301・:333・:338・:353（`collect_chain_states`・private）・:370・:388-394・:415-429 | 一致 |
| `chain_finalize.rs:48`（`ChainFinalized`）・:99（`finalize_chain`）・:108（既定位置判定）・:153（600 フレーム）・:159-165（`ChainFinalizeStall`）・:249（`note_chain_deferral`） | 一致 |
| `window_move.rs:42`・:158（`resize_window_to`）・:274-283・:288-289・:310-318・:342-350（手順 5a）・:366-374・:514-521（`enqueue_window_set_pos`）・:549（limit 関門）・:559（enqueue）・:569-577（bypass ミラー）・:606-611・:902 | 一致。flags は size あり `SWP_NOZORDER\|SWP_NOACTIVATE`、なし `+SWP_NOSIZE`（合流規則の前提と整合） |
| `zorder_pair_maintain.rs:187-207`（`pair_fix_command`・`hwnd_insert_after` は常に `Some`）・:475・:334-335（`NonSendMarker`） | 一致。Z 専用指令は `hwnd_insert_after == None` 条件で合流対象外になることを確認 |
| `graphics/systems/window_pos.rs:89-98` | 一致 |
| `window_proc/window_pos.rs:36`（`WM_WINDOWPOSCHANGED`）・:303（`WM_DPICHANGED`）・:352-363・:372-374（採否判定）・:407-430（同期書込）・:442 | 一致 |
| `lifecycle.rs:122-138`（`WM_DISPLAYCHANGE`・:134 `mark_display_change`） | 一致 |
| `monitor_systems.rs:195`・:218・:224・:253・:279-316・:380・:438-480（`redrive_window_dpi_for_updated_monitors`・窓中心で帰属） | 一致 |
| `monitor.rs:66-74`（`Monitor` component） | 一致 |
| `timing.rs:59-71`（`Stage`）・:91（`EmitContext`）・:158・:167・:191・:208 | 一致 |
| `refresh.rs:52`・:70-73（k 不変）・:74-77（不可視）・:91・:116 | 一致 |
| `diag.rs:69`（`DIAG_TARGET`）・:162（`PlacementRoute`・現在 `ALL` は 10）・:355-374・:383-392 | 一致（設計の 12 語＝10＋2） |
| `anchor.rs:105`（`project_anchor`）・:111・:118・:131 | 一致（実パスは `placement/follow/anchor.rs`） |
| `spawn.rs:237`・:266（`default_char_pos`）・:304-306・:514 | 一致（実パスは `placement/spawn.rs`） |
| `frame_test_support.rs:216`・:256・:328・:351・:406・:431・:439 | 一致 |
| `schedule_labels.rs:8`（`FrameCount(pub u32)`）／`app.rs:20-23` | 一致（実パスは `ecs/world/schedule_labels.rs`・`ecs/app.rs`。設計はファイル名のみ表記＝軽微） |

`SetWindowPosCommand::enqueue` の本番呼出は設計どおり 3 箇所（areka `window_move.rs:559`・zorder `:475`・graphics `:98`）。`crates/areka/examples/mock-shell.rs:403` も `new()` 経由で積むが `new()` の 7 引数は不変なので影響なし。

## 付録 B: 非クリティカル所見（設計討議で triage する候補・最大 6）

1. **D4 の `UploadOutcome` 導入は「:297-301 の字面不変」と両立しない**。`if let Err(e) = chain.upload(..)` を字面どおり残すと `Ok` の中身（`resized`）を受け取れない。代替: upload の**前**に `let prev = chain.size();` を読み、:306 の `size` と比べて `resized = size != prev` を得る。戻り値型も `chain.rs` も変えずに済み、C3 の接触面が減る。
2. **C6 の `WorkAreaResnap` を dpi 相の前に置くと、経路 (b) の遷移で同一窓に 2 度の enqueue（`WorkAreaResnap`＝旧寸・新下端 → `DpiReproject`＝新寸）が積まれ、`[diag.window_move]`・`ground` レコードも 2 件ずつ出る**（SetWindowPos は合流で 1 回）。資源の差替（`sync`）は dpi 相の前のままにし、再射影（`resnap_for_work_area_change`）だけを dpi 相の**後**へ回すと、`Changed<DPI>` の窓は dpi 相が新 snapshot で書き、残りは べき等 skip で 0 書込になり、レコードの重複と bypass ミラーの中間値が消える。
3. **`MonitorDpiTable::dpi_for_point` の帰属規則と `redrive_window_dpi_for_updated_monitors` の `monitor_containing` の規則が一致していること**を実装で明示する（前者は `work_area_for_window` と同じ「最近傍へのフォールバック」を持つ設計、後者は含有のみで非含有は skip）。食い違うと、どのモニタにも中心が乗らない窓で 30 フレームの保留が毎回起きる。同じ純関数を共有するのが簡単。
4. **Existing Architecture 表の事実 4「6 回の SetWindowPos が 1 tick 内」は構造からの推定**（フレーム番号は未採取）。設計 Overview は「再観測で確定した事実」として書いている。ラベルを「静的構造証跡（`emo2_frame_system` 1 回の排他 system で全 `Changed<DPI>` を処理）＋実機の時刻列」と分けて書くと、確定台帳の 2 証跡クラス運用（3.3）と揃う。
5. **`drain_window_pos_commands()` を `pub` にして「本番からは呼ばない」とする運用**は、`#[doc(hidden)]`＋doc の一文で意図を残すか、areka 側テストが要る事情（crate 境界）を C2 に書いておく。
6. **4.4「有界のフレーム数」の決定論値 `TRANSITION_FRAME_BOUND = 0` は現行コードで既に成立する**ため、7.3「是正前失敗・是正後通過」の対はこの量には存在しない（設計が挙げる対テストは合流・`WorkAreaResnap`・`ChainRealign`・hold の 4 つ）。L1（逐次 flush）の是正候補（B-2b 以降）を採る段になったら、その候補ごとの対テスト（または実機のみで測る旨）を C8 の表に列を足して明記する。

## 付録 C: 検証した観点と結論（要点）

- **R1〜R10 の被覆**: Traceability 表で全受入項目に対応先あり。6.4・9.2 は分岐非該当で正当。3.5 の引受先 `areka-P0-draw-load-parity` は `.kiro/specs/` 直下に brief 実在。
- **Requirement 10 の境界**: `show.rs:297-301` 不動（付録 B-1 の代替でより確実に）／`finalize_chain_once_with` の一度きり不変／`persist.rs` 無改変／`window_pos.rs:372-374` 採否判定不変／Z 専用指令（`hwnd_insert_after` が常に `Some`）は合流の対象にも先にもならず相対順不変・幾何指令は全て `SWP_NOZORDER` を持つため Z 結果も不変（10.3 成立）。
- **5.8 一度書き**: 経路 (b)（実測 6/6）は Update（モニタ表＋全窓 DPI 再導出）→ 同 tick FrameFinalize 先頭で snapshot 差替 → dpi 相が新下端で 1 書込、で成立。経路 (a) は Critical 3 のとおり保証範囲が狭い。
- **DpiSyncHold の有界性**: 上限 30 フレームで `ProceedAfterTimeout`（warn）へ抜けるため停止しない。表なし・帰属なしは即 `Proceed`。別モニタへのドラッグ（10.7）は表に既在の dpi へ移るので原則即通過。
- **観測基盤の既定 OFF・決定論性**: 単一 target・debug 水準・`tracing::enabled!` で守る。純関数レコード＋`Stamp` 値渡し・thread_local キュー・`drain_window_pos_commands` で x64・`SingleThreaded` の決定論テストが成立（Critical 1 の読み出し元の是正が前提）。
- **段階裁定の決定可能性**: A-1・B-2a は台帳数量（回数・静的構造）で決まる。B-2b の条件（`msg` が `write` の内側・`Σcall_us` が大半）は数量で決まる。B-4／B-3 の条件は Critical 2 の量を足して初めて数量になる。

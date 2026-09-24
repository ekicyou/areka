# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: ゴースト切替（台帳 #13）・シェルとバルーンの切替（#50）・インストール直後の切替（#15）を作る側。今日の areka は「1 プロセスに 1 回だけ起動して、終わったら終了する」形で組まれており、同じプロセスの中でゴーストを降ろして起こし直す単位が無い。

**今の状態**: ⑴ 終了順序が `fn main`（`crates/areka/src/main.rs`）の `app.run()?` の後ろにべた書きで、`app.run()` が失敗で戻ると終了順序を通らない。⑵「1 回だけスケジュールへ登録する」前提の結線が 8 か所あり、系（system）の登録とゴーストごとに差し替える状態が同じ関数に同居している（2 度呼ぶと系が二重に登録される）。⑶ 起動の結線（`wire_emo2_boot`）が `&WinApp` を取って中で World を借りるので、フレームの系の中からは呼べない。`open_startup_window` も「登録」と「窓を作る」が同居している。⑷ 全窓を消す部品（`app_exit.rs` の `despawn_app_windows`）は私有で、完了 `areka-P0-app-lifetime-separation` 要件 3.6 により単独では呼べない＝「窓を閉じるが終了しない」操作が無い。

**何を変えるか**: 利用者から見える振る舞いを 1 つも変えずに、構造だけを「1 回」から「n 回」へ広げる。終了順序を終了理由を引数に取る 1 つの関数へ、8 か所の結線を「登録（プロセスに 1 回）」と「ゴーストごとの状態の載せ替え」へ分け、起動の結線をフレームの系の外から `&mut World` で呼べる形にし、「全窓を閉じるが終了しない」操作を `app_exit.rs` に足す。切替の語彙とイベント・名前解決・kanade の握手・メニュー登記は持たない（#13・#50）。

> 起票: 2026-09-24 `/kiro-discovery` 再入（棚卸⑯・台帳 #58）。brief の事実は main `0b01f654` の起票時実測で、要件生成時（2026-09-24・main `5db3672a` 相当）に引き直した。

## Introduction

本仕様は、同じプロセスの中でゴーストを降ろして起こし直せる形にするための**構造の括り出し**である。利用者から見える振る舞い（起動の見え方・終了操作 7 種の見え方・終了コード・記録の語彙）は 1 つも変えない。変えるのは「1 回きり」を前提にした 4 つの構造だけで、後続のゴースト切替（`areka-P0-ghost-shell-balloon-switch`）・シェルとバルーンの切替（`areka-P0-shell-balloon-switch`）・インストール（`areka-P0-ghost-install`）はこの形の上に建つ。

要件生成時に引き直した事実（設計はこれを再検証する）:

- 終了順序: `fn main` の `app.run()?` の後ろに、① loop ticker の停止 → ② `GhostRuntime::shutdown(CloseReason::User { scope: 0 })` → ③ seriko の join → ④ perf の最終報告、の順で並ぶ。① は結線ありの起動でだけ走り、結線なし（fallback）の起動では ② だけが走る。`app.run()` が失敗で戻ると ①〜④ は 1 つも走らない。② が失敗すると ③④、③ が失敗すると ④ を飛ばす（コメントで認めている）。
- 「1 回だけ登録する」結線 8 か所: `wire_emo2_boot`（`emo2_boot/mod.rs`・中で `wire_readme` と `wire_user_break` も呼ぶ）・`menu::wire_menu_with`（`main` からは `wire_menu` 経由）・`readme::wire_readme`・`input_events::user_break::wire_user_break`・`input_events::choice_drain::wire_choice_drain`・`input_events::balloon::wire_balloon_choice`・`placement::spawn::wire_zorder_pair`・`main.rs` の `open_startup_window`（`FrameFinalize` へ 3 つの系を登録）。**8 か所とも 2 度呼ぶと系が二重に登録される**。うち readme・user_break・choice_drain・balloon・zorder_pair・menu の 6 か所はコメントで「1 度しか呼ばれない」を前提と明記している（`wire_emo2_boot` 自身の「`main` から 1 度しか呼ばれない」も readme の前提と同じコメントに同居する＝数え方で 7。要件 2.5 は数を固定しない）。加えて `input_events::wire_mouse_input` は系を登録せず窓ごとの状態（`MouseWiring`）だけを置く（9 か所目・状態側のみ）。
- ゴーストごとに差し替えが要る状態: `MouseWiring`・`MenuWiring`・choice_drain の kanade 送り口（`ChoiceForwarder`）・`PersistWiring`・説明書の経路（`ReadmeWiring` の `path`）。ほかに `UserBreakWiring`・`Emo2Wiring`・`BalloonWiring`／`ChoiceSelectionInbox` もゴーストごとに置き直される状態である。
- `wire_emo2_boot` は `&WinApp` を取り、中で `app.world().borrow_mut()` を 4 回行う。`open_startup_window` は「系の登録 3 つ＋`wire_zorder_pair`」と「窓を作る（`app.run()` の最初の tick で走る非同期の指示）」が 1 つの関数に同居する。
- `app_exit.rs`: `despawn_app_windows` は私有、`quit_app(world, ExitOrigin)` はそれを呼んでから必ず終了の指示を出す。「全窓を閉じるが終了しない」操作は無い。`run_ghost_quit_phase`（`emo2_boot/frame.rs`）は停止通知を受けると必ず `quit_app` へ進む。
- 起動成功時の後処理 `on_boot_ok`（`main.rs`）は結線あり・結線なしの両方の腕から呼ばれ、`PersistWiring` の挿入と最後の選択の記憶の書き込みを行う。
- 同じプロセスで 2 度起動する決定論テストは存在しない。既存の背骨の試験台（`SpineHarness`）は `wire_emo2_boot` の結線を手で写しており、`wire_emo2_boot` そのものは呼んでいない。
- 既存テストのうち 3 本が本番ソースの字面で形を固定している: `zorder_wiring_tests.rs`（`main.rs` の `open_startup_window(&app, &cfg)` の呼び出し行）・`frame_schedule_tests.rs`（`pub fn wire_emo2_boot(` と `Update` の登録行）・`spawn_zorder_chain_wiring_tests.rs`（`pub fn wire_zorder_pair(world: &mut World) {`）。
- 行数: `main.rs` 774・`emo2_boot/mod.rs` 727・`emo2_boot/frame.rs` 459・`emo2_boot/frame/wiring.rs` 352・`app_exit.rs` 154。

## Boundary Context

- **In scope**: ① 終了順序を終了理由を引数に取る 1 つの関数へ／② 8 か所の結線を「登録（プロセスに 1 回）」と「ゴーストごとの状態の載せ替え」へ分ける（窓ごとの状態を載せ替えの単位にする）／③ 起動の結線をフレームの系の外から `&mut World` で呼べる形にし、`open_startup_window` の「登録」と「窓を作る」を分ける／④「全窓を閉じるが終了しない」操作を `app_exit.rs` に 1 つ足す／⑤ 同じプロセスで降ろして起こし直す決定論テスト 1 本と、④ の決定論テスト／⑥ 台帳 #57「起動後の途中終了で記憶の確定が飛ぶ」の相乗り（`app.run()` が失敗で戻る腕でも ① の関数を通す。要件 7 の裁定）。
- **Out of scope**: 切替の語彙とイベント（`\![change,…]`・`OnGhostChang*`・`\+`／`\_+`）・名前解決（`random`／`sequential`／`lastinstalled`）・kanade の握手・メニューの「ゴースト」枠の登記（#13）／シェル・バルーンの差し替え（#50）／SHIORI の失敗の告知と終了コードの改訂（#55）／`GhostRuntime` の中身と `crates/areka-ghost`（`GhostBootError` の doc の陳腐化を含む＝#55 の相乗り候補のまま）／kanade（`crates/areka-kanade`）／wintf。
- **Adjacent expectations**: 完了 `areka-P0-app-lifetime-separation`（`ExitPolicy::Explicit`・`quit_app`・`despawn_app_windows`・要件 3.6「片方だけを呼ぶ形を残さない」）と完了 `areka-P0-baseware-root-layout`（`on_boot_ok`）の上に建つ。`areka-P0-shiori-fault-notice`（#55）とは `main.rs`・`app_exit.rs`・`emo2_boot/frame.rs` を共有するので**実装は #55 の着地後**（B1 は文書のみ・B2 で実装）。実装着手時に settled main へ再突合し、本文の「引き直した事実」を更新する。下流の #13 は本仕様の形（登録と状態の切り分け・借用の形）を見てから設計する。

## Requirements

### Requirement 1: 終了順序は繰り返し呼べる 1 つの関数になっている

**Objective:** As a 後続の切替を作る開発者, I want 今日 `fn main` にべた書きされている終了順序が、終了理由を引数に取る 1 つの関数になっていること, so that 「降ろす」を同じプロセスの中で何度でも同じ順序で行え、切替の実装が終了順序を写し取らずに済む

#### Acceptance Criteria

1. The areka shall ゴーストを降ろす順序のうちゴーストごとの 3 段（loop ticker の停止 → ゴースト実行系の終了 → seriko の join）を 1 つの関数として持ち、終了の理由（kanade へ渡す `CloseReason` に相当する情報）を引数で受ける。4 段目の perf の最終報告はプロセスに 1 回の性質（報告スレッドは `app.run()` の前に起き、プロセスの終わりに 1 度だけ報告する）なので関数には含めず、`fn main` の末尾（関数の直後）に残して今日と同じ順序を保つ。
2. When 正常な終了操作（完了 `app-lifetime-separation` の 7 種のいずれか）で `app.run()` が戻る, the areka shall 1.1 の関数を通って終了し、後始末の順序と各段の記録を今日と同じにする。
3. When 結線なし（fallback）の起動で `app.run()` が戻る, the areka shall 同じ 1.1 の関数を通り、無い部品（loop ticker・seriko）の段を飛ばして今日と同じ後始末を行う。
4. If `app.run()` が失敗で戻る, then the areka shall それでも 1.1 の関数を通ってゴースト実行系を終了させてから、今日と同じ終了コード（失敗＝非 0）で戻る（台帳 #57 の相乗り・要件 7.1）。
5. If 1.1 の関数の途中の段が失敗する, then the areka shall 今日と同じく以降の段を飛ばし、失敗を記録に残して呼び手へ返す（段の失敗の扱いは本仕様で変えない）。
6. The areka shall 1.1 の関数を「1 プロセスに 1 回」を前提にしない形にし、同じプロセスで 2 度目に呼んでも 1 度目と同じ順序で走る（要件 6.1 の 2 周テストで確かめる）。
7. The 本仕様 shall `Drop` による自動の終了（終了理由を持てない）を採らない。

### Requirement 2: 系の登録（プロセスに 1 回）とゴーストごとの状態の載せ替えが分かれている

**Objective:** As a 後続の切替を作る開発者, I want 8 か所の結線が「スケジュールへの系の登録（プロセスに 1 回）」と「ゴーストごとに差し替える状態の載せ替え」に分かれていること, so that 2 体目のゴーストを起こすときに状態だけを載せ替えられ、系が二重に登録されない

#### Acceptance Criteria

1. The areka shall 今日「1 回だけ登録する」前提で書かれている 8 か所（起動の結線・メニュー・説明書・中断・選択肢の送り・バルーンの選択・重なり順の対・起動窓の登録）のそれぞれで、系の登録とゴーストごとの状態の載せ替えを別々に呼べる形にする。
2. When 系の登録の側が同じプロセスで 2 度呼ばれる, the areka shall 系を二重に登録しない（登録の側は 1 度しか呼ばれない形にするか、2 度目を無視して記録に残すかのいずれか。どちらを採るかは設計で決め、要件 6.1 の 2 周テストで「登録数が 1 周目と同じ」を確かめる）。
3. When ゴーストごとの状態の載せ替えの側が 2 度呼ばれる, the areka shall 前のゴーストの状態（`MouseWiring`・`MenuWiring`・選択肢の送り口・`PersistWiring`・説明書の経路、および中断の受け口・起動の結線の状態・バルーンの選択の受け口）を新しいゴーストのもので置き換え、前のものを残さない。
4. The areka shall 系を登録せず状態だけを置く結線（今日の `wire_mouse_input`）も載せ替えの単位として同じ形で扱う。
5. The areka shall 各結線の「1 度しか呼ばれない」前提を明記しているコメントを**すべて**、新しい形（登録は 1 回・載せ替えは n 回）に合わせて書き換える（数は固定しない。要件生成時とギャップ分析で 6 と 7 に割れた＝`wire_emo2_boot` 自身の前提と `wire_readme` の前提が同じ 1 つのコメントに同居しており、数え方で揺れる。実装時に grep で該当箇所を引き直す）。
6. The areka shall 起動成功時の後処理（今日の `on_boot_ok`＝`PersistWiring` の挿入と最後の選択の記憶の書き込み）を、載せ替えの単位から呼べる形に保つ。

### Requirement 3: 起動の結線はフレームの系の外から呼べる

**Objective:** As a 後続の切替を作る開発者, I want 起動の結線（今日の `wire_emo2_boot`）が `&mut World`（または World へ届く指示の送り口）で呼べ、`open_startup_window` の「登録」と「窓を作る」が分かれていること, so that フレームの系の中（World を借りている最中）から起こし直しを仕掛けても二重借用で落ちない

#### Acceptance Criteria

1. The areka shall 起動の結線を、呼び手が既に World を借りている文脈から呼べる形（`&mut World` を受ける、または World への指示を積む送り口を通す）にし、自分で `WinApp` から World を借りない。
2. The areka shall 起動の結線がフレームの系の外から呼べることを、その形で呼ぶ決定論テスト（要件 6.1）がコンパイルして通ることで示す。
3. The areka shall `open_startup_window` を「プロセスに 1 回の登録（クリック透過の登録・OS の閉鎖要求の受け口・重なり順の対）」と「ゴーストごとに窓を作る（監視の状態・DPI 表・窓の生成・入力の受け口の装着・復元）」の 2 つに分け、後者を 2 度目以降の起動でも呼べる形にする。
4. When 2 度目以降の起動で窓を作る側が呼ばれる, the areka shall 1 度目と同じ手順（配置の準備 → 監視の状態 → 復元 → 窓の生成 → 入力の受け口の装着）で窓を作る。
5. The areka shall smoke の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）の仕掛けを 1 度目の起動でだけ仕掛け、今日と同じ見え方で終える。

### Requirement 4: 「全窓を閉じるが終了しない」操作がある

**Objective:** As a 後続の切替を作る開発者, I want 全ゴースト窓を閉じてもアプリを終了させない操作が `app_exit.rs` に 1 つあること, so that 切替のときに古い窓を全部閉じてから新しい窓を開けられ、その操作が `quit_app` を通らない（完了 `app-lifetime-separation` の申し送りと一致する）

#### Acceptance Criteria

1. The areka shall 全ゴースト窓を閉じるが終了の指示を出さない操作を `app_exit.rs` に 1 つ持つ。
2. When その操作が呼ばれる, the areka shall 全ゴースト窓を閉じ、終了の指示（`AppExit`）を立てない（戻り値の形＝閉じた枚数か「続きを起こすための証」かは、要件 4.4 の呼び手の限定と合わせて設計で決める）。
3. When その操作が呼ばれる, the areka shall 「切替のために窓を閉じた」ことを、終了操作の記録（`app_exit`）とは別の語彙でライフサイクル事象として記録に残す（info）。
4. The areka shall その操作を「直後に起こし直す」呼び手にだけ公開される形にし（どの形で呼び手を限定するかは設計で決める）、終了経路のどこからも呼べる汎用の口にしない（完了 `app-lifetime-separation` 要件 3.6「片方だけを呼ぶ形を残さない」の意図を守る）。
5. The areka shall その操作と `quit_app` の両方が同じ私有の部品（今日の `despawn_app_windows`）を使い、窓を消す手順を 2 つ持たない。
6. The areka shall 終了経路（`quit_app`・`run_ghost_quit_phase`・OS の閉鎖要求・強制退避・smoke）を本仕様で変えず、これまでどおり必ず終了の指示まで進める。

### Requirement 5: 利用者から見える振る舞いを変えない

**Objective:** As a areka の利用者, I want 本仕様の前後で起動・会話・終了の見え方が区別できないこと, so that 構造の括り出しが製品の挙動に影響しない

#### Acceptance Criteria

1. The areka shall 起動の見え方（窓の出る順序と位置・最後の選択の記憶の書き込み・説明書とメニューの動作）を今日と同じに保つ。
2. The areka shall 終了操作 7 種（完了 `app-lifetime-separation` 要件 3）のそれぞれで、後始末の順序・終了コード・記録の語彙（`app_exit` 事象を含む）を今日と同じに保つ。
3. The areka shall 常設の smoke テスト（`crates/areka/tests/smoke_boot_loop_exit.rs`・3 方向）を変更後も緑にする。
4. The 本仕様 shall kanade（`crates/areka-kanade`）・`crates/areka-ghost`・wintf・`crates/areka` の examples を 1 ファイルも変えない。
5. The 本仕様 shall 切替の語彙（`\![change,…]`・`OnGhostChanging`／`OnGhostChanged`・`\+`／`\_+`）と名前解決を持ち込まず、メニューの枠にも登記しない。
6. The 本仕様 shall 変更後も `crates/areka/src/main.rs`・`crates/areka/src/emo2_boot/mod.rs` を含む触るファイルすべてを 1 ファイル 1,000 行の目安の内側に収める（今日 774・727。分割で減る方向）。
7. The 本仕様 shall `GhostBootOptions` に欄を足さない（構造体リテラルが 27 か所・16 ファイルに波及する。起動元の情報を渡す口は #13 が派生関数で作る）。

### Requirement 6: 決定論テスト

**Objective:** As a 開発者, I want 「同じプロセスで降ろして起こし直しても 1 周目と同じ」と「窓だけ閉じても終了しない」をそれぞれ赤にできること, so that 後続の切替が乗る形が固定される

#### Acceptance Criteria

1. The 本仕様 shall 偽の SHIORI と偽の資産で「起こす → 降ろす（要件 1.1 の関数）→ 全窓を閉じる（要件 4.1 の操作）→ 起こす」を同じプロセス・同じ World で 2 周する決定論テストを 1 本持ち、2 周目の後で ⑴ 各スケジュールに登録された系の数が 1 周目の後と同じであること、⑵ ゴーストごとの状態が 2 周目のもので置き換わり前のものが残っていないこと、⑶ 終了の指示が立っていないことを、集めてから 1 回で判定する（面ごとに止めると赤が 1 面しか見えない）。
2. The 本仕様 shall 要件 4.1 の操作について「全ゴースト窓が閉じ、終了の指示が立っていない」ことを決定論テスト 1 本で確かめる（この判断が壊れると赤になる）。
3. If 要件 1.4 の腕（`app.run()` が失敗で戻る）を採る, then the 本仕様 shall その腕でも降ろす関数が呼ばれることを決定論テスト 1 本で確かめる（`app.run()` の失敗を偽で作る口は areka 側に置き、`WinApp::run` は変えない＝要件 5.4。実機の smoke では確かめない）。
4. The 本仕様 shall 足すテストを上の判断分岐（2 周しても同じ・閉じても終了しない・失敗の腕でも降ろす）に限り、既に確かめられている配線（閉じる → 登録表から外れる・終了の指示 → メッセージループが終わる・kanade の握手・seriko の join）を再テストしない。
5. The 本仕様 shall 既存の決定論テストを 1 本も落とさない。本番ソースの字面で形を固定している 3 本（`zorder_wiring_tests.rs`・`frame_schedule_tests.rs`・`spawn_zorder_chain_wiring_tests.rs`）は、新しい形の字面へ追随させて検査の意図（登録の位置と順序の固定）を保ち、削除しない。
6. The 本仕様 shall 足すテストを本番ファイルの隣の `<stem>_<モジュール名>.rs` に置き、共有する部品は既存の試験台（`SpineHarness`・`ScriptedShioriBackend`・`frame_test_support`）を再利用して複製しない。

### Requirement 7: 裁定（要件生成時に確定・討議で覆せる）

**Objective:** As a 開発者, I want brief が「要件で決める」とした 3 点を要件の段階で確定しておくこと, so that 設計がこの形で進み、討議で覆すときは根拠が残る

#### Acceptance Criteria

1. The 本仕様 shall **裁定 1（台帳 #57 の相乗り＝採る）**として、`app.run()` が失敗で戻る腕でも要件 1.1 の関数を通す形を本仕様に含める。根拠: #55 `shiori-fault-notice` は要件未生成（brief のみ）で #57 を採っておらず、#55 の brief 自身が「#58 が終了順序を関数に括り出すので #57 の直しはその関数の中に置くのが筋」と記す。**#55 が先に #57 を採って着地した場合**は、本仕様は要件 1.4・6.3 を「既に満たされている」として実装で触らず、要件と設計の該当箇所にその旨を記す。これは roadmap ウェーブ B1 の取り決め（「#57 は #55 の要件で相乗りを決める。採らなければ B2＝本仕様へ」）と同じ規則を本仕様の側から書いたもので、判定は本仕様の実装着手時（B2・#55 の着地後）に settled main で行う。
2. The 本仕様 shall **裁定 2（完了 `app-lifetime-separation` 要件 3.6 は上書きしない）**として、要件 4 の操作を「終了経路のどこからも呼べない・切替の呼び手にだけ公開する」形で足し、3.6 の意図（単独で窓を消して寿命を狂わせない）を守る。これは完了 spec の要件の上書きではないので `doc/COMPAT_ARCHITECTURE.md` §8 への記載は要らない。
3. Where 設計の途中で裁定 2 が守れず完了 spec の要件を上書きする必要が判明する, the 本仕様 shall 開発者へ議題として上げ、確定後に `doc/COMPAT_ARCHITECTURE.md` §8 へ 1 行を記す。
4. The 本仕様 shall **裁定 3（採らない形）**として、`Drop` による自動の終了と `GhostBootOptions` への欄の追加を採らない（要件 1.7・5.7）。
5. Where 設計・実装の途中で裁定 1〜3 のいずれかを覆す必要が判明する, the 本仕様 shall 開発者へ議題として上げ、確定を待ってから要件・設計の該当箇所を改める。

# Requirements Document

> 本文の実測は **2026-09-23・本ブランチ**（main `92f5f448` 相当）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> **末尾「確認事項」は 4 件**。要件ディスカッション（2026-09-23）で 1・3・4 は実測の根拠どおり決定済み、2（説明書が無いときの記録の水準）は開発者の裁定。

## Introduction

### 誰が困っているか

wintf のドラッグの状態を読む開発者（areka のメニュー・今後の D&D・サブメニュー登記など、ドラッグの状態を今後読む全 spec）。そして、その読み違いの結果として右クリックメニューが出なくなる利用者。

### いま何が起きているか（2026-09-23 実測）

- **説明と実装が食い違っている。** `crates/wintf/src/ecs/drag/state/mod.rs` の `DragState::JustEnded` の doc は「ドラッグ終了直後（1フレームのみ）」、同ファイルの `reset_to_idle` の doc は「ドラッグ状態を Idle にリセット（dispatch_drag_events 後）」と書く。しかし **`reset_to_idle` を呼ぶ製品コードは 0 件**である。在るのは定義（`state/mod.rs`）・再輸出（`crates/wintf/src/ecs/drag/mod.rs`）・テスト 3 か所（`state/tests.rs` の `test_reset_to_idle_only_from_just_ended`／`test_reset_to_idle_noop_when_preparing`・`crates/wintf/src/ecs/clickthrough/controller_tests.rs` の `eval_honors_drag_snapshot_just_ended_reconverges` の後片付け）だけ。「dispatch_drag_events 後」に走る system は `crates/wintf/src/ecs/drag/systems.rs` の `cleanup_drag_state` だが、これは ECS 側の `DraggingState` を外すだけで、スレッドごとの状態には触れない。
- **製品の状態遷移は「押下 → 解放 → 休み」で止まる。** 左押下は `crates/wintf/src/ecs/window_proc/mouse_click.rs` の `handle_button_message` が `start_preparing` を呼び（`Idle`／`JustEnded` → `Preparing`）、解放は同じ関数が `end_dragging` を呼ぶ（`Preparing`／`JustStarted`／`Dragging` → `JustEnded`）。中断（ESC・`WM_CANCELMODE`・非アクティブ化・捕捉喪失）は `crates/wintf/src/ecs/window_proc/keyboard.rs` の各ハンドラが `cancel_dragging` を呼び、やはり `JustEnded` へ行く。`JustEnded` から出る遷移は `start_preparing`（次の左押下）だけなので、**起動後の最初の左クリック以降、製品では `Idle` へ二度と戻らない**。
- **`JustEnded` が持つ値（entity・位置・中断か）を読む製品コードは 0 件**（読むのは `DragState::snapshot` の写しだけ）。「1 フレームだけ観測できる」ことに依存する読み手も 0 件である。
- **「いま左ボタンを押している間か」を答える述語は `DragState`／`DragStateSnapshot` に 0 件**で、読み手はそれぞれ自分で variant を並べている。並べている場所は次のとおり（製品コード）:
  - 「押している間か」だけを聞く読み手 = 3 か所: `state/mod.rs` の `start_preparing`（`Preparing | JustStarted | Dragging` なら新しい押下を無視）・`crates/wintf/src/ecs/pointer/nchittest_cache.rs` の当たり判定の読み手（同じ 3 つなら透明領域でも `HTCLIENT`）・`crates/areka/src/menu/trigger.rs` の `handle_release`（`Idle | JustEnded` でなければ右解放を無視・2026-09-19 のコミット `b7968925` で是正済み）。
  - 別の問いを聞く読み手 = 1 か所: `crates/wintf/src/ecs/clickthrough/controller.rs` の `resolve_transition`（「移動中か」＝`Dragging | JustStarted`。`Preparing` は含めない＝押下だけでは透過を固定しない）。
  - 値を取り出すために並べる読み手 = `keyboard.rs` の `WM_KEYDOWN`／`WM_CANCELMODE`／`WM_ACTIVATE`／`WM_CAPTURECHANGED`（`WM_CAPTURECHANGED` は捕捉の持ち主を可変で触るので写しでは足りない）・`mouse_click.rs` の `handle_button_message`・`crates/wintf/src/ecs/window_proc/mouse_move.rs` の移動ハンドラ。
- **実害は 1 件出た（2026-09-19）。** `menu/trigger.rs` の解放ハンドラが「`Idle` でなければドラッグ中」と読み、左クリックを 1 度した後は右クリックメニューが二度と出なくなった。決定論テストは「ドラッグ中」の代役に `JustEnded` を置いていた（捕捉の持ち主が要らない唯一の非待機状態だった）ので欠陥を合格条件に固定しており、実機でしか見つからなかった。利用側は是正済みだが、**契約そのものは偽のまま**で、次の読み手が同じ罠を踏む。
- **透過制御（R5.2「`JustEnded` 再収束」）は `JustEnded` を観測することに依存していない。** `resolve_transition` は `Idle`・`Preparing`・`JustEnded` を同じ枝で扱い、現在の当たりに従う。つまり `JustEnded` を `Idle` に読み替えても判定は 1 ビットも変わらない。`crates/wintf/src/runtime/mod.rs` の `wire_click_through` の doc が触れる「`JustEnded` 再収束」も同じ意味（終了直後の周で、押下中の固定が外れて現在の当たりへ戻る）である。

### 相乗り 2 件（`areka-P0-popup-menu-residue` の残件 1・2）

同じ `crates/areka/src/menu/trigger.rs` を触り、性質も同じ「右クリックの引き金の潜在バグ」なので本仕様で一緒に直す。

- **別の窓の預かりの誤配。** `trigger.rs` の `poll_once` は、返事が決着した tick に `take_deferred_double_click` で預かり（`crates/areka/src/input_events/mod.rs` の `PendingDoubleClick`・`scope` を持つ）を取り出し、`decide` が「抑止」を返せば `send_pending_right_double_click` で送る。このとき **預かりの `scope` と要求（`MenuRequest`）の `scope` を比べていない**。実害の条件は「返事待ち 1 秒未満の間に別の窓で右クリック 1 回＋右押下」で実質届かないが、届けばメニューを抑止していない側の窓へ右ダブルクリックの台詞が出る。
- **説明書の無いゴーストで台本の `\![open,readme]` が `error!` を出す。** `crates/areka/src/readme.rs` の `open_from_world` はファイルの実在を見ずに `open` を呼び、無ければ `ShellExecuteW` が「見つからない」（符号 2）を返して `readme_open_failed` を `error!` で記録する。メニューの側は `is_available` が `wiring.path.exists()` で灰色にし、「無い」を初回だけ `debug!`（`readme_missing`）で記録している。説明書の無いゴーストは普通に在るので、台本側の `error!` は障害調査を誤らせる。動作は同じ（何も開かない）で、記録だけが重い。

### 正典（ukadoc）の位置づけ

本仕様が定めるのは wintf の内部契約と areka の記録の水準であり、ukadoc に直接対応する項は無い。相乗り 1 が扱う「メニューを抑止したゴーストへ右ダブルクリック（`OnMouseDoubleClick`・Ref5＝1）を送る」規則は完了 spec `areka-P0-popup-menu-minimal`（要件 1.10）が正典から輸入済みで、本仕様はその規則を変えない。

### 何を変えるか

**直し方は「説明を実装に合わせる」（brief の案 A）を採る。** 挙動は 1 ビットも変えない。

- wintf: ドラッグの状態の説明を「解放または中断のあとは、次の左押下まで `JustEnded` で休む」へ直し、製品の呼び手が 0 件の `reset_to_idle` を撤去する。「左ボタンを押している間か」を答える述語を 1 つ足し、その問いだけを聞いている読み手 3 か所（wintf 2・areka 1）をそれへ寄せる。契約が後退したら赤くなる決定論テストを 1 本置く（製品と同じ関数 `start_preparing` → `end_dragging` を踏む）。
- areka（相乗り）: 預かった右ダブルクリックは、預かりの窓と要求の窓が同じときだけ送る。台本の `\![open,readme]` は、説明書のファイルが無ければ OS を呼ばずに記録して終える。

**案 B（実装を説明に合わせる＝フレームの終わりで `Idle` へ戻す）を採らない根拠**: ⑴ 透過制御 R5.2 は `JustEnded` の観測に依存していない（上記）ので案 B でも壊れないが、⑵ 案 B は製品に新しい遷移（毎フレームの戻し）を足す挙動変更であり、⑶ 「解放の 1 フレーム後に状態が変わる」形そのものが「1 フレーム遅らせて辻褄を合わせる解」に当たる（開発規律）。⑷ `JustEnded` の値を読む製品コードも、1 フレームだけの観測に依存する読み手も 0 件なので、案 B が守るものは無い。

## Boundary Context

- **In scope**:
  - `crates/wintf/src/ecs/drag/state/mod.rs` の状態の説明（5 つの状態すべて）・述語 1 つ・`reset_to_idle` の撤去・契約の決定論テスト 1 本。
  - 読み手の寄せ替え: `start_preparing`（`state/mod.rs`）・`nchittest_cache.rs` の当たり判定の読み手・`crates/areka/src/menu/trigger.rs` の `handle_release`。
  - `reset_to_idle` を使っていた既存テスト 3 か所の追随（陳腐化した 2 本は除外・後片付けの 1 か所は置き換え）。
  - 相乗り 1: `trigger.rs` の `poll_once` で預かりの窓と要求の窓を比べる 1 条件＋決定論テスト。
  - 相乗り 2: `readme.rs` の `open_from_world` でファイルの実在を先に見る＋記録の水準の裁定＋決定論テスト。
- **Out of scope**:
  - ドラッグの挙動そのもの（閾値・捕捉・窓の移動・`DragAccumulatorResource`・`dispatch_drag_events`）。
  - 透過制御の判定規則（`resolve_transition` の枝分け）。「移動中か」（`Dragging | JustStarted`）の述語は足さない（読み手が 1 か所しか無い）。
  - 右ボタン・中ボタンのドラッグ。
  - `JustEnded` という variant の名前の変更（説明を直せば契約は一致する。改名は設計の裁量で、要件は求めない）。
  - 値を取り出すために variant を並べている読み手（`keyboard.rs`・`mouse_click.rs`・`mouse_move.rs`）の書き換え。
  - 右クリックメニューの振る舞い（完了 spec `areka-P0-popup-menu-minimal` は開け直さない）・`areka-P0-popup-menu-residue` の残件 3〜10。
  - 説明書を開く処理を World の借用の外へ出すこと（残件 10）。
- **Adjacent expectations**:
  - `areka-P0-popup-menu-minimal` の決定論テスト `crates/areka/src/menu/trigger_flow_tests.rs` の `a_release_after_a_finished_left_click_still_opens_the_menu` は、既に製品と同じ関数で押下→解放を踏んでいる。本仕様の後も緑のまま、意図を変えない。
  - `crates/wintf/src/ecs/clickthrough/controller_tests.rs` の `eval_honors_drag_snapshot_just_ended_reconverges` は `JustEnded` を手で組んで透過制御を確かめる（透過制御の spec の持ち物）。本仕様が触るのは後片付けの `reset_to_idle` の置き換えだけで、確かめる内容は変えない。
  - `areka-P0-ghost-install`（`WM_DROPFILES`）と `areka-P0-ghost-shell-balloon-switch`（サブメニュー）は、ドラッグの状態を読むなら本仕様の述語を使う。
  - 実機確認は要らない。本仕様の変更はどれも決定論テストで到達でき、実機でしか見えない差（挙動の変更）を 1 つも作らない。

## Requirements

### Requirement 1: ドラッグの状態の説明と実装が一致する

**Objective:** As a ドラッグの状態を読む開発者, I want 状態の説明を読めば「どの出来事で移り、どこで休むか」がそのまま実装と一致していること, so that 「待機でなければドラッグ中」のような読み違いが起きない

#### Acceptance Criteria

1. The wintf shall `DragState` の 5 つの状態それぞれに「どの出来事でこの状態に入り、どの出来事で出るか」を説明に書き、フレーム数（「1 フレームのみ」等）で説明しない（`JustStarted` は閾値到達の後の次の tick の `dispatch_drag_events`（`crates/wintf/src/ecs/drag/dispatch.rs`）で `Dragging` へ、解放・中断なら `JustEnded` へ。`JustEnded` は次の左押下で `Preparing` へ）。
2. The wintf shall `JustEnded` の説明に「解放または中断のあと、次の左押下までここで休む。製品コードがこの状態を `Idle` へ戻すことは無い」ことを書く。
3. The wintf shall 製品コードから呼ばれない状態遷移関数 `reset_to_idle` を撤去し、撤去後の wintf の公開 API に「製品の呼び手が 0 件で、説明が製品の時機（『dispatch_drag_events 後』等）を語る関数」を 0 件にする（`check_threshold` も製品の呼び手は 0 件だが、説明は閾値判定を語るだけで製品の時機を語らないので撤去の対象外）。
4. When 左ボタンが押されて離される（`start_preparing` → `end_dragging`）, the wintf shall 解放後の状態を今日と同じ（`JustEnded`・次の左押下まで変わらない）に保ち、製品コードの状態遷移を 1 つも足さず 1 つも減らさない（挙動は 1 ビットも変えない）。
5. The wintf shall `crates/wintf/src/runtime/mod.rs` と `crates/wintf/src/ecs/clickthrough/controller.rs` の「`JustEnded` 再収束（R5.2）」の説明を、本仕様の説明と矛盾しない文言（「終了直後の周で押下中の固定が外れ、現在の当たりへ戻る」の意味）に保つ。あわせて `controller.rs` の `resolve_transition` の説明と `docs/click_through.md`（「ドラッグ中の透過抑止」の段落）が `JustStarted` を「直前 1 フレーム」と書く箇所（2 か所）を、要件 1.1 と同じ出来事の語（「閾値到達から次の tick の `dispatch_drag_events` まで」）へ直す。透過制御の判定規則は変えない（要件 6.2）。

### Requirement 2: 「左ボタンを押している間か」を 1 つの述語で聞ける

**Objective:** As a ドラッグの状態を読む開発者, I want variant を並べずに「いま左ボタンを押している間か」を 1 つの述語で聞けること, so that 次の読み手が `JustEnded` の扱いで同じ罠を踏まない

#### Acceptance Criteria

1. The wintf shall ドラッグの状態に「左ボタンを押している間か」を答える述語を 1 つ持ち、`Preparing`・`JustStarted`・`Dragging` で真、`Idle`・`JustEnded` で偽を返す。
2. The wintf shall その述語を、状態の写し（`DragStateSnapshot`）を持つ読み手と状態そのもの（`DragState`）を持つ読み手のどちらからも、variant を並べずに呼べるようにする。
3. The wintf shall 「押している間か」だけを聞いている製品の読み手 2 か所（`start_preparing`・`nchittest_cache.rs` の当たり判定の読み手）をこの述語へ寄せ、寄せた後の判定結果を寄せる前と同じに保つ。
4. The areka shall `crates/areka/src/menu/trigger.rs` の `handle_release` の「ドラッグ中なら右解放を無視する」判定をこの述語へ寄せ、寄せた後の判定結果（`Idle`・`JustEnded` で受け付け、それ以外で無視）を寄せる前と同じに保つ。
5. The wintf shall 寄せ替えの後、製品コードに「押している間か」を答えるためだけに variant を並べている箇所を 0 件にする。
6. The wintf shall 別の問いを聞く読み手（`resolve_transition` の「移動中か」＝`Dragging | JustStarted`）と、値を取り出すために並べる読み手（`keyboard.rs`・`mouse_click.rs`・`mouse_move.rs`）を書き換えず、それらの判定結果を変えない。

### Requirement 3: 契約が後退したら赤くなる決定論テスト

**Objective:** As a wintf の保守者, I want 「解放のあとは次の左押下まで休む」契約を製品と同じ関数で踏む決定論テストがあること, so that 説明と実装がふたたび食い違ったときにテストが赤くなる

#### Acceptance Criteria

1. The wintf shall `crates/wintf/src/ecs/drag/state/tests.rs`（実装と同じディレクトリの兄弟テスト）に、製品と同じ関数 `start_preparing` → `end_dragging` を踏む契約のテストを 1 本置き、手で組んだ状態から始めない。
2. When そのテストが解放（`end_dragging`）を踏んだ直後, the test shall 述語が偽（押していない）であること、および他の関数を 1 つも呼ばずに次の `start_preparing` が受け付けられて `Preparing` になることを確かめる。
3. If 将来 `JustEnded` が述語で真になる、または解放後の状態が次の左押下を受け付けなくなる, then the test shall 赤くなる。
4. The wintf shall 中断（`cancel_dragging`）を踏んだ場合についても同じ契約（述語が偽・次の押下を受け付ける）を同じテストまたは兄弟の 1 本で確かめる。
5. The wintf shall `reset_to_idle` を確かめていた既存テスト 2 本（`test_reset_to_idle_only_from_just_ended`・`test_reset_to_idle_noop_when_preparing`）を陳腐化として除外し、後片付けに `reset_to_idle` を使っていた `controller_tests.rs` の 1 か所を `update_drag_state` で `Idle` へ戻す形に置き換える（`crates/areka/src/menu/trigger_flow_tests.rs` の `ResetDragState` と同じ作法）。
6. The areka shall `trigger_flow_tests.rs` の `a_release_after_a_finished_left_click_still_opens_the_menu` を意図を変えずに緑のまま保つ。
7. The wintf shall テストファイルを 1 ファイル 1,000 行以内に保つ（`state/tests.rs` は現在 499 行・総行数）。

### Requirement 4: 預かった右ダブルクリックは同じ窓の抑止でしか送らない（相乗り 1）

**Objective:** As a ゴーストの利用者, I want 右ダブルクリックが、自分がダブルクリックした窓の判定でだけ送られること, so that 別の窓の返事待ちの決着で、メニューを抑止していない側の窓へ台詞が出ない

#### Acceptance Criteria

1. When 照会の返事が「抑止」に決着する and 預かった右ダブルクリックがある and 預かりの窓（`PendingDoubleClick.scope`）が要求の窓（`MenuRequest.scope`）と同じ, the areka shall 今日と同じく預かりを `OnMouseDoubleClick`（右ボタン・Ref5＝1）として送る。
2. When 照会の返事が「抑止」に決着する and 預かった右ダブルクリックがある and 預かりの窓が要求の窓と異なる, the areka shall 預かりを送らずに捨て、両方の窓番号を添えて `trace!` で 1 行記録する。
3. When 照会の返事が「表示」に決着する, the areka shall 今日と同じく預かりを捨てる（窓が同じでも異なっても送らない）。
4. The areka shall 「送る／送らない」を決める場所を引き続き 1 か所（`decide` の判断）に保ち、窓の比較をその判断の入力として扱う（判断の場所を 2 つにしない）。比較の置き場は設計で決めるが、`crates/areka/src/menu/trigger_tests.rs` の `decide` の既存テストを変えずに済む形を取る（要件 6.5）。
5. The areka shall `crates/areka/src/menu/trigger_flow_tests.rs`（または 1,000 行を超えるなら同じ接頭辞の兄弟ファイル・現在 863 行・総行数）に、「窓 1 の預かり＋窓 0 の要求＋抑止の返事」で kanade へ何も届かず預かりが残らず記録の行が出ることを確かめる決定論テストを 1 本置く。
6. The areka shall 「同じ窓の預かり＋抑止の返事」で送られる既存の確認を緑のまま保つ。

### Requirement 5: 説明書の無いゴーストで台本の `\![open,readme]` が重い記録を出さない（相乗り 2）

**Objective:** As a 障害調査をする開発者, I want 説明書を同梱していないゴーストで `\![open,readme]` が来ても、異常でない出来事が `error!` に見えないこと, so that 記録の `error!` を見れば本当の失敗だけが並ぶ

#### Acceptance Criteria

1. When 台本またはメニューから「説明書を開いて」の要求が届く and 起動時に決めた説明書のファイルが存在しない, the areka shall OS の開く処理を呼ばず（呼び出し 0 回）、ファイルのパスを添えて要求 1 件につき 1 行を **`warn!`** で記録し、ゴーストの動作を続ける（記録の水準は確認事項 2・推奨は `warn!`）。
2. When 「説明書を開いて」の要求が届く and ファイルが存在する, the areka shall 今日と同じく OS の開く処理を呼ぶ。
3. When OS の開く処理が失敗する（ファイルは在ったが開けない）, the areka shall 今日と同じくパスと符号を添えて `error!`（`readme_open_failed`）で記録する（本当の失敗の記録は落とさない）。
4. The areka shall メニュー側の振る舞い（ファイルが無ければ「説明書」を灰色にし、「無い」を初回だけ `debug!` で記録する）を変えない。
5. The areka shall `crates/areka/src/readme_tests.rs`（現在 214 行・総行数）に、ファイルの無い一時フォルダで `open_from_world` を呼ぶと `readme_open_failed` が 0 行・要件 5.1 の記録が 1 行（水準つき）であることを確かめる決定論テストを 1 本置く。

### Requirement 6: 変えないこと

**Objective:** As a areka と wintf の保守者, I want 本仕様が説明・述語・記録以外を変えないこと, so that 実機で見える挙動が 1 つも変わらず、既存のテストがそのまま緑で通る

#### Acceptance Criteria

1. The wintf shall ドラッグの挙動（閾値・捕捉の取得と解放・窓の移動・`DragAccumulatorResource` への記録・`dispatch_drag_events`）を変えない。
2. The wintf shall 透過制御の判定規則（`resolve_transition` の枝分け・R5.1〜R5.3）を変えない。
3. The areka shall 右クリックメニューの表示・抑止・項目の動作（相乗り 1 の窓の比較を除く）を変えない。
4. The wintf and areka shall 既存の記録の行の語彙（`[drag]`・`[menu]`・`[readme]` の event 名）を変えず、本仕様で足す記録は新しい event 名で足す。
5. The wintf and areka shall `cargo test -p wintf` と `cargo test -p areka` の既存テストを、要件 3.5 で除外する 2 本を除き 1 本も変えずに緑のまま保つ。

## 確認事項（要件ディスカッションで確認する）

1. **直し方は案 A（説明を実装に合わせる）＋`reset_to_idle` の撤去で進める（決定済み・2026-09-23 要件ディスカッション）。** 根拠: `reset_to_idle` の製品の呼び手 0 件・`JustEnded` の値を読む製品コード 0 件・透過制御 R5.2 は `JustEnded` の観測に依存しない（`resolve_transition` は `Idle` と同じ枝）・案 B は挙動変更かつ「1 フレーム後に戻す」形。brief は「明記するかテスト専用へ格下げ」も許すが、テスト専用にすると再輸出も `cfg(test)` にする手間が増えるだけで守るものが無いので撤去を推す。
2. **説明書が無いときの台本の `\![open,readme]` の記録の水準（裁定）。** 推奨は **`warn!`**（要求 1 件につき 1 行）。理由: ゴーストの台本が「無いものを開け」と言った＝ゴースト側の作りの問題で、利用者から見ると「何も起きなかった」ので原因を記録に残す価値がある。`logging.md` の基準では「回復可能なエラー・フォールバック」＝`warn!`。対案は `debug!`（メニュー側の `readme_missing` と揃える）と `info!`。`error!` のままにはしない。
3. **読み手の寄せ替えは brief の「wintf 内 3 か所」と異なる（報告・ギャップ分析で見落とし 0 件を確認済み）。** brief は `resolve_transition`・`start_preparing`・`keyboard.rs` を挙げるが、実測では `resolve_transition` は別の問い（「移動中か」・`Preparing` を含めない）で、`keyboard.rs` の `WM_CAPTURECHANGED` は捕捉の持ち主を可変で触るので写しの述語では足りない。どちらも述語へ寄せると挙動が変わるか書けない。代わりに `nchittest_cache.rs` の当たり判定の読み手が「押している間か」だけを聞いている。寄せる先は `start_preparing`・`nchittest_cache.rs`・`trigger.rs` の 3 か所。
4. **窓の異なる預かりは「捨てて `trace!`」に決めた（報告）。** `areka-P0-popup-menu-residue` の brief は「捨てるか残すか」を要件で決めるとしていた。残すと、後の無関係な要求の抑止で送られる（`trigger.rs` の `ignore_release` の説明が禁じる形）ので捨てる。

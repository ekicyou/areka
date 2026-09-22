# Requirements Document

> 本文の実測は **2026-09-23・本ブランチ**（main `637799d7` 相当）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> **本文に「仮の裁定」と記した箇所は、この後の要件ディスカッションで開発者が確認する既定である。** 覆されない限り、この既定のまま設計・実装へ進む。

## Introduction

### 誰が困っているか

ゴースト・シェル・バルーンを切り替えたい利用者——の手前に居る、切替を実装する開発者（後続 spec `areka-P0-ghost-shell-balloon-switch`）。

### いま何が起きているか（2026-09-23 実測）

- **areka のアプリは、窓が 1 枚も無くなった瞬間に終わる。** これを決めているのは areka ではなく **wintf** である。`crates/wintf/src/runtime/mod.rs` の `WinApp::new` が `fn wire_shutdown_hook` で窓の登録表（`crates/wintf/src/runtime/window_registry.rs` の `WindowRegistry`）に「空になったら終了の合図を鳴らす」仕掛けを差し込み、`WinApp::run` はその合図が鳴るまで待つだけである。合図は登録表が「空でない → 空」へ遷移した瞬間にだけ鳴る（`fn reconcile_window_registry`）。
- **wintf に「明示的に終了を指示する口」は無い。** 合図を鳴らす手段は登録表の空遷移だけで、それを外から選ぶ設定も無い。`WinApp::new` は引数を取らず、構築時の設定値の器も無い。
- **areka の終了操作は 6 種あり、どれも「全窓を閉じて、あとは wintf に任せる」形である。** 終了の指示に当たる語（`AppExit` 等）は `crates/` に 0 件。全窓を閉じるコード上の場所は 4 つ:
  - `crates/areka/src/emo2_boot/frame.rs` の `fn run_ghost_quit_phase`——kanade の終了系列の完了通知（`KanadeStopped`）を受けて `despawn_ghost_windows` を呼ぶ。**メニューの「終了」・別れの台詞のあとの終了・中断のあとの終了**の 3 操作はすべてここへ集まる（完了 spec `areka-P0-balloon-break` の後の形。中断のあとの終了は `crates/areka-kanade/src/schedule/user_break.rs` が終了系列へ乗せる）。閉じる窓が既に無いときは「正常系として打ち切る」記録を残して戻る。
  - `crates/areka/src/input_events/mod.rs` の `fn on_char_pointer_pressed` の中の**強制退避**（Ctrl＋Shift＋左ダブルクリック）——`despawn_ghost_windows` を呼ぶ。
  - `crates/areka/src/main.rs` の `fn on_dummy_pressed`——ゴーストの根が無いときの**ダミー窓**をダブルクリックで閉じる。
  - `crates/areka/src/main.rs` の `fn open_startup_window` の中の **smoke の自動終了**——環境変数 `AREKA_APP_SMOKE_EXIT_MS` で指定した時間の後に `despawn_smoke_targets` を呼ぶ。
- **`WinApp::run` が戻ったあとの後始末は `main.rs` の `fn main` に在る**: ① SERIKO の再生ループの停止 → ② ゴースト実行環境の終了統括（`crates/areka-ghost/src/runtime.rs` の `fn shutdown`）→ ③ SERIKO アクターの合流 → ④ 性能報告の出力、の順で、終了コード 0 で終わる。
- **wintf 単体の利用者（example 7 本）は「窓 0 で終了」に乗っている。** `clip_demo`・`dcomp_demo`・`dcomp_taffy_demo`・`graphics_reinit_test`・`postmessage_click_test`・`taffy_flex_demo`・`typewriter_demo` は、時間が来たら自分の窓を閉じるだけで終了の指示を持たない。
- **常設の smoke テスト `crates/areka/tests/smoke_boot_loop_exit.rs`** は `AREKA_APP_SMOKE_EXIT_MS=500` で areka を起こし、60 秒の見張りの中で終了コード 0 と起動ログの目印を確かめる。60 秒を過ぎるとプロセスを止めて失敗にする＝**「窓が無いのにプロセスが残る」壊れ方を赤にできる唯一の常設検査**である。

このままでは、ゴースト切替の途中（古いゴーストの窓を全部閉じてから新しいゴーストの窓を開く）で必ず来る「窓 0 の瞬間」にプロセスが終わってしまう。

### 正典（ukadoc）の位置づけ

本仕様が定めるのは areka のプロセスの寿命であり、これを直接定義する項は ukadoc に無い。ただし 2 項が前提を与える（逐語引用）:

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [`\-`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c-:1) | 「本体を終了する。SSPで複数ゴーストが起動している場合、実行したゴーストのみ終了する。」 | ゴーストの終了とアプリの終了は本来別のものである。areka は今日ゴースト 1 体なので**ゴーストの終了＝アプリの終了**の対応を保つ。複数ゴーストは範囲外。 |
| [`OnClose`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnClose:1) | 「終了が指示された際に発生。」 | 終了は**指示**によって始まる。本仕様はその指示の終端（プロセスを終える合図）を明示の 1 本にする。 |

### 何を変えるか

**areka のアプリは「終了の指示」で終わり、窓の数では終わらない。** 窓が 0 枚の瞬間があってもプロセスは生き続ける。今日の終了操作 6 種は、全窓を閉じたあとに明示の終了の指示を 1 つ出すようになり、利用者から見える終わり方は今までと区別が付かない。wintf には「窓 0 で終了するか」を利用側が選べる口と、明示の終了の指示を受ける口を足す。wintf の既定は変えず、example 7 本は 1 文字も書き換えない。

## Boundary Context

- **In scope**:
  - wintf: 「窓 0 で終了する／しない」を利用側が選べる口、明示の終了の指示を受ける口、既定（窓 0 で終了）の不変。
  - areka: 上の 4 つの場所（終了操作 6 種）への終了の指示の結線。終了のための全窓破棄と終了の指示が別々に呼ばれない形。
  - 常設の smoke テストの追随。
  - 「窓 0 でも生きている」「終了の指示で必ず終わる」の決定論テスト（それぞれ赤にできる）。
  - 実機での有界の自動終了による確認（終了後にプロセスが残っていないことを含む）。
- **Out of scope**:
  - 切替そのもの（`areka-P0-ghost-shell-balloon-switch`・`areka-P0-shell-balloon-switch`）。**「窓を全部閉じるが終了しない」操作は本仕様では作らない**——本仕様が保証するのは、その操作を後続が作ったときにプロセスが終わらない土台だけである。
  - kanade の終了理由（`KanadeStopCause`）に「切替」の値を足すこと（ゴースト切替の仕事）。
  - ダミー窓の経路を消すこと（`areka-P0-baseware-root-layout` の仕事）。本仕様は今在る経路に終了の指示を足すだけ。
  - トレイアイコン等の「窓が無いときの操作の入口」（棚卸⑭の裁定候補 ⑷＝含めない）。
  - 複数ゴーストの同時起動と、それに伴う「1 体だけ終了」「すべて終了」（`OnCloseAll`）の区別。
  - 窓 0 のままプロセスが残ったときに時間で自動終了する安全網。切替中は「窓 0 で生きている」のが正常なので、この安全網は切替を壊す。送り忘れは構造で防ぐ（要件 3.5）。
- **Adjacent expectations**:
  - kanade の終了の握手（`crates/areka-kanade/src/schedule/close.rs`・`user_break.rs`）は変えない。本仕様が変えるのは、握手の完了通知を受けた**あと**だけ。握手の決定論テストは 1 本も変えない。
  - `crates/areka-ghost/src/runtime.rs` の `fn shutdown` は変えない。
  - `areka-P0-baseware-root-layout` は `crates/areka/src/main.rs` の別の関数と `crates/areka/tests/smoke_boot_loop_exit.rs` を共有する。本仕様が先に着地し、相手は要件と設計だけ先行する。`main.rs` は 1 ファイル 1,000 行の目安に近い（958 行）ので、本仕様の変更後も目安の内側に収める。**相手への申し送り**: 相手の brief は引数なし起動（フォールバック方向）の smoke テストを陳腐化候補としているが、本仕様の要件 5.4 はこのテストを「窓が無いのにプロセスが残る」壊れ方を赤にする常設の見張りとして残す。相手が起動経路の目印（マーカー）を変えるのは構わないが、60 秒の見張りと終了コード 0 の判定は残すこと。
  - 後続のゴースト切替は、本仕様が wintf に足す「窓 0 で終了しない」設定の上で「窓を全部閉じて開き直す」を組む。本仕様はそのための口を用意するだけで、切替の順序や通知は持たない。

## Requirements

### Requirement 1: areka の寿命は終了の指示で決まる

**Objective:** As a 切替を実装する開発者, I want areka のプロセスが窓の数ではなく明示の終了の指示で終わること, so that 切替の途中で窓が 0 枚になってもプロセスが生き続ける

#### Acceptance Criteria

1. While areka が動作中, when 終了の指示が出されないまま窓の数が 0 になる, the areka shall 終了せず動作を続ける（メッセージループから戻らない）。
2. When 終了の指示が出される, the areka shall メッセージループから戻り、今日と同じ後始末（SERIKO の再生ループの停止 → ゴースト実行環境の終了統括 → SERIKO アクターの合流 → 性能報告の出力）を同じ順序で行い、終了コード 0 でプロセスを終える。
3. When 終了の指示が出される and 画面に窓が残っている, the areka shall 残っている窓をすべて閉じ終えてからメッセージループから戻る（後始末①〜④の間に利用者の画面へ窓を残さない＝今日、合図が鳴る時点で窓が既に無いのと同じ見え方を保つ）。
4. If 終了の指示が 2 回以上出される, then the areka shall 1 回目と同じ終わり方をし、2 回目以降で失敗や二重の後始末を起こさない。
5. If 終了の指示がメッセージループの開始より前に出される, then the areka shall ループの開始後ただちに終わる（指示を取りこぼしてプロセスが残ることがない）。

### Requirement 2: wintf の選択口と明示の終了の指示

**Objective:** As a wintf の利用側（areka）, I want 「窓 0 で終了するか」を構築時に選べ、明示の終了を指示できること, so that areka だけが寿命を切り離し、wintf 単体の利用者は今までどおりに済む

#### Acceptance Criteria

1. The wintf shall アプリの構築時に「窓の数が 0 になったら終了する／しない」を利用側が選べる口を 1 つ持つ。
2. The wintf shall 利用側が何も選ばないときの既定を従来どおり「窓の数が 0 になったら終了する」とする。
3. While 既定（窓 0 で終了する）, when 窓の登録が空でない状態から空になる, the wintf shall 従来どおりメッセージループを終える（example 7 本を 1 本も書き換えずに、今までと同じ終わり方になる）。
4. While 「窓 0 で終了しない」が選ばれている, when 窓の登録が空でない状態から空になる, the wintf shall メッセージループを終えず動作を続け、その後に新しい窓を開ける状態を保つ。
5. The wintf shall 利用側が明示的に終了を指示できる口を 1 本持つ。
6. When 明示の終了の指示を受ける, the wintf shall 「窓 0 で終了する／しない」の選択にかかわらずメッセージループを終える。
7. When 明示の終了の指示を受ける, the wintf shall 終了の指示を受けたことを記録に残す（ライフサイクル事象として info）。
8. The wintf shall 実行中に「窓 0 で終了する／しない」を切り替える口を持たない（仮の裁定 1・要件 6）。

### Requirement 3: areka の終了操作 6 種は同じ見え方で終わる

**Objective:** As a areka の利用者, I want 今日ある終了操作のどれを使っても今までと同じように終わること, so that 寿命の切り離しが利用者に見えない

#### Acceptance Criteria

1. When kanade の終了系列の完了通知を受ける（メニューの「終了」・別れの台詞のあとの終了・中断のあとの終了のいずれか）, the areka shall 全ゴースト窓を閉じ、終了の指示を出す。
2. When kanade の終了系列の完了通知を受ける and 閉じる窓が既に 1 枚も無い（強制退避が先に走った等）, the areka shall それでも終了の指示を出す。
3. When 強制退避（Ctrl＋Shift＋左ダブルクリック）が起きる, the areka shall 全ゴースト窓を閉じ、終了の指示を出す。
4. When ダミー窓が左ダブルクリックされる, the areka shall ダミー窓を閉じ、終了の指示を出す。
5. When smoke の自動終了の時間が来る, the areka shall 起動窓（ダミー窓／ゴースト窓）を閉じ、終了の指示を出す。
6. The areka shall 「終了のために全窓を閉じる」操作と「終了の指示を出す」操作を 1 つの操作として行い、終了経路のどこにも片方だけを呼ぶ形を残さない（仮の裁定 2・要件 6）。
7. When 終了の指示を出す, the areka shall どの終了操作から来たか（終了系列の完了・強制退避・ダミー窓・smoke）を記録に残す（ライフサイクル事象として info）。
8. The areka shall 終了操作 6 種のどれでも、終了の指示のあとの後始末の順序と終了コード（0）を今日と同じに保つ。

### Requirement 4: 既存の振る舞いを変えない

**Objective:** As a 開発者, I want 本仕様が触らない範囲の振る舞いと検査がそのまま残ること, so that 寿命の切り離しが他の経路を壊していないと言える

#### Acceptance Criteria

1. The 本仕様 shall `WinApp::new()`＋`run()` の形で終了を窓 0 に任せている example をすべて 1 本も書き換えない（2026-09-23 実測: wintf の 7 本＝`clip_demo`・`dcomp_demo`・`dcomp_taffy_demo`・`graphics_reinit_test`・`postmessage_click_test`・`taffy_flex_demo`・`typewriter_demo`、加えて同じ形の `crates/areka/examples/` 5 本と `crates/areka-emo-text/examples/` 2 本）。
2. The 本仕様 shall kanade の終了の握手（`schedule/close.rs`・`schedule/user_break.rs`）とその決定論テストを 1 本も変えない。
3. The 本仕様 shall `crates/areka-ghost/src/runtime.rs` の `fn shutdown` を変えない。
4. The 本仕様 shall wintf と areka の既存の決定論テスト（登録表の空遷移で合図が鳴ること・終了系列の完了通知で全ゴースト窓が閉じること・強制退避で全ゴースト窓が閉じることを含む）を 1 本も落とさない。
5. The 本仕様 shall 変更後も `crates/areka/src/main.rs` を 1 ファイル 1,000 行の目安の内側に収める。

### Requirement 5: 決定論テストと実機確認

**Objective:** As a 開発者, I want 「窓 0 でも生きている」と「終了の指示で必ず終わる」をそれぞれ赤にできること, so that 後続の切替が乗る土台が固定される

#### Acceptance Criteria

1. The 本仕様 shall 「窓 0 で終了しない」を選んだアプリで窓の登録が空になってもメッセージループが終わらないことを、決定論テスト 1 本で確かめる（この判断が壊れると赤になる）。
2. The 本仕様 shall 明示の終了の指示でメッセージループが終わることを、決定論テスト 1 本で確かめる（この判断が壊れると赤になる）。
3. The 本仕様 shall 足すテストを上の 2 つの判断分岐（と要件 1.4・1.5 の縁の条件）に限り、既に確かめられている配線（閉じる → 登録表から外れる → 合図が鳴る）を再テストしない。
4. The 本仕様 shall 常設の smoke テスト `crates/areka/tests/smoke_boot_loop_exit.rs` を変更後も緑にし、「窓が無いのにプロセスが残る」壊れ方を 60 秒の見張りで赤にする役をそのまま保つ。
5. When 実機で確認する, the 開発者 shall 有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）とログの絞り込みで終了操作の見え方を確かめ、終了後に areka のプロセスが残っていないことを見る。
6. While 実機で確認する, the 開発者 shall 自分が起こしたと確認できたプロセス以外を止めない。

### Requirement 6: 裁定候補と既定（仮の裁定）

**Objective:** As a 開発者, I want brief が挙げた裁定候補の既定を要件の段階で固定しておくこと, so that ディスカッションで覆されない限り設計がこの形で進む

#### Acceptance Criteria

1. The 本仕様 shall **仮の裁定 1（口の形）**として、wintf の選択口を「アプリの構築時の設定値」＋「明示の終了の指示 1 本」の形とし、「実行中に切り替えられる資源」と「終了の仕掛けを利用側が差し替える」の 2 案を採らない（実行中の切替は「戻し忘れ」という新しい状態を作る。開発規律: 状態を増やさず形を変える）。
2. The 本仕様 shall **仮の裁定 2（送り忘れの防止）**として、areka の終了経路で「終了のための全窓破棄」と「終了の指示」を 1 つの操作にまとめ、片方だけを呼べない形にする。時間で自動終了する安全網は持たない（切替中の「窓 0 で生きている」を壊すため）。
3. Where 開発者が仮の裁定を覆す, the 本仕様 shall 要件・設計の該当箇所を裁定に合わせて改める。

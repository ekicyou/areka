# Brief: areka-P0-sakura-time-critical

> 2026-10-03 `areka-P0-emo-text-file-split` の完了フロー（未解決問題の棚卸）で起票。出どころは同 spec の実機の確かめで開発者が見た「メニューを出した後に後続のトークが止まってないときがある」（`completed/areka-P0-emo-text-file-split/verification/notes.md` §3「持ち越しの議題」）。調べた結果、観測そのものは**正典どおり**で、正典から外れているのは本 spec の `\t` だけだった。file:line は起票時値（ブランチ `claude/areka-p0-emo-text-split-dc4e04`・HEAD `b6f22e53`）。

## Problem

ゴーストの作者は、台本に `\t`（タイムクリティカルセクション）を書くと「この先は台本の終わりまで、なでなでやクリックの反応で話を割り込ませない」と指定できる。メニューや大事な台詞を最後まで見せたいときに使う正典の手段である。areka は `\t` を読まないので、`\t` を書いたゴーストでも、話の最中にマウスを動かすだけでなでなでの返事に置き換わる。

## Current State

**観測（2026-10-03・正典どおりと判定）**: 体のダブルクリックでメニュー（選択肢あり・`OnMouseDoubleClick` の返事）が出た 0.5 秒後、体の上のマウスの動きで `OnMouseMove` の返事が届き、`steady_talk_replace` でメニューが置き換わった。

- 正典の根拠: ukadoc の `\t`（`https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5ct:1`）は「スクリプトブレーク(例:選択肢を選ぶ、バルーンダブルクリックによる中断)か\eまでの間、マウス系などのイベント通知を行わない」。止めたい区間を台本で明示する作り＝既定では話の最中（選択待ちを含む）もマウス系は届き、返事の台本は今の話を中断して再生される。`Status` には `talking`／`choosing`／`timecritical` があり（`spec_shiori3.html#Status_20_5bSSP_62e1_5f35_5d:1`）、SHIORI 側が状態を見て黙る前提（里々の既定「トーク中・誘導中・…時間クリティカル中は反応しません」が SHIORI 側の自衛の例）。
- areka の裁定は既にこの形: 完了 `areka-P0-input-events` の DD-IE-1（話の最中のマウスも常に GET）・DD-IE-2（話の最中に届いた返事は置換・根拠は SSP 2.3.86 の通信記録と「`\t` が防ぐ側の opt-in であること自体が既定＝中断の証左」）。完了 `areka-P0-choice-select-events` 要件 4.3 も選択待ちを同じ単一 slot 調停に従わせる。
- 置換の述語は `crates/areka-kanade/src/schedule/events.rs` の `value_replaces_active_talk`（`OnSecondChange` 以外は置換）、調停は `steady.rs` の `on_reply`。`OnSecondChange` は話の最中は NOTIFY・Reference3=0 で出て返事は捨てる（ukadoc どおり）。
- **`\t` は未実装**: `areka-sakura` に `\t` を読む処理が無い。`crates/areka-kanade/src/status.rs` の `ExecutionState::TimeCritical` は語彙だけあり、出どころの無い差し替え口（常に無効）のまま。`\t` の区間でマウス系の通知を止める仕組みも無い。
- 隣の `areka-P0-status-execution-states`（C1・バグ）の表に「timecritical | `\t` 区間中 | sakura 再生（`\t`）」の行があるが、受け持つのは `Status` に載せることだけで、出どころ（`\t` の区間）は「まだ無い」と書いている。同 spec の完了時（2026-10-03）に、`\t`（ukadoc 網羅の台帳 `ukadoc:list_sakura_script:_5ct:1`）と `timecritical` の持ち主は本 spec へ付け替えた（同 spec 要件 6.3 の記録）。

## Desired Outcome

- 台本の `\t` を読み、その位置から「台本の終わり（`\e`）か、スクリプトブレーク（選択肢を選ぶ・バルーンのダブルクリックでの中断）」までをタイムクリティカルの区間として持つ。
- 区間の間は、マウス系などのイベント通知を SHIORI へ送らない（どの語を止めるかは正典の「マウス系など」を ukadoc で引き直して要件で確定する）。
- 区間の間、`Status` に `timecritical` を載せる（`status-execution-states` の差し替え口へ出どころを渡す）。
- `\t` の無い台本の振る舞いは 1 つも変えない（DD-IE-1／DD-IE-2 の置換はそのまま）。

## Approach

`\t` を台本の位置つきの合図として cue へ写し（`\![enter/leave,nouserbreakmode]` を `user_break_cue.rs` 経由で届けている完了 `balloon-break` の形が先例）、kanade が区間の開始・終了（台本の終わり・中断・選択の確定）を知って、マウス系の入口（`schedule/mod.rs` の `Input::Mouse` を Steady 相で受ける所）で通知を止める。`Status` の `timecritical` は同じ区間の旗から導く。

## Scope

- **In**: `\t` の字句・cue への写し／区間の開始と終了（`\e`・スクリプトブレーク・選択の確定・中断・置換）／区間の間のマウス系などの通知の抑止／`Status` の `timecritical` の出どころ／決定論テスト（区間の内外・終わり方ごと）
- **Out**: `\t` の無い台本の調停の変更（DD-IE-2 の置換は正典どおりで変えない）／ゴースト（emo2 の辞書）の作りの修正（なでなでの返事が `Status` を見て黙るかはゴースト側の領分）／`OnSecondChange` の扱い（既に正典どおり）

## Boundary Candidates

- 台本の側: `\t` の字句と cue の合図（`areka-sakura` の lexer・compile）
- 運行の側: 区間の旗とイベントの抑止（`areka-kanade` の `schedule/`）
- 状態の側: `Status` の `timecritical`（`areka-kanade/src/status.rs`・`status-execution-states` の差し替え口）

## Out of Boundary

- 話の最中の返事の置換の規則そのもの（完了 `input-events` DD-IE-2・`choice-select-events` 4.3 の正本）
- `quicksection`・`balloonwait` などの時間の指令（`sakura-time-directives`）
- 中断禁止 `\![enter,nouserbreakmode]`（完了 `balloon-break`）

## Upstream / Downstream

- **Upstream**: `areka-P0-status-execution-states`（`Status` の差し替え口を実値化する枠組み・C1）／完了 `input-events`・`choice-select-events`・`balloon-break`
- **Downstream**: `\t` を使う実ゴーストの適合

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）
- **Adjacent**: `status-execution-states`（`timecritical` の行を本 spec が出どころとして満たす）・`sakura-time-directives`（同じ compile の時間まわりだが別の語）・台本のコンパイルの列（`compile.rs` を共有する spec と同時に走らせない）・kanade の進行の列（`schedule/` を共有する spec と同時に走らせない）

## Constraints

- 正典は ukadoc。`\t` の終わり方と止めるイベントの範囲は要件の段で ukadoc を引き直して確定する（SSP 実測主義は取らない）。
- 決定論テスト必達。`\t` の無い台本の既存テストは 1 本も変えない。
- 規模の見立て: S〜M（6〜10 タスク）。段は「その他」（正典の穴で、`\t` を使うゴーストが現れるまで利用者に見える害は無い）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S〜M（7〜10 タスク）。切る: なし。
- 前提の状態: `status-execution-states`（PR#220）は着地済み。差し替え口は `crates/areka-kanade/src/status.rs` の導出表の「5. timecritical ← SEAM(Req6.1/6.3)」の行。
- 崩れた前提／古くなった位置:
  - `\t` は今、字句で裸の `t`（`Token::Bare`）になり、`crates/areka-parsers/src/sakura/decode.rs` の `decode_bare` の残りの腕 → `decode_passthrough_bare` → `Instruction::Raw("\\t")` → コンパイルで捨てられる。
  - 真似る雛形は `nouserbreak` の作り（`status-execution-states`・`balloon-break`）がそのまま使える: 台本の合図を cue へ写す → 受け口（`crates/areka/src/emo2_boot/user_break_cue.rs` の `NoUserBreakCueSink` と同じ形）→ `KanadeMsg::ExecutionState(ExecutionStateUpdate::…)` → `status.rs` の `ExternalStates` の写し → `State::snapshot_with_choice`（`schedule/mod.rs`）が「再生中のトーク かつ 写しの旗」で `ExecutionSnapshot` に載せる。
  - **コンパイルを触らずに済む道がある**: `decode_bare` で `"t"` を内部の `\!` キャリアへ写せば（裸の `\+` を `\![change,ghost,random]` へ写している腕が先例）、汎用キャリアのまま cue へ届く＝`crates/areka-sakura/src/compile.rs`（台本のコンパイルの列）に触れない。正典の `\t` の名前とは別の内部名を使うので、消費者の台帳の選び手が台本の書き手から見える `\![…]` とぶつからないことを要件で確かめる。
  - マウス系の抑えは `crates/areka-kanade/src/schedule/steady.rs` の `on_mouse` の先頭（今の「終了の握手の待ちは送らない」防御の隣）に置くのが素直。`schedule/mod.rs` の `route` の `Input::Mouse` の腕に置けば `steady.rs` に触れずに済む（`mod.rs` は 937 行・`steady.rs` は 929 行＝どちらも上限に近い。前回までの「`steady.rs` は分割済み」は誤りで、分割はされていない）。
  - `mouse-drag-events` が `OnMouseDragStart`／`OnMouseDragEnd` を同じ `KanadeMsg::Mouse` の道で足す＝抑えを `on_mouse`（または `route` の腕）に置けばドラッグの 2 語も自動で止まる。逆にどちらかを `RaiseEvent` の道で送る spec が出ると抑えから漏れる。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/sakura/decode.rs`（`decode_bare` の `"t"` の腕）
  - `crates/areka-kanade/src/status.rs`（`ExecutionStateUpdate`・`ExternalStates`・`ExecutionSnapshot`・導出表の 5 行目）・`status_derive_tests.rs`
  - `crates/areka-kanade/src/schedule/mod.rs`（`snapshot_with_choice`・`on_execution_state` の記録の名前・`route` の `Input::Mouse` の腕に抑えを置くならここ）または `schedule/steady.rs`（`on_mouse`）
  - 新規 `crates/areka/src/emo2_boot/time_critical_cue.rs`・`emo2_boot/consumer_ledger.rs`（859）・`emo2_boot/mod.rs`（883・受け口の並び）
  - `doc/ukadoc-coverage/ledger/sakura-script.toml`（`_5ct:1`）
- 議題（答えで作業が変わるものだけ）: 区間で止める「マウス系など」の範囲（`OnMouseMove`・`OnMouseDoubleClick`・着地後の `OnMouseDrag*`・`OnFileDrop2` などの投げ込み・`OnSecondChange` を含めるか）。ukadoc の本文は「マウス系などのイベント通知」とだけ書く＝要件で決める（前回どおり）。
- 見つけた穴: 区間の終わり方のうち「選択の確定」と「置き換え」は、どちらも新しいトークが始まる＝「再生中のトーク かつ 写しの旗」の式だと、UI の旗を下ろし忘れると新しいトークまで区間が続く。`nouserbreak` の旗を誰がいつ下ろしているかを着手のときに読み、同じ点で下ろす（`\t` は「再度書いても解除されない」・`\e` かスクリプトブレークまで）。並走の照合: `mouse-drag-events` とは `on_mouse` を分け合う（抑えを `route` の腕へ置けば文字の衝突は無い）。`balloon-lifecycle-events` とは kanade で同じ関数を触らない見込み。
- 追記（棚卸㉑の分割の指示）: kanade の `schedule/steady.rs`（929 行）・`schedule/mod.rs`（937 行）は分割されていない。本 spec の変更を足して 1,000 行を超えるなら、先頭のタスクで分割する。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（7〜10）のまま。切る: なし。
- 前提の状態: `status-execution-states` は着地済み＝着手できる。kanade の列では `balloon-lifecycle-events`（C4 予定）の後ろ。
- 崩れた前提／古くなった位置:
  - `mouse-drag-events`（✅ 10-05）は `OnMouseDragStart`／`OnMouseDragEnd` を同じ `KanadeMsg::Mouse` の道で足した（`msg.rs` の `MouseEventKind::DragStart`／`DragEnd`・`steady.rs` の `on_mouse` の腕）＝抑えを `on_mouse` の先頭か `schedule/mod.rs` の `route` の `Input::Mouse` の腕に置けばドラッグの 2 語も止まる（前回の見込みどおり）。
  - **同じ場所に先客の印がある**: `on_mouse` のドラッグの腕に `SEAM(Req7.3)`（正典はパッシブモードでドラッグの 2 語を抑える・抑えは「終了の握手の待ちの防御と同じ並び＝組み立ての前」に置く）と書かれた。`\t` の抑えと同じ置き場所＝抑えの判定を 1 つの関数にまとめ、パッシブモードが後から乗れる形にしておく。
  - 前回の穴（旗を下ろし忘れると次のトークまで区間が続く）は雛形で解ける: `crates/areka/src/emo2_boot/user_break_cue.rs` の `NoUserBreakSignal` は `TalkStarted`（閉じ忘れを解く）と `TalkEnded`（下ろす）の二重の守りを持つ。同じ 4 値の形を写す。
  - 行数: `steady.rs` 947・`schedule/mod.rs` 938（どちらも上限の近く）。抑えの判定は新しいファイル（例 `schedule/time_critical.rs`）に置き、`route` の腕か `on_mouse` から 1〜3 行で呼ぶ＝分割は要らない見込み。`status.rs` 538・`decode.rs` 400（`decode_bare` の `"+"` の腕が写しの先例のまま）・`consumer_ledger.rs` 859・`emo2_boot/mod.rs` 883。
- 触るファイル:
  - `crates/areka-parsers/src/sakura/decode.rs`（`decode_bare` に `"t"` の腕）
  - `crates/areka-kanade/src/status.rs`（`ExecutionStateUpdate`・`ExternalStates`・導出表の `timecritical` の行）・`status_derive_tests.rs`
  - `crates/areka-kanade/src/schedule/mod.rs`（`snapshot_with_choice`・`route` の `Input::Mouse` の腕）または `schedule/steady.rs`（`on_mouse`）＋新規 `schedule/time_critical.rs`
  - 新規 `crates/areka/src/emo2_boot/time_critical_cue.rs`・`emo2_boot/{consumer_ledger.rs, mod.rs}`
  - `doc/ukadoc-coverage/ledger/{sakura-script,shiori}.toml`（`_5ct:1` と `Status` の `timecritical`）
- 議題（答えで作業が変わるものだけ）: 止める「マウス系など」の範囲（前回どおり。ドラッグの 2 語は同じ道なので既定で入る）。
- 見つけた穴: なし。並走の照合: kanade の列の `balloon-lifecycle-events` と同時に走らせない。台本のコンパイルの列（`compile.rs`）には触れない。

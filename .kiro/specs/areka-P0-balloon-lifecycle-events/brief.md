# Brief: areka-P0-balloon-lifecycle-events

> 起票: 2026-09-11（棚卸⑬・`areka-P0-balloon-canon-residue` の 3 軸分割 ⑵＝表示寿命側の項目 7〜10 を独立 spec に切り出し）。項目本文の正本は分割元 brief の「`balloon-visibility` からの追加登記（2026-08-12）」節。ここでは所有と着地条件だけを書き、本文は重複させない。

## Problem

完了 spec `balloon-visibility` が語彙・Reference 割当・受け渡し口の型（`BalloonLifecycleNotice`・`crates/areka/src/emo2_boot/talk_lifecycle.rs`・`#[allow(dead_code)]` で本 spec 群を名指し）までを残し、実装を先送りした 4 項目が消費者ゼロのまま残っている:

7. `\![set,balloontimeout,時間]` の実導出（表示側）。
8. `OnBalloonClose`／`OnBalloonTimeout`／`OnBalloonBreak` の SHIORI 発火（UI→kanade の通知路＋kanade 送出側の受理）。
9. `\x`／`\x[noclear]`（クリック待ち＝会話の進行を止める機能・可視性側での近似実装は禁止）。
10. 中断で終わった会話のタイムアウト起点の精密化（8 と一体・単独で先行させない）。

## Current State

2026-09-11 実測: `BalloonLifecycleNotice` の `#[allow(dead_code)]` 注記「消費者ゼロ（意図的予約・Requirement 7.8）: 実発火は areka-P0-balloon-canon-residue が所有」が逐語で現存（**所有者名は本 spec へ読み替える**・分割元 brief の追記に登記）。`noclear`・`balloontimeout` は `crates/` で 0 件。emo2 は 3 イベントとも消費者ゼロ・`\x` も `balloontimeout` も辞書に無い（M1 実害なし）。

## Desired Outcome

4 項目が着地し、`#[allow(dead_code)]` の予約が外れる。`\x` はクリックで会話が進み、`\x` は scope を `\0` へ戻して `\f` 系の効果を解除、`\x[noclear]` は内容・scope・`\f` 系を保持する。3 イベントが正典の Reference で発火し、中断時のタイムアウト起点は中断時刻になる。

## Approach

kanade（会話進行）と UI（表示寿命）の間に通知路を 1 本敷く（8・10 の情報は同一）。9 は「会話を止める」機構＝sakura の再生に待ち相を足す（可視性の規則には触れない）。7 は表示側の既定 30 秒を台本の指定で上書きする読み口。

## Scope

- **In**: 項目 7（表示側）・8・9・10。
- **Out**: 項目 7 の compile 側（`\![set,balloontimeout]` の台本コンパイル時の干渉＝`sakura-time-directives` 所有・双方が揃って初めて 7 が成立）／「`\f` 状態の何がリセットされるか」の権威定義（`text-decoration-canon` が供給・本 spec は消費）／可視性の判断規則そのもの（`balloon-visibility` で完成・不変）。

## Boundary Candidates

- 通知路（8・10）／`\x` の待ち相（9）／`balloontimeout` の読み口（7）の 3 片。8・10 は一体。

## Out of Boundary

- 系列解決（分割元 ⑴）・emo-text 側の残件（`emo-text-canon-residue`）。

## Upstream / Downstream

- **Upstream**: `text-decoration-canon`（9 のリセット意味論・**先行必須**）・`sakura-time-directives`（7 の compile 側・**先行必須**）・完了 `balloon-visibility`（送り元）。
- **Downstream**: これらを使う実ゴーストの適合（M2）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-balloon-canon-residue`（分割元・7〜10 を引き継ぐ）。
- **Adjacent**: `translate-pipeline`（kanade `schedule/events.rs` を共有＝同居不可）・`sakura-time-directives`。

## Constraints

- 編集集合の見込み: `crates/areka/src/emo2_boot/talk_lifecycle.rs`・`crates/areka-kanade/src/schedule/{events,steady}.rs`・`crates/areka-sakura/`（`\x` の待ち相）・`doc/COMPAT_ARCHITECTURE.md` §8。
- 正典の曖昧点 1 件を要件で裁定: `balloontimeout` の「`0` または `-1`」（同一項で表現が割れ `-2` の扱いが曖昧）。
- 決定論テスト必達（3 イベントの発火・`\x` の 2 形・中断起点）。要件定義は Opus で足りる（裁定は上の 1 件と `\x` の scope リセット範囲の 2 件）。

> **📌 2026-09-20 相互登記（`areka-P0-balloon-break` 起票）**——利用者による中断操作（バルーンの左ダブルクリック→再生停止→全バルーン即時非表示）は `areka-P0-balloon-break`が所有する。本 spec の項目 8 の `OnBalloonBreak` はその通知（どのスコープで中断が起きたかを kanade まで届ける）を起点に発火させる。**中断位置（Reference2）の源は `balloon-break` でも作らない**＝台本上の位置を compile→cue→再生へ通す工事は本 spec に残る。項目 10 は、利用者による中断では即時非表示になるので、対象は選択肢タイムアウト・後続トークの割り込みなど残りの中断だけになる。
>
> **📌 2026-09-13 相互登記（`areka-P0-text-decoration-canon` 着地）**——項目 9 の「`\f` 状態の何がリセットされるか」の権威定義は `crates/areka-emo-text/src/state_decoration.rs` の `TextLayerState::reset_decoration(scope)` で、`\x` はこれを `None`（全スコープを 1 回で戻す）で呼び、`\x[noclear]` は呼ばない——という配線を本 spec が足す（親 spec の着地時点で `TextLayerState::reset_decoration` を呼ぶ**本番の経路は 0 件**——同関数はテストからのみ呼ばれており、本 spec が最初の本番呼び出し元になる。本番で実際に戻しているのは同じファイルの `ActorTextState::reset_look`／`reset_look_disabled` を通る 3 経路〔`\f[default]`／`\f[disable]`／台詞開始の `ClearAll`〕で、`reset_decoration` もこれらと同じ共通実体 `ActorTextState::reset_look_to` へ落ちるので、戻し方は 1 つに保たれる）。

---

> **📌 2026-10-01 `/kiro-discovery`（シェル内バルーン）による引き取り**——**項目 9（`\x`／`\x[noclear]`＝クリック待ち）は新 spec `areka-P0-talk-fast-forward` が引き取った**。クリックでの早送り（areka 独自・話している最中のバルーンの 1 クリックで次の `\x` か台詞の終わりまで進む）と、`\x` での停止と再開と、クリック待ちの印 `clickwaitmarker.*` を 1 つの spec に揃えた（どれも「クリックで台詞の時計を進める」同じ仕組み）。開発者裁定「クリック待ちは台本に明示した `\x` だけ・自動でクリックを求める仕組みは作らない」もそちらの brief にある。**本 spec に残るのは項目 7・8・10**（`balloontimeout` の実導出・`OnBalloonClose`／`OnBalloonTimeout`／`OnBalloonBreak` の発火・中断で終わった会話のタイムアウト起点）。上の 2026-09-13 の相互登記（`\x` で `\f` 状態の何が戻るか）は `talk-fast-forward` がそのまま使う。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 残りは項目 7・8・10（項目 9 の `\x` は `talk-fast-forward` へ移した＝`text-decoration-canon` への依存も消えた）。優先度 中。
- **項目 8 と 10 は今すぐ始められる**: 完了 `balloon-break` が `Input::UserBreak{scope}` と `schedule/user_break.rs` を入れたので、`OnBalloonBreak` へ届く道は既に在る。項目 7（`balloontimeout`）だけが `sakura-time-directives` を待つ＝**着手するときは 8 と 10 を先に、7 は切り離す**。
- 変わっていない点: `BalloonLifecycleNotice` は予約のまま（`emo2_boot/talk_lifecycle.rs`・コメントの持ち主の名前は棚卸⑳で本 spec へ直した）・`OnBalloonClose`／`OnBalloonTimeout`／`OnBalloonBreak` は許可の表に無い。
- **触るファイル**: kanade の `schedule/{steady.rs 935, events.rs}`・`crates/areka/src/emo2_boot/{talk_lifecycle.rs, balloon_visibility 系}`・`crates/areka/src/input_events/` のバルーン。大きいファイルの分割は `emo-text-file-split` と `translate-pipeline`（`steady.rs`）が先に済ませる。

## 2026-10-03 ウェーブ C3-④（予定・10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」））

- 段は「優先」。C3 は C2 の着地で brief が動くので、着手の前に同じウェーブの他の spec と触るファイルを照合し直す（`roadmap.md`「ウェーブ編成」の C3 の行）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: Reference2（中断位置）を作るかで変わる。作らなければ M（9〜13 タスク）。作るなら L（18〜24）で、台本のコンパイルの列をまたぐ。20 を超える形になるなら「切る」: ⒜ 項目 8・10 の発火（Reference2 は空で送り、縮退を登記）＝M ／ ⒝ 中断位置を字句から完了の知らせまで通す工事＝M（台本のコンパイルの列の空きで）。順は ⒜→⒝。項目 7 は今までどおり `sakura-time-directives` の後。
- 前提の状態: 項目 8・10 の前提は着地済み（`translate-pipeline`・`emo-text-file-split`・`balloon-break`）。項目 7 の前提 `sakura-time-directives` はまだ。
- 崩れた前提／古くなった位置:
  - **`steady.rs` は分割されていない**（`translate-pipeline` は出口で捕まえる設計にした・929 行）。前回の「大きいファイルの分割は `translate-pipeline` が先に済ませる」は外れ。
  - `OnBalloonBreak` のきっかけは kanade の中にもう在る: 利用者の中断は `schedule/user_break.rs` の `on_user_break` が `state.user_break_talk` に控え、`schedule/mod.rs` の `on_talk_done` が `TalkDone{Interrupted}` で受ける。Reference0（台本）は `ActiveTalk.script` に在る。Reference1（scope）は `Input::UserBreak{scope}` に在るが、今は控えていない（`user_break_talk` は `TalkId` だけ）。
  - **Reference2（中断位置）の源はどこにも無い**: 字句（`areka-parsers/src/sakura/lexer.rs` の `Token`）・`Instruction`・コンパイル・dola の cue・完了の知らせ `areka_talk::TalkDone` のどれにも台本の中の位置を持たない。足すなら `TalkDone` に欄を足すことになり、`TalkDone {` の書き方は 27 ファイル・60 か所（kanade 19・sakura 3・ghost 2・talk 2・areka 1）。
  - `OnBalloonTimeout`／`OnBalloonClose` は表示の側（`emo2_boot/balloon_visibility*.rs`）が時間切れを知る。送る道は汎用の `KanadeMsg::RaiseEvent` で足りる（時間切れのときは台詞が終わって定常・トーク無し）＝kanade の運行表は表へ 3 行足すだけで済む見込み。
  - 3 語の返事の台本も `translate-pipeline` の出口を通り、`OnTranslate` に掛かる（作業は増えない・テストの期待に `OnTranslate` が載る）。
  - 網羅台帳の 3 行（`OnBalloonClose:1`・`OnBalloonTimeout:1`・`OnBalloonBreak:1`）と `\![set,balloontimeout]` の行の `owner` は、まだ分割元の `areka-P0-balloon-canon-residue` のまま。
- 触るファイル（並走の照合用）:
  - `crates/areka-kanade/src/schedule/events.rs`（`ALLOWED_EVENT_IDS` の末尾 3 行・組み立ての関数）・`schedule/events_change_tests.rs`（個数）・`crates/areka-kanade/src/lib.rs`（`pub mod events`）
  - `crates/areka-kanade/src/schedule/user_break.rs`・`schedule/mod.rs`（`on_talk_done` の中断の腕）・`schedule/mod.rs` の `State`（scope を控えるなら欄 1 つ＝`..` なしで `State` を組むテスト 14 か所）
  - `crates/areka/src/emo2_boot/talk_lifecycle.rs`（`BalloonLifecycleNotice` の予約を外す）・`emo2_boot/balloon_visibility.rs`・`balloon_visibility_wait.rs`・`balloon_visibility_phase.rs`
  - `crates/areka/src/input_events/user_break.rs`（中断の scope を運ぶなら）。`input_events/mod.rs` は触らない見込み
  - Reference2 を作るなら追加で: `crates/areka-parsers/src/sakura/{lexer,model,decode}.rs`・`crates/areka-sakura/src/{compile,drive}.rs`・`crates/dola/src/cue/`・`crates/areka-talk/src/lib.rs`
  - `doc/COMPAT_ARCHITECTURE.md` §8・`doc/ukadoc-coverage/ledger/{shiori,sakura-script}.toml`
- 議題（答えで作業が変わるものだけ）:
  1. Reference2（中断位置）を今作るか。作るなら字句から完了の知らせまで位置を通す工事（台本のコンパイルの列と同時に走れない・規模が倍）。作らないなら空で送って COMPAT §8 に縮退を登記し、別 spec に切る。
  2. areka で `OnBalloonClose` が起きる場面はどれか（areka のバルーンには閉じるボタンが無く、ダブルクリックは中断＝`OnBalloonBreak`）。時間切れ以外に「閉じる」が無いなら、`OnBalloonClose` は台詞の差し替え・`\c` などで閉じたときだけになるのか、要件で決める。
- 見つけた穴: 網羅台帳の `owner` が分割元のまま（上記）。実害は無いが、着地のときに本 spec 名へ直す。並走の照合: `mouse-drag-events` とは `events.rs` の表の末尾・個数の行・`lib.rs` の `pub use` の 3 か所で文字が必ず衝突する（中身は独立・後着が足し直せば済む）。`areka` のクレートでは同じファイルを触らない（向こうは `input_events/mod.rs` と新規 `drag.rs`）。
- 追記（棚卸㉑の分割の指示）: kanade の `schedule/steady.rs`（929 行）・`schedule/mod.rs`（937 行）は分割されていない。本 spec の変更を足して 1,000 行を超えるなら、先頭のタスクで分割する。

# Brief: areka-P0-range-choice-tag

> 2026-10-05 `/kiro-discovery` で起票。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。開発者裁定（同日・議題 4）＝`anchor-tag-canon` に足さず、別の spec にして `anchor-tag-canon` の後に作る。

## Problem

- **利用者**: 文字の 1 行より大きな範囲（複数行・画像のバナー）を 1 つの選択肢にしたいゴースト作者。
- 正典の `\__q[ID,...]…\__q`（ukadoc）＝「\__qがくるまでの範囲を選択肢とする。ID仕様は、Onやscript:で始まるものが特別扱いされる点や、ID引数の扱いまで含めて\qと同じ」「改行、\_l[50,50]カーソル移動も可能で、画像を貼れば画像が選択肢になる」。
  - 例: `\__q[script:\j[https://example.com/]]\_b[banner.png,inline]\__q`＝バナーを押すとサイトが開く。
  - 例: `\__q[OnTest,ref]今日のおすすめ\n（クリックで詳しく）\__q`＝2 行のどこを押しても同じ選択肢。
- areka では `\__q` を読まず、範囲の中身はただの文字として出て、押しても何も起きない。

## Current State

- `\__q[…]` と閉じの `\__q` は `crates/areka-parsers/src/sakura/decode.rs` の素通しの腕で `Raw` になり、`compile.rs` が捨てる。
- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\__q` の行は「無い」・引受先なし。
- `anchor-tag-canon` の brief に「`\__q` の仕組みは実装済み」とあったが誤り（本 discovery で同 brief を直した）。
- 選択肢の当たりの行は `crates/areka/src/input_events/balloon.rs` の `hit_choice_row`（1 行の選択肢）。範囲の当たりは `anchor-tag-canon` が作る。

## Desired Outcome

- `\__q[ID,…]…\__q` の範囲（複数行・カーソル移動・画像を含む）全体が 1 つの選択肢として押せ、`\q` と同じ選択肢の待ち・タイムアウト・`OnChoiceSelect(Ex)`・`On` で始まる ID・`script:` が動く。普通のバルーンとシェルの中の箱の両方で。
- 自動改行が行われる（ukadoc「\q[]タグと異なり、自動改行も行われる」）。
- 選択中の見た目は既定の 1 種類（選択肢の飾り分けは `choice-marker-styling`）。

## Approach

- `anchor-tag-canon` が作る「範囲を押せるようにする」仕組み（範囲の当たりの行の集合）をそのまま使う。違うのは結論が選択肢（待ちの柵・タイムアウト・選択肢のイベント）であること。
- `script:` は `choice-script-prefix` の道を通す。

## Scope

- **In**: `\__q` の読み込み・コンパイル・範囲の当たり・選択とホバーの見た目・選択肢の待ちとイベント・箱の中・決定論テスト・台帳の行。
- **Out**:
  - 選択肢の飾り分け（`choice-marker-styling`）。
  - 右クリックのコピー・ホバーの説明（`link-context-copy`・`balloon-link-hover`。本 spec の後に作るので範囲の選択肢もそこで拾う）。

## Boundary Candidates

- 読み込みとコンパイル（範囲の開きと閉じ）。
- 範囲の当たりと選択肢の結論（input_events）。

## Out of Boundary

- アンカー `\_a`（`anchor-tag-canon`）。

## Upstream / Downstream

- **Upstream**: `anchor-tag-canon`（範囲の仕組み）・`choice-script-prefix`（`script:`）。
- **Downstream**: `link-context-copy`・`balloon-link-hover`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `anchor-tag-canon`（同じ範囲の仕組み・brief を直した）・`choice-marker-styling`。

## Constraints

- 文字とバルーンの列と台本のコンパイルの列（`anchor-tag-canon` の次）。
- 画像の選択肢は `\_b[…,inline]` の対応の状況に依る（要件の段で確かめる）。
- 段: 優先（バルーン関係）。規模の見込み M（10〜14）。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `choice-script-prefix` が着地した。`script:` の選択肢は kanade の `crates/areka-kanade/src/schedule/steady_choice_script.rs` が新しいトークとして走らせ、`script:` の綴りは `schedule/choice.rs` の `script_body` の 1 か所にある。選ばれた ID だけを見て動くので、`\__q` の ID もそのまま通る見込み。
  - `anchor-tag-canon` は未着手。台本の読み手（`crates/areka-parsers/src/sakura/decode.rs`）は、どの腕も「読めなかった印」を一緒に返す形になった。`\__q` は今も素通しで、検査 `parse_bare_tag_tests.rs` が素通しを固定している。
  - 選択肢の範囲は、今も「字の通し番号の範囲」で持っている（`crates/areka-emo-text/src/state.rs` の `ChoiceSpan`）。行ごとの割り当て（`choice.rs` の `annotate_lines`）もあるので、開きと閉じの 2 つの合図で範囲を作る形にすれば使い回せる見込み。
- **触るファイル**:
  - `crates/areka-parsers/src/sakura/{decode.rs, model.rs, mod.rs}` と検査 `parse_bare_tag_tests.rs`。
  - `crates/areka-sakura/src/compile.rs`（選択待ちの区切りを出す条件が、今は `\q` の合図だけを数える）・`drive.rs`（選択肢の ID の一覧）。
  - `crates/areka-emo-text/src/{state.rs, choice.rs, actor_present.rs, actor.rs}` と新しい検査ファイル＝`lib.rs` の一覧の席を使う。
  - `crates/areka/src/input_events/{balloon.rs, shell_box.rs, shell_box_handler.rs}`（`anchor-tag-canon` の当たりをそのまま使えれば減る）。
  - 網羅台帳 `sakura-script.toml` の `\__q` の行（持ち主が空のまま）・`doc/COMPAT_ARCHITECTURE.md` §8。kanade は触らない見込み。
- **規模**: M（10〜14 タスク）。
- **先に要るもの**: `anchor-tag-canon`（範囲の当たり）。`choice-script-prefix` は着地済み。同じ列の `link-context-copy`・`balloon-link-hover` はこの後。
- **優先度の区分**: A（開発者の 10-05 の問い「リンクをクリックしたらファイルを開けるか」から起票し、開発者が別の spec にすると決めた）。
- **要件定義のモデル**: Fable（ukadoc の記述が薄い所＝自動改行・閉じ忘れ・範囲の途中の `\q` や `\_a`、読み手から入力まで複数のエンジン）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - 画像の選択肢に要る `\_b[…,inline]`（バルーンへの画像の貼り付け）は areka に無く、網羅台帳の持ち主も空、roadmap にも載っていない。本 brief の例 1（バナーを押すとサイトが開く）は本 spec だけでは動かない＝Out に書き、`\_b` を別に起票する。
  - `seriko-trigger-intervals`・`talk-fast-forward`（台本の読み手と `compile.rs`）とは同じウェーブに置けない。

## 2026-10-10 `areka-P0-anchor-tag-canon` の完了時の申し送り

- **ついでに直す 1 件（選択の送り口の記録が 2 行出る）**: 選択の送り先が消えているとき、送り口の `warn!`（`crates/areka/src/input_events/balloon.rs` の `BalloonWiring::send_selection`）と、呼び手の `error!`（普通のバルーンは `balloon_pressed.rs` の `on_balloon_pointer_pressed`、箱は `shell_box_handler.rs` の `send_selection`）が、同じ名前で 1 行ずつ出る（選択肢は `choice_selection_send_failed`、アンカーは `anchor_selection_send_failed`。名前の表は `balloon.rs` の `selection_events`）。完了 `areka-P0-choice-interact` からある軽い件で、`anchor-tag-canon` でアンカーにも写った。本 spec は `\__q` の範囲の当たりで同じ送り口と `balloon.rs`・`shell_box_handler.rs` を触るので、そのときに 1 回の失敗につき 1 行へ揃える（どちらを残すかは設計で決める）。動きは変えない。

## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）

- **前提の変化**: `anchor-tag-canon` が着地した（10-10）＝先に要るものは無くなり、今すぐ着手できる。上の棚卸㉓の「未着手」は古い。そのまま使えるもの:
  - 開きから閉じまでの字を 1 つの範囲にする記録（`crates/areka-emo-text/src/state_anchor.rs`）と、選択肢とアンカーが種類つきで並ぶ範囲の列（`state.rs` の `ChoiceSpan` の `kind`）。行ごとの割り当て・当たりの行・ホバー・押下・選択の送り口（`crates/areka/src/input_events/balloon.rs` の `ChoiceSelection`）は、もう種類を見て動く＝種類が選択肢の範囲を作れば、押下から `OnChoiceSelect` までは今の道を通る見込み。
  - 開きと閉じの対応の判定（`crates/areka-sakura/src/anchor_pair.rs` の `pair_anchors`。台本の変換と `check_script` が共有）。閉じ忘れ・重なり・迷子の閉じの扱いをアンカーと揃えるかは本 spec で決める（`doc/anchor-compat.md` の 3.1）。
- **触るファイル**（棚卸㉓の一覧を引き直した）:
  - `crates/areka-parsers/src/sakura/{decode.rs, model.rs, mod.rs}`・検査 `parse_bare_tag_tests.rs` と新しい検査ファイル。
  - `crates/areka-sakura/src/{compile.rs, anchor_pair.rs か隣の新しいファイル, lib.rs}`。`drive.rs` は触らない見込み（選択肢の ID の一覧は dola の再生機から読むだけ）。
  - **dola（棚卸㉓の一覧に無かった）**: `crates/dola/src/cue/{command.rs, sink.rs, runtime.rs}`。選択待ちの照合の一覧（`runtime.rs` の `pending_choices`）は今 `Choice` の合図だけを積むので、範囲の選択肢の ID も積まないと選んでも解けない。合図の種類を足すなら、全種類を並べている所（`crates/areka-ghost/src/sink.rs` の名前の表・emo-text の `actor.rs`）にも腕が要る。
  - `crates/areka-emo-text/src/{state.rs, state_anchor.rs, actor.rs, lookahead.rs}` と新しい検査ファイル＝**emo-text にファイルを足す**（`lib.rs` の純粋な一覧は今 76 本・ウェーブに 1 席）。`choice.rs`・`actor_present.rs` は種類つきになったので触らずに済む見込み。
  - `crates/areka/src/mcp/check_script_judge.rs`（開きと閉じの崩れの知らせ。アンカーと同じ形にするなら。**棚卸㉓の一覧に無かった**）と診断の表 `doc/ssp-mcp/areka-tools.md`。
  - `crates/areka/src/input_events/{balloon.rs, balloon_pressed.rs, shell_box_handler.rs}` は、上の申し送りの「記録が 2 行出る」直しだけ。kanade と `crates/areka/src/emo2_boot/` は触らない見込み。
- **規模**: M（10〜14 タスク）のまま（当たりと押下の分が減り、dola と `check_script` の分が増えた）。**優先度の区分**: A のまま。**要件定義のモデル**: Fable のまま。
- **先に要るもの**: 無い＝文字とバルーンの列と、台本の読み手の列の両方の先頭。働きで後ろに待つのは `link-context-copy`・`balloon-link-hover` の 2 本、列の順で待つのは約 13 本。
- **見つけた穴・古くなった記述**:
  - dola は crates.io に公開している。合図の語彙を足すと公開 API の追加になる＝roadmap の「次のリリースへの申し送り」へ 1 項目足す（`anchor-tag-canon` と同じ扱い）。
  - 「選択肢の範囲があるか」の観測（`actor.rs` の `TextLayerRuntime::choice_active`）は、バルーンの時間切れを止める判定が読む。範囲の選択肢でも立つようにする。`balloon-text-bugs` の件 1 が同じ観測のまわりを直すので、同じウェーブに置かない。
  - 同じウェーブに置けない相手: `font-size-keywords`（`check_script_judge.rs`）・`choice-ranges-one-function`・`balloon-font-file`（`actor.rs`）・`talk-fast-forward`（`decode.rs`・`compile.rs`・dola の `runtime.rs`）。`choice-marker-styling` とは、あちらが新しいファイルを足さず、こちらが `choice.rs` に触らなければ重ならない。

## `balloon-text-bugs` からの申し送り（2026-10-16 着地）

- バルーンの時間切れの判定は、もう `TextLayerRuntime::choice_active` を読まない。選択肢つきのトークは、可視性の線の合図 `ChoiceShown` で覚え、トークの終わりの時刻から測り直す（`crates/areka/src/emo2_boot/balloon_visibility_wait.rs` の `effective_display_end`）。範囲の選択肢でも時間切れを正しく扱うには、`crates/areka/src/emo2_boot/talk_lifecycle.rs` の `BalloonLifecycleSink` が範囲の選択肢の cue でも `ChoiceShown` を 1 回送ればよい。
- `choice_active` の意味は「済んでいない選択肢があるか」に変わった（押した場所の選択肢は済み・選択肢の時間切れと中断ではトークの終わりに全部済む）。範囲の選択肢も済みの境目（`ActorTextState::settled`）に乗る形で作る。押下の選択は dispatcher の受け口を通って kanade へ届く（`DispatcherMsg::Choice`／`Anchor`）。

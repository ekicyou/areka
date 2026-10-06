# Brief: areka-P0-anchor-tag-canon

> 起票: 2026-08-27（bvc 要件ディスカッション議題 5 の開発者指示による `/kiro-discovery` 再入・文字装飾系 3 spec 分割の 2 本目）
> **アンカー（`\_a`・本文中のリンク）機能そのものと、その装飾 16 項目を一括所有する。** `\f` のアンカー系装飾は「アンカー機能ごと不在」のため装飾核 spec から分離した。

## Problem

さくらスクリプトのアンカー `\_a[ID]...\_a`（本文中のクリック可能リンク）が areka に存在しない。`\_a` は `\f` 汎用パススルーと同じ経路で黙って捨てられ、アンカー装飾 16 項目（`anchorstyle`・3 状態の色/フォント/メソッド）は規定する対象が無い。

**⚠実バグを 1 件同梱**: 閉じの素の `\_a` は `lexer.rs:172-177` が `'_'` だけを消費して `Raw("\\_")` を作るため、**後続の `a` が可視のバルーン本文へ漏れる**（2026-08-27 実測・M2 ゲートと独立の生きた欠陥——emo2 は `\_a` を使わないため M1 適合には無害だが、`\_a` を含むゴーストを読み込むと即再現する）。

## Current State

- `\_a[ID]`（開き）・`\_a[ID,引数]`（拡張）・`\_a`（閉じ）の 3 形とも未実装（パススルー→破棄）。`OnAnchorSelect`／`OnAnchorSelectEx` イベントの発火も無い。
- アンカー装飾 16 項目（ukadoc・snapshot 実測）: `anchorstyle`（square|underline|square+underline|none）・`anchorcolor`/`anchorbrushcolor`・`anchorfontcolor`・`anchorpencolor`・`anchormethod`（Win32 `SetROP2` 演算子名）・`anchor.font.color`・`anchornotselect*` ×5・`anchorvisited*` ×5——選択中/非選択/訪問済みの 3 状態。
- descript 側の `anchor.font.*`／`anchor.notselect.font.*`／`anchor.visited.font.*` 族も未解析。`anchor.font.color.r` は現行テストで**拾ってはいけない distractor** として使われている（`parse.rs:157`・`parse_tests.rs:157-162`）＝解析拡張時にこのテストの意図を書き換える。
- **アンカー下線の縦書き写像は bvc が語彙確定済み**——縦書きでは列の**右側**に引かれる（bvc R5.3）。
- 選択肢（`\q`）のクリック機構・hover 描画（`viewbox_draw.rs:346-354` の `SetDrawingEffect`）は既存＝アンカーのクリック/hover の先例。

## Desired Outcome

`\_a` の 3 形が本文中のクリック可能範囲として機能し、`OnAnchorSelect(Ex)` が発火し、装飾 16 項目＋descript `anchor.*.font.*` 族が 3 状態で効き、下線は縦書きで列の右側に出る。素の `\_a` の文字漏れバグが解消している。

## Approach

`text-decoration-canon` の per-run 属性基盤（run 分割・`DWRITE_TEXT_RANGE` 適用）の上に、アンカー範囲＝属性付き run ＋当たり判定（選択肢クリックの機構を先例に）を載せる。lexer の `\_` 2 文字タグ解読の是正（バグ修正）は独立スライスで先行可能。

## Scope

- **In**: `\_a` 3 形の解読・アンカー範囲の保持・クリック/hover・`OnAnchorSelect`/`OnAnchorSelectEx`・装飾 16 項目・descript `anchor(.notselect|.visited).font.*` 族の解析・訪問済み状態の管理・縦書き下線位置（bvc 語彙継承）・**素の `\_a` 文字漏れバグの修正**・決定論テスト。
- **Out**: 装飾の per-run 基盤（`text-decoration-canon`）・選択肢 `\q`/`\__q` の機構（完了済み・不変）・URL 起動等のアンカー既定動作のうち OS 連携部の可否（要件段階で裁定）。

## Boundary Candidates

- lexer バグ修正（`\_` 2 文字タグの正しい消費）は独立・最小・先行可能——**M1 中の前倒し単独修正も選択肢**（開発者裁定次第）。
- アンカー範囲＋イベント（機能）と装飾 16 項目（見た目）の 2 相。

## Out of Boundary

- `\_l` 等ほかの `\_` 系タグの意味論（`cursor-tag-canon` ほか各所有者）——ただし lexer の `\_` 消費規則の是正はタグ名の正しい切り出しとして全 `\_` 系に効く（挙動はパススルー先で不変を保証）。

## Upstream / Downstream

- **Upstream**: `text-decoration-canon`（per-run 基盤・必須先行）・choice-select-events／choice-render（クリック/hover の先例・完了済み）・bvc（下線の縦書き写像）。
- **Downstream**: アンカーを使うゴースト資産の互換（メニュー的トークの主要手段）。

## Existing Spec Touchpoints

- **Extends**: なし（新機能）。
- **Adjacent**: `choice-marker-styling`（同じ 3 spec 群・別レンダラ経路）・`areka-parsers` の distractor テスト（意図の書き換えを伴う）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（`text-decoration-canon` の後段）。lexer バグ修正のみ開発者裁定で M1 前倒し可（just-in-time 起票）。
- 決定論テスト必達（3 状態 × 縦横・イベント発火・文字漏れの回帰檻）。

---

> **📌 2026-09-02 棚卸⑫（lexer バグ確定・⓪ 前倒し候補）**——**バグは現行 main に実在**（逐語証跡）: `lexer.rs:152-157` のループは `word="_a"` を正しく作るが、角括弧が無い場合の else 腕（**:172-177**・`let bare = first; (Token::Bare(bare), word_start + 1)`）が `word` を捨てて `'_'` 1 文字ぶんしか進めない。`\_a[id]text\_a` → `Tag{"_a",["id"]}` は `decode_tag`（`decode.rs:191-221`）に `"_a"` 腕なし→`Raw`→`compile.rs:203` で破棄／末尾 `\_a` → `Bare('_')`→`decode_passthrough_bare`（:331-333）→破棄・**残った `a` が `Token::Text("a")` として本文へ**＝「text**a**」と表示。**同じ欠陥は `\_q`・`\_n`・`\_b`・`\_v` 等、全 2 文字 `\_` 系 bare 形に一律**。既存テスト被覆 **0 本**（`\_w[450]`・`\_l[10,20]` の角括弧付きのみ）。
> **2026-09-02 追記: 開発者「spec が無いと開始できない」により S spec `areka-P0-sakura-bare-tag-lexer` として起票（ロードマップ ⓪ 行）。下記の「直接修正」はその spec が行う。** 推奨＝spec を立てず直接修正 1 PR（Path B・⓪）: lexer の bare 腕で `word` 全体を消費（`Token::Tag{word, args: []}` or bare 2 文字型）＋decode の passthrough を `\_X` 全体の `Raw` へ＋決定論檻（bare 形 5 種・角括弧形・`\_` 単独末尾）。**W12 の channels と W13 の decoration が同じ `decode.rs` を触るため先に着地させる**。着地後は本 brief に「lexer 修正は消化済み（PR#）」を登記し、本 spec は L→M。
> アンカー再測定: `parse.rs:157` の distractor 言及は **:164**（`.with_cursor` が :157）・`parse_tests.rs` データ行 :164・`viewbox_draw.rs:346-354` は **reset の腕**（hover 適用は :388）。前提: decoration 未着手（必須先行）・choice 系 ✅・bvc ✅。W14 で choice-marker と `decode.rs`／`viewbox_draw.rs` を共有し得る＝design で所有分割。
> **2026-09-03 追記（消化済み①・areka-P0-sakura-bare-tag-lexer・PR #134）**: 上記「直接修正」＝角括弧なし `\_` タグの消費是正は spec `areka-P0-sakura-bare-tag-lexer` で消化済み（規律＝`\_` ＋ `_` 0〜1 個 ＋ 1 文字の固定長・`\__X` 3 文字形も射程・意味付けなし＝`Instruction::Raw` 素通し・決定論テスト新設）。本 spec に lexer 修正は残らず規模は L→M。

> **📌 2026-09-13 相互登記（`areka-P0-text-decoration-canon` 着地）**——`anchor*` 系と `anchor.font.color` は親 spec が `ActorTextState::unowned_vocab()`（`crates/areka-emo-text/src/state_decoration.rs`）に保持するだけで表示を変えない。`\f[color,default.anchor*]` は当面すべて既定色へ解決して `look.rs::Note::AnchorColorAsDefault` を返す腕（`look.rs::apply_color`）を通るので、本 spec はその腕を 3 状態の色へ差し替えればよい（下線の描画基盤は `viewbox_draw_decoration.rs::apply_font_ranges` に着地済み）。

## `balloon-color-emoji` からの申し送り（2026-09-30 着地）

- バルーンの文字の単位は `char` から書記素クラスタ（人が 1 文字と見る単位）に替わった。範囲（`ChoiceSpan::glyph_range`・`style_runs`・`segment_text_range`）はクラスタの通し番号で数え、UTF-16 の位置はクラスタ文字列の長さを積む。
- `TextItem::Glyph` と `PositionedGlyph` の中身は `text: Arc<str>`（`Copy` なし）。構築は `TextItem::glyph(&str)`、切り方は `areka_sakura::cluster::clusters` だけが決める。文字を比べる処理（行末のぶら下げの判定など）は `&str` で比べる。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **優先度 高**（`\_a` のリンクは文字として出るが、押しても何も起きずイベントも飛ばない）。
- **そのままでは 20 タスクを超える**＝要件の段で「働き（範囲・クリックとホバー・`OnAnchorSelect`／`OnAnchorSelectEx`）」と「装飾 16 × 状態 3」に切り、**働きを先に**。働きの側は `text-align-shadow-canon`・`choice-marker-styling` を待たなくてよい（依存は同じファイルを触るだけ）。
- 変わっていない点: `sakura/decode.rs` に `_a` の腕は無い・`OnAnchorSelect` は 0 件・字句の直しは済み。`viewbox_draw` の行番号は古い。
- **触るファイル**: `crates/areka-parsers/src/sakura/decode.rs`・`crates/areka-sakura/src/compile.rs`・`crates/areka-emo-text/src/{actor 系, viewbox_draw 系}`・`crates/areka/src/input_events/` のバルーン・kanade の `schedule/events.rs`。文字まわりの直列の列に並ぶ（`emo-text-file-split` → `shell-balloon` の後）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: 一括なら 22〜28 タスク＝**切る**。
  - **① 働き（`anchor-tag-canon` の名前のまま）・M（12〜15）**: `\_a` の全形の解読（`\_a[ID]`・`\_a[ID,r2,…]`・**`\_a[OnID,r0,…]`**＝ID が `On` で始まればその名前のイベントを直接送る形〔ukadoc `\_a[OnID,r0,r1...]`・本 brief の「3 形」に抜けている 4 つ目〕・閉じの `\_a`）→ compile → 文字の層でアンカーの範囲を持つ → 当たり判定の行（選択肢の `ChoiceHitRow` と同じ形）→ ホバーとクリック（普通のバルーンと箱）→ `OnAnchorSelectEx`→`OnAnchorSelect` の順の送出（Reference は ukadoc どおり）。見た目は**既定の 1 種類だけ**（非選択は既定の色、ホバーは選択肢と同じ強調）で、作者の指定は効かなくてよい。
  - **② 装飾（新しい spec を起こす。名前の案 `anchor-style-canon`）・M（10〜13）**: `\f[anchor*]` 16 項目（`anchorstyle`・色 4 系・`anchormethod`〔`SetROP2` の名前〕× 選択中／非選択／訪問済みの 3 状態）・`\f[anchor.font.color]`・descript の `anchor.font.*`／`anchor.notselect.font.*`／`anchor.visited.font.*` の読み取り（`balloon/parse.rs` の完全一致の引きの注記と `parse_tests.rs` の「拾ってはいけない」例 `anchor.font.color.r` の意図の書き換え）・訪問済みの記録・縦書きで下線を列の右へ。
  - 順序: ①を先に（列の 2 番目）、②は列の後ろ（`text-align-shadow-canon` の後）。
  - **今 2 本の brief に切るべきか**: 切るべき。roadmap の直列の列は既に①と②を列の 2 番目と 12 番目に置いており、あいだに 9 本が入る。1 spec＝1 ブランチ＝1 PR なので 1 本のままでは列の形どおりに進められない。要件の段で切ると、②の requirements が①の着手から 9 本ぶん寝かされて陳腐化する。今のうちに②の brief を起こし、①の brief の Scope から装飾を外すのがよい。
  - ②は `choice-marker-styling`（`\f[cursor*]` 10 項目）と形がほぼ同じ（形状 4 種・ブラシ／ペン／文字の色・`SetROP2` の描画方法・非選択の 5 項目）。②と `choice-marker-styling` を列で隣に並べ、先に着地する方が「descript × 実行時の 2 層で印の見た目を解く型」と `SetROP2` の名前の受け取りを作り、後の方が使う形を推す（1 本へ合わせると 15〜20 で上限の直前になるので合わせない）。
- 前提の状態: 字句の直し（`sakura-bare-tag-lexer`・PR#134）・`text-decoration-canon`（下線の描画の基盤）・`choice-timeout-directive`（PR#215・`compile.rs`）・`emo-text-file-split`・`shell-balloon` はすべて着地済み。①の前提は満たす。②は `text-align-shadow-canon` の後（同じ `look.rs`・`viewbox_draw` 系）。
- 崩れた前提／古くなった位置:
  - 分割でホバーとクリックは `input_events/balloon_moved.rs`（`on_balloon_pointer_moved`）と `balloon_pressed.rs`（`on_balloon_pointer_pressed`）へ、行の描画と強調の矩形は `viewbox_draw_render.rs`（`render_styled`・`ChoiceDraw`・`highlight_rect`）へ、1 コマの流れは `actor_present.rs`（`present_actor`）へ移った。brief の `viewbox_draw.rs:346-354` は無効。
  - **箱（シェル内バルーン）のクリックは別の道**: `input_events/shell_box_handler.rs`（ホバーと押下の前段）と `shell_box.rs` の純関数（`judge_box_click`・`BoxPressVerdict`＝今は「選択で使った／シェルの操作」の 2 値）。アンカーを箱でも押せるようにするには、ここに「アンカーで使った」を足す必要がある。箱は `ResolvedBalloonText` と描画を普通のバルーンと共有しているので、範囲と見た目は自動で両方に効く。
  - kanade 側の送出は選択肢と同じ並び: `areka-kanade/src/msg.rs`・`schedule/events.rs`・`schedule/choice.rs`（`translate-pipeline` で `schedule/steady.rs` は分割済み）。選択の受け口は `input_events/choice_drain.rs`（`ChoiceSelectionInbox`）。
  - 文字の装飾の受け口は `look.rs` の `Note::AnchorColorAsDefault` の腕と、所有外のキーの判定（`look.rs` の `is_unowned`：`starts_with("anchor")`）。これは②の仕事。
  - dola の `CueCommand` に種類を足すと網羅の match が連鎖する（`emo-text` の `actor.rs`／`state.rs`・`areka-ghost` の `sink.rs`／`prop_sink.rs`・`areka-seriko` の `actor.rs`・`areka-sakura` の `contract.rs`・`dola` の `sink.rs`、ほか `emo2_boot` の `*_cue.rs` 6 本は `Custom` を見るだけ）。`Choice` と同じく専用の種類にするか、既存の種類に乗せるかで①の規模が 2〜3 タスク動く。
- 触るファイル（並走の照合用・①働き）:
  - `crates/areka-parsers/src/sakura/{decode.rs, model.rs}`（`"_a"` の腕・`Instruction`）
  - `crates/areka-sakura/src/compile.rs`（＋`drive.rs` の可能性）
  - `crates/dola/src/cue/command.rs`（種類を足すなら）と網羅の match の各所（上記）
  - `crates/areka-emo-text/src/{state.rs, actor.rs, actor_present.rs, viewbox_draw_render.rs, choice.rs}`（範囲・当たり判定の行・既定の見た目）、範囲を layout が運ぶなら `layout.rs` 系
  - `crates/areka/src/input_events/{balloon.rs, balloon_moved.rs, balloon_pressed.rs, choice_drain.rs, shell_box.rs, shell_box_handler.rs}`
  - `crates/areka-kanade/src/{msg.rs, schedule/events.rs}`（＋`schedule/choice.rs` に倣う新しいファイル）
  - `doc/ukadoc-coverage/ledger/{sakura-script,events}.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 触るファイル（②装飾）: `crates/areka-emo-text/src/{look.rs, state_decoration.rs, viewbox_draw_render.rs, viewbox_draw_decoration.rs, balloon_overrides.rs}`・`crates/areka-parsers/src/balloon/{model.rs, parse.rs, parse_tests.rs}`
- 議題（答えで作業が変わるものだけ）:
  1. アンカーを dola の専用の種類にするか（`Choice` と同じ）、既存の種類に乗せるか。
  2. ①の既定の見た目（作者の指定が無いときの非選択・ホバーの姿）。①で何も描かないと「押せるのに見えない」になる。
  3. 話している最中のアンカーのクリックの扱い（ukadoc は「選択肢タイムアウトしないが、通常トーク同様バルーンタイムアウトする」と書くだけ）。`talk-fast-forward` の早送りのクリックと同じ箱の押下を取り合う＝どちらが先に着地しても、後の方は `shell_box.rs` の判定の順を決め直す。
- 見つけた穴: バグの候補は無い。ただし brief の Scope が `\_a[OnID,…]` の形を数えていない（作業の抜け）。


## 2026-10-04 棚卸㉑で切った後の範囲

- **本 spec は働きだけを持つ**（M・12〜15 タスク）。装飾（`\f[anchor*]` 16 項目×3 状態・`\f[anchor.font.color]`・descript の `anchor(.notselect|.visited).font.*` 族・訪問済み・縦書きの下線）は新しい spec **`areka-P0-anchor-style-canon`** へ移した（開発者「負荷が高すぎる仕様は分割を検討せよ」）。
- **見た目は既定の 1 種類だけ**: 作者の指定が無いときの姿（非選択の姿とホバーの姿）を 1 つ決めて描く。作者の指定（`\f[anchor*]`・descript）は本 spec では効かなくてよい（受け取りは今までどおり `unowned_vocab()` に保持）。何も描かないと「押せるのに見えない」になるので、既定の姿は要件で決める。
- **Scope の読み替え**:
  - In から外す: 「装飾 16 項目」「descript `anchor(.notselect|.visited).font.*` 族の解析」「訪問済み状態の管理」「縦書き下線位置（bvc 語彙継承）」→ すべて `anchor-style-canon`。
  - In に残す: `\_a` の全形の解読・アンカーの範囲の保持・クリックとホバー・`OnAnchorSelect`／`OnAnchorSelectEx`・既定の見た目 1 種類・決定論テスト。
  - In に足す: ⑴ **4 つ目の形 `\_a[OnID,r0,r1,…]`**（ID が `On` で始まれば、クリックでその名前のイベントを直接送り、続く引数を Reference0 以降に入れる・ukadoc の `\_a[OnID,r0,r1...]` の項）。本 brief の「3 形」はこれを数えていない。⑵ **シェル内バルーンの箱の押下**: 箱の押下は `crates/areka/src/input_events/shell_box_handler.rs` → `shell_box.rs` の純関数 `judge_box_click` を通り、結論 `BoxPressVerdict` は今「選択で使った／シェルの操作」の 2 値しかない。ここに「アンカーで使った」を足す（ホバーも同じ前段）。範囲と見た目は箱も普通のバルーンと同じ道（`ResolvedBalloonText`・描画）を通るので自動で効く。
  - Desired Outcome の「装飾 16 項目＋descript `anchor.*.font.*` 族が 3 状態で効き、下線は縦書きで列の右側に出る」は `anchor-style-canon` の到達点へ読み替える。
- Boundary Candidates の「アンカー範囲＋イベント（機能）と装飾 16 項目（見た目）の 2 相」は、この切り出しで消化した。
- 箱の押下の判定は `talk-fast-forward`（話している最中の早送り）・`balloon-markers`（矢印のクリック）も同じ `judge_box_click` に結論を足しにくる。先に着地した方が判定の順（選択 → アンカー → 早送り → シェル の順など）を決め、後の方がそこへ足す。

## 2026-10-05 `/kiro-discovery`「バルーンのリンクと OS の連携」で直したこと

- **誤りの訂正**: 上の Scope の Out にある「選択肢 `\q`/`\__q` の機構（完了済み・不変）」のうち、`\__q` は**未実装**だった（`decode.rs` の素通しの腕で `Raw` になり `compile.rs` が捨てる・網羅台帳 `sakura-script.toml` の `\__q` の行は「無い」・引受先なし）。`\__q` は新しい spec `range-choice-tag` が引き受け、本 spec の後に作る（開発者裁定・議題 4＝本 spec に足さない）。
- **後に乗る spec**: 本 spec が作る「範囲を押せるようにする仕組み」（範囲の当たりの行の集合）の上に、次の 3 本が乗る。
  - `range-choice-tag`（同じ仕組みで範囲を選択肢にする）。
  - `link-context-copy`（右クリックでアンカーの行き先をコピー）。
  - `balloon-link-hover`（`OnAnchorHover` と、アンカーの上に止まったときの行き先の説明）。
  範囲の当たりは、選択肢の範囲にも使い回せる形で作ると後が楽になる（設計の段で検討）。
- **OS 連携部の可否**（Out の 3 つ目）: アンカーから URL などを開くのは、ゴーストが `OnAnchorSelect(Ex)` の応答で `\j[…]` などを返す正典の作法で足りる。`\j` と `\![open,…]` は `open-external-tags` が作る。本 spec はアンカーの既定の動作で OS を呼ばない。

---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（12〜15 タスク）。切らない（装飾は `anchor-style-canon` へ切り出し済み）。
- 前提の状態: 満たす。C3 は本 spec の触るファイルのうち `actor.rs`（注記）・`actor_box.rs`（`shell-balloon-frame-align`）、`input_events/mod.rs`（ドラッグの受け手）・kanade の `msg.rs`・`schedule/{events.rs, steady.rs の on_mouse}`・`lib.rs` の `pub use`（`mouse-drag-events`）を書き換えた。中身はどれもアンカーの道と独立。
- 崩れた前提／古くなった位置:
  - **㉑の「`judge_box_click`・`BoxPressVerdict` は今 2 値」は誤り**（C3 の前からこの形）。`BoxPressVerdict`（`input_events/shell_box.rs` の定義）は「シェルの操作／選択で使った／中断を禁じる区間／中断」の 4 値で、順を決めるのは同じファイルの `judge_box_press`（⑴この押下が選択 ⑵左ダブルクリックでない→シェル ⑶直前の押下が選択 ⑷話していない→シェル ⑸中断を禁じる区間 ⑹中断）。呼び手は `input_events/user_break.rs` の `on_box_press`。`judge_box_click` は「選択肢の確定か」を返すだけ（呼び手は `shell_box_handler.rs` の `on_box_pointer_pressed`）。アンカーの結論は ⑴ の隣に足す＝**`user_break.rs` も触る**。今、箱の上の単クリックは ⑵ で「シェルの操作」になる。
  - kanade: `msg.rs` は 909 行（`mouse-drag-events` が `MouseEventKind` に 2 つ足した）＝新しい入力の型は新しいファイルへ。選択の受け口は `schedule/mod.rs` の `Input::Choice` の腕と `steady.rs` の選択の帳簿で、`steady.rs` 947 行・`schedule/mod.rs` 938 行＝アンカーの受理は `schedule/choice.rs`（346 行）に倣う新しいファイルに置き、`mod.rs` には腕を 1 本足すだけにする。`events.rs` は 733 行。
  - `\_a[OnID,…]` の任意の名前は固定の表 `ALLOWED_EVENT_IDS` に載らない＝選択の `is_allowed_choice_event` と同じ「出所別の受理規則」が要る。
  - dola の `CueCommand` に種類を足すなら、連鎖の match の 1 つ `areka-seriko/src/actor.rs` は C3（`surface-element-nesting`）で部品の時計が入り 660 行になった。
  - 入口は変わらず: `input_events/balloon_moved.rs`・`balloon_pressed.rs` の冒頭に「足す予定の spec: anchor-tag-canon」の注記がある。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/sakura/{decode.rs, model.rs}`・`crates/areka-sakura/src/compile.rs`（＋`drive.rs` の見込み）
  - dola に種類を足すなら `crates/dola/src/cue/command.rs` と連鎖（emo-text `actor.rs`／`state.rs`・`areka-ghost/src/{sink.rs, prop_sink.rs}`・`areka-seriko/src/actor.rs`・`areka-sakura/src/contract.rs`・`dola/src/sink.rs`）
  - `crates/areka-emo-text/src/{state.rs, actor.rs, actor_present.rs, viewbox_draw_render.rs, choice.rs}`・`lib.rs`（新しいファイル）
  - `crates/areka/src/input_events/{balloon.rs, balloon_moved.rs, balloon_pressed.rs, choice_drain.rs, shell_box.rs, shell_box_handler.rs, user_break.rs}`
  - `crates/areka-kanade/src/{msg.rs, lib.rs, schedule/events.rs, schedule/mod.rs（腕 1 本）}`＋新しいファイル（`schedule/choice.rs` に倣う）
  - `doc/ukadoc-coverage/ledger/{sakura-script,shiori}.toml`（`\_a` と `OnAnchorSelect(Ex)` の行）・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: ㉑の 3 件のまま（dola の種類／既定の見た目／話している最中のクリック）。
- 見つけた穴: なし。軽微: ㉑の節と「棚卸㉑で切った後の範囲」の節にある「`judge_box_click` の結論・2 値」の書き方を上のとおり読み替える。
- 並走の判定（厳しめ）: `budoux-reveal-reflow`（`state.rs`・`actor_present.rs`）・`balloon-font-file`（`actor.rs`・`viewbox_draw_render.rs`）・`balloon-lifecycle-events`（kanade の `events.rs`・`schedule/mod.rs`・`lib.rs`）・`talk-fast-forward`・`balloon-markers`・`text-typesetting` とは重なる＝並べない。`balloon-canon-residue` とは、dola に種類を足さなければ 0（足すと `areka-seriko/src/actor.rs` で重なる）。

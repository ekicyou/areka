# Brief: areka-P0-anchor-style-canon

> 2026-10-04 棚卸㉑で `areka-P0-anchor-tag-canon` から切り出した（開発者「負荷が高すぎる仕様は分割を検討せよ」）。元の spec はアンカーの**働き**（範囲・クリックとホバー・イベント）だけを持ち、本 spec はアンカーの**見た目**（装飾 16 項目×3 状態・descript の `anchor.*.font.*` 族・訪問済み・縦書きの下線）を持つ。正典の引用と経緯の正本は元の brief。

## Problem

- **ゴーストの作者**: アンカー（`\_a`・本文中のリンク）の見た目を変えられない。正典はスクリプトの `\f[anchor*]` で、選択中・非選択・訪問済みの 3 状態ごとに形状・色・描画方法を変えることを認めているが、areka は受け取って保持するだけで表示を変えない。
- バルーンの descript.txt の `anchor.font.*`／`anchor.notselect.font.*`／`anchor.visited.font.*` 族も読んでいない。
- 元の spec（働き）が着地すると、アンカーは既定の 1 種類の見た目だけで出る。作者が指定した見た目は本 spec の着地まで効かない。

## Current State

- 装飾 16 項目（ukadoc）: `anchorstyle`（`square`／`underline`／`square+underline`／`none`）・`anchorcolor`＝`anchorbrushcolor`・`anchorfontcolor`・`anchorpencolor`・`anchormethod`（Win32 `SetROP2` の名前・`default` でバルーン設定の標準へ戻る）が選択中、同じ 5 項目の `anchornotselect*` が非選択、`anchorvisited*` が訪問済み。ほかに `\f[anchor.font.color]`（アンカーの文字色）。
- 文字の層は `\f[anchor*]` を `ActorTextState::unowned_vocab()`（`crates/areka-emo-text/src/state_decoration.rs`）に保持するだけ。所有外のキーの判定は `crates/areka-emo-text/src/look.rs` の `is_unowned`（`starts_with("anchor")`）。`\f[color,default.anchor*]` は `look.rs` の `apply_color` で既定色に解かれ、`Note::AnchorColorAsDefault` を返す。
- 下線の描画の基盤は着地済み（`crates/areka-emo-text/src/viewbox_draw_decoration.rs` の `apply_font_ranges` が区間へ下線を渡す）。行の描画と強調の矩形は `viewbox_draw_render.rs`（`render_styled`・`ChoiceDraw`・`highlight_rect`）。
- descript の読み手（`crates/areka-parsers/src/balloon/parse.rs`）は `anchor.font.*` を持たない。`parse.rs` の完全一致の引きの注記と `parse_tests.rs` は `anchor.font.color.r` を「拾ってはいけない例」として使っている＝読み取りを足すときにこのテストの意図を書き換える。
- 縦書きでアンカーの下線は列の**右側**（bvc R5.3 の語彙が確定済み）。

## Desired Outcome

- `\f[anchor*]` 16 項目と `\f[anchor.font.color]` が、選択中・非選択・訪問済みの 3 状態で正典どおりに効く。`default` でバルーン定義の値へ戻る。
- descript の `anchor.font.*`／`anchor.notselect.font.*`／`anchor.visited.font.*` を読み、スクリプトの指定と 2 層（descript の上に実行時が後勝ち）で解く。
- 一度クリックしたアンカーは訪問済みの見た目になる。
- 下線は横書きで字の下、縦書きで列の右側に出る。
- 普通のバルーンとシェル内バルーンの箱の両方で同じ規則が効く。

## Approach

- 元の spec（働き）が置いたアンカーの範囲と、既定の 1 種類の見た目の上に、3 状態の見た目の解決を重ねる。
- 解決の形は選択肢の印（`choice-marker-styling`＝`\f[cursor*]` 10 項目）とほぼ同じ（形状 4 種・ブラシ／ペン／文字の色・`SetROP2` の描画方法・非選択の 5 項目）。先に着地した方が「descript × 実行時の 2 層で印の見た目を解く型」と `SetROP2` の名前の受け取りを作り、後の方が使う。
- 新しい項目は `look.rs` の `TextLook` の欄として足す。戻す操作（`state_decoration.rs` の `TextLayerState::reset_decoration`）は `TextLook` を丸ごと置き換えるので、列挙なしで戻る。箱の `font.follow`（スコープに付いて回る／箱だけ）の振り分けにも同じ理由で自動で乗る。

## Scope

- **In**: `\f[anchor*]` 16 項目×3 状態・`\f[anchor.font.color]`・`default` での復帰・descript の `anchor(.notselect|.visited).font.*` 族の読み取りと 2 層の解決・訪問済みの記録・縦書きの下線（列の右側）・`SetROP2` の名前の受け取りと未知の名前の縮退（記録つき）・普通のバルーンと箱・網羅台帳の更新・決定論テスト（3 状態 × 縦横 × descript の有無）。
- **Out**: アンカーの働き（`\_a` の解読・範囲・当たり判定・クリックとホバー・`OnAnchorSelect`／`OnAnchorSelectEx`・既定の見た目＝`anchor-tag-canon`）・選択肢の印の `\f[cursor*]`（`choice-marker-styling`）・装飾の基盤（`text-decoration-canon`・完了）。

## Boundary Candidates

- 解決の層（descript × 実行時の 2 層・3 状態の選び分け）と、描画（形状・下線・描画方法）の 2 相。

## Out of Boundary

- アンカーの範囲の持ち方とクリックの道（元の spec が決める）。本 spec は範囲と「どの状態か」を読むだけ。

## Upstream / Downstream

- **Upstream**: `anchor-tag-canon`（働き・範囲と既定の見た目・必須先行）・`text-decoration-canon`（完了・下線の基盤）・`text-align-shadow-canon`（同じ `look.rs`・`viewbox_draw_render.rs` を触る＝列で先）・bvc（完了・縦書きの下線の写像）。
- **Downstream**: アンカーを使うゴースト資産の見た目の互換。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-anchor-tag-canon`（切り出し元・装飾の部分を引き継ぐ）。
- **Adjacent**: `choice-marker-styling`（同じ形の 3 状態の印・列で隣に並べる）・`text-align-shadow-canon`（同じファイル）。

## Constraints

- 既定（作者の指定なし）では、元の spec が決めた既定の見た目と同じ結果になること。
- 1 ファイル 1,000 行（`look.rs` 771・`choice.rs` 744 行＝足す量によっては新しいファイルで足す）。決定論テスト網羅は必達。ログ無しの失敗の経路を作らない。

## 2026-10-04 棚卸㉑で切り出し

- 元の spec: `areka-P0-anchor-tag-canon`（一括だと 22〜28 タスクで上限 20 を超えるため、働きと装飾に切った）。
- 規模: M（10〜13 タスク）。
- 前提: `anchor-tag-canon`（働き）の着地。文字とバルーンの直列の列の上では `text-align-shadow-canon` の後（同じ `look.rs`・`viewbox_draw_render.rs`）。
- `choice-marker-styling` と隣に並べる理由: 形がほぼ同じ（形状 4 種・ブラシ／ペン／文字の色・`SetROP2` の描画方法・非選択の 5 項目）で、「descript × 実行時の 2 層の解決」と「`SetROP2` の名前の扱い」を共用できる。離して並べると同じ仕組みを 2 度作るか、後の方が古い設計を読み直すことになる。1 本へ合わせると 15〜20 で上限の直前になるので合わせない。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{look.rs, state_decoration.rs, viewbox_draw_render.rs, viewbox_draw_decoration.rs, balloon_overrides.rs}`（＋足す量によっては新規と `lib.rs`）
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs, parse_tests.rs}`
  - 訪問済みの記録の置き場所（元の spec が作るアンカーの状態の隣・`crates/areka-emo-text/src/state.rs` か `actor.rs` の見込み）
  - `doc/ukadoc-coverage/ledger/{sakura-script,assets}.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）:
  1. `SetROP2` の描画方法（`anchormethod`）を Direct2D でどこまで再現するか（D2D に ROP2 は無い。`copypen` 以外を合成モードへ写すか、既定へ縮退して記録するか）。`choice-marker-styling` と同じ答えにする。
  2. 訪問済みをいつまで覚えるか（その台詞の間・バルーンが閉じるまで・ゴーストが起きている間）。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（10〜13 タスク）のまま。切らない（切り出したばかり）。
- 前提の状態: `anchor-tag-canon`（働き・範囲と既定の見た目）は今も未着手（C4 の予定）＝未。これが着地するまで着手できない。列の上では `choice-marker-styling` の隣。
- 崩れた前提／古くなった位置:
  - 起票時の位置は全部当たる: `look.rs` の `is_unowned`（`starts_with("anchor")`）と `apply_color` の `Note::AnchorColorAsDefault`、`state_decoration.rs` の `unowned_vocab`・`reset_decoration`、`viewbox_draw_decoration.rs` の `apply_font_ranges`、`viewbox_draw_render.rs` の `render_styled`・`ChoiceDraw`・`highlight_rect`。C3 での変化は `look.rs` の注記 1 行（「M2 予約」→「予約」）だけ。
  - descript の読み手: `areka-parsers/src/balloon/parse.rs` の完全一致の引きの注記と `parse_tests.rs` の「拾ってはいけない例」（`anchor.font.color.r`）は今もある。同じテストは `anchor.font.shadowcolor.r` も別の箇所で使っている＝影のキーの読みを足す `text-align-shadow-canon` と同じテストファイルを触る（列で後なので衝突はしない）。
  - 行数: `look.rs` 771・`choice.rs` 744・`balloon/model.rs` 774・`balloon/parse.rs` 252 行。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{look.rs, state_decoration.rs, viewbox_draw_render.rs, viewbox_draw_decoration.rs, balloon_overrides.rs}`（＋足す量によっては新規と `lib.rs`）・訪問済みの置き場所（`anchor-tag-canon` が作るアンカーの状態の隣）
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs, parse_tests.rs}`・`doc/ukadoc-coverage/ledger/{sakura-script,assets}.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題: 起票時のまま 2 件（`anchormethod` の D2D での再現の範囲＝`choice-marker-styling` と同じ答え／訪問済みをいつまで覚えるか）。
- 見つけた穴: なし。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `anchor-tag-canon`（働き）は未着手のまま＝本 spec はまだ着手できない。
  - C4 で `crates/areka-emo-text/src/look.rs` は 802 行（`mcp-author-tools` が、効かなかった `\f` の理由の分類を足した）、`state_decoration.rs` は 635 行（`budoux-reveal-reflow`）になった。起票のときの位置は全部当たる（所有外のキーの判定 `is_unowned`・`Note::AnchorColorAsDefault`・`unowned_vocab`・`reset_decoration`・`apply_font_ranges`・`render_styled`）。
  - 台本を再生せずに確かめる道具 `check_script`（`crates/areka/src/mcp/check_script_judge.rs` の `judge_font`）は、`\f[anchor*]` を今「効かない」と診断する。判定は `look.rs` の結果をそのまま使うので、本 spec が効くようにすれば診断は自動で消える。
- **触るファイル**: 棚卸㉒の一覧のまま。行数の注意:
  - `crates/areka-parsers/src/balloon/parse_tests.rs` は 922 行＝読み取りの検査は新しいファイルへ。`balloon/model.rs` は 774 行。
  - `look.rs` は 802 行＝3 状態の解決は新しいファイルへ置く＝emo-text の `lib.rs` の一覧の席を使う。
- **規模**: M（10〜13 タスク）。下の穴（descript の形・色・描き方の行）を入れると 12〜16。
- **先に要るもの**: `anchor-tag-canon`（必須）。同じファイルを触る `text-align-shadow-canon` の後。
- **優先度の区分**: C（ukadoc の `\f[anchor*]` と descript の `anchor.*` の拾い残し）。
- **要件定義のモデル**: Fable（`anchormethod` を Direct2D でどこまで再現するか・訪問済みをいつまで覚えるか＝開発者に聞く分かれ目と、ukadoc の記述が薄い所）。
- **分割の案**: 切らない（切り出したばかり）。
- **見つけた穴・古くなった記述**:
  - 網羅台帳の持ち主が切り出しの前のまま: `assets.toml` の `anchor.*` 43 行と `sakura-script.toml` の `\f[anchor*]` 16 行の持ち主が `anchor-tag-canon` で、本 spec の持ち物は 0 行。付け替える。
  - Scope は descript を `anchor(.notselect|.visited).font.*` 族としか書いていないが、網羅台帳の 43 行には 3 状態ぶんの `anchor.style`・`anchor.brush.color.*`・`anchor.pen.color.*`・`anchor.blendmethod` も入っている（バルーン定義の読み手はどれも読まない）。`\f[anchorstyle,default]` などの戻り先なので範囲に入れる。影の行は `text-align-shadow-canon` と分け方を決める。
  - roadmap の列の「`choice-marker-styling` ∥ 隣に `anchor-style-canon`」は並走の意味ではない。2 本は `look.rs`・`state_decoration.rs`・`viewbox_draw_render.rs`・`viewbox_draw_decoration.rs` が重なる＝続けて走らせる（同じウェーブには置けない）。

## 2026-10-10 `areka-P0-anchor-tag-canon` の完了時の申し送り

出どころは `completed/areka-P0-anchor-tag-canon/tasks.md` の Implementation Notes（3.3・6.2・6.3・7.2）。前提の `anchor-tag-canon`（働き）は実装と実機の確かめを終えた。上の「2026-10-10 棚卸㉓の再測定」の「未着手のまま」と、台帳の持ち主の付け替えの行は古くなった（`\f[anchor*]` 16 行と descript の `anchor.*` 43 行の持ち主は、`anchor-tag-canon` のタスク 6.3 が本 spec へ付け替えた＝本 spec の持ち物は 59 行）。

1. **見た目の装着より先に届いたアンカーの字**（実機では出ていない・潜んでいる性質）: 見た目の装着（`crates/areka-emo-text/src/state_decoration.rs` の `set_look_layers`）より先にアンカーの中の文字が届くと、その字は「装着前の既定＋下線」の丸ごとの写しとして装飾の表に載る（`look.rs` の `StyleTable::intern`）。`set_look_layers` はその字を載せ直さないので、装着の後もその字だけ装着前の既定の大きさ・色・フォントで描かれうる。
   - 作者が `\f` で飾った字には前からある性質（テスト `attaching_after_an_explicit_look_still_lands_the_balloon_defaults` が再現する窓）。`anchor-tag-canon` の下線で、`\f` を 1 つも書かない台本でも起こりうるようになった。
   - 2026-10-10 の実機では、起動直後の台詞のアンカーの字の大きさと色はほかの字と同じだった（崩れは 0 件）。
   - 下線を丸ごとの写しでなく「既定との差分」として持てば、この窓は消える。3 状態の見た目の解決を作るときに一緒に決める。
2. **差し替える 1 か所**: `anchor-tag-canon` は、アンカーが開いている間「見た目の写しに下線を立てたもの」を `state_decoration.rs` の `push_current_style` で焼いている。本 spec の `anchor.style` の解決は、ここを置き換える。作者が範囲の中で下線を切ってもアンカーの下線が勝つ決め（`doc/anchor-compat.md` の 2.1 の 9）も、ここで見直す。
3. **台帳の備考の書き直し**: `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\f[anchor*]` 16 行の備考は「ログ: 出る（compile の catch-all の debug! 記録）」と書くが、実態と合わない（`anchor-tag-canon` より前から。16 行とも）。本 spec が行の状態を変えるときに書き直す。
4. **隣の spec と同じ所を触る**: `areka-P0-underline-bottom-row-clip`（2026-10-10 起票・バグ）＝文字の領域の下端に来た行の下線が出ない件。下線の位置を DirectWrite に任せている所（`viewbox_draw_decoration.rs` の `apply_font_ranges`）と文字の面の大きさを触るので、本 spec の「縦書きの下線の位置の決め直し」と重なる。同時に走らせない。先に着地した方の決めに、後の方が合わせる。
5. **互換の記録**: アンカーの裁定の正本は `doc/anchor-compat.md`。本 spec が差し替える行は 2.1 の 8・9、本 spec へ波及する事柄は §3（3.1 の表の 1 行目と「4 本が前提にしてよいもの」）。

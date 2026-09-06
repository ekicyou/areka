# Brief: areka-P0-text-decoration-canon

> 起票: 2026-08-27（bvc〔`areka-P0-balloon-vertical-canon`〕要件ディスカッション議題 5 の開発者指示による `/kiro-discovery` 再入・文字装飾系 3 spec 分割の 1 本目＝核）
> **roadmap の M2 予約「②ポップアート級の文字装飾（text effects）」の具体的な着地先**。分割の要は「何が既に存在するか」——本 spec は per-run 文字属性だけで立つ 17 項目＋基盤を所有し、アンカー機能ごと不在の 16 項目は `anchor-tag-canon`、既存の選択肢マーカー上の 10 項目は `choice-marker-styling` が所有する。

## Problem

さくらスクリプトの `\f[...]`（文字装飾タグ族・全 43 項目）が areka で **1 項目も解読されていない**。`\f[bold,1]` は汎用字句解析を通った後、`decode_tag`（`decode.rs:191-221`）に `"f"` の腕が無いため `Instruction::Raw` へ落ち、`compile.rs:202-204` で「M-boot 外タグ」として黙って捨てられる。装飾を使う SSP 向けゴーストは areka で素の文字になる。

どの spec も `\f` を所有していない（2026-08-27 に `.kiro` 全域 grep で確認——言及は bvc・cursor-tag-canon の前方参照と roadmap M2 予約のみ）。

## Current State

2026-08-27 実測（file:line は当日検証値）。

- **`TextEffects` は正真正銘の空シーム**: `canvas.rs:130-137`＝`#[non_exhaustive] pub struct TextEffects {}`（フィールド 0）。予約名は文字列定数のみ（`"outline"`:39・`"multicolor"`:41・`"shadow"`:43・`"rotation"`:45）。格納はされるが読まれない。`canvas.rs:32` が「`\f` 系文字装飾・`disable.font.*` 拡張も同シームに属する」と明記。
- **per-run 属性が存在しない**: `TextItem::Glyph` は `ch` のみ（`state.rs:71-93`）・`PositionedGlyph`／`GlyphRunContent` も同様。フォントはバルーン全体で 1 つ（`ResolvedFont`・`draw.rs:150-166`・descript のみから構築）・`IDWriteTextFormat` 1 本（`draw.rs:302-331`）・weight/style は NORMAL 固定（`:333-339`）。repo 全域に `SetUnderline`/`SetStrikethrough`/`SetFontWeight`/`SetFontStyle`/`SetFontSize` は 0 件（唯一の `DWRITE_TEXT_RANGE` 使用は選択肢 hover の `SetDrawingEffect`・`viewbox_draw.rs:346-354`）。
- **align/valign は全書字方向で未実装**: `draw.rs:271-272` が `DWRITE_TEXT_ALIGNMENT_LEADING`／`DWRITE_PARAGRAPH_ALIGNMENT_NEAR` を固定。
- **descript `font.*` 基底 13 キーのうち解析済みは 5 つだけ**（`font.color.r/g/b`・`font.name`・`font.height`＝`parse.rs:98-105`）。`font.underline`/`bold`/`italic`/`strike`/`shadow*` は未解析。`disable.font.*` は予約シームのみ（`draw.rs:136`・`FontDisableSeam`:142-144）。
- **正典の規模**: `\f[...]` 全 43 項目。本 spec の所有分＝**核 17 項目**——font 系 10（`name`〔カンマ優先列・バルーン/ゴーストフォルダのフォントファイル可〕・`height`〔絶対 px・`+N`/`-N` 相対・`N%`〕・`color`・`bold`・`italic`・`underline`・`strike`・`sub`・`sup`・`outline`〔boolean 6 値=true/1/false/0/default/disable〕）＋影 3（`shadowcolor`・`shadowcolor,none`・`shadowstyle`=offset|outline・2.5.27）＋寄せ 2（`align`=行内・`\n`/`\_l` でリセット・遡及移動・2.5.31／`valign`=行厚み方向・**リセットされない**・2.6.19）＋全体 2（`default`・`disable`=2.5.51）。
- **縦書き写像（bvc が語彙確定済み・SC1 裁定込み）**: `align`＝left=上/right=下・`valign`＝**top=右/bottom=左**（正典 2 ページで逆＝SC1。**bvc requirements.md がバルーン定義ページ側を採る裁定を確定済み——本 spec は再審議しない**）・下線＝列の右側。

## Desired Outcome

`\f` 核 17 項目が 3 書字方向すべてで正典どおりに効き、descript `font.*` 基底 13 キーが既定層として機能し、`\f[default]`/`\f[disable]` のリセット意味論と `\x`/`\x[noclear]` の解除/保持（residue 項目 9 と相互登記）が確定している。

## Approach

語彙より**基盤が本体**: ⑴ `decode_tag` へ `"f"` 腕＋装飾 `CueCommand` variant 新設 ⑵ `TextItem::Glyph`→`PositionedGlyph`→`GlyphRunContent` の per-run 属性の配管 ⑶ `DWRITE_TEXT_RANGE` ベースの run 適用 ⑷ descript 基底 13 キーの解析拡張。**前提タスク＝`draw.rs` の分割**（974/1,000 行・残 26 行——機械番人が赤になるため着手前に必須）。

## Scope

- **In**: 核 17 項目（値形式・リセット規則・遡及移動・`\_l` 相互作用含む）／descript `font.*` 基底 13 キー／`disable.font.*` シームの実体化／`TextEffects` 予約 4 名の実体化（outline/shadow は 17 項目内・multicolor/rotation は M2 拡張シームのまま）／align/valign/underline の縦書き写像（bvc SC1 裁定を継承）／決定論テスト。
- **Out**: アンカー系 16 項目（`anchor-tag-canon`）／選択肢マーカー系 10 項目（`choice-marker-styling`）／`\x`/`\x[noclear]` の実装（`balloon-canon-residue` 項目 9 所有——ただし「`\f` 状態の何がリセットされるか」の権威定義は本 spec が供給し相互登記する）／`\_l` 側の実装（`cursor-tag-canon`——`\_l` 後の左寄せリセット・インデント相互作用は同 spec brief が本 spec を「align 実装 spec」として指名済み）。

## Boundary Candidates

- 基盤（decode 腕・CueCommand・per-run 配管・draw.rs 分割）と語彙群（font 10／影 3／寄せ 2／全体 2）の 2 相。
- descript 基底 13 キーの解析拡張は独立スライス。

## Out of Boundary

- バルーン定義キーの縦書き受口（bvc 完了済み）・フォント縦書き異体の挙動等価（bvc R6・実装済み・不変）。

## Upstream / Downstream

- **Upstream**: bvc（SC1 裁定・縦書き写像の語彙・R5 の登記元）・`cursor-tag-canon`（`\_l` 側の相互作用相手・どちらが先でも可だが相互参照必須）。
- **Downstream**: `anchor-tag-canon`・`choice-marker-styling`（本 spec の per-run 基盤に依存）・`balloon-canon-residue` 項目 9（リセット意味論を消費）・M2 text effects（multicolor/rotation）。

## Existing Spec Touchpoints

- **Extends**: `TextEffects`/`FontDisableSeam` シーム（emo-text 系完了 spec 群の予約を実体化＝予約名 doc の改訂を伴う）。
- **Adjacent**: `balloon-canon-residue`（項目 9 と相互登記必須・単独では着地不可）・`sstpmessage.font.*`／`number.font.*`／`communicatebox.font.*`（機能ごと M1 非実装＝各機能 spec の解禁時に本 spec の基盤へ乗る）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（文字装飾 3 spec の先頭・他 2 本のゲート）。
- **`draw.rs` 分割が着手前提**（残 26 行・1,000 行番人）。
- 決定論テスト必達（3 書字方向 × 17 項目・リセット経路込み）。SC1 は bvc 裁定を継承し再審議しない。

---

> **📌 2026-09-02 棚卸⑫（W13 裁定枠・先頭ゲート・XL＝分割推奨）**——アンカー再測定: `decode.rs:191-221` に `"f"` 腕なし ✅・`compile.rs:202-204` ✅・`state.rs:71-93` ✅・`draw.rs:136` ✅・`parse.rs:98-105` ✅・`SetUnderline` 等 repo 0 件 ✅。**ずれ（+6〜+15）**: `canvas.rs` `TextEffects` :130-137→**:136-143**・予約名定数 :39/:41/:43/:45→**:45/:47/:49/:51**・doc :32→**:38**／`draw.rs` `ResolvedFont` :150-166→**:158-172**・weight/style 固定 :333-339→**:348-349**・align/valign 固定 :271-272→**:277-278**・`FontDisableSeam` :142-144→**:150**。**事実誤認 1 件**: 「唯一の `DWRITE_TEXT_RANGE` 使用は `viewbox_draw.rs:346-354`」は誤り——`crates/wintf/src/ecs/widget/text/typewriter_draw.rs:245,:259` にも 2 件（wintf typewriter・spec 外既存資産）。
> **前提の逼迫**: `draw.rs` **974→980 行（残 20）**・番人の例外表に不在＝**分割が着手前提**（brief どおり・ただし余裕は減った）。`doc/emo2-conformance-scope.md:60` は bvc R11.9 で既に本 spec 群を所有者として明記済み（brief の「どの spec も未所有」は spec 上は真・doc は先行）。
> **分割の継ぎ目（開発者裁定）**: ⑴ 基盤相＝`draw.rs` 分割＋decode 腕＋CueCommand＋per-run 3 層配管（`TextItem::Glyph`→`PositionedGlyph`→`GlyphRunContent`）／⑵ 語彙相＝17 項目（font 10／影 3／寄せ 2／全体 2）／⑶ **descript `font.*` 基底 13 キー＝完全独立スライス**（`areka-parsers/balloon` に閉じ・共有ファイル 0）。`balloon-canon-residue` 項目 9 との相互登記＝単独着地不可（不変）。cursor-tag と `layout.rs`／`state.rs` を共有＝**W12 の cursor-tag 完走後**（W13）。⓪ の lexer 修正が `decode.rs` を先に触る＝rebase 吸収。**design 段階は Fable 推奨**。


### `areka-P0-cursor-tag-canon` からの追加登記（2026-09-04・同 spec Requirement 7.1／7.2 と tasks 6.3）

`\_l`（カーソル位置移動）を実装した `areka-P0-cursor-tag-canon` から、本 spec の所有範囲に属する未実装の副作用 3 件と、本 spec が触ると自ら宣言しているファイルで見つかった引受先不在の所見 1 件を送る。正典逐語の正本は同 spec `requirements.md` 付録 A（`:185-207`）。

- **追加登記 1: `\_l` 直後の行揃えリセット**（正典逐語: 「`\_l`実行直後には、トラブル防止のため行揃えが左揃えにリセットされる点に注意」「`\_l`タグが来た場合は左寄せに戻る」）。`\_l` 側は実装済みで、リセットされる側の `align` が未実装のため現状は観測できない。**リセットされるのは `align` だけ**——正典 `\f[valign,寄せる側]` は「alignと異なり、`\n`や`\_l`ではリセットされない」と明記する（2026-09-04 に ukadoc 本文で確認）。
- **追加登記 2: `\_l` 移動後の中央揃えのインデント**（正典逐語: 「`\_l`タグで移動後に`\f[align]`タグで中央揃えを設定した場合、`\_l`タグのX座標分インデント処理されたと仮定して中央揃え処理される」）。`\_l` の着地位置は `crates/areka-emo-text/src/cursor_tag.rs` が解決済みなので、その X 座標を寄せの計算へ渡す口だけが欠けている。
- **追加登記 3: 疑義 SC8（縦書きでのインデント軸）＝未解決**。上の追加登記 1・2 の正典文は X 軸で書かれたままで、縦書きへの更新がない。縦書きでは行内軸が Y なのでインデント量も Y から来るはずだが、正典に記述がない。`cursor-tag-canon` は裁定せず登記だけを行った（同 spec `requirements.md:139`＝Requirement 7.2・付録 B の SC8 行 `:217`）。**本 spec が裁定する**——同 spec `design.md` の「語彙登記と申し送り」節が本 spec を追跡先として名指している。
- **追加登記 4: `layout.rs` のあふれ判定で、行送り方向へ後戻りした行が境界の外に置き去りになる**（引受先が無いまま残っている所見）。`LayoutEngine::visible_window`（`crates/areka-emo-text/src/layout.rs:634`）は可視範囲を**最新行の遠端だけ**で判定する（`:653-654`）ため、`\_l` で行送り方向へ戻ると、境界の外にある前の行がそのまま残る。実測（横書き・境界 36）: 4 行目の下端 49 が外に残るのに、最新行の下端が 23 なのであふれは発火しない。最小スキップの探索（`:665`）も、行が行送り方向へ単調に並ぶことを暗黙の前提にしている。`cursor-tag-canon` は式の変更を自らの要件（Requirement 2.8）で禁じているため、**今日の値を固定するだけ**にとどめた（`crates/areka-emo-text/src/layout_cursor_overflow_tests.rs:113-166`＝値が変わると赤になる）。本 spec へ送る根拠は、本 spec が `\f[align]`／`\f[valign]` の寄せ（＝行の置き場所）を実装し、`layout.rs` を `cursor-tag` と共有すると自ら宣言していること（上記 `:33`・`:65`）。**寄せは行矩形を行送り軸の方向へも動かす**（正典により縦書きの `valign` は左寄せ／右寄せ＝行送り軸）ので、同じ前提に触れる。**本 spec が要件フェーズで引き受けないと判断した場合は、`areka-P0-ukadoc-survey-sakura-script` の台帳（342 語彙の担当割当）へ差し戻すこと。**

### `emo-text-line-height-canon` からの相互参照（2026-09-06・同 spec Requirement 6.9／9.4／10.3・同 tasks 6.3）

`areka-P0-emo-text-line-height-canon`（行送りの正典化・W12 裁定枠 A′・2026-09-06 実装完了）から、本 spec が着手前に読む前提として 3 点を送る。上の「追加登記 4」の本文は 1 文字も書き換えていない——本節はそこへの**相互参照**である。

- **相互参照 1: 「追加登記 4」は本 spec の所有のまま（line-height は引き受けなかった）**。`emo-text-line-height-canon` は同登記（行送り方向へ後戻りした行があふれ判定の境界の外に残る所見）を**引き受けていない**（同 spec 要件 9.4）。あふれ判定の式——最新行の遠端だけで判定・最小スキップの探索・全行超過時の飽和——は 1 文字も変えておらず、判定の分岐も増減していない。行った作業は、行送りが 13 → 12 へ変わったことによる**現状値の再導出だけ**である（`crates/areka-emo-text/src/layout_cursor_overflow_tests.rs`＝値が変わると赤になる決定論テスト。行矩形の丈＝`font_height` は不変）。したがって本 spec が寄せ（`\f[align]`／`\f[valign]`）を実装するときに触れる前提は、登記された当時のまま残っている。
- **相互参照 2: `font.height` の意味論と行送りの式を継承すること**。開発者裁定（2026-09-05）で `font.height` は **em サイズ**（DirectWrite の fontSize へ値のまま渡す）と確定した。`\f[height,N]`／`\f[height,+N]`／`\f[height,N%]` はこの意味論をそのまま継承する——値を行ボックスの丈と読み替えたり、係数で縮めたりしない。行送りは `line_pitch = font.height + 行間`（行間の既定は**定数 2 image px**）で、式の置き場所は `crates/areka-emo-text/src/state.rs` の `TextLayerConfig::line_pitch` の **1 点**である（旧 `TextLayerConfig::line_pitch_factor` は消えた）。`\f[height]` が行の丈を動かすなら、必ずこの 1 点を通すこと。正典表の全文は同 spec `design.md` §4.1、areka 裁量としての登記は `doc/COMPAT_ARCHITECTURE.md` §8 にある。
- **相互参照 3: 行末禁則のぶら下がりの引受先は residue 側（本 spec ではない）**。`wordwrappoint`＝折返し基準（超えたら折り返す）・`validrect`＝描画範囲の絶対上限（超えてはならない）の二段構えは実装済みだが、**行末禁則文字のぶら下がり（折返しの遅延）は未実装**で、引受先は `areka-P0-balloon-canon-residue` の**項目 15**（2026-09-06 登記）である。`emo-text-line-height-canon` 要件 6.9 の候補欄は本 spec を挙げていたが、バルーン定義の粗さ側の残件（同 residue 項目 14）と同じ軸にあるため residue へ置いた。**本 spec は引き受けなくてよい。**

> **前提の逼迫（`draw.rs`）——本 spec の Constraints の数字が動いた**: 本 spec の Constraints は「`draw.rs` 分割が着手前提（残 26 行）」と書き、棚卸⑫の追記は「974→980 行（残 20）」と更新したが、`emo-text-line-height-canon` の着地で `crates/areka-emo-text/src/draw.rs` は **988 行**になった（main 980 → +8・行数の見張りの上限 1,000 まで残り **12 行**）。同 spec は `draw.rs` の分割を行っていない（境界の外・先取り分割の登記も置いていない）。`draw.rs` を触る次の spec は本 spec なので、**分割は本 spec の最初の作業**として見込むこと。見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の例外表は 1 行も変わっていない（`areka-emo-text` は例外表に不在のまま）。

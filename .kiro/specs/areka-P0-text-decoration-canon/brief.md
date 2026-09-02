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

# Brief: emo-text-dwrite-fidelity

> 2026-10-24 `/kiro-discovery` で起票（開発者がぱすたさんの縦書きの画面を見て「「と、」に、不自然な空白が入ってませんか？DWriteの縦書きに関する設定が何か抜けているのでは。」・ここで言う「と、」は括弧の「「」と「」」のこと）。v0.1.0・区分 B（バグ）。本文の file:line と数値は起票時（main `81d3ebdb`）の実測＝着手時に引き直すこと。

## Problem

- **ぱすたさん（v0.1.0 の検体「窓際のぱすたさん」）を見る人**: 縦書きのバルーンで、開き括弧「「」と続く字の間に、不自然な空きが出る（「……「繋がってる」」の「「→繋」）。
- **ゴーストとバルーンの作者**: SSP で通じるフォント名「游ゴシック Medium」「Yu Gothic Medium」が areka では見つからず、意図した中太でなく Regular の太さで描かれる。ぱすたの縦書きバルーン（`pasta_tategaki\descript.txt` の `font.name,游ゴシック Medium,Yu Gothic Medium,游ゴシック,Yu Gothic,BIZ UDPゴシック,メイリオ`）がちょうどこれに当たる。中太は作者の意図（ぱすた開発に確かめた・画面設計の要件「中太ゴシック or 中太明朝」）。

## Current State（起票時の実測）

areka の本番の描き方（`present_frame` → 読み戻し・游ゴシック Regular 20px・縦書き `VerticalRl`・倍率 1.0）で、字ごとの墨の位置を測った（測りの仮のテストは消してある）。

- **列は 1 本の TextLayout で描いている**。字ごとの送り幅は、1 字だけで測っても行の中で測っても全部 20.00px で、areka が列を切り貼りしてずれているのではない（`viewbox_draw_render.rs` の 1 行 1 TextLayout・`draw_metrics.rs` の 1 字の計測は行と一致）。
- **件 1「「」の墨が字の枠の上端にある**: 「「」は枠の上から 2〜8px（20px の枠の上の 3 分の 1）。縦書きの開き括弧は、続く字の側＝**下端**に寄るのが正しい組み方（JIS X 4051）。そのせいで「…→「」は 4px、「「→繋」は 11px 空く。
  - 「」」は枠の上から 0〜6px で正しい（「る→」」は 2px）。
  - 形は縦書き用（﹁﹂）で、列の左右の真ん中にも乗っている（墨の中心の x は 繋・る と ±1px）。
  - 太さを 500（中太）にしても同じ（±1px）。
  - **同じ游ゴシックを Chromium（字形の並べは HarfBuzz・ラスタライズは DirectWrite）の縦書きで描くと、「「」は下端に寄って続く字にくっつく**。areka の DirectWrite の TextLayout の道だけが上端に置いている＝DirectWrite の縦書きの設定か使い方の何かが抜けている見込みが高い（どれかは未特定）。
  - 書式の言語は `ja-JP`（`draw.rs` の `LOCALE_JA_JP`）で、ここは正しい。
  - `IDWriteTypography` で `vchw`・`halt` を当てると「「」の墨は下端へ動く（上から 12〜17px）が、送り幅は 20px のまま。空きが「…→「」の側へ移るだけ。`vhal`（括弧だけ）は括弧を動かさず範囲の外の「っ」を 2px 動かし、`vpal` は何も変えない（どちらも説明がつかない）。
- **件 2 フォント名**: DirectWrite のシステムのフォント集には「游ゴシック Medium」「Yu Gothic Medium」というファミリが無い（`FindFamilyName` が false）。DirectWrite では中太はファミリ「Yu Gothic」の中の太さ 500。そのため `draw_catalog.rs` の `FontCatalog::family_for`／`has_family` が先頭の 2 つを飛ばし、3 番目の `游ゴシック` を Regular で選ぶ。`draw.rs` の `try_create_format` と `draw_metrics.rs` の `probe_format_for` は太さを NORMAL に決め打ちしている。SSP は GDI の名前で引くので中太になるはず（SSP では未確認）。
  - 正典（ukadoc `descript_balloon` の `font.name`）: 「カンマ区切りでフォント名を複数指定可で、書いた順＝優先度順で自動的にインストールされている中から選択する（SSPのみ）」。areka もカンマ区切りの先頭から入っているものを選ぶ形は実装済み。足りないのは「GDI の名前（ファミリ名＋太さ）を、入っているものとして見つける」ことだけ。
- **件 3 列の左右のずれ（測りのついでに見つけた）**: 列の字の墨は x 405〜424（中心 約 414.5）に描かれるのに、areka が覚えている行の矩形は x 411〜431（中心 421）。列全体が、areka の思う位置より約 6px 左に描かれている。括弧の空きの原因ではない。`vertical-choice-hover-band`（選択肢の反転の帯のずれ）と関わるかもしれない。同 spec の要件の段のセッションは起票時に閉じていて知らせられず、要件はブランチ `claude/kiro-start-vertical-choice-hover-band-9ac66b` の上にある（既定ブランチの brief を書き換えると競合するので触っていない）。持ち主は、本 spec と同 spec のどちらか先に要件の段へ入った方が、もう一方の要件を見て決める（決めるまでは本 spec の件 3）。

## Desired Outcome

- 縦書きの「「」が、続く字の側（字の枠の下端）に寄って描かれる。「」」は今のまま。横書きは変えない。
- 根本の原因（DirectWrite の縦書きの何が抜けていたか）を突き止め、areka の方で正しく設定して直す。手で字をずらす繕いは、設定で直らないと確かめた後の最後の手段。
- 送り幅を変える直しになった場合は、1 字の計測（`draw_metrics.rs`）と行の TextLayout（`draw_line_store.rs`）の両方に同じに効かせる（片方だけだと列の位置と折り返しがずれる）。
- `font.name`（と `\f[name]`）の各候補を、DirectWrite のファミリ名で見つからなければ GDI の名前（ファミリ名＋太さ・斜体の名前）として引き直し、見つかればそのファミリと太さ・斜体で描く。オプションにはしない（正典の名前は SSP＝GDI の名前で書かれるため）。ぱすたの descript は今のままで中太になる。
- ぱすたさんの縦書きの台詞を SSP と並べ、括弧の位置と字の太さが揃うことを確かめる（実機の確かめは `doc/pasta-real-run.md` の入れ方で）。

## Approach

- 件 1: まず原因を測り分ける。候補は、TextLayout の縦書きの字形の向きの扱い（`IDWriteTextFormat1::SetVerticalGlyphOrientation`・`IDWriteTextAnalyzer1` の縦書きの字形の向きと位置の変換）、縦書きの OpenType の機能（`vert`／`vrt2`／`vhal`／`vchw`）の当たり方、フォントの縦の寸法（`vmtx`／`VORG`）の読まれ方。Chromium の並べ（HarfBuzz）を手本に、DirectWrite の TextLayout と `IDWriteTextAnalyzer` の字形の位置を並べて比べる。決定論のテストは、縦書きの読み戻しの既存の試験（`crates/areka-emo-text/tests/decoration_readback/vertical.rs` の形）で「「」の墨が枠の下半分にあること」を檻にする。
- 件 2: `FontCatalog` に「GDI の名前→ファミリと太さ・斜体」の引き直しを足す（`IDWriteGdiInterop::CreateFontFromLOGFONT`、またはフォントセットの `DWRITE_FONT_PROPERTY_ID_WIN32_FAMILY_NAME`／`FULL_NAME` での照合）。引き直した太さ・斜体を、描画の書式（`try_create_format`）と計測の書式（`probe_format_for`）の両方へ渡す。DirectWrite のファミリ名で見つかる名前は今と同じ結果のまま。
- **要件の段の議題（開発者の案・2026-10-24）**: 開発者から「フォント名の宣言を、最初に一致するものとして、カンマ区切りで複数書けるようにすればいいのでは。SSP が無視してくれると良いのだけど」という案が出た。カンマ区切りの先頭から入っているものを選ぶ形は、正典にも areka にも既にあり、SSP は入っていない名前を飛ばす。ただし DirectWrite には「中太」を表すファミリ名が無い（中太はファミリ「Yu Gothic」の中の太さ 500）。そのため、列に areka 向けの名前を足して中太を選ばせるには、太さを書ける areka だけの書き方（例「Yu Gothic:500」）が新しく要る。それでも SSP 向けに作られた世の中のゴーストは Regular のまま。**起票時の推奨**は、書き方を増やさず「GDI の名前も、入っているものとして見つける」こと。カンマ区切りの先頭から選ぶ決まりはそのままで、その中の 1 つずつが GDI の名前でも当たるようになる。続けて開発者から「もしくは、フォント検索処理をより高度にする？GDI名でもマッチするように」＝推奨と同じ方向が出た。この方向で要件を書き、要件の段で確かめる。

## Scope

- **In**: 縦書きの開き括弧の位置（根本の設定の直し・決定論のテスト・SSP との見比べ）。`font.name`・`\f[name]` の GDI の名前の引き直し（太さ・斜体まで・描画と計測の両方）。`number.font.name`・`sstpmessage.font.name`・`communicatebox.font.name` が同じ `FontCatalog` を通るなら一緒に効く（表示そのものは各担当）。網羅台帳の `font.name` の行の注記。
- **Out**: 括弧を半分に詰める組み方（約物の詰め・「」→っ」の全角の空き）＝`text-typesetting` の件。小書きかな（っ・ょ）の下の空き＝フォントの縦書き用の字形どおり（Chromium でも同じ空き方）で直さない。同梱フォントファイルの読み込み＝`balloon-font-file`。

## Boundary Candidates

- 件 1: DirectWrite の縦書きの設定（`draw.rs` の `DirectionRecipe`・`create_text_format`、行の TextLayout の `draw_line_store.rs`、1 字の計測の `draw_metrics.rs`）。
- 件 2: フォント名の解決（`draw_catalog.rs`）と、解決した太さ・斜体の書式への受け渡し（`draw.rs` の `try_create_format`・`draw_metrics.rs` の `probe_format_for`）。

## Out of Boundary

- 約物の詰め・ぶら下げ・禁則・縦中横・字の向き（`text-typesetting`）。
- フォントファイルの読み込み（`balloon-font-file`）。
- 選択肢の反転の帯（`vertical-choice-hover-band`）。件 3 の列の左右のずれは、同 spec が引き取るならそちら。

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-balloon-vertical-canon`（縦書きの寄せと下線の写像）・完了 `vertical-text-layout`・完了 `areka-P0-balloon-font-descript-keys`（`font.name` の読み）・完了 `ghost-inner-balloon`（ぱすたの縦書きバルーンの置き場）。
- **Downstream**: `balloon-font-file`（同じ `FontCatalog` にファイルのフォント集を足す＝本 spec の引き直しの後に載せる）・`text-typesetting`（縦書きの字形・約物の詰め）・v0.1.0 の版の門の最後の確かめ（`doc/pasta-real-run.md`）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。
- **Adjacent**: `balloon-font-file`（`draw_catalog.rs`・`draw.rs` が重なる＝同じウェーブに置かない）・`vertical-choice-hover-band`（emo-text の縦書き・件 3）・`text-typesetting`（約物の詰めを引き取る）・`text-align-shadow-canon`（縦書きの寄せ）。

## Constraints

- DirectWrite（`windows` crate）の範囲で直す。新しい外部依存は足さない。
- 横書きの見た目を変えない（既存の読み戻しの試験が緑のまま）。
- GPU のテストは実 GPU（WARP 案は出さない）。
- SSP との見比べは SSP の MCP（`dump_balloon`・`raise_event`）で、測る前に他のセッションへ知らせる。

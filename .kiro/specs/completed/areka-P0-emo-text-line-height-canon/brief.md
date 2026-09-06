# Brief: areka-P0-emo-text-line-height-canon

> **起票 2026-09-05（`areka-P0-emo2-conformance-e2e` の実機一周走行 A が A14 で踏んだ構造的な製品欠陥・開発者裁定「別 spec を切って先に直し、その後に一周を採り直す」）**。上流の登記: `.kiro/specs/areka-P0-emo2-conformance-e2e/verification/acceptance-record.md` §13.2 #1／#2・同 §13.3・同 `tasks.md` Implementation Notes「⛔ 走行 A（2026-09-05）は A14 で中断」。**M1 完成判定（e2e）のブロッカー**であり、本 spec の完遂が e2e の一周採り直しの前提である（R8.3）。

## Problem

相方側バルーン（`emo2-kakukaku`・`balloonk0s.txt` の `validrect.top,40`／`bottom,-70`＝文字描画範囲の高さ **93px**）で、`menu.pasta` のメニュー（3 行・3 行目は `\_l[5em,2lh]` の字下げ）が収まらず、**先頭の選択肢が描かれない**。3 本のメニュー全部で先頭（「おしゃべり頻度」「しゃべくり」「調整」）が消える。利用者から見ると「メニューの項目が足りない」。SSP は同じバルーン・同じ `font.height,28` で 3 行を 93px に収める（開発者の SSP 実機スクショから逆算して ≈31px/行）。

脳（pasta）は 3 件返し、kanade も 3 件登録し（生ログ `choice_count=3`）、レイアウトも 3 行を正しい座標に置く。欠けるのは**縦の寸法**だけである。

## Current State

- 行送り: `line_pitch = ceil(font.height × 1.25)`（`crates/areka-emo-text/src/state.rs:64-66` の `TextLayerConfig::line_pitch_factor`・`draw.rs:477-479`）。`font.height,28` → **35px**。3 行目の行矩形は y110..138 で `validrect.bottom`＝133 を 5px あふれる。
- あふれ判定: `LayoutEngine::visible_window`（`crates/areka-emo-text/src/layout.rs:532-546`）が「最新行の下端 > validrect.bottom」で 1 行スクロールを返し、`draw.rs:766-774` の `.skip(first_visible_line)` が先頭行を描画対象から外す。文字供給面は validrect ちょうど（`actor.rs:662-669`・実機ログ `physical_size=(432,186)`＝216×93×k2）で逃げ場が無い。
- `font.height` の解釈: DirectWrite の `CreateTextFormat` の **em サイズ**として渡している（`draw.rs:339-351`）。Yu Gothic UI 28em の実行ボックス丈は約 37.2px。SSP（GDI `lfHeight`＝セル丈＝ascent+descent の意味論）とは食い違う疑いが濃い。
- 1.25 は正典値ではなく **areka 裁量値**。完了 spec `areka-P0-emo-text-layer` の design（`.kiro/specs/completed/areka-P0-emo-text-layer/design.md:725`「SSP の行間はユーザ設定＝正典値なしのため areka 裁量値」）に明記され、同 research（`research.md:200`）が「SSP 実測との視覚差が出る可能性」としてリスク登記していた箇所そのもの。
- 決定論の再現（実 parser → 実 compile → 実 state → 実 `TextRegion::resolve`（descript＋balloonk0s の 2 層マージ）→ 実 layout）で確認済み: `region (24,40)-(240,133)` / 行矩形 y40..68・75..103・110..138 / `visible_window = { first_visible_line: 1, block_offset: -35 }`（CharByChar・budoux Segmented とも同一）。製品コードは未改変。
- 関連症状（同根）: `wordwrappoint.x,-34` が画像幅 288 基準で 254 に解決され `validrect.right`（240）の外に出るため、`\_l[5em,…]` 後の「閉じる」（x164..248）の右端 8px が供給面で欠ける。バルーン定義の粗さと areka の供給面寸法の関係に跨る。
- 既存の檻は 1.25 前提で多数固定されている（`layout_*_tests.rs`・`draw_oracle_tests.rs` の byte 等価 golden・`viewbox_draw_*`・`layout_visible_window_tests.rs`）。

## Desired Outcome

1. `font.height` の意味（**箱の丈（セル丈）か em サイズか**）と、行送りの式（`1lh = 1em + 行間`・行間の既定）が **SSP 実測**で確定し、steering／completed spec の正典表（emo-text-layer design の「補足正準」）が書き換わっている。
2. areka が同じバルーン・同じ `font.height` で SSP と同じ行数を収め、行ボックスが重ならない（インクの重なり無し）。`emo2-kakukaku` の相方側で `menu.pasta` の 3 台本すべての選択肢が可視窓に収まる。
3. 決定論の檻: 実物 `emo2-kakukaku` の descript＋balloonk0s を 288×203 で解決し、`menu.pasta` の 3 台本すべてで `visible_window.first_visible_line == 0`（どの選択肢も可視窓から落ちない）。加えて SSP 実測値との対照（行送り・行ボックス丈）を数値で固定する。
4. 「閉じる」右端欠け（`wordwrappoint` と `validrect.right` の関係・供給面の寸法）についての裁定と、必要なら是正。
5. `\_l[N lh]` の着地が新しい行送りへ自動追随する（`cursor-tag-canon` の解決層は `line_pitch` を引数で受けるだけ＝算出式は本 spec が所有）。

## Approach

**本命（推奨）＝意味論を SSP へ寄せる一点変更。** SSP と同じバルーンで同じ文字列を並べて撮り（k=1 と k=2 の 2 水準）、行送りピッチ・行ボックス丈・ベースライン位置を実測して `font.height` の意味を確定する。確定した意味に従い、DWrite へ渡す em サイズを `font.height ÷ line_box_ratio`（セル丈解釈なら）へ変え、`line_pitch = font.height + 行間（既定 0）`・`1lh = font.height` とする。`DWriteMetrics::line_box_height`／`line_pitch`／`TextLayerConfig` を同じ源から導く。3 行は y40/68/96・最終行下端 124 ≤ 133 で収まり、行ボックスも重ならずちょうど敷き詰まる。

**却下＝係数 1.25→1.0 の応急処置。** スクロールは止まるが、実行ボックス丈 37.2px が行送り 28px を超えたままで行間のインクが重なる。あるべき姿ではない（開発者裁定 2026-09-05）。

**進め方**: SSP 実測 → 正典表の改訂（emo-text-layer design 補足正準・steering）→ `draw.rs` の format 生成と metrics → 既存 golden／檻の期待値を**導出し直す**（緩めない）→ 新檻（上記 3）→ e2e の一周採り直しへ引き渡し。

## Scope

- **In**:
  - `font.height` の意味論の確定（SSP 実測・ukadoc の記述は em/セルを区別しない）と正典表の改訂。
  - 行送りの式（`line_pitch`・`1lh`・行間の既定）と `TextLayerConfig` の改訂。
  - `draw.rs` の DWrite format 生成（em サイズの導出）・`DWriteMetrics`・`line_box_height` の整合。
  - `visible_window` の**呼び側**（行矩形の丈・境界）の追随。式そのものは変えない（`cursor-tag-canon` 要件 2.8 と整合）。
  - 「閉じる」右端欠け（`wordwrappoint.x` が `validrect.right` の外に解決される場合の供給面／折返し閾値の関係）の裁定と是正。
  - 既存 golden・檻の期待値の再導出、新規の決定論檻（実物 `emo2-kakukaku` × `menu.pasta` 3 台本）、SSP 実測値の固定。
- **Out**:
  - `\_l` の座標語彙・書字方向の座標解決（`cursor-tag-canon` 所有・完了済み前提）。
  - `\f[align]`／文字装飾（`text-decoration-canon`・W13）。本 spec は `draw.rs` に触るので decoration より**前**に着地させる。
  - バルーン縦書きの受口（`balloon-vertical-canon`・完了）。縦書きの行送り（列送り）は同じ式を軸読み替えで適用するだけで、意味論を新設しない。
  - kanade／pasta 側（脳は正しく 3 件返している）。

## Boundary Candidates

- 意味論の確定（SSP 実測＋正典表）／実装の追随（`draw.rs`・`state.rs`・metrics）／檻の再導出、の 3 相。分割はしない（意味論と実装は同じコミット列で揃えないと正典表と実装がずれる）。

## Out of Boundary

- `visible_window` のあふれ判定式の変更（要件で「変えない」を固定する。変えたい場合は別途裁定）。
- `emo2` 以外のバルーン資産の是正（`wordwrappoint` を validrect の内へ直す等はバルーン側の裁量＝本 spec は areka の挙動を定めるだけ）。
- 行送り方向へ後戻りした行があふれ判定の境界で見せる挙動（`text-decoration-canon` brief「追加登記 4」として登記済み・本 spec は相互参照のみ、または要件段階で引き取りを裁定）。

## Upstream / Downstream

- **Upstream**: `areka-P0-cursor-tag-canon`（**main マージ後に rebase してから着手**。同 spec は `layout.rs` +438／`state.rs` +90／`region.rs` +113 を全面改変し `cursor_tag.rs` を新設。`visible_window`・`draw.rs`・1.25 係数は無改変＝別セッション回答 2026-09-05）。完了 spec `areka-P0-emo-text-layer`（正典表の改訂対象）・`areka-P0-balloon-vertical-canon`（region 解決）。
- **Downstream**: `areka-P0-emo2-conformance-e2e`（本 spec 完遂後に走行 A〜D を採り直し、M1 完成判定へ）・`text-decoration-canon`（`draw.rs` を後から触る＝本 spec の後）・`choice-marker-styling`。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-emo-text-layer`（completed・design「補足正準」の行送りの行と research のリスク登記を消化）。
- **Adjacent**: `areka-P0-cursor-tag-canon`（`lh` 係数の受け手・算出式は本 spec）・`areka-P0-text-decoration-canon`（`draw.rs` 共有・直列）・`areka-P0-balloon-canon-residue`（バルーン記述の粗さの台帳）。

## Constraints

- SSP 実測が要る（同じバルーン・同じ文字列・2 水準の拡大率で撮り、px を読む）。ukadoc の「フォントの高さ方向の大きさ」は em／セルを区別しないため正典文書だけでは決められない。
- 既存の byte 等価 golden は期待値の**再導出**で追随する（緩めない・記憶 obsolete-vs-broken-test-policy）。1,000 行の見張りを超えない（分割は主題単位）。
- `draw.rs` は 1 ウェーブ 1 spec（decoration との直列）。`file_length_guard_test.rs` の例外表には触らない。
- 実装フェーズは e2e の一周採り直しをブロックしているので、意味論が確定したら最短で着地させる（長時間試行禁止）。

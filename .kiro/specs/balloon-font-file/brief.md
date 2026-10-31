# Brief: balloon-font-file

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。普通のバルーンにも効く**正典の実装**。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストとバルーンの作者**: フォルダに同梱したフォントファイル（例: Zen Kurenaido）で台詞を書けない。正典はこれを認めている——バルーンの descript.txt `font.name` は「バルーンのフォルダに置いたフォントファイルも指定可能（SSPのみ）」、`\f[name,…]` は「ghost/master以下や現在のバルーンのフォルダに置いたフォントファイルも指定可能」（例 `\f[name,メイリオ,meiryo.ttf]`）。`number.font.name`・`sstpmessage.font.name`・`communicatebox.font.name` も同じ文言。
- シェル内バルーン（`shell-balloon`）の参考ゴースト「窓際のぱすたさん」は同梱フォントの縦書きが前提。

## Current State

- `crates/areka-emo-text/src/draw_catalog.rs` は `font.name` のカンマ区切りをシステムのフォント集から引く。**`.ttf/.otf/.ttc` は警告を出して意図的に読み飛ばす**（「読み込みは実装しない」）。`draw.rs` の `CreateTextFormat` はフォント集に `None` を渡す＝独自のフォント集・読み込み器はリポジトリに無い。`\f[name]` も同じ規則。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `font.name` と `sakura-script.toml` の `\f[name]` の状態は着手時に確かめる。

## Desired Outcome

- `font.name` と `\f[name,…]` にフォントファイルを書くと、そのフォントで描かれる（カンマ区切りの優先順は正典どおり。ファイルとファミリ名の混在も可）。
- 探す場所: 普通のバルーンは**バルーンのフォルダ → ゴーストのフォルダ（ghost/master）**、シェル内バルーンは**シェルのフォルダ → ゴーストのフォルダ**（2026-10-01 開発者確定。探し場所の一覧は `shell-balloon` が渡す）。
- **縦書きでも同梱フォントのまま描く**。SSP は縦書きで同梱フォントを環境の標準ゴシックの縦書き用異体へ自動で差し替える（ukadoc `vertical` の項）が、これは GDI の `@` 付きフォントの制約によるもので、DirectWrite には無い＝areka は差し替えない（意図的な差。`doc/COMPAT_ARCHITECTURE.md` §8 に登記する）。
- 読めないファイル・壊れたファイルはログに理由を残し、優先順の次へ進む。

## Approach

- DirectWrite のフォントセット（`IDWriteFontSetBuilder` に `AddFontFile`→`CreateFontCollectionFromFontSet`）でフォルダのファイルをフォント集にし、システムのフォント集と合わせて `CreateTextFormat` へ渡す。新しい外部依存は要らない（`windows` crate の機能フラグの範囲）。
- フォント集はバルーン（とシェル内バルーンのシェル）ごとに作り、資産の差し替え（`shell-balloon-switch` の流れ）で作り直す。

## Scope

- **In**: `font.name`・`\f[name]` のファイル指定（バルーン・ゴースト・シェルの各フォルダ）、`number.font.name`・`sstpmessage.font.name`・`communicatebox.font.name` も同じ読み手を通す（表示そのものは各担当）、縦書きでの扱い、ログ、決定論テスト（試験用の小さなフリーのフォントファイルを検体にする＝ライセンスを `THIRD-PARTY-NOTICES.md` に書く）、網羅台帳の更新。
- **Out**: フォントのインストール（OS への登録）・Web フォントの取得・字形の合成。

## Boundary Candidates

- フォントの読み込みとフォント集の寿命（`draw_catalog.rs` の周り）と、`CreateTextFormat` への受け渡し（`draw.rs`）。

## Out of Boundary

- シェル内バルーンの探し場所の決定（`shell-balloon`）。
- 縦書きの字形の確かめ（`text-typesetting`）。

## Upstream / Downstream

- **Upstream**: α の完成宣言。`shell-balloon` とは並走できる（触るのは `draw_catalog.rs`・`draw.rs` の周り。`shell-balloon` が触る `actor.rs`・`region.rs` と別）。
- **基準のフォルダ**（2026-10-25 起票の `talk-script-metadata`・v0.2.0）: ファイルを指すタグの相対パスは、台本の付帯情報（実行したゴーストの置き場）と、その spec の解く関数 1 つで読む。この spec が先に着地するなら、受け手が自分で `GhostSlot` を引く形にしない（`talk-script-metadata` へ乗り換えやすい口に留める）。
- **Downstream**: `text-ruby`（ルビのフォント）・`balloon-markers`（`number.font.name`・`sstpmessage.font.name`）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `text-align-shadow-canon`・`emo-text-canon-residue`（同じ `areka-emo-text`）。

## Constraints

- `draw.rs` 761 行＝足す量によっては新しいファイルで足す。1 ファイル 1,000 行。
- 試験用フォントの同梱はライセンスの確認を要件で決める（OFL など再配布可のものに限る）。

## 2026-10-03 ウェーブ C3-②（予定・10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」））

- 段は「優先」。C3 は C2 の着地で brief が動くので、着手の前に同じウェーブの他の spec と触るファイルを照合し直す（`roadmap.md`「ウェーブ編成」の C3 の行）。

## 2026-10-30 `emo-text-dwrite-fidelity` の着地前の申し送り

- **前提の変化**: `emo-text-dwrite-fidelity` が `FontCatalog`（`crates/areka-emo-text/src/draw_catalog.rs`）の公開の口を変えた。名前は顔 `FontFace`（家族・太さ・幅・斜体）へ解き、`face_for`・`face_or_default`・`face_or` が顔を返す。候補ごとに家族名で引き、無ければ GDI の名前（例「游ゴシック Medium」）で引く。`family_for` は `face_for` の家族名だけを返す薄い口になった。書式は `draw.rs::try_create_format`（顔＋太字・斜体）で作る。
- **この spec への影響**: 「`.ttf` などを読み飛ばす所」と「書式を作る所」の位置と形が変わった。着手時に、同梱フォントを引く計画（読み込んだフォントの名前を候補列で引く所・書式を作る所）をこの口に合わせて引き直すこと。
- **件（申し送り・完了の棚卸で足した）**: 束縛の顔で書式が作れず ＭＳ ゴシック Regular で作り直したとき、計測（`draw_metrics.rs` の `bound_face`）と描画（`viewbox_draw_decoration.rs::apply_font_ranges` の `face_or_default`）は作り直す前の顔を覚えたまま。全滅の計測鍵は失敗した顔で書式を作り直そうとして `Device` の誤りになりうる。直すには `create_text_format` が実際に使った顔を返す形などへ変え、呼び手 3 か所（`viewbox_draw_render.rs`・`DrawExecutor`・`draw_metrics.rs`）を揃える設計の判断が要る。`CreateTextFormat` は名前が無くても失敗しないので、今の作りでは作り直しを決定論の試験で起こせない（偽物を差し込む口が要る）。同梱フォントを読むと書式の作り方が変わる本 spec で、書式を作る所と一緒に扱う。

## 2026-10-31 棚卸㉜の再測定（main d374f901）
- **前提の変化**（棚卸㉔の後）: `emo-text-dwrite-fidelity`（10-30）が `FontCatalog` の口を顔 `FontFace` へ変えた（上の申し送りの節）。`script-small-tags`（10-31）は `draw.rs` の `ResolvedFont` に行の中の印の見た目 `inline_marker` を足し、`actor*.rs`・`viewbox_draw*.rs` も書き換えた。`choice-ranges-one-function`（10-31）は `actor_present.rs` を書き換えた。本 spec が使う所の形は変わっていない: 箱の探し場所の欄（`actor.rs` 271 行目の `box_font_dirs`）と入れ口・読み口（`actor_box.rs` 126・140 行目・**本番の読み手は今も 0**）・`.ttf` などを読み飛ばす所（`draw_catalog.rs` 49 行目の `FONT_FILE_EXTENSIONS`）・描画の組み立て（`actor_decoration.rs` 49 行目の `build_actor_render`・呼び手は `actor_present.rs` 250 行目の 1 つ）・書式を作る 3 か所（`draw.rs` の `ensure_format` 742 行目・`draw_metrics.rs` 144 行目・`viewbox_draw_render.rs` の `ensure_format` 641・646 行目）・フォルダ名を渡す 2 か所（`crates/areka/src/emo2_boot/frame/attach.rs` 428 行目・`frame/switch.rs` 544 行目）。
- **触るファイル**: `crates/areka-emo-text/src/` の `draw_catalog.rs`（360 行）・`draw.rs`（846）・`draw_metrics.rs`（462）・`viewbox_draw.rs`・`viewbox_draw_render.rs`（791）・`viewbox_draw_decoration.rs`（申し送りの件）・`actor_decoration.rs`・`actor.rs`（841）・`actor_attach.rs`・`actor_box.rs`、新しいファイルと `lib.rs` の一覧（検査や読み込みのファイルを足すとき＝1 本だけの席）、`crates/areka/src/emo2_boot/{frame/attach.rs, frame/switch.rs, assets.rs}`、試験用フォントの検体とライセンス文（`THIRD-PARTY-NOTICES.md` は生成物＝`about.hbs` か検体の隣）、台帳 `ledger/{assets,sakura-script}.toml`、`doc/COMPAT_ARCHITECTURE.md` §8。
- **規模**: 9〜12 タスク（棚卸㉔の 8〜11 に、上の申し送りの件＝作り直した顔を計測と描画へ揃える 1）。
- **先に要るもの**: 働きの依存は無い。`emo2_boot` の結線の列では先頭。文字とバルーンの列では `range-choice-tag` → `link-context-copy` → `balloon-link-hover` の後（列の決まり「重ならない席が見つかれば前へ」は残る）。
- **重なり**: `shell-scale-linkage` とは 0（あちらの `emo2_boot/` は `consumer_ledger*`・`mod.rs`・`runtime_boot.rs`。台帳は別の行）。同じウェーブに置けない相手＝`range-choice-tag`（`actor.rs`・`lib.rs` の席）・`choice-marker-styling`（`viewbox_draw_render.rs`）・`text-typesetting`・`talk-fast-forward`（`actor.rs`・`lib.rs` の席）・`emo-text-line-gap`（`draw_metrics.rs`）・`balloon-canon-residue`・`shell-companion-balloon`（`frame/switch.rs`）・`emo-bake-log-overlap`（`emo2_boot/assets.rs`）。並べられる相手＝`font-size-keywords`（重なり 0・本 spec は `look.rs` に触らない見込み）。
- **議題**（棚卸㉑・㉒から持ち越し）: ⑴ ファイルのフォント集を `build_actor_render` で毎回組むか、ランタイムに 1 つ持って共有するか ⑵ 試験用フォントのライセンスの置き場所 ⑶ 探し場所を `build_actor_render` の引数で渡すか、`ResolvedBalloonText` に載せるか（後者なら `actor_present.rs` に触れない）。基準のフォルダは `talk-script-metadata` へ乗り換えやすい口に留める（上の Upstream）。
- **優先度の区分**: A（開発者の 10-01 の指示「シェル内にバルーン領域を持つゴーストを設計したい」・探す順も開発者が決めた）。
- **要件定義のモデル**: Opus（正典の文言ははっきりしていて、分かれ目は作りの話だけ）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**: 本文 Constraints の「`draw.rs` 761 行」は 846 行。roadmap の文字とバルーンの列の説明「`actor.rs`・`viewbox_draw_render.rs` が `anchor-tag-canon` と重なる」は相手が着地して用済み（棚卸㉔のまま）。

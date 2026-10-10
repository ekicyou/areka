# Brief: areka-P0-balloon-font-file

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
- **Downstream**: `text-ruby`（ルビのフォント）・`balloon-markers`（`number.font.name`・`sstpmessage.font.name`）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `text-align-shadow-canon`・`emo-text-canon-residue`（同じ `areka-emo-text`）。

## Constraints

- `draw.rs` 761 行＝足す量によっては新しいファイルで足す。1 ファイル 1,000 行。
- 試験用フォントの同梱はライセンスの確認を要件で決める（OFL など再配布可のものに限る）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M（7〜10 タスク）。**`shell-balloon` とは並走できない**（棚卸⑳で訂正＝`actor` 系・`actor_decoration.rs`・`emo2_boot/frame/attach.rs` を共有）。文字まわりの直列の列で `shell-balloon` の後。
- 合っていた点: `.ttf`／`.otf`／`.ttc` は警告を出して読み飛ばしている（`draw_catalog.rs`）・`create_text_format` はフォント集に `None` を渡している（`draw.rs`）・DirectWrite の機能は既に有効＝`Cargo` の変更は要らない。
- **抜け**: ⑴ 計測用の `CreateTextFormat` がもう 1 か所ある（`draw_metrics.rs`・書式からフォント集を読み直す所も）。⑵ `FontCatalog` はスコープごとに `actor_decoration.rs` の `build_actor_render` で作られ、`BalloonModel` はフォルダの場所を持たない＝探すフォルダの一覧を `emo2_boot`（`frame/attach.rs` か切替の流れ）から emo-text の実行時の状態へ運ぶ必要がある。⑶ **`THIRD-PARTY-NOTICES.md` は生成物**（`tools/test-all.ps1` が作り直す）＝試験用フォントのライセンスを手で書くと消える。検体の隣に置くか `about.hbs` で扱う。⑷ `communicatebox.*` はどこにも実装が無い＝「同じ読み手を通す」は読み手を公開するところまで。⑸ `viewbox_draw` 系も `create_text_format` を 2 か所で呼ぶ。
- **議題**: フォルダの一覧をランタイムごとに渡すかスコープごとに渡すか／ライセンスの置き場所。

## 2026-10-03 ウェーブ C3-②（予定・10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」））

- 段は「優先」。C3 は C2 の着地で brief が動くので、着手の前に同じウェーブの他の spec と触るファイルを照合し直す（`roadmap.md`「ウェーブ編成」の C3 の行）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（8〜11 タスク）。切らない。
- 前提の状態: `emo-text-file-split`（✅ PR#217）・`shell-balloon`（✅ PR#227）とも着地済み＝前提は満たす。
- 崩れた前提／古くなった位置:
  - **シェル内バルーンの探し場所の口は `shell-balloon` が作った**: 純関数 `emo2_boot/shell_box_assets.rs` の `box_font_search_dirs`（シェルのフォルダ → ゴーストのフォルダ）が順を決め、`ShellBoxAssets::hand_to` → `TextLayerRuntime::set_box_layout` で文字の層へ渡り、`actor_box.rs` の `box_font_dirs()` で読める（欄は `actor.rs` の `TextLayerRuntime::box_font_dirs`＝**ランタイムに 1 つ**）。組み立ては `emo2_boot/assets.rs`（`font_dirs: box_font_search_dirs(…)` の行）。本 spec はこれを読むだけでよい。
  - **普通のバルーンの探し場所（バルーンのフォルダ → ゴーストのフォルダ）を運ぶ口はまだ無い**。今文字の層へ渡っているのは警告用のフォルダ名だけ（`actor_decoration.rs` の `set_balloon_label`・呼び手は `frame/attach.rs` の装着と `frame/switch.rs` のバルーン切替）。同じ 2 か所の隣に「探し場所」を渡す呼び出しを足すのが素直＝`frame/attach.rs`・`frame/switch.rs` を触る。
  - 棚卸⑳の議題「ランタイムごとかスコープごとか」は、箱の側が「ランタイムに 1 つ」で決まった。普通のバルーンもゴーストに 1 つのバルーンなので同じ形で足りる（スコープごとに違うバルーンを持つ経路は今無い）。
  - 分割での移り先: 書式を作る `create_text_format` の呼び手は `draw.rs`（`DrawExecutor`・照合用）・`draw_metrics.rs`（`DWriteMetrics::new_shared`）・**`viewbox_draw_render.rs` の `ensure_format`**（本番の描画・分割前は `viewbox_draw.rs`）の 3 か所。`FontCatalog` を作るのは `actor_decoration.rs` の `build_actor_render`（actor ごと・箱も同じ関数）と `viewbox_draw.rs` の `ViewboxExecutor::new`（照合用）と `draw_metrics.rs` の `DWriteMetrics::new`。
  - 箱も普通のバルーンも `ResolvedBalloonText::resolve_with_background` と `build_actor_render` を通る＝読み込みの仕組みは 1 つで両方に効く。違うのは探し場所の一覧だけ（箱＝`box_font_dirs`・普通＝新しい欄）。
  - `.ttf`／`.otf`／`.ttc` を警告して読み飛ばす所は `draw_catalog.rs` の `FONT_FILE_EXTENSIONS` と `FontCatalog::pick` のまま。`THIRD-PARTY-NOTICES.md` が `cargo about generate` の生成物（`tools/test-all.ps1`）なのも変わらない。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/draw_catalog.rs`（フォント集の組み立て・ファイルの読み込み）
  - `crates/areka-emo-text/src/draw.rs`（`create_text_format` にフォント集を渡す）・`draw_metrics.rs`・`viewbox_draw_render.rs`（`ensure_format`）・`viewbox_draw.rs`（`ViewboxExecutor::new`）
  - `crates/areka-emo-text/src/actor_decoration.rs`（`build_actor_render`・探し場所の入れ口）・`actor.rs`（`TextLayerRuntime` に普通のバルーンの探し場所の欄）・`actor_box.rs`（箱の `font.name` に `box_font_dirs` を渡す所）
  - `crates/areka-emo-text/src/look.rs`（`\f[name,…]` の候補の解決が同じ読み手を通るか）
  - 新規ファイル（フォント集の寿命を `draw_catalog.rs` の外に置くなら）＋`crates/areka-emo-text/src/lib.rs`（新しいファイルの登録の一覧 `PURE_SOURCES`／`SOURCES_OUTSIDE_THE_PURE_SCAN`）
  - `crates/areka/src/emo2_boot/frame/attach.rs`・`frame/switch.rs`（普通のバルーンの探し場所を渡す）・`emo2_boot/assets.rs`（バルーンのフォルダの場所の取り出し）
  - 試験用フォントの検体（新規・置き場所は要件で決める）・`about.hbs` か検体の隣のライセンス文・`doc/ukadoc-coverage/ledger/{assets,sakura-script}.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）:
  1. `FontCatalog` を actor ごとに作り直している（`build_actor_render`）。ファイルのフォント集をここで毎回組むか、ランタイムに 1 つ持って共有するか（共有なら切替での作り直しの時機を決める）。
  2. 試験用フォントのライセンスの置き場所（`THIRD-PARTY-NOTICES.md` は生成物で手書きが消える）。
- 見つけた穴: なし（実害のあるものは見つからなかった）。
- 並走の判定（厳しめ）: `shell-balloon-frame-align` とは `actor.rs`・`actor_box.rs`（と新しいファイルを足すなら `lib.rs`）が重なる＝**並べない**。`balloon-canon-residue` とは `frame/attach.rs`・`frame/switch.rs` が重なる＝**並べない**。本 spec は文字の列で単独で走らせる。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（8〜11 タスク）。切らない。
- 前提の状態: `shell-balloon-frame-align`（PR#236）が着地＝前提はすべて満たす。今すぐ着手できる。
- 崩れた前提／古くなった位置:
  - C3 の `shell-balloon-frame-align` は `actor.rs`（`shown_boxes` の欄の注記・`apply_cue` の末尾の注記）と `actor_box.rs`（`sync_boxes` が絵の番号を受け取る形・`balloon_shown_glyphs` の引数・`box_still_shown`）を書き換えた。本 spec が使う箱の探し場所の欄（`actor.rs` の `TextLayerRuntime` の `box_font_dirs`）・入れ口（`actor_box.rs` の `set_box_layout`）・読み口（同 `box_font_dirs`）は変わっていない。
  - **箱の探し場所はまだ本番でどこからも読まれていない**（読み口の呼び手は 0）。㉑の「`actor_box.rs`（箱の `font.name` に `box_font_dirs` を渡す所）」はまだ無い＝本 spec が最初の読み手を作る。
  - フォント集を作る 3 か所・書式を作る 3 か所は㉑のまま（C3 は `draw.rs` の注記の「M2 予約」→「予約」だけ）。`.ttf` などを読み飛ばす所も `draw_catalog.rs` の `FONT_FILE_EXTENSIONS` と `FontCatalog::pick` のまま。`frame/attach.rs`・`frame/switch.rs`・`emo2_boot/assets.rs` は C3 で変わっていない。
  - **㉑の触るファイルの抜け**: `build_actor_render`（`actor_decoration.rs`）の呼び手はただ 1 つ、`actor_present.rs` の `present_frame` の装着の枝（`TextSurface::attach` の直後）。探し場所を引数で足すとここを触る。探し場所（か読み込んだフォント集）を `ResolvedBalloonText`（`actor.rs` の定義・普通のバルーンは `actor_attach.rs` の `register_actor`、箱は `actor_box.rs` の `register_box` で組む）に載せれば、すでに渡っている `&resolved.font` を通って届き、`actor_present.rs` を触らずに済む。列の約束（新しい値はバルーン定義ごと）にも合う。
  - `lib.rs` は src の全部の `.rs`（テストの兄弟ファイルも）を `PURE_SOURCES` か `SOURCES_OUTSIDE_THE_PURE_SCAN` に載せないと赤（純粋の数は 71 で固定）。新しいファイルを 1 つでも足すと `lib.rs` を触る。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{draw_catalog.rs, draw.rs, draw_metrics.rs, viewbox_draw.rs, viewbox_draw_render.rs, actor_decoration.rs, actor.rs, actor_attach.rs, actor_box.rs}`・`lib.rs`（新しいファイルを足すとき）・`actor_present.rs`（探し場所を引数で渡す設計のときだけ）
  - `look.rs` は `\f[name,…]` を名前の列で運ぶだけで、解決は描画の `FontCatalog::pick`＝触らない見込み
  - `crates/areka/src/emo2_boot/{frame/attach.rs, frame/switch.rs, assets.rs}`
  - 試験用フォントの検体・ライセンス文（`about.hbs` か検体の隣）・`doc/ukadoc-coverage/ledger/assets.toml`（`font.name` の行）・`sakura-script.toml`（`\f[name]` の行）・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: ㉑の 2 件のまま。加えて ⑶ 探し場所を `build_actor_render` の引数で渡すか、`ResolvedBalloonText` に載せるか（後者なら `actor_present.rs` に触れず、`budoux-reveal-reflow` と重なるファイルが 1 つ減る）。
- 見つけた穴: なし。軽微: ㉑の触るファイルにある「箱の `font.name` に `box_font_dirs` を渡す所」は実在しない（上記）。
- 並走の判定（厳しめ）: `balloon-lifecycle-events` とは重なり 0＝**並べられる**。`budoux-reveal-reflow` とは `actor_present.rs`（引数で渡す設計のとき）と `lib.rs`（両方が新しいファイルを足すとき）が重なる＝既定では**並べない**（本 spec が `actor_present.rs` を触らず、新しいファイルを足すのが片方だけなら 0 にできる）。`balloon-canon-residue` とは `frame/attach.rs`・`frame/switch.rs`、`anchor-tag-canon` とは `actor.rs`・`viewbox_draw_render.rs`、`text-typesetting` とは `draw.rs`・`draw_metrics.rs`・`viewbox_draw_render.rs`・`actor.rs` が重なる＝並べない。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `budoux-reveal-reflow` が `crates/areka-emo-text/src/` の `actor.rs`（647 行）・`actor_attach.rs`・`actor_box.rs`・`actor_present.rs`・`lib.rs`（純粋な一覧は 73 本）を書き換えた。本 spec が使う所は変わっていない: 箱の探し場所の欄と入れ口と読み口、フォント集を作る 3 か所、書式を作る 3 か所、`.ttf` などを読み飛ばす所（`draw_catalog.rs`）。箱の探し場所の読み口の呼び手は今も 0。
  - `ghost-standard-balloon` で、普通のバルーンは「ゴーストの descript の指定 → 同梱の最初の 1 つ」で決まるようになった（`crates/areka/src/boot_resolve.rs`）。文字の層へ渡す場所（`crates/areka/src/emo2_boot/frame/attach.rs` と `frame/switch.rs` の、フォルダの名前を渡す呼び出しの隣）は変わっていない。
  - `self-alpha-declaration` が `crates/areka/src/emo2_boot/assets.rs` を少し書き換えた（透過の宣言の受け渡し）。本 spec の読む所とは別。
- **触るファイル**: 棚卸㉒の一覧のまま。今の行数は `draw.rs` 761・`draw_catalog.rs` 208・`draw_metrics.rs` 413・`viewbox_draw_render.rs` 571・`actor.rs` 647。検査のファイルを足す＝emo-text の `lib.rs` の一覧の席を使う。
- **規模**: M（8〜11 タスク）。
- **先に要るもの**: 働きの依存は無い。触るファイルが重なる相手＝`anchor-tag-canon`（`actor.rs`・`viewbox_draw_render.rs`）・`choice-marker-styling`（`viewbox_draw_render.rs`）・`balloon-canon-residue`・`shell-companion-balloon`（`frame/switch.rs`）・`text-typesetting`。
- **優先度の区分**: A（開発者の 10-01 の指示「シェル内にバルーン領域を持つゴーストを設計したい」。参考のゴーストは同梱フォントの縦書きが前提で、探す順も開発者が決めた）。
- **要件定義のモデル**: Opus（正典の文言ははっきりしていて、分かれ目は作りの話だけ）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - 穴は無い。棚卸㉒の「`budoux-reveal-reflow` と重なる」は相手が着地して消えた。
  - `anchor-tag-canon` とは同じウェーブに置けない。リンクの列は 4 本続くので、列の決まり（重ならない席があれば前へ）に従うなら、`anchor-tag-canon` の前のウェーブに出すか、リンクの 4 本の後かを優先度で決める。前に出す場合の相手は `reflow-scroll-path-test`・`emo2-real-run-wrap-timeout`・可視性の側だけで直す `choice-balloon-timeout-stuck`（重なり 0）。

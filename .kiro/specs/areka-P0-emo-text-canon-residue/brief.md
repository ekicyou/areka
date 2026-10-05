# Brief: areka-P0-emo-text-canon-residue

> 起票: 2026-09-11（棚卸⑬・`areka-P0-balloon-canon-residue` の 3 軸分割 ⑶＝emo-text 帰属の項目 11・12・14・15 を独立 spec に切り出し）。項目本文の正本は分割元 brief（11・12 は「`balloon-vertical-canon` からの追加登記」節、14・15 は「`emo-text-line-height-canon` からの追加登記」節）。ここでは所有と着地条件だけを書く。

## 2026-09-20 棚卸⑮の再測定

**実測（main `fe157df1`）**

- 本文が「先に着地させる」と書く上流は両方とも完了した（`areka-P0-text-decoration-canon`・`areka-P0-balloon-font-descript-keys`）。`draw.rs` は 988 → 750 行、`actor_decoration.rs` が分かれた。**同居できない待ちは解けている。**
- **2026-09-24: 項目 12 は直接修正で直した**（警告の文言を「指定なしとして扱う」へ・文言を固定するテストを足した。既存の `unknown_value_falls_back_to_horizontal_tb_with_warn` は件数しか見ておらず逐語固定ではなかった）。本仕様の範囲は 11・14・15 の 3 項目。以下は当時の記述。
- **項目 12（未知の `writing_mode` の警告の文言が実際の挙動と食い違う）は、spec を立てずに直す**（roadmap「直接修正候補」）。`crates/areka-emo-text/src/writing.rs` の 1 ファイルで閉じ、文言と、逐語で固定しているテスト `unknown_value_falls_back_to_horizontal_tb_with_warn` を同時に直す。他の 3 項目と共有するファイルは 0。直ったら本仕様から外す。
- 残る 11・14・15 はテストの穴と正典の追加で、α にもバグ修正にも属さないので当面着手しない。
- `layout.rs` 973・`actor.rs` 975・`region.rs` 977 行＝1,000 行の上限の直前。足すときは新規ファイルで。

## Problem

完了 spec `balloon-vertical-canon`（bvc）と `emo-text-line-height-canon` が emo-text 側に残した 4 件が所有者不在のまま `balloon-canon-residue` の台帳に同居していた。系列解決（emo-present）とも表示寿命（kanade）とも軸が違う:

11. 縦書き字形（グリフ直立・縦書き用字形）の**観測点が repo に無い**（字形が退行しても全緑）。
12. `writing_mode` 未知値の警告文言が実挙動とずれている（**現行 main の唯一の実バグ**・文言のみ・`crates/areka-emo-text/src/writing.rs`・逐語固定のインラインテストと同時修正）。
14. 折返し基準が描画範囲の外に解決されるバルーン定義の扱い＋警告の `balloon` 欄がバルーン名でなく定数 `BALLOON_NAME_PLACEHOLDER`（`region.rs` で定義・`actor.rs` が import＝参照 2 ファイル）。`BalloonModel` に `name,` の取得口が無い（`impl Font` の `name` は別物）。
15. 行末禁則文字のぶら下がり（折返しの遅延）が未実装。二段構え（soft `wrap_threshold`／hard `inline_limit`）の足場は `layout.rs` にある。

## Current State

2026-09-11 実測（サブエージェント再測定）: 12 の文言と逐語固定テスト `unknown_value_falls_back_to_horizontal_tb_with_warn` が現存・実挙動は `WritingModeDecl::Unknown → None`（「指定なし」として合流・モジュール doc DD6 と一致）。14 の取得口は `impl BalloonModel` に無い。15 は `LayoutEngine::visible_window` の前提（行が行送り方向へ単調）のまま。`crates/areka-emo-text/src/` は `draw.rs` 988・`layout.rs` 955・`actor.rs` 952・`region.rs` 951 行＝**1,000 行番人の射程**（例外表に不在・新規ファイルで足すこと）。

## Desired Outcome

12 の文言が実挙動と一致し、14 でバルーン名が警告に出て `ledger/assets.toml` の `owner` が埋まり、15 で「」」等の禁則文字が折返し基準を超えてぶら下がり描画範囲は超えない、11 で縦書き字形の退行が赤になる観測檻がある（SSP との一致ではなく areka 内で閉じた反証可能な述語）。

## Approach

12 → 1 手（文言＋テスト）。14 → `crates/areka-parsers/src/balloon/parse.rs` の `map_merged` が `name,` を写し `BalloonModel` に取得口・`region.rs` の定数を差し替え。15 → 禁則文字集合の定義＋行末での折返し遅延の規則（既存 2 値の間で判定）。11 → `draw_readback` 系の読み戻しで字形差を述語化（AI vision 目視は補助）。

## Scope

- **In**: 項目 11・12・14・15。
- **Out**: 系列解決（分割元 ⑴）・表示寿命（`balloon-lifecycle-events`）・`\f` 寄せ（`text-align-shadow-canon`・`layout.rs` を共有＝**同居不可**）。

## Boundary Candidates

- 12 は S・前提なし・直接修正でも可（本 spec の着手前に直せば本 spec から外す）。
- 14・15 は「バルーン定義と折返しの正典」の 1 軸、11 は観測檻＝独立。

## Out of Boundary

- 折返し基準／描画範囲の意味論の再裁定（`emo-text-line-height-canon` で確定・不変）。

## Upstream / Downstream

- **Upstream**: 完了 bvc・完了 `emo-text-line-height-canon`。`text-decoration-canon` ⑴（`draw.rs` 分割）の**後**に着手（`layout.rs`／`region.rs` を触るため W13 と同居不可）。
- **Downstream**: `text-align-shadow-canon`（15 の禁則と寄せが同じ行配置の前提を共有）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-balloon-canon-residue`（分割元・11・12・14・15 を引き継ぐ）。
- **Adjacent**: `balloon-font-descript-keys`（`balloon/parse.rs`・`model.rs` を共有＝**同居不可**・先に着地させる）・`text-decoration-canon`（`region.rs`）。

## Constraints

- 編集集合: `crates/areka-emo-text/src/{writing,region,layout}.rs`（＋兄弟テスト）・`crates/areka-parsers/src/balloon/{parse,model}.rs`・`doc/ukadoc-coverage/ledger/assets.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 項目 14 の定数 `BALLOON_NAME_PLACEHOLDER` の参照は 2026-09-19 時点で **3 ファイル**（`region.rs` が定義・`actor.rs` と `actor_decoration.rs` が import）・**警告 2 種類**（折返し基準の粗さ `warn_coarse_wrap_threshold`／`origin` が描画範囲の外 `warn_ignored_origin`）へ増えている（`areka-P0-balloon-origin-outside-validrect`）——バルーン名へ差し替えるときは上の編集集合に `actor.rs`・`actor_decoration.rs` を足すこと。
- fixture は無改変（当時の `crates/pilot/examples/shiori-host-32/fixtures/emo2/`。2026-09-19 の `nar-install` で `vendors/sample_ghost/emo2.nar` へ畳まれ、追跡から外れている）。
- 規模 M・要件定義は Opus で足りる（裁定は 15 の禁則文字集合の出典 1 件）。



## `balloon-color-emoji` からの申し送り（2026-09-30 着地）

- バルーンの文字の単位は `char` から書記素クラスタ（人が 1 文字と見る単位）に替わった。範囲（`ChoiceSpan::glyph_range`・`style_runs`・`segment_text_range`）はクラスタの通し番号で数え、UTF-16 の位置はクラスタ文字列の長さを積む。
- `TextItem::Glyph` と `PositionedGlyph` の中身は `text: Arc<str>`（`Copy` なし）。構築は `TextItem::glyph(&str)`、切り方は `areka_sakura::cluster::clusters` だけが決める。文字を比べる処理（行末のぶら下げの判定など）は `&str` で比べる。

---

## 2026-10-01 `/kiro-discovery`（シェル内バルーン）による引き取り

- **項目 11（縦書きの字形の観測点）と項目 15（行末のぶら下げ）は新 spec `areka-P0-text-typesetting` が引き取った**。同 spec は禁則（バルーンのキー `line_break`・既定 `anywhere`）・ぶら下げ（`hanging_punctuation,allow-end`）・縦中横（`\![text,combine-upright,…]`・`text_combine_upright,digits 2`）・字の向き（`text_orientation`）を持つ。areka 独自のキーの名前は CSS のプロパティ名の `-` を `_` にしたもので揃える（開発者確定）。ぶら下げは本 brief の項目 15 の意味論（折り返し基準を超えてよい・絶対上限は超えない）のまま実装する。
- **本 spec に残るのは項目 12・14**（12 は 2026-09-24 に消化済みなので実質 14 のみ＝折り返し基準が描画範囲の外に解決されるバルーン定義の警告と、警告のバルーン名の欄）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **取り下げ予定（棚卸⑳）・着手しない**。残っていたのは項目 14（折り返しの警告にバルーンの名前を出す）の 1 件だけになった＝項目 12 は 09-24 に直接修正・項目 11 と 15 は `text-typesetting` へ移した。項目 14 は、名前を持つバルーンを入れて同じファイルを触る `areka-P0-shell-balloon` が引き取る（同 brief に記載）。
- 本フォルダは置いたままにする。消す・移すと網羅台帳の整合検査（`doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` の表が本 spec の名前を持つ）が赤くなる。表から外すのと一緒に `areka-P0-coverage-roadmap-refresh` が片付ける。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: XS（0 タスク＝作業は残っていない）。取り下げ予定のまま・着手しない。
- 前提の状態: 最後の 1 件（項目 14）の引き取り手 `shell-balloon` が着地した（PR#227）。
- 崩れた前提／古くなった位置:
  - **項目 14 は消化済み**: 定数 `BALLOON_NAME_PLACEHOLDER` はソースから消えた（0 件）。折り返しの警告（`actor_attach.rs` の `warn_coarse_wrap_threshold`）と範囲外の `origin` の警告（`actor_decoration.rs` の `warn_ignored_origin`）の `balloon` 欄は、呼び手が渡す名前になった——普通のバルーンは `frame/attach.rs`（装着）と `frame/switch.rs`（バルーンの切替）が `set_balloon_label` で入れる**フォルダの名前**（無ければ `スコープ{番号}のバルーン`）、箱は `balloon.名前`ブレスの名前（`shell-balloon` 要件 3.11・3.12）。
  - 本 brief が書いていた「descript の `name,` を写す」形とは違う（フォルダの名前を使った）。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `name,`（バルーン名）の行は `status = "implemented"`・`owner = "areka-P0-baseware-root-layout"` で、空の担当は残っていない。
  - 項目 14 の前半（粗いバルーン定義を確かめる道具を持つか）は判断だけで作業ではない＝持たない（警告で知らせる今の形）のままで残件にしない。
- 触るファイル（並走の照合用）: なし。
- 議題: なし。
- 見つけた穴: なし。
- 片付け: フォルダの削除は今も `doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` の表（本 spec の名前を持つ）の整合検査を赤くするので、`coverage-roadmap-refresh` が表から外すのと一緒に行う（棚卸⑳のまま）。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: XS（0 タスク）のまま。取り下げ予定・着手しない（据え置き）。
- 前提の状態: 残件なし。片付けの担い手 `coverage-roadmap-refresh` は今も未着手（フォルダ `.kiro/specs/areka-P0-coverage-roadmap-refresh/` は在る）。
- 崩れた前提／古くなった位置: なし。定数 `BALLOON_NAME_PLACEHOLDER` は `crates/` 全体で 0 件のまま。`doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` の表には `name = "areka-P0-emo-text-canon-residue"` の行が今も 1 つある＝フォルダを消すと整合検査が赤くなる事情は変わらない。
- 触るファイル（並走の照合用）: なし。
- 議題: なし。
- 見つけた穴: なし。

# 裁定と検証の記録（signoff-record）

| 項目 | 値 |
|---|---|
| 対象仕様 | `areka-P0-default-balloon-bundle` |
| 本節（§1）の対象要件 | 1.1・1.2・1.3・1.4 |
| 実施日 | 2026-09-18 |
| 実施した作業木 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-balloon-bundle-6fe55e`（ブランチ `claude/areka-p0-balloon-bundle-6fe55e`） |
| §1 記入時の HEAD | `cef8f60d99e919333cf70fa7c208b44b96b162ec` |

この記録は「既定バルーンを `StayseeBalloon` に決めた判断と、その理由（採った理由・採らなかった理由・見た目をそのまま受け入れる理由）が、追加の会話なしに後から読んで分かる」ことを目的とする。取得元・ハッシュ・保管の検査といった**出典の事実**は隣の `provenance.md` が正本なので、ここでは繰り返さず参照する。

本文の値はすべて本ブランチで採り直した実測であり、要件書や先行タスクの報告を写したものではない。各値には**実際に打った命令**を添えた。命令はすべて作業木の根から打っている。

## この記録の埋まり具合

この記録は 8 節から成る。本タスク（1.4）が埋めたのは **§1 だけ**である。§2〜§8 は見出しだけを置いてあり、それぞれの節に書いてある担当タスクが後で埋める。

---

## 0. 結論（先に）

| # | 主張 | 根拠の所在 | 判定 |
|---|---|---|---|
| 1 | 既定バルーンは `StayseeBalloon` に確定している（開発者裁定 2026-09-18） | §1.1 | 確定 |
| 2 | 再配布条件は `LICENSE` 本文と `readme.txt` の**両方**で確認できる | §1.2 ⑴ | 両方で確認 |
| 3 | 特定ゴースト専用の指定は無い | §1.2 ⑵ | 指定なし |
| 4 | 原作者の唯一の注意事項（半透明前提）は areka の固定の扱いと一致する | §1.2 ⑶ | 一致 |
| 5 | 他候補を採らない理由は書き残されている | §1.3 | 4 群すべて記載 |
| 6 | 文字は ukadoc 既定の書体で描かれ、それは正典どおりの挙動である | §1.4 | 正典どおり |
| 7 | 上の 6 を守るために areka 側の本番コードもバルーン側のファイルも変えない | §1.4 | 変更 0 |

---

## 1. 裁定と選定根拠（要件 1.1・1.2・1.3・1.4）

### 1.1 裁定（要件 1.2）

| 項目 | 内容 |
|---|---|
| 日付 | 2026-09-18 |
| 裁定者 | 開発者 |
| 結論 | 「ukadoc 準拠とし、StayseeBalloon をそのまま採用する」 |
| 対象資産 | `Balloon for Staysee Syncfield`（id `StayseeBalloon`・作者「ぽな（ばぐとら研究所/整備班）」・ライセンス CC0-1.0） |
| 根拠 | ⑴ 再配布条件が `LICENSE` 本文と `readme.txt` の両方で確認できる ⑵ 特定ゴースト専用の指定が無い ⑶ 原作者が明記した前提（半透明）が areka の固定の扱いと一致する |

この裁定により、既定バルーンの候補選びはここで閉じる。以降（要件 2 以降）は `StayseeBalloon` を前提に進め、実機での見た目の確認（要件 4）は**採否を覆すための関門ではなく品質確認**として行う。見た目に崩れが出たときの行き先は要件 3.9／4.3（バルーンを改変して合わせるのではなく areka 側の欠陥として扱う）であり、§4 に記録する。

裁定に至るまでの議論そのものは `brief.md` の「議論の記録（2026-09-18・開発者と Claudia）」に残っている。本節はその結論と、結論を支える事実の裏取りである。

関連する別の開発者裁定（同日・本仕様の前提として効いている）:

- 「**areka は常に `use_self_alpha,1`。pna 対応は不要**」——§1.2 ⑶ の一致判定の前提。
- 「**同梱バルーンは癖が無いものが必要。`emo2-kakukaku` では厳しい**」——§1.3 の不採用理由。

### 1.2 候補を選んだ根拠（要件 1.1）

3 点とも保管済みの実物（`vendors/sample_ghost/StayseeBalloon/`）と areka の実コードで裏を取った。

#### ⑴ 再配布条件が `LICENSE` 本文と `readme.txt` の両方で確認できる

| 出所 | 読んだもの | 命令 |
|---|---|---|
| `LICENSE`（ASCII・復号不要） | 1 行目 `Creative Commons Legal Code`・3 行目 `CC0 1.0 Universal` | `head -6 vendors/sample_ghost/StayseeBalloon/LICENSE` |
| `readme.txt`（Shift_JIS＝CP932） | 見出し「■転載・再配布・同梱・改変等について」、本文「煮るなり焼くなり好きにしてください。」、次行「License : CC0 https://creativecommons.org/publicdomain/zero/1.0/deed.ja」 | `iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/readme.txt` |

**両方で確認できることを根拠に据えた理由**: 法文（`LICENSE`）だけなら「作者が意図して置いたのか、雛形が紛れ込んだのか」が読み取れない。作者の言葉（`readme.txt`）だけなら「CC0」が正確な意味で使われているか分からない。二つが同じ結論を指しているので、再配布・同梱の可否は推測ではなく書かれた事実として扱える。同梱して配る側（areka）にとってはこの一致が最も重い。

なお CC0 は帰属表示を義務づけないが、出典と作者名は礼儀として記録し配布物にも残す（要件 5・`provenance.md` §1 と本記録 §6）。

#### ⑵ 特定ゴースト専用の指定が無い

`readme.txt`（復号後）に次の 2 行がある。

- 「特定のゴーストを意識して作ったバルーンですが、専用指定はしていません。」
- 「デザイン的には極端な特化はしていないので、なにか流用できるかもしれません。」

既定バルーンは「どのゴーストが入ってきても、まずこれで喋れる」ための資産なので、特定のゴースト向けという但し書きが付いていないことが要る。原作者自身が流用を想定していると書いているので、用途の面でも借り物にならない。

#### ⑶ 半透明前提が areka の固定の扱いと一致する

原作者が挙げた注意事項は 1 つだけである（`readme.txt` 復号後）。

- 「■！！！注意事項！！！」「半透明（アルファチャンネル）ONを前提に作っており、OFFではまともに見られません。」

areka はこの設定を**バルーンの宣言から読まない**。常に画像のアルファをそのまま尊重する側に固定してある。実在を次の 4 か所で確かめた。

| # | 場所（何の定義か） | 何が書いてあるか |
|---|---|---|
| a | `crates/areka-emo-present/src/balloon.rs` のモジュール doc（「正典整理」の「PNG α 尊重」の項） | 「`use_self_alpha,1` 相当＝`UseSelfAlpha::On` で bake する」「`.pna` 対応は `ElementDecoder::probe_pna` の既存 seam に委ね本 spec では追加しない」 |
| b | 同ファイルの `build_balloon_target_from_faces` の中で `SurfaceSet` を組む箇所 | `alpha_params` に `use_self_alpha: UseSelfAlpha::On` を**定数で**渡す（バルーンの宣言値を参照する引数は無い） |
| c | `crates/areka/src/emo2_boot/assets.rs` の `build_boot_assets`（起動時の焼き込み） | シェル側の `SurfaceSet` にも同じ `use_self_alpha: UseSelfAlpha::On` を定数で渡す。バルーン側はこの関数から `build_balloon_target_from_faces` へ委ねるので、経路のどちらを通っても固定値になる |
| d | `crates/areka-parsers/src/balloon/parse.rs`（バルーン定義の読み手） | `use_self_alpha`／`use_input_alpha`／`paint_transparent_region_black` の 3 つの鍵を**引かない** |

d の「引かない」は 0 件の主張なので、数え方と較正を添える。

```
# 判定: バルーン定義の読み手が 3 鍵に触れているか
grep -n "use_self_alpha\|use_input_alpha\|paint_transparent_region_black" \
     crates/areka-parsers/src/balloon/parse.rs | wc -l
# → 0

# 判定: crates/ 全体で 3 鍵を「設定ファイルの鍵の綴り」として引いている箇所
grep -rn "\"use_self_alpha\"\|\"use_input_alpha\"\|\"paint_transparent_region_black\"" \
     crates/ --include=*.rs | wc -l
# → 0

# 較正: 同じ数え方で、読み手が実際に引いている鍵を数える（0 でないことを確かめる）
grep -on "\"validrect\.\|\"wordwrappoint\|\"font\.\|\"vertical\"\|\"origin\." \
     crates/areka-parsers/src/balloon/parse.rs
# → origin. / wordwrappoint / validrect. / font. などが多数ヒット（0 ではない）
```

較正の意味: 同じファイルに同じやり方を当てて `origin.`・`wordwrappoint`・`validrect.`・`font.` は拾えている。したがって上の 0 件は「探し方が壊れていて何も拾えなかった」のではなく「本当に引いていない」である。

補足として、`crates/` を `use_self_alpha` の語だけで検索すると 60 行が当たる（`grep -rn "use_self_alpha" crates/ --include=*.rs | wc -l`）。その内訳は ⑴ Rust 側の構造体 `AlphaParams` の項目名（`use_self_alpha: UseSelfAlpha::On`）とその説明、⑵ テストが組み立てるシェル側の定義テキストに現れる `seriko.use_self_alpha,1` の行、の 2 種であり、**バルーンの `descript.txt` の鍵として引いている箇所は 1 つも無い**。それが上の 2 本目の命令の 0 件である。

**一致と言える理由**: areka は宣言に関わらず常にアルファを尊重する。つまり「半透明 ON でないと正しく見られない」バルーンは areka では**必ず**その前提で扱われる。逆に「半透明 OFF 前提」で作られたバルーンは areka では救えない。StayseeBalloon は前者なので、areka の固定の扱いとぶつからない側の資産である。この固定は areka の意図した裁量なので、要件 6 で `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳に登記する。

なお StayseeBalloon 自身も `descript.txt` に `use_self_alpha,1` と書いている（`iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt` の `use_self_alpha` 行）。areka はこの行を読まないが、読む・読まないに関わらず結果は同じになる。

### 1.3 他の候補を採らない理由（要件 1.4）

| 候補 | 採らない理由 | 理由の出所 |
|---|---|---|
| SSP 同梱の「SSPデフォルト+」「balloon for Emily/P4」 | **再配布条件が公開されていない**。SSP 公式サイト・SSP ヘルプの FAQ・ukadoc のいずれにも記載が無く、SSP 本体のソースも GitHub には無い（`github.com/ponapalt/ssp` は 404）。条件を知るには SSP の配布 zip に入っている readme を読むか作者に直接尋ねるほかなく、同梱して配る根拠としては薄い。 | `brief.md`「議論の記録」2 の第 1 項（2026-09-18 の調査） |
| `emo2-kakukaku` と派生 2 つ（`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`） | **開発者裁定「同梱バルーンは癖が無いものが必要。`emo2-kakukaku` では厳しい」**（2026-09-18）。これらは作者自作の検証用資産であり、特定の観測を固定するために形を尖らせてある。検証用としては今までどおり使い続けるが、第三者が最初に目にする既定にはしない。 | `brief.md`「議論の記録」1（開発者発言） |
| 整備班（`ms.shillest.net`）の他のバルーン 6 つ | 配布条件が配布ページに書かれていない。上の 1 件目と同じ理由で借用の根拠が立たない。 | `brief.md`「議論の記録」2 の第 3 項 |
| 自作の無地バルーン | 採らないのではなく**次善として残す**。CC0 の候補が見た目の理由で不採用になった場合の逃げ道だが、画像を描く手間と `descript.txt` の調整が要り、既製品より高くつく。§1.1 の裁定で CC0 候補が採用されたので、今回は使わない。 | `brief.md`「議論の記録」2 の第 4 項・要件書 Introduction「候補（2026-09-18 調査）」 |

不採用にした `emo2-kakukaku` 系 3 つが実在し、今も検証用として置かれたままであることを確かめた（本仕様はこれらの中身も置き場所も変えない＝要件 2.7）。

```
find . -type d -name "emo2-kakukaku*" -not -path "./.git/*" | sort
# → ./crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-offsetdpi
#   ./crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-wplimit
#   ./crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku
```

### 1.4 見た目は ukadoc 既定の書体のまま受け入れる（要件 1.3）

#### 事実: StayseeBalloon は書体名を宣言していない

```
# 判定（行頭一致）
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | grep -c "^font\.name"
# → 0（grep -c は 0 件のとき終了コード 1 を返すので、この 1 は失敗ではない）

# 判定（行頭に限らず、文字列としてどこかに現れるか）
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | grep -c "font\.name"
# → 0

# 較正: 同じ数え方で font. で始まる行を数える
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | grep -c "^font\."
# → 4（font.height,12 / font.color.r,0 / font.color.g,40 / font.color.b,100）

# 較正: 復号そのものが成立していること
iconv -f CP932 -t UTF-8 vendors/sample_ghost/StayseeBalloon/descript.txt | wc -l
# → 69 行
```

較正の意味: 同じ命令で `font.` の行は 4 本拾えており、復号も 69 行成立している。したがって `font.name` の 0 件は「復号に失敗して空を数えた」のでも「綴りを間違えて探した」のでもなく、**本当に宣言が無い**。

あわせて、同じやり方で次の 2 つも 0 件だった（いずれも要件 3.3 が「未指定として扱う」と定めているもの）。

| 鍵 | 件数 | 未指定のときの areka の扱い |
|---|---|---|
| `font.name` | 0 件 | 既定の書体へ縮退する |
| `vertical` | 0 件 | 横書きとして扱う |
| `wordwrappoint` | 0 件 | 折返し基準は `validrect` の遠辺へ縮退する |

これら 3 つの 0 件はいずれも欠陥ではない。ukadoc が既定を定めている鍵であり、areka にも既定へ落ちる道が既に通っている。

#### areka 側の既定書体が実在すること

| 場所（何の定義か） | 内容 |
|---|---|
| `crates/areka-emo-text/src/draw.rs` の `DEFAULT_FONT_NAME` の定義 | `pub const DEFAULT_FONT_NAME: &str = "ＭＳ ゴシック";` |
| 同ファイルの `ResolvedFont` の解決手順の説明（`font.name` の縮退を述べた行） | 「`font.name` 欠落 → `DEFAULT_FONT_NAME`（ukadoc 既定＝正常系・ログなし）」 |
| 同ファイルのモジュール doc（既定値を列挙した行） | 「`DEFAULT_FONT_NAME`＝ＭＳ ゴシック／`DEFAULT_FONT_HEIGHT`＝12／色＝黒」 |

「ログなし」と明記されているとおり、書体名の欠落は areka では**正常系**である。記録を出さないのは異常を握り潰しているからではなく、ukadoc が定めた既定へ落ちるだけだからである。

#### 受け入れの結論

StayseeBalloon の本文は ukadoc 既定の書体（ＭＳ ゴシック）と、バルーン自身が宣言する `font.height,12` で描かれる。これは**正典どおりの挙動**なので、直すべきものは無い。したがって本仕様は次のどちらもしない。

| しないこと | 理由 | 守られる要件 |
|---|---|---|
| areka 側の既定書体の定数（`crates/areka-emo-text/src/draw.rs` の `DEFAULT_FONT_NAME`）を変えない | 変える理由が無い。変えれば ukadoc の既定から外れるうえ、本番コードの変更が発生する | 要件 8.1（本番コードの変更 0 行） |
| バルーン側の `descript.txt` に `font.name` を書き足さない | 書き足せば原作のバイト列が変わり、上流との同一性の検証（`provenance.md` §3 のハッシュ一覧）が成り立たなくなる | 要件 2.2（1 バイトも変えない） |

この 2 つを同時に守れるのは、「見た目を合わせるために何かを変える」必要が無いからである。ukadoc 既定の書体で描かれることは受け入れる結論であって、妥協ではない。

見た目が実際に崩れないことは決定論テスト（要件 3・§2）と実機目視（要件 4・§5）で確かめる。そこで崩れが出た場合も、バルーンを改変して合わせるのではなく areka 側の欠陥として扱う（要件 3.9／4.3・行き先は §4 に記録する）。

---

## 2. 決定論テスト（要件 3.1・3.3〜3.8・3.10・3.11）

本節の値はすべて本タスク（2.8）で採り直した実測であり、先行タスクの報告を写したものではない。命令はすべて作業木の根から打っている。記入時の HEAD は `0dd39231`（タスク 2.7 の着地）。測定を始める前の作業木は `git status --porcelain` が無出力（未コミットの変更なし）で、測定を終えた時点で出ているのは本ファイル 1 行だけである（下の較正で一時的に触ったファイルはいずれも元へ戻してある）。

### 2.1 何を固定したか（設計 C2 の Batch/Job Contract の A〜I 行）

新規テストは 1 本の入口 `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs`（検体パスの定数と接続宣言だけ）と、その下の `crates/areka-emo-text/tests/staysee_balloon_fixture/` に置いたテーマ別 9 ファイルから成る。設計の A〜I 行はテーマ別ファイルと 1 対 1 に対応し、9 行すべてに担当が居る（担当の無い行 0・担当の無いファイル 0）。

| 設計の行 | 固定する内容 | 担当ファイル | テスト名 | 本数 | 結果 |
|---|---|---|---|---|---|
| A | 保管フォルダ直下の名前集合 29 本と枠画像 6 枚の原寸 | `assets.rs` | `stored_folder_holds_exactly_the_upstream_29_files`・`frame_png_native_sizes_are_pinned` | 2 | 緑 |
| B | `descript.txt`／`install.txt` の復号と読み取り（宣言された値・未宣言のまま残る値） | `definition.rs` | `descript_identity_keys_are_read_as_declared`・`balloon_model_reads_declared_geometry_and_font`・`undeclared_keys_stay_unspecified_and_resolve_to_horizontal`・`install_directory_matches_descript_id` | 4 | 緑 |
| C | 面の系列解決（本体側・相方側）と、その間に出る記録 | `faces.rs` | `face_series_and_records_are_pinned_for_both_scopes` | 1 | 緑 |
| D | 枠画像の焼き込みと半透明の保持 | `bake.rs` | `baked_face0_keeps_partial_alpha_for_both_scopes` | 1 | 緑 |
| E・F | 文字の描画範囲の座標と、既定書体の受け入れ口（名前と文字寸法） | `region.rs` | `staysee_text_region_is_pinned_for_both_scopes`・`undeclared_wrap_threshold_degenerates_to_the_inline_limit`・`undeclared_font_name_resolves_to_the_canon_default`・`default_font_metrics_are_pinned_at_height_twelve`・`declared_balloon_returns_a_different_font_and_a_split_wrap_threshold` | 5 | 緑 |
| G | 折返しと描画範囲への内包（全角のみ・半角のみ・混在の 3 本文） | `wrapping.rs` | `each_body_wraps_into_the_pinned_line_lengths`・`wrapping_fills_each_line_greedily`・`every_glyph_stays_inside_the_drawing_range`・`placed_glyph_advances_match_the_half_and_full_width_values`・`the_kero_side_fits_exactly_five_lines_without_overflowing`・`one_pixel_narrower_bottom_turns_the_same_lines_into_an_overflow` | 6 | 緑 |
| H | 選択肢（`\q`）と遅延座標指定（`\_l`）の位置・目印の見た目 | `script.rs` | `the_cursor_tag_places_the_next_glyph_at_the_declared_offset`・`the_rows_after_the_cursor_tag_advance_by_the_canonical_line_pitch`・`the_script_records_both_choices_with_their_ids_and_labels`・`choice_glyphs_and_bands_stay_inside_the_drawing_range`・`the_choice_style_is_derived_from_the_cursor_declaration`・`choice_style_follows_the_cursor_style_declaration` | 6 | 緑 |
| I | 表示スケール k∈{1.0, 1.25, 2.0} の扱い | `scale.rs` | `binding_keeps_the_native_image_size_and_the_given_scale`・`physical_conversion_scales_with_the_display_scale`・`text_region_resolves_from_the_image_size_not_the_physical_surface`・`the_observed_scale_set_has_three_values_including_non_unit_ones`・`resolving_from_the_physical_surface_moves_the_far_edges` | 5 | 緑 |

合計 **30 本**。9 ファイル目の `test_support.rs` はテーマ間で共有する値とヘルパの置き場で、テストは持たない（共有項目を複製しないための集約先）。

```
cargo test -p areka-emo-text --test staysee_balloon_fixture_test
# → test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 2.2 較正で動いた期待値は **0 件**

設計 Risks は「導出値（送り幅・行ボックス・行送り・1 行の文字数）は実装で実測して較正し、動いた値と理由（書体の版など）をここへ書く」と定めている。**本仕様で動いた値は 1 つも無い。**

調べ方: 設計の Data Models（Domain Model）の表と C2 の A〜I 行の述語欄から、**数値で書かれている期待値を全部拾い**、新規テストが固定している値と 1 つずつ突き合わせた。拾った数値は 27 件で、内訳は「突き合わせられた 23 組」（下の表）と「テストが数値として固定していない 4 件」（その次の表）である。突き合わせた 23 組の食い違いは **0 組**。30 本すべてがこの値のまま緑になっているので、「テストの中の値」と「実際に走らせて出た値」も一致している。

数値でない項目（`charset,Shift_JIS`・`type,balloon`・`id`・`name` のような綴りそのもの）は書体の版などで動きようがないので較正の対象にならず、この突合には含めていない（それらは設計 B 行の担当テストが逐語で固定している）。

出所の欄は、その値がどこに書かれているかを示す。**1 件（#22）だけは設計ではなく検体の宣言が出所**なので、その旨を欄に明記した。

| # | 突き合わせた値 | 値の出所 | 新規テストが固定している値（何の定義行か） | 判定 |
|---|---|---|---|---|
| 1 | 枠画像 6 枚の原寸 (335,205)(335,205)(335,395)(335,395)(335,135)(335,135) | 設計 C2 の A 行 | `test_support.rs` の `EXPECTED_FRAME_SIZES` の 6 行 | 一致 |
| 2 | 直下のエントリ 29 本 | 設計 C2 の A 行 | `test_support.rs` の `EXPECTED_FILE_NAMES`（29 要素） | 一致 |
| 3 | `validrect` の宣言値 (22, 20, −26, −47) | 設計 C2 の B 行 | `definition.rs` が読み取りを固定している生値 | 一致 |
| 4 | 描画範囲の左辺 22 | 設計 Data Models・C2 の E 行 | `test_support.rs` の `EXPECTED_LEFT` ＝ 22.0 | 一致 |
| 5 | 描画範囲の上辺 20 | 設計 Data Models・C2 の E 行 | 同 `EXPECTED_TOP` ＝ 20.0 | 一致 |
| 6 | 描画範囲の右辺 309（＝335 − 26） | 設計 Data Models・C2 の E 行 | 同 `EXPECTED_RIGHT` ＝ 309.0 | 一致 |
| 7 | 描画範囲の下辺 本体側 158・相方側 88 | 設計 Data Models・C2 の E 行 | 同 `EXPECTED_BOTTOM` ＝ [(0, 158.0), (1, 88.0)] | 一致 |
| 8 | 描画開始点 (22, 20) | 設計 Data Models・C2 の E 行 | `region.rs` の `EXPECTED_START` | 一致 |
| 9 | 折返し基準・遠辺とも 309 | 設計 Data Models・C2 の E 行 | `region.rs` の折返し基準と遠辺の突合 | 一致 |
| 10 | 文字の高さ 12 | 設計 C2 の B 行・F 行 | `test_support.rs` の `EXPECTED_FONT_HEIGHT` ＝ 12.0 | 一致 |
| 11 | 文字の色 (0, 40, 100) | 設計 C2 の B 行・Data Models | `definition.rs` の `EXPECTED_FONT_COLOR` ＝ (0, 40, 100) | 一致 |
| 12 | 半角の送り幅 6 | 設計 Data Models・C2 の F 行 | `test_support.rs` の `EXPECTED_ADVANCE_HALF` ＝ 6.0 | 一致 |
| 13 | 全角の送り幅 12 | 設計 Data Models・C2 の F 行 | 同 `EXPECTED_ADVANCE_FULL` ＝ 12.0 | 一致 |
| 14 | 行ボックスの丈 12 | 設計 Data Models・C2 の F 行 | 同 `EXPECTED_LINE_BOX` ＝ 12.0 | 一致 |
| 15 | 行送り 14（＝12 ＋ 行間 2） | 設計 Data Models・C2 の F 行 | 同 `EXPECTED_LINE_PITCH` ＝ 14.0 | 一致 |
| 16 | 1 行の文字数 全角 23 | 設計 Data Models・C2 の G 行 | `wrapping.rs` の `EXPECTED_LINE_LENGTHS` の全角の行 [23, 23, 14] | 一致 |
| 17 | 1 行の文字数 半角 47 | 設計 Data Models・C2 の G 行 | 同 半角の行 [47, 47, 26] | 一致 |
| 18 | 相方側に収まる行数 5（5 行目の下端が 88 ちょうど） | 設計 Data Models・C2 の G 行 | `wrapping.rs` の `KERO_EXACT_FIT_LINES` ＝ 5 | 一致 |
| 19 | 面の系列 本体側 0〜3 → `balloons0〜3.png` | 設計 Data Models・C2 の C 行 | `faces.rs` の系列の表 | 一致 |
| 20 | 面の系列 相方側 0,1 → `balloonk0/1.png`・2,3 → `balloons2/3.png`（縮退・記録 2 件） | 設計 Data Models・C2 の C 行 | 同 表と記録の逐語突合 | 一致 |
| 21 | `\_l[60,42]` 直後のグリフ左上 (82,62) | 設計 C2 の H 行 | `script.rs` の `CURSOR_CASES` の 1 本目 | 一致 |
| 22 | 目印の色 塗り (192,214,242)・文字 (0,20,50) | **設計ではない**——検体の `descript.txt` の `cursor.brush.color.r/g/b` と `cursor.font.color.r/g/b` の宣言行（設計 H 行が書いているのは「`cursor.style,square` → SquareFill 実導出」までで、色の数値はどこにも無い） | `script.rs` の色の突合 | 一致 |
| 23 | 物理寸法 ceil(287×1.25)=359・ceil(287×2)=574・22×1.25=27.5 | 設計 C2 の I 行 | `scale.rs` の期待値の表 | 一致 |

#### 設計が数値を書いていて、テストが数値として固定していないものが 4 件

上の 23 組に入らなかった設計の数値は次の 4 件である。いずれも「テストが取りこぼした」のではなく、**設計自身が数値を固定しないと決めている**か、観測の入力が届かない範囲の値である。したがって較正の対象にもならない（動いたか動かなかったかを言えない）。

| 値 | 出所 | テストが固定しているもの | 数値を固定しない理由 |
|---|---|---|---|
| 半透明の画素の数 本体側 **2,445** | 設計 Data Models | `bake.rs` は「1 つ以上あること」だけ | 設計 Data Models のその行が「固定するのは『0 より多い』ことだけ」と自ら註している。枚数は復号の丸めで動きうるので、動かない性質（2 値へ潰れていない）だけを固定する |
| 半透明の画素の数 相方側 **2,190** | 設計 Data Models | 同上 | 同上 |
| 面 0 の (0,0) の α ＝ **0** | 設計 Data Models | 固定していない | 設計 D 行の述語は焼き上がりの `0 < A < 255` の個数だけで、原点画素の α は述語に入っていない。本記録では §3.1 で実測して事実として残した |
| 本体側に収まる行数 **10**（10 行目の下端 158） | 設計 Data Models | `wrapping.rs` が固定しているのは相方側の 5 行と、3 本文の各行の文字数 | 設計 G 行が流し込む 3 本文はいずれも 3 行に収まる長さなので、本体側の 10 行目の下端に触れる観測がそもそも無い |

**掃き出した結果の明示**: 設計 Data Models の表（10 行）と C2 の A〜I 行の述語欄を 1 行ずつ読み、数値として取り出せる記述を拾った結果、**23 組が突き合わせ可能**・**4 件が設計自身の決定により固定の対象外**だった。このほかに A〜I 行の述語欄には**観測の入力条件として現れる数値**がある——観測する表示スケールの集合 {1.0, 1.25, 2.0}・描画範囲の幅 287・相方側の高さ 68・行の上端の式 20 ＋ 14(n−1) など——が、これらはテストが入力や式としてそのまま固定しているか、上の表に載っている値からの算術的な帰結であり、**期待値どうしの突合の対象ではない**（動いたかどうかを言う対象にもならない）。したがって「較正で動いた期待値 0 件」という結論は、上の 23 組の全一致と 4 件の理由で支えられている。

この掃き出しは手作業であり、常設の検査は無い。**読む範囲（Data Models の表と A〜I 行の述語欄）を逐語で書いてあるので同じ範囲を読み直せば検算できる**が、機械で数え直す仕事は最終検証へ申し送る。

**0 件であることの意味**: 導出値が動くのは、この機械に入っている `ＭＳ ゴシック` が設計の前提（upem 256・半角 0.5em・行間 2）と違う寸法を返す場合である。1 つも動かなかったということは、この機械の書体がその前提どおりに振る舞ったということであり、**「書体の版が違うので期待値をこう動かした」という記録は本仕様には存在しない**。動いていない以上、理由を書く対象も無い。

あわせて、既定書体から落ちていないことは設計 F 行の名前検査（`region.rs` の `undeclared_font_name_resolves_to_the_canon_default`）が担う。同テストは解決された書体名を逐語のリテラル `ＭＳ ゴシック` と突き合わせ、さらに文字寸法の門（送り幅 6.0／12.0）で「名前だけ合っていて実体が別物」を弾く。名前の比較はこの検体に対しては構造上必ず真になる（未宣言なら定数がそのまま返る）ので、**不在を捕まえているのは名前ではなく文字寸法の側**である。

#### 動いたのではなく「無かったものを足した」値が 1 件

混在の本文（全角と半角を交互に並べた 90 文字）の 1 行あたりの文字数 **[31, 31, 28]** は、設計の G 行が「各行の送り幅合計 ≤ 287 かつ貪欲充填の性質」という**性質**でしか書いておらず、数値を持っていなかった。本仕様で実測し、その値で固定した。これは較正で動いた値ではなく、設計が空けていた欄を埋めた値である（内訳: 3 行で半角 45 文字・全角 45 文字）。

#### 期待値を緩めた箇所は 0 件（要件 3.10）

30 本のどれも、赤を避けるために期待値を広げたり、比較を不等号へ弱めたりしていない。上の 23 組はすべて逐語の等値比較である。既存の検体で固定している期待値についても 1 行も触っていない（§2.5 の差分検査）。

### 2.3 空振りを塞ぐために置いた恒久の対照

「緑であること」が保証になるのは、**その主張が偽になりうる形で書かれているとき**だけである。本仕様の観測にはそれ自体では常に真になってしまうものが混じるので、同じテスト群の中に**恒久の対照**を 7 つ置いた。対照は使い捨ての較正ではなく、これから先も一緒に走り続ける。

| # | 対照（テスト名または assert の位置） | 対照が無ければ空振りする主張 | 対照が赤になる条件 |
|---|---|---|---|
| 1 | `region.rs` の `declared_balloon_returns_a_different_font_and_a_split_wrap_threshold` | 「`font.name` が未宣言だから既定書体へ落ちた」——読み口が**常に**既定書体を返す実装でも同じ緑になる | 書体名を宣言している別の検体（`crates/areka-emo-text/examples/fixtures/emo2-vertical-canon`・読むだけ）を同じ読み口へ通したとき、宣言どおりの書体が返らなければ赤 |
| 2 | 同上（同じテスト内の後半） | 「`wordwrappoint` が未宣言だから折返し基準が遠辺へ縮退した」——解決が**常に**折返し基準を遠辺から導いていても同じ緑になる | 対照の検体で折返し基準 145 と遠辺 149 が分かれなければ赤 |
| 3 | `wrapping.rs` の `one_pixel_narrower_bottom_turns_the_same_lines_into_an_overflow` | 「相方側に 5 行がちょうど収まり、あふれ判定は出ない」——あふれ判定が**そもそも一度も発火しない**実装でも同じ緑になる | 同じ 5 行を 1 画素だけ狭めた描画範囲へ流したときに、あふれ判定が発火せず先頭可視行が動かなければ赤 |
| 4 | `script.rs` の `choice_style_follows_the_cursor_style_declaration`（`cursor.style,none` の腕） | 「`cursor.style,square` の宣言から四角い塗りが導かれた」——導出は宣言を読まない分岐（どの値でも同じ塗りを返す）でも同じ緑になる | 宣言を `none` へ差し替えたのに目印なしにならなければ赤 |
| 5 | 同上（塗り色の腕） | 同上——色が実装に焼き付けられていても同じ緑になる | 塗り色の赤成分の宣言を 192 から 7 へ差し替えたのに、導かれた色が追随しなければ赤 |
| 6 | `scale.rs` の `resolving_from_the_physical_surface_moves_the_far_edges` | 「領域解決の入力は供給面の寸ではなく画像の原寸である」——k=1.0 では両者が一致するので、k=1.0 だけの観測では取り違えを検出できない | 供給面の寸を渡したときに k=1.25／2.0 で答えが動かなければ赤（k=1.0 は動かないことも同時に固定する） |
| 7 | `faces.rs` の `assert_resolution_records` の冒頭 2 つの assert（「対照 ⑴」「対照 ⑵」と注記した箇所） | 「失敗の記録が 0 件」——捕捉窓が空（被験コードが窓の外で走った）でも同じ緑になる | **両方の scope で必ず出る情報の記録 2 件**——系列解決の完了（欄 `chain` を持つ 1 件）と 2 層マージの確定（欄 `validrect_left` を持つ 1 件）——が同じ窓・同じ宛先で取れなければ赤。件数だけでなく**欄で見分ける**ので、本文の言い回しが変わっても空振りしない |

対照 7 が**本体側にも出る 2 件**でなければならない理由: 相方側の縮退の記録は本体側には出ない（本体側は落ちる先を持たない）。それを対照に使うと、本体側の「失敗の記録 0 件」が無防備なまま残る。上の 2 件はどちらの scope でも必ず出るので、両方の 0 件を同じ強さで守れる。相方側の縮退の記録 2 件は対照ではなく、**相方側だけの期待値**として別に固定している（`assert_resolution_records` の後半——宛先・`scope`・`surface_id`・`prefix` の 4 欄を逐語で突き合わせ、本体側は 0 件・相方側はちょうど 2 件であることを同じ関数が判定する）。この非対称そのものが「件数の主張が恒真でないこと」の追加の証拠になっている。

補助として `bake.rs` は、半透明の画素を数える前に⑴走査した画素の総数が矩形の面積と一致すること、⑵透明・半透明・不透明の内訳の合計が総数と一致すること、⑶3 種すべてが 1 つ以上実在すること、を固定している。⑶ は走査範囲が矩形の一部分（縁だけ・中身だけ）に偏っていれば赤になるので、「半透明が 1 つ以上ある」が母数の偏りで真になる道を塞いでいる。

### 2.4 1,000 行の番人（要件 3.11）

新規のファイルはすべて上限（1 ファイル 1,000 行）の内側にある。最長は `wrapping.rs` の 488 行で、余裕は 512 行。

| ファイル | 行数 |
|---|---|
| `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs`（入口） | 103 |
| `staysee_balloon_fixture/assets.rs` | 107 |
| `staysee_balloon_fixture/bake.rs` | 215 |
| `staysee_balloon_fixture/definition.rs` | 177 |
| `staysee_balloon_fixture/faces.rs` | 278 |
| `staysee_balloon_fixture/region.rs` | 374 |
| `staysee_balloon_fixture/scale.rs` | 298 |
| `staysee_balloon_fixture/script.rs` | 588 |
| `staysee_balloon_fixture/test_support.rs` | 234 |
| `staysee_balloon_fixture/wrapping.rs` | 488 |

```
wc -l crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs \
      crates/areka-emo-text/tests/staysee_balloon_fixture/*.rs
# → 上の表（合計 2,862 行）。1,000 を超える行は 1 本も無い

cargo test -p log-capture-kit --test file_length_guard_test
# → test result: ok. 6 passed; 0 failed
```

例外表には触っていない。

```
git diff 082379b3 -- crates/log-capture-kit/tests/file_length_guard_test.rs | wc -l
# → 0（差分の行が 1 本も出ない）
```

**番人が空振りしていないことの確かめ**: `wrapping.rs` に一時的に 520 行を足して 1,008 行にしたところ、番人は赤になり、当該ファイルを名指しした。

```
thread 'no_source_file_exceeds_the_line_limit_outside_the_allow_table' panicked:
1 ファイル 1,000 行の目安（structure.md:176）を超えるファイルがある。…:
  crates/areka-emo-text/tests/staysee_balloon_fixture/wrapping.rs (1008 行)
# → 6 本中 3 本が赤
```

確認後ただちに元へ戻し（`git checkout -- …`）、行数が 488 に戻っていること・`git status --porcelain` が無出力であること・番人が再び 6 本緑であることを確かめた。

### 2.5 既存の期待値を 1 つも緩めていないこと（要件 3.1・3.10・8.1）

基準コミット `082379b3`（本仕様の実装着手前＝タスク生成直後の HEAD）からの差分で確かめた。

| 何を確かめたか | 命令 | 結果 |
|---|---|---|
| 本番コードを変えていない（要件 8.1） | `git diff --numstat 082379b3 \| grep -c '/src/'` | **0** |
| 既存のテスト・example を変えていない（要件 3.1・3.10） | `git diff --numstat 082379b3 -- crates \| grep -v 'staysee_balloon_fixture'` | 出力なし＝**0 行**。`crates/` 配下で差分が出るのは本仕様の新規 10 ファイルだけ |
| 依存記述とロックを変えていない（要件 8.2） | `git diff --numstat 082379b3 \| grep -cE 'Cargo\.(toml\|lock)$'` | **0** |
| 番人の例外表を変えていない（要件 3.11） | `git diff 082379b3 -- crates/log-capture-kit/tests/file_length_guard_test.rs \| wc -l` | **0** |
| 書式 | `cargo fmt --check` | 終了コード 0（出力なし） |

既存テストが赤になって期待値を書き換えた、という事象も起きていない（そもそも既存テストのファイルが 1 行も変わっていない）。

**この検査そのものの較正（重要）**: 上の 3 本目までを、設計 C8 が書いている形——`git diff --stat <base>..HEAD -- 'crates/*/src'` のような**波形の入ったパス指定**——で打つと、**変更が実在しても何も出力されない**。実測で確かめた。

```
# 較正: 本番コードを 1 行だけ一時的に変えてから両方の形で打つ
printf '\n' >> crates/areka-emo-text/src/region.rs

git diff --numstat 082379b3 -- 'crates/*/src'
# → 出力なし（変更は実在するのに 0 に見える＝この形は判定になっていない）

git diff --numstat 082379b3 | grep '/src/'
# → 1  0  crates/areka-emo-text/src/region.rs（正しく捕まる）

git checkout -- crates/areka-emo-text/src/region.rs   # ただちに復旧
```

同じことを既存テストのファイル（`crates/areka-emo-text/tests/decoration_readback_test.rs`）でも確かめた（`-- 'crates/*/tests'` は変更を実在させても無出力・全差分を絞り込む形は `1 0` を返す）。

原因は、git の既定のパス指定が**道の全体に一致すること**を要求する点にある。`crates/*/src` は「末尾が `src` で終わる道」——つまりディレクトリそのもの——にしか当たらず、その下のファイルの道には当たらない。**波形が区切り文字を跨げないからではない**（実測: `-- 'crates/*region.rs'` は 3 段下の `crates/areka-emo-text/src/region.rs` に当たる）。末尾に `/*` か `/**` を足すか、`:(glob)` を付ければ当たる。

したがって本節の判定には**全差分を採ってから絞り込む形**を使い、上の較正で「変更があれば必ず捕まる」ことを示した上で 0 を主張している。この食い違いは検査の道具の問題であって areka の欠陥ではないので、要件 3.9 は発動しない（§4）。設計 C8 と `baseline.md` §1 も同じ形を載せていたが、この発見を受けて**どちらも是正済み**である（設計 C8 に註と正しい 2 つの形、`baseline.md` §1 に採り直した形と値）。後段のタスク 6 は絞り込む形で採り直すこと。

### 2.6 `cargo test --workspace` の本数

命令（着手前・着手後とも同じ旗）: `cargo test --workspace -j 4 --no-fail-fast`。他の `cargo` を 1 つも並走させずに単独で走らせた（壁時計の期限を持つ既存テストを飢餓させないため）。走らせる前に i686 の成果物を 2 つそろえ、ワークスペースのビルドが x64 版で上書きした helper を i686 版へ置き直してから走らせた（手順は `baseline.md` §2.1）。走行の後に `target/debug/shiori-host32-helper.exe` の**対象機械の欄**を読み直し、走行中ずっと i686 版が置かれていたことを確かめた。これを外すと相方の起動試験が赤になり、そこで走行が打ち切られて本数の突き合わせが成立しない（着手前の計測でも一度これで 60 ターゲット・5,411 本に縮んでいる）。

欄の在り処は決め打ちにできない（実行形式の先頭から数えた固定の位置ではなく、見出しの位置を先に読んでから相対で辿る）。採り方と結果:

```
# ⑴ 見出しの位置を先頭 0x3C（10 進 60）の 4 バイトから読む
od -An -tu4 -j 60 -N 4 target/debug/shiori-host32-helper.exe
# → 240

# ⑵ そこが本当に見出しの先頭か（署名を読む）
od -An -c -j 240 -N 4 target/debug/shiori-host32-helper.exe
# → P E \0 \0

# ⑶ 見出しの先頭 ＋4 バイト目が対象機械の欄
od -An -tx1 -j 244 -N 2 target/debug/shiori-host32-helper.exe
# → 4c 01 （下位から並ぶので 0x014C ＝ i686）

```

対照として、同じ 3 手順を x64 の実行形式（`target/debug/areka.exe`）にも当てた。**見出しの位置は実行形式ごとに違う**ので、手順 ⑴ から採り直している。

```
# ⑴ 見出しの位置
od -An -tu4 -j 60 -N 4 target/debug/areka.exe
# → 232 （helper の 240 とは違う）

# ⑵ 署名
od -An -c -j 232 -N 4 target/debug/areka.exe
# → P E \0 \0

# ⑶ 見出しの先頭 ＋4 バイト目 ＝ 236 バイト目
od -An -tx1 -j 236 -N 2 target/debug/areka.exe
# → 64 86 （0x8664 ＝ x64）
```

**これが判定になっている理由**: 同じ 3 手順で x64 の実行形式は別の値（`64 86`）を返す。つまり helper が x64 版で上書きされたままなら、この欄は `4c 01` にならない。

**位置を求め直さなければならない理由**（手順 ⑴ を飛ばせない）: helper の位置 244 をそのまま `areka.exe` へ当てると `00 00` が返る（`od -An -tx1 -j 244 -N 2 target/debug/areka.exe` の実測）。これは対象機械の欄ですらない別のバイトなので、i686 とも x64 とも判定できない。手順 ⑵ の署名の確認は、まさにこの取り違え——位置の求め方を間違えて無関係なバイトを読むこと——を捕まえるために挟んでいる。

| 数え方 | 着手前（`baseline.md` §2.2） | 着手後（本タスクで実測） | 差 |
|---|---|---|---|
| 走行の終了コード | 0 | 0 | — |
| `Running` 行の本数 | 81 | 82 | **＋1**（本仕様の新規テストターゲット 1 本） |
| `Doc-tests` 行の本数 | 22 | 22 | 0 |
| `test result:` 行の本数 | 103 | 104 | **＋1** |
| passed の合計 | 7,668 | 7,698 | **＋30**（本仕様の新規テスト 30 本） |
| failed の合計 | 0 | 0 | 0 |
| ignored の合計 | 40 | 40 | 0 |

数え方（走行の全文に対して掛けた。`Select-Object -First N` や `| tail` を cargo の上流に挟むと部分的な緑を全体の緑に見せるので使っていない）:

```
grep -cE '^[[:space:]]*Running '   <全文>
grep -cE '^[[:space:]]*Doc-tests ' <全文>
grep -c  'test result:'            <全文>
grep -o '[0-9]\+ passed'  <全文> | awk '{s+=$1} END{print s}'
grep -o '[0-9]\+ failed'  <全文> | awk '{s+=$1} END{print s}'
grep -o '[0-9]\+ ignored' <全文> | awk '{s+=$1} END{print s}'
```

完走の裏取り: `test result:` 行の本数（104）が `Running` と `Doc-tests` の合計と一致している（取りこぼし 0）。

---

## 3. 既知の縮退（事実の記録）

ここに挙げる 3 件は**欠陥ではない**。ukadoc が既定を定めている鍵を StayseeBalloon が宣言していないか、正典が定めた受け皿へ落ちる経路をそのまま通っているだけで、areka はいずれも**記録を残した上で**正典どおりの既定へ落ちている。直すべきものは無いので、後から読んだ人が「記録が出ているのは異常では」と迷わないように、実在と理由をここに書き留める。

### 3.1 枠の原点の色が白へ落ちる

| 項目 | 内容 |
|---|---|
| どこで起きるか | `crates/areka/src/emo2_boot/balloon_background.rs` の `face_origin_color`（起動時に面 0 の原点画素をバルーンの背景色として採る関数） |
| 実測 | `balloons0.png`（335×205）と `balloonk0.png`（335×135）のどちらも (0,0) の α は **0**（完全な透明）。α > 0 の画素を囲む矩形は**両方とも (2,2) から始まる**（上端 **2 行**と左端 **2 列**が全画素 α＝0） |
| 何が起きるか | 同関数は原点画素の α が 255 のときだけその色を採る。この検体では原点が焼き込みの前に落ちている（矩形が (2,2) 始まり＝ずれが 0 でない）ので、「原点が矩形の外」の腕を通り、既定の背景色（白）へ落ちて `debug!` を 1 件残す |
| なぜ正典どおりか | 半透明前提の枠は角が抜けているのが普通で、原点が透明なのは崩れではない。areka は「半透明の画素は背景色として使えない」と決めているので、白へ落とすのは設計どおりの受け皿である。記録の無い失敗経路を作らないために `debug!` を残しており、握り潰してはいない |

調べ方（採った実測）: **矩形そのものを採る形**で測った——全画素を走査して α > 0 の画素の x と y の最小・最大を採り、その矩形の中の α を透明・半透明・不透明に数え分けた（PNG を復号して 1 画素ずつ読む）。縁の 1 行・1 列だけを見る形では矩形の開始点は決まらないので使っていない。

```
balloons0.png: 原寸 335×205  (0,0) の α=0
  α>0 の矩形 = (2,2)-(334,204)  大きさ 333×203
  矩形の中: 透明 8,541 ／ 半透明 2,445 ／ 不透明 56,613
balloonk0.png: 原寸 335×135  (0,0) の α=0
  α>0 の矩形 = (2,2)-(334,134)  大きさ 333×133
  矩形の中: 透明 7,631 ／ 半透明 2,190 ／ 不透明 34,468
```

この採り方が意味を持つのは、areka 側の矩形の決め方と同じだからである。`crates/areka-emo-atlas/src/trim.rs` の `TrimResult` を作る走査は「α > 0 の画素を囲む最小の矩形」を採り、そのずれを `trim_offset` として持つ。上の実測ではずれが (2,2) で 0 でないので、原点画素は焼き上がりに含まれず、色を採る腕には入らない。

半透明の画素の数（本体側 2,445・相方側 2,190）は設計 Data Models が書いている値と一致した。矩形の大きさ 333×203 と矩形内の透明画素 8,541 も、原寸 335×205 から上端 2 行と左端 2 列（2×335 ＋ 2×203 ＝ 1,076 画素）が落ちた結果として辻褄が合う。

### 3.2 折返し基準が未宣言なので遠辺へ落ちる

| 項目 | 内容 |
|---|---|
| どこで起きるか | `crates/areka-emo-text/src/region.rs` の `resolve_or`（未指定の座標成分を既定辺へ落とす関数） |
| 実測 | StayseeBalloon の `descript.txt` に `wordwrappoint.*` の行は 1 本も無い。横書きで解く経路は `wordwrappoint.x` を引き、未指定なので描画範囲の右辺 309 を代わりに使い、`debug!`（欄 `key`＝`wordwrappoint.x`・`fallback`＝309）を 1 件残す |
| 何が見えるか | 折返し基準と遠辺がどちらも 309 になる。決定論テスト `region.rs` の `undeclared_wrap_threshold_degenerates_to_the_inline_limit` がこの一致を固定している |
| なぜ正典どおりか | ukadoc は `wordwrappoint` 未宣言のとき `validrect` の遠辺を折返し基準にすると定めている。`warn!` ではなく `debug!` なのは、これが正常系に近い縮退だからである（同ファイルのその旨のコメントのとおり） |

補足（同じ仕組みで出るもう 1 件）: `origin.*` も未宣言なので、`resolve_origin_component` が「未指定の origin 成分を書字開始角へ寄せる」`debug!` を成分ごとに残す。描画開始点が (22,20) になるのはその結果で、これも正典どおりの既定である。

### 3.3 相方側の面 2・3 が本体側の系列へ落ちる（記録 2 件）

| 項目 | 内容 |
|---|---|
| どこで起きるか | `crates/areka-emo-present/src/balloon.rs` の `resolve_balloon_faces`（面の系列を解決する関数） |
| 実測 | 保管フォルダに在る相方側の枠は `balloonk0.png` と `balloonk1.png` の 2 枚だけである。相方側（scope 1）を解決すると面 0・1 は `balloonk*` から採れるが、面 2・3 は相方側に無いので本体側の `balloons2.png`／`balloons3.png` へ落ち、面ごとに `warn!` を 1 件ずつ＝**計 2 件**残す（欄 `surface_id` は 2 と 3・`prefix` は `balloons`） |
| 何が見えるか | 相方側でも 4 面すべてが解決される。決定論テスト `faces.rs` の `face_series_and_records_are_pinned_for_both_scopes` が、系列の 4 行と記録 2 件を逐語で固定し、同じ捕捉窓で失敗の記録が 0 件であることも固定している |
| なぜ正典どおりか | 正典は「相方側に無い面は本体側の系列から採る」と定めており、これは失敗ではなく規定のフォールバックである。同関数の説明も「解決自体は成功として続行する」と明記している。`warn!` なのは、資産の枚数によっては作者が気づきたい情報だからで、areka 側の欠陥の合図ではない |

上の 2 件は「相方側の枠が 2 枚しか無い」という資産の形から必ず出るものなので、StayseeBalloon を使う限りこの記録は毎回出る。第三者向けの説明では「相方のバルーンは本体のバルーンの絵を一部借りて表示される」と読み替えればよい。

---

## 4. 崩れの処理（要件 3.9／4.3）

### 4.1 要件 3.9（決定論テストで崩れが出た場合）——**発動なし**

設計 C2 の A〜I 行の観測（要件 3.3〜3.8）で areka 側の崩れは 1 件も見つからなかった。したがって「本仕様内で直す」も「引受先の spec を実在確認して先送りする」も行っていない。バルーン側のファイルも 1 バイトも変えていない。

その判定の根拠は次の 3 つで、いずれも §2 に命令と結果を書いた。

| # | 根拠 | 出所 |
|---|---|---|
| 1 | 較正で動いた期待値が 0 件である。設計の数値 23 組と新規テストの固定値が 1 つも食い違わず、その値のまま 30 本が緑になった | §2.2 |
| 2 | 期待値を 1 つも緩めていない。既存のテスト・example の差分が 0 行で、新規テストの比較もすべて逐語の等値である | §2.2・§2.5 |
| 3 | 本番コードの変更が 0 行である。崩れを直したなら要件 8.1 の例外として本番コードに手が入るはずだが、差分は 0 行である | §2.5 |

§3 に挙げた 3 件の縮退は、正典が定めた既定へ落ちているだけで崩れではないため、ここでの行き先を持たない（§3 の各項の「なぜ正典どおりか」を参照）。

なお §2.5 に書いた「設計 C8 のパス指定が変更を検出しない」件は、**検査の道具の問題であって areka の表示の崩れではない**ので要件 3.9 の対象外である。本仕様では検査の側を較正済みの形へ差し替えて対処した（設計・`baseline.md` の記述の是正は本タスクの守備範囲外なので手を入れていない）。

### 4.2 要件 4.3（実機目視で崩れが見つかった場合）

この項はタスク 4 が追記する（未記入）。実機目視で崩れが出たかどうかと、出た場合の行き先（本仕様内で直したか、引受先の spec の実在を確かめて先送りしたか）を書く。

---

## 5. 実機目視の観察記録（要件 4.2）

この節はタスク 4 が埋める（未記入）。日付・表示に使ったゴースト・バルーンの絶対パス・表示スケール・観察項目・所見を書く。

## 6. 第三者向け README への申し送り文（要件 5.2）

この節はタスク 5.1 が埋める（未記入）。資産名・作者・CC0・出典 URL・既知の制限を、そのまま写せる形で書く。

## 7. 台帳の検査（要件 6.5）

この節はタスク 3.2 が埋める（未記入）。`cargo test -p ukadoc-survey` の結果と、報告 2 本を作り直したことを書く。

## 8. 下流への申し送りの実施記録（要件 7.3・7.4）

この節はタスク 5.2 が埋める（未記入）。申し送り先 3 つの brief が実在することを確かめたことと、追記した内容を書く。

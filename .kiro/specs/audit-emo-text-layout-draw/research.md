# ギャップ分析: audit-emo-text-layout-draw（文字の層のレイアウトと描画の見直し）

- 測った時点: `main` `73c5b25d`（枝 `claude/kiro-start-audit-emo-text-layout-draw-68feba`・`d817f091`）・2026-11-06
- 物差し: `/ponytail:ponytail-audit`（ponytail 5.3.3・利用者全体に入れてあるまま使い、リポジトリには写していない＝要件 2.6）。作法のうち 2 つを外した（要件 2.2・2.3）。
  - 「最大 20 件」を外した。指摘は全件を下に並べ、省いた件は無い。
  - 「読まなかった所（Not checked）」を残さない。範囲の 81 ファイルを全部読んだ（2 節）。
- 番号は audit の番号（組をまたいで通し番号）。組は audit の 3 つ（必ず直す・直したほうがよい・できれば）のまま。
- この文書はコードを変えていない。直すか残すかの決めは要件の討議に回す（要件 3.1）。

## 1. 要約

- **何をするところか**: さくらスクリプトの台本から作った文字の列（`TextItem`）を、バルーンの文字描画範囲（validrect）・折り返し基準（wordwrappoint）・書字方向に従って並べ（`layout*`・`wrap`・`segment`・`cursor_tag`・`region`・`writing`）、見た目（`look`・`color`・`underline`）を決め、DirectWrite と Direct2D で GPU の面へ描く（`draw*`・`canvas`・`viewbox*`）。描くのは毎フレーム UI スレッドの上で、前のフレームとの差だけを描き直す（`viewbox_diff`）。
- **想定した負荷**: 1 人の利用者の 1 台の机で、インストールしたゴーストの台本とバルーンを毎フレーム並べて描く。台本とバルーンの `descript.txt` はネットから入れたもので、**信用できない入力**として扱った（書き損じひとつで、落ちる・固まる・位置が無限大になる・メモリが際限なく増える、が起きないこと）。台本の大きさは甘く見ない（記憶「ゴーストの台本の大きさを甘く見ない」）。
- **指摘の数**: 必ず直す **8 件**・直したほうがよい **19 件**・できれば **8 件**＝**35 件**（テストの整理と説明の直しは種類ごとに 1 件へまとめ、場所は全部を挙げた）。
- **見つかった型**: ⑴ 大きな数・無限大の位置の守りが無い（1〜4＝申し送りの R17 と同じ型が `\_l` と描き直しの計算にもある）。⑵ 決めごととの食い違い（5＝空行を出す・6＝バルーンの既定の下線が描かれない・14・15＝下線の寄せ）。⑶ 信用できない入力で育つ・2 乗になる（7・9〜12・20）。⑷ 描き直しの無駄と取りこぼし（16〜18）。⑸ 失敗しえないテスト・檻の無い分岐。⑹ 写しと古い説明。
- **大きさ**: 直しは見込みで 28〜33 タスク。要件 3.7 の 20 を越えるので、組で 2 本に切る案を出す（8 節）。

## 2. 読んだ範囲（要件 1.1・1.2・2.3）

- 範囲は `git ls-files crates/areka-emo-text/src` のうち 14 の頭（`layout`・`segment`・`region`・`wrap`・`lookahead`・`cursor_tag`・`writing`・`draw`・`canvas`・`viewbox`・`look`・`color`・`underline`・`place`）で始まるもの。**81 ファイル**（コード 27・テスト 54）・**38,463 行**（コード 10,820 行・テスト 27,643 行・要件の表と一致）。`audit-emo-text-state-actor` の持ち物（`actor*`・`state*`・`choice*`・`sink*`・`surface*`・`lib.rs`・`balloon_overrides*`・`inline_marker*`・`marker_surface*`・`scroll_marker*`・`tests/`・`examples/`・`Cargo.toml`）と `vendors/` は含めていない。
- **81 ファイルを全部、頭から終わりまで読んだ**。読み手は 8 人（下請け）で、ファイルのまとまりごとに分けた。進行役は全員の報告をまとめ、必ず直すの全件と、直したほうがよいのうち重いもの（7・14・16〜18 ほか）の行を自分で開いて確かめた。
  - 組 1（13）: `layout.rs`・`layout_line_ops.rs`・`layout_scan.rs`・`layout_scan_glyph.rs`・`layout_styled.rs`・`layout_test_support.rs`・`wrap.rs`・`layout_cluster_tests.rs`・`layout_hard_limit_tests.rs`・`layout_segmented_tests.rs`・`layout_styled_tests.rs`・`layout_visible_window_tests.rs`・`layout_wrap_tests.rs`
  - 組 2（11）: `cursor_tag.rs`・`cursor_tag_test_support.rs`・`cursor_tag_tests.rs`・`cursor_tag_resolve_tests.rs`・`layout_cursor_center_origin_tests.rs`・`layout_cursor_order_tests.rs`・`layout_cursor_overflow_tests.rs`・`layout_cursor_tests.rs`・`layout_cursor_vertical_canon_tests.rs`・`layout_cursor_vertical_tests.rs`・`layout_cursor_wiring_tests.rs`
  - 組 3（10）: `segment.rs`・`region.rs`・`region_inline_limit_tests.rs`・`region_tests.rs`・`region_vertical_canon_tests.rs`・`lookahead.rs`・`lookahead_tests.rs`・`writing.rs`・`writing_decision_tests.rs`・`place.rs`
  - 組 4（10）: `draw.rs`・`draw_catalog.rs`・`draw_catalog_tests.rs`・`draw_format_metrics_tests.rs`・`draw_line_store.rs`・`draw_line_store_tests.rs`・`draw_metrics.rs`・`draw_metrics_styled_tests.rs`・`draw_oracle_tests.rs`・`draw_test_support.rs`
  - 組 5（9）: `canvas.rs`・`color.rs`・`color_tests.rs`・`look.rs`・`look_font_tag_tests.rs`・`look_font_tag_value_tests.rs`・`look_tests.rs`・`underline.rs`・`underline_tests.rs`
  - 組 6（11）: `viewbox.rs`・`viewbox_axis_tests.rs`・`viewbox_choice_marker_tests.rs`・`viewbox_diff.rs`・`viewbox_dirty_tests.rs`・`viewbox_plan_commit_tests.rs`・`viewbox_style_fingerprint_tests.rs`・`viewbox_test_support.rs`・`viewbox_draw.rs`・`viewbox_draw_plan.rs`・`viewbox_draw_test_support.rs`
  - 組 7（9）: `viewbox_draw_render.rs`・`viewbox_draw_decoration.rs`・`viewbox_draw_underline.rs`・`viewbox_draw_decoration_face_tests.rs`・`viewbox_draw_decoration_tests.rs`・`viewbox_draw_underline_tests.rs`・`viewbox_draw_color_emoji_tests.rs`・`viewbox_draw_inline_marker_tests.rs`・`viewbox_draw_png_dump_tests.rs`
  - 組 8（8）: `viewbox_draw_choice_hover_tests.rs`・`viewbox_draw_frame_render_tests.rs`・`viewbox_draw_line_gap_band_tests.rs`・`viewbox_draw_live_diff_tests.rs`・`viewbox_draw_oracle_regression_tests.rs`・`viewbox_draw_scroll_retain_tests.rs`・`viewbox_draw_vertical_band_readback_tests.rs`・`viewbox_draw_vertical_band_scroll_tests.rs`
  - 13＋11＋10＋10＋9＋11＋9＋8＝**81**＝範囲のファイルの数と一致（要件 2.3）。
- 呼び手を確かめるために範囲の外も読んだ（指摘には数えない）: `state.rs`・`state_decoration.rs`・`actor_present.rs`・`actor_attach.rs`・`actor_decoration.rs`・`actor_scroll.rs`・`choice.rs`・`surface.rs`・`inline_marker.rs`・`crates/areka-parsers/src/sakura/decode.rs`・`crates/areka/src/emo2_boot/frame/scale_text.rs`・`crates/areka/src/mcp/check_script_judge.rs`・`crates/areka/src/emo2_boot/strict_judges.rs`。
- テストは 1 本も走らせていない（読むだけの段）。「PLAUSIBLE」と書いた件は、コードを追って詰めたが走らせて確かめてはいない。

## 3. 指摘の一覧

各件の欄: **場所**／**何か**／**困る場面**／**いちばん小さい直し**／**範囲の外への波及**（ア＝`actor*`・`state*` から呼ばれる関数の形が変わるか、イ＝範囲の外のファイルに触るか）／**残す 3 種類に当たるか**（要件 3.2 の ⑴＝裁定に反する・⑵＝わざと置いた先送りの印・⑶＝SSP 互換・ukadoc 正典）。

### 必ず直す

#### 1. 改行の割合の和が大きいと、配置の位置が無限大や数でない値になる（申し送り R17）（`layout_scan.rs:204,235,358-361`）

- **出どころ**: `audit-parsers-sakura` の `research.md` §13.4 の R17（`.kiro/specs/completed/audit-parsers-sakura/research.md:570`）。parsers は非有限の割合だけを既定の 1.0 へ倒し、有限で大きな値は縛らない（同 spec の指摘 3）。残りの守りを配置の側で受け持つ（要件 5）。
- **何か**: 改行の割合（`TextItem::LineBreak { ratio: f32 }`・`decode.rs:443` の `percent / 100.0`・1 つは最大約 ±3.4e36）は、続く改行のあいだ `layout_scan.rs:204` で足し合わされ（`acc + ratio`）、字が置かれるときに `apply_pending_newline`（`:358-361`）が `*block_pos += block_dir * pitch * sum` で位置へ足す。和にも積にも守りが無い。`\_l` の実効位置の下見（`:235`）も同じ式を写している。
- **困る場面**: 字の高さ 10（行送り 12）で `\n[3e38]` を 10 個続けて字を置くと、`pitch × sum` が f32 を越えて行の位置が `+inf` になる。続けて `\n[-3e38]` を 20 個で `inf + (-inf)`＝NaN。行の矩形に入った inf／NaN は、見える窓の計算（`layout.rs:465-479`）で `block_offset` を `-inf`／NaN にし、指摘 3 のあふれ（試験の作りでは UI スレッドで panic）へつながる。記録は何も出ない。今のテストは割合 1.0・0.5・0・1.0+0.5・1.5 だけで、負・巨大・あふれる和は 0 本。
- **いちばん小さい直し**: 行送りの軸へ足した結果が有限でないときは、送りを捨てて位置をそのまま保ち（行内の軸は今と同じく行頭へ戻す）、`warn!` を残す。判定は 1 か所の小さな関数にまとめ、`apply_pending_newline` と下見（`:235`）の両方から呼ぶ（2 か所が同じ規則で動くように）。和（`:204`）があふれたときも同じ扱い。毎フレーム配置し直すので、`warn!` は字 1 つにつき 1 回にする（今ある `CursorWarnGuard` に縮退の種類を 1 つ足すのが近い。要件 5.2）。ふつうの大きさの割合の位置は変えない（要件 5.4）。
- **壊れたら落ちるテスト**: `layout_wrap_tests.rs`（余白 264）に「`\n[3e38]`×10＋字・`\n[-3e38]`×20＋字で、どの行の位置も有限・`warn!` が 1 件」。見える窓までの通しは `layout_visible_window_tests.rs`（余白 634）。
- **範囲の外への波及**: ア なし（どちらも私有の関数）。イ なし（`layout_scan.rs`・字 1 つ 1 回の記録に `cursor_tag.rs` を使うならそれも範囲の中）。
- **残す 3 種類**: 当たらない（brief と要件 5 で望む姿が決まっている）。

#### 2. `\_l` の座標の解決の結果が無限大になりうる（`cursor_tag.rs:154-162`・`layout_line_ops.rs:87-91`・説明 `cursor_tag.rs:106-107`）

- **何か**: `state.rs:265` の `parse_cursor_coord` は**読んだ数**が有限でないものだけを `Invalid` へ落とす。解決層は「基点＋値×係数」を計算し、その結果を確かめずに `Ok(Some(..))` で返し、配線（`resolve_cursor_component`）も素通しする。説明の「非有限値は語彙層で既に Invalid」は、結果については誤り。
- **困る場面**: `\_l[3e38em,]あ` は 3e38×10＝`+inf` で、字と行の矩形が inf に置かれる。`\_l[3e38em,]\_l[@-3e38em,]あ` は inf+(−inf)＝NaN で、その後は折り返しの比べ（`layout_scan_glyph.rs:83,134`）がいつも偽になり、続く字が全部 1 本の NaN の行に積まれる。`\_l[,3e38lh]` は行送りの軸が inf になり、指摘 1 と同じく見える窓と指摘 3 のあふれへつながる。記録は出ない。
- **いちばん小さい直し**: 解決の結果が有限でないときは `Err(CursorDegrade::Unparsable)` と同じ扱いにする（その軸は動かさない・今ある「字 1 つにつき 1 回」の `warn!` の道を通る・新しい種類は要らない）。直す場所は `resolve_cursor_axis`（`cursor_tag.rs`）か、配線の `resolve_cursor_component`（`layout_line_ops.rs`）の 1 か所。説明の 1 行も直す。
- **壊れたら落ちるテスト**: `cursor_tag_tests.rs`（余白 426）に解決の単体、`layout_cursor_wiring_tests.rs`（余白 451）に「行の位置に非有限が無い・`warn!` が 1 件」。
- **範囲の外への波及**: ア なし（形は変わらない。`cursor_tag` の項目を範囲の外で使う所は grep で 0 件）。イ なし。
- **残す 3 種類**: 当たらない。ukadoc（<https://ukatech.shillest.net/ukadoc-spec/list/sakura-script/#_l-x-y>）は数値を「バルーンの文字描画開始点…からのピクセル単位座標」とだけ書き、あふれには触れない。「字義どおり用い、内側へ寄せない」（完了 spec の R2.6）は有限の点の話で、無限大は位置ではない。

#### 3. スクロールの確定位置が i32 であふれ、試験の作りで panic する（`viewbox.rs:174-189`・不変条件の説明 `:32,52-53,60`）

- **何か**: `resolve_position` は `pos.round() as i32`（飽和）で確定位置を作り、`blit_vector` は `target.committed - self.committed` を素の i32 で引く。面をずらす `surface.rs:568,571`（範囲の外）は `-dy`・`h - dy` を計算する。
- **困る場面**: `あ\_l[0,3000000000]い`（有限の値なので指摘 2 を直しても通る）で、確定位置が `i32::MIN` に飽和し、ずらし量が (0, i32::MIN) になる。`copy_front_to_back_shifted` の `-dy` が「negate with overflow」で panic（試験の作り・`[profile.dev]` は桁あふれの検査を切っていない）。続けて `\_l[0,0]う` か、利用者がホイールで上へ戻すと、`viewbox.rs:188` の `0 - i32::MIN` も panic。出荷の作りでは黙って回り込み、たまたま全面の描き直しへ落ちて見た目は正しいだけ。説明の「|committed − pos| ≤ 0.5 は常に真」は |pos| > i32::MAX で崩れる。指摘 1・2 の無限大も、この道へ流れ込む。
- **いちばん小さい直し**: `blit_vector`（か `plan_with_overhangs`）で引き算を飽和にし、ずらし量の各成分を ±面の寸で頭打ちにする（|ずらし| ≥ 面の寸は「全部が送り出された」と同じ意味なので、露出の帯と写しの意味は変わらない・向きは保つ）。説明の不変条件を事実に合わせる。
- **壊れたら落ちるテスト**: `viewbox_axis_tests.rs`（余白 853）に飽和した確定位置からのずらし、`viewbox_plan_commit_tests.rs`（余白 252）に「±3e9 の位置で plan → commit → plan が panic せず全面の描き直しへ落ちる」。
- **範囲の外への波及**: ア なし。イ なし（`surface.rs` は、ずらし量が面の寸に収まれば触らずに安全になる＝`audit-emo-text-state-actor` の持ち物に触らない）。
- **残す 3 種類**: 当たらない。

#### 4. 描き直しの範囲の計算が i64 であふれ、試験の作りで panic する（`viewbox_diff.rs:426-429`）

- **何か**: `expand_guard_clamp` は `min_x.floor() as i64 - guard`・`max_x.ceil() as i64 + guard` を計算する。|値| ≥ 約 9.2e18（無限大を含む）は `as i64` で `i64::MIN／MAX` に飽和し、ガード（1 以上）を足し引きするとあふれる。
- **困る場面**: `あ\_l[1e30,]い` を文字送りで出すと、`い` が届いた後のフレームで変わった行の範囲がこの関数を通り、「add with overflow」で panic（試験の作り）。指摘 2 を直しても 1e30 は有限なので起きる。指摘 1・2 が残れば無限大でも起きる。
- **いちばん小さい直し**: 4 行の足し引きを `saturating_sub`／`saturating_add` にする。
- **壊れたら落ちるテスト**: `viewbox_dirty_tests.rs`（余白 230）に「位置が ±1e30・±inf の行の範囲が panic せず面の中へ切られる（か `None`）」。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

#### 5. 塊で折り返すとき、`\_l` の直後の塊の前で空行を出し、`\_l` の位置を捨てる（`layout_scan_glyph.rs:111-116,149-158`・説明 `:112-114`・`layout.rs:344-345`）

- **何か**: 塊（BudouX の区切り）の頭で「今の行には収まらないが、行頭からなら収まる」ときに行を送る枝は、`!self.current.is_empty()` を見ない。説明は「行頭では cap_rem == cap_full ゆえ本分岐は構造的に不発火＝空行を作らない（INV-3）」と書くが、`\_l` の直後は行が空なのに行内の位置が行頭でない。`segment.rs:143-150` は `\_l`（`CursorMove`）ごとに区切りを始め直すので、`\_l` の直後の字は必ず塊の頭になり、本番の道で届く（進行役がコードで確かめた）。
- **困る場面**: wordwrappoint の x＝100、BudouX の折り返し、`\_l[50,]あいうえおか`（塊 1 つ・6 字で 60）。字 0 の手前で行内が 50 へ動き、塊の和 60 は残り 50 を越え、行頭からの 100 には収まるので、送りの枝に入る。**字の無い行**（矩形 0,0,50,10）が 1 本出て、行送りが 12 進み、字は次の行の x＝0 に置かれ、`\_l` の x が消える。文字単位の折り返しなら最初の字は 50 に置かれる。開発者の裁定「行は字が置かれたときだけ作る・空行は 1 本も出さない」（2026-09-04・記憶「`\_l` は `\n` と同じ遅延座標指定」）に反する。塊の折り返しと `\_l` を組み合わせたテストは 0 本。
- **いちばん小さい直し**: その枝の条件に `&& !self.current.is_empty()` を足し、行頭の場合は今ある「長い塊は文字の規則へ」の枝へ落とす（`\_l` の x はそのまま・続く字は折り返し基準で 1 字ずつ折れる）。説明 2 か所を直す。
- **壊れたら落ちるテスト**: `layout_segmented_tests.rs`（余白 217）に「`\_l[50,]` の直後の塊で空行が出ず、最初の字が x＝50」。
- **範囲の外への波及**: ア なし。イ なし。`layout_cursor_overflow_tests.rs` の `finish_line(` の数え（4）は変わらない。
- **残す 3 種類**: 当たらない（直しが裁定に沿う）。SSP で BudouX と `\_l` が重なったときの落ち方は測っていない（areka 独自の塊の折り返しなので正典の外）。

#### 6. バルーンの `font.underline,1`・`font.strike,1` が描かれない（`viewbox_draw_decoration.rs:129-138,212`・`viewbox_draw_render.rs:503-508`・テスト `viewbox_draw_decoration_tests.rs:636-669`）

- **何か**: 既定の見た目（`looks.default`）には `font.underline`・`font.strike` が入る（`draw.rs:313`・`look.rs:205`・`look_tests.rs:310`）。ところが台本の字は同じ既定に対して見た目を登録するので、飾りの無い字は全部 `StyleId::DEFAULT` になる。下線の区間は「`DEFAULT` でない区間のうち、`look.underline != default.underline`」だけを選び（`:131-136`）、打ち消し線も「既定と違うとき」だけ焼く（`:212`）。既定だけの行は何もせずに抜ける（`render.rs:503-508`）。DirectWrite の書式（TextFormat）は下線と打ち消し線を持てないので、どこでも描かれない（進行役が `draw_line_store.rs`・`draw.rs`・`render.rs` を grep して確かめた）。
- **困る場面**: `descript.txt` に `font.underline,1` と書いたバルーンで、台詞のどの字にも下線が出ない。打ち消し線も同じ。太字・斜体は書式に載るので効く。完了 spec `areka-P0-balloon-font-descript-keys` の要件 5.5（09-17 追加）「`font.bold,1`（または `italic`／`strike`／`underline` の `1`）が書かれている, the areka shall その装いを既定の見た目として描く」と、同 spec の要件 6.1 の台帳の改め（4 項目を `implemented` へ）に反する。テスト `viewbox_draw_decoration_tests.rs:641-669` は「既定と同じ下線の区間は選ばない（今の選び方と同じ）」と、今の誤りを固めている。
- **いちばん小さい直し**: 下線の区間は解決した見た目の `look.underline` だけで選ぶ（`DEFAULT` の区間も対象・`!= default.underline` を外す）。既定だけの行でも下線か打ち消し線が要るときは、線を焼く道（`line_layout_with_strokes`）を通す。打ち消し線は `look.strike` が真なら焼く。上のテストは向きを逆に直す。
- **壊れたら落ちるテスト**: `viewbox_draw_underline_tests.rs`（余白 503）に「既定が下線つきのバルーンで、飾りの無い行に線が出る（読み戻し）」。打ち消し線は `viewbox_draw_decoration_tests.rs`（余白 141）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: ⑶ に触れる（ukadoc <https://ukatech.shillest.net/ukadoc-spec/descript/balloon/#font.underline> の原文は「常に下線をつける。0で無効/1で有効。」）。ただし直しは正典と完了 spec の求める振る舞いへ**戻す**向きで、⑶ の「変わると互換を崩す」には当たらない見込み。完了 spec `areka-P0-text-decoration-canon` が既定の下線の扱いを別に決めていないかは、設計の段で原文を引いて確かめる（10 節）。

#### 7. validrect を画像の大きさに収めず、ネットから来たバルーンで巨大な面を作ろうとする（`region.rs:281-287`）

- **何か**: `TextRegion::resolve` は left／top／right／bottom を解いた後、画像の寸 (width, height) に収めない。文字の面の寸は `actor_present.rs:244-247`（範囲の外）が `(right-left)×(bottom-top)` から決め、上限が無い。
- **困る場面**: ⑴ 300×200 の画像に `validrect.right,100000`・`validrect.bottom,100000` と書いたバルーンを選ぶと、10 万×10 万画素のスワップチェーン＋写しのテクスチャ 2 枚＋読み戻し 1 枚（`surface.rs:361-375`）を作ろうとする。作れなければデバイスの失敗になり、面が登録されないまま次のフレームでまた試す（毎フレーム `error!`）。16384 以下なら数 GB を確保する。⑵ `validrect.right,300` を幅 200 の画像が継ぐと、折り返しが 300 で起き、200〜300 の字は窓の外で見えない。縦書きの右から左では書き始めの角そのもの（`(right, top)`）が画像の外になり、字が 1 つも見えない。
- **いちばん小さい直し**: `resolve` の中で left／top は `max(0)`、right／bottom は `min(画像の寸)` にする（`region.rs` だけで済む）。もう 1 つの案は、配置は今のまま、面の寸だけを画像の寸で頭打ちにする（`actor_present.rs`＝範囲の外）。
- **壊れたら落ちるテスト**: `region_tests.rs`（余白 433）に「画像より大きい validrect が画像の縁で切られる」。
- **範囲の外への波及**: ア なし（形は変わらない）。イ 1 つ目の案は なし。2 つ目の案は `actor_present.rs`（`audit-emo-text-state-actor` の持ち物＝申し合わせが要る・要件 7.2）。
- **残す 3 種類**: **⑶ の議題**。ukadoc（<https://ukatech.shillest.net/ukadoc-spec/descript/balloon/#validrect.right>）の原文は「テキスト描画範囲の右位置。」「既定値: 不明」とだけ書き、画像をはみ出したときを書いていない。SSP は測っていない（記憶「ukadoc で読めない点は議題の前に SSP の MCP で測る」）。裁定「validrect は越えてはならない絶対上限・面を validrect より広げない」（2026-09-05）には反しない（画像の外は描けないので、狭める向き）。

#### 8. 選択肢の帯のはみ出しを、行の丈ではなく既定の字の高さと比べる（`viewbox_draw_render.rs:116-122,725-758`）PLAUSIBLE

- **何か**: 描き直しの範囲の丈（`viewbox_diff.rs:312-345` の `resident_rect`）は `run.size`＝行の中で最も大きい字の em。ハイライトの帯（`band_extent`・`band_offset`）も行の em から決まる（`choice.rs:231-262`）。ところが帯の超過分は `band_offset + band_extent − font.height`（既定の字の高さ）で求めている。
- **困る場面**: 既定の字の高さ 28 のバルーンで、選択肢の行が `\q[\f[height,14]はい,..]` だけのとき、帯の下端はおよそ 17、描き直しの範囲は 14＋ガード 1、計算上の超過は 2+15−28 で負＝0。マウスを乗せた塗りが下で切れ、外したときに塗りが残る（字のはみ出しの実測が一部を覆うだけ）。縦書きも同じ形。行の em が既定より大きいときは広く見積もるだけ（害は描き直しが広がるだけ）。
- **いちばん小さい直し**: `font.height` の代わりに、その行の丈（`resident_rect` と同じ `run.size` の行送りの軸）を渡す。説明（`:729`「em ボックス（font_height）」）も直す。
- **壊れたら落ちるテスト**: 関数を `pub(super)` にした純粋なテストなら `viewbox_draw_frame_render_tests.rs`（余白 352）、読み戻しなら `viewbox_draw_choice_hover_tests.rs`（余白 286）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

### 直したほうがよい

#### 9. 塊の折り返しは、配置のたびに塊の数の 2 乗の手間がかかり、毎フレーム走る（`layout_line_ops.rs:35-63`・`segment.rs:66-71`・呼び手 `layout_scan_glyph.rs:90,93`）

- **何か**: ⑴ `segment_advance_sum` は塊の頭ごとに `items` を先頭からたどり直す。⑵ `SegmentPlan::segment_starting_at` は、昇順で重なりの無い列なのに `iter().find` で探す。配置は場所ごとに毎フレームやり直す（`actor_present.rs:354` の `arrange_lines`）。
- **困る場面**: 全文の区切り（`Basis::Full`）は台詞全体の塊を持つ。和文 20,000 字（塊 約 7,000）を出し終えると、1 回の配置で約 7,000 万回たどり、約 2,400 万回比べる＝1 フレームに数十 ms（見積もり・測っていない）。
- **いちばん小さい直し**: ⑴ 配置のループが今の `items` の位置を覚えて（`Scan` に持たせる）、`&items[idx..]` を渡す（`pub(super)`・呼び手は 1 つ）。⑵ `binary_search_by_key(&glyph_index, |s| s.start)` の 1 行（既存の `glyph_index_ignores_line_break` が檻になる）。
- **範囲の外への波及**: ア なし。イ なし。範囲の外の申し送り: `actor_present.rs:616` の届いた字の区切りの腕は BudouX の区切りを毎フレームかけ直す（`audit-emo-text-state-actor` の持ち物・伝えるだけ）。
- **残す 3 種類**: 当たらない。研究: 効き目は前後で測る（10 節）。

#### 10. 見た目の表（`StyleTable::intern`）が線形探索で、字ごとに色を変える台本で 2 乗になる（`look.rs:286-298`）PLAUSIBLE

- **何か**: 新しい見た目を登録するたびに `looks` の全部と比べる（`position`）。表は `\c` か新しい話まで育つだけ。説明（`:287-288`）は「実際には数個」と書く。
- **困る場面**: 虹色の文字（字の前ごとに `\f[color,r,g,b]`）は字ごとに違う見た目になる。3 万字で約 4.5 億回の `TextLook` の比べ（名前の `Vec<String>` から比べる）＝数秒（見積もり）。
- **いちばん小さい直し**: `looks` の横に `HashMap` の索引を置く（鍵は見た目の欄・`height` は `to_bits()`）。`clear()` で一緒に空にする。決定論の檻は `look_tests.rs`（余白 461）に「1 万の色を登録して番号が往復する」（時間は比べない）。
- **範囲の外への波及**: ア なし（欄は私有・`starts_with`／`resolve` の形は変わらない）。イ なし。
- **残す 3 種類**: 当たらない。

#### 11. 書体と大きさの引きの表が、台本の値を鍵に際限なく育つ（`draw_catalog.rs:123,134`・`draw_metrics.rs:84,95`・`viewbox_draw_decoration.rs:279-282`）

- **何か**: 書体の候補の引き（`memo`・`warned`）、字幅の表（`cache`＝書体の鍵×字）、探りの書式（`probe_formats`＝鍵 1 つに `IDWriteTextFormat` 1 つ）、色のブラシ（`BrushCache`）は、どれも空にされない。鍵は `\f[name,…]` と `\f[height,N]`（N は正の有限なら何でも・`look.rs:757`）。持ち主の `ActorRender` は場所ごとに 1 度作られ、バルーン・デバイス・拡大率の変わり目までしか捨てられない（`\c` が空にするのは `LineLayoutStore` だけ・`viewbox_draw.rs:258-260`）。
- **困る場面**: `\f[height,%(random)]` を使う常駐のゴースト、入れてある書体を大きさ×太字で次々に見せるゴーストで、何日も動かすと、違う鍵ごとに COM の書式 1 つとその下の字幅の表が増え続ける。
- **いちばん小さい直し**: 型の中に大きさの上限を置き、越えたら既定の鍵以外を空にする（`probe_formats`・`cache`・`memo`）。`BrushCache` は `request_clear` で空にするのが安い。`warned` は「器の一生に 1 回」の記録の決め（`draw_catalog.rs:126-133`）なので残す（失敗のときしか育たない）。檻は `draw_metrics_styled_tests.rs`（余白 383）・`draw_catalog_tests.rs`（余白 385）に「上限を越えると作り直しの数が増え、中身は同じ」。
- **範囲の外への波及**: ア なし（中だけ）。イ なし。
- **残す 3 種類**: `BrushCache` の「上限を設けない」は理由つきの説明（`viewbox_draw_decoration.rs:279-282`）があるが `shortcut:`／`ponytail:` の印ではない＝⑵ には当たらない見込み。上限の値は議題。

#### 12. 作れなかった探りの書式を覚えず、字ごとに `error!` を出し直す（`draw_metrics.rs:246-266,279-295,349-358`）PLAUSIBLE

- **何か**: `try_create_format` が失敗すると `error!` を出して縮退の値を返すが、何も覚えない。同じ鍵の次の字で `CreateTextFormat` をまた試し、また `error!`。字幅の測りの失敗も同じ。
- **困る場面**: DirectWrite が毎回断る鍵があれば、字 1 つごと・配置し直すたびに `error!` が出て、記録があふれる（完了 spec の要件 13.4「記録があふれない」に反する）。高さ ≤ 0・inf・NaN は `look.rs` で先に落ちる。とても大きな高さ（`\f[height,1e30]`）を DirectWrite が断るかは確かめていない（10 節）。
- **いちばん小さい直し**: 失敗した鍵を覚え（`HashMap<FontKey, Option<IDWriteTextFormat>>` か失敗の集合）、記録は 1 回にする。指摘 11 の上限と同じ表で持てる。檻は `draw_metrics_styled_tests.rs`（失敗の鍵で 2 回呼んで `error!` が 1 件）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

#### 13. 先渡しの空回しが、印の字（`\![*]`・`%*`）を「内容を足した」合図に数えない（`lookahead.rs:254-262`）

- **何か**: `added` に入れる合図は `Text`・`Choice`・`NewLine`・`Cursor` だけ。`state.rs:636-644` は印の合図（`CueCommand::Custom` で `is_marker_carrier` が真）でも印の字を 1 つ積む。「字を足す合図」の一覧を 2 か所で持っていて、もう食い違っている。
- **困る場面**: `\0\![*]\1かき`（または `\c` の直後の区間が印だけ）で、本体側の場所が `added` に入らず全文が控えられない。本番の区切りは「先渡しが無い」となり、`warn!` を 1 件出して届いた字の区切りへ落ちる。`lookahead_tests.rs` に印の検体は 0 件（grep）。
- **いちばん小さい直し**: `matches!` の後に `|| crate::inline_marker::is_marker_carrier(&cue.command)` を足す（`pub fn`）。根から直すなら、状態を進める前後で場所の `items` の長さを比べる（一覧を持たない）。檻は `lookahead_tests.rs`（余白 122・約 25 行）。
- **範囲の外への波及**: ア なし。イ なし（`inline_marker.rs` は呼ぶだけ）。
- **残す 3 種類**: 当たらない。

#### 14. 面の外にある行の下線まで、面の縁へ寄せて描く（`underline.rs:93-104`・`viewbox_draw_render.rs:321`）PLAUSIBLE

- **何か**: `pull_inside` は線がどれだけ外にあっても `lo < 0`・`hi > limit` の枝で縁へ寄せる。呼び手の門は `index >= window.first_visible_line` だけ。全面の描き直しのフレームでは、見える窓の最初の行より後の行は全部描く対象に入る（`viewbox_diff.rs:134-139`）ので、利用者が手で上へ送っているときの、面の下にはみ出した行も入る。
- **困る場面**: 10 行のバルーンで 5 行が見えていて、利用者が上へ送って先頭が 0 行目。7 行目に `\f[underline,1]` がある。書式の作り直し・DPI の変わり目などで全面を描き直すと、7 行目の線（上端 130・面の下端 122）が [120, 122] へ寄せられ、4 行目の字の上に知らない線が出る（7 行目の字はクリップで消える）。縦書きの右から左で左に出た列も x＝0 へ寄る。裁定「最下行で面に収まらないときだけ面の内側へ寄せる」（2026-10-16・記憶「下線はベースラインの直下」）に反する（7 行目は見えている最下行ではない）。
- **いちばん小さい直し**: `pull` の頭で、線が丸ごと外なら（`lo > limit || hi < 0`）そのまま返す。檻は `underline_tests.rs`（余白 791）に横書きと縦書きの 1 本ずつ。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない（裁定に沿う直し）。

#### 15. 面の内側へ寄せた最下行の下線が、スクロールの後に元の場所に残る（`viewbox_draw_render.rs:321-325`・計画の側 `viewbox_diff.rs`）PLAUSIBLE

- **何か**: 最下行の線は `pull_inside` で [H−t, H] へ寄せて字に重ねて塗る。次の行が出て p だけ送ると、ずらしで寄せた線ごと上へ動く。前の最下行は指紋が変わらないので描き直されず、描かれるのはガード込みの露出の帯 [H−p−g, H] の中だけ。
- **困る場面**: 線の太さ t がガード g（1 画像画素）より太い大きな字で、寄せた線の [H−t−p, H−p−g) が前の最下行の字の下に残る。見本（オラクル）は下線を描かないので、見本との比べの檻には映らない。
- **いちばん小さい直し**: ずらしが 0 でないフレームは、前の最下行も描き直しの対象に入れる（計画の側・`viewbox_diff.rs`）。檻は `viewbox_draw_underline_tests.rs`（余白 503）に「寄せた最下行の後に送り、1 フレームで全部描いた面とバイトで比べる」。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない（最下行だけ寄せる裁定どおり）。

#### 16. 空の canvas に、毎フレーム面全体の描き直しと表示をする（`viewbox_diff.rs:134-148`・`viewbox.rs:313`）

- **何か**: 「前の行が空」は「まだ描いていない」と「空の canvas を描いた」の 2 つの意味を兼ねていて、前者として面全体を描き直す。commit は空の canvas の行（＝空）を覚えるので、次のフレームもまた全体になる。フレームの駆動は字の出待ちのあいだ起こし続け（`crates/areka/src/emo2_boot/frame/scale_text.rs:312-320`）、表示は面のある場所を空でも表示する（`actor_present.rs:84-90`）。
- **困る場面**: `\_w[3000]こんにちは`、または `\c` の後の待ち。3 秒のあいだ毎フレーム（60 fps で約 180 回）、面全体の消去・裏表の入れ替え・commit・表示・当たり判定の作り直しが走る。
- **いちばん小さい直し（状態の形を変える・0 フレーム）**: 前の行を `Option<Vec<CommittedLine>>` にする（None＝中身が分からない＝全体・commit はいつも `Some`）。空→空は `NoChange` になる。檻は `viewbox_plan_commit_tests.rs` に「空の canvas を commit した次の plan が NoChange」。
- **範囲の外への波及**: ア なし（`pub(crate)`・呼び手は viewbox とテストだけ）。イ なし。
- **残す 3 種類**: 当たらない。

#### 17. 消去（FullClear）のフレームは、同じフレームで見えるようになった字を描かない（`viewbox.rs:227-229`・`viewbox_draw_render.rs:143-156`）

- **何か**: `\c` とその後の字が同じフレームで見えるようになるとき（`\c\_qえお\_q`・遅れを取り戻すフレーム）、そのフレームは消して入れ替えるだけで何も描かない。見本は「えお」を描く＝完了 spec の要件 R4.5（全体の描き直しと画素が一致）がそのフレームだけ崩れる。続けて出す字が無いと、フレームの駆動が起こし直さない（`scale_text.rs:312`）ので、別の理由で起きるまで空のまま残りうる（PLAUSIBLE）。
- **いちばん小さい直し（0 フレーム・裁定「1 フレーム遅らせる解は取らない」に沿う）**: 消去の要求が立っている間の plan は、全域の Update（全面の消去の上に見える行を描く）を返し、commit（Update）で要求を下ろす。`stats.full_clears` は消去に由来するフレームを数え続ける（範囲の外の actor のテストを直さずに済む）。空の canvas のときだけ今の FullClear を残す形もある。
- **範囲の外への波及**: ア なし。イ 消去の変種を丸ごと消すなら `tests/viewbox_scroll_test.rs`（8 か所）ほか範囲の外のテストに及ぶ＝変種を残す形なら なし。範囲の中で期待が変わる: `viewbox_plan_commit_tests.rs:181-192,234-262,509-519`。
- **残す 3 種類**: **⑴ の議題**。コードの説明（`viewbox_draw.rs:256-257`「透明フラッシュは FullClear の 1 フレームのみ」）はこの 1 フレームを許している。裁定「1 フレーム遅らせる解は取らない・状態の持ち方を変えて 0 フレームで解け」（2026-09-13）とどちらが勝つかを確かめる。

#### 18. 全域の描き直しは、面のはるか外の行まで全部描く（`viewbox_diff.rs:137-139`・`viewbox_draw_plan.rs:92-93`）

- **何か**: 全域の道の描く行は「見える窓の最初の行より後の全部」で、面と重なるかを見ない（差分の道は `:199-212` で見ている）。
- **困る場面**: 1000 行の話を先頭まで送り戻すと、ホイールの 1 段ごと（古い側への送り＝全域へ落ちる・`viewbox.rs:244-262`）に約 1000 行の配置と描画（うち約 999 はクリップで捨てる）。指摘 31 の食い違いの確かめも 1000×1000＝100 万回になる。
- **いちばん小さい直し**: 全域の道でも、はみ出し込みの `resident_rect(...).is_some()` で絞る。面のすぐ下の行の上のはみ出しが面へ入る場合を落とさないため、`degrade_if_needed`／`full_domain_update` にはみ出しを渡す（`viewbox_draw_plan.rs` の形と、`viewbox_draw_render.rs:136` の呼び出し 1 つが変わる）。檻は `viewbox_dirty_tests.rs` に「面の下の行は外れ、上のはみ出しが面へ入る行は残る」。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

#### 19. `\_l` で跳んだ直後の最初の字は、validrect の外にも置かれる（`layout_scan_glyph.rs:83,121,126,134`・テスト多数）

- **何か**: `\_l` は行を閉じるので、跳んだ直後の字は「行頭の 1 字」として、折り返し基準（soft）と描画範囲の遠辺（hard）の両方の比べから外れる（行頭の 1 字は無限の折り返しを避けるための例外）。説明 `layout.rs:36-39` の「`\_l` で跳んだ後も描画範囲が勝って折り返す」は、2 字目からしか成り立たない。
- **困る場面**: validrect の右 200 で `\_l[195,]あ` は字が 195〜205 に置かれ、validrect の外へ出る。`\_l[150,]` と折り返し基準 100 なら、最初の字は基準を越えて置かれる。テストはこの振る舞いを固めている（`layout_cursor_wiring_tests.rs:484`＝380〜390 で右 360 の外、`layout_cursor_vertical_tests.rs:125,151,440`、`layout_cursor_vertical_canon_tests.rs:294`、`layout_cursor_tests.rs:396`）。
- **いちばん小さい直し（裁定しだい）**: 描画範囲を勝たせるなら、例外を「行内の位置が行頭のとき」だけにする。`\_l` を字義どおりにするなら、説明 `layout.rs:36-39` を事実に合わせる（見える範囲は描く側のクリップの持ち物と書く）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: **⑴ と ⑶ の議題**。裁定「validrect＝文字を validrect の外に置かない」（2026-09-05）と、完了 spec `cursor-tag-canon` の R2.6「字義どおり用い、内側への自動的な寄せを行わない」がぶつかる。ukadoc（<https://ukatech.shillest.net/ukadoc-spec/list/sakura-script/#_l-x-y>）は「バルーンの文字描画開始点…からのピクセル単位座標」とだけ書く。

#### 20. 台本の `\f[height,N]` に上限が無い（`look.rs:715-761`）

- **何か**: 「正の有限」だけを確かめる。
- **困る場面**: `\f[height,1e38]` は通る（f32 の最大は約 3.4e38）。行送り＝高さ＋2 なので、4 行目の位置が inf にあふれる（指摘 1 の守りが位置は守るが、DirectWrite には 1e38 の em が渡る）。`\f[height,100000]` も通り、指摘 11・12 の表を育てる。emo-text のどこにも頭打ちは無い（grep）。
- **いちばん小さい直し**: `apply_height` の出口の確かめ（`:757`）で上限をかける（値は議題）。檻は `look_font_tag_value_tests.rs`（余白 569）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: **⑶ の議題**。ukadoc（<https://ukatech.shillest.net/ukadoc-spec/list/sakura-script/#f-height>）の原文は「これテキストを指定した文字サイズに変更する。」で、上限を書かない。SSP の上限を測ってから決める。記憶「ゴーストの台本の大きさを甘く見ない・上限で断らず」は台本の長さの話で、字の大きさには直に当たらない。

#### 21. `\f` の `cursor*`・`anchor*` を前方一致で引くので、書き損じのキーが「無視」扱いになる（`look.rs:545-547`）

- **何か**: `key.starts_with("cursor") || key.starts_with("anchor")` を「他の持ち物」として扱う。
- **困る場面**: `\f[cursorcolour,red]`・`\f[anchorx,1]` は `Note::Unowned` になり、`state_decoration` は `debug!` だけ（既定のレベルでは無音）、`check_script` は `unknown_tag` でなく `ignored` を返す。
- **いちばん小さい直し**: ukadoc の `\f` の一覧から `cursor*`（10）と `anchor*`（16〜17）のキーを数え上げて `UNOWNED_KEYS` に入れる。`look_font_tag_tests.rs` §10 の行の名前（`cursornotselectfontcolor`・`anchorvisitedpencolor` ほか）を ukadoc と照らす。
- **範囲の外への波及**: ア なし。イ なし（`crates/areka` の `check_script_judge.rs:135`・`strict_judges.rs:34` は `classify_font_tag` を呼ぶだけで形は変わらない。出る判定の語は変わる）。
- **残す 3 種類**: **⑴ の議題（要件の言い回し）**。完了 spec の R2.5 は `cursor*`／`anchor*` の言い回しで書き、R2.6 は「43 形のいずれでもない」キーを不明（`warn!`）とする＝要件どうしが食い違う。

#### 22. 「本番の呼び出しはここだけ」の檻が、本番の呼び出しを見落としている（`draw_format_metrics_tests.rs:683-739,849-891`）

- **何か**: ⑴ `direction_recipe_factory_has_exactly_one_call_site_inside_create_text_format` は `draw.rs` だけを読むが、本番の 2 つ目の呼び出しが `draw_metrics.rs:261`（`probe_format_for`）にあり、今も緑＝名前と主張が誤り。⑵ 書体名の入口の檻は入口を 2 つと書くが、3 つ目 `viewbox_draw_decoration.rs:187`（区間ごとの `SetFontFamilyName`）を読まない。
- **困る場面**: ⑴ 3 つ目の呼び出しを足しても赤にならない。⑵ そこへ `format!("@{}", ..)` を書いても赤にならない。
- **いちばん小さい直し**: ⑴ `draw_metrics.rs` も数える（`probe_format_for` の中に 1 つ）か、名前と説明を直す。⑵ `viewbox_draw_decoration.rs` を別の小さな表で `@` の檻に入れる（`DRAW_FACADE_SOURCES` に足すと別の一致の檻が赤になる）。説明を「入口 3 つ」に。`draw_format_metrics_tests.rs` は余白 78＝⑴ の約 5 行は入る。⑵ が入らなければ `draw_metrics_styled_tests.rs` へ。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

#### 23. 守っているつもりの分岐に檻が無い（壊しても緑のまま）

- **場所と何か**（どれも「この行を壊しても全部緑」の形を確かめた・PLAUSIBLE は読みで詰めた）:
  - `viewbox_draw_frame_render_tests.rs:477-497`: 太字・斜体だけが変わり配置が変わらないときの描き直し（`viewbox_draw_plan.rs:39-48` の `NoChange` の腕の `rebuilt`）。テストは 2 回目の描画の戻り値と描いた回数を見ない＝腕をいつも `NoChange` にしても緑（古い画素が残る）。
  - `viewbox_draw_plan.rs:59-66`（計画の食い違い → 全体）: 結線のテストが無い。`frame_render_tests.rs:503` の説明「`font_change_degrades_to_full_domain_redraw` が担保する」は誤り（そのテストは `rebuilt` の枝を通る）。
  - `viewbox_diff.rs:43-45`（同じ番号の行の位置が動いた）と `:55` の高さの半分（`nh < ph`）: 後ろ向きの縮みの 2 つの枝。数の減りと幅の縮みしか試していない。
  - `viewbox_draw_render.rs:321` の `index >= window.first_visible_line`: 下線とスクロールを一緒に描くテストが 0 本（`actor_underline_present_tests.rs` にスクロール 0 件）。外すと窓の 1 つ上の行の線が先頭の行へ寄る。
  - `segment.rs:143-150`: `\_l`（CursorMove）でも区切りを切る腕。説明（`:3,12-14,36,53,98-100`）にも書かれず、テストは CursorMove を入れない（layout のテストは手組みの区切りしか使わない）。このため `layout_scan_glyph.rs:135-147` の `over_hard && in_segment` の腕は本番の区切りでは届かない（届くのは手組みの `layout_hard_limit_tests.rs:440` だけ）。
  - `look_font_tag_tests.rs:557-572`: `classify_font_tag` の表に `default`・`disable` の行が無い。説明 `:413-419` は「`\f[default]`／`\f[disable]` の腕は本番の道から届かない」と書くが、`classify_font_tag`（`look.rs:438-452`）は `crates/areka` の `check_script` と厳しい読みから呼ばれ、届く。説明を信じて腕を消すと、正典のタグが `unknown_tag` と報告される。同じ古い主張が `state_decoration.rs:296-297,334-336`（範囲の外）にもある。
  - `layout_cursor_tests.rs:529-571`: 「実導出成功（Px/Em/Lh）は警告しない」と書くが Px しか通さない。%・`@`・正しい軸の centerx は本番の配線で無音を確かめていない。
- **困る場面**: 守っているつもりの分岐が壊れても赤にならない。
- **いちばん小さい直し**: 各テストに 1〜数行の確かめを足す（戻り値と描画の回数・GPU 無しの `degrade_if_needed` の単体・後ろ向きの縮みの 2 本・下線と送りを一緒に描く 1 本・`[字,字,CursorMove,字]` の区切り・表の 2 行＝配列の長さ 14→16・item の列に % と `@` を足す）。説明を直す。
- **範囲の外への波及**: ア なし。イ なし（`state_decoration.rs` の古い説明は伝えるだけ）。
- **残す 3 種類**: 当たらない。

#### 24. 失敗しえないテスト・名前と違うことを確かめるテスト（偽の安心）

- **場所と何か**:
  - 純粋な関数を同じ値で 2 回呼んで比べる: `layout_visible_window_tests.rs:214,236`・`layout_wrap_tests.rs:670`・`wrap.rs:163` の半分・`viewbox_dirty_tests.rs:567-578`（名前の「スクロールに依らない」を見ていない）・`viewbox_axis_tests.rs:107-114`。
  - 自分と同じ値どうし・作った直後の型の比べ: `canvas.rs:481`（`TextEffects::default()` どうし）・`canvas.rs:709-721`・`canvas.rs:511,749-755`（いつも真の `is_translation_only`）。
  - 見本との比べが空の面でも通る・名前のスクロールを確かめない: `viewbox_draw_oracle_regression_tests.rs:81-135,141-198,224-262`（不透明の画素の数の守りが無い）。
  - 全体の描き直しを切り分けていない: `viewbox_draw_frame_render_tests.rs:394-450`（字 10→14 で全行が動くので、ふつうの差分でも同じ数になる）PLAUSIBLE。
  - 拡大率 ≠ 1 の比べが墨の外側の 2 辺しか見ない: `viewbox_draw_live_diff_tests.rs:289-297`（中の行を描き残しても緑）PLAUSIBLE。DPI 追従の本番の道を見本と比べる唯一の檻。
  - 自分の定数だけを確かめる: `layout_cursor_center_origin_tests.rs:493`（実際でなく期待の値を積む）・`:371-379`・`:367-370`。
  - 名前の主張を見ていない: `region_tests.rs:467`（`resolve` は k を受けないので赤になりようがない）・`layout_styled_tests.rs:641`（説明が「ids を無視しても緑」と認める）・`draw_catalog_tests.rs:268-284`（`assert_ne!` は直後の `assert_eq!` から従う）・`draw_metrics_styled_tests.rs:281-288`（字面の確かめだけ・同じことは `:402-445` が振る舞いで見ている）・`viewbox_draw_live_diff_tests.rs:550-604`（違う canvas が違うバイトになるだけ）・`viewbox_draw_frame_render_tests.rs:40-43`（`>= 1`・1 行なら `== 1`）。
  - 見本が本番と同じ規則で測っていない: `draw_format_metrics_tests.rs:325-347`（列の組み立てを通さない＝規則の崩れを見られない・説明は「同じ規則で」と書く）。
  - 私有の確かめが私有でなくても通る: `layout_cursor_overflow_tests.rs:439-442`（`pub(super) fn` にも一致）。
  - 本番の式をテストの中で組み直す: `viewbox_draw_line_gap_band_tests.rs:726-732`（`em.max(em + 2.0)` は `em + 2.0`）。
  - 自分のメッセージより先に桁あふれで落ちる: `viewbox_draw_choice_hover_tests.rs:464-466`（u32 の引き算）。
  - 縦書きの右から左の純粋なテストの canvas が面の外にある: `viewbox_test_support.rs:30`（origin (0,0) で最初の列が x∈[-10,0]・`viewbox_dirty_tests.rs:283-327` の縦の部分は 1 画素の切れ端でしか見ていない）。
- **困る場面**: 本物の誤りが入っても緑のまま。数だけ多く、直すときの手間だけが増える。
- **いちばん小さい直し**: 消す。見本の比べ・拡大率の比べ・全体の描き直しの切り分けは、不透明の画素の数とスクロールの有無の確かめ・行ごとの帯の墨の確かめ・ふつうの差分が全体より少なく描く入力に直す。origin は `Origin::new(None, None)`。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

#### 25. テストの組み立て部品が多くのファイルに写してある

- **場所と何か**（読み手の報告を進行役が grep で確かめた）:
  - 見本と viewbox の対の描画と読み戻し: 7 か所（`viewbox_draw_live_diff_tests.rs:150-221,229-299,683-725`・`viewbox_draw_oracle_regression_tests.rs:84-134,142-197,225-261`・`viewbox_draw_scroll_retain_tests.rs:175-228`）。行の比べのループ 3 か所（`live_diff:713-720`・`oracle_regression:122-129,183-190`）。同じ台本の組み立て 2 か所（`oracle_regression:70-78`・`live_diff:665-676`）。
  - GPU の組み立て（`make_dispatcher_and_compositor`＋`attach`）・`glyph_items`・`opaque_count`: `draw_oracle_tests.rs:41-99,115,150` と `viewbox_draw_test_support.rs:38-96,138,168`。
  - 選択肢の canvas: `viewbox_draw_choice_hover_tests.rs:27-52,166-200,298-325,422-451`・`viewbox_draw_decoration_tests.rs:412-443`・`viewbox_draw_color_emoji_tests.rs:244-271`。帯の画素の数え: `choice_hover:130,265,385`・`viewbox_draw_vertical_band_readback_tests.rs:45`・`viewbox_draw_vertical_band_scroll_tests.rs:46`・`decoration_tests.rs:224-251`・`color_emoji_tests.rs:198-207`。塗りの色 `[25,25,105,255]` が 6 か所（`choice_hover:203,326,420`・`vertical_band_readback:42`・`vertical_band_scroll:42`・`color_emoji:290`）。テストのファイルどうしが互いに取り込んでいる（`line_gap:11,13`・`choice_hover:5`）。
  - 書体の確かめ: `require_yu_gothic`（`viewbox_draw_decoration_face_tests.rs:278-288`・`draw_metrics_styled_tests.rs:419-426,472`）・`face_of`（`draw_metrics_styled_tests.rs:450`）と `read_family_name`（`draw_format_metrics_tests.rs:237`）・`PASTA`（`draw_metrics_styled_tests.rs:340,406,584`）。
  - 配置の部品: `rect_of`（`layout_cursor_center_origin_tests.rs:151`・`layout_cursor_vertical_tests.rs:42`・`layout_cursor_vertical_canon_tests.rs:82`・`layout_cursor_overflow_tests.rs:81`・`layout_styled_tests.rs:377`）・`glyph()` 5 か所・`cursor_px` 4 か所・`newline()` 2 か所・同じ `TextRegion::resolve(..)` の組み立てが `layout_cursor_tests.rs` に 12 回。
  - 領域の部品: `model(origin, wordwrap, validrect)` が `region_tests.rs:14` と `region_vertical_canon_tests.rs:114` で一字一句同じ（`canvas.rs:381` にも）。`IMAGE`＝(400,224) が `region_tests.rs:11`・`region_vertical_canon_tests.rs:84`・`layout_test_support.rs:10`。`writing.rs:329` の `model` は `writing_decision_tests.rs:51` に含まれる。
  - viewbox の部品: `broken_lines`（`viewbox_test_support.rs:40-49` と `layout_test_support.rs:63` が同じ）・`model_rect`・`geo_model`・`build`／`canvas_for`（`viewbox_draw_test_support.rs:143`・`viewbox_test_support.rs:52`・`canvas.rs:403`）・住人の組み立て（`viewbox_choice_marker_tests.rs:16-62` と `viewbox_style_fingerprint_tests.rs:21-69`）。
  - 同じファイルの中: `viewbox_draw_frame_render_tests.rs:110-117` は同じファイルの `multiline_items`（`:317-326`）の写し。PNG の準備（`viewbox_draw_png_dump_tests.rs:153-196` と `:385-426`）。
- **困る場面**: 型に欄を 1 つ足すたびに 10 か所近くを直す。写しの一部だけが古くなる。
- **いちばん小さい直し**: 今ある支えのファイル（`viewbox_draw_test_support.rs`・`viewbox_test_support.rs`・`layout_test_support.rs`・`draw_test_support.rs`・`cursor_tag_test_support.rs`）へ寄せて `pub(super)`／`pub(crate)` で引き、写しを消す。新しいファイルは要らない（要件 7.3）。`render_pair`・`diff_rows` だけで約 150 行、選択肢の canvas で約 70 行減る見込み。
- **範囲の外への波及**: ア なし。イ 範囲の中で閉じる。範囲の外の写し（`ink_in_band` の `actor_scroll_retain_tests.rs:101`・`actor_clear_atomicity_tests.rs:195`、`with_log_cage` の `actor_runtime_frame_tests.rs:26`・`sink.rs:246`、PNG の書き手の `tests/choice_fixture_test.rs:610-`・`tests/emo2_fixture_e2e_test.rs:691-`）は触らない（要件 7.9）。
- **残す 3 種類**: 当たらない。

#### 26. 本番のコードの写し（同じ規則が 2 か所）

- **場所と何か**:
  - 既定の書体の名と高さ: `look.rs:69,74`（`UKADOC_DEFAULT_FONT_*`）と `draw.rs:157-161`（`DEFAULT_FONT_*`）。両者を揃えるだけのテスト `look_tests.rs:70-73`。
  - `device_err`（`draw.rs:830` と `viewbox_draw.rs:331`・同じ文）と `none_err`（`draw.rs:817`（テストだけ）と `viewbox_draw.rs:318`）。
  - `degraded_advance`（`draw_metrics.rs:408-414`）は `FixedMetrics::advance`（`layout.rs:215-221`）の写し（説明も「同じ式」と書く）。
  - `\_l` の実効位置の下見（`layout_scan.rs:230-245`）は、保留の改行と保留のカーソルの適用（`:350-381`）を手で写している（指摘 1 の式もここで 2 か所になる）。
  - 書字方向の軸の名（`region.rs:225-230` の `inline_axis_name`）と軸の割り当て（`:318-327`）。
- **困る場面**: 片方だけ直すと食い違う（指摘 1 の守りを片方にだけ入れる、など）。
- **いちばん小さい直し**: `draw.rs` の定数は `crate::look::UKADOC_*` の別名にして揃えのテストを消す（`draw_format_metrics_tests.rs:771-779` の字面の檻を直す）。`device_err`／`none_err` は `draw` の側を `pub(crate)` にして `viewbox_draw` から引く。`degraded_advance` は `FixedMetrics.advance(..)` を呼ぶ。下見は 2 つの適用の関数を写しの局所変数へ呼ぶ（行送りは `peek()` の値を渡す）。軸の名は割り当てと同じ `match` から返す。
- **範囲の外への波及**: ア なし。イ なし（`crates/areka` が使うのは `draw::DEFAULT_BALLOON_BACKGROUND` だけ＝grep）。
- **残す 3 種類**: 当たらない。

#### 27. PNG の書き出しの既定の出力先がソースの木の中（`target\` の外）（`viewbox_draw_png_dump_tests.rs:152,384`・説明 `:17,380`）

- **何か**: 出力先の環境変数が無いと `"."` に書く。`cargo test` のカレントは `crates/areka-emo-text`。説明の「CARGO_TARGET_TMPDIR 相当のカレント」は誤り。
- **困る場面**: 決まり「実機の根・一時フォルダはワークツリーの `target\` の下だけ」（要件 6.4）に反し、ソースの木に PNG が残る。
- **いちばん小さい直し**: 既定を `target` の下（例 `<target>/areka-diag`）にして `create_dir_all` する。target の求め方は `sample-ghost-kit` の `devroot.rs:127-141` と同じ式か、`CARGO_MANIFEST_DIR` からの相対。単体テストでは `CARGO_TARGET_TMPDIR` が無いので使えない。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

### できれば

#### 28. 同じことを 2 度確かめているテスト

- **場所**:
  - 配置: `layout_segmented_tests.rs:310` と `layout_wrap_tests.rs:47`。
  - カーソル: `layout_cursor_vertical_tests.rs:96,196,309,411,534` の 5 本は `layout_cursor_vertical_canon_tests.rs:265,327,447,369,517` に値ごと含まれる（canon の頭の「重複させない」は誤り・約 140 行）。`layout_cursor_tests.rs:150-179` と x だけの半分 `:245-275` は `layout_cursor_wiring_tests.rs:448-463,427-446`（約 60 行・このファイルの余白 158 を空ける）。`cursor_tag_tests.rs:32,48,80,114,128,142,155,289` と `cursor_tag_resolve_tests.rs:537,686` は `cursor_tag_resolve_tests.rs:374` の全表に含まれる（約 230 行・`:846-850` の一覧も直す）。`cursor_tag_tests.rs:376-384,438-445`。`layout_cursor_center_origin_tests.rs:631-657,590-610`。
  - 領域: `region_tests.rs:264,287,79,97,275,300,314,327,338,389` は `region_vertical_canon_tests.rs` の表と重なる。`region_inline_limit_tests.rs:191` は `:207` の後半と同じ。`writing.rs:353-401` の内側のテストは `writing_decision_tests.rs:160,197,402` に含まれる（約 45 行）。
  - 見た目: `look_tests.rs:497,513,528,344,477`・`color_tests.rs:367,379,396,356,407,203`・`look_font_tag_tests.rs:612`。
  - viewbox: `viewbox_plan_commit_tests.rs:24-64,68-99,455-506`・`viewbox_dirty_tests.rs:519-564`・`viewbox_choice_marker_tests.rs:68-105`（約 200 行）。
- **困る場面**: 振る舞いを変えるとき、同じ期待を何か所も直す。
- **いちばん小さい直し**: 弱い方を消し、固有の 1〜2 行だけを強い方へ移す。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: `look_tests.rs`・`color_tests.rs` の「calibration」の組は、完了 spec の R15.3／R15.6 が名指しで求めた作法＝**⑴ に触れる**。まとめるかを討議で確かめる。

#### 29. 古くなった説明（コードとテスト）

- **コード**:
  - 配置: `layout.rs:80-86`（行の丈は「字の高さ」→ 行の中で最も大きい em・行内の始まり → 最初の字の位置・空行の幅は出ない）・`layout.rs:36-39`（指摘 19）・`layout.rs:344-345` と `layout_scan_glyph.rs:112-114`（指摘 5）・`layout_scan.rs:160-163`（行送り 13 の古い数＝今は 124 と 112・`layout_cursor_order_tests.rs:182-183` と食い違う）・`layout_scan.rs:246-247`（分ける前の「200 行」）。
  - カーソル: `cursor_tag.rs:90`（`font_height` は「タグを書いた時点での文字高さ」→ 実際はその場所の既定の高さ・完了 spec `areka-P0-text-decoration-canon` の design.md:51 の決めどおり）・`cursor_tag.rs:106-107`（指摘 2）。
  - 領域と書字方向: `writing.rs:14`（`actor.rs:153` → `actor_decoration.rs:194`）・`region.rs:257-258,265,407`（`actor.rs` → `actor_attach.rs:325,156`）・`region.rs:81`（「現行 1.0」→ 同じファイル `:16`「1.0 とは限らない」と食い違う・DPI 追従が基本設計）。
  - 区切り: `segment.rs:3,12-14,36,53,98-100`（指摘 23）。
  - 見た目: `canvas.rs:24-25,254-255,260,291`（「M1 唯一の住人は GlyphRun」→ Choice も描く）・`canvas.rs:176-178,263-264`（ハイライト・hover・色の入れ替えは「後続タスク」→ 済み）・`canvas.rs:32-34`（下線は描き手の既定任せ → 2026-10-16 から areka が書字方向ごとに自分で引く）・`canvas.rs:71,138,153`（開発者がやめた語「α 後」）・`look.rs:287-288`（指摘 10）。
  - 描画: `draw.rs:3`（子モジュールは 3 つ）・`draw.rs:50-51`（字幅の表は鍵×字）・`draw.rs:137-140`・`draw.rs:195-199,226-227`（候補は「先頭だけ」→ `FontCatalog::pick` が全部を使う）・`draw_metrics.rs:115-123`（呼び手の手書きの一覧が欠ける）・`viewbox_draw.rs:15`（`plan` → `plan_with_overhangs`）・`viewbox_draw.rs:134-135`（Choice も入る）・`viewbox_draw.rs:286-294`（`render` のテストの呼び手の一覧が 4 ファイル欠ける）・`viewbox_draw_plan.rs:6-9,22-24`（食い違いの引き金は 3 つ）・`viewbox.rs:17,20-21,118`（`layout.rs:130–137` → `:281-290`）・`viewbox.rs:185-186`・`viewbox_diff.rs:32-33`（「全行の行内の始まりは同じ」は `\_l` で誤り＝理由だけ誤り）・`viewbox_diff.rs:85-86,262`・`viewbox_draw_render.rs:50-51,486-487,729`・`viewbox_draw_decoration.rs:13-15,64,160`。
- **テスト**: `layout_cursor_tests.rs:691,801,834`・`layout_cursor_vertical_canon_tests.rs:33-35,594`・`layout_cursor_overflow_tests.rs:374,382,441`・`cursor_tag_resolve_tests.rs:759-761`（コードは `layout_scan*.rs`・`layout_line_ops.rs` へ移った）・`layout_styled_tests.rs:587`（読むファイルは 5 つ）・`layout_test_support.rs:38`（中身の無い見出し）・`region_inline_limit_tests.rs:22-23,32-33`（「毎フレーム」→ 装着と入力の変わり目だけ）・`region_vertical_canon_tests.rs:33-35`（行と値）・`writing_decision_tests.rs:399-400`・`draw_format_metrics_tests.rs:64-79`（名前と文）・`viewbox_draw_frame_render_tests.rs:503`（指摘 23）・`viewbox_draw_vertical_band_readback_tests.rs:5-6`（`choice_hover_tests` は今 714 行）・`viewbox_axis_tests.rs:83,116`・`viewbox_test_support.rs:11`・`viewbox_draw_test_support.rs:110-120`（もう無い関数の説明が混ざる）・`look_font_tag_tests.rs:413-419`（指摘 23）。字の継ぎ目の抜け `cursor_tag_resolve_tests.rs:862`・`layout_cursor_order_tests.rs:204`・書き損じ `layout_cursor_order_tests.rs:178`（`[あ\0]` → `[あ@0]`）。
- **困る場面**: 説明を信じた読み手が誤った前提で直す（指摘 5・23 はこの型）。
- **いちばん小さい直し**: 説明を今のコードに合わせる。作業の経緯の段落は消す。`doc/COMPAT_ARCHITECTURE.md` を名指す行（`canvas.rs`・`color.rs`・`draw.rs`・`look.rs`・`region.rs`）の名前は変えない（`doc-compat-architecture-rename` の持ち物）。
- **範囲の外への波及**: ア なし。イ なし（`state_decoration.rs:296-297,334-336`・`actor_decoration_frame_tests.rs:115` の古い文は伝えるだけ）。
- **残す 3 種類**: 当たらない。`cursor_tag.rs:90` は説明だけ直す（振る舞いを ukadoc の「1em＝タグを書いた時点での文字高さ」<https://ukatech.shillest.net/ukadoc-spec/list/sakura-script/#_l-x-y> へ寄せるのは ⑴（完了 spec の決め）と ⑶ に当たる＝変えない）。

#### 30. 要らない小さなコード

- **場所と何か**（「使われていない」はどれもツリー全体を grep した）:
  - `LayoutEngine::layout_with_cursor_warn`（`layout.rs:402`）: 本番の呼び手 0。テスト 3 か所（`layout_cursor_tests.rs:481,555`・`layout_cursor_wiring_tests.rs:350`）を `layout_styled` に替えれば消せる（説明のリンク `layout.rs:257,370`・`layout_scan.rs:47`・`layout_styled.rs:29`・`layout_styled_tests.rs:174` を直す）。
  - `RegionTransform` の `identity`・`apply`・`then`・`to_matrix`・`is_translation_only`・`debug_assert_m1_translation_only`（`canvas.rs:80-145`）: 本番は `translation` と `offset` だけ（grep の当たりは `canvas.rs` だけ）。消すと約 70 行。
  - `ScaleContract::author_dpi` と `new` の第 2 引数（`region.rs:83-84,92,104`）: 本番の 5 か所（`actor.rs:91`・`actor_present.rs:236,506`・`actor_box.rs:228,377`）は全部 `None`。`Some` は `region_tests.rs:407` だけ。`to_image` も本番の呼び手 0。
  - `viewbox_draw.rs:77-81` の `#[cfg(test)]` の再公開（子のテストは `super::plan::..` で引ける・説明は設計の File Structure Plan を挙げる）。
  - `ChoiceDraw`（1 欄だけの包み・`viewbox_draw_render.rs:305,415-417,659,676-679`）。`apply_color_ranges` の `bool` の戻り値（唯一の呼び手 `viewbox_draw_render.rs:249-257` が捨てる）。
  - `DrawExecutor::line_layout`（`draw.rs:753-763`・素通し）と重ねた `#[cfg(test)]`（`draw.rs:767`）。
  - `viewbox_draw_png_dump_tests.rs:201-206,237`（使わない `mk`）と `dump_case` の捨てられる戻り値（`:489,491,498`）。
  - `#[allow(dead_code)]` の `rig` の欄（`viewbox_draw_live_diff_tests.rs:52`・`viewbox_draw_scroll_retain_tests.rs:138`）: 読む所は 0 だが、GPU の一式を面より後に捨てるために要る＝欄は残し、名前を `_rig` にして許しを外す（`Rig` の `_dq` と同じ作法）。
  - 走らない記録だけのテスト（`#[ignore]`）: `viewbox_draw_choice_hover_tests.rs:484,606`（`M_FONTS` `:594`）・`viewbox_draw_line_gap_band_tests.rs:288` とその部品（約 `:40-299`）。どれも限界を名指しし、調べは完了 spec に入った。消すと `choice_hover` で約 230 行・`line_gap` で約 200 行。
- **困る場面**: 読む・直す行が増えるだけ。
- **いちばん小さい直し**: 消す・畳む。
- **範囲の外への波及**: `author_dpi` は **ア・イ とも あり**（`actor*.rs` 3 ファイルの呼び手と、範囲の外の結合のテスト 4 本＝`audit-emo-text-state-actor` と申し合わせ・要件 7.7）。`layout_with_cursor_warn` は イ あり（`actor_decoration_frame_tests.rs:115` の説明 1 行）。ほかは なし。`crates/areka` はどれも使わない。
- **残す 3 種類**: `RegionTransform` は説明が「R8.1/R8.6 の行列の口（emo-compose との合流）」と書く＝**⑵ の議題**。`author_dpi` は形が変わる＝残すのも妥当（要件 7.7）。`#[ignore]` のテストを消すかは開発者の判断。

#### 31. 毎フレームの小さな余分

- **場所と何か**:
  - ブラシと描き先の bitmap を毎フレーム作る（`viewbox_draw_render.rs:144,264-271,362-366,576-579`）。
  - 住人の指紋を 1 フレームに 3 回求め、毎回 `String` と `Vec<u32>` を作る（`viewbox.rs:241,313`・`viewbox_diff.rs:175`）。
  - 食い違いの確かめが `draw_lines.contains`＝行×範囲の数（`viewbox_draw_plan.rs:125-127`・並んでいるので `binary_search` でよい）。
  - 飾りのある字ごとに `look.font_key()` で `Vec<String>` を複製して既定と比べ、既定の鍵（候補 6 つ）を字ごとにハッシュする（`draw_metrics.rs:338,350`・呼び手 `layout_styled.rs:89`）。
  - `CursorWarnGuard::should_warn` が呼ばれるたびに `ActorKey(String)` を複製する（`cursor_tag.rs:231`）。
  - テストの速さ: `viewbox_draw_scroll_retain_tests.rs:345` は D3D の器を 9 回作る・`vertical_band_case` は 1 件ごとに `FontCatalog` を 9 回作る×16 件（測っていない）。
- **困る場面**: 文字送りの 1 字ごとのフレームで確保が走る。大きな話で比べが増える。
- **いちばん小さい直し**: 既定の色のブラシは別の欄で覚える（`BrushCache` を使うと要件 14.2 の「区間の呼び出し 0 件」の檻が鳴る）。指紋は `plan` で 1 度求めて渡す。`binary_search`。複製せず欄で比べ、既定の鍵の表を分ける。`BTreeMap<ActorKey, [bool;2]>` か、複製の前に確かめる。テストは器と `Rc<FontCatalog>` を使い回す。
- **範囲の外への波及**: ア なし。イ なし（`viewbox_draw.rs` に欄を足すのは範囲の中）。
- **残す 3 種類**: 当たらない。`\_l` の範囲外の `debug!`（`cursor_tag.rs:201`・`layout_scan.rs:292`）が毎フレーム出るのは design.md の縮退表の決め（「一回化しない」）＝⑴ に触れるので、ここでは直さない。

#### 32. 記録の失敗の理由の文が実際と違う（`color.rs:80,148,170,177`）

- **何か**: `\f[color,#gggggg]` は 6 字なのに「3 桁でも 6 桁でもない」と記録する。`\f[color,4294967296,0,0]` は u32 の読みのあふれで「数でない」になる（`256` は「範囲外」）。
- **困る場面**: 記録を読んだ作者が誤った直し方をする。
- **いちばん小さい直し**: 文を「3／6 桁の 16 進でない」にし、u64 で読む（か全部が数字かを先に見る）。檻は `color_tests.rs` §1／§3（余白 587）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない（記録の文だけ）。

#### 33. 区切りの前提が崩れても黙って通る（`segment.rs:163-165,182`）

- **何か**: 塊の連結と run の文字列の長さが違うと、後ろのアイテムを全部最後の塊へまとめる。説明に「記録は出さない」と明記。
- **困る場面**: 将来 BudouX が正規化などで長さを変えると、巨大な塊になって 1 字ずつの規則へ落ちるが、記録が残らない。本番では届いた字の腕でフレームごとに走るので、`warn!` を置くと洪水になる。
- **いちばん小さい直し**: `debug_assert_eq!(塊の合計, run_text.len())`。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 明記された判断として残すのも可（ただし `shortcut:` の印ではない＝⑵ には当たらない）。

#### 34. 見本（オラクル）は書体を台帳で解かずに書式を作る（`draw.rs:744`）

- **何か**: 本番の `ViewboxExecutor::ensure_format` は `self.fonts.pick(font)` を通す（`viewbox_draw_render.rs:640-650`）。見本の `DrawExecutor::ensure_format` は通さない。
- **困る場面**: 見本との比べに「Yu Gothic Medium」やぱすたの候補の列を使うと、viewbox は解いた太さ 500 の字面を、見本は解かない最初の名で別の書体へ落ちて描き、差を viewbox のせいにする。今のテストは入れてある書体名・`None`・「ＭＳ Ｐゴシック」だけなので潜在。
- **いちばん小さい直し**: 見本も自分の `FontCatalog` を持って `pick` してから作る（テストの中だけ）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: 当たらない。

#### 35. 既定の書体への作り直しがほとんど効かない（`draw.rs:411-433`）

- **何か**: `CreateTextFormat` は書体の在る無しを確かめず、`pick` は既に解いている。失敗するのは大きさが不正なときだけで、同じ `font.height` で作り直しても失敗する。唯一のテスト（`draw_format_metrics_tests.rs:296-314`）は高さ 0 で、まさにそれを示している。
- **困る場面**: 今は害が無い。読む・試す行が増え、同じ失敗の記録が 2 回出る。
- **いちばん小さい直し**: 作り直しを消す（テストの `warn!` の数の期待が変わる）。
- **範囲の外への波及**: ア なし。イ なし。
- **残す 3 種類**: **⑴ の議題**（完了 spec の R4.2 の作り直し＝内部の決め・ukadoc ではない）。確信は低い（10 節）。

### 決めごととして指摘に数えなかったもの

- 範囲の 81 ファイルに `shortcut:`・`ponytail:` の注釈は **0 件**（grep）。`#[allow(dead_code)]` は 2 件（`viewbox_draw_live_diff_tests.rs:52`・`viewbox_draw_scroll_retain_tests.rs:138`・指摘 30 で扱った）。
- わざと置いた語彙の口・先送り（⑵ に当たるもの・直さない見込み）:
  - `writing.rs:52-54` の `RESERVED_KEY_TEXT_ORIENTATION`／`RESERVED_KEY_TEXT_COMBINE_UPRIGHT`（「記録のみ・実装しない・R5.7」）。`writing.rs:174-175,241-279` の `WritingDirectionDecision` の欄（本番の読み手 0・追跡 spec `currentghost-property-tree` は `.kiro/specs/` に実在）。
  - `canvas.rs:56-63` の `RESERVED_EFFECT_*`・`TextEffects {}`・`ImageSeam`・`SurfaceSeam`（描く側で `warn!` して飛ばす口）・`look.rs:685` の `Script::None`。
  - 行末の禁則のぶら下げは未実装（`layout.rs:35`・`layout_scan.rs:79`）・後の spec を名指す見出し（`layout_scan.rs:2`＝`text-ruby`・`text-align-shadow-canon`、`layout_scan_glyph.rs:2`＝`text-typesetting`・`text-ruby`）。
  - 後ろ向きの `\_l` の後、前の行が validrect の外に残る件（`layout_cursor_overflow_tests.rs:108-110`・要件 2.8 で式を変えない決め・持ち主 `.kiro/specs/text-align-shadow-canon/brief.md:13` の ⑷）。
  - `draw.rs:549-553` の `DrawExecutor` を見本として残す決め・`DWriteMetrics::new` をテストだけに残す決め（`draw_metrics.rs:109-123`）・`vertical_em_box_left` の届かない `None` の枝（`draw_metrics.rs:386-389`）・`region.rs:120-176` の `physical_extent` の +1（2026-08-14 の裁定 `areka-P0-scale-exact-rational`）・`draw_catalog.rs:126-133` の「器の一生に 1 回」の記録・`ScrollState.pos` の補間の口・`viewbox.rs:106-111` のテストだけの比べ。
- 範囲の外へ渡すもの（指摘に数えない）: `actor_present.rs:616` の BudouX の区切りの毎フレームのかけ直し（指摘 9）、`state_decoration.rs:296-297,334-336` の古い説明（指摘 23）、`surface.rs:568,571` の負の数の桁あふれ（指摘 3 の直しで届かなくなる）。どれも `audit-emo-text-state-actor` の持ち物。

## 4. 要件と手持ちの対応（Requirement-to-Asset Map）

| 要件 | 手持ち | ギャップ |
|---|---|---|
| 1 範囲の確定 | 81 ファイル・コード 10,820・テスト 27,643（2 節） | なし（その後の増減は着手の各段で測り直す） |
| 2 洗い出し | 本文書（35 件・81 ファイル読了・組ごとの数は 1 節） | なし（要件 2.6＝プラグインは写していない） |
| 3 直す対象の確定 | 議題の候補は 6・7・17・19・20・21・28・29（`cursor_tag.rs:90`）・30・35（3 節の各「残す 3 種類」） | **Unknown**: 7・20 は SSP の測りが要る（ukadoc に書いていない）。6 は完了 spec の原文の確かめ |
| 4 全部直す | 各指摘の直しは今のファイルの中で閉じる（30 の `author_dpi` だけ actor に及ぶ） | **Constraint**: 消す前の grep はこの段で済ませた（30）。設計の段で直す前にもう一度 |
| 5 申し送り R17 | `apply_pending_newline` と下見の 2 か所・`CursorWarnGuard`（字 1 つ 1 回の記録の型） | **Missing**: 非有限の守りも、それを確かめるテストも 0 本（指摘 1） |
| 6 守る決まり | 依存の追加は不要（全件 std と既存の依存で直る）・`Cargo.lock` は変わらない | 27 は要件 6.4 の違反を直す。GPU のテストは実 GPU のまま（WARP は使わない） |
| 7 範囲の外と約束 | 新しいファイルは要らない（テストは兄弟のファイル・部品は今ある支えのファイル） | **Constraint**: `draw_format_metrics_tests.rs`（余白 78）・`lookahead_tests.rs`（122）・`cursor_tag_resolve_tests.rs`（129）・`viewbox_draw_decoration_tests.rs`（141）は狭い（9 節）。30 の `author_dpi`・7 の 2 つ目の案は `audit-emo-text-state-actor` と申し合わせ。`wintf-visual-tree` の約束に頼るファイル＝`draw_oracle_tests.rs:30,79`・`viewbox_draw_test_support.rs:22,75`（8 組目の全テストが `Rig::attach` 経由で頼る） |
| 8 壊れたら落ちるテスト | `log-capture-kit` の記録の檻・決定論の土台・見本（オラクル）との画素の比べは揃っている | **Missing**: 1〜8・13〜18 の「壊れたら落ちるテスト」は今 0 本 |
| 9 かけ直し | — | 直した後に同じ作法でかけ直す（検証の段のタスク） |

## 5. 直し方の選択肢

- **案 A（今のファイルの中で直す）**: 全件を今のファイルの中で直す。非有限の守り（1・2）は配置の側の 1 か所ずつ、あふれ（3・4）は viewbox の 2 関数、描き直し（16〜18）は `viewbox.rs`・`viewbox_diff.rs`・`viewbox_draw_plan.rs` の中。テストの部品は今ある支えのファイルへ寄せる。
  - ✅ 構造を変えない・差分が小さい・新しいファイルが要らない（要件 7.3 に合う）。❌ 狭いテストのファイル（9 節）では、先に重複を消してから足す順番が要る。
- **案 B（位置の守りを 1 つの型にまとめる）**: 「行送りの軸と行内の軸の位置」を有限に保つ小さな型（足すと有限でなければ記録して据え置く）を `layout_scan.rs` に置き、改行の送り・`\_l` の解決・自動の折り返しの送り・下見の 4 か所をそれで通す。
  - ✅ 指摘 1・2・26（下見の写し）がまとめて閉じ、同じ型の穴が後で開きにくい。❌ 差分が広がり、`layout_scan*.rs` の行の数えの檻（`finish_line(` の 4 など）を触る。
- **案 C（混ぜる）**: 必ず直すの 1〜8 は案 A で先に入れ（テストつき）、下見の写し（26）を片づけるときに案 B の型を小さく入れるかを設計の段で決める。

## 6. 手間と危うさ

- **手間**: L（1〜2 週）。直しの 1 件ずつは小さいが、件数が多く、テストの整理（23〜25・28）と説明の直し（29）の行数が多い。
- **危うさ**: Medium。7・20 は SSP の測りと開発者の裁定が要る。6・14・15・17 は画面に見える振る舞いが変わり、GPU の読み戻しのテストが要る。19・21・35 は裁定しだい。ほかは既存の型の中の直しで Low。

## 7. 設計の段への申し送り

- 推す並び: 必ず直す（1〜8）→ 直したほうがよい（9〜27）→ できれば（28〜35）。ただしテストの部品の寄せ（25）と重複の削り（28 の一部）は、狭いファイルへ檻を足す前に置くと手間が 1 度で済む（`layout_cursor_tests.rs` は 28 を先に・`draw_format_metrics_tests.rs` は 22⑴ の 5 行だけ）。
- 決めること: 7 の validrect の切り方（配置で切るか面の寸だけか）と SSP の測り・17 の「消去の 1 フレーム」を 0 フレームにするか・19 の `\_l` と validrect のどちらが勝つか・20 の高さの上限の値・21 の要件の言い回し・28 の calibration の組・30 の `RegionTransform`／`author_dpi`／`#[ignore]` のテストを残すか・35 の作り直し。
- 1 の記録の一回化に `CursorWarnGuard` を使うなら、縮退の種類を 1 つ足す（`cursor_tag.rs`・範囲の中）。

## 8. タスクの数の見込みと切り方（要件 3.7）

| 組 | 指摘 | タスクの見込み |
|---|---|---|
| 必ず直す | 1〜8 | 6〜8（1＋2・3＋4 はまとめられる・各件に直し＋壊れたら落ちるテスト） |
| 直したほうがよい（コード） | 9〜21・26・27 | 11〜14（11＋12・14＋15・16〜18 は 2 つ・19〜21 は裁定しだいで 0〜3） |
| 直したほうがよい（テスト） | 22〜25 | 5〜6（25 は 2〜3） |
| できれば | 28〜35 | 4〜5 |
| 確かめ | — | 2（audit のかけ直し・全体テスト） |
| 計 | 35 件 | **28〜35** |

20 を越えるので、要件 3.7 のとおり組で切る案を出す。

- **切り方 甲（推す）**: 本 spec＝必ず直す＋直したほうがよいのコード（1〜21・26・27・約 19〜24）。2 本目（同じ範囲・例 `audit-emo-text-layout-draw-2`）＝直したほうがよいのテスト（22〜25）＋できれば（28〜35）（約 10〜12）。19〜21 を議題で「直さない／説明だけ」と裁定すれば、本 spec は 20 の内に収まる見込み。2 本目はテストと説明だけで、コードの振る舞いは変えない。
- **切り方 乙**: 本 spec＝必ず直す＋直したほうがよい（1〜27・約 24〜28）。2 本目＝できれば（28〜35・約 5〜6）。組の切れ目どおりだが、本 spec が 20 を越えやすい。
- **切らない**: 19〜21・35 を残す裁定、23〜25 をまとめて 2 タスクにする、で約 22〜24。20 を少しまたぐ（決まり「一度切り出した spec を少しまたぐ理由でさらに削らない」は、切った後の話＝ここでは切るかどうかの判断）。

## 9. 1,000 行の番人の余白（テストを足すファイル）

| ファイル | 行 | 余白 | 足す見込みの檻 |
|---|---|---|---|
| `draw_format_metrics_tests.rs` | 922 | **78** | 22⑴（約 5 行）・29 の名前の直し・26 の字面の檻の直し |
| `lookahead_tests.rs` | 878 | **122** | 13（約 25 行） |
| `cursor_tag_resolve_tests.rs` | 871 | **129** | 足さない（28 で減る） |
| `viewbox_draw_decoration_tests.rs` | 859 | **141** | 6 の打ち消し線（`:641-669` の書き換えが主） |
| `layout_cursor_tests.rs` | 842 | **158** | 23 の % と `@`（28 で約 60 行空けてから） |
| `region_vertical_canon_tests.rs` | 863 | 137 | 足さない（25 で約 15 行減る） |
| `draw_line_store_tests.rs` | 852 | 148 | なし |
| `draw_oracle_tests.rs` | 837 | 163 | 34（25 で減る） |
| `layout_styled_tests.rs` | 835 | 165 | 説明の直しだけ |
| `layout_segmented_tests.rs` | 783 | 217 | 5・9 |
| `viewbox_dirty_tests.rs` | 770 | 230 | 4・18・23（後ろ向きの縮み 2 本）（28 で約 60 行減る） |
| `viewbox_plan_commit_tests.rs` | 748 | 252 | 3・16・17（28 で約 140 行減る） |
| `layout_wrap_tests.rs` | 736 | 264 | 1 |
| `viewbox_draw_choice_hover_tests.rs` | 714 | 286 | 8（読み戻しにするなら） |
| `viewbox_draw_frame_render_tests.rs` | 648 | 352 | 8（純粋）・23（3 本）・17（GPU の檻を置くなら） |
| `look_font_tag_tests.rs` | 650 | 350 | 21・23 の 2 行 |
| `draw_metrics_styled_tests.rs` | 617 | 383 | 11・12・22⑵ |
| `draw_catalog_tests.rs` | 615 | 385 | 11 |
| `look_font_tag_value_tests.rs` | 431 | 569 | 20 |
| `color_tests.rs` | 413 | 587 | 32 |
| `cursor_tag_tests.rs` | 574 | 426 | 2 |
| `layout_cursor_wiring_tests.rs` | 549 | 451 | 2 |
| `look_tests.rs` | 539 | 461 | 10 |
| `region_tests.rs` | 567 | 433 | 7 |
| `viewbox_draw_underline_tests.rs` | 497 | 503 | 6・15・23（下線と送り） |
| `layout_visible_window_tests.rs` | 366 | 634 | 1（見える窓までの通し） |
| `underline_tests.rs` | 209 | 791 | 14 |
| `viewbox_axis_tests.rs` | 147 | 853 | 3 |
| `segment.rs`（中のテスト） | 534 | 466 | 23（CursorMove の区切り） |

狭い 5 本（余白 160 未満）に足すのは、22⑴・13・6・23 の % と `@` だけ。どれも見込みの行数が余白に収まる。収まらなくなったら、新しいファイルの席の持ち主（`audit-emo-text-state-actor`）と置き場を申し合わせる（要件 7.5）。

## 10. 調べが要る所（Research Needed）

1. SSP で、validrect が画像の外を指すバルーンの文字がどこまで描かれるか（指摘 7・SSP の MCP で測る）。
2. SSP で、`\f[height,N]` の N の上限と、とても大きな値の扱い（指摘 20）。あわせて DirectWrite がとても大きな高さを断るか（指摘 12 が起きうるか・指摘 35）。
3. 完了 spec `areka-P0-text-decoration-canon` の要件と設計が、バルーンの既定の下線・打ち消し線（`font.underline`・`font.strike`）の描き方を別に決めていないか（指摘 6・原文を引く）。
4. 指摘 17 の「消去の 1 フレーム」が、どの完了 spec の決めか（`viewbox_draw.rs:256-257` の出どころ）と、裁定「1 フレーム遅らせる解は取らない」との関係。
5. 指摘 9・10 の効き目（`tools/perf/` で、長い話の BudouX の折り返しと、字ごとに色を変える台本の 1 フレームの時間を前後で測る）。
6. `\f[color, 255, 0, 0]` のように引数に空白を入れたとき SSP が受けるか（今の areka は断る・ukadoc は書かない・具体の誤りが確かめられていないので指摘に数えていない）。

## 11. 要件の討議の前に確かめたこと（2026-11-06・進行役）

- **10 節の 3（指摘 6）**: 完了 spec `areka-P0-text-decoration-canon` は `font.underline`・`font.strike` の読み取りと配線を `areka-P0-balloon-font-descript-keys` の持ち物として外している（同 spec の requirements.md:41・:111「後に着地した方が配線する」）。既定の下線・打ち消し線の描き方を別に決めた箇所は無い。よって指摘 6 は `balloon-font-descript-keys` の要件 5.5 どおりに直すだけで、⑶ には当たらない。
- **10 節の 4（指摘 17）**: 「透明フラッシュは FullClear の 1 フレームのみ」は `viewbox_draw.rs:256-257` のコードの説明だけにある。`.kiro/specs/completed/` の文書を grep して 0 件＝完了 spec の要件にも開発者の裁定にも無い。裁定「1 フレーム遅らせる解は取らない・状態の持ち方を変えて 0 フレームで解け」（2026-09-13）とぶつかるのはコードの説明だけ。
- **指摘 19**: 完了 spec `areka-P0-cursor-tag-canon` の要件 2.6（requirements.md:79）は「解決後の位置が文字描画範囲の外側になるとき…指定どおりの位置を字義どおり用い（内側への自動的な寄せを行わず）、DEBUG レベルのログを記録する」と、`\_l` の行き先が validrect の外になる場合をはっきり決めている。validrect の絶対上限の裁定（2026-09-05）は折り返しの規則で、`\_l` の行き先そのものは同 spec が個別に決めた。
- **10 節の 1・2（指摘 7・20）**: 討議の推奨案は、どちらも SSP の測りに頼らない形にした（7＝配置は変えず面の寸だけを頭打ちにする＝見える振る舞いは変わらない・20＝上限を置かず、害は 1・11・12 の直しで消す）。推奨案で決まれば測りは要らない。

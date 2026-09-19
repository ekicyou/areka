# ギャップ分析: areka-P0-balloon-origin-outside-validrect

> 2026-09-18・`/kiro-validate-gap`（要件確定後・設計前）。本文の file:line は**本日の実測値**（ブランチ `claude/areka-sender-header-4b5900`・HEAD `b3fa7ec6`）。brief の起票時実測値とずれている箇所は本文で明示する。
>
> 本書は分析と選択肢を提示するもので、最終決定は要件ディスカッションと設計に委ねる。

## 1. 要約（3〜5 行）

- **編集点は実測どおり 1 関数**——`crates/areka-emo-text/src/region.rs` の `resolve_origin_component`（定義 `:432-458`・呼出 `:290`／`:297`・書字開始角の選択 `:286-289`）。撤去前の形 `clamp_origin_component`（`git show b9ede5ad^:crates/areka-emo-text/src/region.rs` の `:302-330`）は「両端を含む範囲判定 → 範囲内なら値・範囲外なら書字開始角＋`debug!`」で、現行関数と引数・分岐構造が同一。返す腕を 1 つ差し替えれば要件 1 は満たせる。
- **最大のギャップは要件 3（WARN を装着 1 回につき 1 件）**——`TextRegion::resolve` は再追従の判定キーを得るために**毎フレーム**呼ばれる（`actor.rs:418` `refresh_actor_binding`・`areka/src/emo2_boot/frame/scale_text.rs:118` が毎フレーム委譲）。同種の先例（折返し基準の警告）は解決側から外し、actor の登録口 `register_actor`（`actor.rs:315`）で `previous == Some(region)` の比較により 1 件に絞っている（`warn_coarse_wrap_threshold` `:188-200`）。ただし登録口は `ResolvedBalloonText` しか受け取らず `BalloonModel` を持たないため、「宣言があって範囲外ゆえ無視した」という事実と宣言値を**`TextRegion` 側に運ぶ器**が要る（§4 の候補）。
- **要件 1.3「両端を範囲内とみなす」は既存テスト約 20 本の生存条件**——`origin (0,0)` を辺 `left 0／top 0` の `validrect` と組で使うテスト補助が多数あり（viewbox 系 11 か所・`layout_hard_limit_tests.rs` 2 か所ほか）、境界を排他にすると一斉に赤くなる。要件 4.2／4.3 の全数確認は本書 §3.4 で済ませた——**範囲外の値を使う既存テストは撤去を固定している 4 本だけで、`\_l` 系は 0 本**。
- **規模 S・リスク低**——新規の意味論は無く、撤去前の形の復元＋先例どおりの警告配置＋テストと文書の追随。唯一の設計上の分かれ目は「範囲外の事実をどの型に載せて登録口へ届けるか」（§5 の裁定候補 1）。
- **1,000 行の見張りに対する余白は 49 行**（`region.rs` 951 行）。フィールド追加＋兄弟ファイルの `#[path]` 宣言で 15 行前後増えるため、書き直す既存テスト 1 本を兄弟ファイルへ移して相殺する案を §5 に載せた。

## 2. 現状調査

### 2.1 解決関数と呼び出し（`crates/areka-emo-text/src/region.rs`）

| 何の定義か | 位置（本日実測） | 内容 |
|---|---|---|
| モジュール冒頭 doc「描画開始点は宣言どおり」「撤去された規約」 | `:24-37` | 要件 6.4 の書き直し対象。撤去（2026-08-27）を正典として説明している |
| `TextRegion::resolve` の doc「描画開始点」行 | `:242-243` | 「宣言された origin 成分は字義どおり（validrect 外なら `debug!` 記録・位置は動かさない）」——追随が要る |
| 書字開始角の選択 `start_corner` | `:286-289` | `HorizontalTb`／`VerticalLr`＝`(left, top)`・`VerticalRl`＝`(right, top)`。**無改変で要件 1.1／1.5／1.7 の「書字開始角」を供給できる** |
| `resolve_origin_component` の呼出（x／y） | `:290-303` | 引数 `(v, extent, range, corner, key)`——`range` は `(left, right)`／`(top, bottom)`（解決後の validrect）、`corner` は書字開始角の当該成分。**成分ごとに独立した呼出**なので要件 1.2 は構造上すでに満たされている |
| `resolve_origin_component` 本体 | `:432-458` | `Some` → `resolve_coord` で負値を反対端基準に絶対値化（要件 1.4 の順序はすでにこの形）→ `resolved < range.0 || range.1 < resolved` なら `debug!` 1 件 → **範囲内外を問わず `resolved` を返す**。`None` → `debug!`＋`corner` |
| 範囲判定の式 | `:442` | 両端を含む判定（`<`／`<` の否定）。要件 1.3 が「現行の記録の判定と同じ」と指すのはこの式で、そのまま返値の判定に流用できる |
| `TextRegion` 構造体 | `:186-204` | `#[derive(Clone, Copy, Debug, PartialEq)]`・全フィールド private・構築点は `resolve` の `:322-331` の 1 か所（ワークスペース全域で構造体リテラル構築は他に無い＝grep で確認） |
| 兄弟テストの接続宣言 | `:946-951` | `#[cfg(test)] #[path = "region_inline_limit_tests.rs"] mod inline_limit_tests;` の形。新設ファイルも同じ形で 3 行足す |

撤去前の形（PR #124 の親コミット `b9ede5ad^`）:

```rust
// crates/areka-emo-text/src/region.rs（b9ede5ad^）の clamp_origin_component
Some(v) => {
    let resolved = resolve_coord(v, extent);
    if range.0 <= resolved && resolved <= range.1 { resolved } else {
        tracing::debug!(key, resolved, range_min = range.0, range_max = range.1, corner,
            "validrect 外の origin 成分を書字開始角へ寄せる（クランプ正準）");
        corner
    }
}
```

現行関数との差は「範囲外の腕が `resolved` を返すか `corner` を返すか」と `debug!` の文言・`corner` 欄の有無だけである。

### 2.2 警告の先例（要件 3 の型）

| 何の定義か | 位置 | 要点 |
|---|---|---|
| `warn_coarse_wrap_threshold` | `crates/areka-emo-text/src/actor.rs:188-200` | `region.wrap_threshold() <= region.inline_limit() \|\| previous == Some(region)` なら何もしない。それ以外は `warn!(balloon = BALLOON_NAME_PLACEHOLDER, axis, wrap_threshold, inline_limit, "折返し基準が描画範囲の外に解決された……")` |
| 呼出点 `register_actor` | `actor.rs:315-333` | `warn_coarse_wrap_threshold(&resolved, self.layout_input.get(&actor).map(\|it\| it.region))`——**前回の解決済み領域**と比較。装着（初回）は `None` なので必ず 1 件、同値の再追従は 0 件、値の変わる再追従は 1 件 |
| 登録口の入力 | `actor.rs:315-320` | `(actor, binding, resolved: ResolvedBalloonText)`——**`BalloonModel` は渡ってこない**。`ResolvedBalloonText`（`:137-150`）は `mode`／`region`／`font`／`wrap`／`choice_style` |
| model を持つ経路 | `actor.rs:364-375` `register_actor_binding`・`:418-460` `refresh_actor_binding` | どちらも `ResolvedBalloonText::resolve_with_background(model, image_size, background)`（`actor_decoration.rs:95`・内部で `TextRegion::resolve` `:107`）を呼んでから `register_actor` へ合流 |
| 毎フレーム解決の根拠 | `actor.rs:250-265`（`resolve` の doc）・`region_inline_limit_tests.rs` 冒頭 | 実機一周で生ログ 30,837 行のうち 27,908 行が同じ警告になった実測。「解決する側に粗さの記録を持たせない」が確立した規律 |
| 記録水準の指針 | `.kiro/steering/logging.md` | `warn!`＝「回復可能なエラー、警告、非推奨の使用／フォールバック発生」。範囲外宣言の無視はこの定義に合致 |
| 件数を数えるテストの先例 | `actor_region_warn_tests.rs`（380 行・6 本） | 文字列 2 層（`DESCRIPT` 定数＋上書き層）を `parse_str` で写像 → `runtime_with_slot` → `register_actor_view`／`refresh_actor_binding` を呼び、`capture` で件数と 4 欄を数える。0 件主張には対照の `error!` を同じ捕捉窓で発行 |

**ギャップ**: 登録口で「範囲外ゆえ無視した」ことと、要件 3.3 が求める欄（宣言どおりに解決した値）を知るには、解決結果（`TextRegion`）がその事実を運んでいなければならない。現行 `TextRegion` は開始点しか持たず、開始点が書字開始角と一致するのは「未宣言」と「範囲外」の両方なので区別できない。

### 2.3 テスト資産（既存・要件 5 の前提）

| ファイル | 現状 | 本仕様との関係 |
|---|---|---|
| `region.rs:677` `origin_components_resolve_literally_and_independently` | `(Some(100), Some(0))`・横書き → 期待 `(100.0, 0.0)`。「y は 46 へ寄らない」と明記 | 要件 5.4 の書き直し対象①（期待 `(100.0, 46.0)` へ） |
| `region_vertical_canon_tests.rs:446` `declared_origin_outside_validrect_is_literal_with_one_debug_per_component` | 5 ケース表（y のみ外／x のみ外／両方外×3 方向）。期待 start は字義値・`counts.debug` は範囲外成分数・**`counts.warn == 0` も主張**（`:505` 付近） | 書き直し対象②。start 期待値を書字開始角へ。`warn == 0` の主張は **WARN を解決側に置くと赤くなる**（登録口に置けば緑のまま） |
| 同 `:576` `negative_origin_resolves_from_opposite_edge_then_is_used_literally` | 3 ケースのうち「範囲外へ解決」`(-380,-200)→(20,24)` と「反対端ちょうど」`(-400,-224)→(0,0)` の 2 つが範囲外 | 書き直し対象③（要件 1.4 の表そのもの。期待を書字開始角へ） |
| 同 `:617` `declared_origin_resolution_is_independent_of_validrect` | 「validrect を変えても宣言 origin の解決結果は動かない」不変条件を 4 通りの validrect で固定。**本仕様の規則と正面から矛盾** | 書き直し対象④（「範囲内では動かない・範囲外になる差し替えでは書字開始角へ動く」へ反転） |
| 同 `:660` `undeclared_origin_does_move_when_validrect_changes` | 未宣言の対照 | 無改変 |
| 同 冒頭 doc `:42-54` の分岐 6・9 の説明 | 「validrect 外の宣言＝字義位置」「クランプを戻すと 6. と 9. が赤になる」 | 文言の追随が要る（doc だけ） |
| `region.rs:646` `in_range_origin_is_kept_as_start_point`・`:690` `negative_origin_resolves_from_opposite_edge`（(300,124) は範囲内） | 範囲内 | 無改変で緑（要件 2.1 の証跡に流用可） |
| `tests/shipped_fixture_region_test.rs` | 原本 `emo2/emo2-kakukaku` と複製 `emo2-kakukaku-wplimit` を 2 層マージで解決し sakura `(36,46)`／kero `(24,40)` を固定。**origin 未宣言**ゆえ本修正で不変 | 要件 2.3 の無改変証跡。**新設する検体テストの雛形**（`CARGO_MANIFEST_DIR` 起点の相対パス・`decode(&bytes, DefaultEncoding::Ansi)`・`parse_str(descript, Some(overlay))`・`WritingMode::resolve`・PNG IHDR 突合） |
| `actor_region_warn_tests.rs` | 上記 | 要件 5.6 の雛形（同じ 2 層文字列＋登録口＋捕捉窓） |
| `lib.rs:182` `PURE_SOURCES`・`:356` `SOURCES_OUTSIDE_THE_PURE_SCAN`・`:397` `pure_layer_modules_have_no_windows_imports`・`:424` `every_source_file_is_either_scanned_or_explicitly_excluded` | `src/` 直下の全ファイルはどちらかの表に載っていなければ赤 | 要件 5.7 の「新設ファイルの走査対象登録」＝新設の兄弟ファイルを `PURE_SOURCES` に足す（`steering/structure.md:181` の規則） |
| `crates/log-capture-kit/tests/file_length_guard_test.rs` | `OVER_LIMIT_ALLOWED` 10 件の例外表＋1,000 行超過の走査 | `region.rs` 951 行＝余白 49 行 |

### 2.4 検体（要件 1.6／5.5）

- `crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-offsetdpi/descript.txt`: `origin.x,0`／`origin.y,0`・`wordwrappoint.x,-34`・`validrect.*` 全 0・`charset,UTF-8`。
- 同 `balloons0s.txt`: `validrect.top,46／bottom,-56／left,36／right,-44`・`wordwrappoint.x,-49`。同 `balloonk0s.txt`: `validrect.top,40／bottom,-70／left,24／right,-48`（`wordwrappoint` 行なし＝基層の `-34` を継承）。
- **リポジトリ全域でバルーン定義に `origin.x`／`origin.y` を書いているのはこの 1 ファイルだけ**（`^origin\.[xy],` の行頭アンカー付き全文検索・`target`／`.git`／`.kiro`／`doc` 除外・ファイルシステム側）。brief の記述と一致。
- 参照している既存コード: `crates/areka/src/placement/transition_judge_offset_signoff_tests.rs`（`:33`／`:46`／`:148`——いずれも**手順書のコメント内のパス**で、コードからは読んでいない）と `doc/ukadoc-coverage/briefing-assets.md:383`。`areka-P0-nar-install` の brief `:25` は同ディレクトリを「`type,balloon`（areka 製の検証用派生）」として一覧に載せている。
- 本番経路 `areka-emo-present/src/balloon.rs:499` `load_scope_balloon_model` は `parse_str(&descript, face_override.as_deref())` で 2 層をマージ。`shipped_fixture_region_test.rs` の `merged_model` と同じ呼び方。
- 画像原寸は原本と同じ `balloons0.png` 400×224／`balloonk0.png` 288×203 と見込むが、**本書では IHDR を読んでいない**——新設テストで原本と同様に IHDR 突合を持たせる（期待値 sakura `(36,46)`／kero `(24,40)` は原寸に依存しない——`left`／`top` は非負の素通しなので原寸が違っても開始点は変わらない。ただし `right`／`bottom` の範囲判定には効く）。

### 2.5 文書（要件 6）

| 対象 | 位置（本日実測） | 状態 |
|---|---|---|
| 撤去の行 | `doc/COMPAT_ARCHITECTURE.md:177` | 題「宣言された `origin` の validrect 外クランプ……の撤去」。判断欄は「宣言どおり」「DEBUG 1 件」「validrect を記録の判定にのみ用い返す値には一切用いない」。根拠欄に「validrect 外の宣言は SSP でも壊れた定義」（反証された前提）。要件 6.1 の書き換え対象 |
| `\_l` の縦書き座標系の行 | `:183` | 「クランプ撤去の行は『撤去により areka 内では二択が発生しない』という帰結として SC15 に触れる」・原点＝`TextRegion::start()`〔`region.rs:292-294`〕・書字開始角 `(right, top)`〔同 `:231-234`〕。要件 6.2 の言い直し対象。**引用の行番号 2 か所が既に陳腐化**（現在は `:290-303`／`:286-289`）——「何の定義行か」で指し直す好機 |
| `\_l[x,y]` の上書き行 | `:209` | 「未宣言成分だけが書字開始角へ落ちる（`region.rs:292-294` の `TextRegion::start()`・書字開始角の分岐は同 `:231-234`）」。要件 6.3 の書き換え対象。同じく行番号が陳腐化 |
| `region.rs` 冒頭 doc | `:24-37` | 要件 6.4 |
| roadmap 台帳 | `.kiro/steering/roadmap.md` の spec 台帳の #46 の行と、ウェーブ表の A0 の行（番号は 2026-09-19 の main 取り込みで #38 → #46） | 要件 7.1（完了時） |
| 隣接 spec | `areka-P0-balloon-canon-residue/brief.md:98`（項目 14＝折返し基準の範囲外）・`areka-P0-emo-text-canon-residue/brief.md:11`（項目 14＝`BALLOON_NAME_PLACEHOLDER`）・`areka-P0-currentghost-property-tree/*.md:23`（`balloon.scope(ID).*` 19 項目に `basepos` を含む） | brief の記述どおり実在。要件 7.2 の申し送り先も実在 |

ログ文言の依存: 現行の 2 つの `debug!` 文言（「宣言された origin 成分が validrect の外にある……」「未指定の origin 成分を書字開始角へ寄せる」）を grep している文書・手順書・スクリプトは**無い**（`.md`／`.rs`／`.ps1` 全文検索でヒットは `region.rs` 本体と本仕様の brief のみ）。文言は自由に改められる。

## 3. 要件ごとの実現可能性と資産対応表

凡例: **既存**＝そのまま使える／**改変**＝既存を直す／**欠落**＝無い（作る）／**不明**＝設計で調べる／**制約**＝既存の規律による縛り

### 3.1 要件 1（範囲外の解決）

| AC | 資産 | 区分 | 備考 |
|---|---|---|---|
| 1.1 範囲外→書字開始角 | `resolve_origin_component` `:442-451` | **改変** | 範囲外の腕で `corner` を返す（撤去前の形）。`corner` はすでに引数で渡っている |
| 1.2 成分ごとに独立 | 呼出 `:290-303` | **既存** | x／y を別々に呼ぶ構造のまま |
| 1.3 両端を含む | 判定式 `:442` | **既存** | 同じ式を返値の判定に使う。**排他にすると既存テスト約 20 本が赤**（§3.4） |
| 1.4 負値→絶対値化→判定 | `resolve_coord` `:405`→`:441` | **既存** | 順序はすでにこの形 |
| 1.5 3 書字方向で同じ規則 | `start_corner` `:286-289` | **既存** | 書字方向は `corner` の選択にだけ効く |
| 1.6 検体 sakura (36,46)／kero (24,40) | `load_scope_balloon_model` 相当の `parse_str` 2 層＋`TextRegion::resolve` | **既存**（経路）／**欠落**（テスト） | §2.4 |
| 1.7 右辺より右→左辺（最寄りでない） | `corner` の意味 | **既存** | `corner` は書字開始角であって最寄りの辺ではない。撤去前も同じ |

### 3.2 要件 2（不変）

| AC | 資産 | 区分 | 備考 |
|---|---|---|---|
| 2.1 範囲内は宣言どおり | 範囲内の腕 | **既存** | `region.rs:646`／`:690`・`layout_cursor_wiring_tests.rs:64`・`layout_cursor_center_origin_tests.rs:107` が固定済み |
| 2.2 未宣言→書字開始角 | `None` の腕 `:453-456` | **既存** | 無改変 |
| 2.3 `shipped_fixture_region_test.rs` 無改変で緑 | 同ファイル | **既存** | 原本・複製とも origin 未宣言 |
| 2.4 解析層を変えない | `areka-parsers::balloon` | **既存** | 触らない。`Origin` は `Option<i32>` で宣言の有無を運んでいる |

### 3.3 要件 3（記録）

| AC | 資産 | 区分 | 備考 |
|---|---|---|---|
| 3.1 WARN | `tracing::warn!`・`logging.md` の水準表 | **既存** | 水準は指針に合致 |
| 3.2 装着 1 回につき成分 1 件・再追従で追加 0 | `register_actor` の `previous` 比較・`warn_coarse_wrap_threshold` | **既存**（型）／**欠落**（本件用の関数） | 解決側に置けない（毎フレーム）。**登録口が範囲外の事実を知る手段が無い**＝本仕様最大のギャップ。§4 |
| 3.3 欄（成分名・解決値・範囲両端・用いた角） | `TextRegion` の `left`〜`bottom`・`start` | **既存**（範囲と角）／**欠落**（宣言どおりの解決値） | 範囲と角は `TextRegion` から読める。宣言どおりの解決値だけが失われる→運ぶ器が要る |
| 3.4 範囲内・未宣言で 0 件 | 判定 | **既存** | 範囲判定の否定側 |
| 3.5 無記録の経路を持たない | 解決側の `debug!` | **既存** | 解決側の `debug!` を残せば、登録口の WARN と二重になるが無記録経路は生じない。残す／消すは裁定候補 2 |

### 3.4 要件 4（`\_l` の原点）——全数確認の結果（2026-09-19 引き直し）

`\_l` の数値座標の原点は `cursor_tag.rs` の `resolve_cursor_axis` が `CursorBasis.origin` から読み、その値は `TextRegion::start()` に由来する。開始点の解決規則を変えれば `\_l` は自動で追随するので、要件 4.1 は実装を 1 行も変えずに成立する。

**結論（要件 4.3 の 0 の明示）**: 2026-09-19 に下の 4 つの方法で引き直した結果、**`\_l` の原点に宣言された `origin` を使う既存テストのうち、`validrect` の範囲外の値を使うものは 0 本**である。調べた対象は `crates/areka-emo-text/src`・`crates/areka-emo-text/tests`・`crates/areka/src` の 3 つのディレクトリで、この中で `Origin` を組み立てる文字列を含むファイルは 29、その行は 38 行（うち 5 行は説明文の中での言及で、実際に値を組み立てている行は 33）。`areka-parsers` は定義を読む層のテストで開始点の解決を通らないため対象外とした。

**この 0 は要件 1.3（範囲の両端を範囲内とみなす）の読みに依存する。** 下で数え上げた宣言つきの呼出しのうち **130 か所**が `origin (0, 0)` と「左辺 0・上辺 0」（または `validrect` 全未宣言＝画像全域）の組で、辺にちょうど重なる値を範囲内とする読みでのみ範囲内になる。両端を範囲外とする読みへ変えれば、これらは一斉に範囲外へ回り、`\_l` のものを含む多数の既存テストの期待値を書き直すことになる。`\_l` 側で端ちょうどに当たるのは `layout_cursor_order_tests.rs`・`layout_cursor_tests.rs`・`layout_cursor_overflow_tests.rs`・`layout_cursor_wiring_tests.rs` が渡す `(0, 0)` の 2 成分と、`cursor_tag_test_support.rs` の定数 `ORIGIN`（`(50, 20)`）の y 成分（同ファイルの `VALID_TOP` が 20）である。一方 `layout_cursor_center_origin_tests.rs` の `(50, 20)`（範囲 left 30・top 8・right 350・bottom 210）と `layout_cursor_wiring_tests.rs` の `(100, 60)`（範囲 left 40・top 20・right 360・bottom 200）は辺に触れておらず、1.3 の読みによらず範囲内である。

#### 方法（4 つ・そのまま再実行できる形）

1. `Origin::new(Some` の全文検索。結果 13 行・12 ファイル（うち `actor_scale_refresh_tests.rs` と `tests/draw_readback_test.rs` の 2 行は説明文の中での言及なので、実際の構築は 11 行・10 ファイル）。
2. `model(`／`model_rect(`／`model_of(` の呼出しのうち、第 1 引数に `(Some(` を持つものを、前後 2 行の窓で拾う検索。宣言された `origin` を渡す呼出しは 15 ファイルに散る（内訳は下の 2 つ目の表）。
3. `origin` を `(0, 0)` に固定してしまう補助関数——`layout_hard_limit_tests.rs` の `model_of`・`viewbox_test_support.rs` の `model_rect`・`viewbox_draw_test_support.rs` の `geo_model`・`actor_test_support.rs` の `geo_model`——の呼び手をたどり、それぞれが渡す `validrect` を列挙。
4. （2026-09-19 に追加）`Origin::new(` の構築点を**全数**、ファイルごとの件数つきで列挙し、引数をそのまま渡すだけの補助関数は**その呼び手まで辿る**。`Origin` の組み立て口は `areka-parsers` の `Origin::new` ただ 1 つで、構造体リテラルや `Default` からの経路は無いことも併せて確かめた（コードの外から入る経路は、実物の定義ファイルを読む `tests/shipped_fixture_region_test.rs` だけ）。

#### 方法 4 の内訳——`Origin::new(` の構築点（ファイルごとの件数）

| 区分 | ファイル | 件数 |
|---|---|---|
| 宣言つき（値を直書き） | `src/actor_choice_contract_tests.rs` | 1 |
| | `src/actor_decoration_tests.rs` | 1 |
| | `src/actor_runtime_frame_tests.rs` | 1 |
| | `src/actor_test_support.rs`（`geo_model`） | 1 |
| | `src/draw_oracle_tests.rs` | 2 |
| | `src/layout_hard_limit_tests.rs`（`model_of`） | 1 |
| | `src/viewbox_draw_test_support.rs`（`geo_model`） | 1 |
| | `src/viewbox_test_support.rs`（`model_rect`） | 1 |
| | `tests/attach_wiring_test.rs`（`geo_model`） | 1 |
| | `tests/viewbox_scroll_test.rs`（`model`） | 1 |
| **小計** | 10 ファイル | **11** |
| 素通し（引数をそのまま渡す補助関数） | `src/canvas.rs`・`src/draw_oracle_tests.rs`・`src/layout_test_support.rs`（2）・`src/region.rs`・`src/region_vertical_canon_tests.rs`・`tests/pipeline_test.rs`・`tests/scale_invariance_test.rs` | **8** |
| 未宣言（`None, None`） | `src/actor_scale_refresh_tests.rs`・`src/actor_scroll_retain_tests.rs`・`src/choice_decorate_tests.rs`・`src/choice_tests.rs`・`src/cursor_tag_test_support.rs`・`src/draw_oracle_tests.rs`・`src/draw_test_support.rs`・`src/viewbox_draw_test_support.rs`・`src/wrap.rs`・`src/writing.rs`・`src/writing_decision_tests.rs`・`tests/draw_readback_test.rs`・`tests/viewbox_blit_spike.rs`・`areka/src/input_events/balloon_pure_core_tests.rs` | **14** |
| 説明文の中での言及（構築ではない） | `src/actor_scale_refresh_tests.rs` 1・`tests/draw_readback_test.rs` 1・`tests/shipped_fixture_region_test.rs` 3 | **5** |
| **合計** | 29 ファイル | **38 行**（実構築 33） |

#### 素通し補助関数の呼び手（宣言された `origin` を渡すものだけ）

| 補助関数 | 呼び手（ファイル: 件数） | 宣言値 | 当該 `validrect` | 判定 |
|---|---|---|---|---|
| `canvas.rs` の `model` | `canvas.rs` 7 | `(0,0)` 6／`(100,50)` 1 | 全未宣言＝画像全域 400×224 | 範囲内（`(0,0)` は端ちょうど） |
| `draw_oracle_tests.rs` の `geo_model` | `draw_oracle_tests.rs` 9 | `(0,0)` 8／`(20,20)` 1 | 全未宣言 | 範囲内 |
| `layout_test_support.rs` の `model`（`validrect` は常に未宣言） | `layout_cursor_order_tests.rs` 1・`layout_cursor_tests.rs` 12・`layout_cursor_wiring_tests.rs` 1・`layout_segmented_tests.rs` 14・`layout_styled_tests.rs` 6・`layout_visible_window_tests.rs` 1・`layout_wrap_tests.rs` 13（計 48） | `(0,0)` 47／`(100,50)` 1 | 画像全域 400×224 | 範囲内 |
| `layout_test_support.rs` の `model_rect` | `layout_cursor_center_origin_tests.rs` 1・`layout_cursor_overflow_tests.rs` 1・`layout_cursor_wiring_tests.rs` 1・`layout_visible_window_tests.rs` 8・`layout_wrap_tests.rs` 1（計 12） | `(50,20)`／`(100,60)`／`(0,40)`／残りは `(0,0)` | 呼び手ごと。`(50,20)` は left 30・top 8・right 350・bottom 210、`(100,60)` は left 40・top 20・right 360・bottom 200、他はすべて left 0・top 0 | 範囲内（前 2 者は内側・他は端ちょうど） |
| `region.rs` の `model` | `region.rs` 6 | `(0,0)`／`(100,50)`／`(100,0)`／`(-100,-100)`→`(300,124)`／`(120,70)` 2 | 退化矩形〔0,0〕・top 46 bottom −56 left 36 right −44・top 50 bottom 200 left 30 right 330 | `(100,0)` の y だけが範囲外（規則そのものを固定する検査） |
| `region_vertical_canon_tests.rs` の `model` | `resolve_counting` 経由 9 か所＋`vertical_region_is_invariant_to_wordwrappoint_x` 2 か所 | 範囲内は `(200,60)`（範囲 left 36・top 46・right 356・bottom 168）。範囲外も意図的に渡す | 同上 | 範囲外を渡すのは規則を固定する検査 |
| `tests/pipeline_test.rs` の `model_rect` | `tests/pipeline_test.rs` 1 | `(0,0)` | top 0・bottom 34・left 0・right 400 | 範囲内（端ちょうど） |
| `tests/scale_invariance_test.rs` の `model` | 宣言された `origin` を渡す呼び手は 0 件（5 か所すべて未宣言） | — | — | 対象外 |

`origin` を固定したまま `validrect` だけを呼び手から受け取る補助関数は 2 つある。`layout_hard_limit_tests.rs` の `model_of` は呼び手 2 か所でいずれも top 0・left 0。`viewbox_test_support.rs` の `model_rect` は `canvas_for` 1 か所から呼ばれ、その `canvas_for` の呼び手は `viewbox_dirty_tests.rs` 27・`viewbox_plan_commit_tests.rs` 19 の計 **46**（旧記録の「11 か所」は実数と合っていなかった）。渡される `validrect` は全 46 か所とも top 0・left 0 で、`(0,0)` は端ちょうど＝範囲内である。残る固定 `origin` の補助関数（`actor_test_support.rs`・`viewbox_draw_test_support.rs`・`tests/attach_wiring_test.rs` の各 `geo_model`、`tests/viewbox_scroll_test.rs` の `model`）は `validrect` も関数の中で決めており、呼び手によらず判定は 1 つに定まる（前 3 者は全未宣言＝画像全域、最後は top 0・bottom 120・left 0・right 100）。

#### 元の 3 手法が落としていたもの

宣言された `origin` が開始点の解決へ届くのに、2026-09-18 の記録（元の 3 手法）に名前が出てこないファイルが 6 つある——`layout_cursor_order_tests.rs`・`layout_cursor_overflow_tests.rs`・`layout_segmented_tests.rs`・`layout_styled_tests.rs`・`layout_wrap_tests.rs`・`tests/pipeline_test.rs`。設計検証が挙げた 6 つのうち `draw_oracle_tests.rs` と `canvas.rs` は旧記録に名前だけは出ているが、出ているのは値を直書きした 2 か所だけで、素通し補助関数を経由する呼び手（それぞれ 9 件・7 件）は数えられていなかった。代わりに `layout_segmented_tests.rs`（14 件）と `layout_styled_tests.rs`（6 件）が丸ごと抜けており、viewbox 系の件数も 11 ではなく 46 だった。つまり漏れは設計検証の見立てよりも広い。方法 4 はファイルごとの件数を突き合わせる形なので、同じ漏れを繰り返せない。いずれの漏れも値は範囲内（端ちょうどを含む）で、結論の 0 は動かない。

#### 範囲外の値を使う箇所（全数・いずれも `\_l` を通らない）

1. `region.rs` の `out_of_range_origin_component_falls_back_to_start_corner_independently`——宣言 `(100, 0)` に対し範囲の上辺が 46。
2. `region_vertical_canon_tests.rs` の `declared_origin_outside_validrect_falls_back_to_start_corner_with_one_debug_per_component`・`negative_origin_resolves_from_opposite_edge_then_is_range_checked`・`declared_origin_follows_validrect_only_when_it_leaves_the_range`・`origin_range_table_holds_for_every_mode_and_component`。
3. `tests/shipped_fixture_region_test.rs` の `offsetdpi_fixture_with_out_of_range_origin_starts_at_writing_corner`——実物の検体 `emo2-kakukaku-offsetdpi` の `origin.x,0`／`origin.y,0`。

いずれも本仕様の規則そのものを固定する検査であり、`\_l` の座標解決を 1 本も通らない。よって要件 4.2 の「期待値を見直す」は発動しない。

#### 併走して確かめたこと（2026-09-19 実測）

`\_l` の既存テスト群——`cursor_tag.rs`（本体）・`cursor_tag_tests.rs`・`cursor_tag_resolve_tests.rs`・`cursor_tag_test_support.rs`・`layout_cursor_tests.rs`・`layout_cursor_order_tests.rs`・`layout_cursor_overflow_tests.rs`・`layout_cursor_wiring_tests.rs`・`layout_cursor_center_origin_tests.rs`・`layout_cursor_vertical_tests.rs`・`layout_cursor_vertical_canon_tests.rs`・`state_cursor_coord_parse_tests.rs`——は本作業で 1 行も触っておらず、12 ファイルの差分は空である。検査は 10 モジュール計 114 件がすべて緑（`cursor_tag::tests` 18・`cursor_tag::resolve_tests` 12・`layout::cursor_tests` 12・`layout::cursor_order_tests` 4・`layout::cursor_overflow_tests` 5・`layout::cursor_wiring_tests` 8・`layout::cursor_center_origin_tests` 5・`layout::cursor_vertical_tests` 15・`layout::cursor_vertical_canon_tests` 11・`state::cursor_coord_parse_tests` 24）。本仕様の期間中にこの群へ入った変更は、タスク 6.1（要件 6.4）が `layout_cursor_wiring_tests.rs` の説明文 1 か所を現在の規則へ言い直した分だけで、検査本体も期待値も動いていない。`cargo test -p areka-emo-text --no-fail-fast` は 17 ターゲット 895 passed・0 failed で、引き直しの前後で変わらない。

**副産物**: 「端ちょうど＝範囲内」を前提に書かれた呼出しが 130 か所ある。内訳は上の 2 つの表のとおりで、素通し補助関数の呼び手が `canvas.rs` 6・`draw_oracle_tests.rs` 8・`layout_test_support.rs` の `model` 47・同 `model_rect` 10・`region.rs` 1・`tests/pipeline_test.rs` 1、`origin` を固定する補助関数が `model_of` 2・`canvas_for` 46、値を直書きした構築点が 9（`actor_choice_contract_tests.rs`・`actor_decoration_tests.rs`・`actor_runtime_frame_tests.rs`・`actor_test_support.rs` の `geo_model`・`draw_oracle_tests.rs` の 2 か所・`viewbox_draw_test_support.rs` の `geo_model`・`tests/attach_wiring_test.rs` の `geo_model`・`tests/viewbox_scroll_test.rs` の `model`）で、6 + 8 + 47 + 10 + 1 + 1 + 2 + 46 + 9 = 130 となる。直書きは全 11 か所あるが、`layout_hard_limit_tests.rs` の `model_of` と `viewbox_test_support.rs` の `model_rect` は `validrect` を呼び手から受け取るため呼び手の側（2・46）で数えており、ここでは重ねない。要件 1.3 の両端包含は好みではなく、既存資産の生存条件である。

### 3.5 要件 5（決定論テスト）

| AC | 資産 | 区分 | 備考 |
|---|---|---|---|
| 5.1 3 方向×2 成分×（外／内／端／負値内／負値外）の表 | `region_vertical_canon_tests.rs` の `resolve_counting`・`assert_capture_alive`・`model`・`ALL_MODES` | **既存**（補助）／**欠落**（表） | 補助関数は `region_vertical_canon_tests.rs` の private。新設ファイルへ複製するか、`region.rs` 側の `tests::model` を `pub(super)` にして共有するかは設計で決める |
| 5.2 兄弟ファイル新設 | `region.rs:946-951` の `#[path]` 宣言・`structure.md:146-181` の命名規則（`region_<テーマ>_tests.rs`） | **既存**（規則） | 例: `region_origin_range_tests.rs`。`region.rs` に 3 行増 |
| 5.3 範囲外の腕を潰して赤を確認 | 手順（実装タスク） | **制約** | 範囲外の腕を `resolved` に戻して赤を確認してから採る |
| 5.4 既存 4 本の書き直し | §2.3 | **改変** | 期待値と doc の追随。`warn == 0` の主張は WARN の置き場所に依存（§4） |
| 5.5 検体テスト | `shipped_fixture_region_test.rs` の関数群（private） | **既存**（雛形）／**欠落**（テスト） | 同ファイルへ関数を足すか、新規 `tests/*.rs` に複製するか（裁定候補 6） |
| 5.6 WARN 件数テスト | `actor_region_warn_tests.rs` | **既存**（雛形）／**欠落** | 同ファイルへ足す（380 行・余裕あり）か兄弟新設 |
| 5.7 見張りを赤くしない | `file_length_guard_test.rs`・`lib.rs:397`／`:424` | **制約** | 新設 `src/` ファイルは `PURE_SOURCES` へ登録（`windows` 非依存であること）。`region.rs` 余白 49 行 |

### 3.6 要件 6・7（文書・申し送り）

§2.5 のとおり対象はすべて実在。ギャップは無く、作業は「書く」だけ。要件 6.6 に従い、COMPAT §8 `:183`／`:209` の陳腐化した行番号引用（`region.rs:292-294`／`:231-234`）を「何の定義行か」へ指し直すかは裁定候補 10。

## 4. 実装アプローチの選択肢

### 4.1 共通部分（どの案でも同じ）

1. `resolve_origin_component` の範囲外の腕を `corner` を返す形へ（撤去前と同型）。
2. 既存テスト 4 本の期待値と doc の追随・兄弟ファイルの表の新設・検体テスト・`PURE_SOURCES` 登録。
3. `region.rs` 冒頭 doc・`TextRegion::resolve` doc・COMPAT §8 の 3 行。

差が出るのは**要件 3（WARN を装着 1 回につき 1 件）の実現方法**だけである。

### 4.2 案 A: 既存の型と登録口を拡張する（`TextRegion` に「無視した宣言値」を載せる）

- `TextRegion` にフィールドを 1 つ足す（例: `ignored_origin: (Option<f32>, Option<f32>)`＝範囲外ゆえ無視した「宣言どおりに解決した値」を成分ごとに保持。範囲内・未宣言は `None`）。`resolve_origin_component` の返値を `(f32, Option<f32>)` にするか、2 つの出力引数にする。getter は `pub(crate)` で足りる。
- `actor.rs` に `warn_ignored_origin(resolved, previous)` を `warn_coarse_wrap_threshold` の隣へ置き、`register_actor` から続けて呼ぶ。判定は `previous == Some(region)` の同じ比較。欄は `balloon`・`key`（`origin.x`／`origin.y`）・`resolved`（無視した宣言値）・`range_min`／`range_max`（`TextRegion` の辺）・`corner`（`start` の当該成分）。
- 解決側の `debug!` は残しても消してもよい（裁定候補 2）。

| 観点 | 評価 |
|---|---|
| 変更点 | `region.rs`（関数 1・フィールド 1・getter 1）・`actor.rs`（関数 1・呼出 1 行）。新規ファイルはテストのみ |
| 先例との整合 | 折返し基準の警告と**同じ場所・同じ回数規律・同じ比較**。「解決側に粗さの記録を持たせない」規律を守る |
| 副作用 | `TextRegion` は `Copy`＋`PartialEq` のまま（`Option<f32>` 2 つは `Copy`）。`PartialEq` は再追従の判定キーに使われるが、新フィールドは model と原寸から決定的に導かれるので同値判定の結果は変わらない。構造体リテラル構築は `resolve` の 1 か所だけなので波及 0 |
| 行数 | `region.rs` に +10〜15 行（余白 49 行の内）。書き直す既存テスト①を兄弟ファイルへ移せば相殺できる |
| リスク | 低。`TextRegion` の意味（「解決済み領域」）に診断情報が 1 つ混ざる点だけが設計上の判断 |

### 4.3 案 B: 新しい部品を作る（範囲判定の純粋関数を切り出し、登録前の経路で再計算する）

- `region.rs` に `pub(crate) fn origin_range_notices(model, image_size, mode) -> [Option<Notice>; 2]` のような純粋関数を新設し、`ResolvedBalloonText::resolve_with_background`（`actor_decoration.rs:95`・model を持つ）で呼んで `ResolvedBalloonText` に新フィールドとして載せる。登録口は案 A と同じ比較で WARN。
- `TextRegion` は無改変。

| 観点 | 評価 |
|---|---|
| 変更点 | `region.rs`（関数 2）・`actor_decoration.rs`（フィールド 1・呼出 1）・`actor.rs`（関数 1・呼出 1） |
| 問題 | 範囲判定と書字開始角の選択が **2 か所**になる（`resolve` の中と新関数）。`resolve` の doc `:306-308` が「引き直すと縮退や負値解決が 2 か所に増える」と明示的に避けている形。片方だけ直る事故の温床 |
| リスク | 中（重複ロジック）。行数は案 A より多い |

### 4.4 案 C: 解決側で WARN を書き、回数は解決側で抑える

- `resolve_origin_component` の範囲外の腕で `warn!` を書く。毎フレームの重複は、`TextRegion::resolve` の呼び手が再追従（毎フレーム）か装着かを区別できないので、静的な「一度きり」機構（`OnceLock`／`HashSet` 等）で抑えることになる。

| 観点 | 評価 |
|---|---|
| 問題 | ⑴ 純粋層に可変の状態が入る（決定論テストの前提「同一入力→同一出力」が記録の面で崩れる）⑵ 「読み込み 1 回につき 1 件」の意味を持つ層は登録口だ、という 2026-09-06 の裁定と正面から逆行 ⑶ `region_vertical_canon_tests.rs:505` 付近の `warn == 0` と `region_inline_limit_tests.rs` の「解決は警告を書かない」規律が赤くなる ⑷ バルーンを替えて同じ定義に戻したとき（再装着）に 2 件目が出ない |
| 評価 | **採らない**。要件 3.2 の「装着 1 回につき」を解決側では表現できない |

### 4.5 推奨

**案 A**。理由: 先例（折返し基準の警告）と場所・回数規律・比較式が完全に揃い、範囲判定を 1 か所に保てる。新設するのは関数 1 つとフィールド 1 つで、要件 3.3 の欄もすべて `TextRegion` から読める。

## 5. 設計判断の候補（要件ディスカッションへ）

1. **範囲外の事実を登録口へ届ける器**——案 A（`TextRegion` にフィールド）／案 B（`ResolvedBalloonText` に別計算で載せる）。推奨は A。判断が変わるのは「`TextRegion` に診断情報を混ぜてよいか」の一点。
2. **解決側の `debug!` を残すか**——残す: 無記録経路の懸念が構造上消える・既存テストの `counts.debug` の期待値（範囲外成分 1 つにつき 1 件）を据え置けて書き直しが小さい。消す: 記録が登録口 1 か所に集まり、毎フレームの DEBUG 行が減る。要件 3.5 はどちらでも満たす。推奨は残す（文言だけ「範囲外ゆえ書字開始角へ寄せる」へ）。
3. **WARN の欄の確定**——要件 3.3 の 4 項目（`key`・`resolved`・`range_min`／`range_max`・`corner`）に加え、先例と同じ `balloon = BALLOON_NAME_PLACEHOLDER` を載せるか。載せると `areka-P0-emo-text-canon-residue` 項目 14（名前への差し替え）の対象が 2 か所になる（申し送りが要る）。
4. **書き直す既存テスト①（`region.rs:677`）の置き場所**——`region.rs` の中で書き直す（要件 5.4 の字義）か、被覆を失わない前提で兄弟ファイルの表へ吸収して `region.rs` の行数を減らすか。余白 49 行と案 A の +10〜15 行の兼ね合い。
5. **新設兄弟ファイルの名前と補助関数の共有**——`region_origin_range_tests.rs`（案）。`model`／`resolve_counting`／`assert_capture_alive` は `region_vertical_canon_tests.rs` の private なので、複製するか `region.rs` の `tests::model` を `pub(super)` に上げて共有するか。
6. **検体テストの置き場所**——`tests/shipped_fixture_region_test.rs` に関数を足す（既存 5 本と同じ補助関数を使え、`nar-install` が寄せる共有ヘルパの対象ファイルが増えない）か、新規 `tests/offsetdpi_fixture_region_test.rs` を作る（要件 7.3 の「39 本目」として申し送り）。前者は「無改変のまま緑」（要件 2.3）の証跡ファイルを触ることになる点だけが難点——2.3 の証跡は既存 5 本の関数を触らないことで足りるか、ファイル単位の無改変が要るかを決める。
7. **端ちょうどの表の範囲**——要件 5.1 の「端ちょうど（両端）」を左右／上下の 4 辺すべてで、3 方向×2 成分に対して取るか（最大 24 ケース）。判定式は成分・方向に依らないので、代表ケースへ絞る余地がある。
8. **退化した validrect との組み合わせ**——`right <= left`（例: 基層のみの全 0）では範囲が `[0,0]` になり、`origin 0` は範囲内・`origin 5` は範囲外→書字開始角 0。既存の退化 `warn!`（`:275-283`）とは別件として現行どおり据え置くか、表に 1 行足して固定するか。
9. **`\_l` 追随の確認の記録形式**——§3.4 の表を design.md か実装タスクの記録へ写す。着手時に引き直す（要件 6.6）。
10. **COMPAT §8 `:183`／`:209` の陳腐化した行番号引用**——要件 6.2／6.3 で当該行を触るついでに「何の定義行か」（`TextRegion::resolve` の `start_corner`／`start` の解決）へ指し直すか、文言の言い直しに限るか。
11. **`debug!`／`warn!` の文言**——撤去前の「validrect 外の origin 成分を書字開始角へ寄せる（クランプ正準）」を復元するか、「クランプ正準」を落として「範囲外の宣言は無視して書字開始角へ寄せる」等の平易な語にするか。既存の手順書はどちらの文言も grep していない（§2.5）。

### 5.1 要件ディスカッションでの決着（2026-09-18・開発者指示「深掘りし、自明な解があれば採用せよ」）

上の 11 項目と、ディスカッションで追加した 1 項目を実コードで引き直し、次のとおり決着した。設計はこれを前提にしてよい（実測が食い違ったときだけ差し戻す）。

| 項目 | 決着 | 決め手（実測） |
|---|---|---|
| 1 器 | **案 A**——`TextRegion` に「無視した宣言の解決値」を成分ごとに持たせ、登録口 `register_actor` で先例と同じ `previous == Some(region)` の比較で 1 件に絞る | 登録口は `ResolvedBalloonText` しか受け取らず（`actor.rs` の `register_actor`）、解決は毎フレーム走る（`refresh_actor_binding` が判定前に必ず解き直す）。「装着 1 回につき 1 件」の意味を持つ層は登録口だけ、と先例の doc が明言している。案 B（WARN を落とす）は brief の「作者に分かる形で記録する」に反し、案 C は純粋層に可変状態を入れる。欄が `PartialEq` に入ることは正しい向きに働く（宣言値が変われば新しい値で 1 件） |
| 2 解決側の `debug!` | **残す**（文言だけ改める） | 未宣言の成分も現に毎フレーム `debug!` を書いており、範囲外だけ消す理由が無い。既存の件数検査（成分 1 つにつき 1 件）の形を据え置ける |
| 3 WARN の欄 | 要件 3.3 の 4 項目＋先例と同じ `balloon = BALLOON_NAME_PLACEHOLDER` | 同じ定数を共有するので、`areka-P0-emo-text-canon-residue` 項目 14 の差し替え先が 2 か所になる（同 spec へ申し送る）。面の区別は `range_min`（sakura 36／kero 24）で付く |
| 4 `region.rs` の既存テスト 1 本 | **その場で期待値を書き直す**（`(100.0, 0.0)` → `(100.0, 46.0)`） | 撤去の経緯を語る doc 段落が減るので行数は増えない。移す必要が無い |
| 5 兄弟ファイル | **新設しない**——`region_vertical_canon_tests.rs` へ足す（要件 5.2 を改訂済み） | 同ファイルは冒頭 doc で「`origin` 解決そのものの判断分岐も固定する」（分岐 5〜9）と宣言済みで、`model`／`resolve_counting`／`assert_capture_alive` を持つ。677 行＝余白 323 行。`#[path]` 宣言・走査対象の登録・補助関数の複製がすべて不要になり、`region.rs` の増分も欄と腕の差し替えだけ（951 → 970 行前後）に収まる |
| 6 検体テスト | `tests/shipped_fixture_region_test.rs` へ関数を足す（要件 2.3 を改訂済み） | 既存の補助関数を使え、`nar-install` へ渡す 39 本目が生じない（要件 7.3 は条件付きのまま空振りする） |
| 7 端の表の網羅度 | **絞らない**——方向×成分×場合を繰り返しで生成する | 行はデータなので全数でも数十行。手で代表を選ぶと選び方が検査の穴になる |
| 8 退化した `validrect` | **据え置き・表に足さない** | 退化時は描画が空領域へ縮退し開始点は観測されない。既存の退化の警告が別に出る |
| 9 `\_l` 確認表の置き場所 | **本書 §3.4 を正本**とし、実装時に引き直して同じ場所を更新する | 写しを増やすと片方が古びる |
| 10 §8 の陳腐化した行番号 | 触る行は「何の定義行か」で指し直す（要件 6.6 へ追記済み） | — |
| 11 記録の文言 | 平易な語（要件 3.3 へ追記済み）。「クランプ正準」は戻さない | — |
| 追加: 遠い側の辺ちょうど | **範囲内のまま**（要件 1.3 へ理由を追記済み） | 判定式は現行の記録の判定と同一なので、この点の挙動は修正の前後で変わらない。「文字が置ける余白」で線を引く案は閾値が恣意（1 画素手前でも置けない）で、観測された欠陥にも対応しない |

## 6. 規模とリスク

- **規模: S**（1〜3 日）。本番コードは `region.rs` の 1 関数＋フィールド 1 つ・`actor.rs` の関数 1 つ。テストは既存 4 本の書き直し＋兄弟ファイル 1 本＋検体 1 本＋WARN 件数 1 本。文書は COMPAT §8 の 3 行と `region.rs` の doc 2 か所。
- **リスク: 低**。撤去前に存在した形の復元で、先例と同じ警告配置。唯一の注意点は「範囲外の腕を潰して赤を確認する」手順を省かないこと（要件 5.3）と、`region.rs` の行数余白（49 行）。

## 7. 設計フェーズへの申し送り

- **推奨: 案 A**（`TextRegion` に無視した宣言値を載せ、登録口で先例と同じ比較で WARN）。
- **着手時に引き直す file:line**: `region.rs` `:24-37`／`:242-243`／`:286-303`／`:432-458`／`:677`／`:946-951`、`actor.rs` `:188-200`／`:315-333`、`region_vertical_canon_tests.rs` `:42-54`／`:446`／`:576`／`:617`、`lib.rs` `:182`／`:356`／`:424`、`doc/COMPAT_ARCHITECTURE.md` `:177`／`:183`／`:209`、`roadmap.md` `:99`／`:149`。
- **調査が要る項目（Research Needed）**:
  - `emo2-kakukaku-offsetdpi` の 2 枚の PNG の IHDR 原寸（原本と同一の見込み。新設テストで突合して確定）。
  - `nar-install` の共有ヘルパの名前と置き場所（同 spec の設計待ち）。本仕様の検体テストは `shipped_fixture_region_test.rs` と同じ `CARGO_MANIFEST_DIR` 相対の形に揃えておけば後着側が機械的に寄せられる。
  - 実機での確認手順（任意）: `areka.exe <emo2 の絶対パス> <emo2-kakukaku-offsetdpi の絶対パス>` で「ちがうよう。」の行頭が欠けないこと＋WARN が 2 面×2 成分＝4 件（装着 1 回）であることの目視。決定論テストが主で、実機は補助。

## 8. 設計フェーズの発見と統合（2026-09-18・`/kiro-spec-design -y`）

発見の種別は **light**（既存システムの拡張・外部依存の追加 0・新しい技術 0）。§5.1 の決着 12 項目はすべて実コードで引き直し、**覆すものは 0 件**だった。設計で新しく確定したのは次の 6 点である。

### 8.1 実測で確定したこと

| 項目 | 実測 | 設計への帰結 |
|---|---|---|
| `actor.rs` の余白 | 970 行（上限 1,000・余白 30）。先例 `warn_coarse_wrap_threshold` は doc 込みで 41 行 | 警告の関数本体を `actor.rs` に置くと 1,000 行付近に達する。**本体は子モジュール `actor_decoration.rs`（141 行）へ `pub(super)` で置き、`register_actor` から `decoration::warn_ignored_origin` として呼ぶ**。同ファイルは既に「`actor.rs` が 1,000 行の見張りの間近にあるため」という理由で `build_actor_render`／`glyph_styles_of` を引き受けており（同ファイルの doc）、同じ理由・同じ形である。§5.1 項目 1 の決着（登録口で、前回領域との比較で 1 件に絞る）は変わらない——変わるのは関数の置き場所だけ |
| 検体の PNG 原寸 | `emo2-kakukaku-offsetdpi/balloons0.png`＝400×224・`balloonk0.png`＝288×203（IHDR の 16〜23 バイト目を直読） | §7 の「調査が要る項目」の 1 つ目は解消。原本と同寸なので、検体テストは既存の定数（`SAKURA_IMAGE_SIZE`／`KERO_IMAGE_SIZE`・`*_EDGES`・`*_START`・`*_WRAP`）をそのまま期待値に使える |
| `TextRegion` の構築点 | 構造体リテラルは `resolve` の末尾の 1 か所だけ（`TextRegion {` の全域検索。ほかのヒットは全て `-> TextRegion {` の関数定義） | 欄の追加の波及は 0 |
| 本番の解決経路 | 本番で `TextRegion::resolve` を呼ぶのは `actor_decoration.rs` の `ResolvedBalloonText::resolve_with_background` の 1 か所で、結果は必ず `register_actor` を通る（`canvas.rs` のヒットは同ファイル内のテスト） | 「解決側 `debug!`＋登録口 `warn!`」で無記録の経路は生じない（要件 3.5） |
| 書き直す 4 本の名前の参照 | 名前を参照しているのは `region.rs` の `resolve_origin_component` の doc 1 か所だけ | 意味が反転する 4 本は名前も改める（design.md「T-書直し」）。doc の書き直しで参照は消える |
| `shipped_fixture_region_test.rs` の検査関数の数 | **6 本**（§2.3／§5 の「既存 5 本」は数え違い。実在＝ファイル実在・PNG 原寸・書字方向・sakura・kero・wplimit 複製） | 要件 2.3 の「既存の検査関数を 1 つも書き換えない」の対象は 6 本 |

### 8.2 設計で気づいた落とし穴（実装への申し送り）

- **`TextRegion` の全体比較**: 欄 `ignored_origin` は `PartialEq` に入る。範囲外の宣言を持つ領域と、同じ位置へ落ちる未宣言の領域は、開始点が同じでも**同値ではなくなる**。検体テストで「原本 `emo2-kakukaku` と `==`」と書くと赤くなるので、成分ごとの比較（既存の `assert_region`）を使う。該当する既存テストは 0 本（範囲外の値を使う既存テストは書き直す 4 本だけで、いずれも全体比較をしない）。
- **検体テストは宣言の存在を前提として確かめる**: 検体から `origin` の 2 行が消えると、テストは未宣言の縮退を測るだけになり規則を固定しなくなる。`model.origin().x() == Some(0)`（y も）を前提として同じテストに置く。同ファイル冒頭 doc の禁則（宣言された生値を assert しない）は原本 `emo2-kakukaku` のテストに向けたものなので、対象を明記して両立させる。
- **表の「辺ちょうど」の行は開始点だけでは内と外を見分けられない**: 近い辺ちょうど（横書きの x＝36 など）は宣言値と書字開始角が一致する。区別は `ignored_origin()` が `None` であることと `debug` 0 件が担う。
- **既存の WARN 件数テスト 6 本は WARN の総数を数えている**: いずれも `origin` を宣言しない入力なので無改変で成り立つ。新設のテストは折返しの警告と混ざらないよう、本警告の文言と一致するものだけを数える。
- **`shipped_fixture_region_test.rs` は `areka-P0-nar-install` が書き換える 38 ファイルの 1 つ**（検体パスを参照している）。§5.1 項目 6 の決着どおり同ファイルへ足すが、追加分（根パス関数とテスト）は**ファイル末尾にまとめ**、既存の `shipped_root()`／`wplimit_root()` の近傍を触らない——後着側の取り込みが行の追記どうしになる。
- **ukadoc 網羅台帳は触らない**: `doc/ukadoc-coverage/ledger/assets.toml` の `origin.x`／`origin.y` は「実装済み・記録なし（正典どおりに書かれた宣言について）」のまま正しい。ukadoc は範囲外の宣言に沈黙しており、判定は変わらない。台帳を触ると briefing と roadmap-draft の数の検査が連動する。
- **`doc/ukadoc-coverage/briefing-*.md` は COMPAT §8 の行の題を引用している**が、`\_l` の行は既に旧題のまま食い違っており、題の一致を見張る検査は無い（`COMPAT_ARCHITECTURE` を読むコードは 0）。§8 の題を改めても赤くならない。本仕様では触らない。

### 8.3 統合（synthesis）

- **一般化**: 要件 1（範囲外の解決）と要件 3（記録）は「範囲判定 1 回の結果を 2 つの消費者が使う」同じ問題である。判定を `resolve_origin_component` の 1 か所に保ち、返値を `(開始点の成分, 無視した宣言の解決値)` の組にして両方へ配る。折返しの警告との共通化（警告の汎用部品化）は**しない**——2 種類しかなく、欄も判定も違う。
- **作るか借りるか**: すべて既存の借用。範囲判定の式（現行の記録の判定）・書字開始角（`start_corner`）・件数規律（`previous == Some(region)`）・プレースホルダ定数・テスト補助（`resolve_counting`／`assert_capture_alive`／`merged_model`／`assert_region`／`capturing`）。新規は欄 1 つ・読み口 1 つ・関数 1 つ。
- **簡素化**: 新規ファイル 0・新しい型 0（欄は `(Option<f32>, Option<f32>)` の組で足り、専用の構造体は作らない）・設定 0。退化した `validrect` の分岐は足さない。読み口は `pub(crate)`（crate の外に消費者がいない）。

### 8.4 行数の見込み（1 ファイル 1,000 行の見張り）

| ファイル | 現在 | 見込み |
|---|---|---|
| `region.rs` | 951 | 965〜975（985 を超える見込みになったら止めて報告） |
| `actor.rs` | 970 | 973〜977（関数本体は置かない） |
| `actor_decoration.rs` | 141 | 175〜190 |
| `region_vertical_canon_tests.rs` | 677 | 790〜840 |
| `actor_region_warn_tests.rs` | 380 | 500〜540 |
| `tests/shipped_fixture_region_test.rs` | 397 | 445〜465 |

## 9. 要件 5.3 の記録——判断を一時的に壊してテストが赤くなることを確かめた（2026-09-19）

設計 `## Testing Strategy` の `### 規則を固定していることの確認（要件 5.3）と見張り（要件 5.7）` の手順 3 を実施した記録である。目的は「テストが本当にこの規則を固定しているか」を確かめることにあり、判断を壊した実装でテストが緑のままなら、そのテストは規則を固定していないことになる。以下は実際に壊し、実際に赤くなり、元へ戻したことの記録である。

壊し方は 3 つある。9.2 と 9.3 は設計が定める 2 つ、9.4 は後から足した 1 つで、足した理由は 9.4 に書いた。

再現手順はいずれも同じで、対象の 1 か所を書き換えてから次を走らせる。

```
cargo test -p areka-emo-text --no-fail-fast
```

### 9.1 出発点（壊す前）

- 走行結果: 対象 17 個すべて ok。合計 895 passed / 0 failed / 2 ignored。
- `crates/areka-emo-text/src/region.rs` の sha256: `19c7f93378d8afe4d9c8f0641d01f328ff4b041d700a3dd9bc3ede86a21b12af`
- `crates/areka-emo-text/src/actor_decoration.rs` の sha256: `fd1c8d835c89ac7b031981434d43c69ddbf51cc52ccec9d45f09a21ba7b9a980`

書き換える前に両ファイルをバイト単位で控え、戻すときはその控えからバイトごと複写した（行末の種類が変わると中身が同じでも sha256 が変わるため、文字列置換で戻してはならない）。

### 9.2 壊し方 1——範囲の外に宣言された成分を、そのまま開始点に使う形へ戻す

`crates/areka-emo-text/src/region.rs` の `resolve_origin_component` の、範囲の外と判定したときの返値だけを書き換えた（範囲の判定式・範囲の中の腕・未宣言の腕・`debug` の記録はいずれも触っていない）。

変更前:

```rust
                (corner, Some(resolved))
```

変更後（取り下げた旧規則「宣言された値はつねにそのまま使う」への後戻り）:

```rust
                (resolved, Some(resolved))
```

赤くなったテスト（走行出力のとおり）:

- ライブラリの対象: 783 passed / 9 failed / 2 ignored
  - `region::tests::out_of_range_origin_component_falls_back_to_start_corner_independently`
  - `region::vertical_canon_tests::origin_range_table_holds_for_every_mode_and_component`
  - `region::vertical_canon_tests::declared_origin_outside_validrect_falls_back_to_start_corner_with_one_debug_per_component`
  - `region::vertical_canon_tests::negative_origin_resolves_from_opposite_edge_then_is_range_checked`
  - `region::vertical_canon_tests::declared_origin_follows_validrect_only_when_it_leaves_the_range`
  - `actor::region_warn_tests::attaching_a_balloon_with_both_origin_components_outside_warns_once_per_component`
  - `actor::region_warn_tests::attaching_a_balloon_with_only_one_origin_component_outside_warns_once`
  - `actor::region_warn_tests::refresh_with_a_changed_region_warns_again_with_the_new_values`
  - `actor::region_warn_tests::ignored_origin_warnings_share_the_window_with_the_coarse_wrap_warning`
- `tests/shipped_fixture_region_test.rs` の対象: 6 passed / 1 failed
  - `offsetdpi_fixture_with_out_of_range_origin_starts_at_writing_corner`

残る 15 個の対象は ok のままだった。

表のテストは食い違いを 1 件見つけた時点で止まるので、出力に現れた行は 1 行だけである。現れたのは次の行だった。

```
近い辺の 1 つ外 / origin.x,35 / HorizontalTb: 範囲内なら解決後の値 35・範囲外なら書字開始角 36（最寄りの辺ではない）から書き始める
  left: 35.0
 right: 36.0
```

この走行で観測できたのは上の 1 行だけである。表の場合の一覧は「近い辺の 1 つ外」が先頭で、先頭の行で止まった走行はそれより後の行へ届いていない。

ここで大事なのは、この壊し方が**要件 1.7 の区別には触れていない**ことである。要件 1.7 は「落ちる先は最寄りの辺ではなく書字開始角である」と言うが、この壊し方は落ちる先を宣言値そのものへ変えるだけなので、最寄りの辺と書字開始角のどちらが正しいかは問われない。この 2 つを分ける赤は 9.4 で別に作った。

検体のテストの食い違いは次のとおりで、旧規則では宣言がそのまま開始点になるため `(0, 0)` へ落ちる。

```
emo2-kakukaku-offsetdpi scope 0 (sakura): 描画開始点
  left: (0.0, 0.0)
 right: (36.0, 46.0)
```

記録の件数を数える 4 本も一緒に赤くなった。これらは無視した宣言の値と実際に用いた角を組で読むため、開始点が変わると読み取る値も変わるからである。壊し方 2 の結果（下記）と合わせて読むと、件数のテストが数えているのは記録の件数だけでなく記録の中身でもあることが分かる。

戻した後: sha256 は `19c7f93378d8afe4d9c8f0641d01f328ff4b041d700a3dd9bc3ede86a21b12af`（出発点と一致）。

### 9.3 壊し方 2——無視した宣言の記録を何もしない形にする

`crates/areka-emo-text/src/actor_decoration.rs` の `warn_ignored_origin` の本体を丸ごと取り除き、引数を捨てるだけの形にした。

変更後:

```rust
pub(super) fn warn_ignored_origin(resolved: &ResolvedBalloonText, previous: Option<TextRegion>) {
    let _ = (resolved, previous);
}
```

赤くなったテスト（走行出力のとおり）:

- ライブラリの対象: 788 passed / 4 failed / 2 ignored
  - `actor::region_warn_tests::attaching_a_balloon_with_both_origin_components_outside_warns_once_per_component`
  - `actor::region_warn_tests::attaching_a_balloon_with_only_one_origin_component_outside_warns_once`
  - `actor::region_warn_tests::refresh_with_a_changed_region_warns_again_with_the_new_values`
  - `actor::region_warn_tests::ignored_origin_warnings_share_the_window_with_the_coarse_wrap_warning`
- 残る 16 個の対象はすべて ok のままだった。

装着の場合（設計の T-警告 a）の食い違いは次のとおりで、期待した 2 件に対して 0 件、捕まえた記録の一覧も空だった。

```
両成分が範囲の外なら装着で 2 件（要件 3.1／3.2）: []
  left: 0
 right: 2
```

「0 件」を主張する側が空振りしていないことは、折返しの警告と同じ捕捉の窓を共有するテストが示している。このテストは総数 3 件（折返し 1 件＋本記録 2 件）を期待しており、壊した実装でも折返しの 1 件は捕まったまま本記録の 2 件だけが消えた。つまり捕捉そのものは生きており、消えたのは記録の側である。

戻した後: sha256 は `fd1c8d835c89ac7b031981434d43c69ddbf51cc52ccec9d45f09a21ba7b9a980`（出発点と一致）。

### 9.4 壊し方 3——最寄りの辺へ寄せる形にする（設計が定める 2 つに加えて足した 1 つ・2026-09-19）

これは設計の手順 3 が定める 2 つの壊し方には**含まれない、後から足した 3 つめ**である。足した理由は次のとおり。

要件 1.7 は「範囲の外に宣言された成分が落ちる先は、**最寄りの辺ではなく書字開始角**である」と言う。これは本仕様でいちばん鋭い主張だが、9.2 と 9.3 のどちらの壊し方もこの区別に触れない——9.2 は落ちる先を宣言値そのものへ変えるだけで、最寄りの辺と書字開始角のどちらが正しいかは問わない。したがってこの主張だけが「壊したら赤くなる」ことを実際に見せていない状態だった。そこで、まさにその区別を壊す形を 1 つ作った。

`crates/areka-emo-text/src/region.rs` の `resolve_origin_component` の、範囲の外と判定したときの返値だけを書き換えた（範囲の判定式・範囲の中の腕・未宣言の腕・`debug` の記録はいずれも触っていない）。第 2 要素は `Some(resolved)` のまま残したので、変わるのは開始点だけである。

変更後（範囲の下側なら下端、上側なら上端＝最寄りの辺へ寄せる）:

```rust
                (
                    if resolved < range.0 { range.0 } else { range.1 },
                    Some(resolved),
                )
```

赤くなったテスト（走行出力のとおり）:

- ライブラリの対象: 788 passed / 4 failed / 2 ignored
  - `region::vertical_canon_tests::origin_range_table_holds_for_every_mode_and_component`
  - `region::vertical_canon_tests::declared_origin_outside_validrect_falls_back_to_start_corner_with_one_debug_per_component`
  - `region::vertical_canon_tests::negative_origin_resolves_from_opposite_edge_then_is_range_checked`
  - `region::vertical_canon_tests::declared_origin_follows_validrect_only_when_it_leaves_the_range`
- 残る 16 個の対象はすべて ok のままだった。

表のテストに現れた行は次のとおり。

```
近い辺の 1 つ外 / origin.x,35 / VerticalRl: 範囲内なら解決後の値 35・範囲外なら書字開始角 356（最寄りの辺ではない）から書き始める
  left: 36.0
 right: 356.0
```

現れた行は、あらかじめ見込んでいた「遠い辺の 1 つ外」ではなく「近い辺の 1 つ外」の縦書きの行だった。理由は書字開始角が書字方向で変わることにある。横書きでは x の書字開始角は左辺なので、左の外に宣言された値は最寄りの辺も書字開始角もどちらも左辺 36 になり、この壊し方では区別がつかない。`vertical_rl` では x の書字開始角は**右**辺 356 であるのに、左の外に宣言された値の最寄りの辺は左辺 36 のままで、両者が正反対を向く。表は横書きから縦書きへ順に回るので、この食い違いが先頭の場合で早々に現れた。見込みより鋭い行が先に出たのであって、期待した行を出すために手を入れたことはない。

同じ理由で、9.2 の壊し方では赤くなったのにこの壊し方では緑のまま残るテストが、合わせて 6 本ある。いずれも横書きで、しかも範囲の下側（左辺より左・上辺より上）に外れた値しか使わないため、最寄りの辺と書字開始角が同じ点になってしまうからである。

落ちる先そのものを直に見ている 2 本:

- `region::tests::out_of_range_origin_component_falls_back_to_start_corner_independently`（横書き・`origin.y,0` が上辺 46 の外）
- `offsetdpi_fixture_with_out_of_range_origin_starts_at_writing_corner`（横書き・検体の `origin` は `(0, 0)` で左上の外）

記録の中身を見ている 4 本も同じく素通りする。いずれも `origin.x,0`／`origin.y,0` を横書きで与えるので、記録の「実際に用いた書き始めの角」の欄は最寄りの辺へ寄せても書字開始角のまま（バルーンの左辺・上辺）で変わらない。9.2 ではこの欄が宣言値 `0.0` になって赤くなった。

- `actor::region_warn_tests::attaching_a_balloon_with_both_origin_components_outside_warns_once_per_component`
- `actor::region_warn_tests::attaching_a_balloon_with_only_one_origin_component_outside_warns_once`
- `actor::region_warn_tests::refresh_with_a_changed_region_warns_again_with_the_new_values`
- `actor::region_warn_tests::ignored_origin_warnings_share_the_window_with_the_coarse_wrap_warning`

つまり「最寄りの辺へ寄せる」後戻りを捕まえているのは、縦書きを回す表のテストと縦書きの場合を持つ 3 本だけであり、横書きだけのテストは——落ちる先を見るものも、記録の欄を見るものも——この後戻りを素通りさせる。将来この規則を触る人は、横書きの検体だけで確かめても足りないことを覚えておくとよい。

戻した後: sha256 は `19c7f93378d8afe4d9c8f0641d01f328ff4b041d700a3dd9bc3ede86a21b12af`（9.1 の出発点と一致）。

### 9.5 戻したことの確かめ

3 つの壊し方はいずれも、控えからのバイト単位の複写で元へ戻した。

- 両ファイルの sha256 が 9.1 の値と一致する。
- `git diff -- crates/` の出力が空である（走らせた書き換えは 1 つも残っていない）。
- 戻した後の `cargo test -p areka-emo-text --no-fail-fast` は対象 17 個すべて ok、合計 895 passed / 0 failed / 2 ignored で、出発点と同じ。

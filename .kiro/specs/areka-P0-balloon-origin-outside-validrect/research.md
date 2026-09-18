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
| roadmap 台帳 | `.kiro/steering/roadmap.md:99`（#38）・`:149`（A0 ウェーブ） | 要件 7.1（完了時） |
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

### 3.4 要件 4（`\_l` の原点）——全数確認の結果

`\_l` の数値座標の原点は `cursor_tag.rs::resolve_cursor_axis` が `CursorBasis.origin` から読み、その値は `TextRegion::start()` 由来（COMPAT §8 `:183`）。解決規則を変えれば自動で追随する（要件 4.1 は無改変で成立）。

**宣言された `origin` を `TextRegion::resolve` に渡している既存テストの全数**（方法: ⑴ `Origin::new(Some` の全文検索 ⑵ `model(`／`model_rect(`／`model_of(` の呼出で第 1 引数に `(Some(` を持つものの 2 行検索 ⑶ origin を `(0,0)` に固定する補助関数 4 つ〔`layout_hard_limit_tests.rs:100` `model_of`・`viewbox_test_support.rs:25` `model_rect`・`viewbox_draw_test_support.rs:84`・`actor_test_support.rs:55`〕の呼出側の validrect を列挙。対象は `crates/areka-emo-text/{src,tests}` と `crates/areka/src`。`areka-parsers` は解析層のテストで解決を通らないので対象外）:

| 場所 | 宣言 origin | 解決後 validrect（当該軸） | 判定 |
|---|---|---|---|
| `layout_cursor_center_origin_tests.rs:107` `region_declared`（V7・`\_l` の原点テストの本体）・`:559`・`:623` | (50, 20) | left 30／top 8／right 350／bottom 210 | **範囲内** |
| `layout_cursor_wiring_tests.rs:64` `region_c_declared_origin`（`:193` で使用） | (100, 60) | left 40／top 20／right 360／bottom 200 | **範囲内** |
| `cursor_tag_tests.rs:48`・`cursor_tag_resolve_tests.rs:836`（`ORIGIN` 定数 (50,20)） | — | `CursorBasis` に直接入れており **`TextRegion::resolve` を通らない** | 対象外（影響なし） |
| `layout_cursor_tests.rs:13`（`layout_test_support::model`＝validrect 全 None） | (0, 0) | 画像端 0..400／0..224 | **端ちょうど＝範囲内** |
| `layout_visible_window_tests.rs:282` | (0, 40) | left 0／top 40 | **両成分とも端ちょうど＝範囲内** |
| `layout_hard_limit_tests.rs:133`／`:136`（`model_of`・origin (0,0) 固定） | (0, 0) | left 0／top 0 | 端ちょうど＝範囲内 |
| viewbox 系 `canvas_for` 経由 11 か所（`viewbox_dirty_tests.rs` 7・`viewbox_plan_commit_tests.rs` 4） | (0, 0) | すべて left 0／top 0 | 端ちょうど＝範囲内 |
| `viewbox_scroll_test.rs:131` | (0, 0) | (0, H, 0, W) | 端ちょうど＝範囲内 |
| `actor_choice_contract_tests.rs:799`・`actor_decoration_tests.rs:31`・`actor_runtime_frame_tests.rs:414`・`actor_test_support.rs:55`・`draw_oracle_tests.rs:566`／`:723`・`viewbox_draw_test_support.rs:84`・`tests/attach_wiring_test.rs:291`・`canvas.rs:488`／`:730` | (0, 0) | validrect 全 None＝画像端 | 端ちょうど＝範囲内 |
| `region.rs:575`（基層のみ・validrect 全 0） | (0, 0) | [0, 0] | 端ちょうど＝範囲内（退化矩形の warn は別件で不変） |
| `region.rs:650`・`:693`・`:894`／`:928` | (100,50)／(−100,−100)→(300,124)／(120,70) | 範囲内 | 範囲内 |
| `region.rs:679`・`region_vertical_canon_tests.rs:446`／`:576`（2 ケース）／`:617` | 範囲外を意図的に使用 | — | **撤去を固定している 4 本＝要件 5.4 の書き直し対象**（`\_l` の原点は見ていない） |

結論: **`\_l` の原点に宣言された `origin` を使う既存テストで範囲外の値を使うものは 0 本**（対象 `crates/areka-emo-text/{src,tests}`・`crates/areka/src`・上記 3 手法）。範囲外を使うのは撤去そのものを固定する 4 本だけで、いずれも `\_l` を扱わない。要件 4.2／4.3 はこの表をそのまま記録に流用できる（設計段階で着手時に引き直すこと）。

**副産物**: 「端ちょうど＝範囲内」を前提に書かれたテストが `(0,0)`×`left 0／top 0` の組で 20 か所以上ある。要件 1.3 の両端包含は好みではなく既存資産の生存条件である。

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

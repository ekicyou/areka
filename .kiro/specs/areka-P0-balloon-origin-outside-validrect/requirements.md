# Requirements Document

## Project Description (Input)

バルーン定義に `origin.x,0`／`origin.y,0` のように、文字を描いてよい範囲（`validrect`）の**外**に文字描画開始点を宣言してある第三者のバルーンを areka で使うと、**各行の行頭が 1 文字ぶん欠けて表示される**（2026-09-18・開発者の実機目視。ゴースト emo2 開発版同梱のバルーン `emo2-kakukaku` で「ちがうよう。」が「がうよう。」になった）。同じバルーンは SSP では欠けずに読める。

現状の areka は、宣言された `origin` 成分を `validrect` の内外を問わず宣言どおりの位置として用いる（完了 spec `areka-P0-balloon-vertical-canon` 要件 3.10・PR #124 で areka 独自の寄せ戻しを撤去）。文字の面は `validrect` と同じ範囲しか持たないので、範囲の外から書き始めた文字は表示されずに切り落とされる。撤去の根拠「範囲外の宣言は SSP でも壊れた定義」は SSP を見ずに置いた仮定で、上の目視が反証した。areka 同梱の検体が無傷なのは、同じ PR が検体 5 本から `origin` の宣言を消したからにすぎない。

本仕様は、**範囲外に宣言された `origin` 成分を「宣言が無いもの」として扱い、書字開始角へ落とす**形へ戻す（撤去前の形の復元・成分ごとに独立・3 書字方向）。範囲内の宣言と未宣言の挙動は不変。範囲外の宣言を無視したことは、バルーンの作者に分かる形で記録する。撤去を固定しているテストと文書（`region.rs` 冒頭 doc・`doc/COMPAT_ARCHITECTURE.md` §8）を追随させる。

正典の根拠（ukadoc `descript_balloon.html`）: `origin.x`＝「テキスト開始位置のX座標。……通常は指定せずvalidrectの定義に任せる場合が多い。横書き=validrect.left 縦書き=validrect.right」（`origin.y` も同文・既定は `validrect.top`）。`validrect`＝「テキストを描画してよい範囲。」範囲の外を宣言したときの扱いは ukadoc のどこにも書かれていない（沈黙）。「描画してよい範囲」の外には描かず、かつ文字を欠かさない、の 2 つを同時に満たす解は「範囲の内へ戻す」しかない。

## Introduction

本仕様は、areka のバルーン文字表示における**文字描画開始点（`origin`）の解決規則**を 1 点だけ改める。すなわち「`validrect` の外に宣言された `origin` 成分は、宣言されていないものとして書字開始角へ落とす」。これは 2026-08-27 の裁定（完了 spec `areka-P0-balloon-vertical-canon` 要件 3.10）で撤去した規則の復元であり、撤去の前提「範囲外の宣言は SSP でも壊れた定義」が 2026-09-18 の実機目視で反証されたことを理由とする。

利用者から見える差は 1 つだけである——**`origin` を `validrect` の外に宣言したバルーンで、行頭が欠けずに読める**。範囲内に宣言したバルーンと、宣言していないバルーン（areka 同梱の検体を含む）の表示は 1 ピクセルも動かない。

規模は S。編集するソースは開始点を解決する 1 関数とその冒頭 doc、テストは兄弟ファイルの新設、文書は `doc/COMPAT_ARCHITECTURE.md` §8 の 3 行である。

## Boundary Context

- **In scope**:
  - `validrect` の外に宣言された `origin` 成分の解決（成分ごとに独立・横書き／縦書き右送り／縦書き左送りの 3 方向）
  - 範囲外の宣言を無視したことの記録（レベル・回数・欄）
  - `\_l` の数値座標の原点が「解決後の開始点」であることの不変の確認（既存テストの全数確認を含む）
  - 撤去を固定している既存テストの書き直しと、範囲外／範囲内／端ちょうどの表の新設（兄弟ファイル）
  - 範囲外を宣言した実物の検体 `emo2-kakukaku-offsetdpi` を使った 1 本の検査（検体は書き換えない）
  - `region.rs` 冒頭 doc・`doc/COMPAT_ARCHITECTURE.md` §8 の該当 3 行の追随（撤去の取り下げ・理由・日付）
- **Out of scope**:
  - `validrect` の内に宣言された `origin` の扱い（不変・宣言どおり）
  - 未宣言の `origin` 成分の縮退（不変・書字開始角）
  - 折返し基準（`wordwrappoint`）が描画範囲の外に解決される件（`areka-P0-balloon-canon-residue` 項目 14 の所有・別の欠陥）
  - プロパティ `currentghost.balloon.scope(ID).basepos.x`／`.y` の公開（`areka-P0-currentghost-property-tree`・α 後。値は解決後の開始点から導けば足りる、とだけ申し送る）
  - SSP が範囲外の `origin` を内部でどう処理しているかの実測（SSP 実測主義は取らない。根拠は ukadoc の 2 文と目視 1 件で足りる）
  - 文字の面の寸法・描画・折返し・`\_l` の解決の各実装（いずれも解決後の開始点を受け取るだけで、無改変のまま追随する）
  - バルーン定義の読み取り（`origin` は今までどおり宣言の有無を潰さずに運ぶ）
  - 上流 `ghost_dev` の emo2（2026-09-18 に 2 行を削除済み）
- **Adjacent expectations**:
  - 完了 spec `areka-P0-balloon-vertical-canon` の要件 3.10（宣言どおりに用いる）を本仕様が**上書き**する。アーカイブ本体は改変せず、上書きの事実を `doc/COMPAT_ARCHITECTURE.md` §8 と本仕様に記録する（先例＝同表の `areka-P0-kero-balloon`・`areka-P0-scope-chain-gap`・`areka-P0-cursor-tag-canon` の上書き行）。同 spec の要件 3.11（未宣言の縮退）と 3.7（負値は反対端基準）は不変のまま引き継ぐ
  - 完了 spec `areka-P0-cursor-tag-canon`（`\_l` の原点＝解決後の `origin`）は無改変。解決規則を変えれば `\_l` は自動で追随する
  - 並走中の `areka-P0-nar-install` が検体参照 38 ファイルを共有ヘルパへ寄せる。本仕様が新設するテストが検体 `emo2-kakukaku-offsetdpi` を参照するときは、既存の `shipped_fixture_region_test.rs` と同じ参照の仕方に揃え、後着側が取り込める形にする。`doc/COMPAT_ARCHITECTURE.md` §8 は行の追記どうしなので後着が取り込む
  - `areka-P0-emo-text-canon-residue`（α 後）の項目 14 が `region.rs` の定数 `BALLOON_NAME_PLACEHOLDER` を差し替える＝同じファイル。本仕様が先に着地する
  - 下流 `areka-P0-default-balloon-bundle`（既定バルーンの `origin` がどう書かれていても欠けない）・`areka-P0-alpha-release-signoff`（第三者のバルーンの持ち込み）は本仕様の着地を前提にできる

## Requirements

### Requirement 1: 範囲外に宣言された `origin` 成分の解決

**Objective:** As a バルーンの作者（第三者）, I want `origin` を `validrect` の外に宣言してあるバルーンでも行頭が欠けずに表示されること, so that SSP で読めているバルーンを areka へそのまま持ち込める

#### Acceptance Criteria

1. When 宣言された `origin` 成分の解決後の値が `validrect` の当該軸の範囲の**外**にあるとき, the areka バルーン文字表示 shall その成分を**宣言されていないもの**として扱い、書字開始角の当該成分（横書き・縦書き左送り＝`validrect` 左上／縦書き右送り＝`validrect` 右上）を文字描画開始点に用いる。
2. The areka バルーン文字表示 shall 範囲の判定を **x と y の成分ごとに独立**に行う——片方の成分だけが範囲外のとき、範囲外の成分だけが書字開始角へ落ち、範囲内の成分は宣言どおりの値のまま残る。
3. The areka バルーン文字表示 shall 範囲の判定で**両端を範囲内とみなす**——解決後の値が `validrect` の辺とちょうど等しい成分は範囲内であり、宣言どおりの位置を用いる（現行の記録の判定と同じ両端を含む判定を、返す値の判定にもそのまま用いる）。
4. When `origin` 成分が負値で宣言されているとき, the areka バルーン文字表示 shall まず反対端基準で絶対値化し（完了 spec `areka-P0-balloon-vertical-canon` 要件 3.7・不変）、**その解決後の値**で範囲の内外を判定する。
5. The areka バルーン文字表示 shall 1.〜4. を横書き（`horizontal_tb`）・縦書き右送り（`vertical_rl`）・縦書き左送り（`vertical_lr`）の 3 書字方向で同じ規則として適用し、書字方向は書字開始角の選択にだけ効く。
6. When `origin` を `validrect` の外に宣言した実物の検体 `emo2-kakukaku-offsetdpi`（`origin.x,0`／`origin.y,0`・面別の `validrect.left` は sakura 36／kero 24）を本番と同じ 2 層マージの経路で解決したとき, the areka バルーン文字表示 shall 文字描画開始点を sakura `(36, 46)`・kero `(24, 40)` に置く（＝同じバルーンから `origin` の 2 行を消したときと同じ位置）。
7. When 横書きで `origin.x` が `validrect` の右辺より右に宣言されているとき, the areka バルーン文字表示 shall 文字描画開始点の x を `validrect` の**左辺**（書字開始角）に置き、右辺には置かない（最寄りの辺へ寄せると行頭が右端に張り付いて 1 文字ごとに折り返すため。縦書きの y についても同様に上辺＝書字開始角へ置く）。

### Requirement 2: 範囲内の宣言と未宣言の挙動の不変

**Objective:** As a areka を使うゴーストの利用者, I want いま正しく表示されているバルーンの表示が 1 ピクセルも動かないこと, so that 本修正で別のバルーンが壊れない

#### Acceptance Criteria

1. When 宣言された `origin` 成分の解決後の値が `validrect` の当該軸の範囲の内（両端を含む）にあるとき, the areka バルーン文字表示 shall 宣言どおりの位置から書き始める（現行と同一・完了 spec `areka-P0-balloon-vertical-canon` 要件 3.10 前半を引き継ぐ）。
2. When `origin` 成分が宣言されていないとき, the areka バルーン文字表示 shall 書字開始角へ縮退する（現行と同一・同 spec 要件 3.11 を引き継ぐ）。
3. The areka ワークスペース shall areka 同梱の検体（`origin` 未宣言の `emo2-kakukaku` とその複製）の文字描画開始点 sakura `(36, 46)`／kero `(24, 40)` を固定している既存テスト `shipped_fixture_region_test.rs` の**既存の検査関数を 1 つも書き換えずに**、修正の前後をまたいで緑に保つ（挙動不変の証跡＝PR #124 と同じ手）。同じファイルへ要件 5.5 の検査関数を足すことは妨げない——不変の証跡は既存の関数の期待値が動かないことで足りる。
4. The areka ワークスペース shall 本修正で `origin`・`wordwrappoint`・`validrect` の読み取り（バルーン定義の解析層）を変えない——宣言の有無はこれまでどおり解決側まで運ばれる。

### Requirement 3: 範囲外の宣言を無視したことの記録

**Objective:** As a バルーンの作者, I want 自分の宣言が無視されて書字開始角へ落ちたことに気づけること, so that 定義の粗さを自分で直せる

#### Acceptance Criteria

1. When 宣言された `origin` 成分が範囲外ゆえ書字開始角へ落ちたとき, the areka バルーン文字表示 shall そのことを **WARN レベル**で記録する（作者に分かる形。DEBUG は開発者向けであり作者には届かない。先例＝折返し基準が描画範囲の外に解決されたときの警告）。
2. The areka バルーン文字表示 shall その記録を**バルーン定義の読み込み（装着）1 回につき、範囲外の成分 1 つにつき 1 件**だけ書く——毎フレームの解決では繰り返さず、解決結果が変わらない再追従でも追加しない（先例＝折返し基準の警告の回数の規律）。
3. The areka バルーン文字表示 shall 記録に少なくとも、成分の名前（`origin.x`／`origin.y`）・宣言どおりに解決した値・`validrect` の当該軸の範囲（両端）・実際に用いた書字開始角の値を含める。記録の文はバルーンの作者が読んで意味の取れる平易な語で書き、プロジェクト内部の符牒を持ち込まない。
4. While 宣言された `origin` 成分が範囲内にあるか、または宣言されていないとき, the areka バルーン文字表示 shall 1. の WARN を出さない（範囲内の宣言と未宣言は粗さではない）。
5. The areka バルーン文字表示 shall 範囲外の宣言をログ無しで黙って落とす経路を持たない（ログ無し失敗経路の禁止）。

### Requirement 4: `\_l` の数値座標の原点の追随

**Objective:** As a ゴーストの作者, I want `\_l[x,y]` の数値座標が「実際の文字描画開始点」から測られ続けること, so that `origin` を範囲外に宣言したバルーンでもカーソル移動の着地が文字の位置と食い違わない

#### Acceptance Criteria

1. The areka バルーン文字表示 shall `\_l` の数値座標の原点を**解決後の文字描画開始点**（本仕様の規則を通した後の `origin`）に置く（完了 spec `areka-P0-cursor-tag-canon` の定義・無改変）——範囲外の宣言が書字開始角へ落ちたバルーンでは、`\_l[0,0]` はその書字開始角を指す。
2. The areka ワークスペース shall `\_l` の原点に宣言された `origin` を使う既存テストを**全数**確認し、それらの宣言値がすべて `validrect` の範囲内であること（＝本修正で期待値が動かないこと）を記録する。範囲外の値を使うものが混じっていたときは期待値を本仕様の規則へ見直す。
3. The areka ワークスペース shall 2. の確認結果として、範囲外の値を使う既存テストが **0 本**であるなら、その 0 を調べた範囲（対象ファイル）と方法とともに明示して残す（表や引き算で導ける 0 は沈黙と同じ）。

### Requirement 5: 決定論テスト

**Objective:** As a areka の開発者, I want 本修正の規則が実 DPI・実 GPU・実窓を要しないテストで固定されること, so that 以後の変更で同じ欠陥が黙って再発しない

#### Acceptance Criteria

1. The areka ワークスペース shall 3 書字方向 × 2 成分について、範囲外／範囲内／端ちょうど（両端）／負値の反対端基準解決後の範囲内と範囲外、の各場合の文字描画開始点を**表**の形で固定する決定論テストを持つ。
2. The areka ワークスペース shall 1. のテストを `region.rs` とは別の**兄弟ファイル**に新設する（`region.rs` は 951 行で、テストを同じファイルへ足すと 1 ファイル 1,000 行の目安を見張る `file_length_guard_test.rs` が赤くなる）。
3. The areka ワークスペース shall 「範囲外の宣言が書字開始角へ落ちる」ことを見るテストを、**範囲外の腕を潰した実装で赤くなることを確かめてから**採る（到達する経路を踏ませる。判断を潰しても緑のままなら、そのテストは規則を固定していない）。
4. The areka ワークスペース shall 撤去を固定している既存テスト（`region.rs` の `origin_components_resolve_literally_and_independently`、`region_vertical_canon_tests.rs` の `declared_origin_outside_validrect_is_literal_with_one_debug_per_component`・`negative_origin_resolves_from_opposite_edge_then_is_used_literally` の範囲外の場合・`declared_origin_resolution_is_independent_of_validrect`）を、本仕様の規則の期待値へ書き直す——ここで「退行させない」とは被覆を失わないことであり、正典の改訂に伴う期待値の更新は退行ではない（完了 spec `areka-P0-balloon-vertical-canon` 要件 7 の読みを引き継ぐ）。
5. The areka ワークスペース shall 要件 1.6 の検体 `emo2-kakukaku-offsetdpi` を、本番と同じ 2 層マージの経路で読んで開始点 sakura `(36, 46)`／kero `(24, 40)` を固定する決定論テストを 1 本持つ（検体は書き換えない。本修正前は `(0, 0)` に解決されるため、修正前に赤・修正後に緑になる）。
6. The areka ワークスペース shall 要件 3 の記録について、範囲外の成分 1 つにつき WARN 1 件（装着 1 回）・同値の再追従で追加 0 件・範囲内と未宣言で 0 件を数えるテストを持ち、0 件の主張が空振りしないよう同じ捕捉窓の中で対照の記録が数えられていることを併せて確かめる（先例＝`actor_region_warn_tests.rs`）。
7. The areka ワークスペース shall 本修正後も、1 ファイル 1,000 行の見張り（`file_length_guard_test.rs`）・純粋層の字面検査（`region.rs` が `windows` 系 crate に依存しないこと）・新設ファイルの走査対象登録の検査を含む既存テストを 1 件も赤くしない。

### Requirement 6: 文書の追随

**Objective:** As a areka の互換方針を読む人, I want 撤去を取り下げた事実と理由が正典沈黙の対応表に残ること, so that 同じ仮定を再び置かない

#### Acceptance Criteria

1. The areka ワークスペース shall `doc/COMPAT_ARCHITECTURE.md` §8 の「宣言された `origin` の validrect 外クランプの撤去」の行を、**撤去を取り下げた**行へ改める——areka の判断欄には本仕様の規則（範囲外の成分は宣言なしとして書字開始角へ・両端は範囲内・成分ごとに独立・WARN 1 件）を、根拠欄には取り下げの理由（撤去の前提「SSP でも壊れた定義」が 2026-09-18 の実機目視で反証されたこと・ukadoc の「描画してよい範囲」と「通常は指定せず validrect の定義に任せる」の 2 文）と日付を、出典欄には本仕様と完了 spec `areka-P0-balloon-vertical-canon` 要件 3.10 の上書きを記す。
2. The areka ワークスペース shall 同表の `\_l` の縦書き座標系の行（疑義 SC15 の主登記）にある「クランプ撤去により……常に一致し、二択は areka 内では発生しない」の理由の文を、撤去を前提にしない形へ言い直す——結論（解決後の文字描画開始点は 1 つであり、`\_l[0,0]` はそれを指すので二択は発生しない）は変えない。
3. The areka ワークスペース shall 同表の `\_l[x,y]` の上書き行にある「未宣言成分だけが書字開始角へ落ちる」の記述を、「未宣言または範囲外の成分が書字開始角へ落ちる」へ改める。
4. The areka ワークスペース shall `region.rs` 冒頭の「描画開始点は宣言どおり」「撤去された規約」の説明を本仕様の規則へ書き直し、撤去→取り下げの経緯（2026-08-27 撤去・2026-09-18 取り下げ・理由）と参照先（本仕様）を残す。
5. The areka ワークスペース shall 完了 spec `areka-P0-balloon-vertical-canon`・`areka-P0-emo-text-layer` のアーカイブ本体を改変しない（上書きの事実は 1.〜4. の記録で足りる）。
6. The areka ワークスペース shall 文書に書く file:line・関数名・テスト名の主張を、書く前に実ファイルで裏取りする（本仕様の brief の file:line は起票時の実測値であり、着手時に引き直す）。あわせて、本仕様が書き換える §8 の行が既に抱えている陳腐化した行番号の引用は、「何の定義行か」で指す形へ改める。

### Requirement 7: 上流・下流への申し送り

**Objective:** As a α 版の計画を持つ開発者, I want 本仕様が他の spec へ持ち込む前提と持ち込まない前提が明文であること, so that 並走する spec と合流したときに取り落としが出ない

#### Acceptance Criteria

1. The areka ワークスペース shall 本仕様の完了時に、`.kiro/steering/roadmap.md` の台帳行（#38）を完了へ更新し、上書きした完了 spec の要件（`areka-P0-balloon-vertical-canon` 3.10）を明記する。
2. The areka ワークスペース shall プロパティ `currentghost.balloon.scope(ID).basepos.x`／`.y` について「値は解決後の文字描画開始点から導けば足りる」とだけ `areka-P0-currentghost-property-tree` へ申し送り、本仕様では実装しない。
3. If 本仕様の新設テストが検体 `emo2-kakukaku-offsetdpi` のパスを直接書くとき, then the areka ワークスペース shall 並走中の `areka-P0-nar-install` が寄せる共有ヘルパの対象としてその 1 ファイルを申し送る（38 ファイルの実測に無い 39 本目になる）。

# Requirements Document

> **改訂 2026-09-17**——`main` の取り込み（`areka-P0-text-decoration-canon` PR#148・`areka-P0-ukadoc-coverage-roadmap` PR#147）で前提が 6 点変わったため、要件を実測に合わせて改訂した。変更点は各所に「（09-17 改訂）」で示す。改訂前の本文は git 履歴（`a6804540` 以前）にある。

## Introduction

### 誰が困っているか

バルーンの `descript.txt` に `font.bold,1` や `font.shadowcolor.r,64` と書いた作者。SSP では効くこれらの宣言が、areka では 1 つも効かない。効かないだけでなく、ログが 1 行も出ないので、作者からは「書いたのに何も起きない・理由も分からない」という形に見える。

読み手の側も困っている。文字装飾を実装した下流の仕様のうち書体まわり（`areka-P0-text-decoration-canon`）は 2026-09-13 に着地し、`\f[...]` タグが指定されなかったときに使う「バルーン既定の装い」と「無効表示の装い」をバルーン定義から受け取る口（`crates/areka-emo-text/src/look.rs` の `LookLayers::from_balloon` の `font_overrides`／`disable_overrides`）を用意して待っている。しかし、その口へ値を運ぶ側——バルーン定義を読んで渡す配線——がまだ無い。影まわり（`areka-P0-text-align-shadow-canon`）は brief のみで未着地であり、こちらは口そのものが無い。

### いま何が起きているか（本ブランチで実測・2026-09-17）

- 正典 `descript_balloon` ページの接頭辞なし `font.*` は **14 見出し**である。網羅調査のカタログ `doc/ukadoc-coverage/catalog.toml` に 14 行あり、同じ 14 項目が網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` にも 1 項目 1 塊で載っている。
- そのうち転記層（`crates/areka-parsers/src/balloon/parse.rs` の写像関数）が引くのは **5 本**（`font.color.r`・`font.color.g`・`font.color.b`・`font.name`・`font.height`）だけである。
- 残る **9 本**（`font.bold`・`font.italic`・`font.outline`・`font.strike`・`font.underline`・`font.shadowcolor.r`・`font.shadowcolor.g`・`font.shadowcolor.b`・`font.shadowstyle`）は、完全一致で引くキーの並びに無い。上流の KV 化層は未知キーも保持するが、転記層に写す先が無いため値はそこで落ちる。台帳の 9 項目はいずれも「壊れ方: 黙って壊れる。記録: なし」と登記されている。
- **（09-17 改訂）無効表示の書体設定 `disable.font.*`**（正典の見出しは 1 つ `disable.font.(フォント定義),(指定)`＝「`font.` で始まる各定義をそのまま定義できる。`disable.font.color` のみバルーンの画像色とミックスした色、ほかは `font.` 定義群と同じ」・SSP 2.5.51）も転記層は読まない。台帳の当該 1 項目は `vocabulary-only`・担当 `areka-P0-text-decoration-canon`（完了済み）で、備考が「キーの読み取り（と、それに伴う状態の実装済みへの進め方）は `areka-P0-balloon-font-descript-keys`」と本仕様を名指ししている。完了済み spec の最終検証も「引受先は `areka-P0-balloon-font-descript-keys`」と登記した。
- **（09-17 改訂）下流の受け口は在る。** `look.rs` の `LookLayers::from_balloon` は `font_overrides`／`disable_overrides` の 2 引数で「`\f` と同じ形のトークン列」（`font.bold,1` → `["bold","1"]`）を受け、台本の `\f` と同じ `apply_font_tag` を通して既定層・無効表示層へ載せる。受ける語彙は `bold`／`italic`／`underline`／`strike`／`outline`（真偽 6 語）・`name`（候補列）・`height`・`color`（3 成分または 1 語）。`outline` は状態だけ更新して表示は変えない（同 spec の要件 5.9・語彙のみ）。`shadowcolor`／`shadowstyle` は同 spec の所有外で、見た目を変えずに済ませる。本番の呼び出し（`crates/areka-emo-text/src/draw.rs` の `ResolvedFont::resolve_with_background`）は両引数に **空の列**を渡している＝配線が無い。
- **（09-17 改訂）受け口は綴り誤りを黙って飛ばす。** `apply_overrides` の doc は「記録はバルーン定義を読む側（`areka-P0-balloon-font-descript-keys`）が行を特定できる位置で出すのが正しく、ここでは行番号もファイル名も持たない」と、記録の責務を本仕様へ申し送っている。
- **（09-17 改訂）台帳の現状**: `implemented` 4（`font.color.r`／`.g`／`.b`・`font.height`・担当 `areka-P0-text-decoration-canon`）、`absent` 10（残り全部・担当は完了済み spec のアーカイブ時に**本仕様へ移送済み**）、`disable.font.*` 1（`vocabulary-only`）。優先度は 15 項目とも `A5`（`ukadoc-coverage-roadmap` の全体の並べ直しで `A11`／`E66`／`A37` から付け直された。備考の「束の順位: 11」等の文はその前の値のまま残っている）。`font.name` の `absent` は実態と合わない（Requirement 7.7）。
- **（09-17 改訂）`font.name` の縮退は 2 点から 1 点へ減った。** カンマ区切りの候補列は `ResolvedFont::resolve_with_background` が記述順のまま候補列として `TextLook::name` へ渡すようになった。残るのは「バルーンのフォルダに置いたフォントファイルを指定できない」（`draw_catalog.rs` がファイル名らしい候補を検出して警告し読み飛ばす）の 1 点である。
- **（09-17 改訂）数の食い違い**: 「基底 13 キー」「残り 8 キー」と書く生きた文書は、本ブランチの全数検索（`grep -c`）で ⑴ 本仕様の brief（「13 キー」4 か所＋完了済み spec が 09-13 に書き足した相互登記の「残り 8 キー」1 か所）、⑵ `areka-P0-text-align-shadow-canon` の brief（「13 キー」1 か所）、⑶ `.kiro/steering/roadmap.md`（「残り 8 キー」1 か所）の **3 文書 7 か所**である。分割元 `areka-P0-text-decoration-canon` の brief（「13 キー」7 か所・「残り 8 キー」1 か所）は `completed/` へ移ったので非改変の側になった。数え落としていたのは `font.outline` である。台帳は `implemented` 4 項目の備考に「担当 spec の brief はバルーンの font 系を 13 キーと書くが、正典の見出しは 14 種ある（担当 spec の記述が古い）」を残している（残り 10 項目からは完了済み spec の移送時に消えている）。
- **（09-17 改訂）全体報告 `report/summary.md` の作り直し**は、統合担当（`areka-P0-ukadoc-coverage-roadmap`）が完了したため、`doc/ukadoc-coverage/README.md` の定めどおり「台帳を触った人が `report-summary` を走らせる」に変わった。

### 何を変えるか

未写像の 9 本と `disable.font.*` の 14 本を転記層の写像対象へ加え、宣言された値と未指定とを潰さずに保つ。転記層なので値の意味づけ（正典既定値の適用・語彙の判定・語彙外の値に対する警告付きの縮退・実際の描画）は一切行わず、そのまま下流へ渡す。**（09-17 改訂）下流の書体まわりが先に着地しているので、本仕様が最後に配線する**——読んだ値を `\f` と同じ形のトークン列にして `LookLayers::from_balloon` の 2 引数へ渡し、受け口が飛ばした綴り誤りは配線側で記録する。影の 4 キーは受け口が無いので配線せず、取り出し口を影まわりの仕様へ申し送る。あわせて 14＋1 の定義箇所に正典 URL のコメントを 1 行ずつ置き、網羅台帳の状態・備考・担当と、そこから機械で作り直すドメイン別報告・全体報告を実測に合わせ、上の「13 キー」を是正する。

## Boundary Context

- **In scope**（作者・下流の実装者から見える範囲）:
  - 接頭辞なし `font.*` 基底 14 キーの全数が、バルーン定義の解析結果から取り出せること。
  - **（09-17 改訂）** `disable.font.*` 14 キー（基底 14 キーと同じ見出しに `disable.` を前置したもの）が、基底とは別の層として解析結果から取り出せること。
  - 値の形（0/1・数値・`none`・形態の語）ごとの保ち方と、未指定・宣言された値・語彙外の値が互いに区別できること。
  - 2 層（バルーン定義の既定層と画像別の上書き層）のどちらに書かれても、既存キーと同じ優先順位で効くこと。
  - **（09-17 改訂）** 書体の飾り 5 本（基底・無効表示の両層）と、無効表示層の書体名・大きさ・色を、`look.rs` の受け口へ配線すること。受け口が飛ばした綴り誤りを配線側で記録すること。
  - 14＋1 本の定義箇所に置く正典 URL のコメント 1 行ずつ。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の当該 15 項目（状態・備考・担当）と、ドメイン別報告 `doc/ukadoc-coverage/report/assets.md`・全体報告 `report/summary.md` の作り直し。
  - 網羅調査のブリーフィング `doc/ukadoc-coverage/briefing-assets.md` の是正候補の段 1 つ（件数と引き取り先が是正で古くなるため）。
  - 「基底 13 キー」「残り 8 キー」と書いた生きた 3 文書（本仕様 brief・`areka-P0-text-align-shadow-canon` の brief・ロードマップ）の是正。
  - 決定論テスト（値の形ごと・2 層の優先順位・未指定との区別・接頭辞付きキーの巻き込み防止・配線の端から端まで）。
- **Out of scope**:
  - 接頭辞付きの `font` 族のうち `disable.` 以外の 8 系統（`anchor.`／`anchor.notselect.`／`anchor.visited.`／`cursor.`／`cursor.notselect.`／`communicatebox.`／`number.`／`sstpmessage.`）。機能ごと M1 非実装であり、各機能の仕様が解禁するときに扱う。ただし本仕様のキーへ値が漏れないことは本仕様が守る（下の Requirement 4）。
  - `\f[...]` タグ側の意味論・リセット規則・実際の描画（`areka-P0-text-decoration-canon`＝着地済み／`areka-P0-text-align-shadow-canon`）。`font.outline` の描画（受け口が語彙のみと定めており、本仕様は値を渡すところまで）。
  - **（09-17 改訂）** 影の 4 キー（`font.shadowcolor.r`／`.g`／`.b`・`font.shadowstyle`。基底・無効表示とも）の配線。受け口が無い（`look.rs` は `shadowcolor`／`shadowstyle` を所有外として見た目を変えない）ため、影まわりの仕様が着地するときに同仕様が配線する。本仕様は取り出し口を申し送る（Requirement 8.3）。
  - `font.name` の残る 1 点の食い違い（バルーンのフォルダに置いたフォントファイルを指定できない）の是正。描画側の機能追加であり、SSP 追従は要望が出た時点で判断するという方針（2026-09-11 開発者裁定）に従い、引受先の spec を起票しない。台帳には「引受先なし・要望が出た時点で起票」と正直に書く。
  - 正典の項目が増減したことに自動で気付く仕組み。areka は SSP の複製ではなく別のアプリであり、追従は実際の要望が出た時点で判断する（2026-09-11 開発者裁定・議題 2・Requirement 9.8）。
  - バルーン名 `name,` の写像（`areka-P0-emo-text-canon-residue` 項目 14）。同じ写像関数を触るため同居させない。本仕様を先に着地させる。
  - `look.rs`・`apply_font_tag` の改変。受け口の形は完了済み spec が定めたもので、本仕様は呼び手として使うだけ。
- **Adjacent expectations**:
  - **前提**: `areka-P0-text-decoration-canon` 着地済み（PR#148・2026-09-17 に `main` 取り込み済み）。
  - **読み手（下流）**: `areka-P0-text-align-shadow-canon`（影 4 キーの既定層・無効表示層として読む。取り出し口は本仕様が用意し、配線は同仕様）・`areka-P0-emo-text-canon-residue`（同じ写像関数へ後着）。
  - **隣接**: `areka-P0-anchor-tag-canon` は `anchor.font.*` を同じ写像関数で引くため、後着側が突合する。

## Requirements

### Requirement 1: 対象キー集合を正典の実測で確定する

**Objective:** 下流の実装者として、「バルーン定義の基底の書体設定」と言ったときに何が入るのかを 1 か所で確定しておきたい。そうすれば、読む側が数を取り違えて一部のキーを配線し忘れることがない。

#### Acceptance Criteria

1. The 本仕様 shall 対象キー集合を、網羅調査のカタログに載る接頭辞なし `font.*` の見出し全数（**14**）と定め、その一覧（`font.bold`・`font.color.b`・`font.color.g`・`font.color.r`・`font.height`・`font.italic`・`font.name`・`font.outline`・`font.shadowcolor.b`・`font.shadowcolor.g`・`font.shadowcolor.r`・`font.shadowstyle`・`font.strike`・`font.underline`）を本文へ書き出す。
2. The 本仕様 shall 14 のうち既に写像済みの 5 本（`font.color.r`／`.g`／`.b`・`font.name`・`font.height`）と、未写像の 9 本（残り）を明示して数え、零（＝対象外 0 本）も明示的に書く。
3. **（09-17 改訂）** When 生きている文書が「基底 13 キー」「残り 8 キー」と書いている, the 本仕様 shall その **3 文書 7 か所**を「14」「残り 9」へ是正し、数え落としていたのが `font.outline` であることを添える。対象は ⑴ 本仕様の brief（「13 キー」4 か所・「残り 8 キー」1 か所）、⑵ `areka-P0-text-align-shadow-canon` の brief（Scope の Out の行の「13 キー」1 か所）、⑶ `.kiro/steering/roadmap.md`（W13 干渉台帳の行の「残り 8 キー」1 か所）である。履歴と完了済みの記録（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/` 配下。分割元 `areka-P0-text-decoration-canon` の brief を含む）は、非改変の方針と着地時点の記録という理由でいずれも対象外とする。
4. If 実装の途中で対象キー集合がカタログと食い違うことが分かった, then the 本仕様 shall 見た目で数え直さずカタログの行を照合元とし、食い違いの内容を記録する。
5. **（09-17 改訂）** When 上の 3 文書を是正した, the 本仕様 shall 網羅調査のブリーフィング `doc/ukadoc-coverage/briefing-assets.md` の是正候補の段（書体の欄の数が 1 つ足りない、という記録）も同じコミットで書き換える。当該の段は「説明書は 6 か所で 13 と書いている」「引き取るのは `areka-P0-text-decoration-canon`」と書くが、是正後は生きた説明書で 13 と書く箇所が 0 になり、引き取ったのは本仕様である。この文書は完了した記録ではなく**これから手を着ける人が読む案内**であり、古いままだと読んだ人が存在しない宿題を探しに行くため、案内としての正しさを優先する（2026-09-11 開発者裁定・議題 3）。件数は是正後に数え直した実測を書き、引き算で導かない。完了済み spec の brief に残る「13 キー」は着地時点の記録として残る旨を添える。

### Requirement 2: 未写像の 9 キーと `disable.font.*` を解析結果へ写す

**Objective:** バルーン作者として、`descript.txt` に書いた書体・影の宣言が、少なくとも areka の内部で読み取られている状態にしたい。そうすれば、描画側が受け取り次第、バルーンを書き直さずに宣言が効くようになる。

#### Acceptance Criteria

1. When バルーン定義に `font.bold`・`font.italic`・`font.outline`・`font.strike`・`font.underline` のいずれかが書かれている, the バルーン定義の転記層 shall その宣言を解析結果から取り出せる形で保つ。
2. When バルーン定義に `font.shadowcolor.r`・`font.shadowcolor.g`・`font.shadowcolor.b` のいずれかが書かれている, the バルーン定義の転記層 shall 3 つの成分を**個別に**保ち、1 つだけ書かれている場合に他の 2 つが未指定であることを区別できるようにする。
3. When バルーン定義に `font.shadowstyle` が書かれている, the バルーン定義の転記層 shall その宣言を解析結果から取り出せる形で保つ。
4. The バルーン定義の転記層 shall 上の 9 キーそれぞれについて、「未指定」「宣言された値」の 2 つを互いに区別できる形で示す。
5. If 宣言された値が正典の語彙に無いもの（たとえば `font.bold,2`・`font.shadowstyle,blur`・`font.shadowcolor.r,300`）である, then the バルーン定義の転記層 shall その値を落とさずそのまま保ち、警告も縮退も行わない（判定は下流の責務）。
6. If `font.shadowcolor.r`／`.g`／`.b` に正典が定める `none`（影を無効化する語）が書かれている, then the バルーン定義の転記層 shall `none` と「未指定」と「数値」の 3 つを互いに区別できる形で保つ。
7. The バルーン定義の転記層 shall 9 キーのいずれについても、値の解釈・描画・エラー通知を行わない（解析結果を返すだけで、失敗し得ない）。
8. **（09-17 追加）** When バルーン定義に `disable.font.<基底キー>`（基底 14 キーのいずれかに `disable.` を前置したもの）が書かれている, the バルーン定義の転記層 shall その 14 本を**基底とは別の層**として保ち、基底の同名キーとは互いに混ざらないようにする。値の保ち方は基底と同じ規則に従う——書体名・大きさ・色 3 成分は既存の写像済み 5 本と同じ形、飾り 5 本と影 4 本は上の 2.1〜2.6 と同じ生文字列。
9. **（09-17 追加）** While `disable.font.*` が 1 本も書かれていない, the バルーン定義の転記層 shall 無効表示層を全キー「未指定」で返し、基底層の値を写し込まない（無効表示層が既定層を継ぐのは下流の受け口の責務であり、転記層は複製しない）。

### Requirement 3: 正典の既定値は適用せず、下流へ申し送る

**Objective:** 下流の実装者として、既定値がどの層で入るのかを一意に決めておきたい。そうすれば、転記層と描画層の両方が既定値を入れて二重に効いたり、どちらも入れずに抜けたりしない。

#### Acceptance Criteria

1. While バルーン定義にキーが書かれていない, the バルーン定義の転記層 shall 正典の既定値を代入せず「未指定」のまま返す。
2. The 本仕様 shall 14 キーの正典既定値を本文の表に登記し、下流が適用する値として申し送る。登記する値は `font.name`＝`ＭＳ ゴシック`、`font.height`＝`12`、`font.color.r`／`.g`／`.b`＝`0`、`font.bold`／`font.italic`／`font.outline`／`font.strike`／`font.underline`＝`0`、`font.shadowcolor.r`／`.g`／`.b`＝`none`、`font.shadowstyle`＝`offset` とする。
3. The 本仕様 shall `font.shadowstyle` の語彙が `offset`（右下にずれた表示）と `outline`（縁取り）の 2 語であること、および同キーが SSP 2.5.27 で登場したことを併せて登記する。
4. **（09-17 改訂）** The 本仕様 shall 既定値の適用先を書き分けて申し送る——書体 10（`font.name`・`font.height`・`font.color.*`・飾り 5）は `look.rs` の `TextLook::ukadoc_default` が既に適用しており（`areka-P0-text-decoration-canon` 着地済み）、本仕様の配線は「宣言されたキーだけを上書きとして渡す」ので二重に入らない。影 4 は `areka-P0-text-align-shadow-canon` が適用する。

### Requirement 4: 既存の引き方の不変条件を保つ

**Objective:** バルーン作者として、選択肢マーカーやアンカーのために書いた `cursor.font.*`・`anchor.font.*` が、本体の書体設定へ化けないことを保証してほしい。そうでないと、選択肢の色を変えただけで本文の色が変わるような、原因の分からない崩れ方をする。

#### Acceptance Criteria

1. When バルーン定義の既定層と画像別の上書き層の両方に同じ `font.*` キーが書かれている, the バルーン定義の転記層 shall 画像別の上書き層の値を採る。
2. When 画像別の上書き層に当該キーが無い, the バルーン定義の転記層 shall バルーン定義の既定層の値を引き継ぐ。
3. When 画像別の上書き層そのものが無い, the バルーン定義の転記層 shall バルーン定義の既定層だけを写す。
4. **（09-17 改訂）** When 接頭辞付きのキー（`anchor.font.shadowcolor.r`・`anchor.font.shadowstyle`・`anchor.notselect.font.shadowcolor.r`・`anchor.visited.font.shadowstyle`・`cursor.font.shadowcolor.g`・`cursor.font.shadowstyle`・`cursor.notselect.font.shadowcolor.b`・`number.font.height`・`sstpmessage.font.name`・`communicatebox.font.color.r`・`disable.font.bold` 等）が書かれている, the バルーン定義の転記層 shall それらの値を接頭辞なしの基底キーへ一切反映しない。`disable.font.*` は無効表示層へだけ写り、逆に基底の `font.*` は無効表示層へ写らない。
5. The バルーン定義の転記層 shall 本仕様が写像しないキー（選択肢・リンク・スクロール系など）を、従来どおり無視する。
6. **（09-17 追加）** The 無効表示層の 14 本 shall 上の 4.1〜4.3 の 2 層の優先順位に、基底と同じ規則で従う。

### Requirement 5: 宣言していないバルーンの解析結果と見た目を 1 つも変えない

**Objective:** 運用者として、この変更で **9 キーも `disable.font.*` も書いていない**バルーンの見た目が変わらないことを保証してほしい。宣言を効かせる仕様であって、既定の装いを変える仕様ではないからである。

#### Acceptance Criteria

1. The バルーン定義の転記層 shall 既に写像済みの 5 キー（`font.color.r`／`.g`／`.b`・`font.name`・`font.height`）の解析結果を変えない。
2. The バルーン定義の転記層 shall 幾何・書字方向・選択肢マーカー・窓の配置など、`font.*` 以外の解析結果を変えない。
3. The 本仕様 shall 既存の決定論テストの期待値を緩めない。既存のテストが赤になった場合は期待値を書き換えず、原因を是正する。
4. **（09-17 改訂）** When 本仕様の変更を取り込んだ, the areka shall 9 キーと `disable.font.*` を 1 本も書いていないバルーンについて、描画される文字の見た目を変えない（配線は宣言されたキーだけを上書きとして渡すので、宣言が無ければ受け口へ渡す列は空のまま＝従来と同じ呼び出し）。リポジトリ内のバルーン定義（`crates/areka-emo-text/examples/fixtures/emo2-vertical*/descript.txt`・`crates/pilot/examples/shiori-host-32/fixtures/emo2*/descript.txt`）に 9 キー・`disable.font.*` の宣言は 0 件なので、既存の描画比較テストの期待値は変わらない。
5. **（09-17 追加）** When バルーン定義に `font.bold,1`（または `italic`／`strike`／`underline` の `1`）が書かれている, the areka shall その装いを既定の見た目として描く（これが本仕様の目的であり、5.4 の「変えない」の対象外である）。

### Requirement 6: 定義箇所に正典 URL の証拠を置く

**Objective:** 後から読む人として、どの行がどの正典項目に対応しているのかを、実装を追い直さずに 1 行で確かめたい。

#### Acceptance Criteria

1. **（09-17 改訂）** The 本仕様 shall 基底 14 キーそれぞれの定義箇所に正典 URL 1 行のコメントを 1 本ずつ、`disable.font.*` の 14 本を引く塊に正典 URL 1 行（見出しはカタログに 1 つ）を置く。既に置かれているのは **4 本**（`font.color.r`・`font.color.g`・`font.color.b`・`font.height`）であり、**新たに置くのは 11 本**（未写像の 9 本＋`font.name`＋`disable.font.*`）である。`font.name` の行には説明のコメントは在るが正典 URL の行が無い。
2. The 本仕様 shall 置く URL をカタログの当該行から写し、見た目で打ち直さない。
3. The 本仕様 shall コメントを行の先頭（字下げを除く）がコメント記号で始まる形とし、`ukadoc:` の後に URL 1 語だけを置いて説明文を続けない（続けると機械が証拠として拾わない）。
4. The 本仕様 shall 正典 URL を定義箇所だけに置き、呼び出し側（配線を含む）には置かない。
5. When 網羅調査の常時検査を走らせた, the 検査 shall 本仕様が置いた URL について「カタログに無い URL」を 1 件も報告しない。

### Requirement 7: 網羅台帳とドメイン別報告を実測に合わせる

**Objective:** 網羅状況を読む人として、台帳が「areka は今どこまでできているか」を正しく映していてほしい。台帳が古いままだと、次に手を着ける項目の順番付けが狂う。

#### Acceptance Criteria

1. **（09-17 改訂）** When 9 キーの写像と配線が着地した, the 本仕様 shall 当該項目の状態を実測で改める——`font.bold`・`font.italic`・`font.strike`・`font.underline` の 4 項目は `absent` から `implemented`（読んで受け口へ渡し、受け口が既定の見た目へ載せる）へ、`font.outline` は `absent` から `vocabulary-only`（読んで渡すが受け口が状態だけ更新して表示は変えない）へ、影の 4 項目（`font.shadowcolor.r`／`.g`／`.b`・`font.shadowstyle`）は `absent` から `vocabulary-only`（読むが受け口が無い）へ。
2. **（09-17 改訂）** The 本仕様 shall 状態を改めた項目の備考から「読むキーを並べた表を完全一致で引く形で、この項目はその表に無い」「描画側にも受け口が無い」という、実測と合わなくなる記述を除き、代わりに実態（転記層のどの取り出し口を通り、配線がどの受け口へ渡し、受け口がどう扱うか）と、記録の有無（正典どおりの宣言では記録が出ないこと・語彙外の値では配線が警告を 1 度出すこと）を書く。
3. The 本仕様 shall 14 項目すべての備考から「担当 spec の brief はバルーンの font 系を 13 キーと書くが、正典の見出しは 14 種ある（担当 spec の記述が古い）」の一文を除く（Requirement 1.3 で是正済みになるため。09-17 の実測では `implemented` 4 項目に残っている）。
4. **（09-17 改訂）** The 本仕様 shall 影の 4 項目の担当を `areka-P0-text-align-shadow-canon` へ改める（読み取りは本仕様が済ませ、配線を同仕様が行う）。飾り 5 本と `font.name` の担当は、状態を着地させた spec として本仕様のままとする（台帳では `implemented` 28 項目が完了済み spec を担当に持っており、「担当＝その状態を着地させた spec」が慣行である）。
5. **（09-17 改訂）** The 本仕様 shall 優先度（15 項目とも `A5`）を据え置き、束の名前だけを実態に合わせる——`implemented` になる項目は既設の束名「台詞の書体・正典どおりに動く」へ、`vocabulary-only` の項目は既設の束名「台詞の書体・名前だけ受けて使わない」（`disable.font.*` の項目が現に使っている名前）へ、`degraded` の項目は「台詞の書体・読めるが正典どおりに描かれない」へ揃える。同じ状態の項目に別の束名を残さない。備考の「束の順位: N」の文は前回の並べ直し以前の値であり、本仕様は触らない（優先度欄が正本）。
6. The 本仕様 shall `font.color.r`／`.g`／`.b`・`font.height` の 4 項目を `implemented` のまま据え置く。
7. **（09-17 改訂）** When `font.name` の状態を見直した, the 本仕様 shall これを `absent` から `degraded`（縮退＝受け取るが正典どおりではない）へ改める。実測では転記層が完全一致で引いて文字列のまま持ち上げ、描画側がカンマ区切りの候補列を記述順のまま書体の候補として使う。効かないのは 1 点（バルーンのフォルダに置いたフォントファイルを指定できない。`draw_catalog.rs` が警告して読み飛ばす）だけであり、これは「何もしていない」ではなく「受け取るが正典どおりでない」に当たる。本仕様はその 1 点の食い違い自体は引き受けず、備考に「引受先なし・要望が出た時点で起票」と書く（2026-09-11 開発者裁定・議題 1 で `degraded` へ改めることを決め、09-17 に縮退の内訳を 2 点から 1 点へ実測で更新）。
8. **（09-17 改訂）** When 台帳を書き換えた, the 本仕様 shall ドメイン別報告 `doc/ukadoc-coverage/report/assets.md` を `cargo run -p ukadoc-survey -- report` で、全体報告 `doc/ukadoc-coverage/report/summary.md` を `cargo run -p ukadoc-survey -- report-summary` で作り直し、台帳と同じコミットに入れる（統合担当は完了済みで、README が「台帳を触った人が作り直す」と定める）。作り直しは改行だけを書き出すので、保存形（`git ls-files --eol` が `i/lf`）と一致していることを確かめ、他ドメインの報告に差分が出ないことを確かめる。
9. When 網羅調査の常時検査とテストを走らせた, the 検査 shall 台帳・報告について 1 件も食い違いを報告しない。
10. **（09-17 追加）** When `disable.font.*` の読み取りと配線が着地した, the 本仕様 shall 台帳の当該 1 項目を `vocabulary-only` から `degraded` へ改め（書体名・大きさ・色・飾り 4 本は効く。`outline` は語彙のみ・影 4 本は受け口が無い）、担当を完了済みの `areka-P0-text-decoration-canon` から本仕様へ改め、備考を実態に合わせる。

### Requirement 8: 下流への配線と引き渡し

**Objective:** バルーン作者として、書体まわりの受け口は既に在るのだから、`font.bold,1` と書けば今すぐ効いてほしい。影まわりの実装者として、既定層を読む口がどこに在るかを本仕様の文書だけで分かるようにしておきたい。

#### Acceptance Criteria

1. **（09-17 改訂）** The 本仕様 shall 基底の飾り 5 本（`font.bold`・`font.italic`・`font.outline`・`font.strike`・`font.underline`）のうち宣言されたものを、`\f` と同じ形のトークン列（`font.bold,1` → `["bold","1"]`）にして `LookLayers::from_balloon` の `font_overrides` へ渡す。宣言されていないキーは列に入れない（受け口の既定に任せる）。
2. **（09-17 改訂）** The 本仕様 shall `disable.font.*` のうち宣言されたもの——飾り 5 本は 8.1 と同じ形、`disable.font.name` は候補列（`["name", 候補…]`）、`disable.font.height` は `["height", 値]`、`disable.font.color.r`／`.g`／`.b` は 3 成分が揃ったときだけ `["color", r, g, b]`——を `disable_overrides` へ渡す。
3. **（09-17 改訂）** The 本仕様 shall 影の 4 キー（基底・無効表示とも）を受け口へ渡さず、取り出し口（型と読み口の名前）を `areka-P0-text-align-shadow-canon` へ申し送る。同仕様が受け口を開けるときに配線する。
4. The 本仕様 shall 引き渡す内容（14 キーの一覧・値の形・未指定の表し方・正典既定値・どの下流がどのキーを受け持つか・配線済みか否か）を、下流が読める形で本仕様の文書に残す。
5. **（09-17 追加）** If 受け口がトークンを飛ばした（語彙外の値 `font.bold,yes`・成分が揃わない `disable.font.color.r` だけの宣言など）, then the 配線 shall その事実を `warn!` で記録する（キー・値・理由）。転記層はログを出さない契約なので、記録は配線側が出す（受け口の doc が申し送る「バルーン定義を読む側」は配線である）。記録は同じバルーン定義の解決 1 回につき同じ（キー, 値）を 1 度までとし、正典どおりの宣言では 1 行も出さない。
6. **（09-17 追加）** The 配線 shall 既存の `ResolvedFont::resolve_with_background` の他の解決結果（書体名・大きさ・色・選択肢文字色・背景色の扱い）を変えない。

### Requirement 9: 決定論テストで固定する

**Objective:** 開発者として、この写像と配線が後の変更で静かに壊れないようにしたい。転記層は失敗を報告しない作りなので、壊れても誰も気付かないからである。

#### Acceptance Criteria

1. The 本仕様 shall 実機にもネットワークにも触れない決定論テストで、9 キーそれぞれについて「宣言した値が取り出せる」ことを固定する。
2. The 本仕様 shall 「未指定のときに未指定として表れる」ことを、宣言された `0` と取り違えない形で固定する。
3. The 本仕様 shall 正典の語彙に無い値（`font.bold,2`・`font.shadowstyle,blur` 等）が落とされず素通しされることを固定する。
4. The 本仕様 shall `font.shadowcolor.r`／`.g`／`.b` について、`none`・数値・未指定の 3 つが互いに区別されることと、3 成分の部分欠落が個別に表れることを固定する。
5. The 本仕様 shall 2 層の優先順位（画像別が勝つ・画像別に無ければ既定層を引き継ぐ・画像別の層そのものが無い）を、新しい 9 キーと `disable.font.*` について固定する。
6. The 本仕様 shall 接頭辞付きのキーが基底キーへ漏れないことを、影のキー（`anchor.font.shadowcolor.r`・`cursor.font.shadowstyle` 等）と `disable.font.*` を含めて固定し、既存の同種のテスト行を壊さない。
7. The 本仕様 shall 対象キー集合が 14 であることを、転記層のテストの中で写像対象のキー名を並べた表を持ち、その要素数が 14 であることを判定する形で固定する（表示するだけの数にせず、食い違えば赤になる判定にする）。この検査は**実装の側だけを見る**——写像を消す・取りこぼすといった実装側の後退は赤になるが、**正典の側に 15 本目が増えても緑のままである**。同じ表を `disable.` 前置で読み戻す判定にも使い、無効表示層の 14 本も同じ表で固定する。
8. The 本仕様 shall 正典の項目が増えたことに自動で気付く仕組み（カタログの行数との突合など）を**設けない**。areka は SSP の複製ではなく別のアプリであり、正典への追従は実際の要望が出た時点で判断する（2026-09-11 開発者裁定・議題 2）。このため検査の置き場は転記層のテストに閉じ、編集集合は brief の約束（`balloon/{parse,model}.rs`＋兄弟テスト＋`ledger/assets.toml`。新しい型を公開面へ出す `balloon/mod.rs` の再輸出は同じモジュール内の付随であり、この約束の範囲に含める）に、**（09-17 改訂）** 後着の配線（`crates/areka-emo-text` の配線 1 モジュールとその兄弟テスト・`draw.rs` の呼び出し 1 か所・走査一覧への登録）を加えたものに保つ。
9. When テストを走らせた, the 本仕様 shall 対象クレート（`areka-parsers`・`areka-emo-text`）のテストと網羅調査の道具のテストの両方が緑であることを、出力を切り詰めない形で確かめる。
10. **（09-17 追加）** The 本仕様 shall 配線を端から端まで通す決定論テストで固定する——バルーン定義に `font.bold,1` を書いて `ResolvedFont::resolve` を通すと既定層の太字が立つこと、`disable.font.bold,1` では無効表示層だけが立ち既定層は立たないこと、`disable.font.color.r/g/b` を揃えると無効表示層の色が混色でなく宣言値になること、`font.outline,1` は状態だけ立ち表示に効かないこと。
11. **（09-17 追加）** The 本仕様 shall 語彙外の値（`font.bold,yes`）と成分が揃わない色（`disable.font.color.r` だけ）で警告が 1 度ずつ記録され、正典どおりの宣言では 0 行であることを、ログを捕捉する決定論テストで固定する。
12. **（09-17 追加）** The 本仕様 shall 配線の判定が本当に赤になることを較正する——転記層の写像 1 本を一時的に外すと転記層のテストが赤になること、配線が空の列を渡す旧実装へ戻すと端から端までのテストが赤になることを実測してから元に戻す。

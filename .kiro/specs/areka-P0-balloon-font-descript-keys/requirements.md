# Requirements Document

## Introduction

### 誰が困っているか

バルーンの `descript.txt` に `font.bold,1` や `font.shadowcolor.r,64` と書いた作者。SSP では効くこれらの宣言が、areka では 1 つも効かない。効かないだけでなく、ログが 1 行も出ないので、作者からは「書いたのに何も起きない・理由も分からない」という形に見える。

読み手の側も困っている。文字装飾を実装する下流の仕様（`areka-P0-text-decoration-canon`＝書体まわり／`areka-P0-text-align-shadow-canon`＝影まわり）は、`\f[...]` タグが指定されなかったときに使う「バルーン既定の装い」をバルーン定義から取りたいが、その値を受け取る口が無い。

### いま何が起きているか（本ブランチで実測・2026-09-11）

- 正典 `descript_balloon` ページの接頭辞なし `font.*` は **14 見出し**である。網羅調査のカタログ `doc/ukadoc-coverage/catalog.toml` に 14 行あり、同じ 14 項目が網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` にも 1 項目 1 塊で載っている。
- そのうち転記層（`crates/areka-parsers/src/balloon/parse.rs` の写像関数）が引くのは **5 本**（`font.color.r`・`font.color.g`・`font.color.b`・`font.name`・`font.height`）だけである。
- 残る **9 本**（`font.bold`・`font.italic`・`font.outline`・`font.strike`・`font.underline`・`font.shadowcolor.r`・`font.shadowcolor.g`・`font.shadowcolor.b`・`font.shadowstyle`）は、完全一致で引くキーの並びに無い。上流の KV 化層は未知キーも保持するが、転記層に写す先が無いため値はそこで落ちる。台帳の 9 項目はいずれも「壊れ方: 黙って壊れる。記録: なし」と登記されている。
- 台帳の状態は、`implemented` が 4（`font.color.r`／`.g`／`.b`・`font.height`）、`absent` が 10（残り全部。`font.name` は写像されてはいるが、バルーンのフォルダに置いたフォントファイルを指定できないこと・カンマ区切りの優先列が効かないことの 2 点で描画側が正典どおりでないため `absent`）。
- **数の食い違い**: 本仕様の brief・分割元 `areka-P0-text-decoration-canon` の brief・`.kiro/steering/roadmap.md` はいずれも「基底 13 キー」「残り 8 キー」と書くが、これは `font.outline` を数え落としている。台帳は 14 項目すべての備考に「担当 spec の brief はバルーンの font 系を 13 キーと書くが、正典の見出しは 14 種ある（担当 spec の記述が古い）」と登記済みである。本仕様の brief は「キー集合の照合元」を台帳と明記しているので、**本仕様は 14 を採り、13 と書いた生きた 4 文書を是正する**（本ブランチで全数検索した結果、`areka-P0-text-align-shadow-canon` の brief `:27` が 4 本目である。履歴と完了済みの記録は非改変）。

### 何を変えるか

未写像の 9 本を転記層の写像対象へ加え、宣言された値と未指定とを潰さずに保つ。転記層なので値の意味づけ（正典既定値の適用・語彙の判定・語彙外の値に対する警告付きの縮退・実際の描画）は一切行わず、そのまま下流へ渡す。あわせて 14 本の定義箇所に正典 URL のコメントを 1 行ずつ置き、網羅台帳の状態・備考・担当と、そこから機械で作り直すドメイン別報告を実測に合わせ、上の「13 キー」を是正する。

## Boundary Context

- **In scope**（作者・下流の実装者から見える範囲）:
  - 接頭辞なし `font.*` 基底 14 キーの全数が、バルーン定義の解析結果から取り出せること。
  - 値の形（0/1・数値・`none`・形態の語）ごとの保ち方と、未指定・宣言された値・語彙外の値が互いに区別できること。
  - 2 層（バルーン定義の既定層と画像別の上書き層）のどちらに書かれても、既存キーと同じ優先順位で効くこと。
  - 14 本の定義箇所に置く正典 URL のコメント 1 行ずつ。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の当該 14 項目（状態・備考・担当）と、ドメイン別報告 `doc/ukadoc-coverage/report/assets.md` の作り直し。
  - 「基底 13 キー」「残り 8 キー」と書いた生きた 4 文書（本仕様 brief・分割元 brief・`areka-P0-text-align-shadow-canon` の brief・ロードマップ）の是正。
  - 決定論テスト（値の形ごと・2 層の優先順位・未指定との区別・接頭辞付きキーの巻き込み防止）。
- **Out of scope**:
  - 接頭辞付きの `font` 族 9 系統（`anchor.`／`anchor.notselect.`／`anchor.visited.`／`cursor.`／`cursor.notselect.`／`communicatebox.`／`number.`／`sstpmessage.`／`disable.`）。機能ごと M1 非実装であり、各機能の仕様が解禁するときに扱う。ただし本仕様のキーへ値が漏れないことは本仕様が守る（下の Requirement 4）。
  - `\f[...]` タグ側の意味論・リセット規則・実際の描画（`areka-P0-text-decoration-canon`／`areka-P0-text-align-shadow-canon`）。
  - `disable.font.*` の実体化（`areka-P0-text-decoration-canon`・描画側）。
  - `font.name` の既知の 2 つの食い違い（バルーンのフォルダに置いたフォントファイルを指定できない・カンマ区切りの優先列が効かない）。どちらも描画側の欠陥であり、台帳の担当も描画側の仕様のままとする。
  - バルーン名 `name,` の写像（`areka-P0-emo-text-canon-residue` 項目 14）。同じ写像関数を触るため同居させない。本仕様を先に着地させる。
  - 全体報告 `doc/ukadoc-coverage/report/summary.md` の作り直し。4 台帳を跨ぐため統合担当（`areka-P0-ukadoc-coverage-roadmap`）が行うと開発者裁定（2026-09-02 議題 2）で決まっており、常時の検査にも入っていない。本仕様は申し送りだけを行う（Requirement 7.7）。
- **Adjacent expectations**:
  - **前提**: なし（即着手可）。
  - **読み手（下流）**: `areka-P0-text-decoration-canon`（書体 10 項目の既定層として読む）・`areka-P0-text-align-shadow-canon`（影 4 キーの既定層として読む）・`areka-P0-emo-text-canon-residue`（同じ写像関数へ後着）。
  - **配線の担当は後着**（brief の相互登記）: 本仕様が先に着地したら、既定層として読む配線は下流が行う。逆に下流が先に着地していたら、本仕様が最後に配線する（Requirement 8）。
  - **隣接**: `areka-P0-anchor-tag-canon` は `anchor.font.*` を同じ写像関数で引くため、後着側が突合する。`areka-P0-ukadoc-coverage-roadmap` は同じウェーブで `doc/ukadoc-coverage/` 配下の新しい文書を作るが、台帳とドメイン別報告には触れない。

## Requirements

### Requirement 1: 対象キー集合を正典の実測で確定する

**Objective:** 下流の実装者として、「バルーン定義の基底の書体設定」と言ったときに何が入るのかを 1 か所で確定しておきたい。そうすれば、読む側が数を取り違えて一部のキーを配線し忘れることがない。

#### Acceptance Criteria

1. The 本仕様 shall 対象キー集合を、網羅調査のカタログに載る接頭辞なし `font.*` の見出し全数（**14**）と定め、その一覧（`font.bold`・`font.color.b`・`font.color.g`・`font.color.r`・`font.height`・`font.italic`・`font.name`・`font.outline`・`font.shadowcolor.b`・`font.shadowcolor.g`・`font.shadowcolor.r`・`font.shadowstyle`・`font.strike`・`font.underline`）を本文へ書き出す。
2. The 本仕様 shall 14 のうち既に写像済みの 5 本（`font.color.r`／`.g`／`.b`・`font.name`・`font.height`）と、未写像の 9 本（残り）を明示して数え、零（＝対象外 0 本）も明示的に書く。
3. When 生きている文書が「基底 13 キー」「残り 8 キー」と書いている, the 本仕様 shall その **4 文書**を「14」「残り 9」へ是正し、数え落としていたのが `font.outline` であることを添える。対象は ⑴ 本仕様の brief、⑵ 分割元 `areka-P0-text-decoration-canon` の brief、⑶ `areka-P0-text-align-shadow-canon` の brief（`:27` の対象外の行）、⑷ `.kiro/steering/roadmap.md`（`:124`。ここは「13 キー」とは書かず「残り 8 キー」とだけ書く）である。履歴と完了済みの記録（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/` 配下）は、非改変の方針と着地時点の記録という理由でいずれも対象外とする。
4. If 実装の途中で対象キー集合がカタログと食い違うことが分かった, then the 本仕様 shall 見た目で数え直さずカタログの行を照合元とし、食い違いの内容を記録する。

### Requirement 2: 未写像の 9 キーを解析結果へ写す

**Objective:** バルーン作者として、`descript.txt` に書いた書体・影の宣言が、少なくとも areka の内部で読み取られている状態にしたい。そうすれば、描画側が用意され次第、バルーンを書き直さずに宣言が効くようになる。

#### Acceptance Criteria

1. When バルーン定義に `font.bold`・`font.italic`・`font.outline`・`font.strike`・`font.underline` のいずれかが書かれている, the バルーン定義の転記層 shall その宣言を解析結果から取り出せる形で保つ。
2. When バルーン定義に `font.shadowcolor.r`・`font.shadowcolor.g`・`font.shadowcolor.b` のいずれかが書かれている, the バルーン定義の転記層 shall 3 つの成分を**個別に**保ち、1 つだけ書かれている場合に他の 2 つが未指定であることを区別できるようにする。
3. When バルーン定義に `font.shadowstyle` が書かれている, the バルーン定義の転記層 shall その宣言を解析結果から取り出せる形で保つ。
4. The バルーン定義の転記層 shall 上の 9 キーそれぞれについて、「未指定」「宣言された値」の 2 つを互いに区別できる形で示す。
5. If 宣言された値が正典の語彙に無いもの（たとえば `font.bold,2`・`font.shadowstyle,blur`・`font.shadowcolor.r,300`）である, then the バルーン定義の転記層 shall その値を落とさずそのまま保ち、警告も縮退も行わない（判定は下流の責務）。
6. If `font.shadowcolor.r`／`.g`／`.b` に正典が定める `none`（影を無効化する語）が書かれている, then the バルーン定義の転記層 shall `none` と「未指定」と「数値」の 3 つを互いに区別できる形で保つ。
7. The バルーン定義の転記層 shall 9 キーのいずれについても、値の解釈・描画・エラー通知を行わない（解析結果を返すだけで、失敗し得ない）。

### Requirement 3: 正典の既定値は適用せず、下流へ申し送る

**Objective:** 下流の実装者として、既定値がどの層で入るのかを一意に決めておきたい。そうすれば、転記層と描画層の両方が既定値を入れて二重に効いたり、どちらも入れずに抜けたりしない。

#### Acceptance Criteria

1. While バルーン定義にキーが書かれていない, the バルーン定義の転記層 shall 正典の既定値を代入せず「未指定」のまま返す。
2. The 本仕様 shall 14 キーの正典既定値を本文の表に登記し、下流が適用する値として申し送る。登記する値は `font.name`＝`ＭＳ ゴシック`、`font.height`＝`12`、`font.color.r`／`.g`／`.b`＝`0`、`font.bold`／`font.italic`／`font.outline`／`font.strike`／`font.underline`＝`0`、`font.shadowcolor.r`／`.g`／`.b`＝`none`、`font.shadowstyle`＝`offset` とする。
3. The 本仕様 shall `font.shadowstyle` の語彙が `offset`（右下にずれた表示）と `outline`（縁取り）の 2 語であること、および同キーが SSP 2.5.27 で登場したことを併せて登記する。
4. The 本仕様 shall 既定値の適用先が下流（`areka-P0-text-decoration-canon`＝書体 10・`areka-P0-text-align-shadow-canon`＝影 4）であることを、どちらの仕様がどのキーを受け持つかまで書き分けて申し送る。

### Requirement 4: 既存の引き方の不変条件を保つ

**Objective:** バルーン作者として、選択肢マーカーやアンカーのために書いた `cursor.font.*`・`anchor.font.*` が、本体の書体設定へ化けないことを保証してほしい。そうでないと、選択肢の色を変えただけで本文の色が変わるような、原因の分からない崩れ方をする。

#### Acceptance Criteria

1. When バルーン定義の既定層と画像別の上書き層の両方に同じ `font.*` キーが書かれている, the バルーン定義の転記層 shall 画像別の上書き層の値を採る。
2. When 画像別の上書き層に当該キーが無い, the バルーン定義の転記層 shall バルーン定義の既定層の値を引き継ぐ。
3. When 画像別の上書き層そのものが無い, the バルーン定義の転記層 shall バルーン定義の既定層だけを写す。
4. When 接頭辞付きのキー（`anchor.font.shadowcolor.r`・`anchor.font.shadowstyle`・`anchor.notselect.font.shadowcolor.r`・`anchor.visited.font.shadowstyle`・`cursor.font.shadowcolor.g`・`cursor.font.shadowstyle`・`cursor.notselect.font.shadowcolor.b`・`number.font.height`・`sstpmessage.font.name`・`communicatebox.font.color.r`・`disable.font` 等）が書かれている, the バルーン定義の転記層 shall それらの値を接頭辞なしの基底キーへ一切反映しない。
5. The バルーン定義の転記層 shall 本仕様が写像しないキー（選択肢・リンク・スクロール系など）を、従来どおり無視する。

### Requirement 5: 既存の解析結果を 1 つも変えない

**Objective:** 運用者として、この変更でバルーンの見た目が変わらないことを保証してほしい。装いを足す仕様ではなく、読み取る口を増やすだけの仕様だからである。

#### Acceptance Criteria

1. The バルーン定義の転記層 shall 既に写像済みの 5 キー（`font.color.r`／`.g`／`.b`・`font.name`・`font.height`）の解析結果を変えない。
2. The バルーン定義の転記層 shall 幾何・書字方向・選択肢マーカー・窓の配置など、`font.*` 以外の解析結果を変えない。
3. The 本仕様 shall 既存の決定論テストの期待値を緩めない。既存のテストが赤になった場合は期待値を書き換えず、原因を是正する。
4. When 本仕様の変更を取り込んだ, the areka shall 描画される文字の見た目を変えない（読み取る値が増えるだけで、使う側がまだ無いため）。

### Requirement 6: 定義箇所に正典 URL の証拠を置く

**Objective:** 後から読む人として、どの行がどの正典項目に対応しているのかを、実装を追い直さずに 1 行で確かめたい。

#### Acceptance Criteria

1. The 本仕様 shall 14 キーそれぞれの定義箇所に、正典 URL 1 行のコメントを 1 本ずつ置く。既に置かれているのは **4 本**（`font.color.r`・`font.color.g`・`font.color.b`・`font.height`）であり、**新たに置くのは 10 本**（未写像の 9 本＋`font.name`）である。`font.name` の行には説明のコメントは在るが正典 URL の行が無いので、写像済みの 5 本のうち 1 本が未設置である。
2. The 本仕様 shall 置く URL をカタログの当該行から写し、見た目で打ち直さない。
3. The 本仕様 shall コメントを行の先頭（字下げを除く）がコメント記号で始まる形とし、`ukadoc:` の後に URL 1 語だけを置いて説明文を続けない（続けると機械が証拠として拾わない）。
4. The 本仕様 shall 正典 URL を定義箇所だけに置き、呼び出し側には置かない。
5. When 網羅調査の常時検査を走らせた, the 検査 shall 本仕様が置いた URL について「カタログに無い URL」を 1 件も報告しない。

### Requirement 7: 網羅台帳とドメイン別報告を実測に合わせる

**Objective:** 網羅状況を読む人として、台帳が「areka は今どこまでできているか」を正しく映していてほしい。台帳が古いままだと、次に手を着ける項目の順番付けが狂う。

#### Acceptance Criteria

1. When 9 キーの写像が着地した, the 本仕様 shall 当該 9 項目の状態を `absent` から `vocabulary-only`（名前だけ登記してあり、受け取っても何もしない）へ改める。
2. The 本仕様 shall 9 項目の備考から「読むキーを並べた表を完全一致で引く形で、この項目はその表に無い」という、実測と合わなくなる記述を除き、代わりに「転記層は読むが、使う側がまだ無い」という実態と、その根拠（どのログが出ないか）を書く。
3. The 本仕様 shall 14 項目すべての備考から「担当 spec の brief はバルーンの font 系を 13 キーと書くが、正典の見出しは 14 種ある（担当 spec の記述が古い）」の一文を除く（Requirement 1.3 で是正済みになるため）。
4. The 本仕様 shall 影の 4 項目（`font.shadowcolor.r`／`.g`／`.b`・`font.shadowstyle`）の担当を `areka-P0-text-align-shadow-canon` へ改め、残る 10 項目の担当は `areka-P0-text-decoration-canon` のまま据え置く（分割の裁定で影と寄せ 2 が同仕様へ移ったため。裁定の文が言う「影 3」は `\f` タグ側の項目数であり、descript 側の見出しは影色 3 成分＋形態の 4 キーである）。
5. While 9 項目の壊れ方の段が変わらない（宣言しても見た目が変わらず、記録も出ないまま）, the 本仕様 shall 優先度 `A11` を据え置き、束の名前だけを実態に合わせ、同じ束の項目には同じ優先度を付けるという台帳の規則を保つ。束の名前は `A11` に属する **10 項目すべて**（未写像の 9 本＋`font.name`）で揃えて改める。現在の束名「台詞の書体・読む経路が無い」は `font.name` については**すでに事実と合っていない**（同項目の備考自身が「完全一致で引いて文字列のまま持ち上げ」と書いており、読む経路は在る）ので、10 項目を揃えて改めることが、束に嘘を 1 件も残さない唯一の形である。
6. The 本仕様 shall 残る 5 項目の状態を据え置く——`font.color.r`／`.g`／`.b`・`font.height` は `implemented` のまま、`font.name` は `absent` のままとする。`font.name` は写像されてはいるが、描画側の 2 つの食い違い（バルーンのフォルダに置いたフォントファイルを指定できない・カンマ区切りの優先列が効かない）が残るためであり、本仕様はその食い違いを引き受けない。
7. When 台帳を書き換えた, the 本仕様 shall ドメイン別報告 `doc/ukadoc-coverage/report/assets.md` を道具で作り直して同じコミットに入れ、全体報告 `doc/ukadoc-coverage/report/summary.md` には触れずに、統合担当（`areka-P0-ukadoc-coverage-roadmap`）へ「assets 台帳の状態分布が変わった」ことを申し送る。
8. When 網羅調査の常時検査とテストを走らせた, the 検査 shall 台帳・報告について 1 件も食い違いを報告しない。

### Requirement 8: 下流への引き渡しを相互登記する

**Objective:** 下流の実装者として、既定層を読む配線を誰がいつ行うのかを、両方の仕様を読まなくても分かる形にしておきたい。

#### Acceptance Criteria

1. Where 本仕様が下流（`areka-P0-text-decoration-canon`・`areka-P0-text-align-shadow-canon`）より先に着地する, the 本仕様 shall 既定層として読む配線を行わず、14 キーの取り出し口と正典既定値の表を申し送るところまでを範囲とする。
2. Where 下流が先に着地していた, the 本仕様 shall 最後の作業として、写像した値を下流の既定層へ繋ぐ。
3. The 本仕様 shall どちらの順序で着地したかを、着地時点の実測（下流の仕様の状態）で判定し、思い込みで決めない。
4. The 本仕様 shall 引き渡す内容（14 キーの一覧・値の形・未指定の表し方・正典既定値・どの下流がどのキーを受け持つか）を、下流が読める形で本仕様の文書に残す。

### Requirement 9: 決定論テストで固定する

**Objective:** 開発者として、この写像が後の変更で静かに壊れないようにしたい。転記層は失敗を報告しない作りなので、壊れても誰も気付かないからである。

#### Acceptance Criteria

1. The 本仕様 shall 実機にもネットワークにも触れない決定論テストで、9 キーそれぞれについて「宣言した値が取り出せる」ことを固定する。
2. The 本仕様 shall 「未指定のときに未指定として表れる」ことを、宣言された `0` と取り違えない形で固定する。
3. The 本仕様 shall 正典の語彙に無い値（`font.bold,2`・`font.shadowstyle,blur` 等）が落とされず素通しされることを固定する。
4. The 本仕様 shall `font.shadowcolor.r`／`.g`／`.b` について、`none`・数値・未指定の 3 つが互いに区別されることと、3 成分の部分欠落が個別に表れることを固定する。
5. The 本仕様 shall 2 層の優先順位（画像別が勝つ・画像別に無ければ既定層を引き継ぐ・画像別の層そのものが無い）を、新しい 9 キーについても固定する。
6. The 本仕様 shall 接頭辞付きのキーが基底キーへ漏れないことを、影のキー（`anchor.font.shadowcolor.r`・`cursor.font.shadowstyle` 等）を含めて固定し、既存の同種のテスト行を壊さない。
7. The 本仕様 shall 対象キー集合が 14 であることを、数え上げで確かめられる形で固定する（表示するだけの数にせず、食い違えば赤になる判定にする）。
8. When テストを走らせた, the 本仕様 shall 対象クレートのテストと網羅調査の道具のテストの両方が緑であることを、出力を切り詰めない形で確かめる。

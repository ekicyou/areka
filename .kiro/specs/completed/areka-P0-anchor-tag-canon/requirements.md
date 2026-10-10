# Requirements Document

## Project Description (Input)

さくらスクリプトのアンカー `\_a[ID]…\_a`（バルーンの本文の中の、押せるリンク）を areka で働かせる。今はタグが読み捨てられ、文字は出るが押しても何も起きず、`OnAnchorSelect`／`OnAnchorSelectEx` も送られない。本 spec はアンカーの**働き**（4 つの形の読み取り・範囲の保持・ホバーとクリック・イベントの送出・既定の見た目 1 種類）だけを持つ。見た目の作者指定（`\f[anchor*]` 16 項目×3 状態・descript の `anchor.*.font.*` 族・訪問済み・縦書きの下線の位置）は `areka-P0-anchor-style-canon` が持つ（出所: `brief.md`・2026-10-04 棚卸㉑の分割）。

## Introduction

ゴーストの台詞の中にあるリンク（アンカー）を、利用者が押せるようにする。押すとゴーストへ `OnAnchorSelectEx`（答えが無ければ続けて `OnAnchorSelect`）が届き、ID が `On` で始まる形ならその名前のイベントが直接届く。選択肢（`\q`）と違い、アンカーは答えを待つ柵を作らず、選択肢の時間切れも無い。バルーンに出ている間だけ押せ、バルーンが消えれば一緒に消える（ukadoc `\_a[ID]`「選択肢タイムアウトしないが、通常トーク同様バルーンタイムアウトする」）。

正典の出所（ukadoc・MCP で 2026-10-10 に引いた）:

- `\_a[ID]`: 「`\_a`がくるまでの範囲をアンカーとする。…ジャンパをクリックするとSHIORIイベントOnAnchorSelectが開始される(SSPのみ先立ってOnAnchorSelectExが開始される)。IDはReference0に格納される。」
- `\_a[ID,r2,r3...]`: 「ジャンパとなったテキストがReference0に、IDはReference1に、続く引数はReference2以降に格納される。OnAnchorSelectExに続けてOnAnchorSelectも発生する」
- `\_a[OnID,r0,r1...]`: 「IDが"On"で始まっている場合は、クリックするとSHIORIイベントOnID(書いた通りのイベント)が開始される。IDに続く引数が順番にReference0以降に格納される。」
- `OnAnchorSelectEx`: 「`\_a`ジャンパがクリックされた瞬間に発生。このイベントにSHIORIが何も返さなかった場合にのみ、続けてOnAnchorSelectが発生する。」Reference0＝ジャンパのテキスト・Reference1＝ID・Reference*＝拡張情報（`\_a` タグ内の 2 番目以降の引数）。
- `OnAnchorSelect`: 「`\_a`ジャンパがクリックされた瞬間に発生。」Reference0＝選択されたジャンパの ID。
- descript `anchor.style`（選択中アンカーの形状）の既定値は `underline`。
- 読みの注記: さくらスクリプトの項 `\_a[ID,r2,r3...]` は「OnAnchorSelectExに続けてOnAnchorSelectも発生する」と、常に続くように読める書き方だが、イベントの項 `OnAnchorSelectEx` は「何も返さなかった場合にのみ」と限定する。areka はイベントの項に従う（選択肢の `OnChoiceSelectEx`→`OnChoiceSelect` と同じ形・Requirement 4.2／4.3）。
- ukadoc の `\_a` には `\q` の `script:` のような ID の綴りに意味を持たせる形は無い。areka は ID をそのまま送り、綴りに特別な意味を持たせない（Requirement 4.1）。

## Boundary Context

- **In scope**: `\_a` の 4 つの形（`\_a[ID]`・`\_a[ID,r2,…]`・`\_a[OnID,r0,…]`・閉じの `\_a`）の読み取り／アンカーの範囲の保持／ホバーの強調とクリック／`OnAnchorSelectEx`→`OnAnchorSelect` の送出と `On` 始まりの直接送出／既定の見た目 1 種類／普通のバルーンとシェルの中の箱の両方／台本を再生せずに確かめる道具（`check_script`）での扱い／網羅台帳と互換記録の更新／決定論テスト。
- **Out of scope**:
  - 見た目の作者指定: `\f[anchorstyle]` ほか 16 項目×選択中／非選択／訪問済みの 3 状態・`\f[anchor.font.color]`・descript の `anchor(.notselect|.visited).font.*` 族・訪問済みの記録・縦書きの下線の位置の決め直し → `areka-P0-anchor-style-canon`。本 spec では、これらの指定は今までどおり受け取って保持するだけで、表示を変えない。
  - 範囲を選択肢にする `\__q` → `areka-P0-range-choice-tag`（本 spec の範囲の仕組みの上に乗る）。
  - アンカーの上で止まったときの `OnAnchorHover`・行き先の説明の表示 → `areka-P0-balloon-link-hover`。
  - 右クリックでの行き先のコピー → `areka-P0-link-context-copy`。
  - アンカーの既定の動作で OS を呼ぶこと（URL を開くなど）: ゴーストが `OnAnchorSelect(Ex)` の答えに `\j[…]` などを返す正典の作法で足りる（`\j`・`\![open,…]` は `areka-P0-open-external-tags` で着地済み）。本 spec はアンカーから OS を呼ばない。
  - 選択肢 `\q` の働き・見た目・時間切れ（着地済み・不変）。
- **Adjacent expectations**:
  - 字句の読み取り（角括弧の無い `\_a` を 1 つのタグとして切り出すこと）は `areka-P0-sakura-bare-tag-lexer` で着地済み。本 spec はその上で `\_a` に意味を与える。
  - 下線を引く描画の基盤（文字の区間への下線）は `areka-P0-text-decoration-canon` で着地済み。本 spec の既定の見た目はそれを使う。
  - 選択肢のクリックとホバー（当たりの行・強調・箱の中の押下の結論）・選択由来のイベントの送出（カスケード・単一の再生枠への差し替え・`On` 始まりの受理）は `areka-P0-choice-select-events`／`choice-interact`／`choice-render` で着地済み。アンカーは同じ利用者体験を踏襲し、違いだけを本書に書く。
  - 本 spec が作る「範囲を押せる仕組み」は、後に `range-choice-tag`・`link-context-copy`・`balloon-link-hover` が使う。範囲の当たりの持ち方はそれらが使い回せる形にする（具体の形は設計で決める）。

## Requirements

### Requirement 1: `\_a` の 4 つの形の読み取り

**Objective:** As a ゴースト作者, I want 台本に書いた `\_a` の全ての形が正典どおりに読まれること, so that 既存のゴーストの辞書を書き換えずにアンカーが働く

#### Acceptance Criteria

1. When 台本に `\_a[ID]` がある, the areka 台本の読み手 shall これを「ID だけを持つアンカーの開き」として読み、付随する引数は無しとする。
2. When 台本に `\_a[ID,r2,r3,…]` がある, the areka 台本の読み手 shall これを「ID と、2 番目以降の引数の列を持つアンカーの開き」として読み、引数の並びと綴りを変えない。
3. When 台本に ID が `On` で始まる `\_a[OnID,r0,r1,…]` がある, the areka 台本の読み手 shall これを「ID が `On` で始まるアンカーの開き」として読み、ID に続く引数の列を記述順に保つ。
4. When 台本に角括弧の無い `\_a` がある, the areka 台本の読み手 shall これを「アンカーの閉じ」として読む。
5. The areka 台本の読み手 shall `\_a[…]` の引数の区切りと引用の規則を、`\q[…]` など他の角括弧付きタグと同じにする。
6. The areka 台本の読み手 shall `\_a` の 4 つの形のいずれも「知らないタグ」の印を付けず、読み捨てない。
7. When アンカーの開きと閉じの間に文字・改行・字の装飾のタグがある, the areka shall それらを今までどおり表示し、開きから閉じまでに表示される文字の並びをアンカーの範囲とする。
8. If アンカーの開きがあるのに同じ台本の中に閉じが無い, then the areka shall 範囲を台本の表示の終わりまでとし、警告として 1 件記録する。
9. If アンカーが開いている間に新たな開きがある, then the areka shall 直前のアンカーをそこで閉じて新たなアンカーを始め、警告として 1 件記録する。
10. If 開いていないのに閉じの `\_a` がある, then the areka shall 閉じを無視して表示を続け、警告として 1 件記録する。
11. If `\_a[]` のように ID が空である, then the areka shall ID を空の文字列として扱い、アンカーとしては成立させる（押せば ID が空のまま送る）。警告や「読めなかった印」は付けない（正典は ID の形を限っていない）。

### Requirement 2: アンカーの範囲と寿命

**Objective:** As a 利用者, I want 台詞の中のリンクがバルーンに出ている間ずっと押せること, so that 話を読みながら気になった所を掘り下げられる

#### Acceptance Criteria

1. While アンカーの範囲の文字がバルーンに表示されている, the areka shall その文字の上を押せる範囲とする。
2. While アンカーの範囲の文字がまだ表示されていない（1 字ずつ出ている途中で、その字に達していない）, the areka shall まだ表示されていない部分を押せる範囲に含めない。
3. When アンカーの範囲が行の折り返しをまたぐ, the areka shall 範囲が占める全ての行の、該当する部分を押せる範囲とする。
4. While ゴーストがまだ話している（台詞の表示が終わっていない）, the areka shall 既に表示されたアンカーを押せる状態に保つ。
5. The areka shall アンカーに選択肢の時間切れを適用せず、アンカーがあることを理由に答えを待つ柵を作らない。
6. The areka shall アンカーがあることを理由にバルーンの自動の閉じ（バルーンの時間切れ）を遅らせない（選択肢が出ている間は閉じを止める今の規則を、アンカーには広げない）。
7. When バルーンの本文が消える（次の台詞に置き換わる・`\c` で本文が消される・バルーンの時間切れ・台詞の中断）, the areka shall そのバルーンのアンカーを全て消し、以後そこを押しても何も送らない。台詞の終わり（`\e`）だけではバルーンは消えないので、アンカーはバルーンが閉じるまで押せる（Requirement 2.1）。
8. When 1 つの台詞に複数のアンカーがある, the areka shall それぞれを別の範囲として保ち、押した範囲のアンカーだけを送る。
9. The areka shall 普通のバルーンとシェルの中のバルーンの箱の両方で、同じ規則でアンカーの範囲を保つ。
10. The areka shall 台本の全文を先に受け取って下見する処理（表示の前の空回し）で、アンカーの範囲の記録や警告を本番と二重に出さない。

### Requirement 3: ホバーとクリック

**Objective:** As a 利用者, I want リンクの上にマウスを置くと分かり、押すと反応すること, so that どこが押せるかに迷わない

#### Acceptance Criteria

1. When マウスがアンカーの押せる範囲の上に乗る, the areka shall その範囲を、選択肢の行にマウスが乗ったときと同じ強調の見た目にする。
2. When マウスがアンカーの範囲から離れる、または窓から出る, the areka shall 強調を元に戻す。
3. When アンカーの押せる範囲が左ボタンで押される, the areka shall その 1 回の押下をアンカーの選択として扱い、同じ押下を台詞の中断・シェルの操作・その他の押下の意味に重ねて使わない。
4. While ゴーストがまだ話している, when アンカーの押せる範囲が左ボタンで押される, the areka shall それをアンカーの選択として扱い、台詞を中断しない。（要件討議 議題 2・2026-10-10 開発者確認: 正典が黙る分岐。答えの台本が来れば Requirement 4.8 の規律で置き換わり、204 なら台詞はそのまま続く。後の `talk-fast-forward` はこの順〔選択肢 → アンカー → 早送り／中断〕の後ろに並ぶ）
5. When 選択肢の行とアンカーの範囲が同じ押下の候補になる, the areka shall 選択肢を先に判定し、選択肢に当たらなかったときにアンカーを判定する。
6. When アンカーでも選択肢でもない場所が押される, the areka shall 今までどおりの押下の扱い（台詞の中断・シェルの操作など）を変えない。
7. The areka shall シェルの中のバルーンの箱でも、普通のバルーンと同じ条件でアンカーのホバーとクリックを受ける。
8. The areka shall 右ボタンや中ボタンの押下をアンカーの選択として扱わない。

### Requirement 4: イベントの送出と Reference の割付

**Objective:** As a ゴースト作者, I want アンカーを押したときに正典どおりのイベントと Reference が届くこと, so that 既存の辞書が参照位置を書き換えずに答えられる

#### Acceptance Criteria

1. When ID が `On` で始まらないアンカーが選択された, the areka shall まず `OnAnchorSelectEx` を送り、Reference0 にアンカーの範囲に表示された文字、Reference1 に ID、Reference2 以降に 2 番目以降の引数を記述順に載せる。ID の綴りに特別な意味を持たせない（選択肢の `script:` の約束は適用しない）。
2. If `OnAnchorSelectEx` にゴーストが何も返さなかった（204）, then the areka shall 続けて `OnAnchorSelect` を送り、Reference0 に ID を載せる。
3. If `OnAnchorSelectEx` にゴーストが台本を返した, then the areka shall `OnAnchorSelect` を送らない。
4. When ID が `On` で始まるアンカーが選択された, the areka shall ID と同じ名前のイベントを送り、ID に続く引数を Reference0 以降に記述順で載せ、表示された文字と ID を Reference に含めず、`OnAnchorSelectEx`／`OnAnchorSelect` を送らない。
5. The areka shall ID が `On` で始まるアンカーのイベント名を、事前の固定の登録なしに逐語で送る（選択肢の `On` 始まり ID と同じ受理規則）。
6. If 付随する引数が無い, then the areka shall 対応する Reference の位置を付けず、空の文字列で埋めない（選択肢と同じ規則）。
7. The areka shall アンカー由来のイベントにも、他のイベントと同じ共通の要求ヘッダ（実行状態の行を含む）を付ける。
8. When アンカー由来のイベントにゴーストが台本を返した, the areka shall その台本を今までどおりの台詞の起動の道で再生し、再生中の台詞があれば既存の単一の再生枠の規律に従って置き換える。
9. When アンカー由来のイベントにゴーストが何も返さなかった, the areka shall 表示中の台詞とバルーンをそのまま続ける。
10. If アンカー由来のイベントの送信に失敗した（送れない・内部の誤り）, then the areka shall 失敗を error として記録し、何も返らなかった（204）ときと同じ扱いで処理を続ける。
11. The areka shall 1 回のアンカーの選択に対して、イベントの列の送出を高々 1 回、台詞の起動を高々 1 回だけ行う。
12. The areka shall 消えたバルーンのアンカーへの知らせを kanade 側で改めて照合しない。バルーンが消えた時点で範囲が消える（Requirement 2.7）ことで「消えた後に知らせが作られない」ことを保証し、知らせの発行から受理までの隙間に台詞が置き換わった場合は、利用者が押したアンカーのイベントをそのまま送る（要件討議 議題 3・2026-10-10 開発者裁定: 照合の鍵を新設しない）。

### Requirement 5: 既定の見た目

**Objective:** As a 利用者, I want リンクが見ただけで分かること, so that 押せる所を見逃さない

#### Acceptance Criteria

1. While アンカーの範囲の文字が表示されていてマウスが乗っていない, the areka shall その文字に下線を引く（ukadoc の descript `anchor.style` の既定値 `underline` に合わせる）。（要件討議 議題 1・2026-10-10 開発者確認: 下線はバルーンの中で初めての常に見える目印になることを了解の上で採用）
2. While マウスがアンカーの範囲に乗っている, the areka shall 選択肢の行にマウスが乗ったときと同じ強調の見た目にする（Requirement 3.1）。
3. The areka shall 下線を、既に着地している文字の区間への下線の描画と同じ位置の規則で引く（縦書きの位置の決め直しは `anchor-style-canon` の範囲）。
4. The areka shall 作者が `\f[anchor*]`・`\f[anchor.font.color]`・descript の `anchor.*` 族で見た目を指定しても、本 spec の範囲では既定の見た目を変えず、指定を今までどおり受け取って保持する。
5. The areka shall 普通のバルーンとシェルの中の箱で同じ既定の見た目を出す。

### Requirement 6: 台本の検査の道具との整合

**Objective:** As a ゴースト作者, I want 台本を再生せずに確かめる道具が `\_a` を正しく扱うこと, so that アンカーのある台本が「知らないタグ」「誰も拾わない命令」と誤って診断されない

#### Acceptance Criteria

1. When `check_script` に `\_a` の 4 つの形を含む台本を渡す, the areka shall `\_a` を知らないタグとして報告しない。
2. When `check_script` に `\_a` を含む台本を渡す, the areka shall `\_a` を「誰も拾わない命令」として報告しない。
3. If `check_script` に Requirement 1.8〜1.10 の崩れた形（閉じ無し・開きの重なり・開き無しの閉じ）を含む台本を渡す, then the areka shall 本番と同じ内容の警告を結果に含める。

### Requirement 7: 互換記録と網羅台帳

**Objective:** As a 開発者, I want 正典に根拠がある決めと areka が裁定した決めを見分けて記録すること, so that 後の spec と実機の確認が同じ前提で進む

#### Acceptance Criteria

1. The areka 互換記録 shall `OnAnchorSelectEx`／`OnAnchorSelect`／`On` 始まりの直接送出の Reference の割付と順序を、ukadoc の該当記述と対応付けて記録する。
2. The areka 互換記録 shall 正典が黙っている分岐について areka が採った決めを、正典に根拠がある決めと見分けられる形で記録する。対象は少なくとも次を含む — 話している最中のアンカーのクリックの扱い（Requirement 3.4）・閉じの無い開き／開きの重なり／開き無しの閉じの扱い（Requirement 1.8〜1.10）・ID が空のときの扱い（Requirement 1.11）・ID の綴りに意味を持たせないこと（Requirement 4.1）・`OnAnchorSelect` を 204 のときだけ続ける読み（Introduction の読みの注記）・選択肢とアンカーが重なったときの判定の順（Requirement 3.5）・マウスが乗っていないときの既定の見た目を descript `anchor.style` の既定値から借りたこと（Requirement 5.1）。
3. When 本 spec が着地する, the areka 網羅台帳 shall `\_a[ID,r2,r3...]`・`\_a[OnID,r0,r1...]`（根 2 件）と `OnAnchorSelect`・`OnAnchorSelectEx` の行を「ある」（台帳の語彙では `implemented`）に改め、持ち主を本 spec にする。別名の行 `\_a[ID]` は台帳の慣例どおり `alias` のまま持ち主を持たない。
4. When 本 spec が着地する, the areka 網羅台帳 shall 本 spec の持ち物として残っているアンカーの見た目の行（`\f[anchor*]` 16 項目・`\f[anchor.font.color]`・descript の `anchor.*` 族）の持ち主を `areka-P0-anchor-style-canon` に付け替える。
5. The areka 互換記録 shall 設計の正本（`COMPAT_ARCHITECTURE.md` §8 の横断表）に本 spec の行を 1 行足し、詳細は互換記録へのポインタで引く。

### Requirement 8: 決定論テスト

**Objective:** As a 開発者, I want アンカーの判断の分岐が実時間・実 GPU・実 SHIORI なしに固定されること, so that 回帰が全体テストで赤になる

#### Acceptance Criteria

1. The areka shall `\_a` の 4 つの形の読み取り（Requirement 1.1〜1.4）と崩れた形（1.8〜1.11）を、台本の文字列だけから検証できるテストで固定する。
2. The areka shall 今「`\_a` は知らないタグとして素通しする」と固定している既存のテストを、本 spec の読み取りに合わせて書き換え、素通しを前提にした見本を `\_a` 以外の知らないタグに置き換える。
3. The areka shall Reference の割付とカスケード（Requirement 4.1〜4.6・4.11）を、模擬の SHIORI と注入した選択の知らせだけで検証できるテストで固定する。
4. The areka shall アンカーの範囲の当たり（Requirement 2.1〜2.3・2.8）と選択肢との判定の順（Requirement 3.5）を、決まった字幅の配置から検証できるテストで固定する。
5. The areka shall 普通のバルーンと箱の押下の結論（Requirement 3.3〜3.7）を、押下の座標と状態だけから検証できる純粋な判定のテストで固定する。
6. The areka shall 話している最中のクリック（Requirement 3.4）と、バルーンが消えた後は知らせが作られないこと（Requirement 2.7）を固定するテストを持つ。
7. The areka shall `check_script` の診断（Requirement 6.1〜6.3）を固定するテストを持つ。
8. Where 実機での確認を行う, the areka shall 実のゴーストで「アンカーを押す → ゴーストが答える → 台詞が置き換わる」の 1 周と、話している最中の押下で台詞が中断されないことを、人が確認できる状態にする。

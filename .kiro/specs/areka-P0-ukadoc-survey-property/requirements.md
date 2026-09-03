# Requirements Document

## Project Description (Input)

ukadoc のプロパティシステムのページ（`list_propertysystem`・実測 188 項目）について、areka が「どの名前に値を返し、どれを名前だけ持ち、どれを持っていないか」を 1 つの台帳へ書き出す調査を行う。

- **誰が困っているか**: このドメインは既に 4 本の brief（`areka-P0-property-query-channels`／`areka-P0-currentghost-property-tree`／`areka-P0-property-catalog-lists`／`areka-P0-zorder-property`）が所有を宣言している。それぞれ 2026-08-27 の手作業サーベイを元にしており、数え方が brief ごとに違う。統合担当（`areka-P0-ukadoc-coverage-roadmap`）も、これから実装する各 spec の担当者も、正典の項目 1 つ 1 つが誰の持ち物なのかを機械で辿れない。
- **現状**: 手作業サーベイの合計は約 180 項目で、正典の実測 188 と食い違う。版番号が付いた項目が 98 件あり（全ページ中で世代の差が最も濃い）、同じ値へ到達する名前が世代ごとに増えている可能性があるが、新旧の仕訳がどこにも無い。さらに `currentghost.seriko.zorder` を 3 本の spec が食い違って主張している（二重所有の裁定が未決）。
- **何が変わるか**: `areka-P0-ukadoc-survey-toolkit` が凍結した台帳形式で `doc/ukadoc-coverage/ledger/property.toml` を書き、正典 id 単位で状態・世代・担当 spec・優先度・関連を付ける。あわせて 4 本の brief の所有宣言を id 単位で突き合わせた表、別名の仕訳、二重所有の裁定案、既存 brief への是正候補を `doc/ukadoc-coverage/briefing-property.md` に書く。areka の実行時の挙動は 1 ビットも変えない（唯一のコード接触は語彙表の頭に置く正典 URL 1 行の doc コメント）。

## Introduction

本 spec は「ukadoc 網羅調査」6 本のうちの調査 spec 1 本であり、担当ドメインは**プロパティシステムの木**（ページ `list_propertysystem`）である。作るものは 3 つある。

1. **台帳** `doc/ukadoc-coverage/ledger/property.toml` — 正典 id ごとに areka の判定を書いた人手の文書。
2. **ブリーフィング** `doc/ukadoc-coverage/briefing-property.md` — 所有の突合表・別名の一覧・所有者なし／裁定待ちの一覧・二重所有の裁定案・既存 brief への是正候補。
3. **正典 URL 1 行** — sylphya の点付き語彙表の頭に置く doc コメント（実行時の挙動を変えない唯一のコード接触）。

台帳の項目形式・欄・状態語彙・仕訳の規則は `areka-P0-ukadoc-survey-toolkit`（要件承認済み）の要件 2・要件 4・付録 A で凍結されている。**本 spec はそれを再定義せず、従う側である。** 道具の実装完了は待たない（付録 B の手順で id 一覧をスナップショットから直接得られ、道具の初期台帳生成は既存の項目を書き換えない＝toolkit 要件 3.3a）。

本 spec の主務は「新規の数え直し」ではない。既に所有を宣言している 4 本の brief と正典 id を突き合わせ、**無所有・二重所有・新旧の名前**を仕訳することである。したがって成果物の中心は台帳そのものと、そこから読める突合表である。

**現状の実測（2026-09-02・本 spec 着手時に取り直した値）**

- ページ `list_propertysystem` の項目は **188 件**（brief 記載と一致）。うち本文に版番号が現れるもの **98 件**（同）。版番号の種類は 21 種（`2.3.72` から `2.8.80` まで）。
- 見出しの重複が 2 件ある（`name` が 2 id・`path` が 2 id）＝**見出しの名前だけでは id が定まらない項目が存在する**。見出しの異なり数は 186。
- id のアンカー部に見出しの日本語を符号化した文字列を含むものが **67 件**（例: `ukadoc:list_propertysystem:system.os._28_30ad_30fc_29:1`）。見出しに丸括弧のセレクタ（`(ID)`／`(名前)`／`(キー)`）を含む項目が 105 件。
- 見出しの先頭で数えた内訳: `currentghost` 69・`system` 25・`rateofuselist` 24・`history` 12・汎用プロパティ名の葉 17・サウンドの要素葉 18（うち `meta.*` 8）・`ghostlist` 5・`activeghostlist` 5・`pluginlist` 5・`balloonlist` 3・`headlinelist` 3・`baseware` 2 ＝ 188。
- 汎用プロパティ名の葉 17 件は、sylphya の `GENERIC_PROP_NAMES` 17 名（`crates/areka-sylphya/src/vocab/dotted.rs:37`）と**名前が 1 対 1 で一致する**。
- 本文から版番号を拾うと SSP の版でない番号が混じる項目が 1 件ある（`system.os.(キー)` が `2.6.26`／`2.8.17`／`5.19.0` の 3 つを含み、`5.19.0` は本文が例示する OS の版）。複数の版番号を持つ項目は本ページではこの 1 件だけ。
- 関連の相手になる他ドメインの id は実在する: `%property[プロパティ名]`／`\![get,property,イベント名,プロパティ名,プロパティ名,...]`／`\![set,property,プロパティ名,値]`（いずれも `list_sakura_script`）・`property.get`／`property.set`（`list_shiori_event` と `list_plugin_event` の双方）。
- `baseware.name`／`baseware.version` の実値（`"areka"` と版文字列）を sylphya の大域点付き区画へ流し込んでいる定義箇所は sylphya の語彙表ではなく `crates/areka-ghost/src/sylphya_wiring.rs:126-127`（2 行の組）である。語彙表 `dotted.rs` はルート枝 `baseware` を名前として持つだけで、`baseware.name` という文字列は見出しの名前と一致する要素として現れない。
- sylphya は語彙表を実行時の判定にも使っている（`crates/areka-sylphya/src/actor.rs:136-165`）: SET の宛先が「根がルート枝 10 のいずれか、または葉が汎用プロパティ名 17 のいずれか」なら正典の語彙とみなして「受理＋警告＋非反映」、それ以外の自由な名前は保存へ回す。読み取り側（`reader.rs`）は語彙表を見ず、値が無ければ一律に「値なし」を返す。
- areka 側（sylphya の点付き語彙表・`crates/areka-sylphya/src/vocab/`）: ルート枝 10（`dotted.rs:17`）・汎用プロパティ名 17（`dotted.rs:37`）・SET 有効群 21（`dotted.rs:72`・件数を固定するテストは同 `:191`）・`property.get`／`property.set` の名前の予約（`dotted.rs:106` と `:109`）。**M1 で実際に値へ導出するのは `baseware.*` だけで、他のルート枝の配下は値なしへ縮退する**と宣言されている（`dotted.rs:4-5`・宣言ブロック全体は `:1-9`）。件数を固定するテストは `crates/areka-sylphya/src/ledger_key_determinism_tests.rs:201-204`。
- 語彙表には既に「ukadoc `list_propertysystem.html`」という語が 2 か所あるが（`dotted.rs:3` と `:67`）、**いずれも URL を伴わない**＝toolkit 要件 5.6 により証拠として扱われない。本ページの正典 URL はソース中に 0 件。
- 縮退の転記元になる既存の登記: `doc/COMPAT_ARCHITECTURE.md:184`（`currentghost.balloon.scope(ID).vertical` の導出規則と、値の枝も照会経路も無いという 2 つの穴）・同 `:185`（`validwidth`／`validheight`／`lines` の 2.8.83 意味論と、スナップショットが 2.8.80 のままである罠）・同 `:136`（SET が無効な名前への書込は受理＋警告＋非反映）。
- `doc/ukadoc-coverage/` は現時点で存在しない。

## Boundary Context

- **In scope**:
  - `list_propertysystem` の全 id を収めた台帳 1 本（`doc/ukadoc-coverage/ledger/property.toml`）。
  - 4 本の brief の所有宣言と正典 id の突合表、別名の仕訳、所有者なし／裁定待ちの一覧。
  - `currentghost.seriko.zorder`（および同じ形の食い違いがある `currentghost.seriko.sticky-window`）の二重所有についての**裁定案**。
  - 値の源・SET が有効な葉の書込先・照会経路の相手 id を `links` へ登記すること。
  - ブリーフィング 1 本（`doc/ukadoc-coverage/briefing-property.md`）。
  - sylphya の点付き語彙表の頭に置く正典 URL 1 行の doc コメント。
- **Out of scope**:
  - 実装（プロパティ値の導出・照会経路の新設・語彙表の件数変更をいずれも行わない）。
  - 照会経路そのものの項目（`%property[...]`・`\![get,property,...]`・`\![set,property,...]` は `areka-P0-ukadoc-survey-sakura-script`、`property.get`／`property.set` は `areka-P0-ukadoc-survey-shiori` の台帳の持ち物）。本台帳はそれらを**相手として指すだけ**で、項目としては置かない。
  - 既存 4 brief の本文の書き換えと、その優先順位の変更。
  - 段階（A〜E）の最終決定（`areka-P0-ukadoc-coverage-roadmap` の仕事）。
  - 統合報告 `doc/ukadoc-coverage/report/summary.md` の生成。
  - 台帳の項目形式・状態語彙・仕訳規則の定義（`areka-P0-ukadoc-survey-toolkit` が凍結済み）。
  - SSP 実機との挙動比較、ukadoc 本文の repo への取り込み。
- **Adjacent expectations**:
  - `areka-P0-ukadoc-survey-toolkit`: 台帳の形式と仕訳の規則を凍結済みとして受け取る。道具の実装完了は待たない。道具が後から初期台帳を生成しても、本 spec が書いた項目は書き換えられない（同要件 3.3a）。道具の整合検査のうち「カタログの全 id が 4 つの台帳のいずれか 1 つにちょうど 1 回だけ現れる」（同要件 6.4）と「関連の両端の id が実在する」（同 6.7）は、他の 3 ドメインの台帳が揃うまで本 spec の台帳 1 本では確かめられない。本 spec はこれを不合格とは扱わず、ブリーフィングの末尾に「他 3 台帳が揃うまで検査できない項目」として注記する。
  - `areka-P0-ukadoc-survey-sakura-script` と `areka-P0-ukadoc-survey-shiori`: 同じ機能の別の面を別ページの id として持つ。同じ id を 2 つの台帳へ置かない。
  - 既存 4 brief: 本 spec は読むだけで書き換えない。是正候補はブリーフィングに置き、受け取るかどうかは各 brief の担当者と `ukadoc-coverage-roadmap` が決める。
  - 編集するファイルは「自分の台帳 1 本＋自分のブリーフィング 1 本＋（道具の着地後は）自分のドメイン別報告 1 本＋ソースの doc コメント」に限られ、他の調査 spec と共有するファイルは無い（並走できる）。

## Requirements

### Requirement 1: 正典 188 項目を 1 件も落とさず収める

**Objective:** 統合担当として、プロパティのページの項目が 1 件も抜けずに台帳へ入っていてほしい。それにより「数え直しても同じ数になる」状態が保てる。

#### Acceptance Criteria

1. When 台帳を書き終える, the property 台帳 shall カタログのページ `list_propertysystem` に属する全 id（2026-09-02 の実測は 188 件）について項目を 1 つずつ持ち、id を 1 件も落とさない。
2. The property 台帳 shall 同ページ以外の id を 1 件も置かない。
3. The プロパティ調査 shall 台帳の項目形式・欄・状態語彙・仕訳の規則を `areka-P0-ukadoc-survey-toolkit` の要件 2・要件 4・付録 A のとおりに用い、本 spec で再定義しない。
4. When id を写す, the プロパティ調査 shall スナップショットの id 文字列を逐語で写し、見出しの日本語を符号化した部分（実測 67 件が `_28_30ad_30fc_29` のような形を含む）を読みやすい形に直さない。`currentghost.seriko.cursor.scope(ID).mouse????list...` の 5 件に含まれる `????`（id では `_3f_3f_3f_3f`）は正典ページの見出しの表記そのものであり（2026-09-02 に生ページで確認。`mouseuplist`／`mousedownlist`／`mousehoverlist`／`mousewheellist` の総称）、写しの過程で失われた記号ではないので、ブリーフィングでは「正典の表記どおり」と説明する。
5. The property 台帳 shall 項目を id の文字順に並べ、同じ id を 2 度書かない。
6. If 台帳の件数がカタログと食い違う, then the プロパティ調査 shall カタログ（id）を正とし、食い違いの内容をブリーフィングに記録する。
7. The プロパティ調査 shall 台帳ファイルを `doc/ukadoc-coverage/ledger/property.toml` 1 本に限り、カタログ・他ドメインの台帳・テーマ定義を編集しない。
8. Where 道具（`areka-P0-ukadoc-survey-toolkit`）がまだ着地していない, the プロパティ調査 shall 付録 B の手順で自分のドメインの id 一覧をスナップショットから直接得て、道具の完成を待たずに台帳を書く。

### Requirement 2: 全項目に状態と壊れ方を付ける

**Objective:** これから実装する spec の担当者として、正典の名前 1 つ 1 つについて areka が今どう応答するかを知りたい。それにより「実装済みのつもりだった」項目を見つけられる。

#### Acceptance Criteria

1. When 台帳を書き終える, the property 台帳 shall 全項目の状態が凍結済みの 7 語彙のいずれかであり、`unclassified` が 0 件である状態にする。
2. Where 項目が areka で実際に値へ導出される, the プロパティ調査 shall 状態を「実装済み」とする（2026-09-02 の実測では点付き語彙のうち実導出は `baseware.*` のみと宣言されており〔`crates/areka-sylphya/src/vocab/dotted.rs:4-5`〕、該当するカタログ id は `ukadoc:list_propertysystem:baseware.name:1` と `baseware.version` の 2 件で、実値の定義箇所は `crates/areka-ghost/src/sylphya_wiring.rs:126-127`。着手時に再確認する）。
3. Where 名前は sylphya の語彙表に登記されているが値を返さない, the プロパティ調査 shall 状態を「語彙のみ」とする。「登記されている」は sylphya が実行時に正典の語彙と判定する規則そのもの（`crates/areka-sylphya/src/actor.rs:136-165`）で決める: id の根がルート枝 10 のいずれかであるか、葉が汎用プロパティ名 17 のいずれかであれば登記済みとみなす（開発者裁定 2026-09-02 議題 1＝塊単位。areka はこれらの名前を認識し、書き込みに対して「受理＋警告＋非反映」で応答するため、「未対応」と呼ぶと実挙動と食い違う）。葉の名前が語彙表に文字どおり載っているか否か（汎用 17・SET 有効群 21）は備考で区別する。
4. Where 正典が定める応答に対して areka が別の応答（値なし・受理して非反映など）を返すことが既に登記されている, the プロパティ調査 shall 状態を「縮退」とし、転記元（`doc/COMPAT_ARCHITECTURE.md` の該当行）を備考に書く。
5. Where 名前が語彙表にも無く応答も無い（2.3 の規則で登記済みとみなされない）, the プロパティ調査 shall 状態を「未対応」とする（本ページの 188 件は全て根がルート枝 10 のいずれかに属するため、2026-09-02 の実測では該当 0 件の見込み。0 件ならその旨をブリーフィングに書く）。
5a. When 備考を書く, the プロパティ調査 shall 読み取りの応答（値を返すか・値なしか）に加えて書き込みの準拠（正典で SET が有効か〔6.4 の 3 形で読む〕と、areka の応答〔SET 有効群なら型だけの予約で実書込なし・語彙のみなら受理＋警告＋非反映・自由な名前なら保存〕が正典と一致するか）を 1 件ずつ書き、状態そのものは 2.9 のとおり読み取りで決める。
6. Where 項目が areka の担当範囲の外にある（正典が他の実装主体の持ち物として定めているなど）, the プロパティ調査 shall 状態を「対象外」とし、なぜ対象外なのかを備考に書く。
7. Where 状態が「別名」である, the プロパティ調査 shall その行で実装状態を判定せず、写像先の正典の行に委ねる。
8. The プロパティ調査 shall 状態が「別名」「対象外」である項目を除く全項目の備考に「壊れ方」（値を返せないときに黙って壊れるか・明示的なエラーになるか・見た目の差にとどまるか）を書き、その根拠としてどのログが出るか／出ないかを添える（「別名」は写像先の行に委ね〔2.7〕、「対象外」は理由だけを書く〔2.6〕）。
9. The プロパティ調査 shall 状態を「areka が値を返すか否か」だけで決め、返す値が正典どおりの意味かどうか（意味論の当否）の判定は所有 spec に委ねる。
10. If 状態が「実装済み」である, then the プロパティ調査 shall その根拠となる正典 URL をソースの定義箇所に置く（要件 9）。

### Requirement 3: 世代と新旧の名前を仕訳する

**Objective:** 実装する spec の担当者として、同じ値へ到達する名前が複数あるときにどれが正典でどれが旧名かを知りたい。それにより同じ値を別名で 2 度実装せずに済む。

#### Acceptance Criteria

1. When 項目の登場した版を決める, the プロパティ調査 shall カタログ（道具の着地前はスナップショットの本文）から拾った版番号の集合の中から選び、集合が空なら「世代不明」を表す空文字とする。
2. The プロパティ調査 shall 版番号が無い項目を最も古いものと決めつけない。
3. If カタログの版番号に SSP の版でない番号が混じる（実測 1 件＝`system.os.(キー)` が `2.6.26`／`2.8.17`／`5.19.0` を含み、`5.19.0` は本文が例示する OS の版）, then the プロパティ調査 shall その番号を登場した版として採らず、採らなかった理由を備考に書く。
4. When 同じ値へ到達する名前が 2 つ以上ある, the プロパティ調査 shall 最も新しい書式を正典・それ以外を別名とし、別名の側から正典の id を指す。向きの決め方は凍結済みの順序（本文の注記 → 版番号 → 人手の判断）に従う。
5. The property 台帳 shall 別名が指す先が別名でない状態に保つ（別名の連鎖を作らない）。
6. Where 新しい名前が旧名を置き換えたことが本文から読める, the プロパティ調査 shall 新しい側から旧 id への「置き換えた」関係も登記してよい。
7. When 別名の仕訳を終える, the プロパティ調査 shall 「同じ値へ到達する名前の群」の一覧（正典 1 つと別名の対応）をブリーフィングに載せ、そのような群が 1 つも無かった場合はその旨を明記する。
8. If brief が挙げた新旧の例がカタログに存在しない（brief は `currentghost.balloon.scope(ID).*` に対する旧 `balloon.*` 系を例示するが、2026-09-02 の実測では本ページに `balloon.` で始まる見出しの id は 0 件）, then the プロパティ調査 shall 例が成り立たなかったことをブリーフィングに記録し、実在する名前だけで仕訳する。

### Requirement 4: 4 本の brief の所有宣言を id 単位で突き合わせる

**Objective:** 統合担当として、正典の項目 1 つ 1 つに担当 spec が 1 つだけ対応している状態がほしい。それにより無所有の項目と、2 本以上が主張している項目が一目で分かる。

#### Acceptance Criteria

1. When 突合を行う, the プロパティ調査 shall 4 本の brief（`areka-P0-property-query-channels`／`areka-P0-currentghost-property-tree`／`areka-P0-property-catalog-lists`／`areka-P0-zorder-property`）が所有を宣言している項目名を、1 つずつカタログ id へ対応付けた表をブリーフィングに載せる。
2. If brief に書かれた項目名がどの id にも対応しない, then the プロパティ調査 shall それを表記の揺れとして表に残し、憶測で近い id に結び付けない。
3. If brief の件数がカタログと食い違う（実測の例: `property-catalog-lists` brief は `history` を 8・`headlinelist` を 2・`pluginlist` を 4 と書くがカタログはそれぞれ 12・3・5。`currentghost-property-tree` brief は `currentghost.*` を約 65 と書くがカタログは 69）, then the プロパティ調査 shall カタログを正とし、差を表に記録する。
4. When 突合を終える, the property 台帳 shall 各項目の担当 spec 欄に spec 名 1 つ、または「裁定待ち」「所有者なし」を表す空文字を持つ。
5. If 担当 spec 欄が空文字である, then the プロパティ調査 shall その項目の全件をブリーフィングの「所有者なし」または「裁定待ち」の一覧に載せる。
6. The プロパティ調査 shall 所有者が見つからない項目を、憶測で既存の brief に押し込まない。まだ起票されていない spec を引受先として提案したい場合は、担当 spec 欄には書かず（付録 A により担当 spec 欄は実在する spec 名か空文字に限る）、備考とブリーフィングの「所有者なし」の一覧に「候補: ...」として書く。
7. Where 2 本以上の spec が同じ id を主張し、一方は値の導出を、他方は sylphya の語彙表の 1 行だけを担当している, the プロパティ調査 shall 担当 spec 欄には値を導出する側を書き、語彙表だけを触る側を備考に書く。
8. Where 項目が既に実装済みで、実装した spec が完了済みである, the プロパティ調査 shall 担当 spec 欄にその完了済み spec の名前を書いてよい（誰が実装したかの記録として）。
9. If 項目が未実装である, then the プロパティ調査 shall 担当 spec 欄に完了済みの spec を書かない（完了した spec は新しい作業を引き受けられないため）。
10. The プロパティ調査 shall 既存 brief の本文を書き換えず、是正候補はブリーフィングにだけ置く。

### Requirement 5: 二重所有について裁定案を出す（決めるのは本 spec ではない）

**Objective:** 開発者として、3 本の spec が食い違って主張している項目について、選択肢と影響が並んだ紙の上で 1 度だけ決めたい。それにより着手後に所有が割れたまま実装が進むことを避けられる。

#### Acceptance Criteria

1. When `currentghost.seriko.zorder` を仕訳する, the プロパティ調査 shall 3 本の spec（`areka-P0-zorder-property`／`areka-P0-currentghost-property-tree`／`areka-P0-property-query-channels`）がそれぞれ何を主張しているかを、brief の該当箇所を示してブリーフィングに並べる。
2. When 主張を並べ終える, the プロパティ調査 shall 裁定案を 1 つ出し、それが**案であって決定ではない**ことをブリーフィングに明記する。
3. When 裁定案を出す, the プロパティ調査 shall その案を採った場合と採らなかった場合に、3 本それぞれの作業がどう変わるかを併記する。
4. While 裁定が未了である, the property 台帳 shall 当該 id の担当 spec 欄を空文字のままにし、備考に裁定待ちであることと候補を書く。
5. Where 同じ形の食い違いが `currentghost.seriko.sticky-window` にもある（`property-query-channels` brief が語彙表の追随に含め、`currentghost-property-tree` brief が `seriko.*` の一括所有に含める）, the プロパティ調査 shall 同じ扱いで並べ、同じ裁定案の対象に含める。
6. The プロパティ調査 shall 裁定そのものを行わず、開発者と `areka-P0-ukadoc-coverage-roadmap` へ渡す。

### Requirement 6: 値の源と照会経路を関連として登記する

**Objective:** 統合担当として、プロパティの葉が「何から値を得て」「誰から読まれるか」を台帳から辿りたい。それにより繋がりの束で優先度を測れる。

#### Acceptance Criteria

1. When 照会経路との関連を書く, the property 台帳 shall さくらスクリプト側と SHIORI／PLUGIN 側の実在する id を種別つきで指す（相手の実測 id: `%property[プロパティ名]`／`\![get,property,イベント名,プロパティ名,プロパティ名,...]`／`\![set,property,プロパティ名,値]`／`property.get`／`property.set`）。
2. The property 台帳 shall 照会経路そのものを項目として置かない（それらは `areka-P0-ukadoc-survey-sakura-script` と `areka-P0-ukadoc-survey-shiori` の台帳の持ち物であり、同じ id を 2 つの台帳へ置かない）。
3. Where 葉の値の源が descript のキー・OS のメトリクス・他エンジンが持つ状態である, the プロパティ調査 shall その相手を種別つきの関連として登記する。
4. When SET が有効な葉を仕訳する, the プロパティ調査 shall 正典の側を基準にして、sylphya の SET 有効群 21 名（`crates/areka-sylphya/src/vocab/dotted.rs:72`・件数を固定するテストは同 `:191`）をカタログ id へ対応付け、⑴ 対応が付かなかった名前と ⑵ 正典では SET が有効なのに 21 名に無い葉、の双方をブリーフィングに列挙する。「正典で SET が有効」は本文の 3 つの書き方のいずれかで読む: ⒜ 見出しの `[SET有効]` の印（実測 14 件）⒝ 本文の「設定も可能」「設定時」「書き込みの場合」などの記述（例: `currentghost.scope(ID).surface.num` は印が無くても「設定も可能で `\s[]` タグと同じ挙動」と書かれている）⒞ 族の頭の「以降 ... で始まるプロパティに共通」による継承（例: `currentghost.mousecursor` の本文が `currentghost.mousecursor`／`currentghost.balloon.mousecursor` で始まる全項目に印を及ぼす）。印の件数だけを正としない。
5. The property 台帳 shall 関連の相手 id をカタログに実在するものだけに限る。
6. Where 汎用プロパティ名の葉がカタログに 1 件ずつ置かれ、複数のルート枝の下で使い回される（実測 17 件で、sylphya の `GENERIC_PROP_NAMES` 17 名〔`dotted.rs:37`〕と名前が 1 対 1 に一致する）, the プロパティ調査 shall その乗算の関係を関連と備考で表し、ルート枝ごとに項目を増やさない。
7. Where サウンドの要素葉がカタログに 1 件ずつ置かれる（実測 18 件＝`meta.*` 8 を含む。`currentghost.sound.*` 3 件と合わせて 21 件で、`property-catalog-lists` brief の約 21 と一致する）, the プロパティ調査 shall 同じ扱いで登記し、要素ごとに項目を増やさない。

### Requirement 7: テーマと優先度を仮に置く

**Objective:** 統合担当として、優先度を決める材料が全項目に付いていてほしい。それにより段階の最終決定を数字の上で行える。

#### Acceptance Criteria

1. The property 台帳 shall テーマ欄を原則として空にする（本ドメインは値を運ぶ配管であり、利用者がゴーストの何を失うかに答えられない項目が大半であるため）。
2. If テーマを付ける, then the プロパティ調査 shall 「この項目が無いと利用者はゴーストの何を失うか」への答えを備考に必ず書く。
3. Where 答えが書ける項目がある（例: 着せ替えの状態を表す `currentghost.seriko.*`）, the プロパティ調査 shall 凍結済みの 8 つのテーマ名の中からだけ選んで付ける。
4. When 優先度を仮に置く, the プロパティ調査 shall 段階を表す 1 文字と数値の形で書き、段階 C の末尾を既定とする。
5. The プロパティ調査 shall 優先度の並びを、凍結済みの 4 つの根拠の序列（壊れ方 → テーマ → 影響する既存資産の広さ → 依存する基盤の共有度）で決める。
6. The プロパティ調査 shall 段階の最終決定を行わない（`areka-P0-ukadoc-coverage-roadmap` の仕事である）。

### Requirement 8: ブリーフィングと報告を出す

**Objective:** 既存 brief の担当者として、自分の brief のどの記述をどう直せばよいかを 1 つの文書から読み取りたい。それにより所有範囲を id の一覧へ書き換えられる。

#### Acceptance Criteria

1. When 調査を終える, the プロパティ調査 shall `doc/ukadoc-coverage/briefing-property.md` を書き、⑴ 所有の突合表 ⑵ 別名の一覧 ⑶ 所有者なし／裁定待ちの一覧と優先度 ⑷ 二重所有の裁定案 ⑸ 既存 brief への是正候補 ⑹ カタログとの件数の差、の 6 つを載せる。
2. The property ブリーフィング shall 是正候補を「どの brief のどの記述を、どの id の一覧へ書き換えるか」の形で書き、書き換えそのものは行わない。⑹ の件数の差のうち原因が特定できたもの（実測: `currentghost` 69 対約 65 は `currentghost.balloon.mousecursor` 系 4 件がどの束にも入っていない・`history` 12 対 8／`headlinelist` 3 対 2／`pluginlist` 5 対 4 はいずれも `.count` の葉の数え落とし）は、⑸ の是正候補としても同じ形で書く。
3. Where 道具（`areka-P0-ukadoc-survey-toolkit`）が着地している, the プロパティ調査 shall `doc/ukadoc-coverage/report/property.md` を台帳から再生成して台帳と一緒にコミットし、報告を手で編集しない。
4. Where 道具がまだ着地していない, the プロパティ調査 shall 報告ファイルを作らない。
5. The プロパティ調査 shall 統合報告 `doc/ukadoc-coverage/report/summary.md` を作らず、触らない。
6. The property ブリーフィング shall 平易な日本語で書き、プロジェクトの中でしか通じない言い回しを持ち込まない。

### Requirement 9: コードに触れるのは正典 URL 1 行だけ

**Objective:** 並走している他 spec の担当者として、この調査が実行時の挙動を 1 ビットも変えないと確信したい。それにより自分の作業への影響を考えずに済む。

#### Acceptance Criteria

1. The プロパティ調査 shall ソースへの変更を doc コメントの追加だけに限り、実行時の挙動を変えない。
2. When 正典 URL をソースに置く, the プロパティ調査 shall `crates/areka-sylphya/src/vocab/dotted.rs` の点付き語彙表の頭に `list_propertysystem` のページ URL（`https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html`）を 1 つだけ置き、個々の名前と id の対応は見出しの名前で付ける。
3. The プロパティ調査 shall 同じディレクトリの `flat.rs`（`%` で始まる平坦な語彙＝さくらスクリプトのページの持ち物）と `shiori_resource.rs`（SHIORI のリソース＝shiori のページの持ち物）に本ページの URL を置かない。
4. If 見出しの名前がカタログの中で 2 件以上の id に一致する（実測 2 件＝`name` と `path` がそれぞれ 2 つの id を持つ）, then the プロパティ調査 shall 名前だけでは対応が定まらないことをブリーフィングに記録し、どちらの id を指すのかを台帳の備考で示す。
5. The プロパティ調査 shall 語彙表の件数を変えず、SET 有効群への追加や汎用プロパティ名の変更を行わない（件数を固定するテスト `crates/areka-sylphya/src/ledger_key_determinism_tests.rs:201-204` と `crates/areka-sylphya/src/vocab/dotted.rs:191` が現状のまま通ること）。
6. When ソースへ URL を置き終える, the プロパティ調査 shall ワークスペースの標準のテスト実行が通ることを確かめる。
7. The プロパティ調査 shall 実装済みの根拠に行番号や内部の識別子を使わず、正典 URL だけを使う。
8. Where ソースに「ukadoc」の語だけがあって URL を伴わない箇所がある（実測 2 件＝`crates/areka-sylphya/src/vocab/dotted.rs:3` と同 `:67`）, the プロパティ調査 shall それを実装済みの根拠として数えない。

### Requirement 10: 境界を守り、他の持ち物に触れない

**Objective:** 開発者として、この調査が並走中の他 spec や既に確定した裁定を壊さないと確信したい。それにより 4 本の調査を同時に走らせられる。

#### Acceptance Criteria

1. The プロパティ調査 shall 実装（値の導出・照会経路の新設・語彙表の件数変更）を 1 つも行わない。
2. The プロパティ調査 shall 他の 3 ドメイン（shiori・assets・sakura-script）の台帳と報告を編集しない。
3. The プロパティ調査 shall 既存 4 brief の本文とその優先順位を変えない。
4. Where スナップショットの本文が現行の正典より古い箇所がある（`currentghost.balloon.scope(ID)` の `validwidth`／`validheight`／`lines` は 2.8.83 で役割が入れ替わったが、スナップショットは 2.8.80 の記述のままである）, the プロパティ調査 shall 台帳にはスナップショットの id と版番号だけを写して意味の判定を行わず、既に確定している登記（`doc/COMPAT_ARCHITECTURE.md:185`。`.vertical` の導出規則は同 `:184`）を備考の転記元として指す。
5. The プロパティ調査 shall `.kiro/steering/roadmap.md` を変更しない。
6. The プロパティ調査 shall ukadoc の本文を repo に取り込まず、台帳に写すのは id・状態・版番号・担当 spec・優先度・テーマ・関連・備考に限る。
7. The プロパティ調査 shall SSP 実機との挙動比較を行わない。

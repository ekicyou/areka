# Requirements Document

## Project Description (Input)

統一プロパティシステム sylphya が持つ「SET が有効なプロパティ名の一覧」が、ukadoc（互換ベースウェアの正典）の現行版より古い。古いままでは、`\![set,property,...]` の経路が別 spec で開通しても、正典どおりに書けると判定される項目が 21 で頭打ちになる。本 spec は値の導出（実際に設定を反映させる処理）には一切手を付けず、**語彙の台帳だけを正典へ追随させる**。あわせて、ukadoc 網羅調査の台帳（`doc/ukadoc-coverage/ledger/property.toml`）の担当欄を埋め、互換記録（`doc/COMPAT_ARCHITECTURE.md` §8）に所有の相互参照を 1 行足す。

## Introduction

本機能は areka の**統一プロパティ語彙台帳**（sylphya が保持する正準プロパティ名の一覧）を対象とする。

利用者（ゴースト作者）から見た変化は 1 点に尽きる: **正典で「設定できる」と定められているプロパティ名へ書き込みを試みたとき、areka が「そもそも設定できない名前」として扱うのをやめる**。本リリースでは実際の設定反映はまだ行わない（書込 API の型シームの予約にとどまる。これは既存の SET 有効群 21 項と同じ扱い）。読み取り側の挙動は一切変わらない（未提供のプロパティは従来どおり「見つからない」を返す）。

照合の正本は完了 spec `areka-P0-ukadoc-survey-property` が作った調査台帳 `doc/ukadoc-coverage/ledger/property.toml`（188 件）であり、正典 URL は同じ id を持つ `doc/ukadoc-coverage/catalog.toml` の行から採る（調査台帳自身は URL 欄を持たない）。

先送り中の `currentghost.seriko.zorder` は本機能では登記しない（開発者裁定 2026-09-11。完了 spec `areka-P0-scope-zorder-pinning` 要件 13.5 の先送りを維持する）。

## Boundary Context

- **In scope**
  - 正典で SET 有効と定められていながら未登記の 4 項目を、SET 有効語彙へ登記する——`seriko.sticky-window`（SSP 2.8.78）とサウンドの 3 葉 `pause`／`playing`／`position`（SSP 2.8.72）。
  - サウンドプロパティの語彙族 18 葉を、**既存の語彙表に相乗りする形で**正準語彙として登記する（うち 3 葉は上記 SET 有効語彙、残り 15 葉は設定できない正準語彙）。
  - 登記の件数を固定する検査を、新しい数と導出根拠つきで更新する。
  - 本 spec が登記した各項目に正典 URL の証跡を残す。
  - 調査台帳 `property.toml` の担当欄が空の 2 行を埋め、「裁定待ち」の記述を取り除く。
  - 互換記録 `doc/COMPAT_ARCHITECTURE.md` §8 に、`currentghost.seriko.zorder` の語彙台帳の行と値の導出がともに追跡 spec `areka-P0-zorder-property` に属することを示す相互参照を足す。
- **Out of scope**
  - **`currentghost.seriko.zorder` の登記**（先送り維持。語彙台帳の行も値の導出も `areka-P0-zorder-property` が持つ）。
  - 値の導出（実際に設定を読み書きして窓・サーフェス・音の状態を動かすこと）。`areka-P0-zorder-property`／`areka-P0-currentghost-property-tree`／`areka-P0-property-catalog-lists` が持つ。
  - `\![set,property,...]`／`\![get,property,...]` などの照会経路そのもの。`areka-P0-property-query-channels` が持つ。
  - `.ext.拡張プロパティ名` 系 4 項目の語彙と運搬。`areka-P0-property-ipc-transport` が持つ（本機能が数える 26 件に含めない理由は要件 4.2 に明記する）。
  - sylphya の M1 縮退宣言文そのものの改訂。`areka-P0-currentghost-property-tree` が持つ。
- **Adjacent expectations**
  - 本機能は、語彙台帳に載っている名前が**実際に効く**ことを保証しない。効くようになるのは上記 Out of scope の各機能が着地したときである。
  - 本機能が先に着地することで、後続 4 spec が同じ台帳ファイルを取り合わずに済む（着手順の前提）。
  - 本機能は、先送り中のプロパティを守る既存の決定論テストを**緑のまま**残す義務を負う（要件 2）。そのテストは他機能の持ち物であり、本機能はそれを書き換えずに通す。

## Requirements

### Requirement 1: 正典で設定可能なプロパティ名の追随

**Objective:** As a ゴースト作者, I want 正典が「設定できる」と定めるプロパティ名が areka でも設定可能な名前として扱われること, so that 正典どおりに書いた台本が「そもそも設定できない名前」として黙って捨てられずに済む

#### Acceptance Criteria

1. The 統一プロパティ語彙台帳 shall `seriko.sticky-window` を、設定可能な名前として保持する。
2. The 統一プロパティ語彙台帳 shall サウンドプロパティのうち正典が SET 有効と定める 3 葉（`pause`・`playing`・`position`）を、設定可能な名前として保持する。
3. When 要件 1.1／1.2 で登記した名前への書き込みが試みられたとき, the 統一プロパティ語彙 shall その書き込みを「設定できない正準語彙」としてではなく「運行コマンドとして予約済み」の扱いへ分類する。
4. While 本リリースの範囲である間, when 要件 1.1／1.2 で登記した名前への書き込みが試みられたとき, the 統一プロパティ語彙 shall 実際の設定反映を行わず、既存の SET 有効群と同じ「受理し、予約済みである旨を記録し、値を反映しない」応答を返す。
5. The 統一プロパティ語彙台帳 shall 登記する名前の綴りを、既存の SET 有効群と同じ書き表し方（先頭の親枝とセレクタを除いた末尾の形）で導く。
6. The 統一プロパティ語彙台帳 shall 既に登記済みの 21 項目の綴り・順序・分類を変更しない。

### Requirement 2: 先送り中のプロパティの尊重

**Objective:** As a 互換ベースウェアの保守者, I want 先送りと決めたプロパティ名が台帳の追随に紛れて登記されないこと, so that 「動かない名前が一覧に載ると利用者から見て提供済みと区別できなくなる」という既存の裁定が守られる

#### Acceptance Criteria

1. The 統一プロパティ語彙台帳 shall `currentghost.seriko.zorder` を、完全な名前でも短縮した形でも、いずれの語彙表へも登記しない。
2. The 統一プロパティシステム shall 本機能の変更後も、`currentghost.seriko.zorder` への書き込みを「設定できない正準語彙」として分類し、参照に対して「見つからない」を返す（変更前と同一の応答）。
3. When 本機能の変更を適用した後に既存の決定論テスト（`crates/areka/src/placement/zorder_property_deferral_tests.rs` の語彙表走査・書込分類・ソース全走査の 3 本）を実行したとき, the 統一プロパティ語彙台帳 shall それらを成功させる。
4. The 統一プロパティ語彙台帳 shall 要件 2.3 のテストが属するファイルを変更しない。
5. The areka の互換記録 shall 完了 spec `areka-P0-scope-zorder-pinning` の要件 13.5 と、それを記録した互換記録の既存行を、本機能では改訂しない。

### Requirement 3: サウンドプロパティ語彙族の登記（既存表への相乗り）

**Objective:** As a 互換ベースウェアの保守者, I want サウンドプロパティの名前が族ごと正準語彙として記録されていること, so that 正典にある名前が areka の台帳から丸ごと欠けている状態を解消し、後続が値の導出だけを足せる

#### Acceptance Criteria

1. The 統一プロパティ語彙台帳 shall 照合元台帳が持つサウンドプロパティの葉 18 件すべてを、正準語彙として保持する。
2. The 統一プロパティ語彙台帳 shall 要件 3.1 の 18 葉のうち、要件 1.2 の 3 葉を除く 15 葉を、設定できない正準語彙として保持する。
3. The 統一プロパティ語彙台帳 shall 要件 3.1／3.2 の登記を、**既存の語彙表に項目を足す形**で行い、新しい公開の語彙表を作らない。
4. When 要件 3.2 で登記した名前への書き込みが試みられたとき, the 統一プロパティ語彙 shall 正典に設定可能との定めが無い名前として扱い、実際の設定反映を行わない。
5. When 要件 3.1 で登記した名前の参照が試みられたとき, the 統一プロパティ語彙 shall 本リリースでは値を持たない名前として、従来どおり「見つからない」を返す。
6. When 本機能の変更を適用した後に、語彙表の増減を見張る既存の決定論テスト（`crates/areka/src/placement/zorder_property_deferral_tests.rs` の公開語彙表の名簿検査）を実行したとき, the 統一プロパティ語彙台帳 shall それを成功させる。
7. If 要件 3.2 の登記によって、サウンド族の外にある既存の名前の書込分類が変わるならば, the 統一プロパティ語彙台帳 shall 変わる名前を残らず列挙し、その変化が利用者から見て何を意味するかを設計フェーズで明示する。

### Requirement 4: 件数の明示的な導出と、判定する検査

**Objective:** As a 互換ベースウェアの保守者, I want 台帳の件数が固定され、その数がどこから来たかが書かれ、かつ機械が不一致を失敗として判定すること, so that 台帳が黙って古びたり、登記漏れ・二重登記が緑のまま通り抜けたりしない

#### Acceptance Criteria

1. The 統一プロパティ語彙台帳 shall SET 有効語彙の件数を **25** に固定する。
2. The 統一プロパティ語彙台帳 shall 25 という数の導出を、次の 2 通りの数え方の双方として文書に明記する（どちらか一方だけでは数の正しさを確かめられないため）。
   - **登記の側**: 既存 21 項＋本機能が足す 4 項（`seriko.sticky-window`・`pause`・`playing`・`position`）＝ 25。
   - **照合元の側**: 調査台帳が「正典 SET 有効」と記録する行は 26 行。そこから先送り中の `currentghost.seriko.zorder` 1 行を除いて 25 行。この 25 行は語彙表の 23 項目へ対応する——`seriko.cursor.path` と `seriko.tooltip.text` はセレクタの書き方違いで台帳が 2 行に分かれているが語彙表では 1 項目ずつだから、合わせて 2 項目ぶん縮む。これに、正典に設定の定めが無いまま areka が先取りで登記している 2 項目（`seriko.cursor.name`・`seriko.tooltip.name`）を足して 25。
3. The 統一プロパティ語彙台帳 shall 要件 4.2 の 2 通りが同じ 25 に着くのは偶然の一致であり、「26 から 1 を引く」だけの数え方は語彙表の件数の根拠にならないことを、検査の側に注記として残す。
4. The 統一プロパティ語彙台帳 shall 照合元台帳が「正典 SET 有効」と記録する 26 行に `.ext.拡張プロパティ名` 系 4 行を含めない理由（同 4 行は本機能の範囲外であり、担い手は `areka-P0-property-ipc-transport` であること）を文書に明記する。
5. The 統一プロパティ語彙台帳 shall 要件 4.2 の「先取りで登記している 2 項目」が正典と食い違っている既知の状態であり、本機能はその食い違いを解消しないことを文書に明記する。
6. The 統一プロパティ語彙台帳 shall 設定できない正準語彙の件数を固定する検査を、本機能の登記後の実数へ更新し、その数の導出（サウンド 15 葉のうち既存の汎用名として既に載っている `name`・`path` の 2 件を除いた分が新規であること、および `meta.` を伴う 8 葉をどの綴りで載せたか）を文書に明記する。
7. The 統一プロパティ語彙台帳 shall 件数を固定する記述が複数箇所に写し取られている場合（実測 2026-09-11: SET 有効群の 21 という数は `crates/areka-sylphya/src/vocab/dotted.rs` の件数の主張と網羅の主張、`crates/areka-sylphya/src/ledger_key_determinism_tests.rs` の網羅の主張と付随する説明文の計 4 箇所に写されている）、そのすべてを更新し、どれか 1 箇所だけが古いまま残る状態を作らない。
8. The 統一プロパティ語彙台帳 shall 件数の検査を、数を印字するだけでなく、期待と実数の不一致を失敗として判定する形で持つ。
9. If 登記した名前に重複があるならば, the 統一プロパティ語彙台帳 shall その重複を検査で検出し、失敗として報告する。
10. When 本機能が足した各項目について名前の解決を試したとき, the 統一プロパティ語彙台帳 shall 要件 1.3／3.4 が定めるとおりの分類になることを、項目ごとに確かめる検査を持つ。

### Requirement 5: 正典 URL の証跡

**Objective:** As a 互換ベースウェアの保守者, I want 登記した各項目がどの正典記述に由来するかを追えること, so that 後続が正典を引き直すとき、名前から出典へ一手で戻れる

#### Acceptance Criteria

1. The 統一プロパティ語彙台帳 shall 本機能が登記した各項目の定義箇所に、その項目の正典 URL を 1 行の注記として残す。
2. The 統一プロパティ語彙台帳 shall 注記に書く URL を、調査カタログ `doc/ukadoc-coverage/catalog.toml` の同一 id を持つ行が保持する URL と一致させる（調査台帳 `property.toml` は URL 欄を持たないため、URL の出所はカタログの側である）。
3. The 統一プロパティ語彙台帳 shall 注記の書き表し方を、本リポジトリで既に用いられている同種の注記（`/// ukadoc: <正典 URL>`）と同じ形にする。
4. The 統一プロパティ語彙台帳 shall 既存の 21 項目に対しては注記の追加を要求しない（本機能が登記した項目のみを対象とする）。

### Requirement 6: 調査台帳の担当欄記入

**Objective:** As a 互換ベースウェアの保守者, I want 調査台帳で担当が空のまま残っている行が埋まること, so that 「裁定待ち」の行が残り続けて誰も引き取らない状態を終わらせられる

#### Acceptance Criteria

1. The ukadoc 調査台帳 shall `currentghost.seriko.zorder` の行の担当欄に `areka-P0-zorder-property` を記入する（語彙台帳の行も値の導出も同 spec が持つ）。
2. The ukadoc 調査台帳 shall `currentghost.seriko.sticky-window` の行の担当欄に `areka-P0-sylphya-set-ledger` を記入する。
3. When 要件 6.1／6.2 の記入が済んだとき, the ukadoc 調査台帳 shall 当該 2 行の備考から「裁定待ち」を示す記述を取り除き、代わりに決まった所有を記す。
4. The ukadoc 調査台帳 shall 要件 6.1〜6.3 以外の行の内容を変更しない。
5. When 本機能の変更を適用した後に担当欄の空きを数えたとき, the ukadoc 調査台帳 shall その数を **0** として示せる（変更前は 2 件であり、上記 2 行が最後の空きである）。

### Requirement 7: 実行時に見える挙動の不変

**Objective:** As a ゴースト作者, I want 台帳の更新によって既存の動作が変わらないこと, so that 語彙を増やしただけで既存ゴーストの挙動が崩れることがない

#### Acceptance Criteria

1. The 統一プロパティシステム shall 本機能の変更によって、プロパティの参照結果を 1 件も変えない。
2. The 統一プロパティシステム shall 本機能の変更によって、既存 21 項目および正準語彙外の自由な名前に対する書き込みの扱いを変えない。
3. The 統一プロパティシステム shall 本機能の変更によって、窓の重なり・サーフェス・音の再生など、利用者に見える状態を 1 つも動かさない。
4. If 本機能の登記が既存の決定論テストを失敗させるならば, the 統一プロパティ語彙台帳 shall その失敗を「台帳が古い前提を固定していた」ものと「本機能が挙動を壊した」ものへ切り分け、前者のみを更新し、後者は変更を取り下げる。
5. The 統一プロパティ語彙台帳 shall 要件 7.4 の切り分けにおいて、要件 2.3／3.6 が指すテストを「台帳が古い前提を固定していた」側に分類しない（それらは先送りが守られていることの証拠であり、失敗したなら本機能の側が誤っている）。

### Requirement 8: 互換記録への相互参照

**Objective:** As a 互換ベースウェアの保守者, I want 先送り中のプロパティの所有が互換記録から一意に読めること, so that 同じプロパティを 3 つの spec が取り合う状態が記録の上でも終わる

#### Acceptance Criteria

1. The areka の互換記録 shall `currentghost.seriko.zorder` に関する既存の記録へ、語彙台帳の行も値の導出もともに `areka-P0-zorder-property` が持つことを示す参照を足す。
2. The areka の互換記録 shall 要件 8.1 の追記を、既存の記録行を書き換えずに足せる形で行う。
3. The areka の互換記録 shall 先送りした語彙の正本が追跡 spec の brief であるという既存の記述を、本機能が持ち越さない（語彙を書き写さない）。
4. The areka の互換記録 shall 本機能が触る箇所を `doc/COMPAT_ARCHITECTURE.md` §8 の該当行の周辺に限り、他機能の節を変更しない。

### Requirement 9: 編集の及ぶ範囲の固定

**Objective:** As a 互換ベースウェアの保守者, I want 本機能が触るファイルが宣言どおりに限られること, so that 同じ台帳を後で触る 4 つの機能が衝突の解消に手間を取られない

#### Acceptance Criteria

1. The 統一プロパティ語彙台帳 shall 本機能の変更を、次の範囲に限る——`crates/areka-sylphya/src/vocab/dotted.rs` と同クレート内のその兄弟テスト、`doc/ukadoc-coverage/ledger/property.toml`、`doc/COMPAT_ARCHITECTURE.md` の §8。
2. The 統一プロパティ語彙台帳 shall `crates/areka/src/placement/zorder_property_deferral_tests.rs` を変更しない。
3. If 要件 1〜8 のいずれかを満たすために要件 9.1 の範囲外のファイルを変更する必要が生じたならば, the 統一プロパティ語彙台帳 shall その変更を行わず、範囲外に触れる必要が生じた事実と理由を報告する。

# Requirements Document

## Project Description (Input)
areka が SHIORI（ゴーストの脳）と交わすやり取り、およびシェル定義ファイル surfaces.txt の読み取りを、ukadoc が定めるとおり**任意の文字コード**で行えるようにする。

- **困っている人**: Shift_JIS や EUC-JP など UTF-8 以外の文字コードで書かれた既存ゴースト（里々の標準テンプレート・古い YAYA ゴーストの大半）を areka で動かしたい利用者と、そのゴーストの作者。
- **現状**: ファイル層（descript.txt・バルーン定義）は `charset,<名前>` 宣言に従って任意の文字コードを読める。しかし ⑴ SHIORI との通信は UTF-8 に固定されており、Shift_JIS で応答する SHIORI の返事は読めず（挨拶が一切出ない）、areka が送る日本語（利用者の入力・ドロップされたファイルのパス等）は SHIORI 側で文字化けしたまま処理される。⑵ surfaces.txt は文字コード宣言を見ずに UTF-8 決め打ちで読むため、Shift_JIS のシェルは起動に失敗する。⑶ ゴーストの descript にある `shiori.encoding`／`shiori.forceencoding` はどこにも読まれない。
- **変わるべきこと**: SHIORI との通信の文字コードを ukadoc の規則（descript の宣言＋SHIORI の応答ヘッダによる交渉）で決め、要求の符号化と応答の復号を選ばれた文字コードで行う。surfaces.txt はファイルごとの `charset` 宣言に従って読む。対応する文字コードの集合は「ファイル層が今解決できる集合と同一」（WHATWG Encoding Standard の全ラベル）であり、Shift_JIS は検体の 1 つに過ぎない。UTF-8 のゴースト（emo2）の挙動は 1 バイトも変えない。

---

## Introduction

伺かのゴーストは、ベースウェア（areka）と SHIORI（脳）の間で SHIORI/3.0 というテキストのやり取りをする。そのテキストの文字コードは固定ではなく、ukadoc は要求・応答の双方に `Charset` ヘッダを定め、ゴースト側の descript に `shiori.encoding`／`shiori.forceencoding` という指定を用意している。UTF-8 以外の文字コードで書かれたゴーストは今も大量に存在し、里々の標準テンプレートはその代表である。

areka の現状は「半分だけ対応」である。ファイル層（descript.txt・バルーン定義）は `charset,<名前>` 宣言から任意の文字コードを解決して読めるが、SHIORI との通信は UTF-8 に固定されており、surfaces.txt の読み取りだけが文字コード宣言を迂回して UTF-8 決め打ちになっている。その結果、Shift_JIS のゴーストを入れると ⑴ SHIORI の応答が読めず挨拶も何も出ない（明示エラー）、⑵ areka が送る日本語を SHIORI が化けたまま処理する（黙って壊れる）、⑶ シェルが起動しない（明示エラー）の 3 つが利用者に見える。

本仕様はこの残り半分を ukadoc の規則どおりに閉じる。**主語は「任意の文字コード」**であり、Shift_JIS だけの分岐を足す仕様ではない。対応集合はファイル層が既に解決している集合（WHATWG Encoding Standard の全ラベル: UTF-8・Shift_JIS・EUC-JP・ISO-2022-JP・EUC-KR・GBK／gb18030・Big5・windows-125x・ISO-8859-x・KOI8 等）と同一とし、通信層をその同じ基盤に載せる。未宣言時の既定を Shift_JIS に固定写像する規則（ファイル層と同じ・OS のロケール設定は読まない）は**既定値**であって**対応範囲**ではない。

正典の引用（ukadoc）:

- `Charset`（SHIORI/3.0 の要求・応答の双方）: 「文字コード。最初の行、または少なくとも文字コードがASCII範囲以外の行の前が望ましい。」（https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#Charset:1 ／ #Charset:2）
- `shiori.encoding,文字コード`: 「SHIORIとの通信を指定した文字コードで行う。SHIORI側からCharsetヘッダが返された場合はSHIORI側が優先される。」（https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#shiori.encoding_2c_6587_5b57_30b3_30fc_30c9:1）
- `shiori.forceencoding,文字コード`: 「SHIORIがCharsetヘッダを返したか否かに関係なく、SHIORIとの通信を指定した文字コードで強制的に行う。」（https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#shiori.forceencoding_2c_6587_5b57_30b3_30fc_30c9:1）
- surfaces.txt の `charset,文字コード`: 「表示する文字コード。旧い環境との互換性を考慮する場合はShift_JIS、それ以外はUTF-8を推奨。SSPにおいて複数のsurfaces\*\*\*.txtを用意した場合はそれぞれのファイルに設定する必要がある（逆に言うとそれぞれのファイルで異なる設定が可能）。」（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#charset_2c_6587_5b57_30b3_30fc_30c9:1）

正典が沈黙している点（`shiori.encoding` も `shiori.forceencoding` も無いときの初期値、`Charset` ヘッダが解決できないときの扱い、変換できない文字の扱い、不正なバイト並びの扱い）は、参照実装の実測を輸入せず、本書で areka の裁定として明示する（Requirement 10）。

---

## Boundary Context

- **In scope（利用者から見える範囲）**
  - SHIORI との通信の文字コードを descript の宣言（`shiori.forceencoding` ＞ `shiori.encoding` ＞ 既定）で初期決定し、SHIORI の応答 `Charset` ヘッダで更新する交渉規則。
  - 要求の符号化（`Charset` ヘッダの綴りを含む）と応答の復号を、選ばれた文字コードで行うこと。
  - 対応集合を「ファイル層と同一の任意の文字コード」とすること、および既知の限界（UTF-16 系・replacement 系）の登記。
  - surfaces.txt の本番読み取り 2 経路（起動時・配置採寸時）を、ファイルごとの `charset` 宣言に従う読み取りへ替えること。
  - 文字コードの決定・切替・後退（未知ラベル・変換不能）のログ。
  - 3 系統以上の文字コード＋未知ラベル 1 を往復させる決定論テストと、UTF-8 経路の不変を守るテスト。
  - 正典文書（`doc/COMPAT_ARCHITECTURE.md` §7 の未決項目消し込み・§8 の裁定登記）と ukadoc 網羅台帳の該当行の更新。
  - Shift_JIS のゴースト 1 体での実機確認（ログによる確認。**画面の目視は UTF-8 ゴースト emo2 で行う**——理由は下の Out of scope の末尾）。

- **Out of scope（本仕様が持たない範囲）**
  - SHIORI/4 in-proc 経路（UTF-16 文字列を直接渡す経路）の文字コード交渉。この経路は UTF-8 固定のまま **0 バイトも変えない**。
  - SSTP／FMO／`updates2.dau`／`install.txt`／`readme.charset`／`surfacetable.txt` の文字コード（各 M2 spec が同じ基盤を再利用する）。
  - `surfaces.txt` 以外の `surfaces***.txt` を読む経路の新設（現状 areka が読むのは `surfaces.txt` 1 枚であり、複数ファイルの読み取りは資産台帳の別項目）。
  - x64 SHIORI/3 DLL の in-proc ロード、SAORI（SHIORI 内部の事柄）。
  - OS のロケール設定を読んで既定を変えること（既定は固定写像のまま）。
  - 32bit helper と IPC（要求・応答のバイト列を意味を持たずに運ぶ側）: **変更 0**。
  - 文字集合の描画（フォント・字形）。
  - シェルの暗黙の基準画像（`surfaceNNNN.png` というファイル名の慣習で面の基準画像を決める正典規則）の実装。里々の標準テンプレートはこの書き方を採るため、**本仕様の着地後もこの検体は立ち絵もバルーンも出ない**（文字コードの経路は最後まで正常に働く。実測は `verification/signoff-record.md` 8 節）。別の層の欠陥であり、引受先は `areka-P0-shell-implicit-surface`（2026-09-13 起票）。

- **Adjacent expectations（隣接する仕様・運用への期待）**
  - 完了仕様 `areka-P0-parser-foundation` のファイル層 charset 解決（`charset,<名前>` 宣言の抽出・ラベル解決・既定への寛容フォールバック）が、本仕様の対応集合とラベル解決規則の正本である。本仕様はその基盤を通信層でも使い、別のラベル表を持たない。
  - 完了仕様 `areka-P0-host32-request` は要件 1.7 で「文字コード切替の拡張シームのみ備える」と定め、要件 2.6 で「応答の `Charset` 省略時は要求側の文字コードを継承する」と定めた。本仕様は前者のシームを実装し、後者を維持する。完了仕様の文書は改訂しない。
  - 完了仕様 `areka-P0-shiori-protocol` 要件 8.2 は「レガシー wire の charset 符号化は host-32 の責務」と委譲境界を定めている。本仕様はその責務を実装する側であり、正準 content（UTF-16）側に文字コード概念を持ち込まない。
  - 完了仕様 `areka-P0-ukadoc-survey-shiori`／`-assets` の台帳は `Charset`（要求側）・`shiori.encoding`・`shiori.forceencoding`・surfaces.txt の `charset` の各行の引受先を本仕様と登記済みである。本仕様の着地でそれらを「実装済み」へ更新し、台帳の常設検査を緑に保つ。同仕様が SHIORI/3.0 の組立・解析コードに置いた ukadoc URL コメント（9 行）は、書き換え後も対応する書き出し文の直上に残す（後着が合わせる）。
  - 完了仕様 `areka-P0-emo2-conformance-e2e` が固定した UTF-8 ゴースト（emo2）の実機・決定論の適合結果を、本仕様の前後で変えない。
  - 同じウェーブで同じクレートの別ファイルを触る `areka-P0-host32-window-thread-pump`（メッセージ窓側）とは共有ファイル 0。`areka-P0-property-query-channels` とは ghost runtime の sink 列を共有するが別ウェーブ。
  - 1,000 行番人（ファイル行数の上限検査）の例外表は増やさない。descript 解決モジュールは既に 959 行であり、キー 2 つの追加で上限を超えないよう配置を選ぶのは設計の責務。

---

## Requirements

### Requirement 1: 対応する文字コードの集合

**Objective:** As a UTF-8 以外の文字コードで書かれたゴーストの利用者, I want areka がファイル層と同じ集合の任意の文字コードで SHIORI と話せること, so that Shift_JIS に限らずどの文字コードのゴーストでも同じ規則で動く

#### Acceptance Criteria

1. The SHIORI 通信層 shall 対応する文字コードの集合を、ファイル層の `charset,<名前>` 宣言が解決できる集合（WHATWG Encoding Standard の全ラベル）と同一とし、通信層だけの独自の一覧を持たない。
2. When 文字コードのラベル（descript の宣言値または応答の `Charset` ヘッダ値）を解決する, the SHIORI 通信層 shall 大文字小文字の違い・前後の空白・同一文字コードの別名（例: `shift_jis`／`Shift-JIS`／`sjis`／`windows-31j` は同じ Shift_JIS）を許容し、同じ文字コードへ解決する。
3. While descript に `shiori.encoding` も `shiori.forceencoding` も無い, the SHIORI 通信層 shall 初期の文字コードを、ゴースト起動時にファイル層へ渡される既定と同じ固定写像で決める（本番の既定は ANSI＝Shift_JIS・UTF-8 既定を渡す起動では UTF-8・OS のロケール設定は読まない・読む箇所は 0）。
4. The SHIORI 通信層 shall 上記 3 の既定を「既定値」として扱い、「対応範囲」として扱わない——既定が Shift_JIS であることを理由に、Shift_JIS と UTF-8 以外の文字コードを特別扱い（拒否・縮退）しない。
5. The SHIORI 通信層 shall 符号化の出力が要求したラベルと一致しない文字コード（UTF-16LE／UTF-16BE／UTF-16、および Encoding Standard が「replacement」に写すラベル群）を「通信では使えない既知の限界」として扱い、その扱い（Requirement 10.3 の裁定）を正典文書へ登記する。

### Requirement 2: 初期の文字コードの決定（descript の宣言）

**Objective:** As a ゴースト作者, I want descript に書いた `shiori.encoding`／`shiori.forceencoding` が ukadoc どおりに効くこと, so that SSP 向けに書いた宣言をそのまま areka でも使える

#### Acceptance Criteria

1. When ゴーストが起動し SHIORI との通信を始める, the ゴースト起動処理 shall 初期の文字コードを `shiori.forceencoding` ＞ `shiori.encoding` ＞ 既定（Requirement 1.3）の優先順で決める。
2. While `shiori.forceencoding` が解決できる文字コードで宣言されている, the SHIORI 通信層 shall その文字コードで要求の符号化と応答の復号を行い、SHIORI が返す `Charset` ヘッダの有無・値に関係なく変更しない（ukadoc「SHIORIがCharsetヘッダを返したか否かに関係なく…強制的に行う」）。
3. While `shiori.encoding` のみが解決できる文字コードで宣言されている, the SHIORI 通信層 shall その文字コードを初期値とし、以後は Requirement 4 の交渉規則に従って SHIORI 側の宣言を優先する（ukadoc「SHIORI側からCharsetヘッダが返された場合はSHIORI側が優先される」）。
4. If `shiori.forceencoding` または `shiori.encoding` の値が解決できないラベルである, then the ゴースト起動処理 shall 警告ログ 1 行（キー名・ラベル・採用した後退先）を出し、その宣言を無視して次の優先順（`shiori.encoding` → 既定）へ後退し、起動を続ける。`shiori.forceencoding` が解決できず後退したときは強制の効力も失われ、以後は Requirement 4 の交渉規則（SHIORI 側の宣言が優先）に従う。
5. The ゴースト起動処理 shall descript の `charset` キー（そのファイル自身の文字コード）を SHIORI 通信の初期値として用いない（用いる箇所は 0）。
6. When 初期の文字コードが決まる, the ゴースト起動処理 shall 決定した文字コードの正規名と決定根拠（`forceencoding`／`encoding`／既定のいずれか）を情報ログ 1 行で記録する。
7. While descript が `shiori.encoding` も `shiori.forceencoding` も宣言せず、SHIORI が応答に `Charset: UTF-8` を返す（emo2 の pasta はこの形）, the SHIORI 通信層 shall 採用が起きるまでは既定の Shift_JIS で要求を送り、最初の**応答待ちイベント**の応答の `Charset: UTF-8` を受けて以降の要求を UTF-8 で送る。既定の Shift_JIS を名乗る要求は **2 本**であり（⑴ `OnInitialize`＝片道のイベントで、Requirement 5.3 によりその応答は採用の根拠にしない ⑵ `username` 照会＝応答待ちのイベントで、この応答で採用が起きる）、UTF-8 になるのは 3 本目から。この 2 本で本仕様前と異なるのは `Charset` ヘッダの値だけであり（本文が ASCII のみのとき——emo2 では実測済み: 最初の片道イベント `OnInitialize` と最初の応答待ちイベント `username` 照会はともに References を持たず、`Sender`／`Status` の値は ASCII の語彙。pasta は要求の `Charset` 値を検査しない）、この既知の差を正典文書に登記する。UTF-8 の SHIORI で最初の要求から UTF-8 を用いたいゴーストは `shiori.encoding,UTF-8` を宣言する（ukadoc どおり）。

### Requirement 3: 要求の符号化

**Objective:** As a Shift_JIS 想定の SHIORI を持つゴーストの利用者, I want areka が送る日本語（利用者の入力・ファイルのパス等）が SHIORI に正しく届くこと, so that OnCommunicate や OnFileDrop が黙って化けない

#### Acceptance Criteria

1. When 要求を組み立てる, the SHIORI 通信層 shall 現在の文字コードでヘッダ値（イベント名・`Reference*`・`Sender`・`Status` を含む全行）を符号化したバイト列を送る。
2. The SHIORI 通信層 shall `Charset` ヘッダの値を、その文字コードの正規名（例: `UTF-8`／`Shift_JIS`／`EUC-JP`／`ISO-2022-JP`＝ukadoc 表記と一致する綴り）で書き、実際に符号化した文字コードと常に一致させる。
3. The SHIORI 通信層 shall `Charset` ヘッダを要求行の直後の最初のヘッダ行に置き、ASCII 範囲外の文字を含み得る行より前に位置させる（ukadoc「最初の行、または少なくとも文字コードがASCII範囲以外の行の前が望ましい」）。
4. While 現在の文字コードが UTF-8 である（宣言による初期値・応答による採用後のいずれも）, the SHIORI 通信層 shall 本仕様適用前と**バイト単位で同一**の要求を送る（差分 0 バイト）。
5. If 要求に現在の文字コードで表せない文字が含まれる, then the SHIORI 通信層 shall 要求全体を失敗させず、Requirement 10.1 の裁定に従う代替表記に置換して送り、警告ログ 1 行（イベント名・置換した文字数）を出す。
6. The SHIORI 通信層 shall 符号化したバイト列を 32bit helper と IPC へ**そのまま**渡し、helper・IPC 側にバイト列の意味を解釈する変更を加えない（変更 0）。

### Requirement 4: 応答の復号と文字コードの交渉

**Objective:** As a UTF-8 以外の文字コードで応答する SHIORI を持つゴーストの利用者, I want SHIORI の返事が正しく読まれ、以後の通信もその文字コードで続くこと, so that 挨拶から会話まで文字化けせずに進む

#### Acceptance Criteria

1. When 応答を受け取る, the SHIORI 通信層 shall 本文を復号するより先に、応答の `Charset` ヘッダをヘッダ名（ASCII）の一致で読み取る。ヘッダの位置は問わない（ukadoc は「望ましい」位置を示すのみ）。
2. When 応答の `Charset` ヘッダが解決できる文字コードを示し、かつ `shiori.forceencoding` が無い, the SHIORI 通信層 shall その文字コードで応答全体を復号し、以後の要求の文字コードとしてそれを採用する（SHIORI 側の宣言が優先）。
3. When 応答の `Charset` ヘッダが省略されている, the SHIORI 通信層 shall 要求に用いた文字コードを継承して復号する（完了仕様 `areka-P0-host32-request` 要件 2.6 を維持）。
4. If 応答の `Charset` ヘッダが解決できないラベルである, then the SHIORI 通信層 shall 要求に用いた文字コードで復号を続け、そのラベルを以後の文字コードとして採用せず、同じラベルにつき初回は警告ログ 1 行、2 回目以降は開発者向け詳細ログとする（毎秒のイベントで警告が氾濫しない）。
5. While `shiori.forceencoding` が有効である, the SHIORI 通信層 shall 応答の `Charset` ヘッダを復号にも採用にも用いず、強制された文字コードと異なるヘッダを初めて受け取ったときに開発者向け詳細ログ 1 行を出す。
6. When 採用する文字コードが直前の要求と異なる値に変わる, the SHIORI 通信層 shall 切替 1 回につき開発者向け詳細ログ 1 行（旧→新の正規名）を出し、変わらない応答ではログを出さない。
7. If 応答のバイト列に、宣言された文字コードとして不正な並びが含まれる, then the SHIORI 通信層 shall Requirement 10.2 の裁定に従って扱い、いずれの裁定でも記録なしに握り潰さない。
8. The SHIORI 通信層 shall ステータス行・ヘッダ名・ステータスコードの解析、および 200／204／311／312／400／500 の扱いを本仕様の前後で変えない（UTF-8 の応答に対する既存テストは、期待値を変えずに緑。呼び出し形の機械的な追随は可）。**例外 1 本**: 不正な UTF-8 バイト列に解析エラーを期待する既存テスト（応答解析の「不正 UTF-8 は解析エラー」）は、Requirement 10.2 の裁定 (a) に従い「成功＋代替文字＋警告ログ 1 行」へ期待値を更新する（例外はこの 1 本のみ・他の既存テストの期待値変更 0）。

### Requirement 5: 交渉状態の持続と適用範囲

**Objective:** As a ゴーストの利用者, I want 一度決まった文字コードがゴーストのセッションを通じて保たれること, so that イベントごとに文字化けしたりしなかったりしない

#### Acceptance Criteria

1. The SHIORI 通信層 shall 採用中の文字コードをゴーストのセッション（SHIORI の load から unload まで）を通じて保持し、応答待ちのイベント（GET）と片道のイベント（NOTIFY）の双方の要求に同じ文字コードを用いる。
2. When ゴーストが起動する（SHIORI を load する）, the SHIORI 通信層 shall 文字コードを Requirement 2 の初期値へ戻し、前回セッションの採用結果を引き継がない。
3. The SHIORI 通信層 shall 片道のイベント（NOTIFY）の応答を従来どおり破棄し、その `Charset` ヘッダを採用の根拠に用いない（NOTIFY 応答からの採用 0・完了仕様 `areka-P0-host32-request` 要件 4.8 を維持）。
4. While SHIORI/4 in-proc 経路（UTF-16 文字列を直接渡す経路）が選ばれている, the SHIORI 通信層 shall 要求の組立と応答の解析を UTF-8 固定のまま行い、交渉を行わず、この経路の挙動を本仕様の前後で変えない（挙動の変更 0。この経路は同じ組立・解析関数を再利用しているため、文字コードを表す型の変更に伴う呼び出し形の機械的な追随は Requirement 4.8 と同じく可）。

### Requirement 6: surfaces.txt の文字コード

**Objective:** As a Shift_JIS で書かれたシェルの利用者, I want surfaces.txt がファイルの `charset` 宣言どおりに読まれること, so that シェルが起動に失敗しない

#### Acceptance Criteria

1. When シェルの surfaces.txt を読む（起動時・配置採寸時の両経路）, the シェル読込処理 shall ファイル層と同じ規則（冒頭の `charset,<名前>` 宣言を優先・未宣言時は既定 Shift_JIS・解決できないラベルは既定へ後退）で復号してから解析する。
2. The シェル読込処理 shall 文字コードの宣言をファイルごとに解釈し、他のファイル（descript.txt 等）の宣言を surfaces.txt へ適用しない。
3. While surfaces.txt が `charset,UTF-8` を宣言している（emo2 を含む）, the シェル読込処理 shall 本仕様適用前と同一の解析結果を産む（UTF-8 経路の差分 0）。
4. If surfaces.txt が存在しない、または読み取れない, then the シェル読込処理 shall 従来どおり明示エラー（起動失敗・配置採寸失敗）として扱い、エラーログを出す。
5. If surfaces.txt に宣言された文字コードとして不正なバイト並びが含まれる, then the シェル読込処理 shall ファイル層と同じく代替文字で吸収して解析を続け、開発者向け詳細ログ 1 行を出す（起動失敗にしない）。
6. The シェル読込処理 shall テスト・サンプル専用の surfaces.txt 読み取り（emo2 の UTF-8 固定物を読む箇所）を変更しない（変更 0）。

### Requirement 7: ログと失敗経路

**Objective:** As a 運用者・ゴースト作者, I want 文字コードの決定・切替・後退がログから追えること, so that 文字化けの原因を黙って壊れる前に突き止められる

#### Acceptance Criteria

1. The SHIORI 通信層 shall 文字コードに関する後退（解決できないラベル・変換できない文字・不正なバイト並び）のすべてに対応するログ行を持ち、ログなしに既定へ倒す経路を持たない（ログなしの後退 0）。
2. The SHIORI 通信層 shall 文字コードの決定・切替・後退のログに、文字コードの正規名と、後退の場合はその根拠（受け取ったラベル・置換した文字数等）を構造化フィールドで含める。
3. The SHIORI 通信層 shall どのような入力（任意のバイト列・任意のラベル）に対しても panic せず、失敗は呼び手へ返す値として表す。
4. When 定常運転中（毎秒のイベントを含む）に同じ内容の後退が繰り返される, the SHIORI 通信層 shall 警告レベルのログを同じ内容につき 1 回に抑え、以後は開発者向け詳細ログとする。

### Requirement 8: 正典文書と台帳の追随

**Objective:** As a 後続の M2 spec の担当者, I want 本仕様の裁定と着地が正典文書と台帳に反映されていること, so that SSTP／NAR など同じ基盤を再利用する仕様が同じ規則を引ける

#### Acceptance Criteria

1. When 本仕様が着地する, the 正典文書 shall `doc/COMPAT_ARCHITECTURE.md` §8 に、未宣言時の既定を Shift_JIS へ固定写像する裁定（OS ロケール不読）、Requirement 10 の各裁定の結果、NOTIFY 応答から採用しない裁定を、出典 spec 付きの行として登記する。
2. When 本仕様が着地する, the 正典文書 shall `doc/COMPAT_ARCHITECTURE.md` §7 の未決項目「Charset交渉の具体」を消し込む（解決済みとして §8 へ導く）。
3. When 本仕様が着地する, the ukadoc 網羅台帳 shall `Charset`（要求側・応答側の 2 行）・`shiori.encoding`・`shiori.forceencoding`・surfaces.txt の `charset` の計 5 行を「実装済み」へ更新し、台帳の常設検査（`cargo test -p ukadoc-survey`）が緑であること。
4. The 実装 shall 完了仕様 `areka-P0-ukadoc-survey-shiori` が SHIORI/3.0 の組立・解析コードに置いた ukadoc URL コメント 9 行を、書き換え後も対応する書き出し文の直上に保つ（削除 0）。
5. The 実装 shall 新たに読む descript キー（`shiori.encoding`／`shiori.forceencoding`）と surfaces.txt の `charset` を読む箇所に、台帳の慣行どおり ukadoc の URL コメントを置く。

### Requirement 9: 決定論テスト

**Objective:** As a 開発者, I want 実装が「Shift_JIS 分岐」に退化したら赤になる決定論テストがあること, so that 任意の文字コードへの対応が言葉だけでなく検査で保たれる

#### Acceptance Criteria

1. The 決定論テスト shall 要求の符号化を少なくとも UTF-8・Shift_JIS・EUC-JP の 3 系統で固定し、期待バイト列を**符号化器から導かず定数として**書く（例: 「あ」＝UTF-8 `E3 81 82`／Shift_JIS `82 A0`／EUC-JP `A4 A2`）。
2. The 決定論テスト shall 応答の復号を同じ 3 系統で固定し、`Charset` ヘッダが宣言した文字コードで `Value` が正しく読めることを示す。
3. The 決定論テスト shall 応答の `Charset` 省略時の継承、解決できないラベルへの後退（警告ログの観測を含む）、`shiori.forceencoding` 下での応答 `Charset` の無視、応答による採用が**次の要求**の `Charset` ヘッダとバイト列に現れること、同一文字コードの別名が同じ文字コードへ解決することを、それぞれ固定する。
4. The 決定論テスト shall descript の 2 キーの優先順（`forceencoding` ＞ `encoding` ＞ 既定）と、解決できない値の後退（警告ログの観測を含む）を固定する。
5. The 決定論テスト shall surfaces.txt について、`charset,Shift_JIS` を宣言した Shift_JIS のバイト列、宣言なしの Shift_JIS のバイト列（既定で読める）、`charset,EUC-JP` を宣言した EUC-JP のバイト列の各固定物が、同内容の UTF-8 固定物と同一の解析結果を産むことを固定する。
6. The 決定論テスト shall UTF-8 の要求バイト列を本仕様適用前と同一の定数と比較して固定し、完了仕様 `areka-P0-emo2-conformance-e2e` の決定論テスト群と emo2 の固定物（ゴースト・シェル・辞書）を無改変のまま緑に保つ。
7. The 決定論テスト shall 3 系統のうち EUC-JP のように「Shift_JIS だけを特別扱いする実装では通らないバイト列」を必ず含め、Shift_JIS 実装を外した変異と UTF-8／Shift_JIS 以外を落とした変異の双方で赤になる形とする。
8. The 決定論テスト shall 32bit helper・実 SHIORI DLL・実機を要さず、純粋な関数の入出力とログの観測だけで成立する（x86 ビルドへの依存 0）。

### Requirement 10: 正典が沈黙する点の裁定（要件段階で決める）

**Objective:** As a 開発者, I want 正典が定めていない 3 点を推測で固定せず、明示の裁定として決めること, so that 参照実装の実測を輸入せずに areka の規則を説明できる

各項目は選択肢と推奨既定を示す。要件ディスカッションで確定した結果を本項に書き戻し、Requirement 8.1 の登記へ写す。

#### Acceptance Criteria

1. If 要求に現在の文字コードで表せない文字（例: Shift_JIS への絵文字）が含まれる, then the SHIORI 通信層 shall 文字ごとに `&#数値;` 形式（10 進の数値文字参照）へ置換して要求を送る（**裁定項目 ⑴ 変換できない文字＝確定 (a)**。2026-09-11 要件ディスカッションで開発者が裁定。要求を失敗させず、SHIORI 側が元の文字を復元できる。符号化器の既定動作と一致。退けた案: (b) `?` への置換＝情報を失う・(c) 要求全体の失敗＝文字 1 つで会話イベントが消える）。Requirement 3.5 の警告ログを伴う。
2. If 応答に宣言された文字コードとして不正な並びが含まれる, then the SHIORI 通信層 shall 不正な箇所を代替文字（U+FFFD）で吸収して解析を続行し、警告ログ 1 行を出す（**裁定項目 ⑵ 不正なバイト並び＝確定 (a)**。2026-09-11 要件ディスカッションで開発者が裁定。ファイル層と同じ寛容。1 バイトの乱れで挨拶全体が消えるより、化けた 1 文字が見える方が利用者にとって原因を追いやすく、ログで記録も残る。退けた案: (b) 解析失敗として応答全体を捨てる＝現行の UTF-8 経路の即時失敗を全文字コードへ広げる）。この裁定は UTF-8 の応答にも適用され、既存テスト 1 本の期待値更新を Requirement 4.8 の例外として認める。
3. If descript の宣言または応答の `Charset` が UTF-16LE／UTF-16BE／UTF-16、または replacement に写るラベルを示す, then the SHIORI 通信層 shall 「解決できないラベル」と同じ扱い（Requirement 2.4／4.4 の経路＝警告ログ＋現在の文字コードを継続・採用しない）とする（**裁定項目 ⑶ 通信で使えない既知の限界＝確定 (a)**。要件ディスカッションで確定: 起動を止める案 (b) は、実資産にほぼ存在しないラベルのために利用者の起動を止め、かつ解決できないラベルの既存経路と別の失敗経路を増やすため採らない。`Charset` ヘッダの綴りと実際のバイト列が食い違う要求は決して送らない。ログでは「解決できない」と「通信で使えない既知の限界」を根拠フィールドで区別してよい）。

### Requirement 11: 実機確認

**Objective:** As a 開発者, I want Shift_JIS の実ゴースト 1 体で挨拶が化けずに出ることを確認すること, so that 決定論テストが隠す欠陥（実 SHIORI の応答形・実ファイルの癖）を炙り出せる

#### Acceptance Criteria

1. When 里々の標準テンプレート（Shift_JIS・32bit の SHIORI DLL。2026-09-11 要件ディスカッションで開発者が検体として確定。配置先の絶対パスは実装フェーズで開発者が指定する——相対パスでは DLL のロードが失敗する）を areka で起動する, the areka shall 文字コードの経路が最後まで正常に働いたことをログで示す（初期値の記録が 1 行出ること・応答による採用の有無が期待どおりであること）。**バルーンへの表示の目視は本仕様では求めない**——この検体は areka にシェルの暗黙の基準画像（`surfaceNNNN.png` というファイル名の慣習）が実装されていないために立ち絵もバルーンも出ず、これは文字コードとは別の層の欠陥だからである（2026-09-13 開発者裁定。障害の実測は `verification/signoff-record.md` 8 節、引受先は `areka-P0-shell-implicit-surface`）。**目視は Acceptance Criteria 3 の UTF-8 ゴースト（emo2）で行う。**
2. The 実機確認 shall 有界の自動終了とログの検索で行い、初期の文字コードの情報ログ 1 行と、応答による採用（該当する場合）の詳細ログ 1 行がログに現れることを確認する。
3. The 実機確認 shall UTF-8 のゴースト（emo2）でも同じ手順を通し、挨拶の表示とログ（初期＝既定 Shift_JIS → 応答 `Charset: UTF-8` による採用 1 回、または `shiori.encoding` 宣言に応じた初期値）が期待どおりであることを確認する。

### Requirement 12: 変えないもの（非機能・境界）

**Objective:** As a 開発者, I want 本仕様が触ってよい範囲と触ってはならない範囲が検査で守られること, so that M1 の適合と凍結済みの層が壊れない

#### Acceptance Criteria

1. The 実装 shall UTF-8 のゴーストの要求バイト列（Requirement 2.7 の最初の 2 本の要求の `Charset` ヘッダ値を除く）・応答の解析結果・surfaces.txt の解析結果を本仕様の前後で同一に保ち、emo2 の固定物に `shiori.encoding` を足す等の改変をしない（差分 0・Requirement 9.6 で固定）。
2. The 実装 shall 32bit helper と IPC のコードを変更しない（挙動・バイト列に関わる変更 0）。ただし本仕様の着地で事実と食い違う説明コメント（helper の「`request` は UTF-8」）は、その 1 行の文言だけを「任意の文字コードのバイト列」へ追随させてよい（古くなった説明を残さない）。
3. The 実装 shall 完了仕様（`areka-P0-host32-request`・`areka-P0-shiori-protocol`・`areka-P0-parser-foundation`・`areka-P0-ukadoc-survey-*`）の文書を改訂しない。
4. The 実装 shall 1,000 行番人の例外表を変更せず、触るファイルを 1,000 行未満に保つ。
5. The 実装 shall 既存の公開ログ行の語彙（起動・終了・エラーの既存メッセージ）を変えず、本仕様のログ行を追加のみとする。

---

## 付録 A: 着手時の再検証（2026-09-11・brief「Current State」との突合）

brief（2026-09-02）の表を実コードで引き直した結果。引用は「何の定義行か」で示す。

| brief の記述 | 再検証結果 |
|---|---|
| ファイル層は任意 charset（`charset/{prescan,decode,model}.rs`） | **一致**。`decode(bytes, default)` が冒頭 ASCII 窓の `charset,<名前>` → ラベル解決 → 既定へ寛容フォールバック → 不正並びは代替文字＋詳細ログ。`DefaultEncoding::to_encoding` が `Ansi→Shift_JIS`／`Utf8→UTF-8` の固定写像。本番の呼び出しは `boot_config.rs` の `default_encoding: Ansi`、`emo2_boot/assets.rs` の `resolve(ghost_root, Ansi)` と balloon 読取、`placement/source.rs` の `resolve`／2 箇所の `decode(.., Ansi)`（brief の `:138` は現在 `resolve` 呼出行・`decode` 呼出行 2 つに分かれている）、`areka-emo-present/src/balloon.rs` の balloon 読取 |
| surfaces.txt が 2 箇所で decode を迂回 | **一致**。`emo2_boot/assets.rs` の「シェル: surfaces.txt 読取 → parse → bake を 1 回」の `read_to_string` と、`placement/measure.rs` `build_shell_assets` の `read_to_string`。shell の decode 層は `charset` 行を寛容スキップ（`shell/decode.rs` 冒頭コメント「charset 行の寛容スキップ（要件 3.1）」）。テスト側の同種読取（`areka-emo-compose/src/world.rs`・`areka-seriko/src/resolve.rs`）は UTF-8 固定物専用で変更不要 |
| SHIORI/3 プロトコル層は UTF-8 固定 | **一致**。`shiori3.rs` の `enum Charset { Utf8 }`（`ShiftJis` はコメントのシームのみ）、`build_request` の `Charset:` 書き出しが `header_value()`＝`"UTF-8"` 固定、`parse_response` が `from_utf8` で即時失敗し `request_charset` を `match` で消費するだけ、ヘッダ走査の末尾コメント「Charset / Reference0 / Marker / 未知ヘッダ等は読み飛ばす」。`client.rs` の `get`／`notify` が `Charset::Utf8` 固定で組立・解析 |
| `shiori.encoding`／`shiori.forceencoding` 未解析 | **一致**。`package/model.rs` の `ShioriMount` は `dir`／`file` のみ。`resolve.rs` の `resolve` は `parse_kv` の全キー保持マップから `shiori` を取り出しており、2 キーの追加は容易。ただし **`resolve.rs` は 959 行**で 1,000 行番人に近い（配置は設計で選ぶ） |
| SHIORI/4 in-proc は「charset 概念なし」 | **要訂正（ズレ）**。`areka-ghost/src/shiori_inproc.rs` は同じ codec を**再利用**している——`build_input` が `build_request(.. charset: Charset::Utf8)` の出力を UTF-16 文字列へ写し、`map_get_outcome` が `parse_response(response_bytes, Charset::Utf8)` で解析する。したがって `Charset` 型の変更は in-proc 経路の呼出点 2 つに波及する。要件としては「UTF-8 固定のまま変更 0」（Requirement 5.4）を明記した |
| 交渉状態は `Shiori3Client` のセッション状態 | **要注意（ズレ）**。`areka-kanade/src/shiori/real.rs` の `ShioriConnection::get`／`notify` は**呼び出しごとに** `Shiori3Client::new(&self.window)` を作り捨てている。client にだけ状態を持たせるとイベントごとに失われる。要件は「セッションを通じて保持」（Requirement 5.1）とし、置き場所は設計の責務 |
| `areka-ghost` runtime／`shiori_wiring` の初期 charset 配線 | **一致（未配線）**。`shiori_wiring.rs` に charset の語は無い。`GhostBootOptions.default_encoding`（ファイル層の既定）はあるが SHIORI 通信の文字コードは渡っていない |
| 32bit helper は素通し | **一致**。`shiori-host32-helper/src/shiori_proxy.rs` 冒頭「意味を持たない生バイト列を FFI 境界の向こうへ運ぶだけ」「`request` は UTF-8（下流・本仕様非呼出）」——後者のコメント文言は本仕様着地後に古くなる（bytes は任意 charset）。コード変更は 0 のままコメントのみ追随してよいかは設計で決める |
| `ukadoc-survey-shiori` と同じ `shiori3.rs` を触る＝後着が rebase | **確定**。survey が先に着地済み（PR#139）。`shiori3.rs` に `// ukadoc:` コメントが **9 行**あり、本仕様が後着として位置を合わせる（Requirement 8.4） |
| 台帳行は本 spec 着地で `implemented` へ | **確定・具体化**。`ledger/shiori.toml` の `spec_shiori3:Charset:1`（要求側）は `degraded`・`owner = "areka-P0-charset-canon"`、`ledger/assets.toml` の `shiori.encoding`／`shiori.forceencoding` は `absent`・同 owner、`descript_shell_surfaces:charset` は「未対応」。応答側 `spec_shiori3:Charset:2` は `absent`・owner 空（本仕様の着地で応答ヘッダを読むため、この行も更新対象に含める） |
| `doc/COMPAT_ARCHITECTURE.md:85`／`:117` | **一致**。§5 過去互換経路の「64bit areka側で早期に HSTRING(UTF-16) → Charsetヘッダ解析 → charset符号化バイト列」、§7 未決の「Charset交渉の具体」 |
| UTF-8 経路は 1 バイトも変えない（emo2 e2e 全緑） | **要精密化（ズレ）**。emo2 の ghost descript（`fixtures/emo2/ghost/master/descript.txt`）は `charset,UTF-8` のみで `shiori.encoding` を宣言しない。既定 Shift_JIS の初期値をそのまま適用すると、最初の 2 本の要求の `Charset` ヘッダ値だけが `UTF-8` から `Shift_JIS` へ変わる（本文は ASCII のみ）。pasta は全応答に `Charset: UTF-8` を書き（`pasta_shiori/src/shiori.rs` の応答組立 2 箇所と `error.rs` のエラー応答）、要求は UTF-8 前提で読む（`lua_request.rs` の「req.charset: utf-8であること」）ため、2 本目（応答待ちイベント）の応答で UTF-8 を採用して以後は不変。既存の scripted fake（`ShioriWiring::Custom`）は codec を通らず、host32 テスト DLL の 200 応答は `Charset: UTF-8` を含む。要件は「採用後は不変・最初の 2 本の要求はヘッダ値のみ・固定物は改変しない」と精密化した（Requirement 2.7／3.4／12.1） |
| encoding 基盤の UTF-16 限界 | **確定**。ワークスペースの `encoding_rs` 0.8.35 は「UTF-16LE／UTF-16BE の符号化器を提供しない（replacement と共に出力符号化は UTF-8）」「符号化器の誤り回復は数値文字参照のみ」「復号器の誤り回復は代替文字のみ」と明記。`for_label` は前後空白と大小文字に寛容 |

## 付録 B: 影響する既存挙動の一覧（変えるもの／変えないもの）

- 変えるもの: 要求の `Charset` ヘッダ値と本文バイト列（UTF-8 以外のとき）、応答の復号に使う文字コード、応答 `Charset` の読取と採用、descript の 2 キーの読取、surfaces.txt 本番読取 2 経路の復号、ログ行の追加、正典文書 §7／§8、台帳 5 行。
- 変えないもの（0）: UTF-8 経路のバイト列と解析結果、32bit helper・IPC、SHIORI/4 in-proc 経路の挙動、NOTIFY 応答の破棄、ステータス行・ヘッダ名の解析規則、テスト専用の surfaces.txt 読取、OS ロケールの読取、完了仕様の文書、1,000 行番人の例外表。

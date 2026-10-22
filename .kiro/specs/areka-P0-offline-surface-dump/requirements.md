# Requirements Document

## Project Description (Input)
シェルの作者・開発者・AI のエージェントが、シェルの絵（surfaces.txt・element・着せ替え）を確かめるとき、今の areka では、areka を立てて窓を出し、ゴーストを起こし、MCP の `dump_surface` で面を 1 枚ずつ撮るしかない。窓・GPU・SHIORI・32bit の SHIORI のための補助がそろわないと撮れず、1 枚のために数秒の起動と終了が要り、並んで走るほかの試験とは窓や入力を取り合う。SSP には `--offline-dump` があり、ゴーストを起こさず、窓も作らずに、指定した面を PNG に書き出してすぐ終わる。surfaces.txt の読み込みの誤りだけを CSV に書き出す使い方もあり、終了コードが最も重い誤りの段を返す。

この spec は、areka に SSP と同じ書き方のオフラインの書き出し（`--offline-dump` と `--dump-shell`・`--dump-scope`・`--dump-surface-list`・`--dump-output-dir`・`--dump-output-prefix`・`--dump-error-log`・`--dump-bind`）を足す。窓も GPU も SHIORI も MCP の待受も使わず、areka の記憶にも書かずに、面を α 付きの PNG に書き出し、読み込みの誤りを SSP と同じ形の CSV と終了コードで返す。書き出した絵は、areka を立てて画面に出す絵と、出す段（拡大・透明度）の差を除いて同じになる。v0.1.0 の目標「SSP 並みの自動試験・開発の速さ」の 1 本で、areka と SSP の同じ面の絵を窓を出さずに画素で比べる試験の土台になる。詳細は同じフォルダの `brief.md`。

## Introduction

SSP のヘルプ「起動オプション一覧」の「オフラインサーフェスダンプ」の節（https://ssp.shillest.net/ukadoc/ssphelp/option.html ・2026-10-22 に ukadoc の MCP で読んだ）が、この spec の正典である。正典の要点（引用は正典の「機能」の欄）:

- `--offline-dump フォルダパス`: 「ゴーストを実際に起動することなく、指定したゴーストフォルダのsurfaces.txtを読み込んでサーフェス画像をダンプし、即終了する。SHIORI/MAKOTO/PIROはロードされない。ウィンドウ・メッセージループも作成されない。」引数はゴーストのフォルダ（末尾の `\` は任意）。`--offline-dump`・`--dump-output-dir`・`--dump-error-log` の相対パスは「SSPを起動したときのカレントディレクトリ基準で解決される」。
- `--dump-shell フォルダ名か名前`: 省略時は「descript.txtで指定されているデフォルトシェル」。「指定したシェルが見つからない場合は、デフォルトシェルでダンプせず、エラーログにErrorを記録して何も出力しない。」
- `--dump-scope (スコープID)`: 省略時は 0。
- `--dump-surface-list リスト`: カンマ区切り。「surface番号のプレフィックス（surface）は省略可能」（各要素で）。「surfaces.txtのサーフェスID指定の拡張形式も使用できる。」「このオプションを省略した場合はサーフェスダンプは行われない」。「範囲でなく単独の番号で指定したサーフェスが存在しない場合は、エラーログにWarningを記録する（範囲指定内の欠番は記録しない）。」
- `--dump-output-dir ディレクトリ`: 省略時は今のフォルダ。
- `--dump-output-prefix プレフィックス`: 「既定の「surface」がこの文字列に置き換わる（付け足しではない）」。例: `x_` で面 0 は `x_0.png`。
- `--dump-error-log ファイルパス`: 「BOM付きUTF-8のCSV（改行はCRLF）。1行が1件で、各列は"で囲まれ、列中の"は""にエスケープされる。」列は `"時刻","ゴースト名","レベル","内容"`、時刻は `HH:MM:SS`、レベルは「Info / Notice / Warning / Error / Critical / System のいずれか」。「ログを最大10000件まで保持し、似た内容のログを要約行（レベルSystem）にまとめる処理も行わない」。「ログが1件もない場合はファイルは作成されない（同じパスに既存のファイルがあっても上書き・削除はされない）。」終了コードは「0：ログなし、またはInfo/Noticeのみ 1：Warningあり（Error/Criticalなし） 2：Errorあり（Criticalなし） 3：Criticalあり」。「ゴーストの読み込み自体に失敗した場合（指定フォルダのdescript.txtが読めない等）は、ログは出力されず終了コードも0になる。」
- `--dump-bind 着せ替え指定`: 「「カテゴリ名,パーツ名,ON/OFF」をセミコロン区切りで複数指定できる。意味は\![bind]と同じ」。パーツ名が空か `__ALL__` でカテゴリの全部。「ON/OFFは 1 または true でON、0 または false でOFF。省略時はONとして扱う」。基準は「シェルのdescript.txtで指定された各パーツのデフォルト（sakura.bindgroup*.default）で、ユーザーが最後に着せ替えた状態（shell\profile内の保存値）は使われない」。「着せ替えは --dump-scope で指定したスコープに対して適用される。」「存在しないカテゴリ・パーツを指定した場合はWarningのログが記録され（--dump-error-log の終了コードは1）、その指定だけを無視してダンプは続行される。」「カテゴリ名・パーツ名の前後の空白は取り除かれない」。
- 書き出す絵の時点: `--dump-animation` の説明に「通常のダンプと同じ画像（runonceのアニメーションを1周させた後の状態）」とある。

起動オプションの名前・値の読み方（`--名前 値`・`--名前=値`・大文字と小文字・知らない名前・値の無いとき・重なったとき）は、完了 spec `areka-P0-startup-options-core` の要件 1 がそのまま効く。今は `--offline-dump` と `--dump-` で始まる名前を「受けて、まだ扱わない」（warn を 1 件記録して読み飛ばす）として読んでいる。この spec はそれを効かせる。

### 用語

- **オフラインの書き出し**: `--offline-dump` で始まる、窓を作らずに面を書き出して終わる 1 回の走り。
- **ゴーストのフォルダ**: `ghost\master\descript.txt` と `shell\` を持つ、ゴースト 1 体の根のフォルダ（例: `ghost\emo2`）。
- **既定のシェル**: ゴーストの `descript.txt` が既定として指すシェル（通常は `master`）。利用者が最後に選んだシェルの記憶ではない。
- **スコープ**: 面を出すキャラクターの番号（0＝本体側・1＝相方側・2 以上＝3 人目以降）。
- **既定の着せ替え**: シェルの `descript.txt` の `sakura.bindgroup*.default`・`kero.bindgroup*.default`・`char*.bindgroup*.default` が決める、スコープごとの着せ替えの初めの状態。
- **書き出しの記録**: オフラインの書き出しのあいだに areka が残す warn と error の記録のうち、要件 7.3 が決めるもの。
- **記録**: areka のログ。重さは info（知らせ）・warn（無視した入力・欠けなど）・error（失敗）。

## Boundary Context

- **In scope**:
  - `--offline-dump` で窓も GPU も SHIORI も MCP の待受も使わずに走り、終わること
  - `--dump-shell`・`--dump-scope`・`--dump-surface-list`（surfaces.txt の面の番号の拡張の書き方を含む）・`--dump-output-dir`・`--dump-output-prefix`・`--dump-error-log`・`--dump-bind`
  - 相対パスの読み方（起動したときの今のフォルダから）
  - 書き出す絵の中身（既定の着せ替え・runonce のアニメーションを 1 周させた後・拡大しない・α 付き）と、画面に出す絵との一致
  - 書き出しの記録の CSV（形・1 件も無いときに作らない）と終了コードの段
  - areka の記憶・利用者の置き場に書かないこと
  - PNG の色の戻し方を MCP の `dump_surface` と同じにすること
  - 配布物の説明書 `dist/README.txt` の起動オプションの節
  - 決定論のテスト（面の絵・CSV・終了コード・書かないこと）
  - SSP の `--offline-dump` で書き出した絵と画素で比べる足場（比べる元の絵は SSP で作る。足場は SSP を起こさない）
- **Out of scope**:
  - `--dump-surface-option`（`backlog`・`collision`。当たり判定の描き込みと顔まわりの切り抜き） → 後の 1 本 `offline-dump-overlays`。受けて warn を 1 件記録し、無いものとして書き出しを続ける
  - `--dump-animation`（アニメーションの連番の書き出し） → v0.2.0 の dola の作り直しの後。受けて warn を 1 件記録し、無いものとして書き出しを続ける
  - `--offline-tool`・`--target-dir`・`--output`・`--password`（NAR・更新定義・書庫を作る） → 今までどおり受けて記録だけ
  - バルーンの絵の書き出し（SSP にも無い）
  - シェルの読み方・面の組み立てそのもの（今のまま使う）と、画面に出す段（拡大・透明度・GPU）
  - surfaces.txt の誤りの記録に定義元のファイルと行（SSP の `Line=12:Surface=10` の形）を付けること → `config-parse-diagnostics`（その spec が記録を詳しくすると、CSV の「内容」もそのまま詳しくなる）
  - 動く絵（APNG・動く WebP）を element で動かすこと → `animated-image-import`
- **Adjacent expectations**:
  - 起動オプションの読み方の決まりは `areka-P0-startup-options-core` の要件 1 のまま。この spec は扱う名前を足すだけで、読み方を変えない。
  - シェルを窓なしで読む今の入口と、面を組む今の合成を使う。画面に出す絵も同じ合成から作っているので、オフラインの絵と画面の絵の差は出す段（拡大・透明度）だけになる。
  - MCP の `dump_surface` の答え（PNG の中身）は、この spec の後も変わらない。
  - 下流の `pasta-in-windows-verification` は、要件 13 の足場で SSP と areka の同じ面を比べる。

## Requirements

### Requirement 1: オフラインの書き出しの走り方

**Objective:** As a シェルの作者・試験を書く開発者・AI のエージェント, I want 窓を出さずに 1 行の起動で面を書き出して終わらせたい, so that 窓や入力を取り合わずに、速く何度でも、CI でもシェルの絵を確かめられる

#### Acceptance Criteria

1. When `--offline-dump` にフォルダが渡されたとき, the areka shall ゴーストを起こさず、オフラインの書き出しを行い、終わったら要件 8 の終了コードで終える。
2. While オフラインの書き出しのあいだ, the areka shall 窓を作らず、画面に何も出さず、メッセージボックスも出さない。
3. While オフラインの書き出しのあいだ, the areka shall SHIORI・MAKOTO を読み込まず、32bit の SHIORI のための補助のプロセスも起こさず、ゴーストへ問い合わせを送らない。
4. While オフラインの書き出しのあいだ, the areka shall MCP を待ち受けない。
5. While オフラインの書き出しのあいだ, the areka shall GPU を使わずに面を組み、PNG を書く。
6. When オフラインの書き出しを始めたとき, the areka shall 書き出しの対象（ゴーストのフォルダ・シェル・スコープ・面の指定・書き出し先・CSV の行き先）を info で 1 件記録する。
7. When オフラインの書き出しを終えたとき, the areka shall 書き出した面の数・書き出せなかった面の数・書き出しの記録の最も重いレベル・終了コードを info で 1 件記録する。
8. If `--offline-dump` に値が無い場合, then the areka shall ゴーストを起こさず、窓も作らずに、そのことを error で記録し、要件 2.6 のゴーストの読み込みに失敗したときと同じに終える。
9. While 起動中の areka や SSP があるあいだ, the areka shall それらに処理を任せず、それらへ何も送らずにオフラインの書き出しを行う。

### Requirement 2: ゴーストとシェルの決め方

**Objective:** As a シェルの作者, I want 置き場に入れていないゴーストのフォルダとシェルを直に指したい, so that 作りかけのシェルをそのまま確かめられる

#### Acceptance Criteria

1. When `--offline-dump` の値がゴーストのフォルダであるとき, the areka shall そのフォルダの `ghost\master\descript.txt` を読み、置き場の並びに関わらずそのゴーストのシェルを書き出しの対象にする。末尾の `\` の有無で結果を変えない。
2. When `--offline-dump`・`--dump-output-dir`・`--dump-error-log` の値が相対パスであるとき, the areka shall areka を起動したときの今のフォルダを基準にして読む。
3. While `--dump-shell` が渡されていないあいだ, the areka shall 既定のシェルを書き出しの対象にし、利用者が最後に選んだシェルの記憶を使わない。
4. When `--dump-shell` が渡されたとき, the areka shall その値を `shell\` の下のフォルダ名か、シェルの名前として探し、見つかったシェルを書き出しの対象にする（名前とフォルダ名の突き合わせの順は、名前でシェルを切り替えるときの今の決まりと同じ）。
5. If `--dump-shell` で指したシェルが見つからない場合, then the areka shall 既定のシェルに代えず、面を 1 枚も書き出さず、見つからなかったことを Error のレベルで書き出しの記録に残す。
6. If ゴーストの読み込みそのものに失敗した（フォルダが無い・`ghost\master\descript.txt` が無いか読めない）場合, then the areka shall 面を書き出さず、CSV を作らず、終了コード 0 で終え、失敗したことを areka の記録には error で残す（正典「ログは出力されず終了コードも0になる」）。
7. If シェルの読み込みに失敗した（シェルのフォルダが読めない・面が 1 つも無いなど）場合, then the areka shall 面を書き出さず、失敗したことを Error のレベルで書き出しの記録に残す。

### Requirement 3: 書き出す面の指定

**Objective:** As a 試験を書く開発者, I want 書き出す面を SSP と同じ書き方で並べたい, so that SSP 向けに書いた 1 行をそのまま areka に向けられる

#### Acceptance Criteria

1. When `--dump-surface-list` が渡されたとき, the areka shall カンマで区切られた各要素を面の番号として読み、各要素の頭の `surface` を省いても付けても同じに扱う。
2. When `--dump-surface-list` の要素が surfaces.txt の面の番号の拡張の書き方（`a-b` の範囲・`!` で始まる除外）であるとき, the areka shall surfaces.txt の面の番号の指定と同じ意味で、書き出す面の集まりを決める。
3. If 範囲でない単独の番号で指した面がシェルに無い場合, then the areka shall その面を飛ばし、無かったことを Warning のレベルで書き出しの記録に残す。
4. When 範囲で指した番号の中にシェルに無い面があるとき, the areka shall その番号を記録に残さずに飛ばす。
5. If `--dump-surface-list` の要素が面の番号として読めない場合, then the areka shall その要素だけを無視し、Warning のレベルで書き出しの記録に残し、ほかの要素は読む。
6. While `--dump-surface-list` が渡されていないか値が無いあいだ, the areka shall 面を書き出さず、シェルの読み込みと書き出しの記録の CSV（要件 7）と終了コード（要件 8）だけを行う。
7. When 同じ面が `--dump-surface-list` の中で 2 度以上指されたとき, the areka shall その面を 1 度だけ書き出す。
8. While `--dump-scope` が渡されていないあいだ, the areka shall スコープ 0 で書き出す。
9. When `--dump-scope` に 0 以上の整数が渡されたとき, the areka shall そのスコープの既定の着せ替えと `--dump-bind` を当てて書き出す。
10. If `--dump-scope` の値が 0 以上の整数として読めない場合, then the areka shall スコープ 0 で書き出し、そのことを Warning のレベルで書き出しの記録に残す。

### Requirement 4: 書き出す絵の中身

**Objective:** As a シェルの作者・試験を書く開発者, I want 書き出した絵が、画面に出る面と同じであってほしい, so that 窓を出さずに撮った絵で、画面の見た目を確かめたと言える

#### Acceptance Criteria

1. The areka shall 各面を、そのスコープの既定の着せ替えに `--dump-bind`（要件 5）を重ねた着せ替えで組み、runonce のアニメーションを 1 周させ終えた後の状態（正典「runonceのアニメーションを1周させた後の状態」）で書き出す。
2. The areka shall 利用者が最後に着せ替えた状態の記憶を、書き出す絵に使わない。
3. The areka shall 書き出す絵を、面の組み立てで決まる大きさのまま、拡大・縮小・DPI による倍率・透明度を当てずに書き出す。
4. The areka shall 書き出す絵を、透明な所を透明のまま残した α 付きの PNG で書き出す。
5. The areka shall 書き出す絵の画素を、同じ面・同じ着せ替え・同じアニメーションの状態を areka の画面に出すために組んだ絵と、出す段（拡大・透明度）の差を除いて同じにする。
6. The areka shall 前もって α を掛けた色を PNG の色へ戻す計算を、MCP の `dump_surface` が返す PNG と同じにする（同じ組んだ絵からは同じ PNG の画素になる）。
7. When 動く絵（APNG・動く WebP）を使う element を持つ面を書き出すとき, the areka shall 画面に出すときと同じコマで書き出す。
8. If 面の組み立てに失敗した（面の画像が読めないなど）場合, then the areka shall その面を書き出さず、Error のレベルで書き出しの記録に残し、ほかの面の書き出しを続ける。

### Requirement 5: `--dump-bind`（着せ替えの指定）

**Objective:** As a 着せ替えを持つシェルの作者, I want 着せ替えの組み合わせごとの絵を書き出したい, so that 着せ替えの組み合わせの見た目を、メニューを触らずに確かめられる

#### Acceptance Criteria

1. When `--dump-bind` が渡されたとき, the areka shall セミコロンで区切られた各指定を `カテゴリ名,パーツ名,ON/OFF` として読み、書かれた順に、既定の着せ替えへ `\![bind]` と同じ意味（排他のカテゴリでパーツを ON にすると、同じカテゴリのほかのパーツは OFF）で重ねる。
2. When 指定のパーツ名が空か `__ALL__` であるとき, the areka shall そのカテゴリのすべてのパーツを対象にする。
3. When 指定の ON/OFF が `1` か `true` であるか省かれたとき, the areka shall そのパーツを ON にする（`\![bind]` と違い、省いても反転しない）。
4. When 指定の ON/OFF が `0` か `false` であるとき, the areka shall そのパーツを OFF にする。
5. The areka shall カテゴリ名とパーツ名の前後の空白を取り除かずに突き合わせる。
6. If 指定のカテゴリかパーツがそのスコープのシェルに無い場合, then the areka shall その指定だけを無視し、Warning のレベルで書き出しの記録に残し、書き出しを続ける。
7. If 指定の書き方が読めない（ON/OFF が `1`・`true`・`0`・`false` のどれでもないなど）場合, then the areka shall その指定だけを無視し、Warning のレベルで書き出しの記録に残し、書き出しを続ける。
8. The areka shall `--dump-bind` を `--dump-scope` で決めたスコープにだけ当てる。

### Requirement 6: 書き出すファイル

**Objective:** As a 試験を書く開発者, I want 書き出した PNG の場所と名前を決めたい, so that 試験の道具がファイルを名前で拾える

#### Acceptance Criteria

1. The areka shall 各面を `<頭の文字列><面の番号>.png` の名前で書き出し、頭の文字列は既定で `surface` とする（例: 面 0 は `surface0.png`）。
2. When `--dump-output-prefix` に値が渡されたとき, the areka shall 既定の `surface` をその値に置き換える（付け足さない。例: `x_` で面 0 は `x_0.png`）。
3. While `--dump-output-dir` が渡されていないあいだ, the areka shall areka を起動したときの今のフォルダへ書き出す。
4. When `--dump-output-dir` が渡されたとき, the areka shall そのフォルダへ書き出し、フォルダが無ければ作る。
5. When 書き出す先に同じ名前のファイルがあるとき, the areka shall そのファイルを書き出した絵で置き換える。
6. If 面のファイルを書き出せなかった（フォルダを作れない・書き込めない・名前にファイル名として使えない文字が入るなど）場合, then the areka shall その面を書き出せなかったことを Error のレベルで書き出しの記録に残し、ほかの面の書き出しを続ける。
7. The areka shall スコープの番号をファイル名に入れない（SSP と同じ名前）。

### Requirement 7: 書き出しの記録の CSV（`--dump-error-log`）

**Objective:** As a シェルの作者・試験を書く開発者, I want シェルの読み込みの警告と誤りを SSP と同じ形のファイルで受け取りたい, so that SSP 向けに書いた確かめの道具をそのまま areka に使える

#### Acceptance Criteria

1. When `--dump-error-log` にファイルが渡され、書き出しの記録が 1 件以上あるとき, the areka shall その記録をそのファイルへ、BOM 付きの UTF-8・改行 CRLF・1 行 1 件・各列を `"` で囲み列の中の `"` を `""` にした CSV で書く。
2. The areka shall CSV の各行を `"時刻","ゴースト名","レベル","内容"` の 4 列にし、時刻はその記録を残した時刻の `HH:MM:SS`（その PC の時刻）、ゴースト名はゴーストの `descript.txt` の `name`、内容は areka の記録の本文とする。
3. The areka shall 書き出しの記録に、オフラインの書き出しのあいだのシェルの読み込み・面の組み立て・PNG の書き出し・`--dump-shell`／`--dump-scope`／`--dump-surface-list`／`--dump-bind` の値の扱いで areka が残した warn と error の記録を、残した順にすべて入れる。起動オプションの名前の読み取りの記録（知らない名前・扱わない名前）は入れない。
4. The areka shall areka の warn の記録をレベル `Warning`、error の記録をレベル `Error` として書き、info の記録は CSV に入れない。
5. The areka shall 書き出しの記録を要約したり間引いたりせず、少なくとも 10,000 件までは 1 件ずつ CSV に書く。
6. If 書き出しの記録が 1 件も無い場合, then the areka shall CSV のファイルを作らず、同じパスに既にあるファイルを上書きも削除もしない。
7. If CSV のファイルを書けなかった場合, then the areka shall そのことを areka の記録に error で残し、終了コードは要件 8 の段のまま返す。
8. While `--dump-error-log` が渡されていないあいだ, the areka shall CSV を書かず、書き出しの記録は areka の記録にだけ残す。

### Requirement 8: 終了コード

**Objective:** As a CI と試験の道具, I want 終了コードだけで誤りの重さを知りたい, so that ファイルを読まずに合否を決められる

#### Acceptance Criteria

1. When オフラインの書き出しを終えるとき, the areka shall 書き出しの記録の最も重いレベルに従い、記録が無ければ 0・`Warning` まで（`Error` なし）なら 1・`Error` があれば 2 の終了コードで終える。
2. The areka shall 要件 8.1 の終了コードを、`--dump-error-log` の有無に関わらず同じに決める。
3. The areka shall `Info`・`Notice`・`Critical`・`System` のレベルを書き出しの記録に使わない（areka の記録は info・warn・error の 3 段で、正典の終了コード 3（`Critical`）は出ない）。
4. When ゴーストの読み込みそのものに失敗したとき（要件 2.6）, the areka shall 終了コード 0 で終える。

### Requirement 9: 書かないこと・触らないこと

**Objective:** As a 普段の areka を使う開発者, I want オフラインの書き出しが普段の areka の記憶や置き場を変えないでほしい, so that 試験で何度書き出しても、次に areka を立てたときの状態が変わらない

#### Acceptance Criteria

1. While オフラインの書き出しのあいだ, the areka shall 起動中の印を書かず、消さない。
2. While オフラインの書き出しのあいだ, the areka shall 最後に使ったゴースト・シェル・バルーンの記憶、窓の位置の記憶、着せ替えの記憶を書かない。
3. While オフラインの書き出しのあいだ, the areka shall 置き場の並びを書かず、利用者ごとの置き場への写しを作らず、ゴーストのフォルダの中にファイルを書かない。
4. The areka shall オフラインの書き出しで書くファイルを、書き出した PNG（要件 6）と CSV（要件 7）と、普段から書いている areka の記録のファイルだけにする。

### Requirement 10: ほかの起動オプションとの組み合わせ

**Objective:** As a 試験を書く開発者, I want オフラインの書き出しに関わらない起動オプションを付けても困らないでほしい, so that 起動の 1 行を使い回せる

#### Acceptance Criteria

1. When `--offline-dump` と一緒に `--ghost`・`--random`・`--option`・`--sstp-listen`・位置の引数が渡されたとき, the areka shall それらを効かせずにオフラインの書き出しを行い、効かせなかったことを warn で 1 件ずつ areka の記録に残す。
2. When `--offline-dump` と `--help` が一緒に渡されたとき, the areka shall `--help` の今の振る舞い（説明書を開いて終わる）を行い、オフラインの書き出しを行わない。
3. When `--dump-surface-option` か `--dump-animation` が `--offline-dump` と一緒に渡されたとき, the areka shall その値を使わず、まだ扱わないことを warn で 1 件ずつ areka の記録に残し、そのオプションが無いものとして書き出しを続ける。
4. When `--dump-` で始まる起動オプションが `--offline-dump` なしで渡されたとき, the areka shall 普段どおりに起動し、`--offline-dump` なしでは効かないことを warn で 1 件ずつ areka の記録に残す。

### Requirement 11: 配布物の説明書

**Objective:** As a 配布物を受け取ったシェルの作者・開発者, I want 説明書でオフラインの書き出しの使い方を読みたい, so that SSP のヘルプを探さずに使える

#### Acceptance Criteria

1. The 配布物の説明書（`dist/README.txt`）の起動オプションの節 shall `--offline-dump` を効く起動オプションに加え、`--dump-shell`・`--dump-scope`・`--dump-surface-list`・`--dump-output-dir`・`--dump-output-prefix`・`--dump-error-log`・`--dump-bind` の働き、書き出す名前、CSV の形、終了コードの段（0・1・2 と、3 は出ないこと）を書く。
2. The 配布物の説明書 shall `--offline-dump` の使い方の例を 1 行以上載せる。
3. The 配布物の説明書 shall `--dump-surface-option` と `--dump-animation` がまだ扱われず、記録に残って無視されることを書く。

### Requirement 12: 決定論のテスト

**Objective:** As a areka の開発者, I want オフラインの書き出しの答えを実機を使わないテストで固めたい, so that シェルの読み方や合成が変わっても、書き出しの約束が崩れないと分かる

#### Acceptance Criteria

1. The areka のテスト shall 引数の列からオフラインの書き出しの指定への読み方を、要件 1.8・2.2・3.1〜3.10・5.1〜5.7・6.1〜6.3・10.1〜10.4 の各分岐について、窓・GPU を使わずに確かめる。
2. The areka のテスト shall 作業用のフォルダの検体のシェルを書き出し、書き出した PNG の画素が、同じ面・同じ着せ替え・同じアニメーションの状態を画面に出すために組んだ絵と同じであること（要件 4.1・4.3〜4.6）、runonce のアニメーションを持つ面が 1 周させ終えた後の状態で書き出されること（要件 4.1）を確かめる。
3. The areka のテスト shall CSV の中身（BOM・CRLF・列の囲みと `""`・4 列・レベルの当てはめ・入れる記録と入れない記録・1 件も無いときに作らず既存のファイルを残すこと）を、要件 7.1〜7.6 について確かめる。
4. The areka のテスト shall 終了コードの段を、要件 8.1・8.2・8.4 と要件 2.5〜2.7 の各分岐について確かめる。
5. The areka のテスト shall オフラインの書き出しの後に、要件 9 の記憶と置き場とゴーストのフォルダが書き出しの前と同じであることを確かめる。
6. The areka のテスト shall 作業用のフォルダと検体と書き出し先を、ワークツリーの `target\` の下にだけ置く。

### Requirement 13: SSP の絵と比べる足場

**Objective:** As a SSP との一致を確かめる開発者・AI のエージェント, I want areka と SSP がそれぞれ書き出した同じ面の PNG を画素で比べたい, so that 「ぱすたが SSP と同じに動く」ことを窓を出さずに確かめられる

#### Acceptance Criteria

1. The areka の開発の道具 shall SSP の `--offline-dump` が書き出した PNG のフォルダと、areka の `--offline-dump` が書き出した PNG のフォルダを受け、同じ名前のファイルどうしを画素で比べる手段を持つ。
2. When 比べ終えたとき, the 比べる手段 shall ファイルごとに「同じ」「大きさが違う」「画素が違う（違う画素の数と、色と α の差の最大）」「片方にしか無い」のどれかを出す。
3. The 比べる手段 shall SSP を起こさず、SSP の設定や SSP のフォルダに触れない（比べる元の絵は、開発者か AI のエージェントが SSP で別に書き出す）。
4. The 比べる手段 shall 合否を自分では決めず、違いの数を返すだけにする（許す違いの大きさは、使う試験が決める）。
5. The areka のテスト shall 比べる手段の答えを、作業用のフォルダに置いた PNG の組（同じ・大きさ違い・画素違い・片方だけ）で確かめる。

## 要件の討議で確かめる点

brief の「要件の段の議題」のうち、上の要件で正典か brief の推しに従って決めたが、ギャップ分析で SSP を測ってから討議で確かめる点。

1. 書き出す絵の時点（要件 4.1）: 正典どおり runonce のアニメーションを 1 周させ終えた後の状態にした。MCP の `dump_surface` の面の指定（単体の合成）はアニメーションを進めない初めの状態を返すので、runonce を持つ面ではそれと違う絵になる。runonce を最後まで進める足場が 2〜3 タスク増える。runonce 以外（always・sometimes など）を進めるかどうかは正典が黙っているので、SSP で runonce と always を持つ面を書き出して測る。
2. 動く絵の element のコマ（要件 4.7）: 画面に出すときと同じコマにした。SSP が何コマ目を書き出すかを測り、違ったときにどちらに合わせるかを決める。
3. レベルの当てはめ（要件 7.4・8.3）: brief の推しどおり warn→`Warning`・error→`Error` とし、info は入れず、`Critical`・`System` は使わない（終了コード 3 は出ない）。
4. ゴーストの読み込みに失敗したときの終了コード（要件 2.6・8.4）: 正典どおり 0（CSV なし）にした。areka の記録には error で残す。
5. CSV の「内容」の形（要件 7.2）: areka の記録の本文のままにし、定義元のファイルと行を付けるのは `config-parse-diagnostics` に任せる。

# Requirements Document

## Project Description (Input)
- **困っている人**: 「．さくら」（華和梨・SHIORI/2.x・`vendors/sample_ghost/dot_sakura.nar`）のような古いゴーストを areka で使う人。そうしたゴーストは右クリックメニューの項目名を、areka が問わない古い名前の資源で持っている。
- **今の状況**: 「ネットワーク更新」の項目について、areka は SHIORI に `updatebutton.caption` だけを問う。「．さくら」は同じ項目名を古い名前 `updatebuttoncaption` で持つので使われず、項目は areka の素の言葉「ネットワーク更新」のままになる。網羅台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）の `updatebuttoncaption` の行は `vocabulary-only`・持ち主なし。加えて、消滅通告・おすすめ・ポータルの項目は areka のメニューに無い（⑵）が、⑵ は本 spec では扱わない（下の Boundary Context）。
- **変えたいこと**: 古い名前の資源を持つゴーストで、「ネットワーク更新」の項目名がゴーストの言葉になる。新しい名前を持つゴーストの項目名は今と変わらない。台帳の `updatebuttoncaption` の行が持ち主と状態の付いた行になる。「．さくら」で右クリックメニューを開き、項目名を SSP と見比べる。

## Introduction

この spec は、右クリックメニューの「ネットワーク更新」の項目名を、新しい名前 `updatebutton.caption` で引けないときに古い名前 `updatebuttoncaption` で引くようにする（brief の ⑴）。出どころは完了した `popup-menu-residue` の件 13（原文は `.kiro/specs/completed/popup-menu-residue/brief.md` の「2026-10-24 追記: 件 13」）。

2026-10-31 棚卸㉜の後の開発者の決め（brief の末尾の節）により、本 spec が取るのは ⑴ だけである。開発者「メニュー関係はreactor解放されたら大改造するので、内容によっては後回しになりますが」。⑴ は SHIORI に何を問うかが大半で、メニューを作り直しても残る。⑵（消滅通告・おすすめ・ポータルの項目を足す）はメニューの組み立てに項目を足す仕事で、windows-reactor の後の作り直しで捨てるため、本 spec の完了の棚卸で roadmap の「windows-reactor 待ち」の段へ渡す（要件 6）。

### 正典（ukadoc）

ukadoc は 2 つの名前を別々の項目として載せ、どちらも同じ言葉で説明している。

- `updatebutton.caption`（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebutton.caption>）: 「ネットワーク更新の名称。」
- `updatebuttoncaption`（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebuttoncaption>）: 「ネットワーク更新の名称。」

ukadoc は、2 つの名前の優先（両方あるときにどちらを使うか）も、ベースウェアがいつ・どの順で問うかも書いていない。本書の優先（要件 1 の 2・3）は brief の Approach「新しい名前が無いときに古い名前で引く」に拠る。SSP と同じかは、要件討議の前に SSP の MCP で測る（下の「測りで決める点」）。測った結果が本書と違えば、要件討議で本書を直す。

### 測りで決める点（要件討議の前に SSP の MCP で測る・推測で埋めない）

測り方は、SSP が SHIORI へ送った要求が残る `ghost\master\ssp_shiori_log.txt` の、測る前のバイト位置からの差分を読む。検体の根・一時フォルダはワークツリーの `target\` の下に置く。

| 測 | 測ること | 答えで変わる所 |
|---|---|---|
| 測 1 | SSP は「．さくら」の右クリックメニューを開くとき、`updatebuttoncaption` を問い合わせるか。問うなら `updatebutton.caption` との順と、問う時（メニューを開くたびか・それ以外か） | 要件 1 の 1 |
| 測 2 | 両方の名前に非空の値を返すゴーストで、SSP はどちらを「ネットワーク更新」の項目名に使うか | 要件 1 の 2 |
| 測 3 | `updatebutton.caption` が空の値（`200` で値が空）を返し、`updatebuttoncaption` が非空の値を返すとき、SSP は古い名前の値に落ちるか（値なし〔`204`〕のときと同じか） | 要件 1 の 3 |
| 測 4 | SSP は `readmebuttoncaption` を問い合わせるか。問うなら、どの項目の名前に使うか（下の「要件討議で決める点」の 1 の材料。本 spec の範囲は変えない） | なし（議題の材料） |

### 要件討議で決める点（本書では範囲に入れていない）

1. **`readmebuttoncaption`**: ukadoc の検索で、`updatebuttoncaption` と同じ形の名前がもう 1 つ見つかった。`readmebuttoncaption`（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#readmebuttoncaption>「Read Me!の名称。」）で、台帳の行は `vocabulary-only`・持ち主なし。ただし今 areka が問う `readmebutton.caption`（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#readmebutton.caption>「キャラクタについての名称。」）とは説明の言葉が違い、同じ項目の古い名前かどうかを ukadoc からは読めない。brief と開発者の決めは ⑴ を `updatebuttoncaption` に限っているので、本書は範囲に入れない。取り込むか・別に渡すかを、測 4 の結果とともに要件討議で決める。
2. **`updatebutton.caption` の行の備考**: 台帳のこの行は「空・値なし・失敗・時間切れのときは既定名「ネットワーク更新」に落ち」と書いており、本 spec の後は事実と食い違う。要件 4 の 3 で文だけを直すとした（brief は「台帳の 1 行」と書くが、隣の行の文が誤りになるのを残さないため）。この扱いでよいかを確かめる。

### この文書で使う言葉

- **新しい名前**: `updatebutton.caption`。**古い名前**: `updatebuttoncaption`。
- **引ける**: SHIORI の答えが非空の値であること。空の値・値なし・照会の失敗は「引けない」（今の項目名の扱いと同じ）。
- **既定名**: 資源が引けなかったときに areka が出す項目名。「ネットワーク更新」の項目では「ネットワーク更新」。
- **1 回の表示の照会**: 右クリックメニューを 1 回出すために、項目名と表示可否の名前をまとめて SHIORI へ問う 1 回の往復。返事を待つ上限は今と同じ。
- **網羅台帳**: `doc/ukadoc-coverage/ledger/shiori.toml`。正典の名前ごとに、対応の状態・持ち主の spec・備考を 1 行ずつ持つ。
- **⑵**: 消滅通告（`vanishbuttoncaption`・`vanishbuttonvisible`）・おすすめ（`sakura.recommendbuttoncaption`・`kero.recommendbuttoncaption`・`sakura.recommendsites`・`kero.recommendsites`）・ポータル（`sakura.portalbuttoncaption`・`sakura.portalsites`）の 8 つの名前と、その項目をメニューに足す仕事。3 人目以降の窓の `char*.recommendbuttoncaption`・`char*.recommendsites`・`char*.recommendsites.caption` の 3 つも ⑵ と一緒に回す。

## Boundary Context

- **In scope**:
  - 「ネットワーク更新」の項目名を、新しい名前で引けないときに古い名前で引くこと。
  - SHIORI に問い合わせてよい名前の表に古い名前を 1 つ足すことと、その兄弟のテスト。
  - 網羅台帳の `updatebuttoncaption` の 1 行（状態と持ち主と備考）。
  - 「．さくら」で右クリックメニューを開き、SSP と見比べる実機の確かめ 1 回。
- **Out of scope**:
  - ⑵ のすべて（消滅通告・おすすめ・ポータルの項目をメニューに足すこと、その 8 つの名前と `char*.recommend*` の 3 つを問うこと、おすすめ・ポータルのサイトの一覧の読み方と開く道）。理由: 開発者の決め（2026-10-31）。メニューの組み立てに項目を足す仕事は windows-reactor の後の作り直しで捨てるので、それまで待つ。⑵ は本 spec の完了の棚卸で roadmap の「windows-reactor 待ち」の段へ渡す（要件 6）。
  - メニューの組み立てと表示（`crates/areka/src/menu/{plan,mod,win32}.rs`）を変えること。開発者の決めにより、本 spec の直しは問う名前の表と項目名の引き方の所に収める。
  - 「ネットワーク更新」以外の項目名の照会、`readmebuttoncaption`（要件討議で決める点の 1）。
  - メニューの見た目（オーナードロー・着せ替え＝`popup-menu-shell-look`）、3 人目以降のキャラクター窓のメニュー（`extra-character-windows`）、省略した形のメニューの作り分け（`popup-menu-residue` が作り分けないと裁定済み）。
  - 更新の項目を選べる条件と、選んだ後の更新の動き（`areka-P0-network-update` の持ち場）。
- **Adjacent expectations**:
  - SHIORI/2.x の相手への資源の問い合わせの組み替えは、完了した `shiori2-protocol` が受け持つ。本 spec は問う名前を 1 つ足すだけで、2.x の書き方には手を入れない。足した名前も今の名前と同じ道で 2.x の相手へ届くことを前提にする（設計の段で確かめる）。
  - 同じウェーブ C15 で並走する `shell-scale-linkage` とは、触るファイルが重ならない（向こうの kanade は `schedule/events.rs`、本 spec は `schedule/resources.rs`。向こうは `menu/` に触らない）。網羅台帳は互いに別の行だけを触る。
  - `popup-menu-shell-look`・`extra-character-windows`・`menu-thumbnail-preview`・`tasktray-icon` は `menu/` を分け合うが、同じウェーブに居ない。

## Requirements

### Requirement 1: 「ネットワーク更新」の項目名を古い名前でも引く

**Objective:** 古いゴーストを使う人として、「ネットワーク更新」の項目名が、ゴーストが古い名前 `updatebuttoncaption` で用意した言葉になってほしい。SSP で見るのと同じ、そのゴーストらしいメニューになるように。

#### Acceptance Criteria

1. When 「ネットワーク更新」の項目を含む右クリックメニューを出す, the areka shall 1 回の表示の照会で、新しい名前に加えて古い名前も SHIORI に問い合わせる。
2. When 新しい名前が引けた, the areka shall 新しい名前の値を「ネットワーク更新」の項目名に使い、古い名前の値は使わない。
3. When 新しい名前が引けず古い名前が引けた, the areka shall 古い名前の値を「ネットワーク更新」の項目名に使う。
4. If 新しい名前も古い名前も引けない, the areka shall 「ネットワーク更新」の項目を既定名「ネットワーク更新」で出し、メニューはそのまま出す。
5. When 項目名に古い名前の値を使う, the areka shall 新しい名前の値を使うときと同じ扱い（値の `&` をアクセラレータ記法として素通しすること・前後を削らないこと）で出す。
6. If 1 回の表示の照会の返事が上限時間までに来ない, the areka shall 今と同じく全件を既定名にしてメニューを出し、古い名前を足したことで返事を待つ上限を延ばさない。
7. While 「ネットワーク更新」の項目が右クリックメニューに登記されていない, the areka shall 古い名前を問い合わせない。

### Requirement 2: 古い名前の空と失敗を今の記録の決まりで残す

**Objective:** 障害のときに記録を読む人として、古い名前が空だったり問い合わせに失敗したりしたことが、今の項目名と同じ決まりで記録に見えてほしい。どの名前が引けなかったかを、新しい記録の種類を覚えずに読めるように。

#### Acceptance Criteria

1. When 古い名前が空の値または値なしを返す, the areka shall その名前を、今の「空または値なし」の記録（`debug`・1 回の表示につき 1 行）に他の名前と一緒に載せる。
2. If 古い名前の問い合わせが失敗する, the areka shall その名前と理由を、今の「照会の失敗」の記録（`warn`・1 回の表示につき 1 行）に他の名前と一緒に載せる。
3. The areka shall 古い名前のために、記録の行の数（1 回の表示につき種類ごとに 1 行）も、記録の水準も、記録の種類の名前（`event=`）も増やさない。

### Requirement 3: 新しい名前を持つゴーストと他の項目の振る舞いを変えない

**Objective:** 今のゴースト（emo2・ぱすたさん・里々・YAYA の検体など）を使う人として、この直しで右クリックメニューが変わらないでほしい。直したのは古い名前に頼るゴーストの所だけ、と言えるように。

#### Acceptance Criteria

1. While ゴーストが新しい名前に非空の値を返す, the areka shall 「ネットワーク更新」の項目名を今と同じ値で出す。
2. The areka shall 「ネットワーク更新」以外の項目名の問い合わせ・既定名・表示可否（`sakura.popupmenu.visible`・`kero.popupmenu.visible`）の判定を今と変えない。
3. The areka shall 「ネットワーク更新」の項目を選べる条件・選んだ後の動作・メニューの項目の並びと有無を今と変えない。
4. The areka shall SHIORI/3.0 のゴーストにも SHIORI/2.x のゴーストにも、同じ決まり（要件 1）で古い名前を問い合わせる。
5. The areka shall 問い合わせない名前の表（`popupmenu.type` の 3 つと `char*.popupmenu.visible`）を今と変えず、それらを問い合わせない。

### Requirement 4: 問い合わせてよい名前の表と網羅台帳

**Objective:** 網羅台帳を読む人と次にメニューへ手を入れる開発者として、古い名前を問うようになったことが、問い合わせてよい名前の表と台帳の両方に正しく現れてほしい。⑵ の名前が誤って「済み」に見えないように。

#### Acceptance Criteria

1. The popup-menu-extra-resources spec shall SHIORI に問い合わせてよい名前の表（`crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`）に古い名前を ukadoc の URL とともに 1 つだけ足し、⑵ の 8 つの名前と `char*.recommend*` の 3 つは足さない。
2. The popup-menu-extra-resources spec shall 網羅台帳の `updatebuttoncaption` の行を、状態を実装済み（`implemented`）・持ち主を本 spec にし、備考を同じ台帳の `updatebutton.caption` の行と同じ組み立て（中身・壊れ方・ログ・根拠の場所・画面の材料・無いと失うもの）で書き、新しい名前との優先（要件 1 の 2・3）と測 1〜3 の結果を書く。
3. The popup-menu-extra-resources spec shall 網羅台帳の `updatebutton.caption` の行の状態と持ち主を変えず、備考の「壊れ方」の文だけを、新しい名前が引けないときは古い名前に落ちることに合わせる。
4. The popup-menu-extra-resources spec shall 網羅台帳の ⑵ の 8 行と `char*.recommend*` の 3 行を変えない。
5. When 台帳の持ち主を本 spec にする, the popup-menu-extra-resources spec shall 台帳の整合の検査（持ち主の名前が spec の一覧に在ること・持ち主の行数など）が通る形にする。

### Requirement 5: 決定論テストと実機の確かめ

**Objective:** 開発者として、新しい名前と古い名前の組み合わせごとの項目名が決定論テストで固定され、「．さくら」の実機で SSP と同じ項目名になったことが記録に残ってほしい。直しが退行したら赤で分かり、画面で確かめた事実が後から読めるように。

#### Acceptance Criteria

1. The popup-menu-extra-resources spec shall 次の組み合わせごとに、項目名が期待どおりになることを決定論テストで確かめる: 新しい名前だけが引ける・古い名前だけが引ける・両方が引ける・どちらも引けない・新しい名前が空の値で古い名前が引ける・新しい名前の問い合わせが失敗し古い名前が引ける。
2. The popup-menu-extra-resources spec shall 「ネットワーク更新」の項目があるときだけ古い名前が 1 回の表示の照会に入り、同じ名前が重ならないことを決定論テストで確かめる。
3. The popup-menu-extra-resources spec shall 問い合わせてよい名前の表の中身を書き下した凍結のテストを、古い名前を足した 15 の名前に合わせ、⑵ の名前が表に無いことを確かめる。
4. The popup-menu-extra-resources spec shall 新しいテストを、確かめる実装と同じディレクトリの兄弟のファイルか、そのクレートの今在るテストの束に置く。
5. When 本 spec の変更が済む, the popup-menu-extra-resources spec shall 触ったクレート（`areka`・`areka-kanade`）と、網羅台帳を検査するクレート（`ukadoc-survey`）のテストがすべて通ることを確かめる。
6. When 直しが済む, the popup-menu-extra-resources spec shall 排他テストの机を取って「．さくら」を areka で起こし、右クリックメニューを開いて、「ネットワーク更新」の項目名がゴーストの言葉になっていることと、他の項目名を、同じゴーストを SSP で開いたメニューと見比べ、結果を検証の記録に残す。
7. When 実機で見比べる, the popup-menu-extra-resources spec shall ⑵ の項目（消滅通告・おすすめ・ポータル）が areka に無いことを既知の差として記録し、本 spec では直さない。
8. The popup-menu-extra-resources spec shall 実機の確かめの根・検体・一時フォルダを、ワークツリーの `target\` の下だけに置く。

### Requirement 6: ⑵ を範囲の外に置き、完了の棚卸で渡す

**Objective:** 開発者として、本 spec で扱わない ⑵ が、閉じた spec の記録に埋もれず、windows-reactor の後の作り直しで拾われてほしい。⑵ が黙って落ちないように。

#### Acceptance Criteria

1. The popup-menu-extra-resources spec shall 消滅通告・おすすめ・ポータルの項目をメニューに足さず、⑵ の 8 つの名前と `char*.recommend*` の 3 つを SHIORI に問い合わせない。
2. The popup-menu-extra-resources spec shall メニューの組み立てと表示（`crates/areka/src/menu/{plan,mod,win32}.rs`）を変えない。
3. When 本 spec を完了する, the popup-menu-extra-resources spec shall 完了の棚卸で ⑵（8 つの名前と `char*.recommend*` の 3 つ・おすすめとポータルのサイトの一覧の読み方と開く道・`popup-menu-shell-look` の件 15 との分け方が未決であること）を、roadmap の「windows-reactor 待ち」の段へ、渡す理由（開発者の決め・メニューの作り直しで項目を組む仕事を捨てるため）とともに渡す。新しい brief にするか本 spec の brief の残りとして置くかは、その完了の棚卸で決める。
4. When ⑵ を渡す, the popup-menu-extra-resources spec shall 本 spec の brief に、渡した先と日付を書く。

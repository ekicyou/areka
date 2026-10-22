# Requirements Document

## Project Description (Input)

- **困っている人**: areka の開発者。2003 年ごろの古い作りのゴースト（華和梨・Shift_JIS・α の無い絵・連番のオンラインの印を持つ同梱バルーン）を、手元で繰り返し確かめる材料が無い。
- **今の状況**:
  - 検体は pasta（`emo2`・`pasta-in-windows`）・里々（`R_POST_and_KOMAINU`）・YAYA（`konnoyayame`・`claudia`・`emily4`）の 3 系統だけで、検体の登記表は 9 行（うち `pasta-in-windows` は書庫を置かず URL で引く行）。
  - 開発者の自作のゴースト「．さくら」（どっとさくら・フォルダ名 `dot_sakura`）は、その古い作りをまとめて持つ。書庫は開発者のサイト <https://ekicyou.github.io/old/dot-sakura/download/nar/dot_sakura.nar> で公開されている（1,425,534 バイト・SHA-256 `0c602c6a431dcbec48c02d934d98e342337d2db9628095e0a6f90d3e83b7a472`）。隣にハッシュのファイル（`dot_sakura.nar.sha256`）は無い（2026-10-22 に確かめて 404）。
  - 中身: SHIORI は 32 ビットの華和梨 7.4（`shiori.dll`）で、areka の検体に無かった 4 つ目の系統。辞書は Shift_JIS。シェル 3 つ（`master`・`sakura100`・`sakura50`）の絵は α の無い PNG だけ。同梱バルーン `bottle`（「ボトル（でふぉ改）」）は、面 `balloons0〜3`・`balloonk0〜1`・`balloonc0〜3` と、印 `arrow0/1`・`online`・`online0〜15`・`sstp` を持ち、絵は 8 ビットのパレットの PNG で角を赤の抜き色で抜く。最上位に `install.txt`（`balloon.directory,bottle`・取り出し元の書き込みなし）・`delete.txt`・`updates.txt`・`readme.txt` が在る。
- **変えたいこと**: どっとさくらを検体の名前 `dot_sakura` で引け、同梱バルーン `bottle` も一緒に入る。検体の README に出どころ・SHA-256・取り扱いの条件が載る。実機で 1 回起こした結果が記録に残り、areka が対応していないために崩れた件が全部起票されている。
- 詳細は同じフォルダの `brief.md`。

## Introduction

本 spec は、開発者の自作のゴースト「．さくら」を、areka の検体に足す spec である。やることは 3 つある。

1. 書庫を受け取ったバイト列のままリポジトリに置き、検体の登記表に 1 行足す（9 行 → 10 行）。
2. 検体の README・steering・根の `README.md` に登記する。
3. 実機で 1 回起こし、見えたことを記録し、areka の未対応で崩れた件を起票する。

**動くところまで直すのは、本 spec の仕事にしない。** 崩れの直しは起票した先が持つ。

書庫の置き方は、2026-10-10 の開発者の決め（書庫をリポジトリに入れる）に従う。URL の行（完了 `areka-P0-sample-url-fetch` の形）にするには、取り寄せ先の隣に `dot_sakura.nar.sha256` が要るが、2026-10-22 の時点で公開されていない（棚卸㉘の「開発者に決めてほしいこと」⑷＝置かなければ書庫をリポジトリに置く形）。

## Boundary Context

- **In scope**:
  - 書庫 `vendors/sample_ghost/dot_sakura.nar` を、受け取ったバイト列のまま置くこと。
  - 検体の登記表の 1 行（ゴースト `dot_sakura`・同梱バルーン `bottle`）と、登記の数を持つ自動テストと文書の数（9 → 10）。
  - `vendors/sample_ghost/README.md` の表と節。
  - steering（`product.md` の検体の表・`structure.md` の検体の顔ぶれ）と、根の `README.md` の動作を確かめているゴーストの表への `dot_sakura` の 1 行。
  - 実機で 1 回起こした記録（本 spec のフォルダの中）と、崩れの起票。
- **Out of scope**（本 spec では行わない）:
  - どっとさくらの起動・会話・メニューを通すための areka の直し（起票した先が持つ）。
  - 華和梨への対応、SHIORI 2.x への対応。
  - 同梱バルーンの印（矢印・オンライン・SSTP）の表示（`areka-P0-balloon-markers` が持つ）。
  - 入力の箱 `balloonc*`（`areka-P0-balloon-canon-residue`・`areka-P0-inputbox-user-input` が持つ）。
  - シェルの縮小版 2 つ（`sakura100`・`sakura50`）の見た目の確かめ。
  - areka の配布物へのどっとさくらの同梱（入れない）。
  - 書庫のバイト列の変更（畳み直し・改行の変換・中のファイルの足し引き）。
  - 書庫を URL の行で引く形への切り替え（取り寄せ先にハッシュのファイルが置かれた後の話）。
  - 根の `README.md` の表への `pasta-in-windows` の行（この表は「動作を確かめているゴースト」の表で、窓際のぱすたさんの確かめは `areka-P0-pasta-in-windows-verification` が持つ）。`emily4` の行は既に在る。
  - steering の `product.md` の検体の表に欠けている、ほかの検体（`emily4`・`pasta-in-windows`）の行の補い。
- **Adjacent expectations**:
  - `.nar` のインストール（`areka-nar`）・32 ビットの SHIORI の載せ台（`shiori-host32`）・表示の経路は、今のまま使う（変更 0）。崩れは起票する。
  - 既定のバルーン（`StayseeBalloon`）と、その解決の決まりには触らない（`areka-P0-default-balloon-selfmade` が持つ）。同梱バルーンを持つゴーストは、起動で同梱の 1 個目が既定より先に採られる（完了 `areka-P0-ghost-standard-balloon` の決まり・変更 0）。
  - 検体の登記表・数のテスト・検体の README は、後ろに `areka-P0-default-balloon-selfmade`（10 → 11）が続く。本 spec の着地の後に、そちらが数を合わせる。
  - 同じウェーブの `areka-P0-pasta-in-windows-verification` は、登記表（`crates/sample-ghost-kit/src/lib.rs`・`lib_tests.rs`）に触らない約束である（棚卸㉘）。

## Requirements

### Requirement 1: 書庫を受け取ったバイト列のまま置く

**Objective:** As a areka の開発者, I want どっとさくらの書庫が、開発者のサイトで公開しているバイト列のままリポジトリに在ること, so that 検体の中身が環境や時期で揺れず、後で URL の行へ移すときも同じバイト列を指せる

#### Acceptance Criteria

1. When 書庫を取り寄せたとき, the 検体を足す作業 shall 取り寄せた書庫の大きさと SHA-256 を、brief.md の値（1,425,534 バイト・`0c602c6a431dcbec48c02d934d98e342337d2db9628095e0a6f90d3e83b7a472`）と比べる。
2. If 取り寄せた書庫の SHA-256 が 1 項の値と違うとき, then the 検体を足す作業 shall 書庫を置かずに止め、違った値を開発者へ伝える（開発者のサイトの書庫が差し替わった可能性があるため）。
3. The 検体の置き場 shall 書庫を `vendors/sample_ghost/dot_sakura.nar` に、取り寄せたバイト列のまま置く。畳み直し・改行の変換・中のファイルの足し引きは、0 件とする。
4. When 書庫をリポジトリに追加した後, the 検体を足す作業 shall 追跡されたファイルを取り出し直した SHA-256 が 1 項の値と一致することを確かめる。
5. The areka の配布物 shall どっとさくらを含めない（配布物の中身の変更 0）。

### Requirement 2: 検体 `dot_sakura` の登記

**Objective:** As a areka の開発者, I want どっとさくらを他の検体と同じ窓口で名前で取り出せること, so that 自動テストと実機の確かめが、同じ検体を同じ手順で使える

#### Acceptance Criteria

1. The 検体の登記表 shall `dot_sakura` を、書庫をリポジトリに置く形の 1 行で持つ。その行が述べる入る物は、ゴースト `dot_sakura` と同梱バルーン `bottle` とする。
2. When 検体の窓口で `dot_sakura` を名指ししたとき, the 検体の窓口 shall 他の検体と同じく、ワークツリーの `target\` の下の根へゴースト `dot_sakura` と同梱バルーン `bottle` を入れ、その根を返す。
3. The 検体の登記の自動テスト shall 登記した検体の数を 10 とする。数の主張・文書の注釈・「既知の名前の一覧を含む失敗」の名前の一覧に、直書きの 9 を残さない（残り 0 か所）。
4. The 検体の登記の自動テスト shall 「登記した検体はどれも登記の行のとおりの位置に入る」と「どの検体の書庫も登記の行が述べる物だけを入れる」を、`dot_sakura` を含めて緑にする。
5. When `dot_sakura` を登記したとき, the 検体のパスの綴りの見張り shall 緑のままとする。新しい見張りの語は `dot_sakura` と `bottle` である。
6. If 5 項の見張りが赤になるとき, then the 検体を足す作業 shall 見張りの側の決まり（どこまでを当たりと見るか）で解き、その理由を記録に書く。検体の名前を変えること・書庫を書き換えることでは解かない。
7. If areka のインストールが `dot_sakura.nar` を断るとき, then the 検体を足す作業 shall 登記表に行を足さず（数は 9 のまま）、断った理由を記録に書いて起票し、本 spec を検体の README の登記（Requirement 3）までで閉じる。この場合、Requirement 4（検体の数と顔ぶれを述べる文書）と Requirement 5（実機の 1 回）は行わない。
8. If ゴースト `dot_sakura` は入るが同梱バルーン `bottle` が入らないとき, then the 検体を足す作業 shall 登記の行が述べる入る物を実際に入る物（ゴースト `dot_sakura` だけ）に合わせ、入らなかった理由を記録に書いて起票する。

### Requirement 3: 検体の README の登記

**Objective:** As a areka の開発者, I want どっとさくらの出どころと取り扱いの条件が、検体の README から読めること, so that 書庫をリポジトリに置いてよい理由と、配布物へ入れない理由を、後から辿れる

#### Acceptance Criteria

1. The `vendors/sample_ghost/README.md` shall 書庫を置く行の検体の表に `dot_sakura.nar` の 1 行（種別・中のファイル数・大きさ）を持ち、表の見出しの検体の数を書庫を置く行の実数に合わせる。
2. The `vendors/sample_ghost/README.md` shall `dot_sakura.nar` の節に、取り寄せ先の URL・SHA-256・取り寄せた日・受け取ったバイト列のまま置いたこと（畳み直していないこと）を書く。
3. The `vendors/sample_ghost/README.md` shall `dot_sakura.nar` の節に、書庫の中の `readme.txt` の取り扱いの条件（「無償ですが著作権複数のためフリーには出来ません」「転載・配布: インターネット上は不可、それ以外は各自の責任において可」）と、シェルの `readme.txt` の条件（リビルド: 日蔭 翠・インターネット上でのアーカイブの転載不可）を書く。
4. The `vendors/sample_ghost/README.md` shall 3 項の条件のもとで書庫をリポジトリに置く根拠として、2026-10-10 の開発者の言葉（「どっとさくらは私の実装なので、検体として利用する分には問題ない。私のサイトで公開してます」）を書き、areka の配布物へ入れないことを書く。
5. The `vendors/sample_ghost/README.md` shall `dot_sakura.nar` の節に、検体として何を見るためのものか（華和梨の 32 ビットの SHIORI・Shift_JIS の辞書・α の無い絵・同梱バルーン `bottle` の連番のオンラインの印と抜き色）を書く。
6. The `vendors/sample_ghost/README.md` shall `dot_sakura.nar` の節に、URL の行にしていない理由（取り寄せ先の隣にハッシュのファイルが無いこと・2026-10-22 時点）を書く。

### Requirement 4: 検体の数と顔ぶれを述べる文書

**Objective:** As a areka の開発者, I want 検体の数と顔ぶれを述べる文書が、登記表と食い違わないこと, so that 文書を読んだ人が、どっとさくらが検体に在ることと、SHIORI の系統が 4 つになったことを知れる

#### Acceptance Criteria

1. The steering の `structure.md` shall 検体の顔ぶれを 10 本（ゴースト 7 体・バルーン 3 本）とし、`dot_sakura`（華和梨・32 ビット・同梱バルーン `bottle`・受け取ったバイト列のまま・配布物へ入れない）を載せ、SHIORI の系統を華和梨を含む 4 つとする。
2. The 根の `README.md` shall 動作を確かめているゴーストの表に `dot_sakura` の 1 行を持ち、備考に、実機の 1 回で分かったこと（起動まで通ったか、通らなければその旨と起票先）と、開発用の検体としてだけ使い配布物には入れないことを書く。
3. The steering の `product.md` shall 適合の検体の表に `dot_sakura`（華和梨・32 ビット）の 1 行を持ち、表の前の「SHIORI 3 系統」の言い方を、華和梨を含む 4 系統に合わせる。

### Requirement 5: 実機で 1 回起こす

**Objective:** As a areka の開発者, I want どっとさくらを実機で 1 回起こした結果が、決まった条件で記録に残ること, so that 古い作りのゴーストで areka の何が崩れるかを、後から読み直して起票と突き合わせられる

#### Acceptance Criteria

1. The 実機の走行 shall debug 版の `areka.exe` を、検体の窓口が `target\` の下に入れた根から、短い絶対パスで起動して回す（`C:\` の直下・`C:\tmp` は使わない）。
2. When 実機の走行を始めるとき, the 実機の走行 shall 起動の前に、32 ビットの SHIORI を読むための i686 の補助プログラムが `areka.exe` の隣に在ることを確かめる（全体テストの後に x64 版へ替わっていることがあるため）。
3. While 実機の走行を回している, the 実機の走行 shall 判定に使う分岐の記録が出る水準まで記録の水準を開け、自動終了を安全弁として 1 回の走行の所要より長い上限で置く。
4. When 実機の走行が終わったとき, the 実機の走行 shall その走行で起こした areka のプロセスを 1 つも残さない。外から止めるときは、その走行で自分が起こしたと確かめたプロセスだけを止める。
5. The 実機の走行 shall 他の spec の実機の窓を出す回・負荷の走行と、同じ時間に回さない。
6. The 実機の記録 shall 本 spec のフォルダの中に置き、少なくとも次を持つ。
   - 同定（コミット・debug 版であること・検体の SHA-256）
   - 機械の構成（OS の版・画面の拡大率）
   - 環境変数と記録の水準
   - 走行の日時（日本時間）
7. The 実機の記録 shall 次の項目ごとに、「出た」「出ない」「確かめていない」のどれか 1 つと、その根拠（記録の行の時刻つきの逐語の引用、または MCP で取った面・バルーンの状態の文での説明）を書く。
   - インストールの結果と、インストールで出た警告（最上位の `delete.txt`・`updates.txt` をどう扱ったかを含む）
   - 華和梨の SHIORI を 32 ビットの載せ台が読み込めたか
   - SHIORI の版（2.x か 3.0 か）と、その判定の根拠
   - 起動したか（起動のイベントの台詞がバルーンに出たか）
   - 1 人目と 2 人目のキャラクターの窓に、シェル `master` の面が出たか
   - α の無い絵の抜き色の透過
   - 同梱バルーン `bottle` が採られて出たか
   - 終了まで通ったか
8. If 華和梨の SHIORI を読み込めないとき、または SHIORI の版が 2.x だけのとき, then the 実機の記録 shall その旨と根拠を書き、7 項の残りの項目を「確かめていない」とし、本 spec の中で直さない。
9. The 実機の記録 shall シェルとバルーンの絵の撮影（スクリーンショット・面の画像）を、文書と PR に貼らない。貼るのは、開発者が許したときだけとする（シェルは第三者の絵を含むため）。
10. The 本 spec shall どっとさくらを動かすための areka のコードの変更を含まない（変更 0）。

### Requirement 6: 崩れの起票

**Objective:** As a areka の開発者, I want 実機の 1 回で見えた崩れが、1 件も漏れずに持ち主の spec へ渡ること, so that どっとさくらで分かった areka の未対応を、後で直す列に載せられる

#### Acceptance Criteria

1. The 実機の記録 shall 崩れた所を 1 件ずつ、症状・根拠・原因の見立て・扱いの 4 つで書く。扱いは「起票した」「既存の spec の brief に足した」「方針で受けない」「ゴーストの側の作り（繕わない）」のどれか 1 つとする。
2. When areka が対応していないために崩れた件を見つけたとき, the 開発者 shall 本 spec の範囲の外のものも含めて全部、`/kiro-discovery` で起票する。触る場所が同じ件は 1 本にまとめ、既存の brief に「件 N」として足してよい。
3. When 崩れた件が既存の spec（例: `areka-P0-balloon-markers`・`areka-P0-balloon-canon-residue`・`areka-P0-inputbox-user-input`）の持ち物であるとき, the 開発者 shall その spec の brief に、どっとさくらで見えた事実を足す（「既知」だけで済ませない）。
4. The 実機の記録 shall 「方針で受けない」の件に、その裁定（日付と要旨）を書く。
5. The 実機の記録 shall 崩れの一覧の末尾に、扱いごとの件数を書く。0 件の扱いも 0 と書く。
6. The 実機の記録 shall 記録の ERROR と WARN を数えるとき、どっとさくらの中身から出るもの（辞書が持たない面を呼ぶなど）を、理由を書いて数から外す。

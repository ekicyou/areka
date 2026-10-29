# Requirements Document

## Project Description (Input)

**誰の何が困っているか**: areka を受け取って使う人と、areka を配る開発者。バルーンを持たないゴーストを入れると、areka は既定のバルーンで喋らせるが、その既定は第三者の `StayseeBalloon`（CC0）を借りたもので、areka が自分の顔として配るバルーンが無い。手本にしたい開発者自作のバルーン「ボトル（でふぉ改）」（自作ゴースト「．さくら」に同梱）は寸法と設定値が使い込まれているが、作りが古い（設定は Shift_JIS・絵は α の無いパレットの PNG を赤の抜き色で抜く・オンラインの印は連番 16 枚・瓶の透かしや夕日と波の絵が入る）。

**今の状態**: 既定バルーンの名前は起動の解決の定数 1 つ（`crates/areka/src/boot_resolve.rs` の `DEFAULT_BALLOON_FOLDER`＝`StayseeBalloon`）で、配布の道具 `tools/package.ps1` が検体の窓口から `StayseeBalloon` を引いて配布物の `balloon/StayseeBalloon/` に入れる。文書（根の `README.md`・`dist/README.txt`・`vendors/sample_ghost/README.md`・steering）も `StayseeBalloon` を既定と書く。

**何を変えるか**: 手本の寸法と設定値を引き継ぎ、絵を簡素にして α 付きで描き直し、設定ファイルを UTF-8 にし、オンラインの印を 1 枚の動く WebP にした areka 自前の既定バルーンを作る。絵の作り方と権利の扱いをリポジトリに残し、既定を `StayseeBalloon` から新しいバルーンへ差し替える。絵柄は要件の討議で案を出して開発者が選ぶ。

> 起票: 2026-10-10 `/kiro-discovery`（開発者「これに含まれるバルーンを参考に、デフォルトバルーンを作ってほしい。設定系パラメーターは引き継ぐ、設定ファイルを utf-8 に変える。online0.png などのアニメは動画 webp などで実現する。デザインをもっとシンプルにする。オンラインなどの通信アニメーションももっとシンプルにする。サイズ感はそのまま流用する。画像ファイルなどは現代的なフォーマットにする」）。版の段で v0.1.0。ウェーブ C12（棚卸㉚）。要件生成: 2026-10-29（main `932d2c30`）。詳細は brief.md。

## Introduction

本仕様は、areka 自前の既定バルーンの絵を作って選ぶ仕事と、既定を差し替える仕事を、1 本で続けて受け持つ。**絵柄は要件の段で案を出して開発者が選ぶ**（`areka-P0-app-icon` と同じ段取り）。この文書は、絵を選ぶ手順（要件 1）と、作るバルーンが満たすべきこと（要件 2〜7）、作り方と権利の記録（要件 8・9）、検体と既定の差し替え（要件 10・11）、確かめ（要件 12）、守ること（要件 13）を書く。

絵の作り方は brief の推し「案 A＝コードで描く」で進める（brief の Constraints「新しい依存を足さない（案 A）」）。リポジトリの中の小さい道具が、角丸の箱としっぽを α 付きで描いて書き出し、動く WebP も同じ道具で組む。寸法が 1 ピクセル単位で手本と合い、何度でも同じバイト列で作り直せ、権利が自作だけで閉じる。**絵柄の案（色・縁・角の丸み・しっぽの形・オンラインの印の絵柄と動き）はこの文書では出さない**。案は要件の討議で主のセッションが出す。

要件生成時に引き直した事実（main `932d2c30`・設計とギャップ分析はこれを再検証する）:

- 既定の名前の定数は `crates/areka/src/boot_resolve.rs` の 34 行目 `DEFAULT_BALLOON_FOLDER: &str = "StayseeBalloon"` だけで、使うのは段 6（325〜326 行目・「`DEFAULT_BALLOON_FOLDER` を参照するのはこの段だけ」）。段の順は 引数 → 記憶 → ゴーストの descript → 同梱 → 唯一 → **既定** → 無作為。
- 既定の名前に頼るテストは **6 本**（brief の挙げた 4 本＝`boot_resolve_tests.rs`・`boot_resolve_balloon_tests.rs`・`boot_config_balloon_tests.rs`・`main_config_input_tests.rs` に、`boot_config_roots_tests.rs` と `boot_config_option_tests.rs` を足す。後者は組み立てで置くだけだが、既定が外れると無作為の段の `no_pick` が panic する）。`crates/` の下で `StayseeBalloon` を綴る `.rs` は 27 ファイルで、ギャップ分析（research.md 3.1）が A＝直さないと赤（上の 6 本）・B＝緑のままだが意味は「既定」・C＝ある 1 つのバルーンの名前として使うだけ、に仕分けた。
- `tools/package.ps1` で既定を引く所は 4 か所（467 行目＝検体の窓口から取り出す・496 行目＝必須の項目・504 行目＝`balloon/` 直下の許可の表・570 行目＝写す行）。配布物の `balloon/` には `emo2-kakukaku` と `StayseeBalloon` の 2 つが入る。
- 検体の登記表 `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES` は 10 本（`StayseeBalloon` は `kind: SampleKind::Balloon`・`source: SampleSource::Vendored`）。この spec で 10 → 11。
- `StayseeBalloon` の中身を固定するテストは `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs` と `staysee_balloon_fixture/` のテーマ別 8 ファイル（`assets`・`bake`・`definition`・`faces`・`region`・`scale`・`script`・`wrapping`）。保管したファイルの名前と原寸、焼き込みで半透明の画素が残ること、定義の読み取り、描画範囲と折り返し、拡大率を固定している。
- 文書で `StayseeBalloon` を既定と書く所: 根の `README.md` 48 行目・`dist/README.txt` 379〜384 行目と 457〜460 行目（同梱物とライセンス）・`vendors/sample_ghost/README.md` 21・32 行目・`.kiro/steering/product.md` 81 行目。`structure.md` 426 行目は検体の顔ぶれ（10 本）を数える。
- 手本「ボトル（でふぉ改）」は検体 `vendors/sample_ghost/dot_sakura.nar` の同梱バルーン `bottle` として引ける（完了 `areka-P0-dot-sakura-specimen`）。
- `bottle` の `validrect.right,0` を areka は「左端から 0」と読み、文字の描画範囲が幅 0 に潰れる（`crates/areka-emo-text/src/region.rs`・完了 `areka-P0-dot-sakura-specimen` の `verification.md` 7 節と 8 節の件 3）。`shiori2-protocol` で「．さくら」が話すようになった今（2026-10-24）は「話しているのに文字が出ない」という見える崩れになっている。

## Boundary Context

- **In scope**:
  - 絵柄の案を開発者に選んでもらう段取り（要件の討議の中）。
  - 新しいバルーンの面の絵（本体側・相方側・入力の箱）と印の絵（矢印・オンライン・SSTP）。
  - UTF-8 の `descript.txt` と面別の設定、`install.txt`・`readme.txt`。
  - 絵を描く道具と作り直しの手順、権利の記録。
  - `.nar` に畳んで検体に登記すること（登記 10 → 11）。
  - 既定の差し替え（定数の値・配布の道具・文書・steering・既定の名前を期待するテスト）。
  - 中身を固定するテストと、実機の確かめ（emo2・どっとさくら）。
  - `validrect.right,0` の正典の読みを SSP で測り、このバルーンの書き方を決めること。
- **Out of scope**:
  - 印と数字を画面に出すこと、1 枚の動く絵の印の読み方（`balloon-markers`）。
  - `.webp` の名前の面を拾うこと（`balloon-webp-names`）。
  - 入力の箱を出すこと（`inputbox-user-input`・`balloon-canon-residue`）、面 1 以降の面別の設定が実際に効くこと（`balloon-canon-residue`）。
  - 縦書きの面・シェルの中のバルーンの絵・2 倍の絵。
  - バルーンの解決の段の順（変えない。定数の値だけ替える）。
  - バルーンの読み手・合成器・文字の層の直し（足りない所は起票する。`validrect` の読みの直しの扱いは議題 10）。
  - 手本「ボトル（でふぉ改）」の絵の手直しや再配布、「．さくら」の書庫の直し。
  - `StayseeBalloon.nar` の中身と、その中身を固定するテスト（残す）。
- **Adjacent expectations**:
  - 上流 `areka-P0-dot-sakura-specimen`（完了）: 手本 `bottle` を検体として引ける。登記表と数のテストを続けて触る。
  - 上流（条件つき）`balloon-webp-names`: 議題 3 で面を `.webp` の名前で持つと決めたときだけ、働きの前提になる（その場合、同 spec は v0.1.0 へ戻る）。
  - 下流 `balloon-markers`: このバルーンの矢印・オンライン・SSTP の印が、同 spec の「1 枚の動く絵の印を持つ検体」になる。印が画面に出るのは同 spec の着地の後。
  - 下流 スキル `/release`: 次の版の配布物から既定が替わる。
  - 並走（ウェーブ C12）: `real-run-log-noise` が emo-text の `region.rs` を持つ。`startup-install-option`・`bridge-store-user-root` が `dist/README.txt` の別の行を持つ。カタログの鎖（`boot_resolve.rs` を作り直す spec）とは、既定を差し替える作業を最後に置き、直前に main を取り込んで重ならないようにする。

## 議題と仮置き（要件の討議で決める）

brief が要件の段へ回した議題 9 つと、測ってから決める `validrect` の読み（議題 10）。下の要件は、答えによらず成り立つ形で書き、答えで変わる所は「（議題 N・仮置き）」と書いて brief の推しを置いた。推しが brief に無いものは「推しなし」と書いた。

- **議題 1 名前と ID**（フォルダ名・`name`・`id`・`craftman`）→ 開発者が決める。推しなし。この文書では `<名前>` と書く。
- **議題 2 絵柄** → 主のセッションが道具の下書きで案を 2〜3 個並べ、開発者が選ぶ（箱の色・縁・角の丸み・しっぽの形・瓶の透かしを無くすか areka の印を小さく入れるか・オンラインの印の絵柄・コマ数・コマの時間）。
- **議題 3 絵の形式** → 推し＝面も印も `.webp`（オンラインの印は動く WebP 1 枚）。SSP では出ないバルーンになることを承知のうえで選ぶ。`.webp` を選ぶと `balloon-webp-names` の着地が既定の差し替えの前提になる。α 付きの PNG を選べば今すぐ作れる。
- **議題 4 引き継ぐ値の線引き** → 推し＝寸法・`origin`・`validrect`・`wordwrappoint`・印と数字と入力の箱の位置はそのまま引き継ぐ。`font.name,ＭＳ ゴシック`／`font.height,12`・カーソルの色・SSTP と数字の字の色は聞いてから決める（決まるまでの仮置きは手本の値＝開発者の起票の言葉「設定系パラメーターは引き継ぐ」）。面別の設定の `windowpos.x/y` は正典の鍵でないので写さない（要件 5.7・ギャップ分析で確定）。
- **議題 5 オンラインの印を 1 枚の動く絵で持つときの書き方**（ファイル名・`onlinemarker.*` の書き方）→ 推しなし。正典は `onlinemarker.filename` の名前に 0 からの連番を足して読み、`onlinemarker.interval`（既定 500 ミリ秒・50 未満は不可）で切り替える。1 枚の動く絵で持つのは areka の拡張で、読み方は `balloon-markers` の議題（本仕様は絵を用意する側）。
- **議題 6 面をいくつ持つか** → 推し＝手本どおり本体側 4・相方側 2・入力の箱 4。
- **議題 7 `StayseeBalloon` の扱い** → 検体には残す。配布物から外すか、選べるバルーンとして残すかは推しなし。記憶に `StayseeBalloon` が残っている利用者の次の起動がどうなるかを確かめて決める。
- **議題 8 権利の書き方** → 再配布の条件（`StayseeBalloon` と同じ CC0 にするか）と置き場。推しなし。手本から引き継ぐのは数値だけで、絵は写さない。
- **議題 9 拡大率の高い画面** → 推し＝等倍の絵だけを作る（今のエンジンに 2 倍の絵を選ぶ口は無い）。
- **議題 10 `validrect.right,0` の正典の読み**（brief「どっとさくらで見えた事実」）→ 要件の討議の最初に SSP の MCP で測る（打つ前に開発者の許しを取る）。正典の読みが areka と同じなら、引き継ぐ値の側を直す。正典の読みが areka と違う（0 が右端を指す）なら、このバルーンは両方で同じ範囲に解ける書き方にし、areka の読みの直しは、`region.rs` を持つ `real-run-log-noise` の着地の後へ回す（本仕様の後ろのタスクにするか、別に起票するかを討議で決める。roadmap 棚卸㉚「`validrect.right,0` を areka の側で直すと決めたら、その足し分は次のウェーブへ回す」）。

## Requirements

### Requirement 1: 絵柄の案を出して開発者が選ぶ

**Objective:** 開発者として、areka の顔になるバルーンの絵柄を、案を見比べたうえで自分で選びたい。そうすれば、後から絵柄で作り直さずに済む。

#### Acceptance Criteria

1. When 要件の討議が絵柄を決める, the 要件の討議 shall リポジトリの絵の道具の下書きで描いた案を 2〜3 個並べて開発者に見せ、開発者が選んでから絵柄を確定とする（エージェントだけで絵柄を決めない）。
2. The 要件の討議 shall 案の絵と途中の絵をワークツリーの `target\` の下に置き、リポジトリには入れない。リポジトリに入れるのは、選んだ絵柄の絵と道具だけとする。
3. When 開発者が絵柄を選ぶ, the 要件の討議 shall この文書に選んだ絵柄の要点（箱の色・縁・角の丸み・しっぽの形・オンラインの印の絵柄とコマ数とコマの時間）と選んだ日を書き足す。
4. The 要件の討議 shall 手本「ボトル（でふぉ改）」の絵を写さず、案の絵を手本の画素から作らない（引き継ぐのは数値だけ）。

### Requirement 2: 面の絵と寸法

**Objective:** 既定バルーンで喋るゴーストを使う人として、手本と同じ大きさで、手本より簡素で、地の色によらず縁がきれいなバルーンで読みたい。そうすれば、手本に合わせて作られたゴーストに被せても見た目の大きさが変わらない。

#### Acceptance Criteria

1. The 既定バルーン shall 本体側の面 4 つ・相方側の面 2 つ・入力の箱の面 4 つを持つ（議題 6・仮置き＝手本どおり）。
2. The 既定バルーン shall 面の絵を手本と同じピクセルの寸法で持つ: 本体側の小 `balloons0`・`balloons1` は 326×169、本体側の大 `balloons2`・`balloons3` は 326×348、相方側 `balloonk0`・`balloonk1` は 326×96、入力の箱 `balloonc0`〜`balloonc3` は 400×63。
3. The 既定バルーン shall 手本と同じく、偶数の面をしっぽが右、奇数の面をしっぽが左の対とし、対の 2 枚と大小の面と相方側の面で同じ絵柄（色・縁・角の丸み）にそろえる。
4. The 既定バルーン shall 面の絵を α 付きで持ち、抜き色を使わず、`.pna` を置かない。縁の柔らかさは半透明の画素で表す。
5. The 既定バルーン shall 手本の飾り（瓶の透かし・夕日と波の絵）を持たず、手本より簡素な絵柄にする（areka の印を小さく入れるかは議題 2）。
6. The 既定バルーン shall 等倍の絵だけを持ち、2 倍の絵を持たない（議題 9・仮置き）。拡大率の高い画面では、areka が画面の拡大率に合わせて拡大する。
7. The 既定バルーン shall 入力の箱の面を、SSP と今後の areka のための絵として持つ（areka が入力の箱を出すのは本仕様の外＝`inputbox-user-input`・`balloon-canon-residue`。今の areka が出さないことは要件 7.3 で説明書に書く）。

### Requirement 3: 印の絵

**Objective:** 既定バルーンで喋るゴーストを使う人として、矢印・オンライン・SSTP の印も、面と同じ簡素な絵柄で持っていてほしい。オンラインの印は連番の 16 枚でなく 1 枚の動く絵であってほしい。そうすれば、印が出るようになったときに、作り直さずにそのまま使える。

#### Acceptance Criteria

1. The 既定バルーン shall 上と下の矢印の印を 11×10、SSTP の印を 7×9 のピクセルの寸法で、α 付きで持つ。
2. The 既定バルーン shall オンラインの印を 77×36 のピクセルの寸法の **1 枚の動く WebP** で持ち、連番の絵（`online0`〜`online15`）で持たない。
3. The オンラインの印 shall 終わりなく繰り返して動き、絵柄と動きを手本（夕日と波・16 コマ）より簡素にする（絵柄・コマ数・コマの時間は議題 2）。
4. The 既定バルーン shall 動く絵に GIF を使わない（動く絵は WebP だけ）。
5. The 既定バルーン shall オンラインの印のファイル名と `descript.txt` の `onlinemarker.*` の書き方を、議題 5 の答え（`balloon-markers` の読み方）に合わせる。
6. While `balloon-markers` が着地していない, the 既定バルーンの説明書 shall 印（矢印・オンライン・SSTP）と数字が areka ではまだ出ないことと、出るのは areka の今後の版であることを書く（どの spec で出るか＝`balloon-markers` は、本仕様の記録に書く）。
7. When 本仕様が着地した, the 本仕様 shall `balloon-markers` の brief に、このバルーンが 1 枚の動く絵のオンラインの印の検体になったこと（名前・置き場・オンラインの印のファイル名と書き方）を申し送る。

### Requirement 4: 絵の形式

**Objective:** 開発者として、絵を今の形式（開発者の言葉で「現代的なフォーマット」）で持ちたい。ただし、その選び方で既定を差し替える時期と SSP での見え方が変わることを、承知のうえで選びたい。

#### Acceptance Criteria

1. Where 議題 3 で `.webp` を選んだ, the 既定バルーン shall 面の絵と印の絵をすべて `.webp` の名前で持つ（議題 3・推し）。
2. Where 議題 3 で `.webp` を選んだ, the 本仕様 shall `balloon-webp-names` が着地して areka が `.webp` の名前の面を出せるようになるまで、既定を新しいバルーンへ差し替えない。
3. Where 議題 3 で `.webp` を選んだ, the 既定バルーンの説明書 shall このバルーンが SSP では面が出ないことを書く。
4. Where 議題 3 で α 付きの PNG を選んだ, the 既定バルーン shall 面の絵と、動かない印の絵を α 付きの `.png` の名前で持ち、オンラインの印だけを動く WebP で持つ。

### Requirement 5: 名前と設定ファイル

**Objective:** バルーンを使うゴーストの作者と利用者として、新しいバルーンの設定を手本と同じ数値で、今の文字コードで読めるようにしたい。そうすれば、手本に合わせて作られたゴーストでも文字の位置が変わらず、設定ファイルを文字化けせずに開ける。

#### Acceptance Criteria

1. The 既定バルーン shall フォルダ名と `descript.txt` の `id` を同じ `<名前>` にし、`name` と `craftman` を持つ（議題 1）。
2. The 既定バルーン shall `<名前>` を、ほかの検体のバルーン（`StayseeBalloon`・`emo2-kakukaku` とその派生・どっとさくらの `bottle` ほか）と、大小の違いを無視しても重ならない名前にする。
3. The 既定バルーン shall `descript.txt`・面別の設定（`balloons0s.txt` など）・`install.txt` を UTF-8 で書き、各ファイルの先頭で `charset,UTF-8` を名乗る（`readme.txt` も UTF-8 で書く）。
4. The 既定バルーン shall `descript.txt` に手本の全体の値のうち、`origin`・`validrect`・`wordwrappoint`・印（矢印・オンライン・SSTP）・SSTP のメッセージ・数字・入力の箱の位置を、手本と同じ数値で持つ（`validrect` は議題 10 の答えで書き方が変わる・要件 6）。
5. The 既定バルーン shall 面ごとに、手本と同じ数値の面別の設定を持つ: `balloons0s.txt`（origin 14,11・wordwrappoint.x -30・onlinemarker.x 3・sstpmarker.x 10・sstpmessage.x 20）、`balloons1s.txt`（origin 23,11・wordwrappoint.x -20・onlinemarker.x 12・sstpmarker.x 19・sstpmessage.x 29）、`balloons2s.txt`（origin 14,14・wordwrappoint.x -30）、`balloons3s.txt`（origin 23,14・wordwrappoint.x -20）、`balloonk0s.txt`（origin 14,14・wordwrappoint.x -30・validrect.bottom -14）、`balloonk1s.txt`（origin 24,14・wordwrappoint.x -20・validrect.bottom -14）。
6. The 既定バルーン shall 字の書体と大きさ（`font.name`・`font.height`）・カーソルの色・SSTP のメッセージと数字の字の書体と色を、議題 4 の答えで持つ（仮置き＝手本の値: `font.name,ＭＳ ゴシック`・`font.height,12`・字の色 0/0/0・カーソルは `mergepennot`・`square`・塗り 22/143/231・線 8/8/194・字 201/255/255・SSTP のメッセージは `ＭＳ Ｐゴシック` 12・56/163/163・数字は `Times New Roman` 14・32/76/120）。
7. The 既定バルーン shall 手本の面別の設定の `windowpos.x,0`・`windowpos.y,0` を写さない（正典の鍵の名前は `windowposition.x/y`・既定 0。`windowpos.*` は正典の鍵でなく、areka も黙って捨てる。値 0 は既定と同じなので、書かないのと同じ意味になる。research.md 3.3）。
8. When areka が既定バルーンの `descript.txt` と面 0 の面別の設定を読む, the areka shall 文字化けせずに、areka が読む鍵の値を宣言どおりに読み取り、読み込みで警告を出さない（areka がまだ読まない鍵＝印・数字・入力の箱の位置などは、今の作りどおり黙って読み飛ばす）。
9. The 既定バルーン shall `descript.txt` に `use_self_alpha,1` を書く（正典の既定は 0＝α を捨てて抜き色を使う。SSP で α どおりに見せるため。research.md 3.4）。

### Requirement 6: 文字の描画範囲が潰れない

**Objective:** 既定バルーンで喋るゴーストを使う人として、台詞の文字がバルーンの中に必ず描かれてほしい。どっとさくらの `bottle` で起きた「話しているのに文字が出ない」崩れを、新しいバルーンでは起こしたくない。

#### Acceptance Criteria

1. When 要件の討議が始まる, the 要件の討議 shall 開発者の許しを取ってから SSP の MCP で `validrect.right,0` の正典の読み（0 が左端からの 0 か、右端を指すか）を測り、測った手順・結果・ukadoc の該当の項（URL と原文の引用）をこの文書に書く。
2. The 既定バルーン shall `validrect` を、SSP と areka のどちらで読んでも同じ範囲に解け、その範囲が空でなく、`origin` と折り返し基準（`wordwrappoint`）を内に含む書き方で持つ（議題 10）。
3. If 測った正典の読みが areka の読み（0 は左端からの 0）と同じ, then the 既定バルーン shall 手本の `validrect.right,0` をそのまま写さず、手本の意図した範囲（文字を描く範囲が空でない）になる値に直して持つ。
4. If 測った正典の読みが areka の読みと違う, then the 本仕様 shall areka の読みの直しを `real-run-log-noise` の着地の後へ回し（本仕様の後ろのタスクにするか、別に起票するかは議題 10）、回した先が `.kiro/specs/` の直下に実在して `completed/` でないことを確かめてから記録する。
5. When areka が既定バルーンの面 0（本体側・相方側）を読み込む, the areka shall 「文字の描画範囲が潰れている」「origin が描画範囲の外」「折り返し基準が描画範囲の外」の警告を 1 件も出さない。
6. When 既定バルーンを被せたどっとさくらが喋る, the areka shall 台詞の文字をバルーンの中に描き、文字の描画範囲・origin・折り返し基準が、手本の数値を正典の読みで解いたものと同じ位置になる。

### Requirement 7: 説明書と配布の書き物

**Objective:** バルーンを受け取る人として、このバルーンが何で、何がまだ出ないかを、バルーンの中の説明書で嘘なく読みたい。

#### Acceptance Criteria

1. The 既定バルーン shall `install.txt`（種類がバルーンであること・入れる先のフォルダ名 `<名前>`）と `readme.txt` を持つ。
2. The 既定バルーンの `readme.txt` shall このバルーンが areka の既定バルーンであること・作り手・再配布の条件（議題 8）・寸法と設定値を手本から引き継いだこと（絵は写していないこと）を書く。
3. The 既定バルーンの `readme.txt` shall areka でまだ出ない物（印と数字・入力の箱・面 1 以降の面別の設定）を、出るようになる見込みの spec の名前を出さずに平易に書き、出るものと出ないものを取り違えない（要件 3.6 と 4.3 を含む）。

### Requirement 8: 絵の道具と作り直しの手順

**Objective:** 開発者として、絵柄を直したくなったときに、同じ手順で面と印の絵を作り直したい。そうすれば、作り方を思い出したり権利を調べ直したりせずに絵を差し替えられる。

#### Acceptance Criteria

1. The リポジトリ shall 面と印の絵（動く WebP のオンラインの印を含む）を描いて書き出す道具を持ち、選んだ絵柄の色・縁・角の丸み・しっぽの形・印の動きを道具の中の値として持つ。
2. The 絵の道具 shall 同じ入力からは同じバイト列の絵を作る（作り直して差分が出たら、入力が変わったと分かる）。
3. The 絵の道具 shall ワークスペースに新しい依存を足さず、`Cargo.lock` を変えない。
4. The 絵の道具 shall 書き出す絵の寸法を要件 2.2 と 3.1・3.2 の値に合わせ、合わない絵を書き出さない。
5. If 絵の道具が絵を書き出せない（書き出し先が無い・書けない・寸法が合わない）, then the 絵の道具 shall 理由を示して失敗し、作りかけの絵で今の絵を上書きしない。
6. The 絵の道具 shall 端末へ出す文を ASCII だけにする（どのコードページでも文字化けしない）。
7. The 手順の説明 shall 絵の道具の回し方・出力の置き場・作り直した絵から `.nar` までの手順・実機の確かめ方（要件 12.6）を書き、バルーンの置き場か検体の README から辿れる所に置く。
8. The 絵の道具とそのテスト shall それぞれ 1 ファイル 1,000 行以下にする。
9. The 絵の道具 shall 書き出した動く WebP が、areka の動く絵の読み手で読めて、要件 3.2 の寸法と、選んだコマ数とコマの時間を持つことを、テストで確かめる。

### Requirement 9: 権利の記録

**Objective:** 配布物を受け取る人として、同梱の既定バルーンが誰のもので、どんな条件で入っているかを知りたい。開発者として、手本から引き継いだのが数値だけであることを後から示せるようにしたい。

#### Acceptance Criteria

1. The 本仕様 shall 既定バルーンの絵と設定の作り手と再配布の条件（議題 8。候補は `StayseeBalloon` と同じ CC0）を、バルーンの `readme.txt` と `vendors/sample_ghost/README.md` に書く。
2. The 本仕様 shall 手本「ボトル（でふぉ改）」から引き継いだのが寸法と設定の数値だけで、絵は写していないことを、同じ 2 か所に書く。
3. The 本仕様 shall 配布の `dist/README.txt` の同梱物とライセンスの節に、新しい既定バルーンの項（名前・フォルダ・作り手・条件）を書く。
4. The 本仕様 shall `THIRD-PARTY-NOTICES.md`（依存のライセンスから作る生成物）を手で直さない。

### Requirement 10: 保管と検体の登記

**Objective:** 開発者として、新しい既定バルーンを、ほかの検体と同じ形で保管し、同じ窓口から名前で引けるようにしたい。そうすれば、配布の道具とテストが今の取り方のまま新しいバルーンを使える。

#### Acceptance Criteria

1. The 本仕様 shall 既定バルーンを検体の決まりどおり `.nar` に畳んで `vendors/sample_ghost/<名前>.nar` に置き、検体の登記表に行を足す（登記は 10 → 11・登記の数を固定するテストも合わせる）。
2. The リポジトリ shall 既定バルーンを、URL から取り寄せる形でなくリポジトリの中に持つ（配布物に必ず入る製品の資産であるため）。
3. When 検体の窓口から `<名前>` を引く, the 検体の窓口 shall 既定バルーンのフォルダを取り出して返す。
4. The 本仕様 shall `vendors/sample_ghost/README.md` の検体の一覧に、既定バルーンの行（名前・種類・ファイルの数・大きさ）を足し、`StayseeBalloon.nar` の行の「既定バルーン」の書きを、議題 7 の答えに合わせて改める。
5. The 本仕様 shall `StayseeBalloon.nar` の中身と、その中身を固定するテスト（`staysee_balloon_fixture` 一式）を変えない。

### Requirement 11: 既定の差し替え

**Objective:** バルーンを指定しないゴーストを使う人として、areka を入れたらそのまま新しい既定バルーンで喋ってほしい。開発者として、既定を書いていた所がすべて新しいバルーンを指していてほしい。

#### Acceptance Criteria

1. When バルーンの解決が段 6（既定）に至り、配布物に `<名前>` のフォルダが在る, the areka shall 新しい既定バルーンを選ぶ。
2. The 本仕様 shall バルーンの解決の段の順と、各段の当たり方を変えない（替えるのは既定の名前の値だけ）。
3. The 本仕様 shall 既定の名前に頼るテスト（research.md 3.1 の A の 6 本）と、緑のままでも「既定」の意味で綴っているテスト（同 B）を新しい既定へ改め、`StayseeBalloon` を「ある 1 つのバルーンの名前」として使っているだけのテスト（同 C）は変えない。
4. When 配布の道具が配布物を作る, the 配布の道具 shall 新しい既定バルーンを検体の窓口から引いて配布物の `balloon/<名前>/` に入れ、必須の項目と `balloon/` 直下の許可の表に `<名前>` を載せる。
5. The 配布の道具 shall `StayseeBalloon` を配布物に入れるかを議題 7 の答えに合わせ、入れないと決めたら、必須の項目・許可の表・取り出し・写す行のすべてから外す。
6. When 前回の記憶に `StayseeBalloon` が残っている利用者が新しい版の areka を起動する, the areka shall バルーンを決められずに止まることなく、何かのバルーンで喋る（議題 7 で配布物から外したときは、記憶の段が外れて既定の段で新しいバルーンに当たる）。本仕様は、この起動の結果を確かめて記録する。
7. The 本仕様 shall 根の `README.md`・`dist/README.txt`・`vendors/sample_ghost/README.md`・steering（`product.md` の既定バルーンの行・`structure.md` の検体の顔ぶれ・数を書く所・`tech.md` 166 行目の既定バルーンの例）・`doc/COMPAT_ARCHITECTURE.md` 381 行目（段の並びの「既定（`StayseeBalloon`）」）を、新しい既定バルーンを指す書きに改める。`dist/README.txt` 402 行目（`use_self_alpha` の説明で同梱の Staysee を例に出す）と `doc/pasta-real-run.md` 56〜84・160 行目（配布物と同じ形の根を組む手順）は、議題 7 の答え（`StayseeBalloon` を配布物に残すか）に合わせて改める（research.md 6.2）。
8. The 本仕様 shall 完了の spec（`areka-P0-default-balloon-bundle`・`areka-P0-default-balloon-nar-fold`）の文書を直さず、既定の役が新しいバルーンへ移ったことを本仕様と steering に書く。
9. The 本仕様 shall 既定を差し替える作業を実装の最後に置き、その直前に main を取り込む（カタログの鎖の spec が `boot_resolve.rs` を作り直すのと重ならないようにする）。

### Requirement 12: 確かめ

**Objective:** 開発者として、新しい既定バルーンの中身と見え方を、毎回のテストと、1 度の実機の確かめの両方で保証したい。そうすれば、絵や設定を作り直したときの後退にすぐ気付ける。

#### Acceptance Criteria

1. The 本仕様 shall 既定バルーンの中身を固定するテストを、`staysee_balloon_fixture` と同じ形で新しいテストファイルに足し、検体は窓口から名前で引く（在処の綴りをテストに持たない）。
2. The 中身を固定するテスト shall 保管したファイルの名前の集合と、面と印の絵の寸法（要件 2.2・3.1・3.2）を固定する。
3. The 中身を固定するテスト shall 面 0 を本体側と相方側の両方で焼き込み、半透明の画素が残ること（完全な不透明・完全な透明・半透明の 3 種がどれも在ること）を固定する。
4. The 中身を固定するテスト shall `descript.txt` と面 0 の面別の設定から読んだ値が、要件 5.4〜5.6 で書いた値と同じであること、文字の描画範囲・origin・折り返し基準が要件 6.2 の範囲に解けることを固定する。
5. The 中身を固定するテスト shall 既定バルーンに半角だけ・全角だけ・混ぜた本文を流し込み、折り返し基準を超えたら折り返し、描画範囲を 1 画素も超えずに描くことを、拡大率 1 と 1 でない値（既存の検体で使っている値）の両方で固定する。
6. When 中身を固定するテストが緑になった, the 開発者 shall 実機で、emo2 と、既定バルーンを被せたどっとさくら（引数かメニューで被せる）が喋る様子を、拡大率 100% と 100% でない値の 2 通りで 1 度ずつ見て、文字がバルーンの中に描かれ、欠けたりはみ出したりしないことを確かめる。
7. When 実機で確かめる, the 本仕様 shall 台本の面の切り替え（`\b[2]` など）で本体側の大の面へ切り替えたときの見え方を見て、面 1 以降の面別の設定が効かないことによる崩れが在れば記録し、起票する（議題 6 の注）。
8. The 本仕様 shall 実機の確かめの記録（使ったゴースト・拡大率・日付・見えたこと・撮った画像か観察の記録）を本仕様の `verification/` の下に残す。
9. If 確かめで崩れが見つかった, then the 本仕様 shall バルーンの絵か設定の誤りなら本仕様で直し、areka の側の欠陥なら境界の外として `/kiro-discovery` で起票するか、生きている spec（`.kiro/specs/` の直下に実在し `completed/` でない）に申し送り、どちらにしたかを記録する。

### Requirement 13: 守ること

**Objective:** 開発者として、並走する spec と既存のテストを壊さずに、新しい既定バルーンを着地させたい。

#### Acceptance Criteria

1. The 本仕様 shall 本番コード（`crates/*/src/` のテストでないファイル）の変更を、既定の名前の定数の値 1 つと検体の登記表 `crates/sample-ghost-kit/src/lib.rs` の行に限る（議題 10 で areka の読みの直しを本仕様の後ろのタスクにすると決めたときだけ例外とし、その場合は何を・なぜ変えたかを記録する）。
2. The 本仕様 shall `crates/areka/src/boot_resolve.rs` の書き込みの関数と `crates/areka-ghost/src/catalog.rs` に触らない。
3. While ウェーブ C12 が並走している, the 本仕様 shall emo-text の `region.rs` に触らない（`real-run-log-noise` の持ち物）。
4. The 本仕様 shall バルーンの読み手・合成器・文字の層を直さない（足りない所は起票する）。
5. The 本仕様 shall 新しい依存を足さない。
6. The 本仕様 shall 足す・触るファイルをそれぞれ 1,000 行以下に保つ（1,000 行の番人が緑）。
7. When 本仕様の変更を取り込んだ, the ワークスペース shall 全体テストが緑になる。
8. The 本仕様 shall 実機の根・検体の取り出し先・一時のファイルを、ワークツリーの `target\` の下にだけ置く。
9. The 本仕様 shall SSP で測る前に、毎回開発者の許しを取る。

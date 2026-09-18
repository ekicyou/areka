# Requirements Document

> 起票 2026-09-12・要件生成 2026-09-18。本文の数値は **2026-09-18 に本ブランチで数え直した実測値**（brief の 09-12 値と一致した項目はそのまま、変わった項目は「（09-18 実測）」と示す）。正典の引用は ukadoc の該当ページ（`manual_install`・`manual_directory`・`descript_install`）を指す。

## Introduction

### 誰が困っているか

areka の開発者と、実機サインオフを回す全員。加えて、本仕様の後に続く α の仕様（`shell-implicit-surface`・`baseware-root-layout`・`default-balloon-bundle`・`ghost-install`・`network-update`）の実装者。

### いま何が起きているか（2026-09-18 実測）

- 本番アプリが起動できる唯一のゴースト **emo2** は、別クレートの example のテスト検体として **展開済みのまま** `crates/pilot/examples/shiori-host-32/fixtures/emo2/`（追跡 **110** ファイル＝ゴースト本体 90＋同梱バルーン `emo2-kakukaku` 20・6.6 MB。brief の「150」は `fixtures/` 全体の数）に置かれている。隣に検証用の派生バルーン 2 つ（`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`・各 20 ファイル）がある。里々の標準テンプレート **R_POST_and_KOMAINU**（追跡 **43** ファイル・1.7 MB・全ファイル Shift_JIS）は `vendors/sample_ghost/R_POST_and_KOMAINU/` に展開形で追跡されている。**リポジトリに `.nar` は 1 つも無い**（`vendors/pasta/` の `hello-pasta.nar` は pasta 側の資産で対象外）。
- **検体が走行で汚れ、次の走行の挙動が変わる。** 起動時にゴースト側とシェル側の `profile/areka/` へ永続化ファイル（窓位置と起動回数）が書かれる。起動回数の記録があると初回イベント `OnFirstBoot` が飛ばされて `OnBoot` になるので、同じ検体で 2 回走らせると別の結果になる。いま `fixtures/emo2/ghost/master/profile/areka/sylphya.toml` が実在している（無視対象のため追跡外）。
- **その場しのぎの初期化が 1 か所だけある。** `crates/areka/src/emo2_boot/spine.rs` の `emo2_root()` は呼ばれるたびに `<root>/ghost/master/profile/areka` を丸ごと消している。他の参照箇所は消していない。
- **無視されていない書き込み先が残っている。** 起動はシェル側 `shell/master/profile/areka/` にも書く経路を持つが、example の `.gitignore` が無視するのはゴースト側 `fixtures/emo2/ghost/master/profile/` だけ。シェル側の書き込みが初めて起きた日に追跡外ファイルが湧く（本日時点では未発生・潜在）。
- **参照は 38 ファイル**（src 内の単体テスト 21・`tests/` 8・`examples/` 9・**本番コード 0**。数え方: 検体パスを**組み立てるコード**を持つ `.rs`。コメントだけで言及する 11 ファイルは含めない）。私家版の `emo2_root()` が **12 定義**、`const FIXTURE_DIR` が 1、直書きが 25。綴りは一括 `join` と分割 `join` の 2 種。**共有の窓口は 1 つも無い。** 同梱バルーンは `emo2_root()` の下に `emo2-kakukaku` を継ぎ足して指されている（`.rs` で 45 ファイルが `emo2-kakukaku` を綴る）。コード以外にも、`tools/perf/` のスクリプト 6 本・`doc/` の文書 3 本・example の README 1 本が同じパスを綴っている。
- **`install.txt` を読む実装は皆無。** `areka-parsers/src/package/resolve.rs` は前置きで「`install.txt` / balloon 系 / NAR には触れない」と自ら宣言し、`package/validation_tests.rs` はそれが解決結果へ漏れないことを固定している。アーカイブ展開器も無い。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `descript_install` ページ 15 項目は全て `absent`・`owner = ""`・優先度 `B1`。
- **検体の `install.txt` は 4 通りの綴りを持つ。** emo2 は `Charset,UTF-8`（キーの頭が大文字）・`type,ghost`・`directory,emo2`・`balloon.directory,emo2-kakukaku`・`balloon.source.directory,emo2-kakukaku`。派生バルーン 2 つは `charset,UTF-8`・`type,balloon`。R_POST_and_KOMAINU は `charset, Shift_JIS`（カンマの後ろに空白）・`type,ghost`・`directory,R_POST_and_KOMAINU`。
- **バイト保存の手当ては片側だけ。** `vendors/sample_ghost/` には `.gitattributes`（`* -text`）と `.gitignore`（`!*_test.txt` `!*_dump.txt`）があり `git check-attr text` は `unset`。`fixtures/emo2/` は `unspecified`＝無防備（ルートに `.gitattributes` は無い）。ルート `.gitignore` の `*_test.txt` は Windows の大小無視で `dic09_Test.txt` のような辞書を黙って落とす。
- 展開先に使う `target/` はルート `.gitignore` の 1 行目で無視されており、ワークスペース走査 3 種（1,000 行の番人・ログの番人・網羅調査）も `target` を除外する。

### 何を変えるか

3 段で進める。① 38 ファイルの検体参照を **1 つの窓口**（検体名を受け取って根を返す）へ寄せる（保管形は変えず挙動不変）。② **NAR エンジン**（本番クレート `areka-nar`）を新設し、`.nar` の読取・`install.txt` の解釈・安全な展開・開発用の根の管理を持たせる。③ 窓口を「無ければ `.nar` から展開」へ切り替え、検体 5 つを `vendors/sample_ghost/*.nar` に畳んで展開済みツリーを削除する。完了時、リポジトリが保管するのは `.nar` だけになり、テストと実機走行は `target/` 配下に展開された使い捨ての根を使う。走行のたびに起動記録の無い根が手に入るので、汚染は構造的に起こらなくなり、`spine.rs` の初期化は不要になる。展開先の形はベースウェアの根の形（`<根>/ghost/<名>/`・`<根>/balloon/<名>/`＝ukadoc「全体の構成」）に揃え、後続の `baseware-root-layout` がその根をそのまま受け取れるようにする。

## Boundary Context

- **In scope**（開発者・後続仕様の実装者から見える範囲）:
  - 検体の窓口 1 つ（検体名 → 展開済みの根／根そのもの）と、38 ファイルの収束。
  - NAR エンジン: アーカイブ読取（`.nar`＝zip 形式・`.zip` も同じ扱い）・ファイル名の文字コード決定・`install.txt` の解釈（`type`・`name`・`directory`・`charset`・`accept`・`refresh`・`refreshundeletemask`・`*.directory`・`*.source.directory`・`*.refresh`・`*.refreshundeletemask`）・パス安全性・配布形 → インストール済み形への展開・既存フォルダへの再インストール時の `refresh` の意味論。
  - 開発用の根の管理（`target/` 配下・原子的な確定・陳腐化の検出・自己修復・並走安全）。
  - 検体 5 つの `.nar` 化（`emo2`・`R_POST_and_KOMAINU`・`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`・`StayseeBalloon`〔存在するとき〕）・`vendors/sample_ghost/` への集約・畳む手順の文書化・展開済みツリーと `spine.rs` の初期化と example 側 `.gitignore` の削除。
  - 検体パスを綴るスクリプト・文書の追随（`tools/perf/` 6 本・`doc/` 3 本・example README 1 本・実機サインオフの手順の記述）。
  - 外部依存 1 本（アーカイブ読取）の追加登記（`tech.md`）・ライセンス検査・`THIRD-PARTY-NOTICES.md` の再生成・`structure.md` のクレート一覧・網羅台帳の `owner` 登記と `roadmap-draft.md` の追随。
  - 決定論テスト（固定の小さな `.nar` で受理・拒否・配置・再インストールの全分岐）と、emo2 での実機一周。
- **Out of scope**:
  - `.nar` の作成（書き込み側）・配布物の生成。検体を `.nar` に畳む作業は製品機能にしない（手順は文書に残す＝Requirement 10.10）。
  - 利用者が投げた `.nar` を受け取る入口（D&D・ファイル選択・台本・SHIORI イベント `OnInstall*`・`accept` の照合）＝`areka-P0-ghost-install`。本仕様は `accept` の値を読んで渡すところまで。
  - `updates2.dau`・ネットワーク更新・`delete.txt`（`descript_install` ページの「相対パス」）＝`areka-P0-network-update`。`developer_options.txt`（同ページの「相対パス,オプション1,…」「相対パス,ignore」）＝作る側の機能（起票なし）。
  - ベースウェアの根の場所の決定（exe の隣・環境変数）・インストール済みのゴースト／バルーンの列挙・既定バルーンの選択＝`areka-P0-baseware-root-layout`。本仕様は根の**形**だけを揃える。
  - 検体の内容変更（emo2 も R_POST_and_KOMAINU も 1 バイトも変えない）・派生バルーンの作り直し・`StayseeBalloon` の入手（`areka-P0-default-balloon-bundle`）。
  - `type` が `plugin`・`headline`・`language`・`calendar skin`・`calendar plugin`・`calendar`・`package` のアーカイブの展開（理由付きで拒否するところまでが本仕様）。
  - 起動時に永続化ファイルを書く場所（`profile/areka/`）の再設計。本仕様は「根が毎回新品」で汚染を断ち、書く場所は変えない。
- **Adjacent expectations**:
  - **前提**: 完了仕様 `areka-P0-charset-canon`（文字コードの復号の既存層）・`areka-P0-test-cage-determinism`（テスト用一時パスの窓口と dev-only の規律）。`vendors/sample_ghost/R_POST_and_KOMAINU/` が展開形で在ること。
  - **読み手（下流）**: `baseware-root-layout`（窓口が返す根をそのまま「根」として受ける）・`shell-implicit-surface`（R_POST_and_KOMAINU の `shell/master/` を窓口経由で受ける）・`default-balloon-bundle`（保管を展開フォルダで行い、`.nar` 化は本仕様が引き受ける。検体名 `StayseeBalloon` の登記は 1 行）・`ghost-install`／`network-update`（エンジンをそのまま呼ぶ・二度実装しない）・実機サインオフを持つ全仕様（手順の「検体の絶対パス」の得方が変わる）。
  - **並走**: A0 で `popup-menu-minimal`・`default-balloon-bundle` と並走する。本仕様が書き換える既存の**コード**は検体パスを参照する 38 ファイル（と `emo2-kakukaku` を継ぎ足す同じ集合内のファイル）に限り、並走側はそれらに触らない約束（roadmap 追記(100)）。steering・網羅台帳・`roadmap.md`・`THIRD-PARTY-NOTICES.md` は全仕様が触る合流点であり、完了時に `main` を取り込んで数え直す（既存の完了手順どおり）。
  - **本仕様の In に無く隣接仕様が本仕様のものと明記している項目**: `refresh`／`refreshundeletemask` の実行の意味論（Requirement 6）は、brief の In 列挙には無いが、`ghost-install` の brief が「`areka-nar` が持つ」と列挙し、本仕様の brief が「エンジンをそのまま昇格して使う（二度実装しない）」と定めるため本仕様に置く。要件ディスカッションで確認する。

## Requirements

### Requirement 1: 検体の窓口——検体名から根を得る唯一の入口

**Objective:** テスト・example・実機手順の書き手として、検体を「名前」で指定して使える根を受け取りたい。そうすれば、保管形式を変えるときに 1 か所を直せば済み、38 か所を追いかけ回さなくてよい。

#### Acceptance Criteria

1. The 検体の窓口 shall 検体名（`emo2`・`R_POST_and_KOMAINU`・`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`、および Requirement 8.5 の条件で `StayseeBalloon`）を受け取り、その検体がインストール済み形で置かれた**ゴーストまたはバルーンのフォルダ**の絶対パスを返す。
2. The 検体の窓口 shall 検体名を受け取り、その検体を含む**ベースウェアの根**（`<根>/ghost/`・`<根>/balloon/` を直下に持つフォルダ）の絶対パスも返せる。
3. The 検体の窓口 shall ゴーストの検体名と、そのゴーストが同時にインストールしたバルーンの `*.directory` 名（emo2 なら `emo2-kakukaku`）を受け取り、そのバルーンがインストール済み形で置かれたフォルダ（`<根>/balloon/<名>/`）の絶対パスを返す（同梱バルーンを指す既存の参照も段 ① で窓口へ寄せ、段 ③ で二度書き換えない）。
4. If 登録されていない検体名、または検体が同時にインストールしていないバルーン名を受け取った, then the 検体の窓口 shall 既知の名前の一覧を含む理由付きの失敗を返す（黙って空のパスを返さない）。
5. The 検体の窓口 shall 検体を 1 つ足す作業を「`.nar` を 1 つ置く」と「名前を 1 行登記する」の 2 手に収める（検体ごとの専用関数を増やさない）。
6. When 段 ①（収束）が完了した, the ワークスペース shall 検体パスを綴る 38 ファイル（src 内単体テスト 21・`tests/` 8・`examples/` 9）の全てが窓口経由になっており、テスト名の集合と合否が段 ① の前と同一で、差分がパスの得方の行に限られる（挙動不変）。
7. The 本番コード（テスト・example 以外） shall 段 ① の前と同じく検体の窓口を呼ぶ箇所が 0 のままである（本番アプリは検体の在処を知らない）。
8. The ワークスペース shall 次の綴りが窓口の定義ファイルの外の `.rs` に現れないことを、常時走る検査で判定する: 旧置き場 `shiori-host-32/fixtures`・展開形の `vendors/sample_ghost/<検体名>/`・展開先の名前空間フォルダ名。検査はコメント行を除いた実行行を見る（既存の番人と同じ）。数を印字するだけでなく 1 件でも現れたら赤になり、既知の 1 件を足すと赤になることを較正で確かめる。文書・スクリプトが `.nar` の保管場所 `vendors/sample_ghost/*.nar` を名指しすることは禁じない。旧置き場をコメントだけで言及する 11 ファイルは、段 ③ で実体が消えるため同時に書き換える。
9. The 開発者 shall 1 つのコマンドで、指定した検体の展開済みの根と検体フォルダ（同梱バルーンがあればそのフォルダも）の絶対パスを標準出力に得られる（実機走行は絶対パス起動が定石のため）。

### Requirement 2: NAR コンテナの読取とファイル名の文字コード

**Objective:** 開発者として、日本語ゴーストの `.nar` をそのまま読み取ってほしい。そうすれば、配布されている形のまま検体を保管でき、SSP と同じ入力で試験できる。

#### Acceptance Criteria

1. The NAR エンジン shall 拡張子 `.nar` と `.zip` のファイルを同じ手順で読む（ukadoc「インストール」: nar は zip の拡張子を変えただけのもの）。
2. When アーカイブ内のエントリ名に「名前は UTF-8」の印（汎用目的ビット 11）が立っている, the NAR エンジン shall その名前を UTF-8 として復号する。
3. When 印が立っていない, the NAR エンジン shall その名前を Shift_JIS（CP932）として復号する（日本語ゴーストの配布物の慣行。CP437 とは解釈しない）。
4. If エントリ名が指定の文字コードで損失なく復号できない, then the NAR エンジン shall アーカイブ全体を拒否し、理由にエントリの番号と名前の生バイト列（16 進）を含める（置換文字で黙って通さない）。
5. If エントリが暗号化されている、または対応外の圧縮方式である, then the NAR エンジン shall アーカイブ全体を拒否し、理由に方式とエントリ名を含める。
6. If アーカイブが壊れている（中央ディレクトリが読めない・エントリの整合性検査に失敗する）, then the NAR エンジン shall 理由付きで拒否し、書き込みを 1 バイトも行わない。
7. The NAR エンジン shall アーカイブ内のシンボリックリンクのエントリを実ファイルシステム上のリンクとして決して作らず、そのようなエントリを含むアーカイブを理由付きで拒否する。

### Requirement 3: `install.txt` の解釈

**Objective:** 後続の `ghost-install` の実装者として、`install.txt` の全キーが 1 か所で読まれ、意味の分かる形で受け取れてほしい。そうすれば、製品側のインストーラは判断（`accept` の照合・イベント送出）だけを書けばよく、解釈を二度実装しない。

#### Acceptance Criteria

1. The NAR エンジン shall アーカイブの**最上位**にある `install.txt` を読む（正典 `manual_install`: 「その一番上のディレクトリに install.txt を置く」）。
2. If 最上位に `install.txt` が無い, then the NAR エンジン shall 理由付きで拒否し、理由に最上位のエントリ名の一覧を含める（1 段の包みフォルダを黙って剥がすことはしない。正典は作る側に「フォルダの階層を深くしてしまわないように注意」と求めている）。
3. The NAR エンジン shall `install.txt` の文字コードを、ファイル内の `charset,<名前>` 行（キーの ASCII 大小は区別しない・値の前後の空白は無視する。検体 emo2 は `Charset,UTF-8`、R_POST_and_KOMAINU は `charset, Shift_JIS` と綴る）から決め、行が無ければ既存の文字コード層の ANSI 既定（Shift_JIS＝CP932 の固定写像。本番の起動設定が charset 未宣言時に使う既定と同じ）で読む（正典 `descript_install#charset`: 省略時は OS の標準設定）。
4. The NAR エンジン shall `install.txt` を既存の `key,value` 層と同じ規則（最初のカンマで分割・前後空白を除去・同一キーは後勝ち・カンマの無い行は無視）で読む。
5. The NAR エンジン shall `type` の値が `ghost`・`shell`・`supplement`・`balloon` のいずれかであるアーカイブを受理する（正典 `descript_install#type`）。
6. If `type` が `plugin`・`headline`・`language`・`calendar skin`・`calendar plugin`・`calendar`（旧仕様＝calendar skin と同義）・`package` またはそれ以外の値である、あるいは `type` 行が無い, then the NAR エンジン shall 理由付きで拒否し、理由に読み取った `type` の値（無ければ「無し」）を含める（製品側が `OnInstallFailure` の理由に写せる形）。
7. If `directory` 行が無い、または値が空である, then the NAR エンジン shall 理由付きで拒否する（正典 `descript_install#directory`: 省略不可・`type,package` のみ不要）。
8. If `name` 行が無い、または値が空である, then the NAR エンジン shall 理由付きで拒否する（正典 `descript_install#name`: 省略不可。検体 5 つは全て `name` を持つ）。
9. If `directory`・`*.directory`・`*.source.directory` の値がパス区切り（`/` `\`）・`..`・絶対パスの形・NUL・Windows で使えない文字を含む, then the NAR エンジン shall 理由付きで拒否する（フォルダ名は 1 階層の名前でなければならない）。
10. The NAR エンジン shall `name` の値を結果に含めて呼び出し側へ返す（後続の `OnInstallComplete` の Reference1 に使う）。
11. The NAR エンジン shall `accept` の値を結果に含めて呼び出し側へ返し、照合はしない（照合は `ghost-install` の責務。正典 `descript_install#accept`）。
12. While `type` が `ghost` または `shell` である, the NAR エンジン shall `balloon.directory` と `balloonN.directory`（N は 0 以上の整数）を「同時にインストールするバルーン」として読み、対応する `*.source.directory`（無ければ `*.directory` と同じ値）をアーカイブ内の取り出し元フォルダ名とする（正典 `descript_install#*.directory`・`#*.source.directory`）。
13. If `*.directory` の `*` が `balloon` 以外（`headline`・`plugin`・`calendar.skin`・`calendar.plugin`）である, then the NAR エンジン shall 警告を記録してその同時インストール指定を読み飛ばし、本体（ゴーストまたはシェル）の展開は続行する。
14. If `type` が `balloon` または `supplement` であるのに `*.directory` が書かれている, then the NAR エンジン shall 警告を記録して読み飛ばす（正典: `type` が `ghost` か `shell` の場合にのみ設定可能）。
15. The NAR エンジン shall `refresh`・`refreshundeletemask`・`*.refresh`・`*.refreshundeletemask` の値を読み、Requirement 6 の再インストールの意味論に渡す（`refreshundeletemask` はコロン区切りのファイル名の列・パス指定不可）。
16. The NAR エンジン shall 上記以外のキー（`bootghost` を含む）を無視し、無視したキー名を記録する（`bootghost` は `type,package` 専用であり、`package` は 3.6 で拒否される）。

### Requirement 4: パス安全性

**Objective:** 開発者として、悪意ある・壊れたアーカイブを渡しても展開先の外へ 1 バイトも書かれないでほしい。そうすれば、利用者から受け取る `.nar` を製品側で扱うときも同じエンジンを信用できる。

#### Acceptance Criteria

1. The NAR エンジン shall 全エントリの名前の検証を**書き込みの前に**終え、1 件でも拒否があればアーカイブ全体を拒否して展開先に 1 バイトも書かない（部分的に展開された木を残さない）。
2. If エントリ名が絶対パス（`/` 始まり）・ドライブレター付き（`C:` 等）・UNC（`\\` 始まり）である, then the NAR エンジン shall 拒否する。
3. If エントリ名のいずれかの区切り要素が `..` である, then the NAR エンジン shall 拒否する。
4. If エントリ名が NUL 文字を含む, then the NAR エンジン shall 拒否する。
5. If エントリ名の区切りに `\` が使われている, then the NAR エンジン shall 拒否する（zip の規格上の区切りは `/`）。
6. If 復号後のエントリ名が Windows のファイルシステムで作れない名前である（`<>:"|?*`・制御文字・末尾のドットや空白・`CON`/`PRN`/`AUX`/`NUL`/`COM1`〜`9`/`LPT1`〜`9` の予約名）, then the NAR エンジン shall 理由付きで拒否する（作成に失敗して途中で止まるのではなく、書き込み前に止める）。
7. If 復号後のエントリ名が大文字小文字だけ異なる別のエントリと衝突する, then the NAR エンジン shall 理由付きで拒否する（Windows では同じファイルに書かれ、後勝ちで一方が消える）。
8. The NAR エンジン shall 展開先のフォルダの外へ解決されるパスを決して作らず、確定後の各ファイルの実パスが展開先の配下にあることを検査で確かめられる。
9. The NAR エンジン shall フォルダのエントリ（末尾 `/`）を空フォルダとして作り、それ以外の外部属性（実行ビット・所有者）は無視する。

### Requirement 5: 配布形からインストール済み形への展開

**Objective:** 後続の `baseware-root-layout` の実装者として、展開先が ukadoc「全体の構成」のベースウェアの根の形になっていてほしい。そうすれば、開発用の根と製品の根を同じコードで扱える。

#### Acceptance Criteria

1. The NAR エンジン shall 呼び出し側から「ベースウェアの根」を受け取り、その配下へ展開する（根の場所を自分で決めない）。
2. While `type` が `ghost` である, the NAR エンジン shall アーカイブ最上位の内容（`install.txt`・`readme.txt`・`ghost/`・`shell/`・その他のファイル）を `<根>/ghost/<directory>/` へ置き、同時インストールのバルーンの取り出し元フォルダは除く。
3. While `type` が `ghost` または `shell` で同時インストールのバルーンが指定されている, the NAR エンジン shall アーカイブ内 `<*.source.directory>/` の内容を `<根>/balloon/<*.directory>/` へ置き、ゴーストのフォルダ側には複製を残さない（ukadoc「全体の構成」: バルーンはベースウェアのバルーン格納フォルダに統括される。複製を残さないのは本仕様の決定＝二重に数えられず容量も増えない）。取り出し元フォルダの中に `install.txt` があっても無視し、拒否の理由にしない（正典 `manual_install`: 「同梱される側には install.txt 不要。あっても無視される」）。
4. If 同時インストールのバルーンの取り出し元フォルダがアーカイブ内に無い, then the NAR エンジン shall 理由付きで拒否する。
5. While `type` が `balloon` である, the NAR エンジン shall アーカイブ最上位の内容を `<根>/balloon/<directory>/` へ置く。
6. While `type` が `shell` である, the NAR エンジン shall 呼び出し側から宛先ゴーストのフォルダ名を受け取り、アーカイブ最上位の内容を `<根>/ghost/<宛先>/shell/<directory>/` へ置く（`accept` から宛先を求めるのは呼び出し側）。
7. If `type` が `shell` または `supplement` で、宛先ゴーストのフォルダ名が渡されない、または `<根>/ghost/<宛先>/` が無い, then the NAR エンジン shall 理由付きで拒否する。
8. While `type` が `supplement` である, the NAR エンジン shall アーカイブ最上位の内容を `<根>/ghost/<宛先>/` へ重ねて置く（ukadoc「インストール」: フォルダ構造はそのまま、追加するファイルのみ）。ただし最上位の `install.txt` は重ねない（本仕様の決定: 重ねるとゴースト自身の `install.txt` が `type,supplement` の物に置き換わり、ゴーストの素性が壊れる。正典はこの点に触れていない）。
9. The NAR エンジン shall 展開したファイルの内容がアーカイブ内のバイト列と一致することを保証し、改行や文字コードの変換を行わない。
10. When 展開が成功した, the NAR エンジン shall 結果として「種別・`name`・`accept`・置いたゴーストまたはバルーンのフォルダの絶対パス・同時インストールしたバルーンのフォルダの絶対パスの列・読み飛ばしたキーの列」を返す。
11. If 展開が途中で失敗した, then the NAR エンジン shall 宛先に書きかけの木を残さず、以前から在った宛先の内容も壊さない（利用者からは「全部入った」か「何も変わっていない」のどちらかしか見えない）。

### Requirement 6: 既存のフォルダへの再インストール（`refresh`）

**Objective:** 後続の `ghost-install`・`network-update` の実装者として、同じゴーストの新しい `.nar` を上から入れたときの正典の振る舞いをエンジンが持っていてほしい。そうすれば、製品側は呼ぶだけで済む。

> 本要件は brief の In 列挙に無い（Boundary Context「本仕様の In に無く隣接仕様が本仕様のものと明記している項目」）。要件ディスカッションで「本仕様に置く」か「値を読んで返すまで（3.15）に留め、実行は `ghost-install` へ送る」かを確認する。

#### Acceptance Criteria

1. While 宛先フォルダが既に存在し `refresh` の値が `1` でない, the NAR エンジン shall 既存のファイルを残し、アーカイブに含まれる同名ファイルだけを上書きする（正典 `descript_install#refresh`: 1 以外は refresh 無効）。
2. While 宛先フォルダが既に存在し `refresh,1` である, the NAR エンジン shall 宛先フォルダの中身を全て消してから展開する。ただし `refreshundeletemask` に列挙されたファイル名（コロン区切り・全ての階層で同名を対象）は消さない（正典 `descript_install#refreshundeletemask`）。
3. While 同時インストールのバルーンの宛先が既に存在する, the NAR エンジン shall `*.refresh`・`*.refreshundeletemask` を同じ規則でバルーン側に適用する。
4. If 再インストールの途中で失敗した, then the NAR エンジン shall 失敗した旨と原因を理由に返し、消した・上書きした範囲を記録に残す（`refresh,1` の削除後の失敗は元に戻せないため、少なくとも何が起きたかが分かること）。
5. The 開発用の根（Requirement 7）shall 再インストールの経路を通らない（毎回新しい宛先へ展開する）。再インストールの検証は Requirement 9 の決定論テストで行う。

### Requirement 7: 開発用の根の管理——使い捨て・新品・自己修復・並走安全

**Objective:** 開発者として、テストや実機走行のたびに起動記録の無い新品の検体で始めたい。そうすれば、`OnFirstBoot` と `OnBoot` の違いのような「前の走行の残り」で結果が変わることが構造的に起こらない。

#### Acceptance Criteria

1. The 検体の窓口 shall 展開先を `target/` 配下の**本仕様専用の名前空間**（`target/debug/` の直下ではない）に作る（`cargo clean` で消える・ビルド成果物と取り違えが起きる `target/debug/` を避ける）。
2. When 検体名に対応する展開が無い、または不完全である, the 検体の窓口 shall `vendors/sample_ghost/<検体>.nar` から展開してから根を返す（事前手順を人に覚えさせない）。
3. When `.nar` の内容が前回の展開時から変わった, the 検体の窓口 shall 古い展開を捨てて新しく展開する（変わったことを窓口自身が検出し、人が消して回らない）。
4. The 検体の窓口 shall 窓口を経由して取得するたびに、含まれるゴーストが**起動記録を持たない**根を返す（ゴースト側・シェル側の `profile/` に前の走行の永続化ファイルが無い）。「窓口から取得 → 起動 → 終了」を 2 回続けて行うと、2 回とも `OnFirstBoot` から始まる。
5. While 1 つの走行の中で複数の利用者（同じプロセスの並走テスト・別プロセスのテストバイナリ）が同じ検体を要求している, the 検体の窓口 shall ある利用者の起動が書いた永続化ファイルが他の利用者の受け取る根に現れないことと、利用中の木が別の利用者の要求によって差し替えられたり消されたりしないことの両方を保つ（利用者ごとに別の複製を渡すか、共有の木を利用中に差し替えないかは設計で決める）。
6. While 複数のプロセスが同時に同じ検体の展開を始めた, the 検体の窓口 shall いずれのプロセスも完全な木を受け取り、壊れた木や半端な木が残らない。
7. If 展開の途中でプロセスが落ちた, then the 検体の窓口 shall 書きかけの木を「使える根」として返さず、次の呼び出しで自己修復する。
8. The 検体の窓口 shall `target/` 配下に置いた展開物のいずれにも「在り続けること」を要求しない（消されたら次の呼び出しで作り直す）。
9. The 検体の窓口 shall 繰り返しの走行で `target/` 配下の使用量が際限なく増えないようにする（前の走行の使い捨ての複製は回収されるか、上書きされる）。
10. The 検体の窓口 shall 一時ファイルの入口（OS の一時フォルダ）を使わない（テスト用一時パスの窓口の迂回検知に触れない・`target/` の名前空間だけを使う）。
11. When 完了後に `cargo test --workspace` と emo2 の実機一周を行った, the リポジトリ shall `git status` に追跡外ファイルを 1 つも増やさない（シェル側の永続化ファイルも含む）。

### Requirement 8: 検体の保管形——`vendors/sample_ghost/*.nar` だけを追跡する

**Objective:** 開発者として、検体は配布形（`.nar`）のまま 1 か所に置きたい。そうすれば、本番アプリが起動する唯一のゴーストが別クレートの example の 5 階層下に間借りしている状態が解消され、検体を増やす手順が「`.nar` を置く」だけになる。

#### Acceptance Criteria

1. When 段 ③ が完了した, the リポジトリ shall 検体として `vendors/sample_ghost/emo2.nar`・`R_POST_and_KOMAINU.nar`・`emo2-kakukaku-offsetdpi.nar`・`emo2-kakukaku-wplimit.nar`（および 8.5 の `StayseeBalloon.nar`）だけを追跡し、`crates/pilot/examples/shiori-host-32/fixtures/` と `vendors/sample_ghost/R_POST_and_KOMAINU/` の展開済みツリーを追跡しない。
2. The `emo2.nar` shall 現在の展開済みツリーの追跡ファイル全て（110 ファイル＝ゴースト本体 90＋同梱バルーン `emo2-kakukaku` 20）を、`install.txt` の意味論どおり配布形（ukadoc「インストール」の「ゴーストにバルーンなどを同梱する場合」の構造）で含み、追跡外の `profile/` は含めない。
3. The `R_POST_and_KOMAINU.nar` shall 現在の展開済みツリーの追跡ファイル全て（43 ファイル・`dic09_Test.txt` のような大小違いの名前を含む）を含む。派生バルーン 2 つの `.nar` は各 20 ファイルを含む。
4. When 各 `.nar` を NAR エンジンで展開した, the 展開結果 shall 畳む前の追跡ツリーと**ファイル集合とバイト列が完全に一致**する。emo2 は「ゴーストのフォルダ `ghost/emo2/`（90）と同時インストールしたバルーンのフォルダ `balloon/emo2-kakukaku/`（20）の和」を、畳む前の `emo2/emo2-kakukaku/…` → `balloon/emo2-kakukaku/…` の写像のもとで突き合わせる。検証の記録として、検体ごとのファイル数とハッシュを本仕様の検証報告に残す。
5. Where 段 ③ の時点で `vendors/sample_ghost/StayseeBalloon/` が展開形で存在する（`default-balloon-bundle` の成果物）, the 本仕様 shall それも `StayseeBalloon.nar` に畳んで展開形を追跡から外し、窓口に `StayseeBalloon` を登記する。存在しなければ登記せず、その旨と「畳むのは `default-balloon-bundle` が Requirement 10.10 の手順で行う」ことを検証報告と同仕様の brief に書く。
6. The `vendors/sample_ghost/` shall `.nar` をバイト保存（改行変換なし）で追跡する属性を持ち、リポジトリの無視規則によって検体の一部が黙って落ちないことを、`.nar` の中身の全ファイル名（`dic09_Test.txt` を含む）に対する `git check-ignore` が 0 件であることで確かめる（`dic09_Test.txt` が「無視されない」と判定されることを較正の 1 件とする）。
7. The 検体の `.nar` shall 中身のファイルの内容を 1 バイトも変えない（emo2 の `install.txt` の `Charset` の綴りもそのまま）。
8. When 段 ③ が完了した, the `crates/areka/src/emo2_boot/spine.rs` shall 起動記録を消す初期化を持たない。
9. When 段 ③ が完了した, the `crates/pilot/examples/shiori-host-32/.gitignore` shall 削除された展開ツリーを指す無視規則を持たない。

### Requirement 9: 失敗の可視性と決定論テスト

**Objective:** 開発者として、拒否や失敗は必ず理由が読め、全ての分岐が実機なしで再現できてほしい。そうすれば、製品側のインストーラの検証も同じ固定入力で行える。

#### Acceptance Criteria

1. The NAR エンジン shall 全ての拒否・失敗を、対象のアーカイブのパスと理由を含む記録（error 相当）と失敗の戻り値の両方で伝える（ログ無しの失敗経路を持たない）。
2. The NAR エンジン shall 拒否の理由を閉じた語彙で返し、製品側がそのまま `OnInstallFailure` の理由に写せる。語彙の全数は設計で確定し、決定論テストが「語彙の各項目に少なくとも 1 つの固定入力が対応する」ことを判定する（少なくとも「install.txt が無い」「対応外の種別」「必須キーが無い」「名前が復号できない」「安全でないパス」「対応外の圧縮方式または暗号化」「壊れたアーカイブ」「宛先が無い」「取り出し元フォルダが無い」を含む）。
3. The 本仕様 shall Requirement 2〜6 の各受理・拒否分岐を、固定の小さな `.nar`（受理 5 種以上: ghost 単体・ghost with balloon・balloon・shell・supplement／拒否: 包みフォルダ 1 段・`type` 無し・対応外 `type`・`directory` 無し・`name` 無し・`..`・絶対パス・`\` 区切り・NUL・予約名・大小衝突・復号不能な名前・シンボリックリンク・取り出し元フォルダ無し・宛先無し／再インストール: `refresh` 無効の上書き・`refresh,1`＋mask）で決定論的に検証する。
4. The 決定論テスト shall UTF-8 の印が立った名前・印の無い Shift_JIS の名前（日本語のフォルダ名とファイル名）の両方を含む固定 `.nar` で、名前の復号を検証する。
5. The 決定論テスト shall Requirement 7.4（取得のたびに新品）を、実機を使わずに「窓口が返した根に起動記録を書く → 再度窓口を呼ぶ → 起動記録が無い」の形で検証し、Requirement 7.5・7.6 を並走・多重プロセスの形で検証する。
6. The 決定論テスト shall 実機・GPU・32bit helper を要さず、純 x64 で `cargo test` の一部として走る。
7. The 本仕様 shall 完了前に emo2 で実機一周（Requirement 1.9 のコマンドで絶対パスを取得 → 起動・有界の自動終了 → ログで `OnFirstBoot` を確認 → 再度取得してもう一周 → 再び `OnFirstBoot`）を行い、結果を検証報告に残す。

### Requirement 10: 依存の登記・文書・台帳の追随

**Objective:** 開発者・後続の実装者として、外部依存の追加とクレートの新設が所定の場所に登記され、検体パスを綴っていた文書やスクリプトが壊れていないでほしい。

#### Acceptance Criteria

1. When アーカイブ読取のための外部依存を 1 本追加する, the 本仕様 shall `.kiro/steering/tech.md` に「意図的依存追加＝日付・承認済」の形で登記し（`encoding_rs` の前例と同じ書式）、ライセンス検査（`cargo deny check`）が緑で、`THIRD-PARTY-NOTICES.md` を再生成する。
2. The 本仕様 shall 追加する依存の既定機能を切り、書き込み側の圧縮機能を有効にしない（読取と展開に必要な機能だけ）。
3. The 新設クレート shall 本番の依存（`[dependencies]`）としてワークスペースに加わり、`log-capture-kit` を `[dependencies]` に置かず、テスト用の部品は `[dev-dependencies]` に限る。
4. The 本仕様 shall `.kiro/steering/structure.md` のクレート一覧に新設クレートと検体の窓口の置き場を登記する。
5. The 本仕様 shall 検体パスを綴る `tools/perf/` のスクリプト 6 本・`doc/COMPAT_ARCHITECTURE.md`・`doc/emo2-conformance-scope.md`・`doc/ukadoc-coverage/briefing-assets.md`・`crates/pilot/examples/shiori-host-32/README.md` を、窓口経由の得方（Requirement 1.9）へ書き換える。
6. The 本仕様 shall 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `descript_install` ページのうち `install.txt` のキー 11 項目（`type`・`name`・`directory`・`charset`・`accept`・`refresh`・`refreshundeletemask`・`*.directory`・`*.source.directory`・`*.refresh`・`*.refreshundeletemask`）の `owner` を本仕様に登記し、着地後に状態を実測へ更新する。`bootghost`（`type,package` 専用・`package` は拒否対象）と、同ページの `install.txt` 以外の 3 見出し（「相対パス」＝`delete.txt`・「相対パス,オプション1,…」と「相対パス,ignore」＝`developer_options.txt`。前者は `network-update`、後者は作る側の範囲）は `owner` を空のまま残し、備考にその理由を書く。
7. When 台帳の `owner` を登記した, the 本仕様 shall `doc/ukadoc-coverage/roadmap-draft.md` の表に本仕様の行（`owner_count`）を足して追随させ、`cargo test -p ukadoc-survey` の整合検査が緑であることと、ドメイン別報告・全体報告を作り直すことを完了条件に含める。
8. The 本仕様 shall 1 ファイル 1,000 行の番人の例外表に触れず、新設ファイルは 1,000 行未満に収める。
9. The 本仕様 shall 実機サインオフの手順に現れる「検体の絶対パス」の得方を、`.kiro/steering/roadmap.md` の制約の節（実機運転の定石）に 1 行で追記する。
10. The 本仕様 shall 検体を `.nar` に畳む手順（追跡ファイルだけを含める・`profile/` を含めない・改行や文字コードを変換しない・配布形の構造・畳んだ後に Requirement 8.4 の一致を確かめる）を、次の検体を足す人が見つける場所（`vendors/sample_ghost/` の README）に書く。

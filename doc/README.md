# doc の案内

`doc/` に置いたファイルとフォルダの役目と、それが今も効いているのか歴史として置いているのかを、この 1 枚で示す。

## 決まり

- 文書は `doc/` の下に置く。根に `docs` のようなほかの置き場を作らない。
- 歴史として置くものは `doc/archive/` の下だけに置く。`doc/archive/` の外に在るものは、どれも今も効いている。
- 例外は日付つきの俯瞰のスナップショット `doc/roadmap-overview-{YYYY-MM-DD}.html` で、`doc/` の直下に置く。ファイル名の日付（roadmap.md の `updated_at`）で「その日の姿」と分かるので、今の roadmap.md と食い違っていても直さない。
- 名前の決まり: 名前は小文字の英字・数字・ハイフン・点だけで書き、語はハイフンでつなぐ。拡張子・版の番号・日付・並びの番号の点とハイフンは含めてよい。各フォルダの案内の `README.md` だけは決まりの外。`doc/archive/` の下は歴史として名前も残し、決まりの外。
- 決まりに合わない名前を残すときは、下の「名前の決まりの例外」の表に理由と行方（後で改名する／改名しない）つきで 1 行足す。フォルダ単位の行（末尾 `/`・下の全部を覆う）は、中の名前を読む側のデータや道具が決めるフォルダにだけ使い、ほかはファイル 1 つずつの行で載せる。例外は今そこにある事情だけを表し、事情が無くなったら（そのものが無くなったか、名前が決まりに合ったら）行を消す。
- `doc/` の直下にファイルやフォルダを足す・動かす・消すときは、同じ変更で下の「直下のファイルとフォルダ」の表の行を足す・直す・消す。動かしたら「動かした名前の対応」の表にも 1 行足す。
- 確かめ: 照合テスト `crates/log-capture-kit/tests/doc_tree_guard_test.rs` が、直下の表と `doc/` の実物（ディスクで数える）の食い違い・名前の決まりに合わない名前・要らなくなった例外の行・根に `docs` のフォルダが無いことを確かめ、1 件でも食い違えば赤になる。いつもの全体テスト（`tools/test-all.ps1`）で回り、単独では次で回す（どのフォルダから打っても動く）。

  ```powershell
  cargo test --manifest-path C:\home\maz\git\areka\Cargo.toml -p log-capture-kit --test doc_tree_guard_test
  ```

  ほかの写し（ワークツリーなど）で回すときは、`C:\home\maz\git\areka` をその写しの場所に替える。
- 表の書き方（照合テストが読むので崩さない）: 照合テストは見出しの文字で表を見分ける。見出し「直下のファイルとフォルダ」「名前の決まりの例外」の文字を変えない。表の 1 列目はバッククォートで囲んだ名前 1 つだけにする。フォルダは名前の末尾に `/` を付ける。置き字は `{YYYY-MM-DD}`（4 桁・2 桁・2 桁の数字をハイフンでつないだ日付）だけを使う。直下の表の 1 列目は `doc/` を付けない名前、例外の表と対応の表はリポジトリの根からのパスで書く。

## 直下のファイルとフォルダ

| 名前 | 役目 | 今か歴史か |
|---|---|---|
| `README.md` | この案内 | 今も効く |
| `CONSTITUTION.md` | areka のプロジェクト憲法 | 今も効く |
| `COMPAT_ARCHITECTURE.md` | 互換アーキテクチャの設計判断の台帳（設計判断の正本） | 今も効く |
| `PASTA_PROFILE.md` | キャラクター「ぱすたさん」の基本プロファイル | 今も効く |
| `emo2-conformance-scope.md` | emo2 がそのまま動くために要る機能の輪郭（M1 の範囲の正本・網羅台帳が今も引く） | 今も効く |
| `anchor-compat.md` | アンカーの互換対応表（Reference の割付・送出の順・正典が黙る所の裁定） | 今も効く |
| `choice-cascade-compat.md` | 選択関連イベントの互換対応表（カスケード則・Reference の割付・タイムアウト） | 今も効く |
| `crates-io-publish.md` | `wintf`・`dola` を crates.io へ出す手順書 | 今も効く |
| `impl-watch.md` | 机の貸し出し `areka-impl-watch` の手順書（道具のエラー文がこの場所を名指す） | 今も効く |
| `pasta-real-run.md` | ゴースト「窓際のぱすたさん」を areka で 7 段回して SSP と比べる、何度でも回せる実機の手順（v0.1.0 の版の門の最後の確かめで回す） | 今も効く |
| `v0.1.0-gate-real-run.md` | v0.1.0 の版の門の最後の確かめ（2026-11-01 にぱすたの 7 段とどっとさくらを実機で回した結果と、残った崩れ） | 今も効く |
| `testing.md` | 試験の手引き（実機の試験の窓口 `real-run-kit` の立て方・呼び方・閉じ方・残した証拠の読み方） | 今も効く |
| `roadmap-overview-{YYYY-MM-DD}.html` | roadmap.md の俯瞰図（版の段ごとの帯と spec のカード）。下の「俯瞰の HTML」の道具で作る | その日の姿のスナップショット（上の例外） |
| `ukadoc-coverage/` | ukadoc の全項目の網羅台帳（`ukadoc-survey` が読む。`report/` は作り直す生成物） | 今も効く |
| `ssp-mcp/` | SSP の内蔵 MCP サーバの実測の控えと areka との差の一覧（`tools-list-ssp-2.9.05.json`・`log-convention.md` はテストが読む） | 今も効く |
| `release-notes/` | 版ごとのリリースの見どころと、その書き方（リリースの処理が読む） | 今も効く |
| `shiori/` | SHIORI の紙の契約のカタログ（`fragments/` が正本。`shiori4-api` が正本と呼ぶ） | 今も効く |
| `images/` | 根の `README.md` が載せる絵 | 今も効く |
| `ideas/` | まだ spec でない案の原文と決めたこと（`usage-stats.md`＝利用統計、`novel-platform.md`＝ノベルゲーム基盤。roadmap の受け口の節と brief が引く） | 今も効く |
| `archive/` | 歴史の置き場（旧 ARCHITECTURE・旧 UI の設計・M1 期の議論の写しなど） | 歴史 |

## 名前の決まりの例外

| 名前 | 理由 | 行方 |
|---|---|---|
| `doc/COMPAT_ARCHITECTURE.md` | 同じウェーブ（C17）で並んで走る見直しとの約束で、`doc-tree-tidy` では名前を変えない | 後で改名する（後続の spec は `doc-tree-tidy` の完了のときに起票し、ここにその名前を書く） |
| `doc/shiori/fragments/` | 名前と結合の順を目録 `_manifest.toml` が決める、道具が読むデータの目録であるため | 改名しない |

## 動かした名前の対応

歴史の記録に残る古いパスから今のファイルを辿るための表（照合テストは読まない）。

| 古いパス | 新しいパス |
|---|---|
| `docs/click_through.md` | `doc/click-through.md` |
| `doc/CONSTITUTION.md` | `doc/constitution.md` |
| `doc/PASTA_PROFILE.md` | `doc/pasta-profile.md` |
| `doc/shiori/equivalence_evidence.toml` | `doc/shiori/equivalence-evidence.toml` |

## 俯瞰の HTML

### 作り方と開き方

どのフォルダから打っても動く。

```powershell
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\skills\kiro-next\overview\roadmap-overview.ps1 -Open
```

- `.kiro/steering/roadmap.md` を読み、`doc/roadmap-overview-{updated_at}.html` を書く（同じ日付のファイルは書き替え、ほかの日付のファイルには触らない）。`-Open` を付けると、書けたときだけ既定のブラウザで開く。`-OutFile <パス>` を付けると、`doc/` でなくそのファイルへ書く（`/kiro-next` は棚卸の最後に、作業用のフォルダの `roadmap-overview-latest.html` へ書いてセッションで公開する。コミットはしない）。roadmap.md の「ウェーブ編成」の節に ``` で囲んだ塊（次のウェーブの開始コマンド）があれば、頁の上にコピーのボタンつきで出す。開くだけなら作ったファイルをブラウザで開けばよい（ネットは使わない）。
- ほかの写し（ワークツリーなど）で作るときは、`C:\home\maz\git\areka` をその写しの場所に替える。道具は自分の在るリポジトリの roadmap.md を読む。
- 端末に `OK wrote ...` が出れば書けた。`WARN` の行は roadmap.md の食い違いの知らせで、HTML は書かれる。`FAIL` の行が出たら何も書かれない。
- 作ったファイルは手で直さない。roadmap.md が動いても、作ったファイルを継続して直すことはしない（見たいときにまた作る）。コミットするかは任意。

### 頭の中身の印の使い方

HTML の頭には、元にした roadmap.md の更新日と中身の印（roadmap.md の `git hash-object` の先頭 12 字）が出る。印は中身だけで決まり、どの枝・どのコミットで作ったかによらない。

- その中身の roadmap.md が入ったコミットを辿る:

  ```powershell
  git -C C:\home\maz\git\areka log --oneline --find-object={印}
  ```

  探すのは今の枝の歴史だけなので、まだマージしていない枝で作った印は main からは見つからない。何も出なければ `--all` を足して打ち直す。

- 手元の roadmap.md と同じ中身かを確かめる（出た値の先頭 12 字が印と同じなら同じ中身）:

  ```powershell
  git -C C:\home\maz\git\areka hash-object .kiro/steering/roadmap.md
  ```

印が「不明」と出ているときは、作ったときに git が使えなかった。

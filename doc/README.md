# doc の案内

`doc/` に置いたファイルとフォルダの役目と、それが今も効いているのか歴史として置いているのかを、この 1 枚で示す。

## 決まり

- 歴史として置くものは `doc/archive/` の下だけに置く。`doc/archive/` の外に在るものは、どれも今も効いている。
- 例外は日付つきの俯瞰のスナップショット `doc/roadmap-overview-{YYYY-MM-DD}.html` で、`doc/` の直下に置く。ファイル名の日付（roadmap.md の `updated_at`）で「その日の姿」と分かるので、今の roadmap.md と食い違っていても直さない。
- `doc/` にファイルやフォルダを足す・動かすときは、この案内の表に 1 行足す（直す）。決まりは書くだけで、機械の検査は無い。

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
| `testing.md` | 試験の手引き（実機の試験の窓口 `real-run-kit` の立て方・呼び方・閉じ方・残した証拠の読み方） | 今も効く |
| `roadmap-overview-{YYYY-MM-DD}.html` | roadmap.md の俯瞰図（版の段ごとの帯と spec のカード）。下の「俯瞰の HTML」の道具で作る | その日の姿のスナップショット（上の例外） |
| `ukadoc-coverage/` | ukadoc の全項目の網羅台帳（`ukadoc-survey` が読む。`report/` は作り直す生成物） | 今も効く |
| `ssp-mcp/` | SSP の内蔵 MCP サーバの実測の控えと areka との差の一覧（`tools-list-ssp-2.9.05.json`・`log-convention.md` はテストが読む） | 今も効く |
| `release-notes/` | 版ごとのリリースの見どころと、その書き方（リリースの処理が読む） | 今も効く |
| `shiori/` | SHIORI の紙の契約のカタログ（`fragments/` が正本。`shiori4-api` が正本と呼ぶ） | 今も効く |
| `images/` | 根の `README.md` が載せる絵 | 今も効く |
| `archive/` | 歴史の置き場（旧 ARCHITECTURE・旧 UI の設計・M1 期の議論の写しなど） | 歴史 |

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

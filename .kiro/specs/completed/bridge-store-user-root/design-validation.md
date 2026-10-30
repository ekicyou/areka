# 設計の検証: bridge-store-user-root

> 2026-10-29・`/kiro-validate-design`（対話なしで実行）。対象は `design.md`（ワークツリーの先頭 `8bd3db76`）。要件（`requirements.md`・討議の 7 点を含む）・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）と、設計が名指すコードを読んで突き合わせた。

## 1. まとめ

設計は、中継の「置き場を作る・写す」働きを `areka-user-data` の新しい口 `locate_app_profile` に置き換え、答えの型 `AppProfile` で「隣に areka が居るが控えを持たない」場面を `LOCALAPPDATA` の段へ落ちない形で分けている。要件の受け入れ基準 47 項目はすべて設計の部品か手順に対応しており、名指したコードの事実（関数・行番号・公開の範囲・`decide` の理由の決まり・`cache.rs` の `create_dir_all` の穴）も今のコードと合っている。直すべき大きな点は 1 つで、直した後の実機の測り（M-2）の手順が、設計でいちばん危うい前提（箱の見立て D-6）を 1 回の起動では確かめられない形になっていること。タスク生成で手順に足せば済むので、判定は **GO**。

## 2. コードとの突き合わせ（確かめたこと）

| 設計の記述 | コードの事実 | 結果 |
|---|---|---|
| 中継は `Config::from_env` で `establish` と `app_profile_dir` を呼ぶ | `crates/areka-mcp-bridge/src/config.rs` 63〜69 行 | 合う |
| `from_values` の段は `profile_dir.or(beside_profile).or(local)`・呼び手は `from_env` とテストだけ | `config.rs` 103〜106 行・`config_tests.rs` だけが呼ぶ（grep） | 合う |
| 置き場が無いときの `warn!` 1 件は `from_env` が出す | `config.rs` 80〜84 行（W3 の文と同じ） | 合う |
| `cache.rs` の `replace` は書く前に `create_dir_all(dir)` で親ごと作る | `cache.rs` 89 行 | 合う（`beside` を `None` にすると `%LOCALAPPDATA%\areka` が作られる、という穴の指摘は正しい） |
| `main.rs` 70 行の注記 | 「置き場が無いときの 1 件は `Config::from_env` の `warn!` が出している」 | 合う |
| `decide_logged`・`log_layout` は非公開、`migrate_source` は `pub(crate)` | `establish.rs` 163・256・250 行 | 合う |
| `decide_logged` は `Beside` と `Fallback` のどちらでも `(None, reason)` を返す | `establish.rs` 172〜186 行 | 合う |
| `decide` は `Beside` の理由をいつも `Neither` にし、`Fallback` の理由は `Neither` にしない | `lib.rs` 124〜171 行（`Fallback` の理由は `EnvVar`・`WingetInstall`・`Unwritable` だけ） | 合う |
| `areka-user-data` の開発用の依存に `log-capture-kit` が既に在る・中継には無い | `areka-user-data/Cargo.toml` 20 行・中継の `[dev-dependencies]` は `temp-path-kit` と `tokio` だけ | 合う |
| areka 本体の呼び手は `boot_config.rs`（`app_profile_dir`／`readonly_app_profile_dir`） | `boot_config.rs` 546〜552 行 | 合う |
| 説明書の行番号 289・297・308 | `dist/README.txt` のその行がそれぞれ zip の形の「中継の控え」・利用者ごとの置き場の形の「中継の控え」・中継の控えの段落 | 合う |
| `exe_test.rs` 52〜53 行の注記 | `AREKA_USER_ROOT` を外す理由の 2 行 | 合う |

要件との対応: 要件 1.1〜1.7・2.1〜2.12・3.1〜3.9・4.1〜4.6・5.1〜5.6・6.1〜6.7 のすべてが「Requirements Traceability」の表に部品・口・手順として載っている。抜けは無い。

## 3. 大きな問題（1 件）

### 問題 1: 直した後の実機の測り（M-2）では、箱の行 B1 を 1 回の起動で見られない

- **何が問題か**: 箱の見立て（`redirect::observe`）は `from_env` の最後、つまり控えを最初に書くより前に走る。S2（本当の `%LOCALAPPDATA%\areka\profile\areka\` が在る形）で直した中継が初めて起こされたとき、控えのフォルダ（`…\profile\areka\mcp-bridge`）はまだどこにも無いので、見立ては `Unknown("not_there_yet")`（行 B2）になる。箱の中に控えのフォルダができるのは、その起動で areka につながって控えを書いた後で、B1（箱のフォルダの場所つき）が出るのは**次の起動から**。ところが手順の 5 は Claude Desktop の起動し直しを 1 回だけ頼み、記録の欄は「S2 で控えが箱の中の `areka\profile\areka\mcp-bridge\`・B1 に箱のフォルダ」を M-2 で示すことにしている。また、控えが書かれるのは中継が areka につながったときだけなので、測りの間に areka が動いていなければ 2 回起こしても B1 は出ない。
- **なぜ大事か**: 「中継の exe で `canonicalize` が箱の場所を返すか」は、`research.md` 10.5 と設計の Revalidation Triggers が「M-2 で確かめ、出なければ D-6 を見直す（設計に戻る）」としている、いちばん危うい前提。今の手順のままだと、M-2 で B2 しか見られず、「D-6 が崩れた」と取り違えるか、確かめないまま終わる。
- **直し方**: M-2 の S2 の手順を次の形にする。⑴ 測りの間、開発者の机で areka を動かしておく（中継がつないで控えを書けるように）。⑵ 1 回目の起動で B2（`why = not_there_yet`）と、箱の中に `areka\profile\areka\mcp-bridge\` ができたことを記録する。⑶ 中継をもう 1 回起こしてもらい（Claude Desktop の起動し直しをもう 1 回頼む。頼む文は「急がなくてよい・切れ目で」が先頭）、B1 と `box_dir` の欄を記録する。記録の表の「期待」の列も、1 回目と 2 回目に分ける。
- **対応する要件**: 1.3・3.5・3.6（と 4.1 の調べの結論への追記）
- **設計の場所**: 「実機の測り（M-1 直す前・M-2 直した後）」の手順 5・6 と「記録」、Revalidation Triggers の 5 つ目、`research.md` 10.5 の 3 つ目

## 4. 設計のよいところ

1. **落ちる道を型で消している**: `from_values` の 3 つ目の引数を `Option<AppProfile>` にし、`None`（隣に areka が居ない）と `Some(Ready 以外)`（居るが控えを持たない）を分けた。今の `cache.rs` の `create_dir_all` が `%LOCALAPPDATA%\areka` を作る穴を、`bool` を足すのでなく型の形で書けなくしている。写しの条件（`migrate_source`）と判断の記録（`decide_logged`・`log_layout`）の持ち主も `areka-user-data` の 1 か所のままで、areka 本体の口は 1 行も変えない。
2. **箱の見立てが文書の約束の外に頼りすぎていない**: パッケージの名乗り（名乗りの無い子では見落とすと実測で分かっている）や `Packages\<名前>\LocalCache\Local` の組み立てを捨て、標準ライブラリの `canonicalize` 2 回だけで見立て、分からないときは「分からない」の行を残す。判断は値だけの `classify` に分けてあり、本物の箱が作れないテストでも全部の腕を固められる。

## 5. 小さな直し（タスク生成か実装で拾えばよいもの）

1. **要件 3.8 のテストの範囲**: 設計は「W1・W2・B1・B2 の文と欄の値が ASCII」を確かめるとしているが、`user_root`・`asked`・`resolved`・`box_dir` の欄は場所の値で、利用者の名前が日本語なら本番では ASCII にならない（要件 3.8 が言うのは「記録の文」）。テストで ASCII を確かめるのは、文と、決まった語の欄（`reason`・`why`）だけにする。今のテストのままだと、ワークツリーを日本語のパスに置いた机で赤になる。
2. **「箱の外の取り違え」の注記の言い方**: `Outside` の腕の `shortcut:` の注記は「初めて見た 1 回」としているが、控えが書かれるのは areka につながったときだけなので、本当は「控えを最初に書けた起動まで」続く（areka が居ない起動が続けば、その間ずっと箱の行が 0 件）。注記の文をそう直す。
3. **`Unlocatable` の見分けを理由から逆に読まない形も選べる**: 設計は「`Beside` の理由はいつも `Neither`」という決まりに頼り、テストと Revalidation Triggers で守る。`decide` は公開の純粋な関数なので、`locate_app_profile` の中で `matches!(decide(signals), Decision::Fallback { .. })` を見れば、この決まりに頼らずに済む（`decide_logged` は変えずに記録に使う。二度呼んでも値だけの計算）。どちらでもよいが、こちらなら Revalidation Triggers の 2 つ目が要らなくなる。
4. **`Unlocatable` のときの記録の並び**: `areka-user-data` の error（"staying beside the exe"）の後に中継の W2（warn・控え無し）が出る。設計は承知のうえで文を共有にしているので直さなくてよいが、`config_prepare_tests.rs` の 2.7 のテストで「error 1 件の後に W2 が 1 件」の並びも固めておくと、後で読む人が取り違えない。
5. **2.6 の「根の `profile\areka` が無い」の場面**: 根は在るが `profile\areka` も exe の隣の記憶も無いときは `Ready` になり、最初の控えの書きで `R\profile\areka` が作られる（箱の外の中継なら本当の置き場に）。要件 2.6 の範囲で正しいが、`config_prepare_tests.rs` のこの場面で、保存を呼んだ後に `R\profile\areka\mcp-bridge` だけができることを木で確かめておくと、「作ってよいのはここだけ」がテストに残る。

## 6. 判定

- **判定: GO**
- **理由**: 名指したコードの事実はすべて合い、受け入れ基準はすべて設計に対応し、層の向き（`areka-user-data` は中継を知らない）・新しい外部依存 0・ASCII の記録・テストの置き方は steering に沿っている。大きな問題は M-2 の測りの手順の 1 件だけで、設計の形を変えずに手順に 1 段足せば直る。
- **次の段**: 設計の討議（`/kiro-design-discussion bridge-store-user-root`）で問題 1 の直し方を決め、設計の「実機の測り」の手順と記録の欄に反映する。小さな直しの 1〜5 は討議でまとめて扱うか、タスク生成（`/kiro-spec-tasks bridge-store-user-root`）に申し送る。

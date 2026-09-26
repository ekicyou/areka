# 設計レビュー: areka-P0-alpha-package

> 2026-09-26・worktree の HEAD `16d39f86`（main `2f5bd24a` の上に spec の文書だけ）で `design.md` を読み、引用されたコードの実在を今の木に対して `grep` で 1 つずつ確かめた。非対話で書いた（開発者への質問は無し）。

## 要約

設計は「外部の道具を順に呼ぶ手順」と「文書」に徹しており、`crates/` に触らず要件 1〜7 を満たす道筋が具体的である。開発機でだけ通る 4 つの経路（VC++ ランタイム・長いパス・自動終了の早すぎ・謝辞の範囲）を名指しして塞ぎ、判定は「集めてから 1 回主張する」形で揃っている。引用されたコードの名前・記録の文言は**すべて今の木に実在**し（下の表）、道具の旗（`cargo about generate --locked`／`--target`／`--workspace`・`cargo deny --locked`）も実物の `--help` で確認できた。残る指摘は手順の穴 1 つと、環境依存が紛れ込む書き方 2 つで、どれも設計の骨格を変えずに直せる。

## 引用の実在の確認（設計が名前で指したもの）

| 設計の引用 | 実在 | 備考 |
|---|---|---|
| `main.rs` の `SMOKE_EXIT_ENV`＝`AREKA_APP_SMOKE_EXIT_MS`・「smoke 自動 close ゲート有効」・`finish_after_run`・`windows_subsystem`・`EnvFilter::new("info")` | ○ | 自動終了は `quit_app(ExitOrigin::Smoke)` を通る。記録は標準出力（`tracing_subscriber::fmt()` の既定）＝設計が 2 本を連結して見るので問題なし |
| `alert.rs` の `NO_ALERT_ENV`・`SHIORI_FAULT_TITLE`「SHIORI が動かなくなりました」・`suppressed_from` | ○ | |
| `boot_config.rs` の `resolve_root_from`・`default_helper_exe_path`・`default_app_profile_dir`・`event = "balloon_resolved"`「バルーンを決めました」（`route = ?`・`dir = %`） | ○ | `route=Companion` の字面は Debug 表示どおり |
| `boot_resolve.rs` の `DEFAULT_GHOST_FOLDER`＝`emo2`・`DEFAULT_BALLOON_FOLDER`＝`StayseeBalloon`・`resolve_ghost`／`resolve_balloon`・`BalloonRoute::Companion`・`companion_balloon_not_found` | ○ | |
| `ghost_session.rs`「本物のゴースト窓を開きました」 | ○ | |
| `areka-kanade/src/schedule/boot.rs` の `to_baseware_version`・`event = "boot_talk"` の 2 文言（「起動グリーティングを再生起動」／「epilogue-only 起動記録トークを再生起動（挨拶トーク皆無）」）・`boot_complete` | ○ | 設計どおり `event` 名でなく文言で見なければ 204 の失敗を通す。判定の文言の選び方は正しい |
| `areka-kanade/src/shiori/real.rs` の `connect_failed`／`helper_exited` | ○ | |
| `smoke_boot_loop_exit.rs` の `is_i686_pe`（`0x3c`→PE ヘッダ・機種 `0x014c`）・`run_smoke` | ○ | |
| `shiori-host32-host/src/job.rs` の `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` | ○ | helper を名前で探して止めない設計は成り立つ |
| `menu/captions.rs` の `FRAME_CAPTIONS`（7 行）・`app_exit.rs` の `quit_app` | ○ | |
| `nar-sample-path.rs` の `root=`／`folder=`／`balloon.<名>=`・`SAMPLES` に `emo2`（同梱 `emo2-kakukaku`）と `StayseeBalloon` | ○ | |
| `.claude/skills/kiro-complete/SKILL.md` License Gate (b) の「環境差として戻す」段落 | ○ | |
| 根の `README.md`: バッジのリンク先 `LICENSE`・「57件…約70%」・「MIT OR Apache-2.0」の節 | ○ | 直す 3 か所は特定どおり |
| `.gitignore` 2 行目 `Cargo.lock`・`LICENSE-MIT`・`about.toml`／`about.hbs`／`deny.toml`・`vendors/sample_ghost/{emo2,StayseeBalloon}.nar`・`completed/` 直下のフォルダ 200 | ○ | `.gitattributes` は無い・`core.autocrlf=true` |
| 実行時に読む環境変数の接頭辞 | `AREKA_`・`WINTF_`・`HOST32_` | `HOST32_*` は host が helper へ自分で設定する 3 種と、テスト専用（`#[cfg(test)]` の中）だけ。`AREKA_`／`WINTF_` を全部外す設計で漏れは無い |

## 重要な指摘（3 件）

### 指摘 1: `Cargo.lock` の「出る側」の手順が無い（並走への波及の片側だけ書いている）

- **問題**: 設計の `workflow.md` の節は「並走中の worktree が本仕様の着地を取り込むとき」（入る側）だけを書いている。逆向き——**枝が PR を出す前に main を取り込み、`Cargo.lock` が全部の `Cargo.toml` と一致していることを確かめる**——が無い。`Cargo.lock` は本仕様が新規に足すファイルなので、依存を足した別の枝（`Cargo.toml` だけを触る）と文字上の衝突は起きず、squash マージは通る。その結果 main の `Cargo.lock` が `Cargo.toml` より古くなり、配布スクリプトは綺麗な main でも「x64 本体ビルド」の段で止まる（設計はこれを「止まるのが正しい」と書くが、直すには `Cargo.lock` だけの PR がもう 1 本要る）。
- **影響**: 要件 7.3 の目的「同じコミットから組めば同じ zip」が、着地の順序次第で main 上で成り立たない期間ができる。
- **提案**: `workflow.md` の同じ節に 1 行足す——「PR を出す前に main を取り込み、`cargo metadata --locked` を 1 回通す（数秒・lock が古ければ非 0）」。本仕様自身の着地手順にも同じことを書く（手順 1 の `cargo generate-lockfile` は着手時に 1 回、PR 直前に main を取り込んでもう 1 回、謝辞も作り直す）。`kiro-complete` の SKILL.md（要件 7.4 で触る）にも PR 作成の前の 1 行として写す。`tools/test-all.ps1` は触らない（境界どおり）。
- **要件**: 7.3・7.5（1.6）
- **設計の箇所**: 「Cargo.lock の追跡の始め方」の「開発の手順」

### 指摘 2: `RUSTFLAGS` の扱いが「置き換え」か「継ぎ足し」か決まっていない

- **問題**: 設計は「`RUSTFLAGS=-C target-feature=+crt-static` を自分のプロセスに設定して、終わったら元に戻す」と書くだけである。開発者のシェルに `RUSTFLAGS`（例: `-C target-cpu=native`）が既に在るとき、継ぎ足せば**開発機の CPU でしか動かない exe**ができ、取り込み表の判定（判定 6）はこれを見つけられない。また `CARGO_ENCODED_RUSTFLAGS` が在ると cargo は `RUSTFLAGS` を無視するので `+crt-static` が黙って効かない（こちらは判定 6 が拾う）。
- **影響**: 要件 2.7 と要件 3 の目的「開発機の環境に助けられて動いているだけの zip を配らない」に、判定に映らない抜け道が 1 つ残る。
- **提案**: 較正値の節に明記する——`RUSTFLAGS` は**置き換える**（継ぎ足さない）、`CARGO_ENCODED_RUSTFLAGS`・`CARGO_BUILD_RUSTFLAGS` はビルドの 2 段の間だけ外す、使った `RUSTFLAGS` の値を `BUILD-INFO.txt` に 1 行足す（5 行目）。今の開発機ではどれも未設定（実測）なので、今日の走行には影響しない。
- **要件**: 2.7（3.3 の目的）
- **設計の箇所**: 「VC++ ランタイム」・段の一覧「x64 本体ビルド」「i686 helper ビルド」

### 指摘 3: `dist/README.txt` の「CRLF」はリポジトリが保証しない

- **問題**: 設計は第三者向け README を「UTF-8（BOM 付き）・改行は CRLF」と定め、同時に「`.gitattributes` は作らない」と書く。この開発機は `core.autocrlf=true` なので、コミット時に CRLF→LF に正規化され、取り出したときに CRLF になるのは autocrlf が有効な機械だけである。zip は作業コピーのバイトをそのまま写す（要件 4.6 は満たす）が、「CRLF」という約束は機械の設定に依る。
- **影響**: 小さい。ただ「設計が定めた性質を、リポジトリが固定していない」形なので、別の機械で組むと第三者に届く改行が変わる。
- **提案**: どちらか 1 つ——(a) `.gitattributes` を 1 行だけ作る（`dist/README.txt text eol=crlf`・境界の「変更 0」一覧から `.gitattributes` を外す）、(b) 設計から「CRLF」の約束を消し「改行は取り出した環境に従う（Windows 10 以降のメモ帳は LF も読める）」とする。作業量は (b) が最小。
- **要件**: 4.1・4.6
- **設計の箇所**: 「dist/README.txt」の「置き場と形」・「変更 0 と明記するもの」

## 設計の強み

1. **開発機でしか見えない失敗を、判定に映る形へ変えている**。`+crt-static` は「ビルドの旗を渡した」で終わらせず、zip から読み戻した exe の取り込み表を拒否表と照らす判定 6 で確かめる。「会話が始まった」も `event` 名でなく文言で見ることで、204 だけを返す失敗（長いパス）を通さない。どちらも「印字するだけ」で済ませていない。
2. **要件の対応表に 0 が明示されている**。要件 6.3（削除済み）は「設計に対応物は無い（0）」、`crates/**`・`tools/test-all.ps1`・`vendors/**`・`about.*`・`deny.toml`・`Cargo.toml`・`LICENSE-MIT`・`.gitattributes` は「変更 0」と名指しで書かれ、境界の内外が読み手に伝わる。

## 要件の網羅と境界

- 要件 1.1〜7.5 はすべて対応表に実現する箱と判定が在る（1.1〜1.7・2.1〜2.7・3.1〜3.7・4.1〜4.6・5.1〜5.8・6.1／6.2／6.4〜6.7・7.1〜7.5）。6.3 は要件側で削除済みで設計も 0 と明記。
- 境界: `crates/` の変更 0・`tools/test-all.ps1` 0・`vendors/sample_ghost/` 0・新しい Rust コード 0・新しい依存 0——設計本文と一致し、逸脱は見つからない。指摘 3 の (a) を採るなら `.gitattributes` 1 行が新規になる（採るなら「変更 0」一覧を直す）。
- `Cargo.lock` の改行: `cargo` は lock を行単位（`lines()`＝`\r\n` も区切り）で比べ、`--locked` では意味で比べるので、CRLF で取り出された lock を書き換えない——設計の見立てどおり。`.gitattributes` 無しで始めてよい。
- `--locked` の付け方: `cargo build`／`cargo run`／`cargo about generate`／`cargo deny` の 4 種すべてに在る。`rustup target add` は cargo でないので対象外。

## 判定

**GO**

理由: 引用されたコードと記録の文言はすべて実在し、要件 1.1〜7.5 が具体的な段と判定に落ちていて、境界も守られている。3 つの指摘は文言と手順の追記で済み、設計の構造（スクリプト 1 本・段の直列・最初の赤で止める・集めてから 1 回主張）を変えない。

次の一手: 設計ディスカッションで指摘 1〜3 の扱いを決め（指摘 1 は追記、指摘 2 は追記、指摘 3 は (a)／(b) の選択）、`design.md` に反映してから `/kiro-spec-tasks areka-P0-alpha-package` へ進む。

## 聞かずに直せる細かい点

1. `git rev-parse --short HEAD` は曖昧さがあると 8 桁以上を返す。zip の名前と `BUILD-INFO.txt` を「7 桁」と決めるなら `--short=7` にする。
2. `ZipFile::CreateFromDirectory` は第 3 引数 `includeBaseDirectory` を `$false` にしないと `stage/` が最上位に入る。設計の段の一覧に引数を書き足す。
3. zip を読み戻したときの項目名は `/` 区切り（.NET 8）だが、判定の前に `\`→`/` へ正規化してから `profile/`・`ghost/` の直下を見ると、区切りの違いで空振りしない。
4. 設計の冒頭の HEAD（`a8fde786`）は今は `16d39f86`。研究の「実行時に読む `AREKA_*` は 14 種／15 種」の数も 2 か所で違う（判定には効かない）。
5. 「否の方向」の試し（`-SmokeExitMs 300`）は「窓が立った」も否になりうるので、終了コード 2 と「会話が始まった＝否」が一覧に出ることの両方を見る、と書いておくと判定の読み違いが減る。

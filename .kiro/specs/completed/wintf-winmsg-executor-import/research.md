# ギャップ分析: wintf-winmsg-executor-import

- 調べた日: 2026-11-14（`/kiro-validate-gap`・要件の段の後）
- 基準: areka の作業木の先頭 `7a0a61e4c`（`main` は `30a343eb`）・元のリポジトリの手元の写し `C:\home\maz\git\wintf-winmsg-executor` の `main`＝`4f20275`
- 調べ方: Grep／Read で areka の木を読み、元のリポジトリは読むだけ（書き込みなし）。手元の短い試し（どれも 2 分以内・追跡されたファイルは書き換えない）を 5 つ回した。置き場は作業の一時フォルダで、何も crates.io へ上げていない（`cargo publish` は `--dry-run` だけ）

## 1. 要約

- **やることの大半は、既に在る `dola` の型をなぞるだけで済む**。ワークスペースは `members = ["crates/*"]` なので、`crates/wintf-winmsg-executor/` に置けば一員になる。根の `Cargo.toml` の依存の行は `dola` と同じ「版＋path」の形、`crates/pilot/Cargo.toml` は `workspace = true` にする。どちらも 1 行ずつで済む。
- **要件の作者が確かめていなかった点の答え＝出せる**。crates.io にまだ無い版の `wintf-winmsg-executor` でも、公開する一覧に入っていれば、ネットを使わずに包む形（全体テストの段）でも、組み立てまでの形（`-Verify`）でも `wintf` を包める。cargo 1.100 は、一緒に包むワークスペースのクレートを一時の置き場（`target\…\package\tmp-registry`）から引くからである。一覧から外すと、ネットを使わずに包む形は `no matching package named ...` で落ちる（3 節）。
- **要件に書かれていない手直しが 3 つ要る**。⑴ 写した `src` は今の rustfmt 1.11 で整形済みでない（`lib.rs` 7 か所・`window.rs` 1 か所）。`-Format` なしの全体テストの段 `fmt --check` が赤になるので、整形を 1 回入れる必要がある（要件 1.3 が許す範囲）。⑵ `tools/crates-io.ps1` の較正の見本は `dola`・`wintf` の 2 つしか持たない。一覧を 3 つにすると、正しい見本が「一覧」の判定で落ちて、道具そのものが赤になる。⑶ `/update` のスキルと `structure.md` の公開の段の説明に、古い形の記述が残る（4 節）。
- **新しく全体テストに入るテストがある**。写したクレートの単体テスト 15 本と doctest 1 本が、`cargo test --workspace` で初めて回る（元のリポジトリには CI が無い）。単独では 15/15 緑（0.69 秒）。うち 5 本は本物の MessageBox を画面に出し、題名を `FindWindowW` で探して閉じる。3 本は同じ題名 `reenter_filter_closure` を使う。並走するワークツリーの全体テストと、画面の上で窓を取り違える余地がある（Research Needed）。
- **規模は S、危うさは低〜中**。作業は道筋がはっきりした型のなぞりである。危うさは、外の操作（Trusted Publishing・元のリポジトリの archive）と、新しく入る画面のテストの 2 つにある。

## 2. 今の形（資産の地図）

| 物 | 今の形 | この spec で動くもの |
|---|---|---|
| 根の `Cargo.toml` | `[workspace] members = ["crates/*"]`・`[workspace.package] version = "0.1.0"`・`license = "MIT"`・`edition = "2024"`。`[workspace.dependencies]` に `wintf-winmsg-executor = "=0.0.5"`（27 行目）と `dola = { version = "0.1.0", path = "crates/dola" }`（33 行目） | 27 行目を `{ version = "0.1.0", path = "crates/wintf-winmsg-executor" }` に |
| 使う側 6 つ | `wintf`・`areka-actor`・`areka-emo-text`・`shiori-host32-host`・`shiori-host32-helper` は `{ workspace = true }`。`crates/pilot/Cargo.toml` 19 行目だけ `"0.0.5"` の直書き（`^0.0.5`＝0.0.5 しか合わない＝path の 0.1.0 とは合わず、直さないと解決できない） | pilot の 1 行（とその上の注記 2 行） |
| `Cargo.lock` | crates.io の `wintf-winmsg-executor 0.0.5`（2746 行目・checksum つき）。依存の名で引く行が 6 つ。`windows 0.62.2`・`windows-core 0.62.2`・`async-task 4.7.1` はどれも 1 版だけ | 項の出どころが path に替わる（source・checksum が消え、版が 0.1.0）。外部の項は動かない見込み（写しの依存は `async-task 4.7.1`・`windows 0.62` だけで、どちらも今の lock に在る） |
| `dola` の manifest（倣う型） | `workspace = "../.."`・`version.workspace`・`license.workspace`・`repository.workspace`・`publish = true`・`keywords`・`categories` | 写しは `license` を自分で持つ（`MIT OR Apache-2.0`）ところだけ型が違う |
| 元の manifest（`4f20275`） | `version = "0.0.5"`・`edition = "2021"`・`license = "MIT OR Apache-2.0"`・`repository` は元のリポジトリ・`readme`・`keywords`・`categories`・`include`（src・examples・README・LICENSE 2 つ）・`[package.metadata.docs.rs]`。依存は `async-task = { version = "4.7.1", default-features = false }` と `windows = { version = "0.62", features = [4 つ] }`。`authors` は無い | `version`→`version.workspace = true`・`repository`→areka（要件 2.1・2.3）。ほかは保つ（要件 2.4・1.7） |
| `tools/crates-io.ps1` | 較正値 `$PUBLISH = @('dola', 'wintf')`（68 行目）。較正の正しい見本 JSON は `dola`・`wintf`・`areka` の 3 項（189〜192 行目）。大きさの較正は `@{ dola = …; wintf = … }`（211〜213 行目） | 一覧を 3 つに・見本に 3 つ目を足す（5 節の議題 6） |
| `tools/test-all.ps1` | 段 `crates.io pre-publish check (package only)` が引数なしの `crates-io.ps1` を回す。`fmt --check`・`cargo test --workspace`・i686 の `cargo build -p shiori-host32-helper …`・`-License` の `cargo deny check`／`cargo about generate` | 動かさない（写しが自動で対象に入る） |
| `.github/workflows/crates-io.yml` | 1 行目の注記「その版の wintf・dola を」。公開は `cargo publish --locked --no-verify @pArgs`（`-Pending` の残りの名前だけ・順は cargo）。認証は `rust-lang/crates-io-auth-action@v1` だけ | 注記の 1 行（要件 4.6） |
| `doc/crates-io-publish.md` | 1 節「出すのは 2 つだけ・`dola` → `wintf`」・2 節の Trusted Publishing（「初回のリリースのタグ `v0.0.2` より前に」）・4 節「2 クレート」・6 節の `-p dola -p wintf`・7 手順目「2 クレートとも」 | 要件 5.1〜5.4 のとおり |
| `tech.md` | 27 行目 Key Libraries「(=0.0.5)…完全 pin」・133 行目からの「crates.io への公開」（「今は `wintf`・`dola`」・「版を上げるときに動かすのは根の `Cargo.toml` の 2 行」） | 要件 5.5 |
| `structure.md` | `crates/wintf-winmsg-executor/` の節が無い。`Workspace Root` の「その他の最上位」に「`workflows/crates-io.yml`＝`wintf`・`dola` を crates.io へ出す公開の段」 | 節を足す（要件 5.6）・公開の段の説明（要件に名指し無し＝議題 9） |
| スキル `/release` | `procedure.md` 56・58 行目（2 行の版を読む）・81〜84 行目（`cargo set-version`・代わりのやり方で 2 行を Edit）・231 行目（`Cargo.toml` の差分「足した行 2・消した行 2」）・289 行目（タグの前に 2 行）・372 行目（2 行が同じ版）・389 行目（「3 行目が増えたら段 2 と 3.2 を直す」）。`SKILL.md` 20 行目「root `Cargo.toml` (2 lines)」 | 要件 5.7・5.8。`Cargo.lock` の判定は `N` を数えて使う形なので直さなくてよい。`THIRD-PARTY-NOTICES.md` の判定「`- {ワークスペースのクレートの名前} {旧版}`」も写しの行（`- wintf-winmsg-executor 0.1.0`）に当てはまる |
| スキル `/update` | `SKILL.md` に「`=` の行（`rmcp`・`rectangle-pack`・`wintf-winmsg-executor`）は…C の候補」・「ワークスペースの自前のクレート（`dola`・`wintf` など `path` の依存）」・「版の表記（`[workspace.package]` の `version`・`[workspace.dependencies]` の `dola` の `version`）を変えない」 | 要件に名指し無し（議題 9） |
| 謝辞 `THIRD-PARTY-NOTICES.md` | `wintf-winmsg-executor 0.0.5` は独立の「MIT License」の塊で、著作権の 2 行（Timo Kröger と winmsg-executor の寄稿者・ekicyou と wintf-winmsg-executor の寄稿者）を持つ（3362〜3368 行目）。ワークスペースのクレート（`dola` など）は、LICENSE のファイルを持たないので、著作権の行の無い MIT の本文の塊にまとまる | 写しはクレートのフォルダに `LICENSE-MIT` を持つので、独立の塊のまま残る見込み（Research Needed 1） |
| `deny.toml`・`about.toml` | MIT・Apache-2.0 を受理。`allow-wildcard-paths = true`・`ignore-dev-dependencies = true` | 動かさない |

**元のリポジトリの今**（読むだけで確かめた）: 作業木は汚れなし（`git status --porcelain` が 0 行）・`HEAD`＝`main`＝`origin/main`＝`4f20275`（`origin/main` は手元の追跡の枝。取りに行って〔fetch〕確かめてはいない）。追跡されたファイルのうち、写す物は `Cargo.toml`・`LICENSE-APACHE`・`LICENSE-MIT`・`README.md`・`examples/basic.rs`・`examples/threads.rs`・`src/lib.rs`・`src/util/mod.rs`・`src/util/msg_filter_hook.rs`・`src/util/window.rs` の 10 個。`Cargo.lock` は追跡されていない（要件 1.2 で写さない物に挙がっているが、元から木に無い）。タグはフォーク元の `v0.2.0`〜`v0.3.2` が在る。CI（`.github/`）は無い。
**crates.io の 0.0.5 との一致**: 手元の cargo の置き場に在る 0.0.5 の中身（`.cargo_vcs_info.json` は `1a71d65f`）と `4f20275` の `src`・`examples`・`README.md`・`LICENSE-MIT` は、改行の違いを除いて同じだった（要件 1.6 の裏づけ）。
**brief の行数との違い**: brief は `src/lib.rs` を 524 行としているが、`4f20275` の実物は 605 行（`util/mod.rs` 7・`msg_filter_hook.rs` 64・`window.rs` 374・例 36＋37）。中身の一致は上のとおり確かめたので、brief の数え方の違いと読む。要件には行数が書かれていないので、作業は変わらない。

## 3. 確かめていなかった点の答え: 未公開の版でも `wintf` を包めるか

**問い**: 写しの版（ワークスペースの版）が crates.io にまだ無いあいだ、全体テストのネットを使わずに包む段（`cargo package --no-verify --allow-dirty --locked --offline -p …`）と、`-Verify`（`cargo publish --dry-run --allow-dirty --locked -p …`）で `wintf` を包めるか。`wintf` の包んだ manifest は、path を外して `wintf-winmsg-executor = "0.1.0"` を crates.io に求める形になる。

**答え: 公開する一覧（`-p`）に一緒に入っていれば包める。外すと包めない。**

一時フォルダに、互いに依存する 2 つのクレート（どちらも crates.io に無い名前・ワークスペースの依存は「版＋path」の形＝`dola` と同じ）を作って試した（cargo 1.100.0）。

| 試し | コマンド | 結果 |
|---|---|---|
| A | `cargo package --no-verify --allow-dirty --locked --offline -p 上 -p 下` | 緑。下 → 上の順に包む。`.crate` は `target\package\tmp-crate\` に出る（`crates-io.ps1` の「大きさ」の判定が見る置き場と同じ） |
| B | 同じ形で `-p 上` だけ | 赤。`no matching package named '下' found`・`location searched: crates.io index`（ネットを使わない形で、一時の置き場を使わない） |
| C | `cargo publish --dry-run --allow-dirty --locked -p 上 -p 下` | 緑。下を一時の置き場 `target\package\tmp-registry` に入れ、そこから引いて上を組み立て、どちらも `aborting upload due to dry run` で止まる（何も上げない） |
| D | `cargo set-version --bump patch --workspace --offline`（cargo-edit 0.13.14） | 根の `[workspace.package]` の版と、`[workspace.dependencies]` の path 付きの行の `version` の両方が上がる（`Updating workspace's dependency`）。`/release` の段 2.1 の「いつものやり方」が 3 行目も動かすと読める |

areka の今の木でも、引数なしの `crates-io.ps1` を回して緑（約 27 秒・`dola`・`wintf` の 0.1.0 を包む）を確かめた。`-Pending` は `present dola 0.1.0`・`present wintf 0.1.0`＝今は両方とも crates.io に在るので、今の木は「未公開の兄弟を一時の置き場から引く」場面の実例にはならない。ただし、過去の版上げ（0.0.2・0.0.3・0.1.0）では、版上げの PR の全体テストと公開の段の `-Verify` が、`dola` のその版がまだ crates.io に無い状態で緑だった。上の A・C はその場面と同じ形である。

**この spec への含み**:
- 要件 4.1（一覧を 3 つにする）が満たされれば、全体テストの段と `-Verify` は、取り込みの直後から次のリリースまで（写しの 0.1.0 が crates.io に無いあいだ）も緑のまま回る。
- 一覧に写しを入れずに `wintf` だけを包むと、ネットを使わない形でも、組み立てまでの形でも落ちる。要件 4.1 と 3.x は、同じコミットで入れる必要がある（分けると、間のコミットで全体テストが赤）。
- 公開の段の「公開」（`cargo publish --locked --no-verify @pArgs`）は、`-Pending` が返した残りの名前だけを出す。`dola` が先に出ていて、`wintf`・`wintf-winmsg-executor` が残っていれば、cargo が写し → `wintf` の順に出す。`wintf` だけが残る場合（写しは出た後）は、写しが既に crates.io に在るので引ける。
- 次のリリースでは、`-Pending` が写しを `absent wintf-winmsg-executor {版}` と示す。crates.io に 0.0.1〜0.0.5 が在るので、索引の 404（「1 つも版が無い」）の枝には入らない＝予備の手順の「最初の版を出す」には当たらない（要件 4.3 は道具を直さずに満たされる）。

## 4. 要件と資産の対応

印: **OK**＝今の物で足りる・**Missing**＝作る／直す物・**Constraint**＝守る縛り・**Unknown**＝確かめが要る（Research Needed）

| 要件 | 判定 | 根拠と残り |
|---|---|---|
| 1.1・1.2 写す物と写さない物 | Missing（手順は単純） | `git archive 4f20275 src examples README.md LICENSE-MIT LICENSE-APACHE Cargo.toml` で 10 個を取り出せる（試しで確かめた）。`.claude`・`.kiro`・`CLAUDE.md`・`.gitignore` は指名しなければ入らない |
| 1.3 1 字違わず | Constraint・Missing | **今の rustfmt 1.11 で整形済みでない**＝`cargo fmt --check` が `src/lib.rs` の 7 か所（412・419・450・480・502・521・554 行目の近く）と `src/util/window.rs` の 1 か所（352 行目の近く）を示す。中身の差は 56 行（改行の違いを除く）。`examples` は差なし。全体テストの `-Format` なしの段 `fmt --check` が赤になるので、整形を入れる。要件 1.3 の「整形の段が動かす分」に当たる。確かめ方は 5 節の議題 1 |
| 1.4 コミットの本文 | Missing | 本文に `ekicyou/wintf-winmsg-executor`・`4f20275` を書く |
| 1.5 汚れ・先頭・食い違いで止まる | Missing（手順） | 今は 3 つとも通る。`origin/main` を取りに行って（fetch）比べるかは議題 10 |
| 1.6 公開面と動き | OK | 0.0.5 の中身と `4f20275` が同じ（2 節） |
| 1.7 edition・依存 | Constraint | 写しの `windows`・`async-task` は**ワークスペースの依存へ寄せず直書きのまま**にする必要がある。`windows = { workspace = true }` にすると、ワークスペースの 27 の機能が公開する manifest に乗り、要件 1.7 の 4 つの機能と食い違う（議題 3）。ワークスペースの中では機能が合わさる（unify）ので、組み立ては 4 つで足りる。単独で組めることは、写しを一時フォルダで `cargo test` して確かめた（下の 1.x 補足） |
| 2.1・2.3 版・repository | Missing | `version.workspace = true`・`repository.workspace = true`（ワークスペースの `repository` は `https://github.com/ekicyou/areka`）のどちらの書き方でもよい |
| 2.2 license | Missing・Constraint | `license = "MIT OR Apache-2.0"` を自分で持つ（`license.workspace` を使わない）。tech.md の注意（cargo-about 0.9.2 が `license` を受け継ぐクレートで 1 行を差し込む件）は、受け継がないので当たらない |
| 2.4 説明の欄と include | OK | 元の manifest のまま保てる |
| 2.5 道具の判定 | OK（一覧を直せば） | 「欄」は `description`・`license`・`repository` が在る。「大きさ」は小さい（1,100 行ほど）。「理由」は出すクレートなので当たらない |
| 2.6 ライセンスの段 | Unknown（低） | `deny.toml` は MIT・Apache-2.0 を受理。謝辞の作り直しは Research Needed 1 |
| 3.1 一員として組む | OK | `members = ["crates/*"]` で自動 |
| 3.2〜3.4 依存の置き換え | Missing | 根の 1 行・pilot の 1 行。pilot の注記（「0.0.5: クラス登録に CS_DBLCLKS…この版を pin し」）も古くなる |
| 3.5・3.6 `Cargo.lock` | Missing（cargo が書く） | path の項に替わる。外部の項は増えない見込み（写しの依存はどちらも今の lock に 1 版ずつ在る） |
| 3.7 i686 | OK（見込み） | crates.io の 0.0.5 を今も i686 で組んでいる（`build i686 artifacts` の段）。同じ中身 |
| 3.8 全体テスト | Unknown（中） | **新しく `cargo test --workspace` に入るテスト**: 単体テスト 15 本（`src/lib.rs` 13・`src/util/window.rs` 2）と doctest 1 本（`compile_fail`）。写しを一時フォルダで単独に回して 15/15・1/1 緑（冷えた組み立て 75 秒・テスト 0.69 秒）。うち 5 本が本物の MessageBox を出し、題名で `FindWindowW` して `WM_CLOSE` を送る。3 本（`reenter_filter_closure_panic`・`…_quit_when_idle`・`…_quit`）は同じ題名 `reenter_filter_closure` を使う（元のコードの注記は「題名はテストごとに一意でなければならない」と言うが、守られていない）。`FindWindowW` は画面全体から探すので、並走する別のワークツリーの全体テストの窓も拾いうる。例 `basic.rs`・`threads.rs` も `cargo test` が組む（Research Needed 2・議題 8） |
| 3.9 謝辞の著作権の表示 | Unknown（低） | Research Needed 1 |
| 4.1 一覧を 3 つ | Missing | `$PUBLISH` の 1 行 |
| 4.2 版の確かめ | OK（一覧を直せば） | `Test-Version` は一覧の全部を見る |
| 4.3 `-Pending` | OK（一覧を直せば） | 名前は 4 文字以上・crates.io に版が在る＝404 の枝に入らない（3 節） |
| 4.4 較正を緑に・赤にできる力を保つ | **Missing（要件に書かれていない落とし穴）** | 正しい見本 JSON が `dola`・`wintf` だけなので、一覧を 3 つにした瞬間に `calibration 'list': good sample (good sample) was rejected: in the list but not publishable: wintf-winmsg-executor` で道具が止まる。見本に 3 つ目の項を足す必要がある（議題 6） |
| 4.5 順を cargo に | OK | 道具も公開の段も `-p` を並べるだけ（3 節の試し A・C） |
| 4.6 公開の段の注記 | Missing | 1 行目の注記だけ。`Test-Workflow`（公開の段の形の判定）は注記を見ないので、較正に響かない |
| 4.7・4.8 出さない・取り下げない | Constraint | 確かめは `-Verify` まで（`--dry-run`）。`-Verify` は crates.io の索引を読む＝全体テストの段には入れない（tech.md「ネットへ出るテストは常時テストに入れない」） |
| 5.1〜5.4 手順書 | Missing | 2 節の表の箇所。2 節の「初回のリリースのタグ `v0.0.2` より前に」は昔の話なので、写しの設定の段取りに書き直す |
| 5.5 tech.md | Missing | 27 行目・「crates.io への公開」の 2 か所・154 行目の「UI スレッド基盤の外部クレ化」の文（外部のクレートと書いている）も読み直す |
| 5.6 structure.md | Missing | 新しい節。加えて「その他の最上位」の `crates-io.yml` の説明（議題 9） |
| 5.7・5.8 `/release` | Missing | 2 節の表の 7 か所＋`SKILL.md` の 1 か所。3 節の試し D から、段 2.1 の「いつものやり方」は 3 行を動かすので、手順の変更は「読む・判定する」行数の書き直しが主になる |
| 5.9 日本語・project の語 | Constraint | — |
| 6.1〜6.5 元のリポジトリの後始末 | Missing（外の操作・許しが要る） | README の変更は向こうの既定の枝へ（向こうも PR の決まりを持つ `CLAUDE.md` が在る）→ archive は `gh repo archive ekicyou/wintf-winmsg-executor`（持ち主の権限）。順は README が入ってから archive（archive の後は書けない）。タグ（フォーク元の `v0.2.0`〜`v0.3.2`）と履歴は消さない |
| 7.1・7.2 完了の報告 | Missing（報告の文） | Trusted Publishing の値は手順書 2 節の表と同じ |

**1.x 補足（単独で組めるか）**: `4f20275` を一時フォルダへ取り出し、`cargo test --offline` を別の target で回した。4 つの機能だけで組み上がり、テストも緑だった（`-Verify` が包んだ物を単独で組むときと同じ条件）。

## 5. 実装の案

### 案 A: 今の型をなぞる（`dola` の形に合わせ、決め打ちの数を 2→3 に直す）

- 写し: `git archive` で 10 個を取り出す → manifest の 2 欄（版・repository）だけを直す → `cargo fmt -p wintf-winmsg-executor` の結果を取り込む。
- 依存: 根の 1 行・pilot の 1 行（と注記）・`cargo update -w` か組み立てで `Cargo.lock`。
- 道具: `$PUBLISH` に 1 語、較正の見本 JSON に 1 項（大きさの較正は今のままでも通るが、3 つにそろえてもよい）。
- 文書: 手順書・`tech.md`・`structure.md`・`/release`（2→3 行）・公開の段の注記。
- ✅ 差分が最小で、どれも既に在る型の繰り返し。レビューで追いやすい
- ✅ 道具の決まり（較正値は冒頭の 1 か所）を崩さない
- ❌ 次に 4 つ目の公開するクレートを足すとき、`/release` の「3 行」と較正の見本をまた手で直す（procedure.md の「見直すきっかけ」が既にこの直しを促す形なので、困りはしない）

### 案 B: 数を決め打ちしない形に一般化する

- 較正の正しい見本を `$PUBLISH` から作る（名前ごとに項を作る）。
- `/release` の手順を「`[workspace.package]` の `version` と、`[workspace.dependencies]` の path 付きの行の `version` のすべて」と書き、行数を手順に書かない。
- ✅ 次にクレートを足すとき、道具と手順を直さなくて済む
- ❌ 較正の見本が `$PUBLISH` から作られると、「一覧の中身を誤って書いた」ときに見本も一緒に誤り、較正が誤りを見逃す（較正は「道具が壊れたまま緑を出さない」ためのもので、見本は道具の外の固定値である方が強い）
- ❌ `/release` の 3.2 の判定（「足した行 N・消した行 N」）を数え方の文に書き換える必要があり、要件 5.7 の「行の数に合わせる」より大きな変更になる。`/release` は承認の止まりなしに回る手順なので、判定をあいまいにする変更は危うい

### 案 C: 混合（道具は案 A・手順書と steering は案 A・`/release` の判定だけ数を明記したまま「見直すきっかけ」を強める）

- 案 A をそのまま行い、`/release` の「見直すきっかけ」の覚え書き（389 行目「3 行目が増えたら段 2 と 3.2 を直す」）を「4 行目が増えたら」に書き直すだけにする。
- ✅ 案 A と同じ大きさで、次の直しの入口も残す
- ❌ 案 A と実質同じ（違いは覚え書きの書き方だけ）

**見立て**: 要件は 4.4・5.7 で「3 つ」「行の数に合わせる」と、決め打ちの形を求めている。案 A（＝案 C）が要件に素直で、差分も最小である。案 B は要件の外の一般化になる。決めは討議で。

## 6. 規模と危うさ

- **規模: S（1〜3 日）**。写しは道具 1 つで取り出せ、manifest・依存は 1〜2 行ずつ、道具は 2 か所、文書は 5 つ。brief の見立て S（5〜8）と合う。
- **危うさ: 低〜中**。低い理由＝型のなぞりで、未公開の版でも包めることを試しで確かめた。中の理由＝⑴ 新しく全体テストに入る MessageBox のテストが、並走するワークツリーと画面の上でぶつかる余地（揺れると原因が分かりにくい） ⑵ Trusted Publishing の設定が開発者の操作で、済まないまま公開の段が回ると、一部のクレートだけが上がった状態になりうる（要件 5.3・7.1 が手当て済み） ⑶ 元のリポジトリの archive は外から見える、取り消しにくい操作（要件 6.1 が許しを求める）。

## 7. 討議に出す議題（推奨つき・決めは討議で）

1. **取り込みのコミットの切り方と「1 字違わず」の確かめ方**: ⑴ `4f20275` の 10 個をそのまま入れるコミット ⑵ `cargo fmt` だけのコミット ⑶ manifest・依存・道具・文書のコミット、に分ける。⑴ の後で `git ls-tree` の blob の番号を `4f20275` と 1 つずつ比べれば、要件 1.3 を機械で確かめられる（areka の作業木は `core.autocrlf=true`・索引は LF で、元のリポジトリの索引も LF）。⑵ の差分は rustfmt の出力だけであることを示せる。**推奨: 3 つに分ける**（squash マージで 1 つになるが、PR の中で確かめが読める）。
2. **整形の差分を受ける**: rustfmt 1.11 で `lib.rs` 7 か所・`window.rs` 1 か所（56 行）。要件 1.3 の範囲内。**推奨: 受ける**（受けないと `-Format` なしの全体テストが赤）。
3. **写しの依存をワークスペースの依存へ寄せない**: `windows`・`async-task` は直書きのまま。**推奨: 寄せない**（寄せると公開する manifest の機能が 4 つ→27 に変わり、要件 1.7 と食い違う）。
4. **manifest の areka の型への寄せ方**: 他のクレートが持つ `workspace = "../.."` と `authors.workspace` を足すか。`workspace` の行は動きに響かない。`authors` は元に無く、足すと crates.io に出る説明が変わる。`version`・`repository` は `.workspace = true` の形で書くか、値を直書きするか。**推奨: `version.workspace`・`repository.workspace`・`workspace = "../.."` を足し、`authors` は足さない**。
5. **`publish` の行**: 出すクレートなので `publish = false` は書かない。`dola` は `publish = true` を明記している。**推奨: `dola` に倣って `publish = true` を書く**（要件の「公開の扱い」を目に見える形に）。
6. **較正の見本の直し方**: 正しい見本 JSON に `wintf-winmsg-executor` の項を手で足す（案 A）か、`$PUBLISH` から作る（案 B）か。あわせて「一覧」の較正に「写しが出さない」誤りの見本（今の「wintf does not publish」と同じ形）を足すか。**推奨: 手で足す。誤りの見本は足さない**（「一覧から外れたクレート」を赤にする力は、今の `wintf` の見本が既に示している＝要件 4.4 を満たす）。
7. **`/release` の直し方**: 行数を 2→3 に決め打ちで直す（要件 5.7 の字面）か、行数を書かない形に一般化するか。**推奨: 決め打ちで直す**（3.2 の判定の強さを保つ。「見直すきっかけ」は「4 行目が増えたら」に直す）。
8. **新しく全体テストに入る MessageBox のテスト**: 写しの `src` は変えられない（要件 1.3・境界外）。そのまま `cargo test --workspace` に入れるか、全体テストから外すか（`--exclude`）。**推奨: そのまま入れる**（外すと土台のクレートのテストが 1 度も回らない。揺れが出たら原因の場所＝題名の取り違えが分かっているので、そのときに起票する。直すのは下流の移行の spec か、別の spec）。
9. **要件に名指しの無い古い記述**: ⑴ スキル `/update` の `SKILL.md` の 3 か所（`=` の行の一覧に `wintf-winmsg-executor`・自前のクレートの例・版の表記の 2 行） ⑵ `structure.md` の「その他の最上位」の `crates-io.yml` の説明（`wintf`・`dola` を出す） ⑶ `tech.md` 154 行目の「UI スレッド基盤の外部クレ化」の文 ⑷ `crates/pilot/Cargo.toml` の注記 2 行。**推奨: 4 つとも同じ PR で直す**（どれも同じ事実の書き換えで、残すと次の `/update`・`/release` が古い一覧を読む。⑴ はスキルの文書で要件 5 の手順の外だが、同じ理由で範囲に入れる）。
10. **元のリポジトリの確かめと後始末の進め方**: 要件 1.5 の「`origin/main` と食い違う」を、手元の追跡の枝で判定するか、先に `git fetch` してから判定するか（fetch は元のリポジトリの手元の写しの追跡の枝を書く。作業木と `main` は動かさない）。後始末は、向こうの決まりどおり README の PR → squash マージ → `gh repo archive` の順。**推奨: fetch してから判定する**（要件 1.5 の狙いは「GitHub の `main` と同じ物を写す」こと。fetch は向こうの作業木を汚さない）。README の PR と archive は、要件 6.1 どおり文と操作を示して許しを得てから。
11. **Trusted Publishing の設定の時期**（情報）: 写しのクレートは crates.io に既に在るので、設定は取り込みの前でも後でも開発者がいつでもできる。要件 7.1 は完了の報告で頼む形。**推奨: 要件のまま**（早く頼んでもよいが、作業は変わらない）。

## 8. 設計で調べること（Research Needed）

1. **謝辞の作り直しで著作権の 2 行が残るか**（要件 3.9）: cargo-about は、今は crates.io から取り出した物のフォルダの `LICENSE-MIT` を読んで、独立の塊を作っている。path のクレートになった後も、クレートのフォルダの `LICENSE-MIT` を同じように読むはずだが、確かめていない。全体テスト `-License` の `cargo about generate` の後に、`THIRD-PARTY-NOTICES.md` の該当の塊を読んで確かめる（版の行が `0.0.5`→`0.1.0` に変わる以外の差が無いこと）。
2. **MessageBox のテストの並走時の揺れ**: 同じ机で 2 つの全体テストが同時に `cargo test --workspace` を回したとき、題名で引く 5 本が他のプロセスの窓を閉じて止まる（固まる）か。重い走行はしない決まりなので、設計では「起きうる・起きたら起票」の扱いにとどめるか、軽い確かめ（そのクレートのテストだけを 2 つ同時に数回）をするかを決める。
3. **Trusted Publishing の設定が無いまま公開の段が回ったときの止まり方**（要件 5.3 の文の正しさ）: 短命の鍵は設定の在るクレートにしか効かない見込みで、cargo の複数クレートの公開は、依存の無い `dola`・写しのどちらを先に上げるかを自分で決める。`dola` だけ上がって止まる、または何も上がらずに止まる、のどちらも起きうる、と手順書に書けばよいか。crates.io の Trusted Publishing の説明を読んで裏づける（実際には試さない）。
4. **v0.2.0 の公開の決め（roadmap「棚卸㊲の裁定」の 8）との関係**（下流への申し送り）: 移行のあいだ `wintf` を crates.io へ出さない案 B になったとき、写しも一覧から外すかは、この spec の範囲外（この spec は一覧を 3 つにする）。設計の境界の節に、下流が決めることとして書く。

## 9. 次の段

- 要件の討議（`/kiro-requirements-discussion wintf-winmsg-executor-import`）で、7 節の議題を決める。議題 9 は要件の境界（In scope）に足すかどうかの決めになる。
- その後、`/kiro-design wintf-winmsg-executor-import` で設計へ進む。設計では 8 節の 1〜3 を確かめ、7 節の決めを design.md に写す。

## 10. 設計の段の調べと決め（2026-11-14・`/kiro-spec-design -y`）

- 調べの種類: light（既に在る型の拡張）。サブエージェントは使わず、主の文脈で Grep／Read と WebFetch・`gh api` だけ。手元の試しは組み立てなし（読むだけ）。置き場は `target\design-research\`（cargo の `cargo_publish.rs` の写し・git に入れない）。
- 使ったスキルとその指針: `kiro-spec-design`（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）。開発者の裁定のメモ: windows-rs の移行は bindgen（`windows` を使わない）・外部クレートはそのまま使う・`windows-core` は `windows` と同じ系列・crates.io の公開の決め（`wintf`・`dola` だけ→今回 3 つ）。
- 2026-11-14 の要件の討議で決まった物（設計の前提）: 写しの MessageBox のテストは全体テストに残し UI テストとして排他テストの机で回す・名指しの無い古い記述 4 か所も書き換える・要件 4.9（areka の CI が出す）。

### 10.1 Research Needed の答え

1. **謝辞の著作権の 2 行**（要件 3.9）: 今の `THIRD-PARTY-NOTICES.md` の 3360〜3368 行目は、`wintf-winmsg-executor 0.0.5` だけの「MIT License」の塊で、著作権の 2 行を持つ。cargo-about は `MIT OR Apache-2.0` から `accepted` の先頭の `MIT` を選び、クレートのフォルダのライセンスのファイルを出どころ（crates.io／path）に依らず読むので、写しの `LICENSE-MIT` から同じ塊を作る見込み。実物の確かめは全体テスト `-License` の後の差分で行う（設計の Validation）。外れたときの予備は `about.toml` の `clarify`。
2. **MessageBox のテストの並走の揺れ**: 写しの `src/lib.rs` を読んだ。`FindWindowW` で窓を探すのは `running_spawned_with_modal_dialog`・`message_loop_with_modal_dialog` の 2 本だけで、題名はどちらも一意。同じ題名 `reenter_filter_closure` の 3 本は窓を探さない（入れ子のメッセージループが終わると MessageBox も閉じる）。全体テストは排他テストの机（`doc/impl-watch.md`）で 1 本ずつしか回らないので、別のワークツリーの全体テストとも重ならない。**決め**: 軽い並走の試しはしない（負荷テストを繰り返さない決まり）。揺れが出たら起票する。
3. **Trusted Publishing が 1 つだけ無いときの止まり方**（要件 5.3）:
   - crates.io の鍵の交換（`src/controllers/trustpub/tokens/exchange/mod.rs`）は、持ち主・リポジトリ・ワークフローのファイル名・environment が合う設定を集め、1 つも無ければ 400（`No Trusted Publishing config found for repository ...`）。在れば、合った設定のクレートの番号（`crate_ids`）だけを鍵に記録する。鍵の有効は 30 分。
   - cargo の複数クレートの公開（`src/cargo/ops/registry/publish.rs`・`PublishPlan`）は、依存の済んだクレートの集まり（`BTreeSet<PackageId>`＝名前の順）を `pop_first` で 1 つずつ上げ、上げに失敗すると `?` でそこで止まる。失敗の文には `note: the following crates have not been published yet:` と残りの名前が付く。依存の済んだ集まりを全部上げてから、索引に載るのを待って次の集まりへ進む。
   - よって `dola`・`wintf-winmsg-executor`・`wintf` の 3 つが残っている回で、写しだけ設定が無いと、鍵は `dola`・`wintf` に効く形で出て、cargo は `dola` を上げた後に写しで断られて止まる＝「`dola` だけ上がった」状態。`dola` が既に出ている回なら何も上がらない。要件 5.3 の文（「途中で止まり、一部のクレートだけが上がった状態になりうる」）は正しい。断られたときの crates.io の文は確かめていない（手順書には書かない）。
   - 出典: crates.io の `main` の上のファイル（WebFetch・2026-11-14）・cargo の `master` の `src/cargo/ops/registry/publish.rs`（`gh api`・2026-11-14）。何も公開していない。
4. **v0.2.0 の公開の決め（案 A／案 B）との関係**: この spec は一覧を 3 つにする。案 B になったときに写しも外すかは、その決めを持つ下流の spec が決める（design.md の Out of Boundary に書いた）。

### 10.2 ギャップ分析の議題の決め（設計で決めた物）

| 議題 | 決め | 理由 |
|---|---|---|
| 1 コミットの切り方 | 4 つ: ⑴ 10 個の写し（本文に元と `4f20275`）⑵ manifest・依存・`Cargo.lock`・一覧と見本・公開の段の注記 ⑶ `cargo fmt --all` の出力だけ ⑷ 文書 | ⑴ で blob の番号を突き合わせられ、⑶ が整形だけと示せる。ギャップの案から整形と manifest の順を入れ替えた（⑴ の直後はワークスペースが解決できないことがあるので、整形は解決できる ⑵ の後に回す）。依存と一覧は包む段が赤にならないよう同じコミット |
| 3 依存をワークスペースへ寄せない | 寄せない（直書き） | 寄せると公開する manifest の `windows` の機能が 27 になり要件 1.7 と食い違う。メモの「版を直書きしない」は `windows-core` の型の割れを防ぐ決まりで、写しは `windows-core` を直接使わず、`Cargo.lock` の `windows` は 1 版のまま |
| 4 manifest の寄せ方 | `workspace = "../.."`・`version.workspace`・`repository.workspace` を足す・`authors` は足さない | 他のクレートと同じ形。`authors` は crates.io の説明を変える |
| 5 `publish` の行 | `publish = true` を書く | `dola` に倣う |
| 6 較正の見本 | 正しい見本に 1 項を手で足す・誤った見本は足さない | 見本は道具の外の固定値が強い。今の `wintf does not publish` の正規表現は `"wintf"` の閉じの引用符まで含むので写しの項に当たらない（読んで確かめた） |
| 7 `/release` の直し方 | 行数を 3 に決め打ちで直す・「見直すきっかけ」は「4 行目が増えたら」 | 3.2 の判定の強さを保つ。`Cargo.lock` と謝辞の判定は `N` を使うので直さない |
| 10 元のリポジトリの確かめ | `git fetch` してから 3 条件を見る。後始末は一時の作業の写し（`target\` の下に clone）から README の PR → squash → archive | 要件 1.5 の文のとおり。手元の写しの作業木を動かさない |

### 10.3 設計で見つけた物

- 要件 5.10 の 4 か所のほかに、同じ事実の古い記述が 3 か所在った: `/update` の `SKILL.md` 102 行目（「`wintf` と `dola` は crates.io に出しており」）・`README.md` 60 行目（「crates.io に版を出しているのは…`wintf`・`dola` だけ」）・`doc/README.md` 34 行目。どれも 1 行で、要件の「同じ事実の古い記述を書き換える」の狙いに入るので、範囲に入れた（設計の討議で確かめてよい）。
- 残す物: 完了 spec・`crates/pilot/examples/` の報告書と例のコメント・`crates/shiori-host32-helper/src/main.rs` の doc コメントの「0.0.5」は、その時点に確かめた版の記録として残す。
- `/release` の段 3.2 の謝辞の判定（「`- {ワークスペースのクレートの名前} {旧版}`」）は、写しの塊の行 `- wintf-winmsg-executor {版}` にも当てはまる。`Cargo.lock` の判定の `N`（`cargo metadata` の `packages` の数）も写しで 1 増えるだけで合う。
- 元のリポジトリの README の移転の案内は、リンクをリポジトリの根（`https://github.com/ekicyou/areka`）と置き場のパスの文字で示す。取り込みの PR が本線に入る前に後始末をしても切れたリンクにならない。

### 10.4 まとめ（synthesis）

- **一般化**: 要件 4.x と 5.7 は「公開するクレートの数の決め打ちを 2→3 にする」1 つの問題の変種だが、一般化（案 B）は取らない。較正の見本と `/release` の判定は、固定値である方が誤りを見逃さない。
- **作るか使うか**: 新しく作る物は無い。写しは `git archive`、依存は `dola` の型、`Cargo.lock` は `cargo update -w`、版上げは既存の `cargo set-version --workspace`（path の行も上げる）、公開の順は cargo に任せる。
- **削る**: 道具に誤った見本を足さない・`test-all.ps1`／`deny.toml`／`release.yml` は直さない・`about.toml` は外れたときだけ・写しのテストは直さない。

### 10.5 危うさの見直し

- 写しだけ Trusted Publishing が無いまま最初のリリースのタグが打たれる → `dola` だけ上がる。手順書 2・5 節と完了の報告で手当て（手順書には止まり方とやり直しを書く）。
- 謝辞の塊が崩れる → 全体テスト `-License` の差分で見つけ、`about.toml` の `clarify` で直す。
- 写しの MessageBox のテストの揺れ → 排他テストの机で起きにくい。起きたら起票。

## 11. 設計の討議の決め（2026-11-14・開発者の一括承認）

1. **Trusted Publishing の設定は今頼む**（検証の指摘 1）: 写しは crates.io に既に版を持つので、設定（`ekicyou`／`areka`／`crates-io.yml`／environment は空）は今すぐできる。討議の場で開発者に頼んだ。道具では読めないので、完了の報告の前に開発者へ済んだかを尋ね、報告で「済み」か「残り」かを示す。
2. **元のリポジトリの後始末の許しは今得た**（検証の指摘 2）: README の先頭に足す文と操作（README の PR → squash マージ → `gh repo archive`）を design.md に固定し、それに対して許しを得た。実装の最後のタスクで聞き直さずに行う。areka の PR が本線に入る前に済むが、`/kiro-complete` のマージがすぐ後に続き、archive は戻せ、リンクはリポジトリの根を指すので、この順を取る。
3. **古い記述は 7 か所を直す**: 要件の討議の 4 か所に、設計で見つけた `/update` の `SKILL.md` 102 行目・`README.md` 60 行目・`doc/README.md` 34 行目の 3 か所を足した（要件 5.10 も 7 か所に改めた）。

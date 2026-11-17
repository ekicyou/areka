# ギャップ分析: audit-test-kits

> 2026-11-17 `/kiro-validate-gap`（`/kiro-start` の段から）。物差し＝ponytail の `/ponytail:ponytail-audit`。範囲＝`crates/real-run-kit/`・`crates/log-capture-kit/`・`crates/sample-ghost-kit/`・`crates/temp-path-kit/` の全部。audit の作法のうち「最大 20 件」と「読まなかった所（Not checked）」は外した（要件 1.3・1.4）。コードは何も変えていない。

## 1. 測り直し（要件 1.2）

- **取り込んだ `main`**: `5749a4c7`（`git fetch` で `origin/main` を確かめた・この 4 crate の差分は 0）。作業ブランチは `5749a4c7` の上に spec の初めのコミット `74996218` が 1 つ乗っただけ。
- **物差しの版**: いま入っている ponytail は **5.3.3**（`~/.claude/plugins/installed_plugins.json`・11-06 更新）。brief が書いた 5.3.2 と比べて `skills/ponytail-audit/SKILL.md` は 1 バイトも違わない＝物差しは同じ。
- **大きさ**（`.rs` 58 本・`Cargo.toml` 4 本＝62 ファイル。テストは `_tests.rs`・`tests/`、`src/bin/`・`examples/` はコードに数えた）:

| crate | ファイル | コード | テスト | `.rs` の計 |
|---|---|---|---|---|
| real-run-kit | 20 | 1,922 | 1,661 | 3,583 |
| log-capture-kit | 21 | 1,112 | 5,537 | 6,649 |
| sample-ghost-kit | 18 | 2,520 | 3,753 | 6,273 |
| temp-path-kit | 3 | 153 | 178 | 331 |
| 計 | 62 | 5,707 | 11,129 | 16,836 |

- **読んだ所**: 62 ファイルを全部、頭から終わりまで読んだ（読み分け: real-run-kit 1 人・log-capture-kit の `src/` と較正 1 人・log-capture-kit の見張り 7 本 1 人・sample-ghost-kit の本番 1 人・sample-ghost-kit のテスト 1 人・temp-path-kit はまとめ役が自分で）。まとめ役が主な指摘（1・2・3・6・17・18 ほか）を原文で確かめ直した。読まなかった所は **0**。
- **想定した負荷**: 1 人の開発者が 1 台の PC で、`cargo test --workspace` と全体テスト（`tools/test-all.ps1`）を回す。ただし同じワークツリーで複数のテストのプロセスが同時に走る（cargo は crate ごとに別プロセス・libtest は 1 プロセスの中で並列）。実機の試験は環境変数で明示したときだけ回る。

## 2. この範囲がしていること

試験の道具の 4 crate で、どれも `[dev-dependencies]` からだけ引かれる（本番のコードには入らない）。`real-run-kit` は本物の areka を子として立て、MCP で話し、利用者と同じ閉じ方で閉じ、落ちた回の証拠を `target\test-roots\` に残す。`log-capture-kit` は tracing の記録の捕まえを硬くし（対照の記録を必ず要求する窓）、`tests/` にワークスペース全体の見張り（1,000 行・一時パス・検体の窓口・`with_default`・文書の木）を持つ。`sample-ghost-kit` は検体のゴーストとバルーンを名前で引き、`.nar` の取り寄せの棚・親の写し・使い捨ての写しを管理する。`temp-path-kit` はプロセス間で一意な一時フォルダを配る。

## 3. 指摘の一覧（要件 1.3・1.5〜1.7）

番号は 3 つの組を通した番号（範囲を分けて読んだので、まとめた後に重なりを除いて振り直した）。以後の段はこの番号で指す。

**各指摘の印**（要件 1.6・1.7・brief の Constraints）:
- **〔外〕** 外への約束（公開の名前と振る舞い・実行体の名前／引数／終了コード／出す行・環境変数・置き場の名前・見張りの決まりと例外表）が変わる。後ろに範囲の外の読み手を書く。
- **〔外のファイル〕** 直しが範囲の外のファイルの書き換えを要る。
- **〔C22〕** その読み手やファイルが C22 のほかの 5 本の触るファイルに当たる。
- **〔重〕** 確かめるのに要件 8.1 の操作（本物の areka の実機の走り・`vendors/` の書き替えなど）が要る。
- **〔残⑴〕〔残⑵〕〔残⑶〕** brief の「直さずに残してよい 3 種」のどれかに当たりうる（要件 2.2。要件の討議で裁定）。
- **〔持ち主〕** 文の持ち主が別の spec（`input-crate-split`・`session-module-gather`）。

### 3.1 必ず直す（5 件）

**1. 見張り ⑷ が `\` 区切りの同梱バルーンのパスを見逃す（いま実在する違反が 1 件）**
- 場所: `crates/log-capture-kit/tests/sample_path_guard_test.rs:136-163`（`Form::tokens`。⑵ 139-147・⑷ 149-161）
- 何か: 検体の窓口を迂回して「展開形の検体フォルダ」⑵ や「ゴーストのフォルダ＋同梱バルーン名」⑷ を手で組む書き方を見つける走査語。
- 問題: 走査語が `/` 区切りと普通の `"…"` しか知らない。Windows の `\` 区切りは引っかからない。
- 起きる場面: `crates/areka/tests/window_link_real_run.rs:448-449` が今 `sample.folder().join(r"balloon\pasta_tategaki")` と書いている。窓口 `sample.balloon("pasta_tategaki")` を通すべき形そのものだが、見張りは緑。⑵ も `r"vendors\sample_ghost\emo2\ghost"` が通る（実在は 0）。
- 直し方: ⑷ と ⑵ に `\` の形の語を足す（生の文字列と `\\` の両方に当たる形・約 +6 行）。見張りが赤にする 2 か所を直す＝`window_link_real_run.rs:448` を `sample.balloon(..)` に、449 行（写し先の根）と `crates/areka-nar/src/error_tests.rs:205`（`r"C:\root\balloon\emo2-kakukaku"`・誤りのテストの偽のパス）を、登記の検体名に当たらない綴りへ。較正の対（違反を見つけて赤）を足す（要件 4.7）。
- 要件: 3.1・3.2・4.1・4.7・7.6・7.7
- 印: 〔外〕見張りの決まりが厳しくなる（読み手＝`.kiro/steering/structure.md:441` の 4 形の説明）。〔外のファイル〕`crates/areka/tests/window_link_real_run.rs`・`crates/areka-nar/src/error_tests.rs`（どちらも C22 の 5 本の触るファイルではない。`window_link_real_run.rs` は C23 の `input-crate-split` が 49 行を触る予定＝別の行）。〔重〕`window_link_real_run.rs` は環境変数で明示したときだけ回る実機の試験＝書き換えた後の効き目は実機でしか確かめられない（コンパイルと見張りは常時のテストで確かめられる）。`sample_path_guard_test.rs` は 926 行＝足しても 1,000 行を越えない見込み（指摘 33・34 で減らせる）。

**2. `fold-samples` が同梱バルーンを古い置き場へ写して照らすので、同梱バルーンを持つゴーストでは往復の確かめが必ず落ちる**
- 場所: `crates/sample-ghost-kit/examples/fold-samples.rs:208-222`（211 行）・同ファイルの説明 37-38 行
- 何か: 追跡されている元のファイルを、本物のインストーラーが置く先へ写し、2 つの木をバイトで比べる見本の道具。
- 問題: 同梱バルーンを `<根>/balloon/<名>/…` へ写している。inner-balloon の spec の後、`areka-nar` は `<根>/ghost/<ゴースト>/balloon/<名>/…` に置く（`crates/areka-nar/src/plan.rs:291` `destination: ghost_dir.join(BALLOON_STORE).join(&companion.directory)`）。
- 起きる場面: `cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/emo2 --check` で、emo2 の `emo2-kakukaku/` の 21 ファイルが「置かれなかった」と「元に無いものが置かれた」の両方に出て、終了コード 1。同梱バルーンを持つ新しい検体を畳む（`vendors/sample_ghost/README.md` の手順 3）ときも同じ。
- 直し方: 同梱バルーンの写し先を `root.join(store).join(&manifest.directory).join("balloon").join(&companion.directory).join(tail)` にし、説明 37-38 行を直す。写しの式（`installed_path`）を引数だけで決まる関数として単体テストで縛る（偽の `InstallManifest` で・`vendors/` に触らない）。
- 要件: 3.1・4.1・8.3
- 印: 外への約束は変わらない（見本の名前・引数・終了コードは同じ）。〔重〕道具をそのまま通しで回すと `git ls-files` の追跡された展開の木が要る＝`vendors/` の書き替えに当たる（要件 8.1）。写しの式の単体テストで筋まで縛り、通しの効き目は次に本当に検体を畳むときに確かめる形が要る（議題）。

**3. 実機の試験の窓口が、MCP の道具の「誤りの答え」を「つながらない」と取り違え、2 秒待ってから返す**
- 場所: `crates/real-run-kit/src/mcp.rs:182-185`（`Ok(Err(e)) => Outcome::Transport(e.to_string())`）・198-215（猶予の待ち）
- 何か: rmcp の `call_tool` の結果を、道具の答えか `McpError` へ分ける所。
- 問題: rmcp の `ServiceError` を全部「運ぶ道の失敗」にしている。生きたサーバーがきちんと返した JSON-RPC の誤り（`ServiceError::McpError`）も入る。窓口はつなぎを捨て、終わらない子を `exit_grace`（2 秒）待ち、`McpError::Unreachable`「MCP につながらない」を返す。`call_until` もその場で Err を返す。
- 起きる場面: `crates/areka-mcp/src/handler.rs:69-73` は引数が inputSchema に合わないと `-32602` を返す。`run.call("get_log", r#"{"log_type":5}"#)` や知らない道具の名前を呼ぶと、2 秒止まって「つながらない」と出る。サーバーはつながっていて、答えている。
- 直し方: `Ok(Err(ServiceError::McpError(data)))` を分け、つなぎを使い回し、猶予の待ちを飛ばして返す（約 6 行）。返す形は 2 案＝今ある `Unreachable` に理由を入れて返す（外への約束は変わらない）か、新しい変種を足す（〔外〕）。偽の子（`listen;window` で `echo` に型の合わない引数）で決定論のテストを足す。
- 要件: 3.1・4.1・7.6
- 印: 新しい変種を足すなら〔外〕（`McpError` を照らす読み手＝`crates/areka/tests/emo2_real_run/common.rs:238-244`〔何でも受ける腕で影響なし〕・`common_tests.rs:129-143`・`crates/areka/tests/emo2_mcp_real_run.rs:124`・`doc/testing.md:58-64` の 3 変種の説明）。既にある `Unreachable` のままなら外への約束は変わらない。確かめは偽の子で足りる（重い操作は要らない）。

**4. `env-filter` を `wintf` のほかで有効にする道が 3 つ、見張りに見えない**
- 場所: `crates/log-capture-kit/tests/with_default_guard_test.rs:243`・276-282・531-544
- 何か: 道具の機能 `env-filter` を宣言してよいのは `wintf` だけ、を確かめる見張り。
- 問題: ⑴ 道具の自分の `Cargo.toml` を読まないので `[features] default = ["env-filter"]` が見えない。⑵ ワークスペースの根の `Cargo.toml` を読まないので `[workspace.dependencies]` で機能を付けて各 crate が `workspace = true` で引く形が見えない。⑶ `"env-filter"` の二重引用符の形しか照らさないので、TOML で正しい `['env-filter']` が通る。
- 起きる場面: 道具の `Cargo.toml` に `default = ["env-filter"]` を 1 行足すと、21 の消費 crate 全部が `capture_under_filter` を使えるようになり、見張りは緑のまま（このファイルの説明 11-14 行は、これを唯一の守りと書いている）。
- 直し方: 引用符を外して `env-filter` を照らす・道具の `[features]` の `default` に `env-filter` が無いこと・根の `[workspace.dependencies]` の道具の項に `env-filter` が無いことを確かめる（約 +8 行）。各道に較正の対を足す。
- 要件: 3.1・4.1・4.7・7.6
- 印: 〔外〕見張りが厳しくなる（読み手＝`structure.md:441`・`logging.md:128`）。いま違反は 0＝範囲の外のファイルの書き換えは要らない。確かめは常時のテストで足りる。

**5. 一時フォルダの入口を `(` 無しで渡す書き方と、別名で引き込む書き方が、見張りに見えない**
- 場所: `crates/log-capture-kit/tests/temp_path_guard_test.rs:72`・81
- 何か: OS の一時フォルダの入口（`temp_dir`）を、窓口 `temp-path-kit` の外で使う書き方を見つける 2 つの走査語。どちらも `(` で終わる。
- 起きる場面: `cache.unwrap_or_else(std::env::temp_dir).join("fixed")` と、`use std::env::temp_dir as tmp;` の後の `tmp().join("fixed")` が、どちらも緑（いま実在は 0）。
- 直し方: 修飾つきの語から `(` を外す（`use` の行にも当たる）。この見張り自身の説明 5・539 行の逆引用符の綴りを言い換える（自分の生の文字の検査を緑に保つ）。較正の対を足す。
- 要件: 3.1・4.1・4.7・7.6
- 印: 〔外〕見張りが厳しくなる（読み手＝`structure.md:441`）。いま違反は 0。読み手の報告は、`var("TEMP")` などの環境変数の読みまで広げる案も挙げたが、これは見張る決まりを増やす＝新しい機能（要件 3.4）に当たりうる。広げるかは議題（推す＝広げない）。

### 3.2 直したほうがよい（32 件）

**6. `TempPath` が、死んだプロセスの残した同じ名前のフォルダを中身ごと配る（`real-run-kit` は自前で 16 回取り直して避けている＝同じ決まりが 2 か所）**
- 場所: `crates/temp-path-kit/src/lib.rs:134-140`（`create_dir_all`）・`crates/real-run-kit/src/evidence.rs:11`・34-48（`CREATE_TRIES = 16` と空かどうかの確かめ）
- 何か: 名前を `areka-{札}-{pid}-{連番}` で組み、`create_dir_all` で作る。連番はどのプロセスも 0 から。
- 問題: `create_dir_all` は既にあるフォルダでも成功する。テストが途中で殺されると（Ctrl+C・時間切れの kill）フォルダが残り（破棄が走らない）、Windows は pid をすぐ使い回すので、後のプロセスが同じ名前を引くと前の回の中身が入ったフォルダを受け取る。
- 起きる場面: 前の回が `areka-install-1234-0\ghost\…` を残して殺され、次の回の pid が 1234 になる。「空の根へインストールして、置かれたファイルの一覧を比べる」テストが、前の回のファイルを見て赤になる（または「無いこと」の主張が前の回の残りで崩れる）。`real-run-kit` だけはこれを知っていて、空でなければ取り直す輪を自前で持つ。
- 直し方: 置き場は `create_dir_all`、葉は `create_dir` で作り、`AlreadyExists` なら次の連番で取り直す（約 5 行）。`real-run-kit/src/evidence.rs` の取り直しの輪を消す（`evidence_tests.rs` の空でないフォルダ・16 回の上限のテストも合わせて改める・Lean 約 -15 行）。同じ名前の残りを置いてから配らせる決定論のテストを足す。
- 要件: 3.1・3.2・4.1・4.8
- 印: 公開の名前と置き場の名前は変わらない（振る舞いは「残りを配らない」へ強くなるだけ）。範囲の外の 97 ファイル（`TempPath::new` 244 か所・`under_target` 390 か所）は書き換え不要。確かめは常時のテストで足りる。

**7. `WorkDir` が、死んだプロセスの残した同じ名前の札やフォルダで作れずに落ちる**
- 場所: `crates/sample-ghost-kit/src/devroot.rs:193-214`
- 何か: 作業フォルダ `<pid>-<連番>` と生存の札 `.lock` を作る。札は `create_new`、フォルダは `create_dir`。連番はどのプロセスも 0 から。
- 問題: 殺されたプロセスが残した `1234-0.lock` や `1234-0\` に、pid を使い回した後のプロセスが当たると `AlreadyExists` で `Io { what: "生存の札の作成" }` になり、テストが落ちる。`WorkDir::new()` は先に掃除をしない（掃除は `fresh_root_in` だけ）。残りのファイルを誰かが掴んだままだと、掃除でも退けられず、その pid と連番は毎回落ちる。
- 起きる場面: 全体テストの途中で止めた後、次の `cargo test -p areka-nar`（`WorkDir::new()` を直に約 110 か所で呼ぶ）が、たまたま同じ pid を引いて赤になる。
- 直し方: `open_lease` か `create_dir` が `AlreadyExists` なら札を退け、次の連番で取り直す（約 6 行の輪）。同じ名前の残りを置いてから作らせるテストを足す。
- 要件: 3.1・4.1
- 印: 外への約束は変わらない。確かめは常時のテストで足りる。

**8. `fold-samples` が往復の確かめの前に `vendors/` の `.nar` を上書きする**
- 場所: `crates/sample-ghost-kit/examples/fold-samples.rs:131-133`（`fold_tree(&stage)?.write_to(&nar)`）→ 135-175（確かめ）
- 問題: 確かめが食い違いを出したときには、正しかった `.nar` はもう消えている（git から戻すしかない）。指摘 2 と重なると、emo2 を畳むと `emo2.nar` が置き換わってから落ちる。
- 直し方: 作業用の置き場へ書き、確かめが 0 件のときだけ `vendors/` へ写す。
- 要件: 3.1・6.2
- 印: 終了コードと出す行は同じ。〔重〕通しの確かめは指摘 2 と同じく `vendors/` の書き替えが要る＝順序の入れ替えは単体の筋（書き先を引数で受ける）で縛る形が要る。

**9. 閉じ忘れた実機の試験で、子を力ずくで止めても緑になり、証拠も消える**
- 場所: `crates/real-run-kit/src/run.rs:196-206`（`Drop for Run`）・`crates/real-run-kit/src/close.rs:101-117`（`stop_quietly`・`eprintln` だけ）
- 何か: テストが `close()` を呼ばずに終わったとき、まだ動いている子を閉じる所。
- 問題: 子が閉じる求めに答えないと、破棄は締め切り（既定 60 秒）まで待って kill し、1 行 `eprintln` を出すだけ。通ったテストの標準エラーは libtest が捨て、`RunDirs` はパニック中でないので回の置き場を消す。力ずくの終わりが何も残らない。
- 起きる場面: `close()` を呼ばないテストで、areka が WM_CLOSE で固まる（この試験が捕まえたい種類のバグ）と、60 秒後に緑で通り、証拠は残らない。
- 直し方: `stop_quietly` が kill したかを返し、破棄の中で `killed && !std::thread::panicking()` なら理由を書いて panic する（二重のパニックにならない・約 4 行）。偽の子（`window-deaf`＋短い締め切り）で決定論のテストを足す。
- 要件: 3.1・4.1・6.2・6.3
- 印: 〔外〕破棄の振る舞い（今まで緑だった「閉じ忘れ＋子が固まる」が赤になる）。いま `close()` を呼ばずに破棄へ任せている範囲の外の呼び手で、子がきちんと閉じるものは影響なし。確かめは偽の子で足りる。

**10. `.env("AREKA_MCP_PORT", …)` で番号を上書きすると、窓口が違う番号へ話しかける**
- 場所: `crates/real-run-kit/src/launch.rs:116-121`（`env` は何でも上書きできると書く）・170（`port = self.mcp_port.unwrap_or_else(free_port)`）・185・`crates/real-run-kit/src/mcp.rs:18-19`（説明）・`crates/real-run-kit/src/run.rs:79-82`（`mcp_port()`）・`launch_tests.rs:70`・82（`AREKA_MCP_PORT=9` が勝つことを縛る）
- 問題: 子は呼び手の値で待ち受けるが、`McpChannel` と `Run::mcp_port()` は窓口が選んだ空きの番号のまま。
- 起きる場面: `.env("AREKA_MCP_PORT","9900")` の後に `run.call(..)` すると、「待受を始めた」の行を見たうえで空きの番号を探し、`Unreachable`「areka と名乗る相手が居ない」になる。`run.mcp_port()` は子が使っていない番号を返す。
- 直し方: `spawn` で、最後に積まれた `AREKA_MCP_PORT` が u16 として読めればそれを `port` に使い、読めなければ理由を書いて panic し `.mcp_port(n)` を案内する（約 6 行）。`mcp.rs:18` の説明を直す。偽の子で決定論のテストを足す。
- 要件: 3.1・4.1
- 印: 〔外〕この上書きの振る舞いが変わる（今は壊れている形が動くようになる）。範囲の外で `AREKA_MCP_PORT` を `.env` で積む呼び手は 0（`crates/`・`tools/`・`doc/` を探した）。`mcp_port` の作り手を指摘 35 で消すなら、直し方の案内先も合わせて決める。

**11. 32bit の仲介が `areka.exe` の隣に既にあると新しくならず、`HOST32_HELPER_EXE` も黙って無視される**
- 場所: `crates/real-run-kit/src/i686.rs:57-59`（`if is_i686_pe(&dest) { return dest; }`）・`i686_tests.rs:135-143`（この振る舞いを縛るテスト）
- 何か: 32bit の仲介（とテスト用の DLL）を `areka.exe` の隣へ揃える所。
- 問題: 隣が 32bit なら元を見ずにそのまま使う。環境変数が誤ったパスや新しいビルドを指していても確かめず、使われもしない。`doc/testing.md:93` は「立てるたびに…揃える（環境変数 → debug → release の順に探す）」と書いている。
- 起きる場面: 開発者が `shiori-host32-helper` を直して `--target i686-pc-windows-msvc` で組み直し、`cargo test -p areka --test smoke_boot_loop_exit` を回す。`target\debug\shiori-host32-helper.exe` は前に窓口が置いた古い 32bit の写しのまま＝古い仲介で試験が走り、何も知らされない。`HOST32_HELPER_EXE` に打ち間違いのパスを入れても何も起きない。
- 直し方: 先に元を決め（環境変数 → debug → release・環境変数の panic はそのまま）、隣が無い・32bit でない・元と違う（長さ＋更新時刻）ときだけ写す。環境変数が無く元も無いときだけ隣の 32bit を使う（約 8 行）。偽の PE で決定論のテストを改める。
- 要件: 3.1・4.1・4.8
- 印: 〔残⑴〕完了した spec `areka-P0-real-run-mcp-kit` の要件 2.1・2.5 が「隣が 32bit ならそのまま」と決めている＝その裁定を覆す直し（要件 2.4）。〔外〕`ensure` の振る舞い（範囲の外の呼び手＝`crates/areka/tests/smoke_boot_loop_exit.rs:238` の `staged.ensure_i686`〔隣は新しい検体の写しなので影響なし〕・`doc/testing.md:93`）。

**12. 記録のファイルの読み取りの失敗を空の文字として飲み込む読み手が 4 つ（同じ手助けの写しでもある）**
- 場所: `crates/real-run-kit/src/run.rs:217-219`（`read_lossy`・`unwrap_or_default`）・`close.rs:162-167`（`ended`）・`output.rs:114`（`failure_text`）・`mcp.rs:306-317`（`line_with`・`filter_map(.ok())`）
- 問題: 読めないファイル（ウイルス対策が掴んでいる・消えた）が "" になり、次の誤りを生む。
- 起きる場面: `!run.stdout().contains(X)`（`crates/areka/tests/emo2_mcp_real_run.rs:165`）が空のまま通る・`close()` が本当の終わり方でなく `EndedBy::Unlogged` を返す・失敗の文が「全 0 バイト」と出る・`ListenFailed` が探した語だけを持つ。
- 直し方: パスと理由を書いて panic する `pub(crate) fn read_text(path)` 1 つにまとめ、4 か所で使う（`failure_text` は誤りの文を埋め込む形でよい・Lean 約 -6 行）。読めないパスでの単体テストを足す。
- 要件: 3.1・4.1・6.2
- 印: 公開の名前は変わらない（読めないときに黙らず落ちるようになる）。確かめは常時のテストで足りる。

**13. `TempPath` の破棄が消せなかったことを黙って捨てる（`sample-ghost-kit` は同じ場面を知らせる＝同じ決まりが 2 通り）**
- 場所: `crates/temp-path-kit/src/lib.rs:155-158`（`let _ = std::fs::remove_dir_all(&self.path);`）。対照＝`crates/sample-ghost-kit/src/devroot.rs:696` 付近の `report_cleanup`（消せなかった木を知らせる）
- 問題: 子のプロセスがファイルを掴んだまま破棄されると、フォルダは残り、何も記録されない。`target\test-roots\` に残りが黙って溜まる（`real-run-kit` の通った回の後始末もこれに頼る）。
- 起きる場面: 実機の試験の回で子を閉じ切る前に置き場が破棄され、`areka-rr-…` が毎回 1 つずつ残る。誰も気付かない。
- 直し方: 消せなかったら、パスと理由を ASCII の 1 行で標準エラーへ出す（破棄の中なので panic しない・約 3 行）。掴んだファイルを残して破棄し、出した行を確かめるテストは標準エラーを読む形が要る＝決定論で縛れるかは設計で確かめる。
- 要件: 3.1・6.2・6.5
- 印: 外への約束は変わらない（消せたときの振る舞いは同じ）。

**14. 実行体のテストが、共有の `target\nar-samples\manual\<検体>\` の木を消して作り直す**
- 場所: `crates/sample-ghost-kit/tests/nar_sample_path_test.rs:86-149`（11 検体）・157-193（emo2 を 2 回）・本番 `src/devroot.rs:605-617`。同じ木の使い手＝`tools/package.ps1:35-36`・470・614・618・`tools/perf/perf-tool-base.ps1:381`・`tools/perf/invoke-perf-run.ps1:574`
- 問題: 引き継いだ `CARGO_TARGET_DIR` のまま `nar-sample-path` を回すので、本物の手動用の根を作り直す。`MANUAL` の鍵（32 行）はこのテストのプロセスの中だけを並べる。全検体の輪は取り寄せた pasta の検体も要るのに、その説明（80-85 行）が無い。毎回 11 検体を写して残す。
- 起きる場面: 開発者が同じワークツリーで `manual/emo2` を使う実機の走りや計測の走り（または `package.ps1` の写し）をしている最中に `cargo test -p sample-ghost-kit` を回すと、開かれていないファイルが全部消え、開かれたファイルで失敗して、テストは赤（終了 2）、走っている側は半分消えた木に残される。
- 直し方: 2 本とも既にある `run_in(private.path(), ..)`（47 行）で回す。輪を、並びの要所を覆う 3 検体（`claudia`〔バルーン 2〕・`R_POST_and_KOMAINU`〔0〕・`emo2-kakukaku-wplimit`〔Balloon の種類〕）に絞る＝取り寄せの検体への依存も消える。
- 要件: 3.1・4.4
- 印: 外への約束は変わらない（実行体の振る舞いは同じ・テストの置き場だけ）。

**15. 相対の `CARGO_TARGET_DIR` を違うフォルダの上で解く**
- 場所: `crates/sample-ghost-kit/src/devroot.rs:128-133`（`std::path::absolute`）・`devroot_tests.rs:166-181`（相対の場合をわざと縛るテスト）
- 問題: `absolute` は今のプロセスの作業フォルダの上で解く。cargo は cargo を起こしたフォルダの上で解くが、テストの実行体はパッケージのフォルダを作業フォルダにして走る。
- 起きる場面: `$env:CARGO_TARGET_DIR='tgt'` で根から `tools/sample-fetch.ps1` を回すと、取り寄せは `<根>\tgt\nar-samples\fetched\` に入る。`cargo test -p sample-ghost-kit` のテストは `<根>\crates\sample-ghost-kit\tgt\nar-samples` を探し、pasta の検体が `FetchedNarMissing`、しかも `work\`・`cache-2\`・`manual\` をソースの木の中に作る（`cargo clean` でも消えない）。
- 直し方: 相対の値なら、実行体の祖先のうちその値で終わる最も近いものを使い、無ければ今の探し方へ戻る。縛るテスト（166-181）を改める。
- 要件: 3.1・4.1・4.8
- 印: 〔外〕相対の値のときの振る舞いだけが変わる（読み手＝`vendors/sample_ghost/README.md:154`・169〔`vendors/` は触らない＝文が古びないかを設計で確かめる〕・`tools/sample-fetch.ps1:372-389` が読む `--fetch-list` の 4・5 列）。いまツリーの中で相対の値を設定する所は 0。

**16. `add_shell_copy` が、元のシェルが無いときに空のシェルのフォルダを残す**
- 場所: `crates/sample-ghost-kit/src/lib.rs:381-388`（`create_dir(&to)` が `copy_tree(shell/from)` より先）
- 起きる場面: `add_shell_copy("mastr", "second", "x")`（打ち間違い）が空の `shell/second/` を作って Err を返す。正しい名前でやり直すと「既にある」で落ち、本当の原因が隠れる。その写しのゴーストには空のシェルのフォルダが残る。
- 直し方: 先に `shell.join(from).is_dir()` を確かめる（または写しが落ちたら `to` を消す）。無い元を渡すテストを足す。
- 要件: 3.1・4.1
- 印: 誤りのときの振る舞いだけ。範囲の外の呼び手（`crates/areka/tests/stay_on_top_real_run.rs:560`・`crates/areka-runtime/src/switch_assets_tests.rs:45`・275・`crates/areka-runtime/src/assets_shell_tests.rs:29`・`crates/areka/src/shell_balloon_switch_session_lap_tests.rs:217`）は書き換え不要（`areka-runtime` の 2 本は C22 の `audit-emo2-boot-spine` の範囲に当たりうるが触らない）。

**17. 走査が、`target` や `vendors` という名前のフォルダを深さに関係なく飛ばす（その除外は何も確かめられていない）**
- 場所: `crates/log-capture-kit/tests/workspace_scan/mod.rs:48`・105-109・508-512・`workspace_scan_test.rs:352-367`
- 問題: 除外がフォルダの名前だけで、`crates/` の下のどこでも当たる。今そういうフォルダは無いので除外は空振りし、確かめのテストも何も測らずに通る。
- 起きる場面: `crates/areka-emo-present/src/presenter/target.rs`（実在のモジュール）を家の割り方で `presenter/target/mod.rs`＋`presenter/target/*_tests.rs` に割ると、そのフォルダのファイルが 4 つの見張り（1,000 行の番人を含む）から黙って消える。
- 直し方: `target` は親に `Cargo.toml` があるときだけ飛ばす（ビルドの出力）。`vendors` と `.git` は外す（走査は `crates/` から始まる）。一時の木で 1 本テストを足す。
- 要件: 3.1・4.7
- 印: 今の列挙の結果は変わらない＝外への約束は変わらない。

**18. 1,000 行の番人が、最後の上限越えのファイルが割られると赤になる**
- 場所: `crates/log-capture-kit/tests/file_length_guard_test.rs:234-239`（上限越えが 1 件以上あることを求める）・84・198-221（較正が例外表の実在の項を 1 つ外す）
- 問題: 例外表の項は減るのが本来の向きだが、持ち主が 4 本を割り終えると「計測か列挙が壊れている疑い」で赤になる。`CALIBRATION_DROPPED_ENTRY` の指すファイルを割ると、割る側が予定していない別の crate のこのファイルまで直す羽目になる。
- 起きる場面: `audit-pilot` が `pilot-clickthrough-alpha-toggle/main.rs` を消すか割り、`audit-placement-follow-residue`・`audit-emo2-boot-frame` が残りを割ると、最後の 1 本でこの見張りが赤。
- 直し方: 「1 件以上が上限越え」を計測の正しさの確かめ（最大のファイルが上限の半分を越える等）に置き換える。外す項を別の定数でなく `OVER_LIMIT_ALLOWED[0]` から取る。最後の項と一緒に外す較正は消してよい（合成の版が `workspace_scan_test.rs:287-300` に残る）。
- 要件: 3.1・4.7
- 印: 〔外〕見張りの例外表の扱い（読み手＝`roadmap.md:60`）。C22 でこの表に触るのはこの spec だけ（約束 ⑵）＝触ってよい。表に載るファイル自体は割らない（要件 7.5）。`temp_path_guard_test.rs:451-455`・197-201 も同じ形だが、その 1 項は残す本番のファイル。

**19. 列挙の較正が、ほかの spec が消すかもしれないファイルに頼る**
- 場所: `crates/log-capture-kit/tests/workspace_scan_test.rs:317-322`（319 行の pilot の見本）
- 起きる場面: `audit-pilot`（C22）が上限越えの `pilot-clickthrough-alpha-toggle/main.rs` を消すと、`walk_includes_examples_integration_tests_and_sibling_test_files` が赤。
- 直し方: 別の入れ子の見本（例 `crates/areka/examples/emo-present/balloon.rs`）を証人にする。
- 要件: 3.1・4.7
- 印: 外への約束は変わらない。`audit-pilot` の着地と同じウェーブ（C22）なので、先に直しておく値打ちがある。

**20. コメントを剥がす処理の危ない枝に較正が無い**
- 場所: `crates/log-capture-kit/tests/workspace_scan/mod.rs:199-244`（生の文字列の `#` の数合わせ `copy_raw_string` 293-305・逃がしの枝 213-217・234-238）。テストは `workspace_scan_test.rs:161-234` だけ
- 問題: `#` 付きで中に `"` を含む生の文字列・`\"`・バイト文字列（`b"…"`・`br#"…"#`）・`'\''`・CRLF の入力を試すテストが無い（本物のファイルは全部 CRLF・試料は全部 LF）。
- 起きる場面: `copy_raw_string` が `#` を数えなくなると、`let s = r#"a "// b"#; let p = std::env::temp_dir();` の生の文字列が中の `"` で閉じ、`// b` 以降が「コメント」になって呼び出しが消え、一時パスの見張りは緑。今のテストは 1 本も赤にならない。
- 直し方: 枝ごとに「走る行／隠れる行」の対を 1 本（約 25 行）。
- 要件: 3.1・4.1・4.7
- 印: 外への約束は変わらない。

**21. 道具の `expect_one`・`ExpectField` の失敗の道にテストが無い**
- 場所: `crates/log-capture-kit/src/expect.rs:16-35`（ちょうど 1 件の確かめ 29-33・欠けた項目の panic 19）
- 起きる場面: 29 行を `assert!(!hits.is_empty())` に変えても、使う 43 ファイルは全部緑のまま＝同じ記録を 2 度出す退行が全部の `expect_one` の呼び手を通る。
- 直し方: `src/expect_tests.rs` に、手で組んだ `CapturedEvent`（`event_tests.rs:244` と同じ形）で 0 件・2 件・欠けた項目の `#[should_panic]` と、通る場合 1 本（約 +30 行）。件 1（§5）の受け皿にもなる。
- 要件: 3.1・4.1・5.2
- 印: 外への約束は変わらない（使い手 43 ファイル）。

**22. `SampleRoot::acquire` の道の検査の値の確かめにテストが無い（2 本が覆うと名乗るが、鎖を自分で組んでいる）**
- 場所: `crates/sample-ghost-kit/src/devroot_fetched_tests.rs:122-133`・135-148。本番 `src/devroot.rs:577-579`（`fresh_root`）・`src/lib.rs:237-240`
- 問題: 2 本とも `resolve()`（`sample_nar_for` を直に）を呼び、127 行はテストの中で `resolve(..).and_then(cached_root_in)` を組む。`fresh_root` も `acquire` も通らない。
- 起きる場面: 誰かが `fresh_root` を URL の行で `fetched::fetched_path_in(..)` を使う形にするか、親の写しが無いときだけ確かめる形にしても、全部緑のまま `acquire("pasta-in-windows")` が改ざんされた（古い）`.nar` の写しを配る。
- 直し方: 自分の実行体を子として起こす既にある形（`devroot_cache_tests.rs:487-536`）で、私用の `CARGO_TARGET_DIR` に食い違う棚を置き、子の中で `acquire` が `Err(FetchedNarMismatch)` を返すことを確かめる 1 本。2 本の名前を実際に試す中身に合わせる。
- 要件: 3.1・4.1
- 印: 範囲の中だけ。ネットは要らない。

**23. `manual_root_in` が消せない木を断る枝にテストが無い**
- 場所: `crates/sample-ghost-kit/src/devroot.rs:606-618`（説明 591-593「消せない木を黙って配り直さない」）
- 起きる場面: 606 行を `let _ = std::fs::remove_dir_all(&manual);` に変えると、一部が掴まれた木の上に写しが重なり、古い `profile/` が残ったまま `nar-sample-path` が根を出して終了 0。赤になるテストは無い。
- 直し方: 私用の名前空間で 1 度作り、中のファイルを共有読みで開き（`devroot_tests.rs:146-150` の形）、2 度目が `Err(SampleError::Io { what: "手動用の根の作り直し", .. })` を返すことを確かめる（約 15 行）。
- 要件: 3.1・4.1
- 印: 範囲の中だけ。

**24. SHA-256 の 55 バイトの詰め物の境目にテストが無い**
- 場所: `crates/sample-ghost-kit/src/sha256.rs:36`・`src/sha256_tests.rs`
- 問題: テストは最後のブロックに残る長さが 0（空・100 万バイト）・3（`abc`）・56（2 ブロックの例）だけ。ちょうど 55 が残る場合（0x80 の印が 55 番目・長さが 56..64）を試さない。
- 起きる場面: `< 56` を `< 55` に変えても 4 本とも緑。長さを 64 で割った余りが 55 の取り寄せは毎回 `FetchedNarMismatch` になる（閉じる側に倒れるが、誤り）。
- 直し方: 55 バイトと 64 バイトの既知の答えを 2 つ足す。
- 要件: 3.1・4.1
- 印: 範囲の中だけ。自作の SHA-256 そのものは開発者の裁定（`sha256.rs:1-4`「裁定 2」・依存を足さない）＝指摘ではない（§6）。

**25. `.nar` の書き手の「同じものを組めば同じバイト」のテストが、名乗る退行で落ちない**
- 場所: `crates/sample-ghost-kit/src/nar_writer_tests.rs:123-136`・294-298。本番 `src/nar_writer.rs:39-42`（固定の DOS 時刻）・330（`sort_by_key`）
- 問題: 同じ瞬間に 2 回組んで比べるだけ。DOS 時刻は 2 秒刻みなので時計の時刻を書いても 1 回の呼び出しの中では同じ。NTFS の `read_dir` は既に名前順で、試料の名前はどちらの順でも同じ並び＝並べ替えを消しても緑。
- 起きる場面: DOS 時刻に `SystemTime::now()` を書くか並べ替えを消すと、`fold-samples` で畳み直した `.nar` のバイトが毎回変わる（git の差分と新しい写しの印）。赤になるテストは無い。
- 直し方: 1 つの解析で局所ヘッダの時刻と日付（オフセット 10..14）が `0x0000`・`0x0021` であることを確かめる。畳み込みのテストに NTFS の順とバイトの順が食い違う名前の対（`B.txt`・`a.txt`）を足す（約 +6 行）。
- 要件: 3.1・4.1
- 印: 範囲の中だけ。

**26. 頼る記録の出所 `areka::app_exit` が本物の areka と合っているかを確かめるテストが無い**
- 場所: `crates/real-run-kit/src/launch.rs:362`（`RELIED_TARGETS`）・`src/bin/real-run-fake-areka.rs:53`（`EXIT_TARGET`）・`close_tests.rs:8`・`launch_tests.rs:225-273`
- 何か: 呼び手が `RUST_LOG` を狭めたとき、`close()` が終わりの行を読めるよう `,areka::app_exit=info` を足す所。
- 問題: 偽の子と単体テストが同じ手書きの名前を使うので互いには合うが、本物の areka とは誰も照らさない。`session-module-gather` が `app_exit` を動かす予定（同 brief 48・64・100・110 行）。
- 起きる場面: `app_exit` が `areka::session::app_exit` へ動き、`RELIED_TARGETS` の直しが漏れると、`.rust_log("warn,...")` の実機の走りが全部 `UserClose` でなく `EndedBy::Unlogged` になる（`crates/areka/tests/emo2_mcp_real_run.rs:115`・247）。その試験は環境変数で明示したときだけ回るので、全体テストは緑のまま。
- 直し方の案（設計で選ぶ）: ⑴ 範囲の中の静的なテストで、areka のソースの終わりの行の出所が `RELIED_TARGETS` と合うことを照らす（範囲の外のファイルを読むだけ）。⑵ 範囲の外の常時の試験 `crates/areka/tests/smoke_boot_loop_exit.rs` に `.rust_log("warn")` の 1 回を足し `ended.by == SafetyExit` を確かめる（約 8 行・本物の areka を起こすが全体テストで既に回る段）。⑶ `crates/areka` の中に、終わりの行の出所の文字が道具の文字と同じことを確かめるテスト。
- 要件: 3.1・4.1・4.2・7.2
- 印: 〔持ち主〕`areka::app_exit` の名指しは `session-module-gather` の持ち物＝名前は書き換えない（照らすだけ）。⑵⑶ なら〔外のファイル〕`crates/areka/`（C22 の 5 本の触るファイルではない）。⑵ は本物の areka を起こすが既に常時回る段なので要件 8.1 の禁止に当たらない（8.2）。

**27. 閉じる求めの送り直しと受け取りの確かめが、たまたまの速さでしか試されない**
- 場所: `crates/real-run-kit/src/close.rs:176-202`（`close_or_kill`）
- 問題: 「最初の taskkill が窓より先に届く（送り直しが要る）」場面を強いるテストも、「受け取りの行が出たら送り直さない」を確かめるテストも無い。偽の子のテストは立ててすぐ閉じるので、送り直しの枝が走るかは窓ができる速さ次第。
- 起きる場面: 閉じる求めを 1 度だけ送って待つだけの形に直しても、偽の子の窓が先にできていれば全部緑。本物の areka の起動の早い時期では締め切りまで固まり、kill される。
- 直し方: 偽の子に `sleep:<ms>` の一歩を足し（約 5 行）、`fake("sleep:1500;window").spawn().close()` が `UserClose` になることを確かめる。できれば `os_close_request` の行を数えて受け取りの後に送り直さないことも。
- 要件: 3.1・4.1・6.3
- 印: 偽の子の一歩を足すのは足すだけ（偽の子の名前は外への約束）。確かめは偽の子で足りる。

**28. 子が生きたままの「運ぶ道の失敗」と「別のサーバーが答えた」の枝にテストが無い**
- 場所: `crates/real-run-kit/src/mcp.rs:198-215`（子が生きているときの猶予と `Unreachable`）・260-266（`Scan::NotFound { others }` が空でない）・266（つなぐ締め切り）
- 起きる場面: `let grace = (Instant::now()+exit_grace).min(end)` から `.min(end)` を外しても、生きている枝で `ChildExited` を返しても、テストは何も赤にならない。
- 直し方: `mcp_tests` の既にある `serve()` で 1 度呼んでつなぎを持ち、サーバーを落とし、`living_child()` でもう 1 度呼んで `Unreachable` を確かめる（約 15 行）。
- 要件: 3.1・4.1
- 印: 範囲の中だけ。

**29. `dropped_listener_is_unreachable` が毎回約 5 秒を無駄にする**
- 場所: `crates/real-run-kit/tests/fake_mcp_test.rs:92-102`・原因 `src/bin/real-run-fake-areka.rs:95-102`（`listen-drop` が `hang()` で終わる）
- 問題: 偽の子に窓が無いので、破棄の `stop_quietly` が 5 秒の締め切りまで 1 秒ごとに taskkill を送り（失敗する）、最後に kill する。テストの中身は数ミリ秒。
- 起きる場面: 全体テストのたびに約 5 秒と 5 回の taskkill を払い、力ずくの終わりの行を出す。
- 直し方: `listen-drop` を `hang()` でなく次の一歩へ進ませ、`fake("listen-drop;window")` と `close_ok(run)` にし、`.deadline(5s)` を外す（約 -2 行）。
- 要件: 3.1
- 印: 偽の子の一歩の意味が変わる（範囲の中だけで使う）。

**30. 見張りが木を何度も読み直し、語を 1 文字ずつ全部試す（遅いはず・測っていない）**
- 場所: `crates/log-capture-kit/tests/workspace_scan/mod.rs:340-366`（`scan_tokens`）と各見張りの読み直し
- 問題: 木は `.rs` 2,652 本・約 35 MB。`scan_tokens` は全部の文字で全部の語を試す（検体の見張りは 45 語・最適化 0 で組む）。`cargo test -p log-capture-kit` 1 回で、読み＋剥がし＋走査が約 10 回、読むだけが 5 回、列挙が約 15 回。
- 直し方: 語の輪の前に先頭の 1 文字で篩う（約 3 行）。できればテストの実行体ごとに木を 1 度だけ読む `OnceLock` の置き場を `mod.rs` に。
- 要件: 3.1
- 印: 外への約束は変わらない。効き目の確かめは時間を測る走り（重い操作ではないが、静かな机でない測りは揺れる）＝「遅い」が本当かを設計の前に 1 度測るかは議題。

**31. 掃除の並走の較正がスレッドの割り当て次第で赤になり、掃除のスレッドが漏れうる**
- 場所: `crates/sample-ghost-kit/src/devroot_sweep_tests.rs:357-369`（譲らない回り続ける輪）・393-398（`passes >= RACED_ROUNDS`）
- 問題: 全体テストの負荷の下では、回り続ける掃除のスレッドが、主のスレッドの 60 周の間に 60 回の割り当てを得られないことがある＝割り当てだけで赤。371-391 行の輪の確かめが panic すると `stop` が立たず、消えた名前空間の上でプロセスが終わるまで 1 コアを焼き続ける。
- 起きる場面: 8 crate が並んでテストする重い時に、まれに `掃除が周回より多く走ったこと: 41` で赤。
- 直し方: `passes > 0` にする（8 スレッドのテスト 503-597 が決定論の檻）。`stop` を破棄の守りで立て、panic でもスレッドを止める。
- 要件: 3.1・4.3
- 印: 範囲の中だけ。

**32. 道具が端末へ出す文が ASCII でない（`real-run-kit` の証拠の行・`sample-ghost-kit` の誤りの行）**
- 場所: `crates/real-run-kit/src/evidence.rs:114-118`・122（「落ちた回の証拠を残しました: …」）・`src/close.rs:109-115`・`crates/sample-ghost-kit/src/lib.rs:654`（`Io` の `what` が日本語）・`src/bin/nar-sample-path.rs:143`・`src/devroot.rs:696`（`report_cleanup`）
- 問題: 決まり「道具が端末へ出す文は ASCII だけ」に反する。`sample-ghost-kit` は取り寄せの 3 つの誤りだけ `ascii()`（`fetched.rs:117`）を通し、同じ実行体のほかの誤りは通さない＝同じ決まりが 2 通り。
- 起きる場面: `cargo test` の出力を UTF-8 でないコードページで受ける（PowerShell の `| Tee-Object` など）と、残した証拠のパスを知らせる唯一の行が化けて見つけにくい。`nar-sample-path emo2` で `.nar` が読めないと「検体の .nar の読み取り failed at …」が出て、`tools/package.ps1:473` がそのまま throw の文に入れる。
- 直し方: 端末へ出す行を英語の ASCII に（例 `real-run-kit: kept failed run: <path>`）。`what` の文を ASCII にするか、出すときに `ascii()` を通す。panic の文は家の形のまま。
- 要件: 3.1・6.4・6.5
- 印: 〔外〕出す行が変わる（読み手＝`crates/real-run-kit/tests/fake_evidence_test.rs:18` の `KEPT_HEAD`・`doc/testing.md:75`・163。`.ps1` でこの文を字面で読む所は 0〔`tools/` を探した・`sample-fetch.ps1:382` は `Show-Ascii` で逃がす〕＝直す前に `u{XXXX}` を解いた grep で確かめ直す〔記憶の決め〕）。テスト用のライブラリの標準エラーが「道具が端末へ出す文」に入るかは開発者の裁定（議題）。

**33. 見張りのファイルの間で、手助けの写しが並ぶ（`workspace_scan/mod.rs` に置くべきもの）**
- 場所: `raw_occurrences`＝`temp_path_guard_test.rs:628-643`・`with_default_guard_test.rs:375-389`・`sample_path_guard_test.rs:891-902`。「この `Cargo.toml` は X を開発用に引くか」＝`with_default_guard_test.rs:249-273` と `sample_path_guard_test.rs:660-669`。`manifest_fixture`＝`with_default:643-646` と `sample:720-723`。`Hit`・`scan_sources`・`unlisted`・`allowed_paths(_without)`・`render`＝`file_length_guard_test.rs:91-110`・`temp:217-277`・`with_default:163-233`。1 行の取り次ぎ `mentions_kit`・`production_kit_dependencies`＝`with_default:254-261`
- 問題: 同じ決まりを最大 3 か所で持つ。1 か所だけ直すと見張りの間で判定が食い違う。
- 直し方: `mod.rs` へ移す（道具箱と判定の分け方は保つ・Lean 約 -60 行）。
- 要件: 3.1・4.8・6.1
- 印: 外への約束は変わらない（`structure.md:441` は既に走査の部品を共有と書く）。`sample_path_guard_test.rs`（926 行）が減る。

**34. 見張りのテストの中に、別のテストの証明をなぞるだけのものが並び、それぞれが木を全部読む**
- 場所: `sample_path_guard_test.rs:383-400`（`removing_the_exclusion_turns_the_guard_red`・いちばん重い走査）・369-373（組み方からして偽になりえない数の比べ）・`the_guard_file_itself_is_not_a_hit`（`temp:617-626`・`sample:882-889`）と `…_because_the_tokens_are_never_spelled_out`（`with_default:354-365`）・`assert_eq!(found, listed)`（`file_length:241-247`・`temp:457-463`）・ファイルが在るかの輪（`file_length:141`・151-155〔35 MB を読んで在るかだけを見る〕・`temp:310`・320-324・`with_default:320`・330-333・455・462-465）
- 起きる場面: どれも、消しても失われる檻が無い（ほかのテストが同じ退行で赤になる）。消さずにいると全部の走査の回数が増える。
- 直し方: 消す（Lean 約 -95 行・走査の回数も減る）。モジュールの説明でテストの名前を挙げている所を直す。
- 要件: 3.1・3.3・4.7
- 印: 見張りの決まりは変わらない。完了した spec の文書（要件 10.2・12.4・12.5 の追跡）がテストの名前を挙げている＝`completed/` の文書は直さず、名前を挙げるのはこの spec の記録に残す（設計で決める）。各見張りに較正の対が 1 本以上残ることを確かめる（要件 4.7）。

**35. `real-run-kit` の外向きの品のうち、使い手の無いもの**
- 場所と中身:
  - `Staged::run_dir()`（`src/launch.rs:248-251`）＝どこからも呼ばれない（`.run_dir()` は全部 `Run::run_dir`）。消す（-4 行）。
  - `Run::stderr()` の `pub`（`src/run.rs:104-107`）＝`output()` だけが使う。外の呼び手 0・`doc/testing.md` に無い。私用に。
  - `pub use run::Limits`（`src/lib.rs:43`）＝外の使い手 0、`Limits` を受ける作り手も無い（外からは既定の値を読めるだけ）。`pub(crate)` に。
  - `Launch::Ghost { folder, balloon }`（`src/launch.rs:25-29`・308-312・`launch_tests.rs:164-176`）＝この crate のテストのほかに呼び手 0。消す（約 -20 行）。
  - `RealRun::mcp_port(n)`（`src/launch.rs:92-96`・縛るだけのテスト `launch_tests.rs:121-128`）＝外の呼び手 0。消す（約 -12 行）。指摘 10 を直すなら残す理由ができる。
  - 偽の子の一歩 `err:`（`src/bin/real-run-fake-areka.rs:75`・説明 6 行）＝どのテストも使わない。消す（-2 行）。
- 起きる場面: 使われない口が、直すたびに読まれ、テストされ、説明される（`doc/testing.md:37`・44 が `Launch::Ghost` と番号の指定を案内している）。
- 要件: 3.1・3.3・7.6
- 印: 〔外〕公開の品が減る（読み手＝`doc/testing.md:37`・44）。範囲の外の呼び手は全部 0（`crates/`・`tools/`・`doc/`・`.kiro/`・`.claude/` を探した）。〔残⑴〕`Launch::Ghost` は完了した spec `areka-P0-real-run-mcp-kit` の要件 1.1⑵（立て方の 5 通り）、`mcp_port` は同 8.1（「MCP の番号」）が求めた口＝消すのはその裁定を覆す。確かめはコンパイルだけ。

**36. `sample-ghost-kit` で、本体のフォルダのパスを組む式が何か所にも並ぶ**
- 場所: `src/lib.rs:253`（`from_copy`）・521（`manual_paths`）がどちらも `root.join(store).join(name)`・`companion_folder` 551-552・`declared_elements` 537・542
- 問題: 完了した `areka-P0-nar-install` の `tasks.md:382` が既に漂いの危うさとして挙げた。inner-balloon の spec でバルーンが動いたときのように 1 つだけ直ると、`nar-sample-path` の `folder=` の行と `SampleRoot::folder()` が食い違い、2 つを照らすテストは無い。
- 直し方: `fn body_folder(sample, root) -> PathBuf` を 1 つにして 3 か所で使う（Lean -3 行）。
- 要件: 3.1
- 印: 外への約束は変わらない。

**37. `sample-ghost-kit` のテストの写し場所 `nar_writer_tests::scratch` が `WorkDir` を迂回し、`target\` に残りを置く**
- 場所: `crates/sample-ghost-kit/src/nar_writer_tests.rs:71-86`（呼び手 264・299・304-305・312・317）
- 問題: 手で組む `<CARGO_MANIFEST_DIR>/../../target/nar-writer-work/<pid>-<札>`。後始末は通った道だけ（299・317 の `remove_dir_all`）。親の `target/nar-writer-work/` は誰も消さない。`CARGO_TARGET_DIR` も見ない（ワークツリーの `target\` の中には留まる）。75-76 行の説明が「`WorkDir` が入ったらそちらへ寄せてよい」と書き、`WorkDir` は今ある。
- 起きる場面: `folding_a_tree_converts_nothing` の確かめが落ちると `target/nar-writer-work/<pid>-fold/` が `cargo clean` まで残る。毎回空の `target/nar-writer-work/` が残る。
- 直し方: `scratch()` を消して `let root = WorkDir::new()?;`（破棄で消える・panic でも）。手の `remove_dir_all` を消す（Lean 約 -14 行）。
- 要件: 3.1・4.5
- 印: 範囲の中だけ。見張り（一時パス）がこの形を拾わない理由も設計で確かめる（拾うべきなら指摘 5 の続き）。

### 3.3 できれば（39 件）

**38. `expect_one`・`expect_field` に `#[track_caller]` が無い** — `crates/log-capture-kit/src/expect.rs:17`・24。失敗の場所が呼び手の行でなく `expect.rs:29`（`:19`）と出る。`crates/areka-emo-text/tests/attach_wiring_test.rs:551` と 595 はどちらも同じ `expect_one(_, attach_msg)`＝どちらが落ちたか分からない。直し方: 2 つに付ける（+2 行）。件 1（§5）で写しを消すと、失敗の場所が呼び手の側から道具の側へ移る＝この指摘で戻せる。要件 3.1・5.2。外への約束は変わらない（43 ファイルの失敗の場所がよくなる）。

**39. `event_tests.rs` の 3 本が、ほかのテストから必ず導ける** — `format_line_is_byte_identical_to_current_formatting_code`（202-227）・`level_is_rendered_with_display_not_debug`（229-239）・`capture_lines_does_not_leak_the_sentinel`（274-285）。どれも、`event.rs`・`capture.rs` のどの変更でもそれだけが赤になることが無い。直し方: 消す（Lean -49 行）。あわせて `capture_tests.rs:385-388` も 389-392 に含まれる。要件 3.1・3.3。〔持ち主〕に当たる行（`event_tests.rs:18-19`・121）は触らない。

**40. テスト用の購読者が `CapturedEvent::from_event` を書き写している** — `crates/log-capture-kit/src/capture_tests.rs:113-124`（`event.rs:66-74` の写し）。この写しのためだけに `FieldCollector` とその中身が `pub(crate)`（`event.rs:40`）。`from_event` が変わる（拾う属性を足す等）と、このテストの購読者だけが違う形を作る。直し方: `.push(CapturedEvent::from_event(event))`・`FieldCollector` を私用に・取り込みを減らす（Lean -9 行）。要件 3.1。範囲の外の使い手 0。

**41. 赤になりえない自己テスト** — `crates/log-capture-kit/src/probe_tests.rs:48-65`（自分の説明が「決めが動かない」だけを確かめると書く＝67-83 が厳しく含む・-18）・`probe_tests.rs:16-23`（文字の定数が `"AREKA_"` で始まるか・-8）・`capture_tests.rs:232`・241・249（合流した 8 スレッドが panic しなければ必ず 8 になる `DONE` の数・-4）。直し方: 消す（Lean 約 -30 行）。要件 3.1・3.3。

**42. `interest_probes_enabled` が `pub` だが範囲の外の呼び手が 0** — `crates/log-capture-kit/src/probe.rs:189-196`・`lib.rs:232` の再輸出・`lib.rs:187-192` の説明。使い手は `probe_tests.rs:32-44` だけ（ほかの当たりは `completed/areka-P0-test-cage-determinism/` の文書）。直し方: `pub(crate)` にし、再輸出と説明の行を消す（Lean 約 -8 行）。要件 3.1・3.3・7.6。〔外〕公開の関数が 1 つ減る（呼び手 0）。

**43. 説明が「古い整形のコードが今も生きている」と書く** — `crates/log-capture-kit/src/event_tests.rs:3-12`・14-20・102・121・`event.rs:136`（「今日出している」）・89-90。挙げた 7 ファイルは今どれも `log_capture_kit::capture_lines` を呼ぶ＝`LegacyLineFormatting` は今は無いコードの凍った写し。読み手が「今のコードとの突き合わせ」を生きた檻と信じる。直し方: 出自の説明として言い直す（`LegacyLineFormatting` は残す＝試料と `format_line` を一緒に直すと赤になる役は生きている）。要件 3.1。〔持ち主〕`event.rs:150`・`event_tests.rs:18-19`・121 の `input_events/` のパスの文は `input-crate-split` の持ち物＝その行は書き換えない（言い直しはその行を避けて行うか、`input-crate-split` へ申し送る＝議題）。

**44. 手で書いた数が説明に残る（件 2 を含む）** — `crates/log-capture-kit/src/lib.rs:28`「消費する 21 crate」（今は正しい・§5）・`filter_tests.rs:3`・5・191「wintf の 96 呼出」（今は `capture_under_filter(` が wintf に 169 か所＝既に古い）。範囲の外の `crates/wintf/src/ecs/test_support.rs:5`「96 箇所」も同じ数。起きる場面: `input-crate-split`（C23）が `crates/areka-input/` を足すと消費は 22 になり、28 行の文は偽になる。数を照らす見張りは無い。直し方: 数を書かない文にする（行数 ±0）。要件 3.1・5.3。外への約束は変わらない。`wintf/src/ecs/test_support.rs:5` は範囲の外＝直すかは要件 7.2（直しに要る変更か）で議題。

**45. 子を起こした後で `Run::new` が panic すると、子が置き去りになる** — `crates/real-run-kit/src/launch.rs:182-185`（起こしてから `Run::new`）・`run.rs:61-72`・`mcp.rs:88-92`（tokio の実行器を組む所の panic）。実行器を組めない（OS の資源切れ）と、`Child` が kill されずに落ち、areka は 45 秒の安全な終わりまで残って、残した回の置き場のファイルを掴む。直し方: `McpChannel::new(port)` を `cmd.spawn()` の前に組んで `Run::new` へ渡す（約 2 行の移動）。要件 3.1・6.3。範囲の中だけ（引き金はまれ）。

**46. `Waits` が `Limits` を項目ごとに写しただけ** — `crates/real-run-kit/src/mcp.rs:65-76`・`run.rs:208-215`（`waits()` の変換）・使い手 `run.rs:139-161`・`mcp_tests.rs:23-30`。直し方: `&Limits`（`Copy`）をそのまま渡す（Lean 約 -18 行）。要件 3.1。

**47. 3 つの環境変数を消してから 2 行後に設定し直す** — `crates/real-run-kit/src/launch.rs:199-206`。消す要があるのは `AREKA_USER_ROOT` だけ。直し方: `cmd.env_remove("AREKA_USER_ROOT")` のみ（Lean -4 行）。要件 3.1。`launch_tests` の確かめはそのまま通る。

**48. 同じ文字を 2 つの定数が持つ** — `crates/real-run-kit/src/close.rs:15`（`ACK[1] = "event=\"app_exit\""`）と 18（`EXIT_LINE`）。直し方: `const ACK: [&str; 2] = ["event=\"os_close_request\"", EXIT_LINE];`。要件 3.1。これは記録の項目 `event` の値で、出所の名前 `areka::app_exit`（`session-module-gather` の持ち物）ではない。

**49. `real-run-kit` のテストの手助けの写し** — `living_child()`（`src/mcp_tests.rs:13-20`・`src/output_tests.rs:25-32`）・`KillOnDrop`（`mcp_tests.rs:210-217`）があるのに 6 本が `child.kill(); child.wait();` を手で書く（試すコードが panic すると ping の子が最大 60 秒残る）・`fake()`（`tests/fake_evidence_test.rs:20-24`・`tests/fake_mcp_test.rs:13-17`・`tests/fake_wait_close_test.rs:16-20`）・`alive()`（`fake_evidence_test.rs:37-43`・`fake_wait_close_test.rs:23-29`）・panic の文の取り出し（`src/i686_tests.rs:83-89`・`fake_wait_close_test.rs:38-42`）。直し方: `cfg(test)` の `src/test_support.rs` と `tests/common/mod.rs` にまとめる（Lean 約 -25 行）。要件 3.1。

**50. 定数や代入をなぞるだけのテスト（`real-run-kit`）** — `src/launch_tests.rs:121-128`（作り手が引数をしまうか・実際の効き目は `crates/areka/tests/smoke_boot_loop_exit.rs:291` が覆う）・217-220（`free_port` が 0 でない＝0 番で bind して 0 は返らない）・`src/i686_tests.rs:245-250`（3 つの定数の文字をなぞる）。直し方: 消す（Lean 約 -17 行）。要件 3.1・3.3。あわせて `tests/fake_wait_close_test.rs:208` は「2 回続けて選んだ空きの番号が違う」を確かめるが、OS は保証しない＝外すか弱める（揺れの種）。

**51. `is_i686_pe` が 6 バイトを見るために実行体を丸ごと読む** — `crates/real-run-kit/src/i686.rs:27-36`（`std::fs::read`）。`spawn` 1 回で最大 4 回（隣・環境変数・debug・release）。直し方: 先頭の 0x400 バイトだけ読む（約 +2 行）。要件 3.1。指摘 11 と同じ所＝一緒に直す。

**52. 取り寄せの `.nar` を、確かめた読みと使う読みで 2 度読む** — `crates/sample-ghost-kit/src/fetched.rs:89-103`（読んで SHA）・`src/devroot.rs:367`（印のためにもう 1 度）・400（`NarArchive::open` で 3 度目）。2 度の読みの間に `sample-fetch` が置き換えると、確かめていないバイトで親の写しを作る（書きかけは終わりの記録が無いので断られる）。直し方: `verified_nar_in` が読んだバイトを返し、そのまま渡す（取得ごとの読みと SHA が 1 回減る）。要件 3.1。

**53. 同じ検体を違うバイトで持つ 2 つのプロセスが、互いの親の写しを退ける** — `crates/sample-ghost-kit/src/devroot.rs:432-463`。テストの最中に `sample-fetch` が `pasta-in-windows.nar` を更新するか `git checkout` が `.nar` を変えると、プロセス B が A の写しの途中で A の親を退け、A の `copy_tree` が NotFound で落ちる。直し方の案: いちばん新しい印だけを残す形にする（読み手の報告は `vendors/sample_ghost/README.md` に「テスト中に検体を変えない」と書く案も挙げたが、`vendors/` は触らない＝要件 7.10）。要件 3.1。〔重〕なし。直し方の形は設計で選ぶ（直す価値の見立ても議題に載せうる）。

**54. 古い世代の写しのフォルダが消えない** — `crates/sample-ghost-kit/src/devroot.rs:101-106`・372。名前を `cache-2` へ上げたときの `cache\` は `cargo clean` まで残り、上げるたびに増える。モジュールの説明（20-27 行）はまだ `cache/<検体>-…` と描く。直し方: 写しの道で `CACHE` でない兄弟の `cache*` を消す（`discard_tree` と同じく先に `work/gc-…` へ退ける）・説明を直す。要件 3.1。外の読み手で `cache\` を名指しする所は 0（`fetched_tests.rs:299` は定数を使う）。

**55. 取り寄せの棚のパスの関数が 3 つに割れている** — `crates/sample-ghost-kit/src/lib.rs:439-468`（`fetched_archive_path`・`fetched_checksum_path`・`on_shelf`）・`src/bin/nar-sample-path.rs:109-116`（`unreachable!` の腕・行ごとに実行体と環境変数の探しを 2 度）。直し方: `pub fn fetched_shelf(sample) -> Result<Option<(PathBuf, PathBuf)>, SampleError>` 1 つに（Lean 約 -12 行）。要件 3.1・7.6。〔外〕公開の関数の名前が変わる（呼び手はこの crate の実行体だけ・`crates/`・`tools/`・`doc/`・`.kiro/`・`.claude/` に当たり 0）。出す行は同じ。

**56. `fold-samples` が 1 つのフォルダのために列を持ち、`--from` を確かめる前に作業の置き場を消す** — `crates/sample-ghost-kit/examples/fold-samples.rs:69-81`（消えた「全部を畳む」形の名残の `Vec` と輪）・62-66（`--from` が無い呼び出しでも `target\fold-samples` を消す）。直し方: `--from` の確かめを先にし、1 つを直に畳む（Lean 約 -8 行）。要件 3.1。README 203 行の「`--from` が無ければ終了 1」は保たれる。

**57. 検体の名前を 1 回の呼び出しで 2 度引く** — `crates/sample-ghost-kit/src/lib.rs:238` と `src/devroot.rs:301`・`lib.rs:514` と `devroot.rs:595`。直し方: `&'static Sample` を渡す（Lean 約 -2 行・全部 `pub(crate)`）。要件 3.1。

**58. `WorkDir::lock_path` が `pub` だが、この crate のテストだけが使う** — `crates/sample-ghost-kit/src/devroot.rs:228-236`。説明が「使われない警告を避けるため公開のまま」と書くが、`#[cfg(test)] pub(crate)` でも避けられる。範囲の外（areka-nar・areka-update・areka-seriko・areka）は `new()` と `path()` だけ。直し方: テストの時だけの私用に（Lean -4 行の説明）。要件 3.1・7.6。〔外〕公開の品が 1 つ減る（呼び手 0）。〔残⑵〕「本番の呼び手が現れるまで公開のまま」はわざと置いた先送りの注記に当たりうる（`shortcut:` の形ではないが理由が書いてある）。

**59. `owner_is_gone` が `removal.is_ok()` を長い説明つきの関数で包む** — `crates/sample-ghost-kit/src/devroot.rs:516-518`（直に呼ぶのは `devroot_sweep_tests.rs:78` だけ）。わざと決めを縛るための口＝境目の指摘。直し方: 包みを外して説明をコメントに残す（Lean 約 -6 行）。要件 3.1。〔残⑵〕決めを固定するための口と書いてある＝残すかを議題にしうる。

**60. `every_fixture_the_testing_strategy_lists_can_be_built` がほとんど何も確かめない** — `crates/sample-ghost-kit/src/nar_writer_tests.rs:320-504`。24 の 1 項目の作り手がそれぞれ `parse(..).len() == 1` を確かめ、壊しの輪は `!bytes.is_empty()`（どの書庫も局所ヘッダを持つので落ちえない）。使い手（areka-nar の `container_tests` ほか・areka の `install/procedure_branch_tests`）と 224-248 行が既に覆う。直し方: 消す（Lean -185 行）。要件 3.1・3.3。323 行の説明が「書き手が形を表せなくなったときだけ赤」と書く＝わざとの置き方の記録だが、`shortcut:` の形ではない。

**61. `registry_records_kind_and_bundled_balloons` が定数の表を言い直す** — `crates/sample-ghost-kit/src/lib_tests.rs:434-454`（11 行のうち 6 行だけ）。本物の `.nar` と照らす往復（502-556）が大事な所を覆う。直し方: 消す（Lean -21 行）。要件 3.1・3.3。

**62. 検体の登記を手で 4 か所に固定している** — `crates/sample-ghost-kit/src/lib_tests.rs:143-158`（10 の名前）・210（`== 11`）・267-279（11 の名前）・503-507（`== 11`）・テストの名前 `all_eleven`（266）。検体を足すと 4〜5 か所を直す（「1 行で検体を足す」の決まり `lib.rs:131` に反する）。直し方: 143-166 の 1 つの列だけを残し、285 行の `known` を `SAMPLES` から作り、`== 11` を `!SAMPLES.is_empty()` に、266 行の名前を改める（Lean 約 -16 行）。要件 3.1。

**63. 重なったテスト（`sample-ghost-kit`）** — `src/devroot_sweep_tests.rs:273-293`（`lib_tests.rs:209-262` と `devroot_tests.rs:72-100` が覆う・-21）・`src/devroot_fetched_tests.rs:81-87`（`lib_tests.rs:264-290` が覆う・-7）・`src/fetched_tests.rs:60-68`（306-336 と重なる・約 -9）。直し方: 消す。要件 3.1・3.3。

**64. `fetched_shelf_is_outside_work_and_cache` がほぼ恒真** — `crates/sample-ghost-kit/src/fetched_tests.rs:295-302`。文字の定数 `"fetched"` が `"work"`・`"cache-2"`・`"manual"` でないことしか見ない。掃除が名前空間の根を走るよう変わっても緑（`devroot_fetched_tests.rs:98-103` が偶然拾う）。直し方: 消す（-8 行）か、`devroot_fetched_tests` で `fresh_root_in` の後に棚のファイルが無事なことを確かめる。要件 3.1。

**65. `check_pasta_layout` の後半は較正されず、`check_registry` と重なる** — `crates/sample-ghost-kit/tests/pasta_inner_balloon_test.rs:37-60`。較正の場面（72-122）は最初の `read_to_string` で落ちるので、「根の `balloon/` が空」の輪はどのテストでも赤にならない。`acquire` は既に `check_registry`（`lib.rs:485-496`・562-577）で根の `balloon/` を断る。直し方: 37-60 行を消し、4-5 行の説明を直す（Lean 約 -25 行）。要件 3.1・3.3。

**66. `sample-ghost-kit` のテストの説明に古い数と主張** — `src/devroot_fetched_tests.rs:4`（URL の行はまだ無い→ある・`lib.rs:184-191`）・67（「今の 9 本」→ 10）・`src/lib_tests.rs:7`（「主張は 9 本」→ 13）・`src/devroot_cache_tests.rs:27`（`RACE_SAMPLE` がいちばん小さい→ `areka-balloon.nar` 28,014 バイトが wplimit 33,906 より小さい）。速さのために検体を選ぶ人を誤らせる。直し方: 直すか数を外す。要件 3.1。

**67. 試料の `.nar` が要らないファイルの往復をする** — `crates/sample-ghost-kit/src/fetched_tests.rs:12-31`（`tiny_nar` が `WorkDir` を取って書いて読み戻す）・`src/devroot_fetched_tests.rs:36-38`（バイトを得るために私用の名前空間を丸ごと取る）。直し方: `NarBuilder::...bytes()` を直に（Lean 約 -6 行）。要件 3.1。

**68. 要るより遅いテスト（`sample-ghost-kit`）** — `src/lib_tests.rs:298-330` が `balloon()` の誤り（ディスクに触らない）を試すために `emo2`（4.5 MB を丸ごと写す）を取る＝`dot_sakura`（1.4 MB）か `from_copy` の探りで足りる。`lib_tests.rs:562-651` が往復（502-556）の直後に emily4（2.7 MB）と dot_sakura を写しなしでもう 1 度展開する＝飛ばした項と警告の確かめを往復へ寄せられる（往復そのものは、古いかもしれない `cache-2` でなく新しいインストールを使う唯一の確かめなので残す）。要件 3.1。

**69. 小さなテストの抜け（どれも黙らず大きく落ちる側）** — `src/devroot.rs:128`（`CARGO_TARGET_DIR=""` や空白で祖先の探しへ戻る・3 行のテスト）・`src/fetched.rs:62`（NotFound でない読みの誤り＝検査の値のパスがフォルダ→ `Io`・終了 2）・`src/fetched.rs:83`（UTF-8 でない検査の値のファイル＝Windows PowerShell の `>` で書いた UTF-16→ `FetchedChecksumInvalid`）・`src/bin/nar-sample-path.rs:71`（引数なし→終了 2）。`bin/nar-sample-path.rs:114`・142-145（target の無い `--fetch-list`・標準出力の書き込みの失敗）は `target\` の外で回さないと試せない＝今のまま（読み手の見立て）。要件 3.1・4.1。

**70. 弱い確かめ（`sample-ghost-kit`）** — `src/devroot_tests.rs:43-55`（`WorkDir::new` と同じ関数 `namespace_dir()` で比べる・名前の定数と比べる＝恒真）・`src/devroot_cache_tests.rs:177-181`（作ったばかりの私用の名前空間の写しが空＝常に真・後の `== vec![leaf(&master)]` が空振りを既に防ぐ・-5）・`src/fetched_tests.rs:72`（定数 `CHECKSUM_SUFFIX` を確かめる・-1）。直し方: 消す。要件 3.1・3.3。

**71. `with_default` の見張りの 2 つの例外表に重複の確かめが無い** — `crates/log-capture-kit/tests/with_default_guard_test.rs:101-156`。ほかの見張り（`file_length:158-169`・`temp:327-338`・`sample:779-784`）は持つ。同じパスを 2 度書いて数を 7 に上げると通り、表が実際より多くを名乗る。直し方: 重複の確かめを足す（+4 行）。要件 3.1・4.7。

**72. TOML のコメントを剥がす処理が引用符を取り違える** — `crates/log-capture-kit/tests/workspace_scan/mod.rs:390-400`（`strip_toml_comment`）。`"…"` の中の `'` や `\"` でも「文字列の中」を切り替える。`demo = { git = "https://x/it's#v", package = "log-capture-kit" }` は `#` で切られ、依存が消えて ⑶ が見逃す（作り事の場面）。直し方: 開いた引用符の種類を覚える。要件 3.1・4.7。

**73. 見張りの説明と失敗の文の古い数と指し先** — `workspace_scan/mod.rs:334-336`（「本番の語は重ならないので規則 2 は遊んでいる」→ 偽：`env::temp_dir(` は `temp_dir(` を含み規則 2 に頼る＝生きた規則を消させる誘い）・`structure.md:176` の引用（`file_length:3`・122・`mod.rs:44`・`workspace_scan_test:257`。決まりは今 180 行）・`mod.rs:17-19` と `workspace_scan_test.rs:91-92`（`wintf/src/ecs/types.rs` の `test_offset_default` は今は無い）・`mod.rs:9`（ファイルに触る関数は 3 つ→ `workspace_manifests`・`collect_manifests` も）・行の指し先（`temp:77` が `mod.rs:318-332`→今 325-366・`with_default:21` が `mod.rs:21-23`→今 28-30）・古い数（`with_default:100` の 5→6・`temp:453` の失敗の文の 16 ファイル／20 か所→1・`roadmap.md:60` の file_length の表 5→4・`structure.md:441` の「4 つの見張りが走査を共有」→ 文書の木を入れて 5）。直し方: 直すか数を外す（失敗の文は赤になったときに出る）。要件 3.1。〔外〕は無いが `roadmap.md`・`structure.md` は steering の自分の行の直し（C22 で許す重なり）。

**74. 見張りの失敗の文の中で、行の継ぎと `\n` が化けている** — `crates/log-capture-kit/tests/with_default_guard_test.rs:413-419`（行末の `\` と `\n` が 9 個の空白の並びと生の改行に化け、失敗の文に空白の並びが出る）。`workspace_scan_test.rs:427-428`・440-441・449-450・`sample_path_guard_test.rs:528-529` も同じ形だが、rustc が CRLF をならすので害は無い。書き込みの道具が逃がしを化かす既知の件（記憶の決め）と同じ。直し方: 逃がしを戻し、直した後にバイトで確かめる。要件 3.1。

**75. `TempPath` の 2 つの入口 `new` と `under_target` が全く同じ** — `crates/temp-path-kit/src/lib.rs:117-132`。説明が「呼び出しを書き換えずに済ませるために 2 つの入口を残している」と書く。同じ物に 2 つの名前があり、どちらを使うかの迷いを毎回生む。直し方: 1 つに寄せて呼び手を全部書き換える（`new` 244 か所・`under_target` 390 か所・97 ファイル）、または今のまま残す。要件 3.1・7.3・7.6。〔外〕公開の関数が 1 つ減る。〔外のファイル〕〔C22〕呼び手の多くが `crates/areka-runtime/src/`・`crates/areka-placement/src/` ほか C22 のほかの 5 本の触るファイルに当たる＝C22 の中では書き換えられない（要件 7.3）。〔残⑴〕〔残⑵〕説明に理由を書いた、わざと残した口＝完了した `areka-P0-test-cage-determinism` の決めかを要件の討議で確かめる。

**76. `temp-path-kit` のテストの説明の数が古い** — `crates/temp-path-kit/src/lib_tests.rs:3`「主張は 3 本」（今は 8 本）。直し方: 数を外す。要件 3.1。

### 3.4 数のまとめ

| 組 | 件数 | 番号 |
|---|---|---|
| 必ず直す | 5 | 1〜5 |
| 直したほうがよい | 32 | 6〜37 |
| できれば | 39 | 38〜76 |
| 計 | 76 | |

- crate ごと: real-run-kit 21（3・9〜12・26〜29・32 の半分・35・45〜51）・log-capture-kit 25（1・4・5・17〜21・30・33・34・38〜44・71〜74）・sample-ghost-kit 27（2・7・8・14〜16・22〜25・31・32 の半分・36・37・52〜70）・temp-path-kit 4（6・13・75・76）。
- 1,000 行の番人: どの直しも 1,000 行を越えさせない見込み（いちばん近い `sample_path_guard_test.rs` 926 行は、指摘 1 で約 +6・指摘 33・34 で減る）。
- Lean: 消せる見込み約 -700 行（大きいのは 60 の -185・34 の -95・33 の -60）。依存は -0（自作の SHA-256 は裁定で残す）。

## 4. 印ごとの抜き出し

### 4.1 外への約束が変わる・範囲の外に触る（要件 1.6）

| 番号 | 変わるもの | 範囲の外の読み手・ファイル | C22 の 5 本に当たるか |
|---|---|---|---|
| 1 | 見張り ⑵⑷ が厳しくなる | `structure.md:441`・書き換え＝`crates/areka/tests/window_link_real_run.rs:448-449`・`crates/areka-nar/src/error_tests.rs:205` | 当たらない（`window_link_real_run.rs` は C23 の `input-crate-split` が 49 行を触る予定） |
| 3 | （新しい変種を足すときだけ）`McpError` | `crates/areka/tests/emo2_real_run/common.rs`・`common_tests.rs`・`emo2_mcp_real_run.rs:124`・`doc/testing.md:58-64` | 当たらない |
| 4 | 見張り ⑷（`env-filter`）が厳しくなる | `structure.md:441`・`logging.md:128` | 当たらない（steering は自分の行） |
| 5 | 一時パスの見張りが厳しくなる | `structure.md:441` | 当たらない |
| 9 | 閉じ忘れ＋子が固まる回が赤になる | `close()` を呼ばない範囲の外の実機の試験（子が閉じれば影響なし） | 当たらない |
| 10 | `AREKA_MCP_PORT` の上書きの振る舞い | 呼び手 0 | — |
| 11 | `ensure` の振る舞い | `crates/areka/tests/smoke_boot_loop_exit.rs:238`（影響なし）・`doc/testing.md:93` | 当たらない |
| 15 | 相対の `CARGO_TARGET_DIR` の解き方 | `vendors/sample_ghost/README.md:154`・169（触らない）・`tools/sample-fetch.ps1:372-389` | 当たらない |
| 18 | 1,000 行の番人の例外表の扱い | `roadmap.md:60` | この spec だけが触る（約束 ⑵） |
| 26 | （案 ⑵⑶ のとき）範囲の外にテストを足す | `crates/areka/tests/smoke_boot_loop_exit.rs` または `crates/areka/` の中 | 当たらない |
| 32 | 端末へ出す行 | `doc/testing.md:75`・163（`fake_evidence_test.rs:18` は範囲の中） | 当たらない |
| 35 | 公開の品が減る | `doc/testing.md:37`・44（呼び手 0） | 当たらない |
| 42 | 公開の関数が 1 つ減る | 呼び手 0 | — |
| 44 | （直すなら）範囲の外の数の文 | `crates/wintf/src/ecs/test_support.rs:5` | 当たらない |
| 55 | 公開の関数の名前 | 呼び手 0 | — |
| 58 | 公開の品が 1 つ減る | 呼び手 0 | — |
| 75 | 公開の関数が 1 つ減る | 97 ファイル 634 か所 | **当たる**（`areka-runtime`・`areka-placement` ほか） |

ほかの指摘（2・6〜8・12〜14・16・17・19〜25・27〜31・33・34・36〜41・43・45〜54・56・57・59〜74・76）は、外への約束を変えず、範囲の外のファイルを書き換えない。

### 4.2 確かめに重い操作が要るもの（要件 1.7・8.3）

- **1**: `window_link_real_run.rs` の書き換えの効き目は環境変数で明示したときだけ回る実機の試験でしか確かめられない（コンパイルと見張りは常時のテスト）。
- **2・8**: `fold-samples` の通しは `git ls-files` の追跡された展開の木が要る＝`vendors/` の書き替え。写しの式と書き先を引数で受けて単体テストで筋まで縛り、通しの効き目は次に本当に検体を畳むときに確かめる形が要る。
- **26 案 ⑵**: 本物の areka を起こすが、既に常時回る `smoke_boot_loop_exit` の段（要件 8.2）。
- **30**: 効き目は時間を測る走りで確かめる（要件 8.1 の操作ではないが、測りは揺れる）。
- ほかは全部、常時のテストか偽の子 `real-run-fake-areka` で確かめられる。

### 4.3 直さずに残してよい 3 種に当たりうるもの（要件 2.2）

- **⑴ 開発者の裁定に反する直し**: **11**（完了した `areka-P0-real-run-mcp-kit` 要件 2.1・2.5「隣が 32bit ならそのまま」）・**35** のうち `Launch::Ghost`（同 要件 1.1⑵）と `mcp_port`（同 要件 8.1）・**75**（説明に理由を書いた 2 つの入口＝完了した `areka-P0-test-cage-determinism` の決めかを確かめる）。指摘 5 の「環境変数の読みまで広げる」案は、見張る決まりを増やす＝新しい機能（要件 3.4）に当たりうる。
- **⑵ わざと置いた先送りの印**: `shortcut:`・`ponytail:` の注釈は範囲の中に 0。`#[allow(dead_code)]` は `log-capture-kit/tests/workspace_scan/mod.rs:40`（理由 38-39 行）と `#[expect(dead_code, reason=…)]` の `sample_path_guard_test.rs:193-197`（`Presence::ArrivesIn`）＝どちらも理由つきで、指摘にしていない。形は違うが理由を書いた先送りとして読めるもの＝**58**（`lock_path` を本番の呼び手が現れるまで公開）・**59**（決めを固定するための `owner_is_gone`）・**75**。
- **⑶ SSP の互換・ukadoc の正典に関わる振る舞いの変化**: **0 件**（4 crate はどれもテスト専用で、areka の外から見た振る舞いを持たない）。指摘 2 は `areka-nar` が既に従う同梱バルーンの置き場（ukadoc の install.txt の正典に沿う）に見本の道具を合わせる直しで、互換の振る舞いは変えない。

### 4.4 持ち主の決まりに関わるもの

- `input-crate-split` の持ち物（`event.rs:146-150`・`event_tests.rs:18-19`・121 の `input_events/` のパスの文）: **39**・**43** が同じファイルに触る。どちらもその行を書き換えずに直せる（43 は言い直しがその行に及ぶなら申し送り＝議題）。
- `session-module-gather` の持ち物（`real-run-kit` の `launch.rs`・`launch_tests.rs`・`close_tests.rs`・`bin/real-run-fake-areka.rs` の `areka::app_exit` の名指し）: **26** は名前を書き換えず照らすだけ。**48** は記録の項目の値 `app_exit` で出所の名前ではない。ほかの `real-run-kit` の指摘も名指しを書き換えない。
- `file_length_guard_test.rs` の上限越えの表と `OVER_LIMIT_ALLOWED_COUNT`: **18** がこの表の扱いを直す（C22 でこの spec だけが触る）。表に載る 4 本（`frame_transition_branch_tests.rs` 1,271・`window_move.rs` 1,234・`transition_judge_verdict_tests.rs` 1,024・`pilot-clickthrough-alpha-toggle/main.rs` 1,006）は割らない。表の 4 項はどれも今も上限越えで、ほかに上限越えのファイルは無い（1,000 ちょうどの `drag_follow.rs` と 999・998 のファイルは境目の手前）。

## 5. 預かった 2 件の事実（要件 1.8）

### 件 1: 記録の捕まえの手助けの写し

- どちらのクレートも `log-capture-kit` を開発用の依存に既に持つ（`crates/areka-emo-present/Cargo.toml:32`・`crates/areka-emo-text/Cargo.toml:71`）。どちらも C22 のほかの 5 本の触るファイルではない（`areka-emo-present` で C22 ① が触るのは `presenter_test_support.rs`・`presenter/hit.rs`）。
- **`crates/areka-emo-present/src/scale_tests.rs:251-263`**: 写しは `expect_one` だけ（`ExpectField` は写していない）。形・戻り値 `&'a CapturedEvent`・当て方（`message().contains(needle)`）・ちょうど 1 件の `assert_eq!(hits.len(), 1, …)`・panic の文「`{needle}` を含むログがちょうど 1 件ではない: {events:?}」は、道具の `expect.rs:24-35` とバイトまで同じ。呼び手は 276・295・308・337・356 行。直し方＝251-263 を消し、249 行で `expect_one` を取り込む（-13 行）。変わるのは失敗の場所（`scale_tests.rs:257` → `expect.rs:29`）だけ＝指摘 38 で戻せる。
- **`crates/areka-emo-text/tests/attach_wiring_test.rs:133-166`**: `type LogEvent = CapturedEvent` の別名・私用の `trait ExpectField` とその実装・`fn expect_one`。`expect_one` は別名のほかは道具と同じ。`expect_field` も同じ `self.field(name)` の引き方（Debug の形・同じ名前は後勝ち）で、欠けた項目で panic する。**ただ 1 つの違いは panic の文の中身**: 写しは `self.fields` を `{:?}` で出す（並びは訪ねた順・同じ名前も全部・`str_raw` 付き）、道具は `self.fields_map()`（名前で並べた `BTreeMap`・後勝ち・`str_raw` 無し）を出す＝道具の文のほうが情報が少ない。主張の強さ（ちょうど 1 件・欠けた項目は失敗・何を探したかが出る）は同じ。呼び手は `expect_one` が 551・574・595 行、`expect_field` が 558・580・582・587・597 行。直し方＝133-166 を消し、33 行を `use log_capture_kit::{ExpectField, capture, expect_one};` に（`CapturedEvent` と `LogEvent` は使われなくなる・-34 行）。
- 要件 5.2 の「失敗の文に何を探したかが出る」は保たれる。欠けた項目のときに出る周りの項目の詳しさが減る点は、要件の討議で「弱めた」に当たるかを確かめる（推す＝当たらない＝何を探したかの名前は両方とも出る）。道具の側の失敗の道のテストは今は 0＝指摘 21 で足す。

### 件 2: 消費する crate の数の文

- 今の文（`crates/log-capture-kit/src/lib.rs:27-29`）: 「消費する 21 crate はいずれも既に `[dependencies]` に `tracing` を持っているので追加の作業は無いが、新しい crate から引くときは 2 行になる。」
- 実際の数: `log-capture-kit` を名指す `Cargo.toml` は 22（道具の自分のを除いて **21**）。21 本とも `[dev-dependencies]` に置き、`[dependencies]` に `tracing` を持つ＝文は今は正しい。21＝areka・areka-actor・areka-emo-atlas・areka-emo-compose・areka-emo-present・areka-emo-text・areka-ghost・areka-kanade・areka-mcp・areka-mcp-bridge・areka-nar・areka-parsers・areka-placement・areka-runtime・areka-sakura・areka-seriko・areka-sylphya・areka-update・areka-user-data・shiori-host32-host・wintf（wintf だけ `features = ["env-filter"]`）。
- 数を照らす見張りは無い。C23 の `input-crate-split` が `crates/areka-input/` を足すと 22 になり文は偽になる（brief の仮置きどおり、数を書かない文にすれば閉じる）。同じ形の古びが同じ crate に既にある（指摘 44 の「wintf の 96 呼出」→ 今 169）。

## 6. 指摘にしなかった決め（記録）

- 自作の SHA-256（`crates/sample-ghost-kit/src/sha256.rs:1-4`「裁定 2」・依存を足さない）。`sha2 0.11.1` は `rmcp` 経由で既に `Cargo.lock:1999` にあるが、使えばこの crate の直の依存が増える＝C22 の約束 ⑸ と要件 6.7 に反する。
- 大域の受け皿が終わりまで育つ（`log-capture-kit/src/global.rs:43`・`lib.rs:123`・使い手 5 か所は開始の位置から切る・負荷は低い）。
- `capture_under_filter` は対照の記録を部分の一致で外す（`filter.rs:121-140`・`lib.rs:174-176` に書いてある）。
- `env-filter` を `wintf` だけに許すので、`areka-placement` に `capture_under_filter` の近い写しが 3 つある（例外表に載る・`with_default_guard_test.rs:101` 以降）。
- `CaptureWindowMarker` の 2 つの定義（`capture.rs:56`・`areka-actor/src/spawn.rs:46`）の漂いは `areka-actor/src/spawn_log_capture_tests.rs:252` が捕まえる。
- `real-run-kit` の空きの番号の選び方の競り（`launch.rs:348-354`・子の記録に出て `ListenFailed` になる）・回の置き場の 16 回（指摘 6 で消える）・失敗の文の末尾 64 KiB（`output.rs:13-14`）。どれも今の負荷で限界を越えていない。
- `nar-sample-path` を 2 つの端末から同時に呼ぶ競り（`bin/nar-sample-path.rs:52-54`・`vendors/sample_ghost/README.md:141` に書いてある）。`fold-samples` の `strip_folder` が先頭の 1 段しか比べない（`roadmap.md:551` に先送りとして記録）。
- 子のプロセスと柵に壁時計の時間切れが無いこと（`devroot_cache_tests.rs:483-485`・`devroot_sweep_tests.rs:501-502` に書いてある）。

## 7. 要件と今あるものの対応（Requirement-to-Asset Map）

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1 洗い出し | この報告 | なし（76 件・読み残し 0） |
| 2 確定 | §4.3 の候補 | **Constraint**: 11・35・75 は裁定が要る |
| 3 直し切る | 76 件の直し方 | **Constraint**: 1・3・4・5・9・10・11・15・18・32・35・42・55・58・75 は外の約束に触る（§4.1） |
| 4 テストで縛る | 偽の子・`WorkDir`・私用の名前空間・自分を子として起こす形が既にある | **Missing**: 21〜28 のテスト・6・7・9・10・12・16・17・20 の赤になるテスト。**Unknown**: 13（標準エラーを決定論で読めるか） |
| 5 預かった 2 件 | §5 の事実 | 件 1 は panic の文の詳しさだけ差。件 2 は文の形の決め |
| 6 守る決まり | — | **Constraint**: 32 は ASCII の決まりがテスト用のライブラリの標準エラーに及ぶかの裁定。依存は足さない（SHA-256 は今のまま） |
| 7 範囲の外・C22 | §4.1・4.4 | **Constraint**: 75 は C22 の中で直せない。1 は範囲の外の 2 ファイル |
| 8 重い操作 | 偽の子・単体の筋 | **Constraint**: 1・2・8 は筋まで縛り、効き目は後で確かめる形 |
| 9 仕上げ | — | かけ直しの audit・全体テスト・差分の確かめはタスクの段 |

## 8. 直し方の進め方の案

- **案 A（既にあるものを直す・範囲の中で閉じる）**: 指摘ごとに、今あるファイルの中で直し、テストは隣の `*_tests.rs` に足す。`real-run-kit` の共通の読み手（12）・テストの手助け（49）・見張りの手助け（33）だけを、既にある置き場（`workspace_scan/mod.rs`）か新しい `test_support.rs`・`tests/common/mod.rs` へまとめる。
  - ✅ 変える所が小さい・今の形と決まりに沿う。❌ 外への約束に触る 15 件は、1 件ずつ議題の裁定を待つ。
- **案 B（外への約束も同じ変更で揃える）**: 75 の入口を 1 つに寄せる・3 の新しい変種を足す・32 の ASCII 化を `doc/testing.md` と一緒に直す、など外への約束を変える直しを、読み手を全部直す形で取る。
  - ✅ 写しと二重の名前が残らない。❌ 75 は C22 のほかの 5 本の触るファイルに及ぶ（要件 7.3 に反する）＝このウェーブでは取れない。
- **案 C（混ぜる）**: 外への約束を変えない直し（約 60 件）は案 A で直し、外への約束に触るものは、範囲の外の読み手が 0 か少ないもの（10・35・42・55・58・1・4・5・18・32）を同じ変更で読み手まで直し、C22 と重なる 75 だけを残すか後の spec へ回す（要件 7.9）。
  - ✅ 方針「全部直す」にいちばん近く、並走の約束も守れる。❌ 75 の扱いの裁定が要る。
- **規模**: M（3〜7 日）。今ある形（偽の子・`WorkDir`・私用の名前空間・自分を子として起こすテスト）でほぼ足りる。**危うさ**: 中（見張りを厳しくする 1・4・5 は全部の spec の前提を動かす・1 は範囲の外の 2 ファイル・18 は C22 のほかの 3 本の割りの後の姿に関わる）。

## 9. タスクの数の見立て（要件 2.6）

指摘は 76 件だが、触るファイルのまとまりでタスクに束ねる見込み（例: 見張り ⑷ と範囲の外の 2 ファイル／`env-filter`／一時パスの見張り／走査の部品〔17・20・30・72〕／見張りの手助けと重なり〔18・19・33・34・71・73・74〕／`temp-path-kit`〔6・13・75・76〕／`WorkDir` と検体の窓口〔7・15・16・36・52〜59〕／`fold-samples`〔2・8・56〕／`sample-ghost-kit` のテストの足し〔22〜25〕／同 テストの削り〔14・31・37・60〜70〕／`real-run-kit` の MCP〔3・10・28〕／閉じ方と破棄〔9・27・29・45〕／32bit〔11・51〕／読み手と削り〔12・35・46〜50〕／出所の照らし〔26〕／ASCII〔32〕／`log-capture-kit` の src〔21・38〜44〕／件 1・件 2／かけ直しの audit／全体テストと差分の確かめ）で **約 21〜24 タスク**＝**20 を少し越える見込み**。
- 束ね方をもう一段粗くすれば 20 に収まりうる。記憶の決め「20 タスクは discovery の分け方の目安で spec の上限ではない・一度切り出した後に規模を理由にした分割を後出しで提案しない」により、推す＝1 本のまま（要件 2.6 の議題としては「越える見込み・分けない」を示すだけ）。

## 10. 設計の段へ持ち越す調べもの（Research Needed）

- 13: `TempPath` の破棄が出す 1 行を、決定論のテストで読めるか（標準エラーの受け方）。
- 26: 出所の照らしを、範囲の中の静的なテスト（areka のソースを読む）で書けるか・その照らしが `session-module-gather` の移しの後も自動で追いつくか。
- 30: 見張りの遅さを 1 度測るか（静かな机でない測りの揺れ）。
- 2・8: `fold-samples` の写しの式と書き先を、`vendors/` に触らない単体テストで縛れる形に割れるか。
- 32: `.ps1` が字面で読んでいないことの確かめ直し（`u{XXXX}` を解いた grep・記憶の決め）。
- 34: 消すテストの名前を挙げている完了した spec の文書への扱い（`completed/` は直さない前提で記録の書き方）。

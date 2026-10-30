# Brief: bridge-store-user-root

> 起票: 2026-10-17（`areka-P0-user-data-root` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-user-data-root/tasks.md` の Implementation Notes の「確かめていない件」（Microsoft Store 版の Claude Desktop が隣に置いた中継を起こしたときの控えと書ける根）と、`completed/areka-P0-mcp-stdio-bridge/` の実測（Store 版は控えを自分のパッケージのフォルダへ回す）。区分 B（バグ）。行数とファイルの位置は起票のとき（`user-data-root` のブランチ・2026-10-17）のもの＝着手時に引き直す。

## Problem

- **利用者**: Microsoft Store 版の Claude Desktop から areka をつなぐと、中継（`areka-mcp-bridge.exe`）の控えが areka の本当の記憶の置き場の隣に置かれず、Claude アプリの箱（`%LOCALAPPDATA%\Packages\Claude_…\LocalCache\Local\` の下）へ回される。中継が利用者ごとの置き場（`%LOCALAPPDATA%\areka`）を箱の中に作り、exe の隣の記憶をそこへ写しうる＝areka 本体が使う置き場とは別の、利用者から見えない写しが箱の中にできる。
- **開発者**: `user-data-root` の要件 6.1（中継は areka と同じ決め方で areka の記憶の置き場を決め、控えをその下に置く）が、Store 版の利用者には効いていない見込み。中継が置き場を「決める」だけでなく「作る・写す」（`establish`）まで行っているので、箱の中で作り・写しが起きる。

## Current State

- `user-data-root` から、中継は `crates/areka-mcp-bridge/src/config.rs` の `from_env`（67 行付近）で `areka_user_data::establish(Some(dir), profile_env)` を呼び、`app_profile_dir` で areka の記憶の置き場を決めて控えをその下（`…\profile\areka\mcp-bridge`）に置く。`establish` は書ける根を作り、exe の隣の記憶を写しうる（`crates/areka-user-data/src/establish.rs` 88 行）。
- 完了 `mcp-stdio-bridge` の実測: Store 版の Claude Desktop が起こした中継の `%LOCALAPPDATA%` への書き込みは、パッケージの `LocalCache\Local\` の下へ回される。
- 完了 `user-data-root` の実測: Claude アプリの Bash から起こした areka の `%LOCALAPPDATA%` が逸らされることは見た（PowerShell からは逸らされない）。**利用者ごとの置き場の形での中継は、実機でまだ測っていない**。
- その場で済ませた件（`user-data-root` の完了時・コミット `c28dd71e`）: `dist/README.txt` の中継の控えの段落に、Store 版が起こした中継の書き込みは `LocalCache\Local\` の下へ回る（例 `LocalCache\Local\areka\profile\areka\mcp-bridge\`）と書いた。説明書の上では今の振る舞いを正しく伝えている。

## Desired Outcome

- Store 版の Claude Desktop が起こした中継が、箱の中に利用者ごとの置き場を作らず、exe の隣の記憶を箱の中へ写さない。
- 中継の控えの置き場が、次のどちらかに決まって固定される（要件で選ぶ）:
  - areka 本体の本当の記憶の置き場（箱の外）に届く。
  - 届かないなら、箱の中に置くことを決まりとして受け入れ、説明書と記録でそう伝える（今の `dist/README.txt` の行を確定の文にする）。
- どちらでも、中継の起動の記録に、決めた置き場と「箱へ回されている」ことが分かる行が残る（ログの無い失敗の道を作らない決まり）。

## Approach

1. **最初のタスクは実機の測り**: winget で入れた areka（利用者ごとの置き場の形）の隣の中継を、Store 版の Claude Desktop から起こし、①箱の中に `areka` の根・`profile` が作られるか ②exe の隣の記憶が箱の中へ写されるか ③控えがどこに置かれるか、を数える。比べに、PowerShell から同じ中継を起こした形も測る。
2. 中継は置き場を「決める」だけにする形を推す（`establish` を呼ばず、`decide`／`app_profile_dir` で道筋だけを決め、作る・写すは areka 本体に任せる）。areka 本体がまだ置き場を作っていないときの控えの扱い（控えなしで動く・本体が作るまで待たない）も要件で決める。
3. 箱の外へ届く手（パッケージの外の実の `%LOCALAPPDATA%` を得る道）があるかを調べる。無い・危ういなら、箱の中に置くことを受け入れる側に倒す。
4. 決めた形を、偽の環境（`LOCALAPPDATA` を `target\` の下へ向ける）で決定論テストに固定し、実機でもう一度測る。

## Scope

- **In**: Store 版の Claude Desktop から起こした中継の実機の測り・中継の置き場の決め方の直し（作る・写すを外す）・箱の外へ届く手の調べ・控えの置き場の記録の行・`dist/README.txt` の該当の段落の確定・決定論テスト。
- **Out**: areka 本体の置き場の決め方（完了 `user-data-root`）・中継の MCP の中継の働きそのもの（完了 `mcp-stdio-bridge`）・書けない場所の伝え方（`write-failure-notice`）・Store 版でない Claude Desktop の振る舞い（今の形で正しい）・arm64 の機械で起きるか（次のリリースへの申し送り）。

## Boundary Candidates

- 中継の置き場の決め方（`crates/areka-mcp-bridge/src/config.rs` の `from_env`）と、置き場を作る・写す働き（`areka-user-data` の `establish`）の境目。
- パッケージの箱へ回される書き込みと、回されない読み取りの違い（OS の決まり＝実機で測る）。
- 説明書の中継の控えの段落（`dist/README.txt`）。

## Out of Boundary

- `areka-user-data` の決め方の規則そのもの（`decide`・`app_profile_dir` の答えを変えない）。
- Claude Desktop の側の設定や配り方。

## Upstream / Downstream

- **Upstream**: ✅ `user-data-root`（2026-10-17 完了・中継に `establish` を持ち込んだ）・✅ `mcp-stdio-bridge`（Store 版の箱の実測）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: 完了 `user-data-root`（要件 6.1＝中継の控えの置き場）・完了 `mcp-stdio-bridge`（中継の控え）。
- **Adjacent**:
  - ✅ `write-failure-notice`（2026-10-17 着地）: 書けない場所の伝え方。`areka-user-data` の `open_dir_for_add` を使う側。中継の `config.rs` には触らない見込み＝着手時に照らす。
  - `winget-release-automation`: 説明書（`dist/README.txt`）の別の行を触る。説明書の別々の行は同じウェーブで許す重なり。

## Constraints

- 段は**バグ**・区分 B・v0.1.0（致命的でない新しいバグは次の版へ。areka 本体の記憶は正しい場所にあり、外れるのは中継の控えと箱の中の写しだけ）。
- 実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。Claude Desktop を落とす・起動し直すのは開発者に頼む（動いている別のセッションがある）。止めてよいプロセスは、自分が起こしたと確かめられるものだけ。
- 測りの途中で開発者の本物の `%LOCALAPPDATA%\areka` や箱の中の控えを上書きしない（測る前に控えを取り、測った後に戻す・測りは `AREKA_USER_ROOT` を `target\` の下へ向けた形を先に試す）。
- 依存は足さない。
- 規模の見込み: S（4〜7。測り 1〜2・直し 2〜3・説明書と記録 1〜2）。
- 要件定義のモデル: Opus。

## 優先度の区分

B（バグ。Store 版の利用者で、中継の控えが本当の置き場に届かず、箱の中に置き場の写しができうる）。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: `readonly-mode`（10-24）が `areka-user-data` に「決めるだけで作らない・写さない」口を足した＝`establish_readonly`（書ける根がまだ無ければ作らずに隣の形で返す）と `readonly_app_profile_dir`（写すはずの場面なら写す元を返す）。`crates/areka-user-data/src/establish.rs` は 240→310 行で、判断と記録は `decide_logged` を両方の道が分け合う。この spec の Approach 2（中継は `establish` を呼ばず道筋だけ決める）は、中継の `config.rs`（130 行・67 行目の `establish` の呼び出し）からこの口を呼ぶ形で書ける見込み＝`areka-user-data` に新しい関数を足さずに済む。ただしこの口は、根がまだ無いとき exe の隣の記憶（winget の置き場）を返す＝そこへ控えを書いてよいか（書けなければ控えなしで動く）は要件で決める。中継のコードと `tools/mcp-bridge-install.ps1` は変わっていない。
- **触るファイル**: `crates/areka-mcp-bridge/src/config.rs` と `config_tests.rs`（166）・`dist/README.txt` の中継の控えの段落（289 行目のまま・説明書は 441→460 行）・要れば中継の起動の記録の行。`establish.rs` は読むだけ。
- **規模**: S（4〜7）のまま（使える口ができた分、下限寄り）。
- **先に要るもの**: 無い＝先頭。重なり 0（`config.rs` を挙げるほかの brief は無い。`mcp-strict-errors` は同じクレートの `stdio.rs`・`errors.rs`、rmcp を上げるなら `Cargo.toml`＝ファイルは別。説明書は段落が別＝許す重なり）。依存は足さない・emo-text に触らない。
- **中の待ち**: 最初のタスクの実機の測りで、Store 版の Claude Desktop の設定を `areka.exe` の隣の中継へ向け直して起動し直すのは開発者に頼む（人の待ち）。開発者の机の中継は `AREKA_MCP_HOME` の形（隣に `areka.exe` が無い）なので、箱の中に置き場を作る件は開発者の机では起きない。静かな机・全体テストの比べは要らない。
- **優先度の区分**: B のまま（Store 版の利用者で、中継の控えが本当の置き場に届かず、箱の中に置き場の写しができうるバグ）。
- **要件定義のモデル**: Opus のまま。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: v0.1.0 に残すかの問い（棚卸㉘で開発者に問うた 1 つ目）は、まだ答えが無い＝次のウェーブに入れる前に答えをもらう。Adjacent の `write-failure-notice` を「進行中」と書くが、10-17 に着地している。

# ギャップ分析: areka-P0-release-package-versioned

> 2026-10-03・`/kiro-validate-gap`。対象は確定した requirements.md（要件 1〜8）と、ブランチ `claude/areka-p0-release-package-688ad4`（main `1ce4c74e` の上・spec の初期化コミット `6be3879a`）の実物。ソースの場所は「何の定義か」で指す。§1〜§8 がギャップ分析、§9 が設計フェーズ（同日 `/kiro-spec-design`）の調査と決定。

## 1. まとめ

- **配布スクリプト**（`tools/package-alpha.ps1`・541 行）は、段を直列に回す骨組み・作りかけの片付け・PE の読み取り・中身の検査 8 項目・起動確認と番犬・記録の判定がそろっている。足りないのは「版を読む」「CPU 種別で分ける」「SHA256」「展開した木の片付け」「置き場を `target\` へ」の 5 つで、どれも今の骨組みの中に足せる。
- **areka 本体**で起動した exe のパスを使うのは `crates/areka/src/boot_config.rs` の 3 つの関数（`resolve_root`・`default_helper_exe_path`・`default_app_profile_dir`）だけ（実測・テスト以外のソースを全部検索）。共通の 1 関数を通す形にすれば `main.rs` を変えずに済む。根の行（`root_resolved`「ベースウェアの根を決めました」）は既に info で出ている＝要件 6.8 は新しい行なしで満たせる見込み。
- **リンクの解き方**は標準ライブラリだけで足りる見込み: `std::fs::read_link` は Windows で `\\?\` を外せるときは外した普通の綴りを返す（rustc 1.99.0 の標準ライブラリのソースで確認）。`GetFinalPathNameByHandleW` を直接呼ぶには `windows` クレートの機能（`Win32_Storage_FileSystem`）を `Cargo.toml` に足す必要があり、境界の「`Cargo.toml` の変更 0」とぶつかる。
- **実機の確かめ（要件 7）に大きな前提がある**: この開発機は開発者モードが切れていて、管理者でないとシンボリックリンクを作れない（実測）。winget はリンクを作れないとき、入れ先のフォルダを PATH に足す方へ倒れる（winget-cli PR #2401）＝**リンクの経路を通らないまま「起動できた」になる**。また `LocalManifestFiles` は今 `false`（実測）で、手元のマニフェストを使うには管理者の操作が要る。
- 規模 **M**・危うさ **中**。危うさの中心は要件 7（winget の手元での確かめの段取り）と、シンボリックリンクを作れない機械での検証。

## 2. 今あるもの（実測）

### 2.1 配布スクリプト `tools/package-alpha.ps1`

| 部分 | 今の作り | 本仕様で使えるか |
|---|---|---|
| 較正値の一覧（冒頭） | `SCRIPT_VERSION`・`EXPAND_DIR_MAX_CHARS = 160`・`ALLOWED_EXECUTABLES`・記録の目印など。説明の欄にも同じ一覧 | そのまま足す場所（CPU 種別ごとの機械種別の表を足す） |
| 段の直列（`Step`） | 段の名前・外部コマンドの出力の末尾・失敗で終了コード 1 | そのまま使える。「後片付け」も 1 段として足せる（議題 ⑶ の仮置きに合う） |
| 後始末（`Invoke-Cleanup`） | `.zip.tmp` の削除・子の停止・環境変数と PATH の復元。**展開先と記録は消さない** | `.sha256` の作りかけと、展開した木の削除を足す場所 |
| 前提の確認の段 | `-SmokeExitMs` の検査・`-CheckDir` の絶対パスの検査・展開先の長さ（`-Check` か `-CheckDir` のときだけ）・**リポジトリの中を断る**・git・`cargo about`／`cargo deny`・`Cargo.lock` | 既定の親を `GetTempPath()` から `target\` の下へ替え、断る条件を「`target\` の外」に絞る（要件 1.1〜1.3） |
| ビルド | `rustup target add i686` → x64 本体と i686 補助 exe を `--target-dir target/alpha` で release ビルド（`+crt-static`） | arm64 本体のビルドを同じ形で足す。**`target/alpha` は cargo のビルドの置き場と zip の置き場を兼ねている** |
| PE の読み取り（`Read-PeInfo`） | PE32／PE32+ の機械種別と取り込み表 | arm64（0xAA64）もそのまま読める（PE32+） |
| 謝辞の生成 | `cargo about generate --target x64 --target i686` | arm64 を足すかどうかで中身が変わりうる（§5 議題 9） |
| 組み立て | 9 項目を `target/alpha/stage` へ写し、`BUILD-INFO.txt` に 5 行 | `version=` と CPU 種別の行を足す |
| 圧縮 | 名前 `areka-alpha-x64-<日付>-<コミット>[-dirty].zip`・`.zip.tmp` に作って最後に改名 | 名前を `areka-{版}-{arch}.zip` に。`.sha256` も同じ「仮の名前→改名」の形にできる |
| 中身の検査 8 項目（`Test-ZipContent`） | 5 番の機械種別は表 `{areka.exe=0x8664, helper=0x014c, pasta.dll=0x014c}` に固定。8 番は `commit=`・`dirty=` だけを見る | 5 番の表を CPU 種別で引く・8 番に `version=` と CPU 種別を足す（要件 3.5・4.3） |
| 起動確認（`-Check`） | 展開→起動（`AREKA_*`・`WINTF_*` を外して 4 つだけ入れる）→番犬（自分が起こした子だけ）→記録の判定 7 行 | 展開先と後片付けだけを変える。判定の各行は変更 0（境界どおり） |
| git status 不変の確認 | 始めと終わりの `git status --porcelain` を比べる | `target` は `.gitignore` の 1 行目（実測）＝`target\` の下に書いても結果は変わらない（要件 1.9） |

- 説明の欄の `.EXAMPLE` が `-CheckDir C:\t` を勧めている（要件 1.10 の直す対象）。
- 自己検査（`tools/` の中のテスト）は無い。`tools/test-all.ps1` は配布スクリプトを呼ばない。

### 2.2 areka 本体の場所の解決 `crates/areka/src/boot_config.rs`

- `resolve_root_from(env, exe)`: 純粋な判断。`AREKA_ROOT` が在ればそれ、無ければ exe の親。`std::path::absolute` で絶対化し、`canonicalize` は使わない（説明に明記）。テストは `crates/areka/src/main_config_input_tests.rs` の `mod root`（7 件・一時フォルダは `temp-path-kit`）。
- `resolve_root()`: env と `current_exe()` を読んで上へ渡す薄い口。
- `default_helper_exe_path()`: `current_exe()` の親＋`shiori-host32-helper.exe`。失敗時は `"."`。呼び手は `main.rs` の 1 か所。
- `default_app_profile_dir()`: `AREKA_PROFILE_DIR` が在ればそれ、無ければ exe の親＋`profile\areka`。**呼び手が多い**（`resolve_boot`・`ghost_boot_options`・`ghost_session.rs`・`shiori_host.rs`・`main.rs`）＝起動の後もゴーストの切替などで何度も呼ばれる。
- ログの初期化（`main.rs` の `tracing_subscriber::fmt()`）は `resolve_boot` より前＝要件 6.6 の警告は記録に載る。
- 他の `current_exe()`: `wintf` の `bitmap_source::resolve_path`（相対パスの画像だけ）は areka の本番の経路で使っていない（areka 側の `resolve_path` は説明書の別関数）。areka は作業フォルダ（カレント）に頼っていない（`current_dir` の使用は補助 exe の起動の `current_dir(load_dir)` だけ）。

### 2.3 シンボリックリンクまわりの既存の資産

- `crates/areka-update/src/testkit.rs` に、ジャンクション（フォルダのリンク・`mklink /J`）を作るテスト用の関数がある。**ファイルのシンボリックリンクを作る道具は無い**。ジャンクションは管理者でなくても作れるが、exe（ファイル）のリンクの代わりにはならない。
- 判断を注入で検査する流儀（`resolve_root_from` が env と exe を引数で受ける）が既にある＝リンクの読み取りも引数で渡す形にそろえられる。

### 2.4 実測した環境の事実

| 項目 | 実測 |
|---|---|
| ワークツリーの根の長さ | 72 字（`…\areka-p0-release-package-688ad4\`） |
| `<根>\target\alpha-check\areka-alpha-check-HHmmss` | 115 字（上限 160 の内） |
| winget の portable の入れ先の例（利用者名 `maz-o`） | `C:\Users\maz-o\AppData\Local\Microsoft\WinGet\Packages\Areka.Areka.Portable_Microsoft.Winget.Source_8wekyb3d8bbwe` で 113 字。zip の中の最も深い書き込み（約 93 字・完了 `alpha-package` の調査 §4.2）を足しても 206 字＝260 字の内 |
| シンボリックリンクの作成（管理者でない・開発者モード） | **失敗**「この操作には管理者特権が必要です」（`target\` の下で試し、試した物は消した） |
| winget | v1.29.380・`LocalManifestFiles=false` |
| Rust | rustc 1.99.0・`aarch64-pc-windows-msvc` の target は入っている・MSVC の bin に arm64 の道具がある |
| `.github/` | 無い |

## 3. 要件と今の資産の対応

| 要件 | 今の資産 | 足りないもの | 種類 |
|---|---|---|---|
| 1.1 展開先を `target\` の下 | 前提の確認の段・`$script:ExpandDir` | 既定の親の差し替え | 欠け |
| 1.2 160 字の上限 | `EXPAND_DIR_MAX_CHARS` の検査 | 既定の置き場でも常に検査（今は `-Check` か `-CheckDir` のときだけ） | 欠け（小） |
| 1.3 `-CheckDir` の受け入れ | リポジトリの中を断る検査 | 「`target\` の下なら受け付ける」の分岐 | 欠け |
| 1.4〜1.7 片付けと残す | 無し（記録の判定の段は置き場を印字するだけ） | 合否で分かれる削除・残す引数・消せなかったときの 1 | 欠け |
| 1.8 子が終わった後に消す | 番犬（自分の子だけ） | 削除の順番。補助 exe（孫）は job で道連れに止まるが、止まり切るまでの間ファイルが掴まれうる | 制約 |
| 1.9 git status | 既存の段 | 変更なし | − |
| 1.10 説明の例 | `.EXAMPLE` | 書き換え | 欠け（小） |
| 2.1 版から名前 | 無し（版を読まない） | 版の読み取り・名前の組み立て | 欠け |
| 2.2 `.sha256` | 無し | `Get-FileHash` と 1 行の書き出し（書式の細部は §5 議題 15） | 欠け |
| 2.3 版を読めない → 3 | 前提の確認の段 | 読み取りの失敗の分岐 | 欠け |
| 2.4・2.5 置き換え・作りかけを残さない | `.zip.tmp` → 改名・`Invoke-Cleanup` | `.sha256` の仮の名前・**前回の同名の zip の扱い**（§5 議題 7） | 欠け・不明 |
| 2.6 印字 | 完成の段 | 版・CPU 種別ごとの印字 | 欠け（小） |
| 3.1・3.2 `-Arch` | 無し | 引数と検査 | 欠け |
| 3.3 arm64 のビルド | x64 の段 | 同じ形の段 | 欠け |
| 3.4・5.3 道具が無い → 3 | `cargo about`／`cargo deny` の検査 | arm64 の道具の調べ方（§5 議題 10） | 不明 |
| 3.5 検査を arm64 に | 5 番の固定の表 | CPU 種別で引く表 | 欠け |
| 3.6 中身の構成は同じ | 組み立ての段 | 謝辞の扱い（§5 議題 9） | 不明 |
| 3.7 `all` は両方そろって成功 | 無し | 片方が落ちたときの片付け（§5 議題 8） | 欠け |
| 3.8・3.9 起動確認は x64 | `-Check` | 組み合わせの検査と印字 | 欠け（小） |
| 4.1〜4.3 `BUILD-INFO.txt` | 5 行と 8 番の検査 | 2 行と検査の追加 | 欠け（小） |
| 5.1 窓を出さない | `-Check` なしなら窓も問いかけも無い（実物を読んで確認） | 変更なし | − |
| 5.2・5.4 同じ名前・同じ終了コード | 0〜3 | 新しい番号を足さない形で上の欠けを埋める | 制約 |
| 5.5 置き場を固定 | `target/alpha/` | 新しい名前にするか（§5 議題 6） | 不明 |
| 6.1〜6.3 リンクを解く | 3 関数が `current_exe()` を直に使う | 共通の 1 関数・リンクのときだけ解く | 欠け |
| 6.4 環境変数が勝つ | 既存の分岐 | 変更なし（解決は env の無い側だけに効かせる） | 制約 |
| 6.5 `\\?\` を付けない | `canonicalize` を使わない方針 | 解き方の選択（§4） | 制約 |
| 6.6 解けない → 警告して続ける | 無し | 警告の行（何度も出さない工夫・§5 議題 2） | 欠け |
| 6.7 決定論テスト | `mod root` の注入の流儀 | リンクの読み取りを注入する判断の関数とその検査 | 欠け |
| 6.8 根の行 | `root_resolved` の info 行（既存） | 変更なしで足りる見込み | − |
| 7.1〜7.5 winget の実機の確かめ | 無し | 手元のマニフェスト・zip を配る口・権限の段取り（§5 議題 4・5） | 不明・調査要 |
| 8.1 改名 | 旧名のファイル | `git mv` と自分の名前を書く行（`BUILD-INFO.txt` の `script=`） | 欠け |
| 8.2 説明の欄 | 既存の欄 | 書き足し | 欠け |
| 8.3・8.4 steering | `structure.md` の「その他の最上位」の 1 行・`tech.md` の arm64 の 2 行（既にある） | 新しい名前と役割・「補助 exe は arm64 の zip でも i686」 | 欠け（小） |
| 8.5 過去の記録は触らない | 完了 spec に旧名が多数（`completed/` の下） | 触らない | 制約 |

## 4. 作り方の候補

### 4.1 配布スクリプト

**案 A: 今のスクリプトを改名して育てる（1 ファイルのまま）**
- `git mv tools/package-alpha.ps1 tools/package.ps1` の上で、前提の確認の段・後始末・ビルドの段・圧縮・中身の検査・`-Check` の段を書き換える。CPU 種別ごとの違い（target の三つ組・期待する機械種別・zip の名前）は冒頭の表 1 つにまとめ、ビルドから完成までを CPU 種別の数だけ回す。
- ✅ 段の骨組み・終了コード・片付けの流儀をそのまま使える。下流（`release-ci-workflow`・`mcp-stdio-bridge`）が触る場所も 1 か所。
- ❌ 541 行から 700 行前後へ増える見込み。1,000 行の決まり（`structure.md`）の内には収まる。

**案 B: 共通の関数を別ファイル（`.psm1` など）へ分ける**
- PE の読み取り・中身の検査・記録の判定を分ける。
- ✅ 1 ファイルが軽くなる。将来の自己検査を足しやすい。
- ❌ 今は自己検査が無く、呼び手もこのスクリプトだけ＝分ける理由が弱い。ファイルが増え、下流の spec の触る場所も増える。

**案 C: CPU 種別ごとにスクリプトを 1 回ずつ呼ぶ外側の口を作る**
- `-Arch all` を外側で 2 回呼ぶ形。
- ❌ 要件 3.7（片方が落ちたらもう片方も完成品として残さない）を外側で片付けることになり、片付けの責任が 2 か所に割れる。勧めにくい。

### 4.2 リンクの解決（`boot_config.rs`）

共通の形: 判断を純粋な関数（例: 起動した exe のパスと「リンクを読む関数」を受け取り、使うパスと警告の理由を返す）に置き、3 つの関数はその結果を使う。`main.rs` は変えない。

**案 1: `std::fs::symlink_metadata` でリンクかを見て、`std::fs::read_link` を繰り返し、相対の先はリンクの置き場から `std::path::absolute` で絶対化する**
- ✅ 標準ライブラリだけ（`Cargo.toml` の変更 0）。`read_link` は `\??\`→`\\?\` に直したうえで、外せるときは外した普通の綴りを返す（rustc 1.99.0 の標準ライブラリ `sys/fs/windows.rs` の `readlink` の中の `from_wide_to_user_path`）。リンクの先の綴りをそのまま使うので、途中のドライブ文字や `subst` の綴りを書き換えない。要件 6.2（リンクのリンク・相対の先）を自分で辿るので回数の上限（輪になったリンク）を決めておく必要がある。
- ❌ 辿る処理を自分で書く（数十行）。外せない形の `\\?\` が残ったときの扱い（警告して起動した exe のパスへ戻す等）を決める必要。

**案 2: リンクのときだけ `std::fs::canonicalize` で最終のパスを取り、`\\?\`（と `\\?\UNC\`）を自分で外す**
- ✅ 辿る処理は OS 任せで短い。
- ❌ 途中のジャンクションや `subst`・割り当てたネットワークドライブも実の場所へ展開される（リンクのときだけなので今の起動には影響しないが、winget の入れ先が `subst` の下にある等の珍しい場合に長くなる）。接頭辞を外す処理は自前（`dunce` クレートは `Cargo.lock` に無い・依存の追加は境界外）。

**案 3: `GetFinalPathNameByHandleW` を直接呼ぶ**
- ❌ `windows` クレートの `Win32_Storage_FileSystem` 機能が要る（ワークスペースにも各クレートにも今は無い・実測）。境界の「各 `Cargo.toml` の依存と版の変更 0」と衝突する。中身は案 2 と同じ。

**解いた結果の持ち方**: `default_app_profile_dir` は実行中に何度も呼ばれる。毎回解くと、解けない場合の警告が繰り返し出て、途中でリンクが張り替えられたときに置き場が変わりうる。プロセスで 1 回だけ解いて覚える形（`std::sync::OnceLock`）が候補（§5 議題 2）。

### 4.3 推す組み合わせ（決定ではない）

スクリプトは案 A、リンクは案 1＋1 回だけ解いて覚える形。理由: どちらも新しい依存とファイルを増やさず、境界（`Cargo.toml`・`main.rs` の変更 0）を守ったまま要件 6.3（リンクでないときは今と同じ）と 6.5（接頭辞を付けない）を満たしやすい。

## 5. 設計の分かれ目（要件討議への議題の候補）

1. **リンクの解き方**: 案 1（`read_link` を繰り返す）か案 2（`canonicalize`＋接頭辞外し）か。案 3 は境界の変更が要る。案 1 なら辿る回数の上限と、外せない `\\?\` が残ったときの扱いも決める。
2. **解いた結果を 1 回だけ求めて覚えるか**: 覚えないと警告（要件 6.6）が呼ばれるたびに出る。覚えると、テストでは判断の関数を直に呼ぶ形になる。
3. **決定論テストの形（要件 6.7）**: リンクの読み取りを注入する判断の関数だけを検査するか、加えて実物のシンボリックリンクで 1 本検査するか。**この開発機は管理者でないとシンボリックリンクを作れない**（実測）ので、実物の検査は「作れないときにどうするか」（黙って飛ばす形は取らない方針との兼ね合い）を決める必要がある。
4. **実機の確かめ（要件 7）の権限の段取り**: winget はリンクを作れないと PATH を足す方へ倒れる（PR #2401）。この開発機の今の状態（開発者モードが切れている・管理者でない）で `winget install` すると、**リンクを通らずに起動してしまい要件 7.2 を満たせない**。選択肢は (a) 確かめの間だけ開発者モードを入れて後で戻す（OS の設定＝要件 7.4 は「winget の設定」しか挙げていない）、(b) 管理者の PowerShell で winget を走らせる（入れ先とリンクの置き場が利用者の範囲のままかは未確認）。どちらにしても `winget settings --enable LocalManifestFiles` は管理者の操作で、後で `--disable` に戻す。
5. **手元のマニフェストで zip をどう渡すか**: マニフェストの `InstallerUrl` にローカルのパスを書くと期待どおり動かない既知の問題がある（winget-cli #4358）。`localhost` の http で `target\` の下の zip を配る形が候補（道具は何にするか・開発機に何があるかは設計で調べる）。
6. **出力先のフォルダの名前（要件 5.5）**: `target/alpha/` のままか、新しい名前（例 `target/package/`）にするか。今は cargo のビルドの置き場（`--target-dir`）も兼ねているので、変えると 1 回目は全部ビルドし直しになる。CI（`release-ci-workflow`）が拾う場所になるので、ここで固定する。
7. **失敗したときの、前回の同じ名前の zip**: 版が同じなら前回の成功の zip が同じ名前で残っている。今回が途中で落ちると、それが「完成品に見える」まま残る（要件 2.4 と 2.5 の間の隙間）。始めに同じ名前の zip と `.sha256` を消すか、成功したときだけ置き換えて古い物は残すか。
8. **`-Arch all` の片方が落ちたときの片付け（要件 3.7）**: 両方を仮の名前で作って最後にそろえて改名するか、先にできた方を後で消すか。
9. **謝辞（`THIRD-PARTY-NOTICES.md`）**: 3 つのターゲット（x64・arm64・i686）で 1 つ作って両方の zip に同じ物を入れるか、CPU 種別ごとに作るか。前者は要件 3.6（違いは `areka.exe` だけ）にそのまま合うが、x64 の zip の謝辞の中身が今と変わりうる（`Cargo.lock` に CPU 種別ごとのクレートは見当たらず、差は小さい見込み）。
10. **arm64 の道具の有無の調べ方（要件 3.4・ビルドの前に 3 で止める）**: rustup の target の有無に加え、Visual Studio の ARM64 の道具の有無を `vswhere`（`-requires Microsoft.VisualStudio.Component.VC.Tools.ARM64`）で見るか。i686 と同じく `rustup target add` を段として自動で行うか、無ければ 3 で止めるか。
11. **版の読み方**: brief の `cargo metadata --no-deps --format-version 1`（JSON を読む・ネットワーク不要）か、`Cargo.toml` を文字として読むか。あわせて、版の文字に `+`（ビルドの付記）が入るとファイル名や URL で困るので弾くか。
12. **展開した木を消す前の待ち**: 補助 exe は areka が終わると道連れで止まる（`shiori-host32-host` の job）が、止まり切るまでや Windows Defender の走査の間はファイルが掴まれうる。少し待って何度か試してから 1 で終わるか、1 回で 1 にするか。
13. **名前の細部**: 展開先のフォルダ名（`areka-alpha-check-…` の「alpha」を残すか）・残す引数の名前・記録の置き場（今の「展開先の名前＋`-logs`」のままか）。
14. **改名に伴う他の文書**: 要件 8 は `structure.md`・`tech.md` だけを挙げる。旧名は他に `roadmap.md`（3 か所）・`product.md`（1 か所）・進行中の spec の brief（`mcp-stdio-bridge`・`mcp-server-core`・`release-ci-workflow`）にある。直すか、それぞれの着手のときの引き直しに任せるか。
15. **`.sha256` の書式の細部**（勝者がほぼ明白・設計で決めてよい候補）: 16 進は小文字にそろえる（`Get-FileHash` は大文字を返す）・改行は LF・BOM なし（Linux の `sha256sum -c` が CRLF の行のファイル名を読み違えないように）。
16. **winget で外すときの記憶の扱い**（要件 7.3 の後片付けにかかわる）: areka は入れ先のフォルダの中（`profile\areka`・ゴーストの `profile`）に書く。winget の portable の削除で、これらが消えるか残るか（`--purge`・`--preserve` の既定）を確かめ、確かめの後にきれいに外す手順を決める。利用者への影響（更新で記憶が消えるか）は下流の `winget-manifest-submission` の関心。

### 5.1 要件討議（2026-10-03）での振り分け

- **要件で決めた**: 議題 7（始めに同じ名前の組を消し、成功したときだけ置く＝要件 2.4）・議題 14（生きている文書の旧名を直す＝要件 8.6）。議題 4・5 は開発者の裁定で (a)＝確かめの間だけ開発者モードをオン＋管理者で `LocalManifestFiles` をオン・winget は利用者の権限・終わったら両方戻す・zip は `localhost` の http で渡す（要件 7.4・7.6）。
- **設計（`/kiro-spec-design`）で決める**: 議題 1・2・3・6・8・9・10・11・12・13・15。議題 3 は「この開発機は管理者でないとシンボリックリンクを作れない」前提で、黙って飛ばすテストを置かない形にする。
- **下流へ申し送る**: 議題 16（winget の portable を外す・上げるときに、入れ先の中の記憶〔`profile\areka`・ゴーストの `profile`〕が消えるか）は、利用者の記憶が版を上げるたびに消えうる問題で、`areka-P0-winget-manifest-submission` の brief にはまだ載っていない（2026-10-03 実測）。本仕様では要件 7.3 の後片付けの手順に効くところだけを設計で確かめる。

## 6. 調べ残し（設計で調べる）

- **リンク経由の起動での `current_exe()` の値**: Windows では `GetModuleFileNameW` を使い、リンク経由ではリンクの場所を返すと強く推定される（winget-cli #2889 の「リンク経由だと隣の DLL を見失う」と同じ仕組み）。Rust の説明は「プラットフォームによる」とだけ書く。この機械ではリンクを作れず実測できなかった＝設計か実機の確かめで確かめる。
- winget の手元のマニフェストで、管理者で入れたときの入れ先とリンクの置き場（議題 4 の (b)）。
- `localhost` の http で配るための道具（議題 5）。
- GitHub の Windows ランナーに Visual Studio の ARM64 の道具が入っているか（下流 `release-ci-workflow` の関心・本仕様では要件 5.3 の印字の形だけ合わせる）。
- 利用者名が長い機械での winget の入れ先の長さ（実測の例は 113 字・上限まで余裕はあるが、`emo2` の約 160 字の決まりとの関係を `winget-manifest-submission` へ申し送る候補）。

## 7. 規模と危うさ

- **規模: M（3〜7 日）** — スクリプトの書き換えは既存の骨組みの延長、本体は 1 ファイルとその兄弟のテスト。winget の実機の確かめに段取りが要る。
- **危うさ: 中** — スクリプトと本体は既知の作りの延長で低い。winget の手元での確かめ（権限・配り方・リンクを作れない機械での倒れ方）は未経験の外部の道具で、要件 7 の判定を誤る（リンクを通らずに「起動できた」と記録する）おそれがある。

## 8. 設計への申し送り

- 推す形は §4.3（スクリプトは改名して育てる・リンクは標準ライブラリで解いて 1 回だけ覚える）。
- 議題 4（実機の権限の段取り）と議題 7（前回の同名の zip）は、答えで作業や要件の読みが変わるので要件討議で決めるのがよい。議題 15 は設計で決めてよい。
- 実機の確かめでは、`areka` がリンクを指していたこと（`(Get-Command areka).Source` がリンクの置き場で、その `LinkType` が `SymbolicLink`）と、起動の記録の「ベースウェアの根を決めました」の `root=` がリンクの先であることを、両方記録に残す（片方だけだと PATH 経由の起動と見分けられない）。

## 9. 設計フェーズの調査と決定（2026-10-03・`/kiro-spec-design`）

### 9.1 まとめ

- **Discovery の種類**: 既存の仕組みの拡張（light）。配布スクリプトも本体の 3 関数も既知の作りの延長で、新しい依存を 1 つも足さない。設計の調べ物は §6 の「調べ残し」のうち本仕様の内のものと、§5 の議題 1・2・3・6・8・9・10・11・12・13・15 の決定に絞った。
- **主な知見**:
  - 開発機の実測: Python 3.13（`python.exe`）・`vswhere.exe`（VS Installer 同梱）・VS 2022 Professional に ARM64 の部品あり・rustup に `aarch64-pc-windows-msvc` あり・`cargo metadata --no-deps --format-version 1` が `areka` の `version=0.0.1` を返す・PowerShell 7.6.6・OS の開発者モードは切（レジストリ `AllowDevelopmentWithoutDevLicense` 未設定）。
  - 完成品の名前の zip と `.sha256` を**最後の段で一度に改名**すれば、要件 2.4・2.5・3.7（作りかけを残さない・両方そろって成功・前回の物を完成品に見せない）が 1 つの決め方で満たされる。起動確認は `.zip.tmp` のままでも展開できる。
  - 本体の判断は「I/O を注入した純粋な関数＋`OnceLock` の口」で、`Cargo.toml` の変更 0・`main.rs` の変更 0・テストはリンクを作れない機械でも全分岐を踏める。
  - ログの初期化（`main` の `tracing_subscriber::fmt()`）は `resolve_boot` と `default_helper_exe_path` より前＝初回の警告は記録に載る（実物で確認）。

### 9.2 調査の記録

#### リンク経由の起動での `current_exe()` の値（§6 の調べ残し）
- **文脈**: リンクの場所が返るのか、先が返るのか。
- **出典**: Rust 標準ライブラリ（Windows の `current_exe` は `GetModuleFileNameW`）・winget-cli #2889（リンク経由で隣の DLL を見失う＝アプリの場所がリンクの置き場と扱われる）。
- **知見**: リンクの場所を返すと強く推定。この機械ではリンクを作れず実測できなかった。
- **設計への影響**: 設計はこの前提で組む。実機の確かめ（要件 7）で `root=` がパッケージのフォルダになることを判定し、もし `current_exe()` が先を返す環境でも `NotALink` の枝で同じ結果になる（どちらでも壊れない）。

#### `std::fs::read_link` の綴り
- **出典**: rustc 1.99.0 の標準ライブラリ `sys/fs/windows.rs`（`readlink` → `from_wide_to_user_path`）。
- **知見**: 再解析の `\??\` を `\\?\` に直し、外せるときは外した普通の綴りを返す。外せないのは 260 字超や末尾の空白・点など。
- **影響**: 本体は `read_link` の綴りをそのまま使い、接頭辞が残ったら警告して起動した exe のパスへ倒す（6.5・6.6）。自前で接頭辞を外す処理は書かない（外せない形は `emo2` の 160 字でも壊れる形で、直す価値が無い）。

#### 手元のマニフェストで zip を配る口（§5 議題 5 の道具）
- **知見**: 開発機に Python 3.13 が在る。`python -m http.server <port> --bind 127.0.0.1 --directory target\package` で `target\package` の zip を `http://127.0.0.1:<port>/areka-{版}-x64.zip` として配れる。PowerShell の `HttpListener` で書く案は数十行で増える分だけ損。
- **影響**: 設計は http.server を既定の道具とし、開発者が代えてよいと書く。

#### winget の portable を外すときの入れ先の中のファイル（§5 議題 16 のうち本仕様に効く分）
- **知見**: winget の portable の削除は、winget が置いたファイル以外が入れ先に在ると止まり、`--purge` で全部消す（`--preserve` で残す）。areka は入れ先の中に `profile\areka` とゴーストの `profile` を書く。
- **影響**: 確かめの後片付けは `winget uninstall Areka.Areka.Portable --purge`（検体を丸ごと消す・7.3）。利用者向けの扱いは `winget-manifest-submission` へ申し送り済み（§5.1）。

#### arm64 の道具の有無の調べ方（議題 10）
- **知見**: `vswhere.exe -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath` がこの機械で VS 2022 Professional のパスを返す（空なら部品が無い）。`-products *` で Build Tools も拾う。GitHub の Windows ランナーにも `vswhere` は在る（下流の関心）。
- **影響**: `-Arch` に arm64 を含むときだけ前提の確認で呼び、空なら 3。rustup の target は i686 と同じく段として `rustup target add`（無ければ入れる・失敗は 1）。

### 9.3 設計の決定（§5 の議題の答え）

| 議題 | 決定 | 理由 |
|---|---|---|
| 1 リンクの解き方 | 案 1（`symlink_metadata`＋`read_link` を繰り返す・相対は親と結合して `std::path::absolute`）。上限 32 回・接頭辞が残れば警告して倒す | 標準ライブラリだけ・`Cargo.toml` 変更 0・途中のドライブ文字や `subst` の綴りを書き換えない（6.3・6.5）。`canonicalize`（案 2）は珍しい環境で長い実パスへ展開しうる |
| 2 1 回だけ求めて覚えるか | 覚える（`OnceLock<Option<PathBuf>>`・`exe_location`）。警告は初回に 1 度 | `default_app_profile_dir` は何度も呼ばれる。警告の繰り返しと途中の張り替えによる置き場の揺れを避ける |
| 3 決定論テストの形 | 偽の `probe` を注入して `follow_exe_links` の全分岐（7 件）を踏む。実物のリンクのテストは置かない | この機械は管理者でないとリンクを作れない＝黙って飛ばすテストになる。実物の経路は要件 7 の実機が 1 回踏む。`probe_link`・`exe_location` は配線 |
| 6 出力先の名前 | `target/package/`（cargo の `--target-dir`・stage・zip・`.sha256`・`check-*` を兼ねる） | スクリプトの名前から「alpha」を落とすのと同じ理由。1 回目の全ビルドのやり直しは 1 度きり。CI が拾う場所はここで固定 |
| 8 `all` の片方が落ちたとき | 全組を仮の名前（`.zip.tmp`・`.sha256.tmp`）で作り、最後の段「完成」で一度に改名。失敗の経路は `.tmp` と今回の完成品を全部消す | 2.4・2.5・3.7 を 1 つの決め方で満たす。起動確認は `.tmp` のまま展開できる |
| 9 謝辞 | 3 ターゲット（x64・arm64・i686）で 1 つを `-Arch` に依らず生成 | 3.6（違いは `areka.exe` だけ）と 5.2（手元 `x64` と CI `all` で同じ中身）に合う |
| 10 arm64 の道具 | 前提の確認で `vswhere` を見て部品名と入れ方を印字して 3。rustup の target は段として足す | 上の調査。「黙って x64 だけ作らない」 |
| 11 版の読み方 | `cargo metadata --no-deps --locked --format-version 1` の `areka` パッケージの `version`。`^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$` に合わなければ 3（`+` を弾く） | cargo は既に前提の道具。`Cargo.toml` を文字で読む正規表現より壊れにくい。`+` はファイル名と URL で困る |
| 12 消す前の待ち | `Remove-Item -Recurse` を 5 回まで 1 秒おきに試し、消せなければ段「後片付け」の失敗（1） | 補助 exe の終了と Defender の走査の間だけ掴まれる。1 回で 1 にすると偽の赤が出やすい |
| 13 名前の細部 | 展開先 `check-<HHmmss>`・記録 `check-<HHmmss>-logs`（「alpha」を落とす・`-logs` は保つ）・残す引数 `-KeepExpanded`・stage は `stage-{arch}` | 受入記録が `check-…-logs` の形で記録を引いてきた流儀に合う |
| 15 `.sha256` の書式 | 小文字 16 進 64 字・空白 2 つ・ファイル名・LF・BOM なし | `sha256sum -c` がそのまま通る。`Get-FileHash` は大文字を返すので小文字へ |
| （追加）完成を最後へ | 「完成」の段を「git status 不変の確認」の後の最後に置く | 議題 8 の帰結。完成品の名前の物が在る ⇔ 終了コード 0 |
| （追加）1.2 の検査の範囲 | 展開先の長さは `-Check` か `-CheckDir` を付けたときだけ検査（今どおり） | 要件 1.2 は起動確認の文脈。`-Check` 無しの CI の長いパスで止めない |
| （追加）警告の形 | `warn!(event = "exe_link_unresolved", exe, reason)` 1 行。新しい info の行は足さない | 6.6 は警告を求め、6.8 は新しい行を足さないと言う。実機の判定は既存の `root_resolved` の `root=` で足りる |
| （設計討議 2026-10-03・自明な修正）`vswhere` の探し方 | 較正値 `VSWHERE_PATH`（`%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe`）→ PATH の順。無ければ探した場所を印字して 3 | `vswhere.exe` は PATH に載らない（開発機で `where.exe vswhere` が空・固定の場所には在る）。名前だけで呼ぶと部品のそろった機械でも常に 3 |
| （同）後始末の契約 | `Invoke-Cleanup([int]$Code)`。`Exit-Script` は自分の `$Code` を渡し、成功の末尾は `Invoke-Cleanup 0`。`$script:Finalized` は改名が済んだ物から順に登録 | 今の `Invoke-Cleanup` は引数なし。終了コードを渡さないと「完成品が在る ⇔ 0」が実装の暗黙の約束になる（設計検証の指摘 2） |
| （同）rustup のターゲット | 「欠けたら 3」に数えず、今どおり段 `rustup target add` で足す（失敗は 1） | 自動で足せる物は前提の欠けではない。CI に別の段を要らせない |
| （同）`TooManyHops` の数え方 | `probe` が `Target` を返した回数が 32 に達したら次を読まない（`probe` はちょうど 32 回） | テストの期待値を 1 つに固定する（設計検証の残る危うさ） |

### 9.4 統合の観点（設計の整理）

- **一般化**: zip の名前・`arch=` の行・機械種別の期待値・ビルドの target は、較正値の表 `ARCHS` と 1 関数 `Get-ArtifactNames` からだけ引く。CPU 種別を足すとき（中継 exe の同梱など）は表に 1 行で済む。本体側は「起動した exe の場所」を 1 つの口に集め、3 関数が同じ値を使う。
- **作るか借りるか**: ハッシュは `Get-FileHash`、zip は .NET、リンクの読み取りは標準ライブラリ、配る口は Python の `http.server`。新しいクレートも新しい道具も足さない。`dunce` のような接頭辞を外すクレートも `GetFinalPathNameByHandleW` も採らない（`Cargo.toml` 変更 0）。
- **単純化**: 共通の関数を別ファイルへ分ける案（§4.1 案 B）・外側で 2 回呼ぶ案（案 C）は採らない。自己検査の枠（Pester 等）は足さず、実走の一覧を `verification/` に記録する（今どおり）。`exe_location` の結果に「リンクを解いたか」の印は持たせない（使う側が無い）。

### 9.5 危うさと手当て

- **`current_exe()` がリンクの先を返す環境**（推定が外れる場合）→ `NotALink` の枝で同じ結果になるので壊れない。実機の確かめで `root=` を見る。
- **winget がリンクを作らず PATH へ倒れる**（開発者モードが切のまま）→ 7.2 の判定（`LinkType` と `root=` の両方）で「確かめは済んでいない」と判る。手順に開発者モードのオンを明記（開発者の手）。
- **後片付けで木を消せない**（補助 exe の掴み・Defender）→ 5 回の再試行の後に 1 で止め、パスと理由を印字（黙って残さない）。
- **`cargo about --target aarch64-pc-windows-msvc` が x64 だけの謝辞と差を生む**→ `Cargo.lock` に CPU 種別ごとのクレートは見当たらず差は小さい見込み。中身の検査 8 番は写し元とバイト比較なので、差があっても検査は通る（zip の中と写し元は同じ物）。
- **1 回目の `target/package/` への全ビルド**（`target/alpha` から移る）→ 1 度きり。古い `target/alpha` は開発者が消してよい（本仕様は触らない）。

## 出典

- [winget-cli #2889（リンク経由の起動で隣の DLL を見失う）](https://github.com/microsoft/winget-cli/issues/2889)
- [winget-cli PR #2401（リンクを作れないときは PATH を足す）](https://github.com/microsoft/winget-cli/pull/2401)
- [winget-cli #4358（`InstallerUrl` のローカルのパスが期待どおり動かない）](https://github.com/microsoft/winget-cli/issues/4358)
- [winget install（`--manifest` と `LocalManifestFiles`）](https://learn.microsoft.com/en-us/windows/package-manager/winget/install)
- [Rust `std::env::current_exe`（リンクの扱いはプラットフォームによる）](https://doc.rust-lang.org/std/env/fn.current_exe.html)

# 反復実行の手順（要件 9.1-9.4）

本書は `areka-P0-test-cage-determinism` の反復検証（同じテストを規定回数くり返し、
1 回も赤が出ないことを示す）を、**誰が何度やっても同じ形で回せる**ようにするための手順書である。
道具の本体は同じフォルダの `repeat-tests.ps1`、走行の記録は `summary.md` に貯まる。
数値の解釈（何回で足りるか・赤が製品欠陥かどうか）は requirements.md 側に置く。

`remeasure.md`（着手前インベントリの再計測手順）と同じ流儀で、**コマンドをそのまま再実行できる形**で残す。

## 0. 実行前提

| 項目 | 値 |
|---|---|
| 作成日 | 2026-08-24（タスク 8.1） |
| ブランチ | `claude/areka-p0-test-cage-determinism-dad056` |
| HEAD | `170edd6a` |
| シェル | **PowerShell 7**（`pwsh`）。リポジトリ**ルート**で実行する |
| cargo | `cargo 1.98.0 (797e8a9bc 2026-08-05)` |
| 前提成果物 | `target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe` と `shiori.dll` |

32bit 側の成果物が無いと `cargo test --workspace` の一部が落ちる。先に:

```powershell
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc
```

`repeat-tests.ps1` は走行前にこの 2 つの在否を検査し、**無ければ走らずに止まる**
（`-SkipI686Check` で外せるが、外した事実は要約の表に残る）。

---

## 1. 使い方

```powershell
# 素の呼び方（PowerShell 7 のプロンプトから、リポジトリルートで）
.\.kiro\specs\areka-P0-test-cage-determinism\verification\repeat-tests.ps1 -Target wintf -Times 30 -Parallel 4
```

| 引数 | 意味 | 既定 |
|---|---|---|
| `-Target` | 対象の名前（§2 の表）。`custom` なら `-CargoArgs` を渡す | 必須 |
| `-Times` | **総回数**（同時に走るぶんも 1 回と数える） | 1 |
| `-Parallel` | 1 巡で同時に起動するプロセス数＝**負荷**（§3） | 1 |
| `-ExpectPassed` | 1 回あたりの期待 passed 件数。`-1` で指定なし | §2 の表の値 |
| `-TimeoutSec` | **1 回あたりの上限秒**（ハングの止め木・§5-b）。`0` で自動 | 自動 |
| `-Tag` | ログ名と要約の見出しに使う札 | 対象名 |
| `-Root` | 走らせるリポジトリのルート（別ワークツリーを測るとき・§7） | 本書の位置から上へ探した先 |
| `-Note` | 要約に 1 行そのまま載る自由記述 | なし |
| `-CargoArgs` / `-TestArgs` | `custom` のときの引数（`--` の前 / 後ろ） | なし |
| `-SkipPrebuild` / `-SkipI686Check` | 事前ビルド / i686 検査を省く（推奨しない） | 実施 |

**Git Bash など PowerShell 以外から起動する場合**、`-File` 形式では配列引数（`-CargoArgs`）が
1 個の文字列として渡り `cargo` が `no such subcommand` で落ちる。配列を渡すときは `-Command` を使う:

```bash
# 配列引数がある呼び出しは -Command で（-File だとカンマ区切りが 1 語になる）
pwsh -NoProfile -Command "& '.kiro/specs/areka-P0-test-cage-determinism/verification/repeat-tests.ps1' -Target custom -CargoArgs @('test','-p','wintf','--lib') -Times 4"
# 配列引数が無い呼び出しは -File でよい
pwsh -NoProfile -File ".kiro/specs/areka-P0-test-cage-determinism/verification/repeat-tests.ps1" -Target wintf -Times 4 -Parallel 4
```

終了コードは**全回が「緑」なら 0**、1 回でも緑でなければ 1。

---

## 2. 対象表と期待件数

`repeat-tests.ps1` の `$Targets` に固定してある。

| `-Target` | 実行コマンド | 期待 passed | 由来 |
|---|---|---:|---|
| `workspace` | `cargo test --workspace` | **5865** | 要件 9.1 |
| `seriko` | `cargo test -p areka-seriko --lib` | **200** | 要件 9.2（要件 3.7 の存在主張テスト群を含む） |
| `wait` | `cargo test -p areka --bins -- spine_e2e_sakura_blink_default_off_emits_nothing spine_s4_balloon_free_onboot_completes_without_balloon_face_switch` | **2** | 要件 9.3（有界化した待機 2 テスト） |
| `wintf` | `cargo test -p wintf --lib` | **842** | 要件 7.2・9 の条件（錠を退役させた crate） |
| `kit` | `cargo test -p log-capture-kit` | **79** | 試走用の小さい対象 |

補足:

- `areka` は**ライブラリを持たない**（`cargo test -p areka --lib` は `no library targets found` で落ちる）。
  待機 2 テストは `main.rs` の下にあるので `--bins` で当てる。フィルタは 2 語を並べて渡してよい
  （libtest は複数フィルタを受ける。実測で 2 件だけが走る）。
- **期待件数は不変量ではない**。テストが増減したら次で採り直して表を更新すること:

  ```powershell
  # 1 回だけ走らせて、要約の passed 列を読む（これが正）
  .\...\repeat-tests.ps1 -Target seriko -Times 1 -Tag cal-seriko
  ```

  `-- --list` の行数で数えてはいけない。`--list` は **ignored のテストも数える**ので、
  実際の `passed` と食い違う（`kit` は `--list` で 81・実測 `79 passed; 2 ignored`。
  この食い違いは実際に「件数不一致」として検出された＝`summary.md` の最初の `kit` 節）。

---

## 3. 負荷の定義

**負荷とは「同じ実行体を `-Parallel` 個のプロセスで同時に起動し、各プロセスが既定の並列度で走る」ことである。**
これはタスク 7.2 が実際に回した形（4 プロセス同時 × 9 巡 = 36 回）と同じで、
別の重い処理を横で回すのではなく、**テスト同士を競合させる**。

実測（2026-08-24・本ハーネス）:

| 走行 | 1 回あたりの所要秒 |
|---|---|
| `wintf` 単独（`-Parallel 1`） | 1.6 |
| `wintf` 同時 4（`-Parallel 4`・8 回） | 4.4 〜 5.8（中央値 5.1） |

1 回あたりが 3 倍前後に伸びており、**プロセスが実際に重なっている**ことが数字で確認できる。

`-Parallel` の目安:

- `wintf` / `seriko` / `wait` / `kit`（単一 crate）: **4**。7.2 の実績と同じ。
- `workspace`: **2**。`cargo test --workspace` は単独でも全コアを使うので、
  これ以上増やしてもスラッシングが増えるだけで競合の質は上がらない。

**さらに別種の負荷をかけたい場合**は、PowerShell をもう 1 枚開いて別の対象を同時に走らせ、
両方の `-Note` に「◯◯と同時走行」と書いて重なりを記録に残すこと
（本ハーネスは 1 回の呼び出しで 1 対象しか回さない）。

なお各プロセスの立ち上がりで cargo が
`Blocking waiting for file lock on build directory` を数秒出すことがある。
これは**ビルドディレクトリの錠**で、テストの実行フェーズに入る前に解ける
（実測で 2 プロセスの走行は重なった）。§5 の事前ビルドはこの待ちを最小化するためでもある。

---

## 4. 出力の読み方

| 置き場所 | 中身 | git |
|---|---|---|
| `logs/<札>-r<回>.out.log` / `.err.log` | 各回の生出力 | **非追跡**（`verification/.gitignore` で `logs/` を除外） |
| `logs/<札>-binaries.txt` | 事前ビルドで解決したテスト実行体の刻印（サイズ・更新時刻・パス） | 非追跡 |
| `logs/<札>-prebuild.json` | `cargo test --no-run --message-format=json` の生出力 | 非追跡 |
| `red/<札>-r<回>.out.log` | **緑でなかった回（赤・ビルド失敗・打ち切り）の生ログの複写**（環境固有パスは伏せた形） | **追跡** |
| `summary.md` | 走行 1 回につき 1 節。表・合計・赤の内訳 | **追跡** |

**何を commit するかの判断**: 生ログは試走だけで 2.5MB あり、8.2 の本走行（workspace 10 回＋各 30 回）では
桁がもう 1 つ増える。再生成できるうえリポジトリを太らせるので `logs/` は追跡しない。
一方で**赤は再生成できない**（次に走らせたら緑かもしれない）ので、
赤の回の生ログだけ `red/` へ複写して追跡し、テスト名と失敗内容は `summary.md` 本文にも転記する。
これで `logs/` を丸ごと消しても「何が落ちたか」は残る（要件 9.4）。

### 判定は 6 値。緑と数えるのは「緑」だけ

| 判定 | 条件 | なぜ分けるか |
|---|---|---|
| 打ち切り | 1 回の上限秒に達したのでプロセス木を止めた | ハングした回で無人走行が止まらないようにする（§5-b） |
| ビルド失敗 | `test result:` 行が 1 本も無い | コンパイルが通っていない回を「0 失敗」と読ませない |
| 赤 | `failed` が 1 件以上、または終了コードが 0 でない | — |
| 空振り | `passed` が 0 | **フィルタの綴りを誤ると `0 passed; N filtered out` で終了コード 0 になる**（タスク 7.2 の申し送り ⑴） |
| 件数不一致 | `passed` が期待値と違う | 一部のテストが黙って走らなくなった回を緑にしない |
| 緑 | 上のどれでもない | — |

要約の合計行は必ずこの 6 つを並べて出す:

```
**8 回走らせて 緑 8・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 5.1 / 最小 4.4 / 最大 5.8）
```

### 記録に残るパスは環境固有の接頭辞を伏せる

`summary.md` の実行コマンドと `red/` に入る本文は、`TEMP` / `TMP` / `USERPROFILE` の値を
`<temp>` / `<tmp>` / `<userprofile>` へ置き換えてから書く。OS アカウント名やその場限りの
一時ディレクトリ名を履歴へ入れないため（§6 の較正はリポジトリ外の使い捨てクレートを使うので、
伏せないと環境固有の絶対パスが追跡ファイルに入る）。**無加工の生ログは `logs/`（非追跡）に残る**ので、
診断に元のパスが要るときはそちらを見る。

`summary.md` への追記は**既存本文の改行に合わせる**（CRLF が優勢なら CRLF、それ以外は LF）。
`core.autocrlf` のチェックアウトで CRLF になった要約へ LF を足して混在させないため。
較正済み: 要約を CRLF へ変換した写しへ 1 節追記したところ `CRLF=350 / bareCR=0 / bareLF=0` のまま、
本体（LF）へ追記した場合は `CRLF=0 / bareLF=353` のままだった（§9-3 の 5 値走査で確認）。

### 表の列

`回 / 開始 / 所要秒 / 終了（終了コード）/ passed / failed / ignored / filtered / 実行体 / 判定 / ログ`。

`実行体` は**その回の出力に現れた `test result:` 行の本数**。
表頭の「事前ビルド … テスト実行体 N 本」とは数が違ってよい
（実測 `workspace` は刻印 **72 本**・`test result:` **92 本**。差はドックテストで、
`--no-run` の JSON には実行体として現れず、走行時にだけ生まれる）。

---

## 5. 事前ビルドと刻印、そして待機の有界化

### 5-a. 事前ビルドと実行体の刻印（道具の罠への対処）

走行の前に `cargo test <対象> --no-run --message-format=json` を 1 回だけ回し、

1. 各回の**所要秒にコンパイル時間を混ぜない**（§7 の比較が壊れる）
2. JSON の `executable` から**実際に走る実行体を解決**し、パス・サイズ・更新時刻を
   `logs/<札>-binaries.txt` へ刻む

の 2 つをやる。2 つ目が要るのは本 spec が実測した 2 つの事故に対応するため:

- **`target/` を glob で拾うと前周の古い実行体を測る。** 実体は必ず `--no-run` の
  `--message-format=json` から解決する（本ハーネスはそうしている）。
- **変異を戻しても mtime が据え置きだと cargo が再ビルドを省き、変異版の実体をそのまま再実行する。**
  `git status` は clean・`git hash-object` も一致するので気付けない。
  ソースを書き換えて前後を比べるときは、**復元後に mtime を進めて**から走らせ、
  `logs/<札>-binaries.txt` の更新時刻が**実際に進んだこと**を突き合わせること。
  刻印が前の走行と同じなら、その走行は前のコードを測っている。

`--no-run` が失敗した場合、本ハーネスは走行に入らず
`logs/<札>-prebuild.err.log` を指して止まる（ビルドが通らないまま「0 失敗」を出さない）。

### 5-b. 待機の有界化（`-TimeoutSec`）

本ハーネスの**待機はすべて有界**である。無界の待機は 1 つも置かない
（各回のプロセス待ち・事前ビルド・停止の完了確認の 3 か所）。
要件 4 が「反復回数固定の待機を有界化する」ことそのものなので、検証側にも同じ規律を当てる。
実務上の理由もある——**8.2 は約 100 回を無人で回すので、1 回のハングで全体が止まり記録も残らない**。

上限に達した回は**プロセス木ごと止めて「打ち切り」**として記録する（緑にはしない）。
止めるのは**自分が起こしたプロセスだけ**で、`Process.Kill($true)` で子（`cargo` が起こした
テスト実行体）まで落とす。実測で打ち切り後の残存プロセスは 0 だった。

既定の上限は自動算出:

```
上限秒 = max(120, ceil(単独実測秒 × 同時プロセス数 × 10))
```

- **単独実測秒**は §2 の対象表に持たせた `-Parallel 1` での実測（`workspace` 36.8 / `seriko` 0.4 /
  `wait` 1.7 / `wintf` 1.6 / `kit` 2.4。いずれも 2026-08-24）。
- **同時プロセス数**を掛けるのは、負荷下では 1 回あたりの所要が同時数に比例して伸びるため
  （実測 `wintf` 単独 1.6 秒 → 同時 4 で 4.4〜5.8 秒＝§3）。
- **係数 10** は余裕。上限は性能の合否ではなく**ハングの止め木**なので、
  正常な揺らぎで誤発火しない側へ倒す。`workspace` を `-Parallel 2` で回すと
  上限は 736 秒（期待所要はおよそ 74 秒）。
- 対象表に無い `custom` は所要の見当が付かないので既定 **1800 秒**。短くしたいときは `-TimeoutSec` を渡す。

上限は要約の表頭に**値と算出根拠**が出る（`| 1 回の上限 | 736 秒（自動＝単独実測 36.8 秒 × 同時 2 × 10（下限 120 秒）） |`）。
打ち切った回は「緑でなかった回の内訳」に**上限値・根拠・停止できたかどうか**が並ぶ。
名前の分からない赤を残さないのと同じ規律で、**理由の分からない打ち切りも残さない**。

打ち切りの回の出力は途中までしか無い。**上限が短すぎたのか本当にハングしたのかは生ログの最終行で判断する**
（較正では `running 1 test` で止まっており、`test result:` 行が無い＝テストが返っていないことが読める）。

---

## 6. 較正（この道具が本当に赤を捕まえるか）

「緑が並んだ」は道具が壊れていても出る。本ハーネスは**赤・空振りを意図的に作って**
検出できることを実測してある。結果は `summary.md` の `cal-red` 節・`cal-empty` 節に載っている。

### 6-a. 赤を作る（テスト名と失敗内容が要約に載ることの確認）

`crates/` は 1 行も触らずに済ませるため、**リポジトリの外**に 1 本だけ落ちるテストを持つ
小さなクレートを置いて当てる（`--manifest-path` で指す）。
置き場所は**一時ディレクトリ直下の固定名** `%TEMP%/areka-cage-calibration/` に決めてある
（要約に載るのは `<temp>/areka-cage-calibration/…` に伏せられた形になる＝§4）。

```bash
CAL="$TEMP/areka-cage-calibration"      # PowerShell なら $env:TEMP
mkdir -p "$CAL/redcal/src"
printf '[package]\nname = "redcal"\nversion = "0.0.0"\nedition = "2021"\n\n[lib]\npath = "src/lib.rs"\n' > "$CAL/redcal/Cargo.toml"
cat > "$CAL/redcal/src/lib.rs" <<'RS'
#[cfg(test)]
mod tests {
    #[test]
    fn redcal_this_one_passes() { assert_eq!(1 + 1, 2); }

    #[test]
    fn redcal_this_one_fails_on_purpose() {
        let got = 41;
        assert_eq!(got, 42, "わざと落とす較正用のテスト（実測 {got}）");
    }
}
RS
```

```bash
pwsh -NoProfile -Command "& '.kiro/specs/areka-P0-test-cage-determinism/verification/repeat-tests.ps1' -Target custom -CargoArgs @('test','--manifest-path',\"\$env:TEMP\areka-cage-calibration\redcal\Cargo.toml\",'--lib') -Times 2 -ExpectPassed 1 -Tag 'cal-red'"
```

実測（2026-08-24）: 2 回とも **判定 赤・終了コード 101・`1 passed; 1 failed`**。
要約に失敗したテスト名 `tests::redcal_this_one_fails_on_purpose` と
`assertion left == right failed: わざと落とす較正用のテスト（実測 41）` を含む失敗本文が載り、
生ログが `red/cal-red-r001.out.log`・`red/cal-red-r002.out.log` へ複写された。
**名前の分からない赤は残らない**（要件 9.4）。

失敗本文は 1 テストあたり 60 行で切る（切ったことは要約に明記され、全文は生ログに残る）。

### 6-b. 空振りを作る（終了コード 0 の偽の緑を弾くことの確認）

フィルタの綴りを 1 文字だけ誤らせる（`event` → `evnt`）。

```bash
pwsh -NoProfile -Command "& '.kiro/specs/areka-P0-test-cage-determinism/verification/repeat-tests.ps1' -Target custom -CargoArgs @('test','-p','log-capture-kit','--lib') -TestArgs @('capture_evnt_from_inside_the_window') -Times 1 -ExpectPassed 1 -Tag 'cal-empty'"
```

実測（2026-08-24）: 生ログは `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 26 filtered out`、
**終了コードは 0**。本ハーネスの判定は **空振り**（緑 0・空振り 1）。
終了コードだけを見る道具ならここが緑になる。

### 6-c. 上限が効くことを確かめる（打ち切りの較正）

同じ置き場所に、**決して終わらないテスト 1 本だけ**を持つクレートを置く。

```bash
CAL="$TEMP/areka-cage-calibration"
mkdir -p "$CAL/hangcal/src"
printf '[package]\nname = "hangcal"\nversion = "0.0.0"\nedition = "2021"\n\n[lib]\npath = "src/lib.rs"\n' > "$CAL/hangcal/Cargo.toml"
cat > "$CAL/hangcal/src/lib.rs" <<'RS'
#[cfg(test)]
mod tests {
    /// わざと終わらないテスト（上限の較正用）。
    #[test]
    fn hangcal_this_one_never_finishes() {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
}
RS
```

```bash
pwsh -NoProfile -Command "& '.kiro/specs/areka-P0-test-cage-determinism/verification/repeat-tests.ps1' -Target custom -CargoArgs @('test','--manifest-path',\"\$env:TEMP\areka-cage-calibration\hangcal\Cargo.toml\",'--lib') -Times 1 -TimeoutSec 15 -ExpectPassed 1 -Tag 'cal-hang'"
```

実測（2026-08-24）: **15.0 秒で判定 打ち切り・終了コード -1・`test result:` 行 0 本**。
要約の内訳に「**上限 15 秒に達したので打ち切った**（-TimeoutSec で明示指定）。上限に達したので
プロセス木を停止した。」が出た。走行後の残存プロセス（`cargo` / `rustc` / `hangcal*`）は **0**。
生ログ `red/cal-hang-r001.out.log` は `running 1 test` で途切れており、
テストが返っていないことが読める。**上限が無ければこの回でハーネスは永久に待っていた。**

### 6-d. 件数不一致（意図せず作れた実例）

対象表の `kit` の期待値を `--list` の行数（81）で置いたところ、実測は `79 passed; 2 ignored` で
**3 回とも「件数不一致」**になった（`summary.md` の最初の `kit` 節がその記録）。
期待値を実測の 79 へ是正した後の再走（`trial-kit` 節）は 3 回とも緑。
`--list` は ignored も数えるという事実がこの経路で露出した。

---

## 7. 移行前後の所要時間の比較（R-3）

要件 9.1 の付随項目（`Interest::sometimes` の常態化が全体の所要時間に効くか）。
**同じ道具・同じ引数で 2 本のツリーを測り、要約の中央値を比べる**。

```powershell
# 比較対象（移行前）を別ワークツリーに用意する。既存のツリーは触らない
git worktree add ../areka-before origin/main
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc  # ../areka-before 側でも要る

# 移行前（-Root で走行ルートを差し替える。要約は本 spec 側に貯まる）
.\...\repeat-tests.ps1 -Target workspace -Times 5 -Parallel 1 -Root ..\areka-before -Tag before -ExpectPassed -1

# 移行後（既定のルート＝本ブランチ）
.\...\repeat-tests.ps1 -Target workspace -Times 5 -Parallel 1 -Tag after
```

注意:

- **`-Parallel 1` で測る**。同時プロセス数を上げると所要秒は負荷の関数になり、比較の意味が消える。
- 事前ビルドは所要秒に**含まれない**（表頭に別途出る）。両ツリーとも 1 回目からビルド済みで測ること。
- 移行前ツリーは期待 passed が違う（テスト本数が違う）ので `-ExpectPassed -1` を明示して
  「件数不一致」を抑える。抑えた事実は表の「期待 passed = 指定なし」に残る。
- 本ブランチの単独実測は **36.8 秒 / 5865 passed / 0 failed / 36 ignored / `test result:` 92 本**
  （2026-08-24・HEAD `170edd6a`）。
- **`-Root` はタスク 8.1 では現在のツリーを明示指定する形でしか動かしていない**
  （要約の `走行ルート` 行が渡した値で出ること・`-ExpectPassed -1` が表の期待値を無効化することを確認。
  `summary.md` の `trial-root-expect` 節）。**別ワークツリーを実際に測るのは 8.2 が初回**なので、
  移行前ツリー側でも i686 成果物のビルドが要ることに注意する（無ければハーネスが走行前に止める）。

---

## 8. タスク 8.2 が回す走行（そのまま貼れる形）

```powershell
# 0. 前提
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc

$H = '.\.kiro\specs\areka-P0-test-cage-determinism\verification\repeat-tests.ps1'

# 9.1 ワークスペース全体を負荷下で 10 回
& $H -Target workspace -Times 10 -Parallel 2 -Tag 'r91-workspace' -Note '要件 9.1'

# 9.2 ログの存在を主張する面表 crate を負荷下で 30 回
& $H -Target seriko -Times 30 -Parallel 4 -Tag 'r92-seriko' -Note '要件 9.2'

# 9.3 有界化した待機 2 テストを負荷下で 30 回
& $H -Target wait -Times 30 -Parallel 4 -Tag 'r93-wait' -Note '要件 9.3'

# 7.2 錠を退役させた窓基盤 crate を負荷下で 30 回
& $H -Target wintf -Times 30 -Parallel 4 -Tag 'r72-wintf' -Note '要件 7.2・9 の条件'
```

各走行の後、`summary.md` の合計行が
「緑 = 回数・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0」であることを確認する。
1 つでも緑でない回があれば、その節の「緑でなかった回の内訳」に載っているテスト名と失敗内容を
requirements.md の申し送り台帳へ登記する（要件 9.4。**名前の分からない赤を残さない**）。

**走らせる前に期待件数を採り直すこと**（§2）。テストが増えていると全回が「件数不一致」になり、
100 回まるごとやり直しになる。

上限（§5-b）は既定のままでよい。`workspace` を `-Parallel 2` で回すと 1 回 736 秒が上限で、
期待所要はおよそ 74 秒。ハングした回だけが上限で打ち切られ、**残りの回は続行して記録に残る**。

**タスク 7.2 で既に回した実測**（本ハーネス導入前・記録は実装者とレビュー側の申告のみ）:
`cargo test -p wintf --lib` を 4 プロセス並列 × 9 巡 = **36 回**、全回 `842 passed` / 終了コード 0。
レビュー側が独立に **156 回**（4 並列 × 9 巡の全件＋24 並列 × 5 巡の絞り込み）を回して同じ結果。
8.2 はこれを上の `r72-wintf` として**本ハーネスで採り直し**、リポジトリに残る記録に合流させること。

---

## 9. 手で回すときの注意（道具が黙って嘘をつく罠）

本 spec がこれまでに実測した罠のうち、反復走行に直接効くもの。

1. **`target/` を glob で拾うと前周の古い実行体を測る。** 実体は `cargo test --no-run --message-format=json`
   から解決する（§5。本ハーネスは実施済み）。
2. **変異を戻しても mtime が保たれると cargo は再ビルドを省く。** `git status` も `git hash-object` も
   清浄に見えるのに、走るのは変異版。復元後は mtime を進め、`logs/<札>-binaries.txt` の
   更新時刻が進んだことで再ビルドを確認する（§5）。
3. **改行を数える道具は bare CR も独立に数える形へ較正する。** 「LF の直前が CR か」しか見ない走査は、
   全行が `\r\r\n` でも「CRLF で正常」と報告する（本 spec で実際に起きた）。
   CR 総数・LF 総数・CRLF・bare CR・bare LF の **5 つを別々に**採ること:

   ```powershell
   $b = [IO.File]::ReadAllBytes($path)
   $cr = 0; $lf = 0; $crlf = 0; $bareCR = 0; $bareLF = 0
   for ($i = 0; $i -lt $b.Length; $i++) {
       if ($b[$i] -eq 13) { $cr++; if ($i + 1 -lt $b.Length -and $b[$i + 1] -eq 10) { $crlf++ } else { $bareCR++ } }
       elseif ($b[$i] -eq 10) { $lf++; if ($i -eq 0 -or $b[$i - 1] -ne 13) { $bareLF++ } }
   }
   "CR=$cr LF=$lf CRLF=$crlf bareCR=$bareCR bareLF=$bareLF"
   ```

4. **`git diff | grep -c $'\r'` も MSYS の `grep -c $'\r$'` も改行の実態を判定できない。**
   `crates/*/Cargo.toml` は CRLF なのに MSYS の `sed` / `cat -A` は LF に見せる。
   真偽が付くのは `git ls-files --eol` か上の生バイト走査だけ。
5. **PowerShell の二重引用符ヒアストリング `@"…"@` も bash の二重引用符もバッククォートを食う。**
   逐語を含む本文は必ず**引用符つきヒアドキュメント**（`<<'EOF'`）でファイルへ書き出してから読み込む。
   インラインの `-c "..."` に本文を埋めない
   （本タスクでも `.ps1` をヒアドキュメントで書こうとしてシェルが構文エラーを出し、
   ファイルを直接書く経路へ切り替えた）。
6. **「空を返せば合格」型の検査は、道具が不在でも空になり得る。** 左辺を意図的に 1 件削ると
   赤になる較正を必ず併記する。本ハーネスにおける対応物が §6 の `cal-red` / `cal-empty`
   （赤と空振りを意図的に作って、判定が緑にならないことを実測してある）。

さらに走行そのものについて 2 点:

7. **終了コード 0 は「テストが走った」ことを意味しない。** フィルタの綴り誤りは
   `0 passed; N filtered out` で終了コード 0 になる。必ず `passed` 件数を期待値と突き合わせる（§4・§6-b）。
8. **`-- --list` の行数を期待件数にしない。** ignored を含むので実測の `passed` と食い違う（§2・§6-d）。

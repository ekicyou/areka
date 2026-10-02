# α 完成 受入記録

> 本書は α（M2）の署名の根拠にする実機一周の受入記録である。章立ては M1 の受入記録（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/acceptance-record.md`）の形を写す（要件 3.4）。手順は前半（§3〜§6）に、結果は後半（§7〜§10）に置く。
>
> - 対象仕様: `areka-P0-alpha-release-signoff`
> - 節の決め方: design.md「受入記録 `verification/acceptance-record.md`」の表（§0〜§10 と、それぞれをいつ書くか）
> - 本書は節を順に書き足す。まだ無い節・空欄の欄は「まだ観測していない」を意味し、合格でも不合格でもない。

---

## 0. 書く前に

### 0.1 目視の合否は開発者が決める（要件 3.7）

- AI が行うのは記録の作成だけである（一周の手順の準備・生の記録の採取と引用・本書の作成）。
- 目視の項目の合否は開発者が決める。§7 には、開発者が決めたことと、決めた日と、開発者の返信の転記を書く。AI は目視の項目を単独で合格にしない。

### 0.2 根拠の種別（要件 3.5）

| 種別 | 何を書くか | 書かないもの |
|---|---|---|
| 目視 | 誰が・いつ・どの走行のどの操作の後に・何を見たか | 伝聞・推定（「たぶん出ていた」） |
| 記録の引用 | 生の記録（`run-<走行>.log`）の行を、時刻と `event=` を含めて逐語で写したものと、そのファイルの名前。件数は「件数を数える区間」（§5・§6 で決める）の中で数える | 要約・言い換え・区間の外の行を混ぜた件数 |
| 両方 | 目視と記録の引用を、行を分けてそれぞれ上の形で書く | 2 つを 1 つの欄に混ぜた記述 |

### 0.3 結果の書き方と縮退（要件 3.5）

- 結果は「合格」「不合格」「中断」のどれか 1 つを書く。中断は、その走行で観測できなかった項目に使う（安全弁の自動終了が先に来た・走行を途中で止めた など）。
- 縮退して合格とする（期待の一部が成り立たないが合格とする）ときは、その項目の結果の直下に「縮退:」の行を置き、⑴ 成り立たなかった期待、⑵ その理由、⑶ 開発者の裁定（日付と返信の転記）を書く。開発者の裁定が無いものは縮退の合格にしない。

### 0.4 バグ `balloon-reappear-short-talk` の着地の判定（要件 1.1・5.4）

**判定: 着地した。**

- 確かめたコミット: 本ブランチ `claude/areka-p0-alpha-signoff-f50113` の HEAD `ce6313409d7dac60de4c8d667088572c26d05a34`（main を取り込んだマージ・ぶつかり 0）。
- バグの PR（PR#206）の squash のコミット: `35209987b951a13fd0e5ca606ffe065d1ef2dad3`。`git merge-base --is-ancestor 35209987 HEAD` が成功する＝上の HEAD に含まれる。
- 根拠:
  - `.kiro/specs/completed/areka-P0-balloon-reappear-short-talk/` が在る（`brief.md`・`requirements.md`・`design.md`・`design-validation.md`・`research.md`・`tasks.md`・`spec.json` の 7 本。`spec.json` の `phase` は `completed`）。
  - 台帳 `.kiro/steering/roadmap.md` の行 `completed/balloon-reappear-short-talk` が、段の列 ✅・状態の列「✅ 完了（2026-10-01）」。
  - 直しの実物が上の HEAD に在る: `crates/areka/src/emo2_boot/balloon_visibility.rs` の `decide_content`（scope ごとの消去の回数が変わったら比べる相手を 0 にする）と、`crates/areka-emo-text/src/state.rs` の `TextLayerState::clear_count`。

この判定により、design.md「バグの着地に依る条項」の「着地したとき」の列を使う。

| 本仕様の条項 | 使う扱い |
|---|---|
| 5.4 ⒞ | 説明書に「隠れたバルーンが 1 文字の台詞で現れない」の行を書かない |
| 7.4・7.7 | 完成判定の文書の持ち越しに、このバグの行を作らない |
| 1.1・1.3 | このバグのための組み直しは要らない（取り込んだ後のコミットで zip を組む） |
| 2.1 の項目 7 | 期待どおり（切替の直後の台詞が 1 文字でもバルーンが現れる）を期待にする |

### 0.5 項目 7 に足す確かめ（タスク 1.2 で §5 の項目 7 へ入れる）

バグ `balloon-reappear-short-talk` の側から頼まれた確かめ: 項目 7 でバルーンを切り替えた後に、ゴーストに 1 文字の台詞（例「ん。」）を話させ、バルーンが現れることを目視で確かめる。

---

## 1. 同定（要件 1.1・1.2）

2026-10-02 に組み直した zip で書き直した（§9.1）。前の zip（`areka-alpha-x64-20261001-7f8f4e8.zip`・コミット `7f8f4e87`）の同定は §9.1 に残す。

| 欄 | 値 |
|---|---|
| zip の名前 | `areka-alpha-x64-20261002-8460506.zip`（名前に `-dirty` が付いていない。x64 の 1 種だけで、arm64 版は組んでいない＝要件 1.5） |
| 組んだコマンド | `pwsh -NoProfile -File tools/package-alpha.ps1 -Check`（`-CheckDir` は付けていない＝既定の一時フォルダへ展開） |
| `commit=` | `8460506`（zip の中の `BUILD-INFO.txt`） |
| 完全なコミットの識別子 | `8460506d95054bfc195e884d13f6b469d93e8491`（`package-meta.txt` の `head=`。件名「壊れた DLL を読んでも OS の「正しくないイメージ」の窓を出さない」）。全体テストを回した署名の根拠のコミット（`verification/alpha-completion.md` §1）と同じ |
| `dirty=` | `0`（`BUILD-INFO.txt`）。組む前の `git status --porcelain` も 0 行（`package-meta.txt` の `dirty=0`・`package-alpha.log` の「コミット 8460506・未コミットの変更 0 件」）。組んだ後も 0 行（`package-meta.txt` の `dirty_after=0`） |
| `-Check` の終了コード | `0`（`package-meta.txt` の `exit=0`・`package-alpha.log` の末尾「全段 緑」） |
| `-Check` の日時（日本時間） | 開始 2026-10-02 21:51:45 〜 終了 2026-10-02 22:00:57（`package-meta.txt`）。zip を組んだ時刻は `BUILD-INFO.txt` の `built=2026-10-02T13:00:34Z`（日本時間 22:00:34） |
| zip の sha256 | `308d1e912b5256f4da91a3a18b564524ff659bc1705c2a2628343f651be82a6d`（生の記録の写しと `target\alpha\` の実物で同じ値。写しの隣の `areka-alpha-x64-20261002-8460506.zip.sha256` の値とも同じ） |
| zip の大きさ | 7,562,223 バイト（写しと `target\alpha\` の実物で同じ） |
| 生の記録の置き場 | `C:\home\maz\lap-records\alpha-signoff-20261001\`（zip の写し・`areka-alpha-x64-20261002-8460506.zip.sha256`・`package-alpha.log`・`package-meta.txt`。全体テストの `test-all-r.log`・`test-all-r-meta.txt` も同じ置き場。前の zip の記録は名前を変えて `package-alpha-7f8f4e8.log`・`package-meta-7f8f4e8.txt` として残した） |
| `-Check` の展開先と記録 | 展開先 `C:\Users\maz-o\AppData\Local\Temp\areka-alpha-check-215146`・記録 `C:\Users\maz-o\AppData\Local\Temp\areka-alpha-check-215146-logs\run.log`（と `run.stderr.log`）。リポジトリの外で、一周の根には使わない。一時フォルダの記録は消えうるので、生の記録の置き場へ `check-215146-logs\` として写した（前の zip の回の記録も `check-221148-logs\` として写した） |

`BUILD-INFO.txt` の全文（逐語）:

```
commit=8460506
dirty=0
built=2026-10-02T13:00:34Z
script=tools/package-alpha.ps1 1.0.0
rustflags=-C target-feature=+crt-static
```

### 1.1 `-Check` の判定（`package-alpha.log` から逐語）

```
判定 1?8 すべて合
```

- `?` は記録の採り方で `?` に化けた 1 文字で、`tools/package-alpha.ps1` の判定の出力の行は `判定 1〜8 すべて合`（前の zip の `package-alpha-7f8f4e8.log` ではこの字のまま残っている）。記録のバイト列をそのまま写した。

```
合 番犬で止めていない（自分で終わった）
合 有界で走った（「smoke 自動 close ゲート有効」1 件）
合 終了コード 0（終了コード 0）
合 ゴーストの窓が立った（「本物のゴースト窓を開きました」1 件）
合 SHIORI の接続の失敗が無い（失敗の目印 0 件）
合 会話が始まった（「起動グリーティングを再生起動」1 件（自動終了より前）・全体 1 件）
合 初回のバルーンは同梱（バルーンを決めました event="balloon_resolved" route=Companion dir=C:\Users\maz-o\AppData\Local\Temp\areka-alpha-check-215146\balloon\emo2-kakukaku）
```

### 1.2 zip の実物で確かめたこと（要件 1.4・1.5・4.9・5.4）

生の記録の置き場の zip の写し（`areka-alpha-x64-20261002-8460506.zip`）を、PowerShell の `System.IO.Compression.ZipFile` で読むだけで開いて確かめた（2026-10-02・展開はしていない）。

| 確かめ | 結果 |
|---|---|
| `README.txt` が仕上げた説明書と同じ（判定 8） | zip の `README.txt` と作業木の `dist/README.txt` がバイト列で一致（どちらも 16,575 バイト） |
| 最上位の中身 | `BUILD-INFO.txt`・`LICENSE-MIT`・`README.txt`・`THIRD-PARTY-NOTICES.md`・`areka.exe`・`balloon/`・`ghost/`・`shiori-host32-helper.exe`（項目は全部で 148） |
| 前の zip との違い | 増えた項目は `ghost/emo2/ghost/master/dic/install.pasta`・`ghost/emo2/ghost/master/dic/system.pasta` の 2 つ（146 → 148）で、消えた項目は 0。中身（CRC）が変わった項目は `areka.exe`・`shiori-host32-helper.exe`・`BUILD-INFO.txt`・`ghost/emo2/updates.txt`・`ghost/emo2/ghost/master/updates.txt`・`ghost/emo2/ghost/master/dic/boot.pasta`・`ghost/emo2/ghost/master/scripts/pasta/shiori/event/boot.lua` の 7 つ。`emo2` の側の違いは §9.1 の差し替えの前の確かめの 6 つと合う |
| `ghost/` が `emo2` だけ（要件 1.4） | `ghost/` の下のフォルダは `emo2` の 1 つ |
| `balloon/` の下 | `StayseeBalloon`・`emo2-kakukaku` の 2 つ |
| x64 の 1 種（要件 1.5） | `areka.exe` の PE の機種 `0x8664`（x64）。`shiori-host32-helper.exe` と `ghost/emo2/ghost/master/pasta.dll` は `0x014c`（32bit の SHIORI を読む補助と、えも？？ の SHIORI。zip の種類を増やすものではない）。名前が `.exe`・`.dll` で終わる項目はこの 3 つだけ |
| 記憶の置き場が入っていない（項目 3・12 の前提） | 名前に `/profile/` を含む項目が 0 件 |
| ⒜ の条件（`emo2` が `halt` の台詞を持つか） | `ghost/emo2/ghost/master/dic/boot.pasta` を UTF-8 として読み、`＊起動halt` を含む行が 3 件＝持つ。2.3 の判断（⒜ を書かない）と合う |
| ⒝ の条件（`emo2-kakukaku` の `homeurl`） | `balloon/emo2-kakukaku/descript.txt` を UTF-8 として読み、`homeurl` で始まる行が 0 件（同じ探し方で `name,` の行は 1 件）。2.3 の判断（⒝ を書かない）と合う |

## 2. 機械の構成

一周の前（2026-10-01）に AI が読み取った値。採り直しの前（2026-10-02）に同じ読み方（OS の版は読み方を足した）で読み直し、変わったのは OS の版（25H2・build 26200 → 26H2・build 26300）だけだった。拡大率は一周の途中で変えない（項目 11）。

| 項目 | 値 | 読み方 |
|---|---|---|
| OS | 2026-10-02（採り直しの前に読んだ値）: Microsoft Windows 11 Pro 26H2（10.0.26300・build 26300・UBR 9550）。2026-10-01（前の zip の走行 E1〜A2）: Microsoft Windows 11 Pro 25H2（10.0.26200・build 26200） | `Win32_OperatingSystem` の `Caption`・`Version`・`BuildNumber`、`[Environment]::OSVersion`（`10.0.26300.0`）、レジストリ `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` の `CurrentBuild`・`UBR`・`DisplayVersion` |
| 起動に使うシェル | PowerShell 7.6.6・ふつうの権限（管理者でない）（2026-10-02 も同じ） | `$PSVersionTable`・`WindowsPrincipal.IsInRole(Administrator)`＝False |
| 画面 1（主） | 拡大率 200%（DPI 192）（2026-10-02 も同じ） | DPI 対応にしたスレッドで `GetDpiForMonitor`（実効 DPI） |
| 画面 2 | 拡大率 150%（DPI 144）・主の画面の左（2026-10-02 も同じ） | 同上 |

- 一周は主の画面（200%）で回す。ゴーストの窓を画面 2 へ動かさない（拡大率の違う画面をまたぐ動きは本一周の項目に無い）。

---

## 3. 環境変数（要件 3.2）

design.md「走行の型」の表を逐語で写す。全走行（E1〜A4）で同じ。

| 変数 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT`・`AREKA_PROFILE_DIR` | 未指定（消す） | 根と記憶の置き場を上書きしない（3.2） |
| `AREKA_NO_ALERT` | 未指定（消す） | 項目 1 の告知を出す |
| `RUST_LOG` | `info,areka=debug,areka_kanade=debug,kanade=trace` | 判定に使う分岐の記録（切替・記憶・印・位置の保存と復元）と、`OnBoot` などの参照（`kanade` の `shiori_request` は trace）まで開ける |
| `NO_COLOR` | `1` | 記録に色の制御の文字を混ぜない |
| `AREKA_APP_SMOKE_EXIT_MS` | `1800000`（30 分） | 安全弁。1 回の走行の所要より長い。走行の終わりは開発者のメニューの「終了」（A2 だけは強制終了）で決める。安全弁が先に発火したら、その走行で未観測の項目は「中断」 |

- 表に無い `AREKA_*`・`WINTF_*` の変数はすべて未指定（起動の前に消す。§6.2 の起動の書き方の 1 行で消える）。
- 起動の引数は無し（要件 3.1。引数で起動したプロセスは記憶と起動中の印に触れない）。

## 4. 置き場（要件 3.1）

| 置き場 | 絶対パス | 文字数 | 確かめ |
|---|---|---|---|
| 根 E（項目 1・2） | `C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\E` | 83 | 160 文字以内・`C:\` の直下ではない |
| 根 A（項目 3〜13） | `C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A` | 83 | 160 文字以内・`C:\` の直下ではない |
| 生の記録 | `C:\home\maz\lap-records\alpha-signoff-20261001\` | — | リポジトリの外・ワークツリーの片付けで消えない。準備日 `20261001` はタスク 3.1 で置き場を作ったときに決めた |

- 文字数は PowerShell の `'<パス>'.Length` で数えた（2026-10-01）。上限 160 は `tools/package-alpha.ps1` の展開先の上限 `EXPAND_DIR_MAX_CHARS` と同じ値。
- 根の下でいちばん長くなる見込みのパスは、項目 4 で入る `claudia` の `ghost\claudia\ghost\master\dic\system\aya_lilith\_loading_order.txt` で、根 A と合わせて 151 文字（検体の中の最長の項目から数えた見込み）。
- 根は `target` の下にあり、`target` は `.gitignore` の 1 行目で無視されるので、一周で根に書かれるもの（記憶・印・入れたゴースト）は `git status` に出ない。
- ワークツリーの場所が長すぎるときの次善は `C:\tmp\alpha-lap\<E|A>`（設計の「走行の型」）。今回は使わない。
- 生の記録の置き場に置くもの: zip の写しと `areka-alpha-x64-20261001-7f8f4e8.zip.sha256`・`package-alpha.log`・`package-meta.txt`（3.2・今は名前を変えて `package-alpha-7f8f4e8.log`・`package-meta-7f8f4e8.txt`）・`test-all.log`・`test-all-meta.txt`・`cargo-deny.txt`（3.1）・`runs.txt`・`run-E1.log`〜`run-A4.log` と各 `.err.log`（4.x）。
- 2026-10-02 の組み直し（§9.1）で足したもの: 新しい zip の写しと `areka-alpha-x64-20261002-8460506.zip.sha256`・`package-alpha.log`・`package-meta.txt`（前の回のものは `package-alpha-7f8f4e8.log`・`package-meta-7f8f4e8.txt` へ名前を変えた）・`check-215146-logs\`・`check-221148-logs\`（`-Check` の起動の記録の写し）・`test-all-r.log`・`test-all-r-meta.txt` と `test-all-r-try1`〜`try4` の `.log`・`-meta.txt`・`badimage-red-1717d29f.log`・`badimage-green-worktree.log`（§9.1 の窓の直す前と後の 1 本の走行）・`run-E1r.log` 以降の採り直しの走行の記録と `E1r-alert.png`。

---

## 5. 検証項目表（要件 2.1〜2.5）

「操作」「期待」は要件 2.1・2.2・2.3 の字面のまま（`→` で続く文は、操作の部分と期待の部分に分けて写した）。「証跡の取り方」「期待の裏付け」は design.md「検証項目表」から、タスク 1.1 で取り込んだ後の HEAD（`ce631340`）で引き直した値を写した。表の中の「N 件」「0 件」は、断りの無いかぎり §5.2 の区間での数である。

| 項目 | 走行 | 操作 | 期待 | 証跡の取り方 | 期待の裏付け |
|---|---|---|---|---|---|
| 1 空の根の告知 | E1 | zip を展開して `ghost\` の中を空にした根で `areka.exe` を起動する | 「ゴーストが見つかりません。」と、置く場所の絶対パスと置くものの形を示す告知が出て、OK で終わる。 | 目視（3 行の本文・置く場所が `<根 E>\ghost` の絶対パス・OK で終わる）＋記録（`event="alert"` の 1 行） | `alert.rs:104-114`・`:60`・`:163-168` |
| 2 告知のとおりに置く | E2 | 項目 1 の告知が示す場所に、展開したゴーストのフォルダ（`konnoyayame`＝手で置き、zip には入れない）を置いて起動し直す | そのゴーストが立って挨拶する。あわせて、目の周りに四角い地色が出ないこと・挨拶の字形が化けないこと・絵の中で左上の画素と同じ色の画素の場所のクリックが背後の窓へ抜けることを目視で見る。 | 目視（`konnoyayame` の挨拶・目の周りの地色・字形・抜き色の場所のクリックが背後へ抜ける）＋記録（ゴーストの決定と挨拶の行） | 完了 `shell-implicit-surface`・`keycolor-clickthrough-coverage`（roadmap の目視 5 件の節） |
| 3 初回の起動 | A1 | zip をそのまま展開した根で起動する | えも？？（`emo2`）が同梱のバルーン `emo2-kakukaku` で立って挨拶する。あわせて、絵の透明な場所のクリックが背後の窓へ抜けることを目視で見る。 | 記録（`balloon_resolved` の `route=Companion`・`\balloon\emo2-kakukaku`・`OnFirstBoot`）＋目視（挨拶・透明な場所のクリックが抜ける・項目 11） | `boot_resolve.rs:199-202` の段 3（同梱）・`package-alpha.ps1:76-78` |
| 4 2 体目・3 体目を入れる | A2 | ⒜ メニューの「インストール…」で里々の標準テンプレート `R_POST_and_KOMAINU.nar` を選ぶ ⒝ ゴーストの窓へ `claudia.nar` をエクスプローラから落とす | どちらもインストールの台詞が流れ、表示中のゴーストのまま切り替わらない。利用条件の画面が出たときは「はい」で進める。 | 記録（`install_done` が 2 件・`file_drop_received` が 1 件・切替の行が 0 件）＋目視（インストールの台詞・えも？？ のまま） | 完了 `ghost-install`（要件 12 裁定 5＝入れた後に切り替えない）・`crates/areka/src/input_events/file_drop.rs` の `file_drop_received` |
| 5 メニューでゴーストを替えて戻る | A2 | メニューの「ゴースト」の一覧（を開く）→ `R_POST_and_KOMAINU` を選ぶ → `claudia` を経て えも？？ へ戻る | 一覧に 3 体が並ぶ。送り出しの台詞の後に切り替わり、挨拶し、絵が出る（ファイル名の慣習で建つ面）。挨拶の字形が化けないことと、絵の中で左上の画素と同じ色の画素の場所のクリックが背後の窓へ抜けることを目視で見る。 | 目視（一覧に 3 体・送り出しの台詞・挨拶・絵・字形・抜き色のクリック）＋記録（`ghost_switch_done` が 3 件） | 完了 `ghost-shell-balloon-switch`・`menu/ghost_frame.rs:45` |
| 6 シェルの切り替え | A3 | 2 つ目のシェルを持つゴースト（一周の根の中で手で作る・zip とリポジトリの検体には入れない）で、メニューの「シェル」から 2 つ目を選び、元へ戻す | どちらも絵が替わり、ゴーストは降りない。 | 目視（絵が替わる・降りない）＋記録（`skin_switch_done`・`OnShellChanged` の送出） | 完了 `shell-balloon-switch`・`frame/switch.rs:464` |
| 7 バルーンの切り替え | A2（付記は A3） | えも？？ でメニューの「バルーン」から `StayseeBalloon` を選び、`emo2-kakukaku` へ戻す | どちらも次の台詞から新しいバルーンで出る。 | 目視（次の台詞から新しいバルーン）＋記録（`skin_switch_done`・`OnBalloonChange`）。**付記（§0.5・バグの側から頼まれた確かめ）**: バルーンを切り替えた後に、ゴーストに 1 文字の台詞を話させ、バルーンが現れることを目視で確かめる。えも？？ の辞書には切替の後に 1 文字だけを話す台詞が無いので、§6.3 の走行の間の準備 ⑶ で根 A の `R_POST_and_KOMAINU` の辞書の写しに 1 文字の台詞を足し、A3 の項目 10 の準備で既定でないバルーンを選んだ直後に見る | 完了 `shell-balloon-switch`・`frame/switch.rs:507`。付記は完了 `balloon-reappear-short-talk`（§0.4 の着地の判定）・`crates/areka/src/emo2_boot/balloon_visibility.rs` の `decide_content` |
| 8 ネットワーク更新 | A3 | えも？？ でメニューの「ネットワーク更新」を選ぶ（更新先は `emo2` の配布サイト。zip の `emo2` と配布サイトの間に差分が無いときは、一周の根の中の `emo2` のファイルを 1 つ手で変えて差分を 1 件以上作ってよい）→ 続けてもう 1 度選ぶ | 進捗の台詞の後にいったん引っ込んで同じゴーストが戻り、最初に `OnUpdateComplete`（更新成功）、続けて `OnUpdateResult` が届く（`OnGhostChanged`・`OnBoot` は送らない）。2 回目は差分が無く引っ込まずに `OnUpdateComplete`（更新無し）→ `OnUpdateResult` で終わる。 | 記録（1 回目＝`OnUpdateComplete` の後に `OnUpdateResult`・`OnGhostChanged`／`OnBoot` が 0 件／2 回目＝読み直し無しで更新無し → `OnUpdateResult`）＋目視（進捗の台詞・いったん引っ込んで戻る）。手で変えたファイルの名前と、変える前・変えた後・更新の後の md5（更新の後は変える前に戻る） | 完了 `network-update` の `signoff.md`・`schedule/boot.rs:244-252`（読み直しの起動の根） |
| 9 終了 | A3 の終わり | メニューの「終了」を選ぶ | 別れの台詞の後にプロセスが終わり、終了コードが 0 で、記録に ERROR が 0 件（要件 3.6 の除外を除く）で、起動中の印が消える。 | 記録（`app_exit`・`session_mark_cleared`・A3 の ERROR が除外の後 0 件）＋終了コード 0（`runs.txt`）＋目視（別れの台詞） | `app_exit.rs:107`・`boot_resolve.rs` の `clear_session_mark`（`session_mark_cleared`） |
| 10 前回の状態の復元 | A4 | 項目 9 の後、引数なしで起動し直す | 前回のゴースト・シェル・バルーン・窓の位置で立つ。 | 記録（`session_mark_found` が 0 件・起こしたゴーストとシェルとバルーンの決定の行・`merge_scope restore` の `saved_win_*` が A3 の「char DragEnd 保存」の `saved_*` と同じ）＋目視 | `boot_resolve.rs:253-279`（記憶の直読み）・`placement/persist.rs:422-434` |
| 11 表示の拡大率 | A1〜A4 | 画面の拡大率を 100% 以外にした機械で項目 3〜10 を回す | 絵・当たり判定・窓の大きさ・バルーンの位置が崩れない。 | 目視（項目 3〜10 のたびに）＋§2 の拡大率 | `.kiro/steering/roadmap.md` の「DPI 追従が基本設計」の行・M1 の項目 18 |
| 12 初回だけの位置合わせ | A1→A2 | 新しい根で 1 回目の起動（えも？？ の初回の台詞が相方の窓を横へずらす）を窓を掴まずに終え、引数なしで 2 回目を起動する | 初回のずらしは繰り返されず、相方は既定の配置で立つ（台詞でずらした位置は保存しない・保存するのは掴んで離した位置だけ。M1 の許容の裁定を開発者が 2026-10-01 に確定した）。これで M1 の完成宣言の「持ち越した事項」の 1 行目を閉じる。 | 記録（A1 は `OnFirstBoot`、A2 は `OnBoot`。A2 の相方〔scope 1〕の `merge_scope restore` が `saved_win_x=None`）＋目視（初回のずらしが繰り返されず既定の配置） | `drag_follow.rs:217-226`（保存は掴んで離したときだけ）・`prop_sink.rs:231-255`・`schedule/boot.rs:233-235` |
| 13 強制終了の次の起動 | A2→A3 | 定常のゴーストを強制終了する → 引数なしで起動し直す | 前回のゴーストではなく えも？？ で立ち、`OnBoot` の参照に `halt` と落ちたゴーストの名前が載る。台詞で落ちたゴーストのことを話すかの目視は、`halt` の台詞を持つ `emo2` が zip に入っているときだけ行い、入っていなければ記録の確認だけを行ってその旨を書く。 | 記録（A2 の印は消えない＝`session_mark_cleared` が 0 件／期待する名前は A2 の最後の `session_mark_steady` の `ghost=` の値を逐語で写したもの〔印の値は `descript.txt` の `name` で、フォルダ名 `claudia` ではない〕。A3 の `session_mark_found` の `ghost=` と、`OnBoot` の `references` の 8 番目がその値と同じ・7 番目が `halt`）＋目視（えも？？ で立ち、落ちたゴーストのことを話す。6.4） | `format.rs:245`・`boot_config.rs:174-183`・`main.rs:429-431`・`events.rs:273-279`・`boot.rs:231-235`・新しい `emo2` の `ghost/master/scripts/pasta/shiori/event/boot.lua` の OnBoot の受け口（Reference6 が `halt` かつ Reference7 が空でないとき「起動halt」） |
| 付随 `\![open,readme]` | A3（項目 6 の中） | ゴーストの台本から `\![open,readme]` を出す（検体の辞書の写しに 1 行足してよい・リポジトリの検体と zip は変えない） | 説明書が開く。 | 目視（説明書が開く）＋記録（`readme_opened`） | `consumer_ledger.rs:325`・`readme.rs:193` |
| 付随 左クリックの後の右クリック | A2 | 左クリックを 1 度した後に右クリックする | メニューが出る。 | 目視（メニューが出る）＋記録（`menu_shown`） | 完了 `wintf-drag-state-rest-contract` |

### 5.1 期待の書き方の決まり（要件 2.5）

- 期待は、上の「期待の裏付け」の列（ソースの定義行か完了 spec の記録）で確かめた振る舞いだけを書く。裏付けの無い期待は書かない。
- 項目 7 の期待は、§0.4 の着地の判定（着地した）により「切替の直後の台詞が 1 文字でもバルーンが現れる」を含む。
- 項目 13 の目視は、zip の `emo2` が `halt` の台詞を持つとき（`emo2.nar` を差し替えたとき＝タスク 2.1）だけ行う。差し替えを取りやめたとき（要件 6.5 の枝）は記録の確認だけになる旨を、タスク 3.1 でここに書き足す。
- 項目 13 の期待する名前は、手で `claudia` と書かない。A2 の記録の**最後の** `session_mark_steady` の行の `ghost=` の値を、1 文字も変えずに写す（印に書かれるのは `descript.txt` の `name` で、フォルダ名ではない。設計の検証の直すべき点 2）。

### 5.2 件数を数える区間（要件 2.1・3.5）

A2 と A3 には同じ記録を出す操作が複数入る（例: A2 の `ghost_switch_done` は項目 5 の 3 件と項目 13 の前の 1 件で計 4 件・A3 の `OnBoot` は項目 13 の 1 件）。証跡の件数は走行全体ではなく、**その項目の操作の始まりから次の項目の操作の始まりまでの区間**で数える。区間の境目は、開発者が各項目の操作の直前に `runs.txt` へ 1 行（`<走行> item=<番号> start=<時刻>`）を足して決める。§7 には項目ごとに区間の始まりと終わりの時刻を書き、件数はその区間の行だけを数える。表の「N 件」「0 件」はすべてこの区間での数である。

細かな決め:

- 区間の終わりは、同じ走行の次の `item=` の行の時刻、それが無ければその走行の終わりの行（`exit=` か `killed`）の時刻。`pid=` の行は境目にしない。
- `<番号>` は項目の番号のほか、次の 4 つを使う: `click`（付随の左クリックの後の右クリック）・`13a`（A2 の終わりの、`claudia` へ替えてから強制終了まで）・`switch`（A3 の、項目 8 の後に `R_POST_and_KOMAINU` へ替える操作。どの項目の数にも入れない）・`prep10`（A3 の項目 10 の準備。項目 7 の付記はこの区間で見る）。付随の `\![open,readme]` は項目 6 の区間（`item=6`）の中で数える。
- `<時刻>` は UTC で書く（`(Get-Date).ToUniversalTime().ToString('o')`、末尾が `Z`）。areka の記録の行の時刻が UTC（末尾 `Z`）なので、同じ時計で比べるため。§7 には日本時間（UTC＋9 時間）を添える。
- 区間ではなく走行全体で数えるのは、設計の表が走行を名指ししている次の 2 つだけ: 項目 9 の「A3 の ERROR が除外の後 0 件」（A3 全体）／項目 13 の「A2 の `session_mark_cleared` が 0 件」（A2 全体）。項目 13 の期待する名前を写す「A2 の最後の `session_mark_steady`」も A2 全体の最後の 1 行。
- 項目 11 は区間を持たない（A1〜A4 の目視をまとめる）。

### 5.3 1 分以内に観測できる単位への切り方（要件 2.4）

各行の 1 つの単位は、操作 1〜3 回とその後の台詞・絵の観測で終わる。1 分を超えて待つ試行は置かない。

| 項目 | 区間の行 | 観測の単位（各 1 分以内） |
|---|---|---|
| 1 | `E1 item=1` | ⑴ 起動して告知を読む ⑵ OK を押して終わるのを見る |
| 2 | `E2 item=2` | ⑴ 起動して挨拶・字形・目の周りを見る ⑵ 抜き色の場所を 1 度クリックして背後へ抜けるのを見る |
| 3 | `A1 item=3` | ⑴ 起動して挨拶とバルーンを見る ⑵ 透明な場所を 1 度クリックして背後へ抜けるのを見る |
| 12 | `A1 item=3`（1 回目）・`A2 item=12`（2 回目） | ⑴ A1 を窓を掴まずに「終了」 ⑵ A2 を起動して相方の配置を見る |
| 4 | `A2 item=4` | ⑴ 「インストール…」で `R_POST_and_KOMAINU.nar` を選び台詞を見る ⑵ `claudia.nar` を窓へ落とし台詞を見る |
| 5 | `A2 item=5` | ⑴ 一覧を開いて 3 体を見る ⑵ `R_POST_and_KOMAINU` を選び送り出し・挨拶・絵・抜き色を見る ⑶ `claudia` を選ぶ ⑷ えも？？ を選ぶ |
| 7 | `A2 item=7` | ⑴ `StayseeBalloon` を選び次の台詞を見る ⑵ `emo2-kakukaku` を選び次の台詞を見る |
| 付随 右クリック | `A2 item=click` | ⑴ 左クリックを 1 度 ⑵ 右クリックでメニューが出るのを見る（メニューは閉じる） |
| 13 前半 | `A2 item=13a` | ⑴ `claudia` を選ぶ ⑵ `session_mark_steady` の行を確かめ 5 秒置く ⑶ 強制終了（§6.4） |
| 13 後半 | `A3 item=13` | ⑴ 起動してえも？？ で立つのを見る ⑵ 落ちたゴーストのことを話す台詞を見る |
| 8 | `A3 item=8` | ⑴ 1 ファイルを変えて md5 を控える ⑵ 1 回目の「ネットワーク更新」 ⑶ 2 回目の「ネットワーク更新」 ⑷ 変えたファイルの md5 を採る |
| 6・付随 説明書 | `A3 item=6` | ⑴ 「シェル」で `second` を選び絵と説明書を見る ⑵ `master` を選び絵と説明書を見る |
| 7 の付記 | `A3 item=prep10` | 既定でないバルーンを選んだ直後の 1 文字の台詞でバルーンが現れるのを見る |
| 9 | `A3 item=9` | ⑴ 「終了」を選び別れの台詞を見る ⑵ 終了コードを書く |
| 10 | `A4 item=10` | ⑴ 起動してゴースト・シェル・バルーン・位置を見る ⑵ 「終了」 |

---

## 6. 走行の手順（要件 2.1〜2.3・3.1〜3.3・3.6・3.9）

design.md「走行の手順」の表の順（準備 → E1 → E2 → A1 → A2 → 走行の間 → A3 → A4）に書く。操作・目視の合否・強制終了の実行は開発者、根の展開・起動の書き方の用意・記録の採取と引用・本書への転記は AI が行う（tasks.md の 4 の分担）。

### 6.1 守ること（一周を通して）

- **項目 12 を観測し終える（A2 の `item=12` の区間を記録し終える）まで、ゴーストの窓を掴まない。** 位置は掴んで離したときだけ保存されるので、掴むと項目 12 が成り立たない。
- **A1 で項目 3 のクリックは、絵の透明な場所だけを押す。キャラクターの絵の上は押さない**（掴んで離した扱いになると位置が保存され、項目 12 が成り立たない）。メニューは右クリックで開く。
- **根 A の最初の起動（A1）から最後の走行（A4）を終えるまで、`tools/test-all.ps1`・`cargo test`・`tools/package-alpha.ps1` を回さない**（要件 3.3。保存された位置を消すテストと、検体の展開の作り直しを避ける）。準備の `nar-sample-path` も A1 より前に済ませ、一周の間は cargo を回さない。
- 画面の拡大率は一周の前に 100% 以外の 1 つの値に決め（§2）、一周の途中で変えない（項目 11）。項目 3〜10 の目視のたびに「絵・当たり判定・窓の大きさ・バルーンの位置が崩れない」を一緒に見る。
- ふつうの権限（管理者でない）の PowerShell 7 から起動する（管理者で起動すると投げ込みが届かない）。
- 止めてよいプロセスは、その一周で起こして PID を書き留めたものだけ（要件 3.9・§6.4）。
- 各項目の操作の直前に、`runs.txt` へ区間の行を足す（§5.2）。書き方:

```powershell
"$RUN item=<番号> start=$((Get-Date).ToUniversalTime().ToString('o'))" | Add-Content "$REC\runs.txt"
```

### 6.2 起動と終わりの書き方（全走行で同じ・design.md「走行の型」）

根は zip を根のフォルダへ展開して作る（`[IO.Compression.ZipFile]::ExtractToDirectory`）。根のフォルダが既に在るときは展開せず、開発者に尋ねる（項目 3・12 は新しい根が前提）。

```powershell
$ZIP = 'C:\home\maz\lap-records\alpha-signoff-20261001\areka-alpha-x64-20261002-8460506.zip'   # 署名の zip の写し（§1・2026-10-02 に組み直したもの。前の 20261001-7f8f4e8 の zip と取り違えない）
$ROOT = '<根の絶対パス>'          # §4 の根 E か根 A
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::ExtractToDirectory($ZIP, $ROOT)
```

起動（ふつうの権限の PowerShell 7・引数なし。起動用に開いた窓で行い、その窓で `runs.txt` への行も足す）:

```powershell
$ROOT = '<根の絶対パス>'
$REC  = 'C:\home\maz\lap-records\alpha-signoff-20261001'
$RUN  = '<走行名 E1〜A4>'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Get-ChildItem Env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
$env:RUST_LOG = 'info,areka=debug,areka_kanade=debug,kanade=trace'
$env:NO_COLOR = '1'
$env:AREKA_APP_SMOKE_EXIT_MS = '1800000'
$p = Start-Process -FilePath "$ROOT\areka.exe" -WorkingDirectory $ROOT -PassThru `
     -RedirectStandardOutput "$REC\run-$RUN.log" -RedirectStandardError "$REC\run-$RUN.err.log"
"$RUN pid=$($p.Id) start=$(Get-Date -Format o)" | Add-Content "$REC\runs.txt"
```

- 起動の直後に `$null = $p.Handle` を 1 度実行しておく（`Start-Process -PassThru` で得たプロセスは、終わる前に一度ハンドルを取っておかないと、終わった後の `$p.ExitCode` が空になることがあるため）。
- 起動の前の区間の行（例 `E1 item=1`）は、上の起動の書き方より先に足す。

終わり（E1 は告知の OK、それ以外はメニューの「終了」を選んだ後）: `$p.WaitForExit()` の後の `$p.ExitCode` を `runs.txt` に書く。

```powershell
$p.WaitForExit()
"$RUN exit=$($p.ExitCode) end=$(Get-Date -Format o)" | Add-Content "$REC\runs.txt"
```

### 6.3 走行ごとの中身

| 走行 | 根 | 中身 |
|---|---|---|
| 準備 | — | 生の記録の置き場を新しく作る（タスク 3.1 で作ったものを使う）／画面の拡大率を 100% 以外にする／`cargo run -q -p sample-ghost-kit --bin nar-sample-path -- konnoyayame` の `folder=` を根 E の外の作業用に控える（一周の間は回さない） |
| E1 | E（zip を展開し `ghost\` の中を空にする） | 項目 1 |
| E2 | E（控えた `konnoyayame` のフォルダを `ghost\konnoyayame` へ写す） | 項目 2 → メニューの「終了」 |
| A1 | A（zip をそのまま展開） | 項目 3 → 窓を掴まずにメニューの「終了」（項目 12 の 1 回目） |
| A2 | A | 項目 12 の観測 → 項目 4 ⒜（`vendors\sample_ghost\R_POST_and_KOMAINU.nar` を「インストール…」で選ぶ）⒝（`vendors\sample_ghost\claudia.nar` を Explorer からえも？？ の窓へ落とす）→ 項目 5（`R_POST_and_KOMAINU` → `claudia` → えも？？）→ 項目 7（えも？？ で `StayseeBalloon` → `emo2-kakukaku`）→ 付随（左クリックの後の右クリック）→ `claudia` へ替え、`session_mark_steady` の行（`ghost=` が `claudia` の `name`）が記録に出てから 5 秒置く（印の書き込みは書き手への投函で非同期のため）→ 強制終了（項目 13 の前半） |
| 走行の間 | A（areka を止めた状態） | 項目 6 の 2 つ目のシェル: `ghost\R_POST_and_KOMAINU\shell\master\` を `shell\second\` へ写し、写した `descript.txt` の `name,master` を `name,second` に置き換え、写した側の `surface0000.png` と `surface0001.png` を入れ替える（完了 `shell-balloon-switch` の `signoff.md` の作り方）。付随の確認: `ghost\R_POST_and_KOMAINU\ghost\master\dic02_Event.txt` の `＊OnShellChanged` の台詞に `：\![open,readme]` の 1 行を足す（Shift_JIS と CRLF を保つ）。どちらも根 A の写しだけを変え、リポジトリの検体と zip は変えない |
| A3 | A | 項目 13 の観測 → 項目 8（えも？？ のまま・手で 1 ファイルを変えてから 2 回選ぶ）→ `R_POST_and_KOMAINU` へ替える → 項目 6（「シェル」で `second` → `master`。付随の `\![open,readme]` をここで見る）→ 項目 10 の準備（もう 1 度 `second` を選び、「バルーン」で既定でないバルーン〔例 `claudia`〕を選び、本体の窓を掴んで離す）→ 項目 9（メニューの「終了」） |
| A4 | A | 項目 10 → メニューの「終了」 |

走行ごとの細部（上の表を操作の順に開いたもの）:

**E1**
1. 根 E へ zip を展開し、`<根 E>\ghost\` の中のもの（`emo2`）を取り除いて `ghost\` を空にする（`ghost\` のフォルダそのものは残す）。
2. `E1 item=1` の行を足し、§6.2 の書き方で起動する。
3. 告知の本文（3 行・置く場所が `<根 E>\ghost` の絶対パス）を開発者が読み、OK を押す。終わりの行を書く。

**E2**
1. 準備で控えた `konnoyayame` のフォルダを `<根 E>\ghost\konnoyayame` へ写す。
2. `E2 item=2` の行を足して起動し、項目 2 を観測する。メニューの「終了」で終え、終わりの行を書く。

**A1**
1. 根 A へ zip をそのまま展開する。
2. `A1 item=3` の行を足して起動し、項目 3 を観測する（クリックは透明な場所だけ・§6.1）。
3. 窓を掴まずに、右クリックのメニューの「終了」で終える（項目 12 の 1 回目）。終わりの行を書く。

**A2**
1. `A2 item=12` の行を足して起動し、項目 12 を観測する。記録を書き終えるまで窓を掴まない。
2. `A2 item=4` → 項目 4 ⒜⒝。
3. `A2 item=5` → 項目 5。
4. `A2 item=7` → 項目 7。
5. `A2 item=click` → 付随の左クリックの後の右クリック。
6. `A2 item=13a` → メニューの「ゴースト」で `claudia` を選ぶ。記録に `session_mark_steady` の行が出たことを次で確かめ、その行が出てから 5 秒置く。

```powershell
Select-String -Path "$REC\run-$RUN.log" -Pattern 'session_mark_steady' | Select-Object -Last 1
```

7. その行の `ghost=` の値を逐語で控える（`runs.txt` に `A2 expect13 ghost=<値>` の行を足す）。この値が項目 13 の期待する名前になる（§5.1）。
8. §6.4 の強制終了の手順で止める。

**走行の間**（areka を止めた状態で、根 A の写しだけを変える）
1. 2 つ目のシェル（上の表のとおり）。
2. 付随の確認の 1 行（上の表のとおり）。
3. 項目 7 の付記の 1 文字の台詞: `ghost\R_POST_and_KOMAINU\ghost\master\dic02_Event.txt` の末尾に、空の行に続けて `＊OnBalloonChange` と、1 文字だけの台詞の行 `：ん` の 2 行を足す（Shift_JIS と CRLF を保つ。行頭の `：` は同じ辞書のほかの台詞と同じ書き方で、話す文字は「ん」の 1 文字）。えも？？ の辞書には切替の後に 1 文字だけを話す台詞が無く、`R_POST_and_KOMAINU` の辞書は `＊OnBalloonChange` を持たないため、ここで足す。§0.5 の例「ん。」は句点を含めて 2 文字なので、1 文字ちょうどの「ん」にする。**この 1 行は設計の走行の手順に無い足しであり、一周の前（タスク 4.3 の準備）に開発者の了承を得てから行う。** 了承が無いときは足さず、項目 7 の付記は「確かめる手段が無かった」として §7 にそう書く。

**A3**
1. `A3 item=13` の行を足して起動し、項目 13 を観測する（`session_mark_found` の `ghost=` と `OnBoot` の references の 8 番目が `A2 expect13` の値と同じ・7 番目が `halt`・えも？？ が落ちたゴーストのことを話すかを開発者が目視）。
2. `A3 item=8` → 項目 8。手で変えるファイル: 配布サイトの更新の一覧（`emo2` の `updates.txt`）に載っている、読むだけのテキストのファイル 1 つ（例 `readme.txt`）の末尾に 1 行足す。変えた名前と md5 を記録に書く。配布サイトの版が zip の版より進んでいれば差分は 2 件以上になってよい（件数を書く）。md5 は `Get-FileHash -Algorithm MD5` で、変える前・変えた後・1 回目の更新の後の 3 回採る。
3. `A3 item=switch` → メニューの「ゴースト」で `R_POST_and_KOMAINU` を選ぶ。
4. `A3 item=6` → 「シェル」で `second` → `master`。どちらでも説明書が開くこと（付随）を見る。
5. `A3 item=prep10` → もう 1 度 `second` を選ぶ → 「バルーン」で既定でないバルーン（例 `claudia`）を選び、直後の 1 文字の台詞でバルーンが現れるかを見る（項目 7 の付記）→ 本体の窓を 1 度掴んで離す（記録に「char DragEnd 保存」が出る）。
6. `A3 item=9` → メニューの「終了」。終わりの行（終了コード）を書く。

**A4**
1. `A4 item=10` の行を足して起動し、項目 10 を観測する。メニューの「終了」で終え、終わりの行を書く。

### 6.4 強制終了の手順（A2 の終わりだけ・要件 3.9）

止める前に `Get-CimInstance Win32_Process -Filter "ParentProcessId=$($p.Id)"` で `$p` の子（`shiori-host32-helper.exe`）の PID を `runs.txt` に書き、`Stop-Process -Id $p.Id -Force` で `$p` だけを止める。数秒後に、書き留めた子の PID がまだ在るときに限り、その PID だけを止めて記録に書く。名前で探して止めることはしない。

```powershell
# ⑴ 止める前に、$p の子の PID（と作られた時刻）を書き留める
$kids = @(Get-CimInstance Win32_Process -Filter "ParentProcessId=$($p.Id)")
"$RUN children=$(($kids | ForEach-Object { "$($_.ProcessId):$($_.Name)" }) -join ',') at=$(Get-Date -Format o)" | Add-Content "$REC\runs.txt"
# ⑵ $p だけを止める
Stop-Process -Id $p.Id -Force
"$RUN killed pid=$($p.Id) end=$(Get-Date -Format o)" | Add-Content "$REC\runs.txt"
# ⑶ 数秒後、書き留めた子が（同じ PID・同じ作られた時刻のまま）まだ在るときだけ、その PID を止める
Start-Sleep -Seconds 5
foreach ($k in $kids) {
    $now = Get-CimInstance Win32_Process -Filter "ProcessId=$($k.ProcessId)"
    if ($now -and $now.CreationDate -eq $k.CreationDate) {
        Stop-Process -Id $k.ProcessId -Force
        "$RUN child-left pid=$($k.ProcessId) stopped at=$(Get-Date -Format o)" | Add-Content "$REC\runs.txt"
    }
}
```

- ⑶ の「作られた時刻が同じ」の確かめは、書き留めた PID が別のプロセスに使い回されていないことを確かめるためで、名前で探すことではない。
- 子が残って止めたときは（`child-left` の行があるときは）、§8 に登記する（design.md の Error Handling）。
- 強制終了は A2 の終わりの 1 回だけ。ほかの走行は §6.2 の終わりの書き方で終える。

### 6.5 走行が思いどおりに進まないとき（design.md の Error Handling）

- 走行の途中で項目が成り立たない: その走行を止め、観測できたところまでを §7 に書き、未観測の項目は「中断」。採り直しは新しい記録の名前で（前の記録は消さない）。
- 安全弁の自動終了（30 分）が先に発火した: その走行の未観測の項目は「中断」。安全弁の終わりはきれいな終わりに数えられる（`main.rs` の `ExitOrigin::Smoke`）ので、A2 で発火したら項目 13 の前半はやり直す。
- 項目が不合格: 欠陥を §8 に登記し、「本仕様の中で直す」か「別の spec を起票する」かを開発者が決める（要件 3.8）。

### 6.6 ERROR と WARN の数から外すもの（要件 3.6）

記録の ERROR と WARN の件数で合否を決めるとき（項目 9 の「A3 の ERROR が除外の後 0 件」など）、次のものは理由を書いて数から外す。除外するたびに、その行を時刻つきで逐語に写し、理由とともに §7 に書く。

| 例 | 理由 |
|---|---|
| E1 の `event="alert"`（ERROR） | 項目 1 の期待どおりの告知（ゴーストが無い根で出るもの） |
| `R_POST_and_KOMAINU` の辞書が持たない面を呼ぶ合成の失敗 | 検体の中身から出るもの（本体の欠陥ではない） |
| 先行 spec の記録で判定の外と決まっている既存の警告 | 判定の外と既に決まっているもの（どの spec のどの記録で決まったかを添える） |

上の例に当てはまらない ERROR・WARN は外さない。外すかどうか迷うものは、外さずに開発者に尋ねる。

### 6.7 利用条件の画面（項目 4）

項目 4 の「利用条件の画面が出たときは『はい』で進める」は、この一周では踏まれない。利用条件のファイル（`crates/areka/src/install/terms.rs` の `TERMS_FILES`＝`terms.txt`・`terms.md`）を、`R_POST_and_KOMAINU.nar` も `claudia.nar` も持たないため（2026-10-01 に 2 つの `.nar` の中身の一覧で `terms.` に当たる項目が 0 件であることを確かめた）。§7 の項目 4 に、画面が出なかったことと、この理由を書く。説明書の利用条件の記述（要件 4.3）は、`TERMS_FILES` と完了 `ghost-install` の `signoff.md`（手で作った `konnoyayame-terms.nar` で画面を確かめた回）で裏付ける（§10）。

---

## 7. 項目ごとの結果

2026-10-02 に組み直した zip（§1・`areka-alpha-x64-20261002-8460506.zip`）で、根 E・根 A を作り直して全項目を採り直した走行（E1r・E2r・A1r・A2r・A3r・A4r）の結果を、すべての項目の判定に使う。前の zip（`areka-alpha-x64-20261001-7f8f4e8.zip`）で回した走行は判定に使わない。E1・E2・A1 の結果は §9.1 の「前の zip での走行（判定に使わない）」に、A2 の経過は §9.1 の「起きたこと」⑴⑵ と「前の zip での結果」に残した。

- 時刻は記録の UTC（末尾 Z）に日本時間を添える。生の記録は §4 の置き場（`runs.txt`・`run-<走行>.log`・`run-<走行>.err.log`）。
- `runs.txt` の `item=` の行は UTC、`pid=`・`exit=`・`killed`・`children=`・`between`・`A3r item8 …` の行は日本時間（+09:00）。区間の終わりが日本時間の行のときは、9 時間を引いて UTC へ直してから比べた。
- 件数の数え方: 生の記録の各行は時刻で始まる（時刻で始まらない行は E1r〜A4r のどの記録にも 0 行）。行頭の時刻が区間の始まり以上・終わり未満の行だけを数えた。ERROR・WARN は、時刻の次の欄（記録の重さ）が `ERROR`・`WARN` の行を数えた（本文に `ERROR` の字を含むだけの行は数えない）。

### 7.0 結果の一覧

| 項目 | 走行 | 結果 | 開発者の返信（逐語） |
|---|---|---|---|
| 1 空の根の告知 | E1r | **合格** | 「E1r はOK」 |
| 2 告知のとおりに置く | E2r | **合格** | 「E2r はOK」 |
| 3 初回の起動 | A1r | **合格** | 「OK」 |
| 12 初回だけの位置合わせ | A1r → A2r | **合格** | 「項目 12 はOK、項目 4 は縮退合格で、emo2 に申し送って。あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」 |
| 4 2 体目・3 体目を入れる | A2r | **合格（縮退）** | 同上 |
| 5 メニューでゴーストを替えて戻る | A2r | **合格** | 「項目 5 はOK」 |
| 7 バルーンの切り替え | A2r（付記は A3r） | **合格** | 「項目 7 はOK」。付記は「項目 10 の準備は済み、「ん」でバルーンが出ました」 |
| 付随 左クリックの後の右クリック | A2r | **合格** | 「付随の右クリックはOK」 |
| 13 強制終了の次の起動 | A2r → A3r | **合格** | 前半「claudia が立ちました」「強制終了しました」・後半「項目 13 はOK、項目 8 も済みました」 |
| 8 ネットワーク更新 | A3r | **合格** | 「項目 8 はOK、emo2 に申し送って」 |
| 6 シェルの切り替え | A3r | **合格** | 「項目 6 はOK」 |
| 付随 `\![open,readme]` | A3r（項目 6 の中） | **合格** | 「項目 6 はOK」 |
| 9 終了 | A3r の終わり | **合格** | 「項目 9 は済みました」「項目 9 はOK、A4r 起動し、終了した」 |
| 10 前回の状態の復元 | A4r | **合格** | 「項目 10 はOK、項目 11 もOK」 |
| 11 表示の拡大率 | A1r〜A4r | **合格** | 同上 |

- 「不合格」は 0 件、「中断」は 0 件。縮退の合格は項目 4 の 1 件（下の項目 4 の「縮退:」の行）。

### 7.1 WARN と ERROR の除外の理由（§6.6）

下の各項目で、数から外した行の後ろに次の記号を付けた。

- ⓔ（ERROR）: E1r の `event="alert"`。§6.6 の 1 つ目の例（項目 1 の期待どおりの告知）。
- ⓐ（WARN）「balloon: 面がデフォルト定義側（本体側）の系列へ縮退した」: 既定バルーン `StayseeBalloon` などが相方用の面 2・3 を持たず、本体側の面へ落ちたことの観測。`crates/areka-emo-present/src/balloon.rs` の `resolve_balloon_faces` の doc が「失敗ではなく正典準拠のフォールバック動作の観測」と定める。§6.6 の 3 つ目の例: 完了 `areka-P0-default-balloon-bundle` の `verification/signoff-record.md` §3.3（と §2 の表の 20 行目）（相方側の面 2・3 が本体側の `balloons2/3.png` へ縮退し記録 2 件＝期待どおり）・完了 `areka-P0-shell-balloon-switch` の `signoff.md` の「既知の制限・観察」（「バルーンの面が既定の系列へ縮退した」を本仕様の範囲外の既存のものとした）。
- ⓑ（WARN）「bake: element が全透明（α=0）でトリム後 0 寸です」（`purple/a/null.png`）: えも？？ のシェルの素材に全透明の部品があることの観測。§6.6 の 3 つ目の例: 完了 `areka-P0-shell-balloon-switch` の `signoff.md` の「既知の制限・観察」（「全透明の部品」を範囲外の既存のものとした）・完了 `areka-P0-ghost-shell-balloon-switch` の `signoff.md`（WARN は検体と既定バルーンに由来するもの＝emo2 の全透明の要素 `purple/a/null.png`）。
- ⓒ（WARN）「折返し基準が描画範囲の外に解決された」: バルーン定義の粗さの警告。§6.6 の 3 つ目の例: 完了 `areka-P0-emo-text-line-height-canon` の `verification/handoff.md`（このバルーンで警告が 1 回出るのは正常な記録）・完了 `areka-P0-shell-balloon-switch` の `signoff.md` の「既知の制限・観察」（「折り返しの基準が描画範囲の外」を範囲外の既存のものとした）。
- ⓓ（WARN）`update_target_skipped reason="no_homeurl" kind="shell"`: シェルに更新先が無いので飛ばしたことの記録。§6.6 の 3 つ目の例: 完了 `areka-P0-network-update` の `signoff.md` の 3 回目 ⑶（「シェルは `update_target_skipped reason=no_homeurl` で飛ばし、総括にはゴーストとバルーンだけ」を合格の振る舞いとして記録）。
- **外さないもの**: `update_target_skipped reason="no_homeurl" kind="balloon"`（項目 8 の 2 件）。§6.6 のどの例にも当たらない（バルーンの更新先が無いときに飛ばすことを判定の外と決めた先行 spec の記録が見つからない）ので数に残す。`emo2-kakukaku` に `homeurl` が無いことは §1.2 の確かめ（`homeurl` で始まる行が 0 件）と合う。項目 8 の期待は WARN の数を定めないので、判定は変わらない。外すかを開発者に尋ね（§6.6「外すかどうか迷うものは、外さずに開発者に尋ねる」）、答え（2026-10-02・逐語）「どちらも A で進めて」（A＝外さずに数に残し、`homeurl` の無いバルーンを飛ばした期待どおりの知らせと注記する）。

### 項目 1 空の根の告知（E1r）

- 区間: `E1r item=1 start=2026-10-02T13:05:36.8227598Z`（22:05:36 JST）〜 `E1r exit=1 end=2026-10-02T22:05:50.7022677+09:00`（13:05:50.70Z）。pid 27664（`E1r pid=27664 start=2026-10-02T22:05:37.2626192+09:00`）。根 E は新しい zip を展開して `ghost\` の中を空にした。
- 記録（`run-E1r.log` から逐語）:
  - `2026-10-02T13:05:37.284499Z  INFO areka::boot_config: ベースウェアの根を決めました event="root_resolved" root=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\E source=ExeDir`
  - `2026-10-02T13:05:37.287636Z ERROR areka::alert: [alert] 利用者へ告げます event="alert" scene=GhostMissing { ghost_store: "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\E\\ghost", argv: None } title="areka を起動できません" body="ゴーストが見つかりません。\n置く場所: C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\E\\ghost\n置くもの: ghost\\master\\descript.txt を持つゴーストのフォルダ" suppressed=false`
- 目視: 告知の窓（題「areka を起動できません」・本文 3 行〔「ゴーストが見つかりません。」・置く場所 `…\target\alpha-lap\E\ghost`・置くもの〕・OK）。開発者が貼った画面の写しを `E1r-alert.png` として置き場に残した。OK を押して終わった（`exit=` の行）。
- 終了コード 1・標準エラー（`run-E1r.err.log`）`Error: Error { code: HRESULT(0x80004005), message: "エラーを特定できません" }`: ゴーストが無いときは告知の後に `E_FAIL` で終わる作り（`crates/areka/src/main.rs` の `resolve_boot` が `Err(scene)` のとき `alert::raise` の後に `E_FAIL` を返す）。期待は「OK で終わる」で、終了コードを定めていないので判定に影響しない。
- ERROR・WARN（区間）: ERROR 1 件・WARN 0 件。外したもの:
  - `2026-10-02T13:05:37.287636Z ERROR areka::alert: [alert] 利用者へ告げます event="alert" scene=GhostMissing { ghost_store: "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\E\\ghost", argv: None } title="areka を起動できません" body="ゴーストが見つかりません。\n置く場所: C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\E\\ghost\n置くもの: ghost\\master\\descript.txt を持つゴーストのフォルダ" suppressed=false` ← ⓔ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「E1r はOK」 → 結果 **合格**。

### 項目 2 告知のとおりに置く（E2r）

- 区間: `E2r item=2 start=2026-10-02T13:06:54.1299994Z`（22:06:54 JST）〜 `E2r exit=0 end=2026-10-02T22:07:48.3665594+09:00`（13:07:48.37Z）。pid 5096。根 E の `ghost\konnoyayame` へ、作り直した控え `target\nar-samples\manual\konnoyayame\ghost\konnoyayame`（§9.1）を写してから起動した。
- 記録（`run-E2r.log` から逐語）:
  - `2026-10-02T13:06:54.650018Z  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Only dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\E\ghost\konnoyayame`
  - `2026-10-02T13:06:56.771036Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None`
  - `2026-10-02T13:06:56.776282Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1`
  - `2026-10-02T13:07:47.540233Z  INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4`
  - `2026-10-02T13:07:47.667560Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"`
- 目視（開発者）: `konnoyayame` が立って挨拶する・目の周りに四角い地色が出ない・挨拶の字形が化けない・左上の画素と同じ色の場所のクリックが背後の窓へ抜ける。メニューの「終了」で終えた（終了コード 0）。
- ERROR・WARN（区間）: ERROR 0 件・WARN 6 件。外したもの:
  - `2026-10-02T13:06:55.067782Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:06:55.080948Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:06:55.116856Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:06:55.117629Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:06:55.376153Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:06:55.377127Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「E2r はOK」 → 結果 **合格**。

### 項目 3 初回の起動（A1r）・項目 12 の 1 回目

- 区間: `A1r item=3 start=2026-10-02T13:08:13.3834358Z`（22:08:13 JST）〜 `A1r exit=0 end=2026-10-02T22:09:12.6268072+09:00`（13:09:12.63Z）。pid 28100。根 A は新しい zip を展開して作り直した（起動の直後の記録に、アプリ・ゴースト・シェルの記憶のファイルがどれも無いことの行 `persist file absent; loading as empty` が並ぶ）。
- 記録（`run-A1r.log` から逐語）:
  - `2026-10-02T13:08:13.866852Z  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Only dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\emo2`
  - `2026-10-02T13:08:13.875648Z  INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Companion dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\balloon\emo2-kakukaku`
  - `2026-10-02T13:08:14.670436Z  INFO areka::persist::restore: merge_scope restore scope=1 anchor=Bottom saved_win_x=None saved_win_y=None default_char_x=1340 default_char_y=904 char_x=1340 char_y=904 char_w=672 char_h=800 saved_off_x=None saved_off_y=None balloon_off_x=292 balloon_off_y=-150 balloon_x=1632 balloon_y=754`
  - `2026-10-02T13:08:16.632573Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None`
  - `2026-10-02T13:08:16.651745Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1`
  - `2026-10-02T13:09:05.963235Z  INFO actor{actor=emo-text}: areka::menu::trigger: [menu] shown event="menu_shown" scope=0 items=8`
  - `2026-10-02T13:09:12.342642Z  INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4`
  - `2026-10-02T13:09:12.458264Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"`
- 本体の窓の位置が保存された件: 13:08:51Z の左クリックがキャラクターの絵（本体・scope 0）に当たり、掴んで離した扱いになって本体の位置が保存された（§6.1 の「キャラクターの絵の上は押さない」から外れた）。
  - `2026-10-02T13:08:51.371241Z  INFO actor{actor=emo-text}: areka::persist::save: char DragEnd 保存 scope=0 char_x=2064 char_y=610 saved_x=2446 saved_y=610 char_w=Some(764) anchor=Bottom`
  - 保存された位置は動かす前と同じ: 保存の行の `saved_x=2446` は、起動のときの本体の既定の位置の下端中央（`default_char_x=2012`＋`char_w=868` の半分＝2446）と同じで、A2r で読み戻した本体は既定の位置のまま立った（下の項目 12 の A2r の scope 0 の行の `char_x=2012 char_y=330` が `default_char_x=2012 default_char_y=330` と同じ）。
  - 項目 12 が見るのは相方（scope 1）で、相方の位置は保存されていない（A1r の「char DragEnd 保存」の行はこの 1 件だけで `scope=0`）。AI は開発者に「A: このまま進め、項目 12 は相方（scope 1）で判定する／A1r をやり直す」を尋ね、開発者の返信（逐語）は「A で進めて、A2r 起動しました」。
  - この「動かさない左クリックで位置が保存される」ことは §8.5 に登記した。
- 目視（開発者）: えも？？ が `emo2-kakukaku` で立って挨拶する・絵の透明な場所のクリックが背後の窓へ抜ける・拡大率 200% で絵・当たり判定・窓の大きさ・バルーンの位置が崩れない（項目 11）。右クリックのメニューの「終了」で終えた（終了コード 0）。
- ERROR・WARN（区間）: ERROR 0 件・WARN 3 件。外したもの:
  - `2026-10-02T13:08:14.504862Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:08:14.803068Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:08:15.140424Z  WARN actor{actor=emo-text}: areka_emo_text::actor: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="(名前なし)" axis="x" wrap_threshold=254.0 inline_limit=240.0` ← ⓒ
- 除外の後: ERROR 0 件・WARN 0 件（項目 3 の期待は WARN の数を定めない）。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「OK」 → 結果 **合格**。

### 項目 12 初回だけの位置合わせ（A1r → A2r）

- 区間: 1 回目は上の A1r の区間。2 回目は `A2r item=12 start=2026-10-02T13:12:07.0643275Z`（22:12:07 JST）〜 `A2r item=4 start=2026-10-02T13:13:20.7739481Z`（22:13:20 JST）。A2r の pid は 4252（`A2r pid=4252 start=2026-10-02T22:12:07.1809078+09:00`）。
- 記録（A1r は `OnFirstBoot`・上の項目 3 の `id=OnFirstBoot` の行。A2r は `run-A2r.log` から逐語）:
  - `2026-10-02T13:12:07.218871Z  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Memory dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\emo2`
  - `2026-10-02T13:12:07.221667Z  INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Memory dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\balloon\emo2-kakukaku`
  - `2026-10-02T13:12:07.418074Z  INFO areka::persist::restore: merge_scope restore scope=0 anchor=Bottom saved_win_x=Some(2446) saved_win_y=Some(610) default_char_x=2012 default_char_y=330 char_x=2012 char_y=330 char_w=868 char_h=1374 saved_off_x=None saved_off_y=None balloon_off_x=-268 balloon_off_y=-258 balloon_x=1744 balloon_y=72`
  - `2026-10-02T13:12:07.418288Z  INFO areka::persist::restore: merge_scope restore scope=1 anchor=Bottom saved_win_x=None saved_win_y=None default_char_x=1340 default_char_y=904 char_x=1340 char_y=904 char_w=672 char_h=800 saved_off_x=None saved_off_y=None balloon_off_x=292 balloon_off_y=-150 balloon_x=1632 balloon_y=754`
  - `2026-10-02T13:12:08.202350Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBoot references=["「コンフィズリー」＆「City-Pop'n」"] status=None`
- 件数（区間）: `id=OnBoot` が 1 件・`id=OnFirstBoot` が 0 件・「char DragEnd 保存」が 0 件。相方（scope 1）の `merge_scope restore` は `saved_win_x=None saved_win_y=None` で、`char_x=1340 char_y=904` が `default_char_x=1340 default_char_y=904` と同じ＝既定の配置。
- 本体（scope 0）は A1r で保存された位置（`saved_win_x=Some(2446) saved_win_y=Some(610)`）を読み戻したが、`char_x=2012 char_y=330` が既定の `default_char_x=2012 default_char_y=330` と同じで、既定の位置に立った（項目 3 の「本体の窓の位置が保存された件」）。項目 12 の判定は相方で行う。
- 目視（開発者）: 初回のずらしが繰り返されず、相方が既定の配置で立った。
- ERROR・WARN（区間）: ERROR 0 件・WARN 3 件。外したもの:
  - `2026-10-02T13:12:07.340206Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:12:07.557150Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:12:07.935137Z  WARN actor{actor=emo-text}: areka_emo_text::actor: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="(名前なし)" axis="x" wrap_threshold=254.0 inline_limit=240.0` ← ⓒ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 12 はOK、項目 4 は縮退合格で、emo2 に申し送って。あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」 → 結果 **合格**。これで M1 の完成宣言の「持ち越した事項」の 1 行目を閉じる。

### 項目 4 2 体目・3 体目を入れる（A2r）

- 区間: `A2r item=4 start=2026-10-02T13:13:20.7739481Z`（22:13:20 JST）〜 `A2r item=5 start=2026-10-02T13:20:09.1951055Z`（22:20:09 JST）。
- 記録 ⒜ メニューの「インストール…」で `R_POST_and_KOMAINU.nar`（`run-A2r.log` から逐語）:
  - `2026-10-02T13:13:35.790615Z  INFO actor{actor=emo-text}: areka::menu::trigger: [menu] shown event="menu_shown" scope=0 items=8`
  - `2026-10-02T13:13:49.320281Z  INFO actor{actor=install}: areka::install::procedure: [install] 書庫の手続きを始めます event="install_begin" archive=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\vendors\sample_ghost\R_POST_and_KOMAINU.nar origin=Menu`
  - `2026-10-02T13:13:49.323709Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallBegin" raised=Script`
  - `2026-10-02T13:13:49.382452Z  INFO actor{actor=install}: areka::install::procedure: [install] 入れました event="install_done" archive=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\vendors\sample_ghost\R_POST_and_KOMAINU.nar kind=Ghost places=["C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\R_POST_and_KOMAINU"]`
  - `2026-10-02T13:13:49.421892Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallCompleteEx" raised=NoReply`
  - `2026-10-02T13:13:49.429341Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnInstallComplete references=["ghost", "Ｒポストと狛犬", ""] status=Some("talking")`
  - `2026-10-02T13:13:49.432828Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallComplete" raised=Script`
- 記録 ⒝ `claudia.nar` をエクスプローラからえも？？ の窓へ落とした（1 度目・`run-A2r.log` から逐語）:
  - `2026-10-02T13:14:25.022770Z  INFO actor{actor=emo-text}: areka::input_events::file_drop: [file_drop] 落とされた物を受け取って振り分けました event="file_drop_received" scope=0 count=1 files=0 dirs=0 installs=1 elapsed_ms=17`
  - `2026-10-02T13:14:25.039071Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallBegin" raised=Script`
  - `2026-10-02T13:14:25.273398Z  INFO actor{actor=install}: areka::install::procedure: [install] 入れました event="install_done" archive=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\vendors\sample_ghost\claudia.nar kind=Ghost places=["C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\claudia", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\balloon\\claudia", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\balloon\\claudia_vertical"]`
  - `2026-10-02T13:14:25.298263Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallCompleteEx" raised=NoReply`
  - `2026-10-02T13:14:25.331255Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnInstallComplete references=["ghost", "悪役令嬢クローディア", "クローディア"] status=Some("talking")`
  - `2026-10-02T13:14:25.334094Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallComplete" raised=NoReply`
- 記録 ⒝ の 2 度目（同じ `claudia.nar` をもう 1 度落とした。入れた場所は 1 度目と同じで、上書きの入れ直し・`run-A2r.log` から逐語）:
  - `2026-10-02T13:14:47.014358Z  INFO actor{actor=emo-text}: areka::input_events::file_drop: [file_drop] 落とされた物を受け取って振り分けました event="file_drop_received" scope=0 count=1 files=0 dirs=0 installs=1 elapsed_ms=2`
  - `2026-10-02T13:14:47.054554Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallBegin" raised=Script`
  - `2026-10-02T13:14:47.391116Z  INFO actor{actor=install}: areka::install::procedure: [install] 入れました event="install_done" archive=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\vendors\sample_ghost\claudia.nar kind=Ghost places=["C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\claudia", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\balloon\\claudia", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\balloon\\claudia_vertical"]`
  - `2026-10-02T13:14:47.420168Z  INFO actor{actor=install}: areka::install::procedure: [install] イベントを送りました event="install_event" id="OnInstallComplete" raised=NoReply`
- 件数（区間）: `install_done` 3 件（⒜ 1・⒝ 1 度目 1・⒝ 2 度目 1）・`file_drop_received` 2 件（2 度とも `installs=1`）・切替の行（`ghost_switch_requested`・`ghost_switch_booted`・`ghost_switch_done`）0 件・`id=OnGhostChanging` 0 件。表の期待（`install_done` 2 件・`file_drop_received` 1 件）より 1 件ずつ多いのは、`claudia.nar` を 2 度落としたため。1 度の落としごとに `file_drop_received` 1 件・`install_done` 1 件で、⒜ 1 回・⒝ 1 回の数え方と合う。
- イベントの返事: `raised=Script` は台本が返ったこと、`raised=NoReply` は返事なし・空の台本（`crates/areka/src/install/procedure.rs` の `Raised` の定義）。
  - ⒜ `R_POST_and_KOMAINU`: `OnInstallBegin` Script → `OnInstallCompleteEx` NoReply → `OnInstallComplete` Script（参照 `["ghost", "Ｒポストと狛犬", ""]`）。始まりと終わりの両方の台詞が返った。
  - ⒝ `claudia`（1 度目・2 度目とも）: `OnInstallBegin` Script → `OnInstallCompleteEx` NoReply → `OnInstallComplete` **NoReply**（参照 `["ghost", "悪役令嬢クローディア", "クローディア"]`＝ゴーストに同梱のバルーンの名前が 3 つ目に載る形）。始まりの台詞は返ったが、終わりの台詞は返らなかった。
- 利用条件の画面: 出なかった。§6.7 のとおり、`R_POST_and_KOMAINU.nar` も `claudia.nar` も利用条件のファイル（`crates/areka/src/install/terms.rs` の `TERMS_FILES`＝`terms.txt`・`terms.md`）を持たないため（2026-10-01 に 2 つの `.nar` の中身の一覧で `terms.` に当たる項目が 0 件であることを確かめた）。
- 目視（開発者）: 2 つとも入り、えも？？ のまま切り替わらなかった。`claudia.nar` を落としたときに開発者が書いたこと（逐語）「「claudia.nar」をドラッグ→「荷物が来たよ」メッセージ。narと認識されていない？」。「荷物が来たよ。」は、根 A の `ghost\emo2\ghost\master\dic\install.pasta` の `＊OnInstallBegin` の台詞の 1 つで、書庫として受け取ったときの始まりの台詞（記録の `install_begin`・`install_done` のとおり書庫として入った）。終わりの台詞が出なかったので、始まりの台詞だけが見えた。
- ERROR・WARN（区間）: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 12 はOK、項目 4 は縮退合格で、emo2 に申し送って。あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」 → 結果 **合格（縮退）**。
- 縮退:
  - ⑴ 成り立たなかった期待: 「どちらもインストールの台詞が流れ」のうち、⒝ `claudia.nar`（ゴーストにバルーンを同梱した書庫）の終わりの台詞。`OnInstallComplete` が 2 度とも `raised=NoReply`（始まりの `OnInstallBegin` は `raised=Script`）。⒜ `R_POST_and_KOMAINU.nar` は始まりと終わりの両方が `raised=Script` で成り立った。
  - ⑵ 理由: えも？？ の辞書（ghost_dev `75e560e` の版・`dic/install.pasta`）が、`claudia.nar` の `OnInstallComplete` に台本を返さなかった（areka の側は参照を送り、返事なしとして手続きを続けた）。`install.pasta` の `SCENE.on_install_complete` は参照の 1 つ目（`^ghost`）だけで分かれ、⒜ も ⒝ もこれを満たすので、原因は分かっていない。観測できた違いは 3 つ目の参照（空か「クローディア」か）と入れ方（メニューか投げ込みか）だけ（§8.3）。2026-10-01 の前の zip の版では、えも？？ の辞書にインストールの場面がそもそも無く、3 つのイベントがどれも `raised=NoReply` だった（§9.1）。今回の版で始まりの台詞と、同梱のないゴーストの終わりの台詞は出るようになり、残ったのは同梱のある書庫の終わりの台詞だけ。
  - ⑶ 開発者の裁定（2026-10-02・逐語）: 「項目 12 はOK、項目 4 は縮退合格で、emo2 に申し送って。あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」。2026-10-02 にセッション「emo2 開発セッション」へ申し送った（§8.3）。

### 項目 5 メニューでゴーストを替えて戻る（A2r）

- 区間: `A2r item=5 start=2026-10-02T13:20:09.1951055Z`（22:20:09 JST）〜 `A2r item=7 start=2026-10-02T13:21:56.1311592Z`（22:21:56 JST）。
- 記録（`run-A2r.log` から逐語）:
  - ⑵ `R_POST_and_KOMAINU` へ:
    - `2026-10-02T13:20:23.040342Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替の要求を kanade へ送った event="ghost_switch_requested" from=Some("えも？？") to=Ｒポストと狛犬 raise_event=true origin="manual" boot_event=None`
    - `2026-10-02T13:20:26.622182Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None`
    - `2026-10-02T13:20:26.639287Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替で起こしたゴーストが定常に入った——切替を終える event="ghost_switch_done" ghost=Some("R_POST_and_KOMAINU") attempt=Target`
  - ⑶ `claudia` へ:
    - `2026-10-02T13:20:36.955954Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替の要求を kanade へ送った event="ghost_switch_requested" from=Some("Ｒポストと狛犬") to=悪役令嬢クローディア raise_event=true origin="manual" boot_event=None`
    - `2026-10-02T13:20:37.866378Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None`
    - `2026-10-02T13:20:37.893872Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替で起こしたゴーストが定常に入った——切替を終える event="ghost_switch_done" ghost=Some("claudia") attempt=Target`
  - ⑷ えも？？ へ:
    - `2026-10-02T13:20:58.367299Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替の要求を kanade へ送った event="ghost_switch_requested" from=Some("悪役令嬢クローディア") to=えも？？ raise_event=true origin="manual" boot_event=None`
    - `2026-10-02T13:21:02.850886Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnGhostChanged references=["Claudia", "\\0\\s[26]ふん、むらさきですって？\\w9\\nせいぜい楽しんでいらっしゃい。\\w8\\1お嬢様、扇を握る手に力が。\\e", "悪役令嬢クローディア", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\claudia", "", "", "", "master"] status=None`
    - `2026-10-02T13:21:02.916393Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替で起こしたゴーストが定常に入った——切替を終える event="ghost_switch_done" ghost=Some("emo2") attempt=Target`
- 件数（区間）: `ghost_switch_done` 3 件（`R_POST_and_KOMAINU`・`claudia`・`emo2` の順）・`menu_shown` 3 件（どれも `items=12`。入れる前の A2r の区間 12 までは `items=8`）。
- 相方の窓の位置が保存された件: `claudia` が立った後の 13:20:46Z に、`claudia` の相方（scope 1）で「char DragEnd 保存」が 1 件出た。項目 12 の観測（A2r の `item=12` の区間）を書き終えた後なので、項目 12 には響かない。保存された値は既定の位置と同じ（`char_x=1548 char_y=704` が直前の `claudia` の起動の `default_char_x=1548 default_char_y=704` と同じ）。§8.5 に登記した。
  - `2026-10-02T13:20:46.217387Z  INFO actor{actor=emo-text}: areka::persist::save: char DragEnd 保存 scope=1 char_x=1548 char_y=704 saved_x=1548 saved_y=704 char_w=Some(666) anchor=Free`
- 目視（開発者）: 一覧に 3 体が並ぶ・送り出しの台詞の後に切り替わり、挨拶し、絵が出る・挨拶の字形が化けない・左上の画素と同じ色の場所のクリックが背後の窓へ抜ける。
- ERROR・WARN（区間）: ERROR 0 件・WARN 15 件。外したもの:
  - `2026-10-02T13:20:26.338526Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:20:26.338960Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:20:26.364200Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:20:26.364455Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:20:26.444371Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:20:26.444626Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:20:37.548158Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:20:37.548543Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:20:37.562092Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:20:37.562336Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:20:37.632981Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:20:37.633203Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:21:01.500808Z  WARN actor{actor=emo-text}: areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:21:01.865929Z  WARN actor{actor=emo-text}: areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:21:02.135339Z  WARN actor{actor=emo-text}: areka_emo_text::actor: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="(名前なし)" axis="x" wrap_threshold=254.0 inline_limit=240.0` ← ⓒ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 5 はOK」 → 結果 **合格**。

### 項目 7 バルーンの切り替え（A2r）

- 区間: `A2r item=7 start=2026-10-02T13:21:56.1311592Z`（22:21:56 JST）〜 `A2r item=click start=2026-10-02T13:23:04.9229392Z`（22:23:04 JST）。
- 記録（`run-A2r.log` から逐語）:
  - `2026-10-02T13:22:04.520182Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBalloonChange references=["Balloon for Staysee Syncfield", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\balloon\\StayseeBalloon"] status=None`
  - `2026-10-02T13:22:04.520171Z  INFO actor{actor=emo-text}: areka::emo2_boot::frame::switch: シェル・バルーンの切替を終えた event="skin_switch_done" kind=Balloon to=StayseeBalloon epoch=1 marked=None swap_ms=7.8855 since_commit_ms=26.7348`
  - `2026-10-02T13:22:18.055448Z  INFO actor{actor=emo-text}: areka::emo2_boot::frame::switch: シェル・バルーンの切替を終えた event="skin_switch_done" kind=Balloon to=emo2-kakukaku epoch=2 marked=None swap_ms=5.8248 since_commit_ms=8.8827`
  - `2026-10-02T13:22:18.055461Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBalloonChange references=["kakukaku for emo-gs", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\balloon\\emo2-kakukaku"] status=None`
- 件数（区間）: `skin_switch_done` 2 件（`StayseeBalloon` epoch=1・`emo2-kakukaku` epoch=2）・`id=OnBalloonChange` 2 件・切替の行（`ghost_switch_*`）0 件。
- 目視（開発者）: どちらも次の台詞から新しいバルーンで出た。
- 付記（切替の後の 1 文字の台詞）: A3r の項目 10 の準備で見た（下の「項目 7 の付記（A3r・項目 10 の準備）」）。
- ERROR・WARN（区間）: ERROR 0 件・WARN 3 件。外したもの:
  - `2026-10-02T13:22:04.425165Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:22:04.425552Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:22:18.061575Z  WARN actor{actor=emo-text}: areka_emo_text::actor: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="(名前なし)" axis="x" wrap_threshold=254.0 inline_limit=240.0` ← ⓒ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 7 はOK」 → 結果 **合格**（付記の結果は下の項目 7 の付記）。

### 付随 左クリックの後の右クリック（A2r）

- 区間: `A2r item=click start=2026-10-02T13:23:04.9229392Z`（22:23:04 JST）〜 `A2r item=13a start=2026-10-02T13:23:51.5667810Z`（22:23:51 JST）。
- 記録（`run-A2r.log` から逐語）:
  - `2026-10-02T13:23:12.861575Z  INFO actor{actor=emo-text}: areka::persist::save: char DragEnd 保存 scope=0 char_x=2064 char_y=610 saved_x=2446 saved_y=610 char_w=Some(764) anchor=Bottom`
  - `2026-10-02T13:23:17.794557Z  INFO actor{actor=emo-text}: areka::menu::trigger: [menu] shown event="menu_shown" scope=0 items=12`
- 1 行目は手順の左クリック（本体の絵の上）で、A1r と同じく位置が保存された（値は A1r と同じ `saved_x=2446 saved_y=610`・§8.5）。項目 12 の観測の後なので響かない。
- 件数（区間）: `menu_shown` 1 件。
- 目視（開発者）: 左クリックを 1 度した後の右クリックでメニューが出た。
- ERROR・WARN（区間）: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「付随の右クリックはOK」 → 結果 **合格**。

### 項目 13 強制終了の次の起動（A2r → A3r）

**前半（A2r の終わり）**

- 区間: `A2r item=13a start=2026-10-02T13:23:51.5667810Z`（22:23:51 JST）〜 `A2r killed pid=4252 end=2026-10-02T22:24:40.9610512+09:00`（13:24:40.96Z）。
- 記録（`run-A2r.log` から逐語）:
  - `2026-10-02T13:23:55.323382Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替の要求を kanade へ送った event="ghost_switch_requested" from=Some("えも？？") to=悪役令嬢クローディア raise_event=true origin="manual" boot_event=None`
  - `2026-10-02T13:24:05.280598Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 定常に入ったゴーストの名前を起動中の印へ投函した event="session_mark_steady" ghost=悪役令嬢クローディア`
  - `2026-10-02T13:24:05.281131Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替で起こしたゴーストが定常に入った——切替を終える event="ghost_switch_done" ghost=Some("claudia") attempt=Target`
- 期待する名前: 上の行が A2r 全体の最後の `session_mark_steady`（A2r の `session_mark_steady` は全体で 4 件で、これが最後）。`runs.txt` に控えた行（逐語）: `A2r expect13 ghost=悪役令嬢クローディア`。印の値は `descript.txt` の `name` で、フォルダ名 `claudia` ではない。
- 強制終了（§6.4 の手順を写した `kill-a2.ps1` で行った。`runs.txt` から逐語）:
  - `A2r children=27964:shiori-host32-helper.exe at=2026-10-02T22:24:40.9486686+09:00`
  - `A2r killed pid=4252 end=2026-10-02T22:24:40.9610512+09:00`
  - `A2r exit=-1 end=2026-10-02T22:24:41.0296134+09:00`
  - `child-left` の行は 0 件＝5 秒後に書き留めた子（pid 27964）は残っていなかった。止めたプロセスは `$p`（pid 4252）の 1 つだけ。
- A2r の印が消えていない: `run-A2r.log` 全体で `session_mark_cleared` 0 件・`event="app_exit"` 0 件。記録の最後の行は 13:24:40.960230Z（強制終了の直前）。
- 目視（開発者・逐語）: `claudia` へ替えた後に「claudia が立ちました」、強制終了の後に「強制終了しました」。
- ERROR・WARN（区間）: ERROR 0 件・WARN 6 件。外したもの:
  - `2026-10-02T13:24:04.767804Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:24:04.775056Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:24:04.800202Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:24:04.800602Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:24:04.941159Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:24:04.941544Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
- 除外の後: ERROR 0 件・WARN 0 件。

**後半（A3r の始まり）**

- 区間: `A3r item=13 start=2026-10-02T13:26:27.9612524Z`（22:26:27 JST）〜 `A3r item=8 start=2026-10-02T13:27:40.3577822Z`（22:27:40 JST）。A3r の pid は 11220（`A3r pid=11220 start=2026-10-02T22:26:28.0452940+09:00`）。
- 記録（`run-A3r.log` から逐語）:
  - `2026-10-02T13:26:28.062257Z  INFO areka::boot_config: 前回はきれいに終わらなかったので、最後のゴーストの記憶を読まずに起動するゴーストを決めます event="session_mark_found" ghost=悪役令嬢クローディア`
  - `2026-10-02T13:26:28.063634Z  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Default dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\emo2`
  - `2026-10-02T13:26:28.812525Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBoot references=["「コンフィズリー」＆「City-Pop'n」", "", "", "", "", "", "halt", "悪役令嬢クローディア"] status=None`
  - `2026-10-02T13:26:28.815277Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1`
- 照合: `session_mark_found` の `ghost=悪役令嬢クローディア` が `A2r expect13 ghost=悪役令嬢クローディア` と同じ。`OnBoot` の `references` の 8 番目（0 から数えて 7）が `悪役令嬢クローディア` で同じ、7 番目（0 から数えて 6）が `halt`。起こしたゴーストは `route=Default` の `emo2`（前回のゴースト `claudia` ではない）。
- 目視（開発者）: えも？？ で立ち、落ちたゴーストのことを話した（zip の `emo2` は `＊起動halt` の台詞を持つ・§1.2）。
- ERROR・WARN（区間）: ERROR 0 件・WARN 3 件。外したもの:
  - `2026-10-02T13:26:28.152154Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:26:28.286036Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:26:28.534519Z  WARN actor{actor=emo-text}: areka_emo_text::actor: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="(名前なし)" axis="x" wrap_threshold=254.0 inline_limit=240.0` ← ⓒ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 前半「claudia が立ちました」「強制終了しました」、後半「項目 13 はOK、項目 8 も済みました」 → 結果 **合格**。

### 走行の間（A2r の後・A3r の前）

areka を止めた状態で、根 A の写しだけを 3 つ変えた（リポジトリの検体と zip は変えていない）。`runs.txt` の行（逐語）:

```
between second-shell=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\R_POST_and_KOMAINU\shell\second dic02_Event md5 before=6A427453663B7FAF93DFEAC4236871B9 after=48F167B8354CFEE9BBB0BF1801DC28E6 at=2026-10-02T22:25:22.1187816+09:00
```

- ⑴ 2 つ目のシェル: `ghost\R_POST_and_KOMAINU\shell\master\` を `shell\second\` へ写し、写した `descript.txt` の `name,master` を `name,second` にし、写した側の `surface0000.png` と `surface0001.png` を入れ替えた。
- ⑵ 付随の確かめ: `ghost\R_POST_and_KOMAINU\ghost\master\dic02_Event.txt` の `＊OnShellChanged` の台詞に `：\![open,readme]` の 1 行を足した。
- ⑶ 項目 7 の付記: 同じ `dic02_Event.txt` の末尾に、空の行に続けて `＊OnBalloonChange` と `：ん` の 2 行を足した（開発者の了承は 2026-10-01・逐語「R_POST の写しだけに、次の 2 行を足したい　どうぞ。」・tasks.md の Implementation Notes）。
- ⑵⑶ とも Shift_JIS と CRLF を保った。`dic02_Event.txt` の md5 は変える前 `6A427453…71B9`・変えた後 `48F167B8…28E6`。
- 2026-10-02（A4r の後）に根 A を読むだけで確かめ直したこと: `shell\master\descript.txt` が `name,master`・`shell\second\descript.txt` が `name,second`／`shell\second\surface0000.png` の md5 が `shell\master\surface0001.png` と同じ（`23e71d2e…0ab6`）、`shell\second\surface0001.png` が `shell\master\surface0000.png` と同じ（`49a8d6b8…28cd`）／2 つのシェルのファイルの名前の一覧は同じ／`dic02_Event.txt` の md5 は `48f167b8…28e6`（変えた後のまま）／Shift_JIS として読むと 48〜51 行目が `＊OnShellChanged` の台詞で 51 行目が `：\![open,readme]`、504・505 行目が `＊OnBalloonChange`・`：ん`、ファイルの末尾のバイト列は `：ん` と CRLF。

### 項目 8 ネットワーク更新（A3r）

- 区間: `A3r item=8 start=2026-10-02T13:27:40.3577822Z`（22:27:40 JST）〜 `A3r item=switch start=2026-10-02T13:32:57.4275591Z`（22:32:57 JST）。
- 手で変えたファイルと md5（`runs.txt` から逐語）:
  - `A3r item8 file=ghost\emo2\readme.txt md5 before=9743530b82d7fb47d8013c4aacc146a8 after_edit=6906bead25ecc2abbe060a18b9c4f012 at=2026-10-02T22:27:24.3909635+09:00`
  - `A3r item8 md5 after_update=9743530b82d7fb47d8013c4aacc146a8 at=2026-10-02T22:29:42.1693443+09:00`
  - 変えたのは `<根 A>\ghost\emo2\readme.txt`（`emo2` の `updates.txt` に載る、読むだけのテキスト）で、AI が項目 8 の直前に末尾へ 1 行足した。更新の後の md5 は変える前と同じに戻った。2026-10-02（A4r の後）に読み直しても `9743530b82d7fb47d8013c4aacc146a8`。
- 記録 1 回目（`run-A3r.log` から逐語）:
  - `2026-10-02T13:27:49.524714Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnUpdateBegin references=["えも？？", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\emo2", "", "ghost", "manual"] status=Some("talking")`
  - `2026-10-02T13:27:54.595141Z  INFO actor{actor=update}: areka::update::procedure: [update] 対象の更新を終えました event="update_target_done" name=えも？？ end=Changed(21)`
  - `2026-10-02T13:27:54.595830Z  INFO actor{actor=update}: areka::update::procedure: [update] 総括を後送りの列の最後に置きます event="update_summary" sent=true id="OnUpdateResult" references=["ghost\u{1}OK\u{1}21"]`
  - `2026-10-02T13:27:54.596384Z  INFO actor{actor=update}: areka::update::procedure: [update] 同じゴーストの読み直しを頼みました event="update_reload_requested" ghost_dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\emo2`
  - `2026-10-02T13:27:54.616588Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替の要求を kanade へ送った event="ghost_switch_requested" from=Some("えも？？") to=えも？？ raise_event=false origin="automatic" boot_event=Some("OnUpdateComplete")`
  - `2026-10-02T13:27:54.616916Z  INFO actor{actor=emo-text}: areka::update::desk: [update] 更新した中身を読むために、同じゴーストへの切替を頼みました event="update_reload_requested" verdict=Accepted folder=emo2`
  - `2026-10-02T13:27:55.483394Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnUpdateComplete references=["changed", "emo2-kakukaku/arrow0.png,emo2-kakukaku/arrow1.png,emo2-kakukaku/balloonc1.png,emo2-kakukaku/balloonc2.png,emo2-kakukaku/balloonc3.png,emo2-kakukaku/balloonc4.png,emo2-kakukaku/balloonk0.png,emo2-kakukaku/balloonk0s.txt,emo2-kakukaku/balloons0.png,emo2-kakukaku/balloons0s.txt,emo2-kakukaku/descript.txt,emo2-kakukaku/install.txt,emo2-kakukaku/marker.png,emo2-kakukaku/online.pdn,emo2-kakukaku/online0.png,emo2-kakukaku/online1.png,emo2-kakukaku/online2.png,emo2-kakukaku/online3.png,emo2-kakukaku/sstp.png,emo2-kakukaku/sstp_new.png,readme.txt", "", "ghost", "manual"] status=None`
  - `2026-10-02T13:27:55.502804Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替で起こしたゴーストが定常に入った——切替を終える event="ghost_switch_done" ghost=Some("emo2") attempt=Target`
  - `2026-10-02T13:27:55.504069Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnUpdateResult references=["ghost\u{1}OK\u{1}21"] status=Some("talking")`
- 1 回目の差分は 21 件（`Changed(21)`）。`OnUpdateComplete` の 2 つ目の参照に並ぶ名前は `emo2-kakukaku/` で始まる 20 個と `readme.txt` の 1 個。手で変えた `readme.txt` のほかに 20 件の差分があったのは、zip の `emo2` の中に `emo2-kakukaku/` が無いのに、配布サイトの更新の一覧がゴーストのフォルダからの相対で `emo2-kakukaku/*` を載せているため（§8.4）。
- 記録 2 回目（`run-A3r.log` から逐語）:
  - `2026-10-02T13:28:13.103304Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnUpdateBegin references=["えも？？", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\emo2", "", "ghost", "manual"] status=None`
  - `2026-10-02T13:28:13.388339Z  INFO actor{actor=update}: areka::update::procedure: [update] 対象の更新を終えました event="update_target_done" name=えも？？ end=Unchanged`
  - `2026-10-02T13:28:13.394690Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnUpdateComplete references=["none", "", "", "ghost", "manual"] status=Some("talking")`
  - `2026-10-02T13:28:13.403200Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnUpdateResult references=["ghost\u{1}OK\u{1}0"] status=Some("talking")`
- 件数（区間）: `OnUpdateComplete` 2 件・`OnUpdateResult` 2 件（どちらも 1 回目・2 回目の順に、`OnUpdateComplete` の後に `OnUpdateResult`）・`id=OnGhostChanged` 0 件・`id=OnBoot` 0 件・`id=OnGhostChanging` 0 件・`ghost_switch_done` 1 件（1 回目の読み直しの 1 件）。`update_reload_requested` の行は 2 行で、どちらも 1 回目の 1 度の読み直し（手続きの側の頼みと、受けた側の `verdict=Accepted`）。2 回目の更新の始まり（13:28:13.08Z）から区間の終わりまでの `update_reload_requested`・`ghost_switch_*` は 0 件＝2 回目は読み直していない。
- 目視（開発者）: 1 回目は進捗の台詞の後にいったん引っ込んで同じゴーストが戻った。2 回目は引っ込まずに終わった。
- ERROR・WARN（区間）: ERROR 0 件・WARN 7 件。外したものと外さないもの:
  - `2026-10-02T13:27:54.595362Z  WARN actor{actor=update}: areka::update::procedure: [update] 更新先が無いので、この対象を飛ばします event="update_target_skipped" reason="no_homeurl" kind="shell" name=「コンフィズリー」＆「City-Pop'n」 dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\emo2\shell\master` ← ⓓ
  - `2026-10-02T13:27:54.595565Z  WARN actor{actor=update}: areka::update::procedure: [update] 更新先が無いので、この対象を飛ばします event="update_target_skipped" reason="no_homeurl" kind="balloon" name=kakukaku for emo-gs dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\balloon\emo2-kakukaku` ← 外さない（§7.1）
  - `2026-10-02T13:27:54.799978Z  WARN actor{actor=emo-text}: areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:27:54.978361Z  WARN actor{actor=emo-text}: areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547` ← ⓑ
  - `2026-10-02T13:27:55.129228Z  WARN actor{actor=emo-text}: areka_emo_text::actor: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="(名前なし)" axis="x" wrap_threshold=254.0 inline_limit=240.0` ← ⓒ
  - `2026-10-02T13:28:13.389504Z  WARN actor{actor=update}: areka::update::procedure: [update] 更新先が無いので、この対象を飛ばします event="update_target_skipped" reason="no_homeurl" kind="shell" name=「コンフィズリー」＆「City-Pop'n」 dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\emo2\shell\master` ← ⓓ
  - `2026-10-02T13:28:13.391115Z  WARN actor{actor=update}: areka::update::procedure: [update] 更新先が無いので、この対象を飛ばします event="update_target_skipped" reason="no_homeurl" kind="balloon" name=kakukaku for emo-gs dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\balloon\emo2-kakukaku` ← 外さない（§7.1）
- 除外の後: ERROR 0 件・WARN 2 件（バルーンの更新先が無い 2 件・§7.1 の「外さないもの」）。項目 8 の期待は WARN の数を定めないので、判定には響かない。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 8 はOK、emo2 に申し送って」 → 結果 **合格**。申し送りの中身は §8.4。

### 走行 A3r の `item=switch`（どの項目の数にも入れない）

- 区間: `A3r item=switch start=2026-10-02T13:32:57.4275591Z`（22:32:57 JST）〜 `A3r item=6 start=2026-10-02T13:33:19.0298760Z`（22:33:19 JST）。
- 記録（`run-A3r.log` から逐語）: `2026-10-02T13:33:09.533456Z  INFO actor{actor=emo-text}: areka::emo2_boot::ghost_switch: 切替で起こしたゴーストが定常に入った——切替を終える event="ghost_switch_done" ghost=Some("R_POST_and_KOMAINU") attempt=Target`
- WARN 6 件（すべて ⓐ）・ERROR 0 件。外したもの:
  - `2026-10-02T13:33:09.221517Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:33:09.234786Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:33:09.251168Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:33:09.251372Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:33:09.298940Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:33:09.299153Z  WARN actor{actor=emo-text}: areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
- 除外の後: ERROR 0 件・WARN 0 件。

### 項目 6 シェルの切り替え・付随 `\![open,readme]`（A3r）

- 区間: `A3r item=6 start=2026-10-02T13:33:19.0298760Z`（22:33:19 JST）〜 `A3r item=prep10 start=2026-10-02T13:35:31.6937977Z`（22:35:31 JST）。ゴーストは `R_POST_and_KOMAINU`（走行の間に作った `second` を持つ）。
- 記録（`run-A3r.log` から逐語）:
  - `2026-10-02T13:33:26.548280Z  INFO actor{actor=emo-text}: areka::emo2_boot::frame::switch: シェル・バルーンの切替を終えた event="skin_switch_done" kind=Shell to=second epoch=1 marked=Some(Completed) swap_ms=21.818299999999997 since_commit_ms=50.005599999999994`
  - `2026-10-02T13:33:26.548278Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnShellChanged references=["second", "Ｒポストと狛犬", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\R_POST_and_KOMAINU\\shell\\second"] status=None`
  - `2026-10-02T13:33:27.193183Z  INFO actor{actor=emo-text}: areka::readme: [readme] opened the readme with the default application event="readme_opened" path=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\R_POST_and_KOMAINU\readme.txt`
  - `2026-10-02T13:33:38.257198Z  INFO actor{actor=emo-text}: areka::emo2_boot::frame::switch: シェル・バルーンの切替を終えた event="skin_switch_done" kind=Shell to=master epoch=2 marked=Some(Completed) swap_ms=7.5165 since_commit_ms=13.1329`
  - `2026-10-02T13:33:38.257199Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnShellChanged references=["master", "Ｒポストと狛犬", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\ghost\\R_POST_and_KOMAINU\\shell\\master"] status=None`
  - `2026-10-02T13:33:38.768565Z  INFO actor{actor=emo-text}: areka::readme: [readme] opened the readme with the default application event="readme_opened" path=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\R_POST_and_KOMAINU\readme.txt`
- 件数（区間）: `skin_switch_done` 2 件（`second` epoch=1・`master` epoch=2）・`id=OnShellChanged` 2 件・`readme_opened` 2 件（どちらも `<根 A>\ghost\R_POST_and_KOMAINU\readme.txt`）・ゴーストが降りたことを示す行（`ghost_switch_*`・`id=OnGhostChanging`・`id=OnClose`）0 件。
- 目視（開発者）: どちらも絵が替わり、ゴーストは降りなかった。どちらの切り替えでも説明書が開いた（AI はこの手順で、絵の替わりと説明書の開きの両方を見るよう開発者に頼んだ）。
- ERROR・WARN（区間）: ERROR 0 件・WARN 4 件。外したもの:
  - `2026-10-02T13:33:25.978504Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:33:25.978681Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
  - `2026-10-02T13:33:37.622679Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:33:37.622984Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 6 はOK」 → 項目 6 は **合格**、付随 `\![open,readme]` も **合格**（同じ返信で 2 つを判定した）。

### 項目 7 の付記（A3r・項目 10 の準備）

- 区間: `A3r item=prep10 start=2026-10-02T13:35:31.6937977Z`（22:35:31 JST）〜 `A3r item=9 start=2026-10-02T13:36:32.2941478Z`（22:36:32 JST）。
- 記録（`run-A3r.log` から逐語）:
  - `2026-10-02T13:35:37.951785Z  INFO actor{actor=emo-text}: areka::emo2_boot::frame::switch: シェル・バルーンの切替を終えた event="skin_switch_done" kind=Shell to=second epoch=3 marked=Some(Completed) swap_ms=18.694300000000002 since_commit_ms=32.714`
  - `2026-10-02T13:35:38.533281Z  INFO actor{actor=emo-text}: areka::readme: [readme] opened the readme with the default application event="readme_opened" path=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\R_POST_and_KOMAINU\readme.txt`
  - `2026-10-02T13:35:51.034835Z  INFO actor{actor=emo-text}: areka::emo2_boot::frame::switch: シェル・バルーンの切替を終えた event="skin_switch_done" kind=Balloon to=claudia epoch=4 marked=None swap_ms=6.1655 since_commit_ms=9.5158`
  - `2026-10-02T13:35:51.034851Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBalloonChange references=["クローディア", "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\A\\balloon\\claudia"] status=None`
  - `2026-10-02T13:35:51.035786Z  INFO actor{actor=kanade}: kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=8 origin="OnBalloonChange"`
  - `2026-10-02T13:35:51.063194Z DEBUG actor{actor=emo-text}: areka_emo_text::state: Text cue 適用（追記＋配送 duration 由来のリビール時刻確定） actor=0 len=1 at=0.0 duration=0.05 interval=0.05`
  - `2026-10-02T13:35:51.083608Z  INFO actor{actor=emo-text}: areka::emo2_boot::balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true`
  - `2026-10-02T13:36:01.866982Z  INFO actor{actor=emo-text}: areka::persist::save: char DragEnd 保存 scope=0 char_x=2386 char_y=780 saved_x=2622 saved_y=780 char_w=Some(472) anchor=Bottom`
- 読み方: 既定でないバルーン `claudia` へ替えた直後の `OnBalloonChange` に、走行の間に足した台詞が返り（`steady_talk … origin="OnBalloonChange"`）、文字の長さ 1 の台詞（`Text cue 適用 … len=1`）でバルーンが現れた（`scope=0 trigger="content" visible=true`。`OnBalloonChange` の送出から 49 ms 後）。バグ `balloon-reappear-short-talk` の側から頼まれた確かめ（§0.5）で、§0.4 の着地の判定（着地した）と合う。
- `second` を選んだときの `readme_opened` 1 件は、走行の間に `＊OnShellChanged` へ足した `\![open,readme]` によるもの（項目 6 と同じ）。
- 最後の行は、手順の「本体の窓を 1 度掴んで離す」で出た「char DragEnd 保存」（項目 10 で比べる値 `saved_x=2622 saved_y=780`）。
- 件数（区間）: `skin_switch_done` 2 件（Shell `second` epoch=3・Balloon `claudia` epoch=4）・`id=OnBalloonChange` 1 件・「char DragEnd 保存」1 件。
- 目視（開発者・逐語）: 「項目 10 の準備は済み、「ん」でバルーンが出ました」。
- ERROR・WARN（区間）: ERROR 0 件・WARN 2 件。外したもの:
  - `2026-10-02T13:35:37.394253Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png` ← ⓐ
  - `2026-10-02T13:35:37.395834Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png` ← ⓐ
- 除外の後: ERROR 0 件・WARN 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 結果: 項目 7 の付記は **合**（上の開発者の返信による）。項目 7 は付記を含めて **合格**。

### 項目 9 終了（A3r の終わり）

- 区間: `A3r item=9 start=2026-10-02T13:36:32.2941478Z`（22:36:32 JST）〜 `A3r exit=0 end=2026-10-02T22:36:35.8360853+09:00`（13:36:35.84Z）。
- 記録（`run-A3r.log` から逐語）:
  - `2026-10-02T13:36:34.129325Z  INFO actor{actor=emo-text}: areka::menu::trigger: [menu] shown event="menu_shown" scope=0 items=13`
  - `2026-10-02T13:36:35.109911Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnClose references=["user", "0", "0"] status=None`
  - `2026-10-02T13:36:35.683844Z  INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4`
  - `2026-10-02T13:36:35.719097Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"`
- 終了コード: `0`（`runs.txt` の `A3r exit=0 end=2026-10-02T22:36:35.8360853+09:00`）。
- A3r 全体の ERROR: 0 件（除外の前から 0 件。外したものは無い）。A3r 全体の WARN は 22 件（区間ごとの内訳: 13 後半 3・8 7・switch 6・6 4・prep10 2・9 0。どれも上の各区間に逐語で写した。外さないものは項目 8 の 2 件）。項目 9 の区間の WARN は 0 件。
- 起動中の印: 上の `session_mark_cleared` の行が 1 件（A3r 全体で 1 件）。
- 目視（開発者）: 別れの台詞の後にプロセスが終わった。開発者の返信（逐語）「項目 9 は済みました」。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 9 はOK、A4r 起動し、終了した」 → 結果 **合格**。

### 項目 10 前回の状態の復元（A4r）

- 区間: `A4r item=10 start=2026-10-02T13:37:16.4312187Z`（22:37:16 JST）〜 `A4r exit=0 end=2026-10-02T22:37:32.9257748+09:00`（13:37:32.93Z）。pid 6332（`A4r pid=6332 start=2026-10-02T22:37:16.5364518+09:00`）。
- 記録（`run-A4r.log` から逐語）:
  - `2026-10-02T13:37:16.565566Z  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Memory dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\R_POST_and_KOMAINU`
  - `2026-10-02T13:37:16.567432Z  INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Memory dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\balloon\claudia`
  - `2026-10-02T13:37:16.603693Z  INFO areka_emo_present::shell_target: shell: シェルの面の画像の一覧が終わった（R6.1） shell_dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\R_POST_and_KOMAINU\shell\second recognized=10 used=10 shadowed=0`
  - `2026-10-02T13:37:16.635636Z  INFO areka::persist::restore: merge_scope restore scope=0 anchor=Bottom saved_win_x=Some(2622) saved_win_y=Some(780) default_char_x=2408 default_char_y=780 char_x=2386 char_y=780 char_w=472 char_h=924 saved_off_x=None saved_off_y=None balloon_off_x=-760 balloon_off_y=140 balloon_x=1626 balloon_y=920`
  - `2026-10-02T13:37:16.709880Z  INFO areka::boot_resolve: [boot_resolve] 最後に使ったものを記憶へ書きました（- は argv なので書いていない） event="last_used_recorded" ghost="R_POST_and_KOMAINU" balloon="claudia" shell="second"`
  - `2026-10-02T13:37:16.867035Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBoot references=["second"] status=None`
  - `2026-10-02T13:37:32.820993Z  INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4`
  - `2026-10-02T13:37:32.857262Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"`
- 照合:
  - ゴースト `R_POST_and_KOMAINU`（`route=Memory`）・バルーン `claudia`（`route=Memory`）・シェル `second`（シェルの面の一覧の行の `shell_dir=…\shell\second` と、`last_used_recorded` の `shell="second"`）。どれも A3r の最後の状態（項目 10 の準備で選んだもの）と同じ。
  - `merge_scope restore scope=0` の `saved_win_x=Some(2622) saved_win_y=Some(780)` が、A3r の項目 10 の準備の「char DragEnd 保存 scope=0 … saved_x=2622 saved_y=780」と同じ。立った位置 `char_x=2386 char_y=780` も A3r の保存の行の `char_x=2386 char_y=780` と同じ（既定の `default_char_x=2408` ではない）。
  - `session_mark_found` 0 件（A3r がきれいに終わったので、前回のゴーストの記憶で起こした）。
- 終了コード: `0`（`A4r exit=0 end=2026-10-02T22:37:32.9257748+09:00`）。
- ERROR・WARN（区間＝A4r 全体）: ERROR 0 件・WARN 0 件。
- 目視（開発者）: 前回のゴースト・シェル・バルーン・窓の位置で立った。メニューの「終了」で終えた。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「項目 10 はOK、項目 11 もOK」 → 結果 **合格**。

### 項目 11 表示の拡大率（A1r〜A4r）

- 区間を持たない（§5.2）。拡大率は §2 のとおり主の画面 200%（DPI 192）で、一周の途中で変えていない。記録でも、窓を作るたびに出る起動時の拡大の行の `primary_dpi=` は E2r・A1r・A2r・A3r・A4r のすべてで `192`（E2r 1・A1r 1・A2r 5・A3r 3・A4r 1 行）。A4r の 1 行（逐語）:
  - `2026-10-02T13:37:16.576396Z  INFO areka::placement: placement: 起動時 k₀ を導出（primary モニタ DPI ÷ 作者基準 DPI・D7） primary_dpi=192 shell_author_dpi=96 balloon_author_dpi=96 k_shell=2.0 k_balloon=2.0 k_shell_ratio=ScaleRatio { num: 2, den: 1 } k_balloon_ratio=ScaleRatio { num: 2, den: 1 }`
- 目視（開発者）: 項目 3・12・4・5・7・6・7 の付記・13・8・9・10 の目視のたびに、絵・当たり判定・窓の大きさ・バルーンの位置が崩れていなかった（上の各項目の判定の返信と、次の返信）。
- 根拠の種別: 目視（拡大率の値は §2 の読み取りと記録の引用）。
- 開発者の判定（逐語）: 「項目 10 はOK、項目 11 もOK」 → 結果 **合格**。

## 8. 登記（要件 3.6・3.8・5.6）

判定に載せない既知の症状・確かめられなかった既知の制限の候補・欠陥と開発者の判断をここに書く。8.1 はタスク 2.5（zip を組む前）で書いた。走行で見つかるもの（3.6 の除外で済まない症状・欠陥・強制終了で残った子のプロセス）はタスク 4.1〜4.4 で書き足す（8.2 は説明書の突き合わせで使ったので、走行のものは 8.3 から）。

### 8.1 確かめられなかった既知の制限の候補（要件 5.6）

申し送られた既知の制限の候補のうち、署名の根拠にするコミットで確かめられないものを、推測で説明書へ書かずにここへ登記する。今わかっている候補は次の 2 つ（design.md「受入記録」の §8 の行）。2 つとも完了 `ghost-install` の `design.md` の「Open Questions / Risks」の 1・2 から申し送られた（同じ設計の「申し送り」の行が、この 2 つを本仕様の既知の制限の候補として渡している）。

| 候補 | 何が分からないか | 確かめられなかった理由 | 扱い |
|---|---|---|---|
| ⑴ 表示中のシェル・使用中のバルーンのフォルダを上書きするインストール | シェルかバルーンの `.nar` を、いま表示しているシェル・いま使っているバルーンと同じフォルダへ入れたとき、areka 自身か SHIORI がそのフォルダのファイルを開いたまま掴んでいて入れ替えに失敗するか（失敗すれば宛先は元のまま・`OnInstallFailure`。完了 `ghost-install` の要件 7.8） | ⒜ ソースからは決められない: 完了 `ghost-install` の設計が「掴んでいるかはソースからは決められない」と書いたとおりで、シェルとバルーンの宛先はゴーストを降ろさずに入れる道（`crates/areka/src/install/judge.rs` の `destination_of`＝種類が `Shell`・`Balloon` なら `Destination::Elsewhere`）を通るため、掴んでいれば失敗し、掴んでいなければ通る。⒝ 実機の記録が無い: 完了 `ghost-install` の `signoff.md` の項目 3 が確かめたのは、起動中のゴーストを降ろしてから入れる道（`install_overwrite_done ok=true`・所要 277 ms）。このとき宛先の `ghost\emo2`（中のシェル `shell\master` を含む）と、使っていたバルーン `balloon\emo2-kakukaku` にも展開した（`install_done kind=Ghost places=[…ghost\\emo2, …balloon\\emo2-kakukaku]`）が、ゴーストを降ろして窓が 0 枚になった後だった（`ghost_switch_down_ms`・`windows_closed_for_restart closed=4` の後）。表示したまま、降ろさずにシェル・バルーンのフォルダを上書きする道（`Elsewhere`）は踏んでいない。⒞ この一周でも踏まない: 項目 4 で入れるのは新しいゴースト 2 体（`R_POST_and_KOMAINU`・`claudia`）で、表示中のシェル・使用中のバルーンの宛先へ入れる操作は検証項目表に無い | 説明書に書かない（要件 5.6）。完成判定の文書の「持ち越した事項」の表（§6）に「確かめられなかった既知の制限の候補」として載せる（要件 7.4）。引受先（台帳に実在する spec・その時点で起票した spec・開発者の手のどれか）は、タスク 5.1 で `roadmap.md` の台帳の行で確かめてから書く（要件 7.7） |
| ⑵ 起動中のゴーストへ入れる間に Windows を終えたときの後始末 | 起動中のゴーストへ入れる一周（降ろす → 展開 → 起こし直す）の途中で Windows の終了が来たとき、後始末が呼ばれるか。完了 `ghost-install` の設計の見立ては「展開の間は窓が 0 枚で、受け手の窓が無いので後始末が呼ばれずにプロセスが終わらされうる。確定の 2 手の間で断たれると宛先のフォルダは無く、元の中身は作業フォルダに 7 日残る」 | ⒜ 手当ては入れていない（完了 `ghost-install` の設計が「手当ては入れず、実機で所要を測り、申し送る」と決めた）。⒝ 実機で測れたのは所要だけ: 完了 `ghost-install` の `signoff.md` の項目 3 で、降ろしてから展開し終えるまでが 277 ms（`install_overwrite_done ms=277`）。この短い間に Windows の終了を当てる操作は誰も行っておらず、起きたときの後始末の結果（宛先・作業フォルダ・次の起動）の記録は無い。⒞ この一周でも踏まない: 検証項目表に Windows の終了を当てる項目は無く、数百ミリ秒の間に終了を当てる操作は 1 分以内に観測できる操作（要件 2.4）にならない | 説明書に書かない（要件 5.6）。なお、元の中身が作業フォルダに 7 日残ること自体は、入れる途中で元へ戻せなかったときの既知の制限として説明書に書いてある（83 行目・`crates/areka-nar/src/install.rs` の `SURVIVOR_RETENTION`）が、「Windows の終了で後始末が呼ばれない」ことは書かない。完成判定の文書の「持ち越した事項」の表（§6）に載せる（要件 7.4）。引受先は ⑴ と同じくタスク 5.1 で台帳の行で確かめる（要件 7.7） |

### 8.2 説明書の行で確かめられなかったもの（要件 4.7・5.5）

**0 行。** §10.1 の突き合わせで、説明書の主張を持つ 125 行はすべて、本体のソース・zip の中身・完了 spec の記録・本仕様の記録のどれかで確かめられた（確かめられない行を書かずに回す先がここだが、回した行は 0）。

### 8.3 えも？？ が `claudia.nar`（バルーン同梱）を入れた後の `OnInstallComplete` に台本を返さなかった（項目 4 の縮退・2026-10-02）

- 何が起きたか: A2r の項目 4 ⒝ で `claudia.nar`（ゴーストにバルーン 2 つを同梱した書庫）を 2 度入れ、2 度とも `OnInstallComplete`（参照 `["ghost", "悪役令嬢クローディア", "クローディア"]`）が `raised=NoReply`（返事なし・空の台本）だった。同じ走行の ⒜ `R_POST_and_KOMAINU.nar`（同梱なし・参照 `["ghost", "Ｒポストと狛犬", ""]`）では `raised=Script` で、どちらも始まりの `OnInstallBegin` は `raised=Script`。記録の逐語は §7 の項目 4。
- 本書の確かめで分かったこと（原因までは分からなかった）: 根 A の `ghost\emo2\ghost\master\dic\install.pasta` の `SCENE.on_install_complete` は参照の 1 つ目（`^ghost`）だけで分かれ、⒜ も ⒝ もこれを満たす。えも？？ の辞書の記録（根 A の `ghost\emo2\ghost\master\profile\pasta\logs\pasta.log`）にも、初回の起動の保存ファイルが無いという WARN 1 行のほかに誤りの行は無い。観測できた ⒜ と ⒝ の違いは、3 つ目の参照（空か「クローディア」か）と、入れ方（メニューか窓への投げ込みか）の 2 つだけで、辞書・SHIORI の側のどこで台本が返らなかったかは分からない。
- areka の欠陥か: areka は入れる手続きを終え、イベントを送り、返事なしとして続けた（`crates/areka/src/install/procedure.rs` の締めの知らせ）。台詞を返すかはゴーストの辞書の側の決めで、areka の欠陥とはしない。
- 2026-10-01 の状況との違い: 前の zip のえも？？ の辞書にはインストールの場面がそもそも無く、3 つのイベントがどれも `raised=NoReply` だった（§9.1 の ⑴・tasks.md の Implementation Notes の 4.2 の行）。ghost_dev の側で場面を足した版（`75e560e`・`dic/install.pasta`）を今回の zip に入れたので、始まりの台詞と、同梱の無いゴーストの終わりの台詞は出るようになった。残ったのは同梱のある書庫の終わりの台詞だけ。
- 開発者の裁定（2026-10-02・逐語）: 「項目 12 はOK、項目 4 は縮退合格で、emo2 に申し送って。あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」→ 項目 4 は縮退の合格（§7）。
- 引受先の調べ（2026-10-02・セッション「emo2 開発セッション」からの知らせ。本書では確かめていない）: 原因は、ghost_dev `6725b9d` で足した `＊OnInstallCompleteAll` の場面が、イベント名の前方一致の抽選で `OnInstallComplete` の検索にも当たり、`CompleteAll` 側が選ばれると単数のインストールでは台本を返さないこと（およそ半分の確率）。3 つ目の参照や入れ方の違いは関係なく、⒜ が当たり ⒝ が 2 度とも外れた偶然。ghost_dev `4a2ba33` で直した（`release/emo2` は本仕様の署名まで変えないので、今回の zip の `emo2` は直る前の版）。
- 引受先: セッション「emo2 開発セッション」へ 2026-10-02 に申し送った。完成判定の文書の持ち越しの行の引受先の書き方は、タスク 5.1 で決める。

### 8.4 えも？？ の更新の一覧が `emo2-kakukaku/*` をゴーストのフォルダからの相対で載せている（項目 8 で見つけた・2026-10-02）

- 何が起きたか: A3r の項目 8 の 1 回目の更新で、手で変えた `readme.txt` のほかに `emo2-kakukaku/` で始まる 20 件が差分として入り（`Changed(21)`）、`<根 A>\ghost\emo2\emo2-kakukaku\`（ファイル 20 個・フォルダは 2026-10-02 22:27:54 JST に作られた）ができた。areka が使うバルーンの `<根 A>\balloon\emo2-kakukaku\` は更新されていない（中のファイルの時刻は zip のまま）。
- 原因: えも？？ の `updates.txt`（根の `ghost\emo2\updates.txt` と `ghost\emo2\ghost\master\updates.txt`）が、それぞれ 20 行で `emo2-kakukaku/…` をゴーストのフォルダからの相対の名前で載せている（2026-10-02 に根 A の 2 つの `updates.txt` を読み、`emo2-kakukaku/` を含む行がそれぞれ 20 行）。ゴーストの更新はゴーストのフォルダからの相対で書き込むので、余分なフォルダができる。zip の `ghost/emo2/` の下に `emo2-kakukaku` の項目は 0 件で、`balloon/emo2-kakukaku/` の下は 20 項目（2026-10-02 に zip の写しを読むだけで数えた）。完了 `areka-P0-network-update` の `signoff.md` の 1 回目にも「ゴーストの更新で同梱バルーンの複製 `emo2-kakukaku/` をゴーストの中へ取りに行くのは SSP も同じ」とある。
- areka の欠陥か: areka は更新の一覧のとおりに書き込んだ。一覧の中身はゴーストの配布の側の決めで、areka の欠陥とはしない。項目 8 の判定は変えない（開発者の判定「項目 8 はOK、emo2 に申し送って」）。
- 引受先: セッション「emo2 開発セッション」へ 2026-10-02 に申し送った（開発者の返信の「emo2 に申し送って」）。

### 8.5 キャラクターの絵の上の動かさない左クリックで、窓の位置が保存される（観察・判定は変えない）

- 見えたこと（記録の逐語は §7 の各項目）:
  - A1r 13:08:51.371241Z の `char DragEnd 保存 scope=0`（項目 3 の透明な場所のクリックが本体の絵に当たった）
  - A2r 13:20:46.217387Z の `char DragEnd 保存 scope=1`（項目 5 の `claudia` の相方）
  - A2r 13:23:12.861575Z の `char DragEnd 保存 scope=0`（付随の左クリックの後の右クリックの、手順の左クリック）
- 3 件とも窓は動いていない: 本体の 2 件は同じ値（`saved_x=2446 saved_y=610`）で、これを読み戻した A2r・A3r の本体は `char_x=2012 char_y=330` が既定の `default_char_x=2012 default_char_y=330` と同じ位置に立った。相方の 1 件は `char_x=1548 char_y=704` が、直前の `claudia` の起動の既定 `default_char_x=1548 default_char_y=704` と同じ。それでも記憶に位置が書かれ、次の起動からは「既定の配置」ではなく「保存した位置」として読み戻される（A2r の本体の `saved_win_x=Some(2446)`）。保存は掴んで離したときだけ、という設計（§5 の項目 12 の期待の裏付け `drag_follow.rs` の保存の条件）から見ると、動かさない左クリックも「掴んで離した」に数えられている。
- 判定への響き: 無い。A1r の 1 件は本体（scope 0）で、項目 12 は相方（scope 1）で判定した（開発者の返信「A で進めて、A2r 起動しました」）。A2r の 2 件は項目 12 の観測の後。
- 扱い: 欠陥かどうかは決めていない（観察として残す）。引受先（台帳に実在する spec・その時点で起票する spec・開発者の手のどれか）は、タスク 5.1 で開発者に尋ねて決める。

### 8.6 `tools/package-alpha.ps1 -Check` が一時フォルダに展開先を残す（候補・2026-10-02）

- 見えたこと: `-Check` は展開先と記録を `%TEMP%` の下の `areka-alpha-check-*`（例 §1 の `areka-alpha-check-215146`・`areka-alpha-check-215146-logs`）に作り、終わった後も残す。2026-10-02 に開発者の求め（逐語「あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」）で、AI が `%TEMP%` の下の `areka-alpha-check-*` のフォルダ 38 個を消した（今回の 2 回の `-Check` の記録は、消す前に生の記録の置き場へ `check-215146-logs\`・`check-221148-logs\` として写してあった・§1）。
- 開発者の決まり（一時フォルダはワークツリーの `target\` の下だけ）との関係: `-Check` の展開先は本仕様より前からの `tools/package-alpha.ps1` の作りで、一周の根（§4・`target\` の下）とは別。
- 扱い: 直すかどうか・引受先は、タスク 5.1 で開発者に尋ねて決める（候補）。署名の根拠の zip と `-Check` の判定（§1.1）は変えない。

### 8.7 強制終了で残った子のプロセス（§6.4）

**0 件。** A2r の強制終了で、止める前に書き留めた子は `shiori-host32-helper.exe`（pid 27964）の 1 つで、5 秒後には残っていなかった（`runs.txt` に `child-left` の行が 0 件）。止めたプロセスは A2r の `areka.exe`（pid 4252）の 1 つだけ。

### 8.8 同梱インストールの `install.txt` の読み方が ukadoc と違う 4 点（候補・2026-10-02）

- 見つけた経緯: セッション「pasta棚卸」（`pasta_check` の同梱バルーンの対応）から、areka の `install.txt` の読み方の問い合わせがあり、その答えと ukadoc を突き合わせて見つかった（areka の側は本体のソース、ukadoc の側は「Install設定」の頁 https://ssp.shillest.net/ukadoc/manual/descript_install.html を 2026-10-02 に読んだ）。
- 違い:
  - ⑴ 番号付きの同梱: ukadoc「探索は無印→0→1→2…の順に行われ、見つからない番号が出た時点で打ち切られる。」。areka は欠番で打ち切らず、`balloon` の直後が数字だけの接頭辞をすべて読み、並びは接頭辞のバイト順（`crates/areka-nar/src/manifest.rs` の `numbered`・`classify`）。
  - ⑵ `*.source.directory` の階層付きの値: ukadoc「SSP 2.9.00以降は、extra\bal1 のようにアーカイブ内の階層を辿る相対パスも指定できる。」。areka は 1 階層の名前の検査（`crates/areka-nar/src/names.rs` の `is_valid_one_level_name`）に掛け、インストール全体を拒否する（`InvalidDirectoryName`）。
  - ⑶ `*.directory` のパス区切り: ukadoc「パス区切りは使えない（「_」に置換される）。」。areka は同じ検査で拒否する。
  - ⑷ `*.source.directory` の `..`: ukadoc「「..」による上位階層への参照はできない（取り除かれる）。」。areka は拒否する。
- この一周への影響: 無い。2026-10-02 に 3 つの検体の `.nar` の `install.txt` を読んだ。`claudia` は `balloon0.directory`・`balloon1.directory`（欠番なし・0 と 1 の順は ukadoc の探索と areka のバイト順で同じ）、`emo2` は無印の `balloon.directory`・`balloon.source.directory` に 1 階層の名前 `emo2-kakukaku`、`R_POST_and_KOMAINU` は同梱なし。欠番・階層付きの値・パス区切り・`..` を使うものは無い。
- 開発者の裁定（2026-10-02・逐語）「どちらも A で進めて」（A＝α では直さず、完成判定の文書の持ち越しに載せる。説明書には書かない）。
- 引受先: α の後の `/kiro-discovery`（spec にするかをそこで決める）。完成判定の文書の持ち越しの行の書き方は、タスク 5.1 で決める。

## 9. 採り直し

該当するときだけ書く（一周の後に zip の中身を変えたとき・要件 1.3）。

### 9.1 2026-10-02: 検体 `emo2.nar` の差し替えと、根の消失による採り直し

- **起きたこと**:
  - ⑴ A2 の項目 4 で、えも？？ の辞書にインストールの場面が無く、インストールの台詞が見えなかった（`OnInstallBegin`・`OnInstallCompleteEx`・`OnInstallComplete` がどれも `raised=NoReply`）。開発者はいったん縮退の合格と裁定し（「こちらは縮退合格としましょう。」）、ghost_dev の側でイベントの台詞を足すよう申し送った。
  - ⑵ A2 は付随の右クリックと項目 13 の前半の前に、安全弁（30 分）で終わった（`event="app_exit" origin=Smoke`・2026-10-01T14:07:15.861719Z）。§6.5 により、未観測の 2 つは「中断」。
  - ⑶ 機械の再起動の後、ワークツリーの `target\` がまるごと消え、根 E・根 A と `konnoyayame` の控えが無くなった（生の記録の置き場は無事）。消えた理由は分からない。
  - ⑷ ghost_dev の側でイベントの台詞を足した版（`75e560e`・md5 `ac23d4479dff69a1edd6d02d153f27f6`・4,591,449 バイト・113 項目）が届き、開発者が今回の α に取り込むと裁定した（逐語「新しい emo2.nar を今回の α に取り込む、targetのまま。targetにテンポラリを置くのは絶対ルールです。掃除漏れが多いのでダメ」）。
- **差し替えの前の確かめ（2026-10-02）**:
  - `install.txt`・`readme.txt`・`shell/master/readme.txt` が前の版とバイトで同じ。
  - `boot.pasta` の `＊起動halt` が 3 件。`emo2-kakukaku/descript.txt` の `homeurl` の行が 0 件。
  - 実行ファイルは `ghost/master/pasta.dll` だけで、`profile/` は無い。
  - 前の版との違いは次の 6 つだけで、メニュー・シェル・descript は同じ。
    - 足したもの: `ghost/master/dic/install.pasta`・`ghost/master/dic/system.pasta`
    - 変えたもの: `boot.pasta`・`boot.lua`・`ghost/master/updates.txt`・`updates.txt`
- **組み直し（2026-10-02 に済んだ）**: 差し替えをコミットした（`1717d29f`）。その後、全体テストの途中で見つかったテストの欠陥を直し（下の「テストが OS の窓を出して止まった件」・`8460506d`）、新しい署名の根拠のコミット `8460506d95054bfc195e884d13f6b469d93e8491` で全体テスト（`-License`）と zip の `-Check` をやり直して、どちらも全段 緑だった（全体テストは `verification/alpha-completion.md` §1・§2、zip は §1）。根 E・根 A は新しい zip の写しから作り直した（根 E は `ghost\` の中を空にした）。`konnoyayame` の控えも `target\nar-samples\manual\konnoyayame\ghost\konnoyayame` に作り直した。
- **テストが OS の窓を出して止まった件（2026-10-02）**:
  - 起きたこと: 全体テストを窓なしで裏で回すと、`areka-ghost` のテスト `invalid_image_returns_err`（DLL でないファイルを `.dll` の名で読ませる）で、Windows が窓「正しくないイメージ」（0xc000012f）を出し、OK が押されるまでテストが止まった。同じ 1 本を窓なしで起こした回で `finished in 17.49s`（窓は画面で見たもので、記録に残るのは所要の 17.49s だけ）。手元の端末から回すと、親のエラーモードを継ぐので窓は出ない。
  - 開発者の判断（逐語）: 「テストがダイアログを出して中断するのはテストとして問題です。これは修正が必要だと判断しますが」
  - 直したもの: コミット `8460506d`（件名「壊れた DLL を読んでも OS の「正しくないイメージ」の窓を出さない」）。DLL を読む本番の 2 か所（`areka-ghost` の `shiori_inproc.rs` の `InProcLibrary::load`・`shiori-host32-helper` の `shiori_proxy.rs` の `ShioriByteProxy::load`）で、読む間だけスレッドのエラーモードに `SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX` を立てて元へ戻す。テスト `invalid_image_returns_err` には、読んだ後にスレッドのエラーモードが元へ戻っていることの確かめを足した。壊れた `pasta.dll` を持つゴーストでも、利用者の画面に OS の窓が出なくなる。
  - 直した後: 同じ起こし方で `finished in 0.05s`（このときは同じモジュールの 4 本〔`shiori_inproc::tests::`〕をまとめて回した）。2 回の出力は生の記録の置き場の `badimage-red-1717d29f.log`（1 本・`1 passed`・`finished in 17.49s`）と `badimage-green-worktree.log`（4 本・`4 passed`・`finished in 0.05s`。コミットの前の作業木で、レビューの提案〔窓を止める設定に失敗したら戻す手順も飛ばす〕を取り込む前の形。取り込んだ後のコミット `8460506d` では全体テストで確かめた）。直した後の全体テスト（`test-all-r.log`）では `test shiori_inproc::tests::invalid_image_returns_err ... ok` と `test inproc_e2e_test::i3_load_failure_invalid_image_returns_err ... ok`。
- **全体テストの試み（2026-10-02・記録はすべて生の記録の置き場に残した）**:

  | 記録 | コミット | 日時（日本時間） | 結果と理由 |
  |---|---|---|---|
  | `test-all-r-try1.log`・`test-all-r-try1-meta.txt` | `1717d29f` | 開始 20:40:35（終わりの行なし） | 「x64 ワークスペース全テスト」の段で、例 `collision-probe` を組むときに `` error: linking with `link.exe` failed: exit code: 0xc000026b `` で失敗し、裏で回していた道具の時間の上限に達して終わった |
  | `test-all-r-try2.log`・`test-all-r-try2-meta.txt` | `1717d29f` | 21:04:20 〜 21:12:03・`exit=1` | 「cargo about generate」の段だけが `FAIL  cargo about generate（0 秒・終了コード 101）`（`` error: no such command: `about` ``）、ほかの 6 段は OK。開発者が Rust の環境を作り直した後で `cargo-about` が入っていなかった。開発者が入れ直した |
  | `test-all-r-try3.log`・`test-all-r-try3-meta.txt` | `1717d29f` | 開始 21:22:52 〜 止めた 21:25:28 | 開発者の求めで止めた（OS の窓の件）。meta の行の印は `stopped-by-request` |
  | `test-all-r-try4.log`・`test-all-r-try4-meta.txt` | `1717d29f` | 開始 21:25:45 〜 止めた 21:31:53 | 開発者の求めで止めた（OS の窓を直すため）。meta の行の印は `stopped-by-request(fix dialog)`。止めたプロセスの木は meta の行に書いてある |
  | `test-all-r.log`・`test-all-r-meta.txt` | `8460506d` | 21:42:08 〜 21:51:05・`exit=0` | 全段 緑（`notices_diff=0`・`dirty_after=0`）。これを署名の根拠にする（`verification/alpha-completion.md` §1） |

- **前の zip の同定（置き換えたもの・記録は消さずに残す）**:
  - zip の名前 `areka-alpha-x64-20261001-7f8f4e8.zip`・7,556,348 バイト・sha256 `0b4532e4815e80740edef23a977a8854e4c5a1a79d36661b21a99eae75084b08`（2026-10-02 に写しを読み直して同じ値。隣の `.sha256` の値とも同じ）・項目 146。
  - コミット `7f8f4e8701ab0e0a890f446665ed1fda715881a0`（`BUILD-INFO.txt` の `commit=7f8f4e8`・`built=2026-10-01T13:14:54Z`）。
  - `-Check`: 2026-10-01 22:11:47 〜 22:15:07・`exit=0`・判定 1〜8 すべて合。記録は名前を変えて `package-alpha-7f8f4e8.log`・`package-meta-7f8f4e8.txt` として残した。
  - 全体テスト: 2026-10-01 22:02:51 〜 22:11:23・`exit=0`・全段 緑（`test-all.log`・`test-all-meta.txt`・`cargo-deny.txt`）。
  - この zip で回した走行 E1・E2・A1・A2 の記録（`run-E1.log`〜`run-A2.log`・`runs.txt`）も残す。
- **採り直す項目**:
  - 全項目（1〜13 と付随 2 つ）。項目 1・2 は `emo2` に依らないが、zip が変わるので新しい zip で採り直す。
  - 根は同じ `<ワークツリー>\target\alpha-lap\{E,A}` に作り直す（置き場の決まり: 一時フォルダは `target\` の下だけ）。
  - 走行の名前は前と区別するため `E1r`・`E2r`・`A1r`〜`A4r` とし、前の記録（`run-E1.log`〜`run-A2.log`）は消さずに残す。
  - 2026-10-02 に採り直し終えた（走行 E1r〜A4r・22:05〜22:37 JST・記録は `run-E1r.log`〜`run-A4r.log` と `runs.txt` の `E1r`〜`A4r`・`between`・`A3r item8` の行）。全項目の判定は §7 に書いた。
- **前の zip での結果（判定に使わない）**: 項目 1・2・3 は合だった（記録はすぐ下の「前の zip での走行（判定に使わない）」へ §7 から移した）。A2 の項目 12・4・5・7 は観測したが、採り直すので判定しない。§7 の判定は採り直しの走行だけで行った。

#### 前の zip での走行（判定に使わない）

前の zip（`areka-alpha-x64-20261001-7f8f4e8.zip`）で 2026-10-01 に回した走行 E1・E2・A1 の結果。2026-10-02 の採り直し（§7）で置き換えたので判定に使わない。書いた当時の §7 の文を、逐語の引用ごとそのまま移した（見出しの段だけ下げ、名前に「（前の zip）」を足した）。

##### 項目 1 空の根の告知（E1）（前の zip）

- 区間: `E1 item=1 start=2026-10-01T13:28:13.8938958Z`（22:28:13 JST）〜 `E1 exit=1 end=2026-10-01T22:29:01.9853510+09:00`。pid 11360。
- 記録（`run-E1.log` から逐語）:
  - `2026-10-01T13:28:14.246184Z  INFO areka::boot_config: ベースウェアの根を決めました event="root_resolved" root=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\E source=ExeDir`
  - `2026-10-01T13:28:14.250318Z ERROR areka::alert: [alert] 利用者へ告げます event="alert" scene=GhostMissing { ghost_store: "C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\E\\ghost", argv: None } title="areka を起動できません" body="ゴーストが見つかりません。\n置く場所: C:\\home\\maz\\git\\areka\\.claude\\worktrees\\roadmap-inventory-028436\\target\\alpha-lap\\E\\ghost\n置くもの: ghost\\master\\descript.txt を持つゴーストのフォルダ" suppressed=false`
- 目視: 告知の窓（題「areka を起動できません」・本文 3 行〔「ゴーストが見つかりません。」・置く場所 `…\target\alpha-lap\E\ghost`・置くもの〕・OK）。開発者が撮った画面の写しを `E1-alert.png` として置き場に残した。OK を押して終わった（`exit=` の行）。
- 終了コード 1・標準エラー `Error: Error { code: HRESULT(0x80004005), message: "エラーを特定できません" }`: ゴーストが無いときは告知の後に `E_FAIL` で終わる作り（`crates/areka/src/main.rs` の `resolve_boot` が `Err(scene)` のとき `alert::raise` の後に `E_FAIL` を返す）。期待は「OK で終わる」で、終了コードを定めていないので判定に影響しない。
- 3.6 の除外: 上の `event="alert"` の ERROR 1 行（§6.6 の例・項目 1 の期待どおりの告知）。ほかの ERROR・WARN は 0 件。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「E1 はOK」 → **合**。

##### 項目 2 告知のとおりに置く（E2）（前の zip）

- 区間: `E2 item=2 start=2026-10-01T13:31:05.9643559Z`（22:31:05 JST）〜 `E2 exit=0 end=2026-10-01T22:32:01.6678706+09:00`。pid 15796。根 E の `ghost\konnoyayame` へ、準備で控えた `nar-sample-path konnoyayame` の `folder=`（`target\nar-samples\manual\konnoyayame\ghost\konnoyayame`）を写してから起動した。
- 記録（`run-E2.log` から逐語）:
  - `2026-10-01T13:31:06.032736Z  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Only dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\E\ghost\konnoyayame`
  - `2026-10-01T13:31:06.648626Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None`
  - `2026-10-01T13:31:06.649487Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1`
  - `2026-10-01T13:32:01.602794Z  INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4`
  - `2026-10-01T13:32:01.621565Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"`
- 目視（開発者）: `konnoyayame` が立って挨拶する・目の周りに四角い地色が出ない・挨拶の字形が化けない・左上の画素と同じ色の場所のクリックが背後の窓へ抜ける。メニューの「終了」で終えた（終了コード 0）。
- 3.6 の除外: ERROR は 0 件。WARN 6 件を除外した。逐語（`run-E2.log`）:
  - `2026-10-01T13:31:06.179757Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png`
  - `2026-10-01T13:31:06.179996Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png`
  - `2026-10-01T13:31:06.190406Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png`
  - `2026-10-01T13:31:06.190535Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png`
  - `2026-10-01T13:31:06.232382Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 prefix=balloons file=balloons2.png`
  - `2026-10-01T13:31:06.232539Z  WARN areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=3 prefix=balloons file=balloons3.png`
  - 理由: 既定バルーン `StayseeBalloon` が相方用の面 2・3 を持たず、本体側の面へ落ちたことの観測。`crates/areka-emo-present/src/balloon.rs` の `resolve_balloon_faces` の doc が「失敗ではなく正典準拠のフォールバック動作の観測」と定める。先行 spec の記録で判定の外と決まっている（§6.6 の 3 つ目の例）: 完了 `areka-P0-default-balloon-bundle` の `verification/signoff-record.md` §1（相方側の面 2・3 が本体側の `balloons2/3.png` へ縮退し記録 2 件＝期待どおり）・完了 `areka-P0-shell-balloon-switch` の `signoff.md`（同じ WARN を本仕様の範囲外の既存のものとした）。
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「E2 はOK」 → **合**。

##### 項目 3 初回の起動（A1）・項目 12 の 1 回目（前の zip）

- 区間: `A1 item=3 start=2026-10-01T13:33:53.6928849Z`（22:33:53 JST）〜 `A1 exit=0 end=2026-10-01T22:34:51.1511968+09:00`。pid 32716。根 A は署名の zip を新しく展開した（`profile` は展開の直後に無かった）。
- 記録（`run-A1.log` から逐語）:
  - `2026-10-01T13:33:54.030610Z  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Only dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\ghost\emo2`
  - `2026-10-01T13:33:54.043913Z  INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Companion dir=C:\home\maz\git\areka\.claude\worktrees\roadmap-inventory-028436\target\alpha-lap\A\balloon\emo2-kakukaku`
  - `2026-10-01T13:33:54.450700Z  INFO areka::persist::restore: merge_scope restore scope=1 anchor=Bottom saved_win_x=None saved_win_y=None default_char_x=1340 default_char_y=904 char_x=1340 char_y=904 char_w=672 char_h=800 saved_off_x=None saved_off_y=None balloon_off_x=292 balloon_off_y=-150 balloon_x=1632 balloon_y=754`
  - `2026-10-01T13:33:55.114593Z TRACE actor{actor=kanade}: kanade: SHIORI 送出 event="shiori_request" method=GET id=OnFirstBoot references=["0"] status=None`
  - `2026-10-01T13:33:55.118344Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1`
  - `2026-10-01T13:34:47.788015Z  INFO actor{actor=emo-text}: areka::menu::trigger: [menu] shown event="menu_shown" scope=1 items=8`
  - `2026-10-01T13:34:51.038214Z  INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4`
  - `2026-10-01T13:34:51.078541Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"`
- 項目 12 の 1 回目: A1 の記録に「char DragEnd 保存」の行は 0 件（窓を掴まずに終えた）。終えた後の `ghost\emo2\ghost\master\profile\areka\sylphya.toml` は `[boot] count = "1"` と `[last] balloon = "emo2-kakukaku"`・`shell = "master"` だけで、窓の位置の項目は無い。
- 目視（開発者）: えも？？ が `emo2-kakukaku` で立って挨拶する・絵の透明な場所のクリックが背後の窓へ抜ける・拡大率 200% で絵・当たり判定・窓の大きさ・バルーンの位置が崩れない（項目 11）。右クリックのメニューの「終了」で終えた（終了コード 0）。
- ERROR は 0 件。WARN 3 件は項目 3 の判定に使わない（項目 3 の期待は WARN の数を定めない。除外ではなく記録として残す）。どれも検体（えも？？ のシェルの素材・バルーンの定義）の中身から出るもの。逐語（`run-A1.log`）:
  - `2026-10-01T13:33:54.374171Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547`
  - `2026-10-01T13:33:54.498279Z  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547`
  - `2026-10-01T13:33:54.625000Z  WARN actor{actor=emo-text}: areka_emo_text::actor: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="(名前なし)" axis="x" wrap_threshold=254.0 inline_limit=240.0`
- 根拠の種別: 両方（目視と記録の引用）。
- 開発者の判定（逐語）: 「A1 はOK」（終えた後に「A1 終了しました」） → **合**。

---

## 10. 説明書の突き合わせ（要件 4.6・4.7・4.8・5.4・5.5）

対象は `dist/README.txt` の HEAD `0055d2827c711d82afb302bdc4e0d3c4cfe5238d`（タスク 2.4 のコミット・説明書を最後に変えたコミット）の版。作業木のファイルは同じ中身（git の `core.autocrlf=true` で改行が CRLF になっているだけ・`tr -d '\r'` で比べて同じ・UTF-8 の BOM つき）。175 行のうち空行が 50 行、主張を持つ行が 125 行。確かめた日は 2026-10-01。

### 10.1 全行の突き合わせ（要件 4.7・5.5）

空行（3・6・7・9・15・16・18・22・23・25・36・37・39・41・44・47・48・50・57・58・60・62・66・69・70・72・92・93・95・97・99・109・111・113・115・117・120・122・124・126・128・129・131・135・140・145・150・157・164・171 行目の 50 行）は主張を持たないので行を立てない。残りの 125 行を、下の表の「行」の列が 1 行も漏らさず重ならずに覆う（欄の見出し 9 行は 2 行目の表の行にまとめた）。「種別」は、本体のソース（`crates/` の定義）・zip の中身（`tools/package-alpha.ps1` が組む物と、その元の `vendors/sample_ghost/*.nar`）・完了 spec の記録（`.kiro/specs/completed/` の文書）・本仕様の記録（本仕様の設計と git の記録）のどれで確かめたか（添え物として、説明書が出どころに挙げる第三者のサイトを読み直した行には「外部の頁」を足す）。ソースは「何の定義か」で指す（パスは `crates/areka/src/` を略したものがある）。

| 行 | 書いてあること | 確かめた先 | 種別 |
|---|---|---|---|
| 1・2 | 題「areka（α 版）をお使いになる方へ」と下線 | 主張は「α 版」だけ。zip の名前が `areka-alpha-x64-<日付>-<7 桁>.zip`（`tools/package-alpha.ps1` の段「圧縮」） | zip の中身 |
| 8・17・24・38・49・59・71・94・130 | 欄の見出し 9 つ | design.md「説明書 `dist/README.txt`」の欄の並び（起動／終了／右クリックメニュー／.nar の入れ方／更新のしかた／記憶の置き場／既知の制限／同梱しているバルーンについて／同梱物とライセンス）と同じ順 | 本仕様の記録 |
| 4 | デスクトップにキャラクター（ゴースト）を住まわせる Windows 用のアプリ | `tools/package-alpha.ps1` の `$X64`（`x86_64-pc-windows-msvc` だけを組む）と zip の判定 5（`areka.exe` の機種 0x8664）・`.kiro/steering/product.md`（「伺か」のような常駐キャラクターのデスクトップマスコット） | zip の中身 |
| 5 | 2026-10-01 時点の内容・α 版なので変わる | 説明書を書いた 3 つのコミット `86675f08`（2.2）・`bf9782af`（2.3）・`0055d282`（2.4）の日付がどれも 2026-10-01（`git log --date=iso -- dist/README.txt`）＝要件 4.1 | 本仕様の記録 |
| 10 | areka.exe を開く・引数は要らない | `boot_config.rs` の `resolve_boot_from`（引数が無ければ根の列挙と記憶で決める）・zip の最上位の `areka.exe`（`package-alpha.ps1` の判定 2 の許可表 `$allowed`） | 本体のソース・zip の中身 |
| 11 | 同じフォルダの ghost・balloon から選ぶ・バルーン＝せりふを表示する吹き出し | `boot_config.rs` の `resolve_root_from`（根＝`areka.exe` の親のフォルダ）・`crates/areka-ghost/src/catalog.rs` の `BasewareRoot::ghost_store`／`balloon_store`（`<根>/ghost`・`<根>/balloon`）・`$allowed` の `ghost`・`balloon` | 本体のソース・zip の中身 |
| 12 | 初めての起動では えも？？（ghost\emo2）が emo2-kakukaku で立つ | `boot_resolve.rs` の `resolve_ghost`（記憶が無く 1 体だけ＝段 3 唯一）と `resolve_balloon`（段 3 同梱＝`install.txt` の `balloon.directory`）・`emo2.nar` の `install.txt` に `balloon.directory,emo2-kakukaku`・`$allowed` の `ghost/` が `emo2` だけ・`-Check` が初回のバルーンを同梱の `emo2-kakukaku` と判定する | 本体のソース・zip の中身 |
| 13 | 2 回目からは前回のゴーストが、そのゴーストで前回使ったシェルとバルーンで立つ | `boot_resolve.rs` の `read_last_ghost`（App スコープ）・`read_last_balloon` と `decide_boot_shell`（起動するゴーストの Ghost スコープ）・`resolve_ghost`／`resolve_balloon` の段 2（記憶） | 本体のソース |
| 14 | 前回きれいに終わらなかった次は えも？？ で立つ・ghost に えも？？ が無ければほかのゴースト | `boot_config.rs` の `resolve_boot_from`（起動中の印が在れば最後のゴーストの記憶を読まない・`session_mark_found`）→ `boot_resolve.rs` の `resolve_ghost` の段 3 唯一・段 4 既定（`DEFAULT_GHOST_FOLDER`＝`emo2`）・段 5 無作為 | 本体のソース |
| 19 | メニューの「終了」→ 別れの台詞のあとに終わる | `menu/mod.rs` の `request_close`（`CloseReason::User` の終了指示を送る・窓を閉じるのは終了の握手の後）・`app_exit.rs` の `quit_app` | 本体のソース |
| 20 | Alt＋F4 などでも同じく別れの台詞のあとに終わる | `app_exit.rs` の `on_ghost_os_close`（OS の閉鎖要求を、メニューの「終了」と同じ `CloseReason::User` の終了要求にして送る・窓は消さない） | 本体のソース |
| 21 | Ctrl と Shift を押しながら左ダブルクリックで台詞なしですぐ終わる | `input_events/mod.rs` の `on_char_pointer_pressed`（Ctrl・左ダブルクリック・結線の後は Shift も → 強制退避で全窓を閉じる）・`app_exit.rs` の `ExitOrigin` の強制退避 | 本体のソース |
| 26 | 右クリックで出る | `menu/trigger.rs` の `on_char_pointer_released`（キャラクター窓で右ボタンを離したとき） | 本体のソース |
| 27 | 項目は上から 7 つ（ゴースト・シェル・バルーン・ネットワーク更新・インストール…・説明書・終了） | `menu/mod.rs` の `Frame::ORDER`・`menu/captions.rs` の `FRAME_CAPTIONS` の既定名 | 本体のソース |
| 28 | 「ゴースト」: ghost フォルダの一覧から選ぶと別れの台詞のあとに切り替わる・起動できないときは えも？？ に戻る | `menu/ghost_frame.rs`（根の目録の並び）・`emo2_boot/ghost_switch.rs` の `request_ghost_switch`（送り出す側へ `OnGhostChanging`）と `switch_to` → `switch_to_default`（`DEFAULT_GHOST_FOLDER`） | 本体のソース |
| 29 | 「シェル」: 今のゴーストのシェルの一覧・今のゴーストのまま替わる | `menu/shell_frame.rs`（今のゴーストのシェルの目録）・`emo2_boot/shell_balloon_switch.rs` の `request_skin_switch`（ゴーストを降ろさず差し替える） | 本体のソース |
| 30 | 「バルーン」: balloon フォルダの一覧・今のゴーストのまま替わる | `menu/balloon_frame.rs`（根のバルーンの目録）・`request_skin_switch` | 本体のソース |
| 31 | 「ネットワーク更新」: 今のゴースト・シェル・バルーンを配布元の新しい版に | `menu/update_frame.rs`（選ぶと `update/desk.rs` の `update_current` が今の 3 つを受付へ掛ける） | 本体のソース |
| 32 | 「インストール…」: ファイルを選ぶ画面・.nar を選ぶとゴースト・シェル・バルーンなどを入れる | `menu/install_frame.rs`・`install/desk.rs` の `pick_and_submit`・`install/pick.rs`（`GetOpenFileNameW`・フィルタ「書庫 (*.nar;*.zip)」） | 本体のソース |
| 33 | 「説明書」: えも？？ なら ghost\emo2\readme.txt・ファイルが無いゴーストでは選べない | `readme.rs` の `resolve_path`（`readme` キーが無ければ `DEFAULT_README`＝`readme.txt` をゴーストのフォルダの直下で）と `is_available`（`menu/mod.rs` の `readme_item` の `enabled`）・`emo2.nar` の `ghost/master/descript.txt` に `readme` の行が 0・書庫の最上位に `readme.txt`（3,105 バイト） | 本体のソース・zip の中身 |
| 34 | 「終了」: areka を終わる | 19 行目と同じ（`menu/mod.rs` の `close_item` → `request_close`） | 本体のソース |
| 35 | ゴーストが項目の名前を用意していればその名前で出る | `menu/captions.rs` の `FRAME_CAPTIONS`（`ghostrootbutton.caption` ほかを照会）と `CaptionMap` | 本体のソース |
| 40・42・43 | .nar にまとめて配られる・入れ方は「インストール…」から選ぶのと、キャラクターの上へ落とすのの 2 つ | 32 行目の出どころ・`input_events/file_drop.rs` の `sort_drops`（ゴースト窓に落とされた `.nar`／`.zip` で `install.txt` を持つものをインストール対象に振り分ける）・完了 `file-drop` | 本体のソース・完了 spec の記録 |
| 45 | 入れた後は今のゴーストのまま・替えるときは「ゴースト」から・入れ終えたときの台詞で自分から替えるゴーストもある | 完了 `ghost-install` の `requirements.md` 要件 12 の裁定 5・`emo2_boot/ghost_switch.rs` の `record_last_installed`（`lastinstalled` の名前で替える道）・完了 `ghost-install` の `signoff.md` 項目 2（`\![change,ghost,lastinstalled]` で替わった） | 完了 spec の記録・本体のソース |
| 46 | 利用条件のファイルを持つ .nar は入れる前に「はい」「いいえ」・「いいえ」なら入れない | `install/terms.rs` の `TERMS_FILES`（`terms.txt`・`terms.md`）・`install/procedure.rs` の `terms_step`（拒否なら `OnGhostTermsDecline` だけを送って入れない）・完了 `ghost-install` の `signoff.md` 項目 4（「はい」「いいえ」とも合格） | 本体のソース・完了 spec の記録 |
| 51 | 「ネットワーク更新」で今のゴースト・シェル・バルーンをそれぞれの配布元の新しい版に入れ替える | `update/desk.rs` の `update_current`（今の 3 つ）・`update/procedure.rs`（対象ごとに配布元を 1 周） | 本体のソース |
| 52 | 更新の間はゴーストが進み具合を話す（内容はゴーストしだい） | `update/procedure.rs` の `UpdatePorts`（イベントを今のゴーストへ送って応えを待つ）・完了 `network-update` の `signoff.md` | 本体のソース・完了 spec の記録 |
| 53 | 何か入れ替わったときだけ、最後の台詞のあとにいったん引っ込み、同じゴーストが戻って更新成功を話す | `update/procedure.rs`（`TargetEnd::Changed` が 1 つでもあれば `request_reload`）・`crates/areka-kanade/src/schedule/boot.rs` の `boot_root`（`BootOrigin::Updated` は渡された名前＝列の先頭の `OnUpdateComplete` を最初に送る）・完了 `network-update` 要件 5.3 | 本体のソース・完了 spec の記録 |
| 54 | 違いが無ければ引っ込まず更新無しを話して終わる | `update/procedure.rs`（`Changed` が無ければ読み直しを頼まない）・完了 `network-update` の `signoff.md` | 本体のソース・完了 spec の記録 |
| 55 | 配布元（更新先）を持たないものは飛ばす | `update/procedure.rs` の `update_target_skipped`（`reason = "no_homeurl"`） | 本体のソース |
| 56 | どれにも更新先が無いときと更新している間は選べない | `update/desk.rs` の `can_update`（段が `Idle`・3 つの `descript.txt` のどれかに更新先）・`menu/update_frame.rs` | 本体のソース |
| 61・63 | 3 か所に覚える・areka の記憶は areka.exe の隣の profile\areka\ | `boot_config.rs` の `default_app_profile_dir`（exe の親の `profile\areka`） | 本体のソース |
| 64 | ゴーストの記憶は ghost\<ゴースト>\ghost\master\profile\areka\ | `boot_resolve.rs` の `ghost_roots`（`<ゴースト>/ghost/master` に `profile_areka_root`）・`crates/areka-ghost/src/sylphya_wiring.rs` の `profile_areka_root`（`profile/areka`） | 本体のソース |
| 65 | シェルの記憶は ghost\<ゴースト>\shell\<シェル>\profile\areka\ | `crates/areka-ghost/src/runtime.rs` の Shell スコープの根（`mount.shell.dir` に `profile_areka_root`） | 本体のソース |
| 67 | 例: えも？？ のゴーストの記憶は ghost\emo2\ghost\master\profile\areka\ | 64 行目に `emo2` を当てた形 | 本体のソース |
| 68 | 消すと初めての起動と同じ状態に戻る | 記憶が無ければ `resolve_ghost`／`resolve_balloon` は段 2 を飛ばす・`crates/areka-ghost/src/runtime.rs` の `apply_boot_record_gate`（起動回数の記憶 `PersistKey::BootCount` が無ければ `config.first_boot` を真にする＝`OnFirstBoot`） | 本体のソース |
| 73 | areka.exe に署名が無い | `tools/package-alpha.ps1` に署名の段が無い（`signtool`・`Set-AuthenticodeSignature` の語が 0 件） | zip の中身 |
| 74 | Windows 10／11 の 64 ビット版専用 | `$X64` だけを組む・判定 5（`areka.exe` が 0x8664）・完了 `alpha-package` の要件 4.5（Windows 専用） | zip の中身・完了 spec の記録 |
| 75 | 深いフォルダに展開しない・長いと同梱のゴーストが話さなくなる | 完了 `alpha-package` の `research.md` §4.2（展開先のパスが長いと pasta の読み込みが失敗し、接続の失敗を出さずに黙る）・`package-alpha.ps1` の `EXPAND_DIR_MAX_CHARS`＝160 | 完了 spec の記録・zip の中身 |
| 76 | 表現力は えも？？ が普通に動く水準まで | `.kiro/steering/roadmap.md` の M2 のゴールの開発者の指示（「表現力増強は emo2 が普通に動いている水準で一旦よい」） | 本仕様の記録（台帳） |
| 77 | 右クリックメニューは Windows の標準の見た目 | `menu/win32.rs`（`TrackPopupMenuEx` で OS のメニューを出す）・完了 `popup-menu-minimal` | 本体のソース |
| 78 | 複数のゴーストを同時に出せない | `ghost_session.rs` の `GhostSlot`（`Option<GhostSession>` を 1 つだけ持つ置き場） | 本体のソース |
| 79 | ほかのアプリからゴーストへ話しかける仕組み（SSTP）が無い | 10.4 の語の探し（受け口 0） | 本体のソース |
| 80 | SAORI はゴースト自身が読み込むものだけ・本体は読み込まない | 10.4 の語の探し（読み込み口 0） | 本体のソース |
| 81 | キャラクターが 3 人以上のゴーストでは 3 人目からの窓が出ない | `emo2_boot/mod.rs` の `derive_scopes`（`vec![0, 1]` 固定） | 本体のソース |
| 82 | 管理者として起動した areka へふつうの権限のエクスプローラから落としても届かない | 完了 `file-drop` の `requirements.md` 要件 8.9（OS の仕組みで窓に届かない・既知の制限として申し送る） | 完了 spec の記録 |
| 83 | 入れる途中で元へ戻せなかったときは元の中身が areka.exe のフォルダの中の .nar-work に残り、7 日を過ぎた後、次に入れるときに片付く | `crates/areka-nar/src/install.rs` の `WORK`（`.nar-work`・根の直下）と `SURVIVOR_RETENTION`（7 日）と `is_retained`・`install/worker.rs`（根＝ベースウェアの根） | 本体のソース |
| 84 | 更新のオプション（確かめるだけ・試すだけ・やり直し）は受けない・付けて頼まれたら何もしない | `emo2_boot/update_cue.rs` の `UPDATE_OPTIONS`（`checkonly`・`testonly`・`recovery`）と `is_update_option` | 本体のソース |
| 85 | シェル・バルーンの更新先を差し替える仕組みには応えない | `crates/areka-kanade/src/schedule/resources.rs`（`other_homeurl_override` を照会に載せない） | 本体のソース |
| 86 | ゴーストの指示で取ってきて入れる仕組みは .nar だけ | `install/judge.rs`（`url` の種別が `nar` 以外は `ScriptRefusal::UnsupportedKind`） | 本体のソース |
| 87 | 取ってきたファイルは %TEMP%\areka\download\ に残り、7 日より古い物は次に取ってくるときに消す | `install/fetch_url.rs` の `download_dir`（`%TEMP%\areka\download`）と `KEEP`（7 日） | 本体のソース |
| 88 | 更新の途中で元へ戻せなかったときの残りが更新した物のフォルダの中の .update-work に残る・取り出しは手で | `crates/areka-update/src/paths.rs` の `WORK_DIR`（`<対象>/.update-work/`）・本仕様の `brief.md` の 2026-09-30 の申し送り ⑸ | 本体のソース |
| 89 | 引数でゴーストのフォルダを指して始めたときは引っ込んで戻らず、次の起動から効く | `update/desk.rs` の `reload_folder`（フォルダ名が無い＝引数の起動は `argv` で読み直さない） | 本体のソース |
| 90 | 今表示していないシェル・バルーンを更新したときは、次に選んだときに効く | `update/desk.rs` の `reload_folder`（読み直すのは同じゴースト・立つのは今のシェル・バルーン）・本仕様の `brief.md` の 2026-09-30 の申し送り ⑺・タスク 2.3 の Implementation Notes（要件 5.3 を実物に合わせた） | 本体のソース・本仕様の記録 |
| 91 | 引っ込んで戻ったとき「更新成功」の台詞の終わりを待たずにまとめの知らせを送る・両方に応えるゴーストでは置き換わる | 完了 `network-update` の `requirements.md` 要件 5.3（既知の制限として明記）・`update/procedure.rs`（総括を後送りの列の最後へ） | 完了 spec の記録・本体のソース |
| 96 | 地の文の 1 文（emo2-kakukaku は えも？？ の同梱物で「1 つ」に数えない・条件は ◆ バルーン「emo2-kakukaku」の欄） | 指す先の欄が 165 行目に在る・`package-alpha.ps1` の `$allowed` の `balloon/` が `emo2-kakukaku` と `StayseeBalloon` の 2 つ・要件 4.6 | zip の中身 |
| 98・100〜108・110・112・114・116・118・119・121・123・125・127 | 既定バルーンの欄の本文（§6.2 の写し） | 10.2 の一致（完了 `default-balloon-bundle` の `verification/signoff-record.md` §6.2 と一致） | 完了 spec の記録 |
| 132〜134 | ほかの作者の作品が入っている・MIT は本体だけ・それぞれの条件に従う | `package-alpha.ps1` の段「組み立て」（`ghost/emo2`・`balloon/emo2-kakukaku`・`balloon/StayseeBalloon` を入れる） | zip の中身 |
| 136〜139 | areka 本体（areka.exe・shiori-host32-helper.exe）の作者 ekicyou・MIT（LICENSE-MIT）・ライブラリの表示は THIRD-PARTY-NOTICES.md | `Cargo.toml` の `authors`（`ekicyou`）・`LICENSE-MIT`（MIT License・Copyright (c) 2026 ekicyou）・`$allowed` の最上位の `areka.exe`・`shiori-host32-helper.exe`・`LICENSE-MIT`・`THIRD-PARTY-NOTICES.md`（段「謝辞の生成」） | zip の中身 |
| 141〜144 | えも？？ の辞書・スクリプトの作者 えちょ（ekicyou）・利用条件の記載なし・出どころ readme.txt と配布サイト | `emo2.nar` の `readme.txt`（「制作者：えちょ（ekicyou）」・「配布サイト：https://ekicyou.github.io/ghost_dev/emo2/」・利用条件の節が無い）・書庫に `terms.` で始まる項目が 0 | zip の中身 |
| 146〜149 | pasta.dll（32 ビット）の作者 ekicyou・MIT・出どころ pasta の LICENSE と THIRD_PARTY_LICENSES.txt | `emo2.nar` の `ghost/master/THIRD_PARTY_LICENSES.txt`（291,413 バイト・「pasta 自体は MIT License で配布されています（https://github.com/ekicyou/pasta/blob/main/LICENSE）」）・`package-alpha.ps1` の判定 5（`pasta.dll` の機種 0x014c＝32 ビット） | zip の中身 |
| 151〜156 | \0 側「コンフィズリー」の作者 ゆゆぴか・禁止事項 4 つ・出どころ readme.txt（同じ内容が confiserie.txt にも）とサイト | `emo2.nar` の `shell/master/readme.txt`（「作成者：ゆゆぴか」・禁止の 4 行「フリーシェルとしての再配布」「伺か関連物以外での使用」「商用利用」「立ち絵の左右反転」）。`shell/master/confiserie.txt`（50 行）はバイトでは違うが、その 1 行目と 4〜50 行目が `readme.txt` の 8 行目と 11〜57 行目に同じ字で入っている（違いは区切りの行と、`readme.txt` の前後の見出しだけ）＝「同じ内容」は成り立つ。サイトは `readme.txt` の `https://yusyuparo.net/` | zip の中身 |
| 158〜163 | \1 側「City-Pop'n」の作者 大槻・条件の 2 つの引用・抜き出せないこと・出どころ CityPop.txt とサイト | `emo2.nar` の `shell/master/CityPop.txt`（500 バイト・「改変や転用、伺かゴースト以外での使用の一切は自由です。」「使用許可の請求も不要です。」「作成者：大槻」「http://th88.blog.shinobi.jp/」）。書き方は議題 5 の開発者の答え（2026-10-01「A. このままでよい」・tasks.md の Implementation Notes 2.4） | zip の中身・本仕様の記録 |
| 165〜170 | emo2-kakukaku の作者 ekicyou・画像素材 フキダシデザイン・規約の要約・出どころ readme.txt「利用バルーン」と規約の URL | `emo2.nar` の `readme.txt` の「利用バルーン」の節（「制作者：えちょ（ekicyou）」・「画像素材：「フキダシデザイン」（https://fukidesign.com/）より」）・規約の要約は完了 `alpha-package` の `research.md` の作者と条件の表（`https://fukidesign.com/terms` を 2026-09-26 に取得して要約）。加えて、タスク 2.5 のレビューで、レビュー係が 2026-10-01 に規約の頁を読み直し、「1ゲームやアプリに使える素材は20素材まで」・著作権の表記は不要・データの再配布は不可の 3 点が説明書の要約と合うことを確かめた（説明書の「1 つにつき 20 素材まで」の裏付け） | zip の中身・完了 spec の記録・外部の頁 |
| 172〜175 | StayseeBalloon の作者 ぽな・CC0 1.0・出どころ LICENSE・readme.txt・GitHub | `vendors/sample_ghost/StayseeBalloon.nar` の `LICENSE`（「CC0 1.0 Universal」）と `readme.txt`・完了 `default-balloon-bundle` の `signoff-record.md` §6.2（入手元 `https://github.com/ponapalt/StayseeBalloon`） | zip の中身・完了 spec の記録 |

覆い方の確かめ: 上の表の「行」の列の行の数は 9（見出し）＋116（それ以外）＝125 で、主張を持つ行の数 125（175 − 空行 50）と同じ。

### 10.2 §6.2 の写しの一致（要件 4.6）

比べ方（design.md「§6.2 の写し方」の「写せたことの確かめ方」のとおり）:

- 写し元: 完了 `areka-P0-default-balloon-bundle` の `verification/signoff-record.md` の `### 6.2` から `### 6.3` の前までの、`>` で始まる行だけ。各行の先頭の `>` とその後の空白 1 つを落とし、表の見出しの行（`| 項目 | 内容 |`）と区切りの行（`|---|---|`）を除き、行頭の見出しの記号 `#`・行頭の箇条の記号 `- ` を落とし、`<br>`・`**`・`` ` ``・`|` を落とし、すべての空白（全角の空白を含む）を落とす。
- 写し先: 説明書の `■ 同梱しているバルーンについて` の行から `■ 同梱物とライセンス` の前の行まで。地の文の 1 文（`えも？？ に付いてくるバルーン emo2-kakukaku` で始まる行）を除き、行頭の空白を落としてから行頭の `■`・`◆`・`・` を落とし、すべての空白を落とす。
- 両方を行の順につないだ文字の列が同じであること。較正として、説明書の写しの「1 つ入っています」を「2 つ入っています」に変えた（1 文字だけ変えた）写しに同じ比べ方を当てて、同じでないこと。
- 道具: 使い捨ての Python の台本（セッションの一時フォルダの `check_readme.py`・リポジトリには置かない）。

コマンドと出力（逐語・2026-10-01）:

```
$ export PYTHONIOENCODING=utf-8
$ python check_readme.py match .kiro/specs/completed/areka-P0-default-balloon-bundle/verification/signoff-record.md dist/README.txt
framing_lines_removed=1
source_chars=1256 readme_chars=1256 equal=True
calibration altered_line=98 readme_chars=1256 equal=False
```

- 判定: 写し元と写し先は 1,256 文字ずつで**一致した**（`equal=True`）。地の文として除いた行はちょうど 1 行。
- 較正: 98 行目の 1 文字を変えた写しは文字の数が同じ 1,256 のまま**一致しない**（`equal=False`）＝比べ方は 1 文字の違いを拾える。

あわせて、§6.3 の判定 ⑵ の探し方（16 語）を説明書の新しい欄に当てた（design.md の同じ節の「加えて」）:

```
$ P=(-e 'crates/' -e '\.rs' -e '§' -e '要件 ' -e '設計 ' -e 'タスク ' -e 'areka-P0-' -e 'UseSelfAlpha' -e 'use_self_alpha' -e 'build_balloon' -e 'validrect' -e 'wordwrappoint' -e 'descript' -e 'surface' -e 'scope' -e '決定論')
$ awk '/^■ 同梱しているバルーンについて/{f=1} /^■ 同梱物とライセンス/{f=0} f' dist/README.txt | grep -c "${P[@]}"
0
$ F=.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/signoff-record.md
$ awk '/^### 6\.1 /{f=1;next} /^### 6\.2 /{f=0} f' "$F" | grep -c "${P[@]}"
8
```

- 判定: 新しい欄（94〜129 行目の 36 行）で 0 件。較正: 同じ探し方を §6.1 に当てると 8 件（完了 `default-balloon-bundle` の記録の値と同じ）。

### 10.3 説明書の全文の内部の言葉の探し（要件 4.8）

探す語（design.md「説明書の全文の語の探し」）: spec 名の形（`areka-`・`alpha-`・`-signoff`）・crate やソースのファイルの名前（`areka_`・`.rs`〔正規表現 `\.rs\b`〕）・`AREKA_`・`WINTF_`・`RUST_LOG`・`event=`・括弧の外の `On` で始まるイベントの名前（各行から全角 `（…）` と半角 `(…)` の中身を取り除いた後に `\bOn[A-Z][A-Za-z0-9]*`）。同じ使い捨ての台本 `check_readme.py` の `scan`。

HEAD の説明書（コマンドと出力・逐語）:

```
$ git show HEAD:dist/README.txt > readme_head.txt      # HEAD = 0055d2827c711d82afb302bdc4e0d3c4cfe5238d
$ python check_readme.py scan readme_head.txt
spec 名 areka-: 0
spec 名 alpha-: 0
spec 名 -signoff: 0
areka_: 0
.rs: 0
AREKA_: 0
WINTF_: 0
RUST_LOG: 0
event=: 0
括弧の外の On…: 0
total=0
$ python check_readme.py scan dist/README.txt | tail -1
total=0
```

- 判定: HEAD の説明書の全文で **0 件**（作業木のファイルでも 0 件）。

較正（仕上げる前の説明書を git の実物から取り出して当てる・`bbf9a620` はタスク 2.1 のコミットで、説明書はタスク 2.2 より前の版）:

```
$ git show bbf9a620:dist/README.txt > readme_pre.txt
$ python check_readme.py scan readme_pre.txt
spec 名 areka-: 0
spec 名 alpha-: 1
    57: （未記入: alpha-release-signoff が仕上げます）
spec 名 -signoff: 1
    57: （未記入: alpha-release-signoff が仕上げます）
areka_: 0
.rs: 0
AREKA_: 0
WINTF_: 0
RUST_LOG: 0
event=: 0
括弧の外の On…: 0
total=2
```

- 較正: 仕上げる前の版の 57 行目（「■ .nar の入れ方」の spec 名）に当たった（2 件・同じ 1 行を 2 つの語が拾った）＝探し方は spec 名を拾える。

### 10.4 既知の制限の裏付けの語の探し（要件 5.1・5.5・SSTP と SAORI）

79・80 行目の「受け口が無い」「読み込み口が無い」を、`crates/` の Rust のソースを語で探して確かめた（2026-10-01・HEAD `0055d282`）。

```
$ grep -rn "SSTP" crates --include=*.rs
crates/areka/src/input_events/user_break.rs:98:/// SSTP に関する判定は 1 つも置かない（要件 5.8）。
crates/shiori-host32-host/src/shiori3.rs:78:/// 二重 CRLF）で示す（要件 1.3）。`SenderType` / `SecurityOrigin` / `X-SSTP-PassThru`
$ grep -rni "saori" crates --include=*.rs
crates/areka/src/session_end.rs:56:/// - 理屈の上で残る唯一の輪は、補助プロセスの中の SHIORI・SAORI が `OnClose` の処理中などに
$ grep -rnE "TcpListener|9801|9821|9811" crates --include=*.rs
crates/areka-update/src/winhttp_real_tests.rs:16:use std::net::{TcpListener, TcpStream};
crates/areka-update/src/winhttp_real_tests.rs:34:    let listener = TcpListener::bind("127.0.0.1:0").expect("待受を立てられる");
```

- SSTP: 当たった 2 行はどちらも注記（中断の判定に SSTP の判定を置かないこと・SHIORI のリクエストの見出しの名前の列挙）で、受け口ではない。待受（`TcpListener`・SSTP の既定の番号）を探すと、当たるのは更新の取得のテストが自分で立てる待受だけ（テストのファイル）。大文字小文字を区別しない探し（`grep -rni "sstp"`）で増える行は、バルーンの絵のファイルの名前（`sstp.png`）・バルーンの設定の名前（`sstpmessage.font.*`）・メニューの名前の資源の名前（`*sstp*button.caption`）・ukadoc の頁の名前（`spec_sstp`）で、どれも受け口ではない。
- SAORI: 当たった 1 行は終了の待ちについての注記（補助プロセスの中の SHIORI・SAORI が固まる場合）で、読み込み口ではない。本体が DLL を読み込むのは `crates/areka-ghost/src/shiori_inproc.rs` の `LoadLibraryW`（SHIORI の DLL）だけ。
- 較正: 同じ探し方が、既知の 1 行（設計の段で見つけていた `user_break.rs` の注記・`session_end.rs` の注記）にそれぞれ当たった＝探し方は語を拾える。

### 10.5 条件つきの制限 ⒜⒝⒞ を書かなかった根拠（要件 5.4）

| 行 | 書く条件 | 確かめた結果（2026-10-01） | 説明書 |
|---|---|---|---|
| ⒜ | zip の `emo2` が `halt` の台詞を持たない | `vendors/sample_ghost/emo2.nar`（md5 `3f5d8777deeeb91fecc587c9071ded32`。2026-10-02 に差し替えた `ac23d4479dff69a1edd6d02d153f27f6` の版でも同じく 3 件）の `ghost/master/dic/boot.pasta` に `＊起動halt` が 3 件。較正: 差し替える前の版（`git show bbf9a620^:vendors/sample_ghost/emo2.nar`）では 0 件 | 書かない |
| ⒝ | zip の `balloon/emo2-kakukaku/descript.txt` に `homeurl` の行がある | 同じ書庫の `emo2-kakukaku/descript.txt` に `homeurl` で始まる行が 0 件（同じ探し方で `name` の行は 1 件）。較正: 差し替える前の版では 1 件 | 書かない |
| ⒞ | バグ `balloon-reappear-short-talk` が未着地 | §0.4 の判定「着地した」（`.kiro/specs/completed/areka-P0-balloon-reappear-short-talk/` が在る） | 書かない |

説明書の「既知の制限」（71〜91 行目）を `halt`・`1 文字`・`古い更新先`・`emo2-kakukaku の更新` で探して 0 件（⒜⒝⒞ の行が無い）。

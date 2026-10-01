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

| 欄 | 値 |
|---|---|
| zip の名前 | `areka-alpha-x64-20261001-7f8f4e8.zip`（名前に `-dirty` が付いていない。x64 の 1 種だけで、arm64 版は組んでいない＝要件 1.5） |
| 組んだコマンド | `pwsh -NoProfile -File tools/package-alpha.ps1 -Check`（`-CheckDir` は付けていない＝既定の一時フォルダへ展開） |
| `commit=` | `7f8f4e8`（zip の中の `BUILD-INFO.txt`） |
| 完全なコミットの識別子 | `git rev-parse 7f8f4e8` → `7f8f4e8701ab0e0a890f446665ed1fda715881a0`（件名「受入記録 §10 に説明書の突き合わせと機械の確かめを書き §8 に候補 2 つを登記する (2.5)」）。全体テストを回した署名の根拠のコミット（`verification/alpha-completion.md` §1）と同じ |
| `dirty=` | `0`（`BUILD-INFO.txt`）。組む前の `git status --porcelain` も 0 行（`package-meta.txt` の `dirty=0`・`package-alpha.log` の「コミット 7f8f4e8・未コミットの変更 0 件」） |
| `-Check` の終了コード | `0`（`package-meta.txt` の `exit=0`・`package-alpha.log` の末尾「全段 緑」） |
| `-Check` の日時（日本時間） | 開始 2026-10-01 22:11:47 〜 終了 2026-10-01 22:15:07（`package-meta.txt`）。zip を組んだ時刻は `BUILD-INFO.txt` の `built=2026-10-01T13:14:54Z`（日本時間 22:14:54） |
| zip の sha256 | `0b4532e4815e80740edef23a977a8854e4c5a1a79d36661b21a99eae75084b08`（生の記録の写しと `target\alpha\` の実物で同じ値） |
| zip の大きさ | 7,556,348 バイト |
| 生の記録の置き場 | `C:\home\maz\lap-records\alpha-signoff-20261001\`（zip の写し・`areka-alpha-x64-20261001-7f8f4e8.zip.sha256`・`package-alpha.log`・`package-meta.txt`。3.1 の `test-all.log`・`test-all-meta.txt`・`cargo-deny.txt` も同じ置き場） |
| `-Check` の展開先と記録 | 展開先 `C:\Users\maz-o\AppData\Local\Temp\areka-alpha-check-221148`・記録 `C:\Users\maz-o\AppData\Local\Temp\areka-alpha-check-221148-logs\run.log`（と `run.stderr.log`）。リポジトリの外で、一周の根には使わない |

`BUILD-INFO.txt` の全文（逐語）:

```
commit=7f8f4e8
dirty=0
built=2026-10-01T13:14:54Z
script=tools/package-alpha.ps1 1.0.0
rustflags=-C target-feature=+crt-static
```

### 1.1 `-Check` の判定（`package-alpha.log` から逐語）

```
判定 1〜8 すべて合
```

```
合 番犬で止めていない（自分で終わった）
合 有界で走った（「smoke 自動 close ゲート有効」1 件）
合 終了コード 0（終了コード 0）
合 ゴーストの窓が立った（「本物のゴースト窓を開きました」1 件）
合 SHIORI の接続の失敗が無い（失敗の目印 0 件）
合 会話が始まった（「起動グリーティングを再生起動」1 件（自動終了より前）・全体 1 件）
合 初回のバルーンは同梱（バルーンを決めました event="balloon_resolved" route=Companion dir=C:\Users\maz-o\AppData\Local\Temp\areka-alpha-check-221148\balloon\emo2-kakukaku）
```

### 1.2 zip の実物で確かめたこと（要件 1.4・1.5・4.9・5.4）

生の記録の置き場の zip の写しを、使い捨ての Python（標準の `zipfile`・リポジトリには置かない）で読んで確かめた（2026-10-01）。

| 確かめ | 結果 |
|---|---|
| `README.txt` が仕上げた説明書と同じ（判定 8） | zip の `README.txt` と `dist/README.txt` がバイト列で一致 |
| 最上位の中身 | `BUILD-INFO.txt`・`LICENSE-MIT`・`README.txt`・`THIRD-PARTY-NOTICES.md`・`areka.exe`・`balloon/`・`ghost/`・`shiori-host32-helper.exe`（項目は全部で 146） |
| `ghost/` が `emo2` だけ（要件 1.4） | `ghost/` の下のフォルダは `emo2` の 1 つ |
| `balloon/` の下 | `StayseeBalloon`・`emo2-kakukaku` の 2 つ |
| x64 の 1 種（要件 1.5） | `areka.exe` の PE の機種 `0x8664`（x64）。`shiori-host32-helper.exe` と `ghost/emo2/ghost/master/pasta.dll` は `0x014c`（32bit の SHIORI を読む補助と、えも？？ の SHIORI。zip の種類を増やすものではない） |
| ⒜ の条件（`emo2` が `halt` の台詞を持つか） | `ghost/emo2/ghost/master/dic/boot.pasta` に `＊起動halt` を含む行が 3 件＝持つ。2.3 の判断（⒜ を書かない）と合う |
| ⒝ の条件（`emo2-kakukaku` の `homeurl`） | `balloon/emo2-kakukaku/descript.txt`（UTF-8）に `homeurl` で始まる行が 0 件（同じ探し方で `name,` の行は 1 件）。2.3 の判断（⒝ を書かない）と合う |

## 2. 機械の構成

タスク 4.1 で書く（一周の前）。

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
- 生の記録の置き場に置くもの: zip の写しと `areka-alpha-x64-20261001-7f8f4e8.zip.sha256`・`package-alpha.log`・`package-meta.txt`（3.2）・`test-all.log`・`test-all-meta.txt`・`cargo-deny.txt`（3.1）・`runs.txt`・`run-E1.log`〜`run-A4.log` と各 `.err.log`（4.x）。

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
$ZIP = 'C:\home\maz\lap-records\alpha-signoff-20261001\areka-alpha-x64-20261001-7f8f4e8.zip'   # 署名の zip の写し（target\alpha の古い zip と取り違えない）
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

タスク 4.1〜4.4 で書く（走行の後）。

## 8. 登記（要件 3.6・3.8・5.6）

判定に載せない既知の症状・確かめられなかった既知の制限の候補・欠陥と開発者の判断をここに書く。8.1 はタスク 2.5（zip を組む前）で書いた。走行で見つかるもの（3.6 の除外で済まない症状・欠陥・強制終了で残った子のプロセス）はタスク 4.1〜4.4 で 8.2 以降に書き足す。

### 8.1 確かめられなかった既知の制限の候補（要件 5.6）

申し送られた既知の制限の候補のうち、署名の根拠にするコミットで確かめられないものを、推測で説明書へ書かずにここへ登記する。今わかっている候補は次の 2 つ（design.md「受入記録」の §8 の行）。2 つとも完了 `ghost-install` の `design.md` の「Open Questions / Risks」の 1・2 から申し送られた（同じ設計の「申し送り」の行が、この 2 つを本仕様の既知の制限の候補として渡している）。

| 候補 | 何が分からないか | 確かめられなかった理由 | 扱い |
|---|---|---|---|
| ⑴ 表示中のシェル・使用中のバルーンのフォルダを上書きするインストール | シェルかバルーンの `.nar` を、いま表示しているシェル・いま使っているバルーンと同じフォルダへ入れたとき、areka 自身か SHIORI がそのフォルダのファイルを開いたまま掴んでいて入れ替えに失敗するか（失敗すれば宛先は元のまま・`OnInstallFailure`。完了 `ghost-install` の要件 7.8） | ⒜ ソースからは決められない: 完了 `ghost-install` の設計が「掴んでいるかはソースからは決められない」と書いたとおりで、シェルとバルーンの宛先はゴーストを降ろさずに入れる道（`crates/areka/src/install/judge.rs` の `destination_of`＝種類が `Shell`・`Balloon` なら `Destination::Elsewhere`）を通るため、掴んでいれば失敗し、掴んでいなければ通る。⒝ 実機の記録が無い: 完了 `ghost-install` の `signoff.md` の項目 3 が確かめたのは、起動中のゴーストを降ろしてから入れる道（`install_overwrite_done ok=true`・所要 277 ms）。このとき宛先の `ghost\emo2`（中のシェル `shell\master` を含む）と、使っていたバルーン `balloon\emo2-kakukaku` にも展開した（`install_done kind=Ghost places=[…ghost\\emo2, …balloon\\emo2-kakukaku]`）が、ゴーストを降ろして窓が 0 枚になった後だった（`ghost_switch_down_ms`・`windows_closed_for_restart closed=4` の後）。表示したまま、降ろさずにシェル・バルーンのフォルダを上書きする道（`Elsewhere`）は踏んでいない。⒞ この一周でも踏まない: 項目 4 で入れるのは新しいゴースト 2 体（`R_POST_and_KOMAINU`・`claudia`）で、表示中のシェル・使用中のバルーンの宛先へ入れる操作は検証項目表に無い | 説明書に書かない（要件 5.6）。完成判定の文書の「持ち越した事項」の表（§6）に「確かめられなかった既知の制限の候補」として載せる（要件 7.4）。引受先（台帳に実在する spec・その時点で起票した spec・開発者の手のどれか）は、タスク 5.1 で `roadmap.md` の台帳の行で確かめてから書く（要件 7.7） |
| ⑵ 起動中のゴーストへ入れる間に Windows を終えたときの後始末 | 起動中のゴーストへ入れる一周（降ろす → 展開 → 起こし直す）の途中で Windows の終了が来たとき、後始末が呼ばれるか。完了 `ghost-install` の設計の見立ては「展開の間は窓が 0 枚で、受け手の窓が無いので後始末が呼ばれずにプロセスが終わらされうる。確定の 2 手の間で断たれると宛先のフォルダは無く、元の中身は作業フォルダに 7 日残る」 | ⒜ 手当ては入れていない（完了 `ghost-install` の設計が「手当ては入れず、実機で所要を測り、申し送る」と決めた）。⒝ 実機で測れたのは所要だけ: 完了 `ghost-install` の `signoff.md` の項目 3 で、降ろしてから展開し終えるまでが 277 ms（`install_overwrite_done ms=277`）。この短い間に Windows の終了を当てる操作は誰も行っておらず、起きたときの後始末の結果（宛先・作業フォルダ・次の起動）の記録は無い。⒞ この一周でも踏まない: 検証項目表に Windows の終了を当てる項目は無く、数百ミリ秒の間に終了を当てる操作は 1 分以内に観測できる操作（要件 2.4）にならない | 説明書に書かない（要件 5.6）。なお、元の中身が作業フォルダに 7 日残ること自体は、入れる途中で元へ戻せなかったときの既知の制限として説明書に書いてある（83 行目・`crates/areka-nar/src/install.rs` の `SURVIVOR_RETENTION`）が、「Windows の終了で後始末が呼ばれない」ことは書かない。完成判定の文書の「持ち越した事項」の表（§6）に載せる（要件 7.4）。引受先は ⑴ と同じくタスク 5.1 で台帳の行で確かめる（要件 7.7） |

### 8.2 説明書の行で確かめられなかったもの（要件 4.7・5.5）

**0 行。** §10.1 の突き合わせで、説明書の主張を持つ 125 行はすべて、本体のソース・zip の中身・完了 spec の記録・本仕様の記録のどれかで確かめられた（確かめられない行を書かずに回す先がここだが、回した行は 0）。

## 9. 採り直し

該当するときだけ書く（一周の後に zip の中身を変えたとき・要件 1.3）。

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
| ⒜ | zip の `emo2` が `halt` の台詞を持たない | `vendors/sample_ghost/emo2.nar`（md5 `3f5d8777deeeb91fecc587c9071ded32`）の `ghost/master/dic/boot.pasta` に `＊起動halt` が 3 件。較正: 差し替える前の版（`git show bbf9a620^:vendors/sample_ghost/emo2.nar`）では 0 件 | 書かない |
| ⒝ | zip の `balloon/emo2-kakukaku/descript.txt` に `homeurl` の行がある | 同じ書庫の `emo2-kakukaku/descript.txt` に `homeurl` で始まる行が 0 件（同じ探し方で `name` の行は 1 件）。較正: 差し替える前の版では 1 件 | 書かない |
| ⒞ | バグ `balloon-reappear-short-talk` が未着地 | §0.4 の判定「着地した」（`.kiro/specs/completed/areka-P0-balloon-reappear-short-talk/` が在る） | 書かない |

説明書の「既知の制限」（71〜91 行目）を `halt`・`1 文字`・`古い更新先`・`emo2-kakukaku の更新` で探して 0 件（⒜⒝⒞ の行が無い）。

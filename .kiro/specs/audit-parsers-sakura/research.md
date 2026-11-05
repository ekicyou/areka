# ギャップ分析: audit-parsers-sakura

> 2026-11-05・HEAD `3bb2ad81`（main `88eb0a43` の上に spec の初期化だけ）。物差しは ponytail 5.3.2 の `/ponytail:ponytail-audit`。受け持つ範囲（`crates/areka-parsers/`・`crates/areka-sakura/`）だけにかけた。
> brief の決めに従い、audit の作法のうち 2 つを外した＝**「最大 20 件」は外して全件を出す**・**「読まなかった所（Not checked）」は残さない（116 ファイルを全部読んだ）**。
> 読み方: 範囲を 6 つ（parsers の balloon・charset・kv／package／sakura／shell、sakura の compile ほか／drive・strict・sysvar）に分け、読むだけの下請け 6 本に全行を読ませた。出た指摘はここでまとめて重なりを除き、Must fix の全件と Should fix の主な件を、この段でコードの行に当てて確かめた。確かめられなかった 1 件は落とした（末尾の「落とした指摘」）。

## 1. この範囲がしていること（What this repo does）

- `areka-parsers` は、ゴースト・シェル・バルーンの設定ファイル（`descript.txt`・`surfaces.txt`）とさくらスクリプトを読み、型のついたモデルにする。文字コードの判定（charset）と、`キー,値` の分割（kv）も持つ。I/O を持つのは `package`（ゴーストのフォルダからの SHIORI とシェルの置き場の解決）だけ。
- `areka-sakura` は、読んだスクリプトを絶対時刻の台本（dola の CueSheet）へ組み（compile）、1 トークを 1 本のアクターで再生する（drive）。MCP の厳しい確かめ（strict）とシステム変数の展開（sysvar）も持つ。
- **想定した負荷**: 1 台の PC で 1 人が使う。ただし入力は**信用できない**（ネットから入れたゴースト・シェル・バルーンが書いたファイルと、SHIORI が返すスクリプト）。スクリプトは数十 KB を超えうる（記憶「ゴーストの台本の大きさを甘く見ない」）。時刻は正確に扱う（記憶「時刻は正確に扱う」）。

## 2. 読んだファイル（全 116 ファイル・漏れ 0）

| まとまり | ファイル数 | 行数 |
|---|---|---|
| parsers: `Cargo.toml`・`lib.rs`・`sample_test_support.rs`・`balloon/`・`charset/`・`kv/` | 23 | 4,087 |
| parsers: `package/` | 10 | 2,843 |
| parsers: `sakura/` | 23 | 5,358 |
| parsers: `shell/` | 30 | 6,017 |
| sakura: `Cargo.toml`・`lib.rs`・`anchor_pair*`・`cluster`・`compile*`・`contract`・`duration`・`error` | 17 | 4,003 |
| sakura: `drive*`・`strict*`・`sysvar*` | 13 | 4,788 |
| **計** | **116** | **27,096**（`git ls-files` の `wc -l` と一致） |

- 先送りの印: `#[allow(dead_code)]` は `charset/model.rs:34` の 1 件だけ（理由はもう古い＝指摘 22）。`shortcut:`・`ponytail:`・`TODO`・`FIXME` は範囲の中に 0 件。
- `doc/`・`docs/` を名指す行は要件 4.5 の 3 行だけ（`strict.rs:5`・`:15`、`sysvar.rs:44`）。**どの指摘の直しも、この 3 行を変えずに済む**（要件 4.6 の議題は無し）。

## 3. 指摘の一覧（全 93 件・番号は audit の通し番号）

組の名は audit のもの（**Must fix**＝必ず直す・**Should fix**＝直したほうがよい・**Nice to have**＝できれば）。各件の「残す 3 種」は brief の Constraints の ①裁定に反する ②わざと置いた先送りの印 ③SSP 互換・ukadoc の正典、に当たる見込みを書く（「—」は当たらない）。「外」は直しが及ぶ範囲の外のファイル、【C17】は同じウェーブで走る見直しの spec の持ち物。

### Must fix（8 件）

**1. ゴーストの descript のシェル名で、ゴーストのフォルダの外を指せる**（`package/resolve_shell.rs:120-132`）
- 説明: `seriko.defaultsurfacedirectoryname` の値をそのまま `ghost_root/shell/<値>` につなぎ、在るかどうかしか見ない。
- 失敗: `seriko.defaultsurfacedirectoryname,..\..\..\Windows` や `C:\Users\Public`（絶対パスは `join` で全部置き換わる）で `Ok` が返り、その先の `descript.txt`・`surfaces.txt`・画像を読む。同じ検査は記憶の名の側だけにある（`crates/areka/src/boot_resolve.rs:414-421`「シェル名つきの解決は名前を検査しないので、ここが `shell/` の外を指す名前を止める」）。
- 直しの方向: 値が `Component::Normal` 1 つのときだけ使い、ほかは既定の `master` へ落として `warn!`。`boot_resolve.rs` の検査をこの共有の関数へ寄せれば 1 か所になる。`..` と絶対パスのテストを 1 本ずつ。
- 外: 要らない（寄せるなら `crates/areka/src/boot_resolve.rs:414-421` を消せる）。
- 残す 3 種: — （ukadoc は値を「ディレクトリ名」と定める＝直しは正典どおり。下の 4 節）。

**2. 空の `seriko.defaultsurfacedirectoryname,` で、`master` でなく `shell\` そのものを置き場にする**（`package/resolve_shell.rs:120-127`）
- 説明: `parse_kv` は空の値を残す。`""` をつなぐと `shell\` になり、`is_dir()` を通る。`.` も同じ。
- 失敗: `shell/master` が在っても `shell\surfaces.txt` を探して起動に失敗し、着せ替えも黙って空。目録の側は「空の値は無しと同じ」と裁定済み（`crates/areka-ghost/src/catalog.rs:439-440`）。
- 直しの方向: 1 と同じ検査に含める（空・1 段でない値は無しと同じ）。テスト 1 本。
- 外: 無し。 残す 3 種: — （ukadoc の既定値は `master`）。

**3. `\n[…]` が `NaN`・`inf`・`1e40` を読み、レイアウトが非有限の位置になる**（`sakura/decode.rs:454-466`）
- 説明: `s.parse::<f32>()` は `NaN`・`inf`・`infinity`・`1e40`（→inf）を受ける。
- 失敗: `\n[inf]あ` で以降の字の位置が inf/NaN になる（`crates/areka-emo-text/src/layout_scan.rs:204`・`:359` が足すだけ）。`NewLineRatio(NaN)` は自分と等しくない。
- 直しの方向: `Ok(percent)` の腕で `is_finite()` を確かめ、そうでなければ `ArgumentDefaulted` と 1.0。`\n[NaN]`・`\n[inf]`・`\n[1e40]` のテスト。
- 外: 要らない。 残す 3 種: — （ukadoc `\n[パーセント]` は数）。

**4. `animation-sort`・`collision-sort` を正典の場所（`descript` ブレスの中）から読まない**（`shell/decode.rs:89-93`・`:118-121`・`:532-553`）
- 説明: 読むのはトップレベルの行だけ。`descript` ブレスは本体ごと捨てる。
- 失敗: 正典の書き方 `descript { animation-sort,ascend }` で `animation_sort == None` になり、着せ替えの重なりが逆になる。記録も無い。テストは正典でない形だけを固めている（`decode_tests_transcription_gap_tests.rs:96-118`）。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml:6098` の「正典どおりに読んで使う」は事実と違う。
- 直しの方向: `dispatch_block` の `descript` の腕で、本体の各行に `decode_sort_key` をかける。トップレベルの道は残す。正典の形のテストを 1 本。
- 外: コードは要らない。台帳の行（`doc/ukadoc-coverage/ledger/assets.toml:6098`・`:7005`）の直しは `doc/` に触る＝議題 D5。
- 残す 3 種: ③に当たる（振る舞いが変わる）が、**正典へ寄せる直し**（下の 4 節）。推奨＝直す。

**5. パターンのウェイト `最小-最大`（SSP の正典）が黙って 0 になる**（`shell/decode.rs:362`）
- 説明: `wait: field_u32(fields, tail)` が `50-100` を読めず 0 にする。元の字も残らない。
- 失敗: `animation0.pattern0,overlay,100,50-100,0,0` で待ち無しのコマになり、瞬きが一瞬で流れる。テスト無し・起票も無し（roadmap・brief・台帳を grep）。
- 直しの方向: 二案（議題 D3）。(a) 小さい直し＝範囲を読み取って最小値を待ちにし、`warn!` で「ランダムは未対応」と残す（モデルは変えない）。(b) 正典どおり＝`Pattern` に最大値を足し、下流で最小〜最大から選ぶ（新しい機能に当たりうる）。
- 外: (b) なら `crates/areka-seriko/src/table_calls.rs:62`・`:75`【C17 `audit-seriko`】、`crates/areka-emo-compose/src/nesting.rs:65`・`fold_merge_tests.rs:165`・`:202`【C17 `audit-emo-compose`】と、各クレートのテストの `Pattern {..}` の字面。
- 残す 3 種: ③（ukadoc「SSPのみ『最小ウェイト-最大ウェイト』…ランダムに選択」）。

**6. 信用できない surfaces.txt の巨大な範囲が、そのまま展開されてメモリを食い尽くす**（`shell/decode.rs:509-513`・`shell/boxes.rs:134` → 外の `crates/areka-emo-compose/src/fold.rs:309-328`・`crates/areka-emo-compose/src/boxes.rs:226`・`:236`）
- 説明: 読む側は `Range{start,end}` を検査せずに渡す（「展開は下流」の設計）。下流が `included.extend(lo..=hi)` で全部並べる。
- 失敗: `surface0-4294967295 { element0,overlay,a.png,0,0 }` で約 43 億の id（17 GB）と同じ数の ECS の実体を作ろうとし、シェルを読む所で落ちるか止まる。`surface.append0,!0-4294967295` も同じ。テスト無し。
- 直しの方向: 信用の境目で幅に上限をかけ、越えたら `error!` で記述子を名指して飛ばす。置き場は二案（議題 D4）＝(a) 読む側（`parse_target_element`）で Range を落とす・(b) 展開する側（emo-compose の fold）。上限の値は SSP で測る（議題 D4）。
- 外: (b) なら `crates/areka-emo-compose/src/fold.rs`・`boxes.rs`【C17 `audit-emo-compose`】。
- 残す 3 種: ③の気配（上限値が SSP の受ける幅と食い違うと振る舞いが変わる）。推奨＝直す（上限は SSP が実際に受ける幅より十分大きく取る）。

**7. 初回起動の挨拶が選択肢で終わり、選んだ後に台本が返ると、初回起動の記録が書かれない**（`drive.rs:542-552`・`compile.rs:570-598`・外の dola `cue/schedule.rs:194-197`・`cue/runtime.rs:310-324`）
- 説明: 初回起動の後始末（`areka.boot.count` を 1 にする epilogue）は、選択待ちの区切りと同じ時刻の後ろに並ぶ。`resolve_choice` の後は誰もそれを出さないので、トークは「終わっていない」のまま次の Tick を待つ。
- 失敗: OnFirstBoot が `\q[はい,yes]\q[いいえ,no]` で終わる → 選ぶ → OnChoiceSelect が台本を返す → kanade は `[ResolveChoice, StartTalk]` を同じ束で送る（`crates/areka-talk/src/lib.rs:70-76` の順序の契約）→ 新しいトークの Start が古いトークを閉じて `TalkDone{Interrupted}` → epilogue は出ない → 次の起動も「初回」。drive の doc の約束「次 Tick を待たない（R-5 の一 tick 遅延を残さない）」（`drive.rs:529-530`）にも反する。drive の段に「選択肢＋空でない epilogue」のテストが無い。
- 直しの方向: 解決の後、同じ時刻で既に来ている項目を出してから完了を見る。置き場は二案（議題 D6）＝(a) drive の `Some` の腕で `player.tick(last_tick)` を呼ぶ（dola の冪等の守り `schedule.rs:195` が通すかを確かめる＝調べ物 R4）・(b) dola の `resolve_choice` が来ている項目を出す。drive の段のテストを 1 本。
- 外: (b) なら `crates/dola/src/cue/schedule.rs`・`runtime.rs` とそのテスト（`audit-dola` は C18＝このウェーブでは走っていない）。
- 残す 3 種: —。

**8. 合図の開始時刻を f64 の秒の足し算で積むので、ms の格子からずれ、1 刻み遅れて出る**（`compile.rs:144`・`:173-188`・`:323-331`、`duration.rs:40-43`）
- 説明: `offset: f64` に `+= d` を重ねる。`duration.rs:32-33` は「整数 ms 算術＋単一変換」と書くが、積む側が守っていない。
- 失敗: `a\s[0]b\s[0]c\s[1]` で 0.05+0.05+0.05 = `0.15000000000000002`（この段で確かめた）。dispatcher の Tick は `(now-base)/1000.0`＝150 ms で `0.15` なので、`\s[1]` はその刻みで出ず、次の刻み（約 16 ms 後）に出る。下請けの試算では `+= 0.05` の最初の 2000 境界のうち 591 が上にずれる。記憶「1 フレーム遅らせる解は取らない」「時刻は正確に扱う」に反する。
- 直しの方向: `offset` を整数（`Duration` か ms の u64）で持ち、`emit` のたびに 1 回だけ秒へ直す。同じ f64 の足し算で期待値を作り直しているテスト（`compile_sheet_tests.rs:55`・`:74-78` ほか）は、整数 ms の期待値へ直し、境目の刻みのテストを 1 本足す。
- 外: 積み方だけ変えるなら無し。`text_playback_duration` の形まで変えるなら `crates/areka/src/emo2_boot/spine_talk_close_tests.rs`・`crates/areka-emo-text/examples/emo-text-layer.rs`・`examples/emo-text-layer/scenario.rs`・`crates/areka-ghost/tests/ghost/spine_e2e_test_s1_boot_success.rs`【C17 `audit-ghost-catalog`】。推奨＝形は変えない。
- 残す 3 種: —。

### Should fix（37 件）

**package**

**9. シェルの descript.txt が読めないとき、黙って「着せ替え無し」になる**（`package/resolve_bind.rs:58-62`）
- 失敗: 共有違反・権限の失敗も記録無しで空になる。呼び手 2 つは誤りを知るためだけに同じファイルを読み直している（`crates/areka/src/emo2_boot/assets.rs:404-411`・`crates/areka/src/placement/source.rs:156-166`）。reload の道（`crates/areka/src/reload.rs:634`）は読み直さないので、既定で着る衣装が記録無しで全部外れる。
- 直し: NotFound 以外は `warn!`（パスと誤り）。読み直しを消すなら結果に誤りを持たせる（議題 D7）。
- 外: 消すなら `crates/areka/src/emo2_boot/assets.rs`・`crates/areka/src/placement/source.rs`。 残す 3 種: —（「致命でない」は裁定済みで、記録が無いことは裁定されていない）。

**10. `ghost/master` を `/` でつなぐので、x64 の SHIORI に区切りの混ざったパスが渡る**（`package/resolve_shell.rs:25`・`:41`・`:104`）
- 失敗: `C:\…\root\ghost/master` が `crates/areka-ghost/src/shiori_inproc.rs:596` からそのまま DLL へ渡る。32bit の道はこれを手で直している（`shiori_wiring.rs:272-279`「区切りの混在…では読めない」）。PathBuf の比較は区切りの違いを無視するので、テストは気付けない。
- 直し: `join("ghost").join("master")`。`as_os_str` を比べるテストを 1 本。 外: 無し。 残す 3 種: ③の気配（SSP は `\` で渡す＝寄せる直し）。

**11. ゴーストの descript を読む 2 か所が、鍵の大文字小文字と空の値で食い違う**（`package/resolve_shell.rs:72-82`・`:112-116`・`:123`・`:148` と `crates/areka-ghost/src/catalog.rs:438-446`）
- 失敗: `Sakura.Name,さくら` は切り替えの一覧に出るが `sakura_name` は None。`Shiori,x.dll` は目録に見えるが SHIORI は「DLL 名未解決」。`sakura.name2,` は目録では無し、package では `Some("")`（`%selfname2` が空になる）。
- 直し: `lowercased` を parsers の kv の隣へ移し、両方が使う。
- 外: `crates/areka-ghost/src/catalog.rs`【C17 `audit-ghost-catalog`】。 残す 3 種: ③（鍵を大文字小文字で区別しないのは SSP 互換として目録の spec が選んだ）。

**12. bindgroup の引き口（`BindScope` と 8 つのメソッド）は外から呼べない死んだコードで、seriko が同じものを持つ**（`package/model.rs:114-225`・テスト `:281-501`）
- 失敗: `BindScope` は非公開の `model` の中で、`mod.rs:38` が出していない。ツリー全体の grep で範囲の外の使用 0。本物は `crates/areka-seriko/src/bind.rs:309-353`。「最後の宣言が勝つ」を変えると 2 か所を直す必要があり、効くのは seriko の方だけ。
- 直し: 引き口・`pick`・そのテスト（約 270 行）を消す。 外: 無し。 残す 3 種: ②の気配（`model.rs:174`「M1 の seriko は未消費＝将来差替シーム」。ただし `shortcut:` の印でも `#[allow(dead_code)]` でもない）。推奨＝消す。

**13. `parse_kv_with_line` は `parse_kv` の写し**（`package/resolve_bind.rs:175-188` と `kv/parse.rs:20-41`）
- 失敗: `parse_kv` を直す（例: 11 の大文字小文字）と、シェルの bindgroup だけ取り残される。`resolve.rs:8`「KV 分割は foundation へ委譲し再実装しない」とも食い違う。
- 直し: 行の番号つきを kv へ移し、`parse_kv` をその上に 1 行で書く（振る舞いは同じ）。 外: 無し。 残す 3 種: —（`resolve_bind.rs:173-174`「`parse_kv` は…変えない」は、振る舞いを保つこの直しと衝突しない）。

**14. 崩れた bindoption と `.default` の宣言は黙って落ち、`.name` だけが `warn!` を出す**（`package/resolve_bind.rs:74`・`:112-114`・`:213-242`、比べる所 `:264-272`）
- 失敗: `sakura.bindoption0.group,腕,mustselct`（綴り違い）で mustselect が消え、記録も無い。`sakura.bindgroup10.default,yes` も黙って off。
- 直し: 鍵と行の番号を添えて `warn!`。寛容な結果はそのまま。 外: 無し。 残す 3 種: —（寛容は裁定済み・記録しないことは裁定されていない）。

**15. bindgroup の欄が sakura・kero・char の 3 通り × 4 属性で並び、属性を 1 つ足すたびに 3 か所を直す**（`package/model.rs:53-91`・`resolve_bind.rs:156-167`）
- 失敗: bindopt の `multiple` を足したときに実際にこの 3 重の追加が要った（`model.rs:74-82`）。seriko も 3 枝を持つ（`bind.rs:238-263`）。
- 直し: スコープ番号を鍵にした 1 つの表（0=sakura・1=kero・2 以上=char）。
- 外: `crates/areka-seriko/src/bind.rs`・`bind_tests.rs`【C17 `audit-seriko`】、`crates/areka/src/offline_dump/mod.rs`。 残す 3 種: —。

**16. package の `model_tests.rs` は言っている契約を証せない**（`package/model_tests.rs:1-8` と全体）
- 失敗: 「下流が公開面だけで組める」と言うが、型は `#[non_exhaustive]` で下流は組めない（下流のテストにもそう書いてある）。残りは自分で入れた値を derive の `PartialEq` で比べるだけで、package のどんな変更でも赤にならない。`model.rs:480-500` も `Default` しか見ていない。
- 直し: ファイル（187 行）と `mod.rs:20-21` を消す。 外: 無し。 残す 3 種: —。

**balloon・charset・kv**

**17. charset の既定へのフォールバックと、壊れた並びの置き換えが `debug!` だけ**（`charset/decode.rs:35-38`・`:51-56`、`charset/prescan.rs:32-38`・`:61`）
- 失敗: `charset,Shift-JISX` のバルーンや、`charset,UTF-8` と書いた SJIS のファイルで、名前が化けても既定の記録の段では 1 行も残らない。SHIORI の層は同じ出来事を `warn!` にしている（charset-canon 要件 2.4・4.4・12.1）＝同じ決まりを 2 か所で違う重さにしている。
- 直し: 2 つの `debug!` を `warn!` へ。 外: 無し。
- 残す 3 種: ①の気配（charset-canon のギャップ表がファイルの層を「不正並びは代替文字＋詳細ログ」で「一致」と記録している）。推奨＝`warn!` へ（記憶「ログ無し失敗経路の禁止」）。

**18. バルーンの数の鍵が読めないとき、黙って None になる（矢印の鍵だけが報告される）**（`balloon/parse.rs:255-257`、約 30 の鍵＝`:77-78`・`:90-123`・`:149-166`・`:203-207`）
- 失敗: `font.height,28px`・`validrect.left,１０`（全角）・`font.color.r,300` が黙って既定になる。矢印の鍵は `KeyIssue` で集めて `crates/areka-emo-present/src/balloon_definitions.rs:100` が `warn!` する。
- 直し: `KeyIssue` の仕組みを全部の数の鍵へ広げ、今の `warn!` の場所で全部出す。
- 外: `crates/areka-emo-present/src/balloon_definitions.rs`【C17 `audit-emo-present`】、`crates/areka-emo-text/src/actor_scroll_present_tests.rs`・`scroll_marker_tests.rs`（`ArrowKeys` の欄を移すなら・ファイルは足さない）。
- 残す 3 種: ③の気配（SSP は `12px` を 12 と読むかもしれない＝読み方を変えるなら測る。記録を足すだけなら当たらない）。推奨＝記録だけ足す。

**19. `parse_kv` の「キーが空の行を飛ばす」枝にテストが無い**（`kv/parse.rs:34-36`）
- 失敗: 枝を消すと `parse_kv(",v\n")` が `{"": "v"}` を返しても全部緑（`blank_lines_are_skipped` はカンマの無い行の枝を通るだけ）。
- 直し: `parse_kv(",v\n  ,w\n")` が空のテスト 1 本。 外: 無し。 残す 3 種: —。

**20. 「知らない charset のラベルは既定へ落ちる」のテストは、落ち先が違っても通る**（`charset/decode_tests.rs:41-49`）
- 失敗: 入力が ASCII だけなので、`decode.rs:39` を UTF-8 や WINDOWS_1252 に変えても緑。
- 直し: SJIS の `charset,NOSUCH\r\nname,かくかく` を `Ansi` で読み、`かくかく` を確かめる。 外: 無し。 残す 3 種: —。

**21. `arrow.filename` と `marker.filename` で空の値の扱いが違う**（`balloon/marker.rs:96` と `:103-109`）
- 失敗: `arrow.filename,` で `Some("")`。下流の隠し場所の鍵（`balloon_definitions.rs:101-112`・`crates/areka/src/emo2_boot/shell_box_assets.rs:88-99`）で `Some("")` と `None` が別に扱われ、同じ矢印の対を 2 回読む。描けているのは下流（`balloon_marker.rs:43`）がもう一度空を落としているから。
- 直し: `marker.rs:96` に `.filter(|v| !v.is_empty())` とテスト。 外: 要らない。 残す 3 種: —。

**22. `DefaultEncoding::to_encoding` の `#[allow(dead_code)]` の理由がもう古い**（`charset/model.rs:32-34`）
- 失敗: 理由は「消費者 `decode`（タスク 2.3）が未実装」だが、`decode.rs:39`・`:43` で使っている。呼び出しが消えても警告が出ない。
- 直し: 3 行を消す。 外: 無し。 残す 3 種: ②の形（`#[allow(dead_code)]`＋理由の doc）だが、理由が事実と違うので当たらない。推奨＝消す。

**23. 落ちようのないテスト（balloon・charset）**
- `balloon/validation_tests.rs:195-208`（`assert_eq!` の直後の `assert_ne!`・r5_1 の重複）・`balloon/model_tests.rs:241-253`（derive の確かめ）・`charset/model_tests.rs:7-28`・`:50-64`（derive と静的な match の同一性）・`charset/decode_tests.rs:11-23`（ASCII だけの入力）。
- 失敗: 対象を壊しても赤にならない（例: UTF-8 の宣言を無視しても `:11-23` は緑）。
- 直し: 消す（`:11-23` は日本語の入力に変えてもよい）。 外: 無し。 残す 3 種: —。

**sakura（parsers）**

**24. 引数の途中の `"` でクォートが始まり、引用符が消えるか、後ろの台本を全部飲み込む**（`sakura/lexer.rs:364-390`）
- 失敗: `\q[27" モニタ,OnA]\e次` で閉じのクォートが無いまま `Unclosed` になり、後ろが 1 つの Raw になる（選択肢・`\e`・「次」が消える）。`\![raise,OnX,a"b"c]` は `abc` になる。テスト無し。
- 直し: 引数の先頭（`cur.is_empty() && !quote_consumed`）のときだけクォートの始まりとし、途中の `"` はただの字として入れる。
- 外: 無し。 残す 3 種: ③（ukadoc notes_escape「その引数全体を""で囲う」＝直しは正典に寄る。SSP では測っていない）。

**25. 旧形の `\q[ID][タイトル]` は、タイトルの直後がタグのときだけ畳まれ、地の文が続くと偽の選択肢になる**（`sakura/decode.rs:183-187`）
- 失敗: `\q[OnA][はい]です\e` で `Choice{disp:"OnA", target:""}` と見える字「[はい]です」が出る（イベント名の書かれた、行き先の無いボタン）。`\q[ID][100%]` も `%` で切れて畳まれない。
- 直し: `[` で始まり `]` をどこかに持つなら、最初の `]` までを畳み、残りを地の文として返す。
- 外: 無し。 残す 3 種: ①の気配（`validation_tests.rs:149-150`・`parse_noted_tests.rs:109`「既存の規則」）と ③（ukadoc「\q[ID][タイトル]または\q*[ID][タイトル]」[旧仕様]）。

**26. 通し番号つきの旧形 `\q0[ID][タイトル]` が地の文として吹き出しに出る**（`sakura/lexer.rs:242-250`・`decode.rs:183-187`）
- 失敗: `\q0[OnYes][はい]` で `Raw("\q")`＋見える字「0[OnYes][はい]」。旧形は黙った Raw にする意図（要件 5.3）に反する。
- 直し: `legacy_q_star` で `*` か 1 桁の数を受ける。
- 外: 無し。 残す 3 種: ③（ukadoc「*は通し番号」の読みが二通り。台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml:5403` は `*` を字のまま読む）＝議題 D8。

**27. 引数を取らないタグの直後に `[` で始まる台詞が来ると、タグの意味が消える**（`sakura/lexer.rs:252-255`・`decode.rs:308-379`）
- 失敗: `\1[小声で]そうなの？` で話し手の切り替えが消え、「[小声で]」も消える。`\e[注]です` はトークが終わらず「です」が出る。
- 直し: 角括弧の道に入るのを、引数を取りうるタグ名だけにする（`e c - 0 1 h u + v` は除く）。
- 外: コードは無し。テスト `lexer_word_boundary_tests.rs:183`・`:189`、`parse_word_boundary_tests.rs:124`。
- 残す 3 種: ①（`lexer_word_boundary_tests.rs:165`「従来どおり角括弧形として読まれる」をテストが固めている）と ③（SSP で測っていない）＝議題 D8。

**28. `Instruction::Move`／`MoveArgs` は汎用の道の型つきの写しで、消費者は 2 つとも汎用へ戻している**（`sakura/model.rs:53-54`・`:167-175`・`decode.rs:423-431`）
- 失敗: `compile.rs:275-285` は `command_carrier("move", args)` へ戻し（「parser が種別で分離済みの転記を戻すだけ」）、`crates/areka/src/mcp/check_script_judge.rs:89-94` も汎用の腕と同じことをする。2 つの腕を一緒に直す必要がある。記憶「\!コマンドは汎用キャリア 1 本・typed 個別新設禁止」。
- 直し: `Move`・`MoveArgs`・`decode_bang` を消し、`"!"` を `decode_passthrough_bang` へ直に。
- 外: `crates/areka/src/mcp/check_script_judge.rs:89-94`、テスト `compile_arm_tests.rs` の 8 か所と parsers の 3 か所。
- 残す 3 種: ①（完了 spec `areka-P0-sakura-parse` の要件 7.1）。推奨＝消す（後の裁定「汎用キャリア 1 本」が上位）＝議題 D9。

**29. 知らないタグと閉じの無い `[` で台本の残りが消えても、ふつうの再生の道では `debug!` しか残らない**（`sakura/parse.rs:28-33`・`lexer.rs:258-261`・`compile.rs:426-428`）
- 失敗: `\0\s[10こんにちは。…（10 KB）…\e` で `\s[` より後ろが 1 つの Raw になり、ふつうの道（`drive.rs:254` → `compile.rs:426`）は `debug!` だけでトークの残りを黙って捨てる。`こんにちは\xさようなら` の `\x` も同じ。記録が出るのは MCP の strict と check_script の道だけ。
- 直し: `compile.rs:426` の腕を `warn!`（固定の `event` 欄つき・ほかの腕と同じ形）にする。閉じの無い `[` を見分けるなら `parse_noted` の印を使う。
- 外: 無し。 残す 3 種: ①の気配（`compile.rs:420-421`「無視ログを記録し cue を生成せず継続」は重さを決めていない）。推奨＝`warn!`。

**30. 同じワークスペースの中の `#[non_exhaustive]` が、全部の型を扱ったかのコンパイラの確かめを止めている**（`sakura/model.rs:12`・`:23`・`:82`・`:180`）
- 失敗: 消費者は全部「その他」の腕を持つ（`compile.rs:426`・`check_script_judge.rs:62`・`:129`）ので、新しい型を足しても黙って `debug!` で落ちる。`CharCodeDropped` と `Anchor` は、まさにそのために手で腕を足した。クレートは `publish = false`。
- 直し: `Instruction`・`ReadNote`・`CharCodeDrop` の `#[non_exhaustive]` を外し、「その他」を全部の腕に置き換える。
- 外: `crates/areka/src/mcp/check_script_judge.rs`・`crates/areka/src/readme/destination.rs`・`crates/areka/src/emo2_boot/spine_conformance_script.rs`・`readme_cue_tests.rs`・`script_restriction_tests.rs`（`match` を持つ所を設計で洗う）。
- 残す 3 種: ①（`model.rs:12`「variant 追加は後方互換（要件 11.1）」・`compile.rs:424`「本 catch-all は構造上必須」）＝議題 D9。

**31. sakura（parsers）の重複したテスト**
- `parse_tests.rs:79` と `validation_tests.rs:203`（同じ中身）・`parse_tests.rs:19` と `validation_tests.rs:229`・`parse_tests.rs:60`／`validation_tests.rs:248`（純粋な関数を 2 回呼んで比べる＝落ちない）・`\q[はい,OnYes]` の 3 重（`decode_tests.rs:186`・`:432`・`validation_tests.rs:268`）・`decode_tests.rs:199`=`validation_tests.rs:281`・`:447`=`:90`・`:327`=`:213`・`:280`=`:311`・`:170`≈`:186`・`:418`≈`:152`・lexer の写し（`lexer_tests.rs:259`・`:286`・`:121`・`:415`・`:428`・`:448` と `lexer_bare_tag_tests.rs`・`lexer_word_boundary_tests.rs`）・`lexer_tests.rs:383`（`:394` が上位）。
- 失敗: 1 つの退行で 2〜3 本が同時に赤くなり、確かめは増えない。直すときは全部を一緒に直す。
- 直し: 片方ずつ消す（確かめの中身は減らさない＝要件 3.3）。 外: 無し。 残す 3 種: —。

**32. sakura（parsers）の `model_tests.rs` はほとんど自分と同じ写しを比べるだけ**（`sakura/model_tests.rs:15-146`）
- 失敗: `End == End` など、derive の `PartialEq` しか通らない。
- 直し: ファイルを消す（`as_str`・`ratio` は他でも確かめている）。 外: 無し。 残す 3 種: —。

**shell**

**33. 途中のブレスの `}` が抜けると、次のブレスが前のブレスに黙って混ざる**（`shell/lexer.rs:159-178`）
- 失敗: `surface0 { element0,…a.png… ` のあと `surface1 { element0,…b.png… }` で、surface0 に 2 つの絵が入り、surface1 は無くなる。記録もテストも無い。
- 直し: 本体の中の `{` だけの行を新しいブレスの始まりとし、前の行を見出しにして、崩れとして記録する。
- 外: 無し。 残す 3 種: ①の気配（`lexer.rs:171-172`「素朴に本体行として…吸収する」）と ③（SSP の振る舞いは測っていない）＝議題 D10。

**34. ファイルの終わりで閉じていないブレスは丸ごと落ち、誰も記録しない**（`shell/lexer.rs:112-117`、無視する側 `decode.rs:95`・`boxes.rs:91`・`cursor.rs:44`・`undrawn.rs:57`）
- 失敗: 最後の `surface9 { element0,…x.png…`（`}` 無し）で surface9 が Shell・boxes・cursor・undrawn のどれからも消える。読み込む側（`crates/areka-emo-present/src/shell_target.rs:390-392`）は気付けない。
- 直し: `parse_undrawn_elements` と同じく、落とした断片（閉じていないブレスの見出しと行の番号）を出し、`shell_target.rs` で `warn!`。`parse` が失敗を返さないのは保つ。
- 外: `crates/areka-emo-present/src/shell_target.rs`【C17 `audit-emo-present`】。 残す 3 種: ③の気配（SSP が閉じていない最後のブレスを残すかは測っていない）。推奨＝記録だけ足す。

**35. ブレスを集める輪が 4 つの写しで、同じ文を 1 回の読み込みで 4 回字句に分ける**（`shell/decode.rs:65-103`・`boxes.rs:75-93`・`cursor.rs:23-46`・`undrawn.rs:30-59`）
- 失敗: 4 つは一緒に直す必要がある。decode の 3 つの枝（`:70-74` 開いたまま次が来る・`:79-81` ブレスの外の行・`:99-103` 終わりで開いたまま）は、字句の段が閉じたブレスしか出さないので決して走らない（テストで直に字句の列を渡す所も 0）。「decode は閉じていないブレスも振り分ける」と doc に書いてあるが事実と違う。
- 直し: 字句の段が閉じたブレス（見出し・本体）の列を直に返し、4 つを 1 本の輪にする。1 回だけ字句に分けて 4 つに渡すかは議題 D11。
- 外: 1 回に絞るなら `crates/areka-emo-present/src/shell_target.rs:390-392`・`:440`【C17 `audit-emo-present`】。 残す 3 種: —。

**36. 読めない数を `unwrap_or(0)` で 0（有効な id）にする**（`shell/decode.rs:204`・`:249`・`:333`・`:355`・`:503-515`・`:604`）
- 失敗: `surface.append10,abc` で追記が surface 0 にも入る。`surface99999999999`（u32 の溢れ）→ 0。`surface.append10,2100 - 2110`（`-` の前後の空白）→ `Range{0,0}`。`smile,[2106,x]` → `[2106,0]` で `\s[smile]` が surface 0 を出しうる。
- 直し: 読めない記述子や別名の項目は 0 にせず飛ばし、34 と同じ道で出す。範囲の端は trim する。
- 外: `crates/areka-emo-compose/src/fold.rs`【C17 `audit-emo-compose`】、seriko の別名の引き（`resolve.rs`）【C17 `audit-seriko`】（振る舞いが変わる分の確かめ）。
- 残す 3 種: ①（`decode.rs:15`・`:497`・`:563`「既定 0 に倒す（要件 3.3）」）＝議題 D10。

**37. shell の `model_tests.rs` はほとんどコンパイラを確かめている**（`shell/model_tests.rs:59-87`・`:112-136`・`:140-162`・`:166-260`）
- 失敗: `assembled_shell_preserves_structure_and_duplicate_alias_keys` は重複する鍵の字面を作って読み返すだけ。`decode_alias_block` が重複を落とすようになっても緑（本物の守りは `decode_tests_alias_tests.rs:73-90`・`validation_tests.rs:237-250`）。
- 直し: 読み取りの往復だけ残すか、ファイルを消す。 外: 無し。 残す 3 種: —（見出し `:6-8`「固定する下流共有 I/O 契約」は満たされていない）。

**38. `Surface.id`（最初の対象の代表 id）が消費者を誤らせる**（`shell/model.rs:65-66`・`decode.rs:139-140`・`:519-530`）
- 失敗: `crates/areka-emo-atlas/src/manifest.rs:145`・`:184` は `surface.id` を鍵にする。`surface1-3 { element0,overlay,eye.png,0,0 }` と bind のパターン `overlay,2,…` で 2 への参照が数えられず、`eye.png` を「bind だけの絵でない」と誤る。
- 直し: `Surface.id` を消し、消費者が `targets` を展開する（鍵が要る所だけ `representative_id`）。
- 外: `crates/areka-emo-atlas/src/manifest.rs` と、各クレートのテストの `Surface { id: .. }`（emo-compose【C17 `audit-emo-compose`】・emo-present【C17 `audit-emo-present`】・seriko【C17 `audit-seriko`】ほか・設計で洗う）。
- 残す 3 種: ①（`model.rs:65`「多 id 形は先頭ターゲットの値」は設計の事後条件）＝議題 D10。

**drive・strict・sysvar**

**39. 合わない `ResolveChoice` を `debug!` で捨てる（ほかの状態の同じ誤投函は `warn!`）**（`drive.rs:553-566` と `:573-591`）
- 失敗: id が合わないと、`\![set,choicetimeout,0]` のときトークは永遠に待ち、残るのは debug の 1 行だけ。
- 直し: `warn!` へ（1 行）。 外: 無し。 残す 3 種: —。

**40. 「A+0.6 で初めて bye が届く」のテストが、届いた時刻を確かめていない**（`drive_delivery_tests.rs:131-142`・`:164-172`）
- 失敗: bye を集めるのは A+1.0 の Tick の後。アンカーが +0.3 s ずれて A+0.9 に出ても緑。`assert_ne!(10.0+0.6, 20.0+0.6)` は定数どうしの比べ。
- 直し: A+0.6 の直後に時限つきで bye を待つ。`assert_ne!` を消す。 外: 無し。 残す 3 種: —。

**compile ほか**

**41. `SakuraError` は 1 行の記録を整えるためだけにあり、`SinkSend` は死んでいる**（`error.rs:12-24`・テスト `:26-71`、`lib.rs:49`、`Cargo.toml:18` の `thiserror`）
- 失敗: `SinkSend` を作る所はツリーに 0。`SakuraError` は `drive.rs:304` で `error!` に渡すだけで、`Err` を返す所は無い（`CueSink::emit` は `()` を返す）。doc（`error.rs:3-5`）は存在しない道を書く。
- 直し: `error.rs` と `thiserror` の依存を消し、`drive.rs:304-305` を `error!(tick = t, …)` に。小さく済ますなら `SinkSend` とそのテストだけ消す（依存の削除が `Cargo.lock` に触るかは議題 D2）。
- 外: 無し（範囲の外の使用 0）。 残す 3 種: ②の気配（`error.rs:23`「拡張シーム（M-dialogue 以降の失敗種別を追加可能）」は `shortcut:` でも `#[allow(dead_code)]` でもない）。推奨＝消す。

**42. `contract.rs` の 8 つの再輸出は、その道で誰も使っていない**（`contract.rs:54` の `SenderHead`・`SenderType`・`SenderWord`、`:61` の `NewLineRatio`・`SurfaceArg`、`:64` の `DEFAULT_USERNAME`・`ResolvedVar`・`resolve_system_var`）
- 失敗: `areka_sakura::` 経由の使用はツリーに 0（複数行の `use` も含めて grep）。外は `areka_sakura::sysvar::` から引く。
- 直し: 8 つの名を消す（`ScriptOrigin`・`SecurityLevel`・`SstpKind`・`SystemVarSnapshot` は残す）。 外: 無し。 残す 3 種: —。

**43. strict の見分けの決まりが compile と check_script の 2 か所に、choicetimeout の確かめが 3 か所にある**（`compile.rs:107-116`・`:120-130`・`:303` と `crates/areka/src/mcp/check_script_judge.rs:56-63`・`:69-78`）
- 失敗: ReadNote の種類を足すか choicetimeout の決まりを変えると 2〜3 か所を直す。strict の道では `parse_choice_timeout` を 1 つの指示に 2 回かける。一致のテスト（`strict_judges_tests.rs`）は食い違いには気付くが、写しは止めない。
- 直し: `crate::strict` に 1 つの関数（印 → 見分けの点・choicetimeout の判定）を置き、3 か所から使う。
- 外: `crates/areka/src/mcp/check_script_judge.rs`。 残す 3 種: —。

**44. compile の重複したテスト**（`compile_arm_tests.rs:266-352` ⊂ `:639-765`・`:406-434` = `:844-880`・`:439-500` ⊂ `:886-928`・`:379-402` ≈ `:795-838`・`:582-594` ≈ `:770-788`・`:11-20` ≈ `:64-77`、`compile_sheet_tests.rs:108-116` ≈ `:155-163`・`:167-172` ≈ `:402-421`・`:368-397` は `assert_clear_all_prefix_and_rest` と同じ）
- 失敗: 1 つの退行で 2 本が赤くなり、確かめは増えない。
- 直し: 弱い方を消す（中身は減らさない）。 外: 無し。 残す 3 種: —。

**45. compile の落ちようのないテスト**（`compile_sheet_tests.rs:176-193`・`duration.rs:86-93`＝純粋な関数を 2 回呼ぶ・`compile_font_tests.rs:81-87`＝長さ 4 の配列の長さ・`duration.rs:95-103`＝作り方から真・`error.rs:56-70`＝死んだ `SinkSend` の書式）
- 直し: 消す。 外: 無し。 残す 3 種: —。

### Nice to have（48 件）

**package**
- **46. `resolve` は素通しの包みで、ただ 1 つの本番の呼び手に要らない失敗を負わせる**（`package/resolve.rs:26-31`）。`crates/areka/src/placement/persist.rs:307` は `shiori.dir` しか要らないのに、既定のシェルが無いと（利用者がシェル B を使い、descript の既定 A を消した）保存した状態を戻さない。直し＝persist は SHIORI の置き場だけを組み、`resolve` を消してテストを `resolve_with_shell(.., None)` へ。外＝`crates/areka/src/placement/persist.rs:30`・`:307`、`crates/areka-ghost/src/config.rs:163`・`:180`・`shiori_wiring.rs:359`・`shiori_inproc_adapter_tests.rs:267`・`shiori_wiring_charset_tests.rs:56`・`tests/ghost/inproc_e2e_test.rs:851`・`:1069`・`tests/ghost/inproc_fixture.rs:426`・`tests/ghost/snapshot_capture_test.rs:353`・`tests/real_pasta_online.rs:224`【C17 `audit-ghost-catalog`】。残す 3 種: —。
- **47. bindgroup の番号が `+` と先頭の 0 を受ける（`char{N}` の決まりと違う）**（`package/resolve_bind.rs:246-250`）。`sakura.bindgroup0010.default,1` と `sakura.bindgroup10.default,0` は別の鍵として後勝ちが効かず、10 が on。直し＝`split_scope` と同じ数字だけの確かめ。外＝無し。残す 3 種: ③の気配（SSP が atoi で読むかは不明）。
- **48. `.name` の空のカテゴリを受ける（bindoption は同じものを断る）**（`package/resolve_bind.rs:258-266` と `:218-220`）。`sakura.bindgroup10.name,,伸び` が `category: ""` で入る。直し＝今の断って `warn!` する枝に `category.is_empty()` を足す。外＝無し。残す 3 種: —。
- **49. パーツ名が無いときの `warn!` に行の番号とスコープが無い**（`package/resolve_bind.rs:266-270`）。`sakura.…10.name,x` と `kero.…10.name,y` が同じ記録になる。直し＝`line` とスコープを欄に足す。外＝無し。
- **50. 1 回の起動で同じ descript を 2〜3 回読んで復号する**（ゴーストの descript＝`resolve_with_shell` と `crates/areka/src/placement/source.rs:154`・`crates/areka/src/emo2_boot/assets.rs:397`、シェルの descript＝`read_bindgroup_defaults` と `source.rs:157`・`assets.rs:406`、`crates/areka/src/offline_dump/mod.rs:130`・`:169`）。小さいファイルなので遅さは小さい。直し＝9 の直しで 1 つ減る。それ以上は範囲の外の直しになる。
- **51. package の重複したテスト**（`package/validation_tests.rs:118-142`・`:218-229` が `:56-70`・`:86-93` と同じ値、`resolve.rs:33-96` の中のテストが `resolve_shell_tests.rs:128-146` と同じ）。直し＝写しを消す（`validation_tests.rs:135-141` の分解は残す）。
- **52. package の古いモジュールの doc**（`package/mod.rs:11-17` は `model`・`resolve` しか挙げず「`model ← resolve`」、`resolve_shell.rs:7-8` の読む鍵の一覧が実際より少ない）。直し＝今の姿に合わせる。

**balloon・charset・kv**
- **53. charset と kv の重複したテスト**（`charset/decode_tests.rs:30-37` = `charset/validation_tests.rs:58-70`、`no_declaration_uses_ansi_default_as_shift_jis` = `validation_tests.rs:85-97` の Ansi 半分・Utf8 半分は ASCII だけ、`kv/validation_tests.rs` 全体 ≈ `kv/parse_tests.rs` の `value_is_kept_as_raw_string`）。直し＝1 本ずつにする。
- **54. 検体のテストが本番と違う既定で復号する**（`balloon/validation_tests.rs:47` が `Utf8`、本番 `crates/areka-emo-present/src/balloon.rs:465` は `Ansi`）。検体から `charset,UTF-8` が消えても緑。直し＝`Ansi` を渡す。外＝無し。
- **55. UTF-16 や「replacement」に当たるラベルでファイル全体が壊れる**（`charset/decode.rs:30-32`・`:48`）。`charset,ISO-2022-KR` で 1 字の U+FFFD になり鍵が全部消える。`charset,UTF-16` は中身が ASCII でも UTF-16LE として読む。直し＝`UTF_16LE`・`UTF_16BE`・`REPLACEMENT` は解けないラベルと同じに扱う（SHIORI の層は charset-canon 要件 10.3 でそうしている。`is_ascii_compatible()` は ISO-2022-JP を断るので使わない）。残す 3 種: ③の気配（SSP は CP50225 で ISO-2022-KR を読める。どちらの案でも areka は読めない）。
- **56. `parse` は画像別の層が無くても descript の表を写し、`parse_str` は 2 回写す**（`balloon/parse.rs:50-59`・`:68-72`）。直し＝層が無いときは `descript` をそのまま `map_merged` に渡し、`parse_str` は持っている表に `extend` する。
- **57. `get_raw` があるのに 8 か所で同じ式を書いている**（`balloon/parse.rs:247-249` と `:85`・`:86`・`:121`・`:128`・`:131`・`:137`・`:146`・`:169`）。直し＝`get_raw` を使う。
- **58. prescan の決して走らない枝**（`charset/prescan.rs:45-47`。窓は ASCII だけなので `from_utf8` は必ず通ると自分の注釈に書いてある）。直し＝窓を 1 回だけ文字列にしてから行に分ける。
- **59. `CursorColor` は `FontColor` の写し**（`balloon/model.rs:772-805` と `:497-526`）。直し＝1 つの色の型。外＝`crates/areka-emo-text`（選択肢の見た目とテスト）。残す 3 種: ①（`model.rs:774-776`「`CursorColor` を命名した意味論分離に従う」）＝議題 D12。
- **60. `windowposition.x` を数と生の字で 2 回持つ**（`balloon/parse.rs:76-87`）。消費者 `crates/areka/src/placement/mod.rs:741-757` が両方を取り、両者は合っていなければならない。直し＝数は下流で生の字から作る。外＝`crates/areka/src/placement/`（`mod.rs`・`windowposition.rs`・`face_windowposition_tests.rs`）と `windowposition().x()` の読み手。残す 3 種: ①（`parse.rs:15-16`・`model.rs:23-26`「既存の数値写像は無改変＝同要件 5.1」）＝議題 D12。
- **61. `BalloonModel::new`（7 引数）＋ 8 つの `with_*` のうち 4 つは範囲の外の呼び手が 0**（`balloon/model.rs:90-189`。`with_marker_filename`・`with_windowposition_raw`・`with_font_decoration_raw`・`with_font_shadow_raw`、`with_disable_font` も範囲の中だけ）。直し＝外から使わないものを `pub(crate)` にする。残す 3 種: ①（`model.rs:80-89`「既存 30 呼出箇所は無改変」）＝議題 D12。
- **62. 重複したテストの道具**（`balloon/model_tests.rs:20-25` の `kv_map` = `balloon/parse_tests.rs:19-24` の `map`、`balloon/validation_tests.rs:35-37` の `fixture_dir` は素通し）。直し＝1 つにして素通しを消す。
- **63. 古い doc**（`lib.rs:1`「純粋・std のみ」なのに `Cargo.toml:13-14` は encoding_rs と tracing に依る、`balloon/mod.rs:3-5` が `marker` を挙げない）。直し＝今の姿に合わせる。

**sakura（parsers）**
- **64. Raw と CharCodeDropped の字は元の綴りでなく、おおよその組み直し**（`sakura/decode.rs:533-539`・`:166-169`・`:200-201`・`:519-522`、`model.rs:77`「`raw` はタグの元の綴り」）。`\_u[]` → `\_u`、`\i["a,b"]` → `\i[a,b]`（2 引数に見える）、末尾の 1 字の `\` → `Raw("\\\\")`（原因＝`lexer.rs:201` が `Bare("\\")` と `\` 込みで持つ）。読むのは記録だけ（`compile.rs:410-414`・`:427`）。直し＝末尾の `\` は `Bare(String::new())`、doc を「おおよそ」に（または範囲で原文を切る）。
- **65. 呼び手が 1 つずつの包みが 3 つ**（`sakura/decode.rs:491-498` `decode_passthrough_tag`・`:519-522` `decode_passthrough_bare`・`:527-530` `decode_passthrough_raw`、名は古いタスク「タスク 4.2 のシーム」から）。直し＝畳み込む（`decode_passthrough_bang` は呼び手 3 つなので残す。`.kiro/specs/property-query-channels/brief.md:25` はそれだけを名指す）。
- **66. 字句の段が決して出さない形への守りの腕と、そこへ届くために手で字句を作るテスト**（`sakura/decode.rs:236-241`・`parse_noted_tests.rs:286-296`）。直し＝`Token::Shorthand.word` を 3 値の列挙にすれば消せる。残す 3 種: ①の気配（`decode.rs:236`「対象外の短縮語…への防御…`Raw` に留める」）。
- **67. テストのモジュールの見出しが中身と合わない**（`sakura/decode_tests.rs:9`「ここでは検証しない」なのに `:318-556` で確かめる、`lexer_tests.rs:11-12` も同じ）。直し＝見出しを直す。

**shell**
- **68. `Token::Raw(String)` の中身を誰も読まない**（grep で `Raw(_)` と `lexer_tests.rs:226` の `matches!` だけ）。`lexer.rs:115` の `raw_lines_from`（`:180-184`）が閉じていないブレスのあと全部を写す。直し＝`Raw` を中身無しにし、`raw_lines_from` を消す（34 の直しと一緒に設計する）。
- **69. `decode_surface_body` は呼び手 1 つの素通しで、`decode_append_block` は同じ 3 つを直に呼ぶ**（`shell/decode.rs:171-180` と `:464-470`）。直し＝畳み込む。
- **70. 一緒に直す必要のある小さな写し**（`shell/boxes.rs:119` の `field` = `decode.rs:265-267` の `field_string`、`"balloon"` の字を `boxes.rs:122` と `undrawn.rs:47` で別々に確かめる）。直し＝`is_image_element_method`（`decode.rs:227`）の隣に `pub(super) fn is_box_element_method` を置く。（`crates/areka-emo-atlas/src/manifest.rs:157-173` の写しは範囲の外の持ち物＝記録だけ）
- **71. 見出しと `{` の間に何かの行があると、ブレスが壊れる**（`shell/lexer.rs:99-104` は空行と `//` だけを飛ばす）。`surface0\nメモ\n{…}` で「メモ」が見出しになり、surface0 が消える。正典 introduction_surfaces.txt は認識されない行を全部コメントとする。直し＝見出しの次の有意な行を探すとき、`{` でない行の扱いを正典に合わせる。残す 3 種: ③（直す前に SSP で測る）＝議題 D10。
- **72. アニメーションの引きが線形の探し**（`shell/decode.rs:396-399` `animation_slot`・注釈 `:314-315`）で 1 ブレスあたり O(A²)、面の本体も 3 回なめる（`:173-177`）。何千の id のブレスでだけ効く。直し＝表にする（提案どまり）。
- **73. shell の重複したテスト**（メソッドの表＝`parse_tests.rs:182-201` と `decode_tests_method_matrix_tests.rs:70-92`、`sometimes` → Other が 4 か所、replace の転記が 4 か所、空の入力 → 空の Shell が 3 か所）。直し＝1 本ずつにする。
- **74. shell の古いモジュールの doc**（`shell/decode.rs:8-37`「スケルトンの枠組み」、`lexer.rs:28-29`「後続タスクで結線」、`decode_tests.rs:3-10`、`decode_tests_skeleton_tests.rs:43-44`・`:65`）。直し＝今の姿に合わせる。
- **75. `decode_tests_pattern_target_tests.rs` の名が置き場と合わない**（`decode.rs:621-623` の子なのに `decode_tests_` の頭を持ち、`decode_tests.rs` からは辿れない）。直し＝`decode_pattern_target_tests.rs` へ改名（ほかの `decode_tests_<主題>_tests.rs` は `decode_tests` の子の命名どおりで問題無し）。
- **76. `ShellBoxes` は 1 欄の包み**（`shell/boxes.rs:20-24`）。直し＝`Vec<BoxDefinition>` を直に返す。外＝`crates/areka-emo-compose/src/boxes.rs`【C17 `audit-emo-compose`】・`crates/areka-emo-present/src/shell_target.rs`【C17 `audit-emo-present`】・`crates/areka-emo-text` のテスト。残す 3 種: —。

**drive・strict・sysvar**
- **77. `send_done` と `send_interrupted` はほぼ写し**（`drive.rs:605-614` と `:620-632`、違いは理由と quit の印と記録の文だけ）。直し＝1 つにする。
- **78. 落とす直前の `player.stop()` は何も見える働きをしない**（`drive.rs:501-503`。`stop` はシンクを呼ばず、すぐ drop する）。直し＝`stop` と `mut` を消す（`CuePlayer::stop` の本番の呼び手はここだけになり、dola のテストだけが使う＝dola は触らない）。
- **79. `choice_notified` を戻すのは将来の機能のための継ぎ目**（`drive.rs:173-180`・`:544-547`。今の compile は 1 トークに区切りを高々 1 つしか出さない）。残す 3 種: ②の気配（「1 トークに複数バリアを持つ将来拡張のシームとしてリセットを残す」と doc にある。`shortcut:` の形ではない）＝議題 D13。
- **80. ゴースト名の `Arc<str>` を 3 重に持つ**（`drive.rs:184`・`:250` の組、`strict_tap.rs:72`・`:84` の `StrictTap.ghost`、`strict_tap.rs:29` の `StrictLedger.ghost`）。直し＝組と引数から外し、台帳から写す。
- **81. 範囲の外の使い手が 0 の公開面**（`lib.rs:41` `pub mod strict_tap`・`lib.rs:45-47` `pub use compile_strict`。`StrictTrace` は `pub` だが欄は `pub(crate)` で外からは使えない）。grep で外の呼び手 0（`crates/areka/src/emo2_boot/strict_judges_tests.rs:54` の注釈だけ）。直し＝`pub(crate)` にする。
- **82. sysvar のフォールバックの `warn!` が台本全体を 2 回書く**（`sysvar.rs:117-122` の `script = %script, expanded = %expanded`）。MB 級の台本で `\w%username`（数字で始まる名）なら、そのたびに台本の約 2 倍を記録へ書く（ファイルの記録に上限は無い）。直し＝変数の名だけか、切った抜粋だけを書く。
- **83. `resolution_is_deterministic` は言っていることを捕まえられない**（`sysvar.rs:209-219`「no I/O・外部環境非読取」。同じプロセスで 2 回呼んでも環境の読みに気付けない）。直し＝消すか、環境の値を置いて無視されることを確かめる形にする。
- **84. 弱い「来ない」の確かめ**（`drive_delivery_tests.rs:229-253` の `try_recv() == Empty` は hello を受けた直後で、アクターの送りと競る）。直し＝ほかのファイルと同じ `recv_timeout(NEG_WINDOW)` の形にする。
- **85. 範囲の中の重複したテストの道具**（`drive_choice_tests.rs:341-343` = `drive_choice_timeout_tests.rs:15-17`、「Choice の合図を待つ」輪が 2 つ（`drive_choice_tests.rs:27-34`・`:469-476`）、3 択のメニューの字面が 3 つ（`drive_choice_tests.rs:204-210`・`:440-446`・`drive_delivery_tests.rs:611-617`）、`StartTalk { epilogue: Vec::new(), … }` の字面 37 か所（`StartTalk::new` が在る）、`two_sinks(x, NoopSink)` の名残）。直し＝1 つの道具にまとめる。
- **86. 範囲の外のファイルにある、sysvar.rs の古い行の番号**（`crates/areka/src/emo2_boot/spine_conformance_script.rs:377-378`。`sysvar.rs:65-68` は今 `66-85`、`:45` は今 `:46`）。直し＝行の番号でなく名で指す。外＝`crates/areka/src/emo2_boot/spine_conformance_script.rs`（範囲の外のファイル＝議題 D14）。

**compile ほか**
- **87. Cue を組む所が 3 つの写し**（`compile.rs:495-502` `emit`・`:510-517` `emit_barrier`・`:587-595` の手書きの `Cue{actor:"0",…}`）。直し＝1 つの `emit(scope, at, dur, payload)`。
- **88. `append_epilogue` の horizon の計算は dola の `CueSheet::absolute_end_time` と同じ**（`compile.rs:577-581` と `crates/dola/src/cue/sheet.rs:85-91`）。直し＝本番は dola を呼ぶ（テストの写し `compile_sheet_tests.rs:429-437` は「独立導出で固定する」と明記された決め＝残す）。外＝dola は呼ぶだけで触らない。
- **89. 「2 つの合図の列が `cue_eq` で等しい」の写しが 5 つ**（`compile_char_code_tests.rs:10-13`・`compile_choice_timeout_tests.rs:155-160`・`compile_anchor_tests.rs:132-135`・`compile_strict_tests.rs:113-117`・`compile_font_tests.rs:143-150`）。直し＝`compile_test_support.rs` に 1 つ。
- **90. `compile_strict_tests.rs:234-240` の `Recording` は `drive_test_support::RecordingSink`（`drive_test_support.rs:54-77`）の作り直し**。直し＝共有の道具を使う。
- **91. `has_choice` は輪の後で合図を全部もう一度なめる**（`compile.rs:455-457`）。直し＝`Choice` の腕で真偽を立てる。
- **92. `CompiledTalk.sheet` の doc が事実と違う**（`compile.rs:603`「`CuePayload::Command` のみ」だが、選択待ちの区切り `:459-465` も入る）。直し＝doc を直す。
- **93. `append_epilogue` は `pub` だが使うのは `drive.rs:260` だけ**（外の呼び手 0）。直し＝`pub(crate)`。

### 落とした指摘（確かめられなかったもの）

- 「`TalkNotice`・`RecordingSink`・`NoopSink` のテストの道具が、areka-ghost・areka-seriko・dola のテストに写されている」（下請けの drive の N10）。具体的な失敗の筋書きが無い（シンクの形が変われば全部コンパイルで止まるので、黙って食い違うことはない）。直すには新しいテストの道具のクレート＝依存の追加が要り、同じウェーブの約束 ⑶ に反する。audit の「筋書きが無ければ指摘でない」に従って落とした。

### まとめ（Verdict と Lean）

- **Verdict**: 先に直すのは 1・2（信用できないゴーストがシェルの置き場をフォルダの外へ向けられる）、6（巨大な範囲のメモリの食い尽くし）、7（初回起動の記録が消える）、8（時刻のずれ）。
- **Lean**: 約 −1,500 行（package の引き口 約 270・3 つの `model_tests.rs` 約 600・重複したテストと落ちないテスト 約 500・`error.rs` 71・`Move` の型と腕ほか）、依存 −1（`areka-sakura` の `thiserror`・ワークスペースのほかの使い手は残る＝議題 D2）。

## 4. ukadoc で確かめた正典（要件 2.4）

| 指摘 | ukadoc の項目と原文（逐語） |
|---|---|
| 1・2 | 「seriko.defaultsurfacedirectoryname,ディレクトリ名」— 「初回起動時や一度もシェルを切り替えたことのない時に、標準で読み込まれるシェルのディレクトリ名を指定する。」「既定値: master」（https://ukatech.shillest.net/ukadoc-spec/descript/ghost/#seriko.defaultsurfacedirectoryname） |
| 4 | descriptブレス記述例 — 「descript { version,1 / collision-sort,ascend / animation-sort,ascend }」（https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/#introduction_descriptbrace）。「animation-sort,ソート順序」は「[Shell設定 - surfaces.txt > descriptブレス]」の項・「既定値: descend」（同 `#animation-sort`） |
| 5 | 「animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X座標,Y座標(,オプション...)」— 「SSPのみ「最小ウェイト-最大ウェイト」とマイナス文字で区切った書き方もでき、その場合は最小～最大間でランダムに選択される。」（同 `#animation-star-.pattern-star`） |
| 3 | 「\n[パーセント]」— 「通常のパーセント分改行する。負の値を指定すると戻る」（https://ukatech.shillest.net/ukadoc-spec/list/sakura-script/#n-2） |
| 24 | さくらスクリプトのエスケープ — 「スクウェアブラケット内に複数の引数を持つタグの第２引数以降で、「,」を引数の内容としたい場合、その引数全体を""で囲うことで全体を一つの引数として扱える。」（同 `#notes_escape`） |
| 25・26 | 「\q[ID][タイトル]または\q*[ID][タイトル]」— 「[旧仕様]選択肢を表示。*は通し番号」（同 `#q-ID-q-star-ID`） |

- 27（引数を取らないタグ＋`[`）・33（`}` の抜け）・34（閉じていない最後のブレス）・36（読めない数）・47（`+`・先頭の 0）・71（見出しと `{` の間の行）・6 の上限値・18 の `12px` は、ukadoc の字面では決まらない＝SSP の MCP で測る（記憶「ukadoc で読めない点は議題の前に SSP の MCP で測る」・測るのは SSP の操作の許可を取ってから＝調べ物 R1）。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1（全部洗い出す） | 本書 3 節 | 無し（116 ファイル・93 件・Not checked 0） |
| 2（3 つの組とも直す） | 直す先は範囲の 2 クレートに収まるものが大半 | **Constraint**: 範囲の外へ及ぶ件 21 件（下の 6 節）。**Unknown**: 議題になる 17 件の裁定 |
| 3（壊れたら落ちるテスト） | 範囲のテストは全部決定論（実 GPU・実機の根を使うテスト無し） | **Missing**: Must fix の 8 件と、バグ・危ない所の Should fix（9・10・11・14・17・18・21・24〜27・29・33・34・36・39）に、落ちるテストが無い |
| 4（境界と約束） | `doc/` を名指す 3 行は直しに無関係 | **Constraint**: 依存の削除（41）が `Cargo.lock` に触るか（D2）・`doc/` の台帳の行の直し（4・D5）・範囲の外のファイルの行の番号（86・D14） |
| 5（決まり） | 1,000 行に近いファイル 5 本 | **Constraint**: 8 の直しで `compile_sheet_tests.rs`（611）・`compile_arm_tests.rs`（928）の期待値を書き直す＝足すなら新しいファイルへ。記録の無い失敗の道（9・14・17・18・29・34・39）は要件 5.2 そのもの |
| 6（0 件と緑） | — | 検証の段のタスク（かけ直し＋全体テスト） |

## 6. 範囲の外の呼び出し元・同じウェーブの spec へ及ぶ件

| 指摘 | 及ぶ先 | 持ち主 |
|---|---|---|
| 5 | `areka-seriko/src/table_calls.rs`・`areka-emo-compose/src/nesting.rs` ほか（案 b のとき） | 【C17】`audit-seriko`・`audit-emo-compose` |
| 6 | `areka-emo-compose/src/fold.rs`・`boxes.rs`（案 b のとき） | 【C17】`audit-emo-compose` |
| 7 | `dola/src/cue/schedule.rs`・`runtime.rs`（案 b のとき） | `audit-dola`（C18・今は走っていない） |
| 8 | `text_playback_duration` の形を変えるときだけ 4 ファイル | 【C17】`audit-ghost-catalog` ほか（推奨は形を変えない） |
| 1 | `areka/src/boot_resolve.rs`（寄せるときだけ） | 本体（crate 分けの前・走っていない） |
| 9 | `areka/src/emo2_boot/assets.rs`・`placement/source.rs`（読み直しを消すとき） | 本体 |
| 11 | `areka-ghost/src/catalog.rs` | 【C17】`audit-ghost-catalog` |
| 15 | `areka-seriko/src/bind.rs`・`bind_tests.rs`・`areka/src/offline_dump/mod.rs` | 【C17】`audit-seriko`・本体 |
| 18 | `areka-emo-present/src/balloon_definitions.rs`・`areka-emo-text` のテスト 2 本 | 【C17】`audit-emo-present`・emo-text（ファイルは足さない） |
| 28・30 | `areka/src/mcp/check_script_judge.rs` ほか `match` を持つ 4 ファイル | 本体 |
| 34・35 | `areka-emo-present/src/shell_target.rs` | 【C17】`audit-emo-present` |
| 36 | `areka-emo-compose/src/fold.rs`・seriko の別名の引き | 【C17】`audit-emo-compose`・`audit-seriko` |
| 38 | `areka-emo-atlas/src/manifest.rs`＋各クレートのテストの字面 | `audit-emo-atlas`（C19）・【C17】3 本 |
| 43 | `areka/src/mcp/check_script_judge.rs` | 本体 |
| 46 | `areka/src/placement/persist.rs`・`areka-ghost` の 9 ファイル | 本体・【C17】`audit-ghost-catalog` |
| 59 | `areka-emo-text`（選択肢の見た目とテスト） | emo-text の見直し（C18） |
| 60 | `areka/src/placement/` | 本体 |
| 76 | `areka-emo-compose/src/boxes.rs`・`areka-emo-present/src/shell_target.rs`・emo-text のテスト | 【C17】`audit-emo-compose`・`audit-emo-present` |
| 86 | `areka/src/emo2_boot/spine_conformance_script.rs` | 本体 |
| 4 | `doc/ukadoc-coverage/ledger/assets.toml:6098`・`:7005` | 【C17】`doc-tree-tidy`（`doc/` の持ち主） |

- 【C17】の相手に及ぶ件は、触る前に相手のセッションと申し合わせる（要件 4.2）。`wintf` の 2 本（`audit-wintf-graphics-world`・`audit-wintf-runtime`）に及ぶ件は 0。

## 7. 直し方の選択肢（Options）

- **案 A（今のファイルを直す）**: 指摘ごとに、今のファイルの中で消す・まとめる・記録を足す。新しいファイルは、1,000 行の番人を越えるテストの置き場だけ。ponytail の「いちばん小さな完全な変更」に合う。✅ 差分が小さい・呼び出し元が少ない ❌ 35（ブレスの輪 4 つ）のような形の直しは、触る所が 4 ファイルに広がる。
- **案 B（新しい部品を置く）**: 35 の「閉じたブレスの列」を字句の段の新しい公開の形にし、43 の strict の判定を `strict` の新しい関数にする、など。✅ 写しが 1 つに減る ❌ 公開の形が増え、範囲の外（`check_script_judge.rs`・`shell_target.rs`）の直しが要る。
- **案 C（組み合わせ・推す）**: 基本は A。写しを 1 つにする件（13・35・43）だけ範囲の中に新しい関数を置く。範囲の外へ及ぶ件は、相手のセッションと申し合わせてから最後の段にまとめる。

## 8. 規模と危うさ

- **規模: L（1〜2 週）**。93 件のうち多くは数行だが、8（時刻を整数へ・期待値の書き直し）・7（dola との境目）・35（ブレスの輪）・28／30（範囲の外の `match`）が重い。
- **危うさ: 中**。直し方の型は知っているが、8 は合図の時刻のビットの形が変わり多くのテストの期待値が変わる。7 は dola の冪等の守りの読み違いの余地がある。③の件は SSP で測るまで振る舞いが決まらない。

## 9. タスクの見積もり

- ファイルのまとまりごとに切って **約 27 タスク**: package 4（1・2／9・14・47〜49／10・11・13／12・15・16・46・50〜52）・balloon／charset／kv 3・sakura（parsers）5（3／24〜27／28・30／29・64〜67／テスト 31・32）・shell 5（4・5／6・36／33・34・35・68〜71／38・76／テストと doc 37・72〜75）・drive／strict／sysvar 3（7／39・40・77〜85／86）・compile 4（8／41〜43／44・45・89・90／87・88・91〜93）・検証 3（範囲の外の呼び出し元の申し合わせと直し・audit のかけ直し・全体テスト）。
- **20 タスクを越える**＝要件 1.6 の議題（D1）。

## 10. 設計への申し送り（議題の候補と調べ物）

### 議題の候補（要件の討議へ）

- **D1. spec を 2 本に分けるか**（要件 1.6）。案＝(a) 推す：1 本目＝Must fix＋Should fix（45 件・約 19 タスク）、2 本目＝同じ範囲の Nice to have（48 件・約 8〜10 タスク）・(b) クレートで分ける（parsers／sakura）・(c) 分けない。(a) は「必ず直す」が早く着地し、v0.2.0 の 8 本の待ちが短くなる。
- **D2. 依存を消すこと（41 の `thiserror`）が同じウェーブの約束 ⑶「`Cargo.lock` の席は空けておく」に触れるか**。案＝(a) 推す：`SinkSend` とそのテストだけ消し、`thiserror` の削除は約束の外のウェーブへ回す・(b) 消す（`Cargo.lock` の 1 行が変わる）。
- **D3. 5（ウェイトの `最小-最大`）の直し方**。案＝(a) 推す：範囲を読み取り最小値を待ちにして `warn!`、ランダムは `/kiro-discovery` で起票・(b) 正典どおりモデルと下流まで直す（新しい機能に当たりうる）。
- **D4. 6（巨大な範囲）の上限の置き場と値**。案＝(a) 推す：読む側で幅に上限（SSP で測った上で十分大きく）・(b) emo-compose の fold で。
- **D5. 4 の直しに合わせて網羅台帳（`doc/ukadoc-coverage/ledger/assets.toml`）の 2 行を直すか**。`doc/` は `doc-tree-tidy` の持ち物で、要件 4.5 は名指しの行の話。案＝(a) 推す：`doc-tree-tidy` と申し合わせて直す・(b) 台帳は直さず起票。
- **D6. 7（初回起動の記録）の直しの置き場**。案＝(a) 推す：drive の中だけで済むか先に確かめ（R4）、済むなら drive・(b) dola を直す（`audit-dola` は C18）。
- **D7. 9 で、呼び手の読み直し（`assets.rs`・`source.rs`）まで消すか**。案＝(a) 推す：`warn!` を足すだけ・(b) 結果に誤りを持たせて読み直しを消す（範囲の外）。
- **D8. さくらスクリプトの正典に関わる 4 件（24〜27）**。SSP で測ってから決める。推す＝24 は直す（正典の字面どおり）・25 は直す・26 と 27 は測った結果に合わせる。
- **D9. 完了 spec の要件を覆す 2 件（28 `Move`・30 `#[non_exhaustive]`）**。推す＝どちらも直す（28 は後の裁定「汎用キャリア 1 本」が上位・30 は `publish = false` で後方互換の理由が無い）。
- **D10. shell の崩れた入力の扱い 5 件（33・36・38・71、と 34）**。推す＝34 は記録だけ足す・36 は 0 に倒さず飛ばす・38 は消す・33 と 71 は SSP で測ってから。
- **D11. 35 で、字句に分けるのを 1 回にまで絞るか**（範囲の外の `shell_target.rs` に及ぶ）。推す＝範囲の中で輪を 1 つにするところまで。
- **D12. balloon の型の決めを覆す 3 件（59・60・61）**。推す＝61 は `pub(crate)` にする（呼び手は変わらない）・59 と 60 は範囲の外の直しが大きく、得が小さいので残す（①として裁定を仰ぐ）。
- **D13. 79（`choice_notified` を戻す継ぎ目）**。推す＝消す（将来の機能のための継ぎ目で、今は効かない）。②として残すかを仰ぐ。
- **D14. 86（範囲の外のファイルの古い行の番号）を本 spec で直すか**。推す＝名で指す形に直す（直しが要るのは本 spec の範囲のファイルが動いたから）。
- **D15. 17（charset の記録の重さ）**。charset-canon の「詳細ログ」と食い違う。推す＝`warn!`。
- **D16. 11（鍵の大文字小文字）**。目録の spec の SSP 互換の決め（R5）に package を合わせる。推す＝合わせる（`audit-ghost-catalog` と申し合わせ）。
- **D17. 12・41 の「将来の継ぎ目」の注釈**（②の正式な印の形ではない）。推す＝消す。

### 調べ物（Research Needed）

- **R1. SSP の MCP で測る**（議題の前・SSP の操作は許可を取ってから）: 27（`\1[…]`・`\e[…]`）・26（`\q0[ID][タイトル]`）・33・34・71（崩れたブレス）・36（`surface.append10,abc`）・6（受ける範囲の幅）・18（`font.height,28px`）・47（`bindgroup0010`）。
- **R2. 8 の直しの後、合図の時刻の期待値を作り直すテストの洗い出し**（`compile_*_tests.rs` と、範囲の外で時刻を比べるテスト）。
- **R3. 30 で `match` を全部の腕にするとき、範囲の外の `match` の数**（今は 5 ファイル）。
- **R4. 7 で、dola の `TimedSchedule::tick` の冪等の守り（`schedule.rs:195`）が、解決の直後の同じ時刻の `tick` を通すか**（通るなら drive の中だけで直る）。

## 次の段

- 要件の討議（`/kiro-requirements-discussion audit-parsers-sakura`）で、本書の 93 件を要件に落とし、D1〜D17 を裁定する。audit 系の議題は推奨案のまま承認してよい（2026-11-04 開発者「audit系は推奨案でよい」）。

## 11. SSP の MCP で測った結果（2026-11-05・要件 7.8）

SSP（`C:\wintools\ssp\ssp.exe`・ゴースト「えも2DEBUG」）を起こし、`mcp__ssp__sakurascript` と `mcp__ssp__dump_balloon` だけで測った（環境には手を入れていない）。測り終えて `\-` で降ろし、127.0.0.1:9276 が空いていることを確かめた。

| 指摘 | 送った台本 | SSP の振る舞い（dump_balloon の目視） | 直し方への含み |
|---|---|---|---|
| 26 | `\0\b[2]\![set,choicetimeout,20000]A\q0[OnMeasureYes][はい]B\q1[OnMeasureNo][いいえ]C` | `\q0[…][はい]`・`\q1[…][いいえ]` は **選択肢**として出る（タイトル「はい」「いいえ」だけが見え、`0[OnMeasureYes]` の字は出ない）。選択待ちになり、時間切れでゴーストの時間切れのトークが出た | 通し番号の 1 桁の数を `legacy_q_star` で受け、旧形の選択肢として扱う（見える字に `0[ID]` を出さない） |
| 27 | `\0\b[2]ア\1\b[2][小声で]そうなの？\0イ\e[注]です` | `\1` は **話し手を切り替え**、`[小声で]そうなの？` は**地の文として字のまま**出る。`\e` は**トークを終え**、後ろの `[注]です` は出ない | 角括弧の道に入るのを引数を取りうるタグだけにする直し（3 節 27 の直しの方向）が SSP と一致する。`lexer_word_boundary_tests.rs:165` の「角括弧形として読まれる」は SSP と違う＝テストを直す |

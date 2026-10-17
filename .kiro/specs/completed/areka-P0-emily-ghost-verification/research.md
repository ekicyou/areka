# ギャップ分析: areka-P0-emily-ghost-verification

- 実施日: 2026-10-17（ワークツリーの HEAD `f3c897f3`）
- 入力: 同じフォルダの `requirements.md`（確定済み）・`brief.md`・`spec.json`（`language: ja`）、steering の `product.md`・`tech.md`・`structure.md`
- やり方: ソースと検体の書庫を読むだけ（ビルド・テスト・実機の走行は回していない）。書庫の中身は `vendors/sample_ghost/emily4.nar` を読み取りだけで数えた。

---

## 1. 要約

- **インストールの扱い（要件 1）は、直す場所が 1 か所に集まっている。** 断っているのは `crates/areka-nar/src/plan.rs` の `companion_placement`（167〜199 行）の `files.is_empty() && dirs.is_empty()` の 1 か所だけ。ここを「その同梱を計画から外し、警告を 1 件積む」に変えれば、組み上げ・確定・巻き戻し・知らせの側（`install.rs`・`crates/areka/src/install/`）は、入った物の列が 1 つ短くなるだけで変更 0 で済む見込み。
- **足りない部品は 3 つ。** ⑴ 計画の段から警告を返す道（今の警告はマニフェストを読む段の `ManifestWarning` だけで、`InstallOutcome.warnings` は `manifest.warnings` の写し）。⑵ 「`*.source.directory` が書かれていたか」の情報（要件 1.4 の鍵の名前に要る。今の `Companion` は持っていない）。⑶ 拒否の語彙 `CompanionSourceMissing` の行き先（消すか残すか）。
- **検体の登記（要件 2）は、要件 1 が入れば 1 行と数の直しで済む。** 検体の窓口は `areka-nar` で展開するので、要件 1 が先に要る。登記の行は同梱のバルーン 0 個（`balloons: &[]`）なので、検体のパスの綴りの番人の見張る語は増えず、緑のままの見込み。
- **実機の検証（要件 3・4）には、要件に書かれていない前提が 2 つある。** ⑴ 検体の窓口が作るえみりだけの根にはバルーンが 1 つも無く、このままでは起動が「バルーンが無い」で止まる（`crates/areka/src/boot_resolve.rs` 285 行）。根にバルーンを並べる手が要る。⑵ えみりは `char2`〜`char4` の鍵を持つので、配置が 3 人目以降の窓を作る見込みがあり、debug 版では `debug_assert!(scope <= 1)` の 2 か所が panic になり得る（静的に読んだ見立て・実機で未確認）。
- **規模は M・危うさは中〜高。** 決まっている部分（インストールの扱い・登記・文書）は 8〜10 タスク。実機で見つかる直しの数は起こすまで分からない。要件 6.4 の 20 の線を、どこで測るかを設計で決めておく必要がある。

---

## 2. 今あるもの（要件ごとの対応）

印: **在る**＝そのまま使える／**直す**＝今あるものを変える／**無い**＝新しく要る／**不明**＝起こすか調べるまで分からない／**制約**＝守るべき決まり

### 要件 1: 取り出し元の無い同梱のバルーンを飛ばす

| 項 | 今あるもの | 印 | メモ |
|----|-----------|----|------|
| 1.1・1.3 | `plan.rs` の `build_plan` が同梱ごとに `companion_placement` を呼び、`?` で最初の 1 件の失敗で全体を断る | 直す | 同梱ごとに「置く」か「飛ばす」かを返す形にする。飛ばした同梱は計画の列に入れない |
| 1.2 | 「配下にファイルもフォルダも 1 件も無い」を `collect_tree` の結果が空かどうかで決めている（178〜185 行。フォルダのエントリだけの書庫も含む） | 在る | 判定はそのまま使える。変えるのは判定の後の扱いだけ |
| 1.4 | 警告は `ManifestWarning`（`error.rs` 251 行）で、`lib.rs` 121〜127 行が `tracing::warn!` で 1 件ずつ出す。記録の欄に `archive`（`.nar` のパス）が載る | 直す・無い | ⑴ 計画の段から警告を返す道が無い。⑵ 今の拒否の理由は鍵の名前を常に `{key}.source.directory` と綴る（`plan.rs` 182 行）。`*.source.directory` が書かれていないとき（`manifest.rs` 429〜432 行で `*.directory` の値に倒す）に `*.directory` の鍵を名乗る情報が `Companion` に無い |
| 1.5 | 知らせの側は `InstallOutcome.installed` の列から Reference を組む（`judge.rs` の `complete_ex_refs`・`complete_legacy_refs`） | 在る | 飛ばした同梱は列に出ないので、入った物がゴーストだけの成功と同じ形になる（Reference2 は空）。新しいイベント・語は要らない |
| 1.6 | 置き場のフォルダは計画に入った配置の分だけ作業フォルダに組み、確定で入れ替える（`install.rs`） | 在る | 計画から外せば作られず、すでに在る中身にも触れない |
| 1.7 | 拒否の語彙 `RefuseReason::CompanionSourceMissing`（`error.rs` 95〜97 行）・写し `judge.rs` 129 行（`InvalidType`） | 直す | 消すか残すかは議題（6 節の 1） |
| 1.8・1.9 | 取り出し元が在るときの道と、ほかの拒否の理由 | 在る | 変えない。既存のテストが緑のままであることで確かめる |
| 1.10 | えみりの `install.txt` は `charset,ASCII`・`type,ghost`・`name,Emily/Phase4`・`directory,emily4`・`balloon.directory,emily4`・`balloon.source.directory,balloon`。書庫に `balloon/` は無い（最上位は `ghost/`・`shell/`・`delete.txt`・`install.txt`・`readme.txt`・`thumbnail.png`・`updates2.dau`・計 260 件） | 在る | 変更後は `ghost/emily4/` だけが置かれ、警告 1 件（鍵 `balloon.source.directory`・値 `balloon`） |
| 1.11 | `boot_resolve.rs` の `resolve_balloon`（段 1 引数 → 2 記憶 → 3 descript → 4 同梱 → 0 個なら失敗 → 5 唯一 → 6 既定 `StayseeBalloon` → 7 無作為） | 在る・制約 | えみりは段 3（`balloon,emily4`）と段 4（ゴーストのフォルダの `install.txt` の `balloon.directory,emily4`）が当たらず、それぞれ警告 1 件（`descript_balloon_not_found`・`companion_balloon_not_found`）。配布物は `emo2-kakukaku` と `StayseeBalloon` を同梱する（`tools/package.ps1` 504 行）ので、利用者の手元では段 6 の `StayseeBalloon` になる。**根にバルーンが 0 個なら起動は止まる**（6 節の 5） |

### 要件 2: 検体 `emily4` の登記

| 項 | 今あるもの | 印 | メモ |
|----|-----------|----|------|
| 2.1 | 登記表 `SAMPLES`（`crates/sample-ghost-kit/src/lib.rs` 99〜135 行・7 行） | 直す | `Sample { name: "emily4", kind: SampleKind::Ghost, balloons: &[] }` の 1 行 |
| 2.2 | `SampleRoot::acquire` → `devroot::fresh_root` → `stage_into_cache` が `NarArchive::install` で展開する（`devroot.rs` 372 行） | 在る | 要件 1 が先に入っていないと、ここで断られる |
| 2.3 | 直書きの 7: `lib_tests.rs` の 5 行目・6 行目（文書の注釈）・117 行（注釈）・124 行（`SAMPLES.len(), 7`）・178 行（注釈）・180 行（関数名 `unknown_sample_fails_with_all_seven_known_names`）・184〜198 行（名前の一覧）・410〜412 行（`SAMPLES.len(), 7`） | 直す | brief の数え（数の主張 2・注釈 3）と合う。名前の一覧と関数名も直す |
| 2.4 | `every_registered_sample_lands_where_its_registry_row_says`・`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares` | 在る | 登記の行が同梱 0 個なので、展開の結果（ゴースト 1 つ）と突き合わせて緑になる見込み |
| 2.5 | `vendors/sample_ghost/README.md` 14 行（表の「未登記」）・43 行（「登記表にまだ載せていない」の段落） | 直す | 文書だけ |
| 2.6・2.7 | `emily4.nar` は 2,677,923 バイト。配布物の許可表は `ghost/` に `emo2` だけ（`tools/package.ps1` 503 行） | 在る | 触らない |
| 2.8 | 番人 `crates/log-capture-kit/tests/sample_path_guard_test.rs`。見張る語は登記表から組む。⑵ は `vendors/sample_ghost/<名>/`・`vendors/sample_ghost/<名>"`、⑷ は同梱のバルーンの名前から組む | 在る | 同梱 0 個なので ⑷ の語は増えない。`crates/areka/src/mcp/` のテスト 13 本の `C:\ssp\ghost\emily4` は ⑵ の形（`vendors/sample_ghost/` を含む）に当たらない。緑のままの見込み（登記のときに回して確かめる） |
| その他 | `crates/sample-ghost-kit/tests/nar_sample_path_test.rs` が `SAMPLES` を回して全検体の手で使う根を作る | 制約 | 検体 1 つ分（2.6 MB の展開）だけ、このテストと登記のテストの時間が延びる |

### 要件 3・4: 実機の検証と「何とか動く」の線

| 項 | 今あるもの | 印 | メモ |
|----|-----------|----|------|
| 3.1 | 実機の根は `target\` の下・絶対パス（前例: 完了 `areka-P0-ghost-standard-balloon` のタスク 6.2 で `claudia` を同じ形で起こした） | 在る | 前例の手順をそのまま写せる |
| 3.2 | 検体の窓口（`SampleRoot::acquire`／`nar-sample-path emily4`）か、右クリックメニューの「インストール…」 | 在る・不明 | 窓口の根はえみりだけでバルーン 0 個（6 節の 5） |
| 3.3 | `tools/sync-i686-helper.ps1`（全体テストの後に x64 版へ替わった補助プログラムを i686 版へ戻し、揃ったかを終了コードで返す） | 在る | 走らせる前にこれを回せば足りる |
| 3.4 | 記録の水準は `RUST_LOG`、自動終了は `AREKA_APP_SMOKE_EXIT_MS`（`crates/areka/src/app_exit.rs`・`main.rs`） | 在る | |
| 3.5 | 机の仕組み（`areka-impl-watch`）の負荷の机 | 在る | 他の spec の実機の回・負荷の走行と時間を分ける |
| 3.6 | MCP の `dump_surface`・`dump_balloon`・`get_log`・`raise_event` | 在る・不明 | debug 版の `dump_balloon` が時間切れで答えない件が 10-06 に 1 度出ている（`areka-P0-mcp-receiver-bugs` の件 2・未着手） |
| 4.1 | 32 ビットの YAYA の道は `claudia` で実機まで通っている | 在る・不明 | えみりの `yaya.dll` と `claudia` のものが同じ版かは見ていない |
| 4.2 | 時間で出る話（えみりの `yaya_config.txt` の `TALK_INTERVAL` は 120 秒） | 不明 | 3 人目の台詞（`\p[2]`）が起動・終了の辞書に 50 回・メニューの辞書に 36 回出る。窓の無いスコープへの台詞で会話の進みが止まらないかは不明 |
| 4.3 | えみりの撫では `aya_mouse_core.dic` が `OnMouseMove` を数えて起こす形 | 不明 | areka が当たり判定の上の `OnMouseMove` を届けることは既存の機能。人の手でのマウスの動きが要る（areka の窓はクリック透過で、画面の自動操作の右クリックが止まる前例がある） |
| 4.4 | areka の右クリックメニュー（OS 標準） | 在る・不明 | 3 人目以降の窓の上で右クリックすると `crates/areka/src/menu/captions.rs` 110 行の `debug_assert!` に当たり得る |
| 4.5 | 終了の台詞の後に終わる・終了コード 0 | 在る | |
| 4.6 | panic と致命の記録 0 | 不明 | 下の 5 節の見立て |

### 要件 5〜8

| 項 | 今あるもの | 印 | メモ |
|----|-----------|----|------|
| 5 | 検証の記録の型（同定・機械の構成・環境変数・走行ごとの日時・項目ごとの結果） | 無い | 文書だけ。前例は完了 spec の `tasks.md` の実機の節 |
| 5.4 | 起票は `/kiro-discovery` | 制約 | |
| 6.3 | SHIORI の `load` に渡すパスの形（`…\ghost/master`・区切りの混在・末尾の区切りなし。出どころは `crates/areka-parsers/src/package/resolve_shell.rs` の `GHOST_MASTER`） | 制約 | 直すなら全部のゴーストへ渡すバイト列が変わる＝要件どおり開発者に聞く |
| 6.5 | 上書きする完了 spec の要件 | 直す | 要件が名指ししているのは `areka-P0-nar-install` の 4.4（正しくは 5.4。要件の討議で直した）だけ。ほかにも上書きになる要件がある（6 節の 1） |
| 7.1〜7.4 | 計画のテスト `plan_tests.rs`（925 行）・`plan_source_path_tests.rs`（390 行）・`plan_roots_tests.rs`、語彙のテスト `lib_vocabulary_tests.rs`、写しのテスト `judge_tests.rs` | 直す・無い | `plan_tests.rs` は 1,000 行の目安に近いので、新しい場面は新しい兄弟のテストへ置く |
| 8.1・8.2 | `doc/COMPAT_ARCHITECTURE.md` §8 の表に、同梱の番号の読み方（331 行）・`*.source.directory` の読み方（332 行）・`*.directory` の区切り（333 行）の行がある。失敗理由の語の写しの行（287 行）が `CompanionSourceMissing` を `invalid type` に挙げている | 直す | 行を 1 つ足し、287 行の列挙を直す（語彙を消す場合） |
| 8.3 | 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` 3863 行（`*.source.directory`）。注記は「壊れ方: 明示エラー」で、取り出し元が無いときの扱いは書いていない | 直す | |
| 8.4 | えみりが使う項目の台帳の行 | 不明 | 実機の結果しだい |

---

## 3. 実装の案

### 案 A: 今の部品を広げる（計画が「飛ばした同梱」を返す）

- `plan.rs` の `companion_placement` を「置く配置か、飛ばした理由か」を返す形にし、`build_plan` が飛ばした分を警告の列に積んで、配置の列と一緒に返す。
- `install.rs` の `commit_all` が作る `InstallOutcome.warnings` に、マニフェストの警告の後ろへ計画の警告を足す。記録は今の `lib.rs` の `warn!` の 1 か所から出る（`archive` の欄に `.nar` のパス）。
- 警告の型は `ManifestWarning` に 1 変種を足す（例: 「取り出し元が無いので同梱を飛ばした」・鍵と値つき）。`ManifestWarning` は `areka-nar` の外から 1 か所も名前で使われていない（全ワークスペースを検索して 0 件）ので、足しても外は壊れない。
- 鍵の名前のために、`Companion` に「`*.source.directory` が書かれていたか」を足す（`manifest.rs` の `collect_companions` で決める）。
- 触るファイル: `plan.rs`（319 行）・`install.rs`（624 行）・`manifest.rs`（485 行）・`error.rs`（338 行）と、新しい兄弟のテスト 1 本、既存の拒否のテスト 3 本（`plan_tests.rs` 2 本・`plan_source_path_tests.rs` 1 本）の書き替え。
- 良い所: 変更が `areka-nar` の中で閉じる。製品の `areka` 側は、語彙を消す場合の `judge.rs` の 1 行だけ。
- 悪い所: 記録の文言が `[areka_nar] manifest entry skipped`（マニフェストの鍵を読み飛ばした）のままで、「同梱を置かなかった」という事実とずれる。`ManifestWarning` の名前も「マニフェストを読む段の警告」を言っている。

### 案 B: 新しい部品を作る（計画の段の警告を別の型にする）

- 計画の段の警告を別の型（例: 「置かなかった物」の列）にし、`InstallOutcome` に別の欄として持たせ、`lib.rs` で別の文言の `warn!` で出す。
- 良い所: 記録の文言と型の名前が事実と合う。後で別の「置かなかった物」が出たときの受け皿にもなる。
- 悪い所: `InstallOutcome` の公開の形が増える（呼び手は `sample-ghost-kit`・`crates/areka/src/install/`）。受け皿の 2 本目は今は要らない。

### 案 C: 両方を混ぜる

- 型は `ManifestWarning` に 1 変種（案 A）、記録の文言だけを変種で分ける（`lib.rs` の `warn!` で変種を見て文言を選ぶ）。
- 良い所: 公開の形を増やさず、記録の文言は事実と合う。
- 悪い所: 記録の出口で変種を見分ける分岐が 1 つ増える。

### 拒否の語彙の扱い（どの案でも別に決める）

| | 消す（14 → 13） | 残す（使われない語彙） |
|---|---|---|
| 触る所 | `error.rs`（「14 変種で閉じる」の注記）・`judge.rs` 129 行・`judge_tests.rs` 219 行・`lib_vocabulary_tests.rs`（164 行の場面・`observed.len(), 14`）・`error_tests.rs` 69・289 行・`COMPAT_ARCHITECTURE.md` 287 行 | `lib_vocabulary_tests.rs` の「どの語彙にも固定入力が 1 つ在る」が満たせなくなる（作れる入力が無い）。完全一致の突き合わせを曲げるか、除外の宣言が要る |
| 上書きする完了要件 | `areka-P0-nar-install` 5.4・9.2（語彙に「取り出し元フォルダが無い」を含む）・9.3（拒否の固定入力に「取り出し元フォルダ無し」）／`areka-P0-install-companion-reading` 2.5・`areka-P0-install-companion-canon` 2.5（今どおり断る） | 同じ（5.4・2.5 は残す場合も上書き） |
| 写しの網羅 | `failure_word` はワイルドカードの腕を置かないので、消せばビルドが場所を教える | 使われない腕が残る |

---

## 4. 規模と危うさ

- **規模: M（3〜7 日）。** 決まっている部分は、インストールの扱い（3〜4 タスク）・登記（1〜2）・文書と台帳（1〜2）・実機の検証と記録（2）＝8〜10 タスク。直しは起こしてから数える。
- **危うさ: 中〜高。** インストールの扱いと登記は、決まった型を広げるだけで低い。高いのは、SSP 前提のゴーストを初めて起こす部分で、次の 5 節の見立てのどれが当たるか分からないこと。特に 3 人目以降の窓の件は、当たれば直す場所が `areka-P0-extra-character-windows` の持ち場と重なる。

---

## 5. えみりで崩れそうな所の見立て（実機の前に）

書庫とソースを読んで立てた見立て。どれも実機で確かめるまで「崩れた」とは書かない。

| # | 見立て | 根拠 | 当たったときの扱いの候補 |
|---|--------|------|--------------------------|
| 1 | **根にバルーンが 0 個で起動が止まる** | 検体の窓口の根はえみりだけ。同梱を飛ばすので `balloon/` は空。`boot_resolve.rs` 285 行が 0 個で失敗を返し、`boot_config.rs` 313 行が「バルーンが無い」の告知にする | 不具合ではなく根の組み方の問題。根に `StayseeBalloon` などを並べる（6 節の 5） |
| 2 | **3 人目以降の窓が作られ、debug 版で panic する** | ゴーストの descript に `char2.name,Emilio`、シェルに `char2.*`・`char3.seriko.alignmenttodesktop`・`char4.seriko.alignmenttodesktop`。配置の `detect_scopes`（`placement/config.rs` 144〜164 行）は `char{n}.` の鍵から scope 2・3・4 を足し、配置の列を絞る所は見つからなかった。`spawn.rs` は配置ごとにキャラクターの窓を作る。一方、`input_events/mod.rs` 398 行と `menu/captions.rs` 110 行が `debug_assert!(scope <= 1)`、起動の絵の側は `emo2_boot/mod.rs` の `derive_scopes` が `[0, 1]` 固定 | 要件 4.6（panic 0）を妨げるので、要件 6.1 の「本 spec で直す」に入る。直す場所が `extra-character-windows` の持ち場と重なる（6 節の 6） |
| 3 | **3 人目の台詞で会話が止まる・欠ける** | `\p[2]` が起動・終了の辞書に 50 回・メニューの辞書に 36 回 | 台詞が出ないだけなら要件 4.7 で妨げにしない。会話の進みが止まるなら要件 4.2 を妨げる＝直す |
| 4 | **YAYA が辞書を読めない** | `load` に渡すパスの形（brief 2026-10-10 の申し送り）。えみりの `yaya.txt` は UTF-8（BOM つき）・辞書 31＋5 本＋Lilith | `claudia` で動いているので最初には疑わない。当たれば要件 6.3 で開発者に聞く |
| 5 | **起動の知らせの順の違いで、えみりの分かれ道が変わる** | areka は `OnBoot` の後に `basewareversion` を送る（`areka-kanade/src/schedule/boot.rs`）。えみりは `On_basewareversion`・`On_installedghostname` を持ち、`yaya_config.txt` の `IGLIST_ACCEL` は SSP／CROW なら `installedghostname` で一覧を作る | 見た目や一部の機能の欠けなら起票。止まるなら直す |
| 6 | **SSP 専用の台本の命令が多い** | `\![set,choicetimeout]` 12・`\![set,alignmenttodesktop]` 8・`\![call,ghost]` 7・`\![get,property]` 6・`\![raiseplugin]` 4・`\![open,inputbox]` 4・`\![set,trayicon]` 2・`\![embed,…]` 3・`\![enter,passivemode]` など | 読めない命令で落ちない限り、起票か「方針で受けない」 |
| 7 | **見た目の違い** | α を持つ PNG は 132 枚中 14 枚・`.pna` 27 枚（受けない裁定）・element の `replace` 6（`ComposeMethod::Replace` は名前は知っているが描かない＝`draw-methods-canon`）・オーナードローのメニューの絵・着せ替え（`sakura.bindgroup100.*`・`sakura.menuitem0`） | 要件 4.9 で不合格にしない。起票か「方針で受けない」 |
| 8 | **descript の最初の面の指定を読まない** | `char2.seriko.defaultsurface,200`。最初の面は `emo2_boot/assets.rs` の 0 と 10 の決め打ち | 3 人目の窓が出ないので、今回は表に出ない見込み |
| 9 | **起動の記録の警告がえみりの中身から出る** | 段 3・段 4 のバルーンの「見つからない」警告 2 件、未知の描画メソッドの警告など | 要件 5.5 で、理由を書いて数から外す |

---

## 6. 設計へ持ち越す議題（答えで作業が変わるもの）

1. **拒否の語彙 `CompanionSourceMissing` を消すか残すか。** 消すと語彙は 13 になり、`areka-P0-nar-install` の 9.2・9.3 も上書きする。残すと語彙のテストの「全部に固定入力が在る」が崩れる。どちらでも `areka-P0-install-companion-reading` の 2.5（今どおり断る）は上書きになるが、要件の文書は `nar-install` の 4.4 しか名指ししていなかった（要件の討議で 5.4 に直し、`install-companion-reading` 2.5・`install-companion-canon` 2.5 も足した）。要件 6.5・8.2 に合わせて、上書きする要件の番号の全部をどこに書くかも決める。
2. **警告の型と記録の文言。** `ManifestWarning` に 1 変種（案 A）／別の型（案 B）／型は 1 変種で文言だけ分ける（案 C）。`InstallOutcome.warnings` の並び（マニフェストの警告の後ろに計画の警告を足すか）も決める。
3. **警告に載せる鍵と値。** 要件 1.4 は「`*.source.directory` が書かれていないときは `*.directory` の鍵」「書かれていた取り出し元の値」。⑴ `Companion` に「書かれていたか」を持たせる形でよいか。⑵ `..` を取り除いた値（`SourceDirectoryCleaned` の後）のとき、載せるのは書かれていた値か、取り除いた後の値か（突き合わせに使ったのは後の値）。
4. **既存の拒否のテスト 3 本の行き先。** `plan_tests.rs` の `refuses_a_companion_whose_source_folder_is_empty`・`refuses_a_companion_whose_source_folder_is_absent` と、`plan_source_path_tests.rs` の 3 つの形（配下の外だけ・フォルダのエントリだけ・前方一致する兄弟だけ）を、「飛ばして警告 1 件」の判定へ書き替えるか、新しい兄弟のテストへ移すか。`plan_tests.rs` は 925 行で、足すと 1,000 行を超える。
5. **実機の根の組み方。** 検体の窓口の根にはバルーンが 0 個で、起動が止まる。案は 3 つ。
   - (a) `claudia` の前例と同じく、同じ根に `StayseeBalloon.nar` も展開する。
   - (b) 置き場の並び（`AREKA_ROOT` の `;`）で、えみりの根と `StayseeBalloon` の根を並べる。
   - (c) 配布物と同じ形の根（`emo2`・`emo2-kakukaku`・`StayseeBalloon`）を作り、areka の「インストール…」でえみりの `.nar` を入れる。利用者の報告の再現に一番近く、要件 3.2 の「`.nar` を入れる操作」と要件 1.11 の「既定のバルーン」（段 6 の `StayseeBalloon`）を同じ走行で確かめられる。
   どれを使ったかは要件 3.2 のとおり記録に書く。
6. **3 人目以降の窓で panic したときの直し方（当たった場合）。** ⑴ 配置の側で窓を scope 0・1 に絞る（起動の絵の側の `derive_scopes` と答えを揃える）。⑵ `debug_assert!` を外して scope 2 以上を縮退で受ける。どちらも `extra-character-windows` の持ち場（窓の数の導き方を 1 か所にする・2 人までの検査を外す）とかぶる。後でその spec が作り直す前提の最小の直しにするか、その spec の要件の一部を先に食べるかを決める。
7. **直しのタスクの予算と、20 の線を測る時点。** 決まった部分で 8〜10 タスクを使う。実機の最初の走行の後に、見つかった崩れの一覧と直しの見積もりをそろえ、そこで 20 を超えるかを測る形でよいか（要件 6.4）。
8. **要件 7.3（`emily4.nar` を入れるとゴーストだけが入る）のテストの置き場。** `sample-ghost-kit` の登記の往復のテストで足りるか、`areka-nar` の側に検体を読む専用のテストを 1 本足すか（警告 1 件の中身まで見るなら後者が要る）。
9. **人の手が要る項目の段取り。** 撫で（要件 4.3）と右クリック（4.4）は人のマウス操作が要る。areka の窓はクリック透過で、画面の自動操作の右クリックが止まる前例がある。開発者が手で操作するか、許可を得て OS の入力の送り込みで行うかを、実機の手順に書く。
10. **警告のダイアログの出し方（要件 1.12・要件の討議の議題 5 で 2026-10-17 に追加）。** ⑴ 今の告知（`crates/areka/src/alert.rs`、アプリが終わるときの最後の手段）を使うか、別の出口を作るか。⑵ ゴーストへの `OnInstallComplete`／`Ex` の前に出すか後に出すか。⑶ 出している間に UI スレッドとゴーストの会話を止めないか。⑷ 決定論のテストで「出す求め」を数えるための継ぎ目。

---

## 7. 調べ物（設計の段で確かめる）

- **調べ物 1**: 配置がえみりで scope 2〜4 の窓を本当に作るか、作るならその窓が当たり判定やポインタの出来事を受けるか（`placement/resolver.rs`・`measure.rs`・`spawn.rs` と `ghost_session.rs` の `prepare_ghost_windows` の流れ）。テストで `char2.name` を持つ固定入力を通せば、実機の前に分かる見込み。
- **調べ物 2**: 窓の無いスコープへの台詞（`\p[2]`）を areka がどう扱うか（文字の層へ接続しない・会話の終わりを待つ側が止まらないか）。`emo2_boot/frame/attach.rs` 304 行の「同じ scope のバルーン資産が無い」の道と、`balloon_visibility_phase.rs` 265 行の道。
- **調べ物 3**: debug 版の `dump_balloon` の時間切れ（`areka-P0-mcp-receiver-bugs` の件 2）が今も出るか。出るなら要件 3.6 の撮り方を `dump_surface` と記録に寄せる。
- **調べ物 4**: えみりの `yaya.dll` と `claudia` の `yaya.dll` の版の違い（同じ道で読めるか）。
- **調べ物 5**: SHIORI へ送る起動の知らせの順（`basewareversion` を `OnBoot` の後に送る）が、ukadoc の書く順と合っているか。合っていれば変えない（SSP の実測は根拠にしない）。

---

## 8. 次の段へ

- 要件の討議（`kiro-requirements-discussion`）で 6 節の議題を扱う。特に 1（語彙）・5（実機の根）・6（3 人目の窓）は、答えでタスクの数が変わる。
- その後、`/kiro-design areka-P0-emily-ghost-verification` で設計へ進む。

---

## 9. 要件の討議の結論（2026-10-17）

- 6 節の 1（語彙）: `CompanionSourceMissing` を消す（14 種 → 13 種）。上書きする完了要件は `nar-install` 5.4・9.2・9.3 の該当部分、`install-companion-reading` 2.5、`install-companion-canon` 2.5（要件 1.7・8.2）。
- 6 節の 5（実機の根）: 案 (c)。配布物と同じ形の根に、areka のインストールの操作でえみりの `.nar` を入れる（要件 3.2）。
- 6 節の 6（3 人目以降の窓）: 窓を scope 0・1 に絞る最小の直し。開発者「最低限パニックは回避して欲しい」を受け、実機で当たるのを待たずに、`char2` 以降の鍵を持つ固定入力の決定論のテストで panic しないことを確かめる（要件 6.7）。7 節の調べ物 1 は、このテストで答えを出す。
- 6 節の 7（20 の線）: 最初の実機の走行の後に、崩れた所の一覧と直しの見積もりをそろえて測る（要件 6.4）。
- 一部が入らなかったことの知らせ: 警告のダイアログを 1 つ出す（要件 1.12・6 節の 10）。
---

## 10. 設計の段の調べと決め（2026-10-17・kiro-spec-design）

- 種類: 既存の仕組みの拡張（軽い調べ）。新しい外部の部品は 0。ukadoc「Install設定」の `*.source.directory` を MCP で引き直し、取り出し元が書庫に無いときの扱いに沈黙していることを確かめた。
- 使った手引き: `kiro-spec-design` の規則（設計の原則・軽い調べ・まとめ方・設計の関門）。steering は `product.md`・`tech.md`・`structure.md`・`logging.md`。

### 10.1 調べ物の答え

- **調べ物 1（3 人目以降の窓）**: 作られる。`placement/config.rs` の `detect_scopes`（144〜164 行）が `char{n}.` の鍵（n が 2 以上）から scope を足し、`placement/mod.rs` の `prepare_stages_for_shell`（406・407 行）→ `resolver.rs` の `resolve_placement` → `spawn.rs` の `spawn_ghost_windows`（415〜472 行）が scope ごとにキャラクターの窓とバルーンの窓を作る。`ghost_session.rs` 282〜294 行がその全部にポインタ・メニュー・投げ込みの受け手を付ける。起動の絵の側（`emo2_boot/mod.rs` 289〜291 行 `derive_scopes` は `[0, 1]` 固定）は scope 2 の窓に絵を付けず、`attach.rs` 196〜200 行の `warn!` で済む。
- **panic の道**: debug 版の `debug_assert!(scope <= 1)` の 2 か所だけ。`input_events/mod.rs` 398 行（`char_scope`・マウスの移動・ダブルクリック・ドラッグ・右クリックの離しから来る）と `menu/captions.rs` 110 行（`visible_resource_for`）。ほかの道（`GhostWindows` の引き当て・測り・窓の位置・バルーンの面の引き当てなど）は scope 2 以上でも落ちない。
- **絞る場所**: `build_placement_config` は呼び手 3 か所（`placement/mod.rs` 406 行・`reseed.rs` 62 行・`emo2_boot/frame/switch.rs` 448 行）がすべて通る。その中の `detect_scopes` が根の 1 か所。
- **`areka-P0-extra-character-windows` の持ち分**: scope の導き方を 1 か所にまとめる・2 か所の `debug_assert!` を外す・3 人目以降の窓を正しく作る。本 spec の絞りは、その spec が後で外す前提の最小の直し。

### 10.2 設計の議題の決め（6 節の 2・3・4・8・9・10）

- **6 節の 2（警告の型）**: 案 B を採る。`ManifestWarning` に足さず、`InstallOutcome` に「飛ばした同梱」の欄 `skipped`（型 `SkippedCompanion`）を足す。理由は 2 つ。⑴ 要件 1.12 の警告のダイアログが、飛ばした同梱の置き場の名前を値として読む必要がある（警告の列を変種で選り分けるより素直）。⑵ 記録の文言（`[areka_nar] manifest entry skipped`）と事実がずれない。`InstallOutcome` を値で組むのは `areka-nar` の `commit_all` 1 か所だけ（全ワークスペースを検索）なので、欄を足しても外は壊れない。
- **6 節の 3（鍵と値）**: `Companion` に `source_written: Option<String>`（`*.source.directory` に書かれていた値・書かれていない／空なら `None`）を足す。鍵は `Some` なら `<接頭辞>.source.directory`・`None` なら `<接頭辞>.directory`。値は書かれていた値（`..` を取り除く前・長さは今の拒否の理由と同じく `bounded_value` で縛る）。`None` のときの値は `*.directory` の値。取り除いた後の値は今どおり `SourceDirectoryCleaned` の記録に載る。
- **6 節の 4（既存の拒否のテスト 3 本）**: 同じ場所で「飛ばして 1 件」の判定へ書き替える（`plan_tests.rs` の 2 本・`plan_source_path_tests.rs` の 1 本）。行は増えない。新しい場面（要件 7.1 の 6 つ）は新しい兄弟 `lib_skip_tests.rs` に置く（`open` から `install` まで通し、記録も捕まえる）。
- **6 節の 8（要件 7.3 の置き場）**: `sample-ghost-kit` の `lib_tests.rs` に 1 本足す。検体の窓口の内側（`devroot::nar_dir()`）から `emily4.nar` を読み、一時の根へ入れて、入った物・置かれなかった物・飛ばした 1 件の中身を見る。`areka-nar` から検体の場所を知る道は作らない（`areka-nar` は検体の在処を知らない、が今の決まり）。
- **6 節の 9（人の手の項目）**: 要件 3.8 のとおり AI が操作する。areka の窓のクリック透過で画面の操作の道具が効かないときは、開発者の許可を得て SendInput で行う。
- **6 節の 10（警告のダイアログ）**: ⑴ 告知の口 `alert.rs` に「OK だけの警告」の口を 1 つ足す（`ask_yes_no` と同じ形・`MessageBoxW` の `MB_OK | MB_ICONWARNING`・`AREKA_NO_ALERT` で抑える）。起動を止める告知（`raise`）は `error!` と赤い印なので使わない。⑵ 出すのは締めの知らせ（`OnInstallCompleteEx`／`OnInstallComplete`）を送った後。ゴーストの完了の台詞をダイアログの後ろに待たせない。締めの知らせが「閉じた」で返っても、入った事実は変わらないので出す。⑶ 出すのはインストールの背景のスレッド（利用条件の画面と同じ）。UI スレッドとゴーストの会話は止まらない。このとき門（終了の待ち）は既に手放しているので、終了も待たせない。⑷ 手続きの口 `InstallPorts` に 1 つ足す（`warn_skipped`）。テストの偽の口がその呼び出しを数え、題名と本文を持つ。

### 10.3 まとめ方の 3 つの見方

- **一般化**: 「取り出し元が無い」の扱いは、ゴースト・シェルの両方と、無印・番号付きの全部が `build_plan` の同梱の 1 本の道を通るので、直す所は 1 か所。ダイアログもメニュー・台本・投げ込み・網から取ってくる・起動中のゴーストの上書きの全部が `procedure.rs` の `archive_steps` を通るので 1 か所。
- **作るか借りるか**: ダイアログは OS の `MessageBoxW`（既存の `alert.rs` の口と同じ）。新しい部品・依存は 0。
- **削ったもの**: ⑴ `ManifestWarning` の変種と記録の出口の分岐（案 A・C）は採らない。⑵ 「置かなかった物」一般の受け皿は作らない（今は同梱のバルーンだけ）。⑶ 3 人目以降の窓は作り直さず、`detect_scopes` で n ≥ 2 を足さないだけにする（`char_scope_of` は呼び手が無くなるので消す）。2 か所の `debug_assert!` は残す（外すのは `extra-character-windows`）。⑷ 実機の根を組む道具は作らない（今ある `nar-sample-path` と複写で足りる）。

### 10.4 危うさ

- `detect_scopes` の絞りは、scope 2 以上を数えていた 3 本のテスト（`config.rs` 570 行 `t_c3_char_n_prefix_adds_scope_n`・`config_limit_tests.rs` 85 行・`reseed_tests.rs` 195 行）の期待を変える。`extra-character-windows` がやり直すときは、この 3 本と本 spec の新しいテストを戻すことになる（設計の「後で確かめ直す条件」に書く）。
- `examples/window-placement.rs` は `placement` を `#[path]` で読み込み、自前の `scope >= 2` の枝を持つ。絞った後はその枝に来ないが、ビルドは通る（枝は消さない）。
- 警告のダイアログが開いている間、同じ依頼の次の書庫は待つ（利用条件の画面と同じ）。
- `areka/src/main.rs` は 1,000 行ちょうど。本 spec は触らない。

## 11. 設計の討議の結論（2026-10-17）

- 議題 1（窓を絞った後の `\p[2]` の台詞の道・7 節の調べ物 2）: 案 1 で確定。実機の前に決定論のテストを 1 本置き、scope `{0, 1}` の配置で `\p[2]` を含む台本が最後まで進み、終了の台詞の後に終わることを確かめる。根拠は「file:line で確かめられる欠陥は実機を待たない」「決定論のテストは必達」と、止まる場合に直しの見積もり（要件 6.4）を実機の前に固められること。退けた案 2（実機の最初の項目にする）は、止まっていたときに直しが走行の後へずれる。
- 自明な修正 6 件（数の洗い出しの取りこぼし 5 か所と全体検索の 1 行・兄弟のテストの置き場・窓の scope の作り方・`shared_test_support` の公開・書庫の大きさ）は設計書へ反映済み。

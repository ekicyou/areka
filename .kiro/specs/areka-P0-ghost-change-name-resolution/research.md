# Gap Analysis: areka-P0-ghost-change-name-resolution

> 実測: 2026-09-27・本ブランチ（`claude/areka-p0-ghost-name-resolution-0d4fff`・main `55a2a1fd` の直後）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。要件（`requirements.md`）は確定済みで、本文書はそれを変えず、設計に渡す材料と要件ディスカッションに出す論点だけを書く。

## 1. 要約（3〜5 点）

- **経路は既に 1 本つながっている。** `\![change,ghost,名]` は `ChangeCueSink::emit`（`crates/areka/src/emo2_boot/change_cue.rs`）→ `drain_change_requests` → `request_ghost_switch`（`crates/areka/src/emo2_boot/ghost_switch.rs`）→ 純関数 `resolve_switch_target` → kanade へ `ChangeGhost`。本仕様が足すのは `list_ghosts` の直後・`resolve_switch_target` の直前の **1 段**（特別な名前 → フォルダ名）と、`decode_bare`（`crates/areka-parsers/src/sakura/decode.rs`）の **2 腕**だけ。`compile.rs`・kanade・`change_cue.rs` は触らない。
- **欠けているのは 3 つ。** ⑴ `random`／`sequential`／`lastinstalled` を解く判断（純関数）、⑵ `lastinstalled` の記録の置き場と書く口（製品コードに `lastinstalled` は 0 件）、⑶ `\+`／`\_+` の転記（今は `Raw` になって捨てられる）。どれも既存の前例（`pick_index` の注入・`Resource` の置き方・裸 `\f` の腕）に乗る。
- **要件に書かれていない接触が 3 か所ある（設計で拾う）。** ⓐ 既存テスト `each_canonical_bracketless_tag_yields_exactly_one_raw`（`parse_bare_tag_tests.rs`）が `\_+` を `Raw` として固定している。ⓑ 台帳の owner を本仕様にすると、`doc/ukadoc-coverage/roadmap-draft.md` に `[[spec]]` の行と `[briefs].count` の +1 が要る（ukadoc-survey の整合検査の腕 a・c・f）。ⓒ 台帳を実装済みにするには、ソースに `// ukadoc: <正典 URL>` の証拠行が要る（検査 `ImplementedWithoutEvidence`）。
- **推奨は A（既存ファイルの拡張）。** `ghost_switch.rs`（699 行）に純関数 1 本と `Resource` 1 つを足し、`request_ghost_switch` の中で呼ぶ。乱数は `request_ghost_switch` の中身を「乱数を引数に取る内側の関数」に分けて `pick_index` を渡す（`boot_into` が `resolve_balloon_for_ghost(&root, dir, pick_index)` に渡している前例と同型）。規模 **S**・リスク **低**。
- **設計判断として残るのは 6 件**（§7）。うち要件の読み替えになりうるのは「実機サインオフの向き（emo2 は `\+` を出せない）」と「argv で起こしたゴーストが根の中にあるときの `random`」の 2 件。

## 2. 現状の調査（既存の資産・前例）

### 2.1 切替の経路（完了 `ghost-shell-balloon-switch` が作ったもの・すべて実物で確認）

| 段 | 定義 | 確かめたこと |
|---|---|---|
| 台本の受け口 | `ChangeCueSink::emit`（`emo2_boot/change_cue.rs`） | `cue.command.as_command_carrier()` で `(name, params)` を開け、`(name, params[0]) == ("change","ghost")` だけ受理。`params.get(1)` を**無変形**で `ChangeRequestRaw { name, raise_event }` に載せ、`params[2..]` のうち `--option=raise-event` だけ真にする。`random` も `sequential` も `lastinstalled` もそのまま通る（受け口は名前を見ない）。 |
| 取り出し | `drain_change_requests`（`ghost_switch.rs`） | 受信端 `ChangeRx` を全件読み、`SwitchRequest { ghost: GhostSpec::Name(raw.name), raise_event, origin: ChangeOrigin::Automatic }` で入口へ渡す。 |
| 唯一の入口 | `request_ghost_switch(world, req) -> SwitchVerdict` | 順序は ⑴ `SwitchInFlight` が在れば `warn!(ghost_switch_busy)` → ⑵ `BootContext` が無ければ `warn!(ghost_switch_no_context, reason="boot_context")` → ⑶ `list_ghosts(&ctx.root)` → `resolve_switch_target(&entries, &req.ghost)` が `None` なら `warn!(ghost_switch_unknown)`・`NotFound` → ⑷ `current_folder = ctx.current.ghost.folder.clone()` → ⑸ `GhostSlot` の kanade 送出端が無ければ `NoContext` → ⑹ `sakura_name` を読み `KanadeMsg::ChangeGhost(ChangeRequest{ target: ChangeTarget{ sakura_name, name, dir }, origin, raise_event })` を送り `SwitchInFlight` を立てる。**特別な名前を解く場所は ⑶ の中（目録を読んだ後・突き合わせの前）**で、要件 5.1・5.6 が求める位置と一致する。今のフォルダ名は ⑷ で既に読んでいるので、読む位置を ⑶ の前へ動かすだけでよい。 |
| 純関数 | `resolve_switch_target(entries: &[GhostEntry], spec: &GhostSpec) -> Option<SwitchTarget>` | `Name` は `identity.name` → `identity.folder` の順、`Folder` はフォルダ名だけ。説明どおり大文字小文字を区別し、今のゴーストも除外しない。`sakura_name` は入口が後で埋める。 |
| 指し方 | `GhostSpec::{Name(String), Folder(String)}` | メニュー（`menu/ghost_frame.rs` の `ghost_frame_item`）は `Folder`、台本は `Name`。 |
| 出どころ | `ChangeOrigin::{Manual, Automatic}`（`crates/areka-kanade/src/change.rs`） | `as_ref_str` が `manual`／`automatic`。本仕様は `Automatic` のまま通す。 |
| 二重要求 | `SwitchInFlight`（NonSend） | 予約の有無で判定。要件 5.6「解決は予約の判定より後」は今の順序のまま満たされる。 |

### 2.2 目録と今のゴースト

- `catalog::list_ghosts(&BasewareRoot) -> Vec<GhostEntry>`（`crates/areka-ghost/src/catalog.rs`）: `subdirs` が `(folder, path)` を `dirs.sort()` で並べる＝**フォルダ名のバイト順**（`String::cmp`）。メニューの「ゴースト」枠（`ghost_frame_item`）はこの並びをそのまま子に出す。要件 3.1 の「メニューと同じ並び」はこの 1 か所で成り立つ。
- `GhostEntry { dir, identity: Identity { folder, name: Option<String>, … } }`。`ghost_switch.rs` は既に `areka_ghost::GhostEntry` を使っている（`use` 済み）。
- 今のゴースト: `BootContext.current.ghost.folder: Option<String>`（`crates/areka/src/boot_config.rs` の `BootContext`／`CurrentGhost`・`GhostDecision`）。**注意**: `resolve_ghost`（`boot_resolve.rs`）の段 1（argv）は `folder: None` を**必ず**返す。argv のパスが根の `ghost/` の中を指していても `None`。→ §7 の論点 2。
- `BootContext` は `#[derive(bevy_ecs::prelude::Resource)]`。`world.get_resource::<BootContext>()` で読む形が入口に在る。

### 2.3 乱数の前例

- `pick_index(n) -> usize`（`boot_resolve.rs`）: `RandomState::new().hash_one(n) % n`。新規依存 0。`ghost_switch.rs` は既に `use crate::boot_resolve::{…, pick_index, …}` している。
- 注入の前例: `resolve_ghost(inputs, pick: impl FnOnce(usize) -> usize)`／`resolve_balloon(inputs, pick)` は純関数で、本番は `boot_into` が `resolve_balloon_for_ghost(&root, &ghost.dir, pick_index)` に**関数を渡す**。要件 2.6「乱数を外から与える形」はこの形で足りる。

### 2.4 プロセス内の記録の前例

- `#[derive(Resource)] pub(crate) struct SessionEnded;`（`session_end.rs`）・`FirstExit(ExitOrigin)`（`app_exit.rs`）: 1 値の `Resource` を `world.insert_resource` で置き、`world.contains_resource`／`get_resource` で読む。`ghost_switch.rs` は `SwitchInFlight` を NonSend で置く前例も持つ。
- `lastinstalled` は製品コードに **0 件**（`crates/` を grep: `ghost_switch_tests.rs` の `unknown_names_warn_once_and_send_nothing` の 1 件と、`areka-parsers/src/shell/decode.rs` の無関係な `"random"` だけ）。ファイル・記憶（sylphya）へ書く経路は作らないので、`boot_resolve.rs`・`boot_config.rs`・`ghost_session.rs`・`main.rs` に触らずに済む。

### 2.5 `\+`／`\_+` の落ち方と転記の前例

- 字句: `lex(r"\_+") == [Token::Bare("_+")]`（`lexer_bare_tag_tests.rs` の `two_char_underscore_tags_consume_as_single_unit`）、`\+` も `Bare("+")`（`lexer_word_boundary_tests.rs` の `ONE_CHAR_WORDS` に `"+"`）。**字句は変えない**。
- 転記: `decode_bare(word)`（`decode.rs`）の腕は `e`・`c`・`-`・`n`・`0|h`・`1|u`・`f`、既定は `decode_passthrough_bare` → `Instruction::Raw("\\+")`。`"f" => Instruction::Font { args: Vec::new() }` が「裸の綴りを角括弧付きと同じ受け皿へ載せる」前例。各腕の直前に `// ukadoc: https://…#_5ce:1` の形の正典 URL 行がある（§2.7 の証拠行の規則）。
- 受け皿: `Instruction::GenericCommand { name: String, raw_args: Vec<String> }`（`crates/areka-parsers/src/sakura/model.rs`・`#[non_exhaustive]`）。`compile`（`crates/areka-sakura/src/compile.rs`）の `Instruction::GenericCommand { name, raw_args }` の腕が `CueCommand::command_carrier(name, raw_args)` に載せる＝**`compile.rs` は無変更で届く**。`Raw` は catch-all が `debug!("M-boot 外タグを無視")` で捨てる（`catch_all_ignored_set_is_raw_only` が固定）。
- `\![change,ghost,random]` の角括弧付きは `decode_bang` → `decode_passthrough_bang` → `GenericCommand { name: "change", raw_args: ["ghost","random"] }`。裸の `\+` をこれと**同じ値**にすれば、以降は角括弧付きと区別がつかない（要件 1.3 の「別名の転記」）。`raise_event` は `params[2..]` が空なので偽＝`OnGhostChanging` を送らない（正典 `\+`／`\_+` と一致）。

### 2.6 既存テストで書き換えが要るもの（実測）

| テスト | 今の固定 | 本仕様での扱い |
|---|---|---|
| `unknown_names_warn_once_and_send_nothing`（`ghost_switch_tests.rs`） | `["Nobody", "random", "lastinstalled"]` の 3 つを `NotFound`・`ghost_switch_unknown` 1 件・送出 0 で固定 | 要件 6.5 のとおり `Nobody` だけに縮める（`random` は候補 B が居るので解ける・`lastinstalled` は記録なしで `warn!` だが理由が別）。 |
| **`each_canonical_bracketless_tag_yields_exactly_one_raw`（`parse_bare_tag_tests.rs`）** | `CANONICAL_BRACKETLESS_SPELLINGS`（12 綴り）に `"_+"` が入り、`parse(r"\_+") == [raw(r"\_+")]` を固定 | **要件に書かれていない接触**。`\_+` を `GenericCommand` にするとこの断言が赤になる。`"_+"` を一覧から外す（12→11・説明文の「全 12 綴り」も直す）か、`\_+` だけ別の期待にする。`parse_bare_tag_tests.rs` は完了 `sakura-bare-tag-lexer` の兄弟テスト＝触る理由を設計に書く。 |
| `lexer_bare_tag_tests.rs`・`lexer_word_boundary_tests.rs` | `Token::Bare("_+")`／`Bare("+")` を固定 | 字句は変えないので**不変**。 |
| `unknown_bare_tag_absorbed_as_raw`（`decode_tests.rs`） | 例は `\i` | 不変。 |

### 2.7 台帳・生成物・整合検査（実測）

- `doc/ukadoc-coverage/ledger/sakura-script.toml`: `[entry."ukadoc:list_sakura_script:_5c_2b:1"]`（`\+`）と `[entry."…:_5c__2b:1"]`（`\_+`）は `status = "absent"`・`owner = ""`・`priority = "B2"`・`links = []`。備考は「decode_bare の既定の腕から … Raw になり compile の catch-all が捨てる」と今日の落ち方を書く。`\_+` の備考には「[所有先未定: … areka-P0-ukadoc-coverage-roadmap の無所有一覧で裁定すると登記している]」の行がある。`\![change,ghost,…]` の行は `implemented`・owner `areka-P0-ghost-shell-balloon-switch`・`links` に `same-feature` で `\+`・`\_+` を指し、備考「名前の解決: … 正典の特別な名前 random／sequential／lastinstalled の解決は areka-P0-ghost-change-name-resolution の持ち場で、本仕様では該当なしとして warn のログ（ghost_switch_unknown）を残し、切り替えない。」を持つ。同じ文が `ledger/shiori.toml` の `OnGhostChanging`／`OnGhostChanged` の備考にも在る（2 か所）→ §7 論点 6。
- **証拠行の規則**: `check_evidence`（`crates/ukadoc-survey/src/check/content.rs`）は `status == Implemented` の項目にソースの `// ukadoc: <URL>` 証拠が 0 件なら `ImplementedWithoutEvidence` を出す。`\+`・`\_+` を実装済みにするなら `decode_bare` の新しい 2 腕の直前に `// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_2b:1`／`#_5c__2b:1` を置く（既存の腕と同型）。`\![change,ghost,…]` の証拠は `consumer_ledger.rs` の `/// ukadoc: …change_2cghost…` に在る（本仕様は触らない）。
- **spec の整合検査**（`crates/ukadoc-survey/tests/consistency/spec_checks.rs`）: 腕 a `[briefs].count` ＝ `[[spec]]` の行数、腕 c `[[spec]].owner_count` ＝ 台帳でその名前を宛先に持つ項目数、腕 f 台帳の非空の宛先はすべて `[[spec]]` か `[[owner_completed]]` に在ること。`doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` は今 **35 行**（`[briefs].count = 35`・`snapshot_on = "2026-09-27"`）で、本仕様の行は**無い**。→ 台帳 2 行の owner を本仕様にする瞬間、`[[spec]]`（`name`・`stage = "B"`・`bundle = "切替"`・`owner_count = 2`・`wave = "B4-②"`）を 1 行足し、`count` を 36 にしないと検査が赤になる。要件 7 にはこのファイルが**書かれていない**（brief の「別枠」と `ghost-install` の brief は数の合流の危険として触れている）。完了 `ghost-shell-balloon-switch` は同じ理由で「2026-09-27 の追加」の段落と `[[spec]]` を足した（手本）。
- 生成物 `doc/ukadoc-coverage/report/*.md` は `cargo run -p ukadoc-survey -- report`／`report-summary`（`doc/ukadoc-coverage/README.md`）で作り直す。手書きの `briefing-sakura-script.md` は `\+`・`\_+` を「未対応（書いてあるのに何も起きない）」と書き、`\![change,ghost,…]` の行は完了 spec が手で「実装済み（2026-09-27 に…）」へ直した前例がある → §7 論点 6。
- `doc/COMPAT_ARCHITECTURE.md` §8: 「角括弧なし `\_` タグ（2 文字形 `\_X`・3 文字形 `\__X`）の字句境界と意味」の行が `\_+` を「所有先未定＝`areka-P0-ukadoc-coverage-roadmap` の無所有一覧で裁定」と書いている。要件 7.4 の裁定 1〜7 の行を足すのに加え、**この行の `\_+` の所有先を本仕様へ直す**必要がある。

### 2.8 並走する spec との接触（実物で再測定）

- `session-mark-residue`（B4-①）の brief「Out of Boundary」: `ghost_switch.rs`・`change_cue.rs`・メニュー・`areka-parsers/src/sakura/` に触らない。本仕様の触るファイルとの重なり **0**。別枠で `doc/COMPAT_ARCHITECTURE.md` §8 の末尾追記が重なる（行の追加どうし＝合流は容易）。
- `shell-balloon-switch`（B6・後）の brief「3 つの約束」: `ConfigInputs`・`CurrentGhost`・`GhostDecision` に欄を足さない／`SwitchRequest`・`GhostSpec`・`request_ghost_switch` の形を変えず呼ばない／`areka-ghost/src/lib.rs` と `areka-parsers/src/sakura/` に触らない。本仕様が `GhostSpec` に腕を足す案（§4 の B）を採ると、この約束の「形を変えない」と `ghost-install` の brief（`GhostSpec::Folder(既定)` を呼ぶ案 (b)）に影響する。
- `ghost-install`（B5・後）の brief: 「受け皿の所有は先に着地する方。同 spec（本仕様）が B4 で先なら同 spec のまま（本仕様は書くだけ）」。本仕様が先なので、書く口の名前と引数を完了時に申し送る（要件の Adjacent expectations どおり）。
- ファイルの大きさ: `ghost_switch.rs` 699・`ghost_switch_tests.rs` 545・`decode.rs` 391・`decode_tests.rs` 624・`parse_bare_tag_tests.rs`（`_+` の 1 語）。1 ファイル 1,000 行の目安（`.kiro/steering/structure.md`）に対し、`ghost_switch.rs` に純関数（30〜60 行）と `Resource`（10 行）を足しても余裕がある。新しい判断のテストは兄弟の新ファイル（例 `ghost_switch_resolve_tests.rs`＋`#[cfg(test)] #[path = …] mod resolve_tests;`）へ置くと `ghost_switch_tests.rs` を 1 か所（3 つ組の縮め）しか触らない。

### 2.9 実機サインオフの前例

完了 `ghost-shell-balloon-switch` の `signoff.md`: 台本からの切替は「R_POST_and_KOMAINU の丸ごとの複製 `ghost\rpost_auto\` の `dic02_Event.txt` の `＊OnBoot` を `：\![change,ghost,fail-one]` の 1 行に差し替えた検体」で起こした（UI を使わず・`AREKA_PROFILE_DIR` を空のフォルダに・`RUST_LOG=info,kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS` で有界）。**emo2 の辞書はリポジトリの外**（`alpha-release-signoff` の行「開発者の手＝emo2 の辞書に `halt` の台詞を足す」）で、`\+` を emo2 に言わせる手段は本仕様の側に無い。→ §7 論点 1。

## 3. 要件 → 資産の対応（欠け・不明・制約）

| 要件 | 既にあるもの | 欠け（Missing）／不明（Unknown）／制約（Constraint） |
|---|---|---|
| 1 `\+`／`\_+` の別名 | 字句 `Bare("+")`／`Bare("_+")`・`decode_bare` の裸 `\f` の前例・`GenericCommand` の受け皿・`compile` の腕・`ChangeCueSink` の自己選別 | **Missing**: `decode_bare` の 2 腕（＋正典 URL の証拠行 2 本）。**Constraint**: `parse_bare_tag_tests.rs` の `_+` の固定を外す（§2.6）。1.4 の「直後の本文が残る」は字句が 1 文字（`_` 始まりは 2 文字）だけ消費するので、腕を足すだけで成り立つ（`bare_f_consumes_exactly_one_character` と同型のテストを置く）。 |
| 2 `random` | `pick_index`・注入の前例・`ctx.current.ghost.folder`・`list_ghosts` | **Missing**: 候補（目録 − 今のゴースト）から 1 体を選ぶ純関数と、本番の乱数を渡す口。**Unknown**: argv で起こしたゴーストが根の中に居るとき、`folder` が `None` なので候補から外せない（§7 論点 2）。 |
| 3 `sequential` | `list_ghosts` の並び（バイト順）・メニューの同じ並び | **Missing**: 今の位置 → 次（末尾→先頭・無ければ先頭・1 体なら自分）の純関数。自分への切替は入口が今日受理する（`ghost_switch_tests.rs` の「今のゴースト自身も受理」）ので、起こし直しの経路は無変更。 |
| 4 `lastinstalled` | `Resource` の前例・`world.insert_resource` | **Missing**: 記録の型（例 `LastInstalledGhost(String)`）と書く口（関数 1 本か `insert_resource` の 1 行）。4.2「最後の 1 件だけ」は `insert_resource` の置き換えで自然に成り立つ。4.5「記録はあるが目録に無い」は純関数がフォルダ名で目録を引くだけ。 |
| 5 1 本の経路 | `request_ghost_switch` の順序（Busy → 文脈 → 目録 → 突き合わせ） | **Constraint**: 解決は ⑶ の中に置く。5.5「同名のゴーストより特別な名前が優先」は、特別な名前の判定を `resolve_switch_target` の**前**に置けば自動で成り立つ（`GhostSpec::Name("random")` を目録と突き合わせない）。5.7「メニューには掛けない」は `GhostSpec::Folder` を解決の対象外にする（`Name` だけ見る）。 |
| 6 ログと判断の固定 | `ghost_switch_no_context` が `reason` 欄で理由を分ける前例・`log_capture_kit::capture`・`fixture_root`（実 fs に ghost/A・ghost/B）・`world_with_slot`（今のゴースト A） | **Missing**: 解けないときの `warn!`（1 事象 1 件）と解けたときの `info!`／`debug!`。既存の `ghost_switch_unknown` に `reason` を足すか、新しい event 名にするかは設計。**Constraint**: 入口のテストは `list_ghosts` が実 fs を読むので、目録は一時フォルダで組む（`fixture_root` の形）。純関数のテストは `entry()` の偽の目録で足りる。 |
| 7 台帳と文書 | 台帳 3 行・`report` の生成器・§8・roadmap.md | **Missing（要件に無い）**: `roadmap-draft.md` の `[[spec]]` 1 行と `[briefs].count` 35→36・§8 の「角括弧なし `\_` タグ」の行の `\_+` の所有先・`shiori.toml` の 2 か所の備考の同じ文（§7 論点 6）。 |
| 8 実機 | `signoff.md` の手順（検体の複製・`：\![change,…]` の差し替え・有界の走行） | **Unknown**: `\+` を出す検体（emo2 は無理）と切替の向き（§7 論点 1）。 |
| 9 暫定裁定 | — | 裁定 1〜7 は §8 の行で足りる。裁定 2（今のゴーストが目録に無いとき先頭）は論点 2 の答えしだいで補足が要る。 |

## 4. 実装案

### A: 既存ファイルの拡張（`ghost_switch.rs` に純関数＋`Resource`、`decode_bare` に 2 腕）— **推奨**

- `ghost_switch.rs`:
  - `#[derive(Resource)] pub(crate) struct LastInstalledGhost(pub String);` と書く口 `pub(crate) fn record_last_installed(world: &mut World, folder: String)`（中身は `insert_resource` 1 行＋`info!`）。`ghost-install` へ申し送る名前はこの 2 つ。
  - 純関数 `resolve_special_name(entries: &[GhostEntry], name: &str, current: Option<&str>, last_installed: Option<&str>, pick: impl FnOnce(usize) -> usize) -> Option<Resolution>`（`Resolution` は「解けた: フォルダ名＋記録用の情報」「解けない: 理由」「特別な名前ではない」の 3 通り）。`random`＝`entries` から `current` を除いた候補 n≥1 で `pick(n)`、`sequential`＝`current` の位置 +1（mod len・見つからなければ 0・空なら解けない）、`lastinstalled`＝記録をフォルダ名で目録に引く。
  - `request_ghost_switch` の中身を `request_ghost_switch_with(world, req, pick)` に分け、`pub(crate) fn request_ghost_switch(world, req)` は `pick_index` を渡す薄い皮にする（`boot_into` → `resolve_balloon_for_ghost(…, pick_index)` と同型）。解けたら `GhostSpec::Folder(folder)` に読み替えて今日の `resolve_switch_target` へ渡す。
- `decode.rs`: `decode_bare` に `"+" => GenericCommand { name: "change", raw_args: ["ghost","random"] }`・`"_+" => … ["ghost","sequential"]` の 2 腕（各腕に `// ukadoc:` 証拠行）。
- 長所: 触るクレートが 2 つ（`areka`・`areka-parsers`）、新しいファイルは兄弟テストだけ、並走 spec との重なり 0、`GhostSpec` の形が不変（`ghost-install`・`shell-balloon-switch` への申し送りが要らない）。
- 短所: `ghost_switch.rs` が 760〜800 行に増える（目安 1,000 の内側）。`decode.rs` の裸タグの腕に「値を持つ GenericCommand を作る」初めての例が入る（`\f` は引数 0 個）。

### B: `GhostSpec` に腕を足す（`Random`／`Sequential`／`LastInstalled`）

- `drain_change_requests` で `raw.name` を見て `GhostSpec` の腕に振り分け、`resolve_switch_target` の引数に `current`・`last_installed`・`pick` を足す。
- 長所: 型で「特別な名前」が見える。メニュー（`Folder`）に解決が掛からないことが型で保証される。
- 短所: `GhostSpec` の形の変更＝完了 `ghost-shell-balloon-switch` の Revalidation Trigger に当たり、`ghost-install`・`shell-balloon-switch` の brief への申し送りが要る。`resolve_switch_target` の引数が 2 → 5 に増え、既存テスト 3 本の呼び出しも書き換える。要件 5.5（同名のゴーストより優先）は `drain` の振り分けで満たすので、解決の場所が「入口の中」（要件 5.1）から半歩外れる。

### C: 純関数を `areka-ghost` の新ファイルへ（brief の当初案）

- `crates/areka-ghost/src/change_name.rs` に純関数を置き、`ghost_switch.rs` はそれを呼ぶ。
- 長所: `catalog.rs` の隣で目録の型に近い。`ghost_switch.rs` が増えない。
- 短所: `crates/areka-ghost/src/lib.rs` に `pub mod` 1 行＝`shell-balloon-switch` が触らないと約束したファイルに本仕様が触る。`lastinstalled` の記録は `World` の話なので `areka` 側に残り、置き場が 2 クレートに割れる。棚卸⑱ が取り下げた案（brief 項目 8）。

## 5. 規模とリスク

- **規模: S**（4〜6 タスク・brief の見立てどおり）。内訳: ① `decode_bare` 2 腕＋兄弟テスト＋`parse_bare_tag_tests` の 1 語 ② 純関数と決定論テスト（新しい兄弟ファイル） ③ 入口への組み込み・`Resource`・乱数の口・3 つ組の縮め ④ 台帳 3 行＋`roadmap-draft.md`＋`report` 生成＋§8＋roadmap.md ⑤ 実機 1 周＋`signoff.md`。
- **リスク: 低**。理由: 既存の前例に全部乗る（注入・`Resource`・裸タグの腕）、経路の後半（降ろす・起こす・戻す）は無変更、並走 spec との共有ソース 0。残るリスクは「文書側の整合検査」（`roadmap-draft.md` を忘れると `cargo test -p ukadoc-survey` が赤・§2.7）と「argv の `folder: None`」の読み（§7 論点 2）。

## 6. 設計フェーズへの推奨

- 案 **A** を採る。`GhostSpec`・`SwitchRequest`・`resolve_switch_target` の形は変えない（並走・後続との約束を守る）。
- 乱数は関数引数で注入し、本番は `pick_index`。`World` に乱数の持ち物は置かない。
- 解けないときの記録は既存の `ghost_switch_unknown` に `reason` 欄（`random_no_candidates`／`sequential_empty`／`lastinstalled_none`／`lastinstalled_missing`／`name`）を足す形が、`ghost_switch_no_context` の `reason` と同型で読みやすい（新しい event 名にする案も可・設計で決める）。解けたときは `info!(event = "ghost_switch_resolved", name, to, index)` 1 件。
- テストの置き場: 純関数は新しい兄弟ファイル（偽の目録・固定の `pick`）、入口の統合は `ghost_switch_tests.rs` の `fixture_root`（A・B の 2 体）を使い `world_with_slot` の今のゴースト A で `random` → B・`sequential` → B・B が今のゴーストのとき `sequential` → A（末尾→先頭）・`lastinstalled` は `insert_resource` で記録を入れてから。
- 文書: 台帳 2 行を実装済み（owner＝本仕様・`links` に `same-feature` で `\![change,ghost,…]` を指し返す）、`\![change,ghost,…]` の備考 1 文、`roadmap-draft.md` の `[[spec]]`＋`count`、§8 の 1〜7 の行と「角括弧なし `\_` タグ」の行の `\_+`、生成物は生成器で。

## 7. 要件ディスカッションに出す論点（答えで作業が変わるものだけ）

1. **実機サインオフの向き（要件 8.1）**。要件は「emo2 を降ろして他のゴーストを起こす」と書くが、emo2 の辞書はリポジトリの外で `\+` を言わせられない。完了 spec の前例どおり里々の検体の複製（例 `ghost\rpost_plus\`・`＊OnBoot` を `：\+` に）を起こすと、目録が {emo2, rpost_plus} のとき `random` の候補は emo2 だけ＝**R_POST → emo2** の向きになる。要件 8.1 の向きを「`\+` を言う検体 → 目録の他の 1 体」に読み替えるか（推奨・作業は検体 1 つの差し替えだけ）、emo2 側の辞書を開発者の手で足すか。
2. **argv で起こしたゴーストが根の中に居るときの `random`／`sequential`（要件 2.4・3.3・裁定 2）**。`resolve_ghost` の段 1 は argv なら常に `folder: None` を返すので、`areka.exe <根>\ghost\A` で起こしても「今のゴーストは目録に無い」扱いになり、`random` が A 自身を選びうる・`sequential` は A の隣でなく先頭へ行く。(a) 要件どおり `folder` だけで判断し、この振る舞いを §8 に「argv 起動は例外」と 1 行書く（推奨・開発者向けの起動法なので実害は小さい）。(b) `dir` の正規化（`std::path::absolute`）で目録の `dir` と突き合わせて位置を求める（純関数の入力にパスが増える・`boot_resolve.rs` は触らずに済む）。
3. **`sequential` で目録が 1 体だけのとき自分を起こし直す（要件 3.4・裁定 3）**。`random` は無視（要件 2.3）なのに `sequential` は起こし直す非対称は正典の読みどおりだが、利用者から見ると「`\_+` を書いたら自分が再起動して挨拶し直す」。無視に揃える案（要件 3.4 を 3.5 と同じ「無視＋`warn!`」に）とどちらにするか。作業差は小さい（純関数の 1 分岐と §8 の 1 行）。
4. **`lastinstalled` の記録の型と書く口の名前**（要件 4.2・Adjacent expectations）。`Resource`（`LastInstalledGhost(String)`＋`record_last_installed(world, folder)`）を推す。`ghost-install` の brief は「NonSend」と書いているが `String` は `Send` なので `Resource` でよい。フォルダ名だけを受けるか、`GhostEntry`（`dir` 付き）を受けるか＝フォルダ名で十分（目録に無ければ無視するのは要件 4.5）。
5. **解けないときの event 名**（要件 6.1）。既存 `ghost_switch_unknown` に `reason` 欄を足す（既存テストの `assert_one_event(…, "ghost_switch_unknown", WARN)` がそのまま使える）か、`ghost_switch_unresolved` を新設するか。ログを grep する側（実機サインオフ・`alpha-release-signoff`）の都合で決める。
6. **文書の接触範囲（要件 7）**。要件に無い 3 か所——`roadmap-draft.md`（`[[spec]]`＋`count`）、§8「角括弧なし `\_` タグ」の行の `\_+` の所有先、`ledger/shiori.toml` の `OnGhostChanging`／`OnGhostChanged` の備考にある同じ文「…の持ち場で、本仕様では該当なしとして…」（`shell-balloon-switch` は `shiori.toml` の `OnShellChang*`／`OnBalloonChange` の行を触るが、この 2 行は触らない）——と、手書きの `briefing-sakura-script.md` の `\+`・`\_+` の「未対応」の行を本仕様で直すかどうか。推奨: `roadmap-draft.md` と §8 の行は必須（検査が赤になる・裁定の記録が矛盾する）、`shiori.toml` の 2 か所の備考は 1 文の差し替えだけなので本仕様で直す、`briefing-sakura-script.md` は完了 spec の前例に倣って直す（いずれも別枠＝並走の重なりには数えない）。

## 8. 設計へ持ち越す調べもの（Research Needed）

- `pick_index` を `request_ghost_switch` の中で毎回呼ぶ形で、`RandomState::new()` がプロセス内で値を変える（std の実装はスレッドごとの鍵を呼ぶたびに進める）ことを、決定論テストではなく実機ログで 2 回以上の `\+` の結果が散ることで確かめる（要件 2.2 は「固定しない」＝分布の検定はしない）。
- `roadmap-draft.md` の `[[spec]]` を足したときの `linkage.md` の束名「切替」の実在（腕 d）と、`briefing.md` の `[[owner_completed]]` に本仕様が要らないこと（完了前は `[[spec]]` に居るだけでよい）を、`cargo test -p ukadoc-survey` で着手時に確かめる。
- 実機で `\+` を言う検体（R_POST の複製）の `OnBoot` 台本が `：\+` 1 行で里々の出力として `\+` を素通しするか（里々が `\+` を自分の記法と誤読しないか）を、走らせる前に検体の台本を areka のログ（`kanade=trace` の応答の生文字列）で確かめる。

# Brief: areka-P0-shell-companion-balloon

> 2026-10-03 `/kiro-discovery`（再入）で起票。`install-companion-canon` を要件の段で 3 本へ分けた 3 本目（開発者指示「現地点で対応していない機能に影響するのであれば、ロードマップを調整し、複数specへの分解で対応して欲しい」）。**シェルの書庫に同梱したバルーンを、そのシェルに紐づける**＝areka に今は無い「シェルごとのバルーン」を作る。ソースの指し先は 2026-10-03（main `d4f9e93d`）の実測＝着手時に引き直すこと。

## Problem

- **シェルの作者**: 追加シェルの書庫にバルーンを同梱しても、そのシェルに着替えたときに、そのバルーンにならない。シェルの絵に合わせたバルーンを配れない。
- **利用者**: 同梱のバルーンは入るが、自分でメニューから選ぶまで使われない。

## Current State

- **正典（ukadoc）**: 「同時インストール」の節（https://ssp.shillest.net/ukadoc/manual/descript_install.html ）。
  - 「なお、ゴーストやシェルに紐づくバルーンとして設定されるのは最初の1個だけである。」
  - `*.directory` の項は「実用上はtypeがghostかshellのアーカイブで用いる。」と書く。
  - 紐づいた後の振る舞い（シェルを替えたときにバルーンも替わるか、など）は書いていない。
- **areka（2026-10-03）**:
  - **入れるところ**: シェルの書庫（`type,shell`・`accept` 必須）は入る（`manifest.rs` の `parse_kind`・宛先は `plan.rs` の `<根>/ghost/<受け手>/shell/<directory>`）。
    - 同梱のバルーンも `<根>/balloon/<名>` へ入る（`collect_companions` が Ghost と Shell を扱う）。
    - シェルの `install.txt` はシェルのフォルダに残る（`plan.rs`・残さないのは supplement だけ）。ただし後から読む者は居ない。
  - **入れた後**: `install/desk.rs` はシェルについて `record_installed_shell`（`lastinstalled` 用・プロセスの中だけ）しか行わない。同梱は何も紐づけない。
  - **覚えるところ**: `PersistKey` は `LastBalloon`・`LastShell`（どちらも Ghost の範囲）だけ。`PersistScope::Shell` は在るが、sylphya の外で使う者は居ない。
  - **シェルを替えるところ**: メニューと `\![change,shell]` → `emo2_boot/frame/switch.rs` の `finish_shell`（見た目の差し替え → 重なり順 → `set_shell_dir` → `record_last_shell` → `OnShellChanged`）。バルーンには触れない（`record_last_shell` の doc が「`LastBalloon` には触れない」と書く）。
  - **起動するところ**: バルーンはシェルより先に決まる（`boot_config.rs` でバルーン → `ghost_session.rs` の `decide_boot_shell`）。今は、シェルがバルーンの決め方に効く道が無い。

## Desired Outcome

- シェルの書庫の同梱の最初の 1 個（`install-companion-reading` の探索順）が、そのシェルに紐づく。
- 紐づいたシェルへ着替えたとき（メニュー・`\![change,shell]`）と、そのシェルで起動したときに、紐づいたバルーンが使われる。
- 振る舞いの決め（下の議題）が `doc/COMPAT_ARCHITECTURE.md` §8 に記されている。
- 当たらなかったとき（紐づいたバルーンが消されている、など）の記録が残る。

## Approach

- 紐づけの持ち方は 2 案。推しは「読む」。
  - **読む**: シェルのフォルダに残った `install.txt` を、使うときに読む。ゴーストの側（`companion_balloon`）と同じ型で、新しい記憶が要らない。
  - **覚える**: 入れたときに `PersistScope::Shell` へ書く。
- 起動では、シェルを先に決めてからバルーンを決める順に並べ替える（`ghost_session.rs`・`boot_config.rs`）。
- 解決の鎖（`boot_resolve.rs`）に「シェルに紐づくバルーン」の段を足す。
- **要件の段の議題**（ukadoc が書いていないこと）:
  1. 紐づくバルーンを、利用者が選んだバルーン（前回の記憶・メニューでの選択）より上に置くか。推しは「着替えの瞬間だけ紐づくバルーンへ替え、その後に利用者が替えたものは記憶として尊重する」。
  2. 紐づくバルーンを持たないシェルへ戻したときに、バルーンを戻すか、そのままにするか。
  3. ゴーストの descript／同梱の標準バルーン（`ghost-standard-balloon`）とどちらを上に置くか。
  4. 切替は台詞の切れ目で行う（完了 `shell-balloon-switch` の決まり）ので、シェルとバルーンを同じ切れ目で一度に替えるか。

## Scope

- **In**:
  - シェルの書庫の同梱をシェルへ紐づけること。
  - 着替え（メニュー・`\![change,shell]`）と起動で使うこと。
  - 起動の順番の並べ替え。
  - 決定論のテスト。
  - `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳の `descript_install` の同梱の行。
- **Out**:
  - インストールの読み方（`install-companion-reading`）。
  - ゴーストの標準バルーン（`ghost-standard-balloon`）。
  - `recommended.balloon(.path)`（覚え書き）。
  - シェルの絵の中に台詞を書くこと（`shell-balloon`）。名前が似ているが別物。

## Boundary Candidates

- シェルのフォルダの `install.txt` の読み手（`areka-ghost` の `catalog.rs`）
- 起動時の解決と順番（`boot_resolve.rs`・`boot_config.rs`・`ghost_session.rs`）
- 着替えの配線（`emo2_boot/frame/switch.rs` の `finish_shell`・`emo2_boot/shell_balloon_switch.rs`・`emo2_boot/ghost_switch.rs`）

## Out of Boundary

- `crates/areka-nar/`（`install-companion-reading`）。
- バルーンの描画とシェルの中のバルーン（`shell-balloon`）。

## Upstream / Downstream

- **Upstream**:
  - `install-companion-reading`（探索順）。
  - `ghost-standard-balloon`（同じ鎖・同じ `catalog.rs`）。
  - `shell-balloon`（`emo2_boot/` の切替の配線を先に変える。同時に走らせると `frame/switch.rs` でぶつかる）。
  - 完了 `shell-balloon-switch`（台詞の切れ目での差し替え）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `shell-balloon-switch` の着替えにバルーンの差し替えを足す）。
- **Adjacent**: `shell-balloon`・`balloon-canon-residue`（`emo2_boot` の結線の列）・`mcp-tool-entrances`（`ghost_session.rs`）。

## Constraints

- 意味論は ukadoc から輸入する。ukadoc が沈黙するところは議題として開発者が決める。
- 決定論のテスト網羅は必達。ログの無い失敗の経路を作らない。
- 段は**優先**（バルーン関係）・**C4 の候補**（`shell-balloon` と `ghost-standard-balloon` の後）・`emo2_boot` の結線の列・規模 M（10〜14 タスク）・Fable 推奨。


## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（10〜14 タスク）。議題 4 で「シェルとバルーンを同じ切れ目で一度に替える」を選ぶと上限を少し超えうる（12〜16）。
- 前提の状態: `shell-balloon` は着地済み（PR#227）。`install-companion-reading`（C2-⑥）・`ghost-standard-balloon`（C3-⑥）は未。
- 崩れた前提／古くなった位置:
  - `shell-balloon` が `crates/areka/src/emo2_boot/frame/switch.rs` を先に変え終えた: `finish_shell` は引数 `boxes: ShellBoxAssets` を受け、最初に `boxes.hand_to(...)`（箱の束を文字の層へ）→ 配置 → 重なり → `set_shell_dir` → `record_last_shell` → `OnShellChanged` の順。`SwapFinish::Shell` に `boxes` の欄が増え、`split` が `assets.boxes` を運ぶ。`finish_balloon` は `set_balloon_label` を呼ぶようになった。`emo2_boot/shell_balloon_switch.rs` は 4 行だけ変わった。brief の「`shell-balloon` と同時に走らせると `frame/switch.rs` でぶつかる」は解けた。
  - 起動の側: `decide_boot_shell` は `ghost_session.rs` ではなく `crates/areka/src/boot_resolve.rs` に在り、`ghost_session.rs` の 2 か所から呼ばれる。バルーンは `boot_config.rs`（`resolve_balloon_for_ghost`）で先に決まる＝brief の「シェルを先に決めてからバルーン」への並べ替えは `boot_config.rs` と `ghost_session.rs` の両方に及ぶ。`ghost_session.rs`（873 行）は `mcp-tool-entrances`（PR#223）でも変わった。
  - **切替の進行中の印は高々 1 つ**（`shell_balloon_switch.rs` の `SkinSwitchInFlight`。在れば次の要求は `skin_switch_busy` の `warn!` で断る）。シェルの着替えの中から今の入口（`request_skin_switch`）でバルーンの切替を出すと断られる。「同じ切れ目で一度に替える」なら、背景の資産づくり（`switch_assets.rs` の `SwapBuilt`）と `SwapFinish` にシェルとバルーンを一緒に持つ形を足す必要がある。「シェルの後に別の切れ目で替える」なら、`finish_shell` の後で改めて要求を出す（台詞の切れ目を 2 度待つ）。
  - `PersistScope::Shell` は今も sylphya の中（`crates/areka-sylphya/src/persist/mod.rs`・`actor.rs`）だけで使われる。推しの「読む」案なら触らない。
- 触るファイル（並走の照合用）:
  - `crates/areka-ghost/src/catalog.rs`（シェルのフォルダの `install.txt` の同梱の読み手）と兄弟のテスト
  - `crates/areka/src/boot_resolve.rs`・`boot_config.rs`・`ghost_session.rs`（起動の順）
  - `crates/areka/src/emo2_boot/frame/switch.rs` 635（`finish_shell`・`split`）・`emo2_boot/shell_balloon_switch.rs` 442・`emo2_boot/switch_assets.rs`（一度に替えるなら）・`emo2_boot/ghost_switch.rs` 891（ゴーストの切替で起こすときのシェルとバルーンの順）
  - `doc/COMPAT_ARCHITECTURE.md` §8・`doc/ukadoc-coverage/ledger/assets.toml`
- 共有しうる相手: `balloon-canon-residue`（`frame/switch.rs`）・`mcp-reload`・`network-update-canon-order`（`ghost_switch.rs`・`ghost_session.rs`）・`ghost-standard-balloon`（`catalog.rs`・`boot_resolve.rs`・`boot_config.rs`＝前提として直列）。`shell-balloon-frame-align`（バグ・`actor_box.rs`・`frame/status_report.rs`・`balloon_visibility_phase.rs`・`spine.rs`）とは重ならない。
- 議題（答えで作業が変わるものだけ）: brief の 4 つ。とくに 4（同じ切れ目で一度に替えるか）は、上の「進行中の印は 1 つ」のため、選んだ側で `switch_assets.rs`・`shell_balloon_switch.rs` まで広がるかが決まる。
- 見つけた穴: 無し。

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

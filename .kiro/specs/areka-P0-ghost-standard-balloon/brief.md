# Brief: areka-P0-ghost-standard-balloon

> 2026-10-03 `/kiro-discovery`（再入）で起票。`install-companion-canon` を要件の段で 3 本へ分けた 2 本目（開発者指示「現地点で対応していない機能に影響するのであれば、ロードマップを調整し、複数specへの分解で対応して欲しい」）。**起動時にゴーストが使うバルーンを、ゴーストの作者が指定したとおりにする**。前身の書きかけの要件 5（`completed/areka-P0-install-companion-canon/requirements.md`）の続き。ソースの指し先は 2026-10-03（main `d4f9e93d`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストの作者**: 作者が指定したバルーンで出てこない。
  - ゴーストの書庫に同梱したバルーンを、番号付き（`balloon0`・`balloon1`）だけで書いた場合。
  - ゴーストの descript.txt に標準のバルーンを書いた場合。
- **利用者**: 初めて起動したゴーストが、作者の想定と違うバルーン（既定の Staysee か無作為）で話す。
  - α の検体 `claudia` が実例（`balloon0`・`balloon1` だけを書いている）。

## Current State

- **正典（ukadoc）**:
  - 「同時インストール」の節（https://ssp.shillest.net/ukadoc/manual/descript_install.html ）:
    - 「なお、ゴーストやシェルに紐づくバルーンとして設定されるのは最初の1個だけである。」
    - 「2個目以降はインストールされるだけで、そのゴーストの標準バルーンにはならない。」
  - ゴーストの descript.txt（https://ssp.shillest.net/ukadoc/manual/descript_ghost.html ）:
    - `balloon,バルーン名`: 「標準で使用するバルーン名。」
    - `default.balloon.path,パス`: 「標準で使用するバルーンの相対パス。」
- **areka の起動時の解決**（`crates/areka/src/boot_resolve.rs` の `resolve_balloon`。入力は `boot_config.rs` の `resolve_balloon_for_ghost` が集める）。順番は次のとおり:
  1. 引数
  2. 前回の記憶（Ghost の範囲の `areka.last.balloon`）
  3. 同梱
  4. 0 個なら失敗
  5. 唯一の 1 個
  6. 既定（Staysee）
  7. 無作為
  - ゴーストの切替（`emo2_boot/ghost_switch.rs`）も同じ鎖を使う。
- **同梱の段の中身**: `crates/areka-ghost/src/catalog.rs` の `companion_balloon` が `<ゴースト>/install.txt` の**無印の `balloon.directory` だけ**を読む。
  - 番号付きの鍵は読まない。テスト `catalog_tests.rs` は `balloon0` を無視することを確かめている。
  - 区切りの置き換えもしない。そのため `a/b` と書いた同梱は、インストールでは `a_b` になる（`install-companion-reading` の後）のに、起動は `a/b` を探して見つからない。
- **descript の鍵**: `balloon`・`default.balloon.path` はどこからも読まれていない。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` では `status = "absent"`・担当なし。注記が指す `boot_config::default_balloon_root` は、もう存在しない。
- 名前の照合は大文字と小文字を区別する完全一致（`boot_resolve.rs` の `find`）。

## Desired Outcome

- 同梱の段が「最初の 1 個」を使う。無印があれば無印、無ければ `install-companion-reading` の探索順で最初に見つかった番号（`balloon0` …）。名前はインストールと同じ `_` への置き換えで読む。
- ゴーストの descript.txt の `balloon`（バルーン名）と `default.balloon.path`（相対パス）が、起動時の解決の段として効く。
- 段の並びと、ukadoc が沈黙するところの決めが `doc/COMPAT_ARCHITECTURE.md` §8 に記されている。
- 網羅台帳の `balloon`・`default.balloon.path` の 2 行と、`descript_install` の同梱の行が実装に合っている。
- 各段が当たらなかったときの記録（今の `companion_balloon_not_found` と同じ形）が残る。

## Approach

- `catalog.rs` に読み手を足し（番号付きの同梱の探索・descript の 2 鍵）、`boot_resolve.rs` の `BalloonInputs` に段を足す。`boot_config.rs` は集めて渡すだけ。
- `_` の置き換えは `install-companion-reading` が置く関数をそのまま使う（同じ規則を 2 か所に書かない）。
- **要件の段の議題**（ukadoc が書いていないこと）:
  1. 前回の記憶・descript の `balloon`／`default.balloon.path`・同梱の最初の 1 個を、どの順で使うか。推しは「記憶 → descript → 同梱 → 唯一 → 既定 → 無作為」。利用者が選んだものを最優先にし、作者が文字で書いた指定を、書庫の詰め方から導いた指定より上に置く。
  2. `balloon` の「バルーン名」をバルーンの descript の `name` で引くか、フォルダ名でも引くか。
  3. `default.balloon.path` の「相対パス」の起点（ゴーストのフォルダか、根の `balloon/` か）。

## Scope

- **In**:
  - 同梱の最初の 1 個。
  - descript の `balloon`・`default.balloon.path` を起動時の段にすること。
  - ゴーストの切替でも同じ鎖になること（鎖は共有なので配線の変更は無い見込み）。
  - 決定論のテスト（段ごとの当たり・外れ・並び）。
  - 網羅台帳と `doc/COMPAT_ARCHITECTURE.md` §8。
- **Out**:
  - **`recommended.balloon`／`recommended.balloon.path`**。これ以外へ切り替えると「強い内容の警告」を出す項目で、メッセージボックスを出さない方針（失敗は既定ゴーストの台詞で伝える）との折り合いが要るので、roadmap の「覚え書き」に置く。
  - シェルに紐づくバルーン（`shell-companion-balloon`）。
  - インストールの読み方（`install-companion-reading`）。
  - `char*.balloon.*` などバルーンの見た目の鍵。

## Boundary Candidates

- ゴーストのフォルダの読み手（`areka-ghost` の `catalog.rs`）
- 起動時のバルーンの解決（`areka` の `boot_resolve.rs`・`boot_config.rs`）

## Out of Boundary

- `crates/areka-nar/`（`install-companion-reading`）。
- シェルの切替の配線（`emo2_boot/frame/switch.rs` の `finish_shell`・`shell_balloon_switch.rs`）と `ghost_session.rs` の起動の順番（`shell-companion-balloon`）。

## Upstream / Downstream

- **Upstream**: `install-companion-reading`（探索順と `_` の置き換えの関数）。完了 `baseware-root-layout`（解決の鎖）・`ghost-shell-balloon-switch`（切替で鎖を再利用）。
- **Downstream**: `shell-companion-balloon`（同じ鎖へ「シェルに紐づくバルーン」の段を足す）。

## Existing Spec Touchpoints

- **Extends**: なし。完了 `baseware-root-layout` が「`balloon.directory`・`recommended.balloon` を読まない・α 後」とした先送りを、`recommended.*` 以外について引き取る。
- **Adjacent**: `shell-companion-balloon`（同じ `boot_resolve.rs`・`catalog.rs`＝直列）。

## Constraints

- 意味論は ukadoc から輸入する。
- 利用者が選んだもの（記憶・引数）を作者の指定で上書きしない（完了 `baseware-root-layout` の順を崩さない）。
- 決定論のテスト網羅は必達。ログの無い失敗の経路を作らない。
- 段は**優先**（バルーン関係）・ウェーブ **C3**（予定・`install-companion-reading` の後）・規模 S〜M（6〜10 タスク）・Fable 推奨。


## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S〜M（6〜10 タスク）。変わらず。
- 前提の状態: 未（`install-companion-reading` が C2-⑥ で未着手）。
- 崩れた前提／古くなった位置:
  - 起票（main `d4f9e93d`）の後に `crates/areka-ghost/src/catalog.rs`・`catalog_tests.rs`・`crates/areka/src/boot_resolve.rs`・`boot_config.rs`・`emo2_boot/ghost_switch.rs` へ入った変更は 0。brief の記述（`companion_balloon` は無印の `balloon.directory` だけ・`BalloonInputs` の段・`find` は完全一致・切替は `ghost_switch.rs` の `resolve_balloon_for_ghost` 呼び出しで同じ鎖）はそのまま。
  - 細部 1: `boot_config.rs` の `resolve_balloon_for_ghost` は `list_balloons` の結果を `identity.folder` だけへ落として `BalloonInputs.listed` に渡している。`catalog.rs` の `Identity` は `name`（バルーンの descript の `name`）も持つので、議題 2 で「名前で引く」を選ぶと `listed` の形（フォルダ名の列 → フォルダ名と名前の組）が変わり、`boot_resolve.rs` の `resolve_balloon` とそのテストへ波及する。
  - 細部 2: 網羅台帳の古い注記「`boot_config::default_balloon_root`（もう無い関数）」は、本 spec の 2 行（`descript_ghost` の `balloon`・`default.balloon.path`）のほか、範囲外の `recommended.balloon`・`recommended.balloon.path` の 2 行にも同じ文で残っている（`doc/ukadoc-coverage/ledger/assets.toml`）。後者を直す担当は居ない。
  - `_` への置き換えの関数は `install-companion-reading` が `areka-parsers` に置くのが自然（`areka-ghost` は `areka-nar` に依存しない・両方とも `areka-parsers` には依存する）＝本 spec は `catalog.rs` からそれを呼ぶだけ。
- 触るファイル（並走の照合用）:
  - `crates/areka-ghost/src/catalog.rs` 377・`catalog_tests.rs` 642（伸ばすなら兄弟の新しいテストへ）
  - `crates/areka/src/boot_resolve.rs` 544・`boot_resolve_tests.rs`・`boot_config.rs` 492
  - `doc/ukadoc-coverage/ledger/assets.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
  - `emo2_boot/ghost_switch.rs` は配線が共有なので無改変の見込み
- 共有の注意: 同じ C3 の `mcp-get-property` が `crates/areka-ghost/src/runtime.rs` を触る（別のファイル）。`shell-companion-balloon` は同じ `catalog.rs`・`boot_resolve.rs`・`boot_config.rs` を触る＝直列（roadmap どおり）。
- 議題（答えで作業が変わるものだけ）: brief の 3 つのまま（段の並び／`balloon` をバルーンの名前で引くかフォルダ名でも引くか＝上の細部 1 で作業量が変わる／`default.balloon.path` の起点）。加えて、範囲外の `recommended.*` の 2 行の古い注記をついでに直してよいか（直すなら 1 タスク足さずに台帳の作業に含められる）。
- 見つけた穴: 無し（brief の不一致そのものが本 spec の仕事）。

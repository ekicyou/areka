# 設計の検証: mcp-scope-rect-property

> 2026-10-23・本ブランチ（main `15d8b0f3` の上・設計のコミット `dd40e000`）の実物で確かめた。対話なしで行った（質問はしていない）。
> 読んだもの: `spec.json`（ja）・`requirements.md`（末尾の決めの表を含む）・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C11 の行）・下に挙げたコード。
> 要件ディスカッションの決め（矩形はプル・表はプッシュ・SSP に合わせた無いときの答え・窓の外枠・触らない約束の扱い・sylphya のコメント）は開き直さない。設計がそれを正しく実装しているかだけを見た。

## まとめ

設計は決めの 6 点をそのまま形にしている。読む道（`mcp/get_property.rs`・`shiori_host.rs`）を 1 行も変えず、sylphya に「形で引く読み手の登録」を足し、areka 側の読み手が OS の窓の矩形を鍵を持たずに 1 回で読む形は、要件 5・7 と完了 spec `areka-P0-sylphya` の要件 2.7・8.3 に合っている。下の 2 点（台帳を直すと赤になる報告の作り直しが抜けている・実窓のテストが DPI を見分けられないおそれ）は、タスクに書き足せば済む大きさで、形を変える必要は無い。**GO**。

## コードで確かめたこと

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| `rewire_runtime_consumers` を初回の起動・切替・`reload` の 3 つが共に通り、実行系を置き場へ付ける前に呼ぶ | `ghost_session.rs` の `boot_wired`（`outcome.ghost` が在るときだけ呼ぶ）・`ghost_session_runtime.rs` の `rebuild_runtime_in_place`（呼んだ後に `attach_runtime`） | 合っている |
| 切替では窓を作り直してから起こす（`wire` が新しい `GhostWindows` を見る） | `emo2_boot/ghost_switch.rs` の `reopen_ghost_windows` → `boot_ghost_strict` の順 | 合っている |
| `register_systems` はプロセスに 1 回・先例が `FrameFinalize` に並ぶ | `ghost_session.rs` の `register_systems`（`register_ghost_windows_click_through` を `FrameFinalize` へ） | 合っている |
| `GhostWindows` の公開の読み取り `scopes`・`char_window`・`balloon_window` | `placement/spawn.rs` | 合っている |
| 窓を閉じるときは `GhostWindows` ごと外し、窓の entity を消す | `app_exit::close_windows_for_restart`（`despawn_app_windows` の後に `remove_resource::<GhostWindows>`） | 合っている。資源が外れても `RemovedComponents<WindowHandle>` が立つので系は起きる |
| ドラッグでは `GhostWindows` の変更の印が立たない | `GhostWindows` を書き換えるのは `window_move.rs` の `track_default_char_pos`（システム由来の再アンカーだけ）・`drain_resnap.rs`（並べ直しの確定）・`reset_pos.rs`（メニュー）・`spawn.rs` の外しのフック。ドラッグの道は通らない | 合っている（要件 5.4） |
| `WindowHandle` は窓ができた時に 1 回だけ付き、entity が消えると外れて `WM_CLOSE` を投函する | wintf `window_factory.rs`（付ける所は 1 か所）・`window_handle.rs` の `on_window_handle_remove` | 合っている。差し替えで付け直す所は無い |
| 本番の `SylphyaReader::new` は 1 か所で、ほかは複製 | `areka-sylphya/src/actor.rs` の `spawn_sylphya` だけ | 合っている＝`PullRegistry` を `Arc` で持てば MCP とゴーストの道は同じ表を見る（要件 6.6） |
| `scope(00)` は `ByIndex(0)`、`scope()`・`scope(x)`・`scope(-1)`・`scope(+1)` は `ByName` | `key.rs` の `parse_seg`（中身が全部数字なら `ByIndex`・空は `ByName("")`） | 合っている。設計の選択子の読み分けで要件 1.9・3.1・3.6 が立つ |
| `currentghost.*` はゴーストが書けない（写しに同じ名前が載らない） | `actor.rs` の `classify_set`（根 `currentghost` は `DOTTED_ROOTS` に在る＝`NotSettable`） | 合っている（写しが先でも読み手の答えが隠れない） |
| MCP の `handle` は UI スレッドで `&mut World` を受け、実行系の読み口を引く | `mcp/get_property.rs` の `handle` | 合っている |
| `Win32_UI_HiDpi`・`Win32_UI_WindowsAndMessaging` はワークスペースで有効・プロセスはモニタごとの v2 | ルートの `Cargo.toml` の `[workspace.dependencies.windows]`・`wintf/src/runtime/mod.rs` | 合っている。依存も機能の指定も足さない |
| 偽の `WindowHandle` を付けても壊れない | `on_window_handle_add` は `GetDpiForWindow` が 0 なら既定に倒れる・`fake_handle` の先例が 7 か所 | 合っている |
| ウェーブ C11 の当たり | ほかの 7 本の要件・調べ（設計はまだ無い）に `ghost_session.rs`・`ghost_session_runtime.rs`・`GhostWindows`・`reader.rs` の名は出てこない | 重なりは見つからない |

決めごとの実装の見立て:

- **どのスレッドから読んでも安全か**: 表は `Arc<RwLock<…>>` で、書くのは UI スレッドの `wire` と系だけ。読み手は窓の印（HWND の生の値）を写すだけで鍵を外し、鍵を持たずに `GetWindowRect` を呼ぶ。登録の表も同じ扱いで、読み手を呼ぶ時は鍵を持たない（再入しても止まらない）。HWND を値（`isize`）で持つのでスレッドをまたいで送れる。sylphya の写しと同じ「鍵は写すだけ」の型で、要件 5.1・5.2 に合う。
- **DPI のスレッドごとの扱い**: 呼ぶ間だけスレッドを v2 にして戻す形は正しい（`GetWindowRect` は呼んだスレッドの扱いで値が変わる）。本番はプロセスが v2 なので普段は何も変わらない。
- **reload・切替の登録の順**: `wire` は実行系を置き場へ付ける前に同期で走り、毎回新しい置き場を作る。前の実行系の読み手は前の置き場ごと捨てられる。切替では窓を作り直した後に `wire` が走り、印がまだ無い窓は `None`＝無いときの答え、印が付いたフレームの `FrameFinalize` で差し替わる。順は正しい。
- **ドラッグ中に毎フレームの仕事をしない**: 系は印（資源の変更・`Added`・`RemovedComponents`）を見るだけで、矩形は作らず写しにも出さない。
- **ウェーブ C11 の約束**: `main.rs`・`emo2_boot/mod.rs`・`emo2_boot/frame.rs`・⑥ の持ち物・`window_move.rs` に触らない。足すのは `ghost_session.rs` に 2 行・`ghost_session_runtime.rs` に 1 行で、理由も設計に書いてある（決め 5 の形）。roadmap の ④ の行は「配線は新しい `emo2_boot` のファイル」と書いているが、新しいファイルは `ghost_session_scope_rect.rs` に置く。どの spec の持ち物にも当たらないので、決め 5 の範囲に入る。

## 大事な指摘（2 件）

### 指摘 1: 台帳の 2 行を直すと、網羅の報告の作り直しが要るのに設計に無い

- **何が問題か**: 設計の「Modified Files」は `doc/ukadoc-coverage/ledger/property.toml` の 2 行を `status = "implemented"` に改めるとだけ書き、機械で作る報告 `doc/ukadoc-coverage/report/property.md`（状態の分布の表＝今は「実装済み 2・語彙のみ 178」）の作り直しを挙げていない。
- **どう効くか**: `crates/ukadoc-survey` の一貫性のテスト（`tests/consistency/checks.rs` の `DomainReportStale`＝報告が台帳から作り直した本文と一致するか）が赤になる。完了の全体テストで初めて気づくと、手戻りになる。
- **直し方**: Modified Files とタスクに「台帳を直した後、`ukadoc-survey` の生成で `report/property.md` を作り直す（手で書き換えない）」を足す。全体の `summary.md` は統合の担当が作り直す決まりなので触らない、も 1 行添える。束が閉じるかで `linkage.md` などが動くかも生成の結果で確かめる。
- **要件**: 9.4
- **設計の箇所**: 「File Structure Plan」の「Modified Files」の `doc/ukadoc-coverage/ledger/property.toml` の項

### 指摘 2: 実窓の `#[ignore]` テストが、DPI の扱いを見分けられない作りになりうる

- **何が問題か**: 「Testing Strategy」の実窓のテストは「呼んだスレッドの DPI の扱いを v2 以外にしておいても物理の画素で返す」を確かめるとあるが、⑴ テストの実行ファイルにはマニフェストが無く、プロセスは DPI 非対応のまま走る。窓を作る時のスレッドの扱いが窓の扱いになるので、窓を作る前にスレッドを v2 にし（先例 `crates/areka/tests/window_link_real_run.rs`）、読む直前だけ非対応へ替える順を決めておかないと、測っている物が変わる。⑵ 窓が 100% の画面に在ると、物理の画素と論理の画素が同じ数になり、DPI の扱いを替えても替えなくても通る（檻にならない）。⑶ `SetThreadDpiAwarenessContext` が失敗した時は記録なしで読み続けるので、座標の系がずれても何も残らない。
- **どう効くか**: 要件 1.3（物理の画素）と 8.5 の確かめが、拡大率 100% の机では空振りのまま緑になる。本番で壊れても気づけない。
- **直し方**: テストの手順を「スレッドを v2 にして窓を作る → `GetDpiForWindow` が 96 でないことを確かめる（96 なら理由を書いて失敗させる、または拡大率の違う画面の上に置く）→ スレッドを非対応へ替えて `os_window_rect` を呼ぶ → `SetWindowPos` で決めた物理の数と一致」と書く。`SetThreadDpiAwarenessContext` の失敗は `debug!`（イベント名つき）を 1 行残す（呼ぶたびに警告は出さない＝要件 5.5 と両立）。
- **要件**: 1.3・8.5（記録は steering の「失敗の道を黙らせない」）
- **設計の箇所**: 「Testing Strategy」の「実窓」・「Components and Interfaces」の `os_window_rect`・「Error Handling」の表の最後の行

## よいところ

1. **sylphya の約束を崩さずに口を足している**: sylphya は「この形の名前はこの読み手が知っている」だけを持ち、OS も HWND も知らない（依存も足さない）。引く順を「写しの区画が先・読み手は無い時だけ」に決め、鍵は写すだけ・読み手は鍵の外で呼ぶ、で完了 spec の要件 2.7・8.3 をそのまま守る。後の spec（倍率・DPI など）は形を変えて登録するだけで、読む道に手を入れずに足せる（要件 7.4）。
2. **結線の置き場の選び方が実物に合っている**: 読み手の登録を `rewire_runtime_consumers` の 1 行にしたことで、初回の起動・切替・`reload` の 3 つが同じ道を通り、実行系が見えた瞬間から読み手が居る。表の差し替えを ECS の変更の印（先例 `register_ghost_windows_click_through` と同じ段）に乗せたので、ドラッグの毎フレームに仕事が増えない。

## 判定

**GO**

- **理由**: 決めの 6 点を正しく実装しており、スレッドの安全・DPI・切替と `reload` の順・ドラッグ中の仕事・ウェーブの約束のどれにも形の上の破れは無い。上の 2 件はタスクの書き足しで直せる。
- **次の手**: 設計の討議（`/kiro-design-discussion mcp-scope-rect-property`）で指摘 1・2 を設計へ書き足すかを決め、その後 `/kiro-spec-tasks mcp-scope-rect-property` でタスクを作る。タスクには指摘 1 の報告の作り直しと、指摘 2 の実窓のテストの手順を入れる。

## 受け入れた危うさ（指摘にしない）

- 閉じかけの窓（`WM_CLOSE` の投函から OS が窓を消すまで）と HWND の使い回し: 同じフレームの `FrameFinalize` で表から外し、切替の間は実行系が外れていて MCP に届かない。設計の「Risks」のとおり受け入れてよい。
- 表を書き換える一瞬だけ、別のスレッドの読みが書きの鍵を待ちうる: sylphya の写し（`SharedMirror`）と同じ型で、窓を持つスレッドの処理の終わりを待つものではない。

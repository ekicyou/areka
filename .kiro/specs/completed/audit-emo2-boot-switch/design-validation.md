# 設計の検証: audit-emo2-boot-switch

- 日付: 2026-11-22
- 対象: `design.md`（660 行）・`requirements.md`・`research.md`（12 節まで）・`spec.json`（phase: design-generated）
- 物差し: `.claude/skills/kiro-validate-design/rules/design-review.md`（Analysis → Critical Issues → Strengths → GO/NO-GO）
- 走らせ方: 対話なし（議題は設計の討議へ渡す）

## 設計の検証のまとめ

76 件の指摘のどれにも、直し方・縛るテスト・束（T1〜T20）が付いている。要件 1〜9 の対応表も、範囲の外で触るファイルの一覧も、1,000 行の番人の見立ても揃っていて、実装へ進める形になっている。コードで突き合わせた主な主張（1 番の `visibility_wake`・2 番の `as u32` の切り捨て・3 番の `SkinOrigin::Menu`・76 番の呼び手の見分け方・29 本の移しが今の依存で組めること）は、どれも今の `main` の字面と合う。直しを要するのは、断りの規則の一貫性と、並走の相手のファイルに頼る 1 か所の 2 点で、どちらも設計の討議で決められる大きさである。

## コードで確かめた主張

| 主張 | 確かめた所 | 結果 |
|---|---|---|
| 1 番: `visibility_wake` は抑止を見ず、満了の後は `Now`（`REARM`）を毎フレーム返す | `crates/areka-runtime/src/balloon_visibility_phase.rs:120`・`:172-185` | 合う。抑止中の超過は `balloon_visibility_wait.rs:231-240` で `deadline` を残したまま保留するので、今の形では毎フレーム `REARM` が立つ |
| 1 番: 相から `prev_suppressed` が読め、`decide` の後は今のフレームの答えに等しい | `balloon_visibility.rs:450`（私有の欄）・`:503-507`（相は子のモジュール）・`balloon_visibility_wait.rs:189-190` | 合う |
| 1 番: 解けた縁は同じフレームで `now + wait` を立て直す | `balloon_visibility_wait.rs:189-198` | 合う（`After(wait)` が同じフレームで預けられる） |
| 2 番: `marker.scope as u32` が上限越えを 0 へ切り捨てる・`window_scope` が正本 | `balloon_visibility_phase.rs:459-465`・`target_map.rs:48-57` | 合う（`1usize << 32` の赤は立つ） |
| 3 番: 入口の断りの並びと「更新中」の位置・`SkinOrigin` が `PartialEq` | `crates/areka-session/src/shell_balloon_switch.rs:112-145`・`skin_switch.rs:51-55` | 合う。`req.origin == SkinOrigin::Menu` は組める |
| 74 番: `accept_skin` が送りの前に組み立てを起こす・`reload.rs:576` が呼び手 | `shell_balloon_switch.rs:210-241`・`reload.rs:576` | 合う |
| 76 番: 入口の呼び手は 4 つで「`Manual` か `Name`」と「`Automatic` かつ `Folder`」で分かれる | `menu/ghost_frame.rs:51-56`・`ghost_switch.rs:80-86`・`update/desk.rs:192-198`・`install/overwrite.rs:109-130`。`reload.rs:374` は `accept_switch` を直に呼ぶ | 合う |
| 40 番: 本体の 29 本（14・4・10・1）が `areka-runtime` の今の依存だけで組める | 本体 `runtime_tests/balloon_visibility*.rs` の `use`・`crates/areka-runtime/Cargo.toml` | 字面では合う。外の crate は `areka_input` の 2 か所だけで、25 番で消すか `areka_runtime::shell_box_hover` へ替える。`crate::` を引く行は無い |
| 48 番: `CoGetApartmentType` は今の `windows` の機能で使える | ワークスペースの `Cargo.toml:66`（`Win32_System_Com`）・前例 `crash-report-kit/src/lib_tests.rs:23-29` | 合う（`Cargo.toml` は触らずに済む） |
| 27 番・15 番の寄せ先が在る | `areka-placement/src/test_support.rs:46-51`・`:88`・`areka-runtime/src/spine_wait.rs:428` | 合う |

## 重大な指摘（3 件）

### 🔴 重大な指摘 1: 台本の要求の扱いが 3 番（D1）と 76 番（D2）で逆になっていて、76 番には D1 と同じ時刻の調べが無い

- **問題**: D1 では、ゴーストがインストールの後に出す台本（`OnInstallComplete(Ex)` の応え）が `install_busy` の真の間に届くことを理由に、台本からの要求を断らない。D2 は、台本からの `\![change,ghost,…]`（`GhostSpec::Name`）を `update_busy` で断る。ところが更新でも同じ形が起きる。`OnUpdateComplete` は後送りの列に積まれ（`update/procedure.rs:217`）、読み直しの切り替えが終わったところで送られる（`desk.rs:274-315` の `send_tail`）。そのとき背景の `run_order` がまだほかの対象を回していれば `Stage::Running` のままなので、`is_busy`（`desk.rs:537-541`）は真になる。
- **影響**: 更新の後の応えの台本でゴーストを切り替えるゴーストは、時刻の運しだいで `warn!(ghost_switch_updating)` 1 行だけ残して止まる。D1 が避けた「自動切替を止めない」（要件 5.16）と同じ種類の黙った取りこぼしが、ゴーストの切り替えの側で起きる。あわせて、今の `skin_switch_updating`（`shell_balloon_switch.rs:132`）は台本も断るので、直した後は「シェル・バルーンはインストール中は台本を通し、更新中は台本を断る」「ゴーストは更新中に台本を断る」という、理由の書かれていない組み合わせが残る。
- **推奨**: 議題 2 に、更新の応えの台本（`OnUpdateComplete`・`OnUpdateResult(Ex)`）が `update_busy` の真の間に入口へ届くかの調べ（`worker.rs` の `request_reload` と `OrderDone` の間に、ほかの対象の処理が挟まるか）を足す。届くなら、D1 と同じく「メニューだけを断る」にそろえるか、台本も断る理由を D2 に書く。どちらにしても、3 つの入口での台本の要求の扱いを 1 つの表にまとめて「見直しが要る変化」に載せる。
- **対応する要件**: 5.16・7.6・3.4（2.1 で直す対象にした 76 番）
- **根拠の箇所**: design.md の「76 番 ゴースト切替の入口の更新中の断り」・「3 番 インストール中の断り」の「台本からの要求に当てない理由」・「設計の決め」の D1・D2・議題 1・2

### 🔴 重大な指摘 2: `SwitchVerdict::Updating` を足すと、並走の相手のファイルを直さない限りワークスペースが組めない。断られたときの道が無い

- **問題**: `install/overwrite.rs:118-130` は `SwitchVerdict` を漏れなく `match` している（`audit-areka-install-update` の持ち物）。腕を 1 つ足せば、相手が腕の名を足すまでビルドが赤になる。design.md の「触れなければ申し送り」（要件 7.8）は、この件では使えない。触らなければ全体テストが組めないからである。一方で、この腕は手続きの要求（`Automatic` かつ `Folder`）には決して返らない。入口の残りの呼び手（メニュー `ghost_frame.rs:51`・取り出し `ghost_switch.rs:80`）も答えで分かれない。だから腕を足す意味は、記録とテストで見分けやすくなることだけである。
- **影響**: T4 の着地が相手のセッションの都合に縛られる。相手が先に `switch_to` を分けて `overwrite.rs` を動かせば、行が衝突する。「着地の順」（議題 4）と合わせて、詰まりの元になりうる。
- **推奨**: T4 の前に、相手と申し合わせを済ませることを関門として tasks に書く。あわせて、申し合わせが取れないときの代わりの形を設計に 1 行足す（新しい腕を足さず、断りは `warn!(ghost_switch_updating)` で見分けて今の腕〔たとえば `Busy`〕を返す。テストは `warn!` の欄で縛る）。いちばん小さい形は後者で、範囲の外のファイルの差分が 1 か所減る。
- **対応する要件**: 7.1・7.2・7.8・9.4・9.5
- **根拠の箇所**: design.md の「境界の約束」の `audit-areka-install-update` の行・「76 番」の節・「並走の spec との継ぎ目」の `SwitchVerdict::Updating` の腕・T4

### 🔴 重大な指摘 3: 34 番の縛るテストは、直す前に赤にならない形のまま残っている

- **問題**: 34 番（目録を取り出しの 1 回につき 1 度だけ読む）のテスト `unresolved_names_in_one_drain_read_the_catalog_once` の表明は「`warn!` が 3 行」と「解けた名を後に置けば受理」で、これは直す前も通る。読む回数をどう数えるかは「実装で選ぶ（数えを足さずに済むなら足さない）」として残っている。さらに、入口の中身の関数 2 本の署名に `catalog: &mut Option<…>` を足し、「受理したら埋めた目録を捨てる」という約束を足す。この約束をテストで縛る形は書かれていない。足す先の `ghost_switch.rs` は 927 行で、番人の上限に近い。
- **影響**: 負荷の直しが戻っても（目録を毎回読む形に戻っても）、どのテストも落ちない。受理の後に古い目録を使い回すと、シェルの候補を前のゴーストのフォルダから選ぶ誤りが入りうるが、それも黙って通る。
- **推奨**: 数え方を設計で 1 つに決める（たとえば、`request_*_with` が目録を引く口を閉包で受け、テストで呼ばれた回数を数える。74 番の `accept_skin_with` と同じ注入の型）。表明は「解けない名 3 件で 1 回」「受理の後の要求では、目録の口が再び呼ばれるか、進行中で断られる」の 2 つにする。注入の口が 960 行を越えそうなら、34 番の見立て（+8 行）とあわせて、番人の欄の「守り方」に書く。
- **対応する要件**: 3.1・4.1・4.2・6.1
- **根拠の箇所**: design.md の「34 番 目録を取り出しの 1 回につき 1 度だけ読む」・「1,000 行の番人」の `ghost_switch.rs` の行・D9

## 設計の強み

1. **既にあるものへ寄せる方針が徹底している**: scope の上限は `target_map::window_scope`、待ちは `spine::join_bounded`、偽の HWND と記憶の書き手は `areka_placement::test_support`、観測は `balloon_visibility::test_support::Frame` を使う。足す継ぎ目は 74 番と 34 番の私有の 2 つだけで、新しい依存は 0。コードで確かめた寄せ先は全部在る。ponytail の「いちばん小さい完全な変更」と要件 6.6 の両方に合っている。
2. **1 番の直しが決定論で縛れる形に落ちている**: `tick_wake` がプロセス共有の 1 つであることを踏まえ、相の後の旗は見ない。純判断の `visibility_wake` と、`decide` の後の状態から赤を立てる。解けた縁を同じフレームで計り直すことも今のコードの字面と合っていて、要件 6.4（1 フレーム遅らせない）を崩さない。

## 最終の判断

- **判断: GO**（条件つき）
- **理由**: 根本から組み直す食い違いは無い。要件の全項目に受け持ちの節とタスクがあり、主な主張はコードと合う。上の 3 件はどれも議題 1・2 と T4・T16 の書き足しで片づく大きさで、設計の骨組み（入口の断りの並び・テストの戻し・公開の面の狭め）は変わらない。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-emo2-boot-switch`）で、指摘 1 を議題 2 に、指摘 2 を議題 4 の申し合わせの関門と代わりの形に、指摘 3 を D9 の書き足しに取り込み、design.md を直す。
  2. 直した後、`/kiro-spec-tasks audit-emo2-boot-switch` でタスクを作る（T4 の前の申し合わせの関門と、34 番の数え方を tasks に写す）。

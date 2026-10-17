# 煙のテストの書けない方向（タスク 6.4・要件 6.1・7.6・7.7）

- 日付: 2026-10-17
- コマンド: `cargo test -p areka --test smoke_boot_loop_exit unwritable_direction_stops_boot_with_memory_write_alert -- --nocapture`
- テスト: `crates/areka/tests/smoke_boot_loop_exit.rs` の ⑤ `unwritable_direction_stops_boot_with_memory_write_alert`
- 入力: `AREKA_ROOT`＝emo2 の検体の根・`AREKA_PROFILE_DIR`＝`target\test-roots\` の下の普通のファイル `profile-is-a-file`・`AREKA_NO_ALERT=1`
- 結果: 緑（1 passed・0.20 秒）。子は番犬の締切（60 秒）の内に非 0 で終わった（`main` が `E_FAIL` を返す＝終了コード 1。子の最後の行は `Error: Error { code: HRESULT(0x80004005), ... }`）
- 人の手: 不要（告知の画面は抑止・窓を作る前に止まるのでモニタの有無によらない）

## 記録の行（抜粋・パスは `<wt>` へ縮めた）

```
ERROR areka::write_failure: [write_failure] 記憶を書けなかった——areka を終える event="memory_write_failed" scope=App what=areka の記憶 dir=Some("<wt>\target\test-roots\areka-smoke-unwritable-profile-17144-0\profile-is-a-file") reason="フォルダでない物が在る: <wt>\target\test-roots\areka-smoke-unwritable-profile-17144-0\profile-is-a-file" moment="boot"
ERROR areka::alert: [alert] 利用者へ告げます event="alert" scene=MemoryWrite { ..., moment: Boot } title="areka を起動できません" body="記憶を書けないので、ゴーストを起こさずに起動を止めました。\n覚えられなかったもの: areka の記憶\n置き場: <wt>\...\profile-is-a-file\n理由: フォルダでない物が在る: <wt>\...\profile-is-a-file\n直し方: この置き場を書ける状態にしてください。置き場の変え方は、説明書（README.txt）の「■ 記憶の置き場」にあります。" suppressed=true
```

起動の決め（`ghost_resolved route=Only`・`balloon_resolved route=Companion`）の直後に上の 2 行が続き、そこで終わる。

## 0 の数

- `memory_write_failed`（moment="boot"）: 1 件
- 題名「areka を起動できません」かつ本文「記憶を書けないので」の `alert`: 1 件
- 本物のゴースト窓の目印「本物のゴースト窓を開きました」: **0 件**
- 印を書いた記録 `session_mark_written`: **0 件**

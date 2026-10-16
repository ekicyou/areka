//! 組んだ `areka.exe` に埋めたアイコンの資源を確かめる結合テスト（areka-P0-app-icon 要件 2.3・
//! 3.1・3.2・3.4・3.6・6.1・6.2）。
//!
//! exe はデータとして開くだけで動かさない（`LOAD_LIBRARY_AS_DATAFILE`）。見ること（設計の
//! 「Service Interface」の検査 1〜6）:
//! 1. 組 1（アプリ）と組 2（通知領域）の `RT_GROUP_ICON` が在り、目録の頭が「予約 0・種別 1・枚数 8」。
//! 2. 目録の 8 項目の寸法の集まりが `SIZES` とちょうど同じで、色の深さが 32。
//! 3. 各項目の絵の番号の `RT_ICON` が在り、大きさが目録と同じで、PNG の幅と高さが項目の寸法と同じ。
//! 4. 組の絵が `icon/<名>.ico` の同じ寸法の絵とバイトで同じ（`.ico` も 8 寸法が揃う）。
//! 5. 16 枚すべてで、exe の絵が `icon/<名>-<寸法>.png` とバイトで同じ。
//! 6. `RT_MANIFEST` の 1 番に標準の部品（comctl32）の版 6 の申告が在る。
//!
//! 欠けは止まらずに全部集め、組の番号・寸法・絵の番号を含む文で並べてから失敗する。
//! MSVC でなければアイコンは入らない（要件 3.5）ので、このファイルは MSVC のときだけ組む。
#![cfg(target_env = "msvc")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{
    FindResourceW, LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW, LoadResource, LockResource,
    SizeofResource,
};
use windows::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON, RT_MANIFEST};
use windows_core::{HSTRING, PCWSTR};

/// 組の番号（資源の番号の表）。下流は自分の定数を持つ。テストの中だけの値。
const APP_GROUP: u16 = 1;
const TRAY_GROUP: u16 = 2;
/// 8 寸法。目録の 0 は 256 と読む。
const SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];
/// 環境変数: 追加で検査する exe の絶対パス（arm64 の確かめ）。
const EXTRA_EXE_ENV: &str = "AREKA_ICON_CHECK_EXE";

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// データとして開いた exe。落ちるときに `FreeLibrary` で閉じる。
struct DataModule(HMODULE);

impl Drop for DataModule {
    fn drop(&mut self) {
        // SAFETY: LoadLibraryExW が返した有効なハンドルを 1 度だけ閉じる。
        let _ = unsafe { FreeLibrary(self.0) };
    }
}

impl DataModule {
    /// 種別 `kind` の番号 `id` の資源のバイト。無ければ `None`。
    fn resource(&self, kind: PCWSTR, id: u16) -> Option<&[u8]> {
        // SAFETY: 番号の資源名（MAKEINTRESOURCE）を渡し、得たポインタは self が生きる間だけ有効な
        // 読み取り専用の領域として、SizeofResource の大きさの分だけ借りる。
        unsafe {
            let info = FindResourceW(Some(self.0), PCWSTR(id as usize as *const u16), kind);
            if info.is_invalid() {
                return None;
            }
            let size = SizeofResource(Some(self.0), info) as usize;
            let data = LoadResource(Some(self.0), info).ok()?;
            let ptr = LockResource(data) as *const u8;
            if ptr.is_null() {
                return None;
            }
            Some(std::slice::from_raw_parts(ptr, size))
        }
    }
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// 目録の 1 バイトの寸法（0 は 256）。
fn dim(byte: u8) -> u32 {
    if byte == 0 { 256 } else { byte as u32 }
}

/// PNG の印と `IHDR` の幅・高さ。PNG でなければ `None`。
fn png_dims(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() < 24 || b[..8] != PNG_SIGNATURE || &b[12..16] != b"IHDR" {
        return None;
    }
    let be = |at: usize| u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    Some((be(16), be(20)))
}

/// 寸法の集まりが `SIZES` とちょうど同じかを見て、違いを名指す文を `defects` へ足す。
fn check_size_set(what: &str, seen: &[u32], defects: &mut Vec<String>) {
    for s in SIZES {
        match seen.iter().filter(|&&x| x == s).count() {
            0 => defects.push(format!("{what}: 寸法 {s} が欠けています")),
            1 => {}
            n => defects.push(format!("{what}: 寸法 {s} が {n} 個重なっています")),
        }
    }
    for &s in seen.iter().filter(|s| !SIZES.contains(s)) {
        defects.push(format!("{what}: 余分な寸法 {s} があります"));
    }
}

/// `.ico` のファイルを読み、寸法ごとの絵のバイトを返す（検査 4 の相手）。形の欠けは `defects` へ。
fn ico_images(path: &Path, defects: &mut Vec<String>) -> BTreeMap<u32, Vec<u8>> {
    let what = path.display().to_string();
    let mut out = BTreeMap::new();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            defects.push(format!("{what}: 読めません: {e}"));
            return out;
        }
    };
    if bytes.len() < 6 || u16_at(&bytes, 0) != 0 || u16_at(&bytes, 2) != 1 {
        defects.push(format!("{what}: アイコンのファイルの頭ではありません"));
        return out;
    }
    let count = u16_at(&bytes, 4) as usize;
    let mut seen = Vec::new();
    for i in 0..count {
        let e = 6 + i * 16;
        if bytes.len() < e + 16 {
            defects.push(format!("{what}: 目録の項目 {i} が途中で切れています"));
            break;
        }
        let size = dim(bytes[e]);
        let len = u32_at(&bytes, e + 8) as usize;
        let off = u32_at(&bytes, e + 12) as usize;
        seen.push(size);
        match bytes.get(off..off + len) {
            Some(img) => {
                out.insert(size, img.to_vec());
            }
            None => defects.push(format!(
                "{what}: 寸法 {size} の絵がファイルの外を指しています"
            )),
        }
    }
    check_size_set(&what, &seen, defects);
    out
}

/// exe を開いて全部の検査をし、欠けの文を返す（空なら合格）。
/// 開けないときは、その理由の 1 文だけを返す。
fn defects_of(exe: &Path) -> Vec<String> {
    let name = HSTRING::from(exe);
    // SAFETY: データとして写すだけでコードは動かさない。得たハンドルは DataModule が閉じる。
    let module = match unsafe { LoadLibraryExW(&name, None, LOAD_LIBRARY_AS_DATAFILE) } {
        Ok(h) => DataModule(h),
        Err(e) => return vec![format!("{} を開けません: {e}", exe.display())],
    };
    let icon_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("icon");
    let mut defects = Vec::new();

    for (group, stem) in [(APP_GROUP, "app"), (TRAY_GROUP, "tray")] {
        // 検査 1: 組と目録の頭。
        let Some(dir) = module.resource(RT_GROUP_ICON, group) else {
            defects.push(format!("組 {group}（{stem}）の RT_GROUP_ICON がありません"));
            continue;
        };
        if dir.len() < 6 {
            defects.push(format!(
                "組 {group}: 目録の頭が短すぎます（{} バイト）",
                dir.len()
            ));
            continue;
        }
        let (reserved, kind, count) = (u16_at(dir, 0), u16_at(dir, 2), u16_at(dir, 4));
        if (reserved, kind, count) != (0, 1, 8) {
            defects.push(format!(
                "組 {group}: 目録の頭が 予約 {reserved}・種別 {kind}・枚数 {count} です（0・1・8 のはず）"
            ));
        }
        let ico = ico_images(&icon_dir.join(format!("{stem}.ico")), &mut defects);

        let mut seen = Vec::new();
        for i in 0..count as usize {
            let e = 6 + i * 14;
            if dir.len() < e + 14 {
                defects.push(format!("組 {group}: 目録の項目 {i} が途中で切れています"));
                break;
            }
            let (w, h) = (dim(dir[e]), dim(dir[e + 1]));
            let bits = u16_at(dir, e + 6);
            let len = u32_at(dir, e + 8) as usize;
            let id = u16_at(dir, e + 12);
            seen.push(w);
            // 検査 2: 色の深さ。
            if bits != 32 {
                defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: 色の深さが {bits} です（32 のはず）"
                ));
            }
            // 検査 3: 絵が在り、大きさと PNG の寸法が目録どおり。
            let Some(img) = module.resource(RT_ICON, id) else {
                defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: RT_ICON がありません"
                ));
                continue;
            };
            if img.len() != len {
                defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: 大きさが {} バイトです（目録は {len}）",
                    img.len()
                ));
            }
            match png_dims(img) {
                Some(d) if d == (w, h) => {}
                Some((pw, ph)) => defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: PNG が {pw}×{ph} です（{w}×{h} のはず）"
                )),
                None => defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: PNG の印で始まっていません"
                )),
            }
            // 検査 4: `.ico` の同じ寸法の絵とバイトで同じ。
            if let Some(want) = ico.get(&w)
                && want.as_slice() != img
            {
                defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: {stem}.ico の絵と中身が違います"
                ));
            }
            // 検査 5: `<名>-<寸法>.png` とバイトで同じ。
            let png = icon_dir.join(format!("{stem}-{w}.png"));
            match std::fs::read(&png) {
                Ok(want) if want == img => {}
                Ok(_) => defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: {} と中身が違います（道具の回し忘れ？）",
                    png.display()
                )),
                Err(e) => defects.push(format!(
                    "組 {group}・寸法 {w}・絵 {id}: {} を読めません: {e}",
                    png.display()
                )),
            }
        }
        check_size_set(&format!("組 {group}"), &seen, &mut defects);
    }

    // 検査 6: マニフェストの版 6 の申告。
    match module.resource(RT_MANIFEST, 1) {
        None => defects.push("RT_MANIFEST の 1 番がありません".to_string()),
        Some(m) => {
            let text = String::from_utf8_lossy(m);
            for needle in ["Microsoft.Windows.Common-Controls", "6.0.0.0"] {
                if !text.contains(needle) {
                    defects.push(format!("RT_MANIFEST の 1 番に {needle} がありません"));
                }
            }
        }
    }
    defects
}

fn assert_no_defects(exe: &Path) {
    let defects = defects_of(exe);
    assert!(
        defects.is_empty(),
        "{} のアイコンの資源に {} 件の欠けがあります:\n{}",
        exe.display(),
        defects.len(),
        defects.join("\n")
    );
}

/// Cargo が先に組んだ `areka.exe` を検査する（要件 6.1・6.2）。
#[test]
fn built_areka_exe_has_app_and_tray_icons() {
    assert_no_defects(Path::new(env!("CARGO_BIN_EXE_areka")));
}

/// 環境変数で渡した exe（arm64 の確かめ・要件 3.6・6.4）を同じ検査に通す。無ければ飛ばしを 1 行出す。
#[test]
fn extra_exe_has_app_and_tray_icons() {
    match std::env::var_os(EXTRA_EXE_ENV) {
        Some(p) => assert_no_defects(&PathBuf::from(p)),
        None => eprintln!("{EXTRA_EXE_ENV} が無いので追加の exe の検査を飛ばしました"),
    }
}

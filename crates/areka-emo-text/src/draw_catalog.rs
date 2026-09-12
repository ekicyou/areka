//! # draw_catalog — フォント候補列の解決（COM 層・draw ファサードの子）
//!
//! [`FontCatalog`] は「記述順に並んだフォント名の候補列」から、この機械に実際に
//! 入っているフォント名を 1 つ選ぶ（要件 9.1／9.2）。選んだ結果は候補列ごとに
//! 覚えるので、同じ列を文字ごとに引き直さない（要件 9.7）。
//!
//! - 候補がフォントファイル名（`.ttf`／`.otf`／`.ttc`・大小無視）のときは読み込まず、
//!   記録を 1 度だけ残して次の候補へ進む（要件 9.3）。
//! - 候補が全滅したときは、試した候補を添えた記録を 1 度だけ残して既定のフォントへ
//!   戻す（要件 9.4／13.2）。
//! - バルーン定義の候補列（`font.name` のカンマ区切り）にも同じ規則を当てる
//!   （[`FontCatalog::pick`]・要件 9.8）。
//!
//! **層規律**: COM 層——UI スレッド専有。失敗は log-first（`error!`／`warn!`）で扱い
//! panic しない。
//!
//! ## 記録の重複判定の鍵
//!
//! 記録済みかどうかは [`WarnKey`]——**候補列そのものを保った**鍵——で判定する。
//! 台本作者の書いた文字列を区切り文字で 1 本の綴りへ潰すと、別々の失敗が同じ鍵に
//! なって片方が無記録で消える（要件 13.5 違反）。引用符はカンマを守るので
//! `\f[name,a,b]` と `\f[name,"a,b"]` は別の候補列として到達する。

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};

use tracing::warn;
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory, IDWriteFactory2, IDWriteFontCollection,
};
use windows::core::{BOOL, HSTRING, Interface};

use super::{DEFAULT_FONT_NAME, ResolvedFont, device_err};
use crate::TextLayerError;

/// フォントファイル名と見なす拡張子（大小無視・要件 9.3）。読み込みは実装しない。
const FONT_FILE_EXTENSIONS: [&str; 3] = [".ttf", ".otf", ".ttc"];

/// 記録済み判定の鍵——**連結した綴りにしない**（モジュール doc「記録の重複判定の鍵」）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum WarnKey {
    /// 読み飛ばしたフォントファイル名の候補 1 つ。
    FontFile(String),
    /// 全滅した候補列（列のまま保つ）。
    AllMissing(Vec<String>),
}

/// 実在するフォント名の問い合わせ先。
enum FamilySource {
    /// システムのフォント集合（本番経路・生成時に 1 度だけ取得する）。
    System(IDWriteFontCollection),
    /// テスト用の偽の集合（小文字化した名前・本番には存在しない）。
    #[cfg(test)]
    Fake(BTreeSet<String>),
}

/// 候補列→実在するフォント名の解決・記憶・記録の 1 度化（要件 9.1〜9.4／9.7／9.8／13.2）。
pub struct FontCatalog {
    families: FamilySource,
    /// 候補列→解決した名前（`None`＝全滅）。鍵は候補列そのもの。
    memo: RefCell<HashMap<Vec<String>, Option<String>>>,
    /// 記録済みの鍵（[`WarnKey`]）。
    warned: RefCell<BTreeSet<WarnKey>>,
    /// 記憶に無くて実際に問い合わせた回数（要件 9.7 の観測点）。
    lookups: Cell<usize>,
}

impl FontCatalog {
    /// システムのフォント集合を 1 度だけ取得して台帳を作る。
    pub fn new(factory: &IDWriteFactory2) -> Result<FontCatalog, TextLayerError> {
        let base: IDWriteFactory = factory
            .cast()
            .map_err(device_err("QueryInterface(IDWriteFactory)"))?;
        let mut system: Option<IDWriteFontCollection> = None;
        unsafe { base.GetSystemFontCollection(&mut system, false) }
            .map_err(device_err("GetSystemFontCollection"))?;
        let collection = system.ok_or_else(|| {
            tracing::error!("GetSystemFontCollection が成功しながら集合を返さなかった");
            TextLayerError::Device {
                hresult: 0,
                context: "GetSystemFontCollection",
            }
        })?;
        Ok(FontCatalog::with_source(FamilySource::System(collection)))
    }

    fn with_source(families: FamilySource) -> FontCatalog {
        FontCatalog {
            families,
            memo: RefCell::new(HashMap::new()),
            warned: RefCell::new(BTreeSet::new()),
            lookups: Cell::new(0),
        }
    }

    /// テスト用——実機のフォント構成に依存しない偽の集合で台帳を作る。
    #[cfg(test)]
    pub(super) fn with_families(installed: &[&str]) -> FontCatalog {
        FontCatalog::with_source(FamilySource::Fake(
            installed.iter().map(|n| n.to_lowercase()).collect(),
        ))
    }

    /// 記憶に無くて実際に問い合わせた回数（要件 9.7 の観測点）。
    #[cfg(test)]
    pub(super) fn lookup_count(&self) -> usize {
        self.lookups.get()
    }

    /// 記述順に問い合わせて最初に見つかった名前を返す。空列は `None`（記録なし）。
    pub fn family_for(&self, candidates: &[String]) -> Option<String> {
        if candidates.is_empty() {
            return None;
        }
        if let Some(hit) = self.memo.borrow().get(candidates).cloned() {
            return hit;
        }
        self.lookups.set(self.lookups.get() + 1);

        let mut resolved = None;
        for candidate in candidates {
            if is_font_file(candidate) {
                // 読み込みは実装しない（要件 9.3）——記録を 1 度残して次の候補へ。
                if self.first_time(WarnKey::FontFile(candidate.clone())) {
                    warn!(
                        candidate = %candidate,
                        "フォントファイル名の候補は読み込まない——次の候補へ進む"
                    );
                }
                continue;
            }
            if self.has_family(candidate) {
                resolved = Some(candidate.clone());
                break;
            }
        }

        if resolved.is_none() && self.first_time(WarnKey::AllMissing(candidates.to_vec())) {
            warn!(
                candidates = %candidates.join(", "),
                "フォントの候補が 1 つも見つからない——既定のフォントへ戻す"
            );
        }
        self.memo
            .borrow_mut()
            .insert(candidates.to_vec(), resolved.clone());
        resolved
    }

    /// バルーン定義の候補列（`[name] ++ fallback_chain`）を解決した名前を `name` に
    /// 据えた複製を返す（要件 9.8）。全滅は [`DEFAULT_FONT_NAME`]。
    ///
    /// 既存の書式生成関数（`create_text_format`／`try_create_format`）は**この複製で**
    /// 呼ぶ——解決は書式生成の手前で終わっているので、生成関数の本文は変わらない。
    pub fn pick(&self, font: &ResolvedFont) -> ResolvedFont {
        let mut picked = font.clone();
        picked.name = self
            .family_for(&font.looks.default.name)
            .unwrap_or_else(|| DEFAULT_FONT_NAME.to_owned());
        picked
    }

    /// この鍵での記録がまだ無ければ記録済みにして `true`（記録の 1 度化・要件 9.3／9.4）。
    fn first_time(&self, key: WarnKey) -> bool {
        self.warned.borrow_mut().insert(key)
    }

    /// この機械にその名前のフォントが入っているか。
    fn has_family(&self, name: &str) -> bool {
        match &self.families {
            FamilySource::System(collection) => {
                let mut index = 0u32;
                let mut exists = BOOL(0);
                match unsafe {
                    collection.FindFamilyName(&HSTRING::from(name), &mut index, &mut exists)
                } {
                    Ok(()) => exists.as_bool(),
                    Err(e) => {
                        // log-first: 失敗は記録して「無い」扱いにする（次の候補へ進む）。
                        let _ = device_err("FindFamilyName")(e);
                        false
                    }
                }
            }
            #[cfg(test)]
            FamilySource::Fake(installed) => installed.contains(&name.to_lowercase()),
        }
    }
}

/// 候補がフォントファイル名か（大小無視・要件 9.3）。
fn is_font_file(candidate: &str) -> bool {
    let lower = candidate.to_ascii_lowercase();
    FONT_FILE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
#[path = "draw_catalog_tests.rs"]
mod tests;

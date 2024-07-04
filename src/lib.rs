// Copyright 2019 Mozilla Foundation. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An iterator adapter that takes an iterator over `char` yielding a sequence of
//! `char`s in Normalization Form C (this precondition is not checked!) and
//! yields `char`s either such that tone marks that wouldn't otherwise fit into
//! windows-1258 are decomposed or such that text is decomposed into orthographic
//! units.
//!
//! Use cases include preprocessing before encoding Vietnamese text into
//! windows-1258 or converting precomposed Vietnamese text into a form that looks
//! like it was written with the (non-IME) Vietnamese keyboard layout (e.g. for
//! machine learning training or benchmarking purposes).

#[repr(align(64))] // Align to cache lines
struct ToneData {
    windows_1258_key: [u8; 16],
    windows_1258_value: [u8; 16],
    middle_key: [u8; 14],
    middle_value: [u8; 14],
    extensions_for_vietnamese: [u16; 90],
}

// These arrays list the actual decomposed code point combinations that should
// replace a single, composed code point. For example, 0x1EA0 ("Ạ") decomposes
// into 0x0041 ("A") + 0x0323 ("Combining dot below"). Decompositions for
// windows-1258 are always two code points.
//
// Entries here pack information about both decomposed points into a single
// integer, where the lower bits indicate the first code point, and the upper
// bits indicate the second code point (usually a tone mark). There are three
// sets of decompositions, and each packs the the two code points together
// differently to make efficient use of memory.
static TONE_DATA: ToneData = ToneData {
    // Index for orthographic-only decompositions. This array lists the Unicode
    // code points that can be decomposed, while the `windows_1258_value` array
    // lists the actual decompositions for them at the corresponding index.
    windows_1258_key: [
        0xC0, // À
        0xC1, // Á
        0xC8, // È
        0xC9, // É
        0xCD, // Í
        0xD3, // Ó
        0xD9, // Ù
        0xDA, // Ú
        0xE0, // à
        0xE1, // á
        0xE8, // è
        0xE9, // é
        0xED, // í
        0xF3, // ó
        0xF9, // ù
        0xFA, // ú
    ],
    // Orthographic only decompositions. Given a composed code point, find it
    // in the above array, then look for the decomposition at the corresponding
    // index in this array.
    //
    // The lower 7 bits of a value is the first replacement code point. The
    // upper bit is the second code point, offset by negative 0x0300. For
    // example, the decomposition of 0xC0 ("À") is 0x41, which represents the
    // code points 0x41 ("A") + 0x0300 ("Combining grave accent"):
    //
    //     0x41  =  0b_0100_0001
    //     First:   0b_0100_0001 = 0x41
    //                  ^^^^^^^^   (Lower 7 bits)
    //     Second:  0b_0         = 0x00 + 0x300 = 0x0300
    //                 ^           (Upper bit)
    //
    windows_1258_value: [
        0x41, // À
        0xC1, // Á
        0x45, // È
        0xC5, // É
        0xC9, // Í
        0xCF, // Ó
        0x55, // Ù
        0xD5, // Ú
        0x61, // à
        0xE1, // á
        0x65, // è
        0xE5, // é
        0xE9, // í
        0xEF, // ó
        0x75, // ù
        0xF5, // ú
    ],
    // Index for decompositions of assorted code points outside the range
    // 0x1ea0 - 0x1efa. This array lists the Unicode code points that can be
    // decomposed (offset by negative 0xC3 so they fit in one byte). The actual
    // decomposition is at the corresponding index of the `middle_value` array.
    middle_key: [
        0x00, // Ã
        0x09, // Ì
        0x0F, // Ò
        0x12, // Õ
        0x1A, // Ý
        0x20, // ã
        0x29, // ì
        0x2F, // ò
        0x32, // õ
        0x3A, // ý
        0x65, // Ĩ
        0x66, // ĩ
        0xA5, // Ũ
        0xA6, // ũ
    ],
    // Decompositions. Given a composed code point, find it in the above array,
    // then find the decomposition at the corresponding index in this array.
    //
    // The lower 7 bits of a value is the first replacement code point. The
    // second code point is more complicated:
    //   - If the first point is 0x59 ("Y") or 0x79 ("y"), it is 0x0301
    //     ("Combining Acute Accent"). For these, ignore the upper bit.
    //   - If the upper bit is 0, it is 0x0300 ("Combining Grave Accent").
    //   - If the upper bit is 1, it is 0x0303 ("Combining Tilde")
    //
    // For example, the decomposition of 0xC3 ("Ã") 0xC1, which is the code
    // points 0x41 ("A") + 0x0303 ("Combining tilde"):
    //
    //     0xC1  =  0b_1100_0001
    //     First:   0b_0100_0001 = 0x41
    //                  ^^^^^^^^   (Lower 7 bits)
    //     Second:  0b_1         = 0x01 -> 0x0303
    //                 ^           (Upper bit)
    //
    middle_value: [
        0xC1, // Ã
        0x49, // Ì
        0x4F, // Ò
        0xCF, // Õ
        0x59, // Ý
        0xE1, // ã
        0x69, // ì
        0x6F, // ò
        0xEF, // õ
        0x79, // ý
        0xC9, // Ĩ
        0xE9, // ĩ
        0xD5, // Ũ
        0xF5, // ũ
    ],
    // Decompositions for code points in the range 0x1ea0 - 0x1efa (the main
    // range of composed vowels + accents + tone marks used in Vietnamese).
    //
    // Decompositions are listed in order, so the decomposition for code point
    // 0x1ea0 is at index 0, 0x1ea0 is at index 1, etc.
    // 
    // The lower 10 bits of a value is the first replacement code point. The
    // upper 6 bits are the second code point, offset by negative 0x0300. For
    // example, the decomposition of 0x1EA0 ("Ạ") is 0x8C41, which represents
    // the code points 0x41 ("A") + 0x0323 ("Combining dot below"):
    //
    //     0x8C41 =  0b_1000_1100_0100_0001
    //     First:    0b_0000_0000_0100_0001 = 0x41
    //                          ^^^^^^^^^^^   (Lower 10 bits)
    //     Second:   0b_1000_11             = 0x23 + 0x300 = 0x0323
    //                  ^^^^^^^               (Upper 6 bits)
    //
    extensions_for_vietnamese: [
        0x8C41, // Ạ
        0x8C61, // ạ
        0x2441, // Ả
        0x2461, // ả
        0x04C2, // Ấ
        0x04E2, // ấ
        0x00C2, // Ầ
        0x00E2, // ầ
        0x24C2, // Ẩ
        0x24E2, // ẩ
        0x0CC2, // Ẫ
        0x0CE2, // ẫ
        0x8CC2, // Ậ
        0x8CE2, // ậ
        0x0502, // Ắ
        0x0503, // ắ
        0x0102, // Ằ
        0x0103, // ằ
        0x2502, // Ẳ
        0x2503, // ẳ
        0x0D02, // Ẵ
        0x0D03, // ẵ
        0x8D02, // Ặ
        0x8D03, // ặ
        0x8C45, // Ẹ
        0x8C65, // ẹ
        0x2445, // Ẻ
        0x2465, // ẻ
        0x0C45, // Ẽ
        0x0C65, // ẽ
        0x04CA, // Ế
        0x04EA, // ế
        0x00CA, // Ề
        0x00EA, // ề
        0x24CA, // Ể
        0x24EA, // ể
        0x0CCA, // Ễ
        0x0CEA, // ễ
        0x8CCA, // Ệ
        0x8CEA, // ệ
        0x2449, // Ỉ
        0x2469, // ỉ
        0x8C49, // Ị
        0x8C69, // ị
        0x8C4F, // Ọ
        0x8C6F, // ọ
        0x244F, // Ỏ
        0x246F, // ỏ
        0x04D4, // Ố
        0x04F4, // ố
        0x00D4, // Ồ
        0x00F4, // ồ
        0x24D4, // Ổ
        0x24F4, // ổ
        0x0CD4, // Ỗ
        0x0CF4, // ỗ
        0x8CD4, // Ộ
        0x8CF4, // ộ
        0x05A0, // Ớ
        0x05A1, // ớ
        0x01A0, // Ờ
        0x01A1, // ờ
        0x25A0, // Ở
        0x25A1, // ở
        0x0DA0, // Ỡ
        0x0DA1, // ỡ
        0x8DA0, // Ợ
        0x8DA1, // ợ
        0x8C55, // Ụ
        0x8C75, // ụ
        0x2455, // Ủ
        0x2475, // ủ
        0x05AF, // Ứ
        0x05B0, // ứ
        0x01AF, // Ừ
        0x01B0, // ừ
        0x25AF, // Ử
        0x25B0, // ử
        0x0DAF, // Ữ
        0x0DB0, // ữ
        0x8DAF, // Ự
        0x8DB0, // ự
        0x0059, // Ỳ
        0x0079, // ỳ
        0x8C59, // Ỵ
        0x8C79, // ỵ
        0x2459, // Ỷ
        0x2479, // ỷ
        0x0C59, // Ỹ
        0x0C79, // ỹ
    ],
};

fn expand(u: u16) -> char {
    unsafe { std::char::from_u32_unchecked(u32::from(u)) }
}

/// An iterator adapter yielding `char` with tone marks detached.
#[derive(Debug)]
pub struct DecomposeVietnamese<I> {
    delegate: I,
    pending: char,
    orthographic: bool,
}

impl<I: Iterator<Item = char>> Iterator for DecomposeVietnamese<I> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        if self.pending != '\u{0}' {
            let c = self.pending;
            self.pending = '\u{0}';
            return Some(c);
        }
        if let Some(c) = self.delegate.next() {
            let s = c as usize;
            let minus_offset = s.wrapping_sub(0x1EA0);
            if minus_offset < TONE_DATA.extensions_for_vietnamese.len() {
                let val = TONE_DATA.extensions_for_vietnamese[minus_offset];
                let base = expand(val & 0x3FF);
                let tone = expand((val >> 10) + 0x0300);
                self.pending = tone;
                return Some(base);
            }
            if c >= '\u{C3}' && c <= '\u{0169}' {
                let key = (s - 0xC3) as u8;
                if let Ok(i) = TONE_DATA.middle_key.binary_search(&key) {
                    let val = TONE_DATA.middle_value[i];
                    let base = char::from(val & 0x7F);
                    let tone = if (val & 0x5F) == b'Y' {
                        // There has to be a more elegant way to handle this.
                        '\u{0301}'
                    } else if (val >> 7) == 0 {
                        '\u{0300}'
                    } else {
                        '\u{0303}'
                    };
                    self.pending = tone;
                    return Some(base);
                }
            }
            if self.orthographic && c >= '\u{C0}' && c <= '\u{FA}' {
                if let Ok(i) = TONE_DATA.windows_1258_key.binary_search(&(c as u8)) {
                    let val = TONE_DATA.windows_1258_value[i];
                    let base = char::from(val & 0x7F);
                    let tone = (val >> 7) as u16 + 0x0300;
                    self.pending = expand(tone);
                    return Some(base);
                }
            }
            return Some(c);
        }
        None
    }
}

/// Trait that adds a `decompose_vietnamese_tones` method to iterators
/// over `char`.
pub trait IterDecomposeVietnamese<I: Iterator<Item = char>> {
    /// Assuming that `self` is an iterator yielding a sequence of
    /// `char`s in Normalization Form C (this precondition is not
    /// checked!), yields a sequence of `char`s with Vietnamese tone
    /// marks less or more decomposed. Note that the output is _not_
    /// in Unicode Normalization Form D or any Normalization Form.
    /// Circumflex and breve are not detached from their base characters.
    ///
    /// If `orthographic` is `false`, tone marks are decomposed if
    /// there is no precomposed form form the incoming character in
    /// windows-1258. E.g. á is not decomposed, but ý is decomposed to
    /// y followed by combining acute and ấ is decomposed to â followed
    /// by combining acute.
    ///
    /// If `orthographic` is `true`, tone marks are always decomposed.
    /// That is, even á is decomposed.
    fn decompose_vietnamese_tones(self, orthographic: bool) -> DecomposeVietnamese<I>;
}

impl<I: Iterator<Item = char>> IterDecomposeVietnamese<I> for I {
    /// Assuming that `self` is an iterator yielding a sequence of
    /// `char`s in Normalization Form C (this precondition is not
    /// checked!), yields a sequence of `char`s with Vietnamese tone
    /// marks less or more decomposed. Note that the output is _not_
    /// in Unicode Normalization Form D or any Normalization Form.
    /// Circumflex and breve are not detached from their base characters.
    ///
    /// If `orthographic` is `false`, tone marks are decomposed if
    /// there is no precomposed form form the incoming character in
    /// windows-1258. E.g. á is not decomposed, but ý is decomposed to
    /// y followed by combining acute and ấ is decomposed to â followed
    /// by combining acute.
    ///
    /// If `orthographic` is `true`, tone marks are always decomposed.
    /// That is, even á is decomposed.
    #[inline]
    fn decompose_vietnamese_tones(self, orthographic: bool) -> DecomposeVietnamese<I> {
        DecomposeVietnamese {
            delegate: self,
            pending: '\u{0}',
            orthographic: orthographic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unic_normal::StrNormalForm;

    fn check(nfc: char, base: char, tone: char) {
        let mut decompose_vietnamese = std::iter::once(nfc).decompose_vietnamese_tones(true);
        assert_eq!(decompose_vietnamese.next(), Some(base));
        assert_eq!(decompose_vietnamese.next(), Some(tone));
        assert_eq!(decompose_vietnamese.next(), None);
    }

    #[test]
    fn test_tones() {
        let bases = [
            'A', 'a', 'Ă', 'ă', 'Â', 'â', 'E', 'e', 'Ê', 'ê', 'I', 'i', 'O', 'o', 'Ô', 'ô',
            'Ơ', 'ơ', 'U', 'u', 'Ư', 'ư', 'Y', 'y',
        ];
        let tones = ['\u{0300}', '\u{0309}', '\u{0303}', '\u{0301}', '\u{0323}'];
        for &base in bases.iter() {
            for &tone in tones.iter() {
                let mut paired = String::new();
                paired.push(base);
                paired.push(tone);
                let nfc = paired.nfc().next().unwrap();
                check(nfc, base, tone);
            }
        }
    }
}

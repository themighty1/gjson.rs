// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

use alloc::string::String;
use alloc::vec::Vec;

/// tostr transmutes a byte slice to a string reference. This function must
/// only be used on path components and json data which originated from the
/// super::get() function. The super::get() function only accepts &str
/// references and expects that the inputs are utf8 validated. All slices to
/// the json and path data during the get().
pub fn tostr<'a>(v: &'a [u8]) -> &'a str {
    // SAFETY: All slices to the json and path data during the get()
    // operation are done at ascii codepoints which ensuring that the
    // conversion is safe.
    unsafe { core::str::from_utf8_unchecked(v) }
}

pub fn trim<'a>(mut bin: &'a [u8]) -> &'a [u8] {
    while bin.len() > 0 && bin[0] <= b' ' {
        bin = &bin[1..];
    }
    while bin.len() > 0 && bin[bin.len() - 1] <= b' ' {
        bin = &bin[..bin.len() - 1];
    }
    bin
}

// unescape a json string.
pub fn unescape(json: &str) -> String {
    let json = json.as_bytes();
    if json.len() < 2 || json[0] != b'"' || json[json.len() - 1] != b'"' {
        return String::new();
    }
    let json = &json[1..json.len() - 1];
    let mut out = Vec::with_capacity(json.len());
    let mut i = 0;
    loop {
        if i == json.len() || json[i] < b' ' {
            break;
        } else if json[i] == b'\\' {
            i += 1;
            if i == json.len() {
                break;
            }
            match json[i] {
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                b'/' => out.push(b'/'),
                b'b' => out.push(8),
                b'f' => out.push(12),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'u' => {
                    if i + 5 > json.len() {
                        break;
                    }
                    let mut r =
                        u32::from_str_radix(tostr(&json[i + 1..i + 5]), 16).unwrap_or(0xFFFD);
                    i += 5;
                    if utf16_is_surrogate(r) {
                        // need another code
                        if (&json[i..]).len() >= 6 && json[i] == b'\\' && json[i + 1] == b'u' {
                            if let Ok(r2) = u32::from_str_radix(tostr(&json[i + 2..i + 6]), 16) {
                                r = utf16_decode(r, r2);
                            } else {
                                r = 0xFFFD;
                            }
                            i += 6
                        }
                    }
                    let ch = core::char::from_u32(r).unwrap_or(core::char::REPLACEMENT_CHARACTER);
                    let mark = out.len();
                    for _ in 0..10 {
                        out.push(0);
                    }
                    let n = ch.encode_utf8(&mut out[mark..]).len();
                    out.truncate(mark + n);
                    continue;
                }
                _ => break,
            }
        } else {
            out.push(json[i]);
        }
        i += 1;
    }
    unsafe { String::from_utf8_unchecked(out) }
}

fn utf16_is_surrogate(r: u32) -> bool {
    0xd800 <= r && r < 0xe000
}

fn utf16_decode(r1: u32, r2: u32) -> u32 {
    if 0xd800 <= r1 && r1 < 0xdc00 && 0xdc00 <= r2 && r2 < 0xe000 {
        (r1 - 0xd800) << 10 | (r2 - 0xdc00) + 0x10000
    } else {
        0xFFFD
    }
}

// fn next_json_encoded_rune(iter: &mut std::str::Chars) -> Option<u16> {
//     (iter.next()?.to_digit(16)? << 16)
//         | (iter.next()?.to_digit(16)? << 8)
//         | (iter.next()?.to_digit(16)? << 4)
//         | (iter.next()?.to_digit(16)? << 0);
//     None
// }

// pub fn need_escaping(s: &str) -> bool {
//     let s = s.as_bytes();
//     for i in 0..s.len() {
//         if s[i] < b' ' || s[i] == b'\n' || s[i] == b'\\' || s[i] == b'"' {
//             return true;
//         }
//     }
//     return false;
// }

/// pmatch returns true if str matches pattern. This is a very
/// simple wildcard match where '*' matches on any number characters
/// and '?' matches on any one character.
///
/// pattern:
///   { term }
/// term:
/// 	 '*'         matches any sequence of non-Separator characters
/// 	 '?'         matches any single non-Separator character
/// 	 c           matches character c (c != '*', '?')
/// 	'\\' c       matches character c
pub fn pmatch<S, P>(pattern: P, string: S) -> bool
where
    S: AsRef<[u8]>,
    P: AsRef<[u8]>,
{
    let mut string = string.as_ref();
    let mut pattern = pattern.as_ref();
    while pattern.len() > 0 {
        if pattern[0] == b'\\' {
            if pattern.len() == 1 {
                return false;
            }
            pattern = &pattern[1..];
        } else if pattern[0] == b'*' {
            if pattern.len() == 1 {
                return true;
            }
            if pattern[1] == b'*' {
                pattern = &pattern[1..];
                continue;
            }
            if pmatch(&pattern[1..], string) {
                return true;
            }
            if string.len() == 0 {
                return false;
            }
            string = &string[1..];
            continue;
        }
        if string.len() == 0 {
            return false;
        }
        if pattern[0] != b'?' && string[0] != pattern[0] {
            return false;
        }
        pattern = &pattern[1..];
        string = &string[1..];
    }
    return string.len() == 0 && pattern.len() == 0;
}

#[cfg(test)]
mod test {

    #[test]
    fn basic() {
        assert_eq!(true, super::pmatch("*", "",));
        assert_eq!(true, super::pmatch("", "",));
        assert_eq!(false, super::pmatch("", "hello world",));
        assert_eq!(false, super::pmatch("jello world", "hello world",));
        assert_eq!(true, super::pmatch("*", "hello world",));
        assert_eq!(true, super::pmatch("*world*", "hello world",));
        assert_eq!(true, super::pmatch("*world", "hello world",));
        assert_eq!(true, super::pmatch("hello*", "hello world",));
        assert_eq!(false, super::pmatch("jello*", "hello world",));
        assert_eq!(true, super::pmatch("hello?world", "hello world",));
        assert_eq!(false, super::pmatch("jello?world", "hello world",));
        assert_eq!(true, super::pmatch("he*o?world", "hello world",));
        assert_eq!(true, super::pmatch("he*o?wor*", "hello world",));
        assert_eq!(true, super::pmatch("he*o?*r*", "hello world",));
        assert_eq!(true, super::pmatch("h\\*ello", "h*ello",));
        assert_eq!(false, super::pmatch("hello\\", "hello\\",));
        assert_eq!(true, super::pmatch("hello\\?", "hello?",));
        assert_eq!(true, super::pmatch("hello\\\\", "hello\\",));

        // test for fast repeating stars
        let string = ",**,,**,**,**,**,**,**,";
        let pattern = ",**********************************************{**\",**,,**,**,**,**,\"\",**,**,**,**,**,**,**,**,**,**]";
        super::pmatch(pattern, string);
    }
    #[test]
    fn unescape() {
        assert_eq!(super::unescape(r#""adsf"#), "");
        assert_eq!(super::unescape(r#""ad\sf""#), "ad");
        assert_eq!(
            super::unescape(r#""ad\"\\\/\b\f\n\r\tsf""#),
            "ad\"\\/\u{08}\u{0C}\n\r\tsf"
        );
        assert_eq!(super::unescape(r#""ad\uD83Dsf""#), "ad�sf");
        assert_eq!(super::unescape(r#""ad\uD83D\usf""#), "ad�");
        assert_eq!(super::unescape(r#""ad\uD83D\uxxxxsf""#), "ad�sf");
        assert_eq!(super::unescape(r#""ad\uD83D\u00FFsf""#), "ad�sf");
    }
}

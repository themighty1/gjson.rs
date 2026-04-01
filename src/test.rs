// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

#[cfg(test)]
extern crate std;

#[cfg(test)]
use super::*;
#[cfg(test)]
use alloc::format;
#[cfg(test)]
use alloc::string::{String, ToString};
#[cfg(test)]
use alloc::vec::Vec;

#[test]
fn various() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    assert_eq!(get(&json, "search_metadata.count").u64(), 100);
}

#[test]
fn iterator() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let mut index = 0;
    let mut res = String::new();
    res.push_str("[");
    parse(&json).each(|key, value| -> bool {
        if key.str() == "statuses" {
            value.each(|_, value| -> bool {
                if index > 0 {
                    res.push_str(",");
                }
                res.push_str(value.get("user.name").json());
                index += 1;
                return true;
            })
        }
        return true;
    });
    res.push_str("]");
    assert_eq!(index, 100);
    assert_eq!(get(&res, "50").str(), "イイヒト");
}

#[test]
fn array() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let res1 = get(&json, "statuses.#.user.name");
    let res2 = parse(&json);
    let res3 = res2.get("statuses.#.user.name");
    assert_eq!(res1.get("#").u64(), 100);
    assert_eq!(res3.get("#").u64(), 100);
    assert_eq!(res1.str(), res3.str());
}

#[test]
fn query() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let res = get(
        &json,
        "statuses.#(user.name==イイヒト).user.profile_link_color",
    );
    assert_eq!(res.str(), "0084B4");
}

#[test]
fn jsonlines() {
    let json = r#"
        {"a": 1 }
        {"a": 2 }
        true
        false
        4
    "#;
    assert_eq!(get(json, "..#").i32(), 5);
    assert_eq!(get(json, "..0.a").i32(), 1);
    assert_eq!(get(json, "..1.a").i32(), 2);
}

#[test]
fn escaped() {
    let json1 = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let json2 = std::fs::read_to_string("testfiles/twitterescaped.json").unwrap();
    assert_eq!(
        get(&json1, "statuses.#").i32(),
        get(&json2, "statuses.#").i32()
    );
    for i in 0..100 {
        let path = format!("statuses.{}.text", i);
        assert_eq!(get(&json1, &path).str(), get(&json2, &path).str());
        let path = format!("statuses.{}.user.name", i);
        assert_eq!(get(&json1, &path).str(), get(&json2, &path).str());
        break;
    }
}

#[cfg(test)]
const EXAMPLE: &str = r#"
{
  "name": {"first": "Tom", "last": "Anderson"},
  "age":37,
  "children": ["Sara","Alex","Jack"],
  "fav.movie": "Deer Hunter",
  "friends": [
    {"first": "Dale", "last": "Murphy", "age": 44, "nets": ["ig", "fb", "tw"]},
    {"first": "Roger", "last": "Craig", "age": 68, "nets": ["fb", "tw"]},
    {"first": "Jane", "last": "Murphy", "age": 47, "nets": ["ig", "tw"]}
  ]
}
"#;

#[cfg(test)]
fn exec_simple_fuzz(data: &[u8]) {
    if let Ok(s) = core::str::from_utf8(data) {
        let _ = core::str::from_utf8(get(s, s).json().as_bytes()).unwrap();
        let _ = core::str::from_utf8(get(EXAMPLE, s).json().as_bytes()).unwrap();
    }
}

#[test]
fn fuzz() {
    // This only runs on crash files in the fuzz directory.
    let crash_dir = "extra/fuzz/out/default/crashes";
    if !std::path::Path::new(crash_dir).exists() {
        return;
    }
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(crash_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();
    files.sort();
    for file in files {
        let fname = file.as_path().to_str().unwrap().to_string();
        std::eprintln!("{}", fname);
        let data = std::fs::read(file).unwrap();
        exec_simple_fuzz(&data);
    }
}

#[test]
fn array_value() {
    const PROGRAMMERS: &str = r#"
    {
        "programmers": [
          {
            "firstName": "Janet",
            "lastName": "McLaughlin",
          }, {
            "firstName": "Elliotte",
            "lastName": "Hunter",
          }, {
            "firstName": "Jason",
            "lastName": "Harold",
          }
        ]
      }
    "#;
    let mut res = String::new();
    let value = get(PROGRAMMERS, "programmers.#.lastName");
    for name in value.array() {
        res.extend(format!("{}\n", name).chars());
    }
    assert_eq!(res, "McLaughlin\nHunter\nHarold\n");
}

#[test]
fn escaped_query_string() {
    const JSON: &str = r#"
    {
        "name": {"first": "Tom", "last": "Anderson"},
        "age":37,
        "children": ["Sara","Alex","Jack"],
        "fav.movie": "Deer Hunter",
        "friends": [
          {"first": "Dale", "last": "Mur\"phy", "age": 44, "nets": ["ig", "fb", "tw"]},
          {"first": "Roger", "last": "Craig", "age": 68, "nets": ["fb", "tw"]},
          {"first": "Jane", "last": "Murphy", "age": 47, "nets": ["ig", "tw"]}
        ]
      }
    }
    "#;
    assert_eq!(get(JSON, r#"friends.#(last="Mur\"phy").age"#).i32(), 44);
    assert_eq!(get(JSON, r#"friends.#(last="Murphy").age"#).i32(), 47);
}

#[test]
fn bool_convert_query() {
    const JSON: &str = r#"
    {
		"vals": [
			{ "a": 1, "b": true },
			{ "a": 2, "b": true },
			{ "a": 3, "b": false },
			{ "a": 4, "b": "0" },
			{ "a": 5, "b": 0 },
			{ "a": 6, "b": "1" },
			{ "a": 7, "b": 1 },
			{ "a": 8, "b": "true" },
			{ "a": 9, "b": false },
			{ "a": 10, "b": null },
			{ "a": 11 }
		]
	}
    "#;

    assert_eq!(get(JSON, r#"vals.#(b==~true)#.a"#).json(), "[1,2,6,7,8]");
}

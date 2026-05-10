> [!WARNING]
> The library API may change any day of the week. The CLI should be stable enough, but there are no guarantees.

# Pangloss

[![Build status](https://github.com/daxida/pangloss/workflows/ci/badge.svg)](https://github.com/daxida/pangloss/actions)
[![Crates.io](https://img.shields.io/crates/v/pangloss.svg)](https://crates.io/crates/pangloss)

Tool for converting, from and to, different glossary (i.e. dictionary) formats.

Inspired by [pyglossary](https://github.com/ilius/pyglossary).

The name is a wordplay on pandoc and pyglossary, and the famous character of Voltaire's Candide.

It only targets a subset of the functionality that is relevant to [Jitendex](https://github.com/Jitendex/Jitendex) and [wiktionary-to-yomitan](https://github.com/yomidevs/wiktionary-to-yomitan). The objective is mainly to do a better job in that particular subset, and (maybe, if I ever learn how to add some bindings) to allow some glossary creation API in dotnet.

At the moment it supports Yomitan, Stardict and Mdict.

## Usage

To install:

```console
$ cargo install pangloss
```

Then run (.ifo detects Stardict, .zip detects Yomitan):

```console
$ pangloss path/to/stardict.ifo path/to/out.zip
```

Use `pangloss --help` to show the help menu.

## Readers

Every non-trivial format targets a concrete reader, f.e. [goldendict-ng] in the case of Mdict. While other readers may be able to import the format, they usually don't support the same range of features. Here is the list of targets:

| Format | Target Reader |
|--------|--------------|
| Yomitan | [yomitan] |
| Stardict | [koreader] |
| Mdict | [goldendict-ng] |

[yomitan]: https://github.com/yomidevs/yomitan
[koreader]: https://github.com/koreader/koreader
[goldendict-ng]: https://github.com/xiaoyifang/goldendict-ng

## Library

You can use this as a library to make dictionaries, but that is not a priority.

## TODO
- [ ] Make a good stardict from jitendex-yomitan!!
  - [x] Where did the pronunciation go? (it's not in yomitan for [reasons](https://github.com/yomidevs/yomitan/issues/324))
  - [x] circled numbers
  - [ ] Star icon
    - [ ] Consume non-css data files!
  - [ ] Requires glossary transform
  - [ ] Compact mode (?)
    - [ ] Can we pass an extra data_file via CLI?

- [ ] (*) (?) Use serde_with (for skip serializing none)
- [ ] Be able to use the dictionary in memory? (like the dictuils AI slop)

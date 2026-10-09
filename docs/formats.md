Document features

### Mdict

- [spec](https://mdict4j.readthedocs.io/zh-cn/latest/reference/fileformat.html)
- [mdict-utils](https://github.com/liuyug/mdict-utils)
- [goldendict parser](https://github.com/xiaoyifang/goldendict-ng/blob/staged/src/dict/mdictparser.cc)

### Stardict

- [spec](https://github.com/huzheng001/stardict-3/blob/master/dict/doc/StarDictFileFormat)
- Doesn't compress the dict file

### Yomitan

- [repo](https://github.com/yomidevs/yomitan)
- [schemas](https://github.com/yomidevs/yomitan/tree/master/ext/data/schemas)
- [fixtures](https://github.com/yomidevs/yomitan/tree/master/test/data/dictionaries/valid-dictionary1)
- [ts schemas](https://github.com/MarvNC/yomichan-dict-builder/tree/master/src/types/yomitan)
  - [term bank](https://github.com/MarvNC/yomichan-dict-builder/blob/master/src/types/yomitan/termbank.ts#L35)

### DSL

ABBYY never published a spec beyond the DSL Compiler chapter of the Lingvo help.

- [dictionary structure](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-dictionary-structure/): header, encodings, `#INCLUDE`
- [card structure](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-card-structure/): headwords, card bodies
- [tags](http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-tags/)
- [pyglossary reader](https://github.com/ilius/pyglossary/tree/master/pyglossary/plugins/dsl): what we follow, html included
- [goldendict parser](https://github.com/xiaoyifang/goldendict-ng/blob/staged/src/dict/dsl_details.cc)

### Babylon

Not a format we support, but one whose leftovers arrive inside the others.

- [reader_charset.py](https://github.com/ilius/pyglossary/blob/master/pyglossary/plugins/babylon_bgl/reader_charset.py)

## Unsupported

For future reference.

### Aard2 slob

- [repo](https://github.com/itkach/slob)
- [site](https://aarddict.org/)

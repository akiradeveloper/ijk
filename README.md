# ijk

[![Tokei](https://tokei.rs/b1/github/akiradeveloper/ijk)](https://github.com/akiradeveloper/ijk)
[![Build Status](https://travis-ci.org/akiradeveloper/ijk.svg?branch=develop)](https://travis-ci.org/akiradeveloper/ijk)

A real editor for real programmers.

![Demo](https://github.com/akiradeveloper/ijk/blob/media/ijk-demo.gif)

## Usage

* `make install` to install.
* `ijk` to start ijk with the current directory.

## Design Doc

[Design Doc](https://docs.google.com/presentation/d/1_oQ_Dryehfi-3vBBCQI_AFZDrvxvXp-LToMcWNIehPM/edit?usp=sharing)

## Features

ijk is similar to vi but with some differences:

* Word range: In vi, dw and cw affects the range from the current cursor position to the end of the word as if user started a visual mode and word-jumped. This behavior is rarely meaningful in software programming. Instead, dw and cw affects the entire word the cursor is on. This behavior is consistent with the char version of delete and replace (x and r) in a sense that they affect the object the cursor is on.
* Native directory explorer: ijk has a native implementation of directory explorer similar to Defx. This is because real software programming rarely end up with coding a single file but many files and directories in tree. With the explorer you can access to files and directories using line jump and string search as in text editting.
* Navigator: Navigator is the central part of ijk and you can switch to navigator anytime by pressing C-w. Navigator is like a stack of living pages. You can access to any open files, directories and other temporary pages like command selector and switch between them very quickly.
* Space prefix: vi's command prefix is often mistyped because it needs two keys (shift+;) pressed at the same time in US keyboard. ijk uses space to start command mode where you can w to save the file etc.

## Tasks

- [x] Syntax highlighting (probably the syntect is a good choice)
- [x] Automated benchmarking (using flamer is planned) and performance optimization
- [x] Code snippet
- [ ] User config
- [ ] Integration with language server
- [ ] Integration with Git

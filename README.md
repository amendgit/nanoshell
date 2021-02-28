# Nanoshell (Experimental embedder for Flutter)

Video:<br/>
[![](https://img.youtube.com/vi/2nzIkQvYnvM/hq1.jpg)](http://www.youtube.com/watch?v=2nzIkQvYnvM "")

## Features

- Leverages existing desktop embedders on each platform
- Unlike regular desktop embedders, nanoshell provides consistent platform agnostic API
- Multi-window support
- Window management
    - Adjusting window styles and geometry
    - Modal dialogs
    - Windows can be set to track content size and resize automatically when content changes
- Platform menus (popup menu at this point, menubar coming)
- Drag and Drop
- Written in Rust, Flutter build integrated with cargo

## Status

- This is project in a very experimental stage, MacOS and Windows backends have feature parity,
  work on Linux backend has not started yet. `nanoshell/src/shell/platform/null` would be the place to start when
  porting to new platform.

## Getting started

In theory, it should be as easy as

```
git clone https://github.com/iocave/nanoshell.git
cd nanoshell/nanoshell_demo
cargo run
```

Reality is a fair bit more complicated, as nanoshell requires latest Flutter master, on Windows with some pull requests applied that haven't been merged yet.

Also `nanoshell_demo/build.rs` has local engine name hardcoded right now.

There is not a whole lot of documentation at this point, example app within `nanoshell_demo` is probably the best place to start.

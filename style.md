### 0. Intro

First, read <https://doc.rust-lang.org/beta/style-guide/index.html>. Below is my own version of this.
Second, use clippy and rustfmt! Many things are omitted from this guide because one of the two tools already cover them.

### 1. Modules

The order for each file should be as shown:
- Comment with LICENSE (not relevant until I make this open-source)
- `extern`s
- `use`s
- `mod`s
- Macros
- Types
- Enums/unions/structs in that order, unless needed otherwise
- Consts/statics
- Impls
- Functions definitions

Unit tests should usually be inline modules. All other modules should be in external files.

### 2. Imports

Only import what is used.
When two items in different crates have the same name, bring the parent modules into scope instead. Don't use `as` to rename an item.
```
// ok
use x::y;
use a::b;

// ...

y::z
b::z
```
```
// not ok
use x::y::z;
use a::b::z as a_z;
```

### 3. Indentation and line width

Keep both comments and doc comments to a width of 79.

### 4. Newlines

Separate everything at indentation 0 with a blank line. Group `use`s, `mod`s, `type`s, `const`s and `static`s though.
Never use two blank lines.

### 5. Names

Use descriptive names. This is Rust, not C.

### 6. Functions and methods

Functions shouldn't be as small as possible for the sake of being small.

The `impl` blocks should be in this order for each struct:
- Constants
- Trait impls
- Methods/associated functions

If two struct share the same type of `impl`, they should be grouped, in alphabetical order.
```
pub struct A;
pub struct B;

impl A {
    pub const FOO: i32 = 5;
}

impl B {
    pub const FOO: i32 = 5;
}

impl A {
    pub fn bar() {}
}

impl B {
    pub fn bar() {}
}
```
Functions and methods should be ordered in whatever order seems logical.

### 7. Assembly

Don't use it.

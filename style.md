### 0. Intro

First, read <https://doc.rust-lang.org/beta/style-guide/index.html>. Below is my own version of this.
Second, use clippy and rustfmt! Many things are omitted from this guide because one of the two tools already cover them.

### 1. Modules

The order for each file should be as shown:
- Comment with LICENSE (not relevant until I make this open-source)
- `extern`s
- `use`s
- `mod`s
- Types
- Enums/unions/structs in that order, unless needed otherwise
- Consts/statics
- Traits
- Impls
- Functions definitions, in descending order of abstraction (so main() first)

Don't use the old file path style. This means you should never make a file called `mod.rs`.
Unit tests should usually be inline modules. All other modules should be in external files.

### 2. Imports

Only import what is used.
```
// ok
use a::{
    b::{
        c::{
            d::e,
            f::g,
        },
    },
    h::i,
};
```
```
// not ok
use a::{b::{c, d}, f};
```

When two items in different crates have the same name, bring the parent modules into scope instead. Don't use `as` to rename an item.
```
// ok
use x::y;
use a::b;

...

y::z
b::z
```
```
// not ok
use x::y::z;
use a::b::z as a_z;
```

### 3. Indentation and line width

Avoid excessive indentation.
Keep both comments and doc comments to 80 lines.

### 4. Newlines

At the highest scope, everything should be separated with a blank line, unless they take only one line. This includes everything defined in the 'order' section.
Function definitions & declarations should be separated with a blank line at any scope.
Never use two blank lines.
```
fn fun() {
    u32 x = 3;
    u32 y = 4;

    if x > y {
        do_something();
    }
}
```

### 5. Spaces

Operators should generally be preceded and followed by a space.
However, unary operators (`&`, `*`, `-`, `!`) should be attached to whatever they affect.

### 6. Names

The length of variable names should be proportional to their scope. A for loop counter might be called `i`; calling a global variable `i` is criminal.
Names should *always* be descriptive, whatever it's naming. If a variable name requires a comment, you are naming it wrong.

### 7. Functions and methods

Functions shouldn't be as small as possible for the sake of being small.
If a function is doing several separate things, fence the separate things into their own braced blocks. If a block needs to be called elsewhere, turn it into a function.
The `impl` blocks should be in this order for each struct:
- Trait impls
- Public constants
- Private constants
- Public associated functions
- Public methods
- Private associated functions
- Private methods

If two struct share the same type of `impl`, they should be grouped, in alphabetical order.
```
struct A;
struct B;

impl A {
    pub fn foo() {}
}

impl B {
    pub fn foo() {}
}

impl A {
    fn bar() {}
}

impl B {
    fn bar() {}
}
```
Functions and methods should be ordered in whatever order seems logical.

### 8. Comments

Comments are an unfortunate necessity. Write your code such that the code itself explains what it's doing. Sometimes comments are needed, but they shouldn't be needed often.
If a comment is a single sentence, the first letter should not be capitalised and the full stop should be omitted; 'shortened' grammar is also fine.
```
// don't do this.
// Don't do this
// Don't do this.
// do this
```

### 9. Assembly

Don't use it.

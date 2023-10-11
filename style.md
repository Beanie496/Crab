### 0. Intro

First, read <https://doc.rust-lang.org/beta/style-guide/index.html>. Below is my own version of this.

### 1. Modules

The order for each file should be as shown:
- Comment with LICENSE (not relevant until I make this open-source)
- `extern`s
- `use`s
- `mod`s
- Types
- Enums/unions/structs in that order, unless needed otherwise
- Traits
- Impls
- Global variables
- Functions definitions, in descending order of abstraction (so main() first)

Don't use the old file path style. This means you should never make a file called `mod.rs`.
Don't make inline modules. Always put the code in an external file.

### 2. Imports

Do not nest imports on one line, and do not nest imports more than one level deep.
```
// ok
use a::b::{c, d};
use a::e::f;
```
```
// ok
use a::{
    b::{c, d},
    e::f,
};
```
```
// not ok
use a::{b::{c, d}, f};
```
```
// not ok
use a::{
    b::{c::{
        d::e,
        f::g,
    },
    f::g,
};
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

Never leave trailing whitespace. Ever.
Avoid excessive indentation. If you can see a refactor that removes at least one level of indentation, do it.
There is no hard line length limit but there is a soft limit of 80. Avoid long lines because they are ugly.

### 4. Newlines

At the highest scope, everything should be separated with a blank line. This includes everything defined in the 'order' section.
Function definitions & declarations should be separated with a blank line at any scope.
When the code changes slightly (e.g. from declarations to expressions), separate with one blank line. Otherwise, cluster.
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

### 5. Curly brackets

K&R-style. Opening curly brackets should almost always be on the same line as the expression that necessitates them.
```
if thing {
    statement;
    statement-2;
}
```

### 6. Spaces

Operators should generally be preceded and followed by a space.
However, unary operators (`&`, `*`, `-`, `!`) should be attached to whatever they affect.

### 7. Names

The length of variable names should be proportional to their scope. A for loop counter might be called `i`; calling a global variable `i` is criminal.
Names should *always* be descriptive, whatever it's naming. If a variable name requires a comment, you are naming it wrong.

### 8. Functions

Functions shouldn't be as small as possible for the sake of being small.
If a function is doing several separate things, fence the separate things into their own braced blocks. If a block needs to be called elsewhere, turn it into a function.
If a function returns nothing, it should *always* have side effects. If a function returns a value, it should *never* have side effects.

### 9. Comments

Comments are an unfortunate necessity. Write your code such that the code itself explains what it's doing. Sometimes comments are needed, but they shouldn't be needed often.
If a comment is a single sentence, the first letter should not be capitalised and the full stop should be omitted; 'shortened' grammar is also fine.
```
// don't do this.
// Don't do this
// Don't do this.
// do this
```

### 10. Assembly

Don't use it.

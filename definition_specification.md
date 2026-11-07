# Specification for format for defining the assembly language

## Header

The header can optionally have the architecture name, variant name and description.

```toml
[meta]
architecture = "arch name"
variant = "variant name"

description = "cool isa"
```

The architecture section defines info about the architecture, this section is optional.

```toml
[architecture]
word_size = 18 # default 8
endian = "big" # default big
align = 1 # default 1, in words
```

The const section defines constants.

```toml
[const]
foo = 8
bar = "foo * 2"
```

The syntax section exists.

```toml
[syntax]
comment = [";", "//"]
is_case_sensitive = false
```

## Data

Data is defined as a field with a name that is not reserved.

```toml
[register]
description = "8-bit register" # optional
width = 2 # optional, if missing inferred from largest member

[register.items]
a = 0
b = 1
c = 2
d = 3
```

For more complex data use processors.

```toml
[[ptr.items]]
pattern = "[{reg:register}]"
data = { reg = "reg", offset = 0 }

[[ptr.items]]
pattern = "[{reg:register} + {offset:S6}]"
data = { reg = "reg", offset = "offset" }
```

## Instructions

```toml
[[instruction]]
pattern = "add {acc:register}, {b:register}"
encoding = "0010'4 @ acc @ b"

[[instruction]]
pattern = "wr {addr:ptr}, {reg:register}"
encoding = "0101'4 @ addr.reg @ reg @ 01'2 @ addr.offset"
```

Something like this is even possible.

```toml
[[instruction]]
pattern = "{acc:register} += {b:register}"
encoding = "0010'4 @ acc @ b"
```

## Patterns

This is a pattern:

`{acc:register} += {b:register}`

A pattern consists of 2 types of objects, literals and bindings.
Whitespace around the pattern is ignored.

To include a literal brace inside a pattern, escape it as `{{` or `}}`. Unescaped `{` and `}` delimit a binding.

This is the syntax for a binding:

`{variable_to_bind:type}`

Spaces are allowed:

`{ variable_to_bind : type }`

## TOC

- [Installation](#installation)
- [Explanation](#explanation)

## Installation <a name="installation"></a>

### From Crates.io

`cargo install reref`

### From Github

`cargo install reref --git https://github.com/paritytech/reref`

## Explanation <a name="explanation"></a>

`reref` is a tool for transforming dependencies fields' on all `Cargo.toml` in a
given project.

Suppose you have the following `Cargo.toml`

```toml
[dependencies]
foo = { git = "https://github.com/org/foo", branch = "master" }
```

And you want to replace all `"branch" = "master"` with `"tag" = "v0.1"` where
`"git" = "https://github.com/org/foo"`. The command would be:

```sh
reref \
  --project path/to/project \
  --match-git https://github.com/org/foo \
  --remove-field branch \
  --add-field tag \
  --added-field-value v0.1
```

If you'd like to automatically Git commit the modifications made, also add the
`--autocommit` flag.
